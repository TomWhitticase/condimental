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
#   * A MACHINE PICKS ITS RECIPE FROM ITS PLUMBING: the two mixers are the
#     same kind on the same board and one makes `ketchup_mix` while the other
#     makes `bbq_mix`, with no picker anywhere in the UI,
#   * A BOX OF BBQ IS MADE AND SHIPPED: `boxed_bbq` appears in the production
#     panel's own snapshot and in the dock's delivery pool,
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

var main: Node
var _ran := false


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

	print(
		"BBQ_BRANCH_OK: %d placements built from a new game's balance with the shipped line \
unmoved, six pipe runs all named (the single tap at (14, -1) reading ketchup, the BBQ mixer's \
outlet reading bbq_mix), %d mixed order(s) on the board, two mixers of one kind running \
different recipes chosen by their plumbing alone, and %d boxes of BBQ made and %d delivered \
in %d ticks"
		% [layout.size(), mixed, _made("boxed_bbq"), _delivered("boxed_bbq"), RUN_TICKS]
	)
	quit(0)
