//! Headless integration test for the chain-rework epic's new mechanism:
//! **a machine can put fluid into a pipe**, so a production line can be
//! piped through its middle rather than belted end to end.
//!
//! `sim_core/src/sim/fluid.rs` unit-tests `can_accept_at`/`deposit_at` in
//! isolation and `machine.rs` unit-tests the craft-completion gate against
//! hand-built networks. This file drives the real `World`, because the part
//! that only the whole system can show is **which network a machine deposits
//! into and which it draws from**, and that is decided by geometry
//! (`world.rs`'s module doc):
//!
//! - a fluid output leaves by the face the machine points at, exactly as a
//!   solid one does;
//! - a fluid input comes from an *other* pipe touching the footprint — one
//!   run per input since the BBQ epic, matched by fluid id, which for a
//!   one-fluid recipe like every one in this file is the single run it
//!   touches;
//! - so a machine standing between two runs is unambiguous without the
//!   player labelling either of them, and a machine wrapped in one run reads
//!   as its outlet and gets no inlet at all.
//!
//! It also covers `validate_chain`'s fluid half: two stages joined only by a
//! pipe share no solid id, so the construction-time check that a recipe file
//! describes one production line has to recognise a shared *fluid* id too —
//! and still panic on a typo.

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::Money;
use sim_core::sim::{ContractRegistry, Direction, PlayerCommand, RecipeRegistry, World};

/// A world with money and nothing built.
fn funded(recipes: RecipeRegistry) -> World {
    World::with_content_and_money(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    )
}

fn build(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
    let item = (kind == "hopper").then_some("tomato".to_string());
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item,
        recipe: None,
    });
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

/// A two-stage chain: a crusher makes paste from tomatoes, and a **mixer**
/// turns that paste into a *liquid* — the smallest content that exercises
/// `fluid_outputs` at all. The mixer needs no fluid input, so its only pipe
/// relationship is the one it deposits into.
fn producer_only() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 1)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato_paste", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 3)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse")
}

/// A crusher feeding a mixer that deposits liquid east, and a hopper feeding
/// the crusher — laid out by hand so the pipe run's position is the test's
/// own choice rather than the reference chain's.
///
/// ```text
///   hopper -> belt x2 -> crusher -> belt x2 -> mixer -> pipe x2
/// ```
fn producing_line() -> World {
    let mut world = funded(producer_only());
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 64,
    });
    build(&mut world, "hopper", (-1, 0), Direction::East);
    build(&mut world, "belt", (0, 0), Direction::East);
    build(&mut world, "belt", (1, 0), Direction::East);
    build(&mut world, "crusher", (2, 0), Direction::East);
    build(&mut world, "belt", (3, 0), Direction::East);
    build(&mut world, "belt", (4, 0), Direction::East);
    // The mixer is 1x2 (data/machines.ron): its inlet face is the two tiles
    // at x = 5, and the belt at (4, 0) points into the first of them.
    build(&mut world, "mixer", (5, 0), Direction::East);
    build(&mut world, "pipe", (6, 0), Direction::East);
    build(&mut world, "pipe", (7, 0), Direction::East);
    world
}

fn mix_volume(world: &World) -> f64 {
    world
        .fluid_snapshots()
        .iter()
        .filter(|n| n.fluid.as_deref() == Some("ketchup_mix"))
        .map(|n| n.total_volume)
        .sum()
}

// ---------------------------------------------------------------------
// 1. A machine fills a pipe.
// ---------------------------------------------------------------------

