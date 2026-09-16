# Sauce Factory — Architecture Plan

A Factorio-like automation game, sauce-themed. Targets: PC, Mac, mobile.

This doc covers **architecture and technology only** — how the game is
built. Gameplay is [GAMEPLAY.md](GAMEPLAY.md); visuals are
[UI_UX.md](UI_UX.md). Full decision records are in
[DECISIONS.md](DECISIONS.md).

The plan follows Factorio's own patterns (deterministic tick simulation,
data-oriented ECS, aggregate item representation, tiered scheduling) and
Godot's text-native project structure. After a critical review, every hedged
"X or Y" was turned into a firm decision: an agent hitting an "or" will pick
something, and it should pick what the team chose.

---

## 0. Background & rationale (why this stack)

The goal is a codebase an AI coding agent can build reliably. That constraint
drove these choices.

**Evidence on agent-friendly development:**

- Training-data density beats paradigm elegance. Prefer mainstream,
  heavily-documented tech.
- Static typing's main value for agents is the compiler-error feedback loop.
- Functional languages do not help LLM generation today (training corpora are
  imperative).
- Exotic languages raise cost and lower the quality ceiling; mainstream
  compiled languages reach the highest quality.
- Visual/node-based tooling (Blueprints, binary scenes) is a hard capability
  boundary: agents operate on text diffs.

**Godot over Unity/Unreal:** `.tscn`/`.tres` are plain text, so the whole
project is agent-diffable end to end. Unity keeps a binary asset database
underneath; Unreal is Blueprint-centric.

**Not Bevy:** pre-1.0, breaking API changes roughly every 3 months, so
training data is stale and agents hallucinate old signatures. Mobile export
is thinner than Godot's.

**Rust for the simulation core** (via GDExtension, not as the whole engine):
the borrow checker gives a sharp agentic feedback loop (LLM repair of
borrow-checker errors succeeds ~74% of the time).

**Accepted risk: `gdext` is pre-1.0 and niche**, with the same stale-API risk
as Bevy. Mitigation: pin exact versions (§1a), treat each `gdext` bump as a
reviewed decision, and keep the `gdext` API surface this codebase touches
small (§4).

**Accepted risk: mobile export via GDExtension is less proven than plain
GDScript.** iOS export of GDExtension projects is flagged experimental;
Android needs real NDK cross-compilation. See §9 milestone 2 and §9a.

---

## 1. Tech stack

| Layer | Tech | Role |
|---|---|---|
| Simulation core | **Rust**, `hecs` (ECS) + `rayon` | Deterministic logic: belts, machines, fluids, timed processes, power. No Godot dependency. |
| Interop | **GDExtension** via `gdext` | Thin binding layer. |
| Presentation | **Godot 4.x**, GDScript | Rendering, UI, input, camera, build mode, interpolation. |
| Data | **RON** files | Items, recipes, machines, shop, contracts — all content is data. |
| Tests | Rust headless tests + GUT | Sim invariants, determinism, interop boundary; UI smoke tests. See §6. |

**Hard rule:** anything deterministic or fast lives in Rust. Anything about
look/feel lives in Godot. The boundary is `SimBridge` (§4).

**`hecs` over `bevy_ecs`:** `bevy_ecs` inherits Bevy's churn. `hecs` is
small and stable; it has no scheduler, which is fine because §3.8 calls for a
custom network-partitioned scheduler with `rayon` underneath.

### 1a. Pinned versions

Bump only as a deliberate, recorded decision (add a line to CLAUDE.md's
decisions log), never incidentally.

| Dependency | Pinned to |
|---|---|
| Godot | 4.6.3-stable |
| `gdext` (crate `godot`) | 0.5.5 (default API level targets Godot 4.6; the `api-4-7` feature is NOT enabled) |
| Rust | edition 2024, MSRV 1.98.1 |
| `hecs` | 0.11.1 (last published 2026-07-28, actively maintained) |
| Godot `.vox` importer | [qbradq/Godot-MagicaVoxel-Importer-with-Extensions](https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions) v1.2, commit `288f1d8`, vendored under `godot_project/addons/` |
| `vengi` (dev tool only) | 0.5.0 |

