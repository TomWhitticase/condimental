//! The machine-load epic, ticket 1: every recipe-running machine keeps a
//! rolling window of what it did (`sim::machine::LoadWindow`), the world
//! reports it with a verdict and a reason (`World::machine_load_snapshots`),
//! and the shipped line's bottleneck reads off it the way it should on a
//! real factory floor — green where the line is flat out, red before it,
//! amber after it.
//!
//! `docs/MACHINE_LOAD_PLAN.md` is the spec; the wire shape asserted here is
//! the one Godot's ticket consumes.

mod common;

use sim_core::sim::types::Direction;
use sim_core::sim::{
    ContractRegistry, Money, LOAD_BUCKET_TICKS, LOAD_WINDOW_TICKS, MachineRegistry, PlayerCommand, RecipeRegistry,
    StallKind, TileSnapshot, WorkState, World, reference_chain_with_budget,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// Long enough for the whole line to back up behind the 240-tick cooking
/// vat. The vat is flat out from about tick 1500, and the bottler and
/// packer are starved from the start, but the *upstream* half takes far
/// longer: each machine holds 8 of its output and there is a belt or a
/// pipe run between every pair, and all of that fills at the vat's pace
/// (one craft per 240 ticks) before the mixer and then the crusher stall.
/// Measured: the mixer reads blocked from about tick 3500, the crusher from
/// about tick 8300 (it was 7000 at the quarter-tile item spacing; at the
/// half-tile spacing of the quality pass one lane delivers four tomatoes
/// per 100 ticks against the crusher's 80-tick craft, so the crusher is
/// belt-limited and fills its own output a little slower). The Godot
/// fixture (`test_layouts.gd`'s `BOTTLENECK_SETTLE_TICKS`) needs the same
/// patience and says the same number.
const SETTLE_TICKS: u32 = 10_000;

/// How long the settled line is watched. One full load window, so every
/// machine's status is judged over a window that began after the line
/// settled, and every momentary stall reason has had a whole cycle of the
/// slowest machine to show itself.
const WATCH_TICKS: u32 = LOAD_WINDOW_TICKS;

/// The reference chain, stocked, kept fed every tick, run until it settles.
fn settled_line() -> World {
    let mut world = reference_chain_with_budget(recipes(), common::STOCK_COST + common::WAGE_RESERVE);
    common::stock_up(&mut world);
    for _ in 0..SETTLE_TICKS {
        common::supply(&mut world);
        world.tick();
    }
    world
}

/// What one machine looked like over [`WATCH_TICKS`] of a settled line:
/// every status it reported, every stall kind it ever gave, and the last
/// entry.
#[derive(Debug)]
struct Watched {
    statuses: Vec<WorkState>,
    stall_kinds: Vec<(StallKind, String)>,
    last: sim_core::sim::MachineLoadEntry,
}

/// Runs the settled line for [`WATCH_TICKS`] more, recording each named
/// machine every tick. A stall is *this tick's* fact and a machine mid-craft
/// has none, so a reason is asserted over the window, not at one tick.
fn watch(world: &mut World, kinds: &[&str]) -> Vec<Watched> {
    let tiles: Vec<(i32, i32)> = kinds.iter().map(|kind| tile_of(world, kind)).collect();
    let mut watched: Vec<Watched> = tiles
        .iter()
        .map(|&tile| Watched {
            statuses: Vec::new(),
            stall_kinds: Vec::new(),
            last: entry_at(world, tile),
        })
        .collect();
    for _ in 0..WATCH_TICKS {
        common::supply(world);
        world.tick();
        for (w, &tile) in watched.iter_mut().zip(&tiles) {
            let entry = entry_at(world, tile);
            w.statuses.push(entry.load.status);
            if let Some(stall) = &entry.stall {
                let kind = (stall.kind, stall.item.clone());
                if !w.stall_kinds.contains(&kind) {
                    w.stall_kinds.push(kind);
                }
            }
            w.last = entry;
        }
    }
    watched
}

fn entry_at(world: &World, tile: (i32, i32)) -> sim_core::sim::MachineLoadEntry {
    world
        .machine_load_snapshots()
        .into_iter()
        .find(|entry| entry.tile == tile)
        .unwrap_or_else(|| panic!("machine_load_snapshots must carry the machine at {tile:?}"))
}

/// The origin tile of the one placed machine of `kind`.
fn tile_of(world: &World, kind: &str) -> (i32, i32) {
    world
        .world_layout()
        .iter()
        .find(|p| p.kind == kind)
        .unwrap_or_else(|| panic!("the reference chain places a {kind}"))
        .tile
}

#[test]
fn the_shipped_line_reads_its_bottleneck_off_the_load_lights() {
    let mut world = settled_line();
    let [crusher, mixer, vat, bottler, packer] = watch(
        &mut world,
        &["crusher", "mixer", "cooking_vat", "bottler", "packer"],
    )
    .try_into()
    .unwrap();

    // The bottleneck: flat out, every tick of the window, and never
    // waiting on anything — mid-cook it has no stall, on the completion
    // tick its run has room and the mixer keeps its inlet run charged.
    assert!(
        vat.statuses.iter().all(|&s| s == WorkState::Working),
        "{vat:?}"
    );
    assert_eq!(vat.last.load.percent, 100, "a saturated vat reads exactly full: {vat:?}");
    // A warmed-up ring clears a whole bucket the tick after it fills, so
    // `observed` sits between a window less one bucket and a full window.
    assert!(
        vat.last.load.observed > LOAD_WINDOW_TICKS - LOAD_BUCKET_TICKS
            && vat.last.load.observed <= LOAD_WINDOW_TICKS,
        "{vat:?}"
    );
    assert_eq!(vat.last.load.window, LOAD_WINDOW_TICKS);
    assert!(vat.stall_kinds.is_empty(), "{vat:?}");

    // Upstream of the bottleneck: finished product with nowhere to go.
    assert!(
        crusher.statuses.iter().all(|&s| s == WorkState::Blocked),
        "{crusher:?}"
    );
    assert_eq!(
        crusher.stall_kinds,
        [(StallKind::Output, "tomato_paste".to_string())],
        "the only reason a blocked crusher ever gives is its full output"
    );
    assert!(
        mixer.statuses.iter().all(|&s| s == WorkState::Blocked),
        "{mixer:?}"
    );
    assert_eq!(
        mixer.stall_kinds,
        [(StallKind::FluidOutput, "ketchup_mix".to_string())],
        "the mixer deposits into the vat's run, which the vat drains too slowly"
    );

    // Downstream: waiting for the vat.
    assert!(
        bottler.statuses.iter().all(|&s| s == WorkState::Starved),
        "{bottler:?}"
    );
    assert!(bottler.last.load.percent < 100);
    assert_eq!(
        bottler.stall_kinds,
        [(StallKind::FluidInput, "ketchup".to_string())],
        "the bottler has a tap into the vat's outlet run; it is the run that is short"
    );
    assert!(
        packer.statuses.iter().all(|&s| s == WorkState::Starved),
        "{packer:?}"
    );
    assert_eq!(
        packer.stall_kinds,
        [(StallKind::Input, "bottled_ketchup".to_string())]
    );
}

#[test]
fn load_snapshots_cover_every_machine_and_no_building_in_layout_order() {
    let world = settled_line();
    let expected: Vec<(i32, i32)> = world
        .world_layout()
        .iter()
        .filter(|p| p.recipe.is_some())
        .map(|p| p.tile)
        .collect();
    assert_eq!(expected.len(), 5, "the shipped chain runs five recipes");
    let found: Vec<(i32, i32)> = world
        .machine_load_snapshots()
        .iter()
        .map(|entry| entry.tile)
        .collect();
    assert_eq!(
        found, expected,
        "one entry per recipe-running machine, in the topology's tile order, and none for \
         the hoppers, the tank, the dock or any belt or pipe"
    );
}

/// The same objects ride every existing snapshot, so a consumer that
/// already has a chunk or a tile does not need a second call.
#[test]
fn chunk_and_tile_snapshots_carry_the_same_load_and_stall() {
    let world = settled_line();
    for entry in world.machine_load_snapshots() {
        let in_chunk = world
            .chunk_snapshot(sim_core::sim::types::chunk_of(entry.tile))
            .machines
            .into_iter()
            .find(|m| m.tile == entry.tile)
            .expect("the chunk holding a machine's origin reports it");
        assert_eq!(in_chunk.load, entry.load);
        assert_eq!(in_chunk.stall, entry.stall);
        match world.query_tile(entry.tile) {
            Some(TileSnapshot::Machine { load, stall, .. }) => {
                assert_eq!(load, entry.load);
                assert_eq!(stall, entry.stall);
            }
            other => panic!("expected a machine at {:?}, got {other:?}", entry.tile),
        }
    }
}

/// The wire shape, verbatim from `docs/MACHINE_LOAD_PLAN.md`: ticket 3
/// reads exactly these keys and these words.
#[test]
fn the_wire_shape_is_the_one_the_plan_promises() {
    let world = settled_line();
    let json = serde_json::to_value(world.machine_load_snapshots()).unwrap();
    let entries = json.as_array().expect("an array");
    for entry in entries {
        // `serde_json::Value` sorts object keys; the wire's own order is
        // the struct's (`tile`, `load`, `stall`), and Godot reads by key.
        let keys: Vec<&str> = entry.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["load", "stall", "tile"]);
        let load = entry["load"].as_object().unwrap();
        let mut load_keys: Vec<&str> = load.keys().map(String::as_str).collect();
        load_keys.sort_unstable();
        assert_eq!(
            load_keys,
            ["blocked", "observed", "percent", "starved", "status", "window", "working"]
        );
        let status = load["status"].as_str().unwrap();
        assert!(matches!(status, "working" | "starved" | "blocked"), "{status}");
        if let Some(stall) = entry["stall"].as_object() {
            let kind = stall["kind"].as_str().unwrap();
            assert!(
                matches!(
                    kind,
                    "input" | "fluid_input" | "no_supply_pipe" | "output" | "fluid_output"
                        | "no_outlet_pipe"
                ),
                "{kind}"
            );
            assert!(stall["item"].is_string());
        } else {
            assert!(entry["stall"].is_null());
        }
    }
    // One concrete stall, spelled the way the plan spells it. The crusher
    // is mid-craft on some ticks (stall `null`), so find a tick it is not.
    let crusher_tile = tile_of(&world, "crusher");
    let mut world = world;
    let crusher_stall = (0..LOAD_WINDOW_TICKS)
        .find_map(|_| {
            common::supply(&mut world);
            world.tick();
            let stall = serde_json::to_value(entry_at(&world, crusher_tile).stall).unwrap();
            (!stall.is_null()).then_some(stall)
        })
        .expect("a blocked crusher reports a stall within one window");
    assert_eq!(crusher_stall, serde_json::json!({"kind": "output", "item": "tomato_paste"}));
}

