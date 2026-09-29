//! Headless integration test for Epic B's ticket 1: the conveyor
//! **splitter** — the first placement in this simulation that resolves to
//! two downstream targets instead of one.
//!
//! `sim_core/src/sim/splitter.rs` owns the two rules (where the second
//! output is, and how items are dealt between them) and unit-tests them in
//! isolation. This file drives a real `World` through the public command
//! API, to prove the things only the whole system can show:
//!
//! 1. **A fed splitter really halves a line.** Twelve units in, six down
//!    each branch — the claim "splits its output across two outputs" has to
//!    mean, and the one a cursor test on its own cannot make.
//! 2. **Mass is conserved through it**, at every tick, not just at the end.
//!    Nothing is duplicated when an item is offered to two sides and
//!    nothing is annihilated when both refuse. This is ARCHITECTURE.md §6's
//!    central invariant, checked the way `tests/core_loop.rs` checks it for
//!    the straight-line chain.
//! 3. **Both degenerate wirings behave.** One output connected feeds that
//!    one; no outputs connected holds everything and backs the line up.
//! 4. **It is an ordinary placeable kind.** Place/Remove/Rotate refuse and
//!    apply exactly as they do for every other kind, and one `rotate` turns
//!    every output together, which is the whole reason a splitter needs no
//!    second direction field.
//! 5. **Its mode picks the sides** (`left`, `right`, `t_junction`,
//!    `three_way`): each deals its input evenly across exactly its own
//!    sides, is set by `set_splitter_mode` over the wire, and survives a
//!    save.
//!
//! Everything below runs with no Godot process involved (ARCHITECTURE.md
//! §7).

mod common;

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::{Money, TileCoord};
use sim_core::sim::{
    ContractRegistry, Direction, HOPPER_CAPACITY, PlayerCommand, RecipeRegistry, SPLITTER_KIND,
    Side, SplitMode, TileSnapshot, World, command_from_json,
};

/// A one-stage recipe file: four tomatoes into one paste, on the crusher.
/// The smallest content that can carry mass end to end, so a test about how
/// a splitter *deals* items isn't also a test about the shipped six-stage
/// chain. (Lifted from `tests/placement.rs`, deliberately — a fixture that
/// small is worth having twice rather than sharing across test binaries.)
fn one_stage() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "tomato_paste",
                    inputs: [("tomato", 4)],
                    outputs: [("tomato_paste", 1)],
                    machine: "crusher",
                    time_ticks: 20,
                ),
            ],
        )"#,
    )
    .expect("test recipe text must parse")
}

/// A world with money to spend and nothing built.
fn funded() -> World {
    World::with_content_and_money(
        one_stage(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(20_000),
    )
}

fn build(world: &mut World, kind: &str, tile: TileCoord, direction: Direction) {
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item: None,
    });
    assert!(
        world.query_tile(tile).is_some(),
        "the fixture must be able to place a {kind} at {tile:?}"
    );
}

/// A run of belt tiles from `start`, each one step along `direction` from
/// the last, all facing that way.
fn build_belt_run(
    world: &mut World,
    start: TileCoord,
    direction: Direction,
    tiles: i32,
) -> Vec<TileCoord> {
    let mut laid = Vec::new();
    let mut tile = start;
    for _ in 0..tiles {
        build(world, "belt", tile, direction);
        laid.push(tile);
        tile = direction.step(tile);
    }
    laid
}

/// How many solid items are sitting on `tiles` right now — belt tiles and
/// splitter tiles alike, since a splitter carries its contents in an
/// ordinary belt segment.
fn items_on(world: &World, tiles: &[TileCoord]) -> usize {
    tiles
        .iter()
        .map(|&tile| match world.query_tile(tile) {
            Some(TileSnapshot::Belt { items_here, .. }) => items_here.len(),
            Some(TileSnapshot::Splitter { items_here, .. }) => items_here.len(),
            _ => 0,
        })
        .sum()
}

