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


## Buys `quantity` units of a raw ingredient, building the warehouse room for
## it first. Since the chain rework nothing enters the factory that was not
## paid for: `fill_hopper` and `fill_fluid_tank` draw from bought stock and
## are silent no-ops without it, so any fixture that intends to actually RUN
## has to stock its larder first. Since W1 nothing is unlimited either — a
## holding is capped at `data/storage.ron`'s `base_cap` plus whatever bays
## the placed warehouse racks supply, and an order over the ceiling is
## silently CLAMPED. A fixture that skipped the racks would therefore run for
## a few hundred ticks and then photograph an idle line, which is how the
## fluid gallery failed when the cap landed.
static func stock(sim, item: String, quantity := 64) -> void:
	make_room(sim, item, quantity)
	sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item, "quantity": quantity,
	}))


## Buys a working amount of EVERY raw ingredient the shipped content prices,
## enumerated from `ingredients_snapshot_json` rather than named here — so a
## third ingredient stocks itself. Called by any test that intends to run a
## line rather than only look at one. Gated ingredients are not in that list
## until their unlock opens, so this buys — and warehouses — only what the
## player could buy at this moment.
static func stock_larder(sim, quantity := 64) -> void:
	for item in stockable(sim):
		stock(sim, item, quantity)


## Tops every stockable raw back up to `quantity`, building nothing and
## asserting nothing.
##
## This is what a long fixture run does instead of stocking once and deeply.
## Before W1 a fixture that needed 22 000 ticks of raw simply bought 3 000 of
## it up front; now it cannot — that is sixty bays per ingredient, a wall of
## racks in a shot that is meant to be of a dock. So the stock arrives the way
## a player's does: the line eats what is in the larder, room appears, and the
## order goes in again. Call it every [constant RESTOCK_EVERY] ticks, which is
## far more often than [constant LARDER_STOCK] can be consumed in.
## `deeper` names the raws to hold above `quantity` — one entry per
## ingredient a fixture eats faster than the rest, so that the shared level
## stays one number.
static func restock(sim, quantity := LARDER_STOCK, deeper := {}) -> void:
	var snapshot = JSON.parse_string(sim.ingredients_snapshot_json())
	if not (snapshot is Dictionary):
		return
	for entry in snapshot.get("entries", []):
		var item := String(entry.get("id", ""))
		var target := int(deeper.get(item, quantity))
		var held := int(entry.get("owned", 0))
		if held < target:
			sim.issue_command_json(JSON.stringify({
				"type": "buy_ingredient", "item": item, "quantity": target - held,
			}))


## Every raw ingredient the player may buy right now, in the sim's own order.
## `ingredients_snapshot_json` drops what is still behind an unlock, so this
## is the gate's verdict rather than a list repeated here.
static func stockable(sim) -> PackedStringArray:
	var items := PackedStringArray()
	var snapshot = JSON.parse_string(sim.ingredients_snapshot_json())
	if not (snapshot is Dictionary):
		return items
	for entry in snapshot.get("entries", []):
		items.append(String(entry.get("id", "")))
	return items


## What one fixture larder holds of each raw: `data/storage.ron`'s free
## `base_cap` of 20 plus one whole rack's six bays of 50. Sized to a RACK
## rather than to a run, so that a fixture's warehouse is one building per
## ingredient and stays out of the way of the thing being photographed; the
## length of the run is [method restock]'s job, not this number's.
const LARDER_STOCK := 320

## How often a settling fixture tops its larder back up. A tick of the
## reference line consumes well under one unit of anything, so a hundred
## ticks cannot empty a larder — this is frequent enough to be invisible and
## rare enough that the JSON does not dominate a 40 000-tick loop.
const RESTOCK_EVERY := 100

## The row the fixtures' warehouse stands on, and the one north of it (a rack
## is 1x2). Every layout in this file builds at `y >= -3` and the stress
## fixture's southernmost tile is `y = -1`, so a rack row here cannot collide
## with any of them however the content grows.
const WAREHOUSE_ROW_Y := -8

## How far east [method add_rack] looks for a free tile before giving up — a
## bound on the search, not on the warehouse.
const WAREHOUSE_ROW_TILES := 96


## Builds however many warehouse racks it takes for `quantity` more of `item`
## to fit, through the same purchase and placement commands a player uses.
##
## Racks are added ONE at a time and the room rechecked, rather than computed
## from `base_cap` and `slot_stack`: free bays are a shared pool, so the room
## for tomato depends on how much sugar is held, and that arithmetic lives in
## `sim_core::sim::storage` alone. A fixture that cannot afford another rack
## stops asking and lets the buy be clamped — the same outcome a player gets,
## and the fixture's own assertions are what should complain about it.
static func make_room(sim, item: String, quantity: int) -> void:
	while stock_room_for(sim, item) < quantity:
		if not add_rack(sim):
			return


## How much more of `item` the warehouse will take — the sim's own figure,
## off the `capacity` the ingredient snapshot carries, never a cap recomputed
## here.
static func stock_room_for(sim, item: String) -> int:
	var snapshot = JSON.parse_string(sim.ingredients_snapshot_json())
	if not (snapshot is Dictionary):
		return 0
	for entry in snapshot.get("entries", []):
		if String(entry.get("id", "")) == item:
			return maxi(0, int(entry.get("capacity", 0)) - int(entry.get("owned", 0)))
	return 0


## One more rack, on the first free tile of the warehouse row. Returns whether
## one went up — a purchase the fixture cannot afford and a row with no gap
## left both come back `false`.
##
## The free tile is found by TRYING rather than by reading occupancy, so this
## stays correct for a fixture that has built something of its own down there,
## and the verdict is `total_bays` moving rather than a placement count: a
## refused `place` is silent, like every other refused command.
static func add_rack(sim) -> bool:
	var before := storage_bays(sim)
	sim.issue_command_json(JSON.stringify({
		"type": "purchase", "kind": WAREHOUSE_KIND,
	}))
	for x in range(WAREHOUSE_ROW_TILES):
		sim.issue_command_json(JSON.stringify({
			"type": "place", "kind": WAREHOUSE_KIND,
			"tile": [x, WAREHOUSE_ROW_Y], "direction": "east",
		}))
		if storage_bays(sim) > before:
			return true
	return false


## The buildable kind that supplies bays. Named once here so the fixtures
## and the `*_PLACEMENTS` counts below agree on what is warehouse and what is
## line.
const WAREHOUSE_KIND := "warehouse_rack"


## How many placements a fixture's LINE has — every `*_PLACEMENTS` constant
## in this file counts these, and none of them count the warehouse.
##
## Stocking a larder puts racks down (see [method make_room]), and how many
## depends on `data/storage.ron`'s bay arithmetic and on what the unlock gate
## has opened, so a fixture that folded them into its total would have to
## restate that arithmetic and would break on a content edit. The layout
## under test is the line.
static func line_placements(sim) -> int:
	var count := 0
	for entry in JSON.parse_string(sim.world_layout_json()):
		if String(entry.get("kind", "")) != WAREHOUSE_KIND:
			count += 1
	return count


## How many bays the placed warehouse supplies in total.
static func storage_bays(sim) -> int:
	var snapshot = JSON.parse_string(sim.storage_snapshot_json())
	if not (snapshot is Dictionary):
		return 0
	return int(snapshot.get("total_bays", 0))


