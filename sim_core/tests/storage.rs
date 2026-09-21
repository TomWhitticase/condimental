//! Headless integration test for **warehouse storage capacity** — the rule
//! that a raw ingredient can only be held in as much room as the player has
//! built for it, and the `warehouse_rack` that supplies the room.
//!
//! `sim_core/src/sim/storage.rs` already unit-tests the arithmetic in
//! isolation (the loader, `bays_for`, `cap_for`, the shared pool, the
//! saturating edges) against a hand-built ledger with no `World` in sight.
//! This file drives the real `World`, to prove the things only the whole
//! system shows:
//!
//! 1. **The clamp is real, and it charges for what it credits.** With no
//!    rack anywhere, a 100-tomato order lands 20 tomatoes and costs 20
//!    tomatoes' worth — never the asked-for 100.
//! 2. **Building raises the ceiling**, by exactly the bays the rack supplies.
//! 3. **Bays are a shared pool, and a bay frees itself.** Fill the warehouse
//!    with tomato and sugar stops above `base_cap`; eat the tomato and sugar
//!    flows again with nothing rebuilt — which is the whole reason bays are
//!    derived from the ledger rather than assigned and stored.
//! 4. **The base-capacity invariant** (`cap_for(id) >= count(id)`, always)
//!    survives a scripted sequence of buys, consumption and construction.
//!    Nothing can be locked out of the room it is guaranteed.
//! 5. **A removal that would strand stock is refused** — the first `Remove`
//!    in the game that can fail — and the rack is still standing and still
//!    not back in the player's inventory afterwards.
//! 6. **A warehouse survives a save.** The racks come back as placements and
//!    the over-base stock they hold comes back with them.
//! 7. **Auto-fill is untouched.** A hopper tops itself up out of held stock
//!    exactly as it did, with racks and without them: capacity governs the
//!    *larder*, not the buildings drawing on it.
//! 8. **R6: a rack is pure storage.** Stood beside a running belt for
//!    thousands of ticks it never puts anything on it, never takes anything
//!    off it, and appears in no hand-off link. The rack's absence from the
//!    tick is the design, so it is asserted rather than assumed.

mod common;

use sim_core::sim::save::SaveData;
use sim_core::sim::types::{Direction, Money};
use sim_core::sim::world::{LinkKind, TileSnapshot};
use sim_core::sim::{
    HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, WAREHOUSE_RACK_KIND, World,
    reference_chain_with_budget,
};

/// Enough for every rack and every order in this file, and for the crew to
/// stay paid through the longest loop in it.
const BUDGET: u32 = 4_000 + common::WAGE_RESERVE;

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    reference_chain_with_budget(recipes(), BUDGET)
}

/// `data/storage.ron`'s numbers, read through the world rather than copied
/// into this file — a balance edit must not need a test edit.
fn knobs(world: &World) -> (u32, u32, u32) {
    let storage = world.storage();
    (
        storage.base_cap(),
        storage.slot_stack(),
        storage.bays_of_kind(WAREHOUSE_RACK_KIND),
    )
}

/// Buys and places one rack at `tile`, asserting it went down. The rack row
/// is south of everything `reference_chain` lays out.
fn build_rack(world: &mut World, tile: (i32, i32)) {
    let before = world.placement_count();
    world.issue_command(PlayerCommand::Purchase {
        kind: WAREHOUSE_RACK_KIND.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: WAREHOUSE_RACK_KIND.to_string(),
        tile,
        direction: Direction::East,
        item: None,
    });
    assert_eq!(
        world.placement_count(),
        before + 1,
        "a rack must be buildable at {tile:?}"
    );
}

/// Orders `quantity` without caring whether it all lands — the clamp is the
/// thing under test in most of this file.
fn order(world: &mut World, item: &str, quantity: u32) {
    world.issue_command(PlayerCommand::BuyIngredient {
        item: item.to_string(),
        quantity,
    });
}

// =====================================================================
// 1. The clamp
// =====================================================================

/// An order bigger than the room is not refused outright — it is *trimmed*,
/// and the trimmed quantity is what the player pays for. Charging for the
/// asked-for 100 while crediting 20 would be theft; refusing the whole order
/// would make a nearly-full larder unusable.
#[test]
fn with_no_warehouse_an_order_is_trimmed_to_the_base_cap_and_charged_for_what_landed() {
    let mut world = world();
    let (base_cap, _, _) = knobs(&world);
    assert_eq!(
        world.storage_bays(),
        0,
        "the reference chain builds no warehouse"
    );
    assert_eq!(world.stock_cap_for("tomato"), base_cap);

    let before = world.money();
    order(&mut world, "tomato", 100);

    assert_eq!(world.owned_ingredient("tomato"), base_cap);
    let price = world.ingredients_snapshot().entries[0].price;
    assert_eq!(
        before - world.money(),
        Money::from_num(f64::from(base_cap) * price),
        "the ledger is charged for the {base_cap} that landed, never for the 100 asked for"
    );
}

