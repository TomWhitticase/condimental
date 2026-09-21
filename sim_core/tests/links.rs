//! Headless integration test for the Pipes & Conveyors Overhaul's ticket 1:
//! **connectivity on the wire** (`docs/PIPES_AND_CONVEYORS_PLAN.md`, "The
//! wire contract").
//!
//! Every belt, splitter and pipe placement in `world_layout` now carries a
//! `links` object saying what the topology pass joined it to and on which
//! side, and the matching `query_tile` variants carry the same object. The
//! point of this file is that those links are **the sim's own** —
//! `World::recompute_topology` records them at the moment it resolves each
//! hand-off, tap and pipe adjacency — so a renderer drawing a corner, a
//! tee or a port fitting from them is drawing where product actually goes.
//! Nothing here ticks: links are a property of what stands where.
//!
//! The shapes are pinned byte-for-byte at the end, because two Godot
//! tickets read these keys verbatim.
//!
//! Everything below runs with no Godot process involved (ARCHITECTURE.md
//! §7).

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, ConveyorLinks, Direction, LinkInput, LinkKind, LinkOutput, Links,
    PipeConnection, PipeLinks, PlacementSnapshot, PlayerCommand, RecipeRegistry, TileSnapshot,
    World, reference_chain,
};

use Direction::{East, North, South, West};
use LinkKind::{Belt, Dock, Hopper, Machine, Pipe, Splitter, Tank};

fn shipped_recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A world with money to spend and nothing built. Links are about geometry,
/// so the shipped content is used everywhere: it has a 1x1 machine, a 1x2
/// one and a 2x2 one, machines with fluid inputs, fluid outputs, both and
/// neither — every case the wire has to describe.
fn funded() -> World {
    World::with_content_and_money(
        shipped_recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    )
}

/// Buys one and places it, asserting it landed — a silently refused
/// placement would otherwise show up as a missing link much later.
fn build(world: &mut World, kind: &str, tile: (i32, i32), direction: Direction) {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item: (kind == "hopper").then(|| "tomato".to_string()),
    });
    assert!(
        world.query_tile(tile).is_some(),
        "{kind} must be standing at {tile:?} after being placed"
    );
}

fn lay_belt(world: &mut World, from: (i32, i32), direction: Direction, count: i32) {
    let mut tile = from;
    for _ in 0..count {
        build(world, "belt", tile, direction);
        tile = direction.step(tile);
    }
}

fn placement_at(world: &World, tile: (i32, i32)) -> PlacementSnapshot {
    world
        .world_layout()
        .into_iter()
        .find(|p| p.occupies.contains(&tile))
        .unwrap_or_else(|| panic!("nothing is placed at {tile:?}"))
}

/// The `links` of the belt or splitter at `tile`.
fn conveyor(world: &World, tile: (i32, i32)) -> ConveyorLinks {
    match placement_at(world, tile).links {
        Some(Links::Conveyor(links)) => links,
        other => panic!("expected conveyor links at {tile:?}, got {other:?}"),
    }
}

/// The `links` of the pipe at `tile`.
fn pipe(world: &World, tile: (i32, i32)) -> PipeLinks {
    match placement_at(world, tile).links {
        Some(Links::Pipe(links)) => links,
        other => panic!("expected pipe links at {tile:?}, got {other:?}"),
    }
}

fn input(from: Direction, kind: LinkKind) -> LinkInput {
    LinkInput { from, kind }
}

fn output(to: Direction, kind: LinkKind) -> Option<LinkOutput> {
    Some(LinkOutput { to, kind })
}

fn conn(to: Direction, kind: LinkKind) -> PipeConnection {
    PipeConnection { to, kind }
}

fn conveyor_links(inputs: Vec<LinkInput>, output: Option<LinkOutput>) -> ConveyorLinks {
    ConveyorLinks { inputs, output }
}

fn pipe_links(connections: Vec<PipeConnection>) -> PipeLinks {
    PipeLinks { connections }
}

// ---------------------------------------------------------------------
// Belts.
// ---------------------------------------------------------------------

/// A straight run: each tile is fed from behind and hands to the tile
/// ahead; the head has no input and the tail has no output.
#[test]
fn a_straight_line_links_each_tile_to_the_one_behind_and_the_one_ahead() {
    let mut world = funded();
    lay_belt(&mut world, (0, 0), East, 3);

    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (1, 0)),
        conveyor_links(vec![input(West, Belt)], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (2, 0)),
        conveyor_links(vec![input(West, Belt)], None)
    );
}

/// A lonely belt has links too — empty ones — so a consumer never has to
/// special-case "no links object" on a belt.
#[test]
fn a_belt_with_nothing_around_it_has_empty_links() {
    let mut world = funded();
    build(&mut world, "belt", (0, 0), North);
    assert_eq!(conveyor(&world, (0, 0)), conveyor_links(vec![], None));
}

