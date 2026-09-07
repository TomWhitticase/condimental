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

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::{PlayerCommand, RecipeRegistry, TileSnapshot, World};

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

/// A registry of `(id, width, height)` kinds.
fn kinds(entries: &[(&str, i32, i32)]) -> MachineRegistry {
    let body: String = entries
        .iter()
        .map(|(id, w, h)| {
            format!(
                "MachineKind(id: \"{id}\", footprint: Footprint(width_tiles: {w}, height_tiles: {h})),"
            )
        })
        .collect();
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
/// further east than a 1x1 would have put it), bottler at x=20, packer at
/// x=25. Every stage's *origin* is on row 0; the vat is the one machine that
/// also claims row 1.
const EXPECTED_SHIPPED_LAYOUT: [(&str, &str, (i32, i32), i32, (i32, i32), (i32, i32)); 5] = [
    ("tomato_paste", "crusher", (0, 0), 4, (4, 0), (1, 1)),
    ("ketchup_mix", "mixer", (5, 0), 4, (9, 0), (1, 1)),
    ("ketchup", "cooking_vat", (10, 0), 4, (14, 0), (2, 2)),
    ("bottled_ketchup", "bottler", (16, 0), 4, (20, 0), (1, 1)),
    ("boxed_ketchup", "packer", (21, 0), 4, (25, 0), (1, 1)),
];

#[test]
fn the_shipped_chains_tile_layout_is_what_the_footprint_rule_says() {
    let world = World::new(shipped_recipes());
    let layout = world.chain_layout();

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
    let world = World::new(shipped_recipes());
    let layout = world.chain_layout();

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
    let world = World::new(shipped_recipes());
    let stage = world
        .chain_layout()
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
    let world = World::new(shipped_recipes());
    for (stage, recipe) in world.chain_layout().iter().zip(recipes.ordered()) {
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
    let world = World::new(shipped_recipes());
    let fluid = world.fluid_snapshot().expect("shipped chain has a pipe run");
    let tiles: Vec<(i32, i32)> = fluid.segments.iter().map(|s| s.tile).collect();
    assert_eq!(tiles, vec![(6, 1), (7, 1), (8, 1), (9, 1)]);
    assert_eq!(fluid.machine_tile, (9, 0));
    assert_eq!(fluid.inlet_index, 3);
    assert_eq!(fluid.machine_recipe, "ketchup_mix");

    // ...and the machine at that tile really is the mixer, not some other
    // stage that merely shares the coordinate.
    match world.query_tile(fluid.machine_tile) {
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
    let world = World::with_machines(
        chain_on(&["cooker", "crusher", "packer"]),
        kinds(&[("cooker", 1, 1), ("crusher", 3, 2), ("packer", 1, 1)]),
    );
    let layout = world.chain_layout();

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
    let world = World::with_machines(
        chain_on(&["cooker", "crusher", "packer"]),
        kinds(&[("cooker", 1, 1), ("crusher", 3, 2), ("packer", 1, 1)]),
    );
    let origin = world.chain_layout()[1].machine_tile;
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
    let world = World::with_machines(
        chain_on(&["cooker", "crusher"]),
        kinds(&[("cooker", 1, 1), ("crusher", 3, 2)]),
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
                    machine: "cooker",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "ketchup",
                    inputs: [("tomato_sauce", 1)],
                    fluid_inputs: [("vinegar", 1)],
                    outputs: [("ketchup", 1)],
                    machine: "vat",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .unwrap();
    // The vat is 2x3: it occupies rows 0, 1 and 2, so the pipe run has to
    // rise to row 3 to stay adjacent-but-not-overlapping.
    let world = World::with_machines(recipes, kinds(&[("cooker", 1, 1), ("vat", 2, 3)]));
    let fluid = world.fluid_snapshot().expect("this chain has a pipe run");
    assert_eq!(fluid.machine_tile, (9, 0));
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
    let world = World::with_machines(
        chain_on(&["cooker", "cooker", "cooker", "cooker", "cooker", "sprawler"]),
        kinds(&[("cooker", 1, 1), ("sprawler", 5, 1)]),
    );
    let last = world.chain_layout().last().unwrap().clone();
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
        let found = snapshot.machines.iter().find(|m| m.kind == "sprawler");
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
    assert!(world.query_tile((34, 0)).is_none());
}

#[test]
fn a_change_to_a_straddling_machine_dirties_both_of_its_chunks() {
    // One 30-wide first-stage machine at x = 4 covers 4..=33, crossing the
    // boundary; putting it first means it starts working within a few
    // hundred ticks of a single spawned item, which is what makes the dirty
    // marking observable without driving the whole chain.
    let world_kinds = kinds(&[("sprawler", 30, 1), ("packer", 1, 1)]);
    let mut world = World::with_machines(chain_on(&["sprawler", "packer"]), world_kinds);
    assert_eq!(world.chain_layout()[0].machine_tile, (4, 0));

    world.issue_command(PlayerCommand::SpawnItem {
        item: "item0".to_string(),
    });

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
    World::with_machines(
        chain_on(&["cooker", "fermenter"]),
        kinds(&[("cooker", 1, 1)]),
    );
}

/// Pins the exact JSON field names ticket 4 (Godot rendering/hover) will
/// read. A rename here should fail this test, not silently break the scene —
/// the same guarantee `fluids.rs` gives `fluid_snapshot_json`.
#[test]
fn the_footprint_json_shape_is_what_presentation_is_told_to_expect() {
    let world = World::with_machines(
        chain_on(&["cooker", "crusher"]),
        kinds(&[("cooker", 1, 1), ("crusher", 3, 2)]),
    );

    // chain_layout_json: one object per stage.
    let layout: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.chain_layout()).unwrap()).unwrap();
    let stage = &layout[1];
    assert_eq!(stage["machine_tile"], serde_json::json!([9, 0]));
    assert_eq!(stage["machine_kind"], "crusher");
    assert_eq!(
        stage["machine_footprint"],
        serde_json::json!({ "width_tiles": 3, "height_tiles": 2 })
    );
    // Pre-existing fields must be untouched, so an existing consumer keeps
    // working with no changes at all.
    assert_eq!(stage["belt_start_tile"], serde_json::json!([5, 0]));
    assert_eq!(stage["belt_length_tiles"], 4);
    assert_eq!(stage["recipe"], "item2");

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
    assert_eq!(tile["origin_tile"], serde_json::json!([9, 0]));
    assert_eq!(
        tile["footprint"],
        serde_json::json!({ "width_tiles": 3, "height_tiles": 2 })
    );
    assert_eq!(tile["recipe"], "item2");
}
