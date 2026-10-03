//! Headless integration test for the "Real Placement" epic's ticket 1: the
//! world starts **empty**, and everything in it is something the player
//! bought and put down.
//!
//! `sim_core/src/sim/world.rs` is where the mechanism lives and
//! `sim_core/src/sim/machines.rs` unit-tests the rotation geometry in
//! isolation. This file drives the real `World` through the public command
//! API, to prove the things only the whole system can show:
//!
//! 1. **The three new commands work and refuse correctly.** Place spends
//!    inventory, Remove gives it back, Rotate turns in place — and each is a
//!    total no-op when it can't happen (unowned, unaffordable geometry, a
//!    collision, an empty tile).
//! 2. **Flow follows adjacency, not a chain.** A line the player assembled
//!    in a shape the old fixed layout could never produce — one that turns a
//!    corner, one that runs west — carries product, and conserves mass while
//!    doing it.
//! 3. **`Direction::East` reproduces the pre-placement behaviour exactly.**
//!    That is the regression anchor for the whole epic, and
//!    `sim::reference_chain` is where it is checked at full scale (every
//!    other test file in this directory builds its world from it); what is
//!    checked here is the per-rule version of the same claim.
//! 4. **Several hoppers, tanks and docks are a supported thing**, each
//!    working independently, because nothing in the simulation ever assumed
//!    exactly one.
//!
//! Everything below runs with no Godot process involved (ARCHITECTURE.md
//! §7).

mod common;

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, Direction, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, TileSnapshot,
    World, command_from_json,
};

fn shipped_recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A one-stage recipe file: four tomatoes into one paste, on the crusher.
/// The smallest content that can carry mass end to end, so a test about
/// *layout* isn't also a test about the shipped six-stage chain.
fn one_stage() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 4)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 20,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse")
}

/// A world with money to spend, a full larder and nothing built — a new
/// game, except richer, so a test doesn't have to earn its way to the thing
/// it is about.
///
/// The larder is part of the fixture since the chain-rework epic: raw
/// ingredients are bought now (GAMEPLAY.md §5), so a world that has not
/// stocked up cannot fill a hopper or a tank at all, and every fill in this
/// file would silently do nothing.
fn funded(recipes: RecipeRegistry) -> World {
    let mut world = World::with_content_and_money(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    );
    for item in ["tomato", "sugar", "vinegar"] {
        // As full a larder as a world with no warehouse in it can have.
        // Since W1 a holding is capped at `data/storage.ron`'s `base_cap`
        // plus the bays the placed racks supply, and this fixture places
        // nothing — the tests in this file count placements, so a rack
        // bought behind their back would change what they measure. A test
        // that needs a whole tank's or hopper's worth asks for the room
        // itself, with `common::make_room`.
        let quantity = world.stock_room_for(item);
        world.issue_command(PlayerCommand::BuyIngredient {
            item: item.to_string(),
            quantity,
        });
    }
    world
}

/// A registry with one deliberately oblong kind, so rotation's extent-swap
/// is observable. Every shipped kind is 1x1 or square (the cooking vat is
/// 2x2), so a turn changes no shipped machine's shape — the mechanism needs
/// a fixture, exactly as footprint *width* needed one before it.
///
/// The ids are real, priced kinds with fabricated footprints: a machine has
/// to be buyable to be placeable, so an invented kind id could never be put
/// down.
fn oblong_kinds() -> MachineRegistry {
    MachineRegistry::from_ron_str(
        r#"( schema_version: 2, machines: [
            MachineKind(id: "hopper",      footprint: Footprint(width_tiles: 1, height_tiles: 1), staff: []),
            MachineKind(id: "fluid_tank",  footprint: Footprint(width_tiles: 1, height_tiles: 1), staff: []),
            MachineKind(id: "dock",        footprint: Footprint(width_tiles: 1, height_tiles: 1), staff: []),
            MachineKind(id: "crusher",     footprint: Footprint(width_tiles: 3, height_tiles: 1), staff: []),
            MachineKind(id: "packer",      footprint: Footprint(width_tiles: 1, height_tiles: 1), staff: []),
        ] )"#,
    )
    .expect("generated machine text must parse")
}

fn buy(world: &mut World, kind: &str, quantity: u32) {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity,
    });
    assert!(
        world.owned_count(kind) >= quantity,
        "the fixture must be able to afford {quantity} x {kind}"
    );
}

fn place(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
    place_holding(world, kind, tile, direction, None);
}

/// [`place`], but naming what a hopper holds — the field
/// `PlayerCommand::Place` gained once the chain had more than one raw
/// ingredient. `None` is the ordinary case for everything else.
fn place_holding(
    world: &mut World,
    kind: &str,
    tile: (i32, i32),
    direction: Direction,
    item: Option<&str>,
) {
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item: item.map(str::to_string),
        recipe: None,
    });
}

