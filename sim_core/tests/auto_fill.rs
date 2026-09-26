//! Headless integration test for **automatic top-up** — the mechanic that
//! takes keeping a factory fed off the player's keyboard and puts it on the
//! buildings themselves (the hopper/tank-panel epic's ticket 1).
//!
//! Until this landed, a hopper was filled by `PlayerCommand::FillHopper` and
//! a tank by `FillFluidTank`, both bound to a key in the scene — so an
//! unattended line ran for as long as one hopper-load and then stopped. A
//! placed hopper or tank can now be switched to automatic
//! (`PlayerCommand::SetHopperAutoFill` / `SetTankAutoFill`), after which
//! `World::tick` tops it up to capacity out of the player's **bought stock**
//! every tick, before anything else in the tick consumes anything.
//!
//! Five things only the whole `World` can show, and this file proves each:
//!
//! 1. **The top-up is the manual fill rule, run automatically.** Both go
//!    through one helper per kind (`World::fill_hopper_entity` /
//!    `fill_tank_entity`), so the two clamps compose in the same order —
//!    owned stock first, then the buffer's own room — and the ledger is
//!    debited by exactly what the buffer took. An empty larder is a silent
//!    stop, not an error and not a flag flip.
//! 2. **It spends stock, never money** (GAMEPLAY.md §5). Buying stays an
//!    explicit `BuyIngredient`; a toggle that quietly drained the balance
//!    would turn the recurring ingredient cost from a decision into a leak.
//! 3. **It runs first in the tick**, so a hopper drained onto its belt is
//!    full again before it is asked to meter the next unit — an auto-filled
//!    line never stutters for a tick.
//! 4. **The flag is a property of the building, not of its contents.** It
//!    survives a `Rotate` and a contents change; a `Remove` destroys it with
//!    the rest of the body, so a re-placed building starts manual again.
//! 5. **Mass is still conserved**, now including the ledger: with auto-fill
//!    on, stock moves out of the larder and into the factory by itself, and
//!    the sum of the two plus everything shipped must not move.
//!
//! `SetHopperItem` — the explicit item picker a placed hopper's panel uses —
//! is covered here too, since it is the third command that panel issues. It
//! is now the ONLY way a placed hopper changes its mind: the
//! `CycleHopperItem` command this file used to point at was deleted with the
//! **Q** keybind that was its only caller.

mod common;

use sim_core::sim::{
    FLUID_TANK_CAPACITY, Fx, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, TileSnapshot, World,
    command_from_json, reference_chain_with_budget,
};

/// Enough money for the stock these tests buy, on top of the shop bill the
/// fixture already spends.
const BUDGET: u32 = 5000 + common::WAGE_RESERVE;

/// The tomato hopper's tile in the reference chain's layout, and the tank's.
/// Both are pinned by `hopper.rs`/`fluids.rs` already; named here so a test
/// says which building it means.
const TOMATO_HOPPER: (i32, i32) = (-1, 0);
const TANK: (i32, i32) = (5, 2);

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world() -> World {
    reference_chain_with_budget(recipes(), BUDGET)
}

/// Buys `quantity` of `item`, building the warehouse room for it first and
/// asserting the purchase landed — an unaffordable or unhousable buy is a
/// silent no-op, which would otherwise surface much later as an unexplained
/// empty bin. `common::buy` is the one implementation of that.
fn buy(world: &mut World, item: &str, quantity: u32) {
    common::buy(world, item, quantity);
}

fn hopper_at(world: &World, tile: (i32, i32)) -> (u32, bool) {
    let snapshot = world
        .hopper_snapshots()
        .into_iter()
        .find(|h| h.tile == tile)
        .expect("the reference chain places a hopper here");
    (snapshot.quantity, snapshot.auto_fill)
}

fn tank_at(world: &World, tile: (i32, i32)) -> (f64, bool) {
    let snapshot = world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|t| t.tile == tile)
        .expect("the reference chain places a tank here");
    (snapshot.quantity, snapshot.auto_fill)
}

fn set_hopper_auto_fill(world: &mut World, tile: (i32, i32), enabled: bool) {
    world.issue_command(PlayerCommand::SetHopperAutoFill { tile, enabled });
}

