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
    // The belt family (the "Pipes & Conveyors Overhaul" plan's ticket 2):
    // three shapes, `BELT_FRAMES` animation frames each, all authored for a
    // +X product path. The scene picks the shape from each tile's `links`
    // and the frame from the tick (ticket 5, `main.gd`'s `_belt_shape_for`
    // and `_belt_frame_for_tick`); `belt_f0` is also the build menu's icon
    // and the ghost. The old `belt`/`belt_inlet`/`belt_outlet` names are
    // gone — see the conveyor section's header.
    Asset { name: "belt_f0", build: belt_f0 },
    Asset { name: "belt_f1", build: belt_f1 },
    Asset { name: "belt_f2", build: belt_f2 },
    Asset { name: "belt_f3", build: belt_f3 },
    Asset { name: "belt_corner_left_f0", build: belt_corner_left_f0 },
    Asset { name: "belt_corner_left_f1", build: belt_corner_left_f1 },
    Asset { name: "belt_corner_left_f2", build: belt_corner_left_f2 },
    Asset { name: "belt_corner_left_f3", build: belt_corner_left_f3 },
    Asset { name: "belt_corner_right_f0", build: belt_corner_right_f0 },
    Asset { name: "belt_corner_right_f1", build: belt_corner_right_f1 },
    Asset { name: "belt_corner_right_f2", build: belt_corner_right_f2 },
    Asset { name: "belt_corner_right_f3", build: belt_corner_right_f3 },
    // Epic B's conveyor splitter and the lit arrow that says which of its two
    // outputs takes the next item. Grouped with the belts rather than with
    // the machines below because that is what a splitter is (see
    // `machine_splitter`); the `machine_` prefix on the body is only there
    // because `main.gd`'s MACHINE_MODELS keys `data/machines.ron` kinds, and
    // a splitter has a footprint in that file like every other placed thing.
    Asset { name: "machine_splitter", build: machine_splitter },
    Asset { name: "splitter_arrow", build: splitter_arrow },
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
    // The pipe family ("Pipes & Conveyors Overhaul" ticket 3): shapes, the
    // fittings Godot overlays on any open leg, and one fluid body per shape.
    // See the pipe section's header for the face convention.
    Asset { name: "pipe", build: pipe },
    Asset { name: "pipe_corner", build: pipe_corner },
    Asset { name: "pipe_tee", build: pipe_tee },
    Asset { name: "pipe_cross", build: pipe_cross },
    Asset { name: "pipe_end", build: pipe_end },
    Asset { name: "pipe_fitting_machine", build: pipe_fitting_machine },
    Asset { name: "pipe_fitting_tank", build: pipe_fitting_tank },
    Asset { name: "pipe_fluid", build: pipe_fluid },
    Asset { name: "pipe_fluid_corner", build: pipe_fluid_corner },
    Asset { name: "pipe_fluid_tee", build: pipe_fluid_tee },
    Asset { name: "pipe_fluid_cross", build: pipe_fluid_cross },
    Asset { name: "pipe_fluid_end", build: pipe_fluid_end },
    Asset { name: "item_tomato", build: item_tomato },
    // The chain's second raw ingredient (the chain-rework epic). It needed a
    // model of its own for UI_UX.md §2b point 12's reason: `main.gd`'s
    // ITEM_MODELS falls back to a shipping box, so a new item id with no
    // entry ships bulk produce down the belt in sealed cardboard and every
    // structural test still passes.
    Asset { name: "item_sugar", build: item_sugar },
    // The chain's THIRD raw ingredient, and the first *fluid* with a model
    // of its own. Nothing carries vinegar on a belt, so this is not a belt
    // item — it is what a shipment of the stuff looks like where the player
    // handles it as stock (`main.gd`'s FLUID_MODELS: the market slot and the
    // larder chip), both of which used to borrow the pipe's icon and so read
    // as "a pipe" rather than as vinegar.
    Asset { name: "item_vinegar", build: item_vinegar },
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
//
// The "Pipes & Conveyors Overhaul" plan's ticket 2
// (docs/PIPES_AND_CONVEYORS_PLAN.md). One parameterised generator produces
// the whole belt family the plan names: a straight tile, a left corner and a
// right corner, each in `BELT_FRAMES` animation frames. Every model is
// authored with its product path leaving through the **+X** face and is
// spun by Godot to the placement's facing (UI_UX.md §2b point 18), so
// nothing in here may depend on which face the camera happens to look at —
// every face of a belt can be the near face.
//
// **Animation is by frames, not UVs** (the plan's "Belt animation"). Frame
// `k` is the same tile with its surface pattern advanced `k *
// BELT_FRAME_SHIFT` voxels along the product path; the structure under it
// (legs, rails, rollers, the return strand) is identical in every frame,
// because on a real conveyor the belt moves and the idlers do not. Godot
// shows frame `floor(tick * BELT_FRAMES / BELT_TICKS_PER_TILE) mod
// BELT_FRAMES`, so the pattern crosses one tile in exactly the 50 ticks an
// item does. The pattern's period along the path is therefore one tile: a
// shorter period (the old cleats every 8 voxels) would make frames 0 and 2 —
// or all four — identical, and a pattern that moves 8 voxels a frame with
// features 8 or 16 apart is the wagon-wheel illusion (built and reasoned
// through, not rendered: two marks a tile could be read as standing still
// or running backwards). So each tile carries **one** arrow, and the
// arrow is the cleat.
//
// **Direction is a shape** (UI_UX.md §7 — never colour alone): a solid
// arrow — a wedge head and a short shaft — standing one voxel proud of the
// rubber, in `hazard_yellow`, the same safety-signage colour the belt's
// rails already wear. The arrow is the cleat.
//
// A **solid** arrow, not a two-arm chevron, and that was decided by the
// render, not by taste (UI_UX.md §2b point 23). The first cut was a thin
// `>` of two diagonal arms, and in the `--closeup` scene render it was an
// `L`: the fixed isometric camera looks straight down one of the tile's
// diagonals, so a line along one diagonal projects to a screen horizontal
// at full length and a line along the other to a short screen vertical. A
// 45° mark on the tile plane cannot read as a chevron from this camera at
// any facing. A filled wedge has no lines to lose — its apex is its apex
// from every angle — which is also why the splitter's readout is a filled
// arrow. **Raised rather than recessed**, for the camera again: a recess
// shows its floor only where its own wall does not shadow it, and one edge
// of any shape always runs along the sight line; a plate is seen from
// above at every facing. The cost is that an item drawn at
// `BELT_SURFACE_Z + 1` stands one voxel into the plate where they overlap;
// at scene scale that is a sliver under the item's foot (checked in the
// `--closeup --populate` render).
//
// **Corners** (the plan's "Corner conventions"): a corner's running surface
// is a quarter annulus about the tile corner between its two connected
// faces, centre-line radius half a tile, so the lane enters and leaves at
// exactly `BELT_LANE` and the rails meet a straight tile's rails voxel for
// voxel. `Turn::Left` is fed from the NORTH face, which is vox **low Y**
// (sim +Y is world +Z is vox -Y; see the junction header further down);
// `Turn::Right` is its mirror in Y. The surface pattern on a corner lives in
// the same `(u, v)` coordinates as on a straight — `u` runs 0..32 along the
// path, so a quarter turn is stretched to a tile's worth of pattern and the
// arrow reaches the exit face in the same frame a straight tile's would;
// the sim already parameterises an item across a corner by the same 0..1
// distance, so surface and items stay in step. The arrow's length is in
// `u` and its width in voxels, so on the inside of the bend the head is
// short and on the outside long: the arrow bends with the belt.
//
// **The belt-side junction variants are retired.** `belt_inlet` and
// `belt_outlet` used to dress the last tile before a machine and the first
// after it with a static steel dead-plate and kerbs. They were east-only
// (point 18) and fell back to the plain tile on three facings out of four,
// and a static plate on an animated surface would either freeze the pattern
// short of the machine or have to be excluded from the frame rule; with
// corners a corner can also be the tile that meets a machine, multiplying
// the set again. The machine-side hood and doorway (`machine_infeed`,
// `machine_outfeed`) already bring a threshold to the tile boundary at
// `BELT_SURFACE_Z`, so the plain tile carries the junction. Ticket 5 wired
// the framed names into the scene and deleted the three aliases (`belt`,
// `belt_inlet`, `belt_outlet`), their models and their icons: a belt meeting
// a machine, hopper or dock is `belt_f0..f3` or a corner at every facing,
// and nothing on the belt's side of the boundary is facing-specific. `belt()`
// below stays as a Rust function only — it is what the junction tests and
// the splitter's stubs are measured against — and is not in `ASSETS`.

/// How many animation frames every belt shape has (the plan's
/// `BELT_FRAMES`). `main.gd` mirrors this; change one, change both.
pub const BELT_FRAMES: u32 = 4;

/// Voxels the surface pattern advances per frame along the product path:
/// a quarter tile, so four frames carry it exactly one tile.
pub const BELT_FRAME_SHIFT: i32 = TILE_VOXELS as i32 / BELT_FRAMES as i32;

/// The arrow cleat's top voxel: one proud of the running surface (see the
/// section header for why raised, not recessed).
pub const BELT_ARROW_Z: i32 = BELT_SURFACE_Z + 1;

/// Where the arrow's apex sits in frame 0, in `u` (voxels along the path).
/// Chosen so the frame-0 arrow — the one the UI icon is rendered from — is
/// centred in its tile: it spans `u` 9..=22.
const ARROW_APEX_U: f32 = 23.0;
/// Length of the wedge head along the path, in `u`.
const ARROW_HEAD_LEN: f32 = 8.0;
/// Half-width of the wedge at its base, in voxels across the lane: the base
/// spans `BELT_LANE` less one voxel of rubber margin each side.
const ARROW_HEAD_HALF_WIDTH: f32 = 8.0;
/// Length of the shaft behind the head, in `u`.
const ARROW_SHAFT_LEN: f32 = 6.0;
/// Half-width of the shaft, in voxels: four rows either side of the centre.
const ARROW_SHAFT_HALF_WIDTH: f32 = 2.5;

/// Which way a corner turns, named as the plan names the models. Both are
/// authored with their output on +X; `Left` is fed from the north (vox low
/// Y) face, `Right` from the south (vox high Y) face.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Turn {
    Left,
    Right,
}

/// The three belt shapes the family covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BeltShape {
    Straight,
    Corner(Turn),
}

/// The straight belt tile at rest — frame 0 of the straight shape. Not an
/// `ASSETS` entry (the scene asks for `belt_f0`); kept as the reference the
/// junction tests and the splitter's stubs are measured against.
pub fn belt(p: &Palette) -> VoxModel {
    belt_shape(p, BeltShape::Straight, 0)
}

macro_rules! belt_frame_fns {
    ($($name:ident => $shape:expr, $phase:expr;)*) => {
        $(
            pub fn $name(p: &Palette) -> VoxModel {
                belt_shape(p, $shape, $phase)
            }
        )*
    };
}

belt_frame_fns! {
    belt_f0 => BeltShape::Straight, 0;
    belt_f1 => BeltShape::Straight, 1;
    belt_f2 => BeltShape::Straight, 2;
    belt_f3 => BeltShape::Straight, 3;
    belt_corner_left_f0 => BeltShape::Corner(Turn::Left), 0;
    belt_corner_left_f1 => BeltShape::Corner(Turn::Left), 1;
    belt_corner_left_f2 => BeltShape::Corner(Turn::Left), 2;
    belt_corner_left_f3 => BeltShape::Corner(Turn::Left), 3;
    belt_corner_right_f0 => BeltShape::Corner(Turn::Right), 0;
    belt_corner_right_f1 => BeltShape::Corner(Turn::Right), 1;
    belt_corner_right_f2 => BeltShape::Corner(Turn::Right), 2;
    belt_corner_right_f3 => BeltShape::Corner(Turn::Right), 3;
}

/// One belt tile of `shape` at animation `phase` (`0..BELT_FRAMES`).
pub fn belt_shape(p: &Palette, shape: BeltShape, phase: u32) -> VoxModel {
    assert!(phase < BELT_FRAMES, "belt phase {phase} is not in 0..{BELT_FRAMES}");
    match shape {
        BeltShape::Straight => belt_straight(p, phase),
        BeltShape::Corner(turn) => belt_corner(p, turn, phase),
    }
}

/// A straight tile: the shared structure, the rubber lane, and the arrow.
fn belt_straight(p: &Palette, phase: u32) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    let last = TILE_VOXELS as i32 - 1;
    let (l0, l1) = BELT_LANE;

    // Legs.
    for (x0, x1) in [(2, 4), (27, 29)] {
        for (y0, y1) in [(5, 7), (24, 26)] {
            m.cuboid((x0, x1), (y0, y1), (0, 4), p.idx("steel_dark"));
        }
    }
    // Return strand: the belt's lower run, between the legs. Structure, not
    // pattern — it is what stops a side view showing bare floor under a
    // striped rail.
    m.cuboid((0, last), (l0 + 2, l1 - 2), (2, 2), p.idx("rubber_dark"));
    // Side frames, running the full length of the tile so consecutive tiles
    // read as one continuous line.
    for (y0, y1) in [(4, 6), (25, 27)] {
        m.cuboid((0, last), (y0, y1), (4, 8), p.idx("steel_dark"));
    }
    // Hazard striping on the outer face of each frame (UI_UX.md §3). Both
    // rails, because either can be the camera-facing one once the tile is
    // turned.
    m.hazard_stripe_x((0, last), (4, 4), (5, 6), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((0, last), (27, 27), (5, 6), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    // Idlers under the belt. Fixed in every frame: the belt moves over
    // them, they do not travel with it. The two nearest the faces are what
    // shows under the rubber's end where a run stops in the open.
    for x in [4, 12, 20, 28] {
        m.cylinder_y((x, 7), 2.5, (l0, l1), p.idx("brass"));
    }

    // Running surface, then the arrow standing on it.
    m.cuboid((0, last), (l0, l1), (BELT_SURFACE_Z - 1, BELT_SURFACE_Z), p.idx("rubber"));
    for x in 0..=last {
        for y in l0..=l1 {
            let cell = straight_surface_cell(x, y).expect("lane cell");
            if arrow_covers(&cell, phase) {
                m.set(x, y, BELT_ARROW_Z, p.idx("hazard_yellow"));
            }
        }
    }
    m
}

/// A corner tile. Built as the LEFT corner (fed from vox low Y) and
/// mirrored in Y for the right, so the two cannot drift apart.
fn belt_corner(p: &Palette, turn: Turn, phase: u32) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    let last = TILE_VOXELS as i32 - 1;
    let (l0, l1) = BELT_LANE;

    // Legs: one pair at each open face where a straight tile has its own,
    // and one under the middle of the outer rail.
    for (x, y) in [(27, 5), (27, 24), (5, 2), (24, 2), (12, 17)] {
        m.cuboid((x, x + 2), (y, y + 2), (0, 4), p.idx("steel_dark"));
    }
    // Idlers at the two open faces, matching a straight tile's end idlers.
    m.cylinder_y((28, 7), 2.5, (l0, l1), p.idx("brass"));
    m.cylinder_x((3, 7), 2.5, (l0, l1), p.idx("brass"));

    for x in 0..=last {
        for y in 0..=last {
            let (r, _) = corner_polar(x, y);
            // Rails: the inner one hugs the pivot corner, the outer one the
            // far corner; both meet a straight tile's rails at the faces.
            if (4.0..7.0).contains(&r) || (25.0..28.0).contains(&r) {
                m.cuboid((x, x), (y, y), (4, 8), p.idx("steel_dark"));
                // Striping on the outer skin of each rail, the faces that
                // look away from the lane.
                if r < 5.0 || r >= 27.0 {
                    for z in 5..=6 {
                        let c = if (x + y + z).rem_euclid(6) < 3 { "hazard_yellow" } else { "hazard_black" };
                        m.set(x, y, z, p.idx(c));
                    }
                }
            }
            if (9.0..23.0).contains(&r) {
                m.set(x, y, 2, p.idx("rubber_dark"));
            }
            if let Some(cell) = corner_surface_cell(x, y) {
                m.cuboid((x, x), (y, y), (BELT_SURFACE_Z - 1, BELT_SURFACE_Z), p.idx("rubber"));
                if arrow_covers(&cell, phase) {
                    m.set(x, y, BELT_ARROW_Z, p.idx("hazard_yellow"));
                }
            }
        }
    }

    match turn {
        Turn::Left => m,
        Turn::Right => mirror_y(&m),
    }
}

