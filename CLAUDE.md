# sauce-factory

Factorio-like automation game, themed around sauce production. Pre-alpha.

@docs/ARCHITECTURE.md
@docs/UI_UX.md
@docs/GAMEPLAY.md

The three docs above are authoritative and always apply. ARCHITECTURE.md is
the tech-stack plan (Rust simulation core + Godot/GDScript presentation via
`gdext`, data-driven recipes, deterministic tick simulation). UI_UX.md is the
visual/UI plan (isometric voxel art, 32×32×32 tile-scale assets, industrial
theme, Minecraft-adjacent UI with clipboard/checklist motifs). GAMEPLAY.md is
the gameplay/economy plan (sauce tiers gated by equipment, one money currency,
contracts as the demand system — not a spot market — to force a diversified
factory). Each doc explains *why* its choices were made; read that reasoning
before you change something that seems to conflict with it.

`docs/DECISIONS.md` (not `@`-included) holds the full text of every decision
record. This file keeps only the condensed rules. `docs/DEVLOG.md` (also not
`@`-included) is the status tracker and dated log.

## Working conventions

- Follow ARCHITECTURE.md for stack/architecture, UI_UX.md for anything
  visual, GAMEPLAY.md for mechanics/progression/economy. Do not deviate
  without discussing it first. Examples of deviations: Unity/Unreal/Bevy,
  gameplay logic in GDScript, recipes hardcoded in Rust, flat-textured
  non-voxel assets, a free-rotating camera, a spot market beside contracts.
- The project is built primarily by Claude Code. When you make a non-trivial
  design decision not covered by the three docs, add the condensed rule to
  the decisions log below, the full reasoning to `docs/DECISIONS.md`, and a
  note to the relevant doc.
- Prefer small, playable increments over big upfront systems.
- Use plan mode for anything that touches architecture or adds a major
  system. Small fixes do not need it.

## Project structure

Milestones 1–7 and several epics have landed (see docs/DEVLOG.md). The game
boots to a main menu, the world starts empty, and the player buys and places
every tile. The shipped chain: tomato + sugar → crusher → mixer → (pipe) →
cooking vat → (pipe) → bottler → packer → dock. Hopper and fluid tank feed
it; contracts pay for what the dock ships.

