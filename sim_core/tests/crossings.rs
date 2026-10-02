//! Headless integration test for the belt/pipe crossing epic, ticket T1
//! (`docs/BELT_PIPE_CROSSING_PLAN.md`): a pipe placed onto a belt, or a belt
//! onto a pipe, at right angles, becomes ONE `crossing` entity carrying both
//! halves, so the grid stays one occupant per tile.
//!
//! Driven through the public command API only, like every file here. What
//! it pins:
//!
//! 1. **Upgrading a live line destroys nothing.** An item standing on the
//!    belt is still there, at the same spot, once a pipe is laid across it;
//!    fluid in a pipe is still there once a belt is laid across it.
//! 2. **The belt half is a belt tile.** A line with crossings in it moves
//!    product tick for tick exactly as the same line of plain belts does.
//! 3. **The open-sides mask.** A crossing's pipe joins only along its own
//!    axis, so two perpendicular runs meeting at one stay two networks —
//!    the one rule in the feature a screenshot cannot prove.
//! 4. **It is assembled, never bought.** `Remove` hands back one belt and
//!    one pipe; `Rotate` turns the whole assembly and four of them are the
//!    identity; parallel pieces and every other pairing are refused.
//! 5. **It saves.** A world with crossings round-trips through save/load,
//!    and the reloaded world keeps running in lockstep with the original.

mod common;

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::{Money, TileCoord};
use sim_core::sim::{
    BELT_KIND, CROSSING_KIND, ContractRegistry, Direction, FLUID_TANK_CAPACITY, PIPE_KIND,
    PlayerCommand, RecipeRegistry, SPLITTER_KIND, TileSnapshot, World,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A world with money to spend and nothing built.
fn funded() -> World {
    World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(50_000),
    )
}

/// Buys one `kind` and tries to place it; returns whether the world took
/// the placement (by whether the owned unit was spent).
fn try_build(world: &mut World, kind: &str, tile: TileCoord, direction: Direction) -> bool {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    let owned = world.owned_count(kind);
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item: (kind == "hopper").then(|| "tomato".to_string()),
    });
    world.owned_count(kind) < owned
}

fn build(world: &mut World, kind: &str, tile: TileCoord, direction: Direction) {
    assert!(
        try_build(world, kind, tile, direction),
        "the fixture must be able to place {kind} at {tile:?} facing {direction:?}"
    );
}

/// A belt facing `belt`, then a pipe laid across it — the player's way of
/// building a crossing.
fn build_crossing(world: &mut World, tile: TileCoord, belt: Direction) {
    build(world, BELT_KIND, tile, belt);
    build(world, PIPE_KIND, tile, belt.rotate_cw());
    assert_eq!(kind_at(world, tile).as_deref(), Some(CROSSING_KIND));
}

/// The `world_layout` kind of whatever stands on `tile`.
fn kind_at(world: &World, tile: TileCoord) -> Option<String> {
    world
        .world_layout()
        .into_iter()
        .find(|p| p.occupies.contains(&tile))
        .map(|p| p.kind)
}

fn items_here(world: &World, tile: TileCoord) -> Vec<String> {
    match world.query_tile(tile) {
        Some(TileSnapshot::Belt { items_here, .. })
        | Some(TileSnapshot::Crossing { items_here, .. }) => items_here,
        other => panic!("expected a belt or crossing at {tile:?}, found {other:?}"),
    }
}

/// Every belt-like tile's exact contents — tile, direction, items with
/// sub-tile positions, surface clock — from the full chunk read, as JSON so
/// two worlds compare field for field.
fn belt_state(world: &World) -> Vec<serde_json::Value> {
    world
        .all_chunks()
        .into_iter()
        .flat_map(|chunk| world.chunk_snapshot(chunk).belts)
        .map(|belt| serde_json::to_value(belt).unwrap())
        .collect()
}

/// The fluid network (by index) holding the pipe tile at `tile`.
fn network_of(world: &World, tile: TileCoord) -> Option<usize> {
    world
        .fluid_snapshots()
        .iter()
        .position(|n| n.segments.iter().any(|s| s.tile == tile))
}

/// A hopper at `(-1, 0)` metering `tomatoes` onto whatever stands at
/// `(0, 0)`.
fn feed_tomatoes(world: &mut World, tomatoes: u32) {
    build(world, "hopper", (-1, 0), Direction::East);
    common::buy(world, "tomato", tomatoes);
    world.issue_command(PlayerCommand::FillHopper {
        amount: tomatoes,
        tile: None,
    });
}

