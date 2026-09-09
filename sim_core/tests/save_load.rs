//! Headless integration test for save/load (the "Save/load + a real main
//! menu" epic's ticket 1).
//!
//! `sim_core/src/sim/save.rs` unit-tests the loader's refusals in isolation
//! — a wrong schema version, an unknown kind, two placements on one tile, a
//! hopper holding something that is not raw, and so on. This file drives the
//! real shipped chain instead, because the thing that actually has to be
//! proven is not that a file parses:
//!
//! **A saved-and-reloaded factory must be running-state-identical, not
//! layout-identical.** A machine mid-craft must not restart its batch, an
//! item 37% of the way across a belt tile must come back at 37%, fluid must
//! be in every pipe at exactly the volume it was, and every contract's own
//! cadence clock must keep counting from where it was. So the central test
//! here builds a chain, runs it into a genuinely messy mid-flight state,
//! saves it, loads the save into a *different* world, and then ticks **both
//! worlds forward in lockstep for more than a full contract cadence,
//! comparing every snapshot the `SimBridge` boundary exposes at every single
//! tick**. Divergence anywhere — a belt position off by one fixed-point bit,
//! a craft restarted, a contract clock reset — shows up as a mismatch on the
//! tick it happens rather than as a vague "it looked wrong later".

mod common;

use sim_core::sim::save::{SaveData, SavedBody};
use sim_core::sim::types::Fx;
use sim_core::sim::{PlayerCommand, RecipeRegistry, World, reference_chain_with_budget};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// How long the two worlds are run side by side after the load. Longer than
/// the longest shipped contract cadence (2500 ticks), so at least one of
/// every contract's windows settles *after* the save — which is what proves
/// the restored clocks are the real ones and not a coincidence.
const LOCKSTEP_TICKS: u32 = 2600;

/// Everything the `gdext` boundary can report about a world, as one string.
/// If two worlds produce the same one, presentation cannot tell them apart —
/// which is the operational definition of "the save came back".
fn fingerprint(world: &World) -> String {
    let mut out = String::new();
    out.push_str(&format!("tick={}\n", world.current_tick()));
    let mut push = |label: &str, json: String| {
        out.push_str(label);
        out.push('=');
        out.push_str(&json);
        out.push('\n');
    };
    push(
        "layout",
        serde_json::to_string(&world.world_layout()).unwrap(),
    );
    push(
        "hoppers",
        serde_json::to_string(&world.hopper_snapshots()).unwrap(),
    );
    push(
        "tanks",
        serde_json::to_string(&world.fluid_tank_snapshots()).unwrap(),
    );
    push(
        "fluids",
        serde_json::to_string(&world.fluid_snapshots()).unwrap(),
    );
    push(
        "docks",
        serde_json::to_string(&world.dock_snapshots()).unwrap(),
    );
    push(
        "economy",
        serde_json::to_string(&world.economy_snapshot()).unwrap(),
    );
    push("shop", serde_json::to_string(&world.shop_snapshot()).unwrap());
    push(
        "ingredients",
        serde_json::to_string(&world.ingredients_snapshot()).unwrap(),
    );
    for chunk in world.all_chunks() {
        push(
            &format!("chunk{chunk:?}"),
            serde_json::to_string(&world.chunk_snapshot(chunk)).unwrap(),
        );
    }
    out
}

/// The shipped chain, stocked, and run until it is genuinely mid-flight:
/// crafts running, items part way along belts, pipes part full, boxes
/// already shipped and contracts already settled — and then left unfed long
/// enough for the hoppers to be part empty rather than sitting at capacity.
///
/// The state this reaches is asserted, not assumed (see
/// [`assert_mid_flight`]): a fixture that quietly stopped exercising a
/// mid-craft machine would turn the whole file green while proving less.
fn mid_flight_world() -> World {
    let mut world = reference_chain_with_budget(recipes(), common::STOCK_COST);
    common::stock_up(&mut world);
    for _ in 0..3000 {
        common::supply(&mut world);
        world.tick();
    }
    // Stop feeding, so the hoppers drain to a partial level instead of
    // being topped back up to full every tick.
    for _ in 0..400 {
        world.tick();
    }
    world
}

