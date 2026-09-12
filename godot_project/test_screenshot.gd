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
# contains nothing at all, so most flags below begin by BUILDING something.
# What they can build is bounded by `sim::STARTING_MONEY`, which the
# inventory-hotbar ticket raised from 200 to 3000 — comfortably more than the
# shipped line's ~1,200 — so `--fixture chain` and `--contracts`
# can build and run the real line again. See test_layouts.gd's header.
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
# the machine that taps it), `vat` (the 2x2 cooking vat, alone), `splitter` (a
# line that forks) or `chain` (THE WHOLE SHIPPED LINE, bought and placed out
# of a new game's own balance — test_layouts.gd's `build_reference_line`).
#
# `--contracts` builds that full line and then runs it until the contract
# board is in a genuinely mixed state — some deliveries made, some missed,
# goods standing on the dock — so the clipboard can be reviewed against a
# working economy rather than an idle one. It implies `--fixture chain`.
#
# `--stress` builds the STRESS LINE (test_layouts.gd's `build_stress_line`:
# a 1500-tile serpentine belt fed by hoppers plus three tank-fed pipe runs)
# and runs it, for reviewing the instanced tile/item rendering the pipes &
# conveyors overhaul's ticket 4 built, at the scale it exists for. It
# implies `--populate`, and the shot's frame prints the rendering server's
# object, primitive and draw-call counts (`SCREENSHOT_RENDER`) so the cost of
# a frame can be compared without a profiler.
#
# `--belt-gallery` builds the BELT GALLERY (test_layouts.gd's
# `build_belt_gallery`: every belt shape at every facing, every belt-to-
# building junction at every facing), the review subject for the pipes and
# conveyors overhaul's belt presentation.
#
# `--bottleneck` builds the BOTTLENECK LINE (test_layouts.gd's
# `build_bottleneck_line`: the reference line on auto-fill, run until it has
# backed up behind the cooking vat), the fixture the machine-load epic's
# hover readout is reviewed on (with `--hover`). Implies `--populate`.
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
# `--stock` buys an assorted starting kit (belts, pipes, a hopper, a tank and
# a machine or two) so the hotbar shows CARRIED sockets, multi-digit counts and
# EMPTY sockets side by side — the only way to review that distinction, since a
# new game carries nothing and every socket is empty.
#
# `--hotbar N` presses hotbar key N (1..9, or 0 for the tenth slot) through
# the scene's real `_unhandled_input`, after buying one of that slot's kind so
# the socket is not empty — so the selected-slot state, the name flyout and
# the ghost that selection puts in the player's hand are all in the shot. It
# is the only way to review the hotbar's selected state without a live
# session, exactly as `--ghost` is for build mode.
#
# `--menu` photographs the MAIN MENU (`menu.tscn`) instead of the game,
# having first written a save file so the load list has something real in it.
# It is the only flag here that renders a different scene, because the main
# menu IS a different scene — it is what `project.godot` opens on now.
#
# `--pause` builds a factory, runs it, and opens the in-game pause menu over
# it. Like `--ghost` and `--hotbar`, it is the only way to review a panel that
# only exists in response to a keypress.
#
# `--closeup TX,TY[,SIZE]` re-aims the ISOMETRIC camera at one simulation
# tile and zooms it right in (SIZE is the orthographic width in world units,
# default CLOSEUP_SIZE). The whole-factory shot is the right frame for "does
# the line read", and the wrong one for "does this belt actually meet this
# machine" — a junction is about four voxels of detail and at 1600x900 the
# full line gives it about six pixels.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const GameSaves = preload("res://game_session.gd")

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
var _hover_splitter := false
var _shop := false
## `--config TX,TY` clicks the hopper or tank standing on that tile once
## the fixture is built, so the frame shows its configuration panel — the
## level bar, FILL with its reason when it is blocked, the AUTO-FILL switch
## and (hoppers) the item picker. Without `--stock` the larder is empty, so
## the panel is photographed in its blocked state, which is the more useful
## of the two to look at.
var _config := false
var _config_tile := Vector2i.ZERO
## `--market` opens the ingredient market, and `--contents N` presses the
## open configuration panel's picker slot N (with `--config`), so a placed
## hopper is photographed after being retargeted to another ingredient.
var _market := false
var _contents := -1
## `--no-hud` hides the whole HUD layer. It exists for `--closeup`: the
## contract clipboard and the hotbar between them cover the top-right and the
## bottom of the frame, which is exactly where a junction lands about half
## the time — so reviewing "does this pipe actually meet this machine", the
## thing closeups are FOR, kept coming down to whether the subject happened
## to miss a panel. The HUD has its own screenshots (--shop, --market,
## --hover-*); a junction shot does not need it in frame.
var _no_hud := false
var _contracts := false
## `--board` opens the contracts board menu (the contract-board epic).
var _board := false
var _hotbar_key := ""
var _stock := false
## `--contracts` waits for a STATE, not a tick count: a fixed count drifts
## with any change to throughput or to the shipped cadences and quietly goes
## back to photographing three identical checklists. What it waits for is at
## least one unit standing on the dock plus at least one delivery AND one miss
## on the board — which against the shipped contracts (asks of 2, 3 and 1 out
## of one shared pool) reads as one docket satisfied and the rest part-filled:
## the checkbox mechanic and the shared-pool competition in a single frame.
## A MINIMUM dock count rather than an exact one, because the contracts check
## every 1000/1500/2500 ticks and drain the pool about as fast as the line can
## fill it.
const CONTRACTS_REVIEW_DOCK_UNITS := 1
const CONTRACTS_REVIEW_TICK_LIMIT := 20000
## `--payout` photographs the celebration a contract payout fires: the coin
## burst over the dock, the "+N" over the money line and the PAID stamp on the
## docket that paid. This is the one review the payout work actually needs —
## `test_payout_feedback.gd` can prove every number, and cannot say whether ten
## coins over a dock read as a fountain or as a mess (UI_UX.md §2b point 12).
##
## Like `--contracts` it waits for the EVENT, not for a tick count: the first
## payout lands at tick 2500 against today's cadences and process times, and a
## hardcoded count would quietly go back to photographing an empty apron the
## day one of those moves.
var _payout := false
const PAYOUT_REVIEW_TICK_LIMIT := 20000
## Where in the burst's arc to stop the clock, as a fraction of
## COIN_FLIGHT_TICKS. Past `COIN_GROW` so the coins are at full size, and near
## the top of the flight, which is where the fan reads as a fountain rather
## than as a pile on the deck.
const PAYOUT_REVIEW_PHASE := 0.5
var _ghost_kind := ""
var _ghost_blocked := false
var _remove_mode := false
## `--menu` renders menu.tscn rather than main.tscn; `--pause` renders the
## in-game pause menu over a running factory. See the header.
var _menu := false
var _pause := false
## The save this run writes so `--menu`'s list is not empty, and removes
## again on the way out. Named so it cannot collide with a real save.
const MENU_REVIEW_SLOT := "zz review shot"
var _menu_review_path := ""

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
	if args.has("--stress"):
		_fixture = "stress"
		_populate = true
	if args.has("--pipe-gallery"):
		_fixture = "pipes"
	if args.has("--belt-gallery"):
		_fixture = "gallery"
	if args.has("--bottleneck"):
		_fixture = "bottleneck"
		_populate = true
	_first_person = args.has("--first-person")
	_hover = args.has("--hover")
	_hover_pipe = args.has("--hover-pipe")
	_hover_vat = args.has("--hover-vat")
	_hover_hopper = args.has("--hover-hopper")
	_hover_tank = args.has("--hover-tank")
	_hover_dock = args.has("--hover-dock")
	_hover_splitter = args.has("--hover-splitter")
	_shop = args.has("--shop")
	var config_arg := _string_arg(args, "--config")
	if config_arg != "":
		var parts: PackedStringArray = config_arg.split(",")
		if parts.size() == 2:
			_config = true
			_config_tile = Vector2i(int(parts[0]), int(parts[1]))
	_market = args.has("--market")
	_board = args.has("--board")
	_no_hud = args.has("--no-hud")
	_contents = int(_string_arg(args, "--contents")) if _string_arg(args, "--contents") != "" else -1
	_contracts = args.has("--contracts")
	if _contracts:
		_fixture = "chain"
	_payout = args.has("--payout")
	if _payout:
		_fixture = "chain"
	_hotbar_key = _string_arg(args, "--hotbar")
	_stock = args.has("--stock")
	_ghost_kind = _string_arg(args, "--ghost")
	if _ghost_kind == "":
		_ghost_kind = _string_arg(args, "--ghost-blocked")
		_ghost_blocked = _ghost_kind != ""
	_remove_mode = args.has("--remove-mode")
	_menu = args.has("--menu")
	_pause = args.has("--pause")
	_parse_closeup(args)
	if _menu:
		# The load list is the half of this screen worth reviewing, so give it
		# something to list. Written before the scene is built, because
		# `menu.gd` reads the directory in `_ready`.
		_write_menu_review_save()
		_main = load("res://menu.tscn").instantiate()
		root.add_child(_main)
		return
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	# NOT here: in a real (non-headless) run, add_child()'s _ready() is
	# deferred to the engine's own next-frame processing, unlike the pure
	# SceneTree tests that call _ready() directly — _main._camera doesn't
	# exist yet at this point. Anything that touches it has to wait for
	# _process()'s first frame instead. (Learned the hard way: this used to
	# live here and crashed with "base object of type Nil".)


