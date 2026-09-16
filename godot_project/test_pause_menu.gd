# Headless proof of the IN-GAME PAUSE MENU, saving, and autosave (the
# "Save/load + a real main menu" epic, ticket 2).
#
# Run with:
#   godot --headless --path godot_project --script test_pause_menu.gd
#
# Verifies:
#   * ESC with nothing open opens the pause menu — and ESC with something
#     open still closes THAT first, which is the whole reason ESC could take
#     on a second meaning without a second key,
#   * the simulation genuinely stops while it is up, and resuming does not
#     fast-forward the factory by however long the player spent reading it,
#   * Save writes `user://saves/<name>.ron` whose contents are exactly
#     `save_ron()`, and remembers the slot for next time,
#   * the reserved `autosave` name is refused, out loud, and writes nothing,
#   * a name that is not a legal file name is sanitized rather than rejected,
#   * `_autosave()` writes `autosave.ron`, and the real-time timer fires it
#     without this test waiting three minutes for it — and does NOT run while
#     the game is paused,
#   * Quit to Menu asks first, then autosaves and arms the session for a
#     fresh start — and leaves the cursor free, so the menu it changes to is
#     not handed a captured mouse.
#
# It writes real files under `user://saves/`, because that is what is being
# tested. It removes the named slot it wrote, and it BACKS UP AND RESTORES
# `autosave.ron` — a developer running this suite should not lose the
# autosave from the game they were actually playing.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const GameSaves = preload("res://game_session.gd")

const SLOT := "zz pause test"
## Deliberately full of characters a file name cannot have, to prove
## `sanitize_slot` is what stands between player text and the filesystem.
const MESSY_SLOT := "  ../my  save!!  "

var _autosave_backup := ""
var _autosave_existed := false


func _fail(msg: String) -> void:
	_restore()
	printerr("PAUSE_MENU_FAIL: %s" % msg)
	quit(1)


func _backup_autosave() -> void:
	var path: String = GameSaves.slot_path(GameSaves.AUTOSAVE_SLOT)
	_autosave_existed = FileAccess.file_exists(path)
	if _autosave_existed:
		_autosave_backup = GameSaves.read_save(path)


func _restore() -> void:
	var autosave_path: String = GameSaves.slot_path(GameSaves.AUTOSAVE_SLOT)
	if _autosave_existed:
		GameSaves.write_slot(GameSaves.AUTOSAVE_SLOT, _autosave_backup, true)
	elif FileAccess.file_exists(autosave_path):
		DirAccess.remove_absolute(autosave_path)
	for slot in [SLOT, GameSaves.sanitize_slot(MESSY_SLOT)]:
		var path: String = GameSaves.slot_path(slot)
		if slot != "" and FileAccess.file_exists(path):
			DirAccess.remove_absolute(path)


