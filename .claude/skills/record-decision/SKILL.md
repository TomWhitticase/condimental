---
name: record-decision
description: Record a non-trivial design decision in the repo's three places — the condensed rule in docs/DECISIONS_LOG.md, the full reasoning in docs/DECISIONS.md, and a note in the design doc it affects — plus the DEVLOG and commit-message rules. Use when you decided something the design docs do not cover, changed a settled rule, or are about to commit and wonder whether it needs writing down.
---

# Recording a decision

A decision is worth recording when it is **non-trivial and not already covered
by `docs/ARCHITECTURE.md`, `docs/UI_UX.md` or `docs/GAMEPLAY.md`** — a rule
the next agent would otherwise re-litigate or contradict. Implementation
detail that follows from an existing rule is not a decision. If it *conflicts*
with one of the three docs, do not record it unilaterally: those docs are
authoritative, and a deviation is discussed first.

## The three places, in this order

1. **The rule → `docs/DECISIONS_LOG.md`.** The condensed log, newest first.
   One bullet, opening with the date and a bolded title:

   ```
   - 2026-09-15 **Quality pass, ticket 5: polish sweep and docs.** <the rule,
     in a few sentences — what is true now, not the story of finding it.
     Name the functions/files it binds.> Full record in `docs/DECISIONS.md`.
   ```

   Write the rule, not the narrative. Someone reading only this file must be
   able to obey it. This log used to live in the root `CLAUDE.md` and moved
   out on 2026-09-16 — put new entries here, not in `CLAUDE.md`.

2. **The reasoning → `docs/DECISIONS.md`, Part A** ("CLAUDE.md decisions log
   (verbatim, newest first)"), at the top, under the **same date and title**
   as the log bullet so the two line up. This is the long form: what the
   options were, what was tried and rejected, what it cost, what evidence
   decided it. Parts B and C are the frozen full texts of UI_UX §2b and
   ARCHITECTURE §9a — do not add new records there.

   **The file is ~5,400 lines. Never read it whole.** Read its head for the
   conventions (`sed -n '1,30p' docs/DECISIONS.md`) and grep for the topic to
   find a nearby entry's format, then insert at the top of Part A.

3. **The note → the design doc it affects.** `docs/ARCHITECTURE.md` for
   stack/architecture/wire, `docs/UI_UX.md` for anything visual (a new §2b
   point keeps the next number — **point numbers are cited from code
   comments, so never renumber existing ones**), `docs/GAMEPLAY.md` for
   mechanics, progression or economy. A sentence or two, pointing at the
   record. If the decision resolves an open question, edit that doc's open-
   questions section in place (strike it through and mark it decided) rather
   than leaving a stale question behind.

A module-level invariant may also want a line in `sim_core/CLAUDE.md`,
`godot_project/CLAUDE.md` or `assets/voxels/CLAUDE.md`.

## Then: the DEVLOG

Separate question, asked on **every** commit: does this need an entry in
`docs/DEVLOG.md`? The `.githooks/pre-commit` hook prints a reminder and never
blocks — it cannot make the judgment for you.

- **Yes** for a decision, a milestone, an epic or ticket landing, or
  meaningful progress. Append at the top of the `## Log` section, newest
  first, as `### YYYY-MM-DD — short title`, written high-level: what changed
  and why it matters, not a diff summary.
- **No** for most small fixes.
- Either way, **check `## Current status` near the top and edit it in place if
  it is now stale.** That section describes *now*; the Log is append-only
  history.

## The commit

Conventional Commits, enforced by the `commit-msg` hook and a CI backstop
(`scripts/lint-commit-msg.sh`; enable hooks once per clone with
`git config core.hooksPath .githooks`):

```
<type>(<optional-lowercase-scope>)!: <lowercase imperative description>
```

Types: `feat fix docs style refactor perf test build ci chore revert`.
Header ≤ 100 chars, description starts lowercase, **no trailing period**.
Breaking change: `!` after the type/scope plus a `BREAKING CHANGE:` paragraph.
`Merge …` headers are exempt. A docs-only decision commit is `docs: …`.

Deeper: `docs/WORKFLOW.md` (commits, the DEVLOG decision, recording a
decision), `CONTRIBUTING.md`, `docs/README.md` (the doc index), and
`docs/DECISIONS_LOG.md`'s own header for when to read the log.
