# Headless proof for milestone 3 (ARCHITECTURE.md §7, §9 item 3).
#
# Run with:
#   godot --headless --path godot_project --script test_core_loop.gd
#
# This is deliberately separate from sim_core's own `cargo test` suite
# (which already proves conservation across thousands of ticks in pure
# Rust). What this script actually exercises is the part cargo test can't:
# the real `gdext` boundary — GDScript building/parsing the JSON
# `issue_command_json`/`chunk_snapshot_json` wire format through Godot's
# own Variant/String/JSON machinery, not sim_core's.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("CORE_LOOP_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var sim = SimBridge.new()

	# Spawn all 4 tomatoes the bundled recipe needs, ticking between each
	# so every spawn respects the belt's min-spacing (mirrors sim_core's
	# own `try_spawn` contract — see sim_core/src/sim/belt.rs).
	var spawned := 0
	var attempts := 0
	while spawned < 4 and attempts < 2000:
		attempts += 1
		var before := _tomato_count(sim)
		sim.issue_command_json(JSON.stringify({"type": "spawn_item", "item": "tomato"}))
		if _tomato_count(sim) > before:
			spawned += 1
		sim.tick()

	if spawned != 4:
		_fail("could not get all 4 tomatoes onto the belt (spawned %d)" % spawned)
		return

	# Run long enough for them to reach the machine and for the recipe's
	# full time_ticks to elapse (data/recipes.ron: time_ticks = 180).
	for i in range(1000):
		sim.tick()

	var snapshot = JSON.parse_string(sim.chunk_snapshot_json(0))
	if snapshot == null:
		_fail("chunk_snapshot_json(0) did not parse as JSON")
		return

	var machine = snapshot.get("machine")
	if machine == null:
		_fail("snapshot has no machine")
		return

	var output_before: Array = machine["output"]
	var sauce_before := _count_of(output_before, "tomato_sauce")
	if sauce_before != 1:
		_fail("expected exactly 1 tomato_sauce in the output buffer, got %d" % sauce_before)
		return

	sim.issue_command_json(JSON.stringify({"type": "collect_output"}))
	var after = JSON.parse_string(sim.chunk_snapshot_json(0))
	var sauce_after := _count_of(after["machine"]["output"], "tomato_sauce")
	if sauce_after != 0:
		_fail("collect_output did not empty the output buffer (still %d)" % sauce_after)
		return

	# query_tile_json: confirm both the belt and machine tiles resolve to
	# the right kind, and an empty tile resolves to nothing.
	var belt_tile = JSON.parse_string(sim.query_tile_json(0, 0))
	if belt_tile == null or belt_tile.get("kind") != "belt":
		_fail("query_tile_json(0,0) did not report a belt tile: %s" % str(belt_tile))
		return
	var machine_tile = JSON.parse_string(sim.query_tile_json(4, 0))
	if machine_tile == null or machine_tile.get("kind") != "machine":
		_fail("query_tile_json(4,0) did not report a machine tile: %s" % str(machine_tile))
		return
	if sim.query_tile_json(99, 99) != "":
		_fail("query_tile_json on an empty tile should return an empty string")
		return

	print("CORE_LOOP_OK: 4 tomatoes -> 1 tomato_sauce, collected, tile queries correct")
	quit(0)


func _tomato_count(sim) -> int:
	var snapshot = JSON.parse_string(sim.chunk_snapshot_json(0))
	var total := 0
	if snapshot.get("belt") != null:
		for pair in snapshot["belt"]["items"]:
			if pair[0] == "tomato":
				total += 1
	if snapshot.get("machine") != null:
		total += _count_of(snapshot["machine"]["input"], "tomato")
	return total


func _count_of(pairs: Array, item_name: String) -> int:
	for pair in pairs:
		if pair[0] == item_name:
			return int(pair[1])
	return 0