/// A declared vinegar tank at `tank` facing east, filled.
fn filled_tank(world: &mut World, tank: TileCoord) {
    build(world, "fluid_tank", tank, Direction::East);
    world.issue_command(PlayerCommand::SetTankFluid {
        tile: tank,
        fluid: "vinegar".to_string(),
    });
    common::buy(world, "vinegar", FLUID_TANK_CAPACITY);
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: Some(tank),
    });
}

fn pipe_volume(world: &World, tile: TileCoord) -> f64 {
    world
        .fluid_snapshots()
        .iter()
        .flat_map(|n| n.segments.iter())
        .find(|s| s.tile == tile)
        .map(|s| s.volume)
        .expect("a pipe tile is always in some network")
}

#[test]
fn an_item_on_the_belt_survives_a_pipe_laid_across_it() {
    let mut world = funded();
    feed_tomatoes(&mut world, 4);
    // A lone belt with nothing past it: product piles up at its end and
    // stays there, which is the live state an upgrade must not disturb.
    build(&mut world, BELT_KIND, (0, 0), Direction::East);
    for _ in 0..200 {
        world.tick();
    }
    let before = belt_state(&world);
    let carried = items_here(&world, (0, 0));
    assert!(
        !carried.is_empty(),
        "the fixture must have product on the belt"
    );

    build(&mut world, PIPE_KIND, (0, 0), Direction::North);

    assert_eq!(kind_at(&world, (0, 0)).as_deref(), Some(CROSSING_KIND));
    assert_eq!(
        items_here(&world, (0, 0)),
        carried,
        "every item on the belt is still on the crossing"
    );
    assert_eq!(
        belt_state(&world),
        before,
        "...at exactly the same sub-tile positions, with the same surface clock"
    );
    assert_eq!(world.placement_count(), 2, "upgraded in place, not added");
    assert_eq!(world.owned_count(PIPE_KIND), 0, "the pipe was spent");
    assert_eq!(
        world.owned_count(BELT_KIND),
        0,
        "the belt it was laid across is not credited back — it is still standing there"
    );
}

#[test]
fn fluid_in_the_pipe_survives_a_belt_laid_across_it() {
    let mut world = funded();
    filled_tank(&mut world, (-1, 5));
    for x in 0..3 {
        build(&mut world, PIPE_KIND, (x, 5), Direction::East);
    }
    for _ in 0..60 {
        world.tick();
    }
    let volume = pipe_volume(&world, (1, 5));
    let total = world.fluid_snapshots()[0].total_volume;
    assert!(
        volume > 0.0,
        "the fixture must have fluid in the middle pipe"
    );

    build(&mut world, BELT_KIND, (1, 5), Direction::North);

    assert_eq!(kind_at(&world, (1, 5)).as_deref(), Some(CROSSING_KIND));
    let Some(TileSnapshot::Crossing {
        direction,
        pipe_direction,
        volume: crossing_volume,
        ..
    }) = world.query_tile((1, 5))
    else {
        panic!("a belt across a pipe is a crossing");
    };
    assert_eq!(
        direction,
        Direction::North,
        "the crossing faces the belt's way"
    );
    assert_eq!(
        pipe_direction,
        Direction::East,
        "and its pipe runs a quarter turn clockwise of it, along the run"
    );
    assert_eq!(crossing_volume, volume, "the pipe's fluid is still in it");
    let networks = world.fluid_snapshots();
    assert_eq!(
        networks.len(),
        1,
        "a crossing along the run keeps the run whole"
    );
    assert_eq!(networks[0].total_volume, total);
    assert_eq!(networks[0].tank_tiles, vec![(-1, 5)]);
}

#[test]
fn a_crossing_hands_solids_on_exactly_as_a_belt_tile_does() {
    // The same seven-tile line twice, once of plain belts and once with
    // crossings where a hopper discharges onto one, where a belt hands into
    // one, where one hands to a belt, and where one hands to another.
    let crossings = [(0, 0), (2, 0), (4, 0), (5, 0)];
    let line = |with_crossings: bool| {
        let mut world = funded();
        feed_tomatoes(&mut world, 20);
        for x in 0..7 {
            if with_crossings && crossings.contains(&(x, 0)) {
                build_crossing(&mut world, (x, 0), Direction::East);
            } else {
                build(&mut world, BELT_KIND, (x, 0), Direction::East);
            }
        }
        world
    };
    let mut belts = line(false);
    let mut crossed = line(true);

    // The belt half's links are a belt's links.
    let conveyor = |world: &World, tile: TileCoord| {
        let placement = world
            .world_layout()
            .into_iter()
            .find(|p| p.tile == tile)
            .unwrap();
        let links = serde_json::to_value(placement.links).unwrap();
        (links["inputs"].clone(), links["output"].clone())
    };
    for &tile in &crossings {
        assert_eq!(
            conveyor(&crossed, tile),
            conveyor(&belts, tile),
            "at {tile:?}"
        );
    }

    for tick in 0..1500 {
        belts.tick();
        crossed.tick();
        assert_eq!(
            belt_state(&crossed),
            belt_state(&belts),
            "a line with crossings in it must move product exactly as plain belt, at tick {tick}"
        );
    }
    assert_eq!(
        items_here(&crossed, (6, 0)).len() + items_here(&crossed, (5, 0)).len(),
        items_here(&belts, (6, 0)).len() + items_here(&belts, (5, 0)).len(),
    );
    assert!(
        !items_here(&crossed, (6, 0)).is_empty(),
        "product must have reached the end of the line through every crossing"
    );
}

