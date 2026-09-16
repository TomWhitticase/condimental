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

## Product standing on the belts, so the line reads as a line that runs
## rather than as a showroom: tomatoes going into the crusher, bottles coming
## out of the bottler, boxes waiting for the dock. Put here by hand — see the
## header; nothing about this screen is simulated.
const PRODUCT := [
	["item_tomato", -2, 0],
	["item_tomato", -1, 0],
	["item_tomato", 2, 0],
	["item_bottle", 11, 0],
	["item_bottle", 12, 0],
	["item_box", 14, 0],
]

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

var _assets := VoxelAssets.new()
var _camera: Camera3D
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


func _ready() -> void:
	name = "TitleScene"
	_assets.load_materials()
	FactoryEnvironment.build_into(self)
	_build_floor()
	_build_models()
	_build_camera()


## The camera's slow drift. `_process` on the backdrop only — the menu's own
## Control tree is static, and a title screen that redraws for a moving
## camera is the one place in this project where that is the point.
func _process(delta: float) -> void:
	_elapsed += delta
	var phase := sin(TAU * _elapsed / DRIFT_PERIOD)
	_camera.position.x = _camera_home().x + phase * DRIFT_TRAVEL
	_camera.rotation_degrees.y = _camera_yaw() + phase * DRIFT_YAW


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
		plant.add_child(_placed(entry[0], entry[1], entry[2], entry[3], FLOOR_Y))
	for entry in BACKGROUND_PLANT:
		plant.add_child(_placed(entry[0], entry[1], entry[2], entry[3], FLOOR_Y))
	for entry in PRODUCT:
		plant.add_child(_placed(entry[0], entry[1], LINE_Z, 0, BELT_SURFACE_Y))

	# The hopper and the tanks are drawn with their contents showing, the
	# same two-part way the game draws them: a body plus a `_fill` slab. A
	# full hopper says the line is about to run.
	var hopper_fill := _placed("hopper_fill", -3, LINE_Z, 0, FLOOR_Y)
	hopper_fill.name = "hopper_fill"
	plant.add_child(hopper_fill)


## One model at a tile, turned `quarter_turns` steps east-to-south. The vat
## is the one entry whose footprint is not 1x1, so it is centred over its
## rectangle the way `main.gd`'s `_footprint_centre` does.
func _placed(asset: String, tx: int, tz: int, quarter_turns: int, height: float) -> Node3D:
	var node := _assets.asset_node(asset)
	var origin := Vector2i(tx, tz)
	if origin == VAT_TILE and asset == "machine_cooking_vat":
		node.position = Vector3(
			(float(tx) + float(VAT_SIZE.x - 1) * 0.5) * TILE,
			height,
			(float(tz) + float(VAT_SIZE.y - 1) * 0.5) * TILE
		)
	else:
		node.position = _tile_centre(tx, tz, height)
	node.rotation.y = deg_to_rad(90.0 * float(quarter_turns))
	return node


func _tile_centre(tx: int, tz: int, height: float) -> Vector3:
	return Vector3(float(tx) * TILE, height, float(tz) * TILE)
