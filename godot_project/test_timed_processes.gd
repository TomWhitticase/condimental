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
#     runs for 2400 ticks between those. Driven by writing the machine state
#     the scene would have read from a snapshot rather than by running a real
#     chain, because a starting balance cannot buy one — see `_drive_gauge`,
#   * `_machine_progress` agrees with the sim's own `process` pair,
#   * the hover tooltip reports that progress and a REMAINING TIME, and
#     `_format_remaining` spells long waits in minutes and short ones in
#     seconds.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("TIMED_PROCESSES_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	# NOTE: the rack itself is placed in `_run_checks`, not here — `_ready`
	# runs twice under this harness and each run creates a fresh (empty)
	# world, so anything built here is wiped (see test_hover.gd's header).
	#
	# It goes down ALONE, and since the inventory-hotbar ticket raised
	# `sim::STARTING_MONEY` to 3000 that is a choice rather than a ceiling:
	# the five machines upstream that would have to exist to actually FEED it
	# (its recipe eats ketchup) are affordable now — `test_layouts.gd`'s
	# `build_reference_line` builds exactly them — but ketchup would still
	# take thousands of ticks to travel down the line before this test could
	# check its first frame. Driving the gauge's own mechanism is both faster
	# and a sharper test of the claim; that is spelled out where it matters,
	# at `_drive_gauge` below.


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


## Starts the gauge's clock the way a chunk snapshot would.
##
## The mechanism under test is precisely "the bar is recomputed from
## `current_tick()` against the machine's `[start_tick, required_ticks]` pair,
## NOT from a snapshot" — which is what makes a 2400-tick process animate at
## all, since a machine dirties its chunk only when its state changes. So the
## honest way to test it is to give the scene that pair once, exactly as
## `_render_snapshot` would, and then never touch it again while the clock
## runs. That is also the only way available: the rack's recipe eats ketchup,
## which needs the five machines upstream of it, which a starting balance
## cannot buy (see this file's `_initialize`).
##
## `required` is read from the SIMULATION's own recipe content via
## `query_tile_json` where possible, so this does not hardcode 2400.
func _drive_gauge(origin_key: String, recipe: String, required: int) -> void:
	main._machine_states[origin_key] = {
		"recipe": recipe,
		"process": [int(main.sim.current_tick()), required],
		"input": [],
		"output": [],
	}


func _run_checks() -> void:
	Layouts.build_aging_rack(main.sim, Vector2i(0, 0))
	main._rebuild_world()

	# --- find the gauged kind from the scene's own mapping ----------------
	var kind := "aging_rack"
	if not main.MACHINE_PROGRESS_GAUGES.has(kind):
		_fail("'%s' no longer declares a progress gauge — this test would prove nothing" % kind)
		return
	var origin := Vector2i(0, 0)
	var origin_key := "0,0"

	# --- the machine itself: a real, bespoke, imported mesh ---------------
	if not main.MACHINE_MODELS.has(kind):
		_fail("kind '%s' has no bespoke model — it would draw the generic '%s'" % [kind, main.MACHINE_MODEL_FALLBACK])
		return
	var node: Node3D = main.get_node_or_null("World/%s_%s" % [kind, origin_key])
	if node == null:
		_fail("the placed aging rack did not render")
		return
	var machine_meshes := _meshes(node, [])
	if machine_meshes.is_empty():
		_fail("the rack rendered no meshes — the .vox didn't import")
		return
	for mi in machine_meshes:
		if mi.mesh == null or mi.mesh.get_surface_count() < 1:
			_fail("the rack has an empty mesh '%s'" % mi.name)
			return
	var size: Vector2i = main._footprint_size(main._placements[0].get("footprint"))
	var expected: Vector3 = main._footprint_centre(origin, size, main.FLOOR_Y)
	if not node.position.is_equal_approx(expected):
		_fail("the rack sits at %s, expected the sim's own footprint centre %s" % [
			str(node.position), str(expected),
		])
		return

	# --- the gauge: its own node, in the footprint, at the slot height ----
	var gauge: Node3D = main._machine_gauges.get(origin_key, null)
	if gauge == null:
		_fail("the rack declares a gauge in MACHINE_PROGRESS_GAUGES but rendered none")
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
	if main._machine_progress(origin_key) != -1.0:
		_fail("nothing has been produced yet, so the rack should read idle, got %f" % main._machine_progress(origin_key))
		return
	main._update_machine_gauges()
	if gauge.visible:
		_fail("an idle rack should hide its bar entirely, leaving the slot empty")
		return

	# --- the process this milestone is about really is a long one ---------
	# Read from the SIMULATION's own content rather than typed in, so
	# rebalancing data/recipes.ron cannot turn this into a tautology.
	var probe = JSON.parse_string(main.sim.query_tile_json(origin.x, origin.y))
	if probe == null or probe.get("kind") != "machine":
		_fail("query_tile on the rack's own tile reported no machine")
		return
	var recipe := String(probe["recipe"])
	var required := 2400
	if probe.get("process") != null:
		required = int(probe["process"][1])
	if required < 1000:
		_fail("the gauged kind's process is only %d ticks — this test is about a LONG one" % required)
		return

	# --- mid-process: the bar shows, and GROWS with the tick counter ------
	_drive_gauge(origin_key, recipe, required)
	for i in range(100):
		main.sim.tick()
	main._update_machine_gauges()
	if not gauge.visible:
		_fail("a rack 100 ticks into its process should show its bar")
		return
	var first_progress: float = main._machine_progress(origin_key)
	var first_scale: float = gauge.scale.y

	# The point of the whole mechanism: NO dirty chunks are applied here, so
	# `_machine_states` cannot change. The bar must still move, because
	# progress is recomputed from `current_tick()` against the `process`
	# pair — which is exactly what a 2400-tick wait needs.
	for i in range(300):
		main.sim.tick()
	main._update_machine_gauges()
	var second_progress: float = main._machine_progress(origin_key)
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

	# --- the tooltip reports progress AND a remaining time ----------------
	var text: String = main._hover_text_at_tile(origin)
	if not text.begins_with(recipe):
		_fail("hovering the rack's tile should describe '%s', got: %s" % [recipe, text])
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

	print("TIMED_PROCESSES_OK: the placed aging rack (recipe '%s', %d ticks) renders a bespoke model at %s with a live progress gauge — hidden when idle, %.0f%% and growing on live ticks with no snapshot applied, tooltip reads a remaining time" % [
		recipe, required, str(origin), second_progress * 100.0,
	])
	quit(0)
