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
//! **The delivery-dock ticket changed how mass LEAVES the chain**, and that
//! needed edits here for the same reason ticket 3's did at the other end:
//! the old behavior being tested is the behavior being removed. Finished
//! boxes are no longer lifted out of the last machine's output buffer by
//! `PlayerCommand::CollectOutput` (which no longer exists); they travel one
//! more real belt into a `Dock` building, which holds them for a contract
//! to take. So "left the chain" is read from the dock's cumulative
//! `shipped_total` instead of being tallied by the caller issuing a
//! command, and whatever still stands on the dock is inside that total. **Nothing in the test issues any command to
//! empty the chain any more**, which is precisely the property this ticket
//! is for.
//!
//! **The "Visual overhaul round 2" epic's ticket 1 did the same to fluid**,
//! and needed the same kind of edit: the pipe run's vinegar used to arrive
//! from an off-map tap that refilled itself every tick, and now comes from a
//! tank the player loads (`PlayerCommand::FillFluidTank`). So the full-chain
//! test loads the tank once up front too.
//!
//! # What the chain-rework epic changed, and why it is a bigger edit
//!
//! Three things at once, and each of them moves what "mass" even means here.
//!
//! 1. **The middle of the chain is piped, not belted.** The mixer's output
//!    and the cooking vat's output are *fluids* now, so between the mixer and
//!    the bottler the product is not on any belt and not in any machine
//!    buffer — it is volume standing in a pipe network. A conservation check
//!    that only walked belts and buffers would read that as a leak, so the
//!    accounting below counts every network's volume and every tank's.
//! 2. **There are two raw ingredients** (tomato and sugar), so a single
//!    "tomato-equivalent" figure can no longer describe the whole system.
//!    The check is run **once per raw ingredient**: how many tomatoes are
//!    accounted for, and separately how many units of sugar. `raw_content`
//!    below is the generalized version of the old `tomato_value` — it
//!    recurses through solid *and* fluid production alike, and answers zero
//!    for an item that embodies none of the raw it is asked about.
//! 3. **Ingredients cost money** (GAMEPLAY.md §5). Nothing can be loaded
//!    that was not bought, so the fixture is
//!    `reference_chain_with_budget` and the test buys exactly the stock it
//!    then accounts for. Vinegar is bought and loaded like the rest but
//!    contributes nothing to either raw's accounting, because it embodies
//!    neither.
//!
//! Everything is measured in fixed-point (`Fx`), because a pipe's volume is
//! continuous. That is not a weakening of the invariant: every fluid
//! transfer in `fluid.rs` is a paired subtract/add clamped to what the donor
//! has, so conservation is an exact equality there too, not an epsilon
//! comparison.

mod common;

use sim_core::sim::{
    BeltSnapshot, DockSnapshot, Fx, MachineSnapshot, PlayerCommand, RecipeRegistry, TileSnapshot,
    World, linear_prefix, reference_chain_with_budget,
};

/// Enough money to buy the stock this file loads, on top of the shop bill
/// the fixture already spends.
const BUDGET: u32 = 400 + common::WAGE_RESERVE;

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// The reference chain's one dock. Several are a real supported thing since
/// the "Real Placement" epic (`placement.rs` covers that); this file's
/// accounting is about the shipped single-line factory.
fn the_dock(world: &World) -> DockSnapshot {
    let mut docks = world.dock_snapshots();
    assert_eq!(docks.len(), 1, "the reference chain places one dock");
    docks.remove(0)
}

/// Buys `quantity` of `item`, building the warehouse room for it first and
/// asserting the purchase landed — an unaffordable or unhousable buy is a
/// silent no-op, which would otherwise surface much later as an unexplained
/// empty bin. `common::buy` is the one implementation of that.
fn buy(world: &mut World, item: &str, quantity: u32) {
    common::buy(world, item, quantity);
}

/// How many units of raw ingredient `raw` one unit of `id` embodies, by
/// recursing back through whichever recipe produces it — following **fluid**
/// production as readily as solid, since the middle of the chain is piped.
///
/// Answers `1` for `raw` itself and `0` for anything nothing produces and
/// that is not `raw` (i.e. a *different* raw ingredient: sugar carries no
/// tomatoes). Relies on the chain's single-strand shape, which
/// `validate_chain` enforces: every recipe past the first produces one unit
/// from a fixed bill of inputs, so an id's raw content is just the sum of
/// its inputs' contents times their quantities.
fn raw_content(id: &str, raw: &str, recipes: &RecipeRegistry) -> u32 {
    if id == raw {
        return 1;
    }
    let Some(recipe) = recipes.ordered().iter().find(|r| {
        r.outputs.iter().any(|(out, _)| out == id)
            || r.fluid_outputs.iter().any(|(out, _)| out == id)
    }) else {
        // Produced by nothing: another raw ingredient, which embodies none
        // of the one being counted.
        return 0;
    };
    recipe
        .inputs
        .iter()
        .chain(recipe.fluid_inputs.iter())
        .map(|(input, qty)| qty * raw_content(input, raw, recipes))
        .sum()
}

