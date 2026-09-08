//! The **reference chain**: the exact factory `World::new` used to build for
//! itself, rebuilt out of nothing but purchase and placement commands.
//!
//! # Why this exists
//!
//! Before the "Real Placement" epic, every world arrived with a complete
//! six-stage production line in it — hopper, belts, crusher, mixer, cooking
//! vat, aging rack, bottler, packer, a vinegar tank and its pipe run, one
//! more belt and a delivery dock — laid out from `data/recipes.ron`'s file
//! order and facing east. `World::new` now builds nothing, so that layout
//! had to go somewhere, and it is here.
//!
//! It does two jobs at once, and the second is the point:
//!
//! 1. **It is the test fixture** every integration test in `sim_core/tests/`
//!    builds its world from, so those tests still assert what they always
//!    asserted — mass conservation across the whole chain, contract payouts,
//!    fluid consumption, footprint geometry, chunk reporting — against the
//!    same factory as before.
//! 2. **It is the proof that placement can reach that factory**, because it
//!    is built *through the public command API a player uses* —
//!    [`PlayerCommand::Purchase`] then [`PlayerCommand::Place`], one call per
//!    piece, with no privileged construction path of any kind. If any of
//!    those commands refused, the layout would come out wrong and the whole
//!    suite would fail. There is deliberately no shortcut here: this module
//!    can do nothing a player cannot do.
//!
//! Every placement it makes faces [`Direction::East`], which is what makes
//! the resulting tiles **byte-identical** to the pre-placement layout (see
//! [`Direction`]'s own doc: `East` is the regression anchor for this whole
//! epic). `tests/machine_footprints.rs` pins those coordinates literally.
//!
//! # The layout rule it reproduces
//!
//! Stage `i` is four belt tiles then that recipe's machine, and the next
//! stage begins immediately after the machine's *whole* width — the
//! accumulate-widths rule the "Machines improvement" epic's ticket 1
//! introduced. The hopper stands one tile west of the first belt, the pipe
//! run occupies four tiles on the row immediately north of whichever machine
//! its recipe feeds, ending beside that machine, with the tank one tile west
//! of the run's head; the dock stands past four more belt tiles at the end
//! of the line.
//!
//! # What it costs
//!
//! It pays for itself. The bill is priced out of `data/shop.ron` before the
//! world is built, the world is constructed with exactly that much money
//! ([`World::with_content_and_money`]), and every unit of it is spent — so a
//! reference-chain world starts with a factory, **zero money and zero
//! unplaced inventory**, which is the clean baseline a test measuring
//! earnings wants. It is emphatically not what a new game starts with
//! ([`super::economy::STARTING_MONEY`] buys a hopper, one machine and some
//! belt): a player works up to this line, and this fixture skips that.

use std::collections::BTreeMap;

use super::contracts::ContractRegistry;
use super::dock::DOCK_KIND;
use super::fluid::FLUID_TANK_KIND;
use super::hopper::HOPPER_KIND;
use super::machines::MachineRegistry;
use super::recipe::RecipeRegistry;
use super::shop::ShopCatalog;
use super::types::{Direction, Fx, TileCoord};
use super::world::{BELT_KIND, PIPE_KIND, PlayerCommand, World};

/// How many belt tiles feed each stage's machine, and how many run from the
/// last machine to the dock. Four, as every stage had before placement.
const STAGE_BELT_TILES: i32 = 4;
/// How many pipe tiles the vinegar run is. Four, as milestone 6 built it.
const PIPE_RUN_TILES: i32 = 4;
/// The row the line sits on.
const CHAIN_ROW_Y: i32 = 0;

/// One thing to buy and put down.
struct Order {
    kind: &'static str,
    tile: TileCoord,
}

/// The reference chain on the shipped `data/machines.ron` and
/// `data/contracts.ron` — see this module's doc.
pub fn reference_chain(recipes: RecipeRegistry) -> World {
    reference_chain_with(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    )
}

