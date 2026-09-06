# sauce-factory

Factorio-like automation game, themed around sauce production. Pre-alpha.

@docs/ARCHITECTURE.md
@docs/UI_UX.md
@docs/GAMEPLAY.md

The three files above are all authoritative and always apply. ARCHITECTURE.md is
the tech-stack/architecture plan (Rust simulation core + Godot/GDScript
presentation via `gdext`, data-driven recipes, deterministic tick simulation).
UI_UX.md is the visual-style and UI/UX plan (isometric voxel art, 32×32×32
tile-scale assets, industrial sauce-factory theme, Minecraft-adjacent UI with
clipboard/checklist motifs). GAMEPLAY.md is the gameplay/progression/economy
plan (sauce-tier progression gated by equipment unlocks like refrigeration and
acid-handling, money economy, and contracts — not a spot market — as the
demand system, deliberately forcing a diversified factory rather than one
optimized sauce). Always follow all three when writing code — each also
explains *why* its choices were made, which matters when a change seems to
conflict with them.

## Working conventions

- Follow docs/ARCHITECTURE.md for stack/architecture decisions, docs/UI_UX.md
  for anything visual/UI, and docs/GAMEPLAY.md for anything about mechanics,
  progression, or economy — don't deviate (e.g. don't reach for
  Unity/Unreal/Bevy, don't put gameplay logic in GDScript, don't hardcode
  recipes/ingredients in Rust, don't introduce flat-textured non-voxel assets
  or a free-rotating camera, don't add a spot market alongside contracts)
  without discussing it first.
- This project is being built primarily by Claude Code. When a non-trivial design
  decision gets made that isn't already covered by one of the three docs, record
  it in the decisions log below (and in the relevant doc itself) so future
  sessions don't re-litigate it.
- Prefer small, playable increments over big upfront systems — this is a game, favor "can I run it and see something" early.
- Use plan mode for anything that touches architecture or adds a new major system; small fixes/tweaks don't need it.

## Project structure

Milestones 1 (Scaffold), 3 (Core loop proof), 4 (First full chain), and 5
(Economy MVP) have landed: a Rust/gdext <-> Godot round trip, plus real
simulation code — a multi-stage production chain (raw tomato -> tomato sauce
-> ketchup -> bottled ketchup -> boxed ketchup) built entirely from
`data/recipes.ron`, now with a money counter and a single hardcoded contract
the chain can actually deliver against and get paid for. A visual-polish
pass on top of that built UI_UX.md §2a's voxel asset pipeline and replaced
the flat 2D placeholder scene with an isometric, real-time-lit 3D one.
Current layout:

```
sauce-factory/
  CLAUDE.md            # this file — always loaded, points at the three docs below
  CONTRIBUTING.md       # commit message convention
  README.md              # short public-facing pitch
  Cargo.toml             # workspace root — members: sim_core, xtask, assets/voxels
  .cargo/config.toml      # `cargo xtask` alias
  assets/voxels/           # the voxel asset pipeline (UI_UX.md §2a/§2b) — a
    Cargo.toml               # workspace member (`voxgen`) whose source IS the
    palette.ron               # `generators/` dir. palette.ron is the single
    generators/                # source of truth for colour AND material class
      main.rs                   # (matte/metal/emissive); every generator
      palette.rs                 # references it, none invent their own colours.
      vox.rs                      # vox.rs is a hand-written MagicaVoxel encoder
      assets.rs                    # (no crate can write what we need — §2b).
    models/                    # generated .vox + materials.json/index.json —
                                 # build output, COMMITTED, never hand-edited
    previews/                   # gitignored isometric PNGs for visual review
                                  # (incl. scene.png, a full render of the scene)
  data/                    # data-driven content (ARCHITECTURE.md §3.6) — the
    recipes.ron              # single source of truth for recipes; sim_core embeds
                               # this file at compile time (include_str!), never
                               # duplicates its content in Rust code. Its list
                               # order IS the production chain (World builds one
                               # stage per recipe, in file order — see world.rs) —
                               # adding a step is a pure edit here. items.ron/
                               # machines.ron/contracts.ron land here too when they exist.
  sim_core/               # Rust crate: pure sim core + the gdext binding
    Cargo.toml              # crate-type = ["cdylib", "rlib"] — rlib is what lets
                              # sim_core/tests/ link against it without Godot
    src/lib.rs                # the ONLY file that touches `godot`/gdext (ARCHITECTURE.md §4)
    src/sim/                   # pure Rust simulation core — no gdext dependency,
      mod.rs                    # fully exercisable by headless `cargo test` (§7)
      types.rs                   # Fx (fixed-point, §3.9), ItemId/RecipeId/TileCoord/ChunkId
      recipe.rs                   # RecipeRegistry — loads + validates data/recipes.ron
      inventory.rs                  # generic slot-based Inventory (§3.2)
      belt.rs                        # BeltSegment — gap-based item storage (§3.3)
      machine.rs                      # Machine + TimedProcess (§3.2/§3.5)
      economy.rs                       # Economy: money (Fx) + one hardcoded Contract
                                         # (fixed mix/cadence/payout), checked against
                                         # an accumulating delivery_buffer Inventory
                                         # (GAMEPLAY.md §6; milestone 5, Economy MVP)
      world.rs                         # hecs::World wrapper; builds a multi-stage
                                         # production chain (one belt+machine per
                                         # recipe, in recipes.ron order); owns Economy
                                         # and ticks it once per World::tick; SimBridge's
                                         # pure-logic half — tick()/dirty_chunks()/
                                         # chunk_snapshot()/issue_command()/
                                         # query_tile()/chain_layout()/economy_snapshot() (§4)
    tests/core_loop.rs        # headless integration test: full-chain N-tick
                                # conservation invariant across every stage (§6)
    tests/economy.rs           # headless integration test: full chain collects
                                 # against the hardcoded contract and pays out
                                 # correctly (money increases on a satisfied mix,
                                 # stays put on a miss) — economy.rs's own #[cfg(test)]
                                 # covers Economy in isolation (cadence/mix edge cases)
  xtask/                   # build script (ARCHITECTURE.md §7): `cargo xtask` builds sim_core,
    Cargo.toml              # copies the compiled extension into godot_project/bin/, and stages
    src/main.rs               # assets/voxels/models/ into godot_project/voxels/ (both gitignored)
  godot_project/             # Godot 4 project (presentation layer)
    project.godot              # run/main_scene = main.tscn
    sim_core.gdextension       # checked in; points at godot_project/bin/, which is gitignored (build output)
    main.tscn / main.gd          # interactive 3D scene, built entirely in code from
                                   # chain_layout_json + chunk snapshots: voxel meshes,
                                   # a FIXED orthographic isometric camera, real-time
                                   # lights/shadows/SSAO/glow, SPACE/C still drive
                                   # issue_command_json, a readout panel reads
                                   # economy_snapshot_json for money/contract status
                                   # (UI_UX.md §1/§2/§4, ARCH §5; not UI_UX.md §5's
                                   # clipboard/checklist styling yet)
    addons/                         # vendored+pinned .vox importer — see its VENDORED.md
      MagicaVoxel_Importer_with_Extensions/
    voxels/                          # gitignored: .vox models staged here by `cargo xtask`
    test_round_trip.gd          # milestone-1 headless proof script (scripts/verify-scaffold.sh)
    test_core_loop.gd            # milestone-3/4/5 headless proof: real gdext JSON
                                   # boundary, driven through the whole chain, incl.
                                   # economy_snapshot_json's shape/initial state
    test_interaction.gd           # milestone-3 headless proof: keyboard input -> SimBridge
    test_presentation.gd           # headless proof of the 3D scene's structure: orthographic
                                     # camera, lights+shadows, SSAO/glow, imported voxel
                                     # meshes, metal shinier than matte (UI_UX.md §1/§2/§4)
    test_screenshot.gd              # NOT headless — renders the real scene to a PNG so a
                                      # visual change can actually be looked at
    bin/                          # gitignored: compiled extension lands here via `cargo xtask`
  scripts/
    lint-commit-msg.sh          # commit-msg hook
    verify-scaffold.sh           # milestone-1 headless verification (build + Godot cache + test script)
    verify-core-loop.sh           # milestone-3/4/5 + visual-pass headless verification
                                    # (cargo test + gdext boundary + interaction + 3D scene)
    build-voxels.sh                # regenerate every .vox from its Rust generator, then
                                     # render isometric review PNGs via vengi-thumbnailer
    screenshot-scene.sh             # render main.tscn to a PNG (the one non-headless check)
  docs/
    README.md              # docs-folder index — read this first when only skimming
    ARCHITECTURE.md          # how the game runs and is built
    UI_UX.md                  # how the game looks and behaves
    GAMEPLAY.md                # what the game actually is to play
    DEVLOG.md                   # status tracker + dated log — not @-included, check/update separately
  .githooks/              # git hooks (commit-msg, pre-commit) — enable via `git config core.hooksPath .githooks`
  .github/workflows/       # CI (commit-message lint)
```

