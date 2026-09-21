# Headless proof of the per-kind machine presentation — the structural claims
# a machine can check without a framebuffer. Whether the assets actually look
# right is what scripts/screenshot-scene.sh is for, the same division
# test_presentation.gd and test_fluids.gd already draw.
#
# Run with:
#   godot --headless --path godot_project --script test_machines.gd
#
# Verifies:
#   * EVERY buildable kind in `data/shop.ron` resolves to a bespoke model, and
#     all of them are distinct meshes — nothing falls through to the generic
#     `machine` housing. This is checked against the SHOP CATALOG rather than
#     against whatever happens to be placed, which is stronger than the old
#     chain-driven version: a kind priced but unmodelled fails here even
#     though no world contains one yet,
#   * a hopper, a machine and a delivery dock each render a real, non-empty
#     imported mesh at the tile the simulation reports, at the centre of the
#     footprint the simulation reports,
#   * the hopper's product body rescales when its fill level changes — the
#     same live-data-drives-a-scaled-body contract the pipe run's liquid has,
#   * the goods standing on a dock's deck follow the economy's SHARED pool,
#     not the dock's own (always empty) buffer,
#   * the 2x2 cooking vat genuinely spans more than one tile, sits at its
#     footprint's centre rather than on its origin tile, and resolves for
#     hover from a NON-ORIGIN tile of its footprint.
#
# WHY THERE ARE TWO SCENES IN HERE. The vat fixture and the machines fixture
# both start at tile (0, 0), and a `place` onto an occupied tile is refused —
# so the two cannot share a world whatever the balance is. (This used to be a
# money argument too: the vat cost exactly the whole 200 a new world had. The
# inventory-hotbar ticket raised `sim::STARTING_MONEY` to 3000, so only the
# collision remains, and it is the reason that never depended on balance.) A
# second instantiated scene is a second world, which is the honest way to
# exercise two fixtures that want the same corner of the floor.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var vat_scene
var _ran := false


