# Headless proof that the SECOND SAUCE is a real factory in a real scene (the
# BBQ epic) — that a player can buy and place the branch, that the renderer
# draws every tile of it, and that boxes of BBQ come off the packer and reach
# the dock.
#
# Run with:
#   godot --headless --path godot_project --script test_bbq_branch.gd
#
# WHY THIS EXISTS BESIDE `sim_core/tests/bbq_branch.rs`. That file already
# runs the branch and already asserts the boxes. What it cannot see is the
# half this one is for: the branch is content plus plumbing, and both halves
# of it cross the gdext JSON boundary before anything is drawn. A pipe run
# that cannot decide what it carries leaves its machines silently stalled
# rather than erroring (`world.rs`, "What a pipe run carries"), a hopper
# placed without an `item` is refused in silence, and a recipe a machine kind
# cannot pick shows up as a factory that merely looks idle. Every one of those
# reads, from the outside, as "the screenshot is boring" — so this asserts it
# in words instead.
#
# Verifies:
#   * THE WHOLE BRANCH IS AFFORDABLE AND PLACEABLE from a new game's 3000:
#     70 placements land, the shipped ketchup line's 37 among them, unmoved,
#   * BOTH SAUCES RENDER: the BBQ mixer, vat, bottler, packer and dock are
#     all drawn at the tiles the simulation reports, and the 1x2 mixer and
#     2x2 vat each cover their whole footprint,
#   * EVERY PIPE RUN KNOWS WHAT IT CARRIES — six of them, named, including
#     the single tap tile at (14, -1) that reads `ketchup` because it hangs
#     off the ketchup line's outlet run, and the BBQ mixer's own outlet run
#     that reads `bbq_mix` even though a mixer could make either,
#   * EVERY RUN IS DRAWN AS ITS OWN LIQUID: each of the six fluid ids has a
#     `FLUID_TINTS` entry of its own and no two of the six are the same
#     colour (issue #68 — three of them fell through to the neutral fallback,
#     so the whole BBQ half of the factory ran pale blue water),
#   * AND THE SAUCE A MACHINE SHOWS IS THAT RECIPE'S: the two bottlers and
#     the two packers are one model each, whose product mesh is tinted from
#     the live recipe (`PRODUCT_TINTS`), so the BBQ pair and the ketchup
#     pair come out different colours — they were both ketchup until
#     2026-09-21, the sauce having been painted into the voxels,
#   * A MACHINE PICKS ITS RECIPE FROM ITS PLUMBING: the two mixers are the
#     same kind on the same board and one makes `ketchup_mix` while the other
#     makes `bbq_mix`, with no picker anywhere in the UI,
#   * A BOX OF BBQ IS MADE AND SHIPPED: `boxed_bbq` appears in the production
#     panel's own snapshot and in the dock's delivery pool,
#   * THE SAUCE IS SPELLED THE WAY A PLAYER WOULD WRITE IT: every BBQ id
#     the HUD shows a label for comes back "BBQ", not the title-cased
#     "Bbq" the formatter alone produces (issue #60),
#   * THE CONTRACT BOARD OFFERS A MIXED ORDER — one docket asking for two
#     different products at once, which is what the board could not express
#     before this epic.
#
# The one-frame-before-checking dance is inherited from test_hover.gd — see
# that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## How long to run, and what to stock it with.
##
## Both numbers are decided by the 3000 a new game starts with. The two lines
## cost 1831 of it, which leaves 1169 for raws and for the starting crew's
## wages — and a factory that runs out of money loses its staff, so this is a
## real ceiling rather than a guess. 12 000 ticks is comfortably past the
## branch's first box (the BBQ cook is 600 ticks and sits behind a 240-tick
## ketchup vat that has to prime two pipe runs first) and the basket below
## costs 823, leaving room for twelve paydays.
##
## The ratio between the two lines is NOT asserted here. Measuring it needs a
## fully-staffed factory run well past the priming transient, which needs more
## money than a new game has; `sim_core/tests/bbq_branch.rs` funds a fixture
## properly and pins the 3:2 split there.
const RUN_TICKS := 12000
const STOCK := {
	"tomato": 211, "sugar": 48, "vinegar": 96,
	"spice_mix": 24, "molasses": 24, "liquid_smoke": 20,
}

