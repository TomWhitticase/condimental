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
#     changes across any of the above — only position and size do.
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

	print("CAMERA_NAV_OK: zoom clamps at both ends, pan moves along screen axes, orientation never changes")
	quit(0)
