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
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 12]);
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
    m
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
    pipe_body(&mut m, p, (18, 25));

    // Heavy inlet flange facing the tank, wider than a plain tile join so
    // the supply end is identifiable from any of the four snap angles.
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 2.0, (0, 2), p.idx("steel"));

    // Isolation valve: a bonnet straddling the run, a stem, and a brass
    // hand wheel — the one thing on this tile a player would reach for, and
    // the only place brass survives on the run (see `pipe_body`'s doc).
    m.cuboid((7, 12), (12, 19), (PIPE_AXIS_Z, 22), p.idx("steel_dark"));
    m.cuboid((8, 11), (13, 18), (23, 24), p.idx("steel_light"));
    m.cuboid((9, 10), (15, 16), (25, 27), p.idx("steel_dark"));
    m.cylinder_z((9, 16), 4.0, (28, 28), p.idx("brass"));
    m.carve_cylinder_z((9, 16), 2.0, (28, 28));
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
    m.cylinder_y((16, PIPE_AXIS_Z), 2.2, (17, last), p.idx("pipe_grey"));
    m.cylinder_y((16, PIPE_AXIS_Z), 3.0, (last - 1, last), p.idx("brass"));
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

    // Drive motor tucked against the -X side, belted up to the near
    // flywheel.
    m.cuboid((0, 4), (9, 22), (10, 18), p.idx("paint_blue"));
    m.cuboid((0, 4), (9, 22), (19, 19), p.idx("steel_light"));
    m.cylinder_y((2, 14), 2.4, (4, 8), p.idx("brass"));

    // Discharge chute, stepping down toward the belt on the +X side and
    // ending flush with the tile edge, just above the belt's surface.
    m.cuboid((26, 29), (11, 20), (9, 13), p.idx("steel"));
    m.cuboid((29, 31), (10, 21), (7, 11), p.idx("steel"));
    m.cuboid((29, 31), (9, 22), (6, 7), p.idx("steel_light"));

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
    m.cylinder_y((16, PIPE_AXIS_Z), 2.2, (0, 8), p.idx("pipe_grey"));
    m.cylinder_y((16, PIPE_AXIS_Z), 3.0, (6, 8), p.idx("brass"));

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
    m.cuboid((26, 28), (23, 26), (12, 27), p.idx("steel"));
    m.cuboid((16, 28), (18, 26), (26, 27), p.idx("steel_light"));
    m.cylinder_y((19, 24), 3.0, (19, 25), p.idx("brass"));
    m.cuboid((16, 18), (18, 25), (22, 24), p.idx("steel_dark"));

    control_panel(&mut m, p, (5, 14), (3, 7), 2);
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
    m.cylinder_x((c, PIPE_AXIS_Z), 2.4, (22, last), p.idx("pipe_grey"));
    m.cylinder_x((c, PIPE_AXIS_Z), 3.2, (last - 1, last), p.idx("brass"));

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
