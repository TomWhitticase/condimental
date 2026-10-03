//! Headless integration test for the BBQ epic's first ticket: **a machine
//! kind may run several recipes, and a recipe may draw several fluids.**
//!
//! Both are content capabilities, so both are proven against a test fixture
//! rather than against `data/recipes.ron` — a capability has to hold for
//! content nobody has written yet, and `fluid_production.rs` and `fluids.rs`
//! go on asserting that the ketchup line behaves exactly as it did. The
//! shipped file has since used both capabilities twice over (BBQ's four
//! entries, then mustard's five), and the one test here that does read it is
//! `the_shipped_file_is_two_lines_and_every_recipe_in_it_starts_from_the_market`
//! below. What this file adds is the three things only the whole `World` can
//! show:
//!
//! - a recipe with **two** fluid inputs draws them from two *distinct*
//!   networks, matched **by fluid id** rather than by position — here from
//!   one run the line makes itself and one a tank feeds;
//! - a placed machine runs the recipe the player set on it and nothing else
//!   (2026-10-03), so two machines of one kind on one line run different
//!   recipes side by side, a machine never switches itself to a recipe its
//!   inputs happen to suit, and switching one is destructive in exactly the
//!   ways `PlayerCommand::SetMachineRecipe` says;
//! - and which recipe a machine is set to survives a save taken mid-craft
//!   (`sim::save::SavedMachine::recipe`).
//!
//! It also covers the two construction-time invariants that changed shape
//! here: `validate_chain` reads the recipe file as a connected *graph*
//! rather than as a sequence, and the pipe naming `world.rs` derives is a
//! true fixpoint — the same layout built in a different order names the
//! same runs.
//!
//! **Why the second raw fluid is vinegar and not liquid smoke.** The BBQ
//! chain this fixture is shaped after draws a raw fluid the shipped
//! `data/ingredients.ron` does not price yet, and an unpriced fluid cannot
//! be bought, so a tank of it could never be filled. The mechanism under
//! test is "two inputs, two runs, matched by id", which a second *vinegar*
//! tank exercises exactly as well — and rather better, since two runs
//! carrying different fluids and two runs carrying the same one are then
//! both in the one layout.

mod common;

use std::collections::BTreeSet;

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::save::{SaveData, SavedBody};
use sim_core::sim::types::Money;
use sim_core::sim::world::TileSnapshot;
use sim_core::sim::{ContractRegistry, Direction, PlayerCommand, RecipeRegistry, World};

/// A world with money and nothing built — everything arrives through the
/// real command API, as `sim_core/CLAUDE.md` requires of a fixture.
fn funded(recipes: RecipeRegistry) -> World {
    World::with_content_and_money(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(500_000),
    )
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
        // The hopper is the only kind that takes one, and every fixture
        // here feeds its line the same raw solid.
        item: (kind == "hopper").then(|| "tomato".to_string()),
        recipe: None,
    });
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

/// Sets the machine at `tile` to `recipe`, as its picker does.
fn set(world: &mut World, tile: (i32, i32), recipe: &str) {
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile,
        recipe: Some(recipe.to_string()),
    });
    assert_eq!(
        recipe_of(world, tile),
        recipe,
        "the machine at {tile:?} took the recipe"
    );
}

/// What a machine is currently set to make, read off the same tile snapshot
/// a hover tooltip reads.
fn recipe_of(world: &World, tile: (i32, i32)) -> String {
    match world.query_tile(tile) {
        Some(TileSnapshot::Machine { recipe, .. }) => recipe.unwrap_or_default(),
        other => panic!("{tile:?} is not a machine: {other:?}"),
    }
}

/// `(start_tick, required_ticks)` if a craft is in flight on that machine.
fn process_of(world: &World, tile: (i32, i32)) -> Option<(u64, u64)> {
    match world.query_tile(tile) {
        Some(TileSnapshot::Machine { process, .. }) => process,
        other => panic!("{tile:?} is not a machine: {other:?}"),
    }
}

/// One machine buffer: `(item, count)` slots.
type Buffer = Vec<(String, u32)>;

