//! **A machine with no recipe chosen names no sauce and makes nothing**
//! (owner's rulings of 2026-10-02 and 2026-10-03).
//!
//! A kind that runs several recipes used to be placed holding the first one
//! `data/recipes.ron` lists for it — ketchup's, at every stage — so every
//! freshly placed mixer, vat, bottler and packer named ketchup on every
//! surface. On 2026-10-02 such a machine became *undecided* until something
//! it was handed picked a recipe; since 2026-10-03 nothing it is handed ever
//! picks one: the player chooses (`PlayerCommand::SetMachineRecipe`, or
//! `Place`'s `recipe`). Until then it is `recipe: null` on every surface,
//! its readout is `undecided`, and it takes nothing off a belt.
//!
//! The kinds under test are read off the content (every kind with more than
//! one recipe), never listed, so a fourth sauce is covered without an edit.

mod common;

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::{Money, chunk_of};
use sim_core::sim::world::{StallKind, TileSnapshot};
use sim_core::sim::{ContractRegistry, Direction, PlayerCommand, RecipeRegistry, World};

fn shipped() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn funded(recipes: RecipeRegistry) -> World {
    World::with_content_and_money(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(500_000),
    )
}

fn place(world: &mut World, kind: &str, tile: (i32, i32), item: Option<&str>) {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction: Direction::East,
        item: item.map(str::to_string),
        recipe: None,
    });
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

/// Every shipped machine kind that has a choice to make — read from the
/// content, so the list grows with it.
fn multi_recipe_kinds(recipes: &RecipeRegistry) -> Vec<String> {
    MachineRegistry::bundled()
        .kinds()
        .into_iter()
        .map(|kind| kind.id.clone())
        .filter(|id| recipes.for_machine(id).count() > 1)
        .collect()
}

/// Every place the wire says what a machine makes, for the machine at
/// `tile`: the layout, the hover's tile snapshot, the chunk snapshot.
fn recipe_everywhere(world: &World, tile: (i32, i32)) -> [Option<String>; 3] {
    let layout = world
        .world_layout()
        .into_iter()
        .find(|p| p.tile == tile)
        .expect("placed");
    assert!(layout.machine, "the layout marks a machine as one");
    let hover = match world.query_tile(tile) {
        Some(TileSnapshot::Machine { recipe, .. }) => recipe,
        other => panic!("{tile:?} is not a machine: {other:?}"),
    };
    let chunk = world
        .chunk_snapshot(chunk_of(tile))
        .machines
        .into_iter()
        .find(|m| m.tile == tile)
        .expect("the machine is in its chunk")
        .recipe;
    [layout.recipe, hover, chunk]
}

fn stall_at(world: &World, tile: (i32, i32)) -> Option<(StallKind, String)> {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|entry| entry.tile == tile)
        .expect("a machine has a load entry")
        .stall
        .map(|stall| (stall.kind, stall.item))
}

/// The headline. Every multi-recipe kind, placed alone and ticked, names no
/// recipe on any surface and reports `undecided` — not ketchup, and not a
/// missing pipe: until a recipe is chosen there is no pipe it is missing.
#[test]
fn a_freshly_placed_machine_names_no_sauce_on_any_surface() {
    let recipes = shipped();
    let kinds = multi_recipe_kinds(&recipes);
    assert!(
        kinds.len() >= 4,
        "the shipped content has a choice at the crusher, mixer, vat, bottler and packer: {kinds:?}"
    );
    let mut world = funded(recipes);
    let tiles: Vec<(String, (i32, i32))> = kinds
        .into_iter()
        .enumerate()
        .map(|(i, kind)| (kind, (i as i32 * 6, 0)))
        .collect();
    for (kind, tile) in &tiles {
        place(&mut world, kind, *tile, None);
    }
    for (kind, tile) in &tiles {
        assert_eq!(
            recipe_everywhere(&world, *tile),
            [None, None, None],
            "a {kind} that has just been placed has not been told its sauce"
        );
    }
    for _ in 0..200 {
        world.tick();
    }
    for (kind, tile) in &tiles {
        assert_eq!(
            recipe_everywhere(&world, *tile),
            [None, None, None],
            "nor has one that has stood idle for 200 ticks — waiting is not a choice ({kind})"
        );
        assert_eq!(
            stall_at(&world, *tile),
            Some((StallKind::Undecided, String::new())),
            "and its readout says it has no recipe, naming no sauce ({kind})"
        );
    }
}

/// The wire spelling of an undecided machine, as Godot reads it.
#[test]
fn an_undecided_machine_is_null_on_the_wire() {
    let mut world = funded(shipped());
    place(&mut world, "mixer", (0, 0), None);
    let layout = serde_json::to_value(world.world_layout()).unwrap();
    assert_eq!(layout[0]["recipe"], serde_json::Value::Null);
    assert_eq!(layout[0]["machine"], true);
    let hover = serde_json::to_value(world.query_tile((0, 0))).unwrap();
    assert_eq!(hover["recipe"], serde_json::Value::Null, "{hover}");
}

