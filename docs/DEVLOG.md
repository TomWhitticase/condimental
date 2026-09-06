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

- **Stage:** Pre-alpha. Milestones 1 (Scaffold), 2 (Mobile export spike), 3 (Core loop proof), 4 (First full chain), and 5 (Economy MVP) are done — a real multi-stage production chain now exists (tomato -> tomato sauce -> ketchup -> bottled ketchup -> boxed ketchup), headlessly verified end to end and driven entirely by `data/recipes.ron`, Android export is proven working, and the chain can now actually deliver finished product against a hardcoded contract to earn money.
- **Repo:** Private GitHub repo (`TomWhitticase/sauce-factory`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
- **Design docs:** ARCHITECTURE.md, UI_UX.md, and GAMEPLAY.md are all written and cross-referenced; docs/README.md indexes them.
- **Milestone progress:** milestones 1-5 (Scaffold through Economy MVP) are being built as an Orca orchestration Run (`run_51e5cb271445`), one Task per milestone, dependency-chained. **Milestones 1 through 5 are all complete and merged** — the core MVP loop (build factory → deliver → get paid) is real end to end. A parallel, non-milestone task — **visual polish** (voxel assets, isometric 3D camera, real-time lighting per UI_UX.md) — is still in progress against the same main; expect a merge conflict against milestone 5's Godot-side HUD changes when it lands, to be reconciled by the coordinator. Check `orca orchestration task-list --run run_51e5cb271445 --json` for live status. There are 11 milestones total, not 9 — a milestone 5 "Economy MVP" was added since the loop wasn't a game without it. Milestone 6 (Fluids) is next once visual polish lands.
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

### 2026-09-06 — Milestone 5: Economy MVP

The loop stops being a production-chain tech demo and starts being a game:
`sim_core::sim::World` now owns a `money` counter (fixed-point, never a
float) and a single hardcoded contract — a fixed sauce mix (2x
`boxed_ketchup`), a fixed delivery cadence (every 1,000 ticks), and a fixed
payout — that the milestone-4 chain can actually deliver against. Collecting
finished product (`CollectOutput`) now deposits it into an accumulating
delivery buffer instead of discarding it; on each cadence boundary the
buffer is checked against the contract's mix, and a satisfied mix pays out
and consumes the delivered stock, while a miss just skips that window
(no contract-loss penalty yet — GAMEPLAY.md §11's still-open question).
Money and contract status are exposed to Godot as a JSON snapshot and
rendered as a minimal HUD label (money + per-item progress + ticks to next
check) — deliberately plain, not UI_UX.md's clipboard/checklist styling,
which is later polish. Verified headlessly two ways: a `cargo test`
integration test drives the whole chain end-to-end (spawn -> travel every
stage -> collect -> get paid) and asserts the exact payout lands only once
the mix is satisfied, plus `Economy`'s own unit tests cover cadence-boundary
edge cases; and the existing gdext-boundary Godot test now also round-trips
the new JSON snapshot. This resolves the money/contract architectural-home
question flagged as open since the cross-doc consistency pass (see
CLAUDE.md's decisions log and GAMEPLAY.md §9 for the actual shape used).
Explicitly deferred, per this milestone's own scope: `contracts.ron` data
loading, multiple concurrent contracts, and `StaffPools` — all still future
work.

### 2026-09-06 — Milestone 4: First full chain

Extended milestone 3's single belt+machine into a real multi-stage
production chain: raw tomato -> tomato sauce -> ketchup -> bottled ketchup ->
boxed ketchup, four sequential belt+machine stages, fully data-driven from
`data/recipes.ron`. The core design call: `sim_core::sim::World` now builds
one stage per recipe *in the data file's own list order*, chaining each
stage's machine output into the next stage's belt automatically — no
Rust code names which recipe follows which. This was proven, not just
claimed: after building and testing a working 3-stage chain, a 4th recipe
(`boxed_ketchup`) was appended to `data/recipes.ron` alone (verified via a
SHA-256 hash check that no `.rs`/`.gd` file changed), and the full headless
test suite — the Rust conservation test plus the real `gdext` JSON boundary
through Godot — passed against the extended 4-stage chain with zero code
changes. `sim_core/tests/core_loop.rs` now walks the whole chain generically
(via a recursive "how many raw tomatoes is this item worth" helper) rather
than asserting fixed counts, so it keeps working as the chain grows. The
Godot side (`main.gd`) reads a new `chain_layout_json` call once at startup
to build its `TileMap` sized to however many stages currently exist, instead
of hardcoding stage geometry — keeping intact milestone 3's "Godot draws
what the sim says exists" principle. See CLAUDE.md's decisions log for the
`ChunkSnapshot` shape change (`belt`/`machine` singular -> `belts`/`machines`
plural, since a chunk can now hold more than one stage) and the deliberate
scope limit (every recipe past the first has exactly one input/output, so
this is one straight line, not yet a branching factory).

### 2026-09-06 — Milestone 2: Mobile export spike

Deliberately run early (ARCHITECTURE.md §9) so mobile-export risk surfaces
now rather than at platform polish. Full details in ARCHITECTURE.md §9a;
headline result: **Android works, iOS doesn't in this environment.**

Android: stood up a standalone Android SDK/NDK (no Android Studio needed),
cross-compiled the milestone-1 `sim_core` crate for `arm64-v8a`/`x86_64` via
`cargo-ndk` with zero source changes, and got Godot's Gradle-based export to
produce a validly-signed APK with the extension actually embedded (confirmed
via `apksigner verify` and `unzip -l`). One real Godot gap found along the
way: its Android export CLI can fail with a **blank** error message if
`rendering/textures/vram_compression/import_etc2_astc` isn't set in
`project.godot` — tracked down by reading Godot's own export-plugin source
rather than guessing, now fixed in this project's settings. Installing and
actually launching the APK on a device/emulator was skipped this round — the
dev host was down to double-digit MB of free memory with sibling sessions
running concurrently, and booting an emulator on top of that felt like the
wrong risk to take. That's the one remaining unverified step before Android
export counts as fully proven, not just built.

iOS: could not be attempted for real. This dev environment has no Xcode.app,
only Command Line Tools — confirmed both via `xcodebuild`/`xcrun` failing to
find an iOS SDK at all, and via a Rust `aarch64-apple-ios` build that
compiled without error but linked against the wrong (macOS) SDK sysroot, so
even that partial result wasn't treated as a pass. Per this milestone's own
instruction to decide explicitly if iOS looks broken: **it stays a
stretch/deferred platform, not a milestone-10 launch commitment**, until
someone re-runs this spike on a Mac with real Xcode or Apple-hosted CI —
that's recorded as a genuinely open question (ARCHITECTURE.md §11), not a
settled "iOS is broken."

Also surfaced, unrelated to mobile export itself: late in this session the
shared dev host (many concurrent users, physical memory nearly exhausted)
started failing to load the project's own unmodified `sim_core` extension at
all — even the plain milestone-1 round-trip check — with a Rust
undefined-behavior-check panic inside `gdext`. Checksum-verified as the exact
same binary that worked fine earlier in the session, and reproduced even in
a clean environment, so this reads as host resource exhaustion tripping an
optional safety check, not a project regression — but it means
`scripts/verify-scaffold.sh` should be re-run on a calmer host before trusting
its result if it ever fails unexpectedly like this again.

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

