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
//!
//! **Ticket 3 changed how mass enters the chain**, and that *did* need edits
//! here — deliberately, since the old behavior being tested is the behavior
//! being removed. Raw tomatoes are no longer teleported onto stage 0's belt
//! by a command; they are loaded into a hopper (`PlayerCommand::FillHopper`)
//! which meters them onto the belt one unit per tick. So the conservation
//! accounting now counts hopper contents as mass in the system (they are:
//! the player put them there and nothing has consumed them yet), and the
//! full-chain test fills once up front and lets the unload happen over the
//! following ticks instead of spawning per tick.
//!
//! **Milestone 7 (ARCHITECTURE.md §9 item 7, "Timed processes") inserted a
//! 2400-tick aging stage mid-chain**, and the only edit it needed here was
//! the stage-count assertion below — the conservation accounting itself
//! reads the chain out of the data file, and it already counted a machine's
//! in-flight `TimedProcess` batch as mass in the system. That was written
//! for craft cycles lasting tens of ticks; it holds unchanged for one
//! lasting thousands, which is a small piece of the milestone's own proof.
//!
//! **The delivery-dock ticket changed how mass LEAVES the chain**, and that
//! needed edits here for the same reason ticket 3's did at the other end:
//! the old behavior being tested is the behavior being removed. Finished
//! boxes are no longer lifted out of the last machine's output buffer by
//! `PlayerCommand::CollectOutput` (which no longer exists); they travel one
//! more real belt into a `Dock` building, which ships them into the
//! economy's shared delivery pool automatically, every tick. So the
//! conservation accounting now counts the dock's own buffer as mass in the
//! system (it is — momentarily, inside a tick), and "collected" is read from
//! the dock's cumulative `shipped_total` instead of being tallied by the
//! caller issuing a command. **Nothing in the test issues any command to
//! empty the chain any more**, which is precisely the property this ticket
//! is for.
//!
//! **The "Visual overhaul round 2" epic's ticket 1 did the same to fluid**,
//! and needed the same kind of edit: the pipe run's vinegar used to arrive
//! from an off-map tap that refilled itself every tick, and now comes from a
//! tank the player loads (`PlayerCommand::FillFluidTank`). So the full-chain
//! test loads the tank once up front too. Vinegar is not counted in the
//! tomato-equivalent accounting below — it enters the chain as a fluid, not
//! as belt-borne mass, and leaves it dissolved in the product.

use sim_core::sim::{
    BeltSnapshot, DockSnapshot, FLUID_TANK_CAPACITY, HopperSnapshot, MachineSnapshot,
    PlayerCommand, RecipeRegistry, TileSnapshot, World, reference_chain,
};

/// The reference chain's one hopper and one dock. Several of each are a real
/// supported thing since the "Real Placement" epic (`placement.rs` covers
/// that); this file's accounting is about the shipped single-line factory.
fn the_hopper(world: &World) -> HopperSnapshot {
    let mut hoppers = world.hopper_snapshots();
    assert_eq!(hoppers.len(), 1, "the reference chain places one hopper");
    hoppers.remove(0)
}

fn the_dock(world: &World) -> DockSnapshot {
    let mut docks = world.dock_snapshots();
    assert_eq!(docks.len(), 1, "the reference chain places one dock");
    docks.remove(0)
}

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
/// long enough to cross a chunk boundary. It genuinely spans three chunks
/// now: the hopper sits west of x = 0, in chunk (-1, 0), and since the
/// delivery-dock ticket the dock sits at x = 35, in chunk (1, 0).
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

