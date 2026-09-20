// The voxel asset browser's whole front end: sidebar, three.js viewer and
// details panel. It knows nothing about the game — every fact on screen
// comes from /api/catalog, which is read live out of assets/voxels/.

import * as THREE from 'three';
import { OrbitControls } from './vendor/examples/jsm/controls/OrbitControls.js';
import { EffectComposer } from './vendor/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from './vendor/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from './vendor/examples/jsm/postprocessing/UnrealBloomPass.js';
import { OutputPass } from './vendor/examples/jsm/postprocessing/OutputPass.js';

import { AO_STRENGTH, aoField } from './ao.mjs';

const TILE = 32;                 // voxels per world tile (vox::TILE_VOXELS)
const FRAME_MS = 180;            // animation speed for frame variations

// The simulation's own clockwise order (`Direction::rotate_cw`, restated in
// main.gd's DIRECTIONS), so facing N is a yaw of N * 90 degrees — the whole
// of what a rotated placement does to a mesh (UI_UX.md §2b point 18).
const FACINGS = ['east', 'south', 'west', 'north'];

const state = {
  catalog: null,
  asset: null,
  base: null,                    // model name currently shown as the body
  frame: 0,                      // index into the asset's frame list
  overlays: { motion: true, fill: true, companion: true },
  classes: {},                   // class name -> visible
  playing: false,
  spinning: false,
  view: 'game',
  facing: 0,                     // index into FACINGS; 0 is as authored
  grid: true,
  ao: true,
  backdrop: 0,                   // index into BACKDROPS
  query: '',
};

const cache = new Map();         // model name -> payload
const groups = new Map();        // model name -> THREE.Group

const $ = (sel) => document.querySelector(sel);
const el = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text !== undefined) n.textContent = text;
  return n;
};

function toast(msg, bad) {
  const box = $('#error');
  box.textContent = msg;
  box.classList.toggle('good', !bad);
  box.classList.add('show');
  clearTimeout(toast.timer);
  toast.timer = setTimeout(() => box.classList.remove('show'), 6000);
}

function fail(msg) {
  toast(msg, true);
  console.error(msg);
}

// ---------------------------------------------------------------- three.js

const viewport = $('#viewport');
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 1.0;
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
viewport.appendChild(renderer.domElement);

const scene = new THREE.Scene();

// ----------------------------------------------------------------- the hall
//
// Everything down to the composer is godot_project/factory_environment.gd's
// `build_into()` ported to three.js: the same two lights at the same angles,
// colours and energies, the same sky-derived ambient, the same ACES exposure
// and the same glow pass. An asset reviewed here is lit the way the factory
// lights it, which is the only way a lighting judgement made in this tool
// survives into the game.
//
// Not ported: SSAO (issue filed) — Godot darkens voxel crevices, so expect
// this viewer to read very slightly flatter in tight detail.

/// The hall, built as the light it throws. `factory_interior_sky.gdshader`
/// paints a roof deck, trusses, clad walls with clerestory glazing, high-bay
/// lamps and a concrete slab, all fading into haze — and a sky IS a radiance
/// map, so the honest port of it is a room of those surfaces around the
/// model rather than a picture of one.
///
/// Values over 1 are the whole point: the shader's lamps burn at
/// `lamp_energy` 4.5 and its glazing at `window_energy` 1.35, and it is those,
/// not the grey cladding, that put the highlight down a stainless tank. A
/// painted canvas cannot hold them (it clips at white); a scene can, because
/// PMREM renders it to a half-float target.
///
/// This is the ambient that fills a voxel's shadowed faces
/// (AMBIENT_SOURCE_SKY) and the roof its metal reflects (REFLECTION_SOURCE_SKY).
const HALL = {
  haze: '#939495',    // HAZE_COLOR — the air, and the isometric backdrop
  deck: '#3A3C40',    // painted roof deck
  clad: '#6A6A63',    // concrete cladding
  slab: '#979489',    // the floor of the hall, under everything
  lamp: '#FFF6EA',    // high-bay fittings
  glass: '#B9C6CE',   // clerestory glazing
};

/// How much of a surface at that distance the haze has eaten
/// (`1 - exp(-t * haze_density)`, HAZE_DENSITY 0.0052): the far wall is 132
/// units off and reads half air, the roof at 26 barely any. Getting this
/// wrong is not cosmetic — the haze is LIGHTER than the cladding it swallows,
/// so a hall modelled without it lights the models too dimly.
function hazed(hex, fog) {
  return new THREE.Color(hex).lerp(new THREE.Color(HALL.haze), fog);
}

