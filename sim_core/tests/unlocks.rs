//! Headless integration test for the first progression gate
//! (`data/unlocks.ron`, docs/BBQ_SAUCE_PLAN.md §5).
//!
//! `sim_core/src/sim/unlocks.rs` already unit-tests the loader — the schema
//! rejection, the cross-file validation, the threshold arithmetic — against
//! hand-written content. This file drives the real `World` through the real
//! shipped chain, to prove the four things only the whole system can show:
//!
//! 1. **The counter means what the gate claims it means.** It is lifetime
//!    *satisfied delivery windows*, so a factory that ships nothing never
//!    advances it however long it runs, and a factory that ships advances it
//!    exactly once per window it satisfies.
//! 2. **The gate is shut the tick before it opens.** Nine deliveries is
//!    still a locked market and a BBQ-free board; the tenth opens both, in
//!    the same tick, with nothing else changed.
//! 3. **A locked id is genuinely unavailable, not merely unlisted.** The
//!    simulation refuses `BuyIngredient` and `SignContract` for one, so a
//!    presentation layer that forgot to filter — or a player driving the
//!    bridge directly — still cannot spend its way past the gate. This is
//!    the assertion that makes the gate a rule rather than a decoration.
//! 4. **It survives a save.** The counter round-trips byte-equal, and the
//!    loaded world derives the same open gates from it.

mod common;

use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, FLUID_TANK_CAPACITY, PlayerCommand, RecipeRegistry, UnlockRegistry, World,
    reference_chain_with_budget,
};

/// Every id the shipped gate holds back, read from the content rather than
/// repeated here — a balance pass that moves an id between tiers must not
/// need this file edited to keep passing.
fn gated() -> (Vec<String>, Vec<String>) {
    let unlock = UnlockRegistry::bundled()
        .get("bbq_sauce")
        .expect("data/unlocks.ron must ship the bbq_sauce gate")
        .clone();
    (unlock.ingredients, unlock.contracts)
}

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// Stock plus a wage reserve, exactly as `tests/economy.rs` sizes it: these
/// fixtures run for tens of thousands of ticks, and a broke factory loses
/// its crew and then stops delivering, which would look like the counter
/// being wrong.
const BUDGET: u32 = common::STOCK_COST + common::WAGE_RESERVE;

fn stocked_chain() -> World {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    common::stock_up(&mut world);
    world
}

/// Runs the shipped chain, kept fed, until its lifetime delivery counter
/// reaches `target` — and stops on the *exact* tick it does, so a caller can
/// ask what the world looked like at nine deliveries and then at ten.
///
/// Panics rather than returning short: every caller's assertions are
/// meaningless against a world that never got there, and a silent shortfall
/// would read as a gate bug.
fn run_until_delivered(world: &mut World, target: u32) {
    for _ in 0..500_000u32 {
        if world.lifetime_deliveries() >= target {
            return;
        }
        common::supply(world);
        world.tick();
    }
    panic!(
        "the shipped chain must reach {target} deliveries within 500k ticks, got {}",
        world.lifetime_deliveries()
    );
}

/// The counter is satisfied windows, not ticks, not boxes and not money.
#[test]
fn the_counter_advances_once_per_satisfied_window_and_not_at_all_without_one() {
    // An unstocked world: the factory is fully built and running, and has
    // nothing to run *on*. `corner_shop` is active from tick 0 and its
    // windows come round every 1 000 ticks, so this passes several of them.
    let mut starving = reference_chain_with_budget(recipes(), BUDGET);
    assert_eq!(
        starving.lifetime_deliveries(),
        0,
        "a new game has shipped nothing"
    );
    for _ in 0..5_000 {
        starving.tick();
    }
    assert_eq!(
        starving.lifetime_deliveries(),
        0,
        "a missed window is not a delivery — the counter must not advance on the \
         clock alone, or the gate would open for a factory that never ran"
    );

    // The same chain, fed. The counter must agree with the per-contract
    // ledgers it is the lifetime sum of.
    let mut world = stocked_chain();
    run_until_delivered(&mut world, 3);
    let per_contract: u32 = world
        .economy()
        .contracts
        .iter()
        .map(|active| active.deliveries_made)
        .sum();
    assert_eq!(
        world.lifetime_deliveries(),
        per_contract,
        "the lifetime counter must be the sum of what the active contracts were \
         actually paid for"
    );
}