fn set_tank_auto_fill(world: &mut World, tile: (i32, i32), enabled: bool) {
    world.issue_command(PlayerCommand::SetTankAutoFill { tile, enabled });
}

// ---------------------------------------------------------------------
// 1. The hopper
// ---------------------------------------------------------------------

#[test]
fn a_new_hopper_is_manual_and_the_toggle_turns_it_on_and_off() {
    let mut world = world();
    assert!(
        !hopper_at(&world, TOMATO_HOPPER).1,
        "a freshly placed hopper is loaded by hand until the player says otherwise"
    );

    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    assert!(hopper_at(&world, TOMATO_HOPPER).1);

    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, false);
    assert!(!hopper_at(&world, TOMATO_HOPPER).1);
}

#[test]
fn an_auto_filling_hopper_is_topped_up_to_capacity_and_the_ledger_pays_for_it() {
    let mut world = world();
    buy(&mut world, "tomato", 200);
    let stock_before = world.owned_ingredient("tomato");

    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    // The toggle itself moves nothing — only a tick does.
    assert_eq!(hopper_at(&world, TOMATO_HOPPER).0, 0);
    assert_eq!(world.owned_ingredient("tomato"), stock_before);

    world.tick();
    let (quantity, _) = hopper_at(&world, TOMATO_HOPPER);
    assert_eq!(
        quantity,
        HOPPER_CAPACITY - 1,
        "step 0 fills it to the brim and step 5 meters one unit onto its belt in \
         the same tick — which is exactly what running the top-up first is for"
    );
    assert_eq!(
        stock_before - world.owned_ingredient("tomato"),
        HOPPER_CAPACITY,
        "the ledger is debited by exactly what the bin took"
    );
}

#[test]
fn auto_fill_spends_stock_and_never_money() {
    let mut world = world();
    buy(&mut world, "tomato", 200);
    let money_before = world.money();

    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    for _ in 0..500 {
        world.tick();
    }

    assert!(
        world.owned_ingredient("tomato") < 200,
        "the top-ups must actually be drawing on the larder"
    );
    assert_eq!(
        world.money(),
        money_before,
        "GAMEPLAY.md §5's recurring ingredient cost stays an explicit purchase — \
         a toggle must never move the balance by itself"
    );
}

#[test]
fn a_partial_larder_is_taken_exactly_and_then_the_top_ups_simply_stop() {
    let mut world = world();
    // Less than one hopper-load, so the first tick can only take part of the
    // room and the second must find nothing left.
    let partial = HOPPER_CAPACITY / 4;
    buy(&mut world, "tomato", partial);

    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    world.tick();

    assert_eq!(
        hopper_at(&world, TOMATO_HOPPER).0,
        partial - 1,
        "the owned-stock clamp binds before the room clamp does (less the one \
         unit metered onto the belt in the same tick)"
    );
    assert_eq!(world.owned_ingredient("tomato"), 0);

    let (quantity_before, _) = hopper_at(&world, TOMATO_HOPPER);
    for _ in 0..50 {
        world.tick();
    }
    assert!(
        hopper_at(&world, TOMATO_HOPPER).1,
        "running out of stock is not an error and must not flip the flag off"
    );
    assert!(
        hopper_at(&world, TOMATO_HOPPER).0 <= quantity_before,
        "nothing may be conjured into a hopper with an empty larder"
    );
    assert_eq!(world.owned_ingredient("tomato"), 0);

    // ...and buying more resumes the top-ups by itself, with no second
    // command: that is the whole point of the flag not being flipped.
    buy(&mut world, "tomato", 100);
    world.tick();
    assert_eq!(hopper_at(&world, TOMATO_HOPPER).0, HOPPER_CAPACITY);
}

#[test]
fn disabling_auto_fill_stops_the_top_ups_immediately() {
    let mut world = world();
    buy(&mut world, "tomato", 300);
    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    world.tick();
    assert_eq!(hopper_at(&world, TOMATO_HOPPER).0, HOPPER_CAPACITY - 1);

    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, false);
    let stock = world.owned_ingredient("tomato");
    for _ in 0..300 {
        world.tick();
    }
    assert_eq!(
        world.owned_ingredient("tomato"),
        stock,
        "a hopper switched back to manual draws nothing at all"
    );
    assert!(
        hopper_at(&world, TOMATO_HOPPER).0 < HOPPER_CAPACITY,
        "and it drains onto its belt like any other hopper"
    );
}

