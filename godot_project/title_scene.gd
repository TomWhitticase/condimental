## The title screen's 3D backdrop: a real factory, built out of real voxel
## models, standing in the same hall the game is played in.
##
## This is what Minecraft's panorama and Factorio's title art are — the game
## showing you what it is before you press anything — and the owner asked for
## one in those words. `menu.gd`'s first cut deliberately used a flat
## charcoal plate instead, on the argument that `main.tscn` costs a full
## voxel load and a lit 3D frame to stand behind a menu the player leaves in
## two seconds. That argument is answered rather than ignored: NONE of
## `main.tscn` is loaded here and no simulation is created. This scene is a
## hand-composed still life of about twenty models on a floor — the shipped
## chain, laid out left to right in the order it actually runs — which costs
## a handful of `.vox` loads, not the game.
##
## **It is art, not a simulation.** Nothing here ticks, nothing is placed by
## a command, and the items sitting on the belts were put there by hand. The
## sim's rules do not apply to a poster. What the models, the materials, the
## floor variants and the hall all come from is the real pipeline, so the
## poster cannot go stale: re-render the voxels or re-tune the lighting and
## this screen changes with the game.
##
## The camera drifts, slowly, the way Minecraft's panorama does — a title
## screen that is perfectly still reads as a paused game rather than as a
## place. See `_process`.
##
## **And the factory RUNS.** The belts march, every machine works through
## its motion frames, product slides along the line and steam comes off the
## vat — all of it the game's own models driven at the game's own pace
## (`BELT_TICKS_PER_TILE`, `MACHINE_TICKS_PER_CYCLE`), but on wall time: a
## poster has no simulation to take a tick from. Over it hangs the game's
## name as a lit marquee sign (`title_sign` in assets.rs), whose bulbs chase
## round its frame.
##
## **The screen opens on an intro** (`INTRO_LENGTH`): the hall's lights
## strike, the sign drops in on its rods and its letters flicker on, then
## the menu fades up — `menu.gd` reads `intro_time` for that half. Any click
## or key skips it (`finish_intro`), and the screenshot harness skips it on
## its first frame so a review shot is the settled screen.
extends Node3D

const MainScript = preload("res://main.gd")
const VoxelAssets = preload("res://voxel_assets.gd")
const FactoryEnvironment = preload("res://factory_environment.gd")

## Scale and heights, read from `main.gd` rather than restated, so the
## backdrop is laid out in the same units as the world it depicts.
var TILE: float = MainScript.TILE
var FLOOR_Y: float = MainScript.FLOOR_Y
var BELT_SURFACE_Y: float = MainScript.BELT_SURFACE_Y

## The floor under the poster, in tiles. Big enough that its far edge lands
## in the hall's haze rather than ending in mid-air — the sky shader's own
## slab takes over out there, and the two were matched to each other at the
## seam (see `factory_interior_sky.gdshader`).
const FLOOR_FROM := Vector2i(-26, -22)
const FLOOR_TO := Vector2i(34, 20)

## Eye height. `factory_interior_sky.gdshader`'s `floor_drop` puts the hall's
## slab 5 units below the VIEWER (a sky has no parallax), and first person
## stands at `TILE * 1.5` = 4.8, so a camera near that height is the one that
## makes the painted slab and the real floor tiles line up. It is also the
## height a person would see this from, which is the point of the shot.
const CAMERA_HEIGHT := 4.9

## The chain, west to east, exactly the order `data/recipes.ron` runs it in:
## hopper, crusher, mixer, the 2x2 cooking vat, bottler, packer, dock, with
## belts between the dry stages and a pipe run between the wet ones. Each
## entry is `[asset, tile_x, tile_z, quarter turns]`; a model is authored
## facing east, so 0 turns is a line running east, which is what this is.
const LINE_Z := 0
const CHAIN := [
	["hopper", -3, 0, 0],
	["belt_f0", -2, 0, 0],
	["belt_f0", -1, 0, 0],
	["machine_crusher", 0, 0, 0],
	["belt_f0", 1, 0, 0],
	["belt_f0", 2, 0, 0],
	["machine_mixer", 3, 0, 0],
	["pipe", 4, 0, 0],
	["pipe", 5, 0, 0],
	# The vat covers (6,0)-(7,1); the encoder centres a model on its own
	# footprint, so it goes at the centre of that rectangle.
	["machine_cooking_vat", 6, 0, 0],
	["pipe", 8, 0, 0],
	["pipe", 9, 0, 0],
	["machine_bottler", 10, 0, 0],
	["belt_f0", 11, 0, 0],
	["belt_f0", 12, 0, 0],
	["machine_packer", 13, 0, 0],
	["belt_f0", 14, 0, 0],
	["machine_dock", 15, 0, 0],
]
## The vat is the one multi-tile machine on the line.
const VAT_TILE := Vector2i(6, 0)
const VAT_SIZE := Vector2i(2, 2)