## The branch's own tiles, from `sim::bbq_branch`.
const BBQ_MIXER := Vector2i(14, -3)
const BBQ_MIXER_FAR := Vector2i(14, -2)
const BBQ_VAT := Vector2i(17, -3)
const BBQ_VAT_FAR := Vector2i(18, -2)
const BBQ_BOTTLER := Vector2i(21, -3)
const BBQ_PACKER := Vector2i(26, -3)
const BBQ_DOCK := Vector2i(31, -3)
const KETCHUP_TAP := Vector2i(14, -1)
const BBQ_MIX_RUN := Vector2i(15, -3)
const SMOKE_RUN := Vector2i(18, -1)
const BBQ_RUN := Vector2i(19, -3)

## How far apart two fluids' tints have to be, summed over RGB, to count as
## two liquids rather than one (issue #68). Low on purpose: this is a guard
## against a forgotten or copied entry, not a judgement about colour — what
## the six actually look like side by side is a rendered-frame review, which
## is how the pale-blue BBQ runs were found in the first place.
const FLUID_TINT_MIN_DISTANCE := 0.08

## The machines that SHOW their product, paired ketchup-against-BBQ: one pair
## per kind, each entry a tile and the recipe that tile must be seen running.
## Two of a kind is the whole point — a table of tints cannot be wrong for a
## factory that only makes one sauce.
const PRODUCT_MACHINE_PAIRS := [
	[[Vector2i(12, 0), "ketchup"], [BBQ_VAT, "bbq"]],
	[[Vector2i(16, 0), "bottled_ketchup"], [BBQ_BOTTLER, "bottled_bbq"]],
	[[Vector2i(21, 0), "boxed_ketchup"], [BBQ_PACKER, "boxed_bbq"]],
]
## How often to ask those six what they are making. Every 50 ticks is a
## little under two seconds of play and a few hundred reads across the run —
## cheap beside the ticks themselves, and dense enough that a stage which
## only runs in bursts is still caught in one.
const PRODUCT_SAMPLE_TICKS := 50


var main: Node
var _ran := false
## origin key -> the set of recipe ids that machine was seen running.
var _product_recipes: Dictionary = {}