/// Everything still in the hopper.
fn in_hoppers(world: &World) -> u32 {
    world.hopper_snapshots().iter().map(|h| h.quantity).sum()
}

/// The whole T-junction this file is about: a hopper metering raw tomatoes
/// straight onto a splitter at the origin, with a three-tile run off each of
/// its two outputs.
///
/// Facing east, so the straight-through output is `(1, 0)` and the branch —
/// the default `left` mode's side, one quarter turn clockwise in the sim,
/// i.e. south — is `(0, -1)`. Returns `(straight_run, branch_run)`.
struct Junction {
    world: World,
    straight: Vec<TileCoord>,
    branch: Vec<TileCoord>,
}

impl Junction {
    /// `straight_tiles`/`branch_tiles` of 0 leaves that output unbuilt,
    /// which is how the degenerate-wiring tests below are expressed.
    fn build(straight_tiles: i32, branch_tiles: i32, tomatoes: u32) -> Self {
        let mut world = funded();
        build(&mut world, "hopper", (-1, 0), Direction::East);
        build(&mut world, SPLITTER_KIND, (0, 0), Direction::East);
        let straight = build_belt_run(&mut world, (1, 0), Direction::East, straight_tiles);
        let branch = build_belt_run(&mut world, (0, -1), Direction::South, branch_tiles);
        // Ingredients cost money since the chain-rework epic (`FillHopper`
        // now draws from bought stock rather than conjuring it) and since W1
        // they need somewhere to go as well — `common::buy` builds the racks
        // the order needs and buys comfortably more than this fixture will
        // ever fill, so the hopper fill below is limited by
        // `tomatoes`/hopper capacity, not by stock.
        common::buy(&mut world, "tomato", tomatoes.max(HOPPER_CAPACITY));
        world.issue_command(PlayerCommand::FillHopper {
            amount: tomatoes,
            tile: None,
        });
        assert_eq!(in_hoppers(&world), tomatoes);
        Self {
            world,
            straight,
            branch,
        }
    }

    /// Every tile that can be holding an item: the splitter itself plus both
    /// runs. The hopper is counted separately (it holds a count, not items).
    fn carriers(&self) -> Vec<TileCoord> {
        let mut tiles = vec![(0, 0)];
        tiles.extend(self.straight.iter().copied());
        tiles.extend(self.branch.iter().copied());
        tiles
    }

    /// Raw ingredient anywhere in the world — in the hopper, on the
    /// splitter, or on either run. Conserved, always.
    fn total(&self) -> u32 {
        in_hoppers(&self.world) + items_on(&self.world, &self.carriers()) as u32
    }
}

/// The headline claim: a splitter with both outputs clear deals its input
/// alternately, so an even number of units divides exactly in half.
#[test]
fn a_fed_splitter_deals_its_input_evenly_between_two_outputs() {
    let mut j = Junction::build(3, 3, 12);
    for _ in 0..3000 {
        j.world.tick();
    }

    let straight = items_on(&j.world, &j.straight);
    let branch = items_on(&j.world, &j.branch);
    assert_eq!(
        in_hoppers(&j.world),
        0,
        "the hopper should have emptied long before 3000 ticks"
    );
    assert_eq!(
        items_on(&j.world, &[(0, 0)]),
        0,
        "nothing should still be standing on the splitter with both branches clear"
    );
    assert_eq!(
        (straight, branch),
        (6, 6),
        "twelve units through a splitter is six down each side, not eleven and one"
    );
}

/// The same fixture, sampled *while* it runs: the count never wobbles, in
/// either direction. An item offered to two sides must never end up on both
/// (duplication), and one both sides refuse must never end up on neither
/// (annihilation) — ARCHITECTURE.md §6's invariant, at the one place in the
/// simulation where an item is offered twice.
#[test]
fn mass_is_conserved_through_a_splitter_on_every_tick() {
    let mut j = Junction::build(3, 3, 12);
    for tick in 0..3000 {
        j.world.tick();
        assert_eq!(
            j.total(),
            12,
            "twelve units in means twelve units somewhere, at tick {tick}"
        );
    }
}

