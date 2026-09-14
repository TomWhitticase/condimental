//! The **reference chain**: the shipped production line, built out of
//! nothing but purchase and placement commands.
//!
//! # Why this exists
//!
//! Before the "Real Placement" epic, every world arrived with a complete
//! production line already in it, laid out from `data/recipes.ron`'s file
//! order and facing east. `World::new` now builds nothing, so that layout
//! had to go somewhere, and it is here.
//!
//! It does two jobs at once, and the second is the point:
//!
//! 1. **It is the test fixture** every integration test in `sim_core/tests/`
//!    builds its world from, so those tests assert what they always
//!    asserted — mass conservation across the whole chain, contract payouts,
//!    fluid consumption, footprint geometry, chunk reporting — against one
//!    agreed factory.
//! 2. **It is the proof that placement can reach that factory**, because it
//!    is built *through the public command API a player uses* —
//!    [`PlayerCommand::Purchase`] then [`PlayerCommand::Place`], one call per
//!    piece, with no privileged construction path of any kind. If any of
//!    those commands refused, the layout would come out wrong and the whole
//!    suite would fail. There is deliberately no shortcut here: this module
//!    can do nothing a player cannot do.
//!
//! Every placement faces [`Direction::East`], which is what makes the
//! resulting tiles reproduce the pre-placement layout wherever the content
//! is unchanged (see [`Direction`]'s own doc: `East` is the regression
//! anchor for the whole placement epic).
//!
//! # The layout rule it reproduces
//!
//! Everything sits on one row (`CHAIN_ROW_Y`), west to east, in
//! `data/recipes.ron` order, and each stage lays down only what its recipe
//! actually needs:
//!
//! - **One feed lane per solid input.** A recipe's solid input at index `j`
//!   gets its own run of [`STAGE_BELT_TILES`] belt tiles on row
//!   `CHAIN_ROW_Y + j`, ending on the tile immediately west of the machine —
//!   i.e. pointing into the `j`-th tile of its inlet face. A stage whose
//!   inputs are entirely fluid (the cooking vat, the bottler) gets no belt
//!   at all: it stands directly at the end of the pipe run feeding it, which
//!   is the visible consequence of the chain rework.
//! - **Every raw solid input gets its own hopper**, at the head of its own
//!   lane, facing east into it. One hopper holds one item, so two raw solids
//!   means two hoppers — and because each has a lane to itself, neither can
//!   crowd the other off a shared belt (see `data/machines.ron`'s note on
//!   why the mixer is 1x2).
//! - **A stage with fluid outputs** gets [`MACHINE_PIPE_RUN_TILES`] pipe
//!   tiles immediately east of its machine, on the chain row: a fluid leaves
//!   by the face the machine points at (`world.rs`'s module doc), so the
//!   pipe standing there *is* the connection. The next stage begins after
//!   that run.
//! - **A stage with a *raw* fluid input** — one nothing upstream produces,
//!   i.e. vinegar — additionally gets a [`PIPE_RUN_TILES`]-tile run on the
//!   row immediately north of the machine's whole footprint, ending beside
//!   it, with a [`FLUID_TANK_KIND`] one tile west of that run's head. That is
//!   milestone 6's arrangement, unchanged except that the machine it reaches
//!   is now two tiles deep.
//! - The exit is [`STAGE_BELT_TILES`] more belt tiles past the last machine
//!   and a [`DOCK_KIND`] at the end of them.
//!
//! # Why the machine-to-machine pipe runs are short
//!
//! [`MACHINE_PIPE_RUN_TILES`] is two, where the tank-fed vinegar run is
//! four, and that is a real decision rather than a spare tile. A pressure
//! network equalizes, so a run of `n` tiles holds roughly `n` times whatever
//! a machine needs at the single tile it draws from — the "standing charge"
//! `FLUID_TANK_CAPACITY`'s own doc describes. The tank-fed run can afford
//! that because the tank presents constant full head and simply pushes
//! through it; a run fed by a *machine* has no such pump behind it, so every
//! extra tile is another whole craft's worth of product that has to be made
//! before the far end can draw once. Two tiles is the shortest length that
//! is unambiguously a run rather than a butt joint, and it keeps that
//! priming cost to one batch.
//!
//! # What it costs
//!
//! It pays for itself. The bill is priced out of `data/shop.ron` before the
//! world is built, the world is constructed with exactly that much money
//! ([`World::with_content_and_money`]), and every unit of it is spent — so a
//! reference-chain world starts with a factory, **zero money and zero
//! unplaced inventory**, which is the clean baseline a test measuring
//! earnings wants.
//!
//! It buys **no ingredients**, deliberately: a world built by this fixture
//! has a factory and an empty larder, so a test that wants the line to run
//! buys and loads exactly the tomatoes, sugar and vinegar it means to
//! account for (GAMEPLAY.md §5). `sim_core/tests/shop.rs` is what pins the
//! bill against [`super::economy::STARTING_MONEY`] — a new game can build
//! this whole line outright and still have most of its balance left, which
//! now has to cover running it as well as building it.

