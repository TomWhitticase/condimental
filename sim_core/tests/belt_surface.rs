//! Headless integration test for the belt's **surface clock** (issue #79).
//!
//! A belt tile's rubber pattern used to be a pure function of the global
//! simulation tick, so every belt in the world ran its surface forever —
//! including one that was completely backed up, where the items said
//! "stopped" and the rubber said "carrying". `UI_UX.md` §6a's rule for a
//! continuous effect (it is driven by the thing's own work clock, not by
//! its existence) says the rubber has to stop with the items.
//!
//! Godot cannot work that out for itself: a belt that stops moving stops
//! dirtying its chunk, so the presentation layer is never told the tile
//! stood still. So the simulation counts, per tile, the ticks that tile's
//! rubber ran for, and `BeltSnapshot::moved_ticks` carries it.
//!
//! What stops that clock is a JAM, and only a jam (2026-09-25). It used to
//! stop on an empty tile too, which made one line of belt roll one tile at
//! a time as a single box walked down it, and made still rubber mean either
//! "jammed" or "nothing here" — so the jam this counter exists to show
//! could not be read off the floor. A conveyor with no box on it is a motor
//! turning, not a starved machine. This file proves the three properties
//! the renderer relies on:
//!
//! 1. A tile advances once per tick while it runs — carrying product or
//!    idle — and every running tile in the world is on the same phase, so a
//!    line of belts reads as one conveyor rather than a row of machines.
//! 2. A jammed tile's count freezes — and stays frozen for as long as the
//!    jam lasts, which is what makes "missing from a delta" mean "still
//!    where you last drew it". It rejoins the phase when it moves again.
//! 3. It survives a save/load, so a loaded factory's rubber stands where
//!    the saved one's did.

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::{Money, TileCoord};
use sim_core::sim::{
    ContractRegistry, Direction, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, World,
};

/// The smallest content set a hopper will meter tomatoes out of.
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
    .expect("the test recipe text must parse")
}

fn funded() -> World {
    World::with_content_and_money(
        one_stage(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    )
}

fn build(world: &mut World, kind: &str, tile: TileCoord, direction: Direction) {
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
        "the fixture must be able to place a {kind} at {tile:?}"
    );
}

/// A hopper at `(-1, 0)` metering tomatoes onto `tiles` belt tiles running
/// east from the origin. Nothing stands at the far end, so the run fills up
/// and jams against its own last tile — the state the issue is about.
fn dead_end_run(tiles: i32, tomatoes: u32) -> (World, Vec<TileCoord>) {
    let mut world = funded();
    build(&mut world, "hopper", (-1, 0), Direction::East);
    let mut laid = Vec::new();
    let mut tile = (0, 0);
    for _ in 0..tiles {
        build(&mut world, "belt", tile, Direction::East);
        laid.push(tile);
        tile = Direction::East.step(tile);
    }
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: tomatoes.max(HOPPER_CAPACITY),
    });
    world.issue_command(PlayerCommand::FillHopper {
        amount: tomatoes,
        tile: None,
    });
    (world, laid)
}

/// Every belt tile's surface clock right now, keyed by tile. Read through
/// the same chunk snapshots Godot reads, not through the segment.
fn surface_clocks(world: &World) -> std::collections::BTreeMap<TileCoord, u64> {
    let mut out = std::collections::BTreeMap::new();
    for chunk in world.all_chunks() {
        for belt in world.chunk_snapshot(chunk).belts {
            out.insert(belt.tile, belt.moved_ticks);
        }
    }
    out
}

fn clock_at(world: &World, tile: TileCoord) -> u64 {
    *surface_clocks(world)
        .get(&tile)
        .unwrap_or_else(|| panic!("no belt snapshot for {tile:?}"))
}

