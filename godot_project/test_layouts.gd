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


## THE BOTTLENECK LINE (the machine-load epic, ticket 3) — the fixture
## `test_machine_load.gd` asserts and `scripts/screenshot-scene.sh
## --bottleneck` photographs: the reference line, with every hopper and the
## tank switched to auto-fill and the larder stocked deep enough to keep
## them fed, run until it has backed up behind its slowest stage.
##
## The 240-tick cooking vat is that stage, so a settled line reads, from
## the stack lights, red · red · green · amber · amber: the crusher and
## mixer BLOCKED (their output has nowhere to go), the vat WORKING (flat
## out), the bottler and packer STARVED (the vat cannot feed them fast
## enough). That is the picture the whole epic exists to show.
##
## It takes ~8500 ticks, not the ~1500 one might guess from the vat's
## cycle: every machine holds 8 of its output, there is a belt or pipe run
## between each pair, and all of that fills at the vat's pace before the
## mixer (from ~3500) and then the crusher (from ~8300) stall
## (`sim_core/tests/machine_load.rs` measured it; its settle constant and
## this one say the same number). The crusher's figure was ~7000 at the
## quarter-tile item spacing; at the half-tile spacing of the quality pass
## one lane delivers four tomatoes per 100 ticks against its 80-tick craft,
## so it is belt-limited and fills its output a little slower. The stock is
## sized for that run with room to spare: the crusher runs as fast as its
## lane feeds it for those first ~8300 ticks filling every buffer with
## paste, four tomatoes a craft, so tomatoes get an extra crate on top of
## the shared amount.
const BOTTLENECK_SETTLE_TICKS := 10000
const BOTTLENECK_STOCK := 400
const BOTTLENECK_EXTRA_TOMATOES := 300
const BOTTLENECK_MACHINES := {
	"4,0": "blocked", # crusher
	"9,0": "blocked", # mixer
	"12,0": "working", # cooking vat
	"16,0": "starved", # bottler
	"21,0": "starved", # packer
}

static func build_bottleneck_line(sim) -> void:
	build_reference_line(sim)
	# The line's bill leaves ~1900; the stock below is ~2000 at the shipped
	# prices (tomato 1, sugar 3, vinegar 2 per unit), so top the balance up.
	fund(sim, 8000)
	stock_larder(sim, BOTTLENECK_STOCK)
	stock(sim, "tomato", BOTTLENECK_EXTRA_TOMATOES)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true,
	}))
	for i in range(BOTTLENECK_SETTLE_TICKS):
		sim.tick()


## THE UNDERSTAFFED LINE (the staff pools MVP, ticket 3) — the fixture
## `test_staff_panel.gd` asserts and `scripts/screenshot-scene.sh
## --understaffed` photographs: the reference line, hoppers and tank on
## auto-fill, with two manufacturing heads FIRED. The shipped crew is sized
## to the line (4 manufacturing: crusher 1, mixer 1, vat 2), and the claim
## is per tick in stable machine order, so with two gone the crusher and
## the mixer take the two that are left every tick and the cooking vat is
## the machine refused — the same one every tick, by design
## (`docs/STAFF_POOLS_PLAN.md`, "Per tick, in stable order"). The line is
## run until the sim reports that refusal on the vat's tile, which takes a
## few hundred ticks (the vat asks only when it has a batch to start, and
## the first paste has to reach it), and is capped rather than open-ended
## so a sim change that stops it ever happening fails the test instead of
## hanging it.
const UNDERSTAFFED_VAT := Vector2i(12, 0)
const UNDERSTAFFED_CATEGORY := "manufacturing"
const UNDERSTAFFED_FIRED := 2
const UNDERSTAFFED_MAX_TICKS := 4000

static func build_understaffed_line(sim) -> void:
	build_reference_line(sim)
	# Funded and stocked like the bottleneck line, for its reason: the
	# line's bill leaves too little for a deep larder.
	fund(sim, 8000)
	stock_larder(sim, BOTTLENECK_STOCK)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true,
	}))
	for i in range(UNDERSTAFFED_FIRED):
		sim.issue_command_json(JSON.stringify({"type": "fire", "category": UNDERSTAFFED_CATEGORY}))
	for i in range(UNDERSTAFFED_MAX_TICKS):
		sim.tick()
		if understaffed_stall(sim, UNDERSTAFFED_VAT) == UNDERSTAFFED_CATEGORY:
			return


