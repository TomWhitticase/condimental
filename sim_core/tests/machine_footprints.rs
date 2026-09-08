//! Headless integration test for the "Machines improvement" milestone's
//! ticket 1: machine footprints are data (`data/machines.ron`), and the
//! chain's layout math uses each stage's own footprint width instead of a
//! fixed per-stage span.
//!
//! `sim_core/src/sim/machines.rs` already unit-tests the registry in
//! isolation (schema version, degenerate footprints, duplicate ids, unknown
//! kinds). This file instead drives the real `World` two ways:
//!
//! 1. **The shipped-layout proof.** Ticket 2 made the shipped content use
//!    the mechanism for real: the cooking vat is 2x2, so the chain's tiles
//!    are no longer what a fixed one-tile-per-machine span would produce.
//!    The expected coordinates below are written out literally, computed by
//!    hand from the footprint rule rather than recomputed from the code —
//!    a test that derived them the same way the code does could not catch
//!    the code being wrong.
//! 2. **The mechanism proof.** A test-only registry with genuinely wide
//!    kinds, so the width math, the "next belt starts after the whole
//!    footprint" rule, multi-tile `query_tile` resolution, and the
//!    chunk-boundary case are all observed at sizes no shipped kind has.
//!
//! It also pins the exact JSON field names ticket 4's Godot hover/render
//! work will consume, the way `fluids.rs` pins `fluid_snapshot_json`'s.

use std::collections::HashSet;

use sim_core::sim::machines::{Footprint, MachineRegistry};
use sim_core::sim::{
    ContractRegistry, FluidSnapshot, PlayerCommand, RecipeRegistry, TileSnapshot, World,
    reference_chain, reference_chain_with,
};

/// The world's one fluid network, or `None` when no pipe is placed. A world
/// can hold several since the "Real Placement" epic; the reference chain
/// lays one.
fn the_network(world: &World) -> Option<FluidSnapshot> {
    let mut networks = world.fluid_snapshots();
    assert!(networks.len() <= 1, "the reference chain lays one pipe run");
    networks.pop()
}

/// A "stage" as this file talks about one: a machine plus the run of belt
/// tiles feeding it.
///
/// **This is derived by the test now, not reported by the simulation.** The
/// "Real Placement" epic deleted `chain_layout` — there is no chain and no
/// stage in the sim any more, only placements — so this file reconstructs
/// the concept from `world_layout` in order to keep asserting exactly what
/// it always asserted about the shipped geometry. Walking west from a
/// machine's origin over contiguous belt tiles is also a stronger check than
/// the old one: it fails if the belts feeding a machine are missing, not
/// just if they are in the wrong place.
#[derive(Debug, Clone)]
struct StageView {
    recipe: String,
    machine_kind: String,
    machine_tile: (i32, i32),
    machine_footprint: Footprint,
    belt_start_tile: (i32, i32),
    belt_length_tiles: i32,
}

fn stages(world: &World) -> Vec<StageView> {
    let layout = world.world_layout();
    let mut stages: Vec<StageView> = layout
        .iter()
        .filter(|p| p.recipe.is_some())
        .map(|p| {
            let mut belt_length_tiles = 0;
            while matches!(
                world.query_tile((p.tile.0 - belt_length_tiles - 1, p.tile.1)),
                Some(TileSnapshot::Belt { .. })
            ) {
                belt_length_tiles += 1;
            }
            StageView {
                recipe: p.recipe.clone().unwrap(),
                machine_kind: p.kind.clone(),
                machine_tile: p.tile,
                machine_footprint: p.footprint,
                belt_start_tile: (p.tile.0 - belt_length_tiles, p.tile.1),
                belt_length_tiles,
            }
        })
        .collect();
    stages.sort_by_key(|s| s.machine_tile.0);
    stages
}

