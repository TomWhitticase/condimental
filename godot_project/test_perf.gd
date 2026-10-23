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
# (a node per tile coming back), not a ten-percent one. They are also
# SCALED to how loaded the machine is while the test runs — see
# `_calibration_msec` and issue #146. The before/after numbers are recorded
# in docs/DEVLOG.md's ticket-4 entry.
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

## Wall clock is a shaky yardstick here. This repo is worked in several
## worktrees at once, and a neighbouring `godot --headless` run stretched the
## very same 120 frames from 350.7 ms to 3733.1 ms with no code between the
## two measurements — a red gate that reads exactly like a regression and
## sends the session that hit it looking for one in its own diff (issue
## #146). So both ceilings above are multiplied by how slowly THIS process is
## running right now, measured against a loop of fixed, pure-script
## arithmetic: 400k iterations, about 12 ms on an unloaded dev machine.
##
## The factor is clamped at 1.0 from below, so on a quiet box the ceilings
## are exactly the constants above and nothing is loosened. It is clamped
## from above too: past a point the machine is too busy for the number to
## mean anything, and a ceiling that rises without limit stops being a test.
##
## The reference is a dev-machine number, so a permanently slower box — a CI
## runner — runs with permanently scaled ceilings. That is the same allowance
## the paragraph above already makes in prose, made by measurement instead of
## by guessing how much slower CI might be.
const CALIBRATION_ITERATIONS := 400000
const CALIBRATION_SAMPLES := 9
const CALIBRATION_REFERENCE_MSEC := 12.0
const CALIBRATION_MAX_FACTOR := 6.0

var main
var _ran := false
## Where the calibration loop's result is parked. Reading it back is what
## stops the loop being dead code the engine is free to skip.
var _calibration_sink := 0


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


## How long a fixed amount of pure-script arithmetic takes right now — the
## MEDIAN of `CALIBRATION_SAMPLES` runs of it.
##
## The median, not the minimum: a process fighting for the CPU still gets a
## full slice now and then, so the fastest sample looks idle however loaded
## the box is. Measured under sixteen spinning processes, the fastest sample
## was 11.9 ms — indistinguishable from an idle machine — against a median of
## 38.3 ms. Nor the mean, which one descheduled sample (113.5 ms in that same
## run) drags around on its own. The median is what a long stretch of work,
## which is what the frame loop is, actually runs at.
func _calibration_msec() -> float:
	var samples: Array[float] = []
	for _sample in range(CALIBRATION_SAMPLES):
		var started := Time.get_ticks_usec()
		var acc := 0
		for i in CALIBRATION_ITERATIONS:
			acc = (acc + i * 3) % 1000003
		_calibration_sink = acc
		samples.append(_msec(started))
	samples.sort()
	return samples[samples.size() / 2]


