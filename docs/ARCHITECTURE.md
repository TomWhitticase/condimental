# Sauce Factory — Architecture Plan

A Factorio-like automation/logistics game, sauce-themed. Target platforms: PC, Mac, mobile.

This doc is about **architecture and technology choices only** — how the game
is built, not what it plays like. Gameplay design (specific recipes,
ingredients, mechanics) belongs in separate design docs; this file should stay
useful even if the theme or feature set changes.

This doc follows the pattern established by Factorio's own architecture (deterministic tick simulation, data-oriented ECS, aggregate item representation, tiered update scheduling) and Godot's text-native, agent-friendly project structure. This file is self-contained — it includes the reasoning behind each tech choice and the sources it's based on, so it doesn't depend on any earlier conversation.

**Revision note:** this plan went through an internal critical review after the first draft, specifically hunting for things that would trip up development later. Several points that were originally left as open options ("X or Y", "prefer X") have since been turned into firm decisions — determinism mechanism (§3.9), ECS/threading primitive (§3.8), SimBridge scope (§4), version pinning (§1a) — because the review's main finding was that hedged decisions in an architecture doc are themselves a risk on an agent-built project: an agent will pick *something* when it hits an "or," and it should be picking the one the team actually chose, not whichever is easier to write in the moment.

---

## 0. Background & rationale (why this stack)

The overall goal is a codebase where an AI coding agent (Claude Code or similar) can do most of the implementation work reliably. That constraint, not just "what's technically best for a factory game," drove the following decisions.

**General evidence on AI-agent-friendly development** (not game-specific, but applied throughout this plan):

- **Training-data density ("on-distribution") beats paradigm elegance.** Anthropic built Claude Code itself in TypeScript specifically because it's a stack the model has seen enormously more of — this is *why* Claude was able to write ~90% of Claude Code's own codebase. The lesson generalizes: prefer mainstream, heavily-documented tech over a technically "purer" niche option.
- **Static/strong typing's main value for agents is the compiler-error feedback loop, not upfront error prevention.** An agent that gets a specific, structured compile error can self-correct in a tight loop; this matters more than the type system stopping mistakes before they're written.
- **Functional programming does *not* currently help LLM code generation**, despite the intuition that stateless/declarative code should be easier to reason about. A 2026 study ("Perish or Flourish?") found Haskell/OCaml/Scala underperform imperative languages for LLM generation, because training corpora are overwhelmingly imperative — an availability problem, not a paradigm-difficulty one.
- **Language choice no longer blocks *whether* an agent can build something, but it does control cost, quality ceiling, and how much supervision you need.** A 2026 study had Claude Code and Codex build chess engines in 17 languages, from mainstream to Brainfuck — all worked, but "strong playing strength is only reachable in mainstream compiled languages," and cost/effort rose sharply with exotic languages.
- **Visual/node-based tooling is a hard capability boundary, not a style preference.** LLM agents operate on text diffs. Anything that can't be faithfully represented as text (Blueprints, visual scripting graphs, binary scene formats) is something agents can't reliably read or edit — 2026 reporting on agentic game-dev specifically flags Unreal Blueprints as "largely AI-resistant" for this reason, while text-serialized engines are the most reliable combination.

**Why Godot over Unity or Unreal:**

Godot's `.tscn`/`.tres` scene and resource files are plain text by design, so an agent edits and diffs them exactly like source code. Unity has improved (YAML text scene serialization is now default, and MCP servers exist to bridge live-editor state) but still layers a binary asset database underneath. Unreal's Blueprint-centric workflows hit the visual-scripting boundary above directly. Godot isn't chosen for being open-source or trendy — it's chosen because the whole project stays text-native end to end.

**Why not a pure-Rust engine (Bevy)** — the obvious alternative once Rust is in play:

- Bevy is pre-1.0 and ships breaking API changes roughly every 3 months, with migration guides listing dozens of renames per release. That churn means a chunk of any LLM's training data on Bevy is for an API version that no longer exists, producing confident hallucinations of stale method signatures.
- Bevy's mobile (iOS/Android) export is functional but noticeably thinner than Unity's or Godot's — a direct problem given this project targets PC/Mac/mobile.