/// The plan's corner rule: a single input from a flank is a corner. For a
/// tile facing east, a feed from the **north** side is the plan's
/// `belt_corner_left` (product turns left with +X ahead), from the south
/// side `belt_corner_right`. The wire only says which side; the naming is
/// pinned here so tickets 2 and 5 read it the same way.
#[test]
fn a_belt_fed_from_a_flank_reports_that_flank() {
    // Left corner: (0, 1) faces south into (0, 0), which faces east.
    let mut world = funded();
    build(&mut world, "belt", (0, 1), South);
    build(&mut world, "belt", (0, 0), East);
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![input(North, Belt)], None)
    );
    assert_eq!(
        conveyor(&world, (0, 1)),
        conveyor_links(vec![], output(South, Belt))
    );

    // Right corner: (0, -1) faces north into (0, 0).
    let mut world = funded();
    build(&mut world, "belt", (0, -1), North);
    build(&mut world, "belt", (0, 0), East);
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![input(South, Belt)], None)
    );

    // The same two corners on a north-facing tile: west is its left flank
    // and east its right, so the absolute side names still resolve.
    let mut world = funded();
    build(&mut world, "belt", (-1, 0), East);
    build(&mut world, "belt", (0, 0), North);
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![input(West, Belt)], None)
    );
    let mut world = funded();
    build(&mut world, "belt", (1, 0), West);
    build(&mut world, "belt", (0, 0), North);
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![input(East, Belt)], None)
    );
}

/// A merge — fed from behind *and* a flank — lists both, in side order
/// (north, east, south, west), whatever order they were placed in.
#[test]
fn a_merge_lists_every_feed_in_side_order() {
    let mut world = funded();
    build(&mut world, "belt", (1, 0), East);
    build(&mut world, "belt", (0, 0), East); // behind
    build(&mut world, "belt", (1, 1), South); // north flank
    build(&mut world, "belt", (1, -1), North); // south flank
    assert_eq!(
        conveyor(&world, (1, 0)),
        conveyor_links(
            vec![input(North, Belt), input(South, Belt), input(West, Belt)],
            None
        )
    );

    // Placement order does not change the wire: the same merge built the
    // other way round reads identically.
    let mut reversed = funded();
    build(&mut reversed, "belt", (1, -1), North);
    build(&mut reversed, "belt", (1, 1), South);
    build(&mut reversed, "belt", (0, 0), East);
    build(&mut reversed, "belt", (1, 0), East);
    assert_eq!(conveyor(&reversed, (1, 0)), conveyor(&world, (1, 0)));
}

/// A belt pointing at another belt's *front* still links: a belt tile
/// accepts from any side, so a head-on pair each feed the other. The wire
/// reports it exactly as the sim links it.
#[test]
fn belts_accept_from_any_side_including_head_on() {
    let mut world = funded();
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "belt", (1, 0), West);
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![input(East, Belt)], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (1, 0)),
        conveyor_links(vec![input(West, Belt)], output(West, Belt))
    );
}

// ---------------------------------------------------------------------
// Hoppers, machines, docks.
// ---------------------------------------------------------------------

/// A hopper discharges onto the belt off its outlet face, on every facing,
/// and the belt names the hopper on the side it stands.
#[test]
fn a_hopper_feeds_the_belt_off_its_outlet_on_every_facing() {
    for facing in Direction::ALL {
        let mut world = funded();
        let belt_tile = facing.step((0, 0));
        build(&mut world, "hopper", (0, 0), facing);
        build(&mut world, "belt", belt_tile, facing);
        assert_eq!(
            conveyor(&world, belt_tile),
            conveyor_links(vec![input(facing.opposite(), Hopper)], None),
            "hopper facing {facing:?}"
        );
        // The hopper itself carries no links object.
        assert_eq!(placement_at(&world, (0, 0)).links, None);
    }
}

/// A belt on a hopper's flank is not fed by it.
#[test]
fn a_belt_beside_a_hopper_is_not_fed_by_it() {
    let mut world = funded();
    build(&mut world, "hopper", (0, 0), East);
    build(&mut world, "belt", (0, 1), East);
    assert_eq!(conveyor(&world, (0, 1)), conveyor_links(vec![], None));
}

/// A belt into a machine's inlet face hands to it; the machine discharges
/// onto the belt off its outlet face; a belt into a tile that is **not on
/// the inlet edge** does nothing and says so.
#[test]
fn a_machine_takes_from_its_inlet_and_gives_to_its_outlet() {
    let mut world = funded();
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "crusher", (1, 0), East);
    build(&mut world, "belt", (2, 0), East);

    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![], output(East, Machine))
    );
    assert_eq!(
        conveyor(&world, (2, 0)),
        conveyor_links(vec![input(West, Machine)], None)
    );
    assert_eq!(placement_at(&world, (1, 0)).links, None);

    // The vat is 2x2, so it has tiles that are on no inlet edge: a belt
    // pointing into one of those is linked to nothing.
    let mut world = funded();
    build(&mut world, "cooking_vat", (10, 0), East); // inlet edge is x = 10
    build(&mut world, "belt", (11, 2), South); // into (11, 1), the far column
    build(&mut world, "belt", (12, 0), West); // into (11, 0) through the outlet face
    assert_eq!(conveyor(&world, (11, 2)), conveyor_links(vec![], None));
    // (12, 0) hands to nothing — but it stands off the outlet face, and a
    // belt takes from any side, so the vat discharges onto it.
    assert_eq!(
        conveyor(&world, (12, 0)),
        conveyor_links(vec![input(West, Machine)], None)
    );
}

