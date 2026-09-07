//! Headless integration test for milestone 4 (ARCHITECTURE.md §9 item 4,
//! "First full chain"; §6's "no item duplication/loss" invariant tier,
//! extended from "one belt, one machine" to the whole multi-stage chain).
//! Runs the full production line — several belts and machines chained
//! end-to-end, entirely driven by `data/recipes.ron` — for thousands of
//! ticks with no Godot process involved at all (ARCHITECTURE.md §7), and
//! checks a strict conservation invariant across *every* stage, every tick.
//!
//! Mass conservation is checked in units of raw "tomato", by walking the
//! recipe graph: every recipe past the first converts N units of the
//! previous stage's single output item into 1 unit of its own output, so
//! any item's "tomato value" is fully determined by recursing back to the
//! chain's root ingredient (`tomato_value` below). This check makes no
//! assumption about how many stages `data/recipes.ron` currently defines —
//! it walks whatever chain `RecipeRegistry::ordered()` reports, which is
//! exactly what lets this same test file keep passing, unmodified, after
//! this milestone's data-file-only chain extension (see this crate's
//! CLAUDE.md/DEVLOG entry for that verification).
//!
//! That property held again through the "Machines improvement" ticket 2
//! content change, which re-split the chain's two-stage front end into three
//! real machines (crusher -> mixer -> cooking vat): the conservation logic
//! below needed no edit at all, because it reads the chain out of the data
//! file rather than naming any stage.

use sim_core::sim::{BeltSnapshot, MachineSnapshot, PlayerCommand, RecipeRegistry, TileSnapshot, World};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

const TOMATOES_PER_CRAFT: u32 = 4;

/// How many raw tomatoes one unit of `item` represents, by recursing back
/// through whichever recipe produces it. Relies on this milestone's
/// single-strand-chain assumption (world.rs's module doc): each non-root
/// recipe has exactly one input and one output unit, so an item's value is
/// just its recipe's input quantity times its input item's own value.
fn tomato_value(item: &str, recipes: &RecipeRegistry) -> u32 {
    if item == "tomato" {
        return 1;
    }
    let recipe = recipes
        .ordered()
        .iter()
        .find(|r| r.outputs.iter().any(|(id, _)| id == item))
        .unwrap_or_else(|| panic!("no recipe in data/recipes.ron produces item '{item}'"));
    let (input_item, input_qty) = recipe
        .inputs
        .first()
        .unwrap_or_else(|| panic!("recipe '{}' has no inputs", recipe.id));
    input_qty * tomato_value(input_item, recipes)
}

/// Every belt/machine snapshot across the whole chain, regardless of how
/// many chunks it spans — gathered via `World::all_chunks` rather than
/// assuming a fixed chunk count, so this keeps working if the chain grows
/// long enough to cross a chunk boundary.
fn full_chain_snapshot(world: &World) -> (Vec<BeltSnapshot>, Vec<MachineSnapshot>) {
    let mut belts = Vec::new();
    let mut machines = Vec::new();
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        belts.extend(snapshot.belts);
        machines.extend(snapshot.machines);
    }
    (belts, machines)
}

/// Total mass currently held anywhere in the sim (belts, machine input/
/// output buffers, and any mid-`TimedProcess` batch), in tomato-equivalent
/// units. Does NOT include anything already handed to the player via
/// `CollectOutput` — the caller adds that back in separately.
fn total_tomato_equivalent(world: &World, recipes: &RecipeRegistry) -> u32 {
    let (belts, machines) = full_chain_snapshot(world);
    let mut total = 0u32;

    for belt in &belts {
        for (item, _pos) in &belt.items {
            total += tomato_value(item, recipes);
        }
    }

    for machine in &machines {
        for (item, qty) in &machine.input {
            total += tomato_value(item, recipes) * qty;
        }
        for (item, qty) in &machine.output {
            total += tomato_value(item, recipes) * qty;
        }
        if machine.process.is_some() {
            // Mid-craft: input has already been consumed but output isn't
            // deposited yet. That in-flight mass equals the tomato-value of
            // the batch's eventual output item (recipe outputs are always
            // qty 1 in this chain, so this is exactly the consumed input
            // mass, just not currently sitting in any visible buffer).
            let recipe = recipes
                .get(&machine.recipe)
                .expect("machine must reference a real recipe");
            let (out_item, _out_qty) = &recipe.outputs[0];
            total += tomato_value(out_item, recipes);
        }
    }

    total
}

/// The conservation invariant this test exists to check: every tomato ever
/// successfully spawned is accounted for exactly once, as either raw mass
/// still somewhere in the chain (on a belt, in a buffer, or mid-craft) or
/// already collected as the finished product — never duplicated, never
/// silently lost, at any of the chain's several stages.
fn assert_conserved(
    world: &World,
    recipes: &RecipeRegistry,
    successful_spawns: u32,
    collected_final: u32,
    final_item_value: u32,
    context: &str,
) {
    let in_system = total_tomato_equivalent(world, recipes);
    let accounted = in_system + collected_final * final_item_value;
    assert_eq!(
        accounted, successful_spawns,
        "tomato mass not conserved at {context}: in_system={in_system}, \
         collected_final={collected_final} (x{final_item_value} each), spawned={successful_spawns}"
    );
}

