//! Headless integration test for the "Machines improvement" milestone's
//! ticket 3: the chain is fed by a **hopper** — a real building that buffers
//! raw ingredient and meters it onto the first belt — instead of by a
//! command that teleported items onto that belt.
//!
//! `sim_core/src/sim/hopper.rs` already unit-tests `Hopper` in isolation
//! (fill clamping, overflow safety, one-unit-per-call unloading, the
//! blocked-belt case). This file drives the real `World` instead, to prove
//! the three things only the whole system can show:
//!
//! 1. **Placement.** The hopper stands immediately west of stage 0's belt,
//!    on tiles that existed in no earlier milestone — so introducing it
//!    moved nothing (the same discipline ticket 1 followed for footprints).
//! 2. **Metering.** `World::tick` unloads at most one unit, only when the
//!    belt has room, so the *machinery* sets the feed rate and the player
//!    only sets the supply. A jammed line leaves the hopper holding.
//! 3. **The wire shape.** The exact JSON field names ticket 4 (Godot
//!    rendering + the hopper's fill UI) consumes verbatim, pinned the way
//!    `fluids.rs` and `machine_footprints.rs` pin theirs — a rename here
//!    fails a test instead of silently breaking the scene.
//!
//! Mass conservation across the whole chain *through* the hopper is covered
//! in `core_loop.rs`, which counts hopper contents as mass in the system.

use sim_core::sim::{
    HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, PlayerCommand, RecipeRegistry, World,
    command_from_json,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    World::new(recipes())
}

/// How many items are on the chain's first belt right now.
fn first_belt_item_count(world: &World) -> usize {
    let start = world.chain_layout()[0].belt_start_tile;
    for chunk in world.all_chunks() {
        for belt in world.chunk_snapshot(chunk).belts {
            if belt.start_tile == start {
                return belt.items.len();
            }
        }
    }
    0
}

// ---------------------------------------------------------------------
// 1. Placement.
// ---------------------------------------------------------------------

#[test]
fn the_hopper_stands_immediately_west_of_the_chains_first_belt() {
    let world = world();
    let hopper = world.hopper_snapshot();
    let first_belt = world.chain_layout()[0].belt_start_tile;

    assert_eq!(hopper.belt_tile, first_belt);
    assert_eq!(
        hopper.tile,
        (first_belt.0 - hopper.footprint.width_tiles, first_belt.1),
        "the hopper's footprint must end on the tile immediately west of the belt it feeds"
    );
    // The shipped placement, written out rather than recomputed: the chain
    // starts at x = 0, so a 1x1 hopper is at x = -1 — outside the chain's
    // own span, which is exactly why adding it moved no existing tile.
    assert_eq!(hopper.tile, (-1, 0));
    assert_eq!(
        (
            hopper.footprint.width_tiles,
            hopper.footprint.height_tiles
        ),
        (1, 1)
    );
    assert_eq!(hopper.kind, "hopper");
    assert_eq!(
        hopper.item, "tomato",
        "the held item comes from data/recipes.ron's first recipe's input, not from Rust"
    );
    assert_eq!(hopper.capacity, HOPPER_CAPACITY);
    assert_eq!(hopper.quantity, 0, "a new hopper starts empty");
    assert_eq!(hopper.fill, 0.0);
}

/// The shipped chain sits in chunk (0, 0); the hopper sits west of it, in
/// chunk (-1, 0). A renderer that only ever asked for the chain's own chunk
/// would never draw the hopper, so `all_chunks` has to report both.
#[test]
fn the_hoppers_chunk_is_reported_alongside_the_chains() {
    let world = world();
    let chunks = world.all_chunks();
    assert!(chunks.contains(&(0, 0)), "{chunks:?}");
    assert!(chunks.contains(&(-1, 0)), "{chunks:?}");
    assert_eq!(
        world.chunk_snapshot((-1, 0)).hoppers.len(),
        1,
        "the hopper must be in the snapshot of the chunk it occupies"
    );
    assert!(
        world.chunk_snapshot((0, 0)).hoppers.is_empty(),
        "and only in that chunk — its footprint doesn't reach the chain's"
    );
}

// ---------------------------------------------------------------------
// 2. Filling and metering.
// ---------------------------------------------------------------------

#[test]
fn filling_past_capacity_is_clamped_not_a_panic_or_an_overflow() {
    let mut world = world();

    world.issue_command(PlayerCommand::FillHopper { amount: u32::MAX });
    assert_eq!(world.hopper_snapshot().quantity, HOPPER_CAPACITY);
    assert_eq!(world.hopper_snapshot().fill, 1.0);

    // ...and again on an already-full hopper: still a silent no-op, with no
    // wrap-around and nothing spilled onto the belt.
    world.issue_command(PlayerCommand::FillHopper { amount: u32::MAX });
    assert_eq!(world.hopper_snapshot().quantity, HOPPER_CAPACITY);
    assert_eq!(
        first_belt_item_count(&world),
        0,
        "filling must never put anything on a belt — that is the hopper's job, on tick"
    );
}

#[test]
fn a_fill_command_dirties_the_hoppers_chunk_so_the_fill_ui_can_redraw() {
    let mut world = world();
    world.issue_command(PlayerCommand::FillHopper { amount: 1 });
    assert_eq!(world.dirty_chunks(), &[(-1, 0)]);
}

