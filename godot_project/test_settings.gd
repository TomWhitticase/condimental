# Headless proof of the SETTINGS screen (the settings + debug menu branch,
# 2026-09-21) — the `Settings` autoload, `settings_panel.gd`'s rows, and the
# two hosts that show them.
#
# Run with:
#   godot --headless --path godot_project --script test_settings.gd
#
# Verifies:
#   * a fresh `Settings` holds every knob's shipped default,
#   * every stepper clamps at both ends and snaps to its own grid, so eight
#     presses of `+` can never leave 0.7999999 in the file,
#   * `save_file()`/`load_file()` round-trip through a real `user://` file —
#     the one thing `persist_enabled` is turned back ON for, around a BACKUP
#     of the developer's own settings, which are restored before this script
#     exits however it exits,
#   * MUSIC/SOUND and their volumes reach the two audio owners,
#   * MOVEMENT SPEED, LOOK SENSITIVITY, FIELD OF VIEW and HALL SIZE reach the
#     camera and the sky shader,
#   * RESET puts every row back,
#   * the pause menu and the main menu build the SAME rows, which is the
#     whole reason `settings_panel.gd` exists.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const SettingsScript = preload("res://settings.gd")
const FactoryEnvironment = preload("res://factory_environment.gd")

var _settings: Node
var _backup := ""
var _backup_existed := false


## Puts the developer's own `user://settings.cfg` back exactly as it was.
## Called before every exit, success or failure: this suite must not be able
## to change the sound on the machine it ran on.
func _restore() -> void:
	var path: String = SettingsScript.SETTINGS_PATH
	if _backup_existed:
		var file := FileAccess.open(path, FileAccess.WRITE)
		if file != null:
			file.store_string(_backup)
			file.close()
	elif FileAccess.file_exists(path):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	if _settings != null:
		_settings.persist_enabled = false
		_settings.reset()


func _fail(msg: String) -> void:
	_restore()
	printerr("SETTINGS_FAIL: %s" % msg)
	quit(1)


func _knob(key: String) -> Dictionary:
	return SettingsScript.knob(key)