/// A machine that has never been fed: starved, and it names the first
/// solid input its recipe wants.
#[test]
fn a_machine_placed_and_never_fed_reads_starved_for_its_first_input() {
    let mut world = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(5_000),
    );
    world.issue_command(PlayerCommand::Purchase {
        kind: "crusher".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "crusher".to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
    });
    let before = world.machine_load_snapshots();
    assert_eq!(before.len(), 1);
    assert_eq!(
        before[0].load.status,
        WorkState::Starved,
        "nothing observed yet reads starved: {:?}",
        before[0]
    );
    assert_eq!(before[0].load.observed, 0);

    for _ in 0..100 {
        world.tick();
    }
    let entry = &world.machine_load_snapshots()[0];
    assert_eq!(entry.tile, (0, 0));
    assert_eq!(entry.load.status, WorkState::Starved);
    assert_eq!(entry.load.starved, 100);
    assert_eq!(entry.load.working, 0);
    assert_eq!(entry.load.percent, 0);
    let stall = entry.stall.as_ref().expect("an unfed machine says what it waits for");
    assert_eq!(stall.kind, StallKind::Input);
    assert_eq!(stall.item, "tomato");
}

/// The fluid-side reasons: a recipe that wants a fluid and has no pipe at
/// all, and one that deposits with no outlet.
#[test]
fn an_unplumbed_machine_names_the_missing_pipe() {
    let mut world = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(5_000),
    );
    for kind in ["bottler", "cooking_vat"] {
        world.issue_command(PlayerCommand::Purchase {
            kind: kind.to_string(),
            quantity: 1,
        });
    }
    world.issue_command(PlayerCommand::Place {
        kind: "bottler".to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "cooking_vat".to_string(),
        tile: (5, 5),
        direction: Direction::East,
        item: None,
    });
    for _ in 0..10 {
        world.tick();
    }
    let snapshots = world.machine_load_snapshots();
    assert_eq!(snapshots.len(), 2);
    let bottler = snapshots.iter().find(|e| e.tile == (0, 0)).unwrap();
    assert_eq!(
        bottler.stall.as_ref().map(|s| (s.kind, s.item.as_str())),
        Some((StallKind::NoSupplyPipe, "ketchup"))
    );
    let vat = snapshots.iter().find(|e| e.tile == (5, 5)).unwrap();
    assert_eq!(
        vat.stall.as_ref().map(|s| (s.kind, s.item.as_str())),
        Some((StallKind::NoSupplyPipe, "ketchup_mix")),
        "an idle vat is short of its input before it can be short of an outlet"
    );
}

/// Load is a polled readout, like fluid: it must never dirty a chunk on
/// its own, or every machine would dirty its chunk every tick.
#[test]
fn a_changing_load_dirties_nothing() {
    let mut world = World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(5_000),
    );
    world.issue_command(PlayerCommand::Purchase {
        kind: "crusher".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "crusher".to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
    });
    world.tick();
    for _ in 0..50 {
        world.tick();
        assert!(
            world.dirty_chunks().is_empty(),
            "a starving crusher's window grows every tick and dirties nothing"
        );
    }
    assert_eq!(world.machine_load_snapshots()[0].load.starved, 51);
}
