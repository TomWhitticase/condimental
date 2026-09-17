//! Headless integration test for the contract board (the contract-board
//! epic, 2026-09-10; GAMEPLAY.md §6/§9/§11). `sim_core/src/sim/economy.rs`
//! unit-tests signing, strikes and loss against an inline fixture; this
//! file proves the same rules through the real `World` command path and the
//! shipped `data/contracts.ron`: a new game starts with the board's starters
//! only, `{"type": "sign_contract"}` buys the rest out of the one money
//! counter, an offer is signed once at a time, a contract that misses its
//! strikes in a row is lost, and a lost contract is bought back at the same
//! price with a clean record.
//!
//! Nothing here builds a factory: every contract goes unserved on purpose,
//! because "what happens when you fail to deliver" is the whole subject.
//! `tests/economy.rs` is where a real chain pays a contract.

mod common;

use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, MachineRegistry, PlayerCommand, RecipeRegistry, World, command_from_json,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    World::new(recipes())
}

fn active_ids(world: &World) -> Vec<String> {
    world
        .economy()
        .contracts
        .iter()
        .map(|c| c.contract.id.clone())
        .collect()
}

fn sign(world: &mut World, id: &str) {
    world.issue_command(PlayerCommand::SignContract { id: id.to_string() });
}

#[test]
fn a_new_game_starts_with_exactly_the_boards_starters_and_offers_the_rest() {
    let world = world();
    let board = ContractRegistry::bundled();
    let starters: Vec<String> = board.starters().map(|c| c.id.clone()).collect();
    assert_eq!(
        starters.len(),
        1,
        "the shipped board starts a new game with ONE basic contract"
    );
    assert_eq!(active_ids(&world), starters);
    assert!(
        board.len() > 1,
        "and there must be bigger contracts to work toward"
    );
    let snapshot = world.economy_snapshot();
    assert_eq!(snapshot.board.len(), board.len());
    assert_eq!(
        snapshot.board.iter().filter(|o| o.active).count(),
        1,
        "exactly the starter reads active on the board"
    );
}

#[test]
fn the_sign_contract_command_parses_from_json_and_buys_an_offer_out_of_money() {
    let mut world = world();
    let offer = ContractRegistry::bundled()
        .ordered()
        .iter()
        .find(|c| !c.starter)
        .cloned()
        .expect("an unsigned offer");
    let before = world.money();

    let cmd = command_from_json(&format!(
        r#"{{"type": "sign_contract", "id": "{}"}}"#,
        offer.id
    ))
    .expect("sign_contract must parse");
    match &cmd {
        PlayerCommand::SignContract { id } => assert_eq!(id, &offer.id),
        other => panic!("expected SignContract, got {other:?}"),
    }
    world.issue_command(cmd);

    assert_eq!(
        world.money(),
        before - offer.price,
        "the fee left the one money counter"
    );
    assert!(world.economy().is_active(&offer.id));
    let signed = world.economy().contract(&offer.id).unwrap();
    assert_eq!(
        signed.next_delivery_tick,
        world.current_tick() + offer.cadence_ticks,
        "a fresh clock from the moment of signing"
    );
    assert_eq!(
        active_ids(&world),
        ContractRegistry::bundled()
            .ordered()
            .iter()
            .filter(|c| c.starter || c.id == offer.id)
            .map(|c| c.id.clone())
            .collect::<Vec<_>>(),
        "the active list stays in contracts.ron order"
    );
}

#[test]
fn an_offer_cannot_be_held_twice() {
    let mut world = world();
    let offer = ContractRegistry::bundled()
        .ordered()
        .iter()
        .find(|c| !c.starter)
        .cloned()
        .unwrap();
    sign(&mut world, &offer.id);
    let after_first = world.money();
    sign(&mut world, &offer.id);
    assert_eq!(
        world.money(),
        after_first,
        "a second signature is a silent no-op"
    );
    assert_eq!(
        active_ids(&world)
            .iter()
            .filter(|id| **id == offer.id)
            .count(),
        1,
        "one of each contract, never two"
    );

    // The starter is active from tick 0 and is just as unsignable.
    let starter = ContractRegistry::bundled()
        .starters()
        .next()
        .unwrap()
        .id
        .clone();
    sign(&mut world, &starter);
    assert_eq!(world.money(), after_first);
}