/// The fixture's own preconditions. Each one is a distinct kind of live
/// state a layout-only save would lose.
fn assert_mid_flight(world: &World) {
    let save = world.to_save();

    let mid_craft: Vec<&str> = save
        .placements
        .iter()
        .filter(|p| {
            matches!(&p.body, SavedBody::Machine(m) if m.process.is_some())
        })
        .map(|p| p.kind.as_str())
        .collect();
    assert!(
        mid_craft.contains(&"crusher") && mid_craft.contains(&"cooking_vat"),
        "the fixture must catch the crusher AND the cooking vat mid-craft, got {mid_craft:?}"
    );

    let fractional_belt_items = save
        .placements
        .iter()
        .filter_map(|p| match &p.body {
            SavedBody::Belt(belt) => Some(belt),
            _ => None,
        })
        .flat_map(|belt| belt.items.iter())
        // A whole tile-unit is 1 << 16 in `Fx`'s raw bits, so a non-zero
        // remainder is an item genuinely part way across its tile.
        .filter(|(_, bits)| bits.0 % (1 << 16) != 0)
        .count();
    assert!(
        fractional_belt_items >= 4,
        "the fixture must have several items at fractional belt positions, got \
         {fractional_belt_items}"
    );

    let partial_hoppers = save
        .placements
        .iter()
        .filter(|p| {
            matches!(&p.body, SavedBody::Hopper(h) if h.quantity > 0 && h.quantity < 64)
        })
        .count();
    assert!(
        partial_hoppers > 0,
        "the fixture must have at least one part-full hopper"
    );

    let filled_networks = world
        .fluid_snapshots()
        .iter()
        .filter(|network| network.total_volume > 0.0)
        .count();
    assert!(
        filled_networks >= 2,
        "the fixture must have fluid standing in at least two pipe runs, got {filled_networks}"
    );

    let shipped: u32 = world
        .dock_snapshots()
        .iter()
        .map(|dock| dock.shipped_total)
        .sum();
    assert!(shipped > 0, "the fixture's dock must have shipped something");

    let settled: u32 = save
        .contracts
        .iter()
        .map(|c| c.deliveries_made + c.deliveries_missed)
        .sum();
    assert!(
        settled > 0,
        "at least one contract must have settled a window before the save"
    );
}

/// **The big one.** Save a mid-flight factory, load it into a new world, and
/// prove the two are indistinguishable — now, and at every tick of a run
/// longer than the longest contract cadence.
#[test]
fn a_saved_factory_comes_back_running_state_identical() {
    let mut original = mid_flight_world();
    assert_mid_flight(&original);

    let text = original.to_save_ron();
    let mut loaded = World::from_save_ron(&text, recipes()).expect("the save must load");

    assert_eq!(
        fingerprint(&loaded),
        fingerprint(&original),
        "a freshly loaded world must be indistinguishable from the one it was saved from"
    );

    for step in 0..LOCKSTEP_TICKS {
        original.tick();
        loaded.tick();
        assert_eq!(
            fingerprint(&loaded),
            fingerprint(&original),
            "the two worlds diverged {} tick(s) after the load",
            step + 1
        );
    }

    // And after all that, the loaded world still saves to the same file the
    // original does — the round trip is stable, not merely close.
    assert_eq!(original.to_save_ron(), loaded.to_save_ron());
}

/// A loaded world announces itself to a chunk-driven renderer: every chunk
/// it occupies is dirty, so presentation redraws all of it rather than only
/// whatever the next tick happens to touch.
#[test]
fn a_loaded_world_reports_every_chunk_dirty() {
    let world = mid_flight_world();
    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).unwrap();

    let mut dirty = loaded.dirty_chunks().to_vec();
    let mut all = loaded.all_chunks();
    dirty.sort();
    all.sort();
    assert!(!all.is_empty(), "the fixture occupies chunks");
    assert_eq!(dirty, all);
}

