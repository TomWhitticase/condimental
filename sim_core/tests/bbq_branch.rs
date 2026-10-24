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

mod common;

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
/// What the warehouse under that basket costs: 21 racks at `data/shop.ron`'s
/// `warehouse_rack` price. `common::buy` puts down as many as the order in
/// front of it needs (`data/storage.ron` caps a holding at `base_cap` plus
/// the bays that exist), and this basket comes to 124 bays. A literal for
/// the same reason [`STOCK_COST`] is one.
const RACK_COST: u32 = 1_575;
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
        common::buy(world, item, quantity);
    }
    assert_eq!(
        before - world.money(),
        sim_core::sim::types::Money::from_num(STOCK_COST + RACK_COST),
        "STOCK_COST + RACK_COST must match what data/ingredients.ron and data/shop.ron \
         actually charge for {STOCK:?} and the warehouse to hold it"
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
    // Every BBQ contract, so the BBQ dock keeps draining: a full dock
    // back-pressures the packer, and a stalled packer would be what this
    // measured instead of the plumbing.
    for contract in ["smokehouse_trial", "grill_pub", "roadhouse_group"] {
        world.issue_command(PlayerCommand::SignContract {
            id: contract.to_string(),
        });
    }

    // The tanks top themselves up in whole units. `supply`'s manual fill bills
    // a part unit as a whole one every tick, which runs the vinegar out long
    // before a run this long ends.
    for tank in world.fluid_tank_snapshots() {
        world.issue_command(PlayerCommand::SetTankAutoFill {
            tile: tank.tile,
            enabled: true,
        });
    }
    let fill_hoppers = |world: &mut World| {
        world.issue_command(PlayerCommand::FillHopper {
            amount: HOPPER_CAPACITY,
            tile: None,
        });
    };

    // Measure a *rate*, over a window that starts after the branch has primed
    // its pipes. Priming is ketchup the branch drinks and never gives back,
    // and since issue #93 the ketchup bottler keeps an equal share while it
    // happens, so it stretches to some 30 000 ticks — past where the ketchup
    // dock, shipping to the Corner Shop alone, backs the line up. So the
    // ketchup bottler is switched off until the BBQ mixer's run is full,
    // which is the old full-rate priming done on purpose, and the ketchup
    // board is signed when it comes back on so its dock keeps draining.
    world.issue_command(PlayerCommand::SetMothballed {
        tile: KETCHUP_BOTTLER,
        enabled: true,
    });
    let mixed = |world: &World| -> f64 {
        world
            .fluid_snapshots()
            .iter()
            .filter(|run| run.fluid.as_deref() == Some("bbq_mix"))
            .map(|run| run.total_volume)
            .sum()
    };
    let mut primed = false;
    for _ in 0..20_000u64 {
        fill_hoppers(&mut world);
        world.tick();
        if mixed(&world) >= 18.0 {
            primed = true;
            break;
        }
    }
    assert!(
        primed,
        "the BBQ mixer's run must fill while it has the vat to itself"
    );
    world.issue_command(PlayerCommand::SetMothballed {
        tile: KETCHUP_BOTTLER,
        enabled: false,
    });
    for contract in ["corner_shop", "market_stall"] {
        world.issue_command(PlayerCommand::SignContract {
            id: contract.to_string(),
        });
    }

    let mut start = BTreeMap::new();
    for tick in 0..26_000u64 {
        fill_hoppers(&mut world);
        world.tick();
        if tick == 2_000 {
            start.insert("bottled_ketchup", made(&world, "bottled_ketchup"));
            start.insert("bbq_mix", made(&world, "bbq_mix"));
        }
    }

    // One bottler fill is one ketchup the bottler drew, and it makes a case's
    // worth of bottles; one `bbq_mix` is one ketchup the branch drew. So the
    // bottles over the fill size and the mixes *are* the split of the vat's
    // output, and nothing else draws from that network. The fill size is
    // read from the recipe rather than written here, so a case resized in
    // `data/recipes.ron` cannot quietly skew the ratio.
    let per_fill = recipes()
        .get("bottled_ketchup")
        .expect("the ketchup bottler's recipe")
        .outputs
        .iter()
        .find(|(item, _)| item == "bottled_ketchup")
        .map(|(_, n)| u64::from(*n))
        .expect("the bottler makes bottled ketchup");
    let ketchup = (made(&world, "bottled_ketchup") - start["bottled_ketchup"]) / per_fill;
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
        .find(|placement| placement.tile == tile)
        .and_then(|placement| placement.recipe)
        .unwrap_or_else(|| panic!("no machine at {tile:?}"))
}

