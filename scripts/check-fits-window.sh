#!/usr/bin/env bash
# Runs test_hud_plates.gd in a REAL window instead of `--headless` (issue
# #101), so its width assertions — every modal fits the window with every
# button reachable, at 1600x900 and 1280x720 — are measured off panels that
# have actually been drawn.
#
# Why a headless pass is not enough on its own: on 2026-09-20 the staff panel
# passed the headless sweep at 1280x720 and the rendered frame still clipped
# it on both edges (docs/UI_UX.md §5c). Why was never pinned down. It is not
# the font, which measures a string to the pixel the same headless and
# windowed, and on Godot 4.6.3 no modal measures differently by mode. So this
# is a guard, not a fix for a known gap: it runs the same assertions with a
# real display server, a real window and real draws, the render's conditions.
# Never pin a contracts-board width from either run: its countdowns move
# with wall-clock time (no --fixed-fps here), so it drifts ~2px run to run.
#
# The window is off-screen and focus is handed back, as for
# scripts/screenshot-scene.sh. Needs a display, so CI (headless Linux) does
# not run it; run it before opening a PR that changes a panel's layout.
#
# Usage: scripts/check-fits-window.sh
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
godot_bin="${GODOT_BIN:-godot}"

# shellcheck source=lib/hand-focus-back.sh
source "$repo_root/scripts/lib/hand-focus-back.sh"

log="$(mktemp)"
trap 'rm -f "$log"' EXIT
"$godot_bin" --path godot_project --script test_hud_plates.gd >"$log" 2>&1 &
godot_pid=$!
trap 'kill "$godot_pid" 2>/dev/null; rm -f "$log"' INT TERM
watcher_pid=""
if command -v lsappinfo >/dev/null; then
  hand_focus_back "$godot_pid" &
  watcher_pid=$!
fi
wait "$godot_pid"
[ -n "$watcher_pid" ] && wait "$watcher_pid" 2>/dev/null

# Godot exits 0 even when a script fails to parse, so the marker is the
# proof — and it must say WINDOWED, or this run measured what the headless
# one already did.
if grep -q "HUD_PLATES_OK: \[windowed\]" "$log"; then
  echo "FITS_WINDOW_OK: every modal fits a real 1600x900 and 1280x720 window"
  exit 0
fi
grep -E "HUD_PLATES_(OK|FAIL)|ERROR|SCRIPT ERROR" "$log" >&2
echo "FITS_WINDOW_FAIL: test_hud_plates.gd did not pass in a real window" >&2
exit 1
