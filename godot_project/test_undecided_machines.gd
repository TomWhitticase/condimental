# Headless proof that a machine nothing has fed is drawn as handling ANY
# sauce — not as a ketchup machine (owner's request, 2026-10-02).
#
# Run with:
#   godot --headless --path godot_project --script test_undecided_machines.gd
#
# The simulation used to place every machine of a multi-recipe kind on the
# first recipe `data/recipes.ron` lists for it, which is ketchup's at every
# stage, so a freshly built line named ketchup in every hover and every
# configuration panel and painted every product mesh ketchup red until some
# other sauce's ingredients arrived. Now such a machine is UNDECIDED —
# `recipe: null` on the wire — and this checks each surface draws that:
#
#   1. HOVER: titled by the machine's kind plus MACHINE_UNDECIDED_TEXT, and
#      naming none of the recipes its kind can run; its load line says it is
#      waiting for ingredients rather than for one sauce's raw.
#   2. PANEL: the configuration panel's title says the same.
#   3. TINT: the product mesh (vat, bottler, packer) carries no override, so
#      it shows the voxel's own neutral pale.
#   4. And the decision still lands: fed, the vat names its recipe in the
#      hover and its product mesh takes a tint.
#
# The recipe names a machine must not show are read from `cookbook_json`'s
# steps, never listed here, so a fourth sauce is covered without an edit.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## How long the fed line may take to get the vat cooking. Measured at about
## 600 ticks; the margin is for a slow shared machine, not the line.
const DECIDE_TICKS := 4000

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("UNDECIDED_FAIL: %s" % msg)
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


func _repoll() -> void:
	main._apply_dirty_chunks()
	main._refresh_buffers()
	main._update_fluid_network()
	main._poll_machine_load()


## kind -> every recipe display name that kind can run, from the cookbook.
func _recipe_names_by_kind() -> Dictionary:
	var names := {}
	var book = JSON.parse_string(main.sim.cookbook_json())
	for sauce in book.get("sauces", []):
		for step in sauce.get("steps", []):
			var kind := String(step.get("machine", ""))
			if not names.has(kind):
				names[kind] = []
			names[kind].append(main._item_display_name(String(step.get("recipe", ""))))
	return names


func _machines() -> Array:
	var out := []
	for placement in JSON.parse_string(main.sim.world_layout_json()):
		if bool(placement.get("machine", false)):
			out.append(placement)
	return out


func _run_checks() -> void:
	Layouts.build_reference_line(main.sim)
	main._rebuild_world()
	_repoll()

	var names := _recipe_names_by_kind()
	var machines := _machines()
	if machines.size() < 5:
		_fail("the reference line placed %d machines, expected five" % machines.size())
		return
	var tinted_kinds := 0
	for placement in machines:
		var kind := String(placement["kind"])
		var origin: Vector2i = main._placement_origin(placement)
		var key: String = main._tile_key(origin)
		if placement.get("recipe") != null:
			_fail("a fresh %s reports recipe '%s' before anything fed it" % [kind, placement["recipe"]])
			return
		if (names.get(kind, []) as Array).size() < 2:
			continue # one recipe: nothing to decide, so not this test's subject

		# 1. Hover.
		var hover: String = main._hover_text_at_tile(origin)
		var expected_title := "%s  ·  %s" % [main._kind_display_name(kind), main.MACHINE_UNDECIDED_TEXT]
		if not hover.begins_with(expected_title):
			_fail("a fresh %s's hover opens '%s', expected '%s'" % [kind, hover.get_slice("\n", 0), expected_title])
			return
		for recipe_name in names[kind]:
			if hover.contains(String(recipe_name)):
				_fail("a fresh %s's hover names '%s', a sauce nothing has fed it:\n%s" % [kind, recipe_name, hover])
				return
		if not hover.contains("waiting for ingredients"):
			_fail("a fresh %s's hover does not say it is waiting for ingredients:\n%s" % [kind, hover])
			return

		# 2. Configuration panel.
		main._open_config_panel(origin, kind)
		main._update_config_panel()
		var title: String = main._config_title.text
		main._close_config_panel()
		if title != "%s — %s" % [main._kind_display_name(kind), main.MACHINE_UNDECIDED_TEXT]:
			_fail("a fresh %s's panel is titled '%s'" % [kind, title])
			return

		# 3. Tint.
		var mesh: MeshInstance3D = main._machine_products.get(key, null)
		if mesh != null:
			tinted_kinds += 1
			if mesh.get_surface_override_material(0) != null:
				_fail("a fresh %s's product mesh is tinted — it should show the neutral pale" % kind)
				return
	if tinted_kinds < 3:
		_fail("only %d fresh machines carry a product mesh; the vat, bottler and packer all should — \
the voxels are not staged (run cargo xtask)" % tinted_kinds)
		return

	# 4. Fed, the vat decides — and every surface follows it.
	var vat_placement: Dictionary = {}
	for placement in machines:
		if String(placement["kind"]) == "cooking_vat":
			vat_placement = placement
	var vat: Vector2i = main._placement_origin(vat_placement)
	var vat_key: String = main._tile_key(vat)
	Layouts.stock_larder(main.sim)
	var decided := ""
	for _i in range(DECIDE_TICKS):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
		main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank", "amount": 100}))
		main.sim.tick()
		var parsed = JSON.parse_string(main.sim.query_tile_json(vat.x, vat.y))
		if parsed != null and parsed.get("recipe") != null:
			decided = String(parsed["recipe"])
			break
	if decided == "":
		_fail("the fed vat never decided in %d ticks" % DECIDE_TICKS)
		return
	_repoll()
	var fed_hover: String = main._hover_text_at_tile(vat)
	if not fed_hover.begins_with(main._item_display_name(decided)):
		_fail("the fed vat's hover opens '%s', expected its recipe '%s'" % [fed_hover.get_slice("\n", 0), decided])
		return
	var vat_mesh: MeshInstance3D = main._machine_products.get(vat_key, null)
	if vat_mesh == null or vat_mesh.get_surface_override_material(0) == null:
		_fail("the fed vat's product mesh took no tint")
		return

	print("UNDECIDED_OK")
	quit(0)
