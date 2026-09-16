// The backdrop: the inside of a factory hall far bigger than anything the
// player will ever build (docs/UI_UX.md §3 — a grounded industrial food
// factory, not a slab of concrete under an open sky).
//
// It is a SKY shader, so every surface below is at infinity and has no
// parallax: the hall is the same size however far the player walks or flies.
// That is the point. Real geometry would need a finite building the camera
// could reach the wall of, and would occlude the isometric view the moment
// the floor grew; an infinite hall reads as "massive" precisely because it
// never resolves into a room you could pace out.
//
// The shape is a box centred on the viewer — roof overhead, four clad walls
// at `hall_half_width`, slab below — intersected analytically against
// EYEDIR. The structural grid (`bay_pitch`) is shared by the roof trusses
// and the wall columns, in both x and z, so a column always lands under a
// truss: that alignment is what makes it read as a building rather than as
// wallpaper.
//
// Colours come in as uniforms rather than literals so they can be named
// against assets/voxels/palette.ron in main.gd's `_build_environment`, and
// so Godot does the sRGB->linear conversion (`source_color`) instead of this
// file carrying hand-converted numbers.
shader_type sky;

// --- The hall, in world units (main.gd's TILE is 3.2) ---------------------
uniform float hall_half_width = 132.0; // wall distance: ~41 tiles each way
uniform float roof_height = 26.0;      // underside of the roof deck
uniform float floor_drop = 5.0;        // slab, about one eye-height down
uniform float bay_pitch = 26.0;        // structural bay: one truss, one column
uniform float haze_density = 0.0052;   // air; what actually sells the distance

// --- Surfaces (palette.ron names in main.gd) ------------------------------
uniform vec3 deck_color : source_color;
uniform vec3 truss_color : source_color;
uniform vec3 purlin_color : source_color;
uniform vec3 cladding_color : source_color;
uniform vec3 plinth_color : source_color;
uniform vec3 slab_color : source_color;
uniform vec3 hazard_color : source_color;
uniform vec3 lamp_color : source_color;
uniform vec3 window_color : source_color;
uniform vec3 haze_color : source_color;
uniform float lamp_energy = 4.5;   // over glow_hdr_threshold, so lamps bloom
uniform float window_energy = 1.35; // daylight through dirty glazing

// A repeating soft-edged line: 1 inside a stripe of half-width `half_width`
// centred on every multiple of `pitch`. `soft` is the analytic antialiasing
// width — sky shaders have no derivatives, so the caller passes a footprint
// derived from how far away the surface it is shading is.
float stripe(float x, float pitch, float half_width, float soft) {
	float d = abs(fract(x / pitch + 0.5) - 0.5) * pitch;
	return 1.0 - smoothstep(half_width - soft, half_width + soft, d);
}

// One soft-edged horizontal band between `lo` and `hi`.
float band(float x, float lo, float hi, float soft) {
	return smoothstep(lo - soft, lo + soft, x) * (1.0 - smoothstep(hi - soft, hi + soft, x));
}

// A rectangle repeated once per `pitch` cell, for light fittings.
float cell_rect(vec2 p, float pitch, vec2 half_size, float soft) {
	vec2 d = abs(fract(p / pitch + 0.5) - 0.5) * pitch;
	return (1.0 - smoothstep(half_size.x - soft, half_size.x + soft, d.x))
		* (1.0 - smoothstep(half_size.y - soft, half_size.y + soft, d.y));
}

// Deterministic per-bay noise for the distant plant silhouette. Never
// time- or frame-dependent: a review screenshot has to be repeatable
// (godot_project/CLAUDE.md, "visual randomness is derived from an index").
float hash11(float n) {
	return fract(sin(n * 127.1) * 43758.5453);
}

// The roof, seen from underneath: profiled steel deck on purlins, primary
// trusses on the bay grid, skylight ribbons down the middle of each bay and
// high-bay fittings hung between them.
void roof_surface(vec2 p, float soft, out vec3 albedo, out vec3 emission) {
	albedo = deck_color;
	emission = vec3(0.0);

	// Trapezoidal deck profile — fine ribs, just enough to catch the eye.
	albedo = mix(albedo, deck_color * 0.72, stripe(p.y, 1.7, 0.16, soft) * 0.8);

	// Purlins across the bays, then the primary trusses on the grid itself.
	albedo = mix(albedo, purlin_color, stripe(p.y, bay_pitch * 0.25, 0.34, soft) * 0.75);
	float truss = max(stripe(p.x, bay_pitch, 1.7, soft), stripe(p.y, bay_pitch, 1.3, soft));
	albedo = mix(albedo, truss_color, truss);
	// Top chord: a bright line down the centre of each truss, which is what
	// makes the perspective converge legibly towards the zenith.
	float chord = max(stripe(p.x, bay_pitch, 0.4, soft), stripe(p.y, bay_pitch, 0.3, soft));
	albedo = mix(albedo, truss_color * 1.45, chord);

	// Skylight ribbons: continuous down the middle of every bay, broken by a
	// mullion at each cross-truss.
	float ribbon = stripe(p.x + bay_pitch * 0.5, bay_pitch, 2.6, soft);
	float mullion = stripe(p.y, bay_pitch * 0.5, 0.35, soft);
	float glazing = ribbon * (1.0 - mullion);
	albedo = mix(albedo, window_color, glazing);
	emission += window_color * window_energy * glazing;

	// High-bay fittings, hung in pairs between the skylights.
	float fitting = cell_rect(p + vec2(bay_pitch * 0.5, bay_pitch * 0.5), bay_pitch * 0.5,
		vec2(1.9, 0.75), soft);
	albedo = mix(albedo, lamp_color, fitting);
	emission += lamp_color * lamp_energy * fitting;
}

