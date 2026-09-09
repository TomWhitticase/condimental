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
    machines.ron           # kind -> footprint (width_tiles, height_tiles).
                           #   Also where per-kind staff/power belong later
    ingredients.ron        # unit price of each RAW ingredient (solid+fluid)
    shop.ron               # price per BUILDABLE kind: every machines.ron kind
                           #   plus belt/pipe tiles. Must cover every kind
    contracts.ron          # contract board: id, name, mix, cadence, payout.
                           #   List order = priority when contracts compete
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
      ingredients.rs       #   IngredientCatalog + OwnedIngredients; owns
                           #     raw_ingredient_ids (the one definition of raw)
      shop.rs              #   ShopCatalog + OwnedEquipment. Money is NOT here
      contracts.rs         #   ContractRegistry
      dock.rs              #   Dock: pass-through into Economy every tick
      economy.rs           #   Economy: money (Fx), Vec<ActiveContract>, one
                           #     shared delivery_buffer, STARTING_MONEY
      world.rs             #   hecs::World of placed things. World::new builds
                           #     NOTHING. Place/Remove/Rotate; topology
                           #     recomputed only on those, never per tick
      save.rs              #   RON snapshot save format; no filesystem code
      reference.rs         #   sim::reference_chain: the shipped layout built
                           #     from Purchase+Place commands; test fixture
    tests/                 # headless integration tests, one file per concern
                           #   (core_loop, placement, machine_footprints,
                           #   fluid_production, ingredients, economy, dock,
                           #   save_load, shop, auto_fill; common/mod.rs
                           #   keeps hoppers/tank fed for long runs)
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
                           #   clipboard, market, pause menu
    sim_core.gdextension   # checked in; points at gitignored bin/
    addons/                # vendored, pinned .vox importer (see VENDORED.md)
    test_*.gd              # headless proofs, each prints its own success
                           #   marker; test_layouts.gd is the shared fixture
                           #   (builds factories through real commands)
    test_screenshot.gd     # NOT headless: renders the scene to a PNG
  scripts/
    lint-commit-msg.sh     # commit-msg hook
    verify-scaffold.sh     # milestone-1 headless check
    verify-core-loop.sh    # cargo test + every headless Godot proof
    build-voxels.sh        # regenerate .vox files + review PNGs
    screenshot-scene.sh    # render main.tscn to PNG (flags: --populate,
                           #   --contracts, --stock, --hotbar N, --no-hud,
                           #   --closeup, --first-person, menu/pause)
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

- 2026-09-09 **Hopper/tank auto-fill, ticket 1 (sim).** Auto-fill runs as
  tick step 0, before tank discharge and hopper metering, so a topped-up bin
  feeds in the same tick. One fill helper per kind (`World::fill_hopper_entity`
  / `fill_tank_entity`) serves both the tick path and the manual command:
  clamp to owned stock, then to buffer room, then debit exactly what `fill`
  reports. Auto-fill spends STOCK, never money; `BuyIngredient` stays an
  explicit action. An empty larder stops top-ups silently and the flag stays
  on. A tank auto-fill asks only for `whole_unit_room` (floor); the manual
  command still rounds a part unit up. A hopper top-up dirties its chunk; a
  tank top-up does not (tank level is poll-only). `SetHopperItem` added
  (refuses non-raw; same item = no-op keeping contents); `CycleHopperItem`
  kept only until ticket 2 deletes it with `main.gd`'s Q binding. The flag
  survives a retarget and a `Rotate`, dies with `Remove`. Save
  `schema_version` stays 1 (`#[serde(default)] auto_fill`).
- 2026-09-09 **Save/load, ticket 2 (Godot).** `game_session.gd` autoload
  carries the New Game/Load choice across `change_scene_to_file`;
  `pending_action` is sticky (headless tests call `_ready()` twice), default
  `new_game`. All save-file rules live there: `user://saves/<slot>.ron`,
  listing, whitelist sanitizer (letters, digits, space, `-`, `_`; else `_`;
  nothing left = refuse), reserved `autosave` slot. A save is proved to load
  (throwaway `SimBridge.load_ron`) before the scene changes; failures show
  `load_ron`'s message on the menu. ESC closes what is open first, then opens
  the pause menu; build mode counts as open. Pause clears `_tick_accum` and
  stops the autosave clock. Autosave: 3-minute real-time timer plus
  quit-to-menu, quit and window close; a manual save named `autosave` is
  refused out loud. `auto_accept_quit` is OFF; `main.gd` handles
  `NOTIFICATION_WM_CLOSE_REQUEST` (autosave, then quit). Flagged: no save
  delete, no overwrite confirm.
