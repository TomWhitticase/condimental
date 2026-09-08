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
simulation code — a multi-stage production chain (raw tomato -> tomato paste
-> ketchup mix -> ketchup -> bottled ketchup -> boxed ketchup, run by a
crusher, a mixer, a 2x2 cooking vat, a bottler and a packer) built entirely
from `data/recipes.ron`, now with a money counter and a single hardcoded contract
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
                               # adding a step is a pure edit here.
    machines.ron             # machine kinds: each `Recipe.machine` id -> a tile
                               # footprint (width_tiles, height_tiles). Loaded by
                               # sim/machines.rs, which errors if a recipe names a
                               # kind that isn't here or a footprint is degenerate.
                               # Per-kind staff/power draw and build cost belong
                               # here too when those land — not a second file.
                               # items.ron lands here when it exists.
    shop.ron                 # what the player can BUY and what it costs — one
                               # entry per buildable kind (every machines.ron kind,
                               # incl. the no-recipe hopper/fluid_tank/dock buildings,
                               # PLUS the belt and pipe tiles, which are
                               # infrastructure, not machines), each with a whole-
                               # number price in the same money economy.rs counts.
                               # Loaded by sim/shop.rs, which errors on a bad schema
                               # version, a duplicate kind, a price below 1, or a
                               # machine kind it doesn't price. Prices are
                               # placeholders (GAMEPLAY.md §10).
    contracts.ron            # the contract board (GAMEPLAY.md §6/§9): id, name,
                               # item-mix, cadence in ticks, payout. Loaded by
                               # sim/contracts.rs, which rejects a bad schema
                               # version or a degenerate contract. List order is
                               # priority when two contracts compete for stock.
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
      machines.rs                      # MachineRegistry — loads + validates
                                         # data/machines.ron; owns the footprint
                                         # origin convention (min-X/min-Y corner,
                                         # = the tile the belt feeds into)
      shop.rs                          # ShopCatalog (loads/validates data/shop.ron) +
                                         # OwnedEquipment (per-kind owned counts) +
                                         # Shop, which pairs them. Money is NOT here —
                                         # it stays on Economy, since GAMEPLAY.md §1
                                         # allows exactly one currency
      contracts.rs                     # ContractRegistry — loads + validates
                                         # data/contracts.ron; owns the Contract type
      dock.rs                          # Dock — the chain's exit building: takes
                                         # finished product off the belt past the
                                         # last machine and ships it into Economy's
                                         # shared delivery pool EVERY TICK (no player
                                         # command; it replaced CollectOutput). A
                                         # pass-through by design — its own buffer is
                                         # empty between ticks, so "what's on the
                                         # dock" is EconomySnapshot::delivery_stock
      economy.rs                       # Economy: money (Fx) + a Vec<ActiveContract>,
                                         # each with its OWN cadence clock, all drawing
                                         # from ONE shared delivery_buffer Inventory —
                                         # a delivered unit is consumed by the contract
                                         # that takes it, never credited to two
                                         # (GAMEPLAY.md §6)
      world.rs                         # hecs::World wrapper; builds a multi-stage
                                         # production chain (one belt+machine per
                                         # recipe, in recipes.ron order); owns Economy
                                         # and ticks it once per World::tick; SimBridge's
                                         # pure-logic half — tick()/dirty_chunks()/
                                         # chunk_snapshot()/issue_command()/
                                         # query_tile()/chain_layout()/economy_snapshot() (§4)
    tests/core_loop.rs        # headless integration test: full-chain N-tick
                                # conservation invariant across every stage (§6)
    tests/machine_footprints.rs # headless integration test: footprint-aware chain
                                 # layout — asserts the shipped 4-stage chain's tiles
                                 # are byte-identical to the pre-footprint formula,
                                 # and drives test-only 2-/3-wide machines for the
                                 # width math, multi-tile query_tile and the
                                 # chunk-boundary case
    tests/economy.rs           # headless integration test: full chain collects
                                 # against the hardcoded contract and pays out
                                 # correctly (money increases on a satisfied mix,
                                 # stays put on a miss) — economy.rs's own #[cfg(test)]
                                 # covers Economy in isolation (cadence/mix edge cases)
    tests/dock.rs              # headless integration test for the delivery dock:
                                 # its geometry past the last machine, the chain
                                 # shipping and EARNING with no player command at
                                 # all, the dock reading empty while the shared pool
                                 # fills, and dock_snapshot_json's exact shape —
                                 # dock.rs's own #[cfg(test)] covers Dock in
                                 # isolation (intake gating, the full-dock refusal)
    tests/shop.rs              # headless integration test: the chain earns contract
                                 # money and spends it on equipment, a refused
                                 # purchase (unaffordable/unknown/zero-quantity) is a
                                 # true no-op, and shop_snapshot_json's exact shape —
                                 # shop.rs's and world.rs's own #[cfg(test)] cover the
                                 # loader's error cases and the Purchase arm's edges
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
    test_machines.gd                # headless proof of the "Machines improvement"
                                      # presentation: a bespoke model per machine kind, a
                                      # multi-tile machine covering its whole footprint, the
                                      # hopper drawn with a live product level, and hover
                                      # resolving a NON-ORIGIN footprint tile
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
at the repo root. `items.ron` belongs here too when it's added.

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