/// [`reference_chain`], with an explicit machine registry and contract board
/// — the seams `World::with_machines`/`World::with_content` already provide,
/// carried through so a test can build the reference layout out of
/// footprints or contracts the shipped content doesn't have.
///
/// Panics if a kind it needs has no footprint or no price, which is a broken
/// fixture rather than a runtime condition: every kind involved is either
/// one of the three no-recipe buildings, the belt, the pipe, or a kind some
/// recipe names, and `World`'s own construction already checks the last of
/// those.
pub fn reference_chain_with(
    recipes: RecipeRegistry,
    machines: MachineRegistry,
    contracts: ContractRegistry,
) -> World {
    // The same cross-file check `World`'s own construction makes, run first
    // so a recipe naming an undefined machine kind fails with `World`'s
    // message rather than with this module's less informative one — the
    // fixture must not change what a content error looks like.
    if let Err(e) = machines.validate_covers(&recipes) {
        panic!("machine content error: {e}");
    }
    let orders = plan(&recipes, &machines);

    // Price the whole bill before building anything, so the world can be
    // constructed with exactly enough money to buy it and no more.
    let catalog = ShopCatalog::bundled();
    let mut wanted: BTreeMap<&'static str, u32> = BTreeMap::new();
    for order in &orders {
        *wanted.entry(order.kind).or_insert(0) += 1;
    }
    let mut bill = Fx::ZERO;
    for (kind, count) in &wanted {
        let entry = catalog.get(kind).unwrap_or_else(|| {
            panic!("data/shop.ron must price '{kind}' for the reference chain to buy one")
        });
        bill += entry.price_money() * Fx::from_num(*count);
    }

    let mut world = World::with_content_and_money(recipes, machines, contracts, bill);
    for (kind, count) in wanted {
        world.issue_command(PlayerCommand::Purchase {
            kind: kind.to_string(),
            quantity: count,
        });
    }
    for order in &orders {
        world.issue_command(PlayerCommand::Place {
            kind: order.kind.to_string(),
            tile: order.tile,
            direction: Direction::East,
        });
    }

    assert_eq!(
        world.placement_count(),
        orders.len(),
        "the reference chain must place every piece it bought — a refused Place means the \
         layout below collided with itself or asked for something unbuildable"
    );
    assert_eq!(
        world.money(),
        Fx::ZERO,
        "the reference chain is priced to spend exactly its bill"
    );
    world
}

/// Every piece of the reference chain and where it goes, in the order it is
/// placed. Pure arithmetic — it touches no world — so the layout rule is
/// readable in one place.
fn plan(recipes: &RecipeRegistry, machines: &MachineRegistry) -> Vec<Order> {
    let ordered = recipes.ordered();
    let mut orders = Vec::new();

    // The hopper: one tile west of the first belt, so it grows away from the
    // line rather than into it.
    let hopper_width = footprint_width(machines, HOPPER_KIND);
    orders.push(Order {
        kind: HOPPER_KIND,
        tile: (-hopper_width, CHAIN_ROW_Y),
    });

    let mut next_stage_x = 0;
    let mut fluid_stage: Option<(TileCoord, i32)> = None;
    for recipe in ordered {
        let footprint = machines
            .footprint_of(&recipe.machine)
            .unwrap_or_else(|| panic!("machine kind '{}' has no footprint", recipe.machine));
        let belt_start_x = next_stage_x;
        let machine_origin_x = belt_start_x + STAGE_BELT_TILES;
        next_stage_x = machine_origin_x + footprint.width_tiles;

        for dx in 0..STAGE_BELT_TILES {
            orders.push(Order {
                kind: BELT_KIND,
                tile: (belt_start_x + dx, CHAIN_ROW_Y),
            });
        }
        orders.push(Order {
            kind: leak(&recipe.machine),
            tile: (machine_origin_x, CHAIN_ROW_Y),
        });
        if !recipe.fluid_inputs.is_empty() {
            fluid_stage = Some((
                (machine_origin_x, CHAIN_ROW_Y),
                footprint.height_tiles,
            ));
        }
    }

    // The pipe run sits on the row immediately north of the machine it
    // feeds — so its inlet is edge-adjacent to that machine's footprint
    // however tall the machine is — and ends directly beside it. The tank
    // stands one tile west of the run's head, like the hopper.
    if let Some((machine_origin, machine_height)) = fluid_stage {
        let pipe_row_y = machine_origin.1 + machine_height;
        for n in 0..PIPE_RUN_TILES {
            orders.push(Order {
                kind: PIPE_KIND,
                tile: (machine_origin.0 - (PIPE_RUN_TILES - 1 - n), pipe_row_y),
            });
        }
        let run_head_x = machine_origin.0 - (PIPE_RUN_TILES - 1);
        let tank_width = footprint_width(machines, FLUID_TANK_KIND);
        orders.push(Order {
            kind: FLUID_TANK_KIND,
            tile: (run_head_x - tank_width, pipe_row_y),
        });
    }

    // The exit: one more ordinary belt run past the last machine, and the
    // dock at the end of it.
    for dx in 0..STAGE_BELT_TILES {
        orders.push(Order {
            kind: BELT_KIND,
            tile: (next_stage_x + dx, CHAIN_ROW_Y),
        });
    }
    orders.push(Order {
        kind: DOCK_KIND,
        tile: (next_stage_x + STAGE_BELT_TILES, CHAIN_ROW_Y),
    });

    orders
}

fn footprint_width(machines: &MachineRegistry, kind: &str) -> i32 {
    machines
        .footprint_of(kind)
        .unwrap_or_else(|| {
            panic!("data/machines.ron must define '{kind}' for the reference chain to place one")
        })
        .width_tiles
}

/// `Order::kind` is a `&'static str` so the common cases are plain
/// constants; a machine kind comes out of a loaded recipe, so it is leaked
/// once here rather than making every order own a `String`. A handful of
/// kind ids per fixture, leaked once per world built in a test process, is
/// the cheaper trade — and this module is a fixture builder, not a hot path.
fn leak(kind: &str) -> &'static str {
    Box::leak(kind.to_string().into_boxed_str())
}
