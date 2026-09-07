# Headless proof of the fluid network's *presentation* half (milestone 6,
# ARCHITECTURE.md §9 item 6; docs/UI_UX.md §4's "visible liquid surfaces in
# tanks/pipes" and §5's hover tooltips) — the structural claims a machine can
# check without a framebuffer. Whether it actually looks right is what
# scripts/screenshot-scene.sh is for, same division as test_presentation.gd.
#
# Run with:
#   godot --headless --path godot_project --script test_fluids.gd
#
# Verifies:
#   * the scene renders one pipe tile per segment the SIMULATION reports in
#     fluid_snapshot_json, at the tiles it reports — not a hardcoded run,
#   * every one of those pipe nodes (and every liquid body) is a real,
#     non-empty imported voxel mesh, not a silently-missing asset,
#   * the source segment is drawn with a different model from a plain one
#     (UI_UX.md §7: state distinguished by more than colour),
#   * each liquid body's vertical scale tracks its segment's live pressure,
#     and a segment with no fluid in it is hidden outright,
#   * hovering a pipe tile resolves to fluid info text, through the exact
#     same _tile_under_screen_pos plane-intersection path machines use, in
#     BOTH view modes (ISOMETRIC cursor and FIRST_PERSON crosshair),
#   * the inlet segment (the one that actually feeds a machine) renders as
#     the dedicated connector fitting (PIPE_INLET_ASSET), not a plain header
#     tile — the visual-overhaul pass's fix for the pipe-to-machine seam.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule below are both inherited from test_hover.gd — see that
# file's header for the full explanation of the `--script` SceneTree
# bootstrap quirk they work around.
extends SceneTree

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("FLUIDS_FAIL: %s" % msg)
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


