//! Headless integration test for `World::spend_preview` — the query that
//! answers "would this spend starve the next payday?" before the money
//! moves (GAMEPLAY.md §7, the 2026-09-26 record).
//!
//! The rule under test is one sentence, and it is the sim's: **after this
//! spend, would the balance still cover the next payday's bill?** For a
//! purchase, an ingredient buy or a contract fee that is
//! `money - cost < next_bill`; for a hire, which debits nothing today but
//! raises the bill, it is `money < next_bill + wage`.
//!
//! Three things only a whole-`World` test can show, and they are why this
//! file exists beside `staff.rs`'s unit coverage of the arithmetic:
//!
//! 1. **The preview quotes the price the command will actually charge.** It
//!    calls `Shop::cost_of`, `Ingredients::cost_of`, the board offer's own
//!    `price` and the catalog's wage — the same functions `issue_command`'s
//!    arms call — so every case here is asserted by *issuing the command*
//!    and comparing the real balance afterwards. A second copy of a price
//!    rule inside `spend_preview` would pass a test that only read its
//!    fields and fail this one.
//! 2. **It mutates nothing.** `revision()` does not move and the save comes
//!    out byte-identical, so presentation may call it on every keypress and
//!    a preview can never change what the command after it does.
//! 3. **Every command answers.** The ones this feature does not gate say
//!    `spends: false` rather than erroring, because Godot funnels *all* its
//!    spends through one call.

mod common;

use sim_core::sim::types::Money;
use sim_core::sim::{
    Direction, Ingredients, PlayerCommand, RecipeRegistry, ShopCatalog, StaffCatalog, World,
    command_from_json, reference_chain, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A world with the shipped chain standing and a chosen balance. Money is
/// granted rather than written into a save, so nothing here needs the save
/// format to stay still.
fn world_with(money: u32) -> World {
    let mut world = reference_chain(recipes());
    world.issue_command(PlayerCommand::DebugGrantMoney {
        amount: Money::from_num(money) - world.money(),
    });
    assert_eq!(world.money(), Money::from_num(money));
    world
}

/// The shipped price of a machine kind, from `data/shop.ron` itself — a
/// balancing pass is expected to move these numbers and must not fail this
/// file for it.
fn price_of(kind: &str) -> Money {
    ShopCatalog::bundled()
        .get(kind)
        .unwrap_or_else(|| panic!("data/shop.ron must price '{kind}'"))
        .price_money()
}

fn wage_of(category: &str) -> u32 {
    StaffCatalog::bundled()
        .get(category)
        .unwrap_or_else(|| panic!("data/staff.ron must define '{category}'"))
        .wage
}

/// The id of a board offer the fixture has not signed — `corner_shop` is a
/// `starter` and active from tick 0, so it is never the answer.
fn unsigned_offer() -> String {
    reference_chain(recipes())
        .economy_snapshot()
        .board
        .iter()
        .find(|o| !o.active)
        .expect("data/contracts.ron ships more than its starters")
        .id
        .clone()
}

fn purchase(kind: &str, quantity: u32) -> PlayerCommand {
    PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity,
    }
}

// ---------------------------------------------------------------------
// The rule
// ---------------------------------------------------------------------

