# Headless proof that a machine with NO RECIPE CHOSEN is drawn as making
# nothing yet — not as a ketchup machine — and that the configuration panel's
# MAKES picker is how a recipe is chosen (owner's rulings of 2026-10-02 and
# 2026-10-03).
#
# Run with:
#   godot --headless --path godot_project --script test_undecided_machines.gd
#
# A machine runs the recipe the player set on it and nothing else; one placed
# without a recipe is `recipe: null` on the wire and makes nothing, however
# it is fed. Checks, for every machine kind with more than one recipe:
#
#   1. HOVER: titled with the machine's kind plus MACHINE_UNDECIDED_TEXT,
#      naming none of the recipes the kind can run; the load line says no
#      recipe is chosen and how to choose one.
#   2. PANEL: the configuration panel's title says the same, and it carries
#      a MAKES picker with one slot per recipe of the kind, none ticked.
#   3. TINT: a product mesh (vat, bottler, packer) carries no override, so it
#      shows the voxel's own neutral pale.
#   4. CHOOSING: pressing a slot in the vat's picker sets that recipe — the
#      tick moves to it, the hover leads with it, the product mesh takes its
#      tint — and the change survives a re-read of the wire.
#
# The recipes each kind can run are read from `machine_recipes_json`, never
# listed here, so a fourth sauce is covered without an edit.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## Where the machines under test stand: one per multi-recipe kind, spaced
## wide enough that no footprint touches another.
const ROW_Y := 0
const SPACING := 6

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


## Every recipe a machine of `kind` can be set to, off the wire.
func _recipes_of(kind: String) -> Array:
	var parsed = JSON.parse_string(main.sim.machine_recipes_json(kind))
	return parsed if parsed is Array else []


func _run_checks() -> void:
	# Every machine kind the content gives a choice, placed with none chosen.
	var kinds: Array = []
	for kind in ["crusher", "mixer", "cooking_vat", "bottler", "packer"]:
		if _recipes_of(kind).size() > 1:
			kinds.append(kind)
	if kinds.size() < 4:
		_fail("only %s offer a choice of recipe; the shipped content gives one at the crusher, mixer, vat, bottler and packer" % str(kinds))
		return
	var origins := {}
	for i in range(kinds.size()):
		var origin := Vector2i(i * SPACING, ROW_Y)
		Layouts.place(main.sim, kinds[i], origin, "east")
		origins[kinds[i]] = origin
	main.sim.tick()
	main._rebuild_world()
	_repoll()

	var tinted_kinds := 0
	for kind in kinds:
		var origin: Vector2i = origins[kind]
		var key: String = main._tile_key(origin)
		var placement: Dictionary = main._placement_at_tile(origin)
		if String(placement.get("kind", "")) != kind:
			_fail("the %s at %s did not place" % [kind, str(origin)])
			return
		if placement.get("recipe") != null:
			_fail("a %s placed with no recipe reports '%s'" % [kind, placement["recipe"]])
			return
		var recipe_names: Array = []
		for recipe in _recipes_of(kind):
			recipe_names.append(main._item_display_name(String(recipe)))

		# 1. Hover.
		var hover: String = main._hover_text_at_tile(origin)
		var expected_title := "%s  ·  %s" % [main._kind_display_name(kind), main.MACHINE_UNDECIDED_TEXT]
		if not hover.begins_with(expected_title):
			_fail("an unset %s's hover opens '%s', expected '%s'" % [kind, hover.get_slice("\n", 0), expected_title])
			return
		for recipe_name in recipe_names:
			if hover.contains(String(recipe_name)):
				_fail("an unset %s's hover names '%s', a recipe nobody chose:\n%s" % [kind, recipe_name, hover])
				return
		if not hover.contains("no recipe chosen"):
			_fail("an unset %s's hover does not say no recipe is chosen:\n%s" % [kind, hover])
			return

		# 2. Configuration panel, and its picker.
		main._open_config_panel(origin, kind)
		main._update_config_panel()
		var title: String = main._config_title.text
		var picker_shown: bool = main._config_picker_box.visible
		var heading: String = main._config_picker_heading.text
		var slots: Array = main._config_picker_slots.duplicate()
		var ticked := 0
		for slot in slots:
			if slot["mark"].visible:
				ticked += 1
		main._close_config_panel()
		if title != "%s — %s" % [main._kind_display_name(kind), main.MACHINE_UNDECIDED_TEXT]:
			_fail("an unset %s's panel is titled '%s'" % [kind, title])
			return
		if not picker_shown or heading != "MAKES":
			_fail("an unset %s's panel has no MAKES picker (shown %s, heading '%s')" % [kind, picker_shown, heading])
			return
		if slots.size() != _recipes_of(kind).size():
			_fail("the %s's picker has %d slots for %d recipes" % [kind, slots.size(), _recipes_of(kind).size()])
			return
		if ticked != 0:
			_fail("an unset %s's picker ticks %d slot(s)" % [kind, ticked])
			return

		# 3. Tint.
		var mesh: MeshInstance3D = main._machine_products.get(key, null)
		if mesh != null:
			tinted_kinds += 1
			if mesh.get_surface_override_material(0) != null:
				_fail("an unset %s's product mesh is tinted — it should show the neutral pale" % kind)
				return
	if tinted_kinds < 3:
		_fail("only %d machines carry a product mesh; the vat, bottler and packer all should — \
the voxels are not staged (run cargo xtask)" % tinted_kinds)
		return

	# 4. Choosing, through the panel's own picker, as a click does.
	var vat: Vector2i = origins["cooking_vat"]
	var vat_key: String = main._tile_key(vat)
	var choice := String(_recipes_of("cooking_vat")[0])
	main._open_config_panel(vat, "cooking_vat")
	var index := -1
	for i in range(main._config_picker_slots.size()):
		if String(main._config_picker_slots[i]["item"]) == choice:
			index = i
	if index < 0:
		_fail("the vat's picker has no slot for '%s'" % choice)
		return
	main._on_config_picker_pressed(index)
	main._update_config_panel()
	if not main._config_picker_slots[index]["mark"].visible:
		_fail("pressing '%s' did not tick it in the vat's picker" % choice)
		return
	main._close_config_panel()
	var parsed = JSON.parse_string(main.sim.query_tile_json(vat.x, vat.y))
	if not (parsed is Dictionary) or String(parsed.get("recipe", "")) != choice:
		_fail("pressing '%s' left the vat's recipe at %s on the wire" % [choice, str(parsed)])
		return
	_repoll()
	var set_hover: String = main._hover_text_at_tile(vat)
	if not set_hover.begins_with(main._item_display_name(choice)):
		_fail("the set vat's hover opens '%s', expected its recipe '%s'" % [set_hover.get_slice("\n", 0), choice])
		return
	var vat_mesh: MeshInstance3D = main._machine_products.get(vat_key, null)
	if vat_mesh == null or vat_mesh.get_surface_override_material(0) == null:
		_fail("the set vat's product mesh took no tint")
		return

	print("UNDECIDED_OK: %d machine kinds placed with no recipe name none and offer a MAKES picker of their own recipes; choosing '%s' in the vat's picker set it, ticked it, titled it and tinted it" % [kinds.size(), choice])
	quit(0)
