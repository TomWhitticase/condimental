//! The asset generators themselves (UI_UX.md §2a point 1): one small,
//! parameterized Rust function per voxel model, built from `vox.rs`'s
//! composable primitives.
//!
//! These functions — not the `.vox` files in `models/` — are the source of
//! truth. If a shape needs to change, change it here and re-run `cargo run
//! -p voxgen`; never hand-edit the binary output (UI_UX.md §2a point 2).
//!
//! Style rules these follow, from UI_UX.md: a grounded, semi-realistic
//! industrial food-factory look (§3) — stainless tanks, rubber belting,
//! poured concrete, hazard striping, cable/pipe runs — with saturated colour
//! reserved for status readouts rather than spent decoratively (§7), and
//! silhouettes designed to read from a fixed isometric angle (§2).

use crate::palette::Palette;
use crate::vox::{TILE_VOXELS, VoxModel};

/// An asset the pipeline knows how to build.
pub struct Asset {
    pub name: &'static str,
    pub build: fn(&Palette) -> VoxModel,
}

pub const ASSETS: &[Asset] = &[
    Asset { name: "floor_tile", build: floor_tile },
    Asset { name: "belt", build: belt },
    // The two junction-adjacent belt variants (the "Visual overhaul round 2"
    // epic's ticket 3). `main.gd` picks between these three per tile from the
    // simulation's own layout — see `belt_inlet`'s doc.
    Asset { name: "belt_inlet", build: belt_inlet },
    Asset { name: "belt_outlet", build: belt_outlet },
    Asset { name: "machine", build: machine },
    // The kinds `data/machines.ron` names that have a silhouette of their
    // own (the "Machines improvement" milestone, ticket 4). `machine` above
    // stays the fallback for every kind that doesn't — see `main.gd`'s
    // MACHINE_MODELS.
    Asset { name: "machine_crusher", build: machine_crusher },
    Asset { name: "machine_mixer", build: machine_mixer },
    Asset { name: "machine_cooking_vat", build: machine_cooking_vat },
    // The two downstream kinds. They fell back to the generic `machine`
    // above until the "Visual overhaul round 2" epic's ticket 2, which is
    // exactly what the project owner's review of the rendered scene named
    // ("some machines are still missing assets or just reuse assets") —
    // every kind in `data/machines.ron` now has a silhouette of its own.
    Asset { name: "machine_bottler", build: machine_bottler },
    Asset { name: "machine_packer", build: machine_packer },
    // ARCHITECTURE.md §9 milestone 7's machine, and the live bar that reads
    // its progress — a `<model>` + `<model>_fill`-shaped pair, exactly like
    // the hopper's and the tank's (see `aging_rack_gauge`).
    Asset { name: "machine_aging_rack", build: machine_aging_rack },
    Asset { name: "aging_rack_gauge", build: aging_rack_gauge },
    // The chain's exit (the delivery-dock ticket): the tile finished cases
    // leave the world through. Not a processing kind — see `machine_dock`.
    Asset { name: "machine_dock", build: machine_dock },
    Asset { name: "hopper", build: hopper },
    Asset { name: "hopper_fill", build: hopper_fill },
    // The pipe run's supply vessel and the liquid column inside it (the
    // "Visual overhaul round 2" epic: ticket 1 made the tank a real,
    // player-loadable building, this ticket draws it).
    Asset { name: "fluid_tank", build: fluid_tank },
    Asset { name: "fluid_tank_fill", build: fluid_tank_fill },
    Asset { name: "pipe", build: pipe },
    Asset { name: "pipe_source", build: pipe_source },
    Asset { name: "pipe_inlet", build: pipe_inlet },
    Asset { name: "pipe_fluid", build: pipe_fluid },
    Asset { name: "item_tomato", build: item_tomato },
    Asset { name: "item_paste_pail", build: item_paste_pail },
    Asset { name: "item_mix_pail", build: item_mix_pail },
    Asset { name: "item_ketchup_pail", build: item_ketchup_pail },
    Asset { name: "item_bottle", build: item_bottle },
    Asset { name: "item_box", build: item_box },
];

/// Items are modelled at half a tile so a finished bottle reads as an object
/// sitting *on* a belt rather than as another tile-sized machine (UI_UX.md
/// §2's shared-scale rule is about the tile unit, not about everything being
/// tile-sized).
const ITEM_VOXELS: u32 = 16;

fn item_volume() -> VoxModel {
    VoxModel::new([ITEM_VOXELS, ITEM_VOXELS, ITEM_VOXELS])
}

// ---------------------------------------------------------------------------
// Factory floor
// ---------------------------------------------------------------------------

