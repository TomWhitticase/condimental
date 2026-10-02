---
name: screenshot-review
description: Render the game scene to a PNG with scripts/screenshot-scene.sh and read it as an image — the project's only non-headless check. Use when reviewing how something looks, checking a UI panel/menu/hover/tooltip/ghost, confirming a model or belt junction renders, or after any presentation change to main.gd or a voxel asset.
---

# Visual review loop

Everything else in this repo is checked without a framebuffer. "Does the
factory look like `docs/UI_UX.md` describes" cannot be, and **this is the only
check that catches a stale content-id lookup** — a renamed model or icon key
ships silently with every headless test green, because the lookup falls back
and the scene just renders the wrong thing (UI_UX.md §2b point 12).

## Invocation

```
scripts/screenshot-scene.sh [output.png] [WIDTHxHEIGHT] [flags...]
```

Both positionals are **positional**: `$1` is the output path (default
`assets/voxels/previews/scene.png`, gitignored), `$2` is the resolution
(default `1600x900`), and **flags are only read from argument 3 onward**.

> **Always pass the resolution when you pass any flag.** The script's own
> header contains an example that omits it — that example is broken; the
> first flag gets eaten as the resolution. Use
> `scripts/screenshot-scene.sh out.png 1600x900 --build --hover`.

Then **read the PNG with the Read tool**. A run that is not looked at has
verified nothing. `1600x900` is the shipped window size and where nothing
scrolls — review there unless you are specifically checking a smaller window.

## Flags (read from the script; the root CLAUDE.md's list is stale)

**Which world you got** — every run prints `SCREENSHOT_FIXTURE: <name>`
beside `SCREENSHOT_OK`. An explicit `--fixture` beats a convenience flag's
implied one (the run prints a `SCREENSHOT_NOTE` saying so, and the flag's
panel/ticks still apply), and an unknown `--fixture` name is refused with a
non-zero status rather than quietly building the demo line (issue #70).

**What to build** — `--build` (demo line: belts turning north, then west
into a crusher's inlet face, a pipe run south — three facings in one frame; the same as `--fixture demo`)
· `--fixture NAME` where NAME is `machines | fluids | vat | splitter | chain
| stress | pipes | gallery | demo` · `--stress` (1500-tile serpentine, implies `--populate`, prints a
`SCREENSHOT_RENDER` line with object/primitive/draw-call counts) ·
`--pipe-gallery` · `--belt-gallery` (every belt shape and junction at every
facing) · `--fluid-gallery` (all nine fluids in nine named runs in one frame —
the frame issue #108's tint guard was not policing) · `--item-gallery` ·
`--warehouse` · `--bottleneck` (whole line backed up behind the vat; implies
`--populate`) · `--understaffed` · `--staff-shortfall` (line funded below its
next bill) · `--contracts` (signs every offer, runs to a mixed board) ·
`--payout` (holds the frame mid-celebration; implies `--fixture chain`) ·
`--populate` (run forward so product is moving).

**HUD / panels** — `--shop` (build menu, E) · `--market` (M) · `--board`
(contracts board, C) — add `--board-filter all|open|signed|locked` or a product id (`boxed_mustard`) to
press one of its filter chips · `--staff` (H, implies `--populate`) · `--production`
(P, implies `--populate`) · `--pause` · `--menu` (renders `menu.tscn`, writing
and removing a real save so the load list has a row) · `--no-hud` (hide the
HUD; the clipboard and hotbar cover the corners a junction usually lands in).

**Cursor / build mode** — `--hover`, `--hover-pipe`, `--hover-vat`,
`--hover-hopper`, `--hover-tank`, `--hover-dock`, `--hover-splitter` ·
`--ghost KIND` (valid state) · `--ghost-blocked KIND` (invalid state) ·
`--remove-mode` · `--stock` (buy a kit so the hotbar shows carried,
multi-digit and empty sockets) · `--hotbar N` (really press key N).

**Camera / framing** — `--first-person` · `--closeup TX,TY[,SIZE]` (aim at one
tile; SIZE is orthographic width in world units, fractional coords allowed
because a junction sits ON a tile boundary) · `--config TX,TY` (click that
hopper/tank open) with optional `--contents N` (press its picker slot N).

Note the `=` form for a negative coordinate, or it reads as a flag:
`--config=-1,0`.

## Review discipline (docs/UI_UX.md)

- **Judge at the real size on the real plate**, never on a blown-up preview.
  An icon is identified by the proportion of its parts at 64px (§2b point 31).
- **Check no panel lands on another plate.** The hotbar, legend and clipboard
  draw over the viewport and are not part of its visible rect — a tooltip or
  config panel low on screen lands in the hotbar, high on screen on the
  clipboard (§2b point 33). Render a hover near each corner.
- **Never colour alone** (§7). Every status needs a glyph, word, shape or
  position too. If the PNG reads only by hue, it is wrong.
- **A whole-factory shot is the wrong frame for "does this belt meet this
  machine"** — use `--closeup`, and `--no-hud` if the HUD covers the joint.
- **Budget a rework pass.** Looking changes the design (§2b point 9).

**The same command twice is the same picture** (issue #82): the script pins
the engine clock with `--fixed-fps 60` and seals the camera against the
machine it runs on, so re-rendering an unchanged tree is byte-identical and a
before/after pair differs only where the change does. Each run prints the
`SCREENSHOT_TICK` it stopped on. A baseline rendered before 2026-09-25 stopped
the sim on a different tick, so re-render it rather than diffing across that
change.

The script needs a display and a `godot` binary (`GODOT_BIN` overrides). Its
window is real but off-screen, the `--hover*` flags do not touch the mouse,
and it hides Godot as soon as Godot takes focus. Rendering therefore does
not disturb whoever is using the machine, and it does not need you to park
the pointer first. Add `--on-screen` only to watch a run. It is slow — pick the one fixture that shows
what you changed rather than sweeping every flag.

## Into a pull request

Every PR body embeds at least one of these PNGs (docs/WORKFLOW.md, "Every pull
request shows the change"). `scripts/pr-screenshot.sh shot.png` publishes one
to the `pr-media` branch and prints the markdown to paste — reading it first is
still the point; publishing an unread frame verifies nothing.

Deeper: `docs/UI_UX.md` (§2b for the traps, §5/§5e for panel and meter rules),
and the script's own header for the reasoning behind each fixture.
