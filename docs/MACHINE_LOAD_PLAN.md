# Machine Load Indicator — plan

Status: **done** (started and finished 2026-09-11; all three tickets
landed). Integration branch:
`TomWhitticase/machine-load-indicator`, worktree
`orca/workspaces/sauce-factory/sheepshead`. Built as one Orca orchestration
Run; the three tickets below map 1:1 to Tasks.

## Goal

The player asked: *"when machines are fully loaded and working at full
capacity I want to be able to see it, and when they aren't I also want to
see it, so I can easily see the bottlenecks in my sauce factory."*

Deliver, for every recipe-running machine (crusher, mixer, cooking vat,
bottler, packer and any future kind; NOT the hopper, tank, dock, belts,
splitters or pipes):

1. A **load** figure the simulation owns: what share of recent time the
   machine spent working, waiting for input (starved) or waiting on its
   output (blocked).
2. A **status** derived from it — `working` (flat out), `starved`,
   `blocked` — and the **reason** it is waiting, as a fact Godot can phrase.
3. An **in-world indicator** readable at a glance across the whole factory:
   an industrial *andon* stack light beside each machine, one lens lit.
4. **Hover** words: the status, the percentage and the reason.

Reading the factory then works the way it does on a real line: the machine
whose light is green while the lights after it are amber and the lights
before it are red is the bottleneck.

## Why it is shaped this way

- **The sim owns the measurement** (ARCHITECTURE.md §1 hard rule: anything
  deterministic lives in Rust; §5: Godot never derives a fact the sim
  knows). A rolling window in GDScript would be gameplay logic in the
  presentation layer, would not survive a save, and could not be tested
  headlessly in Rust.
- **A window, not an instantaneous flag.** `Machine::tick` clears `process`
  on the tick a batch is deposited and starts the next on the tick after,
  so a saturated machine is "idle" at the end of one tick in every cycle.
  An instantaneous readout flickers; a window over the last ten seconds
  does not.
- **Integers only** (ARCHITECTURE.md §3.9). Tick counts in buckets; the
  percentage is computed with integer arithmetic; no `Fx` is needed.
- **Polled, never dirtied.** Load changes every tick on every machine; a
  per-tick dirty would defeat ARCHITECTURE.md §4's per-chunk delta rule.
  Same treatment as fluid movement and tank auto-fill: a light polled
  endpoint (`machine_load_snapshots_json`).
- **The indicator is a stack light**, the real-world signal for exactly this
  question (UI_UX.md §3: real industrial visual language; §4: emissives as
  the readability signal; §7: a lit lens is colour PLUS position in the
  stack, never colour alone). It is drawn in world space at the corner the
  camera can see (§2b point 11) and never rotated with the machine, because
  a lamp on a rotated face is a lamp nobody sees (§2b point 18).
- **Words in hover, not always-on text** (UI_UX.md §5): the number and the
  reason live in the hover panel every other readout already uses.

## Ground rules for every ticket

- Follow CLAUDE.md and the three design docs. Read `docs/DECISIONS.md`
  Part B (UI_UX §2b long form) before drawing anything.
- The sim never learns about art. Godot never derives a fact the sim
  already knows — in particular Godot never re-derives `status` from the
  counts; it reads the sim's `status` string.
- Verify: `cargo test --workspace`, `scripts/verify-core-loop.sh`, and for
  anything visual `scripts/screenshot-scene.sh` — then READ the PNG.
- `cargo` is at `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo`
  (not on PATH). After `cargo xtask`, run
  `godot --headless --path godot_project --import` TWICE or the scene draws
  no meshes. Godot rewrites `godot_project/project.godot` (comment loss):
  never stage that. An untracked `AGENTS.md` may appear: leave it.
- Conventional Commits; a DEVLOG entry per ticket; a condensed rule in
  CLAUDE.md's decisions log plus the full reasoning in `docs/DECISIONS.md`;
  a new UI_UX §2b point for any presentation lesson, numbered after the
  last one present.