func _fail(msg: String) -> void:
	printerr("BBQ_BRANCH_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


func _placement_kind(tile: Vector2i) -> String:
	return String(main._placement_at_tile(tile).get("kind", ""))


func _placement_recipe(tile: Vector2i) -> String:
	return String(main._placement_at_tile(tile).get("recipe", ""))


func _fluid_on(tile: Vector2i) -> String:
	for network in JSON.parse_string(main.sim.fluid_snapshots_json()):
		for segment in network.get("segments", []):
			if int(segment["tile"][0]) == tile.x and int(segment["tile"][1]) == tile.y:
				return String(network.get("fluid", ""))
	return ""


## Records what each product-showing machine is making right now — see
## PRODUCT_MACHINE_PAIRS. Read straight off `query_tile_json` rather than
## through a world rebuild: this runs inside the tick loop, and a rebuild
## there would cost more than the ticks it samples.
func _sample_product_recipes() -> void:
	for pair in PRODUCT_MACHINE_PAIRS:
		for entry in pair:
			var tile: Vector2i = entry[0]
			var parsed = JSON.parse_string(main.sim.query_tile_json(tile.x, tile.y))
			if parsed == null or parsed.get("recipe") == null:
				continue
			var key: String = main._tile_key(tile)
			if not _product_recipes.has(key):
				_product_recipes[key] = {}
			_product_recipes[key][String(parsed["recipe"])] = true


func _made(item_id: String) -> int:
	for entry in JSON.parse_string(main.sim.production_snapshot_json()).get("items", []):
		if String(entry.get("id", "")) == item_id:
			return int(entry.get("produced_total", 0))
	return 0


func _delivered(item_id: String) -> int:
	var economy = JSON.parse_string(main.sim.economy_snapshot_json())
	for entry in economy.get("delivery_stock", []):
		if String(entry[0]) == item_id:
			return int(entry[1])
	return 0


func _run_checks() -> void:
	Layouts.build_bbq_branch(main.sim)
	main._rebuild_world()

	var layout: Array = JSON.parse_string(main.sim.world_layout_json())
	if layout.size() != Layouts.BBQ_BRANCH_PLACEMENTS:
		_fail(
			"built %d placements, expected %d — a new game's 3000 must cover both lines, and a \
refused place is silent"
			% [layout.size(), Layouts.BBQ_BRANCH_PLACEMENTS]
		)
		return

	# Both lines are on the floor, and the multi-tile machines cover all of
	# themselves rather than only their origin.
	for pair in [
		[BBQ_MIXER, "mixer"], [BBQ_MIXER_FAR, "mixer"],
		[BBQ_VAT, "cooking_vat"], [BBQ_VAT_FAR, "cooking_vat"],
		[BBQ_BOTTLER, "bottler"], [BBQ_PACKER, "packer"], [BBQ_DOCK, "dock"],
		[Vector2i(9, 0), "mixer"], [Vector2i(12, 0), "cooking_vat"], [Vector2i(26, 0), "dock"],
	]:
		if _placement_kind(pair[0]) != pair[1]:
			_fail("tile %s must carry a %s, not '%s'" % [pair[0], pair[1], _placement_kind(pair[0])])
			return

	# Six runs, all named. An unnamed one is not an error anywhere in the
	# simulation — its machines simply never draw — so it has to be asserted
	# here or it shows up as a factory that looks idle for no stated reason.
	var networks: Array = JSON.parse_string(main.sim.fluid_snapshots_json())
	if networks.size() != 6:
		_fail("expected six pipe runs (vinegar, ketchup mix, ketchup, BBQ mix, liquid smoke, BBQ), got %d" % networks.size())
		return
	for network in networks:
		if String(network.get("fluid", "")) == "":
			_fail("a pipe run resolved to no fluid — its machines will stall in silence")
			return
	for pair in [
		[KETCHUP_TAP, "ketchup"], [BBQ_MIX_RUN, "bbq_mix"],
		[SMOKE_RUN, "liquid_smoke"], [BBQ_RUN, "bbq"],
	]:
		if _fluid_on(pair[0]) != pair[1]:
			_fail("the run at %s must carry %s, not '%s'" % [pair[0], pair[1], _fluid_on(pair[0])])
			return

	# …and all six are DRAWN as six different liquids (issue #68). A fluid
	# with no `FLUID_TINTS` entry does not fail anywhere — it renders in the
	# neutral fallback — so the whole BBQ half of the factory ran pale blue
	# water for two merges, which is the silent-fallback failure `ITEM_MODELS`
	# records about `tomato_paste`. Asserting against the FALLBACK rather
	# than against three named colours keeps the art call where it belongs
	# (main.gd's table, reviewed in a rendered frame) while still catching
	# the next fluid id that ships without one.
	for network in networks:
		var fluid_id := String(network.get("fluid", ""))
		if not main.FLUID_TINTS.has(fluid_id):
			_fail(
				"the run carrying '%s' has no FLUID_TINTS entry — it renders in the neutral \
fallback, which reads as water rather than as sauce"
				% fluid_id
			)
			return
	# Two fluids a few tiles apart that a player cannot tell apart are the
	# same bug one step later, so the six colours are also checked for being
	# six. The threshold is deliberately loose — it refuses a copied-and-
	# forgotten duplicate, not a deliberate shade pair like ketchup_mix and
	# ketchup, which sit 0.31 apart.
	var seen: Dictionary = {}
	for network in networks:
		var fluid_id := String(network.get("fluid", ""))
		var tint: Color = main.FLUID_TINTS[fluid_id]
		for other_id in seen:
			var other: Color = seen[other_id]
			var distance := (
				absf(tint.r - other.r) + absf(tint.g - other.g) + absf(tint.b - other.b)
			)
			if distance < FLUID_TINT_MIN_DISTANCE:
				_fail(
					"'%s' and '%s' are tinted the same liquid (distance %.3f) — two runs on one \
floor have to be tellable apart"
					% [fluid_id, other_id, distance]
				)
				return
		seen[fluid_id] = tint

	# BBQ is gated now (`data/unlocks.ron`): its ingredients and its orders
	# are held back until the ketchup line has demonstrably run. This test
	# is about the branch itself, not about the gate — `sim_core/tests/
	# unlocks.rs` owns that — so it puts the world on the far side of the
	# gate first, the way a player who had made the deliveries would find
	# it. The threshold is read off the wire rather than written out here,
	# so a balance pass on that file cannot leave this asserting against
	# the wrong side of it.
	var econ_gates: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	var threshold := 0
	for unlock in econ_gates.get("unlocks", []):
		threshold = maxi(threshold, int(unlock.get("after_deliveries", 0)))
	if threshold == 0:
		_fail("data/unlocks.ron gates nothing — the BBQ tier should be behind a gate")
		return
	Layouts.deliver(main.sim, threshold)
	main._rebuild_world()

	# The board can ask for two products in one order now — something it
	# could not express before this epic.
	var mixed := 0
	for offer in JSON.parse_string(main.sim.economy_snapshot_json()).get("board", []):
		if offer.get("mix", []).size() > 1:
			mixed += 1
	if mixed == 0:
		_fail("the contract board must offer at least one mixed order")
		return

	# Now run it.
	for item in STOCK:
		Layouts.stock(main.sim, item, int(STOCK[item]))
	for _i in range(RUN_TICKS):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
		main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank", "amount": 100}))
		main.sim.tick()
		if _i % PRODUCT_SAMPLE_TICKS == 0:
			_sample_product_recipes()
	main._rebuild_world()

	for stage in ["bbq_mix", "bbq", "bottled_bbq", "boxed_bbq"]:
		if _made(stage) == 0:
			_fail("the branch never made '%s' in %d ticks" % [stage, RUN_TICKS])
			return
	# The dock is a pass-through into the shared delivery pool every tick, so
	# this is the box actually leaving the factory — no contract needed, and
	# none signed: signing one costs money this fixture does not have.
	if _delivered("boxed_bbq") == 0:
		_fail("%d boxes of BBQ were made and none reached the dock" % _made("boxed_bbq"))
		return

	# NAMES. The HUD titles an item by title-casing its id, which mangles an
	# acronym — `boxed_bbq` read as "Boxed Bbq" on the board and in the
	# production panel. Content names itself now (`display_name` in
	# `data/recipes.ron`, `display_names_json`), and this asserts the join
	# end to end rather than the file: the ids come off the wire and the
	# labels come out of the scene's own formatter.
	for pair in [
		["bbq_mix", "BBQ Mix"], ["bbq", "BBQ"],
		["bottled_bbq", "Bottled BBQ"], ["boxed_bbq", "Boxed BBQ"],
	]:
		if main._item_display_name(pair[0]) != pair[1]:
			_fail(
				"'%s' is labelled '%s', not '%s' — an acronym title-cases wrong, so this id \
must name itself in data/recipes.ron"
				% [pair[0], main._item_display_name(pair[0]), pair[1]]
			)
			return
	# The other half of the same rule: an id the formatter already gets right
	# must NOT be in the map, or every label has a second copy to keep in step.
	if main._item_display_name("boxed_ketchup") != "Boxed Ketchup":
		_fail(
			"'boxed_ketchup' is labelled '%s' — it title-cases correctly and should never \
have needed naming in data"
			% main._item_display_name("boxed_ketchup")
		)
		return

	# THE PLUMBING IS THE PICKER. Checked here rather than at placement time
	# because a machine's recipe is live state, re-picked every idle tick from
	# what its inputs currently satisfy: a machine that has never run yet
	# still reports its kind's first recipe, so asking before the line has
	# turned over would only be reading the file order back.
	if _placement_recipe(Vector2i(9, 0)) != "ketchup_mix":
		_fail("the ketchup mixer must run ketchup_mix, not '%s'" % _placement_recipe(Vector2i(9, 0)))
		return
	if _placement_recipe(BBQ_MIXER) != "bbq_mix":
		_fail("the BBQ mixer must run bbq_mix, not '%s'" % _placement_recipe(BBQ_MIXER))
		return

	# AND THE PICKER IS WHAT PAINTS THE SAUCE. Six machines of three kinds
	# on this floor show sauce, and until 2026-09-21 all six showed ketchup:
	# the sauce was painted into their voxels, so a bottler filling BBQ
	# stood a row of red bottles and the packer beside it boxed them. The
	# product is the `liquid` mesh now, tinted from the live recipe
	# (main.gd's PRODUCT_TINTS), and this asserts that join end to end.
	#
	# **Against what each machine was SEEN making, not what it is making
	# now.** A machine holds a recipe only while its inputs point at one; a
	# line that has run its stock down holds its kind's first recipe again,
	# which is `bottled_ketchup` for both bottlers — so checking at the end
	# of the run would assert the file order back, exactly as the mixers'
	# note above says. `_sampled_recipes` watches all six while the factory
	# is working instead.
	for pair in PRODUCT_MACHINE_PAIRS:
		var tints := {}
		for entry in pair:
			var tile: Vector2i = entry[0]
			var want := String(entry[1])
			var ran: Dictionary = _product_recipes.get(main._tile_key(tile), {})
			if not ran.has(want):
				_fail(
					"the machine at %s never ran '%s' in %d ticks — it was seen making %s, so the sauce it DRAWS cannot be checked"
					% [tile, want, RUN_TICKS, ", ".join(PackedStringArray(ran.keys()))]
				)
				return
			for recipe in ran:
				if not main.PRODUCT_TINTS.has(recipe):
					_fail(
						"the machine at %s ran '%s', which has no PRODUCT_TINTS entry in main.gd — its product draws in the untinted fallback, so a finished sauce reads as pale and unmade. Every recipe a product-showing machine can run needs a row there; that is the one thing a new sauce has to add to this file."
						% [tile, recipe]
					)
					return
			tints[want] = main.PRODUCT_TINTS[want]
		var colours: Array = tints.values()
		if colours[0] == colours[1]:
			_fail(
				"%s and %s are of one kind making different sauces and drawing the same colour (%s) — this is the defect the product mesh exists to fix"
				% [pair[0][1], pair[1][1], colours[0]]
			)
			return
	# The two rows a VAT uses are the fluid tints its own outlet run carries
	# a tile later: the same cook seen through glass and then through steel,
	# so they are equal by intent rather than by coincidence, and a lift like
	# issue #113's has to move both.
	for fluid_id in ["ketchup", "bbq"]:
		if main.PRODUCT_TINTS[fluid_id] != main.FLUID_TINTS[fluid_id]:
			_fail(
				"the vat draws '%s' as %s while the pipe it fills draws it as %s — one cook, one colour"
				% [fluid_id, main.PRODUCT_TINTS[fluid_id], main.FLUID_TINTS[fluid_id]]
			)
			return
	# And the scene actually put those colours on a mesh. A machine with no
	# `liquid` child is one whose product was never split out — the models
	# are stale, or a generator stopped using `machine_product`.
	for pair in PRODUCT_MACHINE_PAIRS:
		for entry in pair:
			var key: String = main._tile_key(entry[0])
			if main._machine_products.get(key, null) == null:
				_fail(
					"the machine at %s has no `liquid` mesh to paint its product on — run `cargo xtask`, and check `machine_product` still colours it in assets/voxels/generators/assets.rs"
					% entry[0]
				)
				return
			if String(main._machine_product_recipes.get(key, "")) == "":
				_fail("the machine at %s never had its product tinted at all" % entry[0])
				return


	print(
		"BBQ_BRANCH_OK: %d placements built from a new game's balance with the shipped line \
unmoved, six pipe runs all named (the single tap at (14, -1) reading ketchup, the BBQ mixer's \
outlet reading bbq_mix), %d mixed order(s) on the board, two mixers of one kind running \
different recipes chosen by their plumbing alone and drawing \
different sauces from it, every BBQ label spelled 'BBQ' rather than \
title-cased to 'Bbq', and %d boxes of BBQ made and %d delivered in %d ticks"
		% [layout.size(), mixed, _made("boxed_bbq"), _delivered("boxed_bbq"), RUN_TICKS]
	)
	quit(0)
