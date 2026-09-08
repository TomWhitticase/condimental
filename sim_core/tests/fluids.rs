//! Headless integration test for milestone 6 (ARCHITECTURE.md §9 item 6,
//! "Fluids": *pipe network, a recipe requiring a liquid input"; §3.4's
//! connected-network pressure model; §6's testing-strategy tiers).
//!
//! `sim_core/src/sim/fluid.rs` already unit-tests `FluidNetwork` in
//! isolation (tank fill/clamp, pressure equalization, exact volume
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
//! **The "Visual overhaul round 2" epic's ticket 1 changed where the fluid
//! comes from**, and that needed real edits here — deliberately, since the
//! behavior being tested is the behavior being removed. Milestone 6's supply
//! was a pipe segment flagged `is_source` that refilled itself to capacity
//! every tick; it is now a player-loadable [`FluidTank`] standing beside the
//! run's head, drained by the same pressure-equalization pass that moves
//! fluid along the run. So every test that wanted a *supplied* network now
//! says so with a `FillFluidTank` command, an unfilled world is the natural
//! starvation fixture, and there are new tests for the tank's own fill
//! clamping, for the full causal chain (fill -> equalize -> craft -> drain ->
//! stall), and for the JSON shapes tickets 2/3 consume verbatim.
//!
//! Everything below runs with no Godot process involved (ARCHITECTURE.md
//! §7).

use sim_core::sim::types::Fx;
use sim_core::sim::{
    FLUID_TANK_CAPACITY, FLUID_TANK_FILL_AMOUNT, FluidSnapshot, FluidTankSnapshot,
    HopperSnapshot, PlayerCommand, RecipeRegistry, TileSnapshot, World, command_from_json,
    reference_chain,
};

/// The reference chain's one hopper.
fn the_hopper(world: &World) -> HopperSnapshot {
    let mut hoppers = world.hopper_snapshots();
    assert_eq!(hoppers.len(), 1, "the reference chain places one hopper");
    hoppers.remove(0)
}

/// The world's one fluid network, or `None` when no pipe is placed.
///
/// A world can hold several networks since the "Real Placement" epic (one
/// per connected component of placed pipe), which is why the snapshot call
/// returns a list — but the reference chain lays exactly one run, so this
/// file asserts that and then talks about "the" network, the way it always
/// has.
fn the_network(world: &World) -> Option<FluidSnapshot> {
    let mut networks = world.fluid_snapshots();
    assert!(
        networks.len() <= 1,
        "the reference chain lays one pipe run, so at most one network"
    );
    networks.pop()
}

/// The world's one fluid tank, or `None` when none is placed.
fn the_tank(world: &World) -> Option<FluidTankSnapshot> {
    let mut tanks = world.fluid_tank_snapshots();
    assert!(tanks.len() <= 1, "the reference chain places one tank");
    tanks.pop()
}

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

/// Runs `world` for `ticks`, feeding in `tomatoes` raw tomatoes and a full
/// tank of fluid (both loaded up front — since ticket 3 the hopper, and
/// since the "Visual overhaul round 2" epic's ticket 1 the tank, are what
/// actually put those into the chain), and returns the most of `item` that
/// ever existed anywhere in the world at once.
fn run_chain(world: &mut World, tomatoes: u32, ticks: u32, item: &str) -> u32 {
    run_chain_watching_fluid(world, tomatoes, ticks, item).0
}

/// As `run_chain`, but also reports the lowest total fluid volume seen in
/// the *pipe run* once it has had time to fill (`SETTLE_TICKS`). A dip below
/// the run's capacity after that point can only be a machine drawing from
/// it, since the tank otherwise keeps the run topped up — that is how the
/// consumption half of the mechanic is observed from outside.
const SETTLE_TICKS: u32 = 100;

