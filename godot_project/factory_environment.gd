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

## How big the hall is, at HALL SIZE x1.00 — the shader's own defaults, moved
## up here because the settings screen can now scale them.
##
## They were literals inside `factory_interior_sky.gdshader` and `build_into`
## never set them, which was fine while nothing could change them and is not
## fine now: a uniform's default and a scale applied on top of it are two
## sources for one number, and the first time somebody tuned the shader the
## slider would have been multiplying something else. `build_into` states all
## three explicitly, so GDScript owns them and the shader's defaults are only
## what the editor's material preview shows.
##
## **Three, not five.** `floor_drop` stays put — the slab is about one
## eye-height under the camera and that is a relationship to the PLAYER, not
## to the room, so a bigger hall must not drop the floor away. `haze_density`
## stays put too: haze is measured per unit of distance, so a hall twice as
## deep already reads hazier at the far wall without touching it, which is
## exactly how air works.
const HALL_HALF_WIDTH := 132.0 # wall distance: ~41 tiles each way
const HALL_ROOF_HEIGHT := 26.0 # underside of the roof deck
const HALL_BAY_PITCH := 26.0   # structural bay: one truss, one column


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
static func build_into(
	parent: Node, haze_density: float = HAZE_DENSITY, hall_scale: float = 1.0
) -> WorldEnvironment:
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
	_write_hall_scale(sky_material, hall_scale)

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

	# UI_UX.md §4's "light SSAO" — retuned 2026-09-20 alongside the glow, and
	# for the same reason (issue #32): the shipped numbers had never been
	# judged against a rendered frame. The asset browser's worker measured
	# them while calibrating a baked AO pass (issue #49) and found the pass
	# near-inert; differencing rendered PNGs against `ssao_enabled = false`
	# agrees. At 0.9 / 2.4 the deepest of three crease boxes in an isometric
	# closeup lost 1.7 levels out of 133 (x0.987) and 0.22% of the frame
	# darkened by more than 3% — under the eye's threshold on a flat voxel
	# face, i.e. nothing.
	#
	# Intensity alone cannot fix it, because the pass is NOT equally weak in
	# every view: its radius is in world units, so an eye-level camera with
	# geometry a metre away finds far more occlusion per pixel than the
	# orthographic camera looking down from across the hall. 0.9 / 16 reads
	# nicely in isometric and is much too strong in first person, where it
	# swallows the belt's legs in a smear and hangs a dark halo off the
	# belt's lower edge. HALVING THE RADIUS is what reconciles them: it costs
	# isometric bite but takes much more off the first-person smear, because
	# the smear is the far end of a radius that was never crease-sized. 0.45
	# world units is 4.5 voxels — a fitting against a body, not a machine
	# against the floor.
	#
	# Measured on the bottleneck line at a pinned tick, 1200x800, against
	# `ssao_enabled = false`. "crease" is the mean loss in three hand-picked
	# boxes (a pot shoulder, a gantry against a pot, a crusher leg); "smear"
	# is the share of the frame darkened by more than 3%, which is where
	# overshoot shows up first.
	#
	#   radius/intensity    iso crease   iso smear   eye smear   crease:flat
	#   0.9  / 2.4  (old)     -1.2 lv      0.22%       7.8%         2.0x
	#   0.9  / 8              -4.5 lv      1.40%      15.3%         1.9x
	#   0.9  / 16             -7.8 lv      2.83%      18.7%         1.8x  smears
	#   0.45 / 8              -2.1 lv      0.41%      10.2%         2.3x
	#   0.45 / 16   (new)     -3.7 lv      0.87%      12.9%         2.2x
	#   0.45 / 24             -4.8 lv      1.16%      14.0%         2.1x
	#   0.3  / 16             -2.1 lv      0.50%       9.4%         2.3x
	#
	# 0.45 / 16 is the one that reads in all three framings checked
	# (isometric closeup, play zoom, first person on the floor) without any
	# of them looking grubby. Full record in docs/DECISIONS.md.
	env.ssao_enabled = true
	env.ssao_radius = 0.45
	env.ssao_intensity = 16.0
	env.ssao_power = 1.6

	# UI_UX.md §4's "mild bloom on EMISSIVES" — retuned 2026-09-20 (issue #32).
	#
	# The first four numbers here (0.7 / 0.15 / 0.85 / SOFTLIGHT) were chosen
	# while NOTHING in the game emitted: issue #25 meant every lamp was
	# ordinary painted amber at any energy, so the pass was tuned against an
	# input that did not exist. Retuned against the input it actually has, by
	# differencing rendered PNGs of the bottleneck line's crusher and scoring
	# what the pass adds inside a box around one lit status lens against what
	# it adds to the whole rest of the frame — SELECTIVITY, on a deterministic
	# render at a pinned tick. Every figure below is that measurement, in
	# 0-255 levels, against a baseline with the pass ENABLED and contributing
	# nothing (bloom 0, threshold 3.0); see DECISIONS.md for why both of those
	# conditions matter.
	#
	#   intensity/bloom/threshold/blend     lens box   max   rest    ratio
	#   0.7 / 0.15 / 0.85 / SOFTLIGHT  (old) +3.22     +29   +1.21    2.7x
	#   0.7 / 0.0  / 0.85 / SOFTLIGHT        +2.12     +28   +0.15   14.6x
	#   1.0 / 0.0  / 1.2  / SOFTLIGHT        +1.99     +30   +0.04   48.2x
	#   1.0 / 0.0  / 0.85 / SCREEN          +21.58    +218   +0.79   27.3x
	#   1.0 / 0.0  / 1.0  / SCREEN          +19.23    +217   +0.52   37.2x
	#   0.7 / 0.0  / 1.2  / SCREEN          +12.29    +199   +0.17   71.8x
	#   1.0 / 0.0  / 1.2  / SCREEN     (new) +16.12   +213   +0.23   69.1x
	#   1.0 / 0.0  / 1.4  / SCREEN          +13.14    +208   +0.07  179.5x
	#   1.0 / 0.0  / 1.2  / ADDITIVE        +16.52    +213   +0.25   66.4x
	#
	# What each number is, and why:
	#
	# * `glow_bloom = 0.0` (was 0.15). Godot's `glow_bloom` makes glow visible
	#   in areas DARKER than the threshold — it is literally "bloom everything
	#   a little", and it was the only thing producing any bloom at all back
	#   when nothing emitted, which is exactly why it was turned up. It is the
	#   whole of the frame-wide haze: zeroing it alone takes the rest-of-frame
	#   lift from +1.21 to +0.15 (rows 1 and 2), and it is why the old settings
	#   fired on the crusher's white lid, the froth in the tomato jars and the
	#   box labels on the dock, softening all three.
	# * `GLOW_BLEND_MODE_SCREEN` (was SOFTLIGHT). The one that decides whether
	#   a lamp reads at all. SOFTLIGHT's effect scales with the pixel under it,
	#   so it cannot brighten an already-bright lens: its brightest single
	#   pixel is +25..30 at ANY intensity or threshold, against SCREEN's +213.
	#   The lens stays a flat cream cube under SOFTLIGHT however it is tuned.
	# * `glow_hdr_threshold = 1.2` (was 0.85), and it is the knee of the sweep,
	#   not a guess. 1.0-1.2 is where the brightest LIT WHITE surface in the
	#   game sits, which is the line between "bright" and "emitting". Below it
	#   contamination climbs fast (+0.79 at 0.85); above it the lamp is what
	#   pays — 1.4 buys +0.07 instead of +0.23, a difference already under one
	#   luma level, for 19% of the halo.
	# * `glow_intensity = 1.0` (was 0.7), to pay back the reach the other three
	#   took away: at the same threshold, 1.0 against 0.7 is +16.12/+213
	#   against +12.29/+199 for +0.06 of contamination.
	# * ADDITIVE was measured too and is within noise of SCREEN here. SCREEN is
	#   kept because it retains marginally more hue in the lens (saturation
	#   0.36 against 0.34) and is bounded by construction, so a brighter
	#   emissive added later cannot push a lens past white.
	#
	# The effect: the halo around a lit lens goes from 1480 px at rgb(6,2,1)
	# to 7136 px at rgb(18,9,2) — five times the reach, three times the
	# strength, and amber either way (saturation 0.87), which is §4's "a lit
	# voxel glows in its OWN colour". Overall exposure is unmoved (mean luma
	# 135.1 -> 134.4 on that closeup, 153.8 -> 155.0 in first person): this is
	# a change in WHAT glows, not in how bright the game is. In first person
	# the old settings lifted the roof lamps' own FACES (+2.43) and barely
	# their surroundings (+1.67), which is why they read as flat white
	# rectangles against the skylights; the new ones leave the faces alone
	# (+1.91) and light the dark roof around them (+7.26), which is what a
	# lamp does. `SCREENSHOT_RENDER` is identical either way — a full-screen
	# post-pass costs the same whatever it is tuned to.
	env.glow_enabled = true
	env.glow_intensity = 1.0
	env.glow_bloom = 0.0
	env.glow_hdr_threshold = 1.2
	env.glow_blend_mode = Environment.GLOW_BLEND_MODE_SCREEN

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


