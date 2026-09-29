#!/usr/bin/env python3
"""Rewrite a version-6 CONDIMENTAL save as version 7, outside the game.

Usage:
    scripts/migrate-save-v6-to-v7.py ~/path/to/saves/*.ron          # dry run
    scripts/migrate-save-v6-to-v7.py --apply ~/path/to/saves/*.ron

On macOS the saves live in `~/Library/Application Support/Godot/app_userdata/
CONDIMENTAL/saves`; the game prints the resolved directory at startup
(`game_session.gd::resolved_saves_dir`). `--apply` keeps the original beside
the new file as `<slot>.v6.ron.bak`, which the load list ignores because it
only lists `*.ron`. A file at any other version is skipped, not touched.

Version 7 lets a splitter split left, right, both ways as a T, or three ways,
so a splitter's body carries its mode: `Splitter((belt: ..., mode: left,
next_output: straight))` where version 6 wrote no mode. The same version
renamed the cursor's second side from `branch` to `left`, because a splitter
has two sides now and that one is the left. The loader refuses a v6 file
rather than guessing (sim_core/CLAUDE.md: "rejected, never migrated"), so
this one-off tool runs OUTSIDE the game.

Nothing is reconstructed and nothing is lost: every splitter in a version 6
save dealt straight on and to its left, which is exactly `mode: left`, and a
cursor on `branch` was on the side that is now called `left`.
"""

import argparse
import pathlib
import re
import shutil
import sys

FROM_VERSION = 6
TO_VERSION = 7

# `next_output` is written by a splitter and by nothing else in the file, so
# the field name alone finds every splitter body. The lookbehind keeps an
# already-rewritten `mode: left, next_output:` from gaining a second mode.
SPLITTER_CURSOR = re.compile(r"(?<!mode: left, )next_output:\s*(straight|branch)\b")


def migrate(save_path, apply):
    text = save_path.read_text()
    report = [f"{save_path.name}:"]

    found = re.search(r"schema_version:\s*(\d+)", text)
    if not found:
        return report + ["  REFUSED: no schema_version"], None
    version = int(found.group(1))
    if version != FROM_VERSION:
        return report + [f"  skipped: schema_version {version}, not {FROM_VERSION}"], None

    out = re.sub(
        r"schema_version:\s*\d+", f"schema_version: {TO_VERSION}", text, count=1
    )
    side = {"straight": "straight", "branch": "left"}
    out, splitters = SPLITTER_CURSOR.subn(
        lambda m: f"mode: left, next_output: {side[m.group(1)]}", out
    )
    report.append(f"  {splitters} splitter(s), each splitting left as it always did")

    if apply:
        backup = save_path.with_suffix(f".v{FROM_VERSION}.ron.bak")
        shutil.copy2(save_path, backup)
        save_path.write_text(out)
        report.append(f"  written; original kept as {backup.name}")
    else:
        report.append("  dry run, nothing written")
    return report, out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("saves", nargs="+", type=pathlib.Path)
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()

    for save in args.saves:
        lines, _ = migrate(save, args.apply)
        print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
