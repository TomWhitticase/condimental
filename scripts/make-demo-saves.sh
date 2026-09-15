#!/usr/bin/env bash
# Regenerates the demo save files that the main menu's Load Game lists.
#
# Usage: scripts/make-demo-saves.sh
#
# Writes one save per demo layout ("demo line", "full chain", "belt gallery",
# "pipe gallery") into Godot's `user://saves/`, each built out of real
# Purchase/Place commands by godot_project/test_layouts.gd and each proven to
# load back before it is written. See godot_project/make_demo_saves.gd.
#
# Run this after any change to the save schema (`sim_core/src/sim/save.rs`'s
# `schema_version`): a save is rejected, not migrated, on a mismatch
# (ARCHITECTURE.md §4), so every existing demo save stops loading the moment
# the version moves and there is no other way to rebuild them.
#
# The saves live in Godot's per-user data directory, NOT in the repo — they
# are local demo content, not committed fixtures (the fixtures are the
# builders in test_layouts.gd, which is what this regenerates them from).
set -euo pipefail

cd "$(dirname "$0")/.."

godot_bin="${GODOT_BIN:-godot}"
if ! command -v "$godot_bin" >/dev/null 2>&1; then
  echo "error: '$godot_bin' not found on PATH (set GODOT_BIN to override)" >&2
  exit 1
fi

if [ ! -f godot_project/bin/libsim_core.dylib ] \
  && [ ! -f godot_project/bin/libsim_core.so ] \
  && [ ! -f godot_project/bin/sim_core.dll ]; then
  echo "error: no GDExtension library in godot_project/bin — run \`cargo xtask\` first" >&2
  exit 1
fi

# The script prints its own success marker: Godot exits 0 even when a script
# fails to parse, so the marker is the only trustworthy signal (the same rule
# scripts/verify-core-loop.sh applies to every headless proof).
out=$("$godot_bin" --headless --path godot_project --script make_demo_saves.gd 2>&1)
status=$?

echo "$out"

if [ $status -ne 0 ] || ! grep -q "DEMO_SAVES_OK" <<<"$out"; then
  echo "error: demo saves were not written (godot exited $status)" >&2
  exit 1
fi
