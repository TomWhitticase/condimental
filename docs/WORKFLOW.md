# Sauce Factory — Workflow

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
preview PNGs + committed icons), or `cargo run -p voxgen [--bin voxicons]`;
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
| `cargo test -p sim_core` | Sim invariants: conservation, determinism, economy, save/load. 19 integration test files under `sim_core/tests/`, plus unit tests. | fast |
| `cargo test -p voxgen` | Voxel-level asset properties — that a machine's inlet/outlet is a genuinely empty opening, which no mesh or screenshot can tell from a painted rectangle. | fast |
| `scripts/verify-core-loop.sh` | Everything: `cargo test -p sim_core`, `cargo test -p voxgen`, `cargo xtask`, two `--import` passes, an import sanity check, then **30 headless Godot scripts** through the real `gdext` JSON boundary. | slow |
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
floor. The script's header explains every flag; grouped, they are:

- **Fixtures:** `--build`, `--fixture <machines|fluids|vat|splitter|chain|
  stress|pipes|gallery|demo>`, `--stress`, `--pipe-gallery`, `--belt-gallery`,
  `--bottleneck`, `--understaffed`, `--staff-shortfall`, `--contracts`,
  `--payout`; `--populate` runs one forward; also `--stock`, `--hotbar N`
- **Screens:** `--menu`, `--pause`, `--shop`, `--market`, `--board`,
  `--staff`, `--production`, `--no-hud`
- **Build mode:** `--ghost KIND`, `--ghost-blocked KIND`, `--remove-mode`
- **Tooltips:** `--hover`, and `--hover-` + `pipe|vat|hopper|tank|dock|splitter`
- **Camera / panels:** `--first-person`, `--closeup TX,TY[,SIZE]`,
  `--config TX,TY` (+ `--contents N`). A value starting with `-` needs the
  `=` form: `--config=-1,0`.

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
- Full ownership handoffs use `orca-cli` / `orca worktree create --prompt`,
  not Task/Dispatch machinery.
