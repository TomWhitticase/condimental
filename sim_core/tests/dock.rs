//! Headless integration test for the delivery-dock ticket: the chain is
//! emptied by a **dock** — a real building at the end of a real belt, which
//! ships finished product into the economy's shared delivery pool by itself,
//! every tick — instead of by `PlayerCommand::CollectOutput`, a command that
//! reached into the last machine's own output buffer and no longer exists.
//!
//! This is `hopper.rs`'s file, mirrored to the other end of the line, and it
//! is organised the same way.
//!
//! `sim_core/src/sim/dock.rs` already unit-tests `Dock` in isolation (the
//! empty-belt case, the not-yet-arrived case, one-unit-per-call absorbing,
//! the full-dock refusal, and shipping). This file drives the real `World`
//! instead, to prove the four things only the whole system can show:
//!
//! 1. **Geometry.** There is now real tile geometry downstream of the last
//!    machine — one ordinary belt and one building — where until this ticket
//!    there was nothing at all. It is placed by *accumulating* past the last
//!    stage, exactly the way every stage is placed, and it is what pushes
//!    the shipped chain across the x = 32 chunk boundary.
//! 2. **Automatic delivery.** Money is earned with **zero** player-issued
//!    commands beyond supplying the two consumables. That is the whole
//!    point of the ticket, so it is asserted directly rather than implied.
//! 3. **The pool, not the dock, is where product waits.** The dock is a
//!    pass-through (`sim/dock.rs`'s module doc), so its own buffer reads
//!    empty between ticks while `delivery_stock` climbs — the property a
//!    presentation pass has to know before it draws a fill level on the
//!    wrong thing.
//! 4. **The wire shape.** The exact JSON field names ticket 2 (the Godot
//!    dock asset + hover) consumes verbatim, pinned the way `fluids.rs`,
//!    `hopper.rs` and `machine_footprints.rs` pin theirs.
//!
//! Mass conservation across the whole chain *through* the dock is covered in
//! `core_loop.rs`, which counts the dock's buffer as mass in the system and
//! reads "shipped" off the dock rather than tallying a command.

mod common;

use sim_core::sim::{
    DOCK_CAPACITY, DockSnapshot, Direction, RecipeRegistry, TileSnapshot, World,
    reference_chain_with_budget,
};

/// Exactly `common::stock_up`'s ingredient bill, so a stocked world starts
/// with a factory, a full larder and zero money.
/// Stock plus a wage reserve: these fixtures run for many paydays.
const BUDGET: u32 = common::STOCK_COST + common::WAGE_RESERVE;

/// What the factory has earned: the balance less what it started with
/// after stocking up, with the wages it has paid since added back.
fn earned(world: &World) -> sim_core::sim::types::Money {
    world.money() + common::wages_paid(world)
        - sim_core::sim::types::Money::from_num(common::WAGE_RESERVE)
}

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    common::stock_up(&mut world);
    world
}

/// The world's one dock. The reference chain places exactly one; several are
/// a real supported thing since the "Real Placement" epic (see
/// `placement.rs`), just not what this file is about.
fn the_dock(world: &World) -> DockSnapshot {
    let mut docks = world.dock_snapshots();
    assert_eq!(docks.len(), 1, "the reference chain places one dock");
    docks.remove(0)
}

/// Runs the chain with the two consumables kept topped up and **no other
/// command at all**, until the dock has shipped `target` units or the budget
/// runs out. Returns the dock's shipped total.
fn run_until_shipped(world: &mut World, target: u32) -> u32 {
    for _ in 0..500_000u32 {
        common::supply(world);
        world.tick();
        if the_dock(&world).shipped_total >= target {
            break;
        }
    }
    the_dock(&world).shipped_total
}

// ---------------------------------------------------------------------
// 1. Geometry.
// ---------------------------------------------------------------------

