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
#   **A new game has `sim::STARTING_MONEY` (3000) and the shipped six-stage
#   line costs 1,087.** That relationship was the other way round until the
#   inventory-hotbar ticket raised the balance from 200 — under which a Godot
#   test genuinely could not reach the shipped layout by any sequence of
#   commands, because `Purchase` is all-or-nothing and money only arrives
#   through contract deliveries, which need the very line you could not
#   afford. `build_reference_line` below now builds the whole thing out of
#   ordinary purchase/place commands, which is both a fixture and a standing
#   proof that a new save can reach the shipped content.
#
# The SMALL fixtures stayed small anyway, and that is now a choice rather
# than a ceiling: what the Godot tests assert is the WIRE and the SCENE —
# shapes, models, rotations, ghosts, hover — and a nine-tile line shows those
# exactly as well as a forty-tile one while being far quicker to run and far
# easier to read a failure out of. Full-chain BEHAVIOUR (mass conservation,
# contract payouts, fluid consumption, footprint geometry) stays asserted
# where it always was: `cargo test -p sim_core`, against
# `sim::reference_chain`, whose world is built with exactly its own bill
# (`World::with_content_and_money`) so that it starts from a clean zero.
#
# The per-fixture bills below are quoted against the 3000 balance so it stays
# obvious how much headroom each one leaves — none of them is close to it any
# more.
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
## Its bill is 176 of a 3000 starting balance. Its SIZE, though, is decided by
## what it has to show rather than by what it costs (see this file's header):
## every additional tile is another node to build and another line of failure
## output, and none of them would add a facing this line does not already use.
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


## The multi-tile fixture: one 2x2 cooking vat and nothing else (200 of 3000).
## It stands alone because its footprint COLLIDES with the demo line's — both
## start at (0, 0) — not because of what it costs; a `place` onto an occupied
## tile is refused, so the two cannot share a world whatever the balance is.
## It is what proves a footprint bigger than one tile still renders over all
## four of its tiles, and (turned) that a quarter turn is drawn as a quarter
## turn.
static func build_multi_tile(sim, direction := "east") -> void:
	place(sim, "cooking_vat", Vector2i(0, 0), direction)


