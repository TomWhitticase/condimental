#!/usr/bin/env python3
"""Rewrite a version-4 CONDIMENTAL save as version 5, outside the game.

Usage:

    scripts/migrate-save-v4-to-v5.py ~/path/to/saves/*.ron          # dry run
    scripts/migrate-save-v4-to-v5.py --apply ~/path/to/saves/*.ron

macOS saves live in `~/Library/Application Support/Godot/app_userdata/
CONDIMENTAL/saves`; the game prints the resolved directory at startup
(`game_session.gd::resolved_saves_dir`). `--apply` keeps the original beside
the new file as `<slot>.v4.ron.bak`, which the load list ignores because it
only lists `*.ron`. A file of any other version is skipped, not touched.

Version 5 gates each sauce on boxes of a named product sold, where version 4
counted paid contract windows regardless of what shipped. The loader refuses a
v4 file rather than guessing (sim_core/CLAUDE.md: "rejected, never migrated"),
so this is a one-off tool run OUTSIDE the game.

The new figure is RECONSTRUCTED, not invented. A save records, per contract
still on the books, how many windows it satisfied; data/contracts.ron records
what each window shipped. Boxes sold per item is the sum of those products.
Windows belonging to contracts since struck off are attributed to the entry in
`contracts_lost` when exactly one is named, since a contract that leaves the
board leaves its id there and nowhere else.

The reconstruction is then CHECKED against an independent number the save
also holds: every dock's `shipped_total` counts boxes that came off its belt,
so boxes shipped to docks must equal boxes sold plus boxes still standing on
them. When those agree the reconstruction is exact rather than a floor.
"""

import argparse
import pathlib
import re
import shutil
import sys

FROM_VERSION = 4
TO_VERSION = 5


def contract_mixes(path):
    """{contract id: [(item, boxes per window)]} from data/contracts.ron."""
    text = path.read_text()
    mixes = {}
    for block in re.finditer(r'Contract\(\s*id:\s*"([^"]+)"(.*?)\n\s*\),', text, re.S):
        cid, body = block.group(1), block.group(2)
        mix = re.search(r"mix:\s*\[(.*?)\]", body, re.S)
        if mix:
            mixes[cid] = [
                (m.group(1), int(m.group(2)))
                for m in re.finditer(r'\(\s*"([^"]+)"\s*,\s*(\d+)\s*\)', mix.group(1))
            ]
    return mixes


def migrate(save_path, mixes, apply):
    text = save_path.read_text()
    report = [f"{save_path.name}:"]

    found = re.search(r"schema_version:\s*(\d+)", text)
    if not found:
        return report + ["  REFUSED: no schema_version"], None
    version = int(found.group(1))
    if version != FROM_VERSION:
        return report + [f"  skipped: schema_version {version}, not {FROM_VERSION}"], None

    windows = int(re.search(r"lifetime_deliveries:\s*(\d+)", text).group(1))

    active = re.search(r"\n  contracts:\s*\[(.*?)\n  \],", text, re.S).group(1)
    signed = [
        (m.group(1), int(m.group(2)))
        for m in re.finditer(
            r'id:\s*"([^"]+)".*?deliveries_made:\s*(\d+)', active, re.S
        )
    ]
    lost_block = re.search(r"\n  contracts_lost:\s*\[(.*?)\n  \],", text, re.S)
    lost = (
        [m.group(1) for m in re.finditer(r'\(\s*"([^"]+)"', lost_block.group(1))]
        if lost_block
        else []
    )

    sold = {}
    for cid, made in signed:
        for item, per in mixes.get(cid, []):
            sold[item] = sold.get(item, 0) + made * per
        report.append(f"  {cid}: {made} windows")

    attributed = sum(made for _, made in signed)
    unattributed = windows - attributed
    exact = True
    if unattributed > 0:
        if len(lost) == 1:
            for item, per in mixes.get(lost[0], []):
                sold[item] = sold.get(item, 0) + unattributed * per
            report.append(
                f"  {lost[0]} (struck off): {unattributed} windows, "
                f"the only contract the save's own lists leave unaccounted"
            )
        else:
            exact = False
            report.append(
                f"  WARNING: {unattributed} of {windows} windows cannot be "
                f"attributed ({len(lost)} contracts struck off) — the figure "
                f"below is a floor"
            )

    shipped = sum(int(m) for m in re.findall(r"shipped_total:\s*(\d+)", text))
    standing = sum(
        int(m.group(2))
        for block in re.finditer(r"Dock\(\(contents:\s*\[(.*?)\]", text, re.S)
        for m in re.finditer(r'\(\s*"([^"]+)"\s*,\s*(\d+)\s*\)', block.group(1))
    )
    total_sold = sum(sold.values())
    if shipped == total_sold + standing:
        report.append(
            f"  CHECK PASSED: {shipped} boxes reached the docks = "
            f"{total_sold} sold + {standing} still standing"
        )
    else:
        exact = False
        report.append(
            f"  CHECK FAILED: {shipped} boxes reached the docks but "
            f"{total_sold} sold + {standing} standing = {total_sold + standing}"
        )

    ledger = ", ".join(f'"{k}": {v}' for k, v in sorted(sold.items()))
    report.append(f"  lifetime_boxes_sold: {{{ledger}}}{'' if exact else '  (FLOOR)'}")

    out = re.sub(
        r"schema_version:\s*\d+", f"schema_version: {TO_VERSION}", text, count=1
    )
    out = re.sub(
        r"(\n  lifetime_deliveries:\s*\d+,)",
        rf"\1\n  lifetime_boxes_sold: {{{ledger}}},",
        out,
        count=1,
    )
    if out == text:
        return report + ["  REFUSED: nothing was rewritten"], None

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
    ap.add_argument(
        "--contracts",
        type=pathlib.Path,
        default=pathlib.Path(__file__).resolve().parent.parent / "data" / "contracts.ron",
        help="the content file the mixes come from (default: this repo's)",
    )
    ap.add_argument("--apply", action="store_true")
    args = ap.parse_args()

    mixes = contract_mixes(args.contracts)
    for save in args.saves:
        lines, _ = migrate(save, mixes, args.apply)
        print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