- 2026-09-08: **The chain has a real end: a delivery dock** (GAMEPLAY.md
  §6/§9's long-flagged "a real delivery-dock tile is future polish"; ticket 1
  of 2, the sim half — `godot_project/` deliberately untouched). Finished
  product used to leave the world on a keypress: `PlayerCommand::CollectOutput`
  reached into the LAST machine's own `output` buffer and deposited the lot
  into `Economy`'s shared delivery pool, with no tile downstream of the packer
  existing at all. Now `data/machines.ron` has a `dock` kind (1x1, priced at
  70 in `data/shop.ron`), `World` lays one more ordinary `BeltSegment` past
  the last machine and stands the dock at its end, and
  `sim_core::sim::dock::Dock` absorbs from that belt and calls
  `Economy::deposit` every tick, automatically. Six calls worth not
  re-litigating. (1) **The dock is a pass-through, not a store**, and that
  was the ticket's real decision: `Dock::ship_into` empties the buffer in the
  same tick it filled, so the dock's own level always reads zero between
  ticks and `DockSnapshot` deliberately has NO `fill` field. The alternative
  — hold a visible stack and release it on some schedule — would create two
  different numbers both honestly called "what's on the dock," and the one
  the player could see (the dock tile) would be the one contracts *cannot*
  count, since they settle against `Economy`'s pool. That is the same
  gauge-lies-about-the-mechanism failure `FluidTank` rejected when it chose
  constant head over its own fill fraction. It also has nothing gating it:
  contracts already meter the pool out at their own cadences, so a hold would
  be latency with no decision attached. It becomes a real mechanic the day a
  truck cadence or GAMEPLAY.md §7's logistics staff gates the release — one
  line changes then. Presentation draws the pallet stack from
  `EconomySnapshot::delivery_stock`. (2) **`CollectOutput` is REMOVED, not
  kept alongside** — a breaking wire-format change:
  `{"type": "collect_output"}` no longer deserializes, so `command_from_json`
  returns `None` and `issue_command_json` treats it as the silent no-op any
  malformed command gets (a stale caller does not crash, it just stops doing
  anything). `main.gd`'s **C** keybind and the `"C: collect product"`
  fragment of `ISO_HELP_TEXT`/`FP_HELP_TEXT` are ticket 2's to delete. This
  is the same discipline ticket 3 followed when `FillHopper` replaced
  `SpawnItem`: do the rewrite, don't accumulate two ways to do one thing.
  (3) **`DOCK_CAPACITY` is 32 per item and is deliberately unreachable
  today** — the drain empties the dock every tick and the intake takes at
  most one unit per tick, so it never holds more than one. The bound exists
  because `try_absorb` has to be able to say *no*: that is what makes a full
  dock back the belt up instead of annihilating the box, which is the
  property `tests/core_loop.rs`'s mass-conservation invariant actually
  checks, and it is unit-tested with a capacity-1 dock rather than left to a
  world that cannot reach it. (4) **The dock holds a multi-item `Inventory`,
  not the hopper's single fixed item id.** A hopper's item is decided by
  `recipes.ron` (a chain has one raw ingredient); a dock accepts whatever the
  line delivers, which costs nothing today and is the honest model of the
  building. (5) **It is placed EAST, so unlike the hopper and the fluid tank
  it genuinely moves the world's boundaries.** Both of those were placed
  outside the chain's span specifically so introducing them moved no tile;
  the dock is the chain's tail, so it extends it — the shipped line now runs
  x = 0..35 and spans three chunks ((-1, 0), (0, 0), (1, 0)) instead of two.
  That is handled (footprints are reported in every chunk they overlap,
  `all_chunks` lists all three) and now pinned by tests, and it spends the
  one-tile chunk headroom `data/machines.ron`'s `aging_rack` note was
  protecting — that note has been updated rather than left stale. (6) **The
  exit hop reuses `try_transfer_machine_output_to_belt` verbatim**, and the
  dock's belt is an ordinary belt entity with the same `Position`/`BeltSpan`/
  `BeltSegment` components: the chain's exit is now the same conveyor
  hand-off as every hop before it, which is the whole reason a building beats
  a command. New wire surface for ticket 2: `dock_snapshot_json`, a `docks`
  array on `chunk_snapshot_json`, and a `"dock"` variant from
  `query_tile_json`.

