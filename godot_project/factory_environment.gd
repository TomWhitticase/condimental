## The hall the factory stands in: its air, its light and the backdrop you
## see past the machines (`factory_interior_sky.gdshader`).
##
## Lifted out of `main.gd` when the title screen began standing real machines
## behind the main menu. The menu is looking down the SAME hall the game is
## played in — that is the whole point of it — so the sky's twelve colours,
## the exposure and the two lights are stated once, here, and both scenes
## call `build_into`. A second copy in the menu would drift the first time
## either was tuned, and "the menu is lit differently from the game" is the
## exact defect this arrangement exists to make impossible.
##
## **One parameter, and only one, differs between the two callers**: the
## title screen asks for clear air (`TITLE_HAZE_DENSITY`). That is a property
## of the photograph, not of the lighting — the surfaces, both lights, the
## exposure and the palette are still the game's, and the models in the title
## frame measure within three levels of what they did under the hazy sky.
##
## Reached by `preload`, never `class_name` (godot_project/CLAUDE.md).
extends RefCounted

## The hall's air: the colour every distant surface in the sky shader fades
## to, and — see `set_backdrop_for` — what the isometric view shows in place
## of the hall itself. Stated once and passed to the shader below.
const HAZE_COLOR := Color("#939495")

## How fast that air swallows the hall (the shader's `haze_density`). In the
## game it is doing most of the work of making the place feel deep: the far
## wall is 132 units off and the player is walking about in it, so distance
## has to read as distance.
const HAZE_DENSITY := 0.0052

## The title screen gets **clear air**. Owner's call, and it is the right one
## for a poster: haze is atmosphere you notice over a whole play session, but
## in one still frame under a wordmark it reads as a grey wash sitting on the
## picture. A photograph of the hall wants the roof, the glazing and the far
## plant line crisp, which is also what makes the hand-placed chain in front
## of them look like it is standing somewhere specific.
const TITLE_HAZE_DENSITY := 0.0


