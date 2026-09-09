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
    ingredients.ron          # what a unit of each RAW INGREDIENT costs
                               # (GAMEPLAY.md §5's recurring cost axis) — a
                               # separate file from shop.ron because it prices a
                               # different kind of thing: stock a machine eats,
                               # not a building you place and can pick back up.
                               # Loaded by sim/ingredients.rs, which errors on a
                               # bad schema version, a duplicate id or a price
                               # below 1. An id here is "raw": consumed by some
                               # recipe, produced by none — solids and fluids
                               # alike, in one catalog and one ledger.
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
      types.rs                   # Fx (fixed-point, §3.9), ItemId/RecipeId/TileCoord/
                                   # ChunkId, and Direction (four-way; East = +X is the
                                   # facing that reproduces every pre-placement layout)
      recipe.rs                   # RecipeRegistry — loads + validates data/recipes.ron
      inventory.rs                  # generic slot-based Inventory (§3.2)
      belt.rs                        # BeltSegment — gap-based item storage (§3.3)
      machine.rs                      # Machine + TimedProcess (§3.2/§3.5)
      machines.rs                      # MachineRegistry — loads + validates
                                         # data/machines.ron; owns the footprint
                                         # origin convention (min-X/min-Y corner,
                                         # = the tile the belt feeds into) AND the
                                         # rotation convention (a quarter turn swaps
                                         # the extents around that same corner; the
                                         # edge on the facing side is the outlet, the
                                         # opposite edge the inlet, the flanks neither)
      ingredients.rs                   # IngredientCatalog (loads/validates
                                         # data/ingredients.ron) + OwnedIngredients
                                         # (per-id stock) + Ingredients, which pairs
                                         # them — shop.rs's shape with one word
                                         # changed. Also owns `raw_ingredient_ids`,
                                         # the crate's single definition of "raw"
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
      world.rs                         # hecs::World of PLACED things (the "Real
                                         # Placement" epic): World::new builds NOTHING —
                                         # every belt tile, pipe tile, machine and
                                         # building is a Place command spending owned
                                         # inventory, and Remove/Rotate undo and turn it.
                                         # Flow is derived from adjacency and recomputed
                                         # only on Place/Remove/Rotate (ARCH §3.8), never
                                         # per tick. Owns Economy/Shop; SimBridge's
                                         # pure-logic half — tick()/dirty_chunks()/
                                         # chunk_snapshot()/issue_command()/query_tile()/
                                         # world_layout()/economy_snapshot() (§4)
      save.rs                          # the save format (ARCH §4, amended): SaveData/
                                         # SaveError + to_ron/from_ron + World::to_save/
                                         # from_save. A full state SNAPSHOT, not a command
                                         # log; fixed-point values as raw bits; a load
                                         # rebuilds structure through real Purchase/Place
                                         # commands and writes live state on top through
                                         # restore-only methods (pub(crate)/pub(super),
                                         # never player-facing). NO filesystem code —
                                         # `user://` is Godot's, so Rust only ever handles
                                         # the RON string
      reference.rs                     # sim::reference_chain — the pre-placement shipped
                                         # layout, rebuilt out of nothing but Purchase +
                                         # Place commands. Both the fixture every
                                         # integration test builds its world from AND the
                                         # proof placement reaches every configuration
                                         # the fixed chain used to guarantee
    tests/core_loop.rs        # headless integration test: full-chain N-tick
                                # conservation invariant across every stage (§6),
                                # against a world built by reference_chain
    tests/placement.rs        # headless integration test for the "Real Placement"
                                # epic: an empty new world, Place/Remove/Rotate
                                # round-trips and refusals, collision on both Place
                                # and Rotate, conservation through a player-assembled
                                # line that TURNS A CORNER and one that runs west,
                                # several hoppers/docks/pipe networks, and the
                                # place/remove/rotate JSON wire form
    tests/machine_footprints.rs # headless integration test: footprint-aware chain
                                 # layout — asserts the shipped 4-stage chain's tiles
                                 # are byte-identical to the pre-footprint formula,
                                 # and drives test-only 2-/3-wide machines for the
                                 # width math, multi-tile query_tile and the
                                 # chunk-boundary case
    tests/fluid_production.rs  # headless integration test for the chain rework:
                                 # a machine DEPOSITING fluid into a pipe, which
                                 # network it fills vs. draws from (outlet face vs.
                                 # touching), the pipe-loop refusal, and
                                 # validate_chain's fluid-link half
    tests/ingredients.rs       # headless integration test: raw ingredients are
                                 # bought, a refused purchase is a no-op, an
                                 # unstocked factory produces nothing, and a
                                 # delivered unit is worth more than what made it
    tests/common/mod.rs        # shared fixture for the tests that RUN the chain
                                 # for a long time (buy stock, keep both hoppers
                                 # and the tank fed)
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
    tests/save_load.rs         # headless integration test for save/load: a mid-flight
                                 # shipped chain saved, loaded into a NEW world and ticked
                                 # forward in LOCKSTEP for longer than a contract cadence
                                 # with every snapshot compared every tick; plus byte-exact
                                 # idempotence, an empty world, a forked line with turned
                                 # placements, and unplaced inventory. save.rs's own
                                 # #[cfg(test)] covers every loader refusal
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
    main.tscn / main.gd          # interactive 3D scene, rebuilt in code from
                                   # world_layout_json (re-read after every place/remove/
                                   # rotate) + chunk snapshots, with a real BUILD MODE —
                                   # ghost, rotation, removal: voxel meshes,
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
    test_splitters.gd               # headless proof of the conveyor splitter's presentation
                                      # (Epic B ticket 2): a bespoke model at the sim's tile,
                                      # the belt feeding it still drawing the PLAIN tile, the
                                      # next-output arrow along the facing and one quarter
                                      # turn clockwise for the branch, an unbuilt branch
                                      # falling back to straight, a dead end hiding the
                                      # arrow, and items drawn crossing the tile AND down a
                                      # branch that leaves the splitter's own chunk
    test_placement.gd               # headless proof of the "Real Placement" epic's ticket 2:
                                      # an EMPTY starting world, the ghost and its
                                      # footprint-aware validity, all four facings drawn with
                                      # the matching rotation, the junction art's east-only
                                      # fallback, and place/remove driving the scene
    test_hotbar.gd                   # headless proof of the inventory hotbar: an always-
                                       # visible strip built from shop_snapshot_json, number
                                       # keys 1-9/0 selecting a slot into placement mode with
                                       # NO menu open, the same key again deselecting, empty
                                       # sockets, and buying still a separate verb
    test_save_load.gd                # headless proof of save/load across the real boundary:
                                       # a whole RON save crossing as a String both ways, an
                                       # earlier moment coming back exactly, a refused load
                                       # saying WHY and leaving the world untouched, a loaded
                                       # world reporting every chunk dirty, and new_game()
    test_layouts.gd                  # shared fixture: factories built through real purchase/
                                       # place commands. Most are small BY CHOICE (a nine-tile
                                       # line proves a wire format as well as a forty-tile
                                       # one); `build_reference_line` builds the whole shipped
                                       # six-stage chain, which a 3000 starting balance affords
    test_chain_rework.gd            # headless proof of the chain rework's presentation
                                      # (Epic A ticket 2), replacing test_timed_processes.gd
                                      # which went with the aging rack: the rack gone from
                                      # every mapping, a hopper placed WITH ITS CONTENTS
                                      # CHOSEN (and the same click refused without them), Q
                                      # cycling them, the ingredient market buying crates and
                                      # the larder following, the 1x2 mixer covering both
                                      # tiles and hovering from either, and the two
                                      # machine-to-machine pipe runs carrying the right fluid
                                      # on the PLAIN tile while the tank-fed run keeps the
                                      # branch fitting
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

