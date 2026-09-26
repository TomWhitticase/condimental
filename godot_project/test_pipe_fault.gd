# Headless proof of the PRESENTATION half of issue #54: a pipe run that
# carries nothing readable says so, in the scene and in the tooltip.
#
# Run with:
#   godot --headless --path godot_project --script test_pipe_fault.gd
#
# WHY THIS EXISTS BESIDE `sim_core/tests/pipe_naming.rs`. That test proves
# the simulation decides — four namings, two of them faults, and a stall
# reason that names the run instead of claiming there is no pipe. It cannot
# say whether any of that reaches the player, and the bug in #54 was
# precisely that it did not: the run was already unnamed, the vat was already
# stalled, and the game drew the same empty pipe it draws for a run that is
# working and waiting for its first fill.
#
# Verifies, against `test_layouts.gd`'s `build_stalled_pipe_run` (the BBQ
# branch with the liquid-smoke tank turned off its own run — #54's own
# mistake, not a synthetic one):
#   * the simulation calls that run `ambiguous` and its neighbours `carries`,
#     so everything below is read against a real fault,
#   * a hazard clamp stands on EVERY tile the simulation calls dead — the
#     smoke run and the vat's outlet run it takes down with it — and on NO
#     tile of the empty run beside them, each rendering a real imported mesh,
#   * the clamps come DOWN again when the player turns the tank back, which
#     is the fix the tooltip tells them to make,
#   * hovering the dead run says DEAD RUN and names every fluid it could
#     carry; hovering an equally empty run beside it says neither,
#   * the vat on the dead run reports the run as its reason rather than "no
#     supply pipe" about a pipe the player has laid,
#   * `_pipe_fault_text` is driven by the wire's `naming` alone — an unnamed
#     run that is merely `unplumbed` raises nothing, which is what every run
#     looks like while it is still being laid.
#
# The one-frame-before-checking dance is inherited from test_hover.gd — see
# that file's header for the `--script` SceneTree bootstrap quirk it works
# around.

extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("PIPE_FAULT_FAIL: %s" % msg)
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


## Every MeshInstance3D under `node`, so "the asset actually imported" can be
## asserted rather than assumed (a missing .vox yields an empty Node3D).
func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


## The run covering `tile`, found by its segments rather than by index: the
## topology renumbers networks on every placement.
func _run_at(tile: Vector2i) -> Dictionary:
	for network in main._last_fluid:
		for segment in network.get("segments", []):
			var pair: Array = segment["tile"]
			if Vector2i(int(pair[0]), int(pair[1])) == tile:
				return network
	return {}


