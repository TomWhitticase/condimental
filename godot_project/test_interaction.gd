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


func _tomato_count(sim) -> int:
	var snapshot = JSON.parse_string(sim.chunk_snapshot_json(0))
	var total := 0
	if snapshot.get("belt") != null:
		for pair in snapshot["belt"]["items"]:
			if pair[0] == "tomato":
				total += 1
	return total


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	# `_ready()` only fires on the engine's own next-frame processing, which
	# this headless script-mode SceneTree never runs — call it directly.
	main._ready()

	var before := _tomato_count(main.sim)

	var key_event := InputEventKey.new()
	key_event.keycode = KEY_SPACE
	key_event.pressed = true
	main._unhandled_input(key_event)

	var after := _tomato_count(main.sim)
	if after != before + 1:
		_fail("pressing SPACE via _unhandled_input did not spawn a tomato onto the belt (before=%d after=%d)" % [before, after])
		return

	print("INTERACTION_OK: simulated SPACE keypress reached SimBridge and spawned a tomato")
	quit(0)