/// The same model reflected in Y about the tile's centre.
fn mirror_y(m: &VoxModel) -> VoxModel {
    let mut out = VoxModel::new(m.size);
    let last = m.size[1] - 1;
    for (&(x, y, z), &c) in &m.voxels {
        out.voxels.insert((x, last - y, z), c);
    }
    out
}

/// Where a running-surface cell sits along and across the product path.
///
/// `u` runs 0..32 along the path in frame units (one voxel on a straight,
/// a stretched quarter turn on a corner). `v` is the lateral offset of the
/// cell centre from the lane's centre line, in voxels, positive toward vox
/// +Y at the exit face.
pub struct SurfaceCell {
    pub u: f32,
    pub v: f32,
}

/// Surface coordinates of a straight tile's cell, or `None` off the lane.
pub fn straight_surface_cell(x: i32, y: i32) -> Option<SurfaceCell> {
    let (l0, l1) = BELT_LANE;
    if !(0..TILE_VOXELS as i32).contains(&x) || !(l0..=l1).contains(&y) {
        return None;
    }
    Some(SurfaceCell { u: x as f32 + 0.5, v: y as f32 + 0.5 - TILE_VOXELS as f32 / 2.0 })
}

/// Polar coordinates of a cell centre about the LEFT corner's pivot, the
/// tile corner at vox `(32, 0)`: `r` in voxels, `theta` from 0 at the
/// entry (low-Y) face to π/2 at the exit (+X) face.
fn corner_polar(x: i32, y: i32) -> (f32, f32) {
    let dx = TILE_VOXELS as f32 - (x as f32 + 0.5);
    let dy = y as f32 + 0.5;
    ((dx * dx + dy * dy).sqrt(), dy.atan2(dx))
}

/// Surface coordinates of a LEFT corner's cell, or `None` off the lane.
/// (The right corner is the mirror; test it through [`mirror_y`]'s
/// inverse, which is itself.)
pub fn corner_surface_cell(x: i32, y: i32) -> Option<SurfaceCell> {
    let (l0, l1) = BELT_LANE;
    let (r_in, r_out) = (l0 as f32, l1 as f32 + 1.0);
    let (r, theta) = corner_polar(x, y);
    if !(r_in..r_out).contains(&r) {
        return None;
    }
    let quarter = std::f32::consts::FRAC_PI_2;
    let u_per_rad = TILE_VOXELS as f32 / quarter;
    Some(SurfaceCell { u: theta * u_per_rad, v: r - TILE_VOXELS as f32 / 2.0 })
}

/// Distance wrapped into one tile's pattern period, `-16..16`.
fn wrap_period(d: f32) -> f32 {
    let half = TILE_VOXELS as f32 / 2.0;
    (d + half).rem_euclid(TILE_VOXELS as f32) - half
}

/// The arrow apex's `u` at `phase`.
pub fn arrow_apex_u(phase: u32) -> f32 {
    (ARROW_APEX_U + (phase as i32 * BELT_FRAME_SHIFT) as f32).rem_euclid(TILE_VOXELS as f32)
}

/// How far behind the arrow's apex a cell sits along the path, in `u`,
/// wrapped to the pattern period (negative = ahead of the apex).
pub fn arrow_trail_u(cell: &SurfaceCell, phase: u32) -> f32 {
    wrap_period(arrow_apex_u(phase) - cell.u)
}

/// Does the arrow at `phase` stand on this cell? The head is a wedge that
/// widens behind the apex — that is what makes it point the way the product
/// goes — and the shaft a constant-width strip behind the head.
pub fn arrow_covers(cell: &SurfaceCell, phase: u32) -> bool {
    let trail = arrow_trail_u(cell, phase);
    if trail < 0.0 {
        false
    } else if trail < ARROW_HEAD_LEN {
        cell.v.abs() < trail / ARROW_HEAD_LEN * ARROW_HEAD_HALF_WIDTH
    } else if trail < ARROW_HEAD_LEN + ARROW_SHAFT_LEN {
        cell.v.abs() < ARROW_SHAFT_HALF_WIDTH
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Conveyor splitter (Epic B, ticket 2 — the presentation half)
// ---------------------------------------------------------------------------
//
// `sim_core::sim::splitter` made a line able to fork; these two assets are
// what a player actually sees when it does. They live in the conveyor
// section rather than with the machines on purpose: a splitter is priced,
// sized and timed as a belt tile (`data/shop.ron` lists it with the
// infrastructure, `data/machines.ron` makes it 1x1, and it carries product
// in an ordinary one-tile `BeltSegment` so an item crosses it in the same 50
// ticks) — it is a belt tile with a choice at the end of it, and the model
// says so.
//
// **THE GEOMETRY, in this file's coordinates.** Every asset here is authored
// facing EAST, and east is vox +X. The splitter's second output is one
// quarter turn CLOCKWISE from its facing (`splitter.rs` owns that rule), so
// for an east-facing splitter the branch runs SOUTH — and south is world -Z,
// which this file's junction header establishes is vox **+Y**. So the model
// is a T laid out as: intake from -X, straight exit at +X, branch exit at
// +Y, and a closed rail along -Y. That closed rail is the near, camera-facing
// edge (vox low-Y is world +Z), which is the right side to close: it is a
// belt-height frame, so it frames the deck without standing in front of it.
//
// **WHY THE READOUT IS INLAID IN THE DECK.** `SplitterSnapshot::next_output`
// is the one piece of a splitter's state a player cannot infer from the tiles
// around it, and it changes with every item, so it needs to be legible at a
// glance and it must never be in the way of the product crossing the table.
// A raised arm or a swinging gate is both: at this scale it would occlude
// the very lane it describes, and items are drawn sliding across the deck at
// `BELT_SURFACE_Z`, so anything standing proud of the table gets driven
// through. So both output directions are cut into the table as **arrow
// grooves** and the live asset ([`splitter_arrow`]) fills one of them flush,
// emissive — the fifth use of the "one flat slab, positioned by main.gd from
// live data" mechanism `pipe_fluid`, `hopper_fill` and `fluid_tank_fill`
// already share, and the first that carries a DIRECTION rather than a
// level.
//
// The dark groove is not decoration either: it is what makes the readout a
// pair of positions rather than a colour (UI_UX.md §7). An empty groove and a
// lit one are two different shapes on the deck, so "which way next" survives
// being read in one glance, in a screenshot, or by someone who cannot tell
// amber from steel.

/// The steel diverter table in the middle of a splitter, in vox coordinates.
/// Deliberately the same span on both axes: the tile has two outputs at
/// right angles and neither is the special one.
const SPLITTER_TABLE: (i32, i32) = BELT_LANE;

/// Floor of the arrow grooves — one voxel below the deck's top, so a groove
/// is a real recess and [`splitter_arrow`] sits flush in it rather than
/// standing on the table. `main.gd` mirrors this as
/// SPLITTER_ARROW_FLOOR_VOXELS; change one, change both.
pub const SPLITTER_ARROW_Z: i32 = BELT_SURFACE_Z;

/// The cells of the STRAIGHT-through arrow (pointing +X), in vox (x, y).
///
/// It sits **at the exit lip, not at the tile centre**, and that is what
/// makes one asset serve both outputs: two arrows drawn from the middle of
/// the table cross each other near the centre and render as one unreadable
/// blob (built, looked at, rejected — UI_UX.md §2b point 9's rule that the
/// render, not the generator, is what you review). Pushed out to the lips
/// the two grooves are disjoint, and each one sits where the product it
/// describes actually leaves.
///
/// The branch arrow is this one rotated a quarter turn clockwise about the
/// tile centre — `(dx, dy) -> (-dy, dx)` — computed by [`splitter_arrow_cells`]
/// rather than typed out a second time, for exactly the reason the simulation
/// derives the branch output from the facing instead of storing it: two
/// hand-written copies of one quarter turn are two things that can disagree.
fn splitter_straight_arrow() -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    // Shaft.
    for x in 22..=26 {
        for y in 14..=17 {
            cells.push((x, y));
        }
    }
    // Head, tapering to the lip.
    for (i, x) in (27..=30).enumerate() {
        let half = 5 - i as i32;
        for y in (16 - half)..=(15 + half) {
            cells.push((x, y));
        }
    }
    cells
}

/// One quarter turn clockwise about the tile centre (15.5, 15.5), the same
/// turn `Direction::rotate_cw` applies — `(dx, dy) -> (-dy, dx)` — applied
/// `turns` times.
fn splitter_arrow_cells(turns: u32) -> Vec<(i32, i32)> {
    let mut cells = splitter_straight_arrow();
    for _ in 0..turns {
        cells = cells
            .into_iter()
            .map(|(x, y)| {
                // Doubled coordinates so the half-voxel centre stays exact.
                let (dx, dy) = (2 * x - 31, 2 * y - 31);
                (((-dy) + 31) / 2, (dx + 31) / 2)
            })
            .collect();
    }
    cells
}

/// A conveyor splitter: one belt in, two belts out.
///
/// See this section's header for the layout and for why the next-output
/// readout is inlaid in the deck rather than standing on it.
pub fn machine_splitter(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    let last = TILE_VOXELS as i32 - 1;
    let (l0, l1) = BELT_LANE;
    let (t0, t1) = SPLITTER_TABLE;
    let deck = (BELT_SURFACE_Z - 1, BELT_SURFACE_Z);

    // Legs: the belt's own four, plus two more carrying the branch stub, so
    // the tile stands on something wherever it has deck.
    for (x0, x1) in [(2, 4), (27, 29)] {
        for (y0, y1) in [(5, 7), (24, 26)] {
            m.cuboid((x0, x1), (y0, y1), (0, 4), p.idx("steel_dark"));
        }
    }
    for (x0, x1) in [(4, 6), (25, 27)] {
        m.cuboid((x0, x1), (28, 30), (0, 4), p.idx("steel_dark"));
    }

    // Side frames at exactly the belt's own height, so a splitter dropped
    // into a run reads as part of the line. The -Y rail is unbroken (that
    // side has no output); the +Y one is only the two stubs either side of
    // the branch mouth, and the branch's own rails carry on from them.
    m.cuboid((0, last), (4, 6), (4, 8), p.idx("steel_dark"));
    for (x0, x1) in [(0, l0 - 1), (l1 + 1, last)] {
        m.cuboid((x0, x1), (25, 27), (4, 8), p.idx("steel_dark"));
    }
    for (x0, x1) in [(4, 6), (25, 27)] {
        m.cuboid((x0, x1), (25, last), (4, 8), p.idx("steel_dark"));
    }

    // Hazard striping on the outer face of the rails the camera can see
    // (UI_UX.md §3), matching `belt_body`'s.
    m.hazard_stripe_x((0, last), (4, 4), (5, 6), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));

    // Rollers: one pair under the through lane, one under the branch stub.
    for x in [3, 28] {
        m.cylinder_y((x, 7), 2.5, (l0, l1), p.idx("brass"));
    }
    m.cylinder_x((28, 7), 2.5, (l0, l1), p.idx("brass"));

    // Running surfaces. Plain rubber where product travels in a straight
    // line — the intake, the straight exit and the branch stub — at the
    // belt's own height and lane, so the three stubs line up with whatever
    // belt tile they are laid against. Deliberately NO arrow cleat here:
    // the belt family's arrow says "this way", and on a splitter that is
    // the lit arrow's job — a static yellow mark beside an amber one would
    // be a second direction readout on the one tile that has a live one.
    for (x0, x1) in [(0, t0 - 1), (t1 + 1, last)] {
        m.cuboid((x0, x1), (l0, l1), deck, p.idx("rubber"));
    }
    m.cuboid((l0, l1), (t1 + 1, last), deck, p.idx("rubber"));

    // The diverter table itself: stainless, because it is the one part of a
    // splitter that is a mechanism rather than a surface, and it is what
    // makes the tile tell itself apart from the belt run it sits in.
    //
    // `steel` and NOT `steel_light`, which is what it was first built with:
    // rendered in the scene the palette's brightest metal is a blown-out
    // white square filling most of the tile, and everything cut into it
    // (both arrows, the seam) washes out against it. Mid steel keeps the
    // "this is machined, that is rubber" contrast and gives the grooves
    // something to be dark against.
    m.cuboid((t0, t1), (t0, t1), deck, p.idx("steel"));
    // Seam and corner bolts, so the table reads as a fabricated plate rather
    // than as a hole in the belt. One voxel, at the deck's own top: any more
    // and it becomes a lip for product to catch on.
    for i in t0..=t1 {
        for (x, y) in [(i, t0), (i, t1), (t0, i), (t1, i)] {
            m.set(x, y, deck.1, p.idx("steel_dark"));
        }
    }
    for (x, y) in [(t0 + 2, t0 + 2), (t1 - 2, t0 + 2), (t0 + 2, t1 - 2), (t1 - 2, t1 - 2)] {
        m.set(x, y, deck.1, p.idx("steel_dark"));
    }

    // Brass threshold at each of the two exit lips — the same accent every
    // other "product passes through here" fitting in this factory carries,
    // and here it is what marks the tile as having TWO ways out.
    m.cuboid((last, last), (l0, l1), (deck.1, deck.1), p.idx("brass"));
    m.cuboid((l0, l1), (last, last), (deck.1, deck.1), p.idx("brass"));

    // The two arrow grooves, one at each exit lip. Cut, not painted: a
    // painted arrow is a decal the deck's own shading flattens, where a
    // recess keeps an edge at every angle and gives `splitter_arrow`
    // somewhere to sit flush instead of standing in the product's way.
    //
    // Their floor is `steel_dark`, and that is the one colour in the palette
    // that works here: an arrow spans the machined table AND the rubber lane
    // it discharges onto, so the near-black used inside every other opening
    // in this factory would vanish on the rubber and the bright steel used
    // to frame them would vanish on the table. Mid-dark reads against both.
    // Brass was tried first, on the reasoning that it is this factory's
    // "product passes through here" accent, and the render settled it: brass
    // and the lit amber are the same colour at scene scale, so the readout
    // had two arrows and no way to tell which one was on.
    //
    // Both grooves are cut whether or not either is lit, which is the whole
    // of UI_UX.md §7's never-colour-alone rule here: what the player reads is
    // WHICH OF TWO POSITIONS is glowing, and both positions are visible
    // shapes on the deck even with nothing lit at all.
    for turns in [0, 1] {
        for (x, y) in splitter_arrow_cells(turns) {
            m.erase(x, y, SPLITTER_ARROW_Z);
            m.set(x, y, SPLITTER_ARROW_Z - 1, p.idx("steel_dark"));
        }
    }

    // Drive cabinet on the near (-Y) edge, ahead of the rail and clear of
    // every lane: the one piece of the tile that says a splitter is powered
    // kit rather than a bend in a conveyor. Kept below the deck's own height
    // so it cannot stand in front of the table it belongs to.
    m.cuboid((21, 28), (0, 3), (0, 7), p.idx("paint_blue"));
    m.cuboid((21, 28), (0, 3), (8, 8), p.idx("steel_light"));
    m.cuboid((23, 26), (0, 0), (3, 5), p.idx("status_green"));
    m
}

