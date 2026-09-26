//! Headless integration test for the "Shop & Inventory" epic's ticket 1:
//! the player can spend contract money on equipment, and the simulation
//! tracks what they own (GAMEPLAY.md §1's "money buys equipment," §4's
//! "equipment purchases *are* the tech tree").
//!
//! `sim_core/src/sim/shop.rs` already unit-tests the catalog loader and the
//! owned-count ledger in isolation (schema version, degenerate/duplicate
//! entries, cost arithmetic, snapshot construction), and `world.rs`'s own
//! unit tests cover the `Purchase` command against a directly-set money
//! balance. This file drives the real `World` instead, to prove the three
//! things only the whole system can show:
//!
//! 1. **The loop actually closes.** Money earned by delivering against a
//!    contract is the same money a purchase spends — one currency, per
//!    GAMEPLAY.md §1, not a shop with its own wallet.
//! 2. **A refused purchase is a true no-op.** An unaffordable or unknown
//!    kind leaves money *and* every owned count exactly as they were; there
//!    is no partially-applied transaction.
//! 3. **The wire shape.** The exact JSON field names ticket 2 (the Godot
//!    build menu) consumes verbatim, pinned the way `hopper.rs` and
//!    `fluids.rs` pin theirs — a rename here fails a test instead of
//!    silently breaking the UI.
//!
//! **What changed with the "Real Placement" epic:** an owned count is real
//! inventory now, not a ledger of past purchases — `Place` spends one and
//! `Remove` credits one back — and this file's worlds are built by
//! `sim::reference_chain`, i.e. by issuing exactly the `Purchase` commands
//! this file is about. Spending inventory on placement is covered in
//! `placement.rs`; what is asserted here is still the money half.

mod common;

use sim_core::sim::types::Money;
use sim_core::sim::{
    MachineRegistry, PlayerCommand, RecipeRegistry, ShopCatalog, World, command_from_json,
    linear_prefix, reference_chain, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    reference_chain(recipes())
}

/// The unit price of a kind, straight from the shipped catalog — so this
/// file asserts against `data/shop.ron`'s own numbers rather than hardcoding
/// prices a balancing pass is expected to change.
fn price_of(kind: &str) -> Money {
    ShopCatalog::bundled()
        .get(kind)
        .unwrap_or_else(|| panic!("data/shop.ron must price '{kind}'"))
        .price_money()
}

/// Runs the real chain until enough contract payouts have landed to fund
/// this file's purchases several times over, so the returned world has
/// genuine, earned money to spend. Mirrors `economy.rs`'s own drive loop —
/// which, since the delivery-dock ticket, only *feeds* the line: the dock at
/// the chain's tail takes finished boxes off its belt by itself every
/// tick, so no collect command is issued (there isn't one any more).
///
/// The threshold is deliberately a generous flat amount rather than "at
/// least one payout": `data/contracts.ron` now ships three contracts with
/// different payouts (24/50/90), and which one happens to settle first
/// depends on cadence/production timing — stopping at the very first payout
/// risked returning as little as 24, which this file's own
/// `repeated_purchases_of_several_kinds_accumulate_independently` test
/// (needing 36 to cover its purchases) is not guaranteed to cover.
///
/// Since the chain-rework epic the world is built with exactly
/// `common::STOCK_COST` on top of its shop bill and spends every unit of it
/// on raw ingredients before the loop starts — so the balance is still
/// **zero** when the run begins, and every unit of money it then reports is
/// genuinely earned from a contract rather than left over from the fixture.
///
/// **`MIN_MONEY` is 50, not 200 (performance note, added during a
/// codebase-wide perf audit run alongside this same rework).** This function
/// runs once per test in this file (7 of them), each driving the *whole*
/// reference chain — including the 240-tick cooking vat — tick by tick until
/// the threshold is crossed. At 200 that was 24,000 ticks per test, ~168,000
/// ticks total just to set up this file, and it was measurably the most
/// expensive part of `cargo test -p sim_core`. 200 was never load-bearing: it
/// was chosen as "several times the largest single-contract payout" for
/// comfortable headroom, not because any test needs that much. 50 keeps ~39%
/// of headroom over the 36 this file's hungriest test actually spends
/// (repeated purchases of 4 belts + 2 pipes) — comfortably more than one
/// contract's smallest payout (24) alone can satisfy, so the "not guaranteed
/// by a single payout" reasoning above still holds — while cutting the tick
/// count (and this file's wall time) by more than half.
fn world_with_earned_money() -> World {
    let mut world =
        reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE);
    common::stock_up(&mut world);
    // The fixture holds exactly its wage reserve (the staff pools MVP epic:
    // a broke factory loses its crew at the first payday), so "earned" is
    // the balance with that reserve taken out and the wages paid put back.
    assert_eq!(
        world.money(),
        Money::from_num(common::WAGE_RESERVE),
        "the fixture must start with nothing but its wage reserve, or 'earned money' \
         would not mean earned"
    );
    let earned = |world: &World| {
        world.money() + common::wages_paid(world) - Money::from_num(common::WAGE_RESERVE)
    };
    const MIN_MONEY: i64 = 50;

    for _ in 0..2_000_000u32 {
        common::supply(&mut world);
        world.tick();
        if earned(&world) >= Money::from_num(MIN_MONEY) {
            break;
        }
    }

    assert!(
        earned(&world) >= Money::from_num(MIN_MONEY),
        "the chain must earn at least {MIN_MONEY} money before this test can spend it \
         (got {})",
        world.money()
    );
    world
}

