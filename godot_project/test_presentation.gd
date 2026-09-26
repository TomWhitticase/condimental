# Headless proof of the visual-polish pass's *structure* (docs/UI_UX.md §1,
# §2, §4) — the part of "does it look right" that a machine can actually
# check without a framebuffer.
#
# Run with:
#   godot --headless --path godot_project --script test_presentation.gd
#
# This deliberately does NOT claim to verify that the scene looks good; that
# is what the isometric previews in assets/voxels/previews/ and a real
# windowed run are for. What it does verify is every structural claim the
# visual pass makes, each of which would otherwise rot silently:
#
#   * the voxel models actually imported and produced non-empty meshes
#     (i.e. the pinned .vox importer + the hand-written encoder still agree),
#   * the camera is fixed and ORTHOGRAPHIC — UI_UX.md §1's central rule,
#   * real-time lighting with shadows and an SSAO/glow environment exist
#     (§4), rather than the scene silently falling back to flat ambient,
#   * metal is genuinely shinier than matte (§4's deliberate exception),
#   * the chain rendered has one machine per stage the SIMULATION reports,
#     not a hardcoded count,
#   * an item spawned through the real command path gets a voxel node.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const FactoryEnvironment = preload("res://factory_environment.gd")


func _fail(msg: String) -> void:
	printerr("PRESENTATION_FAIL: %s" % msg)
	quit(1)


