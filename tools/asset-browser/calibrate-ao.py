#!/usr/bin/env python3
"""Derives the asset browser's one AO knob from the game's own SSAO — and
refuses the answer when the two passes are not shading the same amount of
machine.

Issue #49 asked the browser for ambient occlusion so a fitting-to-body crease
reads here the way it reads in the game. "The way it reads in the game" is a
number, not a taste. A browser that darkened harder than the renderer it
stands in for would be review tooling that lies — someone would approve a
form that only reads under browser-only occlusion, which is the failure
UI_UX.md §2b point 12 exists to catch. So the strength is measured:

    tools/asset-browser/calibrate-ao.py --write

renders the game's vat fixture with `factory_environment.gd`'s SSAO on and
off, renders this browser on the same model with its AO off and at a probe
strength, and divides one renderer's shading by the other's to get
`AO_STRENGTH` for `web/ao.mjs`. It then renders a frame at the derived value
and checks it, so the run either closes its own loop or says it did not.

WHAT IS MEASURED, AND WHY IT IS NOT A PERCENTILE OF THE TOUCHED PIXELS.
The first version of this script scored each renderer by the deep end of the
pixels it darkened at all. That is area-normalised: a pass that washes a
faint shadow over the whole model and a pass that cuts two deep creases can
score the same, because "how much of the model got darkened" divides out.
It did exactly that — it derived a constant that reproduced to three decimal
places while the browser was darkening 11x more of the machine than the game
(caught in review, 2026-09-20). So the metric now reports, over the machine's
own pixels and nothing else:

  * shade    — mean darkening per machine pixel. Area-SENSITIVE: a broader
               wash scores higher, which is what the derivation divides on.
  * covered  — the fraction of the machine darkened by more than 1% (and by
               more than 3%). This is the term that was missing.
  * deep     — the 99th percentile of darkening over the machine, not over
               the touched set, so a sparse deep crease and a broad wash can
               be told apart.

and the run FAILS, loudly and with a non-zero exit, when the browser's
coverage diverges from the game's. Strength cannot fix a coverage gap: it
scales every darkened pixel alike, so a mismatch there is the kernel's reach
(AO_RADIUS in `web/ao.mjs`, which is `ssao_radius` in voxels) and not this
constant. The script checks that reach against the game's own `ssao_radius`
before it renders anything, and `--write` updates both numbers in place.

"The machine's own pixels" is not a guess either: the game's subject is found
by rendering the SAME camera over the empty world and taking what differs
(the world starts empty, so this is one more `screenshot-scene.sh` run), and
the browser's by taking what differs from its backdrop with the grid off.
Both subjects include the shadow the machine casts, in both renderers.

Needs: a GPU-less Chrome (SwiftShader is fine) and a windowed Godot, because
`scripts/screenshot-scene.sh` is the project's one non-headless check.

WHILE IT RUNS it edits `godot_project/factory_environment.gd` — for about a
minute per game frame — and restores it, including on Ctrl-C. It refuses to
start if that file already has uncommitted changes. Do not run
`screenshot-scene.sh` from this worktree meanwhile: it would photograph an
environment nobody asked for.
"""

import argparse
import atexit
import json
import math
import os
import re
import signal
import socket
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
AO_MJS = HERE / "web" / "ao.mjs"

# assets/voxels: one voxel is 0.1 m, so a radius in metres is a radius in
# voxels times this (vox::VOXEL_UNIT).
VOXEL_UNIT = 0.1

# The vat is the subject because it is the one fixture that is a single
# machine at a readable size, and because its tank-to-body and leg-to-deck
# creases are exactly the two the issue names.
FIXTURE = "vat"
ASSET = "machine_cooking_vat"
CLOSEUP = "0.5,0.5,12"

TEST_GD = ROOT / "godot_project" / "test_screenshot.gd"