/// **The sim's inlet rule as it stands, pinned so the wire is read
/// correctly: a belt is linked into a machine or dock when the tile it
/// points at is on the inlet EDGE, from whichever side it approaches.**
/// For a 1x1 machine every side is that edge, so a belt entering a crusher
/// from its flank feeds it (`godot_project/test_layouts.gd`'s demo line
/// builds exactly that: a north-going belt into a west-facing crusher),
/// and a belt entering a dock's inlet-edge tile from the north feeds the
/// dock. `world.rs`'s module doc says a flank accepts nothing; the code
/// checks the tile, not the approach face. Reported for the overhaul's
/// ticket 7 rather than changed here — this ticket adds a readout and
/// must not move product. When that rule changes, these expectations
/// change with it, which is the point of pinning them.
#[test]
fn a_belt_into_an_inlet_edge_tile_from_the_side_is_linked_as_the_sim_links_it() {
    let mut world = funded();
    build(&mut world, "crusher", (1, 0), East);
    build(&mut world, "belt", (1, 1), South); // into the 1x1 crusher's north side
    build(&mut world, "belt", (2, 0), West); // into its outlet face, even
    assert_eq!(
        conveyor(&world, (1, 1)),
        conveyor_links(vec![], output(South, Machine))
    );
    // ...and that belt is also the one the crusher discharges onto, so the
    // wire shows a belt that both feeds and is fed by the same machine.
    assert_eq!(
        conveyor(&world, (2, 0)),
        conveyor_links(vec![input(West, Machine)], output(West, Machine))
    );

    let mut world = funded();
    build(&mut world, "dock", (1, 0), East); // inlet edge x = 1, y in 0..2
    build(&mut world, "belt", (1, 2), South); // into (1, 1) from the north
    assert_eq!(
        conveyor(&world, (1, 2)),
        conveyor_links(vec![], output(South, Dock))
    );
}

/// The 1x2 mixer accepts on both tiles of its inlet face and discharges
/// off the first belt on its outlet face only: the second outlet belt is
/// not fed (the sim hands one unit per tick to one belt), and the wire
/// says exactly that rather than drawing a junction the item never uses.
#[test]
fn a_multi_tile_inlet_face_takes_on_every_tile_but_gives_to_one_belt() {
    let mut world = funded();
    build(&mut world, "mixer", (1, 0), East); // covers (1,0) and (1,1)
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "belt", (0, 1), East);
    build(&mut world, "belt", (2, 0), East);
    build(&mut world, "belt", (2, 1), East);

    assert_eq!(conveyor(&world, (0, 0)).output, output(East, Machine));
    assert_eq!(conveyor(&world, (0, 1)).output, output(East, Machine));
    assert_eq!(conveyor(&world, (2, 0)).inputs, vec![input(West, Machine)]);
    assert_eq!(conveyor(&world, (2, 1)).inputs, vec![]);
}

/// A belt into the dock's inlet face hands to it; a belt into a tile off
/// the inlet edge does not. The dock is 2x2, so both inlet tiles are
/// checked.
#[test]
fn a_belt_into_a_dock_hands_to_it() {
    let mut world = funded();
    build(&mut world, "dock", (1, 0), East); // inlet face is x = 1, y in 0..2
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "belt", (0, 1), East);
    build(&mut world, "belt", (2, 2), South); // into (2, 1), the far column

    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![], output(East, Dock))
    );
    assert_eq!(
        conveyor(&world, (0, 1)),
        conveyor_links(vec![], output(East, Dock))
    );
    assert_eq!(conveyor(&world, (2, 2)), conveyor_links(vec![], None));
    assert_eq!(placement_at(&world, (1, 0)).links, None);
}

// ---------------------------------------------------------------------
// Splitters.
// ---------------------------------------------------------------------

/// A splitter's `output` is its straight side; its branch (one quarter
/// turn clockwise) stays in `outputs`, and both receiving belts name the
/// splitter on the side it stands.
#[test]
fn a_splitter_reports_its_straight_output_and_feeds_both_sides() {
    let mut world = funded();
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "splitter", (1, 0), East);
    build(&mut world, "belt", (2, 0), East); // straight
    build(&mut world, "belt", (1, -1), South); // branch: east turned clockwise is south

    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![], output(East, Splitter))
    );
    assert_eq!(
        conveyor(&world, (1, 0)),
        conveyor_links(vec![input(West, Belt)], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (2, 0)),
        conveyor_links(vec![input(West, Splitter)], None)
    );
    assert_eq!(
        conveyor(&world, (1, -1)),
        conveyor_links(vec![input(North, Splitter)], None)
    );

    // The existing field is untouched.
    match world.query_tile((1, 0)) {
        Some(TileSnapshot::Splitter { outputs, links, .. }) => {
            assert_eq!(outputs, [Some((2, 0)), Some((1, -1))]);
            assert_eq!(links, conveyor(&world, (1, 0)));
        }
        other => panic!("expected a splitter at (1, 0), got {other:?}"),
    }
}