/// The lit arrow standing in one of the splitter's two deck grooves — the
/// live half of the readout, and the fourth asset built to the same
/// one-flat-slab pattern (`pipe_fluid`, `hopper_fill`, `fluid_tank_fill`).
///
/// **One voxel tall, and authored in the STRAIGHT position only.** `main.gd`
/// turns this node a further quarter turn clockwise to show the branch —
/// which is not a rendering trick but the simulation's own rule reused: the
/// branch output *is* the facing rotated by `Direction::rotate_cw`, so the
/// arrow that points at it is the straight arrow rotated by the same quarter
/// turn. One asset, one rotation, and no second arrow that could end up
/// pointing somewhere the simulation does not.
///
/// `status_amber`, and therefore the `emissive` class, for the reason
/// UI_UX.md §7 gives: this is a status readout, which is what that section
/// reserves saturated colour for, and a deck inlay that did not
/// emit would be a dark shape on a dark shape at scene scale.
pub fn splitter_arrow(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 1]);
    for (x, y) in splitter_straight_arrow() {
        m.set(x, y, 0, p.idx("status_amber"));
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

/// One sack of refined sugar — the second thing a hopper can hold since the
/// chain rework gave the mixer a second solid ingredient.
///
/// A **sack**, not a heap or a crystal, because at half-tile scale on a
/// moving belt an item is read by silhouette alone and a heap of white
/// grains is a shapeless blob beside `item_tomato`'s round red one. A
/// slumped paper sack with a folded top and a spill of crystal at its mouth
/// is instantly a bulk dry ingredient, and it is what sugar actually arrives
/// in. The sack is widest at its base and narrows toward the fold, so it
/// reads as full and soft rather than as a box.
pub fn item_sugar(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    // Slumped body: a bulging ellipsoid sitting flat on the belt, so the
    // sack reads as soft and full. The first cut stacked three cuboids
    // stepping inward and rendered as a ziggurat — a stepped block is what
    // a sack is not.
    m.ellipsoid((8, 8, 4), (5.5, 4.5, 5.0), p.idx("sack_kraft"));
    m.cuboid((4, 11), (5, 10), (0, 1), p.idx("sack_kraft"));
    // The gathered fold at the top, pinched to a ridge along X.
    m.cuboid((5, 10), (7, 8), (9, 10), p.idx("sack_kraft"));
    m.cuboid((5, 10), (7, 8), (11, 11), p.idx("cardboard_dark"));
    // Crystal spilling out of the mouth, and a scatter of it on the base —
    // the one bright thing on the item, and what says "sugar" rather than
    // "flour" or "salt" at a glance is really the pairing of the two.
    m.cuboid((6, 9), (7, 8), (12, 12), p.idx("sugar_white"));
    for (x, y) in [(2, 6), (13, 9), (7, 3), (11, 12)] {
        m.set(x, y, 0, p.idx("sugar_white"));
    }
    m
}

/// One jug of vinegar — the chain's third raw ingredient, and the only one
/// that is a *fluid*.
///
/// Nothing carries vinegar on a belt (it is bought, poured into a tank and
/// piped), so unlike `item_tomato`/`item_sugar` this model exists purely for
/// the places the player handles the stuff as stock: the market slot and the
/// larder chip. Both used to fall through to the pipe's own icon — UI_UX.md
/// §2b point 12's failure in its quiet form, where nothing is *wrong* but
/// there is no way to tell the ingredient from the plumbing.
///
/// **A jug, not a bottle**, deliberately: `item_bottle` (finished ketchup) is
/// already a tall capped cylinder, and at 16 voxels two capped cylinders a
/// hue apart are one object drawn twice. This one is squat and bellied with
/// a pouring bail, which reads as a bulk ingredient arriving rather than a
/// product leaving.
///
/// The liquid stops a band below the shoulder and glass carries on above it
/// — the same "you are looking INTO a container" trick `pail` plays, since a
/// jug filled to its cap is just an amber block.
///
/// **Which way it faces is the one thing this asset does differently from
/// every other item, and it is not an oversight.** The rest are authored for
/// the *game's* camera, which only ever sees +X and low Y (UI_UX.md §2b
/// point 11); `vengi`'s previewer looks from the opposite corner (point 22),
/// which is why `item_box`'s shipping labels are invisible in its own icon.
/// For a model whose ONLY consumer is that render — vinegar cannot ride a
/// belt, so nothing else ever draws it — the previewer's corner IS the
/// camera, so the bail hangs off **-X** and the label wraps the belly all
/// the way round. Put either on the game-facing side and the icon is a bare
/// amber pot. If vinegar ever becomes something a belt carries, this
/// choice, not the model, is what has to be revisited.
///
/// The label is painted onto whichever voxels are actually on the surface at
/// its height, so it hugs the belly's curve instead of standing off it as a
/// slab, and it is `label_cream` rather than printed dark: a dark accent on
/// a face turned toward the viewer reads as a hole (UI_UX.md §2b point 21).
pub fn item_vinegar(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    let glass = p.idx("vinegar_glass");
    let amber = p.idx("vinegar_amber");

    // The vessel: a tucked foot, a tall straight belly, and a shoulder that
    // curves into the neck. **Tall and straightish rather than squat and
    // round**, which is a consequence of the camera and not a style choice:
    // an isometric view looks down, so a squat object shows the viewer
    // mostly its own lid. The first cut was a fat ovoid and its icon was a
    // pale glass plate with a rim of amber around it — the liquid, which is
    // the entire point of the asset, was the smallest thing in the picture.
    m.cylinder_z((8, 8), 3.8, (0, 0), glass);
    m.cylinder_z((8, 8), 4.2, (1, 8), glass);
    m.cylinder_z((8, 8), 3.8, (9, 9), glass);
    m.cylinder_z((8, 8), 3.2, (10, 10), glass);
    m.cylinder_z((8, 8), 2.4, (11, 11), glass);
    // Everything below the fill line is the liquid; the glass above it is
    // the headspace that says the jug is a container and not a solid.
    for z in 0..=9 {
        for y in 0..ITEM_VOXELS as i32 {
            for x in 0..ITEM_VOXELS as i32 {
                if m.filled(x, y, z) {
                    m.set(x, y, z, amber);
                }
            }
        }
    }
    // Neck and screw cap. The cap is the one small bright metal thing on the
    // item, at the top of the silhouette where it does the most work to
    // separate a jug from `item_bottle`'s tall cylinder.
    m.cylinder_z((8, 8), 1.5, (12, 13), glass);
    m.cylinder_z((8, 8), 2.2, (14, 15), p.idx("steel_light"));

    // The wrap-around label: every voxel of the belly's surface across three
    // rows. A surface voxel is one with an empty neighbour to its side —
    // testing sideways only, so the band never creeps over the shoulder.
    let cream = p.idx("label_cream");
    for z in 4..=5 {
        for y in 0..ITEM_VOXELS as i32 {
            for x in 0..ITEM_VOXELS as i32 {
                if !m.filled(x, y, z) {
                    continue;
                }
                let exposed = !m.filled(x + 1, y, z)
                    || !m.filled(x - 1, y, z)
                    || !m.filled(x, y + 1, z)
                    || !m.filled(x, y - 1, z);
                if exposed {
                    m.set(x, y, z, cream);
                }
            }
        }
    }

    // The moulded ear, from the shoulder back down onto the belly on the -X
    // side (see this function's doc for why that side). Glass, not steel: a
    // second bright metal part would compete with the cap, and the cap is
    // doing the work of topping the silhouette.
    m.cuboid((3, 5), (8, 8), (9, 9), glass);
    m.cuboid((3, 3), (8, 8), (6, 9), glass);
    m.cuboid((3, 4), (8, 8), (6, 6), glass);
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
/// UI_UX.md §2b point 12 records, so it is worth not keeping one around.
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
// Fluid pipes (milestone 6 — ARCHITECTURE.md §3.4; the shape family is the
// "Pipes & Conveyors Overhaul" ticket 3, docs/PIPES_AND_CONVEYORS_PLAN.md)
// ---------------------------------------------------------------------------
//
// Material call, made deliberately rather than left ambiguous: this run is
// **stainless steel, material class `metal`** (`steel_*`/`brass`), not PVC.
// UI_UX.md §3 lists "stainless steel tanks ... steel piping" as the
// setting's visual language and §4 makes metals the deliberate shiny
// exception so "important machinery pops" — a pipe run feeding a machine is
// exactly that, and a matte plastic run would visually sink into the
// concrete floor it stands on. GAMEPLAY.md §2/§4 reserve non-standard
// piping material as a *later* gate (tier 5's acid-resistant equipment), so
// the baseline run reads as ordinary food-grade stainless.
//
// **THE FAMILY.** A pipe tile is drawn from three kinds of part, and
// `main.gd` (ticket 6) composes them from the `links` the sim puts on the
// wire, never from geometry it works out itself:
//
//   * a SHAPE — `pipe` (straight), `pipe_corner`, `pipe_tee`, `pipe_cross`,
//     `pipe_end` — decided by how many faces connect and how they lie;
//   * a FLUID BODY per shape — `pipe_fluid`, `pipe_fluid_corner`, ... — one
//     voxel tall, filling that shape's bore footprint, scaled to the live
//     level (UI_UX.md §2b point 5);
//   * zero or more FITTINGS — `pipe_fitting_machine`, `pipe_fitting_tank` —
//     authored on the +X face and overlaid, rotated, on whichever face
//     touches a machine or a tank.
//
// **COORDINATES, stated once because the plan and this file name the same
// face two ways.** Every asset is authored with its product path along +X
// (east). The plan's corner conventions say a corner joins "+X and +Y
// (north)" *in tile space*. Tile-space north is sim +y, which is world +Z,
// which the vox importer maps from vox **LOW Y** (this file's junction
// header derives that; `machine_mixer`'s vinegar stub reaches its north
// neighbour through y = 0). So in the coordinates below:
//
//     +X face  = x = 31 = east          -X face = x = 0  = west
//     y = 0    = NORTH (camera-facing)  y = 31  = SOUTH
//
// `pipe_corner` is open on x = 31 and y = 0; `pipe_tee` on x = 31, x = 0
// and y = 0; `pipe_end` on x = 0 only. Both of a corner's openings are
// therefore on faces the fixed camera can see (§2b point 11), which is why
// the plan chose that pair. Godot turns a shape by `Direction::rotate_cw`
// quarter turns, `(dx, dy) -> (-dy, dx)` here — `VoxModel::rotated_cw` is
// the same turn and is what the tests below use.
//
// **THE AXIS IS ON THE TILE'S MIDLINE, BETWEEN VOXELS 15 AND 16.** The run
// used to be centred on voxel 16, half a voxel south of the midline, and
// that was invisible while every tile faced east. It is not invisible now:
// Godot spins a tile about its footprint centre, so an east tile's axis sat
// at y = 16, a west tile's at y = 15, a south tile's at x = 15 and a north
// tile's at x = 16 — a run at mixed facings stepped by a voxel at every
// joint, and no corner could ever have met both of its neighbours. Every
// shape here is built on `PIPE_C` = 15.5 with the `*_f` primitives, so a
// turned tile lands on exactly the voxels its neighbour expects. The
// machines' own nozzles (`machine_fluid_port`, `fluid_tank`'s spout) still
// sit on voxel 16; a radius-3 nozzle half a voxel off inside a six-wide bore
// is not a step anyone can see, and moving them is a machine-art change
// this ticket does not make.

/// The tile midline in voxel-index units: the pipe's axis on both
/// horizontal axes. See the section header for why it is not 16.
const PIPE_C: f32 = 15.5;
/// Height of the axis. Shared with every machine nozzle and the tank spout,
/// which is what makes a pipe tile meet them without an elbow.
const PIPE_AXIS_Z: i32 = 15;
/// The Y voxel a MACHINE'S nozzle is centred on (`machine_fluid_port`,
/// `fluid_tank`). The pipe's own axis is [`PIPE_C`]; this integer survives
/// for the machine generators and the junction tests, which build nozzles
/// with the integer-centred primitives.
pub const PIPE_AXIS_Y: i32 = 16;
const PIPE_OUTER_R: f32 = 4.5;
const PIPE_BORE_R: f32 = 3.0;
/// Bore floor and height in voxels, mirrored by `main.gd`'s
/// PIPE_BORE_FLOOR_VOXELS / PIPE_BORE_HEIGHT_VOXELS — change one, change
/// the other. Unchanged by the shape family: a full column still rises from
/// z = 12 to the crown.
const PIPE_BORE_Z0: i32 = PIPE_AXIS_Z - 3;
const PIPE_BORE_HEIGHT: i32 = 7;
/// The bore's widest row (at axis height) — what a sight window is cut to.
const PIPE_BORE_Y: (i32, i32) = (13, 18);
/// The fluid body's width, **narrower than the bore**. The old body was as
/// wide as the bore and its side faces coincided with the tube's OUTER wall
/// where the wall curves in toward the crown, so a coloured stripe
/// z-fought its way through the closed pipe along the top of every tile —
/// the single most visible pipe bug in a rendered factory. Two voxels
/// narrower on each side keeps every face of a full column strictly inside
/// wall material except where the window has genuinely been cut away.
const PIPE_FLUID_Y: (i32, i32) = (14, 17);
/// The sight window on a straight tile and an end, in X: an in-line sight
/// glass, short so most of the tile is closed tube (see `pipe_body`).
const PIPE_WINDOW: (i32, i32) = (11, 20);
/// The sight window on a corner, a tee and a cross: on the +X leg, between
/// the hub and where a fitting may stand. Every junction shape has a +X
/// leg, so the window is in the same place on all three.
const PIPE_LEG_WINDOW: (i32, i32) = (20, 26);
/// Fittings keep every voxel at tube height east of this X, so a fitting
/// overlaid on a shape's +X leg never fills that leg's window
/// ([`PIPE_LEG_WINDOW`] ends at 26). Hardware standing ABOVE the tube (the
/// tank valve's hand wheel) may reach two voxels further west: a sight line
/// out of a window rises one voxel per voxel of travel, and the wheel is
/// too high to be in its way.
const PIPE_FITTING_X0: i32 = 27;
const PIPE_FITTING_TOP_X0: i32 = 25;
/// How much larger in radius every fitting part is than the tube or flange
/// it may be overlaid on — see `pipe_fitting_machine`'s doc.
const PIPE_FITTING_MARGIN: f32 = 1.0;
/// The cast body where two legs meet — one radius up from the tube, so a
/// junction has the swelling a real elbow or tee casting has and the eye
/// reads "fitting" before it reads which legs.
const PIPE_HUB_R: f32 = PIPE_OUTER_R + 1.0;

/// The support stand every one-tile shape stands on: one per tile, centred,
/// footplate + post + saddle. `steel`, NOT `steel_dark`: at scene scale the
/// fixed isometric camera sees these posts edge-on and a dark metal facing
/// away from the key light renders very nearly black, so dark stands were
/// visually heavier than the pipe they carried.
fn pipe_stand(m: &mut VoxModel, p: &Palette) {
    m.cuboid((13, 18), (13, 18), (0, 0), p.idx("steel"));
    m.cuboid((14, 17), (15, 16), (1, 8), p.idx("steel"));
    m.cuboid((13, 18), (12, 19), (9, 10), p.idx("steel"));
}

/// A leg of tube along +X from `x.0` to `x.1`, solid. The bore is carved
/// afterwards by the shape, once, so legs and hub hollow out together.
///
/// `steel_light` — the BRIGHTEST metal in the palette. A pipe is a long thin
/// horizontal object seen side-on, so almost all of it is curved wall
/// facing away from the key light, and in `steel` it simply disappeared
/// between the belt beside it and the floor under it. Making the *tube*
/// the bright, continuous thing is what lets the eye follow the run.
fn pipe_leg_x(m: &mut VoxModel, p: &Palette, x: (i32, i32)) {
    m.cylinder_x_f((PIPE_C, PIPE_AXIS_Z as f32), PIPE_OUTER_R, x, p.idx("steel_light"));
}

/// The +Y twin of [`pipe_leg_x`].
fn pipe_leg_y(m: &mut VoxModel, p: &Palette, y: (i32, i32)) {
    m.cylinder_y_f((PIPE_C, PIPE_AXIS_Z as f32), PIPE_OUTER_R, y, p.idx("steel_light"));
}

/// Cuts the sight window through the top of a +X leg over `x`: the bore's
/// full width, from just above the bore FLOOR to the crown, leaving the
/// side rails standing. Starting at the floor rather than at the crown is
/// deliberate: a level below half fill is exactly the state a player most
/// needs to read (a draining pipe means a stalling machine).
fn pipe_window_x(m: &mut VoxModel, x: (i32, i32)) {
    let top = m.size[2] as i32 - 1;
    m.carve_cuboid(x, PIPE_BORE_Y, (PIPE_BORE_Z0 + 1, top));
}

/// The bore of a +X leg, then whatever else the shape carves.
fn pipe_bore_x(m: &mut VoxModel, x: (i32, i32)) {
    m.carve_cylinder_x_f((PIPE_C, PIPE_AXIS_Z as f32), PIPE_BORE_R, x);
}

fn pipe_bore_y(m: &mut VoxModel, y: (i32, i32)) {
    m.carve_cylinder_y_f((PIPE_C, PIPE_AXIS_Z as f32), PIPE_BORE_R, y);
}

/// One bolted flange at the low-X end only — so a join between two straight
/// tiles reads as a single fitting instead of two stacked ones, and so there
/// is exactly ONE ring per tile. An earlier cut put collars either side of
/// the window as well; rendered in the factory that put three pale rings on
/// every tile and the run read as a row of bobbins. Junction shapes carry
/// no end flange at all: their hub is their identity, and a flange on a leg
/// would stack against a neighbour's half the time.
fn pipe_joint_flange(m: &mut VoxModel, p: &Palette) {
    m.cylinder_x_f((PIPE_C, PIPE_AXIS_Z as f32), PIPE_OUTER_R + 1.0, (0, 1), p.idx("steel"));
}

/// A plain length of pipe: one tile of the run, open on both X faces.
///
/// The redesign history worth keeping (the "Visual overhaul round 2" epic's
/// ticket 2, from a full-scene render): ring clutter fixed by one flange
/// per tile; a full-length open trough replaced by a short framed window
/// so the run has a continuous round back to read along; one centred stand
/// instead of two heavy legs. This ticket moved the axis onto the tile
/// midline (section header) and narrowed the fluid body (`PIPE_FLUID_Y`);
/// the profile is rounder for it — a midline axis gives the tube a two-
/// voxel flat at the equator instead of three nine-wide rows, which is the
/// difference between a pipe and a square beam at scene scale.
pub fn pipe(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    let last = TILE_VOXELS as i32 - 1;
    pipe_stand(&mut m, p);
    pipe_leg_x(&mut m, p, (0, last));
    pipe_joint_flange(&mut m, p);
    pipe_bore_x(&mut m, (0, last));
    pipe_window_x(&mut m, PIPE_WINDOW);
    m
}

/// A run's END: open on -X only, the +X face closed by a blind flange.
///
/// A plain `pipe` bores every tile through, so the last tile of a run used
/// to be an open, unterminated tube mouth with the floor visible through it
/// — camera-facing, and one of the most obviously wrong things in the
/// factory once a close-up was rendered. The old fix (a cap welded to the
/// machine-branch tile) capped the wrong end whenever the run's end was not
/// that tile. This shape is the cap alone; `main.gd` puts it on any tile
/// with exactly one connection.
///
/// **No joint flange on the open end** (ticket 6). A straight keeps its
/// one flange on the same side of every tile along an axis (west on an
/// east-west run), which is what stops two neighbours stacking flanges on
/// a shared boundary. An end has only one possible turn — its open face
/// is dictated — so an end open to the EAST would carry its low-X flange
/// on the very boundary the straight east of it carries its own, and the
/// two would stack. The cap is this shape's identity; the joint is left to
/// the neighbour.
///
/// **The cap's face is bright steel with brass BOLTS, not a brass disc.**
/// The old cap put a brass disc in the middle of the plate, and the render
/// showed a black hole: the +X face is camera-facing but on the shadow side
/// of the key light, and brass there reads as near-black — a cap that
/// looked exactly like the open mouth it existed to close. Small brass
/// bolt heads on a pale plate keep the "fitting" accent without giving the
/// eye a dark circle to read as a bore.
pub fn pipe_end(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    let last = TILE_VOXELS as i32 - 1;
    pipe_stand(&mut m, p);
    pipe_leg_x(&mut m, p, (0, last));
    pipe_bore_x(&mut m, (0, last - 2));
    pipe_window_x(&mut m, PIPE_WINDOW);
    // Blind flange over the still-solid last two voxels of tube.
    m.cylinder_x_f((PIPE_C, PIPE_AXIS_Z as f32), PIPE_OUTER_R + 1.0, (last - 1, last), p.idx("steel_light"));
    for (y, z) in [(15, PIPE_AXIS_Z + 4), (16, PIPE_AXIS_Z + 4), (15, PIPE_AXIS_Z - 4), (16, PIPE_AXIS_Z - 4), (11, PIPE_AXIS_Z), (20, PIPE_AXIS_Z)] {
        m.set(last, y, z, p.idx("brass"));
    }
    m
}

/// The junction shapes' shared centre: the cast hub, hollowed with the
/// legs. Each shape draws its legs solid first, calls this, then carves its
/// leg bores — the hub's cavity joins them.
fn pipe_hub(m: &mut VoxModel, p: &Palette) {
    m.sphere_f((PIPE_C, PIPE_C, PIPE_AXIS_Z as f32), PIPE_HUB_R, p.idx("steel_light"));
}

fn pipe_hub_bore(m: &mut VoxModel) {
    m.carve_sphere_f((PIPE_C, PIPE_C, PIPE_AXIS_Z as f32), PIPE_BORE_R);
}

/// An elbow: open on +X (east) and y = 0 (north). See the section header
/// for the face convention, and why both openings are on camera-visible
/// faces. The window sits on the +X leg ([`PIPE_LEG_WINDOW`]); its top is
/// visible from above at every rotation, and a sight line leaving it on
/// the camera's diagonal clears the hub even when this leg points away.
pub fn pipe_corner(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    let last = TILE_VOXELS as i32 - 1;
    pipe_stand(&mut m, p);
    pipe_leg_x(&mut m, p, (13, last));
    pipe_leg_y(&mut m, p, (0, 18));
    pipe_hub(&mut m, p);
    pipe_bore_x(&mut m, (13, last));
    pipe_bore_y(&mut m, (0, 18));
    pipe_hub_bore(&mut m);
    pipe_window_x(&mut m, PIPE_LEG_WINDOW);
    m
}

/// A tee: open on +X, -X and y = 0 (north) — a straight run with a branch
/// toward the camera.
pub fn pipe_tee(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    let last = TILE_VOXELS as i32 - 1;
    pipe_stand(&mut m, p);
    pipe_leg_x(&mut m, p, (0, last));
    pipe_leg_y(&mut m, p, (0, 18));
    pipe_hub(&mut m, p);
    pipe_bore_x(&mut m, (0, last));
    pipe_bore_y(&mut m, (0, 18));
    pipe_hub_bore(&mut m);
    pipe_window_x(&mut m, PIPE_LEG_WINDOW);
    m
}

/// A cross: open on all four faces.
pub fn pipe_cross(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    let last = TILE_VOXELS as i32 - 1;
    pipe_stand(&mut m, p);
    pipe_leg_x(&mut m, p, (0, last));
    pipe_leg_y(&mut m, p, (0, last));
    pipe_hub(&mut m, p);
    pipe_bore_x(&mut m, (0, last));
    pipe_bore_y(&mut m, (0, last));
    pipe_hub_bore(&mut m);
    pipe_window_x(&mut m, PIPE_LEG_WINDOW);
    m
}

// --- Fittings --------------------------------------------------------------
//
// A fitting is the pipe's half of a joint with something that is not a
// pipe. Both are authored on the +X face, occupy only the last few voxels
// of a leg (`PIPE_FITTING_X0`), and are hollow at the bore so the liquid
// runs to the tile boundary. Godot overlays one as its own node, turned to
// the connected face — never merged into the shape — so a straight, a
// corner and a tee all take the same fitting on any open leg.
//
// Their bore is carved a little wider than the shape's (`+ 0.4`) on
// purpose: an overlay's inner faces then sit *inside* the shape's wall
// material rather than coinciding with its bore surface, so the two meshes
// never z-fight where the bore is visible.

/// The joint with a machine's `machine_fluid_port`: a bright collar and a
/// heavy brass flange right on the tile boundary. The machine's own nozzle
/// carries a brass flange at radius 4 on its side of the boundary; a plain
/// tube end (radius 4.5) butting that showed its own flat cut face around
/// the smaller ring, which is the "fittings meet machines badly" of the
/// ticket. A flange one radius larger than the tube, in the same brass,
/// swallows the machine's ring and the joint reads as one bolted spool.
///
/// **Every part of a fitting is at least one voxel of radius larger than
/// whatever it can be overlaid on** (`PIPE_FITTING_MARGIN`, ticket 6). A
/// fitting lands on a straight's LOW-X face too — the first tile of the
/// run leaving a machine — where it covers the straight's own joint
/// flange (radius `PIPE_OUTER_R + 1`). Two voxel solids z-fight wherever a
/// voxel of the inner one is a boundary voxel of both, and a full voxel of
/// radius margin is what guarantees every axis-neighbour of an inner voxel
/// is inside the outer one, so no face is ever shared. The collar clears
/// the tube by that margin and the flange clears the joint flange by it.
pub fn pipe_fitting_machine(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 24]);
    let last = TILE_VOXELS as i32 - 1;
    let axis = (PIPE_C, PIPE_AXIS_Z as f32);
    m.cylinder_x_f(axis, PIPE_OUTER_R + PIPE_FITTING_MARGIN, (PIPE_FITTING_X0, last - 2), p.idx("steel_light"));
    m.cylinder_x_f(axis, PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN, (last - 1, last), p.idx("brass"));
    m.carve_cylinder_x_f(axis, PIPE_BORE_R + 0.4, (PIPE_FITTING_X0, last));
    m
}

