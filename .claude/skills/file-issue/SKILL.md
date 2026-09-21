---
name: file-issue
description: Decide whether a finding you are not fixing deserves a GitHub issue, and file it with gh if it does. Use when you have spotted a real defect — a bug, a contradiction with a design doc, a performance problem — that the branch in hand genuinely cannot fix, because it needs an owner's call, breaks the save format, or is too large. Most findings should be fixed in place instead; this skill's first section is the gate that decides which.
---

# Filing an issue for what you are not fixing

A finding that lives only in a session's closing message is lost — no other
agent shares this session's memory, and the next one to touch that file starts
cold. That is what issues are for. But the backlog is a queue a human has to
read and rank, and **an issue nobody will ever choose to work on costs more
than it records**. The bar is not low.

## First: should this be an issue at all?

Three gates, in order. Any "no" and you do not file.

**1. Can you just fix it?** If the fix is a line or two in a file this branch
already touches — a stale comment, a wrong constant, a typo, a misleading log
line — fix it and mention it in the PR body. Same for a small, safe fix in the
area you are already working in, as long as the PR's title still describes the
PR. Filing is for what you *cannot* do now: it needs a call only the owner can
make, it breaks the save format, it contradicts a design doc, it is real work,
or it is in a part of the tree this branch has no business touching.

**2. Would anyone pick it off the board ahead of the next feature?** Be
honest. These fail the test and do not get filed:

- polish with no defect behind it ("it could look nicer at 4K"),
- speculative hardening — "add a test for X" where X has never broken,
- systems CLAUDE.md lists as deliberately not built yet; their absence is a
  plan, not a bug,
- a limit a design doc already records,
- anything you cannot state as something concrete going wrong for a player or
  an agent.

**3. Is it already filed?** A second sighting is a comment on the open issue,
never a new one.

**Budget: about one issue per pull request.** Wanting to file five means
either one underlying problem worth a single issue, or nits. File the one that
matters; the rest go in the PR body, where review can promote them.

This is also not the decision record. An issue is *work not yet done*; a rule
that is now true goes through the `record-decision` skill, and a milestone
goes in `docs/DEVLOG.md`.

## Check the backlog first

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