/// A purchase that leaves the balance above the next bill does not warn; the
/// same purchase one unit of money lower does. The boundary is exact — the
/// bill itself is affordable, a penny under it is not — and both sides are
/// proved by issuing the command and reading the balance the sim ends with.
#[test]
fn a_purchase_warns_exactly_when_it_would_leave_the_next_payday_short() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    assert!(bill > 0, "the shipped chain hires a crew");
    let cost = price_of("belt");

    // Comfortable: the bill survives the purchase with room to spare.
    let mut rich = world_with(bill + cost.to_num::<u32>() + 1);
    let preview = rich.spend_preview(Some(&purchase("belt", 1)));
    assert!(!preview.warn, "the bill is still covered after this belt");
    assert!(preview.spends);
    assert_eq!(preview.subject, "belt");
    assert_eq!(preview.cost, cost.to_num::<f64>());
    assert_eq!(preview.shortfall_after, 0);
    assert!(!preview.crosses);
    rich.issue_command(purchase("belt", 1));
    assert_eq!(
        rich.money(),
        Money::from_num(bill + 1),
        "the preview's cost is the cost the command charged"
    );
    assert_eq!(
        rich.staff().shortfall(rich.money()),
        0,
        "and the payday it promised would be covered, is"
    );

    // One unit of money poorer: the same purchase eats into the bill.
    let mut tight = world_with(bill + cost.to_num::<u32>() - 1);
    let preview = tight.spend_preview(Some(&purchase("belt", 1)));
    assert!(preview.warn, "this belt takes the balance under the bill");
    assert!(
        preview.crosses,
        "covered before, short after: this purchase is the cause"
    );
    assert_eq!(preview.shortfall_now, 0);
    assert_eq!(preview.shortfall_after, 1);
    assert_eq!(preview.next_bill, bill);
    assert_eq!(
        preview.bill_after, bill,
        "a purchase does not move the bill"
    );
    assert_eq!(preview.money_after, f64::from(bill - 1));
    tight.issue_command(purchase("belt", 1));
    assert_eq!(
        tight.staff().shortfall(tight.money()),
        preview.shortfall_after,
        "the gap the preview quoted is the gap the world is left with"
    );
}

/// Warned while already short, on every purchase — not only on the one that
/// crossed the line (the owner's call, 2026-09-26). `crosses` is what tells
/// the two apart, so the panel can word them differently; `warn` is true for
/// both, and a purchase the balance cannot even afford still previews
/// honestly rather than claiming it is free.
#[test]
fn a_purchase_made_while_already_short_warns_without_crossing() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let world = world_with(bill / 2);
    assert!(world.staff().shortfall(world.money()) > 0, "already short");

    let preview = world.spend_preview(Some(&purchase("belt", 1)));
    assert!(preview.warn, "spending while short warns too");
    assert!(
        !preview.crosses,
        "this purchase is not what caused it — the panel says so"
    );
    assert!(preview.shortfall_now > 0);
    assert!(
        preview.shortfall_after > preview.shortfall_now,
        "it does make the gap worse"
    );

    // Unaffordable, so `issue_command` would refuse it — which is why it does
    // not warn (see `a_spend_the_balance_cannot_cover_is_refused_rather_than_
    // warned_about`). The preview still QUOTES it honestly: `cost` is what the
    // command would charge, not zeroed because this balance cannot meet it.
    let broke = world_with(0);
    let preview = broke.spend_preview(Some(&purchase("cooking_vat", 1)));
    assert_eq!(preview.cost, price_of("cooking_vat").to_num::<f64>());
    assert_eq!(preview.money_after, 0.0, "the balance saturates at zero");
    assert!(!preview.affordable);
    assert!(!preview.warn, "a purchase that cannot happen asks nothing");
}

/// Quantity is priced, not assumed: a bulk purchase warns where a single one
/// does not, and the preview's cost is `quantity × price` because it asks
/// `Shop::cost_of` the same question the command does.
#[test]
fn quantity_is_priced_through_the_shops_own_arithmetic() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let unit = price_of("belt").to_num::<u32>();
    let mut world = world_with(bill + 4 * unit);

    let one = world.spend_preview(Some(&purchase("belt", 1)));
    assert!(
        !one.warn,
        "one belt leaves three belts' worth over the bill"
    );
    let five = world.spend_preview(Some(&purchase("belt", 5)));
    assert_eq!(five.cost, f64::from(5 * unit));
    assert!(five.warn, "five do not");
    assert!(five.crosses);

    // A zero quantity is a refusal in the shop, so it costs nothing and
    // warns about nothing.
    let none = world.spend_preview(Some(&purchase("belt", 0)));
    assert!(!none.spends);
    assert_eq!(none.cost, 0.0);
    assert!(!none.warn);

    world.issue_command(purchase("belt", 5));
    assert_eq!(world.money(), Money::from_num(bill) - Money::from_num(unit));
}

