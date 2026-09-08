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
//! 1. **The loop actually closes.** Money earned by delivering against the
//!    hardcoded contract is the same money a purchase spends — one currency,
//!    per GAMEPLAY.md §1, not a shop with its own wallet.
//! 2. **A refused purchase is a true no-op.** An unaffordable or unknown
//!    kind leaves money *and* every owned count exactly as they were; there
//!    is no partially-applied transaction.
//! 3. **The wire shape.** The exact JSON field names ticket 2 (the Godot
//!    build menu) consumes verbatim, pinned the way `hopper.rs` and
//!    `fluids.rs` pin theirs — a rename here fails a test instead of
//!    silently breaking the UI.
//!
//! Note what is deliberately *not* asserted anywhere: that buying something
//! changes the factory floor. Nothing consumes an owned count yet — the
//! shipped chain is still built unconditionally at tick 0 — and wiring
//! placement to spend this inventory is a later epic. See `sim::shop`'s
//! module doc.

use sim_core::sim::types::Fx;
use sim_core::sim::{
    FLUID_TANK_CAPACITY, MachineRegistry, PlayerCommand, RecipeRegistry, ShopCatalog, World,
    command_from_json,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    World::new(recipes())
}

/// The unit price of a kind, straight from the shipped catalog — so this
/// file asserts against `data/shop.ron`'s own numbers rather than hardcoding
/// prices a balancing pass is expected to change.
fn price_of(kind: &str) -> Fx {
    ShopCatalog::bundled()
        .get(kind)
        .unwrap_or_else(|| panic!("data/shop.ron must price '{kind}'"))
        .price_money()
}

/// How many units of `item` the final stage's machine currently holds in its
/// output buffer (the same helper `economy.rs` uses).
fn final_output_count(world: &World, item: &str) -> u32 {
    for chunk in world.all_chunks() {
        for machine in &world.chunk_snapshot(chunk).machines {
            if machine.recipe == item {
                return machine
                    .output
                    .iter()
                    .find(|(id, _)| id == item)
                    .map(|(_, qty)| *qty)
                    .unwrap_or(0);
            }
        }
    }
    0
}

/// Runs the real chain until the hardcoded contract has paid out at least
/// once, so the returned world has genuine, earned money to spend. Mirrors
/// `economy.rs`'s own drive loop.
fn world_with_earned_money() -> World {
    let registry = recipes();
    let final_item = registry.ordered().last().unwrap().id.clone();
    let mut world = world();

    for _ in 0..500_000u32 {
        world.issue_command(PlayerCommand::FillHopper { amount: 32 });
        world.issue_command(PlayerCommand::FillFluidTank {
            amount: FLUID_TANK_CAPACITY,
        });
        world.tick();
        if final_output_count(&world, &final_item) > 0 {
            world.issue_command(PlayerCommand::CollectOutput);
        }
        if world.money() > Fx::ZERO {
            break;
        }
    }

    assert!(
        world.money() > Fx::ZERO,
        "the chain must earn at least one contract payout before this test can spend it \
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

    assert_eq!(world.owned_count("belt"), 0, "a fresh world owns nothing");

    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 2,
    });

    assert_eq!(
        world.money(),
        before - price * Fx::from_num(2),
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
        before - price_of("belt") * Fx::from_num(4) - price_of("pipe") * Fx::from_num(2),
        "every purchase debits the same single money counter"
    );
}

/// The failure mode this test exists to prevent is a *partially* applied
/// purchase — money gone, nothing owned, or the reverse.
#[test]
fn an_unaffordable_purchase_changes_neither_money_nor_inventory() {
    // A fresh world has zero money and the cooking vat is the most
    // expensive kind in the catalog, so this can never succeed.
    let mut world = world();
    assert_eq!(world.money(), Fx::ZERO);

    world.issue_command(PlayerCommand::Purchase {
        kind: "cooking_vat".to_string(),
        quantity: 1,
    });

    assert_eq!(world.money(), Fx::ZERO, "no money may be spent");
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
            vec!["affordable", "kind", "owned", "price"],
            "each entry has exactly these four fields"
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

/// This ticket is purely additive: it must not change how the chain is
/// built, only add a parallel system beside it.
#[test]
fn a_fresh_world_owns_nothing_and_the_chain_exists_anyway() {
    let world = world();
    let snapshot = world.shop_snapshot();

    assert_eq!(snapshot.money, 0.0);
    assert!(
        snapshot.entries.iter().all(|e| e.owned == 0),
        "nothing is pre-owned"
    );
    assert!(
        snapshot.entries.iter().all(|e| !e.affordable),
        "nothing is affordable at zero money"
    );
    // ...and yet the whole production chain is already standing, because
    // nothing consumes an owned count yet.
    assert_eq!(
        world.chain_layout().len(),
        recipes().ordered().len(),
        "the shipped chain is still built unconditionally at tick 0"
    );
}
