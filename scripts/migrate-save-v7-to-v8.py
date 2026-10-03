#!/usr/bin/env python3
"""Rewrite a version-7 CONDIMENTAL save as version 8, outside the game.

Usage:
    scripts/migrate-save-v7-to-v8.py ~/path/to/saves/*.ron          # dry run
    scripts/migrate-save-v7-to-v8.py --apply ~/path/to/saves/*.ron

On macOS the saves live in `~/Library/Application Support/Godot/app_userdata/
CONDIMENTAL/saves`; the game prints the resolved directory at startup
(`game_session.gd::resolved_saves_dir`). `--apply` keeps the original beside
the new file as `<slot>.v7.ron.bak`, which the load list ignores because it
only lists `*.ron`. A file at any other version is skipped, not touched.

Version 8 (2026-10-03) changed three things, and this rewrites a save for
each:

1. A MACHINE RUNS THE RECIPE THE PLAYER CHOSE. A version-7 machine picked its
   own recipe from its inputs and the save recorded the pick; version 8 reads
   the same field as the player's choice. Every machine keeps the recipe it
   was holding — on a working line, the one it was running — so a factory
   loads making what it made. A machine holding none stays unset.

2. MUSTARD IS GROUND AND MIXED, NOT COOKED. Three recipes are gone:
     crusher      mustard_mash  -> ground_mustard  (seed to a solid, on a belt)
     mixer        mustard_mix   -> mustard         (vinegar now goes in here)
     cooking_vat  mustard       -> (none)          (no vat makes mustard)
   A machine on one of them is moved to its replacement, or left unset where
   there is none; its batch in progress is abandoned and any input its new
   recipe does not take is dropped. A mustard line built for version 7 is
   plumbed the old way round, so it will not run until it is rebuilt: the
   crusher now wants a BELT to the mixer, the mixer a vinegar tank, and the
   vat is not part of it at all. The report names every machine affected.

   EVERY PIPE IS EMPTIED (tanks keep what they hold). The loader names each
   run afresh from the machines' recipes, and a run whose name changed would
   otherwise carry its old fluid across under the new name — mash relabelled
   as vinegar, never bought. A save file does not say which fluid a run held
   (that is derived on load), so this script cannot tell a renamed run from
   an unchanged one, and empties them all. That is NOT free: on a big
   factory it is thousands of units, much of it finished sauce standing
   between a vat and a bottler. The report prints the total destroyed, so
   the player knows before choosing `--apply`; the lines refill it, but the
   cases it would have become are not shipped.

   The production ledger forgets `mustard_mash` and `mustard_mix`, which no
   longer exist.

3. THE SAUCES UNLOCK KETCHUP, MUSTARD, BBQ (it was ketchup, BBQ, mustard).
   Gates are derived from boxes sold, and BBQ now waits on 100 boxes of
   MUSTARD. A save that had already opened BBQ (100 boxes of ketchup sold)
   but has not sold 100 boxes of mustard would come back with BBQ shut on a
   factory that may be built around it — so its `boxed_mustard` count is
   raised to 100, the least that keeps what the player had earned open. The
   report says when it does this. Nothing else in the ledger moves.
"""

import argparse
import pathlib
import re
import shutil
import sys

FROM_VERSION = 7
TO_VERSION = 8

# The v8 gates (`data/unlocks.ron`): mustard after this many boxes of
# ketchup, BBQ after this many of mustard. Both are 100 in v7 and v8.
GATE_BOXES = 100

# Old recipe -> (new recipe or None, the solid inputs the new recipe takes).
RECIPE_MOVES = {
    ("crusher", "mustard_mash"): ("ground_mustard", {"mustard_seed"}),
    ("mixer", "mustard_mix"): ("mustard", {"ground_mustard", "turmeric"}),
    ("cooking_vat", "mustard"): (None, set()),
}

GONE_FLUIDS = ("mustard_mash", "mustard_mix")

MACHINE_LINE = re.compile(
    r'\(kind: "(?P<kind>[a-z_]+)", tile: \((?P<x>-?\d+), (?P<y>-?\d+)\),.*body: Machine\('
)
RECIPE = re.compile(r'recipe: (None|Some\("([a-z_]+)"\))')
PROCESS = re.compile(r"process: (None|Some\(\(\d+, \d+\)\))")
INPUT = re.compile(r"input: \[(?P<items>[^\]]*)\]")
ITEM = re.compile(r'\("([a-z_]+)", (\d+)\)')


