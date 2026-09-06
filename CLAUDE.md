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

Nothing beyond docs exists yet (pre-alpha, milestone 1 of ARCHITECTURE.md §9
hasn't started). Current layout:

```
sauce-factory/
  CLAUDE.md            # this file — always loaded, points at the three docs below
  CONTRIBUTING.md       # commit message convention
  README.md              # short public-facing pitch
  docs/
    README.md              # docs-folder index — read this first when only skimming
    ARCHITECTURE.md          # how the game runs and is built
    UI_UX.md                  # how the game looks and behaves
    GAMEPLAY.md                # what the game actually is to play
    DEVLOG.md                   # status tracker + dated log — not @-included, check/update separately
  scripts/               # repo tooling (e.g. commit-msg lint)
  .githooks/              # git hooks (commit-msg, pre-commit) — enable via `git config core.hooksPath .githooks`
  .github/workflows/       # CI (commit-message lint)
```

As implementation starts, lay the repo out to match what the docs already
commit to — don't improvise a different shape:

- A Rust workspace for the simulation core (ARCHITECTURE.md §1/§7), holding
  the ECS/`hecs`+`rayon` sim, the `SimBridge`/`gdext` interop layer, and the
  voxel-asset generator code (UI_UX.md §2a) — likely `sim_core/` and a
  `tools`/`xtask`-style crate for the build/asset pipeline, per
  ARCHITECTURE.md §7's checked-in build script.
- A Godot 4 project for presentation (ARCHITECTURE.md §1/§5) — GDScript, the
  `.gdextension` file from §7, and the `.vox` import plugin from UI_UX.md §2a.
- `assets/voxels/{palette.ron, generators/, models/, previews/}` per
  UI_UX.md §2a's proposed layout — `generators/` and `palette.ron` are source
  (hand-written, committed), `models/` is a regenerated build artifact (never
  hand-edited), `previews/` is gitignored and only for visual review.
- Data files (`items.ron`, `recipes.ron`, `machines.ron`, and eventually
  `contracts.ron` per GAMEPLAY.md §9) live wherever the Godot/Rust boundary
  expects data files to live — not yet pinned to a specific path; decide this
  at milestone 1 and record it here once it exists.
- Per ARCHITECTURE.md §10: once `sim_core/` and the Godot project exist, each
  gets its own `CLAUDE.md` documenting invariants specific to that module —
  don't let this root file grow to cover implementation-level detail that
  belongs closer to the code.

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
