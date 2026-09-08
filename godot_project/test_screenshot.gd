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
#   godot --path godot_project --script test_screenshot.gd -- /abs/out.png [flags]
#
# **THE WORLD STARTS EMPTY.** Since the "Real Placement" epic a new game
# contains nothing at all, so most flags below now begin by BUILDING
# something — and what they can build is bounded by `sim::STARTING_MONEY`
# (200), which is nothing like the shipped six-stage line. See
# test_layouts.gd's header for the full note; the practical consequence is
# that `--contracts`, `--aging` and `--hover-aging` are gone, because each of
# them needed a factory a starting balance cannot buy.
#
# `--build` puts down the demo line (test_layouts.gd's `build_demo_line`): a
# hopper feeding belts that run east, TURN NORTH into a crusher facing WEST,
# which discharges back west, with a pipe run heading SOUTH beside it. Three
# of the four facings on belts and the fourth on the pipes, which is what
# makes it the right subject for reviewing per-tile rotation and the
# junction art's east-only fallback in one frame.
#
# `--fixture <name>` picks a different one: `machines` (hopper, machine and
# dock, i.e. one of each sort of building), `fluids` (a tank, a pipe run and
# the machine that taps it), `vat` (the 2x2 cooking vat, alone — it costs the
# entire starting balance) or `rack` (the aging rack, alone).
#
# `--populate` runs whatever has been built forward first so the screenshot
# shows product actually moving, not an idle factory.
#
# `--ghost <kind>` buys one of `kind` and enters placement mode with the
# cursor over open ground, so the VALID ghost and its prompt are in the shot.
# `--ghost-blocked <kind>` does the same with the cursor over something
# already placed, for the invalid state. `--remove-mode` enters remove mode
# with the cursor over a placed tile. These three are the only way to review
# build mode without a live interactive session — the same problem the hover
# flags solve for tooltips.
#
# `--first-person` switches to the first-person camera before rendering
# (looking slightly up, from height, so the sky/ground extent is actually in
# frame) — the only way to visually review that mode's lighting/skybox
# without a live interactive session.
#
# `--hover` warps the OS cursor over the first placed machine before
# rendering, so the hover tooltip (docs/UI_UX.md §5) actually shows up in the
# shot. `--hover-pipe`, `--hover-hopper`, `--hover-tank`, `--hover-dock` and
# `--hover-vat` do the same over the first placed pipe, hopper, fluid tank,
# dock, or the FAR CORNER of the first machine whose footprint is bigger than
# one tile (deliberately not its origin — "hovering anywhere on a multi-tile
# machine identifies it" is the claim, and hovering the origin would not show
# it). All of them share one implementation (`_hover_over_tile`) and differ
# only in how they pick the tile out of `world_layout_json`.
#
# `--shop` opens the build menu (docs/UI_UX.md §5) before rendering.
#
# `--closeup TX,TY[,SIZE]` re-aims the ISOMETRIC camera at one simulation
# tile and zooms it right in (SIZE is the orthographic width in world units,
# default CLOSEUP_SIZE). The whole-factory shot is the right frame for "does
# the line read", and the wrong one for "does this belt actually meet this
# machine" — a junction is about four voxels of detail and at 1600x900 the
# full line gives it about six pixels.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

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
var _build := false
var _fixture := ""
var _populate := false
var _first_person := false
var _hover := false
var _hover_pipe := false
var _hover_vat := false
var _hover_hopper := false
var _hover_tank := false
var _hover_dock := false
var _shop := false
var _ghost_kind := ""
var _ghost_blocked := false
var _remove_mode := false
## `--closeup` target tile and orthographic width; `_closeup` gates both.
const CLOSEUP_SIZE := 9.0
var _closeup := false
var _closeup_tile := Vector2.ZERO
var _closeup_size := CLOSEUP_SIZE
var _main: Node


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		_out = args[0]
	_build = args.has("--build")
	_fixture = _string_arg(args, "--fixture")
	_populate = args.has("--populate")
	_first_person = args.has("--first-person")
	_hover = args.has("--hover")
	_hover_pipe = args.has("--hover-pipe")
	_hover_vat = args.has("--hover-vat")
	_hover_hopper = args.has("--hover-hopper")
	_hover_tank = args.has("--hover-tank")
	_hover_dock = args.has("--hover-dock")
	_shop = args.has("--shop")
	_ghost_kind = _string_arg(args, "--ghost")
	if _ghost_kind == "":
		_ghost_kind = _string_arg(args, "--ghost-blocked")
		_ghost_blocked = _ghost_kind != ""
	_remove_mode = args.has("--remove-mode")
	_parse_closeup(args)
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	# NOT here: in a real (non-headless) run, add_child()'s _ready() is
	# deferred to the engine's own next-frame processing, unlike the pure
	# SceneTree tests that call _ready() directly — _main._camera doesn't
	# exist yet at this point. Anything that touches it has to wait for
	# _process()'s first frame instead. (Learned the hard way: this used to
	# live here and crashed with "base object of type Nil".)