/// GAMEPLAY.md §1's loop, closed for the first time: the money a contract
/// delivery paid is the money a purchase spends.
#[test]
fn contract_earnings_buy_equipment_and_the_owned_count_goes_up() {
    let mut world = world_with_earned_money();
    let before = world.money();
    let price = price_of("belt");

    assert_eq!(
        world.owned_count("belt"),
        0,
        "a reference-chain world has placed everything it bought"
    );

    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 2,
    });

    assert_eq!(
        world.money(),
        before - price * Money::from_num(2),
        "money must fall by exactly quantity * unit price"
    );
    assert_eq!(world.owned_count("belt"), 2);
}

#[test]
fn repeated_purchases_of_several_kinds_accumulate_independently() {
    let mut world = world_with_earned_money();
    let before = world.money();

    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 3,
    });
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Purchase {
        kind: "pipe".to_string(),
        quantity: 2,
    });

    assert_eq!(world.owned_count("belt"), 4);
    assert_eq!(world.owned_count("pipe"), 2);
    assert_eq!(world.owned_count("crusher"), 0, "unbought kinds stay at 0");
    assert_eq!(
        world.money(),
        before - price_of("belt") * Money::from_num(4) - price_of("pipe") * Money::from_num(2),
        "every purchase debits the same single money counter"
    );
}

/// The failure mode this test exists to prevent is a *partially* applied
/// purchase — money gone, nothing owned, or the reverse.
#[test]
fn an_unaffordable_purchase_changes_neither_money_nor_inventory() {
    // A reference-chain world has spent its whole bill, and the cooking vat
    // is the most expensive kind in the catalog, so this can never succeed.
    let mut world = world();
    assert_eq!(world.money(), Money::ZERO);

    world.issue_command(PlayerCommand::Purchase {
        kind: "cooking_vat".to_string(),
        quantity: 1,
    });

    assert_eq!(world.money(), Money::ZERO, "no money may be spent");
    assert_eq!(world.owned_count("cooking_vat"), 0, "nothing may be owned");
}

/// One unit short is still short — the boundary case, since an off-by-one in
/// the affordability check is exactly what would let money go negative.
#[test]
fn a_purchase_one_unit_over_the_balance_is_refused_whole() {
    let mut world = world_with_earned_money();
    let money = world.money();
    let unit = price_of("belt");
    // The largest quantity that fits, plus one.
    let affordable_qty = (money / unit).int().to_num::<u32>();
    let too_many = affordable_qty + 1;

    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: too_many,
    });

    assert_eq!(
        world.money(),
        money,
        "an over-budget purchase spends nothing"
    );
    assert_eq!(world.owned_count("belt"), 0);

    // ...and the largest affordable quantity still goes through, so the
    // refusal above is a boundary, not a blanket block.
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: affordable_qty,
    });
    assert_eq!(world.owned_count("belt"), affordable_qty);
}

#[test]
fn an_unknown_kind_is_a_silent_no_op() {
    let mut world = world_with_earned_money();
    let before = world.money();

    world.issue_command(PlayerCommand::Purchase {
        kind: "fermenter".to_string(),
        quantity: 1,
    });

    assert_eq!(world.money(), before);
    assert_eq!(world.owned_count("fermenter"), 0);
    // And it must not have leaked into the catalog either.
    let snapshot = world.shop_snapshot();
    assert!(
        !snapshot.entries.iter().any(|e| e.kind == "fermenter"),
        "an unknown kind must never appear in the catalog"
    );
}

#[test]
fn a_zero_quantity_purchase_is_a_no_op() {
    let mut world = world_with_earned_money();
    let before = world.money();

    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 0,
    });

    assert_eq!(world.money(), before);
    assert_eq!(world.owned_count("belt"), 0);
}

