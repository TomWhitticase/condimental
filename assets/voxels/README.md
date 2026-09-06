# Voxel assets

The asset pipeline described in [docs/UI_UX.md](../../docs/UI_UX.md) §2a/§2b.

| Path | What it is |
|---|---|
| `palette.ron` | **Source.** The single source of truth for every colour *and* every material class (matte / metal / emissive) in the game's art. Every generator references it; none invent their own colours. |
| `generators/` | **Source.** The `voxgen` crate — one small Rust function per asset, built from composable primitives. This, not the `.vox` file, is what an asset actually *is*. |
| `models/` | **Build output, committed.** Regenerate with `cargo run -p voxgen`. Never hand-edit a `.vox` here: the next regeneration will silently wipe the change. |
| `previews/` | **Build output, gitignored.** Isometric PNGs for visual review, plus `scene.png`, a full render of the game scene. |

## Commands

```
scripts/build-voxels.sh              # regenerate every model, then render previews
cargo run -p voxgen                  # regenerate models only (no vengi needed)
cargo xtask                          # stage models into godot_project/voxels/ for Godot
scripts/screenshot-scene.sh          # render the real scene to previews/scene.png
```

`scripts/build-voxels.sh`'s header documents how to install `vengi`, and
UI_UX.md §2b records the two non-obvious things about this toolchain (the
exact thumbnailer flags, and Godot needing two `--import` passes).

## Adding an asset

1. Write a generator function in `generators/assets.rs` and add it to
   `ASSETS`. Use only colours named in `palette.ron` — an unknown name is a
   build-time panic, deliberately.
2. Run `scripts/build-voxels.sh` and *look at* the PNG it writes into
   `previews/`. That is the review step, and skipping it means shipping a
   model nobody has seen.
3. Run `cargo xtask` so Godot gets the new model, then reference it from
   `godot_project/main.gd`.

## Hand-sculpted assets

UI_UX.md §2a point 4 allows a one-off `.vox` sculpted by hand in
`vengi-voxedit` with no generator behind it. There are none right now. If one
is ever added, mark it clearly here so nobody goes looking for a generator
that does not exist — and note that `cargo run -p voxgen` writes only what
its generators produce, so a hand-authored file in `models/` would survive a
regeneration but would not be reproducible from source.
