//! Headless integration test for the contract board (the contract-board
//! epic, 2026-09-10; GAMEPLAY.md §6/§9/§11). `sim_core/src/sim/economy.rs`
//! unit-tests signing, strikes and loss against an inline fixture; this
//! file proves the same rules through the real `World` command path and the
//! shipped `data/contracts.ron`: a new game starts with no contract running,
//! `{"type": "sign_contract"}` buys every one out of the one money
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
fn a_new_game_starts_with_no_contract_and_offers_the_whole_board() {
    let world = world();
    let board = ContractRegistry::bundled();
    assert_eq!(
        board.starters().count(),
        0,
        "the shipped board signs nothing on a new player's behalf"
    );
    assert!(
        active_ids(&world).is_empty(),
        "a new game runs no contract until the player signs one"
    );
    assert!(
        world.economy_snapshot().contracts.is_empty(),
        "and the clipboard Godot draws from is empty too"
    );
    let snapshot = world.economy_snapshot();
    // EVERY offer, in file order — and the ones a progression gate holds
    // back (`data/unlocks.ron`) padlocked, naming that gate, rather than
    // left out. A board that hid them looked like the whole game.
    //
    // `gated` is read from the same unlock state the board was built with,
    // deliberately — the claim here is that the two agree, not that the
    // gates hold the right ids. WHICH ids a tier must hold back is pinned in
    // `tests/unlocks.rs` against the contracts file, because a list checked
    // against itself cannot notice an id going missing.
    let gated: Vec<&str> = board
        .ordered()
        .iter()
        .filter(|c| !world.contract_unlocked(&c.id))
        .map(|c| c.id.as_str())
        .collect();
    assert!(
        !gated.is_empty(),
        "the shipped content must gate something, or this test proves nothing"
    );
    let listed: Vec<&str> = snapshot.board.iter().map(|o| o.id.as_str()).collect();
    let every: Vec<&str> = board.ordered().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(listed, every, "the board lists every offer, in file order");
    for offer in &snapshot.board {
        let gate = world.economy().unlocks().gate_for_contract(&offer.id);
        if gated.contains(&offer.id.as_str()) {
            let gate = gate.expect("a gated offer has a gate");
            assert!(
                offer.locked,
                "'{}' is behind a closed gate and must read locked",
                offer.id
            );
            assert_eq!(
                offer.unlock_name, gate.name,
                "'{}' names its gate",
                offer.id
            );
            assert_eq!(offer.unlock_item, gate.after_sold.item);
            assert_eq!(offer.unlock_boxes, gate.after_sold.boxes);
            assert_eq!(offer.unlock_sold, 0, "a new game has sold nothing");
        } else {
            assert!(
                !offer.locked,
                "'{}' is open and must not read locked",
                offer.id
            );
            assert!(offer.unlock_name.is_empty() && offer.unlock_item.is_empty());
            assert_eq!((offer.unlock_boxes, offer.unlock_sold), (0, 0));
        }
    }
    assert!(
        snapshot.board.iter().all(|o| !o.active),
        "every offer on a new game's board is still for sale"
    );
    assert!(
        snapshot.board.iter().any(|o| o.affordable && !o.locked),
        "and the starting balance can sign at least one of them"
    );
}