## `--flag value` (or `--flag=value`, for a value that starts with `-` and
## would otherwise be read as another flag — a negative tile coordinate) as
## one look-up, since every option below that takes an argument takes
## exactly one.
func _string_arg(args: Array, flag: String) -> String:
	for i in range(args.size()):
		var arg := String(args[i])
		if arg == flag and i + 1 < args.size():
			return String(args[i + 1])
		if arg.begins_with(flag + "="):
			return arg.substr(flag.length() + 1)
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
		"splitter":
			Layouts.build_splitter_line(_main.sim)
		"chain":
			Layouts.build_reference_line(_main.sim)
		"stress":
			Layouts.build_stress_line(_main.sim)
		"pipes":
			Layouts.build_pipe_gallery(_main.sim)
		"gallery":
			Layouts.build_belt_gallery(_main.sim)
		"bottleneck":
			Layouts.build_bottleneck_line(_main.sim)
		_:
			Layouts.build_demo_line(_main.sim)
	# Every fixture is stocked, whether or not this run will tick: a
	# screenshot of a factory with an empty larder is a screenshot of a
	# factory that cannot start, and the larder strip would say so.
	Layouts.stock_larder(_main.sim)
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


func _hover_over_pipe() -> void:
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


func _hover_over_splitter() -> void:
	var tile := _first_tile_of_kind("splitter")
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
			# Applied EVERY tick, not once at the end, because `World::tick`
			# clears its dirty set at the START of each tick — so a batched
			# run reports only whatever the very last tick happened to touch
			# and the shot is drawn from that. It photographed a branch belt
			# as empty while the simulation held eight tomatoes on it, which
			# is exactly the kind of lie this whole script exists to catch.
			_main._apply_dirty_chunks()
			# The two animations the tick drives. `main.gd` advances both at
			# the end of `_advance_simulation`, which this loop deliberately
			# does not go through (it wants ticks without a frame's worth of
			# input, camera and UI work), so without these two lines every
			# screenshot ever taken shows belt frame 0 and every machine's
			# mechanism at rest — an idle-looking factory photographed in the
			# middle of a run, which is the same lie the comment above is
			# about. Both read the tick, so the shot stays reproducible.
			_main._update_belt_frame()
			_main._update_machine_motion()
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()
	# The load records are polled from `_advance_simulation` too, which this
	# loop bypasses; read them once here so a hover in the shot shows the
	# settled verdicts, not the ones the rebuild saw before the run.
	_main._poll_machine_load()


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


