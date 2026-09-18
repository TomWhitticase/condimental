#!/usr/bin/env python3
"""The voxel asset browser — a local web app for reviewing `assets/voxels/`.

    python3 tools/asset-browser/serve.py        # then open the printed URL

Serves the web app in `web/`, plus two JSON endpoints built live from the
repo: `/api/catalog` (every asset, grouped, with its generator's prose) and
`/api/model/<name>` (one model's voxels, split by material class the way
Godot draws it). Nothing is cached to disk, so a `cargo run -p voxgen` is
visible on reload.
"""

from __future__ import annotations

import argparse
import json
import mimetypes
import sys
import threading
import urllib.request
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import unquote, urlparse

sys.path.insert(0, str(Path(__file__).resolve().parent))

import voxlib  # noqa: E402

HERE = Path(__file__).resolve().parent
WEB = HERE / "web"
VENDOR = WEB / "vendor"

THREE_VERSION = "0.169.0"
# Mirrored with unpkg's own paths, because the example modules import each
# other relatively ('./Pass.js', '../shaders/CopyShader.js'); flattening them
# into one folder breaks those imports. Only 'three' itself is bare, and the
# page's import map resolves it.
VENDOR_FILES = [
    "build/three.module.js",
    "examples/jsm/controls/OrbitControls.js",
    # The glow pass: the factory's WorldEnvironment has one, and a status
    # lamp that does not bloom is not the lamp the game shows.
    "examples/jsm/postprocessing/EffectComposer.js",
    "examples/jsm/postprocessing/Pass.js",
    "examples/jsm/postprocessing/RenderPass.js",
    "examples/jsm/postprocessing/ShaderPass.js",
    "examples/jsm/postprocessing/MaskPass.js",
    "examples/jsm/postprocessing/OutputPass.js",
    "examples/jsm/postprocessing/UnrealBloomPass.js",
    "examples/jsm/shaders/CopyShader.js",
    "examples/jsm/shaders/LuminosityHighPassShader.js",
    "examples/jsm/shaders/OutputShader.js",
]


# The catalog is cheap (~80 small files) but not free; rebuild it only when
# something it is derived from has actually changed on disk.
_catalog_lock = threading.Lock()
_catalog: dict | None = None
_catalog_stamp: tuple | None = None


def _sources_stamp() -> tuple:
    watched = [
        voxlib.MODELS,  # directory mtime: a model added or removed
        voxlib.MODELS / "index.json",
        voxlib.ICONS,
        voxlib.VOXELS / "palette.ron",
        voxlib.GENERATORS / "assets.rs",
        voxlib.MAIN_GD,
        voxlib.REPO / "data" / "machines.ron",
    ]
    return tuple(p.stat().st_mtime_ns if p.exists() else 0 for p in watched)


def catalog() -> dict:
    global _catalog, _catalog_stamp
    with _catalog_lock:
        stamp = _sources_stamp()
        if _catalog is None or stamp != _catalog_stamp:
            _catalog = voxlib.build_catalog()
            _catalog_stamp = stamp
        return _catalog


def ensure_vendor() -> None:
    """three.js is fetched once, into a gitignored folder, then used offline."""
    missing = [rel for rel in VENDOR_FILES if not (VENDOR / rel).exists()]
    if not missing:
        return
    print(f"asset-browser: fetching three.js {THREE_VERSION} into web/vendor/ (once)")
    for rel in missing:
        url = f"https://unpkg.com/three@{THREE_VERSION}/{rel}"
        try:
            with urllib.request.urlopen(url, timeout=30) as r:
                data = r.read()
        except Exception as exc:  # pragma: no cover - network path
            sys.exit(
                f"asset-browser: could not download {rel} from {url}\n"
                f"  {exc}\n"
                f"  Drop the file into {VENDOR / rel} by hand and re-run."
            )
        dest = VENDOR / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
        print(f"  {rel}  {len(data) // 1024} KiB")


class Handler(BaseHTTPRequestHandler):
    server_version = "sauce-asset-browser"

    def log_message(self, fmt, *args):  # quieter than the default
        if "--verbose" in sys.argv:
            super().log_message(fmt, *args)

    def _send(self, body: bytes, ctype: str, status: int = 200) -> None:
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def _json(self, payload: dict, status: int = 200) -> None:
        self._send(
            json.dumps(payload).encode(), "application/json; charset=utf-8", status
        )

    def _file(self, path: Path) -> None:
        if not path.is_file():
            self._json({"error": f"not found: {path.name}"}, 404)
            return
        ctype, _ = mimetypes.guess_type(path.name)
        self._send(path.read_bytes(), ctype or "application/octet-stream")

    def do_GET(self) -> None:  # noqa: N802 (http.server's API)
        route = unquote(urlparse(self.path).path)
        try:
            if route == "/api/catalog":
                self._json(catalog())
            elif route == "/api/stamp":
                # What the page polls: a new asset should appear without
                # anyone remembering to reload.
                self._json({"stamp": [str(v) for v in _sources_stamp()]})
            elif route.startswith("/api/model/"):
                name = route[len("/api/model/") :]
                try:
                    self._json(voxlib.model_payload(name))
                except KeyError:
                    self._json({"error": f"unknown model: {name}"}, 404)
            elif route.startswith("/icons/"):
                self._file(voxlib.ICONS / Path(route).name)
            elif route.startswith("/previews/"):
                self._file(voxlib.PREVIEWS / Path(route).name)
            else:
                rel = route.lstrip("/") or "index.html"
                target = (WEB / rel).resolve()
                if not str(target).startswith(str(WEB.resolve())):
                    self._json({"error": "forbidden"}, 403)
                    return
                self._file(target)
        except BrokenPipeError:
            pass
        except Exception as exc:  # surface tool bugs in the browser, not silence
            self._json({"error": repr(exc)}, 500)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--port", type=int, default=8765)
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--no-browser", action="store_true")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    ensure_vendor()
    cat = catalog()
    print(
        f"asset-browser: {len(cat['assets'])} assets / {len(cat['models'])} models "
        f"from {voxlib.MODELS.relative_to(voxlib.REPO)}"
    )
    url = f"http://{args.host}:{args.port}/"
    httpd = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"asset-browser: serving {url}  (ctrl-c to stop)")
    if not args.no_browser:
        threading.Timer(0.5, lambda: webbrowser.open(url)).start()
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nasset-browser: stopped")


if __name__ == "__main__":
    main()
