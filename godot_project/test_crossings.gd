# Headless proof of the belt/pipe crossing's presentation (the crossing epic,
# ticket T3, `docs/BELT_PIPE_CROSSING_PLAN.md`) — the structural claims a
# machine can check without a framebuffer. Whether the gantry actually READS
# as a pipe a line runs under is `scripts/screenshot-scene.sh --fixture
# crossing`'s job, the same division test_splitters.gd draws.
#
# Run with:
#   godot --headless --path godot_project --script test_crossings.gd
#
# Verifies, on test_layouts.gd's crossing line (both crossings assembled by
# the simulation from real purchase/place commands, one each way):
#   * a crossing draws BOTH halves as instances — the straight belt frames
#     under the tile's own key and the gantry under its own — and NOT the
#     generic `machine` housing an unknown kind falls back to,
#   * the gantry takes the belt's yaw, so it lies across the lane at every
#     facing, including after a Rotate,
#   * items on a crossing are drawn at the belt's running height, UNDER the
#     arch, off the same `belts` entry a plain belt's come from,
#   * the liquid body stands `CROSSING_LIFT_VOXELS` above an ordinary run's,
#     in the gantry's own fluid model, and a fault clamp is lifted the same,
#   * hovering a crossing names both halves, in display names, and gives it
#     the pipe's level meter,
#   * the remove ghost is the whole stack, belt and gantry,
#   * the BUILD ghost teaches the rule with the sim's answer, both ways
#     round: a pipe held across a belt (or a belt across a pipe) is a valid
#     ghost drawn as the crossing it will build, named "Pipe over Belt" /
#     "Belt under Pipe"; held along it, the same tile is refused with the
#     sim's own sentence — and nothing here or in main.gd says which facing
#     is "across",
#   * removing a crossing takes both instances away,
#   * `CROSSING_LIFT_VOXELS` is what the staged models say it is: the floor of
#     the gantry's bore stands exactly that far above an ordinary pipe's, read
#     from the `.vox` files `cargo xtask` staged. main.gd mirrors assets.rs's
#     `CROSSING_LIFT` by hand, and this is what turns a moved `GANTRY_AXIS_Z`
#     red instead of drawing the liquid in the gantry's wall.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("CROSSINGS_FAIL: %s" % msg)
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


## Ticks until `predicate` holds or the budget runs out, applying dirty
## chunks and polling the fluid each tick exactly as `_process` does.
func _tick_until(budget: int, predicate: Callable) -> bool:
	for _i in range(budget):
		main.sim.tick()
		main._apply_dirty_chunks()
		if predicate.call():
			return true
	return false


## The live liquid body standing on `tile`, or null.
func _fluid_node_at(tile: Vector2i) -> Node3D:
	var suffix: String = "_" + main._tile_key(tile)
	for node in main._pipe_fluid_nodes.values():
		if node != null and String(node.name).ends_with(suffix):
			return node
	return null


## The voxels of a staged model as a set of `Vector3i`, read straight out of
## the MagicaVoxel file — every material class `voxgen` split it into, since
## a matte voxel and a metal one are both the model. Only SIZE/XYZI are read;
## every other chunk is skipped by its own length.
func _vox_voxels(asset: String) -> Dictionary:
	var voxels: Dictionary = {}
	for path in [main.VOX_DIR + asset + ".vox", main.VOX_DIR + asset + ".metal.vox"]:
		var file := FileAccess.open(path, FileAccess.READ)
		if file == null:
			continue
		file.seek(8) # "VOX " + version
		while file.get_position() < file.get_length():
			var id := file.get_buffer(4).get_string_from_ascii()
			var content := file.get_32()
			var children := file.get_32()
			if id == "MAIN":
				continue
			var next := file.get_position() + content + children
			if id == "XYZI":
				for _i in range(file.get_32()):
					var bytes := file.get_buffer(4)
					voxels[Vector3i(bytes[0], bytes[1], bytes[2])] = true
			file.seek(next)
	return voxels