## A second line standing behind the first, out of focus in the depth of the
## hall: no belts, just plant, so the eye reads "this factory is bigger than
## what is in front of me" without a second chain competing for attention.
const BACKGROUND_PLANT := [
	["machine_mixer", 2, 11, 1],
	["machine_crusher", 6, 12, 1],
	["fluid_tank", 10, 11, 0],
	["machine_packer", 14, 12, 1],
	["machine_bottler", 18, 11, 1],
	["machine_dock", -4, -8, 2],
	["machine_bottler", 1, -9, 2],
	["fluid_tank", 6, -8, 0],
	["machine_crusher", 11, -9, 2],
	["machine_mixer", 16, -8, 2],
	["machine_packer", 21, 2, 0],
	["fluid_tank", 22, 6, 0],
]

## Product riding the line, as `[asset, from tile x, to tile x]`: each run is
## the belt between two machines, and an item is spawned out of one body and
## swallowed by the next, so the line reads as running rather than as a
## showroom. One item per tile of run, spaced a tile apart.
const FLOWS := [
	["item_tomato", -2.6, -0.4],
	["item_paste_pail", 0.6, 2.4],
	["item_bottle", 10.6, 12.4],
	["item_box", 13.6, 14.4],
]

## The game's own paces, read rather than restated: a belt's pattern crosses
## a tile in `BELT_TICKS_PER_TILE` ticks and so does an item on it, and every
## machine runs its motion cycle in `MACHINE_TICKS_PER_CYCLE`.
var BELT_TILES_PER_SECOND: float = MainScript.TICK_HZ / float(MainScript.BELT_TICKS_PER_TILE)
var MOTION_FRAMES_PER_SECOND: float = (
	MainScript.TICK_HZ * float(MainScript.MACHINE_FRAMES) / float(MainScript.MACHINE_TICKS_PER_CYCLE)
)

## The marquee sign. Mirrors `assets.rs`'s `TITLE_SIGN_BULB_FRAMES` and
## `TITLE_SIGN_ROD` — change one, change both.
const SIGN_ASSET := "title_sign"
const SIGN_BULB_FRAMES := 3
const SIGN_ROD_VOXELS := 60
## How far in front of the camera the sign hangs. Well short of the chain
## (about 27 units away), so no machine can ever poke through it.
const SIGN_DISTANCE := 13.0
## Bulb steps a second: a brisk theatre chase, not a strobe.
const SIGN_CHASE_PER_SECOND := 7.0
## The sign's idle swing on its rods — a slow sway, barely a degree.
const SIGN_SWAY_DEGREES := 0.8
const SIGN_SWAY_PERIOD := 5.5

## Steam off the cooking vat: puffs in flight at once, the seconds one takes
## to rise, and how far it climbs.
const STEAM_PUFFS := 7
const STEAM_LIFE := 3.2
const STEAM_RISE := 3.6

## The intro, in seconds from the screen coming up. Exposure is the hall's
## lights: dark, a stutter as the tubes strike, then full. The sign drops in
## while they settle, its letters catch, and the menu follows (`menu.gd`).
const INTRO_LENGTH := 2.6
const INTRO_LIGHTS_FROM := 0.05
const INTRO_LIGHTS_TO := 0.9
const INTRO_SIGN_DROP_FROM := 0.55
const INTRO_SIGN_DROP_TO := 1.45
const INTRO_NEON_AT := 1.25
const INTRO_BULBS_AT := 1.6

