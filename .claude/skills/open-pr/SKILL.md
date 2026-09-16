---
name: open-pr
description: Ship finished work through GitHub — branch, push, open a pull request with gh, watch its checks and merge it. Use when a change is complete and needs to reach main, or when you are about to commit and find yourself on main, or when you catch yourself about to merge or push main locally.
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
its 30 headless markers and `PASS`), and `scripts/screenshot-scene.sh` **with
the PNG actually read** for anything presentational. A sim-core change needs a
headless test covering it before it merges (ARCHITECTURE §10).

Decide the DEVLOG question and the decision-record question here too — both
belong in the same PR as the change, not in a follow-up.

## 3. Push and open the PR

```
git push -u origin HEAD
gh pr create --fill          # --title/--body when it deserves a written-up description
```

`.github/pull_request_template.md` is the body's shape: what changed and why,
**the commands actually run to verify it and what they said**, the
DEVLOG/decision calls, and `Closes #N` for every issue it settles. For a
visual change, put what the screenshot showed **in words** — the PNG is
gitignored and cannot be attached from a headless run.

**File issues for anything you noticed and are not fixing** (the `file-issue`
skill) and list them in the PR body, so the deferral is reviewable rather than
buried in a session transcript.

## 4. Watch it land

An open PR is unfinished work.

```
gh pr checks --watch
gh pr merge --squash --delete-branch
```

**Squash by default** — intermediate commits are noise in a history that feeds
the changelog, and the squashed subject becomes the one Conventional Commit for
the change, so it must pass the same lint (`scripts/lint-commit-msg.sh`). Keep
a real merge commit only when each commit is a reviewed unit in its own right,
as in a ticketed epic.

**Do not merge** a PR whose checks are red or still running, one the owner
asked to review, or one that deviates from a design doc, bumps `gdext`, or
changes the save format. Say it is ready, give the URL, and stop.

## Afterwards

```
git switch main && git pull
```

so the next piece of work branches from what actually landed. Report the PR
URL and any issue numbers filed.

Full rule: `docs/WORKFLOW.md`, "GitHub: branches, pull requests and issues".
