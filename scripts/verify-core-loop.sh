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
#   3. Every headless Godot script under godot_project/ exercises the parts
#      cargo test can't (the count is printed at the end of the run, so this
#      header has no number to drift): the
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
#      state, and a real purchase moving the displayed owned count) plus the
#      always-visible inventory hotbar built from the same snapshot (number
#      keys 1-9/0 selecting a slot into placement mode with no menu open,
#      pressing the same key again to deselect, empty sockets showing no
#      count, and buying still a separate verb), and
#      the chain rework's own presentation (the 1x2 mixer with a belt lane per
#      solid ingredient, the two machine-to-machine pipe runs that replaced
#      the belts through the middle of the line, the hopper's contents chosen
#      at placement time, and the ingredient market that stock is bought in),
#      and the delivery dock that now ends
#      the line (its own bespoke model at the tile the simulation reports, the
#      one belt in the world that belongs to no stage drawn from its snapshot,
#      the goods on its deck following that dock's own stock, its
#      own hover branch — and the removed **C** collect-product control being
#      gone from both the legend and the key handler), and the "Real
#      Placement" epic's ticket 2 (an EMPTY starting world rendering correctly,
#      a translucent ghost with footprint-aware validity signalled by glyph
#      and word as well as colour, all four facings drawn with the matching
#      rotation, the junction art's east-only fallback holding under real
#      placement, and place/remove/rotate driving the rendered scene through
#      the real command boundary), and save/load's own boundary (a whole RON
#      save crossing as a String in both directions, an earlier moment coming
#      back exactly, a refused load saying why and changing nothing, and New
#      Game emptying the floor), and the MAIN MENU and PAUSE MENU built on top
#      of it (menu.tscn's three buttons and its save list read from
#      `user://saves/`, a chosen save coming back as the same factory through
#      the `GameSession` autoload, a corrupt one refused on the menu with
#      `load_ron`'s own reason, and — in the game — ESC opening a pause menu
#      that genuinely stops the simulation, a named save slot, the reserved
#      autosave slot, the three-minute autosave timer and Quit to Menu).
#
# NOTE ON WHAT THESE TESTS BUILD. Since that epic a world starts EMPTY, so
# every script below builds the factory it then asserts against, using real
# purchase/place commands (godot_project/test_layouts.gd). Those fixtures stay
# small on purpose — a nine-tile line proves a wire format and a scene graph
# exactly as well as a forty-tile one, and much faster — not because of what a
# starting balance can afford: `sim::STARTING_MONEY` is 3000 against a ~1,200
# bill for the shipped line since the inventory-hotbar ticket, so
# `test_layouts.gd`'s `build_reference_line` can and does build the whole
# thing. Full-chain BEHAVIOUR stays asserted where it always was, in
# `cargo test -p sim_core` against `sim::reference_chain`.
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
godot_scripts_run=0

run_godot_script() {
  script="$1"
  marker="$2"
  label="$3"
  godot_scripts_run=$((godot_scripts_run + 1))
  echo "==> godot --headless --script $script ($label)"
  out=$("$godot_bin" --headless --path godot_project --script "$script" 2>&1)
  status=$?
  echo "$out"
  if [ "$status" -ne 0 ]; then
    echo "FAIL: $script exited $status" >&2
    exit "$status"
  fi
  # A layout fixture that could not build what it promised (`test_layouts.gd`'s
  # `fail_fixture`) prints `<NAME>_FIXTURE_FAIL:` and carries on, and a caller
  # that never reads `Layouts.fixture_failure()` would still print its own
  # success marker over a broken fixture. The marker is the failure.
  if printf '%s\n' "$out" | grep -q "_FIXTURE_FAIL:"; then
    echo "FAIL: $script printed a _FIXTURE_FAIL: marker — its fixture did not build" >&2
    exit 1
  fi
  if ! printf '%s\n' "$out" | grep -q "$marker"; then
    echo "FAIL: $script exited 0 but never printed '$marker' — it did not actually run" >&2
    exit 1
  fi
  echo
}

# `test_screenshot.gd` is the one script in godot_project/ that no headless
# test loads: it is driven by scripts/screenshot-scene.sh, which needs a
# window, so nothing below would notice it had stopped PARSING. A merge that
# renamed a `_process` parameter under it did exactly that, and every check in
# this repo stayed green while the review render was dead. `--check-only`
# parses without running, and reports the failure the way Godot always does —
# by printing it, not by exiting non-zero.
echo "==> godot --check-only --script test_screenshot.gd (the review harness still parses)"
parse_out=$("$godot_bin" --headless --path godot_project --check-only \
  --script test_screenshot.gd 2>&1)
