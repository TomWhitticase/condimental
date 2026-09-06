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

	# --- Camera: fixed and orthographic (UI_UX.md §1) ---------------------
	var cameras := _find_all(main, "Camera3D", [])
	if cameras.size() != 1:
		_fail("expected exactly 1 Camera3D, found %d" % cameras.size())
		return
	var camera: Camera3D = cameras[0]
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
		var mat: StandardMaterial3D = mi.get_surface_override_material(0)
		if mat == null:
			_fail("MeshInstance3D '%s' has no material override" % mi.name)
			return
		if not mat.vertex_color_use_as_albedo:
			_fail("'%s' must use vertex colours as albedo — voxel palettes are the only colour source" % mi.name)
			return
		match String(mi.name):
			"matte":
				matte_roughness = mat.roughness
			"metal":
				metal_roughness = mat.roughness
				metal_metallic = mat.metallic
			"emissive":
				if not mat.emission_enabled:
					_fail("the 'emissive' material must actually emit (UI_UX.md §4/§7)")
					return

	if metal_metallic < 0.0 or matte_roughness < 0.0:
		_fail("scene is missing a metal or matte voxel mesh — the material split didn't survive import")
		return
	if metal_metallic <= 0.5:
		_fail("metal material is barely metallic (%f) — UI_UX.md §4's exception is the point" % metal_metallic)
		return
	if metal_roughness >= matte_roughness:
		_fail("metal (rough %f) is not shinier than matte (rough %f) — UI_UX.md §4" % [metal_roughness, matte_roughness])
		return

	# --- The chain rendered matches the chain the simulation reports ------
	var layout: Array = JSON.parse_string(main.sim.chain_layout_json())
	if layout.is_empty():
		_fail("chain_layout_json returned no stages")
		return
	for stage in layout:
		var node_name := "machine_%s" % stage["recipe"]
		if main.get_node_or_null("Chain/" + node_name) == null:
			_fail("no rendered machine for stage '%s' — the scene is not following the sim's layout" % stage["recipe"])
			return

	if main.get_node_or_null("Floor") == null:
		_fail("no Floor node — the factory floor didn't build")
		return

	# --- An item spawned through the real command path gets a voxel node --
	var key_event := InputEventKey.new()
	key_event.keycode = KEY_SPACE
	key_event.pressed = true
	main._unhandled_input(key_event)

	var item_nodes := 0
	for child in main.get_children():
		if String(child.name).begins_with("item_"):
			item_nodes += 1
	if item_nodes < 1:
		_fail("spawning a tomato produced no item node in the 3D scene")
		return

	print("PRESENTATION_OK: orthographic camera, %d light(s) w/ shadows, SSAO+glow, %d voxel meshes, %d stage(s), item rendered" % [
		lights.size(), meshes.size(), layout.size()
	])
	quit(0)
