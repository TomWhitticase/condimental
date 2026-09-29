# Headless proof of the conveyor splitter's presentation (Epic B, ticket 2) —
# the structural claims a machine can check without a framebuffer. Whether the
# arrow actually READS at scene scale is what scripts/screenshot-scene.sh
# --fixture splitter is for, the same division test_machines.gd and
# test_presentation.gd already draw.
#
# Run with:
#   godot --headless --path godot_project --script test_splitters.gd
#
# Verifies:
#   * a placed splitter renders its own bespoke model — not the generic
#     `machine` housing — at the tile the simulation reports,
#   * the belts around a splitter — the one feeding it, the one it hands
#     straight to and its left belt — are plain straights (each is fed
#     from behind), and the feeding belt's tooltip names the splitter,
#   * the next-output arrow exists, points along the placement's own facing
#     when the straight side is due, and turns exactly one quarter turn
#     CLOCKWISE — the simulation's own `rotate_cw` — when the left is,
#   * a splitter whose left belt was never built shows the arrow pointing
#     STRAIGHT even though `next_output` says "left", because that is where
#     the item actually goes (`splitter.rs`'s fallback rule), and one with
#     no output connected hides the arrow altogether,
#   * items crossing a splitter render as item nodes at the running height,
#     from the same `items` field a belt tile uses,
#   * product that went DOWN THE BRANCH is drawn — which is a regression test
#     as much as a splitter one: the branch leaves the splitter's own chunk,
#     and the per-pass hide this scene used to do meant one dirty chunk's
#     render wiped the other's items (see `_render_carried_items`),
#   * hovering a splitter names its mode's outputs and which one is next, and
#     says "not connected" about a side that was never laid,
#   * clicking a splitter opens the configuration panel with its four modes,
#     and picking one issues `set_splitter_mode`: the body swaps to that
#     mode's model, the arrow turns the other way for `right`, and the
#     tooltip names only the mode's own sides.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## The fixture's two splitters (test_layouts.gd's `build_splitter_line`): the
## first has both outputs built, the second only its straight one.
const FORKED := Vector2i(2, 0)
const STRAIGHT_ONLY := Vector2i(6, 0)

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("SPLITTERS_FAIL: %s" % msg)
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


## A splitter's body is a MultiMesh instance like a belt's (the pipes &
## conveyors overhaul's ticket 4); only its arrow is a node.
func _splitter_instance(tile: Vector2i) -> Dictionary:
	return main._tile_instance_at(tile)


func _arrow(tile: Vector2i) -> Node3D:
	var nodes: Dictionary = main._splitter_nodes.get(main._tile_key(tile), {})
	return nodes.get("arrow") if not nodes.is_empty() else null


## Ticks until `predicate` holds or the budget runs out, applying dirty
## chunks each tick exactly as `_process` does — a splitter's state only
## reaches the scene through a chunk snapshot, so a test that ticked without
## applying them would be testing nothing.
func _tick_until(budget: int, predicate: Callable) -> bool:
	for _i in range(budget):
		main.sim.tick()
		main._apply_dirty_chunks()
		if predicate.call():
			return true
	return false


