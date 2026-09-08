#!/usr/bin/env bash
# Renders godot_project/main.tscn to a PNG for visual review (docs/UI_UX.md).
#
# This is the one verification step in the project that is NOT headless, and
# deliberately so: everything else about this codebase is checked without a
# framebuffer (ARCHITECTURE.md §7), but "does the factory look like UI_UX.md
# describes" cannot be. It opens a real window briefly, waits for lighting to
# settle, writes the frame, and exits.
#
# Pass --populate to run the production chain forward first, so the shot
# shows product moving along the belts instead of an idle factory. Pass
# --first-person to review that camera mode's lighting/skybox instead of the
# default isometric one.
#
# Pass one of --hover / --hover-pipe / --hover-vat / --hover-hopper /
# --hover-tank to park the cursor over the first machine, the fluid network's
# machine-inlet pipe, the FAR CORNER of the multi-tile cooking vat, the
# chain's hopper, or the pipe run's fluid tank, so that tooltip is in the
# shot. (--hover-vat deliberately picks a non-origin tile: "any tile of a
# multi-tile machine identifies it" is what the "Machines improvement"
# milestone added, and hovering the origin would not show it.)
#
# Pass --closeup TX,TY[,SIZE] to aim the isometric camera at one simulation
# tile and zoom right in (SIZE is the orthographic width in world units;
# fractional tile coordinates are allowed, because a junction sits ON a tile
# boundary). The whole-factory shot is the right frame for "does the line
# read" and the wrong one for "does this belt actually meet this machine" —
# every belt/pipe-to-machine junction in the "Visual overhaul round 2" epic's
# ticket 3 was diagnosed and re-checked one at a time through this flag, e.g.
#
#   scripts/screenshot-scene.sh out.png 1100x800 --populate --closeup 8.4,0.3,10
#
# Usage: scripts/screenshot-scene.sh [output.png] [WIDTHxHEIGHT] [--populate] [--first-person] [--hover|--hover-pipe|--hover-vat|--hover-hopper|--hover-tank] [--closeup TX,TY[,SIZE]]
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
