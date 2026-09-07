//! Headless integration test for milestone 6 (ARCHITECTURE.md §9 item 6,
//! "Fluids": *pipe network, a recipe requiring a liquid input"; §3.4's
//! connected-network pressure model; §6's testing-strategy tiers).
//!
//! `sim_core/src/sim/fluid.rs` already unit-tests `FluidNetwork` in
//! isolation (source refill, pressure equalization, exact volume
//! conservation, all-or-nothing draws) and `machine.rs` unit-tests the
//! craft-start gate. This file instead drives the *whole world* — the real
//! belt chain plus the real pipe run — to prove the two halves are actually
//! wired together: a fluid-requiring stage stalls when its network can't
//! supply it, and the same chain with a satisfiable fluid requirement runs
//! to completion. It also pins the exact JSON snapshot shape the Godot
//! presentation layer consumes (ticket 2 of this milestone reads these
//! fields verbatim), so a rename here fails a test instead of silently
//! breaking the scene.
//!
//! Everything below runs with no Godot process involved (ARCHITECTURE.md
//! §7).

use sim_core::sim::types::Fx;
use sim_core::sim::{PlayerCommand, RecipeRegistry, TileSnapshot, World};

fn shipped_recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A minimal two-stage chain whose second stage needs `vinegar_units` of
/// vinegar per craft. Everything else about the two chains compared below
/// is identical, so any behavioral difference is attributable to the fluid
/// requirement and nothing else.
///
/// Deliberately shaped like the shipped chain's own front end (the
/// "Machines improvement" ticket 2 restructure): a crusher makes paste, and
/// the *mixer* is the fluid-gated stage. Both kinds are real entries in
/// `data/machines.ron`, since `World::new` uses the bundled registry.
const FLUID_GATED_RECIPE: &str = "ketchup_mix";
const FLUID_GATED_INPUT: &str = "tomato_paste";

fn two_stage_chain(vinegar_units: u32) -> RecipeRegistry {
    let text = format!(
        r#"(
            schema_version: 2,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 4)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 20,
                ),
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato_paste", 1)],
                    fluid_inputs: [("vinegar", {vinegar_units})],
                    outputs: [("ketchup_mix", 1)],
                    machine: "mixer",
                    time_ticks: 20,
                ),
            ],
        )"#
    );
    RecipeRegistry::from_ron_str(&text).expect("test recipe text must parse")
}

/// How many units of `item` exist anywhere in the world right now — on any
/// belt, or in any machine's input/output buffer. Counting *anywhere* (not
/// just in the producing machine's output slot) matters: a finished item is
/// handed onto the next stage's belt in the same tick it is produced, so a
/// mid-chain product like `ketchup` is never observable sitting in its own
/// machine's output.
fn count_item_anywhere(world: &World, item: &str) -> u32 {
    let mut total = 0;
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        for belt in &snapshot.belts {
            total += belt.items.iter().filter(|(id, _)| id == item).count() as u32;
        }
        for machine in &snapshot.machines {
            for (id, qty) in machine.input.iter().chain(machine.output.iter()) {
                if id == item {
                    total += qty;
                }
            }
        }
    }
    total
}

/// Runs `world` for `ticks`, feeding in `tomatoes` raw tomatoes (loaded
/// into the chain's hopper up front — since ticket 3 the hopper, not a
/// command, is what puts them on the belt), and returns the most of `item`
/// that ever existed anywhere in the world at once.
fn run_chain(world: &mut World, tomatoes: u32, ticks: u32, item: &str) -> u32 {
    run_chain_watching_fluid(world, tomatoes, ticks, item).0
}

/// As `run_chain`, but also reports the lowest total fluid volume seen once
/// the pipe run has had time to fill (`SETTLE_TICKS`). A dip below the
/// network's capacity after that point can only be a machine drawing from
/// it, since the source otherwise keeps the run topped up — that is how the
/// consumption half of the mechanic is observed from outside.
const SETTLE_TICKS: u32 = 100;

