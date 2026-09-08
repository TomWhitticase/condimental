# Shared fixture: a small factory built the way a PLAYER builds one, through
# nothing but `purchase` and `place` commands over the real gdext JSON
# boundary.
#
# WHY THIS EXISTS, AND WHY IT IS SMALL. Until the "Real Placement" epic every
# world arrived with a complete six-stage chain in it, so a presentation test
# could open the scene and immediately assert things about a factory. Ticket 1
# ended that: `World::new` builds nothing, and the only way to get a belt on
# the floor is to buy it. Every headless Godot test therefore has to build
# whatever it wants to look at first — which is a better test than the old one
# (it drives `place`/`remove`/`rotate` through the same wire format the scene
# does) with one hard limit worth stating plainly:
#
#   **A new game has `sim::STARTING_MONEY` (200) and the shipped six-stage
#   line costs about 1,087.** No amount of command-issuing gets round that:
#   `Purchase` is all-or-nothing and money only arrives through contract
#   deliveries, which need the very line you cannot afford. The Rust side
#   builds that layout as `sim::reference_chain`, which starts its world with
#   exactly the money to pay for it (`World::with_content_and_money`) — a seam
#   that has no wire equivalent, deliberately, because a player has no such
#   seam either.
#
# So these fixtures are what 200 buys, and full-chain behaviour (mass
# conservation, contract payouts, fluid consumption, footprint geometry) stays
# asserted where it already was: `cargo test -p sim_core`, against
# `reference_chain`. What the Godot tests assert is the WIRE and the SCENE —
# shapes, models, rotations, ghosts, hover — which a small line shows just as
# well as a big one.
extends RefCounted


## Buys one of `kind` and places it at `tile` facing `direction`. Both
## commands go through `issue_command_json`, so a test using this is
## exercising the real boundary, not a shortcut.
static func place(sim, kind: String, tile: Vector2i, direction: String) -> void:
	sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": kind}))
	sim.issue_command_json(JSON.stringify({
		"type": "place", "kind": kind, "tile": [tile.x, tile.y], "direction": direction,
	}))


## THE DEMO LINE — the fixture most tests build, and the one
## `scripts/screenshot-scene.sh --build` photographs.
##
## A hopper feeding four belts that run east and then TURN NORTH into a
## crusher facing WEST, which discharges back west along three more belts, with
## a short pipe run heading SOUTH beside it. That shape is chosen, not
## arbitrary: it uses three of the four directions on belts and the fourth on
## the pipes, so "does a rotated tile render correctly" and "does the
## east-only junction art fall back cleanly everywhere else" are both visible
## in one frame. It also actually RUNS — the hopper unloads onto the first
## belt, the belts carry, the crusher crushes and discharges — so it is a
## working factory rather than a diorama.
##
## Its bill is 176 against a starting balance of 200 (see this file's header),
## which is the constraint that decided its size.
static func build_demo_line(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east")
	place(sim, "belt", Vector2i(0, 0), "east")
	place(sim, "belt", Vector2i(1, 0), "east")
	place(sim, "belt", Vector2i(2, 0), "east")
	place(sim, "belt", Vector2i(3, 0), "north")
	place(sim, "belt", Vector2i(3, 1), "north")
	place(sim, "crusher", Vector2i(3, 2), "west")
	place(sim, "belt", Vector2i(2, 2), "west")
	place(sim, "belt", Vector2i(1, 2), "west")
	place(sim, "belt", Vector2i(0, 2), "west")
	place(sim, "pipe", Vector2i(5, 2), "south")
	place(sim, "pipe", Vector2i(5, 1), "south")
	place(sim, "belt", Vector2i(-1, 2), "west")


## The multi-tile fixture: one 2x2 cooking vat and nothing else. It costs 200
## — the ENTIRE starting balance — which is why it cannot be combined with the
## demo line above and gets a fixture of its own. It is what proves a
## footprint bigger than one tile still renders over all four of its tiles,
## and (turned) that a quarter turn is drawn as a quarter turn.
static func build_multi_tile(sim, direction := "east") -> void:
	place(sim, "cooking_vat", Vector2i(0, 0), direction)


## A hopper and one belt: the cheapest thing that is a working factory at all
## (45 of 200), for tests that only need "something is placed".
static func build_minimal(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east")
	place(sim, "belt", Vector2i(0, 0), "east")


## A vinegar tank, a four-tile pipe run and the one machine whose recipe
## declares `fluid_inputs` — 192 of 200, and the most that can be plumbed on a
## starting balance. The mixer never actually crafts (nothing feeds it tomato
## paste, which would need the rest of the chain), which does not matter for
## what this fixture is for: a machine TAPS a network just by touching it, so
## the run has a fluid, a source, an inlet and a level that moves.
static func build_fluid_run(sim) -> void:
	place(sim, "mixer", Vector2i(0, 0), "east")
	place(sim, "pipe", Vector2i(0, 1), "west")
	place(sim, "pipe", Vector2i(1, 1), "west")
	place(sim, "pipe", Vector2i(2, 1), "west")
	place(sim, "pipe", Vector2i(3, 1), "west")
	place(sim, "fluid_tank", Vector2i(4, 1), "west")


## Hopper -> belt -> crusher, then a belt into a delivery dock: 195 of 200,
## and the most *kinds* that fit on a starting balance. It exists to give the
## per-kind model checks and the junction rule something of each sort to look
## at — a feed building, a machine, an exit building, and the two belt tiles
## that sit against a building on either side (one catching a discharge, one
## handing over).
static func build_machines_fixture(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east")
	place(sim, "belt", Vector2i(0, 0), "east")
	place(sim, "crusher", Vector2i(1, 0), "east")
	place(sim, "belt", Vector2i(4, 0), "east")
	place(sim, "dock", Vector2i(5, 0), "east")


## The aging rack on its own — 160 of 200. It cannot be *fed* on a starting
## balance (its recipe eats ketchup, which needs the five machines upstream of
## it), so a test that wants to watch its gauge move drives the gauge's own
## mechanism instead; see test_timed_processes.gd.
static func build_aging_rack(sim, tile := Vector2i(0, 0)) -> void:
	place(sim, "aging_rack", tile, "east")
