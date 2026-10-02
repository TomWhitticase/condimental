# Headless proof of the pipe presentation (the pipes & conveyors overhaul's
# ticket 6, docs/PIPES_AND_CONVEYORS_PLAN.md) — the structural claims a
# machine can check without a framebuffer. Whether a joint actually READS is
# what `scripts/screenshot-scene.sh --pipe-gallery` is for, the same
# division every other presentation test here draws.
#
# Run with:
#   godot --headless --path godot_project --script test_pipe_shapes.gd
#
# Everything is built through real purchase/place commands
# (`test_layouts.gd`'s `build_pipe_gallery`), and every expectation below is
# read back through the wire's `links` as the scene consumes it. Verifies:
#   * EVERY SHAPE AT EVERY TURN: an end open on each of the four sides, a
#     straight on both axes, all four corner turns, all four tee turns and
#     the cross, each drawn as the right model at the right quarter-turn
#     count — from `links.connections` alone,
#   * a tile joined to NOTHING lies along the facing it was placed with,
#     the one case `direction` decides,
#   * FITTINGS: a tank discharging into a pipe from each of four sides and a
#     machine tapping a pipe from each of four sides each put the right
#     fitting on the right side, turned to it, as a rendered instance,
#   * the shipped middle of the line (tank -> vinegar run -> mixer -> vat ->
#     bottler) gets a corner at the tap, a fitting at every machine joint
#     and plain straights between,
#   * every pipe tile and every fitting is a real, non-empty imported mesh,
#   * the LIQUID BODY per shape: each tile's body is the body authored for
#     its shape, at the shape's turn, and it still follows live pressure,
#   * HOVER names the connections in words: the pipe sides, and every
#     machine or tank as "fed by" or "feeds",
#   * the GHOST of a pipe takes the shape its pipe neighbours would give it,
#   * the deprecated one-arrangement tiles (`pipe_source`, `pipe_inlet`) are
#     gone from the staged models.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("PIPE_SHAPES_FAIL: %s" % msg)
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


## Tile -> [shape asset, quarter turns], the whole gallery's expectation
## table. Turn counts are `DIRECTIONS` steps (east 0, south 1, west 2,
## north 3): an end is authored open WEST, a corner open EAST and NORTH, a
## tee closed SOUTH, so e.g. an end open on the east is two turns and a
## corner joining west and south is two turns.
const EXPECTED := {
	# Row 0-3: shapes.
	Vector2i(0, 0): ["pipe_end", 2],
	Vector2i(1, 0): ["pipe_end", 0],
	Vector2i(3, 0): ["pipe_end", 2],
	Vector2i(4, 0): ["pipe", 0],
	Vector2i(5, 0): ["pipe_end", 0],
	Vector2i(7, 0): ["pipe_end", 1],
	Vector2i(7, 1): ["pipe_end", 3],
	Vector2i(9, 0): ["pipe_end", 1],
	Vector2i(9, 1): ["pipe", 1],
	Vector2i(9, 2): ["pipe_end", 3],
	Vector2i(12, 0): ["pipe_corner", 0],
	Vector2i(13, 0): ["pipe_corner", 3],
	Vector2i(12, 1): ["pipe_corner", 1],
	Vector2i(13, 1): ["pipe_corner", 2],
	Vector2i(18, 1): ["pipe_tee", 0],
	Vector2i(22, 1): ["pipe_tee", 2],
	Vector2i(26, 1): ["pipe_tee", 3],
	Vector2i(30, 1): ["pipe_tee", 1],
	Vector2i(17, 1): ["pipe", 0],
	Vector2i(18, 2): ["pipe", 1],
	Vector2i(19, 1): ["pipe_end", 0],
	Vector2i(21, 1): ["pipe_end", 2],
	Vector2i(22, 0): ["pipe_end", 1],
	Vector2i(36, 1): ["pipe_cross", 0],
	Vector2i(35, 1): ["pipe", 0],
	Vector2i(36, 2): ["pipe", 1],
	Vector2i(39, 0): ["pipe", 0],
	Vector2i(39, 2): ["pipe", 1],
	# Row 5-7: fittings.
	Vector2i(1, 6): ["pipe_end", 0],
	Vector2i(4, 6): ["pipe_end", 2],
	Vector2i(8, 6): ["pipe_end", 3],
	Vector2i(11, 6): ["pipe_end", 1],
	Vector2i(14, 6): ["pipe_end", 2],
	Vector2i(19, 6): ["pipe_end", 0],
	Vector2i(22, 6): ["pipe_end", 1],
	Vector2i(25, 6): ["pipe_end", 3],
	# Row 10-12: the shipped middle.
	Vector2i(1, 12): ["pipe", 0],
	Vector2i(2, 12): ["pipe", 0],
	Vector2i(3, 12): ["pipe_corner", 2],
	Vector2i(4, 10): ["pipe", 0],
	Vector2i(5, 10): ["pipe", 0],
	Vector2i(8, 10): ["pipe", 0],
	Vector2i(9, 10): ["pipe", 0],
}

