# Headless proof of the hopper/tank click-to-configure panel (the second
# epic of the save/load + config UI + tuning plan; UI_UX.md §5's
# "machine-side factory UI"). Before it, keeping a factory fed was three
# global keys — SPACE loaded every hopper, V every tank, Q cycled what a
# hopper would hold — none of which was about ONE building. Now a placed
# hopper or tank is clicked, and everything about it is on a panel beside it.
#
# Run with:
#   godot --headless --path godot_project --script test_config_panel.gd
#
# What it asserts, against a world it builds itself through real
# purchase/place commands (test_layouts.gd):
#
#   * a click on a placed hopper opens its panel with the right title, level
#     and picker; the same click again closes it, a click on the OTHER hopper
#     retargets it, and a click on empty floor, on a machine, or on no tile
#     at all closes it,
#   * FILL is disabled with a reason in words while the larder is empty, and
#     loads exactly one fill amount of THAT hopper once stock is bought,
#   * the AUTO-FILL switch issues the new command, reads its state back off
#     the simulation's own snapshot (ON/OFF in a word, never colour alone),
#     and a switched-on hopper is genuinely kept full by the tick,
#   * the item picker retargets a placed hopper and marks the chosen slot,
#   * ESC closes the panel FIRST, without reaching the pause menu, and the
#     panel closes itself when the build menu, the market, build mode or
#     removal of the building takes the screen,
#   * a tank's panel names the fluid its run carries and fills the tank, an
#     unplumbed tank says "Not connected to a pipe run", and the picker is
#     hopper-only,
#   * SPACE, V and Q no longer touch any hopper or tank.
#
# What it does not assert is how the panel LOOKS — that is
# `scripts/screenshot-scene.sh --build --config=-1,0` and reading the PNG.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

# Two hoppers far enough apart to be two different clicks, well away from
# the fluid fixture (mixer at (0, 0)/(0, 1), pipe run on row 2, tank at
# (4, 2)) so nothing collides.
const HOPPER_A := Vector2i(-4, 0)
const HOPPER_B := Vector2i(-4, 4)
const TANK := Vector2i(4, 2)
const LONE_TANK := Vector2i(12, 12)
const MIXER_NON_ORIGIN := Vector2i(0, 1)
const EMPTY_FLOOR := Vector2i(20, 20)

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("CONFIG_PANEL_FAIL: %s" % msg)
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


func _hopper_snapshot(tile: Vector2i) -> Dictionary:
	for hopper in JSON.parse_string(main.sim.hopper_snapshots_json()):
		var at: Array = hopper["tile"]
		if int(at[0]) == tile.x and int(at[1]) == tile.y:
			return hopper
	return {}


func _tank_snapshot(tile: Vector2i) -> Dictionary:
	for tank in JSON.parse_string(main.sim.fluid_tank_snapshots_json()):
		var at: Array = tank["tile"]
		if int(at[0]) == tile.x and int(at[1]) == tile.y:
			return tank
	return {}


func _owned(item: String) -> int:
	return int(main._owned_ingredient_counts().get(item, 0))


