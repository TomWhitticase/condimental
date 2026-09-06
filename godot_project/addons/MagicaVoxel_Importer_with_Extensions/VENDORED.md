# Vendored, pinned: MagicaVoxel Importer with Extensions

This directory is a **verbatim vendored copy** of a third-party Godot 4
editor plugin, checked in rather than fetched, and pinned the same way every
other dependency in this project is (ARCHITECTURE.md §1a).

| | |
|---|---|
| Upstream | https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions |
| Pinned commit | `288f1d8740a0b653bb3b9bbe98256c70985f3ed0` ("Merge Godot 4.x support to main", 2024-10-26) |
| Plugin version | 1.2 |
| License | MIT (see `Metadata/LICENSE`) |

## Why this one

UI_UX.md §2a listed two candidate importers and §9/ARCHITECTURE.md §11 left
"which one" open. Resolved during the visual-polish pass: the second
candidate (`cschram/godot-vox-import`) now redirects to this same repository,
so there is effectively one maintained Godot 4 `.vox` importer, not two. It
imports a `.vox` straight to a `Mesh` resource with greedy face merging and
vertex colours, which is exactly what `main.gd` consumes.

## What it constrains on our side

The importer builds its mesh by walking the file's **scene graph** starting
at node 0, so a `.vox` without `nTRN`/`nGRP`/`nSHP` chunks cannot be imported
at all. That requirement is why `assets/voxels/generators/vox.rs` writes its
own encoder instead of using an off-the-shelf crate — see that file's module
doc.

The plugin is unmodified. Treat any upstream bump as a reviewed decision (the
same rule ARCHITECTURE.md §1a applies to `gdext`), not a routine update.