## `--config TX,TY`: the click that opens a hopper's or tank's panel, driven
## through the same tile-resolved entry point a real click reaches
## (`_config_click_at_tile`), so the shot proves the panel a click actually
## produces — positioned beside the building, with whichever FILL reason the
## current larder gives — rather than a widget merely made visible.
func _open_config_at(tile: Vector2i) -> void:
	_main._config_click_at_tile(tile)
	if not _main._config_open:
		printerr("SCREENSHOT: no hopper or tank at %s to configure" % str(tile))
		return
	if _contents >= 0:
		_main._on_config_picker_pressed(_contents)


## Drives the real chain across several contract cadence boundaries, feeding
## it exactly the way a player does. What puts product in the shared delivery
## pool the contracts draw from is the delivery dock at the end of the line,
## not a command issued from here.
func _run_contract_cycles() -> void:
	# A new game runs the starter only (the contract-board epic); the review
	# frame wants the whole board competing, so sign every offer first — the
	# full line costs 1087 of the 3000 start, which covers the signatures.
	for offer in JSON.parse_string(_main.sim.economy_snapshot_json()).get("board", []):
		if not bool(offer.get("active", false)):
			_main.sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": String(offer["id"])}))
	for i in range(CONTRACTS_REVIEW_TICK_LIMIT):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		_main.sim.tick()
		if _contract_board_is_mixed():
			break
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()