function hallEnvironment() {
  const env = new THREE.Scene();
  const lit = (colour, energy = 1) => new THREE.MeshBasicMaterial({
    color: colour.clone().multiplyScalar(energy),
    side: THREE.BackSide,
  });
  const W = 60, HEIGHT = 26, D = 60;
  const wall = hazed(HALL.clad, 0.5);
  const room = new THREE.Mesh(new THREE.BoxGeometry(W, HEIGHT, D), [
    lit(wall), lit(wall),                                     // +X, -X
    lit(hazed(HALL.deck, 0.13)), lit(hazed(HALL.slab, 0.5)),  // roof, slab
    lit(wall), lit(wall),                                     // +Z, -Z
  ]);
  room.position.y = HEIGHT / 2 - 5;   // the slab sits a little below the floor
  env.add(room);

  // Skylight ribbons down the bays, and the clerestory band under the roof.
  // Small in area, far over 1 in value: the two things in the hall bright
  // enough to be seen reflected in a machine.
  const lamp = new THREE.MeshBasicMaterial({
    // Not the shader's `lamp_energy` of 4.5: these ribbons stand in for
    // fittings that are small, far off and half eaten by haze, so the value
    // is what a machine's stainless measures back, not what the fitting emits.
    color: new THREE.Color(HALL.lamp).multiplyScalar(1.8),
  });
  const glass = new THREE.MeshBasicMaterial({
    color: new THREE.Color(HALL.glass).multiplyScalar(1.35),  // window_energy
  });
  for (const x of [-18, 0, 18]) {
    const ribbon = new THREE.Mesh(new THREE.BoxGeometry(3, 0.4, D * 0.8), lamp);
    ribbon.position.set(x, HEIGHT - 6, 0);
    env.add(ribbon);
  }
  for (const [x, z, w, d] of [[0, -D / 2, W, 0.4], [0, D / 2, W, 0.4],
                              [-W / 2, 0, 0.4, D], [W / 2, 0, 0.4, D]]) {
    const band = new THREE.Mesh(new THREE.BoxGeometry(w, 3, d), glass);
    band.position.set(x, HEIGHT - 9, z);
    env.add(band);
  }
  return env;
}

const pmrem = new THREE.PMREMGenerator(renderer);
const hall = pmrem.fromScene(hallEnvironment(), 0.02).texture;
scene.environment = hall;
// Reflections, at the hall's own radiance. Godot's `ambient_light_energy`
// does NOT scale these — REFLECTION_SOURCE_SKY reads the sky as it is — so
// this knob answers to the metal only, and 1.25 is where a cooking vat's
// stainless lands on the game's: tops #d4d4d3 here against #d2d3d4 in
// `scripts/screenshot-scene.sh --fixture vat`, camera-facing flanks #666c6f
// against #646969. Re-measure if the hall above is ever retuned.
scene.environmentIntensity = 1.25;

// ...and the diffuse half of the same sky, which Godot DOES scale, by
// `ambient_light_energy = 1.2`. three.js has one number for both, so the
// difference arrives as a hemisphere: roof-cool over slab-warm, lighting
// every face a little and reflecting in nothing. It is what holds the painted
// steel up — at 1.2 the vat's blue flanks read #153d57 against the game's
// #173b58, and without it they sink to #113045.
const ambient = new THREE.HemisphereLight(
  new THREE.Color(HALL.glass), new THREE.Color(HALL.slab), 1.2
);
scene.add(ambient);

// Key light. rotation_degrees (-58, -130, 0) resolved to the direction it
// travels, (0.406, -0.848, 0.341), which three.js states as the position it
// shines FROM. Note where it is not: the camera looks down (1, 1, 1), so the
// key is behind the model's shoulder and the two faces the game ever shows a
// player are held up by ambient and fill. That is exactly the constraint
// UI_UX §2b point 11 is about, and it only bites if the light matches.
const key = new THREE.DirectionalLight(0xffffff, 1.6);
key.color.setRGB(1.0, 0.98, 0.95, THREE.LinearSRGBColorSpace);
key.position.set(-0.406, 0.848, -0.341).multiplyScalar(300);
key.castShadow = true;
key.shadow.mapSize.set(2048, 2048);
key.shadow.bias = -0.0015;
// shadow_blur 1.4 / light_angular_distance 0.6: a soft edge, not a hard one.
key.shadow.radius = 2.4;
scene.add(key);

// Fill: rotation_degrees (-28, 55, 0), no shadows. Cool daylight through the
// clerestory, balancing the lamp-white key so grey assets do not photograph
// beige.
const fill = new THREE.DirectionalLight(0xffffff, 0.4);
fill.color.setRGB(0.80, 0.85, 0.96, THREE.LinearSRGBColorSpace);
fill.position.set(0.723, 0.469, 0.506).multiplyScalar(300);
scene.add(fill);

// The game's backdrop is the first of these (BG_COLOR + HAZE_COLOR, what
// `set_backdrop_for` gives the isometric camera); the hall itself is what TAB
// reveals in first person; dark is the tool's own, for reading a silhouette.
const BACKDROPS = [
  { name: 'haze', apply: () => { scene.background = new THREE.Color(HALL.haze); } },
  { name: 'hall', apply: () => { scene.background = hall; scene.backgroundBlurriness = 0.3; } },
  { name: 'dark', apply: () => { scene.background = new THREE.Color(0x16171a); } },
];
BACKDROPS[0].apply();

// Orthographic, direction (1,1,1): the game's own camera (main.gd's
// CAMERA_DIRECTION / PROJECTION_ORTHOGONAL), so a model reads here the way
// it reads in the factory.
const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, -2000, 2000);
const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.dampingFactor = 0.12;
controls.zoomSpeed = 0.8;

// A catcher for the key light's shadow: the grid alone gives no sense of
// how far a part overhangs its tile.
const shadowPlane = new THREE.Mesh(
  new THREE.PlaneGeometry(1, 1),
  new THREE.ShadowMaterial({ opacity: 0.35 })
);
shadowPlane.rotation.x = -Math.PI / 2;
shadowPlane.receiveShadow = true;
scene.add(shadowPlane);

const root = new THREE.Group();          // holds the model, spun by turntable
scene.add(root);

