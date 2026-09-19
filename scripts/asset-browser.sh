#!/usr/bin/env bash
# Opens the voxel asset browser (tools/asset-browser): every asset in
# assets/voxels/, searchable, in a 3D viewer you can turn.
#
# Reads the committed pipeline output live — no build step. If you have just
# changed a generator, run `cargo run -p voxgen --bin voxgen` first and reload the page.
set -euo pipefail

cd "$(dirname "$0")/.."
exec python3 tools/asset-browser/serve.py "$@"