**Why Rust specifically for the simulation core** (via Godot's GDExtension, not as the whole engine):

Rust's compiler gives an unusually sharp agentic feedback loop: the LLM writes code, `cargo check` rejects it with a specific ownership/borrow violation, the LLM fixes exactly that. Research on LLM-assisted Rust repair found this loop resolves borrow-checker violations correctly roughly 74% of the time. That's a sharper version of the "type system as feedback loop" effect than TypeScript or C# provide — worth having specifically on the performance-critical, correctness-sensitive simulation core, without taking on Bevy's engine-level risks by using it for the whole project.

**Known risk this plan accepts, and how it's contained — `gdext` carries a version of the same risk as Bevy.** `gdext` (godot-rust) won't be considered API-stable until its own 1.0, minor versions are allowed to break, and it's more niche than Bevy — meaning less training data, so the "stale method signature" problem the Bevy section above describes can hit here too, arguably harder to self-correct from. This is accepted rather than avoided, because the alternative (writing the Rust↔Godot boundary in C++ via GDNative/classic bindings, or dropping Rust from the equation) is worse on every other axis this doc cares about. The mitigation is process, not architecture: pin exact versions (§1a), treat every `gdext` minor-version bump as a deliberate, reviewed decision — read its migration notes first, the same way the team would treat a Bevy upgrade — and keep the actual surface of `gdext` API touched by this codebase small (§4), since a smaller surface is less exposed to any one breaking change.

**Known risk this plan accepts — mobile export via GDExtension (not plain GDScript) is less proven than the "Godot beats Bevy on mobile" framing above implies.** That framing is true for GDScript-only Godot projects; it does not automatically transfer to a GDExtension-based one. As of this writing, iOS export of GDExtension-based projects is explicitly flagged experimental/buggy in Godot's own issue tracker and forums (works on Android/macOS, fails on iOS in some reported cases); Android export works but requires real NDK cross-compilation setup, not a checkbox. See §9 for how this plan de-risks that early instead of discovering it at platform-polish time.

---

## 1. Tech stack

| Layer | Tech | Role |
|---|---|---|
| Simulation core | **Rust**, `hecs` (ECS) + `rayon` (parallelism) | Deterministic game logic: belts, machines, fluid networks, timed processes, power. No Godot dependency. |
| Interop | **GDExtension** via `gdext` | Thin binding layer between Rust core and Godot. |
| Presentation | **Godot 4.x**, GDScript | Rendering (TileMap), UI, input, camera, build mode, animation/interpolation. |
| Data | **RON or JSON** files | Items, recipes, machines — all content is data-driven, not hardcoded. |
| Tests | **Rust unit/integration/replay tests** (headless) + **GUT** (Godot) | Simulation invariants, determinism, interop boundary + UI smoke tests. See §6. |

**Hard rule:** anything that must be deterministic or fast lives in Rust. Anything about how it looks/feels lives in Godot. The boundary between them is the `SimBridge` API described in §4.

**Why `hecs` over `bevy_ecs` (resolved — was previously left open):** `bevy_ecs` used standalone gives a built-in parallel scheduler, but as a dependency it still inherits Bevy's own churn/breaking-change cadence — the exact risk §0 uses to reject Bevy-as-engine, reintroduced through the back door as a library. `hecs` is smaller, more stable, and has a slower-moving API, at the cost of having no scheduler of its own — which is fine here, because §3.8 already calls for a custom network-partitioned scheduler rather than a generic per-system one; `rayon` supplies the actual parallelism primitive under that scheduler.

### 1a. Pinned versions

Pin these explicitly rather than "latest" — bump only as a deliberate, recorded decision (add a line to the CLAUDE.md decisions log when you do), not incidentally via `cargo update`/editor auto-update:

| Dependency | Pinned to |
|---|---|
| Godot | 4.6.x |
| `gdext` | latest version supporting the Godot 4.6 API level |
| Rust | edition 2024, MSRV = latest stable at project start |
| `hecs` | latest stable at project start |

Rationale: §0 already argues Bevy's churn poisons an agent's training-data reliability for it; `gdext` is comparatively niche and evolving too (see §0's `gdext` risk note), so the same discipline applies to it, not just to the thing being rejected.

---

## 2. High-level architecture

```mermaid
flowchart TB
    subgraph Presentation["Presentation Layer — Godot / GDScript"]
        UI[UI: build menu, recipe browser, factory stats]
        Render[TileMap renderer + interpolation]
        Input[Input / camera / build mode]
    end

    subgraph Interop["Interop Layer — GDExtension (gdext)"]
        Bridge[SimBridge API:\ntick(), dirty_chunks(), issue_command(),\npreview_command(), query()]
    end

    subgraph Sim["Simulation Core — pure Rust, headless, deterministic"]
        World[Chunked tile world]
        ECS[ECS: belts, machines, fluid networks, timed processes, power]
        Scheduler[Tiered update scheduler\nactive / warm / sleeping]
        Threading[Dependency-group threading]
    end

    Data[(Data files: items.ron, recipes.ron, machines.ron)]

    UI --> Bridge
    Input --> Bridge
    Bridge --> Render
    Bridge <--> Sim
    Data --> Sim
    Data --> UI
```

Data files are the single source of truth for both the simulation (what a recipe consumes/produces) and the UI (what to display in the build menu) — no duplicated definitions.

---

## 3. Simulation core (Rust)

### 3.1 World representation

- Tile grid divided into **32×32 chunks**, matching Factorio's proven size — big enough to amortize per-chunk overhead, small enough for fine-grained activity tracking.
- Only chunks near a player, or containing something mid-process, are in the "active" set each tick.

### 3.2 Core components (ECS)

| Component | Purpose |
|---|---|
| `Position` | Tile coordinate |
| `BeltSegment` | Ordered belt line using **gap-based item storage** (see §3.3) — carries solid items |
| `FluidNetwork` | Connected pipe network carrying a liquid as a pressure/volume value, **not** individual particles |
| `Machine` | Current recipe, progress timer, input/output buffers |
| `TimedProcess` | Generic tick-counted long-duration process state (see §3.5) — attached to any entity that needs to represent a multi-tick process beyond a single machine cycle |
| `PowerNode` | Producer/consumer, grouped into `PowerNetwork` |
| `Inventory` | Generic slot-based storage (chests, machine buffers, player) |

### 3.3 Solid items on belts

Directly apply Factorio's FFF #176 trick: a belt line stores **gaps between items**, not absolute item positions. Track the index of the last non-zero gap; while a line is jammed, only that boundary value changes each tick, and everything behind it is provably untouched. This is what lets a factory with thousands of items on belts stay cheap.

### 3.4 Liquids / fluid networks

Model as a **connected-network pressure system** (Factorio pipes), not particles: each `FluidNetwork` has a total volume and a fluid type; flow between adjacent segments equalizes pressure each tick. Never simulate individual droplets — that was the exact mistake Factorio's own devs made with belt items early on, and fluids are the same trap at a smaller item size.

### 3.5 Long-running / timed processes (generic pattern)

Some gameplay mechanics need a process to run over many ticks before producing a result (anything slower than a single machine cycle — aging, curing, growth, etc., depending on what design settles on). The architectural pattern for this, regardless of which specific mechanic uses it: a **tick-counted state machine** (`TimedProcess { start_tick, required_ticks }`) attached to a container, never wall-clock time.

- **Why tick-counted, not wall-clock:** if multiplayer/lockstep is ever added (§8), any wall-clock read is an instant desync source. Counting sim ticks keeps it deterministic for free, and also means fast-forwarding/pausing the simulation behaves correctly.
- Progress is a function of `(current_tick - start_tick)` vs. `required_ticks`. Any gameplay meaning layered on top of that progress value (quality, yield, etc.) is a design decision, not an architecture one — this component just guarantees the timing itself stays deterministic.

### 3.6 Recipes & content — fully data-driven

`recipes.ron` example shape (placeholder ids — actual items/recipes are a game-design concern, not an architecture one):

```ron
Recipe(
    id: "example_recipe",
    inputs: [("ingredient_a", 4), ("ingredient_b", 1)],
    outputs: [("output_item", 2)],
    machine: "processing_machine",
    time_ticks: 900,
)
```

New items, recipes, and machines are added by editing data files, never simulation code — this keeps the Rust core stable (fewer breaking changes = better long-term agent-generated-code reliability, per the earlier Bevy-API-churn discussion) while content grows freely.

### 3.7 Update scheduling (tiered)

- **Active:** belts/machines currently processing something — updated every tick.
- **Warm:** trucks/conveyor-bots in transit — updated every N ticks, position interpolated for rendering in between (Factorio's robot pattern).
- **Sleeping:** a fully-stocked, blocked machine or an idle timed-process mid-wait — skips simulation entirely until an adjacent change wakes it (an item arrives, a recipe finishes upstream).

### 3.8 Threading

Partition by **connected network**, not by entity type. Concurrency primitive: **`rayon`**, used to run independent partitions across a thread pool — not a hand-rolled thread-per-network model.

- All belt-reader passes (read-only) run in parallel freely.
- A given `PowerNetwork` or `FluidNetwork` must be updated entirely by one thread — cross-network order-dependence is exactly the desync bug Factorio hit with shared-entity electric networks. Compute connected-component grouping via **union-find**, once per topology change (a belt/pipe/wire placed or removed), not every tick. Each connected component becomes one `rayon` task.
- **Boundary entities** — an entity that is simultaneously a member of two independently-threaded networks (e.g. a pump that's part of both a `FluidNetwork` and a `PowerNetwork`, or an item handing off between two separately-partitioned belt networks) are a synchronization point, not a normal member of either network. They are updated in a single-threaded pass that runs *after* all per-network parallel passes for the tick complete, reading the now-finalized state of every network it touches. This generalizes the power-network rule above to the general case instead of leaving it implicit.

### 3.9 Determinism

**Decision: fixed-point, not floating point, for all simulation state that is shared across systems, persisted, or could ever be replayed/compared across machines** (positions, item counts, fluid volumes/pressure, power values, timed-process progress). This was previously left as "fixed-point or a deterministic float policy" — that hedge is the risk, not a real option: fixed-point means one integer-backed numeric type threaded through the ECS from the start; "deterministic floats" means solving bit-identical FP behavior across x86_64 (PC) and Apple Silicon ARM64 (Mac) — the project's own two target desktop platforms — which is a genuinely open, often-unsolved problem (FMA contraction, differing transcendental implementations, summation order), not a checkbox. §0's own principle — decide the hard-to-retrofit thing now — applies most to this exact decision, so it's being made now: use a fixed-point crate (e.g. `fixed`) for the types above. Floats remain fine for anything purely presentational in Godot (tweening, camera, particle timing) that never feeds back into simulation state.

Also: fixed timestep (e.g. 60 ticks/sec), deterministic iteration order (stable entity IDs, no hashmap iteration in hot paths). See §6 for how this is actually tested, not just asserted.

---

## 4. Interop layer (`gdext`)

Keep the Rust↔Godot boundary deliberately narrow. The first draft of this API was underspecified in ways that matter — no way to preview a command without applying it, no save/undo story — so this version fills those in:

```rust
pub struct SimBridge;
impl SimBridge {
    pub fn tick(&mut self);

    // Changed-since-last-call, at chunk granularity (not per-entity — see
    // "Delta granularity" below), so Godot only re-reads chunks that actually
    // changed AND are currently visible.
    pub fn dirty_chunks(&self) -> &[ChunkId];
    pub fn chunk_snapshot(&self, id: ChunkId) -> ChunkSnapshot;

    // Mutating: place/rotate/recipe-select/etc. Also pushed onto the
    // undo/inverse-command stack (see "Undo/redo" below).
    pub fn issue_command(&mut self, cmd: PlayerCommand);

    // Non-mutating dry run for build-mode ghosts/placement validation.
    // Never touches real world state — implemented as a validation pass,
    // not "mutate then roll back."
    pub fn preview_command(&self, cmd: &PlayerCommand) -> PreviewResult;

    pub fn query_tile(&self, pos: TileCoord) -> Option<TileSnapshot>;

    pub fn undo(&mut self) -> bool;
    pub fn redo(&mut self) -> bool;
}
```

Godot never sees internal representations (gap arrays, fluid pressure math) — only snapshots and deltas. This means the belt-compression algorithm, the fluid model, anything in §3 can be rewritten later without touching a line of GDScript.

**Delta granularity (resolved — was previously unaddressed):** deltas are per-*chunk*, not per-entity. At the scale milestone 6 stress-tests for ("thousands of entities"), a per-entity `Vec<EntityDelta>` reallocated and copied across FFI every tick is the kind of thing that's invisible at milestone 2 and shows up as frame-time spikes only once the world is large — precisely because chunks (§3.1) already exist as a partitioning unit, dirty tracking reuses them instead of inventing a second granularity. `dirty_chunks()` returns a reused, cleared-not-reallocated buffer.

**Undo/redo (resolved):** implemented Rust-side as a bounded stack of `(command, inverse_command)` pairs — not a Godot-side snapshot diff. This keeps undo consistent with the save format below (both are built out of the same command representation) and keeps Godot from needing to understand simulation internals to reverse an action.

**Save format (resolved — was previously unaddressed):** a save is a **command log plus periodic full-state snapshots** (snapshots are a load-time optimization on top of the log, not the source of truth), not a bare state dump. This is deliberately the same representation §7's lockstep multiplayer needs (replaying commands against a known start state), so save/load and multiplayer replay share one mechanism instead of two, and it's also exactly what a determinism/replay test (§6) replays against.

**Data-file schema versioning:** every RON data file type (`recipes.ron`, `items.ron`, `machines.ron`) carries a `schema_version` field; the loader rejects or explicitly migrates a mismatched version rather than silently misreading a changed shape. This is a separate concern from save-file versioning above — content can change shape between playthroughs of the same save.

**Interop-layer training-data gap:** `SimBridge`'s own API is project-specific and appears in no training corpus, unlike plain Rust or plain GDScript — this is very likely to be the least agent-friendly part of the codebase in practice (see §0's `gdext` risk note for the same issue one layer down). Mitigation: once this API exists, maintain a small set of worked, runnable examples of every call pattern in-repo (doc-tests or an `examples/` crate), and treat changes to this file with higher scrutiny than changes confined to one side of the boundary — an agent can't lean on the outside world for this layer, only on what's already in the repo.

---

## 5. Presentation layer (Godot)

- **TileMap** for the factory floor; layers for ground, belts/pipes, buildings.
- **Interpolation**: for anything in the "warm" tier (§3.7), tween between the last two known snapshots rather than snapping — smooth visuals independent of simulation tick rate.
- **UI reads the same data files** as the simulation (recipes, items) — the build menu is generated from `recipes.ron`, not hand-authored per recipe.
- All scenes/resources kept as Godot's default **text-based `.tscn`/`.tres`** — never switch to binary scene serialization, since that's the property that makes this whole project agent-diffable.
- Avoid Godot's visual scripting entirely; GDScript/C#/Rust text only.

---

## 6. Testing strategy

- **Headless Rust integration tests** simulating N ticks against known factory layouts, asserting invariants: no item duplication/loss across a belt network, fluid volume conservation, timed-process state correctness. This is the project's actual "compiler feedback loop" for gameplay logic — lean on it heavily for any AI-agent-driven changes to the simulation core.
- **Scripted playtests**: build a known factory (e.g. "N input producers → 1 processing chain"), run it headless for X ticks, assert throughput matches the theoretical recipe-chain rate.
- **Determinism/replay tests** (new tier — previously missing, despite determinism being this doc's central bet in §3.9): record a fixed command log, run it twice, hash world state at fixed tick intervals, assert the hash sequence is identical. Once both desktop platforms are in CI, run the same log on an x86_64 runner and an ARM64 runner and assert identical hashes — this is the test that would actually catch a determinism regression (e.g. a stray `f32` sneaking into shared state) before it's buried under months of code, not just something asserted in prose.
- **Interop boundary tests** (new tier — previously missing): tests that exercise `SimBridge` itself — issue a command, tick, confirm `dirty_chunks`/`chunk_snapshot`/`query_tile` reflect it correctly, confirm undo/redo round-trip — without a running Godot process. This is the layer §4 flags as having the least external training data to lean on, so it's also the layer most likely to silently rot without its own tests; "Rust tests cover sim, GUT covers UI" left it with zero coverage.
- **GUT tests** in Godot for UI/menu logic only — not simulation correctness.

---

## 7. Development loop (new section — was previously unaddressed)

The whole rationale for this architecture is a tight edit-compile-test loop for an agent. That loop was previously asserted, not designed — GDExtension hot-reload inside a running Godot editor exists since Godot 4.2 but has open issues around editor-plugin reload ordering and reports of editor crashes on recompile-while-open. Relying on it as the primary loop would tax every single iteration on the simulation core, which is where most of the work happens.

**Decision:** the primary iteration loop is **headless** — `cargo check` / `cargo test` against the Rust simulation core, with no Godot process involved at all. The Godot editor is opened only for milestone-level visual verification, not per-edit. This is also just consistent with §6 already calling headless tests "the project's actual compiler feedback loop" — it means that loop doesn't get undermined by editor hot-reload flakiness on the majority of changes.

Concretely: milestone 1 (§9) must produce a checked-in build script (or `justfile`/`cargo xtask`) that does `cargo build` → copies the resulting `.so`/`.dylib`/`.dll` to where the project's `.gdextension` file expects it, and that `.gdextension` file itself gets checked in at that point too. Don't leave this implicit until someone needs it.

---

## 8. Multiplayer (deferred, but designed for)

Not in early milestones, but §3.9's determinism work means it stays possible without a rewrite: multiplayer would use deterministic lockstep (send player commands, not state) exactly as Factorio does, and this only works because §3.5–3.9 were designed deterministically from the start. It also reuses the command-log save format from §4 directly — a lockstep session and a save file are the same underlying representation.

---

## 9. Milestones

1. **Scaffold** — Godot project + `gdext` hello-world binding, confirm Rust↔Godot round-trip works. Also produces the build script and `.gdextension` file from §7.
2. **Mobile export spike** (moved early — was previously only in platform polish, at milestone 8, where the risk would surface 7 milestones too late): export the milestone-1 hello-world GDExtension to Android, and separately attempt iOS given its export path is flagged experimental upstream (§0). If iOS GDExtension export is genuinely broken at this point, decide explicitly whether iOS slips to a stretch platform rather than discovering that during platform polish.
3. **Core loop proof** — one belt, one machine, one recipe (raw input → processed output), rendered and interactable.
4. **First full chain** — a multi-step production chain end-to-end, driven entirely by data-driven recipes.
5. **Fluids** — pipe network, a recipe requiring a liquid input.
6. **Timed processes** — the generic tick-counted long-duration process pattern (§3.5), proven with one process end-to-end.
7. **Scale-up** — power system, tiered update scheduler, chunk activity tracking; stress-test to thousands of entities. This is also the point at which §4's per-chunk delta design and §3.8's threading model actually get load-tested — don't wait longer than this to find out if they hold up.
8. **Logistics expansion** — transport between multiple factory sites.
9. **Platform polish** — mobile touch controls/export, Mac/PC packaging, UI pass. Mobile export risk should already be known from milestone 2; this is about polish, not first discovery.
10. *(Stretch)* **Multiplayer** — lockstep networking on top of the already-deterministic core.

---

## 10. AI-agent development conventions for this repo

- Keep the Rust simulation core covered by headless tests before merging any agent-written change to it — that's the fast, precise feedback loop this architecture is built around.
- Content (recipes/items/machines) changes should never require touching Rust — if an agent needs to modify simulation code just to add a new item or recipe, that's a sign a data field is missing, not a reason to hardcode.
- Keep `.tscn`/`.tres` as text; never let a "convert to binary for performance" temptation in — the diffability is worth more than the marginal load-time savings at this scale.
- One `CLAUDE.md`-equivalent per major module (`sim_core/`, `godot_project/`) documenting the invariants that aren't obvious from the code (e.g. "belt gaps must remain non-negative," "fluid network recompute only on topology change").
- No `f32`/`f64` in any Rust ECS component that participates in shared simulation state (§3.9) — if a value needs to be shared, persisted, or replayed, it's fixed-point. Floats are fine only for Godot-side, presentation-only values that never feed back into the sim.
- Treat any `gdext` version bump as a reviewed decision, not a routine dependency update — read its migration notes first (§0, §1a).
- The primary edit-test loop is headless `cargo test`, not the Godot editor (§7) — don't default to editor-reload cycles for simulation-core changes.

---

## 11. References

**Factorio engineering (primary source for the simulation-architecture patterns in §3, §6):**
- [Friday Facts #176 — Belts optimization for 0.15](https://www.factorio.com/blog/post/fff-176) — gap-based belt item storage, the "compression is permanent" invariant
- [Friday Facts #421 — Optimizations 2.0](https://www.factorio.com/blog/post/fff-421) — multithreading by dependency group, tiered update scheduling (robots/roboports)
- [Desynchronization — Factorio Wiki](https://wiki.factorio.com/Desynchronization) — deterministic lockstep networking
- [Active chunks — Factorio Forums](https://forums.factorio.com/viewtopic.php?t=107797) — chunk-based entity activity tracking

**Data-oriented design / ECS foundations (§3.2):**
- [CppCon 2014: Mike Acton — "Data-Oriented Design and C++"](https://www.youtube.com/watch?v=rX0ItVEVjHc) — the discipline underlying why ECS architectures fit this genre

**AI-agent development evidence (§0):**
- [How Claude Code is built — Pragmatic Engineer](https://newsletter.pragmaticengineer.com/p/how-claude-code-is-built) — "on-distribution" rationale for TypeScript, Claude writing ~90% of its own codebase
- [Do programming languages still matter to your AI coding agent teammate? — arXiv 2606.13763](https://arxiv.org/abs/2606.13763) — 17-language chess-engine study
- [Perish or Flourish? LLM Code Generation in Functional Programming — arXiv 2601.02060](https://arxiv.org/pdf/2601.02060) — functional languages underperform imperative for LLM generation
- [Security and Quality in LLM-Generated Code — arXiv 2502.01853](https://arxiv.org/html/2502.01853) — typed vs. dynamic language generation accuracy
- [Building a C compiler with a team of parallel Claudes — Anthropic](https://www.anthropic.com/engineering/building-c-compiler) — Rust chosen for a large agent-built project
- [Rust and LLMs: the compiler does what code review shouldn't have to](https://dev.to/arezvov/rust-and-llms-the-compiler-does-what-code-review-shouldnt-have-to-3ia4) — borrow-checker repair feedback loop
- [Fixing Rust Compilation Errors using LLMs — arXiv 2308.05177](https://arxiv.org/pdf/2308.05177) — ~74% borrow-checker violation repair rate via compiler feedback

**Engine/tooling specifics (§0):**
- [Bevy — "When will Bevy be Production Ready?" (GitHub Discussion #21911)](https://github.com/bevyengine/bevy/discussions/21911) — API churn discussion
- [Future support for exporting Bevy games to Android and iOS (GitHub Discussion #20998)](https://github.com/bevyengine/bevy/discussions/20998) — mobile export maturity
- [godot-rust `gdext` — Rust bindings for Godot 4](https://github.com/godot-rust/gdext) — the interop layer this plan uses in §4
- [Compatibility and stability — The godot-rust book](https://godot-rust.github.io/book/toolchain/compatibility.html) — `gdext`'s own API stability policy pre-1.0, source for §0/§1a's version-pinning rationale
- [GDExtension iOS export minimum OS version is hard coded — godotengine/godot #85158](https://github.com/godotengine/godot/issues/85158) and the [Godot Forum thread on GDExtension + iOS](https://forum.godotengine.org/t/gd-extension-in-ios/132963) — source for the iOS GDExtension export risk noted in §0/§9
- [`hecs` — crates.io](https://crates.io/crates/hecs) — the ECS crate chosen in §1; verify current maintenance activity before relying on it long-term, per the review that produced this revision
