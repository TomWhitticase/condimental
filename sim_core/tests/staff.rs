//! Staff pools (the staff pools MVP epic, `docs/STAFF_POOLS_PLAN.md`;
//! GAMEPLAY.md §5, §7, §9, §11), proved end to end against the shipped
//! data through the same `World` API `SimBridge` exposes.
//!
//! What is pinned here, in the order the plan states it: the shipped
//! content agrees with itself (every crew names a real category; the
//! starting crew IS the reference line's summed crew); a fed line runs at
//! full load on that crew with no `staff` stall anywhere; a shortfall lands
//! on exactly the LAST machine in stable order that needs the short
//! category, and nowhere else; a paused machine loses no progress; a
//! refused dock holds and loses nothing; payday debits the whole bill on
//! the boundary, warns before it, idles the unpaid rather than firing them
//! and lets go only those unpaid twice (the quality pass, 2026-09-13);
//! notice pay makes firing before payday cost nothing less; a hire the
//! balance could not pay is refused; the commands round-trip through JSON;
//! a kind with no crew never asks; and all of it survives a save.
//! `staff.rs`'s own unit tests cover the catalog's rejection rules,
//! the claim arithmetic and the zero-wage head, which needs a catalog the
//! shipped `World` does not offer a seam for.

mod common;

use sim_core::sim::save::{MoneyBits, SavedBody, SavedStaff};
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, Direction, LOAD_WINDOW_TICKS, MachineRegistry, PlayerCommand, RecipeRegistry,
    STARTING_MONEY, StaffCatalog, StaffLoadError, StaffPools, StallKind, TileSnapshot, WorkState,
    World, command_from_json, reference_chain, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// The reference line, stocked and funded for many paydays, kept fed every
/// tick for `ticks` ticks — long enough that every machine has work, short
/// enough that none has backed up yet (the vat blocks the mixer only from
/// about tick 3500; see `tests/machine_load.rs`).
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

fn stall_of(world: &World, tile: (i32, i32)) -> Option<(StallKind, String)> {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|e| e.tile == tile)
        .unwrap_or_else(|| panic!("no machine at {tile:?}"))
        .stall
        .map(|s| (s.kind, s.item))
}

fn status_of(world: &World, tile: (i32, i32)) -> WorkState {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|e| e.tile == tile)
        .unwrap()
        .load
        .status
}

/// `(elapsed, required)` of the machine's current craft, from the wire's
/// `process`.
fn progress_of(world: &World, tile: (i32, i32)) -> Option<(u64, u64)> {
    match world.query_tile(tile)? {
        TileSnapshot::Machine { process, .. } => {
            process.map(|(start, required)| (world.current_tick() - start, required))
        }
        _ => None,
    }
}

fn hire(world: &mut World, category: &str) {
    world.issue_command(PlayerCommand::Hire {
        category: category.to_string(),
    });
}

fn fire(world: &mut World, category: &str) {
    world.issue_command(PlayerCommand::Fire {
        category: category.to_string(),
    });
}

fn headcount(world: &World, category: &str) -> u32 {
    world.staff().headcount(category)
}

// ---------------------------------------------------------------------
// The content agrees with itself
// ---------------------------------------------------------------------

#[test]
fn the_shipped_crews_name_only_shipped_categories() {
    StaffCatalog::bundled()
        .validate_machines(&MachineRegistry::bundled())
        .expect("every machines.ron crew line names a staff.ron category, once, with a count");
}