- 2026-09-09: **Save/load, ticket 1 of 2 (the sim + `SimBridge` half;
  `godot_project/main.gd`, `main.tscn`, `assets/voxels/` and `data/`
  deliberately untouched apart from one new headless test script) — a world
  can be written to text and put back running.** Until this ticket there was
  no save code anywhere: `main.gd` built a brand-new empty `World` on every
  launch, so nothing a player did survived closing the window. Eight calls
  worth not re-litigating.

  (1) **A save is a full state SNAPSHOT, not ARCHITECTURE.md §4's command
  log — and §4 is amended to say so.** That paragraph proposed "a command
  log plus periodic full-state snapshots," to share one mechanism with
  undo/redo and with §8's lockstep multiplayer replay. Neither of those
  exists: no undo/redo is implemented and multiplayer is an explicit stretch
  milestone, so a replay engine would have been built to answer "put my
  factory back" — a problem this project does not have. It is also the more
  fragile of the two *here*: a command log is replayed through whatever game
  logic the **current** build has, so changing what a command does silently
  rewrites every existing save, where a snapshot of the resulting state
  cannot drift that way. The doc now records the snapshot as shipped and
  keeps the command log as the right shape for undo/redo and replay **if**
  those land — a snapshot does not preclude one, it is the known start state
  a command log needs.

  (2) **The bar is running-state-identical, not layout-identical, and it is
  pinned by a lockstep test rather than by a list of fields.** A machine
  mid-craft must not restart its batch; an item 37% across a belt tile comes
  back at 37%; every pipe tile, tank, hopper, splitter cursor, dock buffer
  and lifetime shipped total, every contract's own cadence clock and
  delivered/missed counters, the shared delivery pool, money, owned
  equipment, owned stock and the tick count all survive.
  `sim_core/tests/save_load.rs` builds the shipped chain, runs it into a
  genuinely messy mid-flight state (asserting it *reached* one, so the
  fixture cannot quietly stop exercising what it claims), saves, loads into a
  different world, and then compares **every snapshot the boundary exposes,
  at every tick, for 2600 ticks** — longer than the longest contract cadence,
  so a restored clock has to settle a real window after the load. A
  divergence surfaces on the tick it happens rather than as "it looked wrong
  later".

  (3) **Fixed-point values are stored as raw bits (`FxBits`), never as
  decimal text.** `Fx` is `I16F16`; a decimal print-and-reparse can land a
  fraction of a bit away, which is precisely the divergence (2)'s lockstep
  comparison would find a thousand ticks later. `fixed`'s own serde feature
  is deliberately not enabled: raw bits is the promise this format wants to
  make, and it should not be a property of a dependency's formatting
  choices.

  (4) **A load rebuilds STRUCTURE through the real commands and writes LIVE
  STATE on top through restore-only methods.** Half one is `Purchase` then
  `Place` per placement, exactly what `sim::reference_chain` already does —
  so nothing about footprints, collision, hopper contents or topology is
  re-implemented in the loader, and a load cannot build a world a player
  could not have built. A refused `Place` is a **corrupt save** (an `Err`
  naming the kind and the tile), never a silently missing tile. Half two
  writes what no command can express — craft progress, belt item positions,
  a dock's lifetime total, an exact balance — through methods that are
  `pub(crate)` on the body types and `pub(super)` on `World` (world.rs's
  "save/load seam" block). **That visibility is the rule, not an accident:**
  the player-facing API must not gain a way to set a machine's progress just
  because a loader needs one.

  (5) **Money is written twice, and the rebuild is funded one item at a
  time.** Each rebuild purchase spends real money, so the saved balance is
  installed *last* and the rebuild's spending cannot leak into it. Funding
  per item rather than pricing the whole bill up front (which is what
  `reference.rs` does) is not fussiness: `Fx` saturates around 32 767, so a
  single up-front bill for a few thousand belt tiles would not fit in the
  money counter even though every individual price does — a factory that
  saved fine would refuse to load.

  (6) **Two pieces of "internal" state turned out to be genuinely
  behavioural and are saved: a belt's `frontier`, and every inventory's
  EMPTY slots.** `frontier` is `belt.rs`'s scan bound, and it is what
  enforces the module's own "freed space ripples back by at most one item
  per tick" property — a belt restored with a wider one would let a
  compressed run advance further in its first tick than the belt it came
  from. Empty inventory slots matter because slot order is observable: it is
  the order `MachineSnapshot`'s `input`/`output` arrays come out in, and a
  slot emptied by a craft keeps its place so the next unit of that item
  lands back where it was. Dropping zeros would reload a machine that
  reports its buffers in a *different* order the moment the missing
  ingredient arrives again — running-state-nearly-identical, which is not
  the bar. Everything genuinely derived stays out of the file: recipes (a
  machine's kind names its recipe), every capacity (a Rust constant), pipe
  network shape/fluid id/source flags (re-derived by
  `recompute_topology`, the same pass a `Place` triggers), and the dirty set.

  (7) **File I/O is not in `sim_core`, and `load_ron` returns a MESSAGE
  rather than a bool.** Rust produces and consumes the RON *string*;
  resolving `user://saves/<slot>.ron` is Godot's, because `user://` is
  Godot's — so the crate gains no filesystem code at all and ticket 2 owns
  slots, autosave and the menu. `SimBridge::load_ron` returns `""` on
  success and a human-readable reason on failure ("save schema_version 2 is
  not supported by this build (supports 1)"), which is the DEVLOG 2026-09-09
  "SPACE and V say why, instead of doing nothing" lesson applied to the one
  control that can fail for reasons a player cannot see. On failure the live
  world is **untouched** — a load builds a whole new world and swaps it in
  only once every step succeeded — and on success it reports **every** chunk
  dirty, so a chunk-driven renderer redraws all of it. `new_game()` is there
  so a menu's New Game and its Load are the same shape (both replace the
  world under one long-lived bridge); constructing a fresh `SimBridge` would
  do the same job, so it is a convenience, not a capability.

  (8) **A changed contract board is an error in BOTH directions.** A save
  naming a contract this build does not have is rejected; so is a build
  having a contract the save says nothing about. Quietly starting the new
  one from scratch would hide the fact that the save's economy state cannot
  be faithfully restored, and the same reasoning covers an unknown kind, an
  item no recipe mentions any more, and a hopper holding something that is
  not raw — content changed under the save, and this crate's whole loader
  convention is to say so rather than to guess. **Flagged, not fixed:**
  `schema_version` is **1** and stays 1 for the rest of this batch — the
  auto-fill ticket adds an `auto_fill` flag to hoppers and tanks, which the
  per-kind body records are named-field structs precisely to make a one-line
  `#[serde(default)]` addition. Nothing has shipped with a save file yet, so
  v1 is whatever this batch merges with; after that, a field addition is a
  version bump like any other shape change.

- 2026-09-09: **A tuning pass on three directly-requested things after a
  positive playtest — the crusher's process time, the delivery dock's
  footprint, and the size of the whole UI.** The three are independent; what
  they share is that every one of them came off someone actually playing the
  game rather than off a design doc. Seven calls worth not re-litigating.

  (1) **The crusher takes 80 ticks, not 60, and the ORDERING of the three
  front-end process times is the load-bearing claim.** At 60 the crush read
  as instant next to the mixer's 90 and the vat's 240 — a stage the player
  never sees working is a stage that does not look like it is doing
  anything. 80 is deliberately still under the mixer, because
  `data/recipes.ron`'s header, `data/machines.ron`'s cooking-vat note and
  this log's own 2026-09-07 "Machines improvement ticket 2" entry all state
  in words that crushing is the quickest of the three and the cook the
  slowest; a number that inverted that would make three sentences false
  in three files. So the rule is written down where the numbers are:
  **change the numbers and the words together or not at all** (the cook is
  3x the crush now, not 4x, and every file that said 4x says 3x). Keeping
  the crusher faster than the stage it feeds also keeps the line's
  bottleneck at the vat, where it was designed to be. `recipes.ron`'s
  `schema_version` did NOT move: content changed, shape did not, which is
  the rule the 2026-09-07 entry already recorded.

  (2) **One economy test was calibrated on the old timing rather than on its
  own claim, and that is worth knowing about the suite generally.**
  `one_delivered_unit_pays_one_contract...` said "ship exactly one unit,
  then stop feeding the line entirely" — but the line is still full of
  in-flight product when the first box lands, so it goes on shipping for a
  long while afterwards, and whether a *second* box beat the settle boundary
  turned out to be a function of every process time in `data/recipes.ron`.
  It passed at 60 by luck. It now **removes the dock** after the first unit
  ships, through an ordinary player command, which makes "exactly one unit
  is ever collected" true by construction. The general lesson: a fixture
  that stops *supplying* a factory has not stopped the factory.

  (3) **The delivery dock is 2x2, and the extra tile is spent on DEPTH.**
  The playtest was blunt — it did not read as a loading dock — and it did
  not: every piece that makes a loading bay legible (a raised deck a trailer
  backs up to, a striped edge, bumpers, a despatch door under a canopy, and
  clear apron for the goods waiting to go out) was competing for one
  32-voxel tile, which is also why the stack of cases on its deck had to be
  capped at two. **2x2 rather than 2x1** because UI_UX.md §2b point 16's
  rule for this camera is "height at high y, content at low y": a dock twice
  as wide and still one row deep would leave the despatch structure and the
  goods fighting for the same row, buying width the layout does not need and
  none of the depth it does. The far tile is the building now, the near tile
  is open apron. Two things fall out that are not decoration — its inlet
  face is **two tiles long**, so two lines can deliver into one dock (the
  same rule that made the mixer 1x2, arriving at the other end of the line),
  and a **square** footprint claims the same four tiles whichever way it is
  turned, which matters most for the building at the END of a line where a
  player turning it to meet a belt from another direction should not
  discover the tile behind it is no longer free. The 1x1 argument that
  survives is the one about the BUFFER ("a bay, not a warehouse", 32 units,
  a pass-through anyway) — buffer size was simply the wrong thing to size a
  footprint by.

  (4) **Its price did not move with its footprint.** 70, unchanged, and
  stated in `data/shop.ron` rather than left to be re-derived: price there
  tracks what a building DOES for the player, not how many tiles it claims.
  Only the infrastructure group is priced per tile, and the 2x2 cooking vat
  costs 200 for the process it runs rather than for its four tiles; a dock
  does no work at all and is still exactly one door out of the factory
  however much apron stands round it. The extra tiles are already a real
  cost in the currency a factory-builder actually feels, which is floor
  space. Keeping it also leaves `sim_core/tests/shop.rs`'s starting-money-
  against-the-bill relationship undisturbed.

  (5) **The model was re-laid at 64x64, not stretched, and the despatch wall
  had to stop short of the west corner.** Two transfer housings (one per
  inlet tile, because a model with one would describe a mechanism narrower
  than the simulation's), two despatch openings under one continuous canopy,
  a bay light over each, and expansion joints scored along the tile
  boundaries so a 64-wide slab does not read as one blank table. The
  non-obvious bit, found by a failing voxel test rather than by eye: the FAR
  tile's infeed hood occupies the same corner the wall wanted, and a wall
  drawn through it is a building with a belt hood carved out of its end and
  daylight showing through the hole — so the wall starts east of that
  corner, which is also what a real bay looks like (the conveyor comes in at
  the end of the building, not through the middle). The deck's waiting stock
  is a **2x2 block of four** pallet positions now rather than a row of two:
  the cap existed because four cases buried a one-tile building, and that
  reason is gone.

  (6) **The UI is half again as big, and the mechanism is explicit constants
  rather than Godot's `content_scale_factor`.** The report was that the UI
  is "much" too small and "difficult to read"; the body font had been 15px
  since milestone 5's plain money label and roughly two dozen pixel literals
  had accumulated around it. `main.gd` now has exactly two numbers —
  `UI_SCALE` (1.5) and `UI_FONT_BASE` (15) — and four helpers
  (`_ui_px`/`_ui_i`/`_ui_v`/`_ui_font`) that every font size, padding,
  border, separation, icon square and slot size goes through. The engine's
  own scale knob is one line and was rejected for a specific reason: this
  scene maps 3D to screen coordinates in half a dozen places — the hover
  tooltip at the mouse, the ghost prompt through `unproject_position`, the
  hotbar tooltip over its slot, right-drag pan deltas — and every one of
  them would then need the factor divided back out, in exchange for nothing
  the constants do not already give. Scaling the numbers also keeps the
  headless tests that assert pixel sizes meaningful.

  (7) **Half again does not fit a 1280x720 window unchanged, and what gave
  way was layout rather than type size.** The build menu is six columns of
  two rows with a shorter card (its hotkey digit moved into the button's
  corner, the way the hotbar socket's already was, and the "right-click to
  hold" affordance moved to the footer that already said it — in the card it
  wrapped onto a second row, which cost the whole grid a row it does not
  have). All three panels put their heading and their balance on one line.
  The control legend is four grouped lines instead of two run-on ones,
  because at this size the old first line ran off the window and under the
  clipboard. The clipboard's docket stack and the build menu's grid now cap
  themselves against the space **actually left above the hotbar**, measured
  once in `_hotbar_reserved_height()` so no two panels can disagree about
  where the usable screen ends — replacing the clipboard's old fixed
  fraction of window height, which at this scale reached down across the
  larder. **Flagged, and a deliberate last resort:** at 1280x720 the
  build-menu grid scrolls, by one part of a row; at the 1600x900
  `scripts/screenshot-scene.sh` reviews at, and anything taller, nothing
  scrolls at all. Of the two ways to lose that fight, a grid that scrolls a
  little is much better than a menu whose footer disappears under the
  hotbar, which is the exact failure `_position_shop_menu` was written to
  prevent. **At merge the coordinator moved `project.godot`'s default window
  from 1280x720 to 1600x900** for exactly that reason: a rendered 720-tall
  frame showed the grid's second row cut mid-card, which reads as clipping
  however it is documented, and 1600x900 is the size every review frame in
  this project has been judged at anyway. It fits a 16" MacBook's
  1728x1117-logical display with the title bar. Scrolling stays as the
  fallback for a smaller window. One more thing that would not fit: the **hotbar socket takes
  0.88 of the scale**, not all of it, because twelve of them plus
  separations plus the plate's margins have to fit a 1280-wide window —
  84px, a third bigger than the 64 it was. That is the same headroom the
  `SHOP_HOTKEY_SLOTS` note already flags as something a longer catalog will
  have to solve properly.

- 2026-09-08: **The chain rework, ticket 2 of 2 (the Godot/assets half;
  `sim_core/src/sim/` touched in exactly one place, see (1)) — the piped
  middle, drawn, and the hopper finally placeable.** Ticket 1 left `main`
  failing its own headless suite AND, less visibly, **unplayable from a cold
  start**: `main.gd`'s `_confirm_build` sent a `place` command with no `item`
  field, which a world with two raw solids refuses silently, so clicking to
  place a hopper did nothing at all and said nothing. Seven calls worth not
  re-litigating.

  (1) **A hopper's contents are a property of the PLACEMENT, cycled with Q —
  not a second catalog entry.** The obvious alternative was to list
  "Hopper — Tomato" and "Hopper — Sugar" as separate build-menu/hotbar slots,
  and it fails on the hotbar's own recorded contract: fixed slots in
  `data/shop.ron` order, so slot 5 is still the crusher after the last
  crusher is placed. One entry per raw ingredient renumbers every slot after
  the hopper the day a third ingredient ships — the exact property fixed
  slots exist to protect — and it would put two menu entries in front of one
  `hopper` kind with one owned count, so both would print the same "Owned N"
  and neither would be about that entry. So the hopper keeps one slot, one
  price and one owned count; **Q** cycles what the ghost in hand will be
  loaded with, the placement prompt names it in the kind's own name
  ("✔ PLACE Hopper — Sugar"), and the ghost carries a `hopper_fill` slab
  tinted with the item so the choice is visible as well as written (UI_UX.md
  §7 — the word is the primary signal, the colour reinforces it). Putting the
  decision on the ghost also puts it where the player is already looking
  rather than back in a modal grid. **The one sim change** is what makes it
  generalize: `IngredientsSnapshot` gained `hopper_items`, which is
  `World::raw_items` verbatim — the same set `Place` validates against —
  because `ingredients_snapshot_json`'s `entries` list prices fluids too and
  an id's price says nothing about whether it is a solid. A third raw
  ingredient therefore appears in the market, in the larder and in the Q
  cycle from `data/` alone.

  (2) **Buying stock is its own panel on M, and the reason is height before
  it is taxonomy.** The build menu is twelve catalog slots and was already
  re-centred above the hotbar to stop it burying its own footer; a second
  section inside it would push it off a 720-line viewport. It is also an
  honest split — `shop.ron` sells buildings you place and can pick back up,
  `ingredients.ron` sells stock a machine eats and never returns, which is
  the same argument those are two files for. Everything about the panel
  mirrors the build menu deliberately (same plate, same slot styling, same
  icons, money repeated at the top) so it reads as the other half of one shop
  rather than a different screen — which is what `ingredients_snapshot_json`
  being field-for-field `shop_snapshot_json` was for. **One click buys a
  crate of ten**, not one: the crusher eats four tomatoes a craft and a
  hopper holds sixty-four, so buying by ones would be sixteen clicks to fill
  one bin. One gesture, one meaning, printed on the slot.

  (3) **A permanent larder strip above the hotbar, because two controls now
  fail silently.** `SPACE` and `V` load a hopper and a tank *from bought
  stock*, and with none they do nothing and say nothing — so the number that
  explains why is on screen permanently, in the one place the player already
  reads for "what am I carrying". An ingredient at zero reads **"none"** in
  words rather than a grey `0`, per §7. It is a readout, not a control; it
  names the key that buys.

  (4) **The mixer is a horizontal ribbon blender now, and the footprint is
  why.** Its recipe gained sugar, so `data/machines.ron` made it 1x2, and the
  model had to become 32x64 — UI_UX.md §2's tile scale times the declared
  footprint, the coupling `machine_cooking_vat` already documents. Keeping
  the old upright drum and stretching the plinth under it fails twice: half
  the machine becomes bare deck, and the vessel sits across exactly the two
  -X bands where the two feed chutes have to come in. A trough lying along
  +Y is *made of* the extra tile instead of tolerating it, is what a real
  plant folds a dry ingredient into a wet one with, and reads unmistakably as
  two tiles deep. The agitator stays visible through a short crown window —
  `pipe_body`'s sight-glass trick, and for its reason: the first cut ran the
  window two thirds of the drum and the rendered factory showed ribbon
  flights along nearly the whole vessel, which at scene scale reads as
  corrugation rather than as a drum you can see into.

  (5) **A fluid junction is ONE piece where a belt junction needed two**, and
  that falls out of the camera rule UI_UX.md §2b point 11 established. A belt
  opening is a hole, so it is invisible on the -X face and needs a hooded
  housing cut open on the camera-facing side; a pipe junction is a **solid
  protruding nozzle**, which reads from +X dead-on and from -X by the
  silhouette it breaks out of the machine's skirt. So `machine_fluid_port` is
  one helper used on both faces, bright steel at radius 3 with a brass flange
  sitting exactly on the tile boundary — the same fitting `pipe_inlet`'s
  branch already carries, because that pairing is what was found to read as
  one spool piece crossing a boundary. Nothing was needed on the pipe's side:
  a plain `pipe` tile's bore is already open at both X faces at
  `PIPE_AXIS_Z`. **The vat's charging gallery and discharge doorway are
  deleted rather than left as scenery** — a covered belt-width mouth with no
  conveyor in front of it is a fitting describing a mechanism the machine
  does not have, which is the failure the fluid tank's constant head and the
  dock's absent fill gauge both refused.

  (6) **`pipe_inlet` stays authored for one arrangement, and the other falls
  back to the plain tile.** Its T-off reaches out of the tile's south edge
  and its blind flange caps the +X end, which is right for a run lying north
  of the machine it feeds (the tank-fed vinegar run) and wrong for a
  machine-to-machine run lying west-to-east — there the branch would point at
  bare floor and the blind flange would cap the very end the machine draws
  through. So `_rebuild_world` now applies that asset only when the tapping
  machine is actually south-adjacent. This is the same known, tested,
  accepted fallback the belt's east-only junction variants take (UI_UX.md §2b
  point 18), and a four-directional pipe-fitting pass is the same art ticket.

  (7) **Sugar needed a model of its own, and that is UI_UX.md §2b point 12
  for the third time.** `ITEM_MODELS` falls back to a shipping box, so a new
  item id with no entry ships bulk produce down the belt in sealed cardboard
  with every structural test passing. `item_sugar` is a slumped paper sack
  with a folded top and crystal spilling out of it — a *sack* rather than a
  heap, because at half-tile scale on a moving belt an item is read by
  silhouette alone and a pile of white grains is a shapeless blob beside
  `item_tomato`'s round red one. **Flagged, not fixed:** `ITEM_MODELS` still
  maps `ketchup_mix` and `ketchup`, which are fluids now and therefore
  unreachable; the entries are kept deliberately, since an unreachable
  correct mapping costs nothing and a missing one costs a rendered frame. One
  thing added in passing: `scripts/screenshot-scene.sh --no-hud`, because the
  contract clipboard and the hotbar between them cover the top-right and the
  bottom of a closeup, which is where a junction lands about half the time —
  reviewing "does this pipe meet this machine" kept coming down to whether
  the subject happened to miss a panel.

- 2026-09-08: **Epic B, ticket 2 of 2 (the Godot/assets half; `sim_core/` and
  `data/` deliberately untouched) — the fork, drawn.** Ticket 1 left `main`
  failing its own headless suite: `test_machines.gd` asserts every kind
  priced in `data/shop.ron` resolves to a bespoke model and the splitter had
  none. Closing that was the floor; six calls worth not re-litigating.

  (1) **The splitter is drawn as the one MACHINED TABLE in a run of rubber,
  and its model lives with the belts.** A bright stainless diverter plate,
  three open lanes (intake, straight, branch) and one closed rail, at exactly
  the belt's own running height with the belt's own frames and rollers under
  it — so it reads as "the line forks here" from across the factory without
  reading as another machine with a housing on top. That is what a splitter
  IS in this simulation (it is priced with the infrastructure, it is 1x1, and
  it carries product in an ordinary one-tile `BeltSegment` at belt speed), so
  the generator sits in `assets.rs`'s conveyor section, not with the machines.
  The `machine_` prefix on the asset name is only because `main.gd`'s
  `MACHINE_MODELS` is keyed on `data/machines.ron` kinds, which a splitter has
  a footprint in like every other placed thing.

  (2) **The next-output readout is an arrow INLAID FLUSH IN THE DECK, not a
  gate, an arm or a lamp.** `SplitterSnapshot::next_output` is the one piece
  of a splitter's state a player cannot infer from the tiles around it, so it
  needs a passive readout — and this tile has items drawn sliding across it,
  so anything standing proud of the table gets driven through by the product
  it describes. Both output directions are therefore cut into the table as
  grooves and a one-voxel emissive slab fills whichever one is live: the fifth
  use of the "one flat slab, positioned by `main.gd` from live data" mechanism
  `pipe_fluid`, `hopper_fill`, `fluid_tank_fill` and `aging_rack_gauge`
  already share, and the first that carries a DIRECTION rather than a level.
  Cutting both grooves whether or not either is lit is what satisfies UI_UX.md
  §7: the readout is **which of two positions is glowing**, and both positions
  are visible shapes on the deck with nothing lit at all.

  (3) **One arrow asset, turned by the simulation's own quarter turn.** The
  model is authored pointing straight and the branch is shown by rotating the
  node 90° clockwise on top of the placement's own facing — which is literally
  how `sim::splitter` derives the second output (`Direction::rotate_cw`). A
  separately-authored branch arrow could end up pointing somewhere the
  simulation does not; a rotation of the first cannot. The two grooves in the
  base model are generated the same way, from one `arrow_cells` function
  rotated, for the same reason.

  (4) **The arrow shows where the item will ACTUALLY go, which is not always
  `next_output`.** The raw field is which side gets first refusal, and
  `splitter.rs`'s fallback rule sends a refused item down the other side in
  the same tick — so a splitter whose branch was never built reports
  `"branch"` for ever while everything goes straight through it. An arrow
  driven off that field alone would point at bare floor while product sailed
  past in the other direction: the same gauge-lies-about-its-mechanism failure
  the fluid tank's constant head and the dock's absent fill gauge both
  refused. So the one case the wire can settle is settled in
  `_apply_splitter_state` — a due side whose `outputs` entry is `null` hands to
  the other, and with neither connected the arrow is hidden outright. What it
  deliberately cannot show is a side that is built but momentarily FULL, which
  also falls back: that is transient and invisible on the wire, where an
  unbuilt branch is durable and is the mistake a player wants told about. The
  hover tooltip names both output tiles and says "not connected" in words.

  (5) **A splitter is CONVEYOR to `_belt_asset_for`, not a building.**
  `_is_building_tile` decides whether a belt tile draws the
  hand-over-to-a-machine variant, and it was "anything that is not a belt or a
  pipe" — which would have put `belt_inlet`'s dead-plate and guide kerbs in
  front of what is actually more conveyor. One kind added to that test.

  (6) **Two rendering bugs came off the rendered frame, and only one of them
  is about splitters** (UI_UX.md §2b point 12's lesson yet again). Items were
  hidden per RENDER PASS: `_render_snapshot` collected every key it had drawn
  and hid every pooled node missing from that set — but it runs once per DIRTY
  CHUNK, so with two chunks dirty the second pass wiped what the first had
  just drawn. It was invisible while every test factory fitted in one chunk
  and the shipped chain spans three; a splitter branching a line across a
  chunk boundary is what finally showed it, with eight tomatoes on the branch
  belt in the simulation and none on screen. Emptied slots are hidden per TILE
  now, from the tile's own item count, which needs no cross-chunk bookkeeping
  at all. And `screenshot-scene.sh --populate` was batching thousands of ticks
  before reading `dirty_chunk_ids()` **once** — but `World::tick` clears its
  dirty set at the start of every tick, so the flag that exists to photograph
  a running factory was photographing whatever the last tick happened to
  touch. It applies every tick now. **Flagged, not fixed:** the catalog is
  twelve kinds against ten number keys, so `packer` and `dock` are keyless;
  `SHOP_HOTKEY_SLOTS` was written for exactly that and degrades correctly
  (no digit printed, no keycode resolving to them, still buyable and placeable
  from the build menu), which `test_hotbar.gd` now asserts rather than assumes.
  A second key scheme, if anyone wants one, is its own ticket.

- 2026-09-08: **Epic B, ticket 1 of 2 (the sim half; `godot_project/` and
  `assets/voxels/` deliberately untouched) — the conveyor splitter, and with
  it the first one-to-many hand-off in the simulation.** Every link
  `World::recompute_topology` had built until now was one-to-one: a belt
  looks at the single tile its own `Direction` points at. A splitter looks
  at two, which is what lets one crusher feed two mixers, or a line be
  tapped without being cut. Five calls worth not re-litigating.

  (1) **The second output is derived from the placement's one `direction`,
  not stored.** `direction` is the **straight-through** output — a splitter
  placed facing the way the belts around it already run carries product
  exactly where a belt tile would have — and the branch is one quarter turn
  **clockwise** (`Direction::rotate_cw`, the same turn `PlayerCommand::Rotate`
  applies). Three things follow, and they are why this beats a second stored
  facing: a splitter drops into an existing line without the player working
  out which of two facings is "the main one"; one `rotate` turns **both**
  outputs together, for free, with no second field that could drift out of
  step with the first and no new command; and the four facings reach all four
  L-shapes, so nothing is unreachable (a player who wants the branch on the
  other side faces the splitter the other way and lets the straight run take
  the corner). What a splitter *accepts* is deliberately not face-gated at
  all — it is belt-like, so anything that hands to a belt hands to it, which
  is the 1x1 rule this codebase already has (a 1x1 placement's inlet face is
  its only tile, which is why a dock and a hopper already behave this way).

  (2) **The distribution rule is round-robin with fallback**, i.e. Factorio's
  default splitter. The due side gets first refusal; if it cannot take the
  item the other side is offered it **in the same tick**; whichever side
  actually takes it, the other becomes due. The fallback is what makes it a
  splitter rather than a metering valve — a splitter that insisted on its
  turn would stall a whole line on one full buffer, which is a throughput bug
  the player cannot see. Advancing the cursor on *whichever* side took it
  (rather than only on the side that was due) is what makes fairness resume
  by itself: while one branch is blocked, every item goes down the open one
  and the blocked one stays permanently due, so the first item after it
  clears is its. No catch-up counter, no starvation. **Both degenerate
  wirings fall out of that loop rather than being special-cased**: one output
  connected feeds that one, no outputs connected holds everything and backs
  the line up.

  (3) **The buffer is an ordinary one-tile `BeltSegment`, not a bespoke
  one-slot type** — the `Splitter` component holds *only* the round-robin
  cursor. A splitter is a belt tile with a choice at the end of it, so
  reusing the belt's own gap-based storage means an item crosses it in the
  same 50 ticks it would cross a belt: inserting one into a line changes
  where product goes and never how fast it travels, where an instant one-slot
  buffer would have made a line *with* a splitter measurably quicker than the
  same line without one. It also means every existing hand-off works on it
  verbatim (belt-to-splitter is `try_transfer_belt_to_belt`, machine- and
  hopper-to-splitter are the functions they already use for a belt), so the
  "spawn first, then take" ordering that makes a refused hop non-destructive
  is inherited rather than written a fourth time in a place the
  mass-conservation tests would only catch by luck.

  (4) **A genuinely new kind and a genuinely new `BodyKind`, not a flag on
  the belt.** It has its own 1x1 footprint in `data/machines.ron` (the
  *fourth* kind there with no recipe behind it, beside the hopper, the tank
  and the dock) and its own price in `data/shop.ron`, so it is bought,
  placed, rotated and removed by the existing commands with **no new command
  type**. What makes it its own `BodyKind` rather than `Belt` with a second
  target is precisely the one-to-many resolution: `Topology` gained one
  relation, `splitter_outputs: Vec<(Entity, [Option<Entity>; 2])>`, and the
  belt loop's sink classification was factored into one `resolve_sink`
  closure both relations call — so a splitter's outputs reach exactly the
  sinks a belt's hand-off does (another belt, another splitter, a machine's
  inlet face, a dock) and the two can never diverge on what counts as valid.

  (5) **Priced at 25 and listed with the infrastructure**, not with the
  buildings: five belt tiles' worth, a real premium over a plain tile and
  still an order of magnitude under any building. A splitter is a mechanism
  where a belt is a surface, and half the point of building one is that it is
  cheaper than the second production line it lets you feed. **Flagged, not
  fixed:** `main.gd`'s hotbar has ten number keys and `shop.ron` already had
  eleven entries before this, so the catalog has outgrown the strip — adding
  the splitter in the infrastructure group makes the last two kinds keyless
  rather than the last one. Nothing becomes unreachable (the build menu still
  places by right-click), and it is ticket 2's to solve.
- 2026-09-08: **The chain rework, ticket 1 of 2 (the sim half;
  `godot_project/` and `assets/voxels/` deliberately untouched).** Four
  changes that only make sense together: the aging rack is deleted, the
  middle of the production line is **piped instead of belted**, the mixer
  gains **sugar** as a second raw ingredient, and **raw ingredients are
  bought with money** instead of being free and infinite. Seven calls worth
  not re-litigating.

  (1) **`Recipe` gained `fluid_outputs`, and which network it goes into is
  geometric.** Until now a recipe could *consume* a fluid and nothing could
  make one — the only thing that could add volume to the system was the
  player filling a tank — so the middle of a line had to be belt-connected
  even where a real plant pipes it. `fluid_outputs` mirrors `fluid_inputs`
  (`#[serde(default)]`, whole units) and `fluid.rs` gained
  `can_accept_at`/`deposit_at` as the exact mirror of `can_supply_at`/
  `draw_at`, all-or-nothing for its reason. The rule that makes a machine
  plumbed on *both* sides unambiguous without the player labelling a run:
  **a fluid output leaves by the face the machine points at, exactly as a
  solid one does; a fluid input may come from any other pipe touching the
  footprint.** So the topology pass resolves the output tap first and then
  excludes that network when choosing the input — which also decides the
  degenerate case, a loop of pipe touching one machine on two sides: it
  reads as the outlet and the machine gets **no** inlet, because a machine
  must not draw from the run it fills. `Machine::tick` therefore takes two
  taps rather than one; they are always different networks, which is what
  makes holding both mutable borrows sound (`world.rs`'s `two_networks_mut`
  asserts it rather than assuming it). A fluid output is gated at craft
  *completion*, symmetrically with the solid output buffer beside it, so a
  full pipe backs a machine up instead of spilling. `recipes.ron`'s
  `schema_version` goes to **3**, rejecting v2 rather than migrating it —
  the same call v1 got, and more clearly right this time: a v2 file's chain
  is belt-connected end to end, and defaulting it into this build would
  produce a factory that loads, renders and never runs.

  (2) **`validate_chain` recognises a fluid link.** Two stages joined only
  by a pipe share no solid id at all — the mixer's whole output is a liquid
  and the vat's whole input is one — so without this the shipped file would
  look like a broken line. A typo'd *fluid* id is exactly as much a content
  error as a typo'd item id and still panics at construction.

  (3) **A machine's inlet face is as long as its footprint, and that is why
  the mixer is now 1x2.** This is the call the ticket did not anticipate and
  the one most worth recording. A machine accepts solid input on every tile
  of the edge opposite the way it points, so a 1x1 machine consuming two
  different solid ingredients has **one** inlet tile, i.e. a merge point —
  and a merge with no splitter or filtered insertion behind it *deadlocks*:
  whichever ingredient arrives faster fills the shared belt, the other never
  reaches the machine, and the machine then never consumes the first either.
  This was built and measured, not theorised: a 1x1 mixer with sugar
  side-fed onto the paste belt jammed permanently within a few thousand
  ticks. Giving the mixer a second tile of depth gives sugar its own lane
  into its own inlet tile and removes the merge entirely. So: **a machine
  that consumes N solid ingredients needs an inlet face at least N tiles
  long**, recorded in `data/machines.ron`'s header for the next
  multi-ingredient kind. The mixer is also the first shipped kind that is
  genuinely oblong, which is a small bonus — rotation's extent-swap now has
  real content behind it, not only a test fixture. **Note for the
  presentation ticket: the mixer's model is 32x64 voxels now, not 32x32.**

  (4) **Hoppers hold what the player says, and `Place` gained an optional
  `item`.** `World`'s `raw_item: Option<ItemId>` was "the first recipe's
  first input, and nothing else is ever considered raw" — correct while
  there was one raw ingredient, and wrong the moment sugar existed. It is a
  `raw_items: Vec<ItemId>` now, derived by the general rule
  (`ingredients::raw_ingredient_ids`: consumed by some recipe, produced by
  none) so `world.rs` and `ingredients.rs` cannot drift on what "raw" means.
  `PlayerCommand::Place` follows its own documented rule for optionality —
  optional when the simulation owns a sensible answer, required when only
  the player can know it — which here means **optional while inferable,
  required once ambiguous**: omitting `item` still works in a world with one
  raw solid (so nothing about the old wire form changed) and is a silent
  refusal in one with two. Naming a non-raw item is refused too. `FillHopper`
  needs no item parameter: a placed hopper already knows its own.

  (5) **Ingredients are bought, in a second catalog and a second ledger.**
  `data/ingredients.ron` + `sim/ingredients.rs` are `shop.ron` +
  `sim/shop.rs` with one word changed, deliberately — schema version,
  duplicate/degenerate rejection, `include_str!` bundling, and a new
  `PlayerCommand::BuyIngredient` that is `Purchase`'s arm verbatim against
  the other catalog. Two files rather than one column because they price two
  different *kinds of thing*: a shop entry is a building you place and can
  pick back up, an ingredient is stock a machine eats and never returns —
  the same argument `OwnedEquipment` makes for not being an `Inventory`. A
  solid and a fluid *do* share one catalog and one ledger, because ids never
  collide and both are bought in whole units. Money stays on `Economy`
  (GAMEPLAY.md §1 allows one currency). The debit rule is the fiddly bit and
  is stated once: **clamp the request to owned stock, let `fill` clamp again
  to its own capacity, then debit exactly what `fill` reports it added** —
  never the requested amount, since a nearly-full bin accepts less than it
  is offered. A tank holds no fluid id of its own, so what it is filled with
  is the fluid of the run it discharges into; a tank plumbed to a run
  nothing taps cannot be filled at all, because there is nothing to charge
  for. `IngredientCatalog::validate_covers` is a **test**, not a
  construction-time panic — an unpriced ingredient leaves the world coherent
  and merely unfillable, which is the same call `contracts.rs` made about an
  unsatisfiable contract.

  (6) **Overproduction is a real loss now, and that is the design working.**
  With ingredients free, a kept-fed line printed money. It no longer does:
  every box nobody has ordered is ingredients converted into stock in a pool
  no contract wants. The invariant `tests/ingredients.rs` pins is therefore
  the **per-unit margin** (a delivered unit is worth more than the
  ingredients that made it, on every shipped contract, derived from the data
  files rather than written as literals) rather than "a running factory
  earns more than it spends over N ticks", which is false and correctly so —
  it is exactly the pressure GAMEPLAY.md §6 chooses contracts over a spot
  market to create.

  (7) **The reference chain is rebuilt, and every test file with it.**
  `sim::reference_chain`'s `plan()` now lays **one feed lane per solid
  input** (row `CHAIN_ROW_Y + j` for input `j`, with a hopper at its head if
  that input is raw), no belt at all for a stage whose inputs are entirely
  fluid, and a short pipe run east of any stage with a fluid output. The
  machine-to-machine runs are **two tiles** where the tank-fed vinegar run
  is four: a pressure network equalizes, so a run of `n` tiles holds roughly
  `n` times what the far end needs to draw once, and a run fed by a machine
  has no pump behind it to push through that standing charge. Two new
  fixture seams exist because a world that is going to be *run* now needs
  money for stock: `reference_chain_with_budget` and
  `reference_chain_with_content_and_budget`, both of which still assert the
  chain spends its whole shop bill. `tests/timed_processes.rs` is deleted
  with the aging rack; its pause/resume coverage went with it deliberately,
  since that test asserted a property of a 2400-tick process and
  `TimedProcess` itself is unchanged and exercised by every craft cycle in
  the suite.

- 2026-09-08: **An inventory hotbar, and a starting balance that lets the
  loop close.** Two directly-requested changes in one ticket, because the
  second breaks what the first touches. `sim_core` got exactly one changed
  constant; everything else is `godot_project/`. Six calls worth not
  re-litigating.

  (1) **`STARTING_MONEY` is 3000, not 200.** The "Real Placement" epic's
  ticket 2 flagged the hole and left it for the balancing pass: money enters
  the world only through contract deliveries, a delivery needs a line that
  reaches the dock, and that line (`sim::reference_chain`, priced out of
  `data/shop.ron`) costs **1087** — so at 200 a new save could not be played
  forward to a first payout by any sequence of commands. Not a slow game, a
  dead one. 3000 is deliberately **more than 2.5x** the bill rather than
  exactly it: a new player can build the whole shipped line outright and
  still have ~1900 for the mistakes and a second line, and a balance that
  only just covered the bill would make a first factory a puzzle with one
  solution. `sim_core/tests/shop.rs` pins the **relationship** (starting
  money against the chain's own dynamically priced bill) rather than the two
  literals, plus one test pinning the literal itself, so a price change moves
  the expectation instead of breaking it.

  (2) **Every test calibrated against 200 was reworked, not patched.** Three
  Godot files argued from insufficiency — "176 of a 200 balance is why this
  fixture is small", "the vat costs the entire balance so it needs its own
  scene", "the rack cannot be fed on a starting balance" — and every one of
  those reasons had evaporated. Each was rewritten to the reason that
  actually holds now: the fixtures are small *by choice* (a nine-tile line
  proves a wire format as well as a forty-tile one, and much faster), the vat
  needs its own world because its footprint **collides** with the other
  fixture's at (0, 0), and the rack is driven through its own gauge mechanism
  because feeding it for real would mean thousands of ticks of travel. The
  Rust test that asserted "one of each front-end machine costs more than
  twice your balance" was replaced by its opposite-facing twin, which is the
  claim now worth defending. And coverage the old balance had *deleted* came
  back: `test_layouts.gd` gained `build_reference_line` (the whole six-stage
  chain, through ordinary purchase/place commands) and
  `screenshot-scene.sh` got `--contracts` back on top of it.

  (3) **The hotbar is the shop catalog in `data/shop.ron` order — fixed
  slots, not a packed list of what is owned.** Packing reads tidier and
  destroys the one property a hotbar exists for: slot 5 has to still be the
  crusher after the last crusher is placed, or every key the player has
  learned renumbers itself under them. Minecraft, Terraria and Factorio's
  quickbar are all fixed-slot for exactly this reason. A kind owned in
  quantity 0 is drawn as an **empty socket** (ghosted icon, no count at all,
  recessed) rather than a disabled button — the kind is not unavailable, the
  player simply is not carrying one. Bottom edge, centred: the other edges
  are taken by the legend and the clipboard.

  (4) **Number keys select; SHIFT+number buys, and only with the build menu
  open.** Selecting what is in your hand and spending money on more of it are
  different verbs and stay on different actions. The number keys used to buy
  while the menu was up and do nothing otherwise (that handler's own comment
  reserved them "for whatever a later epic wants a hotbar to do" — this was
  that epic); they now select in either view mode, menu or no menu, and
  pressing the selected slot's key again deselects, mirroring ESC. Buying
  keeps its keyboard path on SHIFT+number but **only while the menu is
  open**, because that is the one screen where the price and the balance
  being spent are both on screen. A key that spends money with no prices
  visible is the accident this split exists to prevent.

  (5) **The selected state is signalled four ways at once, and the strip is
  animated.** UI_UX.md §7 forbids colour alone: the slot grows, its frame
  turns hazard-yellow, a caret appears above its icon, and its key digit
  brightens — plus Minecraft's name flyout above the bar, which fades after
  ~1.5s. The growth is a **scale**, not a position, because an
  `HBoxContainer` owns its children's positions and sizes but never their
  transforms; the same 100ms spring is the click pulse. Selection itself is
  **derived from `_build_kind`**, never stored, so ESC, B, X and running out
  of a kind all clear the highlight without knowing the hotbar exists.

  (6) **Two things came off a rendered frame that the code could not show**
  (UI_UX.md §2b point 12's lesson again). The strip's plate was translucent
  enough that the floor's diamond tiling showed through the gaps between
  sockets and turned the row into noise — it is nearly opaque now, like the
  build menu's. And the build menu, centred in the viewport and now a row
  taller (the owned line wraps, so a slot grew), reached down far enough to
  bury **its own footer** — the one line that explains how to buy — under the
  hotbar plate. The menu is centred in the space *above* the strip now and
  went from three columns to four so it fits there. `--stock` and
  `--hotbar N` were added to `screenshot-scene.sh` to make both reviewable.

- 2026-09-08: **"Real Placement" epic, ticket 2 of 2 (the Godot half) — the
  scene boots into an empty floor and the player builds the factory.**
  `sim_core` and `data/` were deliberately untouched: every fact drawn here
  was already on the wire after ticket 1. The largest presentation change the
  project has had, and with ticket 1 it completes the shop/inventory/building
  pivot. Eight calls worth not re-litigating.

  (1) **The scene rebuilds its whole "World" subtree on every
  place/remove/rotate, and never per tick.** `main.gd` used to read
  `chain_layout_json` ONCE, because the factory arrived pre-built and nothing
  could move it; it now re-reads `world_layout_json` and rebuilds. A full
  rebuild rather than an incremental patch, for exactly the reason
  `World::recompute_topology` rebuilds rather than maintaining an incremental
  union-find: at this scale rebuilding a few hundred nodes costs less than
  the bookkeeping an incremental update needs, and it cannot drift out of
  step with the simulation. The trade flips at milestone 8's scale, in that
  one function, on both sides.

  (2) **Rotation needed no new geometry, only a Y rotation and a re-centred
  footprint.** Every asset is modelled facing east and the encoder puts a
  model's origin at its own footprint centre, so a placement is the same mesh
  spun about that origin and re-centred on the rotated rectangle. `east 0°,
  south 90°, west 180°, north 270°` is exactly `index * 90` along one array
  held in the simulation's own clockwise order (`Direction::rotate_cw`), so
  the ghost's rotation and `PlayerCommand::Rotate` cannot disagree about
  which way "rotate" goes.

  (3) **The junction art stays east-only, and the fallback is now tested
  rather than asserted in prose.** UI_UX.md §2b point 11 derives the belt's
  inlet/outlet variants from the fact that this camera can only ever see the
  +X and +Z-world faces; spinning those models 180° puts the carved side
  where nobody can see it and a blank wall where the product path is, which
  is visibly *worse* than the plain symmetric tile. So three of the four
  facings a player can now choose deliberately fall through to the plain
  model. A four-directional junction pass is an art ticket, explicitly not
  this one. `test_connections.gd` asserts the fallback on a tile that WOULD
  take a variant, and a rendered frame confirms nothing floats or clips.

  (4) **Two mouse buttons on a build-menu slot, not one that changes
  meaning.** Left-click buys one; right-click places one you own (number and
  shift+number are the keyboard mirror). The alternative — "click buys when
  you own none, places when you own some" — is shorter to explain and worse
  to use: it makes buying a second of anything impossible without placing the
  first, and makes one gesture mean two things depending on remembered state.
  The affordance is printed on the slot's own owned line and in the menu
  footer.

  (5) **Ghost validity is a pure Godot-side, footprint-aware check against
  the occupancy map, and it is signalled by glyph and word as well as
  colour** (UI_UX.md §7): **✔ PLACE** / **✗ BLOCKED** / **✗ NONE OWNED** in a
  prompt beside the cursor, plus a tint and a matching panel border. The
  ghost's tint MULTIPLIES over the model's own vertex colours rather than
  replacing them — the first cut replaced them and a rendered frame showed a
  belt ghost as a featureless green rectangle, which says a tile is involved
  but not which thing.

  (6) **Remove is a MODE (X), not a modifier-click**, because it is the one
  destructive action in the game (what a removed thing held is destroyed) and
  a mode announces itself in the legend and in the on-screen prompt where a
  held modifier cannot. Its prompt is red even when the click will succeed:
  green means "this click builds", red means "this click destroys, or
  refuses" — the first cut framed a valid REMOVE in the same green a valid
  PLACE gets, which reads as two claims about one action.

  (7) **R rotates what is under the cursor when not placing**, so turning a
  belt you already laid needs no pick-up-and-re-place round trip. It is free:
  `Rotate` resolves from any tile of a footprint and keeps the thing's
  contents.

  (8) **A starting balance cannot buy the shipped line, and that reshaped the
  test suite.** *(Superseded on the same day by the inventory-hotbar entry at
  the top of this log, which raised `STARTING_MONEY` to 3000 — the "Flagged,
  not fixed" note below is the thing it fixed. The test-suite reshaping it
  describes still stands.)* A new game had `STARTING_MONEY` (200); the six-stage chain
  costs about 1,087, and `Purchase` is all-or-nothing with money arriving
  only through contract deliveries — which need that line. So no Godot-side
  test or screenshot can reach it any more. Full-chain behaviour stays where
  it already was, in `cargo test -p sim_core` against `sim::reference_chain`
  (which starts its world with exactly its bill via
  `World::with_content_and_money`, a seam with no wire equivalent — a player
  has none either). Every headless Godot script now BUILDS the factory it
  asserts against out of real purchase/place commands
  (`godot_project/test_layouts.gd`), which is a better test of the boundary.
  **Flagged, not fixed:** the same arithmetic means a new save cannot be
  played forward to a payout at all — 200 buys a hopper, some belt and one
  machine, and the first sellable product is six machines away. That is a
  balance decision for whoever picks up the economy (GAMEPLAY.md §10's
  balancing pass), not a rendering one. One thing fixed in passing:
  `sim_core/tests/timed_processes.rs` asserted that a paused run took longer
  in wall-clock time than an unpaused *control* run, which is below
  run-to-run scheduling noise for two half-second debug-build loops and
  failed reproducibly on a loaded machine (confirmed against a clean
  checkout). It now asserts only that the paused run really spent the pause
  in real time, which is the claim that was actually being made.

- 2026-09-08: **The delivery dock, drawn** (the delivery-dock ticket's
  presentation half; `sim_core` and `data/` deliberately untouched — every
  fact drawn was already on the wire from ticket 1). Five calls worth not
  re-litigating. (1) **The dock is drawn as architecture, not as
  equipment.** Every other kind on the line is identified by machinery (or,
  for the aging rack, by the deliberate absence of it); a dock does no work
  at all, so it is a poured platform at trailer-bed height with a striped
  edge, rubber bumpers, a despatch door and a canopy — which is both what a
  real loading bay is and the one silhouette on this line that cannot be
  mistaken for another housing. (2) **Its layout was decided by an occlusion
  rule, recorded as UI_UX.md §2b point 16.** The obvious design — a truck bay
  carved into the camera-facing +X face — was built first and walls off the
  entire deck, because on this camera a sight line travels `(+x, -y, +z)` and
  every deck voxel's exits through those columns. The tall structure
  therefore stands at the FAR side, where its own face is still square to the
  camera and it occludes nothing. That corollary to point 11 is new and
  applies to any future building with something inside it. (3) **No fill
  gauge, and that is the point rather than an omission.** `Dock::ship_into`
  empties the buffer every tick, so `DockSnapshot` has no `fill` field and a
  bar reading it would be a gauge lying about its mechanism (the same failure
  `FluidTank` refused). What the deck shows instead is one case per unit of
  `EconomySnapshot::delivery_stock` — the shared pool the clipboard already
  labels ON THE DOCK, exactly as `sim::DockSnapshot`'s own doc instructs —
  drawn with the item's OWN model through `ITEM_MODELS`, capped at two
  because a rendered four-slot version buried the building under cardboard.
  The uncapped figure stays on the clipboard and in the hover. (4) **The
  packer gained an outfeed it never had**, and the `NO_OUTFEED` list in
  `voxgen`'s junction tests now names the dock instead of it: a real belt runs
  past the packer now, so its +X face was a wall a case appeared in front of
  out of nowhere. The dock is the end of the line and has no outfeed by the
  same rule. (5) **`C` is removed, not left inert.** The keybind, both help
  strings' `"C: collect product"` fragment, and every `{"type":
  "collect_output"}` call left in the headless test scripts are gone —
  `issue_command_json` silently ignores an unknown command, so a key left in
  place would have advertised a control the simulation no longer has.
  `test_interaction.gd` now asserts both halves (the legend never says
  "collect", and pressing **C** changes neither the economy nor the dock).
  One thing fixed in passing: `--contracts` in `scripts/screenshot-scene.sh`
  waited for the pool to hold *exactly* two units, a state the aging rack's
  throughput makes essentially unreachable, so it silently ran its whole tick
  limit and photographed an empty dock; it is a minimum now.

- 2026-09-08: **"Real Placement" epic, ticket 1 of 2 (the sim half;
  `godot_project/` deliberately untouched) — the world starts empty, and
  everything in it is placed by the player.** The largest single change the
  project has had, and the one that makes GAMEPLAY.md §4's "equipment
  purchases *are* the tech tree" a statement about the simulation rather
  than an intention. `World::new` used to build a complete factory from
  `data/recipes.ron`'s file order — one belt-and-machine stage per recipe
  west to east, a hopper at the head, a tank beside the pipe run, a dock at
  the tail, every stage linked to the next by index at construction. It now
  builds an empty grid and `economy.rs`'s new `STARTING_MONEY`; three new
  commands (`Place`/`Remove`/`Rotate`) do the rest. Nine calls worth not
  re-litigating.

  (1) **`Direction::East` is the regression anchor, and everything is
  measured against it.** `sim::types::Direction` is four-way (`East` = `+X`,
  the way the chain always ran; `North` = `+Y`, the way a footprint always
  grew), and an east-facing world reproduces every pre-placement behaviour
  tile for tile. That is not a claim, it is a test fixture:
  `sim::reference_chain` rebuilds the entire shipped six-stage line out of
  nothing but `Purchase` + `Place` commands, every existing integration test
  now builds its world from it, and `machine_footprints.rs` still pins the
  same literal coordinates it always did (crusher at x = 4, vat at 14,
  packer at 30, dock at 35). The fixture is simultaneously the proof that
  placement can reach every configuration the game already guaranteed.

  (2) **A rotated footprint is anchored at the same origin corner, and a
  quarter turn swaps the extents.** Stated once in `machines.rs`'s module
  doc, which already owned the origin convention. `direction` is *the side
  product leaves by*: the rectangle edge on that side is the outlet face,
  the opposite edge is the inlet face, and the two flanks are neither — a
  belt pointing into a machine's side does nothing, which is what makes
  rotating a machine a decision with a visible consequence. No shipped kind
  is oblong (the 2x2 vat is square), so the extent-swap is exercised by
  deliberately oblong test kinds, exactly as ticket 1 of the "Machines
  improvement" epic exercised widths no shipped kind had.

  (3) **Fluids connect by touching, not by facing.** A machine taps any pipe
  tile orthogonally adjacent to any tile of its footprint, and pipe-to-pipe
  flow is undirected. Not a shortcut: the shipped mixer has always been fed
  from the row *above* it — neither its inlet face nor its outlet — because
  a pressure network is not a conveyor (ARCHITECTURE.md §3.4), and
  face-gating a fluid inlet would have broken the one plumbing arrangement
  the game has. A pipe tile's `direction` is therefore presentation-only.

  (4) **Topology is recomputed on `Place`/`Remove`/`Rotate` and never per
  tick** — ARCHITECTURE.md §3.8's union-find rule made real for the first
  time. It is a full single-threaded rebuild rather than an incremental
  union-find, because at this scale rebuilding a few hundred tiles is
  cheaper than the bookkeeping an incremental structure needs and cannot
  drift out of sync with the occupancy grid. `rayon` and per-network
  threading stay milestone 8's scope, deliberately unbuilt. Pipe *volumes*
  are carried across a rebuild by tile, so joining two runs does not spill
  them.

  (5) **A belt is one tile now**, bought and placed one at a time (which is
  what `shop.ron`'s "priced per tile" always said). Travel time is
  unchanged — a tile is one unit long at the same speed, so four tiles is
  still 200 ticks — and the belt-to-belt hop spawns onto the receiving belt
  *before* taking from the sending one, which is what makes a refused hop
  leave the item where it was rather than annihilating it.

  (6) **The hopper, the fluid tank and the dock are ordinary placeable
  kinds, and a world may hold any number of each.** Allowing several is the
  absence of an invented restriction rather than a feature: nothing in the
  simulation ever assumed exactly one, so forbidding a second would have
  been new code enforcing a rule no design doc states. Several docks still
  ship into the one shared delivery pool, so "what is on the dock" remains
  exactly one number. `FillHopper`/`FillFluidTank` gained an optional
  `tile` naming which one to load; omitting it loads them all, which is what
  a one-hopper world always did on a keypress.

  (7) **A placed machine runs the one recipe its kind names**, from a
  kind → recipe map built at construction. `data/recipes.ron` has always
  been 1:1 on that, and two recipes on one kind is now a construction-time
  panic rather than a silent pick — a recipe-selection mechanic is out of
  scope. One knock-on: test fixtures that invented machine kinds
  (`"cooker"`, `"sprawler"`) now use real, shop-priced kind ids with
  fabricated footprints, because a machine has to be *buyable* to be
  placeable.

  (8) **Removing something destroys what it held** — items on the belt,
  fluid in the pipe, a batch mid-craft. That is the honest reading of
  picking a thing up; pretending otherwise needs an inventory to put the
  spillage in, which nobody has asked for. Mass conservation therefore holds
  across any run in which nothing loaded is removed, which is what the tests
  asserting it actually do. Rotating, by contrast, keeps everything: no
  belt gap array, pipe volume or `TimedProcess` has an absolute-direction
  dependency, so a turn is pure geometry.

  (9) **The wire format broke in several places, all of them ticket 2's to
  consume.** `chain_layout_json` is gone, replaced by `world_layout_json`
  (every placement, with `kind`/`tile`/`direction`/`footprint`/`occupies`/
  `recipe`) — there is no chain and no stage left to report. The hopper,
  tank, dock and fluid snapshots return **arrays** instead of single
  objects; a `BeltSnapshot` is one tile (`tile` + `direction`, no
  `start_tile`/`length_tiles`); a `FluidSnapshot`'s inline machine fields
  became a `taps` array plus `tank_tiles`, and its `segments` are in tile
  order rather than flow order; and every belt, pipe, machine, hopper, tank
  and dock on the wire carries a `direction` string
  (`"north"`/`"east"`/`"south"`/`"west"`). `godot_project/` was not touched,
  so the scene needs ticket 2 before it runs again.

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
  and the biggest machine is visibly the slow one. *(The 2026-09-09 tuning
  pass raised the crush from 60 to 80, so the cook is 3x it now — the
  ORDERING this sentence is really about is unchanged, and is now written
  down as a rule in `data/recipes.ron`'s own header.)* (3) **The fluid
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