**Data file path (resolved — was ARCHITECTURE.md §11's open question):**
top-level `data/`, sibling to `sim_core/`/`godot_project/` — not nested under
either, since ARCHITECTURE.md §2's own diagram has data files feed both the
simulation *and* the UI as one shared source, and UI_UX.md's `assets/voxels/`
already established the pattern of shared, non-Rust, non-Godot content living
at the repo root. `items.ron`/`machines.ron`/`contracts.ron` belong here too
when they're added.

Still not yet laid out (add when the relevant milestone needs it, per the
docs — don't improvise a different shape when the time comes):

- `rayon`-based parallelism and the tiered active/warm/sleeping scheduler
  (ARCHITECTURE.md §3.7/§3.8) — milestone 3's single belt+machine has no need
  for either yet; both are milestone 8 (Scale-up) scope.
- `preview_command`/`undo`/`redo`/the save-format command log (ARCHITECTURE.md
  §4) — `SimBridge` only implements `tick`/`dirty_chunks`/`chunk_snapshot`/
  `issue_command`/`query_tile` so far, deliberately narrow per that section.
- Per ARCHITECTURE.md §10: `sim_core/` and `godot_project/` should each get
  their own `CLAUDE.md` documenting invariants specific to that module once
  there's enough in either to warrant one — don't let this root file grow to
  cover implementation-level detail that belongs closer to the code.

## Orchestration (Orca)

ARCHITECTURE.md §9's milestone list is this project's natural task DAG — use
Orca's orchestration layer (`orca orchestration ...`, not a generic subagent
tool) to actually build it, rather than one continuous session accumulating
context across every milestone.

- **One Run per meaningful chunk of milestone work** (e.g. "sauce-factory MVP:
  milestones 1-5"), not one Run per tiny task.
- **Milestones map to Tasks roughly 1:1.** Encode ARCHITECTURE.md §9's
  dependency chain with `--deps` so the DAG mirrors the doc instead of
  drifting from it — most of that chain is linear (scaffold → core loop →
  first full chain → economy MVP), with the mobile-export spike as the one
  task that branches off scaffold independently.
- **Every worker task spec must be self-contained.** A dispatched worker
  starts with zero conversational context — it gets the repo (CLAUDE.md and
  the `@`-included docs load automatically) plus whatever the task spec says.
  Name the exact milestone and doc section it's implementing; don't assume it
  knows what "the MVP" refers to.
- **Foundational/large milestones get a fresh top-level worktree** (`--worktree
  new-top-level`), not `current` — main stays clean until the result is
  reviewed and merged. Small, low-risk follow-on work can reuse `current` or a
  child worktree.
- **Prefer `worker-start`** (the composed create-worktree/wait/dispatch path)
  over manual `dispatch --inject` unless a topology it doesn't express is
  actually needed.
- **Use `gate-create` at milestone boundaries** where a human sanity-check is
  warranted (e.g. before merging the scaffold, before starting Economy MVP)
  rather than auto-chaining through the whole DAG unattended — this is a
  foundational, mostly-sequential build where an early bad assumption is
  expensive to discover three milestones later.
- **After every accepted `worker_done`**, decide whether it's DEVLOG-worthy
  (see the Commit messages section above) before dispatching the next task or
  ending the coordinating turn — don't let a completed milestone go unlogged.
- Full ownership handoffs ("hand off this to another agent") are a different,
  simpler tool (`orca-cli`/`orca worktree create --prompt`) — don't reach for
  orchestration's Task/Dispatch machinery unless the work genuinely needs
  supervision, a DAG, or a blocking ask/reply loop.

## Commit messages

Conventional Commits, enforced by a local `commit-msg` hook + CI. See
[CONTRIBUTING.md](CONTRIBUTING.md). Run this once per clone/worktree so commits
made here don't get rejected:

```
git config core.hooksPath .githooks
```

**Before finishing a commit, decide whether docs/DEVLOG.md needs a note.**
This is a real step in the commit workflow, not optional housekeeping — a
`pre-commit` hook will print a reminder if a commit doesn't touch
`docs/DEVLOG.md`, but it never blocks the commit, because the actual judgment
call is yours: most small commits (a wording tweak, a minor fix) genuinely
don't need one. Add a brief, high-level entry when the commit represents a
decision, a milestone, or otherwise-meaningful progress — see docs/DEVLOG.md's
own "How to use this doc" for tone (short, general, not a restatement of the
diff). Update docs/DEVLOG.md's "Current status" section in the same commit if
it's gone stale.

## Decisions log

- 2026-09-06: Commit messages must follow Conventional Commits (enforced via
  git hook + CI) so history can generate release notes later. See CONTRIBUTING.md.
- 2026-09-06: Adopted the architecture plan in docs/ARCHITECTURE.md — Rust
  simulation core (ECS, deterministic ticks) + Godot 4/GDScript presentation via
  `gdext`, data-driven content (RON/JSON). Chosen specifically to be
  agent-friendly (text-native scenes, compiler-error feedback loop, mainstream
  languages). See that doc for full rationale and sources.
- 2026-09-06: Ran a critical review of that architecture doc specifically hunting
  for holes, then resolved every finding into firm decisions rather than leaving
  them open — see docs/ARCHITECTURE.md for details. Highlights: simulation state
  is fixed-point, not float (§3.9); ECS is `hecs` + `rayon`, not `bevy_ecs`
  (§1/§3.8); Godot/`gdext`/Rust versions are now pinned (§1a); `SimBridge` gained
  preview/undo/save-format design (§4); a mobile-export spike moved from
  milestone 8 to milestone 2 (§9); primary dev loop is headless `cargo test`, not
  editor hot-reload (§7). Don't re-litigate these without good reason.
- 2026-09-06: Added docs/UI_UX.md as the authoritative visual-style/UI-UX plan,
  kept deliberately separate from docs/ARCHITECTURE.md (looks/feels vs.
  runs/built). Key calls: fixed orthographic isometric camera with 90°
  snap-rotation (no free orbit), full 3D voxel models (not flat-textured
  cubes) at a 32×32×32-voxel tile scale, grounded semi-realistic industrial
  sauce-factory theme, light real-time shaders/lighting ("modded Minecraft
  with shaders" reference point), and a clean Minecraft-adjacent UI built
  around diegetic clipboard/checklist widgets.
- 2026-09-06: Researched and decided the voxel asset pipeline (docs/UI_UX.md
  §2a) — driven by the constraint that assets must be creatable/editable/
  viewable by Claude, not just a human artist. `.vox` (MagicaVoxel format) is
  the asset format, imported into Godot via a community `.vox` plugin. Each
  asset's actual source of truth is a small Rust generator function (not the
  binary `.vox` itself, which is a regenerated build artifact and must never
  be hand-edited); models are "viewed" by rendering an isometric PNG thumbnail
  headlessly via `vengi-voxconvert` and reading that image — no GUI/computer-use
  loop needed. Hand-sculpting in `vengi-voxedit` remains a documented escape
  hatch for one-off assets not worth parametrizing.
- 2026-09-06: Added docs/GAMEPLAY.md as the authoritative gameplay/progression/
  economy plan, kept separate from ARCHITECTURE.md (runs/built) and UI_UX.md
  (looks/feels). Key calls: sauce progression tiers (tomato sauce → ketchup →
  BBQ → equipment-gated categories like mayo/refrigeration and hot
  sauce/acid-handling → elaborate aged/fermented sauces); equipment purchases
  *are* the tech tree (no separate skill-point currency); money economy with
  three cost axes (ingredients, equipment, staff wages); and — the central
  design decision — sales happen via **contracts** (standing orders for a mix
  of sauces on a recurring delivery cadence) rather than a spot market,
  specifically so the winning strategy is a diversified factory rather than
  spamming whichever single sauce is most profitable. See that doc for full
  rationale, the tier table, and open questions (spot-market fallback,
  reputation system, staff-as-mechanic vs. abstract upkeep, spoilage/corrosion
  implementation) left for a future decision.
- 2026-09-06: Extended docs/GAMEPLAY.md with three more firm calls: (1) sauces
  can require other finished sauces as an ingredient (chilli mayo needs mayo +
  hot sauce), giving the sauce list a branching dependency tree — needs no new
  architecture, just the standard intermediate-product recipe pattern; (2)
  sauces come in flavour variants (garlic mayo, spicy BBQ, etc.) built as a
  base sauce plus a cheap flavour-finishing step, and contracts can name a
  specific variant; (3) staff are explicitly **not** simulated NPCs — they're
  an abstract resource pool with four categories (manufacturing, maintenance,
  packaging, logistics) mapped onto the four pipeline stages a sauce passes
  through, where running equipment consumes staff capacity the same way it
  would consume power (understaffing stalls machines, it doesn't simulate a
  missing person). Closest existing architectural analogue is `PowerNetwork`
  (aggregate demand vs. capacity, no per-entity simulation) — see
  docs/GAMEPLAY.md §7 and §9 for the proposed `StaffPools` shape.
- 2026-09-06: Added docs/README.md as a brief docs-folder index (table of the
  three docs + one-line pointer for each) so an agent or human can orient on
  `docs/` without reading everything — not itself authoritative/`@`-included
  (the three docs already are), just a map to them. Any new doc added later
  should get a row here too, per that file's own "Adding a new doc" note.
- 2026-09-06: Fixed stale cross-references between ARCHITECTURE.md/UI_UX.md
  (both predated GAMEPLAY.md and pointed at "separate design docs" vaguely
  instead of naming it), and added a Project Structure section to this file
  describing the current repo layout plus the layout implied by the docs for
  when implementation starts.
- 2026-09-06: Added docs/DEVLOG.md — a status tracker + dated history log,
  explicitly *not* `@`-included (unlike the three design docs) since it grows
  unboundedly; check it for status/history, don't expect it preloaded. Backed
  by a non-blocking `pre-commit` reminder hook (prints a note if a commit
  doesn't touch it, never blocks) plus the rule above this log: before
  finishing a commit, decide whether it's meaningful enough for a brief,
  high-level DEVLOG.md entry — most small commits aren't.
- 2026-09-06: Full cross-doc consistency pass. Found and fixed a real gap: the
  milestone list (ARCHITECTURE.md §9) never scheduled the economy/contracts
  system at all, even though GAMEPLAY.md §1 defines the core loop as requiring
  it — added milestone 5 "Economy MVP" (money + one hardcoded contract) right
  after "First full chain," renumbering the rest. Also flagged that money and
  in-flight contract state have no architectural home yet (GAMEPLAY.md §9,
  same treatment as the existing StaffPools/spoilage callouts), and that
  mobile/touch input is unaddressed in UI_UX.md despite mobile being a target
  platform (added as an open question there, to resolve before milestone 10,
  not at it). Also added `.editorconfig` and extended `.gitignore` for
  Rust/Godot/asset-preview artifacts the docs already commit to producing.
- 2026-09-06: Adopted Orca orchestration (`orca orchestration ...`) as how
  milestone-sized work actually gets built — ARCHITECTURE.md §9's milestones
  become a Run's Task DAG rather than one long accumulating session. See the
  Orchestration section above for the concrete conventions (worktree-per-
  foundational-milestone, gates at milestone boundaries, self-contained task
  specs). First use: milestones 1-5 (scaffold through Economy MVP) dispatched
  as a supervised DAG.
- 2026-09-06: Completed ARCHITECTURE.md §9 milestone 1 (Scaffold) and pinned
  the exact versions ARCHITECTURE.md §1a left as placeholders: Godot
  **4.6.3-stable**, `gdext` (crate `godot`) **0.5.5** — its latest release,
  and the newest one whose *default* API level still targets Godot 4.6 (0.5.4
  added an opt-in `api-4-7` feature that milestone 1 deliberately doesn't
  enable, to stay matched to the 4.6.x Godot pin) — Rust **edition 2024**
  (rustc 1.98.1 at pin time), and `hecs` **0.11.1** (recorded now though not
  yet a dependency; last published 2026-07-28, resolving §11's "re-verify
  maintenance activity" open question). Also decided: `cargo xtask` (not a
  justfile) is the ARCHITECTURE.md §7 build script, so the only required repo
  tool is `cargo` itself; and Godot's non-editor runtime won't discover a
  `.gdextension` file without a one-time `godot --import` pass that populates
  the gitignored `godot_project/.godot/` cache — `scripts/verify-scaffold.sh`
  automates and headlessly verifies this whole path (see its header comment
  for a known, benign Godot 4.6.3 editor-shutdown crash it works around).
- 2026-09-06: Completed ARCHITECTURE.md §9 milestone 3 (Core loop proof) —
  one belt, one machine, one recipe, real simulation code for the first
  time. Resolved §11's data-file-path open question: top-level `data/`
  (see the Project Structure section above). `sim_core/` gained a `sim/`
  module tree (types/recipe/inventory/belt/machine/world) that is pure Rust
  with zero `gdext` dependency; `lib.rs` is now only a thin JSON-over-`gdext`
  translation layer, per §4's own advice to keep that boundary's surface
  small. Two implementation decisions worth recording: (1) the belt's
  gap-based storage (§3.3) represents inter-item gaps as *slack beyond a
  fixed minimum item-spacing*, not raw distance — needed so a fully
  compressed line reads as "touching," not "overlapping," and so the
  front-item/rest-of-belt boundary can shift correctly when an item exits;
  see `sim_core/src/sim/belt.rs`'s module doc for the full reasoning. (2)
  the `SimBridge`↔GDScript wire format is plain JSON strings (via
  `serde_json`), not hand-built `Dictionary`/`Array` values — chosen
  specifically to minimize the `gdext`-specific API surface this codebase
  touches, which is exactly what §4 flags as the least agent-friendly,
  least training-data-backed layer in the whole stack. Recipes
  (`data/recipes.ron`) are embedded via `include_str!` at compile time
  rather than read from disk at runtime, so path resolution is identical
  under `cargo test`, the Godot editor, and an exported build; revisit if
  hot-reloading recipe content ever becomes valuable enough to justify the
  extra complexity. The interactive Godot scene (`main.tscn`/`main.gd`)
  uses genuinely flat placeholder textures (`godot_project/placeholder_art/`,
  its own README explains why) — not a UI_UX.md style decision, just this
  milestone's "prove it's rendered and interactable" bar; real voxel assets
  are UI_UX.md §2a's pipeline, not yet built.
- 2026-09-06: Completed ARCHITECTURE.md §9 milestone 2 (Mobile export spike)
  — see ARCHITECTURE.md §9a for the full writeup. Android export works
  end-to-end (a validly-signed APK with `sim_core` cross-compiled via
  `cargo-ndk` for arm64-v8a/x86_64, no source changes needed); actually
  installing/launching it on a device or emulator is the one thing left
  unverified, skipped this round because the dev host was too memory-
  constrained to safely boot an emulator alongside sibling sessions. iOS
  could not be attempted for real — the dev environment has no Xcode.app,
  only Command Line Tools, confirmed via both `xcodebuild`/`xcrun` and a
  Rust `aarch64-apple-ios` cross-compile that "succeeded" but linked against
  the wrong (macOS) SDK sysroot. Per this milestone's own instruction to
  "decide explicitly" if iOS looks broken: **iOS stays a stretch/deferred
  platform, not a milestone-10 launch commitment**, until this spike is
  re-run to completion on a Mac with full Xcode or Apple-hosted CI — this is
  a genuinely open question (ARCHITECTURE.md §11), not a resolved "no."
  Also recorded as a real, narrow upstream gap: Godot 4.6.3's Android export
  CLI can fail validation with a *completely blank* error message when
  `rendering/textures/vram_compression/import_etc2_astc` isn't enabled in
  `project.godot` (confirmed by reading Godot's own export-plugin source,
  not guessed) — now set in this project's `project.godot`.
- 2026-09-06: Completed ARCHITECTURE.md §9 milestone 4 (First full chain) —
  `sim_core::sim::World` now builds a straight-line, multi-stage production
  chain (one `BeltSegment`+`Machine` pair per recipe) instead of milestone
  3's fixed single-stage layout. The key design call: **chain topology is
  derived entirely from `data/recipes.ron`'s list order**
  (`RecipeRegistry::ordered()`), not described anywhere in Rust — `World`
  builds stage `i` from `ordered()[i]`, chains stage `i`'s machine output
  into stage `i + 1`'s belt, and panics at construction time
  (`validate_chain`) if consecutive recipes don't share an item id, the same
  way a compiler error would catch a Rust mistake. This is what makes
  "add a processing step" a pure data-file edit, and it was verified as
  part of this milestone, not just asserted: after the initial 3-stage chain
  (tomato -> tomato_sauce -> ketchup -> bottled_ketchup) was working and all
  tests passed, a 4th recipe (`boxed_ketchup`) was appended to
  `data/recipes.ron` ALONE — confirmed via a before/after SHA-256 hash
  comparison that zero `.rs`/`.gd` files changed — and the full headless
  verification suite (`cargo test -p sim_core` + the real `gdext` JSON
  boundary via `scripts/verify-core-loop.sh`) re-ran green against the
  4-stage chain with no code changes at all. `ChunkSnapshot` also changed
  shape from milestone 3: `belt`/`machine` (singular `Option`) became
  `belts`/`machines` (`Vec`), since a 32x32 chunk (ARCHITECTURE.md §3.1) can
  now hold more than one stage's entities — Godot's `main.gd` reads this
  plurally and also now calls a new `chain_layout_json` once at startup to
  build its `TileMap` sized to however many stages the data file currently
  defines, rather than hardcoding stage geometry client-side (keeping intact
  the "Godot draws what the sim says exists" principle from milestone 3).
  This milestone's chain is still a deliberate simplification: every recipe
  past the first has exactly one input/one output so `World`'s
  one-straight-line assumption holds; multi-ingredient convergence
  (branching topology) is explicitly deferred to a later milestone, per
  `world.rs`'s own module doc.
- 2026-09-06: Completed ARCHITECTURE.md §9 milestone 5 (Economy MVP) —
  resolves the money/contract-state architectural-home question this log's
  2026-09-06 "Full cross-doc consistency pass" entry flagged as open.
  New `sim_core::sim::economy` module: a fixed-point (`Fx`) `money` counter
  and a single hardcoded `Contract` (fixed mix/cadence/payout, named
  constants — `CONTRACT_ITEM`/`CONTRACT_QUANTITY`/`CONTRACT_CADENCE_TICKS`/
  `CONTRACT_PAYOUT_MONEY` — not `contracts.ron` data yet, per this
  milestone's own deliberately-minimal scope), both owned as plain fields on
  a new `Economy` struct rather than a new ECS component — there's exactly
  one money counter and one active contract at this scope, so ECS
  machinery wasn't warranted (mirrors how `World` already owns `stages` as
  a plain `Vec`, not an ECS resource). The delivery mechanism: milestone 4's
  `CollectOutput` command used to just discard collected product; it now
  deposits it into `Economy`'s `delivery_buffer` (a plain `Inventory`),
  which `Economy::tick` checks against the contract's mix once
  `current_tick` reaches the cadence boundary — on a satisfied mix it pays
  out and advances the boundary by the cadence; on a miss it still advances
  the boundary (no contract-loss/reputation penalty is implemented — that's
  GAMEPLAY.md §11's still-open question, not resolved here) rather than
  re-checking every tick forever. Exposed to Godot as
  `SimBridge::economy_snapshot_json`, read every frame by a new plain
  `Label` in `main.gd` (money + per-item contract progress + ticks to next
  check) — deliberately not UI_UX.md §5's clipboard/checklist styling yet,
  per this milestone's "simple label/counter is enough" scope. Verified two
  ways: a headless `cargo test` integration test
  (`sim_core/tests/economy.rs`) drives the *whole* milestone-4 chain
  (spawn -> travel every stage -> collect -> contract check) and asserts
  money increases by exactly one payout once the mix is delivered, and
  stays at zero through a cadence boundary with nothing delivered; and
  `test_core_loop.gd` now also round-trips `economy_snapshot_json` through
  the real `gdext`/Godot JSON boundary. `Economy` itself has its own
  `#[cfg(test)]` unit tests for cadence-boundary edge cases (a miss that's
  later satisfied, exact payout amount, mix-consumption on success).
  Explicitly deferred, matching the milestone's own scope note: the full
  `contracts.ron` schema, multiple concurrent contracts, and `StaffPools` —
  all unchanged future work per GAMEPLAY.md §7/§9.
- 2026-09-07: Visual-polish pass — built UI_UX.md §2a's voxel asset pipeline
  for real and replaced milestone 3/4's flat 2D placeholder scene with the
  isometric 3D one the doc describes. Presentation-only: `sim_core`,
  `SimBridge` and `data/` were not touched, and the scene still learns the
  chain's shape solely from `chain_layout_json` + chunk snapshots. Decisions
  worth not re-litigating: (1) **the `.vox` encoder is hand-written**
  (`assets/voxels/generators/vox.rs`) — all three crates UI_UX.md §9 named
  were re-checked and none can write a file the Godot importer accepts
  (`vox-format` is parser-only; `vox_writer` 0.1.9 writes no scene-graph
  chunks, which the importer starts from), so §2a's own documented fallback
  applies; see UI_UX.md §2b. (2) **The pinned importer is
  qbradq/Godot-MagicaVoxel-Importer-with-Extensions @ `288f1d8` (v1.2)**,
  vendored under `godot_project/addons/` — the other candidate the doc named
  now redirects to the same repo, so there was only ever one choice; treat a
  bump as a reviewed decision, same rule as `gdext`. (3) **An asset is
  emitted as one `.vox` per material class** (matte/metal/emissive) rather
  than one file, because an imported voxel mesh carries vertex colours but a
  single material, which cannot express UI_UX.md §4's "metals are
  deliberately shinier" rule; Godot rebuilds one `StandardMaterial3D` per
  class from `palette.ron`'s own numbers, exported as `materials.json`, so
  the palette stays the only place those values are written down.
  (4) **`palette.ron` owns material class, not just colour** — that is what
  makes (3) a data decision instead of a per-asset Godot decision.
  (5) **`cargo xtask` stages `assets/voxels/models/` into
  `godot_project/voxels/`** (gitignored), exactly the treatment
  `godot_project/bin/` already gets, because Godot can only load from inside
  `res://` while the committed source of truth lives outside it. (6) **The
  camera is true isometric (a `(1, 1, 1)` view direction) and fixed** — no
  orbit, per UI_UX.md §1; §9's "2:1 vs. Sims-style dimetric" question stays
  formally open, but it is now one constant in `main.gd` rather than nothing
  at all. Two environment gotchas are recorded in UI_UX.md §2b so nobody
  rediscovers them: Godot needs **two** `--import` passes on a cold cache
  (the .vox importer is an editor plugin, registered only on the pass after
  the first scan), and vengi's thumbnailer needs a real video driver
  (`SDL_VIDEODRIVER=dummy` makes it fail) even though it displays nothing.
  New verification: `godot_project/test_presentation.gd` (headless, wired
  into `scripts/verify-core-loop.sh`) asserts the structural half of the look
  — orthographic camera, shadow-casting lights, SSAO/glow, non-empty imported
  meshes, metal rougher-than-matte — and `scripts/screenshot-scene.sh`
  renders the actual scene to a PNG, which is the only non-headless check in
  the project and deliberately so.
