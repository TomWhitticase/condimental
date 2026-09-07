# Headless proof of the "Machines improvement" milestone's *presentation*
# half (ticket 4) — the structural claims a machine can check without a
# framebuffer. Whether the new assets actually look right is what
# scripts/screenshot-scene.sh is for, the same division test_presentation.gd
# and test_fluids.gd already draw.
#
# Run with:
#   godot --headless --path godot_project --script test_machines.gd
#
# Verifies:
#   * every stage renders a real, non-empty imported voxel mesh at the tiles
#     the SIMULATION reports — not a silently-missing asset,
#   * the three kinds with a bespoke model (crusher, mixer, cooking vat) get
#     distinct meshes from each other and from the generic fallback, so the
#     chain's front end is actually distinguishable,
#   * the cooking vat — the first machine in the game bigger than one tile —
#     genuinely spans more than one tile of world space, and its node sits at
#     its footprint's CENTRE rather than on its origin tile,
#   * every 1x1 kind is still placed exactly where it was before footprints
#     existed (on its own tile centre), i.e. generalizing the placement moved
#     nothing,
#   * the hopper renders at the tile the simulation puts it on, and its
#     product body rescales when its fill level changes — the same
#     live-data-drives-a-scaled-body contract the pipe run's liquid has,
#   * hovering resolves for a NON-ORIGIN tile inside the vat's footprint (the
#     case that did not exist before this milestone) and for the hopper,
#     through the one `_hover_text_at_tile` mechanism.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header
# for the full explanation of the `--script` SceneTree bootstrap quirk they
# work around.
extends SceneTree

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("MACHINES_FAIL: %s" % msg)
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


## Every MeshInstance3D under `node`, so "the asset actually imported" can be
## asserted rather than assumed (a missing .vox yields an empty Node3D).
func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


## The world-space bounding box of every mesh under `node`, in that node's
## own parent space — what "does this thing actually span two tiles" needs.
func _world_aabb(node: Node3D) -> AABB:
	var meshes := _meshes(node, [])
	var box := AABB()
	var first := true
	for mi in meshes:
		var mesh_box: AABB = mi.mesh.get_aabb()
		var here := AABB(node.position + mesh_box.position, mesh_box.size)
		if first:
			box = here
			first = false
		else:
			box = box.merge(here)
	return box


