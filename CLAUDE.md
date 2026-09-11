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
    contracts.ron          # contract BOARD (schema 2): id, name, mix, cadence,
                           #   payout, price, strikes, starter. Only starters
                           #   run in a new game; the rest are signed (C).
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
                           #   contract_board, save_load, shop, auto_fill;
                           #   common/mod.rs
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
                           #   clipboard, market, contracts board (C), pause
                           #   menu, and the click-to-configure panel on a
                           #   placed hopper or tank
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
                           #   --contracts, --board, --stock, --hotbar N, --no-hud,
                           #   --closeup, --first-person, --stress,
                           #   --belt-gallery, menu/pause)
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

- 2026-09-11 **Factory floor redesign.** The floor is a BACKGROUND surface
  and is authored by its value range, not its detail (UI_UX §2b points 27,
  28). Its own palette entries — `concrete_slab`/`concrete_mottle`/
  `concrete_shade`, deliberately within eight points of each other — not the
  shared `concrete`, which the dock deck and machine plinths keep. Variation
  is broad ragged blotches on a coarse 8-voxel cell grid, most cells bare;
  NEVER per-voxel speckle (it read as static, and blotches greedy-mesh far
  cheaper: 1 363 384 → 798 272 primitives). **The floor must never hold the
  brightest pixel in the frame** — §4 reserves that for metal — so the
  repair patch is `concrete_shade`, DARKER than the slab, saw-cut in
  `concrete_dark`. Four models (`floor_tile`, `_worn`, `_patch`, `_drain`)
  picked by `main.gd`'s `_floor_variant_for(tx, tz)`, each laid at a
  hash-chosen quarter turn by `_floor_yaw_steps_for` — four stamps repeat
  visibly, rotation fixes it free. **That is why `floor_joint` keeps its cut
  on all FOUR edges**: a rotatable tile's edges must be four-fold symmetric
  or seams step; the doubled interior line was fixed by lightening
  `floor_seam` (`#33332F` → `#50504A`), not by dropping edges. Variant and
  turn are pure functions of (tx, tz) — `_build_floor` runs on every
  place/remove and must not repave. Drains are laid in whole rows
  (`posmod(tz + 4, 9) == 0`, phased off row 0) and are the one model never
  turned; their grating bars stay `concrete` because a grating is the one
  part of this floor that wants contrast. `_floor_node` is a `Node3D` named
  `"Floor"` parenting one `MultiMeshInstance3D` per variant (the name is
  asserted by `test_placement.gd`/`test_presentation.gd`); the whole floor
  stays in the `matte` class because `material_override` is set on it.
  Three voxgen tests pin: nothing stands proud of the slab top (`FLOOR_Y`
  puts everything else on that plane), identical joints on all four edges of
  every rotatable model, drain channel crossing the whole tile on the
  midline. Judged by rendering the EMPTY world, not the fixture.
- 2026-09-11 **UI visualisation pass — one bar language.** A quantity is
  drawn as a bar **only where the simulation defines its ceiling** (hopper
  and tank level, machine cycle, pipe pressure, a contract's countdown
  against its cadence, a price against the balance); a figure with no
  denominator — larder stock, the balance itself — stays a number, because
  a bar against an invented maximum is a gauge that lies. One mechanism:
  `_build_meter(parent, width, fill, track, border, height)` returns the
  TRACK, `_set_meter(track, fraction, fill)` sizes the fill and **records
  `fraction` as node meta** so a headless test can prove a bar moved with no
  viewport. Three fills, each a meaning: `METER_FILL_LEVEL` (green, how
  full), `METER_FILL_PROGRESS` (amber, work under way), `METER_FILL_WARN`
  (red, a cost you cannot meet or a clock nearly out) — always with a word
  or glyph beside it (UI_UX §7). Widget state comes from
  `_hover_panel_state(tile)`, a pure function beside the pure
  `_hover_text_at_tile`. The docket keeps to paper + ink (no third colour,
  UI_UX §5), reads **"8s to go" / "due now"** never ticks
  (`_format_remaining`, ARCH §3.9), carries ✓/✗ tallies, and puts the
  countdown, cadence and both tallies on ONE row — a second row scrolled the
  board at 1600x900 (UI_UX §5c) — with `CONTRACT_METER_MIN_WIDTH` so the
  expanding bar cannot be squeezed to nothing by the longest cadence text.
  Shop/market slots draw a price as its share of money, floored at
  `SHOP_METER_MIN_SHARE` so a cheap thing still draws something, full red
  when unaffordable. The money figure flashes `+N`/`-N` beneath itself on
  every change (`_flash_money_change`, the hotbar flyout's hold/fade; its
  deadline starts one fade in the PAST, since `Time.get_ticks_msec()` is
  small at boot). Every id the player reads goes through
  `_kind_display_name`/`_item_display_name`, never the wire's
  `boxed_ketchup`. Full record in `docs/DECISIONS.md`.
