# Headless proof of the machine animations: every processing machine carries
# a motion overlay whose frames advance while the machine is WORKING and only
# while it is working.
#
# Run with:
#   godot --headless --path godot_project --script test_machine_motion.gd
#
# Verifies:
#   * every machine of the reference line gets `MACHINE_FRAMES` overlay nodes
#     at its body's own position and facing, with frame 0 showing and the
#     rest hidden, drawn from the family its kind names (and from
#     `MACHINE_MOTION_FALLBACK` for a kind that names none),
#   * nothing that is not a recipe-running machine gets one — not the
#     hopper, the tank, the dock, the splitter, a belt or a pipe,
#   * `_motion_frame_for_elapsed` walks all `MACHINE_FRAMES` frames in order
#     across one `MACHINE_TICKS_PER_CYCLE` cycle and starts the next cycle
#     back at frame 0,
#   * `_machine_work_elapsed` counts ticks of WORK: a machine with no
#     process is idle, and so is one whose batch is finished but stuck on a
#     full output (`elapsed >= required`), which is the case that separates
#     "is turning" from the status lamp's "has a batch",
#   * a running factory really does move its mechanisms through
#     `_advance_simulation`, stops them dead with the pause menu up, and puts
#     them back to frame 0 when the work stops.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("MACHINE_MOTION_FAIL: %s" % msg)
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


## One simulation step through the real per-frame path, so what is being
## proved is that `_advance_simulation` drives the mechanisms — not that a
## test can call `_update_machine_motion` itself.
func _step() -> void:
	main._advance_simulation(1.0 / float(main.TICK_HZ))


func _frame_of(key: String) -> int:
	return int(main._machine_motion[key]["frame"])


## One whole frame: a simulation step and then the feedback pass `_process`
## runs after it. The crusher's spray rides that pass, not `_advance_simulation`,
## so a test that only stepped the simulation would prove nothing about it.
func _frame() -> void:
	_step()
	main._update_payout_feedback()


