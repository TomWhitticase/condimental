---
name: wrap-it-up
description: Close out a session so the worktree can be deleted without losing anything — finish the work in hand, land every branch through a merged pull request, sweep for uncommitted/unpushed/stashed work, end with an unmissable all-caps SAFE or NOT SAFE banner, and on a green banner remove the worktree itself unless the user passed a custom ask. Use when the user says wrap it up, we're done, close this session, or before any worktree or session is torn down.
---

# Wrapping up a session

The point of this skill is one guarantee: **after it runs, deleting the
worktree loses nothing.** Everything either reached `main` through a merged
pull request, or is written down somewhere that outlives the session.

Finishing is the default. Filing an issue is the fallback, and a rare one —
see "The last resort" below.

## 1. Take stock, out loud, before touching anything

Only named work gets finished. List it:

```
git status --porcelain                  # uncommitted, including untracked
git log --branches --not --remotes --oneline    # commits on no remote
git stash list --format='%H %gs'        # SHARED with every other worktree
gh pr list --state open --json number,title,headRefName,mergeable
gh issue list --state open --search "author:@me"
```

Add what the tools cannot see: edits you were midway through, a test you
disabled "for a second", a question you meant to ask, a background task still
running. Say the list in the reply, then work it. A wrap-up that starts by
guessing what was left over is how work gets lost.

## 2. Finish what is in hand

Each item ends in exactly one of three states, and you must say which:

1. **Landed** — merged into `main` through a PR.
2. **Deliberately dropped** — reverted or deleted, with a sentence on why.
3. **Filed** — a GitHub issue, and only if it passes the gate in step 5.

"Left on a branch for later" is none of these. A branch in a worktree about to
be removed is deleted work with extra steps.

