#!/usr/bin/env python3
"""Rewrite a version-5 CONDIMENTAL save as version 6, outside the game.

Usage:
    scripts/migrate-save-v5-to-v6.py ~/path/to/saves/*.ron          # dry run
    scripts/migrate-save-v5-to-v6.py --apply ~/path/to/saves/*.ron

On macOS the saves live in `~/Library/Application Support/Godot/app_userdata/
CONDIMENTAL/saves`; the game prints the resolved directory at startup
(`game_session.gd::resolved_saves_dir`). `--apply` keeps the original beside
the new file as `<slot>.v5.ron.bak`, which the load list ignores because it
only lists `*.ron`. A file at any other version is skipped, not touched.

Version 6 lets a warehouse rack be reserved for one ingredient, so a rack's
placement carries a body: `WarehouseRack((filter: None))` where version 5
wrote a bare `WarehouseRack`. The loader refuses a v5 file rather than
guessing (sim_core/CLAUDE.md: "rejected, never migrated"), so this one-off
tool runs OUTSIDE the game.

Nothing is reconstructed and nothing is lost: every rack in a version 5 save
was open to every ingredient, and `filter: None` is exactly that. The stock a
save holds fits the same racks under the same rule, because open racks are
the shared pool version 5 already had.
"""

import argparse
import pathlib
import re
import shutil
import sys

FROM_VERSION = 5
TO_VERSION = 6

# A v5 rack body is the bare variant name, closing its placement tuple. The
# lookahead keeps an already-rewritten `WarehouseRack((` from matching twice.
BARE_RACK = re.compile(r"(body:\s*)WarehouseRack(?!\s*\()")


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
    out, racks = BARE_RACK.subn(r"\1WarehouseRack((filter: None))", out)
    report.append(f"  {racks} rack(s), each left open to every ingredient")

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