var _assets := VoxelAssets.new()
var _camera: Camera3D
var _world_env: WorldEnvironment
## Seconds into the intro. Public so `menu.gd` can fade its column up on the
## same clock, and so a skip is one assignment (`finish_intro`).
var intro_time := 0.0
## The exposure the hall is lit at once the intro is over — whatever
## `factory_environment.gd` built, so the intro never retunes the hall.
var _full_exposure := 1.0
## Animated parts: `{"frames": Array[Node3D], "shown": int}` per belt tile
## and per machine mechanism; item nodes per flow; the steam puffs.
var _belt_sets: Array = []
var _motion_sets: Array = []
var _flow_items: Array = [] # Array[{"node", "from", "to", "slot", "count"}]
var _steam: Array = []
var _steam_origin := Vector3.ZERO
## The sign: a rig in camera space (framed by `frame_sign`), the pivot it
## swings from, its bulb frames and its own copy of the lit material.
var _sign_rig: Node3D
var _sign_pivot: Node3D
var _sign_bulbs: Array = []
var _sign_neon: Material
var _sign_neon_energy := 0.0
## The sign's board in world units at scale 1, and the height of the rods'
## top above its foot — the point it hangs (and swings) from.
var _sign_board_size := Vector2.ZERO
var _sign_rod_top := 0.0
## Seconds since the screen came up, which is all the camera's drift is a
## function of. Wall time, not a tick count: there is no simulation on this
## screen to borrow a clock from, and nothing here is reproduced from a save.
var _elapsed := 0.0

## The drift itself — a slow truck along the line with a matching turn, so
## the factory parallaxes against the hall behind it. Kept well under a
## degree a second: it should be noticed only if you sit and look at it.
const DRIFT_PERIOD := 74.0
const DRIFT_TRAVEL := 5.5 # world units, either side of centre
const DRIFT_YAW := 3.0 # degrees, either side of centre

## Depth of field: sharp to just past the line (about 27 units out), then
## soft. Light — the far plant should read as plant, just not compete.
const DOF_FAR_FROM := 34.0
const DOF_FAR_TRANSITION := 30.0
const DOF_AMOUNT := 0.06


func _ready() -> void:
	name = "TitleScene"
	_assets.load_materials()
	# Clear air, unlike the game: see `TITLE_HAZE_DENSITY`. A poster wants the
	# hall's roof and far wall crisp behind the chain, not washed grey.
	# HALL SIZE applies here too, so a player dragging that row on the menu
	# watches the room behind the menu change rather than having to start a
	# game to find out what they just set.
	_world_env = FactoryEnvironment.build_into(
		self, FactoryEnvironment.TITLE_HAZE_DENSITY, _hall_scale()
	)
	var settings := _settings_node()
	if settings != null and not settings.changed.is_connected(_apply_hall_scale):
		settings.changed.connect(_apply_hall_scale)
	_full_exposure = _world_env.environment.tonemap_exposure
	_build_floor()
	_build_models()
	_build_camera()
	_build_sign()
	_animate(0.0)


## The camera's slow drift, and everything else on the poster that moves.
## `_process` on the backdrop only — the menu's own Control tree is static,
## and a title screen that redraws for a moving camera is the one place in
## this project where that is the point.
func _process(delta: float) -> void:
	_animate(delta)


func _animate(delta: float) -> void:
	_elapsed += delta
	intro_time = minf(intro_time + delta, INTRO_LENGTH)
	var phase := sin(TAU * _elapsed / DRIFT_PERIOD)
	_camera.position.x = _camera_home().x + phase * DRIFT_TRAVEL
	_camera.rotation_degrees.y = _camera_yaw() + phase * DRIFT_YAW
	_animate_line()
	_animate_steam()
	_animate_sign()
	_animate_lights()


## Skips the intro to its settled end — a click or key on the menu, and the
## screenshot harness's first frame.
func finish_intro() -> void:
	intro_time = INTRO_LENGTH
	_animate(0.0)


## 0 → 1 across `[from, to]` of the intro, eased out.
func _intro_ease(from: float, to: float) -> float:
	var t := clampf((intro_time - from) / (to - from), 0.0, 1.0)
	return 1.0 - pow(1.0 - t, 3.0)


func _animate_line() -> void:
	var belt_frame := int(floor(_elapsed * BELT_TILES_PER_SECOND * MainScript.BELT_FRAMES))
	for entry in _belt_sets:
		_show_frame(entry, posmod(belt_frame, entry["frames"].size()))
	var motion_frame := int(floor(_elapsed * MOTION_FRAMES_PER_SECOND))
	for entry in _motion_sets:
		_show_frame(entry, posmod(motion_frame, entry["frames"].size()))
	var travelled := _elapsed * BELT_TILES_PER_SECOND
	for item in _flow_items:
		var span: float = item["to"] - item["from"]
		var along := fposmod(travelled + float(item["slot"]), float(item["count"]))
		var node: Node3D = item["node"]
		# An item off the end of its run is inside a machine: hidden until
		# its slot comes round to the start again.
		node.visible = along <= span
		node.position.x = (item["from"] + minf(along, span)) * TILE