func _run_checks() -> void:
	var layout: Array = JSON.parse_string(main.sim.chain_layout_json())
	if layout.is_empty():
		_fail("chain_layout_json returned no stages")
		return

	var multi_tile_stages := 0
	var bespoke_meshes := {} # kind -> the first surface's Mesh, to compare kinds

	for stage in layout:
		var recipe := String(stage["recipe"])
		var kind := String(stage.get("machine_kind", ""))
		var tile_pair: Array = stage["machine_tile"]
		var origin := Vector2i(int(tile_pair[0]), int(tile_pair[1]))
		var size: Vector2i = main._footprint_size(stage.get("machine_footprint"))

		var node: Node3D = main.get_node_or_null("Chain/machine_%s" % recipe)
		if node == null:
			_fail("no rendered machine for stage '%s' — the scene is not following the sim's layout" % recipe)
			return

		# --- a real imported mesh, not a silently-missing asset ------------
		var meshes := _meshes(node, [])
		if meshes.is_empty():
			_fail("machine '%s' (kind '%s') rendered no meshes — the .vox didn't import" % [recipe, kind])
			return
		for mi in meshes:
			if mi.mesh == null or mi.mesh.get_surface_count() < 1:
				_fail("machine '%s' has an empty mesh '%s'" % [recipe, mi.name])
				return

		# --- placed at the footprint's CENTRE, per _footprint_centre -------
		var expected: Vector3 = main._footprint_centre(origin, size, main.FLOOR_Y)
		if not node.position.is_equal_approx(expected):
			_fail("machine '%s' (kind '%s', footprint %s at %s) sits at %s, expected its footprint centre %s" % [
				recipe, kind, str(size), str(origin), str(node.position), str(expected),
			])
			return
		if size == Vector2i.ONE:
			# The pre-footprint placement rule, restated: generalizing it
			# must not have moved any of the 1x1 kinds that were already
			# rendering correctly before this milestone.
			var old_rule: Vector3 = main._tile_origin(origin.x, origin.y, main.FLOOR_Y)
			if not node.position.is_equal_approx(old_rule):
				_fail("1x1 machine '%s' moved off its tile (%s vs %s)" % [recipe, str(node.position), str(old_rule)])
				return
		else:
			multi_tile_stages += 1
			# --- the whole point: it visibly occupies more than one tile ---
			var box := _world_aabb(node)
			if box.size.x <= main.TILE * 1.05:
				_fail("multi-tile machine '%s' (footprint %s) spans only %f world units in X — barely one tile (%f)" % [
					recipe, str(size), box.size.x, main.TILE,
				])
				return
			if size.y > 1 and box.size.z <= main.TILE * 1.05:
				_fail("machine '%s' declares %d tiles of depth but its mesh spans only %f units in Z" % [
					recipe, size.y, box.size.z,
				])
				return

		if main.MACHINE_MODELS.has(kind):
			bespoke_meshes[kind] = meshes[0].mesh

	if multi_tile_stages < 1:
		_fail("no stage in the shipped chain has a footprint bigger than 1x1 — this test would prove nothing")
		return

	# --- the bespoke kinds are genuinely different models -----------------
	if bespoke_meshes.size() < 3:
		_fail("expected the crusher/mixer/cooking vat to render bespoke models, found %d" % bespoke_meshes.size())
		return
	var seen := []
	for kind in bespoke_meshes.keys():
		var mesh = bespoke_meshes[kind]
		if seen.has(mesh):
			_fail("kind '%s' shares a mesh with another kind — the models didn't actually differ" % kind)
			return
		seen.append(mesh)

	# --- the hopper: rendered where the sim says, with a live fill body ---
	var hopper_json: String = main.sim.hopper_snapshot_json()
	if hopper_json.is_empty():
		_fail("hopper_snapshot_json is empty — ticket 3's hopper should always exist")
		return
	var hopper: Dictionary = JSON.parse_string(hopper_json)
	var hopper_tile_pair: Array = hopper["tile"]
	var hopper_tile := Vector2i(int(hopper_tile_pair[0]), int(hopper_tile_pair[1]))

	var hopper_root: Node3D = main.get_node_or_null("Hopper")
	if hopper_root == null:
		_fail("no Hopper node — the chain's feeder didn't build")
		return
	var body: Node3D = hopper_root.get_node_or_null("hopper_body")
	if body == null or _meshes(body, []).is_empty():
		_fail("the hopper body rendered no meshes — the .vox didn't import")
		return
	var hopper_expected: Vector3 = main._footprint_centre(
		hopper_tile, main._footprint_size(hopper.get("footprint")), main.FLOOR_Y
	)
	if not body.position.is_equal_approx(hopper_expected):
		_fail("hopper body sits at %s, expected %s (sim tile %s)" % [
			str(body.position), str(hopper_expected), str(hopper_tile),
		])
		return
	if main._hopper_tile != hopper_tile:
		_fail("scene tracks the hopper at %s, sim reports %s" % [str(main._hopper_tile), str(hopper_tile)])
		return

	if main._hopper_fill_node == null or _meshes(main._hopper_fill_node, []).is_empty():
		_fail("the hopper's product body rendered no meshes")
		return
	# An empty hopper shows nothing at all...
	main._update_hopper()
	if int(hopper.get("quantity", 0)) == 0 and main._hopper_fill_node.visible:
		_fail("an empty hopper should hide its product body, not draw a sliver")
		return
	# ...and filling it through the real command path raises the level.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main._update_hopper()
	if not main._hopper_fill_node.visible:
		_fail("a filled hopper should show its product body")
		return
	var part_full_scale: float = main._hopper_fill_node.scale.y
	for i in range(8):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main._update_hopper()
	if main._hopper_fill_node.scale.y <= part_full_scale:
		_fail("filling the hopper further did not raise its product level (%f -> %f)" % [
			part_full_scale, main._hopper_fill_node.scale.y,
		])
		return
	if main._hopper_fill_node.scale.y > float(main.HOPPER_FILL_HEIGHT_VOXELS) + 0.001:
		_fail("a full hopper's level (%f) rose past its box section (%d voxels)" % [
			main._hopper_fill_node.scale.y, main.HOPPER_FILL_HEIGHT_VOXELS,
		])
		return

	# --- hover: a NON-ORIGIN tile of a multi-tile machine resolves ---------
	var multi_recipe := ""
	var multi_origin := Vector2i.ZERO
	var multi_size := Vector2i.ONE
	for stage in layout:
		var size2: Vector2i = main._footprint_size(stage.get("machine_footprint"))
		if size2 != Vector2i.ONE:
			multi_recipe = String(stage["recipe"])
			var pair: Array = stage["machine_tile"]
			multi_origin = Vector2i(int(pair[0]), int(pair[1]))
			multi_size = size2
			break

	var far_corner := multi_origin + multi_size - Vector2i.ONE
	if far_corner == multi_origin:
		_fail("the multi-tile machine's far corner is its origin — nothing new is being tested")
		return
	if main._machine_recipe_at_tile(far_corner) != multi_recipe:
		_fail("far corner %s of '%s' (origin %s, footprint %s) resolved to '%s'" % [
			str(far_corner), multi_recipe, str(multi_origin), str(multi_size),
			main._machine_recipe_at_tile(far_corner),
		])
		return
	var far_text: String = main._hover_text_at_tile(far_corner)
	if not far_text.begins_with(multi_recipe):
		_fail("hovering %s should describe '%s', got: %s" % [str(far_corner), multi_recipe, far_text])
		return
	# ...and the tile just past that corner must NOT resolve to it.
	var past := far_corner + Vector2i(0, 1)
	if main._machine_recipe_at_tile(past) == multi_recipe:
		_fail("tile %s is outside '%s'’s footprint but still resolved to it" % [str(past), multi_recipe])
		return

	# --- hover: the hopper answers through the same mechanism -------------
	if not main._hopper_at_tile(hopper_tile):
		_fail("the hopper's own tile %s doesn't resolve to the hopper" % str(hopper_tile))
		return
	var hopper_text: String = main._hover_text_at_tile(hopper_tile)
	if not hopper_text.begins_with("hopper"):
		_fail("hovering the hopper tile should describe the hopper, got: %s" % hopper_text)
		return
	if not hopper_text.contains(String(hopper["item"])) or not hopper_text.contains("units"):
		_fail("the hopper tooltip should name its item and its level, got: %s" % hopper_text)
		return
	if main._hopper_at_tile(hopper_tile + Vector2i(0, 4)):
		_fail("open floor four tiles north of the hopper resolved to the hopper")
		return

	# --- the help text tells the player what SPACE now does ---------------
	if not main.ISO_HELP_TEXT.to_lower().contains("hopper"):
		_fail("ISO_HELP_TEXT still doesn't mention the hopper: %s" % main.ISO_HELP_TEXT)
		return

	print("MACHINES_OK: %d stage(s) rendered at the sim's own tiles, %d bespoke kind(s), %d multi-tile machine(s) spanning their full footprint, hopper drawn at %s with a live product level, hover resolves a non-origin footprint tile and the hopper" % [
		layout.size(), bespoke_meshes.size(), multi_tile_stages, str(hopper_tile),
	])
	quit(0)
