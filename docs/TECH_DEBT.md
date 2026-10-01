# CONDIMENTAL — Tech debt

**Changes since last overview audit: 0** · last audit: none yet (counter
started 2026-10-01) · **Debt total: 0 points**

| Gauge | Threshold | What happens |
|---|---|---|
| Changes since last audit | **25** | An overview audit is due before the next feature starts |
| Debt total | **20 points** | Feature freeze: no new features until the total is back under 10 |

## Keeping the counter

Every PR that merges to `main` adds 1 to "changes since last audit" in its own
branch (the `open-pr` skill's step 4). On a merge conflict over the number,
take the higher side and add 1 — never resolve it downwards.

An **overview audit** is a session that reads across the codebase rather than
one change: it checks the design docs against the code, looks for duplicated
or drifting logic between `sim_core` and Godot, re-scores every item below,
deletes the ones that are paid off, adds what it finds, and resets the counter
to 0 with today's date.

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

Format: `### TD-N · <points> pt · <short title>`, then two to four sentences —
where it is (file or system), what is wrong with it, and why it costs us.
Number items in order added; never reuse a number.

_None yet._
