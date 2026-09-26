//! The **third sauce**, end to end: `sim::mustard_line` — the shipped ketchup
//! line and mustard's own five stages in one world — built out of nothing but
//! `Purchase` and `Place`, run until a box of mustard comes off the packer and
//! the dock ships it.
//!
//! `data/recipes.ron` parsing is not evidence that the factory it describes can
//! be built and run, and mustard's risk is entirely in the plumbing rather than
//! in the content. Mustard buys `vinegar`, which the ketchup mixer already
//! drinks, and `World::resolve_network_fluids` does not ask what recipe a
//! machine is running before deciding what a pipe run beside it could carry:
//! it seeds a run from **every** recipe of every kind that touches it. Put
//! mustard's vinegar on the mixer and a tank run touching any mixer has two raw
//! candidates, narrows to neither, and goes `Ambiguous` — at which point every
//! machine on it stalls with `AmbiguousPipe` **and the tank cannot even be
//! filled**, because `World::tank_fluid` has nothing to return. That is a
//! shipped-line regression a new player would hit before ever unlocking
//! mustard, since `for_machine` is not gate-aware.
//!
//! So mustard's vinegar enters at the **crusher**, which drank nothing, and the
//! mustard mixer drinks the derived `mustard_mash` down a pipe. The bill per
//! box is identical; only the machine that drinks moved. The rule that came out
//! of it — at most one raw fluid per machine kind, drunk by at most one recipe
//! on that kind — is fenced by
//! `sim::recipe::tests::at_most_one_raw_fluid_enters_any_machine_kind`, which
//! reads the data. This file is the other half of the evidence: the running
//! factory that fence is protecting.
//!
//! Three claims:
//!
//! 1. **Every pipe run in a two-line factory resolves**, from the tick it is
//!    laid, and no machine ever stalls on `AmbiguousPipe` or `UnnameablePipe`.
//! 2. **It works.** A `boxed_mustard` is crafted and the dock delivers it
//!    against a mustard contract.
//! 3. **The ketchup line still runs** with mustard's line beside it drinking
//!    from the same market.

mod common;

use sim_core::sim::fluid::PipeNaming;
use sim_core::sim::world::StallKind;
use sim_core::sim::{
    FLUID_TANK_CAPACITY, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, World, mustard_line,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A long run's worth of every raw the two lines between them consume.
/// Generous rather than exact: nothing here measures earnings, and a factory
/// that runs out of stock halfway would quietly become the thing under test.
const STOCK: [(&str, u32); 5] = [
    ("tomato", 3000),
    ("sugar", 600),
    ("vinegar", 2000),
    ("mustard_seed", 3000),
    ("turmeric", 600),
];

/// Enough to stock up, warehouse it, sign a contract and pay an enlarged crew
/// for the whole run, with room to spare — `tests/bbq_branch.rs`'s reasoning
/// for not sizing it to land on zero applies here too.
const BUDGET: u32 = 30_000;

/// The whole factory — both lines — stocked, crewed and ready to run.
fn factory() -> World {
    let mut world = mustard_line(recipes(), BUDGET);
    for (item, quantity) in STOCK {
        common::buy(&mut world, item, quantity);
    }
    staff_up(&mut world);
    world
}

/// Hires until no category is short: a second line leaves every category
/// understaffed, which slows *every* machine in the factory and would wreck
/// any measurement taken here (GAMEPLAY.md §7).
fn staff_up(world: &mut World) {
    for _ in 0..64 {
        let short: Vec<String> = world
            .staff_snapshot()
            .categories
            .iter()
            .filter(|category| category.short > 0)
            .map(|category| category.id.clone())
            .collect();
        if short.is_empty() {
            return;
        }
        for category in short {
            world.issue_command(PlayerCommand::Hire { category });
        }
    }
    panic!("the fixture must be able to fully staff itself");
}

/// One tick of keeping every hopper and every tank topped up.
fn supply(world: &mut World) {
    world.issue_command(PlayerCommand::FillHopper {
        amount: HOPPER_CAPACITY,
        tile: None,
    });
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });
}

fn made(world: &World, id: &str) -> u64 {
    world
        .production_snapshot()
        .items
        .iter()
        .find(|item| item.id == id)
        .map_or(0, |item| item.produced_total)
}

