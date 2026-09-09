# Headless proof of the belt-to-building JUNCTIONS (the "Visual overhaul round
# 2" epic's ticket 3), and of the east-only fallback the "Real Placement"
# epic's ticket 2 left deliberately in place. Structural claims only, the same
# division test_machines.gd and test_fluids.gd already draw: whether a
# junction reads as an opening to a player is what
# scripts/screenshot-scene.sh is for.
#
# Run with:
#   godot --headless --path godot_project --script test_connections.gd
#
# Verifies:
#   * the three belt names (plain / inlet-adjacent / outlet-adjacent) all
#     import as real, non-empty meshes and are IDENTICAL: the pipes and
#     conveyors overhaul's ticket 2 retired the belt-side junction variants
#     (UI_UX.md §2b point 18/23 — the plain tile carries the junction), and
#     until ticket 5 stops asking for the old names they are aliases of
#     `belt_f0`. A variant that differed again would mean someone re-drew a
#     dead-plate onto an animated surface,
#   * that choice is derived from what the tile is actually NEXT TO in the
#     simulation's own layout: a belt facing a building takes the inlet
#     variant, a belt fed by one takes the outlet variant, and a belt with
#     neither stays plain,
#   * **the fallback holds under real placement.** The variants are authored
#     for a camera that can only see two faces (UI_UX.md §2b point 11), so
#     they are east-only, and three of the four facings a player can now
#     choose must fall back to the plain tile rather than draw a doorway
#     pointing away from the camera. That is the known, accepted art gap this
#     epic does not close, and this is where it is checked rather than
#     asserted in prose,
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

const Layouts = preload("res://test_layouts.gd")

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
## identity: the retired belt variants must produce the same number as the
## plain tile, because they ARE the plain tile until ticket 5 drops them.
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
	# Built HERE, not in `_initialize`: `_ready` runs twice under this harness
	# and each run creates a fresh (empty) world — see test_hover.gd's header.
	# hopper -> belt -> crusher ... belt -> dock: a building on both sides of a
	# belt tile, which is exactly what the two variants are for.
	Layouts.build_machines_fixture(main.sim)
	main._rebuild_world()

	# --- three real belt names, one model (the variants are retired) ------
	var plain_count := _vertex_count(main.BELT_ASSET)
	if plain_count <= 0:
		_fail("belt model '%s' imported nothing" % main.BELT_ASSET)
		return
	for asset in [main.BELT_INLET_ASSET, main.BELT_OUTLET_ASSET]:
		var n := _vertex_count(asset)
		if n <= 0:
			_fail("belt model '%s' imported nothing" % asset)
			return
		if n != plain_count:
			_fail("belt model '%s' differs from the plain tile (%d vs %d vertices) — the junction variants are retired aliases of belt_f0 until ticket 5" % [asset, n, plain_count])
			return

	# --- the per-tile choice follows what the tile is next to -------------
	# (0, 0) is fed by the hopper at (-1, 0) and points into the crusher at
	# (1, 0): the tile ahead wins, so it is an inlet tile.
	if main._belt_asset_for(Vector2i(0, 0), "east") != main.BELT_INLET_ASSET:
		_fail("the belt pointing into the crusher should draw the inlet variant, got '%s'" % main._belt_asset_for(Vector2i(0, 0), "east"))
		return
	# (4, 0) points into the dock at (5, 0) — the exit-end junction.
	if main._belt_asset_for(Vector2i(4, 0), "east") != main.BELT_INLET_ASSET:
		_fail("the belt pointing into the dock should draw the inlet variant, got '%s'" % main._belt_asset_for(Vector2i(4, 0), "east"))
		return
	# A belt with a building BEHIND it and nothing ahead catches a discharge.
	# The rule is about a tile's NEIGHBOURS, so it can be asked about a tile
	# nothing is standing on yet — which is exactly what the ghost does while
	# the player is deciding where to put one.
	if main._belt_asset_for(Vector2i(2, 0), "east") != main.BELT_OUTLET_ASSET:
		_fail("the belt catching the crusher's discharge should draw the outlet variant, got '%s'" % main._belt_asset_for(Vector2i(2, 0), "east"))
		return
	# ...and a belt with nothing on either side stays plain.
	if main._belt_asset_for(Vector2i(9, 9), "east") != main.BELT_ASSET:
		_fail("an isolated belt should draw the plain model")
		return

	# --- the east-only fallback, on a tile that WOULD take a variant -------
	# This is the accepted gap: (0, 0) is a real junction, and the moment the
	# player turns it the renderer must stop drawing a junction rather than
	# draw one facing away from the camera.
	for facing in ["north", "south", "west"]:
		var fallback: String = main._belt_asset_for(Vector2i(0, 0), facing)
		if fallback != main.BELT_ASSET:
			_fail("a %s-facing belt drew '%s'; only east variants are authored" % [facing, fallback])
			return
	# And the scene really builds that fallback, not just the function.
	# A belt is a MultiMesh instance, not a node (the pipes & conveyors
	# overhaul's ticket 4), so the scene is asked through `_tile_instance_at`.
	main._issue_world_command({"type": "rotate", "tile": [0, 0]})
	var turned: Dictionary = main._tile_instance_at(Vector2i(0, 0))
	if turned.is_empty():
		_fail("the rotated belt vanished from the scene")
		return
	if turned["asset"] != main.BELT_ASSET:
		_fail("the rotated belt is drawn as '%s', expected the plain '%s'" % [turned["asset"], main.BELT_ASSET])
		return
	if turned["nodes"].is_empty():
		_fail("the rotated belt rendered no meshes")
		return
	if absf(rad_to_deg(float(turned["yaw"])) - 90.0) > 0.5:
		_fail("a south-facing belt should be turned 90 degrees, got %.1f" % rad_to_deg(float(turned["yaw"])))
		return

	# --- the machines reach the tile edges their belts meet ---------------
	# Not "is there an opening" (only the voxel-level tests can tell a carved
	# hole from a dark rectangle) but "does the model reach the tile edges the
	# belt meets", which is what the seam fix and this ticket's housings both
	# had to be true of.
	for placement in main._placements:
		if placement.get("recipe") == null:
			continue
		var tile: Array = placement["tile"]
		var node: Node3D = main.get_node_or_null("World/%s_%d,%d" % [
			String(placement["kind"]), int(tile[0]), int(tile[1]),
		])
		if node == null:
			_fail("machine '%s' rendered no node" % String(placement["kind"]))
			return
		var size: Vector2i = main._footprint_size(placement.get("footprint"))
		var box := AABB()
		var first := true
		for mi in _meshes(node, []):
			var here: AABB = mi.mesh.get_aabb()
			if first:
				box = here
				first = false
			else:
				box = box.merge(here)
		var expected_width: float = float(size.x) * main.TILE
		if box.size.x < expected_width - 0.01:
			_fail("machine '%s' spans %.2f of its %.2f-wide footprint — it does not reach the tile edges its belt meets" % [
				String(placement["kind"]), box.size.x, expected_width,
			])
			return

	print("CONNECTIONS_OK: 3 belt names importing one real model (junction variants retired), the inlet/outlet/plain choice follows what each tile is next to in the sim's own layout, every non-east facing falls back to the plain tile (and the scene builds it, turned), and every machine reaches its footprint's full width")
	quit(0)