/// Two saves of one world are byte-identical, and a save survives a
/// load-and-re-save unchanged. Both are the same property — the format has
/// no incidental ordering in it — and both are what lets a test (or a
/// player's save-slot list) compare two saves at all.
#[test]
fn saving_is_deterministic_and_the_round_trip_is_byte_exact() {
    let world = mid_flight_world();
    let first = world.to_save_ron();
    let second = world.to_save_ron();
    assert_eq!(first, second, "two saves of one world must be identical");

    let loaded = World::from_save_ron(&first, recipes()).unwrap();
    assert_eq!(
        loaded.to_save_ron(),
        first,
        "to_ron(from_ron(s)) must be byte-for-byte s"
    );

    // ...and again, so a save that only survived one trip would still fail.
    let twice = World::from_save_ron(&loaded.to_save_ron(), recipes()).unwrap();
    assert_eq!(twice.to_save_ron(), first);
}

/// A brand new game saved immediately: no placements, no stock, full
/// starting balance, every contract on its opening clock. The degenerate
/// case a main menu hits first.
#[test]
fn an_empty_world_round_trips() {
    let world = World::new(recipes());
    let text = world.to_save_ron();
    let loaded = World::from_save_ron(&text, recipes()).expect("an empty save must load");

    assert_eq!(fingerprint(&loaded), fingerprint(&world));
    assert_eq!(loaded.placement_count(), 0);
    assert_eq!(loaded.money(), world.money());
    assert_eq!(loaded.to_save_ron(), text);
}

/// A world that has been *played* but not built — money spent on stock and
/// equipment that is bought and not yet placed. The unplaced count is the
/// player's inventory, and a rebuild buys and spends one of everything it
/// places, so this is the case that catches a loader letting its own
/// purchases leak into either ledger.
#[test]
fn unplaced_inventory_and_bought_stock_survive() {
    let mut world = World::new(recipes());
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".into(),
        quantity: 7,
    });
    world.issue_command(PlayerCommand::Purchase {
        kind: "crusher".into(),
        quantity: 2,
    });
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".into(),
        quantity: 30,
    });
    // Place one of the seven, so owned and placed are genuinely different
    // numbers rather than accidentally equal.
    world.issue_command(PlayerCommand::Place {
        kind: "belt".into(),
        tile: (0, 0),
        direction: sim_core::sim::Direction::East,
        item: None,
    });
    let money = world.money();

    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).unwrap();
    assert_eq!(loaded.owned_count("belt"), 6);
    assert_eq!(loaded.owned_count("crusher"), 2);
    assert_eq!(loaded.owned_ingredient("tomato"), 30);
    assert_eq!(loaded.placement_count(), 1);
    assert_eq!(
        loaded.money(),
        money,
        "the rebuild's own purchases must not leak into the restored balance"
    );
}

/// A machine caught mid-craft keeps its exact batch: the same `start_tick`,
/// so it finishes on the same absolute tick it would have, rather than
/// restarting its timer from the load.
#[test]
fn a_craft_in_flight_finishes_when_it_would_have() {
    let mut world = mid_flight_world();
    let save = world.to_save();
    let (kind, start_tick, required) = save
        .placements
        .iter()
        .find_map(|p| match &p.body {
            SavedBody::Machine(m) => m.process.map(|(s, r)| (p.kind.clone(), s, r)),
            _ => None,
        })
        .expect("the fixture has a machine mid-craft");
    assert!(
        start_tick < world.current_tick(),
        "'{kind}' must have started its batch before the save"
    );

    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).unwrap();
    let reloaded = loaded.to_save();
    let restored = reloaded
        .placements
        .iter()
        .find_map(|p| match &p.body {
            SavedBody::Machine(m) if p.kind == kind => m.process,
            _ => None,
        })
        .expect("the same machine must still be mid-craft after the load");
    assert_eq!(restored, (start_tick, required));

    // The batch is genuinely still ticking down, not frozen: run both to
    // where it completes and check they agree the whole way.
    let mut loaded = loaded;
    for _ in 0..(required + 10) {
        world.tick();
        loaded.tick();
    }
    assert_eq!(fingerprint(&loaded), fingerprint(&world));
}