func _run_checks() -> void:
	Layouts.build_splitter_line(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()

	# --- a bespoke model, at the tile the simulation reports ---------------
	if main._shop_asset_for("splitter") == main.MACHINE_MODEL_FALLBACK:
		_fail("the splitter kind still resolves to the generic '%s' model" % main.MACHINE_MODEL_FALLBACK)
		return
	var body := _splitter_instance(FORKED)
	if body.is_empty():
		_fail("no rendered instance for the splitter placed at %s" % str(FORKED))
		return
	if body["asset"] != main.SPLITTER_ASSET:
		_fail("the splitter is drawn as '%s', expected '%s'" % [body["asset"], main.SPLITTER_ASSET])
		return
	if body["nodes"].is_empty():
		_fail("the placed splitter rendered no meshes — its .vox didn't import")
		return
	var expected: Vector3 = main._tile_origin(FORKED.x, FORKED.y, main.FLOOR_Y)
	if not body["position"].is_equal_approx(expected):
		_fail("the splitter sits at %s, expected its tile centre %s" % [str(body["position"]), str(expected)])
		return

	# --- the belts around a splitter are plain straights -------------------
	# (1, 0) hands over to the splitter at (2, 0) and is fed from behind;
	# (3, 0) is fed by the splitter from behind; the left belt (2, -1)
	# faces south and is fed by the splitter from the north — behind it. All
	# three are straight, and the splitter keeps its own model.
	var straight: Array = main._belt_frames_for_shape(main.BELT_SHAPE_STRAIGHT)
	for tile in [Vector2i(1, 0), Vector2i(3, 0), Vector2i(2, -1)]:
		var belt: Dictionary = main._tile_instance_at(tile)
		if belt.is_empty() or belt["asset"] != straight[0]:
			_fail("the belt at %s next to a splitter draws '%s', expected the plain '%s'" % [str(tile), str(belt.get("asset", "nothing")), straight[0]])
			return
	if not main._hover_text_at_tile(Vector2i(1, 0)).contains("hands east to a splitter"):
		_fail("the belt feeding the splitter should say so, got: %s" % main._hover_text_at_tile(Vector2i(1, 0)).replace("\n", " / "))
		return

	# --- the arrow, and where it points before anything has moved ---------
	var arrow := _arrow(FORKED)
	if arrow == null:
		_fail("the splitter has no next-output arrow node")
		return
	if _meshes(arrow, []).is_empty():
		_fail("the splitter's arrow rendered no meshes")
		return
	if not arrow.visible:
		_fail("a freshly placed splitter with both outputs built should show its arrow")
		return
	# `Side::default()` is Straight, so a splitter that has passed nothing yet
	# points the way it faces — and that is asked of the simulation
	# (`_seed_splitter_states`), not assumed here.
	var east_yaw: float = main._direction_yaw("east")
	if not is_equal_approx(arrow.rotation.y, east_yaw):
		_fail("an east-facing splitter due to go straight should point east (yaw %f), points %f" % [
			east_yaw, arrow.rotation.y,
		])
		return
	var arrow_y: float = arrow.position.y
	var groove_y: float = main.SPLITTER_ARROW_FLOOR_Y
	if not is_equal_approx(arrow_y, groove_y):
		_fail("the arrow stands at y=%f, expected the deck groove at %f" % [arrow_y, groove_y])
		return

	# --- the left side, driven by the real simulation ---------------------
	# The hopper meters tomatoes onto the line; once one has crossed the
	# splitter the cursor flips and the arrow must turn a quarter turn
	# clockwise from the placement's own facing.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	var branch_yaw := east_yaw + PI * 0.5
	if not _tick_until(2000, func(): return is_equal_approx(_arrow(FORKED).rotation.y, branch_yaw)):
		_fail("after an item crossed the splitter the arrow never turned left (yaw %f), it reads %f" % [
			branch_yaw, _arrow(FORKED).rotation.y,
		])
		return
	# Whatever the simulation says next, the arrow only ever occupies one of
	# those two headings.
	var state: Dictionary = main._splitter_states.get(main._tile_key(FORKED), {})
	if state.is_empty() or not state.has("next_output"):
		_fail("the splitter's snapshot never reached the scene")
		return

	# --- items cross a splitter, drawn like items on a belt ---------------
	var carried := _item_on_splitter()
	if not carried:
		# Not every tick has an item on the splitter; run on until one is.
		if not _tick_until(2000, func(): return _item_on_splitter()):
			_fail("no item was ever drawn crossing a splitter, though the line is running")
			return

	# --- what went down the branch is DRAWN, not just simulated -----------
	# The branch runs to y = -1 and -2, which is a different 32x32 chunk from
	# the splitter that fed it, so this is the case that caught the per-pass
	# item hiding: the simulation held eight tomatoes down there and the
	# scene drew none of them. Asserted against the simulation's own count so
	# it cannot pass by drawing the wrong number.
	var branch_tile := Vector2i(2, -1)
	if not _tick_until(4000, func(): return _items_at(branch_tile) > 0):
		_fail("nothing ever reached the branch belt at %s" % str(branch_tile))
		return
	var carried_there: int = _items_at(branch_tile)
	var drawn := 0
	for i in range(carried_there):
		var key := "belt_%d_%d_%d" % [branch_tile.x, branch_tile.y, i]
		if not main._carried_item(key).is_empty():
			drawn += 1
	if drawn != carried_there:
		_fail("the branch belt carries %d items and the scene draws %d — items in a second chunk are being hidden" % [
			carried_there, drawn,
		])
		return

	# --- A rebuild keeps the OTHER chunk's items (the quality pass) ---------
	# Placing anything rebuilds the scene and empties the item pools, and
	# only the changed chunk comes back dirty. A JAMMED belt in another chunk
	# never dirties its own (nothing on it moves), so its items used to
	# vanish on the next place/remove/rotate anywhere else — for good. Build
	# a dead-end lane straddling the chunk seam at x = 0, let it back up
	# until a tick dirties nothing, then place a belt far away in the line's
	# chunk and look again.
	var jam_tile := Vector2i(-1, 5) # chunk (-1, 0); the dead end at (0, 5) is chunk (0, 0)
	Layouts.place(main.sim, "hopper", Vector2i(-2, 5), "east", "tomato")
	Layouts.place(main.sim, "belt", jam_tile, "east")
	Layouts.place(main.sim, "belt", Vector2i(0, 5), "east")
	main._rebuild_world()
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "tile": [-2, 5]}))
	var settled := _tick_until(4000, func():
		return _items_at(jam_tile) > 0 and main.sim.dirty_chunk_ids().is_empty())
	if not settled:
		_fail("the dead-end lane never jammed still (items on %s: %d, dirty chunks: %s)" % [
			str(jam_tile), _items_at(jam_tile), str(main.sim.dirty_chunk_ids()),
		])
		return
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	main._issue_world_command({"type": "place", "kind": "belt", "tile": [12, 12], "direction": "east"})
	if main._placement_at_tile(Vector2i(12, 12)).is_empty():
		_fail("the far belt was not placed — the rebuild check below would be vacuous")
		return
	var still_there := _items_at(jam_tile)
	var still_drawn := 0
	for i in range(still_there):
		if not main._carried_item("belt_%d_%d_%d" % [jam_tile.x, jam_tile.y, i]).is_empty():
			still_drawn += 1
	if still_there < 1 or still_drawn != still_there:
		_fail("after placing a belt in another chunk the jammed belt at %s carries %d items and the scene draws %d — a rebuild must re-read every chunk, not only the dirty one" % [
			str(jam_tile), still_there, still_drawn,
		])
		return

	if not _tick_until(4000, func(): return _due_branch(STRAIGHT_ONLY)):
		_fail("the branchless splitter never reported 'left' as due — the fallback case was never exercised")
		return
	var lonely := _arrow(STRAIGHT_ONLY)
	if not lonely.visible:
		_fail("a splitter with a working straight output must still show its arrow")
		return
	if not is_equal_approx(lonely.rotation.y, east_yaw):
		_fail("a splitter with no branch must point STRAIGHT (yaw %f) whatever `next_output` says, points %f" % [
			east_yaw, lonely.rotation.y,
		])
		return

	# --- the pure fallback rule, including the dead end --------------------
	# Round the rotation straight, left, right from the due side — the
	# simulation reports a side its mode does not deal to as null, so that
	# walk is the mode's own rotation.
	for case in [
		[{"next_output": "left", "outputs": [[1, 0], null, null]}, main.SPLITTER_STRAIGHT, "a due left with nothing on it falls back straight"],
		[{"next_output": "straight", "outputs": [null, [0, -1], null]}, main.SPLITTER_LEFT, "a due straight with nothing on it falls back left"],
		[{"next_output": "straight", "outputs": [null, null, [0, 1]]}, main.SPLITTER_RIGHT, "a due straight with only a right falls back right"],
		[{"next_output": "left", "outputs": [[1, 0], null, [0, 1]]}, main.SPLITTER_RIGHT, "a three-way's due left falls to right, next in the rotation"],
		[{"next_output": "right", "outputs": [[1, 0], [0, -1], null]}, main.SPLITTER_STRAIGHT, "a due right wraps round to straight"],
		[{"next_output": "straight", "outputs": [null, null, null]}, -1, "a splitter with no output connected has nowhere to send anything"],
	]:
		if main._splitter_effective_side(case[0]) != case[1]:
			_fail("%s (got %d)" % [case[2], main._splitter_effective_side(case[0])])
			return
	main._apply_splitter_state(main._tile_key(FORKED), {"next_output": "straight", "outputs": [null, null, null]})
	if _arrow(FORKED).visible:
		_fail("a splitter with no output connected must hide its arrow, not point at bare floor")
		return

	# --- hover ------------------------------------------------------------
	var forked_text: String = main._hover_text_at_tile(FORKED)
	if not forked_text.begins_with("Splitter"):
		_fail("hovering a splitter says '%s'" % forked_text)
		return
	var lonely_text: String = main._hover_text_at_tile(STRAIGHT_ONLY)
	for needle in ["straight:", "left:", "next:", "left"]:
		if not lonely_text.contains(needle):
			_fail("the splitter tooltip is missing '%s': '%s'" % [needle, lonely_text])
			return
	if not lonely_text.contains("not connected"):
		_fail("a splitter with an unbuilt left belt must say so; it says '%s'" % lonely_text)
		return
	if lonely_text.contains("right:"):
		_fail("a left-mode splitter must not list a right exit it does not have: '%s'" % lonely_text)
		return

	if not _check_mode_panel():
		return

	print("SPLITTERS_OK: a bespoke model at the sim's tile, the feeding belt still plain, the")
	print("  next-output arrow pointing along the facing and one quarter turn clockwise for the")
	print("  branch, an unbuilt branch falling back to straight and a dead end hiding the arrow,")
	print("  items drawn crossing the tile, hover naming the mode's outputs, and the")
	print("  configuration panel switching the mode, the body and the arrow's side")
	quit(0)