## The category the machine on `tile` was refused on the last tick, off
## `query_tile_json`'s `stall`, or "" when its stall is anything else.
static func understaffed_stall(sim, tile: Vector2i) -> String:
	var parsed = JSON.parse_string(sim.query_tile_json(tile.x, tile.y))
	if not (parsed is Dictionary):
		return ""
	var stall = parsed.get("stall")
	if stall is Dictionary and String(stall.get("kind", "")) == "staff":
		return String(stall.get("item", ""))
	return ""


## THE PIPE GALLERY (the pipes & conveyors overhaul, ticket 6) — the fixture
## `test_pipe_shapes.gd` asserts and `scripts/screenshot-scene.sh
## --pipe-gallery` photographs. Every pipe shape at every turn, from real
## placements, in three rows (y grows north):
##
##   * Row 0-3: two-tile runs (an end at each turn), three-tile straights
##     on both axes, a 2x2 ring of the four corner turns, four tees (each
##     closed on a different side), a cross, and two lonely tiles on each
##     axis. The first tee's run and the cross's run are each fed by a tank
##     and tapped by a MIXER (which draws vinegar, a raw fluid the larder
##     can stock — a bottler's ketchup cannot be bought, so a bottler-tapped
##     tank stays empty), so liquid stands in a tee, a cross, ends and
##     straights once the fixture is run.
##   * Row 5-7: a tank discharging into a lone pipe from each of its four
##     sides, then a bottler tapping a lone pipe from each of its four sides
##     — every fitting at every turn.
##   * Row 10-12: the shipped middle of the line at its real geometry
##     (`sim::reference_chain`): a tank-fed vinegar run turning down into
##     the 1x2 mixer, then mixer -> vat -> bottler through two-tile runs.
##
## Bill: about 1,940 of 3,000.
static func build_pipe_gallery(sim) -> void:
	# --- row 0-3: shapes -------------------------------------------------
	for tile in [Vector2i(0, 0), Vector2i(1, 0)]:
		place(sim, "pipe", tile, "east")
	for tile in [Vector2i(3, 0), Vector2i(4, 0), Vector2i(5, 0)]:
		place(sim, "pipe", tile, "east")
	for tile in [Vector2i(7, 0), Vector2i(7, 1)]:
		place(sim, "pipe", tile, "north")
	for tile in [Vector2i(9, 0), Vector2i(9, 1), Vector2i(9, 2)]:
		place(sim, "pipe", tile, "north")
	for tile in [Vector2i(12, 0), Vector2i(13, 0), Vector2i(12, 1), Vector2i(13, 1)]:
		place(sim, "pipe", tile, "east")
	# Tees: (centre, the side it is CLOSED on) — the arm tiles are the other
	# three neighbours. The first is fed and tapped.
	place(sim, "fluid_tank", Vector2i(16, 1), "east")
	place(sim, "mixer", Vector2i(18, 3), "north") # 1x2 turned: covers (18, 3) and (19, 3)
	for tee in [[Vector2i(18, 1), "south"], [Vector2i(22, 1), "north"], [Vector2i(26, 1), "east"], [Vector2i(30, 1), "west"]]:
		var centre: Vector2i = tee[0]
		place(sim, "pipe", centre, "east")
		for side in ["north", "east", "south", "west"]:
			if side != tee[1]:
				place(sim, "pipe", centre + _offset(side), "east")
	# Cross, fed and tapped.
	place(sim, "fluid_tank", Vector2i(34, 1), "east")
	place(sim, "mixer", Vector2i(36, 3), "north")
	place(sim, "pipe", Vector2i(36, 1), "east")
	for side in ["north", "east", "south", "west"]:
		place(sim, "pipe", Vector2i(36, 1) + _offset(side), "east")
	place(sim, "pipe", Vector2i(39, 0), "east")
	place(sim, "pipe", Vector2i(39, 2), "north")
	# --- row 5-7: a fitting on every side ----------------------------------
	place(sim, "fluid_tank", Vector2i(0, 6), "east")
	place(sim, "pipe", Vector2i(1, 6), "east")
	place(sim, "fluid_tank", Vector2i(5, 6), "west")
	place(sim, "pipe", Vector2i(4, 6), "east")
	place(sim, "fluid_tank", Vector2i(8, 5), "north")
	place(sim, "pipe", Vector2i(8, 6), "east")
	place(sim, "fluid_tank", Vector2i(11, 7), "south")
	place(sim, "pipe", Vector2i(11, 6), "east")
	place(sim, "bottler", Vector2i(15, 6), "east")
	place(sim, "pipe", Vector2i(14, 6), "east")
	place(sim, "bottler", Vector2i(18, 6), "west")
	place(sim, "pipe", Vector2i(19, 6), "east")
	place(sim, "bottler", Vector2i(22, 7), "north")
	place(sim, "pipe", Vector2i(22, 6), "east")
	place(sim, "bottler", Vector2i(25, 5), "south")
	place(sim, "pipe", Vector2i(25, 6), "east")
	# --- row 10-12: the shipped middle ------------------------------------
	place(sim, "fluid_tank", Vector2i(0, 12), "east")
	for tile in [Vector2i(1, 12), Vector2i(2, 12), Vector2i(3, 12)]:
		place(sim, "pipe", tile, "east")
	place(sim, "mixer", Vector2i(3, 10), "east")
	for tile in [Vector2i(4, 10), Vector2i(5, 10)]:
		place(sim, "pipe", tile, "east")
	place(sim, "cooking_vat", Vector2i(6, 10), "east")
	for tile in [Vector2i(8, 10), Vector2i(9, 10)]:
		place(sim, "pipe", tile, "east")
	place(sim, "bottler", Vector2i(10, 10), "east")