// The placement's facing, which is a different rotation from the turntable
// above and has to stay separate from it: the turntable is the reviewer
// walking round a machine, this is the machine itself put down the other way
// round. It sits at the origin because the origin IS the footprint centre at
// ground level (the `.vox` encoder's rule, assets/voxels/CLAUDE.md), so a
// yaw applied here is the same mesh about the same pivot the game spins it
// about — not the mesh's own centre, which for an off-centre part is a
// different point and a different picture.
const facingRoot = new THREE.Group();
root.add(facingRoot);

function setFacing(i) {
  state.facing = ((i % 4) + 4) % 4;
  // main.gd's `_direction_yaw`: index * 90 degrees about +Y, and three.js
  // turns about +Y exactly as Godot does (local +X goes to (cos t, 0, -sin t)
  // in both), so this is the game's rotation, not a lookalike.
  facingRoot.rotation.y = state.facing * Math.PI / 2;
  document.querySelectorAll('#facings button').forEach((b) =>
    b.classList.toggle('on', Number(b.dataset.facing) === state.facing));
  renderHudTitle();
  if (state.asset) renderDetails();
}

/// The models on screen, and — when it is not the authored one — the facing
/// they are turned to, because a screenshot of this viewer has to say which
/// of the four it is.
function renderHudTitle(names) {
  if (names) renderHudTitle.names = names;
  const facing = state.facing
    ? `   ·   facing ${FACINGS[state.facing]} (${state.facing * 90}°)`
    : '';
  $('#hud-title').textContent = (renderHudTitle.names || []).join('  +  ') + facing;
}

const gridRoot = new THREE.Group();
scene.add(gridRoot);

function rebuildGrid(size) {
  gridRoot.clear();
  // Two tiles of margin, so the model's own tile is visible as a tile
  // rather than as the whole ground.
  const tiles = Math.max(1, Math.ceil(size / TILE)) + 2;
  const span = tiles * TILE;
  const fine = new THREE.GridHelper(span, tiles * 4, 0x3a3d43, 0x2a2c31);
  const coarse = new THREE.GridHelper(span, tiles, 0xd9a21b, 0xd9a21b);
  coarse.material.opacity = 0.3;
  coarse.material.transparent = true;
  fine.position.y = 0.02;
  coarse.position.y = 0.04;
  gridRoot.add(fine, coarse);
  gridRoot.visible = state.grid;
  shadowPlane.scale.set(span * 2, span * 2, 1);

  key.shadow.camera.left = -span;
  key.shadow.camera.right = span;
  key.shadow.camera.top = span;
  key.shadow.camera.bottom = -span;
  key.shadow.camera.near = 1;
  key.shadow.camera.far = span * 6;
  key.shadow.camera.updateProjectionMatrix();
}

// env.glow_*: intensity 0.7 over an hdr threshold of 0.85. Godot blends it
// SOFTLIGHT and this adds, so the strength is pulled down to land a status
// lamp at about the same bloom rather than a brighter one.
const composer = new EffectComposer(renderer);
composer.addPass(new RenderPass(scene, camera));
// Strength well under Godot's 0.7 because SOFTLIGHT tempers what it adds and
// this pass simply adds; the threshold is the game's, so bright stainless
// bleeds a little exactly as it does in the factory, and a status lamp sits
// in a halo rather than a hard dot.
const bloom = new UnrealBloomPass(new THREE.Vector2(1, 1), 0.35, 0.4, 0.85);
composer.addPass(bloom);
// Tone maps and converts to sRGB at the end, so the bloom above sees the
// same HDR values Godot's glow thresholds against.
composer.addPass(new OutputPass());

function resize() {
  const w = viewport.clientWidth;
  const h = viewport.clientHeight;
  renderer.setSize(w, h);
  composer.setSize(w, h);
  // Half resolution for the glow's own mip chain: it is a blur, nobody can
  // see the difference, and it is most of the cost of the pass.
  bloom.setSize(Math.max(1, Math.round(w / 2)), Math.max(1, Math.round(h / 2)));
  frameCamera(w / h);
}
new ResizeObserver(resize).observe(viewport);

let framedRadius = 40;
let framedCentre = 16;
function frameCamera(aspect) {
  const halfH = framedRadius;
  const halfW = halfH * (aspect || 1);
  camera.left = -halfW;
  camera.right = halfW;
  camera.top = halfH;
  camera.bottom = -halfH;
  camera.updateProjectionMatrix();
}

function setView(which) {
  const d = 400;
  const dirs = {
    game: [1, 1, 1],
    front: [1, 0.25, 0.05],
    side: [0.05, 0.25, 1],
    top: [0.001, 1, 0.001],
  };
  const v = dirs[which] || dirs.game;
  state.view = which;
  const len = Math.hypot(...v);
  camera.position.set((v[0] / len) * d, framedCentre + (v[1] / len) * d, (v[2] / len) * d);
  controls.target.set(0, framedCentre, 0);
  camera.up.set(0, 1, 0);
  controls.update();
  document.querySelectorAll('#views button').forEach((b) =>
    b.classList.toggle('on', b.dataset.view === which));
}

// A handle for the browser console — this is a dev tool, and being able to
// poke the scene ("why is that face dark?") is part of the job.
window.viewer = { THREE, scene, camera, renderer, composer, root, state };

// Set by `?shot=`, cleared by the frame that honours it.
let pendingShot = null;