/// Fluid is restored to the bit, in tanks and in every pipe tile alike —
/// the state most obviously lost by a layout-only save, and the one a
/// decimal round trip would quietly round.
#[test]
fn every_drop_of_fluid_comes_back_exactly() {
    let world = mid_flight_world();
    let before = world.to_save();
    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).unwrap();
    let after = loaded.to_save();

    let volumes = |save: &SaveData| -> Vec<(String, i32)> {
        save.placements
            .iter()
            .filter_map(|p| match &p.body {
                SavedBody::Pipe(pipe) => Some((format!("pipe{:?}", p.tile), pipe.volume_bits.0)),
                SavedBody::FluidTank(tank) => {
                    Some((format!("tank{:?}", p.tile), tank.volume_bits.0))
                }
                _ => None,
            })
            .collect()
    };
    let before = volumes(&before);
    assert!(
        before.iter().any(|(_, bits)| *bits > 0),
        "the fixture must actually have fluid in it"
    );
    assert!(
        before.iter().any(|(_, bits)| bits % (1 << 16) != 0),
        "and at least some of it at a fractional volume, which is what raw bits are for"
    );
    assert_eq!(volumes(&after), before);
}

/// Two text-level refusals, through the same string API the `SimBridge`
/// boundary uses. The full refusal suite lives in `save.rs`'s own unit
/// tests, against a `SaveData` it can mutate field by field; these two are
/// the ones a player can actually hit — a save from another build, and a
/// file that is not a save at all.
#[test]
fn a_save_from_another_build_and_a_file_that_is_not_a_save_are_both_refused() {
    let world = World::new(recipes());
    let text = world.to_save_ron();

    let from_the_future = text.replace("schema_version: 1", "schema_version: 2");
    let Err(error) = World::from_save_ron(&from_the_future, recipes()) else {
        panic!("a different schema version must be refused, never migrated");
    };
    let error = error.to_string();
    assert!(error.contains("schema_version 2"), "unhelpful: {error}");

    let Err(error) = World::from_save_ron("this is not a save file", recipes()) else {
        panic!("garbage must be refused");
    };
    let error = error.to_string();
    assert!(error.contains("parse"), "unhelpful: {error}");
}

/// Money is fixed-point ([`Fx`], ARCHITECTURE.md §3.9), and a save carries
/// its raw bits rather than a decimal print. Nothing in the shipped economy
/// produces a fractional balance today — every price and every payout is a
/// whole number — so this pins the property rather than a currently-visible
/// bug: the moment anything charges a fraction (a per-tick wage, a
/// percentage fee), a decimal round trip would start losing a bit per save.
#[test]
fn money_survives_to_the_bit() {
    let mut world = reference_chain_with_budget(recipes(), common::STOCK_COST + 137);
    common::stock_up(&mut world);
    for _ in 0..1200 {
        common::supply(&mut world);
        world.tick();
    }
    let money = world.money();
    assert!(money >= Fx::from_num(137), "the fixture must hold money");

    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).unwrap();
    assert_eq!(loaded.money().to_bits(), money.to_bits());

    // And a deliberately fractional balance, which no command can currently
    // reach but the format must not be able to lose either.
    let mut save = world.to_save();
    save.money_bits = sim_core::sim::save::FxBits(money.to_bits() + 1);
    let text = save.to_ron();
    let odd = World::from_save_ron(&text, recipes()).unwrap();
    assert_eq!(odd.money().to_bits(), money.to_bits() + 1);
    assert_eq!(odd.to_save_ron(), text);
}