```
sauce-factory/
  CLAUDE.md, CONTRIBUTING.md, README.md
  Cargo.toml               # workspace: sim_core, xtask, assets/voxels
  .cargo/config.toml       # `cargo xtask` alias
  assets/voxels/           # voxel asset pipeline (UI_UX.md §2a/§2b)
    Cargo.toml             # crate `voxgen`; its source IS generators/
    palette.ron            # single source of colour AND material class
    generators/            # main.rs, palette.rs, vox.rs (hand-written
                           #   MagicaVoxel encoder), assets.rs, icons.rs
    models/                # generated .vox + materials.json/index.json.
                           #   Build output, COMMITTED, never hand-edited
    icons/                 # generated UI icons, committed
    previews/              # gitignored review PNGs (incl. scene.png)
  data/                    # data-driven content (ARCH §3.6). Each file has a
                           #   schema_version; loaders reject a mismatch
    recipes.ron            # recipes; embedded via include_str!. Header states
                           #   the process-time ordering rule (see log)
    machines.ron           # kind -> footprint (width_tiles, height_tiles) and
                           #   the REQUIRED staff crew (schema 2). Power
                           #   belongs here later too
    ingredients.ron        # unit price of each RAW ingredient (solid+fluid)
    shop.ron               # price per BUILDABLE kind: every machines.ron kind
                           #   plus belt/pipe tiles. Must cover every kind
    contracts.ron          # contract BOARD (schema 2): id, name, mix, cadence,
                           #   payout, price, strikes, starter. Only starters
                           #   run in a new game; the rest are signed (C).
                           #   List order = priority when contracts compete
    staff.ron              # staff categories (schema 1): id, name, wage,
                           #   starting_headcount, + pay_period_ticks. File
                           #   order = panel order = PAYDAY order. machines.ron
                           #   (schema 2) names a crew per kind from these
  sim_core/                # Rust crate, crate-type cdylib + rlib
    src/lib.rs             # the ONLY file that touches gdext (ARCH §4).
                           #   JSON-string wire format via serde_json
    src/sim/               # pure Rust sim, no gdext, tested headlessly
      types.rs             #   Fx (fixed-point), ids, TileCoord, Direction
                           #     (East = +X is the regression anchor)
      recipe.rs            #   RecipeRegistry (validates recipes.ron)
      inventory.rs         #   slot-based Inventory
      belt.rs              #   BeltSegment, gap-based storage (ARCH §3.3)
      machine.rs           #   Machine + TimedProcess
      machines.rs          #   MachineRegistry; OWNS the footprint origin
                           #     (min-X/min-Y corner) and rotation convention
      fluid.rs             #   FluidNetwork, FluidTank, pressure equalize
      splitter.rs          #   Splitter (round-robin cursor only)
      staff.rs             #   StaffCatalog + StaffPools: per-tick crew
                           #     claims in stable order, payday, hire/fire.
                           #     A plain field on World, like Economy
      ingredients.rs       #   IngredientCatalog + OwnedIngredients; owns
                           #     raw_ingredient_ids (the one definition of raw)
      shop.rs              #   ShopCatalog + OwnedEquipment. Money is NOT here
      contracts.rs         #   ContractRegistry
      dock.rs              #   Dock: pass-through into Economy every tick
      economy.rs           #   Economy: money (Fx), the board, the ACTIVE
                           #     Vec<ActiveContract>, one shared
                           #     delivery_buffer, losses, STARTING_MONEY;
                           #     try_sign + strikes/loss in tick
      world.rs             #   hecs::World of placed things. World::new builds
                           #     NOTHING. Place/Remove/Rotate; topology
                           #     recomputed only on those, never per tick
      save.rs              #   RON snapshot save format; no filesystem code
      reference.rs         #   sim::reference_chain: the shipped layout built
                           #     from Purchase+Place commands; test fixture
    tests/                 # headless integration tests, one file per concern
                           #   (core_loop, placement, machine_footprints,
                           #   fluid_production, ingredients, economy, dock,
                           #   contract_board, save_load, shop, auto_fill,
                           #   staff; common/mod.rs keeps hoppers/tank fed
                           #   for long runs and funds WAGE_RESERVE)
  xtask/                   # `cargo xtask`: build sim_core, copy the lib to
                           #   godot_project/bin/, stage models to
                           #   godot_project/voxels/ (both gitignored)
  godot_project/           # Godot 4 presentation layer
    project.godot          # main_scene = menu.tscn; autoload GameSession;
                           #   window 1600x900
    menu.tscn / menu.gd    # main menu: New Game, Load Game, Quit
    game_session.gd        # autoload: pending new_game/load action, and ALL
                           #   save-file rules (dir, listing, slot sanitizer,
                           #   reserved `autosave` slot)
    main.tscn / main.gd    # the 3D scene: rebuilt from world_layout_json on
                           #   every place/remove/rotate; build mode, hotbar,
                           #   clipboard, market, contracts board (C), staff
                           #   panel (H), pause menu, and the click-to-
                           #   configure panel on a placed hopper or tank
    audio.gd               # every sound, SYNTHESISED into AudioStreamWAV at
                           #   runtime; no .wav binaries. Reached by preload.
                           #   14 feedback cues + music_samples(), the track
    music.gd               # autoload `Music`: the one looping track, started
                           #   by the menu, carried across the scene change,
                           #   rendered on a Thread, ducked under pause
    sim_core.gdextension   # checked in; points at gitignored bin/
    addons/                # vendored, pinned .vox importer (see VENDORED.md)
    test_*.gd              # headless proofs, each prints its own success
                           #   marker; test_layouts.gd is the shared fixture
                           #   (builds factories through real commands;
                           #   build_belt_gallery = every belt shape and
                           #   junction at every facing)
    test_screenshot.gd     # NOT headless: renders the scene to a PNG
  scripts/
    lint-commit-msg.sh     # commit-msg hook
    verify-scaffold.sh     # milestone-1 headless check
    verify-core-loop.sh    # cargo test + every headless Godot proof
    build-voxels.sh        # regenerate .vox files + review PNGs
    screenshot-scene.sh    # render main.tscn to PNG (flags: --populate,
                           #   --contracts, --payout, --stock, --hotbar N,
                           #   --no-hud, --closeup, --first-person, --stress,
                           #   --contracts, --board, --stock, --hotbar N, --no-hud,
                           #   --closeup, --first-person, --stress,
                           #   --belt-gallery, --bottleneck, --staff,
                           #   --understaffed, menu/pause)
  docs/                    # README.md (index), ARCHITECTURE.md, UI_UX.md,
                           #   GAMEPLAY.md, DEVLOG.md, DECISIONS.md
  .githooks/               # commit-msg + pre-commit; enable with
                           #   `git config core.hooksPath .githooks`
  .github/workflows/       # CI: commit-message lint
```

