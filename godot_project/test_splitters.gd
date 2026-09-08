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
#   * a belt handing over TO a splitter draws the plain belt tile, not the
#     hand-over-to-a-machine variant: a splitter is more conveyor, and
#     `belt_inlet`'s dead-plate in front of it would be a lie,
#   * the next-output arrow exists, points along the placement's own facing
#     when the straight side is due, and turns exactly one quarter turn
#     CLOCKWISE — the simulation's own `rotate_cw` — when the branch is,
#   * a splitter whose branch was never built shows the arrow pointing
#     STRAIGHT even though `next_output` says "branch", because that is where
#     the item actually goes (`splitter.rs`'s fallback rule), and one with
#     neither output connected hides the arrow altogether,
#   * items crossing a splitter render as item nodes at the running height,
#     from the same `items` field a belt tile uses,
#   * product that went DOWN THE BRANCH is drawn — which is a regression test
#     as much as a splitter one: the branch leaves the splitter's own chunk,
#     and the per-pass hide this scene used to do meant one dirty chunk's
#     render wiped the other's items (see `_render_carried_items`),
#   * hovering a splitter names both outputs and which one is next, and says
#     "not connected" about a branch that was never laid.
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


func _splitter_node(tile: Vector2i) -> Node3D:
	return main.get_node_or_null("World/splitter_%d,%d" % [tile.x, tile.y])


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
	var body := _splitter_node(FORKED)
	if body == null:
		_fail("no rendered node for the splitter placed at %s" % str(FORKED))
		return
	if _meshes(body, []).is_empty():
		_fail("the placed splitter rendered no meshes — its .vox didn't import")
		return
	var expected: Vector3 = main._tile_origin(FORKED.x, FORKED.y, main.FLOOR_Y)
	if not body.position.is_equal_approx(expected):
		_fail("the splitter sits at %s, expected its tile centre %s" % [str(body.position), str(expected)])
		return

	# --- a belt feeding a splitter is still a plain belt -------------------
	# (1, 0) hands over to the splitter at (2, 0). If a splitter counted as a
	# building the tile would draw `belt_inlet`, which puts a machine's
	# dead-plate and kerbs in front of what is actually more conveyor.
	var feeding: String = main._belt_asset_for(Vector2i(1, 0), "east")
	if feeding != main.BELT_ASSET:
		_fail("the belt feeding a splitter draws '%s', expected the plain '%s'" % [feeding, main.BELT_ASSET])
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

	# --- the branch side, driven by the real simulation -------------------
	# The hopper meters tomatoes onto the line; once one has crossed the
	# splitter the cursor flips and the arrow must turn a quarter turn
	# clockwise from the placement's own facing.
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	var branch_yaw := east_yaw + PI * 0.5
	if not _tick_until(2000, func(): return is_equal_approx(_arrow(FORKED).rotation.y, branch_yaw)):
		_fail("after an item crossed the splitter the arrow never turned to the branch (yaw %f), it reads %f" % [
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
	var carried := false
	for key in main._item_nodes.keys():
		if String(key).begins_with("splitter_"):
			var node: Node3D = main._item_nodes[key]["node"]
			if node.visible and is_equal_approx(node.position.y, main.BELT_SURFACE_Y):
				carried = true
				break
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
		if main._item_nodes.has(key) and main._item_nodes[key]["node"].visible:
			drawn += 1
	if drawn != carried_there:
		_fail("the branch belt carries %d items and the scene draws %d — items in a second chunk are being hidden" % [
			carried_there, drawn,
		])
		return

	# --- the splitter whose branch was never built ------------------------
	# Its `next_output` alternates to "branch" as soon as it has passed one
	# item, and stays there — nothing can ever take it. The arrow must point
	# where the item actually goes.
	if not _tick_until(4000, func(): return _due_branch(STRAIGHT_ONLY)):
		_fail("the branchless splitter never reported 'branch' as due — the fallback case was never exercised")
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
	if main._splitter_effective_side({"next_output": "branch", "outputs": [[1, 0], null]}) != main.SPLITTER_STRAIGHT:
		_fail("a due branch with nothing on it must fall back to the straight output")
		return
	if main._splitter_effective_side({"next_output": "straight", "outputs": [null, [0, -1]]}) != main.SPLITTER_BRANCH:
		_fail("a due straight with nothing on it must fall back to the branch")
		return
	if main._splitter_effective_side({"next_output": "straight", "outputs": [null, null]}) != -1:
		_fail("a splitter with neither output connected has nowhere to send anything")
		return
	main._apply_splitter_state(main._tile_key(FORKED), {"next_output": "straight", "outputs": [null, null]})
	if _arrow(FORKED).visible:
		_fail("a splitter with neither output connected must hide its arrow, not point at bare floor")
		return

	# --- hover ------------------------------------------------------------
	var forked_text: String = main._hover_text_at_tile(FORKED)
	if not forked_text.begins_with("splitter"):
		_fail("hovering a splitter says '%s'" % forked_text)
		return
	var lonely_text: String = main._hover_text_at_tile(STRAIGHT_ONLY)
	for needle in ["straight:", "branch:", "next:"]:
		if not lonely_text.contains(needle):
			_fail("the splitter tooltip is missing '%s': '%s'" % [needle, lonely_text])
			return
	if not lonely_text.contains("not connected"):
		_fail("a splitter with an unbuilt branch must say so; it says '%s'" % lonely_text)
		return

	print("SPLITTERS_OK: a bespoke model at the sim's tile, the feeding belt still plain, the")
	print("  next-output arrow pointing along the facing and one quarter turn clockwise for the")
	print("  branch, an unbuilt branch falling back to straight and a dead end hiding the arrow,")
	print("  items drawn crossing the tile, and hover naming both outputs")
	quit(0)


func _item_on_splitter() -> bool:
	for key in main._item_nodes.keys():
		if String(key).begins_with("splitter_") and main._item_nodes[key]["node"].visible:
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
	return String(state.get("next_output", "")) == "branch"
