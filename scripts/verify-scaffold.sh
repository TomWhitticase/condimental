#!/usr/bin/env bash
# Headless proof of the milestone-1 Rust (gdext) <-> Godot round trip
# (ARCHITECTURE.md §7, §9). No editor GUI, nothing to look at by hand:
#
#   1. `cargo xtask` builds sim_core and copies the extension into
#      godot_project/bin/.
#   2. Godot needs its local import/UID cache (godot_project/.godot/,
#      gitignored — it's a regenerated cache, not a source file) before it
#      will recognize a GDExtension outside the editor. `godot --import`
#      generates it. NOTE: on this Godot 4.6.3 build, the editor process
#      crashes with SIGSEGV *after* writing that cache, while generating
#      GDExtension docs during its own shutdown — a known upstream editor
#      bug (see e.g. godotengine/godot#115897), unrelated to sim_core. We
#      only care that the cache file it produces is correct, so step 2's
#      own exit code is not treated as fatal; we verify the cache directly
#      instead.
#   3. `godot --headless --script test_round_trip.gd` then runs with no
#      editor involved at all, calls into Rust via SimBridge.ping(), and
#      exits non-zero if the round trip didn't produce the expected value.
#      This step's exit code is the real pass/fail signal.
#
# Usage: scripts/verify-scaffold.sh

set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

godot_bin="${GODOT_BIN:-godot}"
if ! command -v "$godot_bin" >/dev/null 2>&1; then
  echo "error: '$godot_bin' not found on PATH (set GODOT_BIN to override)" >&2
  exit 1
fi

echo "==> cargo xtask (build sim_core, copy extension into godot_project/bin/)"
cargo xtask
echo

cache_file="godot_project/.godot/extension_list.cfg"
if [ ! -f "$cache_file" ] || ! grep -q "sim_core.gdextension" "$cache_file"; then
  echo "==> priming Godot's import/UID cache (one-time; see comment above)"
  "$godot_bin" --headless --path godot_project --import >/tmp/godot-import.log 2>&1
  import_status=$?
  if [ ! -f "$cache_file" ] || ! grep -q "sim_core.gdextension" "$cache_file"; then
    echo "error: $cache_file missing sim_core.gdextension after import (exit $import_status)" >&2
    cat /tmp/godot-import.log >&2
    exit 1
  fi
  echo "cache primed ($cache_file registers sim_core.gdextension)"
  echo
fi

echo "==> godot --headless --script test_round_trip.gd"
"$godot_bin" --headless --path godot_project --script test_round_trip.gd
test_status=$?

if [ "$test_status" -ne 0 ]; then
  echo "FAIL: round-trip test exited $test_status" >&2
  exit "$test_status"
fi

echo
echo "PASS: Rust <-> Godot round trip verified headlessly."
