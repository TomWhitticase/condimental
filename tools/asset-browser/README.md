# tools/asset-browser — look at every voxel asset in one place

A local web app for reviewing `assets/voxels/`: a searchable list of every
asset, a 3D viewer you can turn (the models render in the game's own
isometric, under the game's own lighting, and orbit out of it), each asset's
variations, and the facts about it that are otherwise spread across five
files.

```sh
scripts/asset-browser.sh          # or: python3 tools/asset-browser/serve.py
```

It opens <http://127.0.0.1:8765/>. Python 3 only — no build step, no npm.
`three.js` is fetched once into the gitignored `web/vendor/` on first run,
then used offline.

The query string is the automation seam, and every switch in it is one the
buttons already throw: `?view=game&facing=2&ao=0&grid=0&bare=1#machine_cooking_vat`
frames a model the way a screenshot wants it (`bare=1` strips everything but
the viewport, `grid=0` leaves nothing on the backdrop but the model and its
shadow, so a measurement can tell which pixels are the subject). Add
`&shot=<name>` and the page posts that frame to the server's `--shot-dir`;
that is how `calibrate-ao.py` measures the viewer.

## What it reads

Everything on screen is derived live from files already in the repo — the
browser cannot drift from the pipeline:

| On screen | Comes from |
|---|---|
| the model you see | `assets/voxels/models/<name>.<class>.vox`, parsed directly |
| material of each mesh | `models/materials.json` (`metallic`, `roughness`, emission) |
| which classes an asset has | `models/index.json` |
| colour names in the palette panel | `assets/voxels/palette.ron`, in file order |
| the prose under "generator" | the `///` doc comment on the generator fn |
| "drawn for" | `main.gd`'s `MACHINE_MODELS` / `ITEM_MODELS` / … tables |
| footprint | `data/machines.ron` |
| the UI icon strip | `assets/voxels/icons/<name>.png` |

Re-run `cargo run -p voxgen --bin voxgen` and the change is there **without reloading the
page**: it polls `/api/stamp` (the mtimes of `models/`, `index.json`,
`palette.ron`, `assets.rs`, `main.gd` and `machines.ron`) every 1.5s, and on
any change refetches the catalog, rebuilds the meshes in hand and toasts what
arrived — `2 new assets: machine_barrel, machine_barrel_fill`. A new asset
lands in the sidebar under whichever category its name prefix picks, `Misc` if
none does; the asset you were looking at stays on screen, at the same framing,
with its new voxels.

## How assets are grouped

79 `.vox` models are not 79 things to review. The sidebar lists **assets**;
the strip along the bottom of the viewer lists that asset's **variations**,
folded from the naming the pipeline already uses:

- `X_f0..f3` — animation frames of `X` (belts, motion overlays)
- `X_motion_f*` — the motion overlay, which is *cut out of* `X`'s body, so
  the viewer draws body + overlay together and `play` cycles the frames
- `X_fill` — the fill overlay (hopper, fluid tank)
- `pipe_fluid*` and `splitter_arrow` are drawn over a host they are
  meaningless without, so the viewer offers that host behind them

Categories come from the name prefix (`CATEGORY_RULES` in `voxlib.py`). A new
asset appears on its own, with no change here, as soon as `voxgen` writes it.

## Keys

`/` search · `↑`/`↓` previous/next asset · `space` play the frames ·
`1`-`4` game / front / side / top · `f`/`F` facing · `r` turntable ·
`g` grid · `o` ambient occlusion · `b` backdrop

The grid is one game tile per amber square (32 voxels), with two tiles of
margin, so a model's footprint is checkable by eye.

`f` (and the **E S W N** buttons) spins the model the way a rotated placement
does: 0/90/180/270° about the **footprint origin** — the `.vox` origin, which
is the footprint's centre at ground level — in the sim's own clockwise order,
east, south, west, north. That is the whole of what facing means to the
pipeline (UI_UX.md §2b point 18), so what you see is what the game will draw
at that facing, and the footprint line in the details panel swaps its extents
with it. Use it on anything with a front: a carved doorway does not survive a
180° spin, and an overlay that lines up on its host in one facing may be off
in the other three (§2b point 29). The turntable is a different thing — a
free orbit, for reading a silhouette.

`b` cycles three backdrops, all lit identically — only what is *behind* the
model changes, so a silhouette can be judged against each: **haze** (the
hall's air, what the game's camera actually sees past a machine), **hall**
(the factory-interior gradient, roof-cool over slab-warm) and **dark** (a
near-black plate, for reading an emissive lens or a thin bright edge).

## Files

- `serve.py` — the stdlib HTTP server and its three JSON endpoints
  (`/api/catalog`, `/api/model/<name>`, `/api/stamp`), plus `POST /api/shot`,
  which exists only when `--shot-dir` is passed and is how a headless run gets
  the viewer's own pixels out
- `calibrate-ao.py` — derives the AO constants from the game's SSAO, and
  fails when the two passes are not shading the same amount of machine (below)
- `test-ao.mjs` — `node tools/asset-browser/test-ao.mjs`: asserts the bake on
  shapes whose answer is known without rendering anything, chiefly that a
  solid cube reads fully open on all six of its face normals
- `voxlib.py` — `.vox` parsing, `palette.ron`/`assets.rs`/`main.gd` mining,
  and the grouping rules
- `web/` — the page: `app.js` (three.js viewer), `ao.mjs` (the occlusion
  bake, three.js-free so it can be tested under node), `index.html`,
  `style.css`

## Caveats

- The lighting is ported from the game, not shared with it: the two
  `DirectionalLight3D`s of `factory_environment.gd` at their Godot angles and
  energies, the factory-interior sky as an environment map, ACES tonemapping
  and a bloom pass standing in for Godot's glow. It is calibrated by
  measurement — a cooking vat's stainless reads #d4d4d3 on top and #666c6f on
  the camera-facing flank here against #d2d3d4 and #646969 in
  `scripts/screenshot-scene.sh --fixture vat`, its painted blue #153d57
  against #173b58. Re-measure if `factory_environment.gd` or
  `factory_interior_sky.gdshader` is retuned; `app.js` says how.
- The ambient occlusion is baked per voxel, not screen-space: the occupancy
  grid the mesh is built from is blurred and sampled along each face's normal,
  and dims *indirect diffuse only*, as Godot's SSAO does with
  `ssao_light_affect` at 0. It is two constants in `web/ao.mjs`, and neither
  is anybody's taste. `AO_RADIUS` is the game's `ssao_radius` in voxels —
  the hall runs `ssao_radius 0.45 / ssao_intensity 16.0 / ssao_power 1.6`,
  and `ssao_radius` is in WORLD units, so 0.45 m is 4.5 voxels at
  `VOXEL_UNIT` 0.1 — and the kernel and the sample distance follow from it.
  `AO_POWER` is the game's `ssao_power`, applied where Godot applies it: as a
  gamma on the occlusion, before any strength. `AO_STRENGTH` is **measured**:
  `calibrate-ao.py` renders the game's vat with SSAO on and off and this
  browser on the same model with AO on and off, and divides one renderer's
  shading per machine pixel by the other's. Today: **5.067**, at which the
  browser removes 3.49% of the machine's light against the game's 4.36%, over
  0.75x its coverage and 1.26x its depth. Re-derive all of it in one command
  whenever the game's SSAO or ambient energy moves — the answer is a constant,
  not another investigation:

  ```sh
  tools/asset-browser/calibrate-ao.py --write    # --keep DIR keeps the frames
  ```

  **The constant comes from the game beside the line, not from the play
  camera, and that is why the game will look flatter than this.** The bake is
  object-space: it is a property of the model, which is the thing this browser
  exists to show. A 0.45 m world radius is a few pixels wide at the isometric
  play camera and resolves almost nothing there — the same vat at the same
  settings has 67.8% of its pixels darkened by more than 1% standing beside
  it, and 0.3% at the play camera (0.01% of its light removed, against 4.36%).
  Calibrating where the effect cannot resolve would have matched the camera
  rather than the model, and would have shipped a pass nobody could see. So a
  play-camera screenshot really is flatter than this viewer. That is the
  camera's pixel scale, not browser-only exaggeration, and it is not a bug.

  The settings above are PR **#104**'s retune and #104 must merge first. Until
  it does, `main`'s hall runs 0.9 / 2.4 and `calibrate-ao.py` refuses to derive
  against it at all: it exits 1 saying the game darkens only 0.19% of the
  machine, so there is no crease there to calibrate against.

  What it measures is *how much of the machine* each renderer shades and how
  deeply, per machine pixel — never the deep end of the pixels it happened to
  darken, which is area-normalised and cannot tell a broad faint wash from two
  deep creases. It is that blindness that let a bake ship darkening half of
  every model's flat faces at a constant that reproduced to three decimals
  (2026-09-20), so the run now **fails** when the browser's coverage diverges
  from the game's, and says that no strength can fix a coverage gap.

  It also refuses the frames themselves. `screenshot-scene.sh` occasionally
  hands back a frame captured before the fixture is built or before the pose
  is applied — a perfectly plausible PNG of an empty hall at the default
  framing, which is how a nonsense constant got written once. A believable
  before/after has the machine covering a sane share of the frame, has the
  pass-on frame keeping the machine in the same pixels (SSAO moves no
  geometry), and has the pass only ever removing light. Anything else is
  re-shot, three times, and then the run fails.

  Deliberately *no* stronger than the game — precisely, never darker than the
  game's own strongest honest presentation of the same geometry. A browser
  that darkened creases harder than the renderer it stands in for would let a
  form be approved that only reads under browser-only occlusion. What it
  therefore cannot do is rescue a shape the game's own lighting leaves flat.
- The bake has a test, because an open flank reading open is arithmetic and
  not a matter of eye: `node tools/asset-browser/test-ao.mjs` asserts that a
  solid cube reads fully open on all six of its face normals, that each axis
  reads its two faces alike, and that an inner corner is still occluded.
- That AO is per model and object-space, with two consequences. It does not
  cross between an overlay and its host — `pipe_fluid` sitting in `pipe` casts
  no shadow into it — and it does not change with facing or with a hidden
  class: spinning a model or unticking `metal` re-lights nothing, because the
  occlusion is the geometry's, not the frame's.
- For "how does this look in the game" with a floor, neighbours and the HUD,
  `scripts/screenshot-scene.sh` is still the answer; this is for "what is
  this asset, and what else is in it".
- It reads the repo it lives in and serves on loopback only.
