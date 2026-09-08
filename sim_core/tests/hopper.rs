//! Headless integration test for the raw-ingredient **hopper** — the
//! building that buffers raw ingredient and meters it onto a belt, and (since
//! the chain-rework epic) the building the player has to say the *contents*
//! of when they place it.
//!
//! `sim_core/src/sim/hopper.rs` already unit-tests `Hopper` in isolation
//! (fill clamping, overflow safety, one-unit-per-call unloading, the
//! blocked-belt case). This file drives the real `World` instead, to prove
//! the five things only the whole system can show:
//!
//! 1. **Placement, and contents.** The reference chain places *two* hoppers,
//!    one per raw solid ingredient (`tomato` and `sugar`), each at the head
//!    of its own feed lane — one lane per solid input, into its own tile of
//!    the consuming machine's inlet face. Which item a hopper holds is a
//!    `PlayerCommand::Place` argument now: with one raw ingredient the
//!    simulation could infer it, with two it cannot, so omitting it is a
//!    refusal rather than a default.
//! 2. **Metering.** `World::tick` unloads at most one unit, only when the
//!    belt has room, so the *machinery* sets the feed rate and the player
//!    only sets the supply. A jammed line leaves the hopper holding.
//! 3. **Filling costs money.** A fill draws from the stock a
//!    `PlayerCommand::BuyIngredient` paid for (GAMEPLAY.md §5), and is a
//!    silent no-op with an empty larder. `ingredients.rs` covers the ledger
//!    itself; what is proved here is that the two clamps compose — owned
//!    stock first, then the bin's own capacity — and that the ledger is
//!    debited by what the bin actually took.
//! 4. **The wire shape.** The exact JSON field names presentation consumes
//!    verbatim, pinned the way `fluids.rs` and `machine_footprints.rs` pin
//!    theirs — a rename here fails a test instead of silently breaking the
//!    scene.
//! 5. **Retargeting an already-placed hopper.** `Place`'s own "you must name
//!    the contents with more than one raw item" rule (point 1) leaves a real
//!    gap once the hopper is down: nothing before `PlayerCommand::
//!    CycleHopperItem` let a player change a placed hopper's mind short of
//!    removing and re-placing it. This proves cycling actually advances
//!    through every raw item in order, wraps around, clears whatever the
//!    hopper held, and is a no-op on an empty tile, a non-hopper tile, or a
//!    world with only one raw item to begin with.
//!
//! Mass conservation across the whole chain *through* the hoppers is covered
//! in `core_loop.rs`, which counts hopper contents as mass in the system.

use sim_core::sim::{
    Direction, HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, HOPPER_KIND, HopperSnapshot, PlayerCommand,
    RecipeRegistry, World, command_from_json, reference_chain, reference_chain_with_budget,
};

/// Enough money for any stock this file buys, and no more than the shop
/// bill it sits on top of — see `reference_chain_with_budget`.
const BUDGET: u32 = 400;

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    reference_chain_with_budget(recipes(), BUDGET)
}

/// The hopper holding `item`. The reference chain places one per raw
/// ingredient, so a test says which it means rather than indexing an array.
fn hopper_for(world: &World, item: &str) -> HopperSnapshot {
    world
        .hopper_snapshots()
        .into_iter()
        .find(|h| h.item == item)
        .unwrap_or_else(|| panic!("the reference chain places a hopper holding '{item}'"))
}

fn tomato_hopper(world: &World) -> HopperSnapshot {
    hopper_for(world, "tomato")
}

/// Buys `quantity` of `item` and asserts the purchase actually landed —
/// otherwise a silently-unaffordable buy shows up much later as an
/// unexplained empty hopper.
fn buy(world: &mut World, item: &str, quantity: u32) {
    let before = world.owned_ingredient(item);
    world.issue_command(PlayerCommand::BuyIngredient {
        item: item.to_string(),
        quantity,
    });
    assert_eq!(
        world.owned_ingredient(item),
        before + quantity,
        "the fixture must be able to afford {quantity} x {item}"
    );
}