func _run_checks() -> void:
	# The engine's own `_process` on `main` would tick the simulation and
	# re-render between our measurements; the frames below are driven by
	# hand so the count is exact.
	main.set_process(false)

	# Before anything is timed, and again after the frames: contention that
	# arrives halfway through a run is as real as contention that was there
	# from the start, and the worse of the two readings is the one that
	# describes the measurements it brackets.
	var calibration := _calibration_msec()

	var build_start := Time.get_ticks_usec()
	Layouts.build_stress_line(main.sim)
	# One larder rather than the 2 000 of each raw this used to buy: a
	# holding is capped now, and 2 000 is forty bays per ingredient — twenty
	# racks, which would be twenty more buildings in a measurement about
	# belts. The warmup loop below tops it up instead.
	Layouts.stock_larder(main.sim, Layouts.LARDER_STOCK)
	var build_msec := _msec(build_start)

	main._read_world_layout()
	# The fixture's own warehouse is not part of the line being measured, so
	# it is not part of the count either — `Layouts.line_placements` says why.
	# Counted off what `_read_world_layout` parsed rather than off the sim,
	# because what this guards is that the read crossed the wire intact.
	var placed := 0
	for placement in main._placements:
		if String(placement.get("kind", "")) != Layouts.WAREHOUSE_KIND:
			placed += 1
	if placed != Layouts.STRESS_PLACEMENTS:
		_fail("the stress line placed %d of %d things — a purchase or a place was refused (money? a collision?)" % [
			placed, Layouts.STRESS_PLACEMENTS,
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
	# subject of issue #3. It used to re-parse the WHOLE `world_layout_json`
	# (some 350 KB at this scale) on every place, remove and rotate; it now
	# parses only the entries whose text changed. So it is timed the way play
	# pays for it — straight after a rotate, one belt and its neighbours'
	# links changed — not as a re-read of an unchanged world, which would
	# time nothing but cache hits. Reported as a share of the rebuild above,
	# because that share is the number the issue is really about. Not
	# asserted: it is a fraction of an already-asserted total.
	var turned: Array = main._placements[floori(main._placements.size() / 2.0)]["tile"]
	var rotate := JSON.stringify({"type": "rotate", "tile": turned})
	main.sim.issue_command_json(rotate)
	var layout_start := Time.get_ticks_usec()
	main._read_world_layout()
	var layout_msec := _msec(layout_start)
	# Three more quarter turns put the belt back as the line needs it.
	for _turn in range(3):
		main.sim.issue_command_json(rotate)
	main._read_world_layout()

	# Run the line so there is product to draw, then measure frames.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
	for i in range(WARMUP_TICKS):
		if i % 50 == 0:
			Layouts.restock(main.sim)
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

	calibration = maxf(calibration, _calibration_msec())
	var load_factor := clampf(
		calibration / CALIBRATION_REFERENCE_MSEC, 1.0, CALIBRATION_MAX_FACTOR
	)
	var rebuild_ceiling := float(REBUILD_CEILING_MSEC) * load_factor
	var frames_ceiling := float(FRAMES_CEILING_MSEC) * load_factor

	# Counted AFTER the frames (they tick the simulation), against what the
	# scene draws at that same moment.
	carried = _items_in_flight()
	var drawn := -1
	if main.has_method("_carried_item_count"):
		drawn = main._carried_item_count()

	print("PERF: %d placements (%d belts), built through the wire in %.0f ms" % [
		main._placements.size(), Layouts.STRESS_BELTS, build_msec,
	])
	print("PERF: _rebuild_world cold %.1f ms, warm %.1f ms (ceiling %.0f ms)" % [
		cold_msec, rebuild_msec, rebuild_ceiling,
	])
	print("PERF: _read_world_layout %.1f ms of that warm rebuild (%.0f%%), %d placements" % [
		layout_msec, 100.0 * layout_msec / maxf(rebuild_msec, 0.001), main._placements.size(),
	])
	print("PERF: %d frames of _process %.1f ms total, %.2f ms/frame (ceiling %.0f ms) with %d items in flight%s" % [
		FRAMES, frames_msec, frames_msec / float(FRAMES), frames_ceiling, carried,
		"" if drawn < 0 else ", %d drawn" % drawn,
	])
	# The raw milliseconds above are the comparable numbers (docs/DEVLOG.md's
	# ticket-4 entry quotes them); this line says what the ceilings they were
	# judged against actually were on this run.
	print("PERF: machine load %.2fx — %d k-iteration calibration %.1f ms against a %.1f ms reference%s" % [
		load_factor, CALIBRATION_ITERATIONS / 1000, calibration, CALIBRATION_REFERENCE_MSEC,
		"" if load_factor < CALIBRATION_MAX_FACTOR else " (clamped at %.1fx)" % CALIBRATION_MAX_FACTOR,
	])

	if rebuild_msec > rebuild_ceiling:
		_fail("a warm _rebuild_world of the stress line took %.1f ms, over the %.0f ms ceiling (%d ms x %.2f machine load)" % [
			rebuild_msec, rebuild_ceiling, REBUILD_CEILING_MSEC, load_factor,
		])
		return
	if frames_msec > frames_ceiling:
		_fail("%d frames took %.1f ms, over the %.0f ms ceiling (%d ms x %.2f machine load)" % [
			FRAMES, frames_msec, frames_ceiling, FRAMES_CEILING_MSEC, load_factor,
		])
		return
	if drawn >= 0 and drawn != carried:
		_fail("the simulation has %d items on conveyors and the scene draws %d" % [carried, drawn])
		return

	print("PERF_OK: stress line rebuilt in %.1f ms and %d running frames in %.1f ms, at %.2fx machine load" % [
		rebuild_msec, FRAMES, frames_msec, load_factor,
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
