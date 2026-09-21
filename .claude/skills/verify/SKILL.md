---
name: verify
description: Run this project's checks before finishing work — which of verify-core-loop.sh, cargo test -p sim_core, cargo test -p voxgen, verify-scaffold.sh and screenshot-scene.sh applies, and how to read their output. Use when asked to verify, test, check, or validate a change, or before committing / handing work back.
---

# Verifying a change

Pick the cheapest check that actually covers what you touched, then run the
full one before handing work back.

## Which check

| You changed | Run |
|---|---|
| `sim_core/` only (pure Rust sim) | `cargo test -p sim_core` |
| `assets/voxels/generators/` or `palette.ron` | `cargo test -p voxgen` |
| Anything in `godot_project/`, the wire, or data files | `scripts/verify-core-loop.sh` |
| Anything visual / a content-id lookup | `scripts/screenshot-scene.sh` — see the `screenshot-review` skill |
| The `gdext` scaffold, a `gdext`/Godot version bump | `scripts/verify-scaffold.sh` |

**`scripts/verify-core-loop.sh` is the full gate.** In order it runs
`cargo test -p sim_core`, `cargo test -p voxgen`, `cargo xtask`, two
`godot --headless --import` passes, checks `godot_project/.godot/
extension_list.cfg` registers `sim_core.gdextension`, checks every staged
`.vox` produced a `.vox.import`, then runs **every headless Godot script** (the run prints how many),
each grepped for its own success marker. It takes minutes; run it once at the
end, not in a loop.

`scripts/verify-scaffold.sh` is narrower: `cargo xtask`, prime the Godot cache
if it is missing, then `test_round_trip.gd` (a `SimBridge.ping()` round trip).
It is the milestone-1 proof, not a substitute for the core loop.

Both Godot scripts need `godot` on `PATH`; override with `GODOT_BIN=/path/to/godot`.

## Gotchas — each of these has burned someone

1. **Godot exits 0 even when a `--script` file fails to PARSE.** It prints the
   parse error and quits cleanly. Exit status alone is NOT proof a test ran.
   Every `godot_project/test_*.gd` therefore prints its own marker
   (`CORE_LOOP_OK`, `STAFF_PANEL_OK`, …) and `verify-core-loop.sh` greps for
   it. **If you run a `test_*.gd` by hand, grep for its marker**, e.g.

   ```
   godot --headless --path godot_project --script test_hover.gd 2>&1 | grep HOVER_OK
   ```

   A new test script must print a marker and be added to
   `verify-core-loop.sh`'s `run_godot_script` list, or it is not being run.

2. **Godot needs `--import` twice on a cold `.godot/` cache.** The `.vox`
   importer is an editor plugin and registers only on the pass *after* the
   one that first scans the project; the first pass silently skips every
   voxel model. `verify-core-loop.sh` runs it twice unconditionally. A fresh
   worktree needs `cargo xtask` plus both import passes before the world is
   visible at all.

3. **`cargo xtask` builds a DEBUG `sim_core`** (`cargo build -p sim_core`,
   copied from `target/debug/`). Every timing number measured through Godot
   is a debug number. `cargo xtask --release` builds and copies the release
   library instead.

4. **`verify-scaffold.sh` deliberately ignores the import step's exit code.**
   On Godot 4.6.3 the editor process SIGSEGVs *after* writing the cache,
   during GDExtension doc generation on shutdown — a known upstream bug,
   unrelated to `sim_core`. The script verifies the cache file directly
   instead. Do not "fix" this.

5. **A headless test cannot catch a stale content-id lookup.** Structural
   tests pass while the scene renders nothing, because the lookup falls back.
   Only the rendered frame catches it — run `scripts/screenshot-scene.sh` for
   any presentation change.

6. **`cargo` may not be where you expect.** It is not at `~/.cargo/bin/cargo`
   here; it resolves through rustup
   (`~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo`). Use
   `command -v cargo` before concluding Rust is unavailable.

7. `vengi-thumbnailer` (voxel previews/icons) needs a real video driver and is
   not part of any headless check. `cargo test -p voxgen` does not need it.

Deeper: `docs/WORKFLOW.md` ("Build" and "Verification"),
`docs/ARCHITECTURE.md` §6 (testing strategy) and §7 (the headless development
loop); each script's own header comment explains what it proves.