This includes record-keeping, which belongs in the same PR as the change, not
in a follow-up: the DEVLOG decision, and a decision record if a non-trivial
rule was set (the `record-decision` skill). Small things you noticed in files
you already had open are fixes now, not backlog (`docs/WORKFLOW.md`, "Fix it,
or file it").

## 3. Verify before you land, not after

`scripts/verify-core-loop.sh` for anything structural — it must print **PASS**,
the script count and every marker. `cargo test -p sim_core` and
`cargo test -p voxgen` always. Anything presentational gets
`scripts/screenshot-scene.sh` and the PNG **actually read**. The `verify`
skill has the full table and the gotchas.

Run the full gate once, here, rather than repeatedly during the session.

## 4. Land every branch through a merged PR

Follow the `open-pr` skill — it is the authority, and the hooks and CI enforce
it. The wrap-up-specific parts:

- **Every PR opened in this session is merged, explicitly handed over, or left
  open for one of the reasons below, before the session ends.** An open PR is
  unfinished work; an open PR whose worktree has been deleted is unfinished
  work nobody can finish.
- **A PR merges on a review verdict, and you cannot write your own.** A PR is
  reviewed adversarially by a reviewer that does not share your context
  (`docs/WORKFLOW.md`, and the `review` skill): launch a fresh subagent given
  only the PR number and the `review` skill, and merge on its
  `VERDICT: APPROVED`. You cannot supply that verdict yourself, however
  carefully you re-run your own tests, and neither can a subagent you briefed
  with your reasoning. **Wait for that verdict before you close**: the subagent
  dies with the session. So **a PR of yours ends the session open only if it is
  approved and its merge is held (by checks neither green nor passed when
  rerun locally, by the task, or by the owner), or it is explicitly handed
  over with its latest verdict named, or
  no reviewer could be launched at all.** A `CHANGES REQUESTED` verdict is work
  still in hand: fix it and get a fresh re-review, or hand the PR over saying
  so. For an approved PR left open, say which hold applies. Do not merge it to
  tidy the session up.
- **Check mergeability before waiting on checks.** A PR with conflicts gets no
  CI run at all, so `--watch` waits forever on nothing:

  ```
  gh pr view <N> --json mergeable,mergeStateStatus
  gh pr checks <N> --watch --interval 20
  # the latest VERDICT: line across reviews and comments (open-pr step 5)
  gh pr view <N> --json reviews,comments --jq '[(.reviews[]|{t:.submittedAt,b:.body}),(.comments[]|{t:.createdAt,b:.body})]|sort_by(.t)|map(.b|split("\n")[0]|select(startswith("VERDICT:")))|last // "NO VERDICT"'
  gh pr merge <N> --squash --delete-branch   # only on exactly VERDICT: APPROVED
  ```

- **Do not merge** a PR with no `VERDICT: APPROVED`, one whose checks are
  still running or red and not yet passed locally (a red check whose own
  command passes locally is met — `open-pr`, `--admin`), one the owner
  asked to review, or one that deviates from a design doc, bumps `gdext` or
  changes the save format. A missing verdict is not an ending at all — launch
  the reviewer, above — and `CHANGES REQUESTED` is rework, also above. A
  check that fails locally too, or any of the four sign-off cases, leaves an
  *approved* PR open on a
  named hold, which is a red banner (step 7) rather than a judgement call to
  make on the way out.
- The lint ratchet counts **files**, not lines: new Rust almost always needs
  `cargo fmt -p <crate>` before CI goes green.

## 5. The last resort: an issue

Only when finishing now would be *wrong* — it needs a call only the owner can
make, it breaks the save format, it contradicts a design doc, or it is
genuinely bigger than the time left. Then apply the real test from
`docs/WORKFLOW.md`: **would a reasonable person pick this off the board ahead
of the next feature?** If not, it is not an issue; put it in the PR body,
where review can promote it.

Being at the end of a session is **not** a reason to file. "I am wrapping up"
converts cheap fixes into backlog that a human then has to rank, and this repo
already has more of that than it has people to work it. Budget stays roughly
one issue per PR. If you do file, the issue says why it was not fixed, and the
`file-issue` skill's gate decides — it starts by asking whether you should
simply have fixed it.

## 6. Prove the worktree is safe to delete

Every line here must come back empty or accounted for:

```
git status --porcelain                          # nothing, not even untracked
git log --branches --not --remotes --oneline    # nothing unpushed
git stash list                                  # nothing of YOURS
gh pr list --state open                         # nothing of this session's
```

Then:

- **Look at untracked files before deleting them.** `git worktree remove`
  refuses while they exist, and reaching for `--force` is how a file someone
  wanted disappears. Read each one, say what it was, then remove it.
- **The stash stack is shared with every other worktree and session.** Never
  bare `git stash pop`. Drop only entries you pushed, found by your own tag.
- **Anything worth keeping from the scratchpad is already out of it** — the
  scratchpad dies with the session. PR screenshots are safe because
  `scripts/pr-screenshot.sh` pushed them to the `pr-media` branch; that orphan
  branch is never deleted in a cleanup sweep, despite looking abandoned.
- **No background task is still running.** Check before declaring done.
- **Never `pkill godot` by name.** Other worktrees run headless Godot; kill by
  a PID you captured yourself, or not at all.

Removing the tree is step 8, and only after the banner. `ExitWorktree` is a
no-op for a worktree this session did not create, so do not reach for it.

## 7. Say it so it cannot be missed

End with a banner. Use a fenced `diff` block — it is the one thing that
renders in **colour** in the terminal (`+` green, `-` red). ANSI escapes echoed
from Bash do not work: tool output is not reliably shown to the user.

Two questions, and they are **not** the same one. *Is the work finished?*
decides the banner. *Would deleting the tree lose anything?* decides what you
say under it — pushed work is safe even when it has not landed, and local-only
work is not safe however nearly done it is. Answer both, explicitly.

**Everything landed** — green, and only if every box in step 6 is empty:

````
```diff
+ =====================================================
+  WRAPPED UP — SAFE TO CLOSE SESSION AND DELETE TREE
+ =====================================================
```
````

**Anything outstanding** — red, and say exactly what and where it lives, so
the next session can pick it up without reading a transcript:

````
```diff
- =====================================================
-  NOT WRAPPED UP — DO NOT DELETE THIS WORKTREE YET
- =====================================================
```
````

Under either banner, list: the PR URLs and whether each merged, any issue
numbers filed and why they were not fixed, and either "removing this
worktree now" (step 8) or the exact removal command and why you are not
running it. Under the red one, add what is unfinished, where it is (branch,
file, PR), the next concrete step, and — separately and in as many words —
**whether the tree can still be deleted safely**. "PR #N is pushed and
approved, its merge held for the owner; the worktree can go, the work cannot
be lost" is a red banner with a clean tree, and saying only one half of that
misleads. Name the PR you actually held: this example used to cite #162, which
had in fact merged with no hold and no review at all, and a reader who checks
the number learns to trust the rest of the banner less.

**Never print the green banner with work outstanding.** The banner is a
promise; a wrong one costs someone their work, or merges something nobody
reviewed. When in doubt, print the red banner and say what you are unsure of.

## 8. Close the worktree yourself

The owner's rule (2026-10-01): **a plain `wrap-it-up` that ends on the green
banner removes its own worktree.** The user should not have to paste a
command to finish what they asked for. Remove it with the Orca CLI, which
deregisters the worktree from Orca and removes it from git in one step:

First, before the banner, prove the tree is a linked Orca worktree and not
the primary checkout. Orca registers the primary checkout as a worktree too,
so naming *a* worktree proves nothing. This prints `REMOVABLE <path>` only
when `isMainWorktree` is false and Orca's path is git's top level, which holds
from any subdirectory:

```
orca worktree current --json | python3 -c '
import json, subprocess, sys
w = json.load(sys.stdin)["result"]["worktree"]
top = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
print("REMOVABLE " + w["path"] if not w["isMainWorktree"] and w["path"] == top else "KEEP")'
```

Then, last of all (below), remove that exact path:

```
orca worktree rm --worktree path:<path>    # never --force
```

It deletes the checked-out branch only when Orca can prove it is merged. A
squash-merged branch usually cannot be proved merged, so it often survives,
which is expected and not a failure. It is merged, and
`git branch -D <branch>` from any other checkout clears it. Leave `--force`
off. If the command refuses, it returned and you are still running.
Something is still in the tree that step 6 missed, so the green banner was
wrong. Follow it straight away with the red banner, saying what Orca found
and that the worktree was kept.

**Do not remove it** when any of these is true. Print the removal commands
under the banner instead:

- **The banner is red.** Work is outstanding, and the tree may be the only
  copy of it.
- **The user passed a custom ask with the skill** (any argument after
  `/wrap-it-up`, such as "wrap it up but keep the worktree", or "wrap up
  #212 only"). A plain invocation means close. Anything added is the user
  steering, so do what it says and leave the worktree in place unless it
  says to remove it.
- **The check above printed `KEEP`, or failed.** This is the primary checkout,
  `orca worktree current` named no worktree, or its path is not this tree.
  Then hand over:

  ```
  git -C <main-checkout> worktree remove <this-worktree>
  git -C <main-checkout> branch -D <branch>
  ```

**It is the very last thing you do.** The command deletes the directory the
shell is running in, and Orca may close this session's terminal with it, so
nothing after it can be relied on to run or be seen. Write the banner and
everything under it first, ending with "removing this worktree now". Then
make the removal the final tool call.

Deeper: `docs/WORKFLOW.md` ("GitHub: branches, pull requests and issues",
"Closing out a session", "Fix it, or file it — and prefer fixing it",
"Orchestration (Orca)"), and the `open-pr`, `review`, `verify`, `file-issue`
and `record-decision` skills.
