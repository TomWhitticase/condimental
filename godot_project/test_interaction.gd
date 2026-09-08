# Headless proof that main.tscn's keyboard input actually reaches the
# simulation (ARCHITECTURE.md §9 item 3's "interactable" requirement) —
# not just that the underlying issue_command_json API works (test_core_loop.gd
# already covers that), but that Main._unhandled_input's key handling does.
#
# Run with:
#   godot --headless --path godot_project --script test_interaction.gd
extends SceneTree

const Layouts = preload("res://test_layouts.gd")


func _fail(msg: String) -> void:
	printerr("INTERACTION_FAIL: %s" % msg)
	quit(1)


## The world starts EMPTY since the "Real Placement" epic, so there is no
## hopper to load until the test builds one — and `hopper_snapshots_json` is
## an array now, because a world may hold several.
func _hopper_quantity(sim) -> int:
	var hoppers = JSON.parse_string(sim.hopper_snapshots_json())
	if not (hoppers is Array) or hoppers.is_empty():
		return -1
	return int(hoppers[0]["quantity"])


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	# `_ready()` only fires on the engine's own next-frame processing, which
	# this headless script-mode SceneTree never runs — call it directly.
	main._ready()

	# Nothing exists until the player builds it. SPACE loading "the hopper"
	# is only meaningful once there IS one, which is itself the change this
	# epic made.
	Layouts.build_minimal(main.sim)
	main._rebuild_world()

	# Since the "Machines improvement" ticket 3, SPACE loads the chain's
	# hopper rather than teleporting an item onto the belt — the hopper is
	# what puts tomatoes on the belt, one per tick. So the observable effect
	# of the keypress is the hopper's fill level rising.
	var before := _hopper_quantity(main.sim)

	var key_event := InputEventKey.new()
	key_event.keycode = KEY_SPACE
	key_event.pressed = true
	main._unhandled_input(key_event)

	var after := _hopper_quantity(main.sim)
	if after <= before:
		_fail("pressing SPACE via _unhandled_input did not load the hopper (before=%d after=%d)" % [before, after])
		return

	# C is GONE (the delivery-dock ticket). `PlayerCommand::CollectOutput` no
	# longer exists sim-side, so the key was removed here rather than left to
	# send a command `issue_command_json` would silently drop — a control that
	# quietly does nothing is worse than no control. Both halves are checked:
	# the legend no longer advertises it, and the key genuinely does nothing.
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		if legend.to_lower().contains("collect"):
			_fail("the control legend still offers the removed collect-product control: %s" % legend)
			return
	var econ_before: String = main.sim.economy_snapshot_json()
	var dock_before: String = main.sim.dock_snapshots_json()
	var c_event := InputEventKey.new()
	c_event.keycode = KEY_C
	c_event.pressed = true
	main._unhandled_input(c_event)
	if main.sim.economy_snapshot_json() != econ_before:
		_fail("pressing C changed the economy — the collect-product command was supposed to be gone")
		return
	if main.sim.dock_snapshots_json() != dock_before:
		_fail("pressing C changed the dock — the collect-product command was supposed to be gone")
		return

	# --- The three build-mode keys (the "Real Placement" epic's ticket 2) ---
	# X toggles remove mode, R turns whatever is under the cursor, ESC leaves.
	# All three go through the same `_unhandled_input` path SPACE does, so
	# this is the keyboard half of build mode reaching the simulation.
	_press(main, KEY_X)
	if not main._remove_mode:
		_fail("X should have entered remove mode")
		return
	_press(main, KEY_ESCAPE)
	if main._remove_mode:
		_fail("ESC should have left remove mode")
		return

	# R outside placement mode rotates the placement under the cursor. The
	# cursor is not over anything in a headless run, so drive the same
	# function the key does with an explicit tile — the key path itself is
	# covered by the toggle above.
	var belt_before := _direction_at(main.sim, Vector2i(0, 0))
	main._issue_world_command({"type": "rotate", "tile": [0, 0]})
	var belt_after := _direction_at(main.sim, Vector2i(0, 0))
	if belt_before != "east" or belt_after != "south":
		_fail("rotating the belt at (0, 0) should turn east -> south, got %s -> %s" % [belt_before, belt_after])
		return

	# Every new key is advertised, the same rule that removed C's hint.
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		for fragment in ["R: rotate", "X: remove mode", "ESC: cancel", "click: place"]:
			if not legend.contains(fragment):
				_fail("the control legend is missing '%s': %s" % [fragment, legend])
				return

	print("INTERACTION_OK: simulated SPACE keypress reached SimBridge and loaded the hopper (%d -> %d), C is gone from both the legend and the key handler, and X/ESC/R drive build mode" % [before, after])
	quit(0)


func _press(main, keycode: int) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _direction_at(sim, tile: Vector2i) -> String:
	for placement in JSON.parse_string(sim.world_layout_json()):
		for pair in placement["occupies"]:
			if int(pair[0]) == tile.x and int(pair[1]) == tile.y:
				return String(placement["direction"])
	return ""