/// The joint with a `fluid_tank`'s discharge spout: the heaviest flange on
/// the line, with an isolation valve and a brass hand wheel standing on it.
///
/// The valve is what marks a tank connection at a glance (UI_UX.md §7 —
/// silhouette, not colour), and it stands ON the flange rather than
/// mid-tile because a fitting must fit on a junction shape's short leg too.
/// The old one-piece supply tile moved its valve to mid-tile so it would
/// not stand between the camera and the joint; here the valve IS the joint
/// hardware
/// — a flanged valve bolted to a tank outlet is how a real tank is plumbed
/// — and the flange's camera-facing side and the spout stay in view below
/// the bonnet. The bonnet is kept narrow in Y (`13..18`) so a sight line
/// leaving a leg window on the camera's diagonal passes it.
pub fn pipe_fitting_tank(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 32]);
    let last = TILE_VOXELS as i32 - 1;
    let axis = (PIPE_C, PIPE_AXIS_Z as f32);
    // Radii: the brass band clears the tube by the margin; the flange
    // clears a straight's joint flange (`PIPE_OUTER_R + 1`) by it too, for
    // the reason `pipe_fitting_machine` gives.
    m.cylinder_x_f(axis, PIPE_OUTER_R + 0.2 + PIPE_FITTING_MARGIN, (PIPE_FITTING_X0, PIPE_FITTING_X0 + 1), p.idx("brass"));
    m.cylinder_x_f(axis, PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN, (PIPE_FITTING_X0 + 2, last), p.idx("steel_light"));
    m.carve_cylinder_x_f(axis, PIPE_BORE_R + 0.4, (PIPE_FITTING_X0, last));
    // Valve: bonnet on the flange, cap, stem, hand wheel. The bonnet is
    // `steel`, not `steel_dark`: it stands in the tank's own shadow, and
    // the first render showed a dark bonnet as a black block against the
    // tank's dark base (the same lesson as the pipe stands).
    m.cuboid((PIPE_FITTING_X0, last), (13, 18), (PIPE_AXIS_Z + 1, PIPE_AXIS_Z + 8), p.idx("steel"));
    m.cuboid((PIPE_FITTING_X0 + 1, last - 1), (14, 17), (PIPE_AXIS_Z + 9, PIPE_AXIS_Z + 10), p.idx("steel_light"));
    m.cuboid((29, 29), (15, 16), (PIPE_AXIS_Z + 11, PIPE_AXIS_Z + 12), p.idx("steel_dark"));
    m.cylinder_z_f((28.5, PIPE_C), 3.5, (PIPE_AXIS_Z + 13, PIPE_AXIS_Z + 13), p.idx("brass"));
    m.carve_cylinder_z_f((28.5, PIPE_C), 1.6, (PIPE_AXIS_Z + 13, PIPE_AXIS_Z + 13));
    m.cuboid((28, 29), (15, 16), (PIPE_AXIS_Z + 13, PIPE_AXIS_Z + 13), p.idx("brass"));
    m
}

// --- Fluid bodies ----------------------------------------------------------
//
// One per shape, each **one voxel tall**: `main.gd` scales the node
// vertically to the segment's live fill fraction, and a one-voxel slab
// scaled to N voxels is geometrically the same box as an N-voxel column,
// with none of the pivot arithmetic a taller model would need (the encoder
// puts a model's origin at its footprint centre at ground level). Near-
// white on purpose: the per-fluid colour is live simulation data, applied
// in Godot as an albedo tint, so one body per shape serves every fluid
// `data/recipes.ron` ever names. Each fills its shape's bore footprint out
// to every OPEN face and stops short of a cap, so Godot never trims one.

const _: () = assert!(
    PIPE_BORE_Z0 + PIPE_BORE_HEIGHT <= PIPE_AXIS_Z + PIPE_OUTER_R as i32,
    "a full fluid column would rise above the pipe wall"
);

fn pipe_fluid_slab() -> VoxModel {
    VoxModel::new([TILE_VOXELS, TILE_VOXELS, 1])
}

/// A +X leg of liquid over `x`.
fn pipe_fluid_x(m: &mut VoxModel, p: &Palette, x: (i32, i32)) {
    m.cuboid(x, PIPE_FLUID_Y, (0, 0), p.idx("fluid_pale"));
}

fn pipe_fluid_y(m: &mut VoxModel, p: &Palette, y: (i32, i32)) {
    m.cuboid(PIPE_FLUID_Y, y, (0, 0), p.idx("fluid_pale"));
}

/// The liquid in a straight [`pipe`].
pub fn pipe_fluid(p: &Palette) -> VoxModel {
    let mut m = pipe_fluid_slab();
    pipe_fluid_x(&mut m, p, (0, TILE_VOXELS as i32 - 1));
    m
}

/// The liquid in a [`pipe_corner`].
pub fn pipe_fluid_corner(p: &Palette) -> VoxModel {
    let mut m = pipe_fluid_slab();
    pipe_fluid_x(&mut m, p, (PIPE_FLUID_Y.0, TILE_VOXELS as i32 - 1));
    pipe_fluid_y(&mut m, p, (0, PIPE_FLUID_Y.1));
    m
}

/// The liquid in a [`pipe_tee`].
pub fn pipe_fluid_tee(p: &Palette) -> VoxModel {
    let mut m = pipe_fluid_slab();
    pipe_fluid_x(&mut m, p, (0, TILE_VOXELS as i32 - 1));
    pipe_fluid_y(&mut m, p, (0, PIPE_FLUID_Y.1));
    m
}

/// The liquid in a [`pipe_cross`].
pub fn pipe_fluid_cross(p: &Palette) -> VoxModel {
    let mut m = pipe_fluid_slab();
    pipe_fluid_x(&mut m, p, (0, TILE_VOXELS as i32 - 1));
    pipe_fluid_y(&mut m, p, (0, TILE_VOXELS as i32 - 1));
    m
}

/// The liquid in a [`pipe_end`]: stops where the blind flange starts.
pub fn pipe_fluid_end(p: &Palette) -> VoxModel {
    let mut m = pipe_fluid_slab();
    pipe_fluid_x(&mut m, p, (0, TILE_VOXELS as i32 - 3));
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
    machine_footing_rect(m, p, span, span, skirt_top, paint);
}