- 2026-09-10 **Contract board: contracts are signed, struck and lost.**
  `contracts.ron` is `schema_version` 2 (v1 rejected): every contract gains
  `price` (≥ 1), `strikes` (≥ 1) and optional `starter`; at least one
  starter. A new game runs ONLY the starters (shipped: one, `corner_shop`);
  `PlayerCommand::SignContract { id }` (`{"type": "sign_contract"}`) buys an
  offer via `Economy::try_sign` — refused silently for an unknown id, an
  offer ALREADY ACTIVE (one of each, never two) or an unaffordable fee. A
  signed contract is inserted at its FILE-ORDER rank (`insert_in_board_order`),
  so priority never depends on signing order. `Economy::tick` returns
  `Vec<ContractEvent>` (`Paid`/`Missed`/`Lost`); a miss bumps
  `consecutive_misses`, a payout zeroes it, reaching `strikes` REMOVES the
  contract that tick and bumps `Economy::losses`; re-signing costs the price
  again and starts a clean record. Losing is the whole penalty (GAMEPLAY §11
  decided). `Economy` owns the board (`board()`); the snapshot gains `board`
  (every offer: `active`, `affordable`, `times_lost`, `starter`…) and per
  active contract `satisfied`, `window_start_tick`, `price`,
  `consecutive_misses`, `strikes`. Save: schema stays 1 (`#[serde(default)]`
  on `consecutive_misses` and `contracts_lost`); the active list is a SUBSET
  of the board, restored exactly (starters cleared first), an unknown active
  id or a duplicate is an error, an unsigned offer is not
  (`MissingContract` deleted). Godot: **C** opens the CONTRACTS BOARD (dark
  plate, paper offer rows, one SIGN button each; E/M/C mutually exclusive;
  `_board_open` joins every modal guard); the docket gains a status line,
  strike pips and a state border (the §5e countdown meter is the time half)
  from the pure `_contract_risk_for` →
  `ready`/`ok`/`risk` (≥ `CONTRACT_RISK_FRACTION` 0.5 elapsed, short)/
  `last_chance` (next miss loses it); a docket keeps to paper, ink, done-
  green and ONE warning red. Loss/signature is announced by diffing active
  ids between polls (`_announce_contract_changes`; first poll only primes);
  an empty clipboard says "press C". Shipped board: 4 rungs; the top one
  exceeds one line's throughput on purpose. Tests: `tests/contract_board.rs`,
  `test_contract_board.gd`; `test_contract_panel.gd` signs every offer first;
  `screenshot-scene.sh --board`, and `--contracts` signs all. Full record in
  `docs/DECISIONS.md`.
- 2026-09-10 **Hopper/tank configuration, ticket 2 (Godot + one sim deletion).**
  A placed hopper or tank is CLICKED and configured on a panel beside it:
  level, FILL (with its reason in words when refused), AUTO-FILL toggle,
  and an item picker that retargets a PLACED hopper through `SetHopperItem`.
  The global keys SPACE (fill hoppers), V (fill tanks) and Q (cycle hopper
  contents) are GONE, and so is `PlayerCommand::CycleHopperItem`; a hopper
  is placed holding the first raw item and changed on its panel. ESC, menus,
  modes and removal close the panel; an unplumbed tank is refused.
  `test_config_panel.gd` proves it; `screenshot-scene.sh --config TX,TY`
  photographs it. Full record in `docs/DECISIONS.md`.
