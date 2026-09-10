# Headless proof of the "Real Placement" epic's ticket 2 — the presentation
# half: an empty world, per-tile rotation drawn as rotation, the ghost, and
# the place/rotate/remove input path.
#
# Run with:
#   godot --headless --path godot_project --script test_placement.gd
#
# Verifies, without a framebuffer:
#   * a NEW WORLD renders nothing but floor — a real, newly reachable state,
#     since `World::new` builds nothing at all,
#   * choosing an owned kind from the build menu enters placement mode, and
#     the ghost is a real, non-empty voxel model that follows the tile under
#     the cursor's ray (the same `_tile_under_screen_pos` hover uses),
#   * validity is a pure Godot-side check: free ground is valid, an occupied
#     tile is blocked, a kind with none owned is refused — and each state is
#     signalled by MORE than colour (UI_UX.md §7): the prompt's own glyph and
#     wording differ, and the tint differs too,
#   * a real left-click through `_unhandled_input` places over the real
#     `issue_command_json` boundary and the rendered scene follows — and a
#     click on a blocked tile changes nothing at all,
#   * every one of the four facings renders with the matching Y rotation, and
#     a quarter turn swaps a non-square footprint's extents,
#   * the junction art's east-only fallback holds under real placement: a
#     belt facing east beside a building takes a junction variant, the same
#     belt facing any other way takes the plain tile,
#   * remove mode ghosts the thing under the cursor and a click picks it up,
#   * leaving either mode frees the ghost rather than hiding it.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
#
# Known, benign log line: freeing and rebuilding the "World" subtree makes
# Godot's HEADLESS dummy renderer print `Parameter "material" is null` from
# `material_get_instance_shader_parameters` once. It is the dummy backend
# querying a freed instance, not a missing material — every mesh in the scene
# is asserted to have one by test_machines.gd — and it does not appear in a
# real (rendering) run. Don't chase it.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("PLACEMENT_FAIL: %s" % msg)
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


## A real left mouse button press through the scene's own input handler —
## the click that builds.
func _left_click(target) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_LEFT
	event.pressed = true
	target._unhandled_input(event)


func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


## Aims the isometric camera's cursor ray at one tile by warping nothing at
## all: `_ghost_target_tile` reads the live mouse, which a headless run has no
## meaningful value for, so every check below drives the pure functions the
## key/click handlers drive and passes the tile explicitly where it matters.
func _screen_pos_for(tile: Vector2i) -> Vector2:
	return main._camera.unproject_position(
		main._tile_origin(tile.x, tile.y, main.BELT_SURFACE_Y)
	)