/// An ingredient buy is priced through the same clamp the command applies:
/// asking for more than the stock room holds buys — and quotes — only what
/// will fit. A preview of the asked-for quantity would warn about money the
/// player was never going to spend.
#[test]
fn an_ingredient_buy_is_priced_after_the_stock_room_clamp() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let mut world = world_with(bill * 4);
    let room = world.stock_room_for("tomato");
    assert!(room > 0, "the shipped chain has room for tomatoes");

    let asked = room + 1_000;
    let preview = world.spend_preview(Some(&PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: asked,
    }));
    let clamped = Ingredients::bundled()
        .cost_of("tomato", room)
        .expect("the shipped catalog prices tomatoes");
    assert_eq!(
        preview.cost,
        clamped.to_num::<f64>(),
        "priced for what fits, not for what was asked"
    );
    assert_eq!(preview.subject, "tomato");

    let before = world.money();
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: asked,
    });
    assert_eq!(
        before - world.money(),
        clamped,
        "and the command charged exactly that"
    );
}

/// A contract fee is a spend like any other (GAMEPLAY.md §5), so signing one
/// that empties the wage account warns. An offer that is already signed
/// cannot be signed again, so it costs nothing and says so.
#[test]
fn a_contract_fee_warns_and_an_already_signed_offer_costs_nothing() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let sign = |id: &str| PlayerCommand::SignContract { id: id.to_string() };
    // An offer nothing has signed yet. `corner_shop` is a `starter`, active
    // from tick 0, so it is the wrong one to price — and the right one to
    // prove the already-signed case with.
    let offer = reference_chain(recipes())
        .economy_snapshot()
        .board
        .iter()
        .find(|o| !o.active)
        .expect("data/contracts.ron ships more than its starters")
        .clone();
    assert!(offer.price > 0.0, "a contract charges a fee to sign");

    let mut world = world_with(bill + offer.price as u32 - 1);
    let preview = world.spend_preview(Some(&sign(&offer.id)));
    assert!(preview.warn, "the fee takes the balance under the bill");
    assert_eq!(preview.cost, offer.price);
    assert_eq!(preview.subject, offer.id);

    world.issue_command(sign(&offer.id));
    assert!(
        world
            .economy_snapshot()
            .board
            .iter()
            .any(|o| o.id == offer.id && o.active),
        "the signature went through — a warning is not a refusal"
    );
    let again = world.spend_preview(Some(&sign(&offer.id)));
    assert!(
        !again.spends,
        "one signature per offer: a second costs nothing"
    );
    assert!(!again.warn);

    let unknown = world.spend_preview(Some(&sign("no_such_contract")));
    assert!(!unknown.spends);
    assert_eq!(unknown.cost, 0.0);
}

/// The hire is the odd one out, and the reason this query is command-shaped
/// rather than a per-slot boolean: it debits **nothing** today and raises
/// what the next payday charges. So `cost` is zero, `money_after` is
/// unchanged, and `bill_after` is what moved — and the warning still fires.
#[test]
fn a_hire_moves_the_bill_instead_of_the_balance() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let wage = wage_of("manufacturing");
    let hire = PlayerCommand::Hire {
        category: "manufacturing".to_string(),
    };

    // Exactly on the line: the bill plus one more wage is affordable.
    let world = world_with(bill + wage);
    let preview = world.spend_preview(Some(&hire));
    assert!(preview.spends, "it costs nothing now and everything later");
    assert_eq!(preview.cost, 0.0);
    assert_eq!(preview.money_after, preview.money_now);
    assert_eq!(preview.bill_after, bill + wage);
    assert!(!preview.warn, "the raised bill is still covered");
    assert_eq!(preview.shortfall_after, 0);

    // One unit under it: the head can be hired, and cannot be paid.
    let mut tight = world_with(bill + wage - 1);
    let preview = tight.spend_preview(Some(&hire));
    assert!(preview.warn);
    assert!(
        preview.crosses,
        "the balance covers today's bill, not the new one"
    );
    assert_eq!(preview.shortfall_now, 0);
    assert_eq!(preview.shortfall_after, 1);
    assert_eq!(
        preview.cost, 0.0,
        "no money moves — which is exactly why a purchase-shaped check misses this"
    );

    let money = tight.money();
    tight.issue_command(hire);
    assert_eq!(tight.money(), money, "the hire charged nothing");
    assert_eq!(
        tight.staff_snapshot().next_bill,
        preview.bill_after,
        "the bill the preview promised is the bill the payday will charge"
    );
    assert_eq!(
        tight.staff().shortfall(tight.money()),
        preview.shortfall_after
    );

    // An unknown category is refused by `hire`, so it spends nothing and
    // moves no bill — the same answer a `Place` gets. (`warn` is left alone
    // here: this world is now genuinely short, and a panel that appeared for
    // a command which cannot happen would be the bug, which is what `spends`
    // is asserted for.)
    let unknown = tight.spend_preview(Some(&PlayerCommand::Hire {
        category: "astronauts".to_string(),
    }));
    assert!(!unknown.spends, "there is nobody to hire");
    assert_eq!(unknown.bill_after, unknown.next_bill);
    let solvent = world_with(bill * 10);
    assert!(
        !solvent
            .spend_preview(Some(&PlayerCommand::Hire {
                category: "astronauts".to_string(),
            }))
            .warn,
        "and it never warns on its own account"
    );
}