/// The mechanic's real purpose: a hopper feeding a belt is refilled *before*
/// it meters the next unit, so it never runs dry while stock lasts. This is
/// what makes auto-fill running first in the tick (`World::tick`'s step 0)
/// observable rather than an implementation detail.
#[test]
fn a_draining_hopper_is_kept_full_every_tick_while_stock_lasts() {
    let mut world = world();
    buy(&mut world, "tomato", 400);
    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);

    for tick in 0..400 {
        world.tick();
        assert!(
            hopper_at(&world, TOMATO_HOPPER).0 >= HOPPER_CAPACITY - 1,
            "the hopper must be back at the brim at the end of every tick, less \
             at most the one unit it metered onto its belt in that same tick \
             (tick {tick}, quantity {})",
            hopper_at(&world, TOMATO_HOPPER).0
        );
    }
    assert!(
        world.owned_ingredient("tomato") < 400,
        "...and that fullness must be paid for out of the larder"
    );
}

// ---------------------------------------------------------------------
// 2. The tank
// ---------------------------------------------------------------------

#[test]
fn a_new_tank_is_manual_and_the_toggle_turns_it_on_and_off() {
    let mut world = world();
    assert!(!tank_at(&world, TANK).1);

    set_tank_auto_fill(&mut world, TANK, true);
    assert!(tank_at(&world, TANK).1);

    set_tank_auto_fill(&mut world, TANK, false);
    assert!(!tank_at(&world, TANK).1);
}

#[test]
fn an_auto_filling_tank_is_topped_up_to_capacity_and_the_ledger_pays_for_it() {
    let mut world = world();
    buy(&mut world, "vinegar", 400);
    let stock_before = world.owned_ingredient("vinegar");

    set_tank_auto_fill(&mut world, TANK, true);
    assert_eq!(
        tank_at(&world, TANK).0,
        0.0,
        "the toggle alone moves nothing"
    );

    world.tick();
    // The tank discharges into its run in the same tick it was filled (step
    // 1 follows step 0), so it reads slightly under capacity by the end of
    // the tick — but the ledger was debited for the whole load.
    assert_eq!(
        stock_before - world.owned_ingredient("vinegar"),
        FLUID_TANK_CAPACITY,
        "one tick buys a whole tank's worth out of the larder"
    );
    assert!(
        tank_at(&world, TANK).0 > 0.0,
        "and the tank actually holds it"
    );
}

/// The tank's own half of the whole-unit rule: over a long run in which the
/// tank is topping up a *trickle* every tick — the case where rounding a part
/// unit up would bill sixty whole units a second for a dribble — the ledger
/// never pays for volume the tank did not receive.
///
/// The exact equality is asserted end to end by
/// `mass_is_conserved_with_auto_fill_on_across_the_whole_chain`, which counts
/// vinegar as a raw ingredient precisely so this rule is pinned; what this
/// test adds is the per-tick statement, which is where a drift would start.
#[test]
fn an_auto_filling_tank_is_billed_only_for_whole_units_it_actually_took() {
    let mut world = world();
    buy(&mut world, "vinegar", 1500);
    buy(&mut world, "tomato", 600);
    buy(&mut world, "sugar", 150);
    // The whole line on automatic, so the mixer is really drawing vinegar
    // rather than the tank merely wetting a run once and then sitting still.
    for hopper in world.hopper_snapshots() {
        set_hopper_auto_fill(&mut world, hopper.tile, true);
    }
    set_tank_auto_fill(&mut world, TANK, true);

    let mut spent = 0u32;
    let mut trickle_ticks = 0;
    for _ in 0..2000 {
        let before = world.owned_ingredient("vinegar");
        let volume_before = Fx::from_num(tank_at(&world, TANK).0);
        world.tick();
        let debited = before - world.owned_ingredient("vinegar");
        spent += debited;
        let volume_after = Fx::from_num(tank_at(&world, TANK).0);
        if debited > 0 && debited < FLUID_TANK_CAPACITY {
            trickle_ticks += 1;
        }
        assert!(
            volume_after - volume_before <= Fx::from_num(debited),
            "a tank may never gain more volume than the ledger paid for"
        );
    }
    assert!(spent > 0, "the tank must actually have been topping up");
    assert!(
        trickle_ticks > 10,
        "the run must actually reach the trickle case this test exists for \
         (only {trickle_ticks} such ticks)"
    );
    assert!(
        Fx::from_num(tank_at(&world, TANK).0) <= Fx::from_num(FLUID_TANK_CAPACITY),
        "a tank never holds more than its capacity"
    );
}

