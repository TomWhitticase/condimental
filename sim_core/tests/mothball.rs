//! **Mothballing** (issue #19) and **`crewed` per placement** (issue #20),
//! proved through the same `World` API `SimBridge` exposes.
//!
//! Issue #19 named the unpaid spiral: a payday the balance cannot cover
//! leaves heads unpaid, unpaid heads do not work, less work means less
//! money, and the next payday is no smaller — a state a player can enter
//! and cannot play their way out of. Mothballing is the escape hatch that
//! was chosen for it (`docs/DECISIONS.md`, 2026-09-20), and what is pinned
//! here is the whole of what it promises: a machine switched off asks for
//! no crew, so the ESTABLISHMENT falls and the surplus can be let go; it
//! makes and spends nothing while it is off; the craft it was half way
//! through is waiting where it left it when it comes back; and the switch
//! is the player's, so it survives a save.
//!
//! Issue #20 named the other half: the game said *how many* heads were
//! missing but never *which machine* was standing idle for want of one.
//! `crewed` is that fact, per placement, derived.

mod common;

use sim_core::sim::save::MoneyBits;
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, Direction, MachineRegistry, PlayerCommand, RecipeRegistry, StallKind,
    TileSnapshot, WorkState, World, command_from_json, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// The reference line, stocked, funded for many paydays and kept fed for
/// `ticks` ticks — `tests/staff.rs`'s fixture, for the same reason.
fn running_line(ticks: u32) -> World {
    let mut world =
        reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE);
    common::stock_up(&mut world);
    for _ in 0..ticks {
        common::supply(&mut world);
        world.tick();
    }
    world
}

fn tile_of(world: &World, kind: &str) -> (i32, i32) {
    world
        .world_layout()
        .iter()
        .find(|p| p.kind == kind)
        .unwrap_or_else(|| panic!("the reference chain places a {kind}"))
        .tile
}

fn mothball(world: &mut World, tile: (i32, i32), enabled: bool) {
    world.issue_command(PlayerCommand::SetMothballed { tile, enabled });
}

fn required(world: &World, category: &str) -> u32 {
    world
        .staff_snapshot()
        .categories
        .iter()
        .find(|c| c.id == category)
        .unwrap_or_else(|| panic!("data/staff.ron has a {category} category"))
        .required
}

fn headcount(world: &World, category: &str) -> u32 {
    world.staff().headcount(category)
}

fn stall_of(world: &World, tile: (i32, i32)) -> Option<StallKind> {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|e| e.tile == tile)
        .unwrap_or_else(|| panic!("no machine at {tile:?}"))
        .stall
        .map(|s| s.kind)
}

/// Every placement the wire says is going without its crew, by tile.
fn uncrewed(world: &World) -> Vec<(i32, i32)> {
    world
        .world_layout()
        .iter()
        .filter(|p| !p.crewed)
        .map(|p| p.tile)
        .collect()
}

fn ticks(world: &mut World, n: u32) {
    for _ in 0..n {
        common::supply(world);
        world.tick();
    }
}

/// Runs to the tick before the next payday.
fn to_payday_eve(world: &mut World) {
    let target = world.staff().next_payday_tick() - 1;
    while world.current_tick() < target {
        common::supply(world);
        world.tick();
    }
}

/// States a balance by rewriting the save, the way `tests/staff.rs` and
/// `test_layouts.gd`'s `fund` do — the one legitimate way to say what the
/// player has rather than earn it.
fn with_money(world: &World, money: u32) -> World {
    let mut save = world.to_save();
    save.money_bits = MoneyBits::of(Money::from_num(money));
    World::from_save_ron(&save.to_ron(), recipes()).expect("the save must load")
}

// ---------------------------------------------------------------------
// A machine that is off
// ---------------------------------------------------------------------

/// THE WHOLE POINT: a mothballed machine is out of the establishment.
///
/// Nothing else in the game can lower `required` without tearing the
/// machine down and losing what it cost, which is what made the spiral
/// one-way.
#[test]
fn a_mothballed_machine_drops_out_of_the_establishment_and_comes_back_with_it() {
    let mut world = running_line(1_200);
    let vat = tile_of(&world, "cooking_vat");
    assert_eq!(
        required(&world, "manufacturing"),
        4,
        "crusher 1 + mixer 1 + vat 2"
    );

    mothball(&mut world, vat, true);
    assert_eq!(
        required(&world, "manufacturing"),
        2,
        "the vat's two cooks leave the establishment the moment it is switched off"
    );
    // And the payroll advice moves with it: four heads against an
    // establishment of two is two more than the factory needs.
    assert_eq!(headcount(&world, "manufacturing"), 4);
    assert_eq!(
        world
            .staff_snapshot()
            .categories
            .iter()
            .find(|c| c.id == "manufacturing")
            .unwrap()
            .short,
        0,
        "nobody is short of a head they still need"
    );

    mothball(&mut world, vat, false);
    assert_eq!(
        required(&world, "manufacturing"),
        4,
        "returning it to service puts its crew back on the books"
    );
}

