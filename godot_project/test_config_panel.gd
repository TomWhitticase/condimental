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
#     unplumbed tank says "Not connected to a pipe run", and its picker
#     offers the raw FLUIDS with nothing marked while the fluid is merely
#     derived,
#   * declaring a tank's fluid from that picker (issue #43) reaches the
#     simulation, marks the slot, retitles the panel and stops the tank
#     reading as unconnected, and the note beneath it names how much fluid
#     switching would destroy — the tank's AND the run's — before the click,
#   * a MACHINE opens the same panel (issue #19), with the hopper/tank rows
#     hidden, and its mothball switch issues `set_mothballed`, reads its
#     state back off the simulation, and puts "switched off" and the unlit
#     glyph into the hover line and the unlit fill into the load bar,
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
const MIXER := Vector2i(0, 0)
const MIXER_NON_ORIGIN := Vector2i(0, 1)
## One tile of `build_fluid_run`'s own pipe run — a kind with nothing to
## configure, which is what proves the panel did not simply start opening
## for everything when it learned about machines.
const PIPE_TILE := Vector2i(2, 2)
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


## What the pipe run this tank discharges into is holding, read straight off
## the wire rather than through `main`'s cached poll — the panel's warning is
## the thing under test, so its expected number must not be computed by the
## same function that produces it.
func _run_volume_fed_by(tank: Dictionary) -> float:
	var pipe_tile = tank.get("pipe_tile")
	if pipe_tile == null:
		return 0.0
	for run in JSON.parse_string(main.sim.fluid_snapshots_json()):
		for segment in run.get("segments", []):
			var at: Array = segment["tile"]
			if int(at[0]) == int(pipe_tile[0]) and int(at[1]) == int(pipe_tile[1]):
				return float(run.get("total_volume", 0.0))
	return 0.0


