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
#
# `--hover` warps the OS cursor over the first stage's machine before
# rendering, so the hover tooltip (docs/UI_UX.md §5) actually shows up in the
# shot — otherwise there is no way to visually review it without a live
# interactive session, the same problem --first-person solves for that mode.
# `--hover-pipe` does the same over the fluid network's machine-inlet pipe
# segment, for reviewing milestone 6's fluid tooltip. `--hover-vat` parks it
# over the FAR CORNER of the first machine whose footprint is bigger than one
# tile — deliberately not its origin, since "hovering anywhere on a multi-tile
# machine identifies it" is the claim the "Machines improvement" milestone
# added, and hovering the origin would not show it. `--hover-hopper` does the
# same over the chain's hopper, and `--hover-tank` over the pipe run's fluid
# tank (the "Visual overhaul round 2" epic's building).
#
# All five hover flags share one implementation (`_hover_over_tile`) — they
# differ only in which tile they ask the simulation for.
#
# `--contracts` runs the chain forward far enough to cross several contract
# cadence boundaries WITH deliveries collected, so the contract clipboard is
# photographed with its dockets in genuinely different states — one satisfied,
# one part-filled, one that has been missed — rather than the three identical
# 0/N checklists an idle or merely `--populate`d factory gives (`--populate`
# never issues `collect_output`, so nothing ever reaches the delivery pool).
# CONTRACTS_REVIEW_TICKS is tuned against the shipped `data/contracts.ron`
# cadences for exactly that spread; see its own comment.
#
# `--shop` opens the build menu (docs/UI_UX.md §5, the "Shop & Inventory"
# epic's ticket 2) before rendering, and first runs the chain far enough to
# earn a contract payout — a menu photographed with zero money shows every
# slot locked, which is exactly half of what needs reviewing. Implies the
# money-earning run whether or not `--populate` was also passed.
#
# `--closeup TX,TY[,SIZE]` re-aims the ISOMETRIC camera at one simulation
# tile and zooms it right in (SIZE is the orthographic width in world units,
# default CLOSEUP_SIZE). The whole-factory shot above is the right frame for
# "does the line read", and the wrong one for "does this belt actually meet
# this machine" — a junction is about four voxels of detail and at 1600x900
# the full chain gives it about six pixels. Every belt/pipe-to-machine
# connection in the "Visual overhaul round 2" epic's ticket 3 was diagnosed
# and re-checked through this flag, one junction at a time.
extends SceneTree

## Enough frames for shadow atlases, SSAO and glow to be resolved — a
## screenshot on frame 1 shows an unlit, shadowless scene and is misleading.
const WARMUP_FRAMES := 45

## Spawn attempts, and simulation ticks between them, used by `--populate`.
## Driven straight through the scene's real input and tick path rather than
## poking at simulation state — this is a screenshot of the actual game.
const POPULATE_SPAWNS := 60
const POPULATE_TICKS_PER_SPAWN := 10

## `--contracts` runs until the board is in a genuinely MIXED state rather
## than for a fixed number of ticks: a tick count drifts with any change to
## the chain's throughput or the shipped cadences and quietly goes back to
## photographing three identical checklists. The state waited for is a dock
## holding exactly CONTRACTS_REVIEW_DOCK_UNITS, with at least one delivery
## and at least one miss on the board — against the shipped contracts (asks
## of 2, 3 and 1 from one shared pool) that reads as one docket satisfied,
## one part-filled and one satisfied-but-already-missed, which is both the
## checkbox mechanic and the shared-pool competition in a single frame.
const CONTRACTS_REVIEW_DOCK_UNITS := 2
const CONTRACTS_REVIEW_TICK_LIMIT := 20000

var _frames := 0
var _out := "user://scene.png"
var _populate := false
var _first_person := false
var _hover := false
var _hover_pipe := false
var _hover_vat := false
var _hover_hopper := false
var _hover_tank := false
var _shop := false
var _contracts := false
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
	_populate = args.has("--populate")
	_first_person = args.has("--first-person")
	_hover = args.has("--hover")
	_hover_pipe = args.has("--hover-pipe")
	_hover_vat = args.has("--hover-vat")
	_hover_hopper = args.has("--hover-hopper")
	_hover_tank = args.has("--hover-tank")
	_shop = args.has("--shop")
	_contracts = args.has("--contracts")
	_parse_closeup(args)
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	# NOT here: in a real (non-headless) run, add_child()'s _ready() is
	# deferred to the engine's own next-frame processing, unlike the pure
	# SceneTree tests that call _ready() directly — _main._camera doesn't
	# exist yet at this point. Anything that touches it has to wait for
	# _process()'s first frame instead. (Learned the hard way: this used to
	# live here and crashed with "base object of type Nil".)


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


func _hover_over_first_machine() -> void:
	_hover_over_tile(_main._machine_tiles[_main._stage_order[0]])


