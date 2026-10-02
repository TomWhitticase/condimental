# Headless proof that holding CTRL in first person hands the mouse to the HUD
# (the owner's request, 2026-10-02: "if i hold ctrl i can click items in my
# hotbar with my mouse, and when i release control it goes back to
# controlling my looking around").
#
# Run with:
#   godot --headless --path godot_project --script test_ctrl_cursor.gd
#
# Every event below is a REAL one pushed through the viewport
# (`root.push_input`) except mouse motion (see `_motion`), so it goes `_input` -> GUI -> `_unhandled_input` the
# way a keyboard and mouse do — a click on a hotbar slot is taken by the
# slot's own Button, not by a test calling its handler. Headless Godot ignores
# `Input.mouse_mode`, so the capture DECISION (`_fp_wanted_mouse_mode`) is
# what is asserted, as test_camera_nav.gd does for the panels. It checks:
#
#   * before CTRL, first person captures the mouse and mouse motion looks,
#   * CTRL held: the mouse is wanted VISIBLE, the crosshair goes, mouse motion
#     no longer turns the camera, and the movement keys stay live (CTRL takes
#     the mouse only, not the keyboard the panels take),
#   * a CTRL-held left click on a hotbar slot selects that slot, and so does
#     the CTRL-held RIGHT click macOS turns a CTRL+click into,
#   * a CTRL-held click off the HUD, with a buildable ghost under the
#     crosshair, places nothing — and the same click without CTRL does place
#     (the control that proves the first half could have failed),
#   * CTRL released: the mouse is wanted CAPTURED again, the crosshair is
#     back, the first motion after the recapture (the OS warping the pointer
#     home) is dropped and the next one looks,
#   * a panel closing under a held CTRL leaves the cursor free, and losing
#     window focus forgets a CTRL whose release went to another window,
#   * the first-person legend says so.
extends SceneTree

var _main
var _frame := 0
var _kind := ""
var _slot_index := -1
var _done := false


func _fail(msg: String) -> void:
	printerr("CTRL_CURSOR_FAIL: %s" % msg)
	_done = true
	quit(1)


func _key(code: int, pressed: bool) -> void:
	var ev := InputEventKey.new()
	ev.keycode = code
	ev.physical_keycode = code
	ev.pressed = pressed
	ev.ctrl_pressed = pressed and code == KEY_CTRL
	root.push_input(ev)


## Mouse-look is `_unhandled_input`'s, and a captured pointer's motion
## reaches it over no Control; under `--script` the HUD plates sit wherever
## the 64x64 boot window put them, so a pushed motion would land on one.
## Motion is handed to the handler directly — keys and clicks, whose routing
## through `_input` and the GUI IS the thing under test, are pushed.
func _motion(relative: Vector2) -> void:
	_main._unhandled_input(_make_motion(relative))


func _click(at: Vector2, button: int, ctrl: bool) -> void:
	for pressed in [true, false]:
		var ev := InputEventMouseButton.new()
		ev.button_index = button
		ev.pressed = pressed
		ev.ctrl_pressed = ctrl
		ev.position = at
		ev.global_position = at
		ev.button_mask = (MOUSE_BUTTON_MASK_LEFT if button == MOUSE_BUTTON_LEFT else MOUSE_BUTTON_MASK_RIGHT) if pressed else 0
		# In the viewport's own coordinates — the ones `get_global_rect`
		# answers in — not the window's, which the stretch mode scales.
		root.push_input(ev, true)


## The centre of `target` as the GUI's own picker sees it, or (-1, -1) when
## no on-screen point picks it — so a slot that cannot be clicked fails
## here rather than passing on a rect nobody can reach. The rect is the aim;
## the picker is the proof.
func _gui_point_on(target: Control) -> Vector2:
	var at := target.get_global_rect().get_center()
	if not root.get_visible_rect().has_point(at):
		return Vector2(-1.0, -1.0)
	var mv := InputEventMouseMotion.new()
	mv.position = at
	mv.global_position = at
	root.push_input(mv, true)
	return at if root.gui_get_hovered_control() == target else Vector2(-1.0, -1.0)


func _placements() -> int:
	var parsed = JSON.parse_string(_main.sim.world_layout_json())
	return (parsed as Array).size() if parsed is Array else -1


func _initialize() -> void:
	_main = load("res://main.tscn").instantiate()
	# No explicit `_ready()`: the tree runs it when the main loop starts, and
	# a second call built a second HUD whose slots shadowed the first's.
	root.add_child(_main)


