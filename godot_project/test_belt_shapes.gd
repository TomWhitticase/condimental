# Headless proof of the belt presentation from the pipes and conveyors
# overhaul's ticket 5 (docs/PIPES_AND_CONVEYORS_PLAN.md): a belt tile's
# shape comes from the `links` the simulation reports, its animation frame
# from its own surface clock, its items' path from its shape, and its tooltip
# says all of it in words. Built against test_layouts.gd's `build_belt_gallery` through
# real purchase/place commands, so every assertion is about a tile the sim
# actually linked.
#
# Run with:
#   godot --headless --path godot_project --script test_belt_shapes.gd
#
# Verifies:
#   * `_belt_shape_for` is the plan's shape rule, pure, at all four facings:
#     a flank feed from the anticlockwise side is a LEFT corner, from the
#     clockwise side a RIGHT corner, from behind straight, two feeds a
#     merge, no feed a start, no output an end,
#   * the gallery's eight corner tiles are drawn with the corner families at
#     the yaw of their own facing (no yaw of their own), and its merge, start
#     and end tiles with the straight family — four frames each,
#   * a tile's frame index follows `floor(its own moved_ticks * 4 / 50) mod 4`
#     and the scene draws it there — so two tiles of one frame set sit on
#     different frames, a jammed tile's rubber stands still with the items on
#     it (issue #79), and with the pause menu up nothing moves at all,
#   * an item on a straight runs from the input face's midpoint to the
#     output face's; on a corner it follows the quarter-circle arc between
#     them (0, 0.5 and 1 checked), and the arc's end meets the next tile's
#     start,
#   * hover names what feeds a belt and what it hands to, says which way a
#     corner turns, and says "not connected" where the wire says null.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("BELT_SHAPES_FAIL: %s" % msg)
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


func _link(from: String, kind := "belt") -> Dictionary:
	return {"from": from, "kind": kind}


func _links(inputs: Array, to) -> Dictionary:
	return {"inputs": inputs, "output": null if to == null else {"to": to, "kind": "belt"}}


func _yaw_deg(instance: Dictionary) -> float:
	var got := rad_to_deg(float(instance["yaw"]))
	while got < 0.0:
		got += 360.0
	return got