/// A machine's input and output buffers, as the save records them.
fn buffers(world: &World, tile: (i32, i32)) -> (Buffer, Buffer) {
    world
        .to_save()
        .placements
        .into_iter()
        .find_map(|placement| match placement.body {
            SavedBody::Machine(machine) if placement.tile == tile => {
                Some((machine.input, machine.output))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no machine at {tile:?}"))
}

/// How much fluid the pipe run covering a tile holds.
fn fluid_volume(world: &World, tile: (i32, i32)) -> f64 {
    world
        .fluid_snapshots()
        .into_iter()
        .find(|n| n.segments.iter().any(|s| s.tile == tile))
        .unwrap_or_else(|| panic!("no pipe network covers {tile:?}"))
        .total_volume
}

/// What the pipe run covering a tile turned out to carry, or `None` while it
/// is still ambiguous.
fn fluid_of(world: &World, tile: (i32, i32)) -> Option<String> {
    world
        .fluid_snapshots()
        .into_iter()
        .find(|n| n.segments.iter().any(|s| s.tile == tile))
        .unwrap_or_else(|| panic!("no pipe network covers {tile:?}"))
        .fluid
}

// =====================================================================
// The fixture
// =====================================================================

/// Two recipes on `cooking_vat`, the second of them drawing **two** fluids —
/// the shape `docs/GAMEPLAY.md` gives a second sauce derived from the first,
/// without touching the shipped file that will carry the real one.
fn tangy_branch() -> RecipeRegistry {
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
                    fluid_inputs: [("vinegar", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 2)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "ketchup",
                    inputs: [],
                    fluid_inputs: [("ketchup_mix", 1)],
                    outputs: [],
                    fluid_outputs: [("ketchup", 1)],
                    machine: "cooking_vat",
                    time_ticks: 10,
                ),
                // The two-fluid recipe, and the *second* on `cooking_vat`.
                Recipe(
                    id: "tangy_ketchup",
                    inputs: [],
                    fluid_inputs: [("ketchup", 1), ("vinegar", 1)],
                    outputs: [],
                    fluid_outputs: [("tangy_ketchup", 1)],
                    machine: "cooking_vat",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "bottled_tangy_ketchup",
                    inputs: [],
                    fluid_inputs: [("tangy_ketchup", 1)],
                    outputs: [("bottled_tangy_ketchup", 1)],
                    machine: "bottler",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the fixture recipe text must parse")
}

/// Every placement of the tangy line, west to east:
///
/// ```text
///  (4,2) tank -> (5,2) pipe            (9,2) tank -> (10,2) pipe
///                      |                                   |
///  hopper belt crusher belt belt [mixer] pipe [vat] pipe [vat] pipe
///  (0,0)  (1,0) (2,0)  (3,0)(4,0) (5,0) (6,0)(7,0)(9,0)(10,0)(12,0)
/// ```
///
/// Both vats are the same kind, the same size and the same facing, and each
/// is set to its own recipe ([`MACHINE_RECIPES`]): the first touches one
/// fluid run and cooks ketchup; the second touches two and makes it tangy. Tank rows and
/// outlet pipes sit exactly where `sim::reference`'s layout rule puts them —
/// the run east of a footprint is its outlet, the run on the row north of
/// the whole footprint is a raw feed.
const LAYOUT: &[(&str, (i32, i32), Direction)] = &[
    ("hopper", (0, 0), Direction::East),
    ("belt", (1, 0), Direction::East),
    ("crusher", (2, 0), Direction::East),
    ("belt", (3, 0), Direction::East),
    ("belt", (4, 0), Direction::East),
    ("mixer", (5, 0), Direction::East), // 1x2: (5,0) and (5,1)
    ("pipe", (5, 2), Direction::East),  // touches the mixer's north tile
    ("fluid_tank", (4, 2), Direction::East), // discharges east into it
    ("pipe", (6, 0), Direction::East),  // off the mixer's outlet face
    ("cooking_vat", (7, 0), Direction::East), // 2x2: (7..9, 0..2)
    ("pipe", (9, 0), Direction::East),  // the first vat's outlet
    ("cooking_vat", (10, 0), Direction::East),
    ("pipe", (10, 2), Direction::East), // touches the second vat's north edge
    ("fluid_tank", (9, 2), Direction::East), // discharges east into it
    ("pipe", (12, 0), Direction::East), // the second vat's outlet
];

/// What each machine in [`LAYOUT`] is set to make.
const MACHINE_RECIPES: &[((i32, i32), &str)] = &[
    ((2, 0), "tomato_paste"),
    ((5, 0), "ketchup_mix"),
    ((7, 0), "ketchup"),
    ((10, 0), "tangy_ketchup"),
];

/// The layout built, every machine set to its recipe, stocked and ready to
/// tick.
fn tangy_line() -> World {
    let mut world = funded(tangy_branch());
    stock(&mut world);
    for &(kind, tile, direction) in LAYOUT {
        place(&mut world, kind, tile, direction);
    }
    for &(tile, recipe) in MACHINE_RECIPES {
        set(&mut world, tile, recipe);
    }
    world
}

fn stock(world: &mut World) {
    for (item, quantity) in [("tomato", 500), ("vinegar", 500)] {
        // `common::buy` puts down the warehouse racks the order needs first:
        // `data/storage.ron` caps a holding at `base_cap` plus the bays that
        // exist, and an order with nowhere to go is silently clamped.
        common::buy(world, item, quantity);
    }
}

/// Keeps the hopper and both tanks topped up, so a long run starves on
/// nothing but the line itself.
fn supply(world: &mut World) {
    world.issue_command(PlayerCommand::FillHopper {
        amount: 32,
        tile: None,
    });
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: 32,
        tile: None,
    });
}

// =====================================================================
// 1. Several fluid inputs, matched by id
// =====================================================================

/// The headline claim. The second vat touches two pipe runs carrying two
/// different fluids and draws one unit from **each**, and which run supplies
/// which input is decided by the fluid's id: the vinegar run is north of it
/// and the ketchup run west, and `tangy_ketchup` mentions neither direction.
#[test]
fn a_two_fluid_recipe_draws_from_two_distinct_networks_matched_by_id() {
    let mut world = tangy_line();

    // Every run names itself from the graph, with no player labelling.
    assert_eq!(fluid_of(&world, (5, 2)).as_deref(), Some("vinegar"));
    assert_eq!(fluid_of(&world, (6, 0)).as_deref(), Some("ketchup_mix"));
    assert_eq!(fluid_of(&world, (9, 0)).as_deref(), Some("ketchup"));
    assert_eq!(
        fluid_of(&world, (10, 2)).as_deref(),
        Some("vinegar"),
        "a second, separate run of the same fluid — an id is what matches, not a network"
    );
    assert_eq!(fluid_of(&world, (12, 0)).as_deref(), Some("tangy_ketchup"));

    for _ in 0..2_000 {
        supply(&mut world);
        world.tick();
    }

    // The second vat is tapped into two networks, one per fluid its recipe
    // draws, each reporting the units that recipe takes from *that* run.
    let mut taps: Vec<(String, Option<String>, u32)> = world
        .fluid_snapshots()
        .into_iter()
        .filter_map(|n| {
            let tap = n.taps.iter().find(|t| t.machine_tile == (10, 0))?;
            Some((
                n.fluid.clone()?,
                tap.machine_recipe.clone(),
                tap.units_per_craft,
            ))
        })
        .collect();
    taps.sort();
    assert_eq!(
        taps,
        vec![
            ("ketchup".to_string(), Some("tangy_ketchup".to_string()), 1),
            ("vinegar".to_string(), Some("tangy_ketchup".to_string()), 1),
        ],
        "two taps on two distinct networks: one per fluid input, each costed against the run \
         that carries the fluid it names"
    );

    assert!(
        world
            .fluid_snapshots()
            .iter()
            .any(|n| n.fluid.as_deref() == Some("tangy_ketchup") && n.total_volume > 0.0),
        "the two-fluid craft must actually have run: nothing else puts tangy ketchup in a pipe"
    );
}

// =====================================================================
// 2. Several recipes on one kind
// =====================================================================

/// Two machines of the same kind on one line, each running the recipe it
/// was set to — and neither ever re-picking, however long the line runs.
#[test]
fn two_machines_of_one_kind_run_the_recipes_they_were_set_to() {
    let mut world = tangy_line();
    for _ in 0..2_000 {
        supply(&mut world);
        world.tick();
    }
    assert_eq!(recipe_of(&world, (7, 0)), "ketchup");
    assert_eq!(recipe_of(&world, (10, 0)), "tangy_ketchup");
}

/// A machine with no recipe chosen makes nothing — not even when its inputs
/// satisfy exactly one recipe of its kind. The player decides, never the
/// machine: this is the behaviour that used to be "the first satisfied
/// recipe wins", and its absence is the point of the 2026-10-03 change.
#[test]
fn a_machine_with_no_recipe_chosen_makes_nothing_whatever_it_is_fed() {
    let mut world = tangy_line();
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (2, 0),
        recipe: None,
    });
    assert_eq!(recipe_of(&world, (2, 0)), "");
    for _ in 0..1_000 {
        supply(&mut world);
        world.tick();
    }
    assert_eq!(
        recipe_of(&world, (2, 0)),
        "",
        "it never picked one for itself"
    );
    assert_eq!(
        process_of(&world, (2, 0)),
        None,
        "and it never started a craft"
    );
    match world.query_tile((1, 0)) {
        Some(TileSnapshot::Belt { items_here, .. }) => assert!(
            !items_here.is_empty(),
            "the tomato it will not take backs up on the belt in front of it"
        ),
        other => panic!("expected the feed belt, got {other:?}"),
    }
}

