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
buttons already throw: `?view=game&facing=2&ao=0&bare=1#machine_cooking_vat`
frames a model the way a screenshot wants it (`bare=1` strips everything but
the viewport). Add `&shot=<name>` and the page posts that frame to the
server's `--shot-dir`; that is how `calibrate-ao.py` measures the viewer.

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
- `calibrate-ao.py` — derives `AO_STRENGTH` from the game's SSAO (above)
- `voxlib.py` — `.vox` parsing, `palette.ron`/`assets.rs`/`main.gd` mining,
  and the grouping rules
- `web/` — the page: `app.js` (three.js viewer), `index.html`, `style.css`

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
  `ssao_light_affect` at 0. Its strength is one constant, `AO_STRENGTH` in
  `app.js`, and it is **measured rather than chosen** — `calibrate-ao.py`
  renders the game's vat with SSAO on and off and this browser on the same
  model with AO on and off, and divides one deepest-crease darkening by the
  other. Today: the game 5.0%, the browser 4.7% at `AO_STRENGTH = 0.294`. Re-
  derive it in one command whenever `ssao_intensity`, `ssao_radius` or the
  ambient energy moves — the answer is a constant to paste, not another
  investigation:

  ```sh
  tools/asset-browser/calibrate-ao.py        # --keep DIR to keep the frames
  ```

  Deliberately *no* stronger than the game: a browser that darkened creases
  harder than the renderer it stands in for would let a form be approved that
  only reads under browser-only occlusion. What it therefore cannot do is
  rescue a shape the game's own lighting leaves flat.
- That AO is per model and object-space, with two consequences. It does not
  cross between an overlay and its host — `pipe_fluid` sitting in `pipe` casts
  no shadow into it — and it does not change with facing or with a hidden
  class: spinning a model or unticking `metal` re-lights nothing, because the
  occlusion is the geometry's, not the frame's.
- For "how does this look in the game" with a floor, neighbours and the HUD,
  `scripts/screenshot-scene.sh` is still the answer; this is for "what is
  this asset, and what else is in it".
- It reads the repo it lives in and serves on loopback only.
