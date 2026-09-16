---
name: Bug
about: Something is broken now — wrong behaviour, a failing check, a visual defect
title: ''
labels: bug
---

## What happens, and what should happen

## Reproducing it

<!-- The exact command: a `cargo test` name, a `scripts/screenshot-scene.sh`
invocation with its flags, or the fixture in `test_layouts.gd` that shows it.
Every headless Godot test prints its own success marker — say which marker is
missing, since Godot exits 0 on a parse failure. -->

## Where it probably lives

<!-- `sim_core` (deterministic logic), `godot_project` (presentation), the
voxel generators, or the data files. If a number looks wrong, say whether it
was measured on a debug build — `cargo xtask` builds debug unless passed
`--release`, and every timing in the decisions log is a debug figure. -->
