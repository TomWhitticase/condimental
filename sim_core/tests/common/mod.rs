//! Shared fixtures for the integration tests that need the shipped chain to
//! actually *run* for a long time — `dock.rs`, `economy.rs` and `fluids.rs`.
//!
//! Two things every such test now has to do, both consequences of the
//! chain-rework epic:
//!
//! 1. **Buy its ingredients — and build somewhere to put them.** Nothing is
//!    free any more (GAMEPLAY.md §5), and since W1 nothing is unlimited
//!    either: stock is capped per ingredient id at `data/storage.ron`'s
//!    `base_cap` plus whatever **bays** the placed warehouse racks supply.
//!    A fixture that wants a long run's worth of raw therefore has to put
//!    racks down first, exactly as a player would — [`stock_up`] does both,
//!    and [`STOCK_COST`] is the whole bill (racks + ingredients), sized so a
//!    stocked world's balance lands on exactly **zero**, which keeps every
//!    "money increased by exactly one payout" assertion measured against a
//!    clean baseline.
//! 2. **Keep both hoppers fed, not one.** The shipped chain has two raw
//!    solids now, each arriving on its own belt lane into its own tile of
//!    the mixer's inlet face (`data/machines.ron`'s note on why the mixer is
//!    1x2). [`supply`] tops up every hopper and the tank, which is all a
//!    running line needs.

use sim_core::sim::types::{Direction, Fx, Money};
use sim_core::sim::{
    FLUID_TANK_CAPACITY, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, WAREHOUSE_RACK_KIND, World,
};

/// A long run's worth of every raw ingredient the shipped chain needs.
/// Generous rather than exact: these fixtures are about the line running for
/// hundreds of thousands of ticks, not about the bill.
#[allow(dead_code)]
pub const TOMATO_UNITS: u32 = 2000;
#[allow(dead_code)]
pub const SUGAR_UNITS: u32 = 500;
#[allow(dead_code)]
pub const VINEGAR_UNITS: u32 = 1000;

/// The ingredient half of [`stock_up`]'s bill, at `data/ingredients.ron`'s
/// current prices.
///
/// It is a literal, and [`stock_up`] asserts the actual spend matches it —
/// so an ingredient price change fails loudly *here*, with a message about
/// the fixture's bill, rather than silently leaving a test world with money
/// in it and breaking an unrelated assertion about contract payouts
/// somewhere else.
#[allow(dead_code)]
pub const INGREDIENT_COST: u32 = 5500;

/// The row the fixture's warehouse stands on, and the one south of it.
/// Everything `sim::reference_chain` lays down is at `y >= 0` (lanes run
/// `CHAIN_ROW_Y + j`, pipe rows north of that), so a rack row well south of
/// the line cannot collide with it however the content grows.
const RACK_ROW_Y: i32 = -8;

/// How far east [`add_rack`] will look for a free tile before giving up —
/// a bound on the search, not on the warehouse.
const RACK_ROW_TILES: i32 = 64;

/// How many racks [`stock_up`] ends up building, from how much it means to
/// hold.
///
/// `data/storage.ron` gives every id `base_cap` free and charges one bay per
/// `slot_stack` above it, so the stock below needs
/// `ceil(1980/50) + ceil(480/50) + ceil(980/50)` = `40 + 10 + 20` = 70 bays,
/// and a rack supplies six. Spelled out rather than computed so that a
/// `storage.ron` edit fails in [`stock_up`]'s own assertion instead of
/// somewhere downstream in a test about contract payouts.
#[allow(dead_code)]
pub const RACK_COUNT: u32 = 12;

/// `RACK_COUNT` x `data/shop.ron`'s `warehouse_rack` price.
#[allow(dead_code)]
pub const RACK_COST: u32 = 900;

/// What [`stock_up`] spends in total: the warehouse and then the stock that
/// goes in it. Fixtures pass this as their `reference_chain_with_budget`
/// budget, which is what keeps a stocked world's balance at exactly zero.
#[allow(dead_code)]
pub const STOCK_COST: u32 = RACK_COST + INGREDIENT_COST;

