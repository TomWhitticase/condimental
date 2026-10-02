# CONDIMENTAL — Tech debt

**Changes since last overview audit: 18** · **Debt total: 0 points**

Feature freeze: **off** — set by `scripts/tech-debt-status.sh --write`.
Last audit: none yet (the counter started 2026-10-01).
Audit baseline: `cb7d24e` — the `main` commit the change count runs from.

| Gauge | Threshold | What happens |
|---|---|---|
| Changes since last audit | **25** | An overview audit is due before the next feature starts |
| Debt total | **20 points** | Feature freeze starts: no new features |
| Freeze lifts below | **10 points** | A freeze holds until the total drops below this, so it pays down real debt rather than one item |

## Keeping the gauges

**Run `scripts/tech-debt-status.sh` before starting a feature** — it prints
both gauges live and says if a threshold is passed; the rules above read its
output, not the header. The header is a snapshot that can lag by a few PRs
when several were open at once. The change count is derived from git (merged
PRs on `main` since the audit baseline, one squash commit each), because a
hand-bumped number undercounts silently when two open PRs make the same edit.

- Every PR to `main` runs `scripts/tech-debt-status.sh --write` before it
  pushes (the `open-pr` skill's step 4), which refreshes the header and counts
  itself. Orca workers' PRs into an integration branch skip it; the
  integration PR into `main` counts once.
- The debt total must equal the sum of the items, item numbers must be
  unique, and every item heading must be in the exact format below —
  `scripts/check-invariants.sh` fails otherwise, so a silent merge of two
  edits to the total is caught. The freeze line must agree with the totals
  too: on at or above the freeze threshold, off below the lifting one.
- Every PR edits the header line, so a PR open while others merge will
  conflict there. Take either side, then run `--write` again — it recounts
  from git, so the side you took does not matter. Never run `--write` after
  the branch has merged; it would count the branch twice.

An **overview audit** is a session that reads across the codebase rather than
one change: it checks the design docs against the code, looks for duplicated
or drifting logic between `sim_core` and Godot, re-scores every item below,
deletes the ones that are paid off and adds what it finds. Its PR records the
date on the "Last audit" line and sets the baseline to the `main` commit it
branched from, so the audit itself is the first change counted after it.

## What belongs here — read before adding

This list is a brake on feature work, so every entry costs something. **Add
an item only when all of these hold:**

- It is structural: a shortcut or shape that makes *future* changes slower or
  riskier, not a bug (that is an issue) or a missing feature.
- You cannot fix it in the branch at hand — fixing it is the default
  (CLAUDE.md, "Fix what you notice").
- It is not already here. Raise an existing item's points instead of adding a
  near-duplicate.

Do not go looking for debt to record, and do not add an item to justify a
shortcut you are taking now — take the shortcut only if you would still add
the item afterwards. When in doubt, leave it out.

**Points**, by what the debt costs when nobody pays it:

- **1** — friction in one area: slower to change, easy to get wrong.
- **2** — has caused or will cause bugs, or every change in an area pays for it.
- **3** — cross-cutting, threatens determinism or save compatibility, or
  blocks a planned milestone.

Paying an item off deletes it in the same PR and lowers the total. Keep the
total in the header equal to the sum of the list.

## Items

Format, exactly: `### TD-N · <points> pt · <short title>`, then two to four sentences —
where it is (file or system), what is wrong with it, and why it costs us.
Number items in order added; never reuse a number.

_None yet._