fn run_chain_watching_fluid(
    world: &mut World,
    tomatoes: u32,
    ticks: u32,
    item: &str,
) -> (u32, Option<f64>) {
    // Loading the hopper is instantaneous and exact (`Hopper::fill` clamps
    // at capacity and reports what it took); getting those tomatoes onto the
    // belt is the hopper's own job over the following ticks, gated by belt
    // spacing exactly as the old per-tick spawn attempts were.
    let loaded = {
        let before = world.hopper_snapshot().quantity;
        world.issue_command(PlayerCommand::FillHopper { amount: tomatoes });
        world.hopper_snapshot().quantity - before
    };
    assert_eq!(
        loaded, tomatoes,
        "the hopper must accept all {tomatoes} tomatoes this test wants to feed in"
    );

    let mut seen = 0;
    let mut min_volume: Option<f64> = None;
    for tick_i in 0..ticks {
        world.tick();
        seen = seen.max(count_item_anywhere(world, item));
        if tick_i >= SETTLE_TICKS {
            if let Some(fluid) = world.fluid_snapshot() {
                min_volume = Some(match min_volume {
                    Some(v) => v.min(fluid.total_volume),
                    None => fluid.total_volume,
                });
            }
        }
    }
    (seen, min_volume)
}

/// The headline milestone-6 assertion, in the form ARCHITECTURE.md §9 item
/// 6 asks for: the *only* difference between these two worlds is whether
/// the second stage's fluid requirement is satisfiable by the pipe run, and
/// that difference alone decides whether the chain produces anything.
#[test]
fn a_stage_stalls_without_its_fluid_and_runs_once_the_fluid_is_available() {
    // 1000 units per craft is far beyond the whole network's capacity, so
    // no amount of waiting or source refilling can ever satisfy it — the
    // machine is permanently starved of fluid while having plenty of solid
    // input, which is exactly the "stalls, doesn't run degraded" case
    // (GAMEPLAY.md §7).
    let mut starved = World::new(two_stage_chain(1000));
    let starved_output = run_chain(&mut starved, 4, 3_000, FLUID_GATED_RECIPE);
    assert_eq!(
        starved_output, 0,
        "a machine whose fluid requirement can never be met must never complete a craft"
    );

    // ...and it must not have quietly eaten its solid input while stalled.
    let mut paste_waiting = 0;
    for chunk in starved.all_chunks() {
        for machine in starved.chunk_snapshot(chunk).machines {
            if machine.recipe == FLUID_GATED_RECIPE {
                assert!(
                    machine.process.is_none(),
                    "a fluid-starved machine must be idle, not mid-craft"
                );
                paste_waiting = machine
                    .input
                    .iter()
                    .find(|(id, _)| id == FLUID_GATED_INPUT)
                    .map(|(_, qty)| *qty)
                    .unwrap_or(0);
            }
        }
    }
    assert!(
        paste_waiting > 0,
        "the stalled stage should be sitting on unconsumed solid input — otherwise this test \
         proved nothing about the fluid gate (it would just be an empty chain)"
    );

    // Same chain, same everything, a fluid requirement the pipe run can
    // actually meet.
    let mut supplied = World::new(two_stage_chain(2));
    let supplied_output = run_chain(&mut supplied, 4, 3_000, FLUID_GATED_RECIPE);
    assert_eq!(
        supplied_output, 1,
        "with fluid available the identical chain must produce its output"
    );
}