/// Off means off: no crew asked for, no buffer touched, nothing shipped,
/// and the wire says so in words rather than by the machine merely looking
/// quiet.
#[test]
fn a_mothballed_machine_asks_for_nobody_and_makes_nothing() {
    // Long enough that the first boxes have reached the dock: the line
    // takes about two pay periods to prime end to end.
    let mut world = running_line(3_000);
    let vat = tile_of(&world, "cooking_vat");
    let shipped_before = world.dock_snapshots()[0].shipped_total;
    assert!(
        shipped_before > 0,
        "the line is running before we switch it off"
    );

    mothball(&mut world, vat, true);
    let demand_peak_off = {
        let mut peak = 0;
        for _ in 0..600 {
            common::supply(&mut world);
            world.tick();
            let manufacturing = world
                .staff_snapshot()
                .categories
                .into_iter()
                .find(|c| c.id == "manufacturing")
                .unwrap();
            peak = peak.max(manufacturing.demand);
        }
        peak
    };
    assert!(
        demand_peak_off <= 2,
        "with the vat off only the crusher and the mixer can ever ask: got {demand_peak_off}"
    );
    assert_eq!(
        stall_of(&world, vat),
        Some(StallKind::Mothballed),
        "the stall says switched off, not starved-of-something"
    );
    assert_eq!(
        world
            .machine_load_snapshots()
            .into_iter()
            .find(|e| e.tile == vat)
            .unwrap()
            .load
            .status,
        WorkState::Starved,
        "a tick with no work done is a starved tick; the stall carries the reason"
    );

    // Nothing came out of the far end of the line in all that time — the
    // vat is the only route from mixed sauce to a bottle.
    let bottler = tile_of(&world, "bottler");
    assert_eq!(
        stall_of(&world, bottler),
        Some(StallKind::FluidInput),
        "downstream of a switched-off machine there is simply nothing"
    );
}

/// A craft in progress is PAUSED, exactly the way a crew refusal pauses
/// one. Switching a machine off to survive a payday must not throw away
/// the batch it was half way through.
#[test]
fn returning_a_machine_to_service_resumes_the_craft_it_was_part_way_through() {
    let mut world = running_line(1_500);
    let vat = tile_of(&world, "cooking_vat");
    let progress = |w: &World| match w.query_tile(vat) {
        Some(TileSnapshot::Machine { process, .. }) => {
            process.map(|(start, required)| (w.current_tick() - start, required))
        }
        _ => None,
    };
    while !matches!(progress(&world), Some((elapsed, required)) if elapsed > 10 && elapsed + 10 < required)
    {
        common::supply(&mut world);
        world.tick();
    }
    let (elapsed_at_switch_off, required_ticks) = progress(&world).unwrap();

    mothball(&mut world, vat, true);
    ticks(&mut world, 400);
    assert_eq!(
        progress(&world),
        Some((elapsed_at_switch_off, required_ticks)),
        "the paused craft must not age while the machine is off"
    );

    mothball(&mut world, vat, false);
    let left_to_run = required_ticks - elapsed_at_switch_off;
    for _ in 0..left_to_run - 1 {
        common::supply(&mut world);
        world.tick();
        assert!(
            progress(&world).is_some(),
            "still the same craft, still running"
        );
    }
    common::supply(&mut world);
    world.tick();
    assert!(
        progress(&world).is_none() || progress(&world).unwrap().0 < elapsed_at_switch_off,
        "the batch finishes exactly the ticks it had left after the switch, and no later"
    );
}

// ---------------------------------------------------------------------
// The escape hatch (issue #19)
// ---------------------------------------------------------------------

/// How many spare crushers the overbuilt fixture places. Each is a
/// one-tile, one-cook machine, placed away from the line and connected to
/// nothing — a player's half-built second line.
const SPARE_CRUSHERS: u32 = 8;