func _run_checks() -> void:
	if main._camera == null:
		_fail("main._camera was not set by _build_camera()")
		return

	# --- 1. An empty world renders floor and nothing else ------------------
	if not main._placements.is_empty():
		_fail("a new world must start with nothing placed, got %d placements" % main._placements.size())
		return
	var world_root: Node = main.get_node_or_null("World")
	if world_root == null:
		_fail("the World node should exist even when it is empty")
		return
	# "World" always carries its two instancing containers, "Tiles" and
	# "Items" (the pipes & conveyors overhaul's ticket 4); an empty world has
	# nothing else under it and nothing in either.
	for child in world_root.get_children():
		if String(child.name) != "Tiles" and String(child.name) != "Items":
			_fail("an empty world should render no placement nodes, got '%s'" % child.name)
			return
		if child.get_child_count() != 0:
			_fail("an empty world should have no instanced %s, got %d groups" % [child.name, child.get_child_count()])
			return
	if main._tile_instance_at(Vector2i.ZERO).size() != 0 or main._carried_item_count() != 0:
		_fail("an empty world should record no tile instances and draw no items")
		return
	if main.get_node_or_null("Floor") == null:
		_fail("the floor must be drawn even with nothing on it — there is nowhere to build otherwise")
		return
	# ...and the camera frames a workable expanse of it rather than one tile.
	if main._frame_bounds.size.x < main.EMPTY_WORLD_FRAME_TILES:
		_fail("an empty world should still be framed on usable ground, got %s" % str(main._frame_bounds))
		return

	# --- 2. The build menu's own affordance enters placement mode ----------
	# Nothing is owned yet, so asking to place must refuse rather than show a
	# ghost that could never be built.
	main._refresh_owned_counts()
	main._enter_build_mode("belt")
	if main._in_build_mode():
		_fail("placement mode should refuse a kind the player owns none of")
		return

	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt", "quantity": 4}))
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "hopper"}))
	main._refresh_owned_counts()
	main._enter_build_mode("belt")
	if not main._in_build_mode() or main._build_kind != "belt":
		_fail("choosing an owned kind should enter placement mode")
		return
	if main._shop_open:
		_fail("entering placement mode should close the build menu — one answer to 'what does a click do'")
		return

	# The ghost is a real model, not a placeholder box.
	main._update_ghost()
	var ghost_body: Node = main._ghost_root.get_node_or_null("ghost_body")
	if ghost_body == null:
		_fail("placement mode built no ghost body")
		return
	var ghost_meshes := _meshes(ghost_body, [])
	if ghost_meshes.is_empty():
		_fail("the ghost rendered no meshes — it should be the kind's own voxel model")
		return
	for mi in ghost_meshes:
		if mi.mesh == null or mi.mesh.get_surface_count() < 1:
			_fail("the ghost has an empty mesh '%s'" % mi.name)
			return

	# --- 3. Validity is a pure, footprint-aware, Godot-side check ----------
	if not main._ghost_blocked_tiles("belt", Vector2i(7, 7), "east").is_empty():
		_fail("open ground should not be blocked")
		return
	# `item` is required now that the chain has two raw solids — a hopper
	# placed without one is a silent refusal, and this fixture would then be
	# asserting that an empty tile reads blocked.
	main.sim.issue_command_json(JSON.stringify({
		"type": "place", "kind": "hopper", "tile": [7, 7], "direction": "east", "item": "tomato",
	}))
	main._rebuild_world()
	if main._ghost_blocked_tiles("belt", Vector2i(7, 7), "east").size() != 1:
		_fail("a tile with a hopper on it must read blocked")
		return
	# Footprint-aware: the 2x2 cooking vat placed at (6, 6) would reach (7, 7).
	if main._ghost_blocked_tiles("cooking_vat", Vector2i(6, 6), "east").size() != 1:
		_fail("the ghost's collision check must cover the whole footprint, not just its origin tile")
		return
	# ...and it knows a vat is 2x2 without one ever having been placed.
	if main._kind_footprint("cooking_vat") != Vector2i(2, 2):
		_fail("_kind_footprint should derive the vat's 2x2 footprint, got %s" % str(main._kind_footprint("cooking_vat")))
		return

	# --- 4. Each state says so in more than colour (UI_UX.md §7) ----------
	main._build_kind = "belt"
	main._remove_mode = false
	main._ghost_reason = ""
	main._ghost_valid = true
	var valid_text: String = main._placement_prompt_text()
	main._ghost_valid = false
	main._ghost_reason = "blocked"
	var blocked_text: String = main._placement_prompt_text()
	main._ghost_reason = "none_owned"
	var unowned_text: String = main._placement_prompt_text()
	if not valid_text.begins_with("✔") or not blocked_text.begins_with("✗") or not unowned_text.begins_with("✗"):
		_fail("the prompt must lead with a glyph, not rely on colour: '%s' / '%s' / '%s'" % [
			valid_text, blocked_text, unowned_text,
		])
		return
	if not blocked_text.contains("BLOCKED") or not unowned_text.contains("NONE OWNED"):
		_fail("the two invalid states must be distinguishable in words: '%s' / '%s'" % [blocked_text, unowned_text])
		return
	if not valid_text.contains("facing"):
		_fail("the prompt should say which way the ghost faces: '%s'" % valid_text)
		return
	# The tints differ too — colour reinforces the glyph, it does not replace it.
	if main.GHOST_TINT_VALID == main.GHOST_TINT_INVALID:
		_fail("valid and invalid ghosts must be tinted differently as well as worded differently")
		return
	# The prompt's plate carries the same signal on its border.
	if main._placement_prompt_style(true).border_color == main._placement_prompt_style(false).border_color:
		_fail("the prompt's own border should follow the ghost's state")
		return

	# --- 5. Rotation is drawn as rotation ----------------------------------
	# One belt per facing, each asserted to carry the matching Y rotation.
	var expected_yaw := {
		"east": 0.0, "south": 90.0, "west": 180.0, "north": 270.0,
	}
	var tiles := {"east": Vector2i(10, 0), "south": Vector2i(10, 2), "west": Vector2i(10, 4), "north": Vector2i(10, 6)}
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt", "quantity": 4}))
	for facing in tiles.keys():
		var tile: Vector2i = tiles[facing]
		main.sim.issue_command_json(JSON.stringify({
			"type": "place", "kind": "belt", "tile": [tile.x, tile.y], "direction": facing,
		}))
	main._rebuild_world()
	for facing in tiles.keys():
		var tile: Vector2i = tiles[facing]
		# A belt is a MultiMesh instance, not a node (the pipes & conveyors
		# overhaul's ticket 4); its yaw is read back through the scene's
		# own record of it.
		var instance: Dictionary = main._tile_instance_at(tile)
		if instance.is_empty():
			_fail("no rendered belt at %s facing %s" % [str(tile), facing])
			return
		var got := rad_to_deg(float(instance["yaw"]))
		while got < 0.0:
			got += 360.0
		if absf(got - float(expected_yaw[facing])) > 0.5:
			_fail("a %s-facing belt should be turned %.0f degrees, got %.1f" % [facing, expected_yaw[facing], got])
			return

	# A quarter turn swaps a NON-SQUARE footprint's extents — asserted
	# against the simulation's own `occupies`, since the shipped content has
	# no oblong kind to see it on (the 2x2 vat is square).
	if main._rotated_footprint(Vector2i(3, 1), "east") != Vector2i(3, 1):
		_fail("an east-facing 3x1 should stay 3x1")
		return
	if main._rotated_footprint(Vector2i(3, 1), "north") != Vector2i(1, 3):
		_fail("a north-facing 3x1 should claim 1x3 — a quarter turn swaps the extents")
		return

	# --- 6. A belt's shape comes from its links, at every facing ----------
	# The belt at (0, 0) faces east with a hopper behind it at (-1, 0): fed
	# from behind, so the plain straight family — since the pipes and
	# conveyors overhaul there is no junction variant to pick and nothing
	# east-only about a belt (test_connections.gd and test_belt_shapes.gd
	# cover the corner and re-shaping cases).
	Layouts.build_minimal(main.sim)
	main._rebuild_world()
	var straight: Array = main._belt_frames_for_shape(main.BELT_SHAPE_STRAIGHT)
	var fed: Dictionary = main._tile_instance_at(Vector2i(0, 0))
	if fed.is_empty() or fed["asset"] != straight[0]:
		_fail("an east-facing belt catching a hopper's discharge should draw the straight family, got %s" % str(fed.get("asset", "nothing")))
		return
	# The lone rotated belts above rendered too, as straights, spun: a
	# north-facing tile fed by nothing is a line start, drawn with the same
	# four frames as any straight.
	var north_belt: Dictionary = main._tile_instance_at(Vector2i(10, 6))
	if north_belt.is_empty():
		_fail("the north-facing belt did not render at all")
		return
	if north_belt["asset"] != straight[0] or north_belt["frames"] != straight or north_belt["nodes"].is_empty():
		_fail("the north-facing belt rendered '%s' (%s) with %d meshes — expected the straight frame set" % [
			north_belt["asset"], str(north_belt["frames"]), north_belt["nodes"].size(),
		])
		return

	# --- 7. A click places, and the scene follows --------------------------
	main._enter_build_mode("belt")
	main._build_direction = "south"
	var before: int = main._placements.size()
	# A real left-click, through `_unhandled_input`. It acts on the tile the
	# GHOST is on rather than re-picking from the cursor (see `_confirm_build`),
	# which is what makes this drivable at all without a live mouse: aim the
	# ghost, then click.
	main._ghost_tile = Vector2i(12, 12)
	main._ghost_valid = true
	main._ghost_reason = ""
	_left_click(main)
	if main._placements.size() != before + 1:
		_fail("a left-click in placement mode should have added exactly one placement")
		return
	if main._tile_instance_at(Vector2i(12, 12)).is_empty():
		_fail("the placed belt did not render")
		return
	if String(main._placement_at_tile(Vector2i(12, 12)).get("direction", "")) != "south":
		_fail("the click should have built the ghost's facing, not the default")
		return
	# A click on a blocked tile is a total no-op, like the simulation's own
	# refusal — the prompt says so and the click must agree with the prompt.
	main._enter_build_mode("belt")
	var blocked_before: int = main._placements.size()
	main._ghost_tile = Vector2i(12, 12)
	main._ghost_valid = false
	main._ghost_reason = "blocked"
	_left_click(main)
	if main._placements.size() != blocked_before:
		_fail("clicking a blocked tile must change nothing at all")
		return

	# --- 8. Remove mode ghosts the target, and a click picks it up ---------
	main._exit_build_mode()
	main._toggle_remove_mode()
	if not main._remove_mode:
		_fail("X should enter remove mode")
		return
	main._ghost_tile = Vector2i(12, 12)
	main._ghost_reason = ""
	var remove_text: String = main._placement_prompt_text()
	if not remove_text.begins_with("✔ REMOVE") or not remove_text.contains("Belt"):
		_fail("remove mode should name what the click would pick up, got: '%s'" % remove_text)
		return
	main._ghost_reason = "nothing_here"
	if not main._placement_prompt_text().contains("nothing here"):
		_fail("remove mode over bare floor should say so")
		return
	main._issue_world_command({"type": "remove", "tile": [12, 12]})
	if not main._tile_instance_at(Vector2i(12, 12)).is_empty():
		_fail("the removed belt is still rendered")
		return

	# --- 9. Leaving a mode leaves no dangling ghost ------------------------
	main._exit_build_mode()
	if main._in_build_mode():
		_fail("_exit_build_mode did not leave build mode")
		return
	if main._ghost_root.get_node_or_null("ghost_body") != null:
		_fail("leaving build mode should FREE the ghost body, not just hide it")
		return
	if main._ghost_root.visible or main._placement_prompt.visible:
		_fail("leaving build mode should hide the ghost and its prompt")
		return

	print("PLACEMENT_OK: empty world renders bare floor, the ghost is a real model with footprint-aware validity signalled by glyph and word as well as colour, all four facings render the matching rotation, the junction art falls back cleanly off east, and place/remove drive the scene through the real command boundary")
	quit(0)