## Done on a frame, once the scene's `_ready` has run.
func _setup() -> void:
	if _main._view_mode != _main.ViewMode.FIRST_PERSON:
		_fail("the game must open in first person for this test to mean anything")
		return
	# Something to hold: the first hotbar slot's kind, two of them, so the
	# placement control at the end still has one left to place.
	_slot_index = 0
	_kind = String(_main._hotbar_slots[_slot_index]["kind"])
	_main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": _kind, "quantity": 2}))
	_main._refresh_owned_counts()
	_main._update_hotbar()
	# Look down at the floor in front, so the crosshair has a tile to build on.
	_main._fp_pitch = deg_to_rad(-55.0)
	_main._update_fp_camera_transform()


func _process(_delta: float) -> bool:
	if _done:
		return true
	_frame += 1
	# Frames for the HUD's containers to lay the strip out, re-anchored once
	# the 1280x720 root has taken (`_position_hotbar` is a MINSIZE preset).
	# The `--script` window is 64x64, and it reverts any size set before the
	# first frame — so the real one is set here, then the strip re-anchored.
	if _frame == 3:
		root.size = Vector2i(1280, 720)
		_main._position_hotbar()
		_setup()
	if _frame < 6:
		return false
	_run()
	_done = true
	return true


func _run() -> void:
	var main = _main
	var camera: Camera3D = main._camera

	# --- Before CTRL: captured, crosshair up, motion looks ------------------
	if main._fp_ui_cursor() or main._fp_wanted_mouse_mode() != Input.MOUSE_MODE_CAPTURED:
		_fail("with nothing open and no CTRL, first person must want the mouse captured")
		return
	if not main._crosshair.visible:
		_fail("the crosshair must be up before CTRL is pressed")
		return
	# The opening capture may have armed the recapture skip (headless reports
	# the mouse as never captured); spend it, then prove motion looks.
	_motion(Vector2(30.0, 0.0))
	var yaw: float = main._fp_yaw
	_motion(Vector2(30.0, 0.0))
	if is_equal_approx(main._fp_yaw, yaw):
		_fail("mouse motion must turn the first-person camera before CTRL is held")
		return
	if main._fp_skip_next_look:
		_fail("the recapture skip must be spent by the first look motion")
		return

	# --- CTRL held: cursor to the HUD ---------------------------------------
	_key(KEY_CTRL, true)
	if not main._ctrl_held:
		_fail("a pushed CTRL press never reached main's `_input`")
		return
	if main._fp_wanted_mouse_mode() != Input.MOUSE_MODE_VISIBLE:
		_fail("CTRL held in first person must want the mouse VISIBLE")
		return
	if main._crosshair.visible:
		_fail("the crosshair must go while CTRL hands the player a real cursor")
		return
	if not main._camera_keys_live():
		_fail("CTRL takes the mouse only — WASD must still move the player")
		return
	yaw = main._fp_yaw
	var pitch: float = main._fp_pitch
	var basis_before: Basis = camera.global_transform.basis
	_motion(Vector2(250.0, -120.0))
	if not is_equal_approx(main._fp_yaw, yaw) or not is_equal_approx(main._fp_pitch, pitch) \
			or not camera.global_transform.basis.is_equal_approx(basis_before):
		_fail("mouse motion with CTRL held must not turn the camera")
		return

	# --- A CTRL-held click on a hotbar slot selects it -----------------------
	var button: Button = main._hotbar_slots[_slot_index]["button"]
	var at := _gui_point_on(button)
	if at.x < 0.0:
		_fail("no point on screen hit-tests to hotbar slot %d — there is nothing to click" % _slot_index)
		return
	if main._build_kind != "":
		_fail("nothing should be in hand before the slot is clicked")
		return
	_click(at, MOUSE_BUTTON_LEFT, true)
	if main._build_kind != _kind:
		_fail("a CTRL-held left click on hotbar slot %d must select '%s', hand holds '%s'" % [
			_slot_index, _kind, main._build_kind,
		])
		return
	# macOS delivers CTRL+click as a RIGHT click; it must select just the same.
	main._exit_build_mode()
	main._update_hotbar()
	_click(at, MOUSE_BUTTON_RIGHT, true)
	if main._build_kind != _kind:
		_fail("a CTRL-held RIGHT click (macOS's CTRL+click) on hotbar slot %d must select '%s'" % [
			_slot_index, _kind,
		])
		return

	# --- A CTRL-held click off the HUD places nothing ------------------------
	main._update_ghost()
	# ...and while CTRL holds the pointer, the prompt that says "click to
	# build" is not up to say it, nor over the hotbar's own tooltip.
	if main._placement_prompt.visible:
		_fail("the placement prompt must step aside while CTRL has the pointer on the HUD")
		return
	if main._ghost_tile == main.HOVER_NO_TILE or not main._ghost_valid:
		_fail("the fixture's crosshair is not on a buildable tile (%s, valid=%s) — the no-place check would prove nothing" % [
			main._ghost_tile, main._ghost_valid,
		])
		return
	var centre: Vector2 = root.get_visible_rect().get_center()
	var before := _placements()
	var owned_before: int = main._owned_of(_kind)
	_click(centre, MOUSE_BUTTON_LEFT, true)
	main._unhandled_input(_make_click(centre, true))
	if _placements() != before or main._owned_of(_kind) != owned_before:
		_fail("a CTRL-held click in first person placed a '%s' — that click belongs to the HUD" % _kind)
		return
	if main._config_open:
		_fail("a CTRL-held click in first person opened a configuration panel")
		return

	# --- A panel closing under a held CTRL leaves the cursor free ------------
	main._set_market_open(true)
	main._set_market_open(false)
	if main._fp_wanted_mouse_mode() != Input.MOUSE_MODE_VISIBLE:
		_fail("closing a panel while CTRL is still held must not capture the mouse")
		return

	# --- CTRL released: back to looking ---------------------------------------
	_key(KEY_CTRL, false)
	if main._ctrl_held:
		_fail("a pushed CTRL release never reached main's `_input`")
		return
	if main._fp_wanted_mouse_mode() != Input.MOUSE_MODE_CAPTURED:
		_fail("releasing CTRL in first person must want the mouse CAPTURED again")
		return
	if not main._crosshair.visible:
		_fail("the crosshair must come back when CTRL is released")
		return
	if not main._fp_skip_next_look:
		_fail("a recapture must arm the one-motion skip, or the pointer's warp home swings the camera")
		return
	yaw = main._fp_yaw
	_motion(Vector2(400.0, 0.0))
	if not is_equal_approx(main._fp_yaw, yaw):
		_fail("the first motion after a recapture is the pointer warping home and must not turn the camera")
		return
	_motion(Vector2(30.0, 0.0))
	if is_equal_approx(main._fp_yaw, yaw):
		_fail("after CTRL is released, mouse motion must turn the camera again")
		return

	# The control: the same click, without CTRL, does place.
	main._fp_pitch = deg_to_rad(-55.0)
	main._update_fp_camera_transform()
	main._update_ghost()
	if not main._placement_prompt.visible:
		_fail("the placement prompt must come back when CTRL is released")
		return
	_click(centre, MOUSE_BUTTON_LEFT, false)
	if _placements() != before + 1:
		_fail("the control failed: a plain first-person click did not place, so the CTRL check above proves nothing")
		return

	# --- Focus loss forgets CTRL ----------------------------------------------
	_key(KEY_CTRL, true)
	main._notification(Node.NOTIFICATION_APPLICATION_FOCUS_OUT)
	if main._ctrl_held or main._fp_wanted_mouse_mode() != Input.MOUSE_MODE_CAPTURED:
		_fail("losing window focus must forget CTRL, or the cursor stays free with nobody holding it")
		return

	# --- The legend says so -----------------------------------------------------
	if main.FP_HELP_TEXT.find("CTRL") < 0:
		_fail("FP_HELP_TEXT must tell the player CTRL frees the mouse for the hotbar")
		return

	print("CTRL_CURSOR_OK: CTRL in first person freed the mouse (crosshair off, look off, WASD live); a CTRL click on slot %d selected '%s' (left and macOS's right); a CTRL click on a buildable tile placed nothing (prompt hidden) while a plain one placed; a panel closing under CTRL kept the cursor; release recaptured, dropped the warp motion, then looked; focus loss forgot CTRL" % [
		_slot_index, _kind,
	])
	quit(0)


func _make_motion(relative: Vector2) -> InputEventMouseMotion:
	var ev := InputEventMouseMotion.new()
	ev.relative = relative
	return ev


func _make_click(at: Vector2, ctrl: bool) -> InputEventMouseButton:
	var ev := InputEventMouseButton.new()
	ev.button_index = MOUSE_BUTTON_LEFT
	ev.pressed = true
	ev.ctrl_pressed = ctrl
	ev.position = at
	return ev