function tick() {
  requestAnimationFrame(tick);
  if (state.spinning) root.rotation.y += 0.006;
  controls.update();
  composer.render();
  // In the same frame as the draw, because a canvas without
  // `preserveDrawingBuffer` is empty by the next one.
  if (pendingShot) {
    const name = pendingShot;
    pendingShot = null;
    renderer.domElement.toBlob((png) => fetch(`/api/shot?name=${name}`,
      { method: 'POST', body: png }));
  }
}
tick();

// ------------------------------------------------------------- occlusion
//
// The bake itself is `ao.mjs` — arithmetic with no three.js in it, so
// `test-ao.mjs` asserts on the same code the viewer runs. What is left here
// is how it reaches the pixels.

// One uniform object shared by every material, so the `ao` button — and
// `calibrate-ao.py`'s `?ao=` override — is a single assignment rather than a
// walk of the scene graph.
const aoStrength = { value: AO_STRENGTH };

// three has no vertex-attribute AO — `aoMap` is a texture on a second UV set,
// which a voxel mesh has no use for — so the baked value rides in as an
// attribute and lands exactly where `<aomap_fragment>` would have put a map's.
const AO_CHUNK = `
  float ambientOcclusion = 1.0 - voxAoStrength * ( 1.0 - vVoxAo );
  reflectedLight.indirectDiffuse *= ambientOcclusion;
`;

/// Adds the baked AO to whatever `material` already does in its own
/// `onBeforeCompile` (the emissive class has one), rather than replacing it.
function withBakedAo(material) {
  const already = material.onBeforeCompile;
  material.onBeforeCompile = (shader, renderer) => {
    if (already) already(shader, renderer);
    shader.uniforms.voxAoStrength = aoStrength;
    shader.vertexShader = 'attribute float voxAo;\nvarying float vVoxAo;\n' +
      shader.vertexShader.replace('void main() {', 'void main() {\n\tvVoxAo = voxAo;');
    shader.fragmentShader = 'uniform float voxAoStrength;\nvarying float vVoxAo;\n' +
      shader.fragmentShader.replace('#include <aomap_fragment>',
        `#include <aomap_fragment>\n${AO_CHUNK}`);
  };
  // Bound, because the default implementation three supplies reads
  // `this.onBeforeCompile` — called loose it throws, and a material's program
  // is then never compiled at all (a black viewport, 2026-09-20).
  const key = material.customProgramCacheKey.bind(material);
  material.customProgramCacheKey = () => `voxAo|${key()}`;
  return material;
}

// ------------------------------------------------------------ mesh building

const FACES = [
  { dir: [1, 0, 0], corners: [[1, 0, 0], [1, 0, 1], [1, 1, 1], [1, 1, 0]] },
  { dir: [-1, 0, 0], corners: [[0, 0, 1], [0, 0, 0], [0, 1, 0], [0, 1, 1]] },
  { dir: [0, 1, 0], corners: [[0, 1, 0], [1, 1, 0], [1, 1, 1], [0, 1, 1]] },
  { dir: [0, -1, 0], corners: [[0, 0, 1], [1, 0, 1], [1, 0, 0], [0, 0, 0]] },
  { dir: [0, 0, 1], corners: [[1, 0, 1], [0, 0, 1], [0, 1, 1], [1, 1, 1]] },
  { dir: [0, 0, -1], corners: [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]] },
];

// vox (x, y, z) -> three (x, z, -y), the mapping Godot's importer uses, so
// "front" here is the same face as "front" in the game.
const keyOf = (x, y, z) => x | (y << 8) | (z << 16);

function buildClassMesh(voxels, palette, size, material, ao) {
  const filled = new Set();
  for (let i = 0; i < voxels.length; i += 4) {
    filled.add(keyOf(voxels[i], voxels[i + 1], voxels[i + 2]));
  }
  const pos = [];
  const nrm = [];
  const col = [];
  const occ = [];
  const c = new THREE.Color();
  const ox = size[0] / 2;
  const oy = size[1] / 2;

  for (let i = 0; i < voxels.length; i += 4) {
    const x = voxels[i];
    const y = voxels[i + 1];
    const z = voxels[i + 2];
    c.setStyle(palette[voxels[i + 3] - 1] || '#ff00ff');
    for (const face of FACES) {
      // In vox space the face normal is (dx, dy, dz); cull against the
      // same class only, so hiding a class never opens a hole in another.
      const [dx, dy, dz] = face.dir;
      if (filled.has(keyOf(x + dx, y + dy, z + dz))) continue;
      const quad = face.corners.map(([cx, cy, cz]) => [
        x + cx - ox,          // three x  = vox x
        z + cz,               // three y  = vox z (up)
        -(y + cy) + oy,       // three z  = -vox y
      ]);
      // Occlusion is read at the quad's own corners, so it varies ACROSS one
      // voxel face: a face running into a crease is shaded along its length,
      // the way a screen-space pass would shade it, rather than stepping a
      // flat value per voxel.
      const shade = face.corners.map(([cx, cy, cz]) =>
        ao(x + cx, y + cy, z + cz, dx, dy, dz));
      // Wound the other way round the quad than the corner lists read: the
      // lists go clockwise seen from outside, and a clockwise triangle is a
      // BACK face, culled — which left every model drawn from the inside,
      // lit by whatever was behind it. (2026-09-17: that is what made a vat
      // look like a hollow white shell you could see through.)
      for (const i of [0, 2, 1, 0, 3, 2]) {
        const v = quad[i];
        pos.push(v[0], v[1], v[2]);
        nrm.push(dx, dz, -dy);
        col.push(c.r, c.g, c.b);
        occ.push(shade[i]);
      }
    }
  }

  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  geo.setAttribute('normal', new THREE.Float32BufferAttribute(nrm, 3));
  geo.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
  geo.setAttribute('voxAo', new THREE.Float32BufferAttribute(occ, 1));
  return new THREE.Mesh(geo, material);
}