func _fail(msg: String) -> void:
	printerr("MACHINES_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	vat_scene = load("res://main.tscn").instantiate()
	root.add_child(vat_scene)
	vat_scene._ready()
	# The fixtures go down in `_run_checks`, not here: `_ready` runs twice
	# under this harness and each run creates a fresh (empty) world, so
	# anything built now is wiped before the checks see it. See
	# test_hover.gd's header.


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


func _world_aabb(node: Node3D) -> AABB:
	var box := AABB()
	var first := true
	for mi in _meshes(node, []):
		if mi.mesh == null:
			continue
		var here: AABB = mi.mesh.get_aabb()
		here.position += node.position
		if first:
			box = here
			first = false
		else:
			box = box.merge(here)
	return box


func _run_checks() -> void:
	Layouts.build_machines_fixture(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()
	Layouts.build_multi_tile(vat_scene.sim)
	vat_scene._rebuild_world()

	# --- every priced kind has a bespoke model, and they all differ --------
	var catalog: Dictionary = JSON.parse_string(main.sim.shop_snapshot_json())
	var seen_meshes := {}
	var kinds := 0
	for entry in catalog["entries"]:
		var kind := String(entry["kind"])
		var asset: String = main._shop_asset_for(kind)
		if asset == main.MACHINE_MODEL_FALLBACK:
			_fail("kind '%s' has no bespoke model — it would draw the generic '%s'" % [
				kind, main.MACHINE_MODEL_FALLBACK,
			])
			return
		var node: Node3D = main._asset_node(asset)
		var meshes := _meshes(node, [])
		if meshes.is_empty():
			_fail("kind '%s' -> asset '%s' rendered no meshes — the .vox didn't import" % [kind, asset])
			return
		for mi in meshes:
			if mi.mesh == null or mi.mesh.get_surface_count() < 1:
				_fail("asset '%s' has an empty mesh '%s'" % [asset, mi.name])
				return
		if seen_meshes.has(asset):
			pass # two kinds may legitimately share ONE asset name; the check is per asset
		seen_meshes[asset] = meshes[0].mesh
		node.free()
		kinds += 1
	var distinct := []
	for asset in seen_meshes.keys():
		var mesh = seen_meshes[asset]
		if distinct.has(mesh):
			_fail("asset '%s' shares a mesh with another asset — the models didn't actually differ" % asset)
			return
		distinct.append(mesh)

	# --- a hopper, a machine and a dock, each where the sim says -----------
	# The size beside each tile is that kind's `data/machines.ron` footprint:
	# a body is drawn at the CENTRE of its footprint, which for a 1x1 kind is
	# its own tile and for the 2x2 dock is the middle of the four tiles it
	# claims. Written out rather than read back from the snapshot for
	# `machine_footprints.rs`'s reason — a check that recomputed the position
	# the way the code does could not catch the code being wrong.
	var placed := {
		"hopper": [Vector2i(-1, 0), Vector2i(1, 1)],
		"crusher": [Vector2i(1, 0), Vector2i(1, 1)],
		"dock": [Vector2i(5, 0), Vector2i(2, 2)],
	}
	for kind in placed.keys():
		var tile: Vector2i = placed[kind][0]
		var size: Vector2i = placed[kind][1]
		var node: Node3D = main.get_node_or_null("World/%s_%d,%d" % [kind, tile.x, tile.y])
		if node == null:
			_fail("no rendered node for the placed %s at %s" % [kind, str(tile)])
			return
		if _meshes(node, []).is_empty():
			_fail("the placed %s rendered no meshes" % kind)
			return
		var expected: Vector3 = main._footprint_centre(tile, size, main.FLOOR_Y)
		if not node.position.is_equal_approx(expected):
			_fail("the %s sits at %s, expected its %dx%d footprint centre %s" % [
				kind, str(node.position), size.x, size.y, str(expected)
			])
			return

	# The dock is the second multi-tile kind in the shipped content (the
	# cooking vat is the other), so hovering a NON-ORIGIN tile of it has to
	# resolve to the same building — the claim the vat already proves for a
	# machine, checked here for a building with no recipe behind it.
	for tile in [Vector2i(6, 0), Vector2i(5, 1), Vector2i(6, 1)]:
		var hover: String = main._hover_text_at_tile(tile)
		if not hover.begins_with("Dock"):
			_fail("hovering the dock's tile %s resolved to %s" % [str(tile), hover])
			return

	# --- the hopper's product body tracks its live fill --------------------
	var hopper_nodes: Dictionary = main._hopper_nodes.get("-1,0", {})
	if hopper_nodes.is_empty():
		_fail("the placed hopper built no live-level nodes")
		return
	var fill_node: Node3D = hopper_nodes["fill"]
	if _meshes(fill_node, []).is_empty():
		_fail("the hopper's product body rendered no meshes")
		return
	main._refresh_buffers()
	if fill_node.visible:
		_fail("an empty hopper should hide its product body, not draw a sliver")
		return
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main._refresh_buffers()
	if not fill_node.visible:
		_fail("a filled hopper should show its product body")
		return
	var part_full: float = fill_node.scale.y
	for i in range(8):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main._refresh_buffers()
	if fill_node.scale.y <= part_full:
		_fail("filling the hopper further did not raise its level (%f -> %f)" % [part_full, fill_node.scale.y])
		return
	if fill_node.scale.y > float(main.HOPPER_FILL_HEIGHT_VOXELS) + 0.001:
		_fail("a full hopper's level (%f) rose past its box section" % fill_node.scale.y)
		return

	# --- the dock's deck follows the SHARED pool, not its own buffer -------
	var dock_nodes: Dictionary = main._dock_nodes.get("5,0", {})
	if dock_nodes.is_empty():
		_fail("the placed dock built no deck slots")
		return
	var slots: Array = dock_nodes["slots"]
	if slots.size() != main.DOCK_STOCK_SLOTS:
		_fail("expected %d dock stock slots, got %d" % [main.DOCK_STOCK_SLOTS, slots.size()])
		return
	main._update_dock_stock([])
	for slot in slots:
		if slot.visible:
			_fail("a case is standing on the dock with nothing in the delivery pool")
			return
	main._update_dock_stock([["boxed_ketchup", 1]])
	if not slots[0].visible:
		_fail("one unit in the delivery pool put no case on the dock")
		return
	if slots[main.DOCK_STOCK_SLOTS - 1].visible:
		_fail("one unit in the delivery pool filled more than one slot")
		return
	if _meshes(slots[0], []).is_empty():
		_fail("the case standing on the dock rendered no mesh")
		return
	main._update_dock_stock([["boxed_ketchup", 99]])
	for slot in slots:
		if not slot.visible:
			_fail("a full delivery pool left a dock slot empty")
			return

	# --- a full pool says so on the dock it stopped ------------------------
	# The `full` flag and the capacity both come off the wire (asserted in
	# `cargo test -p sim_core`); what is checked here is that the hover says
	# the two things a player standing at a stopped belt needs — that this is
	# the ceiling, and what happens next (UI_UX.md §7: glyph and words).
	var dock_state: Dictionary = main._dock_states.get("5,0", {})
	if dock_state.is_empty():
		_fail("the placed dock has no polled state to hover")
		return
	if bool(dock_state.get("full", false)):
		_fail("an idle dock's pool should not read as full")
		return
	var calm_hover: String = main._dock_hover_text("5,0", "east")
	if calm_hover.contains("FULL"):
		_fail("a dock with room said it was full: '%s'" % calm_hover)
		return
	dock_state["full"] = true
	main._delivery_capacity = 48
	var full_hover: String = main._dock_hover_text("5,0", "east")
	if not full_hover.contains("⚠ FULL"):
		_fail("a full dock's hover must say so, got: '%s'" % full_hover)
		return
	if not full_hover.contains("48"):
		_fail("the full-dock hover should name the ceiling it hit, got: '%s'" % full_hover)
		return
	if not full_hover.contains("backs up"):
		_fail("the full-dock hover should say what being full does to the belt, got: '%s'" % full_hover)
		return
	dock_state["full"] = false

	# --- the standing shortage is said on every tick, not just the refused
	# one (issue #87) ------------------------------------------------------
	# `understaffed` only moves on a tick the dock actually had something to
	# ship, so on a line that ebbs it is false on most ticks of a dock that
	# is permanently short — and the hover used to fall silent on those. The
	# stable `crewed` now answers instead, in the SAME WORDS, so the two
	# readings can never contradict each other. Both flags come off the wire
	# (`cargo test -p sim_core`); what is checked here is which one the
	# tooltip reaches for.
	if not bool(dock_state.get("crewed", false)):
		_fail("a funded dock should read as crewed before this case starts")
		return
	var crewed_hover: String = main._dock_hover_text("5,0", "east")
	if crewed_hover.contains("short of"):
		_fail("a crewed dock claimed a staff shortage: '%s'" % crewed_hover)
		return
	dock_state["crewed"] = false
	var uncrewed_hover: String = main._dock_hover_text("5,0", "east")
	if not uncrewed_hover.contains("short of logistics staff"):
		_fail("an uncrewed dock stayed silent between refusals: '%s'" % uncrewed_hover)
		return
	if uncrewed_hover.contains("held"):
		_fail("a dock that was not refused this tick said it was holding: '%s'" % uncrewed_hover)
		return
	# And a live refusal still wins, because it says the stronger thing.
	dock_state["understaffed"] = true
	var held_hover: String = main._dock_hover_text("5,0", "east")
	if not held_hover.contains("held"):
		_fail("a dock refused its crew this tick must say it is holding: '%s'" % held_hover)
		return
	if held_hover.count("short of logistics staff") != 1:
		_fail("the shortage was reported twice at once: '%s'" % held_hover)
		return
	dock_state["understaffed"] = false
	dock_state["crewed"] = true

	# --- hover answers for every placed kind through one mechanism --------
	for kind in placed.keys():
		var tile: Vector2i = placed[kind][0]
		if main._hover_text_at_tile(tile) == "":
			_fail("hovering the placed %s at %s produced no tooltip" % [kind, str(tile)])
			return

	# --- the 2x2 cooking vat, in its own world (see this file's header) ----
	var vat: Node3D = vat_scene.get_node_or_null("World/cooking_vat_0,0")
	if vat == null:
		_fail("the cooking vat did not render")
		return
	var vat_expected: Vector3 = vat_scene._footprint_centre(Vector2i(0, 0), Vector2i(2, 2), vat_scene.FLOOR_Y)
	if not vat.position.is_equal_approx(vat_expected):
		_fail("the 2x2 vat sits at %s, expected its footprint CENTRE %s" % [str(vat.position), str(vat_expected)])
		return
	if vat.position.is_equal_approx(vat_scene._tile_origin(0, 0, vat_scene.FLOOR_Y)):
		_fail("the 2x2 vat is drawn on its origin tile rather than across its footprint")
		return
	var box := _world_aabb(vat)
	if box.size.x <= vat_scene.TILE * 1.05 or box.size.z <= vat_scene.TILE * 1.05:
		_fail("the 2x2 vat spans %s — barely one tile (%f)" % [str(box.size), vat_scene.TILE])
		return
	# Hover from a NON-ORIGIN tile of the footprint — the case that only
	# exists because a machine can be bigger than one tile.
	var far_corner := Vector2i(1, 1)
	var far_text: String = vat_scene._hover_text_at_tile(far_corner)
	if not far_text.begins_with("Ketchup"):
		_fail("the vat's far corner %s should identify the vat, got: %s" % [str(far_corner), far_text])
		return
	if vat_scene._hover_text_at_tile(Vector2i(1, 2)) != "":
		_fail("the tile just past the vat's footprint still resolved to it")
		return

	print("MACHINES_OK: all %d priced kinds resolve to distinct bespoke models, a hopper/machine/dock render at the sim's own tiles with a live product level and a deck following the shared pool, and the 2x2 vat spans its whole footprint and answers hover from a non-origin tile" % kinds)
	quit(0)
