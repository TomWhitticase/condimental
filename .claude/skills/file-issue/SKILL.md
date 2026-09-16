---
name: file-issue
description: File a GitHub issue for something noticed but deliberately not fixed, using gh. Use when you spot a bug, a contradiction between code and a design doc, a stale number, a missing test or a performance problem, and decide it is out of scope for the work in hand — the deferral is not finished until the issue exists.
---

# Filing an issue for what you are not fixing

**Deciding something is out of scope is only half the decision; the other half
is filing it.** A finding that lives only in a session's closing message is
lost — no other agent shares this session's memory, and the next one to touch
that file starts cold. The bar is low on purpose: **if it was worth putting in
a report, it is worth an issue.**

This is not a substitute for the decision record. An issue is *work not yet
done*; a rule that is now true goes through the `record-decision` skill, and a
milestone goes in `docs/DEVLOG.md`.

## First: is it already filed?

```
gh issue list --state open --search "<a distinctive word from the finding>"
```

If it exists, add a comment with what this session learned rather than filing
a second one — a reproduction, a narrower cause, a second sighting.

## Then file it

```
gh issue create --title "<imperative summary>" \
  --label deferred --label area:sim \
  --body "$(cat <<'BODY'
## What is wrong
…
## Where
`sim_core/src/sim/world.rs`, `resolve_sink`
## How it was found
…
## Why it was deferred
…
## What the fix probably is
…
BODY
)"
```

`.github/ISSUE_TEMPLATE/deferred-finding.md` is the same shape; the template
is what a human sees in the web UI, this is what you use from the CLI.

**Title in the imperative, like a commit subject** — "refuse a belt feeding a
machine from a non-inlet face", not "belt feeding bug". It is the line
everyone reads in a list of forty.

**The five headings each earn their place, and the next agent starts cold:**

- **Where** — file *and* function. "Somewhere in the Godot layer" cannot be
  acted on.
- **How it was found** — the `cargo test` name, the `scripts/screenshot-scene.sh`
  flags, the fixture in `test_layouts.gd`. This is what turns a re-investigation
  into one command. Say if a number came off a **debug** build (`cargo xtask`
  builds debug unless passed `--release`).
- **Why it was deferred** — out of scope, too large, or the owner's call. If
  it is the owner's call, add `owner-decision` and state the question as a
  question.
- **What the fix probably is** — even roughly. Name what it would break:
  saves, the shipped demo line, a fixture, a design doc needing an amendment.

## Labels

`deferred` on everything filed this way. One `area:` label — `area:sim`,
`area:godot`, `area:assets`, `area:docs`, `area:tooling`. `owner-decision`
when it is blocked on a human. Stock `bug` / `enhancement` / `documentation`
for kind. Do not invent new labels for a single issue.

## Afterwards

Mention the issue number in your final message and in the PR body of the work
that found it, so the deferral is visible at review time. Work that fixes an
issue closes it with `Closes #N` in its **PR body** — not by hand later.

Full rule: `docs/WORKFLOW.md`, "GitHub: branches, pull requests and issues".
