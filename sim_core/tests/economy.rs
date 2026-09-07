//! Headless integration test for milestone 5 (ARCHITECTURE.md §9 item 5,
//! "Economy MVP"; §6's testing-strategy tier). `sim_core/src/sim/economy.rs`
//! already unit-tests `Economy` in isolation (cadence boundaries, mix
//! satisfaction, missed-then-later-satisfied deliveries); this file instead
//! drives the *whole* milestone-4 production chain end to end — spawning raw
//! tomatoes, letting them travel every belt/machine stage, collecting the
//! finished product, and letting `World::tick` check the hardcoded contract
//! — to prove "build factory -> make money" is actually true for the real
//! chain, not just for `Economy` in a vacuum.

use sim_core::sim::economy::{CONTRACT_CADENCE_TICKS, CONTRACT_ITEM, CONTRACT_PAYOUT_MONEY, CONTRACT_QUANTITY};
use sim_core::sim::types::Fx;
use sim_core::sim::{FLUID_TANK_CAPACITY, PlayerCommand, RecipeRegistry, World};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
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

#[test]
fn full_chain_delivers_against_the_contract_and_pays_out_exactly_the_contract_payout() {
    let registry = recipes();
    let final_item = registry.ordered().last().unwrap().id.clone();
    assert_eq!(
        final_item, CONTRACT_ITEM,
        "this test assumes the hardcoded contract targets the chain's final product"
    );

    let mut world = World::new(recipes());
    let mut collected = 0u32;

    // Keep feeding raw tomatoes in and collecting finished output as it
    // appears until the full contract mix (CONTRACT_QUANTITY units) has been
    // collected — mirrors core_loop.rs's own spawn-then-tick pattern, just
    // run long enough for more than one unit to complete the whole chain.
    for _ in 0..500_000u32 {
        if collected < CONTRACT_QUANTITY {
            // Ticket 3: keep the hopper topped up rather than spawning onto
            // the belt. `FillHopper` clamps at capacity, so issuing it every
            // tick just means "never let the feed run dry" — the hopper
            // still meters onto the belt at one unit per tick at most, so
            // the chain's throughput is unchanged by this.
            world.issue_command(PlayerCommand::FillHopper {
                amount: CONTRACT_QUANTITY * 4,
            });
            // ...and keep the pipe run's vinegar tank topped up the same
            // way. Since the "Visual overhaul round 2" epic's ticket 1 the
            // tank is the run's only supply, so the fluid-gated mixing stage
            // stalls without this — and like `FillHopper` it clamps at
            // capacity, so issuing it every tick just means "never let the
            // supply run dry."
            world.issue_command(PlayerCommand::FillFluidTank {
                amount: FLUID_TANK_CAPACITY,
            });
        }

        world.tick();

        let ready = final_output_count(&world, &final_item);
        if ready > 0 {
            world.issue_command(PlayerCommand::CollectOutput);
            collected += ready;
        }

        if collected >= CONTRACT_QUANTITY {
            break;
        }
    }

    assert!(
        collected >= CONTRACT_QUANTITY,
        "expected to collect at least {CONTRACT_QUANTITY} units of '{final_item}' well within 500k ticks, got {collected}"
    );

    // The full mix is now sitting in the delivery buffer, but the exact tick
    // it landed on may fall anywhere relative to a cadence boundary — the
    // collection above could have completed just *after* `Economy::tick`
    // already ran (and missed) this cadence window's check. Advance a full
    // extra cadence period to guarantee at least one boundary is checked
    // with the complete mix already deposited.
    let target_tick = world.current_tick() + CONTRACT_CADENCE_TICKS;
    while world.current_tick() < target_tick {
        world.tick();
    }

    assert_eq!(
        world.money(),
        Fx::from_num(CONTRACT_PAYOUT_MONEY),
        "money must increase by exactly one contract payout once the mix is delivered"
    );
}

#[test]
fn money_stays_zero_through_a_cadence_boundary_with_nothing_delivered() {
    let mut world = World::new(recipes());

    // Advance well past the first cadence boundary without ever collecting
    // anything — the contract's mix is never satisfied, so no payout should
    // ever land, no matter how long the chain sits idle.
    for _ in 0..=CONTRACT_CADENCE_TICKS {
        world.tick();
    }

    assert_eq!(
        world.money(),
        Fx::from_num(0),
        "money must not increase when the delivered mix doesn't satisfy the contract"
    );
}