- 2026-09-09 **Save/load, ticket 1 (sim).** A save is a full state SNAPSHOT,
  not a command log (ARCH §4 amended). Bar: running-state-identical, pinned by
  a 2600-tick lockstep test comparing every snapshot every tick. `Fx` stored
  as raw bits (`FxBits`), never decimal; `fixed`'s serde feature stays off.
  Load rebuilds structure via real `Purchase`+`Place` (a refused `Place` =
  corrupt save error), then writes live state through restore-only
  `pub(crate)`/`pub(super)` methods — never public API. Rebuild is funded
  per item (Fx saturates ~32 767); saved money is installed last. Belt
  `frontier` and empty inventory slots ARE saved; recipes, capacities, pipe
  network shape and the dirty set are derived, not saved. No file I/O in
  `sim_core`. `SimBridge::load_ron` returns `""` on success or a reason; on
  failure the live world is untouched; on success every chunk is dirty.
  `new_game()` exists for symmetry. A contract board that differs from the
  save in either direction is an error. `schema_version` = 1.
- 2026-09-09 **Tuning pass after a playtest.** Crusher = 80 ticks (mixer 90,
  vat 240). Rule: crushing is quickest, the cook slowest; change the numbers
  and the prose in `recipes.ron`, `machines.ron` and this log together.
  Content changes never bump `schema_version`. Lesson: a fixture that stops
  supplying a factory has not stopped it (the economy test now removes the
  dock after the first unit). Dock is 2x2, extra tile spent on DEPTH (UI_UX
  §2b point 16); price stays 70 — price tracks function, not tiles; only
  infrastructure is priced per tile. Dock model re-laid at 64x64, despatch
  wall starts east of the far tile's hood, four pallet positions. UI is 1.5x:
  `UI_SCALE` and `UI_FONT_BASE` (15) in `main.gd` with `_ui_px/_ui_i/_ui_v/
  _ui_font`, NOT `content_scale_factor` (3D→screen mappings would need the
  factor divided out). Panels cap against `_hotbar_reserved_height()`.
  `project.godot` window is 1600x900; at 1280x720 the build grid scrolls a
  little, by design. Hotbar socket = 0.88 of the scale.
- 2026-09-08 **Chain rework, ticket 2 (Godot/assets).** A hopper's contents
  are a property of the PLACEMENT (Q cycles while the ghost is up; prompt
  names it; ghost carries a tinted `hopper_fill` slab), not separate catalog
  entries (fixed hotbar slots must not renumber). One sim change:
  `IngredientsSnapshot.hopper_items` = `World::raw_items`. Buying stock is
  its own panel on M, mirroring the build menu; one click buys a crate of 10.
  A permanent larder strip sits above the hotbar; zero reads "none". Mixer is
  a horizontal ribbon blender, 32x64 voxels (footprint × tile scale). A fluid
  junction is ONE nozzle helper (`machine_fluid_port`) on both faces; the
  vat's belt gallery/doorway were deleted. `pipe_inlet` art applies only when
  the tapping machine is south-adjacent; else plain tile. `item_sugar` is a
  sack. `ITEM_MODELS` keeps unreachable `ketchup_mix`/`ketchup` entries.
  `screenshot-scene.sh --no-hud` added.
- 2026-09-08 **Splitter, ticket 2 (Godot/assets).** Drawn as a machined
  diverter table in the belt run; generator lives in `assets.rs`'s conveyor
  section (the `machine_` prefix is only for `MACHINE_MODELS` keying).
  Next-output readout is an arrow inlaid FLUSH in the deck: both grooves
  always cut, one emissive slab lit. One arrow asset, branch = rotate 90°
  clockwise (same as `Direction::rotate_cw`). The arrow shows where the item
  will ACTUALLY go: a `null` output hands to the other side; neither → arrow
  hidden. A splitter is CONVEYOR to `_belt_asset_for`. Items are hidden per
  TILE from the tile's own count, not per render pass (multi-chunk bug).
  `screenshot-scene.sh --populate` applies dirty chunks every tick. `packer`
  and `dock` are keyless (`SHOP_HOTKEY_SLOTS`), asserted by `test_hotbar.gd`.