#[test]
fn two_perpendicular_runs_meeting_at_a_crossing_stay_two_networks() {
    // A north-south run passes through the crossing at the origin; an
    // east-west run ends against its east side, which is where its belt
    // leaves. Physically they do not touch.
    let layout = |crossing: bool| {
        let mut world = funded();
        if crossing {
            build_crossing(&mut world, (0, 0), Direction::East);
        } else {
            build(&mut world, PIPE_KIND, (0, 0), Direction::North);
        }
        for y in [-2, -1, 1, 2] {
            build(&mut world, PIPE_KIND, (0, y), Direction::North);
        }
        for x in 1..4 {
            build(&mut world, PIPE_KIND, (x, 0), Direction::East);
        }
        world
    };

    let control = layout(false);
    assert_eq!(
        control.fluid_snapshots().len(),
        1,
        "control: an ordinary pipe joins on all four sides, so this is one run"
    );

    let mut world = layout(true);
    assert_eq!(
        world.fluid_snapshots().len(),
        2,
        "a crossing's pipe joins only along its axis: two runs, not one"
    );
    let through = network_of(&world, (0, 0));
    assert_eq!(network_of(&world, (0, 2)), through);
    assert_eq!(
        network_of(&world, (0, -2)),
        through,
        "the axis run passes through"
    );
    assert_ne!(
        network_of(&world, (1, 0)),
        through,
        "the run against the belt side is its own network"
    );

    // The wire agrees, from both ends of the refused adjacency.
    let links = |world: &World, tile: TileCoord| {
        let placement = world
            .world_layout()
            .into_iter()
            .find(|p| p.tile == tile)
            .unwrap();
        serde_json::to_value(placement.links).unwrap()
    };
    assert_eq!(
        links(&world, (0, 0))["connections"],
        serde_json::json!([
            {"to": "north", "kind": "pipe"},
            {"to": "south", "kind": "pipe"},
        ])
    );
    assert_eq!(
        links(&world, (1, 0))["connections"],
        serde_json::json!([{"to": "east", "kind": "pipe"}]),
        "the pipe beside the crossing is not joined to it either"
    );

    // Rotated, the axis turns with the belt: now the east-west run passes
    // through and the north and south stubs are cut off on their own.
    world.issue_command(PlayerCommand::Rotate { tile: (0, 0) });
    assert_eq!(world.fluid_snapshots().len(), 3);
    assert_eq!(network_of(&world, (1, 0)), network_of(&world, (0, 0)));
    assert_ne!(network_of(&world, (0, 1)), network_of(&world, (0, -1)));
}

#[test]
fn a_tank_discharges_into_a_crossing_only_along_its_pipe_axis() {
    let mut world = funded();
    // The tank faces the crossing's west side, where its belt comes in.
    build(&mut world, "fluid_tank", (-1, 5), Direction::East);
    build_crossing(&mut world, (0, 5), Direction::East);
    let network = &world.fluid_snapshots()[network_of(&world, (0, 5)).unwrap()];
    assert!(
        network.tank_tiles.is_empty(),
        "a tank against a crossing's closed side is not plumbed into it"
    );

    // A quarter turn puts the pipe axis east-west, facing the tank.
    world.issue_command(PlayerCommand::Rotate { tile: (0, 5) });
    let network = &world.fluid_snapshots()[network_of(&world, (0, 5)).unwrap()];
    assert_eq!(network.tank_tiles, vec![(-1, 5)]);
}

