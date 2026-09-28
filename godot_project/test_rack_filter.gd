# Headless proof of the warehouse rack's configuration panel: a click on a
# rack opens it, and its picker reserves the rack for one raw ingredient
# (`set_rack_filter`).
#
# Run with:
#   godot --headless --path godot_project --script test_rack_filter.gd
#
# Verifies:
#   * a click on either tile of a rack opens ITS panel, with the picker up and
#     FILL and AUTO-FILL hidden — a rack's stock is the larder's, bought at
#     the market, so there is nothing on it to fill,
#   * the picker's slots are "any" followed by every raw the INGREDIENTS
#     snapshot lists (solids, then fluids), read off the wire here rather
#     than off `main`'s own copy, so a new raw appears with no Godot edit,
#   * a gated raw is padlocked and refused (UI_UX §5g), and once its gate is
#     crossed the open panel drops the padlock and the raw can be reserved,
#   * picking a raw reserves the rack in the simulation and the title, the
#     tick and the hover line all follow the simulation's answer,
#   * the market's warehouse line counts a reserved rack's empty bays apart
#     from the free ones (the wire's `free_bays`), not as room for anything,
#   * a pick the simulation would refuse for want of room is dimmed with a ✗
#     BEFORE the click and explained in words under the picker, and the
#     click then changes nothing,
#   * "Any" clears the reservation,
#   * removal's ✗ follows the snapshot's `removal_refused`, which the panel
#     and the ghost share,
#   * and a hopper's panel opened after a rack's still has its FILL button
#     and its CONTENTS heading — the two kinds share one panel.
#
# What it does not assert is how the panel LOOKS — that is
# `scripts/screenshot-scene.sh --warehouse --config=0,0 --contents 1`.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## Well away from anything else, and a hopper beside it for the last check.
const RACK := Vector2i(0, 6)
const HOPPER := Vector2i(-4, 6)

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("RACK_FILTER_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	# The fixture goes down in `_run_checks`: `_ready` runs twice under this
	# harness and each run makes a fresh world (see test_hover.gd).


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


func _repoll() -> void:
	main._refresh_buffers()
	main._update_config_panel()


func _rack() -> Dictionary:
	for rack in JSON.parse_string(main.sim.storage_snapshot_json()).get("racks", []):
		var at: Array = rack["tile"]
		if int(at[0]) == RACK.x and int(at[1]) == RACK.y:
			return rack
	return {}


func _filter() -> String:
	var value = _rack().get("filter")
	return String(value) if value != null else ""


func _slot_index(item: String) -> int:
	for index in range(main._config_picker_slots.size()):
		if String(main._config_picker_slots[index]["item"]) == item:
			return index
	return -1


func _run_checks() -> void:
	Layouts.fund(main.sim, 5000)
	var ingredients = JSON.parse_string(main.sim.ingredients_snapshot_json())
	var solids: Array = ingredients.get("hopper_items", [])
	var fluids: Array = ingredients.get("tank_fluids", [])
	# The ids a new game can buy, in wire order. Which two are used below is
	# the wire's choice, not this file's.
	var open: Array[String] = []
	var locked: Array[String] = []
	for entry in solids + fluids:
		var id := String(entry["id"])
		if bool(entry.get("locked", false)):
			locked.append(id)
		else:
			open.append(id)
	if open.size() < 2:
		_fail("a new game can buy fewer than two raws (%s) — nothing to reserve a rack between" % str(open))
		return
	var first := open[0]
	var second := open[1]

	Layouts.place(main.sim, Layouts.WAREHOUSE_KIND, RACK, "east")
	Layouts.place(main.sim, "hopper", HOPPER, "east", first)
	main._rebuild_world()
	main._refresh_buffers()
	if _rack().is_empty():
		_fail("no rack at %s — the fixture did not build" % str(RACK))
		return

	# --- The click opens the rack's panel, from either tile -----------------
	main._config_click_at_tile(RACK + Vector2i(0, 1))
	_repoll()
	if not main._config_open or main._config_kind != Layouts.WAREHOUSE_KIND or main._config_tile != RACK:
		_fail("a click on the rack's far tile opened %s at %s" % [main._config_kind, str(main._config_tile)])
		return
	if not main._config_picker_box.visible:
		_fail("a rack's panel hides its picker")
		return
	if main._config_fill_button.visible or main._config_auto_button.visible:
		_fail("a rack's panel offers FILL or AUTO-FILL — a rack has nothing to fill")
		return
	if main._config_picker_heading.text != "RESERVED FOR":
		_fail("a rack's picker is headed '%s'" % main._config_picker_heading.text)
		return
	var bays := int(_rack().get("bays", 0))
	if main._config_level_label.text != "0 of %d bays in use" % bays:
		_fail("an empty rack's level reads '%s'" % main._config_level_label.text)
		return

	# --- The slots are "any" and then the wire's raws -----------------------
	var expected: Array = [""]
	for entry in solids + fluids:
		expected.append(String(entry["id"]))
	var got: Array = []
	for slot in main._config_picker_slots:
		got.append(String(slot["item"]))
	if got != expected:
		_fail("the rack's picker offers %s, the wire lists %s" % [str(got), str(expected)])
		return
	if not main._config_title.text.ends_with("any stock"):
		_fail("an open rack's title reads '%s'" % main._config_title.text)
		return
	if not main._config_picker_slots[0]["mark"].visible:
		_fail("an open rack does not tick its 'Any' slot")
		return
	if not main._hover_text_at_tile(RACK).contains("open to any stock"):
		_fail("an open rack's hover does not say so:\n%s" % main._hover_text_at_tile(RACK))
		return

	# --- Picking a raw reserves the rack ------------------------------------
	main._on_config_picker_pressed(_slot_index(first))
	_repoll()
	if _filter() != first:
		_fail("picking '%s' left the rack's filter at '%s'" % [first, _filter()])
		return
	var title := "%s — %s only" % [main._kind_display_name(Layouts.WAREHOUSE_KIND), main._item_display_name(first)]
	if main._config_title.text != title:
		_fail("a reserved rack's title reads '%s', expected '%s'" % [main._config_title.text, title])
		return
	if not main._config_picker_slots[_slot_index(first)]["mark"].visible \
			or main._config_picker_slots[0]["mark"].visible:
		_fail("the tick is not on '%s' alone" % first)
		return
	if not main._hover_text_at_tile(RACK).contains("reserved for %s" % main._item_display_name(first)):
		_fail("a reserved rack's hover does not name what for:\n%s" % main._hover_text_at_tile(RACK))
		return
	# The market's warehouse line does not promise a reserved rack's empty
	# bays to every crate: the only rack is reserved, so nothing is free.
	if int(main._storage_state.get("free_bays", -1)) != 0:
		_fail("the only rack is reserved, yet the wire calls %s bays free" % str(main._storage_state.get("free_bays")))
		return
	main._update_market_storage()
	var market_line: String = main._market_storage_label.text
	if not market_line.contains("·  0 free") \
			or not market_line.contains("%d empty on reserved shelving" % bays):
		_fail("the market's warehouse line counts reserved bays as free:\n%s" % market_line)
		return

	# --- A padlock is refused until its gate opens -------------------------
	# A new game has gated raws; this is not conditional on the content, so a
	# gate table that stopped reaching the rack's picker cannot pass by having
	# nothing to check.
	if locked.is_empty():
		_fail("a new game has no gated raw — the gate half of this test has nothing to check")
		return
	for id in locked:
		if not main._config_picker_slots[_slot_index(id)]["lock"].visible:
			_fail("the gated '%s' has no padlock on the rack's picker" % id)
			return
	if not main._config_picker_note.text.contains("🔒 "):
		_fail("the padlocks are not explained in words under the picker:\n%s" % main._config_picker_note.text)
		return
	main._on_config_picker_pressed(_slot_index(locked[0]))
	_repoll()
	if _filter() != first:
		_fail("a padlocked '%s' was reserved" % locked[0])
		return
	Layouts.cross_every_gate(main.sim, 0)
	main._rebuild_world()
	_repoll()
	if not main._config_open or main._config_kind != Layouts.WAREHOUSE_KIND:
		_fail("the rack's panel closed when the gates opened")
		return
	for id in locked:
		if main._config_picker_slots[_slot_index(id)]["lock"].visible:
			_fail("'%s' opened but the rack's open picker still padlocks it" % id)
			return
	main._on_config_picker_pressed(_slot_index(locked[0]))
	_repoll()
	if _filter() != locked[0]:
		_fail("the newly opened '%s' could not be reserved (rack is '%s')" % [locked[0], _filter()])
		return
	# Back to the first pick, which the rest of this file fills the rack with.
	main._on_config_picker_pressed(_slot_index(first))
	_repoll()
	if _filter() != first:
		_fail("an empty rack could not go back from '%s' to '%s'" % [locked[0], first])
		return

	# --- A pick that would strand stock is marked before the click ----------
	var storage = JSON.parse_string(main.sim.storage_snapshot_json())
	var fill := int(storage.get("base_cap", 0)) + int(storage.get("slot_stack", 0)) * bays
	# Bought directly, not through `Layouts.stock`: that builds racks to make
	# room, and a second rack's open bays are exactly where this stock could
	# go instead.
	main.sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": first, "quantity": fill,
	}))
	_repoll()
	var blocked: Array = _rack().get("blocked_filters", [])
	if not blocked.has(second):
		_fail("a rack full of '%s' could be switched to '%s' by the wire's own account (blocked %s)" % [first, second, str(blocked)])
		return
	var slot: Dictionary = main._config_picker_slots[_slot_index(second)]
	if not slot["blocked"].visible or slot["column"].modulate.a >= 1.0:
		_fail("the '%s' slot is not dimmed with its ✗ although the sim would refuse it" % second)
		return
	if main._config_picker_slots[_slot_index(first)]["blocked"].visible:
		_fail("the current pick carries a ✗")
		return
	if not main._config_picker_note.text.contains("✗ ") \
			or not main._config_picker_note.text.contains(main._item_display_name(second)):
		_fail("the ✗ is not explained in words under the picker:\n%s" % main._config_picker_note.text)
		return
	if not main._config_stock_label.text.contains(main._item_display_name(first)):
		_fail("the panel does not say what is on the rack: '%s'" % main._config_stock_label.text)
		return
	main._on_config_picker_pressed(_slot_index(second))
	_repoll()
	if _filter() != first:
		_fail("a pick marked ✗ went through — the rack is now '%s'" % _filter())
		return
	# And the ghost reads the same answer the snapshot gives.
	if not main._rack_removal_refused(main._tile_key(RACK)) \
			or not main._hover_text_at_tile(RACK).contains("HOLDING"):
		_fail("a rack whose reserved bays hold stock with nowhere else to go is not refused")
		return

	# --- "Any" clears it ----------------------------------------------------
	main._on_config_picker_pressed(0)
	_repoll()
	if _filter() != "":
		_fail("'Any' left the rack reserved for '%s'" % _filter())
		return
	if not main._config_title.text.ends_with("any stock"):
		_fail("a cleared rack's title reads '%s'" % main._config_title.text)
		return

	# --- A hopper's panel after a rack's is still a hopper's ----------------
	main._config_click_at_tile(HOPPER)
	_repoll()
	if main._config_kind != "hopper":
		_fail("a click on the hopper opened %s" % main._config_kind)
		return
	if not main._config_fill_button.visible or not main._config_auto_button.visible \
			or main._config_picker_heading.text != "CONTENTS":
		_fail("a hopper's panel opened after a rack's lost FILL, AUTO-FILL or its heading")
		return

	print("RACK_FILTER_OK: a click on the rack opens its panel with %d slots (any + the wire's raws) and no FILL; '%s' reserves it and the title, tick and hover follow; %s are padlocked and refused until their gates open, and then '%s' can be reserved; with it full, '%s' is dimmed with a ✗ and explained before the click the sim then refuses; 'Any' clears it; and a hopper's panel after it is still a hopper's" % [
		expected.size(), first, str(locked), locked[0], second,
	])
	quit(0)