/// Conservation again, in the state most likely to break it: the branch runs
/// are one tile long, so both back up almost immediately and the splitter
/// spends most of the run being refused by both sides.
#[test]
fn mass_is_conserved_when_both_outputs_are_backed_up() {
    let mut j = Junction::build(1, 1, 24);
    for tick in 0..4000 {
        j.world.tick();
        assert_eq!(
            j.total(),
            24,
            "a splitter refused by both sides must hold, not drop, at tick {tick}"
        );
    }
    assert!(
        in_hoppers(&j.world) > 0,
        "with both branches full the line must have backed all the way up to the hopper"
    );
}

/// One output connected: everything goes down it. This falls out of the
/// offer loop rather than being a special case — a side with nothing on it
/// is a side that always refuses (see `splitter.rs`'s module doc).
///
/// Eight units on a four-tile run: a jammed tile holds two items at the
/// half-tile spacing (`belt.rs`'s module doc — an item at a tile's start
/// is no longer also at the previous tile's end), so eight is exactly what
/// the run can hold with nothing left standing on the splitter.
#[test]
fn a_splitter_with_only_the_straight_output_sends_everything_down_it() {
    let mut j = Junction::build(4, 0, 8);
    for _ in 0..3000 {
        j.world.tick();
    }
    assert_eq!(items_on(&j.world, &j.straight), 8);
    assert_eq!(j.total(), 8);
}

/// The mirror of the test above, and the one that proves the fallback is a
/// fallback rather than a preference for the straight side: with only the
/// *branch* built, every unit goes down the branch even though `Straight` is
/// what a fresh splitter offers first.
#[test]
fn a_splitter_with_only_the_branch_output_sends_everything_down_it() {
    let mut j = Junction::build(0, 4, 8);
    for _ in 0..3000 {
        j.world.tick();
    }
    assert_eq!(items_on(&j.world, &j.branch), 8);
    assert_eq!(j.total(), 8);
}

/// No outputs at all: the splitter fills up, the hopper stops being able to
/// unload onto it, and nothing is lost. A splitter is never a hole in the
/// floor.
#[test]
fn a_splitter_with_no_outputs_holds_everything_and_backs_the_line_up() {
    let mut j = Junction::build(0, 0, 12);
    for tick in 0..3000 {
        j.world.tick();
        assert_eq!(j.total(), 12, "nothing may vanish, at tick {tick}");
    }
    let held = items_on(&j.world, &[(0, 0)]);
    assert!(held > 0, "the splitter should be holding what it took");
    assert_eq!(
        held as u32 + in_hoppers(&j.world),
        12,
        "everything is either still in the hopper or standing on the splitter"
    );
}

