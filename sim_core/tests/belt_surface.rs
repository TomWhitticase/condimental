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
//! 1. A tile counts every tick it runs, once — carrying product or idle.
//! 2. A jammed tile's count freezes — and stays frozen for as long as the
//!    jam lasts, which is what makes "missing from a delta" mean "still
//!    where you last drew it".
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

/// A jam that clears resumes counting from where it stopped rather than
/// re-joining a global clock — the reason this is a counter and not a
/// boolean. The rubber must not jump a fraction of a tile at the moment
/// the line starts again.
#[test]
fn a_cleared_jam_resumes_the_clock_it_stopped_on() {
    let (mut world, tiles) = dead_end_run(3, HOPPER_CAPACITY);
    for _ in 0..3000 {
        world.tick();
    }
    let frozen = clock_at(&world, tiles[0]);
    // Open the dead end: a belt on the far side takes the front item and
    // the run starts moving again, freed space rippling back one item a
    // tick until it reaches the tile this asserts about.
    build(&mut world, "belt", (3, 0), Direction::East);
    const RESUMED_TICKS: u64 = 30;
    for _ in 0..RESUMED_TICKS {
        world.tick();
    }
    let running = clock_at(&world, tiles[0]);
    assert!(
        running > frozen,
        "the line is moving again, so its rubber must be too; stuck at {frozen}"
    );
    assert!(
        running <= frozen + RESUMED_TICKS,
        "the clock resumes from the {frozen} it stopped on — it cannot catch up with a \
         global clock, or the rubber would jump the moment the line cleared; got {running}"
    );
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