/// How many pay periods of the STARTING crew's wages a long-running fixture
/// holds in reserve (the staff pools MVP epic). A fixture that stocks up
/// and is left with nothing has its whole crew unpaid and idle after the
/// first payday and gone after the second — a factory with no money has
/// no staff, which is the mechanic, not a bug —
/// so every fixture that ticks past a payday funds itself with
/// [`WAGE_RESERVE`] on top of what it spends. 300 periods covers the
/// longest loop in these files (200 000 ticks in `fluids.rs`).
#[allow(dead_code)]
pub const WAGE_PERIODS: u64 = 300;
/// `WAGE_PERIODS` × the starting crew's bill per period (4 + 0 + 2 + 1
/// heads at wage 2 = 14). A literal, like `STOCK_COST`, pinned against
/// `data/staff.ron` and `data/machines.ron` by `tests/staff.rs`.
#[allow(dead_code)]
pub const WAGE_RESERVE: u32 = 4200;

/// What the starting crew has been paid so far — the figure to subtract
/// from a fixture's balance before asking what it *earned*. Valid only
/// while the crew is the starting one (no hire, fire or walk-out), which
/// it asserts; a payday count is read off the payday clock, so it needs no
/// bookkeeping of its own.
#[allow(dead_code)]
pub fn wages_paid(world: &World) -> Money {
    let staff = world.staff();
    assert_eq!(
        (staff.walked_out_last_payday(), staff.unpaid_last_payday()),
        (0, 0),
        "wages_paid() assumes the starting crew is intact and paid — fund the fixture with \
         WAGE_RESERVE"
    );
    let bill = u64::from(staff.bill_per_period());
    let period = staff.catalog().pay_period_ticks();
    let paydays = staff.next_payday_tick() / period - 1;
    Money::from_num(bill * paydays)
}

/// Buys `quantity` of `item` into a warehouse big enough to take it,
/// asserting it was affordable — an unaffordable buy is a silent no-op,
/// which would otherwise surface much later as an unexplained idle factory,
/// and since W1 so is an order with nowhere to go.
#[allow(dead_code)]
pub fn buy(world: &mut World, item: &str, quantity: u32) {
    make_room(world, item, quantity);
    let before = world.owned_ingredient(item);
    world.issue_command(PlayerCommand::BuyIngredient {
        item: item.to_string(),
        quantity,
    });
    assert_eq!(
        world.owned_ingredient(item),
        before + quantity,
        "the fixture must be able to afford {quantity} x {item}"
    );
}

/// Builds the fixture's warehouse and fills it: [`STOCK_COST`] of racks and
/// raw ingredients, leaving the world with exactly what it was funded with
/// on top of that.
///
/// The racks are not a separate step — [`buy`] puts down whatever the order
/// it is about to place needs room for. What is asserted here is that the
/// warehouse came out the size [`RACK_COUNT`] says, so that a
/// `data/storage.ron` edit fails on this line rather than on some later
/// test's arithmetic about money.
#[allow(dead_code)]
pub fn stock_up(world: &mut World) {
    let before = world.money();
    buy(world, "tomato", TOMATO_UNITS);
    buy(world, "sugar", SUGAR_UNITS);
    buy(world, "vinegar", VINEGAR_UNITS);
    assert_eq!(
        before - world.money(),
        Money::from_num(STOCK_COST),
        "STOCK_COST must match what data/shop.ron charges for {RACK_COUNT} racks plus what \
         data/ingredients.ron charges for {TOMATO_UNITS} tomato + {SUGAR_UNITS} sugar + \
         {VINEGAR_UNITS} vinegar"
    );
    assert_eq!(
        world.storage_bays(),
        RACK_COUNT * world.storage().bays_of_kind(WAREHOUSE_RACK_KIND),
        "holding {TOMATO_UNITS} tomato + {SUGAR_UNITS} sugar + {VINEGAR_UNITS} vinegar under \
         the current data/storage.ron took a different number of racks than RACK_COUNT's \
         {RACK_COUNT} — update RACK_COUNT and RACK_COST together"
    );
}

/// Buys and places however many warehouse racks it takes for `quantity` of
/// `item` to fit, through the same public commands a player uses.
///
/// Every fixture that holds more than `data/storage.ron`'s `base_cap` of
/// anything needs this, because `BuyIngredient` silently clamps an order to
/// the room that exists. Racks are added one at a time and rechecked rather
/// than computed, so the bay arithmetic lives in `sim_core` alone and this
/// file cannot drift from it.
#[allow(dead_code)]
pub fn make_room(world: &mut World, item: &str, quantity: u32) {
    while world.stock_room_for(item) < quantity {
        assert!(
            add_rack(world),
            "the fixture ran out of money or of floor before it could hold {quantity} x \
             {item} — it has {} bays and {}",
            world.storage_bays(),
            world.money()
        );
    }
}

