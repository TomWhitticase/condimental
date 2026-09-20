//! Headless integration test for issue #43: **a tank says what it holds, so
//! two raw runs into one machine can be told apart.**
//!
//! `resolve_network_fluids` names a pipe run by arc consistency — a run a
//! tank discharges into can only carry a *raw* fluid, and the recipes of the
//! machines touching it narrow that set further. That is enough for every
//! layout the shipped chain has, because each of its machines draws at most
//! one raw fluid. It runs out on the shape issue #43 reports: one machine
//! drawing **two** raw fluids from **two** tank-fed runs. Both runs seed to
//! the same pair of raws, and the narrowing step has nothing to tell them
//! apart with, because there genuinely is nothing — which drum the player
//! filled with what is a fact about the world that only exists in their
//! head.
//!
//! So both runs stay unnamed (`PipeNaming::Ambiguous`), every machine on
//! them stalls, and — because a tank can only be filled with the fluid its
//! run carries — neither tank can even be loaded. The factory is stuck with
//! no mistake in it and nothing the player can do about it.
//!
//! The fix is to let the player *declare* a tank's fluid, which **seeds**
//! the run rather than merely recording the answer: a declared tank's run
//! is a singleton before any narrowing happens. An undeclared tank behaves
//! exactly as it always did, which is what keeps the shipped chain a chain
//! the player never has to label.
//!
//! **Why the fixture kind has two recipes.** With exactly one feasible
//! recipe, `feasible_naming`'s first-fit input matching already pairs the
//! two runs off — by tile order, which is to say arbitrarily. The machine
//! then runs, but the player still has no say in which of their two tanks
//! is the vinegar one; `the_declaration_decides_which_run_is_which` is that
//! half. Add a second recipe drawing one of the two raws — a mixer that can
//! also make plain syrup, an ordinary content shape — and the first-fit
//! pairing stops being unique, no run narrows to one id, and the layout is
//! stuck exactly as the issue describes. Both halves are the same missing
//! fact, and the same declaration closes both.
//!
//! The fixture is the minimum shape of the bug rather than a whole sauce
//! line. `sugar` stands in as the second raw fluid because
//! `data/ingredients.ron` prices it — what makes an id "a raw fluid" is
//! being drawn by `fluid_inputs` and produced by nothing, and the BBQ
//! branch's real second fluids sit behind a delivery gate a headless test
//! cannot open.

use sim_core::sim::fluid::PipeNaming;
use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::save::SaveData;
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, Direction, FLUID_TANK_CAPACITY, FluidTankSnapshot, PlayerCommand,
    RecipeRegistry, World, command_from_json, reference_chain,
};

// =====================================================================
// The fixture
// =====================================================================

/// A mixer drawing two raw fluids, and a second recipe on the same kind
/// drawing one of them — the whole of issue #43 in a dozen lines. It lives
/// here rather than in `data/recipes.ron` because the shipped file's
/// machines each draw one raw fluid at most; the capability is the
/// simulation's, so it is proven against a fixture.
///
/// `sugar_lump` is a solid on purpose: a second recipe filling the outlet
/// run too would leave *that* run ambiguous between two products, which is
/// the issue's other, smaller case and not what this file is about.
fn brine() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "sweet_brine",
                    inputs: [],
                    fluid_inputs: [("vinegar", 1), ("sugar", 1)],
                    outputs: [],
                    fluid_outputs: [("sweet_brine", 1)],
                    machine: "mixer",
                    time_ticks: 10,
                ),
                Recipe(
                    id: "sugar_lump",
                    inputs: [],
                    fluid_inputs: [("sugar", 1)],
                    outputs: [("sugar_lump", 1)],
                    fluid_outputs: [],
                    machine: "mixer",
                    time_ticks: 10,
                ),
            ],
        )"#,
    )
    .expect("the fixture recipe text must parse")
}

/// The mixer's origin, and the tile every "did it run" assertion is about.
const MIXER: (i32, i32) = (3, 0);
/// The run that should end up carrying vinegar, and the tank that feeds it.
const SOUTH_RUN: (i32, i32) = (3, -1);
const SOUTH_TANK: (i32, i32) = (3, -2);
/// The run that should end up carrying sugar, and its tank.
const NORTH_RUN: (i32, i32) = (3, 2);
const NORTH_TANK: (i32, i32) = (3, 3);
/// The mixer's outlet run — where a finished craft has to show up.
const OUTLET: (i32, i32) = (4, 0);

