#!/usr/bin/env bash
# Headless proof of milestone 3 (ARCHITECTURE.md §9 item 3, "Core loop
# proof"). Three layers, cheapest/fastest first:
#
#   1. `cargo test -p sim_core` — the real feedback loop (ARCHITECTURE.md
#      §6/§7): unit tests per component plus an integration test that ticks
#      the belt+machine+recipe for thousands of ticks and asserts strict
#      item conservation. No Godot involved at all.
#   2. `cargo xtask` builds the extension and copies it into
#      godot_project/bin/.
#   3. Two headless Godot scripts exercise the parts cargo test can't: the
#      real `gdext` boundary (JSON command/snapshot wire format going
#      through Godot's own Variant/String/JSON machinery) and the
#      interactive scene's keyboard input actually reaching SimBridge.
#
# Usage: scripts/verify-core-loop.sh

set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

godot_bin="${GODOT_BIN:-godot}"
if ! command -v "$godot_bin" >/dev/null 2>&1; then
  echo "error: '$godot_bin' not found on PATH (set GODOT_BIN to override)" >&2
  exit 1
fi

echo "==> cargo test -p sim_core (headless, no Godot — ARCHITECTURE.md §7)"
cargo test -p sim_core || exit 1
echo

echo "==> cargo xtask (build sim_core, copy extension into godot_project/bin/)"
cargo xtask || exit 1
echo

cache_file="godot_project/.godot/extension_list.cfg"
if [ ! -f "$cache_file" ] || ! grep -q "sim_core.gdextension" "$cache_file"; then
  echo "==> priming Godot's import/UID cache (one-time; see scripts/verify-scaffold.sh's comment)"
  "$godot_bin" --headless --path godot_project --import >/tmp/godot-import.log 2>&1
  if [ ! -f "$cache_file" ] || ! grep -q "sim_core.gdextension" "$cache_file"; then
    echo "error: $cache_file missing sim_core.gdextension after import" >&2
    cat /tmp/godot-import.log >&2
    exit 1
  fi
  echo
fi

echo "==> godot --headless --script test_core_loop.gd (real gdext JSON boundary)"
"$godot_bin" --headless --path godot_project --script test_core_loop.gd
core_loop_status=$?
if [ "$core_loop_status" -ne 0 ]; then
  echo "FAIL: test_core_loop.gd exited $core_loop_status" >&2
  exit "$core_loop_status"
fi
echo

echo "==> godot --headless --script test_interaction.gd (keyboard input -> SimBridge)"
"$godot_bin" --headless --path godot_project --script test_interaction.gd
interaction_status=$?
if [ "$interaction_status" -ne 0 ]; then
  echo "FAIL: test_interaction.gd exited $interaction_status" >&2
  exit "$interaction_status"
fi

echo
echo "PASS: milestone 3 core loop verified headlessly (sim_core tests + gdext boundary + interaction)."