/// A splitter whose straight side is empty reports `output: null` even
/// though its branch is built — the wire's `output` is the straight side
/// and nothing else.
#[test]
fn a_splitter_with_only_a_branch_has_no_straight_output() {
    let mut world = funded();
    build(&mut world, "splitter", (1, 0), East);
    build(&mut world, "belt", (1, -1), South);
    assert_eq!(conveyor(&world, (1, 0)), conveyor_links(vec![], None));
    assert_eq!(
        conveyor(&world, (1, -1)),
        conveyor_links(vec![input(North, Splitter)], None)
    );
}

// ---------------------------------------------------------------------
// Pipes.
// ---------------------------------------------------------------------

/// The five pipe shapes the plan names, all in one network: a cross at
/// the centre of a plus, a straight on a two-tile arm, an end at each tip,
/// plus a corner and a tee grown off one arm.
#[test]
fn pipe_shapes_end_straight_corner_tee_and_cross() {
    let mut world = funded();
    // A plus centred on (5, 5) with two-tile arms.
    for arm in Direction::ALL {
        build(&mut world, "pipe", arm.step((5, 5)), East);
        build(&mut world, "pipe", arm.step(arm.step((5, 5))), East);
    }
    build(&mut world, "pipe", (5, 5), East);

    // Cross.
    assert_eq!(
        pipe(&world, (5, 5)),
        pipe_links(vec![
            conn(North, Pipe),
            conn(East, Pipe),
            conn(South, Pipe),
            conn(West, Pipe)
        ])
    );
    // Straight, on the east arm and the north arm.
    assert_eq!(
        pipe(&world, (6, 5)),
        pipe_links(vec![conn(East, Pipe), conn(West, Pipe)])
    );
    assert_eq!(
        pipe(&world, (5, 6)),
        pipe_links(vec![conn(North, Pipe), conn(South, Pipe)])
    );
    // Ends.
    assert_eq!(pipe(&world, (7, 5)), pipe_links(vec![conn(West, Pipe)]));
    assert_eq!(pipe(&world, (5, 7)), pipe_links(vec![conn(South, Pipe)]));
    assert_eq!(pipe(&world, (3, 5)), pipe_links(vec![conn(East, Pipe)]));
    assert_eq!(pipe(&world, (5, 3)), pipe_links(vec![conn(North, Pipe)]));

    // Grow a corner off the east tip: (7, 5) -> (7, 6) turns the tip into
    // a corner and (7, 6) into a new end.
    build(&mut world, "pipe", (7, 6), East);
    assert_eq!(
        pipe(&world, (7, 5)),
        pipe_links(vec![conn(North, Pipe), conn(West, Pipe)])
    );
    assert_eq!(pipe(&world, (7, 6)), pipe_links(vec![conn(South, Pipe)]));

    // And a tee: a stub south of the east arm's straight tile.
    build(&mut world, "pipe", (6, 4), East);
    assert_eq!(
        pipe(&world, (6, 5)),
        pipe_links(vec![conn(East, Pipe), conn(South, Pipe), conn(West, Pipe)])
    );

    // It is still one network.
    assert_eq!(world.fluid_snapshots().len(), 1);
}

/// Two pipe tiles touching only at a corner are not adjacent, are not one
/// network, and are not linked. A lonely pipe has an empty list.
#[test]
fn pipes_that_only_touch_diagonally_are_not_linked() {
    let mut world = funded();
    build(&mut world, "pipe", (0, 0), East);
    build(&mut world, "pipe", (1, 1), East);
    assert_eq!(pipe(&world, (0, 0)), pipe_links(vec![]));
    assert_eq!(pipe(&world, (1, 1)), pipe_links(vec![]));
    assert_eq!(world.fluid_snapshots().len(), 2);
}

/// A pipe's own `direction` is presentation only, and the links prove it:
/// tiles facing every which way still join on the sides they touch.
#[test]
fn a_pipe_tile_links_regardless_of_the_direction_it_was_placed_at() {
    let mut world = funded();
    build(&mut world, "pipe", (0, 0), North);
    build(&mut world, "pipe", (1, 0), South);
    build(&mut world, "pipe", (2, 0), West);
    assert_eq!(
        pipe(&world, (1, 0)),
        pipe_links(vec![conn(East, Pipe), conn(West, Pipe)])
    );
}