/// Total mass currently held anywhere in the sim (belts — including the
/// dock's own inbound belt — machine input/output buffers, the dock's
/// transfer buffer, and any mid-`TimedProcess` batch), in tomato-equivalent
/// units. Does NOT include anything the dock has already shipped into the
/// economy's delivery pool — the caller adds that back in separately, from
/// the dock's `shipped_total`.
fn total_tomato_equivalent(world: &World, recipes: &RecipeRegistry) -> u32 {
    let (belts, machines) = full_chain_snapshot(world);
    let mut total = 0u32;

    // Anything standing on the dock but not yet shipped. Between ticks this
    // is always empty (the dock is a pass-through — `sim/dock.rs`'s module
    // doc), but counting it is what makes this accounting independent of
    // that decision: if the drain were ever gated, mass parked on the dock
    // would still be accounted for rather than reading as a leak.
    for (item, qty) in &the_dock(world).contents {
        total += tomato_value(item, recipes) * qty;
    }

    // Raw ingredient waiting in the hopper is mass in the system exactly the
    // way mass in a machine's input buffer is (ticket 3) — it has been
    // loaded but not yet consumed. Counting it is what makes conservation
    // hold from the instant of the fill command rather than only once the
    // hopper has finished unloading.
    let hopper = the_hopper(world);
    total += tomato_value(&hopper.item, recipes) * hopper.quantity;

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
/// successfully loaded is accounted for exactly once, as either raw mass
/// still somewhere in the chain (in the hopper, on a belt, in a buffer, on
/// the dock, or mid-craft) or already shipped off the dock as the finished
/// product — never duplicated, never silently lost, at any of the chain's
/// several stages or across the new belt-and-dock exit.
fn assert_conserved(
    world: &World,
    recipes: &RecipeRegistry,
    loaded_tomatoes: u32,
    collected_final: u32,
    final_item_value: u32,
    context: &str,
) {
    let in_system = total_tomato_equivalent(world, recipes);
    let accounted = in_system + collected_final * final_item_value;
    assert_eq!(
        accounted, loaded_tomatoes,
        "tomato mass not conserved at {context}: in_system={in_system}, \
         collected_final={collected_final} (x{final_item_value} each), loaded={loaded_tomatoes}"
    );
}

#[test]
fn full_chain_conserves_mass_end_to_end_and_reaches_the_final_product() {
    let registry = recipes();
    let ordered = registry.ordered();
    // Six solid stages: ticket 2 split the front end into crusher -> mixer
    // -> cooking vat, and milestone 7 inserted the aging rack between the
    // vat and the bottler. Mass has to survive three more hand-offs than it
    // did when this was a four-stage chain — including one across a stage
    // that holds a batch for 2400 ticks, which is exactly the case a
    // conservation check should be nervous about (mass sitting in no visible
    // buffer for a long time is the shape a leak would hide in).
    assert_eq!(
        ordered.len(),
        6,
        "expected the shipped 6-stage chain (crusher, mixer, cooking vat, aging rack, \
         bottler, packer), got {} recipe(s) — if a stage was added or removed deliberately, \
         update this",
        ordered.len()
    );
    let final_recipe = ordered.last().unwrap().clone();
    let final_item = final_recipe.outputs[0].0.clone();
    let final_item_value = tomato_value(&final_item, &registry);

    let mut world = reference_chain(recipes());

    // Ticket 3: one fill command loads the hopper, and the hopper meters
    // those tomatoes onto the first belt over the following ticks — nothing
    // is teleported onto a belt any more. The load happens once, before the
    // loop, precisely to prove the *unload* is what feeds the chain.
    // The pipe run's tank likewise has to be loaded before the fluid-gated
    // mixing stage can run at all (the "Visual overhaul round 2" epic's
    // ticket 1 removed the always-refilling off-map source, so vinegar is a
    // consumable the player supplies now). It is not a tomato and so
    // contributes nothing to the mass accounting below.
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });

    let before_fill = total_tomato_equivalent(&world, &registry);
    world.issue_command(PlayerCommand::FillHopper {
        amount: TOMATOES_PER_CRAFT,
        tile: None,
    });
    let loaded_tomatoes = total_tomato_equivalent(&world, &registry) - before_fill;
    assert_eq!(
        loaded_tomatoes, TOMATOES_PER_CRAFT,
        "an empty hopper must accept a full crafts' worth of raw ingredient"
    );
    assert_eq!(
        the_hopper(&world).quantity,
        TOMATOES_PER_CRAFT,
        "the loaded tomatoes must be sitting in the hopper, not already on a belt"
    );

    // No command is issued inside this loop at all — that is the point of
    // the delivery dock. The chain feeds itself from the hopper, runs, and
    // the dock ships the finished box into the economy's delivery pool by
    // itself; conservation is checked on every single tick along the way.
    let mut shipped = 0u32;
    for tick_i in 0..20_000u32 {
        world.tick();

        shipped = the_dock(&world).shipped_total;
        assert_conserved(
            &world,
            &registry,
            loaded_tomatoes,
            shipped,
            final_item_value,
            &format!("tick {tick_i}"),
        );

        if shipped > 0 {
            break; // one full unit made it through the entire chain — proven.
        }
    }

    assert_eq!(
        the_hopper(&world).quantity,
        0,
        "the hopper should have unloaded all {TOMATOES_PER_CRAFT} tomatoes onto the belt"
    );
    assert_eq!(
        shipped, 1,
        "exactly one unit of '{final_item}' should have made it through all {} stages, \
         crossed the dock's belt and been shipped — with no player command whatsoever, \
         which is what the dock replaced `CollectOutput` to achieve",
        ordered.len()
    );
    // ...and it landed in the ONE shared pool every contract settles
    // against, which is the actual end of GAMEPLAY.md §1's loop.
    assert_eq!(
        world.economy_snapshot().delivery_stock,
        vec![(final_item.clone(), 1)],
        "the shipped box must be in the shared delivery pool, not held on the dock"
    );
    assert!(
        the_dock(&world).contents.is_empty(),
        "the dock is a pass-through: it must be empty again the tick it shipped"
    );
    assert_eq!(
        total_tomato_equivalent(&world, &registry),
        0,
        "no mass should be left anywhere in the chain after the one unit is fully shipped"
    );
}