#[test]
fn remove_returns_one_belt_and_one_pipe() {
    let mut world = funded();
    build_crossing(&mut world, (0, 0), Direction::East);
    assert_eq!(world.owned_count(BELT_KIND), 0);
    assert_eq!(world.owned_count(PIPE_KIND), 0);

    world.issue_command(PlayerCommand::Remove { tile: (0, 0) });

    assert_eq!(world.placement_count(), 0);
    assert!(world.query_tile((0, 0)).is_none());
    assert_eq!(world.owned_count(BELT_KIND), 1, "the belt comes back");
    assert_eq!(world.owned_count(PIPE_KIND), 1, "and so does the pipe");
    assert_eq!(
        world.owned_count(CROSSING_KIND),
        0,
        "a crossing is never owned"
    );
    assert!(world.fluid_snapshots().is_empty());
}

#[test]
fn four_rotates_return_a_crossing_to_where_it_started() {
    let mut world = funded();
    feed_tomatoes(&mut world, 4);
    build_crossing(&mut world, (0, 0), Direction::East);
    for _ in 0..200 {
        world.tick();
    }
    let start = serde_json::to_value(world.query_tile((0, 0))).unwrap();
    let start_layout = serde_json::to_value(world.world_layout()).unwrap();
    let start_belts = belt_state(&world);

    let mut seen = Vec::new();
    for _ in 0..4 {
        world.issue_command(PlayerCommand::Rotate { tile: (0, 0) });
        let Some(TileSnapshot::Crossing {
            direction,
            pipe_direction,
            ..
        }) = world.query_tile((0, 0))
        else {
            panic!("a rotated crossing is still a crossing");
        };
        assert_eq!(pipe_direction, direction.rotate_cw());
        seen.push(direction);
    }
    assert_eq!(
        seen,
        vec![
            Direction::South,
            Direction::West,
            Direction::North,
            Direction::East
        ],
        "R cycles the whole assembly through four states"
    );
    assert_eq!(
        serde_json::to_value(world.query_tile((0, 0))).unwrap(),
        start
    );
    assert_eq!(
        serde_json::to_value(world.world_layout()).unwrap(),
        start_layout
    );
    assert_eq!(
        belt_state(&world),
        start_belts,
        "and nothing on it was lost"
    );
    assert_eq!(world.owned_count(BELT_KIND), 0);
    assert_eq!(world.owned_count(PIPE_KIND), 0);
}

#[test]
fn only_a_perpendicular_belt_and_pipe_assemble() {
    let mut world = funded();
    build(&mut world, BELT_KIND, (0, 0), Direction::East);
    for direction in [Direction::East, Direction::West] {
        assert!(
            !try_build(&mut world, PIPE_KIND, (0, 0), direction),
            "a pipe parallel to the belt ({direction:?}) is refused, not guessed at"
        );
    }
    assert_eq!(kind_at(&world, (0, 0)).as_deref(), Some(BELT_KIND));
    assert!(
        !try_build(&mut world, BELT_KIND, (0, 0), Direction::North),
        "belt onto belt"
    );

    build(&mut world, SPLITTER_KIND, (2, 0), Direction::East);
    assert!(
        !try_build(&mut world, PIPE_KIND, (2, 0), Direction::North),
        "a splitter is not a belt a pipe can cross"
    );

    build(&mut world, PIPE_KIND, (4, 0), Direction::East);
    assert!(
        !try_build(&mut world, PIPE_KIND, (4, 0), Direction::North),
        "pipe onto pipe"
    );
    assert!(
        !try_build(&mut world, BELT_KIND, (4, 0), Direction::West),
        "parallel belt"
    );

    build_crossing(&mut world, (6, 0), Direction::East);
    assert!(
        !try_build(&mut world, PIPE_KIND, (6, 0), Direction::North),
        "onto a crossing"
    );
    assert!(
        !try_build(&mut world, BELT_KIND, (6, 0), Direction::North),
        "onto a crossing"
    );

    // Never bought, so never placed by name.
    world.issue_command(PlayerCommand::Purchase {
        kind: CROSSING_KIND.to_string(),
        quantity: 1,
    });
    assert_eq!(
        world.owned_count(CROSSING_KIND),
        0,
        "a crossing is not for sale"
    );
    world.issue_command(PlayerCommand::Place {
        kind: CROSSING_KIND.to_string(),
        tile: (8, 0),
        direction: Direction::East,
        item: None,
    });
    assert!(world.query_tile((8, 0)).is_none());
}