func _run_checks() -> void:
	# --- 1. the shape rule, pure, at every facing --------------------------
	var expected_yaw := {"east": 0.0, "south": 90.0, "west": 180.0, "north": 270.0}
	for direction in main.DIRECTIONS:
		var behind: String = main._direction_opposite(direction)
		var left: String = main._direction_ccw(direction)
		var right: String = main._direction_cw(direction)
		var cases := {
			main.BELT_SHAPE_CORNER_LEFT: _links([_link(left)], direction),
			main.BELT_SHAPE_CORNER_RIGHT: _links([_link(right, "hopper")], direction),
			main.BELT_SHAPE_STRAIGHT: _links([_link(behind)], direction),
			main.BELT_SHAPE_MERGE: _links([_link(behind), _link(left, "hopper")], direction),
			main.BELT_SHAPE_START: _links([], direction),
			main.BELT_SHAPE_END: _links([_link(behind)], null),
		}
		for shape in cases.keys():
			var got: String = main._belt_shape_for(cases[shape], direction)
			if got != shape:
				_fail("facing %s, links %s should be '%s', got '%s'" % [direction, JSON.stringify(cases[shape]), shape, got])
				return
		# Two flank feeds and nothing behind is a merge, not a corner; a
		# corner with no output is still a corner.
		if main._belt_shape_for(_links([_link(left), _link(right)], direction), direction) != main.BELT_SHAPE_MERGE:
			_fail("two flank feeds facing %s should be a merge" % direction)
			return
		if main._belt_shape_for(_links([_link(right)], null), direction) != main.BELT_SHAPE_CORNER_RIGHT:
			_fail("a dead-ended flank feed facing %s is still a corner" % direction)
			return
		# A lone tile is a start.
		if main._belt_shape_for(_links([], null), direction) != main.BELT_SHAPE_START:
			_fail("a lone belt facing %s should be a line start" % direction)
			return

	# --- 2. the gallery: corners at every facing, spun by their facing -----
	Layouts.build_belt_gallery(main.sim)
	# `line_placements`, not the layout's size: stocking a larder now builds
	# warehouse racks, and they are the fixture's storage rather than part of
	# the gallery being counted.
	var placed := Layouts.line_placements(main.sim)
	if placed != Layouts.BELT_GALLERY_PLACEMENTS:
		_fail("the gallery placed %d things, expected %d — a refused place means a collision in the fixture" % [placed, Layouts.BELT_GALLERY_PLACEMENTS])
		return
	main._rebuild_world()
	var left_frames: Array = main._belt_frames_for_shape(main.BELT_SHAPE_CORNER_LEFT)
	var right_frames: Array = main._belt_frames_for_shape(main.BELT_SHAPE_CORNER_RIGHT)
	var straight: Array = main._belt_frames_for_shape(main.BELT_SHAPE_STRAIGHT)
	for table in [[Layouts.BELT_GALLERY_RIGHT_CORNERS, right_frames, "right"], [Layouts.BELT_GALLERY_LEFT_CORNERS, left_frames, "left"]]:
		var corners: Dictionary = table[0]
		var frames: Array = table[1]
		for direction in corners.keys():
			var tile: Vector2i = corners[direction]
			var instance: Dictionary = main._tile_instance_at(tile)
			if instance.is_empty() or instance["nodes"].is_empty():
				_fail("the %s corner at %s facing %s rendered nothing" % [table[2], str(tile), direction])
				return
			if instance["frames"] != frames:
				_fail("the %s corner at %s facing %s draws %s, expected %s" % [table[2], str(tile), direction, str(instance["frames"]), str(frames)])
				return
			if absf(_yaw_deg(instance) - float(expected_yaw[direction])) > 0.5:
				_fail("the %s corner at %s facing %s is turned %.1f, expected %.0f" % [table[2], str(tile), direction, _yaw_deg(instance), expected_yaw[direction]])
				return
			if main._belt_shapes.get(main._tile_key(tile), "") != ("corner_%s" % table[2]):
				_fail("the scene recorded shape '%s' for %s" % [str(main._belt_shapes.get(main._tile_key(tile), "")), str(tile)])
				return
	# Merge (fed from behind and by the hopper on its flank), two-flank
	# merge, line end and lone start are all the straight family.
	var straights := {
		Vector2i(1, 0): main.BELT_SHAPE_MERGE, Vector2i(10, 6): main.BELT_SHAPE_MERGE,
		Vector2i(11, 6): main.BELT_SHAPE_END, Vector2i(8, 6): main.BELT_SHAPE_START,
		Vector2i(8, 8): main.BELT_SHAPE_START, Vector2i(9, 8): main.BELT_SHAPE_END,
		Vector2i(1, 6): main.BELT_SHAPE_STRAIGHT,
	}
	for tile in straights.keys():
		var instance: Dictionary = main._tile_instance_at(tile)
		if instance.is_empty() or instance["frames"] != straight:
			_fail("the %s at %s should draw the straight family, got %s" % [straights[tile], str(tile), str(instance.get("frames", "nothing"))])
			return
		if main._belt_shapes.get(main._tile_key(tile), "") != straights[tile]:
			_fail("%s should be a %s, the scene recorded '%s'" % [str(tile), straights[tile], str(main._belt_shapes.get(main._tile_key(tile), ""))])
			return

	# --- 3. the frame follows the TILE'S OWN clock (issue #79) ------------
	# The formula is unchanged; the clock it reads is not. `moved_ticks` is
	# how many ticks the SIMULATION advanced product on that tile, so a
	# jammed tile stops counting and its rubber stops with the items on it.
	var frame_cases := {0: 0, 12: 0, 13: 1, 25: 2, 37: 2, 38: 3, 49: 3, 50: 0, 63: 1, 100: 0}
	for clock in frame_cases.keys():
		if main._belt_frame_for_moved_ticks(clock) != frame_cases[clock]:
			_fail("a clock of %d should show frame %d, got %d" % [clock, frame_cases[clock], main._belt_frame_for_moved_ticks(clock)])
			return
	# Run the gallery for real: stock its larder, fill its hoppers, and step
	# the scene's own loop a frame at a time. The twelve-tile clockwise ring
	# takes tomatoes until it is packed solid against itself and then never
	# moves again, which is exactly the state this ticket is about.
	Layouts.stock_larder(main.sim)
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
	var tick_interval: float = 1.0 / main.TICK_HZ
	for _i in range(3000):
		main._advance_simulation(tick_interval)
	# (1, 0) is on the ring, fed by the hopper from its flank and by the
	# tile behind it; (8, 6) is the lone start nothing feeds. Both draw the
	# straight family, so they are instances of the SAME four-frame set.
	var ring := Vector2i(1, 0)
	var lone := Vector2i(8, 6)
	var ring_key: String = main._tile_key(ring)
	var lone_key: String = main._tile_key(lone)
	var ring_clock: int = int(main._belt_surface_ticks.get(ring_key, 0))
	if ring_clock <= 0:
		_fail("the ring tile at %s should have carried tomatoes by now, its clock reads %d" % [str(ring), ring_clock])
		return
	if int(main._belt_surface_ticks.get(lone_key, -1)) != 0:
		_fail("nothing feeds the lone belt at %s, so its clock must read 0, got %s" % [str(lone), str(main._belt_surface_ticks.get(lone_key, -1))])
		return
	# Both are straight tiles in the SAME four-frame set, and they are on
	# different frames — which is the whole change: before this, one scene
	# counter put every belt in the world on one frame.
	var ring_frame: int = main._tile_instances[ring_key]["frame"]
	var lone_frame: int = main._tile_instances[lone_key]["frame"]
	if main._tile_instances[ring_key]["set"] != main._tile_instances[lone_key]["set"]:
		_fail("the fixture wants two tiles in one frame set to compare; %s and %s are in different sets" % [str(ring), str(lone)])
		return
	if ring_frame != main._belt_frame_for_moved_ticks(ring_clock) or lone_frame != 0:
		_fail("tile frames should follow their own clocks: %s reads clock %d on frame %d, %s on frame %d" % [str(ring), ring_clock, ring_frame, str(lone), lone_frame])
		return
	if ring_frame == lone_frame:
		_fail("a carrying tile and a tile that has never carried anything are both on frame %d — the rubber is still running off one scene clock" % ring_frame)
		return
	# Every belt in the world, not just these two: each tile's drawn frame
	# is its own clock's frame.
	#
	# What this CANNOT assert is the transform write underneath, and that is
	# the headless renderer rather than a gap in the check: `MultiMesh`
	# instance buffers live in the RenderingServer, and the dummy backend
	# reads every one of them back as the identity — `get_instance_transform`
	# on a tile this scene positioned at (3.2, 0.3, 0.0) answers the origin.
	# So the record below is the honest seam, and
	# `scripts/screenshot-scene.sh --belt-gallery --closeup` is where the
	# rubber itself is checked (UI_UX.md §2b point 12).
	for key in main._belt_surface_ticks.keys():
		var record: Dictionary = main._tile_instances.get(key, {})
		if record.is_empty() or record["frames"].size() <= 1:
			continue
		var want: int = main._belt_frame_for_moved_ticks(int(main._belt_surface_ticks[key]))
		if int(record["frame"]) != want:
			_fail("the tile %s reads clock %s, which is frame %d, but it draws frame %d" % [key, str(main._belt_surface_ticks[key]), want, int(record["frame"])])
			return
	# THE REGRESSION. The ring is jammed: sixty more ticks pass — long
	# enough for the old scene-wide counter to have stepped four frames —
	# and no tile on it moves a frame, because no tile on it moved an item.
	var jammed := {}
	for key in main._belt_surface_ticks.keys():
		jammed[key] = main._belt_surface_ticks[key]
	var tick_before: int = main.sim.current_tick()
	for _i in range(60):
		main._advance_simulation(tick_interval)
	if main.sim.current_tick() != tick_before + 60:
		_fail("the fixture should have stepped 60 ticks, went %d -> %d" % [tick_before, main.sim.current_tick()])
		return
	if int(main._belt_surface_ticks.get(ring_key, -1)) != ring_clock:
		_fail("the ring is packed solid, so its clock must stand still; %d -> %s over 60 ticks" % [ring_clock, str(main._belt_surface_ticks.get(ring_key, -1))])
		return
	if main._tile_instances[ring_key]["frame"] != ring_frame:
		_fail("a jammed belt moved from frame %d to %d — its rubber is running under items that are not" % [ring_frame, main._tile_instances[ring_key]["frame"]])
		return
	# --- and the pause menu freezes what is still moving ------------------
	tick_before = main.sim.current_tick()
	main._set_pause_open(true)
	for _i in range(60):
		main._advance_simulation(tick_interval)
	if main.sim.current_tick() != tick_before:
		_fail("with the pause menu up the tick moved %d -> %d" % [tick_before, main.sim.current_tick()])
		return
	for key in jammed.keys():
		if int(main._belt_surface_ticks.get(key, -1)) != int(jammed[key]):
			_fail("with the pause menu up the tile %s moved its clock %s -> %s" % [key, str(jammed[key]), str(main._belt_surface_ticks.get(key, -1))])
			return
	main._set_pause_open(false)

	# --- 4. items: straight runs face to face, corners follow the arc -----
	var T: float = main.TILE
	var Y: float = main.BELT_SURFACE_Y
	# (1, 6) is a straight belt facing east.
	var s0: Vector3 = main._carried_item_transform(Vector2i(1, 6), "east", 0.0).origin
	var s1: Vector3 = main._carried_item_transform(Vector2i(1, 6), "east", 1.0).origin
	if not s0.is_equal_approx(Vector3(0.5 * T, Y, 6.0 * T)) or not s1.is_equal_approx(Vector3(1.5 * T, Y, 6.0 * T)):
		_fail("a straight east belt at (1, 6) should carry from its west face to its east face, got %s -> %s" % [str(s0), str(s1)])
		return
	# (3, 3) is a right corner facing south, fed from the west: the arc runs
	# from the west face's midpoint to the south face's about the tile's
	# south-west corner.
	var c0: Vector3 = main._carried_item_transform(Vector2i(3, 3), "south", 0.0).origin
	var c5: Vector3 = main._carried_item_transform(Vector2i(3, 3), "south", 0.5).origin
	var c1: Vector3 = main._carried_item_transform(Vector2i(3, 3), "south", 1.0).origin
	var centre := Vector3(3.0 * T, Y, 3.0 * T)
	var pivot := centre + Vector3(-0.5 * T, 0.0, -0.5 * T)
	var mid := pivot + Vector3(0.5 * T * cos(PI / 4.0), 0.0, 0.5 * T * sin(PI / 4.0))
	if not c0.is_equal_approx(centre + Vector3(-0.5 * T, 0.0, 0.0)):
		_fail("the corner's item at distance 0 should stand on its west face midpoint, got %s" % str(c0))
		return
	if not c1.is_equal_approx(centre + Vector3(0.0, 0.0, -0.5 * T)):
		_fail("the corner's item at distance 1 should stand on its south face midpoint, got %s" % str(c1))
		return
	if not c5.is_equal_approx(mid):
		_fail("the corner's item at distance 0.5 should be on the arc at %s, got %s" % [str(mid), str(c5)])
		return
	if not is_equal_approx((c5 - pivot).length(), 0.5 * T):
		_fail("the arc's radius should be half a tile")
		return
	# Continuity: the corner's end is the next tile's start, and the belt
	# before it ends where the corner starts.
	var next0: Vector3 = main._carried_item_transform(Vector2i(3, 2), "south", 0.0).origin
	var prev1: Vector3 = main._carried_item_transform(Vector2i(2, 3), "east", 1.0).origin
	if not c1.is_equal_approx(next0) or not c0.is_equal_approx(prev1):
		_fail("items jump at the corner: %s -> %s and %s -> %s" % [str(prev1), str(c0), str(c1), str(next0)])
		return
	# A left corner bends the other way: (9, 0) faces north, fed from the
	# west, arc about the north-west corner.
	var l5: Vector3 = main._carried_item_transform(Vector2i(9, 0), "north", 0.5).origin
	var l_pivot := Vector3(9.0 * T, Y, 0.0) + Vector3(-0.5 * T, 0.0, 0.5 * T)
	if not is_equal_approx((l5 - l_pivot).length(), 0.5 * T) or l5.x >= 9.0 * T or l5.z <= 0.0:
		_fail("the left corner's midpoint item should sit on an arc about its north-west corner, got %s" % str(l5))
		return

	# --- 5. hover says it in words -----------------------------------------
	var checks := {
		Vector2i(1, 6): ["carries east", "fed from the west (Hopper)", "hands to Crusher"],
		Vector2i(3, 6): ["fed from the west (Crusher)", "hands to Dock"],
		Vector2i(3, 3): ["carries south", "turns right", "fed from the west", "hands south to the next belt"],
		Vector2i(9, 0): ["turns left"],
		Vector2i(10, 6): ["merge", "fed from the north and the south"],
		Vector2i(11, 6): ["fed from the west", "not connected"],
		Vector2i(8, 6): ["fed by nothing", "not connected"],
		Vector2i(1, 0): ["merge", "(Hopper)"],
	}
	for tile in checks.keys():
		var text: String = main._hover_text_at_tile(tile)
		for phrase in checks[tile]:
			if not text.contains(phrase):
				_fail("hover over %s should contain '%s', got: %s" % [str(tile), phrase, text.replace("\n", " / ")])
				return

	print("BELT_SHAPES_OK: shape rule pure at every facing, gallery corners drawn with the corner families at their facing's yaw, merges/starts/ends straight, each tile's frame = floor(its own moved_ticks*%d/%d) mod %d, frozen while jammed and while paused, items run face to face and arc through corners continuously, hover names feeds and hand-offs" % [main.BELT_FRAMES, main.BELT_TICKS_PER_TILE, main.BELT_FRAMES])
	quit(0)
