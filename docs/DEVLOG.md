# Sauce Factory — Development Log & Status Tracker

This doc answers "what's actually been done, and what's the state of things
right now" for whoever (human or agent) picks this project up cold. It's a
**log and status tracker, not a design doc** — the reasoning and specifics
live in [ARCHITECTURE.md](ARCHITECTURE.md), [UI_UX.md](UI_UX.md), and
[GAMEPLAY.md](GAMEPLAY.md); this file indexes *when* and *why* things changed,
and what's true *now*. It is deliberately **not** `@`-imported into
`CLAUDE.md` (unlike those three) — it will grow without bound over the life
of the project, so it's meant to be checked when you need history/status, not
force-loaded into every session's context.

---

## Current status

*(This section reflects "now" — edit it in place as things change. The Log
below is history and should only ever be appended to.)*

- **Stage:** Pre-alpha. Milestones 1 (Scaffold) and 3 (Core loop proof) are done — real simulation code now exists (one belt, one machine, one recipe), headlessly verified end to end.
- **Repo:** Private GitHub repo (`TomWhitticase/sauce-factory`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
- **Design docs:** ARCHITECTURE.md, UI_UX.md, and GAMEPLAY.md are all written and cross-referenced; docs/README.md indexes them.
- **Milestone progress:** milestones 1-5 (Scaffold through Economy MVP) are being built as an Orca orchestration Run (`run_51e5cb271445`), one Task per milestone, dependency-chained. **Milestones 1 (Scaffold) and 3 (Core loop proof) are complete** — see their log entries below. Milestone 2 (Mobile export spike, depends only on milestone 1) and milestone 4 (First full chain, depends on milestone 3) are next. Check `orca orchestration task-list --run run_51e5cb271445 --json` for live status. There are 11 milestones total, not 9 — a milestone 5 "Economy MVP" was added since the loop wasn't a game without it.
- **Open threads:** each doc's own "Open questions" section (ARCHITECTURE.md §11, UI_UX.md §9, GAMEPLAY.md §11) is the real TODO list at this stage — nothing is blocked, these are just undecided-and-known-undecided.

---

## How to use this doc

- **Newest entries at the top of the Log.**
- **Keep it high-level.** A few sentences per entry: what changed in general terms and why it matters for where the project is at, not a blow-by-blow of every file/decision — that detail already lives in the docs' own decisions/changes and in `git log`/commit messages. If you're tempted to write a bullet per commit, zoom out instead.
- One entry per meaningful chunk of work (a session, a design pass, a milestone), not one per commit.
- **Update "Current status" above whenever it goes stale.** That section is what anyone should read first; the log is for "how did we get here," briefly.
- **Not every commit needs an entry.** Small/trivial changes (typo fixes, a tweaked wording, a minor config change) don't warrant one — see CLAUDE.md's commit convention for the actual rule on when to add one.

---

## Log

### 2026-09-06 — Milestone 3: Core loop proof

First real simulation code: one belt, one machine, one recipe (tomato →
tomato sauce), rendered and interactable in Godot. `sim_core/` now has a
pure-Rust `sim/` module tree (belt/machine/recipe/inventory/world, no `gdext`
dependency) sitting under a thin JSON-based `SimBridge` translation layer —
`tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
`query_tile()`, per ARCHITECTURE.md §4. The belt implements real gap-based
item storage (§3.3, Factorio's FFF #176 trick), the machine runs the generic
tick-counted `TimedProcess` pattern (§3.5), and all shared simulation state
uses fixed-point (`fixed::I16F16`), never float, per §3.9. The recipe itself
lives in a new top-level `data/recipes.ron` — resolving §11's data-file-path
open question — not hardcoded in Rust.

Verified in three layers, cheapest first (`scripts/verify-core-loop.sh`
automates all three): a headless `cargo test` integration test ticks the
belt+machine for thousands of ticks and asserts strict item conservation
(no duplication/loss) every single tick; a headless Godot script drives the
real `gdext` JSON command/snapshot boundary end to end (spawn 4 tomatoes →
1 tomato sauce → collect); and a second headless Godot script simulates an
actual keypress through the interactive scene's input handler to confirm
the "interactable" requirement is real, not assumed. The interactive scene
(`main.tscn`) renders via a genuine `TileMap` + moving item sprites, driven
only by `SimBridge`'s JSON snapshots — it never touches the belt's internal
gap array. Visuals are flat placeholder textures, explicitly not a UI_UX.md
style decision (see `godot_project/placeholder_art/README.md`); the real
voxel asset pipeline is still unbuilt. See CLAUDE.md's decisions log for two
smaller implementation calls worth knowing about: the belt's gap
representation and why the `gdext` boundary is JSON strings instead of
`Dictionary`/`Array` values.

### 2026-09-06 — Milestone 1: Scaffold

First real code. Set up the Rust workspace (`sim_core/` cdylib crate + an
`xtask` build-script crate) and a Godot 4 project (`godot_project/`), wired
together via `gdext` — a trivial `SimBridge.ping()` callable from GDScript
proves the Rust↔Godot round trip end to end. `cargo xtask` builds and copies
the compiled extension into place per ARCHITECTURE.md §7; `sim_core.gdextension`
is checked in. Pinned the exact versions ARCHITECTURE.md §1a had left as
placeholders (Godot 4.6.3-stable, `gdext` 0.5.5, Rust edition 2024, `hecs`
0.11.1) — see CLAUDE.md's decisions log for the reasoning.

Verified entirely headlessly, no editor GUI: `scripts/verify-scaffold.sh`
builds, primes Godot's local import cache (working around a known, benign
Godot 4.6.3 editor-shutdown crash — see the script's own comments), then runs
`godot --headless --script test_round_trip.gd`, which calls `SimBridge.ping()`
and asserts the known return value, exiting non-zero on failure. This is now
the project's headless "does the interop boundary still work" check.

### 2026-09-06 — Milestones 1-5 dispatched via Orca orchestration

Adopted Orca orchestration (see CLAUDE.md's Orchestration section) as how
milestone work actually gets built, then used it for real: created a Run and
a 5-task DAG covering milestones 1-5 (Scaffold, Mobile export spike, Core
loop proof, First full chain, Economy MVP), dependency-chained to match
ARCHITECTURE.md §9. Milestone 1 (above) was the first task dispatched and is
now complete; 2-5 are queued behind their dependencies and will be dispatched
as each prior milestone completes. This is the actual start of writing game
code — everything before this was design docs only.

### 2026-09-06 — Repo set up, full design-doc pass complete

Set up the repo (private, Conventional Commits enforced via hook + CI) and
wrote all three core design docs: **ARCHITECTURE.md** (Rust sim core +
Godot/`gdext` presentation, chosen to be agent-friendly, then hardened via a
critical-review pass that turned several open options into firm decisions),
**UI_UX.md** (isometric voxel art at a 32×32×32 tile scale, Minecraft-adjacent
UI, plus a researched asset pipeline so voxel models can be authored and
reviewed by an agent), and **GAMEPLAY.md** (tiered sauce/equipment
progression, a contracts-based economy designed to force a diversified
factory, staff as an abstract resource pool). Still no game code — the docs
are the whole project so far. Next up: milestone 1 (scaffold) from
ARCHITECTURE.md. For specifics, see the docs themselves, CLAUDE.md's
decisions log, or `git log`.

### 2026-09-06 — Cross-doc consistency pass

Read back through all three design docs together looking for gaps and
contradictions now that they've each been written somewhat independently.
Found and fixed a real one: the milestone list never actually scheduled the
economy/contracts system, even though the gameplay doc defines the core loop
as needing it — added an "Economy MVP" milestone. Also gave the architecture
doc an "Open questions" section (its siblings already had one) and flagged a
couple of things that don't have a designed home yet (money/contract state,
mobile touch input). Tightened up repo config (`.editorconfig`, a fuller
`.gitignore`) while at it. Nothing structural changed — still docs-only,
still pre-milestone-1.