/// Poured-concrete floor slab, one tile. Deliberately only 3 voxels thick:
/// the whole factory floor is tiled with this, so it is the one asset whose
/// triangle count is multiplied by the size of the world.
///
/// The darker perimeter is not decoration — UI_UX.md §1 requires the world
/// to stay "always visually legible as a grid," and a seam at the tile
/// boundary is what actually delivers that at a fixed isometric angle.
pub fn floor_tile(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 3]);
    let last = TILE_VOXELS as i32 - 1;

    m.cuboid((0, last), (0, last), (0, 1), p.idx("concrete_dark"));
    m.cuboid((0, last), (0, last), (2, 2), p.idx("concrete"));

    // Aggregate speckle: a fixed pattern, not randomness, so regenerating
    // the model is byte-stable.
    for y in 0..=last {
        for x in 0..=last {
            // Cheap integer hash rather than a linear expression: a modulo
            // of `x*a + y*b` lays the speckle out in obvious diagonal
            // stripes, which reads as a pattern instead of as aggregate.
            let h = ((x as i64 * 73856093) ^ (y as i64 * 19349663)).rem_euclid(23);
            if h < 2 {
                m.set(x, y, 2, p.idx("concrete_light"));
            }
        }
    }

    // Tile seam.
    for i in 0..=last {
        for (x, y) in [(i, 0), (i, last), (0, i), (last, i)] {
            m.set(x, y, 2, p.idx("floor_seam"));
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Conveyor
// ---------------------------------------------------------------------------

/// One tile of rubber-belt conveyor: steel side frames and legs (shiny),
/// brass rollers (shiny), a matte rubber running surface with cleats.
///
/// The metal/matte split here is the point of UI_UX.md §4 in miniature — the
/// frame and rollers catch the light, the belt rubber does not.
pub fn belt(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    belt_body(&mut m, p);
    m
}

/// The tile immediately **upstream of a machine** — the last belt tile before
/// product is handed over. Distinct from a plain middle tile on purpose: this
/// is the concrete, buildable half of the project owner's *"assets should
/// look different depending on their orientation and what's around them"*,
/// and a belt tile that hands off to a machine genuinely is different kit
/// from one in the middle of a run.
///
/// Which tile this is comes from the simulation's own layout, never from a
/// hardcoded "the front is -X": `main.gd` asks whether the next tile ALONG
/// THE STAGE'S OWN TRAVEL DIRECTION (derived from `chain_layout_json`'s
/// `belt_start_tile` -> `machine_tile`) is covered by a machine footprint.
/// The model itself is authored for a +X run only, which is the one
/// direction the simulation ever produces — see `sim_core/src/sim/world.rs`'s
/// corner note, and `main.gd`'s `_belt_asset_for`, which falls back to the
/// plain tile rather than drawing a wrong-facing end cap if that ever
/// changes.
///
/// What it adds: guide rails converging toward the machine, and a bright
/// dead-plate running out to the tile edge at exactly the height of the
/// machine's own infeed apron ([`BELT_SURFACE_Z`]), so the two are one
/// continuous surface across the boundary instead of two things that stop
/// near each other.
pub fn belt_inlet(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    belt_body(&mut m, p);
    let last = TILE_VOXELS as i32 - 1;
    let (l0, l1) = BELT_LANE;

    // Transition dead-plate: no rollers under it, flush to the tile edge and
    // exactly level with the machine's infeed apron.
    m.cuboid((22, last), (l0, l1), (BELT_SURFACE_Z, BELT_SURFACE_Z), p.idx("steel_light"));
    // Low guide kerbs along it. Deliberately LOW: the first version stood a
    // full hazard-striped portal frame here, and rendered next to the
    // machine's own infeed housing it made the junction two portals deep and
    // a jumble of dark bars. The machine end frames the junction; the belt
    // end should only hand over to it.
    for y in [l0 - 1, l1 + 1] {
        m.cuboid((22, last), (y, y), (BELT_SURFACE_Z, BELT_SURFACE_Z + 1), p.idx("steel_light"));
    }
    m
}

/// The tile immediately **downstream of a machine** (or of the hopper) — the
/// belt tile a discharge lands on. The mirror of [`belt_inlet`], picked the
/// same data-driven way.
///
/// What it adds: a receiving apron at the tile edge under the machine's
/// discharge lip, and taller splash guards along the first stretch, which is
/// the stretch a real line guards because that is where product drops rather
/// than travels.
pub fn belt_outlet(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    belt_body(&mut m, p);
    let (l0, l1) = BELT_LANE;

    // Receiving plate under the machine's discharge lip, then low splash
    // guards along the stretch product lands on rather than travels along.
    // Kept as quiet as `belt_inlet`'s kerbs, and for the same reason.
    m.cuboid((0, 8), (l0, l1), (BELT_SURFACE_Z, BELT_SURFACE_Z), p.idx("steel_light"));
    for y in [l0 - 1, l1 + 1] {
        m.cuboid((0, 8), (y, y), (BELT_SURFACE_Z, BELT_SURFACE_Z + 2), p.idx("steel_light"));
    }
    m
}

/// The conveyor itself, shared by all three belt assets above so a run reads
/// as one line whatever variant a given tile draws.
fn belt_body(m: &mut VoxModel, p: &Palette) {
    let last = TILE_VOXELS as i32 - 1;

    // Legs.
    for (x0, x1) in [(2, 4), (27, 29)] {
        for (y0, y1) in [(5, 7), (24, 26)] {
            m.cuboid((x0, x1), (y0, y1), (0, 4), p.idx("steel_dark"));
        }
    }

    // Side frames, running the full length of the tile so consecutive tiles
    // read as one continuous line.
    for (y0, y1) in [(4, 6), (25, 27)] {
        m.cuboid((0, last), (y0, y1), (4, 8), p.idx("steel_dark"));
    }

    // Hazard striping on the outer face of each frame (UI_UX.md §3).
    m.hazard_stripe_x((0, last), (4, 4), (5, 6), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((0, last), (27, 27), (5, 6), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));

    // Rollers under the belt.
    for x in [4, 12, 20, 28] {
        m.cylinder_y((x, 7), 2.5, (7, 24), p.idx("brass"));
    }

    // Running surface + cleats.
    m.cuboid((0, last), (7, 24), (8, 9), p.idx("rubber"));
    for x in 0..=last {
        if x.rem_euclid(8) < 2 {
            m.cuboid((x, x), (7, 24), (9, 9), p.idx("rubber_dark"));
        }
    }
}

// ---------------------------------------------------------------------------
// Machine
// ---------------------------------------------------------------------------

/// One generic processing unit — concrete plinth, painted housing, stainless
/// vessel with banded seams and a domed head, exhaust stack, front control
/// panel with emissive status lamps.
///
/// Deliberately *one* shape reused for every stage (cooker, reducer,
/// bottler, packer). The chain's stage count is data-driven
/// (data/recipes.ron), so per-machine silhouettes would mean an art asset
/// per recipe — a content decision for a later milestone, not a prerequisite
/// for the visual pass. Distinguishing them visually is a follow-up.
pub fn machine(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let last = TILE_VOXELS as i32 - 1;

    // Plinth. Flush with the tile's own edges (0..last), not inset from
    // them — a belt tile's own frame already runs edge-to-edge (see
    // `belt()`'s doc), and a machine plinth that stopped a voxel short left
    // a hairline seam of bare floor at every belt/pipe-to-machine junction,
    // reading as "these don't actually connect" from the fixed isometric
    // angle where SSAO darkens exactly that kind of step (the visual-
    // overhaul pass's root-cause fix for the connection issue).
    m.cuboid((0, last), (0, last), (0, 1), p.idx("concrete_dark"));
    // Hazard skirt around the plinth top edge.
    m.hazard_stripe_x((0, last), (0, 0), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((0, last), (last, last), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for y in 0i32..=last {
        let a = if (y + 2).rem_euclid(6) < 3 { p.idx("hazard_yellow") } else { p.idx("hazard_black") };
        m.set(0, y, 2, a);
        m.set(last, y, 2, a);
    }

    // Painted housing with steel corner posts.
    m.cuboid((3, 28), (3, 28), (2, 13), p.idx("paint_blue"));
    for (x0, x1) in [(3, 4), (27, 28)] {
        for (y0, y1) in [(3, 4), (27, 28)] {
            m.cuboid((x0, x1), (y0, y1), (2, 14), p.idx("steel_dark"));
        }
    }
    // Machined rim the vessel sits on.
    m.cuboid((3, 28), (3, 28), (14, 15), p.idx("steel"));

    // Stainless vessel.
    m.cylinder_z((16, 16), 9.5, (15, 25), p.idx("steel"));
    for z in [18, 22] {
        m.cylinder_z((16, 16), 10.0, (z, z), p.idx("steel_light"));
    }
    // Domed head.
    m.ellipsoid((16, 16, 25), (9.5, 9.5, 4.0), p.idx("steel_light"));

    // Exhaust stack and a short outlet pipe.
    m.cylinder_z((25, 8), 2.0, (25, 31), p.idx("pipe_grey"));
    m.cylinder_y((8, 20), 2.0, (2, 12), p.idx("pipe_grey"));

    // Front control panel (the -Y face is the camera-facing one, see main.gd).
    m.cuboid((10, 21), (2, 2), (5, 12), p.idx("steel_light"));
    m.cuboid((12, 19), (1, 1), (7, 11), p.idx("hazard_black"));

    // Emissive status lamps — saturated colour reserved for meaning (§7).
    m.cuboid((6, 8), (1, 2), (9, 11), p.idx("status_green"));
    m.cuboid((23, 25), (1, 2), (9, 11), p.idx("status_amber"));
    // A thin lit strip under the vessel rim, so the machine reads as
    // powered from any of the four snap angles, not just the front.
    m.cylinder_z((16, 16), 10.0, (16, 16), p.idx("status_amber"));

    // The fallback kind gets the junction hardware too (see the "Junctions"
    // section): a machine kind added to `data/machines.ron` with no bespoke
    // model must still show a belt where its product goes in and comes out,
    // or the fallback would reintroduce the exact "items vanish into a wall"
    // problem this ticket exists to remove.
    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, last);
    m
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

pub fn item_tomato(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    m.ellipsoid((8, 8, 6), (5.0, 5.0, 4.5), p.idx("tomato_red"));
    // Shaded underside so the sphere reads as round at a fixed camera angle.
    m.ellipsoid((8, 8, 4), (4.6, 4.6, 2.2), p.idx("tomato_shadow"));
    // Calyx + stem.
    for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)] {
        m.set(8 + dx, 8 + dy, 10, p.idx("tomato_leaf"));
    }
    m.cuboid((8, 8), (8, 8), (11, 12), p.idx("tomato_leaf"));
    m
}

/// The chain's three bulk liquid stages, in belt order: crushed
/// **tomato paste**, the **ketchup mix** the vinegar goes into, and the
/// cooked-down **ketchup** itself.
///
/// **All three are open pails of liquid** (the "Visual overhaul round 2"
/// epic's ticket 2). The project owner's review of the rendered scene said
/// they "should all be liquid" and they did not read that way: the old pail
/// capped its own mouth with a flat disc of a *matte* colour, so what a
/// player actually saw travelling down the belt was a painted lid. Two
/// changes fix it, and neither is a repaint. The pail is now genuinely
/// **hollow**, with the product surface sitting a voxel below the rolled
/// rim, so there is a band of inner wall above it and the eye reads "into a
/// container" rather than "at a block". And the surface itself takes
/// `palette.ron`'s `liquid` class — the glossy dielectric milestone 6 added
/// for exactly this ("visible liquid surfaces in tanks/pipes with subtle
/// shading/reflection", UI_UX.md §4) — so it catches a highlight the matte
/// body around it cannot.
///
/// The three keep distinct *body* liveries as well as distinct liquids,
/// which is deliberate: the fixed isometric camera looks down, so the liquid
/// surface is the largest thing it sees, but at scene scale two reds a shade
/// apart are not a reliable difference on their own. Body colour carries the
/// coarse "which stage is this", liquid colour carries the fine one.
///
/// This one is `item_paste_pail`, **renamed** from `item_sauce_pail`: that
/// name still referred to `tomato_sauce`, an item id the "Machines
/// improvement" epic's ticket 2 deleted. A presentation-side name that
/// silently outlives the content id it describes is exactly the failure
/// UI_UX.md §2b point 10 records, so it is worth not keeping one around.
pub fn item_paste_pail(p: &Palette) -> VoxModel {
    pail(p, "pail_grey", "liquid_paste", "brass")
}

/// The uncooked ketchup mix, between the mixer and the cooking vat (the
/// "Machines improvement" milestone's ticket 2 added this stage). A third
/// livery rather than a reuse: it sits on the belt immediately upstream of
/// finished ketchup, so if the two shared a model the chain's longest and
/// most visible process would look like it changed nothing. Its liquid is
/// the lightest and least saturated of the three — paste cut with vinegar,
/// before the cook concentrates it.
pub fn item_mix_pail(p: &Palette) -> VoxModel {
    pail(p, "paint_blue", "liquid_mix", "brass")
}

/// The next intermediate (ketchup) — same silhouette, different livery, so
/// the bulk stages stay distinguishable on the belt. Its liquid is the
/// classic ketchup red the bottled product carries downstream.
pub fn item_ketchup_pail(p: &Palette) -> VoxModel {
    pail(p, "paint_red", "liquid_ketchup", "steel_light")
}

/// A food-grade pail with a bail handle, holding a visible body of liquid.
///
/// Order matters throughout, and the carve is the whole trick: the walls are
/// drawn solid, hollowed out, and only then is the rim ring laid on and the
/// product surface dropped into the cavity. Drawing the rim as a disc (what
/// this used to do) caps the pail; filling the cavity to the rim hides that
/// it is a cavity at all. `contents` must name a `liquid`-class colour — the
/// gloss is the point, see the callers above.
fn pail(p: &Palette, body: &str, contents: &str, handle: &str) -> VoxModel {
    let mut m = item_volume();
    // Walls and base, then hollowed from the inside so the pail has a real
    // interior for the product to sit in.
    m.cylinder_z((8, 8), 5.0, (0, 9), p.idx(body));
    m.carve_cylinder_z((8, 8), 3.8, (2, 10));
    // Rolled rim: a RING, not a disc — drawn wide, then re-opened.
    m.cylinder_z((8, 8), 5.6, (9, 9), p.idx(body));
    m.carve_cylinder_z((8, 8), 4.0, (9, 9));
    // The product surface, one voxel below the rim, so a band of inner wall
    // stands above it and the liquid reads as being *in* something. Two
    // voxels thick so the surface still meshes if a viewer catches it from
    // a shallow angle.
    m.cylinder_z((8, 8), 3.8, (7, 8), p.idx(contents));
    // Bail handle.
    m.cuboid((3, 3), (8, 8), (10, 12), p.idx(handle));
    m.cuboid((13, 13), (8, 8), (10, 12), p.idx(handle));
    m.cuboid((3, 13), (8, 8), (13, 13), p.idx(handle));
    m
}

pub fn item_bottle(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    m.cylinder_z((8, 8), 4.0, (0, 7), p.idx("ketchup_red"));
    m.cylinder_z((8, 8), 4.2, (2, 4), p.idx("label_cream"));
    // Shoulder tapers into a distinctly narrower neck — without the gap the
    // cap sits straight on the body and the silhouette reads as a jar.
    m.ellipsoid((8, 8, 8), (4.0, 4.0, 2.0), p.idx("ketchup_red"));
    m.cylinder_z((8, 8), 1.6, (10, 12), p.idx("glass_pale"));
    m.cylinder_z((8, 8), 2.4, (13, 14), p.idx("cap_white"));
    m
}

pub fn item_box(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    m.cuboid((2, 13), (2, 13), (0, 10), p.idx("cardboard"));
    // Taped seam over the top and down the front.
    m.cuboid((7, 8), (2, 13), (10, 10), p.idx("cardboard_dark"));
    m.cuboid((7, 8), (2, 2), (0, 10), p.idx("cardboard_dark"));
    // Shipping label.
    m.cuboid((3, 6), (2, 2), (4, 8), p.idx("label_cream"));
    m.cuboid((9, 12), (2, 2), (4, 8), p.idx("label_cream"));
    m
}

// ---------------------------------------------------------------------------
// Fluid pipes (milestone 6 — ARCHITECTURE.md §3.4, §9 item 6)
// ---------------------------------------------------------------------------
//
// Material call, made deliberately rather than left ambiguous: this run is
// **stainless steel, material class `metal`** (`pipe_grey`/`steel_*`/
// `brass`), not PVC. Two reasons, both from the docs rather than taste.
// UI_UX.md §3 lists "stainless steel tanks ... PVC/steel piping" as the
// setting's visual language and §4 makes metals the deliberate shiny
// exception so "important machinery pops" — a pipe run feeding a machine is
// exactly that, and a matte plastic run would visually sink into the
// concrete floor it stands on. And GAMEPLAY.md §2/§4 reserve non-standard
// piping material as a *later* gate (tier 5's acid-resistant equipment), so
// the baseline run should read as ordinary food-grade stainless.
//
// The pipe is hollow, with an open sight section cut through the top of its
// middle span. That opening is what makes UI_UX.md §4's "visible liquid
// surfaces in tanks/pipes" literally true here: the fluid body
// (`pipe_fluid`, below) sits in the bore and is seen through it from the
// fixed isometric angle, which looks down on the +Y (model +Z) face.

/// Voxel geometry shared by every pipe tile, so a plain segment, the tank
/// connection and the machine inlet read as one continuous run. Numbers here
/// are mirrored by `main.gd`'s PIPE_* constants (stated in voxels, converted
/// there) — change one, change the other.
const PIPE_AXIS_Y: i32 = 16;
const PIPE_AXIS_Z: i32 = 15;
const PIPE_OUTER_R: f32 = 4.5;
const PIPE_BORE_R: f32 = 3.0;
/// Bore floor / ceiling in voxels, derived from the two above. The fluid
/// body's base sits on `PIPE_BORE_Z0` and it can rise `PIPE_BORE_HEIGHT`.
const PIPE_BORE_Z0: i32 = PIPE_AXIS_Z - 3;
const PIPE_BORE_HEIGHT: i32 = 7;
/// Bore half-width, in Y — the fluid body is this wide too.
const PIPE_BORE_Y0: i32 = PIPE_AXIS_Y - 3;
const PIPE_BORE_Y1: i32 = PIPE_AXIS_Y + 3;
/// The sight section: where the wall is cut away so the liquid inside can
/// actually be seen. See `pipe_body` for why it is a short framed window
/// rather than the full-length open channel it used to be.
const PIPE_WINDOW: (i32, i32) = (11, 20);

/// Draws the steel run itself into `m`, with its sight opening spanning
/// `window` in X. Everything is carved *after* it is drawn: a face is only
/// culled when its neighbour is genuinely absent, so a bore filled with an
/// "air" colour would still mesh as a solid plug (see `VoxModel::erase`).
///
/// **This is a redesign** (the "Visual overhaul round 2" epic's ticket 2 —
/// the project owner's review said "the pipes look kind of weird", and
/// rendering the whole scene rather than the model alone showed exactly
/// why). Three things were wrong, all of them only visible at scene scale:
///
/// 1. **Ring clutter.** Every tile carried a wide flange at *both* ends plus
///    two brass collars mid-span, so a join between two tiles stacked four
///    voxels of oversized ring and the run read as a row of bobbins rather
///    than as one pipe. There is now one flange, at the tile's low-X end
///    only, so a join shows exactly one — and the collars are thin, in the
///    same steel as the run, and used for one job: framing the window.
///    Brass is gone from the run entirely; it survives only on
///    `pipe_source`'s hand wheel and `pipe_inlet`'s branch, where a single
///    gold accent marks a *fitting a player operates or a machine draws
///    from* rather than decorating every tile.
/// 2. **The sight opening ate the silhouette.** Cutting the top wall away
///    across 14 of 32 voxels left a half-open trough with two rails, which
///    from the fixed isometric angle does not read as a tube at all. It is
///    now a short framed window (`PIPE_WINDOW`) — an in-line sight glass,
///    which is a real thing on a real food line — so most of each tile is
///    closed pipe and the run has a continuous round back to read along.
///    The window still starts just above the bore floor, which is the part
///    of the previous design that was right and is kept: a level below half
///    fill is exactly the state a player most needs to see.
/// 3. **The trestle was heavier than the pipe.** Two dark four-voxel legs
///    per tile out-massed the run they carried. One centred stand per tile
///    now does it, with a base plate and a saddle, which also spaces the
///    supports one tile apart like a real pipe rack.
fn pipe_body(m: &mut VoxModel, p: &Palette, window: (i32, i32)) {
    // Support stand: one per tile, centred, with a footplate and a saddle
    // the run sits in. `steel`, NOT `steel_dark`: at scene scale the fixed
    // isometric camera sees these posts edge-on and a dark metal facing away
    // from the key light renders very nearly black, so the first version's
    // dark stands were visually heavier than the pipe they carried — the
    // stand read and the run did not.
    m.cuboid((13, 18), (13, 18), (0, 0), p.idx("steel"));
    m.cuboid((14, 17), (15, 16), (1, 8), p.idx("steel"));
    m.cuboid((13, 18), (12, 19), (9, 10), p.idx("steel"));

    // The run. Full tile length, so consecutive tiles butt into one line.
    //
    // `steel_light` — the BRIGHTEST metal in the palette, and the flange
    // below is the darker one, which is the reverse of how a fitting is
    // usually picked out. That inversion is deliberate and was arrived at
    // by rendering the factory rather than the model: a pipe is a long thin
    // horizontal object seen side-on, so almost all of it is curved wall
    // facing away from the key light, and in `steel` it simply disappeared
    // between the belt beside it and the floor under it. Making the *tube*
    // the bright, continuous thing is what lets the eye follow the run from
    // the tank to the mixer, which is the one job this asset has. (An
    // earlier pass had already ruled out the darker `pipe_grey` here for a
    // milder version of the same reason.)
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R, (0, TILE_VOXELS as i32 - 1), p.idx("steel_light"));
    // One bolted flange, at the low-X end only — so a join between two
    // tiles reads as a single fitting instead of two stacked ones, and so
    // there is exactly ONE bright ring per tile. An earlier cut also put
    // thin collars either side of the sight window; rendered in the factory
    // that put three pale rings on every tile and the run went straight
    // back to reading as a row of bobbins, which is the exact fault this
    // redesign exists to remove. The window's own cut edges mark it well
    // enough without them.
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 1.0, (0, 1), p.idx("steel"));

    // Hollow it out, then cut the sight window through the top wall,
    // leaving the side rails standing. The cut starts just above the bore
    // FLOOR, not at the crown: cutting only the top half was the first
    // version of this and it hid the liquid below roughly half fill, which
    // is exactly the state a player most needs to read (a draining pipe
    // means a stalling machine).
    let top = m.size[2] as i32 - 1;
    let window_floor = PIPE_BORE_Z0 + 1;
    m.carve_cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_BORE_R, (0, TILE_VOXELS as i32 - 1));
    m.carve_cuboid(window, (PIPE_BORE_Y0, PIPE_BORE_Y1), (window_floor, top));
}

/// A plain length of pipe: one tile of the run.
pub fn pipe(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    pipe_body(&mut m, p, PIPE_WINDOW);
    m
}

/// The run's **supply end** — `PipeSegment::is_source` in the simulation,
/// which since the "Visual overhaul round 2" epic's ticket 1 means only
/// "the tile the fluid tank discharges into", not "an off-map tap that is
/// always full".
///
/// So this asset is a *connection*, not a vessel. It used to carry a
/// stainless header vessel of its own, which made sense when the supply
/// genuinely came from nowhere — but the vessel is now a real building
/// standing on the neighbouring tile (`fluid_tank`), and drawing a second
/// one here would say there are two. What marks this tile instead is the
/// hardware a real supply connection has and the rest of the run doesn't: a
/// heavy inlet flange on the **-X** face (the side the tank stands on —
/// `World::with_machines` places the tank one tile west of `segments[0]`)
/// and an isolation valve with a hand wheel standing above the run.
///
/// That keeps UI_UX.md §7's "never encode state in colour alone" satisfied
/// by silhouette, which is what the previous vessel was doing too — this
/// version just stops duplicating a building that now exists.
pub fn pipe_source(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 32]);
    // Window shifted downstream so the valve doesn't stand over it.
    pipe_body(&mut m, p, (22, 29));

    // Heavy inlet flange facing the tank, wider than a plain tile join so
    // the supply end is identifiable from any of the four snap angles — and,
    // since the "Visual overhaul round 2" epic's ticket 3, in the palette's
    // brightest metal: this is one half of the tank-to-run joint (the tank's
    // discharge spout is the other), and the pair was previously dark grey
    // hardware on a dark grey shell in the one place a player needs to see
    // that the tank is actually plumbed into the line.
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 2.0, (0, 2), p.idx("steel_light"));
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 1.2, (3, 4), p.idx("brass"));
    // Re-open the bore those two rings just plugged. `pipe_body` carves the
    // bore before returning, so anything drawn afterwards fills it back in —
    // and this end is the one the tank actually discharges THROUGH, so a
    // solid plug here is both wrong and something the liquid body (which
    // spans the whole tile) would poke out of. Found by the junction tests
    // at the bottom of this file, not by eye.
    m.carve_cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_BORE_R, (0, 4));

    // Isolation valve: a bonnet straddling the run, a stem, and a brass
    // hand wheel — the one thing on this tile a player would reach for, and
    // the only place brass survives on the run's own hardware (see
    // `pipe_body`'s doc). Moved to mid-tile in ticket 3: it used to stand at
    // x = 7..12, i.e. directly over the tank joint, so the one fitting that
    // says "the tank feeds this run" was photographed behind a valve body.
    m.cuboid((13, 18), (12, 19), (PIPE_AXIS_Z, 22), p.idx("steel_dark"));
    m.cuboid((14, 17), (13, 18), (23, 24), p.idx("steel_light"));
    m.cuboid((15, 16), (15, 16), (25, 27), p.idx("steel_dark"));
    m.cylinder_z((15, 16), 4.0, (28, 28), p.idx("brass"));
    m.carve_cylinder_z((15, 16), 2.0, (28, 28));
    m
}