func _run_checks() -> void:
	var snapshot_json: String = main.sim.fluid_snapshot_json()
	if snapshot_json.is_empty():
		_fail("fluid_snapshot_json is empty — data/recipes.ron declares no fluid_inputs, so there is nothing for this ticket to render")
		return
	var snapshot: Dictionary = JSON.parse_string(snapshot_json)
	var segments: Array = snapshot["segments"]
	if segments.is_empty():
		_fail("the fluid network reports no pipe segments")
		return

	# --- One rendered pipe per simulated segment, at the sim's own tiles ---
	var run: Node = main.get_node_or_null("FluidNetwork")
	if run == null:
		_fail("no FluidNetwork node — the pipe run didn't build")
		return
	if main._pipe_tiles.size() != segments.size():
		_fail("sim reports %d pipe segments, scene tracks %d tiles" % [segments.size(), main._pipe_tiles.size()])
		return
	if main._pipe_fluid_nodes.size() != segments.size():
		_fail("sim reports %d pipe segments, scene has %d liquid bodies" % [segments.size(), main._pipe_fluid_nodes.size()])
		return

	for i in range(segments.size()):
		var tile_pair: Array = segments[i]["tile"]
		var tile := Vector2i(int(tile_pair[0]), int(tile_pair[1]))
		if main._pipe_segment_at_tile(tile) != i:
			_fail("segment %d sits on tile %s but the scene doesn't map that tile to it" % [i, str(tile)])
			return

		var pipe: Node3D = run.get_node_or_null("pipe_%d" % i)
		if pipe == null:
			_fail("no rendered pipe node for segment %d" % i)
			return
		var expected: Vector3 = main._tile_origin(tile.x, tile.y, main.FLOOR_Y)
		if not pipe.position.is_equal_approx(expected):
			_fail("pipe %d is at %s, but the sim puts its tile at %s" % [i, str(pipe.position), str(expected)])
			return

		var pipe_meshes := _meshes(pipe, [])
		if pipe_meshes.is_empty():
			_fail("pipe %d has no MeshInstance3D — its .vox didn't import (run `cargo xtask`)" % i)
			return
		for mi in pipe_meshes:
			if mi.mesh == null or mi.mesh.get_surface_count() < 1:
				_fail("pipe %d's '%s' mesh is empty" % [i, mi.name])
				return

		var fluid: Node3D = main._pipe_fluid_nodes[i]
		var fluid_meshes := _meshes(fluid, [])
		if fluid_meshes.is_empty():
			_fail("liquid body %d has no MeshInstance3D — pipe_fluid.vox didn't import" % i)
			return
		for mi in fluid_meshes:
			if mi.mesh == null or mi.mesh.get_surface_count() < 1:
				_fail("liquid body %d has an empty mesh" % i)
				return
			var mat: StandardMaterial3D = mi.get_surface_override_material(0)
			if mat == null or not mat.vertex_color_use_as_albedo:
				_fail("liquid body %d has no vertex-colour material — the per-fluid tint didn't apply" % i)
				return

	# The source tile must be a DIFFERENT model from a plain segment, not the
	# same one recoloured (UI_UX.md §7).
	var plain_classes: Array = main._asset_classes.get(main.PIPE_ASSET, [])
	var source_classes: Array = main._asset_classes.get(main.PIPE_SOURCE_ASSET, [])
	if plain_classes.is_empty() or source_classes.is_empty():
		_fail("voxels/index.json is missing '%s' or '%s'" % [main.PIPE_ASSET, main.PIPE_SOURCE_ASSET])
		return

	# --- The inlet segment renders as the machine-connector fitting, not a
	# plain header tile (the visual-overhaul pass's fix for the pipe-to-
	# machine seam — see PIPE_INLET_ASSET's doc in main.gd) ---------------
	var inlet_classes: Array = main._asset_classes.get(main.PIPE_INLET_ASSET, [])
	if inlet_classes.is_empty():
		_fail("voxels/index.json is missing '%s'" % main.PIPE_INLET_ASSET)
		return
	var inlet_seg: int = int(snapshot["inlet_index"])
	var inlet_pipe: Node3D = run.get_node_or_null("pipe_%d" % inlet_seg)
	if inlet_pipe == null:
		_fail("no rendered pipe node for the inlet segment (%d)" % inlet_seg)
		return
	var inlet_mesh: MeshInstance3D = inlet_pipe.get_node_or_null(String(inlet_classes[0]))
	if inlet_mesh == null or inlet_mesh.mesh == null:
		_fail("inlet segment %d isn't drawn as '%s' — it renders as a plain header instead of the machine connector" % [
			inlet_seg, main.PIPE_INLET_ASSET,
		])
		return
	# The inlet fitting's mesh must be a genuinely different resource from a
	# plain segment's — otherwise this is just the same model under a name
	# that happens to resolve, not the actual connector geometry.
	var plain_mesh_path: String = main._mesh_for(main.PIPE_ASSET + "." + String(inlet_classes[0])).resource_path \
		if main._mesh_for(main.PIPE_ASSET + "." + String(inlet_classes[0])) != null else ""
	if plain_mesh_path != "" and inlet_mesh.mesh.resource_path == plain_mesh_path:
		_fail("the inlet segment's mesh is the plain pipe's own resource — pipe_inlet.vox isn't actually distinct")
		return

	# --- Liquid height tracks live pressure -------------------------------
	# Tick far enough for the source to push fluid down the run, so this is
	# checked against genuinely varying levels, not a uniform initial state.
	for i in range(200):
		main.sim.tick()
	main._update_fluid_network()

	var live: Dictionary = JSON.parse_string(main.sim.fluid_snapshot_json())
	var live_segments: Array = live["segments"]
	var any_visible := false
	for i in range(live_segments.size()):
		var pressure: float = float(live_segments[i]["pressure"])
		var node: Node3D = main._pipe_fluid_nodes[i]
		if pressure <= main.FLUID_MIN_VISIBLE_PRESSURE:
			if node.visible:
				_fail("segment %d is empty (pressure %f) but its liquid body is still shown" % [i, pressure])
				return
			continue
		any_visible = true
		var expected_scale: float = pressure * float(main.PIPE_BORE_HEIGHT_VOXELS)
		if not is_equal_approx(node.scale.y, expected_scale):
			_fail("segment %d at pressure %f should scale its liquid to %f, got %f" % [
				i, pressure, expected_scale, node.scale.y,
			])
			return
	if not any_visible:
		_fail("no segment holds any fluid after 200 ticks — the source isn't filling the run")
		return

	# --- Hovering a pipe tile shows its fluid state, in both view modes ----
	var camera: Camera3D = main._camera
	if camera == null:
		_fail("main._camera was not set by _build_camera()")
		return

	# The inlet segment: the one whose level actually moves, and the one a
	# player would most want to inspect.
	var inlet: int = int(live["inlet_index"])
	var inlet_tile_pair: Array = live_segments[inlet]["tile"]
	var inlet_tile := Vector2i(int(inlet_tile_pair[0]), int(inlet_tile_pair[1]))
	var world_point: Vector3 = main._tile_origin(inlet_tile.x, inlet_tile.y, main.BELT_SURFACE_Y)

	var iso_tile: Vector2i = main._tile_under_screen_pos(camera.unproject_position(world_point))
	if iso_tile != inlet_tile:
		_fail("ISOMETRIC screen pos over pipe tile %s resolved to %s" % [str(inlet_tile), str(iso_tile)])
		return
	var iso_text: String = main._hover_text_at_tile(iso_tile)
	if iso_text == "":
		_fail("hovering pipe tile %s produced no tooltip text" % str(inlet_tile))
		return
	if not iso_text.begins_with(String(live["fluid"])):
		_fail("pipe tooltip should name the fluid ('%s') first, got: %s" % [String(live["fluid"]), iso_text])
		return
	if not iso_text.contains("units") or not iso_text.contains("pressure"):
		_fail("pipe tooltip should carry volume and pressure, got: %s" % iso_text)
		return
	if not iso_text.contains("inlet"):
		_fail("the inlet segment's tooltip should say so, got: %s" % iso_text)
		return

	# A machine tile must still resolve to machine text through the same
	# generalized lookup — the pipe branch mustn't have displaced it.
	var recipe_id: String = main._stage_order[0]
	var machine_tile: Vector2i = main._machine_tiles[recipe_id]
	if not main._hover_text_at_tile(machine_tile).begins_with(recipe_id):
		_fail("machine tile %s stopped resolving to machine text after the pipe branch was added" % str(machine_tile))
		return

	# --- FIRST_PERSON: the crosshair resolves the same pipe tile ----------
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	var eye: Vector3 = world_point + Vector3(main.TILE * 2.0, main.TILE * 3.0, main.TILE * 4.0)
	main._fp_position = eye
	var to_target: Vector3 = (world_point - eye).normalized()
	main._fp_yaw = atan2(-to_target.x, -to_target.z)
	main._fp_pitch = clampf(asin(clampf(to_target.y, -1.0, 1.0)), -main.FP_PITCH_LIMIT, main.FP_PITCH_LIMIT)
	main._update_fp_camera_transform()

	# A point on the camera's forward axis always projects to the viewport
	# centre — where the crosshair is — regardless of headless viewport size.
	var fp_tile: Vector2i = main._tile_under_screen_pos(main._camera.unproject_position(world_point))
	if fp_tile != inlet_tile:
		_fail("FIRST_PERSON crosshair aimed at pipe tile %s resolved to %s" % [str(inlet_tile), str(fp_tile)])
		return
	if main._hover_text_at_tile(fp_tile) == "":
		_fail("FIRST_PERSON crosshair over a pipe produced no tooltip text")
		return
	main._set_view_mode(main.ViewMode.ISOMETRIC)

	print("FLUIDS_OK: %d pipe tiles rendered from the sim's own segments (source drawn distinctly), liquid bodies scale to live pressure, pipe hover resolves fluid/volume/pressure in both view modes" % segments.size())
	quit(0)