- 2026-09-10 **Pipes & conveyors overhaul, ticket 5 (belt presentation).**
  A belt tile's shape is `_belt_shape_for(links, direction)`, pure: input
  from behind → straight (plus any flank = MERGE); exactly one flank input
  and nothing behind → corner, LEFT if fed from the anticlockwise side of
  its facing, RIGHT from the clockwise side; two flanks → merge; no inputs
  → START (a lone tile is a start); inputs but no output → END. Merge,
  start and end draw the straight family; corners need NO yaw of their own
  (authored output-+X, spun by the facing like everything else). Frame sets
  are `<family>_f0..f3`; `BELT_FRAMES` (4) and `BELT_TICKS_PER_TILE` (50)
  are mirrored in `main.gd`; the frame is `floor(tick*4/50) mod 4` from
  `sim.current_tick()` at the end of `_advance_simulation`, so a pause
  freezes it for free. **Items are drawn face to face**: distance 0 is the
  input face's midpoint, 1 the output face's (they were centre-to-next-
  centre, half a tile downstream); a corner's items follow a quarter circle
  of radius half a tile about the pivot corner, and `_carried_item_transform`
  reads the shape from `_belt_shapes`. **The belt ghost stays straight**
  (the would-be links of an unplaced tile are the sim's to compute; ARCH §4's
  `preview_command` is the home). Hover: `carries east · turns left / fed
  from the west (Hopper) / hands to Crusher`, `not connected` where null.
  The `belt`, `belt_inlet`, `belt_outlet` aliases, models and icons are
  DELETED; `SHOP_MODELS["belt"]` is `belt_f0`; `voxgen`'s `belt()` stays
  as the test reference only. `_belt_asset_for` is gone; `_is_building_tile`
  survives only for the pipe roles (ticket 6). `test_layouts.gd`'s
  `build_belt_gallery` + `--belt-gallery`: both hands of corner and every
  belt-to-building junction at all four facings. At 30 Hz the four-frame
  march is 2.4 frame changes a second; judged by numbers, not by eye.
- 2026-09-09 **Pipe presentation (Pipes & Conveyors ticket 6).** A pipe
  tile's shape, turn and fittings come from `links.connections` ONLY
  (`_pipe_shape_for`, `_pipe_fitting_for`; `_pipe_roles` is gone). Turn
  arithmetic in `DIRECTIONS` indices: an end is authored open west (2), a
  corner on (east 0, north 3), a tee closed south (1); one clockwise turn
  adds one. A straight is ALWAYS turn 0 on an east-west axis and 1 on a
  north-south one, so joint flanges never stack; a pipe's `direction`
  decides only a lonely tile's axis. `pipe_end` carries no joint flange
  (its turn is dictated, so it would stack). Fittings are instances keyed
  `fitting:x,y:side`; fluid bodies stay nodes, one asset per shape
  (`PIPE_FLUID_ASSETS`), `FLUID_END_CAP_TRIM` deleted. Every part of a
  fitting is `PIPE_FITTING_MARGIN` (1.0) of radius larger than anything it
  can cover — voxgen pins it on both faces of a straight. Hover: "connects
  west and north · fed by Fluid Tank · feeds Mixer" (tank = fed by; machine
  in `producers` = fed by, in `taps` = feeds; the neighbour placement names
  the kind). The pipe ghost takes its shape from adjacent PIPE placements
  only, no fitting — reconcile with ticket 5's belt ghost in ticket 7.
  `pipe_source`/`pipe_inlet` deleted (generators, models, icons).
  `test_pipe_shapes.gd` runs on `build_pipe_gallery` (= `--pipe-gallery`);
  a fixture run shows liquid only in a run tapped by a machine whose fluid
  input is RAW (a mixer), since a tank fills from bought stock.
- 2026-09-09 **Pipes & conveyors overhaul, ticket 2 (belt model family).**
  One generator, `belt_shape(shape, phase)`, emits `belt_f0..f3`,
  `belt_corner_left_f0..f3`, `belt_corner_right_f0..f3`; `belt`,
  `belt_inlet`, `belt_outlet` are byte-identical to `belt_f0` until ticket 5
  drops the names. `BELT_FRAMES` = 4, shift = 8 voxels per frame along +X;
  frames differ ONLY on the lane surface, pinned by an exact-shift test;
  idlers/rails/legs never move. Pattern period = one tile (one arrow per
  tile). Direction mark = a SOLID raised arrow in `hazard_yellow` at
  `BELT_ARROW_Z` = `BELT_SURFACE_Z + 1`; a thin chevron became an `L` under
  the camera (UI_UX §2b point 23). Corner = quarter annulus about the pivot
  corner; `Left` fed from north = vox low Y, `Right` is its Y-mirror (built
  by `mirror_y`, never authored twice); pattern coordinate `u` is stretched
  over the quarter turn. **Belt-side junction variants retired**: the plain
  tile carries the junction, machine hoods do the rest. Splitter stubs are
  plain rubber (no arrow beside the lit one). `voxicons` skips `_fN` for
  N ≥ 1. Corners were only render-checked via a temporary `main.gd` hack;
  ticket 5 must re-check them at all four facings once it draws them.