/// **A BBQ machine that runs dry stays a BBQ machine.**
///
/// A machine runs the recipe the player set it to (2026-10-03) and never
/// re-picks. Before that, a bottler that had drained its tap tied every
/// candidate at zero and went back to the first in `data/recipes.ron`,
/// `bottled_ketchup`, so a BBQ bottler reported ketchup and drew ketchup
/// bottles the moment it caught up with its supply (`PRODUCT_TINTS` in
/// `godot_project/main.gd`, 2026-09-21). The guard stays: a chosen recipe
/// must survive the line running dry.
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

const KETCHUP_DOCK: (i32, i32) = (26, 0);
const KETCHUP_BOTTLER: (i32, i32) = (16, 0);
const BBQ_DOCK: (i32, i32) = (31, -3);

/// The branch run until its ketchup dock holds a few boxes, then with the
/// ketchup board signed: the Corner Shop's two a thousand ticks plus the
/// Market Stall's one a fifteen hundred out-draw the 2.5 a thousand the
/// shipped line keeps after the branch takes its share
/// (`the_two_sauces_split_one_vat_three_to_two`), so the ketchup dock
/// drains while nothing on the board asks for BBQ. Signed any earlier, the
/// windows fall due while the pipes are still priming and strike the
/// contracts off before a box arrives.
fn ketchup_board() -> World {
    let mut world = branch();
    for _ in 0..30_000u32 {
        supply(&mut world);
        world.tick();
        if dock_at(&world, KETCHUP_DOCK).used >= 4 {
            break;
        }
    }
    assert!(
        dock_at(&world, KETCHUP_DOCK).used >= 4,
        "the shipped line must reach its dock"
    );
    for contract in ["corner_shop", "market_stall"] {
        world.issue_command(PlayerCommand::SignContract {
            id: contract.to_string(),
        });
        assert!(
            world.economy().contract(contract).is_some(),
            "the fixture can sign '{contract}'"
        );
    }
    world
}

fn dock_at(world: &World, tile: (i32, i32)) -> sim_core::sim::world::DockSnapshot {
    world
        .dock_snapshots()
        .into_iter()
        .find(|dock| dock.tile == tile)
        .unwrap_or_else(|| panic!("the branch fixture has a dock at {tile:?}"))
}

/// The regression the per-dock lamp exists for: with two docks shipping two
/// products, the one that fills must leave the other reporting itself clear.
///
/// Each dock holds its own stock, so the facts are per dock by construction:
/// the BBQ dock filling its places is no box on the ketchup dock. Before
/// that, both docks fed one shared pool, and a pool capped on either product
/// once made BOTH lamps pulse.
///
/// BBQ is the one that fills here, not ketchup, and that is the fixture
/// rather than a choice: the branch out-produces the Smokehouse Trial's
/// appetite, while the shipped line is throttled to two fifths of the shared
/// vat and runs its larder down long before its own ceiling.
///
/// Note what is asserted in the middle: `delivery_full` is true. It is the
/// board's summary — SOME dock is at capacity — and never a per-dock fact.
#[test]
fn a_full_bbq_dock_leaves_the_ketchup_dock_reporting_clear() {
    let mut world = ketchup_board();

    // Run the branch until its dock is backing up. The budget is loose
    // because the number is a consequence of every rate in `data/`, not
    // something this test is pinning.
    let mut jammed = false;
    for _ in 0..60_000u32 {
        supply(&mut world);
        world.tick();
        if dock_at(&world, BBQ_DOCK).full {
            jammed = true;
            break;
        }
    }
    assert!(
        jammed,
        "an under-demanded BBQ line must eventually back its dock up — it never did, so \
         either the line stopped or `full` is not being raised"
    );
    let bbq = dock_at(&world, BBQ_DOCK);
    assert!(
        bbq.at_capacity,
        "a jammed dock has every place taken: {bbq:?}"
    );

    let economy = world.economy_snapshot();
    assert!(
        economy.delivery_full,
        "the board's summary says some dock is at capacity: {:?}",
        economy.delivery_stock
    );
    let ketchup = dock_at(&world, KETCHUP_DOCK);
    assert!(
        !ketchup.full && !ketchup.at_capacity,
        "the ketchup dock is not the one backing up: {ketchup:?}"
    );
    assert!(
        ketchup.contents.iter().all(|(item, _)| item != "boxed_bbq"),
        "and not one BBQ box stands on it: {ketchup:?}"
    );
}