- Do not merge into the integration branch yourself. Commit on your worker
  branch and report it in `worker_done`; the coordinator merges.

## The wire contract (ticket 1 builds it; ticket 3 consumes it)

### The load window (sim constants, `sim_core/src/sim/machine.rs`)

```rust
pub const LOAD_BUCKET_TICKS: u32 = 30;   // one bucket = half a second
pub const LOAD_BUCKETS: usize = 20;      // ring of 20 → a 600-tick window
pub const FULL_LOAD_PERCENT: u32 = 95;   // working share at or above this
                                         // is "working" (flat out)
```

Every tick, `Machine::tick` records exactly one `WorkState` for the tick:

| Situation at entry to `tick` | What happened | Recorded as |
|---|---|---|
| `process == None` | a craft started | `Working` |
| `process == None` | nothing started (input or fluid short, no tap) | `Starved` |
| `process == Some`, not complete | timer advanced | `Working` |
| `process == Some`, complete | output deposited, process cleared | `Working` |
| `process == Some`, complete | held (output full, pipe full, no outlet) | `Blocked` |

Consequence, and the reason the completion tick counts as work: a machine
that is fed and drained without interruption reads **exactly 100 %**, tick
after tick, with no one-tick dip per cycle. A test pins this.

The ring: the current bucket accumulates until it holds `LOAD_BUCKET_TICKS`
ticks, then the cursor advances and the bucket it lands on is cleared.
Totals are the sum over all buckets (including the partial current one);
`observed = working + starved + blocked` (≤ 600). `status`:

- `observed == 0` → `starved` (nothing has happened yet);
- `working * 100 >= FULL_LOAD_PERCENT * observed` → `working`;
- else `starved` if `starved >= blocked`, else `blocked`.

### The stall reason (computed at snapshot time in `world.rs`, not per tick)

A structured fact, phrased by Godot:

```json
"stall": null
"stall": {"kind": "input",         "item": "sugar"}      // solid input short
"stall": {"kind": "fluid_input",   "item": "vinegar"}    // tap exists, cannot supply
"stall": {"kind": "no_supply_pipe", "item": "vinegar"}   // recipe wants fluid, no tap
"stall": {"kind": "output",        "item": "ketchup"}    // finished, solid output full
"stall": {"kind": "fluid_output",  "item": "ketchup"}    // finished, outlet pipe cannot accept
"stall": {"kind": "no_outlet_pipe", "item": "ketchup"}   // finished, recipe deposits, no tap
```

`stall` describes THIS tick's situation, regardless of the window's
`status` (a machine may be `working` overall and momentarily short of
sugar). Only the first shortfall found is reported, in recipe order,
solids before fluids.

### On the wire

`MachineSnapshot` (chunk snapshots and `query_tile_json`) gains:

```json
"load": {"working": 540, "starved": 60, "blocked": 0, "observed": 600,
         "window": 600, "percent": 90, "status": "starved"},
"stall": {"kind": "input", "item": "sugar"}
```

A new `SimBridge::machine_load_snapshots_json()` returns an **array** of
`{"tile": [x, y], "load": {...}, "stall": ...}` — one per machine, keyed by
origin tile, in the topology's stable order — and is what Godot polls. It
carries nothing else, so polling it every few ticks is cheap.

Save: `SavedMachine` gains the ring (`#[serde(default)]`, `schema_version`
stays 1, like `auto_fill`). The 2600-tick lockstep test compares chunk
snapshots, and `MachineSnapshot` now carries `load`, so a load that failed to
survive a save/load shows up there without a new test.

## The asset (ticket 2 builds it; ticket 3 places it)

An **andon stack light**: a slim painted post (`matte`) with a stacked
housing of three lenses — from the top down **green, amber, red** — the
industrial convention (green run, amber waiting, red fault/blocked). Three
models, one per lit lens:

- `andon_working` — green lens `status_green` (`emissive`); amber and red
  lenses present but dark (a matte, low-value version of their colour, so
  the tower reads as a tower and the lit lens reads by POSITION as well as
  colour, UI_UX §7).
- `andon_starved` — amber lit, others dark.
- `andon_blocked` — red lit, others dark.

Rules:

- Authored on a 1x1 tile (32x32 footprint) so `vox::encode` centres it on a
  tile; the post stands in the tile's **camera-visible corner** (+X, low Y in
  model space — UI_UX §2b point 11) inset so that no face of the post is
  coplanar with any exposed face of any machine body at any of the four
  facings: `cargo test -p voxgen` proves it by rotating every machine body
  four ways and checking the post's column. The post may pass THROUGH a
  body (an andon pole bolted to the machine's corner); it may not z-fight
  with one (point 24).
- The lens stack sits **above the tallest machine body** so it is never
  occluded; a voxgen test pins `lens_bottom > max body height`. If the
  render shows a single tall post looking absurd beside a 1x1 crusher, the
  fallback is two heights (`andon_<status>_short` for 1x1 footprints,
  `_tall` for larger) chosen by Godot from the footprint — decide by the
  render, record the decision.
- Lenses are visible from EVERY side (a ring, not a face), because the post
  is never rotated and a lens facing away is a lamp nobody sees.
- Lenses read at scene scale: at least 4 voxels wide and 3 tall each, with
  a dark gap between them, same value-contrast reasoning as point 11.
- No catalogue icon (extend `icons.rs`'s skip rule as `is_motion_overlay`
  does). A `--populate` scene render with the light visible at scene scale
  is the review; `scripts/build-voxels.sh` regenerates and previews.

## Presentation (ticket 3)

- `_build_placement` (the machine branch of `match kind`) stands the three
  andon nodes at the **world-space camera-visible corner tile** of the
  footprint, yaw 0, only one visible. Which corner is a function of
  `CAMERA_DIRECTION`, derived once as a constant with a comment, not
  guessed. Siblings of the body (like the motion overlays) so
  `body.get_node_or_null("emissive")` keeps finding the machine's own lamp.
- Poll `machine_load_snapshots_json()` every `LOAD_POLL_TICKS` (30) ticks
  from `_advance_simulation` (so pause freezes it, like belt frames); on
  every rebuild and load, poll once immediately so a loaded factory shows
  its lights on frame one. Flip lens visibility only when the status
  changes. Keep `_machine_load[key]` for hover.
- Hover (`_machine_hover_text`) gains one line, in this language:
  `flat out · 100% of the last 10s` / `starved · 38% busy · waiting for
  sugar` / `blocked · 61% busy · output full`. Reasons in words, item ids
  through `_item_display_name`: `waiting for {item}`, `waiting for {fluid}
  in the pipe`, `no supply pipe for {fluid}`, `output full`, `outlet pipe
  full`, `no outlet pipe`. The "10s" comes from `window / TICK_HZ`, never a
  literal. The hover panel (`_hover_panel_state`) may add a load meter in
  the existing meter language if it fits without crowding the progress bar;
  words are mandatory, the bar optional.
- The fixture: `test_layouts.gd` gains `build_bottleneck_line` — the
  reference line with hopper/tank auto-fill on, ticked long enough (≥ 1500
  ticks) that the cooking vat (240-tick cook) is the bottleneck: crusher
  and mixer `blocked`, vat `working`, bottler and packer `starved`.
  `test_machine_load.gd` proves: three andon nodes per machine and none on
  buildings/tiles; one lens visible and it matches the sim's `status`;
  status changes flip the lens; pause freezes polling; hover text carries
  status, percent and reason for each of the three cases; a loaded save
  shows lights on the first frame. `scripts/screenshot-scene.sh
  --bottleneck` renders the fixture; READ the PNG at default zoom — the
  three lens colours must be tellable apart and the lit lens must be
  identifiable by position at that zoom, or the asset goes back to
  ticket 2's rules.

## Tickets

| # | Ticket | Layer | Depends on |
|---|---|---|---|
| 1 | Load window, status, stall reason, wire, save | `sim_core` | — |
| 2 | Andon stack-light models | `voxgen` | — |
| 3 | Stack lights in the scene, polling, hover, fixture, screenshot flag | Godot | 1, 2 |

Tickets 1 and 2 run in parallel. Ticket 3 starts from the integration
branch once both are merged. The coordinator verifies the union
(`scripts/verify-core-loop.sh` on the merge, not on the branches), writes
the condensed decision entry, and merges to `main`.

## Decisions made by tickets (read before consuming their output)

*(appended by the coordinator as tickets land)*

### Ticket 1 (sim) — landed 2026-09-11

- **The wire is exactly as written above.** `load` keys: `working`,
  `starved`, `blocked`, `observed`, `window`, `percent`, `status`; `stall`
  is `null` or `{kind, item}` with the six kinds spelled as listed. The
  same two objects ride `chunk_snapshot_json`'s `machines` entries and
  `query_tile_json`'s `"machine"` tile (the tile variant did not go through
  `machine_snapshot`, so it was given the fields explicitly). Rust names:
  `sim::WorkState`, `sim::StallKind`, `sim::MachineLoadSnapshot`,
  `sim::MachineLoadEntry`, `sim::LOAD_WINDOW_TICKS` (600).
