# Headless proof of the "Visual overhaul round 2" epic's ticket 3 — the
# belt/pipe-to-machine CONNECTIONS. Structural claims only, the same division
# test_machines.gd and test_fluids.gd already draw: whether a junction reads
# as an opening to a player is what scripts/screenshot-scene.sh is for, and
# every shape in this ticket was arrived at by looking at one.
#
# Run with:
#   godot --headless --path godot_project --script test_connections.gd
#
# Verifies:
#   * the three belt models (plain / inlet-adjacent / outlet-adjacent) all
#     import as real, non-empty, DISTINCT meshes — two identical models would
#     make the per-tile choice below a lie,
#   * that choice is DERIVED FROM THE SIMULATION'S LAYOUT, not hardcoded:
#     every stage's last belt tile draws `belt_inlet` because the tile ahead
#     of it (along that stage's own travel direction) is covered by a machine
#     footprint, every tile after a machine or the hopper draws `belt_outlet`,
#     and everything else stays plain,
#   * `_belt_asset_for` falls back to the plain model for any direction the
#     variants are not authored for — the corner limitation `world.rs`'s
#     module doc states, checked rather than asserted in prose,
#   * `_machine_occupied_tiles` covers EVERY tile of a multi-tile footprint,
#     not just its origin (the cooking vat is 2x2), and includes the hopper,
#     which discharges onto a belt exactly the way a machine does,
#   * the machine models actually grew junction hardware: each kind's mesh
#     spans the full width of its footprint at belt height, i.e. something
#     reaches the tile edges the belt meets. (The openings themselves are
#     checked precisely, voxel by voxel, by `cargo test -p voxgen` — a
#     rendered mesh cannot tell "carved" from "painted dark", and that
#     distinction is the whole point. See assets.rs's junction tests.)
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("CONNECTIONS_FAIL: %s" % msg)
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


func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


## Total vertex count of an asset as actually imported, used only as an
## identity: two belt variants that produced the same number here would mean
## the generator changed nothing between them.
func _vertex_count(asset: String) -> int:
	var node: Node3D = main._asset_node(asset)
	var total := 0
	for mi in _meshes(node, []):
		var mesh: Mesh = mi.mesh
		if mesh == null:
			continue
		for surface in range(mesh.get_surface_count()):
			total += mesh.surface_get_arrays(surface)[Mesh.ARRAY_VERTEX].size()
	node.free()
	return total


