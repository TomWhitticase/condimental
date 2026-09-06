# Headless round-trip proof for milestone 1 (ARCHITECTURE.md §7, §9).
#
# Run with:
#   godot --headless --path godot_project --script test_round_trip.gd
#
# Loads the sim_core GDExtension, calls into Rust via SimBridge.ping(),
# and asserts the known return value. Exits with a non-zero code on failure
# so this is CI-checkable without a human watching a window.
extends SceneTree


func _initialize() -> void:
	var bridge = SimBridge.new()
	var result = bridge.ping()

	if result == 42:
		print("ROUND_TRIP_OK: SimBridge.ping() == %s" % result)
		quit(0)
	else:
		printerr("ROUND_TRIP_FAIL: SimBridge.ping() == %s, expected 42" % result)
		quit(1)