/// The one pipe segment that actually feeds a machine — `FluidNetwork`'s
/// `inlet_index` (see `world.rs`), always the run's machine-adjacent end.
///
/// **The connection this fixes**, found during the visual-overhaul pass: a
/// plain `pipe` tile's solid header sits mid-tile (its bore is centred on
/// `PIPE_AXIS_Y`, several voxels in from either edge — see `pipe_body`), so
/// the run's last segment and the machine tile beside it left a real gap of
/// open floor between the header's own surface and the machine's inlet
/// nozzle, even though the two tiles are adjacent. This is the fitting that
/// closes it: a branch T-off the header, reaching out to this tile's own
/// edge at the SAME axis height (`PIPE_AXIS_Z`) the machine's inlet nozzle
/// targets (see `machine_mixer`'s vinegar inlet) — no elbow needed, since
/// both tiles sit on the floor and the header's axis height already matches
/// the nozzle height once the two are deliberately kept in sync.
///
/// Which tile edge: **high local Y** (`last`), not low. `main.gd` places
/// tiles at `(sim_x, sim_y) -> world (sim_x, sim_y) * TILE`, and the vox
/// importer maps a model's low-Y face to the *higher*-world-Z-facing side of
/// the tile (see `machine_mixer`'s own doc for the derivation, applied there
/// to reach its north neighbour). The inlet pipe segment's machine neighbour
/// sits at the *lower* sim-y (south of it — `world.rs` puts the pipe run
/// north of the machine it feeds), so it's this tile's *high*-Y face that
/// borders the machine, the mirror image of the mixer's own -Y-facing stub.
pub fn pipe_inlet(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    pipe_body(&mut m, p, PIPE_WINDOW);

    let last = TILE_VOXELS as i32 - 1;
    // Branch stub + flange, reaching from inside the header's own bore
    // (well past its outer surface, so the two solids genuinely overlap
    // rather than merely touching) out to the tile edge the machine sits
    // against.
    //
    // **Sized and coloured to match `machine_mixer`'s stub exactly** (the
    // "Visual overhaul round 2" epic's ticket 3). The first version was a
    // 2.2-radius `pipe_grey` stub meeting a 2.2-radius `pipe_grey` stub, and
    // rendering the junction showed the pair as an indistinct dark lump
    // between a bright pipe and a blue machine skirt rather than as one
    // spool piece crossing a tile boundary. Bright steel with a brass flange
    // right on the boundary is what makes the two halves read as one
    // fitting; the flange sits at the boundary on BOTH sides, so the joint
    // shows a single brass collar rather than two rings a voxel apart.
    m.cylinder_y((16, PIPE_AXIS_Z), 3.0, (16, last), p.idx("steel_light"));
    m.cylinder_y((16, PIPE_AXIS_Z), 4.0, (last - 1, last), p.idx("brass"));

    // **A blind flange closing the run's downstream end.** The run used to
    // simply stop: `pipe_body` bores every tile through from x = 0 to x = 31,
    // so the last segment's +X face was an open, unterminated tube mouth —
    // camera-facing, and one of the most obviously wrong things in the whole
    // factory once a close-up was actually rendered. A pipe run ends in a
    // blank flange, so this one does.
    // Bright plate with a brass ring on it, NOT plain `steel`: the end face
    // is a vertical surface the camera looks straight at and the key light
    // does not reach, so the first cut rendered as an unreadable black stump
    // capping the run — the same lesson `pipe_body` records about the tube
    // itself.
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 1.0, (last - 1, last), p.idx("steel_light"));
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_BORE_R + 0.5, (last, last), p.idx("brass"));
    m
}

/// The fluid itself — one tile of liquid sitting in the bore.
///
/// Deliberately **one voxel tall**: `main.gd` scales this node vertically to
/// the segment's live fill fraction, and a one-voxel slab scaled to N voxels
/// is geometrically the same box as an N-voxel column, with none of the
/// pivot arithmetic a taller model would need (the encoder puts a model's
/// origin at its footprint centre *at ground level*, so a slab's base is
/// exactly its node origin — see `vox::encode`). Near-white on purpose: the
/// per-fluid colour is live simulation data, applied in Godot as an albedo
/// tint over these vertices, so this one model serves every fluid
/// `data/recipes.ron` ever names.
pub fn pipe_fluid(p: &Palette) -> VoxModel {
    // `main.gd` mirrors PIPE_BORE_Z0/PIPE_BORE_HEIGHT to place and scale
    // this slab, so the one thing worth asserting here is that a full
    // column still fits inside the run's wall rather than poking out of it.
    debug_assert!(
        PIPE_BORE_Z0 + PIPE_BORE_HEIGHT <= PIPE_AXIS_Z + PIPE_OUTER_R as i32,
        "a full fluid column would rise above the pipe wall"
    );
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 1]);
    let last = TILE_VOXELS as i32 - 1;
    m.cuboid((0, last), (PIPE_BORE_Y0, PIPE_BORE_Y1), (0, 0), p.idx("fluid_pale"));
    m
}

// ---------------------------------------------------------------------------
// Real machines (the "Machines improvement" milestone, ticket 4)
// ---------------------------------------------------------------------------
//
// `machine()` above is one generic silhouette reused for every stage, and its
// own doc comment called distinguishing them "a follow-up." This is that
// follow-up for the three front-end kinds ticket 2 made real (crusher, mixer,
// cooking vat) plus ticket 3's hopper. The two downstream kinds (bottler,
// packer) deliberately keep the generic model — this ticket is about making
// what tickets 1-3 built *visible*, not about drawing every kind in the file.
//
// MATERIAL-CLASS CALL, stated once for all four rather than re-argued per
// asset (UI_UX.md §4: matte baseline, metals the deliberate shiny
// exception). The rule these follow is **food contact is stainless, structure
// is painted or poured**:
//
//   * everything the product actually touches — crushing rolls, mixing
//     vessel and agitator, the vat's body and dome, the hopper's funnel and
//     discharge chute — is `metal`, because in a real food plant it is
//     stainless for exactly that reason, and because §4 wants the machinery
//     to be the thing that pops against the floor;
//   * everything that only holds those parts up — concrete plinths, painted
//     housings/skirts, hazard striping — is `matte`, which is what gives the
//     metal something to pop *against*. An all-steel machine reads as flat
//     as an all-concrete one.
//   * status lamps stay `emissive`, same language `machine()` established,
//     so every kind still reports running/idle the same way (§7).
//
// Each of these grows from the same footing (plinth + hazard skirt) as
// `machine()` on purpose: they should read as members of one factory, not as
// four unrelated props.

