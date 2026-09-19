//! Headless integration test for the ingredient economy (GAMEPLAY.md §5's
//! first cost axis: "ingredients — recurring, scales directly with
//! production volume").
//!
//! `sim_core/src/sim/ingredients.rs` already unit-tests the catalog loader,
//! the owned-stock ledger, the raw-ingredient derivation and the snapshot in
//! isolation. This file drives the real `World`, to prove the three things
//! only the whole system can show:
//!
//! 1. **There is still exactly one currency.** The money a contract delivery
//!    pays is the money an ingredient purchase spends — the same
//!    `Economy::money` a `Purchase` debits (GAMEPLAY.md §1). A refused
//!    ingredient purchase is a total no-op, exactly like a refused
//!    equipment purchase.
//! 2. **Nothing enters the factory that was not bought.** A fill draws from
//!    owned stock and is a silent no-op without it, so an unstocked world
//!    can produce nothing at all — which is the behaviour change this
//!    mechanic exists for.
//! 3. **Running the line costs money over time**, and the shipped prices
//!    leave a margin: a factory that is kept fed earns more than it spends.
//!
//! `hopper.rs` covers the hopper-side clamping in detail (owned stock first,
//! then the bin's own capacity, debited by what was actually taken); this
//! file is about the money and the wire.

mod common;

use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, FLUID_TANK_CAPACITY, IngredientCatalog, PlayerCommand, RecipeRegistry,
    bbq_branch, command_from_json, raw_ingredient_ids, reference_chain,
    reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn price_of(item: &str) -> Money {
    IngredientCatalog::bundled()
        .get(item)
        .unwrap_or_else(|| panic!("data/ingredients.ron must price '{item}'"))
        .price_money()
}

// ---------------------------------------------------------------------
// 1. One currency, and an all-or-nothing purchase.
// ---------------------------------------------------------------------

#[test]
fn buying_an_ingredient_debits_the_one_money_counter_and_credits_stock() {
    let mut world = reference_chain_with_budget(recipes(), 100);
    let before = world.money();

    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 20,
    });

    assert_eq!(
        world.money(),
        before - price_of("tomato") * Money::from_num(20),
        "money must fall by exactly quantity * unit price — the same counter a contract \
         payout credits and an equipment purchase debits"
    );
    assert_eq!(world.owned_ingredient("tomato"), 20);
    assert_eq!(world.owned_ingredient("sugar"), 0, "and nothing else moved");
}

/// Every refusal `BuyIngredient` documents, each checked to leave the world
/// byte-identical — the same contract `Purchase` has.
#[test]
fn a_refused_ingredient_purchase_is_a_total_no_op() {
    // A budget deliberately too small for what is asked for below.
    let mut world = reference_chain_with_budget(recipes(), 5);
    let money = world.money();

    for (item, quantity, why) in [
        ("tomato", 0u32, "a zero quantity buys nothing"),
        ("saffron", 4, "an ingredient no data file prices"),
        ("belt", 1, "a *buildable kind* is not an ingredient"),
        ("tomato", 500, "more than the balance covers"),
        ("tomato", u32::MAX, "a silly quantity, far past the balance"),
    ] {
        world.issue_command(PlayerCommand::BuyIngredient {
            item: item.to_string(),
            quantity,
        });
        assert_eq!(world.money(), money, "{why}: money must not move");
        assert_eq!(
            world.owned_ingredient(item),
            0,
            "{why}: no stock may be credited"
        );
    }

    // ...and a purchase that *is* affordable still works afterwards, so this
    // is a set of targeted refusals rather than a broken command.
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 5,
    });
    assert_eq!(world.owned_ingredient("tomato"), 5);
}

// ---------------------------------------------------------------------
// 2. Nothing enters the factory that was not bought.
// ---------------------------------------------------------------------

/// The behaviour change, stated at its bluntest: a fully-built factory with
/// an empty larder produces nothing, forever. Before this mechanic, the same
/// world ran indefinitely on free material.
#[test]
fn an_unstocked_factory_can_be_filled_with_nothing_and_produces_nothing() {
    let mut world = reference_chain(recipes());
    assert_eq!(
        world.money(),
        Money::ZERO,
        "the fixture spent its whole bill"
    );

    for _ in 0..2_000 {
        world.issue_command(PlayerCommand::FillHopper {
            amount: 64,
            tile: None,
        });
        world.issue_command(PlayerCommand::FillFluidTank {
            amount: FLUID_TANK_CAPACITY,
            tile: None,
        });
        world.tick();
    }

    for hopper in world.hopper_snapshots() {
        assert_eq!(
            hopper.quantity, 0,
            "a hopper cannot be filled from an empty larder"
        );
    }
    for tank in world.fluid_tank_snapshots() {
        assert_eq!(tank.quantity, 0.0, "nor can a tank");
    }
    assert_eq!(world.dock_snapshots()[0].shipped_total, 0);
}

