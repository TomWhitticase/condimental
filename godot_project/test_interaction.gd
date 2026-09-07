# Headless proof that main.tscn's keyboard input actually reaches the
# simulation (ARCHITECTURE.md §9 item 3's "interactable" requirement) —
# not just that the underlying issue_command_json API works (test_core_loop.gd
# already covers that), but that Main._unhandled_input's key handling does.
#
# Run with:
#   godot --headless --path godot_project --script test_interaction.gd
extends SceneTree


func _fail(msg: String) -> void:
	printerr("INTERACTION_FAIL: %s" % msg)
	quit(1)


func _hopper_quantity(sim) -> int:
	var hopper = JSON.parse_string(sim.hopper_snapshot_json())
	if hopper == null:
		return -1
	return int(hopper["quantity"])


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	# `_ready()` only fires on the engine's own next-frame processing, which
	# this headless script-mode SceneTree never runs — call it directly.
	main._ready()

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

	print("INTERACTION_OK: simulated SPACE keypress reached SimBridge and loaded the hopper (%d -> %d)" % [before, after])
	quit(0)