/// The gate itself: shut at nine, open at ten, and open for both lists at
/// once.
#[test]
fn the_gate_is_shut_one_delivery_short_and_open_on_the_threshold() {
    let threshold = UnlockRegistry::bundled()
        .get("bbq_sauce")
        .expect("the shipped gate")
        .after_deliveries;
    let (ingredients, contracts) = gated();
    let mut world = stocked_chain();

    run_until_delivered(&mut world, threshold - 1);
    assert_eq!(world.lifetime_deliveries(), threshold - 1);
    let market = world.ingredients_snapshot();
    let board = world.economy_snapshot();
    for id in &ingredients {
        assert!(
            !world.ingredient_unlocked(id),
            "'{id}' must still be locked one delivery short of the threshold"
        );
        assert!(
            !market.entries.iter().any(|e| &e.id == id),
            "'{id}' must not be listed in the market one delivery short"
        );
    }
    for id in &contracts {
        assert!(
            !world.contract_unlocked(id),
            "'{id}' must still be locked one delivery short of the threshold"
        );
        assert!(
            !board.board.iter().any(|o| &o.id == id),
            "'{id}' must not be listed on the board one delivery short"
        );
    }

    run_until_delivered(&mut world, threshold);
    assert_eq!(world.lifetime_deliveries(), threshold);
    for id in &ingredients {
        assert!(
            world.ingredient_unlocked(id),
            "'{id}' must be on sale on the threshold delivery"
        );
    }
    for id in &contracts {
        assert!(
            world.contract_unlocked(id),
            "'{id}' must be on the board on the threshold delivery"
        );
    }

    // Both lists, the same tick: the announcement is one sentence about one
    // tier, so a gate that released its ingredients before its contracts
    // would be two events and a lie.
    let market = world.ingredients_snapshot();
    let board = world.economy_snapshot();
    for id in &ingredients {
        assert!(
            market.entries.iter().any(|e| &e.id == id),
            "'{id}' must be listed in the market the tick the gate opens"
        );
    }
    for id in &contracts {
        assert!(
            board.board.iter().any(|o| &o.id == id),
            "'{id}' must be listed on the board the tick the gate opens"
        );
    }
    assert_eq!(board.lifetime_deliveries, threshold);
    let bbq = board
        .unlocks
        .iter()
        .find(|u| u.id == "bbq_sauce")
        .expect("the snapshot must carry the gate, so the HUD can announce it");
    assert!(bbq.open);
    assert_eq!(bbq.name, "BBQ Sauce");
}

/// A gate is enforced by the simulation, not by a panel that chose not to
/// draw a row. Refusing the command is what makes it a rule.
#[test]
fn a_locked_ingredient_cannot_be_bought_however_the_command_arrives() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let before = world.money();

    for id in &ingredients {
        world.issue_command(PlayerCommand::BuyIngredient {
            item: id.clone(),
            quantity: 5,
        });
        assert_eq!(
            world.owned_ingredient(id),
            0,
            "'{id}' is behind a closed gate — buying it must be refused, not merely \
             hidden from the market panel"
        );
    }
    assert_eq!(
        world.money(),
        before,
        "a refused purchase is a total no-op: no stock, and no money spent either"
    );
}

/// The same, for the board. A signing fee is real money, so a refusal that
/// charged for it would be worse than no gate at all.
#[test]
fn a_locked_contract_cannot_be_signed_however_the_command_arrives() {
    let (_, contracts) = gated();
    let board = ContractRegistry::bundled();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let before = world.money();
    let active_before = world.economy().contracts.len();

    for id in &contracts {
        assert!(
            board.get(id).is_some(),
            "'{id}' must exist in data/contracts.ron for this to be a gate rather \
             than a typo"
        );
        world.issue_command(PlayerCommand::SignContract { id: id.clone() });
    }

    assert_eq!(
        world.economy().contracts.len(),
        active_before,
        "a contract behind a closed gate must not become active"
    );
    assert_eq!(
        world.money(),
        before,
        "a refused signing must not charge the signing fee"
    );
}