if printf '%s\n' "$parse_out" | grep -qE "Parse Error|Failed to load script"; then
  printf '%s\n' "$parse_out" >&2
  echo "FAIL: test_screenshot.gd does not parse — scripts/screenshot-scene.sh cannot render" >&2
  exit 1
fi
echo "test_screenshot.gd parses"
echo

run_godot_script test_core_loop.gd CORE_LOOP_OK "real gdext JSON boundary"
run_godot_script test_interaction.gd INTERACTION_OK "keyboard input -> SimBridge"
run_godot_script test_presentation.gd PRESENTATION_OK "UI_UX.md §1/§2/§4 scene structure"
run_godot_script test_camera_nav.gd CAMERA_NAV_OK "pan/zoom, UI_UX.md §1"
run_godot_script test_ctrl_cursor.gd CTRL_CURSOR_OK "hold CTRL in first person: the mouse goes to the HUD, look stops, a click selects a hotbar slot and never builds, release recaptures without a jump — 2026-10-02"
run_godot_script test_contract_panel.gd CONTRACT_PANEL_OK "contract clipboard, one docket per contract, UI_UX.md §5"
run_godot_script test_contract_board.gd CONTRACT_BOARD_OK "contracts board (C), signing, strikes, loss and re-signing — the contract-board epic"
run_godot_script test_icons.gd ICONS_OK "_icon_for, UI_UX.md §5"
run_godot_script test_hover.gd HOVER_OK "machine hover tooltips, UI_UX.md §5"
run_godot_script test_fluids.gd FLUIDS_OK "pipe rendering + fluid hover, milestone 6"
run_godot_script test_machines.gd MACHINES_OK "per-kind machine models, multi-tile footprints, hopper fill"
run_godot_script test_connections.gd CONNECTIONS_OK "belt-to-building junctions at every facing, retired belt variants gone"
run_godot_script test_shop_menu.gd SHOP_MENU_OK "build menu grid + purchases, UI_UX.md §5"
run_godot_script test_shop_search.gd SHOP_SEARCH_OK "build menu category sections + search box"
run_godot_script test_hotbar.gd HOTBAR_OK "always-visible hotbar, number-key selection, UI_UX.md §5"
run_godot_script test_ui_scale.gd UI_SCALE_OK "one UI_SCALE knob, no pre-scale label, everything fits the window"
run_godot_script test_hud_plates.gd HUD_PLATES_OK "the right-hand STOCK plate and the collapsible CONTROLS legend — one row per raw ingredient clear of the clipboard and hotbar, the legend folded by its button and by F1, and every cursor panel reading the plates' current rects — plus first person parking a world panel in the corner instead of on the crosshair"
run_godot_script test_chain_rework.gd CHAIN_REWORK_OK "1x2 mixer, piped middle, hopper contents, ingredient market"
run_godot_script test_config_panel.gd CONFIG_PANEL_OK "click a hopper or tank to configure it, UI_UX.md §5"
run_godot_script test_placement.gd PLACEMENT_OK "empty world, ghost, rotation, place/remove — Real Placement ticket 2"
run_godot_script test_splitters.gd SPLITTERS_OK "splitter model, next-output arrow, items crossing, hover — Epic B ticket 2"
run_godot_script test_crossings.gd CROSSINGS_OK "belt/pipe crossing: gantry over the belt, items under it, lifted fluid, hover — crossing ticket 3"
run_godot_script test_save_load.gd SAVE_LOAD_OK "save_ron/load_ron/new_game across the real boundary — save/load ticket 1"
run_godot_script test_main_menu.gd MAIN_MENU_OK "menu.tscn, the save list, the GameSession handoff — save/load ticket 2"
run_godot_script test_pause_menu.gd PAUSE_MENU_OK "ESC pause menu, save slots, autosave, quit to menu — save/load ticket 2"
run_godot_script test_perf.gd PERF_OK "instanced belts/pipes/items on the 1500-tile stress line, rebuild + 120 frames timed against ceilings scaled by measured machine load — pipes & conveyors ticket 4"
run_godot_script test_pipe_shapes.gd PIPE_SHAPES_OK "pipe shape, turn, fittings and fluid body from links; pipe hover — pipes & conveyors ticket 6"
run_godot_script test_belt_shapes.gd BELT_SHAPES_OK "belt shape from links at every facing, frame from tick, corner item arcs, belt hover — pipes & conveyors ticket 5"
run_godot_script test_payout_feedback.gd PAYOUT_FEEDBACK_OK "the payout celebration: coins, the money punch, the PAID stamp and the sound — payout juice"
run_godot_script test_machine_motion.gd MACHINE_MOTION_OK "machine motion overlays: frames per kind, frame from the machine's own elapsed ticks, and motion only while it is actually working"
run_godot_script test_machine_load.gd MACHINE_LOAD_OK "machine load in the hover panel (glyph, word, status-coloured bar), polled on the tick, right on the first frame of a load, and NO poles on the floor — the machine-load epic, ticket 3 as amended; a red person over a machine refused its crew"
run_godot_script test_staff_panel.gd STAFF_PANEL_OK "the staff panel (H): rows from staff_snapshot_json, HIRE/FIRE through the real commands, an understaffed machine and dock in hover, one modal at a time — the staff pools MVP, ticket 3"
run_godot_script test_production_stats.gd PRODUCTION_STATS_OK "the production panel (P): rows sorted and formatted from production_snapshot_json, windows and all time, a running line counted, one modal at a time, polled only while open — the production-stats epic"
run_godot_script test_audio.gd AUDIO_OK "sound effects for every player action and the looping music track: streams deterministic and unclipped, cues fired from the handlers that know the outcome, the score ducked under the pause menu — audio, 2026-09-12"
run_godot_script test_settings.gd SETTINGS_OK "the settings screen: eight knobs at their defaults, every stepper clamped and snapped to its own grid, a real user://settings.cfg round-tripped (and the developer's own put back), a hand-edited file pinned rather than trusted, both volumes and both switches reaching the audio owners, movement/look/FOV/hall reaching the camera and the sky, and the main menu and the pause menu showing the same rows — the settings + debug menu branch, 2026-09-21"
run_godot_script test_debug_menu.gd DEBUG_MENU_OK "the F3 debug menu: a panel that is not a modal and does not pause, GIVE MONEY moving the simulation's own balance by exactly its amount over the real debug_grant_money wire, time speed scaling ticks per frame, FROZEN stopping them without pausing, STEP spending exactly one, and the readout tracking the sim — the settings + debug menu branch, 2026-09-21"
run_godot_script test_bbq_branch.gd BBQ_BRANCH_OK "the third sauce, as a branch: the BBQ line built beside the shipped one through real commands, every pipe run named, a mixed order on the clipboard and a box of BBQ made and delivered — the BBQ epic, ticket 2"
run_godot_script test_mustard_line.gd MUSTARD_OK "the second sauce, as a whole second line from raw: seed and vinegar into a crusher that discharges a FLUID, turmeric in at the mixer, all four runs named by topology alone, a case made and delivered — and every id on mustard's own cookbook page resolving to a model or a tint of its own rather than to a documented fallback, with the three piped stages a measured step of value apart — the mustard epic, M3"
run_godot_script test_pipe_fault.gd PIPE_FAULT_OK "a run the simulation cannot name says so: hazard clamps on the dead run and not on an empty one, the hover naming both fluids it could carry, the vat blaming the run rather than claiming there is no pipe, and every clamp coming down when the tank is turned back — issue #54"
run_godot_script test_undecided_machines.gd UNDECIDED_OK "a machine nothing has fed is drawn as handling any sauce, not ketchup: on a freshly built line every multi-recipe machine's hover is titled by its kind and 'any sauce', names none of the recipes the cookbook gives that kind, and says it is waiting for ingredients; its configuration panel says the same; its product mesh carries no tint; and once fed the vat names its recipe and takes one — owner's request, 2026-10-02"
run_godot_script test_hover_targets.gd HOVER_TARGETS_OK "what each --hover* screenshot flag aims at, and what happens when it is not there: every flag finds nothing in an empty world and REFUSES in words that name the flag and the world it looked in, --hover-splitter still finds nothing in the full_dock line (the reported case), and every flag lands on a placement of its own kind in a fixture that has one — the tooling pass, PR #78"
run_godot_script test_unlocks.gd UNLOCKS_OK "the first progression gate: a new game's market, larder and board list only what data/unlocks.ron has released, crossing the threshold grows all three in place and is announced once by name, a save already past it is not re-announced — the BBQ epic, ticket 3"
run_godot_script test_ingredient_models.gd INGREDIENT_MODELS_OK "every id in data/ingredients.ron has a voxel model of its own: none reaches _market_asset_for's pipe fallback, fluids are registered in FLUID_MODELS and solids in ITEM_MODELS, and each named asset has a real icon rather than the placeholder — issue #67"
run_godot_script test_fluid_gallery.gd FLUID_GALLERY_OK "the fluid gallery stands every fluid the content carries at once: every placement accepted, every run named by topology alone and above the pressure a sight window shows colour at, every tint present, distinct and not near-grey, and the set of tints checked against the wire rather than a list in the test — issues #72, #100 and #108"
run_godot_script test_warehouse_rack.gd WAREHOUSE_RACK_OK "the warehouse stock rack reads out its own stock: six shelves filling bottom level first inside the imported frame, pallets appearing one bay at a time in step with storage_snapshot_json and their loads scaled to the fill, a tint per raw ingredient clear of the measured dead band, hover resolving from either tile of the 1x2, and W1's removal refusal spoken before the click — the warehouse sprint, W3"
run_godot_script test_rack_filter.gd RACK_FILTER_OK "a warehouse rack opens its own configuration panel and its picker reserves it for one raw: any plus every raw the wire lists, the pick followed by title, tick and hover, a pick the sim would refuse for want of room dimmed with a ✗ and explained before the click, Any clearing it, a padlocked raw refused, and removal's refusal read off the rack's own snapshot, even for an open rack the floor-wide bay sum would call spare"
run_godot_script test_frame_clock.gd FRAME_CLOCK_OK "the HUD fades and the screenshot warm-up run on FRAMES, not the wall clock: _hud_now_msec() does not move while real milliseconds pass with no frames, all three fades finish on frames alone, and WARMUP_FRAMES at the harness's pinned FIXED_FPS always spends the same ticks — issue #82"
run_godot_script test_camera_seal.gd CAMERA_SEAL_OK "a review render's camera cannot be moved by the machine it runs on: sealed, all five ways live input moves the camera do nothing, and every one of them still moves it unsealed — the second half of issue #82"
run_godot_script test_payday_warning.gd PAYDAY_WARNING_OK "the payday warning: the simulation answers whether a spend would leave the next payday unpayable and Godot only renders that answer, a spend that crosses the line and one made while already short each raising the confirmation with its own heading, CANCEL leaving money/stock/world untouched and SPEND ANYWAY issuing the command, ESC cancelling without reaching the panel behind it, all four spends plus SHIFT's replacement purchase gated, and a hire the payday cannot cover warning beside a live button — the payday warning, 2026-09-26"
run_godot_script test_cookbook.gd COOKBOOK_OK "THE COOKBOOK (B): the sauce tree drawn from cookbook_json into the canvas meta a headless run can read, a locked family as a silhouette with its gate and nothing else, pages that are their wire entry in seconds, the pages linked, no id or name in the whole book written anywhere in cookbook_panel.gd, one modal at a time, and the same panel hosted by the pause menu — THE COOKBOOK"