/// Every belt/machine snapshot across the whole chain, regardless of how
/// many chunks it spans — gathered via `World::all_chunks` rather than
/// assuming a fixed chunk count. It genuinely spans two chunks: the line
/// sits in chunk (0, 0) and the first feed lane's hopper stands west of it
/// at x = -1, in chunk (-1, 0).
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

/// Total `raw`-equivalent mass currently held anywhere in the sim: hoppers,
/// belts, machine buffers, any mid-`TimedProcess` batch, **every fluid
/// network and tank**. Does NOT include anything that has reached the dock —
/// the caller adds that back in separately, from `shipped_total`, which
/// counts every box the dock ever took off its belt: the ones still standing
/// on it and the ones a contract has since collected alike.
fn total_raw_equivalent(world: &World, raw: &str, recipes: &RecipeRegistry) -> Fx {
    let (belts, machines) = full_chain_snapshot(world);
    let mut total = Fx::ZERO;
    let value = |id: &str| Fx::from_num(raw_content(id, raw, recipes));

    // Raw ingredient waiting in a hopper is mass in the system exactly the
    // way mass in a machine's input buffer is — it has been bought and
    // loaded but not yet consumed. There is one hopper per raw ingredient
    // now, and only the one holding `raw` contributes.
    for hopper in world.hopper_snapshots() {
        total += value(&hopper.item) * Fx::from_num(hopper.quantity);
    }

    for belt in &belts {
        for (item, _pos) in &belt.items {
            total += value(item);
        }
    }

    for machine in &machines {
        for (item, qty) in &machine.input {
            total += value(item) * Fx::from_num(*qty);
        }
        for (item, qty) in &machine.output {
            total += value(item) * Fx::from_num(*qty);
        }
        if machine.process.is_some() {
            // Mid-craft: input has already been consumed but output isn't
            // deposited yet. That in-flight mass is the raw content of what
            // the batch will become — solid, fluid, or both.
            let recipe = recipes
                .get(&machine.recipe)
                .expect("machine must reference a real recipe");
            for (item, qty) in recipe.outputs.iter().chain(recipe.fluid_outputs.iter()) {
                total += value(item) * Fx::from_num(*qty);
            }
        }
    }

    // The piped middle of the chain: liquid mix and liquid ketchup standing
    // in a network are mass, and the only place it lives between the mixer
    // and the bottler.
    for network in world.fluid_snapshots() {
        if let Some(fluid) = &network.fluid {
            total += value(fluid) * Fx::from_num(network.total_volume);
        }
    }
    // ...and the tanks, which hold raw fluid the player bought. (Vinegar
    // embodies no tomato and no sugar, so this contributes nothing today —
    // it is here so that a future raw fluid carrying real mass cannot read
    // as a leak.)
    for tank in world.fluid_tank_snapshots() {
        if let Some(fluid) = &tank.fluid {
            total += value(fluid) * Fx::from_num(tank.quantity);
        }
    }

    total
}

/// The conservation invariant this test exists to check, run once per raw
/// ingredient: every unit ever successfully loaded is accounted for exactly
/// once, as either mass still somewhere in the chain (in a hopper, on a
/// belt, in a buffer, in a pipe, on the dock, or mid-craft) or already
/// shipped off the dock inside the finished product — never duplicated,
/// never silently lost.
fn assert_conserved(
    world: &World,
    recipes: &RecipeRegistry,
    raw: &str,
    loaded: u32,
    collected_final: u32,
    final_item: &str,
    context: &str,
) {
    let in_system = total_raw_equivalent(world, raw, recipes);
    let per_box = Fx::from_num(raw_content(final_item, raw, recipes));
    let accounted = in_system + per_box * Fx::from_num(collected_final);
    assert_eq!(
        accounted,
        Fx::from_num(loaded),
        "{raw} mass not conserved at {context}: in_system={in_system}, \
         collected_final={collected_final} (x{per_box} each), loaded={loaded}"
    );
}