/// The dock's belt starts on the tile immediately after the last stage's
/// machine, and the dock stands immediately after that belt — the same
/// accumulate-past-what-came-before rule every stage follows
/// (`data/machines.ron`'s header). Written out as literal coordinates for
/// `machine_footprints.rs`'s reason: a test that recomputed them the way the
/// code does could not catch the code being wrong.
#[test]
fn the_dock_stands_at_the_end_of_its_own_belt_past_the_last_machine() {
    let world = world();
    let layout = world.world_layout();

    // The shipped chain's last machine is the packer at x = 30, 1x1.
    let last_machine = layout
        .iter()
        .filter(|p| p.recipe.is_some())
        .max_by_key(|p| p.tile.0)
        .expect("the reference chain places machines");
    assert_eq!(last_machine.kind, "packer");
    assert_eq!(last_machine.tile, (21, 0));

    let dock = the_dock(&world);
    assert_eq!(dock.kind, "dock");
    assert_eq!(dock.direction, Direction::East);
    // Four belt tiles run from the packer to the dock — placed one at a
    // time now, where the pre-placement chain built one four-tile segment.
    for x in 22..=25 {
        assert!(
            layout
                .iter()
                .any(|p| p.kind == "belt" && p.tile == (x, 0)),
            "a belt tile must stand at ({x}, 0)"
        );
    }
    assert_eq!(
        dock.belt_tile,
        Some((25, 0)),
        "the dock is fed by the last belt tile of that run"
    );
    assert_eq!(
        dock.tile,
        (26, 0),
        "and the dock stands immediately east of that belt's last tile"
    );
    // 2x2 since the 2026-09-09 tuning pass (`data/machines.ron`'s dock note
    // has the reasoning), so the building claims (26..27, 0..1) — the belt
    // still meets it at the origin tile, which is the west edge's first
    // tile and therefore still on its inlet face.
    assert_eq!(dock.footprint.width_tiles, 2);
    assert_eq!(dock.footprint.height_tiles, 2);
    assert_eq!(dock.capacity_per_item, DOCK_CAPACITY);
}

/// Every tile of the new geometry resolves, and resolves as the right thing
/// — the belt as a belt, the dock as a dock. Until this ticket every one of
/// these tiles was empty.
#[test]
fn every_tile_of_the_dock_and_its_belt_resolves() {
    let world = world();
    let dock = the_dock(&world);

    for x in 22..=25 {
        let tile = (x, 0);
        match world.query_tile(tile) {
            Some(TileSnapshot::Belt { .. }) => {}
            other => panic!("expected the dock's belt at {tile:?}, got {other:?}"),
        }
    }

    match world.query_tile(dock.tile) {
        Some(TileSnapshot::Dock {
            origin_tile,
            contents,
            capacity_per_item,
            shipped_total,
            ..
        }) => {
            assert_eq!(origin_tile, dock.tile);
            assert!(contents.is_empty(), "a fresh dock holds nothing");
            assert_eq!(capacity_per_item, DOCK_CAPACITY);
            assert_eq!(shipped_total, 0);
        }
        other => panic!("expected the dock at {:?}, got {other:?}", dock.tile),
    }

    // Every tile of the 2x2 footprint resolves to the SAME dock, at the same
    // origin — the multi-tile rule `machine_footprints.rs` pins for machines,
    // now exercised by a shipped building as well.
    for tile in [(27, 0), (26, 1), (27, 1)] {
        match world.query_tile(tile) {
            Some(TileSnapshot::Dock { origin_tile, .. }) => {
                assert_eq!(origin_tile, dock.tile, "{tile:?} must resolve to the dock's origin");
            }
            other => panic!("expected the dock at {tile:?}, got {other:?}"),
        }
    }
    assert!(
        world.query_tile((28, 0)).is_none(),
        "and nothing exists past the dock"
    );
}

