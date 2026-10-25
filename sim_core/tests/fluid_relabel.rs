//! Headless integration test for the fluid-relabel hole `Place`, `Remove`
//! and `Rotate` left open (2026-10-25).
//!
//! A pipe run's NAME is derived from what touches it and recomputed on every
//! topology change; its VOLUME is carried across that rebuild tile by tile
//! (`World::recompute_topology`). `SetTankFluid` and `SetMachineRecipe`
//! both know that the two can come apart and purge the run they rename
//! rather than let thirty units of one fluid answer to another name. The
//! three placement commands rename runs too — pull the only consumer off a
//! vinegar run and it reads `Unnameable`; drop a BBQ vat on it and it reads
//! `liquid_smoke` — and nothing on that path remembered what the standing
//! volume was. Measured on shipped content through the public commands:
//! thirty units of bought vinegar in the run and ninety in the tank became
//! a hundred and twenty units of liquid smoke, a gated fluid the player had
//! never bought (`owned_ingredient("liquid_smoke") == 0` throughout), which
//! GAMEPLAY.md §9's "nothing enters the factory that wasn't paid for"
//! forbids and the BBQ vat would have cooked.
//!
//! The fix gives every pipe tile and every undeclared tank a memory of what
//! its volume IS (`PipeSegment::holds`, `FluidTank::holds`), carried across
//! rebuilds like the volume and kept while the run is unnamed. A run that
//! comes back named something else empties the tiles and tank that
//! remember a different fluid — destroyed, never relabelled, the rule the
//! two commands above already follow — and a run that comes back under the
//! same name loses nothing, so moving a machine one tile and putting it back
//! costs no fluid. The last two tests pin that side: the guard must not turn
//! an ordinary rearrangement into a spill.

mod common;

use sim_core::sim::fluid::PipeNaming;
use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, Direction, FluidTankSnapshot, PlayerCommand, RecipeRegistry, World,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// Shipped content, every gate open (liquid smoke is behind the BBQ gate,
/// and the BBQ vat's recipe is refused while it is shut), and money enough
/// to build and stock a line twice over.
fn funded() -> World {
    let mut world = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(50_000),
    );
    sim_core::sim::reference::open_every_gate(&mut world);
    world
}

fn place(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
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
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

fn set_recipe(world: &mut World, tile: (i32, i32), recipe: &str) {
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile,
        recipe: Some(recipe.to_string()),
    });
}

const TANK: (i32, i32) = (0, 0);
const RUN: [(i32, i32); 3] = [(1, 0), (2, 0), (3, 0)];
const MIXER: (i32, i32) = (4, 0);

/// A tank discharging east down three pipe tiles into a ketchup mixer, the
/// tank filled with bought vinegar and run long enough for the pipes to
/// take some of it. The state every test below starts from.
fn vinegar_line() -> World {
    let mut world = funded();
    place(&mut world, "fluid_tank", TANK, Direction::East);
    for tile in RUN {
        place(&mut world, "pipe", tile, Direction::East);
    }
    place(&mut world, "mixer", MIXER, Direction::East);
    set_recipe(&mut world, MIXER, "ketchup_mix");
    common::buy(&mut world, "vinegar", 200);
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: 120,
        tile: None,
    });
    for _ in 0..300 {
        world.tick();
    }
    let (fluid, naming, volume) = run(&world);
    assert_eq!(fluid.as_deref(), Some("vinegar"));
    assert_eq!(naming, PipeNaming::Carries);
    assert!(
        volume > 0.0,
        "the run must be holding vinegar, or this proves nothing"
    );
    assert!(tank(&world).quantity > 0.0, "and so must the tank");
    world
}

/// The one run in these worlds: its name, why, and what it holds.
fn run(world: &World) -> (Option<String>, PipeNaming, f64) {
    let runs = world.fluid_snapshots();
    assert_eq!(runs.len(), 1, "these layouts have exactly one pipe run");
    (runs[0].fluid.clone(), runs[0].naming, runs[0].total_volume)
}

fn tank(world: &World) -> FluidTankSnapshot {
    world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|t| t.tile == TANK)
        .expect("the tank is placed")
}

/// Vinegar off, liquid smoke on: the volume does not change its name with
/// the run. Nothing is credited anywhere — the vinegar is destroyed, as a
/// declaration change would destroy it — and the only fluid the factory
/// ever bought is still the only fluid it ever owned.
#[test]
fn a_consumer_of_another_fluid_empties_the_run_rather_than_renaming_what_stands_in_it() {
    let mut world = vinegar_line();
    let owned_vinegar = world.owned_ingredient("vinegar");

    world.issue_command(PlayerCommand::Remove { tile: MIXER });
    let (fluid, naming, volume) = run(&world);
    assert_eq!(
        (fluid, naming),
        (None, PipeNaming::Unnameable),
        "with nothing drawing from it the tank-fed run has no name"
    );
    assert!(
        volume > 0.0,
        "but the vinegar standing in it is not lost just because the mixer went"
    );
    assert!(tank(&world).quantity > 0.0);

    // A cooking vat's footprint is 2x2, so placed where the mixer was it
    // touches the run's last tile exactly as the mixer did; on `bbq` it
    // draws liquid smoke, the only raw fluid that recipe names.
    place(&mut world, "cooking_vat", MIXER, Direction::East);
    set_recipe(&mut world, MIXER, "bbq");

    let (fluid, naming, volume) = run(&world);
    assert_eq!(
        (fluid.as_deref(), naming),
        (Some("liquid_smoke"), PipeNaming::Carries),
        "the run is named for what now drinks from it"
    );
    assert_eq!(
        volume, 0.0,
        "and the vinegar that was standing in it did not become liquid smoke"
    );
    let tank = tank(&world);
    assert_eq!(tank.fluid.as_deref(), Some("liquid_smoke"));
    assert_eq!(
        tank.quantity, 0.0,
        "the undeclared tank's vinegar took its name from the run, so it goes the same way"
    );
    assert_eq!(world.owned_ingredient("vinegar"), owned_vinegar);
    assert_eq!(
        world.owned_ingredient("liquid_smoke"),
        0,
        "destroyed, not moved: no liquid smoke was ever bought and none exists now"
    );
}