## Whether any drawn item is on a splitter slot, at belt-surface height —
## the same check the belt items get, since a splitter carries exactly like
## a belt tile.
func _item_on_splitter() -> bool:
	for key in main._item_slots.keys():
		if String(key).begins_with("splitter_"):
			var item: Dictionary = main._carried_item(key)
			if not item.is_empty() and is_equal_approx(float(item["position"].y), main.BELT_SURFACE_Y):
				return true
	return false


## How many items the SIMULATION says are on one tile, straight from
## `query_tile`, so a rendering claim can be checked against it.
func _items_at(tile: Vector2i) -> int:
	var parsed = JSON.parse_string(main.sim.query_tile_json(tile.x, tile.y))
	if parsed == null:
		return 0
	return int(parsed.get("items_here", []).size())


func _due_branch(tile: Vector2i) -> bool:
	var state: Dictionary = main._splitter_states.get(main._tile_key(tile), {})
	return String(state.get("next_output", "")) == "left"


## The splitter's configuration panel, end to end through the real click and
## the real command: the four modes offered, the current one ticked, and a
## pick that reaches the simulation and comes back as a new body, arrow and
## tooltip.
func _check_mode_panel() -> bool:
	main._config_click_at_tile(FORKED)
	if not main._config_open or main._config_kind != main.SPLITTER_KIND:
		_fail("clicking a splitter did not open its configuration panel")
		return false
	var offered: Array = []
	for slot in main._config_picker_slots:
		offered.append(String(slot["item"]))
	if offered != ["left", "right", "t_junction", "three_way"]:
		_fail("the splitter panel offers %s, expected the four modes" % str(offered))
		return false
	if not main._config_picker_slots[0]["mark"].visible:
		_fail("a fresh splitter's panel should tick 'left', its default mode")
		return false
	for row in [
		["fill row", main._config_fill_row], ["AUTO-FILL", main._config_auto_button],
		["AUTO-FILL note", main._config_auto_note], ["level", main._config_level_label],
		["level bar", main._config_level_track],
	]:
		if row[1].visible:
			_fail("a splitter has nothing to fill, but its panel shows the %s" % row[0])
			return false

	for pick in [
		["right", "machine_splitter_right", ["straight:", "right:"], ["left:"]],
		["t_junction", "machine_splitter_t_junction", ["left:", "right:"], ["straight:"]],
		["three_way", "machine_splitter_three_way", ["straight:", "left:", "right:"], []],
	]:
		var mode: String = pick[0]
		main._on_config_picker_pressed(main.SPLITTER_MODES.find(mode))
		var placement: Dictionary = main._placement_at_tile(FORKED)
		if String(placement.get("split_mode", "")) != mode:
			_fail("picking '%s' left the splitter in mode '%s'" % [mode, placement.get("split_mode")])
			return false
		var body := _splitter_instance(FORKED)
		if body.get("asset") != pick[1]:
			_fail("a '%s' splitter is drawn as '%s', expected '%s'" % [mode, body.get("asset"), pick[1]])
			return false
		if body["nodes"].is_empty():
			_fail("the '%s' body rendered no meshes — its .vox didn't import" % mode)
			return false
		var ticked := ""
		for slot in main._config_picker_slots:
			if slot["mark"].visible:
				ticked = String(slot["item"])
		if ticked != mode:
			_fail("after picking '%s' the panel ticks '%s'" % [mode, ticked])
			return false
		var label: String = main.SPLITTER_MODE_LABELS[mode]
		if not main._config_title.text.ends_with(label):
			_fail("a '%s' splitter's panel is titled '%s', expected it to name '%s'" % [mode, main._config_title.text, label])
			return false
		var text: String = main._hover_text_at_tile(FORKED)
		for needle in pick[2]:
			if not text.contains(needle):
				_fail("a '%s' splitter's tooltip is missing '%s': '%s'" % [mode, needle, text])
				return false
		for needle in pick[3]:
			if text.contains(needle):
				_fail("a '%s' splitter's tooltip lists '%s', a side it does not deal to: '%s'" % [mode, needle, text])
				return false

	# A right turn is drawn the other way round from a left one.
	var east_yaw: float = main._direction_yaw("east")
	main._apply_splitter_state(main._tile_key(FORKED), {
		"next_output": "right", "outputs": [[3, 0], [2, -1], [2, 1]],
	})
	if not is_equal_approx(_arrow(FORKED).rotation.y, east_yaw - PI * 0.5):
		_fail("a due right should turn the arrow a quarter turn the other way from left (yaw %f), it reads %f" % [
			east_yaw - PI * 0.5, _arrow(FORKED).rotation.y,
		])
		return false
	main._close_config_panel()
	return true
