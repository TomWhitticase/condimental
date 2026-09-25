//! Headless integration test for the one cheat the simulation knows about:
//! `PlayerCommand::DebugGrantMoney`, added with Godot's F3 debug menu
//! (2026-09-21).
//!
//! The reason a cheat is a *command* at all — rather than a number the
//! presentation layer adds to what it draws — is that money is simulation
//! state: it is spent through `try_spend`, saved by `save.rs` and read back
//! by every panel off one counter. A grant Godot kept to itself would mean a
//! balance the sim disagrees with and a save that forgets the money. So it
//! travels the real wire, and this file holds it to the same two rules every
//! other command obeys:
//!
//! 1. **What it does, it does exactly.** A grant moves the balance by its
//!    amount and by nothing else — no chunk dirtied, because money is not a
//!    tile, and nothing else in the world touched.
//! 2. **A refusal leaves the world byte-identical.** `amount <= 0` is
//!    refused, and `to_save_ron()` before and after must be the same string.
//!    There is no negative grant: taking money away would need a rule about
//!    what happens below zero, which `Economy` deliberately does not have.
//!
//! The JSON shape is asserted here too, through `command_from_json`, because
//! that string is what `godot_project/main.gd` actually sends and a typo in
//! the tag would otherwise only show up as a button that silently does
//! nothing.

use sim_core::sim::types::Money;
use sim_core::sim::{PlayerCommand, RecipeRegistry, STARTING_MONEY, World, command_from_json};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    World::new(recipes())
}

#[test]
fn a_grant_credits_exactly_its_amount() {
    let mut world = world();
    let before = world.money();
    assert_eq!(
        before,
        Money::from_num(STARTING_MONEY),
        "an empty world starts on STARTING_MONEY"
    );

    world.issue_command(PlayerCommand::DebugGrantMoney {
        amount: Money::from_num(10_000),
    });

    assert_eq!(
        world.money(),
        before + Money::from_num(10_000),
        "a grant moves the balance by its amount"
    );
}

#[test]
fn grants_accumulate_and_are_spendable_like_any_other_money() {
    let mut world = world();
    for _ in 0..3 {
        world.issue_command(PlayerCommand::DebugGrantMoney {
            amount: Money::from_num(1_000),
        });
    }
    assert_eq!(
        world.money(),
        Money::from_num(STARTING_MONEY) + Money::from_num(3_000),
        "three grants credit three times"
    );

    // Granted money is not marked or special-cased anywhere: it buys the same
    // things the starting balance does. Buying one of the cheapest kinds is
    // enough to prove it goes through the ordinary `try_spend` path.
    let balance_before_purchase = world.money();
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 1,
    });
    assert!(
        world.money() < balance_before_purchase,
        "a purchase spends out of the same counter the grant credited"
    );
}

#[test]
fn a_non_positive_grant_is_refused_and_changes_nothing() {
    for amount in [Money::ZERO, Money::from_num(-1), Money::from_num(-5_000)] {
        let mut world = world();
        let before = world.to_save_ron();
        let money_before = world.money();

        world.issue_command(PlayerCommand::DebugGrantMoney { amount });

        assert_eq!(
            world.money(),
            money_before,
            "a grant of {amount} must not move the balance"
        );
        assert_eq!(
            world.to_save_ron(),
            before,
            "a refused grant of {amount} must leave the world byte-identical"
        );
    }
}

#[test]
fn a_grant_dirties_no_chunk() {
    let mut world = world();
    assert!(
        world.dirty_chunks().is_empty(),
        "an empty world has nothing dirty to start with"
    );

    world.issue_command(PlayerCommand::DebugGrantMoney {
        amount: Money::from_num(500),
    });

    assert!(
        world.dirty_chunks().is_empty(),
        "money is not a tile — a grant must dirty no chunk, exactly as Purchase does"
    );
}

#[test]
fn an_absurd_grant_saturates_rather_than_wrapping() {
    let mut world = world();
    // What a fat-fingered twenty-digit entry in the debug menu would send.
    // `Money`'s 48 integer bits do not hold `i64::MAX`, so the DTO conversion
    // saturates; the balance must come out enormous and POSITIVE, never
    // wrapped round to a debt.
    let command = command_from_json(&format!(
        "{{\"type\":\"debug_grant_money\",\"amount\":{}}}",
        i64::MAX
    ))
    .expect("the wire shape must parse");
    world.issue_command(command);

    assert!(
        world.money() > Money::from_num(STARTING_MONEY),
        "a saturated grant must still be a credit"
    );
}

#[test]
fn the_wire_shape_is_what_godot_sends() {
    let parsed = command_from_json("{\"type\":\"debug_grant_money\",\"amount\":10000}")
        .expect("`debug_grant_money` must parse off the wire");
    match parsed {
        PlayerCommand::DebugGrantMoney { amount } => {
            assert_eq!(
                amount,
                Money::from_num(10_000),
                "the wire carries whole currency units"
            );
        }
        other => panic!("`debug_grant_money` parsed as {other:?}"),
    }

    // Missing amount is malformed, not "grant zero": `command_from_json`
    // returns `None` and `SimBridge` treats that as a no-op.
    assert!(
        command_from_json("{\"type\":\"debug_grant_money\"}").is_none(),
        "a grant with no amount is malformed, not a grant of nothing"
    );
}

#[test]
fn a_granted_balance_survives_save_and_load() {
    let mut world = world();
    world.issue_command(PlayerCommand::DebugGrantMoney {
        amount: Money::from_num(25_000),
    });
    let expected = world.money();

    let reloaded = World::from_save_ron(&world.to_save_ron(), recipes())
        .expect("a world with a granted balance must save and load");

    assert_eq!(
        reloaded.money(),
        expected,
        "the grant is ordinary money, so the save format already carries it"
    );
}