/// Plinth, hazard skirt and painted base skirt, shared by every machine
/// below so the family reads as one factory. `span` is the inclusive
/// footprint the plinth covers — callers pass the full tile/footprint
/// extent (e.g. `(0, TILE_VOXELS - 1)`), not an inset one: a plinth that
/// stops short of its own tile edge leaves a hairline gap of bare floor
/// where a belt or pipe tile butts up against it, which is exactly the
/// "doesn't visually connect" seam the visual-overhaul pass fixed (see
/// `machine()`'s matching note above) — `belt()` and `pipe_body()` already
/// run their own frames flush to the tile boundary, so machines now match
/// them instead of being the one thing inset from it.
/// `skirt_top` is how high the painted base rises.
fn machine_footing(m: &mut VoxModel, p: &Palette, span: (i32, i32), skirt_top: i32, paint: &str) {
    let (lo, hi) = span;
    m.cuboid((lo, hi), (lo, hi), (0, 1), p.idx("concrete_dark"));
    // Hazard skirt around all four edges of the plinth top (UI_UX.md §3).
    m.hazard_stripe_x((lo, hi), (lo, lo), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((lo, hi), (hi, hi), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for y in lo..=hi {
        let a = if (y + 2).rem_euclid(6) < 3 { p.idx("hazard_yellow") } else { p.idx("hazard_black") };
        m.set(lo, y, 2, a);
        m.set(hi, y, 2, a);
    }

    let (blo, bhi) = (lo + 2, hi - 2);
    m.cuboid((blo, bhi), (blo, bhi), (2, skirt_top), p.idx(paint));
    for (x0, x1) in [(blo, blo + 1), (bhi - 1, bhi)] {
        for (y0, y1) in [(blo, blo + 1), (bhi - 1, bhi)] {
            m.cuboid((x0, x1), (y0, y1), (2, skirt_top + 1), p.idx("steel_dark"));
        }
    }
    // Machined rim the vessel/throat sits on.
    m.cuboid((blo, bhi), (blo, bhi), (skirt_top + 1, skirt_top + 2), p.idx("steel"));
}

/// Control panel plus its two status lamps, on the **-Y face** — the one the
/// fixed isometric camera looks at (see this module's header and `machine()`).
/// `x` is the panel's span, `z` its height band.
fn control_panel(m: &mut VoxModel, p: &Palette, x: (i32, i32), z: (i32, i32), face_y: i32) {
    let mid = (z.0 + z.1) / 2;
    m.cuboid(x, (face_y, face_y), z, p.idx("steel_light"));
    m.cuboid((x.0 + 2, x.1 - 2), (face_y - 1, face_y - 1), (z.0 + 1, z.1 - 1), p.idx("hazard_black"));
    m.cuboid((x.0 - 5, x.0 - 3), (face_y - 1, face_y), (mid - 1, mid + 1), p.idx("status_green"));
    m.cuboid((x.1 + 3, x.1 + 5), (face_y - 1, face_y), (mid - 1, mid + 1), p.idx("status_amber"));
}

// ---------------------------------------------------------------------------
// Junctions: where a belt actually meets a machine
// ---------------------------------------------------------------------------
//
// The "Visual overhaul round 2" epic's ticket 3. The project owner's live-demo
// review: *"a conveyor belt going into a mixer seems to just go straight into
// the side there is no connecting so it looks like items just disappear into
// the wall"*. The previous pass had already made every footing flush with its
// tile edge, which closed a floor-level crease — but a flush wall is still a
// wall. Nothing on any machine said "product enters HERE", so an item ran to
// the end of its belt and was swallowed by a blank painted skirt.
//
// **The geometric fact that decides the whole design, established by
// rendering the junctions rather than reasoning from the generator.** The
// shipped camera is fixed isometric with view direction `(1, 1, 1)`
// (`main.gd`'s CAMERA_DIRECTION, UI_UX.md §1), so the only vertical faces it
// can ever see are those whose normal points +X or +Z-world. The chain runs
// +X, so:
//
//   * a machine's OUTLET face (+X) is dead-on camera-facing — a doorway cut
//     into it reads immediately, and that is exactly what `machine_outfeed`
//     below cuts;
//   * a machine's INLET face (-X) points *directly away from the camera* and
//     can never be seen at all. Carving a mouth into it — the obvious move —
//     would be invisible in the only view the game ships.
//
// Worse, an inlet channel sunk into the machine's -X edge can't be saved by
// making it open-topped either: the escape ray from inside such a channel
// travels `(+x, -y, +z)` in vox space, i.e. up and *over the machine's own
// body*, so for anything as tall as the mixer's drum or the vat's shell the
// machine occludes its own inlet. That was checked against real geometry
// before any of this was drawn.
//
// So the inlet is built as a **transfer housing with an open camera-facing
// side**: a hooded steel enclosure standing on the machine's -X edge,
// straddling the belt lane, whose near (vox low-Y, world +Z) wall is cut away
// over a band so the camera looks straight in at a bright apron floor running
// the belt's line and, at the end of it, a black inlet mouth into the machine.
// The item still disappears at the tile boundary — the simulation collects it
// there, and nothing here changes that — but it now disappears *into a piece
// of equipment built to swallow it*, with the path visible, which is what the
// complaint was actually about.
//
// The belt's own half of each junction is `belt_inlet`/`belt_outlet` (see the
// conveyor section above): the last belt tile before a machine and the first
// one after it are genuinely different pieces of kit from a middle tile, and
// now look it. `main.gd` picks them from the layout the simulation reports,
// never from a hardcoded "the front is -X".

/// The belt's running lane in voxels, mirrored from [`belt`] — the Y span its
/// rubber surface occupies. Every junction piece on both sides of a tile
/// boundary is built against this one number, which is what makes a belt's
/// line continue into a machine instead of merely stopping near it.
pub const BELT_LANE: (i32, i32) = (7, 24);
/// Top voxel of that running surface, likewise mirrored from [`belt`]. An
/// apron drawn at this Z is exactly level with the belt beside it.
pub const BELT_SURFACE_Z: i32 = 9;
/// How far into the machine the infeed housing reaches, and how far back
/// from its +X edge the outfeed doorway is cut. Eight voxels is a quarter of
/// a tile: deep enough to read as a structure, shallow enough that it never
/// eats the silhouette the machine is identified by (the mixer's drum starts
/// at x = 4, the vat's shell is inset further still).
const JUNCTION_DEPTH: i32 = 8;
/// Roof height of the infeed housing. Low enough to read as a hood over a
/// transfer point rather than as a room: the first cut ran it to 21 and the
/// rendered junction was a big empty portal.
const JUNCTION_ROOF_Z: i32 = 18;

/// The infeed half of a junction: a hooded transfer housing on the machine's
/// **-X** edge, spanning `lane`, with its camera-facing side open so the
/// product path inside is visible. See this section's header for why an open
/// side rather than an open mouth or an open top.
///
/// `lane` is the belt lane in this model's own Y coordinates. For a 1x1
/// machine that is [`BELT_LANE`] unchanged; for a multi-tile machine the
/// caller offsets it to whichever half of the model the belt's row maps to
/// (the vox importer maps low local Y to high world Z, so a 2x2 machine's
/// sim-y = 0 row is its *upper* local-Y half — see [`machine_cooking_vat`]).
fn machine_infeed(m: &mut VoxModel, p: &Palette, lane: (i32, i32)) {
    let (l0, l1) = lane;
    let d = JUNCTION_DEPTH;
    let floor = BELT_SURFACE_Z;
    let roof = JUNCTION_ROOF_Z;

    // Housing shell, then hollowed. Drawn solid first and carved after, per
    // the rule the pipes already follow: a face is only culled when its
    // neighbour is genuinely absent, so an "opening" has to really be empty
    // (see `VoxModel::erase`). Carving through whatever the machine itself
    // had here is deliberate — this housing IS the machine's inlet, not a
    // box parked next to one.
    m.cuboid((0, d), (l0 - 2, l1 + 2), (floor, roof), p.idx("steel_light"));
    m.carve_cuboid((0, d - 1), (l0, l1), (floor + 1, roof - 2));

    // The camera-facing side, cut away from just above the apron to just
    // under the roof. This window is the whole reason the piece is shaped
    // this way: the machine's -X mouth can never be seen (this section's
    // header derives that), so the product path is shown from the side
    // instead.
    m.carve_cuboid((0, d - 1), (l0 - 2, l0 - 1), (floor + 1, roof - 2));

    // What the window shows: **darkness, framed by bright steel, with a
    // brass threshold at the lip**. Getting this right took three renders
    // and one piece of geometry worth writing down. A sight line entering
    // this window travels `(-x, +y, -z)` in vox space, so everything it can
    // reach is the channel FLOOR between the sill and about ten voxels in —
    // never the far wall, and never the back of the channel. Painting a dark
    // mouth at the inner end therefore did nothing at all, and a bright
    // apron (the second cut) filled the whole opening with pale metal, so
    // the finished piece read as an empty portal frame with a lit box behind
    // it. The floor is what is seen, so the floor is what has to be black.
    m.cuboid((0, d - 1), (l0, l1), (floor, floor), p.idx("hazard_black"));
    m.cuboid((0, d - 1), (l1 + 1, l1 + 2), (floor + 1, roof - 2), p.idx("hazard_black"));
    m.cuboid((d - 2, d), (l0, l1), (floor, roof - 2), p.idx("hazard_black"));
    // Brass threshold on the lip, where the belt's dead-plate hands over —
    // the one bright thing inside the opening, and the same accent every
    // other "product passes through here" fitting in the factory carries.
    m.cuboid((0, 1), (l0, l1), (floor, floor), p.idx("brass"));

    // Window head: one yellow band, because a covered nip point on a real
    // line is marked, and because it separates the housing's bright roof
    // from its bright wall. NOT a full hazard stripe — a stripe is half
    // black, and black on the piece's only camera-facing edge turned the
    // whole hood into a dark bar in the first render.
    m.cuboid((0, d), (l0 - 2, l0 - 2), (roof - 1, roof - 1), p.idx("hazard_yellow"));
    // Corner posts either side of the window (the window carve above cuts
    // through the near wall, so these are redrawn after it): the roof has to
    // visibly stand on something.
    for x in [0, d] {
        m.cuboid((x, x), (l0 - 2, l0 - 2), (floor, roof), p.idx("steel_dark"));
    }
}

/// The outfeed half: a discharge housing standing proud of the machine's
/// **+X** face — the one vertical face the fixed isometric camera looks
/// straight at — with a real mouth carved through it and a lip reaching the
/// tile edge, so the belt starts under a chute instead of against a blank
/// painted slab.
///
/// This one can be a plain doorway precisely because of the camera geometry
/// in this section's header: an opening on +X is dead-on visible, so it
/// needs no side window and no trick. It protrudes rather than being sunk
/// deep into the machine so it never eats the silhouette the machine is
/// identified by — the mixer's belt-driven agitator and the bottler's gantry
/// both live within a few voxels of that face.
///
/// `lane` as in [`machine_infeed`]; `last` is the model's highest X index
/// (31 for a one-tile machine, 63 for the 2x2 vat).
fn machine_outfeed(m: &mut VoxModel, p: &Palette, lane: (i32, i32), last: i32) {
    let (l0, l1) = lane;
    let floor = BELT_SURFACE_Z;
    let head = floor + 8;

    // Housing, then the mouth carved through it. `steel_light` for the same
    // reason the infeed's is: the opening reads by contrast against the
    // frame, so the frame has to be the palette's brightest metal and the
    // hole has to be genuinely black.
    m.cuboid((last - 3, last), (l0 - 2, l1 + 2), (floor, head), p.idx("steel_light"));
    m.carve_cuboid((last - 2, last), (l0, l1), (floor + 1, head - 2));
    // The back of the mouth. Its normal is +X, i.e. straight at the camera,
    // which is what turns a cut-out silhouette into a hole with depth.
    m.cuboid((last - 3, last - 3), (l0, l1), (floor, head - 2), p.idx("hazard_black"));

    // Discharge lip, level with the belt beside it and running out to the
    // tile edge so there is nothing between the two.
    m.cuboid((last - 3, last), (l0, l1), (floor, floor), p.idx("steel_light"));
    // A brass band across the head of the opening — the same accent
    // `pipe_source`'s hand wheel and `pipe_inlet`'s branch flange carry, and
    // for the same reason: it marks a fitting product actually passes
    // through. A hazard stripe was tried here first and rendered as a dark
    // bar, because half a stripe is black and this edge is the only part of
    // the piece the camera looks straight at.
    m.cuboid((last - 2, last), (l0 - 1, l1 + 1), (head - 1, head - 1), p.idx("brass"));
}

/// GAMEPLAY.md §2 tier 1's raw-ingredient prep, `data/machines.ron`'s 1x1
/// `crusher`: tomatoes are tipped into the open feed pocket in the top of
/// the frame, two counter-rotating rolls crush them in the nip, and paste
/// leaves by the discharge chute on the +X (downstream) side.
///
/// **Redesigned** for the "Visual overhaul round 2" epic's ticket 2 — the
/// project owner's review said it "looks kind of weird", and rendering the
/// whole factory rather than the model alone showed why: the old version's
/// wide flared lip was, at scene scale, a flat white slab laid over the
/// machine, so what read was a blue box with a tray on it and the rolls
/// underneath collapsed into a single gold stripe.
///
/// The fix that did NOT work, recorded because the next session will
/// otherwise reach for it: replacing the lip with a *converging mouth* — a
/// stack of rings narrowing downward, the inverse of the hopper's discharge
/// cone. It is the obviously-correct shape for a crusher intake and it
/// renders as a ribbed flat plate, because a shallow taper seen from a
/// fixed oblique angle is a plate; it was strictly worse than what it
/// replaced. **The lesson is that this machine cannot be identified from
/// above** — its top is a hole either way, and a hole is not a silhouette.
///
/// So this version identifies it from the **side**, which is where a real
/// roll crusher is unmistakable: a heavy squared frame with two big driven
/// flywheels on the camera-facing face, a belt guard running to the motor,
/// and the rolls visible in an open pocket between them. The flywheels are
/// the whole point — nothing else in this factory has them, so the crusher
/// stops being another housing with something on top.
pub fn machine_crusher(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    machine_footing(&mut m, p, (0, TILE_VOXELS as i32 - 1), 7, "paint_blue");

    // Heavy squared frame — the crusher body proper, deliberately blunt and
    // chunky rather than tapered (see this function's doc).
    m.cuboid((4, 27), (6, 25), (9, 24), p.idx("steel"));
    // Corner posts, so the frame reads as fabricated rather than as one
    // milled block. `steel_light`, not `steel_dark`: rendered in the factory
    // the dark version turned the whole camera-facing side of the machine
    // into one unlit black mass (see the flywheel note below).
    for (x0, x1) in [(4, 6), (25, 27)] {
        m.cuboid((x0, x1), (6, 25), (9, 25), p.idx("steel_light"));
    }
    // Open feed pocket cut down into the top: this is where product goes
    // in, and it is a hole, not a lid.
    m.carve_cuboid((8, 23), (9, 22), (17, 25));
    m.cuboid((7, 24), (8, 23), (24, 24), p.idx("steel_light"));
    m.carve_cuboid((8, 23), (9, 22), (24, 25));
    m.hazard_stripe_x((7, 24), (8, 8), (24, 24), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));

    // The two crushing rolls, drawn AFTER the pocket is carved so they
    // stand in it. Bright steel against brass so the pair reads as two
    // counter-rotating rolls rather than as one drum — dark steel was the
    // first choice and, in the rendered scene, sank into the frame's own
    // shadow so only one roll was legible.
    m.cylinder_y((12, 16), 4.0, (9, 22), p.idx("steel_light"));
    m.cylinder_y((20, 16), 4.0, (9, 22), p.idx("brass"));

    // The identity of the machine: two big driven flywheels standing proud
    // of the camera-facing (-Y) face, on the roll shafts.
    //
    // Bright rim, dark web, brass hub — and the brightness is not a taste
    // call. The first cut made the wheels `steel_dark` and spanned a full
    // guard plate across the -Y face between them; rendered in the actual
    // factory (not as a preview, where it looked fine) that plate was a
    // 28x15 unlit slab and the machine read as a black wall with the
    // flywheels invisible inside it. The fixed isometric camera lights the
    // tops of things and leaves the vertical faces it looks AT in shadow,
    // so anything that has to be legible on a camera-facing face has to
    // carry its own albedo. The guard is now a narrow strip along the top
    // of the wheels only.
    for x in [12, 20] {
        m.cylinder_y((x, 16), 7.0, (3, 4), p.idx("steel_light"));
        m.cylinder_y((x, 16), 5.0, (2, 3), p.idx("steel_dark"));
        m.cylinder_y((x, 16), 1.8, (2, 5), p.idx("brass"));
    }
    m.cuboid((5, 27), (3, 5), (23, 24), p.idx("steel_light"));

    // Drive motor. It used to sit against the -X side across the whole belt
    // lane (x 0..4, y 9..22) — i.e. exactly where the belt arrives, so the
    // first junction in the factory was a conveyor running into the side of
    // a motor. It now stands on the far (+Y) shoulder, clear of the lane, so
    // the infeed housing below has somewhere to be.
    m.cuboid((0, 5), (25, 30), (10, 19), p.idx("paint_blue"));
    m.cuboid((0, 5), (25, 30), (20, 20), p.idx("steel_light"));
    m.cylinder_y((3, 15), 2.4, (23, 25), p.idx("brass"));

    // The junction hardware, shared with every other machine (see the
    // "Junctions" section): product enters through the hooded transfer
    // housing on -X and leaves through a real doorway on +X. The old
    // hand-built stepped chute this replaces was the only outfeed in the
    // factory, and it was a different shape from nothing at all — which is
    // what every other machine had.
    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, TILE_VOXELS as i32 - 1);

    control_panel(&mut m, p, (11, 20), (3, 6), 1);
    m
}

/// `data/machines.ron`'s 1x1 `mixer`: an open-topped stainless blending
/// vessel with a visible cross-paddle agitator on a driven shaft, plus the
/// vinegar inlet stub for the fluid run.
///
/// The inlet points **-Y**, which is not arbitrary: `World::new` puts the
/// pipe run on the row immediately north of the machine it feeds, and sim +Y
/// maps to world +Z, which the `.vox` importer reaches through vox -Y — so
/// -Y is literally the side the pipes are on. The stub therefore lines up
/// with the real run instead of pointing at empty floor.
pub fn machine_mixer(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    machine_footing(&mut m, p, (0, TILE_VOXELS as i32 - 1), 8, "paint_blue");

    // Vessel: a hollow open-topped drum, so the agitator inside is visible
    // from the fixed isometric angle the same way the crusher's rolls are.
    // Short rather than tall for that reason — the first cut ran the wall to
    // z=26 and the rendered preview showed a closed silo, not a mixer.
    m.cylinder_z((16, 16), 12.0, (11, 21), p.idx("steel"));
    m.carve_cylinder_z((16, 16), 10.0, (13, 22));
    for z in [12, 17] {
        m.cylinder_z((16, 16), 12.4, (z, z), p.idx("steel_light"));
    }

    // Agitator: shaft plus a cross of paddles, sitting high in the vessel so
    // the blades clear the wall line and read from above.
    m.cylinder_z((16, 16), 1.6, (15, 26), p.idx("steel_light"));
    m.cuboid((7, 25), (15, 17), (18, 19), p.idx("steel_light"));
    m.cuboid((15, 17), (7, 25), (18, 19), p.idx("steel_light"));
    // Angled blade tips, so the paddles read as blades that move product
    // rather than as a flat plus sign.
    m.cuboid((7, 9), (15, 17), (19, 21), p.idx("steel_dark"));
    m.cuboid((23, 25), (15, 17), (19, 21), p.idx("steel_dark"));
    m.cuboid((15, 17), (7, 9), (19, 21), p.idx("steel_dark"));
    m.cuboid((15, 17), (23, 25), (19, 21), p.idx("steel_dark"));

    // Belt-driven agitator: the motor sits on a bracket BESIDE the vessel,
    // not on a lid over it. A top-mounted gearbox is what a real agitator
    // usually has, and it was the first cut here — but it (and the gantry
    // that replaced it) covered the opening, and this asset exists to show
    // the agitator. A side drive with a belt up to a pulley is the version
    // of "real" that also leaves the vessel readable.
    m.cuboid((24, 30), (12, 21), (12, 20), p.idx("paint_blue"));
    m.cuboid((24, 30), (12, 21), (21, 21), p.idx("steel_dark"));
    m.cylinder_y((27, 23), 3.0, (13, 20), p.idx("steel_dark"));
    m.cylinder_z((16, 16), 4.0, (24, 25), p.idx("brass"));
    // Belt guard spanning motor pulley to shaft pulley.
    m.cuboid((17, 27), (15, 18), (23, 24), p.idx("steel_dark"));

    // Vinegar inlet: a stub reaching toward the pipe run in front (see doc).
    // Height is PIPE_AXIS_Z exactly, not an independently-chosen number —
    // this stub and the pipe run's own `pipe_inlet` branch (see the fluid
    // pipes section below) are two ends of the same fitting, and the
    // visual-overhaul pass found they'd been modelled 4 voxels apart in
    // height, which is what actually made the mixer's supply line read as
    // disconnected from the pipe feeding it rather than merely a tile away.
    // Deliberately the same diameter and the same bright steel as the
    // `pipe_inlet` branch it meets, with a brass flange right at the tile
    // boundary: the two halves are one spool piece split across two tiles,
    // and the pair only reads as a connection if they match. The first
    // version was a 2-voxel `pipe_grey` nub against a dark blue skirt and
    // rendered as a lump.
    m.cylinder_y((16, PIPE_AXIS_Z), 3.0, (0, 9), p.idx("steel_light"));
    m.cylinder_y((16, PIPE_AXIS_Z), 4.0, (0, 1), p.idx("brass"));
    // Gusset under the stub, so it visibly lands on the machine.
    m.cuboid((15, 17), (2, 7), (9, PIPE_AXIS_Z - 3), p.idx("steel_dark"));

    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, TILE_VOXELS as i32 - 1);

    control_panel(&mut m, p, (11, 20), (3, 7), 2);
    m
}