func _press(main, keycode: int) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _initialize() -> void:
	_backup_autosave()
	var session: Node = root.get_node_or_null("GameSession")
	if session == null:
		_fail("the GameSession autoload is not registered — check project.godot's [autoload]")
		return
	session.arm_new_game()

	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# A running factory, so "the ticks stopped" is a claim about something
	# actually happening rather than about an empty world.
	Layouts.build_machines_fixture(main.sim)
	Layouts.stock_larder(main.sim)
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
	main._rebuild_world()

	# --- The panel exists, is part of the HUD, and starts closed. ----------
	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return
	if main._pause_menu == null:
		_fail("main.gd built no pause menu")
		return
	if main._pause_menu.get_parent() != hud:
		_fail("the pause menu must live on the HUD layer, like every other panel")
		return
	if main._pause_menu.visible or main._pause_open:
		_fail("the game must not start paused")
		return
	for button_name in [
		"ResumeButton", "SaveButton", "SaveQuitButton", "QuitToMenuButton", "QuitGameButton",
	]:
		if main._pause_menu.find_child(button_name, true, false) == null:
			_fail("the pause menu needs a %s — that is its whole contents" % button_name)
			return
	if main._pause_slot_field == null:
		_fail("Save needs a slot-name field")
		return

	# --- Discoverability: the legend names the key. ------------------------
	if not main.ISO_HELP_TEXT.to_lower().contains("pause menu"):
		_fail("the control legend must say ESC opens the pause menu")
		return
	if not main.FP_HELP_TEXT.to_lower().contains("pause menu"):
		_fail("the first-person legend must say so too")
		return

	# --- ESC still closes what IS open, FIRST. -----------------------------
	main._set_shop_menu_open(true)
	_press(main, KEY_ESCAPE)
	if main._shop_open:
		_fail("ESC must still close the build menu")
		return
	if main._pause_open:
		_fail("ESC must not open the pause menu in the same press that closed a panel")
		return
	# Build mode counts as "open" too — putting the ghost down is the first
	# press, not the second. (A kind has to be OWNED to be picked up, so this
	# buys one first, exactly as a player would.)
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	main._refresh_owned_counts()
	main._enter_build_mode("belt")
	if not main._in_build_mode():
		_fail("could not enter build mode to test ESC's ordering")
		return
	_press(main, KEY_ESCAPE)
	if main._in_build_mode():
		_fail("ESC must still leave build mode")
		return
	if main._pause_open:
		_fail("leaving build mode must not also open the pause menu")
		return

	# --- ESC with nothing open opens it. -----------------------------------
	_press(main, KEY_ESCAPE)
	if not main._pause_open or not main._pause_menu.visible:
		_fail("ESC with nothing open must open the pause menu")
		return
	if main._pause_slot_field.text != main._save_slot:
		_fail("the slot field must prefill with this session's slot, got '%s'" % (
			main._pause_slot_field.text
		))
		return

	# --- The factory is STOPPED while it is up. ----------------------------
	var paused_tick: int = main.sim.current_tick()
	for i in range(10):
		main._advance_simulation(1.0)
	if main.sim.current_tick() != paused_tick:
		_fail("the simulation must not tick while paused (ran %d ticks)" % (
			main.sim.current_tick() - paused_tick
		))
		return
	if main._autosave_accum != 0.0:
		_fail("the autosave clock must not run while paused either")
		return

	# --- And resuming does not pay back the pause. -------------------------
	_press(main, KEY_ESCAPE)
	if main._pause_open:
		_fail("ESC must close the pause menu again")
		return
	if main._tick_accum != 0.0:
		_fail("resuming must start from a cleared accumulator, got %f" % main._tick_accum)
		return
	main._advance_simulation(1.0 / main.TICK_HZ)
	var after_one_frame: int = main.sim.current_tick() - paused_tick
	if after_one_frame != 1:
		_fail("one frame's worth of time after a 10-second pause must be ONE tick, got %d" % (
			after_one_frame
		))
		return

	# --- Save writes the file, and it IS the save. -------------------------
	main._set_pause_open(true)
	main._pause_slot_field.text = SLOT
	main._on_pause_save_pressed()
	var slot_path: String = GameSaves.slot_path(SLOT)
	if not FileAccess.file_exists(slot_path):
		_fail("Save wrote no file at %s (status: %s)" % [slot_path, main._pause_status.text])
		return
	if GameSaves.read_save(slot_path) != String(main.sim.save_ron()):
		_fail("the file Save wrote must be exactly what save_ron() returns")
		return
	if not main._pause_status.text.contains(SLOT):
		_fail("Save must say where it saved, got '%s'" % main._pause_status.text)
		return
	if String(main._save_slot) != SLOT:
		_fail("Save must remember the slot for next time, got '%s'" % main._save_slot)
		return
	if String(session.current_slot) != SLOT:
		_fail("the session must remember it too, so quitting and reloading offers it")
		return

	# --- A save loads back, which is the only proof the file is real. ------
	var probe = SimBridge.new()
	var refusal: String = probe.load_ron(GameSaves.read_save(slot_path))
	if refusal != "":
		_fail("the file Save wrote must be loadable, got '%s'" % refusal)
		return
	if probe.current_tick() != main.sim.current_tick():
		_fail("the saved file must carry this world's tick count")
		return

	# --- A messy name is sanitized, not refused. ---------------------------
	var messy_slot: String = GameSaves.sanitize_slot(MESSY_SLOT)
	if messy_slot == "" or messy_slot.contains("/") or messy_slot.contains("."):
		_fail("sanitize_slot must produce a plain file name, got '%s'" % messy_slot)
		return
	main._pause_slot_field.text = MESSY_SLOT
	main._on_pause_save_pressed()
	if not FileAccess.file_exists(GameSaves.slot_path(messy_slot)):
		_fail("a messy name must still save, as '%s'" % messy_slot)
		return

	# --- The reserved name is refused, and writes nothing. -----------------
	var autosave_path: String = GameSaves.slot_path(GameSaves.AUTOSAVE_SLOT)
	if FileAccess.file_exists(autosave_path):
		DirAccess.remove_absolute(autosave_path)
	main._pause_slot_field.text = GameSaves.AUTOSAVE_SLOT
	main._on_pause_save_pressed()
	if FileAccess.file_exists(autosave_path):
		_fail("a manual save must never be allowed to write the autosave's own slot")
		return
	if not main._pause_status.text.to_lower().contains("autosave"):
		_fail("the refusal must say why, got '%s'" % main._pause_status.text)
		return
	if String(main._save_slot) != messy_slot:
		_fail("a refused save must not change the remembered slot")
		return
	# Capitalisation does not get round it: the reservation is about the file.
	if not GameSaves.is_reserved_slot("AutoSave"):
		_fail("the reserved name must be case-insensitive")
		return

	# --- Autosave writes its own slot, and only its own slot. --------------
	main._autosave("timer")
	if not FileAccess.file_exists(autosave_path):
		_fail("_autosave() must write %s" % autosave_path)
		return
	if GameSaves.read_save(autosave_path) != String(main.sim.save_ron()):
		_fail("the autosave file must be this world's save_ron()")
		return
	if String(main._save_slot) != messy_slot:
		_fail("autosave must not steal the named slot the player is saving to")
		return

	# --- The three-minute timer fires, without waiting three minutes. ------
	DirAccess.remove_absolute(autosave_path)
	main._set_pause_open(false)
	main._autosave_accum = main.AUTOSAVE_INTERVAL_SECONDS - 0.01
	main._advance_simulation(0.005)
	if FileAccess.file_exists(autosave_path):
		_fail("the autosave timer fired early")
		return
	main._advance_simulation(0.02)
	if not FileAccess.file_exists(autosave_path):
		_fail("the autosave timer must fire once AUTOSAVE_INTERVAL_SECONDS has passed")
		return
	if main._autosave_accum >= main.AUTOSAVE_INTERVAL_SECONDS:
		_fail("the autosave clock must restart after it fires")
		return

	# --- Closing the WINDOW autosaves too. ---------------------------------
	# `main.gd` turns off `auto_accept_quit` in `_ready` precisely so it gets
	# this notification before the process goes away; without the interception
	# the last few minutes of building would leave with the window.
	DirAccess.remove_absolute(autosave_path)
	main._notification(Node.NOTIFICATION_WM_CLOSE_REQUEST)
	if not FileAccess.file_exists(autosave_path):
		_fail("closing the window must autosave before quitting")
		return

	# --- Quit to Menu asks first. ------------------------------------------
	main._set_pause_open(true)
	main._on_pause_quit_to_menu_pressed()
	if not main._pause_confirm_quit or not main._pause_confirm_box.visible:
		_fail("Quit to Menu must ask before throwing unsaved progress away")
		return
	if main._pause_buttons_box.visible:
		_fail("the confirm step replaces the buttons rather than stacking over them")
		return
	main._on_pause_cancel_quit_pressed()
	if main._pause_confirm_quit or not main._pause_buttons_box.visible:
		_fail("Cancel must put the buttons back")
		return
	if not main._pause_open:
		_fail("Cancel must leave the pause menu up, not resume the game")
		return

	# --- ...and then does it, WITHOUT taking the cursor with it. -----------
	# Quitting from first-person: closing the pause menu re-runs the capture
	# rule, which used to recapture the mouse in the frame before the scene
	# change and hand the main menu an invisible pointer. Headless Godot
	# ignores `Input.mouse_mode` (see test_camera_nav.gd), so the DECISION is
	# what is asserted — `_fp_mouse_free` is what every capture seam reads.
	main._view_mode = main.ViewMode.FIRST_PERSON
	DirAccess.remove_absolute(autosave_path)
	session.arm_load("res://nowhere.ron", "stale")
	main._quit_to_menu()
	if not main._fp_mouse_free():
		_fail("quitting to the menu must hand the cursor back, not leave it captured")
		return
	if main._crosshair != null and main._crosshair.visible:
		_fail("the crosshair must go with the factory, not follow it to the menu")
		return
	if not FileAccess.file_exists(autosave_path):
		_fail("quitting to the menu must autosave first")
		return
	if String(session.pending_action) != "new_game":
		_fail("quitting to the menu must clear a stale pending load, got '%s'" % (
			session.pending_action
		))
		return
	if main._pause_open:
		_fail("quitting to the menu must close the pause menu behind it")
		return

	_restore()
	print("PAUSE_MENU_OK: ESC closes what is open first and opens the pause menu when")
	print("               nothing is, the factory genuinely stops and resumes without a")
	print("               jump, Save writes save_ron() to a sanitized slot, the reserved")
	print("               autosave name is refused out loud, the three-minute timer fires")
	print("               and Quit to Menu confirms, autosaves, re-arms the session and")
	print("               hands the cursor back on its way out")
	quit(0)