/// A tank discharges into the pipe off its outlet face, on every facing;
/// the pipe names the tank on the side it stands. A pipe beside the
/// tank's flank is not connected to it.
#[test]
fn a_tank_connects_to_the_pipe_off_its_outlet_on_every_facing() {
    for facing in Direction::ALL {
        let mut world = funded();
        let pipe_tile = facing.step((0, 0));
        build(&mut world, "fluid_tank", (0, 0), facing);
        build(&mut world, "pipe", pipe_tile, East);
        build(&mut world, "pipe", facing.rotate_cw().step((0, 0)), East); // on a flank
        assert_eq!(
            pipe(&world, pipe_tile),
            pipe_links(vec![conn(facing.opposite(), Tank)]),
            "tank facing {facing:?}"
        );
        assert_eq!(
            pipe(&world, facing.rotate_cw().step((0, 0))),
            pipe_links(vec![])
        );
        assert_eq!(placement_at(&world, (0, 0)).links, None);
    }
}

/// The mixer draws its vinegar from a pipe **touching** its footprint and
/// pushes its mix into the pipe off its outlet **face**. Both are reported
/// on the pipe tile the sim actually taps — and only that tile: a run laid
/// along the whole outlet face connects on its first tile, and a run
/// touching a machine with no fluid inputs at all connects nowhere.
#[test]
fn a_machine_tap_and_a_machine_producer_are_reported_on_the_tapped_tile() {
    let mut world = funded();
    build(&mut world, "mixer", (5, 0), East); // covers (5,0) and (5,1)
    // The tank at the head of the touching run is REQUIRED since the BBQ
    // epic, and it is required for a reason worth stating. A `mixer` now has
    // two recipes — ketchup mix draws vinegar, BBQ mix draws finished
    // ketchup — so a bare pipe touching a bare mixer could be carrying
    // either, and `resolve_network_fluids` leaves a run it cannot narrow to
    // ONE possibility unnamed (never a panic; the machines on it simply
    // stall). A tank-fed run is intersected with the RAW fluids, and
    // ketchup is not raw, so the tank is what collapses the pair to vinegar.
    // That is the real disambiguator in a real factory too: what you plumb
    // into a machine is what decides what it makes.
    build(&mut world, "fluid_tank", (4, 2), East);
    build(&mut world, "pipe", (5, 2), East); // touches the north tile: the vinegar tap
    build(&mut world, "pipe", (6, 0), East); // off the outlet face: the product run
    build(&mut world, "pipe", (6, 1), East); // also off the outlet face, second tile

    assert_eq!(
        pipe(&world, (5, 2)),
        pipe_links(vec![conn(South, Machine), conn(West, Tank)])
    );
    assert_eq!(
        pipe(&world, (6, 0)),
        pipe_links(vec![conn(North, Pipe), conn(West, Machine)])
    );
    assert_eq!(pipe(&world, (6, 1)), pipe_links(vec![conn(South, Pipe)]));

    // The same facts the older snapshots already carry, so they agree.
    let networks = world.fluid_snapshots();
    let vinegar = networks
        .iter()
        .find(|n| n.fluid.as_deref() == Some("vinegar"))
        .expect("the touching run is claimed as vinegar");
    assert_eq!(vinegar.taps.len(), 1);
    let mix = networks
        .iter()
        .find(|n| n.fluid.as_deref() == Some("ketchup_mix"))
        .expect("the faced run is claimed as ketchup mix");
    assert_eq!(mix.producers.len(), 1);
    assert_eq!(mix.producers[0].segment_index, 0);

    // A machine with no fluid in its recipe connects to nothing, however
    // close the pipe is.
    let mut world = funded();
    build(&mut world, "crusher", (0, 0), East);
    build(&mut world, "pipe", (1, 0), East);
    build(&mut world, "pipe", (0, 1), East);
    assert_eq!(pipe(&world, (1, 0)), pipe_links(vec![]));
    assert_eq!(pipe(&world, (0, 1)), pipe_links(vec![]));
}

/// The 2x2 vat, tapped on a far corner of its footprint and facing south,
/// so the "any tile, any side" input rule and the rotated outlet face are
/// both exercised on a machine bigger than one tile.
#[test]
fn a_big_rotated_machine_is_tapped_on_the_touching_side_and_produces_off_its_face() {
    let mut world = funded();
    build(&mut world, "cooking_vat", (10, 10), South); // covers (10..12, 10..12)
    build(&mut world, "pipe", (12, 11), East); // east of the north-east tile
    build(&mut world, "pipe", (11, 9), East); // south of the south-east tile: outlet

    assert_eq!(
        pipe(&world, (12, 11)),
        pipe_links(vec![conn(West, Machine)])
    );
    assert_eq!(
        pipe(&world, (11, 9)),
        pipe_links(vec![conn(North, Machine)])
    );
}

