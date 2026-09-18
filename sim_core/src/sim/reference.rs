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
//! # A fixture is one line, not the whole file
//!
//! [`reference_chain`] lays out **one linear chain**: a single row of
//! machines, west to east, each stage consuming what the stage before it
//! made. That is all it has ever been able to express, and until the BBQ epic
//! nobody had to say so, because `data/recipes.ron` held exactly one such
//! line and "the chain" and "the file" were the same thing.
//!
//! They are not any more. `recipes.ron` is a production **graph**, and the
//! BBQ epic hangs a second sauce off the middle of the ketchup line. A branch
//! has no west-to-east reading, so generalising this layout rule to cover one
//! does not produce a harder-to-read fixture — it produces a *wrong* one. The
//! first thing it did, when four BBQ recipes were appended to the file and
//! nothing else changed, was park the BBQ mixer's hopper on top of the
//! ketchup packer, and take roughly a hundred and forty integration tests
//! down with it.
//!
//! So the rule is now stated rather than inferred. [`linear_prefix`] says
//! which recipes this fixture is for — the first entry and every following
//! entry that continues it — and **each branching sauce gets a fixture of its
//! own**: [`bbq_branch`] is the second one, drawn by hand, and the sauce
//! after it will be the third. A fixture encodes a *layout*, and there is no
//! general layout rule for an arbitrary graph.
//!
//! Both fixtures are pinned tile by tile by the tests at the bottom of this
//! file, because everything else in `sim_core/tests/` is measured against
//! their coordinates.
//!
//! # The layout rule it reproduces
//!
//! Everything sits on one row (`CHAIN_ROW_Y`), west to east, in
//! [`linear_prefix`] order, and each stage lays down only what its recipe
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
use super::recipe::{Recipe, RecipeRegistry};
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
    build(recipes, machines, contracts, Money::from_num(budget), plan)
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
    build(recipes, machines, contracts, Money::ZERO, plan)
}

/// The one builder all three entry points share: price the plan, construct a
/// world holding exactly that bill plus `budget`, buy everything, place
/// everything, and check that nothing was refused.
fn build(
    recipes: RecipeRegistry,
    machines: MachineRegistry,
    contracts: ContractRegistry,
    budget: Money,
    layout: fn(&RecipeRegistry, &MachineRegistry) -> Vec<Order>,
) -> World {
    // The same cross-file check `World`'s own construction makes, run first
    // so a recipe naming an undefined machine kind fails with `World`'s
    // message rather than with this module's less informative one — the
    // fixture must not change what a content error looks like.
    if let Err(e) = machines.validate_covers(&recipes) {
        panic!("machine content error: {e}");
    }
    let orders = layout(&recipes, &machines);

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
        "a fixture must place every piece it bought — a refused Place means the layout \
         below collided with itself or asked for something unbuildable"
    );
    assert_eq!(
        world.money(),
        budget,
        "a fixture is priced to spend exactly its bill, leaving only the budget"
    );
    world
}

/// The recipes this fixture lays out: the **linear prefix** of
/// `data/recipes.ron` — the first entry, and every following entry that
/// consumes something the entry before it produces. It stops at the first
/// entry that does not continue the line.
///
/// This exists because `recipes.ron` stopped being a single line. Until the
/// BBQ epic every entry chained into the next, so "the whole file" and "the
/// line" were the same set and this function would have been an identity.
/// BBQ hangs a branch off the middle of the ketchup line, and a branch is
/// not something a one-row, west-to-east layout rule can express: laid out
/// linearly its first stage's hopper lands on the previous stage's machine.
///
/// So the rule is *stated* rather than discovered, and it is stated in terms
/// of the data rather than of any recipe id — no content id appears in this
/// file. On the shipped content it selects the five ketchup stages and stops
/// at `bbq_mix`, whose inputs (spice mix, molasses and finished ketchup)
/// have nothing to do with what the packer before it produces.
///
/// The branch gets [`bbq_branch`] instead, and the next sauce after it will
/// get its own fixture too. That is the honest shape: a fixture encodes a
/// *layout*, and there is no general layout rule for an arbitrary graph.
pub fn linear_prefix(recipes: &RecipeRegistry) -> &[Recipe] {
    let ordered = recipes.ordered();
    let mut end = 0;
    for (index, recipe) in ordered.iter().enumerate() {
        if index > 0 {
            let previous = &ordered[index - 1];
            let produced = |id: &str| {
                previous.outputs.iter().any(|(out, _)| out == id)
                    || previous.fluid_outputs.iter().any(|(out, _)| out == id)
            };
            let continues = recipe.inputs.iter().any(|(item, _)| produced(item))
                || recipe.fluid_inputs.iter().any(|(fluid, _)| produced(fluid));
            if !continues {
                break;
            }
        }
        end = index + 1;
    }
    &ordered[..end]
}