## THE WAREHOUSE ROW — what `scripts/screenshot-scene.sh --warehouse`
## photographs, and the only fixture whose subject is the STOCK rather than
## the line that eats it.
##
## Three racks side by side holding three raws, and every readout W3 draws in
## one frame: two racks with all six bays bound, a third with two, a part-full
## load on the last bay of each ingredient, and four bare shelves to compare
## the bound ones against. Two of the racks hold two ingredients each — the
## tints are what tell those apart, and a rack that is not "the tomato rack"
## is the visible consequence of bays being a shared pool rather than a
## per-item shelf.
##
## IT IS ALSO SIZED TO REFUSE ITS OWN REMOVAL. Fourteen of the eighteen bays
## are in use, so picking any rack up would leave twelve for stock that needs
## fourteen, and `--warehouse --remove-mode` photographs W1's refusal being
## said out loud rather than a rack quietly failing to come up. Stocking this
## row less deeply would silently cost that second shot.
##
## THE AMOUNTS ARE CHOSEN AGAINST `data/storage.ron`, not against appetite:
## each is `base_cap` (free, and binds no bay at all) plus a number of whole
## `slot_stack`s plus a remainder, so that the last bay of each ingredient is
## visibly PART full. They are read back off the wire below rather than
## assumed — a price rise or a `base_cap` edit that puts the order over the
## ceiling would otherwise be photographed as a rack that is simply emptier
## than the reviewer was told, which is the failure `build_fluid_gallery`
## refuses for the same reason.
##
## ONLY THE UNGATED RAWS STAND HERE. `data/unlocks.ron` holds the three BBQ
## ingredients back until ten deliveries are made, so a fresh world cannot
## buy spice mix, molasses or liquid smoke at any price and this row shows
## the three a new game has (see `build_bbq_running_line`, which pays 40 000
## ticks for the other three). That every raw the game prices — gated ones
## included — has a rack tint of its own, distinct from the floor, is proved
## headlessly by `test_warehouse_rack.gd` instead: it is a claim about a
## table, and it does not need a camera.
const WAREHOUSE_ROW_RACKS := [Vector2i(0, 0), Vector2i(2, 0), Vector2i(4, 0)]

## The holding each raw is bought to, written as `[item, whole bays,
## remainder]` and turned into units below so that the intent survives an edit
## to either knob: six bays of tomato, four of sugar and four of vinegar, and
## a part-full bay on the end of each — 0.6, 0.9 and 0.3 of a bay, three
## visibly different heights.
const WAREHOUSE_ROW_STOCK := [
	["tomato", 5, 30],
	["sugar", 3, 45],
	["vinegar", 3, 15],
]


## Builds the row above. Returns "" or a complaint naming what is missing —
## the fixture is its contents, so a rack row that came up short is refused
## rather than photographed.
static func build_warehouse_row(sim) -> String:
	for tile in WAREHOUSE_ROW_RACKS:
		sim.issue_command_json(JSON.stringify({
			"type": "purchase", "kind": WAREHOUSE_KIND,
		}))
		sim.issue_command_json(JSON.stringify({
			"type": "place", "kind": WAREHOUSE_KIND,
			"tile": [tile.x, tile.y], "direction": "east",
		}))
	if line_placements(sim) != 0 or placement_count(sim) != WAREHOUSE_ROW_RACKS.size():
		return "the warehouse row placed %d of %d racks (and %d other things)" % [
			placement_count(sim), WAREHOUSE_ROW_RACKS.size(), line_placements(sim),
		]
	var storage = JSON.parse_string(sim.storage_snapshot_json())
	if not (storage is Dictionary):
		return "storage_snapshot_json returned no object"
	var base_cap := int(storage.get("base_cap", 0))
	var slot_stack := int(storage.get("slot_stack", 0))
	for entry in WAREHOUSE_ROW_STOCK:
		var item := String(entry[0])
		var units: int = base_cap + int(entry[1]) * slot_stack + int(entry[2])
		sim.issue_command_json(JSON.stringify({
			"type": "buy_ingredient", "item": item, "quantity": units,
		}))
		var held := stock_held(sim, item)
		if held != units:
			return (
				"the warehouse row holds %d of %s, not the %d it was built for — "
				% [held, item, units]
				+ "the order was clamped, so the shot would show a rack emptier "
				+ "than its caption claims"
			)
	return ""


## How many units of one raw the factory is holding, off the ingredient
## snapshot — the same figure the market plate shows.
static func stock_held(sim, item: String) -> int:
	var snapshot = JSON.parse_string(sim.ingredients_snapshot_json())
	if not (snapshot is Dictionary):
		return 0
	for entry in snapshot.get("entries", []):
		if String(entry.get("id", "")) == item:
			return int(entry.get("owned", 0))
	return 0


## Every placement in the world, warehouse included — [method line_placements]
## counts the other half.
static func placement_count(sim) -> int:
	return JSON.parse_string(sim.world_layout_json()).size()


## THE DEMO LINE — the fixture most tests build, and the one
## `scripts/screenshot-scene.sh --build` photographs.
##
## A hopper feeding four belts that run east and then TURN NORTH into a
## crusher facing WEST, which discharges back west along three more belts, with
## a short pipe run heading SOUTH beside it. That shape is chosen, not
## arbitrary: it uses three of the four directions on belts and the fourth on
## the pipes, so "does a rotated tile render correctly" and "does a belt pick
## the right shape where it turns and where it meets a machine" are both
## visible in one frame. It also actually RUNS — the hopper unloads onto the first
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


## THE BBQ BRANCH (the BBQ epic) — the shipped line above plus the second
## sauce hanging off the middle of it, 872 on top of the reference line's 959
## and so still inside the 3000 a new game starts with.
##
## It mirrors `sim::bbq_branch` tile for tile — that fixture's doc comment
## carries the picture and the reasoning, and
## `sim_core/src/sim/reference.rs` pins both layouts to an exact coordinate
## list, so a content change that moves either one fails there first with a
## better message than a Godot test could give.
##
## The three joints worth knowing before moving anything here:
##
##   * the SINGLE PIPE at (14, -1) is the branch. It touches the ketchup
##     line's outlet run below it and the BBQ mixer above it, which is how
##     finished ketchup gets out of one line and into the other;
##   * the LIQUID SMOKE run keeps to the east half of row -1. Three tiles
##     further west its head would touch the ketchup run and the two
##     networks would merge — pipes join pipes, which is also why this run
##     crossing over the ketchup line's exit BELTS is harmless;
##   * the branch gets its OWN DOCK. Every dock feeds the same shared
##     delivery pool, so a second one is free and much clearer than merging
##     two product streams onto one belt.
static func build_bbq_branch(sim) -> void:
	build_reference_line(sim)

	# Two raw solids, one lane each into its own tile of the 1x2 mixer's
	# inlet face — the same arrangement, and the same reason, as the ketchup
	# mixer's sugar and tomato paste lanes.
	place(sim, "hopper", Vector2i(9, -3), "east", "spice_mix")
	place(sim, "hopper", Vector2i(9, -2), "east", "molasses")
	for x in [10, 11, 12, 13]:
		place(sim, "belt", Vector2i(x, -3), "east")
		place(sim, "belt", Vector2i(x, -2), "east")
	place(sim, "mixer", Vector2i(14, -3), "east")  # 1x2: also covers y=-2

	# The branch itself.
	place(sim, "pipe", Vector2i(14, -1), "east")

	# Mixer -> vat, off the mixer's outlet face, and the liquid smoke the vat
	# needs alongside the BBQ mix.
	for x in [15, 16]:
		place(sim, "pipe", Vector2i(x, -3), "east")
	place(sim, "cooking_vat", Vector2i(17, -3), "east")  # 2x2
	place(sim, "fluid_tank", Vector2i(16, -1), "east")
	for x in [17, 18, 19, 20]:
		place(sim, "pipe", Vector2i(x, -1), "east")

	# Vat -> bottler -> packer -> its own dock.
	for x in [19, 20]:
		place(sim, "pipe", Vector2i(x, -3), "east")
	place(sim, "bottler", Vector2i(21, -3), "east")
	for x in [22, 23, 24, 25]:
		place(sim, "belt", Vector2i(x, -3), "east")
	place(sim, "packer", Vector2i(26, -3), "east")
	for x in [27, 28, 29, 30]:
		place(sim, "belt", Vector2i(x, -3), "east")
	place(sim, "dock", Vector2i(31, -3), "east")