/// Determinism: the counter is persisted state, so it round-trips exactly,
/// and the loaded world derives the same gates from it.
#[test]
fn the_counter_and_the_gates_it_opens_survive_a_save_round_trip() {
    let threshold = UnlockRegistry::bundled()
        .get("bbq_sauce")
        .expect("the shipped gate")
        .after_deliveries;
    let (ingredients, contracts) = gated();

    let mut world = stocked_chain();
    run_until_delivered(&mut world, threshold);

    let text = world.to_save_ron();
    let loaded = World::from_save_ron(&text, recipes()).expect("the save must load");

    assert_eq!(
        loaded.lifetime_deliveries(),
        world.lifetime_deliveries(),
        "the counter is saved state, not something recomputed from the active \
         contracts — a lost contract must not re-lock a tier"
    );
    for id in ingredients.iter() {
        assert!(
            loaded.ingredient_unlocked(id),
            "'{id}' must stay unlocked across a load"
        );
    }
    for id in contracts.iter() {
        assert!(
            loaded.contract_unlocked(id),
            "'{id}' must stay unlocked across a load"
        );
    }
    assert_eq!(
        loaded.to_save_ron(),
        text,
        "a save of a loaded save must be byte-identical, counter included"
    );
}

/// A save written before the gate existed has no `lifetime_deliveries`
/// field. It loads at zero and re-locks the tier, deliberately — see
/// `save.rs`'s module doc for the argument. This pins the behaviour so the
/// decision cannot be reversed by accident.
#[test]
fn a_save_written_before_the_gate_existed_loads_at_zero_deliveries() {
    let world = reference_chain_with_budget(recipes(), 1_000);
    let text = world.to_save_ron();
    assert!(
        text.contains("lifetime_deliveries"),
        "a save written now records the counter"
    );

    // Exactly what an older save is: the same RON with the field absent.
    let older: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("lifetime_deliveries:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!older.contains("lifetime_deliveries"));

    let loaded = World::from_save_ron(&older, recipes()).expect("an older save must still load");
    assert_eq!(loaded.lifetime_deliveries(), 0);
    assert!(
        !loaded.ingredient_unlocked("spice_mix"),
        "the tier re-locks rather than being handed back unannounced"
    );
    assert_eq!(
        loaded.money(),
        Money::from_num(1_000),
        "and nothing else about the save changed"
    );
}

// =====================================================================
// The contents picker (issue #90)
// =====================================================================
//
// A gate holds back an ingredient, and until this the two commands that
// point a BUILDING at one — `SetHopperItem` and `SetTankFluid` — accepted it
// anyway. The player could aim a hopper at an ingredient nobody could buy,
// and the hopper then sat empty forever saying only "in stock: 0". The
// market's rule ("refused, not merely hidden") now covers them, and the
// picker's wire carries what the panel needs to say so.

/// The reference chain's first hopper, and its tank — the two buildings the
/// configuration panel's CONTENTS picker is drawn for.
const FIRST_HOPPER: (i32, i32) = (-1, 0);

fn hopper_item(world: &World, tile: (i32, i32)) -> String {
    match world.query_tile(tile) {
        Some(sim_core::sim::TileSnapshot::Hopper { item, .. }) => item,
        other => panic!("expected a hopper at {tile:?}, found {other:?}"),
    }
}

/// Pointing a placed hopper at a locked ingredient is refused, and refused
/// *totally*: the bin keeps the item it had, so a misdirected click cannot
/// even empty it.
#[test]
fn a_hopper_cannot_be_pointed_at_a_locked_ingredient() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let before = hopper_item(&world, FIRST_HOPPER);
    assert!(
        !ingredients.contains(&before),
        "the fixture's hopper must start on an UNLOCKED item for this to mean anything"
    );

    for id in &ingredients {
        world.issue_command(PlayerCommand::SetHopperItem {
            tile: FIRST_HOPPER,
            item: id.clone(),
        });
        assert_eq!(
            hopper_item(&world, FIRST_HOPPER),
            before,
            "'{id}' is behind a closed gate, so the hopper must keep holding '{before}' — \
             a dimmed slot the simulation still honours is a decoration, not a gate"
        );
    }
}

