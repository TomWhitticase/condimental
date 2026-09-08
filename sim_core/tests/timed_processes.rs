//! Headless integration test for ARCHITECTURE.md §9 milestone 7, "Timed
//! processes": *"the generic tick-counted long-duration process pattern
//! (§3.5), proven with one process end-to-end."*
//!
//! **What was and wasn't already proven.** [`TimedProcess`] itself is not new
//! — it has existed since milestone 3 and every machine's craft cycle runs on
//! it, so the *mechanism* was already exercised. What it had never been
//! exercised at is a qualitatively different timescale: every process in the
//! game ran for tens to a few hundred ticks (the cooking vat's 240-tick cook
//! was the longest). GAMEPLAY.md §2's tier 6 — aged and fermented sauces —
//! needs processes that are a different kind of thing, where the wait itself
//! is the gameplay rather than a throughput number. This milestone adds one:
//! `data/recipes.ron`'s `aged_ketchup`, a **2400-tick** stage running on an
//! `aging_rack`, inserted between the cooking vat and the bottler.
//!
//! **The milestone's real claim is a negative one, so the tests are shaped
//! around it.** ARCHITECTURE.md §3.5 designed `TimedProcess` to be generic;
//! if that design held, a process ten times longer than any before it needs
//! *no* Rust change at all — not to `TimedProcess`, not to `Machine`, not to
//! `World`, not to the `SimBridge` snapshot surface, and not to
//! `economy.rs`. It did hold: this milestone changed `data/recipes.ron` and
//! `data/machines.ron` and nothing under `sim_core/src/`. So the tests below
//! prove the stage really behaves as claimed *and* pin the things that
//! didn't have to change, since "no code changed" is only a meaningful
//! result if something would fail when it does.
//!
//! Coverage here, in order:
//!
//! 1. The duration is genuinely long — an order of magnitude beyond every
//!    other process in the data file, asserted against the file rather than
//!    hardcoded twice.
//! 2. The process takes **exactly** its full duration: not complete one tick
//!    early, complete on the tick it should be.
//! 3. Pausing mid-age and resuming much later in *wall-clock* time finishes
//!    at the identical simulation tick — §3.5's whole point.
//! 4. `economy.rs` never learned this milestone happened: the contract still
//!    targets `boxed_ketchup`, and its module doesn't mention the new ids.
//! 5. The existing `SimBridge` JSON surface already carries the rack's
//!    progress; no field was added.
//!
//! Mass conservation through the new stage is covered where it already
//! lived, in `core_loop.rs` — that test asserts the invariant on every tick
//! of a full run, which now includes 2400 consecutive ticks with a batch
//! sitting inside a machine and in no visible buffer. And the end-to-end
//! "chain still pays out" proof is `economy.rs`'s own integration test,
//! which passes unmodified.

use std::time::{Duration, Instant};

use sim_core::sim::{
    FLUID_TANK_CAPACITY, MachineSnapshot, PlayerCommand, RecipeRegistry, TileSnapshot, World,
    reference_chain,
};

/// The milestone's stage, by the two ids `data/recipes.ron` and
/// `data/machines.ron` give it.
const AGING_RECIPE: &str = "aged_ketchup";
const AGING_MACHINE_KIND: &str = "aging_rack";
/// The chosen duration, pinned here so a data-file edit that changes it is a
/// deliberate act with a failing test attached, not a silent re-balance.
/// 2400 ticks = 40 seconds at ARCHITECTURE.md §3.9's 60 ticks/sec.
const AGING_TICKS: u64 = 2400;

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A world with one craft's worth of raw material loaded and its vinegar
/// tank full, fed exactly once so that every later tick is pure simulation.
/// Both halves of the pause/resume comparison below use this, which is what
/// makes them comparable: no per-tick commands means no way for the two runs
/// to diverge except through the thing under test.
fn loaded_world() -> World {
    let mut world = reference_chain(recipes());
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: None,
    });
    world.issue_command(PlayerCommand::FillHopper { amount: 4, tile: None });
    world
}

/// The aging rack's snapshot, found by recipe id across every chunk the
/// chain occupies (the chain spans two chunks since the hopper sits west of
/// x = 0).
fn rack(world: &World) -> MachineSnapshot {
    for chunk in world.all_chunks() {
        for machine in world.chunk_snapshot(chunk).machines {
            if machine.recipe == AGING_RECIPE {
                return machine;
            }
        }
    }
    panic!("the shipped chain must have a stage running the '{AGING_RECIPE}' recipe");
}

