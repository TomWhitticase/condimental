# Headless proof of ARCHITECTURE.md §9 milestone 7's *presentation* half
# (ticket 2) — the structural claims about the aging rack that a machine can
# check without a framebuffer. Whether the rack actually reads as "a slow
# thing that is waiting" is what scripts/screenshot-scene.sh is for, the same
# division test_machines.gd and test_connections.gd already draw.
#
# Run with:
#   godot --headless --path godot_project --script test_timed_processes.gd
#
# Verifies:
#   * the chain has a stage whose machine kind declares a progress gauge
#     (`MACHINE_PROGRESS_GAUGES`) — found from the SIMULATION's own layout,
#     not from a hardcoded "the fourth stage is the rack", so this test keeps
#     meaning the same thing if data/recipes.ron reorders,
#   * that stage renders a real, non-empty, BESPOKE imported mesh at the tile
#     the simulation reports (test_machines.gd asserts the no-fallback rule
#     across the whole chain; this asserts the rack's own model exists at
#     all, which is what would break first if the .vox failed to import),
#   * its gauge is a separate rendered node sitting inside the machine's
#     footprint at the slot height the generator declares,
#   * an idle rack HIDES the bar (an empty slot, not a zero-height sliver),
#   * a rack mid-process shows the bar, and the bar GROWS as ticks pass while
#     no chunk snapshot arrives at all — which is the actual claim, since a
#     machine only dirties its chunk when its state changes and this process
#     runs for 2400 ticks between those,
#   * `_machine_progress` agrees with the sim's own `process` pair,
#   * the hover tooltip reports that progress and a REMAINING TIME, and
#     `_format_remaining` spells long waits in minutes and short ones in
#     seconds.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("TIMED_PROCESSES_FAIL: %s" % msg)
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


func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


## Drives the real chain — both consumables loaded, and the delivery dock
## clearing the far end by itself — until the gauged stage's machine actually
## has a process running,
## applying dirty chunks as the scene itself does. Returns the tick it
## started on, or -1 if the chain never got that far.
func _run_until_gauged_stage_is_working(recipe: String, limit: int) -> int:
	for i in range(limit):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		main.sim.tick()
		main._apply_dirty_chunks()
		if main._machine_progress(recipe) >= 0.0:
			return int(main.sim.current_tick())
	return -1