## Runs the shipped line until a contract genuinely pays, then stops the clock
## partway through the burst it fires.
##
## The line is put on AUTOMATIC rather than hand-fed a command a tick: the two
## raw solids and the raw fluid all have to keep arriving for 2500 ticks, and
## `stock_larder` buys only the solids, so the vinegar is bought explicitly
## here. A line that stalls halfway photographs as a factory with nothing on
## its dock and no payout at all.
func _run_to_payout() -> void:
	Layouts.fund(_main.sim, 30000)
	Layouts.stock(_main.sim, "vinegar", 1000)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		_main.sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true}))
	_main.sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true}))

	for i in range(PAYOUT_REVIEW_TICK_LIMIT):
		_main.sim.tick()
		_main._update_contract_panel()
		if _main.payout_events > 0:
			break
	if _main.payout_events == 0:
		printerr("SCREENSHOT_NOTE: --payout ran %d ticks without a contract paying"
			% PAYOUT_REVIEW_TICK_LIMIT)

	# Advance to the chosen point in the arc. The whole effect is a function of
	# `sim.current_tick()`, so "hold the frame mid-burst" is ticking the
	# simulation, not sleeping — which is also why two runs of this flag give
	# the identical picture.
	for i in range(int(_main.COIN_FLIGHT_TICKS * PAYOUT_REVIEW_PHASE)):
		_main.sim.tick()
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()
	_main._update_payout_feedback()
	# Then FREEZE the scene. Every other flag sets up a state that persists
	# (a board, an open menu, a hovered tile), so the WARMUP_FRAMES the shot
	# waits out cost them nothing; a burst lasts COIN_FLIGHT_TICKS (34) and
	# `main._process` advances up to MAX_TICKS_PER_FRAME (8) a frame, so 43
	# more frames of it would leave an empty apron to photograph. Holding the
	# scene still is what "hold the frame mid-burst" means here, and the
	# review wants a still anyway.
	_main.set_process(false)


func _contract_board_is_mixed() -> bool:
	var econ: Dictionary = JSON.parse_string(_main.sim.economy_snapshot_json())
	var stock := 0
	for pair in econ.get("delivery_stock", []):
		stock += int(pair[1])
	if stock < CONTRACTS_REVIEW_DOCK_UNITS:
		return false
	var made := 0
	var missed := 0
	for contract in econ.get("contracts", []):
		made += int(contract["deliveries_made"])
		missed += int(contract["deliveries_missed"])
	return made >= 1 and missed >= 1


## Presses a hotbar key for real, having first bought one of that slot's kind
## so the socket has something in it — an empty socket refuses selection, by
## design, and would photograph as nothing happening.
func _select_hotbar_key() -> void:
	var index: int = _main.HOTBAR_KEYCODES.find(_hotbar_key_code())
	if index < 0 or index >= _main._hotbar_slots.size():
		printerr("SCREENSHOT_NOTE: --hotbar '%s' is not a hotbar key" % _hotbar_key)
		return
	var kind: String = String(_main._hotbar_slots[index]["kind"])
	_main.sim.issue_command_json(JSON.stringify({
		"type": "purchase", "kind": kind, "quantity": 1,
	}))
	_main._refresh_owned_counts()
	var event := InputEventKey.new()
	event.keycode = _hotbar_key_code()
	event.pressed = true
	_main._unhandled_input(event)
	# Park the cursor on open ground so the ghost selection put in the
	# player's hand reads as valid rather than blocked.
	_hover_over_tile(_open_ground_tile())
	_main._update_ghost()
	_main._update_hotbar()


## An assorted kit, so a review frame shows every socket state at once. The
## quantities are picked to include a two-digit count (belts), single-digit
## counts, and several kinds left at zero.
func _stock_the_hotbar() -> void:
	for pair in [["belt", 24], ["pipe", 6], ["hopper", 1], ["fluid_tank", 1], ["crusher", 2]]:
		_main.sim.issue_command_json(JSON.stringify({
			"type": "purchase", "kind": pair[0], "quantity": pair[1],
		}))
	_main._refresh_owned_counts()
	_main._update_hotbar()