/// How much stock the full-chain run below buys and loads. Sized against
/// what the piped middle actually needs, and *small* deliberately:
///
/// - **Sugar is the tight one.** A hopper meters its whole load onto the
///   belt as fast as the belt will take it, and the mixer's input buffer
///   holds only so much of any one item — so a sugar hopper loaded to the
///   brim would fill the mixer's feed belt with sugar and starve it of the
///   tomato paste it also needs. Buying only what the run consumes is both
///   the fix and the natural incentive the ingredient economy creates: over-
///   stocking a secondary ingredient wastes money *and* jams the line. (A
///   splitter or filtered insertion is what would make over-stocking safe;
///   that is deliberately a later epic's job.)
/// - **Six mixer batches** is what it takes for one box to come out the far
///   end, because each machine-to-machine pipe run has to be primed before
///   its far end can draw a whole unit at the single tile it taps (see
///   `sim::reference`'s note on run length). Six batches needs six sugar and
///   six tomato paste, i.e. twenty-four tomatoes.
const SUGAR_UNITS: u32 = 6;
const TOMATOES_PER_PASTE: u32 = 4;
const TOMATO_UNITS: u32 = SUGAR_UNITS * TOMATOES_PER_PASTE;
/// Vinegar is bought generously: the tank presents constant full head into a
/// four-tile run that keeps a standing charge, so the mixer only ever sees
/// vinegar at its own tap once the run is wetted (`FLUID_TANK_CAPACITY`'s
/// own doc). It carries no tomato and no sugar, so its quantity does not
/// enter the accounting at all.
const VINEGAR_UNITS: u32 = 60;

#[test]
fn full_chain_conserves_mass_end_to_end_and_reaches_the_final_product() {
    let registry = recipes();
    // The reference LINE, not every recipe in the file: since the BBQ epic
    // `recipes.ron` is a graph, and this fixture lays out its linear prefix
    // (the ketchup line). The BBQ branch is `tests/bbq_branch.rs`'s.
    let ordered = linear_prefix(&registry);
    // Five stages: crush, mix, cook, bottle, pack. The chain-rework epic
    // removed the aging rack and turned the two middle links into pipe runs,
    // so mass now has to survive two hand-offs that are not belt hand-offs at
    // all — which is exactly the case a conservation check should be nervous
    // about, since fluid in a pipe sits in no buffer any snapshot of a
    // machine would show.
    assert_eq!(
        ordered.len(),
        5,
        "expected the shipped 5-stage ketchup line (crusher, mixer, cooking vat, bottler, \
         packer), got {} stage(s) — if a stage was added or removed deliberately, update this",
        ordered.len()
    );
    let final_recipe = ordered.last().unwrap().clone();
    let final_item = final_recipe.outputs[0].0.clone();

    let mut world = reference_chain_with_budget(recipes(), BUDGET);

    // Nothing is free any more: every unit accounted for below was paid for
    // out of the one money counter first (GAMEPLAY.md §5).
    let money_before = world.money();
    buy(&mut world, "tomato", TOMATO_UNITS);
    buy(&mut world, "sugar", SUGAR_UNITS);
    buy(&mut world, "vinegar", VINEGAR_UNITS);
    assert!(
        world.money() < money_before,
        "buying ingredients must cost money — that is the whole point of the mechanic"
    );

    // One fill command per buffer, before the loop, precisely to prove the
    // *unload* is what feeds the chain: nothing is teleported onto a belt
    // and nothing is conjured into a pipe.
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: VINEGAR_UNITS,
        tile: None,
    });
    let before_fill: Vec<Fx> = ["tomato", "sugar"]
        .iter()
        .map(|raw| total_raw_equivalent(&world, raw, &registry))
        .collect();
    world.issue_command(PlayerCommand::FillHopper {
        amount: TOMATO_UNITS.max(SUGAR_UNITS),
        tile: None,
    });
    for (i, raw) in ["tomato", "sugar"].iter().enumerate() {
        let loaded = total_raw_equivalent(&world, raw, &registry) - before_fill[i];
        let expected = if *raw == "tomato" {
            TOMATO_UNITS
        } else {
            SUGAR_UNITS
        };
        assert_eq!(
            loaded,
            Fx::from_num(expected),
            "the {raw} hopper must accept exactly the stock that was bought for it"
        );
    }
    assert_eq!(
        world.owned_ingredient("tomato"),
        0,
        "the whole tomato load went into the hopper, not half of it"
    );

    // No command is issued inside this loop at all — that is the point of
    // the delivery dock. The chain feeds itself from the hoppers, runs, and
    // the dock takes the finished box off its belt by itself; conservation is checked, for both raw ingredients, on every
    // single tick along the way.
    let mut shipped = 0u32;
    for tick_i in 0..40_000u32 {
        world.tick();

        shipped = the_dock(&world).shipped_total;
        for raw in ["tomato", "sugar"] {
            let loaded = if raw == "tomato" {
                TOMATO_UNITS
            } else {
                SUGAR_UNITS
            };
            assert_conserved(
                &world,
                &registry,
                raw,
                loaded,
                shipped,
                &final_item,
                &format!("tick {tick_i}"),
            );
        }

        if shipped > 0 {
            break; // one full unit made it through the entire chain — proven.
        }
    }

    assert_eq!(
        shipped,
        1,
        "exactly one unit of '{final_item}' should have made it through all {} stages — \
         crushed on a belt, mixed and cooked entirely in pipe, bottled back onto a belt, \
         packed and shipped — with no player command whatsoever",
        ordered.len()
    );
    // ...and it waits on the dock, in the stock every contract settles
    // against, which is the actual end of GAMEPLAY.md §1's loop.
    assert_eq!(
        the_dock(&world).contents,
        vec![(final_item.clone(), 1)],
        "the shipped box must stand on the dock it arrived at"
    );
    assert_eq!(
        world.economy_snapshot().delivery_stock,
        vec![(final_item.clone(), 1)],
        "and be on offer to every contract"
    );
}