## The floor of the bore at the model's centre column: one above the highest
## solid voxel there, since a pipe's bore is carved open to the sky. The same
## height `PIPE_BORE_FLOOR_VOXELS` names for an ordinary run.
func _bore_floor(voxels: Dictionary) -> int:
	var centre: int = int(main.VOXELS_PER_TILE) / 2 - 1
	var top := -1
	for voxel in voxels.keys():
		if voxel.x == centre and voxel.y == centre:
			top = maxi(top, voxel.z)
	return top + 1


## Holds `kind` over `tile` at `facing` and reads back what the ghost became.
## "" when it is what the case expects. The expected half is only ever the
## outcome — valid or not, and the words — never which facings are legal.
func _ghost_case(kind: String, tile: Vector2i, facing: String, valid: bool, name: String) -> String:
	main._enter_build_mode(kind)
	main._build_direction = facing
	main._pointer_override = main._camera.unproject_position(
		main._tile_origin(tile.x, tile.y, main.BELT_SURFACE_Y)
	)
	main._update_ghost()
	if main._ghost_tile != tile:
		return "the ghost landed on %s — the aim missed" % str(main._ghost_tile)
	var prompt: String = main._placement_prompt_text()
	var preview: Dictionary = main._preview_place(kind, tile, facing)
	if main._ghost_valid != valid or bool(preview.get("valid", not valid)) != valid:
		return "the ghost reads %s and the sim says %s, expected %s" % [
			"valid" if main._ghost_valid else "refused", str(preview), "valid" if valid else "refused"
		]
	if valid:
		var belt_facing: String = facing if kind == "belt" else "east"
		if main._ghost_asset != main._crossing_ghost_asset(belt_facing):
			return "the valid ghost is '%s', not the crossing stack" % main._ghost_asset
		if not is_equal_approx(main._ghost_root.rotation.y, main._direction_yaw(belt_facing)):
			return "the crossing ghost is not at the belt's yaw"
		if not prompt.begins_with("✔") or not prompt.contains(name):
			return "the prompt does not say '%s': %s" % [name, prompt.replace("\n", " / ")]
	else:
		var reason := String(preview.get("reason", ""))
		if reason == "" or not prompt.begins_with("✗") or not prompt.contains(reason):
			return "the refused prompt does not carry the sim's reason '%s': %s" % [
				reason, prompt.replace("\n", " / ")
			]
		if main._ghost_asset.contains(main.GHOST_ASSET_STACK):
			return "a refused ghost is drawn as a crossing"
	return ""