- 2026-09-08 **Splitter, ticket 1 (sim).** `direction` = straight output;
  branch = one quarter turn clockwise, derived not stored, so one `Rotate`
  turns both. Accepts from any side (1x1 rule). Round-robin with same-tick
  fallback; the cursor advances on whichever side TOOK the item. Buffer is an
  ordinary one-tile `BeltSegment`; `Splitter` holds only the cursor. Own
  kind in `machines.ron` and `shop.ron`, own `BodyKind`;
  `Topology.splitter_outputs` and belts share one `resolve_sink` closure.
  Priced 25, listed with infrastructure.
- 2026-09-08 **Chain rework, ticket 1 (sim).** `Recipe.fluid_outputs` mirrors
  `fluid_inputs`; `fluid.rs` gained `can_accept_at`/`deposit_at`. A fluid
  output leaves by the FACING face; a fluid input may come from any OTHER
  pipe touching the footprint; the output tap is resolved first and excluded
  from input choice (a loop touching both sides = outlet only). Fluid output
  gated at craft completion. `recipes.ron` `schema_version` = 3 (v2
  rejected). `validate_chain` recognises a fluid link. A machine's inlet face
  is as long as its footprint, so N solid ingredients need an inlet face ≥ N
  tiles (a 1x1 mixer deadlocked); the mixer is 1x2. `World::raw_items` is a
  `Vec` from `ingredients::raw_ingredient_ids` (consumed, never produced).
  `Place.item` is optional while inferable, required (silent refusal) once
  ambiguous; non-raw refused. `data/ingredients.ron` + `sim/ingredients.rs` +
  `BuyIngredient` are `shop.ron`/`shop.rs`/`Purchase` with one word changed.
  Fill debit rule: clamp to owned, let `fill` clamp to capacity, debit what
  `fill` added. A tank is filled with the fluid of the run it discharges
  into; a run nothing taps cannot be filled. `validate_covers` is a test, not
  a construction panic. Overproduction is a real loss; tests pin the
  per-unit margin, not "a running factory profits". Reference chain: one
  feed lane per solid input, machine-to-machine pipe runs are 2 tiles, the
  tank-fed run 4; `reference_chain_with_budget` seams. Aging rack and
  `tests/timed_processes.rs` deleted.
- 2026-09-08 **Inventory hotbar + starting balance.** `STARTING_MONEY` =
  3000 (> 2.5x the 1087 reference-chain bill); `tests/shop.rs` pins the
  relationship, not the literals. Tests calibrated on 200 were rewritten to
  reasons that still hold (small fixtures by choice; the vat needs its own
  world because of a footprint collision). Hotbar = `shop.ron` order, FIXED
  slots; owned-0 draws as an empty socket. Number keys select in any mode
  (same key again deselects); SHIFT+number buys ONLY with the build menu
  open. Selection is signalled four ways (scale, hazard-yellow frame, caret,
  bright digit) plus a fading name flyout; growth is a scale, not a position;
  selection is derived from `_build_kind`, never stored. Hotbar plate nearly
  opaque; build menu centred above the strip. `--stock`/`--hotbar N` added.
- 2026-09-08 **Real Placement, ticket 2 (Godot).** The scene rebuilds its
  whole World subtree on every place/remove/rotate, never per tick. Rotation
  = Y rotation of `index * 90` along `Direction::rotate_cw` order; the encoder
  centres models on their footprint. Junction art stays east-only; other
  facings fall back to the plain tile (tested). Build-menu slot: left-click
  buys, right-click places. Ghost validity is a Godot-side footprint check
  signalled by glyph + word + colour (✔ PLACE / ✗ BLOCKED / ✗ NONE OWNED);
  the tint MULTIPLIES over vertex colours. Remove is a MODE (X) with a red
  prompt even when valid. R rotates what is under the cursor. Every headless
  Godot test builds its factory through real commands (`test_layouts.gd`).
- 2026-09-08 **Delivery dock, drawn.** Architecture, not equipment; layout
  by the occlusion rule UI_UX §2b point 16. No fill gauge (the buffer empties
  every tick); the deck shows one case per unit of `delivery_stock`, capped.
  The packer gained an outfeed; the dock is the only kind with none. `C` and
  `collect_output` removed everywhere; `test_interaction.gd` asserts it.