/// [`machine_footing`] for a footprint that is not square — the mixer, which
/// is `data/machines.ron`'s only oblong kind (1x2) since the chain rework
/// gave it a second solid ingredient and therefore a second inlet tile.
/// `machine_footing` is this with both spans equal, so the square kinds read
/// exactly as they did.
fn machine_footing_rect(
    m: &mut VoxModel,
    p: &Palette,
    x_span: (i32, i32),
    y_span: (i32, i32),
    skirt_top: i32,
    paint: &str,
) {
    let (xlo, xhi) = x_span;
    let (ylo, yhi) = y_span;
    m.cuboid((xlo, xhi), (ylo, yhi), (0, 1), p.idx("concrete_dark"));
    // Hazard skirt around all four edges of the plinth top (UI_UX.md §3).
    m.hazard_stripe_x((xlo, xhi), (ylo, ylo), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    m.hazard_stripe_x((xlo, xhi), (yhi, yhi), (2, 2), 3, p.idx("hazard_yellow"), p.idx("hazard_black"));
    for y in ylo..=yhi {
        let a = if (y + 2).rem_euclid(6) < 3 { p.idx("hazard_yellow") } else { p.idx("hazard_black") };
        m.set(xlo, y, 2, a);
        m.set(xhi, y, 2, a);
    }

    let (bxlo, bxhi) = (xlo + 2, xhi - 2);
    let (bylo, byhi) = (ylo + 2, yhi - 2);
    m.cuboid((bxlo, bxhi), (bylo, byhi), (2, skirt_top), p.idx(paint));
    for (x0, x1) in [(bxlo, bxlo + 1), (bxhi - 1, bxhi)] {
        for (y0, y1) in [(bylo, bylo + 1), (byhi - 1, byhi)] {
            m.cuboid((x0, x1), (y0, y1), (2, skirt_top + 1), p.idx("steel_dark"));
        }
    }
    // Machined rim the vessel/throat sits on.
    m.cuboid((bxlo, bxhi), (bylo, byhi), (skirt_top + 1, skirt_top + 2), p.idx("steel"));
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
// The belt's own half of each junction is the plain tile (see the conveyor
// section above — the `belt_inlet`/`belt_outlet` dressings were retired by
// the pipes & conveyors overhaul), so everything that makes a junction read
// as a junction is on the machine's side of the boundary, built against
// `BELT_LANE` and `BELT_SURFACE_Z` below.

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
    // A brass band across the head of the opening — the same accent the
    // pipe fittings' flanges and the tank valve's hand wheel carry, and
    // for the same reason: it marks a fitting product actually passes
    // through. A hazard stripe was tried here first and rendered as a dark
    // bar, because half a stripe is black and this edge is the only part of
    // the piece the camera looks straight at.
    m.cuboid((last - 2, last), (l0 - 1, l1 + 1), (head - 1, head - 1), p.idx("brass"));
}

/// The **fluid** half of a junction: a nozzle standing off one of the
/// machine's X faces at the pipe run's own axis height, so a pipe tile
/// butting against it reads as plumbed into the machine rather than parked
/// beside it.
///
/// This is the chain rework's junction piece, and it exists because that
/// change gave three machines a face that a belt no longer meets. The mixer
/// discharges liquid east, the cooking vat is fed and drained entirely by
/// pipe (no belt touches it at all), and the bottler is where the liquid
/// becomes a bottle again — so where those faces used to carry
/// [`machine_infeed`]'s hood or [`machine_outfeed`]'s doorway, a belt-shaped
/// opening would now be a mouth with no conveyor in front of it.
///
/// **It is deliberately the same fitting on both sides, and the same one
/// `pipe_fitting_machine` answers it with**: bright steel at radius 3 with
/// a brass flange sitting exactly on the tile boundary. That pairing is
/// what the "Visual overhaul round 2" epic's ticket 3 found actually reads
/// as one spool piece crossing a tile boundary rather than as two stubs a
/// voxel apart. The pipe's half of the joint is `pipe_fitting_machine`,
/// which Godot overlays on whichever face of the tapping tile touches the
/// machine (`links` says which); a plain `pipe` tile's bore is open at
/// both of its X faces at exactly `PIPE_AXIS_Z`, so nothing more is needed
/// on the pipe's side of these
/// junctions: the machine's nozzle meets an open bore.
///
/// `lane_y` is the nozzle's centre in this model's own Y coordinates — for a
/// 1x1 machine, `PIPE_AXIS_Y`; for the 1x2 mixer, the middle of whichever
/// half the pipe's row maps to. `face` is `Face::LowX` for an inlet on the
/// -X edge or `Face::HighX` for a discharge on +X.
///
/// Unlike a belt junction there is no camera asymmetry to design around: a
/// nozzle is a solid protruding object, not an opening, so it reads from the
/// +X side dead-on and from the -X side by the silhouette it breaks out of
/// the machine's own skirt. That is the whole reason the fluid junctions
/// need one piece where the belt junctions needed two.
enum Face {
    LowX,
    HighX,
}

fn machine_fluid_port(m: &mut VoxModel, p: &Palette, lane_y: i32, face: Face, last_x: i32) {
    // How far the nozzle reaches back INTO the machine: far enough to
    // genuinely overlap whatever shell it comes out of rather than merely
    // touching it, which is the same rule the pipe fittings follow.
    const REACH: i32 = 10;
    let (span, flange) = match face {
        Face::LowX => ((0, REACH), (0, 1)),
        Face::HighX => ((last_x - REACH, last_x), (last_x - 1, last_x)),
    };
    m.cylinder_x((lane_y, PIPE_AXIS_Z), 3.0, span, p.idx("steel_light"));
    m.cylinder_x((lane_y, PIPE_AXIS_Z), 4.0, flange, p.idx("brass"));
    // A gusset under the nozzle, so it visibly lands on the machine instead
    // of floating off its side — the mixer's original vinegar stub needed
    // exactly this and it is the same shape here.
    let gusset = match face {
        Face::LowX => (2, REACH - 2),
        Face::HighX => (last_x - REACH + 2, last_x - 2),
    };
    m.cuboid(gusset, (lane_y - 1, lane_y + 1), (9, PIPE_AXIS_Z - 3), p.idx("steel_dark"));
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

/// `data/machines.ron`'s **1x2** `mixer` — a horizontal ribbon blender.
///
/// **Redesigned, and the footprint is why.** The chain rework gave the
/// mixer's recipe a second solid ingredient (sugar beside tomato paste), and
/// `data/machines.ron`'s own header states the rule that follows: a machine
/// accepts solid input on every tile of the edge opposite the way it points,
/// so a machine consuming N solid ingredients needs an inlet face at least N
/// tiles long or the two lanes merge onto one belt and deadlock. The mixer
/// is therefore 1 tile wide and 2 tiles deep, and this model is 32x64
/// voxels — UI_UX.md §2's 32-per-tile scale times the footprint the data
/// file declares, the same coupling `machine_cooking_vat` documents. A
/// machine's size is content; change the footprint and the model changes
/// with it.
///
/// **A horizontal drum lying along +Y, not the old upright vessel.** The
/// obvious move was to keep the 1x1 drum and stretch the plinth under it,
/// and it is wrong twice over: a round vessel centred in a 1x2 footprint
/// leaves half the machine as bare deck, and — worse — it puts solid shell
/// across exactly the two -X bands where the two feed chutes have to come
/// in. A ribbon blender is what a real plant uses to fold a dry ingredient
/// into a wet one, its natural shape is a long horizontal trough, and that
/// shape is *made of* the extra tile rather than merely tolerating it. It
/// also reads unmistakably as two tiles deep at scene scale, which an
/// upright vessel on a long plinth does not.
///
/// The agitator stays visible, which was the whole point of the previous
/// design: the drum's crown is cut away over a long sight window, so the
/// ribbon screw inside is seen from above exactly as the crusher's rolls
/// are. That is `pipe_body`'s own sight-window trick, for its reason — a
/// fully open trough stops reading as a vessel.
///
/// **Three junctions, and each is on the face the simulation actually uses.**
/// Two belt infeeds on -X, one per tile of the inlet face (the model's two
/// local-Y halves — the vox importer maps low local Y to high world Z, so
/// the ORIGIN row, sim-y = 0, is the *upper* half; see
/// `machine_cooking_vat`). One fluid discharge nozzle on +X at the origin
/// row, because a fluid output leaves by the face the machine points at
/// (`sim_core/src/sim/world.rs`'s module doc) and `sim::reference_chain`
/// lays that run east of the origin tile. And the vinegar inlet stub still
/// points -Y, i.e. north, off the sim-y = 1 half — which is where the
/// reference chain's tank run now sits, one row north of the whole
/// footprint.
///
/// There is deliberately **no belt outfeed**: this machine's only output is
/// a liquid now, so a discharge doorway would be a chute product never comes
/// out of.
pub fn machine_mixer(p: &Palette) -> VoxModel {
    let tile = TILE_VOXELS as i32;
    let last_x = tile - 1;
    let last_y = tile * 2 - 1;
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS * 2, 30]);
    machine_footing_rect(&mut m, p, (0, last_x), (0, last_y), 8, "paint_blue");

    // The two lanes, in this model's own Y. `BELT_LANE` is the sim-y = 1
    // (low local Y) half; the origin row is a tile further up.
    let near_lane = BELT_LANE;
    let origin_lane = (BELT_LANE.0 + tile, BELT_LANE.1 + tile);
    // Centre of the origin row, which is where the discharge nozzle and the
    // pipe tile east of it meet.
    let origin_mid = tile + PIPE_AXIS_Y;

    // The drum: a horizontal cylinder running the machine's whole length,
    // its axis at x = 17 so its west wall stands clear of the two feed
    // housings (which reach x = 8) and its east wall reaches far enough to
    // carry the discharge nozzle.
    const DRUM_X: i32 = 17;
    const DRUM_Z: i32 = 18;
    m.cylinder_y((DRUM_X, DRUM_Z), 11.0, (5, last_y - 5), p.idx("steel"));
    // Bands, the same seam language every other vessel on the line carries.
    for y in [9, 26, 37, 54] {
        m.cylinder_y((DRUM_X, DRUM_Z), 11.6, (y, y), p.idx("steel_light"));
    }
    // Dished ends, so the drum is a vessel rather than a length of duct.
    for (y, r) in [(4, 9.0f32), (3, 6.5), (last_y - 4, 9.0), (last_y - 3, 6.5)] {
        m.cylinder_y((DRUM_X, DRUM_Z), r, (y, y), p.idx("steel_light"));
    }

    // Hollow it, then cut the sight window through the crown — `pipe_body`'s
    // trick, for `pipe_body`'s reason: an open trough reads as a gutter, a
    // framed window reads as a vessel you can see into.
    m.carve_cylinder_y((DRUM_X, DRUM_Z), 9.0, (5, last_y - 5));
    // A SHORT window, not a long one. The first cut ran it two thirds of the
    // drum's length and the rendered factory showed the ribbon flights along
    // almost the whole vessel — which at scene scale reads as corrugation, a
    // ribbed duct, rather than as a closed drum you can see into. Half a
    // tile is enough to show the agitator turning and leaves the drum's own
    // round back to read along, exactly the trade `pipe_body` records about
    // the pipe run's sight glass.
    m.carve_cuboid((DRUM_X - 5, DRUM_X + 5), (24, last_y - 24), (DRUM_Z + 1, 29));

    // Ribbon agitator: a shaft down the drum's axis with a helical ribbon
    // wound round it, drawn as a run of short chords stepping in angle. This
    // is the part the window exists to show, so it sits high in the drum
    // where the sight line reaches it.
    m.cylinder_y((DRUM_X, DRUM_Z), 1.8, (4, last_y - 4), p.idx("steel_light"));
    for y in 8..=(last_y - 8) {
        // One full turn every 16 voxels; the ribbon is two opposed flights.
        let phase = (y as f32) * std::f32::consts::TAU / 16.0;
        for half in [0.0f32, std::f32::consts::PI] {
            let a = phase + half;
            let dx = (a.cos() * 7.0).round() as i32;
            let dz = (a.sin() * 7.0).round() as i32;
            m.cuboid(
                (DRUM_X + dx - 1, DRUM_X + dx + 1),
                (y, y),
                (DRUM_Z + dz - 1, DRUM_Z + dz + 1),
                p.idx("steel_dark"),
            );
        }
    }

    // Drive end: a motor and gearbox on the far (+Y) end of the shaft.
    // Far, not near, and that is the camera again: `control_panel`'s own doc
    // records that low local Y is the face this camera looks at, so a drive
    // that bulky on the near end would stand between the viewer and the
    // sight window this machine is identified by.
    m.cuboid((11, 23), (last_y - 7, last_y - 1), (10, 22), p.idx("paint_blue"));
    m.cuboid((11, 23), (last_y - 7, last_y - 1), (23, 23), p.idx("steel_dark"));
    m.cylinder_y((DRUM_X, DRUM_Z), 4.0, (last_y - 8, last_y - 6), p.idx("brass"));

    // Vinegar inlet: the stub reaching north off the sim-y = 1 half, toward
    // the pipe run the reference chain lays one row north of the footprint.
    // Height is PIPE_AXIS_Z exactly, matching `pipe_fitting_machine` — the
    // two are one spool piece split across a tile boundary, and the pair
    // only reads as a connection because they are the same diameter, the
    // same bright steel and carry their brass flange at the same boundary.
    m.cylinder_y((DRUM_X - 1, PIPE_AXIS_Z), 3.0, (0, 9), p.idx("steel_light"));
    m.cylinder_y((DRUM_X - 1, PIPE_AXIS_Z), 4.0, (0, 1), p.idx("brass"));
    m.cuboid((DRUM_X - 2, DRUM_X), (2, 7), (9, PIPE_AXIS_Z - 3), p.idx("steel_dark"));

    // The two feed chutes: a hooded transfer housing per inlet tile, then a
    // short throat from each up into the drum's crown, so what arrives on
    // either belt visibly goes somewhere.
    machine_infeed(&mut m, p, near_lane);
    machine_infeed(&mut m, p, origin_lane);
    for lane in [near_lane, origin_lane] {
        let mid = (lane.0 + lane.1) / 2;
        m.cuboid((6, 10), (mid - 3, mid + 3), (BELT_SURFACE_Z, 24), p.idx("steel_light"));
        m.cuboid((10, DRUM_X - 6), (mid - 2, mid + 2), (22, 24), p.idx("steel_light"));
    }

    // The liquid discharge, on the outlet face at the origin row.
    machine_fluid_port(&mut m, p, origin_mid, Face::HighX, last_x);

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
///
/// **Since the chain rework it is the one machine on the line with no belt
/// junction at all**: liquid mix arrives by pipe and finished ketchup leaves
/// by pipe, so both of its X faces carry [`machine_fluid_port`] nozzles
/// where they used to carry a charging gallery and a discharge doorway.
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

    // The junction hardware — and since the chain rework, **no belt touches
    // this machine at all**.
    //
    // Its recipe takes a fluid and makes a fluid (`data/recipes.ron`), so
    // the charging gallery this face used to carry is gone: a covered
    // belt-width hood with no conveyor in front of it is a mouth product
    // never arrives at, which is the same class of lie as a gauge reading a
    // level the mechanism does not have. What replaces it is a pipe nozzle
    // on each X face at the run's own axis height.
    //
    // Both sit on the vat's **sim-y = 0 row**, which is the model's UPPER
    // local-Y half (the importer maps low local Y to high world Z — the same
    // derivation `machine_mixer` uses), because that is the row
    // `sim::reference_chain` lays both runs along: the mix arrives from the
    // mixer to the west and the cooked ketchup leaves east to the bottler.
    let pipe_lane_mid = TILE_VOXELS as i32 + PIPE_AXIS_Y;
    let last = TILE_VOXELS as i32 * 2 - 1;
    machine_fluid_port(&mut m, p, pipe_lane_mid, Face::LowX, last);
    machine_fluid_port(&mut m, p, pipe_lane_mid, Face::HighX, last);

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

    // The chain rework made this the stage where the product stops being a
    // fluid: liquid ketchup is drawn from the pipe run touching this machine
    // and bottles leave on the belt off its outlet face. So the -X face
    // carries a pipe nozzle where it used to carry a belt hood, and only the
    // +X face is still a conveyor junction. It is the only machine in the
    // factory with one of each, which is exactly what "the piped middle of
    // the chain rejoins the conveyor here" looks like.
    machine_fluid_port(&mut m, p, PIPE_AXIS_Y, Face::LowX, TILE_VOXELS as i32 - 1);
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
    // function's doc), reaching the tile edge so it meets
    // `pipe_fitting_tank`'s flange rather than stopping short of it.
    // Bright, and thick enough to see: ticket 3 found this spout rendering
    // as dark grey on the tank's own dark shell, so the one piece of
    // hardware that says "this vessel feeds the run" was invisible at scene
    // scale. It now matches `pipe_fitting_tank`'s brass band in metal, and
    // the two meet on the tile boundary.
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
    const {
        assert!(
            TANK_FILL_Z0 + TANK_FILL_HEIGHT <= 29,
            "a full tank's level would rise into its domed head"
        )
    };
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
    const {
        assert!(
            HOPPER_FILL_Z0 + HOPPER_FILL_HEIGHT <= 30,
            "a full hopper's level would rise above its loading lip"
        )
    };
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

/// `data/machines.ron`'s **2x2** `dock` — the chain's exit, where finished
/// cases leave the factory and become money (`sim_core::sim::dock`).
///
/// **Why a raised platform with a despatch door, rather than another housing
/// with a mechanism on top.** Every processing kind on this line is
/// identified by the machinery that does its work — flywheels, an agitator,
/// a filling gantry, a taping head. A dock does no work at all: it is
/// a *place*, the one building in the world that is not equipment. So it is
/// drawn as architecture — a poured platform at trailer-bed height, its edge
/// striped, with despatch openings and a canopy behind them — which is both
/// what a real loading bay looks like and the one silhouette on this line
/// that could not be mistaken for a machine.
///
/// **It is 64x64 because its footprint is 2x2** (UI_UX.md §2's tile scale
/// times the declared footprint, the coupling `machine_cooking_vat` and
/// `machine_mixer` already document). The 2026-09-09 tuning pass grew it from
/// one tile after a playtest said the one-tile version did not read as a
/// loading dock, and the extra tile is spent on DEPTH rather than on making
/// the old design bigger — see `data/machines.ron`'s dock note for the
/// footprint decision. Concretely, the second row is what finally lets the
/// two halves of a loading bay stop competing for one tile: **the far tile
/// is the building** (despatch wall, two doors, canopy, bay lights) and
/// **the near tile is open apron** (bumpers, striped edge, and the deck the
/// waiting goods actually stand on — `main.gd`'s `_update_dock_stock`).
///
/// **Where each piece goes is decided by the camera, exactly as the junction
/// section above derives it.** The fixed isometric view (UI_UX.md §1) can
/// only ever see faces whose normal points +X or -Y in this model's own
/// coordinates, and — the part that actually drove this layout — a sight
/// line leaving any point travels `(+x, -y, +z)`, so anything at HIGH x and
/// LOW y stands in front of everything behind it. That rules out the obvious
/// design, which was tried first on the one-tile version: a truck bay carved
/// into the +X face. It reads beautifully on its own and it walls off the
/// entire deck behind it, because every deck voxel's sight line has to pass
/// through the last few +X columns on its way out. So the tall structure
/// stands on the FAR (high-y) side, where its -Y face is still square to the
/// camera but it occludes nothing.
///
/// **Two doors, not one wide one.** The width is the other half of what the
/// bigger footprint bought, and a real bay of this size has several
/// despatch positions rather than one enormous shutter; two openings with
/// their own shutters and their own bay lights under one continuous canopy
/// also give the wall a rhythm, where a single 40-voxel hole would read as a
/// missing wall. Both are genuinely EMPTY (see this file's junction tests
/// for why that is checked voxel by voxel rather than eyeballed).
///
/// **Material** (UI_UX.md §2b point 8, "food contact is stainless, structure
/// is painted or poured"): a dock is almost entirely structure — poured
/// concrete deck, painted steel wall and canopy, rubber bumpers, hazard
/// striping — so it is the least stainless building in the factory, and
/// deliberately so. The stainless is exactly where product passes: the
/// infeed transfer housings and their brass thresholds, and the despatch
/// openings' frames. That split is the rule doing its job rather than an
/// exception to it.
pub fn machine_dock(p: &Palette) -> VoxModel {
    let tile = TILE_VOXELS as i32;
    let mut m = VoxModel::new([TILE_VOXELS * 2, TILE_VOXELS * 2, TILE_VOXELS]);
    let last = tile * 2 - 1;
    let deck = DOCK_DECK_Z;

    // --- The platform ------------------------------------------------------
    // A solid poured block, not a machine footing: this is the one building
    // on the line the player is meant to read as floor rather than as
    // equipment. Flush with the footprint's own edges for `machine_footing`'s
    // reason — the last belt tile butts straight against it.
    // Poured mid-grey with a lighter wearing surface and a dark kerb course
    // at the bottom. The first cut of the one-tile version made the whole
    // block `concrete_dark`, which is right for a floor slab's *seam* and
    // wrong for a mass this size: rendered in the scene it read as a black
    // pit with a lit rail round it, not as a platform goods stand on.
    m.cuboid((0, last), (0, last), (0, 1), p.idx("concrete_dark"));
    m.cuboid((0, last), (0, last), (2, deck - 1), p.idx("concrete_light"));
    m.cuboid((1, last - 1), (1, last - 1), (2, deck - 1), p.idx("concrete"));
    m.cuboid((0, last), (0, last), (deck, deck), p.idx("concrete_light"));
    // One expansion joint along each tile boundary, scored into the wearing
    // surface. A 64x64 slab poured in one piece is not a thing, and without
    // the score lines the deck reads as a single blank table at scene scale —
    // the joints are what say "this is concrete, and it is two bays wide".
    m.cuboid((tile - 1, tile), (0, last), (deck, deck), p.idx("concrete"));
    m.cuboid((0, last), (tile - 1, tile), (deck, deck), p.idx("concrete"));

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
    // without standing between the camera and anything. Three of them across
    // a bay this wide rather than the one-tile version's two, spaced so the
    // rhythm reads as a wall of a real dock rather than as two lumps.
    for (y0, y1) in [(6, 11), (29, 34), (52, 57)] {
        m.cuboid((last - 1, last), (y0, y1), (deck - 6, deck - 1), p.idx("rubber_dark"));
    }

    // --- The despatch wall, on the FAR side --------------------------------
    // Painted steel, ribbed, standing at high y so its -Y face is square to
    // the camera and it occludes nothing on the apron in front of it. Its
    // front face is at DOCK_WALL_FACE_Y; everything from there to the model's
    // far edge is building, everything in front of it is deck.
    //
    // It starts at DOCK_WALL_WEST_X rather than at the model's own west edge,
    // and that is not a stylistic inset: the FAR tile's transfer housing
    // stands in that corner (see the junctions below), and a wall drawn
    // through it would be a building with a belt hood carved out of its
    // west end and daylight showing through the hole. Leaving the corner
    // open is also what a real bay looks like — the conveyor comes in at the
    // end of the building, not through the middle of it.
    let wall_face_y = DOCK_WALL_FACE_Y;
    m.cuboid((DOCK_WALL_WEST_X, last), (wall_face_y, last), (deck + 1, 27), p.idx("paint_blue"));
    for x in (DOCK_WALL_WEST_X + 2..=62).step_by(4) {
        m.cuboid((x, x), (wall_face_y, wall_face_y), (deck + 1, 25), p.idx("steel_dark"));
    }

    // The despatch openings, carved through to a black back wall — real
    // holes, not dark rectangles painted on the wall (see this file's
    // junction tests for why that distinction is checked rather than
    // eyeballed). Each opening's floor is what a sight line entering it
    // actually reaches (the infeed housing's doc derives that for the
    // mirrored case), so the floor is black and the frame around it is the
    // palette's brightest metal.
    for (x0, x1) in DOCK_DOOR_SPANS {
        m.cuboid((x0 - 3, x1 + 3), (wall_face_y - 2, wall_face_y), (deck + 1, 23), p.idx("steel_light"));
        m.carve_cuboid((x0, x1), (wall_face_y - 2, last - 1), (deck + 1, 21));
        m.cuboid((x0, x1), (last, last), (deck + 1, 21), p.idx("hazard_black"));
        m.cuboid((x0, x1), (wall_face_y - 2, last - 1), (deck + 1, deck + 1), p.idx("hazard_black"));
        // Brass threshold at the lip: the same accent every other fitting
        // product physically passes through carries.
        m.cuboid((x0, x1), (wall_face_y - 2, wall_face_y - 2), (deck + 1, deck + 1), p.idx("brass"));
        // The roller shutter, wound up above the opening — what says this
        // opening is a door rather than a window.
        m.cuboid((x0 - 1, x1 + 1), (wall_face_y - 3, wall_face_y - 2), (22, 23), p.idx("steel_dark"));
        m.cuboid((x0 - 1, x1 + 1), (wall_face_y - 3, wall_face_y - 3), (22, 22), p.idx("steel_light"));
    }

    // --- Canopy over the bay ----------------------------------------------
    // One continuous canopy over both doors, cantilevered off the wall toward
    // the camera. It stops well short of the deck's near edge so the goods
    // standing there stay in clear air. Integral with the wall's own head
    // rather than a slab hovering over it: the one-tile version's first cut
    // stood it two voxels clear on thin posts and rendered as a white shelf
    // floating in front of a blue wall — at scene scale a gap that small is
    // not read as a gap, it is read as two unrelated objects. Shallow, too,
    // because every voxel it reaches toward the camera is a voxel of deck it
    // shades, and the deck is where the goods stand.
    let canopy_face_y = DOCK_CANOPY_FACE_Y;
    let canopy_x = (DOCK_WALL_WEST_X + 2, 61);
    m.cuboid(canopy_x, (canopy_face_y, wall_face_y), (26, 27), p.idx("steel"));
    m.cuboid(canopy_x, (canopy_face_y, canopy_face_y), (25, 25), p.idx("hazard_yellow"));
    for x in [canopy_x.0, 37, 38, canopy_x.1] {
        m.cuboid((x, x), (canopy_face_y + 1, wall_face_y), (24, 25), p.idx("steel_dark"));
    }

    // A bay light under the canopy's leading edge above each door — hung one
    // voxel proud of it so the canopy itself doesn't shadow it out of the
    // frame. These are the dock's status lamps: `main.gd` swaps them green
    // when stock is waiting to be sold and amber when the deck is clear, the
    // same idle/running pair every machine's lamp uses. Both swap together,
    // which is correct — there is one shared delivery pool, not one per door.
    for (x0, x1) in DOCK_DOOR_SPANS {
        let mid = (x0 + x1) / 2;
        m.cuboid((mid - 3, mid + 3), (canopy_face_y - 2, canopy_face_y - 1), (24, 25), p.idx("steel_light"));
        m.cuboid((mid - 2, mid + 2), (canopy_face_y - 2, canopy_face_y - 2), (23, 24), p.idx("status_amber"));
    }

    // --- The junctions -----------------------------------------------------
    // A belt from the packer runs in on -X, through exactly the transfer
    // housing every other machine on the line is fed through. TWO of them,
    // one per tile of the inlet face: a 2x2 dock accepts on both tiles of its
    // west edge (`data/machines.ron`'s "an inlet face is as long as the
    // footprint" rule), so two lines really can deliver into one dock, and a
    // model with a single housing would be describing a mechanism narrower
    // than the one the simulation has. No outfeed on either: nothing is
    // downstream of the dock — product leaves the world here.
    machine_infeed(&mut m, p, BELT_LANE);
    machine_infeed(&mut m, p, (BELT_LANE.0 + tile, BELT_LANE.1 + tile));
    m
}

/// The x spans of the dock's two despatch openings, in its own voxel
/// coordinates. Shared by the wall carve and the bay lights above them so
/// the lamps cannot drift off the doors they belong to, and read by this
/// file's own tests so those check the doors that exist rather than
/// coordinates typed in twice.
const DOCK_DOOR_SPANS: [(i32, i32); 2] = [(19, 33), (44, 58)];
/// The x the despatch wall starts at. West of it is the corner the far
/// tile's belt transfer housing occupies — see [`machine_dock`].
const DOCK_WALL_WEST_X: i32 = 14;
/// The y of the despatch wall's camera-facing face: everything from here to
/// the model's far edge is building, everything in front of it is open deck.
/// Roughly two thirds of the way back, so the far tile carries the structure
/// and the near tile stays clear (see [`machine_dock`]).
const DOCK_WALL_FACE_Y: i32 = 52;
/// The y of the canopy's leading edge, eight voxels proud of the wall. Every
/// voxel it reaches toward the camera is a voxel of deck it shades.
const DOCK_CANOPY_FACE_Y: i32 = 44;

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

    /// Every kind's junction geometry, so a new machine kind added to
    /// `ASSETS` without the hardware its recipe implies fails here rather
    /// than shipping a wall for product to vanish into.
    ///
    /// **Which faces a kind has is decided by its recipe, not by a habit**,
    /// and the chain rework is what made that distinction real: a solid
    /// input arrives on a belt and needs [`machine_infeed`]'s hood, a solid
    /// output leaves on a belt through [`machine_outfeed`]'s doorway, and a
    /// fluid on either side needs [`machine_fluid_port`]'s nozzle instead.
    /// Three machines changed sides in that ticket — the mixer's output, the
    /// vat's input *and* output, and the bottler's input all became fluids —
    /// so the tests below assert per-face rather than "every machine has a
    /// belt on both ends", which is what they used to say and is no longer
    /// true of half the line.
    struct Junctions {
        name: &'static str,
        model: VoxModel,
        /// Belt lanes on the **-X** face, in this model's own Y — one per
        /// tile of the inlet face that a belt actually feeds. Empty for a
        /// machine fed entirely by pipe.
        belt_inlets: Vec<(i32, i32)>,
        /// The span each infeed housing covers, which is wider than the belt
        /// lane for a machine that needs one (see the vat's history).
        housing_lanes: Vec<(i32, i32)>,
        /// The belt lane on the **+X** face, or `None` for a machine whose
        /// output is a fluid or which has no output at all.
        belt_outlet: Option<(i32, i32)>,
        /// Pipe nozzle centres, in this model's own Y, per face.
        fluid_in: Option<i32>,
        fluid_out: Option<i32>,
        last_x: i32,
    }

    fn machines(p: &Palette) -> Vec<Junctions> {
        let tile = TILE_VOXELS as i32;
        let tile_last = tile - 1;
        let far_lane = (BELT_LANE.0 + tile, BELT_LANE.1 + tile);
        // A plain belt-in / belt-out one-tile machine, which is what the
        // crusher, the packer and the generic housing still are.
        let belt_both = |name, model| Junctions {
            name,
            model,
            belt_inlets: vec![BELT_LANE],
            housing_lanes: vec![BELT_LANE],
            belt_outlet: Some(BELT_LANE),
            fluid_in: None,
            fluid_out: None,
            last_x: tile_last,
        };
        vec![
            belt_both("machine", machine(p)),
            belt_both("machine_crusher", machine_crusher(p)),
            belt_both("machine_packer", machine_packer(p)),
            // Two belts in (one per inlet tile), a pipe out: the shape
            // `data/machines.ron`'s "an inlet face is as long as the
            // footprint" rule produces.
            Junctions {
                name: "machine_mixer",
                model: machine_mixer(p),
                belt_inlets: vec![BELT_LANE, far_lane],
                housing_lanes: vec![BELT_LANE, far_lane],
                belt_outlet: None,
                fluid_in: None,
                fluid_out: Some(tile + PIPE_AXIS_Y),
                last_x: tile_last,
            },
            // Pipe in, pipe out, no belt anywhere: the one machine on the
            // line a conveyor never touches.
            Junctions {
                name: "machine_cooking_vat",
                model: machine_cooking_vat(p),
                belt_inlets: vec![],
                housing_lanes: vec![],
                belt_outlet: None,
                fluid_in: Some(tile + PIPE_AXIS_Y),
                fluid_out: Some(tile + PIPE_AXIS_Y),
                last_x: tile * 2 - 1,
            },
            // Pipe in, belt out — where the piped middle rejoins the line.
            Junctions {
                name: "machine_bottler",
                model: machine_bottler(p),
                belt_inlets: vec![],
                housing_lanes: vec![],
                belt_outlet: Some(BELT_LANE),
                fluid_in: Some(PIPE_AXIS_Y),
                fluid_out: None,
                last_x: tile_last,
            },
            // The end of the line: belts in and nothing out, because
            // product leaves the world here (`sim_core::sim::dock`). TWO
            // inlets since the dock became 2x2 — its west edge is two tiles
            // long, so both take a belt.
            Junctions {
                name: "machine_dock",
                model: machine_dock(p),
                belt_inlets: vec![BELT_LANE, far_lane],
                housing_lanes: vec![BELT_LANE, far_lane],
                belt_outlet: None,
                fluid_in: None,
                fluid_out: None,
                last_x: tile * 2 - 1,
            },
        ]
    }

    /// The splitter's own geometry claims — it is deliberately NOT in
    /// `machines()` above, because it is not a housed machine: it has no
    /// hooded infeed and no discharge doorway, for the same reason a belt
    /// tile has neither. What it has instead is three open lanes and one
    /// closed rail, and two arrow grooves that must be genuinely cut rather
    /// than painted (the same distinction the junction tests exist for: a
    /// mesh cannot tell a recess from a rectangle painted dark, and neither
    /// can a screenshot at scene scale).
    #[test]
    fn the_splitter_is_open_on_three_sides_and_closed_on_the_fourth() {
        let p = palette();
        let m = machine_splitter(&p);
        let last = TILE_VOXELS as i32 - 1;
        let mid = (BELT_LANE.0 + BELT_LANE.1) / 2;
        let deck = BELT_SURFACE_Z;

        // Intake (-X), straight output (+X) and branch output (+Y): a lane
        // runs out to the tile edge at the belt's own running height on all
        // three, so product never meets a wall.
        assert!(m.filled(0, mid, deck), "the -X intake lane stops short of the tile edge");
        assert!(m.filled(last, mid, deck), "the +X straight output stops short of the tile edge");
        assert!(m.filled(mid, last, deck), "the +Y branch output stops short of the tile edge");
        // ...and nothing stands in any of them.
        for z in (deck + 1)..=(deck + 4) {
            assert!(!m.filled(0, mid, z), "something blocks the intake lane");
            assert!(!m.filled(last, mid, z), "something blocks the straight output");
            assert!(!m.filled(mid, last, z), "something blocks the branch output");
        }
        // The fourth side is a rail, not an output. It is BELOW the deck, as
        // a belt's own side frame is, so it closes the tile without standing
        // in front of the table the camera has to see into.
        assert!(m.filled(mid, 5, 6), "the -Y side has no rail, so the tile reads as open on all four sides");
        assert!(!m.filled(mid, 5, deck + 1), "the -Y rail stands above the deck and would occlude the table");

        // Both arrow grooves are real recesses, one voxel down, and they do
        // not overlap — two arrows crossing at the tile centre render as one
        // unreadable blob (see `splitter_straight_arrow`).
        let straight: std::collections::BTreeSet<(i32, i32)> =
            splitter_arrow_cells(0).into_iter().collect();
        let branch: std::collections::BTreeSet<(i32, i32)> =
            splitter_arrow_cells(1).into_iter().collect();
        assert!(!straight.is_empty(), "the straight arrow is empty");
        assert_eq!(straight.len(), branch.len(), "a quarter turn must not change an arrow's size");
        assert!(
            straight.is_disjoint(&branch),
            "the two arrow grooves overlap, so neither reads as an arrow"
        );
        for &(x, y) in straight.union(&branch) {
            assert!(!m.filled(x, y, SPLITTER_ARROW_Z), "the arrow groove at {x},{y} was painted, not cut");
            assert!(m.filled(x, y, SPLITTER_ARROW_Z - 1), "the arrow groove at {x},{y} has no floor to sit on");
        }

        // The lit slab fills exactly the straight groove, so it sits flush
        // rather than standing proud of the deck in the product's way.
        let arrow = splitter_arrow(&p);
        let lit: std::collections::BTreeSet<(i32, i32)> =
            arrow.voxels.keys().map(|&(x, y, _)| (x as i32, y as i32)).collect();
        assert_eq!(lit, straight, "the lit arrow does not match the groove cut for it");
    }

    #[test]
    fn every_belt_fed_face_has_an_open_inlet_at_the_belt_lane() {
        let p = palette();
        for j in machines(&p) {
            for (lane, housing) in j.belt_inlets.iter().zip(&j.housing_lanes) {
                let mid = (lane.0 + lane.1) / 2;
                assert!(
                    !j.model.filled(0, mid, BELT_SURFACE_Z + 2),
                    "{}: the -X face is solid at the belt lane centred on {mid} — a belt \
                     feeding it would run into a wall",
                    j.name
                );
                assert!(
                    j.model.filled(0, housing.0 - 2, BELT_SURFACE_Z + 2),
                    "{}: the inlet housing has no side frame, so its opening has no edge",
                    j.name
                );
            }
        }
    }

    /// Nothing is downstream of the delivery dock (product leaves the world
    /// there, `sim_core::sim::dock`) and nothing is downstream of the mixer
    /// or the vat on a *belt* — their outputs are fluids since the chain
    /// rework. So this checks the face each kind actually discharges solids
    /// through, and skips the ones that discharge none.
    #[test]
    fn every_belt_fed_face_has_an_open_outlet_at_the_belt_lane() {
        let p = palette();
        for j in machines(&p) {
            let Some(lane) = j.belt_outlet else { continue };
            let mid = (lane.0 + lane.1) / 2;
            assert!(
                !j.model.filled(j.last_x, mid, BELT_SURFACE_Z + 2),
                "{}: the +X face is solid at the belt's lane and height — product \
                 would appear on the belt out of nowhere",
                j.name
            );
        }
    }

    /// The chain rework's own junctions, and the check that stops the two
    /// kinds of junction being confused for each other. A pipe meets a
    /// machine at `PIPE_AXIS_Z`, six voxels above the belt's running
    /// surface, so a hood built for a belt is not a connection for a pipe —
    /// and a machine that no longer takes a belt on a face must not still be
    /// advertising a mouth there, which is the same "gauge describing a
    /// mechanism it does not have" failure the fluid tank refused.
    #[test]
    fn every_piped_face_carries_a_nozzle_and_no_belt_mouth() {
        let p = palette();
        for j in machines(&p) {
            for (face_x, lane) in [(0, j.fluid_in), (j.last_x, j.fluid_out)] {
                let Some(mid) = lane else { continue };
                assert!(
                    j.model.filled(face_x, mid, PIPE_AXIS_Z),
                    "{}: no fluid nozzle reaching the tile edge at x = {face_x}, so the pipe \
                     beside it would butt against a blank wall",
                    j.name
                );
                // The brass flange sits ON the boundary, which is what makes
                // the machine's half and the pipe's half read as one fitting
                // rather than as two stubs a voxel apart (`pipe_fitting_machine`).
                assert!(
                    j.model.filled(face_x, mid, PIPE_AXIS_Z + 4),
                    "{}: the nozzle at x = {face_x} has no flange on the tile boundary",
                    j.name
                );
            }
            if j.fluid_in.is_some() {
                assert!(
                    j.belt_inlets.is_empty(),
                    "{}: claims both a pipe inlet and a belt inlet on its -X face",
                    j.name
                );
                // And no infeed APRON either. That is the honest inverse
                // of `junction_surfaces_sit_exactly_at_the_belts_running_height`:
                // a machine's own body never reaches its tile edge above the
                // plinth, so "is the face solid" says nothing, but a plate
                // at the belt's running surface is drawn by exactly one
                // thing (`machine_infeed`) and its presence would mean this
                // machine is still advertising a conveyor hand-over it can
                // no longer receive.
                let mid = (BELT_LANE.0 + BELT_LANE.1) / 2;
                assert!(
                    !j.model.filled(0, mid, BELT_SURFACE_Z),
                    "{}: its -X face still carries a belt infeed apron, but nothing \
                     hands product to it on a belt any more",
                    j.name
                );
            }
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
            for lane in &j.belt_inlets {
                let mid = (lane.0 + lane.1) / 2;
                assert!(
                    j.model.filled(0, mid, BELT_SURFACE_Z),
                    "{}: no infeed apron at the belt's own surface height",
                    j.name
                );
            }
            if let Some(lane) = j.belt_outlet {
                let mid = (lane.0 + lane.1) / 2;
                assert!(
                    j.model.filled(j.last_x, mid, BELT_SURFACE_Z),
                    "{}: no discharge lip at the belt's own surface height",
                    j.name
                );
            }
        }
    }

    // ----- the belt family (the "Pipes & Conveyors Overhaul" plan's ticket 2)

    fn belt_shapes() -> [(&'static str, BeltShape); 3] {
        [
            ("belt", BeltShape::Straight),
            ("belt_corner_left", BeltShape::Corner(Turn::Left)),
            ("belt_corner_right", BeltShape::Corner(Turn::Right)),
        ]
    }

    /// Surface coordinates for any shape's cell, undoing the right corner's
    /// mirror so both corners are judged in the left corner's frame.
    fn surface_cell(shape: BeltShape, x: i32, y: i32) -> Option<SurfaceCell> {
        let last = TILE_VOXELS as i32 - 1;
        match shape {
            BeltShape::Straight => straight_surface_cell(x, y),
            BeltShape::Corner(Turn::Left) => corner_surface_cell(x, y),
            BeltShape::Corner(Turn::Right) => corner_surface_cell(x, last - y),
        }
    }

    /// The arrow's cells in a frame: everything standing at `BELT_ARROW_Z`.
    fn arrow_cells(m: &VoxModel) -> std::collections::BTreeSet<(i32, i32)> {
        m.voxels
            .keys()
            .filter(|&&(_, _, z)| z as i32 == BELT_ARROW_Z)
            .map(|&(x, y, _)| (x as i32, y as i32))
            .collect()
    }

    #[test]
    fn every_belt_frame_carries_an_arrow_on_its_surface_pointing_along_the_path() {
        let p = palette();
        let yellow = p.idx("hazard_yellow");
        for (name, shape) in belt_shapes() {
            for phase in 0..BELT_FRAMES {
                let m = belt_shape(&p, shape, phase);
                let cells = arrow_cells(&m);
                assert!(cells.len() >= 60, "{name} f{phase}: arrow has only {} cells", cells.len());
                let (mut apex_cells, mut base_cells) = (0, 0);
                for &(x, y) in &cells {
                    assert_eq!(
                        m.voxels.get(&(x as u32, y as u32, BELT_ARROW_Z as u32)),
                        Some(&yellow),
                        "{name} f{phase}: arrow cell {x},{y} is not the signage colour"
                    );
                    assert!(
                        m.filled(x, y, BELT_SURFACE_Z),
                        "{name} f{phase}: arrow cell {x},{y} floats — nothing under it at the running height"
                    );
                    let cell = surface_cell(shape, x, y)
                        .unwrap_or_else(|| panic!("{name} f{phase}: arrow cell {x},{y} is off the lane"));
                    // Pointing along the path means: no cell is ahead of the
                    // apex, and the head only widens as it trails behind it.
                    let trail = arrow_trail_u(&cell, phase);
                    assert!(trail >= 0.0, "{name} f{phase}: arrow cell {x},{y} is ahead of the apex — it points backwards");
                    assert!(
                        trail < ARROW_HEAD_LEN + ARROW_SHAFT_LEN,
                        "{name} f{phase}: arrow cell {x},{y} trails further than the arrow is long"
                    );
                    if trail < ARROW_HEAD_LEN {
                        assert!(
                            cell.v.abs() <= trail / ARROW_HEAD_LEN * ARROW_HEAD_HALF_WIDTH + 1e-3,
                            "{name} f{phase}: arrow cell {x},{y} is wider than the wedge allows this close to the apex"
                        );
                    } else {
                        assert!(cell.v.abs() < ARROW_SHAFT_HALF_WIDTH + 1e-3, "{name} f{phase}: shaft cell {x},{y} is off the shaft");
                    }
                    if trail <= 2.0 {
                        apex_cells += 1;
                    }
                    if cell.v.abs() >= ARROW_HEAD_HALF_WIDTH - 1.5 {
                        base_cells += 1;
                    }
                }
                assert!(apex_cells >= 1, "{name} f{phase}: the arrow has no apex on the centre line");
                assert!(base_cells >= 2, "{name} f{phase}: the wedge never reaches its full width — it is not a wedge");
                // Nothing but the arrow stands above the running surface
                // anywhere on the lane.
                for &(x, y, z) in m.voxels.keys() {
                    let (x, y, z) = (x as i32, y as i32, z as i32);
                    if surface_cell(shape, x, y).is_some() {
                        assert!(z <= BELT_ARROW_Z, "{name} f{phase}: something stands in the lane at {x},{y},{z}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_four_frames_of_a_straight_belt_differ_only_by_the_phase_shift() {
        let p = palette();
        let frames: Vec<VoxModel> = (0..BELT_FRAMES).map(|k| belt_shape(&p, BeltShape::Straight, k)).collect();
        let tile = TILE_VOXELS as i32;
        let (l0, l1) = BELT_LANE;
        let on_lane = |x: i32, y: i32, z: i32| (l0..=l1).contains(&y) && z >= BELT_SURFACE_Z - 1 && x < tile;
        for (k, frame) in frames.iter().enumerate() {
            let shift = k as i32 * BELT_FRAME_SHIFT;
            for x in 0..tile {
                for y in 0..tile {
                    for z in 0..16 {
                        let expected = if on_lane(x, y, z) {
                            frames[0].voxels.get(&(((x - shift).rem_euclid(tile)) as u32, y as u32, z as u32))
                        } else {
                            frames[0].voxels.get(&(x as u32, y as u32, z as u32))
                        };
                        assert_eq!(
                            frame.voxels.get(&(x as u32, y as u32, z as u32)),
                            expected,
                            "belt f{k}: voxel {x},{y},{z} is not frame 0 shifted by {shift}"
                        );
                    }
                }
            }
        }
        for a in 0..frames.len() {
            for b in (a + 1)..frames.len() {
                assert_ne!(frames[a].voxels, frames[b].voxels, "belt f{a} and f{b} are the same model");
            }
        }
    }

    #[test]
    fn the_four_frames_of_a_corner_share_a_structure_and_advance_the_arrow_a_quarter_pattern() {
        let p = palette();
        for turn in [Turn::Left, Turn::Right] {
            let shape = BeltShape::Corner(turn);
            let frames: Vec<VoxModel> = (0..BELT_FRAMES).map(|k| belt_shape(&p, shape, k)).collect();
            let structure = |m: &VoxModel| -> Vec<((u32, u32, u32), u8)> {
                m.voxels.iter().filter(|&(&(_, _, z), _)| (z as i32) < BELT_ARROW_Z).map(|(&k, &c)| (k, c)).collect()
            };
            let mut apexes = Vec::new();
            for (k, frame) in frames.iter().enumerate() {
                assert_eq!(structure(frame), structure(&frames[0]), "{turn:?} corner f{k}: structure differs from f0");
                // Where the arrow actually stands, read back off the voxels:
                // the circular mean of its centre-line cells' `u` (circular
                // because a frame can straddle the tile edge).
                let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0);
                for c in arrow_cells(frame).into_iter().filter_map(|(x, y)| surface_cell(shape, x, y)) {
                    if c.v.abs() < 1.0 {
                        let a = c.u / TILE_VOXELS as f32 * std::f32::consts::TAU;
                        sx += a.cos();
                        sy += a.sin();
                        n += 1;
                    }
                }
                assert!(n > 0, "{turn:?} corner f{k}: no centre-line cells");
                apexes.push(sy.atan2(sx) / std::f32::consts::TAU * TILE_VOXELS as f32);
            }
            for k in 1..frames.len() {
                let advance = (apexes[k] - apexes[k - 1] + 16.0).rem_euclid(32.0) - 16.0;
                assert!(
                    (advance - BELT_FRAME_SHIFT as f32).abs() < 2.0,
                    "{turn:?} corner f{k}: arrow advanced {advance} u, not {BELT_FRAME_SHIFT}"
                );
            }
            for a in 0..frames.len() {
                for b in (a + 1)..frames.len() {
                    assert_ne!(arrow_cells(&frames[a]), arrow_cells(&frames[b]), "{turn:?} corner f{a} and f{b} share an arrow");
                }
            }
        }
    }

    #[test]
    fn a_corner_is_open_on_exactly_its_two_connected_faces() {
        let p = palette();
        let last = TILE_VOXELS as i32 - 1;
        let (l0, l1) = BELT_LANE;
        for turn in [Turn::Left, Turn::Right] {
            // The plan: output on +X; Left is fed from the north (vox low
            // Y) face, Right from the south (vox high Y) face.
            let entry_y = match turn {
                Turn::Left => 0,
                Turn::Right => last,
            };
            let closed_y = last - entry_y;
            for phase in 0..BELT_FRAMES {
                let m = belt_shape(&p, BeltShape::Corner(turn), phase);
                let name = format!("{turn:?} corner f{phase}");
                for i in 0..=last {
                    let in_lane = (l0..=l1).contains(&i);
                    // Exit face (+X) and entry face carry the lane at the
                    // belt's running height, exactly BELT_LANE wide, with
                    // nothing standing in it.
                    assert_eq!(m.filled(last, i, BELT_SURFACE_Z), in_lane, "{name}: +X face lane wrong at y = {i}");
                    assert_eq!(m.filled(i, entry_y, BELT_SURFACE_Z), in_lane, "{name}: entry face lane wrong at x = {i}");
                    for z in (BELT_ARROW_Z + 1)..16 {
                        assert!(!m.filled(last, i, z), "{name}: +X face blocked at y = {i}, z = {z}");
                        assert!(!m.filled(i, entry_y, z), "{name}: entry face blocked at x = {i}, z = {z}");
                    }
                    // The other two faces carry no belt at all.
                    for z in 0..16 {
                        assert!(!m.filled(0, i, z), "{name}: the -X face is not closed at y = {i}, z = {z}");
                        assert!(!m.filled(i, closed_y, z), "{name}: the closed Y face is not closed at x = {i}, z = {z}");
                    }
                }
                // Rails meet a straight tile's rails voxel for voxel at
                // both open faces.
                let straight = belt(&p);
                for i in 0..=last {
                    for z in 4..=8 {
                        assert_eq!(
                            m.filled(last, i, z),
                            straight.filled(last, i, z),
                            "{name}: +X face differs from a straight belt at y = {i}, z = {z}"
                        );
                        assert_eq!(
                            m.filled(i, entry_y, z),
                            straight.filled(last, i, z),
                            "{name}: entry face differs from a straight belt's face at x = {i}, z = {z}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_belt_tile_is_open_above_its_lane_at_both_faces() {
        let p = palette();
        let last = TILE_VOXELS as i32 - 1;
        let (l0, l1) = BELT_LANE;
        for phase in 0..BELT_FRAMES {
            let m = belt_shape(&p, BeltShape::Straight, phase);
            for y in l0..=l1 {
                for x in [0, last] {
                    assert!(m.filled(x, y, BELT_SURFACE_Z), "belt f{phase}: lane stops short at x = {x}, y = {y}");
                    for z in (BELT_ARROW_Z + 1)..16 {
                        assert!(!m.filled(x, y, z), "belt f{phase}: face blocked at x = {x}, y = {y}, z = {z}");
                    }
                }
            }
        }
    }

    /// `belt()` is what the junction and splitter tests measure against, so
    /// it has to be the tile the scene actually draws at rest.
    #[test]
    fn the_plain_belt_is_frame_zero_of_the_straight_shape() {
        let p = palette();
        assert_eq!(belt(&p).voxels, belt_f0(&p).voxels, "`belt` is not frame 0");
    }

    #[test]
    fn the_splitters_deck_sits_at_the_belts_running_height_with_plain_rubber_stubs() {
        let p = palette();
        let m = machine_splitter(&p);
        let belt = belt(&p);
        let (l0, l1) = BELT_LANE;
        let last = TILE_VOXELS as i32 - 1;
        let rubber = p.idx("rubber");
        // Intake stub: rubber at exactly the belt's height and lane, and
        // no cleat pattern of its own.
        for x in 0..=3 {
            for y in l0..=l1 {
                assert_eq!(m.voxels.get(&(x, y as u32, BELT_SURFACE_Z as u32)), Some(&rubber), "splitter intake at {x},{y} is not plain rubber");
                assert!(!m.filled(x as i32, y, BELT_ARROW_Z), "splitter intake carries an arrow cleat at {x},{y}");
            }
        }
        // Rails identical to the belt's on the -X face.
        for y in 0..=last {
            for z in 4..=8 {
                assert_eq!(m.filled(0, y, z), belt.filled(0, y, z), "splitter -X face differs from a belt at y = {y}, z = {z}");
            }
        }
    }

    #[test]
    fn the_docks_deck_is_solid_out_to_the_platform_edge() {
        let p = palette();
        let dock = machine_dock(&p);
        let tile = TILE_VOXELS as i32;
        let last = tile * 2 - 1;
        let mid = (BELT_LANE.0 + BELT_LANE.1) / 2;

        assert_eq!(
            dock.size,
            [TILE_VOXELS * 2, TILE_VOXELS * 2, TILE_VOXELS],
            "the dock's model must be its 2x2 footprint at UI_UX.md §2's tile scale"
        );
        assert_eq!(
            DOCK_DECK_Z, BELT_SURFACE_Z,
            "the dock's deck must stand at the belt's own running height"
        );
        // Both tiles of the deck, at both inlet lanes: the near one the
        // reference chain feeds, and the far one a second line could.
        for lane_mid in [mid, mid + tile] {
            assert!(
                dock.filled(last, lane_mid, DOCK_DECK_Z),
                "the deck stops short of the +X footprint edge at y = {lane_mid}"
            );
            assert!(
                !dock.filled(last, lane_mid, DOCK_DECK_Z + 1),
                "something stands on the platform edge at a belt lane — the dock \
                 has no downstream conveyor, so nothing should promise one"
            );
        }

        // The despatch openings: empty through the wall, with a wall behind
        // each, framed. Driven off DOCK_DOOR_SPANS rather than off literals,
        // so moving a door moves its test with it.
        for (x0, x1) in DOCK_DOOR_SPANS {
            let door_mid = (x0 + x1) / 2;
            assert!(
                !dock.filled(door_mid, last - 3, DOCK_DECK_Z + 6),
                "the despatch opening at x = {door_mid} is filled — a painted door is not a door"
            );
            assert!(
                dock.filled(door_mid, last, DOCK_DECK_Z + 6),
                "the despatch opening at x = {door_mid} has no back wall, so it shows \
                 straight through the building"
            );
            assert!(
                dock.filled(x0 - 2, DOCK_WALL_FACE_Y - 1, DOCK_DECK_Z + 6),
                "the despatch opening at x = {door_mid} has no frame, so its edge is nothing"
            );
        }
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
            machine_mixer(&p).filled(16, 0, PIPE_AXIS_Z),
            "the mixer's inlet stub does not reach the tile edge the run sits against"
        );
        // The pipe's half of both joints is a fitting on the tile boundary:
        // the family test below pins its bore and flange there.
    }

    // -----------------------------------------------------------------------
    // The pipe family ("Pipes & Conveyors Overhaul" ticket 3)
    // -----------------------------------------------------------------------

    /// Which faces of a one-tile shape carry an open bore, in the vox
    /// coordinates the pipe section's header fixes.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum PipeFace {
        /// x = 31.
        East,
        /// x = 0.
        West,
        /// y = 0 — camera-facing.
        North,
        /// y = 31.
        South,
    }

    const ALL_FACES: [PipeFace; 4] = [PipeFace::East, PipeFace::West, PipeFace::North, PipeFace::South];

    /// The voxel of `face`'s boundary plane at `(along, up)`, where `along`
    /// is the in-plane horizontal index and `up` the Z.
    fn on_face(face: PipeFace, along: i32, up: i32) -> (i32, i32, i32) {
        let last = TILE_VOXELS as i32 - 1;
        match face {
            PipeFace::East => (last, along, up),
            PipeFace::West => (0, along, up),
            PipeFace::North => (along, 0, up),
            PipeFace::South => (along, last, up),
        }
    }

    /// A face is OPEN when the bore's centre pair on the boundary plane is
    /// empty AND the tube wall stands either side of it at axis height —
    /// i.e. there is genuinely a tube reaching this face with a hole in it.
    /// A capped end fails the first half; a face no leg reaches fails the
    /// second. Both are "closed" for the purpose of a joint.
    fn face_open(m: &VoxModel, face: PipeFace) -> bool {
        let z = PIPE_AXIS_Z;
        let bore_empty = [15, 16].iter().all(|&a| {
            let (x, y, zz) = on_face(face, a, z);
            !m.filled(x, y, zz)
        });
        let wall_present = [11, 20].iter().all(|&a| {
            let (x, y, zz) = on_face(face, a, z);
            m.filled(x, y, zz)
        });
        bore_empty && wall_present
    }

    /// Every shape with the faces the plan declares for it, and its fluid
    /// body. The table IS the plan's "Corner conventions"; a shape that
    /// drifts from it fails here before Godot ever rotates it wrong.
    fn pipe_shapes(p: &Palette) -> Vec<(&'static str, VoxModel, VoxModel, Vec<PipeFace>)> {
        use PipeFace::*;
        vec![
            ("pipe", pipe(p), pipe_fluid(p), vec![East, West]),
            ("pipe_corner", pipe_corner(p), pipe_fluid_corner(p), vec![East, North]),
            ("pipe_tee", pipe_tee(p), pipe_fluid_tee(p), vec![East, West, North]),
            ("pipe_cross", pipe_cross(p), pipe_fluid_cross(p), vec![East, West, North, South]),
            ("pipe_end", pipe_end(p), pipe_fluid_end(p), vec![West]),
        ]
    }

    #[test]
    fn every_pipe_shape_is_open_exactly_on_its_declared_faces() {
        let p = palette();
        for (name, shape, _, open) in pipe_shapes(&p) {
            for face in ALL_FACES {
                assert_eq!(
                    face_open(&shape, face),
                    open.contains(&face),
                    "{name}: face {face:?} should be {}",
                    if open.contains(&face) { "open" } else { "closed" }
                );
            }
        }
    }

    /// The plan's rotation rule, proved on the corner: two clockwise quarter
    /// turns take (+X, north) to (-X, south), so a corner whose neighbours
    /// are west and south is this model turned twice — which is what
    /// `main.gd` will do to it.
    #[test]
    fn a_corner_turned_twice_opens_west_and_south() {
        let p = palette();
        let turned = pipe_corner(&p).rotated_cw(2);
        assert!(face_open(&turned, PipeFace::West) && face_open(&turned, PipeFace::South));
        assert!(!face_open(&turned, PipeFace::East) && !face_open(&turned, PipeFace::North));
        let once = pipe_corner(&p).rotated_cw(1);
        assert!(face_open(&once, PipeFace::South) && face_open(&once, PipeFace::East), "one CW turn: +X -> south, north -> east");
    }

    /// Each fluid body lies inside its shape's bore: every voxel of the slab
    /// has empty bore above it at axis height (the bore's widest row), it
    /// reaches every open face and it stops short of every closed one.
    #[test]
    fn every_fluid_body_lies_inside_its_shapes_bore_and_reaches_only_open_faces() {
        let p = palette();
        for (name, shape, fluid, open) in pipe_shapes(&p) {
            assert_eq!(fluid.size[2], 1, "{name}: the fluid body must be one voxel tall");
            for &(x, y, _) in fluid.voxels.keys() {
                for z in PIPE_BORE_Z0 + 1..=PIPE_AXIS_Z {
                    assert!(
                        !shape.filled(x as i32, y as i32, z),
                        "{name}: fluid at ({x}, {y}) is inside the shape's wall at z = {z}"
                    );
                }
            }
            for face in ALL_FACES {
                let reaches = [15, 16].iter().any(|&a| {
                    let (x, y, _) = on_face(face, a, 0);
                    fluid.filled(x, y, 0)
                });
                assert_eq!(
                    reaches,
                    open.contains(&face),
                    "{name}: the fluid body {} reach face {face:?}",
                    if open.contains(&face) { "must" } else { "must not" }
                );
            }
        }
    }

    /// Every shape shows its liquid: the sight window is a genuinely empty
    /// column over the bore, from just above the bore floor to the top of
    /// the model, on a leg every shape has (+X for the junctions, mid-tile
    /// for the straight and the end).
    #[test]
    fn every_pipe_shape_has_a_sight_window_cut_to_the_top() {
        let p = palette();
        for (name, shape, _, _) in pipe_shapes(&p) {
            let window = if name == "pipe" || name == "pipe_end" { PIPE_WINDOW } else { PIPE_LEG_WINDOW };
            let top = shape.size[2] as i32 - 1;
            for x in window.0..=window.1 {
                for y in PIPE_BORE_Y.0..=PIPE_BORE_Y.1 {
                    for z in PIPE_BORE_Z0 + 1..=top {
                        assert!(!shape.filled(x, y, z), "{name}: window voxel ({x}, {y}, {z}) is not empty");
                    }
                }
            }
            // ...and it is a window, not a missing wall: the tube is still
            // there just outside it, either side.
            assert!(shape.filled(window.0 - 1, PIPE_BORE_Y.0, PIPE_AXIS_Z + 3) || shape.filled(window.1 + 1, PIPE_BORE_Y.0, PIPE_AXIS_Z + 3), "{name}: no tube wall frames the window");
        }
    }

    /// A fitting's flange sits ON the tile boundary at the pipe's axis
    /// height, its bore is open there, and it keeps clear of the leg window
    /// so it can be overlaid on any shape's +X leg.
    #[test]
    fn a_fittings_flange_sits_on_the_tile_boundary_at_pipe_axis_height() {
        let p = palette();
        let last = TILE_VOXELS as i32 - 1;
        for (name, fitting, r) in [
            ("pipe_fitting_machine", pipe_fitting_machine(&p), PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN),
            ("pipe_fitting_tank", pipe_fitting_tank(&p), PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN),
        ] {
            // The flange is centred on the tile midline, half a voxel from
            // voxel 15/16, so its reach along an axis is sqrt(r² - ¼).
            let reach = (r * r - 0.25).sqrt().floor() as i32;
            for (y, z) in [(15, PIPE_AXIS_Z + reach), (16, PIPE_AXIS_Z - reach), (15 - reach, PIPE_AXIS_Z), (16 + reach, PIPE_AXIS_Z)] {
                assert!(fitting.filled(last, y, z), "{name}: flange does not reach ({y}, {z}) on the boundary");
            }
            // Checked on the UNDERSIDE: the tank fitting's valve stands on
            // top of its flange.
            assert!(!fitting.filled(last, 15, PIPE_AXIS_Z - reach - 1), "{name}: flange is larger than its documented radius");
            for a in [15, 16] {
                assert!(!fitting.filled(last, a, PIPE_AXIS_Z), "{name}: the bore is plugged at the boundary");
            }
            for &(x, _, z) in fitting.voxels.keys() {
                let floor = if (z as i32) <= PIPE_AXIS_Z + PIPE_OUTER_R as i32 + 2 { PIPE_FITTING_X0 } else { PIPE_FITTING_TOP_X0 };
                assert!(x as i32 >= floor, "{name}: hardware at x = {x}, z = {z} would cover a leg window");
            }
        }
    }

    /// A fitting overlaid on a straight's LOW-X face — the first tile of a
    /// run leaving a machine, or the tile a tank discharges into — covers
    /// the straight's own joint flange, and on its +X face it covers plain
    /// tube. Two voxel solids z-fight wherever a voxel of both has an
    /// outward neighbour in neither (both then expose a face on the same
    /// plane), so every voxel the two share must have every outward
    /// neighbour inside one of them. "Outward" is away from the pipe axis;
    /// an inward face lies in the bore and is a ticket-3 call (`+ 0.4`).
    /// The tile-boundary plane itself is exempt: both end there by design,
    /// and what stands on the other side of it (the machine's nozzle, the
    /// tank's spout) is what the render checks.
    #[test]
    fn a_fitting_overlaid_on_either_face_of_a_straight_shares_no_outer_surface() {
        let p = palette();
        let shape = pipe(&p);
        for (name, fitting) in [
            ("pipe_fitting_machine", pipe_fitting_machine(&p)),
            ("pipe_fitting_tank", pipe_fitting_tank(&p)),
        ] {
            for turns in [0, 2] {
                let overlay = fitting.rotated_cw(turns);
                let mut both = shape.clone();
                both.merge(&overlay);
                let radial = |y: i32, z: i32| {
                    let (dy, dz) = (y as f32 - PIPE_C, z as f32 - PIPE_AXIS_Z as f32);
                    dy * dy + dz * dz
                };
                for &(x, y, z) in overlay.voxels.keys() {
                    let (x, y, z) = (x as i32, y as i32, z as i32);
                    if !shape.filled(x, y, z) {
                        continue;
                    }
                    for (nx, ny, nz) in [(x + 1, y, z), (x - 1, y, z), (x, y + 1, z), (x, y - 1, z), (x, y, z + 1), (x, y, z - 1)] {
                        if radial(ny, nz) < radial(y, z) || nx < 0 || nx >= TILE_VOXELS as i32 {
                            continue;
                        }
                        assert!(
                            both.filled(nx, ny, nz),
                            "{name} turned {turns}: the shape and the fitting both expose a face at \
                             ({x}, {y}, {z}) toward ({nx}, {ny}, {nz}) — they will z-fight there"
                        );
                    }
                }
            }
        }
    }
}