/// ```text
///            (3,3) tank, facing south
///            (3,2) pipe  <- the north run
///   [mixer] (3,0)-(3,1), facing east, 1x2   ->  (4,0) pipe, the outlet
///            (3,-1) pipe <- the south run
///            (3,-2) tank, facing north
/// ```
///
/// The two feed runs are three tiles apart, so they stay two networks; each
/// touches one tile of the mixer's footprint, and each is discharged into by
/// the single tank that faces it. Nothing else in the layout says anything
/// about which is which — that is the point.
const LAYOUT: &[(&str, (i32, i32), Direction)] = &[
    ("mixer", MIXER, Direction::East),
    ("pipe", SOUTH_RUN, Direction::East),
    ("fluid_tank", SOUTH_TANK, Direction::North),
    ("pipe", NORTH_RUN, Direction::East),
    ("fluid_tank", NORTH_TANK, Direction::South),
    ("pipe", OUTLET, Direction::East),
];

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
        item: None,
    });
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

/// The layout built and stocked, with nothing declared: the exact state
/// issue #43 reports.
fn ambiguous_line() -> World {
    let mut world = funded(brine());
    for item in ["vinegar", "sugar"] {
        world.issue_command(PlayerCommand::BuyIngredient {
            item: item.to_string(),
            quantity: 500,
        });
    }
    for &(kind, tile, direction) in LAYOUT {
        place(&mut world, kind, tile, direction);
    }
    world
}

/// The same line with both tanks declared — the fix applied, and the state
/// the acceptance criterion is about.
fn declared_line() -> World {
    let mut world = ambiguous_line();
    declare(&mut world, SOUTH_TANK, "vinegar");
    declare(&mut world, NORTH_TANK, "sugar");
    world
}

fn declare(world: &mut World, tile: (i32, i32), fluid: &str) {
    world.issue_command(PlayerCommand::SetTankFluid {
        tile,
        fluid: fluid.to_string(),
    });
}

/// How the run covering `tile` came out: its name, if it has one, and why.
fn naming_at(world: &World, tile: (i32, i32)) -> (Option<String>, PipeNaming, Vec<String>) {
    let run = world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|segment| segment.tile == tile))
        .unwrap_or_else(|| panic!("no pipe run covers {tile:?}"));
    (run.fluid, run.naming, run.candidates)
}

fn tank_at(world: &World, tile: (i32, i32)) -> FluidTankSnapshot {
    world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|tank| tank.tile == tile)
        .unwrap_or_else(|| panic!("no tank at {tile:?}"))
}

fn volume_at(world: &World, tile: (i32, i32)) -> f64 {
    world
        .fluid_snapshots()
        .into_iter()
        .find(|run| run.segments.iter().any(|segment| segment.tile == tile))
        .map(|run| run.total_volume)
        .unwrap_or_else(|| panic!("no pipe run covers {tile:?}"))
}

/// Keeps both tanks topped up and runs the line.
fn run_for(world: &mut World, ticks: u32) {
    for _ in 0..ticks {
        world.issue_command(PlayerCommand::FillFluidTank {
            amount: FLUID_TANK_CAPACITY,
            tile: None,
        });
        world.tick();
    }
}

// =====================================================================
// 1. The bug
// =====================================================================

/// Two tank-fed runs into one two-fluid machine, with nothing declared.
/// Both runs narrow to the same pair and the resolver refuses to guess, so
/// the whole thing is inert: no run is named, no tank can be filled (a fill
/// needs to know what it is pouring), and no craft ever starts.
#[test]
fn two_raw_runs_feeding_one_machine_cannot_be_told_apart_by_the_graph() {
    let mut world = ambiguous_line();

    for tile in [SOUTH_RUN, NORTH_RUN] {
        let (fluid, naming, candidates) = naming_at(&world, tile);
        assert_eq!(fluid, None, "{tile:?} must not be named by a guess");
        assert_eq!(
            naming,
            PipeNaming::Ambiguous,
            "{tile:?} is fed by a tank and touches a machine drawing two raw fluids, so it \
             narrows to two candidates, not none"
        );
        assert_eq!(
            candidates,
            vec!["sugar".to_string(), "vinegar".to_string()],
            "and the wire says which two, so the panel can offer them"
        );
    }

    run_for(&mut world, 200);

    for tile in [SOUTH_TANK, NORTH_TANK] {
        assert_eq!(
            tank_at(&world, tile).quantity,
            0.0,
            "a tank on an unnamed run cannot be filled: there is no answer to what with"
        );
    }
    assert_eq!(
        volume_at(&world, OUTLET),
        0.0,
        "and with nothing to draw, the mixer never crafts"
    );
}

// =====================================================================
// 2. The fix — the acceptance case
// =====================================================================