def migrate_machine(line, report):
    m = MACHINE_LINE.search(line)
    if not m:
        return line
    kind, tile = m.group("kind"), (int(m.group("x")), int(m.group("y")))
    found = RECIPE.search(line)
    old = found.group(2) if found else None
    move = RECIPE_MOVES.get((kind, old))
    if move is None:
        # Kept as it was. A v7 machine with no recipe could not hold a craft
        # (the v7 loader gave one its kind's first recipe), so there is no
        # orphaned process to clear here.
        return line
    new, takes = move
    line = RECIPE.sub(f'recipe: Some("{new}")' if new else "recipe: None", line, count=1)
    line = PROCESS.sub("process: None", line, count=1)

    def keep_inputs(match):
        kept = [f'("{i}", {n})' for i, n in ITEM.findall(match.group("items")) if i in takes]
        return "input: [" + ", ".join(kept) + "]"

    line = INPUT.sub(keep_inputs, line, count=1)
    report.append(
        f"  {kind} at {tile}: {old} -> {new or 'no recipe'}"
        + ("" if new else " (no vat makes mustard now)")
    )
    return line


FX_ONE = 65536  # `volume_bits` are I16F16 fixed point (`sim::save::FxBits`)


def empty_pipes(line):
    """Returns the line with its pipe emptied, and the units destroyed (None
    for a line that is not a pipe)."""
    if "body: Pipe(" in line:
        pattern, replacement = r"volume_bits: (-?\d+)", "volume_bits: 0"
    elif "body: Crossing(" in line:
        pattern, replacement = r"pipe: \(volume_bits: (-?\d+)", "pipe: (volume_bits: 0"
    else:
        return line, None
    found = re.search(pattern, line)
    units = int(found.group(1)) / FX_ONE if found else 0.0
    return re.sub(pattern, replacement, line, count=1), units


def drop_ledger_entries(text, report):
    """Removes `(id: "<gone>", kind: fluid, ...)` entries from the production
    ledger's item list, by balanced parentheses."""
    for gone in GONE_FLUIDS:
        start = text.find(f'(id: "{gone}", kind:')
        while start >= 0:
            depth, end = 0, start
            while True:
                if text[end] == "(":
                    depth += 1
                elif text[end] == ")":
                    depth -= 1
                    if depth == 0:
                        break
                end += 1
            end += 1
            # The entry's trailing comma and whitespace go with it.
            tail = re.match(r",\s*", text[end:])
            if tail:
                end += tail.end()
            text = text[:start] + text[end:]
            report.append(f"  production ledger: dropped '{gone}'")
            start = text.find(f'(id: "{gone}", kind:')
    return text


def keep_bbq_open(text, report):
    block = re.search(r"lifetime_boxes_sold: \{(?P<body>[^}]*)\}", text)
    if not block:
        return text
    sold = {k: int(v) for k, v in re.findall(r'"([a-z_]+)": (\d+)', block.group("body"))}
    if sold.get("boxed_ketchup", 0) < GATE_BOXES or sold.get("boxed_mustard", 0) >= GATE_BOXES:
        return text
    had = sold.get("boxed_mustard", 0)
    sold["boxed_mustard"] = GATE_BOXES
    indent = re.search(r"\n(\s*)\"", block.group(0))
    pad = indent.group(1) if indent else "    "
    closing = re.search(r"\n(\s*)\}$", block.group(0))
    body = "".join(f'\n{pad}"{k}": {v},' for k, v in sorted(sold.items()))
    new_block = "lifetime_boxes_sold: {" + body + "\n" + (closing.group(1) if closing else "  ") + "}"
    report.append(
        f"  boxes sold: boxed_mustard {had} -> {GATE_BOXES}, so the BBQ tier this save "
        "had already opened stays open (BBQ now unlocks after mustard)"
    )
    return text[: block.start()] + new_block + text[block.end():]


def migrate(save_path, apply):
    text = save_path.read_text()
    report = [f"{save_path.name}:"]

    found = re.search(r"schema_version:\s*(\d+)", text)
    if not found:
        return report + ["  REFUSED: no schema_version"], None
    version = int(found.group(1))
    if version != FROM_VERSION:
        return report + [f"  skipped: schema_version {version}, not {FROM_VERSION}"], None

    out = re.sub(r"schema_version:\s*\d+", f"schema_version: {TO_VERSION}", text, count=1)
    lines, pipes, destroyed = [], 0, 0.0
    for line in out.split("\n"):
        line = migrate_machine(line, report)
        line, units = empty_pipes(line)
        if units is not None:
            pipes += 1
            destroyed += units
        lines.append(line)
    out = "\n".join(lines)
    report.append(
        f"  {pipes} pipe tile(s) emptied, destroying {destroyed:.1f} units of fluid standing in "
        "them (tanks keep theirs); every machine keeps its recipe unless listed above"
    )
    out = drop_ledger_entries(out, report)
    out = keep_bbq_open(out, report)

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