function materialFor(cls) {
  const def = (state.catalog.materials || {})[cls] || {};
  const mat = new THREE.MeshStandardMaterial({
    vertexColors: true,
    metalness: def.metallic ?? 0,
    roughness: def.roughness ?? 0.9,
  });
  const energy = def.emission_energy || 0;
  if (energy > 0) {
    // voxel_emissive.gdshader: the lens is ALBEDO *and* EMISSION, so a lamp
    // is a lit surface that also glows in its own colour — not an unlit
    // patch. three's standard material carries one emissive colour for the
    // whole mesh and an asset can hold lamps of several colours, so the
    // emission is taken from the vertex colour instead, the same way the
    // shader does, and the glow pass above turns it into bloom.
    mat.onBeforeCompile = (shader) => {
      shader.uniforms.voxEmission = { value: energy };
      shader.fragmentShader = shader.fragmentShader
        .replace('void main() {', 'uniform float voxEmission;\nvoid main() {')
        .replace(
          '#include <emissivemap_fragment>',
          '#include <emissivemap_fragment>\n\ttotalEmissiveRadiance = vColor * voxEmission;'
        );
    };
    mat.customProgramCacheKey = () => `voxEmission:${energy}`;
  }
  return withBakedAo(mat);
}

async function modelPayload(name) {
  if (cache.has(name)) return cache.get(name);
  const res = await fetch(`/api/model/${name}`);
  if (!res.ok) throw new Error(`model ${name}: ${res.status}`);
  const payload = await res.json();
  cache.set(name, payload);
  return payload;
}

async function modelGroup(name) {
  if (groups.has(name)) return groups.get(name);
  const payload = await modelPayload(name);
  const group = new THREE.Group();
  group.name = name;
  const ao = aoField(payload.classes, payload.size);
  for (const [cls, part] of Object.entries(payload.classes)) {
    const mesh = buildClassMesh(
      part.voxels, payload.palette, payload.size, materialFor(cls), ao
    );
    mesh.userData.cls = cls;
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    group.add(mesh);
  }
  group.userData.size = payload.size;
  groups.set(name, group);
  return group;
}

// -------------------------------------------------------------- asset logic

const framesOf = (asset) => asset.variations.filter((v) => v.role === 'base' || v.role === 'frame');
const motionOf = (asset) => asset.variations.filter((v) => v.role === 'motion');
const fillsOf = (asset) => asset.variations.filter((v) => v.role === 'fill');

function currentModels() {
  const asset = state.asset;
  const frames = framesOf(asset);
  const shown = [];
  const base = frames[Math.min(state.frame, frames.length - 1)] || asset.variations[0];
  shown.push(base.name);
  const motion = motionOf(asset);
  if (state.overlays.motion && motion.length) {
    shown.push(motion[state.frame % motion.length].name);
  }
  if (state.overlays.fill) {
    for (const f of fillsOf(asset)) shown.push(f.name);
  }
  // The host this asset is only ever drawn over (a pipe under its liquid).
  if (asset.companion && state.overlays.companion) shown.unshift(asset.companion);
  state.base = base.name;
  return shown;
}

async function renderCurrent({ refit = false, quiet = false } = {}) {
  const names = currentModels();
  let loaded;
  try {
    loaded = await Promise.all(names.map(modelGroup));
  } catch (err) {
    fail(String(err));
    return;
  }
  facingRoot.clear();
  let size = [1, 1, 1];
  for (const g of loaded) {
    facingRoot.add(g);
    const s = g.userData.size;
    size = [Math.max(size[0], s[0]), Math.max(size[1], s[1]), Math.max(size[2], s[2])];
  }
  applyClassVisibility();
  if (refit) {
    // Half the ortho frustum height: a cube of side s projects about 1.7s
    // tall in this isometric, so 0.95s of half-height leaves a little air.
    framedRadius = Math.max(size[0], size[1], size[2]) * 0.95;
    framedCentre = size[2] / 2;
    rebuildGrid(Math.max(size[0], size[1]));
    resize();
    setView(state.view);
  }
  renderHudTitle(names);
  renderVariations();
  // Playback re-renders every frame; rebuilding the details panel that
  // often would fight the reader's scroll position for no new information.
  if (!quiet) renderDetails();
}

function applyClassVisibility() {
  root.traverse((n) => {
    if (n.isMesh) n.visible = state.classes[n.userData.cls] !== false;
  });
}

// ------------------------------------------------------------------ sidebar

function renderList() {
  const list = $('#list');
  list.innerHTML = '';
  const q = state.query.trim().toLowerCase();
  const terms = q.split(/\s+/).filter(Boolean);
  const match = (a) => terms.every((t) => a.search.includes(t));
  let shown = 0;

  for (const cat of state.catalog.categories) {
    const assets = state.catalog.assets.filter((a) => a.category === cat && match(a));
    if (!assets.length) continue;
    list.appendChild(el('div', 'cat', cat));
    for (const asset of assets) {
      const row = el('button', 'row');
      row.dataset.id = asset.id;
      if (asset.icon) {
        const img = el('img');
        img.src = `/icons/${asset.icon}`;
        img.alt = '';
        row.appendChild(img);
      } else {
        row.appendChild(el('div', 'no-icon'));
      }
      row.appendChild(el('span', 'name', asset.label));
      if (asset.variations.length > 1) {
        row.appendChild(el('span', 'n', `${asset.variations.length}`));
      }
      row.classList.toggle('sel', state.asset && asset.id === state.asset.id);
      row.onclick = () => select(asset.id);
      list.appendChild(row);
      shown += 1;
    }
  }
  $('#counts').textContent =
    `${shown} of ${state.catalog.assets.length} assets · ` +
    `${Object.keys(state.catalog.models).length} models`;
}