/// A machine takes only what its recipe uses off a belt: a crusher set to
/// one of its kind's recipes leaves the other's raw on the belt rather than
/// filling its buffer with stock it will never use.
#[test]
fn a_machine_runs_only_its_recipe_and_takes_only_that_recipe_s_inputs() {
    fn two_crusher_recipes() -> RecipeRegistry {
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
                        id: "sugar_syrup",
                        inputs: [("sugar", 1)],
                        outputs: [("sugar_syrup", 1)],
                        machine: "crusher",
                        time_ticks: 10,
                    ),
                    Recipe(
                        id: "boxed",
                        inputs: [("tomato_paste", 1), ("sugar_syrup", 1)],
                        outputs: [("boxed", 1)],
                        machine: "packer",
                        time_ticks: 10,
                    ),
                ],
            )"#,
        )
        .expect("the fixture recipe text must parse")
    }

    let run = |item: &str, recipe: &str| {
        let mut world = funded(two_crusher_recipes());
        common::buy(&mut world, item, 100);
        world.issue_command(PlayerCommand::Purchase {
            kind: "hopper".to_string(),
            quantity: 1,
        });
        world.issue_command(PlayerCommand::Place {
            kind: "hopper".to_string(),
            tile: (0, 0),
            direction: Direction::East,
            item: Some(item.to_string()),
            recipe: None,
        });
        place(&mut world, "belt", (1, 0), Direction::East);
        place(&mut world, "crusher", (2, 0), Direction::East);
        set(&mut world, (2, 0), recipe);
        for _ in 0..400 {
            supply(&mut world);
            world.tick();
        }
        world
    };

    let matched = run("sugar", "sugar_syrup");
    let (input, output) = buffers(&matched, (2, 0));
    assert!(
        output.iter().any(|(id, _)| id == "sugar_syrup")
            || input.iter().any(|(id, _)| id == "sugar"),
        "fed what its recipe takes, it takes it and crushes: in {input:?}, out {output:?}"
    );

    let mismatched = run("sugar", "tomato_paste");
    assert_eq!(
        recipe_of(&mismatched, (2, 0)),
        "tomato_paste",
        "it did not switch itself"
    );
    assert_eq!(
        process_of(&mismatched, (2, 0)),
        None,
        "and it started nothing"
    );
    let (input, _) = buffers(&mismatched, (2, 0));
    assert!(input.is_empty(), "the sugar stayed on the belt: {input:?}");
}