- 2026-09-08 **Real Placement, ticket 1 (sim).** `World::new` builds an
  empty grid; `Place`/`Remove`/`Rotate` do the rest. `Direction::East` is the
  regression anchor; `sim::reference_chain` rebuilds the shipped line from
  commands and every integration test uses it. A rotated footprint keeps its
  origin corner and swaps extents (`machines.rs` doc). `direction` = outlet
  face; opposite edge = inlet; flanks accept nothing. Fluids connect by
  TOUCHING any footprint tile; a pipe's `direction` is presentation-only.
  Topology is a full single-threaded rebuild on Place/Remove/Rotate, never
  per tick; pipe volumes carried over by tile. A belt is one tile; the
  belt-to-belt hop spawns on the receiver before taking from the sender.
  Hopper, tank and dock are ordinary kinds, any number of each; `FillHopper`
  /`FillFluidTank` take an optional `tile`. Kind→recipe is 1:1 (two recipes
  on a kind panics). `Remove` destroys contents; `Rotate` keeps them. Wire:
  `world_layout_json` replaced `chain_layout_json`; snapshots are arrays and
  carry `direction` strings.
- 2026-09-08 **Delivery dock, ticket 1 (sim).** `dock` kind (priced 70).
  The dock is a PASS-THROUGH: `ship_into` empties it every tick into
  `Economy`'s pool, `DockSnapshot` has no `fill`. `CollectOutput` REMOVED
  (unknown commands are silent no-ops). `DOCK_CAPACITY` = 32 per item so
  `try_absorb` can refuse (mass conservation). Multi-item `Inventory`. Reuses
  `try_transfer_machine_output_to_belt`. Wire: `dock_snapshot_json`, `docks`
  on chunk snapshots, `"dock"` from `query_tile_json`.
- 2026-09-08 **Contracts, ticket 2 (clipboard).** One board, one docket per
  contract (paper tone + steel hairline). Money shown once; cadence, payout
  and delivered/missed per docket. One "on the dock" line for the shared
  pool. Overflow is a capped scrolling stack; rebuilds guarded on the set of
  contract ids. Tick mark sized to its box.
- 2026-09-08 **Milestone 7, ticket 2 (aging rack drawn; rack since
  deleted).** Rules that survive: `MACHINE_PROGRESS_GAUGES` is keyed on kind
  and sparse (short cycles get no bar); a progress bar is recomputed from
  `current_tick()` every frame, not from a snapshot; a gauge on this camera
  must be emissive and show its full-scale mark when empty; oak is the one
  exception to "food contact is stainless" (`palette.ron` `oak`).
- 2026-09-08 **Contracts, ticket 1 (sim).** `data/contracts.ron` (schema 1,
  three contracts), `sim/contracts.rs`, `Economy` holds `Vec<ActiveContract>`
  with independent clocks over ONE shared pool; a settling contract CONSUMES
  its mix (never credited twice). File order is the scarcity tiebreak. No
  `penalty` field until GAMEPLAY §11 decides one. The contracts-vs-recipes
  check is a test, not a construction panic. `economy_snapshot_json`:
  `contracts` array + `delivery_stock`; per-contract `progress` is clamped.
- 2026-09-08 **Visual overhaul r2, ticket 3 (junctions).** Outlet = doorway
  on the +X face; inlet = hooded housing cut open on the camera-facing side
  (UI_UX §2b point 11). Openings must be genuinely empty voxels — asserted by
  `cargo test -p voxgen`. `belt_inlet`/`belt_outlet` variants chosen per tile
  from the layout, +X only. The vat's inlet is widened to an unoccluded
  corner. `MAX_TICKS_PER_FRAME` clamps catch-up so two screenshot runs are
  byte-identical. Pipe run's downstream end terminated.
- 2026-09-08 **Shop, ticket 1 (sim).** `data/shop.ron` is its own file
  (buildable kinds ⊃ machine kinds); `validate_covers` errors on an unpriced
  machine kind. One money counter on `Economy`; `Economy::try_spend` is the
  only debit and is all-or-nothing. Owned counts are a `BTreeMap` field on
  `World`, not an `Inventory`. `Purchase` is a silent no-op on any failure
  and dirties no chunk. Ticket 2 built the build menu (B, later E).
- 2026-09-07 **Visual overhaul r2, ticket 1 (FluidTank).** The tank is a
  placed building with a footprint, kept OUT of `FluidNetwork::segments`; one
  `equalize` function moves tank→segments[0] and pipe→pipe. It presents
  CONSTANT full head (not its fill fraction) so flow is one-way and an empty
  tank moves nothing. `is_source` now marks only the tile the tank connects
  to. Capacity 120, a Rust constant like `HOPPER_CAPACITY` (64).
