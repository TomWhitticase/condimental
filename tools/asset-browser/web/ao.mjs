// The baked ambient occlusion: the arithmetic only, with no three.js in it,
// so `test-ao.mjs` can run the very code the viewer runs under plain node.
//
// The factory's WorldEnvironment runs SSAO (factory_environment.gd's
// `ssao_radius` / `ssao_intensity` / `ssao_power`) and this viewer ran
// without any, so every crease read flatter here than in the game (issue
// #49). It is baked into the mesh instead of found in screen space, for two
// reasons: three's stock `SSAOPass` reconstructs view-space depth as if the
// camera were perspective and this camera is orthographic (patching it means
// vendoring a copy of a file that is fetched, not committed), and the voxel
// grid the occlusion is *about* is already in hand at mesh-build time.
// Counting neighbours in a grid is cheaper and more exact than inferring the
// same grid back out of a depth buffer.
//
// What is measured: occupancy, box-blurred to a near-Gaussian, sampled
// AO_OFFSET voxels out along each vertex's own normal — how much solid sits
// in the hemisphere that vertex looks into, which is the quantity SSAO
// estimates, at the radius Godot uses.
//
// AO is applied to INDIRECT DIFFUSE only (in `app.js`), because that is what
// Godot's SSAO dims: `ssao_light_affect` is left at 0, so the key and fill
// land on a crease undimmed, and the sky reflection down a stainless flank —
// the thing the lighting port was calibrated on — is not touched either.

/// The game's `ssao_radius` in voxels: metres / `VOXEL_UNIT` (0.1). This is
/// the only number to move when the environment is retuned — the kernel and
/// the sample distance below follow from it, so the two cannot drift apart.
/// `calibrate-ao.py` reads `ssao_radius` out of `factory_environment.gd` and
/// refuses to derive a strength against a stale one.
export const AO_RADIUS = 4.5;

/// Three box passes make a near-Gaussian; a box radius of a third of the
/// reach puts the kernel's own support at about AO_RADIUS.
export const AO_PASSES = 3;
export const AO_BOX = Math.max(1, Math.round(AO_RADIUS / AO_PASSES));

/// Sample this far out along the face normal — one box radius, far enough
/// that the face's own voxel is off the kernel's centre and near enough that
/// a crease one voxel deep still reads.
export const AO_OFFSET = AO_BOX;

/// The game's `ssao_power`, applied the way Godot applies it: as a gamma on
/// the occlusion, before any strength. It is what makes SSAO a crease finder
/// rather than a wash — at 1.6 a fully buried vertex keeps its darkening
/// while a vertex a fifth occluded loses two thirds of its. Porting the reach
/// and the strength but not this one was a shape mismatch, not a scale one,
/// and no strength could have fixed it. `calibrate-ao.py` reads it out of
/// `factory_environment.gd` alongside the radius.
export const AO_POWER = 1.6;

// How dark the deepest crease goes: the ONE knob, and not a number anybody
// chose by eye. `calibrate-ao.py` renders the game's vat fixture with
// `factory_environment.gd`'s SSAO on and off, renders this browser on the
// same model with the strength on and off, and divides one measured
// darkening by the other. Re-derive it with a single command when the game's
// SSAO moves:
//
//     tools/asset-browser/calibrate-ao.py --write
//
// It is so much smaller than one because the browser is meant to show what
// the game shows, so that nobody can approve a form that only reads under
// browser-only occlusion.
export const AO_STRENGTH = 5.067;

/// One box-blur pass along one axis of a 3D field, running-sum so the cost is
/// the field's size and not the kernel's. Everything outside the field counts
/// as empty, which is why the grid below is padded: a model's own edge must
/// not read as occluded by the end of the array.
export function blurAxis(src, dst, dims, axis, radius) {
  const strides = [1, dims[0], dims[0] * dims[1]];
  const stride = strides[axis];
  const n = dims[axis];
  const [oa, ob] = [0, 1, 2].filter((a) => a !== axis);
  const norm = 1 / (2 * radius + 1);
  for (let b = 0; b < dims[ob]; b += 1) {
    for (let a = 0; a < dims[oa]; a += 1) {
      const base = a * strides[oa] + b * strides[ob];
      let sum = 0;
      for (let i = 0; i <= radius && i < n; i += 1) sum += src[base + i * stride];
      for (let i = 0; i < n; i += 1) {
        dst[base + i * stride] = sum * norm;
        const add = i + radius + 1;
        const drop = i - radius;
        if (add < n) sum += src[base + add * stride];
        if (drop >= 0) sum -= src[base + drop * stride];
      }
    }
  }
}