/// The shipped chain runs east and has no splitter in it, so the reference
/// fixture above never exercises three things a real player's factory will:
/// **a placement facing somewhere other than east**, **a splitter's
/// next-due side**, and **more than one dock**. This builds a small line
/// that has all three — a hopper feeding a belt into a splitter that deals
/// alternately to two docks facing different ways, plus a tank-fed pipe run
/// no machine taps — and round-trips it in lockstep.
#[test]
fn a_forked_line_with_turned_placements_round_trips() {
    use sim_core::sim::Direction;

    let mut world = World::new(recipes());
    let buy = |world: &mut World, kind: &str, quantity: u32| {
        world.issue_command(PlayerCommand::Purchase {
            kind: kind.to_string(),
            quantity,
        });
        assert_eq!(
            world.owned_count(kind),
            quantity,
            "the fixture must be able to afford {quantity} x {kind}"
        );
    };
    buy(&mut world, "hopper", 1);
    buy(&mut world, "belt", 4);
    buy(&mut world, "splitter", 1);
    buy(&mut world, "dock", 2);
    buy(&mut world, "fluid_tank", 1);
    buy(&mut world, "pipe", 2);

    let place = |world: &mut World, kind: &str, tile, direction, item: Option<&str>| {
        let before = world.placement_count();
        world.issue_command(PlayerCommand::Place {
            kind: kind.to_string(),
            tile,
            direction,
            item: item.map(str::to_string),
        });
        assert_eq!(
            world.placement_count(),
            before + 1,
            "the fixture's {kind} at {tile:?} was refused"
        );
    };
    place(&mut world, "hopper", (0, 0), Direction::East, Some("tomato"));
    place(&mut world, "belt", (1, 0), Direction::East, None);
    place(&mut world, "belt", (2, 0), Direction::East, None);
    // Straight is east to (4, 0); the branch is one quarter turn clockwise,
    // i.e. south to (3, -1).
    place(&mut world, "splitter", (3, 0), Direction::East, None);
    place(&mut world, "belt", (4, 0), Direction::East, None);
    place(&mut world, "dock", (5, 0), Direction::East, None);
    place(&mut world, "belt", (3, -1), Direction::South, None);
    place(&mut world, "dock", (3, -2), Direction::South, None);
    // A run with fluid in it that nothing taps — a `None`-fluid network,
    // which is a shape the shipped chain never has.
    place(&mut world, "fluid_tank", (0, 3), Direction::East, None);
    place(&mut world, "pipe", (1, 3), Direction::East, None);
    place(&mut world, "pipe", (2, 3), Direction::West, None);

    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".into(),
        quantity: 40,
    });
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "vinegar".into(),
        quantity: 20,
    });
    world.issue_command(PlayerCommand::FillHopper {
        amount: 40,
        tile: None,
    });
    // The tank deliberately stays empty: a tank plumbed into a run no
    // machine taps has no fluid id to charge the player for, so
    // `FillFluidTank` is a no-op there (`world.rs`). That empty, unnamed
    // network is itself the case being round-tripped here; the *full* pipe
    // runs are covered by the shipped-chain fixture above.
    for _ in 0..900 {
        world.tick();
    }

    let save = world.to_save();
    assert!(
        save.placements
            .iter()
            .any(|p| matches!(&p.body, SavedBody::Splitter(_))),
        "the fixture must contain a splitter"
    );
    assert!(
        save.placements
            .iter()
            .any(|p| p.direction != Direction::East),
        "the fixture must contain a turned placement"
    );
    let shipped: u32 = world
        .dock_snapshots()
        .iter()
        .map(|dock| dock.shipped_total)
        .sum();
    assert!(shipped > 0, "both docks must have taken product by now");

    let text = world.to_save_ron();
    let mut loaded = World::from_save_ron(&text, recipes()).expect("the forked line must load");
    assert_eq!(fingerprint(&loaded), fingerprint(&world));

    let mut world = world;
    for step in 0..600 {
        world.tick();
        loaded.tick();
        assert_eq!(
            fingerprint(&loaded),
            fingerprint(&world),
            "the forked line diverged {} tick(s) after the load",
            step + 1
        );
    }
    assert_eq!(loaded.to_save_ron(), world.to_save_ron());
}