fn shipped_recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A straight chain of `kinds.len()` stages: stage `i` consumes item `i` and
/// produces item `i + 1`, running on `kinds[i]`. Deliberately minimal — the
/// point of these chains is their *geometry*, not their content.
fn chain_on(kinds: &[&str]) -> RecipeRegistry {
    let mut recipes = String::new();
    for (i, kind) in kinds.iter().enumerate() {
        recipes.push_str(&format!(
            r#"Recipe(
                id: "item{next}",
                inputs: [("item{i}", 1)],
                outputs: [("item{next}", 1)],
                machine: "{kind}",
                time_ticks: 10,
            ),"#,
            next = i + 1
        ));
    }
    RecipeRegistry::from_ron_str(&format!(
        "( schema_version: 2, recipes: [{recipes}] )"
    ))
    .expect("generated recipe text must parse")
}

/// A registry of `(id, width, height)` kinds, plus the three 1x1 kinds every
/// reference chain needs regardless of its recipes: the `hopper`, the
/// `fluid_tank` (only used when a recipe declares `fluid_inputs`) and the
/// `dock`. All three take their footprint from this registry like any other
/// placed building. Added here rather than at each call site because none of
/// these fixtures are *about* any of them — they are about machine geometry.
///
/// **The `id`s these fixtures use are real kind ids** (`crusher`,
/// `cooking_vat`, ...) with fabricated *footprints*, which is a change the
/// "Real Placement" epic forced and is worth understanding: a machine is now
/// something the player buys and places, so it has to be priced in
/// `data/shop.ron`, and an invented kind like the old `"cooker"` is not.
/// Fabricating the footprint is still exactly what these fixtures need —
/// no shipped kind is 3 wide or 30 wide — and the recipes remain synthetic.
fn kinds(entries: &[(&str, i32, i32)]) -> MachineRegistry {
    let mut body: String = concat!(
        "MachineKind(id: \"hopper\", footprint: Footprint(width_tiles: 1, height_tiles: 1)),",
        "MachineKind(id: \"fluid_tank\", footprint: Footprint(width_tiles: 1, height_tiles: 1)),",
        "MachineKind(id: \"dock\", footprint: Footprint(width_tiles: 1, height_tiles: 1)),",
    )
    .to_string();
    body += &entries
        .iter()
        .map(|(id, w, h)| {
            format!(
                "MachineKind(id: \"{id}\", footprint: Footprint(width_tiles: {w}, height_tiles: {h})),"
            )
        })
        .collect::<String>();
    MachineRegistry::from_ron_str(&format!("( schema_version: 1, machines: [{body}] )"))
        .expect("generated machine text must parse")
}

// ---------------------------------------------------------------------
// 1. The shipped-layout proof: where ticket 2's content actually lands.
// ---------------------------------------------------------------------

/// The shipped chain's tile layout, as `(recipe, machine kind, belt start,
/// belt length, machine origin, footprint w x h)` per stage, worked out by
/// hand from the footprint rule (`data/machines.ron`'s header): a stage's
/// belt is 4 tiles, its machine's origin is the tile immediately after the
/// belt, and the next stage begins immediately after the machine's *whole*
/// width.
///
/// So: crusher 1 wide at x=4 (next stage at 5), mixer 1 wide at x=9 (next at
/// 10), **cooking vat 2 wide at x=14, covering 14 and 15** (next at 16, one
/// further east than a 1x1 would have put it), aging rack at x=20, bottler
/// at x=25, packer at x=30. Every stage's *origin* is on row 0; the vat is
/// the one machine that also claims row 1.
///
/// Milestone 7's aging rack is the reason the last two rows moved five tiles
/// east of where the "Machines improvement" epic left them: it is an
/// ordinary 1x1 stage inserted mid-chain, so it costs exactly one stage span
/// (4 belt tiles + 1 machine tile) and shifts everything downstream by that
/// much, changing nothing upstream. Note the packer's machine now sits at
/// x=30 — one tile short of the east edge of chunk (0, 0) (ARCHITECTURE.md
/// §3.1's 32x32 chunks). That headroom is why the rack is 1x1; see
/// `data/machines.ron`'s note on the kind.
const EXPECTED_SHIPPED_LAYOUT: [(&str, &str, (i32, i32), i32, (i32, i32), (i32, i32)); 6] = [
    ("tomato_paste", "crusher", (0, 0), 4, (4, 0), (1, 1)),
    ("ketchup_mix", "mixer", (5, 0), 4, (9, 0), (1, 1)),
    ("ketchup", "cooking_vat", (10, 0), 4, (14, 0), (2, 2)),
    ("aged_ketchup", "aging_rack", (16, 0), 4, (20, 0), (1, 1)),
    ("bottled_ketchup", "bottler", (21, 0), 4, (25, 0), (1, 1)),
    ("boxed_ketchup", "packer", (26, 0), 4, (30, 0), (1, 1)),
];