## How many placements `build_bbq_branch` makes — the reference line's 37
## plus 33 of its own.
const BBQ_BRANCH_PLACEMENTS := 70


## How long the BBQ line runs once the branch is in, and how deep its
## larder is.
##
## 15 000 ticks is chosen between two walls, and it is counted from the
## moment the branch is PLACED rather than from tick zero. Below about 9 000
## the branch is still priming its two new pipe runs and the ketchup bottler
## is starved, so the shot shows one sauce; past about 30 000 the delivery
## pool hits its ceiling and the packers back up, so the shot shows a stalled
## factory. In between, both lines run at the 3:2 split
## `data/contracts.ron`'s mixed orders are written against and there are
## boxes of both sauces on the floor.
const BBQ_SETTLE_TICKS := 15000

## How long the ketchup line is allowed to run on its own before the fixture
## gives up waiting for `data/unlocks.ron`'s BBQ gate to open. The shipped
## gate (ten deliveries) opens at about 12 000 ticks on one line serving the
## starter order alone; this ceiling is generous enough that a balance pass
## on that file moves the shot's run length rather than breaking it, and
## tight enough that a gate which can never open fails the fixture instead of
## hanging it. Reaching this ceiling is a HARD FAILURE — the wait below
## latches `BBQ_FIXTURE_FAIL:` and returns rather than building on, because a
## branch built behind a shut gate cannot buy its raws and never makes a box.
const BBQ_GATE_TICK_CEILING := 40000

## How often the wait above checks the gate. A snapshot parse per tick would
## dominate the run; the gate is checked on the same 250-tick grid the
## balance measurements in `docs/DECISIONS.md` were taken on.
const BBQ_GATE_POLL_TICKS := 250


## The complaint of the first fixture failure this run, or "" while nothing
## has gone wrong. Read it with `fixture_failure()`.
##
## **A latch rather than an `assert`.** `assert()` does not halt the calling
## chain here, so "print a marker and assert" printed the marker and then
## built on regardless: the shutter fired, `SCREENSHOT_OK:` printed, a PNG
## was written and the run exited 0 with a dead-branch photograph in it
## (issue #110). A caller that reads this latch refuses in every build,
## release included, which is the property `assert` does not have.
##
## **The FIRST failure wins.** A broken snapshot used to raise its marker
## once per poll — 322 times in the issue's break test — and then raise the
## gate marker too, so a broken wire read in the log as a shut gate, which is
## the precise confusion the marker was added to remove.
static var _fixture_failure := ""


## The first fixture failure, or "" if the fixture built what it promised.
## `test_screenshot.gd` checks this immediately after `_build_fixture()` and
## refuses the run; anything else that builds a fixture should do the same.
static func fixture_failure() -> String:
	return _fixture_failure


## Latches a fixture failure under `marker` and prints it once.
##
## The marker is the caller's, so a grep that already looks for
## `BBQ_FIXTURE_FAIL:` keeps working while other fixtures can name
## themselves. Every caller must return straight after calling this: the
## latch stops the run at the next check, it cannot unwind the stack.
static func fail_fixture(marker: String, message: String) -> void:
	if _fixture_failure != "":
		return
	_fixture_failure = message
	printerr("%s: %s" % [marker, message])


## True once `data/unlocks.ron` has opened at least one gate — which, with
## one unlock shipped, means BBQ is buyable and its contracts signable.
##
## Read from the simulation's own snapshot rather than counted here: a gate
## is a rule in `sim_core` (`BuyIngredient` and `SignContract` are REFUSED
## for a locked id), and a fixture that re-derived the threshold in GDScript
## would be the presentation layer deciding a gameplay question.
##
## A snapshot that will not parse is NOT "the gate is shut" — it is the wire
## broken, and returning `false` for it would let a fixture wait out its
## ceiling and then build anyway. It latches a fixture failure instead, and
## the caller must stop on it: `false` is still returned because this returns
## a bool, and it is `fixture_failure()` that says which kind of false it is.
static func any_unlock_open(sim) -> bool:
	var economy = JSON.parse_string(sim.economy_snapshot_json())
	if not (economy is Dictionary):
		fail_fixture(
			"BBQ_FIXTURE_FAIL", "economy_snapshot_json did not parse as a Dictionary"
		)
		return false
	for unlock in economy.get("unlocks", []):
		if bool(unlock.get("open", false)):
			return true
	return false


## THE MIXED EXIT (issue #57) — a splitter on the BBQ branch's exit run whose
## side output drops onto the KETCHUP line's exit belt, so that one belt in
## the frame carries both sauces alternating.
##
## WHY THE SHOT NEEDS IT. Whether `box_of_ketchup` and `box_of_bbq` can be
## told apart was settled on `--item-gallery` (#40): four items on a bare
## belt, one of each two tiles apart. What that cannot answer is how the pair
## reads MIXED — alternating on a working belt, with a packer, a dock and the
## contract clipboard in the same frame competing for the same glance. No
## fixture could put that in one frame, because the shipped branch gives BBQ
## its own dock.
##
## WHAT IT DOES NOT CHANGE. `build_bbq_branch` — the layout
## `sim_core/src/sim/reference.rs` pins tile for tile — is untouched, and so
## is its third joint: the branch still has its own dock, and the splitter's
## STRAIGHT output still runs east into it. Only the side output is new, so
## the frame shows both arrangements at once (two product streams kept apart,
## and two merged) rather than replacing the shipped one with a merge.
##
## THE ROUTE, and why the splitter faces NORTH rather than along the run it
## sits in. A splitter carries one direction: its placement's facing is the
## STRAIGHT output and the second output is one quarter turn clockwise from
## it (`sim::splitter`'s one direction rule). The branch runs east, the
## ketchup line is NORTH of it (`Direction::North` is +y, so row -3 is below
## row 0), and clockwise from east is south — the wrong way. Faced north, the
## two outputs land exactly where this fixture wants them with no second
## field and no new rule: STRAIGHT north onto the new run, CLOCKWISE east
## onto the branch's own exit belt and its own dock.
##
## From (27, -2) the run heads west along a row that is clear east of the
## cooking vat, then north across row -1 into the ketchup line's exit belt at
## (22, 0), which carries both sauces for its last four tiles and onto the
## dock's deck. Belts crossing the liquid-smoke run on row -1 are harmless:
## only pipes join pipes (the branch's second joint says the same thing about
## that run).
static func build_mixed_exit(sim) -> void:
	sim.issue_command_json(JSON.stringify({
		"type": "remove", "tile": [MIXED_SPLITTER.x, MIXED_SPLITTER.y],
	}))
	place(sim, "splitter", MIXED_SPLITTER, "north")
	for x in [27, 26, 25, 24, 23]:
		place(sim, "belt", Vector2i(x, -2), "west")
	place(sim, "belt", Vector2i(22, -2), "north")
	place(sim, "belt", Vector2i(22, -1), "north")


## The splitter that makes the exit mixed, and the belt the two sauces share
## once it does — the two tiles any check or shot of the mixed line names.
const MIXED_SPLITTER := Vector2i(27, -3)
const MIXED_BELT := Vector2i(23, 0)