func _run_checks() -> void:
	var layout: Array = JSON.parse_string(main.sim.chain_layout_json())
	if layout.is_empty():
		_fail("chain_layout_json returned no stages")
		return

	# --- find the gauged stage from the simulation's own layout -----------
	var recipe := ""
	var kind := ""
	var origin := Vector2i.ZERO
	var size := Vector2i.ONE
	for stage in layout:
		var stage_kind := String(stage.get("machine_kind", ""))
		if not main.MACHINE_PROGRESS_GAUGES.has(stage_kind):
			continue
		recipe = String(stage["recipe"])
		kind = stage_kind
		var pair: Array = stage["machine_tile"]
		origin = Vector2i(int(pair[0]), int(pair[1]))
		size = main._footprint_size(stage.get("machine_footprint"))
		break
	if recipe == "":
		_fail("no stage in the shipped chain uses a kind with a progress gauge — milestone 7's rack is not in the layout")
		return

	# --- the machine itself: a real, bespoke, imported mesh ---------------
	if not main.MACHINE_MODELS.has(kind):
		_fail("kind '%s' has no bespoke model — it would draw the generic '%s'" % [kind, main.MACHINE_MODEL_FALLBACK])
		return
	var node: Node3D = main.get_node_or_null("Chain/machine_%s" % recipe)
	if node == null:
		_fail("no rendered machine for the gauged stage '%s'" % recipe)
		return
	var machine_meshes := _meshes(node, [])
	if machine_meshes.is_empty():
		_fail("machine '%s' (kind '%s') rendered no meshes — the .vox didn't import" % [recipe, kind])
		return
	for mi in machine_meshes:
		if mi.mesh == null or mi.mesh.get_surface_count() < 1:
			_fail("machine '%s' has an empty mesh '%s'" % [recipe, mi.name])
			return
	var expected: Vector3 = main._footprint_centre(origin, size, main.FLOOR_Y)
	if not node.position.is_equal_approx(expected):
		_fail("machine '%s' sits at %s, expected the sim's own footprint centre %s" % [
			recipe, str(node.position), str(expected),
		])
		return

	# --- the gauge: its own node, in the footprint, at the slot height ----
	var gauge: Node3D = main._machine_gauges.get(recipe, null)
	if gauge == null:
		_fail("stage '%s' declares a gauge in MACHINE_PROGRESS_GAUGES but rendered none" % recipe)
		return
	if _meshes(gauge, []).is_empty():
		_fail("the gauge's bar rendered no meshes — '%s' didn't import" % main.MACHINE_PROGRESS_GAUGES[kind])
		return
	var gauge_expected: Vector3 = main._footprint_centre(origin, size, main.MACHINE_GAUGE_FLOOR_Y)
	if not gauge.position.is_equal_approx(gauge_expected):
		_fail("the gauge sits at %s, expected %s (the slot floor its generator declares)" % [
			str(gauge.position), str(gauge_expected),
		])
		return

	# --- idle: an EMPTY SLOT, not a zero-height sliver --------------------
	if main._machine_progress(recipe) != -1.0:
		_fail("nothing has been produced yet, so the rack should read idle, got %f" % main._machine_progress(recipe))
		return
	main._update_machine_gauges()
	if gauge.visible:
		_fail("an idle rack should hide its bar entirely, leaving the slot empty")
		return

	# --- mid-process: the bar shows, and GROWS with the tick counter ------
	# Generous: the rack is the fifth stage, so the chain has to run all the
	# way through a crush, a mix and a 240-tick cook before it ever loads.
	var started := _run_until_gauged_stage_is_working(recipe, 4000)
	if started < 0:
		_fail("the chain never delivered anything to '%s' — its process never started" % recipe)
		return

	# A batch that has only just been loaded is legitimately below the
	# sub-voxel threshold, so let it run a little before the bar is expected
	# to be drawable at all.
	for i in range(100):
		main.sim.tick()
	main._update_machine_gauges()
	if not gauge.visible:
		_fail("a rack 100 ticks into its process should show its bar")
		return
	var first_progress: float = main._machine_progress(recipe)
	var first_scale: float = gauge.scale.y

	# The point of the whole mechanism: NO dirty chunks are applied here, so
	# `_last_machines` cannot change. The bar must still move, because
	# progress is recomputed from `current_tick()` against the `process`
	# pair — which is exactly what a 2400-tick wait needs.
	for i in range(300):
		main.sim.tick()
	main._update_machine_gauges()
	var second_progress: float = main._machine_progress(recipe)
	if second_progress <= first_progress:
		_fail("300 ticks passed and progress did not advance (%f -> %f) — the gauge is not reading the live tick" % [
			first_progress, second_progress,
		])
		return
	if gauge.scale.y <= first_scale:
		_fail("progress advanced (%f -> %f) but the bar did not grow (%f -> %f)" % [
			first_progress, second_progress, first_scale, gauge.scale.y,
		])
		return
	if gauge.scale.y > float(main.MACHINE_GAUGE_HEIGHT_VOXELS) + 0.001:
		_fail("the bar (%f) rose past its slot (%d voxels)" % [
			gauge.scale.y, main.MACHINE_GAUGE_HEIGHT_VOXELS,
		])
		return

	# --- `_machine_progress` agrees with the simulation's own numbers -----
	var probe = JSON.parse_string(main.sim.query_tile_json(origin.x, origin.y))
	if probe == null or probe.get("kind") != "machine":
		_fail("query_tile on the rack's own origin %s reported no machine" % str(origin))
		return
	var process = probe.get("process")
	if process == null:
		_fail("the rack should still be mid-process at this point")
		return
	var required: int = int(process[1])
	var elapsed: int = int(main.sim.current_tick()) - int(process[0])
	var expected_fraction: float = float(elapsed) / float(required)
	if absf(second_progress - expected_fraction) > 0.001:
		_fail("_machine_progress said %f, the sim's own process pair says %f" % [
			second_progress, expected_fraction,
		])
		return
	# ...and this really is the long process the milestone is about, not a
	# craft cycle that happens to be running. Asserted against the OTHER
	# stages rather than against 2400, so rebalancing data/recipes.ron
	# doesn't silently turn this test into a tautology.
	var longest_other := 0
	for stage in layout:
		if String(stage["recipe"]) == recipe:
			continue
		var other = JSON.parse_string(main.sim.query_tile_json(
			int(stage["machine_tile"][0]), int(stage["machine_tile"][1])
		))
		if other == null:
			continue
		var other_process = other.get("process")
		if other_process != null:
			longest_other = maxi(longest_other, int(other_process[1]))
	if longest_other > 0 and required <= longest_other:
		_fail("the gauged stage's process (%d ticks) is no longer than another stage's (%d) — it isn't the slow one" % [
			required, longest_other,
		])
		return

	# --- the tooltip reports progress AND a remaining time ----------------
	var text: String = main._hover_text_at_tile(origin)
	if not text.begins_with(recipe):
		_fail("hovering the rack's tile %s should describe '%s', got: %s" % [str(origin), recipe, text])
		return
	if not text.contains("%") or not text.contains("left"):
		_fail("the rack's tooltip should report both a percentage and a remaining time, got: %s" % text)
		return

	# `_format_remaining` itself: seconds while there are seconds to show,
	# minutes once there is a minute — the whole reason it exists is that a
	# multi-minute wait read as "4800s".
	if main._format_remaining(0) != "0s":
		_fail("_format_remaining(0) should be '0s', got '%s'" % main._format_remaining(0))
		return
	if not main._format_remaining(int(main.TICK_HZ) * 5).begins_with("5s"):
		_fail("_format_remaining(5 seconds' worth of ticks) should read '5s', got '%s'" % main._format_remaining(int(main.TICK_HZ) * 5))
		return
	var long_text: String = main._format_remaining(required)
	if int(required / main.TICK_HZ) >= 60 and not long_text.contains("m "):
		_fail("a %d-tick wait should be spelled in minutes, got '%s'" % [required, long_text])
		return

	print("TIMED_PROCESSES_OK: stage '%s' (kind '%s', %d ticks) renders a bespoke model at %s with a live progress gauge — hidden when idle, %.0f%% and growing on live ticks with no snapshot, tooltip reads '%s'" % [
		recipe, kind, required, str(origin), second_progress * 100.0, main._format_remaining(required - elapsed),
	])
	quit(0)
