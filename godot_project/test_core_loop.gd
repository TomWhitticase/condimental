# Headless proof of the real `gdext` JSON boundary (ARCHITECTURE.md §7).
#
# Run with:
#   godot --headless --path godot_project --script test_core_loop.gd
#
# This is deliberately separate from sim_core's own `cargo test` suite, which
# already proves conservation, contract payouts and fluid consumption across
# the whole shipped chain in pure Rust (against `sim::reference_chain`).
# What this script exercises is the part cargo test can't: GDScript building
# and parsing the JSON `issue_command_json` / `world_layout_json` /
# `chunk_snapshot_json` / `query_tile_json` wire format through Godot's own
# Variant/String/JSON machinery.
#
# **It now starts from an empty world and BUILDS one**, which is both the new
# reality (`World::new` places nothing — the "Real Placement" epic's ticket 1)
# and a better test: the factory it asserts against is one `purchase` and
# `place` put there over this same boundary. See test_layouts.gd for why that
# factory is small.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")


func _fail(msg: String) -> void:
	printerr("CORE_LOOP_FAIL: %s" % msg)
	quit(1)


## Packs a chunk coordinate the way `sim::encode_chunk_id` does, so a test can
## ask for a specific chunk rather than depending on `dirty_chunk_ids`, which
## only reports chunks the LAST tick touched.
func _chunk_id(cx: int, cy: int) -> int:
	return (cx << 32) | (cy & 0xFFFFFFFF)