/// A larder already at its ceiling takes a further order as a no-op — no
/// stock, and no money.
#[test]
fn an_order_into_a_full_larder_costs_nothing() {
    let mut world = world();
    order(&mut world, "tomato", 1_000);
    let held = world.owned_ingredient("tomato");
    let money = world.money();

    order(&mut world, "tomato", 1_000);

    assert_eq!(world.owned_ingredient("tomato"), held);
    assert_eq!(world.money(), money, "a clamp to zero spends nothing");
}

// =====================================================================
// 2. Building raises the ceiling
// =====================================================================

#[test]
fn a_rack_raises_every_ceiling_by_the_bays_it_supplies() {
    let mut world = world();
    let (base_cap, slot_stack, bays) = knobs(&world);

    build_rack(&mut world, (0, -8));

    assert_eq!(world.storage_bays(), bays);
    let raised = base_cap + slot_stack * bays;
    assert_eq!(world.stock_cap_for("tomato"), raised);
    assert_eq!(
        world.stock_cap_for("sugar"),
        raised,
        "an empty warehouse raises every id's ceiling at once — the bays are not \
         spoken for until something is standing in them"
    );

    order(&mut world, "tomato", raised);
    assert_eq!(world.owned_ingredient("tomato"), raised);
}

// =====================================================================
// 3. A shared pool, and a bay that frees itself
// =====================================================================

/// The consequence the player has to feel: a warehouse is a finite room.
/// Filling it with one thing does stop another going in — and consuming the
/// first makes room again with nothing rebuilt, because a bay is derived
/// from the ledger rather than assigned to an id and kept.
#[test]
fn free_bays_are_shared_and_a_bay_frees_itself_when_the_stock_in_it_is_used() {
    let mut world = world();
    let (base_cap, slot_stack, _) = knobs(&world);
    build_rack(&mut world, (0, -8));

    // Fill the warehouse with tomato, to the last bay.
    let full = world.stock_cap_for("tomato");
    order(&mut world, "tomato", full);
    assert_eq!(world.owned_ingredient("tomato"), full);
    assert_eq!(
        world.stock_cap_for("sugar"),
        base_cap,
        "with every bay holding tomato, sugar is down to the base every id is guaranteed"
    );

    order(&mut world, "sugar", 200);
    assert_eq!(
        world.owned_ingredient("sugar"),
        base_cap,
        "sugar is trimmed to the base — the bays are taken, not gone"
    );

    // Spend a bay's worth of tomato into a hopper. Nothing is built, nothing
    // is reconfigured; the bay is simply no longer needed.
    let mut drawn = 0;
    while drawn < slot_stack {
        world.issue_command(PlayerCommand::FillHopper {
            amount: HOPPER_CAPACITY,
            tile: None,
        });
        for _ in 0..HOPPER_CAPACITY {
            world.tick();
        }
        drawn = full - world.owned_ingredient("tomato");
    }

    assert!(
        world.stock_cap_for("sugar") > base_cap,
        "the bay the eaten tomato stood in is available to sugar immediately"
    );
    // Not `base_cap + room`: `FillHopper { tile: None }` tops up *every*
    // hopper, and the chain has a sugar one, so the sugar ledger moved too.
    let held = world.owned_ingredient("sugar");
    let room = world.stock_room_for("sugar");
    order(&mut world, "sugar", room);
    assert_eq!(world.owned_ingredient("sugar"), held + room);
}

// =====================================================================
// 4. The base-capacity invariant
// =====================================================================