/// The headline claim: a network with no tank anywhere near it fills up,
/// because a machine put liquid in it, and the network takes its fluid id
/// from that machine's recipe.
#[test]
fn a_machine_deposits_its_fluid_output_into_the_run_off_its_outlet_face() {
    let mut world = producing_line();

    let networks = world.fluid_snapshots();
    assert_eq!(networks.len(), 1, "one connected run of pipe");
    assert_eq!(
        networks[0].fluid.as_deref(),
        Some("ketchup_mix"),
        "a run a machine deposits into is named from what that machine can make — the \
         BBQ epic turned this into `resolve_network_fluids`'s narrowing, but a run with \
         exactly one possible fluid still resolves to it"
    );
    assert!(
        networks[0].tank_tiles.is_empty(),
        "nothing but the mixer supplies this run"
    );
    assert_eq!(networks[0].producers.len(), 1);
    assert_eq!(networks[0].producers[0].machine_tile, (5, 0));
    assert_eq!(networks[0].producers[0].units_per_craft, 3);
    assert_eq!(
        networks[0].producers[0].segment_index, 0,
        "the deposit lands on the tile off the outlet face, not somewhere along the run"
    );
    assert!(
        networks[0].taps.is_empty(),
        "nothing draws from it, so `taps` stays empty — producers and consumers are \
         separate lists"
    );
    assert_eq!(networks[0].total_volume, 0.0, "and it starts empty");

    world.issue_command(PlayerCommand::FillHopper {
        amount: 8,
        tile: None,
    });
    for _ in 0..2_000 {
        world.tick();
    }
    assert!(
        mix_volume(&world) > 0.0,
        "liquid must have appeared in a run with no tank behind it — there is no other way \
         for it to get there"
    );
}

/// The producer side of back-pressure, at world level: once the run is
/// brim-full the mixer holds its finished batch instead of spilling it, and
/// the volume in the pipes never exceeds their capacity.
#[test]
fn a_full_run_backs_the_producer_up_rather_than_spilling() {
    let mut world = producing_line();
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 200,
    });

    let capacity = world.fluid_snapshots()[0].total_capacity;
    for _ in 0..20_000 {
        world.issue_command(PlayerCommand::FillHopper {
            amount: 64,
            tile: None,
        });
        world.tick();
        assert!(
            mix_volume(&world) <= capacity,
            "a deposit must never push a network past its capacity"
        );
    }

    // It plateaus just short of brim-full, and that is the all-or-nothing
    // deposit rule visible at world scale rather than a leak: a charge lands
    // whole on the single tile off the outlet face, so once *that tile* has
    // less room than one charge the machine stops — even though the run as a
    // whole still has a little space, which pressure has spread out along
    // it.
    let network = &world.fluid_snapshots()[0];
    let deposit_tile = &network.segments[network.producers[0].segment_index];
    let charge = f64::from(network.producers[0].units_per_craft);
    assert!(
        deposit_tile.capacity - deposit_tile.volume < charge,
        "the run stopped filling, so the deposit tile must have less room than one charge \
         ({} of {}, charge {charge})",
        deposit_tile.volume,
        deposit_tile.capacity
    );
    assert!(
        mix_volume(&world) > capacity - charge * network.segments.len() as f64,
        "...and it is genuinely nearly full, not stalled early"
    );
    // ...and the mixer is holding a completed batch it cannot put anywhere,
    // exactly as it would hold one against a full output buffer.
    let mixer = world
        .chunk_snapshot((0, 0))
        .machines
        .into_iter()
        .find(|m| m.kind == "mixer")
        .expect("the mixer is placed");
    assert!(
        mixer.process.is_some(),
        "a finished batch with nowhere to go is held, not dropped"
    );
}

// ---------------------------------------------------------------------
// 2. Which network is which.
// ---------------------------------------------------------------------

/// A machine standing between two runs draws from one and fills the other,
/// and it is *geometry* that decides which: the outlet face is the deposit
/// side, so the run merely touching the machine is the draw side.
#[test]
fn a_machine_between_two_runs_draws_from_the_one_it_touches_and_fills_the_one_it_faces() {
    // The shipped chain is exactly this shape at the cooking vat, so it is
    // what the fixture uses.
    let world = sim_core::sim::reference_chain(
        RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron")).unwrap(),
    );
    let networks = world.fluid_snapshots();

    let vat_tile = world
        .world_layout()
        .into_iter()
        .find(|p| p.kind == "cooking_vat")
        .expect("the shipped chain has a cooking vat")
        .tile;

    let drawn_from = networks
        .iter()
        .find(|n| n.taps.iter().any(|t| t.machine_tile == vat_tile))
        .expect("the vat draws from a network");
    let filled = networks
        .iter()
        .find(|n| n.producers.iter().any(|p| p.machine_tile == vat_tile))
        .expect("the vat fills a network");

    assert_eq!(drawn_from.fluid.as_deref(), Some("ketchup_mix"));
    assert_eq!(filled.fluid.as_deref(), Some("ketchup"));
    assert_ne!(
        drawn_from.segments[0].tile, filled.segments[0].tile,
        "they must be two genuinely different runs — the topology pass refuses to hand a \
         machine the same network twice"
    );

    // The filled run is the one off the vat's outlet face (east of it); the
    // drawn-from run merely touches its footprint (west of it).
    assert!(
        filled.segments.iter().all(|s| s.tile.0 > vat_tile.0),
        "a fluid output leaves by the face the machine points at"
    );
    assert!(
        drawn_from.segments.iter().all(|s| s.tile.0 < vat_tile.0),
        "and the run it draws from is the one merely touching it"
    );
}