func _machine_snapshot(tile: Vector2i) -> Dictionary:
	for machine in JSON.parse_string(main.sim.machine_load_snapshots_json()):
		var at: Array = machine["tile"]
		if int(at[0]) == tile.x and int(at[1]) == tile.y:
			return machine
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
	# `_process` re-reads the fluid networks every frame too, and since
	# issue #43's purge the tank panel's warning counts the run's volume as
	# well as the tank's — so a poll that skipped this would read a stale
	# number that no player could ever see.
	main._update_fluid_network()
	# The load record is polled on its own cadence in the live scene and
	# carries the machine's staffing facts (issues #19 and #20), so a test
	# with no frames has to ask for it by hand like everything else here.
	main._poll_machine_load()
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
	# A MACHINE retargets it too now (issue #19): the panel grew a third
	# shape rather than a second panel, and a non-origin footprint tile
	# resolves to the origin exactly as a hopper's click does.
	main._config_click_at_tile(MIXER_NON_ORIGIN)
	if not main._config_open or main._config_tile != MIXER:
		_fail("clicking the mixer's second tile did not open its panel at %s (open=%s, tile=%s)" % [
			str(MIXER), main._config_open, str(main._config_tile)
		])
		return
	if main._config_kind != "mixer":
		_fail("the machine panel is about '%s', not the mixer" % main._config_kind)
		return
	# A pipe is still nothing to configure, and still closes it.
	main._config_click_at_tile(PIPE_TILE)
	if main._config_open:
		_fail("clicking a pipe left a panel open")
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

	# --- 9. A tank: the fluid its run carries, FILL, AUTO-FILL, its picker ---
	main._config_click_at_tile(TANK)
	if not main._config_open or main._config_kind != "fluid_tank" or main._config_tile != TANK:
		_fail("clicking the tank did not open a tank panel (open=%s, kind=%s)" % [
			main._config_open, main._config_kind
		])
		return
	# The picker is on both kinds since issue #43, built from the raw FLUIDS
	# this time — and with nothing marked, because this tank's fluid was
	# derived from its run rather than chosen by anybody.
	if not main._config_picker_box.visible:
		_fail("a tank's panel has no contents picker")
		return
	if main._config_picker_slots.size() != main._tank_fluids.size() \
			or main._tank_fluids.is_empty():
		_fail("the tank picker has %d slots for %s" % [
			main._config_picker_slots.size(), str(main._tank_fluids)
		])
		return
	for slot in main._config_picker_slots:
		if slot["mark"].visible:
			_fail("an undeclared tank marks '%s' as chosen — nobody chose it" % slot["item"])
			return
	if main._config_picker_note.text != "read from the pipe run — pick one to say for certain":
		_fail("an undeclared but named tank's note reads '%s'" % main._config_picker_note.text)
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
	# "no fluid chosen" rather than "unconnected" since issue #43: a tank
	# with no fluid is now something the player can fix from this panel,
	# and two different causes (no pipe, or a run nothing can name) share
	# the title. The FILL reason below is what tells them apart.
	if main._config_title.text != "Fluid Tank — no fluid chosen":
		_fail("an unplumbed tank's title is '%s', expected 'Fluid Tank — no fluid chosen'" % main._config_title.text)
		return
	if not main._config_fill_button.disabled or main._config_fill_reason.text != "Not connected to a pipe run":
		_fail("an unplumbed tank's FILL is not blocked with 'Not connected to a pipe run' (got '%s')" % main._config_fill_reason.text)
		return
	if main._config_picker_note.text != "nothing can name this run — pick what this tank holds":
		_fail("an unnamed tank's picker note reads '%s'" % main._config_picker_note.text)
		return

	# --- 10b. Declaring a tank's fluid (issue #43) ---------------------------
	# The player-facing half of #43: two tank-fed runs into one machine that
	# draws two raw fluids cannot be told apart by the simulation, so the
	# tank is told. Proven here on the tank nothing else names, which is the
	# same state both runs of that layout are in.
	var declared_index: int = main._tank_fluids.size() - 1
	var declared_fluid := String(main._tank_fluids[declared_index])
	main._on_config_picker_pressed(declared_index)
	_repoll()
	var lone := _tank_snapshot(LONE_TANK)
	if String(lone.get("declared", "")) != declared_fluid:
		_fail("set_tank_fluid did not declare %s (wire says %s)" % [
			declared_fluid, str(lone.get("declared"))
		])
		return
	if main._fluid_id_of(lone) != declared_fluid:
		_fail("a declared tank's fluid is '%s', expected '%s'" % [
			main._fluid_id_of(lone), declared_fluid
		])
		return
	if main._config_title.text != "Fluid Tank — %s" % main._item_display_name(declared_fluid):
		_fail("a declared tank's title reads '%s'" % main._config_title.text)
		return
	# The note names what switching would destroy, before the click. This
	# tank is empty and feeds no run, so there is nothing to warn about yet.
	if main._config_picker_note.text != "switching empties this tank and the run it feeds":
		_fail("a declared empty tank's picker note reads '%s'" % main._config_picker_note.text)
		return
	# --- 10c. The warning counts the run, not just the tank -----------------
	# Switching a declaration purges the pipe run as well as the tank (the
	# sim would otherwise relabel what is already in the run as the new
	# fluid — found in review of this ticket's pull request). That is more
	# than this panel shows a level for, so the note says how much goes
	# BEFORE the click rather than leaving the player to notice a number
	# that dropped.
	main._config_click_at_tile(TANK)
	var fluid_slot: int = main._tank_fluids.find(fluid)
	if fluid_slot < 0:
		_fail("the tank picker does not offer '%s', which is what its run carries" % fluid)
		return
	main._on_config_picker_pressed(fluid_slot)
	Layouts.stock(main.sim, fluid, 256)
	# FILL buys one crate at a time and the mixer drinks what reaches it, so
	# fill to the brim and let only a few ticks pass: the point is a tank AND
	# a run that both hold something when the picker is next drawn.
	for _i in range(4):
		main._on_config_fill_pressed()
	for _i in range(5):
		main.sim.tick()
	main._apply_dirty_chunks()
	_repoll()
	var topped_up := _tank_snapshot(TANK)
	var in_tank := float(topped_up.get("quantity", 0.0))
	var in_run := _run_volume_fed_by(topped_up)
	if in_tank <= 0.0 or in_run <= 0.0:
		_fail("tank holds %.1f and its run %.1f — the warning proves nothing" % [in_tank, in_run])
		return
	# Spelled out from the wire, not from `_tank_doomed_volume`: a warning
	# that counted only the tank would match itself and pass while lying.
	var warning := "switching destroys %.1f units — this tank and the run it feeds" % (
		in_tank + in_run
	)
	if main._config_picker_note.text != warning:
		_fail("a filled tank's note reads '%s', expected '%s'" % [
			main._config_picker_note.text, warning
		])
		return

	main._config_click_at_tile(LONE_TANK)
	var marked: Array = []
	for slot in main._config_picker_slots:
		if slot["mark"].visible:
			marked.append(String(slot["item"]))
	if marked != [declared_fluid]:
		_fail("a declared tank marks %s, expected only %s" % [str(marked), declared_fluid])
		return
	if main._config_fill_reason.text == "Not connected to a pipe run":
		_fail("a declared tank still reads as unconnected")
		return
	main._close_config_panel()

	# --- 11. The machine panel and its mothball switch (issue #19) ----------
	# The escape hatch from the unpaid spiral, on the panel the player
	# already opens to change a building's mind. What is checked here is the
	# BOUNDARY — the right command, the state read back off the simulation's
	# own snapshot rather than flipped locally, and the words. That the
	# establishment and the wage bill actually fall is proved in Rust
	# (`sim_core/tests/mothball.rs`), where it belongs.
	main._config_click_at_tile(MIXER)
	if not main._config_open or main._config_kind != "mixer":
		_fail("clicking the mixer did not open a machine panel")
		return
	# A machine has nothing to fill and nothing to keep filled, so those
	# rows are hidden rather than left disabled forever.
	if main._config_fill_row.visible or main._config_auto_button.visible \
			or main._config_auto_note.visible or main._config_picker_box.visible:
		_fail("a machine's panel still shows the hopper/tank rows")
		return
	if not main._config_machine_box.visible:
		_fail("a machine's panel hides the mothball switch")
		return
	if main._config_mothballed:
		_fail("a freshly placed mixer reads as already mothballed")
		return
	if main._config_mothball_button.text != "IN SERVICE:  ON":
		_fail("the switch on a running mixer reads '%s'" % main._config_mothball_button.text)
		return
	if main._config_machine_state.text.find("in service") < 0:
		_fail("a running mixer's panel does not say it is in service: '%s'" % main._config_machine_state.text)
		return

	main._on_config_mothball_pressed()
	_repoll()
	if not _machine_snapshot(MIXER).get("mothballed", false):
		_fail("the switch did not reach the simulation — the mixer is still in service")
		return
	if not main._config_mothballed or main._config_mothball_button.text != "MOTHBALLED:  OFF":
		_fail("the panel did not read the mothballed state back: '%s'" % main._config_mothball_button.text)
		return
	# Never colour alone (UI_UX.md §7): the hover line carries the glyph AND
	# the words, and the bar behind it goes to the unlit grey.
	var mixer_key: String = main._tile_key(MIXER)
	var load_text: String = main._machine_load_text(mixer_key)
	if load_text.find("switched off") < 0 or load_text.find(main.LOAD_OFF_GLYPH) < 0:
		_fail("a mothballed machine's load line is '%s'" % load_text)
		return
	if main._load_bar_color(main._machine_load[mixer_key]) != main.METER_FILL_OFF:
		_fail("a mothballed machine's load bar is not the unlit fill")
		return
	if main._machine_hover_text(mixer_key, "mixer", "east").find("switched off") < 0:
		_fail("the mixer's hover does not say it is switched off")
		return

	main._on_config_mothball_pressed()
	_repoll()
	if _machine_snapshot(MIXER).get("mothballed", true):
		_fail("throwing the switch back did not return the mixer to service")
		return
	if main._config_mothballed or main._config_mothball_button.text != "IN SERVICE:  ON":
		_fail("the panel did not read the returned-to-service state back")
		return
	main._close_config_panel()

	# --- 12. The three old keys are gone -------------------------------------
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
		if legend.find("click a building: configure") < 0:
			_fail("the legend does not say a building is clicked to configure it")
			return
		if legend.find("Q:") >= 0 or legend.find("V:") >= 0 or legend.find("SPACE:") >= 0:
			_fail("the legend still advertises a removed fill/contents key: %s" % legend)
			return

	print("CONFIG_PANEL_OK: click opens/toggles/retargets/closes, FILL per building with reasons, AUTO-FILL round-trips and keeps a %d-unit hopper and a %.0f-unit tank full, picker retargets a placed hopper and declares a tank's fluid after warning what switching destroys, ESC/menus/modes/removal close it, unplumbed tank refused, a machine's mothball switch round-trips through the sim and reads back in words, SPACE/V/Q inert" % [capacity, tank_capacity])
	quit(0)