func _hotbar_key_code() -> int:
	if _hotbar_key == "0":
		return KEY_0
	if _hotbar_key.length() == 1 and _hotbar_key >= "1" and _hotbar_key <= "9":
		return KEY_1 + (_hotbar_key.to_int() - 1)
	return 0


## Writes one real save so `--menu` photographs a list with a row in it
## rather than the empty state. Removed again once the frame is written.
func _write_menu_review_save() -> void:
	var sim = SimBridge.new()
	Layouts.build_machines_fixture(sim)
	Layouts.stock_larder(sim)
	for i in range(200):
		sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
		sim.tick()
	if GameSaves.write_slot(MENU_REVIEW_SLOT, String(sim.save_ron())) == "":
		_menu_review_path = GameSaves.slot_path(MENU_REVIEW_SLOT)


## `--menu` shows the LOAD view, not the three buttons: the buttons are the
## easy half and the list is the half with layout worth checking.
func _open_menu_load_view() -> void:
	_main._on_load_pressed()


## `--pause`: the in-game menu, over a factory that has been running.
func _open_pause_menu() -> void:
	_main._set_pause_open(true)


func _process(_delta: float) -> bool:
	_frames += 1
	# Build first: every flag below is about looking at something, and since
	# the "Real Placement" epic there is nothing to look at until one of the
	# fixtures has been placed.
	if _menu:
		if _frames == 1:
			_open_menu_load_view()
		if _frames < WARMUP_FRAMES:
			return false
		return _write_frame()
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
		_hover_over_pipe()
	if _frames == 2 and _hover_vat:
		_hover_over_multi_tile_machine()
	if _frames == 2 and _hover_hopper:
		_hover_over_hopper()
	if _frames == 2 and _hover_tank:
		_hover_over_fluid_tank()
	if _frames == 2 and _hover_dock:
		_hover_over_dock()
	if _frames == 2 and _hover_splitter:
		_hover_over_splitter()
	if _frames == 2 and _ghost_kind != "":
		_enter_ghost_mode()
	if _frames == 2 and _remove_mode:
		_enter_remove_mode()
	if _frames == 2 and _shop:
		_open_shop_menu()
	if _frames == 2 and _no_hud:
		var hud := _main.get_node_or_null("HUD")
		if hud != null:
			hud.visible = false
	if _frames == 2 and _market:
		_main._set_market_open(true)
	if _frames == 2 and _board:
		_main._set_contract_board_open(true)
	if _frames == 2 and _contracts:
		_run_contract_cycles()
	if _frames == 2 and _payout:
		_run_to_payout()
	if _frames == 2 and _stock:
		_stock_the_hotbar()
	if _frames == 2 and _hotbar_key != "":
		_select_hotbar_key()
	# After --stock, so a stocked larder photographs FILL enabled and an
	# empty one photographs it blocked with its reason.
	if _frames == 2 and _config:
		_open_config_at(_config_tile)
	# Last, so it draws over whatever else the flags set up — which is
	# exactly what a review of it wants to see.
	if _frames == 3 and _pause:
		_open_pause_menu()
	if _frames < WARMUP_FRAMES:
		return false
	return _write_frame()


func _write_frame() -> bool:
	var image := root.get_texture().get_image()
	if _menu_review_path != "" and FileAccess.file_exists(_menu_review_path):
		DirAccess.remove_absolute(_menu_review_path)
	if image == null:
		printerr("SCREENSHOT_FAIL: no viewport image (is this running with --headless?)")
		return true
	var err := image.save_png(_out)
	if err != OK:
		printerr("SCREENSHOT_FAIL: could not write %s (error %d)" % [_out, err])
		return true

	# What the frame cost the renderer, in the renderer's own terms. Objects
	# is what instancing collapses (one MultiMesh is one object however many
	# tiles it draws); draw calls follow it; primitives do not move, since
	# the same voxels are drawn either way.
	print("SCREENSHOT_RENDER: objects %d, primitives %d, draw calls %d, in frame %d" % [
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_OBJECTS_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		_frames,
	])
	print("SCREENSHOT_OK: %s (%dx%d)" % [_out, image.get_width(), image.get_height()])
	return true