- 2026-09-08: "Contracts as data" epic, ticket 2 of 2 — **the clipboard
  shows every active contract**, one docket card each. Presentation only:
  `sim_core` and `data/` were untouched, and every fact drawn was already on
  the wire in ticket 1's `economy_snapshot_json`. Four calls worth not
  re-litigating. (1) **One board, several dockets — not a clipboard per
  contract, and not one flat checklist.** A PanelContainer per contract
  repeats the paper frame and the 12px steel clip bar for no new
  information; a single flat list of checklist rows loses which rows belong
  to which order the moment two contracts ask for the same item, which every
  shipped contract does (they all want `boxed_ketchup`). So the panel keeps
  its one sheet and one clip, and a contract is a printed docket on it — the
  checkbox's own paper tone plus a steel hairline, i.e. two colours the panel
  already used, so separation was added without a new visual language.
  (2) **Money is shown once, everything else per docket.** GAMEPLAY.md §1
  allows exactly one currency and `Economy` keeps exactly one counter, so a
  per-contract money figure would be a lie; cadence countdown, payout and
  the delivered/missed tally are per contract, because each clocks
  independently in `Economy`. (3) **The shared delivery pool gets one line of
  its own ("on the dock").** Per-contract `progress` is clamped to that
  contract's own ask, so three dockets can all read satisfied while the pool
  holds fewer boxes than they collectively want — without the unclamped pool
  figure beside them that reads as three imminent payouts rather than the
  competition ticket 1's allocation rule actually creates. It is a line of
  text, not a second progress widget: surfacing the rule is worth it,
  inventing a parallel visual language for it is not. (4) **Overflow is a
  capped, scrolling stack, chosen over a compact/summary mode.** GAMEPLAY.md
  names no contract count; `contracts.ron` ships three and a realistic board
  is a handful, so the cap (a fraction of window height) never engages today
  and exists so a content change cannot push an obligation off the bottom of
  the screen unseen. Node rebuilds are guarded on the set of contract ids, so
  the per-frame poll costs a string compare, not a rebuild. Also fixed, and
  only findable in a rendered frame (UI_UX.md §2b point 12's lesson again):
  the checklist tick mark had been body-text sized inside a 14px box since
  the clipboard first landed, rendering as a clipped diagonal in dark ink on
  dark green. **This completes the contracts epic.**

- 2026-09-08: ARCHITECTURE.md §9 milestone 7 ("Timed processes"), ticket 2 —
  **the aging rack's presentation, and the project's first live progress
  readout on a machine.** Ticket 1 put a 2400-tick `aging_rack` into the
  chain; this draws it. `sim_core`, `SimBridge` and `data/` were not touched.
  Five calls worth not re-litigating. (1) **The rack is the one machine on
  the line with no mechanism, deliberately.** Every other kind is identified
  by something that moves — flywheels, an agitator, a gantry, a taping head —
  and the rack is identified by the absence of one, because what makes it
  different from its neighbours is that it only *waits*. Four oak casks
  standing on a stainless frame; standing rather than lying in a cradle
  because a cask on its side recedes into the tile as a ridge under this
  camera and four of them read as corrugation (UI_UX.md §2b point 15).
  (2) **Its casks are oak, which is UI_UX.md §2b point 8's "food contact is
  stainless" rule's one deliberate exception** — in a barrel-aging room the
  wood *is* the process, and GAMEPLAY.md §2's tier 6 names "barrel-aged"
  outright. Argued once at `palette.ron`'s `oak` entry so a later
  consistency pass doesn't "fix" it to steel. (3) **A long process gets a
  passive gauge; a short one does not.** `main.gd`'s `MACHINE_PROGRESS_GAUGES`
  is keyed on machine kind and holds exactly one entry: a bar tracking the
  crusher's 60-tick cycle would be a flicker, and five flickering bars along
  a line is noise. The gauge itself is the FOURTH use of the
  one-voxel-slab-scaled-every-frame mechanism (`pipe_fluid`, `hopper_fill`,
  `fluid_tank_fill`), not a fifth mechanism. (4) **The bar is recomputed from
  `current_tick()` every frame, not from a snapshot.** A machine dirties its
  chunk only when its state changes, so nothing at all arrives during a
  forty-second wait — a snapshot-driven bar would sit still and then jump.
  This is the one place in the presentation layer where polling is not a
  convenience but the only correct option, and `test_timed_processes.gd`
  asserts it by ticking with no chunks applied. (5) **The gauge emits its
  own light, and shows its scale when empty.** Both came off the rendered
  scene: the face it sits on is permanently unlit under this camera + key
  light, so a painted gauge is a dark stripe on a dark column; and a hidden
  bar leaves a black slot with no indication of how far it has to climb, so
  an always-lit green finish line marks the top (a position, not a state in
  colour — UI_UX.md §7). Also: the hover tooltip now prints a remaining TIME
  beside its percentage, because a percentage of a duration the player was
  never told is not an answer at this timescale. **This completes milestone
  7.** One thing fixed in passing that is worth knowing, though since
  superseded: this ticket's branch predated the contracts epic's ticket 2
  above, so `main.gd`'s contract panel still read the pre-multi-contract
  `next_delivery_tick` field at the time, and the resulting GDScript error
  **aborted `_process` at that line every frame**, silently stopping
  fluid/hopper/hover/camera updates in the live scene. A narrow
  tolerate-both-shapes patch fixed that live-blocking bug on this branch;
  rebasing onto main afterward picked up the real N-contract clipboard
  rebuild from the entry above instead, which is what actually shipped —
  the narrow patch itself did not survive the rebase.

- 2026-09-08: **Contracts are data, and several run at once** (GAMEPLAY.md
  §6/§9; ticket 1 of 2, the sim half — `godot_project/` deliberately
  untouched). Milestone 5's Economy MVP had exactly one `Contract`,
  hardcoded in Rust from four named constants; the design pressure
  GAMEPLAY.md §6 argues contracts exist *for* therefore did not exist, since
  with one contract there is nothing to diversify between. Now: a new
  `data/contracts.ron` (schema_version 1) with three seeded contracts, a new
  `sim_core::sim::contracts` module loading/validating it the way
  `recipe.rs`/`machines.rs` already do, and `Economy` holding a
  `Vec<ActiveContract>` each with its own cadence clock. Five calls worth not
  re-litigating. (1) **The delivery-allocation rule, which GAMEPLAY.md had
  left open: ONE shared pool of collected stock, and a settling contract
  CONSUMES its mix from it — goods are never credited to two contracts at
  once.** The alternative (credit every contract that wants the item)
  duplicates goods, so stacking contracts onto one product line would double
  income for free — the exact "build only the best sauce" strategy §6 rejects
  a spot market to prevent. Contracts competing for one line's throughput is
  the point, not a side effect. (2) **`contracts.ron` file order is the
  scarcity tiebreak**, stated in the file itself: when two contracts fall due
  on the same tick and stock covers one, the earlier-listed wins. A
  player-set priority is the natural upgrade and is the same open question
  GAMEPLAY.md §11 already has for staff; file order is the honest placeholder
  rather than an invented answer. (3) **No `penalty` field yet.** GAMEPLAY.md
  §9 lists penalty terms as part of this file's eventual shape, but §11 has
  not decided what a miss costs — a field nothing reads would answer that
  question in data by accident. A miss costs that window's payout, increments
  a `deliveries_missed` counter, and nothing else. (4) **The
  contracts-vs-recipes cross-check does NOT panic at `World` construction**,
  unlike `machines.rs`'s `validate_covers`. A missing machine kind has no
  footprint, so the layout would be silently wrong; an unsatisfiable contract
  leaves the world coherent and merely never pays — which is the legitimate
  state a deliberately-out-of-reach contract will be in once §6's tier gating
  lands. It is a shipped-files-against-each-other check run from
  `contracts.rs`'s own test instead. (5) **`economy_snapshot_json` is a
  breaking shape change**: the four top-level contract fields
  (`contract_mix`/`contract_progress`/`cadence_ticks`/`next_delivery_tick`)
  are replaced by a `contracts` array plus a `delivery_stock` pool; `money`
  is unchanged. Per-contract `progress` is clamped to the ask (a checklist
  reads 2/2, never 7/2); the unclamped pool is `delivery_stock`. Every
  shipped contract targets `boxed_ketchup` and that is a content limit, not a
  shape limit — the multi-item mix is implemented and tested, but the chain
  makes one sellable product and the chain has exactly one exit (the
  delivery dock, added later the same day, which only ever receives that
  line's final product), so any other target would be permanently
  unsatisfiable. Real mix diversity
  needs a second sauce line.

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
  `CollectOutput` command used to just discard collected product; it then
  deposited it into `Economy`'s `delivery_buffer` (a plain `Inventory`) —
  and that command has since been replaced entirely by the delivery dock,
  see the 2026-09-08 entry at the top of this log —
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
- 2026-09-07: "Machines improvement" epic, ticket 1 — **a machine's tile
  footprint is data, in a new `data/machines.ron`**, and `World`'s chain
  layout accumulates each stage's own machine width instead of multiplying a
  fixed `STAGE_SPAN_TILES` constant. Three calls worth not re-litigating.
  (1) **The footprint origin is its minimum-X/minimum-Y corner, which is
  also the tile the stage's belt feeds into** — a machine grows east/north
  from there, so widening a kind never moves its input side and only pushes
  downstream stages along. Stated once in `sim_core/src/sim/machines.rs`'s
  module doc; every other layer (layout, `query_tile`, snapshots, and
  eventually the renderer) refers to that rather than restating it.
  (2) **A footprint straddling a 32x32 chunk boundary is handled, not
  rejected**: the machine is reported in *every* chunk it overlaps, with the
  same origin tile each time, so a renderer loading only visible chunks
  still learns about a machine whose origin is off-screen — the cost being
  that a consumer stitching chunks together must de-duplicate by `tile`.
  (3) **`World::new` keeps its one-argument signature** and loads the
  bundled `machines.ron` itself (the `include_str!` lives in `sim/`, not
  `lib.rs`, unlike `recipes.ron`), with `World::with_machines` as the seam
  tests use to exercise a footprint no shipped kind has — every shipped kind
  is 1x1, deliberately, so this ticket moved zero existing tiles.
- 2026-09-07: "Machines improvement" epic, ticket 2 — **the chain's
  abstract two-step front end is now three real machines**: a `crusher`
  (tomato -> tomato_paste, 60 ticks), a `mixer` (tomato_paste + 2 vinegar ->
  ketchup_mix, 90 ticks) and a `cooking_vat` (ketchup_mix -> ketchup, 240
  ticks), replacing the old `cooker`/`reducer` pair and their `tomato_sauce`
  intermediate. Pure content: no `sim_core` logic changed, the chain is still
  one straight line, and the two downstream stages (bottler, packer) are
  untouched. Four calls worth not re-litigating. (1) **The cooking vat is
  2x2** — the first shipped kind bigger than one tile, and deliberately the
  *only* one: a crusher and an in-line blender are small machines, a batch vat
  running the chain's longest process is not, so size tracks what the machine
  actually does. 2x2 rather than 2x1 because it is the smallest footprint that
  exercises ticket 1's mechanism in both axes (width shifts every downstream
  stage east by one tile; height claims a second row) — anything bigger would
  dwarf the belts feeding it. (2) **Process times track the work**: crushing
  is the quickest step, the cook is 4x it, so the line has a real bottleneck
  and the biggest machine is visibly the slow one. (3) **The fluid
  requirement moved from the cooked stage to the mixing stage**, which is
  where a real line adds vinegar; this needed no Rust change at all, because
  `World` builds its pipe run for "whichever recipe declares `fluid_inputs`"
  — moving the declaration re-plumbed the run, which is the data-driven
  property doing exactly what it promised. (4) **Neither `schema_version` was
  bumped** (recipes stays v2, machines stays v1): both files' *shapes* are
  unchanged — only their content is — and ARCHITECTURE.md §4's versioning
  rule is about a changed schema, not changed content. Bumping on content
  would make the version meaningless as a compatibility signal.
- 2026-09-07: Milestone 6 (Fluids), presentation half. `sim_core` was not
  touched: the pipe run, the liquid in it, and the pipe hover tooltip are all
  driven from the JSON ticket 1 already exposed — geometry from
  `fluid_snapshot_json`'s own `segments` list (not `chain_layout_json`, which
  describes belt/machine stages only, and not `chunk_snapshot_json`, whose
  `pipes` array only arrives for *dirty* chunks — and fluid movement
  deliberately doesn't dirty its chunk, so a chunk-driven renderer would wait
  forever). Three decisions worth not re-litigating: (1) **`palette.ron`
  gained a fourth material class, `liquid`** (glossy dielectric) rather than
  reusing `matte`, because UI_UX.md §4's "visible liquid surfaces ... with
  subtle shading/reflection" is a material property and §2a point 5 already
  makes material class palette data; only the per-*fluid* tint is decided in
  GDScript, since which liquid is in a pipe is live simulation data.
  (2) **The pipe run is stainless steel, not PVC** — see UI_UX.md §2b point 6
  for the §4/GAMEPLAY.md-tier-5 reasoning. (3) **The fluid model is one voxel
  tall and scaled to the live fill fraction**, which works exactly because the
  encoder puts a model's origin at its footprint centre at ground level. Also
  fixed a real hole in `scripts/verify-core-loop.sh` found while adding
  `test_fluids.gd`: **Godot exits 0 when a `--script` file fails to parse**,
  so the suite printed PASS for a test that never loaded — every headless
  Godot test now has to print its own success marker, checked by the runner.