/// How many units of the aged product exist anywhere in the world — in the
/// rack's own output buffer, on any belt, or in any machine's buffers.
///
/// Counting the whole world rather than just the rack's output buffer is
/// deliberate: `World::tick` hands a finished item from a machine's output
/// onto the next stage's belt in the *same* tick it was deposited, so
/// watching only the rack would sometimes see the completed batch appear and
/// vanish inside one tick and read as "nothing was produced."
fn aged_units(world: &World) -> u32 {
    let mut total = 0;
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        for belt in &snapshot.belts {
            total += belt.items.iter().filter(|(id, _)| id == AGING_RECIPE).count() as u32;
        }
        for machine in &snapshot.machines {
            for (id, qty) in machine.input.iter().chain(machine.output.iter()) {
                if id == AGING_RECIPE {
                    total += qty;
                }
            }
        }
    }
    total
}

/// Ticks until the aging rack has actually started a batch, returning that
/// batch's `(start_tick, required_ticks)` exactly as the snapshot reports
/// them. Panics rather than looping forever if the chain never gets that
/// far, so a broken upstream stage fails loudly here instead of as a
/// timeout.
fn tick_until_aging_starts(world: &mut World) -> (u64, u64) {
    for _ in 0..20_000 {
        world.tick();
        if let Some(process) = rack(world).process {
            return process;
        }
    }
    panic!("the aging rack never received a batch from the cooking vat within 20k ticks");
}

// ---------------------------------------------------------------------
// 1. The duration is genuinely long, relative to the file it lives in.
// ---------------------------------------------------------------------

/// "An order of magnitude longer than any craft cycle" is the milestone's
/// actual requirement, so it is asserted as a *relationship* read out of
/// `data/recipes.ron` — not as a second copy of the number. Re-balancing
/// some other stage upward can therefore invalidate this milestone's proof,
/// and this test is what would say so.
#[test]
fn the_aging_stage_is_an_order_of_magnitude_longer_than_every_other_process() {
    let registry = recipes();
    let ordered = registry.ordered();

    let aging = ordered
        .iter()
        .find(|r| r.id == AGING_RECIPE)
        .expect("data/recipes.ron must define the milestone-7 aging stage");
    assert_eq!(aging.time_ticks, AGING_TICKS);
    assert_eq!(aging.machine, AGING_MACHINE_KIND);

    let longest_other = ordered
        .iter()
        .filter(|r| r.id != AGING_RECIPE)
        .map(|r| r.time_ticks)
        .max()
        .expect("the chain has other stages");
    assert_eq!(
        longest_other, 240,
        "the cooking vat was the longest process before this milestone; if that changed, \
         the comparison below is still the point but this landmark needs updating"
    );
    assert!(
        aging.time_ticks >= longest_other * 10,
        "the aging stage ({} ticks) must be at least 10x the longest other process \
         ({longest_other} ticks) — a merely-slower craft would not prove anything \
         ARCHITECTURE.md §3.5 hadn't already proven",
        aging.time_ticks
    );
}

// ---------------------------------------------------------------------
// 2. It takes exactly its full duration — not a tick less.
// ---------------------------------------------------------------------

/// The core timing assertion, checked on the real chain rather than on a
/// `Machine` in isolation: at `required_ticks - 1` the batch is still in the
/// rack and nothing has been produced; on `required_ticks` exactly, the
/// process ends and one unit of aged product exists.
#[test]
fn the_aging_process_completes_exactly_at_required_ticks_and_not_a_tick_before() {
    let mut world = loaded_world();
    let (start_tick, required_ticks) = tick_until_aging_starts(&mut world);

    assert_eq!(
        required_ticks, AGING_TICKS,
        "the rack's live process must run for the duration the data file declares"
    );
    assert_eq!(
        aged_units(&world),
        0,
        "nothing is aged yet at the instant the batch is loaded"
    );

    // One tick short of the boundary.
    while world.current_tick() < start_tick + required_ticks - 1 {
        world.tick();
        assert_eq!(
            rack(&world).process,
            Some((start_tick, required_ticks)),
            "the batch must stay in progress, unchanged, for the whole wait (tick {})",
            world.current_tick()
        );
        assert_eq!(
            aged_units(&world),
            0,
            "the aging stage must not complete early (tick {})",
            world.current_tick()
        );
    }

    // The boundary tick itself.
    world.tick();
    assert_eq!(world.current_tick(), start_tick + required_ticks);
    assert!(
        rack(&world).process.is_none(),
        "the process must have ended on exactly the tick its required_ticks elapsed"
    );
    assert_eq!(
        aged_units(&world),
        1,
        "exactly one aged unit must exist once the full duration has elapsed"
    );
}