#[test]
fn a_tank_plumbed_to_nothing_is_never_topped_up_and_is_never_charged() {
    let mut world = world();
    buy(&mut world, "vinegar", 400);

    // Pull the tank's own discharge pipe out from under it: with no network
    // to discharge into, there is no fluid id to buy a drum of.
    let pipe = world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|t| t.tile == TANK)
        .and_then(|t| t.pipe_tile)
        .expect("the reference chain plumbs its tank into a run");
    world.issue_command(PlayerCommand::Remove { tile: pipe });

    set_tank_auto_fill(&mut world, TANK, true);
    let stock = world.owned_ingredient("vinegar");
    let money = world.money();
    for _ in 0..100 {
        world.tick();
    }
    assert_eq!(tank_at(&world, TANK).0, 0.0);
    assert_eq!(
        world.owned_ingredient("vinegar"),
        stock,
        "you cannot buy a drum of nothing — automatically or otherwise"
    );
    assert_eq!(world.money(), money);
    assert!(
        tank_at(&world, TANK).1,
        "the flag itself is still allowed to be set on an unplumbed tank"
    );
}

#[test]
fn disabling_a_tanks_auto_fill_stops_the_top_ups() {
    let mut world = world();
    buy(&mut world, "vinegar", 500);
    set_tank_auto_fill(&mut world, TANK, true);
    world.tick();

    set_tank_auto_fill(&mut world, TANK, false);
    let stock = world.owned_ingredient("vinegar");
    for _ in 0..500 {
        world.tick();
    }
    assert_eq!(
        world.owned_ingredient("vinegar"),
        stock,
        "a tank switched back to manual draws nothing at all"
    );
}

// ---------------------------------------------------------------------
// 3. The flag is a property of the building
// ---------------------------------------------------------------------

#[test]
fn rotating_a_building_keeps_its_auto_fill_flag() {
    let mut world = world();
    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    set_tank_auto_fill(&mut world, TANK, true);

    world.issue_command(PlayerCommand::Rotate {
        tile: TOMATO_HOPPER,
    });
    world.issue_command(PlayerCommand::Rotate { tile: TANK });

    assert!(
        hopper_at(&world, TOMATO_HOPPER).1,
        "a rotation is pure geometry — it must not reach into the body"
    );
    assert!(tank_at(&world, TANK).1);
}

#[test]
fn removing_and_replacing_a_hopper_starts_it_manual_again() {
    let mut world = world();
    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);

    world.issue_command(PlayerCommand::Remove {
        tile: TOMATO_HOPPER,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "hopper".to_string(),
        tile: TOMATO_HOPPER,
        direction: sim_core::sim::Direction::East,
        item: Some("tomato".to_string()),
    });

    assert!(
        !hopper_at(&world, TOMATO_HOPPER).1,
        "`Remove` destroys the body with everything in it, the flag included"
    );
}

#[test]
fn changing_a_hoppers_contents_keeps_its_auto_fill_flag() {
    let mut world = world();
    buy(&mut world, "tomato", 100);
    buy(&mut world, "sugar", 100);
    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, true);
    world.tick();
    assert_eq!(hopper_at(&world, TOMATO_HOPPER).0, HOPPER_CAPACITY - 1);

    world.issue_command(PlayerCommand::SetHopperItem {
        tile: TOMATO_HOPPER,
        item: "sugar".to_string(),
    });
    assert!(
        hopper_at(&world, TOMATO_HOPPER).1,
        "\"keep this bin full\" is about the bin, not about what is in it"
    );
    assert_eq!(
        hopper_at(&world, TOMATO_HOPPER).0,
        0,
        "retargeting still clears the bin"
    );

    // ...and the very next tick tops it up with the *new* item.
    world.tick();
    assert!(hopper_at(&world, TOMATO_HOPPER).0 >= HOPPER_CAPACITY - 1);
    assert_eq!(
        world.owned_ingredient("sugar"),
        100 - HOPPER_CAPACITY,
        "the top-up is paid for out of the new item's stock"
    );
}

