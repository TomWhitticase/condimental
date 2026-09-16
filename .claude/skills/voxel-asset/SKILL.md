---
name: voxel-asset
description: Author or change a voxel model or UI icon in assets/voxels/ — write the Rust generator, pick colours from palette.ron, regenerate the .vox and PNGs, and actually look at the render. Use when adding a new machine/belt/pipe/item/icon model, editing an existing one, changing a colour or material class, or when a model looks wrong in the scene.
---

# Authoring a voxel asset

The source of an asset is a **Rust generator function**, never the `.vox`
file. The `.vox` and icon PNGs are committed build output; regenerating
silently wipes any hand edit.

## Procedure

1. **Write or edit the generator** in `assets/voxels/generators/assets.rs`
   and register it in the `ASSETS` table (`main.rs` iterates `assets::ASSETS`;
   an unregistered function emits nothing). Build from the file's existing
   primitives rather than new ad-hoc loops.

2. **Take every colour from `assets/voxels/palette.ron`** via `p.idx("name")`.
   That file is the single source of BOTH colour and material class
   (`matte` / `metal` / `emissive` / `liquid`). An unknown name panics at
   build time, deliberately. Palette indices come from file order — **append
   only**; reordering re-indexes every generated `.vox`.

3. **Read `assets/voxels/CLAUDE.md` before touching geometry.** It holds the
   expensive traps in condensed form: symmetry about voxel 15.5 (not 16) for
   anything Godot rotates, the fitting-overlay margin, live bodies narrower
   than their hollow shell, motion overlays cut out of the body at even voxel
   heights with consecutive frames differing, and the fixed camera's rule
   that only a tile's +X and +Z-world faces are ever seen. `docs/UI_UX.md`
   §2b has the same points at length (point numbers are cited from code
   comments — keep them stable).

4. **Regenerate:**

   ```
   scripts/build-voxels.sh              # models + review PNGs + icons (needs vengi)
   cargo run -p voxgen                  # models only, no vengi — fastest inner loop
   cargo run -p voxgen --bin voxicons   # icons only (needs vengi)
   ```

   `build-voxels.sh` runs `voxgen`, renders a 512px isometric preview per
   *combined* model into `assets/voxels/previews/` (the per-material-class
   `name.metal.vox` splits are skipped), then runs `voxicons`.

5. **LOOK at the render.** Open `assets/voxels/previews/<name>.png` with the
   Read tool and judge the image. Reviewing the generator is not reviewing
   the asset (§2b point 9). **Budget a rework pass** — looking changes the
   design, every time. For an icon, judge `assets/voxels/icons/<name>.png` at
   its real 64px size on the real plate, not blown up (§2b point 31).

6. **`cargo test -p voxgen`** — voxel-level assertions a mesh or a screenshot
   cannot make: that a carved inlet/outlet is a genuinely empty opening, that
   motion frames differ, that heights are even, that overlays meet the host
   at all four facings.

7. **`cargo xtask`** stages `models/` + `icons/` into the gitignored
   `godot_project/voxels/`, then reference the model from
   `godot_project/main.gd`. Every content-id lookup needs a fallback — a
   stale key ships silently with every test green. Finish with
   `scripts/screenshot-scene.sh` (see the `screenshot-review` skill) to see
   the asset in the real scene.

8. **Commit `assets/voxels/models/` and `assets/voxels/icons/`** along with
   the generator — `main.gd` loads both at runtime. `assets/voxels/previews/`
   is gitignored and never committed.

## Gotchas

- `vengi-thumbnailer` renders through OpenGL and **needs a real video
  driver**. `SDL_VIDEODRIVER=dummy` fails with an OpenGL error — don't set
  it. Override the binary with `VENGI_THUMBNAILER=/path/to/vengi-thumbnailer`;
  `build-voxels.sh`'s header has the install command for the pinned v0.5.0.
- **If vengi is missing, `build-voxels.sh` warns and exits 0** with previews
  and icons skipped. A green exit is not proof a preview was written — check
  the file exists before concluding the asset looks fine.
- Godot needs `godot --headless --path godot_project --import` **twice** on a
  cold `.godot/` cache: the `.vox` importer is an editor plugin and registers
  only on the pass after the first scan.
- Icons need no separate registration — `voxicons` renders one per combined
  model automatically.

Deeper: `assets/voxels/CLAUDE.md`, `assets/voxels/README.md`,
`docs/UI_UX.md` §2a/§2b, `docs/DECISIONS.md` Part B.