/// The dock, its belt and the hoppers all have to be reachable from
/// `all_chunks`, or a renderer that only asked for the line's own chunk
/// would never draw one end of it. The shipped line is shorter than it was
/// before the chain rework (the aging rack is gone and two belt runs became
/// short pipe runs), so it no longer crosses ARCHITECTURE.md §3.1's x = 32
/// boundary — the chunk fan-out itself is exercised deliberately, on
/// synthetic wide machines, in `machine_footprints.rs`.
#[test]
fn the_docks_chunk_is_reported_alongside_the_chains() {
    let world = world();
    let chunks = world.all_chunks();
    assert!(chunks.contains(&(0, 0)), "{chunks:?}");
    assert!(chunks.contains(&(-1, 0)), "the hoppers' chunk: {chunks:?}");

    let dock = the_dock(&world);
    let dock_chunk = (dock.tile.0.div_euclid(32), dock.tile.1.div_euclid(32));
    assert!(chunks.contains(&dock_chunk), "the dock's chunk: {chunks:?}");
    assert_eq!(
        world.chunk_snapshot(dock_chunk).docks.len(),
        1,
        "the dock must be in the snapshot of the chunk it occupies"
    );
    assert!(
        world.chunk_snapshot((-1, 0)).docks.is_empty(),
        "and only in that chunk"
    );
    assert_eq!(
        world
            .chunk_snapshot((0, 0))
            .belts
            .iter()
            .filter(|b| b.tile == (22, 0))
            .count(),
        1,
        "the belt tiles feeding the dock are reported in the chunk holding them"
    );
}

// ---------------------------------------------------------------------
// 2. Automatic delivery — the point of the ticket.
// ---------------------------------------------------------------------

/// The headline claim: keep the line supplied and money arrives, with no
/// collect command anywhere (there is no such command any more). Everything
/// between the packer and the contract payout happens because buildings did
/// it.
#[test]
fn the_chain_ships_and_earns_with_no_player_command_but_refilling() {
    let mut world = world();
    let final_item = recipes().ordered().last().unwrap().outputs[0].0.clone();

    // Two boxes is the shipped starter contract's whole mix, so reaching it
    // means the loop `GAMEPLAY.md` §1 describes has closed by itself.
    let shipped = run_until_shipped(&mut world, 2);
    assert!(
        shipped >= 2,
        "the dock must ship on its own; got {shipped} in the tick budget"
    );

    let stock: u32 = world
        .economy_snapshot()
        .delivery_stock
        .iter()
        .filter(|(item, _)| *item == final_item)
        .map(|(_, qty)| *qty)
        .sum();
    assert!(
        stock > 0 || earned(&world) > sim_core::sim::types::Money::ZERO,
        "every shipped box must be either waiting in the shared pool or already spent \
         paying a contract — never nowhere"
    );

    // ...and run on long enough for a contract window to actually settle.
    let longest = world
        .economy()
        .contracts
        .iter()
        .map(|c| c.contract.cadence_ticks)
        .max()
        .unwrap();
    let until = world.current_tick() + longest;
    while world.current_tick() < until {
        common::supply(&mut world);
        world.tick();
    }

    assert!(
        earned(&world) > sim_core::sim::types::Money::ZERO,
        "an unattended-but-supplied factory must earn money now that the dock delivers \
         for the player"
    );
    assert!(
        world
            .economy()
            .contracts
            .iter()
            .any(|c| c.deliveries_made > 0),
        "at least one contract must have settled against automatically-delivered stock"
    );
}

/// A box goes onto the dock's belt, travels it, and reaches the dock — it is
/// not teleported past the geometry. Checked by watching the belt actually
/// carry something, which is the observable difference between a real exit
/// and the old command.
#[test]
fn finished_product_actually_travels_the_dock_belt() {
    let mut world = world();
    let belt_start = the_dock(&world)
        .belt_tile
        .expect("the dock is fed by a belt");
    // Located by chunk arithmetic rather than by a literal, so this test
    // keeps working whichever side of a chunk boundary the line's tail
    // happens to fall on as content changes.
    let belt_chunk = (belt_start.0.div_euclid(32), belt_start.1.div_euclid(32));
    let mut saw_item_on_the_dock_belt = false;

    for _ in 0..500_000u32 {
        common::supply(&mut world);
        world.tick();

        let on_belt = world
            .chunk_snapshot(belt_chunk)
            .belts
            .iter()
            .find(|b| b.tile == belt_start)
            .map(|b| b.items.len())
            .unwrap_or(0);
        if on_belt > 0 {
            saw_item_on_the_dock_belt = true;
        }
        if saw_item_on_the_dock_belt && the_dock(&world).shipped_total > 0 {
            break;
        }
    }

    assert!(
        saw_item_on_the_dock_belt,
        "a finished box must be visible travelling the dock's belt — if it never is, \
         product is still skipping the geometry the way CollectOutput did"
    );
    assert!(the_dock(&world).shipped_total > 0);
}