func _find_all(node: Node, type_name: String, out: Array) -> Array:
	if node.is_class(type_name):
		out.append(node)
	for child in node.get_children():
		_find_all(child, type_name, out)
	return out


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	if not (main is Node3D):
		_fail("main.tscn's root must be a Node3D — UI_UX.md §1/§2 is a 3D voxel scene, not a 2D one")
		return
	root.add_child(main)
	# `_ready()` only fires on the engine's own next-frame processing, which
	# this headless script-mode SceneTree never runs — call it directly.
	main._ready()
	# The world starts empty (the "Real Placement" epic), so the test builds
	# the factory it then asserts is rendered. See test_layouts.gd.
	Layouts.build_demo_line(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()

	# --- Camera: fixed and orthographic (UI_UX.md §1) ---------------------
	var cameras := _find_all(main, "Camera3D", [])
	if cameras.size() != 1:
		_fail("expected exactly 1 Camera3D, found %d" % cameras.size())
		return
	var camera: Camera3D = cameras[0]
	# A session opens in first person (`DEFAULT_VIEW_MODE`), and the two modes
	# share this one node — so TAB to the view the assertions below are about.
	# That there is still exactly ONE Camera3D whichever mode is current is
	# the check above, and it is checked before the switch on purpose.
	main._set_view_mode(main.ViewMode.ISOMETRIC)
	if camera.projection != Camera3D.PROJECTION_ORTHOGONAL:
		_fail("camera must be orthographic (UI_UX.md §1), projection was %d" % camera.projection)
		return
	if camera.size <= 0.0:
		_fail("orthographic camera size must be positive, was %f" % camera.size)
		return

	# --- Lighting (UI_UX.md §4) -------------------------------------------
	var lights := _find_all(main, "DirectionalLight3D", [])
	if lights.is_empty():
		_fail("no DirectionalLight3D — UI_UX.md §4 requires real-time dynamic lighting")
		return
	var shadow_casters := 0
	for light in lights:
		if light.shadow_enabled:
			shadow_casters += 1
	if shadow_casters < 1:
		_fail("no shadow-casting light — UI_UX.md §4 requires soft shadows")
		return

	var envs := _find_all(main, "WorldEnvironment", [])
	if envs.size() != 1:
		_fail("expected exactly 1 WorldEnvironment, found %d" % envs.size())
		return
	var env: Environment = envs[0].environment
	if env == null:
		_fail("WorldEnvironment has no Environment resource")
		return
	if not env.ssao_enabled:
		_fail("SSAO disabled — UI_UX.md §4 asks for ambient occlusion in voxel crevices")
		return
	if not env.glow_enabled:
		_fail("glow disabled — UI_UX.md §4 asks for mild bloom on emissives")
		return
	if env.sky == null:
		_fail("the environment must carry a Sky resource in BOTH view modes — it is the ambient and reflection source, not just the backdrop, and dropping it leaves every shadowed voxel face unlit")
		return
	# The backdrop is the inside of a factory hall, not weather (UI_UX.md §3).
	# Asserted here because it is invisible to every other check: the
	# isometric camera shows a sliver of it in one corner, so an accidental
	# return to an outdoor sky would pass the whole suite and only be caught
	# by someone pressing TAB.
	var sky_material := env.sky.sky_material
	if not (sky_material is ShaderMaterial):
		_fail("the sky must be the factory-interior shader, not a %s — UI_UX.md §3 puts the factory INSIDE a hall, and a ProceduralSkyMaterial is weather" % sky_material.get_class())
		return
	var sky_shader: Shader = (sky_material as ShaderMaterial).shader
	if sky_shader == null or not String(sky_shader.resource_path).ends_with("factory_interior_sky.gdshader"):
		_fail("the sky's shader must be factory_interior_sky.gdshader, was '%s'" % ("<none>" if sky_shader == null else sky_shader.resource_path))
		return
	if env.ambient_light_source != Environment.AMBIENT_SOURCE_SKY:
		_fail("ambient must come from that sky — an interior lights the scene it stands in, and the shadowed faces of a voxel model have nothing else filling them")
		return

	# --- Which backdrop each view mode gets -------------------------------
	# A sky is found by intersecting a box against EYEDIR, which needs rays
	# that fan out; the isometric camera's are parallel, so the hall came out
	# as hard stripes fanning from the corners. So the hall is DRAWN in first
	# person and stood in for by its own haze colour in the isometric view
	# (`factory_environment.gd`'s `set_backdrop_for`). Asserted because it is
	# a one-line property with a rendered symptom: swap the two branches and
	# every test still passes.
	if env.background_mode != Environment.BG_COLOR:
		_fail("the isometric view must fill the background with the hall's haze (BG_COLOR), not draw the sky — an orthographic camera turns the hall into a starburst")
		return
	if not env.background_color.is_equal_approx(FactoryEnvironment.HAZE_COLOR):
		_fail("the isometric backdrop must be the shader's own haze colour %s, was %s — any other colour leaves a visible edge where the floor runs out" % [FactoryEnvironment.HAZE_COLOR, env.background_color])
		return
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	if env.background_mode != Environment.BG_SKY:
		_fail("first person must draw the hall itself (BG_SKY) — that is the view the sky shader exists for")
		return
	main._set_view_mode(main.ViewMode.ISOMETRIC)
	if env.background_mode != Environment.BG_COLOR:
		_fail("TAB back to isometric must restore the haze fill")
		return
	# Only the BACKGROUND changes: the factory is lit identically either way.
	if env.sky == null or env.ambient_light_source != Environment.AMBIENT_SOURCE_SKY:
		_fail("switching view modes must not touch the sky or the ambient source — TAB changes what is BEHIND the world, not the world")
		return

	# --- Assets: real voxel meshes, and metal is shinier than matte -------
	var meshes := _find_all(main, "MeshInstance3D", [])
	if meshes.size() < 2:
		_fail("expected several MeshInstance3D nodes for the chain, found %d" % meshes.size())
		return

	var matte_roughness := -1.0
	var metal_roughness := -1.0
	var metal_metallic := -1.0
	for mi in meshes:
		if mi.mesh == null or mi.mesh.get_surface_count() < 1:
			_fail("MeshInstance3D '%s' has no mesh surfaces — did the .vox import fail?" % mi.name)
			return
		var mat: Material = mi.get_surface_override_material(0)
		if mat == null:
			_fail("MeshInstance3D '%s' has no material override" % mi.name)
			return
		# `emissive` is the one class that is a ShaderMaterial (issue #25):
		# emission has to follow each voxel's OWN colour, which
		# StandardMaterial3D cannot express. Both kinds still take their
		# colour from the mesh's vertices and nowhere else.
		if mat is ShaderMaterial:
			if String(mi.name) != "emissive":
				_fail("'%s' should be a StandardMaterial3D — only 'emissive' needs a shader" % mi.name)
				return
			var shader: Shader = (mat as ShaderMaterial).shader
			if shader == null or not String(shader.resource_path).ends_with("voxel_emissive.gdshader"):
				_fail("the 'emissive' material must use voxel_emissive.gdshader, was '%s'" % ("<none>" if shader == null else shader.resource_path))
				return
			# Unless this particular lamp is currently showing the OFF
			# material, which is emission-free by design — a dead lens
			# that still emitted would be a lamp half on. The guard is
			# about the CLASS material still being built to emit, and an
			# `emissive` mesh wearing `lamp_idle` is a lamp whose state
			# has been set, not a material that lost its emission.
			var energy := float(mat.get_shader_parameter("emission_energy"))
			if energy <= 0.0 and mat != main._lamp_idle:
				_fail("the 'emissive' material must actually emit (UI_UX.md §4/§7)")
				return
			continue
		var std: StandardMaterial3D = mat
		if not std.vertex_color_use_as_albedo:
			_fail("'%s' must use vertex colours as albedo — voxel palettes are the only colour source" % mi.name)
			return
		match String(mi.name):
			"matte":
				matte_roughness = std.roughness
			"metal":
				metal_roughness = std.roughness
				metal_metallic = std.metallic

	if metal_metallic < 0.0 or matte_roughness < 0.0:
		_fail("scene is missing a metal or matte voxel mesh — the material split didn't survive import")
		return
	if metal_metallic <= 0.5:
		_fail("metal material is barely metallic (%f) — UI_UX.md §4's exception is the point" % metal_metallic)
		return
	if metal_roughness >= matte_roughness:
		_fail("metal (rough %f) is not shinier than matte (rough %f) — UI_UX.md §4" % [metal_roughness, matte_roughness])
		return

	# --- What is rendered matches what the simulation says is placed ------
	# A machine or building is a node under "World" named `<kind>_<origin
	# tile>`; a belt, splitter or pipe is a MultiMesh instance the scene
	# reports through `_tile_instance_at` (the pipes & conveyors overhaul's
	# ticket 4). Either way this is a direct correspondence check between
	# `world_layout_json` and the scene — the same "Godot draws what the sim
	# says exists" rule, over a world the test itself built.
	var layout: Array = JSON.parse_string(main.sim.world_layout_json())
	if layout.is_empty():
		_fail("world_layout_json returned nothing after the demo line was built")
		return
	var instanced_tiles := 0
	for placement in layout:
		var tile: Array = placement["tile"]
		var kind := String(placement["kind"])
		var origin := Vector2i(int(tile[0]), int(tile[1]))
		if main._is_instanced_kind(kind):
			var instance: Dictionary = main._tile_instance_at(origin)
			if instance.is_empty() or instance["nodes"].is_empty():
				_fail("no rendered instance for %s at %s — the scene is not following the sim's layout" % [kind, str(origin)])
				return
			instanced_tiles += 1
			continue
		var node_name := "%s_%d,%d" % [kind, origin.x, origin.y]
		if main.get_node_or_null("World/" + node_name) == null:
			_fail("no rendered node for %s — the scene is not following the sim's layout" % node_name)
			return
	if instanced_tiles < 1:
		_fail("the demo line has belts and pipes, but none rendered as an instance")
		return
	var multimeshes := _find_all(main.get_node("World/Tiles"), "MultiMeshInstance3D", [])
	if multimeshes.is_empty():
		_fail("no MultiMeshInstance3D under World/Tiles — belts and pipes should be instanced")
		return
	for mmi in multimeshes:
		if mmi.material_override == null or not mmi.material_override.vertex_color_use_as_albedo:
			_fail("tile MultiMesh '%s' must carry the same per-class vertex-colour material a node would" % mmi.name)
			return

	if main.get_node_or_null("Floor") == null:
		_fail("no Floor node — the factory floor didn't build")
		return

	# --- An item fed in through the real command path gets a voxel node ----
	# Loading a hopper is a click on the building and its panel's FILL
	# button since the hopper/tank configuration ticket (it was the global
	# SPACE key before that); the hopper unloads onto the belt on the *next*
	# tick, so this ticks the sim a little before looking for the rendered
	# item.
	main._config_click_at_tile(Vector2i(-1, 0))
	if not main._config_open:
		_fail("clicking the demo line's hopper did not open its configuration panel")
		return
	main._on_config_fill_pressed()
	main._close_config_panel()

	var item_nodes := 0
	for i in range(10):
		main.sim.tick()
		main._apply_dirty_chunks()
		item_nodes = main._carried_item_count()
		if item_nodes > 0:
			break
	if item_nodes < 1:
		_fail("a tomato metered out of the hopper produced no drawn item in the 3D scene")
		return
	var drawn: Dictionary = main._carried_item("belt_0_0_0")
	if drawn.is_empty() or not is_equal_approx(float(drawn["position"].y), main.BELT_SURFACE_Y):
		_fail("the first belt's first slot should carry the tomato at belt-surface height, got %s" % str(drawn))
		return

	# --- A full dock pulses its lamp -------------------------------------
	# The lamp's hue comes from the mesh's own vertex colours, which no
	# material can repaint per state, so "full" cannot be a second colour of
	# lamp. It is a time channel instead — the one channel a lit lamp and a
	# stack of crates do not already use — and it is clocked off the
	# simulation tick, so pause freezes it and a screenshot run repeats
	# (UI_UX.md §6a).
	if main._dock_lamp_lit(false, false, 0):
		_fail("an empty dock's lamp should be dark")
		return
	for tick in [0, 7, 40, 601]:
		if not main._dock_lamp_lit(true, false, tick):
			_fail("a dock holding stock should hold its lamp steady, dark at tick %d" % tick)
			return
	var lit_ticks := 0
	var dark_ticks := 0
	for tick in range(main.DOCK_FULL_PULSE_TICKS * 4):
		if main._dock_lamp_lit(true, true, tick):
			lit_ticks += 1
		else:
			dark_ticks += 1
	if lit_ticks == 0 or dark_ticks == 0:
		_fail("a full dock's lamp must blink, not hold: %d lit / %d dark ticks" % [lit_ticks, dark_ticks])
		return
	if lit_ticks != dark_ticks:
		_fail("the full-dock blink should be even, got %d lit / %d dark" % [lit_ticks, dark_ticks])
		return
	if not main._dock_lamp_lit(true, true, 0) or main._dock_lamp_lit(true, true, main.DOCK_FULL_PULSE_TICKS):
		_fail("the blink should turn over every DOCK_FULL_PULSE_TICKS ticks")
		return
	# A full dock that somehow holds nothing drawable still reads as full:
	# "full" is the sim's own verdict, not one inferred from the crates.
	if not main._dock_lamp_lit(false, true, 0):
		_fail("a full dock's lamp should blink whatever the deck is showing")
		return

	# --- and only the dock that is actually jammed pulses ------------------
	# "Full" is per dock (`DockSnapshot::full` — THIS dock refused a box off
	# its belt this tick). Each dock's lamp reads its own snapshot and
	# nothing else; the sim side is covered by `bbq_branch.rs`. The same
	# dock, the same stock, the same ticks — its own `full` is the only
	# thing that changes.
	# The demo line ends at a crusher and owns no dock, so buy one through
	# the ordinary command path and rebuild the scene around it.
	Layouts.place(main.sim, "dock", Vector2i(8, 0), "east")
	main._rebuild_world()
	if main._dock_nodes.is_empty():
		_fail("placing a dock built no lamp nodes to drive")
		return
	var dock_key: String = main._dock_nodes.keys()[0]
	var stock := [["boxed_ketchup", 1]]
	var steady_states := {}
	var blinked_states := {}
	var other_full_states := {}
	for tick in range(main.DOCK_FULL_PULSE_TICKS * 2):
		main._update_dock_stock({dock_key: {"contents": stock, "full": false}}, tick)
		steady_states[bool(main._dock_nodes[dock_key]["stocked"])] = true
		main._update_dock_stock({dock_key: {"contents": stock, "full": true}}, tick)
		blinked_states[bool(main._dock_nodes[dock_key]["stocked"])] = true
		# Another dock jamming never reaches this one's lamp.
		main._update_dock_stock({dock_key: {"contents": stock, "full": false}, "99,99": {"contents": stock, "full": true}}, tick)
		other_full_states[bool(main._dock_nodes[dock_key]["stocked"])] = true
	if other_full_states.size() != 1 or not other_full_states.has(true):
		_fail("another dock being full made this dock's lamp blink")
		return
	if steady_states.size() != 1 or not steady_states.has(true):
		_fail("a dock that is NOT backing up should hold its lamp lit, not blink")
		return
	if blinked_states.size() != 2:
		_fail("the dock named in blocked_keys should blink, it held at %s" % str(blinked_states.keys()))
		return

	# --- and the two halves of that blink have to LOOK different ----------
	# They did not. `lamp_idle` was `lamp_running`'s emission at a quarter
	# energy until 2026-09-16, and the two rendered at most 2/255 apart: the
	# blink above was correct on every count this file could check and still
	# invisible on screen. Emission was why — it contributed nothing at any
	# energy (issue #25) — so the difference was moved into ALBEDO, which did
	# reach the screen. The shader fixed emission afterwards, and BOTH
	# assertions stay: albedo is what still separates the two halves in a
	# frame where the glow pass is what the eye reads, and it is the part a
	# headless run can actually hold.
	var lamp_off: ShaderMaterial = main._lamp_idle
	var lamp_on: ShaderMaterial = main._lamp_running
	if lamp_off == null or lamp_on == null:
		_fail("the scene should hold both status-lamp materials")
		return
	if float(lamp_off.get_shader_parameter("emission_energy")) > 0.0:
		_fail("an unlit lens that still emitted would be a lamp half on")
		return
	if float(lamp_on.get_shader_parameter("emission_energy")) <= 0.0:
		_fail("a lit lens must emit — that is what `emissive` is for (UI_UX.md §4/§7)")
		return
	var off_albedo: Color = lamp_off.get_shader_parameter("albedo_scale")
	var on_albedo: Color = lamp_on.get_shader_parameter("albedo_scale")
	if off_albedo.v > on_albedo.v * 0.6:
		_fail("an off lamp must be visibly darker than a lit one, not merely dimmer in a channel that does not render: off %s vs on %s" % [
			off_albedo, on_albedo
		])
		return

	print("PRESENTATION_OK: orthographic camera, %d light(s) w/ shadows, SSAO+glow, %d voxel meshes, %d placement(s), item rendered, full dock pulses" % [
		lights.size(), meshes.size(), layout.size()
	])
	quit(0)
