//! Headless integration test for the economy (ARCHITECTURE.md §6's
//! testing-strategy tier; GAMEPLAY.md §6's contracts).
//! `sim_core/src/sim/economy.rs` and `contracts.rs` already unit-test the
//! loader and `Economy` in isolation (schema/degenerate-content rejection,
//! cadence boundaries, the shared-pool allocation rule); this file instead
//! drives the *whole* production chain end to end — loading raw tomatoes,
//! letting them travel every belt/machine stage, letting the delivery dock
//! ship the finished product, and letting `World::tick` settle contracts —
//! to prove "build factory -> make money" is actually true for the real
//! chain, not just for `Economy` in a vacuum.
//!
//! It was written against milestone 5's single hardcoded contract; the
//! invariants it asserts are unchanged ("money increases on a satisfied
//! mix, stays put on a miss"), they are just per-contract now.
//!
//! **The delivery-dock ticket removed the one player action this file used
//! to depend on.** Every drive loop below used to call
//! `PlayerCommand::CollectOutput` whenever the last machine held product;
//! that command no longer exists, because the chain now ends in a real
//! belt-and-dock exit that ships into the shared delivery pool by itself
//! every tick. The loops therefore only *feed* the line now — hopper and
//! vinegar tank — and everything downstream happens with no player
//! involvement at all, which is exactly the behaviour these tests should be
//! proving in the first place.

mod common;

use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, DELIVERY_STOCK_CAPACITY, DockSnapshot, MachineRegistry, PlayerCommand,
    RecipeRegistry, World, linear_prefix, reference_chain_with_budget,
    reference_chain_with_content_and_budget,
};

/// The reference chain's one dock — where every delivery this file measures
/// comes from.
fn the_dock(world: &World) -> DockSnapshot {
    let mut docks = world.dock_snapshots();
    assert_eq!(docks.len(), 1, "the reference chain places one dock");
    docks.remove(0)
}

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn contracts() -> ContractRegistry {
    ContractRegistry::bundled()
}

/// A world whose only contract is the shipped `corner_shop` one — the same
/// 2-boxes-per-1000-ticks-for-50 agreement milestone 5 hardcoded. Its terms
/// are re-serialized out of `data/contracts.ron` rather than copied into
/// constants here, so this fixture can never drift from the shipped
/// content; what it drops is the *other* shipped contracts, so a test can
/// reason about exactly one payout without their windows also moving
/// `money`.
fn single_contract_world() -> World {
    let board = contracts();
    let c = board.get("corner_shop").expect("shipped starter contract");
    let mix: Vec<String> = c
        .mix
        .iter()
        .map(|(item, qty)| format!("(\"{item}\", {qty})"))
        .collect();
    let text = format!(
        r#"(schema_version: 2, contracts: [Contract(id: "{}", name: "{}", mix: [{}], cadence_ticks: {}, payout: {}, price: {}, strikes: {}, starter: true),],)"#,
        c.id,
        c.name,
        mix.join(", "),
        c.cadence_ticks,
        c.payout.to_num::<u32>(),
        c.price.to_num::<u32>(),
        c.strikes,
    );
    stocked(reference_chain_with_content_and_budget(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::from_ron_str(&text).expect("re-serialized shipped contract must load"),
        BUDGET,
    ))
}

/// Exactly `common::stock_up`'s ingredient bill — so a stocked world has a
/// factory, a full larder and **zero money**, which is the baseline every
/// payout assertion below is measured against.
/// Stock plus a wage reserve (the staff pools MVP epic): every fixture here
/// runs past a payday, and a broke factory loses its crew. `earned` below
/// is what the assertions compare — the balance with the reserve taken out
/// and the wages paid put back — so "money is exactly the payouts" still
/// means exactly that.
const BUDGET: u32 = common::STOCK_COST + common::WAGE_RESERVE;

fn earned(world: &World) -> Money {
    world.money() + common::wages_paid(world) - Money::from_num(common::WAGE_RESERVE)
}

/// Buys the raw ingredients a long run needs. `reference_chain*` spends
/// exactly its shop bill and leaves only the budget it was given, and
/// ingredients cost money now (GAMEPLAY.md §5), so a fixture that wants the
/// line to actually run has to be built with a budget and then stocked.
fn stocked(mut world: World) -> World {
    common::stock_up(&mut world);
    world
}