// --------------------------------------------------------- variation strip

function renderVariations() {
  const strip = $('#variations');
  strip.innerHTML = '';
  const asset = state.asset;
  const frames = framesOf(asset);

  frames.forEach((v, i) => {
    const b = el('button', 'var');
    b.appendChild(document.createTextNode(v.frame === null ? 'body' : `frame ${v.frame}`));
    b.appendChild(el('span', 'role', v.name));
    b.classList.toggle('sel', i === Math.min(state.frame, frames.length - 1));
    b.onclick = () => { state.frame = i; renderCurrent(); };
    strip.appendChild(b);
  });

  const motion = motionOf(asset);
  if (motion.length) {
    const b = el('button', 'var');
    b.appendChild(document.createTextNode('motion overlay'));
    b.appendChild(el('span', 'role', `${motion.length} frames · cut out of the body`));
    b.classList.toggle('sel', state.overlays.motion);
    b.onclick = () => { state.overlays.motion = !state.overlays.motion; renderCurrent(); };
    strip.appendChild(b);
  }

  if (asset.companion) {
    const b = el('button', 'var');
    b.appendChild(document.createTextNode(`with ${asset.companion}`));
    b.appendChild(el('span', 'role', 'the host it is drawn over'));
    b.classList.toggle('sel', state.overlays.companion);
    b.onclick = () => {
      state.overlays.companion = !state.overlays.companion;
      renderCurrent();
    };
    strip.appendChild(b);
  }

  for (const f of fillsOf(asset)) {
    const b = el('button', 'var');
    b.appendChild(document.createTextNode('fill overlay'));
    b.appendChild(el('span', 'role', f.name));
    b.classList.toggle('sel', state.overlays.fill);
    b.onclick = () => { state.overlays.fill = !state.overlays.fill; renderCurrent(); };
    strip.appendChild(b);
  }
}

// -------------------------------------------------------------- class chips

function renderClassToggles() {
  const box = $('#classes');
  box.innerHTML = '';
  const companion = state.asset.companion;
  const classes = new Set(state.asset.classes);
  if (companion) for (const c of state.catalog.assets.find((a) => a.id === companion).classes) classes.add(c);
  for (const cls of classes) {
    if (!(cls in state.classes)) state.classes[cls] = true;
    const b = el('button', state.classes[cls] ? 'on' : '', cls);
    b.title = `show / hide the ${cls} mesh`;
    b.onclick = () => {
      state.classes[cls] = !state.classes[cls];
      b.classList.toggle('on', state.classes[cls]);
      applyClassVisibility();
    };
    box.appendChild(b);
  }
}

// ------------------------------------------------------------ details panel