// ---------------------------------------------------------------------
// 3. Tick-counted, not wall-clock (ARCHITECTURE.md §3.5's whole point).
// ---------------------------------------------------------------------

/// Two identical worlds age one batch; the second is **paused** partway
/// through — no `tick()` calls at all — for a quarter of a second of real
/// time before resuming. Both must finish on the same simulation tick.
///
/// A quarter of a second is roughly fifteen ticks at ARCHITECTURE.md §3.9's
/// 60 ticks/sec, so any wall-clock leakage into the timing (a `SystemTime`
/// read, an `Instant` delta) would show up as a completion-tick difference
/// rather than being lost in rounding. The test also asserts that the paused
/// run really did spend at least the pause in wall-clock time, so it cannot
/// quietly degrade into comparing two identical runs.
#[test]
fn pausing_mid_age_in_wall_clock_time_does_not_change_the_tick_it_completes_on() {
    const PAUSE: Duration = Duration::from_millis(250);

    // --- Control: never paused. ---
    let mut control = loaded_world();
    let (control_start, required) = tick_until_aging_starts(&mut control);
    let control_began = Instant::now();
    while aged_units(&control) == 0 {
        control.tick();
    }
    let control_wall = control_began.elapsed();
    let control_finish = control.current_tick();
    assert_eq!(control_finish, control_start + required);

    // --- Paused: identical, but stopped dead partway through the age. ---
    let mut paused = loaded_world();
    let (paused_start, paused_required) = tick_until_aging_starts(&mut paused);
    assert_eq!(
        (paused_start, paused_required),
        (control_start, required),
        "the two runs must be identical up to the pause, or the comparison proves nothing"
    );

    let paused_began = Instant::now();
    while paused.current_tick() < paused_start + required / 2 {
        paused.tick();
    }

    // The pause itself: real time passes, no ticks are issued.
    let mid_process = rack(&paused).process;
    let mid_tick = paused.current_tick();
    std::thread::sleep(PAUSE);
    assert_eq!(
        paused.current_tick(),
        mid_tick,
        "a pause must not advance the simulation clock — nothing ticked it"
    );
    assert_eq!(
        rack(&paused).process,
        mid_process,
        "the in-flight process must be byte-identical across the pause: a TimedProcess is \
         (start_tick, required_ticks), and neither is a function of real time"
    );

    while aged_units(&paused) == 0 {
        paused.tick();
    }
    let paused_wall = paused_began.elapsed();

    assert_eq!(
        paused.current_tick(),
        control_finish,
        "a run paused for {PAUSE:?} of wall-clock time must complete on the very same \
         simulation tick as one that never paused"
    );
    // The paused half must genuinely have spent real time standing still, or
    // the equality above would be comparing two identical runs and proving
    // nothing.
    //
    // Asserted against the PAUSE alone, deliberately, and NOT against the
    // control run's own wall clock. That comparison was here and is a flake:
    // both runs are several hundred milliseconds of debug-build ticking, and
    // on a loaded machine the run-to-run scheduling variance comfortably
    // exceeds a 250ms pause — so the unpaused control can legitimately
    // measure *longer* than the paused run and fail an assertion about a
    // property neither run has. What is actually claimed here is that real
    // time passed while the simulation did not advance, and the sleep below
    // is what makes that true regardless of how fast either run ticked.
    assert!(
        paused_wall >= PAUSE,
        "the paused run must genuinely have spent at least the pause in real time \
         ({paused_wall:?}, pause {PAUSE:?}; the control ran in {control_wall:?}), or this \
         test isn't exercising a wall-clock difference at all"
    );
}

// ---------------------------------------------------------------------
// 4. economy.rs never learned this milestone happened.
// ---------------------------------------------------------------------

