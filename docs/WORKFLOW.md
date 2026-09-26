# CONDIMENTAL — Workflow

What to run, what to commit, what to record, and how milestone-sized work is
orchestrated. [ARCHITECTURE.md](ARCHITECTURE.md), [UI_UX.md](UI_UX.md) and
[GAMEPLAY.md](GAMEPLAY.md) say *what* to build; this one says how.

## Working conventions

- ARCHITECTURE.md governs stack/architecture, UI_UX.md anything visual,
  GAMEPLAY.md mechanics/economy. Do not deviate without discussing it first.
- **Prefer small, playable increments** over big upfront systems, and **use
  plan mode** for anything touching architecture or adding a major system.
  Small fixes do not need plan mode.
- **Content changes never require a Rust change.** If an item, recipe,
  machine, contract or price needs code, a data field is missing — add the
  field (ARCHITECTURE.md §3.6, §10).
- **Cover every sim-core change with headless tests** before merging. No
  `f32`/`f64` in shared simulation state (§3.9), `.tscn`/`.tres` stay text,
  and a `gdext` bump is a reviewed decision (§10).

## Build

`cargo xtask` (aliased in `.cargo/config.toml`) is the whole build: it runs
`cargo build -p sim_core`, copies the cdylib into `godot_project/bin/` where
`sim_core.gdextension` expects it, and stages `assets/voxels/models/` and
`icons/` into `godot_project/voxels/`. Both destinations are gitignored;
never hand-copy into them. **It builds a DEBUG `sim_core`** unless passed
`--release`, which matters whenever you read a performance number — every
timing in the decisions log is a debug measurement. Regenerating voxel models
is a separate, explicit step: `scripts/build-voxels.sh` (models + gitignored
preview PNGs + committed icons), or `cargo run -p voxgen --bin voxgen|voxicons`;
`vengi-thumbnailer` renders the previews and icons and needs a real video
driver (`SDL_VIDEODRIVER=dummy` fails). **Godot needs `--import` twice on a
cold `.godot/` cache** — the `.vox` importer is an editor plugin and only
registers on the pass after the first scan; both verification scripts do this.

## Verification

**The primary loop is headless** (ARCHITECTURE.md §7): `cargo check` /
`cargo test` on the sim core, no Godot process. Open the editor only for
milestone-level visual checks. Both Godot scripts honour `GODOT_BIN`.

| Command | What it proves | Cost |
|---|---|---|
| `cargo test -p sim_core` | Sim invariants: conservation, determinism, economy, save/load. 28 integration test files under `sim_core/tests/`, plus unit tests. | fast |
| `cargo test -p voxgen` | Voxel-level asset properties — that a machine's inlet/outlet is a genuinely empty opening, which no mesh or screenshot can tell from a painted rectangle. | fast |
| `scripts/verify-core-loop.sh` | Everything: `cargo test -p sim_core`, `cargo test -p voxgen`, `cargo xtask`, two `--import` passes, an import sanity check, then **every headless Godot script** through the real `gdext` JSON boundary, and prints how many it ran. | slow |
| `scripts/verify-scaffold.sh` | The milestone-1 Rust ↔ Godot round trip alone (`cargo xtask`, prime the cache, `test_round_trip.gd`). Re-run this on a quiet machine before treating a `gdext` load failure as a regression. | medium |
| `scripts/screenshot-scene.sh` | The one NON-headless check (below). | slow |

**Every headless Godot test prints its own success marker** and
`verify-core-loop.sh` greps for it: Godot exits 0 even when a `--script` file
fails to *parse*. A new `test_*.gd` must print a marker and be registered.

### The non-headless check