/// `cap_for(id) >= count(id)`, for every id, after every step — the
/// guarantee that makes a shared pool safe. It cannot be got at by
/// over-buying (the clamp), by building (that only adds), or by removing
/// (that is refused when it would strand).
#[test]
fn a_holding_is_never_over_its_own_ceiling_however_the_warehouse_changes() {
    let mut world = world();
    let ids = ["tomato", "sugar", "vinegar"];
    let check = |world: &World, step: &str| {
        for id in ids {
            assert!(
                world.stock_cap_for(id) >= world.owned_ingredient(id),
                "after {step}, {id} is held at {} over a ceiling of {}",
                world.owned_ingredient(id),
                world.stock_cap_for(id)
            );
        }
    };

    check(&world, "construction");
    for id in ids {
        order(&mut world, id, 400);
    }
    check(&world, "buying into no warehouse");

    build_rack(&mut world, (0, -8));
    check(&world, "the first rack");
    build_rack(&mut world, (1, -8));
    check(&world, "the second rack");

    for id in ids {
        order(&mut world, id, 400);
    }
    check(&world, "buying into two racks");

    world.issue_command(PlayerCommand::Remove { tile: (1, -8) });
    check(&world, "a removal");

    for _ in 0..2_000 {
        common::supply(&mut world);
        world.tick();
    }
    check(&world, "2000 ticks of production");
}

// =====================================================================
// 5. Removal
// =====================================================================

/// The first removal in the game that can be refused. Taking the floor out
/// from under stock would either destroy it silently or leave a holding over
/// its own ceiling, so the rack simply does not come up while it is needed.
#[test]
fn a_rack_holding_stock_up_is_refused_and_stays_standing() {
    let mut world = world();
    let (base_cap, slot_stack, bays) = knobs(&world);
    build_rack(&mut world, (0, -8));
    build_rack(&mut world, (1, -8));

    // Exactly one rack's worth of bays in use, with two racks standing.
    order(&mut world, "tomato", base_cap + slot_stack * bays);
    assert_eq!(
        world.stock_cap_for("sugar"),
        base_cap + slot_stack * bays,
        "one rack's bays are spoken for and the other's are not"
    );

    // The spare rack comes up: its bays are not the ones holding anything.
    let placements = world.placement_count();
    let owned_racks = world.owned_count(WAREHOUSE_RACK_KIND);
    world.issue_command(PlayerCommand::Remove { tile: (1, -8) });
    assert_eq!(
        world.placement_count(),
        placements - 1,
        "a rack whose bays are free must come up like anything else"
    );
    assert_eq!(world.owned_count(WAREHOUSE_RACK_KIND), owned_racks + 1);

    // The one that is actually holding the stock does not.
    world.issue_command(PlayerCommand::Remove { tile: (0, -8) });
    assert_eq!(
        world.placement_count(),
        placements - 1,
        "a rack whose bays are in use must still be standing"
    );
    assert_eq!(
        world.owned_count(WAREHOUSE_RACK_KIND),
        owned_racks + 1,
        "a refused removal must not put the rack back in the inventory either — \
         a refusal is a no-op, not a half-move"
    );
    assert_eq!(world.storage_bays(), bays);
    assert_eq!(
        world.owned_ingredient("tomato"),
        base_cap + slot_stack * bays,
        "and the stock it was holding is untouched"
    );
}

// =====================================================================
// 6. Save and load
// =====================================================================

#[test]
fn a_warehouse_and_the_stock_in_it_survive_a_round_trip() {
    let mut world = world();
    build_rack(&mut world, (0, -8));
    build_rack(&mut world, (1, -8));
    order(&mut world, "tomato", 400);
    order(&mut world, "sugar", 120);

    let text = world.to_save_ron();
    let loaded = World::from_save_ron(&text, recipes()).expect("the save must load");

    assert_eq!(loaded.storage_bays(), world.storage_bays());
    assert_eq!(loaded.owned_ingredient("tomato"), 400);
    assert_eq!(loaded.owned_ingredient("sugar"), 120);
    assert_eq!(
        loaded.stock_cap_for("tomato"),
        world.stock_cap_for("tomato")
    );
    assert!(matches!(
        loaded.query_tile((0, -8)),
        Some(TileSnapshot::WarehouseRack { .. })
    ));
}

/// A hand-edited or content-drifted save that holds more than its racks can
/// is rejected at load, not silently trimmed — the same rule every other
/// content mismatch follows.
#[test]
fn a_save_holding_more_than_its_racks_can_is_refused() {
    let mut world = world();
    build_rack(&mut world, (0, -8));
    order(&mut world, "tomato", 400);
    let mut data = SaveData::from_ron(&world.to_save_ron()).expect("the save must parse");
    for (id, count) in &mut data.owned_ingredients {
        if id == "tomato" {
            *count = 4_000;
        }
    }

    assert!(
        World::from_save(data, recipes()).is_err(),
        "stock past what the saved racks can hold is a refusal, not a silent trim"
    );
}

// =====================================================================
// 7. Auto-fill
// =====================================================================