func _show_frame(entry: Dictionary, frame: int) -> void:
	if entry["shown"] == frame:
		return
	var frames: Array = entry["frames"]
	frames[entry["shown"]].visible = false
	frames[frame].visible = true
	entry["shown"] = frame


## Each puff's life is a phase in [0, 1), staggered by its index — never
## `randf()` (godot_project/CLAUDE.md), so a review shot repeats. It swells
## as it leaves the vat and shrinks to nothing at the top.
func _animate_steam() -> void:
	for i in range(_steam.size()):
		var puff: Node3D = _steam[i]
		var life := fposmod(_elapsed / STEAM_LIFE + float(i) / float(_steam.size()), 1.0)
		var drift := Vector3(sin(float(i) * 2.39) * 0.9, 0.0, cos(float(i) * 1.71) * 0.7)
		puff.position = _steam_origin + drift * (0.4 + life) + Vector3(0.0, life * STEAM_RISE, 0.0)
		var size := sin(life * PI) * (0.55 + 0.35 * life)
		puff.scale = Vector3.ONE * maxf(size, 0.001)
		puff.rotation.y = life * 2.0 + float(i)


func _animate_sign() -> void:
	if _sign_pivot == null:
		return
	# The drop: from well above the top of the frame, landing with one
	# damped bounce on its rods.
	var drop := _intro_ease(INTRO_SIGN_DROP_FROM, INTRO_SIGN_DROP_TO)
	var bounce := sin(drop * PI * 2.5) * (1.0 - drop) * 0.35
	var lift := (1.0 - drop + bounce) * _sign_rod_top * 1.6
	_sign_pivot.position.y = lift
	_sign_pivot.rotation_degrees.z = (
		sin(TAU * _elapsed / SIGN_SWAY_PERIOD) * SIGN_SWAY_DEGREES
		+ (1.0 - drop) * 4.0
	)
	# The letters catch the way a struck tube does: a couple of stutters,
	# then on.
	if _sign_neon != null:
		var lit := 1.0
		var since := intro_time - INTRO_NEON_AT
		if since < 0.0:
			lit = 0.0
		elif since < 0.3:
			lit = 0.25 if fmod(since, 0.1) < 0.05 else 0.9
		_sign_neon.set_shader_parameter("emission_energy", _sign_neon_energy * lit)
	var chase := 0
	if intro_time >= INTRO_BULBS_AT:
		chase = int(floor(_elapsed * SIGN_CHASE_PER_SECOND))
	for k in range(_sign_bulbs.size()):
		_sign_bulbs[k].visible = k == posmod(chase, _sign_bulbs.size())


## The hall's lights striking: exposure from dark to full, with one stutter
## as the tubes catch. On the environment `factory_environment.gd` built, so
## once the intro is done the hall is exactly the game's.
func _animate_lights() -> void:
	if _world_env == null or _world_env.environment == null:
		return
	var level := _intro_ease(INTRO_LIGHTS_FROM, INTRO_LIGHTS_TO)
	if intro_time > 0.25 and intro_time < 0.38:
		level *= 0.35
	_world_env.environment.tonemap_exposure = _full_exposure * maxf(level, 0.02)


## Hangs the sign so its BOARD fills `rect` (screen pixels) as nearly as its
## proportions allow, centred in it — `menu.gd` hands over the space its
## column keeps free for the wordmark, so the 2D and 3D halves of the title
## cannot drift apart at any window size. The rig is a child of the camera,
## so the sign stays framed while the factory drifts past behind it.
func frame_sign(rect: Rect2, viewport_size: Vector2) -> void:
	if _sign_rig == null or viewport_size.y <= 0.0 or _sign_board_size.x <= 0.0:
		return
	var world_per_px := 2.0 * SIGN_DISTANCE * tan(deg_to_rad(_camera.fov) * 0.5) / viewport_size.y
	var fit := minf(
		rect.size.x * world_per_px / _sign_board_size.x,
		rect.size.y * world_per_px / _sign_board_size.y
	)
	var centre := rect.get_center() - viewport_size * 0.5
	_sign_rig.scale = Vector3.ONE * fit
	# The rig's origin is the rods' top (the sign hangs from it); the board's
	# foot is `_sign_rod_top` below that, and goes half a board under the
	# rect's centre.
	_sign_rig.position = Vector3(
		centre.x * world_per_px,
		-centre.y * world_per_px + (_sign_rod_top - _sign_board_size.y * 0.5) * fit,
		-SIGN_DISTANCE
	)