fn run_chain_watching_fluid(
    world: &mut World,
    tomatoes: u32,
    ticks: u32,
    item: &str,
) -> (u32, Option<f64>) {
    // Fill the tank first: it is the pipe run's only supply, so without this
    // the fluid-gated stage never starts and the chain produces nothing.
    // `FillFluidTank` clamps at capacity, exactly as `FillHopper` does, so
    // asking for the whole capacity is simply "fill it up".
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });

    // Loading the hopper is instantaneous and exact (`Hopper::fill` clamps
    // at capacity and reports what it took); getting those tomatoes onto the
    // belt is the hopper's own job over the following ticks, gated by belt
    // spacing exactly as the old per-tick spawn attempts were.
    let loaded = {
        let before = the_hopper(&world).quantity;
        world.issue_command(PlayerCommand::FillHopper { amount: tomatoes, tile: None });
        the_hopper(&world).quantity - before
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
            if let Some(fluid) = the_network(&world) {
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
    let mut starved = reference_chain(two_stage_chain(1000));
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
    let mut supplied = reference_chain(two_stage_chain(2));
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
    let mut world = reference_chain(shipped_recipes());
    let before =
        the_network(&world).expect("the reference chain lays a pipe run for the fluid recipe");
    assert_eq!(before.fluid.as_deref(), Some("vinegar"));
    let tap = before
        .taps
        .first()
        .expect("a machine must be tapping the run");
    assert_eq!(
        tap.machine_recipe, FLUID_GATED_RECIPE,
        "the fluid requirement is on the mixing stage — which machine taps the run follows \
         the data file and the placement, not a hardcoded stage"
    );
    assert!(
        tap.units_per_craft > 0,
        "the fluid recipe must consume a non-zero amount"
    );

    let tank_before = the_tank(&world).expect("a pipe run has a tank supplying it");
    assert_eq!(tank_before.fluid.as_deref(), Some("vinegar"));
    assert_eq!(tank_before.quantity, 0.0, "a new tank starts empty");

    let (produced, min_volume) = run_chain_watching_fluid(&mut world, 4, 20_000, "ketchup");
    assert_eq!(
        produced, 1,
        "the shipped chain must still reach its cooked-ketchup stage"
    );

    // The tank was filled to capacity by `run_chain_watching_fluid`; what it
    // holds now is strictly less, because the run drew from it to wet the
    // pipes and the mixer drew from the run. This is the assertion the
    // always-refilling source could never have supported.
    let tank_after = the_tank(&world).unwrap();
    assert_eq!(tank_after.capacity, f64::from(FLUID_TANK_CAPACITY));
    assert!(
        tank_after.quantity > 0.0 && tank_after.quantity < tank_after.capacity,
        "one craft's worth of production must visibly draw the tank down without emptying it \
         ({} of {})",
        tank_after.quantity,
        tank_after.capacity
    );
    assert!(
        // Fixed-point tolerance: `fill` is computed as an `Fx` division and
        // then converted, so it can differ from the f64 ratio by a ULP or
        // two of `Fx` (~1.5e-5). That is quantization, not disagreement.
        (tank_after.fill - tank_after.quantity / tank_after.capacity).abs() < 1e-4,
        "`fill` must be quantity/capacity — presentation drives a level from it directly"
    );

    let min_volume = min_volume.expect("the run is long enough to observe the settled network");
    assert!(
        min_volume < before.total_capacity,
        "total volume never dipped below capacity ({min_volume} vs {}) — the craft was not \
         actually drawing from the network",
        before.total_capacity
    );

    // ...and the tank keeps the run supplied afterwards, so one craft
    // doesn't permanently dry out the line.
    let after = the_network(&world).unwrap();
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
    let mut world = reference_chain(registry);
    world.tick();

    assert!(the_network(&world).is_none());
    assert!(
        the_tank(&world).is_none(),
        "no pipe run means no tank either — there would be nothing for it to supply"
    );
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        assert!(
            snapshot.pipes.is_empty(),
            "no fluid recipe means no pipes anywhere"
        );
        assert!(
            snapshot.fluid_tanks.is_empty(),
            "...and no tank anywhere either"
        );
    }

    // And the command that would fill it is a silent no-op, per
    // `issue_command`'s "an impossible command does nothing" contract —
    // never a panic on the `None`.
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });
    world.tick();
    assert!(the_tank(&world).is_none());
}