/// The shipped chain (`data/recipes.ron`) still runs end to end now that
/// its *mixing* stage needs vinegar — i.e. the pipe run `World` builds
/// really does deliver fluid to the machine that asked for it, and the
/// network really is being drawn from. Watching `ketchup` (the cooking
/// vat's output, one stage *past* the fluid gate) rather than the mix
/// itself is deliberate: nothing downstream of the mixer can exist unless
/// the mixer actually got its vinegar.
#[test]
fn the_shipped_chain_consumes_fluid_as_it_produces() {
    let mut world = World::new(shipped_recipes());
    let before = world
        .fluid_snapshot()
        .expect("data/recipes.ron declares a fluid recipe, so a pipe run must exist");
    assert_eq!(before.fluid, "vinegar");
    assert_eq!(
        before.machine_recipe, FLUID_GATED_RECIPE,
        "ticket 2 moved the fluid requirement onto the mixing stage — the pipe run must \
         follow the data file, not a hardcoded stage"
    );
    assert!(
        before.units_per_craft > 0,
        "the fluid recipe must consume a non-zero amount"
    );

    let (produced, min_volume) = run_chain_watching_fluid(&mut world, 4, 20_000, "ketchup");
    assert_eq!(
        produced, 1,
        "the shipped chain must still reach its cooked-ketchup stage"
    );

    let min_volume = min_volume.expect("the run is long enough to observe the settled network");
    assert!(
        min_volume < before.total_capacity,
        "total volume never dipped below capacity ({min_volume} vs {}) — the craft was not \
         actually drawing from the network",
        before.total_capacity
    );

    // ...and the source keeps the run supplied afterwards, so one craft
    // doesn't permanently dry out the line.
    let after = world.fluid_snapshot().unwrap();
    assert!(
        after.total_volume <= after.total_capacity,
        "volume must never exceed capacity"
    );
    assert!(after.total_volume > 0.0);
}

/// A recipe file with no `fluid_inputs` anywhere must behave exactly as it
/// did before milestone 6: no pipe run, no fluid snapshot, no change.
#[test]
fn a_chain_with_no_fluid_recipe_has_no_pipe_run_at_all() {
    let registry = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 2,
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
    .unwrap();
    let mut world = World::new(registry);
    world.tick();

    assert!(world.fluid_snapshot().is_none());
    for chunk in world.all_chunks() {
        assert!(
            world.chunk_snapshot(chunk).pipes.is_empty(),
            "no fluid recipe means no pipes anywhere"
        );
    }
}

/// Pins the JSON wire shape ticket 2 (Godot presentation) consumes. If a
/// field is renamed or dropped, this fails here rather than in a scene that
/// silently renders nothing.
#[test]
fn fluid_snapshot_json_has_the_documented_shape() {
    let mut world = World::new(shipped_recipes());
    for _ in 0..50 {
        world.tick();
    }

    let snapshot = world.fluid_snapshot().expect("pipe run exists");
    let value: serde_json::Value =
        serde_json::to_value(&snapshot).expect("FluidSnapshot must serialize");

    for field in [
        "fluid",
        "total_volume",
        "total_capacity",
        "pressure",
        "segments",
        "inlet_index",
        "machine_tile",
        "machine_recipe",
        "units_per_craft",
    ] {
        assert!(
            value.get(field).is_some(),
            "FluidSnapshot JSON must carry '{field}' — ticket 2 reads it by name"
        );
    }

    let segments = value["segments"].as_array().expect("segments is an array");
    assert_eq!(segments.len(), snapshot.segments.len());
    for segment in segments {
        for field in ["tile", "volume", "capacity", "pressure", "is_source"] {
            assert!(
                segment.get(field).is_some(),
                "PipeSnapshot JSON must carry '{field}'"
            );
        }
        let pressure = segment["pressure"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&pressure), "pressure is a fill fraction");
    }

    // Geometry contract: exactly one source, it comes first, the inlet is
    // last, and the inlet tile is orthogonally adjacent to the machine it
    // feeds — ticket 2 draws the run assuming all four.
    let sources: Vec<usize> = snapshot
        .segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.is_source)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(sources, vec![0], "exactly one source, at the head of the run");
    assert_eq!(
        snapshot.inlet_index,
        snapshot.segments.len() - 1,
        "the inlet is the far end of the run"
    );
    let inlet_tile = snapshot.segments[snapshot.inlet_index].tile;
    let (mx, my) = snapshot.machine_tile;
    assert_eq!(
        (inlet_tile.0 - mx).abs() + (inlet_tile.1 - my).abs(),
        1,
        "the inlet segment must sit directly beside the machine it feeds"
    );
    assert_eq!(snapshot.machine_recipe, FLUID_GATED_RECIPE);
}

