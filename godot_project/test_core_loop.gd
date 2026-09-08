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

	# Load the 4 tomatoes the first recipe needs into the chain's hopper
	# ("Machines improvement" ticket 3). The hopper — not this command —
	# meters them onto the first belt over the following ticks, one at a
	# time, so nothing here has to respect the belt's min-spacing itself.
	sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 4}))
	# ...and load the pipe run's vinegar tank the same way ("Visual overhaul
	# round 2" ticket 1 replaced the always-refilling off-map source with a
	# tank the player fills), or the fluid-gated mixing stage never runs.
	sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
	var hopper = JSON.parse_string(sim.hopper_snapshot_json())
	if hopper == null or not hopper.has("quantity") or not hopper.has("capacity") or not hopper.has("fill"):
		_fail("hopper_snapshot_json did not return the expected shape: %s" % str(hopper))
		return
	if int(hopper["quantity"]) != 4:
		_fail("expected 4 tomatoes loaded into the hopper, got %s" % str(hopper["quantity"]))
		return
	if _tomato_count(sim) != 0:
		_fail("filling the hopper must not put anything on a belt — that happens on tick")
		return

	# Run long enough for the single resulting unit to travel every belt, clear
	# every machine's `time_ticks`, run down the DOCK's own belt and be shipped
	# into the economy's shared delivery pool.
	#
	# That last hop is the delivery-dock ticket's whole point, and this loop is
	# where it is proven through the real gdext boundary: no command is issued
	# here at all. `PlayerCommand::CollectOutput` — the **C** keypress that used
	# to lift finished cases out of the last machine's buffer — does not exist
	# any more, and nothing replaced it; the dock ships by itself, every tick.
	for i in range(20000):
		sim.tick()
		if _delivered_count(sim, final_recipe) > 0:
			break

	var delivered := _delivered_count(sim, final_recipe)
	if delivered != 1:
		_fail("expected exactly 1 unit of '%s' shipped into the delivery pool with no player command, got %d" % [final_recipe, delivered])
		return
	if _final_output_count(sim, final_recipe) != 0:
		_fail("the last machine should have handed its case to the dock's belt, but still holds %d" % _final_output_count(sim, final_recipe))
		return

	# The dock's own snapshot, over the same wire. `contents` is asserted
	# EMPTY on purpose: the dock is a pass-through (see sim_core's dock.rs),
	# so between ticks it holds nothing and `shipped_total` is the number
	# that actually moved.
	var dock = JSON.parse_string(sim.dock_snapshot_json())
	if dock == null:
		_fail("dock_snapshot_json did not parse")
		return
	for field in ["tile", "kind", "footprint", "belt_start_tile", "belt_length_tiles", "contents", "capacity_per_item", "shipped_total"]:
		if not dock.has(field):
			_fail("dock_snapshot_json is missing '%s': %s" % [field, str(dock)])
			return
	if String(dock["kind"]) != "dock":
		_fail("dock_snapshot_json reported kind '%s'" % str(dock["kind"]))
		return
	if int(dock["shipped_total"]) < 1:
		_fail("the dock reports nothing shipped even though the pool holds a case: %s" % str(dock))
		return
	if dock["contents"].size() != 0:
		_fail("the dock is a pass-through — its buffer must read empty between ticks, got %s" % str(dock["contents"]))
		return

	# Milestone 5 (Economy MVP, ARCHITECTURE.md §9 item 5): economy_snapshot_json
	# exposes money + every active contract's status through the same gdext
	# JSON boundary this script already exercises for chunk/tile/layout data.
	# Full contract economics (cadence math, payout correctness) is already
	# proven headlessly in `cargo test` (sim_core/tests/economy.rs); this just
	# confirms the wire format survives a real Godot Variant/JSON round trip.
	var econ = JSON.parse_string(sim.economy_snapshot_json())
	if econ == null or not econ.has("money") or not econ.has("delivery_stock") or not econ.has("contracts"):
		_fail("economy_snapshot_json did not return the expected shape: %s" % str(econ))
		return
	# Contracts became `data/contracts.ron` content, so this is an ARRAY of
	# them (the four inline contract fields this used to read are gone), each
	# with its own mix, cadence clock and counters. All of them draw from the
	# one shared `delivery_stock` pool.
	var contracts: Array = econ["contracts"]
	if contracts.is_empty():
		_fail("economy_snapshot_json reported no contracts at all: %s" % str(econ))
		return
	for contract in contracts:
		for field in ["id", "name", "mix", "progress", "cadence_ticks", "next_delivery_tick", "payout", "deliveries_made", "deliveries_missed"]:
			if not contract.has(field):
				_fail("contract '%s' is missing '%s': %s" % [str(contract.get("id", "?")), field, str(contract)])
				return
		if contract["mix"].size() != contract["progress"].size():
			_fail("contract '%s': mix and progress must zip positionally" % str(contract["id"]))
			return

	# The single unit the dock shipped above went into the shared pool even
	# though it alone isn't enough to satisfy every contract's quantity —
	# confirm both the raw pool and per-contract progress see it.
	var stock_seen := false
	for pair in econ["delivery_stock"]:
		if pair[0] == final_recipe and int(pair[1]) > 0:
			stock_seen = true
	if not stock_seen:
		_fail("expected delivery_stock to hold the shipped unit of '%s': %s" % [final_recipe, str(econ["delivery_stock"])])
		return
	var found_progress := false
	for contract in contracts:
		for pair in contract["progress"]:
			if pair[0] == final_recipe and int(pair[1]) > 0:
				found_progress = true
	if not found_progress:
		_fail("expected some contract's progress to reflect the shipped unit of '%s': %s" % [final_recipe, str(contracts)])
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

	# The dock's tile answers query_tile_json too, with its own variant.
	var dock_tile_pair: Array = dock["tile"]
	var dock_tile = JSON.parse_string(sim.query_tile_json(int(dock_tile_pair[0]), int(dock_tile_pair[1])))
	if dock_tile == null or dock_tile.get("kind") != "dock":
		_fail("query_tile_json did not report a dock tile at %s: %s" % [str(dock_tile_pair), str(dock_tile)])
		return

	print("CORE_LOOP_OK: 4 tomatoes unloaded from the hopper, travelled all %d stages -> 1 '%s', shipped by the dock with no player command, tile queries correct" % [layout.size(), final_recipe])
	quit(0)


## How many units of `item` are standing in the economy's shared delivery
## pool — i.e. what the dock has shipped and no contract has claimed yet.
func _delivered_count(sim, item: String) -> int:
	var econ = JSON.parse_string(sim.economy_snapshot_json())
	if econ == null:
		return 0
	for pair in econ.get("delivery_stock", []):
		if String(pair[0]) == item:
			return int(pair[1])
	return 0


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