/// A tank holds no fluid id of its own, so what it is being filled with —
/// and therefore what the player is charged for — is the fluid of the run it
/// discharges into.
#[test]
fn filling_a_tank_charges_for_the_fluid_of_the_run_it_feeds() {
    let mut world = reference_chain_with_budget(recipes(), 200);
    common::buy(&mut world, "vinegar", 40);
    common::buy(&mut world, "tomato", 10);

    world.issue_command(PlayerCommand::FillFluidTank {
        amount: 25,
        tile: None,
    });

    assert_eq!(
        world.owned_ingredient("vinegar"),
        15,
        "the tank's own run carries vinegar, so vinegar is what was drawn"
    );
    assert_eq!(
        world.owned_ingredient("tomato"),
        10,
        "and no other stock was touched"
    );
    assert_eq!(world.fluid_tank_snapshots()[0].quantity, 25.0);
}

// ---------------------------------------------------------------------
// 3. Running the line costs money, and still pays.
// ---------------------------------------------------------------------

/// GAMEPLAY.md §5's actual claim, stated at the level it holds at: **a unit
/// of product is worth more than the ingredients that went into it**, on
/// every shipped contract.
///
/// Deliberately *not* "a running factory earns more than it spends over N
/// ticks". That is false, and correctly so: the shipped contract board asks
/// for a handful of boxes per cadence window, a kept-fed line makes far more
/// than that, and every box nobody has ordered is ingredients turned into
/// stock in a pool no contract wants. Overproduction being a real loss is
/// the design pressure GAMEPLAY.md §6 chooses contracts over a spot market
/// to create — so the invariant worth pinning is the per-unit margin, not
/// the balance of an arbitrary run.
///
/// Derived from the data files rather than written as literals, so a
/// balancing pass moves the numbers and this test moves with them; what must
/// not change is the sign.
#[test]
fn a_delivered_unit_is_worth_more_than_the_ingredients_that_made_it() {
    let registry = recipes();
    let final_item = registry
        .ordered()
        .last()
        .expect("the chain has a last stage")
        .outputs[0]
        .0
        .clone();

    let ingredient_cost: Money = raw_ingredient_ids(&registry)
        .iter()
        .map(|raw| Money::from_num(raw_content(&final_item, raw, &registry)) * price_of(raw))
        .sum();
    assert!(
        ingredient_cost > Money::ZERO,
        "the final product must embody some bought ingredient"
    );

    let board = ContractRegistry::bundled();
    for contract in board.ordered() {
        let units: u32 = contract.mix.iter().map(|(_, qty)| *qty).sum();
        let per_unit = contract.payout / Money::from_num(units);
        assert!(
            per_unit > ingredient_cost,
            "contract '{}' pays {per_unit} per unit but a unit costs {ingredient_cost} in \
             ingredients — at that price the factory loses money by running",
            contract.id
        );
    }
}

/// How many units of `raw` one unit of `id` embodies — the same walk
/// `core_loop.rs` uses for its conservation accounting, following fluid
/// production as readily as solid.
fn raw_content(id: &str, raw: &str, recipes: &RecipeRegistry) -> u32 {
    if id == raw {
        return 1;
    }
    let Some(recipe) = recipes.ordered().iter().find(|r| {
        r.outputs.iter().any(|(out, _)| out == id)
            || r.fluid_outputs.iter().any(|(out, _)| out == id)
    }) else {
        return 0;
    };
    recipe
        .inputs
        .iter()
        .chain(recipe.fluid_inputs.iter())
        .map(|(input, qty)| qty * raw_content(input, raw, recipes))
        .sum()
}

/// ...and the other half of the same mechanic, observed on the real line:
/// running it genuinely *consumes* bought stock and genuinely *earns*
/// contract money. Neither number is compared to the other (see above) —
/// what is asserted is that both are non-zero, i.e. that the recurring cost
/// axis is real rather than nominal.
#[test]
fn running_the_line_both_consumes_bought_stock_and_earns_contract_money() {
    // Stock plus a wage reserve: 30 000 ticks is thirty paydays, and a broke
    // factory loses its crew at the first one (the staff pools MVP epic).
    let mut world =
        reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE);
    common::stock_up(&mut world);
    assert_eq!(
        world.money(),
        Money::from_num(common::WAGE_RESERVE),
        "it spent the whole budget on stock, bar the wage reserve"
    );
    let stocked = [
        world.owned_ingredient("tomato"),
        world.owned_ingredient("sugar"),
        world.owned_ingredient("vinegar"),
    ];

    for _ in 0..30_000 {
        common::supply(&mut world);
        world.tick();
    }

    for (i, item) in ["tomato", "sugar", "vinegar"].iter().enumerate() {
        assert!(
            world.owned_ingredient(item) < stocked[i],
            "running the line must eat into the {item} the player bought"
        );
    }
    assert!(
        world.money() + common::wages_paid(&world) > Money::from_num(common::WAGE_RESERVE),
        "...and contract deliveries must pay for it"
    );
}

// ---------------------------------------------------------------------
// 4. Content and wire.
// ---------------------------------------------------------------------