func _initialize() -> void:
	_settings = root.get_node_or_null("Settings")
	if _settings == null:
		_fail("the Settings autoload is not registered — check project.godot's [autoload]")
		return
	if _settings.persist_enabled:
		_fail("a headless run must not be allowed to write the developer's settings.cfg")
		return
	if _settings.loaded:
		_fail("a headless run must not have read the developer's settings.cfg either")
		return

	# --- Defaults ----------------------------------------------------------
	if not _settings.is_default():
		_fail("a headless run must start on the shipped defaults")
		return
	for entry in SettingsScript.KNOBS:
		var key: String = entry["key"]
		if not is_same(_settings.get_value(key), entry["default"]):
			_fail("%s starts at %s, not its default %s" % [
				key, _settings.get_value(key), entry["default"]
			])
			return
		# Never a bare number on a row (UI_UX.md §7).
		var shown: String = _settings.format_value(key)
		if shown.is_valid_float():
			_fail("%s reads as the bare number '%s'" % [key, shown])
			return

	# --- Clamping and snapping --------------------------------------------
	for entry in SettingsScript.KNOBS:
		if String(entry["kind"]) != "step":
			continue
		var key: String = entry["key"]
		var low: float = entry["min"]
		var high: float = entry["max"]
		var step: float = entry["step"]
		# Walk off the bottom, then off the top, one press at a time — the
		# only way a player can move a stepper.
		var presses := int((high - low) / step) + 4
		for i in range(presses):
			_settings.nudge(key, -1)
		if absf(float(_settings.get_value(key)) - low) > 0.0001:
			_fail("%s fell through its floor to %s" % [key, _settings.get_value(key)])
			return
		for i in range(presses):
			_settings.nudge(key, 1)
		if absf(float(_settings.get_value(key)) - high) > 0.0001:
			_fail("%s ran past its ceiling to %s" % [key, _settings.get_value(key)])
			return
		# And every value on the way is on the grid, not drifting off it.
		_settings.set_value(key, low)
		for i in range(presses):
			_settings.nudge(key, 1)
			var value: float = float(_settings.get_value(key))
			var steps: float = (value - low) / step
			if absf(steps - round(steps)) > 0.0001:
				_fail("%s landed on %.7f, which is not on its %s grid" % [key, value, step])
				return
	_settings.reset()

	# --- The file round-trip ----------------------------------------------
	# The one place persistence is switched back on, around a backup.
	var path: String = SettingsScript.SETTINGS_PATH
	_backup_existed = FileAccess.file_exists(path)
	if _backup_existed:
		var file := FileAccess.open(path, FileAccess.READ)
		if file != null:
			_backup = file.get_as_text()
			file.close()
	_settings.persist_enabled = true

	_settings.set_value("music_enabled", false)
	_settings.set_value("music_volume", 0.3)
	_settings.set_value("sfx_volume", 0.7)
	_settings.set_value("move_speed", 2.0)
	_settings.set_value("field_of_view", 90.0)
	_settings.set_value("hall_scale", 2.0)
	var error: String = _settings.save_file()
	if error != "":
		_fail("saving the settings failed: %s" % error)
		return
	if not FileAccess.file_exists(path):
		_fail("save_file() reported success but wrote nothing to %s" % path)
		return

	_settings.reset()
	if not _settings.is_default():
		_fail("reset() must put every row back to its default")
		return
	if not _settings.load_file():
		_fail("load_file() must report reading a file it has just written")
		return
	if bool(_settings.music_enabled):
		_fail("the saved MUSIC switch did not come back off")
		return
	for pair in [["music_volume", 0.3], ["sfx_volume", 0.7], ["move_speed", 2.0],
			["field_of_view", 90.0], ["hall_scale", 2.0]]:
		if absf(float(_settings.get_value(String(pair[0]))) - float(pair[1])) > 0.0001:
			_fail("%s came back as %s, not %s" % [
				pair[0], _settings.get_value(String(pair[0])), pair[1]
			])
			return

	# A file that is missing a key keeps that key's default rather than
	# erroring — a settings file is a record, not a schema.
	var partial := ConfigFile.new()
	partial.set_value(SettingsScript.SECTION, "move_speed", 1.5)
	partial.save(path)
	_settings.reset()
	if not _settings.load_file():
		_fail("a file with only one key is still a file that loaded")
		return
	if absf(float(_settings.move_speed) - 1.5) > 0.0001:
		_fail("the one key in a partial file was not read")
		return
	if absf(float(_settings.field_of_view) - float(_knob("field_of_view")["default"])) > 0.0001:
		_fail("a key missing from the file must keep its default")
		return

	# A file out of range is pinned on the way in, so a hand-edited value can
	# never put a row somewhere the panel cannot bring it back from.
	var wild := ConfigFile.new()
	wild.set_value(SettingsScript.SECTION, "field_of_view", 9000.0)
	wild.set_value(SettingsScript.SECTION, "music_volume", -4.0)
	wild.save(path)
	_settings.reset()
	_settings.load_file()
	if float(_settings.field_of_view) != float(_knob("field_of_view")["max"]):
		_fail("a hand-edited FOV of 9000 was not pinned to the maximum")
		return
	if float(_settings.music_volume) != float(_knob("music_volume")["min"]):
		_fail("a hand-edited negative volume was not pinned to the minimum")
		return

	_settings.persist_enabled = false
	_settings.reset()

	# --- The game reads them ----------------------------------------------
	var session: Node = root.get_node_or_null("GameSession")
	if session == null:
		_fail("the GameSession autoload is not registered")
		return
	session.arm_new_game()
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	Layouts.build_machines_fixture(main.sim)
	main._rebuild_world()

	var music: Node = root.get_node_or_null("Music")
	if music == null:
		_fail("the Music autoload is not registered")
		return
	# On the knob's own 10% grid: `_clamped` snaps every write, so a value
	# off the grid would come back as the nearest one and this assertion
	# would be testing the snap rather than the wiring.
	_settings.set_value("music_volume", 0.3)
	if absf(float(music.volume) - 0.3) > 0.0001:
		_fail("MUSIC VOLUME must reach the Music autoload")
		return
	_settings.set_value("music_enabled", false)
	if bool(music.enabled):
		_fail("the MUSIC switch must reach the Music autoload")
		return
	_settings.set_value("sfx_volume", 0.5)
	if absf(float(main._audio.volume) - 0.5) > 0.0001:
		_fail("SOUND VOLUME must reach the cue player")
		return
	_settings.set_value("sfx_enabled", false)
	if bool(main._audio.enabled):
		_fail("the SOUND switch must reach the cue player")
		return
	# Muting must not stop the bookkeeping — the same rule
	# `test_payout_feedback.gd` holds the switch to.
	main._audio.plays.clear()
	main._audio.play(main._audio.PAYOUT)
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 1:
		_fail("a muted game must still bookkeep play(), or mute changes timing")
		return

	# FIELD OF VIEW reaches the first-person camera, live.
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	_settings.set_value("field_of_view", 100.0)
	if absf(main._camera.fov - 100.0) > 0.01:
		_fail("FIELD OF VIEW must reach the camera, got %.2f" % main._camera.fov)
		return

	# MOVEMENT SPEED and LOOK SENSITIVITY multiply the tuned constants.
	_settings.set_value("move_speed", 1.0)
	var origin: Vector3 = main._fp_position
	main._apply_fp_move(Vector3.FORWARD, 1.0)
	var single: float = origin.distance_to(main._fp_position)
	_settings.set_value("move_speed", 2.0)
	main._fp_position = origin
	main._apply_fp_move(Vector3.FORWARD, 1.0)
	var doubled: float = origin.distance_to(main._fp_position)
	if absf(doubled - single * 2.0) > 0.01:
		_fail("MOVEMENT SPEED x2 moved %.3f against x1's %.3f" % [doubled, single])
		return

	_settings.set_value("look_sensitivity", 1.0)
	main._fp_yaw = 0.0
	main._apply_look(Vector2(10.0, 0.0))
	var turn: float = absf(main._fp_yaw)
	_settings.set_value("look_sensitivity", 3.0)
	main._fp_yaw = 0.0
	main._apply_look(Vector2(10.0, 0.0))
	if absf(absf(main._fp_yaw) - turn * 3.0) > 0.0001:
		_fail("LOOK SENSITIVITY x3 turned %.5f against x1's %.5f" % [absf(main._fp_yaw), turn])
		return

	# HALL SIZE reaches the sky shader's three uniforms and only those three.
	var sky_material: ShaderMaterial = main._world_environment.environment.sky.sky_material
	var floor_drop = sky_material.get_shader_parameter("floor_drop")
	_settings.set_value("hall_scale", 2.0)
	if absf(
		float(sky_material.get_shader_parameter("hall_half_width"))
		- FactoryEnvironment.HALL_HALF_WIDTH * 2.0
	) > 0.01:
		_fail("HALL SIZE must reach the shader's hall_half_width")
		return
	if absf(
		float(sky_material.get_shader_parameter("roof_height"))
		- FactoryEnvironment.HALL_ROOF_HEIGHT * 2.0
	) > 0.01:
		_fail("HALL SIZE must scale the roof with the walls, or a bigger hall is a corridor")
		return
	if not is_same(sky_material.get_shader_parameter("floor_drop"), floor_drop):
		_fail("HALL SIZE must leave floor_drop alone — the slab is under the PLAYER")
		return

	# --- The panel itself --------------------------------------------------
	var panel = main._pause_settings_panel
	if panel == null:
		_fail("the pause menu built no settings panel")
		return
	for entry in SettingsScript.KNOBS:
		var key: String = entry["key"]
		if not panel.value_controls.has(key):
			_fail("the settings panel has no row for %s" % key)
			return
		if String(entry["kind"]) == "step" and not panel.range_bars.has(key):
			_fail("the stepped row %s has no range bar" % key)
			return
	# A toggle says which way it is set, in words.
	var music_row: Button = panel.value_controls["music_enabled"]
	_settings.set_value("music_enabled", true)
	panel.refresh()
	if music_row.text != "MUSIC: ON":
		_fail("a toggle row must caption itself in words, got '%s'" % music_row.text)
		return
	music_row.pressed.emit()
	if bool(_settings.music_enabled) or music_row.text != "MUSIC: OFF":
		_fail("pressing a toggle row must flip the setting and the caption")
		return

	# The range bar records its own fraction, so a headless run can read it
	# with no layout pass — the meter language's rule, under its own name.
	_settings.set_value("field_of_view", float(_knob("field_of_view")["max"]))
	panel.refresh()
	var bar: Panel = panel.range_bars["field_of_view"]
	if absf(float(bar.get_meta("fraction", -1.0)) - 1.0) > 0.0001:
		_fail("a maxed row's bar must read full, got %s" % bar.get_meta("fraction", -1.0))
		return
	if bar.name != "SettingRange_field_of_view":
		_fail("a settings bar must not borrow the sim meter's node name, got '%s'" % bar.name)
		return
	_settings.set_value("field_of_view", float(_knob("field_of_view")["min"]))
	panel.refresh()
	if absf(float(bar.get_meta("fraction", -1.0))) > 0.0001:
		_fail("a floored row's bar must read empty")
		return

	# The +/- buttons are the row, and RESET is the way back.
	var plus: Button = panel.root.find_child("Plus_move_speed", true, false)
	var minus: Button = panel.root.find_child("Minus_move_speed", true, false)
	if plus == null or minus == null:
		_fail("a stepped row must have both a + and a - button")
		return
	_settings.set_value("move_speed", 1.0)
	plus.pressed.emit()
	if absf(float(_settings.move_speed) - (1.0 + float(_knob("move_speed")["step"]))) > 0.0001:
		_fail("+ must move MOVEMENT SPEED by exactly one step")
		return
	minus.pressed.emit()
	if absf(float(_settings.move_speed) - 1.0) > 0.0001:
		_fail("- must undo what + did")
		return
	panel.reset_button.pressed.emit()
	if not _settings.is_default():
		_fail("RESET must put every row back")
		return
	if panel.value_controls["music_enabled"].text != "MUSIC: ON":
		_fail("RESET must redraw the rows it changed")
		return

	# BACK closes the page without closing the pause menu behind it.
	main._set_pause_open(true)
	main._set_pause_settings(true)
	if not main._pause_settings_box.visible or main._pause_buttons_box.visible:
		_fail("the settings page must replace the pause menu's buttons, not sit beside them")
		return
	panel.back_button.pressed.emit()
	if main._pause_settings_box.visible or not main._pause_buttons_box.visible:
		_fail("BACK must bring the pause menu's buttons back")
		return
	if not main._pause_open:
		_fail("BACK must leave the pause menu open — it is one step back, not two")
		return
	main._set_pause_open(false)

	# --- The other host builds the same rows -------------------------------
	var menu = load("res://menu.tscn").instantiate()
	root.add_child(menu)
	menu._ready()
	if menu._settings_panel == null:
		_fail("the main menu built no settings panel")
		return
	var menu_keys: Array = menu._settings_panel.value_controls.keys()
	var pause_keys: Array = panel.value_controls.keys()
	menu_keys.sort()
	pause_keys.sort()
	if menu_keys != pause_keys:
		_fail("the two hosts show different rows: menu %s, pause %s" % [menu_keys, pause_keys])
		return
	menu._on_settings_pressed()
	if not menu._settings_open or menu._load_open:
		_fail("SETTINGS on the main menu must open the settings page and nothing else")
		return
	menu._settings_panel.back_button.pressed.emit()
	if menu._settings_open:
		_fail("BACK on the main menu must close the settings page")
		return

	_restore()
	print("SETTINGS_OK: eight knobs at their defaults, every stepper clamped and snapped to")
	print("             its own grid, a real user://settings.cfg round-tripped (and the")
	print("             developer's own put back), a partial or hand-edited file degraded")
	print("             rather than erroring, both volumes and both switches reaching the")
	print("             audio owners, movement/look/FOV/hall reaching the camera and the")
	print("             sky, RESET putting every row back, and both hosts showing the")
	print("             same rows")
	quit(0)