// ---------------------------------------------------------------------
// 4. Refusals — a bad command leaves the world byte-identical
// ---------------------------------------------------------------------

/// Everything a snapshot of the whole world can see, as one comparable
/// string. A refused command must not move any of it.
fn world_fingerprint(world: &World) -> String {
    let mut chunks = String::new();
    for chunk in world.all_chunks() {
        chunks.push_str(&serde_json::to_string(&world.chunk_snapshot(chunk)).unwrap());
    }
    format!(
        "{chunks}|{}|{}|{}|{}|{}",
        serde_json::to_string(&world.hopper_snapshots()).unwrap(),
        serde_json::to_string(&world.fluid_tank_snapshots()).unwrap(),
        serde_json::to_string(&world.world_layout()).unwrap(),
        world.money(),
        world.owned_ingredient("tomato"),
    )
}

#[test]
fn a_toggle_aimed_at_the_wrong_kind_of_tile_is_a_true_no_op() {
    let mut world = world();
    buy(&mut world, "tomato", 100);
    let before = world_fingerprint(&world);

    // A belt tile, a machine tile, a tile with nothing on it, and each
    // toggle aimed at the *other* kind of building.
    let belt = (0, 0);
    let empty = (900, 900);
    for cmd in [
        PlayerCommand::SetHopperAutoFill {
            tile: belt,
            enabled: true,
        },
        PlayerCommand::SetHopperAutoFill {
            tile: empty,
            enabled: true,
        },
        PlayerCommand::SetHopperAutoFill {
            tile: TANK,
            enabled: true,
        },
        PlayerCommand::SetTankAutoFill {
            tile: belt,
            enabled: true,
        },
        PlayerCommand::SetTankAutoFill {
            tile: empty,
            enabled: true,
        },
        PlayerCommand::SetTankAutoFill {
            tile: TOMATO_HOPPER,
            enabled: true,
        },
        PlayerCommand::SetHopperItem {
            tile: belt,
            item: "tomato".to_string(),
        },
        PlayerCommand::SetHopperItem {
            tile: empty,
            item: "tomato".to_string(),
        },
    ] {
        world.issue_command(cmd);
    }

    assert_eq!(
        world_fingerprint(&world),
        before,
        "a command that cannot apply must leave the world byte-identical"
    );
}

#[test]
fn setting_a_toggle_to_the_value_it_already_has_changes_nothing() {
    let mut world = world();
    let before = world_fingerprint(&world);
    set_hopper_auto_fill(&mut world, TOMATO_HOPPER, false);
    set_tank_auto_fill(&mut world, TANK, false);
    assert_eq!(world_fingerprint(&world), before);
}

// ---------------------------------------------------------------------
// 5. `SetHopperItem` — the explicit picker
// ---------------------------------------------------------------------

#[test]
fn set_hopper_item_switches_and_clears() {
    let mut world = world();
    buy(&mut world, "tomato", 100);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 32,
        tile: Some(TOMATO_HOPPER),
    });
    assert_eq!(hopper_at(&world, TOMATO_HOPPER).0, 32);

    world.issue_command(PlayerCommand::SetHopperItem {
        tile: TOMATO_HOPPER,
        item: "sugar".to_string(),
    });
    let snapshot = world
        .hopper_snapshots()
        .into_iter()
        .find(|h| h.tile == TOMATO_HOPPER)
        .unwrap();
    assert_eq!(snapshot.item, "sugar");
    assert_eq!(
        snapshot.quantity, 0,
        "a hopper holds one item, so switching discards what it had"
    );
}