/// `data/machines.ron`'s **2x2** `cooking_vat` — the milestone's big machine,
/// and the first asset in this pipeline larger than one tile.
///
/// The volume is 64x64 voxels, i.e. exactly UI_UX.md §2's 32-per-tile scale
/// times the footprint `data/machines.ron` declares, so it lands on its four
/// tiles without any per-asset fudge: `vox::encode` puts a model's origin at
/// its footprint centre at ground level, and `main.gd` places it at the
/// centre of the declared footprint rectangle. Change the footprint in the
/// data file and the model has to change with it — that is a deliberate
/// coupling, not an oversight; a machine's size is content.
///
/// 48 voxels tall (1.5 tiles), which is the point of it: it holds a batch
/// through the chain's longest process (240 ticks, `data/recipes.ron`), so it
/// should visibly be the biggest thing on the line. It gets the details a
/// vessel that size really has — a jacketed body, an access platform and
/// ladder, a steam stack, a sight glass showing the sauce level.
pub fn machine_cooking_vat(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS * 2, TILE_VOXELS * 2, 48]);
    machine_footing(&mut m, p, (0, TILE_VOXELS as i32 * 2 - 1), 10, "paint_blue");
    let c = (TILE_VOXELS as i32, TILE_VOXELS as i32); // (32, 32)

    // Jacketed vessel body + insulation bands + domed head.
    // Tall body, shallow head: the first cut paired a short barrel with a
    // 9-voxel dome and the rendered preview read as a sphere on a plinth,
    // not as a vat.
    m.cylinder_z(c, 25.0, (13, 38), p.idx("steel"));
    for z in [18, 25, 32] {
        m.cylinder_z(c, 25.6, (z, z), p.idx("steel_light"));
    }
    m.ellipsoid((c.0, c.1, 38), (25.0, 25.0, 6.0), p.idx("steel_light"));
    // Lit ring under the vessel skirt, so the vat reads as powered from any
    // of the four snap angles rather than only from the panel side (§7).
    m.cylinder_z(c, 25.6, (14, 14), p.idx("status_amber"));

    // Agitator drive on the dome, and a steam stack off to one side.
    m.cylinder_z(c, 5.0, (43, 44), p.idx("brass"));
    m.cuboid((25, 38), (25, 38), (45, 47), p.idx("paint_blue"));
    m.cylinder_z((49, 18), 3.0, (34, 47), p.idx("pipe_grey"));
    m.cylinder_z((49, 18), 4.0, (46, 46), p.idx("steel_light"));

    // Sight glass on the camera-facing side: the sauce level inside, framed
    // in stainless. Matte `sauce_red` on purpose — it is product, not a
    // polished surface.
    m.cuboid((29, 36), (5, 8), (16, 34), p.idx("steel_light"));
    m.cuboid((31, 34), (4, 6), (17, 29), p.idx("sauce_red"));

    // Access platform, handrail and ladder — the industrial language
    // UI_UX.md §3 asks for, and what actually sells the vat's size.
    m.cuboid((14, 50), (2, 8), (30, 31), p.idx("steel_dark"));
    m.hazard_stripe_x((14, 50), (2, 2), (30, 31), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for x in [14, 22, 30, 38, 46, 50] {
        m.cuboid((x, x), (2, 2), (32, 37), p.idx("steel_dark"));
    }
    m.cuboid((14, 50), (2, 2), (37, 37), p.idx("steel_dark"));
    for (x0, x1) in [(18, 18), (23, 23)] {
        m.cuboid((x0, x1), (3, 3), (3, 30), p.idx("steel_dark"));
    }
    for z in (5..30).step_by(4) {
        m.cuboid((18, 23), (3, 3), (z, z), p.idx("steel_light"));
    }

    // The junction hardware, and the one place in the factory where the
    // standard piece needed a second thought.
    //
    // The belt's row is the vat's sim-y = 0 tile, which is the model's UPPER
    // local-Y half (the importer maps low local Y to high world Z — the same
    // derivation `machine_mixer`'s inlet stub uses). That half is the vat's
    // FAR half from the fixed camera, and the vat is the only machine big
    // enough for that to matter: a sight line out of an infeed window there
    // has to cross the whole footprint toward the camera, and the vat's own
    // 25-voxel shell stands in the way for every height the window could
    // occupy. The first cut put a belt-width housing on that corner and it
    // rendered as nothing at all — correct geometry, invisible.
    //
    // So the vat's infeed is a **charging gallery across its whole -X face**
    // rather than a belt-width hood: same piece, wider lane. The belt still
    // hands over at its own row, and the gallery's window carries on to the
    // camera-facing corner, where nothing occludes it — which is where a
    // player actually sees product go in. A wide covered charging face is
    // also what a vat this size really has; the shape that solves the camera
    // is the honest one, the same way the hopper's box-over-cone was.
    //
    // The outfeed needs none of this: it is on +X, which the camera looks
    // straight at, and the shell stops well short of that edge.
    let belt_lane = (BELT_LANE.0 + TILE_VOXELS as i32, BELT_LANE.1 + TILE_VOXELS as i32);
    machine_infeed(&mut m, p, (BELT_LANE.0, belt_lane.1));
    machine_outfeed(&mut m, p, belt_lane, TILE_VOXELS as i32 * 2 - 1);

    control_panel(&mut m, p, (36, 50), (4, 9), 4);
    m
}

/// `data/machines.ron`'s 1x1 `bottler` — GAMEPLAY.md §2 tier 2's bottling &
/// sealing line, and one of the two kinds that had **no model at all** until
/// the "Visual overhaul round 2" epic's ticket 2. Both fell through
/// `main.gd`'s `MACHINE_MODEL_FALLBACK` to the generic `machine`, so the
/// last two stages of the chain rendered as two more copies of the same
/// blue box with a tank on it — which is precisely what the project owner's
/// review meant by "some machines are still missing assets or just reuse
/// assets (bottling machine)".
///
/// What makes a bottling line identifiable, and what this therefore models,
/// is the vertical arrangement: bottles standing in a row on a low
/// stainless deck, a gantry straddling them, and filling nozzles hanging
/// down from it onto the bottle mouths, with a capping head at the
/// downstream end. The bottles are the readable part, so they sit on the
/// camera-facing (-Y) half of the deck and the gantry legs stand behind
/// them at high Y, where they frame rather than hide.
pub fn machine_bottler(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    machine_footing(&mut m, p, (0, TILE_VOXELS as i32 - 1), 7, "paint_blue");

    // Stainless run-out deck the bottles travel along, with guide rails
    // either side of the bottle lane.
    m.cuboid((3, 28), (6, 25), (10, 11), p.idx("steel"));
    for y in [11, 20] {
        m.cuboid((3, 28), (y, y), (12, 13), p.idx("steel_light"));
    }

    // Three bottles in the lane, mid-fill — the same silhouette
    // `item_bottle` uses, so what leaves this machine is recognizably what
    // is standing in it.
    for x in [8, 16, 24] {
        m.cylinder_z((x, 16), 2.6, (12, 17), p.idx("ketchup_red"));
        m.cylinder_z((x, 16), 2.8, (14, 15), p.idx("label_cream"));
        m.cylinder_z((x, 16), 1.2, (18, 19), p.idx("glass_pale"));
    }

    // Filling gantry: two legs at high Y (behind the bottles) and a beam
    // across, with a nozzle dropping onto each of the first two bottles.
    for (x0, x1) in [(6, 8), (17, 19)] {
        m.cuboid((x0, x1), (21, 24), (12, 25), p.idx("steel"));
    }
    m.cuboid((5, 27), (21, 24), (26, 28), p.idx("paint_blue"));
    m.cuboid((5, 27), (21, 21), (26, 28), p.idx("steel_dark"));
    for x in [8, 16] {
        m.cuboid((x - 1, x + 1), (15, 22), (25, 26), p.idx("steel_light"));
        m.cylinder_z((x, 16), 1.4, (20, 25), p.idx("steel_light"));
    }

    // Capping head over the last bottle: a brass rotary chuck on a stem off
    // the beam — the "sealing" half of "bottling and sealing".
    m.cuboid((23, 25), (16, 22), (25, 26), p.idx("steel_light"));
    m.cylinder_z((24, 16), 3.2, (22, 24), p.idx("brass"));
    m.cylinder_z((24, 16), 1.4, (20, 22), p.idx("steel_dark"));

    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, TILE_VOXELS as i32 - 1);

    control_panel(&mut m, p, (11, 20), (3, 7), 2);
    m
}

/// `data/machines.ron`'s 1x1 `packer` — the chain's last stage, boxing
/// finished bottles, and the other kind that had no model of its own before
/// the "Visual overhaul round 2" epic's ticket 2 (see `machine_bottler`).
///
/// A packing station is a *table*, not a vessel, and that is the whole
/// silhouette decision here: it must not read as another tank on a plinth or
/// the two downstream stages will still look like the same machine twice.
/// So it is low and flat, and what stands on it is cardboard — a stack of
/// flat blanks waiting on the upstream side, an open case being filled on
/// the downstream side, and a taping head on a stand above it. Cardboard is
/// also the one large matte, warm-toned surface anywhere in the factory,
/// which makes this stage tell apart from its neighbours at a glance even
/// though it is the shortest machine on the line.
pub fn machine_packer(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    machine_footing(&mut m, p, (0, TILE_VOXELS as i32 - 1), 7, "paint_blue");

    // Stainless packing table.
    m.cuboid((2, 29), (4, 27), (10, 11), p.idx("steel"));
    m.cuboid((2, 29), (4, 4), (12, 12), p.idx("steel_light"));

    // Stack of flat case blanks waiting on the upstream (-X) side, banded
    // so it reads as a stack of sheets rather than a solid block.
    m.cuboid((3, 13), (8, 23), (12, 18), p.idx("cardboard"));
    for z in [13, 15, 17] {
        m.cuboid((3, 13), (8, 23), (z, z), p.idx("cardboard_dark"));
    }

    // The case being packed, open-topped with its flaps folded out, holding
    // a bottle — the moment this machine exists to depict.
    m.cuboid((17, 28), (9, 22), (12, 20), p.idx("cardboard"));
    m.carve_cuboid((19, 26), (11, 20), (14, 21));
    m.cuboid((17, 28), (6, 8), (20, 20), p.idx("cardboard_dark"));
    m.cuboid((17, 28), (23, 25), (20, 20), p.idx("cardboard_dark"));
    m.cylinder_z((22, 15), 2.2, (14, 19), p.idx("ketchup_red"));
    m.cylinder_z((22, 15), 1.2, (20, 21), p.idx("glass_pale"));

    // Taping head on a stand over the case: an arm carrying a brass tape
    // roll, reaching in from the back so it doesn't block the open case.
    // The stand stops at x = 27 rather than 28 so it clears the discharge
    // housing added below, which occupies the last four columns.
    m.cuboid((25, 27), (23, 26), (12, 27), p.idx("steel"));
    m.cuboid((16, 27), (18, 26), (26, 27), p.idx("steel_light"));
    m.cylinder_y((19, 24), 3.0, (19, 25), p.idx("brass"));
    m.cuboid((16, 18), (18, 25), (22, 24), p.idx("steel_dark"));

    // Infeed AND outfeed. The packer used to have no discharge at all,
    // because it was the chain's last stage and the player lifted finished
    // cases straight out of it with a keypress. The delivery-dock ticket
    // deleted that command and laid a real belt past this machine, running
    // into a dock — so this face now hands product to a conveyor like every
    // other stage's does, and would otherwise be a blank wall a case
    // appears in front of out of nowhere.
    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, TILE_VOXELS as i32 - 1);

    control_panel(&mut m, p, (5, 14), (3, 7), 2);
    m
}

// ---------------------------------------------------------------------------
// Aging rack (ARCHITECTURE.md §9 milestone 7, "Timed processes")
// ---------------------------------------------------------------------------
//
// Numbers `main.gd` mirrors to place and scale the maturation gauge's bar,
// stated in voxels — the same arrangement the pipe run (PIPE_BORE_*), the
// hopper (HOPPER_FILL_*) and the tank (TANK_FILL_*) already use. Change one,
// change both.
/// Bottom of the gauge slot, i.e. where a progress of 0 sits.
const GAUGE_Z0: i32 = 11;
/// How far a full bar rises above that floor. One voxel short of the slot's
/// own height, so a finished batch reads as a full bar inside a frame rather
/// than as a bar that has burst out of it.
const GAUGE_HEIGHT: i32 = 21;
/// The slot's inner span. The bar model is exactly this cross-section, so it
/// sits flush in the recess instead of hovering in front of it.
const GAUGE_X: (i32, i32) = (27, 29);
const GAUGE_Y: (i32, i32) = (0, 1);
/// The gauge post itself: the rack's camera-near corner upright, thickened
/// to carry the readout. High X and low Y is the one corner of a tile that
/// both visible faces meet at (see the junction section's camera
/// derivation), so it is the corner a glanceable readout belongs on.
const GAUGE_POST_X: (i32, i32) = (26, 30);
const GAUGE_POST_Y: (i32, i32) = (0, 4);