/// Every piece of the reference chain and where it goes, in the order it is
/// placed. Pure arithmetic — it touches no world — so the layout rule is
/// readable in one place. See this module's doc for that rule stated in
/// prose.
fn plan(recipes: &RecipeRegistry, machines: &MachineRegistry) -> Vec<Order> {
    let ordered = linear_prefix(recipes);
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
    orders.push(Order::east(
        DOCK_KIND,
        (cursor_x + STAGE_BELT_TILES, CHAIN_ROW_Y),
    ));

    orders
}

/// The **BBQ branch**: the reference chain, plus the second sauce hanging off
/// the middle of it — the fixture that proves a `boxed_bbq` can be made and
/// shipped by a player who only ever issues `Purchase` and `Place`.
///
/// It is a separate fixture rather than an extension of the layout rule in
/// [`plan`], and that is the whole point. The reference chain is *one linear
/// chain* laid out west to east; BBQ is a **branch**, and a branch has no
/// west-to-east reading. Trying to express one by generalising the linear rule
/// produces a layout nobody drew and nobody can read — the first thing it did,
/// when the BBQ recipes were merely appended to `data/recipes.ron`, was park
/// the BBQ mixer's hopper on top of the ketchup packer. So the branch is drawn
/// by hand, once, here, and the next sauce gets its own fixture too.
///
/// ```text
///   y=-3   [spice]--belts-->[ mixer ]==bbq_mix==>[ BBQ ]==bbq==>[bottler]--belts-->[packer]--belts-->[dock]
///   y=-2   [molas]--belts-->[ (1x2) ]            [ vat ]
///   y=-1                         ^ketchup tap             [====liquid smoke====]<-[tank]
///   y= 0   ...the shipped ketchup line, untouched: vat -> pipes -> bottler -> packer -> dock...
/// ```
///
/// Three joints are worth reading twice, because each is a place where a tile
/// moved one square would silently break the branch:
///
/// 1. **The ketchup tap is a single pipe** on the row between the two lines,
///    edge-adjacent to the ketchup vat's outlet run below it and to the BBQ
///    mixer's footprint above it. That one tile is the branch: it joins the
///    ketchup network, and the mixer draws finished ketchup out of it. It sits
///    on the mixer's *north* face, never its outlet face, so the mixer cannot
///    mistake it for somewhere to deposit.
/// 2. **The liquid-smoke run approaches the vat from the east half of the row
///    above it**, not the west half, because a pipe on the west half would be
///    edge-adjacent to the ketchup run on the chain row and would merge liquid
///    smoke into the ketchup network. Belts on that row are harmless — only
///    pipes join pipes — which is why the run happily crosses over the
///    ketchup line's exit belts.
/// 3. **The branch gets its own dock.** Every dock deposits into the same
///    shared delivery pool (`economy.rs`), so a second one is bookkeeping-free
///    and far clearer than merging two product streams onto one belt.
///
/// Nothing on the ketchup line moves. `the_bbq_branch_layout_is_pinned` in
/// this file holds both fixtures to an exact coordinate list so that stays
/// true.
pub fn bbq_branch(recipes: RecipeRegistry, budget: u32) -> World {
    build(
        recipes,
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(budget),
        bbq_plan,
    )
}

/// The row the BBQ branch's machines sit on. Three north of the chain row,
/// which is the closest the branch can get: the chain row is two tiles deep
/// (the mixer and the vat are), and the row between the two lines carries the
/// ketchup tap and the liquid-smoke run.
const BBQ_ROW_Y: i32 = -3;
/// The row between the two lines — the ketchup tap and the liquid-smoke run.
const BBQ_TAP_ROW_Y: i32 = -1;