/// The same layout, with each tank told what it holds. Both runs resolve to
/// exactly what was declared, the mixer draws one unit from each, and sweet
/// brine appears in the outlet run — which nothing but a completed craft
/// puts there.
#[test]
fn a_declared_tank_names_its_run_and_the_two_fluid_machine_runs() {
    let mut world = declared_line();

    assert_eq!(
        naming_at(&world, SOUTH_RUN).0.as_deref(),
        Some("vinegar"),
        "the declaration seeds the run, so it names itself before any narrowing"
    );
    assert_eq!(naming_at(&world, SOUTH_RUN).1, PipeNaming::Carries);
    assert_eq!(naming_at(&world, NORTH_RUN).0.as_deref(), Some("sugar"));
    assert_eq!(naming_at(&world, NORTH_RUN).1, PipeNaming::Carries);
    assert_eq!(
        naming_at(&world, OUTLET).0.as_deref(),
        Some("sweet_brine"),
        "the outlet still names itself from the recipe — declaring seeds tank-fed runs only"
    );

    run_for(&mut world, 500);

    for (tile, fluid) in [(SOUTH_TANK, "vinegar"), (NORTH_TANK, "sugar")] {
        let tank = tank_at(&world, tile);
        assert_eq!(tank.fluid.as_deref(), Some(fluid));
        assert!(
            tank.quantity > 0.0,
            "a declared tank can be filled, and this one is being kept topped up"
        );
    }

    assert!(
        volume_at(&world, OUTLET) > 0.0,
        "the two-fluid craft must actually have run: nothing else puts sweet brine in a pipe"
    );

    // ...and it drew from both runs, one tap each, costed against the run
    // carrying the fluid it names.
    let mut taps: Vec<(String, u32)> = world
        .fluid_snapshots()
        .into_iter()
        .filter_map(|run| {
            let tap = run.taps.iter().find(|tap| tap.machine_tile == MIXER)?;
            Some((run.fluid.clone()?, tap.units_per_craft))
        })
        .collect();
    taps.sort();
    assert_eq!(
        taps,
        vec![("sugar".to_string(), 1), ("vinegar".to_string(), 1)],
        "two taps on two distinct tank-fed networks, matched by id"
    );
}

/// Swapping the two declarations over swaps the two runs over: the
/// declaration is the answer, not a hint the resolver is free to overrule.
#[test]
fn the_declaration_decides_which_run_is_which() {
    let mut world = ambiguous_line();
    declare(&mut world, SOUTH_TANK, "sugar");
    declare(&mut world, NORTH_TANK, "vinegar");

    assert_eq!(naming_at(&world, SOUTH_RUN).0.as_deref(), Some("sugar"));
    assert_eq!(naming_at(&world, NORTH_RUN).0.as_deref(), Some("vinegar"));

    run_for(&mut world, 500);
    assert!(
        volume_at(&world, OUTLET) > 0.0,
        "the mixer draws by id, so the mirrored layout runs exactly as well"
    );
}

// =====================================================================
// 3. What a declaration may not do
// =====================================================================

/// An undeclared tank is still named by the graph. The shipped chain has
/// one tank and never needed labelling; issue #43 must not have turned that
/// into a chore.
#[test]
fn an_undeclared_tank_still_takes_its_name_from_the_graph() {
    let recipes = RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("data/recipes.ron must parse");
    let world = reference_chain(recipes);

    let tank = world
        .fluid_tank_snapshots()
        .into_iter()
        .next()
        .expect("the reference chain places a tank");
    assert_eq!(
        tank.declared, None,
        "the fixture builds through the command API and never declares anything"
    );
    assert_eq!(
        tank.fluid.as_deref(),
        Some("vinegar"),
        "and it is still vinegar, derived exactly as it was before #43"
    );
}

/// A tank is how fluid enters the graph from outside, so only a raw fluid
/// may be declared. A recipe's *product* is refused, as is an id no recipe
/// mentions and a tile that is not a tank at all — all of them the same
/// silent no-op every other malformed command is.
#[test]
fn only_a_raw_fluid_may_be_declared() {
    let mut world = declared_line();

    for (tile, fluid) in [
        (SOUTH_TANK, "sweet_brine"),
        (SOUTH_TANK, "ketchup"),
        (MIXER, "sugar"),
        ((99, 99), "sugar"),
    ] {
        declare(&mut world, tile, fluid);
    }

    assert_eq!(
        tank_at(&world, SOUTH_TANK).declared.as_deref(),
        Some("vinegar"),
        "a refused declaration leaves the old one standing"
    );
    assert_eq!(naming_at(&world, SOUTH_RUN).0.as_deref(), Some("vinegar"));
}