/// A loop of pipe wrapped around a machine on both sides is read as its
/// outlet only (world.rs's module doc): the wire shows one machine
/// connection, on the outlet side, and none on the touching side.
#[test]
fn a_run_touching_a_machine_on_both_sides_connects_as_its_outlet_only() {
    let mut world = funded();
    build(&mut world, "mixer", (1, 0), East); // covers (1,0) and (1,1)
    // A U of pipe from the outlet face round the north side to the inlet.
    for tile in [(2, 0), (2, 1), (2, 2), (1, 2), (0, 2), (0, 1), (0, 0)] {
        build(&mut world, "pipe", tile, East);
    }
    let machine_connections: Vec<((i32, i32), Direction)> = world
        .world_layout()
        .into_iter()
        .filter_map(|p| match p.links {
            Some(Links::Pipe(links)) => Some((p.tile, links)),
            _ => None,
        })
        .flat_map(|(tile, links)| {
            links
                .connections
                .into_iter()
                .filter(|c| c.kind == Machine)
                .map(move |c| (tile, c.to))
        })
        .collect();
    assert_eq!(machine_connections, vec![((2, 0), West)]);
}

// ---------------------------------------------------------------------
// Links follow the world.
// ---------------------------------------------------------------------

/// Removing the middle of a run breaks both neighbours' links; putting it
/// back restores them.
#[test]
fn links_update_on_remove() {
    let mut world = funded();
    lay_belt(&mut world, (0, 0), East, 3);
    world.issue_command(PlayerCommand::Remove { tile: (1, 0) });

    assert_eq!(conveyor(&world, (0, 0)), conveyor_links(vec![], None));
    assert_eq!(conveyor(&world, (2, 0)), conveyor_links(vec![], None));

    build(&mut world, "belt", (1, 0), East);
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (2, 0)),
        conveyor_links(vec![input(West, Belt)], None)
    );
}

/// Rotating a belt moves its output and its neighbours' inputs with it,
/// and rotating a machine moves which belts it takes from and gives to.
#[test]
fn links_update_on_rotate() {
    let mut world = funded();
    lay_belt(&mut world, (0, 0), East, 3);

    // East -> South: (1, 0) now points at empty floor.
    world.issue_command(PlayerCommand::Rotate { tile: (1, 0) });
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (1, 0)),
        conveyor_links(vec![input(West, Belt)], None)
    );
    assert_eq!(conveyor(&world, (2, 0)), conveyor_links(vec![], None));

    // South -> West: (1, 0) now points back at (0, 0).
    world.issue_command(PlayerCommand::Rotate { tile: (1, 0) });
    assert_eq!(
        conveyor(&world, (0, 0)),
        conveyor_links(vec![input(East, Belt)], output(East, Belt))
    );
    assert_eq!(
        conveyor(&world, (1, 0)),
        conveyor_links(vec![input(West, Belt)], output(West, Belt))
    );

    // A machine: the 2x2 vat facing east with a belt into its inlet edge
    // and a belt off its outlet edge, then turned to face south — the old
    // inlet belt now points at a tile off the new inlet edge, and the old
    // outlet belt is no longer fed.
    let mut world = funded();
    build(&mut world, "belt", (9, 0), East);
    build(&mut world, "cooking_vat", (10, 0), East); // covers (10..12, 0..2)
    build(&mut world, "belt", (12, 0), East);
    world.issue_command(PlayerCommand::Rotate { tile: (10, 0) }); // east -> south
    assert_eq!(conveyor(&world, (9, 0)), conveyor_links(vec![], None));
    assert_eq!(conveyor(&world, (12, 0)), conveyor_links(vec![], None));
    build(&mut world, "belt", (10, 2), South); // into the new (north) inlet
    build(&mut world, "belt", (11, -1), South); // off the new (south) outlet
    assert_eq!(
        conveyor(&world, (10, 2)),
        conveyor_links(vec![], output(South, Machine))
    );
    assert_eq!(
        conveyor(&world, (11, -1)),
        conveyor_links(vec![input(North, Machine)], None)
    );
}

/// Rotating a tank re-points its discharge, and the pipes say so.
#[test]
fn pipe_links_update_when_a_tank_turns() {
    let mut world = funded();
    build(&mut world, "fluid_tank", (0, 0), East);
    build(&mut world, "pipe", (1, 0), East);
    build(&mut world, "pipe", (0, -1), East);
    assert_eq!(pipe(&world, (1, 0)), pipe_links(vec![conn(West, Tank)]));
    assert_eq!(pipe(&world, (0, -1)), pipe_links(vec![]));

    world.issue_command(PlayerCommand::Rotate { tile: (0, 0) }); // east -> south
    assert_eq!(pipe(&world, (1, 0)), pipe_links(vec![]));
    assert_eq!(pipe(&world, (0, -1)), pipe_links(vec![conn(North, Tank)]));
}

// ---------------------------------------------------------------------
// The shipped chain, and the invariant every link obeys.
// ---------------------------------------------------------------------