/// Every run, named, with the fluid it carries — the sentence a failure should
/// print, since "some run is ambiguous" is not enough to find it by.
fn runs(world: &World) -> Vec<(PipeNaming, Option<String>, (i32, i32))> {
    world
        .fluid_snapshots()
        .into_iter()
        .map(|run| {
            let head = run.segments.first().map_or((0, 0), |segment| segment.tile);
            (run.naming, run.fluid.clone(), head)
        })
        .collect()
}

/// Whichever machine is stalled on a pipe it cannot read, if any.
fn pipe_faults(world: &World) -> Vec<((i32, i32), StallKind)> {
    world
        .machine_load_snapshots()
        .into_iter()
        .filter_map(|entry| entry.stall.map(|stall| (entry.tile, stall.kind)))
        .filter(|(_, kind)| matches!(kind, StallKind::AmbiguousPipe | StallKind::UnnameablePipe))
        .collect()
}

/// Claim 1, before a single tick: two lines whose mixers, vats and bottlers are
/// the same kinds, one of them drinking a bought fluid at the crusher, and
/// every run still narrows to exactly one fluid.
#[test]
fn every_run_of_a_two_line_factory_names_one_fluid() {
    let world = mustard_line(recipes(), BUDGET);
    for (naming, fluid, head) in runs(&world) {
        assert_eq!(
            naming,
            PipeNaming::Carries,
            "the run at {head:?} did not resolve — the whole factory reads {:?}",
            runs(&world)
        );
        assert!(fluid.is_some());
    }
    assert!(
        pipe_faults(&world).is_empty(),
        "no machine may stall on an unreadable pipe: {:?}",
        pipe_faults(&world)
    );

    // And the vinegar the two lines compete for reaches both of them: one run
    // per line, each carrying it, because each line has its own tank.
    let vinegar = runs(&world)
        .into_iter()
        .filter(|(_, fluid, _)| fluid.as_deref() == Some("vinegar"))
        .count();
    assert_eq!(
        vinegar,
        2,
        "each line has its own vinegar tank and its own run — {:?}",
        runs(&world)
    );
}

/// Claims 2 and 3: the factory runs, mustard ships, and ketchup keeps going.
#[test]
fn a_box_of_mustard_is_made_and_the_dock_ships_it() {
    let mut world = factory();
    world.issue_command(PlayerCommand::SignContract {
        id: "deli_counter".to_string(),
    });

    let mut shipped_at = None;
    for tick in 0..30_000u64 {
        supply(&mut world);
        world.tick();
        if shipped_at.is_none()
            && world
                .economy()
                .contract("deli_counter")
                .is_some_and(|contract| contract.deliveries_made > 0)
        {
            shipped_at = Some(tick);
        }
        // Claim 1 again, on a running factory rather than an empty one: a run
        // is re-resolved on every placement and every tank change, and a
        // fault that only appeared once fluid was moving would be invisible
        // to the check above. Sampled rather than every tick, because the
        // snapshots cost more than the tick does.
        if tick % 500 == 0 {
            assert!(
                pipe_faults(&world).is_empty(),
                "a pipe went unreadable at tick {tick}: {:?} — runs read {:?}",
                pipe_faults(&world),
                runs(&world)
            );
        }
    }

    // Every stage, so a failure says which one stopped rather than only that
    // the last one did.
    for stage in [
        "mustard_mash",
        "mustard_mix",
        "mustard",
        "bottled_mustard",
        "boxed_mustard",
    ] {
        assert!(
            made(&world, stage) > 0,
            "the mustard line must craft '{stage}' — it made {:?}",
            world
                .production_snapshot()
                .items
                .iter()
                .map(|item| (item.id.clone(), item.produced_total))
                .collect::<Vec<_>>()
        );
    }
    assert!(
        shipped_at.is_some(),
        "the Deli Counter order must settle against boxes the dock delivered by itself — \
         {} boxes were made",
        made(&world, "boxed_mustard")
    );
    assert!(
        made(&world, "boxed_ketchup") > 0,
        "the shipped ketchup line must still run with a mustard line beside it"
    );
}