/// A belt with nothing on it is still running, so its rubber keeps
/// counting — one tick per tick, the same rate a loaded tile counts at.
/// That is what makes a long idle line move as one surface instead of
/// lighting up a tile at a time behind a single box, and it is the rule
/// Godot extrapolates between snapshots (`main.gd`'s `_belt_idle_phase`),
/// so the exact rate matters and not merely that it moves.
#[test]
fn an_empty_belt_keeps_its_rubber_running() {
    let (mut world, tiles) = dead_end_run(3, 0);
    const TICKS: u64 = 200;
    for _ in 0..TICKS {
        world.tick();
    }
    for tile in &tiles {
        assert!(
            matches!(
                world.query_tile(*tile),
                Some(sim_core::sim::TileSnapshot::Belt { ref items_here, .. }) if items_here.is_empty()
            ),
            "this asserts about an EMPTY tile, and {tile:?} has something on it"
        );
        assert_eq!(
            clock_at(&world, *tile),
            TICKS,
            "an idle belt runs, so its rubber counts one tick per tick like any other"
        );
    }
}

/// While product is crossing a tile, that tile counts one tick per tick —
/// which is what makes the rendered pattern cross a tile in exactly the
/// ticks an item does.
#[test]
fn a_carrying_belt_counts_one_tick_per_tick() {
    let (mut world, tiles) = dead_end_run(4, HOPPER_CAPACITY);
    let first = tiles[0];
    // Let the hopper get an item onto the first tile and moving.
    let mut before = clock_at(&world, first);
    let mut counted = 0;
    for _ in 0..40 {
        world.tick();
        let now = clock_at(&world, first);
        assert!(
            now == before || now == before + 1,
            "a tile's surface clock moves by 0 or 1 a tick, never more: {before} -> {now}"
        );
        if now > before {
            counted += 1;
        }
        before = now;
    }
    assert!(
        counted > 30,
        "a fed belt tile should be advancing product on nearly every tick, counted {counted} of 40"
    );
}

/// The whole point. Once the run is packed solid against its dead end,
/// every tile on it stops counting — and stays stopped, for long enough
/// that a renderer driving four frames off it would have shown three
/// changes.
#[test]
fn a_jammed_belt_freezes_its_surface_clock() {
    let (mut world, tiles) = dead_end_run(3, HOPPER_CAPACITY);
    for _ in 0..3000 {
        world.tick();
    }
    let jammed = surface_clocks(&world);
    assert!(
        jammed.values().all(|&ticks| ticks > 0),
        "the fixture must have run product over every tile before it jammed: {jammed:?}"
    );
    for _ in 0..200 {
        world.tick();
    }
    assert_eq!(
        surface_clocks(&world),
        jammed,
        "a backed-up line advances nothing, so no tile's rubber may advance either"
    );
    for &tile in &tiles {
        assert!(
            matches!(
                world.query_tile(tile),
                Some(sim_core::sim::TileSnapshot::Belt { ref items_here, .. }) if !items_here.is_empty()
            ),
            "the jam this asserts about must actually be items standing on {tile:?}"
        );
    }
}

/// A jam that clears REJOINS the phase every other running belt is on,
/// rather than resuming its own (amended 2026-09-25 — the owner played a
/// floor of belts laid at different times and asked for them in sync). The
/// jump that costs is one repeating pattern shifting under items at the
/// instant they start moving again; the drift it replaces was on every
/// tile, permanently.
#[test]
fn a_cleared_jam_rejoins_the_phase_the_running_belts_are_on() {
    let (mut world, tiles) = dead_end_run(3, HOPPER_CAPACITY);
    for _ in 0..3000 {
        world.tick();
    }
    let frozen = clock_at(&world, tiles[0]);
    assert!(
        frozen < world.current_tick(),
        "the jam must have left this tile behind the tick for the rejoin to mean anything"
    );
    // Open the dead end: a belt on the far side takes the front item and
    // the run starts moving again, freed space rippling back one item a
    // tick until it reaches the tile this asserts about.
    build(&mut world, "belt", (3, 0), Direction::East);
    for _ in 0..30 {
        world.tick();
    }
    // Not exactly the tick: a line that has just been freed moves in
    // stop-and-go waves, and a tile that did not move on the very last tick
    // is legitimately a tick or two behind. The point is that it is level
    // with the running floor again rather than hundreds of ticks adrift.
    let rejoined = clock_at(&world, tiles[0]);
    let behind = world.current_tick() - rejoined;
    assert!(
        behind <= 5,
        "the line is moving again, so this tile's rubber stands with every other \
         running tile's; it reads {rejoined} against tick {}, and it froze at {frozen}",
        world.current_tick()
    );
}

