//! `World::placement_fluid_preview` — the build ghost's sharing warning
//! (issue #92): before a machine is placed, which pipe runs would it draw
//! from that a placed machine already draws from?
//!
//! Since issue #93 two machines on one run take turns rather than one
//! starving, but turns are still a share. Building the BBQ branch onto a
//! ketchup line that serves the three starter orders halves what that line
//! bottles and loses two of the three contracts within 40 000 ticks (measured
//! on 2026-10-24, the issue's own scenario, on main with #93 in). The player
//! is told before the click, not by a lost contract thousands of ticks on.

use sim_core::sim::{
    Direction, PlayerCommand, RecipeRegistry, World, reference_chain, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("data/recipes.ron must parse")
}

/// The tile the BBQ branch taps the ketchup run from, and the origin of the
/// 1x2 mixer that stands beside it (covering (14, -3) and (14, -2), the
/// latter against the tap) — `sim::bbq_branch`'s joint 1.
const BBQ_TAP: (i32, i32) = (14, -1);
const BBQ_MIXER: (i32, i32) = (14, -3);

fn build(world: &mut World, kind: &str, tile: (i32, i32), recipe: Option<&str>) {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction: Direction::East,
        item: None,
        recipe: recipe.map(str::to_string),
    });
    assert!(
        world.query_tile(tile).is_some(),
        "the fixture's {kind} must go down at {tile:?}"
    );
}

/// Enough to buy the branch's pieces on top of the shipped line, which
/// `reference_chain` alone is funded for exactly.
const BUDGET: u32 = 5_000;

/// The shipped line with the branch's one joint pipe laid, so the next thing
/// a player places is the mixer that would drink from it.
fn line_with_the_tap_laid() -> World {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    build(&mut world, "pipe", BBQ_TAP, None);
    world
}

fn tile_of(world: &World, kind: &str) -> (i32, i32) {
    world
        .world_layout()
        .into_iter()
        .find(|placement| placement.kind == kind)
        .unwrap_or_else(|| panic!("the reference chain places a {kind}"))
        .tile
}

#[test]
fn a_mixer_placed_on_the_ketchup_run_is_told_it_shares_it_with_the_bottler() {
    let world = line_with_the_tap_laid();
    let shared = world.placement_fluid_preview("mixer", BBQ_MIXER, Direction::East);
    assert_eq!(shared.len(), 1, "one run, the ketchup one: {shared:?}");
    assert_eq!(shared[0].pipe, BBQ_TAP, "drawn through the joint pipe");
    assert_eq!(shared[0].fluid.as_deref(), Some("ketchup"));
    assert_eq!(
        shared[0].drawn_by,
        vec!["bottler".to_string()],
        "the shipped bottler is the machine already living on that run"
    );

    // And the click gets exactly the plumbing the warning described: the
    // placed mixer drinks from the run the joint pipe is on.
    let mut world = world;
    build(&mut world, "mixer", BBQ_MIXER, Some("bbq_mix"));
    let run = world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|s| s.tile == BBQ_TAP))
        .expect("the joint pipe is on a run");
    assert_eq!(run.fluid.as_deref(), Some("ketchup"));
    assert!(
        world.placement_fluid_preview("mixer", BBQ_MIXER, Direction::East)[0]
            .drawn_by
            .contains(&"mixer".to_string()),
        "once placed, the mixer is itself one of the machines on that run"
    );
}

#[test]
fn a_run_nobody_draws_from_yet_warns_nothing() {
    // The vat's outlet run before its bottler is placed: the bottler is the
    // run's first consumer, which is the ordinary way a line is built.
    let mut world = reference_chain(recipes());
    let bottler = tile_of(&world, "bottler");
    world.issue_command(PlayerCommand::Remove { tile: bottler });
    assert!(world.query_tile(bottler).is_none(), "the bottler is lifted");
    assert_eq!(
        world.placement_fluid_preview("bottler", bottler, Direction::East),
        vec![],
        "a first consumer shares with nobody"
    );
}

#[test]
fn a_kind_that_draws_no_fluid_or_touches_no_pipe_warns_nothing() {
    let world = line_with_the_tap_laid();
    // The packer is a solid-only kind, beside the same joint pipe.
    assert_eq!(
        world.placement_fluid_preview("packer", BBQ_MIXER, Direction::East),
        vec![]
    );
    // A mixer well away from any pipe.
    assert_eq!(
        world.placement_fluid_preview("mixer", (40, 40), Direction::East),
        vec![]
    );
    // A lone pipe on open ground makes a run no machine draws from. Of
    // `pipe_join_preview`'s guards only fewer-than-two answers it: the
    // ground accepts it, and there is no shared run for `already_shared`.
    assert_eq!(
        world.placement_fluid_preview("pipe", (40, 40), Direction::East),
        vec![],
        "a lone pipe on open ground joins nobody"
    );
}

#[test]
fn a_preview_mutates_nothing() {
    let world = line_with_the_tap_laid();
    let (revision, save) = (world.revision(), world.to_save_ron());
    let _ = world.placement_fluid_preview("mixer", BBQ_MIXER, Direction::East);
    assert_eq!(
        world.revision(),
        revision,
        "a question does not move revision()"
    );
    assert_eq!(
        world.to_save_ron(),
        save,
        "and changes nothing a save would see"
    );
}