#[test]
fn the_shipped_chains_tile_layout_is_what_the_footprint_rule_says() {
    let world = reference_chain(shipped_recipes());
    let layout = stages(&world);

    assert_eq!(
        layout.len(),
        EXPECTED_SHIPPED_LAYOUT.len(),
        "data/recipes.ron's stage count changed — if that was deliberate, update \
         EXPECTED_SHIPPED_LAYOUT, but note this test exists to catch it happening by accident"
    );
    for (stage, expected) in layout.iter().zip(EXPECTED_SHIPPED_LAYOUT) {
        let (recipe, kind, belt_start, belt_len, machine_tile, footprint) = expected;
        assert_eq!(
            (
                stage.recipe.as_str(),
                stage.machine_kind.as_str(),
                stage.belt_start_tile,
                stage.belt_length_tiles,
                stage.machine_tile,
                (
                    stage.machine_footprint.width_tiles,
                    stage.machine_footprint.height_tiles
                ),
            ),
            (recipe, kind, belt_start, belt_len, machine_tile, footprint),
            "stage '{}' is not where the footprint rule puts it",
            stage.recipe
        );
    }
}

/// The payoff of ticket 1's accumulate-widths layout, observed on *shipped*
/// content rather than a fixture: the 2-wide cooking vat pushes everything
/// downstream of it one tile east of where a one-tile-per-machine span would
/// have placed it, and nothing upstream of it moves at all.
#[test]
fn the_cooking_vats_extra_width_shifts_only_what_is_downstream_of_it() {
    let world = reference_chain(shipped_recipes());
    let layout = stages(&world);

    // A fixed 5-tile-per-stage span (4 belt tiles + a 1-tile machine) is what
    // every stage would sit at if machine width were still assumed to be 1.
    for (i, stage) in layout.iter().enumerate() {
        let fixed_span_start = (i as i32) * 5;
        if i <= 2 {
            assert_eq!(
                stage.belt_start_tile.0, fixed_span_start,
                "stage {i} is at or before the vat, so its own tiles must not have moved"
            );
        } else {
            assert_eq!(
                stage.belt_start_tile.0,
                fixed_span_start + 1,
                "stage {i} is downstream of the 2-wide vat, so it must sit exactly one tile \
                 further east than a fixed one-tile-per-machine span would put it"
            );
        }
    }
}

/// Hovering a big machine has to work anywhere on it, not just on the corner
/// the belt feeds — checked here against the real shipped cooking vat, at
/// every one of the four tiles it occupies.
#[test]
fn every_tile_of_the_shipped_cooking_vat_resolves_to_the_cooking_vat() {
    let world = reference_chain(shipped_recipes());
    let stage = stages(&world)
        .into_iter()
        .find(|s| s.machine_kind == "cooking_vat")
        .expect("the shipped chain has a cooking vat");
    let origin = stage.machine_tile;
    assert_eq!(origin, (14, 0));
    assert_eq!(
        (
            stage.machine_footprint.width_tiles,
            stage.machine_footprint.height_tiles
        ),
        (2, 2)
    );

    for tile in [(14, 0), (15, 0), (14, 1), (15, 1)] {
        match world.query_tile(tile) {
            Some(TileSnapshot::Machine {
                ref machine_kind,
                footprint,
                origin_tile,
                ref recipe,
                ..
            }) => {
                assert_eq!(machine_kind, "cooking_vat", "at {tile:?}");
                assert_eq!(origin_tile, origin, "at {tile:?}");
                assert_eq!(recipe, "ketchup", "at {tile:?}");
                assert_eq!(
                    (footprint.width_tiles, footprint.height_tiles),
                    (2, 2),
                    "at {tile:?}"
                );
            }
            other => panic!("expected the cooking vat at {tile:?}, got {other:?}"),
        }
    }

    // Immediately outside it on every side: the belt tile west of the origin
    // belongs to the vat's own stage, the tile east of it starts the
    // bottler's belt, and the row above the vat is empty (the pipe run is
    // further west, beside the mixer).
    assert!(matches!(
        world.query_tile((13, 0)),
        Some(TileSnapshot::Belt { .. })
    ));
    assert!(matches!(
        world.query_tile((16, 0)),
        Some(TileSnapshot::Belt { .. })
    ));
    for tile in [(14, 2), (15, 2), (13, 1), (16, 1), (14, -1)] {
        assert!(
            world.query_tile(tile).is_none(),
            "{tile:?} is outside the vat's footprint and must be empty"
        );
    }
}