/// Switching a machine's recipe is the destructive act the command says it
/// is: the craft in progress is abandoned, an input the new recipe does not
/// take is discarded, and fluid in a run the switch renames is emptied rather
/// than relabelled — the vat switched from ketchup to tangy must not turn the
/// ketchup standing in its outlet run into tangy ketchup nobody made.
#[test]
fn switching_a_recipe_abandons_the_craft_and_empties_a_renamed_run() {
    let mut world = tangy_line();
    let vat = (7, 0);
    for _ in 0..2_000 {
        supply(&mut world);
        world.tick();
        if process_of(&world, vat).is_some() && fluid_volume(&world, (9, 0)) > 0.0 {
            break;
        }
    }
    assert!(
        process_of(&world, vat).is_some(),
        "the first vat must be cooking"
    );
    assert!(
        fluid_volume(&world, (9, 0)) > 0.0,
        "with ketchup in its outlet run"
    );

    // Re-selecting the recipe it already has changes nothing at all.
    let before = world.to_save_ron();
    set(&mut world, vat, "ketchup");
    assert_eq!(world.to_save_ron(), before, "re-selecting is not a switch");

    set(&mut world, vat, "tangy_ketchup");
    assert_eq!(
        process_of(&world, vat),
        None,
        "the ketchup craft was abandoned"
    );
    assert_ne!(
        fluid_of(&world, (9, 0)).as_deref(),
        Some("ketchup"),
        "the outlet run is no longer ketchup's"
    );
    assert_eq!(
        fluid_volume(&world, (9, 0)),
        0.0,
        "and the ketchup in it was destroyed, not relabelled"
    );
}

