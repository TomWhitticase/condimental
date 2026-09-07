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
    Asset { name: "pipe", build: pipe },
    Asset { name: "pipe_source", build: pipe_source },
    Asset { name: "pipe_fluid", build: pipe_fluid },
    Asset { name: "item_tomato", build: item_tomato },
    Asset { name: "item_sauce_pail", build: item_sauce_pail },
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

    // Plinth.
    m.cuboid((1, 30), (1, 30), (0, 1), p.idx("concrete_dark"));
    // Hazard skirt around the plinth top edge.
    m.hazard_stripe_x((1, 30), (1, 1), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((1, 30), (30, 30), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for y in 1i32..=30 {
        let a = if (y + 2).rem_euclid(6) < 3 { p.idx("hazard_yellow") } else { p.idx("hazard_black") };
        m.set(1, y, 2, a);
        m.set(30, y, 2, a);
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

/// The bulk intermediate (tomato sauce): a food-grade pail with a brass
/// bail handle.
pub fn item_sauce_pail(p: &Palette) -> VoxModel {
    pail(p, "pail_grey", "sauce_red", "brass")
}

/// The next intermediate (ketchup) — same silhouette, different livery, so
/// the two bulk stages stay distinguishable on the belt.
pub fn item_ketchup_pail(p: &Palette) -> VoxModel {
    pail(p, "paint_red", "ketchup_red", "steel_light")
}

fn pail(p: &Palette, body: &str, contents: &str, handle: &str) -> VoxModel {
    let mut m = item_volume();
    m.cylinder_z((8, 8), 5.0, (0, 8), p.idx(body));
    // Rolled rim, then the visible sauce surface inside it — order matters:
    // a solid disc for the rim would cap the pail and hide the contents.
    m.cylinder_z((8, 8), 5.4, (9, 9), p.idx(body));
    m.cylinder_z((8, 8), 4.2, (8, 9), p.idx(contents));
    // Bail handle.
    m.cuboid((3, 3), (8, 8), (9, 12), p.idx(handle));
    m.cuboid((13, 13), (8, 8), (9, 12), p.idx(handle));
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

/// Voxel geometry shared by every pipe tile, so a plain segment and the
/// source tile read as one continuous run. Numbers here are mirrored by
/// `main.gd`'s PIPE_* constants (stated in voxels, converted there) —
/// change one, change the other.
const PIPE_AXIS_Y: i32 = 16;
const PIPE_AXIS_Z: i32 = 15;
const PIPE_OUTER_R: f32 = 5.0;
const PIPE_BORE_R: f32 = 3.2;
/// Bore floor / ceiling in voxels, derived from the two above. The fluid
/// body's base sits on `PIPE_BORE_Z0` and it can rise `PIPE_BORE_HEIGHT`.
const PIPE_BORE_Z0: i32 = PIPE_AXIS_Z - 3;
const PIPE_BORE_HEIGHT: i32 = 7;
/// Bore half-width, in Y — the fluid body is this wide too.
const PIPE_BORE_Y0: i32 = PIPE_AXIS_Y - 3;
const PIPE_BORE_Y1: i32 = PIPE_AXIS_Y + 3;

/// Draws the steel run itself into `m`, with its sight opening spanning
/// `window` in X. Everything is carved *after* it is drawn: a face is only
/// culled when its neighbour is genuinely absent, so a bore filled with an
/// "air" colour would still mesh as a solid plug (see `VoxModel::erase`).
fn pipe_body(m: &mut VoxModel, p: &Palette, window: (i32, i32)) {
    let last = TILE_VOXELS as i32 - 1;

    // Support trestle: two legs and a saddle bracket the pipe rests in.
    for (x0, x1) in [(4, 7), (24, 27)] {
        m.cuboid((x0, x1), (14, 18), (0, 9), p.idx("steel_dark"));
        m.cuboid((x0, x1), (12, 20), (9, 10), p.idx("steel_dark"));
    }

    // The run. Full tile length, so consecutive tiles butt into one line.
    // `steel`, not the darker `pipe_grey` the machine's exhaust stack uses:
    // reviewed as a rendered scene, a run in the darker grey sank into the
    // concrete instead of popping the way UI_UX.md §4 wants machinery to.
    m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R, (0, last), p.idx("steel"));
    // Bolted flanges at the tile joins and a brass collar either side of the
    // sight section — both wider than the run, so they read as fittings.
    for (x0, x1) in [(0, 1), (last - 1, last)] {
        m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 1.0, (x0, x1), p.idx("steel_light"));
    }
    for (x0, x1) in [(5, 6), (25, 26)] {
        m.cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_OUTER_R + 1.0, (x0, x1), p.idx("brass"));
    }

    // Hollow it out, then cut the sight opening through the top wall,
    // leaving the side rails standing.
    // Cut the wall away from just above the bore floor, leaving the two
    // side rails standing: an open inspection channel, not a slot in the
    // top. Cutting only the top half was the first attempt and it was
    // wrong — it hid the liquid below roughly half fill, which is exactly
    // the state a player most needs to see (a draining pipe means a
    // stalling machine). This way the level reads at ANY fill fraction from
    // the fixed isometric angle, which looks obliquely down into the run.
    let top = m.size[2] as i32 - 1;
    let window_floor = PIPE_BORE_Z0 + 1;
    m.carve_cylinder_x((PIPE_AXIS_Y, PIPE_AXIS_Z), PIPE_BORE_R, (0, last));
    m.carve_cuboid(window, (PIPE_BORE_Y0, PIPE_BORE_Y1), (window_floor, top));
}

/// A plain length of pipe: one tile of the run.
pub fn pipe(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    pipe_body(&mut m, p, (9, 22));
    m
}

/// The off-map supply tap at the far end of the run (`PipeSegment::is_source`
/// in the simulation): the same pipe, plus a stainless header vessel it
/// feeds from. Visually distinct on purpose — UI_UX.md §7's "never encode
/// critical state in colour alone" instinct applied to geometry: the source
/// end should be identifiable by silhouette, not by being the tile that
/// happens to always look full.
pub fn pipe_source(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 32]);
    // Sight opening shifted clear of the vessel below.
    pipe_body(&mut m, p, (14, 27));

    // Plinth + hazard skirt, matching the machine's own footing language.
    m.cuboid((1, 13), (9, 23), (0, 1), p.idx("concrete_dark"));
    m.hazard_stripe_x((1, 13), (9, 9), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((1, 13), (23, 23), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));

    // Header vessel, drawn over the run so the pipe reads as leaving it.
    m.cylinder_z((7, 16), 6.0, (2, 24), p.idx("steel"));
    for z in [9, 17] {
        m.cylinder_z((7, 16), 6.4, (z, z), p.idx("steel_light"));
    }
    m.ellipsoid((7, 16, 24), (6.0, 6.0, 4.0), p.idx("steel_light"));

    // Shut-off valve on top: stem and hand wheel.
    m.cuboid((7, 7), (16, 16), (28, 29), p.idx("steel_dark"));
    m.cylinder_z((7, 16), 3.2, (30, 30), p.idx("brass"));
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