## Hides the sign while the menu's settings rows have its space, and brings
## it back with the buttons — the 3D half of `menu.gd`'s wordmark.
func set_sign_visible(shown: bool) -> void:
	if _sign_rig != null:
		_sign_rig.visible = shown


## Where the camera sits at the middle of its drift: south of the line and
## roughly level with its middle, looking at it nearly broadside, low enough
## to be standing on the floor rather than looking down at a diagram.
##
## The framing is **built around a centred menu**. The plate covers the
## middle of the screen, so the two things worth seeing are put either side
## of it: tomatoes riding into the crusher on the left, bottles coming out of
## the bottler on the right — "sugar in, sauce out" spread across the frame
## with the menu in the gap. An earlier version looked down the line from its
## west end, which read well under a menu standing off to one side and left
## one dead half of the screen once the menu moved to the middle.
##
## The whole composition is judged in a rendered frame
## (`scripts/screenshot-scene.sh out.png 1600x900 --title`), not from these
## numbers — UI_UX.md §2b point 12.
func _camera_home() -> Vector3:
	return Vector3(2.8 * TILE, CAMERA_HEIGHT, 8.4 * TILE)


## Barely turned off broadside — just enough that the belts and machine faces
## are not perfectly flat to the screen, which would read as an elevation
## drawing rather than a photograph.
func _camera_yaw() -> float:
	return -6.0


func _build_camera() -> void:
	_camera = Camera3D.new()
	_camera.name = "TitleCamera"
	# PERSPECTIVE, deliberately, where the game is orthographic (UI_UX.md
	# §1). The fixed isometric projection is a rule about how the game is
	# PLAYED — it keeps the factory readable and the grid honest. A title
	# screen is a photograph of the place, and the hall's converging roof and
	# receding line are the whole subject; an orthographic poster would throw
	# away the depth this backdrop exists to show.
	_camera.projection = Camera3D.PROJECTION_PERSPECTIVE
	_camera.fov = 58.0
	_camera.near = 0.1
	_camera.far = 400.0
	_camera.position = _camera_home()
	_camera.rotation_degrees = Vector3(-3.0, _camera_yaw(), 0.0)
	# A long lens's depth: the line and the sign sharp, the far plant and the
	# hall's end wall going soft behind them. It is what makes the frame read
	# as a photograph of a place rather than as a render of a level.
	var lens := CameraAttributesPractical.new()
	lens.dof_blur_far_enabled = true
	lens.dof_blur_far_distance = DOF_FAR_FROM
	lens.dof_blur_far_transition = DOF_FAR_TRANSITION
	lens.dof_blur_amount = DOF_AMOUNT
	_camera.attributes = lens
	add_child(_camera)


## The same poured-concrete floor the game lays, from the same four models
## picked by the same hash — `main.gd`'s `_floor_variant_for` and
## `_floor_yaw_steps_for` are static for exactly this reason. A tile of floor
## in the menu is the same tile of floor in the game, worn patch for worn
## patch.
func _build_floor() -> void:
	var by_variant := {}
	for tz in range(FLOOR_FROM.y, FLOOR_TO.y + 1):
		for tx in range(FLOOR_FROM.x, FLOOR_TO.x + 1):
			var variant := MainScript._floor_variant_for(tx, tz)
			if not by_variant.has(variant):
				by_variant[variant] = []
			var steps := MainScript._floor_yaw_steps_for(tx, tz, variant)
			var basis := Basis(Vector3.UP, deg_to_rad(90.0 * steps))
			by_variant[variant].append(Transform3D(basis, _tile_centre(tx, tz, 0.0)))

	var floor_node := Node3D.new()
	floor_node.name = "Floor"
	add_child(floor_node)

	var floor_material = _assets.class_materials.get("matte", null)
	for variant in by_variant:
		var mesh := _assets.mesh_for(variant)
		if mesh == null:
			continue
		var placements: Array = by_variant[variant]
		var multimesh := MultiMesh.new()
		multimesh.transform_format = MultiMesh.TRANSFORM_3D
		multimesh.mesh = mesh
		multimesh.instance_count = placements.size()
		for i in range(placements.size()):
			multimesh.set_instance_transform(i, placements[i])
		var node := MultiMeshInstance3D.new()
		node.name = variant
		node.multimesh = multimesh
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		if floor_material != null:
			node.material_override = floor_material
		floor_node.add_child(node)