/// What the same kernel reads AO_OFFSET voxels off the face of an infinite
/// flat wall. It is not zero — a kernel centred outside a half-space still
/// catches a fifth of it — and subtracting it is what keeps an open flank at
/// exactly its measured colour instead of a fifth darker. The blur is
/// separable, so a half-space is a 1-D step through it and this is that step,
/// blurred by the very same passes rather than by a constant someone has to
/// keep in step with them.
///
/// `wall` is the first EMPTY cell of the step, which is the cell `ao()` must
/// measure from on both signs of the normal for the subtraction to cancel.
export const AO_BASELINE = (() => {
  const n = 4 * (AO_PASSES * AO_BOX + AO_OFFSET + 2);
  const wall = Math.floor(n / 2);
  let src = new Float32Array(n);
  let dst = new Float32Array(n);
  for (let i = 0; i < wall; i += 1) src[i] = 1;
  for (let p = 0; p < AO_PASSES; p += 1) {
    blurAxis(src, dst, [n, 1, 1], 0, AO_BOX);
    [src, dst] = [dst, src];
  }
  return src[wall + AO_OFFSET];
})();

/// The occlusion field of one model, over ALL of its classes at once: a
/// stainless fitting darkens the painted body it is bolted to, which it
/// cannot do if each class is blurred alone. (A class hidden from the chips
/// still casts its occlusion, the same way hiding it does not re-cull the
/// faces behind it.)
export function aoField(classes, size) {
  const pad = AO_PASSES * AO_BOX + AO_OFFSET + 1;
  const dims = [size[0] + 2 * pad, size[1] + 2 * pad, size[2] + 2 * pad];
  const idx = (x, y, z) => (x + pad) + (y + pad) * dims[0] + (z + pad) * dims[0] * dims[1];
  let src = new Float32Array(dims[0] * dims[1] * dims[2]);
  let dst = new Float32Array(src.length);
  for (const part of Object.values(classes)) {
    const v = part.voxels;
    for (let i = 0; i < v.length; i += 4) src[idx(v[i], v[i + 1], v[i + 2])] = 1;
  }
  for (let axis = 0; axis < 3; axis += 1) {
    for (let p = 0; p < AO_PASSES; p += 1) {
      blurAxis(src, dst, dims, axis, AO_BOX);
      [src, dst] = [dst, src];
    }
  }
  const field = src;

  /// The openness at one vertex of one face: the field AO_OFFSET voxels out
  /// along the face's normal, less what open wall reads there. 1 is open sky,
  /// 0 is buried. Geometry only — how dark that is made to look is
  /// AO_STRENGTH, at draw time, so it stays one knob and a measurable one.
  ///
  /// The sample is counted off the first EMPTY cell on BOTH signs of the
  /// normal. A corner index is a lattice point between two cells: on a
  /// positive normal `c` is already the empty cell outside the face, on a
  /// negative one `c` is still the last solid cell and the empty one is
  /// `c - 1`. Measuring from `c` on both signs put the three negative
  /// normals one voxel nearer the surface than AO_BASELINE was derived at,
  /// so the subtraction under-cancelled and a perfectly open flank came back
  /// 2.9% dark — on half the faces of every model (found in review,
  /// 2026-09-20; `test-ao.mjs` is the assertion that it stays fixed).
  return function ao(x, y, z, dx, dy, dz) {
    const sx = (dx < 0 ? x - 1 : x) + dx * AO_OFFSET;
    const sy = (dy < 0 ? y - 1 : y) + dy * AO_OFFSET;
    const sz = (dz < 0 ? z - 1 : z) + dz * AO_OFFSET;
    if (sx < -pad || sy < -pad || sz < -pad) return 1;
    if (sx >= size[0] + pad || sy >= size[1] + pad || sz >= size[2] + pad) return 1;
    const occ = (field[idx(sx, sy, sz)] - AO_BASELINE) / (1 - AO_BASELINE);
    return 1 - Math.pow(Math.min(1, Math.max(0, occ)), AO_POWER);
  };
}
