#!/usr/bin/env python3
"""Derives the asset browser's one AO knob from the game's own SSAO.

Issue #49 asked the browser for ambient occlusion so a fitting-to-body crease
reads here the way it reads in the game. "The way it reads in the game" is a
number, not a taste, and it is a small one: at the shipped
`ssao_intensity = 2.4` under `ambient_light_energy = 1.2`, Godot's SSAO moves
the vat fixture's deepest crease by a couple of percent. A browser that
darkened creases harder than the renderer it stands in for would be review
tooling that lies — someone would approve a form that only reads under
browser-only occlusion, which is the failure UI_UX.md §2b point 12 exists to
catch. So the strength is measured, not chosen:

    tools/asset-browser/calibrate-ao.py

renders four frames — the game's vat fixture with `factory_environment.gd`'s
SSAO on and off, and this browser on the same model with its AO on and off —
measures how far each renderer darkens its deepest crease, and divides one by
the other to get `AO_STRENGTH` for `web/app.js`. It then renders a fifth
frame at the derived value and reports what it actually achieved, so the run
either closes its own loop or says it did not.

Re-run it whenever `ssao_intensity`, `ssao_radius`, `ssao_power` or the
ambient energy moves; the follow-up is then a constant edit, not another
investigation. Nothing is written for you — the number is printed with the
line to paste.

Needs: a GPU-less Chrome (SwiftShader is fine) and a windowed Godot, because
`scripts/screenshot-scene.sh` is the project's one non-headless check.
"""

import argparse
import json
import os
import re
import shutil
import signal
import socket
import statistics
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent

CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
ENV_GD = ROOT / "godot_project" / "factory_environment.gd"
APP_JS = HERE / "web" / "app.js"

# The vat is the subject because it is the one fixture that is a single
# machine at a readable size, and because its tank-to-body and leg-to-deck
# creases are exactly the two the issue names.
FIXTURE = "vat"
ASSET = "machine_cooking_vat"
CLOSEUP = "0.5,0.5,12"
SIZE = (1600, 900)

# Pixels this dark carry no lighting to dim, and a ratio taken across them is
# all quantisation noise.
LIT_FLOOR = 8
# A pixel is "touched" once it loses a full percent; below that a 0-255 frame
# cannot tell occlusion from dithering.
TOUCH = 0.99
# The deep end of the touched set, not its single darkest pixel: one pixel is
# an antialiasing accident, the 2nd percentile is a crease.
DEEP_PCT = 2


def luma(px):
    return 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]