/// A factory that has built more than it can pay for: the shipped line
/// plus a second line's worth of crushers, crewed to the establishment
/// they create, and one shipping clerk hired ahead of the second dock.
/// Its balance is then stated at `money`.
fn overbuilt_line(money: u32) -> World {
    let mut world = running_line(1_200);
    for i in 0..SPARE_CRUSHERS as i32 {
        world.issue_command(PlayerCommand::Purchase {
            kind: "crusher".into(),
            quantity: 1,
        });
        world.issue_command(PlayerCommand::Place {
            kind: "crusher".into(),
            tile: (2 * i, 10),
            direction: Direction::East,
            item: None,
        });
    }
    assert_eq!(required(&world, "manufacturing"), 4 + SPARE_CRUSHERS);
    for _ in 0..SPARE_CRUSHERS {
        world.issue_command(PlayerCommand::Hire {
            category: "manufacturing".into(),
        });
    }
    // And one shipping clerk hired ahead of the second dock, which is the
    // next thing they mean to build.
    world.issue_command(PlayerCommand::Hire {
        category: "logistics".into(),
    });
    assert_eq!(headcount(&world, "manufacturing"), 4 + SPARE_CRUSHERS);
    assert_eq!(
        (
            required(&world, "logistics"),
            headcount(&world, "logistics")
        ),
        (1, 2)
    );

    to_payday_eve(&mut world);
    with_money(&world, money)
}

/// **THE ACCEPTANCE TEST for issue #19.** A factory is driven into arrears
/// — a payday its balance could not cover, a head left unpaid — mothballs
/// the machinery it cannot afford to crew, and climbs back out.
///
/// Every step of the climb is asserted, because each was a separate part
/// of the complaint: the ESTABLISHMENT falls (nothing else could lower it
/// without demolishing the machine), the WAGE BILL falls with the firing
/// that the smaller establishment invites, the notice-pay period costs
/// exactly one more full bill and no more, and from the payday after that
/// the factory is paying its way and its balance is climbing.
#[test]
fn mothballing_takes_a_factory_that_missed_a_payday_back_out_of_arrears() {
    // One coin short of the bill — the crunch payday.
    let bill = 2 * (4 + SPARE_CRUSHERS + 2 + 2);
    let mut world = overbuilt_line(bill - 1);
    assert_eq!(world.staff().bill_per_period(), bill);
    assert_eq!(world.staff().next_bill(), bill);
    assert_eq!(
        world.staff().shortfall(world.money()),
        1,
        "the wire warns before the payday, not after"
    );

    // The payday lands. Heads are paid one at a time down `staff.ron`, so
    // the coin runs out on the LAST head — the second dock's clerk.
    ticks(&mut world, 1);
    assert_eq!(world.staff().unpaid_last_payday(), 1, "one head in arrears");
    assert_eq!(world.staff().pool("logistics").unwrap().unpaid, 1);
    assert_eq!(
        world.staff().walked_out_last_payday(),
        0,
        "nobody walks out on the first miss"
    );
    let money_at_the_miss = world.money();
    // The factory is still shipping — the clerk who WAS paid is enough to
    // keep the dock working — which is the runway the escape hatch needs.
    // A control copy of this exact moment, left alone, is what the escape
    // is measured against at the bottom.
    let untouched = World::from_save_ron(&world.to_save_ron(), recipes()).expect("clone");

    // The escape: switch the half-built second line off. The establishment
    // falls the moment the command lands — this is the move that nothing
    // in the game could make before.
    for i in 0..SPARE_CRUSHERS as i32 {
        mothball(&mut world, (2 * i, 10), true);
    }
    assert_eq!(
        required(&world, "manufacturing"),
        4,
        "back to the shipped line's crew"
    );

    // Which is what makes the firing safe to do: the payroll can now be
    // cut to the establishment without leaving a machine uncrewed.
    for _ in 0..SPARE_CRUSHERS {
        world.issue_command(PlayerCommand::Fire {
            category: "manufacturing".into(),
        });
    }
    world.issue_command(PlayerCommand::Fire {
        category: "logistics".into(),
    });
    assert_eq!(
        world.staff().bill_per_period(),
        14,
        "the wage bill falls to the shipped line's"
    );
    assert_eq!(
        world.staff().pool("logistics").unwrap().unpaid,
        0,
        "firing takes the head in arrears first, so the arrears go with it"
    );
    assert!(uncrewed(&world).is_empty(), "nothing is left short");

    // One more full bill: every head was employed in part of this period,
    // so notice pay charges for all of them one last time. That is the
    // rule, and it is the price of the escape.
    assert_eq!(world.staff().next_bill(), bill, "notice pay, in full");
    to_payday_eve(&mut world);
    ticks(&mut world, 1);
    assert_eq!(world.staff().last_bill(), bill);
    assert_eq!(
        (
            world.staff().unpaid_last_payday(),
            world.staff().walked_out_last_payday()
        ),
        (0, 0),
        "the line kept shipping through the arrears, so the notice bill was met in full"
    );

    // And from here the factory pays its way. Two more paydays, each met
    // out of income, each one leaving more in the bank than the last.
    assert_eq!(
        world.staff().next_bill(),
        14,
        "the smaller bill, from now on"
    );
    let mut balance = world.money();
    for payday in 0..2 {
        to_payday_eve(&mut world);
        ticks(&mut world, 1);
        assert_eq!(
            (
                world.staff().unpaid_last_payday(),
                world.staff().walked_out_last_payday()
            ),
            (0, 0),
            "payday {payday} after the escape must be met in full"
        );
        assert!(
            world.money() > balance,
            "payday {payday}: the balance must be climbing, was {balance} now {}",
            world.money()
        );
        balance = world.money();
    }
    assert!(
        balance > money_at_the_miss,
        "the factory is better off than it was at the miss: {balance} vs {money_at_the_miss}"
    );

    // WHAT THE ESCAPE WAS WORTH. The control ran the same ticks carrying
    // the crew it had built for, and it is still paying for eight crushers
    // that never crushed anything. Mothballing is not the difference
    // between living and dying here — a line that out-earns its bill
    // recovers from one missed payday either way — it is the difference
    // between paying for what you use and paying for what you own, and
    // that is a whole establishment and more than half the wage bill.
    let mut control = untouched;
    while control.current_tick() < world.current_tick() {
        common::supply(&mut control);
        control.tick();
    }
    assert_eq!(
        control.staff().bill_per_period(),
        bill,
        "the control never cut its payroll"
    );
    assert_eq!(required(&control, "manufacturing"), 4 + SPARE_CRUSHERS);
    assert!(
        world.money() > control.money(),
        "the mothballed factory is ahead: {} vs {}",
        world.money(),
        control.money()
    );
}

