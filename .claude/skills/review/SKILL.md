---
name: review
description: Review a pull request you did not write, adversarially — build the branch in your own worktree, reproduce the failure it claims to fix, revert each fix to prove its test bites, and post the VERDICT comment the merge gates on. Use when asked to review or re-review a PR, when a branch is waiting on a gate before it merges, or when you are about to approve work somebody else wrote.
---

# Adversarial review

A review here is the last gate — whoever merges merges on it. The job is to
find what is wrong before `main` does, not to bless it, so **trust none of
the author's claims**: not the PR body, not the green checks, not the
screenshots, and not "the new test passes".

## 0. Your own worktree, your own build

Never edit, push to or merge the branch under review.

```
git fetch origin
git worktree add ../rev-<pr> <branch>        # your copy, not theirs
git -C ../rev-<pr> config core.hooksPath .githooks
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
cargo xtask                                  # a fresh worktree has no godot_project/bin/
```

Without `cargo xtask` every Godot run dies on `Identifier "SimBridge" not
declared`; a headless run that exits 137 instantly wants
`codesign -s - --force godot_project/bin/libsim_core.dylib`.

## 1. Re-run the issue's own reproduction, not the author's test

`gh pr view N`, then `gh issue view` **every** issue named in `Closes #N`, in
full. Each issue says how it was found — run that, here, on this branch. **An
issue is closed when the thing it describes has stopped happening**, not when
a test the author also wrote goes green.

## 2. Reproduce on the pre-fix commit, then show it gone on the tip

Name both commits in the review. A failure you never saw yourself is one you
cannot say is fixed; a fix written against a misread report passes its own
test and changes nothing.

## 3. Revert each fix in turn — the strongest check there is

For every new or changed test: **undo the fix in your working copy, keep the
test, and confirm the test fails.** A test that still passes without its fix
proves nothing and is worse than no test, because it will go on passing
straight through the regression. Say which tests you checked this way and
what each one printed.

This catches the commonest bad fix in this repo: **a guard given more entries
rather than made to refuse.** In #108, `test_fluid_gallery.gd`'s saturation
floor walked a hand-written list of the six fluids that existed, so a seventh
near-grey tint — precisely the bug the guard was added for — rendered
`FLUID_GALLERY_OK` and exit 0. Entries had been added; nothing had been made
to fail.

## 4. Count the markers yourself — exit 0 proves nothing

Both of these are measured in this repo on Godot 4.6.3, not folklore:

- **Godot exits 0 when a `--script` file fails to parse.** Every
  `godot_project/test_*.gd` therefore prints its own success marker,
  `scripts/verify-core-loop.sh` greps for each one (`FAIL: <script> exited 0
  but never printed '<marker>'`) and the run ends by printing how many it
  ran — 38 today. Check that line and the markers against the list, rather
  than checking the exit status. Count by hand carefully too:
  `grep -c '^run_godot_script' scripts/verify-core-loop.sh` answers **39**,
  because it also matches the function's own definition (issue #138 shipped
  that off-by-one into a comment, which is why the script prints the number
  now).
- **`assert()` does not stop the run.** Headless, it prints
  `SCRIPT ERROR: Assertion failed: <msg>` with a backtrace and abandons the
  rest of *that function* — but **the caller carries on** and the process
  exits 0. A three-line probe: a `_init` that calls a function whose
  `assert(false, …)` is followed by a print gets `PROBE_BEFORE`, the script
  error, no line from after the assert, then `PROBE_AFTER_CALL` and exit 0.
  So a fixture can print `BBQ_FIXTURE_FAIL:`, fire the shutter anyway, write
  a PNG and report `SCREENSHOT_OK` — #110, on merged `main`. When a branch
  adds a failure marker, break the thing deliberately and confirm the harness
  actually goes red.

## 5. Render it yourself and read the PNG

