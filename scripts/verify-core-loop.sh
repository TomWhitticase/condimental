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
#   3. Thirteen headless Godot scripts exercise the parts cargo test can't: the
#      real `gdext` boundary (JSON command/snapshot/layout wire format going
#      through Godot's own Variant/String/JSON machinery, driven through the
#      whole chain end to end), the interactive scene's keyboard input
#      actually reaching SimBridge, the visual pass's structural claims
#      (orthographic isometric camera, real-time lighting, imported voxel
#      meshes, metal shinier than matte — docs/UI_UX.md §1/§2/§4), camera
#      pan/zoom (§1: allowed, unlike free rotation of the fixed angle), and
#      UI_UX.md §5's diegetic UI epic — the contract clipboard panel (now one
#      docket per active contract, since contracts became data), voxel-
#      rendered icons, and machine hover tooltips (hover, not dense
#      always-on text) — plus milestone 6's fluid presentation (pipe tiles
#      rendered from the simulation's own segment list, liquid bodies scaled
#      to live pressure, pipe hover info in both view modes), and the
#      "Machines improvement" milestone's presentation (a bespoke model per
#      machine kind, a multi-tile machine actually covering its footprint,
#      the hopper drawn with a live product level, and hover resolving a
#      non-origin footprint tile), and the "Shop & Inventory" epic's build
#      menu (a grid of voxel-rendered icon slots built from
#      shop_snapshot_json, toggled with B, affordability as a visible slot
#      state, and a real purchase moving the displayed owned count), and
#      milestone 7's aging rack (a bespoke model at the tile the simulation
#      reports, and a progress gauge whose bar tracks a 2400-tick process
#      recomputed from the live tick rather than from a chunk snapshot —
#      which is the only way a process that dirties nothing for a minute at
#      a time can be shown moving at all), and the delivery dock that now ends
#      the line (its own bespoke model at the tile the simulation reports, the
#      one belt in the world that belongs to no stage drawn from its snapshot,
#      the goods on its deck following the economy's shared delivery pool, its
#      own hover branch — and the removed **C** collect-product control being
#      gone from both the legend and the key handler), and the "Real
#      Placement" epic's ticket 2 (an EMPTY starting world rendering correctly,
#      a translucent ghost with footprint-aware validity signalled by glyph
#      and word as well as colour, all four facings drawn with the matching
#      rotation, the junction art's east-only fallback holding under real
#      placement, and place/remove/rotate driving the rendered scene through
#      the real command boundary).
#
# NOTE ON WHAT THESE TESTS BUILD. Since that epic a world starts EMPTY, so
# every script below builds the factory it then asserts against, using real
# purchase/place commands (godot_project/test_layouts.gd). What they can build
# is bounded by `sim::STARTING_MONEY`; the shipped six-stage line costs several
# times that and stays asserted where it always was, in `cargo test -p sim_core`
# against `sim::reference_chain`. See test_layouts.gd's header.
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

# The voxel generators have tests of their own since the "Visual overhaul
# round 2" epic's ticket 3: a machine's inlet/outlet has to be a genuinely
# EMPTY opening, and a mesh (or a screenshot) cannot tell a carved hole from
# a rectangle painted black — only the voxel volume can. See
# assets/voxels/generators/assets.rs's "Junction tests" section.
echo "==> cargo test -p voxgen (voxel geometry: machine inlets/outlets are real openings)"
cargo test -p voxgen || exit 1
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

# Godot exits 0 even when a `--script` file fails to PARSE — it prints the
# parse error and quits cleanly — so exit status alone is NOT proof a test
# actually ran. This was found the hard way: a freshly added test_fluids.gd
# with a type-inference error never loaded, and this script happily printed
# PASS. Every script below therefore also has to print its own success
# marker, and its absence is a failure.
run_godot_script() {
  script="$1"
  marker="$2"
  label="$3"
  echo "==> godot --headless --script $script ($label)"
  out=$("$godot_bin" --headless --path godot_project --script "$script" 2>&1)
  status=$?
  echo "$out"
  if [ "$status" -ne 0 ]; then
    echo "FAIL: $script exited $status" >&2
    exit "$status"
  fi
  if ! printf '%s\n' "$out" | grep -q "$marker"; then
    echo "FAIL: $script exited 0 but never printed '$marker' — it did not actually run" >&2
    exit 1
  fi
  echo
}

run_godot_script test_core_loop.gd CORE_LOOP_OK "real gdext JSON boundary"
run_godot_script test_interaction.gd INTERACTION_OK "keyboard input -> SimBridge"
run_godot_script test_presentation.gd PRESENTATION_OK "UI_UX.md §1/§2/§4 scene structure"
run_godot_script test_camera_nav.gd CAMERA_NAV_OK "pan/zoom, UI_UX.md §1"
run_godot_script test_contract_panel.gd CONTRACT_PANEL_OK "contract clipboard, one docket per contract, UI_UX.md §5"
run_godot_script test_icons.gd ICONS_OK "_icon_for, UI_UX.md §5"
run_godot_script test_hover.gd HOVER_OK "machine hover tooltips, UI_UX.md §5"
run_godot_script test_fluids.gd FLUIDS_OK "pipe rendering + fluid hover, milestone 6"
run_godot_script test_machines.gd MACHINES_OK "per-kind machine models, multi-tile footprints, hopper fill"
run_godot_script test_connections.gd CONNECTIONS_OK "belt/pipe-to-machine junctions, visual overhaul round 2"
run_godot_script test_shop_menu.gd SHOP_MENU_OK "build menu grid + purchases, UI_UX.md §5"
run_godot_script test_timed_processes.gd TIMED_PROCESSES_OK "aging rack + live progress gauge, milestone 7"
run_godot_script test_placement.gd PLACEMENT_OK "empty world, ghost, rotation, place/remove — Real Placement ticket 2"

echo
echo "PASS: real placement + milestone 3/4 core loop + full chain + visual pass + camera nav + contract clipboard"
echo "      + icons + hover + milestone 6 fluid presentation + machine footprints/hopper"
echo "      + belt/pipe junctions + the build menu + milestone 7's aging rack and its"
echo "      progress gauge + the delivery dock that ends the line, all verified"
echo "      headlessly (sim_core tests"
echo "      + gdext boundary + interaction + 3D scene structure + pan/zoom + contract panel"
echo "      structure + icons + hover tooltips + pipe/fluid rendering + per-kind machine"
echo "      models and the hopper's fill + shop slots/affordability/purchases + the"
echo "      rack's live progress bar + the dock's model/belt/deck stock/hover)."