## UI_UX.md §4: real-time dynamic lighting with soft shadows, light ambient
## occlusion in voxel crevices, and mild bloom on emissives — the
## "modded Minecraft with a shader pack" reference point, not photorealism.
##
## The sky IS the visible backdrop (BG_SKY), not just a light source — a
## flat near-black BG_COLOR was tried first specifically to avoid a hard
## horizon line/sun streak behind the tightly-framed isometric view, but it
## reads as an empty void the moment first-person mode lets you look around
## freely, which fights UI_UX.md §3's "a real factory you could walk into,"
## not a factory floating in nothing.
##
## An outdoor sky was the first answer to that, and it made the factory an
## open-air slab of concrete under blue daylight. The backdrop is now the
## INSIDE of a hall far bigger than anything the player can build — roof
## trusses and skylight ribbons overhead, clad walls with clerestory glazing
## and other plant standing along them, all fading into air haze
## (`factory_interior_sky.gdshader`). It keeps every property the outdoor sky
## was chosen for — no sun streak, no void, a real place to look at in
## first-person — and answers UI_UX.md §3 with the setting itself.
##
## Colours below are palette.ron's, by name, so the backdrop is made of the
## same industrial greys as the voxel models standing in front of it.
static func build_into(parent: Node, haze_density: float = HAZE_DENSITY) -> WorldEnvironment:
	var sky_material := ShaderMaterial.new()
	sky_material.shader = preload("res://factory_interior_sky.gdshader")
	sky_material.set_shader_parameter("deck_color", Color("#3A3C40"))      # painted deck, below steel_dark
	sky_material.set_shader_parameter("truss_color", Color("#6F7680"))     # steel_dark
	sky_material.set_shader_parameter("purlin_color", Color("#565A60"))
	sky_material.set_shader_parameter("cladding_color", Color("#6A6A63"))  # concrete
	sky_material.set_shader_parameter("plinth_color", Color("#43433F"))    # concrete_dark
	sky_material.set_shader_parameter("slab_color", Color("#979489"))     # a touch over concrete_light, warm like the floor tiles
	sky_material.set_shader_parameter("hazard_color", Color("#D9A21B"))    # hazard_yellow
	sky_material.set_shader_parameter("lamp_color", Color("#FFF6EA"))
	sky_material.set_shader_parameter("window_color", Color("#B9C6CE"))
	# The haze is what the eye reads as "this hall goes on" — lighter than the
	# surfaces it swallows, as lit dust actually is, but kept neutral: a warm
	# haze tints the ambient, and the concrete floor then photographs beige.
	sky_material.set_shader_parameter("haze_color", HAZE_COLOR)
	sky_material.set_shader_parameter("haze_density", haze_density)

	var sky := Sky.new()
	sky.sky_material = sky_material
	# Nothing in the shader animates (no TIME, no LIGHT_*), so the radiance
	# map is worth baking once at full quality rather than letting AUTOMATIC
	# pick the incremental path and drip it in over the first frames — which
	# a 45-frame screenshot run would photograph half-converged.
	sky.process_mode = Sky.PROCESS_MODE_QUALITY

	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	# Metal now reflects a roof and lit glazing instead of blue daylight,
	# which is most of why the stainless reads as indoor stainless.
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	# Sky-derived ambient is what fills the shadowed sides of a voxel model;
	# without it, unlit faces read as flat black and the medium looks cheap.
	# An interior is a much darker source than an open sky, so the energy is
	# up from 0.8 to hold the same fill on those faces.
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 1.2

	env.ssao_enabled = true
	env.ssao_radius = 0.9
	env.ssao_intensity = 2.4
	env.ssao_power = 1.6

	env.glow_enabled = true
	env.glow_intensity = 0.7
	env.glow_bloom = 0.15
	env.glow_hdr_threshold = 0.85
	env.glow_blend_mode = Environment.GLOW_BLEND_MODE_SOFTLIGHT

	env.tonemap_mode = Environment.TONE_MAPPER_ACES
	env.tonemap_white = 6.0
	# 0.92 under the first dim sky, 1.05 under the bright outdoor one; back a
	# little now that the hall's roof and lamps light the scene.
	env.tonemap_exposure = 1.0
	# Deliberately no depth fog: it was tried, and at this camera distance it
	# flattened the whole factory into one haze-grey value and cost more
	# contrast than the atmosphere was worth.

	# Key light: warm, shadow-casting, angled so machinery casts shadows
	# across the belt line rather than straight down it. Steeper than the
	# original -52 now that it stands in for the roof's high-bay fittings
	# rather than a sun, but deliberately not vertical: overhead light casts
	# shadows straight down the line, where nothing can read them.
	var sun := DirectionalLight3D.new()
	sun.name = "KeyLight"
	sun.rotation_degrees = Vector3(-58.0, -130.0, 0.0)
	# Lamp-white rather than the old sunlight-warm 0.95/0.88. With no blue
	# sky left to balance it, a warm key turned every concrete surface in the
	# game beige — the floor measured r-b = 21 against the old frame's 5.
	sun.light_color = Color(1.0, 0.98, 0.95)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 160.0
	sun.shadow_blur = 1.4
	# A non-zero angular size is what actually softens the shadow edge with
	# distance, rather than just blurring the whole map uniformly.
	sun.light_angular_distance = 0.6
	parent.add_child(sun)

	# Fill from the opposite side, no shadows — keeps the shadowed faces of
	# stainless steel from going dead grey. Still cool, but daylight through
	# the clerestory glazing rather than open sky: the warm key needs
	# something on the other side of neutral to balance it, or every grey
	# asset in the game photographs beige.
	var fill := DirectionalLight3D.new()
	fill.name = "FillLight"
	fill.rotation_degrees = Vector3(-28.0, 55.0, 0.0)
	fill.light_color = Color(0.80, 0.85, 0.96)
	fill.light_energy = 0.4
	fill.shadow_enabled = false
	parent.add_child(fill)

	var world_env := WorldEnvironment.new()
	world_env.name = "Environment"
	world_env.environment = env
	parent.add_child(world_env)
	return world_env


## Which backdrop a camera gets, and the one thing about this hall that is
## NOT the same in both view modes.
##
## The hall is a sky: every surface of it is at infinity and is found by
## intersecting a box against `EYEDIR`. That needs rays that fan out, and an
## **orthographic camera's rays are parallel** — the whole screen samples
## barely a degree of direction, so two or three roof bays get magnified into
## hard white stripes that fan across the corners of the frame and slide
## sideways as the player pans, since a sky has no parallax to pin them to
## the floor. It is not a bug in the shader; a painted infinite room simply
## cannot be photographed with a parallel projection.
##
## So the hall is drawn where it reads and stands up to being looked at —
## first person, which is where the player actually looks around — and the
## isometric view gets the same hall's AIR instead: a flat fill of the very
## colour every distant surface in the shader fades to, so the floor runs out
## into haze rather than into a starburst. Pressing TAB then reveals the room
## that haze was standing in, which is the right way round.
##
## **Only the background changes.** The sky resource stays on the environment
## and stays the ambient and reflection source in both modes (that is what
## fills the shadowed side of every voxel and puts a roof in the stainless),
## so the factory itself is lit identically either way and TAB changes what is
## BEHIND the world, not the world.
static func set_backdrop_for(env: Environment, first_person: bool) -> void:
	if env == null:
		return
	if first_person:
		env.background_mode = Environment.BG_SKY
	else:
		env.background_mode = Environment.BG_COLOR
		env.background_color = HAZE_COLOR