---

## 2. High-level architecture

```mermaid
flowchart TB
    subgraph Presentation["Presentation — Godot / GDScript"]
        UI[UI: build menu, hotbar, clipboard]
        Render[3D scene + interpolation]
        Input[Input / camera / build mode]
    end
    subgraph Interop["Interop — GDExtension (gdext)"]
        Bridge[SimBridge: tick, dirty_chunks, chunk_snapshot,\nissue_command, query_tile, save_ron/load_ron]
    end
    subgraph Sim["Simulation core — pure Rust, headless, deterministic"]
        World[Chunked tile world]
        ECS[ECS: belts, machines, fluids, timed processes]
        Scheduler[Tiered scheduler (future)]
    end
    Data[(data/*.ron)]
    UI --> Bridge
    Input --> Bridge
    Bridge --> Render
    Bridge <--> Sim
    Data --> Sim
    Data --> UI
```

Data files are the single source of truth for both the simulation and the
UI. No duplicated definitions.

---

## 3. Simulation core (Rust)

### 3.1 World representation

- Tile grid in **32×32 chunks** (Factorio's size).
- Only chunks near a player or with something mid-process are "active"
  (future, §3.7).

### 3.2 Core components (ECS)

| Component | Purpose |
|---|---|
| `Position` | Tile coordinate |
| `BeltSegment` | Belt tile with **gap-based item storage** (§3.3) |
| `FluidNetwork` | Connected pipes carrying a fluid as volume/pressure, not particles |
| `Machine` | Recipe, progress timer, input/output buffers |
| `TimedProcess` | Generic tick-counted long process (§3.5) |
| `PowerNode` | Producer/consumer in a `PowerNetwork` (future) |
| `Inventory` | Generic slot-based storage |

### 3.3 Solid items on belts

Factorio FFF #176: a belt stores **gaps between items**, not positions, and
tracks the last non-zero gap. A jammed line changes one value per tick.
Implementation detail: gaps are slack beyond a fixed minimum item spacing
(see `belt.rs`'s module doc).

### 3.4 Liquids / fluid networks

A **connected-network pressure system**: each network has a volume and a
fluid type; flow equalizes between adjacent segments each tick. Never
simulate droplets.

### 3.5 Long-running / timed processes

A **tick-counted state machine** `TimedProcess { start_tick, required_ticks }`
attached to a container. Never wall-clock time: wall-clock reads desync
lockstep multiplayer (§8) and break pause/fast-forward. Progress is a
function of `current_tick - start_tick`. Gameplay meaning layered on top is a
design concern.

### 3.6 Recipes & content — fully data-driven

```ron
Recipe(
    id: "example_recipe",
    inputs: [("ingredient_a", 4), ("ingredient_b", 1)],
    outputs: [("output_item", 2)],
    machine: "processing_machine",
    time_ticks: 900,
)
```

Items, recipes and machines are added by editing data files, never
simulation code. `data/recipes.ron` also carries `fluid_inputs` and
`fluid_outputs`. If a new item needs a Rust change, a data field is missing.

### 3.7 Update scheduling (tiered) — future, milestone 8

- **Active:** belts/machines processing — every tick.
- **Warm:** things in transit — every N ticks, interpolated in rendering.
- **Sleeping:** blocked or idle — skipped until an adjacent change wakes it.

### 3.8 Threading

**Status (2026-09-08):** the "compute connected components once per topology
change, not every tick" half is real for fluid networks and belt/machine
links. `World::recompute_topology` rebuilds the occupancy grid, every
hand-off link and every pipe network on `Place`/`Remove`/`Rotate`, never
from `tick`. It is a full single-threaded rebuild, not an incremental
union-find: at this scale that is cheaper and cannot drift out of sync. It
flips around milestone 8, in that one function. `rayon`, per-network threads
and boundary entities are unbuilt.

Design when it lands: partition by **connected network**, not entity type,
with `rayon` running independent partitions. Read-only belt passes run in
parallel freely. A `PowerNetwork`/`FluidNetwork` is updated entirely by one
thread. **Boundary entities** (a member of two networks) update in a
single-threaded pass after all parallel passes complete.

### 3.9 Determinism

**Fixed-point, not floats, for all shared, persisted or replayable
simulation state** (positions, counts, volumes, power, progress). The `Fx`
type is `fixed`'s `I16F16`; **money is `Money`, `fixed`'s `I48F16`** (the
quality pass, 2026-09-13: the same 16 fractional bits, 48 integer bits —
`I16F16`'s ceiling of about 32 767 was reachable in minutes by a five-line
board, and at the ceiling a payout panics in a debug build and wraps to a
negative balance in release; a payout is a saturating add on top). Bit-identical floats across x86_64 and ARM64 is
an unsolved problem; fixed-point is one integer type threaded from the
start. Floats are fine for Godot-side presentation values that never feed
back into the sim.

Also: fixed timestep (**30 ticks/s**), deterministic iteration order (stable
ids, no hashmap iteration in hot paths). Tested per §6, not asserted.

The rate is set in exactly one place, `godot_project/main.gd`'s `TICK_HZ`;
`sim_core` itself only counts `tick()` calls and assumes no rate. This
section read "60 ticks/s" until 2026-09-16 — that was never the rate the
game ran (`TICK_HZ` has been 30 since milestone 3), so every seconds-figure
derived from it was twice the real elapsed time. Player-facing clocks use
the rate the game actually runs (UI_UX.md §5e).

---

## 4. Interop layer (`gdext`)

Keep the boundary narrow. Current and planned API:

```rust
impl SimBridge {
    pub fn tick(&mut self);
    // Changed-since-last-call, per CHUNK, so Godot re-reads only what changed.
    pub fn dirty_chunks(&self) -> &[ChunkId];
    pub fn chunk_snapshot(&self, id: ChunkId) -> ChunkSnapshot;
    pub fn issue_command(&mut self, cmd: PlayerCommand);
    pub fn query_tile(&self, pos: TileCoord) -> Option<TileSnapshot>;
    // The quality pass (2026-09-13): the tick path's read, and the poll gate.
    pub fn chunk_delta(&self, id: ChunkId) -> ChunkSnapshot; // what changed in the chunk
    pub fn revision(&self) -> u64;                           // ticks + commands so far
    // Save/load (built 2026-09-09). RON text both ways; file I/O is Godot's.
    // load_ron returns "" on success or a human-readable reason, leaving
    // the live world untouched on failure.
    pub fn save_ron(&self) -> GString;
    pub fn load_ron(&mut self, ron: GString) -> GString;
    pub fn new_game(&mut self);
    // Planned, not built:
    pub fn preview_command(&self, cmd: &PlayerCommand) -> PreviewResult; // validation pass, never mutate-then-rollback
    pub fn undo(&mut self) -> bool;
    pub fn redo(&mut self) -> bool;
}
```

Also on the wire today (`economy_snapshot_json` carries the whole contract
`board` beside the active `contracts` since the contract-board epic, and
`sign_contract` is a command): `world_layout_json`, `economy_snapshot_json`,
`shop_snapshot_json`, `ingredients_snapshot_json`, `fluid_snapshot_json`,
`dock_snapshot_json`, `machine_load_snapshots_json` (the machine-load epic,
2026-09-11: every recipe-running machine's rolling load window, `status`
and `stall` reason, polled because it changes every tick and dirties
nothing), `staff_snapshot_json` (the staff pools MVP, 2026-09-12: per
category headcount, the last tick's `demand`/`assigned`/`short`, the payday
clock and bill, polled for the same reason; `hire`/`fire` are commands,
`stall` gained a `staff` kind, machine and dock tile snapshots and shop
entries carry `crew`, the dock snapshot `understaffed`; the quality pass,
2026-09-13, added `next_bill`, `shortfall`, `unpaid_last_payday` and per
category `owed`/`unpaid`/`left_last_payday`/`can_hire`, so a panel can warn
before a payday and announce after it), `production_snapshot_json`
(the production-stats epic, 2026-09-13: per item and fluid, what recipes
MADE and USED over the sim's rolling windows and for all time, plus what
a paying contract took from the pool — the P screen's wire, polled for
the same reason and saved with the world), and JSON forms of
the above. The dock-capacity change (2026-09-16) added
`delivery_capacity` and `delivery_full` to `economy_snapshot_json` and a
`full` flag to every dock tile snapshot: the shared delivery pool now has
a per-item ceiling (GAMEPLAY.md §9), and the presentation layer draws its
bar and its full state from the simulation's own number rather than
inventing a full mark of its own (UI_UX.md §5e). Godot never sees internal
representations, only snapshots. Since the pipes & conveyors overhaul
(ticket 1, 2026-09-09) every belt, splitter and pipe entry in
`world_layout_json` — and its `query_tile_json` variant — carries a `links`
object (`docs/PIPES_AND_CONVEYORS_PLAN.md`, "The wire contract") recorded
by the topology pass itself, so Godot derives belt and pipe shapes from it
rather than from an adjacency scan of its own (§5).

**Wire format is JSON strings via `serde_json`**, not `Dictionary`/`Array`,
to minimise the `gdext`-specific API surface (the least agent-friendly layer).

**Delta granularity: per chunk, not per entity.** A per-entity delta copied
across FFI every tick shows up as frame spikes at milestone 8's scale.
`dirty_chunks()` returns a reused buffer. Consequence: fluid movement and
tank auto-fill deliberately dirty nothing; those readouts are polled.
**What a dirty chunk's read carries is a delta (the quality pass,
2026-09-13):** `chunk_delta_json(chunk)` is the chunk snapshot cut down to
the belts/splitters whose contents changed and the bodies whose state did
since the dirty set was last cleared, and the whole snapshot after a
place/remove/rotate/load touched the chunk. The call is still one per
chunk; `chunk_snapshot_json` stays the full read (a rebuild reads it for
every occupied chunk). `revision()` counts ticks plus commands so the
presentation layer polls its snapshots only on frames where something
could have moved.

**Undo/redo (planned):** a bounded Rust-side stack of
`(command, inverse_command)` pairs, not a Godot-side snapshot diff.

**Save format (amended 2026-09-09):** a save is a **full state snapshot**
with a `schema_version`, in RON, rejected (not migrated) on a version
mismatch. The original plan was a command log plus snapshots, shared with
undo/redo and multiplayer replay; neither exists, and a command log is
replayed through the *current* game logic, so any behaviour change would
silently rewrite every save. Fixed-point values are stored as raw bits.
Loading rebuilds structure by replaying real `Purchase`/`Place` commands
(a load cannot build what a player could not), then writes live state
through restore-only non-public methods. **File I/O is not in `sim_core`**;
`godot_project/game_session.gd` owns `user://saves/…`, slot names and the
autosave. A command log can still sit on top of this later.

**Data-file schema versioning:** every `data/*.ron` carries `schema_version`;
loaders reject a mismatch. Content changes never bump it.

**Interop training-data gap:** `SimBridge` appears in no training corpus.
Keep worked, runnable examples of every call pattern in-repo (the headless
`test_*.gd` scripts serve this role) and review changes here with more
scrutiny than one-sided changes.

---

## 5. Presentation layer (Godot)

- 3D isometric scene rebuilt from `world_layout_json` on every
  place/remove/rotate, never per tick (a full rebuild, mirroring §3.8).
- Interpolate "warm" entities between snapshots (future).
- UI is generated from the same data files (`shop.ron` drives the build
  menu and hotbar).
- Keep scenes as text `.tscn`/`.tres`. Never binary. No visual scripting.

---

## 6. Testing strategy

- **Headless Rust integration tests** (`sim_core/tests/`): run N ticks
  against known layouts built by `sim::reference_chain` and assert
  invariants — no item duplication/loss, fluid conservation, timed-process
  correctness. This is the gameplay-logic compiler loop.
- **Scripted playtests:** build a factory headlessly, assert throughput and
  economics.
- **Determinism/replay tests:** the save/load lockstep test compares every
  snapshot every tick for 2600 ticks. Cross-platform hash comparison (x86_64
  vs ARM64 CI) is still to do.
- **Interop boundary tests:** the headless `godot_project/test_*.gd`
  scripts exercise the real `gdext` JSON boundary. Each prints its own
  success marker because Godot exits 0 on a script parse failure.
- **GUT tests** for UI logic only.
- **`cargo test -p voxgen`** asserts voxel-level properties of assets
  (openings genuinely empty).
- **`scripts/screenshot-scene.sh`** is the one non-headless check; it finds
  stale presentation lookups no structural test can (UI_UX.md §2b point 12).

---

## 7. Development loop

The primary loop is **headless**: `cargo check`/`cargo test` on the sim
core, no Godot process. GDExtension editor hot-reload is flaky; open the
editor only for milestone-level visual checks. `cargo xtask` builds
`sim_core`, copies the library to where `sim_core.gdextension` expects it,
and stages voxel models. `scripts/verify-core-loop.sh` runs everything.

---

## 8. Multiplayer (deferred, designed for)

Deterministic lockstep (send commands, not state), possible because §3.5–3.9
are deterministic. Needs a command log on top of §4's snapshot save.

---

## 9. Milestones

1. **Scaffold** — ✅ Godot + `gdext` round trip, build script, `.gdextension`.
2. **Mobile export spike** — ✅ Android builds; iOS blocked, see §9a.
3. **Core loop proof** — ✅ one belt, one machine, one recipe.
4. **First full chain** — ✅ multi-step chain from data.
5. **Economy MVP** — ✅ money + a contract to deliver against. (Added during
   a consistency pass; GAMEPLAY.md §1's loop needs it.)
6. **Fluids** — ✅ pipe network, a recipe needing a liquid.
7. **Timed processes** — ✅ proven by a 2400-tick `aging_rack` on the
   existing `TimedProcess` with no sim change. The rack has since been
   deleted (ageing is not a mechanic this game wants), so nothing shipped
   exercises the long timescale; `main.gd`'s `MACHINE_PROGRESS_GAUGES` is
   kept as an empty table for the next long process.
8. **Scale-up** — power, tiered scheduler, chunk activity; stress-test to
   thousands of entities. Load-tests §4's per-chunk deltas and §3.8.
9. **Logistics expansion** — transport between factory sites.
10. **Platform polish** — mobile touch controls/export, packaging, UI pass.
11. *(Stretch)* **Multiplayer**.

Epics outside this list (all done, see DEVLOG): machines improvement
(footprints), visual overhaul rounds, shop & inventory, real placement,
contracts as data, delivery dock, splitters, chain rework, hotbar, save/load,
auto-fill.

---

## 9a. Mobile export spike — summary (milestone 2, 2026-09-06)

Full findings: [DECISIONS.md](DECISIONS.md) Part C.

**Android works.** `cargo ndk -t arm64-v8a -t x86_64 build -p sim_core`
cross-compiled `sim_core` unmodified; the Gradle-based export produced a
validly-signed APK with both `.so`s embedded. One-time local setup: Android
SDK/NDK paths in Editor Settings, the Gradle build template extracted by
hand into `godot_project/android/build/`, a debug keystore (machine-local).
Two traps: `rendering/textures/vram_compression/import_etc2_astc=true` is
required in `project.godot` or export fails with a **blank** error (Godot
4.6.3 bug, confirmed in its source); and `--install-android-build-template`
with unresolved preset errors opens a native blocking dialog even under
`--headless`. **Unverified:** install/launch on a device (skipped for host
memory). Close before milestone 10.

**iOS blocked, not proven broken.** The dev host has only Command Line
Tools, no Xcode.app; `xcodebuild`/`xcrun` fail and a Rust
`aarch64-apple-ios` build links against the macOS SDK (a "successful" build
that is not a valid iOS artifact). **iOS is stretch/deferred** until the
spike is re-run on a Mac with full Xcode or Apple-hosted CI.

**Host instability note:** under heavy load the shared host began failing to
load a byte-identical `libsim_core.dylib` with a `gdext` UB-check panic. Re-run
`scripts/verify-scaffold.sh` on a quiet machine before treating that failure
as a regression.

---

## 10. AI-agent development conventions for this repo

- Cover every sim-core change with headless tests before merging.
- Content changes never require Rust changes; if they do, add a data field.
- Keep `.tscn`/`.tres` as text.
- One `CLAUDE.md` per major module documenting non-obvious invariants.
  Built 2026-09-16 for `sim_core/`, `godot_project/` and `assets/voxels/`;
  each is loaded only when work happens in that directory. `AGENTS.md` is a
  symlink to `CLAUDE.md` beside each one, so the two cannot drift.
- **An always-loaded file is a tax on every request, not a one-off cost**
  (2026-09-16). The root `CLAUDE.md` holds only what would be wrong for an
  agent not to know on a turn that has nothing to do with it; everything
  else — this doc, UI_UX.md, GAMEPLAY.md, the decisions log, the workflow —
  is a link it follows on demand. An `@`-include is inlining, so there are
  none. Repeatable procedures are skills under `.claude/skills/`.
- No `f32`/`f64` in shared simulation state (§3.9).
- Treat any `gdext` bump as a reviewed decision.
- The primary loop is headless `cargo test` (§7).
- Review the rendered scene for presentation changes (§6).

---

## 11. Open questions

Resolved: data-file path (`data/`, top-level); `hecs` maintenance; `gdext`
version; `.vox` importer; money/contract home (GAMEPLAY.md §9); per-tile
orientation (real placement epic — `Direction` on every placement, one Y
rotation in Godot; the belt's east-only junction art was retired by the
pipes & conveyors overhaul, UI_UX.md §2b point 18, and its ticket 6 retired
`pipe_source`/`pipe_inlet`, the last east-only assets, so **no asset in the
project is authored for one facing any more** — every tile derives its shape
and turn from its `links`).

Still open:

- ~~`StaffPools` (GAMEPLAY.md §7) has no ECS shape.~~ Resolved 2026-09-12
  (the staff pools MVP): it is not an ECS shape at all — `sim::staff::
  StaffPools`, a plain field on `World` beside `Economy`, four aggregate
  counters reset and claimed in stable order each tick (GAMEPLAY.md §9).
- Spoilage and corrosion components (GAMEPLAY.md §9) — needed before tiers
  4/5.
- iOS export viability — re-run the spike with full Xcode (§9a).
- Android runtime launch — unverified (§9a). Close before milestone 10.
- Cross-platform determinism hash test in CI (§6).

---

## 12. References

**Factorio engineering:** [FFF #176 belts](https://www.factorio.com/blog/post/fff-176),
[FFF #421 optimizations](https://www.factorio.com/blog/post/fff-421),
[Desynchronization wiki](https://wiki.factorio.com/Desynchronization),
[Active chunks forum](https://forums.factorio.com/viewtopic.php?t=107797).

**Data-oriented design:** [Mike Acton, CppCon 2014](https://www.youtube.com/watch?v=rX0ItVEVjHc).

**AI-agent evidence:** [How Claude Code is built](https://newsletter.pragmaticengineer.com/p/how-claude-code-is-built),
[17-language chess-engine study, arXiv 2606.13763](https://arxiv.org/abs/2606.13763),
[Perish or Flourish, arXiv 2601.02060](https://arxiv.org/pdf/2601.02060),
[LLM code quality by language, arXiv 2502.01853](https://arxiv.org/html/2502.01853),
[Building a C compiler with parallel Claudes](https://www.anthropic.com/engineering/building-c-compiler),
[Rust and LLMs](https://dev.to/arezvov/rust-and-llms-the-compiler-does-what-code-review-shouldnt-have-to-3ia4),
[Fixing Rust errors with LLMs, arXiv 2308.05177](https://arxiv.org/pdf/2308.05177).

**Engine/tooling:** [Bevy production readiness #21911](https://github.com/bevyengine/bevy/discussions/21911),
[Bevy mobile #20998](https://github.com/bevyengine/bevy/discussions/20998),
[gdext](https://github.com/godot-rust/gdext),
[gdext compatibility policy](https://godot-rust.github.io/book/toolchain/compatibility.html),
[GDExtension iOS issue #85158](https://github.com/godotengine/godot/issues/85158),
[Godot forum: GDExtension + iOS](https://forum.godotengine.org/t/gd-extension-in-ios/132963),
[`hecs`](https://crates.io/crates/hecs).
