//! Headless integration test for the economy (ARCHITECTURE.md §6's
//! testing-strategy tier; GAMEPLAY.md §6's contracts).
//! `sim_core/src/sim/economy.rs` and `contracts.rs` already unit-test the
//! loader and `Economy` in isolation (schema/degenerate-content rejection,
//! cadence boundaries, the shared-pool allocation rule); this file instead
//! drives the *whole* production chain end to end — spawning raw tomatoes,
//! letting them travel every belt/machine stage, collecting the finished
//! product, and letting `World::tick` settle contracts — to prove "build
//! factory -> make money" is actually true for the real chain, not just for
//! `Economy` in a vacuum.
//!
//! It was written against milestone 5's single hardcoded contract; the
//! invariants it asserts are unchanged ("money increases on a satisfied
//! mix, stays put on a miss"), they are just per-contract now.

use sim_core::sim::types::Fx;
use sim_core::sim::{
    ContractRegistry, FLUID_TANK_CAPACITY, MachineRegistry, PlayerCommand, RecipeRegistry, World,
};

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
        r#"(schema_version: 1, contracts: [Contract(id: "{}", name: "{}", mix: [{}], cadence_ticks: {}, payout: {}),],)"#,
        c.id,
        c.name,
        mix.join(", "),
        c.cadence_ticks,
        c.payout.to_num::<u32>(),
    );
    World::with_content(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::from_ron_str(&text).expect("re-serialized shipped contract must load"),
    )
}