// ---------------------------------------------------------------------
// 3. The pool, not the dock, is where product waits.
// ---------------------------------------------------------------------

/// The drain decision, observed through the real world: the dock's own
/// buffer is empty at every tick boundary while the shared delivery pool
/// fills up. A presentation pass that drew a fill level from the dock would
/// draw a permanently-empty building; `delivery_stock` is the number to use.
#[test]
fn the_dock_is_empty_between_ticks_while_the_shared_pool_fills() {
    let mut world = world();

    for _ in 0..500_000u32 {
        common::supply(&mut world);
        world.tick();

        assert!(
            the_dock(&world).contents.is_empty(),
            "the dock must hold nothing at a tick boundary — it ships everything it \
             receives in the same tick it receives it"
        );

        if the_dock(&world).shipped_total >= 2 {
            break;
        }
    }

    assert!(the_dock(&world).shipped_total >= 2);
    let pooled: u32 = world
        .economy_snapshot()
        .delivery_stock
        .iter()
        .map(|(_, qty)| *qty)
        .sum();
    assert!(
        pooled > 0,
        "what left the dock has to be somewhere, and the shared pool is where"
    );
}

// ---------------------------------------------------------------------
// 4. The wire shape ticket 2 consumes.
// ---------------------------------------------------------------------

/// Pins the exact JSON field names ticket 2 (the Godot dock asset, its hover
/// text, and the control-legend rework) will read. A rename here should fail
/// this test, not silently break the scene.
#[test]
fn the_dock_json_shape_is_what_presentation_is_told_to_expect() {
    let world = world();

    // dock_snapshots_json serializes an array of exactly these.
    let dock: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&the_dock(&world)).unwrap()).unwrap();
    assert_eq!(dock["tile"], serde_json::json!([26, 0]));
    assert_eq!(dock["kind"], "dock");
    assert_eq!(
        dock["footprint"],
        serde_json::json!({ "width_tiles": 2, "height_tiles": 2 })
    );
    assert_eq!(dock["direction"], "east");
    assert_eq!(dock["belt_tile"], serde_json::json!([25, 0]));
    assert!(
        dock.get("belt_start_tile").is_none() && dock.get("belt_length_tiles").is_none(),
        "a dock does not own a belt any more — it reports which belt tile feeds it: {dock}"
    );
    assert_eq!(dock["contents"], serde_json::json!([]));
    assert_eq!(dock["capacity_per_item"], DOCK_CAPACITY);
    assert_eq!(dock["shipped_total"], 0);
    assert!(
        dock.get("fill").is_none(),
        "a dock has no fill level on purpose — presentation must read \
         economy_snapshot_json's delivery_stock instead: {dock}"
    );

    // chunk_snapshot_json's new `docks` array, carrying the identical object.
    let chunk: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.chunk_snapshot((0, 0))).unwrap())
            .unwrap();
    let docks = chunk["docks"].as_array().expect("a `docks` array");
    assert_eq!(docks.len(), 1);
    assert_eq!(docks[0], dock, "same struct, same shape, both places");

    // query_tile_json's new `dock` variant.
    let tile: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.query_tile((26, 0)).unwrap()).unwrap())
            .unwrap();
    assert_eq!(tile["kind"], "dock");
    assert_eq!(tile["direction"], "east");
    assert_eq!(tile["origin_tile"], serde_json::json!([26, 0]));
    assert_eq!(tile["contents"], serde_json::json!([]));
    assert_eq!(tile["capacity_per_item"], DOCK_CAPACITY);
    assert_eq!(tile["shipped_total"], 0);
}