func _run_checks() -> void:
	# Built HERE, not in `_initialize`: `_ready` runs twice under this harness
	# and each run creates a fresh (empty) world — see test_hover.gd's header.
	Layouts.build_stalled_pipe_run(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()
	main._update_fluid_network()

	# --- the fault is the simulation's verdict, not this test's -----------
	var dead := _run_at(Layouts.STALLED_RUN_TILE)
	if dead.is_empty():
		_fail("no run covers %s" % str(Layouts.STALLED_RUN_TILE))
		return
	if String(dead.get("naming", "")) != "ambiguous":
		_fail("the tank was turned off its run, so the run should be ambiguous, not '%s'" % \
			str(dead.get("naming")))
		return
	if Array(dead.get("candidates", [])) != Layouts.STALLED_RUN_CANDIDATES:
		_fail("the dead run should report what it narrowed to, got %s" % str(dead.get("candidates")))
		return
	var alive := _run_at(Layouts.STALLED_NEIGHBOUR_TILE)
	if String(alive.get("naming", "")) != "carries":
		_fail("the run beside it is named and merely empty; got '%s'" % str(alive.get("naming")))
		return
	if float(alive.get("total_volume", -1.0)) != 0.0:
		_fail("the comparison only means something while the neighbour is EMPTY, got %s units" % \
			str(alive.get("total_volume")))
		return

	# --- a clamp on every tile of every dead run, and on nothing else -----
	# Every dead run, not only the one the tank pinned: with the smoke run
	# unnameable the vat can no longer be told which sauce it cooks, so its
	# OUTLET run narrows to several as well. That knock-on is the simulation's
	# own answer and the marking follows it exactly — which is the whole
	# point of reading `naming` off the wire instead of guessing here.
	var dead_tiles: Array = []
	for network in main._last_fluid:
		if not main.PIPE_FAULT_NAMINGS.has(String(network.get("naming", ""))):
			continue
		for segment in network.get("segments", []):
			var pair: Array = segment["tile"]
			dead_tiles.append(main._tile_key(Vector2i(int(pair[0]), int(pair[1]))))
	for key in dead_tiles:
		if not main._pipe_fault_nodes.has(key):
			_fail("no fault clamp on %s, a tile of a run the sim cannot name" % key)
			return
		if _meshes(main._pipe_fault_nodes[key], []).is_empty():
			_fail("the fault clamp on %s rendered no meshes — '%s' didn't import" % \
				[key, main.PIPE_FAULT_ASSET])
			return
	if main._pipe_fault_nodes.size() != dead_tiles.size():
		_fail("%d clamps stand for %d dead tiles — a run the sim DID name is being marked" % \
			[main._pipe_fault_nodes.size(), dead_tiles.size()])
		return
	if main._pipe_fault_nodes.has(main._tile_key(Layouts.STALLED_NEIGHBOUR_TILE)):
		_fail("a merely empty run is flagged; empty and dead are the two states this exists to separate")
		return

	# --- hover: the reason, and which of the two faults it is -------------
	var dead_text: String = main._hover_text_at_tile(Layouts.STALLED_RUN_TILE)
	if not dead_text.contains("DEAD RUN"):
		_fail("the dead run's tooltip should say so, got: %s" % dead_text)
		return
	for candidate in Layouts.STALLED_RUN_CANDIDATES:
		var name: String = main._item_display_name(candidate)
		if not dead_text.contains(name):
			_fail("the tooltip should name %s, one of the fluids confused, got: %s" % [name, dead_text])
			return
	var alive_text: String = main._hover_text_at_tile(Layouts.STALLED_NEIGHBOUR_TILE)
	if alive_text.contains("DEAD RUN"):
		_fail("an empty but named run is not a fault, got: %s" % alive_text)
		return
	if not alive_text.contains("units"):
		_fail("the empty run should still report its level, got: %s" % alive_text)
		return

	# --- the machine on the dead run blames the run ------------------------
	main._poll_machine_load()
	var vat_text: String = main._hover_text_at_tile(Vector2i(17, -3))
	if not vat_text.contains("more than one fluid"):
		_fail("the vat on the dead run should blame the run, got: %s" % vat_text)
		return
	if vat_text.contains("no supply pipe"):
		_fail("the vat's supply pipe is laid; claiming it is missing is the wrong answer #54 is about")
		return

	# --- `unplumbed` is not a fault ---------------------------------------
	# Loose pipe joined to nothing narrows to nothing too, and marking it
	# would put a hazard clamp on every tile a player lays.
	if main._pipe_fault_text({"naming": "unplumbed", "candidates": []}) != "":
		_fail("a run still being laid must raise nothing")
		return
	if main._pipe_fault_text({"naming": "unnameable", "candidates": []}) == "":
		_fail("a run nothing can fill must say so even with no candidates to name")
		return

	# --- and the clamps come down when the mistake is undone ---------------
	# Three more quarter turns is the player's own fix: the tank faces its
	# run again, the run is pinned to liquid smoke, and nothing about it is
	# marked any more.
	for _turn in range(3):
		main._issue_world_command({"type": "rotate", "tile": [Layouts.STALLED_TANK.x, Layouts.STALLED_TANK.y]})
	main._update_fluid_network()
	var fixed := _run_at(Layouts.STALLED_RUN_TILE)
	if String(fixed.get("naming", "")) != "carries":
		_fail("turning the tank back should name the run again, got '%s'" % str(fixed.get("naming")))
		return
	if not main._pipe_fault_nodes.is_empty():
		_fail("%d clamps are still standing on a run that is fixed" % main._pipe_fault_nodes.size())
		return

	print("PIPE_FAULT_OK: %d dead tiles flagged and no named run marked, the tooltip names every fluid the run could carry, the vat blames the run, and every clamp comes down when the tank is turned back" % dead_tiles.size())
	quit(0)