/// `data/machines.ron`'s 1x1 `aging_rack` — ARCHITECTURE.md §9 milestone 7's
/// machine: four oak casks resting on a stainless rack, with a maturation
/// gauge on the corner post.
///
/// **The silhouette decision, which is the whole ticket.** Every other
/// machine on this line is *driven* — the crusher has flywheels, the mixer an
/// agitator, the bottler a gantry, the packer a taping head — and each of
/// them is identified by its mechanism. This machine has no mechanism at
/// all, deliberately: what makes it different from every stage around it is
/// that it does nothing but hold a batch while 2400 ticks pass
/// (`data/recipes.ron`) — over a minute of real time at the rate `main.gd`
/// actually ticks, and ten times the longest process on the line. A rack
/// is the honest shape for that, and "the one machine with no moving part"
/// is also, at scene scale, a genuinely distinct read.
///
/// **Why casks, and why upright.** GAMEPLAY.md §2's tier 6 names
/// "barrel-aged" outright, and a hooped cask is a silhouette nothing else in
/// this factory has. They stand on end rather than lying in a cradle because
/// of the camera (UI_UX.md §1, the (1, 1, 1) view direction): a cask lying on
/// its side presents a round head to the camera but recedes into the tile as
/// a long ridge, and four of them read as corrugation; standing, each is a
/// separate round-topped object with its own visible head and bung, and four
/// of those read as *four things resting* — which is the point.
///
/// **Material.** The rack, deck and gauge post are stainless per UI_UX.md
/// §2b point 8; the casks are oak, which is that rule's one deliberate
/// exception, argued at `palette.ron`'s `oak` entry rather than here.
pub fn machine_aging_rack(p: &Palette) -> VoxModel {
    // Taller than a tile, like the cooking vat: `data/machines.ron` explains
    // that this kind stays 1x1 because its cost is TIME, not floor space —
    // "height in voxels is free, height in tiles is not."
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 38]);
    machine_footing(&mut m, p, (0, TILE_VOXELS as i32 - 1), 7, "paint_blue");

    // Stainless deck the casks stand on, with a raised lip on the
    // camera-facing edge so it reads as a tray rather than as a painted band.
    m.cuboid((8, 27), (1, 30), (10, 11), p.idx("steel"));
    m.cuboid((8, 27), (1, 1), (12, 12), p.idx("steel_light"));

    // Four casks in a 2x2. Each is a stack of discs: a narrow chime at top
    // and bottom, a belly in the middle. Hoops are full discs rather than
    // rings on purpose — only the outer surface is ever visible, so a disc
    // costs nothing and needs no ring primitive.
    for cx in [13, 22] {
        for cy in [10, 21] {
            for (z0, z1, r, wood) in [
                (12, 13, 3.4, "oak_dark"),
                (14, 15, 3.9, "oak"),
                (16, 24, 4.2, "oak"),
                (25, 26, 3.9, "oak"),
                (27, 28, 3.4, "oak_dark"),
            ] {
                m.cylinder_z((cx, cy), r, (z0, z1), p.idx(wood));
            }
            for (z, r) in [(14, 4.0), (18, 4.35), (23, 4.35), (27, 3.5)] {
                m.cylinder_z((cx, cy), r, (z, z), p.idx("steel_dark"));
            }
            // Head and bung: what says "sealed and left alone" from above,
            // which is the only angle this camera really sees a cask's top
            // from.
            m.cylinder_z((cx, cy), 3.2, (29, 29), p.idx("oak_dark"));
            m.cuboid((cx - 1, cx + 1), (cy - 1, cy + 1), (29, 30), p.idx("brass"));
        }
    }

    // The rack cage: legs and a top rectangle of beams. The legs stand in the
    // two Y bands the casks do NOT occupy, which is also what keeps them
    // clear of the infeed housing's camera-facing window (carved at y = 5..6
    // — a leg there would be sliced in half by it).
    //
    // Three legs, not four: the camera-near corner's leg is the gauge post
    // below, which stands in exactly that spot and is taller. Drawing both
    // put two uprights within two voxels of each other and the rendered
    // close-up read as a picket fence across the machine's front, with the
    // readout lost among them.
    //
    // `steel`, not `steel_dark`: a vertical face on a metal catches almost
    // no light at this fixed camera angle, so the first cut's dark legs
    // rendered as black bars and became the machine's loudest feature —
    // louder than the casks they exist to hold.
    for (x0, x1, y0, y1) in [(9, 10, 2, 3), (9, 10, 28, 29), (25, 26, 28, 29)] {
        m.cuboid((x0, x1), (y0, y1), (8, 34), p.idx("steel"));
    }
    for (y0, y1) in [(2, 3), (28, 29)] {
        m.cuboid((9, 26), (y0, y1), (33, 34), p.idx("steel"));
    }
    for (x0, x1) in [(9, 10), (25, 26)] {
        m.cuboid((x0, x1), (2, 29), (33, 34), p.idx("steel"));
    }

    // --- the maturation gauge ------------------------------------------
    // A recessed slot in the corner post, with a bar that grows up it. See
    // GAUGE_Z0 above and `aging_rack_gauge` below for the moving half.
    m.cuboid(GAUGE_POST_X, GAUGE_POST_Y, (8, 36), p.idx("steel_light"));
    // GENUINELY carved, not painted: a face is only culled when its
    // neighbour is absent (see `VoxModel::erase`), so a slot filled with a
    // dark colour would mesh flat and the bar would appear to be stuck to a
    // wall rather than sitting in a channel.
    m.carve_cuboid(GAUGE_X, GAUGE_Y, (GAUGE_Z0, GAUGE_Z0 + GAUGE_HEIGHT));
    // Back of the slot, black, so an empty gauge reads as an EMPTY SLOT and
    // not merely as "the amber one is off" — UI_UX.md §7 forbids state in
    // colour alone, and an unlit bar is exactly that failure mode.
    m.cuboid(GAUGE_X, (2, 2), (GAUGE_Z0, GAUGE_Z0 + GAUGE_HEIGHT), p.idx("hazard_black"));
    // The finish line, at the head of the scale: the one part of this
    // readout that is lit whether or not a batch is in. The gauge sits on a
    // face the fixed camera can only ever see UNLIT (every vertical face
    // pointing this way is away from the key light), so an empty slot is a
    // black stripe on a dark column and the scale's extent is invisible —
    // the player would have no idea how far the bar still has to travel.
    // A green line does not encode state in colour alone; it is a position,
    // and what it says is "when the amber reaches here, the batch is out."
    m.cuboid(GAUGE_X, (0, 1), (GAUGE_Z0 + GAUGE_HEIGHT + 1, GAUGE_Z0 + GAUGE_HEIGHT + 1), p.idx("status_green"));
    // Graduations down both sides of the slot, on the camera-facing face:
    // brass, the same accent every "product passes through here" fitting in
    // this factory already carries, and the thing that makes a bar read as a
    // measurement rather than as a light.
    for i in 0..=4 {
        let z = GAUGE_Z0 + (GAUGE_HEIGHT * i) / 4;
        for x in [GAUGE_POST_X.0, GAUGE_POST_X.1] {
            m.cuboid((x, x), (0, 0), (z, z), p.idx("brass"));
        }
    }
    // Weather hood over the head of the gauge, and a bracket at its foot.
    m.cuboid((25, 31), (0, 5), (36, 36), p.idx("steel"));
    m.cuboid((25, 31), (0, 5), (8, 9), p.idx("steel_dark"));

    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, TILE_VOXELS as i32 - 1);

    control_panel(&mut m, p, (11, 20), (3, 7), 2);
    m
}

/// The bar standing in the aging rack's gauge — the live half of the
/// readout, and the fourth asset built to this same one-voxel-slab pattern
/// (`pipe_fluid`, `hopper_fill`, `fluid_tank_fill`).
///
/// **One voxel tall**: `main.gd` scales this node vertically to the
/// machine's live process fraction, and a one-voxel slab scaled to N voxels
/// is geometrically the same box as an N-voxel column with none of the pivot
/// arithmetic (`vox::encode` puts a model's origin at its footprint centre
/// *at ground level*).
///
/// `status_amber`, and therefore the `emissive` class: this is a status
/// readout, which is exactly what UI_UX.md §7 reserves saturated colour for,
/// and emissive is what lets it be read at a glance across a lit factory
/// floor — the whole point of giving this process a passive indicator rather
/// than leaving it to the hover tooltip. Unlike the fill bodies above it
/// carries its own hue rather than a near-white one Godot tints: what a
/// hopper holds is content, but "how far through" is one universal quantity
/// with no per-machine colour to look up.
pub fn aging_rack_gauge(p: &Palette) -> VoxModel {
    debug_assert!(
        GAUGE_Z0 + GAUGE_HEIGHT <= 32,
        "a full bar would rise past the gauge slot"
    );
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 1]);
    m.cuboid(GAUGE_X, GAUGE_Y, (0, 0), p.idx("status_amber"));
    m
}

// ---------------------------------------------------------------------------
// Fluid tank (the "Visual overhaul round 2" epic — ticket 1's building)
// ---------------------------------------------------------------------------
//
// Numbers `main.gd` mirrors to place and scale the liquid column, stated in
// voxels — the same arrangement the pipe run (PIPE_BORE_*) and the hopper
// (HOPPER_FILL_*) already use. The column only ever occupies the vessel's
// straight-walled barrel: the domed head above it and the dished base below
// change cross-section with height, so a slab scaled inside either would
// poke through the shell.
/// Floor of the barrel section, i.e. where a fill level of 0 sits.
const TANK_FILL_Z0: i32 = 8;
/// How far a full tank's level rises above that floor. Sized so a full tank
/// stops short of the domed head — and deliberately not taller than that:
/// `World` places the tank on the pipe run's row, which is one tile NORTH of
/// the chain, so from the fixed isometric angle the tank stands in front of
/// the crusher. The first cut ran the barrel to z=34 with the head above it
/// and a vent stack above that, and in the rendered factory the tank simply
/// hid the machine behind it. A storage drum should read as substantial, not
/// as the tallest thing on the line.
const TANK_FILL_HEIGHT: i32 = 18;
/// Inside radius of the barrel; the liquid column matches it.
const TANK_INNER_R: f32 = 8.6;
const TANK_OUTER_R: f32 = 10.6;
const TANK_CENTER: i32 = 16;

/// `data/machines.ron`'s 1x1 `fluid_tank` — the vinegar the mixer runs on,
/// standing one tile west of the pipe run's head.
///
/// Ticket 1 of this epic made the tank real in simulation (a capped,
/// player-loadable buffer that replaced milestone 6's always-refilling
/// off-map tap) but shipped no model, so the thing a player is now supposed
/// to manage was not drawn at all — the project owner's review asked for
/// tanks, and this is the concrete one that was missing.
///
/// **It is a closed vessel with a sight window, not an open one.** The
/// hopper next door is open-topped because that is what a bulk feed hopper
/// is and because its level has to be seen from above; a vinegar storage
/// tank is neither, and drawing it open would have been the readable choice
/// at the cost of the grounded-industrial rule (UI_UX.md §3) that has
/// decided every other asset here. What a real tank has instead — and what
/// makes the level readable anyway — is a **level gauge**: a slot cut down
/// the camera-facing (-Y) wall, framed top and bottom, with the liquid
/// column visible through it. `fluid_tank_fill` below is that column.
///
/// The discharge is on +X at exactly `PIPE_AXIS_Z`, the height the pipe
/// run's own header sits at, for the same reason `machine_mixer`'s inlet
/// stub is pinned to that constant rather than to an independently chosen
/// number: the two are opposite ends of one fitting, and the visual-overhaul
/// pass already found what happens when such a pair drifts apart.
pub fn fluid_tank(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 40]);
    let last = TILE_VOXELS as i32 - 1;
    let c = TANK_CENTER;

    // Plinth + hazard skirt, flush with the tile's own edges, matching every
    // machine's footing (see `machine_footing`'s doc for why flush).
    m.cuboid((0, last), (0, last), (0, 1), p.idx("concrete_dark"));
    m.hazard_stripe_x((0, last), (0, 0), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((0, last), (last, last), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for y in 0i32..=last {
        let a = if (y + 2).rem_euclid(6) < 3 { p.idx("hazard_yellow") } else { p.idx("hazard_black") };
        m.set(0, y, 2, a);
        m.set(last, y, 2, a);
    }

    // Four short legs and a skirt ring, so the vessel stands clear of its
    // plinth the way a real one does rather than growing out of the floor.
    for (x0, x1) in [(8, 10), (21, 23)] {
        for (y0, y1) in [(8, 10), (21, 23)] {
            m.cuboid((x0, x1), (y0, y1), (2, 5), p.idx("steel_dark"));
        }
    }
    m.cylinder_z((c, c), TANK_OUTER_R, (5, 6), p.idx("steel_dark"));

    // Barrel, hollowed, with a domed head. Carved after both are drawn, so
    // the shell is a shell and not a solid billet.
    //
    // `steel_light`, not `steel`: this is a tall vertical cylinder, so what
    // the camera mostly sees of it is curved side wall facing away from the
    // key light, and in the first render the whole vessel went black. Height
    // and albedo are the two things that decide whether a tank reads, and
    // both were wrong; see `TANK_FILL_HEIGHT` for the height half.
    m.cylinder_z((c, c), TANK_OUTER_R, (6, 30), p.idx("steel_light"));
    m.ellipsoid((c, c, 30), (TANK_OUTER_R, TANK_OUTER_R, 4.0), p.idx("steel"));
    // Carve only up to the springing line of the head, NOT through it: an
    // earlier cut carved into the dome, whose crown is narrower than
    // `TANK_INNER_R` up there, so the whole top came away and the tank
    // rendered open, like a bucket.
    m.carve_cylinder_z((c, c), TANK_INNER_R, (TANK_FILL_Z0, 29));
    // One rolled seam band. A plate vessel is built in courses and the bands
    // say so, but each one is a raised ring that shadows itself, and three
    // of them plus the two gauge frames turned the barrel into corrugation.
    m.cylinder_z((c, c), TANK_OUTER_R + 0.6, (18, 18), p.idx("steel"));

    // Level gauge: a slot down the camera-facing wall, framed above and
    // below by re-drawn rings so it reads as a fitted gauge rather than as
    // damage. The frame rings are drawn AFTER the cut for that reason.
    m.carve_cuboid((c - 4, c + 4), (0, c - 6), (TANK_FILL_Z0 + 1, TANK_FILL_Z0 + TANK_FILL_HEIGHT + 1));

    for z in [TANK_FILL_Z0, TANK_FILL_Z0 + TANK_FILL_HEIGHT + 2] {
        m.cylinder_z((c, c), TANK_OUTER_R, (z, z), p.idx("steel_light"));
        m.carve_cylinder_z((c, c), TANK_INNER_R, (z, z));
    }

    // Discharge on +X at the pipe run's own axis height (see this
    // function's doc), reaching the tile edge so it meets `pipe_source`'s
    // inlet flange rather than stopping short of it.
    // Bright, and thick enough to see: ticket 3 found this spout rendering
    // as dark grey on the tank's own dark shell, so the one piece of
    // hardware that says "this vessel feeds the run" was invisible at scene
    // scale. It now matches `pipe_source`'s inlet flange in both metal and
    // diameter, and the two brass collars meet on the tile boundary.
    m.cylinder_x((c, PIPE_AXIS_Z), 3.0, (20, last), p.idx("steel_light"));
    m.cylinder_x((c, PIPE_AXIS_Z), 4.2, (last - 1, last), p.idx("brass"));
    // Gusset under the spout, so it lands on the vessel rather than floating.
    m.cuboid((22, last), (c - 1, c + 1), (PIPE_AXIS_Z - 6, PIPE_AXIS_Z - 3), p.idx("steel_dark"));

    // Fill point on top: a man-way and a short vent stack, which is how a
    // player's "load the tank" action reads as a physical thing.
    m.cylinder_z((c, c), 4.0, (34, 35), p.idx("steel_light"));
    m.cylinder_z((c - 6, c + 5), 1.6, (32, 37), p.idx("pipe_grey"));

    // Access ladder up the far (+Y) side.
    for x in [c - 3, c + 3] {
        m.cuboid((x, x), (27, 27), (3, 32), p.idx("steel_dark"));
    }
    for z in (5..32).step_by(4) {
        m.cuboid((c - 3, c + 3), (27, 27), (z, z), p.idx("steel_light"));
    }

    // A single status lamp on the camera-facing face, so an empty tank can
    // report itself the way the hopper and a stalled machine already do
    // (UI_UX.md §7).
    m.cuboid((c - 8, c - 6), (5, 6), (24, 26), p.idx("status_amber"));
    m
}

/// The liquid standing in the tank — the tank's counterpart of `pipe_fluid`,
/// and built the same way for the same reason.
///
/// **One voxel tall**: `main.gd` scales this node vertically to the tank's
/// live fill fraction, and a one-voxel slab scaled to N voxels is
/// geometrically the same column as an N-voxel one, with none of the pivot
/// arithmetic a taller model would need (`vox::encode` puts a model's origin
/// at its footprint centre *at ground level*). Near-white and `liquid`-class
/// on purpose, exactly like `pipe_fluid`: *which* fluid a tank holds is live
/// simulation data (`FluidTankSnapshot::fluid`), so Godot multiplies the
/// per-fluid tint over these vertices and one model serves every fluid
/// `data/recipes.ron` will ever name.
pub fn fluid_tank_fill(p: &Palette) -> VoxModel {
    debug_assert!(
        TANK_FILL_Z0 + TANK_FILL_HEIGHT <= 29,
        "a full tank's level would rise into its domed head"
    );
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 1]);
    m.cylinder_z((TANK_CENTER, TANK_CENTER), TANK_INNER_R, (0, 0), p.idx("fluid_pale"));
    m
}