/// Buys and places in one step, asserting it actually landed — for the
/// fixtures below, where a silently-refused placement would show up as a
/// confusing failure much later.
fn build(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
    // A hopper has to be told what it holds once a chain has more than one
    // raw ingredient, and every fixture here feeds tomatoes. Naming it
    // unconditionally is also correct for a single-raw-ingredient chain,
    // where it would have been inferred.
    let item = (kind == "hopper").then_some("tomato");
    buy(world, kind, 1);
    place_holding(world, kind, tile, direction, item);
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

/// Lays `count` belt tiles from `from`, each facing `direction`, stepping
/// that way — a straight run.
fn lay_belt(world: &mut World, from: (i32, i32), direction: Direction, count: i32) {
    let mut tile = from;
    for _ in 0..count {
        build(world, "belt", tile, direction);
        tile = direction.step(tile);
    }
}

// ---------------------------------------------------------------------
// 1. The empty world, and the three new commands.
// ---------------------------------------------------------------------

/// The headline change: `World::new` builds nothing at all. Before this
/// epic it built a complete factory that no purchase had paid for.
#[test]
fn a_new_world_is_empty_and_every_query_agrees() {
    let world = World::new(shipped_recipes());

    assert_eq!(world.placement_count(), 0);
    assert!(world.world_layout().is_empty());
    assert!(world.all_chunks().is_empty());
    assert!(world.hopper_snapshots().is_empty());
    assert!(world.fluid_tank_snapshots().is_empty());
    assert!(world.dock_snapshots().is_empty());
    assert!(world.fluid_snapshots().is_empty());
    assert!(world.query_tile((0, 0)).is_none());
    let chunk = world.chunk_snapshot((0, 0));
    assert!(chunk.belts.is_empty() && chunk.machines.is_empty() && chunk.pipes.is_empty());
}

/// An empty world must also be *safe* to tick — nothing to iterate is a
/// legitimate state, not an edge case that trips an unwrap.
#[test]
fn an_empty_world_ticks_without_doing_anything() {
    let mut world = World::new(shipped_recipes());
    for _ in 0..100 {
        world.tick();
        assert!(world.dirty_chunks().is_empty());
    }
    assert_eq!(world.current_tick(), 100);
    assert!(world.economy_snapshot().delivery_stock.is_empty());
}

/// Place spends inventory (not money — that went at purchase time), Remove
/// gives it back, and the returned unit can be spent again. The two ledgers
/// — what you own and what you have placed — always add up to what you
/// bought.
#[test]
fn place_spends_an_owned_unit_and_remove_returns_it() {
    let mut world = funded(one_stage());
    buy(&mut world, "belt", 2);
    let money_after_buying = world.money();

    place(&mut world, "belt", (0, 0), Direction::East);
    assert_eq!(world.owned_count("belt"), 1, "placing spends exactly one");
    assert_eq!(
        world.money(),
        money_after_buying,
        "placing must not touch money — the money went at purchase time"
    );
    assert_eq!(world.placement_count(), 1);

    world.issue_command(PlayerCommand::Remove { tile: (0, 0) });
    assert_eq!(world.owned_count("belt"), 2, "removing credits one back");
    assert_eq!(world.money(), money_after_buying, "and still not money");
    assert_eq!(world.placement_count(), 0);
    assert!(world.query_tile((0, 0)).is_none());

    // ...and the returned unit is spendable again, which is what makes the
    // round trip a round trip rather than two half-transactions.
    place(&mut world, "belt", (5, 5), Direction::North);
    assert_eq!(world.owned_count("belt"), 1);
    match world.query_tile((5, 5)) {
        Some(TileSnapshot::Belt { direction, .. }) => assert_eq!(direction, Direction::North),
        other => panic!("expected the re-placed belt at (5, 5), got {other:?}"),
    }
}

/// Placing something you do not own is a total no-op — not a free build,
/// and not an error either. This is the rule that makes the shop mean
/// something.
#[test]
fn placing_with_a_zero_owned_count_is_a_no_op() {
    let mut world = funded(one_stage());
    assert_eq!(world.owned_count("crusher"), 0);

    place(&mut world, "crusher", (0, 0), Direction::East);

    assert_eq!(world.placement_count(), 0, "nothing may appear unpaid for");
    assert!(world.query_tile((0, 0)).is_none());
    assert_eq!(
        world.owned_count("crusher"),
        0,
        "and no count went negative"
    );
}

/// An unknown kind, and a kind that is priced but has no recipe to run, are
/// both refusals rather than a machine that sits there doing nothing.
#[test]
fn an_unbuildable_kind_is_refused_even_when_owned() {
    let mut world = funded(one_stage());
    // `mixer` is priced and has a footprint, but this recipe file has no
    // recipe for it — so there is nothing for a placed one to do.
    buy(&mut world, "mixer", 1);
    place(&mut world, "mixer", (0, 0), Direction::East);
    assert_eq!(world.placement_count(), 0);
    assert_eq!(
        world.owned_count("mixer"),
        1,
        "a refused placement must not consume the owned unit either"
    );

    // ...and a kind that does not exist at all.
    world.issue_command(PlayerCommand::Place {
        kind: "fermenter".to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
        recipe: None,
    });
    assert_eq!(world.placement_count(), 0);
}

/// Overlap is checked against the whole rotated rectangle, not just the
/// origin tile — and a refused placement leaves the owned count alone.
#[test]
fn placing_into_an_occupied_tile_is_refused_whole() {
    let mut world = funded(one_stage());
    build(&mut world, "crusher", (4, 0), Direction::East);
    buy(&mut world, "belt", 3);

    for tile in [(4, 0), (4, 0)] {
        place(&mut world, "belt", tile, Direction::East);
    }
    assert_eq!(
        world.owned_count("belt"),
        3,
        "no unit may be spent on a refusal"
    );
    assert_eq!(world.placement_count(), 1);

    // The 2x2 case: a cooking vat covers four tiles, so all four are
    // claimed, not just the one it was placed on.
    let mut world = funded(shipped_recipes());
    build(&mut world, "cooking_vat", (10, 0), Direction::East);
    buy(&mut world, "belt", 4);
    for tile in [(10, 0), (11, 0), (10, 1), (11, 1)] {
        place(&mut world, "belt", tile, Direction::East);
        assert!(
            matches!(world.query_tile(tile), Some(TileSnapshot::Machine { .. })),
            "{tile:?} belongs to the vat and must stay the vat's"
        );
    }
    assert_eq!(world.owned_count("belt"), 4);
}

/// Removing works from **any** tile of a multi-tile footprint, and takes the
/// whole thing — the same "point at the machine, not at its corner" rule
/// `query_tile` follows.
#[test]
fn remove_takes_a_whole_multi_tile_machine_from_any_of_its_tiles() {
    for tile in [(10, 0), (11, 0), (10, 1), (11, 1)] {
        let mut world = funded(shipped_recipes());
        build(&mut world, "cooking_vat", (10, 0), Direction::East);

        world.issue_command(PlayerCommand::Remove { tile });

        assert_eq!(
            world.placement_count(),
            0,
            "removing from {tile:?} must take the whole vat"
        );
        assert_eq!(world.owned_count("cooking_vat"), 1);
        for t in [(10, 0), (11, 0), (10, 1), (11, 1)] {
            assert!(world.query_tile(t).is_none(), "{t:?} must be clear");
        }
    }
}

#[test]
fn removing_an_empty_tile_is_a_no_op() {
    let mut world = funded(one_stage());
    build(&mut world, "belt", (0, 0), Direction::East);

    world.issue_command(PlayerCommand::Remove { tile: (7, 7) });

    assert_eq!(world.placement_count(), 1);
    assert_eq!(
        world.owned_count("belt"),
        0,
        "nothing may be credited for nothing"
    );
}

/// Four rotations is a full turn, and a rotation changes only the facing —
/// never the anchor tile.
#[test]
fn rotate_turns_clockwise_in_place_and_returns_after_four() {
    let mut world = funded(one_stage());
    build(&mut world, "belt", (2, 3), Direction::East);

    for expected in [
        Direction::South,
        Direction::West,
        Direction::North,
        Direction::East,
    ] {
        world.issue_command(PlayerCommand::Rotate { tile: (2, 3) });
        match world.query_tile((2, 3)) {
            Some(TileSnapshot::Belt { direction, .. }) => assert_eq!(direction, expected),
            other => panic!("expected the belt still at (2, 3), got {other:?}"),
        }
        assert_eq!(world.placement_count(), 1, "a rotation places nothing new");
    }
}

/// The case a "rotation is just a mesh transform" reading would get wrong: a
/// 3x1 machine turned 90° becomes 1x3 and claims two tiles it did not have.
/// If someone else is standing on them, the turn is refused — and if nobody
/// is, it succeeds and the machine really does occupy the new tiles.
#[test]
fn rotating_into_a_collision_is_refused_where_the_unrotated_footprint_fits() {
    let mut world = World::with_content_and_money(
        one_stage(),
        oblong_kinds(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    );
    build(&mut world, "crusher", (0, 0), Direction::East);
    // The crusher covers (0,0), (1,0), (2,0). Nothing is north of it yet, so
    // this tile is free — until we put a belt on it.
    build(&mut world, "belt", (0, 1), Direction::East);

    world.issue_command(PlayerCommand::Rotate { tile: (1, 0) });

    match world.query_tile((0, 0)) {
        Some(TileSnapshot::Machine {
            direction,
            footprint,
            ..
        }) => {
            assert_eq!(
                direction,
                Direction::East,
                "the turn must be refused: turning would claim (0, 1), which the belt holds"
            );
            assert_eq!((footprint.width_tiles, footprint.height_tiles), (3, 1));
        }
        other => panic!("expected the crusher unmoved at (0, 0), got {other:?}"),
    }
    assert!(
        matches!(world.query_tile((2, 0)), Some(TileSnapshot::Machine { .. })),
        "and it must still cover its third tile"
    );

    // Clear the obstruction and the very same rotation now succeeds, taking
    // the two tiles it wanted.
    world.issue_command(PlayerCommand::Remove { tile: (0, 1) });
    world.issue_command(PlayerCommand::Rotate { tile: (1, 0) });
    match world.query_tile((0, 0)) {
        Some(TileSnapshot::Machine { direction, .. }) => {
            assert_eq!(direction, Direction::South, "east turns clockwise to south")
        }
        other => panic!("expected the turned crusher at (0, 0), got {other:?}"),
    }
    for tile in [(0, 0), (0, 1), (0, 2)] {
        assert!(
            matches!(world.query_tile(tile), Some(TileSnapshot::Machine { .. })),
            "{tile:?} must now belong to the turned crusher"
        );
    }
    for tile in [(1, 0), (2, 0)] {
        assert!(
            world.query_tile(tile).is_none(),
            "{tile:?} must have been released by the turn"
        );
    }
}

/// Rotating does not spill a belt or restart a craft: the contents have no
/// absolute-direction dependency, so turning one is a pure geometry change.
#[test]
fn rotating_keeps_what_a_belt_and_a_machine_are_holding() {
    let mut world = funded(one_stage());
    build(&mut world, "hopper", (0, 0), Direction::East);
    lay_belt(&mut world, (1, 0), Direction::East, 2);
    build(&mut world, "crusher", (3, 0), Direction::East);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 8,
        tile: None,
    });
    // Long enough for the hopper to have metered several tomatoes onto the
    // first belt, short enough that they are still on it.
    for _ in 0..60 {
        world.tick();
    }

    let belt_items_before = belt_items(&world, (1, 0));
    let crusher_before = machine_state(&world, (3, 0));
    assert!(
        belt_items_before > 0 || crusher_before.1.is_some(),
        "the fixture must have something in flight to be worth checking"
    );

    world.issue_command(PlayerCommand::Rotate { tile: (1, 0) });
    world.issue_command(PlayerCommand::Rotate { tile: (3, 0) });

    assert_eq!(
        belt_items(&world, (1, 0)),
        belt_items_before,
        "a rotated belt must still be carrying what it was carrying"
    );
    assert_eq!(
        machine_state(&world, (3, 0)),
        crusher_before,
        "a rotated machine must keep its buffers and its running process — the batch does \
         not restart because the machine was turned"
    );
}