#[test]
fn validate_machines_rejects_an_unknown_category_a_duplicate_and_a_zero_count() {
    let registry = |staff: &str| {
        MachineRegistry::from_ron_str(&format!(
            r#"(schema_version: 2, machines: [
                MachineKind(id: "cooker", footprint: Footprint(width_tiles: 1, height_tiles: 1), staff: {staff}),
            ])"#
        ))
        .unwrap()
    };
    let catalog = StaffCatalog::bundled();
    assert!(catalog.validate_machines(&registry("[]")).is_ok());
    assert!(
        catalog
            .validate_machines(&registry(r#"[("manufacturing", 1)]"#))
            .is_ok()
    );
    assert!(matches!(
        catalog.validate_machines(&registry(r#"[("plumbers", 1)]"#)),
        Err(StaffLoadError::UnknownCategoryInMachine { .. })
    ));
    assert!(matches!(
        catalog.validate_machines(&registry(r#"[("manufacturing", 1), ("manufacturing", 2)]"#)),
        Err(StaffLoadError::DuplicateCategoryInMachine { .. })
    ));
    assert!(matches!(
        catalog.validate_machines(&registry(r#"[("manufacturing", 0)]"#)),
        Err(StaffLoadError::ZeroCrewCount { .. })
    ));
}

/// THE ESTABLISHMENT MOVES WHEN THE FACTORY MOVES, AND AT NO OTHER TIME.
///
/// This is the rule the 2026-09-16 staffing change exists to create, and it
/// is stated as two assertions because a player feels it as two things:
/// placing a machine tells you at once that it needs a crew, and nothing
/// you are not doing ever changes that number under you.
#[test]
fn the_establishment_tracks_what_is_placed_and_never_the_tick() {
    let mut world = running_line(60);
    let required = |w: &World, id: &str| -> u32 {
        w.staff_snapshot()
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .required
    };

    // The shipped chain: crusher 1 + mixer 1 + cooking vat 2.
    let before = required(&world, "manufacturing");
    assert_eq!(
        before, 4,
        "the reference chain's manufacturing establishment"
    );

    // Running the factory for a long while does not move it, however the
    // line ebbs and flows. `demand` is free to breathe underneath.
    for _ in 0..200 {
        common::supply(&mut world);
        world.tick();
        assert_eq!(
            required(&world, "manufacturing"),
            before,
            "the establishment moved without a machine being placed or removed"
        );
    }

    // A second crusher asks for one more cook, the moment it is placed.
    world.issue_command(PlayerCommand::Purchase {
        kind: "crusher".into(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "crusher".into(),
        tile: (12, 12),
        direction: Direction::East,
        item: None,
        recipe: None,
    });
    assert_eq!(
        required(&world, "manufacturing"),
        before + 1,
        "a placed crusher raises the manufacturing establishment by its crew"
    );

    // And takes it back when it is removed.
    world.issue_command(PlayerCommand::Remove { tile: (12, 12) });
    assert_eq!(
        required(&world, "manufacturing"),
        before,
        "removing it gives the head back"
    );
}

/// A 2x2 cooking vat is ONE crew of two, not four crews of two: the
/// establishment counts placements, and a multi-tile machine is one
/// placement however many tiles it covers.
#[test]
fn a_multi_tile_machine_is_counted_once() {
    let world = running_line(1);
    let vat_crew: u32 = world
        .crew_of_kind("cooking_vat")
        .iter()
        .filter(|(c, _)| c == "manufacturing")
        .map(|(_, n)| n)
        .sum();
    assert_eq!(vat_crew, 2, "the shipped vat asks for two cooks");
    let manufacturing = world
        .staff_snapshot()
        .categories
        .iter()
        .find(|c| c.id == "manufacturing")
        .unwrap()
        .required;
    assert_eq!(
        manufacturing, 4,
        "crusher 1 + mixer 1 + vat 2 — the vat counted once, not once per tile"
    );
}

/// The starting crew is sized to the reference line the way `STARTING_MONEY`
/// is sized to its bill: for every category, `staff.ron`'s
/// `starting_headcount` equals the summed `machines.ron` crew of the kinds
/// the line places. Computed from the data, never from literals, so a
/// re-crewed kind or a new stage moves the expectation with it.
#[test]
fn the_starting_crew_is_exactly_the_reference_chains_summed_crew() {
    let world = reference_chain(recipes());
    let catalog = StaffCatalog::bundled();
    for category in catalog.categories() {
        let demanded: u32 = world
            .world_layout()
            .iter()
            .flat_map(|p| world.crew_of_kind(&p.kind).to_vec())
            .filter(|(c, _)| *c == category.id)
            .map(|(_, n)| n)
            .sum();
        assert_eq!(
            category.starting_headcount, demanded,
            "staff.ron's starting {} must equal the reference chain's summed demand",
            category.id
        );
    }
    assert!(
        catalog
            .categories()
            .iter()
            .any(|c| c.starting_headcount > 0),
        "the line is not crewed by nobody"
    );
}

/// `tests/common`'s `WAGE_RESERVE` is a literal every long fixture funds
/// itself with; pin it against the data so a wage change is loud.
#[test]
fn the_test_wage_reserve_matches_the_starting_crews_bill() {
    let pools = StaffPools::bundled();
    assert_eq!(
        u64::from(common::WAGE_RESERVE),
        u64::from(pools.bill_per_period()) * common::WAGE_PERIODS,
        "common::WAGE_RESERVE must equal the starting crew's bill per period x WAGE_PERIODS"
    );
}

// ---------------------------------------------------------------------
// A fed line on the starting crew
// ---------------------------------------------------------------------

#[test]
fn a_fed_reference_chain_runs_on_the_starting_crew_with_no_staff_stall_anywhere() {
    let mut world = running_line(200);
    let mut ever_staff_stalled = Vec::new();
    for _ in 0..2_800 {
        common::supply(&mut world);
        world.tick();
        for entry in world.machine_load_snapshots() {
            if matches!(&entry.stall, Some(s) if s.kind == StallKind::Staff) {
                ever_staff_stalled.push((world.current_tick(), entry.tile));
            }
        }
        let snapshot = world.staff_snapshot();
        for c in &snapshot.categories {
            assert_eq!(
                c.short,
                0,
                "{} short at tick {}",
                c.id,
                world.current_tick()
            );
            assert_eq!(c.assigned, c.demand);
            assert!(c.assigned <= c.headcount);
        }
    }
    assert!(
        ever_staff_stalled.is_empty(),
        "the starting crew covers the line; staff stalls: {ever_staff_stalled:?}"
    );
    // The crusher is the machine nothing upstream can starve of anything
    // but tomatoes and nothing downstream has yet blocked. Since the
    // half-tile item spacing (the quality pass, 2026-09-13) its one lane
    // delivers four tomatoes per 100 ticks against an 80-tick craft, so it
    // is belt-limited: the only reason it ever gives is its input, never
    // staff.
    let crusher = tile_of(&world, "crusher");
    assert!(
        matches!(
            stall_of(&world, crusher),
            None | Some((StallKind::Input, _))
        ),
        "the crusher waits for tomatoes or for nothing: {:?}",
        stall_of(&world, crusher)
    );
    assert!(
        status_of(&world, crusher) == WorkState::Working
            || status_of(&world, crusher) == WorkState::Starved,
        "belt-limited, not blocked"
    );
    // On a tick when everything asks, the whole starting crew is at work;
    // on a tick the crusher is waiting for a tomato it asks for nobody.
    let snapshot = world.staff_snapshot();
    let by_id = |id: &str| {
        snapshot
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .clone()
    };
    assert!(
        by_id("manufacturing").assigned == 4 || by_id("manufacturing").assigned == 3,
        "{:?}",
        by_id("manufacturing")
    );
    assert!(by_id("packaging").assigned <= 2);
    assert_eq!(by_id("engineering").demand, 0);
    // The dock asks only on a tick it holds something; over the window it
    // shipped, so logistics was asked for at least once.
    assert!(world.dock_snapshots()[0].shipped_total > 0);
}

// ---------------------------------------------------------------------
// A shortfall lands on the last machine in stable order
// ---------------------------------------------------------------------

/// Firing one manufacturing head leaves three for a crusher (1), a mixer
/// (1) and a vat (2) claiming in tile order: the vat, LAST, is the one
/// refused — every tick, naming manufacturing — and the other two never are.
#[test]
fn firing_one_manufacturing_head_idles_exactly_the_last_manufacturing_machine() {
    let mut world = running_line(1_500);
    let crusher = tile_of(&world, "crusher");
    let mixer = tile_of(&world, "mixer");
    let vat = tile_of(&world, "cooking_vat");
    assert!(
        crusher.0 < mixer.0 && mixer.0 < vat.0,
        "stable order is tile order"
    );
    assert_eq!(status_of(&world, vat), WorkState::Working, "the vat before");
    // The crusher is the one machine a single lane cannot saturate since
    // the half-tile item spacing (the quality pass, 2026-09-13): a belt
    // delivers four tomatoes per 100 ticks and its craft takes 80, so it
    // waits for a tomato about a fifth of the time, and the mixer behind
    // it gets one paste per 100 ticks against its 90-tick craft. Both read
    // `starved` for an INPUT before the line backs up — never for staff.
    for tile in [crusher, mixer] {
        assert!(
            matches!(stall_of(&world, tile), None | Some((StallKind::Input, _))),
            "{tile:?} before: belt-limited at most, got {:?}",
            stall_of(&world, tile)
        );
    }

    fire(&mut world, "manufacturing");
    assert_eq!(headcount(&world, "manufacturing"), 3);

    // The rule under test: on every tick all three would work, the vat —
    // last in tile order — is the one refused, and the two before it are
    // never short. On a tick the crusher or the mixer is waiting for its
    // input it claims nothing, and then the vat's two heads ARE free: the
    // vat runs on exactly those ticks, which is the same first-come rule
    // read the other way round.
    let mut vat_refusals = 0;
    let mut vat_runs = 0;
    for _ in 0..120 {
        common::supply(&mut world);
        world.tick();
        for tile in [crusher, mixer] {
            assert!(
                !matches!(stall_of(&world, tile), Some((StallKind::Staff, _))),
                "{tile:?} must never go short: it claims before the vat"
            );
        }
        let snapshot = world.staff_snapshot();
        let manufacturing = snapshot
            .categories
            .iter()
            .find(|c| c.id == "manufacturing")
            .unwrap();

        // THE REGRESSION THAT MOTIVATED THE ESTABLISHMENT (2026-09-16).
        // One head was fired from a line whose placed machinery needs four,
        // so the answer to "how many must I hire?" is ONE, on every tick,
        // for as long as nothing is placed or removed. It used to be
        // `demand - assigned`, which read 2 on a tick the vat was refused
        // and 0 on a tick it ran — flapping between two wrong numbers while
        // the factory sat uniformly one head short, and moving again before
        // the player could act on what it said.
        assert_eq!(
            (manufacturing.required, manufacturing.short),
            (4, 1),
            "the establishment and the hiring advice must not move tick to tick"
        );

        let tally = (manufacturing.demand, manufacturing.assigned);
        match stall_of(&world, vat) {
            Some((StallKind::Staff, category)) => {
                assert_eq!(category, "manufacturing");
                assert_eq!(tally, (4, 2), "crusher and mixer claimed, the vat refused");
                vat_refusals += 1;
            }
            None => {
                // `demand` still describes the tick honestly: a machine
                // stood aside, so fewer heads were asked for and the vat
                // got its two. That figure is worth showing as "working
                // right now"; it is simply not the hiring question.
                assert!(tally.0 <= 3, "{tally:?}");
                vat_runs += 1;
            }
            other => panic!("the vat is refused or it runs, got {other:?}"),
        }
    }
    assert!(
        vat_refusals > vat_runs,
        "the vat is the one short on most ticks: refused {vat_refusals}, ran {vat_runs}"
    );
    assert_eq!(
        status_of(&world, vat),
        WorkState::Starved,
        "understaffed IS a kind of starved"
    );
    for tile in [crusher, mixer] {
        assert!(
            !matches!(stall_of(&world, tile), Some((StallKind::Staff, _))),
            "{tile:?} is belt-limited at most, never staff-limited"
        );
    }
    // Hover's `crew` for the vat, on a tick it was refused (the last tick
    // of the loop may be one the vat ran on).
    for _ in 0..20 {
        if matches!(stall_of(&world, vat), Some((StallKind::Staff, _))) {
            break;
        }
        common::supply(&mut world);
        world.tick();
    }
    match world.query_tile(vat) {
        Some(TileSnapshot::Machine { crew, stall, .. }) => {
            assert_eq!(crew.len(), 1);
            assert_eq!(
                (crew[0].category.as_str(), crew[0].count),
                ("manufacturing", 2)
            );
            assert_eq!(stall.unwrap().kind, StallKind::Staff);
        }
        other => panic!("expected the vat, got {other:?}"),
    }
}

fn understaffed_at(world: &World, tile: (i32, i32)) -> bool {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|e| e.tile == tile)
        .unwrap()
        .understaffed
}

/// `understaffed` — what the red person over a machine reads (PR #192) —
/// holds for as long as the machine stays short, where the per-tick
/// `staff` stall blinks: on a tick the crusher or mixer claims nothing the
/// vat gets its crew and runs, and its stall goes to none. It is never
/// raised over a machine that has not yet been fed (it asks for nobody, so
/// it is never refused), nor over one switched off.
#[test]
fn an_understaffed_machine_stays_understaffed() {
    let mut world =
        reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE);
    common::stock_up(&mut world);
    let crusher = tile_of(&world, "crusher");
    let mixer = tile_of(&world, "mixer");
    let vat = tile_of(&world, "cooking_vat");
    fire(&mut world, "manufacturing");

    // Short from the start, but nothing has reached the vat yet: it is
    // waiting for its input — so far any sauce's (`undecided`) — which a
    // hire would not fix.
    assert!(matches!(
        stall_of(&world, vat),
        Some((
            StallKind::Input
                | StallKind::FluidInput
                | StallKind::NoSupplyPipe
                | StallKind::Undecided,
            _
        ))
    ));
    assert!(
        !understaffed_at(&world, vat),
        "an unfed vat is not idle for want of a crew"
    );

    // Once its input has arrived it reads understaffed and stays so, tick
    // after tick, while the stall underneath it blinks.
    let mut first = None;
    let (mut staff_ticks, mut ran_ticks, mut stall_flips) = (0, 0, 0);
    let mut last_stall = None;
    for tick in 0..6_000 {
        common::supply(&mut world);
        world.tick();
        for tile in [crusher, mixer] {
            assert!(
                !understaffed_at(&world, tile),
                "{tile:?} claims before the vat"
            );
        }
        let stall = stall_of(&world, vat).map(|(kind, _)| kind);
        let understaffed = understaffed_at(&world, vat);
        match first {
            None if understaffed => first = Some(tick),
            None => continue,
            Some(since) => assert!(
                understaffed,
                "the vat is still one head short, but understaffed dropped at tick {tick} \
                 (raised at {since}, stall {stall:?})"
            ),
        }
        match stall {
            Some(StallKind::Staff) => staff_ticks += 1,
            None => ran_ticks += 1,
            _ => {}
        }
        if stall != last_stall {
            stall_flips += 1;
        }
        last_stall = stall;
    }
    assert!(first.is_some(), "the vat never read understaffed");
    // The reason this field exists: the per-tick stall really does blink.
    assert!(
        staff_ticks > 0 && ran_ticks > 0 && stall_flips > 10,
        "expected the raw stall to blink: {staff_ticks} refused, {ran_ticks} ran, {stall_flips} flips"
    );

    // Switched off, it needs no crew, so it is not short of one.
    world.issue_command(PlayerCommand::SetMothballed {
        tile: vat,
        enabled: true,
    });
    world.tick();
    assert!(
        !understaffed_at(&world, vat),
        "a mothballed vat is not understaffed"
    );
    world.issue_command(PlayerCommand::SetMothballed {
        tile: vat,
        enabled: false,
    });
    world.tick();
    assert!(
        understaffed_at(&world, vat),
        "switched back on, it is short again"
    );

    // And the hire clears it at once.
    hire(&mut world, "manufacturing");
    assert!(
        !understaffed_at(&world, vat),
        "hired back, the vat is crewed"
    );
}

/// Short on the establishment is not the same as held back by it, and only
/// the second earns the red person (PR #192's second review). The
/// establishment hands heads out in topology order to every machine in
/// service, working or not; the per-tick pools go only to machines that
/// would work. So a spare crusher placed early in the order with nothing
/// feeding it holds a head on the books it never claims, the vat at the end
/// reads uncrewed — and runs on every tick it has work. It gets no marker;
/// the spare, which is idle for want of tomatoes, gets none either.
#[test]
fn a_running_machine_short_on_paper_is_not_understaffed() {
    let mut world =
        reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE);
    common::stock_up(&mut world);
    world.issue_command(PlayerCommand::Purchase {
        kind: "crusher".into(),
        quantity: 1,
    });
    let spare = (-10, -10);
    world.issue_command(PlayerCommand::Place {
        kind: "crusher".into(),
        tile: spare,
        direction: Direction::East,
        item: None,
        recipe: None,
    });
    let vat = tile_of(&world, "cooking_vat");
    let entry = |world: &World, tile| {
        world
            .machine_load_snapshots()
            .into_iter()
            .find(|e| e.tile == tile)
            .unwrap()
    };
    assert!(
        entry(&world, spare).crewed,
        "the spare comes first in order"
    );
    assert!(!entry(&world, vat).crewed, "so the vat is short on paper");

    let (mut refused, mut ran) = (0, 0);
    for tick in 0..3_000 {
        common::supply(&mut world);
        world.tick();
        let vat_entry = entry(&world, vat);
        assert!(!vat_entry.crewed, "the vat stays short on paper");
        match vat_entry.stall.map(|s| s.kind) {
            Some(StallKind::Staff) => refused += 1,
            None => ran += 1,
            _ => {}
        }
        assert!(
            !vat_entry.understaffed,
            "the vat is running, not held back, but is marked at tick {tick}"
        );
        assert!(
            !entry(&world, spare).understaffed,
            "the spare is idle for want of tomatoes, not a crew"
        );
    }
    // The premise: the vat really did run, and was never turned away.
    assert_eq!(refused, 0, "the vat was refused its crew");
    assert!(ran > 1_000, "the vat ran on only {ran} ticks");
}

/// The 2026-09-27 report: fire a head from a running factory and a cooker
/// stops with no red person over it. The owner's save had three vats on
/// one pipe off the mixer; the fire left the last one short. While it sits
/// refused, the two crewed vats beside it drain the shared pipe, so between
/// refusals it reads `fluid_input` — and a marker gated on "nothing on its
/// own supply side" dropped for ~120 ticks at a time. It is idle for want
/// of a crew all the same, and must keep the person.
#[test]
fn a_short_vat_on_a_shared_pipe_keeps_its_marker() {
    let mut world = running_line(0);
    // The owner's layout: pipes along both sides of the reference vat, and
    // two more vats beside it on the same runs (north of it here, where the
    // reference line leaves room).
    for (kind, tiles) in [
        (
            "pipe",
            vec![
                (11, -1),
                (11, -2),
                (11, -3),
                (11, -4),
                (14, -1),
                (14, -2),
                (14, -3),
                (14, -4),
            ],
        ),
        ("cooking_vat", vec![(12, -2), (12, -4)]),
    ] {
        world.issue_command(PlayerCommand::Purchase {
            kind: kind.into(),
            quantity: tiles.len() as u32,
        });
        for tile in tiles {
            world.issue_command(PlayerCommand::Place {
                kind: kind.into(),
                tile,
                direction: Direction::East,
                item: None,
                // The extra vats cook what the shipped one cooks.
                recipe: (kind == "cooking_vat").then(|| "ketchup".to_string()),
            });
        }
    }
    for _ in 0..4 {
        hire(&mut world, "manufacturing");
    }
    let vats = |world: &World| {
        world
            .machine_load_snapshots()
            .into_iter()
            .filter(|e| [(12, 0), (12, -2), (12, -4)].contains(&e.tile))
            .collect::<Vec<_>>()
    };
    assert_eq!(vats(&world).len(), 3, "all three vats are placed");
    assert!(
        vats(&world).iter().all(|e| e.crewed),
        "every vat starts crewed"
    );
    for _ in 0..1_500 {
        common::supply(&mut world);
        world.tick();
    }
    fire(&mut world, "manufacturing");
    let short: Vec<_> = vats(&world)
        .iter()
        .filter(|e| !e.crewed)
        .map(|e| e.tile)
        .collect();
    assert_eq!(short.len(), 1, "the fire leaves exactly one vat short");
    let last = short[0];
    let entry = |world: &World| {
        world
            .machine_load_snapshots()
            .into_iter()
            .find(|e| e.tile == last)
            .unwrap()
    };

    // Run until it is first refused, then it must hold the person.
    let mut marked_from = None;
    let (mut starved_while_marked, mut unmarked) = (0, 0);
    for tick in 0..3_000 {
        common::supply(&mut world);
        world.tick();
        let e = entry(&world);
        if marked_from.is_none() {
            if e.understaffed {
                marked_from = Some(tick);
            }
            continue;
        }
        if e.understaffed {
            if matches!(e.stall.map(|s| s.kind), Some(StallKind::FluidInput)) {
                starved_while_marked += 1;
            }
        } else {
            unmarked += 1;
        }
    }
    let from = marked_from.expect("the short vat is never marked");
    // It rises the first tick the vat asks for its crew and is turned
    // away, which waits on its next batch's worth of sauce.
    assert!(
        from < LOAD_WINDOW_TICKS,
        "the person took {from} ticks to appear"
    );
    assert_eq!(
        unmarked, 0,
        "the short vat lost its person on {unmarked} ticks"
    );
    // The premise: it really did go without sauce between refusals, which
    // is what the old supply gate cleared the person on.
    assert!(
        starved_while_marked > 100,
        "the vat was waiting for sauce on only {starved_while_marked} ticks"
    );
}

/// A paused craft keeps its elapsed time exactly, and finishes the same
/// number of ticks after the crew returns as it had left to run.
#[test]
fn hiring_the_head_back_resumes_the_paused_craft_with_no_lost_progress() {
    let mut world = running_line(1_500);
    let vat = tile_of(&world, "cooking_vat");
    // Make sure the vat is genuinely mid-craft when it loses its crew.
    // Bounded: a bare `while` here hung the whole `cargo test -p sim_core`
    // run when the vat never crafted at all, instead of failing and naming
    // the reason. A fed vat crafts every 240-600 ticks, so 5_000 is slack.
    let mut budget = 5_000;
    while !matches!(progress_of(&world, vat), Some((elapsed, required)) if elapsed > 10 && elapsed + 10 < required)
    {
        common::supply(&mut world);
        world.tick();
        budget -= 1;
        assert!(
            budget > 0,
            "the cooking vat never got a craft under way, so there is no \
             paused craft to test: stall is {:?}",
            stall_of(&world, vat)
        );
    }
    let (elapsed_at_fire, required) = progress_of(&world, vat).unwrap();

    // Three heads, not one: with one gone the vat still runs on the ticks
    // the belt-limited crusher or mixer stands aside (see the test above),
    // and this test is about a craft that is genuinely PAUSED. With one
    // head left the crusher or the mixer takes it and the vat's two are
    // never free.
    for _ in 0..3 {
        fire(&mut world, "manufacturing");
    }
    assert_eq!(headcount(&world, "manufacturing"), 1);
    for _ in 0..100 {
        common::supply(&mut world);
        world.tick();
        assert_eq!(
            progress_of(&world, vat),
            Some((elapsed_at_fire, required)),
            "a paused craft's elapsed time stands still"
        );
    }

    for _ in 0..3 {
        hire(&mut world, "manufacturing");
    }
    assert_eq!(headcount(&world, "manufacturing"), 4);
    let remaining = required - elapsed_at_fire;
    for step in 1..remaining {
        common::supply(&mut world);
        world.tick();
        assert_eq!(
            progress_of(&world, vat),
            Some((elapsed_at_fire + step, required)),
            "after the crew returns the craft advances one tick per tick"
        );
        assert!(!matches!(
            stall_of(&world, vat),
            Some((StallKind::Staff, _))
        ));
    }
    // The completion tick: `elapsed == required` deposits and clears the
    // process (the vat's outlet pipe has room in a running line).
    common::supply(&mut world);
    world.tick();
    assert!(
        progress_of(&world, vat).is_none() || progress_of(&world, vat).unwrap().0 < elapsed_at_fire,
        "the batch landed exactly when it would have, less the pause"
    );
}

// ---------------------------------------------------------------------
// The dock
// ---------------------------------------------------------------------

/// Without its logistics head the dock holds what arrives: the boxes pile
/// on the dock (and then the belt), no contract can draw on them, nothing
/// vanishes, and they are on offer again the tick the head is back.
#[test]
fn firing_the_logistics_head_makes_the_dock_hold_and_loses_nothing() {
    let mut world = running_line(2_500);
    let dock = world.dock_snapshots()[0].clone();
    assert!(dock.shipped_total > 0, "the line reaches the dock by now");
    assert!(!dock.understaffed);

    // A dock claims its crew only while it holds something a contract
    // orders (`World::tick` step 6), so the wait below stays inside the
    // starter order's life: it would strike out at tick 3000 with the
    // dock's head gone, and a dock holding what nobody ordered asks for
    // nobody.
    assert!(world.economy().contract("corner_shop").is_some());

    let delivered_before = world.economy().lifetime_deliveries;
    fire(&mut world, "logistics");
    assert_eq!(headcount(&world, "logistics"), 0);

    for _ in 0..400 {
        common::supply(&mut world);
        world.tick();
    }
    let held = world.dock_snapshots()[0].clone();
    assert_eq!(
        world.economy().lifetime_deliveries,
        delivered_before,
        "nothing is delivered from an understaffed dock"
    );
    assert!(
        world.economy_snapshot().delivery_stock.is_empty(),
        "and nothing on it is on offer to a contract"
    );
    assert!(held.understaffed, "the dock says why");
    assert!(
        held.used > 0,
        "arriving boxes wait on the dock rather than disappearing"
    );
    assert!(held.used <= held.places);
    match world.query_tile(held.tile) {
        Some(TileSnapshot::Dock {
            crew, understaffed, ..
        }) => {
            assert!(understaffed);
            assert_eq!((crew[0].category.as_str(), crew[0].count), ("logistics", 1));
        }
        other => panic!("expected the dock, got {other:?}"),
    }
    let snapshot = world.staff_snapshot();
    let logistics = snapshot
        .categories
        .iter()
        .find(|c| c.id == "logistics")
        .unwrap();
    assert_eq!(
        (
            logistics.headcount,
            logistics.demand,
            logistics.assigned,
            logistics.short
        ),
        (0, 1, 0, 1)
    );

    hire(&mut world, "logistics");
    common::supply(&mut world);
    world.tick();
    let shipping = world.dock_snapshots()[0].clone();
    assert!(!shipping.understaffed);
    assert!(
        shipping.used >= held.used,
        "nothing was lost while it waited"
    );
    assert_eq!(
        world.economy_snapshot().delivery_stock,
        shipping.contents,
        "everything that waited on the dock is on offer the tick the crew is back"
    );
}

// ---------------------------------------------------------------------
// Payday
// ---------------------------------------------------------------------

#[test]
fn payday_debits_the_whole_bill_on_the_boundary_and_every_period_after() {
    let mut world = World::new(recipes());
    let pools = world.staff();
    let period = pools.catalog().pay_period_ticks();
    let bill = Money::from_num(pools.bill_per_period());
    assert!(bill > Money::ZERO);
    assert_eq!(pools.next_payday_tick(), period);
    let start = Money::from_num(STARTING_MONEY);

    while world.current_tick() < period - 1 {
        world.tick();
    }
    assert_eq!(world.money(), start, "nothing is owed before the boundary");
    world.tick();
    assert_eq!(world.current_tick(), period);
    assert_eq!(
        world.money(),
        start - bill,
        "the whole bill leaves on the payday tick"
    );
    let snapshot = world.staff_snapshot();
    assert_eq!(snapshot.next_payday_tick, 2 * period);
    assert_eq!(snapshot.last_bill, world.staff().bill_per_period());
    assert_eq!(snapshot.walked_out_last_payday, 0);
    assert_eq!(snapshot.money, (start - bill).to_num::<f64>());

    for _ in 0..period {
        world.tick();
    }
    assert_eq!(
        world.money(),
        start - bill - bill,
        "and again one period later"
    );
    assert_eq!(world.staff().next_payday_tick(), 3 * period);
    // Every head is still on the books: the bill was met.
    for c in StaffCatalog::bundled().categories() {
        assert_eq!(headcount(&world, &c.id), c.starting_headcount);
    }
}

/// A world with `money` in the bank and nothing built — the fixture for
/// every payday test, because the bill is the only thing that moves money.
fn empty_world_with_money(money: u32) -> World {
    World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(money),
    )
}

/// Sets the balance by rewriting the save, the way `test_layouts.gd`'s
/// `fund` does — the one legitimate way to state a balance rather than
/// earn it — and proves the pools came back identical on the way.
fn with_money(world: &World, money: u32) -> World {
    let mut save = world.to_save();
    save.money_bits = MoneyBits::of(Money::from_num(money));
    let loaded = World::from_save_ron(&save.to_ron(), recipes()).expect("the save must load");
    let mut expected = world.staff_snapshot();
    let mut actual = loaded.staff_snapshot();
    expected.money = 0.0;
    actual.money = 0.0;
    expected.shortfall = 0;
    actual.shortfall = 0;
    for c in expected
        .categories
        .iter_mut()
        .chain(actual.categories.iter_mut())
    {
        c.hire_warns = false;
    }
    assert_eq!(actual, expected, "the pools survive the save unchanged");
    loaded
}

fn unpaid(world: &World, category: &str) -> u32 {
    world.staff().pool(category).unwrap().unpaid
}

fn available(world: &World, category: &str) -> u32 {
    world.staff().pool(category).unwrap().available
}

/// **The reported bug** (the quality pass): a factory that spends down to
/// a few coins before a payday used to lose most of its crew silently.
/// Now the wire warns before the payday (`next_bill`, `shortfall`), the
/// first payday leaves the unpaid heads hired but idle (`unpaid`, per
/// category and in total, nobody `left`), and only a SECOND unpaid payday
/// lets them go — announced per category in `left_last_payday`. In
/// between, a payday that is covered clears the arrears.
#[test]
fn a_factory_that_cannot_pay_is_warned_then_idled_and_loses_its_crew_only_at_the_second_miss() {
    let catalog = StaffCatalog::bundled();
    let period = catalog.pay_period_ticks();
    // Enough for the first two manufacturing wages and part of the third.
    let manufacturing_wage = catalog.get("manufacturing").unwrap().wage;
    let money = 2 * manufacturing_wage + 1;
    let mut world = empty_world_with_money(money);
    let bill = world.staff().bill_per_period();
    assert!(bill > money, "the fixture must be unable to meet the bill");
    let total_start: u32 = catalog
        .categories()
        .iter()
        .map(|c| c.starting_headcount)
        .sum();

    // BEFORE: the warning is on the wire from the first tick.
    world.tick();
    let snapshot = world.staff_snapshot();
    assert_eq!(snapshot.next_bill, bill);
    assert_eq!(snapshot.shortfall, bill - money);
    assert_eq!(snapshot.unpaid_last_payday, 0);
    assert_eq!(snapshot.walked_out_last_payday, 0);

    // FIRST PAYDAY: two wages paid, nobody leaves, five are unpaid.
    while world.current_tick() < period {
        world.tick();
    }
    assert_eq!(
        world.money(),
        Money::from_num(1),
        "two wages paid, then nothing more"
    );
    for c in catalog.categories() {
        assert_eq!(
            headcount(&world, &c.id),
            c.starting_headcount,
            "{}: nobody left",
            c.id
        );
    }
    assert_eq!(unpaid(&world, "manufacturing"), 2);
    assert_eq!(unpaid(&world, "engineering"), 0);
    assert_eq!(
        unpaid(&world, "packaging"),
        2,
        "every wage-earner after the refusal is unpaid"
    );
    assert_eq!(unpaid(&world, "logistics"), 1);
    let snapshot = world.staff_snapshot();
    assert_eq!(snapshot.unpaid_last_payday, total_start - 2);
    assert_eq!(
        snapshot.walked_out_last_payday, 0,
        "a first miss is arrears, not a walk-out"
    );
    assert_eq!(snapshot.last_bill, 2 * manufacturing_wage);
    assert_eq!(
        snapshot.next_bill, bill,
        "arrears do not stack — one wage per head"
    );
    assert!(snapshot.categories.iter().all(|c| c.left_last_payday == 0));

    // The unpaid do not work: next tick only the paid heads are at their post.
    world.tick();
    assert_eq!(available(&world, "manufacturing"), 2);
    assert_eq!(available(&world, "packaging"), 0);
    assert_eq!(available(&world, "logistics"), 0);

    // A hire the next payday could not pay is WARNED about, not refused
    // (2026-09-26): the wire says so and the hire still happens, costing
    // nothing now — going into deeper arrears is what it costs. Tried on a
    // COPY, because a hire cannot be undone for the branches below: firing
    // the head back off leaves `owed` raised (notice pay), which is a
    // different world from the one that never hired.
    assert!(world.staff_snapshot().categories[3].hire_warns);
    let mut warned = with_money(&world, 1);
    hire(&mut warned, "logistics");
    assert_eq!(
        headcount(&warned, "logistics"),
        2,
        "the warned hire goes through"
    );
    assert_eq!(warned.money(), Money::from_num(1), "and costs nothing now");

    // Branch A — SECOND PAYDAY still broke: the heads in arrears leave, by
    // category, and the panel is told who; the two paid manufacturing heads
    // now go unpaid for the first time and stay.
    let mut broke = with_money(&world, 1);
    while broke.current_tick() < 2 * period {
        broke.tick();
    }
    assert_eq!(headcount(&broke, "manufacturing"), 2);
    assert_eq!(unpaid(&broke, "manufacturing"), 2);
    assert_eq!(headcount(&broke, "packaging"), 0);
    assert_eq!(headcount(&broke, "logistics"), 0);
    let snapshot = broke.staff_snapshot();
    assert_eq!(snapshot.walked_out_last_payday, 5);
    assert_eq!(snapshot.unpaid_last_payday, total_start);
    assert_eq!(snapshot.last_bill, 0);
    let left: Vec<(&str, u32)> = snapshot
        .categories
        .iter()
        .map(|c| (c.id.as_str(), c.left_last_payday))
        .collect();
    assert_eq!(
        left,
        vec![
            ("manufacturing", 2),
            ("engineering", 0),
            ("packaging", 2),
            ("logistics", 1)
        ]
    );
    // And the third: the last two go.
    while broke.current_tick() < 3 * period {
        broke.tick();
    }
    assert_eq!(headcount(&broke, "manufacturing"), 0);
    assert_eq!(broke.staff_snapshot().walked_out_last_payday, 2);
    assert_eq!(broke.money(), Money::from_num(1));

    // Branch B — money arrives before the second payday: the bill is met,
    // the arrears clear, everyone is back at work and nobody left.
    let mut recovered = with_money(&world, 100);
    assert_eq!(recovered.staff_snapshot().shortfall, 0);
    while recovered.current_tick() < 2 * period {
        recovered.tick();
    }
    assert_eq!(recovered.money(), Money::from_num(100 - bill));
    for c in catalog.categories() {
        assert_eq!(headcount(&recovered, &c.id), c.starting_headcount);
        assert_eq!(unpaid(&recovered, &c.id), 0);
    }
    let snapshot = recovered.staff_snapshot();
    assert_eq!(snapshot.unpaid_last_payday, 0);
    assert_eq!(snapshot.walked_out_last_payday, 0);
    assert_eq!(snapshot.last_bill, bill);
    recovered.tick();
    assert_eq!(available(&recovered, "manufacturing"), 4);
    assert_eq!(available(&recovered, "logistics"), 1);
}

/// **The fire-before-payday dodge is closed.** A head is owed for any
/// period it was employed in any part of: firing everyone the tick before
/// payday leaves the bill unchanged (notice pay), and hiring them all back
/// the tick after bills them again at the next one — two periods of
/// wages either way.
#[test]
fn firing_everyone_the_tick_before_payday_leaves_the_bill_unchanged() {
    let catalog = StaffCatalog::bundled();
    let period = catalog.pay_period_ticks();
    let mut world = empty_world_with_money(1_000);
    let bill = world.staff().bill_per_period();

    while world.current_tick() < period - 1 {
        world.tick();
    }
    for c in catalog.categories() {
        for _ in 0..c.starting_headcount {
            fire(&mut world, &c.id);
        }
        assert_eq!(headcount(&world, &c.id), 0);
    }
    assert_eq!(
        world.staff().bill_per_period(),
        0,
        "nobody is on the books..."
    );
    assert_eq!(
        world.staff_snapshot().next_bill,
        bill,
        "...but everybody is owed"
    );
    world.tick();
    assert_eq!(world.current_tick(), period);
    assert_eq!(
        world.money(),
        Money::from_num(1_000 - bill),
        "the bill is exactly what keeping them would have cost"
    );
    let snapshot = world.staff_snapshot();
    assert_eq!(snapshot.last_bill, bill);
    assert_eq!(snapshot.next_bill, 0, "the notice is settled");
    assert_eq!(
        (snapshot.unpaid_last_payday, snapshot.walked_out_last_payday),
        (0, 0)
    );

    // Hired back the tick after: owed in full at the next payday, even
    // though they were employed for one tick less than a period — and a
    // head hired one tick BEFORE that payday is owed in full too.
    for c in catalog.categories() {
        for _ in 0..c.starting_headcount {
            hire(&mut world, &c.id);
        }
        assert_eq!(headcount(&world, &c.id), c.starting_headcount);
    }
    assert_eq!(world.staff_snapshot().next_bill, bill);
    while world.current_tick() < 2 * period - 1 {
        world.tick();
    }
    hire(&mut world, "packaging");
    let packaging_wage = catalog.get("packaging").unwrap().wage;
    assert_eq!(world.staff_snapshot().next_bill, bill + packaging_wage);
    world.tick();
    assert_eq!(
        world.money(),
        Money::from_num(1_000 - 2 * bill - packaging_wage),
        "two periods of wages, plus the head hired one tick before payday"
    );
}

/// Notice pay is settled FIRST, so a broke factory cannot make its unpaid
/// heads absorb what it owes the ones it fired: with money for one head,
/// the fired manufacturing head's notice takes it and the hired heads are
/// the unpaid ones.
#[test]
fn notice_pay_is_settled_before_the_heads_still_hired() {
    let catalog = StaffCatalog::bundled();
    let period = catalog.pay_period_ticks();
    let wage = catalog.get("manufacturing").unwrap().wage;
    let mut world = empty_world_with_money(wage);
    fire(&mut world, "manufacturing");
    assert_eq!(headcount(&world, "manufacturing"), 3);
    while world.current_tick() < period {
        world.tick();
    }
    assert_eq!(
        world.money(),
        Money::ZERO,
        "the one wage went to the fired head"
    );
    assert_eq!(headcount(&world, "manufacturing"), 3, "nobody left");
    assert_eq!(
        unpaid(&world, "manufacturing"),
        3,
        "and all three hired heads are unpaid"
    );
    assert_eq!(world.staff_snapshot().last_bill, wage);
}

// ---------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------

#[test]
fn hire_and_fire_round_trip_through_json_and_dirty_no_chunk() {
    let mut world = reference_chain_with_budget(recipes(), 1_000);
    world.tick();
    let before = headcount(&world, "packaging");

    let hire_cmd =
        command_from_json(r#"{"type": "hire", "category": "packaging"}"#).expect("hire parses");
    assert!(matches!(&hire_cmd, PlayerCommand::Hire { category } if category == "packaging"));
    let dirty_before = world.dirty_chunks().to_vec();
    world.issue_command(hire_cmd);
    assert_eq!(headcount(&world, "packaging"), before + 1);
    assert_eq!(
        world.dirty_chunks(),
        dirty_before.as_slice(),
        "hire dirties no chunk"
    );

    let fire =
        command_from_json(r#"{"type": "fire", "category": "packaging"}"#).expect("fire parses");
    assert!(matches!(&fire, PlayerCommand::Fire { category } if category == "packaging"));
    world.issue_command(fire);
    assert_eq!(headcount(&world, "packaging"), before);
    assert_eq!(
        world.dirty_chunks(),
        dirty_before.as_slice(),
        "fire dirties no chunk"
    );

    // Silent no-ops: fire at zero, and either with an unknown category.
    assert_eq!(headcount(&world, "engineering"), 0);
    world.issue_command(
        command_from_json(r#"{"type": "fire", "category": "engineering"}"#).unwrap(),
    );
    assert_eq!(headcount(&world, "engineering"), 0);
    world.issue_command(command_from_json(r#"{"type": "hire", "category": "plumbers"}"#).unwrap());
    world.issue_command(command_from_json(r#"{"type": "fire", "category": "plumbers"}"#).unwrap());
    let snapshot = world.staff_snapshot();
    assert_eq!(snapshot.categories.len(), 4, "no category was invented");
    assert!(
        command_from_json(r#"{"type": "hire"}"#).is_none(),
        "category is required"
    );
    // Hiring is free — the wage comes at payday — and since 2026-09-26 a
    // balance that could not pay it does not refuse the hire, it warns:
    // a world with nothing in the bank hires and goes into arrears.
    let money = world.money();
    world.issue_command(PlayerCommand::Hire {
        category: "manufacturing".to_string(),
    });
    assert_eq!(world.money(), money, "a hire costs nothing today");
    let mut broke = reference_chain(recipes());
    assert_eq!(
        broke.money(),
        Money::ZERO,
        "the plain reference chain spends its whole budget"
    );
    let before = headcount(&broke, "manufacturing");
    assert!(
        broke.staff_snapshot().categories[0].hire_warns,
        "an empty balance cannot pay the next bill plus one more wage"
    );
    hire(&mut broke, "manufacturing");
    assert_eq!(
        headcount(&broke, "manufacturing"),
        before + 1,
        "warned, not refused: the head is hired and goes unpaid at payday"
    );
}

// ---------------------------------------------------------------------
// A kind with no crew
// ---------------------------------------------------------------------

/// A kind with `staff: []` never asks: a world whose kinds all have an
/// empty list ships with nobody on the books and never reports a `staff`
/// stall. (On the shipped registry the hopper, tank and splitter are the
/// crew-less kinds; the fed-line test above sees exactly the three recipe
/// machines' four heads in `demand`, never a fifth for the hopper.)
#[test]
fn a_machine_with_no_crew_never_asks() {
    let text = include_str!("../../data/machines.ron")
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("staff:") {
                "            staff: [],".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let uncrewed = MachineRegistry::from_ron_str(&text).unwrap();
    let mut world = sim_core::sim::reference_chain_with_content_and_budget(
        recipes(),
        uncrewed,
        ContractRegistry::bundled(),
        common::STOCK_COST,
    );
    common::stock_up(&mut world);
    for c in StaffCatalog::bundled().categories() {
        for _ in 0..c.starting_headcount {
            fire(&mut world, &c.id);
        }
    }
    assert_eq!(world.staff().bill_per_period(), 0);
    for _ in 0..2_500 {
        common::supply(&mut world);
        world.tick();
        for entry in world.machine_load_snapshots() {
            assert!(
                !matches!(&entry.stall, Some(s) if s.kind == StallKind::Staff),
                "a kind with staff: [] never asks, so it is never refused"
            );
        }
        for c in &world.staff_snapshot().categories {
            assert_eq!((c.demand, c.assigned, c.short), (0, 0, 0));
        }
    }
    assert!(
        world.dock_snapshots()[0].shipped_total > 0,
        "the line ships unstaffed"
    );
    assert!(!world.dock_snapshots()[0].understaffed);
    assert_eq!(world.staff_snapshot().last_bill, 0);
    assert_eq!(world.staff_snapshot().walked_out_last_payday, 0);
}

// ---------------------------------------------------------------------
// The wire
// ---------------------------------------------------------------------

#[test]
fn staff_snapshot_json_has_the_documented_shape_in_staff_ron_order() {
    // Far enough in that every machine has asked (tick 1200: one payday
    // behind it, the line running), so the printed sample is a live one.
    let world = running_line(1_200);
    // Printed so `cargo test --test staff -- --nocapture` shows a real one.
    println!(
        "{}",
        serde_json::to_string(&world.staff_snapshot()).unwrap()
    );
    let json = serde_json::to_value(world.staff_snapshot()).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "bill_per_period",
            "categories",
            "last_bill",
            "money",
            "next_bill",
            "next_payday_tick",
            "pay_period_ticks",
            "shortfall",
            "unpaid_last_payday",
            "walked_out_last_payday"
        ]
    );
    assert_eq!(json["pay_period_ticks"], 1000);
    assert_eq!(json["next_payday_tick"], 2000);
    assert_eq!(json["last_bill"], 14);
    assert_eq!(json["next_bill"], 14);
    assert_eq!(json["bill_per_period"], 14, "nothing is stood down");
    assert_eq!(json["shortfall"], 0);
    assert_eq!(json["unpaid_last_payday"], 0);
    assert_eq!(json["walked_out_last_payday"], 0);
    assert_eq!(json["money"], world.money().to_num::<f64>());
    let ids: Vec<&str> = json["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, StaffCatalog::bundled().category_ids());
    for c in json["categories"].as_array().unwrap() {
        let mut keys: Vec<&str> = c.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "assigned",
                "demand",
                "headcount",
                "hire_warns",
                "id",
                "left_last_payday",
                "name",
                "on_payroll",
                "owed",
                "required",
                "short",
                "sources",
                "stood_down",
                "unpaid",
                "wage"
            ]
        );
        assert_eq!(
            c["owed"], c["headcount"],
            "nobody hired or fired this period"
        );
        assert_eq!(c["unpaid"], 0);
        assert_eq!(c["left_last_payday"], 0);
        assert_eq!(
            c["hire_warns"], false,
            "the fixture is funded for 300 periods"
        );
        assert_eq!(
            c["short"].as_u64().unwrap(),
            c["demand"].as_u64().unwrap() - c["assigned"].as_u64().unwrap()
        );
    }
}

#[test]
fn crew_rides_query_tile_and_the_shop_snapshot_and_the_stall_kind_spells_staff() {
    let world = reference_chain(recipes());
    let shop = serde_json::to_value(world.shop_snapshot()).unwrap();
    let entry = |kind: &str| {
        shop["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["kind"] == kind)
            .unwrap()
            .clone()
    };
    assert_eq!(entry("belt")["crew"], serde_json::json!([]));
    assert_eq!(entry("hopper")["crew"], serde_json::json!([]));
    assert_eq!(
        entry("cooking_vat")["crew"],
        serde_json::json!([{"category": "manufacturing", "count": 2}])
    );
    assert_eq!(
        entry("dock")["crew"],
        serde_json::json!([{"category": "logistics", "count": 1}])
    );
    let vat =
        serde_json::to_value(world.query_tile(tile_of(&world, "cooking_vat")).unwrap()).unwrap();
    assert_eq!(
        vat["crew"],
        serde_json::json!([{"category": "manufacturing", "count": 2}])
    );
    let dock = serde_json::to_value(world.query_tile(tile_of(&world, "dock")).unwrap()).unwrap();
    assert_eq!(
        dock["crew"],
        serde_json::json!([{"category": "logistics", "count": 1}])
    );
    assert_eq!(dock["understaffed"], false);
    let dock_snapshot = serde_json::to_value(&world.dock_snapshots()[0]).unwrap();
    assert_eq!(dock_snapshot["understaffed"], false);
    assert_eq!(serde_json::to_value(StallKind::Staff).unwrap(), "staff");
}

/// Issue #87: the dock's POLLED snapshot carries the standing `crewed`
/// fact, not only the per-tick `understaffed` one — and it follows hiring
/// and firing, which deliberately dirty no chunk.
///
/// This is the whole reason the field rides `dock_snapshots` as well as
/// `query_tile`: the presentation reads the former every frame and the
/// latter only when the scene is rebuilt, so a `crewed` available only on
/// the rebuild channel would go stale the moment a clerk was let go.
#[test]
fn the_docks_polled_snapshot_carries_crewed_and_it_follows_hiring() {
    let mut world = reference_chain_with_budget(recipes(), 1_000);
    world.tick();
    assert!(
        world.dock_snapshots()[0].crewed,
        "the reference chain hires the clerk its dock asks for"
    );

    let dirty_before = world.dirty_chunks().to_vec();
    fire(&mut world, "logistics");
    assert_eq!(headcount(&world, "logistics"), 0);
    assert_eq!(
        world.dirty_chunks(),
        dirty_before.as_slice(),
        "firing still dirties no chunk — the polled snapshot is how this reaches the HUD"
    );
    assert!(
        !world.dock_snapshots()[0].crewed,
        "a dock whose only clerk was fired is not crewed"
    );

    hire(&mut world, "logistics");
    assert!(
        world.dock_snapshots()[0].crewed,
        "hiring the clerk back crews the dock again"
    );
    // And the two flags are independent: the standing answer says nothing
    // about whether this particular tick refused a shipment.
    assert!(!world.dock_snapshots()[0].understaffed);
    assert_eq!(
        serde_json::to_value(&world.dock_snapshots()[0]).unwrap()["crewed"],
        true,
        "and it is on the wire under that name"
    );
}

/// A `staff` stall is reported only when nothing older applies: an idle
/// machine with no input reads `input`, never `staff`, even with nobody
/// hired.
#[test]
fn a_staff_stall_never_masks_an_input_shortfall() {
    let mut world = reference_chain(recipes());
    for c in StaffCatalog::bundled().categories() {
        for _ in 0..c.starting_headcount {
            fire(&mut world, &c.id);
        }
    }
    for _ in 0..50 {
        world.tick();
    }
    let crusher = tile_of(&world, "crusher");
    assert_eq!(
        stall_of(&world, crusher).map(|(k, _)| k),
        Some(StallKind::Input),
        "an unfed crusher wants ingredients, not people"
    );
    for c in &world.staff_snapshot().categories {
        assert_eq!(
            c.demand, 0,
            "a machine that would not have worked asks for nobody"
        );
    }
}

// ---------------------------------------------------------------------
// Save / load
// ---------------------------------------------------------------------

fn fingerprint(world: &World) -> String {
    let mut out = format!("tick={}\n", world.current_tick());
    out.push_str(&serde_json::to_string(&world.world_layout()).unwrap());
    out.push_str(&serde_json::to_string(&world.machine_load_snapshots()).unwrap());
    out.push_str(&serde_json::to_string(&world.dock_snapshots()).unwrap());
    out.push_str(&serde_json::to_string(&world.economy_snapshot()).unwrap());
    out.push_str(&serde_json::to_string(&world.hopper_snapshots()).unwrap());
    out.push_str(&serde_json::to_string(&world.fluid_snapshots()).unwrap());
    for chunk in world.all_chunks() {
        out.push_str(&serde_json::to_string(&world.chunk_snapshot(chunk)).unwrap());
    }
    out
}

/// A world with a fired head, a mid-craft machine paused for want of it and
/// a payday already behind it comes back identical and stays identical in
/// lockstep — including the staff snapshot from the first tick on.
#[test]
fn a_fired_head_and_a_paused_craft_survive_a_save() {
    let mut original = running_line(1_500);
    let vat = tile_of(&original, "cooking_vat");
    fire(&mut original, "manufacturing");
    for _ in 0..60 {
        common::supply(&mut original);
        original.tick();
    }
    assert_eq!(
        stall_of(&original, vat).map(|(k, _)| k),
        Some(StallKind::Staff),
        "the fixture must be saving a refused machine"
    );
    assert!(
        progress_of(&original, vat).is_some(),
        "...that is mid-craft"
    );
    assert!(
        original.staff().next_payday_tick() > 1_000,
        "...after a payday"
    );

    let text = original.to_save_ron();
    assert!(text.contains("staff: Some("), "the pools are in the file");
    assert!(text.contains("refused_by: Some(\"manufacturing\")"));
    let mut loaded = World::from_save_ron(&text, recipes()).expect("the save must load");

    assert_eq!(loaded.staff().headcount("manufacturing"), 3);
    assert_eq!(
        loaded.staff().next_payday_tick(),
        original.staff().next_payday_tick()
    );
    assert_eq!(loaded.staff().last_bill(), original.staff().last_bill());
    assert_eq!(fingerprint(&loaded), fingerprint(&original));
    assert_eq!(
        stall_of(&loaded, vat).map(|(k, _)| k),
        Some(StallKind::Staff),
        "the refusal is back before the first tick"
    );

    for step in 0..1_200 {
        common::supply(&mut original);
        common::supply(&mut loaded);
        original.tick();
        loaded.tick();
        assert_eq!(
            fingerprint(&loaded),
            fingerprint(&original),
            "diverged at step {step}"
        );
        assert_eq!(
            serde_json::to_string(&loaded.staff_snapshot()).unwrap(),
            serde_json::to_string(&original.staff_snapshot()).unwrap(),
            "staff snapshots diverged at step {step}"
        );
    }
    assert_eq!(original.to_save_ron(), loaded.to_save_ron());
}

/// A save written before the epic has no `staff` at all: it loads with the
/// starting crew and a payday one period from its own tick, and an
/// understaffed flag or refused category it never had reads false / none.
#[test]
fn a_pre_epic_save_loads_with_the_starting_crew_and_a_payday_one_period_out() {
    let world = running_line(1_234);
    let mut save = world.to_save();
    save.staff = None;
    let text = save.to_ron();
    assert!(!text.contains("staff: Some("));
    let loaded = World::from_save_ron(&text, recipes()).unwrap();
    let catalog = StaffCatalog::bundled();
    for c in catalog.categories() {
        assert_eq!(loaded.staff().headcount(&c.id), c.starting_headcount);
    }
    assert_eq!(
        loaded.staff().next_payday_tick(),
        1_234 + catalog.pay_period_ticks()
    );
    assert_eq!(loaded.staff().last_bill(), 0);
    assert_eq!(loaded.staff().walked_out_last_payday(), 0);
    assert_eq!(loaded.staff().unpaid_last_payday(), 0);
    for c in catalog.categories() {
        let pool = loaded.staff().pool(&c.id).unwrap();
        assert_eq!(
            (pool.unpaid, pool.owed, pool.left_last_payday),
            (0, c.starting_headcount, 0)
        );
    }

    // A save from the staff pools MVP (headcounts and the clock, none of
    // the quality pass's fields): nobody in arrears, every head owed.
    let mut mvp = world.to_save();
    let staff = mvp.staff.as_mut().unwrap();
    staff.unpaid.clear();
    staff.owed.clear();
    staff.left_last_payday.clear();
    staff.unpaid_last_payday = 0;
    // Cleared maps still print as `owed: {}`; cut the keys out entirely.
    let mut text = mvp.to_ron();
    for key in [
        "unpaid:",
        "owed:",
        "left_last_payday:",
        "unpaid_last_payday:",
    ] {
        text = without(&text, key);
    }
    assert!(!text.contains("owed") && !text.contains("unpaid"), "{text}");
    let loaded = World::from_save_ron(&text, recipes()).unwrap();
    let full = World::from_save_ron(&world.to_save_ron(), recipes()).unwrap();
    assert_eq!(
        serde_json::to_string(&loaded.staff_snapshot()).unwrap(),
        serde_json::to_string(&full.staff_snapshot()).unwrap(),
        "a paid-up crew's save reads the same with or without the new fields"
    );

    // And the strict form of "not there": the keys removed from the text.
    // Cut `key: value,` (or `key: value` before a closing paren) wherever
    // either key appears, whatever the RON's line breaks.
    fn without(text: &str, key: &str) -> String {
        let mut out = text.to_string();
        while let Some(i) = out.find(key) {
            let rest = &out[i..];
            let stop = rest.find([',', ')']).expect("a field ends");
            let end = if rest.as_bytes()[stop] == b',' {
                stop + 1
            } else {
                stop
            };
            out = format!("{}{}", &out[..i], &rest[end..]);
        }
        out
    }
    let stripped = without(
        &without(&world.to_save_ron(), "understaffed:"),
        "refused_by:",
    );
    assert!(!stripped.contains("understaffed") && !stripped.contains("refused_by"));
    let loaded = World::from_save_ron(&stripped, recipes()).expect("defaults fill the gaps");
    assert_eq!(loaded.dock_snapshots().len(), 1);
    assert!(!loaded.dock_snapshots()[0].understaffed);
    for entry in loaded.machine_load_snapshots() {
        assert!(!matches!(entry.stall, Some(s) if s.kind == StallKind::Staff));
    }
}

/// Arrears, owed counts and last payday's leavers ride the save and come
/// back exactly; an impossible state (more in arrears than hired) is
/// refused with a reason, like every other corrupt save.
#[test]
fn arrears_survive_a_save_and_an_impossible_state_is_refused() {
    let period = StaffCatalog::bundled().pay_period_ticks();
    let mut world = empty_world_with_money(3);
    fire(&mut world, "packaging");
    while world.current_tick() < period + 5 {
        world.tick();
    }
    let snapshot = world.staff_snapshot();
    assert!(
        snapshot.unpaid_last_payday > 0,
        "the fixture must be in arrears"
    );
    assert_eq!(unpaid(&world, "manufacturing"), 3);

    let text = world.to_save_ron();
    assert!(
        text.contains("unpaid:") && text.contains("owed:") && text.contains("left_last_payday:")
    );
    let mut loaded = World::from_save_ron(&text, recipes()).unwrap();
    assert_eq!(
        serde_json::to_string(&loaded.staff_snapshot()).unwrap(),
        serde_json::to_string(&snapshot).unwrap()
    );
    for step in 0..period {
        world.tick();
        loaded.tick();
        assert_eq!(
            serde_json::to_string(&loaded.staff_snapshot()).unwrap(),
            serde_json::to_string(&world.staff_snapshot()).unwrap(),
            "staff snapshots diverged at step {step}"
        );
    }
    assert_eq!(loaded.money(), world.money());

    let mut save = world.to_save();
    save.staff
        .as_mut()
        .unwrap()
        .unpaid
        .insert("logistics".to_string(), 9);
    let message = World::from_save_ron(&save.to_ron(), recipes())
        .err()
        .expect("more in arrears than hired is a corrupt save")
        .to_string();
    assert!(message.contains("logistics"), "{message}");
    assert!(message.contains("unpaid"), "{message}");
    let mut save = world.to_save();
    save.staff
        .as_mut()
        .unwrap()
        .owed
        .insert("manufacturing".to_string(), 0);
    // Fewer OWED than hired is not corrupt — since standing down (issue
    // #19) it is what a save taken during a shutdown looks like — so it
    // loads, and the establishment rebuilt from the replayed placements
    // decides how much of it stands. Nothing is mothballed in this world,
    // so every head goes straight back on the payroll.
    let loaded = World::from_save_ron(&save.to_ron(), recipes())
        .expect("fewer owed than hired is a shutdown, not a corrupt save");
    assert_eq!(
        loaded.staff().pool("manufacturing").unwrap().owed,
        loaded.staff().headcount("manufacturing"),
    );
}

#[test]
fn a_save_naming_an_unknown_staff_category_is_refused() {
    let world = running_line(10);
    let mut save = world.to_save();
    let staff: &mut SavedStaff = save.staff.as_mut().unwrap();
    staff.headcount.insert("plumbers".to_string(), 3);
    let message = World::from_save_ron(&save.to_ron(), recipes())
        .err()
        .expect("an unknown category is a corrupt save")
        .to_string();
    assert!(message.contains("plumbers"), "{message}");
    assert!(message.contains("staff.ron"), "{message}");
}

/// Every save written before 2026-10-02 names the engineering crew by its
/// old id, `maintenance`. `data/staff.ron`'s `renamed_from` maps it: the
/// counts land on `engineering` — in every map, and on a machine the crew
/// refused — and the next save writes only the new id. No schema bump.
#[test]
fn a_save_naming_the_old_maintenance_crew_loads_into_engineering() {
    let world = running_line(10);
    let mut save = world.to_save();
    let staff: &mut SavedStaff = save.staff.as_mut().unwrap();
    for map in [
        &mut staff.headcount,
        &mut staff.unpaid,
        &mut staff.owed,
        &mut staff.left_last_payday,
    ] {
        map.remove("engineering");
    }
    staff.headcount.insert("maintenance".to_string(), 3);
    staff.unpaid.insert("maintenance".to_string(), 1);
    staff.owed.insert("maintenance".to_string(), 3);
    staff.left_last_payday.insert("maintenance".to_string(), 2);
    let machine = save
        .placements
        .iter_mut()
        .find_map(|p| match &mut p.body {
            SavedBody::Machine(m) => Some(m),
            _ => None,
        })
        .expect("the reference line has a machine");
    machine.refused_by = Some("maintenance".to_string());
    let text = save.to_ron();
    assert!(
        text.contains("\"maintenance\""),
        "the fixture is an old save"
    );

    let loaded = World::from_save_ron(&text, recipes()).expect("an old save must load");
    let pool = loaded.staff().pool("engineering").unwrap();
    assert_eq!(
        (
            pool.headcount,
            pool.unpaid,
            pool.owed,
            pool.left_last_payday
        ),
        (3, 1, 3, 2)
    );
    assert!(loaded.staff().pool("maintenance").is_none());
    let resaved = loaded.to_save_ron();
    assert!(!resaved.contains("maintenance"), "{resaved}");
    assert!(resaved.contains("refused_by: Some(\"engineering\")"));
    assert!(World::from_save_ron(&resaved, recipes()).is_ok());
}

/// A map naming one category under both its ids could keep only one of
/// the two counts, so it is a corrupt save, not a silent pick.
#[test]
fn a_save_naming_a_category_under_both_its_ids_is_refused() {
    let world = running_line(10);
    let mut save = world.to_save();
    let staff: &mut SavedStaff = save.staff.as_mut().unwrap();
    staff.headcount.insert("engineering".to_string(), 1);
    staff.headcount.insert("maintenance".to_string(), 2);
    let message = World::from_save_ron(&save.to_ron(), recipes())
        .err()
        .expect("two counts for one category is a corrupt save")
        .to_string();
    assert!(message.contains("engineering"), "{message}");
    assert!(message.contains("maintenance"), "{message}");
}

/// The shipped catalog keeps the old id, and the loader refuses a former
/// id that would make a save's id mean two categories.
#[test]
fn the_shipped_catalog_maps_maintenance_and_rejects_a_colliding_former_id() {
    let shipped = StaffCatalog::bundled();
    assert_eq!(shipped.resolve("maintenance").unwrap().id, "engineering");
    assert!(shipped.get("maintenance").is_none());
    let text = include_str!("../../data/staff.ron");
    for collision in [
        r#"["maintenance", "packaging"]"#,
        r#"["maintenance", "maintenance"]"#,
    ] {
        let bad = text.replacen(r#"["maintenance"]"#, collision, 1);
        assert_ne!(bad, text, "the fixture edit must apply");
        assert!(
            matches!(
                StaffCatalog::from_ron_str(&bad),
                Err(StaffLoadError::AmbiguousFormerId { .. })
            ),
            "{collision} must be refused"
        );
    }
}