/// A splitter's outputs reach exactly the sinks a belt's hand-off does — a
/// machine's inlet face and a delivery dock — not only other belts. Here the
/// straight side feeds a crusher and the branch ships to a dock, and both
/// receive.
#[test]
fn a_splitter_feeds_a_machine_and_a_dock_the_way_a_belt_does() {
    let mut world = funded();
    build(&mut world, "hopper", (-1, 0), Direction::East);
    build(&mut world, SPLITTER_KIND, (0, 0), Direction::East);
    // Straight (east): a crusher, facing east so its inlet face is the tile
    // the splitter hands into.
    build(&mut world, "crusher", (1, 0), Direction::East);
    // Branch (clockwise from east = south): a dock. It is 2x2 and its origin
    // is its min-X/min-Y corner, so it is anchored at (0, -2) and claims
    // (0..1, -2..-1) — clear of the splitter and the crusher on row 0, with
    // the tile the branch hands into, (0, -1), on its west edge, i.e. its
    // inlet face.
    build(&mut world, "dock", (0, -2), Direction::East);
    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 16,
    });
    world.issue_command(PlayerCommand::FillHopper {
        amount: 16,
        tile: None,
    });

    for _ in 0..4000 {
        world.tick();
    }

    let shipped: u32 = world.dock_snapshots().iter().map(|d| d.shipped_total).sum();
    assert!(
        shipped > 0,
        "the branch output should have delivered raw tomatoes to the dock"
    );

    // The crusher takes four tomatoes per craft, so finished paste in its
    // output buffer is the proof the straight output fed it.
    let crusher = world
        .chunk_snapshot((0, 0))
        .machines
        .into_iter()
        .find(|m| m.tile == (1, 0))
        .expect("the crusher is still standing at (1, 0)");
    let paste: u32 = crusher
        .output
        .iter()
        .filter(|(item, _)| item == "tomato_paste")
        .map(|(_, qty)| qty)
        .sum();
    assert!(
        paste > 0,
        "the straight output should have fed the crusher enough to craft, got {:?} in / {:?} out",
        crusher.input,
        crusher.output
    );
}

/// The direction convention, end to end: `direction` is the straight-through
/// output and the branch is one quarter turn clockwise, so a single `rotate`
/// turns **both** outputs together. This is the property that means a
/// splitter needs no second direction field and no new command.
#[test]
fn rotating_a_splitter_turns_both_outputs_together() {
    let mut world = funded();
    build(&mut world, SPLITTER_KIND, (0, 0), Direction::East);
    // A belt on all four sides, each facing away, so every facing resolves
    // to a real output and nothing feeds back into the splitter.
    build(&mut world, "belt", (1, 0), Direction::East);
    build(&mut world, "belt", (0, -1), Direction::South);
    build(&mut world, "belt", (-1, 0), Direction::West);
    build(&mut world, "belt", (0, 1), Direction::North);

    let outputs_now = |world: &World| -> (Direction, [Option<TileCoord>; 3]) {
        let Some(TileSnapshot::Splitter {
            direction, outputs, ..
        }) = world.query_tile((0, 0))
        else {
            panic!("(0, 0) is the splitter");
        };
        (direction, outputs)
    };

    let expected = [
        (Direction::East, [Some((1, 0)), Some((0, -1)), None]),
        (Direction::South, [Some((0, -1)), Some((-1, 0)), None]),
        (Direction::West, [Some((-1, 0)), Some((0, 1)), None]),
        (Direction::North, [Some((0, 1)), Some((1, 0)), None]),
    ];
    for (facing, tiles) in expected {
        assert_eq!(
            outputs_now(&world),
            (facing, tiles),
            "facing {facing:?}: straight is the facing, the branch is one turn clockwise"
        );
        world.issue_command(PlayerCommand::Rotate { tile: (0, 0) });
    }
    // Four turns is a full circle, back where it started.
    assert_eq!(outputs_now(&world).0, Direction::East);
}

/// Rotating actually re-routes product, not just the reported geometry: the
/// same fixture, turned once, sends its straight-through units where the
/// branch used to go.
#[test]
fn rotating_a_splitter_reroutes_what_it_carries() {
    // Six: what a jammed three-tile run holds at two items a tile.
    let mut j = Junction::build(3, 3, 6);
    // One clockwise turn: east becomes south, so what was the branch run is
    // now the straight-through and what was the straight run is neither.
    j.world
        .issue_command(PlayerCommand::Rotate { tile: (0, 0) });
    // The hopper still points east into the splitter, so it still feeds it.
    for _ in 0..3000 {
        j.world.tick();
    }
    assert_eq!(
        items_on(&j.world, &j.straight),
        0,
        "the east run is on neither output of a south-facing splitter"
    );
    assert_eq!(
        items_on(&j.world, &j.branch),
        6,
        "a south-facing splitter's straight-through output is the south run"
    );
    assert_eq!(j.total(), 6);
}