/// The piped middle of the chain, stated as its own claim rather than left
/// implicit in the conservation walk above: between the mixer and the
/// bottler the product exists **only** as volume in a fluid network, and
/// gets there because a machine put it there.
#[test]
fn the_middle_of_the_chain_moves_product_through_pipes_with_no_tank_behind_them() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    buy(&mut world, "tomato", TOMATO_UNITS);
    buy(&mut world, "sugar", SUGAR_UNITS);
    buy(&mut world, "vinegar", VINEGAR_UNITS);
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: VINEGAR_UNITS,
        tile: None,
    });
    world.issue_command(PlayerCommand::FillHopper {
        amount: TOMATO_UNITS,
        tile: None,
    });

    for _ in 0..40_000 {
        world.tick();
        if the_dock(&world).shipped_total > 0 {
            break;
        }
    }

    let networks = world.fluid_snapshots();
    assert_eq!(
        networks.len(),
        3,
        "three runs: the tank-fed vinegar supply, mixer -> vat, and vat -> bottler"
    );

    let mix = networks
        .iter()
        .find(|n| n.fluid.as_deref() == Some("ketchup_mix"))
        .expect("the mixer's output run must carry the mix it names");
    assert!(
        mix.tank_tiles.is_empty(),
        "nothing but a machine supplies this run — that is what fluid_outputs added"
    );
    assert_eq!(
        mix.producers.len(),
        1,
        "the mixer is on the producing end of it"
    );
    assert_eq!(mix.producers[0].machine_recipe, "ketchup_mix");
    assert_eq!(
        mix.taps.len(),
        1,
        "and the cooking vat is on the drawing end"
    );
    assert_eq!(mix.taps[0].machine_recipe, "ketchup");

    let cooked = networks
        .iter()
        .find(|n| n.fluid.as_deref() == Some("ketchup"))
        .expect("the vat's output run must carry finished ketchup");
    assert!(cooked.tank_tiles.is_empty());
    assert_eq!(cooked.producers[0].machine_recipe, "ketchup");
    assert_eq!(cooked.taps[0].machine_recipe, "bottled_ketchup");
    assert!(
        cooked.total_volume > 0.0,
        "a run that has been primed keeps a standing charge — that is the pressure model, \
         not a leak (`sim::reference` explains why the machine-to-machine runs are short)"
    );

    // Neither of the two piped stages touches a belt at all: no belt tile is
    // adjacent to the cooking vat, and that is the visible shape of the
    // rework rather than an accident of this layout.
    let layout = world.world_layout();
    let vat = layout
        .iter()
        .find(|p| p.kind == "cooking_vat")
        .expect("the vat is placed");
    for tile in &vat.occupies {
        for neighbour in [
            (tile.0 + 1, tile.1),
            (tile.0 - 1, tile.1),
            (tile.0, tile.1 + 1),
            (tile.0, tile.1 - 1),
        ] {
            assert!(
                !matches!(world.query_tile(neighbour), Some(TileSnapshot::Belt { .. })),
                "no belt may touch the cooking vat: liquid in, liquid out"
            );
        }
    }
}