## THE BBQ LINE, RUNNING — what `scripts/screenshot-scene.sh --bbq`
## photographs: the branch above, funded, fully crewed, auto-filled and run
## until both sauces are moving at once.
##
## **IT BUILDS IN THE ORDER THE GAME DICTATES** (issue #55), and that is the
## whole shape of this fixture. `data/unlocks.ron` holds the three BBQ raws
## back until ten contract deliveries have been made, so a fixture that
## placed the branch at tick zero and stocked the larder in the same breath
## bought no spice mix, no molasses and no liquid smoke — `stock_larder`
## enumerates the ingredients the market is OFFERING, and a locked one is
## absent from it. That is what this shot did between the gate landing and
## this change: it photographed a complete BBQ branch that had never made a
## single box, and every test of it passed, because nothing asserts that a
## fixture's factory actually ran.
##
## So: the ketchup line runs on its own until the gate opens, and only then
## is the branch placed, stocked, crewed and its contracts signed. That is
## the sequence a player lives through, and it is the reason this fixture no
## longer needs to sign the starter order back up — see below.
##
## **NO RE-SIGN, AND WHY THAT IS NOW HONEST** (issue #55). This fixture used
## to sign three contracts again after the settle, because building the
## branch starved the ketchup bottler for thousands of ticks and the Corner
## Shop order struck out. Measured (`docs/DECISIONS.md`, the BBQ gate
## ruling), that transient costs a one-line factory about 8 500 ticks of
## ketchup — but a factory that has reached the gate serving the starter
## order alone has banked about 22 boxes by then, and the order draws two per
## 1 000 ticks from that bank without missing a window. The gate is what buys
## the slack; the re-sign was papering over a fixture that built in the wrong
## order.
##
## It funds itself rather than living inside a new game's 3000, for the same
## reason `build_bottleneck_line` does: two lines, a bigger crew and 27 000
## ticks of raws cost more than a new game has, and what this fixture is for
## is the PICTURE of a working two-sauce factory. That a player can afford to
## BUILD it is a separate claim, proved separately, from a real starting
## balance, by `test_bbq_branch.gd`.
static func build_bbq_running_line(sim) -> void:
	# 1. The ketchup line alone — everything a new game can reach.
	build_reference_line(sim)
	fund(sim, 30000)
	stock_larder(sim, LARDER_STOCK)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true,
	}))
	hire_until_staffed(sim)

	# 2. Run it until the game offers BBQ. The starter order is the only one
	#    active, exactly as a new game starts, so the deliveries that open
	#    the gate also bank the stock that pays for the transient below.
	var waited := 0
	while waited < BBQ_GATE_TICK_CEILING:
		if any_unlock_open(sim):
			break
		# A snapshot that would not parse has already latched. Leave NOW
		# rather than ticking out the remaining ceiling and then reporting a
		# shut gate on top of it (issue #110).
		if fixture_failure() != "":
			return
		# A poll is longer than RESTOCK_EVERY, so this is the top-up: the
		# ketchup line runs for up to 40 000 ticks here and a single larder
		# is 320 of each raw.
		restock(sim)
		for _i in range(BBQ_GATE_POLL_TICKS):
			sim.tick()
		waited += BBQ_GATE_POLL_TICKS
	# The ceiling is a failure, not a fallback. Building on past it buys the
	# three BBQ raws from a market that still has them locked, so
	# `stock_larder` silently buys nothing and the branch is a complete line
	# that never makes a box — which is the exact bug this fixture was
	# rebuilt to stop photographing. Say so loudly, and stop: the rest of this
	# function would build the dead branch the marker exists to prevent.
	if fixture_failure() != "":
		return
	if not any_unlock_open(sim):
		fail_fixture(
			"BBQ_FIXTURE_FAIL",
			"no unlock open after %d ticks (ceiling %d) — " % [waited, BBQ_GATE_TICK_CEILING]
			+ "the BBQ raws are still locked, so the branch would never run"
		)
		return

	# 3. Build the branch the moment it is unlocked. `build_bbq_branch`
	#    re-issues the ketchup line's placements first; every one of them is
	#    refused on an occupied tile, harmlessly.
	fund(sim, 30000)
	build_bbq_branch(sim)
	build_mixed_exit(sim)
	fund(sim, 30000)
	# The larder is stocked AGAIN, and this time the three BBQ raws are in
	# the market to be bought — they were not, before the gate opened.
	stock_larder(sim, LARDER_STOCK)
	for tile in [Vector2i(9, -3), Vector2i(9, -2)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [16, -1], "enabled": true,
	}))
	# A second line needs a bigger crew, and an understaffed factory runs
	# slow everywhere rather than only on the new machines (GAMEPLAY.md §7) —
	# which would quietly turn this into a photograph of the staffing
	# mechanic instead of of the branch.
	hire_until_staffed(sim)
	# Both BBQ contracts that ask for one sauce, plus the mixed order: the
	# clipboard in the shot then shows what the epic added to the board. Both
	# signatures are accepted now and would have been REFUSED at tick zero —
	# a BBQ contract is behind the same gate its ingredients are.
	for id in ["smokehouse_trial", "grill_pub"]:
		sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": id}))

	# 4. Both lines running.
	for i in range(BBQ_SETTLE_TICKS):
		if i % RESTOCK_EVERY == 0:
			restock(sim)
		sim.tick()


## THE SPLIT DOCK LAMPS — `scripts/screenshot-scene.sh --split-docks`: the
## BBQ branch run on until ONE of its two docks is backing up and the other
## is not, which is the whole subject of the per-dock lamp change.
##
## Built on `build_bbq_running_line` because it is the only fixture with two
## docks shipping two products into the one shared pool. BBQ is the sauce
## that caps: the branch out-produces the orders on the board while the
## shipped line is throttled to its share of the vat (the same asymmetry
## `tests/bbq_branch.rs` relies on).
##
## The wait is long, and has to be: the board keeps BUYING. Ketchup sits
## around 2 in the pool for the whole run because its contracts settle as
## fast as the line ships, and BBQ climbs only ~3 per 4 000 ticks against
## what the two BBQ orders take — so the ceiling arrives somewhere past tick
## 70 000. That gap is the point of the shot rather than an inconvenience:
## one product outruns its demand, the other does not, and only one dock
## should say so.
##
## Returns with the docks disagreeing, or having run out of patience — the
## caller checks `split_dock_lamps_found` rather than trusting the frame.
const SPLIT_DOCK_MAX_TICKS := 140000


static func build_split_dock_line(sim) -> void:
	build_bbq_running_line(sim)
	if fixture_failure() != "":
		return
	for i in range(SPLIT_DOCK_MAX_TICKS):
		if i % RESTOCK_EVERY == 0:
			restock(sim)
		sim.tick()
		if split_dock_lamps_found(sim):
			return


## Whether the docks currently DISAGREE about backing up — one jammed, one
## clear. The sim's own per-dock verdict (`DockSnapshot::full`), never a
## pool level compared here, for `delivery_full`'s reason above.
static func split_dock_lamps_found(sim) -> bool:
	var docks = JSON.parse_string(sim.dock_snapshots_json())
	if not (docks is Array) or docks.size() < 2:
		return false
	var jammed := 0
	for dock in docks:
		if dock is Dictionary and bool(dock.get("full", false)):
			jammed += 1
	return jammed > 0 and jammed < docks.size()