- 2026-09-07: "Machines improvement" epic, ticket 4 (last of four) — **the
  presentation half**, on top of tickets 1-3. `sim_core`, `SimBridge` and
  `data/` were not touched: every fact this ticket draws was already on the
  JSON wire. Five calls worth not re-litigating. (1) **Which model a machine
  gets is keyed on its `kind`, with the generic `machine` model as the
  fallback** (`main.gd`'s `MACHINE_MODELS`) — so the three front-end kinds
  have real silhouettes, the two downstream ones still don't, and adding a
  kind to `data/machines.ron` never requires a Godot edit before the chain
  will draw. (2) **A machine's node sits at its footprint's CENTRE**
  (`_footprint_centre`), because the voxel encoder puts a model's origin at
  its own footprint centre at ground level — which for every 1x1 kind is
  exactly the old per-tile placement, so generalizing it moved nothing, and
  the 2x2 cooking vat is modelled at 64x64 voxels, i.e. UI_UX.md §2's
  32-per-tile scale times its declared footprint. A kind's model size and its
  `machines.ron` footprint are therefore coupled on purpose: size is content.
  (3) **The hopper's product level reuses the fluid body's mechanism** — one
  voxel tall, scaled to a live fraction every frame, hidden below a minimum —
  rather than inventing a second way to show "how full is this"; the only new
  thing is `palette.ron`'s `bulk_pale`, near-white so a per-item tint can
  multiply over it (which item a hopper holds is data), and class `matte`
  rather than `liquid`, because a heap of tomatoes is not a wet surface.
  (4) **Hover grew a third branch, not a third mechanism**:
  `_machine_recipe_at_tile` became footprint-aware so any tile of a
  multi-tile machine resolves, and `_hover_text_at_tile` gained the hopper —
  the same shape milestone 6's pipes were added in. (5) **The material rule
  for machine assets is "food contact is stainless, structure is painted or
  poured"** — recorded in UI_UX.md §2b point 8 so it isn't re-argued per
  asset. Also fixed a real, silent bug only the rendered scene could find:
  `ITEM_MODELS` still named `tomato_sauce`, an item ticket 2 deleted, so two
  stages of the chain had been rendering cardboard boxes instead of bulk
  product. Its lesson is UI_UX.md §2b point 10. **This ticket completes the
  "Machines improvement" epic.**
- 2026-09-07: "Visual overhaul round 2" epic, ticket 1 — **the pipe run's
  fluid supply is a player-loadable `FluidTank`, not an always-refilling
  off-map tap**. Milestone 6 flagged one `PipeSegment` `is_source` and
  refilled it to capacity every tick; the project owner's live-demo review
  asked for "a fluid equivalent of a hopper for the vinegar," and this is it.
  Simulation + `data/machines.ron` only (plus one line each in two headless
  `.gd` test scripts, to issue the new fill command — no rendering change).
  Four calls worth not re-litigating. (1) **The tank is a placed building,
  not a pipe segment**: it has a footprint in `data/machines.ron` like the
  hopper, and is deliberately kept out of `FluidNetwork::segments` so the
  run's snapshot geometry did not change shape at all. What it shares with a
  segment is the *flow rule*, not the type — `FluidNetwork::tick` runs one
  `equalize` function over `tank <-> segments[0]` and then every adjacent
  pipe pair, so there is one flow mechanism in the file rather than a second
  bookkeeping path for "the tank drains." (2) **It presents constant full
  head, not its own fill fraction.** It is a raised vessel discharging
  downhill; equalizing on fill fraction would have a 120-unit tank feeding a
  40-unit run keep three quarters of every remaining unit inside itself
  forever, so the player would watch the mixer starve against a gauge reading
  a quarter full. A *constant* maximum also makes flow strictly one-way with
  no special case — `equalize` already clamps a transfer to what the donor
  has, so an empty tank moves nothing; letting the value fall to zero when
  empty inverts the difference and has the empty tank siphoning the pipes.
  (3) **`PipeSegment::is_source` kept its name and lost its magic**: it now
  marks only "the tile the tank connects to" (presentation keys a distinct
  silhouette off it), the segment starts empty like every other, and it
  drains normally — so `fluid_snapshot_json`'s shape is unchanged, but its
  `total_volume` can now legitimately read zero. (4) **Capacity is 120 units,
  a Rust constant**, sized against the line the way `HOPPER_CAPACITY`'s 64
  was — roughly three and a half full hoppers of tomatoes per tank fill, so
  vinegar is the slower-burning of the two consumables. It moves to
  `machines.ron` when buffer sizes become per-kind content, together with the
  hopper's, not before. Placement follows the hopper's precedent exactly:
  one tile west of the run's first pipe tile, outside all existing geometry,
  so introducing it moved no tile.
- 2026-09-08: "Visual overhaul round 2" epic, ticket 3 (last of three) —
  **belt/pipe-to-machine junctions**. Presentation-only; `sim_core` gained a
  doc-comment clarification and nothing else, and `data/` was untouched.
  The project owner's live-demo review: *"a conveyor belt going into a mixer
  seems to just go straight into the side there is no connecting so it looks
  like items just disappear into the wall"* — the previous pass's flush
  footings had closed a floor-level crease but a flush wall is still a wall.
  Five calls worth not re-litigating. (1) **The camera decides the design,
  and it is asymmetric.** With a fixed `(1, 1, 1)` view direction the only
  vertical faces ever visible are the +X and +Z-world ones, so a machine's
  outlet face is camera-facing and its inlet face can *never* be seen —
  cutting an inlet doorway, the ticket's literal instruction, would have
  been invisible. Outlets are therefore real doorways carved through the +X
  wall; inlets are hooded transfer housings whose *camera-facing side* is cut
  away instead. Full derivation in UI_UX.md §2b point 11. (2) **An opening
  has to be genuinely empty, and that is what the new `cargo test -p voxgen`
  suite asserts** — a mouth painted dark meshes as a flat rectangle and
  renders as a decal, a distinction no screenshot at scene scale can show,
  so it is checked voxel by voxel in `assets.rs`'s own tests rather than left
  to the eye. (3) **The belt gained two junction-adjacent variants**
  (`belt_inlet`/`belt_outlet`), chosen per tile from `chain_layout_json`
  alone: whether the neighbour along that stage's *derived* travel direction
  is covered by a machine footprint (or by the hopper, which discharges the
  same way). This is the buildable half of "assets should look different
  depending on their orientation and what's around them" — what a tile is
  *next to* is on the wire; which way it *points* still is not, so the
  variants are +X-only and `_belt_asset_for` falls back to the plain model
  otherwise. The corner limitation in `world.rs`'s module doc is unchanged
  and was re-stated, not weakened. (4) **The 2x2 cooking vat needed the
  standard piece widened, not a bespoke one**: its belt row is its far row,
  so its own shell occludes a belt-width inlet completely — the fix is a
  charging gallery across its whole -X face, which is the same helper called
  with a wider lane and is also what a vat that size really has. (5) **The
  simulation's per-frame tick catch-up is now clamped** (`MAX_TICKS_PER_FRAME`):
  it was unbounded, so one slow frame silently fast-forwarded the factory and
  two identical screenshot runs photographed visibly different states, which
  makes a rendered scene useless as a before/after check. Two runs are now
  byte-identical. Also fixed while in there: the pipe run's downstream end was
  an open bore (it "just stopped"), and `pipe_source`'s inlet flange had been
  silently plugging the bore the tank discharges through.

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

- 2026-09-08: "Shop & Inventory" epic, ticket 1 (of 2) — **equipment is
  something the player buys, and `data/shop.ron` is where its price lives.**
  The first half of a directly-requested priority shift from a fixed
  always-built line toward a real build-your-own-factory game (ticket 2 is
  the Godot build menu; a following epic is real placement/rotation/pickup).
  Rust + data only. Five calls worth not re-litigating. (1) **A new data
  file, not a field on `machines.ron`.** `machines.ron`'s own header does
  invite "build cost ... belongs in this file too" — but a shop entry is not
  a machine property: the belt and the pipe are priced and are not machines
  at all, and `machines.ron` is loaded to answer "how big is that thing,"
  which every world needs, whereas a price is only needed by the economy. So
  `shop.ron` is the catalog of *buildable kinds*, a superset of machine
  kinds, and `ShopCatalog::validate_covers` makes an unpriced machine kind an
  error rather than a thing the player can never buy. (2) **There is exactly
  one money counter, and it stays on `Economy`.** GAMEPLAY.md §1 is explicit
  that no second currency or tech-point resource exists; a shop wallet would
  be one by the back door. `Economy::try_spend` is the only way money leaves,
  and it is all-or-nothing, so a purchase cannot half-apply. (3) **Owned
  counts are a plain `Shop` field on `World`**, beside `economy` and `fluid`,
  for their reason: one per world, so ECS machinery buys nothing. They are a
  `BTreeMap`, not `Inventory` — an unbounded ledger of *kind* ids is not a
  capacity-bounded store of *item* ids, and conflating the two would let one
  be passed where the other is meant. (4) **`Purchase` is a silent no-op on
  failure**, like every other command in the enum (`FillHopper` past
  capacity, `FillFluidTank` in a world with no pipe run): an unknown kind, a zero
  quantity, a total `Fx` can't represent, or simply not enough money all
  leave the world byte-identical. It also dirties no chunk, because money and
  a count are not tiles — presentation polls `shop_snapshot_json`, as it
  already polls `economy_snapshot_json`. (5) **This ticket changes nothing
  about the factory.** Every machine still exists unconditionally from tick
  0; buying credits a number and nothing else. That is deliberate — the
  economy layer and the placement rewiring are kept apart so this half is
  testable in isolation and moved no existing tile, and the whole
  pre-existing suite passes untouched.