#[test]
fn full_chain_conserves_mass_end_to_end_and_reaches_the_final_product() {
    let registry = recipes();
    let ordered = registry.ordered();
    // Five solid stages since ticket 2 split the front end into crusher ->
    // mixer -> cooking vat: mass has to survive two more hand-offs than it
    // did when this was a four-stage chain.
    assert_eq!(
        ordered.len(),
        5,
        "expected the shipped 5-stage chain (crusher, mixer, cooking vat, bottler, packer), \
         got {} recipe(s) — if a stage was added or removed deliberately, update this",
        ordered.len()
    );
    let final_recipe = ordered.last().unwrap().clone();
    let final_item = final_recipe.outputs[0].0.clone();
    let final_item_value = tomato_value(&final_item, &registry);

    let mut world = World::new(recipes());
    let mut successful_spawns = 0u32;
    let mut collected_final = 0u32;

    for tick_i in 0..20_000u32 {
        if successful_spawns < TOMATOES_PER_CRAFT {
            let before = total_tomato_equivalent(&world, &registry);
            world.issue_command(PlayerCommand::SpawnItem {
                item: "tomato".to_string(),
            });
            let after = total_tomato_equivalent(&world, &registry);
            if after > before {
                successful_spawns += 1;
            }
        }

        world.tick();

        assert_conserved(
            &world,
            &registry,
            successful_spawns,
            collected_final,
            final_item_value,
            &format!("tick {tick_i}"),
        );

        let (_, machines) = full_chain_snapshot(&world);
        let final_machine = machines
            .iter()
            .find(|m| m.recipe == final_recipe.id)
            .expect("final stage's machine must be present in some chunk snapshot");
        let final_output_qty = final_machine
            .output
            .iter()
            .find(|(id, _)| *id == final_item)
            .map(|(_, qty)| *qty)
            .unwrap_or(0);

        if final_output_qty > 0 {
            world.issue_command(PlayerCommand::CollectOutput);
            collected_final += final_output_qty;
            assert_conserved(
                &world,
                &registry,
                successful_spawns,
                collected_final,
                final_item_value,
                &format!("tick {tick_i} (post-collect)"),
            );
            break; // one full unit made it through the entire chain — proven.
        }
    }

    assert_eq!(
        successful_spawns, TOMATOES_PER_CRAFT,
        "all {TOMATOES_PER_CRAFT} tomatoes should have found room on the first belt"
    );
    assert_eq!(
        collected_final, 1,
        "exactly one unit of '{final_item}' should have made it through all {} stages \
         and been collected — the theoretical chain-throughput this milestone proves",
        ordered.len()
    );
    assert_eq!(
        total_tomato_equivalent(&world, &registry),
        0,
        "no mass should be left anywhere in the chain after the one unit is fully collected"
    );
}

#[test]
fn collecting_output_with_nothing_produced_yet_is_a_harmless_no_op() {
    let mut world = World::new(recipes());
    world.issue_command(PlayerCommand::CollectOutput);
    world.tick();
    let (_, machines) = full_chain_snapshot(&world);
    for machine in &machines {
        assert!(machine.output.is_empty(), "no machine should hold output yet");
    }
}

#[test]
fn query_tile_reflects_every_stage_belt_and_machine_position() {
    let registry = recipes();
    let ordered = registry.ordered().to_vec();
    let mut world = World::new(registry);
    world.issue_command(PlayerCommand::SpawnItem {
        item: "tomato".to_string(),
    });
    world.tick();

    let layout = world.chain_layout();
    assert_eq!(layout.len(), ordered.len(), "one stage per recipe in the data file");

    for (stage, recipe) in layout.iter().zip(ordered.iter()) {
        assert_eq!(stage.recipe, recipe.id, "stage order must follow recipes.ron order");
        assert_eq!(
            stage.machine_kind, recipe.machine,
            "each stage must run on the machine kind its recipe names"
        );

        for dx in 0..stage.belt_length_tiles {
            let tile = (stage.belt_start_tile.0 + dx, stage.belt_start_tile.1);
            match world.query_tile(tile) {
                Some(TileSnapshot::Belt { .. }) => {}
                other => panic!("expected belt at {tile:?}, got {other:?}"),
            }
        }

        // Every tile the machine covers, not just its origin — with the
        // cooking vat now genuinely 2x2 (data/machines.ron), this loop walks
        // four real tiles for that stage and one for each of the others.
        for dy in 0..stage.machine_footprint.height_tiles {
            for dx in 0..stage.machine_footprint.width_tiles {
                let tile = (stage.machine_tile.0 + dx, stage.machine_tile.1 + dy);
                match world.query_tile(tile) {
                    Some(TileSnapshot::Machine {
                        recipe: found,
                        origin_tile,
                        machine_kind,
                        ..
                    }) => {
                        assert_eq!(found, recipe.id, "at {tile:?}");
                        assert_eq!(machine_kind, recipe.machine, "at {tile:?}");
                        assert_eq!(
                            origin_tile, stage.machine_tile,
                            "every tile of a machine must report the same origin ({tile:?})"
                        );
                    }
                    other => panic!("expected machine at {tile:?}, got {other:?}"),
                }
            }
        }
    }

    assert!(world.query_tile((10_000, 10_000)).is_none());
}

#[test]
fn dirty_chunks_report_touched_chunks_and_clear_between_ticks() {
    let mut world = World::new(recipes());
    world.issue_command(PlayerCommand::SpawnItem {
        item: "tomato".to_string(),
    });
    world.tick();
    // The whole 5-stage chain (26 tiles wide, 2 rows tall where the cooking
    // vat sits) still fits inside chunk (0,0) (ARCHITECTURE.md §3.1's 32x32
    // chunks), so a spawn on the first belt is the only thing that changes
    // this tick and dirties exactly that chunk — not a claim that
    // dirty_chunks == all_chunks in general.
    assert_eq!(world.dirty_chunks(), &[(0, 0)]);
    assert_eq!(world.all_chunks(), vec![(0, 0)]);

    // A tick where absolutely nothing moves/changes reports no dirty
    // chunks — every machine is idle without input, every belt is empty.
    let mut idle_world = World::new(recipes());
    idle_world.tick();
    assert!(idle_world.dirty_chunks().is_empty());
}