## `--flag value` as one look-up, since every option below that takes an
## argument takes exactly one.
func _string_arg(args: Array, flag: String) -> String:
	for i in range(args.size()):
		if String(args[i]) == flag and i + 1 < args.size():
			return String(args[i + 1])
	return ""


## Builds whatever the flags asked for, through the same purchase/place
## commands a player uses (test_layouts.gd). The camera is re-framed
## afterwards, because it frames the placed world and there was nothing to
## frame when `_ready` ran.
func _build_fixture() -> void:
	match _fixture:
		"machines":
			Layouts.build_machines_fixture(_main.sim)
		"fluids":
			Layouts.build_fluid_run(_main.sim)
		"vat":
			Layouts.build_multi_tile(_main.sim)
		"rack":
			Layouts.build_aging_rack(_main.sim)
		_:
			Layouts.build_demo_line(_main.sim)
	_main._rebuild_world()
	_main._refresh_owned_counts()
	_reframe()


## Re-runs the isometric camera's own auto-framing against the world as it now
## stands. The scene frames once at startup, deliberately (re-framing on every
## placement would yank the view around while the player builds), so a
## screenshot that builds a factory after `_ready` has to ask for it.
func _reframe() -> void:
	_main._frame_bounds = _main._framing_bounds()
	_main._camera_focus = Vector3(
		(float(_main._frame_bounds.position.x) + float(_main._frame_bounds.size.x) * 0.5) * _main.TILE,
		_main.TILE * 0.35,
		(float(_main._frame_bounds.position.y) + float(_main._frame_bounds.size.y) * 0.5) * _main.TILE
	)
	_main._camera_base_size = _main._framing_size(_main._camera.transform.basis)
	_main._camera_size = _main._camera_base_size
	_main._update_camera_transform()


## `--closeup TX,TY[,SIZE]` — the argument is one comma-separated token so
## it survives the `--` passthrough without an argument-order convention.
func _parse_closeup(args: Array) -> void:
	for i in range(args.size()):
		if String(args[i]) != "--closeup":
			continue
		if i + 1 >= args.size():
			return
		var parts: PackedStringArray = String(args[i + 1]).split(",")
		if parts.size() < 2:
			return
		_closeup = true
		# Fractional on purpose: a junction sits ON a tile boundary, so the
		# frame worth photographing is usually "half a tile past the belt".
		_closeup_tile = Vector2(float(parts[0]), float(parts[1]))
		if parts.size() >= 3:
			_closeup_size = float(parts[2])
		return


## Aims the isometric camera at one tile, close in. Deliberately writes
## `_camera_focus`/`_camera_size` and calls the scene's own
## `_update_camera_transform` rather than posing the Camera3D directly, so
## what this photographs is the same camera the game uses (UI_UX.md §1's
## fixed angle included) at a zoom the player could also reach.
func _enter_closeup() -> void:
	_main._camera_focus = Vector3(
		_closeup_tile.x * _main.TILE, _main.TILE * 0.35, _closeup_tile.y * _main.TILE
	)
	_main._camera_size = _closeup_size
	_main._update_camera_transform()


func _enter_first_person() -> void:
	_main._set_view_mode(_main.ViewMode.FIRST_PERSON)
	# Higher up and looking a bit further up than the default entry pose, so
	# the sky/horizon/ground extent — the thing being reviewed — is actually
	# in frame, not just a close-up of one machine.
	_main._fp_position += Vector3(0.0, 4.0, 0.0)
	_main._fp_pitch = deg_to_rad(5.0)
	_main._update_fp_camera_transform()


## Warps the real OS cursor over one world tile, using the same tile ->
## world -> screen math `_tile_under_screen_pos` inverts (via the camera's own
## `unproject_position`, which needs the camera actually inside the tree — see
## the `_ready()` timing note in `_initialize` above, which is exactly why
## this can't run any earlier than `_process`). Every --hover* flag is this
## one function plus a different way of picking the tile.
func _hover_over_tile(tile: Vector2i) -> void:
	var world_point: Vector3 = _main._tile_origin(tile.x, tile.y, _main.BELT_SURFACE_Y)
	Input.warp_mouse(_main._camera.unproject_position(world_point))


## The first placed thing of a given kind, or HOVER_NONE. Everything the
## hover flags need comes out of `world_layout_json`, so none of them types a
## tile in — which is what keeps them meaningful when the fixture changes.
const HOVER_NONE := Vector2i(-999999, -999999)


func _first_tile_of(predicate: Callable) -> Vector2i:
	for placement in JSON.parse_string(_main.sim.world_layout_json()):
		if predicate.call(placement):
			var pair: Array = placement["tile"]
			return Vector2i(int(pair[0]), int(pair[1]))
	return HOVER_NONE


func _first_tile_of_kind(kind: String) -> Vector2i:
	return _first_tile_of(func(p): return String(p["kind"]) == kind)


## The first placement that RUNS A RECIPE — a machine, as opposed to a belt,
## a pipe or one of the buffer buildings.
func _first_machine_tile() -> Vector2i:
	return _first_tile_of(func(p): return p.get("recipe") != null)