- 2026-09-07 **Machines improvement, ticket 4 (presentation).**
  `MACHINE_MODELS` keyed on kind with `machine` as fallback. A machine node
  sits at its footprint CENTRE; model size = footprint × 32 voxels. Hopper
  fill reuses the one-voxel-slab mechanism over `bulk_pale` (matte). Hover is
  footprint-aware. Material rule: food contact is stainless, structure is
  painted or poured (UI_UX §2b point 8). Stale `ITEM_MODELS` entries are only
  visible in a rendered frame (§2b point 12).
- 2026-09-07 **Milestone 6 presentation (pipes).** Pipe geometry comes from
  `fluid_snapshot_json`'s `segments`, not chunk snapshots (fluid movement
  never dirties a chunk). `palette.ron` has a `liquid` material class; fluid
  hue is decided in GDScript. Pipes are stainless steel. The fluid model is
  one voxel tall, scaled to fill. Godot exits 0 when a `--script` fails to
  parse, so every headless test prints its own success marker.
- 2026-09-07 **Machines improvement, ticket 2 (content).** Crusher, mixer,
  2x2 cooking vat replaced cooker/reducer. Vat is the ONLY 2x2 by design.
  The fluid requirement moved to the mixing stage with no Rust change.
  `schema_version` is not bumped for content.
- 2026-09-07 **Machines improvement, ticket 1 (footprints).**
  `data/machines.ron`; origin = min-X/min-Y corner = the tile the belt feeds
  (stated once in `machines.rs`). A footprint straddling a chunk boundary is
  reported in EVERY chunk it overlaps (consumers de-duplicate by `tile`).
  `World::with_machines` is the test seam.
- 2026-09-07 **Visual-polish pass (voxel pipeline + 3D scene).**
  Hand-written `.vox` encoder (`generators/vox.rs`). Importer pinned:
  qbradq/Godot-MagicaVoxel-Importer-with-Extensions @ `288f1d8` (v1.2),
  vendored; a bump is a reviewed decision. One `.vox` per material class;
  `palette.ron` owns material class; Godot rebuilds one material per class
  from `materials.json`. `cargo xtask` stages models into
  `godot_project/voxels/`. Camera is true isometric `(1, 1, 1)`, fixed. Godot
  needs TWO `--import` passes on a cold cache; vengi's thumbnailer needs a
  real video driver. `test_presentation.gd` + `screenshot-scene.sh`.
- 2026-09-06 **Milestone 5 (Economy MVP).** `sim/economy.rs`: `Fx` money +
  contracts as plain fields on `Economy`, owned by `World` (no ECS). A miss
  advances the cadence boundary (no penalty). `economy_snapshot_json`.
- 2026-09-06 **Milestone 4 (first full chain).** Chain topology came from
  `recipes.ron` list order (since superseded by placement); `validate_chain`
  panics on a broken link. Adding a recipe was verified as a data-only edit.
  `ChunkSnapshot` fields are plural.
- 2026-09-06 **Milestone 3 (core loop).** `data/` is top-level (resolves
  ARCH §11). `sim/` module tree has zero gdext dependency; `lib.rs` is a thin
  JSON layer. Belt gaps are slack beyond a fixed minimum spacing (`belt.rs`
  doc). Wire format is JSON strings via `serde_json`, to minimise gdext
  surface. `recipes.ron` is `include_str!`-embedded.
- 2026-09-06 **Milestone 2 (mobile export spike).** Android export works
  end-to-end; install/launch unverified. iOS blocked in this environment (no
  Xcode.app) → iOS is stretch/deferred, an open question. See ARCH §9a.
- 2026-09-06 **Milestone 1 (scaffold).** Pins: Godot 4.6.3-stable, `gdext`
  0.5.5, Rust edition 2024 (MSRV 1.98.1), `hecs` 0.11.1. `cargo xtask` is
  the build script. Godot needs a one-time `--import` to discover the
  extension (`verify-scaffold.sh`).
- 2026-09-06 **Process and docs.** Conventional Commits (hook + CI).
  ARCHITECTURE.md adopted after a critical review that turned every hedge
  into a decision (fixed-point, `hecs`+`rayon`, pinned versions, headless
  loop). UI_UX.md, GAMEPLAY.md (contracts not spot market; equipment IS the
  tech tree; staff are an abstract pool of four categories; derived sauces
  and variants), docs/README.md, docs/DEVLOG.md (not `@`-included) added.
  Voxel pipeline: `.vox` format, Rust generators as source of truth,
  headless vengi renders. Milestone 5 "Economy MVP" was added to ARCH §9.
  Orca orchestration adopted for milestone-sized work.