func _press_key(keycode: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


## The panel is repolled by `_process` in the live scene; a test has no
## frames, so it polls the same way by hand after anything it changes.
func _repoll() -> void:
	main._refresh_buffers()
	main._update_config_panel()


func _run_checks() -> void:
	# --- The fixture: two hoppers, a plumbed tank, an unplumbed one --------
	# `_ready` runs twice under this harness, so the world is built here
	# rather than in `_initialize` (see test_fluids.gd).
	var items: Array = main._hopper_items if not main._hopper_items.is_empty() else []
	main._refresh_hopper_items()
	items = main._hopper_items
	if items.size() < 2:
		_fail("the shipped content has fewer than two raw solids (%s) — the picker has nothing to choose between" % str(items))
		return
	var first_item := String(items[0])
	var second_item := String(items[1])

	Layouts.place(main.sim, "hopper", HOPPER_A, "east", first_item)
	Layouts.place(main.sim, "hopper", HOPPER_B, "east", first_item)
	Layouts.build_fluid_run(main.sim)
	Layouts.place(main.sim, "fluid_tank", LONE_TANK, "east")
	main._rebuild_world()
	main._update_fluid_network()
	_repoll()

	for tile in [HOPPER_A, HOPPER_B]:
		if String(main._placement_at_tile(tile).get("kind", "")) != "hopper":
			_fail("no hopper placed at %s — the fixture did not build" % str(tile))
			return
	if String(main._placement_at_tile(TANK).get("kind", "")) != "fluid_tank":
		_fail("no fluid tank at %s — build_fluid_run did not build" % str(TANK))
		return
	if String(main._placement_at_tile(LONE_TANK).get("kind", "")) != "fluid_tank":
		_fail("no fluid tank at %s — the unplumbed tank did not place" % str(LONE_TANK))
		return

	# --- Nothing is open until something is clicked -------------------------
	if main._config_open or main._config_panel == null or main._config_panel.visible:
		_fail("the configuration panel is open before anything was clicked")
		return

	# --- 1. A click on a hopper opens ITS panel -----------------------------
	main._config_click_at_tile(HOPPER_A)
	if not main._config_open or not main._config_panel.visible:
		_fail("clicking hopper A did not open the panel")
		return
	if main._config_kind != "hopper" or main._config_tile != HOPPER_A:
		_fail("the panel is about %s at %s, not the hopper at %s" % [
			main._config_kind, str(main._config_tile), str(HOPPER_A)
		])
		return
	var expected_title := "Hopper — %s" % main._item_display_name(first_item)
	if main._config_title.text != expected_title:
		_fail("hopper title is '%s', expected '%s'" % [main._config_title.text, expected_title])
		return
	var snap := _hopper_snapshot(HOPPER_A)
	if snap.is_empty():
		_fail("hopper A has no snapshot on the wire")
		return
	var capacity := int(snap.get("capacity", 0))
	if capacity <= 0:
		_fail("hopper A reports capacity %d" % capacity)
		return
	if main._config_level_label.text != "0 / %d units" % capacity:
		_fail("a fresh hopper's level reads '%s', expected '0 / %d units'" % [
			main._config_level_label.text, capacity
		])
		return
	if main._config_level_fill.visible:
		_fail("the level bar is drawn on an empty hopper")
		return
	if not main._config_picker_box.visible:
		_fail("a hopper's panel hides the item picker")
		return
	if main._config_picker_slots.size() != items.size():
		_fail("the picker offers %d slots for %d raw solids" % [
			main._config_picker_slots.size(), items.size()
		])
		return
	for slot in main._config_picker_slots:
		var is_current: bool = String(slot["item"]) == first_item
		if slot["mark"].visible != is_current:
			_fail("picker slot '%s' is %smarked while the hopper holds '%s'" % [
				slot["item"], "" if slot["mark"].visible else "not ", first_item
			])
			return

	# --- 2. FILL with nothing in the larder is refused, in words ------------
	if _owned(first_item) != 0:
		_fail("the test expects an empty larder to start (owns %d %s)" % [_owned(first_item), first_item])
		return
	if not main._config_fill_button.disabled:
		_fail("FILL is enabled with no %s in stock" % first_item)
		return
	var expected_reason := "No %s in stock — buy some at the Ingredient Market (M)" % main._item_display_name(first_item)
	if not main._config_fill_reason.visible or main._config_fill_reason.text != expected_reason:
		_fail("blocked FILL says '%s' (visible=%s), expected '%s'" % [
			main._config_fill_reason.text, main._config_fill_reason.visible, expected_reason
		])
		return
	if main._config_stock_label.text != "in stock: 0":
		_fail("stock line reads '%s', expected 'in stock: 0'" % main._config_stock_label.text)
		return
	# Even a press that somehow gets through changes nothing: the simulation
	# refuses a fill with no stock, and the panel says so.
	main._on_config_fill_pressed()
	if int(_hopper_snapshot(HOPPER_A).get("quantity", 0)) != 0:
		_fail("a fill with an empty larder put something in the hopper")
		return

	# --- 3. With stock bought, FILL loads ONE fill amount of THIS hopper ----
	Layouts.stock_larder(main.sim)
	_repoll()
	var owned_before := _owned(first_item)
	if owned_before <= 0:
		_fail("stock_larder bought no %s" % first_item)
		return
	if main._config_fill_button.disabled or main._config_fill_reason.visible:
		_fail("FILL is still blocked ('%s') with %d %s in stock" % [
			main._config_fill_reason.text, owned_before, first_item
		])
		return
	if main._config_stock_label.text != "in stock: %d" % owned_before:
		_fail("stock line reads '%s', expected 'in stock: %d'" % [
			main._config_stock_label.text, owned_before
		])
		return
	main._on_config_fill_pressed()
	var loaded := int(_hopper_snapshot(HOPPER_A).get("quantity", 0))
	if loaded <= 0 or loaded >= capacity:
		_fail("one FILL loaded %d of %d — expected one fill amount, not none and not all" % [loaded, capacity])
		return
	if _owned(first_item) != owned_before - loaded:
		_fail("FILL debited %d from the larder for %d loaded" % [owned_before - _owned(first_item), loaded])
		return
	if int(_hopper_snapshot(HOPPER_B).get("quantity", 0)) != 0:
		_fail("FILL on hopper A's panel also loaded hopper B — the old global SPACE behaviour")
		return
	if main._config_level_label.text != "%d / %d units" % [loaded, capacity]:
		_fail("level reads '%s' after the fill, expected '%d / %d units'" % [
			main._config_level_label.text, loaded, capacity
		])
		return
	if not main._config_level_fill.visible or main._config_level_fill.size.x <= 0.0:
		_fail("the level bar is not drawn on a part-full hopper")
		return

	# --- 4. AUTO-FILL: the switch, read back off the snapshot ----------------
	if bool(_hopper_snapshot(HOPPER_A).get("auto_fill", true)):
		_fail("a freshly placed hopper reports auto_fill on")
		return
	if main._config_auto_button.text != "AUTO-FILL:  OFF" or main._config_auto_enabled:
		_fail("the switch reads '%s' for a manual hopper, expected 'AUTO-FILL:  OFF'" % main._config_auto_button.text)
		return
	main._on_config_auto_pressed()
	if not bool(_hopper_snapshot(HOPPER_A).get("auto_fill", false)):
		_fail("pressing AUTO-FILL did not set the hopper's auto_fill flag on the wire")
		return
	if main._config_auto_button.text != "AUTO-FILL:  ON" or not main._config_auto_enabled:
		_fail("the switch reads '%s' after turning on, expected 'AUTO-FILL:  ON'" % main._config_auto_button.text)
		return
	if bool(_hopper_snapshot(HOPPER_B).get("auto_fill", false)):
		_fail("turning auto-fill on for hopper A also armed hopper B")
		return
	# The tick keeps it full — to capacity, out of stock, with no key pressed.
	for _i in range(3):
		main.sim.tick()
	main._apply_dirty_chunks()
	_repoll()
	var topped := int(_hopper_snapshot(HOPPER_A).get("quantity", 0))
	if topped != capacity:
		_fail("an auto-fill hopper holds %d of %d after ticking" % [topped, capacity])
		return
	if main._config_level_label.text != "%d / %d units" % [capacity, capacity]:
		_fail("level reads '%s' on a full hopper" % main._config_level_label.text)
		return
	if not main._config_fill_button.disabled or main._config_fill_reason.text != "Full — nothing more will fit":
		_fail("a full hopper's FILL is not blocked with 'Full — nothing more will fit' (got '%s', disabled=%s)" % [
			main._config_fill_reason.text, main._config_fill_button.disabled
		])
		return
	main._on_config_auto_pressed()
	if bool(_hopper_snapshot(HOPPER_A).get("auto_fill", true)) or main._config_auto_button.text != "AUTO-FILL:  OFF":
		_fail("pressing AUTO-FILL again did not turn it off")
		return

	# --- 5. The picker retargets a PLACED hopper ----------------------------
	var second_index := -1
	for index in range(main._config_picker_slots.size()):
		if String(main._config_picker_slots[index]["item"]) == second_item:
			second_index = index
			break
	if second_index < 0:
		_fail("the picker has no slot for '%s'" % second_item)
		return
	main._on_config_picker_pressed(second_index)
	if String(_hopper_snapshot(HOPPER_A).get("item", "")) != second_item:
		_fail("the picker did not retarget hopper A to '%s' (holds '%s')" % [
			second_item, _hopper_snapshot(HOPPER_A).get("item", "")
		])
		return
	if main._config_title.text != "Hopper — %s" % main._item_display_name(second_item):
		_fail("the title did not follow the retarget: '%s'" % main._config_title.text)
		return
	for slot in main._config_picker_slots:
		if slot["mark"].visible != (String(slot["item"]) == second_item):
			_fail("the picker's mark did not move to '%s'" % second_item)
			return
	if String(_hopper_snapshot(HOPPER_B).get("item", "")) != first_item:
		_fail("retargeting hopper A changed hopper B")
		return

	# --- 6. The click is a toggle; another building retargets --------------
	main._config_click_at_tile(HOPPER_A)
	if main._config_open or main._config_panel.visible:
		_fail("clicking the open hopper again did not close its panel")
		return
	main._config_click_at_tile(HOPPER_A)
	main._config_click_at_tile(HOPPER_B)
	if not main._config_open or main._config_tile != HOPPER_B or main._config_kind != "hopper":
		_fail("clicking hopper B while A's panel was up did not retarget the panel (open=%s, tile=%s)" % [
			main._config_open, str(main._config_tile)
		])
		return
	if main._config_title.text != "Hopper — %s" % main._item_display_name(first_item):
		_fail("the retargeted panel still shows hopper A's title: '%s'" % main._config_title.text)
		return
	# Any other kind closes it, from any tile of its footprint.
	main._config_click_at_tile(MIXER_NON_ORIGIN)
	if main._config_open:
		_fail("clicking the mixer left a hopper's panel open")
		return
	main._config_click_at_tile(HOPPER_B)
	main._config_click_at_tile(EMPTY_FLOOR)
	if main._config_open:
		_fail("clicking empty floor left the panel open")
		return
	main._config_click_at_tile(HOPPER_B)
	main._config_click_at_tile(main.HOVER_NO_TILE)
	if main._config_open:
		_fail("a click resolving to no tile at all left the panel open")
		return

	# --- 7. ESC closes the panel first, and only the panel -----------------
	main._config_click_at_tile(HOPPER_B)
	_press_key(KEY_ESCAPE)
	if main._config_open:
		_fail("ESC did not close the panel")
		return
	if main._pause_open:
		_fail("ESC with a panel open fell through to the pause menu")
		return

	# --- 8. A modal panel or a mode takes the screen; removal takes it away -
	main._config_click_at_tile(HOPPER_B)
	main._set_shop_menu_open(true)
	_repoll()
	if main._config_open:
		_fail("the build menu opened over a live configuration panel")
		return
	main._set_shop_menu_open(false)

	main._config_click_at_tile(HOPPER_B)
	main._set_market_open(true)
	_repoll()
	if main._config_open:
		_fail("the market opened over a live configuration panel")
		return
	main._set_market_open(false)

	# Build mode needs something in hand, so buy one belt first (it is never
	# placed; the ghost alone is enough to be "in" a mode).
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	main._refresh_owned_counts()
	main._config_click_at_tile(HOPPER_B)
	main._enter_build_mode("belt")
	if not main._in_build_mode():
		_fail("could not enter build mode with a belt bought")
		return
	_repoll()
	if main._config_open:
		_fail("entering build mode left the configuration panel open")
		return
	main._exit_build_mode()

	main._config_click_at_tile(HOPPER_B)
	main._toggle_remove_mode()
	if not main._remove_mode:
		_fail("could not enter remove mode")
		return
	_repoll()
	if main._config_open:
		_fail("entering remove mode left the configuration panel open")
		return
	main._exit_build_mode()

	main._config_click_at_tile(HOPPER_B)
	main._issue_world_command({"type": "remove", "tile": [HOPPER_B.x, HOPPER_B.y]})
	_repoll()
	if main._config_open:
		_fail("removing the hopper left its panel open")
		return
	if not main._placement_at_tile(HOPPER_B).is_empty():
		_fail("the remove command did not remove hopper B")
		return

	# --- 9. A tank: the fluid its run carries, FILL, AUTO-FILL, no picker ---
	main._config_click_at_tile(TANK)
	if not main._config_open or main._config_kind != "fluid_tank" or main._config_tile != TANK:
		_fail("clicking the tank did not open a tank panel (open=%s, kind=%s)" % [
			main._config_open, main._config_kind
		])
		return
	if main._config_picker_box.visible:
		_fail("a tank's panel shows the hopper's item picker")
		return
	var tank := _tank_snapshot(TANK)
	if tank.is_empty():
		_fail("the tank has no snapshot on the wire")
		return
	var fluid: String = main._fluid_id_of(tank)
	if fluid == "":
		_fail("the plumbed tank reports no fluid — the mixer should have tapped its run")
		return
	var tank_title := "Fluid Tank — %s" % main._item_display_name(fluid)
	if main._config_title.text != tank_title:
		_fail("tank title is '%s', expected '%s'" % [main._config_title.text, tank_title])
		return
	var tank_capacity := float(tank.get("capacity", 0.0))
	if main._config_level_label.text != "%.1f / %.0f units" % [0.0, tank_capacity]:
		_fail("an empty tank's level reads '%s'" % main._config_level_label.text)
		return
	if main._config_fill_button.disabled:
		_fail("tank FILL is blocked ('%s') with %d %s in stock" % [
			main._config_fill_reason.text, _owned(fluid), fluid
		])
		return
	var fluid_before := _owned(fluid)
	main._on_config_fill_pressed()
	var tank_loaded := float(_tank_snapshot(TANK).get("quantity", 0.0))
	if tank_loaded <= 0.0:
		_fail("FILL on the tank's panel loaded nothing")
		return
	if _owned(fluid) >= fluid_before:
		_fail("filling the tank did not debit %s from the larder" % fluid)
		return
	if float(_tank_snapshot(LONE_TANK).get("quantity", 0.0)) != 0.0:
		_fail("FILL on one tank's panel also loaded the other tank — the old global V behaviour")
		return
	if main._config_auto_button.text != "AUTO-FILL:  OFF":
		_fail("a fresh tank's switch reads '%s'" % main._config_auto_button.text)
		return
	main._on_config_auto_pressed()
	if not bool(_tank_snapshot(TANK).get("auto_fill", false)) or main._config_auto_button.text != "AUTO-FILL:  ON":
		_fail("the tank's AUTO-FILL switch did not turn on (wire=%s, text='%s')" % [
			_tank_snapshot(TANK).get("auto_fill"), main._config_auto_button.text
		])
		return
	# The tank discharges into its run every tick and is topped up every
	# tick, so once the run has charged it sits within a whole unit of full —
	# PROVIDED the larder can cover both. Auto-fill spends stock and never
	# buys (GAMEPLAY.md §5's cost axis), so with only the market's one crate
	# left it would simply stop, and that is checked too: first the dry
	# case, then the stocked one.
	for _i in range(200):
		main.sim.tick()
	main._apply_dirty_chunks()
	_repoll()
	if _owned(fluid) != 0:
		_fail("auto-fill left %d %s in a larder it should have drained into the run" % [_owned(fluid), fluid])
		return
	if bool(_tank_snapshot(TANK).get("auto_fill", false)) != true:
		_fail("running the larder dry switched the tank's auto-fill off — it should stay armed")
		return
	Layouts.stock(main.sim, fluid, 256)
	for _i in range(200):
		main.sim.tick()
	main._apply_dirty_chunks()
	_repoll()
	var kept := float(_tank_snapshot(TANK).get("quantity", 0.0))
	if kept < tank_capacity - 1.0:
		_fail("an auto-fill tank holds %.1f of %.0f after 200 stocked ticks" % [kept, tank_capacity])
		return
	if _owned(fluid) >= 256:
		_fail("auto-fill topped the tank up without spending stock")
		return
	main._on_config_auto_pressed()
	if bool(_tank_snapshot(TANK).get("auto_fill", true)):
		_fail("the tank's AUTO-FILL switch did not turn off")
		return

	# --- 10. A tank plumbed to nothing says so -------------------------------
	main._config_click_at_tile(LONE_TANK)
	if not main._config_open or main._config_tile != LONE_TANK:
		_fail("clicking the unplumbed tank did not open its panel")
		return
	if main._config_title.text != "Fluid Tank — unconnected":
		_fail("an unplumbed tank's title is '%s', expected 'Fluid Tank — unconnected'" % main._config_title.text)
		return
	if not main._config_fill_button.disabled or main._config_fill_reason.text != "Not connected to a pipe run":
		_fail("an unplumbed tank's FILL is not blocked with 'Not connected to a pipe run' (got '%s')" % main._config_fill_reason.text)
		return
	main._close_config_panel()

	# --- 11. The three old keys are gone -------------------------------------
	var a_before := _hopper_snapshot(HOPPER_A)
	var tank_before := _tank_snapshot(TANK)
	var stock_before: Dictionary = main._owned_ingredient_counts().duplicate()
	for keycode in [KEY_SPACE, KEY_V, KEY_Q]:
		_press_key(keycode)
	_repoll()
	var a_after := _hopper_snapshot(HOPPER_A)
	if int(a_after.get("quantity", -1)) != int(a_before.get("quantity", -2)) \
			or String(a_after.get("item", "")) != String(a_before.get("item", "?")):
		_fail("SPACE/V/Q changed hopper A (%s -> %s)" % [str(a_before), str(a_after)])
		return
	if float(_tank_snapshot(TANK).get("quantity", -1.0)) != float(tank_before.get("quantity", -2.0)):
		_fail("SPACE/V/Q changed the tank's level")
		return
	if main._owned_ingredient_counts() != stock_before:
		_fail("SPACE/V/Q spent stock")
		return
	if main._config_open:
		_fail("a bare keypress opened the configuration panel")
		return
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		if legend.find("hopper or tank") < 0:
			_fail("the legend does not say a hopper or tank is clicked to configure it")
			return
		if legend.find("Q:") >= 0 or legend.find("V:") >= 0 or legend.find("SPACE:") >= 0:
			_fail("the legend still advertises a removed fill/contents key: %s" % legend)
			return

	print("CONFIG_PANEL_OK: click opens/toggles/retargets/closes, FILL per building with reasons, AUTO-FILL round-trips and keeps a %d-unit hopper and a %.0f-unit tank full, picker retargets a placed hopper, ESC/menus/modes/removal close it, unplumbed tank refused, SPACE/V/Q inert" % [capacity, tank_capacity])
	quit(0)