#[test]
fn every_shipped_stage_reports_the_machine_kind_its_recipe_names() {
    let recipes = shipped_recipes();
    let world = reference_chain(shipped_recipes());
    for (stage, recipe) in stages(&world).iter().zip(recipes.ordered()) {
        assert_eq!(stage.recipe, recipe.id);
        assert_eq!(stage.machine_kind, recipe.machine);
    }
}

/// The pipe run's position is derived from the fluid-fed machine's footprint
/// (the row immediately north of it) rather than a constant, so this pins
/// that the 1x1 case still lands exactly where milestone 6 put it: row 1,
/// four tiles ending beside the fluid-fed machine at x = 9. Ticket 2 moved
/// the fluid requirement from the old `ketchup`/reducer stage onto the new
/// `ketchup_mix`/mixer stage — which happens to sit at the same x, so the run
/// itself did not move, but it is now feeding a different machine entirely.
#[test]
fn the_shipped_pipe_run_feeds_the_mixer() {
    let world = reference_chain(shipped_recipes());
    let fluid = the_network(&world).expect("shipped chain has a pipe run");
    let tiles: Vec<(i32, i32)> = fluid.segments.iter().map(|s| s.tile).collect();
    assert_eq!(tiles, vec![(6, 1), (7, 1), (8, 1), (9, 1)]);
    let tap = fluid.taps.first().expect("a machine taps the run");
    assert_eq!(tap.machine_tile, (9, 0));
    assert_eq!(tap.segment_index, 3);
    assert_eq!(tap.machine_recipe, "ketchup_mix");

    // ...and the machine at that tile really is the mixer, not some other
    // stage that merely shares the coordinate.
    match world.query_tile(tap.machine_tile) {
        Some(TileSnapshot::Machine { machine_kind, .. }) => {
            assert_eq!(machine_kind, "mixer")
        }
        other => panic!("expected the mixer at the pipe run's machine tile, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 2. The mechanism proof: genuinely non-1x1 footprints.
// ---------------------------------------------------------------------

#[test]
fn a_wide_machine_pushes_every_downstream_stage_east_by_its_extra_width() {
    // Stage 1's machine is 3 wide; stages 0 and 2 are ordinary 1x1s.
    let world = reference_chain_with(
        chain_on(&["bottler", "crusher", "packer"]),
        kinds(&[("bottler", 1, 1), ("crusher", 3, 2), ("packer", 1, 1)]),
        ContractRegistry::bundled(),
    );
    let layout = stages(&world);

    // Stage 0 is untouched — a machine's size only affects what comes after
    // it, because the origin is the belt-facing corner.
    assert_eq!(layout[0].belt_start_tile, (0, 0));
    assert_eq!(layout[0].machine_tile, (4, 0));

    // Stage 1: belt immediately after stage 0's 1-tile machine, machine
    // immediately after its own 4 belt tiles.
    assert_eq!(layout[1].belt_start_tile, (5, 0));
    assert_eq!(layout[1].machine_tile, (9, 0));
    assert_eq!(layout[1].machine_footprint.width_tiles, 3);
    assert_eq!(layout[1].machine_footprint.height_tiles, 2);

    // The payoff: stage 2's belt starts after the WHOLE 3-tile footprint
    // (9, 10, 11), i.e. at 12 — not at 10, which is what a fixed one-tile
    // stage span would have produced.
    assert_eq!(layout[2].belt_start_tile, (12, 0));
    assert_eq!(layout[2].machine_tile, (16, 0));
}

#[test]
fn query_tile_resolves_a_machine_from_every_tile_of_its_footprint() {
    let world = reference_chain_with(
        chain_on(&["bottler", "crusher", "packer"]),
        kinds(&[("bottler", 1, 1), ("crusher", 3, 2), ("packer", 1, 1)]),
        ContractRegistry::bundled(),
    );
    let origin = stages(&world)[1].machine_tile;
    assert_eq!(origin, (9, 0));

    // All 3x2 = 6 tiles must resolve to the same machine, reporting the same
    // origin — that is what lets a hover on any part of a big machine work.
    for tile in [(9, 0), (10, 0), (11, 0), (9, 1), (10, 1), (11, 1)] {
        match world.query_tile(tile) {
            Some(TileSnapshot::Machine {
                ref machine_kind,
                footprint,
                origin_tile,
                ref recipe,
                ..
            }) => {
                assert_eq!(machine_kind, "crusher", "at {tile:?}");
                assert_eq!(origin_tile, origin, "at {tile:?}");
                assert_eq!(recipe, "item2", "at {tile:?}");
                assert_eq!(
                    (footprint.width_tiles, footprint.height_tiles),
                    (3, 2),
                    "at {tile:?}"
                );
            }
            other => panic!("expected the crusher at {tile:?}, got {other:?}"),
        }
    }

    // Just outside the footprint on every side. (8, 0) is stage 1's last
    // belt tile (a belt, not the machine); (12, 0) is stage 2's first belt
    // tile; the rest are empty.
    assert!(matches!(
        world.query_tile((8, 0)),
        Some(TileSnapshot::Belt { .. })
    ));
    assert!(matches!(
        world.query_tile((12, 0)),
        Some(TileSnapshot::Belt { .. })
    ));
    for tile in [(9, 2), (11, 2), (9, -1), (12, 1)] {
        assert!(
            world.query_tile(tile).is_none(),
            "{tile:?} is outside the footprint and must be empty"
        );
    }
}

#[test]
fn a_wide_machine_reports_its_footprint_in_chunk_snapshots() {
    let world = reference_chain_with(
        chain_on(&["bottler", "crusher"]),
        kinds(&[("bottler", 1, 1), ("crusher", 3, 2)]),
        ContractRegistry::bundled(),
    );
    let machines: Vec<_> = world
        .all_chunks()
        .into_iter()
        .flat_map(|c| world.chunk_snapshot(c).machines)
        .collect();
    let crusher = machines
        .iter()
        .find(|m| m.kind == "crusher")
        .expect("the crusher must appear in a chunk snapshot");
    assert_eq!(crusher.tile, (9, 0));
    assert_eq!(crusher.footprint.width_tiles, 3);
    assert_eq!(crusher.footprint.height_tiles, 2);
}

/// A tall machine would sit exactly where milestone 6's pipe run used to be
/// hardcoded (row 1), so the run is placed relative to the machine's own
/// footprint instead. This is the test that would fail if that reverted to a
/// constant.
#[test]
fn the_pipe_run_clears_a_tall_fluid_fed_machine_instead_of_overlapping_it() {
    let recipes = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 2,
            recipes: [
                Recipe(
                    id: "tomato_sauce",
                    inputs: [("tomato", 1)],
                    outputs: [("tomato_sauce", 1)],
                    machine: "bottler",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "ketchup",
                    inputs: [("tomato_sauce", 1)],
                    fluid_inputs: [("vinegar", 1)],
                    outputs: [("ketchup", 1)],
                    machine: "cooking_vat",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .unwrap();
    // The vat is 2x3: it occupies rows 0, 1 and 2, so the pipe run has to
    // rise to row 3 to stay adjacent-but-not-overlapping.
    let world = reference_chain_with(recipes, kinds(&[("bottler", 1, 1), ("cooking_vat", 2, 3)]), ContractRegistry::bundled());
    let fluid = the_network(&world).expect("this chain has a pipe run");
    assert_eq!(fluid.taps[0].machine_tile, (9, 0));
    assert_eq!(
        fluid.segments.iter().map(|s| s.tile).collect::<Vec<_>>(),
        vec![(6, 3), (7, 3), (8, 3), (9, 3)],
        "the run must sit on the row immediately north of the vat's 3-tall footprint"
    );
    // And every tile of the vat still resolves to the vat, not to a pipe.
    for tile in [(9, 0), (10, 0), (9, 2), (10, 2)] {
        assert!(
            matches!(world.query_tile(tile), Some(TileSnapshot::Machine { .. })),
            "{tile:?} must be the vat"
        );
    }
}

// ---------------------------------------------------------------------
// 3. Chunk boundaries (ARCHITECTURE.md §3.1's 32x32 chunks).
// ---------------------------------------------------------------------

/// A footprint crossing a chunk boundary is *handled*, not rejected: the
/// machine belongs to every chunk it overlaps, so a renderer that only loads
/// visible chunks still learns about it when its origin chunk is off-screen.
#[test]
fn a_machine_straddling_a_chunk_boundary_appears_in_both_chunks() {
    // Five 1x1 stages put stage 5's machine origin at x = 29; a 5-wide
    // machine there covers 29..=33, crossing the x = 32 chunk boundary.
    let world = reference_chain_with(
        chain_on(&["bottler", "crusher", "mixer", "packer", "cooking_vat", "aging_rack"]),
        kinds(&[
            ("bottler", 1, 1),
            ("crusher", 1, 1),
            ("mixer", 1, 1),
            ("packer", 1, 1),
            ("cooking_vat", 1, 1),
            ("aging_rack", 5, 1),
        ]),
        ContractRegistry::bundled(),
    );
    let last = stages(&world).last().unwrap().clone();
    assert_eq!(last.machine_tile, (29, 0));
    assert_eq!(last.machine_footprint.width_tiles, 5);

    let chunks = world.all_chunks();
    assert!(chunks.contains(&(0, 0)), "chunks: {chunks:?}");
    assert!(
        chunks.contains(&(1, 0)),
        "the chunk the footprint spills into must be listed too: {chunks:?}"
    );

    for chunk in [(0, 0), (1, 0)] {
        let snapshot = world.chunk_snapshot(chunk);
        let found = snapshot.machines.iter().find(|m| m.kind == "aging_rack");
        let found = found
            .unwrap_or_else(|| panic!("chunk {chunk:?} must report the straddling machine"));
        // Identical in both, including an origin tile that is outside chunk
        // (1, 0) — the documented de-duplication contract.
        assert_eq!(found.tile, (29, 0));
        assert_eq!(found.footprint.width_tiles, 5);
    }

    // And every tile resolves, on both sides of the boundary.
    for x in 29..=33 {
        match world.query_tile((x, 0)) {
            Some(TileSnapshot::Machine { origin_tile, .. }) => {
                assert_eq!(origin_tile, (29, 0), "at x = {x}")
            }
            other => panic!("expected the sprawler at x = {x}, got {other:?}"),
        }
    }
    // x = 34 is past the sprawler's east edge. Since the delivery-dock
    // ticket that tile is not empty — it is the first tile of the belt
    // running from the last machine into the chain's dock — but it must not
    // resolve as any part of the straddling machine, which is what this
    // check is actually for.
    match world.query_tile((34, 0)) {
        Some(TileSnapshot::Belt { .. }) => {}
        other => panic!(
            "x = 34 is one past the sprawler's footprint and should be the dock's inbound \
             belt, got {other:?}"
        ),
    }
}

#[test]
fn a_change_to_a_straddling_machine_dirties_both_of_its_chunks() {
    // One 30-wide first-stage machine at x = 4 covers 4..=33, crossing the
    // boundary; putting it first means it starts working within a few
    // hundred ticks of a single spawned item, which is what makes the dirty
    // marking observable without driving the whole chain.
    let world_kinds = kinds(&[("aging_rack", 30, 1), ("packer", 1, 1)]);
    let mut world = reference_chain_with(chain_on(&["aging_rack", "packer"]), world_kinds, ContractRegistry::bundled());
    assert_eq!(stages(&world)[0].machine_tile, (4, 0));

    world.issue_command(PlayerCommand::FillHopper { amount: 4, tile: None });

    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    for _ in 0..400 {
        world.tick();
        seen.extend(world.dirty_chunks().iter().copied());
    }
    assert!(seen.contains(&(0, 0)), "seen: {seen:?}");
    assert!(
        seen.contains(&(1, 0)),
        "the far half of the straddling footprint must be dirtied too: {seen:?}"
    );
}

// ---------------------------------------------------------------------
// 4. Content-error handling and the JSON wire shape.
// ---------------------------------------------------------------------

#[test]
#[should_panic(expected = "does not define")]
fn building_a_world_whose_recipe_names_an_undefined_kind_fails_loudly() {
    // "fermenter" exists in no registry here — construction must stop, not
    // guess a 1x1 footprint and silently mis-place the rest of the chain.
    reference_chain_with(
        chain_on(&["bottler", "fermenter"]),
        kinds(&[("bottler", 1, 1)]),
        ContractRegistry::bundled(),
    );
}

/// Pins the exact JSON field names ticket 4 (Godot rendering/hover) will
/// read. A rename here should fail this test, not silently break the scene —
/// the same guarantee `fluids.rs` gives `fluid_snapshot_json`.
#[test]
fn the_footprint_json_shape_is_what_presentation_is_told_to_expect() {
    let world = reference_chain_with(
        chain_on(&["bottler", "crusher"]),
        kinds(&[("bottler", 1, 1), ("crusher", 3, 2)]),
        ContractRegistry::bundled(),
    );

    // world_layout_json (which replaced chain_layout_json): one object per
    // placement, machines and belts alike.
    let layout: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.world_layout()).unwrap()).unwrap();
    let machine = layout
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["kind"] == "crusher")
        .expect("the crusher is a placement");
    assert_eq!(machine["tile"], serde_json::json!([9, 0]));
    assert_eq!(machine["direction"], "east");
    assert_eq!(
        machine["footprint"],
        serde_json::json!({ "width_tiles": 3, "height_tiles": 2 })
    );
    assert_eq!(machine["recipe"], "item2");
    assert_eq!(
        machine["occupies"],
        serde_json::json!([[9, 0], [10, 0], [11, 0], [9, 1], [10, 1], [11, 1]]),
        "`occupies` is the rotated rectangle, spelled out so a renderer needn't re-derive it"
    );
    // The belts feeding it are placements in their own right now, each one
    // tile — there is no `belt_start_tile`/`belt_length_tiles` anywhere.
    for x in 5..=8 {
        let belt = layout
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["kind"] == "belt" && p["tile"] == serde_json::json!([x, 0]))
            .unwrap_or_else(|| panic!("a belt tile must stand at ({x}, 0)"));
        assert_eq!(belt["direction"], "east");
        assert!(belt["recipe"].is_null(), "a belt runs no recipe");
    }

    // chunk_snapshot_json's machines array.
    let snapshot: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.chunk_snapshot((0, 0))).unwrap())
            .unwrap();
    let machine = snapshot["machines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["kind"] == "crusher")
        .expect("crusher present");
    assert_eq!(machine["tile"], serde_json::json!([9, 0]));
    assert_eq!(machine["direction"], "east");
    assert_eq!(
        machine["footprint"],
        serde_json::json!({ "width_tiles": 3, "height_tiles": 2 })
    );
    assert_eq!(machine["recipe"], "item2");

    // query_tile_json, asked about a NON-origin tile of the footprint.
    let tile: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.query_tile((11, 1)).unwrap()).unwrap())
            .unwrap();
    assert_eq!(tile["kind"], "machine");
    assert_eq!(tile["machine_kind"], "crusher");
    assert_eq!(tile["direction"], "east");
    assert_eq!(tile["origin_tile"], serde_json::json!([9, 0]));
    assert_eq!(
        tile["footprint"],
        serde_json::json!({ "width_tiles": 3, "height_tiles": 2 })
    );
    assert_eq!(tile["recipe"], "item2");
}