/// And the same click, once the gate has opened, is accepted — so the
/// refusal above is the gate and not some other rule quietly declining.
#[test]
fn the_same_hopper_click_lands_once_the_gate_opens() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    sim_core::sim::reference::open_every_gate(&mut world);
    let wanted = ingredients
        .iter()
        .find(|id| world.raw_items().iter().any(|raw| raw == *id))
        .expect("the gate must hold back at least one RAW solid for this test");

    world.issue_command(PlayerCommand::SetHopperItem {
        tile: FIRST_HOPPER,
        item: wanted.clone(),
    });
    assert_eq!(
        &hopper_item(&world, FIRST_HOPPER),
        wanted,
        "with the gate open this is an ordinary retarget"
    );
}

/// Where the rule deliberately STOPS: a hopper may be placed holding a
/// locked ingredient, and is then a bin that can never be filled.
///
/// `Place`'s `item` is not a choice made against a gate. The build ghost
/// seeds it — and skips locked ids when it does (`_selected_hopper_item`) —
/// while a layout fixture names it to stand a factory up for a screenshot.
/// Refusing it would close nothing a player can reach and would cost every
/// BBQ fixture ten deliveries of real play before it could place a bin,
/// which `godot_project/test_layouts.gd` has no honest way to pay.
///
/// What makes that safe is the half asserted below: the gate still bites the
/// bin. `BuyIngredient` is refused for the locked raw, so the larder never
/// holds a unit of it, so neither a manual fill nor auto-fill can put
/// anything in the hopper. The player cannot reach a state the picker would
/// not have let them reach; they just get an empty bin instead of a refusal.
#[test]
fn a_hopper_placed_holding_a_locked_ingredient_can_never_be_filled() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let locked = ingredients
        .iter()
        .find(|id| world.raw_items().iter().any(|raw| raw == *id))
        .expect("the gate must hold back at least one RAW solid for this test")
        .clone();
    let free = (40, 40);

    world.issue_command(PlayerCommand::Purchase {
        kind: "hopper".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "hopper".to_string(),
        tile: free,
        direction: sim_core::sim::Direction::East,
        item: Some(locked.clone()),
    });
    assert_eq!(
        hopper_item(&world, free),
        locked,
        "a Place naming a locked raw stands the bin up — the gate is not on this command"
    );

    // And now the half that matters. Buy all you like: the gate refuses.
    world.issue_command(PlayerCommand::BuyIngredient {
        item: locked.clone(),
        quantity: 50,
    });
    world.issue_command(PlayerCommand::SetHopperAutoFill {
        tile: free,
        enabled: true,
    });
    for _ in 0..50 {
        world.tick();
    }
    let held = world
        .hopper_snapshots()
        .into_iter()
        .find(|hopper| hopper.tile == free)
        .expect("the hopper this test placed");
    assert_eq!(
        held.quantity, 0,
        "'{locked}' cannot be bought, so an auto-filling hopper of it stays empty forever"
    );

    // Once the gate opens the same bin fills from the same commands.
    sim_core::sim::reference::open_every_gate(&mut world);
    world.issue_command(PlayerCommand::BuyIngredient {
        item: locked.clone(),
        quantity: 50,
    });
    world.tick();
    let filled = world
        .hopper_snapshots()
        .into_iter()
        .find(|hopper| hopper.tile == free)
        .expect("the hopper this test placed");
    assert!(
        filled.quantity > 0,
        "with the gate open the same bin fills — the emptiness was the gate's, not the hopper's"
    );
}