/// Every link is mirrored by what stands on the other side of it. Run over
/// the whole reference chain: a belt's output is the neighbour's input from
/// the opposite side and vice versa; every pipe-to-pipe connection appears
/// from both ends; a hopper, machine, dock or tank named on a side is what
/// `query_tile` finds there.
#[test]
fn every_link_in_the_reference_chain_is_mirrored_by_its_neighbour() {
    let world = reference_chain(shipped_recipes());
    let layout = world.world_layout();
    let at = |tile: (i32, i32)| layout.iter().find(|p| p.occupies.contains(&tile));
    let kind_at = |tile: (i32, i32)| -> Option<LinkKind> {
        Some(match world.query_tile(tile)? {
            TileSnapshot::Belt { .. } => Belt,
            TileSnapshot::Splitter { .. } => Splitter,
            TileSnapshot::Machine { .. } => Machine,
            TileSnapshot::Pipe { .. } => Pipe,
            TileSnapshot::Hopper { .. } => Hopper,
            TileSnapshot::FluidTank { .. } => Tank,
            TileSnapshot::Dock { .. } => Dock,
            // A rack is never wired to anything (sprint requirement R6), so
            // the reference chain has none and this arm is unreachable here.
            // It is spelled out rather than wildcarded so that the day a
            // rack *does* grow a link, this test stops compiling instead of
            // quietly mis-reporting one.
            TileSnapshot::WarehouseRack { .. } => {
                unreachable!("the reference chain places no warehouse rack")
            }
        })
    };

    let mut belt_like = 0;
    let mut pipes = 0;
    let mut machine_ports = 0;
    let mut tank_ports = 0;
    for placement in &layout {
        match &placement.links {
            None => assert!(
                !matches!(placement.kind.as_str(), "belt" | "splitter" | "pipe"),
                "{} at {:?} must carry links",
                placement.kind,
                placement.tile
            ),
            Some(Links::Conveyor(links)) => {
                belt_like += 1;
                let tile = placement.tile;
                for input in &links.inputs {
                    let neighbour = input.from.step(tile);
                    assert_eq!(kind_at(neighbour), Some(input.kind), "input at {tile:?}");
                    if input.kind == Belt {
                        let theirs = at(neighbour).and_then(|p| p.links.clone());
                        assert_eq!(
                            theirs,
                            Some(Links::Conveyor(ConveyorLinks {
                                inputs: match at(neighbour).unwrap().links.clone() {
                                    Some(Links::Conveyor(l)) => l.inputs,
                                    _ => unreachable!(),
                                },
                                output: output(input.from.opposite(), Belt),
                            })),
                            "the belt feeding {tile:?} must hand to it"
                        );
                    }
                }
                if let Some(output) = &links.output {
                    let neighbour = output.to.step(tile);
                    assert_eq!(kind_at(neighbour), Some(output.kind), "output at {tile:?}");
                    if output.kind == Belt {
                        let theirs = match at(neighbour).unwrap().links.clone() {
                            Some(Links::Conveyor(l)) => l,
                            _ => unreachable!(),
                        };
                        assert!(
                            theirs.inputs.contains(&input(output.to.opposite(), Belt)),
                            "the belt {tile:?} hands to must list it as an input"
                        );
                    }
                }
            }
            Some(Links::Pipe(links)) => {
                pipes += 1;
                let tile = placement.tile;
                for connection in &links.connections {
                    let neighbour = connection.to.step(tile);
                    assert_eq!(
                        kind_at(neighbour),
                        Some(connection.kind),
                        "pipe at {tile:?}"
                    );
                    match connection.kind {
                        Pipe => {
                            let theirs = match at(neighbour).unwrap().links.clone() {
                                Some(Links::Pipe(l)) => l,
                                _ => unreachable!(),
                            };
                            assert!(
                                theirs
                                    .connections
                                    .contains(&conn(connection.to.opposite(), Pipe))
                            );
                        }
                        Machine => machine_ports += 1,
                        Tank => tank_ports += 1,
                        other => panic!("a pipe cannot connect to a {other:?}"),
                    }
                }
            }
        }
    }
    assert!(
        belt_like > 0 && pipes > 0,
        "the reference chain has belts and pipes"
    );
    // Three machines draw (mixer, vat, bottler) and two deposit (mixer,
    // vat); one tank feeds the vinegar run.
    assert_eq!(machine_ports, 5);
    assert_eq!(tank_ports, 1);
}

// ---------------------------------------------------------------------
// The wire, byte for byte.
// ---------------------------------------------------------------------