/// Place/Remove refuse and apply exactly as they do for every other kind
/// (`PlayerCommand`'s universal "an impossible command is a silent, total
/// no-op").
#[test]
fn placing_a_splitter_follows_every_other_kinds_rules() {
    let mut world = funded();

    // Owning none: a total no-op.
    world.issue_command(PlayerCommand::Place {
        kind: SPLITTER_KIND.to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
    });
    assert!(world.query_tile((0, 0)).is_none());
    assert_eq!(world.placement_count(), 0);

    // Buying one, placing it: money went at purchase time, the owned unit
    // goes at placement time.
    world.issue_command(PlayerCommand::Purchase {
        kind: SPLITTER_KIND.to_string(),
        quantity: 2,
    });
    assert_eq!(world.owned_count(SPLITTER_KIND), 2);
    world.issue_command(PlayerCommand::Place {
        kind: SPLITTER_KIND.to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
    });
    assert_eq!(world.owned_count(SPLITTER_KIND), 1);
    assert_eq!(world.placement_count(), 1);

    // Overlapping an occupied tile: refused whole, and the owned unit is
    // NOT spent.
    world.issue_command(PlayerCommand::Place {
        kind: SPLITTER_KIND.to_string(),
        tile: (0, 0),
        direction: Direction::North,
        item: None,
    });
    assert_eq!(world.owned_count(SPLITTER_KIND), 1);
    assert_eq!(world.placement_count(), 1);
    let Some(TileSnapshot::Splitter { direction, .. }) = world.query_tile((0, 0)) else {
        panic!("(0, 0) is still the original splitter");
    };
    assert_eq!(
        direction,
        Direction::East,
        "a refused placement must not have re-faced the thing already there"
    );

    // A splitter blocks anything else being placed on it, too.
    world.issue_command(PlayerCommand::Purchase {
        kind: "belt".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "belt".to_string(),
        tile: (0, 0),
        direction: Direction::East,
        item: None,
    });
    assert_eq!(world.owned_count("belt"), 1);

    // Remove credits one back and clears the tile.
    world.issue_command(PlayerCommand::Remove { tile: (0, 0) });
    assert_eq!(world.owned_count(SPLITTER_KIND), 2);
    assert_eq!(world.placement_count(), 0);
    assert!(world.query_tile((0, 0)).is_none());

    // Removing and rotating an empty tile are no-ops, not panics.
    world.issue_command(PlayerCommand::Remove { tile: (0, 0) });
    world.issue_command(PlayerCommand::Rotate { tile: (0, 0) });
    assert_eq!(world.owned_count(SPLITTER_KIND), 2);
    assert_eq!(world.placement_count(), 0);
}

/// Removing a splitter destroys what it was holding — `PlayerCommand::Remove`'s
/// documented behaviour for every kind, restated here because a splitter is
/// the newest thing that can be holding something.
#[test]
fn removing_a_loaded_splitter_takes_its_contents_with_it() {
    let mut j = Junction::build(0, 0, 6);
    for _ in 0..600 {
        j.world.tick();
    }
    assert!(items_on(&j.world, &[(0, 0)]) > 0);
    j.world
        .issue_command(PlayerCommand::Remove { tile: (0, 0) });
    assert!(j.world.query_tile((0, 0)).is_none());
    assert_eq!(
        items_on(&j.world, &j.carriers()),
        0,
        "what a removed splitter held goes with it, like every other placement"
    );
}