/// Feeding a machine with no recipe decides nothing: a crusher with a
/// tomato hopper in front of it stays unset, starts nothing, and leaves the
/// tomato on the belt. (On 2026-10-02 one tomato made it a tomato crusher.)
#[test]
fn feeding_a_machine_with_no_recipe_decides_nothing() {
    let mut world = funded(shipped());
    common::buy(&mut world, "tomato", 8);
    place(&mut world, "hopper", (-1, 0), Some("tomato"));
    place(&mut world, "belt", (0, 0), None);
    place(&mut world, "crusher", (1, 0), None);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 8,
        tile: None,
    });
    for _ in 0..400 {
        world.tick();
    }
    assert_eq!(recipe_everywhere(&world, (1, 0)), [None, None, None]);
    assert_eq!(
        stall_at(&world, (1, 0)),
        Some((StallKind::Undecided, String::new()))
    );
    match world.query_tile((0, 0)) {
        Some(TileSnapshot::Belt { items_here, .. }) => assert!(
            !items_here.is_empty(),
            "the tomato waits on the belt for a crusher that has been told what to make"
        ),
        other => panic!("expected the belt, got {other:?}"),
    }

    // Choosing is what starts it, and from then on it names the raw it is
    // short of — the readout this whole change must not cost.
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (1, 0),
        recipe: Some("tomato_paste".to_string()),
    });
    assert_eq!(
        recipe_everywhere(&world, (1, 0)),
        std::array::from_fn(|_| Some("tomato_paste".to_string()))
    );
    for _ in 0..400 {
        world.tick();
    }
    assert!(
        world
            .production_snapshot()
            .items
            .iter()
            .any(|item| item.id == "tomato_paste" && item.produced_total > 0),
        "set, it crushes the tomatoes it was handed"
    );
}

/// An undecided machine saves as one and loads as one: `recipe: None` is
/// what it writes, and a save without the field reads the same way.
#[test]
fn an_undecided_machine_survives_a_save() {
    let mut world = funded(shipped());
    place(&mut world, "bottler", (0, 0), None);
    for _ in 0..10 {
        world.tick();
    }
    let text = world.to_save_ron();
    let loaded = World::from_save_ron(&text, shipped()).expect("a fresh save loads");
    assert_eq!(recipe_everywhere(&loaded, (0, 0)), [None, None, None]);
    assert_eq!(
        loaded.to_save_ron(),
        text,
        "and the round trip loses nothing"
    );
}

/// A kind with ONE recipe has nothing to decide, so it holds that recipe
/// from the moment it is placed — and keeps naming the raw it waits for.
/// Undecided is about a choice, not about being new.
#[test]
fn a_kind_with_one_recipe_holds_it_from_placement() {
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
            ],
        )"#,
    )
    .expect("the test registry parses");
    let mut world = funded(recipes);
    place(&mut world, "crusher", (0, 0), None);
    let expected = Some("tomato_paste".to_string());
    assert_eq!(
        recipe_everywhere(&world, (0, 0)),
        [expected.clone(), expected.clone(), expected]
    );
    world.tick();
    assert_eq!(
        stall_at(&world, (0, 0)),
        Some((StallKind::Input, "tomato".to_string()))
    );
}

/// Once a pipe-fed machine IS set to a recipe, the pipe-side readouts come
/// back: a lone vat set to cook ketchup has no pipe to draw mix from and
/// says so, and a mixer plumbed onto a sound, empty vinegar run is waiting —
/// not missing a pipe and not a pipe fault.
#[test]
fn a_set_pipe_fed_machine_names_its_missing_pipe_or_its_missing_fluid() {
    let mut world = funded(shipped());
    place(&mut world, "cooking_vat", (10, 0), None);
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (10, 0),
        recipe: Some("ketchup".to_string()),
    });
    world.tick();
    assert_eq!(
        stall_at(&world, (10, 0)).map(|(kind, _)| kind),
        Some(StallKind::NoSupplyPipe),
        "no pipe at all, and its recipe drinks ketchup mix"
    );

    place(&mut world, "mixer", (5, 0), None); // 1x2: (5, 0) and (5, 1)
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (5, 0),
        recipe: Some("ketchup_mix".to_string()),
    });

    place(&mut world, "fluid_tank", (4, 2), None);
    place(&mut world, "pipe", (5, 2), None);
    for _ in 0..20 {
        world.tick();
    }
    let run = world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|s| s.tile == (5, 2)))
        .expect("the pipe at (5, 2) is a run");
    assert_eq!(
        (run.fluid.as_deref(), run.total_volume),
        (Some("vinegar"), 0.0),
        "the fixture's premise: a sound run, named by the mixer's recipe, and empty"
    );
    assert_ne!(
        stall_at(&world, (5, 0)).map(|(kind, _)| kind),
        Some(StallKind::NoSupplyPipe),
        "plumbed, it is waiting, not missing a pipe"
    );
    assert!(
        !matches!(
            stall_at(&world, (5, 0)).map(|(kind, _)| kind),
            Some(StallKind::AmbiguousPipe | StallKind::UnnameablePipe)
        ),
        "and the run is sound"
    );
}
