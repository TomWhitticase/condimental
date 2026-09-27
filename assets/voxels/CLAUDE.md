# assets/voxels — authoring a voxel asset

Reasoning lives in `docs/UI_UX.md` §2a/§2b and `docs/DECISIONS.md` Part B —
§2b's point numbers are cited from code comments, so **keep them stable**.
`README.md` here has paths, commands and the icons-vs-previews writeup.

## Shape of the pipeline

- **Source of truth is a Rust generator function** in `generators/assets.rs`,
  registered in `ASSETS` (§2a point 1).
- **`models/` and `icons/` are generated build output — committed, never
  hand-edited** (§2a point 2); regeneration silently wipes a manual change.
  They are committed because `main.gd` loads both at runtime; `previews/` is
  gitignored review output.
- **Review is a headless render you actually look at** (§2a point 3).
- **`palette.ron` is the single source of BOTH colour and material class**
  (§2a point 5). Use `p.idx("name")`; an unknown name panics at build time,
  deliberately. Indices come from file order — **append**, never reorder.
- **One asset = one mesh per material class** (§2b point 2). `voxgen` writes
  `name.vox` (combined, for review) plus `name.<class>.vox` per class used;
  Godot rebuilds one `StandardMaterial3D` per class from `materials.json`,
  classes listed in `index.json`. Classes: matte, metal, emissive, liquid.
- **On a machine, `liquid` means "this is the product" — nothing else may
  use it.** A machine that shows sauce paints it with `machine_product`, and
  `main.gd` tints that one mesh from the recipe the machine is actually
  running (`PRODUCT_TINTS`), so one model serves every sauce. A second
  `liquid` colour on such a body would be repainted as sauce along with it;
  a voxgen test pins that none exists.
- **The `.vox` encoder is hand-written** (`generators/vox.rs`, §2b point 1) —
  no Rust crate emits the `nTRN`/`nGRP`/`nSHP` scene graph the pinned Godot
  importer needs. Origin is the **footprint centre at ground level**.
- **One tile = 32×32×32 voxels** (`vox::TILE_VOXELS`); a kind's model size is
  its `data/machines.ron` footprint × 32 (1x2 mixer 32×64, 2x2 vat 64×64).

## Geometry traps — the expensive ones

- **Anything Godot spins is symmetric about voxel 15.5, not 16** (§2b point
  20). Build on `PIPE_C = 15.5` with the `*_f` float primitives, or a run at
  mixed facings steps at every joint.
- **A fitting overlay is a full voxel of radius larger than what it covers**
  (`PIPE_FITTING_MARGIN`, §2b point 24) or the solids z-fight at
  shared-neighbour voxels; it clears the host's sight windows (§2b p22).
- **A live body inside a hollow shape stays strictly narrower than the outer
  wall at every height it reaches** (§2b point 19) — z-fighting you only see
  with liquid actually in the run.
- **Motion overlays are CUT OUT of the body, never duplicated** (§2b point
  27, motion): `<machine>_motion_f0..f3`, siblings, one visible. Every motion
  volume is an **even voxel height**; **consecutive frames must differ**, or
  a reciprocating part reads as stopped.
- **Check an overlay against the host turned all four ways with a test, not
  by eye** (§2b point 29) — a preview shows one facing.

## Camera rules (fixed isometric, never orbits)

- **Only a tile's +X and +Z-world faces are ever seen** (§2b point 11): an
  outlet is a doorway carved through +X, an inlet a hooded housing cut open
  on the camera-facing side (the true inlet face is invisible), both read by
  **value contrast, not colour**. A tall kind (the vat) widens its opening
  to an unoccluded corner.
- **A dark accent on a camera-facing shadow-side face reads as a hole** (§2b
  point 21) — keep accents on bright plates.
- **A thin diagonal mark does not survive this camera; a solid shape does**
  (§2b point 23) — a chevron rendered as an `L`.
- **A rotated placement reuses the same mesh spun about its footprint
  origin** (§2b point 18), so carved doorways do not survive a 180° spin —
  belt/pipe junctions are the plain tile plus overlaid fittings.

## The icon pass

- **A second render pass** (`generators/icons.rs`, §2b point 7): 256px
  render, flood-filled transparent from the border pixels, cropped tight,
  downscaled to 64px — not a shrunk preview (`vengi-thumbnailer` has no
  alpha cvar, so previews are always opaque and padded). The same pass also
  writes `app_icon.png`, `item_bottle` at 512px from a 1024px render: the
  window/dock icon `project.godot`'s `config/icon` loads.
- **A model whose only consumer is the icon pass is authored for vengi's
  camera, the −X/+Y corner, not the game's** (§2b point 35):
  the two cameras see opposite sides of the same model.
- **At icon scale a thing is identified by the proportion of its parts, not
  their presence** (§2b point 31) — judge it at real size on the real plate.

## Material direction

- **Food contact stainless, structure painted or poured, emissive for status
  lamps only** (§2b point 8; `oak` is the one exception, §2b point 14).
- **Metal keeps the brightest pixel in the frame** (UI_UX §4); the floor,
  judged as thousands of copies, is broad low-contrast blotches — never
  per-voxel speckle — and must never hold it (§2b point 36).

## Workflow

```
scripts/build-voxels.sh              # models + previews + icons (needs vengi)
cargo run -p voxgen --bin voxgen     # models only, no vengi needed
cargo run -p voxgen --bin voxicons   # icons only
cargo test -p voxgen                 # voxel asserts: openings empty, frames
                                     #   differ, even heights, symmetry
cargo xtask                          # stage into godot_project/voxels/
```

- **`voxgen` does not touch `icons/`.** A recolour or reshape that stops at
  `--bin voxgen` leaves the old look in the build menu, the hotbar and every
  hover panel, which read the committed PNGs — run `--bin voxicons` (or
  `scripts/build-voxels.sh`) too, then `cargo xtask`. The render is
  deterministic: only the model you changed comes back modified.
- `scripts/asset-browser.sh` opens every asset at once in a browser you can
  turn — variations, palette, footprint and the generator's own doc comment
  side by side. It is a reading of the `.vox` files lit the way the game
  lights them (ported, not shared — and with no ambient occlusion), so it
  supplements the render checks below rather than replacing them.
- **Review the rendered asset, not the generator, and budget a rework pass
  per asset** (§2b point 9) — looking changes the design.
- `vengi-thumbnailer` renders through OpenGL and **needs a real video
  driver**; `SDL_VIDEODRIVER=dummy` fails (§2b point 3).
- Godot needs `--import` **twice** on a cold cache (§2b point 4).
- A hand-sculpted `.vox` with no generator is allowed (§2a point 4) but must
  be marked in `README.md`; there are none today.