## Tile -> [fitting asset, side]. Every fitting the gallery should draw, and
## (checked the other way round) no tile outside this table draws one.
const EXPECTED_FITTINGS := {
	Vector2i(17, 1): [["pipe_fitting_tank", "west"]],
	Vector2i(18, 2): [["pipe_fitting_machine", "north"]],
	Vector2i(35, 1): [["pipe_fitting_tank", "west"]],
	Vector2i(36, 2): [["pipe_fitting_machine", "north"]],
	Vector2i(1, 6): [["pipe_fitting_tank", "west"]],
	Vector2i(4, 6): [["pipe_fitting_tank", "east"]],
	Vector2i(8, 6): [["pipe_fitting_tank", "south"]],
	Vector2i(11, 6): [["pipe_fitting_tank", "north"]],
	Vector2i(14, 6): [["pipe_fitting_machine", "east"]],
	Vector2i(19, 6): [["pipe_fitting_machine", "west"]],
	Vector2i(22, 6): [["pipe_fitting_machine", "north"]],
	Vector2i(25, 6): [["pipe_fitting_machine", "south"]],
	Vector2i(1, 12): [["pipe_fitting_tank", "west"]],
	Vector2i(3, 12): [["pipe_fitting_machine", "south"]],
	Vector2i(4, 10): [["pipe_fitting_machine", "west"]],
	Vector2i(5, 10): [["pipe_fitting_machine", "east"]],
	Vector2i(8, 10): [["pipe_fitting_machine", "west"]],
	Vector2i(9, 10): [["pipe_fitting_machine", "east"]],
}