#[test]
fn the_crossing_wire_shape_is_what_presentation_is_told_to_expect() {
    let mut world = funded();
    build(&mut world, BELT_KIND, (-1, 0), Direction::East);
    build_crossing(&mut world, (0, 0), Direction::East);
    build(&mut world, BELT_KIND, (1, 0), Direction::East);
    build(&mut world, PIPE_KIND, (0, 1), Direction::North);

    let layout = serde_json::to_value(world.world_layout()).unwrap();
    let crossing = layout
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["kind"] == CROSSING_KIND)
        .unwrap();
    assert_eq!(
        *crossing,
        serde_json::json!({
            "kind": "crossing",
            "tile": [0, 0],
            "direction": "east",
            "footprint": {"width_tiles": 1, "height_tiles": 1},
            "occupies": [[0, 0]],
            "recipe": null,
            "crewed": true,
            "links": {
                "inputs": [{"from": "west", "kind": "belt"}],
                "output": {"to": "east", "kind": "belt"},
                "connections": [{"to": "north", "kind": "pipe"}],
            },
        })
    );
    // The belt feeding it, and the pipe joined to it, see a belt and a pipe.
    let belt = layout
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["tile"] == serde_json::json!([-1, 0]))
        .unwrap();
    assert_eq!(
        belt["links"]["output"],
        serde_json::json!({"to": "east", "kind": "belt"})
    );
    let pipe = layout
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["tile"] == serde_json::json!([0, 1]))
        .unwrap();
    assert_eq!(
        pipe["links"]["connections"],
        serde_json::json!([{"to": "south", "kind": "pipe"}])
    );

    // The chunk read carries the tile once under `belts` and once under
    // `pipes`, in each list's own shape.
    let chunks: Vec<_> = world
        .all_chunks()
        .into_iter()
        .map(|chunk| world.chunk_snapshot(chunk))
        .collect();
    let belt = chunks
        .iter()
        .flat_map(|c| c.belts.iter())
        .find(|b| b.tile == (0, 0))
        .expect("in belts");
    assert_eq!(belt.direction, Direction::East);
    let pipe = chunks
        .iter()
        .flat_map(|c| c.pipes.iter())
        .find(|p| p.tile == (0, 0))
        .expect("in pipes");
    assert_eq!(
        pipe.direction,
        Direction::South,
        "the pipe half's own direction"
    );

    let hover = serde_json::to_value(world.query_tile((0, 0))).unwrap();
    assert_eq!(hover["kind"], "crossing");
    assert_eq!(hover["direction"], "east");
    assert_eq!(hover["pipe_direction"], "south");
    assert_eq!(hover["links"], crossing["links"]);
}

#[test]
fn a_world_with_crossings_round_trips_through_save_and_load() {
    let mut original = funded();
    feed_tomatoes(&mut original, 12);
    build_crossing(&mut original, (0, 0), Direction::East);
    build(&mut original, BELT_KIND, (1, 0), Direction::East);
    build_crossing(&mut original, (2, 0), Direction::East);
    // A fluid run through the second crossing, fed from a tank to the south.
    filled_tank(&mut original, (2, -3));
    // (The tank faces east; turn its run north through the crossing.)
    build(&mut original, PIPE_KIND, (3, -3), Direction::North);
    build(&mut original, PIPE_KIND, (3, -2), Direction::North);
    build(&mut original, PIPE_KIND, (3, -1), Direction::North);
    build(&mut original, BELT_KIND, (3, 0), Direction::East);
    build(&mut original, PIPE_KIND, (3, 0), Direction::North);
    build(&mut original, PIPE_KIND, (3, 1), Direction::North);
    for _ in 0..90 {
        original.tick();
    }
    assert!(
        pipe_volume(&original, (3, 0)) > 0.0,
        "the fixture must have fluid in a crossing"
    );
    assert!(
        !belt_state(&original)
            .iter()
            .all(|b| b["items"].as_array().unwrap().is_empty()),
        "and product on the line"
    );

    let text = original.to_save_ron();
    assert!(
        text.contains("Crossing("),
        "a crossing saves as its own body"
    );
    let mut loaded = World::from_save_ron(&text, recipes()).expect("the save must load");
    assert_eq!(loaded.to_save_ron(), text, "a reload is identical");
    assert_eq!(
        serde_json::to_value(loaded.world_layout()).unwrap(),
        serde_json::to_value(original.world_layout()).unwrap()
    );
    assert_eq!(
        loaded.owned_count(BELT_KIND),
        original.owned_count(BELT_KIND)
    );
    assert_eq!(
        loaded.owned_count(PIPE_KIND),
        original.owned_count(PIPE_KIND)
    );

    for tick in 0..600 {
        original.tick();
        loaded.tick();
        assert_eq!(
            belt_state(&loaded),
            belt_state(&original),
            "the reloaded line runs in lockstep, at tick {tick}"
        );
    }
    assert_eq!(loaded.to_save_ron(), original.to_save_ron());
}