/// Runs the chain until the delivery dock has shipped `target` units into
/// the shared delivery pool, or the tick budget runs out. Returns how many
/// it actually shipped.
///
/// Note what this does *not* do since the delivery-dock ticket: nothing here
/// empties the chain. The only commands issued are the two that supply
/// consumables; the dock does the rest by itself, every tick.
fn run_until_shipped(world: &mut World, target: u32) -> u32 {
    for _ in 0..500_000u32 {
        if the_dock(&world).shipped_total < target {
            // Keep the line supplied — see `common`'s module doc for why
            // sugar is metered rather than dumped onto a shared belt.
            common::supply(world);
        }

        world.tick();

        if the_dock(&world).shipped_total >= target {
            break;
        }
    }
    the_dock(&world).shipped_total
}

#[test]
fn full_chain_delivers_against_a_contract_and_pays_out_exactly_that_contracts_payout() {
    let registry = recipes();
    let final_item = linear_prefix(&registry).last().unwrap().id.clone();

    let board = contracts();
    let corner_shop = board.get("corner_shop").expect("shipped starter contract");
    let (want_item, want_qty) = corner_shop.mix.first().unwrap().clone();
    assert_eq!(
        want_item, final_item,
        "this test assumes the starter contract targets the chain's final product"
    );
    let cadence = corner_shop.cadence_ticks;
    let payout = corner_shop.payout;

    // Only this one contract is active, so `money` moving means *this*
    // contract paid — with the whole shipped board the others would settle
    // too and the assertion would be about their sum instead.
    let mut world = single_contract_world();
    let shipped = run_until_shipped(&mut world, want_qty);

    assert!(
        shipped >= want_qty,
        "expected the dock to ship at least {want_qty} units of '{final_item}' well within \
         500k ticks with no collect command at all, got {shipped}"
    );

    // The full mix is now sitting in the delivery pool, but the exact tick
    // it landed on may fall anywhere relative to a cadence boundary — the
    // last box could have arrived just *after* the contract's check already
    // ran (and missed) this window. Advance a full extra
    // cadence period to guarantee at least one boundary is checked with the
    // complete mix already deposited.
    let target_tick = world.current_tick() + cadence;
    while world.current_tick() < target_tick {
        world.tick();
    }

    assert_eq!(
        earned(&world),
        payout,
        "money must increase by exactly one contract payout once the mix is delivered"
    );
    assert_eq!(
        world
            .economy()
            .contract("corner_shop")
            .unwrap()
            .deliveries_made,
        1
    );
}

#[test]
fn money_stays_zero_through_a_cadence_boundary_with_nothing_delivered() {
    let mut world = stocked(reference_chain_with_budget(recipes(), BUDGET));
    let longest = world
        .economy()
        .contracts
        .iter()
        .map(|c| c.contract.cadence_ticks)
        .max()
        .expect("the shipped board has at least one contract");

    // Advance past every shipped contract's first cadence boundary without
    // ever collecting anything — no mix is ever satisfied, so no payout
    // should ever land, no matter how long the chain sits idle.
    for _ in 0..=longest {
        world.tick();
    }

    assert_eq!(
        earned(&world),
        Money::from_num(0),
        "money must not increase when the delivered mix doesn't satisfy any contract"
    );
    for active in &world.economy().contracts {
        assert_eq!(active.deliveries_made, 0);
        assert!(
            active.deliveries_missed >= 1,
            "contract '{}' should have missed at least one window",
            active.contract.id
        );
    }
}