/// How many items are on every belt in the world right now. Since the "Real
/// Placement" epic a belt is one *tile*, so what used to be one four-tile
/// `BeltSegment` is four separate snapshots — an accounting test has to add
/// them all up.
fn all_belt_item_count(world: &World) -> u32 {
    world
        .all_chunks()
        .into_iter()
        .flat_map(|chunk| world.chunk_snapshot(chunk).belts)
        .map(|belt| belt.items.len() as u32)
        .sum()
}

/// How many items are on the belt tile a given hopper discharges onto.
fn fed_belt_item_count(world: &World, hopper: &HopperSnapshot) -> usize {
    let Some(start) = hopper.belt_tile else {
        return 0;
    };
    for chunk in world.all_chunks() {
        for belt in world.chunk_snapshot(chunk).belts {
            if belt.tile == start {
                return belt.items.len();
            }
        }
    }
    0
}

// ---------------------------------------------------------------------
// 1. Placement, and contents.
// ---------------------------------------------------------------------

/// One hopper per raw solid ingredient, each standing at the head of its own
/// feed lane and facing east along it.
#[test]
fn a_hopper_stands_at_the_head_of_its_own_feed_lane_one_per_raw_ingredient() {
    let world = world();
    let hoppers = world.hopper_snapshots();
    assert_eq!(
        hoppers.len(),
        2,
        "the shipped chain has two raw solid ingredients, so it has two hoppers"
    );
    assert_eq!(
        world.raw_items(),
        ["tomato".to_string(), "sugar".to_string()],
        "raw = consumed by some recipe, produced by none — read out of data/recipes.ron, \
         never named in Rust"
    );

    for hopper in &hoppers {
        let belt = hopper
            .belt_tile
            .expect("every reference-chain hopper discharges onto a belt");
        assert_eq!(
            hopper.tile,
            (belt.0 - hopper.footprint.width_tiles, belt.1),
            "a hopper's footprint ends on the tile immediately west of the lane it feeds"
        );
        assert_eq!(hopper.direction, Direction::East);
        assert_eq!(hopper.kind, HOPPER_KIND);
        assert_eq!(
            (hopper.footprint.width_tiles, hopper.footprint.height_tiles),
            (1, 1)
        );
        assert_eq!(hopper.capacity, HOPPER_CAPACITY);
        assert_eq!(hopper.quantity, 0, "a new hopper starts empty");
        assert_eq!(hopper.fill, 0.0);
    }

    // The shipped placements, written out rather than recomputed. Tomato is
    // the crusher's only input, so its lane is row 0 starting at x = 0 and
    // its hopper is at (-1, 0) — exactly where the pre-placement chain put
    // the one hopper it had. Sugar is the mixer's *second* solid input, so
    // it gets lane row 1 (starting at x = 5, level with the paste lane) and
    // its own hopper at the head of that.
    assert_eq!(tomato_hopper(&world).tile, (-1, 0));
    assert_eq!(hopper_for(&world, "sugar").tile, (4, 1));
}

/// The chain-rework epic's real change to `Place`: with two raw ingredients
/// the simulation no longer owns a sensible default for what a hopper holds,
/// so it has to be told — and told something that is genuinely raw.
#[test]
fn placing_a_hopper_requires_naming_which_raw_item_it_holds() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    let before = world.placement_count();
    world.issue_command(PlayerCommand::Purchase {
        kind: HOPPER_KIND.to_string(),
        quantity: 3,
    });

    // (a) No item named, and two raw items to choose from: a refusal, not a
    // guess. Nothing is placed and the owned unit is not spent.
    world.issue_command(PlayerCommand::Place {
        kind: HOPPER_KIND.to_string(),
        tile: (0, -5),
        direction: Direction::North,
        item: None,
    });
    assert_eq!(world.placement_count(), before);
    assert_eq!(world.owned_count(HOPPER_KIND), 3);

    // (b) An item that exists in the chain but is *made* by it, not raw.
    world.issue_command(PlayerCommand::Place {
        kind: HOPPER_KIND.to_string(),
        tile: (0, -5),
        direction: Direction::North,
        item: Some("bottled_ketchup".to_string()),
    });
    assert_eq!(world.placement_count(), before);

    // (c) An item no recipe mentions at all.
    world.issue_command(PlayerCommand::Place {
        kind: HOPPER_KIND.to_string(),
        tile: (0, -5),
        direction: Direction::North,
        item: Some("saffron".to_string()),
    });
    assert_eq!(world.placement_count(), before);

    // (d) A genuine raw item: placed, and holding what was asked for.
    world.issue_command(PlayerCommand::Place {
        kind: HOPPER_KIND.to_string(),
        tile: (0, -5),
        direction: Direction::North,
        item: Some("sugar".to_string()),
    });
    assert_eq!(world.placement_count(), before + 1);
    assert_eq!(world.owned_count(HOPPER_KIND), 2);
    assert_eq!(
        world
            .hopper_snapshots()
            .iter()
            .filter(|h| h.tile == (0, -5))
            .map(|h| h.item.clone())
            .collect::<Vec<_>>(),
        vec!["sugar".to_string()]
    );
}