/// The wire form ticket 2 (presentation) will read: a splitter appears in
/// its chunk's snapshot with the documented fields, and `query_tile` reports
/// the same facts.
#[test]
fn the_splitter_wire_form_has_the_documented_shape() {
    let j = Junction::build(2, 2, 4);
    let value =
        serde_json::to_value(j.world.chunk_snapshot((0, 0))).expect("a chunk snapshot serializes");
    let splitters = value["splitters"]
        .as_array()
        .expect("a chunk snapshot has a splitters array");
    assert_eq!(splitters.len(), 1);

    let entry = splitters[0].as_object().expect("each entry is an object");
    let mut fields: Vec<&str> = entry.keys().map(|k| k.as_str()).collect();
    fields.sort();
    assert_eq!(
        fields,
        vec![
            "direction",
            "footprint",
            "items",
            "kind",
            "mode",
            "moved_ticks",
            "next_output",
            "outputs",
            "sides",
            "tile",
        ],
        "SplitterSnapshot's documented fields"
    );
    assert_eq!(entry["kind"], "splitter");
    assert_eq!(entry["direction"], "east");
    assert_eq!(entry["tile"], serde_json::json!([0, 0]));
    assert_eq!(
        entry["outputs"],
        serde_json::json!([[1, 0], [0, -1], null]),
        "straight, left, right — and a fresh splitter deals only to the first two"
    );
    assert_eq!(entry["mode"], "left", "a fresh splitter splits left");
    assert_eq!(
        entry["sides"],
        serde_json::json!(["straight", "left"]),
        "the sides reported are the mode's own, in dealing order"
    );
    assert_eq!(
        entry["next_output"], "straight",
        "a fresh splitter offers its first item straight through"
    );

    // The same placement is also in `world_layout`, with no recipe — it is a
    // building that moves product, not one that changes it.
    let placement = j
        .world
        .world_layout()
        .into_iter()
        .find(|p| p.kind == SPLITTER_KIND)
        .expect("the splitter is in the world layout");
    assert_eq!(placement.tile, (0, 0));
    assert_eq!(placement.direction, Direction::East);
    assert_eq!(placement.occupies, vec![(0, 0)]);
    assert!(placement.recipe.is_none());
    assert_eq!(placement.split_mode, Some(SplitMode::Left));

    let tile = serde_json::to_value(j.world.query_tile((0, 0)).expect("occupied"))
        .expect("a tile snapshot serializes");
    assert_eq!(tile["kind"], "splitter");
    assert_eq!(tile["next_output"], "straight");
    assert_eq!(tile["outputs"], serde_json::json!([[1, 0], [0, -1], null]));
    assert_eq!(tile["mode"], "left");
    assert_eq!(tile["sides"], serde_json::json!(["straight", "left"]));
}

/// A splitter is placed by the same JSON command every other kind is — no
/// new command type, which was one of this ticket's constraints.
#[test]
fn a_splitter_is_placed_by_the_ordinary_place_command_over_the_wire() {
    let mut world = funded();
    for json in [
        r#"{"type": "purchase", "kind": "splitter", "quantity": 1}"#,
        r#"{"type": "place", "kind": "splitter", "tile": [3, 2], "direction": "south"}"#,
    ] {
        let cmd = command_from_json(json).expect("the command parses");
        world.issue_command(cmd);
    }
    let Some(TileSnapshot::Splitter {
        direction,
        next_output,
        ..
    }) = world.query_tile((3, 2))
    else {
        panic!("the splitter was placed at (3, 2)");
    };
    assert_eq!(direction, Direction::South);
    assert_eq!(next_output, Side::Straight);
}