func _run_checks() -> void:
	# --- 1. the frame arithmetic, pure -------------------------------------
	if main._motion_frame_for_elapsed(-1) != 0:
		_fail("an idle machine (-1 elapsed) must show frame 0")
	var seen := {}
	var previous := -1
	for elapsed in range(main.MACHINE_TICKS_PER_CYCLE):
		var frame: int = main._motion_frame_for_elapsed(elapsed)
		if frame < previous:
			_fail("frame went backwards inside one cycle at elapsed %d: %d after %d" % [
				elapsed, frame, previous,
			])
		previous = frame
		seen[frame] = true
	if seen.size() != main.MACHINE_FRAMES:
		_fail("one cycle must show all %d frames, showed %d" % [main.MACHINE_FRAMES, seen.size()])
	if main._motion_frame_for_elapsed(0) != 0:
		_fail("work starts on frame 0")
	if main._motion_frame_for_elapsed(main.MACHINE_TICKS_PER_CYCLE) != 0:
		_fail("the cycle must close: the tick after a full cycle is frame 0 again")
	if main._motion_frame_for_elapsed(main.MACHINE_TICKS_PER_CYCLE - 1) != main.MACHINE_FRAMES - 1:
		_fail("the last tick of a cycle must be the last frame")

	# --- 2. what counts as working ------------------------------------------
	var probe := "77,77"
	var now: int = main.sim.current_tick()
	if main._machine_work_elapsed(probe) != -1:
		_fail("a machine this scene has never seen is idle")
	main._machine_states[probe] = {"process": null}
	if main._machine_work_elapsed(probe) != -1:
		_fail("a machine with no process is idle")
	main._machine_states[probe] = {"process": [now - 5, 80]}
	if main._machine_work_elapsed(probe) != 5:
		_fail("a machine 5 ticks into an 80-tick batch has done 5 ticks of work, got %d" % [
			main._machine_work_elapsed(probe),
		])
	# The one that matters. `Machine::tick` clears `process` only once the
	# goods have somewhere to go, so a finished batch behind a full output
	# buffer keeps its process and retries every tick. The lamp says "has a
	# batch"; the mechanism must say "not turning".
	main._machine_states[probe] = {"process": [now - 80, 80]}
	if main._machine_work_elapsed(probe) != -1:
		_fail("a finished-but-blocked machine must not be animated: its work is done")
	main._machine_states[probe] = {"process": [now - 400, 80]}
	if main._machine_work_elapsed(probe) != -1:
		_fail("a long-blocked machine must not be animated either")
	main._machine_states.erase(probe)

	# --- 3. the overlay nodes on a real line --------------------------------
	Layouts.build_reference_line(main.sim)
	main._rebuild_world()

	var expected := {
		"4,0": "machine_crusher_motion",
		"9,0": "machine_mixer_motion",
		"12,0": "machine_cooking_vat_motion",
		"16,0": "machine_bottler_motion",
		"21,0": "machine_packer_motion",
	}
	for key in expected.keys():
		if not main._machine_motion.has(key):
			_fail("the machine at %s has no motion overlay" % key)
		var motion: Dictionary = main._machine_motion[key]
		if String(motion.get("family", "")) != expected[key]:
			_fail("the machine at %s draws motion family %s, expected %s" % [
				key, str(motion.get("family")), expected[key],
			])
		var frames: Array = motion["frames"]
		if frames.size() != main.MACHINE_FRAMES:
			_fail("the machine at %s has %d motion frames, expected %d" % [
				key, frames.size(), main.MACHINE_FRAMES,
			])
		for k in range(frames.size()):
			var node: Node3D = frames[k]
			if node == null or not is_instance_valid(node):
				_fail("the machine at %s is missing motion frame %d" % [key, k])
			if node.visible != (k == 0):
				_fail("a machine at rest shows frame 0 and nothing else; %s frame %d visible=%s" % [
					key, k, str(node.visible),
				])
		# The overlay stands exactly where the body does. Both models are cut
		# on the machine's own footprint with their origins at footprint
		# centre, ground level, so any offset here would be undoing that.
		var body: Node3D = null
		for child in main._world_root.get_children():
			if child is Node3D and String(child.name).ends_with("_%s" % key) \
					and not String(child.name).begins_with("motion_"):
				body = child
				break
		if body == null:
			_fail("no body node found for the machine at %s" % key)
		for k in range(frames.size()):
			var node: Node3D = frames[k]
			if node.position.distance_to(body.position) > 0.001:
				_fail("motion frame %d of %s stands at %s, its body at %s" % [
					k, key, str(node.position), str(body.position),
				])
			if absf(node.rotation.y - body.rotation.y) > 0.001:
				_fail("motion frame %d of %s faces %f, its body %f" % [
					k, key, node.rotation.y, body.rotation.y,
				])

	# Everything that is not a recipe-running machine: no overlay at all. The
	# dock and the splitter are in MACHINE_MODELS (they have footprints) but
	# they are not machines, and a spinning drive spider on a loading dock
	# would say the factory was doing something it is not.
	for key in ["-1,0", "5,2", "26,0", "0,0", "10,0"]:
		if main._machine_motion.has(key):
			_fail("%s is not a processing machine and must have no motion overlay" % key)

	# The fallback, which is the reason this table is allowed to be sparse: a
	# kind added to `data/machines.ron` has to draw AND visibly run before
	# anybody edits this file (UI_UX.md §2b point 12's rule, applied to
	# motion).
	main._build_machine_motion("77,77", "a_kind_nobody_has_modelled", Vector3.ZERO, 0.0)
	if String(main._machine_motion["77,77"].get("family", "")) != main.MACHINE_MOTION_FALLBACK:
		_fail("an unmodelled kind must fall through to %s, got %s" % [
			main.MACHINE_MOTION_FALLBACK, str(main._machine_motion["77,77"].get("family")),
		])
	main._machine_motion.erase("77,77")

	# --- 4. a running factory moves; a paused one does not -------------------
	Layouts.stock_larder(main.sim)
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	var crusher := "4,0"
	var working := false
	for i in range(600):
		_step()
		if main._machine_work_elapsed(crusher) >= 0:
			working = true
			break
	if not working:
		_fail("the reference line's crusher never started a batch in 600 ticks")

	var frames_seen := {}
	for i in range(main.MACHINE_TICKS_PER_CYCLE):
		frames_seen[_frame_of(crusher)] = true
		_step()
	if frames_seen.size() < 2:
		_fail("a working crusher's mechanism never changed frame over a whole cycle")
	# What is shown is what the bookkeeping says is shown.
	var shown: Array = main._machine_motion[crusher]["frames"]
	for k in range(shown.size()):
		if shown[k].visible != (k == _frame_of(crusher)):
			_fail("the crusher shows frame %d but node %d visible=%s" % [
				_frame_of(crusher), k, str(shown[k].visible),
			])

	var before := _frame_of(crusher)
	var tick_before: int = main.sim.current_tick()
	main._pause_open = true
	for i in range(main.MACHINE_TICKS_PER_CYCLE):
		_step()
	main._pause_open = false
	if main.sim.current_tick() != tick_before:
		_fail("the pause menu must stop the simulation")
	if _frame_of(crusher) != before:
		_fail("the mechanisms must freeze with the pause menu up: %d became %d" % [
			before, _frame_of(crusher),
		])

	# --- 4b. a working crusher throws pulp, on its own beat ------------------
	# The spray is the animation's louder half (the crusher-particles change,
	# 2026-09-16) and it is driven by the same `_machine_work_elapsed` the
	# frames are, so it inherits the same meaning of "working" — which is the
	# thing worth proving, since a machine holding a finished batch still has
	# a lamp on and still has a process.
	if not main._crush_anchors.has(crusher):
		_fail("the placed crusher recorded no anchor to throw pulp from")
	var anchor_point: Vector3 = main._crush_anchors[crusher]
	if not is_equal_approx(anchor_point.y, main.CRUSH_MOUTH_Y):
		_fail("pulp should leave from the feed pocket's mouth (%f), not %f" % [
			main.CRUSH_MOUTH_Y, anchor_point.y,
		])
	for other in ["-1,0", "5,0", "6,0"]:
		if main._crush_anchors.has(other):
			_fail("only a crusher throws pulp; '%s' has an anchor too" % other)

	var beats_before: int = main.crush_beats_thrown
	var seen_in_air := 0
	for i in range(main.CRUSH_BEAT_TICKS * 3):
		_frame()
		seen_in_air = maxi(seen_in_air, main._active_crush_count())
	if main.crush_beats_thrown <= beats_before:
		_fail("a working crusher threw no pulp over three whole beats")
	if seen_in_air <= 0:
		_fail("pulp was thrown but nothing was ever in the air")
	if seen_in_air > main.CRUSH_POOL:
		_fail("more flecks in the air (%d) than the pool holds (%d)" % [seen_in_air, main.CRUSH_POOL])

	# Paused, the pulp hangs where it is: the flight is clocked off the
	# simulation tick, never wall time (UI_UX.md §6a).
	var beats_at_pause: int = main.crush_beats_thrown
	var in_air_at_pause: int = main._active_crush_count()
	main._pause_open = true
	for i in range(main.CRUSH_BEAT_TICKS * 2):
		_frame()
	main._pause_open = false
	if main.crush_beats_thrown != beats_at_pause:
		_fail("the pause menu must stop the spray: %d beats became %d" % [
			beats_at_pause, main.crush_beats_thrown,
		])
	if main._active_crush_count() != in_air_at_pause:
		_fail("pulp in the air must freeze with the game, %d became %d" % [
			in_air_at_pause, main._active_crush_count(),
		])

	# --- 5. work stops, the mechanism stops ---------------------------------
	main._machine_states[crusher] = {"process": null}
	main._update_machine_motion()
	if _frame_of(crusher) != 0:
		_fail("an idle machine must come back to rest on frame 0, showing %d" % _frame_of(crusher))
	var resting: Array = main._machine_motion[crusher]["frames"]
	for k in range(resting.size()):
		if resting[k].visible != (k == 0):
			_fail("at rest only frame 0 is drawn; node %d visible=%s" % [k, str(resting[k].visible)])
	# ...and throws nothing. A crusher holding a finished batch it cannot put
	# down has a process and a lit lamp, and is exactly the case that must
	# not keep spraying (UI_UX.md §2b point 28).
	# Driven directly rather than through `_frame()`: a real step re-polls the
	# machine states from the simulation, which would put the forced-idle
	# crusher straight back to work. The tick is handed in for the same
	# reason a real frame hands it in — the flight is a function of it.
	var idle_beats: int = main.crush_beats_thrown
	var idle_tick: int = main.sim.current_tick()
	for i in range(main.CRUSH_BEAT_TICKS * 3 + main.CRUSH_FLIGHT_TICKS + 1):
		main._update_crush_spray(idle_tick + i)
	if main.crush_beats_thrown != idle_beats:
		_fail("a machine that is not working must throw no pulp")
	if main._active_crush_count() != 0:
		_fail("every fleck must be reclaimed once its flight is over, %d still out" % main._active_crush_count())

	print("MACHINE_MOTION_OK: mechanisms turn only while working, and a working crusher throws pulp on its own beat")
	quit(0)