/// Pipes are reachable through the two existing per-location APIs too, so a
/// chunk-driven renderer and a hover tooltip both work without a special
/// case.
#[test]
fn pipes_appear_in_chunk_snapshots_and_query_tile() {
    let mut world = World::new(shipped_recipes());
    for _ in 0..50 {
        world.tick();
    }
    let snapshot = world.fluid_snapshot().unwrap();

    let mut pipes_in_chunks = 0;
    for chunk in world.all_chunks() {
        pipes_in_chunks += world.chunk_snapshot(chunk).pipes.len();
    }
    assert_eq!(
        pipes_in_chunks,
        snapshot.segments.len(),
        "every pipe segment must show up in exactly one chunk snapshot"
    );

    for (i, segment) in snapshot.segments.iter().enumerate() {
        match world.query_tile(segment.tile) {
            Some(TileSnapshot::Pipe {
                fluid,
                is_source,
                is_inlet,
                pressure,
                ..
            }) => {
                assert_eq!(fluid, "vinegar");
                assert_eq!(is_source, i == 0);
                assert_eq!(is_inlet, i == snapshot.inlet_index);
                assert!((0.0..=1.0).contains(&pressure));
            }
            other => panic!("expected a pipe at {:?}, got {other:?}", segment.tile),
        }
    }

    // Pipes must not have displaced anything: every belt/machine tile still
    // answers as it did before (the pipe run is on its own row).
    for stage in world.chain_layout() {
        assert!(matches!(
            world.query_tile(stage.machine_tile),
            Some(TileSnapshot::Machine { .. })
        ));
        assert!(matches!(
            world.query_tile(stage.belt_start_tile),
            Some(TileSnapshot::Belt { .. })
        ));
    }
}

/// The whole point of §3.4's pressure model, observed at world level: fluid
/// enters at the source and works its way along the run over time, rather
/// than the inlet simply being declared full.
#[test]
fn the_pipe_run_fills_from_the_source_over_successive_ticks() {
    let world_at = |ticks: u32| {
        let mut world = World::new(shipped_recipes());
        for _ in 0..ticks {
            world.tick();
        }
        world.fluid_snapshot().unwrap()
    };

    let first = world_at(0);
    assert_eq!(
        first.segments[first.inlet_index].volume, 0.0,
        "the inlet starts empty — the source has to push fluid down the run"
    );
    assert!(
        first.total_volume > 0.0,
        "the source segment itself starts full"
    );

    let later = world_at(30);
    assert!(
        later.segments[later.inlet_index].volume > 0.0,
        "pressure must have carried fluid to the far end of the run"
    );
    assert!(
        later.total_volume > first.total_volume,
        "the run holds more fluid overall once it has filled"
    );
}

/// Guards ARCHITECTURE.md §3.9 at the type level for the new state: fluid
/// volume is `Fx`, and the presentation-facing `f64` is a one-way
/// conversion at the snapshot edge only.
#[test]
fn fluid_volume_is_fixed_point_simulation_state() {
    let mut world = World::new(shipped_recipes());
    for _ in 0..30 {
        world.tick();
    }
    let snapshot = world.fluid_snapshot().unwrap();
    // Round-tripping through Fx must be lossless, which it can only be if
    // the snapshot came from an Fx in the first place (an f64 accumulator
    // would drift into values Fx can't represent exactly).
    for segment in &snapshot.segments {
        let as_fx = Fx::from_num(segment.volume);
        assert_eq!(
            as_fx.to_num::<f64>(),
            segment.volume,
            "snapshot volume must be an exactly-representable Fx value"
        );
    }
}
