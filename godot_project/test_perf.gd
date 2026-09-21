# Headless timing of the presentation layer on the STRESS LINE — a 1500-tile
# serpentine belt fed by hoppers, plus three tank-fed pipe runs
# (test_layouts.gd's `build_stress_line`). The pipes & conveyors overhaul's
# ticket 4 replaced a Node3D-per-tile world with MultiMesh instancing for
# belts, splitters, pipes and carried items, and this is the number that
# says whether it worked: how long a full `_rebuild_world` takes (every
# place/remove/rotate pays it) and how long 120 frames of `_process` take
# with the line running.
#
# This is a scripting-side measurement. Headless Godot has no renderer, so
# what is timed is what the script does per rebuild and per frame — node
# creation, snapshot parsing, transform updates — which is exactly the cost
# instancing removes. GPU cost is reviewed with
# `scripts/screenshot-scene.sh --stress`, not asserted.
#
# The ceilings are GENEROUS on purpose: a CI box is slower than a dev
# machine, and this test exists to catch an order-of-magnitude regression
# (a node per tile coming back), not a ten-percent one. The before/after
# numbers are recorded in docs/DEVLOG.md's ticket-4 entry.
#
# Run: godot --headless --path godot_project --script test_perf.gd
# Prints PERF_OK on success (Godot exits 0 even when a script fails to parse,
# so the marker is what scripts/verify-core-loop.sh checks for).
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## The rebuild ceiling covers the SECOND rebuild, once every mesh is in the
## resource cache — that is the cost a place/remove/rotate pays in play.
const REBUILD_CEILING_MSEC := 1500
## 120 frames at 60 Hz is two seconds of play; the ceiling is what would
## still be an interactive frame rate on a slow box (about 25 ms a frame).
const FRAMES := 120
const FRAMES_CEILING_MSEC := 3000
## Ticks run (with dirty chunks applied, as `_process` does) before timing,
## so the belts near every feed carry product when the frames are measured.
const WARMUP_TICKS := 400

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("PERF_FAIL: %s" % msg)
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


func _msec(from_usec: int) -> float:
	return float(Time.get_ticks_usec() - from_usec) / 1000.0


func _run_checks() -> void:
	# The engine's own `_process` on `main` would tick the simulation and
	# re-render between our measurements; the frames below are driven by
	# hand so the count is exact.
	main.set_process(false)

	var build_start := Time.get_ticks_usec()
	Layouts.build_stress_line(main.sim)
	Layouts.stock_larder(main.sim, 2000)
	var build_msec := _msec(build_start)

	main._read_world_layout()
	if main._placements.size() != Layouts.STRESS_PLACEMENTS:
		_fail("the stress line placed %d of %d things — a purchase or a place was refused (money? a collision?)" % [
			main._placements.size(), Layouts.STRESS_PLACEMENTS,
		])
		return

	# First rebuild: pays for loading every mesh from disk into the resource
	# cache. Reported, not asserted.
	var cold_start := Time.get_ticks_usec()
	main._rebuild_world()
	var cold_msec := _msec(cold_start)

	# Second rebuild: what a place/remove/rotate actually costs in play.
	var warm_start := Time.get_ticks_usec()
	main._rebuild_world()
	var rebuild_msec := _msec(warm_start)

	# `_read_world_layout` ON ITS OWN, the first thing a rebuild does and the
	# subject of issue #3: it re-parses the WHOLE `world_layout_json` (some
	# 336 KB at this scale) on every place, remove and rotate. Reported as a
	# share of the rebuild above, because that share is the number the issue
	# is really about — and because the 40-80 ms it was first quoted at came
	# off a DEBUG `sim_core`, where the wire is written by an unoptimised
	# build. Not asserted: it is a fraction of an already-asserted total.
	var layout_start := Time.get_ticks_usec()
	main._read_world_layout()
	var layout_msec := _msec(layout_start)

	# Run the line so there is product to draw, then measure frames.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
	for i in range(WARMUP_TICKS):
		if i % 50 == 0:
			main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		main.sim.tick()
		main._apply_dirty_chunks()
	var carried := _items_in_flight()
	if carried < 1:
		_fail("after %d ticks nothing is on the belts — the hoppers are not feeding the line" % WARMUP_TICKS)
		return

	var frames_start := Time.get_ticks_usec()
	for _i in range(FRAMES):
		main._process(1.0 / 60.0)
	var frames_msec := _msec(frames_start)

	# Counted AFTER the frames (they tick the simulation), against what the
	# scene draws at that same moment.
	carried = _items_in_flight()
	var drawn := -1
	if main.has_method("_carried_item_count"):
		drawn = main._carried_item_count()

	print("PERF: %d placements (%d belts), built through the wire in %.0f ms" % [
		main._placements.size(), Layouts.STRESS_BELTS, build_msec,
	])
	print("PERF: _rebuild_world cold %.1f ms, warm %.1f ms (ceiling %d ms)" % [
		cold_msec, rebuild_msec, REBUILD_CEILING_MSEC,
	])
	print("PERF: _read_world_layout %.1f ms of that warm rebuild (%.0f%%), %d placements" % [
		layout_msec, 100.0 * layout_msec / maxf(rebuild_msec, 0.001), main._placements.size(),
	])
	print("PERF: %d frames of _process %.1f ms total, %.2f ms/frame (ceiling %d ms) with %d items in flight%s" % [
		FRAMES, frames_msec, frames_msec / float(FRAMES), FRAMES_CEILING_MSEC, carried,
		"" if drawn < 0 else ", %d drawn" % drawn,
	])

	if rebuild_msec > float(REBUILD_CEILING_MSEC):
		_fail("a warm _rebuild_world of the stress line took %.1f ms, over the %d ms ceiling" % [
			rebuild_msec, REBUILD_CEILING_MSEC,
		])
		return
	if frames_msec > float(FRAMES_CEILING_MSEC):
		_fail("%d frames took %.1f ms, over the %d ms ceiling" % [FRAMES, frames_msec, FRAMES_CEILING_MSEC])
		return
	if drawn >= 0 and drawn != carried:
		_fail("the simulation has %d items on conveyors and the scene draws %d" % [carried, drawn])
		return

	print("PERF_OK: stress line rebuilt in %.1f ms and %d running frames in %.1f ms" % [
		rebuild_msec, FRAMES, frames_msec,
	])
	quit(0)


## How many items the SIMULATION says are on belts and splitters right now,
## summed over every chunk snapshot — the figure the scene's drawn count is
## checked against.
func _items_in_flight() -> int:
	var total := 0
	var seen := {}
	for placement in main._placements:
		var kind := String(placement.get("kind", ""))
		if kind != "belt" and kind != "splitter":
			continue
		var tile: Array = placement["tile"]
		var key := "%d,%d" % [int(tile[0]), int(tile[1])]
		if seen.has(key):
			continue
		seen[key] = true
		var parsed = JSON.parse_string(main.sim.query_tile_json(int(tile[0]), int(tile[1])))
		if parsed != null:
			total += int(parsed.get("items_here", []).size())
	return total