/// Pins the JSON wire shape ticket 2 (Godot presentation) consumes. If a
/// field is renamed or dropped, this fails here rather than in a scene that
/// silently renders nothing.
#[test]
fn fluid_snapshot_json_has_the_documented_shape() {
    let mut world = reference_chain(shipped_recipes());
    for _ in 0..50 {
        world.tick();
    }

    let snapshot = the_network(&world).expect("pipe run exists");
    let value: serde_json::Value =
        serde_json::to_value(&snapshot).expect("FluidSnapshot must serialize");

    // The shape changed with the "Real Placement" epic: a world now holds
    // one network per connected component of placed pipe, so the single
    // machine that used to be named inline became a `taps` array and the
    // tanks feeding it became `tank_tiles`.
    for field in [
        "fluid",
        "total_volume",
        "total_capacity",
        "pressure",
        "segments",
        "taps",
        "tank_tiles",
    ] {
        assert!(
            value.get(field).is_some(),
            "FluidSnapshot JSON must carry '{field}' — ticket 2 reads it by name"
        );
    }
    for field in ["inlet_index", "machine_tile", "machine_recipe", "units_per_craft"] {
        assert!(
            value.get(field).is_none(),
            "'{field}' moved into the `taps` array — a stale reader should fail loudly"
        );
    }
    let tap = &value["taps"][0];
    for field in [
        "machine_tile",
        "machine_recipe",
        "units_per_craft",
        "segment_index",
    ] {
        assert!(
            tap.get(field).is_some(),
            "a tap must carry '{field}': {tap}"
        );
    }

    let segments = value["segments"].as_array().expect("segments is an array");
    assert_eq!(segments.len(), snapshot.segments.len());
    for segment in segments {
        for field in [
            "tile",
            "direction",
            "volume",
            "capacity",
            "pressure",
            "is_source",
        ] {
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
    let tap = &snapshot.taps[0];
    assert_eq!(
        tap.segment_index,
        snapshot.segments.len() - 1,
        "the tapped segment is the far end of this run"
    );
    let inlet_tile = snapshot.segments[tap.segment_index].tile;
    let (mx, my) = tap.machine_tile;
    assert_eq!(
        (inlet_tile.0 - mx).abs() + (inlet_tile.1 - my).abs(),
        1,
        "the tapped segment must sit directly beside the machine it feeds"
    );
    assert_eq!(tap.machine_recipe, FLUID_GATED_RECIPE);
    assert_eq!(
        snapshot.tank_tiles.len(),
        1,
        "one tank supplies this run, and it names itself here"
    );
}

/// Pipes are reachable through the two existing per-location APIs too, so a
/// chunk-driven renderer and a hover tooltip both work without a special
/// case.
#[test]
fn pipes_appear_in_chunk_snapshots_and_query_tile() {
    let mut world = reference_chain(shipped_recipes());
    for _ in 0..50 {
        world.tick();
    }
    let snapshot = the_network(&world).unwrap();

    let mut pipes_in_chunks = 0;
    for chunk in world.all_chunks() {
        pipes_in_chunks += world.chunk_snapshot(chunk).pipes.len();
    }
    assert_eq!(
        pipes_in_chunks,
        snapshot.segments.len(),
        "every pipe segment must show up in exactly one chunk snapshot"
    );

    let inlet_index = snapshot.taps[0].segment_index;
    for (i, segment) in snapshot.segments.iter().enumerate() {
        match world.query_tile(segment.tile) {
            Some(TileSnapshot::Pipe {
                fluid,
                is_source,
                is_inlet,
                pressure,
                ..
            }) => {
                assert_eq!(fluid.as_deref(), Some("vinegar"));
                assert_eq!(is_source, i == 0);
                assert_eq!(is_inlet, i == inlet_index);
                assert!((0.0..=1.0).contains(&pressure));
            }
            other => panic!("expected a pipe at {:?}, got {other:?}", segment.tile),
        }
    }

    // Pipes must not have displaced anything: every belt/machine tile still
    // answers as it did before (the pipe run is on its own row).
    for placement in world.world_layout() {
        let expected_machine = placement.recipe.is_some();
        match world.query_tile(placement.tile) {
            Some(TileSnapshot::Machine { .. }) => assert!(expected_machine),
            Some(_) => assert!(!expected_machine),
            None => panic!("every placed tile must resolve: {:?}", placement.tile),
        }
    }
}

/// The whole point of §3.4's pressure model, observed at world level: fluid
/// enters at the tank end and works its way along the run over time, rather
/// than the inlet simply being declared full.
///
/// The "before" case is also the ticket's headline behavior change: an
/// untouched world's pipe run is **completely empty and stays that way**.
/// Before the tank existed, the run's first segment started full and topped
/// itself back up every tick forever.
#[test]
fn the_pipe_run_fills_from_the_tank_over_successive_ticks() {
    let mut untouched = reference_chain(shipped_recipes());
    for _ in 0..500 {
        untouched.tick();
    }
    let dry = the_network(&untouched).unwrap();
    assert_eq!(
        dry.total_volume, 0.0,
        "with nobody filling the tank the run must stay bone dry — there is no self-refilling \
         source any more"
    );
    assert_eq!(
        dry.segments[0].volume, 0.0,
        "including the segment flagged `is_source`, which is now an ordinary pipe tile"
    );
    assert!(
        dry.segments[0].is_source,
        "the flag itself stays: it marks which tile the tank connects to"
    );

    let world_at = |ticks: u32| {
        let mut world = reference_chain(shipped_recipes());
        world.issue_command(PlayerCommand::FillFluidTank {
            amount: FLUID_TANK_CAPACITY,
            tile: None,
        });
        for _ in 0..ticks {
            world.tick();
        }
        (the_network(&world).unwrap(), the_tank(&world).unwrap())
    };

    let (first, first_tank) = world_at(0);
    assert_eq!(
        first.total_volume, 0.0,
        "filling the tank puts nothing in the pipes until the network ticks"
    );
    assert_eq!(
        first_tank.quantity,
        f64::from(FLUID_TANK_CAPACITY),
        "...it all sits in the tank"
    );

    let (later, later_tank) = world_at(30);
    assert!(
        later.segments[later.taps[0].segment_index].volume > 0.0,
        "pressure must have carried fluid to the far end of the run"
    );
    assert!(
        later.total_volume > first.total_volume,
        "the run holds more fluid overall once it has filled"
    );
    assert!(
        later_tank.quantity < first_tank.quantity,
        "and every unit now in the run came out of the tank ({} -> {})",
        first_tank.quantity,
        later_tank.quantity
    );
}

/// Guards ARCHITECTURE.md §3.9 at the type level for the new state: fluid
/// volume is `Fx`, and the presentation-facing `f64` is a one-way
/// conversion at the snapshot edge only.
#[test]
fn fluid_volume_is_fixed_point_simulation_state() {
    let mut world = reference_chain(shipped_recipes());
    for _ in 0..30 {
        world.tick();
    }
    let snapshot = the_network(&world).unwrap();
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

// ---------------------------------------------------------------------
// The fluid tank (the "Visual overhaul round 2" epic's ticket 1).
// ---------------------------------------------------------------------

/// Where the tank stands, following the hopper's precedent exactly: one tile
/// beyond the end of the geometry it feeds, so introducing it moved nothing.
/// The shipped coordinates are written out literally rather than recomputed
/// the way the code computes them — a test that re-derived them could not
/// catch the derivation being wrong.
#[test]
fn the_tank_stands_immediately_west_of_the_pipe_runs_first_tile() {
    let world = reference_chain(shipped_recipes());
    let tank = the_tank(&world).expect("the shipped chain has a pipe run");
    let fluid = the_network(&world).unwrap();

    let pipe_tile = tank
        .pipe_tile
        .expect("the tank discharges into the run's first tile");
    assert_eq!(pipe_tile, fluid.segments[0].tile);
    assert_eq!(
        tank.tile,
        (pipe_tile.0 - tank.footprint.width_tiles, pipe_tile.1),
        "the tank's footprint must end on the tile immediately west of the pipe it feeds"
    );
    // The shipped placement: the run starts at (6, 1), so a 1x1 tank is at
    // (5, 1) — a tile no belt, machine or pipe already claimed, which is why
    // adding it moved nothing.
    assert_eq!(pipe_tile, (6, 1));
    assert_eq!(tank.tile, (5, 1));
    assert_eq!(
        (tank.footprint.width_tiles, tank.footprint.height_tiles),
        (1, 1)
    );
    assert_eq!(tank.kind, "fluid_tank");
    assert_eq!(
        tank.fluid.as_deref(),
        Some("vinegar"),
        "the held fluid comes from data/recipes.ron's fluid_inputs (via the machine tapping \
         the run this tank feeds), not from Rust"
    );
    assert_eq!(tank.capacity, f64::from(FLUID_TANK_CAPACITY));
    assert_eq!(tank.quantity, 0.0, "a new tank starts empty");
    assert_eq!(tank.fill, 0.0);

    // ...and the chain's own tiles are exactly where they were: the tank is
    // beside the run, not in it.
    for placement in world.world_layout().iter().filter(|p| p.recipe.is_some()) {
        assert!(matches!(
            world.query_tile(placement.tile),
            Some(TileSnapshot::Machine { .. })
        ));
    }
    assert!(matches!(
        world.query_tile((6, 1)),
        Some(TileSnapshot::Pipe { .. })
    ));
}

#[test]
fn filling_the_tank_past_capacity_is_clamped_not_a_panic_or_an_overflow() {
    let mut world = reference_chain(shipped_recipes());

    // `Fx` is a 16.16 fixed-point type, so `u32::MAX` is not even
    // representable — clamping has to happen before any conversion.
    world.issue_command(PlayerCommand::FillFluidTank { amount: u32::MAX, tile: None });
    let tank = the_tank(&world).unwrap();
    assert_eq!(tank.quantity, f64::from(FLUID_TANK_CAPACITY));
    assert_eq!(tank.fill, 1.0);

    // ...and again on an already-full tank: still a silent no-op, with no
    // wrap-around and nothing spilled into the pipes.
    world.issue_command(PlayerCommand::FillFluidTank { amount: u32::MAX, tile: None });
    assert_eq!(
        the_tank(&world).unwrap().quantity,
        f64::from(FLUID_TANK_CAPACITY)
    );
    assert_eq!(
        the_network(&world).unwrap().total_volume,
        0.0,
        "filling must never put anything in the pipes — that is the equalization pass's job, \
         on tick"
    );
}

#[test]
fn a_fill_command_dirties_the_tanks_chunk_so_the_fill_ui_can_redraw() {
    let mut world = reference_chain(shipped_recipes());
    // One tick first, to clear the dirt from *building* the chain: placing
    // anything dirties its chunks, and the fixture places forty-odd things.
    world.tick();
    world.issue_command(PlayerCommand::FillFluidTank { amount: 1, tile: None });
    assert_eq!(world.dirty_chunks(), &[(0, 0)]);
}

#[test]
fn the_fill_tank_command_parses_from_json_with_or_without_an_amount() {
    let mut world = reference_chain(shipped_recipes());
    world.issue_command(
        command_from_json(r#"{"type": "fill_fluid_tank", "amount": 3}"#).expect("must parse"),
    );
    assert_eq!(the_tank(&world).unwrap().quantity, 3.0);

    // Omitting `amount` means the simulation's own default load, exactly as
    // `fill_hopper` does, so the common "player pressed the load key" case
    // needn't restate a constant.
    world.issue_command(command_from_json(r#"{"type": "fill_fluid_tank"}"#).expect("must parse"));
    assert_eq!(
        the_tank(&world).unwrap().quantity,
        f64::from(3 + FLUID_TANK_FILL_AMOUNT)
    );
}

/// **The ticket's headline behavior, proved as a causal chain rather than as
/// a coincidence.** Each step is asserted in order, so a failure says which
/// link broke: fill the tank -> the equalization pass carries fluid down the
/// run -> the mixer crafts -> tank and run both drain -> once they are dry
/// the mixer stalls again, holding (not eating) its solid input.
///
/// This is the fluid twin of `hopper.rs`'s `a_jammed_belt_leaves_the_hopper_
/// holding_its_contents`, and it is the test the old always-refilling source
/// made impossible to write at all.
#[test]
fn the_tank_drains_as_the_mixer_crafts_and_the_mixer_stalls_once_it_is_dry() {
    // A deliberately small fill, so the run can actually be drained inside a
    // reasonable number of ticks — but larger than the pipe run's own
    // capacity (4 x 10 = 40 units), so the tank still holds a reserve once
    // the line is wet and that reserve is what crafting eats through.
    const FILL: u32 = 60;
    // The shipped chain, not a two-stage fixture: its mixer hands output
    // downstream, so the only thing that can stop the mixer here is fluid.
    // (A two-stage chain's last machine simply fills its own output buffer
    // and blocks, which looks identical from outside and would prove
    // nothing.)
    let mut world = reference_chain(shipped_recipes());

    // Step 1: fill the tank. Nothing is in the pipes yet.
    world.issue_command(PlayerCommand::FillFluidTank { amount: FILL, tile: None });
    assert_eq!(the_tank(&world).unwrap().quantity, f64::from(FILL));
    assert_eq!(the_network(&world).unwrap().total_volume, 0.0);

    // Step 2: the equalization pass carries it down the run to the inlet.
    for _ in 0..200 {
        world.tick();
    }
    let settled = the_network(&world).unwrap();
    assert!(
        settled.segments[settled.taps[0].segment_index].volume > 0.0,
        "pressure must have carried fluid all the way to the machine's inlet"
    );
    let tank_after_wetting = the_tank(&world).unwrap().quantity;
    assert!(
        tank_after_wetting < f64::from(FILL),
        "wetting the line has to come out of the tank"
    );
    assert!(
        tank_after_wetting > 0.0,
        "...but a fill bigger than the run holds must leave a reserve in the tank"
    );

    // Step 3: with the hopper kept fed (and the delivery dock at the chain's
    // tail clearing finished product by itself, every tick, since the
    // delivery-dock ticket), the mixer crafts — and *only* fluid can stop
    // it.
    let mut crafts = 0;
    let mut stalled_for = 0;
    for _ in 0..200_000 {
        world.issue_command(PlayerCommand::FillHopper {
            amount: FLUID_TANK_CAPACITY,
            tile: None,
        });
        world.tick();
        crafts = crafts.max(count_item_anywhere(&world, FLUID_GATED_RECIPE));
        // The mixer is starved once it sits idle while holding solid input.
        if mixer_is_starved(&world) {
            stalled_for += 1;
        } else {
            stalled_for = 0;
        }
        if stalled_for > 2_000 {
            break;
        }
    }
    assert!(crafts > 0, "the mixer must have run at all while fluid was available");
    assert!(
        stalled_for > 2_000,
        "the run must actually dry out within the tick budget, not merely be slow"
    );

    // Step 4: tank and run are both dry, and it is the *fluid* that ran out
    // — the mixer is sitting on solid input it cannot use.
    let tank = the_tank(&world).unwrap();
    assert_eq!(
        tank.quantity, 0.0,
        "sustained crafting must empty the tank completely, not strand a share in it"
    );
    assert_eq!(tank.fill, 0.0);
    let run = the_network(&world).unwrap();
    assert!(
        run.segments[run.taps[0].segment_index].volume < f64::from(run.taps[0].units_per_craft),
        "the inlet must be unable to supply another craft ({} < {})",
        run.segments[run.taps[0].segment_index].volume,
        run.taps[0].units_per_craft
    );
    assert!(
        mixer_is_starved(&world),
        "with the fluid gone the mixer must be idle while still holding solid input — a stall, \
         not a silent partial run"
    );

    // Step 5: and it is genuinely recoverable — refilling the tank restarts
    // the line, which is the whole reason this is a managed resource.
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });
    let before = crafts;
    for _ in 0..5_000 {
        world.tick();
        crafts = crafts.max(count_item_anywhere(&world, FLUID_GATED_RECIPE));
    }
    assert!(
        crafts > before || !mixer_is_starved(&world),
        "refilling the tank must get the mixer running again"
    );
}

/// Whether the fluid-gated stage is idle while holding solid input — i.e.
/// stalled on fluid specifically, rather than merely having nothing to do.
fn mixer_is_starved(world: &World) -> bool {
    for chunk in world.all_chunks() {
        for machine in world.chunk_snapshot(chunk).machines {
            if machine.recipe == FLUID_GATED_RECIPE {
                let waiting = machine
                    .input
                    .iter()
                    .find(|(id, _)| id == FLUID_GATED_INPUT)
                    .map(|(_, qty)| *qty)
                    .unwrap_or(0);
                return machine.process.is_none() && waiting > 0;
            }
        }
    }
    false
}

/// Pins the exact JSON wire shapes tickets 2/3 (Godot rendering + the tank's
/// fill UI) consume verbatim — the same guarantee `hopper.rs` gives the
/// hopper's. A rename here fails a test instead of silently breaking a scene.
#[test]
fn the_fluid_tank_json_shape_is_what_presentation_is_told_to_expect() {
    let mut world = reference_chain(shipped_recipes());
    world.issue_command(PlayerCommand::FillFluidTank { amount: 30, tile: None });

    // fluid_tank_snapshots_json (`SimBridge`) serializes an array of
    // exactly these.
    let snapshot: serde_json::Value = serde_json::from_str(
        &serde_json::to_string(&the_tank(&world).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        snapshot,
        serde_json::json!({
            "tile": [5, 1],
            "kind": "fluid_tank",
            "direction": "east",
            "footprint": { "width_tiles": 1, "height_tiles": 1 },
            "fluid": "vinegar",
            "quantity": 30.0,
            "capacity": 120.0,
            "fill": 0.25,
            "pipe_tile": [6, 1],
        })
    );

    // chunk_snapshot_json carries the identical object under `fluid_tanks`,
    // alongside the pre-existing belts/machines/pipes/hoppers arrays.
    let chunk: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.chunk_snapshot((0, 0))).unwrap())
            .unwrap();
    assert_eq!(chunk["fluid_tanks"][0], snapshot);
    for existing in ["belts", "machines", "pipes", "hoppers"] {
        assert!(
            chunk[existing].is_array(),
            "the pre-existing `{existing}` array must be untouched"
        );
    }

    // query_tile_json answers with a `fluid_tank` tile: enough for a hover
    // tooltip without a second call.
    let tile: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.query_tile((5, 1)).unwrap()).unwrap())
            .unwrap();
    assert_eq!(
        tile,
        serde_json::json!({
            "kind": "fluid_tank",
            "direction": "east",
            "footprint": { "width_tiles": 1, "height_tiles": 1 },
            "origin_tile": [5, 1],
            "fluid": "vinegar",
            "quantity": 30.0,
            "capacity": 120.0,
            "fill": 0.25,
        })
    );

    // world_layout_json (which replaced chain_layout_json) reports the tank
    // like any other placement — a placed building with no recipe.
    let entry = world
        .world_layout()
        .into_iter()
        .find(|p| p.kind == "fluid_tank")
        .expect("the tank is a placement like any other");
    assert_eq!(entry.tile, (5, 1));
    assert!(entry.recipe.is_none());
}

/// The tank's volume is fixed-point simulation state too (ARCHITECTURE.md
/// §3.9), with the presentation-facing `f64` a one-way conversion at the
/// snapshot edge — the same guard `fluid_volume_is_fixed_point_simulation_state`
/// gives the pipe run.
#[test]
fn tank_volume_is_fixed_point_simulation_state() {
    let mut world = reference_chain(shipped_recipes());
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });
    for _ in 0..30 {
        world.tick();
    }
    let quantity = the_tank(&world).unwrap().quantity;
    assert_eq!(
        Fx::from_num(quantity).to_num::<f64>(),
        quantity,
        "snapshot quantity must be an exactly-representable Fx value"
    );
}