/// The property the whole floor is judged on: every belt that is running
/// shows the same phase, whenever it was placed and whatever it has been
/// through. A tile laid 500 ticks after its neighbour is in step with it,
/// and so is one that spent a spell jammed.
#[test]
fn every_running_belt_stands_at_the_same_phase() {
    let (mut world, _) = dead_end_run(3, 0);
    for _ in 0..500 {
        world.tick();
    }
    // A tile laid late, and one that has been jammed and freed, both join
    // the same surface as the tiles that have run all along.
    build(&mut world, "belt", (3, 0), Direction::East);
    for _ in 0..40 {
        world.tick();
    }
    let clocks = surface_clocks(&world);
    assert_eq!(clocks.len(), 4, "the fixture should have four belt tiles");
    for (tile, clock) in &clocks {
        assert_eq!(
            *clock,
            world.current_tick(),
            "every running belt is in phase, and {tile:?} is not"
        );
    }
}

/// The case the owner reported, and the one a tick of slack hides: a belt
/// bought and dropped into a floor that has been running for an hour is in
/// phase **in the frame it is first drawn in**, before it has ticked once.
///
/// Reading it after a tick or two proves nothing, because the tile's first
/// tick would fix a clock that started at zero. Godot never gets that
/// second look: it reads a placed tile once, on the rebuild the placement
/// triggers, and an idle tile dirties no chunk afterwards — so a tile that
/// is out of phase at that instant stays out of phase until something
/// crosses it.
#[test]
fn a_belt_laid_into_a_running_floor_is_in_phase_before_it_ticks() {
    let (mut world, tiles) = dead_end_run(3, 0);
    for _ in 0..777 {
        world.tick();
    }
    build(&mut world, "belt", (3, 0), Direction::East);
    // No tick here, deliberately.
    let clocks = surface_clocks(&world);
    for tile in tiles.iter().chain(std::iter::once(&(3, 0))) {
        assert_eq!(
            clocks.get(tile).copied(),
            Some(world.current_tick()),
            "{tile:?} is drawn out of phase in the very frame it is placed in"
        );
    }
}

/// A save written before the surface clock was a tick — so every idle
/// tile's number is the tick it last *carried* something, which is stale —
/// loads with its empty belts on the loaded world's phase, not on that
/// stale one. Nothing would ever correct it: an idle tile dirties no chunk,
/// so the scene draws the number it was handed at load forever.
#[test]
fn an_empty_belt_in_an_old_save_loads_in_phase() {
    let (mut world, _) = dead_end_run(3, 0);
    for _ in 0..900 {
        world.tick();
    }
    // What a pre-2026-09-25 save of this world holds: belts that have never
    // carried anything, and so never advanced a counter.
    let stale = world
        .to_save_ron()
        .replace("moved_ticks: 900", "moved_ticks: 0");
    assert!(
        stale.contains("moved_ticks: 0"),
        "the fixture must really have rewritten the saved clocks"
    );
    let loaded = World::from_save_ron(&stale, one_stage()).expect("the save must load");
    for (tile, clock) in surface_clocks(&loaded) {
        assert_eq!(
            clock,
            loaded.current_tick(),
            "{tile:?} loaded on a stale phase and nothing will ever correct it"
        );
    }
}

/// Save/load carries it, so a reloaded factory's rubber stands exactly
/// where the saved one's did under the same items.
#[test]
fn the_surface_clock_survives_a_save_and_load() {
    let (mut world, _) = dead_end_run(3, HOPPER_CAPACITY);
    for _ in 0..600 {
        world.tick();
    }
    let before = surface_clocks(&world);
    assert!(
        before.values().any(|&ticks| ticks > 0),
        "the fixture must have a non-zero clock to be worth round-tripping"
    );
    let loaded =
        World::from_save_ron(&world.to_save_ron(), one_stage()).expect("the save must load");
    assert_eq!(
        surface_clocks(&loaded),
        before,
        "a loaded belt's surface clock is the saved one's"
    );
}