func _initialize() -> void:
	var sim = SimBridge.new()

	# --- A new game is EMPTY. This is a real, newly reachable state. --------
	var empty: Array = JSON.parse_string(sim.world_layout_json())
	if not (empty is Array) or not empty.is_empty():
		_fail("a new world must contain nothing at all, got %s" % str(empty))
		return
	for call_name in ["hopper_snapshots_json", "fluid_tank_snapshots_json", "dock_snapshots_json", "fluid_snapshots_json"]:
		var parsed = JSON.parse_string(sim.call(call_name))
		if not (parsed is Array) or not parsed.is_empty():
			_fail("%s must be an empty array in a new world, got %s" % [call_name, str(parsed)])
			return
	var shop: Dictionary = JSON.parse_string(sim.shop_snapshot_json())
	# STARTING_MONEY is 3000 since the inventory-hotbar ticket, and the
	# shipped line costs about 1,200 — so a fresh balance has to be enough
	# to buy a whole factory, not just a first belt. The exact relationship is
	# pinned in `sim_core/tests/shop.rs` against the chain's own dynamically
	# priced bill; what is checked here is that the wire reports a real
	# balance at all.
	if float(shop["money"]) <= 0.0:
		_fail("a new world must start with STARTING_MONEY, got %s" % str(shop["money"]))
		return
	for entry in shop["entries"]:
		if int(entry["owned"]) != 0:
			_fail("a new world owns nothing, but '%s' reports %s" % [entry["kind"], str(entry["owned"])])
			return

	# --- Build one, through the same wire the scene uses. -------------------
	Layouts.build_demo_line(sim)
	var layout: Array = JSON.parse_string(sim.world_layout_json())
	if layout.size() != 13:
		_fail("expected the 13-piece demo line, got %d placements" % layout.size())
		return
	var directions_seen := {}
	for placement in layout:
		for field in ["kind", "tile", "direction", "footprint", "occupies", "recipe"]:
			if not placement.has(field):
				_fail("world_layout_json entry is missing '%s': %s" % [field, str(placement)])
				return
		directions_seen[String(placement["direction"])] = true
	# The fixture deliberately uses more than one facing — the whole point of
	# ticket 1 — so a wire format that dropped `direction` would show up here.
	for wanted in ["east", "north", "west", "south"]:
		if not directions_seen.has(wanted):
			_fail("the demo line should place something facing %s: %s" % [wanted, str(directions_seen.keys())])
			return

	# --- The three new commands round-trip. ---------------------------------
	# Rotate: any tile of a footprint, 90 degrees clockwise, contents kept.
	var before := _placement_at(sim, Vector2i(0, 0))
	if String(before["direction"]) != "east":
		_fail("the first belt should face east, got %s" % str(before["direction"]))
		return
	sim.issue_command_json(JSON.stringify({"type": "rotate", "tile": [0, 0]}))
	var after := _placement_at(sim, Vector2i(0, 0))
	if String(after["direction"]) != "south":
		_fail("rotate must turn east -> south (Direction::rotate_cw), got %s" % str(after["direction"]))
		return
	sim.issue_command_json(JSON.stringify({"type": "rotate", "tile": [0, 0]}))
	sim.issue_command_json(JSON.stringify({"type": "rotate", "tile": [0, 0]}))
	sim.issue_command_json(JSON.stringify({"type": "rotate", "tile": [0, 0]}))
	if String(_placement_at(sim, Vector2i(0, 0))["direction"]) != "east":
		_fail("four rotations must return a placement to where it started")
		return

	# Remove: credits one back to the owned count, and the tile empties.
	sim.issue_command_json(JSON.stringify({"type": "remove", "tile": [-1, 2]}))
	if not _placement_at(sim, Vector2i(-1, 2)).is_empty():
		_fail("remove left something at (-1, 2)")
		return
	if _owned(sim, "belt") != 1:
		_fail("remove must credit one belt back to inventory, got %d" % _owned(sim, "belt"))
		return
	# Place it back, facing a different way, out of that same credited unit.
	sim.issue_command_json(JSON.stringify({
		"type": "place", "kind": "belt", "tile": [-1, 2], "direction": "south",
	}))
	if String(_placement_at(sim, Vector2i(-1, 2))["direction"]) != "south":
		_fail("re-placing the removed belt should have kept the direction asked for")
		return
	if _owned(sim, "belt") != 0:
		_fail("placing must spend the owned unit again")
		return
	# A collision is a total no-op, not a partial apply.
	var placement_count: int = JSON.parse_string(sim.world_layout_json()).size()
	sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	sim.issue_command_json(JSON.stringify({
		"type": "place", "kind": "belt", "tile": [0, 0], "direction": "east",
	}))
	if JSON.parse_string(sim.world_layout_json()).size() != placement_count:
		_fail("placing onto an occupied tile must change nothing at all")
		return

	# --- The built line actually runs. --------------------------------------
	# Raw ingredients cost money since the chain rework, and `fill_hopper`
	# draws from bought stock — so a line that is never stocked produces
	# nothing at all, silently. Buying first is what a player does too.
	Layouts.stock(sim, "tomato", 4)
	sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 4}))
	var hoppers: Array = JSON.parse_string(sim.hopper_snapshots_json())
	if hoppers.size() != 1 or int(hoppers[0]["quantity"]) != 4:
		_fail("expected one hopper holding 4 tomatoes, got %s" % str(hoppers))
		return
	if hoppers[0].get("belt_tile") == null:
		_fail("the hopper faces a belt, so belt_tile must not be null: %s" % str(hoppers[0]))
		return

	var produced := 0
	for i in range(4000):
		sim.tick()
		produced = _item_count(sim, "tomato_paste")
		if produced > 0:
			break
	if produced <= 0:
		_fail("the hopper -> belts -> crusher line never produced tomato_paste")
		return

	# --- Tile queries, one per variant the fixture contains. ----------------
	var expectations := {
		"belt": Vector2i(1, 0),
		"machine": Vector2i(3, 2),
		"hopper": Vector2i(-1, 0),
		"pipe": Vector2i(5, 2),
	}
	for expected_kind in expectations.keys():
		var tile: Vector2i = expectations[expected_kind]
		var parsed = JSON.parse_string(sim.query_tile_json(tile.x, tile.y))
		if parsed == null or String(parsed.get("kind", "")) != expected_kind:
			_fail("query_tile_json at %s should be a %s: %s" % [str(tile), expected_kind, str(parsed)])
			return
		if not parsed.has("direction"):
			_fail("every query_tile_json variant carries a direction now: %s" % str(parsed))
			return
	if sim.query_tile_json(9999, 9999) != "":
		_fail("query_tile_json on an empty tile should return an empty string")
		return

	# --- The economy's wire shape (milestone 5, then contracts-as-data). ----
	var econ = JSON.parse_string(sim.economy_snapshot_json())
	if econ == null or not econ.has("money") or not econ.has("delivery_stock") or not econ.has("contracts"):
		_fail("economy_snapshot_json did not return the expected shape: %s" % str(econ))
		return
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

	print("CORE_LOOP_OK: empty world -> 13 placements bought and placed over the wire, place/remove/rotate round-trip, the built line produced tomato_paste, tile queries and economy shape correct")
	quit(0)


func _placement_at(sim, tile: Vector2i) -> Dictionary:
	for placement in JSON.parse_string(sim.world_layout_json()):
		for pair in placement["occupies"]:
			if int(pair[0]) == tile.x and int(pair[1]) == tile.y:
				return placement
	return {}


func _owned(sim, kind: String) -> int:
	for entry in JSON.parse_string(sim.shop_snapshot_json())["entries"]:
		if String(entry["kind"]) == kind:
			return int(entry["owned"])
	return -1


## Units of `item` anywhere on a belt or in a machine buffer, across the two
## chunks the demo line straddles (it reaches x = -1, i.e. chunk (-1, 0)).
func _item_count(sim, item: String) -> int:
	var total := 0
	for chunk in [_chunk_id(0, 0), _chunk_id(-1, 0)]:
		var snapshot = JSON.parse_string(sim.chunk_snapshot_json(chunk))
		if snapshot == null:
			continue
		for belt in snapshot.get("belts", []):
			for pair in belt["items"]:
				if String(pair[0]) == item:
					total += 1
		for machine in snapshot.get("machines", []):
			for pair in machine["output"]:
				if String(pair[0]) == item:
					total += int(pair[1])
	return total
