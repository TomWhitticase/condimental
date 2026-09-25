---
name: open-pr
description: Ship finished work through GitHub — branch, push, screenshot the change, open a pull request with gh, watch its checks, and merge it once another session has posted VERDICT: APPROVED. Use when a change is complete and needs to reach main, or when you are about to commit and find yourself on main, or when you catch yourself about to merge or push main locally.
---

# Getting work onto `main`

**Every change reaches `main` through a pull request.** Not a local merge, not
a direct push — `main` is the branch other worktrees and sessions build on, and
a change that lands without a PR has no review artifact, no CI run and nothing
to read later. A `pre-push` hook refuses a direct push to `main`.

`gh` is authenticated in this environment. If `gh auth status` fails, stop and
say so — do not fall back to pushing `main`.

## 1. Be on a branch

```
git branch --show-current          # if this says main, you are not ready to commit
git switch -c <type>/<short-kebab-description>
```

The prefix is the Conventional Commit type of the work: `feat/`, `fix/`,
`docs/`, `chore/`, `refactor/`, `perf/`. Branch **before** the first commit.
If you already committed on `main`, move the commits rather than pushing them:

```
git switch -c <type>/<description>     # takes the commits with you
git branch -f main origin/main         # only if local main is otherwise unpushed
```

## 2. Verify before you push, not after

Per `docs/WORKFLOW.md`: `cargo test -p sim_core` and `cargo test -p voxgen`
always, `scripts/verify-core-loop.sh` for anything structural (it must print
every headless marker, the script count it prints, and `PASS`), and `scripts/screenshot-scene.sh` **with
the PNG actually read** for anything presentational. A sim-core change needs a
headless test covering it before it merges (ARCHITECTURE §10).

Decide the DEVLOG question and the decision-record question here too — both
belong in the same PR as the change, not in a follow-up.

## 3. Screenshot the change — every PR, no exceptions

**A PR body that embeds no image does not open.** A `PreToolUse` hook refuses
the command and CI fails the PR; both exist because "visual changes only" rots
into nobody rendering anything. The change under review is a frame, so show it.

```
scripts/screenshot-scene.sh shot.png 1600x900 --board   # the flag that shows it
```

Pick the one fixture or panel this branch touches (the `screenshot-review`
skill lists every flag), **read the PNG** — a shot nobody looked at has
verified nothing — and then publish it:

```
scripts/pr-screenshot.sh shot.png        # prints ![caption](url), ready to paste
```

That pushes to `pr-media`, an orphan branch never merged into `main`, and
prints a link form that renders in this private repo (a
`raw.githubusercontent.com` URL does not — do not hand-write one). The file
name becomes the caption, so name the PNG after its subject.

**A change with no visual surface still gets a screenshot**: show the state it
affects — the panel whose numbers moved, the line that backs up, or the demo
line (`--build`) as it stands at this commit. Say beside the image what it
shows and what to look at.

## 4. Push and open the PR

```
git push -u origin HEAD
gh pr create --title "…" --body "…"      # --fill holds no image, so it cannot pass
```

`.github/pull_request_template.md` is the body's shape: what changed and why,
**the commands actually run to verify it and what they said**, the screenshots
under `## Screenshots`, the DEVLOG/decision calls, and `Closes #N` for every
issue it settles. The image does not replace the words: what the screenshot
showed still goes in "How it was verified".

**Say in the PR body what you noticed and did not fix**, so the deferral is
reviewable rather than buried in a session transcript. A note in the body is
usually the right home for it; the `file-issue` skill decides the few that
also earn a backlog entry, and it starts by asking whether you should have
just fixed it instead.

## 5. Get it reviewed, then land it

**Every PR is reviewed before it merges** (the owner's rule, 2026-09-25).
Green CI is not a review, and a review after the merge does not count. The
verdict comes from a session that did not write the branch (the `review`
skill), posted as a comment review (`gh pr review <N> --comment`) whose first
line is `VERDICT: APPROVED`. `gh` returns that under `reviews`, not
`comments`, so check both, and read the **latest** verdict: an approval
followed by a later `CHANGES REQUESTED` is not an approval. Being told to
"make a PR and merge it" does not change this. If no reviewer's verdict is
there yet, give the URL, say the PR is waiting on review, and stop.

```
gh pr checks <N> --watch
gh pr view <N> --json reviews,comments --jq '[(.reviews[]|{t:.submittedAt,b:.body}),(.comments[]|{t:.createdAt,b:.body})]|sort_by(.t)|map(.b|split("\n")[0]|select(startswith("VERDICT:")))|last // "NO VERDICT"'
gh pr merge <N> --squash --delete-branch    # only if that printed exactly VERDICT: APPROVED
```

**Squash by default** — intermediate commits are noise in a history that feeds
the changelog, and the squashed subject becomes the one Conventional Commit for
the change, so it must pass the same lint (`scripts/lint-commit-msg.sh`). Keep
a real merge commit only when each commit is a reviewed unit in its own right,
as in a ticketed epic.

**Do not merge** a PR with no `VERDICT: APPROVED`, one whose checks are red
or still running, one the owner
asked to review, or one that deviates from a design doc, bumps `gdext`, or
changes the save format. Say it is ready, give the URL, and stop.

## Afterwards

```
git switch main && git pull
```

so the next piece of work branches from what actually landed. Report the PR
URL and any issue numbers filed.

Full rule: `docs/WORKFLOW.md`, "GitHub: branches, pull requests and issues".