## Re-sizes the hall on a live environment — the HALL SIZE row of the
## settings screen, applied without rebuilding anything. `set_backdrop_for`'s
## twin: the one place that decides, called by everyone who needs it.
##
## The sky is a shader, so a bigger hall costs nothing to draw; what it does
## cost is one radiance re-bake, because `build_into` asks for
## `PROCESS_MODE_QUALITY` (nothing in the shader animates, so a full bake once
## beats an incremental one dripping in over the first frames of a screenshot
## run). That is why this is a *stepped* setting and not a dragged slider:
## eight presses of `+` are eight bakes, a drag would be one per frame.
static func set_hall_scale(env: Environment, hall_scale: float) -> void:
	if env == null or env.sky == null:
		return
	var material := env.sky.sky_material
	if material is ShaderMaterial:
		_write_hall_scale(material as ShaderMaterial, hall_scale)


## The three uniforms, scaled together so the hall keeps its proportions —
## a room twice as wide with the same roof on it is a corridor, not a bigger
## factory. See the `HALL_*` consts for which two are deliberately left out.
static func _write_hall_scale(material: ShaderMaterial, hall_scale: float) -> void:
	var scale: float = maxf(0.05, hall_scale)
	material.set_shader_parameter("hall_half_width", HALL_HALF_WIDTH * scale)
	material.set_shader_parameter("roof_height", HALL_ROOF_HEIGHT * scale)
	material.set_shader_parameter("bay_pitch", HALL_BAY_PITCH * scale)