func _run_checks() -> void:
	var layout: Array = JSON.parse_string(main.sim.chain_layout_json())
	if layout.is_empty():
		_fail("chain_layout_json returned no stages")
		return

	# --- the three belt models are real and distinct ----------------------
	var counts := {}
	for asset in [main.BELT_ASSET, main.BELT_INLET_ASSET, main.BELT_OUTLET_ASSET]:
		var n := _vertex_count(asset)
		if n == 0:
			_fail("belt model '%s' imported no geometry — run `cargo xtask`" % asset)
			return
		counts[asset] = n
	if counts[main.BELT_ASSET] == counts[main.BELT_INLET_ASSET]:
		_fail("belt_inlet is geometrically identical to a plain belt tile")
		return
	if counts[main.BELT_ASSET] == counts[main.BELT_OUTLET_ASSET]:
		_fail("belt_outlet is geometrically identical to a plain belt tile")
		return
	if counts[main.BELT_INLET_ASSET] == counts[main.BELT_OUTLET_ASSET]:
		_fail("belt_inlet and belt_outlet are geometrically identical to each other")
		return

	# --- occupancy covers whole footprints, and the hopper ----------------
	var occupied: Dictionary = main._machine_occupied_tiles(layout)
	var multi_tile_checked := 0
	for stage in layout:
		var pair: Array = stage["machine_tile"]
		var origin := Vector2i(int(pair[0]), int(pair[1]))
		var size: Vector2i = main._footprint_size(stage.get("machine_footprint"))
		if size != Vector2i.ONE:
			multi_tile_checked += 1
		for dy in range(size.y):
			for dx in range(size.x):
				var tile := origin + Vector2i(dx, dy)
				if not occupied.has("%d,%d" % [tile.x, tile.y]):
					_fail("footprint tile %s of stage '%s' is not occupied" % [str(tile), stage["recipe"]])
					return
	if multi_tile_checked == 0:
		_fail("no multi-tile machine in the layout — the footprint case is untested")
		return

	var hopper: Dictionary = main._read_hopper_snapshot()
	if hopper.is_empty():
		_fail("hopper_snapshot_json returned nothing; the outlet-tile rule can't be checked")
		return
	var hopper_pair: Array = hopper["tile"]
	if not occupied.has("%d,%d" % [int(hopper_pair[0]), int(hopper_pair[1])]):
		_fail("the hopper's own tile is not treated as a discharge neighbour")
		return

	# --- the per-tile choice matches the layout ---------------------------
	var inlet_tiles := 0
	var outlet_tiles := 0
	var plain_tiles := 0
	for stage in layout:
		var start_pair: Array = stage["belt_start_tile"]
		var start := Vector2i(int(start_pair[0]), int(start_pair[1]))
		var machine_pair: Array = stage["machine_tile"]
		var direction: Vector2i = main._belt_direction(
			start, Vector2i(int(machine_pair[0]), int(machine_pair[1]))
		)
		if direction == Vector2i.ZERO:
			_fail("stage '%s' has no derivable belt direction" % stage["recipe"])
			return
		var length := int(stage["belt_length_tiles"])
		for i in range(length):
			var tile := start + direction * i
			var key := "%d,%d" % [tile.x, tile.y]
			if not main._belt_assets.has(key):
				_fail("belt tile %s was never drawn" % str(tile))
				return
			var drew: String = main._belt_assets[key]
			var expect: String = main.BELT_ASSET
			if occupied.has("%d,%d" % [tile.x + direction.x, tile.y + direction.y]):
				expect = main.BELT_INLET_ASSET
			elif occupied.has("%d,%d" % [tile.x - direction.x, tile.y - direction.y]):
				expect = main.BELT_OUTLET_ASSET
			if drew != expect:
				_fail("belt tile %s drew '%s', expected '%s'" % [str(tile), drew, expect])
				return
			if expect == main.BELT_INLET_ASSET:
				inlet_tiles += 1
			elif expect == main.BELT_OUTLET_ASSET:
				outlet_tiles += 1
			else:
				plain_tiles += 1

	if inlet_tiles != layout.size():
		_fail("expected one inlet-adjacent belt tile per stage (%d), got %d" % [layout.size(), inlet_tiles])
		return
	if outlet_tiles != layout.size():
		# One per stage: stage 0's head sits against the hopper, every later
		# stage's head against the previous stage's machine.
		_fail("expected one outlet-adjacent belt tile per stage (%d), got %d" % [layout.size(), outlet_tiles])
		return
	if plain_tiles == 0:
		_fail("no plain belt tiles left — the variants would be meaningless")
		return

	# --- the DOCK's belt, which belongs to no stage -----------------------
	# The delivery-dock ticket laid one more belt past the last machine, into
	# the dock. It is not in `chain_layout_json` (nothing in recipes.ron names
	# the dock), so the loop above never sees it — and it is exactly the run
	# whose two ends are the ticket's own junctions: its head sits against the
	# packer, which therefore had to grow an outfeed it never had, and its
	# tail sits against the dock, which is only picked up if
	# `_machine_occupied_tiles` was taught about the dock.
	var dock: Dictionary = JSON.parse_string(main.sim.dock_snapshot_json())
	if dock.is_empty():
		_fail("dock_snapshot_json returned nothing — every world has a dock")
		return
	var dock_start_pair: Array = dock["belt_start_tile"]
	var dock_start := Vector2i(int(dock_start_pair[0]), int(dock_start_pair[1]))
	var dock_len := int(dock["belt_length_tiles"])
	if dock_len < 2:
		_fail("the dock's belt is %d tile(s) long — too short to have two distinct ends" % dock_len)
		return
	var head_key := "%d,%d" % [dock_start.x, dock_start.y]
	var tail := dock_start + Vector2i(dock_len - 1, 0)
	var tail_key := "%d,%d" % [tail.x, tail.y]
	if not main._belt_assets.has(head_key) or not main._belt_assets.has(tail_key):
		_fail("the dock's belt was not drawn (head %s, tail %s)" % [head_key, tail_key])
		return
	if String(main._belt_assets[head_key]) != main.BELT_OUTLET_ASSET:
		_fail("the dock belt's head sits against the last machine but drew '%s'" % str(main._belt_assets[head_key]))
		return
	if String(main._belt_assets[tail_key]) != main.BELT_INLET_ASSET:
		_fail("the dock belt's tail sits against the dock but drew '%s'" % str(main._belt_assets[tail_key]))
		return
	if not occupied.has("%d,%d" % [dock_start.x + dock_len, dock_start.y]):
		_fail("_machine_occupied_tiles doesn't cover the dock's own tile")
		return

	# --- a direction the variants are NOT authored for stays plain --------
	# `world.rs`'s corner note: nothing in the simulation ever lays a belt
	# along another axis, and no rotated model exists, so the renderer must
	# fall back rather than draw a wrong-facing end cap.
	var first_stage: Dictionary = layout[0]
	var first_start_pair: Array = first_stage["belt_start_tile"]
	var first_start := Vector2i(int(first_start_pair[0]), int(first_start_pair[1]))
	for turned in [Vector2i(0, 1), Vector2i(-1, 0), Vector2i(0, -1)]:
		var fallback: String = main._belt_asset_for(first_start, turned, occupied)
		if fallback != main.BELT_ASSET:
			_fail("a belt running %s drew '%s'; only +X variants are authored" % [str(turned), fallback])
			return

	# --- the machines actually grew junction hardware ---------------------
	# Not "is there an opening" (only the voxel-level tests can tell a carved
	# hole from a dark rectangle) but "does the model reach the tile edges the
	# belt meets", which is what the previous seam fix and this ticket's
	# housings both had to be true of.
	var tile: float = main.TILE
	for stage in layout:
		var recipe := String(stage["recipe"])
		var node: Node3D = main.get_node_or_null("Chain/machine_%s" % recipe)
		if node == null:
			_fail("stage '%s' rendered no machine node" % recipe)
			return
		var size: Vector2i = main._footprint_size(stage.get("machine_footprint"))
		var box := AABB()
		var first := true
		for mi in _meshes(node, []):
			var here: AABB = mi.mesh.get_aabb()
			if first:
				box = here
				first = false
			else:
				box = box.merge(here)
		var expected_width := float(size.x) * tile
		if box.size.x < expected_width - 0.01:
			_fail("machine '%s' spans %.2f of its %.2f-wide footprint — it does not reach the tile edges its belt meets" % [
				recipe, box.size.x, expected_width,
			])
			return

	print("CONNECTIONS_OK: 3 distinct belt models, %d inlet-adjacent + %d outlet-adjacent + %d plain tiles all chosen from the sim's own layout, %d footprint(s) bigger than one tile covered, non-+X directions fall back to the plain model, every machine reaches its footprint's full width" % [
		inlet_tiles, outlet_tiles, plain_tiles, multi_tile_checked,
	])
	quit(0)
