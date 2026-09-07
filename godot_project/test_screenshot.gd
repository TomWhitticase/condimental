# Renders the real scene to a PNG so a visual change can actually be
# reviewed — by a person or by a Claude Code session reading the image.
#
# The rest of this project's verification is headless (ARCHITECTURE.md §7),
# and headless Godot has no framebuffer, so none of it can see the thing
# docs/UI_UX.md is actually about. This script closes that gap without
# needing a human at a screen: it runs main.tscn with real rendering, waits
# for the lighting/shadow/glow passes to settle, grabs the viewport, and
# exits.
#
# Run via scripts/screenshot-scene.sh, or directly:
#   godot --path godot_project --script test_screenshot.gd -- /abs/out.png [--populate]
#
# `--populate` runs the chain forward first so the screenshot shows product
# actually moving, not an idle factory — the belts and item models are half
# the point of the visual pass.
#
# `--first-person` switches to the first-person camera before rendering
# (looking slightly up, from height, so the sky/ground extent is actually in
# frame) — the only way to visually review that mode's lighting/skybox
# without a live interactive session.
extends SceneTree

## Enough frames for shadow atlases, SSAO and glow to be resolved — a
## screenshot on frame 1 shows an unlit, shadowless scene and is misleading.
const WARMUP_FRAMES := 45

## Spawn attempts, and simulation ticks between them, used by `--populate`.
## Driven straight through the scene's real input and tick path rather than
## poking at simulation state — this is a screenshot of the actual game.
const POPULATE_SPAWNS := 60
const POPULATE_TICKS_PER_SPAWN := 10

var _frames := 0
var _out := "user://scene.png"
var _populate := false
var _first_person := false
var _main: Node


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		_out = args[0]
	_populate = args.has("--populate")
	_first_person = args.has("--first-person")
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	# NOT here: in a real (non-headless) run, add_child()'s _ready() is
	# deferred to the engine's own next-frame processing, unlike the pure
	# SceneTree tests that call _ready() directly — _main._camera doesn't
	# exist yet at this point. Anything that touches it has to wait for
	# _process()'s first frame instead. (Learned the hard way: this used to
	# live here and crashed with "base object of type Nil".)


func _enter_first_person() -> void:
	_main._set_view_mode(_main.ViewMode.FIRST_PERSON)
	# Higher up and looking a bit further up than the default entry pose, so
	# the sky/horizon/ground extent — the thing being reviewed — is actually
	# in frame, not just a close-up of one machine.
	_main._fp_position += Vector3(0.0, 4.0, 0.0)
	_main._fp_pitch = deg_to_rad(5.0)
	_main._update_fp_camera_transform()


func _run_chain_forward() -> void:
	# Calls SimBridge directly rather than simulating a SPACE keypress
	# through _unhandled_input: SPACE only spawns in ISOMETRIC mode (it's
	# "fly up" in first-person), so going through the view-mode-sensitive
	# input path would silently do nothing when --first-person is also set.
	for i in range(POPULATE_SPAWNS):
		_main.sim.issue_command_json(JSON.stringify({"type": "spawn_item", "item": "tomato"}))
		for j in range(POPULATE_TICKS_PER_SPAWN):
			_main.sim.tick()
	_main._apply_dirty_chunks()


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames == 1 and _first_person:
		_enter_first_person()
	if _frames == 2 and _populate:
		_run_chain_forward()
	if _frames < WARMUP_FRAMES:
		return false

	var image := root.get_texture().get_image()
	if image == null:
		printerr("SCREENSHOT_FAIL: no viewport image (is this running with --headless?)")
		return true
	var err := image.save_png(_out)
	if err != OK:
		printerr("SCREENSHOT_FAIL: could not write %s (error %d)" % [_out, err])
		return true

	print("SCREENSHOT_OK: %s (%dx%d)" % [_out, image.get_width(), image.get_height()])
	return true
