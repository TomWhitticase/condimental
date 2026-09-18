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

Re-run `cargo run -p voxgen` and the change is there **without reloading the
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
`1`-`4` game / front / side / top · `r` turntable · `g` grid · `b` backdrop

The grid is one game tile per amber square (32 voxels), with two tiles of
margin, so a model's footprint is checkable by eye.

`b` cycles three backdrops, all lit identically — only what is *behind* the
model changes, so a silhouette can be judged against each: **haze** (the
hall's air, what the game's camera actually sees past a machine), **hall**
(the factory-interior gradient, roof-cool over slab-warm) and **dark** (a
near-black plate, for reading an emissive lens or a thin bright edge).

## Files

- `serve.py` — the stdlib HTTP server and its three JSON endpoints
  (`/api/catalog`, `/api/model/<name>`, `/api/stamp`)
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
- There is no ambient occlusion, which Godot's `WorldEnvironment` has on
  (issue #49): contact shadows where a fitting meets a body are softer here
  than in the game.
- For "how does this look in the game" with a floor, neighbours and the HUD,
  `scripts/screenshot-scene.sh` is still the answer; this is for "what is
  this asset, and what else is in it".
- It reads the repo it lives in and serves on loopback only.
