# Headless proof of the DEBUG MENU (F3) — the development cheats added with
# the settings screen, 2026-09-21.
#
# Run with:
#   godot --headless --path godot_project --script test_debug_menu.gd
#
# Verifies:
#   * F3 opens and closes the panel, and it is NOT one of the five modals —
#     ESC must not take it, and it must not pause the factory behind it,
#   * GIVE MONEY moves the simulation's own balance by exactly the amount,
#     over the real `debug_grant_money` wire (`sim_core/tests/debug_grant.rs`
#     is the same claim from the other side),
#   * TIME SPEED changes how many ticks one fixed frame delta produces, x1
#     still being the shipped rate,
#   * FROZEN produces none at all — and is not the same thing as paused,
#     because the panels and the camera keep running,
#   * STEP advances exactly one tick while frozen, and exactly one,
#   * the readout tracks the sim rather than a copy of it,
#   * the panel frees the mouse in first person, since it has buttons on it.
#
# It touches no file: the cheats are simulation state and presentation state,
# and neither is persisted anywhere this test could dirty.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")


func _fail(msg: String) -> void:
	printerr("DEBUG_MENU_FAIL: %s" % msg)
	quit(1)


## The money the SIMULATION holds, off the same wire every panel reads.
func _money(main) -> float:
	var parsed = JSON.parse_string(main.sim.economy_snapshot_json())
	if typeof(parsed) != TYPE_DICTIONARY:
		return -1.0
	return float(parsed.get("money", -1.0))


## One key press through the same door a real one comes in by.
func _press_key(main, keycode: int) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


## How many ticks `seconds` of frame time produces at the current speed.
func _ticks_over(main, seconds: float) -> int:
	var before: int = main.sim.current_tick()
	main._advance_simulation(seconds)
	return main.sim.current_tick() - before