/// How many units of `item` the final stage's machine currently holds in its
/// output buffer, searching every chunk the chain occupies (mirrors
/// `core_loop.rs`'s own `full_chain_snapshot` helper).
fn final_output_count(world: &World, item: &str) -> u32 {
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        for machine in &snapshot.machines {
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

/// Runs the chain until `target` units of its final product have been
/// collected into the shared delivery pool, or the tick budget runs out.
/// Returns how many were actually collected.
fn run_until_collected(world: &mut World, final_item: &str, target: u32) -> u32 {
    let mut collected = 0u32;
    for _ in 0..500_000u32 {
        if collected < target {
            // Keep the hopper topped up rather than spawning onto the belt.
            // `FillHopper` clamps at capacity, so issuing it every tick just
            // means "never let the feed run dry" — the hopper still meters
            // onto the belt at one unit per tick at most, so the chain's
            // throughput is unchanged by this.
            world.issue_command(PlayerCommand::FillHopper {
                amount: target.saturating_mul(4).max(4),
            });
            // ...and keep the pipe run's vinegar tank topped up the same
            // way, since it is the run's only supply and the fluid-gated
            // mixing stage stalls without it.
            world.issue_command(PlayerCommand::FillFluidTank {
                amount: FLUID_TANK_CAPACITY,
            });
        }

        world.tick();

        let ready = final_output_count(world, final_item);
        if ready > 0 {
            world.issue_command(PlayerCommand::CollectOutput);
            collected += ready;
        }

        if collected >= target {
            break;
        }
    }
    collected
}

#[test]
fn full_chain_delivers_against_a_contract_and_pays_out_exactly_that_contracts_payout() {
    let registry = recipes();
    let final_item = registry.ordered().last().unwrap().id.clone();

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
    let collected = run_until_collected(&mut world, &final_item, want_qty);

    assert!(
        collected >= want_qty,
        "expected to collect at least {want_qty} units of '{final_item}' well within 500k ticks, got {collected}"
    );

    // The full mix is now sitting in the delivery pool, but the exact tick
    // it landed on may fall anywhere relative to a cadence boundary — the
    // collection above could have completed just *after* the contract's
    // check already ran (and missed) this window. Advance a full extra
    // cadence period to guarantee at least one boundary is checked with the
    // complete mix already deposited.
    let target_tick = world.current_tick() + cadence;
    while world.current_tick() < target_tick {
        world.tick();
    }

    assert_eq!(
        world.money(),
        payout,
        "money must increase by exactly one contract payout once the mix is delivered"
    );
    assert_eq!(
        world.economy().contract("corner_shop").unwrap().deliveries_made,
        1
    );
}

#[test]
fn money_stays_zero_through_a_cadence_boundary_with_nothing_delivered() {
    let mut world = World::new(recipes());
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
        world.money(),
        Fx::from_num(0),
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
    let final_item = registry.ordered().last().unwrap().id.clone();

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
            schema_version: 1,
            contracts: [
                Contract(id: "quick", name: "Quick", mix: [("{final_item}", 1)], cadence_ticks: {quick_cadence}, payout: 10),
                Contract(id: "slow", name: "Slow", mix: [("{final_item}", 2)], cadence_ticks: {slow_cadence}, payout: 35),
                Contract(id: "impossible", name: "Impossible", mix: [("ketchup", 5)], cadence_ticks: {quick_cadence}, payout: 999),
            ],
        )"#
    );
    let mut world = World::with_content(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::from_ron_str(&board).expect("fixture board must load"),
    );

    // Run the chain continuously (never stop feeding, never stop
    // collecting) through exactly one slow-cadence window — the slow
    // contract's very first boundary. Production ramps up for the first
    // ~3000 ticks (the aging rack's own hold plus every stage upstream and
    // downstream of it) and then settles into steady state well before
    // that boundary, so by the time it's checked the pool has had time to
    // build past both contracts' requirements even after the quick
    // contract's own two checks (at one and two quick-cadences in) have
    // each taken their share.
    while world.current_tick() < slow_cadence {
        world.issue_command(PlayerCommand::FillHopper { amount: 32 });
        world.issue_command(PlayerCommand::FillFluidTank {
            amount: FLUID_TANK_CAPACITY,
        });
        world.tick();
        if final_output_count(&world, &final_item) > 0 {
            world.issue_command(PlayerCommand::CollectOutput);
        }
    }

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
        slow.deliveries_made, 1,
        "a {slow_cadence}-tick contract should have settled exactly once by tick {}",
        world.current_tick()
    );
    assert_eq!(
        impossible.deliveries_made, 0,
        "a contract asking for an item the delivery mechanism never produces must never pay"
    );
    assert!(impossible.deliveries_missed >= 2);

    // 'impossible' pays 999; if it had ever paid, this would be nowhere near.
    let expected = Fx::from_num(quick.deliveries_made * 10 + slow.deliveries_made * 35);
    assert_eq!(
        world.money(),
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
    let final_item = registry.ordered().last().unwrap().id.clone();

    // Two identical contracts, both due at tick 1000, each wanting one unit.
    // Exactly one unit is ever collected, so exactly one may be paid — and
    // it must be the earlier-listed one (file order is the documented
    // scarcity tiebreak).
    let board = format!(
        r#"(
            schema_version: 1,
            contracts: [
                Contract(id: "first", name: "First", mix: [("{final_item}", 1)], cadence_ticks: 1000, payout: 10),
                Contract(id: "second", name: "Second", mix: [("{final_item}", 1)], cadence_ticks: 1000, payout: 10),
            ],
        )"#
    );
    let mut world = World::with_content(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::from_ron_str(&board).expect("fixture board must load"),
    );

    // Collect exactly one unit, then stop feeding the line entirely.
    let collected = run_until_collected(&mut world, &final_item, 1);
    assert_eq!(collected, 1, "the chain produces one unit at a time");

    let target = world.current_tick() + 1000;
    while world.current_tick() < target {
        world.tick();
    }

    assert_eq!(
        world.money(),
        Fx::from_num(10),
        "one box must pay exactly one contract — if stock were credited to every matching \
         contract this would read 20, which is the free-money bug the shared-pool rule exists \
         to prevent"
    );
    assert_eq!(world.economy().contract("first").unwrap().deliveries_made, 1);
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
    let world = World::new(recipes());
    let snapshot = world.economy_snapshot();
    let board = contracts();

    assert_eq!(snapshot.contracts.len(), board.len());
    assert!(snapshot.contracts.len() >= 2, "several contracts must ship");
    let ids: Vec<&str> = snapshot.contracts.iter().map(|c| c.id.as_str()).collect();
    let expected: Vec<&str> = board.ordered().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, expected, "snapshot order must match contracts.ron order");

    assert_eq!(snapshot.money, 0.0);
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
    assert!(json.contains("\"delivery_stock\""), "{json}");
    assert!(
        !json.contains("contract_mix"),
        "the old single-contract fields must be gone: {json}"
    );
}