/// A payment names the docks it took from, and only those: a ketchup-only
/// contract paid while BBQ sits on the other dock leaves the BBQ dock's
/// `last_paid_tick` untouched — the fact the coin burst reads to throw its
/// coins over the docks that paid and no other.
#[test]
fn a_ketchup_payment_is_paid_from_the_ketchup_dock_alone() {
    let mut world = ketchup_board();
    let made_before = |world: &World| {
        world
            .economy()
            .contract("corner_shop")
            .map_or(0, |c| c.deliveries_made)
    };

    let mut paid_on = None;
    for _ in 0..30_000u32 {
        supply(&mut world);
        let before = made_before(&world);
        world.tick();
        if made_before(&world) > before && !dock_at(&world, BBQ_DOCK).contents.is_empty() {
            paid_on = Some(world.current_tick());
            break;
        }
    }
    let paid_on = paid_on.expect("a corner-shop window must pay while BBQ stands on its dock");

    assert_eq!(
        dock_at(&world, KETCHUP_DOCK).last_paid_tick,
        Some(paid_on),
        "the ketchup dock paid, on that tick"
    );
    assert_eq!(
        dock_at(&world, BBQ_DOCK).last_paid_tick,
        None,
        "the BBQ dock paid nothing: no contract on the board asks for BBQ"
    );
}

/// Two docks on the one logistics head a new game starts with, one of them
/// holding BBQ nobody has ordered. A dock claims its crew only while it
/// holds something an active contract orders (`World::tick` step 6), so the
/// head goes to the ketchup dock whose stock is wanted. Claiming it on every
/// tick a dock held anything gave the first dock in topology order the head
/// forever: the BBQ dock's unordered stock kept the ketchup dock
/// `understaffed` on every tick and its cases unsold (PR #182 review).
#[test]
fn two_docks_share_the_starting_logistics_crew() {
    let mut world = bbq_branch(recipes(), BUDGET);
    stock_up(&mut world);
    let logistics = |world: &World| {
        world
            .staff_snapshot()
            .categories
            .into_iter()
            .find(|category| category.id == "logistics")
            .expect("data/staff.ron has a logistics category")
    };
    assert_eq!(
        logistics(&world).headcount,
        1,
        "the fixture runs on the starting logistics crew of one: {:?}",
        logistics(&world)
    );
    for _ in 0..100_000u32 {
        supply(&mut world);
        world.tick();
        if dock_at(&world, KETCHUP_DOCK).used >= 4 && !dock_at(&world, BBQ_DOCK).contents.is_empty()
        {
            break;
        }
    }
    assert!(
        dock_at(&world, KETCHUP_DOCK).used >= 4 && !dock_at(&world, BBQ_DOCK).contents.is_empty(),
        "both lines must reach their docks on the starting crew: ketchup {:?}, bbq {:?} at tick {}",
        dock_at(&world, KETCHUP_DOCK),
        dock_at(&world, BBQ_DOCK),
        world.current_tick()
    );
    world.issue_command(PlayerCommand::SignContract {
        id: "corner_shop".to_string(),
    });
    assert!(world.economy().contract("corner_shop").is_some());

    let signed_on = world.current_tick();

    let mut ketchup_short = 0u32;
    for _ in 0..20_000u32 {
        supply(&mut world);
        world.tick();
        if dock_at(&world, KETCHUP_DOCK).understaffed {
            ketchup_short += 1;
        }
    }
    assert_eq!(
        ketchup_short, 0,
        "the BBQ dock's unordered stock must not hold the one logistics head"
    );
    let ketchup = dock_at(&world, KETCHUP_DOCK);
    assert!(
        ketchup.last_paid_tick.is_some_and(|tick| tick > signed_on),
        "the ketchup dock ships on the shared head: {ketchup:?}"
    );
    assert!(
        !dock_at(&world, BBQ_DOCK).contents.is_empty(),
        "and the BBQ dock still holds what nobody ordered"
    );
    assert!(
        !dock_at(&world, BBQ_DOCK).understaffed,
        "asking for nobody, so it reports no shortage"
    );
}