// ---------------------------------------------------------------------------
// Hopper (the "Machines improvement" milestone, ticket 3's building)
// ---------------------------------------------------------------------------
//
// Numbers `main.gd` mirrors to place and scale the fill body, stated in
// voxels — the same arrangement the pipe run already uses (PIPE_BORE_*).
// The fill only ever occupies the hopper's *straight-walled* upper section:
// a converging funnel's cross-section changes with height, so a slab scaled
// inside one would poke through the walls at low fill. Real bulk hoppers are
// built exactly this way — a straight box section over a converging
// discharge cone — so the shape that makes the visual work is also the
// honest one.
/// Floor of the straight section, i.e. where a fill level of 0 sits.
const HOPPER_FILL_Z0: i32 = 21;
/// How far a full hopper's level rises above that floor.
const HOPPER_FILL_HEIGHT: i32 = 9;
/// Inner half-width of the straight section; the fill body matches it.
const HOPPER_INNER_HALF: i32 = 10;
const HOPPER_CENTER: i32 = 16;

/// `data/machines.ron`'s 1x1 `hopper` — ticket 3's raw-ingredient feeder,
/// and deliberately **not** a bin or a crate: a wide open-topped box section
/// over a converging funnel, narrowing to an enclosed discharge chute that
/// steps down over the head of the first belt on the +X side. That silhouette
/// is what says "this meters bulk product onto a line" at a glance.
///
/// Open-topped because the fill level has to be *seen*: the fixed isometric
/// camera looks obliquely down (UI_UX.md §1), so the level inside reads
/// directly, the same way the pipe run's sight opening makes its liquid
/// readable. `hopper_fill` below is the body that sits in it.
pub fn hopper(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let c = HOPPER_CENTER;

    // Plinth + hazard skirt, matching every machine's footing — flush with
    // the tile's own edges (0..last), not inset, for the same reason
    // `machine_footing`'s doc gives: the hopper stands directly against the
    // first belt tile, and an inset plinth left a hairline gap of bare
    // floor at exactly that seam (the visual-overhaul pass's fix).
    let last = TILE_VOXELS as i32 - 1;
    m.cuboid((0, last), (0, last), (0, 1), p.idx("concrete_dark"));
    m.hazard_stripe_x((0, last), (0, 0), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((0, last), (last, last), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));

    // Four corner columns running the full height, with the funnel slung
    // between them — how a real bulk hopper is actually supported, and (the
    // reason it is drawn this way here) the arrangement that leaves the
    // funnel's sloping faces exposed. Two earlier cuts stood the whole thing
    // on short legs with cross-bracing, and in both renders the bracing
    // dominated the silhouette while the funnel disappeared underneath the
    // box entirely.
    for (x0, x1) in [(3, 6), (25, 28)] {
        for (y0, y1) in [(3, 6), (25, 28)] {
            m.cuboid((x0, x1), (y0, y1), (2, 22), p.idx("steel_dark"));
        }
    }

    // Converging discharge cone: a stack of square rings widening upward,
    // each hollowed so product has somewhere to be. This is the half of the
    // silhouette that says "hopper" rather than "bin", so it gets more of
    // the height than the straight section above it does.
    for z in 9..=20 {
        let h = 4 + ((z - 9) * 8) / 11;
        m.cuboid((c - h, c + h), (c - h, c + h), (z, z), p.idx("steel"));
        m.carve_cuboid((c - h + 2, c + h - 2), (c - h + 2, c + h - 2), (z, z));
    }

    // Straight box section: walls 2 thick, open at the top. Product level
    // only ever shows here (see HOPPER_FILL_Z0's note above).
    let outer = HOPPER_INNER_HALF + 2;
    m.cuboid((c - outer, c + outer), (c - outer, c + outer), (HOPPER_FILL_Z0, 30), p.idx("steel"));
    m.carve_cuboid(
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (HOPPER_FILL_Z0, 31),
    );
    // Flared loading lip, and hazard striping on the camera-facing wall.
    m.cuboid((c - outer - 1, c + outer + 1), (c - outer - 1, c + outer + 1), (30, 30), p.idx("steel_light"));
    m.carve_cuboid(
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (30, 31),
    );
    m.hazard_stripe_x(
        (c - outer, c + outer),
        (c - outer, c - outer),
        (23, 25),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );

    // Enclosed discharge chute, leaving the cone's throat and stepping down
    // toward the belt (+X) to end just above its running surface, with a
    // slide gate and a painted actuator box — the metering hardware the
    // simulation's one-unit-per-tick unload stands for.
    m.cuboid((15, 23), (12, 19), (11, 15), p.idx("steel"));
    m.cuboid((23, 31), (12, 19), (10, 13), p.idx("steel"));
    m.cuboid((28, 31), (11, 20), (9, 12), p.idx("steel_light"));
    m.cuboid((21, 22), (10, 21), (10, 18), p.idx("steel"));
    m.cuboid((20, 23), (10, 12), (18, 21), p.idx("paint_blue"));
    m.cuboid((21, 22), (11, 11), (16, 18), p.idx("brass"));

    // A single status lamp on the -Y face, so an empty hopper can eventually
    // report itself the way a stalled machine does (§7).
    m.cuboid((10, 12), (5, 6), (25, 27), p.idx("status_amber"));
    m
}

/// The bulk product sitting in the hopper's straight section — the solid
/// counterpart of `pipe_fluid`, and built the same way for the same reason.
///
/// **One voxel tall**: `main.gd` scales this node vertically to the hopper's
/// live fill fraction, and a one-voxel slab scaled to N voxels is
/// geometrically the same box as an N-voxel column, with none of the pivot
/// arithmetic a taller model would need (`vox::encode` puts a model's origin
/// at its footprint centre *at ground level*). Near-white and matte on
/// purpose: *which* raw ingredient a hopper holds comes from
/// `data/recipes.ron`, so Godot multiplies a per-item tint over these
/// vertices — and unlike a fluid, a heap of product is not a wet surface, so
/// it takes the `matte` class rather than `liquid` (see palette.ron).
pub fn hopper_fill(p: &Palette) -> VoxModel {
    debug_assert!(
        HOPPER_FILL_Z0 + HOPPER_FILL_HEIGHT <= 30,
        "a full hopper's level would rise above its loading lip"
    );
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 1]);
    let c = HOPPER_CENTER;
    m.cuboid(
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (0, 0),
        p.idx("bulk_pale"),
    );
    m
}

// ---------------------------------------------------------------------------
// Delivery dock (the delivery-dock ticket, GAMEPLAY.md §6/§9)
// ---------------------------------------------------------------------------

/// Top voxel of the dock's raised platform, and deliberately the same number
/// as [`BELT_SURFACE_Z`]: a case leaving the last belt has to arrive on a
/// surface level with the one it left, or the transfer housing at the end of
/// this line would be the only junction in the factory with a step in it.
/// `main.gd` mirrors it (DOCK_DECK_VOXELS) to stand the waiting stock on the
/// deck — change one, change both.
pub const DOCK_DECK_Z: i32 = BELT_SURFACE_Z;

