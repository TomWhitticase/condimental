#!/usr/bin/env bash
# Headless proof of milestone 3 (ARCHITECTURE.md §9 item 3, "Core loop
# proof") and milestone 4 (§9 item 4, "First full chain") — the same three
# layers verify both, since milestone 4 only extended the single-belt/
# single-machine layout into a multi-stage chain, not the verification
# approach itself. Three layers, cheapest/fastest first:
#
#   1. `cargo test -p sim_core` — the real feedback loop (ARCHITECTURE.md
#      §6/§7): unit tests per component plus an integration test that ticks
#      the whole multi-stage chain (belts+machines+recipes, per
#      data/recipes.ron) for thousands of ticks and asserts strict item
#      conservation across every stage, not just one. No Godot involved.
#   2. `cargo xtask` builds the extension and copies it into
#      godot_project/bin/.
#   3. Four headless Godot scripts exercise the parts cargo test can't: the
#      real `gdext` boundary (JSON command/snapshot/layout wire format going
#      through Godot's own Variant/String/JSON machinery, driven through the
#      whole chain end to end), the interactive scene's keyboard input
#      actually reaching SimBridge, the visual pass's structural claims
#      (orthographic isometric camera, real-time lighting, imported voxel
#      meshes, metal shinier than matte — docs/UI_UX.md §1/§2/§4), and camera
#      pan/zoom (§1: allowed, unlike free rotation of the fixed angle).
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

# Godot's import pass has to run TWICE on a cold godot_project/.godot/ cache,
# and this is not obvious enough to leave implicit: the .vox importer is an
# *editor plugin*, and a plugin is only registered on the pass after the one
# that first scans the project. On the first run Godot therefore doesn't yet
# know .vox is an importable extension and silently skips every voxel model;
# the second run imports all of them. Running it unconditionally (it is
# incremental and cheap once warm) also picks up whatever `cargo xtask` just
# re-staged into godot_project/voxels/.
echo "==> godot --headless --import (twice: the .vox importer is a plugin, see comment)"
"$godot_bin" --headless --path godot_project --import >/tmp/godot-import.log 2>&1
"$godot_bin" --headless --path godot_project --import >>/tmp/godot-import.log 2>&1

cache_file="godot_project/.godot/extension_list.cfg"
if [ ! -f "$cache_file" ] || ! grep -q "sim_core.gdextension" "$cache_file"; then
  echo "error: $cache_file missing sim_core.gdextension after import" >&2
  cat /tmp/godot-import.log >&2
  exit 1
fi

# Every staged .vox must have produced an .import file, or the scene would
# quietly render nothing and every test below would still pass.
vox_count=$(ls godot_project/voxels/*.vox 2>/dev/null | wc -l | tr -d ' ')
imported_count=$(ls godot_project/voxels/*.vox.import 2>/dev/null | wc -l | tr -d ' ')
if [ "$vox_count" = "0" ] || [ "$vox_count" != "$imported_count" ]; then
  echo "error: $imported_count of $vox_count voxel models imported — see /tmp/godot-import.log" >&2
  exit 1
fi
echo "imported $imported_count/$vox_count voxel models"
echo

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
echo "==> godot --headless --script test_presentation.gd (UI_UX.md §1/§2/§4 scene structure)"
"$godot_bin" --headless --path godot_project --script test_presentation.gd
presentation_status=$?
if [ "$presentation_status" -ne 0 ]; then
  echo "FAIL: test_presentation.gd exited $presentation_status" >&2
  exit "$presentation_status"
fi

echo
echo "==> godot --headless --script test_camera_nav.gd (pan/zoom, UI_UX.md §1)"
"$godot_bin" --headless --path godot_project --script test_camera_nav.gd
camera_nav_status=$?
if [ "$camera_nav_status" -ne 0 ]; then
  echo "FAIL: test_camera_nav.gd exited $camera_nav_status" >&2
  exit "$camera_nav_status"
fi

echo
echo "PASS: milestone 3/4 core loop + full chain + visual pass + camera nav verified headlessly"
echo "      (sim_core tests + gdext boundary + interaction + 3D scene structure + pan/zoom)."