def darkening(on_path, off_path, step=1):
    """How far `on` darkens `off`, as the renderer's own deepest crease.

    Returns the deep-end darkening (0.03 = 3%) and what fraction of the lit
    frame it touched at all. Both are relative to the frame they came from,
    which is what makes them comparable between two renderers that do not
    share a camera, a palette or a tone map.
    """
    on = Image.open(on_path).convert("RGB")
    off = Image.open(off_path).convert("RGB")
    if on.size != off.size:
        sys.exit(f"frames differ in size: {on.size} vs {off.size}")
    pon, poff = on.load(), off.load()
    w, h = on.size
    lit = 0
    ratios = []
    for y in range(0, h, step):
        for x in range(0, w, step):
            lb = luma(poff[x, y])
            if lb <= LIT_FLOOR:
                continue
            lit += 1
            r = luma(pon[x, y]) / lb
            if r < TOUCH:
                ratios.append(r)
    if not ratios:
        return {"deepest": 0.0, "touched": 0.0, "lit": lit, "n": 0}
    ratios.sort()
    deep = ratios[min(len(ratios) - 1, len(ratios) * DEEP_PCT // 100)]
    return {
        "deepest": 1 - deep,
        "touched": len(ratios) / lit,
        "lit": lit,
        "n": len(ratios),
        "darkest": 1 - ratios[0],
    }


# ------------------------------------------------------------------- the game


def shoot_game(out, ssao):
    """One vat frame, with `factory_environment.gd`'s SSAO forced on or off.

    The file is restored from a byte copy rather than from git, so a run
    cannot eat an uncommitted edit somebody else is making to it.
    """
    backup = ENV_GD.read_bytes()
    try:
        src = backup.decode()
        want = "env.ssao_enabled = %s" % ("true" if ssao else "false")
        patched, n = re.subn(r"env\.ssao_enabled = (?:true|false)", want, src)
        if n != 1:
            sys.exit(f"expected exactly one ssao_enabled in {ENV_GD}, found {n}")
        ENV_GD.write_text(patched)
        run = subprocess.run(
            [
                str(ROOT / "scripts" / "screenshot-scene.sh"),
                str(out),
                "%dx%d" % SIZE,
                "--fixture", FIXTURE,
                "--no-hud",
                "--closeup", CLOSEUP,
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if "SCREENSHOT_OK" not in run.stdout:
            sys.exit(f"screenshot-scene.sh failed:\n{run.stdout[-2000:]}\n{run.stderr[-2000:]}")
    finally:
        ENV_GD.write_bytes(backup)


# ---------------------------------------------------------------- the browser


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Server:
    """The browser's own server, on a port of its own, so a calibration run
    neither needs nor disturbs one a reviewer already has open."""

    def __init__(self, shots):
        self.port = free_port()
        self.shots = shots
        self.proc = subprocess.Popen(
            [sys.executable, str(HERE / "serve.py"), "--port", str(self.port),
             "--no-browser", "--shot-dir", str(shots)],
            cwd=ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        for _ in range(100):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{self.port}/api/catalog", timeout=1)
                return
            except (urllib.error.URLError, ConnectionError):
                time.sleep(0.2)
        self.stop()
        sys.exit("the asset browser's server did not come up")

    def stop(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()


def shoot_browser(server, name, ao, tmp):
    """One browser frame, through a headless Chrome, at AO strength `ao`.

    The page posts its own canvas to the server (`?shot=`) instead of Chrome
    screenshotting it: Chrome captures on a load heuristic that lands before
    the catalog fetch has even returned, and its `--virtual-time-budget`
    never expires against a page with an animation loop. Chrome is started in
    its own session and that session — only ever the process this function
    launched — is signalled once the PNG lands.
    """
    out = server.shots / f"{name}.png"
    out.unlink(missing_ok=True)
    url = (f"http://127.0.0.1:{server.port}/"
           f"?view=game&bare=1&shot={name}&ao={ao:g}#{ASSET}")
    proc = subprocess.Popen(
        [
            CHROME,
            "--headless=new",
            "--disable-gpu",
            "--use-gl=angle",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--hide-scrollbars",
            "--force-device-scale-factor=1",
            f"--window-size={SIZE[0]},{SIZE[1]}",
            f"--user-data-dir={tmp}/chrome",
            url,
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    try:
        for _ in range(300):
            time.sleep(0.2)
            if out.exists():
                return out
        sys.exit(f"the browser never posted {name}.png — is Chrome rendering WebGL?")
    finally:
        if proc.poll() is None:
            os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)


def current_strength():
    m = re.search(r"const AO_STRENGTH = ([0-9.]+);", APP_JS.read_text())
    return float(m.group(1)) if m else None


# -------------------------------------------------------------------- the run


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--probe", type=float, default=1.0,
                    help="browser AO strength to measure the response at (default 1.0)")
    ap.add_argument("--step", type=int, default=1,
                    help="sample every Nth pixel; 2 is four times faster and moves the "
                         "numbers in the third decimal")
    ap.add_argument("--keep", metavar="DIR",
                    help="keep the five frames here instead of a temp dir")
    ap.add_argument("--json", action="store_true", help="machine-readable result")
    args = ap.parse_args()

    if not Path(CHROME).exists():
        sys.exit(f"no Chrome at {CHROME}")

    tmp = Path(args.keep) if args.keep else Path(tempfile.mkdtemp(prefix="ao-calib-"))
    tmp.mkdir(parents=True, exist_ok=True)

    say = (lambda *a: None) if args.json else (lambda *a: print(*a, flush=True))

    say(f"frames: {tmp}")
    say("game: vat fixture, SSAO on ...")
    shoot_game(tmp / "game-ao.png", ssao=True)
    say("game: vat fixture, SSAO off ...")
    shoot_game(tmp / "game-flat.png", ssao=False)
    game = darkening(tmp / "game-ao.png", tmp / "game-flat.png", args.step)

    server = Server(tmp)
    try:
        say(f"browser: {ASSET}, AO off ...")
        flat = shoot_browser(server, "web-flat", 0, tmp)
        say(f"browser: {ASSET}, AO at the probe strength {args.probe:g} ...")
        probe_png = shoot_browser(server, "web-probe", args.probe, tmp)
        probe = darkening(probe_png, flat, args.step)

        if probe["deepest"] <= 0:
            sys.exit("the browser darkened nothing at the probe strength — is the AO wired up?")
        derived = args.probe * game["deepest"] / probe["deepest"]

        say(f"browser: {ASSET}, AO at the derived strength {derived:.3f} ...")
        got = shoot_browser(server, "web-derived", derived, tmp)
        achieved = darkening(got, flat, args.step)
    finally:
        server.stop()

    result = {
        "game_deepest": game["deepest"],
        "game_touched": game["touched"],
        "browser_probe": args.probe,
        "browser_probe_deepest": probe["deepest"],
        "derived_strength": round(derived, 3),
        "achieved_deepest": achieved["deepest"],
        "achieved_touched": achieved["touched"],
        "current_strength": current_strength(),
        "frames": str(tmp),
    }
    if args.json:
        print(json.dumps(result, indent=2))
        return

    def row(name, d):
        print(f"  {name:<28} deepest {d['deepest'] * 100:5.2f}%   "
              f"touched {d['touched'] * 100:5.2f}% of the lit frame   "
              f"({d['n']} px)")

    print()
    print("deepest crease, as a fraction of the pixel's own unoccluded luma:")
    row("game (ssao_intensity)", game)
    row(f"browser at {args.probe:g}", probe)
    row(f"browser at {derived:.3f}", achieved)
    print()
    print(f"  AO_STRENGTH = {derived:.3f};   "
          f"(web/app.js currently says {current_strength()})")
    print(f"  achieved {achieved['deepest'] * 100:.2f}% against the game's "
          f"{game['deepest'] * 100:.2f}%")


if __name__ == "__main__":
    main()
