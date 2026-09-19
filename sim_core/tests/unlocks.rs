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
    ContractRegistry, PlayerCommand, RecipeRegistry, UnlockRegistry, World,
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
    assert_eq!(starving.lifetime_deliveries(), 0, "a new game has shipped nothing");
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
        assert!(loaded.ingredient_unlocked(id), "'{id}' must stay unlocked across a load");
    }
    for id in contracts.iter() {
        assert!(loaded.contract_unlocked(id), "'{id}' must stay unlocked across a load");
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