- 2026-09-09 **Pipe model family (Pipes & Conveyors ticket 3, assets
  only).** Shapes `pipe`/`pipe_corner`/`pipe_tee`/`pipe_cross`/`pipe_end`,
  fittings `pipe_fitting_machine`/`pipe_fitting_tank` (authored on +X,
  overlaid by Godot on any open leg, hardware at x ≥ 27 so no leg window is
  covered), one one-voxel fluid body per shape. Face convention: vox LOW Y
  is north (camera-facing); a corner is open on x = 31 and y = 0. The pipe
  axis sits on the tile midline (`PIPE_C` = 15.5, `*_f` primitives), not on
  voxel 16; machine nozzles stay on 16. The fluid body is narrower than the
  bore (`PIPE_FLUID_Y` 14..17) so no face of a full column coincides with
  the tube's outer wall. Junction shapes carry a hub and NO end flange; the
  straight keeps one at low X. A cap is a bright plate with brass bolts,
  never a brass disc. `main.gd`'s PIPE_BORE_* stay 12/7; the end body stops
  before its cap. `pipe_source`/`pipe_inlet` are deprecated compositions
  until ticket 6 deletes them. Voxgen tests pin open faces, body-in-bore,
  window and flange-on-boundary; `VoxModel::rotated_cw` is `rotate_cw`.
- 2026-09-09 **Pipes & conveyors, ticket 4 (instanced rendering).** Belts,
  splitters, pipes and carried items are `MultiMesh` instances: one
  `MultiMeshInstance3D` per (asset, material class) under `World/Tiles`,
  one swap-remove pool per item asset under `World/Items`. Machines,
  buildings, level bodies (hopper/tank/pipe fluid), the splitter arrow, dock
  cases and the ghost STAY nodes: anything with live per-instance state
  (material swap, scale, visibility) is a node. `_asset_for_placement` still
  picks the asset; `_tile_frames_for` wraps it as a FRAME SET and
  `_set_tile_frame` flips per-frame visibility (ticket 5 animates through
  it). Per-tile yaw is supplied by the caller of `_stage_tile_instance`,
  never derived inside. Items are positioned only by
  `_carried_item_transform(tile, direction, distance)`; corners extend it.
  Per-tile item hiding is kept. Tests use `_tile_instance_at`,
  `_carried_item`, `_carried_item_count`, not node names. Stress fixture
  `build_stress_line` (1500 belts) funds itself via `fund` (save/load money
  patch, ceiling ~32 767 = `Fx`); `test_perf.gd` asserts generous ceilings;
  `screenshot-scene.sh --stress` prints `SCREENSHOT_RENDER` counts. The
  remaining per-frame cost is chunk-snapshot JSON parsing (wire, milestone
  8), not the scene.
- 2026-09-09 **Pipes & conveyors overhaul, ticket 1 (`links` on the wire).**
  Every belt, splitter and pipe in `world_layout_json` (and its
  `query_tile_json` variant) carries `links`; every other kind OMITS the key
  (`skip_serializing_if`), so its JSON is byte-identical to before. Shape is
  `docs/PIPES_AND_CONVEYORS_PLAN.md` "The wire contract": belt/splitter
  `{inputs: [{from, kind}], output: {to, kind} | null}`, pipe
  `{connections: [{to, kind}]}`; directions absolute, named from the tile
  toward the neighbour; entries in north/east/south/west order (at most one
  thing per side, so the order is geometry, not walk order). Recorded ONCE
  in `World::recompute_topology` as `Topology.links`, at the moment each
  relation is resolved — never a second adjacency scan, never per tick;
  `world_layout`/`query_tile` look it up. A lonely belt still has
  `{inputs: [], output: null}`. A splitter's `output` is its straight side
  only; the branch stays in `outputs`. A pipe lists a machine or tank on
  the ONE tile the sim taps (`machine_taps`/`machine_output_taps`/
  `tank_discharges`), never on every touching tile. Found and deliberately
  NOT changed (ticket 7): `resolve_sink` links a belt into a machine or
  dock when the tile it points at is on the inlet EDGE, from any side — a
  1x1 machine is fed from all four sides, including through its outlet
  face; `test_layouts.gd`'s demo line depends on it. `tests/links.rs` pins
  the current rule so a fix shows up there.
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
