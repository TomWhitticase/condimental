## The voxel asset loader: `res://voxels/` -> meshes and materials.
##
## Lifted out of `main.gd` unchanged when the title screen started standing
## real machines behind the main menu (`title_scene.gd`). Both scenes have to
## build a voxel model exactly the same way — the same per-class split, the
## same materials read out of `palette.ron` via `voxels/materials.json` — and
## a second copy of this in the menu is precisely the drift UI_UX.md §2a
## point 5 forbids: the menu would be lit by one palette and the game by
## another, and nothing would say which was right.
##
## A `RefCounted` rather than an autoload: each scene owns its own loader and
## its own materials, so a menu that is freed takes its materials with it.
## Reached by `preload`, never `class_name` (godot_project/CLAUDE.md).
extends RefCounted

const VOX_DIR := "res://voxels/"

## What `lamp_idle` multiplies a status lamp's own colour by to read as OFF.
## Multiplies rather than replaces because the hue lives in the mesh's vertex
## colours, not in the material: one number has to take amber, green and red
## down together and leave each still recognisable as its own lens.
##
## Set by rendering, the way `palette.ron` set its `status_*_off` trio — and
## against that trio, which is the same judgement made where the colour could
## be chosen outright: `status_amber` #FFB020 against `status_amber_off`
## #5C4212 is a little over a third, and so is red's and green's.
const LAMP_OFF_ALBEDO := 0.38

## "matte"/"metal"/"emissive"/"liquid" -> Material. `emissive` is the one
## `ShaderMaterial` (`voxel_emissive.gdshader`); the rest are
## `StandardMaterial3D` and stay that way — see `make_emissive_material`.
var class_materials: Dictionary = {}
## The two status-lamp variants; see `load_materials`.
var lamp_idle: ShaderMaterial
var lamp_running: ShaderMaterial
## Asset name -> Array of the material classes `voxgen` split it into.
var asset_classes: Dictionary = {}


## Rebuilds one material per class from `voxels/materials.json`, which
## `voxgen` exports straight out of `assets/voxels/palette.ron`. The numbers
## deliberately are NOT written down here: palette.ron stays the single source
## of truth for colour and material (UI_UX.md §2a point 5), and GDScript just
## consumes it. A class that emits gets a `ShaderMaterial`, everything else a
## `StandardMaterial3D` — `make_emissive_material` says why.
func load_materials() -> void:
	var defaults := {
		"matte": {"metallic": 0.0, "roughness": 0.95, "emission_energy": 0.0},
		"metal": {"metallic": 0.9, "roughness": 0.18, "emission_energy": 0.0},
		"emissive": {"metallic": 0.0, "roughness": 0.55, "emission_energy": 2.6},
		"liquid": {"metallic": 0.0, "roughness": 0.14, "emission_energy": 0.0},
	}
	var table: Dictionary = defaults
	var parsed = read_json(VOX_DIR + "materials.json")
	if parsed is Dictionary:
		table = parsed
	else:
		push_warning("voxels/materials.json missing — falling back to built-in material values")

	for class_name_ in table.keys():
		var def: Dictionary = table[class_name_]
		var metallic := float(def.get("metallic", 0.0))
		var roughness := float(def.get("roughness", 1.0))
		var energy := float(def.get("emission_energy", 0.0))
		class_materials[class_name_] = (
			make_emissive_material(metallic, roughness, energy, Color(1, 1, 1))
			if energy > 0.0
			else make_material(metallic, roughness)
		)

	# Two variants of the emissive material so a machine's status lamps can
	# actually report state (UI_UX.md §4/§7: saturated colour reserved for
	# meaning, never decoration). Same mesh, swapped override material.
	#
	# The OFF variant darkens ALBEDO and drops emission to zero, and it keeps
	# doing BOTH now that emission works. Albedo is not redundant: it is what
	# made the dock's full-pool blink visible when emission contributed nothing
	# (2026-09-16 — the two variants rendered at most 2/255 apart while the
	# only difference between them was energy), and it is what keeps a dead
	# lens reading as a dead lens in a frame where the glow pass is doing the
	# talking. It is also the channel `palette.ron` itself reaches for: the
	# `status_*_off` entries are the same hues taken down to a dark matte,
	# "dark enough that a dead lens does not compete with a live one, light
	# enough not to vanish", and `LAMP_OFF_ALBEDO` is that idea applied to a
	# mesh whose colour cannot be repainted per state.
	var lamp_energy: float = float(table.get("emissive", defaults["emissive"]).get("emission_energy", 2.6))
	lamp_idle = make_emissive_material(
		0.0, 0.6, 0.0, Color(LAMP_OFF_ALBEDO, LAMP_OFF_ALBEDO, LAMP_OFF_ALBEDO)
	)
	lamp_running = make_emissive_material(0.0, 0.45, lamp_energy, Color(1, 1, 1))

	var index = read_json(VOX_DIR + "index.json")
	if index is Dictionary:
		asset_classes = index


