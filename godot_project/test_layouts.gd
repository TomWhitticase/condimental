# Shared fixture: a small factory built the way a PLAYER builds one, through
# nothing but `purchase` and `place` commands over the real gdext JSON
# boundary.
#
# WHY THIS EXISTS, AND WHY IT IS SMALL. Until the "Real Placement" epic every
# world arrived with a complete production chain in it, so a presentation test
# could open the scene and immediately assert things about a factory. Ticket 1
# ended that: `World::new` builds nothing, and the only way to get a belt on
# the floor is to buy it. Every headless Godot test therefore has to build
# whatever it wants to look at first — which is a better test than the old one
# (it drives `place`/`remove`/`rotate` through the same wire format the scene
# does) with one hard limit worth stating plainly:
#
#   **A new game has `sim::STARTING_MONEY` (3000) and the shipped line costs
#   about 1,200.** That relationship was the other way round until the
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
##
## `item` is what a HOPPER will be loaded with, and since the chain rework it
## is required rather than optional: the shipped chain has two raw solids
## (tomato and sugar), so `PlayerCommand::Place` can no longer infer which
## bin the player meant and refuses a hopper placed without one — silently,
## like every other refused command. Every fixture below therefore names it,
## and a fixture that forgot would show up as a missing placement rather than
## as an error, which is exactly how this was found on `main`.
static func place(sim, kind: String, tile: Vector2i, direction: String, item := "") -> void:
	sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": kind}))
	var command := {
		"type": "place", "kind": kind, "tile": [tile.x, tile.y], "direction": direction,
	}
	if item != "":
		command["item"] = item
	sim.issue_command_json(JSON.stringify(command))


## Buys `quantity` units of a raw ingredient. Since the chain rework nothing
## enters the factory that was not paid for: `fill_hopper` and
## `fill_fluid_tank` draw from bought stock and are silent no-ops without it,
## so any fixture that intends to actually RUN has to stock its larder first.
static func stock(sim, item: String, quantity := 64) -> void:
	sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item, "quantity": quantity,
	}))


## Buys a working amount of EVERY raw ingredient the shipped content prices,
## enumerated from `ingredients_snapshot_json` rather than named here — so a
## third ingredient stocks itself. Called by any test that intends to run a
## line rather than only look at one.
static func stock_larder(sim, quantity := 64) -> void:
	var snapshot = JSON.parse_string(sim.ingredients_snapshot_json())
	if not (snapshot is Dictionary):
		return
	for entry in snapshot.get("entries", []):
		stock(sim, String(entry.get("id", "")), quantity)


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
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
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
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
	place(sim, "belt", Vector2i(0, 0), "east")


## A vinegar tank, a four-tile pipe run and the one machine whose recipe
## declares `fluid_inputs` — everything a pipe network needs to be worth
## looking at. The mixer never actually crafts (nothing feeds it tomato paste
## or sugar, which would need the rest of the chain — see
## `build_reference_line` for the version that does), which does not matter
## for what this fixture is for: a machine TAPS a network just by touching it,
## so the run has a fluid, a source, an inlet and a level that moves.
##
## The run sits on **row 2**, not row 1: the chain rework made the mixer 1x2,
## so it now covers (0, 0) AND (0, 1) and the old run collided with its own
## machine — a refused `place` and therefore a fixture that quietly built four
## fewer tiles than it printed.
static func build_fluid_run(sim) -> void:
	place(sim, "mixer", Vector2i(0, 0), "east")
	place(sim, "pipe", Vector2i(0, 2), "west")
	place(sim, "pipe", Vector2i(1, 2), "west")
	place(sim, "pipe", Vector2i(2, 2), "west")
	place(sim, "pipe", Vector2i(3, 2), "west")
	place(sim, "fluid_tank", Vector2i(4, 2), "west")