/// The same hole through the pipe rather than the machine: shortening the
/// run leaves the tank's half unnamed, and a vat laid alongside that half
/// renames it.
#[test]
fn a_run_cut_off_from_its_consumer_is_not_renamed_by_the_next_thing_laid_beside_it() {
    let mut world = vinegar_line();

    world.issue_command(PlayerCommand::Remove { tile: RUN[2] });
    let (fluid, naming, volume) = run(&world);
    assert_eq!((fluid, naming), (None, PipeNaming::Unnameable));
    assert!(volume > 0.0, "the two tiles left keep what they held");

    place(&mut world, "cooking_vat", (2, 1), Direction::East);
    set_recipe(&mut world, (2, 1), "bbq");

    let (fluid, _, volume) = run(&world);
    assert_eq!(fluid.as_deref(), Some("liquid_smoke"));
    assert_eq!(volume, 0.0);
    assert_eq!(tank(&world).quantity, 0.0);
}

/// The guard is about a DIFFERENT fluid. Taking the mixer up and putting it
/// straight back is the commonest thing a player does to a line, and it
/// must cost nothing: the run is vinegar again and every drop is where it
/// was.
#[test]
fn putting_the_same_consumer_back_keeps_every_drop() {
    let mut world = vinegar_line();
    let (_, _, run_before) = run(&world);
    let tank_before = tank(&world).quantity;

    world.issue_command(PlayerCommand::Remove { tile: MIXER });
    place(&mut world, "mixer", MIXER, Direction::East);
    set_recipe(&mut world, MIXER, "ketchup_mix");

    let (fluid, naming, volume) = run(&world);
    assert_eq!(
        (fluid.as_deref(), naming),
        (Some("vinegar"), PipeNaming::Carries)
    );
    assert_eq!(
        volume, run_before,
        "nothing was purged on the way out or back"
    );
    assert_eq!(tank(&world).quantity, tank_before);

    // And a consumer of the SAME fluid is as good as the old one: a mustard
    // mixer drinks vinegar too, so the run keeps its name and its contents.
    world.issue_command(PlayerCommand::Remove { tile: MIXER });
    place(&mut world, "mixer", MIXER, Direction::East);
    set_recipe(&mut world, MIXER, "mustard");
    let (fluid, _, volume) = run(&world);
    assert_eq!(fluid.as_deref(), Some("vinegar"));
    assert_eq!(volume, run_before);
    assert_eq!(tank(&world).quantity, tank_before);
}

/// Re-routing a pipe is not a spill either: cut a run in two and both
/// halves keep what they held, name or no name, and joining them back
/// restores the whole.
#[test]
fn cutting_and_rejoining_a_run_moves_no_fluid() {
    let mut world = vinegar_line();
    let (_, _, before) = run(&world);

    world.issue_command(PlayerCommand::Remove { tile: RUN[1] });
    let total: f64 = world.fluid_snapshots().iter().map(|r| r.total_volume).sum();
    // The removed tile takes its own contents with it (`Remove`'s rule);
    // everything on the other two tiles stands where it was.
    assert!(total > 0.0 && total < before);
    let kept = total;

    place(&mut world, "pipe", RUN[1], Direction::East);
    let (fluid, naming, volume) = run(&world);
    assert_eq!(
        (fluid.as_deref(), naming),
        (Some("vinegar"), PipeNaming::Carries)
    );
    assert_eq!(volume, kept);
}

/// The memory is derived, not saved: a loaded run takes its name as what it
/// holds, so the guard still bites after a round trip through a save.
#[test]
fn the_guard_survives_a_save_and_a_load() {
    let original = vinegar_line();
    let mut world =
        World::from_save_ron(&original.to_save_ron(), recipes()).expect("the save must load");
    assert_eq!(run(&world).2, run(&original).2);

    world.issue_command(PlayerCommand::Remove { tile: MIXER });
    place(&mut world, "cooking_vat", MIXER, Direction::East);
    set_recipe(&mut world, MIXER, "bbq");

    let (fluid, _, volume) = run(&world);
    assert_eq!(fluid.as_deref(), Some("liquid_smoke"));
    assert_eq!(volume, 0.0);
    assert_eq!(tank(&world).quantity, 0.0);
}