/// Two splitters in series quarter a line — the composition case, and a
/// check that a splitter counts as a valid output of another splitter
/// (it is belt-like, so it does).
#[test]
fn splitters_compose_into_a_four_way_split() {
    let mut world = funded();
    build(&mut world, "hopper", (-1, 0), Direction::East);
    // First splitter: straight to (1, 0), branch south to (0, -1).
    build(&mut world, SPLITTER_KIND, (0, 0), Direction::East);
    // Second splitter directly on the first's straight output: straight to
    // (2, 0), branch south to (1, -1).
    build(&mut world, SPLITTER_KIND, (1, 0), Direction::East);

    let a = build_belt_run(&mut world, (2, 0), Direction::East, 3);
    let b = build_belt_run(&mut world, (1, -1), Direction::South, 3);
    let c = build_belt_run(&mut world, (0, -1), Direction::South, 3);

    world.issue_command(PlayerCommand::BuyIngredient {
        item: "tomato".to_string(),
        quantity: 12,
    });
    world.issue_command(PlayerCommand::FillHopper {
        amount: 12,
        tile: None,
    });
    for _ in 0..5000 {
        world.tick();
    }

    let (na, nb, nc) = (
        items_on(&world, &a),
        items_on(&world, &b),
        items_on(&world, &c),
    );
    assert_eq!(
        na + nb + nc,
        12,
        "every unit arrived somewhere: {na} / {nb} / {nc}"
    );
    // The first splitter halves (6 down each side); the second halves its
    // half (3 and 3).
    assert_eq!((na, nb, nc), (3, 3, 6));
}

/// A splitter at the origin facing east with a three-tile run off every one
/// of its three sides — straight `(1, 0)` east, left `(0, -1)` south, right
/// `(0, 1)` north — fed by a hopper from the west, in `mode`.
struct Cross {
    world: World,
    straight: Vec<TileCoord>,
    left: Vec<TileCoord>,
    right: Vec<TileCoord>,
}

