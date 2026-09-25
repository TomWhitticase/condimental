# Headless proof that a review render's camera cannot be moved by the
# machine it is rendered on — the second half of issue #82.
#
# Run with:
#   godot --headless --path godot_project --script test_camera_seal.gd
#
# WHY THIS EXISTS. `scripts/screenshot-scene.sh` aims the camera itself
# (`--closeup`, `--first-person`, a fixture's framing) and then lets the
# scene run 45 ordinary frames before the shutter. Those frames poll the
# REAL keyboard in `_pan_camera` and see the REAL mouse in `_input`, so a
# key held in another window, a stray wheel notch, or the OS pointer another
# process warps moved the camera between the aim and the shutter. It was
# measured: with one pan key held, `--bottleneck --closeup 4,0,10 --no-hud`
# rendered a8b1b9e8a63c instead of 52493c1c3da8 on the same simulation tick,
# the camera having drifted about a tile and a half in 43 frames. The
# picture was of somewhere the command never asked for, and nothing said so.
#
# `main.gd`'s `_camera_input_sealed` shuts every door live input comes
# through, and `test_screenshot.gd` sets it on frame 1 before it parks the
# cursor. This proves each door: sealed, the five entry points do nothing;
# unsealed, every one of them still moves the camera, which is what stops
# this being a test that would pass with the whole feature deleted.
#
# The flag is never set for a player — it is a property of a photograph, not
# a mode of the game — so `_camera_input_sealed` starting false is checked
# first, and checked as a claim of its own.
extends SceneTree

## A shove far larger than any real input, so a guard that merely damps
## rather than blocks is caught as loudly as one that is missing.
const SHOVE_PX := Vector2(120.0, 90.0)
const SHOVE_SECONDS := 1.0

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("CAMERA_SEAL_FAIL: %s" % msg)
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


## The camera's whole movable state, as one comparable value.
func _pose() -> Array:
	return [
		main._camera_focus, main._camera_size,
		main._fp_position, main._fp_yaw, main._fp_pitch,
	]


## Every way live input moves the camera, called directly with an explicit
## argument — the polled-key readers (`_pan_input_direction`,
## `_fp_input_direction`) are the one part of panning that cannot be driven
## headlessly, so what is exercised here is what they feed.
func _shove(what: String) -> void:
	match what:
		"pan":
			main._apply_pan(main._pan_right, SHOVE_SECONDS)
		"drag":
			main._apply_drag_pan(SHOVE_PX)
		"zoom":
			main._zoom_camera(2.0)
		"look":
			main._apply_look(SHOVE_PX)
		"walk":
			main._apply_fp_move(Vector3.FORWARD, SHOVE_SECONDS)
		_:
			_fail("no such shove: %s" % what)


func _run_checks() -> void:
	if main._camera_input_sealed:
		_fail(
			"a scene that nothing has photographed starts with its camera sealed — "
			+ "the seal belongs to a render, and a player who cannot pan is the "
			+ "regression this ordering is here to catch"
		)
		return

	# Unsealed first: each door has to be shown to be a door, or "sealed"
	# proves nothing about it.
	var moved: Array[String] = []
	for what in ["pan", "drag", "zoom", "look", "walk"]:
		# First person for the two first-person controls, isometric for the
		# rest, because each is ignored in the other mode by design.
		main._view_mode = (
			main.ViewMode.FIRST_PERSON if what in ["look", "walk"] else main.ViewMode.ISOMETRIC
		)
		var before := _pose()
		_shove(what)
		if _pose() == before:
			_fail(
				"unsealed, %s did not move the camera at all (%s) — this test cannot "
				% [what, before]
				+ "tell a working seal from a control that no longer does anything"
			)
			return
		moved.append(what)

	# Sealed: the same five shoves, from a pose recorded after each one, and
	# nothing may move.
	main._camera_input_sealed = true
	for what in moved:
		main._view_mode = (
			main.ViewMode.FIRST_PERSON if what in ["look", "walk"] else main.ViewMode.ISOMETRIC
		)
		var before := _pose()
		_shove(what)
		if _pose() != before:
			_fail(
				"sealed, %s still moved the camera: %s -> %s. A render aims the camera "
				% [what, before, _pose()]
				+ "and then runs 45 frames; anything that can move it in between is a "
				+ "photograph of somewhere the command did not ask for (issue #82)"
			)
			return

	# And the seal is a switch, not a one-way door: a scene that stops being
	# photographed is a scene a player can drive again.
	main._camera_input_sealed = false
	main._view_mode = main.ViewMode.ISOMETRIC
	var before_unseal := _pose()
	_shove("pan")
	if _pose() == before_unseal:
		_fail("unsealing did not give the camera back: pan still does nothing")
		return

	print(
		"CAMERA_SEAL_OK: a sealed camera ignores all five ways live input moves it "
		+ "(pan, right-drag, zoom, first-person look, first-person walk), every one of "
		+ "which moves it when unsealed; the seal is off in a scene nothing is "
		+ "photographing, and unsealing gives the camera back"
	)
	quit(0)