/// [`plan`] for the branch: the ketchup line as it always was, then the BBQ
/// stages drawn out tile by tile. See [`bbq_branch`] for the picture and for
/// why this is a hand-drawn table rather than a rule.
fn bbq_plan(recipes: &RecipeRegistry, machines: &MachineRegistry) -> Vec<Order> {
    let mut orders = plan(recipes, machines);
    let row = BBQ_ROW_Y;

    // Two raw solids, one lane each, exactly as the ketchup mixer is fed --
    // and for the same reason `data/machines.ron` gives for the mixer being
    // 1x2: one hopper holds one item, and neither may crowd the other off a
    // shared belt.
    for (lane, item) in [(0, "spice_mix"), (1, "molasses")] {
        let lane_y = row + lane;
        orders.push(Order {
            kind: HOPPER_KIND,
            tile: (9, lane_y),
            direction: Direction::East,
            item: Some(item),
        });
        for dx in 0..STAGE_BELT_TILES {
            orders.push(Order::east(BELT_KIND, (10 + dx, lane_y)));
        }
    }
    orders.push(Order::east("mixer", (14, row)));

    // The branch itself: one pipe, touching the ketchup run below it and the
    // mixer above it. See [`bbq_branch`], joint 1.
    orders.push(Order::east(PIPE_KIND, (14, BBQ_TAP_ROW_Y)));

    // mixer -> vat, off the mixer's outlet face.
    for n in 0..MACHINE_PIPE_RUN_TILES {
        orders.push(Order::east(PIPE_KIND, (15 + n, row)));
    }
    orders.push(Order::east("cooking_vat", (17, row)));

    // Liquid smoke: the reference chain's own tank arrangement -- a
    // `PIPE_RUN_TILES` run on the row north of the machine's footprint with
    // the tank one tile west of its head, facing east into it -- shifted
    // three tiles east. See [`bbq_branch`], joint 2: unshifted, the run's
    // head would land beside the ketchup line's outlet pipes and merge
    // liquid smoke into the ketchup network.
    orders.push(Order::east(FLUID_TANK_KIND, (16, BBQ_TAP_ROW_Y)));
    for n in 0..PIPE_RUN_TILES {
        orders.push(Order::east(PIPE_KIND, (17 + n, BBQ_TAP_ROW_Y)));
    }

    // vat -> bottler, off the vat's outlet face, then the ordinary solid tail.
    for n in 0..MACHINE_PIPE_RUN_TILES {
        orders.push(Order::east(PIPE_KIND, (19 + n, row)));
    }
    orders.push(Order::east("bottler", (21, row)));
    for dx in 0..STAGE_BELT_TILES {
        orders.push(Order::east(BELT_KIND, (22 + dx, row)));
    }
    orders.push(Order::east("packer", (26, row)));
    for dx in 0..STAGE_BELT_TILES {
        orders.push(Order::east(BELT_KIND, (27 + dx, row)));
    }
    orders.push(Order::east(DOCK_KIND, (31, row)));

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every placement of a fixture, as `(origin x, origin y, kind)`, sorted
    /// north-to-south then west-to-east — a stable reading of the layout that
    /// does not depend on the order `plan` happens to emit orders in.
    fn layout_of(world: &World) -> Vec<(i32, i32, String)> {
        let mut tiles: Vec<(i32, i32, String)> = world
            .world_layout()
            .iter()
            .map(|placement| {
                let origin = placement.occupies[0];
                (origin.0, origin.1, placement.kind.clone())
            })
            .collect();
        tiles.sort_by(|a, b| (a.1, a.0, &a.2).cmp(&(b.1, b.0, &b.2)));
        tiles
    }

    fn assert_layout(world: &World, expected: &[(i32, i32, &str)], fixture: &str) {
        let actual = layout_of(world);
        let expected: Vec<(i32, i32, String)> = expected
            .iter()
            .map(|&(x, y, kind)| (x, y, kind.to_string()))
            .collect();
        assert_eq!(
            actual.len(),
            expected.len(),
            "{fixture} places {} things, not {}",
            expected.len(),
            actual.len()
        );
        assert_eq!(
            actual, expected,
            "{fixture}'s layout moved. This list is the pin, not a record: roughly every \
             integration test in sim_core/tests/ measures something against these exact tiles, \
             and a content change that shifts them shifts all of those at once — which is how \
             appending the BBQ recipes to data/recipes.ron first parked a hopper on top of the \
             ketchup packer. If the move is deliberate, say so by updating the list; if it is \
             not, the change that caused it is the bug."
        );
    }

    /// The shipped line, tile by tile. It is where it has been since the
    /// placement epic, and `bbq_branch` must leave it there.
    #[test]
    fn the_reference_chain_layout_is_pinned() {
        let recipes = RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("data/recipes.ron must parse");
        assert_layout(
            &reference_chain(recipes),
            &[
                (-1, 0, "hopper"),
                (0, 0, "belt"),
                (1, 0, "belt"),
                (2, 0, "belt"),
                (3, 0, "belt"),
                (4, 0, "crusher"),
                (5, 0, "belt"),
                (6, 0, "belt"),
                (7, 0, "belt"),
                (8, 0, "belt"),
                (9, 0, "mixer"),
                (10, 0, "pipe"),
                (11, 0, "pipe"),
                (12, 0, "cooking_vat"),
                (14, 0, "pipe"),
                (15, 0, "pipe"),
                (16, 0, "bottler"),
                (17, 0, "belt"),
                (18, 0, "belt"),
                (19, 0, "belt"),
                (20, 0, "belt"),
                (21, 0, "packer"),
                (22, 0, "belt"),
                (23, 0, "belt"),
                (24, 0, "belt"),
                (25, 0, "belt"),
                (26, 0, "dock"),
                (4, 1, "hopper"),
                (5, 1, "belt"),
                (6, 1, "belt"),
                (7, 1, "belt"),
                (8, 1, "belt"),
                (5, 2, "fluid_tank"),
                (6, 2, "pipe"),
                (7, 2, "pipe"),
                (8, 2, "pipe"),
                (9, 2, "pipe"),
            ],
            "reference_chain",
        );
    }

    /// The branch fixture, tile by tile — the shipped line above, unmoved,
    /// plus the BBQ line three rows north of it and the two runs between
    /// them. See [`bbq_branch`] for which of these tiles are load-bearing and
    /// why.
    #[test]
    fn the_bbq_branch_layout_is_pinned() {
        let recipes = RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("data/recipes.ron must parse");
        let mut expected: Vec<(i32, i32, &str)> = vec![
            (9, -3, "hopper"),
            (10, -3, "belt"),
            (11, -3, "belt"),
            (12, -3, "belt"),
            (13, -3, "belt"),
            (14, -3, "mixer"),
            (15, -3, "pipe"),
            (16, -3, "pipe"),
            (17, -3, "cooking_vat"),
            (19, -3, "pipe"),
            (20, -3, "pipe"),
            (21, -3, "bottler"),
            (22, -3, "belt"),
            (23, -3, "belt"),
            (24, -3, "belt"),
            (25, -3, "belt"),
            (26, -3, "packer"),
            (27, -3, "belt"),
            (28, -3, "belt"),
            (29, -3, "belt"),
            (30, -3, "belt"),
            (31, -3, "dock"),
            (9, -2, "hopper"),
            (10, -2, "belt"),
            (11, -2, "belt"),
            (12, -2, "belt"),
            (13, -2, "belt"),
            (14, -1, "pipe"),
            (16, -1, "fluid_tank"),
            (17, -1, "pipe"),
            (18, -1, "pipe"),
            (19, -1, "pipe"),
            (20, -1, "pipe"),
        ];
        expected.extend_from_slice(&[
            (-1, 0, "hopper"),
            (0, 0, "belt"),
            (1, 0, "belt"),
            (2, 0, "belt"),
            (3, 0, "belt"),
            (4, 0, "crusher"),
            (5, 0, "belt"),
            (6, 0, "belt"),
            (7, 0, "belt"),
            (8, 0, "belt"),
            (9, 0, "mixer"),
            (10, 0, "pipe"),
            (11, 0, "pipe"),
            (12, 0, "cooking_vat"),
            (14, 0, "pipe"),
            (15, 0, "pipe"),
            (16, 0, "bottler"),
            (17, 0, "belt"),
            (18, 0, "belt"),
            (19, 0, "belt"),
            (20, 0, "belt"),
            (21, 0, "packer"),
            (22, 0, "belt"),
            (23, 0, "belt"),
            (24, 0, "belt"),
            (25, 0, "belt"),
            (26, 0, "dock"),
            (4, 1, "hopper"),
            (5, 1, "belt"),
            (6, 1, "belt"),
            (7, 1, "belt"),
            (8, 1, "belt"),
            (5, 2, "fluid_tank"),
            (6, 2, "pipe"),
            (7, 2, "pipe"),
            (8, 2, "pipe"),
            (9, 2, "pipe"),
        ]);
        assert_layout(&bbq_branch(recipes, 0), &expected, "bbq_branch");
    }
}
