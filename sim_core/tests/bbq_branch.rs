//! The **second sauce**, end to end: `sim::bbq_branch` built out of nothing
//! but `Purchase` and `Place`, run until a box of BBQ comes off the packer
//! and the dock ships it.
//!
//! `data/recipes.ron` can be made to load with almost anything in it. That a
//! file parses is not evidence that the factory it describes can be *built*
//! and *run*, and the BBQ epic is mostly a plumbing problem rather than a
//! content one: a branch draws a finished fluid back out of the middle of the
//! line that makes it, a machine kind now runs two recipes, and a pipe run
//! that cannot work out what it carries leaves its machines silently stalled
//! rather than failing loudly (`world.rs`'s "What a pipe run carries"). Every
//! one of those goes wrong quietly. So this file runs the thing.
//!
//! It asserts three separate claims, and the middle one is the reason the
//! epic's numbers are what they are:
//!
//! 1. **It works.** A `boxed_bbq` is crafted and delivered.
//! 2. **The split is 3:2.** The ketchup vat is the only source of ketchup,
//!    and the BBQ branch drinks from it, so the two sauces divide one vat's
//!    output between them. `docs/BBQ_SAUCE_PLAN.md` §2 picks the BBQ cook
//!    time so that division comes out at three boxes of ketchup to two of
//!    BBQ, and `data/contracts.ron`'s mixed orders ask for exactly that
//!    ratio. This measures it on a running factory.
//! 3. **The shipped line did not move.** Every placement of
//!    `reference_chain` is still in `bbq_branch`, on the same tile, facing
//!    the same way.

use std::collections::BTreeMap;

use sim_core::sim::{
    FLUID_TANK_CAPACITY, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, World, bbq_branch,
    reference_chain,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("data/recipes.ron must parse")
}

/// A long run's worth of every raw the two lines between them consume, in
/// `data/ingredients.ron` order. Generous rather than exact — the 60 000-tick
/// loop below uses roughly a third of each.
const STOCK: [(&str, u32); 6] = [
    ("tomato", 3000),
    ("sugar", 600),
    ("vinegar", 1200),
    ("spice_mix", 400),
    ("molasses", 400),
    ("liquid_smoke", 600),
];
/// What [`stock_up`] spends at `data/ingredients.ron`'s current prices — a
/// literal, asserted below, so a price change fails here with a message about
/// this fixture's bill rather than as an unexplained idle factory further
/// down. (`tests/common/mod.rs` keeps its own for the same reason; this file
/// buys a different basket, so it needs its own number.)
const STOCK_COST: u32 = 12_600;
/// Enough to stock up, sign every BBQ contract and pay the enlarged crew for
/// the whole run, with room to spare. Unlike `tests/common/mod.rs` this one
/// is deliberately *not* sized to land on zero: nothing here measures
/// earnings, and a fixture that runs out of money loses its staff, which
/// would quietly become the thing being measured.
const BUDGET: u32 = 20_000;

/// The whole factory, stocked, crewed and ready to run.
fn branch() -> World {
    let mut world = bbq_branch(recipes(), BUDGET);
    stock_up(&mut world);
    staff_up(&mut world);
    world
}

fn stock_up(world: &mut World) {
    let before = world.money();
    for (item, quantity) in STOCK {
        world.issue_command(PlayerCommand::BuyIngredient {
            item: item.to_string(),
            quantity,
        });
        assert_eq!(
            world.owned_ingredient(item),
            quantity,
            "the fixture must be able to afford {quantity} x {item}"
        );
    }
    assert_eq!(
        before - world.money(),
        sim_core::sim::types::Money::from_num(STOCK_COST),
        "STOCK_COST must match what data/ingredients.ron actually charges for {STOCK:?}"
    );
}

/// Hires until no category is short.
///
/// The starting crew is sized for the shipped line alone, so a second line
/// leaves manufacturing, packaging and logistics understaffed and *every*
/// machine in the factory — the ketchup ones included — running slow. That is
/// the staffing mechanic working (GAMEPLAY.md §7), and it is exactly the
/// thing that would wreck a throughput measurement, so this fixture pays for
/// the heads it needs before measuring anything.
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