// ---------------------------------------------------------------------
// Save/load
// ---------------------------------------------------------------------

/// The switch is the player's, so it rides the save — and so does the
/// establishment it implies, which is the part a wrong answer would hide:
/// a factory reopened with its mothballed machines back in service is a
/// hidden wage bill.
#[test]
fn the_mothballed_switch_and_the_establishment_it_implies_survive_a_save() {
    let mut world = running_line(1_500);
    let vat = tile_of(&world, "cooking_vat");
    mothball(&mut world, vat, true);
    assert_eq!(required(&world, "manufacturing"), 2);

    let loaded = World::from_save_ron(&world.to_save_ron(), recipes()).expect("the save loads");
    assert_eq!(
        required(&loaded, "manufacturing"),
        2,
        "the loaded world must not quietly re-hire the vat's cooks"
    );
    assert_eq!(stall_of(&loaded, vat), Some(StallKind::Mothballed));
    assert!(
        loaded
            .machine_load_snapshots()
            .into_iter()
            .find(|e| e.tile == vat)
            .unwrap()
            .mothballed
    );
    assert_eq!(
        world.to_save_ron(),
        loaded.to_save_ron(),
        "and the save round-trips byte for byte"
    );
}

/// A save written before issue #19 has no `mothballed` field at all: it
/// loads with every machine in service, which is the only state it could
/// have been written in.
#[test]
fn a_save_from_before_mothballing_loads_with_every_machine_in_service() {
    let world = running_line(600);
    let text = world.to_save_ron();
    assert!(text.contains("mothballed"), "the field is written");
    let stripped = text
        .replace("mothballed: false,", "")
        .replace("mothballed: false", "");
    assert!(!stripped.contains("mothballed"));
    let loaded = World::from_save_ron(&stripped, recipes()).expect("defaults fill the gap");
    assert_eq!(required(&loaded, "manufacturing"), 4);
    for entry in loaded.machine_load_snapshots() {
        assert!(!entry.mothballed);
        assert_ne!(entry.stall.map(|s| s.kind), Some(StallKind::Mothballed));
    }
}

// ---------------------------------------------------------------------
// `crewed` per placement (issue #20)
// ---------------------------------------------------------------------