use std::collections::BTreeMap;

use super::contracts::ContractRegistry;
use super::dock::DOCK_KIND;
use super::fluid::FLUID_TANK_KIND;
use super::hopper::HOPPER_KIND;
use super::ingredients::raw_ingredient_ids;
use super::machines::MachineRegistry;
use super::recipe::RecipeRegistry;
use super::shop::ShopCatalog;
use super::types::{Direction, Money, TileCoord};
use super::world::{BELT_KIND, PIPE_KIND, PlayerCommand, World};

/// How many belt tiles feed a stage that has solid inputs, and how many run
/// from the last machine to the dock. Four, as every stage had before
/// placement.
const STAGE_BELT_TILES: i32 = 4;
/// How many pipe tiles the tank-fed vinegar run is. Four, as milestone 6
/// built it.
const PIPE_RUN_TILES: i32 = 4;
/// How many pipe tiles a machine-to-machine run is — see this module's doc
/// for why it is shorter than the tank-fed one.
const MACHINE_PIPE_RUN_TILES: i32 = 2;
/// The row the line sits on. A stage's `j`-th solid input arrives on row
/// `CHAIN_ROW_Y + j`, into the `j`-th tile of the machine's inlet face.
const CHAIN_ROW_Y: i32 = 0;

/// One thing to buy and put down.
struct Order {
    kind: &'static str,
    tile: TileCoord,
    direction: Direction,
    /// Which raw item a hopper holds. `None` for everything else — see
    /// [`PlayerCommand::Place`].
    item: Option<&'static str>,
}

impl Order {
    /// The common case: something on the line, facing east, holding nothing
    /// in particular.
    fn east(kind: &'static str, tile: TileCoord) -> Self {
        Self {
            kind,
            tile,
            direction: Direction::East,
            item: None,
        }
    }
}

/// The reference chain on the shipped `data/machines.ron` and
/// `data/contracts.ron` — see this module's doc. Ends with **zero money**:
/// it is priced to spend exactly its bill.
pub fn reference_chain(recipes: RecipeRegistry) -> World {
    reference_chain_with(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
    )
}

/// [`reference_chain`] with `budget` left in the bank afterwards — the
/// fixture a test that actually wants to *run* the line uses.
///
/// It exists because ingredients stopped being free (GAMEPLAY.md §5). A
/// zero-money world can buy no tomatoes, so it can fill no hopper, so it can
/// make nothing — which is correct behaviour and a useless starting point
/// for a throughput test. The budget is a separate argument rather than
/// folded into the bill so a test still states, in one number, exactly how
/// much money it is allowed to spend on stock; anything left over at the end
/// is an assertion the test can make.
pub fn reference_chain_with_budget(recipes: RecipeRegistry, budget: u32) -> World {
    reference_chain_with_content_and_budget(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        budget,
    )
}

/// The fully-explicit form: [`reference_chain_with`]'s content seams plus
/// [`reference_chain_with_budget`]'s money, for a test that needs both (a
/// custom contract board *and* the funds to run the line against it).
pub fn reference_chain_with_content_and_budget(
    recipes: RecipeRegistry,
    machines: MachineRegistry,
    contracts: ContractRegistry,
    budget: u32,
) -> World {
    build(recipes, machines, contracts, Money::from_num(budget))
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
    build(recipes, machines, contracts, Money::ZERO)
}

/// The one builder all three entry points share: price the plan, construct a
/// world holding exactly that bill plus `budget`, buy everything, place
/// everything, and check that nothing was refused.
fn build(
    recipes: RecipeRegistry,
    machines: MachineRegistry,
    contracts: ContractRegistry,
    budget: Money,
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
    let mut bill = Money::ZERO;
    for (kind, count) in &wanted {
        let entry = catalog.get(kind).unwrap_or_else(|| {
            panic!("data/shop.ron must price '{kind}' for the reference chain to buy one")
        });
        bill += entry.price_money() * Money::from_num(*count);
    }

    let mut world = World::with_content_and_money(recipes, machines, contracts, bill + budget);
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
            direction: order.direction,
            item: order.item.map(str::to_string),
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
        budget,
        "the reference chain is priced to spend exactly its bill, leaving only the budget"
    );
    world
}