# WHICH FRAME THE CONSTANT IS DERIVED AGAINST, and why it is not the one the
# player spends the game in.
#
# `ssao_radius` is in WORLD units, so how much occlusion Godot's pass resolves
# per pixel depends on how near the geometry is. Under the isometric play
# camera a 0.45 m radius covers so few pixels that the whole pass reaches
# 0.3% of a machine — deriving a strength there measures the camera, not the
# model, and produces a browser pass sitting below one 8-bit level. The bake
# is OBJECT-SPACE: it is a property of the model, which is what this browser
# exists to show. So it is calibrated against the game's own strongest honest
# presentation of the same geometry — standing beside the line at eye height,
# where the same settings shade about a tenth of the frame — and the
# isometric pair is measured too, and printed, so the gap is published rather
# than hidden. (Coordinator ruling, 2026-09-20; DECISIONS.md Part A.)
#
# The pose is patched into `test_screenshot.gd` for exactly as long as the
# render takes, because the harness's own first-person pose stands 4 m above
# the focus and pitches UP, across an empty hall — a frame with no near
# geometry in it cannot say anything about an occlusion pass.
# The SAME lone vat as the isometric pair and as the browser — one machine on
# an empty floor, so nothing occludes it but itself, which is the only thing a
# per-model bake can reproduce. Eye height, three tiles out, looking at it.
EYE_FIXTURE = FIXTURE
EYE_POSE = """func _enter_first_person() -> void:
	_main._set_view_mode(_main.ViewMode.FIRST_PERSON)
	_main._fp_position = Vector3(-4.5, 1.7, -4.5)
	_main._fp_yaw = deg_to_rad(225.0)
	_main._fp_pitch = deg_to_rad(-2.0)
	_main._update_fp_camera_transform()
"""
SIZE = (1600, 900)

# Pixels this dark carry no lighting to dim, and a ratio taken across them is
# all quantisation noise.
LIT_FLOOR = 8
# A pixel counts as darkened once it loses a full percent; below that a 0-255
# frame cannot tell occlusion from dithering. The second threshold is the one
# a reviewer would call "visible shading".
COVER = (0.01, 0.03)
# The deep end of the machine, not of the touched set.
DEEP_PCT = 99
# A pixel differing from the subject-free reference by this much luma is the
# machine (or its shadow); below it is the floor, or the backdrop, unchanged.
SUBJECT_DELTA = 3.0

# How far the browser's coverage may sit from the game's before the run is a
# failure rather than a calibration. Wide, because the two renderers do not
# share a camera, a shadow or a floor — but nothing like wide enough to let a
# 11x wash through.
COVER_BAND = (0.5, 2.0)
DEEP_BAND = (0.5, 2.0)
# Below this the game's pass has no creases of its own to match: not "it shades
# a small area" — Godot's SSAO is a narrow, deep crease finder and a *sparse*
# result is the correct one for a lone machine under the isometric camera —
# but "nothing it does is deeper than the threshold a reviewer would see". At
# ssao_radius 0.9 / ssao_intensity 2.4 the game cleared this on no pixel at
# all, which is what made that setting uncalibratable rather than merely small.
DEEP_FLOOR = 0.0001

# What a believable pair of game frames looks like before any of it is
# measured. The machine covers this share of the frame; the pass-on frame
# still has the machine in the same pixels (SSAO moves no geometry, so this
# is containment, not equality — the pass DOES add contact shadow on the
# floor just outside the machine's own silhouette); and the pass can only
# take light away, never add it.
MACHINE_SHARE = (0.02, 0.60)
MASK_KEPT = 0.95

# How close the rendered strength has to land to the game before the
# correction below stops paying for itself, and how many renders it may
# spend getting there.
REFINE_TOL = 0.02
REFINE_PASSES = 8


def luma(px):
    return 0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]


def load(path):
    """A frame as a flat list of luma, plus its RGB rows — indexed y * w + x."""
    im = Image.open(path).convert("RGB")
    w, h = im.size
    raw = im.tobytes()          # not getdata(), which Pillow 14 removes
    px = [tuple(raw[i:i + 3]) for i in range(0, len(raw), 3)]
    return {"w": w, "h": h, "rgb": px, "luma": [luma(p) for p in px]}