/// The wire form ticket 2's build menu issues. `quantity` is optional and
/// defaults to one, matching `fill_hopper`'s optional `amount`.
#[test]
fn the_purchase_command_parses_from_json_with_and_without_a_quantity() {
    let mut world = world_with_earned_money();

    let explicit = command_from_json(r#"{"type": "purchase", "kind": "belt", "quantity": 3}"#)
        .expect("an explicit-quantity purchase must parse");
    world.issue_command(explicit);
    assert_eq!(world.owned_count("belt"), 3);

    let defaulted = command_from_json(r#"{"type": "purchase", "kind": "belt"}"#)
        .expect("an omitted quantity must default to 1, not fail to parse");
    world.issue_command(defaulted);
    assert_eq!(world.owned_count("belt"), 4);

    // `kind` has no sensible default, so a purchase without one is
    // malformed — and malformed commands are `None`, per
    // `command_from_json`'s contract.
    assert!(command_from_json(r#"{"type": "purchase"}"#).is_none());
}

/// Pins the exact JSON ticket 2 consumes. A field rename must fail here.
#[test]
fn shop_snapshot_json_has_the_documented_shape() {
    let mut world = world_with_earned_money();
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 2,
    });

    let value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.shop_snapshot()).unwrap()).unwrap();

    let object = value.as_object().expect("the snapshot is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["entries", "money"],
        "the snapshot has exactly these two top-level fields"
    );
    assert_eq!(
        value["money"].as_f64().expect("money is a number"),
        world.money().to_num::<f64>(),
        "the snapshot repeats the same money economy_snapshot_json reports"
    );

    let entries = value["entries"].as_array().expect("entries is an array");
    assert_eq!(
        entries.len(),
        ShopCatalog::bundled().entries().len(),
        "every priced kind is listed, owned or not"
    );

    for entry in entries {
        let mut fields: Vec<&str> = entry
            .as_object()
            .expect("each entry is an object")
            .keys()
            .map(|k| k.as_str())
            .collect();
        fields.sort();
        assert_eq!(
            fields,
            vec!["affordable", "crew", "kind", "owned", "price"],
            "each entry has exactly these five fields (`crew` since the staff pools MVP epic)"
        );
        assert!(
            entry["crew"].is_array(),
            "crew is the kind's staff list, empty for a tile"
        );
        assert!(entry["kind"].is_string());
        assert!(
            entry["price"].as_f64().unwrap() >= 1.0,
            "prices are positive"
        );
        assert!(entry["owned"].is_u64());
        assert!(entry["affordable"].is_boolean());
    }

    // Order is `data/shop.ron`'s own file order — the intended menu order,
    // so a UI can render it as it arrives.
    let listed: Vec<String> = entries
        .iter()
        .map(|e| e["kind"].as_str().unwrap().to_string())
        .collect();
    let expected: Vec<String> = ShopCatalog::bundled()
        .entries()
        .iter()
        .map(|e| e.id.clone())
        .collect();
    assert_eq!(listed, expected);

    // The purchase above is visible in the snapshot, and only for that kind.
    let belt = entries.iter().find(|e| e["kind"] == "belt").unwrap();
    assert_eq!(belt["owned"].as_u64().unwrap(), 2);
    assert!(
        entries
            .iter()
            .filter(|e| e["kind"] != "belt")
            .all(|e| e["owned"].as_u64().unwrap() == 0),
        "no other kind's count moved"
    );
}

/// The catalog covers everything a player could ever place: every machine
/// kind `data/machines.ron` defines, plus the two pieces of infrastructure
/// that are not machines. `shop.rs`'s own unit tests assert this too; it is
/// repeated here because it is the invariant most likely to be broken by a
/// *later* ticket adding a machine kind and forgetting the price.
#[test]
fn the_shipped_catalog_prices_every_placeable_kind() {
    let catalog = ShopCatalog::bundled();
    catalog
        .validate_covers(&MachineRegistry::bundled())
        .expect("every machine kind in data/machines.ron must be priced in data/shop.ron");
    for infra in ["belt", "pipe"] {
        assert!(catalog.get(infra).is_some(), "'{infra}' must be priced");
    }
}

