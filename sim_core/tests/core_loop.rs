//! Headless integration test for milestone 3 (ARCHITECTURE.md §9 item 3,
//! §6's "no item duplication/loss" invariant tier). Runs the whole loop —
//! belt, machine, timed process, recipe — for thousands of ticks with no
//! Godot process involved at all (ARCHITECTURE.md §7), and checks a strict
//! conservation invariant every single tick, not just at the end.

use sim_core::sim::{PlayerCommand, RecipeRegistry, World};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

const TOMATOES_PER_CRAFT: u32 = 4;

struct Counts {
    tomatoes_in_system: u32,
    sauce_in_output: u32,
    in_flight_batches: u32,
}

/// Reads every place a conserved unit of mass could currently be sitting:
/// raw tomatoes on the belt or in the machine's input buffer, a batch
/// mid-`TimedProcess` (consumed from input, not yet deposited to output),
/// and finished sauce sitting in the output buffer.
fn read_counts(world: &World) -> Counts {
    let snapshot = world.chunk_snapshot((0, 0));

    let belt_tomatoes = snapshot
        .belt
        .as_ref()
        .map(|b| b.items.iter().filter(|(id, _)| id == "tomato").count() as u32)
        .unwrap_or(0);

    let machine = snapshot
        .machine
        .as_ref()
        .expect("machine must be in chunk (0,0)");
    let input_tomatoes = machine
        .input
        .iter()
        .find(|(id, _)| id == "tomato")
        .map(|(_, qty)| *qty)
        .unwrap_or(0);
    let sauce_in_output = machine
        .output
        .iter()
        .find(|(id, _)| id == "tomato_sauce")
        .map(|(_, qty)| *qty)
        .unwrap_or(0);
    let in_flight_batches = if machine.process.is_some() { 1 } else { 0 };

    Counts {
        tomatoes_in_system: belt_tomatoes + input_tomatoes,
        sauce_in_output,
        in_flight_batches,
    }
}

/// The conservation invariant this test exists to check: every tomato that
/// was ever successfully spawned is accounted for exactly once, as either
/// a raw tomato still in transit, part of one in-flight craft, or turned
/// into sauce (still held or already collected) — never duplicated, never
/// silently lost.
fn assert_conserved(world: &World, successful_spawns: u32, collected_sauce: u32, context: &str) {
    let counts = read_counts(world);
    let accounted_tomatoes = counts.tomatoes_in_system
        + TOMATOES_PER_CRAFT * counts.in_flight_batches
        + TOMATOES_PER_CRAFT * (counts.sauce_in_output + collected_sauce);
    assert_eq!(
        accounted_tomatoes,
        successful_spawns,
        "tomato mass not conserved at {context}: in_system={}, in_flight={}, \
         sauce_output={}, sauce_collected={}, spawned={}",
        counts.tomatoes_in_system,
        counts.in_flight_batches,
        counts.sauce_in_output,
        collected_sauce,
        successful_spawns
    );
}

#[test]
fn belt_and_machine_conserve_items_across_a_full_recipe_cycle() {
    let mut world = World::new(recipes(), "tomato_sauce".to_string());
    let mut successful_spawns = 0u32;
    let mut collected_sauce = 0u32;

    for tick_i in 0..5_000u32 {
        if successful_spawns < TOMATOES_PER_CRAFT {
            let before = read_counts(&world).tomatoes_in_system;
            world.issue_command(PlayerCommand::SpawnItem {
                item: "tomato".to_string(),
            });
            let after = read_counts(&world).tomatoes_in_system;
            if after > before {
                successful_spawns += 1;
            }
        }

        world.tick();

        assert_conserved(
            &world,
            successful_spawns,
            collected_sauce,
            &format!("tick {tick_i}"),
        );

        // Collect the moment sauce is available, then keep ticking so the
        // conservation check also covers the post-collection tail.
        let counts = read_counts(&world);
        if counts.sauce_in_output > 0 {
            world.issue_command(PlayerCommand::CollectOutput);
            collected_sauce += counts.sauce_in_output;
            assert_conserved(
                &world,
                successful_spawns,
                collected_sauce,
                &format!("tick {tick_i} (post-collect)"),
            );
        }
    }

    assert_eq!(
        successful_spawns, TOMATOES_PER_CRAFT,
        "all 4 tomatoes should have found room on the belt"
    );
    assert_eq!(
        collected_sauce, 1,
        "exactly one batch's worth of sauce should have been produced and collected"
    );

    let final_counts = read_counts(&world);
    assert_eq!(
        final_counts.tomatoes_in_system, 0,
        "no tomato should be left in the system"
    );
    assert_eq!(
        final_counts.sauce_in_output, 0,
        "collected output must actually be removed from the buffer"
    );
    assert_eq!(
        final_counts.in_flight_batches, 0,
        "no craft should still be running"
    );
}

#[test]
fn collecting_output_with_nothing_produced_yet_is_a_harmless_no_op() {
    let mut world = World::new(recipes(), "tomato_sauce".to_string());
    world.issue_command(PlayerCommand::CollectOutput);
    world.tick();
    assert_eq!(read_counts(&world).sauce_in_output, 0);
}

#[test]
fn query_tile_reflects_belt_and_machine_positions() {
    use sim_core::sim::TileSnapshot;

    let mut world = World::new(recipes(), "tomato_sauce".to_string());
    world.issue_command(PlayerCommand::SpawnItem {
        item: "tomato".to_string(),
    });
    world.tick();

    match world.query_tile((0, 0)) {
        Some(TileSnapshot::Belt { .. }) => {}
        other => panic!("expected belt at (0,0), got {other:?}"),
    }
    match world.query_tile((4, 0)) {
        Some(TileSnapshot::Machine { recipe, .. }) => assert_eq!(recipe, "tomato_sauce"),
        other => panic!("expected machine at (4,0), got {other:?}"),
    }
    assert!(world.query_tile((100, 100)).is_none());
}

#[test]
fn dirty_chunks_report_the_touched_chunk_and_clear_between_ticks() {
    let mut world = World::new(recipes(), "tomato_sauce".to_string());
    world.issue_command(PlayerCommand::SpawnItem {
        item: "tomato".to_string(),
    });
    world.tick();
    assert_eq!(world.dirty_chunks(), &[(0, 0)]);

    // A tick where absolutely nothing moves/changes reports no dirty
    // chunks — the machine is idle without input, the belt is empty.
    let mut idle_world = World::new(recipes(), "tomato_sauce".to_string());
    idle_world.tick();
    assert!(idle_world.dirty_chunks().is_empty());
}