/// The multi-contract half of the same loop, driven through the real chain:
/// two contracts on different cadences, both wanting the chain's product,
/// with one of them impossible to satisfy. The satisfiable pair must pay on
/// their own independent boundaries; the impossible one must never pay and
/// must not block the others.
#[test]
fn several_contracts_settle_independently_against_one_real_chain() {
    let registry = recipes();
    let final_item = linear_prefix(&registry).last().unwrap().id.clone();

    // Cadences chosen relative to milestone 7's aging-rack-bottlenecked
    // throughput (roughly one unit per ~2600 ticks in steady state, an
    // order of magnitude slower than when this test was first written,
    // since one unit's farm-to-final-product latency now includes the
    // aging rack's 2400-tick hold — see timed_processes.rs). Both cadences
    // must stay comfortably longer than that per-unit production interval:
    // if a cadence were shorter than the production interval (as the
    // pre-milestone-7 values of 900/3000 now are), that contract would
    // check for stock more often than new stock arrives and would drain
    // the shared pool the instant anything lands in it, permanently
    // starving the other contract rather than the two settling on their
    // own independent schedules, which is the actual thing under test.
    let quick_cadence: u64 = 6_000;
    let slow_cadence: u64 = 15_000;
    let board = format!(
        r#"(
            schema_version: 2,
            contracts: [
                Contract(id: "quick", name: "Quick", mix: [("{final_item}", 1)], cadence_ticks: {quick_cadence}, payout: 10, price: 1, strikes: 100, starter: true),
                Contract(id: "slow", name: "Slow", mix: [("{final_item}", 2)], cadence_ticks: {slow_cadence}, payout: 35, price: 1, strikes: 100, starter: true),
                Contract(id: "impossible", name: "Impossible", mix: [("ketchup", 5)], cadence_ticks: {quick_cadence}, payout: 999, price: 1, strikes: 100, starter: true),
            ],
        )"#
    );
    let mut world = stocked(reference_chain_with_content_and_budget(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::from_ron_str(&board).expect("fixture board must load"),
        BUDGET,
    ));

    // Run the chain continuously (never stop feeding; the dock never stops
    // shipping) through exactly one slow-cadence window — the slow
    // contract's very first boundary. Production ramps up for the first
    // ~3000 ticks (the aging rack's own hold plus every stage upstream and
    // downstream of it) and then settles into steady state well before
    // that boundary, so by the time it's checked the pool has had time to
    // build past both contracts' requirements even after the quick
    // contract's own two checks (at one and two quick-cadences in) have
    // each taken their share.
    while world.current_tick() < slow_cadence {
        common::supply(&mut world);
        world.tick();
    }
    assert!(
        the_dock(&world).shipped_total > 0,
        "the dock must have shipped product on its own for either contract to settle"
    );

    let econ = world.economy();
    let quick = econ.contract("quick").unwrap();
    let slow = econ.contract("slow").unwrap();
    let impossible = econ.contract("impossible").unwrap();

    assert!(
        quick.deliveries_made >= 2,
        "a {quick_cadence}-tick contract should have settled several times by tick {}, got {}",
        world.current_tick(),
        quick.deliveries_made
    );
    assert_eq!(
        slow.deliveries_made,
        1,
        "a {slow_cadence}-tick contract should have settled exactly once by tick {}",
        world.current_tick()
    );
    assert_eq!(
        impossible.deliveries_made, 0,
        "a contract asking for an item the delivery mechanism never produces must never pay"
    );
    assert!(impossible.deliveries_missed >= 2);

    // 'impossible' pays 999; if it had ever paid, this would be nowhere near.
    let expected = Money::from_num(quick.deliveries_made * 10 + slow.deliveries_made * 35);
    assert_eq!(
        earned(&world),
        expected,
        "money must be exactly the sum of the payouts that actually settled"
    );
}