/// `data/machines.ron`'s 1x1 `dock` — the chain's exit, where finished cases
/// leave the factory and become money (`sim_core::sim::dock`).
///
/// **Why a raised platform with a despatch door, rather than another housing
/// with a mechanism on top.** Every processing kind on this line is
/// identified by the machinery that does its work — flywheels, an agitator,
/// a filling gantry, a taping head — and the aging rack by the deliberate
/// absence of one (UI_UX.md §2b point 15). A dock does no work at all: it is
/// a *place*, the one tile in the world that is not equipment. So it is
/// drawn as architecture — a poured platform at trailer-bed height, its edge
/// striped, with a despatch opening and a canopy behind it — which is both
/// what a real loading bay looks like and the one silhouette on this line
/// that could not be mistaken for a machine.
///
/// **Where each piece goes is decided by the camera, exactly as the junction
/// section above derives it.** The fixed isometric view (UI_UX.md §1) can
/// only ever see faces whose normal points +X or -Y in this model's own
/// coordinates, and — the part that actually drove this layout — a sight
/// line leaving any point travels `(+x, -y, +z)`, so anything at HIGH x and
/// LOW y stands in front of everything behind it. That rules out the obvious
/// design, which was tried first: a truck bay carved into the +X face. It
/// reads beautifully on its own and it walls off the entire deck behind it,
/// because every deck voxel's sight line has to pass through x = 28..31 on
/// its way out. So the tall structure went to the FAR (high-y) side, where
/// its -Y face is still square to the camera but it occludes nothing, and
/// the deck's near half was left clear for the goods that actually stand on
/// it (see `main.gd`'s `_update_dock_stock`).
///
/// **Material** (UI_UX.md §2b point 8, "food contact is stainless, structure
/// is painted or poured"): a dock is almost entirely structure — poured
/// concrete deck, painted steel wall and canopy, rubber bumpers, hazard
/// striping — so it is the least stainless building in the factory, and
/// deliberately so. The stainless is exactly where product passes: the
/// infeed transfer housing and its brass threshold, and the despatch
/// opening's frame. That split is the rule doing its job rather than an
/// exception to it.
pub fn machine_dock(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let last = TILE_VOXELS as i32 - 1;
    let deck = DOCK_DECK_Z;

    // --- The platform ------------------------------------------------------
    // A solid poured block, not a machine footing: this is the one building
    // on the line the player is meant to read as floor rather than as
    // equipment. Flush with the tile's own edges for `machine_footing`'s
    // reason — the last belt tile butts straight against it.
    // Poured mid-grey with a lighter wearing surface and a dark kerb course
    // at the bottom. The first cut made the whole block `concrete_dark`,
    // which is right for a floor slab's *seam* and wrong for a mass this
    // size: rendered in the scene it read as a black pit with a lit rail
    // round it, not as a platform goods stand on.
    m.cuboid((0, last), (0, last), (0, 1), p.idx("concrete_dark"));
    m.cuboid((0, last), (0, last), (2, deck - 1), p.idx("concrete_light"));
    m.cuboid((1, last - 1), (1, last - 1), (2, deck - 1), p.idx("concrete"));
    m.cuboid((0, last), (0, last), (deck, deck), p.idx("concrete_light"));

    // Edge striping on the two faces the camera can actually see. A loading
    // dock's edge is marked because the drop is the hazard, so this is
    // signage doing its real job rather than decoration (UI_UX.md §3). Two
    // voxels, not three: half a hazard stripe is black, and this band runs
    // the full length of both camera-facing edges — the wider first cut put
    // more black on the dock than anything else.
    m.hazard_stripe_x((0, last), (0, 0), (deck - 1, deck), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for y in 0..=last {
        let a = if (y + 2).rem_euclid(6) < 3 { p.idx("hazard_yellow") } else { p.idx("hazard_black") };
        for z in (deck - 1)..=deck {
            m.set(last, y, z, a);
        }
    }

    // Rubber dock bumpers on the +X edge, where a trailer would come to rest
    // against the platform. Short (they sit below deck level), so they add a
    // recognisable piece of dock hardware to the most camera-facing face
    // without standing between the camera and anything.
    for (y0, y1) in [(4, 8), (23, 27)] {
        m.cuboid((last - 1, last), (y0, y1), (deck - 6, deck - 1), p.idx("rubber_dark"));
    }

    // --- The despatch wall, on the FAR side --------------------------------
    // Painted steel, ribbed, standing at high y so its -Y face is square to
    // the camera and it occludes nothing on the deck in front of it.
    m.cuboid((0, last), (26, last), (deck + 1, 27), p.idx("paint_blue"));
    for x in (2..=30).step_by(4) {
        m.cuboid((x, x), (26, 26), (deck + 1, 25), p.idx("steel_dark"));
    }

    // The despatch opening, carved through to a black back wall — a real
    // hole, not a dark rectangle painted on the wall (see this file's
    // junction tests for why that distinction is checked rather than
    // eyeballed). Its floor is what a sight line entering it actually
    // reaches (the infeed housing's doc derives that for the mirrored case),
    // so the floor is black and the frame around it is the palette's
    // brightest metal.
    m.cuboid((8, 25), (24, 26), (deck + 1, 23), p.idx("steel_light"));
    m.carve_cuboid((11, 22), (24, 30), (deck + 1, 21));
    m.cuboid((11, 22), (last, last), (deck + 1, 21), p.idx("hazard_black"));
    m.cuboid((11, 22), (24, 30), (deck + 1, deck + 1), p.idx("hazard_black"));
    // Brass threshold at the lip: the same accent every other fitting
    // product physically passes through carries.
    m.cuboid((11, 22), (24, 24), (deck + 1, deck + 1), p.idx("brass"));
    // The roller shutter, wound up above the opening — what says this
    // opening is a door rather than a window.
    m.cuboid((10, 23), (23, 24), (22, 23), p.idx("steel_dark"));
    m.cuboid((10, 23), (23, 23), (22, 22), p.idx("steel_light"));

    // --- Canopy over the bay ----------------------------------------------
    // Cantilevered off the wall toward the camera. It stops well short of
    // the deck's near edge so the goods standing there stay in clear air.
    // Integral with the wall's own head rather than a slab hovering over it:
    // the first cut stood it two voxels clear on thin posts and rendered as
    // a white shelf floating in front of a blue wall — at scene scale a gap
    // that small is not read as a gap, it is read as two unrelated objects.
    // Shallow, too, because every voxel it reaches toward the camera is a
    // voxel of deck it shades, and the deck is where the goods stand.
    m.cuboid((6, 25), (20, 26), (26, 27), p.idx("steel"));
    m.cuboid((6, 25), (20, 20), (25, 25), p.idx("hazard_yellow"));
    for x in [6, 25] {
        m.cuboid((x, x), (21, 26), (24, 25), p.idx("steel_dark"));
    }

    // Bay light under the canopy's leading edge — hung one voxel proud of it
    // so the canopy itself doesn't shadow it out of the frame. This is the
    // dock's status lamp: `main.gd` swaps it green when stock is waiting to
    // be sold and amber when the dock is clear, the same idle/running pair
    // every machine's lamp uses.
    m.cuboid((13, 18), (18, 19), (24, 25), p.idx("steel_light"));
    m.cuboid((14, 17), (18, 18), (23, 24), p.idx("status_amber"));

    // --- The junction ------------------------------------------------------
    // The belt from the packer runs in on -X, through exactly the transfer
    // housing every other machine on the line is fed through. No outfeed:
    // nothing is downstream of the dock — product leaves the world here.
    machine_infeed(&mut m, p, BELT_LANE);
    m
}

// ---------------------------------------------------------------------------
// Junction tests (the "Visual overhaul round 2" epic's ticket 3)
// ---------------------------------------------------------------------------
//
// These assert the ONE thing about this ticket a headless check can actually
// prove: that every machine has a real, EMPTY opening where its belt meets
// it, at the belt's own lane and height. "Real" is the load-bearing word —
// an inlet painted dark is not an inlet, because a voxel face is only culled
// when its neighbour is genuinely absent (see `VoxModel::erase`), so a
// coloured-in mouth meshes as a flat rectangle and renders as a decal. That
// distinction is invisible to a screenshot at scene scale and is exactly the
// kind of thing that would rot silently, so it is checked here rather than
// left to the eye.
//
// What these CANNOT check is whether the opening reads to a player — that
// needs `scripts/screenshot-scene.sh` and someone looking, which is how the
// design in this file was arrived at in the first place.
#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        Palette::load()
    }

    /// Every kind's (model, lane, last-X) triple, so a new machine kind added
    /// to `ASSETS` without junction hardware fails here rather than shipping
    /// a wall for a belt to vanish into.
    struct Junctions {
        name: &'static str,
        model: VoxModel,
        /// Where the belt actually meets this machine, in ITS coordinates.
        belt_lane: (i32, i32),
        /// The span the infeed housing covers, which is wider than the belt
        /// lane for the cooking vat (see its doc for why).
        housing_lane: (i32, i32),
        last_x: i32,
    }

    fn machines(p: &Palette) -> Vec<Junctions> {
        let tile_last = TILE_VOXELS as i32 - 1;
        let vat_lane = (BELT_LANE.0 + TILE_VOXELS as i32, BELT_LANE.1 + TILE_VOXELS as i32);
        let one_tile = |name, model| Junctions {
            name,
            model,
            belt_lane: BELT_LANE,
            housing_lane: BELT_LANE,
            last_x: tile_last,
        };
        vec![
            one_tile("machine", machine(p)),
            one_tile("machine_crusher", machine_crusher(p)),
            one_tile("machine_mixer", machine_mixer(p)),
            one_tile("machine_bottler", machine_bottler(p)),
            one_tile("machine_packer", machine_packer(p)),
            one_tile("machine_aging_rack", machine_aging_rack(p)),
            one_tile("machine_dock", machine_dock(p)),
            Junctions {
                name: "machine_cooking_vat",
                model: machine_cooking_vat(p),
                belt_lane: vat_lane,
                housing_lane: (BELT_LANE.0, vat_lane.1),
                last_x: TILE_VOXELS as i32 * 2 - 1,
            },
        ]
    }

    /// The one building at the end of the line that takes product in and
    /// hands none on: nothing is downstream of the delivery dock, because
    /// product leaves the world there (`sim_core::sim::dock`). Everything
    /// else must have an outfeed — including the packer, which used to be
    /// on this list and lost its place the moment the delivery-dock ticket
    /// laid a real belt past it.
    ///
    /// The dock is *skipped* rather than checked with the rule inverted:
    /// its +X face is a platform edge, so the air above it is open by
    /// design, and a "there is no doorway here" assertion would read that
    /// open air as a doorway. What the dock's own face has to satisfy is
    /// checked by `the_docks_deck_is_solid_out_to_the_platform_edge`.
    const NO_OUTFEED: &[&str] = &["machine_dock"];

    #[test]
    fn every_machine_has_an_open_inlet_at_the_belt_lane() {
        let p = palette();
        for j in machines(&p) {
            let mid = (j.belt_lane.0 + j.belt_lane.1) / 2;
            assert!(
                !j.model.filled(0, mid, BELT_SURFACE_Z + 2),
                "{}: the -X face is solid at the belt's lane and height — a belt \
                 feeding it would run into a wall",
                j.name
            );
            assert!(
                j.model.filled(0, j.housing_lane.0 - 2, BELT_SURFACE_Z + 2),
                "{}: the inlet housing has no side frame, so its opening has no edge",
                j.name
            );
        }
    }

    #[test]
    fn every_machine_but_the_last_has_an_open_outlet_at_the_belt_lane() {
        let p = palette();
        for j in machines(&p) {
            let mid = (j.belt_lane.0 + j.belt_lane.1) / 2;
            if NO_OUTFEED.contains(&j.name) {
                continue;
            }
            assert!(
                !j.model.filled(j.last_x, mid, BELT_SURFACE_Z + 2),
                "{}: the +X face is solid at the belt's lane and height — product \
                 would appear on the belt out of nowhere",
                j.name
            );
        }
    }

    /// The lip and apron are what make a junction continuous rather than two
    /// pieces that stop near each other, so their height is checked against
    /// `belt()`'s own running surface rather than against a number typed in
    /// twice.
    #[test]
    fn junction_surfaces_sit_exactly_at_the_belts_running_height() {
        let p = palette();
        let belt = belt(&p);
        assert!(belt.filled(0, 16, BELT_SURFACE_Z), "BELT_SURFACE_Z is not the belt's top voxel");
        assert!(!belt.filled(0, 16, BELT_SURFACE_Z + 1), "BELT_SURFACE_Z is not the belt's TOP");

        for j in machines(&p) {
            let mid = (j.belt_lane.0 + j.belt_lane.1) / 2;
            assert!(
                j.model.filled(0, mid, BELT_SURFACE_Z),
                "{}: no infeed apron at the belt's own surface height",
                j.name
            );
            if !NO_OUTFEED.contains(&j.name) {
                assert!(
                    j.model.filled(j.last_x, mid, BELT_SURFACE_Z),
                    "{}: no discharge lip at the belt's own surface height",
                    j.name
                );
            }
        }
    }

    /// The junction-adjacent belt tiles must differ from a plain one, and
    /// differ at the END that faces the machine — `main.gd` picks between
    /// them per tile, so two identical models would make that choice a lie.
    #[test]
    fn belt_variants_differ_from_a_plain_tile_at_the_end_that_meets_a_machine() {
        let p = palette();
        let (plain, inlet, outlet) = (belt(&p), belt_inlet(&p), belt_outlet(&p));
        let last = TILE_VOXELS as i32 - 1;
        let mid = (BELT_LANE.0 + BELT_LANE.1) / 2;

        assert!(inlet.voxels.len() > plain.voxels.len(), "belt_inlet adds nothing");
        assert!(outlet.voxels.len() > plain.voxels.len(), "belt_outlet adds nothing");
        // The hand-over plate reaches the shared tile edge on the machine side.
        assert!(inlet.filled(last, mid, BELT_SURFACE_Z), "belt_inlet's dead-plate stops short");
        assert!(outlet.filled(0, mid, BELT_SURFACE_Z), "belt_outlet's receiving plate stops short");
        // ... and neither dresses the end it does NOT meet a machine at.
        assert!(!inlet.filled(0, BELT_LANE.0 - 1, BELT_SURFACE_Z + 1), "belt_inlet dressed its upstream end");
        assert!(!outlet.filled(last, BELT_LANE.0 - 1, BELT_SURFACE_Z + 1), "belt_outlet dressed its downstream end");
    }

    /// The aging rack's maturation gauge (ARCHITECTURE.md §9 milestone 7).
    /// Same class of check as the junctions above and for the same reason: a
    /// slot painted dark meshes flat, so the bar would read as a sticker on
    /// a wall rather than as a level in a channel — a distinction no
    /// screenshot at scene scale can settle. Also pins the bar model to the
    /// slot it is supposed to sit in, since the two are separate models that
    /// only line up because `vox::encode` centres both on the same footprint.
    #[test]
    fn the_aging_racks_gauge_slot_is_genuinely_open_and_its_bar_fits_it() {
        let p = palette();
        let rack = machine_aging_rack(&p);
        let bar = aging_rack_gauge(&p);
        let mid_x = (GAUGE_X.0 + GAUGE_X.1) / 2;

        for z in [GAUGE_Z0, GAUGE_Z0 + GAUGE_HEIGHT] {
            assert!(
                !rack.filled(mid_x, GAUGE_Y.0, z),
                "the gauge slot is filled at z = {z} — a painted slot is not a slot"
            );
        }
        // ...and it is a slot, not a hole through the post: the black back
        // wall is what an empty gauge shows.
        assert!(
            rack.filled(mid_x, GAUGE_Y.1 + 1, GAUGE_Z0 + 1),
            "the gauge slot has no back wall, so an empty gauge would show straight through"
        );
        // The bar occupies exactly the slot's cross-section, one voxel tall.
        assert!(bar.filled(mid_x, GAUGE_Y.0, 0), "the gauge bar misses the slot");
        assert!(!bar.filled(mid_x, GAUGE_Y.1 + 1, 0), "the gauge bar is deeper than the slot");
        assert!(!bar.filled(GAUGE_X.1 + 1, GAUGE_Y.0, 0), "the gauge bar is wider than the slot");
        // The slot has a head above a full bar, so the bar reads as filling
        // a frame rather than as running off the top of the post.
        assert!(
            rack.filled(mid_x, GAUGE_Y.0, GAUGE_Z0 + GAUGE_HEIGHT + 2),
            "the gauge slot is open above a full bar's head"
        );
    }

    /// The delivery dock's own face, which the belt-junction rules above
    /// deliberately skip (see `NO_OUTFEED`). Three things have to hold, and
    /// none of them are visible in a screenshot at scene scale:
    ///
    ///  * the deck is solid right out to the +X tile edge, so the platform
    ///    reads as a platform rather than as a machine with a lip;
    ///  * it is level with the belt that feeds it, so a case arriving does
    ///    not step up or down at the tile boundary;
    ///  * the despatch opening in the far wall is a genuinely EMPTY hole
    ///    with a back wall behind it — a dark rectangle painted on the wall
    ///    would mesh flat and render as a sticker, which is the same
    ///    distinction the junction tests above exist for.
    #[test]
    fn the_docks_deck_is_solid_out_to_the_platform_edge() {
        let p = palette();
        let dock = machine_dock(&p);
        let last = TILE_VOXELS as i32 - 1;
        let mid = (BELT_LANE.0 + BELT_LANE.1) / 2;

        assert_eq!(
            DOCK_DECK_Z, BELT_SURFACE_Z,
            "the dock's deck must stand at the belt's own running height"
        );
        assert!(
            dock.filled(last, mid, DOCK_DECK_Z),
            "the deck stops short of the +X tile edge"
        );
        assert!(
            !dock.filled(last, mid, DOCK_DECK_Z + 1),
            "something stands on the platform edge at the belt's lane — the dock \
             has no downstream conveyor, so nothing should promise one"
        );

        // The despatch opening: empty through the wall, with a wall behind it.
        assert!(
            !dock.filled(16, 27, DOCK_DECK_Z + 6),
            "the despatch opening is filled — a painted door is not a door"
        );
        assert!(
            dock.filled(16, last, DOCK_DECK_Z + 6),
            "the despatch opening has no back wall, so it shows straight through \
             the building"
        );
        assert!(
            dock.filled(10, 26, DOCK_DECK_Z + 6),
            "the despatch opening has no frame, so its edge is nothing"
        );
    }

    /// The pipe run's two ends. Its downstream end is closed by a blind
    /// flange (it used to be an open bore, which is what the run "just
    /// stopping in mid-air" actually was), and its supply end is not.
    #[test]
    fn the_pipe_runs_downstream_end_is_capped_and_its_supply_end_is_not() {
        let p = palette();
        let last = TILE_VOXELS as i32 - 1;
        assert!(
            !pipe(&p).filled(last, PIPE_AXIS_Y, PIPE_AXIS_Z),
            "a mid-run pipe tile must stay bored through, or segments cannot join"
        );
        assert!(
            pipe_inlet(&p).filled(last, PIPE_AXIS_Y, PIPE_AXIS_Z),
            "the run's downstream end is still an open bore"
        );
        assert!(
            !pipe_source(&p).filled(0, PIPE_AXIS_Y, PIPE_AXIS_Z),
            "the supply end must stay open — the tank discharges into it"
        );
    }

    /// The tank/pipe and pipe/mixer joints are each one fitting split across
    /// a tile boundary, so both halves have to reach their shared edge at the
    /// same axis. This is the check that would have caught the four-voxel
    /// height mismatch the previous visual pass found by eye.
    #[test]
    fn fluid_joints_meet_at_the_tile_boundary_on_a_shared_axis() {
        let p = palette();
        let last = TILE_VOXELS as i32 - 1;
        assert!(
            fluid_tank(&p).filled(last, TANK_CENTER, PIPE_AXIS_Z),
            "the tank's discharge does not reach the tile edge the run starts at"
        );
        assert!(
            pipe_inlet(&p).filled(16, last, PIPE_AXIS_Z),
            "the run's branch does not reach the tile edge the machine sits against"
        );
        assert!(
            machine_mixer(&p).filled(16, 0, PIPE_AXIS_Z),
            "the mixer's inlet stub does not reach the tile edge the run sits against"
        );
    }
}