/// A recipe the machine's kind does not run is refused, and so is a tile
/// that is not a machine — silently, changing nothing.
#[test]
fn a_recipe_that_does_not_run_on_the_kind_is_refused() {
    let mut world = tangy_line();
    let before = world.to_save_ron();
    for (tile, recipe) in [
        ((7, 0), "tomato_paste"), // a crusher's recipe on a vat
        ((7, 0), "no_such_recipe"),
        ((1, 0), "ketchup"),   // a belt
        ((40, 40), "ketchup"), // nothing there
    ] {
        world.issue_command(PlayerCommand::SetMachineRecipe {
            tile,
            recipe: Some(recipe.to_string()),
        });
    }
    assert_eq!(world.to_save_ron(), before);
}

/// Placing a machine with a recipe sets it in one command, and placing one
/// with a recipe its kind does not run is refused outright — not placed
/// with none.
#[test]
fn a_place_can_carry_the_recipe_and_a_wrong_one_refuses_the_place() {
    let mut world = funded(tangy_branch());
    world.issue_command(PlayerCommand::Purchase {
        kind: "cooking_vat".to_string(),
        quantity: 2,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "cooking_vat".to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
        recipe: Some("tangy_ketchup".to_string()),
    });
    assert_eq!(recipe_of(&world, (0, 0)), "tangy_ketchup");
    world.issue_command(PlayerCommand::Place {
        kind: "cooking_vat".to_string(),
        tile: (5, 0),
        direction: Direction::East,
        item: None,
        recipe: Some("tomato_paste".to_string()),
    });
    assert!(
        world.query_tile((5, 0)).is_none(),
        "a crusher's recipe on a vat is refused"
    );
}

// =====================================================================
// 3. The save format
// =====================================================================