/// Capacity governs the larder, not the buildings drawing on it: a hopper
/// tops itself up from held stock at exactly the same rate with a warehouse
/// and without one.
#[test]
fn auto_fill_behaves_identically_with_a_warehouse_and_without() {
    // Auto-fill for 500 ticks out of a larder of `stock`, with `racks`
    // standing: what the ledger lost, and what the bins are holding.
    let run = |racks: u32, stock: u32| {
        let mut world = world();
        for n in 0..racks as i32 {
            build_rack(&mut world, (n, -8));
        }
        order(&mut world, "tomato", stock);
        let start = world.owned_ingredient("tomato");
        assert_eq!(start, stock, "the larder must actually hold {stock}");
        for tile in hopper_tiles(&world) {
            world.issue_command(PlayerCommand::SetHopperAutoFill {
                tile,
                enabled: true,
            });
        }
        for _ in 0..500 {
            world.tick();
        }
        (start - world.owned_ingredient("tomato"), in_hoppers(&world))
    };

    let (base_cap, slot_stack, bays) = knobs(&world());
    assert_eq!(
        run(0, base_cap),
        run(3, base_cap),
        "three racks standing on the floor change nothing about how a hopper draws on \
         a larder of the same size — capacity governs the larder, not the buildings \
         drawing on it"
    );

    // And with the room actually used, auto-fill simply goes on longer: it
    // is the larder that got bigger, not the draw that got faster.
    let (drawn_small, _) = run(0, base_cap);
    let (drawn_large, _) = run(3, base_cap + slot_stack * bays);
    assert!(
        drawn_large > drawn_small,
        "a stocked warehouse must keep the bins fed past the point a base larder runs dry"
    );
}

/// Every placed hopper's tile.
fn hopper_tiles(world: &World) -> Vec<(i32, i32)> {
    world
        .hopper_snapshots()
        .into_iter()
        .map(|hopper| hopper.tile)
        .collect()
}

/// Everything the hoppers are holding, in units.
fn in_hoppers(world: &World) -> u32 {
    world
        .hopper_snapshots()
        .into_iter()
        .map(|hopper| hopper.quantity)
        .sum()
}

// =====================================================================
// 8. R6 — a rack is pure storage
// =====================================================================

/// The rack does not touch the line. It is not a hopper: it meters nothing
/// onto a belt, takes nothing off one, and joins no hand-off link. That
/// absence is the design (the sprint plan's R6), and an absence has to be
/// asserted or it will be "fixed" into existence by the next person reading
/// `topology`.
#[test]
fn a_rack_beside_a_running_belt_never_touches_it() {
    let mut world = world();
    build_rack(&mut world, (0, -8));
    // A belt of its own, running east past the rack's north face, fed by
    // nothing. If the rack pushed, this is where the item would appear.
    let lonely = (0, -6);
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 3,
    });
    for dx in 0..3 {
        world.issue_command(PlayerCommand::Place {
            kind: "belt".to_string(),
            tile: (lonely.0 + dx, lonely.1),
            direction: Direction::East,
            item: None,
        });
    }
    let cap = world.stock_cap_for("tomato");
    order(&mut world, "tomato", cap);
    assert!(world.owned_ingredient("tomato") > 0, "the rack holds stock");

    // The belt's own links are where a hand-off would be declared, and the
    // rack is not in them: `BodyKind::WarehouseRack` maps to no conveyor
    // link, so the topology pass never pairs the two.
    for dx in 0..3 {
        match world.query_tile((lonely.0 + dx, lonely.1)) {
            // The run feeds itself west to east; what must not appear is an
            // input from anything that is not another belt — the rack stands
            // against the head's southern flank.
            Some(TileSnapshot::Belt { links, .. }) => assert!(
                links
                    .inputs
                    .iter()
                    .all(|input| input.kind == LinkKind::Belt),
                "the belt beside a rack is fed by nothing but belt: {links:?}"
            ),
            other => panic!("a belt was placed at dx {dx}, not {other:?}"),
        }
    }

    for _ in 0..2_000 {
        common::supply(&mut world);
        world.tick();
    }

    let items: usize = (0..3)
        .map(|dx| belt_item_count(&world, (lonely.0 + dx, lonely.1)))
        .sum();
    assert_eq!(
        items, 0,
        "nothing reached the belt beside the rack — a rack is pure storage"
    );
}

fn belt_item_count(world: &World, tile: (i32, i32)) -> usize {
    let chunk = (tile.0.div_euclid(32), tile.1.div_euclid(32));
    world
        .chunk_snapshot(chunk)
        .belts
        .into_iter()
        .find(|belt| belt.tile == tile)
        .map(|belt| belt.items.len())
        .unwrap_or(0)
}