fn belt_items(world: &World, tile: (i32, i32)) -> usize {
    match world.query_tile(tile) {
        Some(TileSnapshot::Belt { items_here, .. }) => items_here.len(),
        other => panic!("expected a belt at {tile:?}, got {other:?}"),
    }
}

fn machine_state(world: &World, tile: (i32, i32)) -> (Option<String>, Option<(u64, u64)>) {
    match world.query_tile(tile) {
        Some(TileSnapshot::Machine {
            recipe, process, ..
        }) => (recipe, process),
        other => panic!("expected a machine at {tile:?}, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 2. Flow follows adjacency.
// ---------------------------------------------------------------------

/// How many tomatoes' worth of mass the line holds right now: in hoppers,
/// on belts, in machine buffers, and mid-craft. What stands on a dock is
/// [`shipped_tomatoes`]' to count, not this. Everything in this file's
/// one-stage content is either a tomato (1) or a paste (4).
fn tomatoes_in_system(world: &World) -> u32 {
    let mut total = 0;
    for hopper in world.hopper_snapshots() {
        total += hopper.quantity;
    }
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        for belt in &snapshot.belts {
            for (item, _) in &belt.items {
                total += tomato_value(item);
            }
        }
        for machine in &snapshot.machines {
            for (item, qty) in machine.input.iter().chain(machine.output.iter()) {
                total += tomato_value(item) * qty;
            }
            if machine.process.is_some() {
                // Mid-craft: the input is consumed and the output not yet
                // deposited, so the batch is in no visible buffer.
                total += tomato_value("tomato_paste");
            }
        }
    }
    total
}

fn tomato_value(item: &str) -> u32 {
    match item {
        "tomato" => 1,
        "tomato_paste" => 4,
        other => panic!("unexpected item '{other}' in this fixture's world"),
    }
}

/// What the docks hold, in tomatoes. Read off the docks rather than the
/// economy's summed stock, which leaves out a dock short of its clerk.
fn shipped_tomatoes(world: &World) -> u32 {
    world
        .dock_snapshots()
        .iter()
        .flat_map(|dock| dock.contents.iter())
        .map(|(item, qty)| tomato_value(item) * qty)
        .sum()
}

/// A line the pre-placement world could never have produced: it **turns a
/// corner**. Product goes north, then east, through a crusher, and out at a
/// dock — and mass is conserved on every tick along the way.
///
/// This is the test that proves flow is derived from adjacency rather than
/// from `recipes.ron`'s order. Nothing about this layout is describable as
/// "stage i, then stage i + 1 to its east"; the simulation links it by
/// asking, per tile, what is in the tile that tile points at.
#[test]
fn a_player_assembled_line_that_turns_a_corner_carries_product_and_conserves_mass() {
    let mut world = funded(one_stage());

    // Hopper -> two belts east -> two belts north -> one belt east ->
    // crusher -> belt -> dock. The corner is at (3, 0): a belt facing north
    // where its upstream neighbour faces east.
    build(&mut world, "hopper", (0, 0), Direction::East);
    lay_belt(&mut world, (1, 0), Direction::East, 2);
    build(&mut world, "belt", (3, 0), Direction::North);
    build(&mut world, "belt", (3, 1), Direction::North);
    build(&mut world, "belt", (3, 2), Direction::East);
    build(&mut world, "crusher", (4, 2), Direction::East);
    build(&mut world, "belt", (5, 2), Direction::East);
    build(&mut world, "dock", (6, 2), Direction::East);

    let loaded = 8;
    world.issue_command(PlayerCommand::FillHopper {
        amount: loaded,
        tile: None,
    });
    assert_eq!(tomatoes_in_system(&world), loaded);

    let mut shipped = 0;
    for tick_i in 0..20_000u32 {
        world.tick();
        shipped = shipped_tomatoes(&world);
        assert_eq!(
            tomatoes_in_system(&world) + shipped,
            loaded,
            "mass must be conserved through the corner at tick {tick_i}"
        );
        if shipped >= loaded {
            break;
        }
    }

    assert_eq!(
        shipped, loaded,
        "all eight tomatoes must reach the dock as two crates of paste, having gone north \
         and then east to get there"
    );
    assert_eq!(
        world.dock_snapshots()[0].shipped_total,
        2,
        "two pastes were shipped (four tomatoes each)"
    );
}

/// The same line facing the other way. Nothing in the simulation prefers
/// `+X`: a machine's inlet is whichever face is opposite the way it points,
/// so a westward line works identically.
#[test]
fn a_line_running_west_works_exactly_like_one_running_east() {
    let mut world = funded(one_stage());

    build(&mut world, "hopper", (0, 0), Direction::West);
    lay_belt(&mut world, (-1, 0), Direction::West, 3);
    build(&mut world, "crusher", (-4, 0), Direction::West);
    build(&mut world, "belt", (-5, 0), Direction::West);
    // The dock is 2x2, and a footprint's origin is its min-X/min-Y corner
    // whichever way the building faces (`machines.rs`'s module doc), so a
    // WEST-facing dock grows east from the tile named here. Its origin is
    // therefore two tiles past the belt, not one: it claims (-7..-6, 0..1),
    // and the tile the belt hands into, (-6, 0), is on its east edge — the
    // inlet face of something facing west.
    build(&mut world, "dock", (-7, 0), Direction::West);

    world.issue_command(PlayerCommand::FillHopper {
        amount: 4,
        tile: None,
    });
    for _ in 0..20_000u32 {
        world.tick();
        if shipped_tomatoes(&world) == 4 {
            break;
        }
    }

    assert_eq!(shipped_tomatoes(&world), 4);
    assert_eq!(world.dock_snapshots()[0].shipped_total, 1);
}

/// A belt pointing into a machine's **flank** does not feed it — only its
/// inlet face does. This is what makes rotating a machine a decision with a
/// visible consequence rather than a cosmetic one.
#[test]
fn a_belt_pointing_at_the_wrong_face_feeds_nothing() {
    let mut world = World::with_content_and_money(
        one_stage(),
        oblong_kinds(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    );
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 64,
    });
    // A 3x1 crusher facing east at (2, 0): its inlet face is the tile (2, 0)
    // column, i.e. only a belt at (1, 0) pointing east feeds it. This hopper
    // and belt aim at its southern flank instead.
    build(&mut world, "crusher", (2, 0), Direction::East);
    build(&mut world, "hopper", (3, -2), Direction::North);
    build(&mut world, "belt", (3, -1), Direction::North);

    world.issue_command(PlayerCommand::FillHopper {
        amount: 8,
        tile: None,
    });
    for _ in 0..2_000 {
        world.tick();
    }

    match world.query_tile((2, 0)) {
        Some(TileSnapshot::Machine { process, .. }) => assert!(
            process.is_none(),
            "a machine fed at its flank must never start a craft"
        ),
        other => panic!("expected the crusher, got {other:?}"),
    }
    assert!(
        belt_items(&world, (3, -1)) > 0,
        "the tomatoes must have backed up on the belt instead of vanishing into the flank"
    );
}

// ---------------------------------------------------------------------
// 3. East is the regression anchor.
// ---------------------------------------------------------------------

/// Every rule this epic introduced, checked in its east-facing form, which
/// is the form that has to reproduce the pre-placement behaviour exactly:
/// a machine's origin is the tile its belt feeds into, it discharges east,
/// and its footprint grows east and north.
///
/// `sim::reference_chain` is the full-scale version of this claim — it
/// rebuilds the entire pre-placement factory out of east-facing placements,
/// and every other integration test in this directory then asserts the same
/// things about it that it always did.
#[test]
fn facing_east_reproduces_the_pre_placement_conventions() {
    let mut world = funded(shipped_recipes());
    build(&mut world, "cooking_vat", (14, 0), Direction::East);

    // Grows east and north from the origin, exactly as `machines.ron`'s
    // header has always said.
    for tile in [(14, 0), (15, 0), (14, 1), (15, 1)] {
        match world.query_tile(tile) {
            Some(TileSnapshot::Machine {
                origin_tile,
                direction,
                ..
            }) => {
                assert_eq!(origin_tile, (14, 0), "at {tile:?}");
                assert_eq!(direction, Direction::East, "at {tile:?}");
            }
            other => panic!("expected the vat at {tile:?}, got {other:?}"),
        }
    }
    assert!(world.query_tile((13, 0)).is_none());
    assert!(world.query_tile((16, 0)).is_none());

    // The belt west of the origin feeds it; the belt east of its far column
    // takes its output.
    build(&mut world, "belt", (13, 0), Direction::East);
    build(&mut world, "belt", (16, 0), Direction::East);
    build(&mut world, "hopper", (12, 0), Direction::East);

    let layout = world.world_layout();
    let vat = layout
        .iter()
        .find(|p| p.kind == "cooking_vat")
        .expect("the vat is placed");
    assert_eq!(vat.tile, (14, 0));
    assert_eq!(
        vat.occupies,
        vec![(14, 0), (15, 0), (14, 1), (15, 1)],
        "row-major from the min-X/min-Y corner"
    );
    let hopper = &world.hopper_snapshots()[0];
    assert_eq!(
        hopper.belt_tile,
        Some((13, 0)),
        "an east-facing hopper discharges onto the belt to its east"
    );
}

// ---------------------------------------------------------------------
// 4. Several of the buildings that used to be exactly one.
// ---------------------------------------------------------------------

/// Two hoppers feeding two lines into two docks: both fill, both meter,
/// both ship, and each crate stands on the dock its own line feeds — a box
/// arriving at one dock puts nothing on the other.
#[test]
fn two_hoppers_and_two_docks_work_independently() {
    let mut world = funded(one_stage());

    for row in [0, 4] {
        build(&mut world, "hopper", (0, row), Direction::East);
        lay_belt(&mut world, (1, row), Direction::East, 2);
        build(&mut world, "crusher", (3, row), Direction::East);
        build(&mut world, "belt", (4, row), Direction::East);
        build(&mut world, "dock", (5, row), Direction::East);
    }
    assert_eq!(world.hopper_snapshots().len(), 2);
    assert_eq!(world.dock_snapshots().len(), 2);

    // Filling with no tile named tops up *every* hopper — the behaviour a
    // one-hopper world always had on a keypress.
    world.issue_command(PlayerCommand::FillHopper {
        amount: 4,
        tile: None,
    });
    for hopper in world.hopper_snapshots() {
        assert_eq!(hopper.quantity, 4);
    }

    for _ in 0..20_000u32 {
        world.tick();
        if shipped_tomatoes(&world) == 8 {
            break;
        }
    }
    let docks = world.dock_snapshots();
    assert_eq!(
        (docks[0].shipped_total, docks[1].shipped_total),
        (1, 1),
        "each line must have shipped its own crate"
    );
    assert_eq!(
        shipped_tomatoes(&world),
        8,
        "and nothing was lost on the way"
    );
    for dock in &docks {
        assert_eq!(
            dock.contents,
            vec![("tomato_paste".to_string(), 1)],
            "each dock holds exactly its own line's crate: {dock:?}"
        );
    }
}

/// A fill command naming a tile loads only that building — the addressing a
/// build-mode UI needs once there are several.
#[test]
fn a_fill_command_can_name_which_hopper_to_load() {
    let mut world = funded(one_stage());
    build(&mut world, "hopper", (0, 0), Direction::East);
    build(&mut world, "hopper", (0, 4), Direction::East);

    world.issue_command(PlayerCommand::FillHopper {
        amount: 5,
        tile: Some((0, 4)),
    });

    let quantities: Vec<u32> = world
        .hopper_snapshots()
        .iter()
        .map(|hopper| hopper.quantity)
        .collect();
    assert_eq!(
        quantities,
        vec![0, 5],
        "only the named hopper was loaded (snapshots come in tile order, row-major, so the \
         one at (0, 0) is first)"
    );

    // A tile holding something else, or nothing, is a no-op rather than a
    // fallback to "all of them".
    world.issue_command(PlayerCommand::FillHopper {
        amount: 5,
        tile: Some((9, 9)),
    });
    assert_eq!(
        world
            .hopper_snapshots()
            .iter()
            .map(|h| h.quantity)
            .sum::<u32>(),
        5
    );
}

/// Two separate pipe runs are two separate networks, each with its own tank
/// and its own fluid — the thing a single hardcoded run could not express.
#[test]
fn two_disconnected_pipe_runs_are_two_networks() {
    let mut world = funded(shipped_recipes());
    // Run A, tapped by a mixer (whose recipe declares the vinegar input).
    // The mixer is 1x2, so it stands clear of the run and taps it edge-on —
    // fluids connect by touching, not by facing.
    build(&mut world, "fluid_tank", (0, 1), Direction::East);
    build(&mut world, "pipe", (1, 1), Direction::East);
    build(&mut world, "pipe", (2, 1), Direction::East);
    build(&mut world, "mixer", (3, 0), Direction::East);
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (3, 0),
        recipe: Some("ketchup_mix".to_string()),
    });
    // Run B, three tiles away and tapped by nothing.
    build(&mut world, "fluid_tank", (0, 5), Direction::East);
    build(&mut world, "pipe", (1, 5), Direction::East);

    let networks = world.fluid_snapshots();
    assert_eq!(networks.len(), 2, "two disconnected runs, two networks");
    let tapped = networks
        .iter()
        .find(|n| !n.taps.is_empty())
        .expect("one run is tapped by the mixer");
    assert_eq!(tapped.fluid.as_deref(), Some("vinegar"));
    assert_eq!(tapped.segments.len(), 2);
    assert_eq!(tapped.taps[0].machine_tile, (3, 0));
    let untapped = networks
        .iter()
        .find(|n| n.taps.is_empty())
        .expect("the other run is tapped by nobody");
    assert!(
        untapped.fluid.is_none(),
        "a network nothing taps carries nothing in particular — it is pipe with no customer"
    );

    // Filling every tank at once loads only the one whose run has a fluid
    // id — the other is plumbed to pipe nothing taps, so there is nothing to
    // buy for it and the fill is a silent no-op (GAMEPLAY.md §5: a fill
    // draws from bought stock, and stock is per ingredient).
    // A whole tank's worth is over `base_cap`, so this test buys the room
    // for it — `funded` deliberately leaves the world without a warehouse.
    let short_by = 60 - world.owned_ingredient("vinegar");
    common::buy(&mut world, "vinegar", short_by);
    let vinegar_before = world.owned_ingredient("vinegar");
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: 60,
        tile: None,
    });
    assert_eq!(
        vinegar_before - world.owned_ingredient("vinegar"),
        60,
        "exactly one tank's worth was bought and loaded, not two"
    );
    for _ in 0..50 {
        world.tick();
    }
    let tapped = world
        .fluid_snapshots()
        .into_iter()
        .find(|n| !n.taps.is_empty())
        .unwrap();
    assert!(
        tapped.total_volume > 0.0,
        "the tank facing the tapped run must have discharged into it"
    );
    let untapped = world
        .fluid_snapshots()
        .into_iter()
        .find(|n| n.taps.is_empty())
        .unwrap();
    assert_eq!(
        untapped.total_volume, 0.0,
        "and the run nothing taps stays dry, because its tank could not be filled"
    );
}

