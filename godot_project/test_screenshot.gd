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
var _main: Node


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		_out = args[0]
	_populate = args.has("--populate")
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)


func _run_chain_forward() -> void:
	var space := InputEventKey.new()
	space.keycode = KEY_SPACE
	space.pressed = true
	for i in range(POPULATE_SPAWNS):
		_main._unhandled_input(space)
		for j in range(POPULATE_TICKS_PER_SPAWN):
			_main.sim.tick()
	_main._apply_dirty_chunks()


func _process(_delta: float) -> bool:
	_frames += 1
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