/// Every piece of the reference chain and where it goes, in the order it is
/// placed. Pure arithmetic — it touches no world — so the layout rule is
/// readable in one place. See this module's doc for that rule stated in
/// prose.
fn plan(recipes: &RecipeRegistry, machines: &MachineRegistry) -> Vec<Order> {
    let ordered = recipes.ordered();
    let raw = raw_ingredient_ids(recipes);
    let is_raw = |id: &str| raw.iter().any(|r| r == id);
    let mut orders = Vec::new();

    let mut cursor_x = 0;
    for recipe in ordered {
        let footprint = machines
            .footprint_of(&recipe.machine)
            .unwrap_or_else(|| panic!("machine kind '{}' has no footprint", recipe.machine));
        assert!(
            footprint.height_tiles >= recipe.inputs.len() as i32,
            "machine kind '{}' is {} tile(s) deep but its recipe takes {} solid ingredients — \
             each needs its own tile of inlet face (data/machines.ron's header note)",
            recipe.machine,
            footprint.height_tiles,
            recipe.inputs.len()
        );

        // One feed lane per solid input, each ending against its own tile of
        // the inlet face. A fluid-only stage gets no lane at all.
        let belt_start_x = cursor_x;
        if !recipe.inputs.is_empty() {
            for (index, (item, _)) in recipe.inputs.iter().enumerate() {
                let lane_y = CHAIN_ROW_Y + index as i32;
                for dx in 0..STAGE_BELT_TILES {
                    orders.push(Order::east(BELT_KIND, (belt_start_x + dx, lane_y)));
                }
                // A raw ingredient has nothing upstream to fill its lane, so
                // it gets a hopper at the lane's head.
                if is_raw(item) {
                    let hopper_width = footprint_width(machines, HOPPER_KIND);
                    orders.push(Order {
                        kind: HOPPER_KIND,
                        tile: (belt_start_x - hopper_width, lane_y),
                        direction: Direction::East,
                        item: Some(leak(item)),
                    });
                }
            }
            cursor_x += STAGE_BELT_TILES;
        }

        let machine_origin = (cursor_x, CHAIN_ROW_Y);
        orders.push(Order::east(leak(&recipe.machine), machine_origin));
        cursor_x += footprint.width_tiles;

        // A raw fluid input (vinegar) needs a tank and its own run, on the
        // row immediately north of the machine's whole footprint so its far
        // end is edge-adjacent to it however deep the machine is.
        let needs_a_tank = recipe
            .fluid_inputs
            .first()
            .is_some_and(|(fluid, _)| is_raw(fluid));
        if needs_a_tank {
            let pipe_row_y = machine_origin.1 + footprint.height_tiles;
            for n in 0..PIPE_RUN_TILES {
                orders.push(Order::east(
                    PIPE_KIND,
                    (machine_origin.0 - (PIPE_RUN_TILES - 1 - n), pipe_row_y),
                ));
            }
            let run_head_x = machine_origin.0 - (PIPE_RUN_TILES - 1);
            let tank_width = footprint_width(machines, FLUID_TANK_KIND);
            orders.push(Order::east(
                FLUID_TANK_KIND,
                (run_head_x - tank_width, pipe_row_y),
            ));
        }

        // A fluid output leaves by the outlet face, so the run carrying it
        // east is what connects this stage to the next one.
        if !recipe.fluid_outputs.is_empty() {
            for n in 0..MACHINE_PIPE_RUN_TILES {
                orders.push(Order::east(PIPE_KIND, (cursor_x + n, CHAIN_ROW_Y)));
            }
            cursor_x += MACHINE_PIPE_RUN_TILES;
        }
    }

    // The exit: one more ordinary belt run past the last machine, and the
    // dock at the end of it.
    for dx in 0..STAGE_BELT_TILES {
        orders.push(Order::east(BELT_KIND, (cursor_x + dx, CHAIN_ROW_Y)));
    }
    orders.push(Order::east(DOCK_KIND, (cursor_x + STAGE_BELT_TILES, CHAIN_ROW_Y)));

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