/// The other half of that rule: in a world whose recipes name exactly one
/// raw ingredient, the simulation *does* own the answer, so omitting `item`
/// still works — which is why nothing about a single-ingredient world's wire
/// form changed.
#[test]
fn a_single_raw_ingredient_chain_still_infers_the_hoppers_contents() {
    let one_raw = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 4)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 60,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse");
    let world = reference_chain(one_raw);
    let hoppers = world.hopper_snapshots();
    assert_eq!(hoppers.len(), 1);
    assert_eq!(
        hoppers[0].item, "tomato",
        "with one raw item, `Place` infers it and the player is never asked"
    );
}

/// The shipped chain sits in chunk (0, 0); the first lane's hopper reaches
/// back to x = -1, in chunk (-1, 0). A renderer that only ever asked for the
/// line's own chunk would never draw it, so `all_chunks` has to report
/// both.
#[test]
fn the_hoppers_chunk_is_reported_alongside_the_chains() {
    let world = world();
    let chunks = world.all_chunks();
    assert!(chunks.contains(&(0, 0)), "{chunks:?}");
    assert!(chunks.contains(&(-1, 0)), "{chunks:?}");
    assert_eq!(
        world.chunk_snapshot((-1, 0)).hoppers.len(),
        1,
        "the tomato hopper is the one that reaches into the chunk west of the line"
    );
    assert_eq!(
        world.chunk_snapshot((0, 0)).hoppers.len(),
        1,
        "...and the sugar hopper stands inside the line's own chunk"
    );
}

// ---------------------------------------------------------------------
// 2. Filling: bought stock, then capacity.
// ---------------------------------------------------------------------

/// GAMEPLAY.md §5's ingredient cost axis, at world level: a fill takes what
/// the player owns and no more, and an empty larder is a silent no-op rather
/// than free material.
#[test]
fn a_fill_draws_from_bought_stock_and_does_nothing_without_any() {
    let mut world = world();

    world.issue_command(PlayerCommand::FillHopper {
        amount: 16,
        tile: Some((-1, 0)),
    });
    assert_eq!(
        tomato_hopper(&world).quantity,
        0,
        "an empty larder must fill nothing — ingredients are not free"
    );

    buy(&mut world, "tomato", 10);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 16,
        tile: Some((-1, 0)),
    });
    assert_eq!(
        tomato_hopper(&world).quantity,
        10,
        "the fill is clamped to what was actually bought"
    );
    assert_eq!(world.owned_ingredient("tomato"), 0, "and the stock is spent");
}

/// The second clamp, and the rule that makes the two compose without leaking:
/// the ledger is debited by what the bin **took**, never by what was asked
/// for.
#[test]
fn a_nearly_full_hopper_is_only_charged_for_what_it_accepted() {
    let mut world = world();
    buy(&mut world, "tomato", HOPPER_CAPACITY + 10);
    world.issue_command(PlayerCommand::FillHopper {
        amount: HOPPER_CAPACITY + 10,
        tile: Some((-1, 0)),
    });
    assert_eq!(tomato_hopper(&world).quantity, HOPPER_CAPACITY);
    assert_eq!(
        world.owned_ingredient("tomato"),
        10,
        "the ten units the full bin refused must still be in the larder"
    );

    // ...and again on an already-full hopper: still a silent no-op, nothing
    // spilled onto the belt and nothing taken off the ledger.
    world.issue_command(PlayerCommand::FillHopper {
        amount: u32::MAX,
        tile: Some((-1, 0)),
    });
    assert_eq!(tomato_hopper(&world).quantity, HOPPER_CAPACITY);
    assert_eq!(world.owned_ingredient("tomato"), 10);
    assert_eq!(
        fed_belt_item_count(&world, &tomato_hopper(&world)),
        0,
        "filling must never put anything on a belt — that is the hopper's job, on tick"
    );
}