#[test]
fn set_hopper_item_refuses_an_item_that_is_not_raw() {
    let mut world = world();
    buy(&mut world, "tomato", 100);
    let before = world_fingerprint(&world);

    for item in ["tomato_paste", "boxed_ketchup", "not_an_item_at_all"] {
        world.issue_command(PlayerCommand::SetHopperItem {
            tile: TOMATO_HOPPER,
            item: item.to_string(),
        });
    }
    assert_eq!(
        world_fingerprint(&world),
        before,
        "only what `World::raw_items` names may go in a hopper — the same set \
         `Place` validates against"
    );
}

#[test]
fn set_hopper_item_naming_the_current_item_keeps_the_contents() {
    let mut world = world();
    buy(&mut world, "tomato", 100);
    world.issue_command(PlayerCommand::FillHopper {
        amount: 40,
        tile: Some(TOMATO_HOPPER),
    });

    world.issue_command(PlayerCommand::SetHopperItem {
        tile: TOMATO_HOPPER,
        item: "tomato".to_string(),
    });
    assert_eq!(
        hopper_at(&world, TOMATO_HOPPER).0,
        40,
        "a picker redraws its own current selection; re-selecting must not empty the bin"
    );
}

// ---------------------------------------------------------------------
// 6. The wire forms presentation issues
// ---------------------------------------------------------------------