/// The delivery-allocation rule (`sim/economy.rs`'s module doc), proven
/// through the real chain rather than a unit fixture: one collected box is
/// spent by the contract that takes it, never credited to a second one.
#[test]
fn one_delivered_unit_pays_one_contract_not_every_contract_that_wants_it() {
    let registry = recipes();
    let final_item = linear_prefix(&registry).last().unwrap().id.clone();

    // Two identical contracts, both due at tick 1000, each wanting one unit.
    // Exactly one unit is ever collected, so exactly one may be paid — and
    // it must be the earlier-listed one (file order is the documented
    // scarcity tiebreak).
    let board = format!(
        r#"(
            schema_version: 2,
            contracts: [
                Contract(id: "first", name: "First", mix: [("{final_item}", 1)], cadence_ticks: 1000, payout: 10, price: 1, strikes: 100, starter: true),
                Contract(id: "second", name: "Second", mix: [("{final_item}", 1)], cadence_ticks: 1000, payout: 10, price: 1, strikes: 100, starter: true),
            ],
        )"#
    );
    let mut world = stocked(reference_chain_with_content_and_budget(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::from_ron_str(&board).expect("fixture board must load"),
        BUDGET,
    ));

    // Ship exactly one unit, then take the DOCK back off the floor so no
    // second unit can ever reach the pool. Merely stopping the fill
    // commands is not enough and used to pass only by luck: the line is
    // still full of in-flight product when the first box lands, so it goes
    // on shipping for a long while afterwards, and whether a second box
    // beats the settle boundary is a function of every process time in
    // `data/recipes.ron`. Removing the exit makes "exactly one unit is ever
    // collected" true by construction — through an ordinary player command,
    // like everything else this fixture does.
    let shipped = run_until_shipped(&mut world, 1);
    assert_eq!(shipped, 1, "the chain produces one unit at a time");
    world.issue_command(PlayerCommand::Remove {
        tile: the_dock(&world).tile,
    });
    assert!(
        world.dock_snapshots().is_empty(),
        "the exit must really be gone, or the pool can still grow"
    );

    let target = world.current_tick() + 1000;
    while world.current_tick() < target {
        world.tick();
    }

    assert_eq!(
        earned(&world),
        Money::from_num(10),
        "one box must pay exactly one contract — if stock were credited to every matching \
         contract this would read 20, which is the free-money bug the shared-pool rule exists \
         to prevent"
    );
    assert_eq!(
        world.economy().contract("first").unwrap().deliveries_made,
        1
    );
    assert_eq!(
        world.economy().contract("second").unwrap().deliveries_made,
        0,
        "file order is the documented tiebreak: the earlier-listed contract takes the stock"
    );
}

/// The snapshot the Godot layer reads (`SimBridge::economy_snapshot_json`)
/// must report every active contract, not just the first — the shape
/// Ticket 2's contract cards are built against.
#[test]
fn the_economy_snapshot_reports_one_entry_per_shipped_contract() {
    let world = stocked(reference_chain_with_budget(recipes(), BUDGET));
    let snapshot = world.economy_snapshot();
    let board = contracts();

    // Active = the board's starters only (the contract-board epic: the rest
    // are signed); the whole board rides along beside them.
    let ids: Vec<&str> = snapshot.contracts.iter().map(|c| c.id.as_str()).collect();
    let expected: Vec<&str> = board.starters().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids, expected,
        "a new world's active contracts are the starters, in contracts.ron order"
    );
    assert_eq!(snapshot.board.len(), board.len());
    assert!(
        snapshot.board.len() >= 2,
        "several contracts must ship, to be signed"
    );
    let offered: Vec<&str> = snapshot.board.iter().map(|o| o.id.as_str()).collect();
    let all: Vec<&str> = board.ordered().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(offered, all, "board order must match contracts.ron order");
    for offer in &snapshot.board {
        assert_eq!(
            offer.active, offer.starter,
            "in a new world exactly the starters are active"
        );
        assert!(offer.price > 0.0);
        assert!(offer.strikes >= 1);
    }

    assert_eq!(
        snapshot.money,
        f64::from(common::WAGE_RESERVE),
        "stocked, untouched: the reserve"
    );
    assert!(
        snapshot.delivery_stock.is_empty(),
        "a fresh world has delivered nothing yet"
    );
    for contract in &snapshot.contracts {
        assert!(!contract.mix.is_empty());
        assert_eq!(contract.mix.len(), contract.progress.len());
        assert!(contract.payout > 0.0);
        assert_eq!(contract.next_delivery_tick, contract.cadence_ticks);
    }

    // And it must actually serialize to the JSON the bridge hands Godot.
    let json = serde_json::to_string(&snapshot).expect("EconomySnapshot must serialize");
    assert!(json.contains("\"contracts\":["), "{json}");
    assert!(json.contains("\"board\":["), "{json}");
    assert!(json.contains("\"delivery_stock\""), "{json}");
    // The pool's ceiling and whether it is at it (the dock-capacity change,
    // 2026-09-16) — what the clipboard's ON THE DOCK line draws its bar
    // against (UI_UX.md §5e) and warns with.
    assert!(
        json.contains(&format!("\"delivery_capacity\":{DELIVERY_STOCK_CAPACITY}")),
        "{json}"
    );
    assert!(json.contains("\"delivery_full\":false"), "{json}");
    assert!(
        !json.contains("contract_mix"),
        "the old single-contract fields must be gone: {json}"
    );
}