impl Cross {
    fn build(mode: &str, tomatoes: u32) -> Self {
        let mut world = funded();
        build(&mut world, "hopper", (-1, 0), Direction::East);
        build(&mut world, SPLITTER_KIND, (0, 0), Direction::East);
        let straight = build_belt_run(&mut world, (1, 0), Direction::East, 3);
        let left = build_belt_run(&mut world, (0, -1), Direction::South, 3);
        let right = build_belt_run(&mut world, (0, 1), Direction::North, 3);
        let command =
            format!(r#"{{"type": "set_splitter_mode", "tile": [0, 0], "mode": "{mode}"}}"#);
        world.issue_command(command_from_json(&command).expect("the command parses"));
        common::buy(&mut world, "tomato", tomatoes.max(HOPPER_CAPACITY));
        world.issue_command(PlayerCommand::FillHopper {
            amount: tomatoes,
            tile: None,
        });
        Self {
            world,
            straight,
            left,
            right,
        }
    }

    fn counts(&self) -> (usize, usize, usize) {
        (
            items_on(&self.world, &self.straight),
            items_on(&self.world, &self.left),
            items_on(&self.world, &self.right),
        )
    }
}

/// The headline claim of the modes: each one deals its input evenly across
/// exactly the sides it names, and nothing at all down a side it does not —
/// even though every side has a belt ready to take it.
#[test]
fn each_mode_deals_evenly_across_exactly_its_own_sides() {
    for (mode, expected) in [
        ("left", (6, 6, 0)),
        ("right", (6, 0, 6)),
        ("t_junction", (0, 6, 6)),
        ("three_way", (4, 4, 4)),
    ] {
        let mut cross = Cross::build(mode, 12);
        for _ in 0..3000 {
            cross.world.tick();
        }
        assert_eq!(
            cross.counts(),
            expected,
            "mode {mode}: (straight, left, right)"
        );
    }
}

/// `set_splitter_mode` is a real player command: it changes the mode the
/// wire reports and the outputs it lists, and it is a silent no-op on a
/// tile that is not a splitter.
#[test]
fn set_splitter_mode_changes_the_reported_mode_and_outputs() {
    let mut cross = Cross::build("t_junction", 0);
    let Some(TileSnapshot::Splitter {
        mode,
        outputs,
        sides,
        next_output,
        links,
        ..
    }) = cross.world.query_tile((0, 0))
    else {
        panic!("(0, 0) is the splitter");
    };
    assert_eq!(mode, SplitMode::TJunction);
    assert_eq!(outputs, [None, Some((0, -1)), Some((0, 1))]);
    assert_eq!(sides, vec![Side::Left, Side::Right]);
    assert_eq!(
        next_output,
        Side::Left,
        "a T never leaves straight due — it does not deal straight on"
    );
    assert!(
        links.output.is_none(),
        "the wire's single `output` is the straight side, which a T does not have"
    );
    let placement = cross
        .world
        .world_layout()
        .into_iter()
        .find(|p| p.kind == SPLITTER_KIND)
        .expect("the splitter is in the layout");
    assert_eq!(placement.split_mode, Some(SplitMode::TJunction));

    cross.world.issue_command(PlayerCommand::SetSplitterMode {
        tile: (0, 0),
        mode: SplitMode::ThreeWay,
    });
    let Some(TileSnapshot::Splitter { mode, outputs, .. }) = cross.world.query_tile((0, 0)) else {
        panic!("(0, 0) is the splitter");
    };
    assert_eq!(mode, SplitMode::ThreeWay);
    assert_eq!(outputs, [Some((1, 0)), Some((0, -1)), Some((0, 1))]);

    // A belt is not a splitter, and an empty tile is nothing: both are
    // silent no-ops that change nothing and panic nowhere.
    let before = cross.world.to_save_ron();
    cross.world.issue_command(PlayerCommand::SetSplitterMode {
        tile: (1, 0),
        mode: SplitMode::Right,
    });
    cross.world.issue_command(PlayerCommand::SetSplitterMode {
        tile: (40, 40),
        mode: SplitMode::Right,
    });
    assert_eq!(cross.world.to_save_ron(), before);
}

/// Switching mode mid-flow moves nothing that is already on the splitter
/// and loses nothing: mass is conserved across the switch.
#[test]
fn switching_mode_under_load_conserves_every_unit() {
    let mut cross = Cross::build("left", 12);
    for _ in 0..200 {
        cross.world.tick();
    }
    cross.world.issue_command(PlayerCommand::SetSplitterMode {
        tile: (0, 0),
        mode: SplitMode::Right,
    });
    for _ in 0..3000 {
        cross.world.tick();
    }
    let (straight, left, right) = cross.counts();
    assert_eq!(straight + left + right, 12, "{straight} / {left} / {right}");
    assert!(left > 0 && right > 0, "both modes dealt to their side");
}

/// A splitter's mode and its cursor survive a save: a three-way splitter
/// part-way round its rotation comes back three-way and due on the same
/// side, and carries on dealing exactly as the unsaved one does.
#[test]
fn a_splitters_mode_survives_a_save() {
    let mut cross = Cross::build("three_way", 12);
    for _ in 0..400 {
        cross.world.tick();
    }
    let text = cross.world.to_save_ron();
    assert!(text.contains("mode: three_way"), "the mode is in the file");
    let mut loaded = World::from_save_ron(&text, one_stage()).expect("the save loads");
    let (
        Some(TileSnapshot::Splitter {
            mode,
            next_output,
            outputs,
            ..
        }),
        Some(original),
    ) = (loaded.query_tile((0, 0)), cross.world.query_tile((0, 0)))
    else {
        panic!("(0, 0) is the splitter in both worlds");
    };
    let TileSnapshot::Splitter {
        next_output: original_next,
        ..
    } = original
    else {
        unreachable!()
    };
    assert_eq!(mode, SplitMode::ThreeWay);
    assert_eq!(next_output, original_next);
    assert_eq!(outputs, [Some((1, 0)), Some((0, -1)), Some((0, 1))]);
    for _ in 0..3000 {
        cross.world.tick();
        loaded.tick();
    }
    let after_load = (
        items_on(&loaded, &cross.straight),
        items_on(&loaded, &cross.left),
        items_on(&loaded, &cross.right),
    );
    assert_eq!(after_load, cross.counts());
    assert_eq!(after_load, (4, 4, 4));
}
