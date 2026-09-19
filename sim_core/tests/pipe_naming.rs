//! Headless integration test for issue #54: **a pipe run that can carry
//! nothing readable says so.**
//!
//! `resolve_network_fluids` has always narrowed each run to the set of
//! fluids it could hold and left a run that narrowed to none — or to
//! several — unnamed. That is still the rule; nothing here changes the
//! narrowing. What is under test is that the *outcome* reaches the wire:
//! an unnamed run now reports which of the two happened and what it
//! narrowed to, and every machine on it stalls with a reason that names a
//! pipe rather than claiming there isn't one.
//!
//! Both fixtures are real player mistakes, not synthetic ones, and both are
//! built out of `sim::bbq_branch` — the layout on `main` — by issuing one
//! further command a player could issue:
//!
//! 1. **The liquid-smoke tank turned away from its run** — literally how
//!    #54 was hit while building the BBQ branch. A tank discharges only
//!    into the pipe it *faces*, so the run loses its only pin to a raw
//!    fluid and narrows to two candidates instead of one: `ambiguous`.
//! 2. **A tank plumbed into the run between two machines** — the mirror
//!    mistake, "I'll feed the mix line straight from a tank". A run a tank
//!    discharges into can hold only a raw fluid, and the only thing that
//!    can reach that run is the mixer's blended output, so it narrows to
//!    nothing at all: `unnameable`.
//!
//! The third case is the one that must NOT shout: a length of pipe joined
//! to nothing is `unplumbed`, because it is what every run looks like while
//! the player is still laying it.

use sim_core::sim::fluid::PipeNaming;
use sim_core::sim::types::Direction;
use sim_core::sim::world::{FluidSnapshot, StallKind};
use sim_core::sim::{PlayerCommand, RecipeRegistry, World, bbq_branch};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("data/recipes.ron must parse")
}

/// The BBQ branch with enough money to place one more thing — the layout a
/// player who followed the plan has, before they make the mistake.
fn branch() -> World {
    bbq_branch(recipes(), 20_000)
}

/// The run that owns `tile`, by its segments rather than by index: the
/// topology renumbers networks on every placement, so an index pinned in a
/// test would be pinning the wrong thing.
fn run_at(world: &World, tile: (i32, i32)) -> FluidSnapshot {
    world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|segment| segment.tile == tile))
        .unwrap_or_else(|| panic!("no pipe run covers {tile:?}"))
}

/// Why the machine whose origin is `tile` is waiting, as the wire reports
/// it — the same `machine_load_snapshots` the hover panel polls.
fn stall_at(world: &World, tile: (i32, i32)) -> Option<(StallKind, String)> {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|entry| entry.tile == tile)
        .unwrap_or_else(|| panic!("no machine at {tile:?}"))
        .stall
        .map(|stall| (stall.kind, stall.item))
}

fn purchase_and_place(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item: None,
    });
}

/// The control the other two tests are read against: as shipped, every run
/// on the branch is named, and an unfilled one is *empty*, which is a
/// completely different state from unnamed and now says so.
#[test]
fn every_run_of_the_shipped_branch_is_named_and_merely_empty() {
    let world = branch();
    for run in world.fluid_snapshots() {
        assert_eq!(
            run.naming,
            PipeNaming::Carries,
            "the shipped branch names every run; {:?} did not",
            run.segments.iter().map(|s| s.tile).collect::<Vec<_>>()
        );
        assert!(run.fluid.is_some());
        assert_eq!(run.candidates.len(), 1);
        assert!(!run.naming.is_fault());
        // Empty, and named: the pairing that used to be indistinguishable
        // from the two faults below.
        assert_eq!(run.total_volume, 0.0);
    }
}

