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
# Pass --payout to build that same line, run it until a contract genuinely
# pays out, and hold the frame partway through the celebration that fires:
# the coin burst over the dock, the rising "+N" over the money line, the
# money figure mid-punch and the paying docket reading PAID. This is the one
# check the payout work needs eyes on — test_payout_feedback.gd can prove
# every number and cannot say whether the burst reads as a fountain.
# It implies --fixture chain, and it FREEZES the scene once the frame is set
# up, since a burst is over in 34 ticks and the shot's warm-up is longer.
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
# Pass --bottleneck to build the BOTTLENECK LINE (test_layouts.gd's
# `build_bottleneck_line`): the whole shipped line with its hoppers and tank
# on auto-fill, run until it has backed up behind the 240-tick cooking vat,
# so the machines read blocked · blocked · working · starved · starved
# along the line — the fixture the machine-load indicator is reviewed on.
# It implies --populate. Add --hover to put the crusher's load line (glyph,
# word, number, reason) and status-coloured load bar in the tooltip, which
# is where the indicator lives since the stack-light poles were removed.
#
# Pass --staff to build the shipped line, run it (implies --populate, so the
# meters read a line asking for its crew) and open the STAFF panel (H) over
# it — one row per category with its icon, hired count, assigned/asked-for
# meter, wage and HIRE/FIRE, and the wage bill and payday countdown in the
# footer (the staff pools MVP). Pass --understaffed to build the
# UNDERSTAFFED LINE (test_layouts.gd's `build_understaffed_line`: the same
# line on auto-fill with two manufacturing heads fired, run until the
# cooking vat is refused its crew) with the cursor on the vat, so the hover
# shows `▲ starved · … · short of manufacturing staff` over its crew line.
# It implies --populate.
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
# raw ingredients are bought (the chain-rework epic) — or --board to open the
# CONTRACTS BOARD, where contracts are signed and lost ones signed again (the
# contract-board epic). --contracts signs every offer before it runs the
# line, so the clipboard photographs the whole board competing.
#
# Pass --no-hud to hide the HUD entirely. Closeups are for reviewing whether
# a belt or a pipe actually meets a machine, and the contract clipboard and
# the hotbar cover the top-right and the bottom of the frame — which is where
# a junction lands about half the time.
#
# Pass --config TX,TY to click the hopper or tank on that tile after the
# fixture is built, so the frame shows its configuration panel (the
# hopper/tank click-to-configure ticket). With an empty larder FILL is
# photographed blocked, with its reason; add --stock to see it enabled.
# --contents N on top presses the open panel's picker slot N, so a placed
# hopper is photographed retargeted to another raw ingredient, e.g.
#
#   scripts/screenshot-scene.sh out.png --build --config=-1,0 --contents 1
#
# (the demo line's hopper stands at (-1, 0); note the `=` form, since a
# value starting with `-` would otherwise be read as another flag).
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
# Usage: scripts/screenshot-scene.sh [output.png] [WIDTHxHEIGHT] [--build] [--fixture NAME] [--stress] [--pipe-gallery] [--belt-gallery] [--bottleneck] [--staff] [--understaffed] [--contracts] [--payout] [--populate] [--first-person] [--ghost KIND|--ghost-blocked KIND|--remove-mode] [--hover|--hover-pipe|--hover-vat|--hover-hopper|--hover-tank|--hover-dock|--hover-splitter] [--menu] [--pause] [--shop] [--market] [--board] [--no-hud] [--stock] [--hotbar N] [--config TX,TY [--contents N]] [--closeup TX,TY[,SIZE]]
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