// A wall: dark plinth and hazard band at the bottom, profiled cladding,
// a pipe-rack gantry, clerestory glazing high up, columns on the bay grid,
// and a silhouette of other plant standing in front of it.
void wall_surface(float u, float v, float soft, out vec3 albedo, out vec3 emission) {
	albedo = cladding_color;
	emission = vec3(0.0);

	// Vertical cladding profile.
	albedo = mix(albedo, cladding_color * 0.78, stripe(u, 1.3, 0.3, soft) * 0.85);

	// Clerestory glazing, panelled, stopping short of each column.
	float glass = band(v, roof_height * 0.60, roof_height * 0.84, soft)
		* (1.0 - stripe(u, 4.2, 0.45, soft))
		* (1.0 - stripe(u, bay_pitch, 2.2, soft));
	albedo = mix(albedo, window_color, glass);
	emission += window_color * window_energy * glass;

	// Pipe-rack gantry running the length of the wall, with a handrail over
	// it — the horizontal that gives the wall its scale.
	float gantry = band(v, roof_height * 0.30, roof_height * 0.36, soft);
	albedo = mix(albedo, truss_color * 0.55, gantry);
	albedo = mix(albedo, truss_color * 1.2, band(v, roof_height * 0.36, roof_height * 0.375, soft));

	// Columns, on the same grid as the roof trusses.
	albedo = mix(albedo, truss_color * 0.8, stripe(u, bay_pitch, 1.1, soft));

	// Plinth and the hazard band painted along its top edge.
	albedo = mix(albedo, plinth_color, 1.0 - smoothstep(2.2 - soft, 2.2 + soft, v));
	float hazard = band(v, 1.5, 2.2, soft) * stripe(u, 2.4, 0.6, soft);
	albedo = mix(albedo, hazard_color, hazard);

	// Other plant, far off down the hall: block silhouettes of a height that
	// varies per bay, with the odd status light burning on one. Drawn last,
	// over the plinth, so the line stands ON the floor instead of appearing
	// to hover above a band of wall.
	float unit = floor(u / 7.0);
	float present = step(0.22, hash11(unit + 5.0)); // gangways between the blocks
	float height = 2.0 + hash11(unit) * 9.5;
	float mass = present
		* (1.0 - smoothstep(height - soft, height + soft, v))
		* smoothstep(-0.4 - soft, -0.4 + soft, v);
	albedo = mix(albedo, mix(cladding_color, truss_color, 0.25) * 0.62, mass);
	float lit = step(0.72, hash11(unit + 31.0));
	float lamp = lit * mass * cell_rect(vec2(u, v - height + 1.1), 7.0, vec2(0.26, 0.26), soft);
	emission += lamp_color * lamp_energy * 0.4 * lamp;
}

// Distance along the ray to whichever of a facing pair of planes the ray is
// heading towards. A ray running parallel to the pair never meets it, and a
// near-parallel one must not be allowed to come back as a huge NEGATIVE
// distance: that is behind the viewer, it wins the min() below, and the
// exp() in the haze then returns an infinity that spreads through the whole
// radiance map — which showed up as every lit surface in the world turning
// black, not as anything wrong with the sky.
float plane_distance(float dir, float ahead, float behind) {
	if (abs(dir) < 1e-4) {
		return 1e9;
	}
	return (dir > 0.0 ? ahead : behind) / dir;
}

void sky() {
	vec3 d = EYEDIR;
	float tx = plane_distance(d.x, hall_half_width, -hall_half_width);
	float tz = plane_distance(d.z, hall_half_width, -hall_half_width);
	float ty = plane_distance(d.y, roof_height, -floor_drop);
	float t = min(min(tx, tz), ty);

	vec3 p = d * t;
	// Distance-proportional filter width, standing in for the derivatives a
	// sky shader cannot take.
	float soft = clamp(t * 0.0032, 0.045, 2.2);

	vec3 albedo;
	vec3 emission;
	if (t == ty && d.y > 0.0) {
		roof_surface(p.xz, soft, albedo, emission);
	} else if (t == ty) {
		// The slab, below the world's own floor mesh: plain, with the bay
		// grid picked out in expansion joints.
		albedo = slab_color;
		albedo = mix(albedo, slab_color * 0.8,
			max(stripe(p.x, bay_pitch, 0.35, soft), stripe(p.z, bay_pitch, 0.35, soft)));
		emission = vec3(0.0);
	} else if (t == tx) {
		wall_surface(p.z, p.y, soft, albedo, emission);
	} else {
		wall_surface(p.x, p.y, soft, albedo, emission);
	}

	// Air: the haze is doing most of the work of making the hall feel deep,
	// so it is keyed to the true distance to the surface rather than to
	// height. Grazing rays run for hundreds of units and fade out entirely.
	float fog = clamp(1.0 - exp(-t * haze_density), 0.0, 1.0);
	COLOR = mix(albedo + emission, haze_color, fog);
}