func _hover_over_first_machine() -> void:
	var tile := _first_machine_tile()
	if tile != HOVER_NONE:
		_hover_over_tile(tile)


func _hover_over_pipe_inlet() -> void:
	var tile := _first_tile_of_kind("pipe")
	if tile != HOVER_NONE:
		_hover_over_tile(tile)


## The far corner of the first multi-tile machine — NOT its origin, see this
## file's header. Does nothing if nothing placed is bigger than one tile,
## rather than silently hovering something else.
func _hover_over_multi_tile_machine() -> void:
	for placement in JSON.parse_string(_main.sim.world_layout_json()):
		var size: Vector2i = _main._footprint_size(placement.get("footprint"))
		if size == Vector2i.ONE:
			continue
		var pair: Array = placement["tile"]
		_hover_over_tile(Vector2i(int(pair[0]), int(pair[1])) + size - Vector2i.ONE)
		return


func _hover_over_hopper() -> void:
	var tile := _first_tile_of_kind("hopper")
	if tile != HOVER_NONE:
		_hover_over_tile(tile)


func _hover_over_fluid_tank() -> void:
	var tile := _first_tile_of_kind("fluid_tank")
	if tile != HOVER_NONE:
		_hover_over_tile(tile)


func _hover_over_dock() -> void:
	var tile := _first_tile_of_kind("dock")
	if tile != HOVER_NONE:
		_hover_over_tile(tile)


## Loads both consumables and ticks, so the shot shows product moving rather
## than an idle factory. Whatever was built is what runs; a fixture with no
## hopper simply has nothing to meter out.
func _run_chain_forward() -> void:
	for i in range(POPULATE_SPAWNS):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		for j in range(POPULATE_TICKS_PER_SPAWN):
			_main.sim.tick()
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()


## Enters placement mode with the cursor parked somewhere the ghost reads as
## valid (open ground) or invalid (on top of something already placed) — the
## two states `_update_ghost` distinguishes, and the only way to photograph
## either without a live session.
func _enter_ghost_mode() -> void:
	_main.sim.issue_command_json(JSON.stringify({
		"type": "purchase", "kind": _ghost_kind, "quantity": 1,
	}))
	_main._refresh_owned_counts()
	if _main._owned_of(_ghost_kind) <= 0:
		# The fixture spent the starting balance and this kind could not be
		# afforded, so there is nothing to ghost. Said out loud rather than
		# photographed as an empty frame — see this file's header on what a
		# starting balance buys.
		printerr("SCREENSHOT_NOTE: could not afford one '%s' after building the fixture — no ghost to show" % _ghost_kind)
		return
	_main._enter_build_mode(_ghost_kind)
	var target := _open_ground_tile()
	if _ghost_blocked:
		var occupied := _first_machine_tile()
		if occupied == HOVER_NONE:
			occupied = _first_tile_of(func(_p): return true)
		if occupied != HOVER_NONE:
			target = occupied
	_hover_over_tile(target)
	_main._update_ghost()


## A tile next to what has been built but not on it, so the valid ghost is in
## frame beside the factory rather than off in empty floor.
func _open_ground_tile() -> Vector2i:
	var bounds: Rect2i = _main._world_tile_bounds()
	return Vector2i(bounds.position.x + bounds.size.x + 1, bounds.position.y)


## Remove mode, with the cursor over something removable.
func _enter_remove_mode() -> void:
	_main._toggle_remove_mode()
	var tile := _first_machine_tile()
	if tile == HOVER_NONE:
		tile = _first_tile_of(func(_p): return true)
	if tile != HOVER_NONE:
		_hover_over_tile(tile)
	_main._update_ghost()


func _open_shop_menu() -> void:
	_main._set_shop_menu_open(true)


func _process(_delta: float) -> bool:
	_frames += 1
	# Build first: every flag below is about looking at something, and since
	# the "Real Placement" epic there is nothing to look at until one of the
	# fixtures has been placed.
	if _frames == 1 and (_build or _fixture != ""):
		_build_fixture()
	if _frames == 1 and _first_person:
		_enter_first_person()
	if _frames == 2 and _populate:
		_run_chain_forward()
	# The closeup is applied BEFORE any hover flag: a hover warps the OS cursor
	# to where a tile currently projects on screen, so re-aiming the camera
	# afterwards would leave the cursor pointing at whatever moved under it.
	if _frames == 2 and _closeup:
		_enter_closeup()
	if _frames == 2 and _hover:
		_hover_over_first_machine()
	if _frames == 2 and _hover_pipe:
		_hover_over_pipe_inlet()
	if _frames == 2 and _hover_vat:
		_hover_over_multi_tile_machine()
	if _frames == 2 and _hover_hopper:
		_hover_over_hopper()
	if _frames == 2 and _hover_tank:
		_hover_over_fluid_tank()
	if _frames == 2 and _hover_dock:
		_hover_over_dock()
	if _frames == 2 and _ghost_kind != "":
		_enter_ghost_mode()
	if _frames == 2 and _remove_mode:
		_enter_remove_mode()
	if _frames == 2 and _shop:
		_open_shop_menu()
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