## A hopper and one belt: the smallest thing that is a working factory at all
## (45 of 3000), for tests that only need "something is placed".
static func build_minimal(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east")
	place(sim, "belt", Vector2i(0, 0), "east")


## A vinegar tank, a four-tile pipe run and the one machine whose recipe
## declares `fluid_inputs` — 192 of 3000, and everything a pipe network needs
## to be worth looking at. The mixer never actually crafts (nothing feeds it
## tomato paste, which would need the rest of the chain — see
## `build_reference_line` for the version that does), which does not matter
## for what this fixture is for: a machine TAPS a network just by touching it,
## so the run has a fluid, a source, an inlet and a level that moves.
static func build_fluid_run(sim) -> void:
	place(sim, "mixer", Vector2i(0, 0), "east")
	place(sim, "pipe", Vector2i(0, 1), "west")
	place(sim, "pipe", Vector2i(1, 1), "west")
	place(sim, "pipe", Vector2i(2, 1), "west")
	place(sim, "pipe", Vector2i(3, 1), "west")
	place(sim, "fluid_tank", Vector2i(4, 1), "west")


## Hopper -> belt -> crusher, then a belt into a delivery dock: 195 of 3000,
## and one of each SORT of thing on the floor. It exists to give the
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


## A LINE THAT FORKS (Epic B) — 130 of 3000. A hopper feeding two belts into
## a splitter that has both of its outputs built, then further along a second
## splitter with only its straight output built.
##
## The second one is the point of the fixture as much as the first: a
## splitter whose branch was never laid reports `next_output: "branch"`
## forever while every item goes straight through it (`splitter.rs`'s
## fallback rule), so it is the case where a readout driven straight off that
## field would point at bare floor. Having both in one world means the
## arrow's two behaviours can be seen side by side, in a test and in a
## screenshot.
##
## The geometry is the simulation's own rule, not a guess: a splitter's
## straight output is the tile it faces and its branch is one quarter turn
## CLOCKWISE from that, so an EAST-facing splitter at (2, 0) hands straight
## to (3, 0) and branches SOUTH to (2, -1).
static func build_splitter_line(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east")
	place(sim, "belt", Vector2i(0, 0), "east")
	place(sim, "belt", Vector2i(1, 0), "east")
	place(sim, "splitter", Vector2i(2, 0), "east")
	# The straight run on, and the branch heading south away from it.
	place(sim, "belt", Vector2i(3, 0), "east")
	place(sim, "belt", Vector2i(4, 0), "east")
	place(sim, "belt", Vector2i(2, -1), "south")
	place(sim, "belt", Vector2i(2, -2), "south")
	# The second splitter: straight output built, branch (6, -1) deliberately
	# left as bare floor.
	place(sim, "belt", Vector2i(5, 0), "east")
	place(sim, "splitter", Vector2i(6, 0), "east")
	place(sim, "belt", Vector2i(7, 0), "east")


## The aging rack on its own — 160 of 3000. Feeding it for real is now
## affordable (its recipe eats ketchup, which needs the five machines upstream
## of it; that is what `build_reference_line` is), but it would also mean
## running the chain for the thousands of ticks it takes ketchup to reach the
## rack before a single frame could be checked. A test that wants to watch the
## gauge move therefore drives the gauge's own mechanism instead, which is a
## sharper test of the thing being claimed as well as a faster one; see
## test_timed_processes.gd.
static func build_aging_rack(sim, tile := Vector2i(0, 0)) -> void:
	place(sim, "aging_rack", tile, "east")


## THE WHOLE SHIPPED LINE, bought and placed like anything else — the
## six-stage chain `sim::reference_chain` builds on the Rust side, tile for
## tile, all of it east-facing: hopper, four belts per stage into that stage's
## machine, the vinegar tank and its four-tile pipe run on the row above the
## mixer, and four last belts into the delivery dock.
##
## It costs 1,087 of a 3000 starting balance, and that is the point of it.
## Under the old 200 this fixture could not exist at all, which is why the
## screenshot flags that needed a working economy (`--contracts` and the
## aging-rack ones) were deleted rather than fixed. It is both the fixture
## those flags came back on and a standing proof, through the real wire, that
## a new save can reach the shipped content by playing rather than by a test
## seam.
##
## The coordinates mirror `sim_core/src/sim/reference.rs`'s `plan()` — which
## derives them from `data/recipes.ron` order and `data/machines.ron`
## footprints — and `sim_core/tests/machine_footprints.rs` pins the same
## literals, so a content change that moves the line fails there first, with a
## far better message than a Godot script could give.
static func build_reference_line(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east")
	for x in [0, 1, 2, 3]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "crusher", Vector2i(4, 0), "east")
	for x in [5, 6, 7, 8]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "mixer", Vector2i(9, 0), "east")
	for x in [10, 11, 12, 13]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "cooking_vat", Vector2i(14, 0), "east") # 2x2: it also covers x=15 and y=1
	for x in [16, 17, 18, 19]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "aging_rack", Vector2i(20, 0), "east")
	for x in [21, 22, 23, 24]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "bottler", Vector2i(25, 0), "east")
	for x in [26, 27, 28, 29]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "packer", Vector2i(30, 0), "east")
	for x in [31, 32, 33, 34]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "dock", Vector2i(35, 0), "east")
	# The vinegar supply: the run sits on the row north of the mixer it feeds,
	# ending beside it, with the tank one tile west of the run's head. A
	# machine taps a network by TOUCHING it, not by facing it, which is why
	# feeding the mixer from above works at all.
	place(sim, "fluid_tank", Vector2i(5, 1), "east")
	for x in [6, 7, 8, 9]:
		place(sim, "pipe", Vector2i(x, 1), "east")