/// The degenerate plumbing the rule has to have an answer for: one run of
/// pipe wrapped around a machine so it touches both the outlet face and a
/// flank. It reads as the **outlet**, and the machine gets no inlet at all —
/// a machine must not draw from the run it fills, which would be a machine
/// feeding itself.
#[test]
fn a_loop_of_pipe_around_one_machine_is_its_outlet_and_gives_it_no_inlet() {
    let recipes = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 1)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato_paste", 1)],
                    fluid_inputs: [("vinegar", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse");
    let mut world = funded(recipes);

    // A 1x2 mixer at (5, 0) covering (5, 0) and (5, 1), with one continuous
    // run of pipe reaching from its outlet face round to its north flank.
    build(&mut world, "mixer", (5, 0), Direction::East);
    for tile in [(6, 0), (6, 1), (6, 2), (5, 2)] {
        build(&mut world, "pipe", tile, Direction::East);
    }

    let networks = world.fluid_snapshots();
    assert_eq!(networks.len(), 1, "it is all one connected component");
    assert_eq!(
        networks[0].fluid.as_deref(),
        Some("ketchup_mix"),
        "the output claim wins: the run is resolved as the outlet first"
    );
    assert_eq!(networks[0].producers.len(), 1);
    assert!(
        networks[0].taps.is_empty(),
        "and the machine gets no inlet from it — a machine never draws from the run it \
         fills"
    );

    // Consequence: the mixer can never run, because its vinegar has nowhere
    // to come from. A stall, not a machine quietly feeding itself.
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 64,
    });
    build(&mut world, "hopper", (3, 0), Direction::East);
    build(&mut world, "belt", (4, 0), Direction::East);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 8,
        tile: None,
    });
    for _ in 0..5_000 {
        world.tick();
    }
    assert_eq!(
        world.fluid_snapshots()[0].total_volume,
        0.0,
        "nothing can have been produced, so the loop must still be empty"
    );
}

// ---------------------------------------------------------------------
// 3. `validate_chain`'s fluid half.
// ---------------------------------------------------------------------

/// Two stages joined only by a pipe share no solid id at all, so the
/// construction-time chain check has to accept a shared **fluid** id as a
/// link. Building the world at all is the assertion.
#[test]
fn two_stages_joined_only_by_a_fluid_are_a_valid_chain() {
    let recipes = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "bottled_ketchup",
                    inputs: [],
                    fluid_inputs: [("ketchup_mix", 1)],
                    outputs: [("bottled_ketchup", 1)],
                    machine: "bottler",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse");
    let world = funded(recipes);
    assert_eq!(
        world.placement_count(),
        0,
        "the world builds, and builds empty"
    );
}

/// ...and a typo'd fluid id is still a construction-time panic, which is the
/// whole reason the check exists: the two stages above with one letter
/// changed describe two unrelated lines, not one.
#[test]
#[should_panic(expected = "chain error")]
fn a_typod_fluid_id_between_two_stages_is_a_construction_time_panic() {
    let recipes = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mixx", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "bottled_ketchup",
                    inputs: [],
                    fluid_inputs: [("ketchup_mix", 1)],
                    outputs: [("bottled_ketchup", 1)],
                    machine: "bottler",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse");
    funded(recipes);
}

/// A recipe may deposit into exactly one network, for the same reason it may
/// draw from exactly one: a machine has one outlet face.
#[test]
#[should_panic(expected = "deposits into exactly one network")]
fn a_recipe_declaring_two_fluid_outputs_is_a_construction_time_panic() {
    let recipes = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 1), ("whey", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse");
    funded(recipes);
}