## THE STALLED RUN (issue #54) — the fixture `test_pipe_fault.gd` asserts
## and `scripts/screenshot-scene.sh --stalled-run` photographs: the BBQ
## branch exactly as it ships, with ONE mistake made in it.
##
## The mistake is the one the branch was actually built with. A fluid tank
## discharges only into the pipe it FACES, so the liquid-smoke tank at
## (16, -1), turned off its run, stops pinning that run to liquid smoke; the
## run narrows to two fluids instead of one, the simulation refuses to guess
## which, and every machine on it waits forever. Nothing about it used to be
## visible: an unnamed run and a run merely waiting for its first fill are
## both empty pipe.
##
## Deliberately NOT run forward. The comparison the shot has to make is
## between a DEAD run and a merely EMPTY one, and a settled factory has no
## empty runs left in it — so the whole branch stands unfilled, the smoke run
## flagged and its neighbours, equally empty, not.
static func build_stalled_pipe_run(sim) -> void:
	build_bbq_branch(sim)
	sim.issue_command_json(JSON.stringify({
		"type": "rotate", "tile": [STALLED_TANK.x, STALLED_TANK.y],
	}))


## The liquid-smoke tank `build_stalled_pipe_run` turns away from its run,
## the first tile of the run that is left dead, and a tile of the run beside
## it that is merely empty — the three things any check or shot of that
## fixture needs to name.
const STALLED_TANK := Vector2i(16, -1)
const STALLED_RUN_TILE := Vector2i(18, -1)
const STALLED_NEIGHBOUR_TILE := Vector2i(15, -3)
## What the dead run narrowed to, in the order the simulation reports it.
const STALLED_RUN_CANDIDATES := ["ketchup_mix", "liquid_smoke"]


## Hires into every category the simulation reports short, until none is.
static func hire_until_staffed(sim) -> void:
	for _attempt in range(64):
		var short: Array = []
		for category in JSON.parse_string(sim.staff_snapshot_json()).get("categories", []):
			if int(category.get("short", 0)) > 0:
				short.append(String(category.get("id", "")))
		if short.is_empty():
			return
		for category in short:
			sim.issue_command_json(JSON.stringify({"type": "hire", "category": category}))


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
const BOTTLENECK_EXTRA_TOMATOES := 300
## What `restock` holds this line at: the shared larder, with the crusher's
## appetite for tomato on top.
const BOTTLENECK_LARDER := {"tomato": LARDER_STOCK + BOTTLENECK_EXTRA_TOMATOES}
const BOTTLENECK_MACHINES := {
	"4,0": "blocked", # crusher
	"9,0": "blocked", # mixer
	"12,0": "working", # cooking vat
	"16,0": "starved", # bottler
	"21,0": "starved", # packer
}

static func build_bottleneck_line(sim) -> void:
	build_reference_line(sim)
	# The line's bill leaves ~1900, and the larder plus the warehouse it now
	# needs costs more than that, so top the balance up.
	fund(sim, 8000)
	stock_larder(sim, LARDER_STOCK)
	stock(sim, "tomato", BOTTLENECK_EXTRA_TOMATOES)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true,
	}))
	for i in range(BOTTLENECK_SETTLE_TICKS):
		if i % RESTOCK_EVERY == 0:
			restock(sim, LARDER_STOCK, BOTTLENECK_LARDER)
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
	stock_larder(sim, LARDER_STOCK)
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
		if i % RESTOCK_EVERY == 0:
			restock(sim)
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


## THE UNCLERKED DOCK (issue #87) — the reference line with its ONE
## logistics head fired, photographed in the window where the dock has not
## yet been asked for a clerk. `scripts/screenshot-scene.sh
## --unclerked-dock --hover-dock` is the repro frame.
##
## `Dock::understaffed` only moves on a tick the dock actually held
## something and was refused, so between firing the clerk and the first box
## arriving — which on a cold line is hundreds of ticks, and on a line that
## is also starved upstream is forever — the dock reported nothing wrong at
## all while the establishment could not crew it. That silent window is the
## subject: the hover has to say "short of logistics staff" from the
## STANDING `crewed` fact, not wait for a refusal.
##
## Built deliberately short: the line is placed, the head is fired, and the
## sim is ticked only far enough to settle the scene. Running it on would
## eventually latch `understaffed` and photograph the other case, which
## `--full-dock` already covers.
const UNCLERKED_CATEGORY := "logistics"
const UNCLERKED_SETTLE_TICKS := 20

static func build_unclerked_dock(sim) -> void:
	build_reference_line(sim)
	fund(sim, 8000)
	sim.issue_command_json(JSON.stringify({
		"type": "fire", "category": UNCLERKED_CATEGORY,
	}))
	for i in range(UNCLERKED_SETTLE_TICKS):
		sim.tick()


## Whether EVERY placed dock is in the silent window this fixture exists
## for: the establishment cannot crew it (`crewed` false) and yet it was
## not refused on the last tick it mattered (`understaffed` false), so
## before issue #87 the hover said nothing at all. Read off the POLLED dock
## snapshot, which is the channel the fix put `crewed` on.
##
## Asked of every dock rather than of a named tile on purpose: the caller
## hovers whichever dock the scene found first, and a hardcoded coordinate
## here would go quietly wrong the day the reference line moves.
static func unclerked_dock_is_silent(sim) -> bool:
	var parsed = JSON.parse_string(sim.dock_snapshots_json())
	if not (parsed is Array) or parsed.is_empty():
		return false
	for entry in parsed:
		if not (entry is Dictionary):
			return false
		if bool(entry.get("crewed", true)) or bool(entry.get("understaffed", false)):
			return false
	return true


## THE UNDERFUNDED LINE (the quality pass, ticket 4) — the fixture that
## CANNOT pay its next bill, for `test_staff_panel.gd`'s BEFORE-payday
## warning and `screenshot-scene.sh --staff-shortfall`. The shipped line's
## starting crew bills 14 a period (manufacturing 4, packaging 2, logistics
## 1, each at wage 2); `fund` to 8 leaves it short by exactly 6, and paying
## the cascade in `staff.ron` order spends the whole balance on
## manufacturing's four heads before packaging's first — packaging's 2 and
## logistics' 1 all go unpaid, 3 in total. Those two numbers (6 short, 3
## unpaid) are the ticket's own example wording, so this fixture is what
## makes that sentence literally true rather than merely plausible. Placed,
## not empty, so the panel's WHY line has something to say too.
static func build_underfunded_line(sim) -> void:
	build_reference_line(sim)
	fund(sim, 8)


## THE FULL-DOCK LINE (the dock-capacity change, 2026-09-16) — the fixture
## `scripts/screenshot-scene.sh --full-dock` photographs: the reference line
## on auto-fill, run until the shared delivery pool is at its ceiling and
## the dock has to REFUSE the belt feeding it.
##
## Nothing here is broken. The pool holds `DELIVERY_STOCK_CAPACITY` (48) of
## any one item, contracts are the only thing that drains it, and a line
## making boxes faster than its contracts ask for them fills that pool up —
## after which the dock stops taking, the belt behind it backs up, the
## clipboard's ON THE DOCK line reads `48/48` in warning red with
## `⚠ FULL — belt backing up` under it, and the dock's deck lamps switch
## from steady to PULSING (`main.gd`'s `_dock_lamp_lit`). Those are the two
## presentation states the dock-capacity work added and nothing could
## photograph on demand.
##
## Reaching it takes ~22 900 ticks against the starter contract's appetite —
## it asks for 2 boxes every 1 000 and the line makes rather more than that
## (`sim_core/tests/dock.rs`'s
## `a_full_pool_backs_the_line_up_instead_of_losing_product` measures the
## same climb) — so this waits for the STATE and caps the wait, the way
## `build_understaffed_line` does: a sim change that stops the pool ever
## filling should leave a note rather than hang the shot.
##
## The larder is stocked AFTER the balance is topped up — `stock` is
## refused outright when the money is not there, and the shipped line's own
## bill leaves ~1 900 of the 3 000 a new game starts with — and it is topped
## up as the run goes, rather than bought deep. One larder is 320 of each
## raw and the whole climb eats several thousand, which before W1 was simply
## bought up front; a holding is capped now, and 3 000 of one ingredient is
## sixty bays, so this fixture would have photographed a wall of racks
## instead of a dock. `restock` inside the loop is both smaller and truer to
## how a player gets there.
const FULL_DOCK_FUNDS := 30000
const FULL_DOCK_MAX_TICKS := 40000

