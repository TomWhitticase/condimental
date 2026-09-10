#!/usr/bin/env bash
# Renders godot_project/main.tscn to a PNG for visual review (docs/UI_UX.md).
#
# This is the one verification step in the project that is NOT headless, and
# deliberately so: everything else about this codebase is checked without a
# framebuffer (ARCHITECTURE.md §7), but "does the factory look like UI_UX.md
# describes" cannot be. It opens a real window briefly, waits for lighting to
# settle, writes the frame, and exits.
#
# **THE WORLD NOW STARTS EMPTY** (the "Real Placement" epic). With no flags
# at all this photographs exactly that — bare floor, a build menu key and
# nothing else — which is a real state worth reviewing, not a broken one.
#
# Pass --build to lay down the demo line first: a hopper feeding belts that
# run east, TURN NORTH into a crusher facing WEST which discharges back west,
# with a pipe run heading SOUTH beside it. Three of the four facings on belts
# and the fourth on the pipes, which makes it the right subject for reviewing
# per-tile rotation and the junction art's east-only fallback in one frame.
# Pass --fixture <machines|fluids|vat|splitter|chain|stress|pipes|gallery|demo> for a different one:
# `machines` (a hopper, a machine and a dock), `fluids` (a tank, a pipe run
# and the machine tapping it), `vat` (the 2x2 cooking vat alone), `splitter`
# (a line that FORKS — one splitter with both outputs built and a second with
# only its straight one, so the next-output arrow's two behaviours are in one
# frame) or `chain` (THE WHOLE SHIPPED LINE — about 1,200 of a new game's
# 3,000 balance, bought and placed like anything else, with its piped middle
# and its two hoppers).
#
# Pass --contracts to build that full line and then run it until the contract
# board is genuinely mixed (some deliveries made, some missed, goods on the
# dock), so the clipboard can be reviewed against a working economy. It came
# back with `sim::STARTING_MONEY`'s rise from 200 to 3000: under the old
# balance no fixture could reach a factory that earns anything.
#
# Pass --stock to buy an assorted kit (belts, pipes, a hopper, a tank, a
# machine or two) so the hotbar shows carried sockets, multi-digit counts and
# empty sockets side by side, and --hotbar N to press hotbar key N for real —
# the only way to review the selected-slot state, the name flyout and the
# ghost it puts in the player's hand.
#
# Pass --stress to build and run the STRESS LINE (a 1500-tile serpentine belt
# fed by hoppers, plus three tank-fed pipe runs — test_layouts.gd's
# `build_stress_line`), the fixture the instanced tile/item rendering is
# measured on. It implies --populate, and the run prints a SCREENSHOT_RENDER
# line with the frame's object, primitive and draw-call counts.
#
# Pass --pipe-gallery to build the PIPE GALLERY (test_layouts.gd's
# `build_pipe_gallery`): every pipe shape at every turn, a tank discharging
# into a pipe from each of four sides, a machine tapping a pipe from each
# side, and the shipped mixer -> vat -> bottler runs — the fixture the pipe
# presentation (the pipes & conveyors overhaul, ticket 6) is reviewed on.
# Add --populate to see liquid standing in the fed runs, and --closeup for
# any one joint; a whole-gallery frame is too small to judge a flange by.
# Pass --belt-gallery to build the BELT GALLERY (test_layouts.gd's
# `build_belt_gallery`): both hands of corner at all four facings, a belt
# meeting a hopper, a crusher and a dock at all four facings, a two-flank
# merge, a line start and a line end — every belt shape the pipes and
# conveyors overhaul draws, in one frame. Combine with --populate to see items
# following the corners' arcs, and with --closeup on one of its corners
# (they are at (0,0), (3,0), (3,3), (0,3) for right turns and (6,0), (9,0),
# (9,3), (6,3) for left) to judge one junction.
#
# Pass --populate to run whatever was built forward first, so the shot shows
# product moving instead of an idle factory. Pass --first-person to review
# that camera mode's lighting/skybox instead of the default isometric one.
#
# Pass --ghost KIND to enter placement mode with the ghost over open ground
# (the VALID state), or --ghost-blocked KIND with it over something already
# placed (the INVALID state), or --remove-mode to enter remove mode over a
# placed building. These are the only way to review build mode without a live
# interactive session — the same problem the hover flags solve for tooltips.
#
# Pass one of --hover / --hover-pipe / --hover-vat / --hover-hopper /
# --hover-tank / --hover-dock / --hover-splitter to park the cursor over the
# first placed machine, pipe, multi-tile machine's FAR CORNER, hopper, fluid
# tank, dock or conveyor splitter,
# so that tooltip is in the shot. (--hover-vat deliberately picks a
# non-origin tile: "any tile of a multi-tile machine identifies it" is the
# claim, and hovering the origin would not show it.)
#
# Pass --menu to photograph the MAIN MENU instead of the game — it renders
# menu.tscn, having first written a real save so its load list has a row in
# it, and removes that save again afterwards. Pass --pause to open the
# in-game pause menu (ESC) over whatever else was built.
#
# Pass --shop to open the build menu (docs/UI_UX.md §5) before the shot, or
# --market to open the ingredient market — the stock half of the shop, where
# raw ingredients are bought (the chain-rework epic).
#
# Pass --no-hud to hide the HUD entirely. Closeups are for reviewing whether
# a belt or a pipe actually meets a machine, and the contract clipboard and
# the hotbar cover the top-right and the bottom of the frame — which is where
# a junction lands about half the time.
#
# Pass --contents N alongside --ghost hopper to press Q N times first, so the
# hopper ghost is photographed holding the SECOND raw ingredient rather than
# the first. That is the only way to see the contents key doing anything.
#
# Pass --closeup TX,TY[,SIZE] to aim the isometric camera at one simulation
# tile and zoom right in (SIZE is the orthographic width in world units;
# fractional tile coordinates are allowed, because a junction sits ON a tile
# boundary). The whole-factory shot is the right frame for "does the line
# read" and the wrong one for "does this belt actually meet this machine",
# e.g.
#
#   scripts/screenshot-scene.sh out.png 1100x800 --build --closeup 0.5,0.3,10
#
# Usage: scripts/screenshot-scene.sh [output.png] [WIDTHxHEIGHT] [--build] [--fixture NAME] [--stress] [--pipe-gallery] [--belt-gallery] [--contracts] [--populate] [--first-person] [--ghost KIND|--ghost-blocked KIND|--remove-mode] [--hover|--hover-pipe|--hover-vat|--hover-hopper|--hover-tank|--hover-dock|--hover-splitter] [--menu] [--pause] [--shop] [--market] [--no-hud] [--stock] [--hotbar N] [--contents N] [--closeup TX,TY[,SIZE]]
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Defaults into the same gitignored previews/ directory the voxel-model
# thumbnails go to — all of it is regenerable review output, none of it ships.
out="${1:-$repo_root/assets/voxels/previews/scene.png}"
resolution="${2:-1600x900}"
godot_bin="${GODOT_BIN:-godot}"

case "$out" in
  /*) ;;
  *) out="$repo_root/$out" ;;
esac
mkdir -p "$(dirname "$out")"

"$godot_bin" --path godot_project --resolution "$resolution" \
  --script test_screenshot.gd -- "$out" "${@:3}"
status=$?
if [ ! -f "$out" ]; then
  echo "error: no screenshot written to $out (godot exited $status)" >&2
  exit 1
fi
echo "wrote $out"