/// A save taken **mid-craft** on a multi-recipe kind comes back on the same
/// recipe. Before this epic the recipe was re-derived from the kind on load
/// and there was only ever one to derive; now the pick is live state, and a
/// reload that re-picked it from a half-consumed buffer could finish a craft
/// as something the player never started.
#[test]
fn a_save_mid_craft_restores_the_recipe_that_craft_is() {
    let mut world = tangy_line();
    let mut saved = None;
    for _ in 0..2_000 {
        supply(&mut world);
        world.tick();
        if recipe_of(&world, (10, 0)) == "tangy_ketchup" && process_of(&world, (10, 0)).is_some() {
            saved = Some(world.to_save_ron());
            break;
        }
    }
    let text = saved.expect("the second vat must start a tangy craft within 2000 ticks");
    assert!(
        text.contains("tangy_ketchup"),
        "the recipe id has to be *in* the file, not re-derived on load"
    );

    let loaded = World::from_save_with_content(
        SaveData::from_ron(&text).expect("the save must parse"),
        tangy_branch(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    )
    .expect("the save must load");
    assert_eq!(
        recipe_of(&loaded, (10, 0)),
        "tangy_ketchup",
        "the in-flight craft came back as the craft it was"
    );
    assert_eq!(
        recipe_of(&loaded, (7, 0)),
        "ketchup",
        "and the other vat of the same kind came back on its own recipe"
    );
    assert_eq!(
        loaded.to_save_ron(),
        text,
        "a loaded world saves the same file again — the round trip loses nothing"
    );
}

/// A machine with no recipe chosen saves as `recipe: None` and loads that
/// way: unset is a state the player put it in, not a field to fill in.
#[test]
fn a_machine_with_no_recipe_saves_and_loads_with_none() {
    let mut world = tangy_line();
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (10, 0),
        recipe: None,
    });
    let text = world.to_save_ron();
    let loaded = World::from_save_with_content(
        SaveData::from_ron(&text).expect("the save must parse"),
        tangy_branch(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    )
    .expect("the save must load");
    assert_eq!(recipe_of(&loaded, (10, 0)), "");
    assert_eq!(recipe_of(&loaded, (7, 0)), "ketchup");
    assert_eq!(loaded.to_save_ron(), text);
}