static func build_full_dock_line(sim) -> void:
	build_reference_line(sim)
	# Funded BEFORE the stocking, not after: `fund` SETS the balance rather
	# than adding to it, so funding second would leave the larder empty and
	# then hand the empty line a fortune. The stocking spends ~18 000 of
	# this at the shipped prices, and ~40 paydays cost 560 of the rest.
	fund(sim, FULL_DOCK_FUNDS)
	stock_larder(sim, LARDER_STOCK)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true,
	}))
	for i in range(FULL_DOCK_MAX_TICKS):
		if i % RESTOCK_EVERY == 0:
			restock(sim)
		sim.tick()
		if delivery_full(sim):
			return


## Whether the shared delivery pool is at its ceiling — the sim's own
## verdict off `economy_snapshot_json`, never a count compared here against a
## capacity read separately.
static func delivery_full(sim) -> bool:
	var econ = JSON.parse_string(sim.economy_snapshot_json())
	return econ is Dictionary and bool(econ.get("delivery_full", false))


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


## Sets the lifetime delivery counter — the one number every progression
## gate in `data/unlocks.ron` is resolved against — through the same
## save/load door `fund` uses, and for the same reason: the counter is
## simulation state with no command that sets it, and the alternative is
## running a Godot fixture for the ten thousand ticks it takes to earn ten
## deliveries honestly. `sim_core/tests/unlocks.rs` is what earns them
## honestly; a fixture here only needs to be ON one side of the gate or the
## other so a panel can be photographed in both states.
##
## `save_ron` writes it as a plain integer (`lifetime_deliveries: N`), and a
## save from before the field existed simply has no such line — so this
## inserts one rather than only substituting, and a world already past the
## gate is left alone by the substitution path.
static func deliver(sim, deliveries: int) -> void:
	var text: String = sim.save_ron()
	var regex := RegEx.new()
	regex.compile("lifetime_deliveries:\\s*\\d+")
	var patched: String = text
	if regex.search(text) != null:
		patched = regex.sub(text, "lifetime_deliveries: %d" % deliveries)
	else:
		push_error("deliver(): this save has no lifetime_deliveries field")
		return
	var error: String = sim.load_ron(patched)
	if error != "":
		push_error("deliver(): load_ron refused the patched save: %s" % error)


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


## THE ITEM GALLERY (the second-sauce epic, `docs/BBQ_SAUCE_PLAN.md`) — a
## bare straight belt run for `test_screenshot.gd --item-gallery` to stand one
## of each finished-goods model on, side by side, at the size an item really
## renders. 80 of a 3,000 starting balance.
##
## A belt run rather than an empty floor because an item is only ever judged
## ON a belt: half a tile of product against black rubber is the contrast a
## player actually reads, and `_carried_item_transform` is what knows where
## on a tile it stands. Nothing feeds it, so what is on it is exactly what
## the screenshot put there — which is the whole point, since this fixture
## exists to photograph models for items `data/recipes.ron` does not
## necessarily make yet.
##
## Deliberately one belt per item with a gap between them: adjacent items on
## a real belt touch, and two cases nose to nose would be reviewed as one
## silhouette instead of two.
const ITEM_GALLERY_SLOTS := 4
static func build_item_gallery(sim) -> void:
	for x in range(ITEM_GALLERY_SLOTS * 2):
		place(sim, "belt", Vector2i(x, 0), "east")


## Which tile the nth item-gallery slot stands on. Every other tile, so the
## run reads as a belt rather than as a shelf.
static func item_gallery_tile(slot: int) -> Vector2i:
	return Vector2i(slot * 2, 0)


## THE CROSSING LINE (the belt/pipe crossing epic, ticket T3) — a live belt
## with two live pipe runs crossing it, every tile of it bought and placed
## through the real commands, so the two crossings in it are ones the
## SIMULATION assembled (`docs/BELT_PIPE_CROSSING_PLAN.md`). It replaced the
## spike's hand-stood models, which photographed art the sim could not build.
##
## One belt east along y = 0, fed by a tomato hopper and ending in nothing,
## so product runs under both crossings and then backs up beneath them — a
## crossing is judged with things passing under it. Two north-south pipe runs
## cross it, one assembled each way the game allows:
##
##   * x = CROSSING_PIPE_ONTO_BELT_X — the belt row is laid first, then the
##     pipe run, so the tile is a pipe laid ACROSS a belt;
##   * x = CROSSING_BELT_ONTO_PIPE_X — the pipe run is laid first and the belt
##     row across it, so the tile is a belt laid across a pipe.
##
## The belt is fed tomatoes by an auto-filling hopper and runs into nothing,
## so once it has been RUN it backs up and every tile — the two under the
## pipes included — holds items to be seen passing under the arch. It has to
## be run for that: the fixture only builds, so `--fixture crossing` settles
## it through `_run_chain_forward` (`test_screenshot.gd`) and the tests tick
## it themselves. Built and not ticked, this line is a bare belt and two grey
## pipe runs.
##
## **Each run is fed by ONE tank, at its north end only.** Feeding both ends
## proves nothing: both halves of a run colour whether or not the crossing
## joined them, which is the one fact the colour is here to show. Fed from
## the north alone, fluid reaches the south half only by passing THROUGH the
## crossing, so the colour is the proof.
##
## That is a legibility choice and not a fill fix — how much each run gets is
## [constant CROSSING_VINEGAR]'s business, and the two were confused once
## already. The line costs 1,335 of a 3,000 starting balance (read off the
## HUD: 1,665 left after the build).
static func build_crossing_line(sim) -> void:
	stock(sim, "vinegar", CROSSING_VINEGAR)
	stock(sim, "tomato", 64)
	for y in CROSSING_RUN_YS:
		place(sim, "pipe", Vector2i(CROSSING_BELT_ONTO_PIPE_X, y), "north")
	place(sim, "hopper", Vector2i(-1, 0), "east", "tomato")
	for x in range(CROSSING_BELT_TILES):
		place(sim, "belt", Vector2i(x, 0), "east")
	sim.issue_command_json(JSON.stringify({
		"type": "set_hopper_auto_fill", "tile": [-1, 0], "enabled": true,
	}))
	for y in CROSSING_RUN_YS:
		place(sim, "pipe", Vector2i(CROSSING_PIPE_ONTO_BELT_X, y), "north")
	for x in [CROSSING_PIPE_ONTO_BELT_X, CROSSING_BELT_ONTO_PIPE_X]:
		var tank := Vector2i(x, CROSSING_RUN_YS[0] - 1)
		place(sim, "fluid_tank", tank, "north")
		sim.issue_command_json(JSON.stringify({
			"type": "set_tank_fluid", "tile": [tank.x, tank.y], "fluid": "vinegar",
		}))
		sim.issue_command_json(JSON.stringify({
			"type": "set_tank_auto_fill", "tile": [tank.x, tank.y], "enabled": true,
		}))