/// A factory with no crew has no payday to starve, so nothing warns however
/// broke it is. The feature is about wages, not about affordability — the
/// shop's own refusal already covers that.
#[test]
fn nothing_warns_while_nobody_is_on_the_payroll() {
    let mut world = reference_chain_with_budget(recipes(), 100_000);
    for category in StaffCatalog::bundled().category_ids() {
        for _ in 0..world.staff().headcount(&category) {
            world.issue_command(PlayerCommand::Fire {
                category: category.to_string(),
            });
        }
    }
    // Firing leaves the period's notice pay owed, so run past the payday
    // that settles it: only then is the next bill truly zero.
    let period = world.staff_snapshot().pay_period_ticks;
    while world.current_tick() <= period {
        world.tick();
    }
    assert_eq!(world.staff_snapshot().next_bill, 0, "nobody is owed");

    let preview = world.spend_preview(Some(&purchase("belt", 1)));
    assert!(preview.spends);
    assert!(!preview.warn, "no bill, nothing to starve");
    assert_eq!(preview.shortfall_after, 0);
}

// ---------------------------------------------------------------------
// The contract the caller relies on
// ---------------------------------------------------------------------

/// Every command answers, including the ones this feature does not gate, so
/// Godot can funnel every spend site through one call without a table of
/// which types are previewable. An unparseable string is the same answer —
/// that path is `SimBridge::spend_preview_json`'s, exercised here through
/// `None`.
#[test]
fn a_command_that_costs_nothing_answers_rather_than_erroring() {
    let world = world_with(0);
    let free = [
        PlayerCommand::Place {
            kind: "belt".to_string(),
            tile: (40, 40),
            direction: Direction::East,
            item: None,
        },
        PlayerCommand::Remove { tile: (0, 0) },
        PlayerCommand::Rotate { tile: (0, 0) },
        PlayerCommand::Fire {
            category: "manufacturing".to_string(),
        },
        PlayerCommand::SetMothballed {
            tile: (0, 0),
            enabled: true,
        },
    ];
    for cmd in &free {
        let preview = world.spend_preview(Some(cmd));
        assert!(!preview.spends, "{cmd:?} moves no money");
        assert!(!preview.warn, "{cmd:?} must not warn");
        assert_eq!(preview.cost, 0.0);
        assert_eq!(preview.subject, "");
        assert_eq!(preview.money_after, preview.money_now);
        assert_eq!(preview.bill_after, preview.next_bill);
    }
    let nothing = world.spend_preview(None);
    assert!(!nothing.spends, "an unparseable command spends nothing");
    assert!(!nothing.warn);
}

/// **A preview mutates nothing.** Not the revision Godot's per-frame poll is
/// gated on, and not one byte of the save — so it is safe on every keypress,
/// and it cannot change what the command issued after it does.
#[test]
fn a_preview_leaves_the_world_byte_identical() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let world = world_with(bill);
    let revision = world.revision();
    let before = world.to_save_ron();

    let previews = [
        world.spend_preview(Some(&purchase("belt", 3))),
        world.spend_preview(Some(&PlayerCommand::BuyIngredient {
            item: "tomato".to_string(),
            quantity: 50,
        })),
        world.spend_preview(Some(&PlayerCommand::SignContract {
            id: unsigned_offer(),
        })),
        world.spend_preview(Some(&PlayerCommand::Hire {
            category: "logistics".to_string(),
        })),
        world.spend_preview(None),
    ];
    assert!(
        previews.iter().take(4).all(|p| p.spends),
        "all four are spends — the point is that ASKING costs nothing"
    );
    assert!(
        previews[3].warn,
        "and the hire is the one that is affordable whatever the balance, so \
         it is the one whose warning does not depend on the shipped prices"
    );
    assert_eq!(world.revision(), revision, "a preview is not a command");
    assert_eq!(world.to_save_ron(), before, "and it is not a mutation");

    // Asking twice gives the same answer, which is what makes a confirmation
    // panel's numbers the same numbers the command will act on.
    let repeat = world.spend_preview(Some(&purchase("belt", 3)));
    assert_eq!(repeat, previews[0]);
}

