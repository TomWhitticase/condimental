# Headless proof of camera navigation (docs/UI_UX.md §1: pan and zoom are
# explicitly allowed, only free rotation of the fixed isometric angle isn't).
#
# Run with:
#   godot --headless --path godot_project --script test_camera_nav.gd
#
# Verifies, without a framebuffer:
#   * scrolling changes the orthographic `size` (zoom) and it clamps at both
#     ends instead of zooming forever,
#   * a pan direction moves `_camera_focus` along the screen-relative pan
#     axes (not raw world X/Z), and no direction leaves it untouched,
#   * the camera's orientation (the fixed isometric angle itself) never
#     changes across any of the above — only position and size do,
#   * a macOS trackpad's InputEventPanGesture (not InputEventMouseButton —
#     the two are genuinely different event types, which is why scroll-to-
#     zoom silently didn't work on trackpads until this was added) and the
#     +/- keys both reach `_unhandled_input` and actually zoom,
#   * TAB toggles between ISOMETRIC (orthographic) and FIRST_PERSON
#     (perspective) on the one shared Camera3D node,
#   * in first-person mode, mouse-look rotates the camera and clamps pitch,
#     while WASD movement stays purely horizontal regardless of pitch (E/Q
#     are the only things that change height) — the Minecraft-flight rule,
#   * click-and-drag panning (isometric) moves `_camera_focus` opposite the
#     drag direction along the screen-relative axes, only while the right
#     mouse button is actually held.
#
# `_pan_camera` itself reads the `Input` singleton to decide direction
# (held WASD/arrows) — that part is a thin, deliberately untested read per
# main.gd's own doc comment, since headless `Input.parse_input_event` isn't
# reliably reflected in `is_key_pressed` outside a running main loop. This
# test instead calls `_apply_pan` directly with an explicit direction, which
# is where all the actual movement logic (and everything worth verifying)
# lives.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("CAMERA_NAV_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	var camera: Camera3D = main._camera
	if camera == null:
		_fail("main._camera was not set by _build_camera()")
		return

	var initial_basis: Basis = camera.transform.basis
	var initial_size: float = camera.size
	var initial_focus: Vector3 = main._camera_focus

	# --- Zoom: changes size, clamps at both ends, never touches orientation --
	main._zoom_camera(0.5) # zoom in
	if camera.size >= initial_size:
		_fail("zooming in should decrease size (was %f, now %f)" % [initial_size, camera.size])
		return
	if not camera.transform.basis.is_equal_approx(initial_basis):
		_fail("zoom must not change camera orientation")
		return

	for _i in range(200): # far more than enough to hit the clamp either way
		main._zoom_camera(0.5)
	var min_size: float = main._camera_base_size * main.ZOOM_MIN_FACTOR
	if not is_equal_approx(camera.size, min_size):
		_fail("zoom-in should clamp at ZOOM_MIN_FACTOR * base size (expected ~%f, got %f)" % [min_size, camera.size])
		return

	for _i in range(200):
		main._zoom_camera(2.0)
	var max_size: float = main._camera_base_size * main.ZOOM_MAX_FACTOR
	if not is_equal_approx(camera.size, max_size):
		_fail("zoom-out should clamp at ZOOM_MAX_FACTOR * base size (expected ~%f, got %f)" % [max_size, camera.size])
		return

	main._camera_size = initial_size
	main._update_camera_transform()

	# --- Pan: an explicit direction moves focus along that screen axis -----
	main._apply_pan(main._pan_right, 1.0) # 1 simulated second
	var focus_after_right: Vector3 = main._camera_focus
	if focus_after_right.is_equal_approx(initial_focus):
		_fail("_apply_pan(_pan_right, ...) should move _camera_focus")
		return
	var moved := focus_after_right - initial_focus
	if moved.normalized().dot(main._pan_right) < 0.99:
		_fail("panning right should move along _pan_right specifically, moved by %s" % str(moved))
		return
	if not camera.transform.basis.is_equal_approx(initial_basis):
		_fail("pan must not change camera orientation")
		return

	# A zero direction (nothing held) must not move the focus at all.
	main._apply_pan(Vector3.ZERO, 1.0)
	if not main._camera_focus.is_equal_approx(focus_after_right):
		_fail("_apply_pan(Vector3.ZERO, ...) should not move _camera_focus, but it did")
		return

	# --- Trackpad pan-gesture and +/- keys both reach _unhandled_input -----
	# `_camera_size` is back to `initial_size` here (pan never touched it),
	# comfortably inside the clamp range, so a single zoom step in either
	# direction below is unambiguous — not fighting a clamp.
	var pan_gesture := InputEventPanGesture.new()
	pan_gesture.delta = Vector2(0.0, -5.0) # an upward swipe: same sense as scroll-up
	var size_before_gesture: float = camera.size
	main._unhandled_input(pan_gesture)
	if camera.size >= size_before_gesture:
		_fail("an upward InputEventPanGesture should zoom in (decrease size), was %f now %f" % [size_before_gesture, camera.size])
		return

	var minus_key := InputEventKey.new()
	minus_key.keycode = KEY_MINUS
	minus_key.pressed = true
	var size_before_minus: float = camera.size
	main._unhandled_input(minus_key)
	if camera.size <= size_before_minus:
		_fail("KEY_MINUS should zoom out (increase size), was %f now %f" % [size_before_minus, camera.size])
		return

	var plus_key := InputEventKey.new()
	plus_key.keycode = KEY_EQUAL
	plus_key.pressed = true
	var size_before_plus: float = camera.size
	main._unhandled_input(plus_key)
	if camera.size >= size_before_plus:
		_fail("KEY_EQUAL (+) should zoom in (decrease size), was %f now %f" % [size_before_plus, camera.size])
		return

	# --- Drag-pan (isometric): only while the right button is actually held --
	var drag_start := InputEventMouseButton.new()
	drag_start.button_index = MOUSE_BUTTON_RIGHT
	drag_start.pressed = true
	main._unhandled_input(drag_start)
	if not main._iso_dragging:
		_fail("holding the right mouse button should set _iso_dragging")
		return

	var focus_before_drag: Vector3 = main._camera_focus
	var drag_motion := InputEventMouseMotion.new()
	drag_motion.relative = Vector2(40.0, 0.0) # dragged right
	main._unhandled_input(drag_motion)
	if main._camera_focus.is_equal_approx(focus_before_drag):
		_fail("dragging with the right button held should move _camera_focus")
		return
	# "Grab and drag": dragging right should move the focus opposite
	# _pan_right (the world slides right under the cursor, following it).
	var drag_moved: Vector3 = main._camera_focus - focus_before_drag
	if drag_moved.normalized().dot(main._pan_right) > -0.99:
		_fail("dragging right should move focus opposite _pan_right, moved by %s" % str(drag_moved))
		return

	var drag_end := InputEventMouseButton.new()
	drag_end.button_index = MOUSE_BUTTON_RIGHT
	drag_end.pressed = false
	main._unhandled_input(drag_end)
	if main._iso_dragging:
		_fail("releasing the right mouse button should clear _iso_dragging")
		return

	var focus_after_release: Vector3 = main._camera_focus
	main._unhandled_input(drag_motion) # same motion again, but button is up now
	if not main._camera_focus.is_equal_approx(focus_after_release):
		_fail("mouse motion without the right button held should not pan")
		return

	# --- A release the GUI swallowed still ends the drag (quality pass) -----
	# A drag that ends over a panel with a mouse filter (the hotbar, the
	# clipboard) has its release consumed by that panel, so `_unhandled_input`
	# never sees it; `_input` does, and must clear the drag, or the camera
	# follows every mouse move until the next right click.
	main._unhandled_input(drag_start)
	if not main._iso_dragging:
		_fail("a second right press should start a second drag")
		return
	main._input(drag_end) # the GUI ate it: only `_input` sees the release
	if main._iso_dragging:
		_fail("a right-button release seen by _input alone must end the drag")
		return
	var focus_after_swallowed: Vector3 = main._camera_focus
	main._unhandled_input(drag_motion)
	if not main._camera_focus.is_equal_approx(focus_after_swallowed):
		_fail("after a swallowed release, mouse motion must not pan")
		return
	# The pause menu opening mid-drag is the other lost release.
	main._unhandled_input(drag_start)
	main._set_pause_open(true)
	if main._iso_dragging:
		_fail("opening the pause menu must end a drag in progress")
		return
	main._set_pause_open(false)

	# --- TAB toggles ISOMETRIC <-> FIRST_PERSON on the shared Camera3D ------
	var tab_key := InputEventKey.new()
	tab_key.keycode = KEY_TAB
	tab_key.pressed = true

	# --- TAB with a panel up must NOT capture the mouse (quality pass) ------
	# The capture decision used to check the build menu alone, so TAB with
	# the market (or the board, staff, production or a configuration panel)
	# open captured the cursor under a panel full of buttons. Headless Godot
	# ignores `Input.mouse_mode`, so the DECISION is what is asserted:
	# `_fp_mouse_free` is what every first-person capture seam reads.
	main._set_market_open(true)
	main._unhandled_input(tab_key)
	if main._view_mode != main.ViewMode.FIRST_PERSON:
		_fail("TAB should switch to FIRST_PERSON with the market open")
		return
	if not main._fp_mouse_free():
		_fail("with the market open the first-person mouse must stay free (visible), not captured")
		return
	if main._crosshair != null and main._crosshair.visible:
		_fail("the crosshair must not show while the market holds the cursor")
		return
	main._set_market_open(false)
	if main._fp_mouse_free():
		_fail("with every panel closed the first-person mouse should be captured")
		return
	main._unhandled_input(tab_key) # back to isometric for the checks below
	if main._view_mode != main.ViewMode.ISOMETRIC:
		_fail("TAB again should return to ISOMETRIC")
		return

	main._unhandled_input(tab_key)
	if main._view_mode != main.ViewMode.FIRST_PERSON:
		_fail("TAB should switch to FIRST_PERSON")
		return
	if camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		_fail("first-person mode must use a perspective camera, projection was %d" % camera.projection)
		return

	# --- Entering first person PLACES the camera, before any input ----------
	# The bug this guards (fixed 2026-09-16): `_set_view_mode` ended with an
	# unconditional `_update_camera_transform()`, so the first frame of first
	# person was rendered from the isometric camera's position and angle and
	# only snapped into place on the first mouse-look or WASD input. The
	# switch must leave the camera exactly where `_update_fp_camera_transform`
	# would put it, with nothing pressed.
	var placed_on_entry := camera.transform
	main._update_fp_camera_transform()
	if not placed_on_entry.is_equal_approx(camera.transform):
		_fail("entering first person must place the camera itself: it sat at %s until the first input moved it to %s" % [placed_on_entry, camera.transform])
		return
	if not placed_on_entry.origin.is_equal_approx(main._fp_position):
		_fail("the first-person camera must stand at _fp_position %s, not %s" % [main._fp_position, placed_on_entry.origin])
		return

	# --- First-person mouse-look: rotates and clamps pitch ------------------
	var yaw_before: float = main._fp_yaw
	main._apply_look(Vector2(100.0, 0.0)) # simulate the mouse moving right
	if is_equal_approx(main._fp_yaw, yaw_before):
		_fail("_apply_look with a nonzero X delta should change _fp_yaw")
		return

	main._fp_pitch = 0.0
	for _i in range(2000): # far more than enough to hit the clamp
		main._apply_look(Vector2(0.0, -1000.0)) # looking up, repeatedly
	if not is_equal_approx(main._fp_pitch, main.FP_PITCH_LIMIT):
		_fail("pitch should clamp at FP_PITCH_LIMIT (%f), got %f" % [main.FP_PITCH_LIMIT, main._fp_pitch])
		return

	# --- First-person movement: WASD stays horizontal regardless of pitch ---
	# Pitch is still pinned at the clamp from above (looking almost straight
	# up) — if movement used the tilted look direction instead of a yaw-only
	# heading, "forward" would shoot the camera upward. It must not.
	var fp_pos_before: Vector3 = main._fp_position
	main._apply_fp_move(main._fp_heading_basis().z * -1.0, 1.0) # "forward" for 1 second
	var fp_moved: Vector3 = main._fp_position - fp_pos_before
	if not is_zero_approx(fp_moved.y):
		_fail("forward movement must stay horizontal even while pitched up, but Y moved by %f" % fp_moved.y)
		return
	if is_zero_approx(fp_moved.length()):
		_fail("_apply_fp_move with a nonzero direction should move _fp_position")
		return

	# E/Q (up/down) are the only things that change height in first-person.
	var fp_pos_before_up: Vector3 = main._fp_position
	main._apply_fp_move(Vector3.UP, 1.0)
	if main._fp_position.y <= fp_pos_before_up.y:
		_fail("_apply_fp_move(Vector3.UP, ...) should increase _fp_position.y")
		return

	# --- TAB back to ISOMETRIC restores the orthographic camera -------------
	main._unhandled_input(tab_key)
	if main._view_mode != main.ViewMode.ISOMETRIC:
		_fail("TAB again should switch back to ISOMETRIC")
		return
	if camera.projection != Camera3D.PROJECTION_ORTHOGONAL:
		_fail("switching back to isometric must restore the orthographic camera, projection was %d" % camera.projection)
		return

	print("CAMERA_NAV_OK: zoom clamps at both ends, pan moves along screen axes, trackpad gesture and +/- keys both zoom, drag-pan (and a release the GUI or the pause menu swallowed still ends it) and TAB view-mode switching (never capturing the mouse under a panel) and first-person look/move all work, orientation never changes in isometric mode")
	quit(0)