## Every placement `build_pipe_gallery` makes, so a test asserting it was
## built in full does not restate the layout.
const PIPE_GALLERY_PLACEMENTS := 41 + 16 + 11 # rows 0-3, 5-7 and 10-12


## Tile-space step per direction, the simulation's own convention (east +X,
## north +Y — `sim::types::Direction::offset`).
static func _offset(direction: String) -> Vector2i:
	match direction:
		"north":
			return Vector2i(0, 1)
		"south":
			return Vector2i(0, -1)
		"west":
			return Vector2i(-1, 0)
	return Vector2i(1, 0)


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


## THE BELT GALLERY (the pipes & conveyors overhaul, ticket 5) — every belt
## shape at every facing, and a belt meeting a machine, a hopper and a dock
## at every facing, in one world. `test_belt_shapes.gd` asserts against it
## and `scripts/screenshot-scene.sh --belt-gallery` photographs it. About
## 1,300 of a 3,000 starting balance.
##
## What is on the floor, in simulation coordinates (north = +Y):
##
##   * A CLOCKWISE ring of twelve belts on x 0..3, y 0..3. Every corner of a
##     clockwise loop turns RIGHT, so its four corner tiles are
##     `belt_corner_right` at all four facings: (0,3) east, (3,3) south,
##     (3,0) west, (0,0) north. A hopper at (1,-1) faces north onto (1,0),
##     which is fed from behind by the ring as well, so (1,0) is a MERGE and
##     the ring carries product (with `--populate`) round both hands of
##     corner and jams up, which is a real state worth drawing.
##   * An ANTICLOCKWISE ring on x 6..9, y 0..3: `belt_corner_left` at all
##     four facings — (6,0) east, (9,0) north, (9,3) west, (6,3) south.
##     Unfed, so its corners are photographed clean.
##   * Four MACHINE LINES, hopper -> belt -> crusher -> belt -> dock, one
##     per facing: east along y = 6, west along y = 9, north up x = 12,
##     south down x = 16. Every belt-to-building junction the game has, at
##     every facing the player can choose. (The 2x2 dock's inlet edge is the
##     whole edge facing the belt, so each line's last belt meets the dock's
##     origin tile.)
##   * A T MERGE at (10,6): fed from both flanks by (10,5) facing north and
##     (10,7) facing south, nothing behind, handing east to (11,6), which
##     hands to nothing — a two-flank merge and a line END side by side.
##   * A lone belt at (8,6) (a line START with no output) and a two-tile
##     start-then-end at (8,8) -> (9,8). (Not on row 7: a belt accepts from
##     any side, so (9,7) would hand into the merge's south-facing feeder.)
static func build_belt_gallery(sim) -> void:
	# The clockwise ring (right corners). Along the top row east, down the
	# east column south, along the bottom row west, up the west column north.
	for x in [0, 1, 2]:
		place(sim, "belt", Vector2i(x, 3), "east")
	place(sim, "belt", Vector2i(3, 3), "south")
	for y in [2, 1]:
		place(sim, "belt", Vector2i(3, y), "south")
	place(sim, "belt", Vector2i(3, 0), "west")
	for x in [2, 1]:
		place(sim, "belt", Vector2i(x, 0), "west")
	place(sim, "belt", Vector2i(0, 0), "north")
	for y in [1, 2]:
		place(sim, "belt", Vector2i(0, y), "north")
	place(sim, "belt", Vector2i(0, 3), "east")
	place(sim, "hopper", Vector2i(1, -1), "north", "tomato")

	# The anticlockwise ring (left corners). Along the bottom row east, up
	# the east column north, along the top row west, down the west column
	# south.
	place(sim, "belt", Vector2i(6, 0), "east")
	for x in [7, 8]:
		place(sim, "belt", Vector2i(x, 0), "east")
	place(sim, "belt", Vector2i(9, 0), "north")
	for y in [1, 2]:
		place(sim, "belt", Vector2i(9, y), "north")
	place(sim, "belt", Vector2i(9, 3), "west")
	for x in [8, 7]:
		place(sim, "belt", Vector2i(x, 3), "west")
	place(sim, "belt", Vector2i(6, 3), "south")
	for y in [2, 1]:
		place(sim, "belt", Vector2i(6, y), "south")

	# Machine lines, one per facing.
	place(sim, "hopper", Vector2i(0, 6), "east", "tomato")
	place(sim, "belt", Vector2i(1, 6), "east")
	place(sim, "crusher", Vector2i(2, 6), "east")
	place(sim, "belt", Vector2i(3, 6), "east")
	place(sim, "dock", Vector2i(4, 6), "east")

	place(sim, "dock", Vector2i(0, 9), "west")
	place(sim, "belt", Vector2i(2, 9), "west")
	place(sim, "crusher", Vector2i(3, 9), "west")
	place(sim, "belt", Vector2i(4, 9), "west")
	place(sim, "hopper", Vector2i(5, 9), "west", "tomato")

	place(sim, "hopper", Vector2i(12, 0), "north", "tomato")
	place(sim, "belt", Vector2i(12, 1), "north")
	place(sim, "crusher", Vector2i(12, 2), "north")
	place(sim, "belt", Vector2i(12, 3), "north")
	place(sim, "dock", Vector2i(12, 4), "north")

	place(sim, "dock", Vector2i(16, 0), "south")
	place(sim, "belt", Vector2i(16, 2), "south")
	place(sim, "crusher", Vector2i(16, 3), "south")
	place(sim, "belt", Vector2i(16, 4), "south")
	place(sim, "hopper", Vector2i(16, 5), "south", "tomato")

	# The T merge and its dead end, the lone start, the start-then-end.
	place(sim, "belt", Vector2i(10, 5), "north")
	place(sim, "belt", Vector2i(10, 7), "south")
	place(sim, "belt", Vector2i(10, 6), "east")
	place(sim, "belt", Vector2i(11, 6), "east")
	place(sim, "belt", Vector2i(8, 6), "east")
	place(sim, "belt", Vector2i(8, 8), "east")
	place(sim, "belt", Vector2i(9, 8), "east")


## How many placements `build_belt_gallery` makes: 24 ring belts, 8 line
## belts, 7 loose belts, 5 hoppers, 4 crushers, 4 docks.
const BELT_GALLERY_PLACEMENTS := 24 + 8 + 7 + 5 + 4 + 4

## The gallery's corner tiles by hand and facing, for tests and closeups.
const BELT_GALLERY_RIGHT_CORNERS := {
	"east": Vector2i(0, 3), "south": Vector2i(3, 3), "west": Vector2i(3, 0), "north": Vector2i(0, 0),
}
const BELT_GALLERY_LEFT_CORNERS := {
	"east": Vector2i(6, 0), "north": Vector2i(9, 0), "west": Vector2i(9, 3), "south": Vector2i(6, 3),
}