/// Joining two runs by placing one pipe between them makes them one
/// network, and the fluid already in both is carried across the rebuild
/// rather than spilled.
#[test]
fn placing_a_pipe_between_two_runs_merges_them_without_losing_fluid() {
    let mut world = funded(shipped_recipes());
    build(&mut world, "fluid_tank", (0, 0), Direction::East);
    build(&mut world, "pipe", (1, 0), Direction::East);
    build(&mut world, "pipe", (3, 0), Direction::East);
    // A machine has to tap one of the runs for either to have a *fluid*:
    // a network's id comes from the recipe of whatever taps it, and since
    // ingredients cost money a tank plumbed to an unnamed run cannot be
    // filled at all — there is nothing to charge the player for.
    build(&mut world, "mixer", (1, 1), Direction::East);
    world.issue_command(PlayerCommand::SetMachineRecipe {
        tile: (1, 1),
        recipe: Some("ketchup_mix".to_string()),
    });
    assert_eq!(world.fluid_snapshots().len(), 2, "a gap at (2, 0)");

    world.issue_command(PlayerCommand::FillFluidTank {
        amount: 30,
        tile: None,
    });
    for _ in 0..20 {
        world.tick();
    }
    let before: f64 = world
        .fluid_snapshots()
        .iter()
        .map(|network| network.total_volume)
        .sum();
    assert!(before > 0.0, "the fixture must have fluid in the pipes");

    build(&mut world, "pipe", (2, 0), Direction::East);

    let after = world.fluid_snapshots();
    assert_eq!(
        after.len(),
        1,
        "the gap is closed, so it is one network now"
    );
    assert_eq!(after[0].segments.len(), 3);
    assert_eq!(
        after[0].total_volume, before,
        "volume is carried across a topology rebuild by tile — a recompute must not spill \
         the run"
    );
}