/// The core of the ticket: the belt is fed by the simulation, one unit at a
/// time, at the rate the belt can accept — not by the command.
#[test]
fn the_hopper_unloads_exactly_one_unit_at_a_time_gated_by_the_belt() {
    let mut world = world();
    world.issue_command(PlayerCommand::FillHopper { amount: 10 });

    world.tick();
    assert_eq!(
        world.hopper_snapshot().quantity,
        9,
        "exactly one unit leaves the hopper on a tick, never the whole buffer"
    );
    assert_eq!(first_belt_item_count(&world), 1);

    // The very next tick, the belt's min-spacing rule (belt.rs) means there
    // is no room at its start yet — so the hopper must simply hold.
    world.tick();
    assert_eq!(
        world.hopper_snapshot().quantity,
        9,
        "a belt with no room at its start must not accept a second unit"
    );
    assert_eq!(first_belt_item_count(&world), 1);

    // Given enough ticks the whole load reaches the line.
    for _ in 0..2_000 {
        world.tick();
    }
    assert_eq!(world.hopper_snapshot().quantity, 0);
}

/// The stall case, at world level: nothing downstream ever consumes, so the
/// first belt fills up and the hopper has to keep the rest. This is the
/// same "gate on availability, consume on success" contract a `Machine` has
/// with its inputs — it holds, it never spills and never silently drops.
#[test]
fn a_jammed_belt_leaves_the_hopper_holding_its_contents() {
    // A one-stage chain whose machine never consumes (its recipe wants an
    // item nothing on this belt ever becomes), so the belt backs up
    // completely and stays that way.
    let jammed = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 2,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 4)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 100000000,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse");
    let mut world = World::new(jammed);
    world.issue_command(PlayerCommand::FillHopper {
        amount: HOPPER_CAPACITY,
    });

    for _ in 0..20_000 {
        world.tick();
    }

    let held = world.hopper_snapshot().quantity;
    assert!(
        held > 0,
        "a backed-up line must leave raw ingredient in the hopper, not consume it into nothing"
    );
    let on_belt = first_belt_item_count(&world) as u32;
    let in_machine = {
        let mut n = 0;
        for chunk in world.all_chunks() {
            for machine in world.chunk_snapshot(chunk).machines {
                for (item, qty) in &machine.input {
                    if item == "tomato" {
                        n += qty;
                    }
                }
            }
        }
        n
    };
    // 4 tomatoes are consumed by the single (effectively endless) craft.
    assert_eq!(
        held + on_belt + in_machine + 4,
        HOPPER_CAPACITY,
        "every loaded tomato is either still in the hopper, on the belt, buffered, or in the \
         one craft that started — none duplicated, none lost"
    );
}

// ---------------------------------------------------------------------
// 3. The wire shape ticket 4 consumes.
// ---------------------------------------------------------------------

#[test]
fn the_hopper_json_shape_is_what_presentation_is_told_to_expect() {
    let mut world = world();
    world.issue_command(PlayerCommand::FillHopper { amount: 16 });

    // hopper_snapshot_json (`SimBridge`) serializes exactly this.
    let snapshot: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.hopper_snapshot()).unwrap()).unwrap();
    assert_eq!(
        snapshot,
        serde_json::json!({
            "tile": [-1, 0],
            "kind": "hopper",
            "footprint": { "width_tiles": 1, "height_tiles": 1 },
            "item": "tomato",
            "quantity": 16,
            "capacity": 64,
            "fill": 0.25,
            "belt_tile": [0, 0],
        })
    );

    // chunk_snapshot_json carries the identical object under `hoppers`,
    // alongside the pre-existing belts/machines/pipes arrays.
    let chunk: serde_json::Value = serde_json::from_str(
        &serde_json::to_string(&world.chunk_snapshot((-1, 0))).unwrap(),
    )
    .unwrap();
    assert_eq!(chunk["hoppers"][0], snapshot);
    for existing in ["belts", "machines", "pipes"] {
        assert!(
            chunk[existing].is_array(),
            "the pre-existing `{existing}` array must be untouched"
        );
    }

    // query_tile_json answers with a `hopper` tile: enough for a hover
    // tooltip without a second call.
    let tile: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.query_tile((-1, 0)).unwrap()).unwrap())
            .unwrap();
    assert_eq!(
        tile,
        serde_json::json!({
            "kind": "hopper",
            "footprint": { "width_tiles": 1, "height_tiles": 1 },
            "origin_tile": [-1, 0],
            "item": "tomato",
            "quantity": 16,
            "capacity": 64,
            "fill": 0.25,
        })
    );

    // The hopper is not a recipe stage, so it is deliberately absent from
    // chain_layout_json — which stays one object per recipe, unchanged.
    let layout = world.chain_layout();
    assert_eq!(layout.len(), recipes().ordered().len());
    assert_eq!(layout[0].belt_start_tile, (0, 0));
}

#[test]
fn the_fill_command_parses_from_json_with_or_without_an_amount() {
    let mut world = world();
    world.issue_command(
        command_from_json(r#"{"type": "fill_hopper", "amount": 3}"#).expect("must parse"),
    );
    assert_eq!(world.hopper_snapshot().quantity, 3);

    // Omitting `amount` means the simulation's own default load, so the
    // common "player pressed the load key" case needn't restate a constant.
    world.issue_command(command_from_json(r#"{"type": "fill_hopper"}"#).expect("must parse"));
    assert_eq!(world.hopper_snapshot().quantity, 3 + HOPPER_FILL_AMOUNT);

    // The removed debug command is now simply unknown — an unparseable
    // command is a no-op, per `command_from_json`'s contract, so an
    // out-of-date caller can't silently teleport items onto a belt.
    assert!(command_from_json(r#"{"type": "spawn_item", "item": "tomato"}"#).is_none());
}