#[test]
fn the_new_commands_have_the_documented_wire_forms() {
    let mut world = world();
    buy(&mut world, "tomato", 100);

    // set_hopper_auto_fill — `enabled` optional, defaulting to true.
    let cmd = command_from_json(r#"{"type": "set_hopper_auto_fill", "tile": [-1, 0]}"#)
        .expect("set_hopper_auto_fill must parse");
    world.issue_command(cmd);
    assert!(hopper_at(&world, TOMATO_HOPPER).1);

    let cmd =
        command_from_json(r#"{"type": "set_hopper_auto_fill", "tile": [-1, 0], "enabled": false}"#)
            .expect("an explicit `enabled` must parse");
    world.issue_command(cmd);
    assert!(!hopper_at(&world, TOMATO_HOPPER).1);

    // set_tank_auto_fill — the same shape.
    let cmd = command_from_json(r#"{"type": "set_tank_auto_fill", "tile": [5, 2]}"#)
        .expect("set_tank_auto_fill must parse");
    world.issue_command(cmd);
    assert!(tank_at(&world, TANK).1);

    let cmd =
        command_from_json(r#"{"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": false}"#)
            .expect("an explicit `enabled` must parse");
    world.issue_command(cmd);
    assert!(!tank_at(&world, TANK).1);

    // set_hopper_item — both fields required.
    let cmd = command_from_json(r#"{"type": "set_hopper_item", "tile": [-1, 0], "item": "sugar"}"#)
        .expect("set_hopper_item must parse");
    world.issue_command(cmd);
    match world.query_tile(TOMATO_HOPPER) {
        Some(TileSnapshot::Hopper {
            item, auto_fill, ..
        }) => {
            assert_eq!(item, "sugar");
            assert!(!auto_fill, "query_tile carries the flag too");
        }
        other => panic!("expected a hopper tile, got {other:?}"),
    }

    assert!(
        command_from_json(r#"{"type": "set_hopper_item", "tile": [-1, 0]}"#).is_none(),
        "`item` is required — only the player knows which ingredient"
    );
    assert!(
        command_from_json(r#"{"type": "keep_it_topped_up", "tile": [-1, 0]}"#).is_none(),
        "an unknown variant is still `None`, as it always was"
    );
}

// ---------------------------------------------------------------------
// 7. Conservation, with the ledger in the sum
// ---------------------------------------------------------------------

/// How many units of raw ingredient `raw` one unit of `id` embodies —
/// `core_loop.rs`'s own walk, which this file needs for the same reason it
/// does and which is only a handful of lines.
fn raw_content(id: &str, raw: &str, recipes: &RecipeRegistry) -> u32 {
    if id == raw {
        return 1;
    }
    let Some(recipe) = recipes.ordered().iter().find(|r| {
        r.outputs.iter().any(|(out, _)| out == id)
            || r.fluid_outputs.iter().any(|(out, _)| out == id)
    }) else {
        return 0;
    };
    recipe
        .inputs
        .iter()
        .chain(recipe.fluid_inputs.iter())
        .map(|(input, qty)| qty * raw_content(input, raw, recipes))
        .sum()
}

/// Every unit of `raw` the world can currently account for: **the larder**,
/// plus everything the factory holds in any form, plus everything already
/// shipped inside finished product. With auto-fill on, stock moves out of
/// the larder into the factory with no command at all — so counting the two
/// together is exactly the check that the transfer is a transfer and not a
/// duplication or a leak.
fn accounted(world: &World, raw: &str, recipes: &RecipeRegistry, final_item: &str) -> Fx {
    let value = |id: &str| Fx::from_num(raw_content(id, raw, recipes));
    let mut total = Fx::from_num(world.owned_ingredient(raw));

    for hopper in world.hopper_snapshots() {
        total += value(&hopper.item) * Fx::from_num(hopper.quantity);
    }
    for tank in world.fluid_tank_snapshots() {
        if let Some(fluid) = &tank.fluid {
            total += value(fluid) * Fx::from_num(tank.quantity);
        }
    }
    for network in world.fluid_snapshots() {
        if let Some(fluid) = &network.fluid {
            total += value(fluid) * Fx::from_num(network.total_volume);
        }
    }
    for chunk in world.all_chunks() {
        let snapshot = world.chunk_snapshot(chunk);
        for belt in &snapshot.belts {
            for (item, _) in &belt.items {
                total += value(item);
            }
        }
        for splitter in &snapshot.splitters {
            for (item, _) in &splitter.items {
                total += value(item);
            }
        }
        for machine in &snapshot.machines {
            for (item, qty) in machine.input.iter().chain(machine.output.iter()) {
                total += value(item) * Fx::from_num(*qty);
            }
            if machine.process.is_some() {
                let recipe = recipes
                    .get(&machine.recipe)
                    .expect("a machine references a real recipe");
                for (item, qty) in recipe.outputs.iter().chain(recipe.fluid_outputs.iter()) {
                    total += value(item) * Fx::from_num(*qty);
                }
            }
        }
        // `shipped_total` counts every box that reached the dock, standing
        // on it or since collected, so `contents` is already inside it.
        for dock in &snapshot.docks {
            total += value(final_item) * Fx::from_num(dock.shipped_total);
        }
    }
    total
}

/// With both hoppers and the tank on automatic and nothing being removed,
/// the total of the larder plus everything in the factory plus everything
/// shipped must not move — every tick, for both raw solids.
#[test]
fn mass_is_conserved_with_auto_fill_on_across_the_whole_chain() {
    let registry = recipes();
    let final_item = registry.ordered().last().unwrap().outputs[0].0.clone();
    let mut world = reference_chain_with_budget(recipes(), 4000 + common::WAGE_RESERVE);

    buy(&mut world, "tomato", 600);
    buy(&mut world, "sugar", 150);
    buy(&mut world, "vinegar", 400);

    for hopper in world.hopper_snapshots() {
        set_hopper_auto_fill(&mut world, hopper.tile, true);
    }
    for tank in world.fluid_tank_snapshots() {
        set_tank_auto_fill(&mut world, tank.tile, true);
    }

    // **Vinegar is in this list, and that is the point.** It is the one raw
    // ingredient with a continuous buffer, so it is the one the whole-unit
    // rule protects: an auto-fill that asked for its *ceiling* of room would
    // bill a whole unit for a fraction of one, sixty times a second, and this
    // equality would drift within a handful of ticks.
    let raws = ["tomato", "sugar", "vinegar"];
    let baselines: Vec<Fx> = raws
        .iter()
        .map(|raw| accounted(&world, raw, &registry, &final_item))
        .collect();

    for tick in 0..3000 {
        world.tick();
        for (index, raw) in raws.iter().enumerate() {
            assert_eq!(
                accounted(&world, raw, &registry, &final_item),
                baselines[index],
                "{raw} was duplicated or lost at tick {tick} with auto-fill on"
            );
        }
    }

    // ...and the run really did exercise the mechanic rather than idling.
    assert!(
        world.owned_ingredient("tomato") < 600,
        "the larder must actually have been drawn on"
    );
    let shipped: u32 = world
        .dock_snapshots()
        .iter()
        .map(|dock| dock.shipped_total)
        .sum();
    assert!(
        shipped > 0,
        "an auto-filled line must actually reach the dock"
    );
}