/// Changing a tank's mind empties it, exactly as retargeting a hopper
/// empties its bin: what was in there was a different fluid, and a tank
/// holds one.
#[test]
fn switching_a_declared_tank_to_another_fluid_empties_it() {
    let mut world = declared_line();
    run_for(&mut world, 20);
    assert!(tank_at(&world, SOUTH_TANK).quantity > 0.0);

    declare(&mut world, SOUTH_TANK, "sugar");
    assert_eq!(
        tank_at(&world, SOUTH_TANK).quantity,
        0.0,
        "the vinegar that was in it does not become sugar"
    );

    // Re-declaring what it already holds is a no-op, so an idle click on the
    // panel's current selection does not dump the tank.
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: Some(SOUTH_TANK),
    });
    let filled = tank_at(&world, SOUTH_TANK).quantity;
    assert!(filled > 0.0);
    declare(&mut world, SOUTH_TANK, "sugar");
    assert_eq!(tank_at(&world, SOUTH_TANK).quantity, filled);
}

/// Two tanks discharging into one run, declaring different fluids, is
/// ambiguous rather than first-one-wins. A declaration is a fact the
/// resolver is given, so two contradictory facts are still no answer — and
/// the fix is the same as it ever was, two runs rather than one.
#[test]
fn two_contradicting_declarations_on_one_run_are_ambiguous_not_a_guess() {
    let mut world = ambiguous_line();
    declare(&mut world, SOUTH_TANK, "vinegar");
    place(&mut world, "fluid_tank", (2, -1), Direction::East);
    declare(&mut world, (2, -1), "sugar");

    let (fluid, naming, candidates) = naming_at(&world, SOUTH_RUN);
    assert_eq!(fluid, None);
    assert_eq!(naming, PipeNaming::Ambiguous);
    assert_eq!(
        candidates,
        vec!["sugar".to_string(), "vinegar".to_string()],
        "the run reports both declarations, so the panel can show the disagreement"
    );
}

/// The wire form, as Godot's tank panel sends it.
#[test]
fn the_set_tank_fluid_command_parses_from_json() {
    let mut world = ambiguous_line();
    world.issue_command(
        command_from_json(r#"{"type": "set_tank_fluid", "tile": [3, -2], "fluid": "vinegar"}"#)
            .expect("must parse"),
    );
    assert_eq!(
        tank_at(&world, SOUTH_TANK).declared.as_deref(),
        Some("vinegar")
    );
}

// =====================================================================
// 4. The save format
// =====================================================================

/// `SAVE_SCHEMA_VERSION` went to 2 for this field, so a v2 save has to carry
/// it: a declaration is player intent, and a world reloaded without it would
/// come back ambiguous and stuck. The undeclared tank in the same file is
/// half the test — `None` has to survive as `None` rather than being
/// back-filled from the name the graph derived.
#[test]
fn a_save_round_trips_the_declaration_and_leaves_an_undeclared_tank_undeclared() {
    let mut world = ambiguous_line();
    declare(&mut world, SOUTH_TANK, "vinegar");
    run_for(&mut world, 50);
    let text = world.to_save_ron();
    assert!(
        text.contains("fluid: Some(\"vinegar\")"),
        "the declaration has to be *in* the file, not re-derived on load"
    );

    let loaded = World::from_save_with_content(
        SaveData::from_ron(&text).expect("the save must parse"),
        brine(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    )
    .expect("the save must load");

    assert_eq!(
        tank_at(&loaded, SOUTH_TANK).declared.as_deref(),
        Some("vinegar")
    );
    assert_eq!(
        tank_at(&loaded, NORTH_TANK).declared,
        None,
        "the tank the player never declared comes back undeclared"
    );
    assert_eq!(naming_at(&loaded, SOUTH_RUN).0.as_deref(), Some("vinegar"));
    assert_eq!(
        loaded.to_save_ron(),
        text,
        "a loaded world saves the same file again — the round trip loses nothing"
    );
}

/// A save whose tank names a fluid this build's recipes do not draw raw is
/// refused with a message naming the tile and the fluid, rather than loading
/// a world with a tank nobody can fill. Editing `recipes.ron` under a save
/// is how a player reaches this.
#[test]
fn a_save_declaring_a_fluid_this_build_has_no_raw_for_is_refused() {
    let mut world = ambiguous_line();
    declare(&mut world, SOUTH_TANK, "vinegar");
    let text = world.to_save_ron().replace(
        "fluid: Some(\"vinegar\")",
        "fluid: Some(\"brine_concentrate\")",
    );

    let loaded = World::from_save_with_content(
        SaveData::from_ron(&text).expect("the save must parse"),
        brine(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    );
    let Err(error) = loaded else {
        panic!("a tank holding a fluid nothing can supply must not load");
    };
    let message = error.to_string();
    assert!(
        message.contains("brine_concentrate") && message.contains("(3, -2)"),
        "the refusal has to name the fluid and the tile: {message}"
    );
}