/// A fully crewed factory has nothing uncrewed, a factory one head short
/// has exactly ONE — the last machine in stable order that needs the short
/// category — and every other placement is still fine.
#[test]
fn crewed_is_false_for_exactly_the_placements_that_go_without() {
    let mut world = running_line(1_200);
    assert!(
        uncrewed(&world).is_empty(),
        "the shipped crew covers the shipped line exactly"
    );

    world.issue_command(PlayerCommand::Fire {
        category: "manufacturing".into(),
    });
    let vat = tile_of(&world, "cooking_vat");
    assert_eq!(
        uncrewed(&world),
        vec![vat],
        "the vat is last in stable order among the machines needing cooks"
    );

    // The polled per-machine channel carries the same answer, because that
    // is the one the load bar reads (issue #20's "the load bar goes amber
    // with the same words the hover already carries").
    for entry in world.machine_load_snapshots() {
        assert_eq!(entry.crewed, entry.tile != vat, "at {:?}", entry.tile);
    }
    match world.query_tile(vat) {
        Some(TileSnapshot::Machine {
            crewed, mothballed, ..
        }) => {
            assert!(!crewed);
            assert!(
                !mothballed,
                "short of a head is not the same as switched off"
            );
        }
        other => panic!("expected the vat, got {other:?}"),
    }

    // Hiring the head back clears it, with no tick in between: `crewed` is
    // derived at snapshot time, so it cannot lag a `Hire` that dirties no
    // tile at all.
    world.issue_command(PlayerCommand::Hire {
        category: "manufacturing".into(),
    });
    assert!(uncrewed(&world).is_empty());
}

/// A switched-off machine is not "short of staff" — it is asking for
/// nobody. It reads crewed, and `mothballed` is what distinguishes it, so
/// the hover can say "switched off" rather than accuse the player of an
/// understaffing they have already dealt with.
#[test]
fn a_mothballed_machine_reads_crewed_and_frees_the_head_for_the_machine_behind_it() {
    let mut world = running_line(1_200);
    let crusher = tile_of(&world, "crusher");
    let vat = tile_of(&world, "cooking_vat");
    world.issue_command(PlayerCommand::Fire {
        category: "manufacturing".into(),
    });
    assert_eq!(uncrewed(&world), vec![vat]);

    // Switching off the crusher — first in stable order — hands its cook
    // to the vat, which is the whole first-come rule read backwards.
    mothball(&mut world, crusher, true);
    assert!(
        uncrewed(&world).is_empty(),
        "the freed head covers the machine that was going without"
    );
    let entry = world
        .machine_load_snapshots()
        .into_iter()
        .find(|e| e.tile == crusher)
        .unwrap();
    assert!(
        entry.crewed,
        "a machine asking for nobody is not short of anybody"
    );
    assert!(entry.mothballed);
}

// ---------------------------------------------------------------------
// The wire
// ---------------------------------------------------------------------

#[test]
fn the_set_mothballed_command_round_trips_through_json() {
    let parsed =
        command_from_json(r#"{"type": "set_mothballed", "tile": [12, 0], "enabled": true}"#)
            .expect("the wire form parses");
    assert!(matches!(
        parsed,
        PlayerCommand::SetMothballed {
            tile: (12, 0),
            enabled: true
        }
    ));
    // `enabled` defaults to true, like the other two switches, so a caller
    // that omits it switches the machine off rather than silently on.
    let defaulted = command_from_json(r#"{"type": "set_mothballed", "tile": [12, 0]}"#)
        .expect("the wire form parses without `enabled`");
    assert!(matches!(
        defaulted,
        PlayerCommand::SetMothballed {
            tile: (12, 0),
            enabled: true
        }
    ));
}

/// The command is a machine's. A hopper, a tank, a belt and a pipe ask for
/// nobody, so there is nothing to release; the DOCK asks for one clerk but
/// is the only route product has to money, so switching it off is never
/// the move and the escape hatch for a spare one is `Remove`, which hands
/// it back to the shed with nothing lost but its position
/// (`docs/DECISIONS.md`, 2026-09-20). Every one of them is refused rather
/// than silently recording a switch nothing reads.
#[test]
fn only_a_machine_can_be_mothballed() {
    let mut world = running_line(600);
    let hopper = tile_of(&world, "hopper");
    let tank = tile_of(&world, "fluid_tank");
    let dock = tile_of(&world, "dock");
    let before = world.to_save_ron();
    for tile in [hopper, tank, dock, (-999, -999)] {
        mothball(&mut world, tile, true);
    }
    assert_eq!(world.to_save_ron(), before, "nothing was recorded");
    assert_eq!(
        required(&world, "logistics"),
        1,
        "the dock still wants its clerk"
    );
}

/// `World::new` builds nothing and the registries are the shipped ones —
/// the fixture every other file in `tests/` leans on. Kept here so this
/// file fails loudly if the content changes shape under it.
#[test]
fn the_fixture_still_describes_the_shipped_content() {
    let empty = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::ZERO,
    );
    assert_eq!(
        required(&empty, "manufacturing"),
        0,
        "an empty world needs nobody"
    );
    assert!(uncrewed(&empty).is_empty());
}