func _run_checks() -> void:
	# --- the lift main.gd mirrors is the lift the models have ---------------
	var plain_floor := _bore_floor(_vox_voxels(main.PIPE_ASSET))
	var raised_floor := _bore_floor(_vox_voxels(main.CROSSING_GANTRY_ASSET))
	if plain_floor <= 0 or raised_floor <= 0:
		_fail("could not read the bore of '%s' / '%s' from the staged .vox files — run cargo xtask"
			% [main.PIPE_ASSET, main.CROSSING_GANTRY_ASSET])
		return
	if raised_floor - plain_floor != main.CROSSING_LIFT_VOXELS:
		_fail("the gantry's bore floor is %d voxels above a pipe's (%d vs %d) but CROSSING_LIFT_VOXELS is %d — \
assets.rs's CROSSING_LIFT moved and main.gd's mirror did not"
			% [raised_floor - plain_floor, raised_floor, plain_floor, main.CROSSING_LIFT_VOXELS])
		return

	Layouts.build_crossing_line(main.sim)
	main._rebuild_world()

	var built := Layouts.crossing_line_failure(main.sim)
	if built != "":
		_fail("the fixture built no crossing: %s" % built)
		return

	var straight: Array = main._belt_frames_for_shape(main.BELT_SHAPE_STRAIGHT)
	var belt_yaw: float = main._direction_yaw("east")
	for x in [Layouts.CROSSING_PIPE_ONTO_BELT_X, Layouts.CROSSING_BELT_ONTO_PIPE_X]:
		var tile := Vector2i(x, 0)
		# --- both halves, as instances, and no machine housing ---------------
		var belt: Dictionary = main._tile_instance_at(tile)
		if belt.is_empty() or String(belt["asset"]) != String(straight[0]):
			_fail("the crossing at %s draws '%s' under its own key, expected the straight belt '%s'"
				% [str(tile), str(belt.get("asset", "nothing")), straight[0]])
			return
		var gantry: Dictionary = main._tile_instances.get(main._crossing_gantry_key(tile), {})
		if gantry.is_empty() or String(gantry["asset"]) != main.CROSSING_GANTRY_ASSET:
			_fail("the crossing at %s has no '%s' gantry instance (got '%s')"
				% [str(tile), main.CROSSING_GANTRY_ASSET, str(gantry.get("asset", "nothing"))])
			return
		if not is_equal_approx(float(gantry["yaw"]), belt_yaw):
			_fail("the gantry at %s is at yaw %.3f, not the belt's %.3f" % [str(tile), float(gantry["yaw"]), belt_yaw])
			return
		if main._world_root.get_node_or_null("%s_%s" % [main.CROSSING_KIND, main._tile_key(tile)]) != null:
			_fail("the crossing at %s was ALSO built as a node — the machine fallback path" % str(tile))
			return
		if main._asset_for_placement(main._placement_at_tile(tile)) == main.MACHINE_MODEL_FALLBACK:
			_fail("`_asset_for_placement` answers the machine fallback for a crossing")
			return

	# --- items pass UNDER the arch ------------------------------------------
	var under := Vector2i(Layouts.CROSSING_PIPE_ONTO_BELT_X, 0)
	var item_key := "belt_%d_%d_0" % [under.x, under.y]
	if not _tick_until(3000, func(): return not main._carried_item(item_key).is_empty()):
		_fail("no item was ever drawn on the crossing at %s in 3000 ticks" % str(under))
		return
	var item: Dictionary = main._carried_item(item_key)
	if not is_equal_approx(float(item["position"].y), main.BELT_SURFACE_Y):
		_fail("an item on the crossing stands at y %.3f, not the belt's running surface %.3f"
			% [float(item["position"].y), main.BELT_SURFACE_Y])
		return

	# --- the liquid rides the raised span -----------------------------------
	var ordinary := Vector2i(Layouts.CROSSING_PIPE_ONTO_BELT_X, 1)
	if not _tick_until(3000, func():
		main._update_fluid_network()
		return _fluid_node_at(under) != null and _fluid_node_at(ordinary) != null
	):
		_fail("no liquid body ever stood on the crossing at %s and the pipe beside it" % str(under))
		return
	var raised := _fluid_node_at(under)
	var plain := _fluid_node_at(ordinary)
	var gantry_fluid := String(main.PIPE_FLUID_ASSETS.get(main.CROSSING_GANTRY_ASSET, ""))
	if gantry_fluid == "" or not String(raised.name).begins_with(gantry_fluid):
		_fail("the crossing's liquid body is '%s', not the gantry's own fluid model ('%s' in PIPE_FLUID_ASSETS)"
			% [raised.name, gantry_fluid])
		return
	var lift: float = raised.position.y - plain.position.y
	if not is_equal_approx(lift, main.CROSSING_LIFT_Y):
		_fail("the crossing's liquid stands %.3f above the ordinary run's, expected CROSSING_LIFT_Y %.3f"
			% [lift, main.CROSSING_LIFT_Y])
		return

	# --- the fault clamp is lifted the same ---------------------------------
	# Fed a snapshot rather than a real dead run: which run the sim cannot name
	# is its own business (issue #54), and what is under test is only where
	# the clamp stands once it has said so.
	main._last_fluid = [{"naming": "unnameable", "segments": [
		{"tile": [under.x, under.y]}, {"tile": [ordinary.x, ordinary.y]},
	]}]
	main._update_pipe_fault_markers()
	var clamp_raised: Node3D = main._pipe_fault_nodes.get(main._tile_key(under))
	var clamp_plain: Node3D = main._pipe_fault_nodes.get(main._tile_key(ordinary))
	if clamp_raised == null or clamp_plain == null:
		_fail("the fault clamp did not stand on both the crossing and the pipe beside it")
		return
	if not is_equal_approx(clamp_raised.position.y - clamp_plain.position.y, main.CROSSING_LIFT_Y):
		_fail("the crossing's fault clamp is %.3f above the ordinary one's, expected %.3f"
			% [clamp_raised.position.y - clamp_plain.position.y, main.CROSSING_LIFT_Y])
		return
	main._update_fluid_network()

	# --- hover names both halves --------------------------------------------
	var hover: String = main._hover_text_at_tile(under)
	for part in [
		"%s over %s" % [main._kind_display_name("pipe"), main._kind_display_name("belt")],
		"%s  ·  carries east" % main._kind_display_name("belt"),
		"%s  ·  connects" % main._kind_display_name("pipe"),
		"remove returns both",
	]:
		if not hover.contains(part):
			_fail("the crossing's tooltip does not say '%s', got: %s" % [part, hover.replace("\n", " / ")])
			return
	if hover.contains("crossing"):
		_fail("the crossing's tooltip shows a wire id: %s" % hover.replace("\n", " / "))
		return
	if main._hover_panel_state(under).get("meter", {}).is_empty():
		_fail("hovering a crossing shows no level meter for its pipe")
		return

	# --- the remove ghost is the whole stack --------------------------------
	var stack: Node3D = main._ghost_asset_node(main._crossing_ghost_asset("east"))
	var belt_node: Node3D = main._asset_node(String(straight[0]))
	var gantry_node: Node3D = main._asset_node(main.CROSSING_GANTRY_ASSET)
	var stacked := _meshes(stack, []).size()
	var belt_only := _meshes(belt_node, []).size()
	var gantry_only := _meshes(gantry_node, []).size()
	for node in [stack, belt_node, gantry_node]:
		node.free()
	if stacked != belt_only + gantry_only or gantry_only == 0:
		_fail("the crossing's ghost holds %d meshes, expected the belt's %d plus the gantry's %d"
			% [stacked, belt_only, gantry_only])
		return

	# --- the build ghost, valid across and refused along --------------------
	main._set_view_mode(main.ViewMode.ISOMETRIC)
	for kind in ["pipe", "belt"]:
		main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": kind}))
	main._refresh_owned_counts()
	var run_tile := Vector2i(Layouts.CROSSING_PIPE_ONTO_BELT_X, 2)
	for case in [
		# held kind, tile, facing, valid, the name the prompt gives it
		["pipe", Layouts.CROSSING_GHOST_TILE, "north", true, "Pipe over Belt"],
		["pipe", Layouts.CROSSING_GHOST_TILE, "east", false, ""],
		["belt", run_tile, "east", true, "Belt under Pipe"],
		["belt", run_tile, "north", false, ""],
	]:
		var failure := _ghost_case(case[0], case[1], case[2], case[3], case[4])
		if failure != "":
			_fail("%s held over %s facing %s: %s" % [case[0], str(case[1]), case[2], failure])
			return
	main._exit_build_mode()

	# --- Rotate turns the whole assembly, gantry with it --------------------
	main._issue_world_command({"type": "rotate", "tile": [under.x, under.y]})
	var turned: Dictionary = main._tile_instances.get(main._crossing_gantry_key(under), {})
	var turned_belt: Dictionary = main._tile_instance_at(under)
	var facing: String = main._placement_direction(main._placement_at_tile(under))
	if facing == "east" or turned.is_empty() \
			or not is_equal_approx(float(turned["yaw"]), main._direction_yaw(facing)) \
			or not is_equal_approx(float(turned_belt["yaw"]), main._direction_yaw(facing)):
		_fail("after a Rotate the crossing faces '%s' and its gantry/belt yaws are %s/%s"
			% [facing, str(turned.get("yaw")), str(turned_belt.get("yaw"))])
		return

	# --- Remove takes both halves away --------------------------------------
	main._issue_world_command({"type": "remove", "tile": [under.x, under.y]})
	if not main._tile_instance_at(under).is_empty() \
			or main._tile_instances.has(main._crossing_gantry_key(under)):
		_fail("removing the crossing at %s left an instance behind" % str(under))
		return

	print("CROSSINGS_OK")
	quit(0)