Not yet built, deliberately (add when the milestone needs it; do not
improvise a different shape): `rayon` parallelism and the tiered scheduler
(ARCH §3.7/§3.8, milestone 8); `preview_command`/`undo`/`redo` (ARCH §4);
per-module `CLAUDE.md` files for `sim_core/` and `godot_project/` (ARCH §10)
once there is enough to warrant one.

## Orchestration (Orca)

Use Orca's orchestration layer (`orca orchestration ...`) to build
milestone-sized work, not one long session.

- One Run per meaningful chunk of milestone work.
- Milestones map to Tasks about 1:1; encode ARCH §9's dependency chain with
  `--deps`.
- Every task spec must be self-contained: name the milestone and doc
  section. A worker starts with zero conversational context.
- Foundational/large milestones get `--worktree new-top-level`. Small
  follow-on work can reuse `current` or a child worktree.
- Prefer `worker-start` over manual `dispatch --inject`.
- Use `gate-create` at milestone boundaries for a human check.
- After every accepted `worker_done`, decide whether it is DEVLOG-worthy.
- Full ownership handoffs use `orca-cli`/`orca worktree create --prompt`,
  not Task/Dispatch machinery.

## Commit messages

Conventional Commits, enforced by a `commit-msg` hook + CI. See
CONTRIBUTING.md. Run once per clone/worktree:

```
git config core.hooksPath .githooks
```

**Before finishing a commit, decide whether docs/DEVLOG.md needs a note.**
The pre-commit hook prints a reminder but never blocks. Most small commits
do not need one. Add a brief, high-level entry for a decision, a milestone,
or meaningful progress, and update DEVLOG's "Current status" if it is stale.

## Decisions log (condensed)

Newest first. Each entry is the rule; `docs/DECISIONS.md` has the reasoning.
Do not re-litigate these without a good reason.