static func make_material(metallic: float, roughness: float) -> StandardMaterial3D:
	var mat := StandardMaterial3D.new()
	# Voxel meshes carry their colour per vertex, straight from the .vox
	# palette — there are no textures anywhere in this pipeline.
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mat.metallic = metallic
	mat.roughness = roughness
	return mat


## The `emissive` class, and only it: a lamp lit in its own colour.
##
## `StandardMaterial3D` cannot do that (issue #25, fixed 2026-09-16). Its
## emission comes from a flat colour and an emission TEXTURE, never from the
## vertex colours that are this pipeline's only colour source, so the previous
## `EMISSION_OP_MULTIPLY` against an absent texture multiplied by nothing:
## every lamp in the factory was ordinary painted amber at any energy, and
## `emission_energy: 50` rendered a byte-identical frame. The full reasoning —
## and why `EMISSION_OP_ADD` is not the answer either — is at the top of
## `voxel_emissive.gdshader`.
##
## `albedo_scale` multiplies the lens colour in the ALBEDO channel alone, which
## is how `lamp_idle` reads as OFF while staying the same hue of lens.
static func make_emissive_material(
	metallic: float, roughness: float, emission_energy: float, albedo_scale: Color
) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = preload("res://voxel_emissive.gdshader")
	mat.set_shader_parameter("metallic", metallic)
	mat.set_shader_parameter("roughness", roughness)
	mat.set_shader_parameter("emission_energy", emission_energy)
	mat.set_shader_parameter("albedo_scale", albedo_scale)
	return mat


static func read_json(path: String):
	if not FileAccess.file_exists(path):
		return null
	var text := FileAccess.get_file_as_string(path)
	if text.is_empty():
		return null
	return JSON.parse_string(text)


## Builds one asset as a `Node3D` holding a `MeshInstance3D` per material
## class it uses. Returns an empty node (with a warning) rather than failing
## if the models haven't been staged — `cargo xtask` is what puts them in
## res://voxels/, and a missing asset should not take down the whole scene.
func asset_node(asset: String) -> Node3D:
	var root := Node3D.new()
	root.name = asset

	var built := 0
	for class_name_ in asset_class_names(asset):
		var mesh := class_mesh_for(asset, class_name_)
		if mesh == null:
			continue
		var mi := MeshInstance3D.new()
		mi.name = String(class_name_)
		mi.mesh = mesh
		# Only when there IS one: assigning a null override makes the headless
		# dummy renderer log "Parameter 'material' is null" when the node is
		# later freed, which is noise in every test log.
		var class_material = class_materials.get(class_name_, null)
		if class_material != null:
			mi.set_surface_override_material(0, class_material)
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		root.add_child(mi)
		built += 1

	if built == 0:
		push_warning("no voxel meshes for '%s' under %s — run `cargo xtask`" % [asset, VOX_DIR])
	return root


func mesh_for(asset: String) -> Mesh:
	var path := VOX_DIR + asset + ".vox"
	if not ResourceLoader.exists(path):
		return null
	return load(path) as Mesh


## The material classes an asset was split into by `voxgen` (from
## `index.json`), or every class when the index is missing — a class with no
## file simply contributes no mesh.
func asset_class_names(asset: String) -> Array:
	return asset_classes.get(asset, ["matte", "metal", "emissive", "liquid"])


## One asset's per-class mesh (`name.<class>.vox`, UI_UX.md §2b point 2), or
## null when that class was not written for it or failed to import.
func class_mesh_for(asset: String, class_name_: String) -> Mesh:
	var path := "%s%s.%s.vox" % [VOX_DIR, asset, class_name_]
	if not ResourceLoader.exists(path):
		return null
	return load(path) as Mesh