/// The other half of the crew rule: two docks that BOTH hold ordered stock
/// need two logistics heads, one each, exactly as the establishment
/// (`required`) says. On one head the dock earlier in topology order takes
/// it and the other is refused on every tick. The staff snapshot names the
/// shortage, so the fix is in front of the player: hire one. A deliberate
/// rule (DECISIONS 2026-09-26), pinned so it cannot drift silently.
#[test]
fn two_docks_both_holding_ordered_stock_need_two_heads() {
    let mut world = bbq_branch(recipes(), BUDGET);
    stock_up(&mut world);
    let logistics = |world: &World| {
        world
            .staff_snapshot()
            .categories
            .into_iter()
            .find(|category| category.id == "logistics")
            .expect("data/staff.ron has a logistics category")
    };
    assert_eq!(logistics(&world).headcount, 1, "the starting crew of one");
    let both_stocked = |world: &World| {
        dock_at(world, KETCHUP_DOCK)
            .contents
            .iter()
            .any(|(item, _)| item == "boxed_ketchup")
            && dock_at(world, BBQ_DOCK)
                .contents
                .iter()
                .any(|(item, _)| item == "boxed_bbq")
    };
    for _ in 0..100_000u32 {
        supply(&mut world);
        world.tick();
        if both_stocked(&world) {
            break;
        }
    }
    assert!(both_stocked(&world), "both lines must reach their docks");
    for id in ["corner_shop", "smokehouse_trial"] {
        world.issue_command(PlayerCommand::SignContract { id: id.to_string() });
        assert!(
            world.economy().contract(id).is_some(),
            "the fixture can sign '{id}'"
        );
    }
    supply(&mut world);
    world.tick();

    let refused: Vec<(i32, i32)> = [KETCHUP_DOCK, BBQ_DOCK]
        .into_iter()
        .filter(|&tile| dock_at(&world, tile).understaffed)
        .collect();
    assert_eq!(
        refused.len(),
        1,
        "one head serves one of the two wanted docks and the other is refused: {refused:?}"
    );
    let crew = logistics(&world);
    assert_eq!(
        (crew.required, crew.short),
        (2, 1),
        "the staff snapshot names the missing head: {crew:?}"
    );

    world.issue_command(PlayerCommand::Hire {
        category: "logistics".to_string(),
    });
    supply(&mut world);
    world.tick();
    assert!(
        !dock_at(&world, KETCHUP_DOCK).understaffed && !dock_at(&world, BBQ_DOCK).understaffed,
        "a second head crews both docks"
    );
}