/// A craft in progress with no recipe behind it could never finish, and no
/// build writes one — so a save holding one is refused as corrupt rather
/// than loaded onto a guessed recipe. (Version 7 files, where a machine had
/// picked its own recipe, are rewritten by `scripts/migrate-save-v7-to-v8.py`
/// before they get here.)
#[test]
fn a_save_with_a_craft_and_no_recipe_is_refused() {
    let mut world = tangy_line();
    let vat = (7, 0);
    for _ in 0..2_000 {
        supply(&mut world);
        world.tick();
        if process_of(&world, vat).is_some() {
            break;
        }
    }
    assert!(process_of(&world, vat).is_some(), "the vat never started");
    let mut data = world.to_save();
    for placement in &mut data.placements {
        if placement.tile == vat
            && let SavedBody::Machine(machine) = &mut placement.body
        {
            machine.recipe = None;
        }
    }
    let refused = World::from_save_with_content(
        data,
        tangy_branch(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    );
    assert!(
        matches!(
            refused,
            Err(sim_core::sim::save::SaveError::CraftWithoutRecipe { tile }) if tile == vat
        ),
        "{:?}",
        refused.err()
    );
}

// =====================================================================
// 4. The construction-time invariants that changed shape
// =====================================================================

/// `validate_chain` reads the file as a graph now rather than as a sequence,
/// and it still fails loudly on a recipe joined to nothing — which is what a
/// typo'd id looks like when it *breaks* the line. Here the packer's input
/// is misspelled, so it hangs off the rest of the file on its own and the
/// only raw it could be fed by is an id nothing prices: a fragment nothing
/// can set in motion. The other case, a typo that merely invents a new
/// unpriced raw in a component some *other* recipe anchors, is caught by
/// `IngredientCatalog::validate_covers` instead; see `validate_chain`'s doc
/// for why the two nets are split that way.
#[test]
#[should_panic(expected = "nothing can set in motion")]
fn a_recipe_joined_to_nothing_is_a_construction_time_panic() {
    let orphaned = RecipeRegistry::from_ron_str(
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
                    id: "boxed_ketchup",
                    inputs: [("bottled_ketchpu", 1)],
                    outputs: [("boxed_ketchup", 1)],
                    machine: "packer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the file parses — it is the content that is wrong");
    let _ = funded(orphaned);
}

/// A fork is fine, though, and that is the point of the rewrite: two recipes
/// consuming one product is a branch in a line, not a break in it, and the
/// old consecutive-pairs rule would have rejected the whole BBQ shape.
#[test]
fn a_line_that_forks_is_still_one_line() {
    let _ = funded(tangy_branch());
}

/// **Two independent lines are content, not a fault** (issue #42). This is
/// the shape the old one-connected-component rule would have false-panicked
/// on: a tomato line and a smoke line that share not one id between them,
/// which is exactly what mayonnaise (eggs and oil, GAMEPLAY.md §2 tier 4)
/// will look like beside the tomato line it is landing next to. Both halves
/// have a way in — a machine `data/shop.ron` sells, fed by raws
/// `data/ingredients.ron` prices — so there is nothing here to reject.
#[test]
fn two_independent_production_lines_construct_fine() {
    let two_lines = RecipeRegistry::from_ron_str(
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
                    id: "smoke_rub",
                    inputs: [("spice_mix", 1)],
                    fluid_inputs: [("liquid_smoke", 1)],
                    outputs: [("smoke_rub", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "boxed_smoke_rub",
                    inputs: [("smoke_rub", 1)],
                    outputs: [("boxed_smoke_rub", 1)],
                    machine: "packer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the file parses");
    let mut world = funded(two_lines);
    place(&mut world, "mixer", (0, 0), Direction::East);
    assert!(
        world.query_tile((0, 0)).is_some(),
        "the second line is real content — its mixer buys and places like any other"
    );
}

/// And with mustard, that shape is the shipped content rather than a
/// fixture (issue #42's case stops being hypothetical): `data/recipes.ron`
/// holds a tomato line and a mustard line that share no solid id at all and
/// meet only at the vinegar the market sells them both.
///
/// The reachability half is `validate_chain`'s own fixpoint, run here as an
/// assertion instead of a panic so that a failure names the recipe: every
/// entry in the file must be startable from ids the market prices, working
/// forwards. A new sauce whose input id is a typo shows up as a stage that
/// never becomes reachable.
#[test]
fn the_shipped_file_is_two_lines_and_every_recipe_in_it_starts_from_the_market() {
    let recipes = RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse");
    let consumes = |recipe: &sim_core::sim::Recipe| -> Vec<String> {
        recipe
            .inputs
            .iter()
            .chain(recipe.fluid_inputs.iter())
            .map(|(id, _)| id.clone())
            .collect()
    };
    let produces = |recipe: &sim_core::sim::Recipe| -> Vec<String> {
        recipe
            .outputs
            .iter()
            .chain(recipe.fluid_outputs.iter())
            .map(|(id, _)| id.clone())
            .collect()
    };

    // What the market sells is what no recipe makes — the same definition
    // `World::raw_items` and `data/ingredients.ron` use.
    let made: BTreeSet<String> = recipes.ordered().iter().flat_map(produces).collect();
    let mut obtainable: BTreeSet<String> = recipes
        .ordered()
        .iter()
        .flat_map(consumes)
        .filter(|id| !made.contains(id))
        .collect();
    let mut started: BTreeSet<String> = BTreeSet::new();
    loop {
        let before = started.len();
        for recipe in recipes.ordered() {
            if started.contains(&recipe.id) {
                continue;
            }
            if consumes(recipe).iter().all(|id| obtainable.contains(id)) {
                started.insert(recipe.id.clone());
                obtainable.extend(produces(recipe));
            }
        }
        if started.len() == before {
            break;
        }
    }
    for recipe in recipes.ordered() {
        assert!(
            started.contains(&recipe.id),
            "'{}' can never be started: nothing the market sells reaches it",
            recipe.id
        );
    }

    // The independence claim, as ids rather than as prose. Mustard's four
    // stages eat mustard's own products, mustard's own two raws, and the one
    // fluid the ketchup mixer also drinks — and nothing else. An id from the
    // tomato side appearing here would make this a branch, which is the BBQ
    // tier's job and not this one's.
    let mustard_side: BTreeSet<String> = recipes
        .ordered()
        .iter()
        .filter(|recipe| recipe.id.contains("mustard"))
        .flat_map(consumes)
        .collect();
    let expected: BTreeSet<String> = [
        "mustard_seed",
        "ground_mustard",
        "turmeric",
        "vinegar",
        "mustard",
        "bottled_mustard",
    ]
    .iter()
    .map(|id| (*id).to_string())
    .collect();
    assert_eq!(
        mustard_side, expected,
        "the mustard line is fed from the market and from itself, and shares only vinegar"
    );
}

/// The half of the old rule that survives: a part of the file **nothing can
/// set in motion** is still a construction-time panic. Here the second line
/// is internally coherent and perfectly connected to itself, so the old
/// component flood would not have saved us either — what is wrong with it is
/// that its only raw is an id `data/ingredients.ron` does not price, so no
/// player could ever feed it a single unit and both its machines would stand
/// there forever with no clue why.
#[test]
#[should_panic(expected = "nothing can set in motion")]
fn a_second_line_with_no_way_into_it_is_a_construction_time_panic() {
    let unreachable = RecipeRegistry::from_ron_str(
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
                    id: "yolk_paste",
                    inputs: [("egg_yolk", 1)],
                    outputs: [("yolk_paste", 1)],
                    machine: "crusher",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "boxed_yolk_paste",
                    inputs: [("yolk_paste", 1)],
                    outputs: [("boxed_yolk_paste", 1)],
                    machine: "packer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the file parses — it is the content that is wrong");
    let _ = funded(unreachable);
}

/// The other half: a line whose every recipe runs on a machine the shop does
/// not sell can never be built, however well priced its raws are. `smoker`
/// is in neither `data/machines.ron` nor `data/shop.ron` — the registry
/// check catches the missing footprint first, which is the right order (no
/// footprint is the more basic fault), so this asserts *that* message and
/// stands as the record of where the two checks meet.
#[test]
#[should_panic(expected = "machine content error")]
fn a_line_on_a_machine_nobody_sells_is_a_construction_time_panic() {
    let unsellable = RecipeRegistry::from_ron_str(
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
                    id: "smoked_molasses",
                    inputs: [("molasses", 1)],
                    outputs: [("smoked_molasses", 1)],
                    machine: "smoker",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the file parses");
    let _ = funded(unsellable);
}

/// Two fluid inputs naming the same id stay a content error even now that a
/// recipe may have several: a machine matches each input to a *different*
/// touching run, so one fluid means one input with the units added up.
#[test]
#[should_panic(expected = "draws 'vinegar' twice")]
fn a_recipe_drawing_one_fluid_twice_is_a_construction_time_panic() {
    let doubled = RecipeRegistry::from_ron_str(
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
                    fluid_inputs: [("vinegar", 1), ("vinegar", 2)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the file parses — it is the content that is wrong");
    let _ = funded(doubled);
}

/// The determinism claim under the pipe naming, asserted rather than merely
/// written down: the same layout built in a different order names every run
/// identically. If the naming were "whichever machine reached the run first"
/// — which it was before this ticket — reversing the placement order would
/// change the answer.
#[test]
fn pipe_naming_is_independent_of_placement_order() {
    let named = |reverse: bool| {
        let mut order = LAYOUT.to_vec();
        if reverse {
            order.reverse();
        }
        let mut world = funded(tangy_branch());
        stock(&mut world);
        for (kind, tile, direction) in order {
            place(&mut world, kind, tile, direction);
        }
        let mut choices = MACHINE_RECIPES.to_vec();
        if reverse {
            choices.reverse();
        }
        for (tile, recipe) in choices {
            set(&mut world, tile, recipe);
        }
        let mut runs: Vec<(_, _)> = world
            .fluid_snapshots()
            .into_iter()
            .map(|n| (n.segments[0].tile, n.fluid))
            .collect();
        runs.sort();
        runs
    };

    let forwards = named(false);
    assert_eq!(
        forwards.len(),
        5,
        "five runs: the two vinegar feeds, the mix, the ketchup and the tangy outlet"
    );
    assert!(
        forwards.iter().all(|(_, fluid)| fluid.is_some()),
        "every run in this layout resolves: {forwards:?}"
    );
    assert_eq!(
        forwards,
        named(true),
        "the naming is a property of the graph, not of the order it was built in"
    );
}