/// A hopper knows its own item, so a fill command never names one — and
/// filling every hopper at once charges each of them against its own stock.
#[test]
fn filling_every_hopper_charges_each_against_its_own_ingredient() {
    let mut world = world();
    buy(&mut world, "tomato", 20);
    buy(&mut world, "sugar", 5);

    world.issue_command(PlayerCommand::FillHopper {
        amount: 8,
        tile: None,
    });

    assert_eq!(tomato_hopper(&world).quantity, 8);
    assert_eq!(hopper_for(&world, "sugar").quantity, 5, "clamped to the sugar owned");
    assert_eq!(world.owned_ingredient("tomato"), 12);
    assert_eq!(world.owned_ingredient("sugar"), 0);
}

#[test]
fn a_fill_command_dirties_the_hoppers_chunk_so_the_fill_ui_can_redraw() {
    let mut world = world();
    buy(&mut world, "tomato", 1);
    // One tick first, to clear the dirt from *building* the chain: placing
    // something dirties its chunks too, and the fixture places thirty-odd
    // things before this test starts.
    world.tick();
    world.issue_command(PlayerCommand::FillHopper {
        amount: 1,
        tile: Some((-1, 0)),
    });
    assert_eq!(world.dirty_chunks(), &[(-1, 0)]);
}

/// The core of the hopper's whole reason to exist: the belt is fed by the
/// simulation, one unit at a time, at the rate the belt can accept — not by
/// the command.
#[test]
fn the_hopper_unloads_exactly_one_unit_at_a_time_gated_by_the_belt() {
    let mut world = world();
    buy(&mut world, "tomato", 10);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 10,
        tile: Some((-1, 0)),
    });

    world.tick();
    assert_eq!(
        tomato_hopper(&world).quantity,
        9,
        "exactly one unit leaves the hopper on a tick, never the whole buffer"
    );
    assert_eq!(fed_belt_item_count(&world, &tomato_hopper(&world)), 1);

    // The very next tick, the belt's min-spacing rule (belt.rs) means there
    // is no room at its start yet — so the hopper must simply hold.
    world.tick();
    assert_eq!(
        tomato_hopper(&world).quantity,
        9,
        "a belt with no room at its start must not accept a second unit"
    );
    assert_eq!(fed_belt_item_count(&world, &tomato_hopper(&world)), 1);

    // Given enough ticks the whole load reaches the line.
    for _ in 0..2_000 {
        world.tick();
    }
    assert_eq!(tomato_hopper(&world).quantity, 0);
}