/// The milestone inserted a stage **mid-chain** precisely so the chain's
/// final output id would not move, leaving `sim_core/src/sim/economy.rs`
/// untouched. This asserts that from both directions: the contract's
/// observable values are what they were, and the module's own source
/// contains no mention of anything this milestone introduced.
///
/// The source-text half is deliberately a *negative* check on the new ids
/// rather than a hash of the file. A hash would "prove" more but would also
/// fail on any unrelated comment edit, which makes it noise rather than a
/// signal; what actually matters is that the economy module has no knowledge
/// of the aging stage, and that is exactly what is asserted.
#[test]
fn every_shipped_contract_still_targets_the_chains_final_product_with_economy_rs_untouched() {
    // The chain's last stage still produces the item every shipped
    // contract's mix names, because the new stage went in the middle, not
    // at the end.
    let registry = recipes();
    let ordered = registry.ordered();
    let final_recipe = ordered.last().expect("the chain has stages");
    let final_item = &final_recipe.outputs[0].0;
    assert_eq!(
        ordered[ordered.len() - 2].id,
        "bottled_ketchup",
        "the aging stage goes upstream of bottling, not at the end of the chain"
    );

    let economy_src = include_str!("../src/sim/economy.rs");
    for new_id in [AGING_RECIPE, AGING_MACHINE_KIND] {
        assert!(
            !economy_src.contains(new_id),
            "economy.rs mentions '{new_id}' — this milestone is supposed to have required \
             zero changes there"
        );
    }

    // The live snapshot on a fresh world still has every shipped contract
    // (data/contracts.ron, three of them as of this milestone) naming the
    // chain's unmoved final output — proving the insertion mid-chain left
    // every contract's target intact, not just one hardcoded constant.
    let snapshot = reference_chain(recipes()).economy_snapshot();
    assert_eq!(snapshot.money, 0.0);
    assert!(
        !snapshot.contracts.is_empty(),
        "data/contracts.ron must ship at least one contract"
    );
    for contract in &snapshot.contracts {
        assert!(
            contract.mix.iter().any(|(item, _)| item == final_item),
            "contract '{}' must still target the chain's final output '{final_item}'",
            contract.id
        );
    }
}

// ---------------------------------------------------------------------
// 5. The existing snapshot surface already carries the rack's progress.
// ---------------------------------------------------------------------

/// Deliverable 5 of the milestone, asserted rather than asserted-in-prose:
/// nothing was added to `SimBridge`'s JSON surface, because
/// `MachineSnapshot::process` and `TileSnapshot::Machine::process` are
/// generic `(start_tick, required_ticks)` pairs and were already enough.
/// A renderer derives an aging rack's progress bar from these plus
/// `current_tick` the same way it derives a cooking vat's — which is why
/// ticket 2 (the Godot half) needs no new bridge call.
#[test]
fn the_racks_progress_is_readable_through_the_unchanged_snapshot_surface() {
    let mut world = loaded_world();
    let (start_tick, required_ticks) = tick_until_aging_starts(&mut world);
    assert_eq!(required_ticks, AGING_TICKS);

    // Via the chunk snapshot (what the renderer polls per dirty chunk).
    let snapshot = rack(&world);
    assert_eq!(snapshot.kind, AGING_MACHINE_KIND);
    assert_eq!(snapshot.process, Some((start_tick, required_ticks)));

    // Via query_tile (what a hover tooltip uses), on the rack's own tile.
    let placement = world
        .world_layout()
        .into_iter()
        .find(|p| p.recipe.as_deref() == Some(AGING_RECIPE))
        .expect("the aging rack must appear in world_layout");
    match world.query_tile(placement.tile) {
        Some(TileSnapshot::Machine {
            machine_kind,
            recipe,
            process,
            ..
        }) => {
            assert_eq!(machine_kind, AGING_MACHINE_KIND);
            assert_eq!(recipe, AGING_RECIPE);
            assert_eq!(process, Some((start_tick, required_ticks)));
        }
        other => panic!("expected the aging rack at {:?}, got {other:?}", placement.tile),
    }

    // Progress is derivable from what is already on the wire — no field
    // carries a percentage, and none needs to.
    let elapsed = world.current_tick() - start_tick;
    assert!(elapsed < required_ticks, "the batch is still aging at this point");
}
