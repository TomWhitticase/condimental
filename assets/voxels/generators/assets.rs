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
    // The floor family (the floor redesign). `main.gd`'s
    // `_floor_variant_for` picks one per tile from the tile's own
    // coordinates, so a world gets a floor with some life in it out of four
    // models — see the factory-floor section below.
    Asset {
        name: "floor_tile",
        build: floor_tile,
    },
    Asset {
        name: "floor_tile_worn",
        build: floor_tile_worn,
    },
    Asset {
        name: "floor_tile_patch",
        build: floor_tile_patch,
    },
    Asset {
        name: "floor_tile_drain",
        build: floor_tile_drain,
    },
    // The belt family (the "Pipes & Conveyors Overhaul" plan's ticket 2):
    // three shapes, `BELT_FRAMES` animation frames each, all authored for a
    // +X product path. The scene picks the shape from each tile's `links`
    // and the frame from the tick (ticket 5, `main.gd`'s `_belt_shape_for`
    // and `_belt_frame_for_tick`); `belt_f0` is also the build menu's icon
    // and the ghost. The old `belt`/`belt_inlet`/`belt_outlet` names are
    // gone — see the conveyor section's header.
    Asset {
        name: "belt_f0",
        build: belt_f0,
    },
    Asset {
        name: "belt_f1",
        build: belt_f1,
    },
    Asset {
        name: "belt_f2",
        build: belt_f2,
    },
    Asset {
        name: "belt_f3",
        build: belt_f3,
    },
    Asset {
        name: "belt_corner_left_f0",
        build: belt_corner_left_f0,
    },
    Asset {
        name: "belt_corner_left_f1",
        build: belt_corner_left_f1,
    },
    Asset {
        name: "belt_corner_left_f2",
        build: belt_corner_left_f2,
    },
    Asset {
        name: "belt_corner_left_f3",
        build: belt_corner_left_f3,
    },
    Asset {
        name: "belt_corner_right_f0",
        build: belt_corner_right_f0,
    },
    Asset {
        name: "belt_corner_right_f1",
        build: belt_corner_right_f1,
    },
    Asset {
        name: "belt_corner_right_f2",
        build: belt_corner_right_f2,
    },
    Asset {
        name: "belt_corner_right_f3",
        build: belt_corner_right_f3,
    },
    // Epic B's conveyor splitter and the lit arrow that says which of its two
    // outputs takes the next item. Grouped with the belts rather than with
    // the machines below because that is what a splitter is (see
    // `machine_splitter`); the `machine_` prefix on the body is only there
    // because `main.gd`'s MACHINE_MODELS keys `data/machines.ron` kinds, and
    // a splitter has a footprint in that file like every other placed thing.
    Asset {
        name: "machine_splitter",
        build: machine_splitter,
    },
    Asset {
        name: "machine_splitter_right",
        build: machine_splitter_right,
    },
    Asset {
        name: "machine_splitter_t_junction",
        build: machine_splitter_t_junction,
    },
    Asset {
        name: "machine_splitter_three_way",
        build: machine_splitter_three_way,
    },
    Asset {
        name: "splitter_arrow",
        build: splitter_arrow,
    },
    Asset {
        name: "machine",
        build: machine,
    },
    // The kinds `data/machines.ron` names that have a silhouette of their
    // own (the "Machines improvement" milestone, ticket 4). `machine` above
    // stays the fallback for every kind that doesn't — see `main.gd`'s
    // MACHINE_MODELS.
    Asset {
        name: "machine_crusher",
        build: machine_crusher,
    },
    Asset {
        name: "machine_mixer",
        build: machine_mixer,
    },
    Asset {
        name: "machine_cooking_vat",
        build: machine_cooking_vat,
    },
    // The two downstream kinds. They fell back to the generic `machine`
    // above until the "Visual overhaul round 2" epic's ticket 2, which is
    // exactly what the project owner's review of the rendered scene named
    // ("some machines are still missing assets or just reuse assets") —
    // every kind in `data/machines.ron` now has a silhouette of its own.
    Asset {
        name: "machine_bottler",
        build: machine_bottler,
    },
    Asset {
        name: "machine_packer",
        build: machine_packer,
    },
    // The chain's exit (the delivery-dock ticket): the tile finished cases
    // leave the world through. Not a processing kind — see `machine_dock`.
    Asset {
        name: "machine_dock",
        build: machine_dock,
    },
    // The motion overlays: one small model per processing kind holding only
    // the parts that move, in `MACHINE_FRAMES` arrangements, drawn over the
    // body by `main.gd` only while the machine is actually mid-process. See
    // the "Machine motion" section for the rules every family here follows.
    // The generic `machine` gets one too, for the reason it gets junction
    // hardware: a kind with no bespoke model must still look like it works.
    Asset {
        name: "machine_motion_f0",
        build: machine_motion_f0,
    },
    Asset {
        name: "machine_motion_f1",
        build: machine_motion_f1,
    },
    Asset {
        name: "machine_motion_f2",
        build: machine_motion_f2,
    },
    Asset {
        name: "machine_motion_f3",
        build: machine_motion_f3,
    },
    Asset {
        name: "machine_crusher_motion_f0",
        build: machine_crusher_motion_f0,
    },
    Asset {
        name: "machine_crusher_motion_f1",
        build: machine_crusher_motion_f1,
    },
    Asset {
        name: "machine_crusher_motion_f2",
        build: machine_crusher_motion_f2,
    },
    Asset {
        name: "machine_crusher_motion_f3",
        build: machine_crusher_motion_f3,
    },
    Asset {
        name: "machine_mixer_motion_f0",
        build: machine_mixer_motion_f0,
    },
    Asset {
        name: "machine_mixer_motion_f1",
        build: machine_mixer_motion_f1,
    },
    Asset {
        name: "machine_mixer_motion_f2",
        build: machine_mixer_motion_f2,
    },
    Asset {
        name: "machine_mixer_motion_f3",
        build: machine_mixer_motion_f3,
    },
    Asset {
        name: "machine_cooking_vat_motion_f0",
        build: machine_cooking_vat_motion_f0,
    },
    Asset {
        name: "machine_cooking_vat_motion_f1",
        build: machine_cooking_vat_motion_f1,
    },
    Asset {
        name: "machine_cooking_vat_motion_f2",
        build: machine_cooking_vat_motion_f2,
    },
    Asset {
        name: "machine_cooking_vat_motion_f3",
        build: machine_cooking_vat_motion_f3,
    },
    Asset {
        name: "machine_bottler_motion_f0",
        build: machine_bottler_motion_f0,
    },
    Asset {
        name: "machine_bottler_motion_f1",
        build: machine_bottler_motion_f1,
    },
    Asset {
        name: "machine_bottler_motion_f2",
        build: machine_bottler_motion_f2,
    },
    Asset {
        name: "machine_bottler_motion_f3",
        build: machine_bottler_motion_f3,
    },
    Asset {
        name: "machine_packer_motion_f0",
        build: machine_packer_motion_f0,
    },
    Asset {
        name: "machine_packer_motion_f1",
        build: machine_packer_motion_f1,
    },
    Asset {
        name: "machine_packer_motion_f2",
        build: machine_packer_motion_f2,
    },
    Asset {
        name: "machine_packer_motion_f3",
        build: machine_packer_motion_f3,
    },
    Asset {
        name: "hopper",
        build: hopper,
    },
    Asset {
        name: "hopper_fill",
        build: hopper_fill,
    },
    // The pipe run's supply vessel and the liquid column inside it (the
    // "Visual overhaul round 2" epic: ticket 1 made the tank a real,
    // player-loadable building, this ticket draws it).
    Asset {
        name: "fluid_tank",
        build: fluid_tank,
    },
    Asset {
        name: "fluid_tank_fill",
        build: fluid_tank_fill,
    },
    // The warehouse rack and the two models that show what is on it (the
    // warehouse-racks sprint, ticket W2). Grouped with the hopper and the
    // tank because all three are storage the player places and the sim
    // fills; see the "Warehouse rack" section for why this one is an open
    // frame rather than a housing with carved openings.
    Asset {
        name: "warehouse_rack",
        build: warehouse_rack,
    },
    Asset {
        name: "warehouse_pallet",
        build: warehouse_pallet,
    },
    Asset {
        name: "warehouse_pallet_load",
        build: warehouse_pallet_load,
    },
    // The pipe family ("Pipes & Conveyors Overhaul" ticket 3): shapes, the
    // fittings Godot overlays on any open leg, and one fluid body per shape.
    // See the pipe section's header for the face convention.
    Asset {
        name: "pipe",
        build: pipe,
    },
    Asset {
        name: "pipe_corner",
        build: pipe_corner,
    },
    Asset {
        name: "pipe_tee",
        build: pipe_tee,
    },
    Asset {
        name: "pipe_cross",
        build: pipe_cross,
    },
    Asset {
        name: "pipe_end",
        build: pipe_end,
    },
    Asset {
        name: "pipe_fitting_machine",
        build: pipe_fitting_machine,
    },
    Asset {
        name: "pipe_fitting_tank",
        build: pipe_fitting_tank,
    },
    // The clamp on a run the sim cannot name (issue #54), overlaid per tile.
    Asset {
        name: "pipe_fault",
        build: pipe_fault,
    },
    // The floating "!" over a placement that needs the player (a dock with
    // every place taken, first; racks and machines can raise it too).
    Asset {
        name: "warning_marker",
        build: warning_marker,
    },
    // Its red sibling: a person over a machine that would be working but
    // is short of its crew (`MachineLoadEntry::understaffed`).
    Asset {
        name: "understaffed_marker",
        build: understaffed_marker,
    },
    Asset {
        name: "pipe_fluid",
        build: pipe_fluid,
    },
    Asset {
        name: "pipe_fluid_corner",
        build: pipe_fluid_corner,
    },
    Asset {
        name: "pipe_fluid_tee",
        build: pipe_fluid_tee,
    },
    Asset {
        name: "pipe_fluid_cross",
        build: pipe_fluid_cross,
    },
    Asset {
        name: "pipe_fluid_end",
        build: pipe_fluid_end,
    },
    // The belt/pipe crossing: the pipe half the scene stacks over a belt, its
    // liquid, and the two merged for the icon. See the "Crossings" section.
    Asset {
        name: "pipe_crossing",
        build: pipe_crossing,
    },
    Asset {
        name: "pipe_fluid_crossing",
        build: pipe_fluid_crossing,
    },
    Asset {
        name: "crossing",
        build: crossing,
    },
    Asset {
        name: "item_tomato",
        build: item_tomato,
    },
    // The chain's second raw ingredient (the chain-rework epic). It needed a
    // model of its own for UI_UX.md §2b point 12's reason: `main.gd`'s
    // ITEM_MODELS falls back to a shipping box, so a new item id with no
    // entry ships bulk produce down the belt in sealed cardboard and every
    // structural test still passes.
    Asset {
        name: "item_sugar",
        build: item_sugar,
    },
    // The chain's THIRD raw ingredient, and the first *fluid* with a model
    // of its own. Nothing carries vinegar on a belt, so this is not a belt
    // item — it is what a shipment of the stuff looks like where the player
    // handles it as stock (`main.gd`'s FLUID_MODELS: the market slot and the
    // larder chip), both of which used to borrow the pipe's icon and so read
    // as "a pipe" rather than as vinegar.
    Asset {
        name: "item_vinegar",
        build: item_vinegar,
    },
    // The BBQ branch's three raws (issue #67). Until now all three fell
    // through `_market_asset_for` to the pipe, so the market drew one
    // nozzle three times over; `item_liquid_smoke` joins `item_vinegar` in
    // FLUID_MODELS and is likewise icon-only, the other two ride belts.
    Asset {
        name: "item_spice_mix",
        build: item_spice_mix,
    },
    Asset {
        name: "item_molasses",
        build: item_molasses,
    },
    Asset {
        name: "item_liquid_smoke",
        build: item_liquid_smoke,
    },
    // The third sauce's two raws (the MUSTARD epic). Both ride belts, so
    // both are authored for the game camera as well as vengi's; the row
    // argument that shaped the BBQ three is what shaped these, and it is
    // eight raws wide now (see `item_mustard_seed`).
    Asset {
        name: "item_mustard_seed",
        build: item_mustard_seed,
    },
    Asset {
        name: "item_turmeric",
        build: item_turmeric,
    },
    Asset {
        name: "item_ground_mustard",
        build: item_ground_mustard,
    },
    Asset {
        name: "item_paste_pail",
        build: item_paste_pail,
    },
    Asset {
        name: "item_mix_pail",
        build: item_mix_pail,
    },
    Asset {
        name: "item_ketchup_pail",
        build: item_ketchup_pail,
    },
    Asset {
        name: "item_bottle",
        build: item_bottle,
    },
    Asset {
        name: "item_box",
        build: item_box,
    },
    // The second sauce's two finished goods (`docs/BBQ_SAUCE_PLAN.md`).
    // `bottled_bbq` and `boxed_bbq` are separate item ids made on the same
    // bottler and packer, so they are separate models rather than a tint:
    // which sauce is on a belt is fixed by `data/recipes.ron`, not live
    // state, and a per-item tint would move colour out of palette.ron (the
    // reasoning `liquid_ketchup` records there). Registered here BEFORE the
    // items exist in `data/` — the epic's ticket 2 adds them — because
    // `main.gd`'s ITEM_MODEL_FALLBACK is a shipping box, so a new item id
    // arriving without a model ships bulk sauce in sealed cardboard and
    // every structural test still passes (§2b point 12).
    Asset {
        name: "item_bottle_bbq",
        build: item_bottle_bbq,
    },
    Asset {
        name: "item_box_bbq",
        build: item_box_bbq,
    },
    // The third sauce's two finished goods (the MUSTARD epic), registered
    // BEFORE `data/` has the items for the reason the BBQ pair above was:
    // the epic's content ticket is a separate branch, and ITEM_MODEL_FALLBACK
    // ships an unmodelled item in a plain shipping box with every test green.
    Asset {
        name: "item_bottle_mustard",
        build: item_bottle_mustard,
    },
    Asset {
        name: "item_box_mustard",
        build: item_box_mustard,
    },
    // The coin a paid contract throws off the dock (the payout-feedback
    // ticket). Not an item and not a building — the one asset in this list
    // that never occupies a tile.
    Asset {
        name: "coin",
        build: coin,
    },
    // The pulp the crusher throws while it works (the crusher-particles
    // change). Never occupies a tile either, and never appears in a menu —
    // it exists only to be thrown.
    Asset {
        name: "crush_bit",
        build: crush_bit,
    },
    // The title screen's hanging marquee sign (the title-screen pass): the
    // game's name in voxels, and the bulbs chasing round its frame in
    // `TITLE_SIGN_BULB_FRAMES` overlays. Never occupies a tile, never in a
    // menu — `title_scene.gd` hangs it over the hall behind the main menu.
    Asset {
        name: "title_sign",
        build: title_sign,
    },
    Asset {
        name: "title_sign_bulbs_f0",
        build: title_sign_bulbs_f0,
    },
    Asset {
        name: "title_sign_bulbs_f1",
        build: title_sign_bulbs_f1,
    },
    Asset {
        name: "title_sign_bulbs_f2",
        build: title_sign_bulbs_f2,
    },
    // A length of the sign's two rods, stacked above the body's own rods by
    // `title_scene.gd` until they run out of the top of the frame — however
    // tall the window is (`TITLE_SIGN_ROD_SEGMENT`).
    Asset {
        name: "title_sign_rods",
        build: title_sign_rods,
    },
    // A puff of steam off the cooking vat on the title screen. Like the
    // crush bit it exists only to be thrown — `title_scene.gd` lifts, swells
    // and shrinks a handful of them over the vat.
    Asset {
        name: "steam_puff",
        build: steam_puff,
    },
    // The staff-category icons (the staff-pools epic, ticket 2): four more
    // props that never occupy a tile, one per `data/staff.ron` category,
    // drawn only to be rendered by `voxicons` for the staff panel. See the
    // "Staff category icons" section.
    Asset {
        name: "staff_manufacturing",
        build: staff_manufacturing,
    },
    Asset {
        name: "staff_engineering",
        build: staff_engineering,
    },
    Asset {
        name: "staff_packaging",
        build: staff_packaging,
    },
    Asset {
        name: "staff_logistics",
        build: staff_logistics,
    },
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
//
// A power-trowelled slab-on-grade in a food plant, which is a specific real
// thing and not just "grey": one continuous pour, saw-cut into bays, falling
// to trench drains that the place is hosed down into at the end of a shift,
// worn where the traffic goes and cut out and re-poured where something once
// broke. UI_UX.md §3 asks for exactly that vocabulary; the floor is the
// largest single surface in the frame and had none of it.
//
// **What the first floor got wrong**, since this is a redesign and the
// reasons are the design:
//
//   * It drew a dark seam around ALL FOUR sides of every tile, so a joint
//     between two neighbours was two voxels of near-black. UI_UX.md §1 does
//     require the grid to stay legible, but the old tile over-delivered it:
//     a heavy border on a 32-voxel square reads as GROUT, and a floor of
//     grouted squares reads as a bathroom, not as a factory. The joint is
//     one voxel now, drawn on the low-X and low-Y edges only (so the shared
//     joint between two tiles is drawn once, by one of them), in a
//     `floor_seam` lightened from #33332F. That is a saw cut in a pour.
//   * It speckled the surface with single voxels of `concrete_light`, four
//     stops brighter than the ground under them. At scene scale that is
//     television static. Aggregate in a trowelled floor is a BLOTCH, low in
//     contrast and several voxels across, and that is also much kinder to
//     the mesher (see [`floor_mottle`]).
//   * Every tile in the world was the same tile. Nothing said the floor had
//     ever been used.
//
// Hence a family of four rather than one model. Everything here is a fixed
// function of position — never randomness — so regenerating a model is
// byte-stable, and `main.gd` picks between them from tile coordinates by the
// same rule every time (a save that is reloaded gets the floor it had).
//
// **The one hard constraint**: the slab is [`FLOOR_VOXELS`] thick and
// `main.gd`'s `FLOOR_Y` puts every belt, machine and item on top of it, so
// no floor detail may stand PROUD of the top surface. Detail here goes down
// into the slab (the drain) or is flush with it (everything else).

/// Slab thickness, mirrored by `main.gd`'s `FLOOR_VOXELS`.
const FLOOR_VOXELS: u32 = 3;
/// The z of the floor's walking surface — the highest voxel a floor model
/// may occupy.
const FLOOR_TOP: i32 = FLOOR_VOXELS as i32 - 1;
/// The trench drain's channel, in vox Y. Symmetric about the tile midline
/// between voxels 15 and 16 (UI_UX.md §2b point 20), so a row of drains laid
/// by `main.gd` is one continuous trench rather than a step at every joint.
const FLOOR_DRAIN_Y: (i32, i32) = (13, 18);

/// A cheap integer hash of a position, `salt` distinguishing one use of it
/// from another. A modulo of a linear `x*a + y*b` lays its results out in
/// obvious diagonal stripes — which is how the old speckle betrayed itself
/// as a pattern — so the two terms are XORed, not added.
fn floor_hash(x: i32, y: i32, salt: i64) -> i64 {
    ((x as i64).wrapping_mul(73_856_093)
        ^ (y as i64).wrapping_mul(19_349_663)
        ^ salt.wrapping_mul(83_492_791))
    .rem_euclid(1_000_003)
}

/// The bare slab: dark body, walking surface on top, nothing else. Every
/// floor model starts here.
fn floor_slab(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, FLOOR_VOXELS]);
    let last = TILE_VOXELS as i32 - 1;
    m.cuboid(
        (0, last),
        (0, last),
        (0, FLOOR_TOP - 1),
        p.idx("concrete_dark"),
    );
    m.cuboid(
        (0, last),
        (0, last),
        (FLOOR_TOP, FLOOR_TOP),
        p.idx("concrete_slab"),
    );
    m
}

/// Aggregate and trowel mottling: a few broad, ragged blotches a handful of
/// points either side of the slab's own tone.
///
/// Laid out over a coarse cell grid with each blotch offset inside its own
/// cell, rather than per voxel: the cell grid guarantees the blotches are
/// spread out (a pure per-voxel hash clumps and leaves bald patches), and
/// the offset stops the grid itself from showing. Most cells stay bare on
/// purpose — an evenly-covered floor is a texture, and a sparsely-covered
/// one is a surface.
///
/// Each blotch is built as a stack of rows of DIFFERENT lengths and offsets
/// rather than as a rectangle, and that is not fussiness, it is the second
/// thing the render corrected. A rectangle with its corners knocked off is a
/// plus sign, and thirteen plus signs a tile read as confetti dropped on the
/// floor. A ragged edge has no shape to recognise, which is what aggregate
/// in a trowelled pour actually looks like from three metres up.
fn floor_mottle(m: &mut VoxModel, p: &Palette, salt: i64) {
    const CELL: i32 = 8;
    let light = p.idx("concrete_mottle");
    let shade = p.idx("concrete_shade");
    let cells = TILE_VOXELS as i32 / CELL;

    for cy in 0..cells {
        for cx in 0..cells {
            let h = floor_hash(cx, cy, salt);
            if h % 8 >= 3 {
                continue; // most cells are bare slab
            }
            let w = 4 + ((h / 7) % 4) as i32;
            let d = 3 + ((h / 11) % 4) as i32;
            let x0 = cx * CELL + ((h / 13) as i32) % (CELL - w).max(1);
            let y0 = cy * CELL + ((h / 17) as i32) % (CELL - d).max(1);
            let color = if (h / 19) % 2 == 0 { light } else { shade };
            for row in 0..d {
                // Each row is inset and shortened by its own hash, so the
                // blotch's outline wanders instead of being a box.
                let r = floor_hash(x0, y0 + row, salt ^ 0x5F5F);
                let inset = (r % 3) as i32;
                let trim = ((r / 3) % 3) as i32;
                for x in x0 + inset..x0 + w - trim {
                    m.set(x, y0 + row, FLOOR_TOP, color);
                }
            }
        }
    }
}

/// The saw-cut control joint, one voxel wide, around all four edges.
///
/// **Four edges, not two, and that is a decision about ROTATION rather than
/// about the line.** Drawing it on the low-X and low-Y edges only makes each
/// shared joint the responsibility of exactly one of the two tiles that meet
/// there, so a joint is one voxel wide instead of two — which is tidier, and
/// which was the first cut. But it also makes the tile four-fold
/// ASYMMETRIC, and `main.gd` spins these slabs by a quarter turn at a time
/// to stop several thousand copies of one model reading as a repeating
/// texture (see `_floor_variant_for`'s neighbours). A spun asymmetric joint
/// puts two cuts along one boundary and none along the next, which is a
/// visible hole in the grid UI_UX.md §1 asks for.
///
/// Two voxels of joint is only a problem at the value the joint used to be.
/// At `floor_seam`'s lightened tone the doubled line still reads as a
/// hairline, and the render is what says so — which is the trade this
/// function is making: a slightly wider cut, in exchange for a floor with no
/// stamp in it.
fn floor_joint(m: &mut VoxModel, p: &Palette) {
    let last = TILE_VOXELS as i32 - 1;
    let seam = p.idx("floor_seam");
    for i in 0..=last {
        m.set(i, 0, FLOOR_TOP, seam);
        m.set(i, last, FLOOR_TOP, seam);
        m.set(0, i, FLOOR_TOP, seam);
        m.set(last, i, FLOOR_TOP, seam);
    }
}

/// The floor the great majority of the world is paved with: slab, mottle,
/// joint.
pub fn floor_tile(p: &Palette) -> VoxModel {
    let mut m = floor_slab(p);
    floor_mottle(&mut m, p, 0);
    floor_joint(&mut m, p);
    m
}

/// A slab that has been walked and driven over: a broad darkened area of
/// polished, ground-in traffic wear, roughly diagonal so it does not line up
/// with anything, plus the ordinary mottling around it.
///
/// The wear is `concrete_shade` — the same tone the mottle already uses at
/// blotch scale — so a worn tile beside a plain one is a change of DENSITY,
/// not a change of colour. A wear patch that announced itself would tile as
/// visibly as the thing it replaced.
pub fn floor_tile_worn(p: &Palette) -> VoxModel {
    let mut m = floor_slab(p);
    let last = TILE_VOXELS as i32 - 1;
    let shade = p.idx("concrete_shade");
    let mottle = p.idx("concrete_mottle");

    for y in 0..=last {
        for x in 0..=last {
            // A soft-edged band across the tile: inside its core it is
            // solid, and toward the edges the hash thins it out, which is
            // what keeps it from having an outline.
            let along = x + y; // constant along one screen axis
            let core = (along - last).abs();
            let h = floor_hash(x, y, 41);
            let thinning = 6 + (h % 9) as i32;
            if core < thinning {
                m.set(x, y, FLOOR_TOP, shade);
            } else if core < thinning + 3 && h % 3 == 0 {
                m.set(x, y, FLOOR_TOP, mottle);
            }
        }
    }

    floor_mottle(&mut m, p, 7);
    floor_joint(&mut m, p);
    m
}

/// A slab that was cut out and re-poured: a rectangle of newer concrete
/// inside its own saw cut.
///
/// This is the one floor model that is deliberately RECTILINEAR. Everything
/// else here is trying not to look man-made; a repair patch is the opposite
/// — somebody cut a square out of this floor with a saw, and the fact that
/// it does not match is the whole reason it reads. Used sparingly by
/// `main.gd`: a floor with a patch on every tenth tile is a chessboard.
///
/// **The patch is DARKER than the slab, and that direction was decided by
/// rendering it both ways.** Two brighter fills were tried first, on the
/// real-world theory that a fresh pour is paler than a floor polished by
/// traffic: `concrete_light` read as white paving slabs dropped at random,
/// and even `concrete` — one step up, the shared mid tone — measured +16
/// render units against the slab in an empty-world frame where the whole
/// rest of the floor varies by two. It was the loudest thing on screen. The
/// governing rule is UI_UX.md §4: metal is the deliberate bright exception,
/// so **the floor must never hold the brightest pixel in the frame**, and a
/// resurfaced bay reads perfectly well as a darker, greyer rectangle. The
/// saw cut around it is `concrete_dark` rather than `floor_seam`, which the
/// fill would otherwise swallow — and a cut into a repair being deeper and
/// dirtier than a control joint is true anyway. There is one of these per
/// ~125 tiles, so a darker line here is not the grout problem `floor_seam`
/// was lightened to fix; that was a line on all four edges of every tile.
pub fn floor_tile_patch(p: &Palette) -> VoxModel {
    let mut m = floor_slab(p);
    floor_mottle(&mut m, p, 3);

    let (x0, x1) = (7, 24);
    let (y0, y1) = (9, 25);
    // The cut itself, then the fresh pour inside it. Cut first so the fill
    // cannot spill over its own edge.
    m.cuboid(
        (x0, x1),
        (y0, y1),
        (FLOOR_TOP, FLOOR_TOP),
        p.idx("concrete_dark"),
    );
    m.cuboid(
        (x0 + 1, x1 - 1),
        (y0 + 1, y1 - 1),
        (FLOOR_TOP, FLOOR_TOP),
        p.idx("concrete_shade"),
    );
    // Newer concrete is not uniform either, just newer. The flecks are the
    // SLAB tone, so the patch is lifted toward the floor around it rather
    // than away from it.
    for y in y0 + 1..y1 {
        for x in x0 + 1..x1 {
            if floor_hash(x, y, 23) % 13 == 0 {
                m.set(x, y, FLOOR_TOP, p.idx("concrete_slab"));
            }
        }
    }

    floor_joint(&mut m, p);
    m
}

/// A trench drain crossing the slab — the detail that says "food factory"
/// rather than "warehouse" more than anything else on this floor. A plant
/// that cooks and bottles sauce is hosed down, and the water has to go
/// somewhere.
///
/// The channel runs along **+X** and is centred on the tile midline
/// (UI_UX.md §2b point 20's voxel 15.5, so a run of these is symmetric),
/// with its grating bars ACROSS it, on a 4-voxel period that divides the
/// tile exactly. `main.gd` lays these in whole rows, so consecutive tiles
/// join into one continuous trench with an unbroken bar spacing rather than
/// a puddle per tile — a drain that stopped at a tile boundary would be a
/// grid mark, which is what this redesign is getting rid of.
///
/// The channel is cut DOWN into the slab and the bars sit flush with the
/// walking surface, per this section's one hard constraint: a belt or a
/// machine can stand on a drain tile, exactly as it would in a real plant,
/// without anything poking up through it.
pub fn floor_tile_drain(p: &Palette) -> VoxModel {
    let mut m = floor_slab(p);
    let last = TILE_VOXELS as i32 - 1;
    floor_mottle(&mut m, p, 11);
    floor_joint(&mut m, p);

    // Symmetric about the tile midline between voxels 15 and 16.
    let (cy0, cy1) = FLOOR_DRAIN_Y;
    // The concrete lip the grating sits in, then the channel void below it.
    m.cuboid(
        (0, last),
        (cy0 - 1, cy1 + 1),
        (FLOOR_TOP, FLOOR_TOP),
        p.idx("concrete_shade"),
    );
    m.cuboid(
        (0, last),
        (cy0, cy1),
        (FLOOR_TOP - 1, FLOOR_TOP),
        p.idx("drain_grate"),
    );

    // Grating bars across the channel, flush with the walking surface. In
    // `concrete` rather than in the mottle tone: the bars are the only part
    // of this floor that WANT contrast with what is beside them (a grating
    // is read by its stripes), and pinning them to the mottle would dim
    // them every time the mottle is pulled closer to the slab.
    let mut x = 0;
    while x <= last {
        m.cuboid(
            (x, x + 1),
            (cy0, cy1),
            (FLOOR_TOP, FLOOR_TOP),
            p.idx("concrete"),
        );
        x += 4;
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
// or running backwards). So each tile carries **one** cleat.
//
// **The cleat is a plain transverse bar of dark rubber**, two voxels long
// along the path, one voxel proud of the running surface, a voxel short of
// the rubber's edge on each side — the moulded cleat a real conveyor belt
// wears, and nothing more. Until 2026-09-10 it was a solid `hazard_yellow`
// direction arrow (a filled wedge with a short shaft; a thin chevron had
// rendered as an `L` under this camera, UI_UX.md §2b point 23) and the
// player asked for the arrows to go. What the arrow said — which way the
// tile carries — is said in words instead: the build prompt names the
// ghost's facing (`✔ PLACE  Belt  ·  facing east`) and a placed tile's
// hover says `carries east`; a running line shows it by moving. The
// cleat's job is only to make the frames differ, so the belt visibly runs.
// **Raised rather than recessed**, for the camera: a recess shows its floor
// only where its own wall does not shadow it, and one edge of any shape
// always runs along the sight line; a bar is seen from above at every
// facing. The cost is that an item drawn at `BELT_SURFACE_Z + 1` stands
// one voxel into the cleat where they overlap; at scene scale that is a
// sliver under the item's foot.
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
// cleat reaches the exit face in the same frame a straight tile's would;
// the sim already parameterises an item across a corner by the same 0..1
// distance, so surface and items stay in step. The cleat's length is in
// `u` and its width in voxels, so on the inside of the bend it is thin and
// on the outside wide: the cleat bends with the belt.
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

/// The cleat's top voxel: one proud of the running surface (see the
/// section header for why raised, not recessed).
pub const BELT_CLEAT_Z: i32 = BELT_SURFACE_Z + 1;

/// Where the cleat's leading edge sits in frame 0, in `u` (voxels along
/// the path). Chosen so the frame-0 cleat — the one the UI icon is rendered
/// from — is centred in its tile: it covers the cells at `u` 15.5 and 16.5.
const CLEAT_LEAD_U: f32 = 17.0;
/// Length of the cleat along the path, in `u`.
const CLEAT_LEN: f32 = 2.0;
/// Half-width of the cleat, in voxels across the lane: it spans `BELT_LANE`
/// less one voxel of rubber margin each side.
const CLEAT_HALF_WIDTH: f32 = 8.0;

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
#[allow(
    dead_code,
    reason = "a reference shape for the junction tests, not an ASSETS entry"
)]
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
    assert!(
        phase < BELT_FRAMES,
        "belt phase {phase} is not in 0..{BELT_FRAMES}"
    );
    match shape {
        BeltShape::Straight => belt_straight(p, phase),
        BeltShape::Corner(turn) => belt_corner(p, turn, phase),
    }
}