- **`observed` is not `window`, even on a machine that has run for
  minutes.** The ring clears a whole bucket the tick after it fills, so a
  warmed-up window observes between 571 and 600 ticks. Use `window` for
  the "of the last 10 s" wording (`window / TICK_HZ`), and `percent` /
  `status` for the verdict; never compare `observed` to `window`.
- **`stall` is a per-tick fact and is `null` more often than not on a
  blocked machine.** A blocked crusher is mid-craft on most ticks (its
  batch is not yet complete, so it is not yet "held"), and reports a stall
  only on the ticks a finished batch is being held. Hover should keep the
  last non-null reason while `status` is unchanged rather than flickering
  between "output full" and nothing, or say only the status when the reason
  is `null`. Likewise a `working` machine may momentarily report
  `{"kind": "input", ...}` on a deposit tick.
- **The bottleneck fixture needs ~8000 ticks, not 1500.** With 8-slot
  buffers and a belt or pipe run between every pair of machines, the
  shipped line backs up at the vat's pace: the vat reads `working` from
  about tick 1500, the mixer `blocked` from about 3500, the crusher
  `blocked` from about 7000; the bottler and packer are `starved`
  throughout. `build_bottleneck_line` should tick ≥ 8000 (with hopper and
  tank auto-fill on) before a screenshot expects red–red–green–amber–amber.
  `sim_core/tests/machine_load.rs` settles at 8000 and asserts over a full
  window after that.
- **Load changes never dirty a chunk.** Poll `machine_load_snapshots_json`;
  do not wait for `dirty_chunks` to redraw a lens. A test pins this.
- **Save:** `SavedMachine.load` (`#[serde(default)]`, schema 1). A save
  from before this ticket loads with every window empty (`status`
  `starved`, `observed` 0) until the machines have ticked; a loaded save
  from after it shows its lights on frame one.
### Ticket 2 (assets) — landed 2026-09-11

- **Model names (six, two heights):** `andon_working_short`,
  `andon_starved_short`, `andon_blocked_short` for a **1x1** footprint;
  `andon_working_tall`, `andon_starved_tall`, `andon_blocked_tall` for
  **anything larger** (the 1x2 mixer, the 2x2 vat, any future kind). Ticket
  3 picks by footprint. There is no plain `andon_<status>`.
- **Why two heights:** rendered beside a crusher, a vat and a packer. At the
  single height the plan proposed (lens bottom 48, clearing the vat's stack
  at z 47) the post is 64 voxels — two and a half crushers tall, the stack
  floating a whole machine above the body. The short stack reads as bolted
  to the machine. Cost, stated: a short stack's red lens (z 32..35) can be
  clipped by a vat standing directly camera-side of and adjacent to a 1x1
  machine; amber and green cannot be, and the tall stack never is.