echo
echo "$godot_scripts_run headless Godot scripts run."
echo "PASS: real placement + milestone 3/4 core loop + full chain + visual pass + camera nav + contract clipboard"
echo "      + icons + hover + milestone 6 fluid presentation + machine footprints/hopper"
echo "      + belt/pipe junctions + the build menu + the inventory hotbar + the chain rework's"
echo "      piped middle, 1x2 mixer, hopper-contents choice and ingredient market"
echo "      + the delivery dock that ends the line + save/load across the boundary"
echo "      + the main menu, save slots, autosave and the pause menu"
echo "      + instanced belts/pipes/items timed on the stress line"
echo "      + pipe shapes, fittings and fluid bodies from the wire's links,"
echo "      + belt shapes from links, animation frames, corner arcs and belt hover,"
echo "      + the payout celebration a real contract payout fires, and its decay to nothing,"
echo "      + mustard as a second line from raw and BBQ as a branch off ketchup, both shipping,"
echo "      + the warehouse stock rack drawing the raw stock it is holding, bay by bay,"
echo "      all verified"
echo "      headlessly (sim_core tests"
echo "      + gdext boundary + interaction + 3D scene structure + pan/zoom + contract panel"
echo "      structure + icons + hover tooltips + pipe/fluid rendering + per-kind machine"
echo "      models and the hopper's fill + shop slots/affordability/purchases + the"
echo "      market's crates and the larder + the dock's model/belt/deck stock/hover"
echo "      + save_ron/load_ron/new_game + menu.tscn/the save list/the pause menu)."
