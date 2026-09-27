#!/usr/bin/env bash
# Rebuilds the voxel asset pipeline (UI_UX.md §2a) end to end:
#
#   1. `cargo run -p voxgen --bin voxgen` regenerates every .vox in assets/voxels/models/
#      from the Rust generators in assets/voxels/generators/ (the actual
#      source of truth — never hand-edit the .vox files).
#   2. `vengi-thumbnailer` renders each *combined* model to an isometric PNG
#      in assets/voxels/previews/, which is how a Claude Code session (or a
#      human) actually LOOKS at the asset it just generated without opening a
#      GUI voxel editor — UI_UX.md §2a point 3.
#   3. `cargo run -p voxgen --bin voxicons` renders a second, different kind
#      of PNG per combined model into assets/voxels/icons/ (plus a 512px
#      app_icon.png, the window/dock icon): a small (64px),
#      transparent-background, tightly-cropped UI icon (UI_UX.md §5 — "icons
#      are rendered from the real assets ... not separately-drawn 2D icon
#      art"), NOT a copy of step 2's review preview. See
#      assets/voxels/generators/icons.rs's module doc for why a preview PNG
#      turned out to be the wrong shape for a UI icon (opaque background,
#      padded for "look at the whole model," not "sit in a small UI slot"),
#      confirmed by actually inspecting vengi-thumbnailer's own output and
#      its `--jsonconfig` cvar list rather than assumed.
#
# Steps 2 and 3 both need vengi-thumbnailer; step 1 doesn't. If vengi isn't
# installed the script still regenerates the models, warns, and exits 0.
# Previews are gitignored review output, not something the build depends on
# — but icons ARE committed (godot_project/main.gd loads them at runtime,
# same treatment as assets/voxels/models/), so a machine without vengi can
# still run the game against whatever icons the last commit shipped.
#
# Installing vengi (macOS, matching the version this was set up with):
#   mkdir -p ~/.local/vengi/thumbnailer.app && cd ~/.local/vengi/thumbnailer.app
#   curl -sSLO https://github.com/vengi-voxel/vengi/releases/download/v0.5.0/mac-vengi-thumbnailer-app.zip
#   unzip -q mac-vengi-thumbnailer-app.zip && rm mac-vengi-thumbnailer-app.zip
# Linux/Windows builds of the same release are on that releases page too.
# Override the binary with VENGI_THUMBNAILER=/path/to/vengi-thumbnailer.
#
# Note for headless/CI use: vengi's thumbnailer renders through OpenGL, so it
# needs a real video driver. Forcing SDL_VIDEODRIVER=dummy makes it fail with
# "OpenGL support is either not configured in SDL..." — don't.
#
# Usage: scripts/build-voxels.sh

set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

echo "==> cargo run -p voxgen --bin voxgen (regenerate assets/voxels/models/)"
cargo run -q -p voxgen --bin voxgen || exit 1
echo

thumbnailer="${VENGI_THUMBNAILER:-}"
if [ -z "$thumbnailer" ]; then
  if command -v vengi-thumbnailer >/dev/null 2>&1; then
    thumbnailer="$(command -v vengi-thumbnailer)"
  elif [ -x "$HOME/.local/vengi/thumbnailer.app/Contents/MacOS/vengi-thumbnailer" ]; then
    thumbnailer="$HOME/.local/vengi/thumbnailer.app/Contents/MacOS/vengi-thumbnailer"
  fi
fi

if [ -z "$thumbnailer" ]; then
  echo "warning: vengi-thumbnailer not found — models regenerated, previews SKIPPED." >&2
  echo "         See this script's header for how to install it." >&2
  exit 0
fi

previews="assets/voxels/previews"
mkdir -p "$previews"

echo "==> rendering isometric previews with $thumbnailer"
status=0
for model in assets/voxels/models/*.vox; do
  name="$(basename "$model" .vox)"
  # Only the combined models are worth reviewing; the per-material-class
  # split files (name.metal.vox etc.) are a Godot import detail.
  case "$name" in
    *.matte|*.metal|*.emissive|*.liquid) continue ;;
  esac
  out="$previews/$name.png"
  # Only ever replace this model's own preview: scripts/screenshot-scene.sh
  # writes a full-scene render into the same (gitignored) directory.
  rm -f "$out"
  if ! "$thumbnailer" -i "$model" -o "$out" -s 512 --image --isometric >/dev/null 2>&1; then
    echo "  FAIL $name" >&2
    status=1
    continue
  fi
  echo "  $out"
done

echo
echo "==> rendering UI icons with $thumbnailer (assets/voxels/icons/, committed — UI_UX.md §5)"
if ! VENGI_THUMBNAILER="$thumbnailer" cargo run -q -p voxgen --bin voxicons; then
  status=1
fi

exit "$status"