/// The state a reference-chain world starts in, which is what every other
/// test in this file measures from: a whole factory standing, **nothing
/// owned and nothing in the bank**.
///
/// That combination is deliberate and is worth reading as a statement about
/// the fixture rather than about the game. `sim::reference_chain` prices its
/// own bill out of `data/shop.ron`, starts with exactly that much money and
/// spends every unit of it placing the line — so the wallet and the
/// unplaced-inventory ledger both land on zero, and a test measuring
/// earnings starts from a clean baseline. A *new game* is the opposite
/// shape: `sim::STARTING_MONEY` in the bank and an empty floor.
#[test]
fn a_reference_chain_world_owns_nothing_and_has_spent_everything() {
    let world = world();
    let snapshot = world.shop_snapshot();

    assert_eq!(snapshot.money, 0.0);
    assert!(
        snapshot.entries.iter().all(|e| e.owned == 0),
        "every purchased unit was placed, so nothing is left unplaced"
    );
    assert!(
        snapshot.entries.iter().all(|e| !e.affordable),
        "nothing is affordable at zero money"
    );
    // ...and the whole production chain is standing, built entirely out of
    // Purchase + Place commands.
    assert_eq!(
        world
            .world_layout()
            .iter()
            .filter(|p| p.recipe.is_some())
            .count(),
        linear_prefix(&recipes()).len(),
        "one placed machine per stage of the reference LINE — the ketchup line, not every \
         recipe in the file: the BBQ branch has its own fixture"
    );
}

/// The other side of that coin: a **fresh** world (what `World::new` builds,
/// and what a new game is) owns nothing, has `STARTING_MONEY`, and contains
/// nothing at all. This is the assertion the "Real Placement" epic exists to
/// make true — before it, a fresh world arrived with a complete factory in
/// it that no purchase had paid for.
#[test]
fn a_fresh_world_is_empty_and_holds_only_starting_money() {
    let world = World::new(recipes());
    assert_eq!(
        world.placement_count(),
        0,
        "a new game starts with a bare floor"
    );
    assert!(world.all_chunks().is_empty());
    assert_eq!(
        world.money(),
        Money::from_num(sim_core::sim::STARTING_MONEY),
        "and with exactly the documented starting balance"
    );
    let snapshot = world.shop_snapshot();
    assert!(snapshot.entries.iter().all(|e| e.owned == 0));
    // Every kind in the shipped catalog is within reach on day one — the
    // starting balance is not a filter on *what* can be built, only on how
    // much of it (see `starting_money_covers_the_reference_chain_with_room_to_spare`).
    let unaffordable: Vec<&str> = snapshot
        .entries
        .iter()
        .filter(|e| !e.affordable)
        .map(|e| e.kind.as_str())
        .collect();
    assert!(
        unaffordable.is_empty(),
        "a new game should be able to afford one of anything in the catalog: {unaffordable:?}"
    );
}

/// **The relationship `STARTING_MONEY` exists to hold**, and the reason it
/// stopped being 200.
///
/// Money only ever enters the world through a contract delivery, a delivery
/// needs a line that reaches the dock, and the shipped line is
/// `sim::reference_chain`. So if a new game cannot afford that line, the
/// core loop GAMEPLAY.md §1 describes cannot close from a cold start at all
/// — which is exactly what 200 against a 1087 bill meant. This test is the
/// check that keeps that from happening again silently.
///
/// It prices the chain **dynamically**, out of `data/shop.ron`, by summing
/// what every placement in a reference-chain world cost — so a balancing
/// pass that changes prices moves this test's expectation with it, and only
/// a change that breaks the *relationship* fails. The 2.5x margin is the
/// deliberate part: covering the bill exactly would make a first factory a
/// puzzle with one solution, and the surplus is what pays for building it
/// wrong once.
#[test]
fn starting_money_covers_the_reference_chain_with_room_to_spare() {
    let starting = Money::from_num(sim_core::sim::STARTING_MONEY);

    // The bill, summed off the shipped catalog rather than restated: one
    // placement is one purchased unit (a belt tile, a pipe tile, a machine,
    // a building), so this is the same arithmetic `reference_chain` does
    // before it builds anything.
    let bill = world()
        .world_layout()
        .iter()
        .fold(Money::ZERO, |acc, p| acc + price_of(&p.kind));

    assert!(
        starting > bill,
        "a new game must be able to build the shipped line at all: {starting} in the bank \
         against a {bill} bill"
    );
    assert!(
        starting > bill * Money::from_num(2.5),
        "and with real room left over, not just barely: {starting} against a {bill} bill"
    );
}

/// The literal, pinned once so a change to it is a deliberate edit to a test
/// rather than a number that drifts. Everything *about* the value is checked
/// as a relationship above; this only asserts what was actually chosen.
#[test]
fn starting_money_is_the_documented_value() {
    assert_eq!(sim_core::sim::STARTING_MONEY, 3000);
}