/// The stall case, at world level: nothing downstream ever consumes, so the
/// first belt fills up and the hopper has to keep the rest. This is the
/// same "gate on availability, consume on success" contract a `Machine` has
/// with its inputs — it holds, it never spills and never silently drops.
#[test]
fn a_jammed_belt_leaves_the_hopper_holding_its_contents() {
    // A one-stage chain whose machine never finishes, so the belt backs up
    // completely and stays that way.
    let jammed = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
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
    let mut world = reference_chain_with_budget(jammed, BUDGET);
    buy(&mut world, "tomato", HOPPER_CAPACITY);
    world.issue_command(PlayerCommand::FillHopper {
        amount: HOPPER_CAPACITY,
        tile: None,
    });

    for _ in 0..20_000 {
        world.tick();
    }

    let held = world.hopper_snapshots()[0].quantity;
    assert!(
        held > 0,
        "a backed-up line must leave raw ingredient in the hopper, not consume it into nothing"
    );
    let on_belt = all_belt_item_count(&world);
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
// 3. The wire shape presentation consumes.
// ---------------------------------------------------------------------

#[test]
fn the_hopper_json_shape_is_what_presentation_is_told_to_expect() {
    let mut world = world();
    buy(&mut world, "tomato", 16);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 16,
        tile: Some((-1, 0)),
    });

    // hopper_snapshots_json (`SimBridge`) serializes an array of exactly
    // these.
    let snapshot: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&tomato_hopper(&world)).unwrap()).unwrap();
    assert_eq!(
        snapshot,
        serde_json::json!({
            "tile": [-1, 0],
            "kind": "hopper",
            "direction": "east",
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
    let chunk: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.chunk_snapshot((-1, 0))).unwrap())
            .unwrap();
    assert!(
        chunk["hoppers"]
            .as_array()
            .unwrap()
            .contains(&snapshot),
        "the hopper's chunk snapshot must carry the identical object"
    );
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
            "direction": "east",
            "footprint": { "width_tiles": 1, "height_tiles": 1 },
            "origin_tile": [-1, 0],
            "item": "tomato",
            "quantity": 16,
            "capacity": 64,
            "fill": 0.25,
        })
    );

    // world_layout_json (which replaced chain_layout_json) reports the
    // hopper like any other placement — it is a placed building, not a
    // recipe stage, and the new layout call makes no distinction.
    let layout = world.world_layout();
    let entry = layout
        .iter()
        .find(|p| p.kind == "hopper")
        .expect("the hopper is a placement like any other");
    assert_eq!(entry.tile, (-1, 0));
    assert_eq!(entry.direction, Direction::East);
    assert_eq!(entry.occupies, vec![(-1, 0)]);
    assert!(
        entry.recipe.is_none(),
        "a hopper buffers, it does not run a recipe"
    );
}