## Hopper -> belt -> crusher, then a belt into a delivery dock: 195 of 3000,
## and one of each SORT of thing on the floor. It exists to give the
## per-kind model checks and the junction rule something of each sort to look
## at — a feed building, a machine, an exit building, and the two belt tiles
## that sit against a building on either side (one catching a discharge, one
## handing over).
static func build_machines_fixture(sim) -> void:
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
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
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
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


## THE WHOLE SHIPPED LINE, bought and placed like anything else — the
## five-stage chain `sim::reference_chain` builds on the Rust side, tile for
## tile, all of it east-facing.
##
## **Rebuilt for the chain rework**, which changed its shape in four ways at
## once and is why every coordinate below moved: the aging rack is gone, the
## mixer is 1x2 and takes a second feed lane with its own sugar hopper, the
## mixer-to-vat and vat-to-bottler links are PIPE rather than belt, and the
## vat and the bottler therefore have no feed belt at all.
##
## It costs about 1,200 of a 3000 starting balance, and that is the point of
## it: it is both the fixture the full-chain screenshots use and a standing
## proof, through the real wire, that a new save can reach the shipped
## content by playing rather than by a test seam.
##
## The coordinates mirror `sim_core/src/sim/reference.rs`'s `plan()` — which
## derives them from `data/recipes.ron` order and `data/machines.ron`
## footprints — and `sim_core/tests/machine_footprints.rs` pins the same
## literals, so a content change that moves the line fails there first, with
## a far better message than a Godot script could give.
static func build_reference_line(sim) -> void:
	# Stage 1, the crusher: one feed lane, fed by a tomato hopper.
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
	for x in [0, 1, 2, 3]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "crusher", Vector2i(4, 0), "east")

	# Stage 2, the mixer: TWO feed lanes, one per solid ingredient — paste
	# arrives on row 0 from the crusher, sugar on row 1 from its own hopper.
	# That second lane is the whole reason the mixer is two tiles deep (see
	# `data/machines.ron`'s header): one shared inlet tile would deadlock.
	for x in [5, 6, 7, 8]:
		place(sim, "belt", Vector2i(x, 0), "east")
		place(sim, "belt", Vector2i(x, 1), "east")
	place(sim, "hopper", Vector2i(4, 1), "east", "sugar")
	place(sim, "mixer", Vector2i(9, 0), "east") # 1x2: it also covers y=1

	# The vinegar supply, on the row north of the mixer's WHOLE footprint
	# (y = 2 now that the machine is two deep), with the tank one tile west
	# of the run's head. A machine taps a network by TOUCHING it, not by
	# facing it, which is why feeding the mixer from above works at all.
	place(sim, "fluid_tank", Vector2i(5, 2), "east")
	for x in [6, 7, 8, 9]:
		place(sim, "pipe", Vector2i(x, 2), "east")

	# The piped middle. A fluid output leaves by the face the machine points
	# at, so the run east of the mixer IS the link to the vat, and the run
	# east of the vat is the link to the bottler. Neither the vat nor the
	# bottler has a feed belt at all — the clearest thing on the floor that
	# says a stage's connections are content rather than geometry.
	for x in [10, 11]:
		place(sim, "pipe", Vector2i(x, 0), "east")
	place(sim, "cooking_vat", Vector2i(12, 0), "east") # 2x2: also x=13 and y=1
	for x in [14, 15]:
		place(sim, "pipe", Vector2i(x, 0), "east")
	place(sim, "bottler", Vector2i(16, 0), "east")

	# ...and out the other side as solids again: bottles to the packer,
	# boxes to the dock.
	for x in [17, 18, 19, 20]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "packer", Vector2i(21, 0), "east")
	for x in [22, 23, 24, 25]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "dock", Vector2i(26, 0), "east")


## How many placements `build_reference_line` makes. Stated once here so a
## test asserting the line was built in full does not restate the layout.
const REFERENCE_LINE_PLACEMENTS := 37