#[test]
fn an_unaffordable_or_unknown_signature_changes_nothing() {
    let dearest = ContractRegistry::bundled()
        .ordered()
        .iter()
        .filter(|c| !c.starter)
        .max_by_key(|c| c.price)
        .cloned()
        .unwrap();
    // A world one unit short of the fee.
    let short = dearest.price - Money::from_num(1);
    let mut world = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        short,
    );
    sign(&mut world, &dearest.id);
    assert_eq!(world.money(), short);
    assert!(!world.economy().is_active(&dearest.id));

    world.issue_command(PlayerCommand::SignContract {
        id: "midnight_kebab_van".to_string(),
    });
    assert_eq!(world.money(), short);
    assert_eq!(active_ids(&world).len(), 1);
}

#[test]
fn a_contract_left_unserved_for_its_strikes_is_lost_and_goes_back_on_the_board() {
    let mut world = world();
    let starter = ContractRegistry::bundled()
        .starters()
        .next()
        .unwrap()
        .clone();
    assert!(
        starter.strikes >= 2,
        "the shipped starter forgives at least one miss"
    );

    // Every window is missed: nothing is ever placed, let alone delivered.
    let first_loss_tick = starter.cadence_ticks * u64::from(starter.strikes);
    while world.current_tick() < first_loss_tick - 1 {
        world.tick();
    }
    let active = world
        .economy()
        .contract(&starter.id)
        .expect("still active one tick before");
    assert_eq!(
        active.consecutive_misses,
        starter.strikes - 1,
        "one strike from gone"
    );
    let snapshot = world.economy_snapshot();
    let card = snapshot
        .contracts
        .iter()
        .find(|c| c.id == starter.id)
        .unwrap();
    assert_eq!(card.consecutive_misses, starter.strikes - 1);
    assert_eq!(card.strikes, starter.strikes);
    assert!(!card.satisfied);

    world.tick();
    assert!(
        !world.economy().is_active(&starter.id),
        "the strikes are used up: the contract is gone"
    );
    assert!(
        active_ids(&world).is_empty(),
        "a new game has nothing else running"
    );
    assert_eq!(world.economy().times_lost(&starter.id), 1);
    let snapshot = world.economy_snapshot();
    assert!(snapshot.contracts.is_empty());
    let offer = snapshot.board.iter().find(|o| o.id == starter.id).unwrap();
    assert!(!offer.active, "back on the board");
    assert_eq!(offer.times_lost, 1);

    // The factory keeps ticking with no contract at all — a legal, if
    // penniless, state — and nothing settles.
    let money = world.money();
    let wages_before = common::wages_paid(&world);
    for _ in 0..starter.cadence_ticks {
        world.tick();
    }
    // Only wages move the balance (the staff pools MVP epic): no payout.
    assert_eq!(
        world.money(),
        money - (common::wages_paid(&world) - wages_before)
    );

    // Buying it back costs the listed price and starts a clean record.
    let before = world.money();
    sign(&mut world, &starter.id);
    assert_eq!(world.money(), before - starter.price);
    let again = world.economy().contract(&starter.id).expect("re-signed");
    assert_eq!(again.deliveries_missed, 0);
    assert_eq!(again.consecutive_misses, 0);
    assert_eq!(
        again.next_delivery_tick,
        world.current_tick() + starter.cadence_ticks
    );
}

#[test]
fn a_one_strike_contract_is_lost_on_its_first_miss() {
    let mut world = world();
    let Some(harsh) = ContractRegistry::bundled()
        .ordered()
        .iter()
        .find(|c| c.strikes == 1)
        .cloned()
    else {
        // The shipped board has one; if content changes, the rule is still
        // pinned by economy.rs's unit tests.
        return;
    };
    sign(&mut world, &harsh.id);
    assert!(world.economy().is_active(&harsh.id));
    for _ in 0..harsh.cadence_ticks {
        world.tick();
    }
    assert!(!world.economy().is_active(&harsh.id), "one miss, gone");
}

#[test]
fn the_economy_json_carries_the_board_and_the_strike_state() {
    let mut world = world();
    let offer = ContractRegistry::bundled()
        .ordered()
        .iter()
        .find(|c| !c.starter)
        .cloned()
        .unwrap();
    sign(&mut world, &offer.id);
    let json = serde_json::to_string(&world.economy_snapshot()).expect("must serialize");
    for key in [
        "\"board\":[",
        "\"satisfied\":",
        "\"window_start_tick\":",
        "\"consecutive_misses\":",
        "\"strikes\":",
        "\"price\":",
        "\"affordable\":",
        "\"times_lost\":",
        "\"starter\":",
        "\"active\":",
    ] {
        assert!(json.contains(key), "missing {key} in {json}");
    }
}