#[test]
fn the_fill_command_parses_from_json_with_or_without_an_amount() {
    let mut world = world();
    buy(&mut world, "tomato", 64);
    world.issue_command(
        command_from_json(r#"{"type": "fill_hopper", "amount": 3, "tile": [-1, 0]}"#)
            .expect("must parse"),
    );
    assert_eq!(tomato_hopper(&world).quantity, 3);

    // Omitting `amount` means the simulation's own default load, so the
    // common "player pressed the load key" case needn't restate a constant.
    world.issue_command(
        command_from_json(r#"{"type": "fill_hopper", "tile": [-1, 0]}"#).expect("must parse"),
    );
    assert_eq!(tomato_hopper(&world).quantity, 3 + HOPPER_FILL_AMOUNT);

    // The removed debug command is now simply unknown — an unparseable
    // command is a no-op, per `command_from_json`'s contract, so an
    // out-of-date caller can't silently teleport items onto a belt.
    assert!(command_from_json(r#"{"type": "spawn_item", "item": "tomato"}"#).is_none());
}

/// `place`'s new optional `item` on the wire, and the fact that omitting it
/// still parses (it is a *refusal*, decided by the simulation, not a parse
/// error).
#[test]
fn the_place_command_carries_an_optional_item_on_the_wire() {
    let with_item = command_from_json(
        r#"{"type": "place", "kind": "hopper", "tile": [3, -3], "direction": "north",
            "item": "sugar"}"#,
    )
    .expect("must parse");
    match with_item {
        PlayerCommand::Place {
            ref kind,
            tile,
            direction,
            ref item,
        } => {
            assert_eq!(kind, HOPPER_KIND);
            assert_eq!(tile, (3, -3));
            assert_eq!(direction, Direction::North);
            assert_eq!(item.as_deref(), Some("sugar"));
        }
        other => panic!("expected a place command, got {other:?}"),
    }

    let without_item =
        command_from_json(r#"{"type": "place", "kind": "belt", "tile": [3, 0]}"#)
            .expect("must parse");
    match without_item {
        PlayerCommand::Place {
            direction, item, ..
        } => {
            assert_eq!(direction, Direction::East, "the default facing is unchanged");
            assert!(item.is_none());
        }
        other => panic!("expected a place command, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 5. Retargeting an already-placed hopper.
// ---------------------------------------------------------------------

/// The whole point: a hopper placed as one raw item can become another
/// without removing and re-placing it, and its old contents do not survive
/// the switch — a hopper is single-item, so there is no "half tomato, half
/// sugar" state to preserve.
#[test]
fn cycling_switches_the_item_and_clears_whatever_it_held() {
    let mut world = world();
    buy(&mut world, "tomato", 20);

    let before = tomato_hopper(&world);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 10,
        tile: Some(before.tile),
    });
    assert!(
        hopper_for(&world, "tomato").quantity > 0,
        "the fixture must actually have filled the hopper before cycling it away"
    );

    world.issue_command(PlayerCommand::CycleHopperItem { tile: before.tile });

    let after = world
        .hopper_snapshots()
        .into_iter()
        .find(|h| h.tile == before.tile)
        .expect("the hopper is still there — cycling changes its item, not its existence");
    assert_ne!(after.item, "tomato", "cycling must have changed the item");
    assert_eq!(
        after.quantity, 0,
        "the old contents must not survive a switch to a different item"
    );
}

/// Cycling through every raw item returns to where it started — a hopper
/// with N raw items to choose from has exactly N states, arranged in a ring.
#[test]
fn cycling_all_the_way_around_returns_to_the_original_item() {
    let mut world = world();
    let raw_items = world.raw_items().to_vec();
    assert!(
        raw_items.len() >= 2,
        "this test needs at least two raw items to prove a cycle, not just a no-op"
    );

    let tile = tomato_hopper(&world).tile;
    let original = hopper_for(&world, "tomato").item.clone();
    for _ in 0..raw_items.len() {
        world.issue_command(PlayerCommand::CycleHopperItem { tile });
    }
    let final_item = world
        .hopper_snapshots()
        .into_iter()
        .find(|h| h.tile == tile)
        .unwrap()
        .item;
    assert_eq!(
        final_item, original,
        "cycling through every raw item exactly once must return to the start"
    );
}

/// No hopper, no cycle — the same silent-no-op convention every other
/// impossible command in this codebase already follows.
#[test]
fn cycling_an_empty_or_non_hopper_tile_is_a_no_op() {
    let mut world = world();
    let empty_tile = (500, 500);
    world.issue_command(PlayerCommand::CycleHopperItem { tile: empty_tile });
    assert!(
        world.hopper_snapshots().iter().all(|h| h.tile != empty_tile),
        "cycling an empty tile must not create a hopper out of nothing"
    );

    // A machine tile (not a hopper) must refuse just as quietly — reuse the
    // crusher's own tile, which the reference chain always places.
    let crusher_tile = world
        .all_chunks()
        .into_iter()
        .flat_map(|chunk| world.chunk_snapshot(chunk).machines)
        .find(|m| m.kind == "crusher")
        .expect("the reference chain places a crusher")
        .tile;
    let before = hopper_for(&world, "tomato").item.clone();
    world.issue_command(PlayerCommand::CycleHopperItem {
        tile: crusher_tile,
    });
    assert_eq!(
        hopper_for(&world, "tomato").item,
        before,
        "cycling a machine tile must not touch any hopper"
    );
}

/// A world with only one raw item has nothing to cycle *to* — cycling must
/// leave it exactly as it was rather than "advancing" to itself and quietly
/// discarding its contents, which would be a real, player-visible loss for
/// zero benefit.
#[test]
fn cycling_with_only_one_raw_item_is_a_no_op_that_keeps_contents() {
    let mut world = reference_chain(recipes());
    if world.raw_items().len() != 1 {
        // This fixture only exercises the single-raw-item case; the
        // multi-item cases above already cover today's shipped content.
        return;
    }
    let hopper = world.hopper_snapshots().into_iter().next().unwrap();
    world.issue_command(PlayerCommand::FillHopper {
        amount: 5,
        tile: Some(hopper.tile),
    });
    let before = world.hopper_snapshots().into_iter().next().unwrap();
    world.issue_command(PlayerCommand::CycleHopperItem { tile: before.tile });
    let after = world.hopper_snapshots().into_iter().next().unwrap();
    assert_eq!(after.item, before.item);
    assert_eq!(
        after.quantity, before.quantity,
        "a no-op cycle must not clear contents"
    );
}

#[test]
fn the_cycle_hopper_item_command_parses_from_json() {
    let parsed = command_from_json(r#"{"type": "cycle_hopper_item", "tile": [-1, 0]}"#)
        .expect("must parse");
    match parsed {
        PlayerCommand::CycleHopperItem { tile } => assert_eq!(tile, (-1, 0)),
        other => panic!("expected a cycle_hopper_item command, got {other:?}"),
    }
}