`scripts/screenshot-scene.sh [out.png] [WIDTHxHEIGHT] [flags…]` renders
`main.tscn` (default `1600x900`, writing to the gitignored
`assets/voxels/previews/scene.png`). It is the only thing that catches stale
content-id lookups and other presentation bugs that ship silently with every
test green (UI_UX.md §2b point 12) — **run it for any presentation change and
read the PNG.** The world starts empty, so with no flags it photographs bare
floor. **The same command twice is the same picture** (issue #82): the
script pins the engine clock with `--fixed-fps 60`, so the warm-up before the
shutter always spends the same simulation ticks and every fade is at the same
point, whatever else the machine is doing — two renders of one command are
byte-identical, and each prints the `SCREENSHOT_TICK` it stopped on. The
camera is sealed for the run as well, so a key held in another window cannot
pan the shot between the aim and the shutter. Invoke
`test_screenshot.gd` through the script, not by hand; without the flag it
refuses to render. A baseline taken before 2026-09-25 is a different moment,
so re-render rather than diff across that change. The script's header explains every flag; grouped, they are:

- **Fixtures:** `--build`, `--fixture <machines|fluids|vat|splitter|chain|
  stress|pipes|gallery|demo>` (an explicit one WINS over the flags below,
  which each imply a fixture; an unknown name is refused, and every run
  prints the `SCREENSHOT_FIXTURE` it built), `--stress`, `--pipe-gallery`, `--belt-gallery`,
  `--fluid-gallery`, `--item-gallery`, `--warehouse`,
  `--bottleneck`, `--understaffed`, `--staff-shortfall`, `--contracts`,
  `--payout`; `--populate` runs one forward; also `--stock`, `--hotbar N`
- **Screens:** `--menu`, `--pause`, `--shop`, `--market`, `--board`,
  `--staff`, `--production`, `--no-hud`
- **Build mode:** `--ghost KIND`, `--ghost-blocked KIND`, `--remove-mode`
- **Tooltips:** `--hover`, and `--hover-` + `pipe|vat|hopper|tank|dock|splitter`
- **Camera / panels:** `--first-person`, `--closeup TX,TY[,SIZE]`,
  `--config TX,TY` (+ `--contents N`). A value starting with `-` needs the
  `=` form: `--config=-1,0`.

### What CI runs

The same commands, on every pull request, so that a check skipped in a
session is still caught before the change reaches `main`. Workflows live in
`.github/workflows/`; each one's header says why it exists.

| Workflow | Runs | On |
|---|---|---|
| `rust.yml` | `cargo test -p sim_core`, `cargo test -p voxgen`, `cargo xtask`, the fmt/clippy ratchet, and a regenerate-and-diff of `assets/voxels/models` | every PR and push to `main` |
| `invariants.yml` | `scripts/check-invariants.sh`, `shellcheck`, `actionlint` | every PR and push to `main` |
| `commitlint.yml` | `scripts/lint-commit-msg.sh` over the whole range | every PR and push to `main` |
| `pr-screenshot.yml` | `scripts/check-pr-screenshot.sh` — the body embeds an image | every PR |
| `verify.yml` | `scripts/verify-core-loop.sh` in full, Godot and all | nightly, on demand, or on a PR labelled `full-verify` |
| `audit.yml` | `cargo audit` against Cargo.lock | lockfile changes, and weekly |
| `claude.yml`, `claude-review.yml` | `@claude` on an issue or PR, and a first-pass review of every PR | on mention / on PR open |

Two of these are worth knowing the shape of before they surprise you.

**The fmt/clippy ratchet is not `-D warnings`.**
`scripts/check-rust-lint.sh` holds a high-water mark — the count of
unformatted files and of clippy warnings the repo has today. A change may not
push either number up; when you bring one down, lower the baseline in the
same commit. The script says so when it can be lowered. The backlog itself is
tracked as an issue, and the day it reaches zero both become ordinary
all-or-nothing gates. Run it locally with `scripts/check-rust-lint.sh`; it
needs nothing but `cargo`.

**The full Godot gate is opt-in per PR.** `verify-core-loop.sh` costs minutes
and a Godot download, so `verify.yml` runs it nightly rather than on every
push. Put the `full-verify` label on a pull request that touches the wire
format, `godot_project/`, `xtask` or `data/` and it runs there too. This does
not replace running it yourself before you push (§ above) — it is the
backstop for the session that did not.


## Commits

[Conventional Commits](https://www.conventionalcommits.org/), enforced by a
`commit-msg` hook locally and by `.github/workflows/commitlint.yml` in CI
(which re-checks every commit in the range, catching `--no-verify`). Once per
clone **and per worktree**: `git config core.hooksPath .githooks`.

Rules, in full in [CONTRIBUTING.md](../CONTRIBUTING.md), enforced by
`scripts/lint-commit-msg.sh`: `<type>(<optional scope>): <description>`; types
`feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert`; scope lowercase
`[a-z0-9.-]`; description imperative, lowercase start, no trailing period;
header ≤ 100 chars; `!` plus a `BREAKING CHANGE:` body paragraph for breaks.
`Merge …` headers are exempt.

### The DEVLOG decision

**Before finishing a commit, decide whether [DEVLOG.md](DEVLOG.md) needs a
note.** The `pre-commit` hook prints a reminder when a commit does not touch
it, but **never blocks** — whether an entry is warranted is a judgment call.
Most small commits do not need one (typos, wording, minor config).

- Add a brief, **high-level** entry for a decision, a milestone or meaningful
  progress: a few sentences on what changed and why it matters, not a bullet
  per commit. One entry per chunk of work (a session, a pass, a milestone),
  newest at the top of `## Log`, headed `### YYYY-MM-DD — summary`.
- **Update DEVLOG's "Current status" in place** whenever it goes stale — it is
  what someone picking the project up cold reads first. The log below it is
  history, only ever appended to.

### Recording a decision

A non-trivial design decision not covered by the three design docs goes in
**three places**: the **condensed rule** (newest first) in
[DECISIONS_LOG.md](DECISIONS_LOG.md); the **full reasoning** in
[DECISIONS.md](DECISIONS.md), Part A; and a **note in the doc it affects**, so
the rule is found where it is needed. UI_UX.md §2b point numbers and
ARCHITECTURE.md §9a are cited from code comments — keep them stable, and do
not re-litigate a logged decision without a good reason.

## GitHub: branches, pull requests and issues

**The repository on GitHub is the record of the work, not just a backup of
it.** Every change arrives on `main` through a pull request, and a finding
worth someone's time that this branch will not fix becomes an issue. Both
rules exist for the same reason: this project is built mostly by agents in
separate worktrees over sessions that do not share memory, so a finding that
lives only in a session's final message is lost, and a change that lands by a
local merge has no reviewable artifact and no CI run behind it. The issue
half of that has a cost the PR half does not — see *Fix it, or file it*
below, and prefer the fix.

`gh` is the interface — it is authenticated in this environment and needs no
token handling. If `gh auth status` fails, stop and say so rather than falling
back to pushing `main` by hand.

### Never commit to `main`

Work happens on a branch, always, however small the change:

```
git switch -c <type>/<short-kebab-description>    # feat/, fix/, docs/, chore/…
```

The branch prefix matches the Conventional Commit type of the work. `main` is
only ever updated by merging a pull request. **Do not `git push origin main`,
do not `git merge` a feature branch locally and push the result** — a
`pre-push` hook refuses the first and there is no reason to do the second.
A worktree whose `HEAD` is already `main` should branch before its first
commit, not after.

### Open a pull request, and finish it

```
git push -u origin HEAD
gh pr create --title "…" --body "…"   # --fill carries no screenshot; see below
```

`.github/pull_request_template.md` is the body's shape: what changed and why,
**what was run to verify it** (name the actual commands — `cargo test -p
sim_core`, `scripts/verify-core-loop.sh`, which `screenshot-scene.sh` flags
and whether the PNG was read), the DEVLOG/decision-log calls that were made,
and `Closes #N` for every issue the branch settles.

### Every pull request shows the change

**A PR body embeds at least one screenshot — every PR, not only the visual
ones.** This is a game: the thing under review is a frame, and a paragraph
describing a frame is a claim about it where the image is the evidence. The
rule is unconditional because the conditional version rots — "visual changes
only" becomes an author deciding their change is not visual, and the one check
that catches a stale content-id lookup (UI_UX.md §2b point 12) is the one
nobody ran. A sim change still has a surface: the panel whose numbers moved,
the line that now backs up, the demo line as it stands at this commit.

```
scripts/screenshot-scene.sh shot.png 1600x900 --board   # the flag that shows it
                                                        # then READ the PNG
scripts/pr-screenshot.sh shot.png                       # prints the markdown
```

`scripts/pr-screenshot.sh` exists because `gh` cannot attach a file to a PR
body and the rendered PNGs are gitignored. It pushes them to **`pr-media`**, an
orphan branch that shares no history with `main` and is never merged into it,
and prints a `github.com/<owner>/<repo>/blob/pr-media/…?raw=true` link. That
URL form is not incidental: this repository is private, so a
`raw.githubusercontent.com` link needs a token and renders as a broken image
for every reader, while the `github.com` one is fetched with the reader's own
session. The file name becomes the caption — name the PNG after its subject.

Beside each image, say what it shows and what to look at. The screenshot
replaces nothing in "How it was verified", where the findings still go in
words. Two layers hold the rule up, because a rule is only worth having if it
survives a session in a hurry: a `PreToolUse` hook
(`.claude/hooks/require-pr-screenshot.sh`) refuses a `gh pr create` whose body
has no image — which is why `--fill` alone no longer opens a PR here — and the
`PR screenshot` CI job (`scripts/check-pr-screenshot.sh`) fails a body with no
image, one with a `raw.githubusercontent.com` link, or one linking a file that
does not exist.

Then **get it reviewed, and land it** — an open PR is unfinished work until it
merges, and the one legitimate reason to leave it open is that it is waiting
on a reviewer's verdict (below):

```
gh pr checks <N> --watch      # commitlint must pass; a red PR is not done
gh pr view <N> --json reviews,comments --jq '[(.reviews[]|{t:.submittedAt,b:.body}),(.comments[]|{t:.createdAt,b:.body})]|sort_by(.t)|map(.b|split("\n")[0]|select(startswith("VERDICT:")))|last // "NO VERDICT"'
gh pr merge <N> --squash --delete-branch   # only on exactly VERDICT: APPROVED
```

The verdict is posted with `gh pr review <N> --comment`, which `gh` returns
under `reviews` rather than `comments` — hence the query reads both and takes
the latest `VERDICT:` line.

**Squash by default.** A branch's intermediate commits ("fix the test",
"wording") are noise in a history that feeds the changelog (CONTRIBUTING.md);
the squashed subject becomes the one Conventional Commit that describes the
change, so it must pass the same lint. Use a real merge commit only when the
individual commits are genuinely worth keeping — a ticketed epic where each
commit is a reviewed unit.

**Every PR is reviewed before it merges. No exceptions for routine changes,
and no reviewing after the merge** (the owner's rule, 2026-09-25). A PR
merges only once it has a `VERDICT: APPROVED` comment from a reviewer that
does not share the author's context (below), and its checks are green. An
instruction to "open a PR and merge it" means that: get the review, then
merge on its verdict. PR #163 is why this is spelled out. It merged on green CI under the
old wording ("merging your own work unreviewed is fine for routine
changes"), and its review could only happen afterwards, with the code
already on `main`. A deviation from the design docs, a `gdext` bump, a
save-format change, or anything the owner flagged also waits for the owner,
on top of the review.

**A PR is reviewed adversarially, by a reviewer that does not share the
author's context.** What makes a review independent is not which session runs
it but what the reviewer knows. It starts from the PR and the repository alone:
no transcript, none of the author's reasoning, no account of what was decided,
what was tried or why something is fine. A reviewer who has been told why the
code is right checks the author's argument instead of the code, and passes the
same blind spots.

The usual way to get one is for the author to **launch a fresh subagent** (the
Agent tool, `general-purpose`) whose whole prompt is the PR number and the
instruction to follow the `review` skill and post its verdict, e.g. *"Review
PR #N adversarially, following `.claude/skills/review/SKILL.md`, and post the
verdict with `gh pr review N --comment`."* Add nothing: no description of the
change, no list of what was verified, no hint at what to skip or what is known
to be acceptable. The PR body is already there, and the skill tells the
reviewer to distrust it. A separate session that did not write the branch is
just as good. What never counts is the author reviewing its own work, however
carefully it re-runs its tests, or a subagent briefed with the author's
reasoning. On `CHANGES REQUESTED`, fix the branch and send a fresh subagent
the same bare prompt for the re-review.

The reviewer builds the branch in its own worktree, re-runs the reproduction
of every issue the PR claims to close, and reverts each fix in turn to prove the new
test actually fails without it. GitHub refuses `--approve` and
`--request-changes` on a PR owned by the same account — which is every PR here
— so the verdict is posted with `gh pr review N --comment`, opening with a line
that is exactly `VERDICT: APPROVED` or `VERDICT: CHANGES REQUESTED`, and the
merge gates on that line (`reviewDecision` stays empty). The `review` skill is
the full procedure, for the first review and for a re-review.

### Fix it, or file it — and prefer fixing it

**Filing is the fallback, not the reflex.** A finding that only ever lived in
a session's final message dies with the session, which is why the
`resolve_sink` inlet-rule contradiction (`docs/DEVLOG.md`, the pipes &
conveyors close-out) sat in a DEVLOG paragraph for a week. But the opposite
failure is the one this project actually has: a backlog of small, true,
unprioritised findings that a human has to read and rank, and that nobody ever
picks up. **An issue nobody will ever choose to work on costs more than it
records.**

So the first question is not "is this worth an issue" but "why am I not
fixing it right now":

- **A one-line correction in a file you already have open is a fix**, not a
  backlog entry — a stale comment, a wrong constant in a doc header, a typo,
  a misleading log line. Make it in the branch at hand and mention it in the
  PR body. Two of the longest-lived issues in this repo were "a comment says
  thirty scripts, there are 38" and "a comment says 60 Hz, the tick is 30 Hz";
  both were cheaper to fix than to file — and the first was itself wrong, the
  true count being 37, which is why the script now prints what it ran.
- **A small, safe fix in the area you are already working in is a fix too**,
  even if it was not in the brief, as long as it does not enlarge the PR's
  story past what its title claims.
- **File it when fixing it now would be wrong**: it needs a call only the
  owner can make, it breaks the save format, it contradicts a design doc, it
  is genuinely more work than the branch at hand, or it is in a part of the
  codebase this branch has no business touching.

Then apply the test: **would a reasonable person pick this off the board
ahead of the next feature?** If the honest answer is no, it is not an issue.
Things that fail the test and should not be filed:

- polish ideas with no defect behind them ("it could look nicer at 4K"),
- speculative hardening — "add a test for X" where X has never broken,
- observations about systems deliberately not built yet (CLAUDE.md lists
  them; their absence is a plan, not a bug),
- restatements of a limit a design doc already records,
- anything you cannot say concretely goes wrong for a player or an agent.

**Budget: roughly one issue per pull request.** That is a smell test, not a
quota — a session that wants to file five has usually either found one
underlying problem worth a single issue, or is filing nits. If several real
deferrals genuinely come out of one branch, file the one that matters and put
the rest in the PR body, where review can promote them.

```
gh issue create --title "<imperative summary>" \
  --label deferred --label area:sim --body "$(cat <<'EOF'
…
EOF
)"
```

`.github/ISSUE_TEMPLATE/deferred-finding.md` is the body's shape, and the
`file-issue` skill is the full procedure. Search first (`gh issue list
--search`) — a second sighting belongs as a comment on the open issue, not as
a second issue. What makes an issue useful here is
that the next agent starts cold: name the **file and function**, say how it
was **found** (which test, which screenshot flag, which run), say what the
**fix probably is** if you know, and say **why it was deferred** — which of
the reasons above applies, in a sentence that survives being read cold.

Labels, kept deliberately few:

| Label | For |
|---|---|
| `deferred` | Spotted during other work and consciously not fixed. The one that matters most. |
| `owner-decision` | Blocked on a call only the owner can make — a gameplay change, a deviation from a design doc, anything that invalidates saves. |
| `area:sim` `area:godot` `area:assets` `area:docs` `area:tooling` | Where the work would happen. |

plus GitHub's stock `bug`, `enhancement` and `documentation` for kind.

**Close issues from the work that fixes them**, with `Closes #N` in the PR
body so the merge closes them automatically — never by hand afterwards, which
is a step that gets forgotten. If a finding turns out to be wrong or already
fixed, close it with a comment saying which, rather than deleting it.

### What still belongs in the repo, not in GitHub

Issues are the *backlog*; they are not the decision record and not the status
tracker. A decision still goes in the three places under *Recording a decision*
above, a milestone still goes in `docs/DEVLOG.md`, and a design plan is still a doc in `docs/`. An
issue is the right home only for work that is not being done yet.

### Closing out a session

A session ends when its worktree could be deleted without losing anything —
not when the last edit is made. Every branch has reached `main` through a
merged pull request, nothing is uncommitted, unpushed or left in the shared
stash, and no pull request this session opened is still open.

The `wrap-it-up` skill is the procedure, including the sweep that proves it
and the banner that reports it. Two rules it exists to enforce:

- **Being near the end of a session is not a reason to file an issue.** The
  gate above is unchanged at the close: finish it, drop it deliberately, or —
  rarely — file it because finishing it now would be wrong.
- **A branch left behind in a worktree about to be removed is deleted work.**
  If it genuinely cannot land, say so plainly and leave the worktree in place;
  do not report a clean close over the top of it.
- **A PR of your own waiting on its review verdict is the one legitimate way
  to end with a PR open.** You cannot review what you wrote, but you can
  launch a context-free reviewer subagent for it (above). If that is not
  possible, pushing it and naming what it waits on is finishing your half;
  merging it unreviewed to tidy up is not.

## Orchestration (Orca)

Use Orca's orchestration layer (`orca orchestration …`) for milestone-sized
work, not one long session.

- One Run per meaningful chunk of milestone work.
- Milestones map to Tasks about 1:1; encode ARCHITECTURE.md §9's dependency
  chain with `--deps`.
- **Every task spec must be self-contained**: name the milestone and doc
  section. A worker starts with zero conversational context.
- Foundational/large milestones get `--worktree new-top-level`. Small
  follow-on work can reuse `current` or a child worktree.
- Prefer `worker-start` over manual `dispatch --inject`.
- Use `gate-create` at milestone boundaries for a human check.
- After every accepted `worker_done`, decide whether it is DEVLOG-worthy.
- **A worker's branch lands the same way as anything else: a pull request.**
  A ticket's branch is pushed and `gh pr create`d rather than merged into
  the integration branch by hand, and the integration branch reaches `main`
  by one PR of its own. A worker that defers a finding files the issue
  itself (it has `gh`) rather than leaving it in `worker_done`, where only
  the orchestrator ever reads it.
- Full ownership handoffs use `orca-cli` / `orca worktree create --prompt`,
  not Task/Dispatch machinery.
