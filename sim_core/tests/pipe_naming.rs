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
//! 1. **A second tank declared as something else, discharging into the
//!    liquid-smoke run** — two declarations the player made disagree, and
//!    the resolver will not pick between them: `ambiguous`. (#54 was first
//!    hit by turning the smoke tank away from its run. Since 2026-10-03 that
//!    no longer confuses the run: the vat's chosen recipe pins it, and the
//!    vat says it is waiting for liquid smoke — see the test of that name.)
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
use sim_core::sim::{
    ContractRegistry, MachineRegistry, Money, PlayerCommand, RecipeRegistry, World, bbq_branch,
};

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
        recipe: None,
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

/// Issue #54's own mistake, and why it stopped being one. The liquid-smoke
/// tank at (16, -1) faces east into the head of its run; turned, it
/// discharges into bare ground. That used to leave the run with every fluid
/// any vat recipe draws as a candidate — ambiguous — because a vat claimed
/// for its whole kind. A vat set to BBQ claims only what BBQ draws, so the run
/// stays liquid smoke, empty, and the vat says exactly what it is short of.
#[test]
fn a_tank_turned_off_its_run_leaves_the_run_named_and_the_vat_waiting() {
    let mut world = branch();
    assert_eq!(
        run_at(&world, (17, -1)).fluid.as_deref(),
        Some("liquid_smoke")
    );

    world.issue_command(PlayerCommand::Rotate { tile: (16, -1) });

    let smoke = run_at(&world, (17, -1));
    assert_eq!(smoke.naming, PipeNaming::Carries);
    assert_eq!(smoke.fluid.as_deref(), Some("liquid_smoke"));
    assert_eq!(
        stall_at(&world, (17, -3)).map(|(kind, _)| kind),
        Some(StallKind::FluidInput),
        "the BBQ vat is short of a fluid, not stalled on a fault"
    );
}

/// Two tanks declared as two different fluids, discharging into one run: the
/// player has said two things and the resolver will not choose between them.
/// The run reads `ambiguous`, names both, and the vat blames the run.
#[test]
fn two_disagreeing_declarations_on_one_run_leave_it_ambiguous() {
    let world = disagreeing_tanks();
    let smoke = run_at(&world, (17, -1));
    assert_eq!(smoke.naming, PipeNaming::Ambiguous);
    assert!(smoke.naming.is_fault());
    assert_eq!(smoke.fluid, None);
    // Every end of the ambiguity is reported, because which fluids have been
    // confused is the whole of what the player needs to know.
    assert_eq!(smoke.candidates, vec!["liquid_smoke", "vinegar"]);
    // Every tile of the run carries the run's verdict, so a renderer
    // walking a chunk can mark it without rebuilding the networks.
    assert!(
        smoke
            .segments
            .iter()
            .all(|segment| segment.naming == PipeNaming::Ambiguous)
    );
    // The vat's first fluid input, the BBQ mix, is honestly empty in a world
    // that has not run, so that is what it reports first; what it must never
    // do is call the confused run a missing pipe, which is what #54 was.
    assert_ne!(
        stall_at(&world, (17, -3)).map(|(kind, _)| kind),
        Some(StallKind::NoSupplyPipe),
        "the BBQ vat must not report a missing pipe beside a run it is plumbed to"
    );
}