func _build_models() -> void:
	var plant := Node3D.new()
	plant.name = "Plant"
	add_child(plant)

	for entry in CHAIN:
		if String(entry[0]).begins_with("belt_"):
			_build_belt(plant, entry)
			continue
		plant.add_child(_placed(entry[0], entry[1], entry[2], entry[3], FLOOR_Y))
		_build_motion(plant, entry)
	for entry in BACKGROUND_PLANT:
		plant.add_child(_placed(entry[0], entry[1], entry[2], entry[3], FLOOR_Y))
		_build_motion(plant, entry)

	# The hopper and the tanks are drawn with their contents showing, the
	# same two-part way the game draws them: a body plus a `_fill` slab. A
	# full hopper says the line is about to run.
	var hopper_fill := _placed("hopper_fill", -3, LINE_Z, 0, FLOOR_Y)
	hopper_fill.name = "hopper_fill"
	plant.add_child(hopper_fill)

	_build_flows(plant)
	_build_steam(plant)


## Every machine's moving parts, drawn the way the game draws them: the
## kind's motion overlay in `MACHINE_FRAMES` arrangements, siblings of the
## body, one shown at a time (`main.gd`'s `MACHINE_MOTION_MODELS`). Kinds
## with no entry there (the dock) have nothing that moves.
func _build_motion(plant: Node3D, entry: Array) -> void:
	var asset := String(entry[0])
	var kind := asset.trim_prefix("machine_")
	if not asset.begins_with("machine_") or not MainScript.MACHINE_MOTION_MODELS.has(kind):
		return
	var family := String(MainScript.MACHINE_MOTION_MODELS[kind])
	var frames: Array = []
	for k in range(MainScript.MACHINE_FRAMES):
		var node := _placed("%s_f%d" % [family, k], entry[1], entry[2], entry[3], FLOOR_Y)
		# Placed as the body is — the vat's centring keys on the body's name.
		if kind == "cooking_vat":
			node.position = _vat_centre(FLOOR_Y)
		node.visible = k == 0
		plant.add_child(node)
		frames.append(node)
	_motion_sets.append({"frames": frames, "shown": 0})


## A belt tile as its four frames, one shown — `_animate_line` marches them.
func _build_belt(plant: Node3D, entry: Array) -> void:
	var frames: Array = []
	for k in range(MainScript.BELT_FRAMES):
		var node := _placed("belt_f%d" % k, entry[1], entry[2], entry[3], FLOOR_Y)
		node.visible = k == 0
		plant.add_child(node)
		frames.append(node)
	_belt_sets.append({"frames": frames, "shown": 0})


func _build_flows(plant: Node3D) -> void:
	for flow in FLOWS:
		var span: float = flow[2] - flow[1]
		# One slot per tile of run plus one hidden inside the next machine,
		# so the spacing stays a tile and the stream never bunches.
		var count := int(ceil(span)) + 1
		for slot in range(count):
			var node := _assets.asset_node(String(flow[0]))
			node.position = Vector3(float(flow[1]) * TILE, BELT_SURFACE_Y, float(LINE_Z) * TILE)
			plant.add_child(node)
			_flow_items.append({
				"node": node, "from": float(flow[1]), "to": float(flow[2]),
				"slot": slot, "count": count,
			})


func _build_steam(plant: Node3D) -> void:
	var top := TILE
	var vat_mesh := _assets.mesh_for("machine_cooking_vat")
	if vat_mesh != null:
		top = vat_mesh.get_aabb().end.y
	_steam_origin = _vat_centre(FLOOR_Y + top)
	for i in range(STEAM_PUFFS):
		var puff := _assets.asset_node("steam_puff")
		puff.name = "Steam%d" % i
		for child in puff.get_children():
			(child as GeometryInstance3D).cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		plant.add_child(puff)
		_steam.append(puff)