/// A spend the balance cannot cover is a REFUSAL, and a refusal has no
/// consequence to confirm. The economy declines it all-or-nothing, the
/// balance does not move, and the payday is left exactly as short as it
/// already was — so `warn` stays false and the click keeps the plain "no" it
/// has always got, rather than being asked to think about a payday this
/// purchase was never going to reach. `affordable` is what says so, and the
/// rest of the preview still answers honestly.
#[test]
fn a_spend_the_balance_cannot_cover_is_refused_rather_than_warned_about() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    assert!(bill > 0, "the shipped chain hires crew");

    // Flat broke, and the payday is already unpayable. Every one of the three
    // debiting commands answers the same way.
    let mut broke = world_with(0);
    let unaffordable: Vec<PlayerCommand> = vec![
        purchase("belt", 1),
        PlayerCommand::BuyIngredient {
            item: "tomato".to_string(),
            quantity: 1,
        },
        PlayerCommand::SignContract {
            id: unsigned_offer(),
        },
    ];
    for cmd in &unaffordable {
        let preview = broke.spend_preview(Some(cmd));
        assert!(preview.spends, "{cmd:?} is a spend, and says so");
        assert!(preview.cost > 0.0, "{cmd:?} is priced");
        assert!(
            !preview.affordable,
            "{cmd:?} costs {} against a balance of nothing",
            preview.cost
        );
        assert!(
            !preview.warn,
            "{cmd:?} cannot happen, so there is nothing to confirm"
        );
        // The shortfall is reported all the same: it is true, it is simply
        // not this command's doing.
        assert_eq!(preview.shortfall_now, bill);
        assert_eq!(preview.shortfall_after, bill);
        assert!(!preview.crosses);
    }
    // And the sim agrees: the commands are no-ops, so the panel would have
    // been asking about nothing.
    let before = broke.to_save_ron();
    for cmd in &unaffordable {
        broke.issue_command(cmd.clone());
    }
    assert_eq!(broke.money(), Money::ZERO);
    assert_eq!(
        broke.to_save_ron(),
        before,
        "an unaffordable spend leaves the world byte-identical"
    );

    // The boundary: a balance that covers the cost EXACTLY is affordable, and
    // that is the case the warning exists for — it can be spent, and spending
    // it leaves the payday with nothing.
    let cost = price_of("belt").to_num::<u32>();
    let exact = world_with(cost);
    let preview = exact.spend_preview(Some(&purchase("belt", 1)));
    assert!(preview.affordable, "the price itself is affordable");
    assert!(preview.warn, "and spending it starves the payday");
    assert!(preview.crosses || preview.shortfall_now > 0);
}

/// The JSON the panel actually reads, pinned field by field the way
/// `staff.rs` pins its snapshot: a rename here fails a test instead of
/// silently emptying a row of the confirmation.
#[test]
fn the_wire_shape_is_pinned() {
    let bill = reference_chain(recipes()).staff_snapshot().next_bill;
    let world = world_with(bill);
    let cmd = command_from_json(r#"{"type": "purchase", "kind": "belt", "quantity": 2}"#)
        .expect("purchase parses");
    let json = serde_json::to_value(world.spend_preview(Some(&cmd))).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "affordable",
            "bill_after",
            "cost",
            "crosses",
            "money_after",
            "money_now",
            "next_bill",
            "shortfall_after",
            "shortfall_now",
            "spends",
            "subject",
            "warn",
        ]
    );
    assert_eq!(json["subject"], "belt");
    assert_eq!(json["spends"], true);
    assert_eq!(json["warn"], true);
    assert_eq!(json["next_bill"], bill);
}