/// **THE MUSTARD COOK AND THE MUSTARD MIXED ORDER ARE ONE DECISION**, and
/// this is the assertion that keeps them one. `data/recipes.ron` cooks
/// mustard for 360 ticks against ketchup's 240 so that a factory with one
/// finished line of each settles at three cases of ketchup to two of
/// mustard, and `deli_chain` asks for exactly that mix. Stated as ids and
/// numbers: the order asks for the same number of **vat-ticks** from each
/// line, 3 x 240 = 2 x 360 = 720, so the obvious two-line factory serves it
/// without tuning. Change either number and this fails.
///
/// `grill_pub`'s 3 + 2 is the same shape for a different reason and is
/// deliberately not checked here: BBQ's vat drinks the ketchup vat's output,
/// so its ratio comes from the draw rate rather than from two independent
/// cooks, and `tests/bbq_branch.rs` measures it on a running factory. Two
/// independent lines can be multiplied out on paper; a branch cannot.
#[test]
fn the_mustard_mixed_order_asks_for_the_same_vat_ticks_from_each_line() {
    let recipes = recipes();
    let ketchup_cook = recipes.get("ketchup").expect("the shipped cook").time_ticks;
    let mustard_cook = recipes.get("mustard").expect("the shipped cook").time_ticks;

    let board = ContractRegistry::bundled();
    let mixed = board
        .get("deli_chain")
        .expect("the mustard tier's mixed order");
    let cases = |id: &str| -> u64 {
        mixed
            .mix
            .iter()
            .find(|(item, _)| item == id)
            .map(|(_, count)| u64::from(*count))
            .unwrap_or_else(|| panic!("{} asks for no {id}", mixed.id))
    };
    // No literal (240, 360) above this: a cook change has to reach THIS
    // assertion and print this sentence, rather than trip a bare
    // `left == right` on the pair and say nothing about why it matters.
    assert_eq!(
        cases("boxed_ketchup") * ketchup_cook,
        cases("boxed_mustard") * mustard_cook,
        "{} asks for {} ketchup and {} mustard, which is not what a {}-tick vat and a \
         {}-tick vat make in the same window — the cook in data/recipes.ron and this mix \
         move together or not at all",
        mixed.id,
        cases("boxed_ketchup"),
        cases("boxed_mustard"),
        ketchup_cook,
        mustard_cook
    );
}

/// Every payout in the mustard tier is derived from what a case costs to
/// make, and the rate per case rises with what the order commits the player
/// to — the ordering the shipped board already has, asserted rather than
/// described. A case of mustard costs 17 in raws (`tests/cookbook.rs` walks
/// that back to the market), the board's margin rule puts ingredients at
/// roughly a third of revenue, and 44/46/52 is that band.
#[test]
fn the_mustard_tier_pays_by_the_case_and_pays_better_the_more_it_asks() {
    let board = ContractRegistry::bundled();
    let rate = |id: &str| -> f64 {
        let contract = board.get(id).unwrap_or_else(|| panic!("no contract {id}"));
        let cases: u32 = contract.mix.iter().map(|(_, count)| count).sum();
        contract.payout.to_num::<f64>() / f64::from(cases)
    };
    // 2 x 44 = 88, 3 x 46 = 138, 8 x 52 = 416.
    assert_eq!(rate("deli_counter"), 44.0);
    assert_eq!(rate("hot_dog_stand"), 46.0);
    assert_eq!(rate("sandwich_franchise"), 52.0);
    // 3 x 36 + 2 x 48 = 204 over five cases: a mixed order is priced per
    // side, so its blended rate sits below both, which is why the board's
    // header excludes it from the ladder.
    assert!((rate("deli_chain") - 40.8).abs() < 0.001);

    let mustard = board
        .get("deli_counter")
        .expect("the tier's trial order")
        .payout
        .to_num::<f64>()
        / 2.0;
    assert!(
        mustard > 17.0 * 2.0,
        "the worst rate in the tier still has to beat twice what the case cost"
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
    assert!(active_ids(&world).is_empty());
}

#[test]
fn a_contract_left_unserved_for_its_strikes_is_lost_and_goes_back_on_the_board() {
    let mut world = world();
    // The board's first offer, signed on tick 0 the way a new player's
    // first signature would be.
    let starter = ContractRegistry::bundled().ordered()[0].clone();
    assert!(
        starter.strikes >= 2,
        "the first offer on the board forgives at least one miss"
    );
    sign(&mut world, &starter.id);
    assert!(world.economy().is_active(&starter.id));

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