## Sets the balance to `amount` through the save/load boundary — the one
## door into a sim the presentation layer is allowed to use, and the only
## way a Godot fixture can be bigger than `sim::STARTING_MONEY` (3000)
## allows without a Rust test seam. `save_ron` writes money as raw `Fx`
## bits (`money_bits: N`, I16F16, so N = amount * 65536); the text is
## rewritten and loaded back. `Fx` saturates near 32 767, so that is the
## ceiling. Everything else in the world survives the round trip unchanged,
## which the save/load lockstep test is what guarantees.
static func fund(sim, amount: int) -> void:
	var text: String = sim.save_ron()
	var regex := RegEx.new()
	regex.compile("money_bits:\\s*-?\\d+")
	var bits: int = mini(amount, 32767) * 65536
	var patched := regex.sub(text, "money_bits: %d" % bits)
	var error: String = sim.load_ron(patched)
	if error != "":
		push_error("fund(): load_ron refused the patched save: %s" % error)


## THE STRESS LINE (the pipes & conveyors overhaul, ticket 4) — the fixture
## `test_perf.gd` times and `scripts/screenshot-scene.sh --stress`
## photographs. Big on purpose, where every other fixture here is small: it
## exists to make per-tile rendering costs visible, and a thousand tiles is
## where a node per tile stops being free.
##
## A serpentine of STRESS_ROWS x STRESS_COLUMNS belts: even rows run east,
## odd rows run west, and the last tile of every row turns north into the
## next one — so the whole thing is ONE line of 1500 conveyor tiles with a
## corner at each end of each row, on three of the four facings. Fed by a
## hopper at its head and by hoppers on the free flanks of the first and last
## rows, so product enters at several points and the belts near the feeds
## carry items from the first few hundred ticks. Beside it, three tank-fed
## pipe runs, each tapped by a mixer so the run carries vinegar and its
## liquid bodies have something to show.
##
## Its bill (about 8,500) is far past a new game's balance, so it funds
## itself through `fund` first. The line's tail hands to nothing, so the last
## row backs up over time — a jammed stretch is a realistic thing to draw.
const STRESS_ROWS := 30
const STRESS_COLUMNS := 50
const STRESS_BELTS := STRESS_ROWS * STRESS_COLUMNS
const STRESS_PIPE_RUNS := 3
const STRESS_PIPES_PER_RUN := 8
const STRESS_FLANK_HOPPER_COLUMNS := [10, 20, 30, 40]

static func build_stress_line(sim) -> void:
	fund(sim, 32000)
	sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt", "quantity": STRESS_BELTS}))
	for row in range(STRESS_ROWS):
		var eastward := row % 2 == 0
		for column in range(STRESS_COLUMNS):
			var x := column if eastward else STRESS_COLUMNS - 1 - column
			var last := column == STRESS_COLUMNS - 1
			var direction := "north" if last else ("east" if eastward else "west")
			sim.issue_command_json(JSON.stringify({
				"type": "place", "kind": "belt", "tile": [x, row], "direction": direction,
			}))
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
	for column in STRESS_FLANK_HOPPER_COLUMNS:
		place(sim, "hopper", Vector2i(column, -1), "north", "tomato")
		place(sim, "hopper", Vector2i(column, STRESS_ROWS), "south", "sugar")
	for run in range(STRESS_PIPE_RUNS):
		var y := STRESS_ROWS + 4 + run * 3
		place(sim, "fluid_tank", Vector2i(-1, y), "east")
		for x in range(STRESS_PIPES_PER_RUN):
			place(sim, "pipe", Vector2i(x, y), "east")
		place(sim, "mixer", Vector2i(STRESS_PIPES_PER_RUN, y), "east")


## How many placements `build_stress_line` makes, so a test can assert the
## whole thing was affordable and legal.
const STRESS_PLACEMENTS := STRESS_BELTS + 1 + 2 * 4 + STRESS_PIPE_RUNS * (2 + STRESS_PIPES_PER_RUN)
