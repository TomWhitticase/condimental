# Headless proof for milestone 4 (ARCHITECTURE.md §7, §9 item 4).
#
# Run with:
#   godot --headless --path godot_project --script test_core_loop.gd
#
# This is deliberately separate from sim_core's own `cargo test` suite
# (which already proves conservation across the whole multi-stage chain in
# pure Rust). What this script actually exercises is the part cargo test
# can't: the real `gdext` boundary — GDScript building/parsing the JSON
# `issue_command_json`/`chunk_snapshot_json`/`chain_layout_json` wire format
# through Godot's own Variant/String/JSON machinery, not sim_core's — driven
# through the whole chain end to end, not just one belt/machine.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("CORE_LOOP_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var sim = SimBridge.new()

	var layout: Array = JSON.parse_string(sim.chain_layout_json())
	if layout.size() < 2:
		_fail("expected a multi-stage chain (>=2 recipes), got %d stage(s)" % layout.size())
		return
	var final_recipe: String = layout[layout.size() - 1]["recipe"]

	# Spawn all 4 tomatoes the first recipe needs, ticking between each so
	# every spawn respects the belt's min-spacing (mirrors sim_core's own
	# `try_spawn` contract — see sim_core/src/sim/belt.rs).
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
		_fail("could not get all 4 tomatoes onto the first belt (spawned %d)" % spawned)
		return

	# Run long enough for the single resulting unit to travel every belt and
	# clear every machine's `time_ticks` in the chain.
	for i in range(20000):
		sim.tick()
		if _final_output_count(sim, final_recipe) > 0:
			break

	var final_output := _final_output_count(sim, final_recipe)
	if final_output != 1:
		_fail("expected exactly 1 unit of the final product ('%s') in output, got %d" % [final_recipe, final_output])
		return

	sim.issue_command_json(JSON.stringify({"type": "collect_output"}))
	var after := _final_output_count(sim, final_recipe)
	if after != 0:
		_fail("collect_output did not empty the final stage's output buffer (still %d)" % after)
		return

	# query_tile_json: confirm the first belt and first machine tiles
	# resolve to the right kind, and an empty tile resolves to nothing.
	var first_belt_start: Array = layout[0]["belt_start_tile"]
	var belt_tile = JSON.parse_string(sim.query_tile_json(int(first_belt_start[0]), int(first_belt_start[1])))
	if belt_tile == null or belt_tile.get("kind") != "belt":
		_fail("query_tile_json did not report a belt tile at the chain start: %s" % str(belt_tile))
		return
	var first_machine_tile: Array = layout[0]["machine_tile"]
	var machine_tile = JSON.parse_string(sim.query_tile_json(int(first_machine_tile[0]), int(first_machine_tile[1])))
	if machine_tile == null or machine_tile.get("kind") != "machine":
		_fail("query_tile_json did not report a machine tile at the first stage: %s" % str(machine_tile))
		return
	if sim.query_tile_json(9999, 9999) != "":
		_fail("query_tile_json on an empty tile should return an empty string")
		return

	print("CORE_LOOP_OK: 4 tomatoes travelled all %d stages -> 1 '%s', collected, tile queries correct" % [layout.size(), final_recipe])
	quit(0)


func _tomato_count(sim) -> int:
	var total := 0
	# The whole chain fits in chunk (0,0) at this milestone's scale (see
	# sim_core/tests/core_loop.rs's equivalent note) — always read it
	# directly rather than depending on dirty_chunk_ids, which only reports
	# chunks touched by the *last* tick.
	var snapshot = JSON.parse_string(sim.chunk_snapshot_json(0))
	if snapshot == null:
		return 0
	for belt in snapshot.get("belts", []):
		for pair in belt["items"]:
			if pair[0] == "tomato":
				total += 1
	for machine in snapshot.get("machines", []):
		total += _count_of(machine["input"], "tomato")
	return total


func _final_output_count(sim, final_recipe: String) -> int:
	var snapshot = JSON.parse_string(sim.chunk_snapshot_json(0))
	if snapshot == null:
		return 0
	for machine in snapshot.get("machines", []):
		if machine["recipe"] == final_recipe:
			var total := 0
			for pair in machine["output"]:
				total += int(pair[1])
			return total
	return 0


func _count_of(pairs: Array, item_name: String) -> int:
	for pair in pairs:
		if pair[0] == item_name:
			return int(pair[1])
	return 0
