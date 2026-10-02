//! **A machine nothing has fed is not a ketchup machine** (owner's request,
//! 2026-10-02).
//!
//! A kind that runs several recipes used to be placed holding the first one
//! `data/recipes.ron` lists for it — ketchup's, at every stage — and to keep
//! holding it until some other sauce's ingredients arrived. So every freshly
//! placed mixer, vat, bottler and packer named ketchup in its hover, its
//! panel and its stall readout, and painted its product mesh ketchup red.
//!
//! Now such a machine is **undecided** — `recipe: null` on every surface —
//! until something it is handed picks one (`Machine::choose`'s rule 4).
//! What it is shown as is Godot's business; this file pins what the wire
//! says.
//!
//! The kinds under test are read off the content (every kind with more than
//! one recipe), never listed, so a fourth sauce is covered without an edit.

mod common;

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::{Money, chunk_of};
use sim_core::sim::world::{StallKind, TileSnapshot};
use sim_core::sim::{
    ContractRegistry, Direction, PlayerCommand, RecipeRegistry, World, reference_chain_with_budget,
};

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
/// recipe on any surface and reports `undecided` — not ketchup.
#[test]
fn a_freshly_placed_machine_names_no_sauce_on_any_surface() {
    let recipes = shipped();
    let kinds = multi_recipe_kinds(&recipes);
    // A kind every one of whose recipes drinks a fluid can only be fed by a
    // pipe, and these stand alone with none.
    let pipe_only: Vec<String> = kinds
        .iter()
        .filter(|kind| {
            recipes
                .for_machine(kind)
                .all(|r| !r.fluid_inputs.is_empty())
        })
        .cloned()
        .collect();
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
        let expected = if pipe_only.contains(kind) {
            StallKind::NoSupplyPipe
        } else {
            StallKind::Undecided
        };
        assert_eq!(
            stall_at(&world, *tile),
            Some((expected, String::new())),
            "and its readout names no sauce — waiting for ingredients, or, for a kind only a \
             pipe can feed, the missing pipe ({kind})"
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

/// One tomato decides a crusher, though a batch wants four: what it has
/// been HANDED picks the recipe, not what it can already start. And from
/// then on it names the raw it is short of, which is the readout this whole
/// change must not cost.
#[test]
fn the_first_ingredient_decides_the_sauce() {
    let mut world = reference_chain_with_budget(shipped(), common::WAGE_RESERVE + 1_000);
    common::buy(&mut world, "tomato", 1);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 1,
        tile: None,
    });
    let crusher = world
        .world_layout()
        .into_iter()
        .find(|p| p.kind == "crusher")
        .expect("the reference chain has a crusher")
        .tile;
    let downstream: Vec<(i32, i32)> = world
        .world_layout()
        .into_iter()
        .filter(|p| p.machine && p.kind != "crusher")
        .map(|p| p.tile)
        .collect();
    for _ in 0..400 {
        world.tick();
        if recipe_everywhere(&world, crusher)[0].is_some() {
            break;
        }
    }
    assert_eq!(
        recipe_everywhere(&world, crusher),
        std::array::from_fn(|_| Some("tomato_paste".to_string())),
        "one tomato on the crusher makes it a tomato crusher"
    );
    assert_eq!(
        stall_at(&world, crusher),
        Some((StallKind::Input, "tomato".to_string())),
        "and it says it wants more tomatoes"
    );
    for tile in downstream {
        assert_eq!(
            recipe_everywhere(&world, tile),
            [None, None, None],
            "nothing has reached the machine at {tile:?}, so it is still undecided"
        );
    }
}

/// The same crusher fed the OTHER sauce's raw picks the other recipe: the
/// decision follows the content, and ketchup is not a default it has to be
/// talked out of.
#[test]
fn another_sauces_ingredient_decides_the_other_sauce() {
    let mut world = funded(shipped());
    // Mustard is gated behind boxes sold; the gate is not the subject.
    sim_core::sim::reference::open_every_gate(&mut world);
    common::buy(&mut world, "mustard_seed", 1);
    place(&mut world, "hopper", (-1, 0), Some("mustard_seed"));
    place(&mut world, "belt", (0, 0), None);
    place(&mut world, "crusher", (1, 0), None);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 1,
        tile: None,
    });
    for _ in 0..400 {
        world.tick();
        if recipe_everywhere(&world, (1, 0))[0].is_some() {
            break;
        }
    }
    assert_eq!(
        recipe_everywhere(&world, (1, 0))[0].as_deref(),
        Some("mustard_mash")
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

/// The review's case: a lone undecided bottler or vat — every recipe of
/// its kind drinks a fluid — has no pipe, so it can NEVER be fed and never
/// decide. "Waiting for ingredients" would be true forever and point at
/// nothing; it says the fix instead, with no sauce named. Plumbed onto a
/// run (even one nothing has filled), it goes back to waiting.
#[test]
fn an_undecided_machine_with_no_pipe_names_the_missing_pipe() {
    let recipes = shipped();
    let pipe_only: Vec<String> = multi_recipe_kinds(&recipes)
        .into_iter()
        .filter(|kind| {
            recipes
                .for_machine(kind)
                .all(|r| !r.fluid_inputs.is_empty())
        })
        .collect();
    assert!(
        pipe_only.iter().any(|k| k == "bottler") && pipe_only.iter().any(|k| k == "cooking_vat"),
        "the shipped bottler and vat are fed by pipe alone: {pipe_only:?}"
    );
    let mut world = funded(recipes);
    place(&mut world, "bottler", (0, 0), None);
    place(&mut world, "cooking_vat", (5, 0), None);
    place(&mut world, "packer", (10, 0), None);
    for _ in 0..50 {
        world.tick();
    }
    for tile in [(0, 0), (5, 0)] {
        assert_eq!(
            stall_at(&world, tile),
            Some((StallKind::NoSupplyPipe, String::new())),
            "the machine at {tile:?} has no pipe and says so"
        );
    }
    assert_eq!(
        stall_at(&world, (10, 0)),
        Some((StallKind::Undecided, String::new())),
        "a packer is belt-fed, so it has no pipe to miss"
    );
    // A pipe laid against the bottler: it can be fed now, so it waits.
    place(&mut world, "pipe", (-1, 0), None);
    world.tick();
    assert_ne!(
        stall_at(&world, (0, 0)).map(|(kind, _)| kind),
        Some(StallKind::NoSupplyPipe),
        "plumbed in, the bottler no longer reports a missing pipe"
    );
}
