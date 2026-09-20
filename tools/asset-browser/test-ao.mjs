#!/usr/bin/env node
// Asserts the baked occlusion of `web/ao.mjs` — the same module the viewer
// imports — on shapes whose answer is known without rendering anything:
//
//     node tools/asset-browser/test-ao.mjs
//
// The regression it exists for shipped once (2026-09-20): the sample point
// was counted off the corner index, which is the first empty cell on a
// positive normal but the last solid one on a negative normal, so three of
// the six face normals darkened a perfectly open flank by 2.9% — exactly the
// browser-only shading this viewer must never invent. An open flank reading
// open is not something to check by eye on a vat: it is two lines of
// arithmetic on a cube.

import { AO_BASELINE, AO_OFFSET, AO_RADIUS, aoField } from './web/ao.mjs';

let failures = 0;

function check(name, ok, detail) {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? `  ${detail}` : ''}`);
  if (!ok) failures += 1;
}

/// A solid box of voxels, as /api/model hands one over: x, y, z, palette index.
function box(x0, y0, z0, x1, y1, z1) {
  const v = [];
  for (let z = z0; z < z1; z += 1) {
    for (let y = y0; y < y1; y += 1) {
      for (let x = x0; x < x1; x += 1) v.push(x, y, z, 1);
    }
  }
  return v;
}

const DIRS = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

// ---------------------------------------------------------------- open sky

// A solid cube: every one of its six outer faces is a flat flank with
// nothing at all in front of it, so every one must read fully open.
const N = 40;
const cube = aoField({ matte: { voxels: box(0, 0, 0, N, N, N) } }, [N, N, N]);

// The face plane on a positive normal is the lattice index N, on a negative
// one it is 0 — the two sides of the same cube.
const planeOf = (d) => (d > 0 ? N : 0);

for (const [dx, dy, dz] of DIRS) {
  let worst = 0;
  let where = null;
  // Every vertex of the face, corners and edges included: a vertex on the
  // cube's edge sees the neighbouring wall, but only through the tangential
  // axes, and a flat flank of this size has interior vertices a whole
  // AO_RADIUS clear of any edge. Measure those.
  for (let a = AO_RADIUS + 1; a < N - AO_RADIUS; a += 1) {
    for (let b = AO_RADIUS + 1; b < N - AO_RADIUS; b += 1) {
      const p = dx ? [planeOf(dx), a, b] : dy ? [a, planeOf(dy), b] : [a, b, planeOf(dz)];
      const dark = 1 - cube(p[0], p[1], p[2], dx, dy, dz);
      if (dark > worst) { worst = dark; where = p; }
    }
  }
  check(`open flank on normal [${dx},${dy},${dz}] is untouched`,
    worst < 1e-9, `worst darkening ${(worst * 100).toFixed(4)}% at ${where}`);
}

// Stronger than the above, because it holds at the cube's edges too: the
// cube is mirror-symmetric, so a face and its opposite must read the same
// occlusion at mirrored vertices. This is the property the sample-base bug
// broke, and it breaks it whatever AO_BASELINE happens to be.
for (const axis of [0, 1, 2]) {
  const pos = [0, 0, 0];
  const neg = [0, 0, 0];
  pos[axis] = 1;
  neg[axis] = -1;
  let worst = 0;
  for (let a = 0; a <= N; a += 1) {
    for (let b = 0; b <= N; b += 1) {
      const at = (plane) => {
        const p = [a, b];
        p.splice(axis, 0, plane);
        return p;
      };
      const o = cube(...at(N), ...pos);
      const u = cube(...at(0), ...neg);
      worst = Math.max(worst, Math.abs(o - u));
    }
  }
  check(`axis ${'xyz'[axis]} reads its two faces alike`,
    worst < 1e-9, `worst |+face - -face| ${worst.toExponential(2)}`);
}

// A single voxel alone in space occludes nothing, on any normal.
{
  const lone = aoField({ matte: { voxels: box(8, 8, 8, 9, 9, 9) } }, [16, 16, 16]);
  let worst = 0;
  for (const [dx, dy, dz] of DIRS) {
    for (const [cx, cy, cz] of [[8, 8, 8], [9, 9, 9], [8, 9, 8]]) {
      worst = Math.max(worst, 1 - lone(cx, cy, cz, dx, dy, dz));
    }
  }
  check('a lone voxel shades nothing', worst < 0.02,
    `worst darkening ${(worst * 100).toFixed(4)}%`);
}

// ------------------------------------------------------------- real creases

// Two slabs meeting at a right angle. The inner corner is the shape the bake
// exists for; if this stops darkening, the pass has been turned into a
// constant and the assertions above would still pass.
{
  const M = 48;
  const ell = aoField({
    matte: { voxels: box(0, 0, 0, M, M, 8).concat(box(0, 0, 8, 8, M, M)) },
  }, [M, M, M]);
  // Floor vertex right against the wall, versus one far out on the open floor.
  const crease = 1 - ell(9, 24, 8, 0, 0, 1);
  const open = 1 - ell(M - 4, 24, 8, 0, 0, 1);
  check('an inner corner is occluded', crease > 0.25,
    `crease ${(crease * 100).toFixed(1)}% occluded`);
  check('the open floor beside it is not', open < 1e-9,
    `open floor ${(open * 100).toFixed(4)}%`);
  // …and it fades with distance rather than switching on at the wall.
  const mid = 1 - ell(9 + AO_RADIUS, 24, 8, 0, 0, 1);
  check('the crease fades outward', crease > mid && mid >= 0,
    `${(crease * 100).toFixed(1)}% at the wall, ${(mid * 100).toFixed(1)}% AO_RADIUS out`);
}

// The baseline is the whole reason an open flank cancels; a zero here would
// mean the subtraction had quietly become a no-op.
check('the flat-wall baseline is the fifth it should be',
  AO_BASELINE > 0.05 && AO_BASELINE < 0.5,
  `AO_BASELINE ${AO_BASELINE.toFixed(6)} at AO_OFFSET ${AO_OFFSET}`);

console.log(failures ? `\n${failures} assertion(s) failed` : '\nall assertions passed');
process.exit(failures ? 1 : 0);