/// One more rack, on the first free tile of the row south of the line.
///
/// Racks are 1x2, so the row occupies [`RACK_ROW_Y`] and the row north of
/// it; both are empty in every reference layout. The free tile is found by
/// *trying* rather than by reading occupancy, which keeps this helper
/// correct for a test that has built something of its own down there.
fn add_rack(world: &mut World) -> bool {
    world.issue_command(PlayerCommand::Purchase {
        kind: WAREHOUSE_RACK_KIND.to_string(),
        quantity: 1,
    });
    for x in 0..RACK_ROW_TILES {
        let before = world.placement_count();
        world.issue_command(PlayerCommand::Place {
            kind: WAREHOUSE_RACK_KIND.to_string(),
            tile: (x, RACK_ROW_Y),
            direction: Direction::East,
            item: None,
            recipe: None,
        });
        if world.placement_count() > before {
            return true;
        }
    }
    false
}

/// One tick's worth of keeping the line fed, issuing nothing but fill
/// commands. Call it before `world.tick()`.
#[allow(dead_code)] // not every test file that uses this module needs both
pub fn supply(world: &mut World) {
    supply_solids(world);
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });
}

/// [`supply`] without topping the fluid tank up — for a test that wants to
/// watch the tank *drain*, which it cannot do while something refills it
/// every tick.
#[allow(dead_code)]
pub fn supply_solids(world: &mut World) {
    world.issue_command(PlayerCommand::FillHopper {
        amount: HOPPER_CAPACITY,
        tile: None,
    });
}

/// Tops the larder back up to [`stock_up`]'s quantities, spending whatever
/// the world can afford and asserting nothing.
///
/// This is what a long run does instead of stocking twice. Before W1 a
/// fixture that needed two larders' worth simply bought two, because a
/// holding had no ceiling; now it cannot — a second `stock_up` would ask for
/// 140 bays of warehouse and be clamped. So the extra stock arrives the way
/// a player's does: as the line eats what is there, room appears and the
/// order goes in again. Unaffordable buys are silent no-ops here on purpose,
/// because a run that outlives its budget is exactly what the calling test
/// is measuring.
#[allow(dead_code)]
pub fn restock(world: &mut World) {
    for (item, target) in [
        ("tomato", TOMATO_UNITS),
        ("sugar", SUGAR_UNITS),
        ("vinegar", VINEGAR_UNITS),
    ] {
        let held = world.owned_ingredient(item);
        if held < target {
            world.issue_command(PlayerCommand::BuyIngredient {
                item: item.to_string(),
                quantity: target - held,
            });
        }
    }
}

/// How many units of raw ingredient `raw` one unit of `id` embodies, by
/// recursing back through whichever recipe produces it — following **fluid**
/// production as readily as solid, since the middle of the chain is piped.
///
/// Answers `1` for `raw` itself and `0` for anything nothing produces and
/// that is not `raw` (i.e. a *different* raw ingredient: sugar carries no
/// tomatoes). A recipe's bill is divided by how many of `id` one craft
/// makes, because not every stage makes one: a bottler fills a case's worth
/// of bottles from one batch, so a bottle carries an eighth of that batch.
/// That is why this is `Fx` rather than a whole count. One copy, shared by
/// every conservation and costing test, so a yield can never be honoured in
/// one file and forgotten in another.
#[allow(dead_code)]
pub fn raw_content(id: &str, raw: &str, recipes: &RecipeRegistry) -> Fx {
    if id == raw {
        return Fx::ONE;
    }
    let Some(recipe) = recipes.ordered().iter().find(|r| {
        r.outputs.iter().any(|(out, _)| out == id)
            || r.fluid_outputs.iter().any(|(out, _)| out == id)
    }) else {
        // Produced by nothing: another raw ingredient, which embodies none
        // of the one being counted.
        return Fx::ZERO;
    };
    let per_craft = recipe
        .outputs
        .iter()
        .chain(recipe.fluid_outputs.iter())
        .find(|(out, _)| out == id)
        .map_or(1, |(_, n)| *n);
    let bill: Fx = recipe
        .inputs
        .iter()
        .chain(recipe.fluid_inputs.iter())
        .map(|(input, qty)| Fx::from_num(*qty) * raw_content(input, raw, recipes))
        .sum();
    bill / Fx::from_num(per_craft)
}