The body's images may be stale, from another build, or framed to miss the
problem. Re-render with `scripts/screenshot-scene.sh` (the
`screenshot-review` skill has the flags) and **look at the file** — at
1600x900, and at 1280x720 too if the change is visual. Check what the frame
*is*, not only what is in it: in #114 `--closeup=X,Y,S` was parsed by nothing,
silently dropped, and the run rendered a whole-factory frame at default zoom
with `SCREENSHOT_OK` and exit 0, so a reviewer measures a picture they did not
ask for. Any flag the harness accepts and ignores is a finding.

## 6. A false absolute is worse than a bug

A wrong line in `docs/DECISIONS.md`, a design doc or a `CLAUDE.md` is what the
next agent reasons *from* instead of measuring, so it stays true-by-citation
long after the code has moved — and a bug gets found by the next test run,
while this does not.

So **re-derive every number a PR writes into a doc**, from your own windows
and your own run, rather than re-running the author's script. #111 caught a
`DECISIONS.md` record whose mask, gap and containment figures did not
reproduce (three implementations of one written method gave 39.5, 42.5 and
30.4). When a claim is withdrawn, grep for it in every place it was written —
a correction that misses one of five records leaves the false version as the
one the next person finds. Hold a PR to the docs it changed, as well: #121 was
a destructive action with no warning, breaking a `docs/UI_UX.md` rule the same
PR had just written.

## 7. The failure modes this repo actually has

- gameplay logic drifted into GDScript (it belongs in `sim_core`), or content
  hardcoded in Rust that should be a field in `data/*.ron`;
- `f32`/`f64` in shared simulation state — determinism is a hard requirement;
- a silent fallback that hides a missing entry instead of failing loudly;
- a fix for the symptom the issue measured, leaving the cause it diagnosed;
- scope creep into files the branch does not own — other worktrees are in
  flight, and that is somebody else's merge conflict;
- a deviation from `docs/ARCHITECTURE.md`, `docs/UI_UX.md` or
  `docs/GAMEPLAY.md` with no `docs/DECISIONS.md` record; a missing
  `docs/DEVLOG.md` entry; `godot_project/project.godot` or `AGENTS.md` staged
  by accident.

## Re-reviewing a rework

A re-review is **not a diff of the rework**. Read the first review's blocking
findings and the author's reply (`gh pr view N --json comments,reviews`), and
assume neither that it was right nor that it was complete. Then:

- for each blocking finding, reproduce the original failure on the pre-rework
  commit and show it gone on the tip, naming both;
- review the rework itself as new code — a fix written under pressure to
  satisfy a reviewer is where regressions enter, and nobody has read these
  lines yet;
- **re-run everything, not the changed part.** The first review's "what I
  verified and it held" is now stale, because the tip has moved and the rework
  may have broken something that review cleared;
- check the PR body was updated to describe what the code now does.

## The verdict

GitHub refuses a self-review, and every PR here is authored by the same
account:

```
$ gh pr review 157 --approve --body "…"
failed to create review: GraphQL: Review Can not approve your own pull request (addPullRequestReview)
$ gh pr review 157 --request-changes --body "…"
failed to create review: GraphQL: Review Can not request changes on your own pull request (addPullRequestReview)
```

So post the verdict as a comment review, whose **first line is exactly**
`VERDICT: APPROVED` or `VERDICT: CHANGES REQUESTED`:

```
gh pr review N --comment --body-file <absolute path>
```

The merge gates on that line, because `gh pr view N --json reviewDecision`
stays `""` however thorough the review was — it is empty on #91 and #98, both
of which carry full CHANGES REQUESTED reviews.

Every finding gets `file:line`, what is wrong, what you did to prove it, and
what you want done. An approval is auditable too: say what you verified and
how, including which fixes you reverted and what failed. **Approve only if you
would be comfortable being the one who broke `main`.**

## Boundaries

Do not edit the branch, push to it, merge it, or open a competing fix PR.
Something the PR could fix is a requested change, not a new issue — file only
genuinely separate work (the `file-issue` skill is the gate), and name it in
the review.

Deeper: `docs/WORKFLOW.md` ("GitHub: branches, pull requests and issues"), the
`verify` and `screenshot-review` skills for the procedures this leans on, and
`open-pr` for the other side of the same gate.