// ---------------------------------------------------------------------
// 5. The wire format ticket 2 issues.
// ---------------------------------------------------------------------

#[test]
fn the_three_new_commands_parse_from_json() {
    let mut world = funded(one_stage());
    buy(&mut world, "belt", 2);

    world.issue_command(
        command_from_json(
            r#"{"type": "place", "kind": "belt", "tile": [4, 2], "direction": "north"}"#,
        )
        .expect("place must parse"),
    );
    match world.query_tile((4, 2)) {
        Some(TileSnapshot::Belt { direction, .. }) => assert_eq!(direction, Direction::North),
        other => panic!("expected the placed belt, got {other:?}"),
    }

    // `direction` is optional and defaults to east — the facing that
    // reproduces every pre-placement behaviour.
    world.issue_command(
        command_from_json(r#"{"type": "place", "kind": "belt", "tile": [6, 2]}"#)
            .expect("place must parse without a direction"),
    );
    match world.query_tile((6, 2)) {
        Some(TileSnapshot::Belt { direction, .. }) => assert_eq!(direction, Direction::East),
        other => panic!("expected the placed belt, got {other:?}"),
    }

    world.issue_command(
        command_from_json(r#"{"type": "rotate", "tile": [6, 2]}"#).expect("rotate must parse"),
    );
    match world.query_tile((6, 2)) {
        Some(TileSnapshot::Belt { direction, .. }) => assert_eq!(direction, Direction::South),
        other => panic!("expected the rotated belt, got {other:?}"),
    }

    world.issue_command(
        command_from_json(r#"{"type": "remove", "tile": [6, 2]}"#).expect("remove must parse"),
    );
    assert!(world.query_tile((6, 2)).is_none());
    assert_eq!(world.owned_count("belt"), 1);

    // `tile` is required — there is no sensible default for *where*.
    assert!(command_from_json(r#"{"type": "place", "kind": "belt"}"#).is_none());
    assert!(command_from_json(r#"{"type": "remove"}"#).is_none());
    // ...and an unknown direction is a malformed command, not a default.
    assert!(
        command_from_json(
            r#"{"type": "place", "kind": "belt", "tile": [0, 0], "direction": "up"}"#
        )
        .is_none()
    );
}

/// A `place`/`remove`/`rotate` dirties every chunk it touched, so a
/// chunk-driven renderer redraws without polling the whole world.
#[test]
fn placement_commands_dirty_the_chunks_they_touch() {
    let mut world = funded(shipped_recipes());
    buy(&mut world, "cooking_vat", 1);
    world.tick(); // clear
    assert!(world.dirty_chunks().is_empty());

    // A footprint straddling the x = 32 boundary must dirty both chunks.
    place(&mut world, "cooking_vat", (31, 0), Direction::East);
    let dirty = world.dirty_chunks().to_vec();
    assert!(
        dirty.contains(&(0, 0)) && dirty.contains(&(1, 0)),
        "{dirty:?}"
    );

    world.tick();
    world.issue_command(PlayerCommand::Remove { tile: (32, 1) });
    let dirty = world.dirty_chunks().to_vec();
    assert!(
        dirty.contains(&(0, 0)) && dirty.contains(&(1, 0)),
        "{dirty:?}"
    );
}

/// A hopper placed on a world whose recipes have a raw ingredient holds that
/// ingredient — the same "from `recipes.ron`, never from Rust" rule the
/// pre-placement hopper had, now applied at placement time.
#[test]
fn a_placed_hopper_holds_the_chains_raw_ingredient() {
    let mut world = funded(shipped_recipes());
    build(&mut world, "hopper", (0, 0), Direction::East);
    let hopper = &world.hopper_snapshots()[0];
    assert_eq!(hopper.item, "tomato");
    assert_eq!(hopper.capacity, HOPPER_CAPACITY);
    assert_eq!(hopper.quantity, 0, "a newly placed hopper starts empty");
}

/// Determinism (ARCHITECTURE.md §3.9): the same commands in the same order
/// produce the same world, tick for tick.
///
/// This matters more since placement than it did before, because tick order
/// is no longer a fixed chain — it is derived from where things happen to
/// stand. Every list the topology pass builds is therefore in tile order,
/// and nothing in a tick iterates a `HashMap`; this is the test that would
/// notice if that stopped being true.
#[test]
fn the_same_commands_produce_the_same_world_twice() {
    fn build_and_run() -> (String, String) {
        let mut world = funded(one_stage());
        build(&mut world, "hopper", (0, 0), Direction::East);
        lay_belt(&mut world, (1, 0), Direction::East, 2);
        build(&mut world, "belt", (3, 0), Direction::North);
        build(&mut world, "belt", (3, 1), Direction::East);
        build(&mut world, "crusher", (4, 1), Direction::East);
        build(&mut world, "belt", (5, 1), Direction::East);
        build(&mut world, "dock", (6, 1), Direction::East);
        world.issue_command(PlayerCommand::FillHopper {
            amount: 12,
            tile: None,
        });
        for _ in 0..3_000 {
            world.tick();
        }
        let chunks: Vec<_> = world
            .all_chunks()
            .into_iter()
            .map(|chunk| serde_json::to_string(&world.chunk_snapshot(chunk)).unwrap())
            .collect();
        (
            chunks.join("|"),
            serde_json::to_string(&world.economy_snapshot()).unwrap(),
        )
    }

    assert_eq!(
        build_and_run(),
        build_and_run(),
        "two identical command sequences must produce byte-identical snapshots"
    );
}