/// **The issue's own build order** (`sim::bbq_branch`'s plan): the mixer
/// goes down first, touching no pipe, and then the joint pipe joins it to the
/// run the bottler drinks from. Neither machine is placed by the second click,
/// so it is the PIPE's ghost that has to say what the click does.
#[test]
fn the_pipe_that_joins_a_placed_mixer_to_the_bottler_s_run_is_warned() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    assert_eq!(
        world.placement_fluid_preview("mixer", BBQ_MIXER, Direction::East),
        vec![],
        "the mixer, placed first, touches no pipe yet"
    );
    build(&mut world, "mixer", BBQ_MIXER, Some("bbq_mix"));

    let shared = world.placement_fluid_preview("pipe", BBQ_TAP, Direction::East);
    assert_eq!(
        shared.len(),
        1,
        "the joint makes one shared run: {shared:?}"
    );
    assert_eq!(shared[0].pipe, BBQ_TAP);
    assert_eq!(shared[0].fluid.as_deref(), Some("ketchup"));
    let mut drawn_by = shared[0].drawn_by.clone();
    drawn_by.sort();
    assert_eq!(
        drawn_by,
        vec!["bottler".to_string(), "mixer".to_string()],
        "both machines the joined run would serve"
    );

    // And the click does exactly that: both now draw from one run.
    build(&mut world, "pipe", BBQ_TAP, None);
    let run = world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|s| s.tile == BBQ_TAP))
        .expect("the joint pipe is on a run");
    assert_eq!(run.fluid.as_deref(), Some("ketchup"));
    let layout = world.world_layout();
    let kind_at = |tile: (i32, i32)| {
        layout
            .iter()
            .find(|placement| placement.tile == tile)
            .map(|placement| placement.kind.clone())
            .unwrap_or_default()
    };
    let mut drawers: Vec<String> = run
        .taps
        .iter()
        .map(|tap| kind_at(tap.machine_tile))
        .collect();
    drawers.sort();
    drawers.dedup();
    assert!(
        drawers.contains(&"mixer".to_string()) && drawers.contains(&"bottler".to_string()),
        "the run the warning named now taps both: {drawers:?}"
    );
    assert_eq!(
        world.placement_fluid_preview("pipe", BBQ_TAP, Direction::East),
        vec![],
        "a tile that is already laid is refused as occupied, and warns nothing"
    );
    let shared_now = world.placement_fluid_preview("mixer", (14, -6), Direction::East);
    assert!(
        shared_now.is_empty(),
        "nothing is near (14, -6): {shared_now:?}"
    );
}

#[test]
fn a_pipe_that_extends_a_run_without_adding_a_consumer_warns_nothing() {
    let world = line_with_the_tap_laid();
    // One tile further west of the joint, along the empty ground above the
    // ketchup run: it lengthens the run. It does stand against one machine,
    // the 2x2 cooking vat below it, but that is the machine that FILLS this
    // run, and a machine never draws from the run it fills — so the bottler
    // is still the only one drinking.
    assert_eq!(
        world.placement_fluid_preview("pipe", (13, -1), Direction::East),
        vec![],
        "the bottler alone still drinks from it"
    );
    // And it really does join that run, so the case above is the
    // lengthening it claims rather than a lone pipe.
    let mut world = world;
    build(&mut world, "pipe", (13, -1), None);
    let run = world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|s| s.tile == (13, -1)))
        .expect("the pipe is on a run");
    assert!(
        run.segments.iter().any(|s| s.tile == BBQ_TAP),
        "the new pipe is on the tap's run"
    );
    assert_eq!(run.fluid.as_deref(), Some("ketchup"));
}

/// Once the branch exists the mixer and the bottler already share the
/// ketchup run. Lengthening that run puts nobody new on it, so it must not
/// read as "joins Mixer, Bottler" all over again — `pipe_join_preview`'s
/// `already_shared` is what says so, and this is the test that bites if it
/// goes.
#[test]
fn a_pipe_that_lengthens_an_already_shared_run_warns_nothing() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    build(&mut world, "mixer", BBQ_MIXER, Some("bbq_mix"));
    build(&mut world, "pipe", BBQ_TAP, None);
    assert_eq!(
        world.placement_fluid_preview("pipe", (13, -1), Direction::East),
        vec![],
        "the run already serves the mixer and the bottler; one more tile adds no one"
    );
}

/// A pipe the ground refuses previews nothing, because the click it
/// describes would build nothing — `plan_place_on_ground`'s answer. Asked on
/// the one tile where nothing else would stop the warning: the BBQ joint,
/// which on free ground joins the mixer and the bottler, here occupied by a
/// hopper the player put down first.
#[test]
fn a_pipe_the_ground_refuses_previews_nothing() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    build(&mut world, "mixer", BBQ_MIXER, Some("bbq_mix"));
    assert_eq!(
        world
            .placement_fluid_preview("pipe", BBQ_TAP, Direction::East)
            .len(),
        1,
        "on free ground the joint warns, so the assertion below is the refusal's"
    );
    world.issue_command(PlayerCommand::Purchase {
        kind: "hopper".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "hopper".to_string(),
        tile: BBQ_TAP,
        direction: Direction::East,
        item: Some("spice_mix".to_string()),
        recipe: None,
    });
    assert!(
        world.query_tile(BBQ_TAP).is_some(),
        "the hopper stands on the joint's tile"
    );
    assert_eq!(
        world.placement_fluid_preview("pipe", BBQ_TAP, Direction::East),
        vec![],
        "a pipe on an occupied tile is refused, and warns nothing"
    );
}
