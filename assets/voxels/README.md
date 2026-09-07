# Voxel assets

The asset pipeline described in [docs/UI_UX.md](../../docs/UI_UX.md) §2a/§2b,
plus the UI-icon extension of it described in §5.

| Path | What it is |
|---|---|
| `palette.ron` | **Source.** The single source of truth for every colour *and* every material class (matte / metal / emissive) in the game's art. Every generator references it; none invent their own colours. |
| `generators/` | **Source.** The `voxgen` crate — one small Rust function per asset, built from composable primitives, plus the `voxicons` binary (`generators/icons.rs`) that turns a model into a UI icon. This, not the `.vox`/icon `.png` files, is what an asset actually *is*. |
| `models/` | **Build output, committed.** Regenerate with `cargo run -p voxgen`. Never hand-edit a `.vox` here: the next regeneration will silently wipe the change. |
| `icons/` | **Build output, committed.** Small (64px), transparent-background, tightly-cropped PNG icons — one per combined model, for UI use (UI_UX.md §5). Regenerate with `cargo run -p voxgen --bin voxicons` (needs `vengi`, see below). Committed like `models/`, not gitignored like `previews/`, because `godot_project/main.gd` loads these at runtime. |
| `previews/` | **Build output, gitignored.** Isometric PNGs for visual review, plus `scene.png`, a full render of the game scene. **Not the same thing as `icons/`** — see "Icons vs. previews" below. |

## Commands

```
scripts/build-voxels.sh              # regenerate models, then render previews AND icons
cargo run -p voxgen                  # regenerate models only (no vengi needed)
cargo run -p voxgen --bin voxicons   # regenerate icons only (needs vengi)
cargo xtask                          # stage models/ and icons/ into godot_project/voxels/ for Godot
scripts/screenshot-scene.sh          # render the real scene to previews/scene.png
```

`scripts/build-voxels.sh`'s header documents how to install `vengi`, and
UI_UX.md §2b records the two non-obvious things about this toolchain (the
exact thumbnailer flags, and Godot needing two `--import` passes).

## Icons vs. previews

Both are PNGs rendered from the same `.vox` models via the same
`vengi-thumbnailer`, and it's tempting to assume one is just a smaller copy
of the other — they are not, and this was checked against the actual files
rather than assumed (see `generators/icons.rs`'s module doc for the full
writeup). `previews/*.png` are 512px, full-frame, **opaque** (vengi always
fills the background solid; there is no transparency/background cvar at
all) renders meant to be looked at large during review. `icons/*.png` are a
second, separate render pass: 64px, background keyed out to transparent,
cropped tight to the model — meant to sit in a small UI slot next to text,
which is exactly what `main.gd`'s `_icon_for` does with them (the
contract-status readout's item icon). Regenerating previews does not
regenerate icons or vice versa; `scripts/build-voxels.sh` runs both.

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
