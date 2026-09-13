//! Shared fixtures for the integration tests that need the shipped chain to
//! actually *run* for a long time — `dock.rs`, `economy.rs` and `fluids.rs`.
//!
//! Two things every such test now has to do, both consequences of the
//! chain-rework epic:
//!
//! 1. **Buy its ingredients.** Nothing is free any more (GAMEPLAY.md §5), so
//!    a world that is going to be run has to be built with a budget
//!    (`reference_chain_with_budget`) and then stocked. [`STOCK_COST`] is
//!    sized so a stocked world's balance lands on exactly **zero**, which
//!    keeps every "money increased by exactly one payout" assertion measured
//!    against a clean baseline.
//! 2. **Keep both hoppers fed, not one.** The shipped chain has two raw
//!    solids now, each arriving on its own belt lane into its own tile of
//!    the mixer's inlet face (`data/machines.ron`'s note on why the mixer is
//!    1x2). [`supply`] tops up every hopper and the tank, which is all a
//!    running line needs.

use sim_core::sim::types::Fx;
use sim_core::sim::{FLUID_TANK_CAPACITY, HOPPER_CAPACITY, PlayerCommand, World};

/// A long run's worth of every raw ingredient the shipped chain needs.
/// Generous rather than exact: these fixtures are about the line running for
/// hundreds of thousands of ticks, not about the bill.
#[allow(dead_code)]
pub const TOMATO_UNITS: u32 = 2000;
#[allow(dead_code)]
pub const SUGAR_UNITS: u32 = 500;
#[allow(dead_code)]
pub const VINEGAR_UNITS: u32 = 1000;

/// What [`stock_up`] spends, at `data/ingredients.ron`'s current prices.
///
/// It is a literal, and [`stock_up`] asserts the actual spend matches it —
/// so an ingredient price change fails loudly *here*, with a message about
/// the fixture's bill, rather than silently leaving a test world with money
/// in it and breaking an unrelated assertion about contract payouts
/// somewhere else. Fixtures pass this as their `reference_chain_with_budget`
/// budget, which is what keeps a stocked world's balance at exactly zero.
#[allow(dead_code)]
pub const STOCK_COST: u32 = 5500;

/// How many pay periods of the STARTING crew's wages a long-running fixture
/// holds in reserve (the staff pools MVP epic). A fixture that stocks up
/// and is left with nothing loses its whole crew at the first payday — a
/// factory with no money has no staff, which is the mechanic, not a bug —
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
pub fn wages_paid(world: &World) -> Fx {
    let staff = world.staff();
    assert_eq!(
        staff.walked_out_last_payday(),
        0,
        "wages_paid() assumes the starting crew is intact — fund the fixture with WAGE_RESERVE"
    );
    let bill = u64::from(staff.bill_per_period());
    let period = staff.catalog().pay_period_ticks();
    let paydays = staff.next_payday_tick() / period - 1;
    Fx::from_num(bill * paydays)
}

/// Buys `quantity` of `item`, asserting it was affordable — an unaffordable
/// buy is a silent no-op, which would otherwise surface much later as an
/// unexplained idle factory.
#[allow(dead_code)]
pub fn buy(world: &mut World, item: &str, quantity: u32) {
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

/// Buys [`STOCK_COST`] worth of raw ingredients.
#[allow(dead_code)]
pub fn stock_up(world: &mut World) {
    let before = world.money();
    buy(world, "tomato", TOMATO_UNITS);
    buy(world, "sugar", SUGAR_UNITS);
    buy(world, "vinegar", VINEGAR_UNITS);
    assert_eq!(
        before - world.money(),
        Fx::from_num(STOCK_COST),
        "STOCK_COST must match what data/ingredients.ron actually charges for \
         {TOMATO_UNITS} tomato + {SUGAR_UNITS} sugar + {VINEGAR_UNITS} vinegar"
    );
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