/// A straight tile: the shared structure, the rubber lane, and the cleat.
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
    m.hazard_stripe_x(
        (0, last),
        (4, 4),
        (5, 6),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    m.hazard_stripe_x(
        (0, last),
        (27, 27),
        (5, 6),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    // Idlers under the belt. Fixed in every frame: the belt moves over
    // them, they do not travel with it. The two nearest the faces are what
    // shows under the rubber's end where a run stops in the open.
    for x in [4, 12, 20, 28] {
        m.cylinder_y((x, 7), 2.5, (l0, l1), p.idx("brass"));
    }

    // Running surface, then the cleat standing on it.
    m.cuboid(
        (0, last),
        (l0, l1),
        (BELT_SURFACE_Z - 1, BELT_SURFACE_Z),
        p.idx("rubber"),
    );
    for x in 0..=last {
        for y in l0..=l1 {
            let cell = straight_surface_cell(x, y).expect("lane cell");
            if cleat_covers(&cell, phase) {
                m.set(x, y, BELT_CLEAT_Z, p.idx("rubber_dark"));
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
                        let c = if (x + y + z).rem_euclid(6) < 3 {
                            "hazard_yellow"
                        } else {
                            "hazard_black"
                        };
                        m.set(x, y, z, p.idx(c));
                    }
                }
            }
            if (9.0..23.0).contains(&r) {
                m.set(x, y, 2, p.idx("rubber_dark"));
            }
            if let Some(cell) = corner_surface_cell(x, y) {
                m.cuboid(
                    (x, x),
                    (y, y),
                    (BELT_SURFACE_Z - 1, BELT_SURFACE_Z),
                    p.idx("rubber"),
                );
                if cleat_covers(&cell, phase) {
                    m.set(x, y, BELT_CLEAT_Z, p.idx("rubber_dark"));
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
    Some(SurfaceCell {
        u: x as f32 + 0.5,
        v: y as f32 + 0.5 - TILE_VOXELS as f32 / 2.0,
    })
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
    Some(SurfaceCell {
        u: theta * u_per_rad,
        v: r - TILE_VOXELS as f32 / 2.0,
    })
}

/// Distance wrapped into one tile's pattern period, `-16..16`.
fn wrap_period(d: f32) -> f32 {
    let half = TILE_VOXELS as f32 / 2.0;
    (d + half).rem_euclid(TILE_VOXELS as f32) - half
}

/// The cleat's leading edge's `u` at `phase`.
pub fn cleat_lead_u(phase: u32) -> f32 {
    (CLEAT_LEAD_U + (phase as i32 * BELT_FRAME_SHIFT) as f32).rem_euclid(TILE_VOXELS as f32)
}

/// How far behind the cleat's leading edge a cell sits along the path, in
/// `u`, wrapped to the pattern period (negative = ahead of the edge).
pub fn cleat_trail_u(cell: &SurfaceCell, phase: u32) -> f32 {
    wrap_period(cleat_lead_u(phase) - cell.u)
}

/// Does the cleat at `phase` stand on this cell? A constant-width bar
/// `CLEAT_LEN` long behind its leading edge.
pub fn cleat_covers(cell: &SurfaceCell, phase: u32) -> bool {
    let trail = cleat_trail_u(cell, phase);
    (0.0..CLEAT_LEN).contains(&trail) && cell.v.abs() < CLEAT_HALF_WIDTH
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
// facing EAST, and east is vox +X. A splitter's left output is one quarter
// turn CLOCKWISE from its facing in sim terms (`splitter.rs` owns that
// rule), so for an east-facing splitter the left runs SOUTH — world -Z,
// which this file's junction header establishes is vox **+Y** — and the
// right runs NORTH, vox -Y. The default `left` body is a T laid out as:
// intake from -X, straight exit at +X, left exit at +Y, and a closed rail
// along -Y. That closed rail is the near, camera-facing edge (vox low-Y is
// world +Z), which frames the deck at belt height without standing in front
// of it. The other three modes' bodies (`SplitterExits`) open and close the
// same three sides by the same rules: a closed side gets the rail, an open
// one a groove.
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

/// A conveyor splitter in `left` mode, the default: straight on and left.
///
/// See this section's header for the layout and for why the next-output
/// readout is inlaid in the deck rather than standing on it.
pub fn machine_splitter(p: &Palette) -> VoxModel {
    splitter_body(p, SplitterExits::LEFT)
}

/// A splitter in `right` mode: straight on and right.
pub fn machine_splitter_right(p: &Palette) -> VoxModel {
    splitter_body(p, SplitterExits::RIGHT)
}

/// A splitter in `t_junction` mode: left and right, nothing straight on.
pub fn machine_splitter_t_junction(p: &Palette) -> VoxModel {
    splitter_body(p, SplitterExits::T_JUNCTION)
}

/// A splitter in `three_way` mode: straight on, left and right.
pub fn machine_splitter_three_way(p: &Palette) -> VoxModel {
    splitter_body(p, SplitterExits::THREE_WAY)
}

/// Which of a splitter's three exits a body is built open — one per
/// `sim_core::sim::SplitMode`, whose `sides()` this mirrors. Left is vox +Y
/// and right is vox -Y (this section's header has why).
#[derive(Clone, Copy)]
struct SplitterExits {
    straight: bool,
    left: bool,
    right: bool,
}

impl SplitterExits {
    const LEFT: Self = Self {
        straight: true,
        left: true,
        right: false,
    };
    const RIGHT: Self = Self {
        straight: true,
        left: false,
        right: true,
    };
    const T_JUNCTION: Self = Self {
        straight: false,
        left: true,
        right: true,
    };
    const THREE_WAY: Self = Self {
        straight: true,
        left: true,
        right: true,
    };

    /// The quarter turns of [`splitter_arrow_cells`] that point at each open
    /// exit: 0 straight, 1 left (+Y), 3 right (-Y).
    fn arrow_turns(self) -> Vec<u32> {
        [(self.straight, 0), (self.left, 1), (self.right, 3)]
            .into_iter()
            .filter_map(|(open, turns)| open.then_some(turns))
            .collect()
    }
}

/// One splitter body with `exits` open and every other side railed shut.
fn splitter_body(p: &Palette, exits: SplitterExits) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 16]);
    let last = TILE_VOXELS as i32 - 1;
    let (l0, l1) = BELT_LANE;
    let (t0, t1) = SPLITTER_TABLE;
    let deck = (BELT_SURFACE_Z - 1, BELT_SURFACE_Z);
    // With no straight exit the tile ends at a rail hard against the table,
    // and the side frames stop there with it.
    let end_x = if exits.straight { last } else { t1 + 3 };

    // Legs: the belt's own four, plus two more under each branch stub, so
    // the tile stands on something wherever it has deck.
    for (x0, x1) in [(2, 4), (27, 29)] {
        for (y0, y1) in [(5, 7), (24, 26)] {
            m.cuboid((x0, x1), (y0, y1), (0, 4), p.idx("steel_dark"));
        }
    }
    for (open, (y0, y1)) in [(exits.left, (28, 30)), (exits.right, (1, 3))] {
        if open {
            for (x0, x1) in [(4, 6), (25, 27)] {
                m.cuboid((x0, x1), (y0, y1), (0, 4), p.idx("steel_dark"));
            }
        }
    }

    // Side frames at exactly the belt's own height, so a splitter dropped
    // into a run reads as part of the line. A closed side is one unbroken
    // rail; an open one is only the two stubs either side of the branch
    // mouth, and the branch's own rails carry on from them.
    for (open, (y0, y1), (b0, b1)) in [
        (exits.right, (4, 6), (0, 6)),
        (exits.left, (25, 27), (25, last)),
    ] {
        if open {
            for (x0, x1) in [(0, l0 - 1), (l1 + 1, end_x)] {
                m.cuboid((x0, x1), (y0, y1), (4, 8), p.idx("steel_dark"));
            }
            for (x0, x1) in [(4, 6), (25, 27)] {
                m.cuboid((x0, x1), (b0, b1), (4, 8), p.idx("steel_dark"));
            }
        } else {
            m.cuboid((0, end_x), (y0, y1), (4, 8), p.idx("steel_dark"));
        }
    }
    if !exits.straight {
        m.cuboid((t1 + 1, end_x), (4, 27), (4, 8), p.idx("steel_dark"));
    }

    // Hazard striping on the outer face of the near rail the camera can see
    // (UI_UX.md §3), matching `belt_body`'s — broken where the right branch
    // leaves through it.
    let near_spans = if exits.right {
        vec![(0, l0 - 1), (l1 + 1, end_x)]
    } else {
        vec![(0, end_x)]
    };
    for x in near_spans {
        m.hazard_stripe_x(
            x,
            (4, 4),
            (5, 6),
            3,
            p.idx("hazard_yellow"),
            p.idx("hazard_black"),
        );
    }

    // Rollers: one under the intake, and one under each open exit.
    m.cylinder_y((3, 7), 2.5, (l0, l1), p.idx("brass"));
    if exits.straight {
        m.cylinder_y((28, 7), 2.5, (l0, l1), p.idx("brass"));
    }
    if exits.left {
        m.cylinder_x((28, 7), 2.5, (l0, l1), p.idx("brass"));
    }
    if exits.right {
        m.cylinder_x((3, 7), 2.5, (l0, l1), p.idx("brass"));
    }

    // Running surfaces. Plain rubber where product travels in a straight
    // line — the intake and each exit stub — at the belt's own height and
    // lane, so every stub lines up with whatever belt tile is laid against
    // it. Deliberately NO arrow cleat here: the belt family's arrow says
    // "this way", and on a splitter that is the lit arrow's job — a static
    // yellow mark beside an amber one would be a second direction readout on
    // the one tile that has a live one.
    m.cuboid((0, t0 - 1), (l0, l1), deck, p.idx("rubber"));
    if exits.straight {
        m.cuboid((t1 + 1, last), (l0, l1), deck, p.idx("rubber"));
    }
    if exits.left {
        m.cuboid((l0, l1), (t1 + 1, last), deck, p.idx("rubber"));
    }
    if exits.right {
        m.cuboid((l0, l1), (0, t0 - 1), deck, p.idx("rubber"));
    }

    // The diverter table itself: stainless, because it is the one part of a
    // splitter that is a mechanism rather than a surface, and it is what
    // makes the tile tell itself apart from the belt run it sits in.
    //
    // `steel` and NOT `steel_light`, which is what it was first built with:
    // rendered in the scene the palette's brightest metal is a blown-out
    // white square filling most of the tile, and everything cut into it
    // (the arrows, the seam) washes out against it. Mid steel keeps the
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
    for (x, y) in [
        (t0 + 2, t0 + 2),
        (t1 - 2, t0 + 2),
        (t0 + 2, t1 - 2),
        (t1 - 2, t1 - 2),
    ] {
        m.set(x, y, deck.1, p.idx("steel_dark"));
    }

    // Brass threshold at each exit lip — the same accent every other
    // "product passes through here" fitting in this factory carries, and
    // here it is what counts the tile's ways out.
    if exits.straight {
        m.cuboid((last, last), (l0, l1), (deck.1, deck.1), p.idx("brass"));
    }
    if exits.left {
        m.cuboid((l0, l1), (last, last), (deck.1, deck.1), p.idx("brass"));
    }
    if exits.right {
        m.cuboid((l0, l1), (0, 0), (deck.1, deck.1), p.idx("brass"));
    }

    // One arrow groove at each open exit's lip. Cut, not painted: a painted
    // arrow is a decal the deck's own shading flattens, where a recess keeps
    // an edge at every angle and gives `splitter_arrow` somewhere to sit
    // flush instead of standing in the product's way.
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
    // Every open exit's groove is cut whether or not it is lit, which is the
    // whole of UI_UX.md §7's never-colour-alone rule here: what the player
    // reads is WHICH OF THE POSITIONS is glowing, and every position is a
    // visible shape on the deck even with nothing lit at all.
    for turns in exits.arrow_turns() {
        for (x, y) in splitter_arrow_cells(turns) {
            m.erase(x, y, SPLITTER_ARROW_Z);
            m.set(x, y, SPLITTER_ARROW_Z - 1, p.idx("steel_dark"));
        }
    }

    // Drive cabinet, clear of every lane: the one piece of the tile that
    // says a splitter is powered kit rather than a bend in a conveyor. Kept
    // below the deck's own height so it cannot stand in front of the table
    // it belongs to. It sits along the near (-Y) edge ahead of the rail when
    // that edge is closed; when the right branch leaves through it, the
    // cabinet shrinks into the near +X corner beside the branch's rails.
    let (cx, cy, lamp) = if exits.right {
        ((28, last), (0, 3), (29, 30))
    } else {
        ((21, 28), (0, 3), (23, 26))
    };
    m.cuboid(cx, cy, (0, 7), p.idx("paint_blue"));
    m.cuboid(cx, cy, (8, 8), p.idx("steel_light"));
    m.cuboid(lamp, (0, 0), (3, 5), p.idx("status_green"));
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
    m.hazard_stripe_x(
        (0, last),
        (0, 0),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    m.hazard_stripe_x(
        (0, last),
        (last, last),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    for y in 0i32..=last {
        let a = if (y + 2).rem_euclid(6) < 3 {
            p.idx("hazard_yellow")
        } else {
            p.idx("hazard_black")
        };
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

/// The BBQ branch's three raw ingredients — a **spice bag**, a **molasses
/// jar** and a **liquid-smoke drum** (issue #67).
///
/// All three went on sale the moment the BBQ gate opened and none of them
/// had a model, so `main.gd`'s `_market_asset_for` fell through to
/// `PIPE_ASSET` for each: the market drew three identical pipe nozzles for
/// three different shipments, and a tank full of liquid smoke was titled
/// "Fluid Tank — Liquid Smoke" beside a picture of a pipe. That is UI_UX.md
/// §2b point 12's silent fallback again, in the same quiet form the vinegar
/// jug was drawn to end.
///
/// **They are designed as a ROW, not as three objects**, because the frame
/// that has to work is the market, where every raw on sale stands side by
/// side — six of them when this was written, eight since the mustard branch:
/// a red sphere (tomato), a kraft sack (sugar), an amber jug (vinegar), and
/// then these three. So each takes a silhouette none of the others has — a
/// tied paper bag, a squat capped jar, a straight-sided ribbed drum — and a
/// value none of the others has, rather than three variations on
/// "container". At icon scale it is the proportion of the parts that
/// identifies a thing (§2b point 31), and the part doing the work here is
/// respectively the tie, the cap and the hoops. The bag is the newest of
/// the three and the one that has to work hardest, because the row already
/// holds a bag: see `item_spice_mix` for what keeps it from being sugar's.
///
/// **Which camera each is authored for differs, and it follows what
/// carries it.** `spice_mix` and `molasses` are solids: a hopper unloads
/// them onto a belt and the game's fixed isometric camera sees them there
/// (+X and low Y, §2b point 11), while the market renders the same model
/// through `vengi`'s opposite corner (§2b point 35). Two cameras, so
/// neither gets the detail to itself: the identifying marks on those two
/// are either on TOP (both cameras look down) or wrapped all the way round
/// the body, the way `item_vinegar`'s label band is. `liquid_smoke` is a
/// fluid — nothing can carry it on a belt — so its only consumer is the
/// icon pass, and it is authored for `vengi`'s corner alone, exactly as
/// `item_vinegar` is and for the same reason.
///
/// One bag of ground spice: a small spice-dyed paper sack, tied at the
/// neck, with a spill of the blend at its mouth.
///
/// **A SACK, and deliberately `item_sugar`'s sack one size down** — the
/// project owner's call, 2026-09-21. This was an open steel tin with the
/// blend heaped above its rim, drawn wide and low so that no two of the
/// raws in the market row shared an outline; the row argument above is
/// still the one that matters, which is why the SIZE is as load-bearing
/// here as the colour. Two bags of bulk dry goods are told apart the way
/// two real ones are — by how big they are and what is printed on them — so
/// this one is narrower, shallower AND shorter than the sugar sack in every
/// dimension (a test pins all three), and its paper is `sack_spice` where
/// sugar's is undyed `sack_kraft`, 123/255 of luma apart.
///
/// What the heap used to do, the spill now does: it puts the substance's
/// own colour on the surface BOTH cameras look down at (§2b point 11 for
/// the game's, point 35 for the market's), and it is the one bright thing
/// on the bag for the reason `item_sugar`'s crystal is — a sack alone is a
/// sack, and it is the pairing of the sack with what is coming out of it
/// that names the ingredient.
pub fn item_spice_mix(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    let sack = p.idx("sack_spice");
    // Slumped body over a tucked flat base, widest at the base and
    // narrowing toward the neck, so it reads as full and soft rather than
    // as a box — `item_sugar`'s own note records what the stepped-cuboid
    // version of this looked like, and it is not worth rediscovering.
    m.ellipsoid((8, 8, 4), (4.6, 3.8, 4.2), sack);
    m.cuboid((5, 10), (6, 9), (0, 1), sack);
    // The gathered neck, pinched to a ridge along X.
    m.cuboid((6, 9), (7, 8), (8, 9), sack);
    // A twine tie standing one voxel PROUD of that neck, where the sugar
    // sack's fold is a flat dark band flush with it. That overhang is the
    // one silhouette difference between the two bags, and it is what keeps
    // them apart in the market row at a size where a colour can be lost to
    // a shadow; `label_kraft` rather than a dark cord because against
    // `sack_spice` a dark band is 14/255 and this is 114/255.
    m.cuboid((5, 10), (7, 8), (10, 10), p.idx("label_kraft"));
    // The blend itself, spilling from the mouth and dusted on the floor
    // beside the bag. `spice_paprika` is the substance's own colour
    // wherever it appears (the palette's one-colour-per-substance rule), so
    // the bag on the belt and the crate in the market are the same stuff.
    let paprika = p.idx("spice_paprika");
    m.cuboid((6, 9), (7, 8), (11, 11), paprika);
    // Whole spice through the grind, and a scatter of the ground blend on
    // the ground. `bbq_brown` rather than a black for the fleck, exactly as
    // the tin used: on a face this camera lights, a black reads as a hole
    // in the spill (§2b point 21). Fixed positions, never random — a review
    // screenshot has to be repeatable.
    m.set(7, 7, 11, p.idx("bbq_brown"));
    for (x, y) in [(3, 7), (12, 9), (8, 4)] {
        m.set(x, y, 0, paprika);
    }
    m
}

/// One jar of molasses: a narrow glass jar of something nearly black under a
/// flat screw cap, with a kraft label round its belly.
///
/// **Straight-sided with a flat cap, where `item_bottle` is shouldered with
/// a narrow neck** — and that contrast is the point rather than a detail.
/// The finished BBQ bottle (`item_bottle_bbq`) is also a dark liquid behind
/// glass under a kraft label, so the raw and the product would be one object
/// drawn twice if this were another bottle. `bottle`'s own doc records that
/// the gap between body and cap is what stops a bottle reading as a jar;
/// this takes that gap away deliberately, so the two silhouettes part
/// company at a glance and on a belt.
///
/// It is narrow and TALL, not squat: see the rework note in the body, where
/// the first two cuts were squat and read as the spice tin standing next to
/// it in the market row.
///
/// The contents are `liquid`-class and nearly black, the glass above the
/// fill line is not: the headspace band is what says "you are looking into a
/// container" rather than at a dark block, which is the same trick `pail`
/// and `item_vinegar` play.
pub fn item_molasses(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    let glass = p.idx("glass_pale");
    // One straight-sided body, no shoulder and no neck. This took the two
    // rework passes §2b point 9 says to budget: the first cut tapered
    // through three radii in four voxels of height, which at this scale is
    // a staircase rather than a shoulder, and the wide cap on top of it ate
    // 40% of the silhouette. The second was still squat enough to read as a
    // tin. It is narrow and tall now precisely BECAUSE the spice tin beside
    // it in the market is wide and low — the pair has to differ in outline
    // before it differs in colour.
    m.cylinder_z((8, 8), 3.8, (0, 11), glass);
    // Everything below the fill line is syrup, glass included — the same
    // one-pass repaint `item_vinegar` uses, and for the same reason: a jar
    // whose glass and contents are separately modelled is two shells that
    // z-fight, and at 16 voxels nobody can see the difference anyway.
    // The four voxels of empty glass left above it are load-bearing: that
    // band is the only thing saying "container of something dark" rather
    // than "dark block".
    let syrup = p.idx("molasses_black");
    for z in 0..=9 {
        for y in 0..ITEM_VOXELS as i32 {
            for x in 0..ITEM_VOXELS as i32 {
                if m.filled(x, y, z) {
                    m.set(x, y, z, syrup);
                }
            }
        }
    }
    // A tamper band under the cap, and it is there for a rendering reason
    // rather than a realistic one: `glass_pale` and `steel_light` sit within
    // 35 of each other in luma, so an empty glass headspace touching a steel
    // cap merged into one pale block and the jar read as a pale cylinder
    // with stripes. One dark ring between them separates the two, and it is
    // also the step the cap screws down onto.
    m.cylinder_z((8, 8), 3.4, (12, 12), p.idx("bbq_brown"));
    // The cap: two voxels tall, overhanging that band. Steel rather than a
    // painted colour because it is the top of the silhouette and the only
    // bright thing on an almost-black item, so it is what a player's eye
    // lands on first — but two voxels, not four, because the thing being
    // identified is the JAR.
    m.cylinder_z((8, 8), 4.2, (13, 14), p.idx("steel_light"));
    // The label, wrapped all the way round on the surface voxels rather
    // than laid on one face as a slab (`item_vinegar`'s band, for the
    // reason given there): this model is seen from the game's camera on a
    // belt and from vengi's in the market, and a band on one side is
    // invisible from the other. Sideways exposure only, so it cannot creep
    // over the shoulder.
    let kraft = p.idx("label_kraft");
    for z in 3..=5 {
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
                    m.set(x, y, z, kraft);
                }
            }
        }
    }
    m
}

/// One drum of liquid smoke — the BBQ branch's raw FLUID, and the second
/// asset (after `item_vinegar`) that exists purely to be an icon.
///
/// Nothing carries liquid smoke on a belt: it is bought, poured into a
/// `fluid_tank` and drawn by the BBQ vat. Its only consumers are the market
/// crate, the larder chip and the fluid tank's own configuration panel, all
/// of which render `assets/voxels/icons/`, so this is authored for
/// **vengi's camera — the -X/+Y corner** (§2b point 35), not the game's.
/// That is why the bung sits where it does; put it on the game-facing side
/// and the icon is a plain cylinder.
///
/// **The body carries the fluid's own colour** (`smoke_slate`, equal to
/// `main.gd`'s `FLUID_TINTS["liquid_smoke"]`), which is `item_vinegar`'s
/// rule: the drum in the market and the liquid standing in the run are the
/// same substance, so a player who has seen one recognises the other. What
/// keeps it from being a grey cylinder is the hardware — two rolling hoops
/// and a chime at each end, in bright stainless — and those also satisfy
/// §4's "metal keeps the brightest pixel" on an otherwise dark, matte
/// object.
pub fn item_liquid_smoke(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    let steel = p.idx("steel");
    // The painted body: straight-sided and full height, so the silhouette
    // is a drum rather than a pot. Nothing else in the raws is a plain
    // vertical cylinder — the tin is low, the jar is capped, the jug is
    // bellied — so the straightness is doing identification work.
    m.cylinder_z((8, 8), 4.8, (0, 13), p.idx("smoke_slate"));
    // Two rolling hoops, proud of the body the way a real drum's are, and
    // the chimes that cap each end. Drawn wide and then the top chime
    // re-opened, so the lid inside it stays visible from above — both
    // cameras look down, and a solid disc here would hide the bung.
    for z in [(4, 5), (8, 9)] {
        m.cylinder_z((8, 8), 5.2, z, steel);
    }
    m.cylinder_z((8, 8), 5.2, (0, 0), p.idx("steel_light"));
    m.cylinder_z((8, 8), 5.2, (13, 13), p.idx("steel_light"));
    m.cylinder_z((8, 8), 4.0, (13, 13), p.idx("smoke_slate"));
    // The bung, on the corner vengi's camera actually sees (this function's
    // doc), standing one voxel proud of the lid so it reads as a fitting
    // rather than as a stain. Brass, the one warm accent, on a lid face
    // that is lit from this angle rather than in shadow (§2b point 21).
    m.cylinder_z((5, 10), 1.4, (14, 14), p.idx("brass"));
    m
}

/// The mustard branch's two raw ingredients — bulk **mustard seed** in an
/// open tote and a **hand of turmeric** (the third-sauce epic's art ticket).
///
/// **The row is still the frame that has to work**, and it is now eight raws
/// wide: a red sphere (tomato), two paper sacks (sugar, spice blend), an
/// amber jug (vinegar), a capped jar (molasses), a straight drum (liquid
/// smoke) and then these two. Adding to a row is harder than starting one,
/// because every silhouette worth having is taken, so each of these takes
/// the one shape the other seven do not have. The seed goes in a **wide,
/// low, OPEN, rectangular** vessel with its contents heaped through the
/// mouth — nothing else in the row is open, and nothing else is rectangular.
/// The turmeric is **not a container at all**: it is the ingredient itself,
/// knobbly and lying nearly flat, which makes it the row's one irregular
/// outline and its answer to the tomato (the row's other bare ingredient,
/// and a perfect sphere).
///
/// The open tote deliberately reoccupies the shape the retired spice TIN
/// used to hold — that became a sack on the project owner's 2026-09-21 call,
/// which left the slot vacant while the argument for it stayed true. An open
/// vessel is the only way a dry raw's own colour reaches the surface BOTH
/// cameras look down on (§2b point 11 for the game's, point 35 for vengi's),
/// and bulk seed is bought by the tonne and handled in exactly this.
///
/// `seed_tan` is measured against the steel it sits in rather than against
/// the row (31.0/255 under `steel`, 72.2 under `steel_light`), because on an
/// open tote the fill and the vessel are the only two colours there are;
/// `palette.ron`'s entry carries the arithmetic. The rim is `steel_light` so
/// the vessel's outline holds the brightest pixel (UI_UX.md §4) and the heap
/// is read against it.
pub fn item_mustard_seed(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    let steel = p.idx("steel");
    // Walls and base solid, then hollowed — `pail`'s order, for `pail`'s
    // reason: a vessel filled to its own brim is a block with a lid on it.
    m.cuboid((2, 13), (3, 12), (0, 5), steel);
    m.carve_cuboid((3, 12), (4, 11), (1, 5));
    // The rolled rim: a RING, drawn wide and re-opened, so the tote stays
    // open from above where both cameras can see into it.
    m.cuboid((2, 13), (3, 12), (6, 6), p.idx("steel_light"));
    m.carve_cuboid((3, 12), (4, 11), (6, 6));
    // The seed: the cavity filled, then heaped two voxels proud of the rim.
    // The heap has to BREAK the outline — a tote filled flush reads as an
    // empty tote, which is what the old spice tin's note records.
    let seed = p.idx("seed_tan");
    m.cuboid((3, 12), (4, 11), (1, 5), seed);
    m.ellipsoid((8, 8, 5), (4.4, 3.4, 4.0), seed);
    // A few seeds spilled on the floor beside it — the pairing
    // `item_sugar`'s crystal makes, since a vessel alone is a vessel. They
    // go at the DIAGONAL corners, clear of the walls in both axes, because
    // the first two passes put them alongside a wall and the render came
    // back with four little feet under the tote: a lone voxel touching a
    // silhouette joins it. Fixed positions, never random — a review
    // screenshot has to repeat.
    for (x, y) in [(0, 1), (1, 0), (15, 14), (14, 15)] {
        m.set(x, y, 0, seed);
    }
    m
}

/// One hand of turmeric: a knobbly rhizome with fingers off it, a snapped
/// stub standing up out of the knob, and a dusting of ground root on the
/// floor.
///
/// **The bright colour is on top and on the +X end, and nowhere else.** The
/// skin of a turmeric root is dull brown; what says "turmeric" is the cut
/// face, so the model is `cardboard_dark` almost everywhere with
/// `turmeric_orange` only where the root is broken. Both cameras look down
/// (§2b points 11 and 35), so the stub's snapped end points UP — the one
/// surface each of them gets — and the finger's snapped end points +X, the
/// face the game's fixed isometric sees. That is the same answer
/// `item_spice_mix` reached with its spill at the mouth, and for the same
/// reason: an identifying mark on a side face this camera shades is a mark
/// the player never gets.
///
/// **A small organic shape beats a flat panel here**, which is why the root
/// is not simply painted yellow. `turmeric_orange` cannot clear the
/// reads-reliably threshold from `hazard_yellow` in luma — the gap between
/// `vinegar_amber` and `hazard_yellow` is only 48.1/255 wide, so nothing
/// inside it clears 26 from both ends, and `palette.ron`'s entry works the
/// arithmetic through. It separates by HUE (17.5 degrees, dE76 22.6 from
/// hazard yellow) and by SHAPE instead: a cut face on a knobbly root 18px
/// across is not signage whatever its hue, because signage in this game is a
/// large flat straight-edged panel on a machine. **Not by area** — the cut is
/// about a sixth of the root by volume, but the repaint below puts it on
/// exactly the faces this camera sees, so a play-zoom frame shows far more of
/// it than that ratio suggests; the argument has to survive that, and it
/// does. The dark skin also buys the cut face 42.6/255 of local contrast,
/// which is more than any yellow-on-yellow arrangement could.
///
/// Ellipsoids and short cylinders rather than one straight shape, because
/// straightness is what identifies `item_liquid_smoke`'s drum in the same
/// row, and a root that reads as machined reads as a fitting.
pub fn item_turmeric(p: &Palette) -> VoxModel {
    let mut m = item_volume();
    let skin = p.idx("cardboard_dark");
    // A hand of rhizomes: a knuckle with two fat fingers off it at
    // different angles and a lobe at the far end, all lying low.
    // Overlapping ellipsoids rather than one shape, so nothing about the
    // outline is straight or symmetric — straightness is what identifies
    // `item_liquid_smoke`'s drum in the same row, and a root that reads as
    // machined reads as a fitting.
    m.ellipsoid((6, 7, 2), (2.4, 2.4, 2.4), skin);
    m.ellipsoid((9, 6, 2), (4.0, 1.9, 1.9), skin);
    m.ellipsoid((5, 10, 2), (1.9, 3.4, 1.9), skin);
    m.ellipsoid((11, 8, 1), (2.0, 1.8, 1.6), skin);
    // The snapped stub, standing out of the knuckle — the tallest thing on
    // the root and the part that is visible whatever else overlaps it.
    m.cylinder_z((6, 7), 1.7, (3, 6), skin);
    let flesh = p.idx("turmeric_orange");
    // The CUTS. The stub's top face is a disc of exactly the stub's radius,
    // so it is the broken end of that stub and not a plate resting on it,
    // and it faces the one direction both cameras look from (§2b points 11
    // and 35).
    m.cylinder_z((6, 7), 1.7, (6, 6), flesh);
    // The second cut is the +X-facing skin of the long finger and the lobe,
    // repainted over the form the root already has rather than drawn as
    // another primitive — `item_vinegar`'s fill line does the same, and for
    // the same reason: a bright end cap drawn with a primitive sticks out
    // past the shape it is supposed to be the end of. +X is the face the
    // game's fixed isometric camera sees (point 11), so a belt gets a cut
    // root and the market gets the stub.
    for z in 0..ITEM_VOXELS as i32 {
        for y in 0..ITEM_VOXELS as i32 {
            for x in 11..ITEM_VOXELS as i32 {
                if m.filled(x, y, z) && !m.filled(x + 1, y, z) {
                    m.set(x, y, z, flesh);
                }
            }
        }
    }
    // Ground root dusted on the floor, `item_spice_mix`'s pairing again,
    // and clear of the root for `item_mustard_seed`'s reason.
    for (x, y) in [(2, 13), (13, 3), (9, 14)] {
        m.set(x, y, 0, flesh);
    }
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

/// Ground mustard seed: the crusher's mustard output, riding the belt to the
/// mixer since mustard stopped being milled wet (2026-10-03). The paste
/// pail's silhouette, because it is the same crusher handing on the same
/// kind of bulk intermediate — and a DRY fill, `mustard_gold` in the `matte`
/// class, because it is flour, not sauce (`palette.ron`'s `bulk_pale` rule;
/// `a_pail_s_contents_take_the_class_of_the_substance` pins it). Gold in a
/// grey pail is told apart from red paste in the same pail by hue and by
/// value at once.
pub fn item_ground_mustard(p: &Palette) -> VoxModel {
    pail(p, "pail_grey", "mustard_gold", "brass")
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
/// it is a cavity at all.
///
/// **`contents` names the class as well as the colour, and both readings are
/// deliberate.** All three callers above pass a `liquid`-class colour
/// because the gloss is what says a pail of paste is wet (see their own
/// note) — but that is a fact about those three substances, not about this
/// helper. A dry solid in a pail would have to pass a `matte` colour, since
/// the same gloss on a flour would read as sauce in a bin (`palette.ron`'s
/// `bulk_pale` records that rule). So this helper does not police the class
/// — `a_pail_s_contents_take_the_class_of_the_substance` does, per caller,
/// which is the only place the answer can be right.
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

/// The shipped ketchup bottle. Bright `ketchup_red` behind glass, cream
/// label — the colour the whole chain has been working toward since the
/// tomato.
pub fn item_bottle(p: &Palette) -> VoxModel {
    bottle(p, "ketchup_red", "label_cream", "cap_white")
}

/// The BBQ bottle (the second-sauce epic, `docs/BBQ_SAUCE_PLAN.md`).
///
/// Same vessel, different sauce: `bottled_bbq` is a separate item made on the
/// same bottler, so it has to read as the same bottle carrying something
/// else rather than as a different object. The contents carry the whole
/// distinction and they carry it by VALUE, not hue — `bbq_brown` is about a
/// third of `ketchup_red`'s luminance — because at the size an item renders
/// on a belt (half a tile, ~16px) two warm reds a shade apart are not a
/// difference a player can act on, and this camera sees a bottle's shadow
/// side (§2b point 13). Dark body, bright cap: the silhouette stays a
/// bottle, the body reads as filled with something almost black.
///
/// The label goes kraft rather than cream for the same reason at one step
/// down: a band that is only slightly darker than the cap would read as the
/// same label in different light, while kraft against the dark body is
/// plainly a different livery.
pub fn item_bottle_bbq(p: &Palette) -> VoxModel {
    bottle(p, "bbq_brown", "label_kraft", "cap_white")
}

/// The mustard bottle (the third-sauce epic).
///
/// The same vessel a third time — one bottler makes all three sauces, so
/// this has to read as that bottle carrying something else — and the
/// distinction is `item_bottle_bbq`'s VALUE argument run the other way.
/// `bbq_brown` went far darker than `ketchup_red`; `mustard_gold` goes far
/// brighter, and is in fact the brightest body colour in the game. The three
/// bottles therefore spread across the whole value range (mustard sits
/// 135.6/255 of luma above ketchup and 148.6 above BBQ, with ketchup between
/// them) rather than crowding one end of it, which is what keeps three of
/// them tellable apart at the ~16px an item renders at on a belt.
///
/// **Its cap and label band are white — the project owner's ruling.** Mustard
/// is yellow and white, not yellow and black (2026-10-02, DECISIONS.md); the
/// band and cap were a near-black olive ink until then. That ink answered a
/// real problem, and the white answers it too, from the bright side: a
/// bright body under the ordinary `cap_white` loses the top of its
/// silhouette, because `mustard_gold` is 5.0/255 from `glass_pale`, the neck
/// every bottle shares, so the three-voxel neck and two-voxel cap — one
/// five-pixel mark at item size — come to 213.9 against a 192.7 body, 21.2
/// apart and under the 26 that reads. `label_mustard` is therefore a
/// brighter white than `cap_white` (luma 241.0), which lifts that mark to
/// about 224, 31.3 over the body, and puts the band 48.3 over it.
/// `a_bottle_s_cap_contrasts_with_the_neck_and_the_body` measures the top;
/// `the_mustard_bottle_is_yellow_and_white` holds the ruling.
pub fn item_bottle_mustard(p: &Palette) -> VoxModel {
    bottle(p, "mustard_gold", "label_mustard", "label_mustard")
}

/// A bottle of finished sauce: a body of `contents` with a `label` band
/// around it, a narrower glass neck and a `cap`.
///
/// Parameterised over its colours rather than copied per sauce (the
/// second-sauce epic): the shoulder ellipsoid has to be the same colour as
/// the body and sit exactly where the cylinder ends or the neck reads as a
/// jar's, which is precisely the relationship a second hand-written
/// generator drifts on.
///
/// **The cap became a parameter for the third sauce**, and not for variety's
/// sake. Both earlier bottles are a dark body under a bright `cap_white`, so
/// the cap tops the silhouette by contrast; `mustard_gold` is the brightest
/// body in the game and sits 5.0/255 of luma from `glass_pale`, the neck
/// every bottle shares, so on a mustard bottle the neck and a `cap_white`
/// cap together come to 213.9 against a 192.7 body — 21.2 apart, under the
/// 26 that reads — and body, neck and cap are one pale column. (Neither pair
/// on its own says so: that cap is 26.6 from the neck and 31.6 from the
/// body. At the ~16px an item draws on a belt the neck is three voxels and
/// the cap two, so the top is one mark and its combined value is what the
/// eye gets.) Mustard's cap is a brighter white instead, `label_mustard`,
/// which clears the combined figure from above; it was a dark ink until the
/// owner ruled mustard yellow and white (2026-10-02).
/// `a_bottle_s_cap_contrasts_with_the_neck_and_the_body` asserts the top
/// either way up.
fn bottle(p: &Palette, contents: &str, label: &str, cap: &str) -> VoxModel {
    let mut m = item_volume();
    m.cylinder_z((8, 8), 4.0, (0, 7), p.idx(contents));
    m.cylinder_z((8, 8), 4.2, (2, 4), p.idx(label));
    // Shoulder tapers into a distinctly narrower neck — without the gap the
    // cap sits straight on the body and the silhouette reads as a jar.
    m.ellipsoid((8, 8, 8), (4.0, 4.0, 2.0), p.idx(contents));
    m.cylinder_z((8, 8), 1.6, (10, 12), p.idx("glass_pale"));
    m.cylinder_z((8, 8), 2.4, (13, 14), p.idx(cap));
    m
}

/// The shipped case of ketchup, the item the dock ships and every contract
/// on the board asks for.
pub fn item_box(p: &Palette) -> VoxModel {
    box_of_sauce(p, "label_cream")
}

/// The BBQ case (the second-sauce epic).
///
/// **The carton does not change — the label does.** A case is cardboard
/// because it is a cardboard box, and a second sauce shipping in mahogany
/// cartons would say the packaging is different rather than that the
/// contents are; the project owner asked for the label to carry it. So the
/// print swaps cream for `label_bbq`, the sauce's own dark brown, and that
/// is the entire difference.
///
/// Dark patches rather than another pale colour, deliberately. A box is the
/// smallest thing on the belt, and against `cardboard` a second light label
/// would be a hue shift of a few points at ~16px — the two cases would be
/// the same object. Reversing the CONTRAST DIRECTION (bright patches on a
/// mid carton, or dark ones) survives any size the box renders at, and it
/// is what a printed label on kraft actually looks like. §2b point 21's
/// warning about dark accents reading as holes is about a dark mark on a
/// machine's shadow-side FACE; these sit on a small bright object with a
/// `cardboard_dark` tape seam already crossing the same face, and the
/// rendered pair was checked at real size before this was kept.
///
/// **Where that print goes is the whole of issue #40**, and the answer is
/// the lid rather than a bigger label — see [`box_of_sauce`].
pub fn item_box_bbq(p: &Palette) -> VoxModel {
    box_of_sauce(p, "label_bbq")
}

/// The mustard case (the third-sauce epic), and the one asset in this set
/// whose distinction is NOT carried by value.
///
/// The carton does not change: the project owner's ruling on the BBQ case
/// holds for every sauce, so all three cases are `cardboard` with print on
/// them. That leaves print, and print has only two ends of the value range —
/// ketchup took the bright one (`label_cream`, 83/255 over the carton) and
/// BBQ the dark one (`label_bbq`, 88 under it). A third ink can only land
/// between them, where it would read as an unprinted box, so mustard's ink
/// is the sauce's own `mustard_gold` and the channel carrying it is CHROMA:
/// 27.6/255 of luma from `label_cream` but 55.8 of CIELAB dC, against the
/// 4.0 same-carton dC noise floor DECISIONS.md's 2026-09-20 record measured.
///
/// **That record found chroma the weaker channel, and it was right about the
/// pair it tested.** A dark red ink on warm kraft barely moves chroma at
/// all, so cream-versus-BBQ had to be a value difference. But the channel it
/// left empty is exactly the one a saturated yellow fills, so the three
/// cases end up separated bright / dark / SATURATED and no case needs a
/// third value. The PR that added this measured all three pairs on one
/// frame; mustard-versus-ketchup is the weakest of the three in luma and the
/// strongest in chroma, and that trade-off is stated rather than hidden.
///
/// Bright ink rather than dark for one more reason: a dark mark on kraft can
/// read as a hole (§2b point 21), and the BBQ case gets away with it only
/// because a `cardboard_dark` tape seam already crosses the same face. A
/// bright saturated patch on kraft can only read as print.
pub fn item_box_mustard(p: &Palette) -> VoxModel {
    box_of_sauce(p, "mustard_gold")
}

/// A taped shipping case, printed on both of its long faces and on its lid.
///
/// `label` is the only thing a caller chooses: everything else about a case
/// is the same case (the second-sauce epic).
///
/// **The LID print is what makes the two sauces' cases tellable apart at
/// whole-factory zoom** (issue #40). The two side labels alone did not: at
/// the framing a player actually plays at a case is about 30px and a label
/// patch about 5x6px of it, on the camera's SHADOW-side face, and the
/// rendered pair measured 1.2/255 apart in mean brightness — i.e. nothing.
/// The lid is the same print on the face this camera lights and sees most
/// of, so the same two colours buy about twenty times the separation
/// (26.5/255 on the same frame) for no new colour, no new voxel and no
/// change to the silhouette: every voxel this paints was already carton.
/// That ratio, not the patch size, is the lesson — at this camera a mark on
/// the lit top face is worth several of the same mark on a side face, which
/// is §2b point 21 read forwards rather than as a warning.
///
/// Print on the two SHORT faces as well was the next cut and was rejected:
/// it measured better still (43.2/255) and at closeup the kraft had stopped
/// being the thing you see, so the case read as coloured packaging rather
/// than as a printed box — the one reading the project owner ruled out. The
/// lid panels stop two voxels short of the carton's edges and leave the tape
/// seam crossing between them for that reason: the box has to stay a
/// cardboard box with print on it.
///
/// **The `y = 13` labels are new, and they are for the ICON.** The `y = 2`
/// pair is what the fixed isometric camera sees (§2b point 11); the icon
/// pass renders the same model from the OPPOSITE corner (§2b point 35), so
/// until this a case's icon was a blank carton — and with two sauces
/// shipping in cases, `boxed_ketchup` and `boxed_bbq` had the same blank
/// carton on the contract clipboard, in the larder and in the production
/// panel, which is UI_UX.md §2b point 12's silent failure with the two most
/// valuable items in the game. A second pair of labels is also just what a
/// shipping case looks like. It changes `item_box`, deliberately: the face
/// it is printed on is one the game camera cannot see, so no rendered frame
/// of the factory moves, and the first-person view gains a box that is
/// labelled from both sides rather than one that is labelled from one.
fn box_of_sauce(p: &Palette, label: &str) -> VoxModel {
    let mut m = item_volume();
    m.cuboid((2, 13), (2, 13), (0, 10), p.idx("cardboard"));
    // Taped seam over the top and down the front.
    m.cuboid((7, 8), (2, 13), (10, 10), p.idx("cardboard_dark"));
    m.cuboid((7, 8), (2, 2), (0, 10), p.idx("cardboard_dark"));
    // Shipping labels: the camera's face, then the icon's.
    for y in [2, 13] {
        m.cuboid((3, 6), (y, y), (4, 8), p.idx(label));
        m.cuboid((9, 12), (y, y), (4, 8), p.idx(label));
    }
    // The LID print, one panel either side of the tape (issue #40).
    m.cuboid((3, 6), (4, 11), (10, 10), p.idx(label));
    m.cuboid((9, 12), (4, 11), (10, 10), p.idx(label));
    m
}

// ---------------------------------------------------------------------------
// Payout feedback
// ---------------------------------------------------------------------------

/// The coin a paid contract throws off the dock ("money feels like money").
///
/// Small on purpose — half an item, a quarter of a tile — because a dozen of
/// these arrive at once and UI_UX.md §6 asks juice to stay legible over
/// spectacle: a burst of tile-sized coins would hide the factory it is
/// celebrating.
///
/// Two material classes rather than one, in three concentric rings, and both
/// classes are doing a job. The `brass` ring (class `metal`) is UI_UX.md
/// §4's deliberate shiny exception and is what makes a coin catch the light
/// as it tumbles; the lit outer rim and centre pip are `status_amber`
/// (class `emissive`), for UI_UX.md §2b point 13's reason — both faces this
/// camera can see are the shadow side, so anything that has to read as a
/// *signal* rather than as scenery has to emit, and a coin means a contract
/// just paid. Rings rather than a lit face over a brass rim: at ten voxels
/// a solid inner disc renders as a square (checked in a preview, twice),
/// and it is the ring that says "coin" anyway.
///
/// The disc's axis is the vox **Y** axis, which the importer maps to world
/// **+Z** — one of the two directions this camera can see a face from
/// (§2b point 11) — so a coin at rest shows its face and a coin spun about
/// world Y flashes edge-on and back. Its centre sits on the volume midline
/// (`COIN_C`, between voxels 3 and 4) for §2b point 20's reason: `main.gd`
/// spins these, and an integer-centred axis wobbles off the pivot instead of
/// turning about it.
pub fn coin(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([COIN_VOXELS, COIN_VOXELS, COIN_VOXELS]);
    m.cylinder_y_f((COIN_C, COIN_C), 4.4, (4, 5), p.idx("status_amber"));
    m.cylinder_y_f((COIN_C, COIN_C), 3.2, (4, 5), p.idx("brass"));
    m.cylinder_y_f((COIN_C, COIN_C), 1.6, (4, 5), p.idx("status_amber"));
    m
}

/// A coin's volume, in voxels: under a third of a tile, well under
/// [`ITEM_VOXELS`]. Ten rather than eight because eight is where a voxel
/// disc stops reading as round and starts reading as a square with the
/// corners knocked off — checked in a preview render, not assumed.
const COIN_VOXELS: u32 = 10;

/// The midline of that volume — between voxels 4 and 5 (UI_UX.md §2b point
/// 20). Not 5.
const COIN_C: f32 = (COIN_VOXELS as f32 - 1.0) / 2.0;

/// A fleck of crushed tomato — what the crusher spits while it is actually
/// working (the crusher-particles change, 2026-09-16). Like [`coin`] it is
/// an asset that never occupies a tile: `main.gd` keeps a pool of these and
/// throws them out of the crusher's feed pocket on a beat.
///
/// **A lump, not a ball.** Three overlapping spheres with their centres off
/// the volume's midline in different directions, so the silhouette has a
/// waist and a corner and reads as a piece torn off something rather than as
/// a bead. That matters more here than any surface detail does: at a quarter
/// of a tile, thrown, and gone in under a second, the outline is the whole
/// of what a player sees (UI_UX.md §2b point 31's rule, one scale down).
///
/// **Dark underneath, light on top,** the same reason [`item_tomato`] is
/// shaded: this camera lights the tops of things, so a fleck with one flat
/// albedo reads as a paper cut-out. Nothing brighter than the red is on it:
/// the first cut put two pale seed voxels on the crown and the preview read
/// them as a stalk — i.e. as a whole tomato, the one thing this is not.
///
/// The lump straddles the volume midline (`CRUSH_BIT_C`) because `main.gd`
/// spins these about world Y as they fly, exactly as it spins a coin, and an
/// off-centre mass orbits the pivot instead of turning on it (point 20).
pub fn crush_bit(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([CRUSH_BIT_VOXELS, CRUSH_BIT_VOXELS, CRUSH_BIT_VOXELS]);
    let c = CRUSH_BIT_C;
    m.sphere_f((c - 0.8, c + 0.2, c + 0.6), 2.1, p.idx("tomato_red"));
    m.sphere_f((c + 1.0, c - 0.6, c - 0.2), 1.6, p.idx("tomato_red"));
    // The wet, torn underside, cast after the body so it wins the voxels
    // both claim.
    m.sphere_f((c, c, c - 1.4), 1.9, p.idx("tomato_shadow"));
    // One torn corner: a single voxel proud of the lump on the lit top face,
    // so the outline has a point in it. Kept to one voxel because a cuboid
    // here (the second cut of this) rendered as a slab hanging off the side
    // and read as two objects rather than one torn one.
    let i = c.round() as i32;
    m.set(i + 2, i - 1, i + 1, p.idx("tomato_red"));
    m
}

/// The fleck's volume, in voxels: a quarter of a tile, smaller than an item
/// (half a tile) and smaller than a coin, because several are in the air at
/// once over a machine that is itself only one tile.
const CRUSH_BIT_VOXELS: u32 = 8;

/// Its midline — between voxels 3 and 4 (UI_UX.md §2b point 20). Not 4.
const CRUSH_BIT_C: f32 = (CRUSH_BIT_VOXELS as f32 - 1.0) / 2.0;

// ---------------------------------------------------------------------------
// Staff category icons (the staff-pools epic, ticket 2 —
// docs/STAFF_POOLS_PLAN.md, "The icons")
// ---------------------------------------------------------------------------
//
// Four UI props, one per `data/staff.ron` category, so the staff panel
// (ticket 3) can put a picture beside each name: a hard hat for
// manufacturing, an open-ended spanner for engineering, a taped carton with
// its roll of tape for packaging, a pallet jack for logistics. Each is the
// tool the category holds, not a person — GAMEPLAY.md §7 is explicit that
// staff are a capacity, not NPCs, and a figure would promise a person who
// is not in the simulation. UI_UX.md §5 wants every icon rendered from a
// real voxel asset, never drawn flat, so these are voxel models like the
// coin: registered in `ASSETS`, given no footprint, never placed.
//
// **Authored for vengi's camera, not the game's** (UI_UX.md §2b point 35,
// the vinegar jug's precedent): the icon pass is the only thing that ever
// looks at these, and `vengi-thumbnailer` looks from the **-X/+Y** corner,
// so a face meant to be seen goes on x = 0, on y = 31, or on top. Nothing
// here is symmetric about a tile midline on purpose — nothing spins these.
//
// **Silhouette first, at 64 px, against a dark plate.** A staff icon is
// read at a glance in a row of four; it has to be tellable from its
// neighbours by outline alone. So each object is the *whole* object and
// lies the way its outline is widest to an isometric camera — the spanner
// flat, the carton square-on, the jack forks-forward — and detail is spent
// only where it changes the outline (the hat's brim and crest, the jaw
// slots, the roll on the lid, the tiller).
//
// **Colour is the plan's** (UI_UX.md §7): the hard hat and the tape are
// `hazard_yellow`, the one signage colour, which is also what a hard hat and
// a roll of packing tape actually are; the spanner and the jack are `metal`
// so they catch the light the way every other tool in this factory does
// (§4); the carton is `cardboard`, matte, like `item_box`. Nothing new was
// added to `palette.ron` — every colour needed already had a job here.

/// **Manufacturing: a hard hat.** A domed crown on a brim, with a peak at
/// the front and a raised crest over the top. The crest and the brim are
/// what stop it being a yellow egg at 64 px: a plain dome from above is a
/// disc, and a disc says "coin" in this UI. The peak points at the camera
/// (-X), so the brim reads as a brim and not as a flat rim all round.
///
/// The crown is a low elliptical dome, wider than tall (see `HAT_H` for the
/// bell it was before). One colour, no stripe or badge: a hard hat is a
/// hard hat by shape.
pub fn staff_manufacturing(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let yellow = p.idx("hazard_yellow");
    // The brim: a flat disc with a longer peak on the camera-facing -X side.
    m.cylinder_z_f(HAT_C, HAT_BRIM_R, (0, 1), yellow);
    m.cylinder_z_f(
        (HAT_C.0 - HAT_PEAK, HAT_C.1),
        HAT_BRIM_R - 1.5,
        (0, 1),
        yellow,
    );
    // The crown, sliced: an elliptical profile, wider than it is tall, with
    // one crest running front to back a voxel proud of the shell.
    for z in HAT_BASE_Z..=HAT_BASE_Z + HAT_H {
        let t = (z - HAT_BASE_Z) as f32 / HAT_H as f32;
        let f = (1.0 - t * t).max(0.0).sqrt();
        m.cylinder_z_f(HAT_C, HAT_R * f, (z, z), yellow);
        let rc = (HAT_R + 1.0) * f;
        let lo = (HAT_C.0 - rc).round() as i32;
        let hi = (HAT_C.0 + rc).round() as i32;
        m.cuboid((lo, hi), (14, 17), (z, z), yellow);
    }
    m
}

/// The hard hat's centre on the tile plane (between voxels 15 and 16, so
/// the crest is two voxels either side of it).
const HAT_C: (f32, f32) = (15.5, 15.5);
const HAT_BRIM_R: f32 = 14.5;
/// How far the peak's disc is pushed toward -X past the crown's centre.
const HAT_PEAK: f32 = 5.0;
const HAT_R: f32 = 12.0;
const HAT_BASE_Z: i32 = 2;
/// The crown's height above the brim. **Wider than tall, by a lot**: the
/// first cut stood 14 high on a radius of 11 with a squared-off
/// (superellipse) profile, and its icon was a bell — a jelly mould with
/// ribs. A hard hat's crown is a low dome; at 64 px it is the proportion
/// of crown to brim that says "hat", not the ribbing.
const HAT_H: i32 = 9;

/// **Engineering: an open-ended spanner.** A double-ender lying flat, the
/// classic tool-icon silhouette: a straight shaft with a round head and an
/// open jaw at each end. Flat, not standing, because from above the jaws
/// are what say "spanner", and on edge it is a bar. The two jaws are offset
/// to opposite sides, the way a real double open-ender's are angled, which
/// also breaks the mirror symmetry that made the first cut read as a
/// dumbbell.
///
/// Stainless (`steel_light`, class `metal`) with a `steel` stamp band down
/// the shaft's top — the one darker line a forged spanner's I-section
/// gives it.
pub fn staff_engineering(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let light = p.idx("steel_light");
    let steel = p.idx("steel");
    let z = (0, SPANNER_THICK - 1);
    m.cuboid((8, 23), (12, 19), z, light);
    m.cylinder_z_f((7.5, 15.5), SPANNER_HEAD_R, z, light);
    m.cylinder_z_f((24.5, 15.5), SPANNER_HEAD_R, z, light);
    // The jaws: a slot cut in from each end, each offset a voxel to its own
    // side. Shallower than the head is long, so both lips survive to the
    // mouth and the outline is a C, not a fork.
    m.carve_cuboid((0, 5), (12, 16), z);
    m.carve_cuboid((26, 31), (15, 19), z);
    // The stamp band along the shaft's top face.
    m.cuboid((10, 21), (14, 17), (z.1, z.1), steel);
    m
}

const SPANNER_THICK: i32 = 4;
const SPANNER_HEAD_R: f32 = 7.5;

/// **Packaging: a taped carton with the roll of tape on its lid.** The
/// carton is `item_box`'s cardboard at twice the scale, its lid seam sealed
/// with a strip of `hazard_yellow` tape that turns down the +Y face (the one
/// vengi sees), and the roll lies flat on the lid at the camera's corner:
/// a ring of tape on a cardboard core with a hole through it. The roll is
/// the whole reason the icon is not just a box: a box alone is `item_box`,
/// which is the *product*, and this icon has to mean the people who seal
/// it. Yellow tape rather than the box's brown, for the same reason.
pub fn staff_packaging(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let card = p.idx("cardboard");
    let dark = p.idx("cardboard_dark");
    let tape = p.idx("hazard_yellow");
    let (b0, b1) = CARTON;
    m.cuboid(CARTON, CARTON, (0, CARTON_TOP), card);
    // The lid's flap seam, and the fold lines where the flaps meet the sides.
    m.cuboid((15, 16), CARTON, (CARTON_TOP, CARTON_TOP), dark);
    m.cuboid(CARTON, (b0, b0), (CARTON_TOP, CARTON_TOP), dark);
    m.cuboid(CARTON, (b1, b1), (CARTON_TOP, CARTON_TOP), dark);
    // The sealing tape over the seam, turned down both ends.
    m.cuboid((13, 18), CARTON, (CARTON_TOP, CARTON_TOP), tape);
    m.cuboid((13, 18), (b1, b1), (CARTON_TOP - 8, CARTON_TOP), tape);
    m.cuboid((13, 18), (b0, b0), (CARTON_TOP - 8, CARTON_TOP), tape);
    // A shipping label on the -X face, which vengi also sees.
    m.cuboid((b0, b0), (7, 13), (5, 11), p.idx("label_cream"));
    // The roll, lying on the lid at the camera's corner.
    let roll_z = (CARTON_TOP + 1, CARTON_TOP + ROLL_HEIGHT);
    m.cylinder_z_f(ROLL_C, ROLL_R, roll_z, tape);
    m.cylinder_z_f(ROLL_C, ROLL_CORE_R, roll_z, card);
    m.carve_cylinder_z_f(ROLL_C, ROLL_HOLE_R, roll_z);
    m
}

const CARTON: (i32, i32) = (3, 28);
const CARTON_TOP: i32 = 17;
const ROLL_C: (f32, f32) = (9.5, 21.5);
const ROLL_R: f32 = 6.5;
const ROLL_CORE_R: f32 = 3.4;
const ROLL_HOLE_R: f32 = 2.2;
const ROLL_HEIGHT: i32 = 4;

/// **Logistics: a pallet jack.** Two forks, a pump housing on its steer
/// wheels, and a tiller with a T-handle. Forks toward the camera (-X) so
/// they are the widest thing in the outline and nothing stands in front of
/// them; the housing and the tiller rise at the far end, where a tall part
/// hides nothing (UI_UX.md §2b point 16's depth rule, applied to vengi's
/// camera). The tiller leans back over the housing rather than standing
/// plumb — a vertical post over two bars reads as a lamppost on a sledge.
///
/// Forks and tiller `steel_light`, housing `steel_dark`, all class `metal`;
/// the four wheels and the two grips are `rubber_dark`, because a jack's
/// wheels are the one part of it that is not steel and the dark accents are
/// what make the forks read as *off the ground*.
pub fn staff_logistics(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let light = p.idx("steel_light");
    let steel = p.idx("steel");
    let dark = p.idx("steel_dark");
    let rubber = p.idx("rubber_dark");
    // Load wheels first, so a fork drawn over one keeps its own colour.
    for &(y0, y1) in &FORK_Y {
        let yc = (y0 + y1) as f32 / 2.0;
        m.cylinder_x_f((yc, 1.5), 1.6, (4, 7), rubber);
        m.cuboid((2, FORK_X1), (y0, y1), (2, 5), light);
        // A tapered tip.
        m.cuboid((0, 1), (y0 + 1, y1 - 1), (2, 4), light);
    }
    // The carriage that joins the forks, and the pump housing behind it.
    m.cuboid(
        (FORK_X1 - 2, FORK_X1 + 2),
        (FORK_Y[0].0, FORK_Y[1].1),
        (2, 7),
        steel,
    );
    m.cuboid((FORK_X1 + 1, FORK_X1 + 5), (11, 20), (0, 12), dark);
    // Steer wheels either side of the housing.
    m.cylinder_y_f((FORK_X1 as f32 + 3.5, 2.5), 2.5, (8, 10), rubber);
    m.cylinder_y_f((FORK_X1 as f32 + 3.5, 2.5), 2.5, (21, 23), rubber);
    // The tiller: a 2x2 arm leaning back from the housing top to the handle.
    for k in 0..TILLER_RISE {
        let x = FORK_X1 + 3 + (k * TILLER_LEAN) / TILLER_RISE;
        m.cuboid((x, x + 1), (14, 17), (13 + k, 13 + k), light);
    }
    let top = 13 + TILLER_RISE;
    let hx = FORK_X1 + 2 + TILLER_LEAN;
    m.cuboid((hx, hx + 2), (6, 25), (top, top + 2), light);
    m.cuboid((hx, hx + 2), (6, 8), (top, top + 2), rubber);
    m.cuboid((hx, hx + 2), (23, 25), (top, top + 2), rubber);
    m
}

/// The two forks' Y spans; the tips point at -X.
const FORK_Y: [(i32, i32); 2] = [(5, 10), (21, 26)];
/// Where the forks end and the carriage begins.
const FORK_X1: i32 = 23;
const TILLER_RISE: i32 = 12;
/// How far (+X) the tiller's top sits behind its foot: the lean.
const TILLER_LEAN: i32 = 3;

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
/// Kept beside `PIPE_FITTING_X0` because the pair is the measurement, and
/// reading one without the other is how the sight line above gets
/// mis-remembered. Nothing draws with it today.
#[allow(
    dead_code,
    reason = "the other half of a measurement, kept for the doc above"
)]
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
    m.cylinder_x_f(
        (PIPE_C, PIPE_AXIS_Z as f32),
        PIPE_OUTER_R,
        x,
        p.idx("steel_light"),
    );
}

/// The +Y twin of [`pipe_leg_x`].
fn pipe_leg_y(m: &mut VoxModel, p: &Palette, y: (i32, i32)) {
    m.cylinder_y_f(
        (PIPE_C, PIPE_AXIS_Z as f32),
        PIPE_OUTER_R,
        y,
        p.idx("steel_light"),
    );
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
    m.cylinder_x_f(
        (PIPE_C, PIPE_AXIS_Z as f32),
        PIPE_OUTER_R + 1.0,
        (0, 1),
        p.idx("steel"),
    );
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
    m.cylinder_x_f(
        (PIPE_C, PIPE_AXIS_Z as f32),
        PIPE_OUTER_R + 1.0,
        (last - 1, last),
        p.idx("steel_light"),
    );
    for (y, z) in [
        (15, PIPE_AXIS_Z + 4),
        (16, PIPE_AXIS_Z + 4),
        (15, PIPE_AXIS_Z - 4),
        (16, PIPE_AXIS_Z - 4),
        (11, PIPE_AXIS_Z),
        (20, PIPE_AXIS_Z),
    ] {
        m.set(last, y, z, p.idx("brass"));
    }
    m
}

/// The junction shapes' shared centre: the cast hub, hollowed with the
/// legs. Each shape draws its legs solid first, calls this, then carves its
/// leg bores — the hub's cavity joins them.
fn pipe_hub(m: &mut VoxModel, p: &Palette) {
    m.sphere_f(
        (PIPE_C, PIPE_C, PIPE_AXIS_Z as f32),
        PIPE_HUB_R,
        p.idx("steel_light"),
    );
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
    m.cylinder_x_f(
        axis,
        PIPE_OUTER_R + PIPE_FITTING_MARGIN,
        (PIPE_FITTING_X0, last - 2),
        p.idx("steel_light"),
    );
    m.cylinder_x_f(
        axis,
        PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN,
        (last - 1, last),
        p.idx("brass"),
    );
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
    m.cylinder_x_f(
        axis,
        PIPE_OUTER_R + 0.2 + PIPE_FITTING_MARGIN,
        (PIPE_FITTING_X0, PIPE_FITTING_X0 + 1),
        p.idx("brass"),
    );
    m.cylinder_x_f(
        axis,
        PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN,
        (PIPE_FITTING_X0 + 2, last),
        p.idx("steel_light"),
    );
    m.carve_cylinder_x_f(axis, PIPE_BORE_R + 0.4, (PIPE_FITTING_X0, last));
    // Valve: bonnet on the flange, cap, stem, hand wheel. The bonnet is
    // `steel`, not `steel_dark`: it stands in the tank's own shadow, and
    // the first render showed a dark bonnet as a black block against the
    // tank's dark base (the same lesson as the pipe stands).
    m.cuboid(
        (PIPE_FITTING_X0, last),
        (13, 18),
        (PIPE_AXIS_Z + 1, PIPE_AXIS_Z + 8),
        p.idx("steel"),
    );
    m.cuboid(
        (PIPE_FITTING_X0 + 1, last - 1),
        (14, 17),
        (PIPE_AXIS_Z + 9, PIPE_AXIS_Z + 10),
        p.idx("steel_light"),
    );
    m.cuboid(
        (29, 29),
        (15, 16),
        (PIPE_AXIS_Z + 11, PIPE_AXIS_Z + 12),
        p.idx("steel_dark"),
    );
    m.cylinder_z_f(
        (28.5, PIPE_C),
        3.5,
        (PIPE_AXIS_Z + 13, PIPE_AXIS_Z + 13),
        p.idx("brass"),
    );
    m.carve_cylinder_z_f((28.5, PIPE_C), 1.6, (PIPE_AXIS_Z + 13, PIPE_AXIS_Z + 13));
    m.cuboid(
        (28, 29),
        (15, 16),
        (PIPE_AXIS_Z + 13, PIPE_AXIS_Z + 13),
        p.idx("brass"),
    );
    m
}

/// The clamp Godot stands on every tile of a run the simulation cannot name
/// (issue #54 — `sim::PipeNaming`'s `unnameable` and `ambiguous`). A run
/// that carries nothing readable is empty, and so is a run that is merely
/// waiting for its first fill; the two used to render identically, which is
/// exactly how a whole branch of the factory sat idle without a word.
///
/// **It says "stopped", not "empty".** A hazard-striped block bolted
/// straight over the tube, wider and taller than the tube it covers, so a
/// faulted run reads as a line of beads rather than as a smooth pipe — a
/// silhouette a monochrome screenshot still carries (UI_UX.md §7: never
/// colour alone). It is signage, not a lamp: `matte` hazard striping, no
/// emissive, and no post — the andon poles came down on 2026-09-12 and the
/// hover panel is the readout, so this marks WHERE to look and the panel
/// says what is wrong.
///
/// **Square in X and Y about [`PIPE_C`], so it never turns.** A run's tiles
/// face whichever way the run runs; a marker overlaid on each of them would
/// otherwise need §2b point 29's four-facing check. Built symmetric under a
/// quarter turn instead (pinned by
/// `a_pipe_fault_clamp_is_unchanged_by_a_quarter_turn`), it sits at the same
/// yaw on a straight, a corner or a tee and covers the tube of each.
pub fn pipe_fault(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let (yellow, black) = (p.idx("hazard_yellow"), p.idx("hazard_black"));
    // Radius `PIPE_HUB_R + 1` from the axis on both horizontal axes: one
    // voxel clear of the widest casting a shape has at mid-tile, so the
    // clamp is never coplanar with what it covers (`PIPE_FITTING_MARGIN`'s
    // lesson, §2b point 24).
    let half = PIPE_HUB_R + 1.0;
    let span = (
        (PIPE_C - half).ceil() as i32,
        (PIPE_C + half).floor() as i32,
    );
    let z = (PIPE_AXIS_Z - 6, PIPE_AXIS_Z + 7);
    // Hazard bands ACROSS the clamp rather than [`VoxModel::hazard_stripe_x`]'s
    // diagonal: that stripe varies with x, and anything varying with x reads
    // one way on an east/west run and another on a north/south one. Banding
    // by height is the same signage and is the same from every side.
    for zz in z.0..=z.1 {
        let band = if ((zz - z.0) / 3) % 2 == 0 {
            yellow
        } else {
            black
        };
        m.cuboid(span, span, (zz, zz), band);
    }
    // A black cap band along the top edge: the block's own top face is the
    // one this camera sees most of, and a solid dark band there separates
    // the clamp from the bright `steel_light` tube by VALUE as well as by
    // shape (§2b point 11).
    m.cuboid(span, span, (z.1 - 1, z.1), black);
    m
}

/// The floating "!" Godot hangs over a placement that needs the player —
/// first a dock with every place taken (2026-09-26), and by design anything
/// else that has to say "look here" (a rack out of bays, a stuck machine).
/// It marks WHERE; the hover says what, as the pipe clamp above does.
///
/// **A solid bang, not a plate.** A flat sign would read on one axis of this
/// camera and edge-on in first person; a square-section bar and dot read as
/// "!" from every horizontal direction, so the marker needs no billboard and
/// no yaw. Square about voxel 15.5 on both axes (§2b point 20), pinned by
/// `a_warning_marker_is_unchanged_by_a_quarter_turn`.
///
/// **It glows.** `status_amber`, the andon's own warning hue, in the
/// `emissive` class, so it carries over a busy floor under the glow pass.
/// It floats clear of whatever it marks, so the floor behind it is the
/// darker grey the amber is read against. One uniform width throughout: a
/// first render with a dark collar and a wider head read as a pedestal
/// holding a block, not as a "!".
pub fn warning_marker(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let amber = p.idx("status_amber");
    // The dot, a gap as tall as the dot, then the bar to the top. The gap is
    // six voxels because Godot draws the marker at 0.75: a four-voxel gap
    // nearly closed at play zoom and read as one lit bar.
    m.cuboid((12, 19), (12, 19), (0, 5), amber);
    m.cuboid((12, 19), (12, 19), (12, 31), amber);
    m
}

/// The floating red person over a machine standing idle for want of a crew —
/// the [`warning_marker`]'s sibling, raised through the same `main.gd` door
/// with its own asset. The universal "user" pictogram in the round: a head
/// over a stepped, shouldered body, both turned about the tile's vertical
/// midline, so it reads as a person from every side with no yaw (§2b point
/// 20, `an_understaffed_marker_is_unchanged_by_a_quarter_turn`).
///
/// `status_red`, emissive, for the reason the "!" is amber: it has to carry
/// over a busy floor under the glow pass, and red is the andon's "stopped".
/// The neck gap is four voxels, the "!"'s lesson at a 0.75 draw scale.
pub fn understaffed_marker(p: &Palette) -> VoxModel {
    let mut m = VoxModel::tile();
    let red = p.idx("status_red");
    let mid = (15.5, 15.5);
    // The body, stepping in at the shoulders.
    m.cylinder_z_f(mid, 8.0, (0, 8), red);
    m.cylinder_z_f(mid, 7.0, (9, 10), red);
    m.cylinder_z_f(mid, 5.5, (11, 12), red);
    // The head, z 17..=28 over a gap of 13..=16.
    m.sphere_f((15.5, 15.5, 22.5), 6.0, red);
    m
}

// --- Crossings --------------------------------------------------------------
//
// The pipe half of a `crossing` tile: one entity carrying both a belt segment
// and a pipe tile (docs/BELT_PIPE_CROSSING_PLAN.md). The shape was found by a
// visual spike on 2026-09-25 that drew the design doc's flat bridge beside a
// raised gantry and rendered both with items on the belt; the gantry is what
// shipped and why is [`CROSSING_ITEM_TOP_Z`].
//
// Authored with the BELT running +X — the crossing's own `Direction` — and the
// PIPE along Y, so one yaw turns the pair exactly as it turns every other
// tile. [`pipe_crossing`] does not draw the belt: the scene stacks it over an
// ordinary `belt_f*` instance, so a crossing keeps the belt's four animation
// frames for nothing. [`crossing`] is the two merged, for the icon alone.
//
// **Everything here is symmetric about [`PIPE_C`] on BOTH horizontal axes**
// (§2b point 20; `a_crossing_is_symmetric_about_the_tile_midline`). A crossing
// turns four ways, and a belt running west is this model turned twice: an arch
// half a voxel off-centre would step at both of its joints with the run.

/// What a crossing actually has to clear, and it is NOT the belt.
///
/// The design doc's arithmetic stopped at the conveyor: the cleat tops out at
/// [`BELT_CLEAT_Z`] = 10 and the run's tube starts at 11, so a pipe left at
/// [`PIPE_AXIS_Z`] misses it by a voxel. But the belt is a floor for things
/// standing on it. An item is placed with its base on the running surface at
/// `BELT_SURFACE_VOXELS` = [`BELT_CLEAT_Z`] and `item_bottle` is 15 voxels
/// tall, so the tallest thing the line carries reaches z = 24 — a foot and a
/// half INTO a tube whose bore is z 12..19. Every solid on that belt would
/// pass through the pipe.
///
/// A measured fact about today's item models, not a derivation: the bottles
/// and the molasses jar tie for tallest. The vinegar jug stands at 25, but
/// vinegar is stock and never rides a belt. The value is enforced by
/// `a_crossing_clears_every_item_the_line_can_carry`, so a taller item fails
/// `cargo test` and does not quietly graze the arch.
const CROSSING_ITEM_TOP_Z: i32 = BELT_CLEAT_Z + 14;
/// Where the belt's traffic runs in plan: the widest *fill* among today's
/// item models, stood centred on the tile. An item's box is [`ITEM_VOXELS`]
/// = 16 wide, which centred is y 8..23, and `item_mustard_seed`'s sack fills
/// it to the edge — so the lane is the whole box. (It was 10..21, the boxes'
/// width, until mustard landed beside the crossing work.) The same test as
/// [`CROSSING_ITEM_TOP_Z`] holds this range to the real models. With that
/// constant, this is the box a crossing must leave empty.
const CROSSING_ITEM_Y: (i32, i32) = (8, 23);
/// Height of the raised run, one full tile above the floor. Measured, the
/// lowest voxel of [`pipe_crossing`] BETWEEN THE RISERS — y 8..23, which
/// contains [`CROSSING_ITEM_Y`] — is z = 28, which leaves three voxels of
/// daylight (25..27) over [`CROSSING_ITEM_TOP_Z`]. Not "over the lane": the
/// risers stand on the lane's own boundary rows, y 7 and y 24, and carry
/// steel from z 13 up. Nothing travels those rows.
/// Lower and the bottles graze it, which reads as the same mistake as
/// passing through it.
const GANTRY_AXIS_Z: i32 = TILE_VOXELS as i32;
/// How far the crossing's run sits above an ordinary run — **mirrored by
/// `main.gd`**, which stands every per-tile pipe overlay on a crossing this
/// many voxels higher than on any other pipe tile: the liquid body at
/// `PIPE_BORE_FLOOR_VOXELS + CROSSING_LIFT` and the fault clamp at the floor
/// plus it. Both overlays are authored at the run's own height so that one
/// constant is the whole difference; change one side, change the other.
pub const CROSSING_LIFT: i32 = GANTRY_AXIS_Z - PIPE_AXIS_Z;
/// Tall enough for the raised tube's crown at z = 36 plus the window carve.
const GANTRY_MODEL_Z: u32 = 38;
/// Where the run turns up, and (mirrored about [`PIPE_C`]) back down. Fixed
/// by the items again, this time in plan: the lane's traffic occupies
/// [`CROSSING_ITEM_Y`], so the free bands either side are y 0..7 and y
/// 24..31. Measured, everything the gantry has below the span (riser, elbow
/// castings and posts) fills y 0..7 and y 24..31. The lane is open from y 8
/// to 23 — exactly [`CROSSING_ITEM_Y`], with no row to spare: the seed sack
/// passes between the risers touching neither.
const GANTRY_ELBOW_Y: f32 = 3.0;
/// The elbow casting, and NOT [`PIPE_HUB_R`]. A corner tile carries one hub
/// and can afford the swelling that says "fitting"; a gantry carries four
/// within a third of a tile, and at the run's own hub radius the first render
/// came back with a lumpy pale column at each end of the arch instead of two
/// elbows. Half a voxel of swell is enough to break the line and little
/// enough to keep the silhouette a pipe's.
const GANTRY_HUB_R: f32 = PIPE_OUTER_R + 0.5;
/// The sight glass over the lane, in Y — centred on the belt, so the
/// liquid shows directly over the product passing under it.
const GANTRY_WINDOW: (i32, i32) = (12, 19);

const _: () = assert!(
    PIPE_BORE_Z0 + CROSSING_LIFT > CROSSING_ITEM_TOP_Z,
    "the raised run's bore floor must stand above the tallest item on the belt"
);
// `<=`, not `<`: the casting's equator reaches y = 8.0 exactly, but only on
// the x = 15.5 axis ([`PIPE_C`]), where no voxel centre sits, so it fills
// nothing past y 7. `a_crossing_clears_what_the_belt_is_carrying` is the
// voxel-level proof; this is the cheap tripwire for a bigger casting.
const _: () = assert!(
    GANTRY_ELBOW_Y + GANTRY_HUB_R <= CROSSING_ITEM_Y.0 as f32,
    "the elbow casting must stand clear of the lane's traffic in plan"
);

/// The two posts a crossing stands on, replacing [`pipe_stand`]'s single
/// central one — which cannot stay, because it stands in the belt's lane.
/// Both sit clear of the conveyor's side frames (y 4..6 and 25..27) and of
/// its legs, so the two models never share a voxel.
fn crossing_posts(m: &mut VoxModel, p: &Palette) {
    let steel = p.idx("steel");
    for (y0, y1) in [(1, 3), (28, 30)] {
        m.cuboid((13, 18), (y0, y1), (0, 0), steel);
        m.cuboid((14, 17), (y0, y1), (1, PIPE_AXIS_Z - 7), steel);
        m.cuboid(
            (13, 18),
            (y0 - 1, y1 + 1),
            (PIPE_AXIS_Z - 6, PIPE_AXIS_Z - 5),
            steel,
        );
    }
}

/// **The crossing's pipe: a gantry.** The run arrives at [`PIPE_AXIS_Z`] on
/// the tile's edge, elbows up at [`GANTRY_ELBOW_Y`], crosses the lane at
/// [`GANTRY_AXIS_Z`] — clear over the tallest bottle — and comes back down
/// to meet its neighbour at the far face. Both open faces are a plain bore on
/// the run's own axis, so a crossing drops into an existing north–south run
/// with no fitting and no change to the tiles either side of it.
///
/// Drawn solid and hollowed afterwards, the rule every junction shape here
/// follows: legs and elbow castings then open into one another instead of
/// meeting as two separately-bored tubes. The sight glass is on the span over
/// the lane ([`GANTRY_WINDOW`]), the only length of this run that is ever
/// level at a height the liquid body can stand in.
pub fn pipe_crossing(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, GANTRY_MODEL_Z]);
    let last = TILE_VOXELS as i32 - 1;
    let low = PIPE_AXIS_Z as f32;
    let high = GANTRY_AXIS_Z as f32;
    let near = GANTRY_ELBOW_Y;
    let far = PIPE_C * 2.0 - GANTRY_ELBOW_Y;
    let steel = p.idx("steel_light");

    crossing_posts(&mut m, p);

    pipe_leg_y(&mut m, p, (0, near as i32));
    pipe_leg_y(&mut m, p, (far as i32, last));
    for y in [near, far] {
        m.cylinder_z_f(
            (PIPE_C, y),
            PIPE_OUTER_R,
            (PIPE_AXIS_Z, GANTRY_AXIS_Z),
            steel,
        );
        m.sphere_f((PIPE_C, y, low), GANTRY_HUB_R, steel);
        m.sphere_f((PIPE_C, y, high), GANTRY_HUB_R, steel);
    }
    m.cylinder_y_f(
        (PIPE_C, high),
        PIPE_OUTER_R,
        (near as i32, far as i32),
        steel,
    );

    pipe_bore_y(&mut m, (0, near as i32));
    pipe_bore_y(&mut m, (far as i32, last));
    for y in [near, far] {
        m.carve_cylinder_z_f((PIPE_C, y), PIPE_BORE_R, (PIPE_AXIS_Z, GANTRY_AXIS_Z));
        m.carve_sphere_f((PIPE_C, y, low), PIPE_BORE_R);
        m.carve_sphere_f((PIPE_C, y, high), PIPE_BORE_R);
    }
    m.carve_cylinder_y_f((PIPE_C, high), PIPE_BORE_R, (near as i32, far as i32));
    // [`pipe_window_x`] turned along Y and lifted: from just above the
    // raised bore's floor to the top of the model.
    let top = m.size[2] as i32 - 1;
    m.carve_cuboid(
        PIPE_BORE_Y,
        GANTRY_WINDOW,
        (PIPE_BORE_Z0 + CROSSING_LIFT + 1, top),
    );
    m
}