## The two crossing columns, shared with `test_screenshot.gd` and the tests
## so the fixture and what checks it cannot drift apart.
const CROSSING_PIPE_ONTO_BELT_X := 3
const CROSSING_BELT_ONTO_PIPE_X := 9
const CROSSING_BELT_TILES := 13
const CROSSING_RUN_YS := [-3, -2, -1, 0, 1, 2, 3]
## Vinegar bought for the line's two tanks. Measured with the tanks' own
## auto-fill as the only source, 200 ticks, reading `fluid_snapshots_json`:
##
##   64 -> x = 3 full, x = 9 pressure 0.000   (what shipped)
##   128 -> x = 9 0.114    160 -> x = 9 0.571    200 -> x = 9 0.999
##
## The FIRST tank drinks whatever it can reach, so a small number leaves the
## second run dry no matter how the tanks are arranged — 64 across FOUR tanks
## was dry at x = 9 as well. The tank count was never what starved it; this
## number was. [method crossing_line_idle] checks the result.
const CROSSING_VINEGAR := 200

## The pressure below which a run photographs grey however wet the sim says
## it is. `main.gd` draws pipe liquid by scaling a ONE-VOXEL body by
## `pressure * PIPE_BORE_HEIGHT_VOXELS` (7), so under 1/7 the column is
## sub-voxel: the node is still visible — `FLUID_MIN_VISIBLE_PRESSURE` (0.02)
## only suppresses a z-fighting sliver — and nothing reads in the frame.
##
## This is here because `volume > 0` was not enough. The x = 9 run sat at
## 0.114, eight tenths of a voxel, and passed a wet-in-every-segment check
## while photographing entirely grey. A guard that goes green on the bad
## picture is no guard.
const CROSSING_MIN_DRAWN_PRESSURE := 1.0 / 7.0
## An ordinary belt tile of the line with open floor either side of it along
## the belt — where `--fixture crossing`'s ghost frames hold a pipe, across
## the belt (a crossing) or along it (refused).
const CROSSING_GHOST_TILE := Vector2i(6, 0)


## Why the crossing line did not come out as two crossings, or "" if it did —
## read back from the simulation's own `query_tile_json`, never from what
## this file meant to build. A Place the sim refused is silent, so a rule
## change that stopped either assembly would otherwise hand back a frame of a
## belt running past two gaps.
static func crossing_line_failure(sim) -> String:
	for x in [CROSSING_PIPE_ONTO_BELT_X, CROSSING_BELT_ONTO_PIPE_X]:
		var parsed = JSON.parse_string(sim.query_tile_json(x, 0))
		var kind := String(parsed.get("kind", "")) if parsed is Dictionary else ""
		if kind != "crossing":
			return "(%d, 0) is '%s', not a crossing" % [x, kind]
	return ""


## Why the crossing line is not SHOWING what it was built to show, or "" if
## it is — asked after the settle, not after the build, and separate from
## [method crossing_line_failure] because they fail for opposite reasons: that
## one says the sim refused to assemble a crossing, this one says the line
## assembled and then stood there dead.
##
## Both are needed because both have happened, and the second of them took
## two goes to catch. The line shipped for a day with nothing ticking it, so
## every frame was a bare belt under grey pipe runs and every frame passed.
## The first version of THIS check then asked only that each segment hold
## some fluid, and passed a frame in which the x = 9 run was grey from end to
## end — wet in the sim at 0.114 pressure, sub-voxel on screen. So it asks
## for a level that DRAWS ([constant CROSSING_MIN_DRAWN_PRESSURE]), not a
## level above zero.
##
## A bare belt and a grey pipe run look like art that has not landed, not
## like a fixture nobody ran. A frame is evidence in a PR body, so the
## harness fails rather than hand one over.
static func crossing_line_idle(sim) -> String:
	for x in [CROSSING_PIPE_ONTO_BELT_X, CROSSING_BELT_ONTO_PIPE_X]:
		var parsed = JSON.parse_string(sim.query_tile_json(x, 0))
		var tile: Dictionary = parsed if parsed is Dictionary else {}
		if Array(tile.get("items_here", [])).is_empty():
			return "no item is on the crossing at (%d, 0) — the belt was never run" % x
	var runs := 0
	for snapshot in JSON.parse_string(sim.fluid_snapshots_json()):
		var segments: Array = snapshot.get("segments", [])
		if segments.is_empty():
			continue
		runs += 1
		var run_x := int(segments[0]["tile"][0])
		if segments.size() != CROSSING_RUN_YS.size():
			return "the pipe run at x = %d has %d segments, not the %d it was built with" % [
				run_x, segments.size(), CROSSING_RUN_YS.size(),
			]
		var faint := 0
		for segment in segments:
			if float(segment.get("pressure", 0.0)) < CROSSING_MIN_DRAWN_PRESSURE:
				faint += 1
		if faint > 0:
			return "the pipe run at x = %d is too faint to draw in %d of its %d segments (under %.3f pressure)" % [
				run_x, faint, segments.size(), CROSSING_MIN_DRAWN_PRESSURE,
			]
	# Counted rather than assumed: every check above is inside a loop over the
	# runs the sim reports, so a line that reported NO runs at all would walk
	# none of them and return "" — a pass earned by having nothing to look at.
	if runs != 2:
		return "the line carries fluid in %d pipe runs, not the 2 it was built with" % runs
	return ""


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