func _run_checks() -> void:
	Layouts.build_pipe_gallery(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()
	main._update_fluid_network()

	var layout: Array = JSON.parse_string(main.sim.world_layout_json())
	# `line_placements`, not the layout's size: stocking a larder now builds
	# warehouse racks, and they are the fixture's storage rather than part of
	# the gallery being counted.
	var placed := Layouts.line_placements(main.sim)
	if placed != Layouts.PIPE_GALLERY_PLACEMENTS:
		_fail("the gallery placed %d things, expected %d — a refused placement" % [
			placed, Layouts.PIPE_GALLERY_PLACEMENTS
		])
		return

	# --- 1. every shape at every turn, from `links` alone -----------------
	var pipe_tiles: Array = []
	for placement in layout:
		if String(placement["kind"]) != "pipe":
			continue
		var pair: Array = placement["tile"]
		var tile := Vector2i(int(pair[0]), int(pair[1]))
		pipe_tiles.append(tile)
		if not placement.has("links"):
			_fail("the pipe at %s carries no `links` — ticket 1's wire field is missing" % str(tile))
			return
	if pipe_tiles.size() != 37 + 8 + 7:
		_fail("expected 52 pipe tiles in the gallery, got %d" % pipe_tiles.size())
		return
	for tile in EXPECTED.keys():
		var want: Array = EXPECTED[tile]
		var shape: Dictionary = main._pipe_shape_at(tile)
		if shape.is_empty():
			_fail("no shape record for the pipe at %s" % str(tile))
			return
		if String(shape["asset"]) != String(want[0]) or int(shape["turns"]) != int(want[1]):
			_fail("the pipe at %s is '%s' turned %d, expected '%s' turned %d (links: %s)" % [
				str(tile), shape["asset"], int(shape["turns"]), want[0], int(want[1]),
				str(main._placement_at_tile(tile).get("links"))
			])
			return
		var instance: Dictionary = main._tile_instance_at(tile)
		if instance.is_empty():
			_fail("the pipe at %s was not staged as an instance" % str(tile))
			return
		if String(instance["asset"]) != String(want[0]):
			_fail("the instance at %s draws '%s', expected '%s'" % [str(tile), instance["asset"], want[0]])
			return
		var want_yaw: float = deg_to_rad(float(int(want[1])) * 90.0)
		if not is_equal_approx(float(instance["yaw"]), want_yaw):
			_fail("the instance at %s is turned %.2f rad, expected %.2f" % [str(tile), float(instance["yaw"]), want_yaw])
			return
		if main._pipe_asset_for(tile) != String(want[0]):
			_fail("`_pipe_asset_for(%s)` says '%s', expected '%s'" % [str(tile), main._pipe_asset_for(tile), want[0]])
			return
	# Every pipe tile is a real mesh, not a silently-missing asset.
	for tile in pipe_tiles:
		var instance: Dictionary = main._tile_instance_at(tile)
		if instance.is_empty() or instance["nodes"].is_empty():
			_fail("the pipe at %s rendered no meshes — its .vox didn't import" % str(tile))
			return

	# --- 2. fittings on the right side, turned to it ---------------------
	for tile in pipe_tiles:
		var fittings: Array = main._pipe_fittings_at(tile)
		var want: Array = EXPECTED_FITTINGS.get(tile, [])
		if fittings.size() != want.size():
			_fail("the pipe at %s draws %d fittings, expected %d: %s" % [
				str(tile), fittings.size(), want.size(), str(fittings)
			])
			return
		for i in range(want.size()):
			var fitting: Dictionary = fittings[i]
			if String(fitting["asset"]) != String(want[i][0]) or String(fitting["to"]) != String(want[i][1]):
				_fail("the pipe at %s has fitting %s, expected %s on %s" % [
					str(tile), str(fitting), want[i][0], want[i][1]
				])
				return
			var key: String = main._pipe_fitting_key(tile, String(fitting["to"]))
			if not main._tile_instances.has(key):
				_fail("the fitting on the %s side of %s was not staged as an instance" % [fitting["to"], str(tile)])
				return
			var record: Dictionary = main._tile_instances[key]
			if not is_equal_approx(float(record["yaw"]), main._direction_yaw(String(fitting["to"]))):
				_fail("the fitting on the %s side of %s is not turned to that side" % [fitting["to"], str(tile)])
				return
			var frame_nodes: Array = main._tile_sets[record["set"]]["nodes"]
			if frame_nodes.is_empty() or frame_nodes[0].is_empty():
				_fail("the fitting '%s' rendered no meshes — its .vox didn't import" % fitting["asset"])
				return

	# --- 3. the liquid body per shape, at the shape's turn ---------------
	var bodies_by_tile: Dictionary = {}
	for body in main._pipe_fluid_nodes.values():
		bodies_by_tile[String(body.name).get_slice("_", String(body.name).get_slice_count("_") - 1)] = body
	var seen_bodies: Dictionary = {}
	for tile in EXPECTED.keys():
		var shape: Dictionary = main._pipe_shape_at(tile)
		var body: Node3D = bodies_by_tile.get("%d,%d" % [tile.x, tile.y], null)
		if body == null:
			_fail("no liquid body for the pipe at %s" % str(tile))
			return
		var expected_name := "%s_%d,%d" % [shape["fluid_asset"], tile.x, tile.y]
		if String(body.name) != expected_name:
			_fail("the liquid body at %s is '%s', expected '%s'" % [str(tile), body.name, expected_name])
			return
		if String(main.PIPE_FLUID_ASSETS[shape["asset"]]) != String(shape["fluid_asset"]):
			_fail("the shape '%s' names the body '%s'" % [shape["asset"], shape["fluid_asset"]])
			return
		if not is_equal_approx(body.rotation.y, float(shape["yaw"])):
			_fail("the liquid body at %s is not turned with its shape" % str(tile))
			return
		if _meshes(body, []).is_empty():
			_fail("the liquid body '%s' rendered no meshes" % shape["fluid_asset"])
			return
		seen_bodies[String(shape["fluid_asset"])] = true
	for shape_asset in main.PIPE_FLUID_ASSETS.keys():
		# The crossing's raised body is no shape of a pipe run, so the gallery
		# cannot draw it; test_crossings.gd proves it on a real crossing.
		if shape_asset == main.CROSSING_GANTRY_ASSET:
			continue
		var fluid_asset: String = main.PIPE_FLUID_ASSETS[shape_asset]
		if not seen_bodies.has(fluid_asset):
			_fail("the gallery never drew the '%s' body" % fluid_asset)
			return
	# Nothing is filled yet, so every body is hidden; fill the shipped run's
	# tank and the fed runs' tanks and let them pressurise.
	for body in main._pipe_fluid_nodes.values():
		if body.visible:
			_fail("an empty run should hide its liquid entirely")
			return
	main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
	for i in range(200):
		main.sim.tick()
	main._update_fluid_network()
	var visible := 0
	for body in main._pipe_fluid_nodes.values():
		if body.visible:
			visible += 1
			if body.scale.y > float(main.PIPE_BORE_HEIGHT_VOXELS) + 0.001:
				_fail("a liquid body (%f) rose past its bore" % body.scale.y)
				return
	if visible < 3:
		_fail("three tanks were filled and the runs ticked, but only %d bodies are drawn" % visible)
		return
	var corner_body: Node3D = bodies_by_tile["3,12"]
	if not corner_body.visible:
		_fail("the vinegar run's corner should hold liquid by now")
		return

	# --- 4. hover names the connections in words -------------------------
	var cases := {
		Vector2i(3, 12): ["connects west", "feeds Mixer"],
		Vector2i(4, 10): ["connects east", "fed by Mixer"],
		Vector2i(5, 10): ["connects west", "feeds Cooking Vat"],
		Vector2i(1, 12): ["connects east", "fed by Fluid Tank"],
		Vector2i(18, 1): ["connects north, east and west"],
		Vector2i(12, 0): ["connects north and east"],
		Vector2i(39, 0): ["no pipe joined", "east-west"],
		Vector2i(39, 2): ["no pipe joined", "north-south"],
		Vector2i(8, 6): ["fed by Fluid Tank"],
		Vector2i(22, 6): ["feeds Bottler"],
	}
	for tile in cases.keys():
		var text: String = main._hover_text_at_tile(tile)
		if not text.begins_with("Pipe"):
			_fail("hovering %s should describe a pipe, got: %s" % [str(tile), text])
			return
		for phrase in cases[tile]:
			if not text.contains(phrase):
				_fail("hover at %s should say '%s', got:\n%s" % [str(tile), phrase, text])
				return
	var level_text: String = main._hover_text_at_tile(Vector2i(2, 12))
	if not level_text.contains("units") or not level_text.contains("pressure"):
		_fail("a pipe in a run should report its level and pressure, got: %s" % level_text)
		return
	# A tank-only pipe's fitting and a machine-only pipe's fitting are named
	# even before anything flows.
	var lonely_text: String = main._hover_text_at_tile(Vector2i(39, 0))
	if lonely_text.contains("fed by") or lonely_text.contains("feeds"):
		_fail("a lonely pipe should name no feeder, got: %s" % lonely_text)
		return

	# --- 5. the ghost of a pipe takes its neighbours' shape ---------------
	var ghost_cases := [
		[Vector2i(2, 0), "east", "pipe", 0], # between (1,0) and (3,0)
		[Vector2i(13, 2), "east", "pipe_end", 3], # above the ring's (13,1)
		[Vector2i(11, 1), "east", "pipe_end", 2], # west of the ring's (12,1)
		[Vector2i(50, 0), "east", "pipe", 0], # nothing near: the facing's axis
		[Vector2i(50, 0), "south", "pipe", 1],
		[Vector2i(14, 0), "north", "pipe_end", 0], # east of the ring's (13,0): open west
	]
	for ghost_case in ghost_cases:
		var shape: Dictionary = main._pipe_ghost_shape(ghost_case[0], ghost_case[1])
		if String(shape["asset"]) != String(ghost_case[2]) or int(shape["turns"]) != int(ghost_case[3]):
			_fail("a pipe ghost at %s facing %s would be '%s' turned %d, expected '%s' turned %d" % [
				str(ghost_case[0]), ghost_case[1], shape["asset"], int(shape["turns"]),
				ghost_case[2], int(ghost_case[3])
			])
			return
	# ...and placement mode builds a real ghost body from it (the tile under
	# the cursor cannot be aimed headlessly — test_placement.gd draws the
	# same line — so the shape function above is the assertion and this is
	# the smoke check that the ghost path runs).
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "pipe"}))
	main._refresh_owned_counts()
	main._enter_build_mode("pipe")
	main._update_ghost()
	if main._ghost_root.get_node_or_null("ghost_body") == null:
		_fail("placement mode built no ghost body for a pipe")
		return
	main._exit_build_mode()

	# --- 6. the deprecated tiles are gone ----------------------------------
	for stale in ["pipe_source", "pipe_inlet"]:
		if ResourceLoader.exists("res://voxels/%s.vox" % stale):
			_fail("'%s' is still staged — ticket 6 deletes it" % stale)
			return
		if main._asset_classes.has(stale):
			_fail("'%s' is still in index.json" % stale)
			return

	print("PIPE_SHAPES_OK: %d pipe tiles drew every shape at every turn from their links, %d fittings on the right sides, one liquid body per shape, hover in words and a neighbour-shaped ghost" % [
		pipe_tiles.size(), EXPECTED_FITTINGS.size()
	])
	quit(0)


func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out