/// The crossing as the player knows it — belt, arch and a bottle passing
/// under it — **for the icon pass alone** (§2b point 35). The scene never
/// places this: it stacks [`pipe_crossing`] over the live belt frames, and an
/// item here would stand still on a moving line. The icon is the only place a
/// crossing is shown with nothing around it, and the arch on its own reads
/// as a bent pipe, not as something a line runs under.
pub fn crossing(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, GANTRY_MODEL_Z]);
    m.merge(&belt_f0(p));
    // Stood the way `main.gd` stands a carried item: centred on the tile,
    // base on the running surface.
    let inset = (TILE_VOXELS - ITEM_VOXELS) as i32 / 2;
    for (&(x, y, z), &idx) in &item_bottle(p).voxels {
        m.set(
            x as i32 + inset,
            y as i32 + inset,
            z as i32 + BELT_CLEAT_Z,
            idx,
        );
    }
    m.merge(&pipe_crossing(p));
    m
}

/// The liquid in a [`pipe_crossing`]'s raised span — the one-voxel slab every
/// other fluid body is, authored at the run's own height like them, and
/// stood [`CROSSING_LIFT`] higher by `main.gd`. It runs elbow to elbow and no
/// further: the low stubs either side have no window to show liquid through,
/// and a slab reaching the tile face at the raised height would poke out of
/// the elbow casting into open air.
pub fn pipe_fluid_crossing(p: &Palette) -> VoxModel {
    let mut m = pipe_fluid_slab();
    let near = GANTRY_ELBOW_Y as i32;
    pipe_fluid_y(&mut m, p, (near, TILE_VOXELS as i32 - 1 - near));
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

// --- Where each machine is cut into a fixed half and a moving half ---------
//
// Every number below is read from TWO places — the body generator and the
// matching `*_motion` overlay in the "Machine motion" section — which is
// exactly why it is a named constant rather than a literal in each. The
// radii come in core/outer pairs on purpose: `cylinder_*` fills `d <= r` and
// `arm_cells` fills `inner < d <= outer`, so a fixed core of radius R and a
// moving annulus starting at the same R interlock and never share a voxel
// (UI_UX.md §2b point 24 is the same problem seen from the other side).

/// The crusher's flywheel hub, which stays with the body, and the radius its
/// spokes reach out to.
const CRUSHER_HUB_R: f32 = 1.8;
const CRUSHER_SPOKE_R: f32 = 5.0;
/// The crushing rolls: a smooth core on the body, teeth in the annulus out
/// to the diameter the rolls have always had. Widening the OUTER radius is
/// not free — at 4.0 a tooth just clears the feed pocket's east wall.
const CRUSHER_ROLL_CORE_R: f32 = 3.0;
const CRUSHER_ROLL_R: f32 = 4.0;
const CRUSHER_ROLL_Y: (i32, i32) = (9, 22);
/// The floor of the carved feed pocket. Below it the rolls are inside the
/// crusher's own frame, so that is where the teeth stop.
const CRUSHER_POCKET_FLOOR_Z: i32 = 17;

/// The mixer drum's axis, and the ribbon agitator wound about it: radius,
/// and how many opposed flights it has (which is the fold its animation
/// closes on).
const MIXER_DRUM_X: i32 = 17;
const MIXER_DRUM_Z: i32 = 18;
const MIXER_RIBBON_R: f32 = 7.0;
/// The rows of the drum the ribbon runs through. NOT the drum's full length,
/// and the voxgen tests are what found why: two feed throats and the
/// discharge nozzle reach in through the drum wall, and a flight at
/// [`MIXER_RIBBON_R`] passes straight through all three. Those rows are also
/// the ones the sight window (y 24..39) does not show, so what the player
/// can see of the ribbon is unchanged — a shorter ribbon here, rather than a
/// thinner one everywhere, keeps the flights out at the wall where the
/// window shows them turning.
const MIXER_RIBBON_Y: (i32, i32) = (20, 43);
const MIXER_RIBBON_FLIGHTS: u32 = 2;

/// The bottler's three stations: two filled, one capped.
const BOTTLER_FILL_X: [i32; 2] = [8, 16];
const BOTTLER_CAP_X: i32 = 24;
const BOTTLER_NOZZLE_R: f32 = 1.4;
/// The capping spindle the body hangs off the beam, and the chuck that rides
/// down it. Core and annulus, as above.
const BOTTLER_SPINDLE_R: f32 = 1.4;
const BOTTLER_CHUCK_R: f32 = 3.2;
/// How far the filling and capping heads are down on each frame. There and
/// back, so the cycle closes the way a rotation does.
const BOTTLER_PLUNGE: [i32; MACHINE_FRAMES as usize] = [0, -1, -2, -1];

/// The packer's taping carriage: its X offset per frame, and the tape roll
/// it carries. The roll clears the stand at its far end only because it is
/// this narrow and this low — see `machine_packer_motion`.
const PACKER_TRAVEL: [i32; MACHINE_FRAMES as usize] = [0, 3, 6, 3];
const PACKER_ROLL_Z: i32 = 23;
const PACKER_ROLL_R: f32 = 2.5;

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
    m.hazard_stripe_x(
        (xlo, xhi),
        (ylo, ylo),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    m.hazard_stripe_x(
        (xlo, xhi),
        (yhi, yhi),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    for y in ylo..=yhi {
        let a = if (y + 2).rem_euclid(6) < 3 {
            p.idx("hazard_yellow")
        } else {
            p.idx("hazard_black")
        };
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
    m.cuboid(
        (bxlo, bxhi),
        (bylo, byhi),
        (skirt_top + 1, skirt_top + 2),
        p.idx("steel"),
    );
}

/// Control panel plus its two status lamps, on the **-Y face** — the one the
/// fixed isometric camera looks at (see this module's header and `machine()`).
/// `x` is the panel's span, `z` its height band.
fn control_panel(m: &mut VoxModel, p: &Palette, x: (i32, i32), z: (i32, i32), face_y: i32) {
    let mid = (z.0 + z.1) / 2;
    m.cuboid(x, (face_y, face_y), z, p.idx("steel_light"));
    m.cuboid(
        (x.0 + 2, x.1 - 2),
        (face_y - 1, face_y - 1),
        (z.0 + 1, z.1 - 1),
        p.idx("hazard_black"),
    );
    m.cuboid(
        (x.0 - 5, x.0 - 3),
        (face_y - 1, face_y),
        (mid - 1, mid + 1),
        p.idx("status_green"),
    );
    m.cuboid(
        (x.1 + 3, x.1 + 5),
        (face_y - 1, face_y),
        (mid - 1, mid + 1),
        p.idx("status_amber"),
    );
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
    m.cuboid(
        (0, d),
        (l0 - 2, l1 + 2),
        (floor, roof),
        p.idx("steel_light"),
    );
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
    m.cuboid(
        (0, d - 1),
        (l1 + 1, l1 + 2),
        (floor + 1, roof - 2),
        p.idx("hazard_black"),
    );
    m.cuboid(
        (d - 2, d),
        (l0, l1),
        (floor, roof - 2),
        p.idx("hazard_black"),
    );
    // Brass threshold on the lip, where the belt's dead-plate hands over —
    // the one bright thing inside the opening, and the same accent every
    // other "product passes through here" fitting in the factory carries.
    m.cuboid((0, 1), (l0, l1), (floor, floor), p.idx("brass"));

    // Window head: one yellow band, because a covered nip point on a real
    // line is marked, and because it separates the housing's bright roof
    // from its bright wall. NOT a full hazard stripe — a stripe is half
    // black, and black on the piece's only camera-facing edge turned the
    // whole hood into a dark bar in the first render.
    m.cuboid(
        (0, d),
        (l0 - 2, l0 - 2),
        (roof - 1, roof - 1),
        p.idx("hazard_yellow"),
    );
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
    m.cuboid(
        (last - 3, last),
        (l0 - 2, l1 + 2),
        (floor, head),
        p.idx("steel_light"),
    );
    m.carve_cuboid((last - 2, last), (l0, l1), (floor + 1, head - 2));
    // The back of the mouth. Its normal is +X, i.e. straight at the camera,
    // which is what turns a cut-out silhouette into a hole with depth.
    m.cuboid(
        (last - 3, last - 3),
        (l0, l1),
        (floor, head - 2),
        p.idx("hazard_black"),
    );

    // Discharge lip, level with the belt beside it and running out to the
    // tile edge so there is nothing between the two.
    m.cuboid(
        (last - 3, last),
        (l0, l1),
        (floor, floor),
        p.idx("steel_light"),
    );
    // A brass band across the head of the opening — the same accent the
    // pipe fittings' flanges and the tank valve's hand wheel carry, and
    // for the same reason: it marks a fitting product actually passes
    // through. A hazard stripe was tried here first and rendered as a dark
    // bar, because half a stripe is black and this edge is the only part of
    // the piece the camera looks straight at.
    m.cuboid(
        (last - 2, last),
        (l0 - 1, l1 + 1),
        (head - 1, head - 1),
        p.idx("brass"),
    );
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
    m.cuboid(
        gusset,
        (lane_y - 1, lane_y + 1),
        (9, PIPE_AXIS_Z - 3),
        p.idx("steel_dark"),
    );
}

/// The colour every machine paints the product it is visibly working on:
/// the sauce standing in `machine_bottler`'s bottles, the bottle in
/// `machine_packer`'s open case, and the level behind
/// `machine_cooking_vat`'s sight glass.
///
/// **This is a mechanism, not a colour choice** (2026-09-21). It is
/// `fluid_pale` — the same near-white the pipe run and the fluid tank are
/// drawn in — and `fluid_pale` is the `liquid` class, which no other voxel
/// of any of those three machines uses. So `voxgen` splits exactly the
/// product into a mesh of its own (`machine_bottler.liquid.vox`, §2b point
/// 2) and `main.gd` multiplies the sauce's own colour over that one mesh,
/// from the machine's LIVE recipe (its `PRODUCT_TINTS`). A bottler drawing
/// BBQ shows BBQ; the same bottler switched back to ketchup shows ketchup,
/// with no second model and no per-kind code.
///
/// The three models keep their painted sauce in the COMBINED `name.vox`
/// the icon pass renders, because the tint only exists in the running
/// game: a build-menu icon therefore shows a bottler holding near-white
/// bottles, which is the honest picture of a machine that is not making
/// anything yet — the same reading `fluid_tank`'s empty pale column and
/// `hopper`'s empty bin already ship.
///
/// **Keeping `liquid` to the product alone is what holds this up**, so a
/// test asserts it for all three: paint anything else on one of these
/// machines with a `liquid` colour and it would be tinted as if it were
/// sauce.
fn machine_product(p: &Palette) -> u8 {
    p.idx("fluid_pale")
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
    m.hazard_stripe_x(
        (7, 24),
        (8, 8),
        (24, 24),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );

    // The two crushing rolls, drawn AFTER the pocket is carved so they
    // stand in it. Both bright steel: dark steel was the first choice and,
    // in the rendered scene, sank into the frame's own shadow so only one
    // roll was legible. The second roll was brass for a while, to split the
    // pair by colour; the owner asked for both grey (2026-09-26), and the
    // teeth in `machine_crusher_motion` still part them at the nip.
    //
    // Only the smooth CORE is drawn here: the teeth that make a roll read as
    // turning belong to `machine_crusher_motion`, in the annulus this core
    // stops at, and a roll drawn out to its full radius here would z-fight
    // every one of them.
    m.cylinder_y(
        (12, 16),
        CRUSHER_ROLL_CORE_R,
        CRUSHER_ROLL_Y,
        p.idx("steel_light"),
    );
    m.cylinder_y(
        (20, 16),
        CRUSHER_ROLL_CORE_R,
        CRUSHER_ROLL_Y,
        p.idx("steel_light"),
    );

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
    //
    // The wheels are a bright FACE and a brass hub here; the dark web between
    // them became four spokes in `machine_crusher_motion`, which is what
    // makes them visibly turn. A running crusher and a stopped one differ in
    // where those spokes point, not in whether they exist.
    for x in [12, 20] {
        m.cylinder_y((x, 16), 7.0, (3, 4), p.idx("steel_light"));
        m.cylinder_y((x, 16), CRUSHER_HUB_R, (1, 5), p.idx("brass"));
    }
    m.cuboid((5, 27), (3, 5), (23, 24), p.idx("steel_light"));

    // Drive motor. It used to sit against the -X side across the whole belt
    // lane (x 0..4, y 9..22) — i.e. exactly where the belt arrives, so the
    // first junction in the factory was a conveyor running into the side of
    // a motor. It now stands on the far (+Y) shoulder, clear of the lane, so
    // the infeed housing below has somewhere to be. It stops at y 29, two
    // voxels short of the tile edge, and not at 30: turned 180° that far
    // shoulder becomes the +X/low-Y corner, where a free-standing prop one
    // voxel inside the tile edge meets it (UI_UX.md §2b point 29 — learned
    // from the andon stack light that stood there until it was removed on
    // 2026-09-12). One voxel further in, nothing z-fights.
    m.cuboid((0, 5), (25, 29), (10, 19), p.idx("paint_blue"));
    m.cuboid((0, 5), (25, 29), (20, 20), p.idx("steel_light"));
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
    m.cylinder_y(
        (MIXER_DRUM_X, MIXER_DRUM_Z),
        11.0,
        (5, last_y - 5),
        p.idx("steel"),
    );
    // Bands, the same seam language every other vessel on the line carries.
    for y in [9, 26, 37, 54] {
        m.cylinder_y(
            (MIXER_DRUM_X, MIXER_DRUM_Z),
            11.6,
            (y, y),
            p.idx("steel_light"),
        );
    }
    // Dished ends, so the drum is a vessel rather than a length of duct.
    for (y, r) in [(4, 9.0f32), (3, 6.5), (last_y - 4, 9.0), (last_y - 3, 6.5)] {
        m.cylinder_y(
            (MIXER_DRUM_X, MIXER_DRUM_Z),
            r,
            (y, y),
            p.idx("steel_light"),
        );
    }

    // Hollow it, then cut the sight window through the crown — `pipe_body`'s
    // trick, for `pipe_body`'s reason: an open trough reads as a gutter, a
    // framed window reads as a vessel you can see into.
    m.carve_cylinder_y((MIXER_DRUM_X, MIXER_DRUM_Z), 9.0, (5, last_y - 5));
    // A SHORT window, not a long one. The first cut ran it two thirds of the
    // drum's length and the rendered factory showed the ribbon flights along
    // almost the whole vessel — which at scene scale reads as corrugation, a
    // ribbed duct, rather than as a closed drum you can see into. Half a
    // tile is enough to show the agitator turning and leaves the drum's own
    // round back to read along, exactly the trade `pipe_body` records about
    // the pipe run's sight glass.
    m.carve_cuboid(
        (MIXER_DRUM_X - 5, MIXER_DRUM_X + 5),
        (24, last_y - 24),
        (MIXER_DRUM_Z + 1, 29),
    );

    // Ribbon agitator: a shaft down the drum's axis, carrying a helical
    // ribbon. This is the part the window exists to show, so it sits high in
    // the drum where the sight line reaches it — and it is also the part that
    // has to TURN, so only the shaft is here. The ribbon itself is
    // `machine_mixer_motion`, wound about this same axis at
    // `MIXER_RIBBON_R`, well clear of both the shaft and the drum wall.
    m.cylinder_y(
        (MIXER_DRUM_X, MIXER_DRUM_Z),
        1.8,
        (4, last_y - 4),
        p.idx("steel_light"),
    );

    // Drive end: a motor and gearbox on the far (+Y) end of the shaft.
    // Far, not near, and that is the camera again: `control_panel`'s own doc
    // records that low local Y is the face this camera looks at, so a drive
    // that bulky on the near end would stand between the viewer and the
    // sight window this machine is identified by.
    m.cuboid(
        (11, 23),
        (last_y - 7, last_y - 1),
        (10, 22),
        p.idx("paint_blue"),
    );
    m.cuboid(
        (11, 23),
        (last_y - 7, last_y - 1),
        (23, 23),
        p.idx("steel_dark"),
    );
    m.cylinder_y(
        (MIXER_DRUM_X, MIXER_DRUM_Z),
        4.0,
        (last_y - 8, last_y - 6),
        p.idx("brass"),
    );

    // Vinegar inlet: the stub reaching north off the sim-y = 1 half, toward
    // the pipe run the reference chain lays one row north of the footprint.
    // Height is PIPE_AXIS_Z exactly, matching `pipe_fitting_machine` — the
    // two are one spool piece split across a tile boundary, and the pair
    // only reads as a connection because they are the same diameter, the
    // same bright steel and carry their brass flange at the same boundary.
    m.cylinder_y(
        (MIXER_DRUM_X - 1, PIPE_AXIS_Z),
        3.0,
        (0, 9),
        p.idx("steel_light"),
    );
    m.cylinder_y((MIXER_DRUM_X - 1, PIPE_AXIS_Z), 4.0, (0, 1), p.idx("brass"));
    m.cuboid(
        (MIXER_DRUM_X - 2, MIXER_DRUM_X),
        (2, 7),
        (9, PIPE_AXIS_Z - 3),
        p.idx("steel_dark"),
    );

    // The two feed chutes: a hooded transfer housing per inlet tile, then a
    // short throat from each up into the drum's crown, so what arrives on
    // either belt visibly goes somewhere.
    machine_infeed(&mut m, p, near_lane);
    machine_infeed(&mut m, p, origin_lane);
    for lane in [near_lane, origin_lane] {
        let mid = (lane.0 + lane.1) / 2;
        m.cuboid(
            (6, 10),
            (mid - 3, mid + 3),
            (BELT_SURFACE_Z, 24),
            p.idx("steel_light"),
        );
        m.cuboid(
            (10, MIXER_DRUM_X - 6),
            (mid - 2, mid + 2),
            (22, 24),
            p.idx("steel_light"),
        );
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
    // in stainless.
    //
    // The sauce behind it is the product mesh (`machine_product`), so the
    // vat shows what it is actually cooking. It was flat `sauce_red`, under
    // a note here saying matte was deliberate — "it is product, not a
    // polished surface" — and it is that reading, not the rule, that was
    // wrong: this vat cooks BBQ as well as ketchup (`data/recipes.ron`), so
    // a fixed red was never product, it was one product. What stands behind
    // a sight glass is liquid in a vessel, exactly as `fluid_tank_fill` is,
    // and the gloss the class brings is the one every other visible liquid
    // in this factory already carries.
    m.cuboid((29, 36), (5, 8), (16, 34), p.idx("steel_light"));
    m.cuboid((31, 34), (4, 6), (17, 29), machine_product(p));

    // Access platform, handrail and ladder — the industrial language
    // UI_UX.md §3 asks for, and what actually sells the vat's size.
    m.cuboid((14, 50), (2, 8), (30, 31), p.idx("steel_dark"));
    m.hazard_stripe_x(
        (14, 50),
        (2, 2),
        (30, 31),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
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
    // is standing in it. Their CONTENTS are the product mesh
    // (`machine_product`), which is what keeps that true of a second sauce:
    // a bottler drawing BBQ off its pipe filled three ketchup-red bottles
    // until 2026-09-21.
    //
    // The label band at z 14..15 replaces the sauce outright over those two
    // layers rather than ringing it, because r 2.6 and r 2.8 round to the
    // same cells here — a one-voxel ring would be empty, and the two solids
    // would share every cell they cover (§2b point 24).
    for x in [8, 16, 24] {
        m.cylinder_z((x, 16), 2.6, (12, 17), machine_product(p));
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
    // Only the HANGERS are structure. The nozzles that hang off them are
    // `machine_bottler_motion`'s, because a filling nozzle that never moves
    // is a spout.
    for x in BOTTLER_FILL_X {
        m.cuboid((x - 1, x + 1), (15, 22), (25, 26), p.idx("steel_light"));
    }

    // Capping head over the last bottle: a brass rotary chuck on a stem off
    // the beam — the "sealing" half of "bottling and sealing".
    // The arm and the fixed SPINDLE under it; the brass chuck that runs down
    // that spindle onto the bottle is `machine_bottler_motion`'s, cut as a
    // ring around it so the two never share a voxel.
    m.cuboid((23, 25), (16, 22), (25, 26), p.idx("steel_light"));
    m.cylinder_z(
        (BOTTLER_CAP_X, 16),
        BOTTLER_SPINDLE_R,
        (22, 25),
        p.idx("steel_dark"),
    );

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
    // The bottle in it is the one thing on this machine that says WHICH
    // sauce is being packed, so it is the product mesh (see
    // `machine_product`) and not a painted red: a packer boxing BBQ used
    // to stand a ketchup bottle in its case.
    m.cylinder_z((22, 15), 2.2, (14, 19), machine_product(p));
    m.cylinder_z((22, 15), 1.2, (20, 21), p.idx("glass_pale"));

    // Taping head on a stand over the case: an arm carrying a brass tape
    // roll, reaching in from the back so it doesn't block the open case.
    // The stand stops at x = 27 rather than 28 so it clears the discharge
    // housing added below, which occupies the last four columns.
    // The stand and the beam only. What hangs off the beam — the taping head
    // and its tape roll — is one carriage that traverses the case's seam, so
    // it is `machine_packer_motion`'s and nothing here.
    m.cuboid((25, 27), (23, 26), (12, 27), p.idx("steel"));
    m.cuboid((16, 27), (18, 26), (26, 27), p.idx("steel_light"));

    // Infeed AND outfeed. The packer used to have no discharge at all,
    // because it was the chain's last stage and the player lifted finished
    // cases straight out of it with a keypress. The delivery-dock ticket
    // deleted that command and laid a real belt past this machine, running
    // into a dock — so this face now hands product to a conveyor like every
    // other stage's does, and would otherwise be a blank wall a case
    // appears in front of out of nowhere.
    machine_infeed(&mut m, p, BELT_LANE);
    machine_outfeed(&mut m, p, BELT_LANE, TILE_VOXELS as i32 - 1);

    // Centred like every other 1x1 kind's, and not at (5, 14) as it was:
    // that put the green lamp at x 0..2, which turned 90° lands one voxel
    // in from the +X edge — where a free-standing prop on that corner
    // z-fought it (UI_UX.md §2b point 29; the andon post that found this
    // has since been removed).
    control_panel(&mut m, p, (11, 20), (3, 7), 2);
    m
}

// ---------------------------------------------------------------------------
// Machine motion (the machine-animation pass)
// ---------------------------------------------------------------------------
//
// Every processing kind on the line gets a **motion overlay**: a small model
// with the same X/Y footprint as the machine's body, holding only the parts
// that move, in `MACHINE_FRAMES` arrangements. Godot stands all four frames
// on the machine's tile as siblings of the body and shows exactly one
// (`main.gd`'s `_update_machine_motion`), which is the belt family's own
// mechanism (UI_UX.md §2b point 23) applied to something that is not a tile.
//
// Four rules hold for every family here, and the voxgen tests pin all four:
//
//  1. **The mechanism is a rearrangement, not an appearance.** Frame 0 is
//     the machine at rest and is always drawn, so a stopped machine still
//     has its flywheels, its ribbon and its taping head — the animation adds
//     no part that was not already there. That is why steam was left out:
//     a puff is mass a stopped vat should not have, and a frozen cloud over
//     a cold vat states the opposite of what the lamp says. What the tests
//     pin is that CONSECUTIVE frames differ, not that all four do: a
//     reciprocating part honestly passes through the same pose twice a
//     cycle (the bottler's plunge is down, half, up, half), and only a
//     turning part must be at four different angles.
//  2. **The overlay's voxels are disjoint from the body's.** Two voxel
//     solids z-fight wherever they share a cell (UI_UX.md §2b point 24), so
//     anything that moves is REMOVED from the body and lives only here — the
//     crusher's flywheel spokes and roll teeth, the mixer's ribbon, the
//     bottler's nozzles and capping chuck, the packer's taping carriage.
//     Where a moving part turns inside a fixed one the two are cut as a core
//     and an annulus about the same axis (`cylinder_*` fills `d <= r`,
//     [`arm_cells`] fills `inner < d <= outer`), so they interlock without
//     ever sharing a cell.
//  3. **The cycle closes exactly.** An `n`-fold feature advances one `n`th
//     of a turn spread over `MACHINE_FRAMES` frames ([`spin`]), so frame
//     `MACHINE_FRAMES` IS frame 0 and a running machine never jumps
//     backwards. A linear travel (the packer's carriage, the bottler's
//     plunge) uses a there-and-back offset table of the same length, for the
//     same reason.
//  4. **The volume may be TALLER than the body, never wider.** `vox::encode`
//     lifts a model by half its own height, so the volume's floor lands on
//     the ground whatever `size[2]` is, but its X/Y centring is the
//     footprint's — the vat's drive coupling therefore stands above a 48-
//     voxel body in a 56-voxel volume and still lines up with it. That lift
//     is `size[2] / 2` in WHOLE voxels, so an ODD height sinks the model
//     half a voxel and the overlay meets the body along a seam instead of a
//     face: every volume here is even, and a test says so.
//
// Frames are also deliberately UNIFORM across kinds: `MACHINE_FRAMES` frames
// per cycle for every machine, with the pace set once in `main.gd`
// (`MACHINE_TICKS_PER_CYCLE`). A per-kind speed would make the fast crusher
// and the slow vat look like different factories.

/// Animation frames in one machine's motion cycle. Mirrored by `main.gd`'s
/// `MACHINE_FRAMES`; change one, change both. Four, like the belt: enough
/// for a turn to read as a turn, few enough that each frame is a pose
/// somebody chose (UI_UX.md §6's chunky, snappy motion).
pub const MACHINE_FRAMES: u32 = 4;

/// How far a `fold`-fold feature has turned by frame `phase`: one `fold`th
/// of a full turn, divided into `MACHINE_FRAMES` steps. Frame
/// `MACHINE_FRAMES` therefore lands exactly back on frame 0's arrangement,
/// which is what makes the loop seamless rather than nearly seamless.
fn spin(phase: u32, fold: u32) -> f32 {
    std::f32::consts::TAU / fold as f32 * phase as f32 / MACHINE_FRAMES as f32
}

/// The cells of `count` evenly spaced arms lying in the annulus `radii`
/// about `center`, the first arm's centreline at `angle` radians.
///
/// Axis-agnostic on purpose: the caller decides what the two coordinates
/// mean — `(x, z)` for a wheel on a horizontal shaft (the crusher's
/// flywheels, drawn along Y), `(x, y)` for a drive spider on a vertical one
/// (the vat's coupling). That is the same freedom `cylinder_x/y/z` give,
/// expressed once.
///
/// `thickness` is an arm's half-width measured along the ARC, in voxels, so
/// an arm is the same width at the hub as at the rim. Measuring it in
/// radians instead would fan every arm into a wedge, and a wedge of spokes
/// is a disc.
///
/// The inner radius is meant to be exactly the radius of whatever the body
/// draws on the same axis: `cylinder_*` fills `d <= r` and this fills
/// `inner < d <= outer`, so a core and the arms turning around it cannot
/// share a voxel (rule 2 in this section's header).
fn arm_cells(
    center: (i32, i32),
    radii: (f32, f32),
    count: u32,
    thickness: f32,
    angle: f32,
) -> Vec<(i32, i32)> {
    use std::f32::consts::{PI, TAU};
    let (r_in, r_out) = radii;
    let span = (r_out + 1.0).ceil() as i32;
    let step = TAU / count as f32;
    let mut cells = Vec::new();
    for v in (center.1 - span)..=(center.1 + span) {
        for u in (center.0 - span)..=(center.0 + span) {
            let (du, dv) = ((u - center.0) as f32, (v - center.1) as f32);
            let d2 = du * du + dv * dv;
            if d2 <= r_in * r_in || d2 > r_out * r_out {
                continue;
            }
            let d = d2.sqrt();
            let a = dv.atan2(du);
            for k in 0..count {
                let mut delta = (a - (angle + step * k as f32)).rem_euclid(TAU);
                if delta > PI {
                    delta -= TAU;
                }
                if delta.abs() * d <= thickness {
                    cells.push((u, v));
                    break;
                }
            }
        }
    }
    cells
}

macro_rules! machine_motion_fns {
    ($($name:ident => $build:ident, $phase:expr;)*) => {
        $(
            pub fn $name(p: &Palette) -> VoxModel {
                $build(p, $phase)
            }
        )*
    };
}

machine_motion_fns! {
    machine_motion_f0 => machine_motion, 0;
    machine_motion_f1 => machine_motion, 1;
    machine_motion_f2 => machine_motion, 2;
    machine_motion_f3 => machine_motion, 3;
    machine_crusher_motion_f0 => machine_crusher_motion, 0;
    machine_crusher_motion_f1 => machine_crusher_motion, 1;
    machine_crusher_motion_f2 => machine_crusher_motion, 2;
    machine_crusher_motion_f3 => machine_crusher_motion, 3;
    machine_mixer_motion_f0 => machine_mixer_motion, 0;
    machine_mixer_motion_f1 => machine_mixer_motion, 1;
    machine_mixer_motion_f2 => machine_mixer_motion, 2;
    machine_mixer_motion_f3 => machine_mixer_motion, 3;
    machine_cooking_vat_motion_f0 => machine_cooking_vat_motion, 0;
    machine_cooking_vat_motion_f1 => machine_cooking_vat_motion, 1;
    machine_cooking_vat_motion_f2 => machine_cooking_vat_motion, 2;
    machine_cooking_vat_motion_f3 => machine_cooking_vat_motion, 3;
    machine_bottler_motion_f0 => machine_bottler_motion, 0;
    machine_bottler_motion_f1 => machine_bottler_motion, 1;
    machine_bottler_motion_f2 => machine_bottler_motion, 2;
    machine_bottler_motion_f3 => machine_bottler_motion, 3;
    machine_packer_motion_f0 => machine_packer_motion, 0;
    machine_packer_motion_f1 => machine_packer_motion, 1;
    machine_packer_motion_f2 => machine_packer_motion, 2;
    machine_packer_motion_f3 => machine_packer_motion, 3;
}

/// The generic [`machine`]'s moving part: a four-armed drive spider on the
/// crown of its dome.
///
/// The fallback kind needs one for the reason its junction hardware does —
/// a kind added to `data/machines.ron` with no bespoke model still has to
/// look like it is working when it is (UI_UX.md §2b point 12's fallback
/// rule, applied to motion). The spider sits on TOP because that is the one
/// surface this camera looks straight down at, and because the dome's crown
/// is the only part of the generic body nothing else stands on: the whole
/// overlay lives above the body's own volume, so it removes nothing.
pub fn machine_motion(p: &Palette, phase: u32) -> VoxModel {
    // 34 voxels tall to hold 33 voxels of dome and spider, and the EVEN
    // number is the point rather than the spare row: see this section's
    // rule 4.
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS, 34]);
    let c = (16, 16);
    m.cylinder_z(c, 2.6, (30, 32), p.idx("brass"));
    for (u, v) in arm_cells(c, (2.6, 7.5), 4, 1.6, spin(phase, 4)) {
        m.cuboid((u, u), (v, v), (30, 31), p.idx("steel_light"));
    }
    m
}

/// The crusher's moving parts: the two flywheels' spokes and the teeth on
/// the two crushing rolls, all four on the same two shafts and therefore at
/// the same two angles.
///
/// **The rolls counter-rotate**, which is what a twin-roll crusher does and
/// the reason both wheels are worth animating at all: two wheels turning the
/// same way read as one wheel drawn twice. `sense` is that mirror.
///
/// The spokes stand at y = 1..2, one voxel PROUD of the bright wheel face
/// the body keeps at y = 3..4, so the dark arms read against it. This is the
/// camera-facing side, where UI_UX.md §2b point 11's value-contrast rule
/// applies: an unlit face carries its own albedo or it carries nothing.
///
/// The teeth are cut only where the body's feed pocket is genuinely open
/// (z >= [`CRUSHER_POCKET_FLOOR_Z`]). Below that the rolls are buried in the
/// crusher's own frame, so a tooth there would be a moving part inside a
/// solid — invisible, and z-fighting with the frame it is inside. Eight
/// teeth on a four-step shaft is deliberate: the teeth turn through half a
/// pitch a frame while the spokes turn through a quarter of theirs, which is
/// two features on one shaft rather than two shafts.
pub fn machine_crusher_motion(p: &Palette, phase: u32) -> VoxModel {
    let mut m = VoxModel::tile();
    for (x, sense) in [(12i32, 1.0f32), (20, -1.0)] {
        let angle = sense * spin(phase, 4);
        for (u, v) in arm_cells((x, 16), (CRUSHER_HUB_R, CRUSHER_SPOKE_R), 4, 1.7, angle) {
            m.cuboid((u, u), (1, 2), (v, v), p.idx("steel_dark"));
        }
        for (u, v) in arm_cells(
            (x, 16),
            (CRUSHER_ROLL_CORE_R, CRUSHER_ROLL_R),
            8,
            1.0,
            angle,
        ) {
            if v < CRUSHER_POCKET_FLOOR_Z {
                continue;
            }
            m.cuboid((u, u), CRUSHER_ROLL_Y, (v, v), p.idx("steel_light"));
        }
    }
    m
}

/// The mixer's moving part: the whole ribbon agitator, advanced a quarter of
/// its own period per frame.
///
/// It is the same helix `machine_mixer` used to draw inline, with [`spin`]
/// added to its phase — a ribbon blender's flights are two opposed screws,
/// so the pattern's period is half a turn and a quarter of that per frame
/// closes the loop (this section's rule 3). Turning the helix looks like
/// translating it along the drum, which is exactly what a ribbon does to the
/// batch inside; nothing needs to move along Y to say so.
///
/// Only the stretch under the drum's sight window is ever seen, but the
/// whole length is drawn: the window is a hole in a round vessel, and which
/// flights show through it changes with each of the four facings.
pub fn machine_mixer_motion(p: &Palette, phase: u32) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS * 2, 30]);
    let advance = spin(phase, MIXER_RIBBON_FLIGHTS);
    for y in MIXER_RIBBON_Y.0..=MIXER_RIBBON_Y.1 {
        // One full turn every 16 voxels; the ribbon is two opposed flights.
        let base = (y as f32) * std::f32::consts::TAU / 16.0 + advance;
        for half in [0.0f32, std::f32::consts::PI] {
            let a = base + half;
            let dx = (a.cos() * MIXER_RIBBON_R).round() as i32;
            let dz = (a.sin() * MIXER_RIBBON_R).round() as i32;
            m.cuboid(
                (MIXER_DRUM_X + dx - 1, MIXER_DRUM_X + dx + 1),
                (y, y),
                (MIXER_DRUM_Z + dz - 1, MIXER_DRUM_Z + dz + 1),
                p.idx("steel_dark"),
            );
        }
    }
    m
}

/// The vat's moving part: the agitator's drive coupling, standing on the
/// gearbox at the top of the tallest machine in the factory.
///
/// The vat is the one kind whose mechanism is entirely INSIDE it — a
/// jacketed vessel with an agitator in the sauce — so the honest thing to
/// animate is the drive that turns it, and the top of the gearbox is where a
/// real one couples. It is also, on this camera, the most visible square
/// inch of the whole machine: the vat is 48 voxels tall and the player is
/// looking down at it.
///
/// Nothing is removed from the body for it. The coupling sits at z >= 48,
/// above the body's own 48-voxel volume, in a volume 8 voxels taller — see
/// this section's rule 4 for why that still lands on the ground correctly.
pub fn machine_cooking_vat_motion(p: &Palette, phase: u32) -> VoxModel {
    let mut m = VoxModel::new([TILE_VOXELS * 2, TILE_VOXELS * 2, 56]);
    let c = (TILE_VOXELS as i32, TILE_VOXELS as i32);
    m.cylinder_z(c, 4.0, (48, 51), p.idx("brass"));
    for (u, v) in arm_cells(c, (4.0, 11.0), 4, 1.8, spin(phase, 4)) {
        m.cuboid((u, u), (v, v), (48, 49), p.idx("steel_light"));
    }
    m
}

/// The bottler's moving parts: the two filling nozzles and the capping
/// chuck, plunging together.
///
/// **A plunge, not a spin**, and that is the camera's call rather than the
/// machine's. A rotary chuck the size of this one (radius 3.2) turns through
/// a voxel and a half per frame, which at scene scale is a shimmer; the same
/// parts dropping two voxels onto a bottle mouth is unmistakable, and it is
/// what a bottling line visibly does. [`BOTTLER_PLUNGE`] is a there-and-back
/// table, so the cycle closes like a rotation does.
///
/// The chuck is an ANNULUS around the fixed spindle the body hangs off the
/// beam, so it slides down over the bottle's neck without either solid ever
/// sharing a voxel with the other — the same core-and-annulus cut the
/// crusher's rolls use. `arm_cells` with one arm 64 voxels wide is a full
/// ring: the count/thickness pair says "every cell of the annulus" without a
/// second primitive that would mean exactly this one.
pub fn machine_bottler_motion(p: &Palette, phase: u32) -> VoxModel {
    let mut m = VoxModel::tile();
    let drop = BOTTLER_PLUNGE[phase as usize];
    // The two filling nozzles, hanging off the gantry beam over the first
    // two bottles. They lengthen rather than detach: the top stays on its
    // hanger at z = 24 and the tip reaches down.
    for x in BOTTLER_FILL_X {
        m.cylinder_z(
            (x, 16),
            BOTTLER_NOZZLE_R,
            (22 + drop, 24),
            p.idx("steel_light"),
        );
    }
    // The capping chuck, riding the body's spindle down over the last
    // bottle's neck.
    for (u, v) in arm_cells(
        (BOTTLER_CAP_X, 16),
        (BOTTLER_SPINDLE_R, BOTTLER_CHUCK_R),
        1,
        64.0,
        0.0,
    ) {
        m.cuboid((u, u), (v, v), (21 + drop, 22 + drop), p.idx("brass"));
    }
    m
}

/// The packer's moving part: the taping carriage — the head and the tape
/// roll it carries — running down the case's seam and back.
///
/// A packing station is a table, so there is no wheel on it to turn; what it
/// does is drive a taping head along a case. [`PACKER_TRAVEL`] is the
/// carriage's X offset per frame, three voxels a step and back again, which
/// is the largest, plainest motion of any machine on the line and belongs on
/// the shortest machine — the packer has no silhouette to carry a subtler
/// one.
///
/// The roll is narrower and lower than the one it replaces (four voxels of
/// width at z 21..25, not seven at 21..27) for one reason: at the far end of
/// its travel the old roll passed straight through both the stand beside it
/// and the beam above it.
pub fn machine_packer_motion(p: &Palette, phase: u32) -> VoxModel {
    let mut m = VoxModel::tile();
    let dx = PACKER_TRAVEL[phase as usize];
    m.cuboid((16 + dx, 18 + dx), (18, 25), (22, 24), p.idx("steel_dark"));
    m.cylinder_y(
        (19 + dx, PACKER_ROLL_Z),
        PACKER_ROLL_R,
        (19, 22),
        p.idx("brass"),
    );
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
    m.hazard_stripe_x(
        (0, last),
        (0, 0),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    m.hazard_stripe_x(
        (0, last),
        (last, last),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    for y in 0i32..=last {
        let a = if (y + 2).rem_euclid(6) < 3 {
            p.idx("hazard_yellow")
        } else {
            p.idx("hazard_black")
        };
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
    m.ellipsoid(
        (c, c, 30),
        (TANK_OUTER_R, TANK_OUTER_R, 4.0),
        p.idx("steel"),
    );
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
    m.carve_cuboid(
        (c - 4, c + 4),
        (0, c - 6),
        (TANK_FILL_Z0 + 1, TANK_FILL_Z0 + TANK_FILL_HEIGHT + 1),
    );

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
    m.cuboid(
        (22, last),
        (c - 1, c + 1),
        (PIPE_AXIS_Z - 6, PIPE_AXIS_Z - 3),
        p.idx("steel_dark"),
    );

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
    m.cylinder_z(
        (TANK_CENTER, TANK_CENTER),
        TANK_INNER_R,
        (0, 0),
        p.idx("fluid_pale"),
    );
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
    m.hazard_stripe_x(
        (0, last),
        (0, 0),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    m.hazard_stripe_x(
        (0, last),
        (last, last),
        (2, 2),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );

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
    m.cuboid(
        (c - outer, c + outer),
        (c - outer, c + outer),
        (HOPPER_FILL_Z0, 30),
        p.idx("steel"),
    );
    m.carve_cuboid(
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (c - HOPPER_INNER_HALF, c + HOPPER_INNER_HALF),
        (HOPPER_FILL_Z0, 31),
    );
    // Flared loading lip, and hazard striping on the camera-facing wall.
    m.cuboid(
        (c - outer - 1, c + outer + 1),
        (c - outer - 1, c + outer + 1),
        (30, 30),
        p.idx("steel_light"),
    );
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
// Warehouse rack (the warehouse-racks sprint, ticket W2)
// ---------------------------------------------------------------------------
//
// The building that decides how much raw stock the player may own. Three
// models, and the split between them is the one `hopper`/`hopper_fill`
// already uses, for the same reason: the vessel is fixed geometry, what is
// *in* it is live simulation state that Godot scales and tints.
//
//   * `warehouse_rack`        — the racking itself, four empty bays.
//   * `warehouse_pallet`      — the pallet standing in a bay that is BOUND
//                               to an ingredient. Its presence alone is
//                               "this bay is in use".
//   * `warehouse_pallet_load` — one voxel of near-white goods on that
//                               pallet, scaled to the bay's fill fraction
//                               and tinted per ingredient, exactly the way
//                               `hopper_fill` works.
//
// **Why an open frame rather than a cabinet with four doorways.** Every
// other placed kind in this game is a closed housing, so the carved-opening
// rule (UI_UX.md §2b point 11) applies to it: cut the doorway through +X,
// because that is one of the only two faces this camera ever sees. A rack
// cannot take that deal. Point 18 is the reason — a rotated placement
// reuses the same mesh spun about its footprint origin, so a doorway carved
// through +X points *away* from the viewer at 180°, and the one kind whose
// entire job is to be read from the outside would go blank at half its
// facings.
//
// Real selective pallet racking is already the answer: two-post frames tied
// across the depth, rails spanning between them, and NO back, no sides and
// no doors at all. The bays are then empty from every direction, which
// satisfies point 11's "an opening must be empty, not dark" trivially and
// survives all four facings without a single overlay. It is also the one
// silhouette on this floor that is not a solid box — a see-through frame
// among a dozen closed housings — so a rack is tellable from a machine at
// play zoom by its shape before any of its colour is resolved.
//
// **Why the goods sit on a pallet rather than in a tote.** The first cut
// put an open-topped stacking crate in each bay and filled it like a
// hopper, and the render killed it: a tote wide enough to be worth drawing
// is nearly as wide as its bay, so its own walls stand between this camera
// and the one thing in the bay that carries meaning. The rack came back as
// a grey box with slots in it — every advantage the open frame had just
// bought, spent on four containers. Palletised goods are what is actually
// in a rack, and they put the product where the sight line already is:
// **the tinted load IS the readout**, and the pallet under it is a dark
// two-voxel plinth that says the bay is bound even when it is nearly empty.
//
// **Why two bays of three levels.** A rack is six slots, and there are two
// ways to spend them. The first cut spent four of them in ONE ROW, because
// this camera punishes anything standing above a bay: a sight line leaves
// every voxel along `(+x, -y, +z)`, so a beam at the front of an upper
// level hides a band of whatever is twelve voxels below and behind it. That
// version measured well and **looked like a bike rack** — five evenly
// spaced hoops with nothing spanning them, which is what racking is not.
// The horizontal load beams running the full length of the run are the one
// feature that says "warehouse" before anything else is resolved, and a row
// of unconnected frames has none.
//
// So the beams are back and the levels with them, and the camera is paid
// off with spacing rather than by deletion:
//
//   * every pair in [`RACK_BEAM_Z`] clears the load below it by three
//     voxels, which is what puts the whole of that load's FRONT face — the
//     face this camera reads a fill level from — below the beam that would
//     otherwise cross it.
//   * [`RACK_BRACE_Z`] sits above each level's goods for the same reason,
//     not across them.
//   * The upper deck stops at [`RACK_BAY_X`]`.1`, short of the front beam,
//     so it never overhangs the goods below. It eats the back of the lower
//     load's top face and none of its front.
//   * Nothing at all stands above the top level. Its beams are the ones it
//     sits on, which is how real selective racking is built anyway.
//
// **Height is not capped by the footprint** — see [`RACK_HEIGHT`]. The first
// two-level cut assumed `footprint x 32` bounded z as well and squeezed both
// levels into 30 voxels, which left a rack with the proportions of a coffee
// table. The rule only ever governed the footprint plane.
//
// The earlier one-row version is in the history of this file if the two
// levels ever stop reading; what is NOT worth retrying is a rack with no
// longitudinal beams at all.
//
// **Material** (§2b point 8, structure is painted, food contact is
// stainless): a rack touches nothing, so it is almost all paint. The
// uprights AND the load beams are both `paint_blue`, the same as the machine
// skirts, because a rack is factory equipment and not a second visual
// family. Decking is steel because that is what a deck is. The column guards
// are hazard striped — a real rack in a plant where powered pallet trucks
// drive has exactly those.
//
// **The beams were `paint_red` #82372F until the project owner's
// 2026-09-21 call to take red off this model**, and the render says why it
// was worth taking: the warehouse row's loads are tinted stock, the stock a
// player holds most of is tomatoes, and a red beam under a red load put the
// model's one readout and its structure in the same colour family.
//
// **The beams are not a second colour at all now** — the owner's follow-up
// call the same day, after a green cut was rendered: one paint for the whole
// structure. A two-tone rack is what real racking is sold in, so the case
// for it was never invented, but the case AGAINST spending a hue here is the
// stronger one on this kind: a rack's entire readout is the tinted stock in
// its bays (UI_UX.md §7), and every painted colour on the frame is one more
// thing competing with it. What used to be carried by hue is carried by
// shading instead, and the isometric key light is what makes that work: a
// beam's lit top face and the upright's two vertical faces are different
// planes, so the same paint comes back at different values. Measured on the
// warehouse row, a beam's top edge reads NN.N against the upright beside it
// at NN.N — MEASURED, see palette.ron. Do not "restore" a contrasting beam
// colour without rendering it beside this: the reason this holds is the
// light, not the paint.

/// The rack's upright frames: THREE of them along the model's long (Y)
/// axis, each a pair of posts spanning the depth, with a bay in each of the
/// two gaps between them.
///
/// The end frames sit flush with the footprint edge (y = 0 and y = 63) for
/// `machine_footing`'s reason — a structure inset from its own tile leaves a
/// hairline of bare floor where the neighbouring tile butts up. Three frames
/// of four and two bays of twenty-six is 64 exactly, which is what makes the
/// bays **identical** so that one pallet model serves all of them.
const RACK_FRAME_Y: [(i32, i32); 3] = [(0, 3), (30, 33), (60, 63)];
const RACK_POST_X: [(i32, i32); 2] = [(2, 5), (26, 29)];

/// The two bays' Y spans, i.e. the gaps between consecutive frames, and the
/// X span every bay shares (between the posts). Pinned as constants rather
/// than recomputed because [`warehouse_pallet`] is sized and centred from
/// them, and because the tests assert the bays are empty against them.
// The four constants below are the WIRE BETWEEN W2 AND W3 and have no
// consumer inside this generator: `main.gd` positions a bay's pallet and
// scales its load against them, and this file's tests assert the geometry
// holds them true. `dead_code` therefore fires on the binary build and is
// silenced here rather than by deleting the numbers, which would move them
// into GDScript — the thing `DOCK_DECK_Z`'s comment exists to prevent.
#[allow(dead_code)]
const RACK_BAY_Y: [(i32, i32); 2] = [(4, 29), (34, 59)];
#[allow(dead_code)]
const RACK_BAY_X: (i32, i32) = (6, 25);

/// The z a pallet's base stands on, per LEVEL — the plinth for the bottom
/// level, a deck laid over each beam pair for the two above it. Two bays
/// times three levels is six slots per rack.
///
/// **The pitch is fifteen and every level gets the same three voxels of
/// clearance**, which is the geometry the first two-level rack measured
/// best on (issue #144: the level with three voxels over it read 27-38/255,
/// the one with a brace across its goods read 16). Three levels at the
/// roomier nineteen would stand 58 voxels tall and out-top the cooking vat;
/// see [`RACK_HEIGHT`].
///
/// **`main.gd` places pallets against these numbers**, together with
/// [`RACK_BAY_Y`] above: a bound slot's pallet is drawn at the rack origin,
/// offset by its bay's centre in X/Y, lifted to its level, with the load
/// scaled on top of it. They are the wire between W2 and W3 — change one,
/// change both.
#[allow(dead_code)]
const RACK_LEVEL_Z: [i32; 3] = [2, 17, 32];

/// The longitudinal load beams, one pair per level ABOVE the bottom one,
/// and the deck slice laid across each pair.
///
/// **The three voxels between the load below and `.0` are the whole
/// design.** A front beam hides what is twelve voxels below and behind it,
/// so a beam any lower would lay a band across the front face of that
/// level's goods — which is the face a fill level is read from, and the
/// defect that killed the first two-level cut.
const RACK_BEAM_Z: [(i32, i32); 2] = [(14, 16), (29, 31)];

/// The depth braces that make a pair of posts an upright FRAME. Horizontal,
/// never diagonal — §2b point 23: a thin diagonal does not survive this
/// camera, and racking's real lattice bracing is exactly the thing that
/// would vanish into noise here. Two heights, one per level, so an empty
/// rack still reads as braced rather than as bare poles.
///
/// **Each brace sits in the clear air ABOVE its level's goods**, not across
/// them. They used to sit mid-bay, where the front frame's brace laid a
/// diagonal band over the stock behind it — measurably so: the level whose
/// goods the old brace crossed read 16/255 against the clear level's 27
/// (issue #144). Up here they cost the readout nothing.
///
/// The first two are at their beam pair's own height, so the brace between a
/// frame's posts and the beams outside them make ONE continuous line across
/// the rack's full width at every level. The third is the top tie, above the
/// goods on the top level, which has no beams over it.
const RACK_BRACE_Z: [(i32, i32); 3] = [(14, 15), (29, 30), (42, 43)];

/// The posts' top voxel, the cap plate above them, and the model's own
/// height.
///
/// **A model is only bound to `footprint x 32` in the footprint PLANE** —
/// height is free, and the line already spends it: `fluid_tank` is 1x1 and
/// 40 tall, `machine_cooking_vat` 2x2 and 48. Racking that stops at one tile
/// reads as shelving, so this stands at 45 occupied, three levels of it.
///
/// **48 is the ceiling, and it is the cooking vat's.** The vat is the thing
/// meant to be the biggest object on the line (see [`machine_cooking_vat`])
/// and a store room should not out-tower the process — a test pins the rack
/// under the vat's real model height rather than a literal. That ceiling is
/// what fixes the level pitch at fifteen: three levels at the two-level
/// rack's roomier nineteen came to 58.
const RACK_POST_TOP_Z: i32 = 44;
const RACK_CAP_Z: i32 = 45;
const RACK_HEIGHT: u32 = 46;

/// A pallet's footprint and height, and how tall a full load stands on it.
///
/// The footprint is inset from [`RACK_BAY_X`]/[`RACK_BAY_Y`] at every edge
/// so a pallet never touches the posts it stands between.
/// [`RACK_LOAD_MAX_H`] is what `main.gd` scales a full slot to, and it is
/// capped to clear the beams above it under the sight-line rule — the
/// test pins that rather than the comment.
const RACK_PALLET_SIZE: [u32; 3] = [18, 22, 2];
#[allow(dead_code)]
const RACK_LOAD_MAX_H: i32 = 8;

/// **The luma a bay's tint may NOT have** — the answer to issue #144, and
/// the last piece of the W2/W3 wire. `main.gd` tints the near-white
/// [`warehouse_pallet_load`] slab per item the way it tints a hopper's fill;
/// a tint inside this band renders the goods at the value of the thing
/// behind them and a full bay becomes indistinguishable from an empty one.
///
/// **Measured, not judged.** At 82 px (a 1x2 at default zoom), against the
/// factory floor's own #9C9C90 rather than vengi's black, with one
/// difference mask per slot and the anchor at 26/255. The floor is the
/// point: an empty bay in game shows FLOOR, so a saturated tint is not too
/// dark to see — it reads dark-on-light, which is perfectly legible — and
/// what actually disappears is a tint that lands ON the floor's own 155.
///
/// The band was 55..175 until the bottom bay got its own deck (see
/// [`warehouse_rack`]); the plinth under it was the only backdrop on the
/// rack that was not floor-coloured. Worst-slot separation of the six
/// shipped hopper tints against the decked rack:
///
/// | item | tint | luma | worst |
/// |---|---|---|---|
/// | molasses | `#2A160E` | 25.7 | 84.3 |
/// | liquid_smoke | `#706430` | 98.8 | **23.1** |
/// | tomato | `#C22B21` | 74.4 | 44.1 |
/// | spice_mix | `#A8481C` | 89.2 | 31.5 |
/// | vinegar | `#AD6B23` | 115.8 | **9.1** |
/// | sugar | `#F2F0E6` | 239.7 | 70.0 |
///
/// So **four of the six may be reused verbatim and two may not**. W3's rack
/// table should darken those two rather than invent new hues — `vinegar`
/// `Color(0.461, 0.286, 0.093)` (luma 79, worst 39.8) and `liquid_smoke`
/// `Color(0.325, 0.290, 0.139)` (luma 73, worst 44.6), which is each one's
/// own colour at 0.68/0.74 value and still reads as amber and as smoke.
/// Do NOT edit `HOPPER_FILL_TINTS` to match: a hopper's fill is seen against
/// the hopper's own dark throat, not against the floor, and those two were
/// measured there (issues #80, #131).
///
/// **Identity is not this band's job.** The tints that clear it cluster
/// between 73 and 89 luma, so a player tells 40 units from 10 by the tint
/// and tells molasses from smoke by the bay's ICON. Spending the band on
/// mutual discriminability as well would leave nothing for fill level,
/// which is the readout the rack exists for (plan requirement R3).
#[allow(dead_code)]
const RACK_TINT_DEAD_BAND: (u8, u8) = (100, 175);

/// `data/machines.ron`'s **1x2** `warehouse_rack` — placed storage, and the
/// only kind in the game that neither processes nor moves anything.
///
/// See this section's header for why it is an open frame rather than a
/// housing with carved openings, why it is two bays of three levels, and for
/// the material split. The geometry below is three upright frames
/// ([`RACK_FRAME_Y`]) braced across the depth, two pairs of load beams
/// running the rack's whole length at [`RACK_BEAM_Z`], and a deck over each
/// bay at each pair — which leaves the six volumes in [`RACK_BAY_X`] x
/// [`RACK_BAY_Y`] above each of [`RACK_LEVEL_Z`] genuinely empty, as this
/// file's tests check voxel by voxel rather than by eye.
pub fn warehouse_rack(p: &Palette) -> VoxModel {
    let tile = TILE_VOXELS as i32;
    let last_x = tile - 1;
    let last_y = tile * 2 - 1;
    let mut m = VoxModel::new([TILE_VOXELS, TILE_VOXELS * 2, RACK_HEIGHT]);

    // Plinth and hazard skirt, flush with the footprint, so a rack sits in
    // the same family as every machine (see `machine_footing_rect`) — but
    // WITHOUT that helper's painted base block, which would fill the
    // footprint solid and take the bays away.
    m.cuboid((0, last_x), (0, last_y), (0, 1), p.idx("concrete_dark"));
    for (y0, y1) in [(0, 0), (last_y, last_y)] {
        m.hazard_stripe_x(
            (0, last_x),
            (y0, y1),
            (2, 2),
            3,
            p.idx("hazard_yellow"),
            p.idx("hazard_black"),
        );
    }
    for y in 0..=last_y {
        let a = if (y + 2).rem_euclid(6) < 3 {
            p.idx("hazard_yellow")
        } else {
            p.idx("hazard_black")
        };
        m.set(0, y, 2, a);
        m.set(last_x, y, 2, a);
    }

    // The load beams: one pair running the rack's whole length, in the two
    // post planes, which is what turns three frames into a RUN of racking
    // rather than three things standing near each other.
    for &(x0, x1) in &RACK_POST_X {
        for &span in &RACK_BEAM_Z {
            // Solid `paint_blue` all the way through, INCLUDING the top
            // face — and no lighter cap even though the beam is now the
            // upright's own colour and a cap is the obvious way to separate
            // them. The first cut capped each beam with a bright `steel_light`
            // slice to carry the line down the rack, and that slice is square
            // to the light: it rendered as two white bars running the full 64
            // and became the loudest thing in the frame, with the beams
            // reading as steel and the paint invisible beneath them. The
            // beam's own lit top face is already the line; it does not need
            // help.
            m.cuboid((x0, x1), (0, last_y), span, p.idx("paint_blue"));
        }
    }

    // The upper level's decking, per bay, laid across the beams and
    // stopping SHORT of the front beam (`RACK_BAY_X.1` = 25, the beam
    // starts at 26). A deck that overhung its beam would put its own edge
    // across the goods below — the whole reason the first two-level rack
    // failed.
    for &(y0, y1) in &RACK_BAY_Y {
        for &span in &RACK_BEAM_Z {
            m.cuboid(RACK_BAY_X, (y0, y1), (span.1, span.1), p.idx("steel_dark"));
        }
        // And one over the plinth, under the BOTTOM level. Measured, not
        // decorative: with the bottom bay floored in `concrete_dark` the
        // goods on it had a dark backdrop where the levels above have the
        // factory floor, so a dark tint separated by 86/255 up top and 17
        // down here — the bottom level alone widened the unusable tint band
        // to 55..175 luma (issue #144). Decking it in the same steel as the
        // other levels makes all six slots behave alike.
        m.cuboid(RACK_BAY_X, (y0, y1), (1, 1), p.idx("steel_dark"));
    }

    // The three upright frames.
    for &(y0, y1) in &RACK_FRAME_Y {
        for &(x0, x1) in &RACK_POST_X {
            m.cuboid(
                (x0, x1),
                (y0, y1),
                (2, RACK_POST_TOP_Z),
                p.idx("paint_blue"),
            );
            // Column guard: the striped foot every upright in a plant with
            // powered pallet trucks in it carries.
            m.hazard_stripe_x(
                (x0, x1),
                (y0, y1),
                (3, 7),
                3,
                p.idx("hazard_yellow"),
                p.idx("hazard_black"),
            );
            // Post cap — a post has to END in something, or it reads as
            // sawn off. Its own footprint only: an earlier cut ran one
            // bright plate right across each frame and those bars became
            // the loudest thing in the render, a rack wearing a hat.
            m.cuboid(
                (x0, x1),
                (y0, y1),
                (RACK_CAP_Z, RACK_CAP_Z),
                p.idx("steel_dark"),
            );
        }
        // The depth braces between a frame's two posts.
        for &z in &RACK_BRACE_Z {
            m.cuboid(RACK_BAY_X, (y0, y1), z, p.idx("paint_blue"));
        }
    }

    // A bay-identification header across the top of the -Y end frame, which
    // is the face this camera sees and where a real rack carries its
    // location signage. A rack has no controls to put there, and an end
    // frame with nothing on it at all reads as scaffolding rather than as a
    // piece of the factory. Above the top level's goods, so it costs the
    // readout nothing.
    // Small and dark-bodied on purpose: at bay width in `steel_light` it
    // was a white plate the size of a pallet, and the rack grew a second
    // subject. A `paint_blue` board with one bright line of `steel_light`
    // across it reads as signage at any zoom that can resolve it and as
    // part of the frame at any zoom that cannot.
    let sign = (RACK_BAY_X.0 + 4, RACK_BAY_X.1 - 4);
    m.cuboid(
        sign,
        (0, 0),
        (RACK_POST_TOP_Z - 3, RACK_POST_TOP_Z),
        p.idx("paint_blue"),
    );
    m.cuboid(
        (sign.0 + 1, sign.1 - 1),
        (0, 0),
        (RACK_POST_TOP_Z - 1, RACK_POST_TOP_Z - 1),
        p.idx("steel_light"),
    );

    m
}

/// The pallet standing in a **bound** slot — the half of the rack's readout
/// that says "this slot is in use at all", where [`warehouse_pallet_load`]
/// says how full it is.
///
/// Its own volume, not the rack's, so `main.gd` instantiates one per bound
/// slot and positions it from [`RACK_BAY_Y`] and [`RACK_LEVEL_Z`]; the .vox
/// origin convention puts this model's pivot at its footprint centre at
/// ground level, so a slot's transform is the bay centre offset with no
/// further arithmetic.
///
/// Two voxels tall and deliberately dark. It is the one part of a bound
/// slot that does not change with the stock level, so it has to be legible
/// enough to say "bound" on its own and quiet enough that the tinted goods
/// standing on it are what the eye lands on. `tote_grey` feet under a
/// `steel_dark` deck: a plastic hygienic pallet, which is what a food plant
/// actually stores raw ingredients on, and not the timber one — UI_UX.md
/// §2b point 14 records what a second warm brown object costs on a floor
/// that already has `cardboard` cases on it.
pub fn warehouse_pallet(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new(RACK_PALLET_SIZE);
    let last_x = RACK_PALLET_SIZE[0] as i32 - 1;
    let last_y = RACK_PALLET_SIZE[1] as i32 - 1;
    m.cuboid((0, last_x), (0, last_y), (1, 1), p.idx("steel_dark"));
    // Three runner feet under the deck, along the rack's length, which is
    // the one detail at this size that says "pallet" and not "board".
    for &(y0, y1) in &[
        (0, 2),
        (last_y / 2 - 1, last_y / 2 + 1),
        (last_y - 2, last_y),
    ] {
        m.cuboid((0, last_x), (y0, y1), (0, 0), p.idx("tote_grey"));
    }
    m
}

/// The goods standing on a pallet: ONE voxel of near-white bulk that
/// `main.gd` scales vertically to the slot's fill fraction and multiplies a
/// per-item tint over, exactly as `hopper_fill` is used.
///
/// `bulk_pale` rather than a colour, because *which* ingredient a slot holds
/// is simulation state and a tint multiplies cleanly over near-white without
/// reading as a painted block. Inset a voxel from the pallet's own deck at
/// every edge, which is §2b point 19 — a body flush with what it stands on
/// z-fights, and it is the defect you only ever see once there is product
/// in the thing.
pub fn warehouse_pallet_load(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([RACK_PALLET_SIZE[0], RACK_PALLET_SIZE[1], 1]);
    let last_x = RACK_PALLET_SIZE[0] as i32 - 1;
    let last_y = RACK_PALLET_SIZE[1] as i32 - 1;
    m.cuboid((1, last_x - 1), (1, last_y - 1), (0, 0), p.idx("bulk_pale"));
    for x in [1, last_x - 1] {
        for y in [1, last_y - 1] {
            m.erase(x, y, 0);
        }
    }
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
    m.cuboid(
        (1, last - 1),
        (1, last - 1),
        (2, deck - 1),
        p.idx("concrete"),
    );
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
    m.hazard_stripe_x(
        (0, last),
        (0, 0),
        (deck - 1, deck),
        3,
        p.idx("hazard_yellow"),
        p.idx("hazard_black"),
    );
    for y in 0..=last {
        let a = if (y + 2).rem_euclid(6) < 3 {
            p.idx("hazard_yellow")
        } else {
            p.idx("hazard_black")
        };
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
        m.cuboid(
            (last - 1, last),
            (y0, y1),
            (deck - 6, deck - 1),
            p.idx("rubber_dark"),
        );
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
    m.cuboid(
        (DOCK_WALL_WEST_X, last),
        (wall_face_y, last),
        (deck + 1, 27),
        p.idx("paint_blue"),
    );
    for x in (DOCK_WALL_WEST_X + 2..=62).step_by(4) {
        m.cuboid(
            (x, x),
            (wall_face_y, wall_face_y),
            (deck + 1, 25),
            p.idx("steel_dark"),
        );
    }

    // The despatch openings, carved through to a black back wall — real
    // holes, not dark rectangles painted on the wall (see this file's
    // junction tests for why that distinction is checked rather than
    // eyeballed). Each opening's floor is what a sight line entering it
    // actually reaches (the infeed housing's doc derives that for the
    // mirrored case), so the floor is black and the frame around it is the
    // palette's brightest metal.
    for (x0, x1) in DOCK_DOOR_SPANS {
        m.cuboid(
            (x0 - 3, x1 + 3),
            (wall_face_y - 2, wall_face_y),
            (deck + 1, 23),
            p.idx("steel_light"),
        );
        m.carve_cuboid((x0, x1), (wall_face_y - 2, last - 1), (deck + 1, 21));
        m.cuboid(
            (x0, x1),
            (last, last),
            (deck + 1, 21),
            p.idx("hazard_black"),
        );
        m.cuboid(
            (x0, x1),
            (wall_face_y - 2, last - 1),
            (deck + 1, deck + 1),
            p.idx("hazard_black"),
        );
        // Brass threshold at the lip: the same accent every other fitting
        // product physically passes through carries.
        m.cuboid(
            (x0, x1),
            (wall_face_y - 2, wall_face_y - 2),
            (deck + 1, deck + 1),
            p.idx("brass"),
        );
        // The roller shutter, wound up above the opening — what says this
        // opening is a door rather than a window.
        m.cuboid(
            (x0 - 1, x1 + 1),
            (wall_face_y - 3, wall_face_y - 2),
            (22, 23),
            p.idx("steel_dark"),
        );
        m.cuboid(
            (x0 - 1, x1 + 1),
            (wall_face_y - 3, wall_face_y - 3),
            (22, 22),
            p.idx("steel_light"),
        );
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
    m.cuboid(
        canopy_x,
        (canopy_face_y, wall_face_y),
        (26, 27),
        p.idx("steel"),
    );
    m.cuboid(
        canopy_x,
        (canopy_face_y, canopy_face_y),
        (25, 25),
        p.idx("hazard_yellow"),
    );
    for x in [canopy_x.0, 37, 38, canopy_x.1] {
        m.cuboid(
            (x, x),
            (canopy_face_y + 1, wall_face_y),
            (24, 25),
            p.idx("steel_dark"),
        );
    }

    // A bay light under the canopy's leading edge above each door — hung one
    // voxel proud of it so the canopy itself doesn't shadow it out of the
    // frame. These are the dock's status lamps: `main.gd` swaps them green
    // when stock is waiting to be sold and amber when the deck is clear, the
    // same idle/running pair every machine's lamp uses. Both swap together,
    // which is correct — both doors are one dock, holding one stock.
    for (x0, x1) in DOCK_DOOR_SPANS {
        let mid = (x0 + x1) / 2;
        m.cuboid(
            (mid - 3, mid + 3),
            (canopy_face_y - 2, canopy_face_y - 1),
            (24, 25),
            p.idx("steel_light"),
        );
        m.cuboid(
            (mid - 2, mid + 2),
            (canopy_face_y - 2, canopy_face_y - 2),
            (23, 24),
            p.idx("status_amber"),
        );
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
// ---------------------------------------------------------------------------
// The title sign
// ---------------------------------------------------------------------------

/// The game's name as a bitmap, one string per glyph row, `#` lit. A chunky
/// two-pixel stroke rather than a 5x7 hairline face: the sign is read from
/// across a hall at a quarter of the screen's width, and a one-pixel stroke
/// at that size is a stencil, not a word. Widths vary (M and N are seven —
/// a six-wide N has no room for its diagonal and reads as an H), so each
/// glyph carries its own.
const TITLE_SIGN_WORD: &[&[&str; 8]] = &[
    &[
        ".####.", "##..##", "##....", "##....", "##....", "##....", "##..##", ".####.",
    ], // C
    &[
        ".####.", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", ".####.",
    ], // O
    &[
        "##...##", "###..##", "####.##", "##.####", "##..###", "##...##", "##...##", "##...##",
    ], // N
    &[
        "#####.", "##..##", "##..##", "##..##", "##..##", "##..##", "##..##", "#####.",
    ], // D
    &[
        "######", "..##..", "..##..", "..##..", "..##..", "..##..", "..##..", "######",
    ], // I
    &[
        "##...##", "###.###", "#######", "##.#.##", "##...##", "##...##", "##...##", "##...##",
    ], // M
    &[
        "######", "##....", "##....", "#####.", "##....", "##....", "##....", "######",
    ], // E
    &[
        "##...##", "###..##", "####.##", "##.####", "##..###", "##...##", "##...##", "##...##",
    ], // N
    &[
        "######", "..##..", "..##..", "..##..", "..##..", "..##..", "..##..", "..##..",
    ], // T
    &[
        ".####.", "##..##", "##..##", "##..##", "######", "##..##", "##..##", "##..##",
    ], // A
    &[
        "##....", "##....", "##....", "##....", "##....", "##....", "##....", "######",
    ], // L
];

/// Voxels per glyph pixel. Three keeps the whole word inside the `.vox`
/// format's 256-voxel ceiling with the frame around it.
const TITLE_SIGN_PX: i32 = 3;
/// Glyph pixels between letters.
const TITLE_SIGN_GAP: i32 = 1;
/// From the board's edge to the letters: the steel rim, the bulb ring and a
/// dark margin either side of the bulbs, so the chase never touches a letter.
const TITLE_SIGN_MARGIN: i32 = 9;
/// The rim's width, and where the bulb ring runs inside it.
const TITLE_SIGN_RIM: i32 = 3;
const TITLE_SIGN_BULB_INSET: i32 = 5;
/// Bulb pitch along the ring, and how many frames the chase takes: bulb `i`
/// is lit in frame `i % TITLE_SIGN_BULB_FRAMES`, which marches the light
/// round the frame one bulb per frame.
const TITLE_SIGN_BULB_PITCH: i32 = 6;
const TITLE_SIGN_BULB_FRAMES: usize = 3;
/// Depth: the board, then the letters standing proud of it. Built with the
/// face toward high Y and turned round at the end ([`title_sign_face_front`]):
/// the importer puts LOW vox Y toward world +Z, the title camera's side, and
/// a face built at high Y renders as the back of the sign — mirrored.
const TITLE_SIGN_BOARD_DEPTH: i32 = 6;
const TITLE_SIGN_LETTER_DEPTH: i32 = 4;
/// How far the ketchup-red block behind each letter is pushed down and
/// right — the drop shadow that makes the word read as solid rather than as
/// a decal, the way an old enamel sign's lettering does.
const TITLE_SIGN_SHADOW: i32 = 2;
/// The two hanging rods' length above the board, on the body itself. They
/// must run out of the top of the screen — the sign hangs from the roof, it
/// does not float — and how far that is depends on the window, so
/// `title_scene.gd` carries them on with [`title_sign_rods`] lengths.
const TITLE_SIGN_ROD: i32 = 60;
/// The rods' length in one `title_sign_rods` piece. The body's own
/// [`TITLE_SIGN_ROD`] only reaches the top of the frame when the sign's slot
/// sits near it, as it does in a 1280x720 window; in a taller one (a
/// fullscreen MacBook, 16:10) the menu column is centred lower and the rods
/// stopped in mid-air. `title_scene.gd` stacks as many of these above them
/// as the window needs, so the sign hangs from the roof at every size.
const TITLE_SIGN_ROD_SEGMENT: i32 = 32;

fn title_sign_word_width() -> i32 {
    let pixels: i32 = TITLE_SIGN_WORD.iter().map(|g| g[0].len() as i32).sum();
    (pixels + TITLE_SIGN_GAP * (TITLE_SIGN_WORD.len() as i32 - 1)) * TITLE_SIGN_PX
}

/// The board's outer size in voxels: (width, height).
fn title_sign_board() -> (i32, i32) {
    (
        title_sign_word_width() + TITLE_SIGN_MARGIN * 2,
        8 * TITLE_SIGN_PX + TITLE_SIGN_MARGIN * 2,
    )
}

/// Every model in the family shares this volume, so a bulb overlay lands on
/// the body exactly — the importer centres each on its own footprint.
fn title_sign_volume() -> VoxModel {
    let (w, h) = title_sign_board();
    VoxModel::new([
        w as u32,
        (TITLE_SIGN_BOARD_DEPTH + TITLE_SIGN_LETTER_DEPTH) as u32,
        (h + TITLE_SIGN_ROD) as u32,
    ])
}

/// The bulb positions round the ring, in chase order (clockwise from the
/// bottom-left corner), as the (x, z) of each 2x2 bulb's low corner.
fn title_sign_bulbs() -> Vec<(i32, i32)> {
    let (w, h) = title_sign_board();
    let (lo_x, hi_x) = (TITLE_SIGN_BULB_INSET, w - TITLE_SIGN_BULB_INSET - 2);
    let (lo_z, hi_z) = (TITLE_SIGN_BULB_INSET, h - TITLE_SIGN_BULB_INSET - 2);
    let mut out = Vec::new();
    let mut z = lo_z;
    while z < hi_z {
        out.push((lo_x, z));
        z += TITLE_SIGN_BULB_PITCH;
    }
    let mut x = lo_x;
    while x < hi_x {
        out.push((x, hi_z));
        x += TITLE_SIGN_BULB_PITCH;
    }
    let mut z = hi_z;
    while z > lo_z {
        out.push((hi_x, z));
        z -= TITLE_SIGN_BULB_PITCH;
    }
    let mut x = hi_x;
    while x > lo_x {
        out.push((x, lo_z));
        x -= TITLE_SIGN_BULB_PITCH;
    }
    out
}

/// The title screen's hanging sign: CONDIMENTAL in mustard-lit letters with
/// a ketchup drop shadow, on a dark board in a steel rim, with a hazard
/// stripe along its foot and two rods running up to the roof.
///
/// The bulbs are NOT here — they are cut out into the
/// `title_sign_bulbs_f*` overlays (§2b point 27's rule, borrowed from the
/// machine motion frames): every bulb is in every frame, lit or dark, so
/// the body never has a voxel the overlay also draws.
pub fn title_sign(p: &Palette) -> VoxModel {
    let mut m = title_sign_volume();
    let (w, h) = title_sign_board();
    let board_y = (0, TITLE_SIGN_BOARD_DEPTH - 1);
    let front = TITLE_SIGN_BOARD_DEPTH;

    // The rim, the dark face inside it, and the hazard stripe on its foot.
    m.cuboid((0, w - 1), board_y, (0, h - 1), p.idx("steel"));
    m.cuboid(
        (TITLE_SIGN_RIM, w - 1 - TITLE_SIGN_RIM),
        (0, TITLE_SIGN_BOARD_DEPTH),
        (TITLE_SIGN_RIM, h - 1 - TITLE_SIGN_RIM),
        p.idx("rubber_dark"),
    );
    m.carve_cuboid(
        (TITLE_SIGN_RIM, w - 1 - TITLE_SIGN_RIM),
        (front, front),
        (TITLE_SIGN_RIM, h - 1 - TITLE_SIGN_RIM),
    );
    m.hazard_stripe_x(
        (0, w - 1),
        board_y,
        (0, TITLE_SIGN_RIM - 1),
        3,
        p.idx("hazard_yellow"),
        p.idx("rubber_dark"),
    );

    // The rods, with a bracket where each meets the rim.
    for x in title_sign_rod_xs() {
        title_sign_rod(&mut m, p, x, (h, h + TITLE_SIGN_ROD - 1));
        m.cuboid((x - 3, x + 2), (1, 4), (h - 1, h + 2), p.idx("steel"));
    }

    // The word: the red shadow block first, the lit face over it.
    let base_z = TITLE_SIGN_MARGIN;
    let mut gx = TITLE_SIGN_MARGIN;
    for glyph in TITLE_SIGN_WORD {
        for (row, line) in glyph.iter().enumerate() {
            let z0 = base_z + (7 - row as i32) * TITLE_SIGN_PX;
            for (col, ch) in line.chars().enumerate() {
                if ch != '#' {
                    continue;
                }
                let x0 = gx + col as i32 * TITLE_SIGN_PX;
                m.cuboid(
                    (
                        x0 + TITLE_SIGN_SHADOW,
                        x0 + TITLE_SIGN_SHADOW + TITLE_SIGN_PX - 1,
                    ),
                    (front, front + TITLE_SIGN_LETTER_DEPTH - 2),
                    (
                        z0 - TITLE_SIGN_SHADOW,
                        z0 - TITLE_SIGN_SHADOW + TITLE_SIGN_PX - 1,
                    ),
                    p.idx("ketchup_red"),
                );
            }
        }
        for (row, line) in glyph.iter().enumerate() {
            let z0 = base_z + (7 - row as i32) * TITLE_SIGN_PX;
            for (col, ch) in line.chars().enumerate() {
                if ch != '#' {
                    continue;
                }
                let x0 = gx + col as i32 * TITLE_SIGN_PX;
                m.cuboid(
                    (x0, x0 + TITLE_SIGN_PX - 1),
                    (front, front + TITLE_SIGN_LETTER_DEPTH - 1),
                    (z0, z0 + TITLE_SIGN_PX - 1),
                    p.idx("sign_neon"),
                );
            }
        }
        gx += (glyph[0].len() as i32 + TITLE_SIGN_GAP) * TITLE_SIGN_PX;
    }
    title_sign_face_front(&m)
}

/// The X of each hanging rod, shared by the body and [`title_sign_rods`] so
/// a stacked length lands exactly on the rod below it.
fn title_sign_rod_xs() -> [i32; 2] {
    let (w, _) = title_sign_board();
    [w / 5, w - 1 - w / 5]
}

/// One rod at `x`, over the vox-Z range `z` (before the face is turned
/// front — [`title_sign_face_front`]).
fn title_sign_rod(m: &mut VoxModel, p: &Palette, x: i32, z: (i32, i32)) {
    m.cuboid((x - 1, x), (2, 3), z, p.idx("steel_light"));
}

/// A [`TITLE_SIGN_ROD_SEGMENT`]-voxel length of both rods. The same X/depth
/// footprint as the body, so the importer centres it on the same origin and
/// it stacks straight up off the body's rod tops with no offset to compute.
pub fn title_sign_rods(p: &Palette) -> VoxModel {
    let (w, _) = title_sign_board();
    let mut m = VoxModel::new([
        w as u32,
        (TITLE_SIGN_BOARD_DEPTH + TITLE_SIGN_LETTER_DEPTH) as u32,
        TITLE_SIGN_ROD_SEGMENT as u32,
    ]);
    for x in title_sign_rod_xs() {
        title_sign_rod(&mut m, p, x, (0, TITLE_SIGN_ROD_SEGMENT - 1));
    }
    title_sign_face_front(&m)
}

/// Mirrors a sign-family model in vox Y, so the face built at high Y ends up
/// at low Y — the side the importer turns toward the title camera. A mirror
/// rather than a half turn, because a half turn would also reverse X and
/// spell the word backwards.
fn title_sign_face_front(m: &VoxModel) -> VoxModel {
    let mut out = VoxModel::new(m.size);
    let depth = m.size[1] as i32;
    for (&(x, y, z), &colour) in &m.voxels {
        out.set(x as i32, depth - 1 - y as i32, z as i32, colour);
    }
    out
}

/// One frame of the bulb chase: every bulb, the `frame`th of each run of
/// `TITLE_SIGN_BULB_FRAMES` lit and the rest dark. A bulb is a 2x2 lens
/// standing two voxels proud of the dark face.
fn title_sign_bulbs_frame(p: &Palette, frame: usize) -> VoxModel {
    let mut m = title_sign_volume();
    let front = TITLE_SIGN_BOARD_DEPTH;
    for (i, (x, z)) in title_sign_bulbs().into_iter().enumerate() {
        let colour = if i % TITLE_SIGN_BULB_FRAMES == frame {
            p.idx("marquee_bulb")
        } else {
            p.idx("marquee_bulb_off")
        };
        m.cuboid((x, x + 1), (front, front + 1), (z, z + 1), colour);
    }
    title_sign_face_front(&m)
}

pub fn title_sign_bulbs_f0(p: &Palette) -> VoxModel {
    title_sign_bulbs_frame(p, 0)
}

pub fn title_sign_bulbs_f1(p: &Palette) -> VoxModel {
    title_sign_bulbs_frame(p, 1)
}

pub fn title_sign_bulbs_f2(p: &Palette) -> VoxModel {
    title_sign_bulbs_frame(p, 2)
}

/// A puff of steam: three overlapping balls, pale, the lump off-centre for
/// the same reason [`crush_bit`]'s is — a perfect sphere of voxels reads as
/// a bead, a lump reads as vapour. Centred on the volume midline because it
/// is spun as it rises (§2b point 20).
pub fn steam_puff(p: &Palette) -> VoxModel {
    let mut m = VoxModel::new([12, 12, 12]);
    let c = 5.5;
    m.sphere_f((c, c, c), 4.2, p.idx("bulk_pale"));
    m.sphere_f((c + 2.0, c - 1.0, c + 1.5), 3.0, p.idx("cap_white"));
    m.sphere_f((c - 2.0, c + 1.5, c + 0.5), 2.8, p.idx("sugar_white"));
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::MaterialClass;

    fn palette() -> Palette {
        Palette::load()
    }

    /// Whether `m`, placed on a crossing tile, has any voxel in the box an
    /// item riding the belt occupies ([`CROSSING_ITEM_Y`] across the whole
    /// lane, up to [`CROSSING_ITEM_TOP_Z`]). `lift` stands `m` that many
    /// voxels higher first, the way `main.gd` stands a crossing's overlays.
    fn fouls_belt_traffic(m: &VoxModel, lift: i32) -> bool {
        m.voxels.keys().any(|&(_, y, z)| {
            let (y, z) = (y as i32, z as i32 + lift);
            (CROSSING_ITEM_Y.0..=CROSSING_ITEM_Y.1).contains(&y) && z <= CROSSING_ITEM_TOP_Z
        })
    }

    /// The crossing spike's finding (2026-09-25), as an assertion. An item
    /// rides the belt with its base on the running surface and its model
    /// centred on the tile, so a solid on a crossing tile fills
    /// [`CROSSING_ITEM_Y`] up to [`CROSSING_ITEM_TOP_Z`] — NOT the same box as
    /// the belt's own. The shipped crossing leaves it empty.
    ///
    /// The other half is the regression guard. The spike proved the lift was
    /// needed with a rejected `pipe_bridge_flat` model; that model was the
    /// ordinary straight turned across the lane on two posts, so the straight
    /// turned across the lane is the same witness without keeping a model
    /// nothing ships in `ASSETS`. If it ever stops fouling the box, the belt or
    /// the items changed and the gantry may no longer be needed.
    #[test]
    fn a_crossing_clears_what_the_belt_is_carrying() {
        let p = palette();
        assert!(
            !fouls_belt_traffic(&pipe_crossing(&p), 0),
            "the crossing stands in the space an item on the belt occupies"
        );
        assert!(
            fouls_belt_traffic(&pipe(&p).rotated_cw(1), 0),
            "a pipe left at the run's own height no longer fouls the item box — \
             the belt or the items changed and the gantry may be unneeded"
        );
        // The bottle is the tallest item, and the box has to hold it.
        let bottle_top = item_bottle(&p)
            .voxels
            .keys()
            .map(|&(_, _, z)| z as i32)
            .max();
        assert_eq!(
            bottle_top.map(|z| z + BELT_CLEAT_Z),
            Some(CROSSING_ITEM_TOP_Z),
            "the tallest item's top moved; CROSSING_ITEM_TOP_Z must follow it"
        );
    }

    /// [`CROSSING_ITEM_Y`] and [`CROSSING_ITEM_TOP_Z`] are hand-written
    /// numbers claiming to be facts about the item models, and the crossing's
    /// compile-time clearance asserts rest on them. This makes the claim
    /// true: every `item_*` model in `ASSETS`, stood the way `main.gd` stands
    /// a carried item (centred on the tile, base on the running surface),
    /// must fill no voxel outside that box.
    ///
    /// It walks `ASSETS` rather than naming the belt items, so a new item is
    /// covered the moment it ships (the issue #108 shape was a guard holding
    /// a list of what existed when it was written). The exemptions go the
    /// other way: a model that never rides a belt has to be named here, with
    /// its reason.
    #[test]
    fn a_crossing_clears_every_item_the_line_can_carry() {
        // `main.gd`'s FLUID_MODELS: stock icons for the market and the larder
        // that nothing carries on a belt. The vinegar jug stands at z = 25.
        const NEVER_ON_A_BELT: [&str; 2] = ["item_vinegar", "item_liquid_smoke"];
        let p = palette();
        let inset = (TILE_VOXELS - ITEM_VOXELS) as i32 / 2;
        let items: Vec<_> = ASSETS
            .iter()
            .filter(|a| a.name.starts_with("item_"))
            .collect();
        for name in NEVER_ON_A_BELT {
            assert!(
                items.iter().any(|a| a.name == name),
                "{name} is exempted but no longer ships; drop the exemption"
            );
        }
        let mut reach: Option<(i32, i32, i32)> = None;
        for a in items.iter().filter(|a| !NEVER_ON_A_BELT.contains(&a.name)) {
            for &(_, y, z) in (a.build)(&p).voxels.keys() {
                let (y, z) = (y as i32 + inset, z as i32 + BELT_CLEAT_Z);
                assert!(
                    (CROSSING_ITEM_Y.0..=CROSSING_ITEM_Y.1).contains(&y),
                    "{} fills y = {y} on the belt, outside CROSSING_ITEM_Y {:?} \
                     that the crossing is built to clear",
                    a.name,
                    CROSSING_ITEM_Y
                );
                assert!(
                    z <= CROSSING_ITEM_TOP_Z,
                    "{} reaches z = {z} on the belt, above CROSSING_ITEM_TOP_Z = {} \
                     that the crossing is built to clear",
                    a.name,
                    CROSSING_ITEM_TOP_Z
                );
                let (y0, y1, top) = reach.unwrap_or((y, y, z));
                reach = Some((y0.min(y), y1.max(y), top.max(z)));
            }
        }
        // And the other direction: the constants are the items' real extent,
        // not a loose bound, so the comments that quote them stay true.
        assert_eq!(
            reach,
            Some((CROSSING_ITEM_Y.0, CROSSING_ITEM_Y.1, CROSSING_ITEM_TOP_Z)),
            "the belt items no longer reach the edges of the crossing's box; \
             tighten CROSSING_ITEM_Y / CROSSING_ITEM_TOP_Z to match"
        );
    }

    /// The crossing and the belt it is stacked over are two meshes in one
    /// place, so they must never share a voxel — a shared voxel is a pair of
    /// coincident faces, z-fighting on every crossing in the factory. Every
    /// straight frame, since the cleats move.
    #[test]
    fn a_crossing_shares_no_voxel_with_the_belt_under_it() {
        let p = palette();
        let crossing = pipe_crossing(&p);
        for (frame, belt) in [belt_f0(&p), belt_f1(&p), belt_f2(&p), belt_f3(&p)]
            .iter()
            .enumerate()
        {
            for &(x, y, z) in belt.voxels.keys() {
                assert!(
                    !crossing.filled(x as i32, y as i32, z as i32),
                    "belt_f{frame} and the crossing share voxel ({x}, {y}, {z})"
                );
            }
        }
    }

    /// Open on exactly the two faces its pipe runs through, at the run's own
    /// axis — so it drops into a north–south run with no fitting — and closed
    /// on the two the belt runs through, which a pipe beside the belt must
    /// never join (the plan's open-sides mask).
    #[test]
    fn a_crossing_is_open_only_on_its_pipe_axis() {
        let p = palette();
        let crossing = pipe_crossing(&p);
        for face in ALL_FACES {
            let open = matches!(face, PipeFace::North | PipeFace::South);
            assert_eq!(
                face_open(&crossing, face),
                open,
                "pipe_crossing: face {face:?} should be {}",
                if open { "open" } else { "closed" }
            );
        }
    }

    /// Mirror-symmetric about [`PIPE_C`] on both horizontal axes (§2b point
    /// 20), so a crossing on a westbound belt — this model turned twice —
    /// lands on exactly the voxels an eastbound one does, and both of its
    /// joints meet the run without a step at any of the four facings.
    #[test]
    fn a_crossing_is_symmetric_about_the_tile_midline() {
        let p = palette();
        let last = TILE_VOXELS - 1;
        for (name, m) in [
            ("pipe_crossing", pipe_crossing(&p)),
            ("pipe_fluid_crossing", pipe_fluid_crossing(&p)),
        ] {
            for (&(x, y, z), &c) in &m.voxels {
                for (mx, my) in [(last - x, y), (x, last - y)] {
                    assert_eq!(
                        m.voxels.get(&(mx, my, z)),
                        Some(&c),
                        "{name}: ({x}, {y}, {z}) has no mirror at ({mx}, {my}, {z})"
                    );
                }
            }
        }
    }

    /// The crossing shows its liquid the way every other pipe tile does: a
    /// one-voxel slab that, stood [`CROSSING_LIFT`] up, lies inside the raised
    /// run's bore at every voxel, is seen through the sight glass, and has
    /// tube wall beside it all the way to a full column (§2b point 19).
    #[test]
    fn a_crossings_fluid_body_lies_in_its_raised_bore_under_the_sight_glass() {
        let p = palette();
        let (shape, fluid) = (pipe_crossing(&p), pipe_fluid_crossing(&p));
        assert_eq!(fluid.size[2], 1, "the fluid body must be one voxel tall");
        assert!(
            !fluid.voxels.is_empty(),
            "the crossing carries no fluid body"
        );
        let floor = PIPE_BORE_Z0 + CROSSING_LIFT;
        for &(x, y, _) in fluid.voxels.keys() {
            let (x, y) = (x as i32, y as i32);
            // The family test's band, lifted: the bore is empty above the
            // slab up to the axis. Higher, a column at an elbow meets the
            // casting's round crown — wall around liquid, which hides it.
            for z in floor + 1..=GANTRY_AXIS_Z {
                assert!(
                    !shape.filled(x, y, z),
                    "fluid at ({x}, {y}) is inside the crossing's wall at z = {z}"
                );
            }
            for z in floor..floor + PIPE_BORE_HEIGHT {
                // Walled in along X outside the sight glass: a side face that
                // met open air there would be the old coloured-stripe bug on
                // the raised run. Inside it, being seen is the point.
                if (GANTRY_WINDOW.0..=GANTRY_WINDOW.1).contains(&y) {
                    continue;
                }
                for step in [-1, 1] {
                    let walled = (1..TILE_VOXELS as i32).any(|d| shape.filled(x + step * d, y, z));
                    assert!(
                        walled,
                        "a full column's side face at ({x}, {y}, {z}) faces open air"
                    );
                }
            }
        }
        let seen = fluid.voxels.keys().any(|&(x, y, _)| {
            (PIPE_BORE_Y.0..=PIPE_BORE_Y.1).contains(&(x as i32))
                && (GANTRY_WINDOW.0..=GANTRY_WINDOW.1).contains(&(y as i32))
        });
        assert!(seen, "no fluid lies under the crossing's sight glass");
        for y in [0, TILE_VOXELS as i32 - 1] {
            assert!(
                !(0..TILE_VOXELS as i32).any(|x| fluid.filled(x, y, 0)),
                "the raised slab reaches the tile face at y = {y}, where there is no \
                 raised tube to hold it"
            );
        }
    }

    /// The fault clamp is overlaid on a crossing too — a crossing's pipe is a
    /// pipe tile of its run like any other — and `main.gd` lifts it by the
    /// same [`CROSSING_LIFT`] as the liquid. Lifted, it must still cover the
    /// raised tube it marks and must no longer stand in the belt's traffic,
    /// or a faulted run would stop the line it crosses.
    #[test]
    fn a_lifted_fault_clamp_covers_a_crossing_and_clears_its_belt() {
        let p = palette();
        let clamp = pipe_fault(&p);
        assert!(
            fouls_belt_traffic(&clamp, 0),
            "the clamp at run height no longer fouls the lane, so a crossing may \
             not need it lifted at all"
        );
        assert!(
            !fouls_belt_traffic(&clamp, CROSSING_LIFT),
            "the fault clamp lifted onto a crossing stands in the belt's traffic"
        );
        let crossing = pipe_crossing(&p);
        let lifted = |x: i32, y: i32, z: i32| clamp.filled(x, y, z - CROSSING_LIFT);
        let mut covered = 0;
        for &(x, y, z) in crossing.voxels.keys() {
            let (x, y, z) = (x as i32, y as i32, z as i32);
            // The raised span's tube, where the clamp straddles it.
            if z < GANTRY_AXIS_Z - PIPE_OUTER_R as i32 || !lifted(PIPE_C as i32, y, z) {
                continue;
            }
            assert!(
                lifted(x, y, z),
                "the raised tube at ({x}, {y}, {z}) pokes out of the lifted clamp"
            );
            covered += 1;
        }
        assert!(
            covered > 0,
            "the lifted clamp covers none of the raised tube"
        );
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
        assert!(
            m.filled(0, mid, deck),
            "the -X intake lane stops short of the tile edge"
        );
        assert!(
            m.filled(last, mid, deck),
            "the +X straight output stops short of the tile edge"
        );
        assert!(
            m.filled(mid, last, deck),
            "the +Y branch output stops short of the tile edge"
        );
        // ...and nothing stands in any of them.
        for z in (deck + 1)..=(deck + 4) {
            assert!(!m.filled(0, mid, z), "something blocks the intake lane");
            assert!(
                !m.filled(last, mid, z),
                "something blocks the straight output"
            );
            assert!(
                !m.filled(mid, last, z),
                "something blocks the branch output"
            );
        }
        // The fourth side is a rail, not an output. It is BELOW the deck, as
        // a belt's own side frame is, so it closes the tile without standing
        // in front of the table the camera has to see into.
        assert!(
            m.filled(mid, 5, 6),
            "the -Y side has no rail, so the tile reads as open on all four sides"
        );
        assert!(
            !m.filled(mid, 5, deck + 1),
            "the -Y rail stands above the deck and would occlude the table"
        );

        // Both arrow grooves are real recesses, one voxel down, and they do
        // not overlap — two arrows crossing at the tile centre render as one
        // unreadable blob (see `splitter_straight_arrow`).
        let straight: std::collections::BTreeSet<(i32, i32)> =
            splitter_arrow_cells(0).into_iter().collect();
        let branch: std::collections::BTreeSet<(i32, i32)> =
            splitter_arrow_cells(1).into_iter().collect();
        assert!(!straight.is_empty(), "the straight arrow is empty");
        assert_eq!(
            straight.len(),
            branch.len(),
            "a quarter turn must not change an arrow's size"
        );
        assert!(
            straight.is_disjoint(&branch),
            "the two arrow grooves overlap, so neither reads as an arrow"
        );
        for &(x, y) in straight.union(&branch) {
            assert!(
                !m.filled(x, y, SPLITTER_ARROW_Z),
                "the arrow groove at {x},{y} was painted, not cut"
            );
            assert!(
                m.filled(x, y, SPLITTER_ARROW_Z - 1),
                "the arrow groove at {x},{y} has no floor to sit on"
            );
        }

        // The lit slab fills exactly the straight groove, so it sits flush
        // rather than standing proud of the deck in the product's way.
        let arrow = splitter_arrow(&p);
        let lit: std::collections::BTreeSet<(i32, i32)> = arrow
            .voxels
            .keys()
            .map(|&(x, y, _)| (x as i32, y as i32))
            .collect();
        assert_eq!(
            lit, straight,
            "the lit arrow does not match the groove cut for it"
        );
    }

    /// Each split mode's body is open on exactly the exits the simulation
    /// deals to (`SplitMode::sides`) and railed shut on the rest, and cuts
    /// an arrow groove at every open exit and none at a closed one — the
    /// body is the half of the mode readout that does not move.
    #[test]
    fn each_splitter_body_opens_exactly_its_modes_exits() {
        let p = palette();
        let last = TILE_VOXELS as i32 - 1;
        let mid = (BELT_LANE.0 + BELT_LANE.1) / 2;
        let deck = BELT_SURFACE_Z;
        for (name, m, exits) in [
            ("left", machine_splitter(&p), SplitterExits::LEFT),
            ("right", machine_splitter_right(&p), SplitterExits::RIGHT),
            (
                "t_junction",
                machine_splitter_t_junction(&p),
                SplitterExits::T_JUNCTION,
            ),
            (
                "three_way",
                machine_splitter_three_way(&p),
                SplitterExits::THREE_WAY,
            ),
        ] {
            assert!(m.filled(0, mid, deck), "{name}: no intake lane");
            // (edge cell of the exit lane, a cell of the closing rail)
            for (open, lip, rail, side) in [
                (exits.straight, (last, mid), (26, mid), "straight"),
                (exits.left, (mid, last), (mid, 26), "left"),
                (exits.right, (mid, 0), (mid, 5), "right"),
            ] {
                let lane_to_edge = m.filled(lip.0, lip.1, deck);
                assert_eq!(
                    lane_to_edge, open,
                    "{name}: the {side} exit's lane reaching the edge should be {open}"
                );
                if open {
                    assert!(
                        !m.filled(lip.0, lip.1, deck + 1),
                        "{name}: something blocks the {side} exit"
                    );
                } else {
                    assert!(
                        m.filled(rail.0, rail.1, 6),
                        "{name}: the closed {side} side has no rail"
                    );
                    assert!(
                        !m.filled(rail.0, rail.1, deck + 1),
                        "{name}: the {side} rail stands above the deck"
                    );
                }
            }
            for (turns, open) in [(0, exits.straight), (1, exits.left), (3, exits.right)] {
                // Only the cells over the table: past its edge a closed
                // side has no deck at groove height to cut or not.
                let (t0, t1) = SPLITTER_TABLE;
                for (x, y) in splitter_arrow_cells(turns)
                    .into_iter()
                    .filter(|&(x, y)| (t0..=t1).contains(&x) && (t0..=t1).contains(&y))
                {
                    assert_eq!(
                        !m.filled(x, y, SPLITTER_ARROW_Z),
                        open,
                        "{name}: groove at {x},{y} (turns {turns}) cut should be {open}"
                    );
                }
            }
        }
        // The right arrow is the left one turned half way round, and the
        // three grooves are pairwise disjoint.
        let cells = |t| -> std::collections::BTreeSet<(i32, i32)> {
            splitter_arrow_cells(t).into_iter().collect()
        };
        assert!(cells(0).is_disjoint(&cells(1)));
        assert!(cells(0).is_disjoint(&cells(3)));
        assert!(cells(1).is_disjoint(&cells(3)));
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
        assert!(
            belt.filled(0, 16, BELT_SURFACE_Z),
            "BELT_SURFACE_Z is not the belt's top voxel"
        );
        assert!(
            !belt.filled(0, 16, BELT_SURFACE_Z + 1),
            "BELT_SURFACE_Z is not the belt's TOP"
        );

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

    // ----- the factory floor (the floor redesign)

    fn floor_models(p: &Palette) -> [(&'static str, VoxModel); 4] {
        [
            ("floor_tile", floor_tile(p)),
            ("floor_tile_worn", floor_tile_worn(p)),
            ("floor_tile_patch", floor_tile_patch(p)),
            ("floor_tile_drain", floor_tile_drain(p)),
        ]
    }

    /// Nothing on the floor may stand proud of the slab's top surface.
    ///
    /// `main.gd`'s `FLOOR_Y` puts every belt, machine and building on the
    /// plane `FLOOR_VOXELS` high, so a single voxel of floor detail one step
    /// taller does not read as texture — it intersects everything the player
    /// builds. That constraint is invisible in the generator (each helper
    /// writes `FLOOR_TOP` and looks correct on its own) and invisible in a
    /// render until something is standing on the offending tile, which is
    /// exactly the shape of bug a voxel-property test is for.
    #[test]
    fn no_floor_detail_stands_proud_of_the_slab() {
        let p = palette();
        for (name, m) in floor_models(&p) {
            assert_eq!(
                m.size,
                [TILE_VOXELS, TILE_VOXELS, FLOOR_VOXELS],
                "{name}: a floor tile is one tile square and FLOOR_VOXELS deep"
            );
            for &(x, y, z) in m.voxels.keys() {
                assert!(
                    z <= FLOOR_TOP as u32,
                    "{name}: voxel at ({x}, {y}, {z}) stands above the slab's top surface"
                );
            }
        }
    }

    /// The saw-cut control joint is the same on all four edges of every
    /// floor model.
    ///
    /// This is what makes `main.gd`'s `_floor_yaw_steps_for` legal. Four
    /// models laid over thousands of tiles are still only four stamps, and
    /// the magnified render said so: every worn tile's scuff ran the same
    /// way across the screen. The fix is to lay each slab at a hash-chosen
    /// quarter turn, and a quarter turn is only invisible at the joint if
    /// the joint is four-fold symmetric. An edge dropped from `floor_joint`
    /// to halve the doubled interior line would break the grid the moment a
    /// tile was rotated — silently, and only along some seams. Pin it here.
    #[test]
    fn every_rotatable_floor_tile_carries_the_same_joint_on_all_four_edges() {
        let p = palette();
        let seam = p.idx("floor_seam");
        let last = TILE_VOXELS as i32 - 1;
        for (name, m) in floor_models(&p) {
            // The drain is the one model `_floor_yaw_steps_for` never turns,
            // and its trench deliberately crosses the joint on two edges —
            // a control joint does not stop a channel cut through the floor.
            if name == "floor_tile_drain" {
                continue;
            }
            for i in 0..=last {
                for (x, y) in [(i, 0), (i, last), (0, i), (last, i)] {
                    assert_eq!(
                        m.voxels.get(&(x as u32, y as u32, FLOOR_TOP as u32)),
                        Some(&seam),
                        "{name}: the joint is broken at ({x}, {y}) — a rotated tile \
                         would not line up with its neighbours"
                    );
                }
            }
        }
    }

    /// The drain's channel runs straight through the tile, and it is
    /// symmetric about the tile midline (UI_UX.md §2b point 20).
    ///
    /// `main.gd` lays these in whole rows so consecutive tiles join into one
    /// trench; a channel that stopped short of either edge, or sat a voxel
    /// off centre, would show up as a step at every tile boundary along a
    /// run — the same failure point 20 records for pipes.
    #[test]
    fn the_drain_channel_crosses_the_whole_tile_on_the_midline() {
        let p = palette();
        let m = floor_tile_drain(&p);
        let last = TILE_VOXELS as i32 - 1;
        for x in 0..=last {
            for y in FLOOR_DRAIN_Y.0..=FLOOR_DRAIN_Y.1 {
                let mirrored = last - y;
                assert_eq!(
                    m.voxels.get(&(x as u32, y as u32, FLOOR_TOP as u32)),
                    m.voxels.get(&(x as u32, mirrored as u32, FLOOR_TOP as u32)),
                    "the drain is not symmetric about the tile midline at ({x}, {y})"
                );
            }
        }
        // The channel reaches both x faces, so a row of drains is one
        // trench rather than a row of puddles. Neither end may be slab or
        // joint: the tile hands the channel straight to its neighbour.
        let mid = (FLOOR_DRAIN_Y.0 + FLOOR_DRAIN_Y.1) / 2;
        let drain_colours = [p.idx("drain_grate"), p.idx("concrete")];
        for x in [0, last] {
            let here = m
                .voxels
                .get(&(x as u32, mid as u32, FLOOR_TOP as u32))
                .copied();
            assert!(
                here.is_some_and(|c| drain_colours.contains(&c)),
                "the drain channel does not reach x = {x}, so a run of them is a row of puddles"
            );
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

    /// The cleat's cells in a frame: everything standing at `BELT_CLEAT_Z`.
    fn cleat_cells(m: &VoxModel) -> std::collections::BTreeSet<(i32, i32)> {
        m.voxels
            .keys()
            .filter(|&&(_, _, z)| z as i32 == BELT_CLEAT_Z)
            .map(|&(x, y, _)| (x as i32, y as i32))
            .collect()
    }

    /// The cleat is a plain bar across the lane in belt rubber: no signage
    /// colour anywhere on the running surface (the arrows were removed on
    /// 2026-09-10 at the player's request), a bar that is short along the
    /// path and spans the lane less its margin, and nothing else standing
    /// above the rubber.
    #[test]
    fn every_belt_frame_carries_a_plain_cleat_bar_across_its_lane_and_no_arrow() {
        let p = palette();
        let dark = p.idx("rubber_dark");
        let yellow = p.idx("hazard_yellow");
        for (name, shape) in belt_shapes() {
            for phase in 0..BELT_FRAMES {
                let m = belt_shape(&p, shape, phase);
                let cells = cleat_cells(&m);
                assert!(
                    cells.len() >= 16,
                    "{name} f{phase}: cleat has only {} cells",
                    cells.len()
                );
                let (mut near_edge, mut inner) = (0, 0);
                for &(x, y) in &cells {
                    assert_eq!(
                        m.voxels.get(&(x as u32, y as u32, BELT_CLEAT_Z as u32)),
                        Some(&dark),
                        "{name} f{phase}: cleat cell {x},{y} is not belt rubber"
                    );
                    assert!(
                        m.filled(x, y, BELT_SURFACE_Z),
                        "{name} f{phase}: cleat cell {x},{y} floats — nothing under it at the running height"
                    );
                    let cell = surface_cell(shape, x, y).unwrap_or_else(|| {
                        panic!("{name} f{phase}: cleat cell {x},{y} is off the lane")
                    });
                    let trail = cleat_trail_u(&cell, phase);
                    assert!(
                        (0.0..CLEAT_LEN).contains(&trail),
                        "{name} f{phase}: cleat cell {x},{y} is outside the bar along the path"
                    );
                    assert!(
                        cell.v.abs() < CLEAT_HALF_WIDTH,
                        "{name} f{phase}: cleat cell {x},{y} reaches the rubber's edge"
                    );
                    if cell.v.abs() >= CLEAT_HALF_WIDTH - 1.5 {
                        near_edge += 1;
                    }
                    if cell.v.abs() < 1.0 {
                        inner += 1;
                    }
                }
                assert!(
                    near_edge >= 2,
                    "{name} f{phase}: the cleat does not span the lane"
                );
                assert!(
                    inner >= 1,
                    "{name} f{phase}: the cleat has a gap on the centre line"
                );
                // Nothing on the running surface, or above it, is signage
                // colour, and nothing but the cleat stands above the rubber.
                for (&(x, y, z), &c) in &m.voxels {
                    let (x, y, z) = (x as i32, y as i32, z as i32);
                    if surface_cell(shape, x, y).is_some() && z >= BELT_SURFACE_Z {
                        assert_ne!(
                            c, yellow,
                            "{name} f{phase}: a signage-colour mark at {x},{y},{z} — an arrow is back"
                        );
                        assert!(
                            z <= BELT_CLEAT_Z,
                            "{name} f{phase}: something stands in the lane at {x},{y},{z}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_four_frames_of_a_straight_belt_differ_only_by_the_phase_shift() {
        let p = palette();
        let frames: Vec<VoxModel> = (0..BELT_FRAMES)
            .map(|k| belt_shape(&p, BeltShape::Straight, k))
            .collect();
        let tile = TILE_VOXELS as i32;
        let (l0, l1) = BELT_LANE;
        let on_lane =
            |x: i32, y: i32, z: i32| (l0..=l1).contains(&y) && z >= BELT_SURFACE_Z - 1 && x < tile;
        for (k, frame) in frames.iter().enumerate() {
            let shift = k as i32 * BELT_FRAME_SHIFT;
            for x in 0..tile {
                for y in 0..tile {
                    for z in 0..16 {
                        let expected = if on_lane(x, y, z) {
                            frames[0].voxels.get(&(
                                ((x - shift).rem_euclid(tile)) as u32,
                                y as u32,
                                z as u32,
                            ))
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
                assert_ne!(
                    frames[a].voxels, frames[b].voxels,
                    "belt f{a} and f{b} are the same model"
                );
            }
        }
    }

    #[test]
    fn the_four_frames_of_a_corner_share_a_structure_and_advance_the_cleat_a_quarter_pattern() {
        let p = palette();
        for turn in [Turn::Left, Turn::Right] {
            let shape = BeltShape::Corner(turn);
            let frames: Vec<VoxModel> =
                (0..BELT_FRAMES).map(|k| belt_shape(&p, shape, k)).collect();
            let structure = |m: &VoxModel| -> Vec<((u32, u32, u32), u8)> {
                m.voxels
                    .iter()
                    .filter(|&(&(_, _, z), _)| (z as i32) < BELT_CLEAT_Z)
                    .map(|(&k, &c)| (k, c))
                    .collect()
            };
            let mut apexes = Vec::new();
            for (k, frame) in frames.iter().enumerate() {
                assert_eq!(
                    structure(frame),
                    structure(&frames[0]),
                    "{turn:?} corner f{k}: structure differs from f0"
                );
                // Where the cleat actually stands, read back off the voxels:
                // the circular mean of its centre-line cells' `u` (circular
                // because a frame can straddle the tile edge).
                let (mut sx, mut sy, mut n) = (0.0f32, 0.0f32, 0);
                for c in cleat_cells(frame)
                    .into_iter()
                    .filter_map(|(x, y)| surface_cell(shape, x, y))
                {
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
                    "{turn:?} corner f{k}: cleat advanced {advance} u, not {BELT_FRAME_SHIFT}"
                );
            }
            for a in 0..frames.len() {
                for b in (a + 1)..frames.len() {
                    assert_ne!(
                        cleat_cells(&frames[a]),
                        cleat_cells(&frames[b]),
                        "{turn:?} corner f{a} and f{b} share a cleat"
                    );
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
                    assert_eq!(
                        m.filled(last, i, BELT_SURFACE_Z),
                        in_lane,
                        "{name}: +X face lane wrong at y = {i}"
                    );
                    assert_eq!(
                        m.filled(i, entry_y, BELT_SURFACE_Z),
                        in_lane,
                        "{name}: entry face lane wrong at x = {i}"
                    );
                    for z in (BELT_CLEAT_Z + 1)..16 {
                        assert!(
                            !m.filled(last, i, z),
                            "{name}: +X face blocked at y = {i}, z = {z}"
                        );
                        assert!(
                            !m.filled(i, entry_y, z),
                            "{name}: entry face blocked at x = {i}, z = {z}"
                        );
                    }
                    // The other two faces carry no belt at all.
                    for z in 0..16 {
                        assert!(
                            !m.filled(0, i, z),
                            "{name}: the -X face is not closed at y = {i}, z = {z}"
                        );
                        assert!(
                            !m.filled(i, closed_y, z),
                            "{name}: the closed Y face is not closed at x = {i}, z = {z}"
                        );
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
                    assert!(
                        m.filled(x, y, BELT_SURFACE_Z),
                        "belt f{phase}: lane stops short at x = {x}, y = {y}"
                    );
                    for z in (BELT_CLEAT_Z + 1)..16 {
                        assert!(
                            !m.filled(x, y, z),
                            "belt f{phase}: face blocked at x = {x}, y = {y}, z = {z}"
                        );
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
                assert_eq!(
                    m.voxels.get(&(x, y as u32, BELT_SURFACE_Z as u32)),
                    Some(&rubber),
                    "splitter intake at {x},{y} is not plain rubber"
                );
                assert!(
                    !m.filled(x as i32, y, BELT_CLEAT_Z),
                    "splitter intake carries a belt cleat at {x},{y}"
                );
            }
        }
        // Rails identical to the belt's on the -X face.
        for y in 0..=last {
            for z in 4..=8 {
                assert_eq!(
                    m.filled(0, y, z),
                    belt.filled(0, y, z),
                    "splitter -X face differs from a belt at y = {y}, z = {z}"
                );
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

    const ALL_FACES: [PipeFace; 4] = [
        PipeFace::East,
        PipeFace::West,
        PipeFace::North,
        PipeFace::South,
    ];

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
            (
                "pipe_corner",
                pipe_corner(p),
                pipe_fluid_corner(p),
                vec![East, North],
            ),
            (
                "pipe_tee",
                pipe_tee(p),
                pipe_fluid_tee(p),
                vec![East, West, North],
            ),
            (
                "pipe_cross",
                pipe_cross(p),
                pipe_fluid_cross(p),
                vec![East, West, North, South],
            ),
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
                    if open.contains(&face) {
                        "open"
                    } else {
                        "closed"
                    }
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
        assert!(
            face_open(&once, PipeFace::South) && face_open(&once, PipeFace::East),
            "one CW turn: +X -> south, north -> east"
        );
    }

    /// Each fluid body lies inside its shape's bore: every voxel of the slab
    /// has empty bore above it at axis height (the bore's widest row), it
    /// reaches every open face and it stops short of every closed one.
    #[test]
    fn every_fluid_body_lies_inside_its_shapes_bore_and_reaches_only_open_faces() {
        let p = palette();
        for (name, shape, fluid, open) in pipe_shapes(&p) {
            assert_eq!(
                fluid.size[2], 1,
                "{name}: the fluid body must be one voxel tall"
            );
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
                    if open.contains(&face) {
                        "must"
                    } else {
                        "must not"
                    }
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
            let window = if name == "pipe" || name == "pipe_end" {
                PIPE_WINDOW
            } else {
                PIPE_LEG_WINDOW
            };
            let top = shape.size[2] as i32 - 1;
            for x in window.0..=window.1 {
                for y in PIPE_BORE_Y.0..=PIPE_BORE_Y.1 {
                    for z in PIPE_BORE_Z0 + 1..=top {
                        assert!(
                            !shape.filled(x, y, z),
                            "{name}: window voxel ({x}, {y}, {z}) is not empty"
                        );
                    }
                }
            }
            // ...and it is a window, not a missing wall: the tube is still
            // there just outside it, either side.
            assert!(
                shape.filled(window.0 - 1, PIPE_BORE_Y.0, PIPE_AXIS_Z + 3)
                    || shape.filled(window.1 + 1, PIPE_BORE_Y.0, PIPE_AXIS_Z + 3),
                "{name}: no tube wall frames the window"
            );
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
            (
                "pipe_fitting_machine",
                pipe_fitting_machine(&p),
                PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN,
            ),
            (
                "pipe_fitting_tank",
                pipe_fitting_tank(&p),
                PIPE_OUTER_R + 1.0 + PIPE_FITTING_MARGIN,
            ),
        ] {
            // The flange is centred on the tile midline, half a voxel from
            // voxel 15/16, so its reach along an axis is sqrt(r² - ¼).
            let reach = (r * r - 0.25).sqrt().floor() as i32;
            for (y, z) in [
                (15, PIPE_AXIS_Z + reach),
                (16, PIPE_AXIS_Z - reach),
                (15 - reach, PIPE_AXIS_Z),
                (16 + reach, PIPE_AXIS_Z),
            ] {
                assert!(
                    fitting.filled(last, y, z),
                    "{name}: flange does not reach ({y}, {z}) on the boundary"
                );
            }
            // Checked on the UNDERSIDE: the tank fitting's valve stands on
            // top of its flange.
            assert!(
                !fitting.filled(last, 15, PIPE_AXIS_Z - reach - 1),
                "{name}: flange is larger than its documented radius"
            );
            for a in [15, 16] {
                assert!(
                    !fitting.filled(last, a, PIPE_AXIS_Z),
                    "{name}: the bore is plugged at the boundary"
                );
            }
            for &(x, _, z) in fitting.voxels.keys() {
                let floor = if (z as i32) <= PIPE_AXIS_Z + PIPE_OUTER_R as i32 + 2 {
                    PIPE_FITTING_X0
                } else {
                    PIPE_FITTING_TOP_X0
                };
                assert!(
                    x as i32 >= floor,
                    "{name}: hardware at x = {x}, z = {z} would cover a leg window"
                );
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
                    for (nx, ny, nz) in [
                        (x + 1, y, z),
                        (x - 1, y, z),
                        (x, y + 1, z),
                        (x, y - 1, z),
                        (x, y, z + 1),
                        (x, y, z - 1),
                    ] {
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

    /// The person stands over a machine at a fixed yaw, like the "!".
    #[test]
    fn an_understaffed_marker_is_unchanged_by_a_quarter_turn() {
        let person = understaffed_marker(&palette());
        assert_eq!(
            person.voxels,
            person.rotated_cw(1).voxels,
            "the understaffed marker must read the same from every side"
        );
    }

    /// A head over a body with daylight between them, all of it the lit
    /// "stopped" red.
    #[test]
    fn an_understaffed_marker_is_a_lit_head_over_a_body() {
        let p = palette();
        let person = understaffed_marker(&p);
        let red = p.idx("status_red");
        let lit_at = |z: i32| person.filled(15, 15, z);
        assert!(lit_at(0) && lit_at(12), "the body stands on the floor");
        assert!(
            (13..=16).all(|z| !lit_at(z)),
            "a neck gap parts head from body"
        );
        assert!(lit_at(17) && lit_at(28), "the head rises above it");
        assert!(
            person.voxels.values().all(|&c| c == red),
            "every voxel is status_red"
        );
    }

    /// The warning marker stands at a fixed yaw over whatever it marks, so
    /// it has to be the same shape from every side it can be seen from.
    #[test]
    fn a_warning_marker_is_unchanged_by_a_quarter_turn() {
        let bang = warning_marker(&palette());
        assert_eq!(
            bang.voxels,
            bang.rotated_cw(1).voxels,
            "the warning marker must read the same from every side"
        );
    }

    /// A "!" is a bar over a dot with daylight between them, and the lit part
    /// is the emissive warning hue — the shape and the glow are the marker.
    #[test]
    fn a_warning_marker_is_a_lit_bar_over_a_dot() {
        let p = palette();
        let bang = warning_marker(&p);
        let amber = p.idx("status_amber");
        let lit_at = |z: u32| {
            bang.voxels
                .iter()
                .any(|(&(_, _, vz), &c)| c == amber && vz == z)
        };
        assert!(lit_at(0), "the dot must stand at the foot");
        assert!(
            !(6..=11).any(lit_at),
            "the dot and the bar must have a gap between them"
        );
        assert!(lit_at(31), "the bar must reach the head");
    }

    /// The fault clamp (issue #54) is overlaid on a tile whose own facing is
    /// whatever its run's is, and `main.gd` stands it at a fixed yaw rather
    /// than at its host's. That is only safe while the model is unchanged by
    /// a quarter turn — otherwise the striping would run one way on an
    /// east/west run and the other on a north/south one, which is §2b point
    /// 29's defect with the check skipped rather than passed.
    #[test]
    fn a_pipe_fault_clamp_is_unchanged_by_a_quarter_turn() {
        let clamp = pipe_fault(&palette());
        assert_eq!(
            clamp.voxels,
            clamp.rotated_cw(1).voxels,
            "the fault clamp must look the same on a run at any facing"
        );
    }

    /// And it must actually COVER the tube it marks, on both horizontal
    /// axes: the marker's whole job is that a faulted run cannot be mistaken
    /// for a merely empty one, and a clamp narrower than the pipe would read
    /// as a band tucked inside it. Checked against a straight and against
    /// that straight turned, because a run's tiles face both ways.
    #[test]
    fn a_pipe_fault_clamp_covers_the_tube_it_marks() {
        let p = palette();
        let clamp = pipe_fault(&p);
        for turns in [0, 1] {
            let shape = pipe(&p).rotated_cw(turns);
            let mut covered = 0;
            for &(x, y, z) in shape.voxels.keys() {
                let (x, y, z) = (x as i32, y as i32, z as i32);
                // Only the mid-tile slice of TUBE the clamp straddles is its
                // business: the rest of the run is deliberately left in view
                // so a marked run still reads as a run, and the support
                // stand below z = 11 is floor hardware, not pipe.
                let along = if turns == 0 { x } else { y };
                if z < PIPE_AXIS_Z - PIPE_OUTER_R as i32 || !clamp.filled(along, PIPE_C as i32, z) {
                    continue;
                }
                assert!(
                    clamp.filled(x, y, z),
                    "turned {turns}: the tube at ({x}, {y}, {z}) is inside the clamp's span but \
                     not inside the clamp — it would poke out of its own marker"
                );
                covered += 1;
            }
            assert!(
                covered > 0,
                "turned {turns}: the clamp covers no tube at all"
            );
        }
    }

    /// Hazard striping, both colours, and NO emissive: this is signage on a
    /// dead run, not a lamp. The andon stack lights came down on 2026-09-12
    /// (DECISIONS.md) and an emissive marker would be the same poles in
    /// another shape.
    #[test]
    fn a_pipe_fault_clamp_is_matte_hazard_signage() {
        let p = palette();
        let clamp = pipe_fault(&p);
        for class in [
            MaterialClass::Metal,
            MaterialClass::Emissive,
            MaterialClass::Liquid,
        ] {
            assert!(
                clamp.filter_class(&p, class).is_empty(),
                "the fault clamp must carry no {class:?} voxels"
            );
        }
        for name in ["hazard_yellow", "hazard_black"] {
            assert!(
                clamp.voxels.values().any(|&c| c == p.idx(name)),
                "the fault clamp must be striped in {name}"
            );
        }
    }

    /// The coin is round about the volume MIDLINE, not about a voxel
    /// (UI_UX.md §2b point 20). `main.gd` spins these about world Y as they
    /// fly, and a disc centred on voxel 4 would orbit its own pivot rather
    /// than turn on the spot — the same defect that made pipe runs step at
    /// every joint. A 180° turn is the cheapest complete statement of that
    /// symmetry: it maps x to `COIN_VOXELS - 1 - x`, so an off-midline disc
    /// cannot survive it.
    #[test]
    fn coin_is_symmetric_about_its_midline() {
        let coin = coin(&palette());
        assert_eq!(
            coin.voxels,
            coin.rotated_cw(2).voxels,
            "the coin must be unchanged by a half turn about its own centre"
        );
    }

    /// A coin has to be BOTH shiny and lit: the brass rim is UI_UX.md §4's
    /// metal exception (it catches the light while tumbling) and the amber
    /// face is §2b point 13's emissive rule (both faces this camera sees are
    /// the shadow side, and a payout is a signal, not scenery). Losing
    /// either class would still render a coin-shaped object and still pass
    /// every structural test, so the split is pinned here.
    #[test]
    fn coin_carries_both_a_metal_rim_and_an_emissive_face() {
        let p = palette();
        let coin = coin(&p);
        for class in [MaterialClass::Metal, MaterialClass::Emissive] {
            assert!(
                !coin.filter_class(&p, class).is_empty(),
                "the coin must have voxels in the {:?} class",
                class
            );
        }
    }

    // -----------------------------------------------------------------------
    // Machine motion
    // -----------------------------------------------------------------------
    //
    // The four rules the "Machine motion" section's header states, one test
    // each. They are worth having as tests rather than as a render check for
    // the same reason the junction tests above are: an overlay that shares a
    // voxel with the body it sits on z-fights, and a z-fight at scene scale
    // looks like a texture seam, not like a bug someone would go looking for.

    /// Every machine that has a body and a motion overlay, paired.
    fn motion_families(p: &Palette) -> Vec<(&'static str, VoxModel, Vec<VoxModel>)> {
        let frames = |f: fn(&Palette, u32) -> VoxModel| {
            (0..MACHINE_FRAMES).map(|k| f(p, k)).collect::<Vec<_>>()
        };
        vec![
            ("machine", machine(p), frames(machine_motion)),
            (
                "machine_crusher",
                machine_crusher(p),
                frames(machine_crusher_motion),
            ),
            (
                "machine_mixer",
                machine_mixer(p),
                frames(machine_mixer_motion),
            ),
            (
                "machine_cooking_vat",
                machine_cooking_vat(p),
                frames(machine_cooking_vat_motion),
            ),
            (
                "machine_bottler",
                machine_bottler(p),
                frames(machine_bottler_motion),
            ),
            (
                "machine_packer",
                machine_packer(p),
                frames(machine_packer_motion),
            ),
        ]
    }

    /// Rule 2: a motion frame never shares a cell with the body it is drawn
    /// over. This is the one that actually costs geometry — it is why the
    /// crusher's rolls lost their outer millimetre, the mixer its ribbon and
    /// the packer its taping head — so it is also the one most likely to be
    /// quietly broken by a later tweak to either half.
    #[test]
    fn a_motion_frame_shares_no_voxel_with_its_machine_body() {
        let p = palette();
        for (name, body, frames) in motion_families(&p) {
            for (k, frame) in frames.iter().enumerate() {
                for &cell in frame.voxels.keys() {
                    assert!(
                        !body.voxels.contains_key(&cell),
                        "{name}_motion_f{k} and {name} both fill {cell:?} — they will z-fight"
                    );
                }
            }
        }
    }

    /// Rule 4: the overlay is centred on the same footprint as the body, so
    /// it lands on the machine's tiles rather than beside them. Height is
    /// deliberately NOT pinned: `vox::encode` lifts a model by half its own
    /// height, so a taller volume still has its floor on the ground, which
    /// is what lets the vat's drive coupling stand above a body that has no
    /// room left for it.
    #[test]
    fn a_motion_frame_has_its_machine_s_footprint() {
        let p = palette();
        for (name, body, frames) in motion_families(&p) {
            for (k, frame) in frames.iter().enumerate() {
                assert_eq!(
                    (frame.size[0], frame.size[1]),
                    (body.size[0], body.size[1]),
                    "{name}_motion_f{k} is not on {name}'s footprint"
                );
            }
        }
    }

    /// Rule 4's other half, and the reason the generic machine's spider
    /// lives in a 34-voxel volume: `encode` lifts a model by `size[2] / 2`
    /// in whole voxels, so an odd volume sinks half a voxel. Cheap to get
    /// wrong, invisible in a preview render (which centres whatever it is
    /// given), and in the scene it is a seam along the join.
    #[test]
    fn a_motion_volume_has_an_even_height() {
        let p = palette();
        for (name, _, frames) in motion_families(&p) {
            for (k, frame) in frames.iter().enumerate() {
                assert_eq!(
                    frame.size[2] % 2,
                    0,
                    "{name}_motion_f{k} is {} voxels tall, an odd number, so it will sit half a \
                     voxel low",
                    frame.size[2]
                );
            }
        }
    }

    /// Rule 1, half of it: every frame is a real arrangement of parts, so a
    /// machine at rest (frame 0, always drawn) is never a machine with a
    /// hole where its mechanism should be.
    #[test]
    fn every_motion_frame_holds_something() {
        let p = palette();
        for (name, _, frames) in motion_families(&p) {
            for (k, frame) in frames.iter().enumerate() {
                assert!(!frame.is_empty(), "{name}_motion_f{k} is empty");
            }
        }
    }

    /// ...and the other half: the machine never stands still for a frame
    /// while it is meant to be working. Stated as CONSECUTIVE frames rather
    /// than all four, and the difference is rule 1's: a plunge passes
    /// through its half-way pose on the way down and again on the way up, so
    /// the bottler's f1 and f3 are legitimately the same model. Two frames
    /// that coincide side by side are not — that is a stutter, the same
    /// failure the belt family's pattern-period rule avoids (UI_UX.md §2b
    /// point 23). Checked round the loop, so the last frame is compared with
    /// frame 0 too.
    #[test]
    fn consecutive_motion_frames_of_one_machine_differ() {
        let p = palette();
        for (name, _, frames) in motion_families(&p) {
            for a in 0..frames.len() {
                let b = (a + 1) % frames.len();
                assert_ne!(
                    frames[a].voxels, frames[b].voxels,
                    "{name}_motion_f{a} and {name}_motion_f{b} are the same model, so the \
                     mechanism stops for a frame"
                );
            }
        }
    }

    /// A turning family has the stronger property: four frames, four
    /// different angles. A repeat here would mean the fold count and
    /// [`MACHINE_FRAMES`] divide each other, so the mechanism completes its
    /// symmetry early and the last frames of the cycle are wasted.
    #[test]
    fn a_turning_machine_shows_four_different_angles() {
        let p = palette();
        for (name, build) in TURNING_MOTION {
            let frames: Vec<VoxModel> = (0..MACHINE_FRAMES).map(|k| build(&p, k)).collect();
            for a in 0..frames.len() {
                for b in (a + 1)..frames.len() {
                    assert_ne!(
                        frames[a].voxels, frames[b].voxels,
                        "{name}_motion_f{a} and {name}_motion_f{b} are at the same angle"
                    );
                }
            }
        }
    }

    /// The families whose mechanism turns, as opposed to travelling: every
    /// rule stated in terms of an angle applies to exactly these.
    /// One entry of [`TURNING_MOTION`]: the family's model-name stem and the
    /// generator that draws one of its motion frames.
    type MotionFamily = (&'static str, fn(&Palette, u32) -> VoxModel);

    const TURNING_MOTION: [MotionFamily; 4] = [
        ("machine", machine_motion),
        ("machine_crusher", machine_crusher_motion),
        ("machine_mixer", machine_mixer_motion),
        ("machine_cooking_vat", machine_cooking_vat_motion),
    ];

    /// Rule 3, for the families driven by [`spin`]: one more frame lands
    /// exactly back on frame 0, so the loop closes instead of jumping.
    /// Asserted by asking the generator for phase `MACHINE_FRAMES` — the
    /// frame after the last one — rather than by trusting the arithmetic.
    #[test]
    fn a_turning_motion_cycle_closes_on_frame_zero() {
        let p = palette();
        for (name, build) in TURNING_MOTION {
            assert_eq!(
                build(&p, MACHINE_FRAMES).voxels,
                build(&p, 0).voxels,
                "{name}'s motion does not close: frame {MACHINE_FRAMES} differs from frame 0"
            );
        }
    }

    /// Rule 3, for the two families driven by an offset table instead of an
    /// angle: there and back, starting at rest. A table that ran 0, 1, 2, 3
    /// would snap back to 0 on the frame after the last, which reads as the
    /// carriage being yanked home.
    #[test]
    fn a_travelling_motion_cycle_is_there_and_back() {
        for (name, table) in [
            ("bottler plunge", BOTTLER_PLUNGE),
            ("packer travel", PACKER_TRAVEL),
        ] {
            assert_eq!(table[0], 0, "{name} does not start at rest");
            let n = MACHINE_FRAMES as usize;
            for k in 1..n {
                assert_eq!(
                    table[k],
                    table[n - k],
                    "{name} is not symmetric about its midpoint, so its cycle does not close"
                );
            }
        }
    }

    /// A turning mechanism is the SAME mechanism at a different angle, so
    /// its voxel count may wobble with the grid but must not drift: a frame
    /// that gained or lost a fifth of its mass is a part appearing, not a
    /// part turning (rule 1).
    ///
    /// Two exclusions, both of them real motion rather than an appearance.
    /// The bottler's and packer's parts travel, and a nozzle reaching down
    /// is longer than a nozzle at rest. The crusher's teeth are culled at
    /// [`CRUSHER_POCKET_FLOOR_Z`], so a tooth turning down into the nip
    /// leaves the model — which is exactly what a tooth turning down into
    /// the nip does to the player's view of it, since below that line it is
    /// inside the frame.
    #[test]
    fn a_turning_motion_frame_keeps_its_mass() {
        let p = palette();
        for (name, build) in TURNING_MOTION {
            if name == "machine_crusher" {
                continue;
            }
            let counts: Vec<usize> = (0..MACHINE_FRAMES)
                .map(|k| build(&p, k).voxels.len())
                .collect();
            let lo = *counts.iter().min().unwrap() as f32;
            let hi = *counts.iter().max().unwrap() as f32;
            assert!(
                hi <= lo * 1.2,
                "{name}'s motion frames vary in size too much to be one mechanism turning: {counts:?}"
            );
        }
    }

    /// The crusher's teeth stay inside the feed pocket the body carves.
    /// Stated separately from the disjointness test above because it is the
    /// REASON that test passes for this kind, and because the pocket is a
    /// hole in a solid frame: a tooth a voxel too far east is not merely
    /// hidden, it is inside the frame's east wall.
    #[test]
    fn the_crushers_teeth_stay_in_its_feed_pocket() {
        let p = palette();
        for k in 0..MACHINE_FRAMES {
            for &(x, y, z) in machine_crusher_motion(&p, k).voxels.keys() {
                let (x, y, z) = (x as i32, y as i32, z as i32);
                // The flywheel spokes are the model's other half; they live
                // on the camera-facing face, outside the pocket entirely.
                if y <= 2 {
                    continue;
                }
                assert!(
                    (8..=23).contains(&x) && (9..=22).contains(&y) && z >= CRUSHER_POCKET_FLOOR_Z,
                    "crusher tooth at ({x}, {y}, {z}) on frame {k} is outside the carved pocket"
                );
            }
        }
    }

    /// Both crushing rolls are the same grey, core and teeth, on the body
    /// and on every motion frame — the owner's 2026-09-26 call, replacing a
    /// brass second roll. Only the part standing in the feed pocket is
    /// checked; below its floor the rolls are inside the frame.
    #[test]
    fn the_crushers_two_rolls_are_both_grey() {
        let p = palette();
        let grey = p.idx("steel_light");
        let mut models = vec![machine_crusher(&p)];
        models.extend((0..MACHINE_FRAMES).map(|k| machine_crusher_motion(&p, k)));
        for roll_x in [12i32, 20] {
            let mut seen = 0;
            for m in &models {
                for (&(x, y, z), &c) in &m.voxels {
                    let (x, y, z) = (x as i32, y as i32, z as i32);
                    let (dx, dz) = ((x - roll_x) as f32, (z - 16) as f32);
                    let on_roll = (dx * dx + dz * dz).sqrt() <= CRUSHER_ROLL_R
                        && (CRUSHER_ROLL_Y.0..=CRUSHER_ROLL_Y.1).contains(&y)
                        && z >= CRUSHER_POCKET_FLOOR_Z;
                    if on_roll {
                        seen += 1;
                        assert_eq!(
                            c, grey,
                            "roll x={roll_x} voxel ({x}, {y}, {z}) is not steel_light"
                        );
                    }
                }
            }
            assert!(seen > 0, "roll x={roll_x} has no voxels in the pocket");
        }
    }

    // -- Staff category icons (the staff-pools epic, ticket 2) -----------

    fn staff_assets() -> Vec<&'static Asset> {
        ASSETS
            .iter()
            .filter(|a| a.name.starts_with("staff_"))
            .collect()
    }

    /// The four category props exist, are not empty, and are authored in
    /// the one-tile volume like every other icon-bearing prop. `VoxModel::set`
    /// clips silently at the volume (UI_UX.md §2b point 29), so a generator
    /// that reached past 32 would lose voxels without a word — the voxel
    /// bounds check is what turns that into a failure.
    #[test]
    fn staff_models_are_non_empty_and_fit_one_tile() {
        let p = palette();
        let staff = staff_assets();
        let names: Vec<&str> = staff.iter().map(|a| a.name).collect();
        assert_eq!(
            names,
            [
                "staff_manufacturing",
                "staff_engineering",
                "staff_packaging",
                "staff_logistics"
            ],
            "one staff icon per data/staff.ron category, in category order"
        );
        for asset in staff {
            let m = (asset.build)(&p);
            assert!(!m.is_empty(), "{} is empty", asset.name);
            assert_eq!(
                m.size, [TILE_VOXELS; 3],
                "{} is not a 32^3 volume",
                asset.name
            );
            for &(x, y, z) in m.voxels.keys() {
                assert!(
                    x < TILE_VOXELS && y < TILE_VOXELS && z < TILE_VOXELS,
                    "{} has a voxel outside its volume at ({x}, {y}, {z})",
                    asset.name
                );
            }
        }
    }

    /// The plan's colour rule (docs/STAFF_POOLS_PLAN.md, "The icons"): the
    /// hard hat and the tape are `hazard_yellow`, the spanner and the jack
    /// are `metal`, the carton is `matte`. A repaint that kept every
    /// silhouette would still pass the test above, so pin the classes.
    #[test]
    fn staff_icons_use_the_planned_materials() {
        let p = palette();
        let yellow = p.idx("hazard_yellow");
        let has_yellow = |m: &VoxModel| m.voxels.values().any(|&c| c == yellow);

        let hat = staff_manufacturing(&p);
        assert!(has_yellow(&hat), "the hard hat is hazard_yellow");
        assert!(
            hat.filter_class(&p, MaterialClass::Metal).is_empty(),
            "a hard hat has no steel on it"
        );

        let spanner = staff_engineering(&p);
        assert!(
            spanner.filter_class(&p, MaterialClass::Matte).is_empty(),
            "the spanner is all metal"
        );
        assert!(!spanner.filter_class(&p, MaterialClass::Metal).is_empty());

        let carton = staff_packaging(&p);
        assert!(has_yellow(&carton), "the tape is hazard_yellow");
        assert!(
            carton.filter_class(&p, MaterialClass::Metal).is_empty(),
            "a carton has no steel on it"
        );

        let jack = staff_logistics(&p);
        assert!(
            !jack.filter_class(&p, MaterialClass::Metal).is_empty(),
            "the jack's forks and tiller are metal"
        );
        assert!(!has_yellow(&jack), "the jack carries no signage colour");
    }

    /// The spanner's jaws are genuinely open — empty voxels, not a darker
    /// colour — for the same reason the machines' doorways are (UI_UX.md
    /// §2b point 11): a filled slot meshes as a solid head and the tool
    /// reads as a dumbbell.
    #[test]
    fn spanner_jaws_are_open() {
        let m = staff_engineering(&palette());
        assert!(!m.filled(2, 14, 0), "the -X jaw is open at its mouth");
        assert!(
            m.filled(2, 11, 0) && m.filled(2, 18, 0),
            "the -X jaw has two lips"
        );
        assert!(!m.filled(29, 17, 0), "the +X jaw is open at its mouth");
        assert!(
            m.filled(29, 14, 0) && m.filled(29, 21, 0),
            "the +X jaw has two lips"
        );
    }

    /// Every sauce's finished goods are the SAME OBJECTS in a different
    /// livery (`docs/BBQ_SAUCE_PLAN.md`, and the mustard epic after it): one
    /// bottler makes all three bottles and one packer makes all three cases,
    /// so a bottle that had gained a different silhouette would be claiming
    /// a machine the factory has not got. Occupied voxels identical, colours
    /// not — which is also what keeps `bottle`/`box_of_sauce` honest as
    /// shared helpers, since a second hand-written generator is free to
    /// drift and this would say so.
    ///
    /// Written over a list of sauces rather than a pair, because the third
    /// one arriving is when a helper stops being shared: `bottle` grew a
    /// `cap` parameter for mustard, and this is what says the parameter
    /// changed colours and nothing else.
    #[test]
    fn every_sauce_s_item_is_its_ketchup_counterpart_in_another_livery() {
        let p = palette();
        for (ketchup, other, sauce, what) in [
            (item_bottle(&p), item_bottle_bbq(&p), "BBQ", "bottle"),
            (item_box(&p), item_box_bbq(&p), "BBQ", "case"),
            (
                item_bottle(&p),
                item_bottle_mustard(&p),
                "mustard",
                "bottle",
            ),
            (item_box(&p), item_box_mustard(&p), "mustard", "case"),
        ] {
            let shape: Vec<_> = ketchup.voxels.keys().collect();
            let other_shape: Vec<_> = other.voxels.keys().collect();
            assert_eq!(
                shape, other_shape,
                "the {sauce} {what} has the ketchup {what}'s silhouette"
            );
            assert_ne!(
                ketchup.voxels, other.voxels,
                "the {sauce} {what} is not the ketchup {what} repainted in the same colours"
            );
        }
    }

    /// **A case's carton does not change between sauces — its LABEL does**
    /// (the project owner's call, `docs/BBQ_SAUCE_PLAN.md`). A box is the
    /// smallest thing on a belt, and a second sauce shipping in a differently
    /// coloured carton would read as different packaging rather than as
    /// different contents. It held for the third sauce too, which is why the
    /// mustard case's whole distinction is a saturated ink and not a
    /// yellow box.
    ///
    /// Stated as "exactly the print voxels differ" rather than by counting
    /// them: the print is panels of a known size, and the interesting
    /// failure is a future edit tinting the CARTON, which a count would miss.
    ///
    /// Three print regions since issue #40: the label on the camera's face,
    /// the label on the icon's, and the LID panels that carry the
    /// distinction at whole-factory zoom. The lid is listed here rather than
    /// folded into a looser bound because "the top of the box may differ"
    /// and "the box may differ" are one voxel apart — the tape seam at
    /// `x = 7..8` and the carton's edges have to stay out of it.
    #[test]
    fn only_a_case_s_print_tells_the_sauces_apart() {
        let p = palette();
        let ketchup = item_box(&p);
        for (sauce, case) in [("BBQ", item_box_bbq(&p)), ("mustard", item_box_mustard(&p))] {
            let differing: Vec<_> = ketchup
                .voxels
                .iter()
                .filter(|(at, colour)| case.voxels.get(at) != Some(colour))
                .map(|(at, _)| *at)
                .collect();
            assert!(
                !differing.is_empty(),
                "the {sauce} case is not the ketchup case"
            );
            for (x, y, z) in differing {
                // Every print region leaves the carton's side edges and the
                // tape seam between the panels in plain kraft.
                assert!(
                    (3..=6).contains(&x) || (9..=12).contains(&x),
                    "only a print panel differs on the {sauce} case, at ({x}, {y}, {z})"
                );
                let on_a_labelled_face = (y == 2 || y == 13) && (4..=8).contains(&z);
                let on_the_lid = z == 10 && (4..=11).contains(&y);
                assert!(
                    on_a_labelled_face || on_the_lid,
                    "only a labelled face or the lid differs on the {sauce} case, \
                     at ({x}, {y}, {z})"
                );
            }
        }
    }

    /// **The lid print paints voxels that were already carton** (issue #40):
    /// a case's SILHOUETTE is unchanged by it, which is what lets a fix for
    /// "the two cases look alike" ship without touching the packer, the
    /// belt or `every_sauce_s_item_is_its_ketchup_counterpart_in_another_livery`.
    /// Asserted against the lid's own geometry rather than a stored fixture,
    /// since the claim is about the top FACE being solid, not about a count.
    #[test]
    fn the_lid_print_replaces_carton_rather_than_adding_to_it() {
        let p = palette();
        let case = item_box(&p);
        for x in 2..=13 {
            for y in 2..=13 {
                assert!(
                    case.filled(x, y, 10),
                    "the lid is solid at ({x}, {y}) with or without print"
                );
            }
        }
        // And the seam still crosses between the two panels.
        let dark = p.idx("cardboard_dark");
        for y in 2..=13 {
            assert_eq!(
                case.voxels.get(&(7, y, 10)),
                Some(&dark),
                "the tape seam survives the lid print at y = {y}"
            );
        }
    }

    /// Bounding span of every occupied voxel, as (width in X, depth in Y,
    /// height in Z).
    fn extent(m: &VoxModel) -> (u32, u32, u32) {
        let span = |f: fn(&(u32, u32, u32)) -> u32| {
            let lo = m.voxels.keys().map(f).min().unwrap();
            let hi = m.voxels.keys().map(f).max().unwrap();
            hi - lo + 1
        };
        (span(|at| at.0), span(|at| at.1), span(|at| at.2))
    }

    /// [`extent`] with the floor course left out — the vessel, not what
    /// spilled off it.
    ///
    /// Several raws are drawn with a few grains on the ground beside them,
    /// and `item_mustard_seed`'s go at the DIAGONAL corners of the volume
    /// (its note says why), so they reach x 0 and x 15: a plain [`extent`] of
    /// the seed tote is the whole 16-voxel volume, and "the widest thing in
    /// the row" measured that way is measuring where the spill was placed
    /// rather than how wide the tote is. Narrowing the whole tote — walls,
    /// rim and fill — by four voxels left the first cut of the test below
    /// green, because the spill still spanned x 0..15; measured above the
    /// floor it goes red. (The review of PR #179 found this by narrowing the
    /// walls alone, which leaves a 12-wide rim overhanging them, so the
    /// outline really is still the widest in the row.)
    fn body_extent(m: &VoxModel) -> (u32, u32, u32) {
        let span = |f: fn(&(u32, u32, u32)) -> u32| {
            let above = || m.voxels.keys().filter(|at| at.2 > 0);
            let lo = above().map(f).min().expect("a body above the floor");
            let hi = above().map(f).max().unwrap();
            hi - lo + 1
        };
        (span(|at| at.0), span(|at| at.1), span(|at| at.2))
    }

    /// **The mustard branch adds two outlines the row did not have.** Eight
    /// raws now stand on the market plate at 64px, where a thing is
    /// identified by the proportion of its parts (§2b point 31), and the two
    /// new ones are deliberately the row's broad-and-low pair: the seed tote
    /// is the widest thing on the plate and the turmeric hand the shortest
    /// and the sparsest. Everything the BBQ three were measured against
    /// still holds — this test is the same claim extended to eight, and it
    /// exists because adding a raw is exactly when a row stops working.
    ///
    /// Stated as orderings and distinct outlines rather than as fixed
    /// numbers: the claim is that no two raws share a silhouette, not that
    /// any one of them keeps a particular size for ever.
    #[test]
    fn the_mustard_raws_add_two_outlines_the_row_did_not_have() {
        let p = palette();
        // Every measurement here is of the vessel above the floor course,
        // never of the grains spilled beside it — see [`body_extent`].
        let row = [
            ("tomato", item_tomato(&p)),
            ("sugar", item_sugar(&p)),
            ("vinegar", item_vinegar(&p)),
            ("spice", item_spice_mix(&p)),
            ("molasses", item_molasses(&p)),
            ("smoke", item_liquid_smoke(&p)),
            ("seed", item_mustard_seed(&p)),
            ("turmeric", item_turmeric(&p)),
        ]
        .map(|(name, m)| {
            (
                name,
                body_extent(&m),
                m.voxels.keys().filter(|at| at.2 > 0).count(),
            )
        });
        for (i, a) in row.iter().enumerate() {
            for b in &row[i + 1..] {
                assert_ne!(
                    (a.1.0, a.1.2),
                    (b.1.0, b.1.2),
                    "{} and {} would draw the same outline ({:?} vs {:?})",
                    a.0,
                    b.0,
                    a.1,
                    b.1
                );
            }
        }
        // Strict orderings, not `max_by_key`/`min_by_key`: those return the
        // LAST of a tie, so a raw that merely equalled the record would pass
        // as holding it.
        let one = |name: &str| *row.iter().find(|r| r.0 == name).expect("in the row");
        let rest = |name: &'static str| row.iter().filter(move |r| r.0 != name);
        let (_, seed, seed_n) = one("seed");
        assert!(
            rest("seed").all(|r| r.1.0 < seed.0),
            "the open seed tote is strictly the widest in the row ({seed:?}), and these are \
             not narrower: {:?}",
            rest("seed").filter(|r| r.1.0 >= seed.0).collect::<Vec<_>>()
        );
        let (_, turmeric, turmeric_n) = one("turmeric");
        assert!(
            rest("turmeric").all(|r| r.1.2 > turmeric.2),
            "the turmeric hand lies strictly lowest in the row ({turmeric:?}), and these are \
             not taller: {:?}",
            rest("turmeric")
                .filter(|r| r.1.2 <= turmeric.2)
                .collect::<Vec<_>>()
        );
        assert!(
            rest("turmeric").all(|r| r.2 > turmeric_n),
            "and it is strictly the least of the eight by volume ({turmeric_n} voxels against \
             the tote's {seed_n}), unlike {:?}",
            rest("turmeric")
                .filter(|r| r.2 <= turmeric_n)
                .collect::<Vec<_>>()
        );
        // The two new ones are the row's only two broader than they are
        // tall — an open tote and a root lying down, against six standing
        // containers. This is what the plate reads first.
        for (name, (w, _, h), _) in row {
            let broad = w > h;
            assert_eq!(
                broad,
                name == "seed" || name == "turmeric",
                "{name} is {w} wide by {h} tall, and only the mustard raws lie down"
            );
        }
    }

    /// **The seed tote is genuinely OPEN and its heap genuinely breaks the
    /// rim** — the two things that make it read as bulk seed rather than as
    /// a painted steel block, and the two a later edit is most likely to
    /// undo (the old spice tin's note records the same pair of mistakes).
    /// The rim is a ring: fill the centre of it and the tote is lidded, and
    /// at 64px a lidded tote is a crate.
    #[test]
    fn the_seed_tote_is_open_and_its_heap_stands_proud_of_the_rim() {
        let p = palette();
        let tote = item_mustard_seed(&p);
        let seed = p.idx("seed_tan");
        let rim = p.idx("steel_light");
        let top_of = |c: u8| {
            tote.voxels
                .iter()
                .filter(|e| *e.1 == c)
                .map(|e| e.0.2)
                .max()
                .expect("the tote has both a rim and a fill")
        };
        assert!(
            top_of(seed) > top_of(rim),
            "the heap ({}) stands above the rim ({})",
            top_of(seed),
            top_of(rim)
        );
        // The rim is a RING: its top course of `steel_light` is exactly the
        // one-voxel perimeter of its own mouth. This is the leg that proves
        // the tote is open — sampling the tote's axis cannot, because the
        // heap ellipsoid is drawn after the rim and repaints that voxel
        // whether the mouth was carved or not (the review of PR #179 lidded
        // the tote with the axis assertion still green).
        let rim_top = top_of(rim);
        let course: Vec<_> = tote
            .voxels
            .iter()
            .filter(|e| e.0.2 == rim_top && *e.1 == rim)
            .collect();
        let side = |f: fn(&(u32, u32, u32)) -> u32| {
            let hi = course.iter().map(|e| f(e.0)).max().expect("a rim course");
            let lo = course.iter().map(|e| f(e.0)).min().unwrap();
            hi - lo + 1
        };
        let (w, d) = (side(|at| at.0), side(|at| at.1));
        assert_eq!(
            course.len(),
            (2 * (w + d) - 4) as usize,
            "the rim is a one-voxel RING round a {w}x{d} mouth, not a lid ({} voxels of \
             rim metal at z {rim_top})",
            course.len()
        );
        assert_eq!(
            tote.voxels.get(&(8, 8, rim_top)),
            Some(&seed),
            "and the heap comes up through that mouth on the tote's own axis"
        );
        // And the vessel's brightest pixel is its outline (UI_UX.md §4), not
        // the substance in it.
        let luma = |c: u8| {
            let [r, g, b] = p.rgb_of(c).expect("a palette colour");
            0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64
        };
        assert!(
            luma(rim) > luma(seed),
            "the steel rim is brighter than the seed inside it"
        );
    }

    /// **Turmeric is dark with a bright CUT, and the cut has to reach both
    /// cameras.** Its colour is the reason it is in the recipe, but the skin
    /// of a root is dull brown, so the flesh goes only where the root is
    /// broken: the topmost face (both cameras look down, §2b points 11 and
    /// 35) and the +X end the game's fixed isometric sees (point 11).
    ///
    /// **The +X leg is measured as the FACE it is, not as "some flesh out at
    /// the far end"** — the first cut of this test asked only whether any
    /// voxel at the model's maximum x was flesh, and the review of PR #179
    /// deleted the entire +X repaint with the test still green, because the
    /// dusting of ground root on the FLOOR reaches x 13 as well and a single
    /// speck at (13, 3, 0) satisfied it. So the floor is excluded and the
    /// whole +X-facing surface of the root's far end has to be flesh.
    ///
    /// The third leg is a VOLUME ratio and says so. It is not the frame's
    /// area fraction: the repaint puts the cut on exactly the faces this
    /// camera sees, so a play-zoom frame shows far more of the cut than a
    /// sixth of the root. What keeps that off the hazard-marking reading is
    /// the root's small organic shape, not its share of the picture —
    /// `palette.ron`'s `turmeric_orange` entry carries that argument.
    #[test]
    fn the_turmeric_root_shows_its_cut_faces_to_both_cameras() {
        let p = palette();
        let root = item_turmeric(&p);
        let flesh = p.idx("turmeric_orange");
        let skin = p.idx("cardboard_dark");
        let top = root.voxels.keys().map(|at| at.2).max().unwrap();
        let top_face: Vec<_> = root.voxels.iter().filter(|e| e.0.2 == top).collect();
        assert!(
            top_face.iter().all(|e| *e.1 == flesh),
            "the highest face of the root is all cut flesh"
        );
        // The +X cut: every voxel of the root's +X-facing surface from the
        // repaint's own x 11 outward, with the floor left out so the ground
        // dusting cannot stand in for it.
        let facing: Vec<_> = root
            .voxels
            .iter()
            .filter(|e| {
                e.0.2 > 0
                    && e.0.0 >= 11
                    && !root.filled(e.0.0 as i32 + 1, e.0.1 as i32, e.0.2 as i32)
            })
            .collect();
        assert!(
            facing.len() >= top_face.len(),
            "the +X cut ({} voxels) is no smaller than the top cut ({}) — the game's camera \
             is the one a player watches an item on a belt with",
            facing.len(),
            top_face.len()
        );
        assert!(
            facing.iter().all(|e| *e.1 == flesh),
            "and all of it is cut flesh rather than skin (only {} of {} are)",
            facing.iter().filter(|e| *e.1 == flesh).count(),
            facing.len()
        );
        let cut = root.voxels.values().filter(|&&c| c == flesh).count();
        let body = root.voxels.values().filter(|&&c| c == skin).count();
        assert!(
            cut * 4 < body,
            "and by VOLUME the cut stays a fraction of a dark root ({cut} flesh against \
             {body} skin) — see this test's doc for what the frame shows"
        );
    }

    /// **A pail's contents take the class of the SUBSTANCE, not of the
    /// pail.** The `liquid` class is a gloss that says "this is wet", and
    /// all three pails in the game earn it: tomato paste, mixed paste and
    /// finished ketchup are wet. The other half of the rule is in
    /// `palette.ron`'s `bulk_pale` — the same gloss on a dry solid reads as
    /// sauce in a tank rather than product in a bin — so a pail of flour,
    /// which [`pail`] is perfectly able to draw, would have to pass a
    /// `matte` colour.
    ///
    /// This is the test [`pail`] defers to, and it is per-CALLER on purpose.
    /// The contract used to live in the helper as "`contents` is a `liquid`
    /// colour", which is a claim about the drawing code when the truth is a
    /// claim about each substance; asserted here, a fourth pail has to say
    /// what is in it and why that is wet or dry.
    ///
    /// Mustard is the case that forced the distinction, and since
    /// 2026-10-03 the dry pail it describes exists: the crusher grinds seed
    /// to a flour that rides the belt, so its fill is `matte`.
    #[test]
    fn a_pail_s_contents_take_the_class_of_the_substance() {
        let p = palette();
        for (name, model, colour, class) in [
            (
                "item_paste_pail",
                item_paste_pail(&p),
                "liquid_paste",
                MaterialClass::Liquid,
            ),
            (
                "item_mix_pail",
                item_mix_pail(&p),
                "liquid_mix",
                MaterialClass::Liquid,
            ),
            (
                "item_ketchup_pail",
                item_ketchup_pail(&p),
                "liquid_ketchup",
                MaterialClass::Liquid,
            ),
            (
                "item_ground_mustard",
                item_ground_mustard(&p),
                "mustard_gold",
                MaterialClass::Matte,
            ),
        ] {
            let contents = p.idx(colour);
            assert_eq!(
                p.class_of(contents),
                class,
                "{colour} is the `{}` class, because of what it IS",
                class.name()
            );
            assert!(
                model.voxels.values().any(|&c| c == contents),
                "{name} shows `{colour}` and nothing else stands in for it"
            );
            // And the surface keeps its headspace, so whichever class it is
            // is visible from above rather than reading as a lid.
            let fill = model
                .voxels
                .iter()
                .filter(|e| *e.1 == contents)
                .map(|e| e.0.2)
                .max()
                .unwrap();
            assert!(
                fill < 9 && !model.filled(8, 8, 9),
                "{name}'s mouth is open above a fill line at {fill}"
            );
        }
    }

    /// **A bottle's TOP has to contrast with its body**, and the shared
    /// `glass_pale` neck makes that harder for a bright body: `cap_white`
    /// tops a dark ketchup or BBQ bottle and loses a `mustard_gold` one,
    /// which is why [`bottle`] takes a `cap` at all.
    ///
    /// **The pairwise legs are not what catches that**, and the first cut of
    /// this test claimed they were (the review of PR #179 falsified it in
    /// thirty seconds): `cap_white` on a mustard body is 26.6 from the neck
    /// and 31.6 from the body, so both clear the 26/255 anchor and the bottle
    /// still loses its top. What fails is the top taken TOGETHER. At the
    /// ~16px an item draws on a belt the three-voxel neck and the two-voxel
    /// cap are one mark of about five pixels, and their voxel-weighted value
    /// is what the eye compares with the body: 213.9 against a 192.7 body,
    /// 21.2 apart. Mustard's cap is `label_mustard`, a brighter white, which
    /// puts that mark at about 224 — 31.3 over the body. (It was a dark ink
    /// at 111.1 until the owner ruled mustard yellow and white on
    /// 2026-10-02; this test holds whichever way up the contrast runs.)
    ///
    /// So all three legs are asserted as properties rather than as colours,
    /// and the combined one is the one that bites: a fourth sauce copying
    /// `cap_white` onto a pale body fails here instead of in a screenshot.
    #[test]
    fn a_bottle_s_cap_contrasts_with_the_neck_and_the_body() {
        let p = palette();
        let luma = |c: u8| {
            let [r, g, b] = p.rgb_of(c).expect("a palette colour");
            0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64
        };
        for (name, model) in [
            ("item_bottle", item_bottle(&p)),
            ("item_bottle_bbq", item_bottle_bbq(&p)),
            ("item_bottle_mustard", item_bottle_mustard(&p)),
        ] {
            let at = |z| *model.voxels.get(&(8, 8, z)).expect("the bottle's axis");
            let (body, neck, cap) = (at(0), at(11), at(14));
            assert_eq!(
                neck,
                p.idx("glass_pale"),
                "{name} has the shared glass neck"
            );
            assert!(
                (luma(cap) - luma(neck)).abs() >= 26.0,
                "{name}'s cap is {:.1} from its neck, and 26 is what reads",
                (luma(cap) - luma(neck)).abs()
            );
            assert!(
                (luma(cap) - luma(body)).abs() >= 26.0,
                "{name}'s cap is {:.1} from its body",
                (luma(cap) - luma(body)).abs()
            );
            // The leg that bites: neck and cap as the one mark they are at
            // item size, voxel-weighted over the voxels each contributes,
            // against the body colour that fills the rest of the silhouette.
            let neck_z = model
                .voxels
                .iter()
                .filter(|e| *e.1 == neck)
                .map(|e| e.0.2)
                .min()
                .expect("the glass neck starts somewhere");
            let above: Vec<f64> = model
                .voxels
                .iter()
                .filter(|e| e.0.2 >= neck_z)
                .map(|e| luma(*e.1))
                .collect();
            let mark = above.iter().sum::<f64>() / above.len() as f64;
            assert!(
                (mark - luma(body)).abs() >= 26.0,
                "{name}'s neck and cap together read as {mark:.1} against a body at {:.1} \
                 ({:.1} apart), and at ~16px that pair is one mark of five pixels",
                luma(body),
                (mark - luma(body)).abs()
            );
        }
    }

    /// **Mustard is yellow and white, not yellow and black** — the project
    /// owner's ruling of 2026-10-02 (DECISIONS.md), which replaced a
    /// near-black olive band and cap. So the bottle's two accents, its label
    /// band and its cap, are asserted as a WHITE: brighter than the gold
    /// body by the 26/255 that reads, and near-neutral (no channel more than
    /// a few points off the others), so a later "legibility" fix cannot
    /// quietly darken them again or slide them to another tint.
    #[test]
    fn the_mustard_bottle_is_yellow_and_white() {
        let p = palette();
        let luma = |c: u8| {
            let [r, g, b] = p.rgb_of(c).expect("a palette colour");
            0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64
        };
        let m = item_bottle_mustard(&p);
        let at = |z| *m.voxels.get(&(12, 8, z)).expect("the bottle's outer wall");
        let (body, band) = (at(0), at(3));
        let cap = *m.voxels.get(&(8, 8, 14)).expect("the bottle's cap");
        assert_eq!(body, p.idx("mustard_gold"), "the mustard bottle is yellow");
        for (what, c) in [("label band", band), ("cap", cap)] {
            let [r, g, b] = p.rgb_of(c).expect("a palette colour");
            let spread = r.max(g).max(b) - r.min(g).min(b);
            assert!(
                luma(c) - luma(body) >= 26.0,
                "the mustard bottle's {what} is {:.1} against a {:.1} body — white, not black, \
                 and bright enough to read as such",
                luma(c),
                luma(body)
            );
            assert!(
                spread <= 16,
                "the mustard bottle's {what} is #{r:02X}{g:02X}{b:02X}, {spread} between \
                 channels — a tint, not white"
            );
        }
    }

    /// **The BBQ branch's three raws part company by OUTLINE before they
    /// part by colour** (issue #67). They are bought from one plate that
    /// shows every unlocked ingredient at 64px side by side, where a thing is
    /// identified by the proportion of its parts (UI_UX §2b point 31) — so
    /// the bag is the shortest of the three, the jar is narrow and tall,
    /// and the drum is the widest. This is the claim two rework passes were
    /// spent on, and the one a later edit is most likely to undo by
    /// widening the jar or raising the bag a couple of voxels "to balance
    /// the row".
    ///
    /// The bag replaced a wide, low spice TIN on 2026-09-21 and the
    /// assertions moved with it, which is the honest way round: the claim
    /// is that the three outlines stay distinct, not that any one of them
    /// keeps a particular shape for ever.
    #[test]
    fn the_new_raw_ingredients_do_not_share_a_silhouette() {
        let p = palette();
        let (bag_w, _, bag_h) = extent(&item_spice_mix(&p));
        let (jar_w, _, jar_h) = extent(&item_molasses(&p));
        let (drum_w, _, drum_h) = extent(&item_liquid_smoke(&p));
        assert!(
            bag_h < jar_h && bag_h < drum_h,
            "the spice bag is the shortest of the three (bag {bag_h}, jar {jar_h}, drum {drum_h})"
        );
        assert!(
            jar_h > jar_w,
            "the molasses jar is taller than it is wide ({jar_w} x {jar_h})"
        );
        assert!(
            jar_w < bag_w && bag_w < drum_w,
            "the three widths stay ordered jar {jar_w} < bag {bag_w} < drum {drum_w}"
        );
        let outlines = [(bag_w, bag_h), (jar_w, jar_h), (drum_w, drum_h)];
        for (i, a) in outlines.iter().enumerate() {
            for b in &outlines[i + 1..] {
                assert_ne!(a, b, "no two of the three have the same outline");
            }
        }
    }

    /// **The spice bag is the sugar sack ONE SIZE DOWN, and that is the
    /// whole of what keeps them apart on a belt** (the project owner's
    /// 2026-09-21 call to make spice a bag). The two now share a shape, so
    /// the row argument the test above makes cannot be what separates
    /// them: size and paper are, and both are asserted here because a
    /// later edit "rounding out" the spice bag would silently ship two
    /// identical sacks of different things.
    #[test]
    fn the_spice_bag_is_smaller_than_the_sugar_sack_in_every_dimension() {
        let p = palette();
        let (sw, sd, sh) = extent(&item_sugar(&p));
        let (bw, bd, bh) = extent(&item_spice_mix(&p));
        assert!(
            bw < sw && bd < sd && bh < sh,
            "the spice bag {bw}x{bd}x{bh} is smaller than the sugar sack {sw}x{sd}x{sh} \
             in all three"
        );
        let spice = item_spice_mix(&p);
        let sugar = item_sugar(&p);
        assert!(
            !spice.voxels.values().any(|&c| c == p.idx("sack_kraft")),
            "the spice bag's paper is `sack_spice`, never sugar's undyed kraft"
        );
        assert!(
            !sugar.voxels.values().any(|&c| c == p.idx("sack_spice")),
            "and the sugar sack's is never the spice paper"
        );
    }

    /// **A bag of spice has to show the spice** — the blend spills from its
    /// mouth and is the topmost thing on the model, or what ships is a
    /// small brown bag and the market offers two sacks. Asserted as
    /// "paprika above the highest paper" rather than by a voxel count,
    /// because the failure worth catching is a future edit tying the neck
    /// shut over the spill.
    #[test]
    fn the_spice_bag_shows_its_blend_at_the_mouth() {
        let p = palette();
        let bag = item_spice_mix(&p);
        let paprika = p.idx("spice_paprika");
        let top_of = |colour: u8| {
            bag.voxels
                .iter()
                .filter(|&(_, &c)| c == colour)
                .map(|(at, _)| at.2)
                .max()
                .unwrap_or(0)
        };
        assert!(
            bag.voxels.values().any(|&c| c == paprika),
            "the bag is filled from the palette's own `spice_paprika`"
        );
        assert!(
            top_of(paprika) > top_of(p.idx("sack_spice")),
            "the spill ({}) stands above the paper ({})",
            top_of(paprika),
            top_of(p.idx("sack_spice"))
        );
        assert!(
            top_of(paprika) > top_of(p.idx("label_kraft")),
            "and above the tie ({})",
            top_of(p.idx("label_kraft"))
        );
    }

    /// **`liquid` means "this is the product" on a machine, and nothing
    /// else may use it** (2026-09-21). `main.gd` tints one mesh per machine
    /// with the sauce its live recipe is making, and the mesh it reaches
    /// for is the `liquid` one — so a second `liquid` colour anywhere on
    /// one of these bodies would be repainted as sauce every time the
    /// recipe changed. `machine_product` is the only way in, and this is
    /// the assertion that keeps it the only way in.
    #[test]
    fn a_machine_spends_its_liquid_class_on_the_product_alone() {
        let p = palette();
        let product = machine_product(&p);
        assert_eq!(
            p.class_of(product),
            MaterialClass::Liquid,
            "the product colour has to BE the liquid class or nothing is split out"
        );
        for (name, model) in [
            ("machine_bottler", machine_bottler(&p)),
            ("machine_packer", machine_packer(&p)),
            ("machine_cooking_vat", machine_cooking_vat(&p)),
        ] {
            let mut liquid = model
                .voxels
                .values()
                .copied()
                .filter(|&c| p.class_of(c) == MaterialClass::Liquid);
            assert!(
                liquid.clone().next().is_some(),
                "{name} has a product mesh at all"
            );
            assert!(
                liquid.all(|c| c == product),
                "{name} spends `liquid` on nothing but `machine_product`"
            );
        }
    }

    /// **The jar keeps a band of empty glass above its fill line.** Its own
    /// doc calls the headspace load-bearing and `pail`/`item_vinegar` play
    /// the same trick: without it, a jar of something nearly black is a
    /// black block, and "container of dark syrup" is exactly what has to
    /// survive at 64px.
    #[test]
    fn the_molasses_jar_has_a_headspace_above_its_syrup() {
        let p = palette();
        let jar = item_molasses(&p);
        let syrup = p.idx("molasses_black");
        let glass = p.idx("glass_pale");
        let fill_line = jar
            .voxels
            .iter()
            .filter(|&(_, &c)| c == syrup)
            .map(|(at, _)| at.2)
            .max()
            .expect("the jar holds syrup");
        assert!(
            jar.voxels
                .iter()
                .any(|(at, &c)| c == glass && at.2 > fill_line),
            "empty glass stands above the fill line at z = {fill_line}"
        );
    }

    /// **The drum is authored for VENGI's camera, not the game's** (§2b
    /// point 35): nothing ever puts it on a belt or a tile — its only
    /// consumer is the icon pass — and the two cameras look at opposite
    /// corners of the same model. Its bung is therefore deliberately on the
    /// -X/+Y side, which looks wrong in the game's frame and is right in the
    /// only frame that exists. `item_vinegar` is the precedent; this test is
    /// here because "fixing" it to match every other item is a one-line edit
    /// that no render in this repo would catch.
    #[test]
    fn the_liquid_smoke_drum_faces_the_camera_that_photographs_it() {
        let p = palette();
        let drum = item_liquid_smoke(&p);
        let centre = ITEM_VOXELS / 2;
        let brass = p.idx("brass");
        let bung: Vec<_> = drum
            .voxels
            .iter()
            .filter(|&(_, &c)| c == brass)
            .map(|(at, _)| *at)
            .collect();
        assert!(!bung.is_empty(), "the drum has a brass bung");
        for (x, y, _) in bung {
            assert!(
                x < centre && y > centre,
                "the bung sits on vengi's -X/+Y corner, not the game camera's"
            );
        }
        assert!(
            drum.voxels.values().any(|&c| c == p.idx("smoke_slate")),
            "the drum's body is the palette's `smoke_slate`, the number FLUID_TINTS mirrors"
        );
    }

    /// **The four slots are genuinely empty openings**, not dark ones
    /// (DECISIONS.md Part B). The rack's whole readout is that a player can
    /// see the stock standing in it, so a single stray voxel left across a
    /// slot by a future edit — a brace dropped a level, a deck that
    /// overhangs, a back panel — is the one defect that costs the asset its
    /// reason to exist, and it is invisible in a preview of the *empty*
    /// rack.
    #[test]
    fn the_racks_slots_are_empty_volumes() {
        let rack = warehouse_rack(&palette());
        let load_h = RACK_PALLET_SIZE[2] as i32 + RACK_LOAD_MAX_H;
        for (bay, &(y0, y1)) in RACK_BAY_Y.iter().enumerate() {
            for (level, &base) in RACK_LEVEL_Z.iter().enumerate() {
                for x in RACK_BAY_X.0..=RACK_BAY_X.1 {
                    for y in y0..=y1 {
                        for z in base..base + load_h {
                            assert!(
                                !rack.filled(x, y, z),
                                "bay {bay} level {level} is obstructed at ({x}, {y}, {z})"
                            );
                        }
                    }
                }
            }
        }
    }

    /// A full load still stands clear of what is above it.
    ///
    /// The camera makes this concrete: a sight line leaves every voxel along
    /// `(+x, -y, +z)`, so the lower level's goods start disappearing behind
    /// the front load beam once their top reaches it. [`RACK_LOAD_MAX_H`] is
    /// what `main.gd` scales a full slot to, so the two numbers are wired
    /// together whether or not anyone remembers it — hence the assert rather
    /// than the comment.
    #[test]
    fn a_full_slot_stands_clear_of_the_frame() {
        let load_top = |base: i32| base + RACK_PALLET_SIZE[2] as i32 + RACK_LOAD_MAX_H - 1;
        // Every level but the top one is roofed by the beam pair above it;
        // the top one by the posts' own ceiling.
        for (level, &base) in RACK_LEVEL_Z.iter().enumerate() {
            let ceiling = RACK_BEAM_Z
                .get(level)
                .map(|span| span.0)
                .unwrap_or(RACK_POST_TOP_Z);
            assert!(
                load_top(base) < ceiling,
                "a full load on level {level} tops out at z = {}, at or above the {ceiling} over it",
                load_top(base)
            );
        }
        // Each level above the bottom one stands on a deck over a beam pair,
        // not above thin air.
        for (level, &span) in RACK_BEAM_Z.iter().enumerate() {
            assert_eq!(
                RACK_LEVEL_Z[level + 1],
                span.1 + 1,
                "level {} does not sit on the deck laid over its beams",
                level + 1
            );
        }
        assert_eq!(
            RACK_LEVEL_Z.len(),
            RACK_BEAM_Z.len() + 1,
            "every level but the bottom one needs a beam pair to stand on"
        );
        // And a pallet never touches the posts it stands between, or a slot
        // reads as filled even when its stock is nearly gone.
        let bay_w = RACK_BAY_X.1 - RACK_BAY_X.0 + 1;
        let bay_d = RACK_BAY_Y[0].1 - RACK_BAY_Y[0].0 + 1;
        assert!(
            (RACK_PALLET_SIZE[0] as i32) < bay_w && (RACK_PALLET_SIZE[1] as i32) < bay_d,
            "the pallet ({} x {}) is inset from its bay ({bay_w} x {bay_d})",
            RACK_PALLET_SIZE[0],
            RACK_PALLET_SIZE[1]
        );
    }

    /// **The beams run the rack's whole length.** They are the feature that
    /// makes the thing read as racking rather than as a row of hoops — the
    /// defect the one-row version shipped with — and deleting them is a
    /// one-line edit that no other check here would notice.
    #[test]
    fn the_load_beams_span_the_whole_run() {
        let rack = warehouse_rack(&palette());
        let last_y = TILE_VOXELS as i32 * 2 - 1;
        for &(x0, x1) in &RACK_POST_X {
            for &span in &RACK_BEAM_Z {
                for y in 0..=last_y {
                    assert!(
                        rack.filled(x0, y, span.0),
                        "the z {} beam in the x {x0}..{x1} plane is broken at y = {y}",
                        span.0
                    );
                }
            }
        }
    }

    /// The rack fills its `data/machines.ron` **1x2** footprint and nothing
    /// more, and it stays short of the cooking vat: a storage building as
    /// tall as the chain's centrepiece would stand over the line it sits
    /// beside and hide it (UI_UX.md's rule that height is spent on what
    /// matters). The footprint half of this is the hard one — the height is
    /// free, but the plan the rack has to fit is 1x2.
    #[test]
    fn the_rack_keeps_to_its_footprint_and_its_height() {
        let rack = warehouse_rack(&palette());
        assert_eq!(
            rack.size,
            [TILE_VOXELS, TILE_VOXELS * 2, RACK_HEIGHT],
            "the rack is the 1x2 footprint x RACK_HEIGHT"
        );
        let vat = machine_cooking_vat(&palette());
        assert!(
            rack.size[2] < vat.size[2],
            "the rack ({}) has grown to the cooking vat's {}",
            rack.size[2],
            vat.size[2]
        );
        let tallest = rack.voxels.keys().map(|at| at.2).max().unwrap();
        assert_eq!(
            tallest as i32, RACK_CAP_Z,
            "the post caps are the highest thing on the rack"
        );
    }

    /// The bays are **identical**, which is what lets one pallet model and
    /// one placement rule serve all four slots in `main.gd`.
    #[test]
    fn every_bay_is_the_same_size() {
        let spans: Vec<i32> = RACK_BAY_Y.iter().map(|&(a, b)| b - a).collect();
        assert!(
            spans.iter().all(|s| *s == spans[0]),
            "the bays differ in depth: {spans:?}"
        );
        for (frame, bay) in RACK_FRAME_Y.iter().zip(RACK_BAY_Y.iter()) {
            assert_eq!(frame.1 + 1, bay.0, "a bay starts where its frame ends");
        }
        let last = RACK_BAY_Y[RACK_BAY_Y.len() - 1].1;
        assert_eq!(
            last + 1,
            RACK_FRAME_Y[RACK_FRAME_Y.len() - 1].0,
            "the last bay ends where the end frame begins"
        );
    }

    /// The title sign's bulb chase: every frame holds every bulb at the same
    /// place (only which are lit changes), consecutive frames differ, the
    /// lit ones march, and no bulb lands on a body voxel — the overlay rule
    /// the machine motion frames follow (§2b point 27).
    #[test]
    fn the_title_sign_bulb_frames_chase_and_never_overlap_the_body() {
        let p = palette();
        let body = title_sign(&p);
        let frames = [
            title_sign_bulbs_f0(&p),
            title_sign_bulbs_f1(&p),
            title_sign_bulbs_f2(&p),
        ];
        assert_eq!(frames.len(), TITLE_SIGN_BULB_FRAMES);
        let lit = p.idx("marquee_bulb");
        for (k, frame) in frames.iter().enumerate() {
            assert_eq!(
                frame.size, body.size,
                "frame {k} must share the body's volume"
            );
            let cells: Vec<_> = frame.voxels.keys().collect();
            let first: Vec<_> = frames[0].voxels.keys().collect();
            assert_eq!(cells, first, "frame {k} must hold every bulb, lit or dark");
            assert!(
                frame
                    .voxels
                    .keys()
                    .all(|cell| !body.voxels.contains_key(cell)),
                "frame {k} draws over a body voxel"
            );
            assert!(
                frame.voxels.values().any(|&c| c == lit),
                "frame {k} lights nothing"
            );
            let next = &frames[(k + 1) % frames.len()];
            assert_ne!(
                frame.voxels, next.voxels,
                "frames {k} and its successor are identical"
            );
        }
    }

    /// The whole word fits the `.vox` ceiling with its frame round it, and
    /// its lettered face is on the low-Y side the importer turns toward the
    /// title camera (`title_sign_face_front`).
    #[test]
    fn the_title_sign_fits_and_faces_the_camera() {
        let p = palette();
        let sign = title_sign(&p);
        assert!(
            sign.size.iter().all(|&n| n <= 256),
            "{:?} is past the .vox ceiling",
            sign.size
        );
        let neon = p.idx("sign_neon");
        let min_neon_y = sign
            .voxels
            .iter()
            .filter(|&(_, &c)| c == neon)
            .map(|(&(_, y, _), _)| y)
            .min()
            .expect("the sign has lit letters");
        assert_eq!(
            min_neon_y, 0,
            "the letters must stand on the front (low Y) face"
        );
    }

    /// A stacked rod length continues the body's rods exactly: the same
    /// columns and depth, the body's top rod layer and the piece's bottom
    /// layer holding the same (x, y) cells, and nothing else in the piece.
    #[test]
    fn the_title_sign_rod_lengths_continue_the_body_rods() {
        let p = palette();
        let body = title_sign(&p);
        let rods = title_sign_rods(&p);
        assert_eq!(rods.size[0], body.size[0], "same width as the body");
        assert_eq!(rods.size[1], body.size[1], "same depth as the body");
        assert_eq!(rods.size[2], TITLE_SIGN_ROD_SEGMENT as u32);
        let top = body.size[2] - 1;
        let mut body_top: Vec<_> = body
            .voxels
            .keys()
            .filter(|&&(_, _, z)| z == top)
            .map(|&(x, y, _)| (x, y))
            .collect();
        body_top.sort();
        for layer in 0..rods.size[2] {
            let mut cells: Vec<_> = rods
                .voxels
                .keys()
                .filter(|&&(_, _, z)| z == layer)
                .map(|&(x, y, _)| (x, y))
                .collect();
            cells.sort();
            assert_eq!(
                cells, body_top,
                "rod layer {layer} must match the body's rod tops"
            );
        }
    }

    /// `title_scene.gd` frames the sign from three of this family's numbers —
    /// how many bulb frames to load, how far above the board the rods
    /// reach (it hangs the sign from their top), and how long a stacked rod
    /// length is (it stacks them on up to the top of the frame). GDScript cannot import a
    /// Rust constant, so its copies are checked against these instead: a
    /// change here that left them behind would frame the sign wrongly with
    /// every Godot test still green (PR #226's review).
    #[test]
    fn title_scene_gd_mirrors_the_sign_constants() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../godot_project/title_scene.gd");
        let script = std::fs::read_to_string(&path).expect("read title_scene.gd");
        for (name, value) in [
            ("SIGN_BULB_FRAMES", TITLE_SIGN_BULB_FRAMES as i32),
            ("SIGN_ROD_VOXELS", TITLE_SIGN_ROD),
            ("SIGN_ROD_SEGMENT_VOXELS", TITLE_SIGN_ROD_SEGMENT),
        ] {
            let line = format!("const {name} := {value}");
            assert!(
                script.lines().any(|l| l.trim() == line),
                "title_scene.gd must declare `{line}` to match assets.rs"
            );
        }
    }
}