## THE FLUID GALLERY (issue #72) — all SIX fluids the game has, standing in
## six pipe runs in one frame, so the question "are these six colours
## actually tellable apart?" can be answered by looking at a picture instead
## of by reasoning about six hex values.
##
## `main.gd`'s FLUID_TINTS grew from two entries to six across the BBQ epic
## and was never once seen whole: the shipped line shows three of them, the
## BBQ fixture four, and nothing showed `bbq_mix` beside `ketchup_mix` or
## `bbq` beside `ketchup` — which are exactly the two pairs most likely to
## collide, both being dark red-browns. The colours were reviewed in
## isolation, which is not a review (UI_UX.md §2b point 9).
##
## **Naming a run is not the same as filling it, and this fixture does both
## differently.** A run's fluid id comes from the MACHINE RECIPES either end
## of it (`sim::world::resolve_network_fluids`), not from anything that has
## ever flowed through it, so the naming here is pure topology and is exact
## the moment the last pipe is placed — nothing has to run. The layout is
## chosen so that each of the six runs narrows to exactly one candidate:
##
##   tank -> [A] -> mixer -> [B] -> vat -> [C] -> mixer -> [D] -> vat -> [F]
##                                                          tank -> [E] ->
##
##   * A is tank-fed, so it is narrowed to RAW fluids, and the one raw fluid
##     a mixer draws is vinegar.
##   * B is filled by that mixer, whose two recipes make `ketchup_mix` and
##     `bbq_mix`; `bbq_mix` needs ketchup on an input run and this mixer's
##     only input run is A, so `ketchup_mix` is the only feasible one.
##   * C is filled by a vat fed only by B, so `bbq` (which needs `bbq_mix`
##     AND liquid smoke, on two distinct runs) is out and it is `ketchup`.
##   * D is filled by the second mixer, fed only by C; `ketchup_mix` needs
##     vinegar, so it is `bbq_mix`.
##   * E is the second tank's run, drawn by the second vat: raw again, and
##     the one raw fluid a vat draws is `liquid_smoke`.
##   * F leaves that vat, which now has `bbq_mix` and `liquid_smoke` on two
##     distinct input runs and no `ketchup_mix` anywhere: `bbq`.
##
## PRESSURE is the other half, and it is set directly rather than earned.
## Priming these runs by simulation would mean two full ketchup sub-chains
## feeding two mixers, a 600-tick BBQ cook and something like fifteen
## thousand ticks — and the intermediate runs would still be draining into
## the next machine at the moment of the shot, which is the opposite of what
## a colour comparison wants. `fill_fluid_runs` below sets every volume
## through the save/load door instead. That makes this a still life and not
## a working factory, exactly as the pipe and belt galleries are.
##
## Returns `check_fluid_gallery`'s complaint, or "" when the gallery really
## is one. **The caller must refuse on a complaint** — `test_screenshot.gd`
## writes no PNG and exits 1 — because every way this fixture degrades
## leaves a frame that still looks like a factory (see `check_fluid_gallery`
## below). It is returned rather than acted on here so that a headless
## caller with no screenshot to suppress can decide for itself.
##
## READING the result: at the fixture's own framing this frame answers
## "all six exist, named and standing" well, and "are these six tellable
## apart" only roughly — two of the runs sit ~600px apart rather than side
## by side. `--fluid-gallery --closeup <x>,<y>,6` is the route that answers
## the second question, one run at a time at 1:1.
static func build_fluid_gallery(sim) -> String:
	# Run A, vinegar: a tank and two pipes into the first mixer's west side.
	place(sim, "fluid_tank", Vector2i(0, 0), "east")
	for x in [1, 2]:
		place(sim, "pipe", Vector2i(x, 0), "east")
	place(sim, "mixer", Vector2i(3, 0), "east")  # 1x2: also covers y=1

	# Run B, ketchup mix: off the mixer's outlet face into the first vat.
	for x in [4, 5]:
		place(sim, "pipe", Vector2i(x, 0), "east")
	place(sim, "cooking_vat", Vector2i(6, 0), "east")  # 2x2: also x=7, y=1

	# Run C, ketchup: out of that vat and into the second mixer.
	for x in [8, 9]:
		place(sim, "pipe", Vector2i(x, 0), "east")
	place(sim, "mixer", Vector2i(10, 0), "east")

	# Run D, BBQ mix: into the second vat.
	for x in [11, 12]:
		place(sim, "pipe", Vector2i(x, 0), "east")
	place(sim, "cooking_vat", Vector2i(13, 0), "east")  # 2x2: also x=14, y=1

	# Run E, liquid smoke. The tank TOUCHES the vat's north-west tile, which
	# is how a machine taps a network (see `build_reference_line`), and the
	# run keeps to row 2 so it never touches run D on row 0 — a tile closer
	# and the two networks would merge into one unnameable run.
	place(sim, "fluid_tank", Vector2i(13, 2), "east")
	for x in [14, 15]:
		place(sim, "pipe", Vector2i(x, 2), "east")

	# Run F, BBQ: out of the second vat, with row 1 left empty between it
	# and run E for the same reason.
	for x in [15, 16]:
		place(sim, "pipe", Vector2i(x, 0), "east")

	fill_fluid_runs(sim)
	var complaint := check_fluid_gallery(sim)
	if complaint != "":
		# Said out loud as well as returned: `push_error` is what a Godot
		# log shows, and the return value is what actually stops the run.
		# It is deliberately NOT the only thing that happens — issue #72's
		# first cut pushed this error and then let the harness photograph
		# the degraded gallery anyway, exit 0, which is the #70/#75 failure
		# mode this repo has now fixed three times.
		push_error("build_fluid_gallery(): %s" % complaint)
	return complaint


## How many placements `build_fluid_gallery` makes: 12 pipes, 2 tanks,
## 2 mixers and 2 vats — 816 of the 3000 a new game starts with.
const FLUID_GALLERY_PLACEMENTS := 12 + 2 + 2 + 2

## Every fluid the game has. The gallery's whole claim is that it stands all
## of them up at once, so the list it is checked against is written here
## once rather than recounted from the layout.
const FLUID_GALLERY_FLUIDS: PackedStringArray = [
	"vinegar", "ketchup_mix", "ketchup", "bbq_mix", "liquid_smoke", "bbq",
]


## Asks the simulation whether the gallery is actually a gallery: all six
## fluids present, each on a run the sim can NAME (`carries`, not `ambiguous`
## or `unnameable`), each above the pressure `main.gd` will draw a body at.
## Returns "" when it is, or what is wrong with it.
##
## This exists because every way this fixture can fail is a quiet one. Move
## one pipe a tile and two runs merge into a single unnameable network; add
## a machine and a run goes ambiguous; get the fill wrong and a run is named
## but invisible. In all three cases the harness still renders a perfectly
## good-looking factory, and the frame that comes back is simply missing a
## colour the reviewer was told it contained — which is worse than no frame,
## because it looks like an answer.
static func check_fluid_gallery(sim) -> String:
	var snapshots = JSON.parse_string(sim.fluid_snapshots_json())
	if not (snapshots is Array):
		return "fluid_snapshots_json did not return an array"
	var seen := {}
	for run in snapshots:
		var fluid := String(run.get("fluid", ""))
		var naming := String(run.get("naming", ""))
		if naming != "carries":
			return "a run is '%s' rather than carrying one fluid — the layout has merged or confused two networks" % naming
		if float(run.get("pressure", 0.0)) <= FLUID_MIN_VISIBLE_PRESSURE:
			return "the %s run is at pressure %s, at or below the %s main.gd needs to draw a body" % [
				fluid, run.get("pressure", 0.0), FLUID_MIN_VISIBLE_PRESSURE,
			]
		seen[fluid] = true
	var missing := []
	for fluid in FLUID_GALLERY_FLUIDS:
		if not seen.has(fluid):
			missing.append(fluid)
	if not missing.is_empty():
		return "no run carries %s (the gallery has %d runs, and the point of it is all %d fluids at once)" % [
			", ".join(missing), snapshots.size(), FLUID_GALLERY_FLUIDS.size(),
		]
	return ""


## `main.gd`'s own floor for drawing a fluid body at all, restated as a
## number here rather than reached through the scene: these are static
## helpers with no `main` to ask.
const FLUID_MIN_VISIBLE_PRESSURE := 0.02


## Stands liquid in EVERY pipe and tank in the world at once, through the
## same save/load door `fund` and `deliver` use and for the same reason: a
## volume is simulation state and no player command sets one directly. The
## honest alternative is to run the factory, and for a fixture whose whole
## job is to hold six colours still side by side that is both far more
## expensive and actively wrong — a running line is a line in which half the
## runs are momentarily empty.
##
## `save_ron` writes both a pipe's and a tank's contents as `volume_bits`
## (raw `Fx` bits, I16F16, so N = units * 65536), and load VALIDATES the
## result against each buffer's real capacity — so this fills to
## `PIPE_SEGMENT_CAPACITY`, the smaller of the two, and a tank simply comes
## back a twelfth full rather than refused. Full, not a trickle, because
## `main.gd` scales a fluid body's height by pressure: at the 0.02 floor of
## FLUID_MIN_VISIBLE_PRESSURE there is one voxel of colour to judge.
static func fill_fluid_runs(sim, units := 10) -> void:
	var text: String = sim.save_ron()
	var regex := RegEx.new()
	regex.compile("volume_bits:\\s*-?\\d+")
	if regex.search(text) == null:
		push_error("fill_fluid_runs(): the save has no volume_bits field — nothing to fill")
		return
	var patched := regex.sub(text, "volume_bits: %d" % (units * 65536), true)
	var error: String = sim.load_ron(patched)
	if error != "":
		push_error("fill_fluid_runs(): load_ron refused the patched save: %s" % error)
