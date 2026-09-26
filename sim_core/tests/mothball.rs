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
    ContractRegistry, MachineRegistry, PlayerCommand, RecipeRegistry, StallKind, TileSnapshot,
    WorkState, World, command_from_json, reference_chain_with_budget,
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

/// What contracts have collected off the docks in their lifetimes — the
/// factory's whole income, and so the whole of what an idle line costs it.
/// Everything a dock ever took off its belt, less what still stands on it:
/// a dock short of its clerk keeps taking boxes, but none of them is sold.
fn shipped(world: &World) -> u32 {
    world
        .dock_snapshots()
        .iter()
        .map(|d| d.shipped_total - d.used)
        .sum()
}

/// What the lines have carried to their docks in their lifetimes, sold or
/// not — whether the factory is making anything at all.
fn delivered(world: &World) -> u32 {
    world.dock_snapshots().iter().map(|d| d.shipped_total).sum()
}

fn ticks(world: &mut World, n: u32) {
    for _ in 0..n {
        common::supply(world);
        world.tick();
    }
}

/// Runs to the tick a payday lands on and through it, with the deliveries
/// of raw material cut off — the line runs itself dry and then stands
/// still. What that is for: a factory earning nothing either way, so the
/// ONLY thing moving the balance is the wage bill, which is the one thing
/// under test here.
fn to_next_dry_payday(world: &mut World) {
    let target = world.staff().next_payday_tick();
    while world.current_tick() < target {
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
/// The only other way to lower `required` is `Remove`, which is not free:
/// the equipment does go back to the shed (`try_remove` credits the owned
/// count, so the purchase is not lost), but the machine's input and output
/// buffers and whatever batch it was half way through are despawned with
/// it, and the player has to re-place it and re-link the line. Mothballing
/// keeps all of that standing — which is what makes the spiral escapable
/// rather than one-way.
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
    // Bounded on purpose: this used to be a bare `while`, so a vat that never
    // starts a craft at all hung the whole `cargo test -p sim_core` run
    // instead of failing it. A vat crafts every 240-600 ticks when it is fed,
    // so 5_000 is a decade of slack; running out of them is a real defect
    // upstream of this test and says so.
    let mut budget = 5_000;
    while !matches!(progress(&world), Some((elapsed, required)) if elapsed > 10 && elapsed + 10 < required)
    {
        common::supply(&mut world);
        world.tick();
        budget -= 1;
        assert!(
            budget > 0,
            "the cooking vat never got a craft under way, so there is no \
             paused batch to test: stall is {:?}",
            stall_of(&world, vat)
        );
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

/// The shipped line's own wage bill: manufacturing 4, packaging 2,
/// logistics 1, every head at wage 2.
const SHIPPED_BILL: u32 = 14;

/// **The state issue #19 describes**, built the way a player reaches it:
/// the shipped line, running and fed, with the balance stated at `money`
/// on payday eve. Nothing is over-built and nobody is over-hired — the
/// establishment is exactly the line's, which is the point. What follows
/// is the spiral: the payday cannot be met, every head the money did not
/// reach stops working, a factory with nobody working earns nothing, and
/// the next payday is no smaller than the one just missed.
fn broke_on_payday_eve(money: u32) -> World {
    let mut world = running_line(1_200);
    to_payday_eve(&mut world);
    let world = with_money(&world, money);
    assert_eq!(world.staff().next_bill(), SHIPPED_BILL);
    world
}

/// Every placement in the world, switched off. The player's move: the line
/// is standing idle anyway, so shut it.
fn mothball_everything(world: &mut World) {
    for tile in world
        .world_layout()
        .iter()
        .map(|p| p.tile)
        .collect::<Vec<_>>()
    {
        mothball(world, tile, true);
    }
}

fn switch_everything_back_on(world: &mut World) {
    for tile in world
        .world_layout()
        .iter()
        .map(|p| p.tile)
        .collect::<Vec<_>>()
    {
        mothball(world, tile, false);
    }
}

/// Runs to the tick a payday lands on and through it.
fn to_next_payday(world: &mut World) {
    to_payday_eve(world);
    ticks(world, 1);
}

/// **The reviewer's own A/B, and the number that had to move.** The first
/// round of this work was rejected because running four paydays with and
/// without mothballing gave bit-identical balances: `required` fell, and
/// not one coin followed it. This is that experiment, and it is the
/// regression guard for the whole mechanic — cut `stood_down` out of
/// `bill_per_period` and this fails on the first payday.
///
/// The fixture is the shipped line with no ingredients bought, so neither
/// factory earns anything and the ONLY thing that can move either balance
/// is payroll. Both start on 100 with the same seven heads.
///
/// * The control is billed 14 four times and ends on 44.
/// * The mothballed factory is billed 14 ONCE — the notice period it was
///   already half way through, which is what stops switching off on payday
///   eve being a free period of labour — and then nothing. It ends on 86.
///
/// Both keep all seven heads: this is what mothballing costs against
/// firing, which would have ended with 86 and an empty factory.
#[test]
fn four_paydays_with_and_without_mothballing_end_on_different_balances() {
    let placed = reference_chain_with_budget(recipes(), 0);
    let mut world = with_money(&placed, 100);
    let mut control = World::from_save_ron(&world.to_save_ron(), recipes()).expect("clone");
    assert_eq!(world.staff().bill_per_period(), SHIPPED_BILL);

    mothball_everything(&mut world);
    assert_eq!(
        world.staff().bill_per_period(),
        0,
        "the run rate falls the instant the switch is thrown"
    );
    assert_eq!(
        control.staff().bill_per_period(),
        SHIPPED_BILL,
        "and the control is still paying for the same seven heads"
    );

    for payday in 0..4 {
        to_next_dry_payday(&mut world);
        to_next_dry_payday(&mut control);
        assert_eq!(
            shipped(&world) + shipped(&control),
            0,
            "payday {payday}: neither factory has earned a thing, so wages are the whole story"
        );
    }

    assert_eq!(
        control.money(),
        Money::from_num(100 - 4 * SHIPPED_BILL),
        "four full bills"
    );
    assert_eq!(
        world.money(),
        Money::from_num(100 - SHIPPED_BILL),
        "one notice period, then nothing"
    );
    assert_eq!(
        (
            headcount(&world, "manufacturing"),
            headcount(&world, "packaging"),
            headcount(&world, "logistics")
        ),
        (4, 2, 1),
        "and it kept the crew it saved the money on"
    );
}

/// **THE ACCEPTANCE TEST for issue #19**, and the one the first round of
/// this work did not have.
///
/// Two factories in the identical broken state — a missed payday, every
/// head in arrears, nothing running, nothing earning. One of them throws
/// the switch; the other plays on. The escape hatch exists if and only if
/// those two histories DIVERGE, so the control is not decoration here, it
/// is the measurement.
///
/// What the switch buys, in order:
///
/// * the run rate (`bill_per_period`) falls the instant it is thrown,
/// * the crew does NOT walk out at the next payday, which is the one the
///   two-strike rule would otherwise have taken them at,
/// * the payday after that bills nothing, so the arrears CLEAR with every
///   head still on the books,
/// * and the line switched back on is the line that went off — same crew,
///   no re-hiring, and the money climbs from there.
///
/// The control does none of it: it is billed in full at a payday it cannot
/// meet, its whole crew walks out on the second strike, and it ends with
/// an empty factory it has no money to re-staff.
#[test]
fn mothballing_is_what_takes_a_broke_factory_out_of_arrears_and_a_control_proves_it() {
    // One coin short of the bill, on payday eve.
    let mut world = broke_on_payday_eve(SHIPPED_BILL - 1);
    ticks(&mut world, 1);
    assert_eq!(
        world.staff().unpaid_last_payday(),
        1,
        "the money reaches all but the last head down staff.ron — the shipping clerk"
    );
    assert_eq!(world.staff().pool("logistics").unwrap().unpaid, 1);
    assert_eq!(
        uncrewed(&world),
        vec![tile_of(&world, "dock")],
        "issue #20's headline case: the dock is the placement going without"
    );

    // The spiral: the clerk in arrears does not work, so the dock ships
    // nothing, so nothing is earned towards the payday that follows.
    let shipped_before = shipped(&world);
    ticks(&mut world, 400);
    assert_eq!(
        shipped(&world),
        shipped_before,
        "an unpaid clerk is the whole income of this factory, and they are not at work"
    );

    // Both futures start HERE, from one save, so nothing but the switch
    // can account for the difference.
    let mut control = World::from_save_ron(&world.to_save_ron(), recipes()).expect("clone");

    // The escape: shut the line.
    mothball_everything(&mut world);
    assert_eq!(
        (
            required(&world, "manufacturing"),
            required(&world, "logistics")
        ),
        (0, 0),
        "nothing switched on asks for anybody"
    );
    assert_eq!(
        world.staff().bill_per_period(),
        0,
        "the run rate falls the instant the switch is thrown — issue #19's whole complaint"
    );
    assert_eq!(
        world.staff().next_bill(),
        SHIPPED_BILL,
        "but the period already half-worked is still billed: the notice-pay rule, \
         which is what stops mothballing on payday eve being a free period of labour"
    );

    // The payday the two-strike rule takes the clerk at — the one head
    // already a period in arrears. The shutdown is still inside its notice
    // period here, so it is billed the full 14 it cannot pay; the
    // difference is that a stood-down head is never the one shown the
    // door, so nobody leaves.
    to_next_payday(&mut world);
    to_next_payday(&mut control);
    assert_eq!(
        world.staff().walked_out_last_payday(),
        0,
        "a stood-down head does not walk out, however deep the arrears"
    );
    assert_eq!(
        control.staff().walked_out_last_payday(),
        1,
        "the control's clerk, unpaid twice, is gone"
    );
    assert_eq!(
        (
            headcount(&world, "logistics"),
            headcount(&control, "logistics")
        ),
        (1, 0)
    );

    // And the payday after it. For the shutdown: nobody on the payroll, so
    // nothing billed, so nobody unpaid — the arrears are gone and the crew
    // is intact. For the control: the six heads it could not pay last time
    // are on their second strike, and they all walk.
    to_next_payday(&mut world);
    to_next_payday(&mut control);
    assert_eq!(
        (world.staff().next_bill(), world.staff().unpaid()),
        (0, 0),
        "a shutdown settles at a bill of zero and clears what it owed"
    );
    assert_eq!(
        (
            headcount(&world, "manufacturing"),
            headcount(&world, "packaging"),
            headcount(&world, "logistics")
        ),
        (4, 2, 1),
        "every head that went into the shutdown came out of it"
    );
    assert_eq!(
        control.staff().walked_out_last_payday(),
        6,
        "the control loses the rest of its crew on their second strike"
    );
    assert_eq!(headcount(&control, "manufacturing"), 0);

    // Reopen. No hiring, no `can_hire` gate, nothing re-trained: the
    // establishment is back where it was and every head is still there to
    // fill it.
    switch_everything_back_on(&mut world);
    assert_eq!(world.staff().bill_per_period(), SHIPPED_BILL);
    assert!(uncrewed(&world).is_empty(), "fully crewed again");

    // And the decisive difference, in the one currency that is not money:
    // the factory that threw the switch MAKES THINGS AGAIN, and the one
    // that played on cannot, because it has nobody left to make them with.
    let (was_world, was_control) = (delivered(&world), delivered(&control));
    while control.current_tick() < world.current_tick() {
        common::supply(&mut control);
        control.tick();
    }
    for _ in 0..2 {
        to_next_payday(&mut world);
        to_next_payday(&mut control);
    }
    assert!(
        delivered(&world) > was_world,
        "the reopened line ships again: {was_world} -> {}",
        delivered(&world)
    );
    assert_eq!(
        delivered(&control),
        was_control,
        "an empty factory ships nothing, ever"
    );
    assert_eq!(
        (
            headcount(&control, "manufacturing"),
            headcount(&control, "logistics")
        ),
        (0, 0)
    );
    assert!(
        !control.staff().can_hire("logistics", control.money()),
        "and it cannot buy its people back: `can_hire` refuses while the balance \
         cannot cover the next bill, and a factory that ships nothing has no balance"
    );

    // WHAT THIS DOES NOT CLAIM. The shutdown got the crew and the arrears
    // back under control; it did not conjure income. Shipping resumes, but
    // the standing order was lost to missed windows while the line stood
    // still, and signing it again costs money this balance has not got
    // (`data/contracts.ron`: "a lost contract is never free to recover").
    // That residue is issue #85, filed against this same work and left
    // open deliberately — mothballing is the bill escape, not a cash one.
    assert_eq!(
        world
            .economy_snapshot()
            .board
            .iter()
            .filter(|offer| offer.active)
            .count(),
        0,
        "the standing order did not survive the shutdown — issue #85"
    );
}

/// The reviewer's question, asked directly: is it the MOTHBALLING that
/// moves the money, or was it the firing all along?
///
/// Three factories from one identical broke save. One mothballs and fires
/// nobody; one fires its way down to the same bill and mothballs nothing;
/// one does neither. The first two both survive — that is the point, they
/// are alternatives — but they are not the same move, and the difference
/// is the crew: the factory that fired is left unable to buy its people
/// back, because [`StaffPools::can_hire`] refuses while the balance cannot
/// cover the next bill.
///
/// Before the stand-down rule these two histories were bit-identical, and
/// that identity was the proof that mothballing did nothing at all.
#[test]
fn mothballing_and_firing_are_not_the_same_move() {
    let mut broke = broke_on_payday_eve(SHIPPED_BILL - 1);
    ticks(&mut broke, 401);

    let mut mothballed = World::from_save_ron(&broke.to_save_ron(), recipes()).expect("clone");
    let mut fired = World::from_save_ron(&broke.to_save_ron(), recipes()).expect("clone");

    mothball_everything(&mut mothballed);
    for category in ["manufacturing", "packaging", "logistics"] {
        for _ in 0..headcount(&fired, category) {
            fired.issue_command(PlayerCommand::Fire {
                category: category.into(),
            });
        }
    }
    assert_eq!(
        (
            mothballed.staff().bill_per_period(),
            fired.staff().bill_per_period()
        ),
        (0, 0),
        "two ways to the same run rate"
    );

    // Three paydays on, both are clear of arrears — and only one of them
    // still has a factory's worth of people.
    for _ in 0..3 {
        to_next_payday(&mut mothballed);
        to_next_payday(&mut fired);
    }
    assert_eq!(
        (mothballed.staff().unpaid(), fired.staff().unpaid()),
        (0, 0)
    );
    assert_eq!(
        (
            headcount(&mothballed, "manufacturing"),
            headcount(&fired, "manufacturing")
        ),
        (4, 0),
        "the escape that keeps the crew, and the escape that spends it"
    );
    assert!(
        !fired.staff().can_hire("manufacturing", fired.money()),
        "and the one that spent it cannot buy it back — the trap firing leaves"
    );

    // Reopening costs the mothballed factory one command per machine and
    // nothing else.
    switch_everything_back_on(&mut mothballed);
    assert_eq!(mothballed.staff().bill_per_period(), SHIPPED_BILL);
    assert!(uncrewed(&mothballed).is_empty());
}

/// Standing down is capped by the machinery that is actually off: a head
/// hired beyond the establishment still costs full wage, so over-hiring is
/// as expensive as it ever was and mothballing one machine cannot carry a
/// payroll of ten.
#[test]
fn only_the_crew_of_mothballed_machinery_comes_off_the_payroll() {
    let mut world = running_line(200);
    for _ in 0..3 {
        world.issue_command(PlayerCommand::Hire {
            category: "manufacturing".into(),
        });
    }
    assert_eq!(headcount(&world, "manufacturing"), 7);
    assert_eq!(
        world.staff().bill_per_period(),
        SHIPPED_BILL + 6,
        "three surplus heads at wage 2"
    );

    // The vat is a two-cook machine, so switching it off stands two down —
    // not the three that are surplus.
    let vat = tile_of(&world, "cooking_vat");
    mothball(&mut world, vat, true);
    let manufacturing = world.staff().pool("manufacturing").unwrap();
    assert_eq!(
        (manufacturing.stood_down, manufacturing.on_payroll()),
        (2, 5)
    );
    assert_eq!(
        world.staff().bill_per_period(),
        SHIPPED_BILL + 2,
        "the surplus is still paid in full; only the vat's two came off"
    );

    // And a head fired out of the surplus is one less head to pay, not a
    // second saving on the same two.
    world.issue_command(PlayerCommand::Fire {
        category: "manufacturing".into(),
    });
    assert_eq!(world.staff().pool("manufacturing").unwrap().stood_down, 2);
    assert_eq!(world.staff().bill_per_period(), SHIPPED_BILL);
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

/// **The rule is the crew list, not the Rust type.** A placement may be
/// switched off iff its kind asks for a crew in `machines.ron`: the
/// command exists to take wages off the bill (issue #19), so switching off
/// a hopper, a tank, a belt or a pipe would cost production and save
/// nothing, while the DOCK — which does draw a clerk — must be switchable,
/// or a factory with no income can never get its bill to zero and never
/// climbs out of arrears (`docs/DECISIONS.md`, 2026-09-20). Asking the
/// content keeps the answer in `machines.ron` where a new crewed kind
/// inherits it for free.
///
/// The refusals are silent and byte-exact: a refused command must leave the
/// world identical rather than record a switch nothing reads.
#[test]
fn only_a_crewed_placement_can_be_mothballed() {
    let mut world = running_line(600);
    let hopper = tile_of(&world, "hopper");
    let tank = tile_of(&world, "fluid_tank");
    let belt = tile_of(&world, "belt");
    let pipe = tile_of(&world, "pipe");
    let before = world.to_save_ron();
    for tile in [hopper, tank, belt, pipe, (-999, -999)] {
        mothball(&mut world, tile, true);
    }
    assert_eq!(
        world.to_save_ron(),
        before,
        "a kind that asks for nobody has nothing to stand down, so nothing is recorded"
    );

    // The dock is the other side of the same rule.
    let dock = tile_of(&world, "dock");
    mothball(&mut world, dock, true);
    assert_eq!(
        required(&world, "logistics"),
        0,
        "a switched-off dock stops asking for its clerk"
    );
    assert_ne!(world.to_save_ron(), before, "and the switch is persisted");
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