/// The cross-file check `IngredientCatalog::validate_covers` makes, run
/// against the world's own view of what is raw — so adding a raw ingredient
/// to `data/recipes.ron` without pricing it fails here.
#[test]
fn every_raw_ingredient_the_shipped_chain_needs_is_priced_and_buyable() {
    let registry = recipes();
    let raw = raw_ingredient_ids(&registry);
    assert_eq!(
        raw,
        vec![
            "tomato".to_string(),
            "sugar".to_string(),
            "vinegar".to_string(),
            "spice_mix".to_string(),
            "molasses".to_string(),
            "liquid_smoke".to_string(),
        ]
    );

    // Every raw is *priced*. Whether it is on sale yet is the progression
    // gate's business (`data/unlocks.ron`), so this runs in a world that has
    // earned every tier — `bbq_branch` states that premise — and proves the
    // stronger thing: once nothing is held back, every raw can be bought.
    // `tests/unlocks.rs` owns the other half, that a gated raw is refused.
    let mut world = bbq_branch(recipes(), 2_000);
    for item in &raw {
        assert!(
            world.ingredient_unlocked(item),
            "'{item}' must be on sale in a world that has earned every gate"
        );
        let before = world.owned_ingredient(item);
        world.issue_command(PlayerCommand::BuyIngredient {
            item: item.clone(),
            quantity: 1,
        });
        assert_eq!(
            world.owned_ingredient(item),
            before + 1,
            "'{item}' is raw, so it has to be buyable — otherwise the hopper or tank \
             holding it can never be filled"
        );
    }

    // Only the *solid* raws are hopper contents; vinegar and liquid smoke go
    // in tanks.
    assert_eq!(
        world.raw_items(),
        [
            "tomato".to_string(),
            "sugar".to_string(),
            "spice_mix".to_string(),
            "molasses".to_string()
        ]
    );
}

/// Pins the JSON wire shape the presentation ticket consumes — deliberately
/// the same shape as `shop_snapshot_json`, so one buy panel can drive both.
#[test]
fn the_ingredients_snapshot_json_has_the_documented_shape() {
    let mut world = reference_chain_with_budget(recipes(), 100);
    common::buy(&mut world, "tomato", 12);

    let snapshot: serde_json::Value =
        serde_json::to_value(world.ingredients_snapshot()).expect("must serialize");
    assert_eq!(snapshot["money"], serde_json::json!(88.0));
    let entries = snapshot["entries"].as_array().expect("an entries array");
    assert_eq!(
        entries.len(),
        3,
        "every priced ingredient a gate is not holding back is listed, including \
         ones the player has none of"
    );
    for held_back in ["spice_mix", "molasses", "liquid_smoke"] {
        assert!(
            !entries.iter().any(|e| e["id"] == held_back),
            "'{held_back}' is behind a closed gate and must not be on the market"
        );
    }
    assert_eq!(
        entries[0],
        serde_json::json!({
            "id": "tomato",
            "price": 1.0,
            "owned": 12,
            "affordable": true,
        }),
        "entries are in data/ingredients.ron's own order"
    );
    assert_eq!(entries[1]["id"], "sugar");
    assert_eq!(entries[1]["owned"], 0);
    assert_eq!(entries[2]["id"], "vinegar");
}

#[test]
fn the_buy_ingredient_command_parses_from_json_with_and_without_a_quantity() {
    let mut world = reference_chain_with_budget(recipes(), 100);

    world.issue_command(
        command_from_json(r#"{"type": "buy_ingredient", "item": "sugar", "quantity": 4}"#)
            .expect("must parse"),
    );
    assert_eq!(world.owned_ingredient("sugar"), 4);

    // Omitting `quantity` buys one, exactly as `purchase`'s omitted quantity
    // does.
    world.issue_command(
        command_from_json(r#"{"type": "buy_ingredient", "item": "sugar"}"#).expect("must parse"),
    );
    assert_eq!(world.owned_ingredient("sugar"), 5);

    // A malformed command is a no-op, not a panic — `command_from_json`'s
    // documented contract.
    assert!(command_from_json(r#"{"type": "buy_ingredient"}"#).is_none());
}

/// A `World` type-level guard: stock is a `u32` ledger and money is `Money`, and
/// the two are never conflated — buying does not change what is *placed*,
/// and placing does not change stock.
#[test]
fn ingredient_stock_and_equipment_inventory_are_separate_ledgers() {
    let mut world = reference_chain_with_budget(recipes(), 200);
    let placements = world.placement_count();

    common::buy(&mut world, "tomato", 10);
    assert_eq!(
        world.owned_count("tomato"),
        0,
        "an item id is not a buildable kind"
    );
    assert_eq!(world.placement_count(), placements, "buying places nothing");

    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 1,
    });
    assert_eq!(world.owned_count("belt"), 1);
    assert_eq!(
        world.owned_ingredient("belt"),
        0,
        "a buildable kind is not an ingredient"
    );
    assert_eq!(
        world.owned_ingredient("tomato"),
        10,
        "and stock is untouched"
    );
}
