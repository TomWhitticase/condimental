//! The P screen's pay cycle (ticket 7, 2026-09-26; `sim::stats`'s module
//! doc): per demanded product, what the active contracts require over one
//! payday period, and what the factory made this cycle and last. Proved
//! against the shipped data through the `World` API `SimBridge` exposes:
//! the cycle IS the staff payday (not a new clock), a payday rolls this
//! cycle into last and starts again at zero, "required" is the active
//! contracts' mixes scaled to the period, a world with no demand lists
//! nothing, and all of it survives a save.

mod common;

use sim_core::sim::{
    PlayerCommand, RecipeRegistry, World, bbq_branch, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

const PRODUCT: &str = "boxed_ketchup";

/// The reference line, funded and stocked, with `corner_shop` running —
/// signed if the build's starters did not already make it active, so the
/// test does not lean on which contracts a new game starts with.
fn line() -> World {
    let mut world =
        reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE + 100);
    if world.economy().contract("corner_shop").is_none() {
        world.issue_command(PlayerCommand::SignContract {
            id: "corner_shop".to_string(),
        });
    }
    assert!(world.economy().contract("corner_shop").is_some());
    common::stock_up(&mut world);
    world
}

/// What the active contracts ask of `item` over `period` ticks, from the
/// economy's own wire — the independent sum `required` is checked against.
fn expected_required(world: &World, item: &str, period: u64) -> f64 {
    world
        .economy_snapshot()
        .contracts
        .iter()
        .flat_map(|c| {
            c.mix
                .iter()
                .filter(|(id, _)| id == item)
                .map(move |(_, qty)| f64::from(*qty) * period as f64 / c.cadence_ticks as f64)
        })
        .sum()
}

#[test]
fn a_payday_rolls_this_cycle_into_last_and_required_is_the_contracts_summed() {
    let mut world = line();
    let period = world.staff().catalog().pay_period_ticks();
    let snap = world.production_snapshot().cycle;
    assert_eq!(snap.period_ticks, period, "the cycle is the payday period");
    assert_eq!(snap.next_tick, world.staff().next_payday_tick());
    assert_eq!(snap.cycles_closed, 0);

    // Three paydays of a running line. At every payday the closed cycle's
    // count is exactly what the lifetime total grew by since the one
    // before, and the new cycle starts at zero.
    let mut total_at_close = 0u64;
    let mut closes = 0;
    let mut made_any = false;
    // Bounded, so a payday that stops closing the cycle fails by name
    // rather than hanging the test binary.
    for _ in 0..period * 4 {
        if closes == 3 {
            break;
        }
        common::supply(&mut world);
        let before = world.production().cycles_closed();
        world.tick();
        let ledger = world.production();
        if ledger.cycles_closed() == before {
            continue;
        }
        closes += 1;
        let total = ledger.produced_total(PRODUCT);
        assert_eq!(
            u64::from(ledger.produced_last_cycle(PRODUCT)),
            total - total_at_close,
            "payday {closes} closed a cycle that is not what was made since the last one"
        );
        assert_eq!(ledger.produced_this_cycle(PRODUCT), 0);
        assert_eq!(
            world.current_tick() % period,
            0,
            "a close lands on a payday"
        );
        made_any |= total > total_at_close;
        total_at_close = total;
    }
    assert_eq!(
        closes,
        3,
        "three paydays in {} ticks closed only {closes} cycles",
        period * 4
    );
    assert!(
        made_any,
        "the line never boxed anything — the test proved nothing"
    );

    let snap = world.production_snapshot().cycle;
    assert_eq!(snap.cycles_closed, 3);
    let ids: Vec<&str> = snap.lines.iter().map(|l| l.id.as_str()).collect();
    let wanted = world.economy().wanted();
    assert_eq!(
        ids.len(),
        wanted.len(),
        "one line per demanded product: {ids:?}"
    );
    assert!(ids.iter().all(|id| wanted.contains(*id)), "{ids:?}");
    let line = snap.lines.iter().find(|l| l.id == PRODUCT).unwrap();
    assert!((line.required - expected_required(&world, PRODUCT, period)).abs() < 1e-3);
    assert_eq!(
        line.short,
        f64::from(line.produced_last_cycle) < line.required,
        "short is last cycle's verdict"
    );
    assert_eq!(
        line.produced_last_cycle,
        world.production().produced_last_cycle(PRODUCT)
    );
}

/// `corner_shop`'s cadence equals the pay period, so its line cannot tell a
/// scaled mix from a raw one. `smokehouse_trial` orders on a different
/// cadence: its REQUIRED is the mix times period over cadence, not the mix.
#[test]
fn required_scales_a_mix_whose_cadence_is_not_the_period() {
    // The BBQ branch built is what unlocks the smokehouse's order.
    let mut world = bbq_branch(recipes(), 20_000);
    world.issue_command(PlayerCommand::SignContract {
        id: "smokehouse_trial".to_string(),
    });
    let contract = world
        .economy_snapshot()
        .contracts
        .into_iter()
        .find(|c| c.id == "smokehouse_trial")
        .expect("smokehouse_trial signs");
    let period = world.staff().catalog().pay_period_ticks();
    assert_ne!(
        contract.cadence_ticks, period,
        "the case needs a cadence that is not the period"
    );
    let snap = world.production_snapshot().cycle;
    let line = snap
        .lines
        .iter()
        .find(|l| l.id == "boxed_bbq")
        .expect("a signed contract's product gets a cycle line");
    let expected = expected_required(&world, "boxed_bbq", period);
    let raw: u32 = contract.mix.iter().map(|(_, qty)| *qty).sum();
    assert!(
        (expected - f64::from(raw)).abs() > 1e-3,
        "the scaled and raw figures must differ"
    );
    assert!(
        (line.required - expected).abs() < 1e-3,
        "required {} is not the mix scaled to the period ({expected})",
        line.required
    );
}

#[test]
fn nothing_is_short_before_the_first_payday() {
    let world = line();
    let snap = world.production_snapshot().cycle;
    assert!(!snap.lines.is_empty());
    assert!(
        snap.lines
            .iter()
            .all(|l| !l.short && l.produced_last_cycle == 0)
    );
}

#[test]
fn a_world_with_no_active_contract_lists_no_cycle_lines() {
    let mut world = World::new(recipes());
    // Whatever a new game starts with, run the starters out of strikes:
    // nothing is built, so every window misses.
    for _ in 0..40_000 {
        if world.economy_snapshot().contracts.is_empty() {
            break;
        }
        world.tick();
    }
    assert!(world.economy_snapshot().contracts.is_empty());
    let snap = world.production_snapshot().cycle;
    assert!(
        snap.lines.is_empty(),
        "no contract, no demand: {:?}",
        snap.lines
    );
    let json = serde_json::to_string(&world.production_snapshot()).unwrap();
    assert!(json.contains("\"lines\":[]"), "{json}");
}

#[test]
fn the_cycle_counts_survive_a_save() {
    let mut world = line();
    let period = world.staff().catalog().pay_period_ticks();
    for _ in 0..(period * 2 + period / 2) {
        common::supply(&mut world);
        world.tick();
    }
    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).expect("round trip");
    assert_eq!(loaded.production().cycles_closed(), 2);
    assert_eq!(
        loaded.production().produced_this_cycle(PRODUCT),
        world.production().produced_this_cycle(PRODUCT)
    );
    assert_eq!(
        loaded.production().produced_last_cycle(PRODUCT),
        world.production().produced_last_cycle(PRODUCT)
    );
    assert_eq!(loaded.production_snapshot(), world.production_snapshot());
}
