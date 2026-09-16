// The `emissive` material class: a status lamp glows in ITS OWN colour.
//
// WHY A SHADER AND NOT A StandardMaterial3D (issue #25). Every other class in
// this pipeline is a plain `StandardMaterial3D` and should stay one — see
// `voxel_assets.gd`'s `make_material`. Emission is the one thing that class
// cannot do here. A voxel mesh carries its colour PER VERTEX, straight out of
// `palette.ron`; there are no textures anywhere. `StandardMaterial3D` offers
// exactly three emission sources — a flat colour, an emission TEXTURE, and
// that texture multiplied or added to the flat colour — and none of them is
// the vertex colour. The old code asked for `EMISSION_OP_MULTIPLY` against an
// absent texture, so the product was zero and nothing in the factory emitted
// at any energy. `EMISSION_OP_ADD` does light up, and lights every lamp the
// same white: the crusher carries `status_green` at one end of a single
// `emissive` submesh and `status_amber` at the other, so one flat additive
// colour washes both to the same signal. `EMISSION = COLOR.rgb` is the only
// answer that keeps green reading as green, and it takes a shader.
//
// The mesh's colours are sRGB (the `.vox` palette is written in hex), which is
// what `vertex_color_is_srgb` handles for the other classes. Here the
// conversion is explicit and the SAME linear value feeds both channels: a lamp
// emits the colour it is painted, never a second hue laid over the first.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_burley, specular_schlick_ggx;

uniform float metallic : hint_range(0.0, 1.0) = 0.0;
uniform float roughness : hint_range(0.0, 1.0) = 0.55;
/// Multiplied into ALBEDO only, never into EMISSION — this is the channel the
/// OFF lamp variant darkens (`voxel_assets.gd`'s `LAMP_OFF_ALBEDO`). Declared
/// `source_color` so a `Color` set from GDScript is converted to linear on the
/// way in, exactly as `StandardMaterial3D.albedo_color` is.
uniform vec4 albedo_scale : source_color = vec4(1.0, 1.0, 1.0, 1.0);
/// `palette.ron`'s `emission_energy` for this class, via `materials.json`.
/// Zero on the OFF variant: an unlit lens that still emitted would be a lamp
/// half on.
uniform float emission_energy = 0.0;

// Godot's own sRGB -> linear curve, the one `vertex_color_is_srgb` applies.
// Spelled out rather than taken from a built-in so this reads identically on
// any 4.x the project is pinned to.
vec3 vox_srgb_to_linear(vec3 c) {
	return mix(
		pow((c + vec3(0.055)) * (1.0 / 1.055), vec3(2.4)),
		c * (1.0 / 12.92),
		lessThan(c, vec3(0.04045))
	);
}

void fragment() {
	vec3 lens = vox_srgb_to_linear(COLOR.rgb);
	ALBEDO = lens * albedo_scale.rgb;
	METALLIC = metallic;
	ROUGHNESS = roughness;
	EMISSION = lens * emission_energy;
}