/// The exit end's equivalent of "an unfilled hopper unloads nothing": an
/// idle chain must ship nothing, rather than the dock's per-tick drain
/// manufacturing a delivery out of an empty buffer.
#[test]
fn an_unfed_chain_ships_nothing_and_the_dock_stays_empty() {
    let mut world = reference_chain(recipes());
    for _ in 0..500 {
        world.tick();
    }

    let dock = the_dock(&world);
    assert_eq!(dock.shipped_total, 0, "nothing was ever produced to ship");
    assert!(dock.contents.is_empty());
    assert!(
        world.economy_snapshot().delivery_stock.is_empty(),
        "an idle factory must not put anything in the delivery pool"
    );

    let (_, machines) = full_chain_snapshot(&world);
    for machine in &machines {
        assert!(machine.output.is_empty(), "no machine should hold output yet");
    }
}

/// `collect_output` was a real command until the delivery-dock ticket. A
/// stale GDScript caller (`main.gd`'s **C** key, until ticket 2 removes it)
/// must get the same silent no-op any malformed command gets — never a
/// panic, and never a second way to move product.
#[test]
fn the_removed_collect_output_command_no_longer_parses() {
    assert!(
        sim_core::sim::command_from_json(r#"{"type": "collect_output"}"#).is_none(),
        "collect_output must no longer deserialize — the dock replaced it"
    );
    // ...and the commands that remain still do, so this is a targeted
    // removal rather than a broken parser.
    assert!(sim_core::sim::command_from_json(r#"{"type": "fill_hopper"}"#).is_some());
    assert!(sim_core::sim::command_from_json(r#"{"type": "fill_fluid_tank"}"#).is_some());
    assert!(
        sim_core::sim::command_from_json(r#"{"type": "purchase", "kind": "belt"}"#).is_some()
    );
}

#[test]
fn query_tile_reflects_every_stage_belt_and_machine_position() {
    let registry = recipes();
    let ordered = registry.ordered().to_vec();
    let mut world = reference_chain(registry);
    world.issue_command(PlayerCommand::FillHopper { amount: 4, tile: None });
    world.tick();

    // `world_layout` replaced `chain_layout`: it reports every placement, so
    // the machines are found by filtering rather than by stage index. They
    // still come out in recipes.ron order because that is the order the
    // reference chain lays them out in, west to east.
    let layout = world.world_layout();
    let machines: Vec<_> = layout.iter().filter(|p| p.recipe.is_some()).collect();
    assert_eq!(
        machines.len(),
        ordered.len(),
        "one machine per recipe in the data file"
    );

    for (placement, recipe) in machines.iter().zip(ordered.iter()) {
        assert_eq!(
            placement.recipe.as_deref(),
            Some(recipe.id.as_str()),
            "machine order must follow recipes.ron order"
        );
        assert_eq!(
            placement.kind, recipe.machine,
            "each machine must run the recipe its kind names"
        );

        // Every tile the machine covers, not just its origin — with the
        // cooking vat genuinely 2x2 (data/machines.ron), this loop walks
        // four real tiles for that stage and one for each of the others.
        for &tile in &placement.occupies {
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
                        origin_tile, placement.tile,
                        "every tile of a machine must report the same origin ({tile:?})"
                    );
                }
                other => panic!("expected machine at {tile:?}, got {other:?}"),
            }
        }

        // The four belt tiles feeding it — one placement each now, where the
        // pre-placement chain built a single four-tile segment.
        for dx in 1..=4 {
            let tile = (placement.tile.0 - dx, placement.tile.1);
            match world.query_tile(tile) {
                Some(TileSnapshot::Belt { .. }) => {}
                other => panic!("expected belt at {tile:?}, got {other:?}"),
            }
        }
    }

    assert!(world.query_tile((10_000, 10_000)).is_none());
}

#[test]
fn dirty_chunks_report_touched_chunks_and_clear_between_ticks() {
    let mut world = reference_chain(recipes());
    world.issue_command(PlayerCommand::FillHopper { amount: 4, tile: None });
    world.tick();
    // The six recipe-driven stages run x = 0..30 and still fit inside chunk
    // (0,0) (ARCHITECTURE.md §3.1's 32x32 chunks); the hopper sits west of
    // them at x = -1, i.e. in chunk (-1, 0). On this tick the hopper unloads
    // its first tomato, which changes both: the belt it landed on and the
    // hopper's own fill level. Nothing at the dock end has moved yet, so
    // its chunk is NOT dirty — dirty tracking is per-tick, unlike
    // `all_chunks`.
    // The order is the order the tick's passes touch them, which is
    // deterministic (every pass walks a tile-ordered list) — the hopper's
    // own chunk is marked when it unloads, then the belt's when the tomato
    // lands.
    assert_eq!(world.dirty_chunks(), &[(-1, 0), (0, 0)]);
    // `all_chunks` is the whole world, and since the delivery-dock ticket
    // that is three chunks: the dock stands at x = 35, past the x = 32
    // boundary, and its inbound belt straddles it.
    assert_eq!(
        world.all_chunks(),
        vec![(-1, 0), (0, 0), (1, 0)],
        "the hopper's chunk and the dock's chunk must both be reported, or a renderer \
         never draws either end of the line"
    );

    // A tick where absolutely nothing moves/changes reports no dirty
    // chunks — every machine is idle without input, every belt is empty,
    // and an unfilled hopper has nothing to unload.
    let mut idle_world = reference_chain(recipes());
    // The first tick clears the dirt left by *building* the chain — placing
    // anything dirties its chunks, and the fixture places forty-odd things.
    idle_world.tick();
    idle_world.tick();
    assert!(idle_world.dirty_chunks().is_empty());
}
