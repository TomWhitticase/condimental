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

## "matte"/"metal"/"emissive"/"liquid" -> StandardMaterial3D.
var class_materials: Dictionary = {}
## The two status-lamp variants; see `load_materials`.
var lamp_idle: StandardMaterial3D
var lamp_running: StandardMaterial3D
## Asset name -> Array of the material classes `voxgen` split it into.
var asset_classes: Dictionary = {}


## Rebuilds one `StandardMaterial3D` per material class from
## `voxels/materials.json`, which `voxgen` exports straight out of
## `assets/voxels/palette.ron`. The numbers deliberately are NOT written down
## here: palette.ron stays the single source of truth for colour and material
## (UI_UX.md §2a point 5), and GDScript just consumes it.
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
		class_materials[class_name_] = make_material(
			float(def.get("metallic", 0.0)),
			float(def.get("roughness", 1.0)),
			float(def.get("emission_energy", 0.0))
		)

	# Two variants of the emissive material so a machine's status lamps can
	# actually report state (UI_UX.md §4/§7: saturated colour reserved for
	# meaning, never decoration). Same mesh, swapped override material.
	#
	# The OFF variant is dark ALBEDO, not dimmed emission. It was a quarter of
	# the lit emission energy until 2026-09-16, and the two were
	# indistinguishable in a rendered frame — at most 2/255 between them, so
	# the dock's full-pool blink (`main.gd`'s `_dock_lamp_lit`) was a signal
	# nobody could see. Emission turns out to contribute NOTHING here at any
	# energy: `make_material`'s note and issue #25 say why. Albedo is the
	# channel that moves, and it is the one the palette itself reaches for —
	# the `status_*_off` entries in `palette.ron` are the same hues taken
	# down to a dark matte, "dark enough that a dead lens does not compete
	# with a live one, light enough not to vanish", and `LAMP_OFF_ALBEDO` is
	# that idea applied to a mesh whose colour cannot be repainted per state.
	var lamp_energy: float = float(table.get("emissive", defaults["emissive"]).get("emission_energy", 2.6))
	lamp_idle = make_material(0.0, 0.6, 0.0)
	lamp_idle.albedo_color = Color(LAMP_OFF_ALBEDO, LAMP_OFF_ALBEDO, LAMP_OFF_ALBEDO)
	lamp_running = make_material(0.0, 0.45, lamp_energy)

	var index = read_json(VOX_DIR + "index.json")
	if index is Dictionary:
		asset_classes = index


static func make_material(metallic: float, roughness: float, emission_energy: float) -> StandardMaterial3D:
	var mat := StandardMaterial3D.new()
	# Voxel meshes carry their colour per vertex, straight from the .vox
	# palette — there are no textures anywhere in this pipeline.
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mat.metallic = metallic
	mat.roughness = roughness
	if emission_energy > 0.0:
		mat.emission_enabled = true
		mat.emission = Color(1, 1, 1)
		# MULTIPLY was chosen so that an emissive mesh holding several
		# differently coloured lamps kept each one's hue, where a flat
		# additive emission washes them all to the same white.
		#
		# It does not do that, and NOTHING HERE EMITS — issue #25. Godot's
		# MULTIPLY multiplies by the emission TEXTURE, not by albedo and not
		# by the mesh's vertex colours, and this pipeline has no textures at
		# all, so the product is zero: raising `emission_energy` to 50
		# renders a byte-identical frame. Left as it stands on purpose. ADD
		# would light every lamp white, which is the failure the line above
		# set out to avoid; emission that follows each voxel's own colour
		# needs a spatial shader and a relight of every machine, and that
		# wants its own review pass rather than a rider on a bug fix.
		#
		# Until then, a lamp's state must be told by ALBEDO, the channel that
		# does reach the screen — see `LAMP_OFF_ALBEDO`.
		mat.emission_operator = BaseMaterial3D.EMISSION_OP_MULTIPLY
		mat.emission_energy_multiplier = emission_energy
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