/// Issue #54's own case. The liquid-smoke tank at (16, -1) faces east into
/// the head of its run; turned, it discharges into bare ground, and the run
/// it was pinning is left with two fluids it could be.
#[test]
fn a_tank_turned_off_its_run_leaves_the_run_ambiguous() {
    let mut world = branch();
    assert_eq!(
        run_at(&world, (17, -1)).fluid.as_deref(),
        Some("liquid_smoke")
    );

    world.issue_command(PlayerCommand::Rotate { tile: (16, -1) });

    let smoke = run_at(&world, (17, -1));
    assert_eq!(smoke.naming, PipeNaming::Ambiguous);
    assert!(smoke.naming.is_fault());
    assert_eq!(smoke.fluid, None);
    // Both ends of the ambiguity are reported, because which two fluids
    // have been confused is the whole of what the player needs to know.
    assert_eq!(smoke.candidates, vec!["ketchup_mix", "liquid_smoke"]);
    // Every tile of the run carries the run's verdict, so a renderer
    // walking a chunk can mark it without rebuilding the networks.
    assert!(
        smoke
            .segments
            .iter()
            .all(|segment| segment.naming == PipeNaming::Ambiguous)
    );

    // And the vat blames the run, where it used to report "no supply pipe"
    // about a pipe the player had laid. The `item` is whichever fluid its
    // CURRENT recipe wants (an idle vat still holds the pick it was given),
    // which is why only the kind is pinned here: the kind is the sentence
    // the hover panel reads, and it is the half that used to be wrong.
    assert_eq!(
        stall_at(&world, (17, -3)).map(|(kind, _)| kind),
        Some(StallKind::AmbiguousPipe),
        "the BBQ vat must blame the run, not report a missing pipe"
    );
}

/// The mirror mistake: a tank discharging into the run between the mixer
/// and the vat. Only a raw fluid can enter the graph from outside, and only
/// a blended mix can reach that run from inside, so nothing can name it.
#[test]
fn a_tank_plumbed_into_a_machine_run_leaves_it_unnameable() {
    let mut world = branch();
    assert_eq!(run_at(&world, (15, -3)).fluid.as_deref(), Some("bbq_mix"));

    purchase_and_place(&mut world, "fluid_tank", (15, -2), Direction::South);

    let mix = run_at(&world, (15, -3));
    assert_eq!(mix.naming, PipeNaming::Unnameable);
    assert!(mix.naming.is_fault());
    assert_eq!(mix.fluid, None);
    assert!(mix.candidates.is_empty(), "nothing survived the narrowing");

    // The vat cannot draw from it, and says so — rather than the old
    // "no supply pipe" about the pipe running into its own footprint.
    assert_eq!(
        stall_at(&world, (17, -3)).map(|(kind, _)| kind),
        Some(StallKind::UnnameablePipe),
        "the BBQ vat must say its supply run cannot be named"
    );
    // The mixer on the other end is reported against its own first
    // shortfall, which at tick zero is a solid one (`Stall`'s precedence:
    // solids before fluids). That is deliberately unchanged — the point of
    // the two new kinds is that a fluid shortfall stops lying, not that it
    // jumps the queue.
    assert_eq!(
        stall_at(&world, (14, -3)).map(|(kind, _)| kind),
        Some(StallKind::Input)
    );
}

/// The state that must stay quiet. Pipe joined to nothing narrows to
/// nothing too — but nothing is stalled by it, it is simply unfinished, and
/// a fault marker on every tile a player lays would be noise.
#[test]
fn pipe_joined_to_nothing_is_unplumbed_rather_than_a_fault() {
    let mut world = branch();
    for x in 0..3 {
        purchase_and_place(&mut world, "pipe", (4 + x, -6), Direction::East);
    }

    let laid = run_at(&world, (4, -6));
    assert_eq!(laid.naming, PipeNaming::Unplumbed);
    assert!(!laid.naming.is_fault());
    assert_eq!(laid.fluid, None);
    assert!(laid.candidates.is_empty());
}

/// The wire spelling, pinned: Godot matches on these strings, and the two
/// faults exist precisely so it can tell them apart.
#[test]
fn the_naming_states_are_distinct_on_the_wire() {
    let mut world = branch();
    world.issue_command(PlayerCommand::Rotate { tile: (16, -1) });
    let json = serde_json::to_string(&run_at(&world, (17, -1))).expect("a run must serialize");
    assert!(
        json.contains(r#""naming":"ambiguous""#),
        "ambiguous must reach the wire under that name: {json}"
    );
    assert!(json.contains(r#""candidates":["ketchup_mix","liquid_smoke"]"#));

    let mut world = branch();
    purchase_and_place(&mut world, "fluid_tank", (15, -2), Direction::South);
    let json = serde_json::to_string(&run_at(&world, (15, -3))).expect("a run must serialize");
    assert!(
        json.contains(r#""naming":"unnameable""#),
        "unnameable must reach the wire under that name: {json}"
    );
}
