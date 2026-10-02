# Headless proof of the belt-to-building JUNCTIONS: that a belt meeting a
# machine, a hopper or a dock is drawn connected at EVERY facing, now that the
# pipes and conveyors overhaul retired the belt-side junction variants
# (UI_UX.md §2b points 18 and 23 — the plain tile carries the junction and the
# machine's own hood and doorway do the rest). Structural claims only, the
# same division test_machines.gd and test_fluids.gd draw: whether a junction
# READS as an opening to a player is what scripts/screenshot-scene.sh
# (`--belt-gallery`) is for.
#
# Run with:
#   godot --headless --path godot_project --script test_connections.gd
#
# Verifies:
#   * the retired names (`belt`, `belt_inlet`, `belt_outlet`) no longer
#     import at all — their models and icons were deleted, not left as
#     aliases, so a stale lookup would fail loudly rather than draw frame 0
#     under an old name — and the four straight frames do import as real,
#     non-empty meshes,
#   * a belt handing to a machine and a belt handing to a dock are both drawn
#     with the plain straight family, and the tooltip says in words what each
#     hands to (`_belt_hover_text` reads the same `links` the shape does),
#   * turning the belt that a hopper feeds RE-SHAPES it: the hopper is then on
#     a flank, so the same tile becomes a right corner, spun to its new
#     facing. That replaces the old "every non-east facing falls back to the
#     plain tile" assertion, which pinned an art gap this overhaul closed,
#   * the machine models actually carry junction hardware: each kind's mesh
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


## Total vertex count of an asset as actually imported: zero means nothing
## under that name imported.
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
	# belt tile, which is exactly what a junction is.
	Layouts.build_machines_fixture(main.sim)
	main._rebuild_world()

	# --- the retired names are gone; the framed straight imports ---------
	for retired in ["belt", "belt_inlet", "belt_outlet"]:
		for class_name_ in ["matte", "metal"]:
			if ResourceLoader.exists("%s%s.%s.vox" % [main.VOX_DIR, retired, class_name_]):
				_fail("retired belt model '%s.%s.vox' is still staged — the aliases were meant to be deleted, not kept" % [retired, class_name_])
				return
	var straight: Array = main._belt_frames_for_shape(main.BELT_SHAPE_STRAIGHT)
	if straight.size() != main.BELT_FRAMES:
		_fail("the straight belt should be a %d-frame set, got %s" % [main.BELT_FRAMES, str(straight)])
		return
	for asset in straight:
		if _vertex_count(asset) <= 0:
			_fail("belt frame '%s' imported nothing" % asset)
			return

	# --- a belt into a machine, and one into a dock, are plain straights --
	# (0, 0) is fed by the hopper at (-1, 0) and points into the crusher at
	# (1, 0): fed from behind, so straight — the crusher's hood is the
	# junction. Likewise (4, 0) into the dock at (5, 0).
	for tile in [Vector2i(0, 0), Vector2i(4, 0)]:
		var instance: Dictionary = main._tile_instance_at(tile)
		if instance.is_empty() or instance["nodes"].is_empty():
			_fail("the belt at %s rendered nothing" % str(tile))
			return
		if instance["asset"] != straight[0]:
			_fail("the belt at %s meeting a building draws '%s', expected the plain '%s'" % [str(tile), instance["asset"], straight[0]])
			return
	var into_crusher: String = main._hover_text_at_tile(Vector2i(0, 0))
	if not into_crusher.contains("hands to Crusher") or not into_crusher.contains("fed from the west"):
		_fail("the belt into the crusher should say what feeds it and what it hands to, got: %s" % into_crusher.replace("\n", " / "))
		return
	var into_dock: String = main._hover_text_at_tile(Vector2i(4, 0))
	if not into_dock.contains("hands to Dock"):
		_fail("the belt into the dock should say so, got: %s" % into_dock.replace("\n", " / "))
		return

	# --- turning a fed belt re-shapes it, at the new facing ----------------
	# Rotate (0, 0) to face south. The hopper still feeds it from the west,
	# which is now the flank one quarter turn CLOCKWISE from south: a right
	# corner, drawn spun 90 degrees. This is the case the old east-only
	# fallback used to flatten to a plain tile.
	main._issue_world_command({"type": "rotate", "tile": [0, 0]})
	var turned: Dictionary = main._tile_instance_at(Vector2i(0, 0))
	if turned.is_empty() or turned["nodes"].is_empty():
		_fail("the rotated belt vanished from the scene")
		return
	var right: Array = main._belt_frames_for_shape(main.BELT_SHAPE_CORNER_RIGHT)
	if turned["asset"] != right[0] or turned["frames"] != right:
		_fail("the belt fed from its flank is drawn as '%s', expected the right corner '%s'" % [turned["asset"], right[0]])
		return
	if absf(rad_to_deg(float(turned["yaw"])) - 90.0) > 0.5:
		_fail("a south-facing corner should be turned 90 degrees like any south-facing tile, got %.1f" % rad_to_deg(float(turned["yaw"])))
		return
	if not main._hover_text_at_tile(Vector2i(0, 0)).contains("turns right"):
		_fail("the rotated belt's tooltip should say it turns right, got: %s" % main._hover_text_at_tile(Vector2i(0, 0)).replace("\n", " / "))
		return

	# --- the machines reach the tile edges their belts meet ---------------
	# Not "is there an opening" (only the voxel-level tests can tell a carved
	# hole from a dark rectangle) but "does the model reach the tile edges the
	# belt meets", which is what the seam fix and the housings both had to be
	# true of.
	for placement in main._placements:
		# `machine`, not `recipe`: an unfed machine is undecided and its
		# recipe is null like a belt's.
		if not bool(placement.get("machine", false)):
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

	print("CONNECTIONS_OK: the retired belt names no longer import and the four straight frames do, a belt into a machine or a dock is the plain family and says so in words, turning a fed belt re-shapes it into a corner at the new facing, and every machine reaches its footprint's full width")
	quit(0)