## The marquee sign over the menu (`title_sign` in assets.rs), its bulb
## frames, and a pivot at the rods' top so its sway is a hanging sign's.
func _build_sign() -> void:
	_sign_rig = Node3D.new()
	_sign_rig.name = "TitleSign"
	_camera.add_child(_sign_rig)
	_sign_pivot = Node3D.new()
	_sign_pivot.name = "SignPivot"
	_sign_rig.add_child(_sign_pivot)

	var body := _assets.asset_node(SIGN_ASSET)
	_sign_pivot.add_child(body)
	var mesh := _assets.mesh_for(SIGN_ASSET)
	if mesh != null:
		var extent := mesh.get_aabb().size
		_sign_board_size = Vector2(
			extent.x, extent.y - float(SIGN_ROD_VOXELS) * MainScript.VOXEL_UNIT
		)
	else:
		# Not staged (`cargo xtask` has not run): a sign of nothing still has
		# to be framed somewhere.
		_sign_board_size = Vector2(25.0, 4.6)
	# The letters get their own copy of the lit material, so the intro can
	# strike them without flickering every lamp that shares the class.
	var neon_node := body.get_node_or_null("emissive") as MeshInstance3D
	var shared = _assets.class_materials.get("emissive", null)
	if neon_node != null and shared is ShaderMaterial:
		_sign_neon = (shared as ShaderMaterial).duplicate()
		_sign_neon_energy = float(_sign_neon.get_shader_parameter("emission_energy"))
		neon_node.set_surface_override_material(0, _sign_neon)
	for k in range(SIGN_BULB_FRAMES):
		var bulbs := _assets.asset_node("%s_bulbs_f%d" % [SIGN_ASSET, k])
		bulbs.visible = k == 0
		_sign_pivot.add_child(bulbs)
		_sign_bulbs.append(bulbs)
	# The pivot hangs from the rods' top; the sign itself sits below it.
	var rod_top := _sign_board_size.y + float(SIGN_ROD_VOXELS) * MainScript.VOXEL_UNIT
	for child in _sign_pivot.get_children():
		child.position.y = -rod_top
	_sign_pivot.position.y = 0.0
	_sign_rod_top = rod_top
	for node in _sign_pivot.find_children("*", "GeometryInstance3D", true, false):
		(node as GeometryInstance3D).cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF


## One model at a tile, turned `quarter_turns` steps east-to-south. The vat
## is the one entry whose footprint is not 1x1, so it is centred over its
## rectangle the way `main.gd`'s `_footprint_centre` does.
func _placed(asset: String, tx: int, tz: int, quarter_turns: int, height: float) -> Node3D:
	var node := _assets.asset_node(asset)
	var origin := Vector2i(tx, tz)
	if origin == VAT_TILE and asset == "machine_cooking_vat":
		node.position = _vat_centre(height)
	else:
		node.position = _tile_centre(tx, tz, height)
	node.rotation.y = deg_to_rad(90.0 * float(quarter_turns))
	return node


func _vat_centre(height: float) -> Vector3:
	return Vector3(
		(float(VAT_TILE.x) + float(VAT_SIZE.x - 1) * 0.5) * TILE,
		height,
		(float(VAT_TILE.y) + float(VAT_SIZE.y - 1) * 0.5) * TILE
	)


func _tile_centre(tx: int, tz: int, height: float) -> Vector3:
	return Vector3(float(tx) * TILE, height, float(tz) * TILE)


## The hall size the menu's backdrop is drawn at. Falls back to x1.00 with no
## `Settings` autoload — the poster is art, and art with no settings file
## behind it is still a poster.
func _hall_scale() -> float:
	var settings := _settings_node()
	return 1.0 if settings == null else float(settings.hall_scale)


## Re-sizes the room behind the menu when the HALL SIZE row moves. One
## argument-free listener because `Settings.changed` carries no key: every
## listener re-reads what it cares about (see `settings.gd`).
func _apply_hall_scale() -> void:
	if _world_env == null:
		return
	FactoryEnvironment.set_hall_scale(_world_env.environment, _hall_scale())


## The `Settings` autoload, or null. Looked up off the tree root for the
## reason `settings.gd::music_node` documents.
func _settings_node() -> Node:
	var loop := Engine.get_main_loop()
	if not (loop is SceneTree):
		return null
	var tree_root: Node = (loop as SceneTree).root
	return tree_root.get_node_or_null("Settings") if tree_root != null else null