/// The branch with a second tank discharging into the liquid-smoke run from
/// its far end, the original declared liquid smoke and the new one vinegar.
fn disagreeing_tanks() -> World {
    let mut world = branch();
    world.issue_command(PlayerCommand::SetTankFluid {
        tile: (16, -1),
        fluid: "liquid_smoke".to_string(),
    });
    // The run is (17..21, -1); a tank at (21, -1) facing west discharges
    // into its last tile.
    purchase_and_place(&mut world, "fluid_tank", (21, -1), Direction::West);
    world.issue_command(PlayerCommand::SetTankFluid {
        tile: (21, -1),
        fluid: "vinegar".to_string(),
    });
    world
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
    // The mixer on the other end is reported against its own state, which
    // at tick zero is that its solid inputs have not arrived. The fault is
    // on its OUTLET, which a machine that has not started cannot be short
    // of. That is deliberately unchanged — the point of the two new kinds is
    // that a fluid shortfall stops lying, not that it jumps the queue.
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
    let world = disagreeing_tanks();
    let json = serde_json::to_string(&run_at(&world, (17, -1))).expect("a run must serialize");
    assert!(
        json.contains(r#""naming":"ambiguous""#),
        "ambiguous must reach the wire under that name: {json}"
    );
    assert!(json.contains(r#""candidates":["liquid_smoke","vinegar"]"#));

    let mut world = branch();
    purchase_and_place(&mut world, "fluid_tank", (15, -2), Direction::South);
    let json = serde_json::to_string(&run_at(&world, (15, -3))).expect("a run must serialize");
    assert!(
        json.contains(r#""naming":"unnameable""#),
        "unnameable must reach the wire under that name: {json}"
    );
}

// ---------------------------------------------------------------------
// Two recipes on one kind drinking one raw fluid (2026-10-03)
// ---------------------------------------------------------------------

/// A world with money and nothing in it, for the cases below: two mixers
/// beside one tank is not a shape any shipped fixture lays out.
fn funded() -> World {
    let mut world = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    );
    sim_core::sim::reference::open_every_gate(&mut world);
    world
}

fn build(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
    purchase_and_place(world, kind, tile, direction);
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?}"
    );
}

fn set(world: &mut World, tile: (i32, i32), recipe: &str) {
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile,
        recipe: Some(recipe.to_string()),
    });
}

/// Issue #184's case, and the reason the choice moved to the player. A
/// ketchup mixer and a mustard mixer both stand on one tank's vinegar run.
/// While a machine claimed for every recipe of its kind, the two mixers'
/// outlet runs could each be ketchup mix or mustard and the narrowing could
/// not tell which; with each set to its recipe, every run names itself.
#[test]
fn one_vinegar_run_feeds_a_ketchup_mixer_and_a_mustard_mixer() {
    let mut world = funded();
    build(&mut world, "mixer", (5, 0), Direction::East); // (5,0) and (5,1)
    build(&mut world, "mixer", (7, 3), Direction::East); // (7,3) and (7,4)
    build(&mut world, "fluid_tank", (4, 2), Direction::East);
    for x in 5..8 {
        build(&mut world, "pipe", (x, 2), Direction::East); // touches both
    }
    build(&mut world, "pipe", (6, 0), Direction::East); // ketchup mixer's outlet
    build(&mut world, "pipe", (8, 3), Direction::East); // mustard mixer's outlet
    set(&mut world, (5, 0), "ketchup_mix");
    set(&mut world, (7, 3), "mustard");

    let vinegar = run_at(&world, (5, 2));
    assert_eq!(
        vinegar.naming,
        PipeNaming::Carries,
        "one raw drunk by two recipes on one kind: {:?}",
        vinegar.candidates
    );
    assert_eq!(vinegar.fluid.as_deref(), Some("vinegar"));
    assert_eq!(vinegar.taps.len(), 2, "both mixers tap the run");
    assert_eq!(run_at(&world, (6, 0)).fluid.as_deref(), Some("ketchup_mix"));
    assert_eq!(run_at(&world, (8, 3)).fluid.as_deref(), Some("mustard"));
}

/// A machine with no recipe chosen claims nothing for any run it touches: a
/// run between two unset machines is unfinished, not a fault, and setting
/// one of them is what names it.
#[test]
fn a_machine_with_no_recipe_claims_nothing() {
    let mut world = funded();
    build(&mut world, "mixer", (5, 0), Direction::East);
    build(&mut world, "pipe", (6, 0), Direction::East);
    build(&mut world, "cooking_vat", (7, 0), Direction::East);
    let between = run_at(&world, (6, 0));
    assert_eq!(
        between.naming,
        PipeNaming::Unplumbed,
        "{:?}",
        between.candidates
    );
    assert!(!between.naming.is_fault());

    // Setting the mixer names the run, with the vat still unset beside it.
    set(&mut world, (5, 0), "ketchup_mix");
    assert_eq!(run_at(&world, (6, 0)).fluid.as_deref(), Some("ketchup_mix"));
}