- 2026-09-13 **Production stats — the P screen.** A `ProductionLedger` on
  `World` counts what each recipe DEPOSITS (made) and TAKES (used) at craft
  start/completion, plus what a paying contract takes from the delivery
  pool; buying stock, filling a bin, and belt metering count as nothing.
  Four rolling windows (5s/1m/10m/1h) plus lifetime totals, polled via
  `production_snapshot_json`, saved with the world. **P** opens the panel
  (dark plate, like H/C/E/M); Godot only sorts and formats (MADE/USED bars,
  rate-per-minute over the window's observed span), never counts.
- 2026-09-12 **Music and a full set of sound effects.** Reverses UI_UX §8's
  "no soundtrack" half, keeps "feedback, synthesised, no `.wav`". Fourteen
  cues in `audio.gd`, each fired by the handler that knows the real
  outcome (never by the input), so every refused command shares one buzz.
  One looping 8-bar track on a `Music` autoload starts on the main menu,
  ducks (not stops) under pause with its own ON/OFF switch, and is skipped
  entirely in headless test runs.
- 2026-09-12 **Staff pools MVP, ticket 3 (Godot).** **H** opens a STAFF
  panel: per category, hired count, wage, and an assigned/demand meter
  with any shortfall called out; footer shows the upcoming wage bill and
  payday countdown. Godot reads the sim's own demand/assigned/short
  numbers, never recomputes them; hover and the build-menu card both
  surface a machine's or dock's crew and any staffing shortfall.
- 2026-09-12 **Staff pools MVP, ticket 1 (sim).** Staff are four aggregate
  pools (`data/staff.ron`); every kind in `machines.ron` (schema 2)
  declares its required crew. Each tick, machines claim their whole crew
  in stable order — a machine that can't gets `Starved` with the reason
  recorded and its progress held, not lost. Payday pays wages in category
  order from the one money counter; an unpayable head is fired. Long
  test fixtures fund a wage reserve so they don't go broke.
- 2026-09-12 **Staff pools, ticket 2 (icons).** Four UI-only voxel icons —
  hard hat, spanner, taped carton, pallet jack — one per staff category,
  never placed in the world.
- 2026-09-12 **Machine load: the stack-light poles are removed.** A
  playtest verdict on looks — a pole beside every machine was not wanted
  — so the six `andon_*` models and their scene nodes are gone (the two
  tickets that built them are superseded). The sim's load window and wire
  are unchanged; the hover panel now carries the whole indicator instead
  (status glyph + word, and a bar filled in the status colour). Floor-wide
  bottleneck legibility (GAMEPLAY §9) is an open question again.
- 2026-09-11 **Machine load, ticket 1 (sim).** Every recipe-running
  machine keeps a rolling ~600-tick `LoadWindow` of `Working`/`Starved`/
  `Blocked` per tick (the completion tick counts as work, so a
  fed-and-drained machine reads 100%). Never dirties a chunk — polled via
  `machine_load_snapshots_json`, which also gives the current stall reason
  (first shortfall in recipe order, solids before fluids). Saved with the
  world; an older save loads a fresh window.
- 2026-09-11 **Payout feedback — what earning money looks and sounds
  like.** Detected entirely in Godot by polling each contract's
  `deliveries_made` for a rise (no sim change); a coin fountain, a money
  counter punch, a "+N" flyout and a PAID stamp fire together, clocked off
  `sim.current_tick()` so it pauses with the game and stays reproducible
  in a screenshot run. A dock arrival gets a much smaller cue. First audio
  in the project, synthesised at runtime — no committed `.wav` files.
- 2026-09-11 **Machine animations.** Every processing machine has a
  four-frame motion overlay on its own footprint (moving parts cut out of
  the body, never duplicated, so nothing z-fights), advanced from the tick
  so pause freezes it. It runs only while `elapsed < required`, so a
  machine finished-but-blocked on a full output stands visibly still with
  its status lamp still lit. One pace for every kind; no steam.
- 2026-09-10 **Belt arrows removed.** The belt's raised direction arrow is
  gone at the player's request; the moving mark is a plain rubber cleat,
  there only so the animation frames read as motion. Direction is stated
  in words (build prompt, hover), never drawn on the tile.
- 2026-09-11 **Vinegar drawn as vinegar.** A fluid gets its own
  `palette.ron` colour, and that RGB is what tints its pipe body too — a
  market icon and a pipe run are seen together, so the value has one home.
  Amber, not clear: a colourless fluid in a stainless pipe reads as empty.
- 2026-09-11 **Factory floor redesign.** The floor is a background surface,
  authored by its value range rather than its detail (UI_UX §2b points 27
  in "the floor" and 28): broad low-contrast blotches, never per-voxel
  speckle, and the floor must never hold the brightest pixel in the frame
  (metal keeps that). Four tile variants laid at a per-tile hashed
  rotation so the repeat doesn't read, with edges kept four-fold symmetric
  so a turned tile's seam never steps.
- 2026-09-11 **UI visualisation pass — one bar language.** A quantity is
  drawn as a bar only where the simulation defines its ceiling (hopper/tank
  level, machine cycle, pipe pressure, a contract's countdown, a price
  against the balance); a figure with no real denominator (larder stock,
  the balance itself) stays a number. One shared meter mechanism, three
  fills each with one meaning (level/progress/warn), always paired with a
  word or glyph. The money figure flashes `+N`/`-N` on every change.
- 2026-09-10 **Contract board: contracts are signed, struck and lost.**
  `contracts.ron` is schema 2: every contract has a `price` to sign, a
  `strikes` count, and an optional `starter` (a new game runs only
  starters). Missing `strikes` windows in a row loses the contract —
  losing IS the penalty, no fine on top; a paid window resets the count.
  **C** opens the contracts board to sign offers; the clipboard docket
  gains a status line, strike pips and a state border.
- 2026-09-11 **Contract board sized in lines' worth of demand.** The board
  is a starter tier one kept-fed line can just serve, plus a growth tier
  of six larger, slower orders sized so a second/third/fourth line always
  has an order to go after; per-box payout never falls as order size
  rises, and file order (smallest first) is the scarcity tiebreak.
- 2026-09-10 **Hopper/tank configuration.** A placed hopper or tank is
  clicked and configured on a panel beside it: level, FILL (with its
  refusal reason in words), an AUTO-FILL toggle, and — for a hopper — an
  item picker. The global fill/cycle keybinds are gone; configuration
  lives on the placed building, not the keyboard.
- 2026-09-10 **Pipes & conveyors overhaul, ticket 5 (belt presentation).**
  A belt tile's shape (straight/corner/merge/start/end) is derived purely
  from its `links` and facing — never authored per-junction. Items are
  drawn face to face (input face's midpoint to output face's), not
  centre-to-centre, so a corner's arc actually leaves through the right
  face. Hover states what a belt carries, turns, and hands off to, in
  words.
- 2026-09-09 **Pipe presentation (ticket 6).** A pipe tile's shape, turn
  and fittings come from its `links.connections` alone. A straight pipe's
  turn is fixed by its axis (never by facing), so joint flanges never
  stack when a run bends; only a tile joined to nothing visibly rotates.
- 2026-09-09 **Pipes & conveyors overhaul, ticket 2 (belt model family).**
  One generator emits every belt/corner shape across 4 animation frames
  on a one-tile pattern period. The belt-side junction art variants are
  retired — the plain tile carries the junction, the machine's own hood
  does the rest.
- 2026-09-09 **Pipe model family (ticket 3, assets only).** Straight/
  corner/tee/cross/end pipe shapes plus machine/tank fittings, on the
  tile's true 15.5 midline (not voxel 16) so a turned run stays symmetric.
  A fitting overlays a shape with a margin so the two meshes never share a
  face.
- 2026-09-09 **Pipes & conveyors, ticket 4 (instanced rendering).** Belts,
  splitters, pipes and carried items are drawn as `MultiMesh` instances
  (one per asset/material class) rather than one node each; anything with
  live per-instance state (machines, the ghost, level bodies) stays a
  node. A 1500-tile stress fixture and `test_perf.gd` guard the win.
- 2026-09-09 **Pipes & conveyors overhaul, ticket 1 (`links` on the
  wire).** Every belt, splitter and pipe in `world_layout_json` carries a
  `links` object naming exactly what feeds it, what it hands to, or what
  it is joined to per side — recorded once by the topology pass, never a
  second adjacency scan, so Godot derives shape from data instead of
  re-deriving adjacency itself.
- 2026-09-09 **Hopper/tank auto-fill.** A placed hopper or tank can be
  switched to automatic, topping itself up from bought stock every tick.
  It spends stock, never money — buying stays an explicit, visible player
  action; an empty larder just pauses the top-ups.
- 2026-09-09 **Save/load, ticket 2 (Godot).** `game_session.gd` owns every
  save-file rule (slot naming/sanitizing, the reserved `autosave` slot,
  listing). A save is proven to load before the scene changes. Autosave
  fires on a 3-minute timer and on every quit path.
- 2026-09-09 **Save/load, ticket 1 (sim).** A save is a full state
  snapshot with a `schema_version`, not a command log (amends ARCH §4) —
  a command log would replay through *current* logic and silently rewrite
  old saves. Fixed-point values are stored as raw bits. Load rebuilds
  structure via real `Purchase`/`Place` commands, then restores live
  state through non-public methods. No file I/O lives in `sim_core`.
- 2026-09-09 **Tuning pass after a playtest.** Crusher/mixer/vat cycle
  times set (crushing quickest, cooking slowest — change the numbers and
  the prose together). The dock grew to 2x2 for depth, not price. UI
  scaled 1.5x via explicit constants, not `content_scale_factor`.
- 2026-09-08 **Chain rework, ticket 2 (Godot/assets).** A hopper's
  contents are chosen at placement, not on a separate catalog entry
  (fixed hotbar slots must not renumber). Buying raw stock got its own
  panel (**M**); a permanent larder strip shows what's bought.
- 2026-09-08 **Splitter, ticket 2 (Godot/assets).** Drawn as a machined
  diverter table with a next-output arrow inlaid flush in the deck
  (nothing stands proud of the surface product slides over); the arrow
  shows where an item will *actually* go, never the raw field alone.
- 2026-09-08 **Splitter, ticket 1 (sim).** A splitter's branch output is
  always one quarter turn clockwise from its straight output, derived
  rather than stored, so one `Rotate` turns both. Round-robin with
  same-tick fallback to the open side so a blocked branch never stalls
  the line.
- 2026-09-08 **Chain rework, ticket 1 (sim).** The middle of the chain is
  piped, not belted: `Recipe.fluid_outputs` mirrors `fluid_inputs`, so a
  fluid product leaves by the facing face and a fluid input may come from
  any other touching pipe. `recipes.ron` bumped to schema 3. Overproduction
  is a real loss now that raw ingredients cost money.
- 2026-09-08 **Inventory hotbar + starting balance.** `STARTING_MONEY` is
  set well above (>2.5x) the shipped chain's full build cost, pinned as a
  *relationship* in tests rather than as two literals, so a price change
  can't silently break the guarantee that a new game can reach a payout.
- 2026-09-08 **Real Placement, ticket 2 (Godot).** The scene rebuilds its
  whole World subtree on every place/remove/rotate, never per tick. Ghost
  validity is a Godot-side footprint check signalled by glyph + word +
  colour, never colour alone. Every headless Godot test builds its
  factory through real placement commands.
- 2026-09-08 **Delivery dock, drawn.** Architecture, not equipment,
  following UI_UX §2b's occlusion rule (height at the far side, content at
  the near side). No fill gauge — its buffer empties every tick — the
  deck instead shows delivery stock directly.