## Aimed at the pipe segment the machine actually draws from — the one whose
## level visibly moves.
func _hover_over_pipe_inlet() -> void:
	var json: String = _main.sim.fluid_snapshot_json()
	if json.is_empty():
		return # no pipe run in this world; nothing to hover
	var snapshot: Dictionary = JSON.parse_string(json)
	var segment: Dictionary = snapshot["segments"][int(snapshot["inlet_index"])]
	var tile_pair: Array = segment["tile"]
	_hover_over_tile(Vector2i(int(tile_pair[0]), int(tile_pair[1])))


## The far corner of the first multi-tile machine — NOT its origin, see this
## file's header. Does nothing if the loaded content has no multi-tile kind,
## rather than silently hovering something else.
func _hover_over_multi_tile_machine() -> void:
	for stage in JSON.parse_string(_main.sim.chain_layout_json()):
		var size: Vector2i = _main._footprint_size(stage.get("machine_footprint"))
		if size == Vector2i.ONE:
			continue
		var pair: Array = stage["machine_tile"]
		_hover_over_tile(Vector2i(int(pair[0]), int(pair[1])) + size - Vector2i.ONE)
		return


func _hover_over_hopper() -> void:
	var json: String = _main.sim.hopper_snapshot_json()
	if json.is_empty():
		return
	var tile_pair: Array = JSON.parse_string(json)["tile"]
	_hover_over_tile(Vector2i(int(tile_pair[0]), int(tile_pair[1])))


func _hover_over_fluid_tank() -> void:
	var json: String = _main.sim.fluid_tank_snapshot_json()
	if json.is_empty():
		return # no pipe run in this world, so no tank
	var tile_pair: Array = JSON.parse_string(json)["tile"]
	_hover_over_tile(Vector2i(int(tile_pair[0]), int(tile_pair[1])))


func _run_chain_forward() -> void:
	# Calls SimBridge directly rather than simulating a SPACE keypress
	# through _unhandled_input: SPACE only spawns in ISOMETRIC mode (it's
	# "fly up" in first-person), so going through the view-mode-sensitive
	# input path would silently do nothing when --first-person is also set.
	#
	# BOTH consumables get loaded, not just the hopper. Since the "Visual
	# overhaul round 2" epic's ticket 1 the pipe run has no self-refilling
	# source, so a populate run that only fed tomatoes stalled the whole chain
	# at the mixer and photographed empty belts downstream of it — which is
	# not "the factory running", it is a picture of a starved factory.
	for i in range(POPULATE_SPAWNS):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		for j in range(POPULATE_TICKS_PER_SPAWN):
			_main.sim.tick()
	_main._apply_dirty_chunks()


## Ticks the real chain until the hardcoded contract pays out at least once,
## so `--shop` photographs a menu with some slots affordable and some not.
## `collect_output` every tick is a documented no-op on an empty machine, so
## this needs no chunk inspection — same drive loop test_shop_menu.gd uses.
const SHOP_EARN_TICK_LIMIT := 20000


func _earn_contract_money() -> void:
	for i in range(SHOP_EARN_TICK_LIMIT):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		_main.sim.tick()
		_main.sim.issue_command_json(JSON.stringify({"type": "collect_output"}))
		if float(JSON.parse_string(_main.sim.shop_snapshot_json())["money"]) > 0.0:
			break
	_main._apply_dirty_chunks()


## Drives the real chain, with deliveries collected, across several contract
## cadence boundaries. Same command path as _run_chain_forward — the only
## addition is `collect_output`, which is what actually puts product in the
## shared delivery pool the contracts draw from.
func _run_contract_cycles() -> void:
	for i in range(CONTRACTS_REVIEW_TICK_LIMIT):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		_main.sim.tick()
		_main.sim.issue_command_json(JSON.stringify({"type": "collect_output"}))
		if _contract_board_is_mixed():
			break
	_main._apply_dirty_chunks()
	_main._update_contract_panel()


func _contract_board_is_mixed() -> bool:
	var econ: Dictionary = JSON.parse_string(_main.sim.economy_snapshot_json())
	var stock := 0
	for pair in econ.get("delivery_stock", []):
		stock += int(pair[1])
	if stock != CONTRACTS_REVIEW_DOCK_UNITS:
		return false
	var made := 0
	var missed := 0
	for contract in econ.get("contracts", []):
		made += int(contract["deliveries_made"])
		missed += int(contract["deliveries_missed"])
	return made >= 1 and missed >= 1


func _open_shop_menu() -> void:
	_earn_contract_money()
	_main._set_shop_menu_open(true)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames == 1 and _first_person:
		_enter_first_person()
	if _frames == 2 and _populate:
		_run_chain_forward()
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
	if _frames == 2 and _contracts:
		_run_contract_cycles()
	if _frames == 2 and _shop:
		_open_shop_menu()
	if _frames == 2 and _closeup:
		_enter_closeup()
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