function docToHtml(doc) {
  const esc = doc.replace(/[&<>]/g, (ch) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[ch]));
  return esc
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/`([^`]+)`/g, '<code>$1</code>');
}

function renderDetails() {
  const panel = $('#details');
  const asset = state.asset;
  const model = state.catalog.models[state.base];
  panel.innerHTML = '';

  panel.appendChild(el('h2', null, asset.label));
  panel.appendChild(el('div', 'sub', `${asset.category} · ${asset.variations.length} model${asset.variations.length > 1 ? 's' : ''}`));

  const facts = el('section');
  facts.appendChild(el('h3', null, `showing ${model.name}`));
  const dl = el('dl');
  const put = (k, v) => { dl.appendChild(el('dt', null, k)); dl.appendChild(el('dd', null, v)); };
  put('volume', `${model.size.join(' × ')} voxels`);
  put('tiles', `${model.tiles[0]} × ${model.tiles[1]} × ${model.tiles[2]}`);
  if (model.bbox) put('occupied', `${model.bbox.size.join(' × ')}`);
  put('voxels', model.voxels.toLocaleString());
  put('file', `${(model.bytes / 1024).toFixed(1)} KiB`);
  if (asset.footprint) {
    // A quarter turn SWAPS the extents (main.gd's `_rotated_footprint`), so
    // the tiles a 1x2 mixer claims are not the tiles it claims turned.
    const [fw, fh] = asset.footprint;
    put('footprint', state.facing % 2
      ? `${fw} × ${fh} tiles · ${fh} × ${fw} facing ${FACINGS[state.facing]}`
      : `${fw} × ${fh} tiles`);
  }
  facts.appendChild(dl);
  const chips = el('div');
  for (const cls of model.classes) {
    const n = model.class_voxels[cls];
    chips.appendChild(el('span', `chip ${cls}`, n ? `${cls} · ${n}` : cls));
  }
  facts.appendChild(chips);
  panel.appendChild(facts);

  if (asset.used_by.length) {
    const s = el('section');
    s.appendChild(el('h3', null, 'drawn for'));
    const d = el('dl');
    for (const u of asset.used_by) {
      d.appendChild(el('dt', null, u.key));
      d.appendChild(el('dd', 'mono', u.table));
    }
    s.appendChild(d);
    panel.appendChild(s);
  }

  if (model.icon) {
    const s = el('section');
    s.appendChild(el('h3', null, 'ui icon'));
    const shot = el('div', 'iconshot');
    const img = el('img');
    img.src = `/icons/${model.icon}`;
    img.width = 64;
    img.height = 64;
    shot.appendChild(img);
    const small = el('img');
    small.src = `/icons/${model.icon}`;
    small.width = 32;
    small.height = 32;
    shot.appendChild(small);
    shot.appendChild(el('span', 'muted', 'at 64 and 32 px'));
    s.appendChild(shot);
    panel.appendChild(s);
  }

  const colors = el('section');
  colors.appendChild(el('h3', null, 'palette'));
  const sw = el('div', 'swatches');
  for (const c of model.colors) {
    const row = el('div', 'sw');
    const chip = el('i');
    chip.style.background = c.rgb;
    row.appendChild(chip);
    row.appendChild(el('span', 'sw-name', c.name));
    row.appendChild(el('span', 'sw-n muted', `${c.class} · ${c.count}`));
    row.title = `${c.rgb} (palette index ${c.index})`;
    sw.appendChild(row);
  }
  colors.appendChild(sw);
  panel.appendChild(colors);

  const variations = el('section');
  variations.appendChild(el('h3', null, 'variations'));
  const vdl = el('dl');
  for (const v of asset.variations) {
    const m = state.catalog.models[v.name];
    const dt = el('dt', 'mono', v.role === 'base' ? 'body' : v.role + (v.frame !== null ? ` ${v.frame}` : ''));
    const dd = el('dd', null, `${v.name} · ${m.voxels.toLocaleString()} voxels`);
    if (v.name === state.base) dd.style.color = 'var(--accent)';
    vdl.appendChild(dt);
    vdl.appendChild(dd);
  }
  variations.appendChild(vdl);
  panel.appendChild(variations);

  const src = model.source;
  if (src) {
    const s = el('section');
    s.appendChild(el('h3', null, 'generator'));
    if (src.doc) {
      const d = el('div', 'doc');
      d.innerHTML = docToHtml(src.doc);
      s.appendChild(d);
    } else if (asset.note) {
      const d = el('div', 'doc');
      d.innerHTML = docToHtml(asset.note);
      s.appendChild(d);
    }
    const link = el('span', 'src', `${src.file}:${src.line}  fn ${src.fn}()`);
    s.appendChild(link);
    if (src.via) s.appendChild(el('span', 'src', src.via));
    panel.appendChild(s);
  }
}

// ----------------------------------------------------------------- selection

function select(id, { push = true } = {}) {
  const asset = state.catalog.assets.find((a) => a.id === id);
  if (!asset) return;
  state.asset = asset;
  state.frame = 0;
  state.classes = {};
  renderClassToggles();
  renderList();
  const drawn = renderCurrent({ refit: true });
  if (push) location.hash = id;
  return drawn;
}

// ------------------------------------------------------------- live reload
//
// An asset enters the game exactly one way — a generator in
// `generators/assets.rs` and `cargo run -p voxgen --bin voxgen` — and that run rewrites
// `models/`. The server stamps those files; the page watches the stamp and
// rebuilds the catalog in place when it moves. So a newly generated asset
// walks into the sidebar on its own, and an asset you are LOOKING at while
// you regenerate it redraws itself, which is the tighter loop of the two:
// edit the generator, re-run voxgen, watch the model change.
const POLL_MS = 1500;
let stamp = null;

async function pollSources() {
  let next;
  try {
    const res = await fetch('/api/stamp');
    if (!res.ok) return;
    next = JSON.stringify((await res.json()).stamp);
  } catch {
    return;                      // server down or restarting; try again later
  }
  if (stamp === null || next === stamp) {
    stamp = next;
    return;
  }
  stamp = next;
  await reloadCatalog();
}

async function reloadCatalog() {
  let next;
  try {
    next = await (await fetch('/api/catalog')).json();
  } catch (err) {
    fail(`could not reload the catalog: ${err}`);
    return;
  }
  const before = new Set(state.catalog.assets.map((a) => a.id));
  state.catalog = next;

  // Every mesh in hand was built from the old .vox bytes. Drop the lot —
  // detached from the scene first, since the render loop is still running.
  facingRoot.clear();
  cache.clear();
  for (const group of groups.values()) {
    group.traverse((n) => {
      if (!n.isMesh) return;
      n.geometry.dispose();
      n.material.dispose();
    });
  }
  groups.clear();

  const added = next.assets.filter((a) => !before.has(a.id));
  const kept = next.assets.find((a) => a.id === state.asset?.id);
  if (kept) {
    state.asset = kept;
    renderList();
    await renderCurrent();       // same asset, same framing, new voxels
  } else {
    // The asset on screen is gone (renamed, or its generator withdrawn).
    select((added[0] || next.assets[0]).id, { push: false });
  }
  toast(added.length
    ? `${added.length} new asset${added.length > 1 ? 's' : ''}: ${added.map((a) => a.name).join(', ')}`
    : 'assets reloaded');
}

// ------------------------------------------------------------------ chrome

let timer = null;
function setPlaying(on) {
  state.playing = on;
  $('#play').classList.toggle('on', on);
  if (timer) clearInterval(timer);
  timer = null;
  if (!on) return;
  timer = setInterval(() => {
    const asset = state.asset;
    const n = Math.max(framesOf(asset).length, motionOf(asset).length);
    if (n <= 1) return;
    state.frame = (state.frame + 1) % n;
    renderCurrent({ quiet: true });
  }, FRAME_MS);
}

function wire() {
  $('#search').addEventListener('input', (e) => {
    state.query = e.target.value;
    renderList();
  });
  document.querySelectorAll('#views button').forEach((b) => {
    b.onclick = () => setView(b.dataset.view);
  });
  document.querySelectorAll('#facings button').forEach((b) => {
    b.onclick = () => setFacing(Number(b.dataset.facing));
  });
  $('#ao-toggle').onclick = (e) => {
    state.ao = !state.ao;
    aoStrength.value = state.ao ? AO_STRENGTH : 0;
    e.currentTarget.classList.toggle('on', state.ao);
  };
  $('#spin').onclick = (e) => {
    state.spinning = !state.spinning;
    e.currentTarget.classList.toggle('on', state.spinning);
    if (!state.spinning) root.rotation.y = 0;
  };
  $('#play').onclick = () => setPlaying(!state.playing);
  $('#grid-toggle').onclick = (e) => {
    state.grid = !state.grid;
    gridRoot.visible = state.grid;
    e.currentTarget.classList.toggle('on', state.grid);
  };
  $('#bg-toggle').onclick = (e) => {
    state.backdrop = (state.backdrop + 1) % BACKDROPS.length;
    const mode = BACKDROPS[state.backdrop];
    mode.apply();
    e.currentTarget.textContent = `bg: ${mode.name}`;
    e.currentTarget.classList.toggle('on', state.backdrop !== 0);
  };

  addEventListener('keydown', (e) => {
    if (e.target.tagName === 'INPUT') {
      if (e.key === 'Escape') e.target.blur();
      return;
    }
    if (e.key === '/') { e.preventDefault(); $('#search').focus(); return; }
    if (e.key === ' ') { e.preventDefault(); setPlaying(!state.playing); return; }
    if (e.key === 'r') $('#spin').click();
    if (e.key === 'g') $('#grid-toggle').click();
    if (e.key === 'b') $('#bg-toggle').click();
    if (e.key === 'o') $('#ao-toggle').click();
    // One step clockwise, which is the step the game's own R makes
    // (`Direction::rotate_cw`); shift-F walks back the other way.
    if (e.key === 'f' || e.key === 'F') {
      setFacing((state.facing + (e.key === 'F' ? 3 : 1)) % 4);
    }
    if (['1', '2', '3', '4'].includes(e.key)) {
      setView(['game', 'front', 'side', 'top'][Number(e.key) - 1]);
    }
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const rows = [...document.querySelectorAll('.row')].map((r) => r.dataset.id);
      const i = rows.indexOf(state.asset.id);
      const next = rows[(i + (e.key === 'ArrowDown' ? 1 : rows.length - 1)) % rows.length];
      if (next) {
        select(next);
        document.querySelector(`.row[data-id="${next}"]`)?.scrollIntoView({ block: 'nearest' });
      }
    }
  });

  addEventListener('hashchange', () => {
    const id = decodeURIComponent(location.hash.slice(1));
    if (id && (!state.asset || id !== state.asset.id)) select(id, { push: false });
  });
}

/// The query string is the automation seam: `calibrate-ao.py` needs the same
/// model twice at two AO strengths, framed identically and with nothing but
/// the model in the frame, and a browser that can only be driven by hand
/// cannot be measured against the game at all.
///
///     ?ao=0&view=game&facing=2&bare=1#machine_cooking_vat
///
/// `ao` is the strength uniform (blank means AO_STRENGTH); `grid=0` takes the
/// tile grid away, so what is left on the backdrop is the model and the
/// shadow it casts and a measurement can tell which pixels are the subject;
/// `bare` strips the chrome down to the viewport; `shot=<name>` posts the next drawn frame to
/// the server's `--shot-dir` and is how a headless run gets pixels back
/// (Chrome's own `--screenshot` fires on a load heuristic, before a model has
/// been fetched). Nothing here is a second way to configure the viewer —
/// every switch is the one the buttons already throw.
function applyQuery() {
  const q = new URLSearchParams(location.search);
  if (q.has('view')) setView(q.get('view'));
  if (q.has('facing')) setFacing(Number(q.get('facing')));
  if (q.has('ao')) {
    aoStrength.value = Number(q.get('ao'));
    state.ao = aoStrength.value > 0;
    $('#ao-toggle').classList.toggle('on', state.ao);
  }
  if (q.get('grid') === '0') $('#grid-toggle').click();
  if (q.get('bare') === '1') {
    for (const sel of ['#sidebar', '#details', '#toolbar', '#hud', '#variations']) {
      $(sel).style.display = 'none';
    }
    document.querySelector('#app').style.gridTemplateColumns = 'minmax(0, 1fr)';
    resize();
  }
  if (q.has('shot')) pendingShot = q.get('shot');
}

async function boot() {
  try {
    const res = await fetch('/api/catalog');
    state.catalog = await res.json();
  } catch (err) {
    fail(`could not load the catalog: ${err}`);
    return;
  }
  wire();
  renderList();
  resize();
  const wanted = decodeURIComponent(location.hash.slice(1));
  const first = state.catalog.assets.find((a) => a.id === wanted) || state.catalog.assets[0];
  await select(first.id, { push: false });
  applyQuery();
  document.body.dataset.ready = '1';
  pollSources();
  setInterval(pollSources, POLL_MS);
}

boot();