- 2026-09-08 **Real Placement, ticket 1 (sim).** `World::new` builds an
  empty grid; `Place`/`Remove`/`Rotate` do everything else, and
  `sim::reference_chain` rebuilds the shipped line from real commands as
  the one test fixture every integration test shares. Flow is derived
  from adjacency, recomputed only on placement changes, never per tick.
- 2026-09-08 **Delivery dock, ticket 1 (sim).** The dock is a
  pass-through: it empties into `Economy`'s shared pool every tick rather
  than holding stock, so what's "on the dock" is always one number in one
  place. `CollectOutput` is removed — a supplied factory earns money with
  no player action.
- 2026-09-08 **Contracts, ticket 2 (clipboard).** One board, one paper
  docket per contract, money and the shared delivery pool shown once above
  them, per-contract facts on each docket. Past a capped height the stack
  scrolls.
- 2026-09-08 **Milestone 7, ticket 2 (aging rack, since deleted).** The
  rack itself is gone (ageing isn't a mechanic this game wants), but the
  progress-gauge rules it proved survive: a bar recomputed from
  `current_tick()` every frame, emissive on this camera, sparse — only
  processes an order of magnitude longer than a craft cycle get one.
- 2026-09-08 **Contracts, ticket 1 (sim).** Several contracts run
  concurrently over ONE shared delivery pool; a settling contract consumes
  its mix so nothing is ever credited twice. File order is the scarcity
  tiebreak.
- 2026-09-08 **Visual overhaul r2, ticket 3 (junctions).** On this fixed
  camera only two faces of a tile are ever seen, so an outlet is a doorway
  carved through the camera-facing wall and an inlet is a hooded housing
  cut open on that same side — cutting the *true* inlet face would be
  invisible (UI_UX §2b point 11).
- 2026-09-08 **Shop, ticket 1 (sim).** `data/shop.ron` prices every
  buildable kind (a superset of machine kinds — belts and pipes are
  priced too). One money counter on `Economy`; `try_spend` is the only,
  all-or-nothing debit.
- 2026-09-07 **Visual overhaul r2, ticket 1 (FluidTank).** The tank is a
  placed building presenting a constant full head (not its fill
  fraction), so flow stays one-way and an empty tank moves nothing.
- 2026-09-07 **Machines improvement, ticket 4 (presentation).** Machine
  models are keyed by kind with a generic fallback so a missing model is
  visible rather than silently wrong; food-contact surfaces are stainless,
  structure is painted or poured (UI_UX §2b point 8).
- 2026-09-07 **Milestone 6 presentation (pipes).** Pipe geometry comes
  from the polled fluid snapshot, not chunk snapshots — fluid movement
  deliberately dirties nothing. Pipes are stainless steel; a liquid is its
  own material class.
- 2026-09-07 **Machines improvement, ticket 2 (content).** Crusher, mixer,
  and a genuinely 2x2 cooking vat replaced the old cooker/reducer; the vat
  is the only 2x2 kind by design.
- 2026-09-07 **Machines improvement, ticket 1 (footprints).**
  `data/machines.ron` gives every machine kind a footprint; origin is
  always the min-X/min-Y corner, the tile a belt feeds into.
- 2026-09-07 **Visual-polish pass (voxel pipeline + 3D scene).** A
  hand-written `.vox` encoder (no Rust crate can write the format this
  pipeline needs); the `.vox` importer is pinned and vendored. Camera is
  true isometric `(1,1,1)`, fixed. Godot needs two `--import` passes on a
  cold cache.
- 2026-09-06 **Milestone 5 (Economy MVP).** Money and contracts are plain
  fields on a new `Economy` struct owned by `World`, not ECS entities — no
  per-entity multiplicity to justify that yet.
- 2026-09-06 **Milestone 4 (first full chain).** `validate_chain` panics
  on a broken recipe link at load time rather than failing silently at
  runtime.
- 2026-09-06 **Milestone 3 (core loop).** `data/` is top-level; `sim/` has
  zero `gdext` dependency; the wire is JSON strings via `serde_json` to
  minimise the `gdext`-specific surface.
- 2026-09-06 **Milestone 2 (mobile export spike).** Android export works
  end-to-end (install/launch unverified). iOS is blocked in this dev
  environment (no Xcode.app) — stretch/deferred. See ARCH §9a.
- 2026-09-06 **Milestone 1 (scaffold).** Pinned: Godot 4.6.3-stable,
  `gdext` 0.5.5, Rust edition 2024 (MSRV 1.98.1), `hecs` 0.11.1.
- 2026-09-06 **Process and docs.** Conventional Commits enforced by hook +
  CI. The three design docs adopted after a critical review that turned
  every hedged "X or Y" into a firm decision. Orca orchestration adopted
  for milestone-sized work.