/// **A branch built onto a running line leaves that line a share** (issue
/// #93).
///
/// The BBQ mixer drinks from the run the ketchup bottler already lives on,
/// and it is the faster of the two, so while the branch fills its own pipes
/// it wants every unit the vat makes. Both draw from one run whose tiles
/// cross a whole unit at nearly the same moment, and before #93 the same
/// tile crossed first every time: the bottler made nothing for the ~8 000
/// ticks the branch took to prime, which is three missed delivery windows
/// and a lost contract (#55). Now a run another machine is waiting on serves
/// nobody until that machine's tile has filled too
/// (`World::held_networks`), so the two take turns.
///
/// Built the way a player builds it — the shipped line first, running, and
/// then the branch's pieces bought and placed on top. The pieces are
/// `bbq_branch` less `reference_chain`, so the layout is pinned in one place.
#[test]
fn a_branch_built_on_a_running_line_leaves_it_a_share_of_the_vat() {
    let branch_pieces: Vec<_> = {
        let full = bbq_branch(recipes(), BUDGET);
        let shipped: Vec<(i32, i32)> = reference_chain(recipes())
            .world_layout()
            .into_iter()
            .map(|placement| placement.tile)
            .collect();
        let hoppers = full.hopper_snapshots();
        full.world_layout()
            .into_iter()
            .filter(|placement| !shipped.contains(&placement.tile))
            .map(|placement| {
                let item = hoppers
                    .iter()
                    .find(|hopper| hopper.tile == placement.tile)
                    .map(|hopper| hopper.item.clone());
                (placement.kind, placement.tile, item, placement.recipe)
            })
            .collect()
    };

    let mut world = sim_core::sim::reference_chain_with_budget(recipes(), BUDGET);
    sim_core::sim::reference::open_every_gate(&mut world);
    stock_up(&mut world);
    staff_up(&mut world);
    // Whole-unit top-ups, for the reason the 3:2 test gives.
    let auto_fill_tanks = |world: &mut World| {
        for tank in world.fluid_tank_snapshots() {
            world.issue_command(PlayerCommand::SetTankAutoFill {
                tile: tank.tile,
                enabled: true,
            });
        }
    };
    auto_fill_tanks(&mut world);
    let fills = |world: &World| made(world, "bottled_ketchup");
    let run = |world: &mut World, ticks: u64| {
        for _ in 0..ticks {
            world.issue_command(PlayerCommand::FillHopper {
                amount: HOPPER_CAPACITY,
                tile: None,
            });
            world.tick();
        }
    };

    // The shipped line, primed and running on its own.
    run(&mut world, 6_000);
    let before = fills(&world);
    run(&mut world, 4_000);
    let alone = fills(&world) - before;
    assert!(
        alone > 0,
        "the shipped line must be bottling before the branch arrives"
    );

    for (kind, tile, item, recipe) in &branch_pieces {
        world.issue_command(PlayerCommand::Purchase {
            kind: kind.clone(),
            quantity: 1,
        });
        world.issue_command(PlayerCommand::Place {
            kind: kind.clone(),
            tile: *tile,
            direction: sim_core::sim::Direction::East,
            item: item.clone(),
            recipe: recipe.clone(),
        });
        assert!(
            world.query_tile(*tile).is_some(),
            "the branch's {kind} must go down at {tile:?}"
        );
    }
    staff_up(&mut world);
    auto_fill_tanks(&mut world);

    // Twice the window the line was measured over alone, all of it inside
    // the branch's priming: its mixer's run is still filling at the end.
    let (ketchup_from, bbq_from) = (fills(&world), made(&world, "bbq_mix"));
    run(&mut world, 8_000);
    let shared = fills(&world) - ketchup_from;
    let bbq = made(&world, "bbq_mix") - bbq_from;
    assert!(bbq > 0, "the branch must be drawing ketchup");
    assert!(
        shared * 2 >= alone,
        "the ketchup bottler must keep a share of the vat while the branch primes: it bottled \
         {alone} in 4 000 ticks alone and only {shared} in the 8 000 after the branch went \
         down, while the branch's mixer drew {bbq} units"
    );
}

/// Which runs are held is worked out from the tick's state and never stored
/// (`World::held_networks`), so a factory saved while its two consumers are
/// taking turns comes back taking the same turns. Saved mid-priming, when the
/// shared run is held on most ticks, and run in lockstep with the original.
#[test]
fn a_shared_run_saved_mid_turn_comes_back_in_lockstep() {
    let mut original = branch();
    for _ in 0..3_000u64 {
        supply(&mut original);
        original.tick();
    }
    let mut loaded = World::from_save_ron(&original.to_save_ron(), recipes())
        .expect("the branch's save must load");
    for step in 0..4_000u64 {
        supply(&mut original);
        supply(&mut loaded);
        original.tick();
        loaded.tick();
        if step % 250 == 0 {
            assert_eq!(
                loaded.to_save_ron(),
                original.to_save_ron(),
                "the loaded branch diverged {} tick(s) after the load",
                step + 1
            );
        }
    }
    assert_eq!(loaded.to_save_ron(), original.to_save_ron());
    assert!(
        made(&original, "bbq_mix") > 0 && made(&original, "bottled_ketchup") > 0,
        "both consumers must have drawn from the shared run over the stretch compared"
    );
}