/// The documented JSON shape, exactly: `links` last, present on a belt and
/// a pipe, **absent** (not null) on a hopper, a machine and a tank, and
/// inside it the documented keys in the documented order.
#[test]
fn world_layout_json_matches_the_documented_shape_byte_for_byte() {
    let mut world = funded();
    build(&mut world, "hopper", (-1, 0), East);
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "crusher", (1, 0), East);
    build(&mut world, "fluid_tank", (0, 1), East);
    build(&mut world, "pipe", (1, 1), East);
    build(&mut world, "pipe", (2, 1), East);

    let json = serde_json::to_string(&world.world_layout()).unwrap();
    let expected = concat!(
        "[",
        r#"{"kind":"hopper","tile":[-1,0],"direction":"east","footprint":{"width_tiles":1,"height_tiles":1},"occupies":[[-1,0]],"recipe":null,"crewed":true}"#,
        ",",
        r#"{"kind":"belt","tile":[0,0],"direction":"east","footprint":{"width_tiles":1,"height_tiles":1},"occupies":[[0,0]],"recipe":null,"crewed":true,"links":{"inputs":[{"from":"west","kind":"hopper"}],"output":{"to":"east","kind":"machine"}}}"#,
        ",",
        r#"{"kind":"crusher","tile":[1,0],"direction":"east","footprint":{"width_tiles":1,"height_tiles":1},"occupies":[[1,0]],"recipe":"tomato_paste","crewed":true}"#,
        ",",
        r#"{"kind":"fluid_tank","tile":[0,1],"direction":"east","footprint":{"width_tiles":1,"height_tiles":1},"occupies":[[0,1]],"recipe":null,"crewed":true}"#,
        ",",
        r#"{"kind":"pipe","tile":[1,1],"direction":"east","footprint":{"width_tiles":1,"height_tiles":1},"occupies":[[1,1]],"recipe":null,"crewed":true,"links":{"connections":[{"to":"east","kind":"pipe"},{"to":"west","kind":"tank"}]}}"#,
        ",",
        r#"{"kind":"pipe","tile":[2,1],"direction":"east","footprint":{"width_tiles":1,"height_tiles":1},"occupies":[[2,1]],"recipe":null,"crewed":true,"links":{"connections":[{"to":"west","kind":"pipe"}]}}"#,
        "]",
    );
    assert_eq!(json, expected);
}

/// A belt with nothing around it still has the object, with the documented
/// empty values — `[]` and `null`, never a missing key.
#[test]
fn a_lonely_belt_and_a_lonely_pipe_serialize_their_empty_links() {
    let mut world = funded();
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "pipe", (5, 5), East);
    let json = serde_json::to_string(&world.world_layout()).unwrap();
    assert!(
        json.contains(r#""links":{"inputs":[],"output":null}"#),
        "{json}"
    );
    assert!(json.contains(r#""links":{"connections":[]}"#), "{json}");
}

/// Kinds without links serialize exactly as before the field existed: the
/// key is not there at all. Every such kind in one world.
#[test]
fn kinds_without_links_do_not_mention_the_key() {
    let mut world = funded();
    build(&mut world, "hopper", (0, 0), East);
    build(&mut world, "crusher", (2, 0), East);
    build(&mut world, "mixer", (4, 0), East);
    build(&mut world, "cooking_vat", (6, 0), East);
    build(&mut world, "fluid_tank", (9, 0), East);
    build(&mut world, "dock", (11, 0), East);
    let json = serde_json::to_string(&world.world_layout()).unwrap();
    assert!(!json.contains("links"), "{json}");
    for tile in [(0, 0), (2, 0), (4, 0), (6, 0), (9, 0), (11, 0)] {
        let tile_json = serde_json::to_string(&world.query_tile(tile).unwrap()).unwrap();
        assert!(!tile_json.contains("links"), "{tile_json}");
    }
}

/// `query_tile` carries the same object under the same key on the three
/// kinds that have one, so hover needs no second call.
#[test]
fn query_tile_json_carries_links_on_belts_splitters_and_pipes() {
    let mut world = funded();
    build(&mut world, "belt", (0, 0), East);
    build(&mut world, "splitter", (1, 0), East);
    build(&mut world, "belt", (2, 0), East);
    build(&mut world, "pipe", (0, 1), East);
    build(&mut world, "pipe", (1, 1), East);

    let belt = serde_json::to_string(&world.query_tile((0, 0)).unwrap()).unwrap();
    assert_eq!(
        belt,
        r#"{"kind":"belt","direction":"east","items_here":[],"links":{"inputs":[],"output":{"to":"east","kind":"splitter"}}}"#
    );
    let splitter = serde_json::to_string(&world.query_tile((1, 0)).unwrap()).unwrap();
    assert_eq!(
        splitter,
        r#"{"kind":"splitter","direction":"east","items_here":[],"outputs":[[2,0],null],"next_output":"straight","links":{"inputs":[{"from":"west","kind":"belt"}],"output":{"to":"east","kind":"belt"}}}"#
    );
    let pipe = serde_json::to_string(&world.query_tile((0, 1)).unwrap()).unwrap();
    assert_eq!(
        pipe,
        r#"{"kind":"pipe","direction":"east","fluid":null,"naming":"unplumbed","candidates":[],"volume":0.0,"capacity":10.0,"pressure":0.0,"is_source":false,"is_inlet":false,"links":{"connections":[{"to":"east","kind":"pipe"}]}}"#
    );

    // And it is the same object `world_layout` reports.
    match world.query_tile((2, 0)) {
        Some(TileSnapshot::Belt { links, .. }) => assert_eq!(links, conveyor(&world, (2, 0))),
        other => panic!("expected a belt, got {other:?}"),
    }
}