/// The wire the CONTENTS picker is drawn from. A locked id is KEPT here and
/// flagged, where the market listing omits it: the picker is the full set of
/// things this building will ever hold, and a gate nobody can see coming is
/// one nobody anticipates (GAMEPLAY.md §2).
#[test]
fn the_picker_wire_keeps_a_locked_choice_and_names_its_gate() {
    let (ingredients, _) = gated();
    let world = reference_chain_with_budget(recipes(), 10_000);
    let snapshot: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.ingredients_snapshot()).unwrap())
            .unwrap();

    let threshold = UnlockRegistry::bundled()
        .get("bbq_sauce")
        .expect("data/unlocks.ron must ship the bbq_sauce gate")
        .after_deliveries;
    let name = UnlockRegistry::bundled()
        .get("bbq_sauce")
        .unwrap()
        .name
        .clone();

    let mut seen_locked = 0;
    for list in ["hopper_items", "tank_fluids"] {
        let choices = snapshot[list].as_array().expect("a list of choices");
        assert!(!choices.is_empty(), "{list} must not be empty");
        for choice in choices {
            let id = choice["id"].as_str().unwrap().to_string();
            let locked = choice["locked"].as_bool().unwrap();
            assert_eq!(
                locked,
                ingredients.contains(&id),
                "'{id}' in {list} reports locked={locked}, which disagrees with the gate"
            );
            if locked {
                seen_locked += 1;
                assert_eq!(choice["unlock_name"].as_str().unwrap(), name);
                assert_eq!(
                    choice["after_deliveries"].as_u64().unwrap() as u32,
                    threshold
                );
            } else {
                assert_eq!(choice["unlock_name"].as_str().unwrap(), "");
                assert_eq!(choice["after_deliveries"].as_u64().unwrap(), 0);
            }
            assert!(
                locked
                    || snapshot["entries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|entry| entry["id"] == id.as_str()),
                "an unlocked choice is on the market too"
            );
        }
    }
    assert!(
        seen_locked > 0,
        "the shipped gate must hold back at least one raw, or this test proves nothing"
    );
    assert_eq!(
        snapshot["deliveries"].as_u64().unwrap(),
        0,
        "and the count to read those thresholds against rides along"
    );
    for id in &ingredients {
        assert!(
            !snapshot["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["id"] == id.as_str()),
            "'{id}' is still absent from the MARKET listing — the picker shows what \
             the market hides, not the other way round"
        );
    }
}

/// Every choice unlocks together with the gate, and nothing then claims a
/// gate to describe.
#[test]
fn the_picker_wire_goes_quiet_once_the_gate_opens() {
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    sim_core::sim::reference::open_every_gate(&mut world);
    let snapshot = world.ingredients_snapshot();
    assert!(
        snapshot
            .hopper_items
            .iter()
            .chain(snapshot.tank_fluids.iter())
            .all(|choice| !choice.locked
                && choice.unlock_name.is_empty()
                && choice.after_deliveries == 0),
        "with every gate open no choice is locked, and an unlocked one carries no gate"
    );
}

/// The tank half of the same rule. Issue #43 built both pickers out of one
/// piece of Godot code and #90 notes that two pickers behaving differently
/// would be worse than one consistent gap, so the fluid command declines a
/// locked fluid exactly as the solid one declines a locked ingredient — and
/// declines it BEFORE the purge, so a refused declaration does not cost the
/// player the vinegar already in the drum.
#[test]
fn a_tank_cannot_be_declared_to_a_locked_fluid() {
    /// The reference chain's fluid tank (`tests/auto_fill.rs` pins the same tile).
    const TANK: (i32, i32) = (5, 2);
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    common::stock_up(&mut world);
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: Some(TANK),
    });
    let held = world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|tank| tank.tile == TANK)
        .expect("the reference chain ships a fluid tank");
    assert!(
        held.quantity > 0.0,
        "the fixture wants fluid standing in the drum, so a wrongly-allowed purge would show"
    );

    let locked = ingredients
        .iter()
        .find(|id| world.raw_fluids().contains(*id))
        .expect("the gate must hold back at least one RAW fluid for this test")
        .clone();
    world.issue_command(PlayerCommand::SetTankFluid {
        tile: TANK,
        fluid: locked.clone(),
    });
    let after = world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|tank| tank.tile == TANK)
        .unwrap();
    assert_eq!(
        after.fluid, held.fluid,
        "'{locked}' is behind a closed gate, so the drum still holds what it held"
    );
    assert_eq!(
        after.quantity, held.quantity,
        "and a refused declaration purges nothing — the refusal comes first"
    );
}