func _initialize() -> void:
	var session: Node = root.get_node_or_null("GameSession")
	if session == null:
		_fail("the GameSession autoload is not registered — check project.godot's [autoload]")
		return
	session.arm_new_game()

	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# A running factory, so "the ticks stopped" and "the ticks sped up" are
	# claims about something actually happening.
	Layouts.build_machines_fixture(main.sim)
	Layouts.stock_larder(main.sim)
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
	main._rebuild_world()

	# --- The panel exists, in the HUD, closed ------------------------------
	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("the scene has no HUD")
		return
	if main._debug_menu == null:
		_fail("the debug menu was not built")
		return
	if main._debug_menu.get_parent() != hud:
		_fail("the debug menu must be a direct child of the HUD")
		return
	if main._debug_menu.visible or main._debug_open:
		_fail("the debug menu must start closed — it is a tool, not a greeting")
		return
	# It draws over the pause menu (the one panel allowed up beside anything),
	# but the payout flyout still draws over it.
	var pause_index: int = hud.get_children().find(main._pause_menu)
	var debug_index: int = hud.get_children().find(main._debug_menu)
	if pause_index < 0 or debug_index < pause_index:
		_fail("the debug menu must be built after the pause menu so it draws over it")
		return

	# --- F3 ----------------------------------------------------------------
	_press_key(main, KEY_F3)
	if not main._debug_open or not main._debug_menu.visible:
		_fail("F3 must open the debug menu")
		return
	if main._modal_open():
		_fail("the debug menu must not count as a modal — it is not one of E/M/C/H/P")
		return
	if main._pause_open:
		_fail("opening the debug menu must not pause the factory")
		return

	# ESC belongs to the five panels and to the pause menu; it must leave this
	# one alone, or a developer watching a cheat land loses the panel every
	# time they dismiss something else.
	_press_key(main, KEY_ESCAPE)
	if not main._debug_open:
		_fail("ESC must not close the debug menu")
		return
	if not main._pause_open:
		_fail("ESC with nothing else open must still reach the pause menu")
		return
	main._set_pause_open(false)

	_press_key(main, KEY_F3)
	if main._debug_open or main._debug_menu.visible:
		_fail("F3 must close the debug menu again")
		return
	_press_key(main, KEY_F3)

	# --- GIVE MONEY --------------------------------------------------------
	var row: Node = main._debug_menu.find_child("DebugMoneyRow", true, false)
	if row == null:
		_fail("the debug menu has no GIVE MONEY row")
		return
	# The row carries its own GIVE MONEY label as well as the buttons, so the
	# claim is about the BUTTONS: one per entry, and no fourth one left behind
	# by a shortened list.
	var grants := 0
	for child in row.get_children():
		if child is Button:
			grants += 1
	if grants != main.DEBUG_MONEY_GRANTS.size():
		_fail("the GIVE MONEY row has %d buttons for %d DEBUG_MONEY_GRANTS entries" % [
			grants, main.DEBUG_MONEY_GRANTS.size()
		])
		return
	for amount in main.DEBUG_MONEY_GRANTS:
		var button: Button = main._debug_menu.find_child("DebugGrant_%d" % int(amount), true, false)
		if button == null:
			_fail("no button for a grant of %d" % int(amount))
			return
		if not button.text.contains("+"):
			_fail("a grant button must read as an addition, got '%s'" % button.text)
			return
		var before: float = _money(main)
		button.pressed.emit()
		var moved: float = _money(main) - before
		if absf(moved - float(amount)) > 0.5:
			_fail("a grant of %d moved the balance by %.2f" % [int(amount), moved])
			return

	# --- TIME SPEED --------------------------------------------------------
	# One second of frame time at x1 is TICK_HZ ticks, and MAX_TICKS_PER_FRAME
	# is the ceiling, so the measurement below uses a slice small enough that
	# no speed on the panel is clamped by it.
	var slice: float = float(main.MAX_TICKS_PER_FRAME) / float(main.TICK_HZ) * 0.5
	var baseline := 0
	for index in range(main.DEBUG_TIME_SCALES.size()):
		var speed_button: Button = main._debug_menu.find_child(
			"DebugSpeed_%d" % index, true, false
		)
		if speed_button == null:
			_fail("no button for time speed %s" % String(main.DEBUG_TIME_LABELS[index]))
			return
		speed_button.pressed.emit()
		var scale: float = float(main.DEBUG_TIME_SCALES[index])
		# The selected speed is said in words, not shown by the plate alone
		# (UI_UX.md §7).
		if not speed_button.text.begins_with("["):
			_fail("the selected speed must mark itself in the caption, got '%s'" % (
				speed_button.text
			))
			return
		var ticks: int = _ticks_over(main, slice)
		if scale <= 0.0:
			if ticks != 0:
				_fail("FROZEN must produce no ticks, got %d" % ticks)
				return
			continue
		if index == 1:
			baseline = ticks
			if baseline <= 0:
				_fail("x1 must still run the factory, got %d ticks" % baseline)
				return
			continue
		if baseline > 0 and absf(float(ticks) / float(baseline) - scale) > 0.25:
			_fail("%s produced %d ticks against x1's %d" % [
				String(main.DEBUG_TIME_LABELS[index]), ticks, baseline
			])
			return

	# --- FROZEN is not PAUSED, and STEP is the way out of it ---------------
	main._debug_menu.find_child("DebugSpeed_0", true, false).pressed.emit()
	if main._pause_open:
		_fail("FROZEN must not pause the game — the camera and panels keep running")
		return
	var frozen_tick: int = main.sim.current_tick()
	main._advance_simulation(10.0)
	if main.sim.current_tick() != frozen_tick:
		_fail("a frozen factory must not advance, ran %d ticks" % (
			main.sim.current_tick() - frozen_tick
		))
		return
	var step: Button = main._debug_menu.find_child("DebugStepButton", true, false)
	if step == null:
		_fail("the debug menu has no STEP button")
		return
	step.pressed.emit()
	if main.sim.current_tick() != frozen_tick:
		_fail("STEP must take its tick on the next frame, not inside the button handler")
		return
	main._advance_simulation(0.0)
	if main.sim.current_tick() != frozen_tick + 1:
		_fail("STEP must advance exactly one tick, got %d" % (
			main.sim.current_tick() - frozen_tick
		))
		return
	main._advance_simulation(10.0)
	if main.sim.current_tick() != frozen_tick + 1:
		_fail("a step must be spent once, not held down")
		return

	# Back to normal speed for the rest of the run.
	main._debug_menu.find_child("DebugSpeed_1", true, false).pressed.emit()

	# --- The readout tracks the sim ----------------------------------------
	main._advance_simulation(1.0)
	main._update_debug_readout()
	var text: String = main._debug_readout.text
	if not text.contains("tick %d" % int(main.sim.current_tick())):
		_fail("the readout does not show the current tick:\n%s" % text)
		return
	if not text.contains("revision %d" % int(main.sim.revision())):
		_fail("the readout does not show the current revision:\n%s" % text)
		return
	if not text.contains("money %s" % main.Money.amount(_money(main))):
		_fail("the readout does not show the simulation's own balance:\n%s" % text)
		return
	# One `user://` line for both files, so what it must name is the directory
	# they are both in — not `res://`, and not Godot's own unshortened spelling
	# of a path forty characters longer than the panel is wide.
	if not text.contains("user://"):
		_fail("the readout must say where user:// really is:\n%s" % text)
		return
	if not text.contains("settings.cfg"):
		_fail("the readout must name the settings file:\n%s" % text)
		return
	var home := OS.get_environment("HOME")
	if home != "" and text.contains(home):
		_fail("the user:// path must be written with ~, not the whole home directory:\n%s" % text)
		return

	# --- The mouse ---------------------------------------------------------
	# The panel has buttons; a captured mouse cannot press them.
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	if not main._fp_mouse_free():
		_fail("the debug menu must hand the mouse back in first person")
		return
	_press_key(main, KEY_F3)
	if main._fp_mouse_free():
		_fail("closing the debug menu must take the mouse back for looking around")
		return

	print("DEBUG_MENU_OK: F3 toggles a panel that is not a modal and does not pause; each")
	print("               GIVE MONEY button moves the sim's own balance by exactly its")
	print("               amount over the real wire; time speed scales ticks per frame,")
	print("               FROZEN stops them without pausing, STEP spends exactly one; the")
	print("               readout tracks the sim and names the settings file; the panel")
	print("               frees the mouse in first person")
	quit(0)