/// One tick of keeping every hopper and both tanks topped up.
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

#[test]
fn a_box_of_bbq_is_made_and_the_dock_ships_it() {
    let mut world = branch();
    world.issue_command(PlayerCommand::SignContract {
        id: "smokehouse_trial".to_string(),
    });

    let mut shipped_at = None;
    for tick in 0..30_000u64 {
        supply(&mut world);
        world.tick();
        if shipped_at.is_none()
            && world
                .economy()
                .contract("smokehouse_trial")
                .is_some_and(|c| c.deliveries_made > 0)
        {
            shipped_at = Some(tick);
        }
    }

    // Every stage of the branch, so a failure says which one stopped rather
    // than only that the last one did.
    for stage in ["bbq_mix", "bbq", "bottled_bbq", "boxed_bbq"] {
        assert!(
            made(&world, stage) > 0,
            "the BBQ branch must craft '{stage}' — it made {:?}",
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
        "the Smokehouse Trial must settle against boxes the dock delivered by itself — \
         {} boxes were made",
        made(&world, "boxed_bbq")
    );
    assert!(
        made(&world, "boxed_ketchup") > 0,
        "the shipped ketchup line must still run with the branch drinking from it"
    );
}

#[test]
fn the_two_sauces_split_one_vat_three_to_two() {
    let mut world = branch();
    // Every BBQ contract, so the delivery pool keeps draining: a full pool
    // back-pressures the packer, and a stalled packer would be what this
    // measured instead of the plumbing.
    for contract in ["smokehouse_trial", "grill_pub", "roadhouse_group"] {
        world.issue_command(PlayerCommand::SignContract {
            id: contract.to_string(),
        });
    }

    // Measure a *rate*, over a window that starts well after the branch has
    // primed its pipes. The first few thousand ticks are spent filling two
    // new pipe runs, and that priming is ketchup the branch drinks and never
    // gives back — a cumulative count taken from tick zero reads the
    // transient, not the ratio.
    let mut start = BTreeMap::new();
    for tick in 0..36_000u64 {
        supply(&mut world);
        world.tick();
        if tick == 12_000 {
            start.insert("bottled_ketchup", made(&world, "bottled_ketchup"));
            start.insert("bbq_mix", made(&world, "bbq_mix"));
        }
    }

    // One `bottled_ketchup` is one ketchup the bottler drew; one `bbq_mix` is
    // one ketchup the branch drew. So these two counts *are* the split of the
    // vat's output, and nothing else draws from that network.
    let ketchup = made(&world, "bottled_ketchup") - start["bottled_ketchup"];
    let bbq = made(&world, "bbq_mix") - start["bbq_mix"];
    assert!(
        ketchup > 0 && bbq > 0,
        "both lines must be drawing from the vat — ketchup {ketchup}, bbq {bbq}"
    );
    let ratio = ketchup as f64 / bbq as f64;
    assert!(
        (ratio - 1.5).abs() < 0.1,
        "the ketchup vat's output must divide three boxes of ketchup to two of BBQ: the vat \
         makes one ketchup per 240 ticks, the BBQ branch's 600-tick cook takes one per 600, \
         and the 2.5-per-1000 remainder is what reaches the ketchup bottler. Measured \
         {ketchup} ketchup to {bbq} BBQ over 24 000 ticks, a ratio of {ratio:.2} rather than \
         1.50. data/contracts.ron's mixed orders ask for 3:2 because of this number — if the \
         cook time moved, they have to move with it."
    );
}

#[test]
fn every_run_in_the_branch_knows_what_it_carries() {
    let world = branch();
    let carried: Vec<Option<String>> = world
        .fluid_snapshots()
        .iter()
        .map(|network| network.fluid.clone())
        .collect();
    assert_eq!(
        carried.len(),
        6,
        "the branch has six separate pipe runs: vinegar, ketchup mix and ketchup on the \
         shipped line, BBQ mix, liquid smoke and BBQ on the branch"
    );
    assert!(
        carried.iter().all(Option::is_some),
        "every run must resolve to one fluid. An unnamed run is not a panic — its machines \
         simply never draw and never deposit (world.rs, 'What a pipe run carries') — so a \
         branch plumbed one tile out of place looks like a factory that is merely idle. \
         Runs resolved to: {carried:?}"
    );
}

#[test]
fn the_branch_leaves_the_shipped_line_exactly_where_it_was() {
    let shipped: Vec<_> = reference_chain(recipes())
        .world_layout()
        .iter()
        .map(|p| (p.kind.clone(), p.occupies.clone(), p.direction))
        .collect();
    let branched: Vec<_> = bbq_branch(recipes(), 0)
        .world_layout()
        .iter()
        .map(|p| (p.kind.clone(), p.occupies.clone(), p.direction))
        .collect();
    for placement in &shipped {
        assert!(
            branched.contains(placement),
            "the BBQ branch must leave the shipped line alone, and {placement:?} moved. The \
             branch hangs off the ketchup line's outlet run; it must never rearrange it."
        );
    }
    assert!(
        branched.len() > shipped.len(),
        "the branch must add to the shipped line, not replace it"
    );
}

/// The recipe the machine on `tile` is holding.
fn recipe_at(world: &World, tile: (i32, i32)) -> String {
    world
        .world_layout()
        .into_iter()
        .find(|placement| placement.tile == tile.into())
        .and_then(|placement| placement.recipe)
        .unwrap_or_else(|| panic!("no machine at {tile:?}"))
}

/// **A BBQ machine that runs dry stays a BBQ machine.**
///
/// `Machine::choose` re-picks an idle machine's recipe every tick from what
/// its buffers and taps hold, and when nothing is satisfied it holds the
/// closest. A bottler that has drained its tap and emptied its buffer
/// satisfies *nothing*, so every candidate ties at zero — and the tie used
/// to go to whichever came first in `data/recipes.ron`, which is
/// `bottled_ketchup`. The BBQ bottler therefore reported ketchup the moment
/// it caught up with its supply, which is a lie in two visible places: the
/// stall readout said it was waiting for ketchup it has never touched, and
/// the machine drew a row of ketchup bottles (`PRODUCT_TINTS` in
/// `godot_project/main.gd`, 2026-09-21).
///
/// Asserted on a bottler AND a packer because they starve differently — one
/// is fed by a pipe and one by a belt — and against a run long enough for
/// the branch to be genuinely idle rather than momentarily between crafts.
#[test]
fn a_bbq_machine_that_runs_dry_keeps_its_own_recipe() {
    let mut world = branch();
    for _ in 0..20_000u64 {
        supply(&mut world);
        world.tick();
    }
    assert!(
        made(&world, "boxed_bbq") > 0,
        "the branch has to have RUN before it can be asked what it runs"
    );
    assert_eq!(recipe_at(&world, (21, -3)), "bottled_bbq");
    assert_eq!(recipe_at(&world, (26, -3)), "boxed_bbq");

    // Now starve it: no more raws, and long enough for every pipe, buffer
    // and belt in the branch to empty out.
    for _ in 0..20_000u64 {
        world.tick();
    }
    assert_eq!(
        recipe_at(&world, (21, -3)),
        "bottled_bbq",
        "an empty BBQ bottler must not report itself as a ketchup bottler"
    );
    assert_eq!(
        recipe_at(&world, (26, -3)),
        "boxed_bbq",
        "nor an empty BBQ packer"
    );
    // The ketchup line is the control: it holds its own recipes throughout,
    // which is what says the rule is "keep what you hold" and not "keep BBQ".
    assert_eq!(recipe_at(&world, (16, 0)), "bottled_ketchup");
    assert_eq!(recipe_at(&world, (21, 0)), "boxed_ketchup");
}
