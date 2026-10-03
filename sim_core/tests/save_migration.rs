//! A version-7 save, converted by `scripts/migrate-save-v7-to-v8.py`, loads
//! into this build and runs.
//!
//! `fixtures/save_v7_two_lines.ron` is a real version-7 save, written by the
//! last version-7 build: the ketchup line and the old, cooked mustard line
//! (seed milled wet at the crusher, a mash and a mix in pipes, a mustard
//! vat), both mid-craft, with every gate open. `save_v7_two_lines.migrated.ron`
//! is what the script makes of it, and `scripts/test_migrate_save_v7_to_v8.py`
//! (run in CI) holds the script to producing exactly that file — so this test
//! is about the script's real output, not a hand-edited copy.
//!
//! What a converted save must be, in the order the script's doc states it:
//!
//! 1. every machine keeps the recipe it was running, and the ketchup line
//!    keeps making ketchup;
//! 2. the three retired mustard recipes are moved to their replacements, or
//!    unset where there is none, with no craft left half-done;
//! 3. every pipe is empty, so no fluid is relabelled under a new run name —
//!    and the one run the old mustard line no longer uses, the vinegar into
//!    its wet crusher, is flagged dead rather than quietly named;
//! 4. a save that had opened BBQ keeps it open under the new gate order.

mod common;

use sim_core::sim::world::TileSnapshot;
use sim_core::sim::{RecipeRegistry, SAVE_SCHEMA_VERSION, World};

const V7: &str = include_str!("fixtures/save_v7_two_lines.ron");
const MIGRATED: &str = include_str!("fixtures/save_v7_two_lines.migrated.ron");

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn recipe_at(world: &World, tile: (i32, i32)) -> Option<String> {
    match world.query_tile(tile) {
        Some(TileSnapshot::Machine { recipe, .. }) => recipe,
        other => panic!("{tile:?} is not a machine: {other:?}"),
    }
}

fn process_at(world: &World, tile: (i32, i32)) -> Option<(u64, u64)> {
    match world.query_tile(tile) {
        Some(TileSnapshot::Machine { process, .. }) => process,
        other => panic!("{tile:?} is not a machine: {other:?}"),
    }
}

#[test]
fn the_unconverted_file_is_refused_and_names_the_script() {
    let error = World::from_save_ron(V7, recipes())
        .err()
        .expect("a version 7 file is refused, never migrated in the game");
    assert!(
        error.to_string().contains("migrate-save-v7-to-v8.py"),
        "{error}"
    );
}

#[test]
fn a_converted_version_7_save_loads_keeps_its_machines_and_runs() {
    assert_eq!(SAVE_SCHEMA_VERSION, 8, "this fixture is the v7 -> v8 step");
    let mut world = World::from_save_ron(MIGRATED, recipes())
        .unwrap_or_else(|e| panic!("the converted save must load: {e}"));

    // 1. The ketchup line kept every recipe it was running (the reference
    //    chain's machines, west to east).
    for (tile, recipe) in [
        ((4, 0), "tomato_paste"),
        ((9, 0), "ketchup_mix"),
        ((12, 0), "ketchup"),
        ((16, 0), "bottled_ketchup"),
        ((21, 0), "boxed_ketchup"),
    ] {
        assert_eq!(recipe_at(&world, tile).as_deref(), Some(recipe), "{tile:?}");
    }

    // 2. The old mustard line's three retired recipes, moved or unset, and
    //    none of them holding a craft it could no longer finish.
    assert_eq!(recipe_at(&world, (4, 5)).as_deref(), Some("ground_mustard"));
    assert_eq!(recipe_at(&world, (7, 5)).as_deref(), Some("mustard"));
    assert_eq!(recipe_at(&world, (10, 5)), None, "no vat makes mustard now");
    for tile in [(4, 5), (7, 5), (10, 5)] {
        assert_eq!(
            process_at(&world, tile),
            None,
            "{tile:?}'s old craft is abandoned"
        );
    }

    // 3. Every pipe is empty, so nothing standing in one was relabelled.
    for run in world.fluid_snapshots() {
        assert_eq!(run.total_volume, 0.0, "{:?}", run.segments[0].tile);
    }

    // 4. BBQ was open in version 7 (it opened on ketchup); it still is.
    let gates = world.economy_snapshot().unlocks;
    assert!(
        gates.iter().all(|gate| gate.open),
        "every gate the save had opened stays open: {gates:?}"
    );

    // And it runs: the ketchup line ships, nothing stalls on a pipe it
    // cannot read, and the file saves as the current version.
    // Topped up through `common::buy`, which builds the racks it needs —
    // `stock_up` assumes a fresh world with none already standing.
    for (item, quantity) in [("tomato", 400), ("sugar", 100), ("vinegar", 300)] {
        common::buy(&mut world, item, quantity);
    }
    let made = |world: &World| {
        world
            .production_snapshot()
            .items
            .iter()
            .find(|item| item.id == "boxed_ketchup")
            .map_or(0, |item| item.produced_total)
    };
    let before = made(&world);
    for _ in 0..3_000 {
        common::supply(&mut world);
        world.tick();
    }
    assert!(
        made(&world) > before,
        "the ketchup line keeps making ketchup"
    );
    // Exactly one run is flagged, and it is the one the script's report
    // tells the player to rebuild: the vinegar run into the old WET mustard
    // crusher. A crusher drinks nothing now, so that tank pours into a run no
    // machine draws from, and the hazard clamps on it are the right signal.
    let faults: Vec<_> = world
        .fluid_snapshots()
        .into_iter()
        .filter(|run| run.naming.is_fault())
        .map(|run| (run.segments[0].tile, run.naming))
        .collect();
    assert_eq!(
        faults,
        vec![((1, 4), sim_core::sim::fluid::PipeNaming::Unnameable)],
        "only the old mustard crusher's vinegar run may be dead"
    );
    assert!(
        world
            .to_save_ron()
            .contains(&format!("schema_version: {SAVE_SCHEMA_VERSION}"))
    );
}