# ------------------------------------------------------------------ subjects


def game_subject(flat, empty):
    """Every pixel the machine put on screen: what differs from the same
    camera over the empty world. Exact, not a colour guess — the floor is
    drawn identically in both frames (verified: zero difference off the
    machine), so this is the machine and the shadow it casts."""
    if (flat["w"], flat["h"]) != (empty["w"], empty["h"]):
        sys.exit("the game's frames differ in size")
    a, b = flat["luma"], empty["luma"]
    return [abs(a[i] - b[i]) > SUBJECT_DELTA for i in range(len(a))]


def browser_subject(flat):
    """The same thing for the browser, where the subject-free reference is the
    backdrop itself: with the grid off (`grid=0`) the only things on it are
    the model and its shadow, so a row's outer columns — which the model never
    reaches — say what that row's backdrop is."""
    w, h, lum = flat["w"], flat["h"], flat["luma"]
    edge = max(8, w // 25)
    mask = [False] * (w * h)
    for y in range(h):
        row = y * w
        strip = sorted(lum[row:row + edge] + lum[row + w - edge:row + w])
        bg = strip[len(strip) // 2]
        for x in range(w):
            mask[row + x] = abs(lum[row + x] - bg) > SUBJECT_DELTA
    covered = sum(1 for y in range(h) for x in range(edge)
                  if mask[y * w + x] or mask[y * w + w - 1 - x])
    if covered > 0.01 * h * edge * 2:
        sys.exit("the model reaches the frame's outer columns, so the backdrop "
                 "cannot be read off them — widen the framing")
    return mask


# ------------------------------------------------------------------- measure


def measure(off_path, on_path, mask, step=1):
    """How much of the subject `on` darkens, and how deeply.

    Every number is per MACHINE pixel, so it survives the two renderers not
    sharing a camera; none of them is normalised by the darkened area, so a
    broad faint wash cannot read as a narrow deep crease.
    """
    off, on = load(off_path), load(on_path)
    if (off["w"], off["h"]) != (on["w"], on["h"]):
        sys.exit(f"frames differ in size: {off['w']}x{off['h']} vs {on['w']}x{on['h']}")
    w, h = off["w"], off["h"]
    a, b = off["luma"], on["luma"]
    darks = []
    shade = 0.0
    for y in range(0, h, step):
        row = y * w
        for x in range(0, w, step):
            i = row + x
            if not mask[i] or a[i] <= LIT_FLOOR:
                continue
            d = 1 - b[i] / a[i]
            darks.append(d)
            if d > 0:
                shade += d
    if not darks:
        sys.exit("the subject mask caught no lit pixels — wrong frames?")
    darks.sort()
    n = len(darks)
    return {
        "subject": n,
        "shade": shade / n,
        "cover1": sum(1 for d in darks if d > COVER[0]) / n,
        "cover3": sum(1 for d in darks if d > COVER[1]) / n,
        "deep": darks[min(n - 1, n * DEEP_PCT // 100)],
        "darkest": darks[-1],
    }


# ------------------------------------------------------------------- the game


_RESTORE = []


def _restore(*_args):
    while _RESTORE:
        path, data = _RESTORE.pop()
        path.write_bytes(data)
        print(f"RESTORED {path} to its committed bytes", file=sys.stderr, flush=True)


atexit.register(_restore)
for _sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
    signal.signal(_sig, lambda s, _f: (_restore(), sys.exit(128 + s)))


def refuse_if_dirty():
    for path in (ENV_GD, TEST_GD):
        run = subprocess.run(["git", "status", "--porcelain", "--", str(path)],
                             cwd=ROOT, capture_output=True, text=True)
        if run.stdout.strip():
            sys.exit(f"{path.relative_to(ROOT)} has uncommitted changes. This script "
                     "edits that file and restores it from a byte copy; refusing to run "
                     "on top of someone's work. Commit or stash it first.")


def shoot_game(out, ssao=None, pose="iso"):
    """One game frame: `pose` picks the camera, `ssao` picks the pass.

    `ssao=None` renders the empty world at the same camera, which is what says
    which pixels are machine. Otherwise `factory_environment.gd` is forced on
    or off for exactly as long as the render takes, and at `pose="eye"` the
    harness's first-person pose is replaced by one standing beside the line.
    Every restore is a byte copy, registered before the write and fired by
    `finally`, `atexit` and every catchable signal — a Ctrl-C mid-render
    leaves both files as it found them, and says so.
    """
    if pose == "eye":
        args = [str(ROOT / "scripts" / "screenshot-scene.sh"), str(out), "%dx%d" % SIZE,
                "--no-hud", "--first-person"]
        backup = TEST_GD.read_bytes()
        patched, n = re.subn(r"func _enter_first_person\(\) -> void:\n(?:\t.*\n|\n(?=\t))*",
                             EYE_POSE, backup.decode())
        if n != 1 or EYE_POSE not in patched:
            sys.exit(f"could not find _enter_first_person() in {TEST_GD}")
        _RESTORE.append((TEST_GD, backup))
        TEST_GD.write_text(patched)
    else:
        args = [str(ROOT / "scripts" / "screenshot-scene.sh"), str(out), "%dx%d" % SIZE,
                "--no-hud", "--closeup", CLOSEUP]
    if ssao is not None:
        args[3:3] = ["--fixture", EYE_FIXTURE if pose == "eye" else FIXTURE]
        backup = ENV_GD.read_bytes()
        want = "env.ssao_enabled = %s" % ("true" if ssao else "false")
        patched, n = re.subn(r"env\.ssao_enabled = (?:true|false)", want, backup.decode())
        if n != 1:
            sys.exit(f"expected exactly one ssao_enabled in {ENV_GD}, found {n}")
        _RESTORE.append((ENV_GD, backup))
        ENV_GD.write_text(patched)
    try:
        run = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
        if "SCREENSHOT_OK" not in run.stdout:
            sys.exit(f"screenshot-scene.sh failed:\n{run.stdout[-2000:]}\n{run.stderr[-2000:]}")
    finally:
        _restore()


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
           f"?view=game&bare=1&grid=0&shot={name}&ao={ao:g}#{ASSET}")
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


# ------------------------------------------------------------ the constants


def read_const(name, text=None):
    text = AO_MJS.read_text() if text is None else text
    m = re.search(rf"export const {name} = ([0-9.]+);", text)
    return float(m.group(1)) if m else None


def write_const(name, value):
    text = AO_MJS.read_text()
    new, n = re.subn(rf"(export const {name} = )[0-9.]+;", rf"\g<1>{value};", text)
    if n != 1:
        sys.exit(f"expected exactly one {name} in {AO_MJS}, found {n}")
    AO_MJS.write_text(new)


def game_ssao():
    text = ENV_GD.read_text()
    out = {}
    for key in ("radius", "intensity", "power"):
        m = re.search(rf"env\.ssao_{key} = ([0-9.]+)", text)
        if not m:
            sys.exit(f"no ssao_{key} in {ENV_GD}")
        out[key] = float(m.group(1))
    return out


# -------------------------------------------------------------------- the run


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--probe", type=float, default=1.0,
                    help="browser AO strength to measure the response at (default 1.0)")
    ap.add_argument("--step", type=int, default=1,
                    help="sample every Nth pixel; 2 is four times faster and moves the "
                         "numbers in the third decimal")
    ap.add_argument("--keep", metavar="DIR",
                    help="keep the six frames here instead of a temp dir")
    ap.add_argument("--write", action="store_true",
                    help="paste the derived constants into web/ao.mjs")
    ap.add_argument("--json", action="store_true", help="machine-readable result")
    args = ap.parse_args()

    if not Path(CHROME).exists():
        sys.exit(f"no Chrome at {CHROME}")
    refuse_if_dirty()

    say = (lambda *a: None) if args.json else (lambda *a: print(*a, flush=True))

    # The reach first: the strength is derived against whatever kernel the
    # viewer is built with, so deriving one against a stale kernel bakes the
    # wrong shape in at a plausible number.
    ssao = game_ssao()
    want_radius = round(ssao["radius"] / VOXEL_UNIT, 3)
    have_radius = read_const("AO_RADIUS")
    if abs(want_radius - have_radius) > 1e-6:
        if not args.write:
            sys.exit(
                f"AO_RADIUS is stale: the game's ssao_radius is {ssao['radius']} m, "
                f"which is {want_radius:g} voxels, and web/ao.mjs says {have_radius:g}.\n"
                "Deriving a strength against the wrong reach would match the amount of "
                "shading with the wrong shape. Re-run with --write, or set it by hand."
            )
        say(f"AO_RADIUS {have_radius:g} -> {want_radius:g} voxels "
            f"(the game's ssao_radius {ssao['radius']} m)")
        write_const("AO_RADIUS", f"{want_radius:g}")

    # The shape, second. ssao_power is a gamma on the occlusion, and it is the
    # difference between a crease finder and a wash — a mismatch here cannot be
    # corrected by any strength, so it is checked the same way the reach is.
    have_power = read_const("AO_POWER")
    if abs(ssao["power"] - have_power) > 1e-6:
        if not args.write:
            sys.exit(
                f"AO_POWER is stale: the game's ssao_power is {ssao['power']}, and "
                f"web/ao.mjs says {have_power:g}.\n"
                "That is the falloff's shape, not its scale; re-run with --write."
            )
        say(f"AO_POWER {have_power:g} -> {ssao['power']:g} (the game's ssao_power)")
        write_const("AO_POWER", f"{ssao['power']:g}")

    tmp = Path(args.keep) if args.keep else Path(tempfile.mkdtemp(prefix="ao-calib-"))
    tmp.mkdir(parents=True, exist_ok=True)

    say(f"frames: {tmp}")
    say(f"game: ssao_radius {ssao['radius']} / ssao_intensity {ssao['intensity']} / "
        f"ssao_power {ssao['power']}")
    def game_pair(pose, tag):
        """The game's own shading at one pose, per machine pixel.

        Every frame is checked before it is believed. `test_screenshot.gd`
        applies a closeup on its second frame and builds the fixture before
        that, so a capture that fires early comes back as the empty world at
        the default framing — a plausible-looking PNG that would quietly
        become a calibration. Two things catch it: the machine has to cover a
        sane share of the frame, and the pass-on frame has to cover the SAME
        pixels as the pass-off one, because SSAO moves no geometry.
        """
        empty = tmp / f"{tag}-empty.png"
        flat = tmp / f"{tag}-flat.png"
        on = tmp / f"{tag}-ao.png"

        def subject_of(path, ref):
            mask = game_subject(load(path), ref)
            return mask, sum(mask) / len(mask)

        def mean_over(path, mask):
            lum = load(path)["luma"]
            n = sum(mask)
            return sum(l for l, m in zip(lum, mask) if m) / n if n else 0.0

        for attempt in range(1, 4):
            say(f"game [{tag}]: the empty world, to find what the machines cover ..."
                if attempt == 1 else f"game [{tag}]: re-shooting (attempt {attempt}) ...")
            shoot_game(empty, pose=pose)
            ref = load(empty)
            say(f"game [{tag}]: SSAO off ...")
            shoot_game(flat, ssao=False, pose=pose)
            say(f"game [{tag}]: SSAO on ...")
            shoot_game(on, ssao=True, pose=pose)
            mask, share = subject_of(flat, ref)
            on_mask, on_share = subject_of(on, ref)
            kept = sum(1 for a, b in zip(mask, on_mask) if a and b) / sum(mask) if any(mask) else 0.0
            off_lit, on_lit = mean_over(flat, mask), mean_over(on, mask)
            sane = MACHINE_SHARE[0] <= share <= MACHINE_SHARE[1]
            if sane and kept >= MASK_KEPT and on_lit <= off_lit:
                say(f"  the machine covers {share * 100:.1f}% of the frame; the pass keeps "
                    f"{kept * 100:.1f}% of it and takes it from {off_lit:.1f} to {on_lit:.1f}")
                return measure(flat, on, mask, args.step)
            say(f"  REJECTED: the machine covers {share * 100:.1f}% of the pass-off frame and "
                f"{on_share * 100:.1f}% of the pass-on one, which keeps {kept * 100:.1f}% of it "
                f"and takes its mean luma from {off_lit:.1f} to {on_lit:.1f}")
        sys.exit(
            f"three {tag} triples in a row were not a believable before/after: either the "
            "machine was not where the other frame had it, or the pass made it brighter. "
            "That is the screenshot harness capturing a frame before the fixture is built "
            "or before the pose is applied, not something a calibration can average out.")

    # The pose the constant comes from, and the pose the player sees. The
    # second is measured only so the PR, the README and anyone comparing the
    # browser with a play screenshot can see the size of the gap.
    game = game_pair("eye", "game-eye")
    iso = game_pair("iso", "game-iso")

    server = Server(tmp)
    try:
        say(f"browser: {ASSET}, AO off ...")
        flat = shoot_browser(server, "web-flat", 0, tmp)
        bmask = browser_subject(load(flat))
        say(f"browser: {ASSET}, AO at the probe strength {args.probe:g} ...")
        probe_png = shoot_browser(server, "web-probe", args.probe, tmp)
        probe = measure(flat, probe_png, bmask, args.step)

        if probe["shade"] <= 0:
            sys.exit("the browser darkened nothing at the probe strength — is the AO wired up?")
        # Two statistics, one knob: the strength that matches the light removed
        # over the whole machine, and the one that matches the deep end of it.
        # Fitting either alone leaves the other free to drift, so take the
        # strength between them and let the bands below judge what is left.
        by_shade = args.probe * game["shade"] / probe["shade"]
        by_deep = (args.probe * game["deep"] / probe["deep"]) if probe["deep"] > 0 else by_shade
        derived = round(math.sqrt(by_shade * by_deep), 3)
        say(f"  strength matching the light removed {by_shade:.3f}, "
            f"matching the deepest hundredth {by_deep:.3f} -> {derived:.3f}")

        # Scaling from the probe assumes the darkening is linear in the
        # strength, and it is not: at strength 1 most of the field dims by
        # less than one 8-bit level, so it rounds away and the probe reads the
        # browser as flatter than it is — which lands the first guess about a
        # sixth too dark. Correct it against a real render instead of against
        # that assumption, and keep correcting while it is worth a render.
        for attempt in range(REFINE_PASSES):
            say(f"browser: {ASSET}, AO at {derived:.3f} ...")
            got = shoot_browser(server, "web-derived", derived, tmp)
            achieved = measure(flat, got, bmask, args.step)
            fix_shade = game["shade"] / achieved["shade"] if achieved["shade"] else 1.0
            fix_deep = (game["deep"] / achieved["deep"]) if achieved["deep"] else fix_shade
            fix = math.sqrt(fix_shade * fix_deep)
            settled = abs(fix - 1) <= REFINE_TOL
            if settled or attempt == REFINE_PASSES - 1:
                break
            say(f"  it removes {1 / fix_shade:.2f}x the game's light and reaches "
                f"{1 / fix_deep:.2f}x its depth -> {round(derived * fix, 3):.3f}")
            derived = round(derived * fix, 3)
    finally:
        server.stop()

    cover_ratio = achieved["cover1"] / game["cover1"] if game["cover1"] else float("inf")
    deep_ratio = achieved["deep"] / game["deep"] if game["deep"] else float("inf")
    verdict = []
    if not settled:
        verdict.append(
            f"THE STRENGTH DID NOT SETTLE: {REFINE_PASSES} renders in, {derived:.3f} still "
            f"removes {achieved['shade'] / game['shade']:.2f}x the game's light and reaches "
            f"{deep_ratio:.2f}x its depth, and the correction is still moving it. Something "
            f"other than the strength differs between the two")
    if game["cover3"] < DEEP_FLOOR:
        verdict.append(
            f"the game darkens no part of the machine by more than {COVER[1] * 100:.0f}% at "
            f"these SSAO settings ({game['cover3'] * 100:.3f}% of it, against a floor of "
            f"{DEEP_FLOOR * 100:.2f}%) — there is no crease here to calibrate against, and "
            f"any strength at all would be browser-only shading")
    elif not COVER_BAND[0] <= cover_ratio <= COVER_BAND[1]:
        verdict.append(
            f"COVERAGE DIVERGES: the browser darkens {cover_ratio:.1f}x the share of the "
            f"machine the game does ({achieved['cover1'] * 100:.1f}% against "
            f"{game['cover1'] * 100:.1f}%). AO_STRENGTH cannot fix this — it scales every "
            f"darkened pixel alike. The reach is what differs: AO_RADIUS "
            f"{read_const('AO_RADIUS'):g} voxels / AO_OFFSET in web/ao.mjs against the "
            f"game's ssao_radius {ssao['radius']} m and ssao_power {ssao['power']}")
    if game["deep"] and not DEEP_BAND[0] <= deep_ratio <= DEEP_BAND[1]:
        verdict.append(
            f"DEPTH DIVERGES: the browser's 99th-percentile darkening is {deep_ratio:.1f}x "
            f"the game's ({achieved['deep'] * 100:.2f}% against {game['deep'] * 100:.2f}%) "
            f"even with the same mean shading — the two are shading different shapes")

    result = {
        "ssao": ssao,
        "derived_against": "first person beside the %s fixture" % EYE_FIXTURE,
        "ao_radius": read_const("AO_RADIUS"),
        "ao_power": read_const("AO_POWER"),
        "game_eye": game,
        "game_iso": iso,
        "browser_probe": args.probe,
        "probe": probe,
        "derived_strength": derived,
        "achieved": achieved,
        "cover_ratio": cover_ratio,
        "deep_ratio": deep_ratio,
        "current_strength": read_const("AO_STRENGTH"),
        "failures": verdict,
        "frames": str(tmp),
    }

    if not verdict and args.write:
        write_const("AO_STRENGTH", f"{derived:.3f}")

    if args.json:
        print(json.dumps(result, indent=2))
        return 1 if verdict else 0

    def row(name, d):
        print(f"  {name:<24} shade {d['shade'] * 100:5.2f}%   "
              f">1% {d['cover1'] * 100:5.1f}%   >3% {d['cover3'] * 100:5.1f}%   "
              f"p{DEEP_PCT} {d['deep'] * 100:5.2f}%   ({d['subject']} px of machine)")

    print()
    print("per machine pixel, against the same renderer with the pass off:")
    row("game, eye level", game)
    row(f"browser at {args.probe:g}", probe)
    row(f"browser at {derived:.3f}", achieved)
    print()
    print("the play camera, for the record — not what the constant is derived")
    print("against, because a 0.45 m world radius resolves almost nothing there:")
    row("game, isometric", iso)
    print()
    if verdict:
        for line in verdict:
            print(f"  FAIL: {line}")
        print()
        print(f"  no constant written; web/ao.mjs still says "
              f"AO_STRENGTH = {read_const('AO_STRENGTH')}")
        return 1
    where = ("written to web/ao.mjs" if args.write
             else f"web/ao.mjs says {read_const('AO_STRENGTH')}")
    print(f"  AO_STRENGTH = {derived:.3f};   ({where})")
    print(f"  derived against the game standing beside the line at "
          f"ssao_radius {ssao['radius']} / ssao_intensity {ssao['intensity']} / "
          f"ssao_power {ssao['power']}")
    print(f"  shading {achieved['shade'] * 100:.2f}% against the game's "
          f"{game['shade'] * 100:.2f}%, over {cover_ratio:.2f}x its coverage "
          f"and {deep_ratio:.2f}x its depth")
    return 0


if __name__ == "__main__":
    sys.exit(main())
