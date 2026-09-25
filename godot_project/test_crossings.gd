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
#   * removing a crossing takes both instances away.
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


func _run_checks() -> void:
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
	if not String(raised.name).begins_with(String(main.PIPE_FLUID_ASSETS[main.CROSSING_GANTRY_ASSET])):
		_fail("the crossing's liquid body is '%s', not the gantry's own fluid model" % raised.name)
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