/// The exit end's equivalent of "an unfilled hopper unloads nothing": an
/// idle chain must ship nothing, rather than the dock's per-tick drain
/// manufacturing a delivery out of an empty buffer.
#[test]
fn an_unfed_chain_ships_nothing_and_the_dock_stays_empty() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    for _ in 0..500 {
        world.tick();
    }

    let dock = the_dock(&world);
    assert_eq!(dock.shipped_total, 0, "nothing was ever produced to ship");
    assert!(dock.contents.is_empty());
    assert!(
        world.economy_snapshot().delivery_stock.is_empty(),
        "an idle factory must not offer anything to a contract"
    );

    let (_, machines) = full_chain_snapshot(&world);
    for machine in &machines {
        assert!(
            machine.output.is_empty(),
            "no machine should hold output yet"
        );
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
    assert!(sim_core::sim::command_from_json(r#"{"type": "purchase", "kind": "belt"}"#).is_some());
    // ...including the one the ingredient economy added.
    assert!(
        sim_core::sim::command_from_json(r#"{"type": "buy_ingredient", "item": "tomato"}"#)
            .is_some()
    );
}

#[test]
fn query_tile_reflects_every_stage_belt_and_machine_position() {
    let registry = recipes();
    // The stages the fixture actually lays out, not every recipe in the file.
    let ordered = linear_prefix(&registry).to_vec();
    let mut world = reference_chain_with_budget(registry, BUDGET);
    buy(&mut world, "tomato", 4);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 4,
        tile: None,
    });
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
        "one machine per stage of the reference line"
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

        // What stands immediately west of the machine is now a property of
        // its recipe rather than a fixed rule: a stage with solid inputs is
        // belt-fed (four tiles), and a stage whose inputs are entirely fluid
        // stands straight off the pipe run that feeds it. That is the chain
        // rework in one assertion.
        let feed = (placement.tile.0 - 1, placement.tile.1);
        let belt_fed = !recipe.inputs.is_empty();
        match (world.query_tile(feed), belt_fed) {
            (Some(TileSnapshot::Belt { .. }), true) => {
                for dx in 1..=4 {
                    let tile = (placement.tile.0 - dx, placement.tile.1);
                    match world.query_tile(tile) {
                        Some(TileSnapshot::Belt { .. }) => {}
                        other => panic!("expected belt at {tile:?}, got {other:?}"),
                    }
                }
            }
            (Some(TileSnapshot::Pipe { .. }), false) => {}
            (other, _) => panic!(
                "recipe '{}' has {} solid inputs, so the tile west of it should be a {} — \
                 got {other:?}",
                recipe.id,
                recipe.inputs.len(),
                if belt_fed { "belt" } else { "pipe" }
            ),
        }
    }

    assert!(world.query_tile((10_000, 10_000)).is_none());
}

#[test]
fn dirty_chunks_report_touched_chunks_and_clear_between_ticks() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    buy(&mut world, "tomato", 4);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 4,
        tile: None,
    });
    world.tick();
    // The five recipe-driven stages run x = 0..26 and fit inside chunk (0,0)
    // (ARCHITECTURE.md §3.1's 32x32 chunks); the tomato hopper stands at the
    // head of the first feed lane, west of them at x = -1, i.e. in chunk
    // (-1, 0). On this tick it unloads its first tomato, which changes both:
    // the belt it landed on and the hopper's own fill level.
    // The order is the order the tick's passes touch them, which is
    // deterministic (every pass walks a tile-ordered list) — the hopper's
    // own chunk is marked when it unloads, then the belt's when the tomato
    // lands.
    assert_eq!(world.dirty_chunks(), &[(-1, 0), (0, 0)]);
    // `all_chunks` is the whole world: the line's own chunk and the one the
    // first lane's hopper reaches back into.
    assert_eq!(
        world.all_chunks(),
        vec![(-1, 0), (0, 0)],
        "the tomato hopper's chunk and the line's must both be reported, or a renderer never \
         draws the feed end"
    );

    // A tick where absolutely nothing moves/changes reports no dirty
    // chunks — every machine is idle without input, every belt is empty,
    // and an unfilled hopper has nothing to unload.
    let mut idle_world = reference_chain_with_budget(recipes(), BUDGET);
    // The first tick clears the dirt left by *building* the chain — placing
    // anything dirties its chunks, and the fixture places thirty-odd things.
    idle_world.tick();
    idle_world.tick();
    assert!(idle_world.dirty_chunks().is_empty());
}