- **Post:** 3x3 voxels, `paint_grey` (new palette entry), at the tile's
  +X/low-Y corner **inset 1 voxel** (`ANDON_INSET`): column x 28..30,
  y 1..3 in model space. It runs from z 0 (through the plinth) to the
  collar. Never rotate it; place it at yaw 0 on the camera-visible corner
  tile of the footprint.
- **Lenses:** rounded 5x5 rings (`cylinder_z_f` radius 2 about (29, 2)),
  **4 voxels tall**, one `hazard_black` band between, a 2-voxel collar below
  and a 2-voxel cap above. From the bottom: red, amber, green.
  Short: red 32..35, amber 37..40, green 42..45, model height 48.
  Tall: red 48..51, amber 53..56, green 58..61, model height 64.
  (`andon_lens_z(lens_bottom, i)` in `assets.rs` is the one definition.)
- **Material classes:** `matte` + `emissive` only (per-class files
  `andon_*.matte.vox` and `andon_*.emissive.vox`; the emissive part is the
  lit lens alone, so the glow pass lights exactly one ring).
- **Bodies nudged:** the crusher's motor now ends at y 29 (was 30) and the
  packer's control panel is centred at (11, 20) (was (5, 14)); both had a
  face on the post's inset when turned. Their icons re-rendered.
- **No icon** for any `andon_*` model (`icons.rs`'s `is_andon`).
- **`VoxModel::rotated_cw`** now turns non-square models (X/Y extents swap
  per odd turn).

### Ticket 3 (Godot) — landed 2026-09-11

- **Nodes:** `_build_machine_andon` stands `andon_<key>_<status>` for each
  of `ANDON_STATUSES` (`working`, `starved`, `blocked`) as siblings of the
  body under `World`, at the centre of `_andon_tile(origin, rotated size)`,
  yaw 0, `_short` for a 1x1 footprint and `_tall` otherwise; recorded in
  `_machine_andons[key] = {lenses, status, height, tile}`.
- **Corner:** `ANDON_CORNER = (sign(CAMERA_DIRECTION.x),
  sign(CAMERA_DIRECTION.z))` on tile axes = `(1, 1)`: the far tile on both
  axes (vat at (12,0) → light on (13,1); mixer at (9,0) → (9,1)).
- **Polling:** `_poll_machine_load` every `LOAD_POLL_TICKS` (30) of SIM
  time from the end of `_advance_simulation`, and once in `_rebuild_world`
  after `_seed_machine_states`. `_machine_load[key] = {load, stall, reason,
  status}`; `reason` is the last non-null stall while the status holds.
  `_set_machine_andon` flips visibility only on a change.
- **Hover:** `_machine_load_text` appends one line to `_machine_hover_text`
  in the plan's language; **the window reads 20 s** (`600 / TICK_HZ 30`),
  not 10 s. `_hover_panel_state` gains `load_meter` (green level bar,
  `_hover_load_meter`), drawn under the progress bar.
- **Fixture:** `build_bottleneck_line` — reference line, hoppers and tank
  on auto-fill, funded to 8000, 400 of each raw + 300 tomatoes, settled
  `BOTTLENECK_SETTLE_TICKS` = 8000; `BOTTLENECK_MACHINES` pins the five
  statuses. `test_machine_load.gd` (in `verify-core-loop.sh`);
  `screenshot-scene.sh --bottleneck` (implies `--populate`, whose loop
  polls the load at its end).
- **Render verdict:** the closeup reads unambiguously. At the whole-line
  auto-framing a lit lens is a 2–3 px dot: the colours are tellable apart,
  the position on the post needs one step of zoom. No Godot-side cause
  (nothing occludes a post on the shipped line); left for the coordinator.

## Coordinator notes

*(appended as work lands)*
