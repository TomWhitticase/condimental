# CONDIMENTAL — Decision records (full history)

This file holds the **full, verbatim decision records** that used to live in
`CLAUDE.md`'s decisions log, `docs/UI_UX.md` §2b and `docs/ARCHITECTURE.md`
§9a. They were moved here on 2026-09-09 because the three `@`-included docs
had grown past the context budget. Nothing in this file is loaded
automatically; it is read on demand.

What each record produced is the **rule**, in condensed form:

- `docs/DECISIONS_LOG.md` → the condensed log (it lived in `CLAUDE.md` until
  the context restructure of 2026-09-16, Part A's newest entry)
- `docs/UI_UX.md` → §2b (same point numbers as below)
- `docs/ARCHITECTURE.md` → §9a (summary)

None of those three is loaded automatically either, since the restructure.
`CLAUDE.md` links to them instead.

Read this file when you need the *why* behind a rule, or the history of a
change. New decisions: add the condensed rule to the loaded doc, and the full
reasoning here (newest first within each part). Point/entry numbering here
matches what code comments and data files cite.

---

## Part A — CLAUDE.md decisions log (verbatim, newest first)

- 2026-09-25: **Every PR is reviewed before it merges, with no exceptions.**
  (Owner's rule, stated after PR #163.)

  **What happened.** A session was told to "pick an issue, fix it, make a PR
  and merge it". It fixed #99, opened PR #163 and merged it on green CI. That
  was allowed at the time: WORKFLOW.md said "merging your own work
  unreviewed is fine for routine changes the owner has not asked to see",
  and the `open-pr` skill went straight from `gh pr checks --watch` to
  `gh pr merge`. The `wrap-it-up` skill (#162), which landed minutes later,
  already assumed PRs merge on a review verdict, so the session reported the
  missing review as "owed post-merge". The owner rejected that: *"we
  shouldn't have post merge reviews, we should always have review before
  merging, that's important"*.

  **The rule.** Green CI plus a `VERDICT: APPROVED` comment from a session
  that did not write the branch. Both are required before `gh pr merge`, for
  every PR. A post-merge review is not a substitute, because by then the
  unreviewed code is already on `main` and every other worktree branches
  from it. An instruction to merge is read as "merge once reviewed". If no
  reviewer is available, the author stops with the PR open. `wrap-it-up`
  already treats that as the one legitimate way to end a session with a PR
  still open.

  **What was rejected.** Keeping the routine-change exemption was rejected.
  Whether a change counts as "routine" is judged by its author, which is
  exactly the judgement a review exists to check. #163 was a
  presentation-only HUD fix, the most routine kind of change, and it still
  shipped with nobody but its author having read it.

- 2026-09-25: **The review render's clock is PINNED — `--fixed-fps` at the
  engine, a frame-summed clock for the HUD fades — so one command gives one
  picture.** (Issue #82.)

  **The defect.** `scripts/screenshot-scene.sh` builds a fixture, then lets
  `main.gd`'s own `_process` run `WARMUP_FRAMES` (45) frames before the
  shutter, and `_advance_simulation` spends real frame deltas against
  `TICK_HZ`. How many simulation ticks the warm-up spends was therefore a
  function of how long those 45 frames took on that host at that moment. Two
  runs of the identical command landed on tick **10621** and **10622**, and
  the PNG hash tracked the tick exactly: items sit a step further along a
  belt, a clipboard digit ticks over, `SCREENSHOT_RENDER`'s primitive count
  changes (967526 vs 969058 on one pair). It is a quiet failure — a wrong
  frame is as plausible as a right one — and it poisons every measurement
  taken through a render, which is how this repo judges its own art.

  **The fix is the engine's, not the game's.** Godot's `--fixed-fps <n>`
  hands every frame the same delta and stops synchronising to real time.
  `screenshot-scene.sh` passes `--fixed-fps 60`, and nothing in
  `_advance_simulation` changes: the tick loop a player runs is untouched,
  byte for byte, which is the constraint that ruled out the obvious
  alternatives (ticking the sim by hand in the harness would have made the
  review path a *different* code path from the game's, and the throwaway
  harness the glow sweep used is exactly that mistake, kept private). A
  pinned 60 Hz frame against a 30 Hz tick spends `floor(45 * 30 / 60)` = **22
  ticks** in the warm-up, every time, on any machine, under any load.

  **The three HUD fades needed a second half.** The money flyout, the hotbar
  name flyout and the contract notice are deliberately outside the tick —
  they run in the `else` branch of `_wire_changed` so they keep fading with
  the pause menu open — and they read `Time.get_ticks_msec()`, milliseconds
  since *process start*, which `--fixed-fps` cannot reach: the same fixture
  photographed after a slow boot shows a different alpha. Moving them onto
  `sim.current_tick()` would have frozen a money flyout behind the pause
  menu, which is the property they were given wall clock for. So `main.gd`
  keeps a clock of its own, `_hud_clock_msec`, summed from the deltas the
  scene is actually handed and read through `_hud_now_msec()`. For a player
  that IS the wall clock. For a review render it is the pinned delta, so the
  fade is reproducible. And a frozen scene (`--payout` turns processing off)
  now freezes its fades with it, which is wanted: a photograph of a
  celebration should not age while the shutter is open.

  **The clock was half of it: the camera had to be sealed too.** A render
  aims the camera — `--closeup`, `--first-person`, a fixture's own framing —
  and then lets the scene run 45 ordinary frames. Those frames poll the REAL
  keyboard in `_pan_camera` and see the REAL mouse in `_input`, so a key held
  in another window on the host, a stray wheel notch or a drag moved the
  camera between the aim and the shutter. Measured here at about one run in
  four, drifting a tile and a half, with the simulation tick identical — the
  failure was invisible in every number the run printed, and visible only in
  the picture. `main.gd` now carries
  `_camera_input_sealed`, and every way live input moves the camera returns
  early when it is set: `_apply_pan`, `_apply_drag_pan`, `_zoom_camera`,
  `_apply_look`, `_apply_fp_move`. The guard sits on the Input-FREE half of
  each pair (`_apply_pan`, not `_pan_camera`) for the reason that half exists
  at all — it is the half a headless test can drive.

  The flag is false for a player, always, and nothing but the review harness
  sets it: this is a property of a photograph, not a mode of the game, and a
  seal that a player could end up inside would be a camera that stops
  responding. `test_screenshot.gd` sets it on frame 1, before the view mode
  is chosen and before any flag has aimed anything. Proven by mutation: with
  the engine made to report a pan key as held, the same command on the same
  tick (10622) renders `972e7d6bbea3` — 946548 primitives, 44 objects, the
  closeup framing quietly undone — unsealed, and the canonical
  `52493c1c3da8` (969058, 64) sealed. `test_camera_seal.gd` fails on the
  removal of any one of the five guards.

  **PR #167 landed under this branch, and the seal stays anyway.** That PR
  moved the render's window off-screen and made it no-focus, for its own
  reason — not interrupting the person at the laptop — and a window that
  never holds focus is not sent the host's keystrokes. Instrumented across a
  whole warm-up on this tip, `DisplayServer.window_is_focused()` is false on
  all 45 frames and `Input.is_key_pressed(KEY_W)` never goes true, so the
  default render path no longer carries live input at all. The seal is kept
  for two reasons. `--on-screen`, the supported flag for watching a run, DOES
  take focus — measured true from frame 20 of the same warm-up — so the
  defect is still reachable through a documented flag. And a reproducibility
  guarantee that rests on a macOS window flag is one that a later change to
  the window silently repeals; sealed, "the photograph is of the camera the
  flags aimed" is a property of the harness, held by a headless test, on
  every platform and in every window mode.

  **The harness refuses an unpinned clock rather than trusting the script.**
  `test_screenshot.gd` measures its own delta against `FIXED_FPS` and prints
  `SCREENSHOT_FAIL` naming the remedy, because `godot --script
  test_screenshot.gd` run by hand would otherwise silently be back to a
  picture that depends on machine load. The pair is kept in step by
  `scripts/check-invariants.sh`, which greps that the script sets
  `fixed_fps`, actually passes `--fixed-fps` to `godot`, and agrees with the
  constant the harness checks against — the only way to catch it, since
  neither half of a non-headless renderer can be run by the headless suite.
  Every render also prints `SCREENSHOT_TICK`, so a disagreement between two
  frames can be read rather than guessed at.

  **Measured, on the rebased tip, cursor parked, 1280x720.** Before:
  `--bottleneck --closeup 4,0,10 --no-hud` four times gave four different
  pictures (`3dba3d7512bd`, `594c0244cc32`, `52493c1c3da8`, `204eff32b9bd`);
  `--fixture chain --populate` gave one hash four times on a quiet machine
  and three different hashes under a six-way CPU load — the exact trap the
  issue names, a quiet window that looks like determinism. After: the
  closeup is `52493c1c3da8` six times (four quiet, two under an eight-way
  load), `SCREENSHOT_TICK: 10622`, 969058 primitives every time; the chain
  fixture is `149e338eb2fc` eight times (four quiet, four loaded),
  `SCREENSHOT_TICK: 622`. No flag is excluded as "deliberately
  non-deterministic": `--populate` ticks a fixed 60x10 loop and was never the
  problem, and `--payout` and `--unlock-toast` are byte-identical too.

  **The consequence to know.** A pinned warm-up stops on a different tick
  than a loaded one did, so a baseline rendered before this change is a
  different moment, not a corrupted one — re-render it rather than diffing
  across the fix. `godot_project/test_camera_seal.gd`
  (`CAMERA_SEAL_OK`) holds the seal, and `godot_project/test_frame_clock.gd`
  (`FRAME_CLOCK_OK`) holds the clock headlessly: `_hud_now_msec()` does not move across 250
  real milliseconds with no frames, each of the three fades reaches alpha 0
  on frames alone, and `WARMUP_FRAMES` at the pinned delta spends 22 ticks
  twice running.
- 2026-09-25: **A status lamp's two states are palette data, written as
  modifiers of the `emissive` class, and every renderer reads the resolved
  JSON.** (Issue #81: the asset browser drew every `*.emissive.vox` mesh at
  full emission, so a cooking vat's rim read bright amber there and dark brown
  in the game.)

  **What the problem actually was.** A status lamp has two presentations, not
  one. `main.gd` calls `_set_machine_running(key, false)` the moment a machine
  body is built and only lights it while the machine is processing, so the dark
  state is the one a machine is in for most of its life. That dark state was
  three numbers hardcoded in `voxel_assets.gd` — a roughness, a zero emission
  and `LAMP_OFF_ALBEDO`. That made it the one thing in the voxel pipeline with
  no representation in `palette.ron`, and therefore the one thing a second
  renderer could not read. The asset browser derives everything else it draws
  live from repo files, which is the whole reason it cannot drift from the
  pipeline; on lamps it had nothing to derive from, so it drew them all lit and
  a reviewer approving a lamp there was approving a state the game rarely
  shows.

  **Why palette.ron, and why modifiers.** `palette.ron` is already the single
  source of truth for colour and material (UI_UX.md §2a), and `voxgen` already
  exports the class table to `models/materials.json` for exactly this reason.
  A lamp state is a material fact, so it belongs there. It is written as a
  *modifier* of the `emissive` class rather than as absolute numbers:
  `emission_scale` multiplies the class's `emission_energy` instead of
  restating it. The alternative — a literal `emission_energy: 1.2` in the lamp
  table — would be a second copy of a number that already exists six lines
  above, and retuning the class would leave the lit lamp behind at the old
  value with nothing to catch it. With a scale there is no second copy to
  drift: a lit lamp is *by construction* the emissive class at full strength.

  **Why a separate file rather than a bigger materials.json.** `materials.json`
  is a flat class → material map that both `voxel_assets.gd` and the browser's
  `voxlib.py` iterate. Nesting a per-state table inside it would have forced a
  shape migration on both readers to express something that is not a material
  class. `lamp_states.json` is written beside it by the same pass, so it is
  still generator output, still staged by `cargo xtask`, and still covered by
  `rust.yml`'s regenerate-and-diff of `assets/voxels/models`.

  **Why both emission and albedo.** A dead lens drops emission to zero *and*
  darkens albedo to 0.38. The second is not redundant. It is what made the
  dock's full-pool blink visible back when emission contributed nothing (the
  two variants once rendered at most 2/255 apart with only energy between
  them), and it is what keeps a dead lens reading as dead in a frame where the
  glow pass is doing the talking. It is also the channel `palette.ron` itself
  reaches for: the `status_*_off` entries are the same hues taken down to a
  dark matte, which is a little over a third of the lit colour — the same
  judgement, made where the colour could be chosen outright. 0.38 is that idea
  applied to a mesh whose colour cannot be repainted per state, because voxel
  colour is per-vertex.

  **The schema bump.** Adding the table changes the shape of `palette.ron`, and
  a loader that rejects rather than migrates has to be told. `schema_version`
  went 1 → 2 and `SUPPORTED_SCHEMA_VERSION` with it. A content change — a
  different hex, a different roughness — still would not bump it.

  **What it is worth.** Measured over the cooking vat's lamp ring and divided
  by the vat's own bright stainless in the same frame, the browser now draws an
  idle ring at 0.193 against the game's 0.235, where before it drew 1.006. The
  gap that remains is the browser's simpler light rig putting slightly less
  ambient on a dead lens; the gap that closed was a factor of five.

- 2026-09-21: **Settings are presentation; a cheat that moves money is a
  real simulation command; F3 is ungated in pre-alpha.** (Owner asked for a
  settings menu and a development cheat panel; the four shape questions —
  which extra knobs, which hosts, how the tool is gated, which cheats — were
  put to them and answered.)

  **What the settings are, and where they live.** Eight knobs: music on/off
  and level, sound effects on/off and level, movement speed, look
  sensitivity, field of view, hall size. They live in a `Settings` autoload
  and in `user://settings.cfg`, written with `ConfigFile`. They are **not**
  in `sim_core` and **not** in the save format. The reason is the save
  format's own rule: a save is a full RON state snapshot with a
  `schema_version` that is *rejected, never migrated*. Putting a volume
  slider in it would mean a preference change could invalidate a save, and a
  save carried to another machine would bring that machine's field of view
  with it. A preference is about the person at the keyboard, not about the
  factory.

  This is also what lets the two audio autoloads stay the only things that
  know about sound: the panel writes `Music.volume` and `FactoryAudio.volume`
  and nothing else in the HUD learns a second way to be quiet. It settles the
  note `main.gd`'s `_toggle_sound` had carried since the pause menu grew
  SOUND and MUSIC buttons — *"a persisted audio setting belongs with the rest
  of a settings screen this project does not have yet, not smuggled into the
  save format"*. Those two buttons are gone from the pause menu now; they are
  rows of this screen, and the tests press the rows.

  **One panel, two hosts.** `settings_panel.gd` builds the rows and both
  `main.gd` and `menu.gd` host them, each swapping them in place of its own
  button column — the shape the menu's Load Game view and the pause menu's
  quit confirmation already have. A second plate over the first would be two
  things to dismiss, and two copies of the rows would be a knob that exists
  in one place and not the other. The panel takes `ui_scale` and `font_base`
  as **arguments** rather than preloading `main.gd` for them: `main.gd`
  preloads the panel, and a preload cycle does not load. For the same reason
  it restates `_button`'s styling and the whole-row snapping instead of
  sharing them.

  **The knobs are steppers, not sliders.** A drag cannot be asserted in a
  headless test, the repo contains no slider to match, and each of these is a
  short list of sensible values. Every value is formatted rather than shown
  bare (`100%`, `×1.25`, `75°`), and the fill bar under a stepper is
  deliberately **not** `_build_meter`'s language: that bar means "the sim
  defines this ceiling", and no part of the simulation has an opinion about
  the field of view.

  **A headless run must not read or write the developer's file.** Every
  existing headless test instantiates `main.tscn`, so a developer who had set
  movement speed to ×3 would have failed `test_camera_nav.gd` on their
  machine alone. `Settings.persist_enabled` is false under
  `DisplayServer.get_name() == "headless"`, decided in the **member
  initializer** and not in `_ready`: an autoload's `_ready` has not run when
  a `--script` test's `_initialize()` does, so a flag set there reads as its
  declared value at exactly the moment the test needs it. `test_settings.gd`
  is the one place that turns persistence back on, around a backup of the
  real file that every exit path restores. `test_screenshot.gd` forces the
  defaults for the same reason from the other side — it is the one check in
  the project with a real display, and a review frame rendered at whatever
  FOV happens to be in one developer's config is not evidence. `--set
  key=value` is how a frame of what a knob *does* gets taken.

  **Granting money is not presentation.** Money is simulation state: spent
  through `try_spend`, written by `save.rs`, read by every panel off one
  counter. A grant Godot kept to itself would mean a balance the sim
  disagrees with and a save that forgets the money. So it is
  `PlayerCommand::DebugGrantMoney { amount: Money }` over the real JSON wire,
  with an `i64` DTO (no `f64` anywhere near it), refused at `amount <= 0`
  leaving the world byte-identical, dirtying no chunk because money is not a
  tile. Named `Debug…` so the enum says which of its arms is a cheat. There
  is no negative grant: taking money away would need a rule about what
  happens below zero that `Economy` deliberately does not have.

  **Time speed is presentation.** Nothing in `sim_core` knows what a second
  is — the tick is the only clock — so the speed lives in
  `_advance_simulation`, which scales the tick interval *and* raises the
  `MAX_TICKS_PER_FRAME` catch-up budget in proportion, so the clamp keeps
  meaning "one slow frame must not fast-forward the factory" instead of
  silently capping ×10 at whatever the frame rate allows. FROZEN is **not
  PAUSED**: no ticks, but the panels, the camera and the animations keep
  running, which is what makes it useful for looking at something. STEP
  spends exactly one tick, on the next frame. The panel states that in-game
  clocks still read in the simulation's seconds, because at ×10 a "40s" load
  window really is four seconds of sitting there.

  **F3 is ungated.** Not behind `OS.is_debug_build()`: this is pre-alpha,
  the owner is the only player, and a review screenshot should show what an
  export does. It is **not a modal** — not one of the five E/M/C/H/P panels,
  so ESC does not take it, it closes nothing and pauses nothing, and the
  factory keeps running behind it, which is the entire point of watching a
  cheat land. It does join `_fp_mouse_free()`, because it is covered in
  buttons and a captured mouse cannot press one.

  **Hall size is three shader uniforms, not five.** `hall_half_width`,
  `roof_height` and `bay_pitch` scale together so the room keeps its
  proportions; `floor_drop` stays put (the slab is one eye-height under the
  camera whatever the room does) and `haze_density` stays put (a bigger room
  already reads hazier because there is more of it to cross). The stepped
  knob is also why this is affordable: each press is one radiance re-bake,
  and a dragged slider would have been one per frame. The title screen takes
  the same scale, so the row changes the room behind the menu as it is
  pressed.
- 2026-09-21: **A review verdict is a COMMENT whose first line is exactly
  `VERDICT: APPROVED` or `VERDICT: CHANGES REQUESTED`.** (Issue #124.)

  **The constraint.** Every pull request in this repository is authored by the
  same GitHub account, and GitHub will not let an account review its own work.
  Measured rather than assumed, against a real PR of this account:

  ```
  $ gh pr review 157 --approve --body "…"
  failed to create review: GraphQL: Review Can not approve your own pull request (addPullRequestReview)
  $ gh pr review 157 --request-changes --body "…"
  failed to create review: GraphQL: Review Can not request changes on your own pull request (addPullRequestReview)
  ```

  `--comment` is accepted, but it produces a review in state `COMMENTED`, so
  `gh pr view N --json reviewDecision` answers `""` — empty on #91 and #98,
  both of which carry full, blocking adversarial reviews. There is no field
  for an automated merge step, or the next session, to read.

  **The rule.** The verdict is therefore carried by the body's own first line,
  exactly `VERDICT: APPROVED` or `VERDICT: CHANGES REQUESTED`, and whoever
  merges gates on that line. It is a convention rather than a mechanism, which
  is the cost: nothing enforces it, so a review that buries its verdict in
  prose is unreadable to the merge step. Making it the *first* line is what
  keeps it greppable and keeps a skim from inverting it.

  **Why the standard behind it is written down at all.** The procedure existed
  only as prompt text held by one coordinator session, so each new session
  either reinvented it or reviewed to a weaker bar without noticing the bar had
  moved. It is now the `review` skill. Across the 2026-09-20 sweep it caught,
  among others: a guard that had merely been given more entries rather than
  made to refuse (#108); an `assert()` that prints and does not abort, so a
  broken fixture still reported `SCREENSHOT_OK` and exit 0 (#110);
  measurements in a `DECISIONS.md` record that did not reproduce (#111); a
  `--closeup=X,Y,S` flag the harness accepted and ignored, rendering the wrong
  frame silently (#114); and a destructive action with no warning that broke a
  `docs/UI_UX.md` rule the same PR had just written (#121).

  The load-bearing half of the procedure is **reverting each fix in turn**: a
  test that still passes with its fix removed proves nothing and is worse than
  no test, because it keeps passing through the regression. The second half is
  that a re-review is not a diff of the rework — the first review's "what I
  verified and it held" is stale the moment the tip moves, so it is re-run.

  **The standing rule the whole thing rests on:** a false absolute in
  `docs/DECISIONS.md` or a `CLAUDE.md` is worse than a bug, because it is what
  the next person reasons *from* instead of measuring, and it is not found by
  the next test run. Hence: re-derive a PR's documented numbers from your own
  run rather than re-running the author's script, and when a claim is
  withdrawn, grep for it in every place it was written.
- 2026-09-21: **A content id is a KEY; what a player reads is a
  `display_name` in the same file.** (Owner's call, asked for directly: "can
  you rename warehouse rack to stock shelving".)

  **What was asked, and the two ways to do it.** The kind the player buys to
  store raw ingredients was called "Warehouse Rack" in the build menu, the
  hotbar tooltip and its hover panel — none of which is a written label.
  `main.gd`'s `_kind_display_name` title-cases the kind id, so all three
  said "Warehouse Rack" because the id is `warehouse_rack`. The obvious
  reading of "rename it" is therefore to rename the id, and the obvious
  reading is the expensive one.

  **What renaming the id would have cost.** A kind id is not a label with a
  lookup behind it; it is a key, and four separate things key off it. It is
  written into every save file twice (the tile's `kind` and the body tag
  `SavedBody::WarehouseRack`), so a rename means a `SAVE_SCHEMA_VERSION`
  bump — and this format **rejects rather than migrates** (see `sim::save`'s
  module doc), so every save anyone holds is refused. `data/shop.ron` and
  `data/storage.ron` both name it. `main.gd`'s `MACHINE_MODELS` maps it to a
  voxel asset, and the asset, its generator function and its `.vox` files
  carry the name too. Beyond that it is the subject of roughly 440 lines of
  design prose across `assets.rs`, `world.rs`, `storage.rs`, `main.gd` and
  two dated records in this file — prose that is measured, cited by point
  number, and worth more than the consistency a blanket rewrite would buy.
  None of that is visible to a player, and all of it would have moved to
  change one string in three panels.

  **What was done instead.** `MachineKind` gained an optional
  `display_name`, the field `Recipe` and `IngredientKind` have carried since
  issue #60, and `World::display_names` gathers all three files into the one
  map the wire already exposes as `display_names_json`. `data/machines.ron`
  says `display_name: "Stock Shelving"` on the `warehouse_rack` entry and
  every panel follows, because each of them already asked that map before
  title-casing. The id stayed. That is the rule this record exists for, and
  it is not a rule about this kind: **a rename in front of the player is a
  content change, and content changes are data.** If a rename ever does need
  the key to move — a save-format break for another reason, say — the label
  is already in the right place to survive it.

  The two hand-written strings that named the kind rather than asking for it
  moved by hand: the market's warehouse line reads "Warehouse: 3 shelving
  unit(s)" and "build shelving for more". "Unit" rather than the kind's own
  name, because "3 Stock Shelvings" is not English — the label names the
  thing, the sentence counts it.

  **What is deliberately still called a rack.** Every identifier
  (`warehouse_rack`, `WAREHOUSE_RACK_KIND`, `RACK_BAY_X`, `_rack_nodes`) and
  every line of design prose. The model draws selective pallet racking and
  that is what the generator should call it; the *kind* is called Stock
  Shelving, and `data/machines.ron` is where the two are tied together. A
  reader who greps either word finds the same object.

- 2026-09-21: **Nothing on a storage model is painted the colour of a
  thing the game stores, and its structure wears one paint.** (The owner's
  call, asked directly: remove any red from the warehouse rack model; then,
  after a green cut was rendered, make the beams the same blue as the rest
  of it.)

  **What was wrong with the red.** The rack's load beams were `paint_red`
  #82372F against `paint_blue` uprights — the two-tone real racking is sold
  in, chosen in the W2 record below for exactly that reason. What that
  record did not weigh is that this kind's whole job is to show **tinted
  stock**: `main.gd` tints each bay's goods per ingredient, and the
  ingredient a player holds most of is tomatoes, `#C22B21`. So the one model
  whose readout is colour had its structure painted in the same family as
  the most common thing it reads out. In the `--warehouse` render the beams
  and the top-level tomato loads are the same warm red at a glance.

  **Why the answer is the uprights' own blue and not another hue.** A
  second structural colour was worth having when the beams were the only
  thing distinguishing a run of racking from three frames standing near each
  other. It is not worth having against the tinted goods: every painted hue
  on the frame is one more thing competing with the only colour on this kind
  that carries information. `paint_blue` #2E5772 spends none — the frame is
  one paint, the goods are the only saturated thing a rack shows (UI_UX §7),
  and the blue is already the machine-skirt colour, so a rack stays factory
  equipment rather than a second visual family.

  **What replaces the hue: the light.** A beam still reads as a beam
  because its lit top face and the upright's vertical faces are different
  planes under the fixed isometric key. Measured over the beam pixels of the
  `--warehouse` render, the beams' top faces come back at 134.7 where the
  unchanged blue structure beside them sits at 55.7 — a separation of about
  79 luma, which is why the beams do not dissolve into the frame. This is
  the thing to re-render before anyone "restores" a contrasting beam colour:
  the separation is real, it is just made of shading rather than paint.

  **The swap is value-neutral, deliberately.** Over that same mask the blue
  beams measure mean 76.4 where the red ones measured 74.7 (flat colour
  luma 77.8 against 76.5). Every relationship the rack was tuned by is a
  relationship in value — the goods above the beams, `RACK_TINT_DEAD_BAND`
  and the six shipped tints measured against it — so a swap that moves hue
  and not value costs no re-measuring. The green cut this passed through,
  #3A6B33, measured **+11.6 luma** over the same mask and was rejected on
  that number rather than by eye; #32602F was the value-matched green that
  replaced it, and the owner's second call retired both.

  **The rule, past this asset:** on anything that displays stock, structure
  is painted in a hue no product uses — and preferably in a hue the model
  already wears, since structure can separate from structure by value alone.
  When an existing colour is replaced on an asset whose readings were
  measured, match its luma and move only hue, or the measurements have to be
  redone.

- 2026-09-21: **A session opens in FIRST PERSON; isometric is a TAB away.**
  (Owner's call, asked for directly.)

  **What changed, and what did not.** `main.gd` grew one constant,
  `DEFAULT_VIEW_MODE`, and `_ready` applies it with a single
  `_set_view_mode` call. Nothing about either camera changed: the isometric
  camera is still the fixed `(1, 1, 1)` orthographic one UI_UX.md §1
  specifies, first person is still the free-look fly-cam TAB has reached
  since it was added, and the two still share one `Camera3D`. The opening
  pose in first person is the same pose TAB gives — hovering `TILE * 1.5`
  above the framed focus point, pitched 15° down — deliberately, so the view
  the game opens in and the view TAB produces are one behaviour, not two
  that can drift.

  **Why this does not contradict §1.** §1 is about the *angle*: the assets
  are modelled to read from fixed directions, so the isometric camera may
  not orbit. Which view a session opens in is a separate question, and the
  factory has had an answer to it worth changing since the interior sky
  landed — the hall is drawn in first person (§3), the voxel art is built at
  a human scale, and the isometric view is where a line gets laid out, which
  is the second thing a player does. The refusal list in `CLAUDE.md` still
  stands: no free-rotating camera, and the isometric view is unchanged.

  **The call has to come after `_build_ui`.** `_set_view_mode` shows the
  crosshair, rewrites the help legend and applies the first-person mouse
  capture, and none of those exist before the HUD is built — placing it
  beside `_build_camera` gets a first-person camera with an isometric
  legend and no crosshair. `_view_mode` still *initialises* to `ISOMETRIC`
  rather than to the constant, because `_build_camera` has already
  configured the camera that way and `_set_view_mode` early-returns on a
  mode it is already in: that makes flipping `DEFAULT_VIEW_MODE` back to
  `ISOMETRIC` a correct no-op rather than a broken one.

  **The screenshot tool does not follow the game.** `test_screenshot.gd`
  pins the view itself now (`_enter_view_mode`): isometric unless
  `--first-person`. Every other framing flag it has is an isometric idea —
  `--closeup`/`--zoom`/`--pan` write `_camera_focus`/`_camera_size`, and
  every `--hover*` aims the cursor at where a tile *projects* — so
  inheriting the new default would have silently re-aimed every command
  line in `docs/`, `scripts/` and CI at a camera those flags cannot steer,
  and the PNGs would have come back wrong rather than refused. The default
  itself is proved by `test_camera_nav.gd`, which asserts the mode, the
  projection, the crosshair and the legend straight out of `_ready` before
  TABbing back to isometric for the navigation tests it already had.

- 2026-09-21: **The one wall-clock assertion in the gate is scaled by how
  loaded the machine is, measured in the same process.** (Issue #146.)

  **The fault.** `godot_project/test_perf.gd` asserts that 120 hand-driven
  `_process` frames on the 1500-tile stress line finish inside
  `FRAMES_CEILING_MSEC := 3000`, and that a warm `_rebuild_world` finishes
  inside 1500 ms. Every other test in the suite asserts an outcome — a count,
  a refusal, a rectangle — and is reproducible. These two assert a duration,
  and this repo is worked in six worktrees on one machine, each of which runs
  `godot --headless --script test_*.gd` whenever it likes. On 2026-09-21 a
  full verify on a branch that touched nothing but panel placement reported
  `PERF_FAIL: 120 frames took 3733.1 ms`; re-running the same script alone,
  same build, no edit between the two, gave 350.7 ms. Ten times, from
  neighbours.

  **Why that is worth fixing rather than tolerating.** The failure does not
  read as noise. It reads as a performance regression, in the one test whose
  whole job is to catch performance regressions, and it arrives at the end of
  a long verify when the session is trying to open a PR. The cost is a
  session-hour spent hunting a regression in a diff that has none — and the
  cheaper the disproof looks, the more likely the next agent is to skip it
  and "fix" a phantom.

  **Why not just raise the ceiling.** The ceilings are already about 8x the
  measured solo cost, and the test's own header says why: they are generous
  on purpose so that this catches an order-of-magnitude regression — a
  Node3D per tile coming back — and not a ten-percent one. A ceiling loose
  enough to survive four concurrent Godot runs is loose enough to let the
  regression it exists for pass.

  **The rule.** The test times a fixed amount of pure-script arithmetic
  (400k iterations, about 12 ms unloaded) in its own process, once before the
  measurements and once after them, takes the worse of the two, and
  multiplies both ceilings by `measured / reference`. Contention that arrives
  halfway through a run is as real as contention that was there at the start,
  which is why it is bracketed rather than sampled once. The factor is
  clamped to 1.0 below, so a quiet box gets exactly the constants and nothing
  is loosened, and to 6.0 above, because past that the machine is too busy
  for any of it to mean much and a ceiling that rises without limit is not a
  test. The raw milliseconds are still printed first and unscaled: the
  ticket-4 before/after numbers in DEVLOG quote those lines, and a number
  that moves with the weather is no good to compare against.

  **The median, never the minimum.** A process fighting for the CPU still
  gets a full slice now and then, so its fastest sample is indistinguishable
  from an idle machine's — measured under sixteen spinning processes, the
  fastest of nine samples was 11.9 ms against an idle 12.2 ms, while the
  median was 38.3 ms and the slowest 113.5 ms. The mean is no better: one
  descheduled sample drags it around on its own. The median is what a long
  stretch of work, which is what the frame loop is, actually runs at.

  **What this does not license.** Scaling is for a contended clock, not for a
  slow implementation. Nothing else in the suite may grow a wall-clock
  assertion on the strength of this; an outcome to assert is almost always
  available, and where it is not, the number is printed and reviewed rather
  than gated.

- 2026-09-21: **Stock has a ceiling, a rack is pure storage, and its bays are
  derived from the ledger rather than stored.** (Warehouse stock racks, W1.)

  **Why a ceiling at all.** Until `data/storage.ron` existed, `BuyIngredient`
  credited `OwnedIngredients` with no limit but money — and money is the thing
  a running factory produces. A player one contract in could buy four thousand
  tomatoes and never think about supply again, which collapses GAMEPLAY.md
  §5's "ingredients — recurring, scales directly with production volume" cost
  axis into a single early purchase. The cap is what makes stock something you
  build room for. `base_cap` (20) is free and unconditional, so a new game
  works with no warehouse and no ingredient can ever be crowded out of
  existence by another; above it, stock occupies **bays**, and bays are what a
  rack supplies.

  **R6 wins: a rack does not touch belts or pipes.** The sprint plan
  contradicted itself — its R6 said racks have no interaction with the
  logistics network, its W1 scope asked for "the rack's one-unit-per-tick belt
  push in tick step 5", and its W3 asked to "wire the config panel's item
  picker to the rack's output item". Those are three different buildings. A
  rack is **pure storage**: no output item, no picker, no facing-dependent
  discharge, no topology links, no step-5 arm. The hopper stays the only thing
  that meters raw onto a belt, which keeps one answer to "how does raw enter
  the line" instead of two that a player would have to learn to tell apart.
  `BodyKind::WarehouseRack` appears in `body_kind` and nowhere in tick step 4
  or 5, and `a_rack_beside_a_running_belt_never_touches_it` pins that absence.

  **Six bays at a 50 stack, priced 75.** Six is the model's own geometry, not
  a free parameter: W2's voxel asset is two bays wide by three levels tall
  (issue #147), and W3 draws one pallet per bay, so a rack that claimed a
  different number would be a building lying about what it holds. Six slots
  instead of the plan's four makes a rack +300 per ingredient rather than
  +200, so the plan's ~50 becomes **75** to hold the capacity-per-money it
  intended — level with the crusher, above the fluid tank's 60.

  **Bays are derived, not assigned.** The plan bound each bay to an item id
  and persisted it. Deriving instead —
  `bays_for(count) = ceil(max(0, count − base_cap) / slot_stack)`, summed over
  the ledger — means there is no allocation state to drift out of step with
  the ledger, nothing to migrate in a save (`SavedBody::WarehouseRack` carries
  no fields and `schema_version` stays 2), and a bay frees itself the moment
  the stock in it is consumed rather than staying reserved for an ingredient
  the player owns one unit of. The consequence to know is that free bays are a
  **shared pool**: filling every bay with tomato does stop the player buying
  sugar above `base_cap` until some tomato is used. That is the intended
  pressure — a warehouse is a finite room — and `base_cap` is the guarantee
  that it can never become a deadlock.

  **No new refusal channel.** There is no generic "why was that refused" wire,
  and this did not add one. The established pattern (#143) is that the
  simulation declines silently and presentation derives the reason from a
  snapshot field, so `IngredientSnapshot` gained `capacity` and the market
  panel will say "at capacity" itself. A clamped buy is **charged for what was
  credited, never for what was asked** — the same rule `OwnedIngredients::take`
  already states for the hopper path.

  **What the cap cost the fixtures, and the shape of the fix.** Every fixture
  that ran a line for long enough to matter bought a deep larder up front;
  with a ceiling, none of them could. Both test layers now build the warehouse
  they need through the real commands (`common::make_room` in Rust,
  `Layouts.make_room` in GDScript) by adding one rack and re-asking, so the
  bay arithmetic stays in `sim_core` and neither fixture restates it. The long
  runs top up instead of stocking deep (`restock`): 3 000 of one ingredient is
  sixty bays, and `--full-dock` would have photographed a wall of racks
  instead of a dock. `*_PLACEMENTS` counts exclude the warehouse
  (`Layouts.line_placements`) — the layout under test is the line.

- 2026-09-21: **A voxel asset is judged against the factory floor, and the
  fixed camera is paid with spacing, not with deletion.** (Warehouse stock
  racks, W2. Issues #144 and #147.)

  **The bike rack.** The first design was one row of four bays: five ∩
  frames standing on a deck, with no longitudinal load beam. The beam was
  dropped on purpose — this camera sees a tile's +X and −Y faces, a sight
  line travels `(+x, −y, +z)`, and a front beam therefore hides whatever sits
  about twelve voxels below and behind it. Deleting it kept every load
  visible and destroyed the silhouette: the owner's verdict was "more like a
  bike rack than a warehouse rack". The occlusion was real and the remedy was
  wrong. The shipped rack runs solid painted beams the full 64 voxels at
  both post planes and buys the loads their visibility with **clearance**
  instead — a fifteen-voxel level pitch, the load's top standing well under
  the beam above it. Rule: when this camera eats a feature, move the feature
  or move what it hides; removing the structural member removes the reading.

  **"footprint × 32" governs the footprint plane only.** Stated here because
  it was got wrong in the session that wrote this: the rule that a kind's
  model is its `data/machines.ron` footprint × 32 constrains X and Y. Height
  is free — `fluid_tank` is 1×1 and 40 tall, `machine_cooking_vat` is 2×2 and
  48. The only ceiling on a tall asset is a design one, that the vat stays
  the biggest object on the line, and a voxgen test pins the rack against the
  vat's *actual* model height rather than a magic number.

  **Black is not the background.** `vengi-thumbnailer` has no alpha cvar, so
  every preview stands on an opaque black pad, and every open volume inside
  an asset is measured against black. The game's backdrop is the factory
  floor, `#9C9C90`, luma **155.1**. The two give opposite answers. Against
  black the rack's bay tints all read "too dark to see"; against floor the
  saturated ones read dark-on-light and are perfectly legible, and what
  actually vanishes is a **mid** tint sitting at the floor's own value. So:
  repaint the pure-black pixels with the floor colour before taking any mean,
  and state the verdict as a dead **band** rather than a threshold.

  For this rack the band is **100..175 luma**, recorded as
  `RACK_TINT_DEAD_BAND` with its method and its results table in the doc
  comment. Measuring it found a genuine defect. The bottom bay stood on the
  `concrete_dark` plinth while the two levels above are backed by floor, so
  the same tint separated by 86/255 up top and 17/255 down there — decking
  the bottom bay in the same `steel_dark` as the others narrowed the band
  from 55..175 to 100..175 and made all six slots behave alike. Four of the
  six existing hopper tints clear the band; `vinegar` and `liquid_smoke` do
  not and get their own darker rack values. `HOPPER_FILL_TINTS` is left
  alone: those were measured against a hopper's dark throat (issues #80,
  #131), which is a different problem with a different answer.

  **The generator is the wire to W3.** The bay geometry and the dead band are
  `const`s in `assets/voxels/generators/assets.rs`, several of them unused by
  the generator itself and `#[allow(dead_code)]` rather than deleted.
  Deleting them moves the numbers into GDScript, which is the thing
  `DOCK_DECK_Z`'s comment exists to prevent. W3 reads them; it does not
  re-derive them.
- 2026-09-21: **A panel that points at the world is FOLLOWED in isometric and
  PARKED in first person.** (Issue #105.)

  **The fault.** `_update_hover_tooltip` asks about a screen point — the live
  cursor in isometric, the crosshair at the centre of the screen in first
  person, where the mouse is captured and has no meaningful position — and
  handed that point to `_panel_position_near` in both. That function's job is
  to keep a panel off the tile it describes by flipping it into whichever
  quadrant around the *cursor* has room, which is exactly right when the
  subject is under the pointer. In first person the subject is behind the
  crosshair and the crosshair is the middle of the frame. At 1600x900 the flip
  happened to find room to the right and the fault never showed; at 1280x720
  the legend and the right-hand column take enough of the frame that all four
  quadrants fail, the fallback clamps the panel back on screen, and it lands
  squarely on the crosshair. PR #78 published the frame as its own evidence
  without anyone reading it that way: the dock, the belt feeding it and the
  crosshair are all behind the panel describing them.

  **Why not offset the query point.** Moving the anchor off the subject and
  clamping is the obvious repair and it does not work: the clamp is the thing
  that puts the panel back over the middle, and an anchor moved off the
  crosshair describes a tile the player is not looking at. Nor is "below the
  crosshair" available — at 1280x720 the strip between the crosshair and the
  hotbar's reserved height is about 230px and a machine hover is 283 tall.

  **The rule.** First person parks the panel in the lower-left corner rather
  than following anything. That corner is the one region of the frame no fixed
  plate claims (the legend is top-left, the clipboard/STOCK/CREW are the
  right-hand column, the hotbar has its own strip) and the one the crosshair
  never enters at any resolution a panel narrower than half the frame is drawn
  at. Being fixed is a second gain: a followed panel slides across the screen
  on every mouse-look, which a parked one does not.

  **The crosshair is a veto, not a score.** The corner is then stepped clear
  of whatever plate it still touches — at 1280x720 the legend reaches far
  enough down to clip a tall tooltip's top — by the same least-HUD-covered
  scoring `_panel_position_near` falls back to, ties keeping the corner. But a
  candidate containing the crosshair is dropped before it is scored at all.
  Covering the legend, which the player can fold away with F1, is always
  better than covering the machine they walked across the hall to look at, and
  a preference that strong is stated rather than left to an area sum.

  **One door for both panels.** The placement prompt anchors on the crosshair
  in first person too and had the same fault, unreported. Both now go through
  `_world_panel_position`, which picks the rule from the view mode, so the two
  cannot be fixed one at a time. The plates they avoid are one list too
  (`_hud_avoid_rects`), for the reason issue #88 already wrote down: a branch
  per plate is how CREW came to be missing from one of them for four days.

- 2026-09-21: **A gated ingredient is padlocked in the contents picker, not
  hidden from it — and pointing a vessel at one is REFUSED.** (Issue #90.)

  **The fault.** `data/unlocks.ron` states its own doctrine plainly: a gate
  holds back ingredient and contract ids, and both `BuyIngredient` and
  `SignContract` are refused for a locked id "not merely hidden from a
  panel — so a gate is a rule rather than a decoration on the UI". The
  configuration panel's CONTENTS picker was the decoration. It listed every
  raw the content files define, the simulation accepted a click on any of
  them, and the player's hopper then sat empty behind an "in stock: 0" that
  described the symptom and not the cause. Pre-existing for hoppers; issue
  #43 mirrored it onto tanks on purpose, because two pickers built from one
  piece of code behaving differently would have been worse than one
  consistent gap.

  **Kept and padlocked, rather than hidden.** The market omits a locked row
  and the picker keeps it. That asymmetry is the decision, and it is about
  what each panel is for. The market answers "what may I buy now", where an
  unbuyable entry is noise. The picker is the full set of things this vessel
  will ever hold — the place a player learns the game has a liquid smoke in
  it at all. A gate the player can see coming is something to aim at;
  GAMEPLAY §2's progression only pulls if the next tier is visible from the
  current one. So the slot stays, dimmed, with a 🔒 in the corner the ✔ would
  have taken (never colour alone, UI_UX §7) and a line under the row naming
  the unlock and the distance to it: "🔒 Liquid Smoke — BBQ Sauce, 0 of 10
  deliveries". The glyph says shut; only the words say when it opens, and
  the number is the part the player can act on.

  **The button stays live.** The obvious alternative is `disabled = true`,
  which swallows the click and says nothing. A live button issues the
  command, the simulation refuses it, and the cue is read back off what the
  building holds afterwards — so the press sounds REFUSED, exactly as the
  audio rule requires (a cue is fired by the handler that knows the real
  outcome, never by the input). Pressing the slot the vessel already holds
  is let through silently, because that press asked for nothing.

  **The wire carries the verdict.** `IngredientsSnapshot`'s `hopper_items`
  and `tank_fluids` became `ContentsChoice` records — `id`, `locked`,
  `unlock_name`, `after_deliveries` — beside the lifetime delivery count,
  and `World::ingredients_snapshot` passes the gate in as a closure. Godot
  does not re-derive a gate from `unlocks.ron`; it reads a decision the
  simulation already made, which is the same division `next_bill` and
  `shortfall` are on.

  **Where the rule stops, and why that is not a hole.** `Place`'s `item` is
  NOT gated. It is how the build ghost seeds a newly placed bin and how a
  layout fixture stands a factory up for a screenshot — not a choice made
  against a gate. The ghost's default now skips locked ids, so no player
  path produces such a Place. Refusing it as well would close nothing a
  player can reach, and would cost every BBQ fixture ten deliveries of real
  play before it could place a hopper: `sim::reference` can state that
  premise cheaply (`open_every_gate`), but `godot_project/test_layouts.gd`
  builds only through the public command API and has no honest way to pay
  it, so the price would have been a test-only back door on the bridge —
  worse than the hole it closed. The gate still bites the bin it seeds: the
  raw cannot be bought, so neither a manual fill nor auto-fill can put a
  unit in it, and `unlocks.rs` pins exactly that.

  **The refusal lands before the purge.** Declaring a *different* fluid
  empties the tank and the run it feeds. A refused declaration must
  therefore refuse first and destroy nothing, or a click the game was never
  going to honour would still cost the player the vinegar standing in the
  drum. `try_set_tank_fluid` checks the gate immediately after the raw
  check, above `purge_runs_fed_by`, and the test asserts the volume as well
  as the name.

- 2026-09-21: **An issue is for what you cannot fix now, and the bar is no
  longer low.** (Amends the 2026-09-16 GitHub entry below.)

  **What went wrong with the old rule.** "Deciding something is out of scope
  is only half the decision; filing it is the other half" was written against
  a real failure — the `resolve_sink` contradiction that sat in a DEVLOG
  paragraph for a week because there was nowhere else to put it — and it
  fixed that failure. The part that did not survive contact is the bar:
  *if it was worth putting in a report, it is worth an issue.* Agents write
  thorough reports. The rule therefore converted every observation into a
  backlog entry, and the backlog became a queue of findings that are small,
  true, and unprioritised. Its longest-lived entries were `verify-core-loop.sh`'s
  header claiming thirty Godot scripts where it runs 37, and a `recipes.ron`
  comment citing 60 ticks/sec against a 30 Hz tick — each a one-line edit
  that cost more to file, read and rank than to make.

  **The asymmetry that the old rule missed.** A PR is read once and then it
  is gone; an issue is read by a human on every pass over the board until
  someone closes it. So the two halves of "fix it or file it" do not cost the
  same, and the rule now says so: **an issue nobody will ever choose to work
  on costs more than it records.**

  **The gates.** Three, in order, in `file-issue`'s first section. *Could you
  just fix it* — a line or two in a file the branch already touches (a stale
  comment, a wrong constant, a typo, a misleading log line) is a fix plus a
  line in the PR body, and so is a small safe fix in the area you are already
  working in, provided the PR's title still describes the PR. Filing is for
  what you cannot do now: an owner's call, a save-format break, a design-doc
  contradiction, genuinely larger work, or a part of the tree this branch has
  no business touching. *Would anyone pick it off the board ahead of the next
  feature* — polish with no defect behind it, speculative hardening ("add a
  test for X" where X has never broken), systems CLAUDE.md lists as
  deliberately not built yet, and restatements of a limit a design doc already
  records all fail this. *Is it already filed* — a second sighting is a
  comment on the open issue.

  **The budget is a smell test, not a quota.** Roughly one issue per pull
  request. A session that wants to file five has usually found one underlying
  problem worth a single issue, or is filing nits; where several real
  deferrals do come out of one branch, the rest go in the PR body, which is
  the right venue anyway — review can promote a line in a body into an issue,
  and review is a human deciding, which is exactly the judgement the backlog
  was accumulating in place of.

  **What does not change.** A finding that passes the gates must still be
  filed, with the same cold-reader body (file *and* function, how it was
  found, why it was deferred, what the fix probably is) and the same short
  label set. Issues are still the backlog and nothing else. The failure the
  2026-09-16 rule was written against is still a failure; this narrows what
  counts as a finding, not what happens to one.

  **Where it is written.** `docs/WORKFLOW.md`'s "Fix it, or file it — and
  prefer fixing it" is the rule; `file-issue`'s first section is the gate and
  its description says so, so the skill is not reached for by reflex;
  `open-pr` and `.github/pull_request_template.md` now ask for a line in the
  body first and an issue only past the gate; `CLAUDE.md` carries the
  one-sentence version.
- 2026-09-21: **A machine kind may name several recipes, and a placed machine
  runs whichever one its inputs satisfy — there is never a picker.** (The BBQ
  epic, T1/PR #45, landed 2026-09-18; recorded here as T5, late.)

  **The invariant this replaced was deliberate.** `world.rs`'s
  `recipe_for_kind` panicked on a second recipe naming a kind, and said why:
  there was no recipe-selection mechanic, so a second recipe would have been
  content the simulation could not act on. `save.rs` leaned on the same fact
  — a machine's recipe was *not* saved, because "a placed machine runs the one
  recipe its kind names, so saving it would be saving content". Both were
  correct while the chain made one sauce. BBQ is a second recipe on the mixer,
  the cooking vat, the bottler and the packer — four kinds at once — so the
  rule had to go, and the question was only what replaces it.

  **Three ways to choose a recipe, and why inputs won.** (A) **A player-facing
  picker** on the configuration panel: explicit, and the shape most games use.
  Rejected on the owner's constraint and on `docs/BBQ_SAUCE_PLAN.md` §7 — a
  machine-selection screen is a different game's affordance, and this game's
  verb is plumbing. (B) **A recipe chosen at build time** and stored on the
  placement: no UI at run time, but it is still a player choice, still needs a
  screen to make it on, and now also needs one to *change* it. (C) **The
  inputs decide**, which is what shipped: the plumbing already distinguishes
  the cases with no ambiguity to resolve. A mixer standing on a vinegar pipe
  beside a sugar hopper blends ketchup mix; a mixer on a ketchup pipe beside
  spice and molasses hoppers blends BBQ mix. The two never both match, so the
  player's layout *is* the answer, and the game gains a second sauce without
  gaining a single new verb.

  **What `Machine::choose` does, exactly.** `RecipeRegistry::for_machine`
  returns a kind's recipes in `data/recipes.ron` file order. The **first
  fully-satisfied** one is started, so file order is the deterministic
  tiebreak when two match (ARCHITECTURE §3.9 requires one; it is the same
  tiebreak `contracts.ron` already uses) — and that is the only role file
  order plays in selection. When *none* is satisfied, the machine holds the
  **closest** one: most requirements met, earliest winning a tie. Nothing in
  the tick turns on that branch — an unsatisfied machine does not move — but
  the snapshot and the stall readout are read off `Machine::recipe`, and
  "waiting for molasses" has to be a true sentence about a BBQ mixer where
  "waiting for vinegar" is not.

  **The save-format consequence, which is why this PR was not self-merged.**
  Once a kind runs several recipes, a machine halfway through a BBQ cook can
  no longer re-derive its recipe on load: the inputs it would be derived from
  have already been consumed by the craft in flight. So `SavedMachine::recipe`
  is persisted. It went in as a `#[serde(default)]` addition with no
  `SAVE_SCHEMA_VERSION` bump, inside the window `save.rs` had explicitly
  reserved for exactly that — **that window has since closed**, and the first
  real bump (schema 2, `SavedTank::fluid`, 2026-09-20) is recorded below. A
  save written before the field loads with no in-flight recipe, which is what
  a machine with no craft in flight reads as anyway; a save naming a recipe
  its kind can no longer run is refused with its own error rather than
  silently re-picked.

  **The knock-on nobody costed in advance: naming a pipe run got hard.** A
  network's fluid id used to be one line — the first machine to reach an
  unclaimed run named it from its kind's only recipe, and producer and
  consumer agreed by naming the same id in `recipes.ron`. With a kind running
  several recipes, a mixer's outlet run is `ketchup_mix` or `bbq_mix`
  depending on what is plumbed *into* the mixer, which is the very thing being
  derived; asking each machine in turn would make the answer depend on which
  machine the walk reached first, and this simulation does not get to have
  answers like that. `resolve_network_fluids` therefore derives the whole
  graph at once by narrowing — each run starts with every id it could hold and
  is narrowed to a fixpoint. That pass, and the ambiguity it can legitimately
  end on, is the root of issues #43 and #53 and of the tank-declaration record
  below. It is the real price of this decision, and it was paid once.

  **`validate_chain` came with it.** `recipes.ron` no longer describes one
  production line, so "recipe i+1 consumes something recipe i produces" was
  the wrong invariant and was relaxed to a graph check. It has since been
  refined again (issue #42: per connected component, each needing a recipe
  that can actually be *started*) — see that record; the rule to obey is the
  later one.

- 2026-09-21: **A recipe may draw several fluid networks, matched by fluid id;
  it still deposits into exactly one.** (The BBQ epic, T1/PR #45, landed
  2026-09-18; recorded here as T5, late.)

  **What was true before.** The 2026-09-08 chain rework (below) settled how a
  machine plumbed on both sides stays unambiguous without the player labelling
  a run: **a fluid output leaves by the face the machine points at, exactly as
  a solid one does; a fluid input may come from any other pipe touching the
  footprint.** `validate_fluid_scope` then asserted `fluid_inputs.len() <= 1`,
  and `two_networks_mut` handed out exactly one draw plus one deposit and
  asserted they differ.

  **What changed, and what deliberately did not.** The input cap is gone. The
  **output** cap is not, and is not up for discussion: a machine still
  deposits into exactly one network, off its outlet face, which is the whole
  mechanism that keeps a both-ways-plumbed machine readable from the layout
  alone. `validate_fluid_scope` now enforces a different pair of rules: at
  most one fluid *output*, and **no recipe may name the same fluid twice** —
  each input is matched to a *different* network, so one fluid means one input
  with the units added up. Each fluid input is matched **by id** against the
  networks touching the footprint, greedily in recipe order, each network
  supplying at most one input. `world.rs` hands `Machine::tick` N disjoint
  draw borrows plus one deposit, and still *asserts* disjointness rather than
  assuming it — the assertion is the only thing making the aliasing sound, so
  it was kept when the arity changed.

  **Why it was bent, and why now rather than later.** The owner's two choices
  for BBQ — the mixer outputs a liquid, and liquid smoke goes in at the cook —
  make the cook a two-network recipe (`bbq_mix` plus `liquid_smoke`), and
  there is no layout that dodges it. Neither choice is arbitrary flavour: they
  are what make BBQ a *branch* tapped off the ketchup pipe rather than a
  second line standing beside it. And every sauce after it wants the same
  capability — mayo is oil plus acid, hot sauce is vinegar plus mash
  (GAMEPLAY §2, tiers 4 and 5) — so it is better bent once, now, with a real
  two-fluid recipe in the shipped content to test it against, than bent later
  against a fixture.

- 2026-09-21: **A progression gate is a filter on two lists, keyed to lifetime
  deliveries, and it announces itself.** (The BBQ epic, T3/PR #66, landed
  2026-09-19; recorded here as T5, late.)

  **The game gated nothing before this.** Every ingredient in
  `ingredients.ron` and every contract in `contracts.ron` was available in a
  new game, so the only thing that grew as you played was the balance. That
  was right while the chain made one sauce — there was nothing to hold back.
  The second sauce changes it: handing a player the whole two-sauce board in
  their first minute makes the ketchup line read as a chore standing between
  them and the interesting one.

  **A gate names two lists, and there is deliberately no third.**
  `data/unlocks.ron` (schema 1, loader `sim::unlocks::UnlockRegistry`): an
  `Unlock` has an id, a player-facing `name`, an `after_deliveries` threshold,
  `ingredients` held back from the market and `contracts` held back from the
  board. **There is no way to lock a recipe and no field to add one with.**
  The BBQ recipes sit in `recipes.ron` from a new game and are simply
  unbuildable — their raws cannot be bought — and unsellable, because nobody
  is asking. That is not a simplification: a locked recipe would be a state
  every placed machine consults on every tick, the hottest path in the
  simulation, to answer a question whose answer changes a handful of times in
  a session. Two filtered lists cost the tick nothing at all — `machine.rs`,
  `belt.rs` and `fluid.rs` do not read this module.

  **A gate is a rule, not a decoration on a panel.** Both lists are enforced
  in the simulation: `BuyIngredient` is refused for a locked id (`world.rs`),
  and `SignContract` returns `SignRefusal::Locked` whether or not a row was
  ever drawn for it. Godot hiding the row is then presentation, and a bug in
  the panel cannot sell the player a gate that is shut.

  **Why lifetime deliveries, and not money, ticks or machines placed.** Money
  can be sat on, ticks pass while the player reads the keybinds, and a placed
  machine need never run — but a delivery is proof that a whole line ran end
  to end and something was shipped. It counts satisfied delivery windows only
  (`Economy::tick`'s `Paid` event); a missed window counts for nothing, a lost
  contract does not take its past deliveries back, and the count never
  decreases, so a gate once open stays open. The threshold is **ten**, which
  is ninety seconds to three minutes of a working line; the measured sweep
  behind that number, and the reason it was *not* raised to paper over issue
  #55, is the 2026-09-20 record "The bank at the moment of building decides
  whether the BBQ branch costs you the board" and the protocol in
  `unlocks.ron`'s own comment.

  **The counter is saved; the gate's state is derived.** `SaveData` stores
  `lifetime_deliveries` and nothing else about progression. Saving "bbq_sauce
  is unlocked" would freeze a save against the numbers in the file on the day
  it was written; deriving it means a balance pass on `unlocks.ron` re-reads
  correctly for every existing save.

  **Why the group is named rather than each entry carrying `unlock_after`.**
  The obvious alternative spreads one decision across seven entries in two
  files and leaves nothing to announce — and **the announcement is the point
  of a gate**: one that opens silently is one the player never experiences.
  A toast needs a subject, so the group has a `name`, and that name is
  player-facing prose rather than an id: "★ BBQ Sauce unlocked — new
  ingredients in the shop, new orders on the board". The sim decides which
  gates are open and publishes them in its snapshot; `main.gd`'s
  `_announce_unlocks` diffs that list against what it has already seen, and
  primes itself on the first snapshot so a loaded save does not re-announce a
  tier the player earned sessions ago.

  **What this gate deliberately does not do.** It does not amend live
  contracts on opening — losing a contract you were serving fine because the
  game changed its terms reads as a bug, not as difficulty. It does not
  tier-lock the rest of the board: nothing else on it is gated at all. And it
  is not a reputation meter — see GAMEPLAY §11, where this settles the
  gating half of that question and leaves reputation itself open.

- 2026-09-21: **A belt's rubber runs on that tile's own work clock, and the
  simulation is what counts it.** (Issue #79.)

  **The fault.** UI_UX.md §6a already says a continuous effect is driven by
  the machine's own work clock — a starved crusher throws no pulp. The belt
  animation predated that rule and broke it: the frame index was
  `floor(tick * BELT_FRAMES / BELT_TICKS_PER_TILE) mod BELT_FRAMES` off the
  *global* simulation tick, one number for every belt in the world. Back a
  line up behind the 240-tick cooking vat and the items stand still while
  the rubber under them races — a decoration running because the belt
  exists, which is the exact thing the rule forbids, and a loud lie at that:
  running rubber is the strongest signal on the floor that product is
  moving.

  **Why the count is in Rust and not in Godot.** This looks like pure
  presentation, and the reflex is to keep it in GDScript. Godot cannot
  compute it. Delta granularity is per chunk, and a belt that stops moving
  stops dirtying its chunk, so the presentation layer is never told "this
  tile stood still" — absence from a delta is all it gets, and absence is
  indistinguishable from "this chunk had nothing to say". The simulation is
  the only party that knows a tile advanced product this tick. So
  `BeltSegment::tick_toward` increments `moved_ticks` exactly when it
  reports movement, and `BeltSnapshot`/`SplitterSnapshot` carry it.

  This also makes the dirty-chunk machinery do the work for free: a moving
  belt dirties its chunk and re-sends its count, a jammed one is absent and
  Godot keeps the last count it was given, which is already where the
  pattern belongs. Cost lands exactly where the movement is.

  **A counter, not a flag.** The cheaper wire field is a boolean "this tile
  moved", with Godot still stepping a global phase while it is true. That
  is wrong at the moment a jam clears: the tile rejoins whatever phase the
  global clock has drifted to, and the rubber jumps a fraction of a tile
  under items that started moving smoothly. A counter resumes on the phase
  it stopped on, and the pattern crosses a tile in exactly the ticks an item
  does — the property the original formula had and the reason it was chosen.
  `tests/belt_surface.rs` pins both halves: frozen while jammed, and
  resumed-not-caught-up when the jam clears.

  **Amended 2026-09-25 (second amendment): the field is the TICK the tile
  last ran on, and every running belt is in phase.** The paragraph above
  weighed a one-off jump against nothing, because at the time every tile's
  count started at zero on the same tick and the drift was invisible. In a
  played factory it is not: belts are laid minutes apart, and any tile that
  has ever been held up is permanently a fraction of a tile behind its
  neighbours. The owner's words, after the first amendment shipped, were
  that the belts "need to all be synced up, they are a bit out of sync".
  A run of conveyor that is out of step reads as a row of separate machines
  bolted together, which is exactly the thing the surface is there to deny.

  So `tick_toward` assigns `current_tick` instead of incrementing. Nothing
  about the frame formula or the rate changes — the value still advances
  once per tick, so the pattern still crosses a tile in exactly the ticks an
  item does — and the jam still freezes it, since a jammed tile is simply
  not assigned. What changes is the recovery: a freed tile REJOINS the
  phase, and its pattern jumps up to three frames once. That is the trade
  made knowingly, and the two sides are not equal. The jump is one
  repeating pattern shifting on one tile, at the instant its items start
  moving and the eye is on them; the drift it replaces was on every tile,
  all the time, and never resolved. `every_running_belt_stands_at_the_same_phase`
  and `a_cleared_jam_rejoins_the_phase_the_running_belts_are_on` pin the new
  halves; the latter allows a few ticks of slack, because a line that has
  just been freed moves in stop-and-go waves and a tile that did not move on
  the last tick is honestly a tick behind.

  **Saved, at the current `schema_version`.** `SavedBelt::moved_ticks` is
  `#[serde(default)]`, like `SavedHopper::auto_fill` and `SavedMachine::load`
  before it — an additive field is not a shape change, and a save written
  without it loads at phase 0, which is where a freshly placed belt starts
  anyway. A reloaded factory's rubber therefore stands where the saved one's
  did rather than snapping under unmoved items.

  **The consequence accepted on purpose: an empty belt shows no motion.**
  Real conveyors run empty, and the first instinct is to keep the rubber
  going and stop it only when blocked. That is a third clock — "powered" —
  and the game has no such concept; more to the point it would make running
  rubber mean nothing, since it would be true of every belt ever placed.
  Under the §6a reading the motion is a *statement* that product is moving
  here, and on an empty belt that statement is false. The idle crusher is
  the precedent and the answer is the same one.

  **Amended 2026-09-25: that consequence was wrong, and an empty belt is a
  running belt.** The paragraph above is kept as written because the
  reasoning that produced it is exactly what the play test refuted. The
  owner built a long line and watched one box cross it: the rubber lit up a
  single tile at a time, walking down the run under the box like a spotlight
  — nothing a factory has ever done. The analogy to the idle crusher is what
  failed. A crusher with no tomato in it is *not doing its job*; a conveyor
  with no box on it is a motor turning, which is what a conveyor's job looks
  like when there is nothing to carry. "Powered" is not a third clock the
  game lacks: a belt the player has paid for and placed is powered, always,
  and the only state it has to report is whether it is BLOCKED.

  And the rule as written defeated its own purpose. Still rubber meant two
  different things — jammed, and nothing on it — and an empty belt is by far
  the commoner of the two, so the jam signal this whole record exists to
  build could not be read off the floor at all. Counting the idle tick makes
  still rubber mean exactly one thing. §6a is kept where it bites: the
  surface stops on a jam, because that is the tile's own state rather than
  its mere existence, and a jam is the statement worth making.

  **Why Godot extrapolates, and why that is not a second clock.** An idle
  tile dirties no chunk, deliberately and unchanged: a delta for every empty
  belt in the world thirty times a second is the cost this counter exists to
  avoid, and the reason for it in the first place. So `main.gd` records
  `moved_ticks - current_tick` when it last saw a tile empty
  (`_belt_idle_phase`) and adds the tick back to get the tile's clock on any
  later frame. That is EXACT rather than a guess, and only because the
  simulation's increment is exactly one per tick: the number the scene draws
  while a tile is idle is the number the tile's next snapshot brings, so the
  rubber does not jump when a box lands on it. The moment anything is on the
  tile the offset is dropped and the wire's value is authoritative again.
  The sim still decides; Godot is interpolating a rule it was given, the way
  it interpolates an item's position between ticks.

  **Exact only if the clock was right the one time Godot read it** (review of
  PR #177, 2026-09-25). The scene reads a placed tile ONCE — on the rebuild
  the placement triggers — and an idle tile dirties nothing afterwards, so
  there is no second look to correct anything with. A belt whose clock
  started at 0 was therefore drawn on the wrong frame for as long as it
  stayed empty, however right the simulation's own number became on the next
  tick. That is not a flaw in extrapolating; it is the reason a tile is now
  LAID ON A TICK (`BeltSegment::new`'s `laid_on_tick`, seeded from
  `World::tick_count` at the `Place`, and the save's own `tick_count` for an
  empty belt at load). The renderer's assumption is a contract the simulation
  has to honour at every entry point, not only in `tick_toward`.

  **What proves it.** `tests/belt_surface.rs` (eight cases, through
  `chunk_snapshot`, i.e. the wire Godot reads — including
  `an_empty_belt_keeps_its_rubber_running`, which pins the rate at one per
  tick and not merely "it moves", and
  `a_belt_laid_into_a_running_floor_is_in_phase_before_it_ticks`, which reads
  the clock with no tick between the placement and the read, because a tick
  of slack is exactly what hid the placement bug) and `test_belt_shapes.gd`
  sections 3 and 3b, which step the scene's own loop and assert each tile
  draws its own clock's frame, that over one full cycle the empty tile shows
  all four frames while the jammed tile beside it in the same frame set shows
  one, that a full re-read of the chunks brings the simulation's own number
  for the idle tile — the extrapolation, checked against the thing it
  extrapolates — and that a belt bought and dropped in 3 000 ticks late draws
  the same frame as one that has run since tick 0, for a whole cycle
  afterwards. Section 3b exists because every other belt in every fixture is
  placed before the first tick, so every idle offset in them is 0 and a scene
  that ignored offsets altogether passed the lot.
  `tests/chunk_delta.rs` pins the silence: an idle belt's clock advances
  every tick and it is still absent from the delta. Not provable headless:
  the `MultiMesh` transform writes underneath, because the dummy
  RenderingServer reads every instance transform back as the identity. That
  half is `scripts/screenshot-scene.sh --fixture chain --populate --closeup`,
  where the run's cleats stand one frame further on than the same world
  renders them without this change.
- 2026-09-20: **A width fix must be MEASURED at every window size, and
  `snap_rows` is for rows the player signs.** (Follow-up to the entry below,
  from the review of PR #102.)

  **The fault.** The entry below states the doctrine plainly — measure against
  `_modal_available_width()`, do not assume — and then two of its own fixes
  were applied unconditionally, which is the same mistake in the other
  direction. `_build_contract_board_row` put the offer's terms on their own
  line always. That is load-bearing at 1280x720: unsplit, the board's grid
  wants 1324px against 1264px available. At 1600x900 it is not: the unsplit
  panel is 1380px in a 1600px window, 220px to spare. The cost of splitting
  there is vertical — a card goes 120px to 157px, so three need 489px against
  a 484px cap, and `snap_rows` drops the board to TWO whole offers. The board
  showed one fewer offer than it did before the width pass, missing by five
  pixels, and the prose the split was protecting was never in danger at that
  size.

  **The fix.** `_fit_contract_board_split` measures the panel unsplit against
  `_modal_available_width()` and reparents the terms label into the mix row
  when it fits, into the text column when it does not — measured, per window,
  like everything else in this layout. At 1600x900 the board is back to three
  whole offers with the sentence on one line; at 1280x720 the rendered frame
  is byte-identical to before.

  **`snap_rows` is a cost, so it needs a reason.** Moving the cut down to the
  last whole row buys a tidy edge and pays height for it. The build menu was
  already exempted on the grounds that the half-row it shows past the cap is
  what says "scroll". The production panel asked for it and paid 161px at
  1600x900 — four rows where main had shown four plus a readable fifth. The
  general rule the two cases share: **`snap_rows` is for a stack whose rows
  are DECISIONS the player signs** — an offer on the board, a hire on the
  staff panel — where half a card is half a choice and the player might sign
  the half they can see. A production row is a readout. Nothing is signed, so
  nothing is lost by cutting one in half, and the half says "scroll" for free.
  The production panel drops it.

  **And one more per-plate branch.** `_panel_position_near` avoided the
  legend, the contract panel and `_stock_rect` but not `_crew_rect` — in the
  same commit that created `_crew_rect` and wrote into `godot_project/CLAUDE.md`
  that the right-hand column is ONE list and never a branch per plate. A
  tooltip near the cursor landed on the wage bill. The entry is added, and
  `test_hud_plates.gd` now loops the column rather than naming one plate, so
  the next tenant of that edge is caught by a test that already exists.
- 2026-09-20: **Never out-darken the game's own strongest honest presentation
  of the same geometry.** This restates a rule given earlier in the same
  review — "never out-darken the game" — because the naive form of it turned
  out to have an accident in it, and the accident produced a pass nobody could
  see.

  **What the rule protects.** The asset browser is where a model is approved.
  If it shades a form harder than the game can, an artist signs off on a
  silhouette that only reads under browser-only occlusion, and the machine
  ships flat. So the browser's baked AO is calibrated *down* to the game
  rather than tuned by eye, and `AO_STRENGTH` is a measured quotient:
  `calibrate-ao.py` renders the game's own vat fixture with SSAO on and off,
  renders the browser on the same model with AO on and off, and divides the
  shading per machine pixel. Nobody chooses the number; the two renderers do.

  **Where the naive form broke.** Godot's `ssao_radius` is in WORLD units, so
  the retuned 0.45 m radius is 4.5 voxels of a model — but the play camera is
  isometric and sees a whole factory, so those 4.5 voxels are a couple of
  pixels on screen. Measured on the same vat at the same settings: standing
  beside it, 67.8% of the machine's pixels darken by more than 1% and 4.36% of
  its light is removed; at the isometric play camera, 0.3% and 0.01%. A
  strength calibrated against 0.01% is about 0.05 — a darkening of under one
  8-bit level on the greys these models are painted in, which is not a match
  but a rounding error. "Never darker than the game" read literally therefore
  demanded a feature that is switched on and invisible, which is exactly the
  outcome issue #49 exists to prevent.

  **What was decided, and why this pose.** Calibrate against the game standing
  beside the line, at eye height, facing the machine. The deciding argument is
  that the bake is **object-space**: it is a property of the model, and the
  model is the thing the browser exists to show. The play camera does not fail
  to show the occlusion because the occlusion is not there — it fails because
  its pixel scale cannot resolve it. Deriving where the effect cannot resolve
  bakes a camera's limitation into an artist's inspection tool.

  **The conditions attached, all of them binding.** (1) Publish the gap: the
  isometric numbers stay in the table, and the README and the PR state which
  pose the constant came from, at what settings, and that a play-camera
  screenshot is flatter on purpose — an artist must find the explanation
  before filing it as a bug, not after. (2) State the settings beside the
  constant: `ssao_radius 0.45 / ssao_intensity 16.0 / ssao_power 1.6`, PR
  #104's retune, which merges first. (3) The refusals stay: the coverage gate,
  the staleness check against `factory_environment.gd`, `test-ao.mjs`, and the
  requirement that the strength settle. (4) #49 is closed by a pass a person
  can see, shown as a before/after at the browser's own default view.

  **Two findings from the same class, recorded because the next port will
  repeat them.** `ssao_power` had not been ported at all. It is a gamma on the
  occlusion term, which is the difference between a crease finder and a wash,
  and porting it alone cut the browser's coverage from 24.8% of the machine to
  9.1%. No strength could have corrected it: a strength scales every darkened
  pixel alike, so a shape mismatch survives any amount of scaling. Port every
  setting of a pass or none. Second, `test-ao.mjs` had assumed `AO_RADIUS` was
  a whole number; at 4.5 it indexed the occlusion field at fractional
  coordinates, and three assertions compared `undefined` instead of failing.
  A test that passes by rounding is not a test.

  **And the frames are not trusted either.** `screenshot-scene.sh`
  occasionally returns a frame captured before the fixture is built or before
  the pose is applied — an empty hall at the default framing, which looks like
  a perfectly good PNG and once got as far as writing a constant to
  `web/ao.mjs`. A believable before/after has the machine covering a sane
  share of the frame, has the pass-on frame keeping 95% of the pass-off
  frame's machine pixels (SSAO moves no geometry), and has the pass only
  removing light. Anything else is re-shot three times and then fails the run.

  Option "ship it switched off" was considered and rejected: it would close
  #49 on paper while leaving the browser exactly as it was.

- 2026-09-20: **Glow means "this is emitting", not "this is bright" — and
  SSAO means "this is a crease".** Issue
  #32. The hall's glow settings — `glow_intensity 0.7`, `glow_bloom 0.15`,
  `glow_hdr_threshold 0.85`, SOFTLIGHT — were chosen for UI_UX §4's "mild
  bloom on EMISSIVES" at a time when **nothing in the game emitted**: issue
  #25 meant `StandardMaterial3D` multiplied its emission by an absent texture,
  so every status lamp was ordinary painted amber at any energy. The pass was
  tuned against an input that did not exist, and #25's fix (the
  `voxel_emissive` shader) changed what it is fed without anyone re-reading
  the frame.

  **How it was measured, and two traps in measuring it.** Glow has no numeric
  read-out, so each setting was scored by differencing two rendered PNGs and
  taking mean channel deltas inside a box around one lit status lens against
  the whole rest of the frame — *selectivity*, the ratio of the two.

  The first trap is the baseline. Baselining against `glow_enabled = false`
  gives nonsense (the lens appearing to gain *less* than the floor), because
  enabling the pass at all shifts the image by about +4.5 mean luma
  independently of whether anything crosses the threshold. The fix is to
  baseline against a render with glow **enabled and contributing nothing** —
  but a high threshold is not enough on its own: `glow_bloom` blooms
  everything *below* the threshold, so a baseline that keeps `glow_bloom` also
  keeps the haze and every reading taken against it is wrong. **The null
  baseline is `glow_bloom = 0.0` with `glow_hdr_threshold = 3.0`.**

  The second trap is the renderer. `scripts/screenshot-scene.sh` lets the
  scene's own `_process` run for 45 frames before it saves, and
  `_advance_simulation` ticks on the WALL CLOCK, so how many simulation ticks
  pass between building the fixture and taking the shot depends on machine
  load. Two renders of *identical* settings differ in where the items on the
  belts are, and that difference swamps any glow delta — six of eight
  variants in one sweep failed to reproduce between two passes. This sweep was
  therefore rendered by a throwaway harness that disables the scene's process
  mode and counts the ticks by hand; every render is then byte-identical run
  to run. Filed separately as its own issue — **fixed on 2026-09-25**: the
  review render pins the engine clock with `--fixed-fps`, so a sweep like
  this one no longer needs a harness of its own (see that record below).

  **The sweep** (bottleneck line, crusher closeup at tile 4,0 size 10,
  1200x800, tick 10060; deltas in 0-255 levels against the null baseline):

  | intensity / bloom / threshold / blend | lens box | max | rest of frame | selectivity |
  |---|---|---|---|---|
  | 0.7 / 0.15 / 0.85 / SOFTLIGHT *(shipped)* | +3.22 | +29 | +1.21 | 2.7x |
  | 0.7 / 0.0 / 0.85 / SOFTLIGHT | +2.12 | +28 | +0.15 | 14.6x |
  | 0.7 / 0.0 / 1.2 / SOFTLIGHT | +1.32 | +25 | +0.04 | 34.9x |
  | 1.0 / 0.0 / 1.2 / SOFTLIGHT | +1.99 | +30 | +0.04 | 48.2x |
  | 1.0 / 0.0 / 0.85 / SCREEN | +21.58 | +218 | +0.79 | 27.3x |
  | 1.0 / 0.0 / 1.0 / SCREEN | +19.23 | +217 | +0.52 | 37.2x |
  | 0.7 / 0.0 / 1.2 / SCREEN | +12.29 | +199 | +0.17 | 71.8x |
  | **1.0 / 0.0 / 1.2 / SCREEN** *(new)* | **+16.12** | **+213** | **+0.23** | **69.1x** |
  | 1.0 / 0.0 / 1.4 / SCREEN | +13.14 | +208 | +0.07 | 179.5x |
  | 1.0 / 0.0 / 1.2 / ADDITIVE | +16.52 | +213 | +0.25 | 66.4x |

  **What the sweep says.** `glow_bloom` is the whole of the frame-wide haze
  (rows 1-2: zeroing it alone takes contamination from +1.21 to +0.15), which
  is why the old settings fired on the crusher's white lid, the jars' froth
  and the dock's box labels and softened all three — a legibility cost (§2b)
  paid for an effect meant to be about lamps. The blend mode then decides
  everything: **SOFTLIGHT's brightest single pixel is +25..30 at any
  intensity and any threshold**, because its effect scales with the pixel
  beneath it and a lit lens is already bright, so it cannot make a halo at all
  — the lens stays a flat cream cube however it is tuned. SCREEN takes the
  same lens to +213. Threshold 1.2 is the knee rather than a guess: 1.0-1.2 is
  where the brightest *lit white* surface in the game sits, below it
  contamination climbs fast (+0.79 at 0.85) and above it the lamp is what
  pays — 1.4 buys +0.07 instead of +0.23, a difference already under one luma
  level, for 19% of the halo. Intensity went 0.7 -> 1.0 last, worth
  +16.12/+213 against +12.29/+199 for +0.06 of contamination. ADDITIVE was
  measured and is within noise of SCREEN; SCREEN is kept because it retains
  marginally more hue in the lens (saturation 0.36 against 0.34) and is
  bounded by construction, so a brighter emissive added later cannot push a
  lens past white. (The earlier claim that ADDITIVE clips and spills is not
  supported by the measurement: clipped pixels in the lens box are 3.0%
  against 3.1%.)

  **Result:** 1.0 / 0.0 / 1.2 / SCREEN. The halo around a lit lens goes from
  1480 px at rgb(6,2,1) to 7136 px at rgb(18,9,2) — five times the reach,
  three times the strength, amber either way (saturation 0.87), which is §4's
  "a lit voxel glows in its OWN colour". Overall exposure is unmoved (mean
  luma 135.1 -> 134.4 on the closeup, 153.8 -> 155.0 in first person), and
  `SCREENSHOT_RENDER` is identical (objects 1189, primitives 4538006, draw
  calls 233 on the stress line): a full-screen post-pass costs the same
  whatever it is tuned to. Read across a bright frame and a dark one: in
  first person the old settings lifted the roof lamps' own *faces* (+2.43)
  and barely their surroundings (+1.67), which is why they read as flat white
  rectangles indistinguishable from the skylight panels; the new ones leave
  the faces alone (+1.91) and light the dark roof around them (+7.26), which
  is what a lamp does.

  **The SSAO in the same block had the same defect**, and was retuned with it
  after the asset browser's worker (issue #49) measured the game's pass while
  calibrating a baked per-voxel AO pass against it and found it near-inert: on
  a vat fixture, toggling `ssao_enabled` moved the isometric closeup by at
  most 5/255 and darkened 0.01% of pixels by more than 3%. That is the same
  failure as the glow — `ssao_radius 0.9 / ssao_intensity 2.4 / ssao_power
  1.6` was a number chosen where it could not be seen and never checked
  against a frame.

  Reproduced on the bottleneck line at a pinned tick (the same deterministic
  harness the glow used, for the same reason), differencing rendered PNGs
  against `ssao_enabled = false`. At the shipped settings the deepest of three
  hand-picked crease boxes — a pot shoulder, a gantry arm against a pot, a
  crusher leg — lost **1.7 levels out of 133** (x0.987), mean over the three
  1.2 levels, with 0.22% of the frame darkened by more than 3%. On a flat
  voxel face, 1% is not a shading cue; it is nothing.

  **The finding that changed the answer: the pass is not uniformly weak.**
  `ssao_radius` is in world units, so how much occlusion it finds per pixel
  depends on how near the geometry is to the camera — and this game has two
  cameras that differ enormously in that. The same setting that is invisible
  under the orthographic camera looking down from across the hall is already
  doing real work at eye level. At the shipped 0.9 / 2.4, first person
  standing beside the line already darkens 7.8% of the frame by more than 3%,
  against isometric's 0.22%. So "raise the intensity until isometric reads"
  produces a first-person view that is filthy.

  Measured, 1200x800. *iso crease* is the mean loss over the three boxes;
  *smear* is the share of the frame darkened by more than 3%, which is where
  overshoot shows up before the eye catches it; *crease:flat* is mean
  darkening within 3 px of a luma edge over mean darkening elsewhere.

  | radius / intensity | iso crease | iso smear | eye smear | crease:flat |
  |---|---|---|---|---|
  | 0.9 / 2.4 *(shipped)* | -1.2 lv | 0.22% | 7.8% | 2.0x |
  | 0.9 / 8 | -4.5 lv | 1.40% | 15.3% | 1.9x |
  | 0.9 / 16 | -7.8 lv | 2.83% | 18.7% | 1.8x |
  | 0.45 / 8 | -2.1 lv | 0.41% | 10.2% | 2.3x |
  | **0.45 / 16** *(new)* | **-3.7 lv** | **0.87%** | **12.9%** | **2.2x** |
  | 0.45 / 24 | -4.8 lv | 1.16% | 14.0% | 2.1x |
  | 0.3 / 16 | -2.1 lv | 0.50% | 9.4% | 2.3x |

  `ssao_light_affect` was tried at 0.3, 0.6 and 1.0 and changed the rendered
  frame by **nothing at all** — byte-identical measurements — so it is not
  the lever it looks like and is not set.

  **Result: 0.45 / 16.0 / 1.6.** Halving the radius is what reconciles the two
  cameras: it costs isometric bite but takes much more off the first-person
  smear, because that smear is the far end of a radius that was never
  crease-sized to begin with. 0.45 world units is 4.5 voxels — *a fitting
  against a body*, not a machine against the floor — and the crease:flat ratio
  confirms it, rising from 2.0x to 2.2x in isometric and 2.5x to 2.6x at eye
  level while the bite triples. Read at three framings, not two: isometric
  closeup, play zoom (where the tomatoes gain a tight dark seat on the belt
  rubber instead of floating), and first person standing beside the line. The
  rejected 0.9 / 16 is the instructive frame — in first person it swallows the
  belt's legs into a single dark smear and hangs a halo off the belt's lower
  edge, which is exactly the "dirty-looking model" this is supposed to avoid.

  **A trap for whoever measures this next**: a first-person shot aimed across
  the empty hall shows no SSAO change at *any* intensity, because there is no
  geometry near the camera to occlude anything. That is the pass behaving
  correctly, not evidence that it is inert. The eye-level numbers above come
  from a pose standing beside the line, not from the default first-person
  pose `test_screenshot.gd` reaches.
- 2026-09-20: **A tank declares what it holds; the save format's
  field-addition window is closed.** Two issues, one mechanism, and one
  question that had to go back to the owner.

  **The hole (#43).** `resolve_network_fluids` names a pipe run by arc
  consistency: a run a tank discharges into can only carry a *raw* fluid, a
  run off a machine's outlet face can only carry what that kind's recipes
  make, and the feasible recipes of the machines touching a run narrow it
  further. That is enough for every layout the shipped chain has. It runs
  out on one machine drawing **two** raw fluids from **two** tank-fed runs:
  both runs seed to the same pair of raws and nothing in the graph separates
  them. Worse than stuck, in the single-recipe case it is *arbitrary* —
  `feasible_naming` matches inputs to runs first-fit, so the runs are paired
  off in tile order, and the player whose vinegar drum is on the other side
  has no way to say so. Add a second recipe on the kind drawing one of the
  two raws (a mixer that also makes plain syrup — ordinary content) and the
  first-fit pairing stops being unique, no run narrows to one id, both go
  `Ambiguous`, every machine on them stalls, and neither tank can even be
  filled, because a fill needs to know what it is pouring.

  **Why a declaration and not a cleverer resolver.** There was no bug in the
  propagation to fix. Which drum the player filled with what is a fact about
  the world that exists only in their head; no amount of graph reasoning
  recovers information that was never in the graph. So the tank is asked.
  `PlayerCommand::SetTankFluid { tile, fluid }` sets `FluidTank`'s optional
  fluid, `World::recompute_topology` collects the declarations per network
  and hands them to `resolve_network_fluids`, and a declared run's possible
  set is **replaced** by the declaration before step 2 narrows anything.

  Replaced rather than intersected, deliberately: a declared tank names its
  run even when nothing is plumbed to it yet, so the player can fill a drum
  before building what drinks from it — which they could not do before.
  Disagreement is not silently resolved either: two tanks declaring
  different fluids into one run leave two ids and the run reads `Ambiguous`,
  and a declaration no machine there can use narrows to nothing and reads
  `Unnameable`. Both are the graceful unnamed-with-a-reason state issue #54
  built, not a new failure mode. An **undeclared** tank is untouched — its
  fluid is still whatever its run turns out to carry — which is what keeps
  the shipped chain a chain nobody has to label, and what
  `an_undeclared_tank_still_takes_its_name_from_the_graph` pins.

  **The load-time consequence, found by the test.** Loading rebuilds
  structure through real `Place` commands and only then restores bodies. A
  declaration arrives with the bodies, so every network had already been
  named as though no tank had declared anything, and the very layout #43
  exists for came back ambiguous and stuck however it was saved.
  `World::from_save_with_content` now calls
  `recompute_topology_after_restore` once the bodies are in. Pipe volumes
  survive it: the pass carries them across by tile, which is what lets it
  run on a live world at all.

  **The save format (the owner's call, asked and answered).** The ticket
  asked for "version and migrate the save", which contradicts
  ARCHITECTURE §4 and `save.rs`'s own "rejected rather than migrated —
  copying, deliberately, what every other loader in this crate does", so it
  went back to the coordinator rather than being guessed. Three options
  were on the table: (A) spend `save.rs`'s field-addition window and add
  `SavedTank::fluid` as a `#[serde(default)]` with no bump; (B) build the
  crate's first migration engine; (C) bump and reject.

  (A) is spending a window that is already closed: `save.rs` said "nothing
  has shipped with a save file yet, so schema_version 1 is whatever this
  batch of work merges with" and then closed it explicitly — "after this
  batch ships, a field addition is a version bump like any other shape
  change" — and that batch (`auto_fill`, `SavedMachine::recipe`,
  `lifetime_deliveries`) has been on `main` for a fortnight. (B) contradicts
  two design docs for one ticket's convenience. **(C) is what the docs
  already chose**: `SAVE_SCHEMA_VERSION` is 2, a v1 save is refused, and the
  refusal names both numbers so it reads as "this save predates the tank
  fluid change" rather than as a parse failure. The cost is bounded and will
  never be this cheap again — no release has ever written a save. The
  Forward-compatibility note in `save.rs` was rewritten so the next person
  does not re-litigate it.

  **Content validation moved too (#42), for a related reason.**
  `validate_chain` demanded that every recipe be reachable from recipe 0 by
  a shared ingredient id, which would have panicked at construction the
  moment mayonnaise (eggs + oil, GAMEPLAY §2 tier 4) landed in
  `recipes.ron`: a second production line shares nothing with tomato. The
  rule is now per connected component, and what each component must contain
  is a recipe that can actually be *started* — its machine is one
  `data/shop.ron` sells, and its inputs are obtainable, where obtainable is
  the fixpoint of priced raws plus the outputs of recipes that are
  themselves startable. Several independent lines are fine; a line with no
  way into it is not, and neither is one on a machine nobody sells. The
  weaker "does any recipe in the component take a raw" rule was tried first
  and rejected: a mid-line recipe with no raw inputs anchored a dead
  component vacuously. `machines.validate_covers` now runs *before*
  `validate_chain`, because "this kind has no footprint" is the more basic
  fault and should be the message the player gets.

  **Amended in review of the pull request: a declaration change destroys the
  run as well as the tank.** The first cut emptied only the `FluidTank`, and
  that was measurably not enough: `recompute_topology` rebuilds the fluid
  networks and *carries volumes across by tile*, so the pipe run the tank
  feeds kept everything it held and the rebuild simply renamed it. The
  reviewer measured 59.9968 units of bought vinegar becoming liquid smoke, on
  shipped content, through the shipped picker, never purchased. Two fixes
  were possible. **Refusing the switch while the run is non-empty** was
  considered and rejected: a player who mis-declares a tank would then have no
  way to drain the run they no longer want — the machines on it will not drink
  the wrong fluid, so nothing empties it, and the mistake is permanent. So the
  emptying is carried *through* the recompute instead:
  `World::try_set_tank_fluid` calls `purge_runs_fed_by`, which finds every
  network the tank discharges into via `topology.tank_discharges`, calls
  `FluidNetwork::purge` on each and dirties the pipe tiles, all *before*
  `recompute_topology` runs, so the rebuild carries zeroes across. The volume
  is **destroyed, not credited and not moved** — there is no refund and no
  transfer to a neighbouring run; `switching_a_declared_tank_purges_the_run_it_fed_rather_than_relabelling_it`
  asserts conservation in the negative by summing every *other* run and tank
  before and after and requiring the total to be unchanged. Destruction is
  acceptable only because it is consented to, which is the UI half of the
  rule: the picker's note names what the click will cost before it happens —
  "switching destroys 89.7 units — this tank and the run it feeds" when there
  is something to lose, and the unquantified "switching empties this tank and
  the run it feeds" when there is not (`main.gd`'s `_tank_picker_note` /
  `_tank_doomed_volume`; section 10c of `test_config_panel.gd` computes its
  expected number straight off the wire, because an assertion built from the
  same helper it is testing passes even when the helper ignores the run — that
  was tried, and it did).

  **And an unusable declaration blames the tank, not the outlet.** The code
  claimed a declaration no machine on the run could use would read
  `Unnameable`; it did not — the run read `Carries` and the machine
  downstream stalled, so the blame landed on an innocent outlet. Step 2b of
  `resolve_network_fluids` now clears a declared run's candidate set when no
  recipe of any machine kind drinking from it takes that fluid, which makes
  the run read `Unnameable` and points the player at the tank they just
  mis-declared. It tests the machine **kind**'s recipes, not the layout as it
  stands this instant, so a run does not flicker `Unnameable` while a line is
  half laid; a run nothing drinks from yet, or one fed by a machine outlet, is
  left alone.
- 2026-09-20: **A modal panel yields WIDTH before it is clipped, and an
  unreachable control outranks every other layout rule.** (Issues #62, #63,
  #69, #17.)

  **The fault.** The 2026-09-09 scale change taught every list to yield
  HEIGHT: `_fit_grid_scroll` caps it and it scrolls, UI_UX §5c's deliberate
  last resort. Nothing ever made a panel yield WIDTH, because at the shipped
  1600x900 nothing needed to. At 1280x720 — the smallest window the project
  claims to support — three panels were simply wider than the screen and were
  drawn off both edges of it: the staff panel wanted 1350px, the build menu
  1412px, the contracts board 1380px. Clipping a centred panel takes the same
  amount off each side, so what went was the outermost column on each: the
  icons on the left, and on the right the FIRE −1 button — the only way to
  undo a hire — and SIGN, the only way to take a contract. The panel looked
  fine in the middle. You could not play the game from it.

  **The rule that follows.** §5f already ranked layout faults worst-first: a
  panel drawn ON another panel, a card cut through a line, a list showing less
  than it could. None of them covers this, and this is worse than all of them:
  a panel over a panel still leaves both readable if you close one, a cut card
  still shows what it shows. **A control the player cannot reach is the worst
  thing a layout can do**, and it goes on top of that list. Practically, that
  makes width a budget like height: `_modal_available_width()` is the viewport
  less `MODAL_EDGE_MARGIN` on each side, and every modal now measures itself
  against it. The margin is in WINDOW pixels rather than `_ui_px` on purpose —
  it is the last few pixels before a control is off the screen, not a design
  gap that should scale with the UI.

  **What is spent, in order.** Three things can give width back, and they are
  ranked by how much the player loses.

  1. *A sentence moves to the hover.* The staff row already ranks five
     sentences and shows one, the rest on the row's tooltip (issue #7); at a
     window this narrow the fifth goes there too, along with the per-tick
     "3 of 4 working right now" and the meter's "hired / asked for" caption.
     Nothing is lost — it is all one hover away, where four fifths of it
     already lived — and what stays on the row is the icons, the meter, the
     numbers, the state chip and the two buttons: the parts that can be
     clicked. `_fit_staff_width` measures rather than assumes, so a wider
     window shows the prose again and a future row that outgrows the window
     gives its prose up with no breakpoint to re-tune. The wanted/shown split
     (`note_wanted`, `working_wanted`) exists because the poll and the width
     rule both want to set `visible`, and only one of them may own it.
  2. *A grid of cards takes fewer columns.* The build menu and the market are
     fixed-size icon slots; there is no prose in them to move. `SHOP_SLOT_COLUMNS`
     had already been hand-tuned three times as the catalog grew, which is the
     same decision made worse each time. `_fit_grid_columns` asks the panel
     whether it fits, one column at a time, at most six tries of a cached
     minimum-size read — measuring the *panel*, not dividing the window by a
     card, because the real width is the grid plus margins plus scrollbar plus
     whichever of heading and footer is longest, and every one of those has
     moved. Then it balances: five columns for six crates is a row of five and
     a lonely sixth where 3+3 is two even rows, and fewer columns than the
     width allows is always still narrow enough, so balancing cannot undo the
     fit. Both constants now read as *the most* a grid takes, not the number
     it always takes.
  3. *The list scrolls.* Already true.

  **What is never done: `autowrap_mode` on a Label a fit pass MEASURES.** The
  obvious fix for a 1030px sentence is to let it wrap. It cannot be used
  there. A wrapping Label reports a minimum width of one pixel and a minimum
  height shaped at whatever width it last had — and a Label is only reshaped
  when it is DRAWN. Under `--headless`, where nothing draws, a wrapped
  sentence measures 1x1609 and every panel sized around it is nonsense. This
  layout is measurement-driven end to end (`_fit_grid_scroll`,
  `_whole_rows_height`, `_fit_grid_columns`) and its regression proof is a
  headless test, so a control whose size cannot be measured without drawing it
  cannot be part of what is measured. Where such a sentence must get narrower
  it is **broken at a join it already has**: the board's offer is a mix label
  and a terms label, so the column is as wide as its longer half rather than
  as wide as both. A break at a fixed point, not a reflow.

  The rule is about the MEASUREMENT, not the property, and the absolute
  version of it was false the day it was written: 14 `autowrap_mode`
  assignments were already live in `main.gd` and are still there — the shop
  and market slots, the crew affordance line, the configuration panel and the
  pause menu. They are safe for a reason worth stating, because it is the
  reason: every one of them has its width PINNED before anything measures it,
  so the 1px minimum is never what a fit pass reads. Seven sit inside a
  fixed-size ancestor — `SHOP_SLOT_SIZE` (`main.gd:5719`) and
  `MARKET_SLOT_SIZE` (6312) — and the contents-picker label under
  `CONFIG_PICKER_SLOT_SIZE` (10177); the other six set their own
  `custom_minimum_size.x`: the three configuration-panel notes take
  `CONFIG_PANEL_WIDTH`, the crew affordance line `CREW_TEXT_WIDTH`, the two
  pause-menu labels `PAUSE_BUTTON_MIN_SIZE.x`. Note that the configuration
  panel IS measured by a fit pass — `_position_config_panel` reads
  `get_combined_minimum_size()` and positions against the right-hand plates
  from it — so "not sized by a fit pass" is not the exemption; the pin is.
  Stated as an absolute, the rule sends the next
  reader to delete working code or to spend a session proving the document
  wrong (raised in the review of PR #102, corrected 2026-09-20).

  **The measurement trap, twice.** The same "shaping happens on draw" rule bit
  in two more places, and both are worth knowing before trusting a number.
  First: **headless font metrics are narrower than a real window's.** The
  staff panel measured 1339px against 1264px available headless, passed the
  test, and was still clipped in the render at exactly that size; the fix
  (dropping the 262px per-tick figure as well) was found by instrumenting
  `_fit_staff_width` during a real `scripts/screenshot-scene.sh` run.
  A headless assertion is a regression net. **The render is the acceptance.**
  Second: `get_combined_minimum_size()` can UNDERREPORT what is on screen —
  the machine hover reported 299x285 while being drawn at 545x285, and
  `_panel_position_near` placed it as if its right-hand 246px did not exist,
  over the legend's last line. The panel's *current* size is what the layout
  pass actually gave it, so `maxf(minimum, size)` on both axes is the honest
  estimate, and erring large only ever buys clearance.

  **When nothing fits.** At 1280x720 with the legend open there is genuinely
  nowhere for a machine hover to stand: 550x283 of tooltip, a 757x259 legend
  starting 24px in, 499px to its right, 24px above and — after the hotbar
  takes its strip — 282px below, one pixel short. (Figures as recorded then.
  Measured again for issue #99, the tooltip is 456x283 and the band 235px:
  48px short, not one.) Pushing off one plate at a
  time is no answer: each push clears the plate it is given and can land on
  the next, and one pass across three plates put the tooltip over two at once.
  So when every position is a fault, `_panel_position_near` scores every
  position it can reach — the four quadrants and the four ways off each plate,
  each after the clamp — by total overlapped area (`_hud_overlap_area`) and
  keeps the smallest, ties going to the earlier preference so §2b point 33's
  ordering still decides whenever the overlap is equal. The remaining overlap
  at that one window size is filed rather than fudged: the real answer is a
  legend that folds itself or a shorter tooltip, which is a content decision.
  (Taken 2026-09-25, issue #99: the tooltip sheds its `crew:` line and then
  its load bar — see UI_UX.md §5c.)

  **The same lesson, a third time: the configuration panel** (issue #88, folded
  in because it turned out to be this fix and not a separate one). The panel a
  click on a hopper or tank opens has always had a documented bound meant to
  keep it off the right-hand column, and it was failing in two ways at once.
  The bound named the clipboard and the stock plate in two hand-written
  branches; the CREW plate arrived on that column in the 2026-09-16 playtest
  pass and nobody added a third branch, so a panel that cleared the other two
  was drawn over the wage bill and the payday countdown. And the branches
  tested an UNCLAMPED `y`: a fluid tank's panel is taller than the band between
  the clipboard and the hotbar, so the final clamp lifted it up into rows the
  test had just decided it was nowhere near — §2b point 33's "judge after the
  clamp", in the one panel that had not learned it. Even with both fixed the
  flip itself was wrong: `anchor.x - offset - size.x` is a fixed displacement,
  not a guarantee, and at 1280x720 it landed the panel's last 15px on a
  clipboard whose left edge is 580px in. It now clamps `y` first, loops one
  list of three rects, and flips the panel's right edge ON to the bound.
  The legend stays off that list for `_position_market_panel`'s reason: at
  1280x720 no position clears both it and the column, and covering a static
  keybind reference the player can fold with F1 beats covering a live readout.
  The headless assertion has to point the camera AT the building first — the
  panel is anchored to the building's projected position, so a building off the
  side of the frame puts the panel on a window edge and the check passes
  vacuously, which the first version of it did.

  **Two smaller rules from the same pass.** The staff panel's scroll box is
  now FLAT — rows and explainer in one `VBoxContainer` — because
  `_whole_rows_height` snaps between the CHILDREN of the box it is given, and
  with the rows nested one level down its only choices were "the whole list"
  and "a cut through the explainer's own lines" (issue #63). Flat, every
  boundary a reader can see is a boundary the cut can land on, and the
  explainer is shown whole or not at all. And the clipboard's board hint is a
  line the docket stack may spend: `_update_contract_panel` — the only thing
  that knows what the stack got — hides it when the cut it made falls below
  the stack's first line boundary (`_first_line_boundary`), which is exactly
  what buys the cut back onto a boundary at 1280x720 with every offer signed.
  That was the case §5f had conceded to issue #17, and it closes it. "Sign
  more orders" is also the least useful sentence on a clipboard whose every
  order is signed. Issue #13 made the same trade against the SHORT-STAFFED
  strip; the difference is that there is now exactly ONE place that decides,
  so `_update_staff_alert` no longer touches the hint.

- 2026-09-20: **The bank at the moment of building decides whether the BBQ
  branch costs you the board.** Issue #55: building the BBQ branch off a
  running ketchup line starves the starter contract while the new pipes prime,
  and `corner_shop` strikes out and is lost. The issue's own cheap fix was
  "let the tier arrive with breathing room" — raise `after_deliveries` in
  `data/unlocks.ron` until the player has slack by the time BBQ lands. This
  pass swept the number, kept **10** unchanged, and **left #55 open**.

  **This record was corrected on the day it was written** (review of PR #98).
  The first version claimed the whole board is lost at *every* gate tried,
  including a 50 000-tick run, and concluded categorically that no gate value
  can help. Neither claim reproduces. What went wrong is worth keeping: the
  runs behind it were not all the same experiment, because the PR never
  stated **when the extra contracts were signed**, and that single unstated
  variable moves every number below. A measurement whose protocol is not
  written down is not a measurement.

  **The protocol.** Every run: build `test_layouts.gd`'s reference ketchup
  line through real purchase and place commands; fund it; stock the larder
  with 1 200 of each raw; leave hopper and tank auto-fill on; hire until fully
  staffed; sign the starter contracts **at the stated moment**; wait for the
  stated lifetime-delivery count; build the BBQ branch and its mixed exit;
  settle 15 000 ticks; read `economy_snapshot_json` and
  `production_snapshot_json`, sampled every 250 ticks. "Bank" is finished
  `boxed_ketchup` on hand at the moment the branch goes in. A gate value other
  than ten is **emulated** by waiting for that many lifetime deliveries before
  building — the shipped gate is long open by then and nothing else reads the
  number, so it is exactly equivalent and needs no rebuild per value.

  **What the transient costs.** `boxed_ketchup`'s `produced_total` flatlines
  for about **8 500 ticks** from the moment the branch is placed: the two new
  pipe runs and the second cooking vat fill from the *existing* fluid network,
  and roughly **35 boxes' worth of throughput** disappears into pipework
  before any of it comes back. Delivery windows keep counting through it;
  three misses lose a contract.

  **The corrected sweep.**

  | when contracts were signed | gate | bank at build | lost to the branch |
  |---|---|---|---|
  | one order only | 10 | 22 | none |
  | three, at the gate | 10 | 22 | `diner_supply` |
  | three, at tick 0 | 10 | 7 | `corner_shop`, `diner_supply` |
  | three, at tick 0 | 20 (emulated) | 12 | two |
  | three, at tick 0 | 30 (emulated) | 19 | one |
  | three, at tick 0 | 40 (emulated) | 24 | **none** |
  | one order, soaked to 50 000 ticks | life 48 | 46 | none |
  | three at tick 0, soaked to 50 000 | life 68 | 43 | none |

  **The control that the first version was missing.** With three orders signed
  at tick 0 and **no branch ever built**, `market_stall` still strikes out at
  tick 1 500. That death is the ketchup line's own start-up, not the BBQ
  branch, and the first version counted it against the branch. Signing three
  orders at the gate instead, with no branch, loses nothing.

  **The rule, stated from what reproduces.** The deciding variable is the
  **bank at the moment of building**. The number of concurrent orders matters
  only through the bank it leaves: three orders signed at the gate bank the
  same 22 as one order and lose one contract; the same three signed at tick 0
  bank 7, because the early windows ate the stock, and lose two. A delivery
  count steers that bank *by proxy, and badly*, because signing more contracts
  is both how a player reaches the count sooner and what makes the transient
  cost more. **Pushed far enough the gate does work** — 40 deliveries leaves
  the three-order player 24 banked and costs them nothing — so the categorical
  "no number can fix this" is false, and was removed from `data/unlocks.ron`,
  `docs/GAMEPLAY.md`, `docs/DECISIONS_LOG.md` and this record.

  **Why ten is kept anyway.** Forty deliveries is not "inside a first
  session", which is the entire reason ten was chosen (see the file's comment
  above the number). Buying #55 with a gate of 40 pays for a transient with
  the thing the gate exists to deliver, and it hides a missing affordance
  behind a balance value that reads as arbitrary to whoever meets it next —
  the worst kind of number, one that is load-bearing for a reason nobody can
  see.

  **What was NOT chosen, and why.** Priming the pipes on placement was ruled
  out before this pass: it breaks fluid conservation, the one property the
  fluid system exists to guarantee. Shortening the pipe runs in the fixture
  would fix the fixture, not the game. Giving `corner_shop` more strikes moves
  the same cliff somewhere else and weakens every contract in the game to
  patch one interaction.

  **What the fix actually is**, both outside this pass's ownership and both
  filed: a warning at placement time, before the player commits (#92), and a
  fluid network that will not hand a brand-new consumer 100% of a network the
  old one is living on (#93). **#55 stays open**, with these measurements
  posted on it. The table and the protocol live in `data/unlocks.ron`'s own
  comment so the next person to reach for that number meets the evidence
  first, and the player-facing shape of it is in `docs/GAMEPLAY.md` §6.

- 2026-09-20: **A legibility mask must be proved to contain the pixels in
  question, before any number it produces is quoted.** Issue #57 asked whether
  `box_of_ketchup` and `box_of_bbq` can be told apart once both ride one belt.
  The `voxel-legibility-measurement` method answers that kind of question with
  a fixed silhouette mask, mean luma inside it, 1:1 crops never upscaled to
  judge. The first answer this pass produced was **wrong**, and catching it
  is the durable part of the record.

  **The broken instrument.** The mask was built with the warm-pixel test
  earlier passes used: `r > g > b and r - b > 18`. That is right for kraft
  cardboard and wrong for the print on it — `label_cream` renders at
  **(241, 239, 234)**, `r - b = 7`, near-neutral. The mask therefore excluded
  the printed lid panels: *exactly the pixels the question is about.* It
  scored every candidate on the cardboard around them, reported a **5.3/255**
  gap against a 17.0 noise floor, and concluded the art fails. A comment
  saying so was posted on #57 and an owner question raised on it.

  **How it was caught.** Candidate C1 — the print recoloured to each sauce's
  own ink, a change visible across the room — measured *identical* to shipped.
  That cannot happen if the mask covers the paint. It is the same class of
  failure as a screenshot harness aiming at the wrong tile (#4): the
  instrument drops the subject and then reports a confident number.

  **The right mask.** The case's whole silhouette **with interior holes
  filled** — largest warm connected component, then flood the complement from
  the window border (4-connectivity) and keep everything the flood cannot
  reach. It is correct for this subject because the kraft outline encloses the
  print on every face the camera sees, so "inside the cardboard" is exactly
  "on the carton". The mask grows 465 → 628 px on the gallery frame, 201 → 260
  px on the mixed-line frame.

  **What it then measured.** On a real mixed line — a splitter merging the BBQ
  branch's packer output onto the ketchup exit belt, cases alternating, whole
  factory framing, case ≈ 20×21 px — a `boxed_ketchup` and a `boxed_bbq` on
  the **same belt in the same frame** read **156.7** and **117.2**: a
  **39.5/255** gap. The windows are 26×26 at (964,561) and (1023,538) of
  `bbq_mixed.png`, aligned by silhouette overlap. The frame is
  `TomWhitticase-iss-balance/20260920-152320-bbq_mixed.png` on the `pr-media`
  branch, which is the only copy — per #82 the screenshot harness was not
  reproducible run to run when this was measured, so that exact frame cannot
  be re-rendered. (#82 is fixed as of 2026-09-25, but a pinned warm-up stops
  the sim on a different tick than a wall-clock one did, so a re-render is a
  new frame, not this one.)

  **The gap is implementation-sensitive; read it as a range, not a figure.**
  Three independent implementations of the method as written above give
  **30.4, 39.5 and 42.5** — a spread of 12 luma, larger than the 5.02
  same-kind floor quoted below, because the written recipe does not pin the
  mask down to the pixel (the same three give masks of 233 and 260 px). The
  earlier claim here that the 260 px mask "contains all 45 of the window's
  near-neutral `label_cream` pixels" did not reproduce — a re-implementation
  got 23 of 45 — and is withdrawn (#111). What survives is what the ruling
  needs: **every implementation clears the ~26/255 "reads reliably" anchor**,
  the lowest of them by about 15%. The dock's contract-clipboard icons separate by 28.5 luma, 11.6
  dE76. The method's anchors are 1.2/255 = invisible and ~26/255 = reads
  reliably, so the gap clears the reading threshold on every implementation of
  the method tried — by half again on the most generous, by about 15% on the
  least.

  **A control must be the same subject kind as the subject — and every window
  must be logged** (corrected the same day, review of PR #98). The first
  version of this record judged that gap against a "noise floor" of **12.6**,
  taken from six `boxed_ketchup` windows at (663,330), (676,337), (688,315),
  (726,337), (751,380) and (776,366) — cases standing **back to back in the
  packer queue**, touching, each window therefore containing its neighbours'
  cardboard. That is a different pixel population from one case alone on dark
  belt, which is what the claim is about, and the six coordinates appeared
  nowhere in the PR, the issue or this record: unreproducible. **The 12.6
  floor and the 2.4× ratio built on it are withdrawn.** The same-kind control
  is a single `boxed_bbq` on the shared exit belt measured across **14
  independent renders** at the belt positions and animation frames they
  caught: 117.03 to 122.05, a spread of **5.02/255** (windows logged per
  render, x 963-967, y 560-563, 26×26). It bounds render-to-render and
  small-position variation; it does not sample lighting along the whole belt
  run, because every re-render put the case within five pixels of the same
  spot. The claim that carries the ruling is the **pairwise gap**, which two
  people reproduced from different frames.

  **Chroma, tested and negative.** A hue metric was worth trying before
  touching art, so CIELAB dC (`hypot(da*, db*)`, lighting-invariant) ran
  alongside luma on the same mask and crops, with a chroma candidate, on the
  deterministic `--item-gallery` fixture:

  | candidate | dLuma | dL* | dC | dE76 |
  |---|---|---|---|---|
  | shipped (lid print) | **39.2** | 15.2 | 3.6 | 15.6 |
  | sauce ink (chroma) | 6.1 | 5.5 | **13.7** | 14.8 |
  | short faces (the cut on record) | **55.3** | 21.7 | 4.5 | 22.2 |

  On the mixed frame, dC separates the two cartons by 3.7 against a
  same-carton 4.0 — **worse than noise**. Chroma is the weaker channel at this
  size, not the stronger one: a red-on-kraft panel is a handful of pixels at
  20 px while light-against-dark is the whole lid. Trading a 39.2 luma gap for
  a 6.1 one to buy 13.7 chroma barely moves total dE76 (15.6 → 14.8).

  **The rulings.** (1) **The sauce cartons are not repainted** — the shipped
  lid print does its job on a shared belt, in the packer queue, on the dock
  and on the clipboard, so **#40's ruling stands** and the short-faces cut
  recorded in `box_of_sauce`'s doc comment stays on the shelf. It measures
  better (55.3) and is still the next cut if this ever needs headroom; it is
  not needed, and it costs the kraft reading #40 ruled for. (2) **Validate a
  mask by measuring a candidate that visibly changes the art.** If it scores
  the same as shipped, the instrument is broken, not the candidate — and no
  number from that mask may be quoted until it does. The retracted numbers and
  the corrected ones are both on #57 on purpose: the comment that shows a
  wrong number being caught is worth more to whoever reads it in six months
  than a clean one that only shows the answer.

- 2026-09-20: **Mothballing, and `crewed` on a placement** (issues #19 and
  #20). Two halves of one complaint about the staff pools: there was no way
  out of the unpaid spiral, and no way to tell which machine was the one
  going without.

  **The spiral, measured.** Unpaid heads idle the machines they crew, which
  cuts the income that would have paid them, so a factory that missed one
  payday missed the next one harder. Before this there was exactly one lever
  — `Remove` — and it refunds the machine to the shed, losing its position
  and its half-made batch. That is a fine answer for a machine you regret
  buying and a terrible one for a line you want back next period.

  **What was built.** `PlayerCommand::SetMothballed { tile, enabled }`,
  wire `{"type": "set_mothballed", "tile": [x, y], "enabled": true}`. A
  mothballed placement is skipped by `World::recompute_establishment`, so
  `StaffPools::set_required` falls the moment the switch is thrown.
  `Machine::tick` returns early, advancing `start_tick` in step with the
  clock so the batch is HELD rather than lost or completed. It records
  `WorkState::Starved` and reports `StallKind::Mothballed`.

  **Standing crew down: the correction that made this work at all.** The
  first round stopped at the establishment, and review proved it did
  nothing — an A/B over four paydays came back BIT-IDENTICAL, because
  `required` never entered `bill_per_period` or `next_bill`. Shrinking the
  requirement only told the player they *could* fire someone; the ruling
  was that firing is not an acceptable escape hatch, since it destroys a
  trained crew the factory then pays to re-hire. So the pools now stand
  crew down:

  - `stood_down = min(headcount - required, the mothballed requirement)`,
  - `on_payroll() = headcount - stood_down`,
  - `bill_per_period` = Σ `on_payroll()` × wage, so the bill falls on the
    tick the switch is thrown, with nobody fired,
  - `next_bill` = Σ `owed` × wage, and `sync_payroll` only ever RAISES
    `owed` towards `on_payroll()`. Only payday lowers it. Mothballing on
    payday eve therefore buys **notice pay**, not free labour — the same
    principle `Fire` already followed,
  - a stood-down head **never walks out**: `left` is capped by
    `on_payroll()`. This is the whole point. Firing loses the crew;
    mothballing parks it.

  The test the reviewer wished existed is now in `tests/mothball.rs`:
  `four_paydays_with_and_without_mothballing_end_on_different_balances`
  runs their exact experiment on two clones of one world and asserts the
  balances DIVERGE — the control pays four full bills, the mothballed
  factory pays one notice period and then nothing.

  **Why not a fourth `WorkState`.** `LoadBucket` rides the save, so a fourth
  counter is a schema change to record something the existing three already
  record: the window really did contain no work. The stall kind carries the
  *why*, which is where every other "not working, and here is the reason"
  already lives.

  **Why any kind that asks for a crew — reversing "machines only".** The
  first round refused anything without a `Machine` component, on the
  argument that the dock is the only route from product to money so
  switching it off strands the player. Measurement killed that: with the
  dock always on, `required("logistics")` never reaches 0, the clerk stays
  on the payroll, and a factory with no income cannot get its bill to zero
  however much of the line it switches off. The escape hatch did not
  actually reach the bottom.

  So the test is now the CONTENT's: a placement may be mothballed iff its
  kind has a non-empty `staff` list in `data/machines.ron`. A hopper, a
  tank, a belt, a pipe and a splitter ask for nobody, so switching one off
  would release nobody and they are refused byte-exactly (pinned by
  `only_a_crewed_placement_can_be_mothballed`). The dock asks for one
  logistics clerk, so it is switchable. This is the same rule the rest of
  the game follows — content decides what a kind is — and it retired
  `_config_kind_is_machine`, a hardcoded kind list in GDScript, in favour
  of `_config_can_mothball`, which asks whether the polled crew is
  non-empty. The build-ahead case that was going to be a deferred issue is
  simply answered: a spare dock is switched off like anything else.

  **Why no new data number.** Mothballing has no cost and no direct saving;
  what it changes is which heads the establishment asks for, and the figures
  that govern that (`wage`, `pay_period_ticks`) are already in
  `data/staff.ron`. A "mothball fee" would be a real balance lever and a
  real data field — it is not being added on speculation.

  **Where the switch lives.** The click-to-configure panel, which grew a
  third shape: a machine's panel hides the FILL row, the AUTO-FILL switch
  and the picker (hidden, not disabled — a FILL button greyed out forever on
  every crusher reads as a fault to clear) and shows the batch as its level
  bar plus a labelled switch. Not a keybind: Q, SPACE and V were retired for
  acting on a building the player could not see, and a fourth global key
  would be that mistake with a new letter.

  **What it does NOT rescue, stated plainly.** Mothballing gets a factory
  out of ARREARS and keeps its crew: the acceptance test
  (`mothballing_is_what_takes_a_broke_factory_out_of_arrears_and_a_control
  _proves_it`) shows the control losing its clerk at the second payday and
  the other six heads at the third, while the mothballed factory loses
  nobody, clears what it owes, comes back to (4, 2, 1) and SHIPS again —
  and the control cannot even afford to re-hire a logistics head. So
  `Closes #19` stands.

  What mothballing cannot do is conjure income. A `corner_shop` standing
  order is lost to three consecutive missed windows during a shutdown, and
  re-signing costs 40, which a factory that just came out of arrears has
  not got; the reference line takes ~2200 ticks to ship its first box, so a
  player who enters the spiral before their first delivery has no way back
  even with every machine switched off. The acceptance test asserts that
  honestly rather than claiming a recovery it did not see. That residue is
  issue #85 (a contract lapsing during a shutdown), which stays OPEN — it
  is a contract-board question, not a payroll one.

  **`crewed`, and why it is not the last tick.** `PlacementSnapshot.crewed`
  and `MachineLoadEntry.crewed` come from `World::uncrewed_placements`,
  which allocates each pool's `headcount` in the order the tick claims in —
  `topology.machines`, then `topology.docks`, whole crew or nothing — and
  returns whoever cannot be afforded. The obvious alternative, recording
  what the last tick actually refused, was rejected: the tick only claims
  for a machine that would otherwise have worked, so the vat reads crewed on
  every tick the crusher happened to be waiting for a tomato. That flicker
  is precisely what the 2026-09-16 establishment change existed to end. The
  question `crewed` answers is the stable one — *if everything placed wanted
  to run at once, who goes without?*

  It is derived, so nothing was added to the save. It is placed BEFORE
  `links` in `PlacementSnapshot` because `links` is the only optional key
  there and `tests/links.rs` pins the layout JSON byte for byte.

  **How Godot shows it.** The hover's load line, in the same words a `staff`
  stall gives — `_uncrewed_reason_text` and `_stall_reason_text` both go
  through `_crew_categories_text`, so the two readings cannot drift — and
  the load bar. The bar needed one new idea: an uncrewed machine does no
  work, so its window decays to 0% and there is no FILL left to make amber
  at exactly the moment the player needs telling. So the TRACK's border
  carries the colour (`_set_meter_outline`), which is honest — the fill is
  the quantity, the border is the state — and the words directly above it
  still say which state it is (UI_UX §7). A mothballed machine takes
  precedence over both and reads `○ switched off` on an unlit grey bar,
  because the player chose it and nothing about it is a fault. **No in-world
  pole**: the andon poles were removed on 2026-09-12 and the hover panel
  remains the one place status lives.

- 2026-09-16: **The game is called CONDIMENTAL.** "sauce-factory was just a
  temporary name, ive now come up with the name 'CONDIMENTAL'". The owner's
  call, and the only naming decision this project has ever actually made —
  `sauce-factory` was never chosen, it was what the directory happened to be
  called on day one.

  **What it means, and where it stops.** CONDIMENTAL is the *title*: the
  wordmark on the menu (`menu.gd`'s `title.text`), `config/name`, the
  `package/name` of the mobile exports, and the heading of every design doc.
  It is styled all-caps because that is how the owner wrote it and because
  `menu.gd` was already setting the wordmark in caps.

  It is not a theme change. "Sauce" is still what the game is about and still
  the noun in every recipe, ingredient, contract and plan — `boxed_ketchup`
  does not become `boxed_condiment`, and `docs/BBQ_SAUCE_PLAN.md` keeps its
  name. A rename that reached into `data/*.ron` would be a content change
  wearing a rename's clothes.

  **The three places the old name deliberately survives.** A later agent will
  grep for `sauce-factory`, find these, and want to be helpful. It should not
  be.

  1. *The local checkout and the worktree paths.* `orca/projects/sauce-factory`
     and `orca/workspaces/sauce-factory/*` keep the old name. Three live
     worktrees and their running sessions resolve through those paths — a
     worktree's `.git` file is an absolute path into the parent's
     `.git/worktrees/` — so renaming the directory breaks every one of them to
     buy nothing. Git does not care what a checkout is called.

     (The *GitHub* repo was `TomWhitticase/sauce-factory` when this record was
     written, with the rename left as the owner's call. See the addendum
     below: they made it the same day.)
  2. *Dated history.* DEVLOG.md's log and this file's older records say
     "Sauce Factory" because that is what it was called on those dates. A log
     that edits its own past to agree with the present is worth less than one
     that does not.
  3. *The archived `*_PLAN.md` files*, for the same reason — including
     `PIPES_AND_CONVEYORS_PLAN.md`'s worker prompt, which is quoted verbatim
     and should stay a faithful quote.

  **The one real consequence: saves move.** `user://` resolves through
  `config/name`, so the save directory went from
  `app_userdata/Sauce Factory/` to `app_userdata/CONDIMENTAL/`. Existing
  local saves are not migrated and not listed — they sit in the old
  directory. This is accepted rather than solved: the project is pre-alpha
  and already *rejects* saves whose schema has moved rather than migrating
  them, so a save surviving a rename would be a stronger guarantee than a
  save survives an ordinary week. Anyone who wants an old save back can move
  the directory by hand.

  **What did not move.** The crate is still `sim_core`, so every library
  filename in `sim_core.gdextension` is unchanged, and `entry_symbol` is
  still gdext's default `gdext_rust_init` — the `ExtensionLibrary` struct was
  renamed to `CondimentalExtension` for readability only, and no binding
  depends on its name. The Android/iOS ids moved from `com.saucefactory.spike`
  to `com.condimental.spike`, which costs nothing because nothing has ever
  been published under either.

  **Addendum, 2026-09-16, hours later: the repo is renamed too.** "go ahead
  and rename it from sauce-factory to condimental." `gh repo rename` →
  `TomWhitticase/condimental`, and the current-tense links went with it: the
  clone instructions in README.md, the issues link, the two
  `ISSUE_TEMPLATE/config.yml` URLs, `claude.yml`'s description of where the
  agent is, and DEVLOG.md's **Repo:** line.

  Two things made this cheap. GitHub **redirects a renamed repo's old paths
  indefinitely**, so no link was ever dead — not the ones in this file's
  history, not the `pr-media` screenshot URLs embedded in every past PR body,
  which is why those are left alone. And the tooling never hardcoded the name:
  `pr-screenshot.sh` and `check-pr-screenshot.sh` both derive it from
  `gh repo view --json nameWithOwner`, so they followed the rename with no
  edit at all. That is worth preserving — a script that spells out the repo is
  a script that breaks the next time something is renamed.

  `git remote set-url` was handled by `gh` itself, and because the three
  worktrees share one `.git`, that single update reached all of them.

- 2026-09-16: **The title screen's scrim is gone — nothing is washed over the
  hall.** "In the splash screen, get rid of the dark haze/shadowy effect
  that's behind the menu text, I don't like how it looks." The owner's call,
  and it reverses one clause of the centred-menu record below.

  **What it was.** `menu.gd::_build_backdrop` laid a `TextureRect` over the
  whole viewport, filled with a four-stop horizontal `GradientTexture2D`:
  `SCRIM_INK` (0.03, 0.03, 0.04, α 0.82) opaque across the middle 40% of the
  screen, falling to clear at both edges (`SCRIM_CORE = 0.30`). Its stated
  job was "the price of putting text over a photograph" — holding the
  wordmark's contrast wherever the drifting camera happened to be pointing.

  **Why it goes.** The rendered frame is the argument. At 82% black over the
  middle of the screen it is not a subtle wash: it dims the skylight ribbons,
  the clerestory glazing and the far plant line — the exact crispness the
  same 2026-09-16 haze decision below went to the trouble of buying with
  `TITLE_HAZE_DENSITY = 0.0` — and because its edges are a gradient and the
  hall behind it is not, it reads as a soft shadow lying *on* the picture
  rather than as anything in the scene. The screen exists to show the player
  a working factory; a band across its middle is the one thing on it working
  against that.

  **What pays for the contrast instead.** Two things, both of which were
  already most of the way there:

  - **The text carries its own edge.** The wordmark's outline goes 9px → 14px
    at `UI_SCALE` (`TITLE_OUTLINE`) and the tagline and splash line go 5px →
    8px (`STRAP_OUTLINE`). These are deliberately heavier than any outline
    inside a plate, and they are named constants beside `COLUMN_MARGIN`
    rather than `_i(9)` at the call site, because they are now load-bearing
    rather than decoration. An outline follows the glyph, so it costs a few
    pixels around the letters instead of a third of the frame.
  - **The plate was never relying on the scrim.** `PLATE_BG` is α 0.94 with a
    hazard rule along its top; the three buttons are as legible over a lit
    skylight as over a dark plinth either way.

  Checked in the rendered frame at 1600x900 (`--title`), which is the only
  place this is visible: wordmark, tagline and splash all still read, and the
  hall behind them comes up at full brightness.

  **And it is asserted.** `test_main_menu.gd` now refuses any full-rect
  `ColorRect`/`TextureRect`/`Panel` child of the menu, and checks the
  wordmark's outline against `TITLE_OUTLINE`. This is a purely rendered
  symptom — put the scrim back and every other check in that file still
  passes — which is the standing reason (UI_UX §2b point 12) for pinning it
  in a test rather than trusting the next reader of the comment.

- 2026-09-16: **An emissive voxel emits its own colour, and that takes the
  pipeline's one material shader.** Issue #25: nothing in the factory was
  emitting. Every `emissive` voxel — machine status lamps, the cooking vat's
  ring, the splitter's power lens, the payout coin — was being drawn as
  ordinary painted amber, green or red, and `palette.ron`'s
  `emission_energy: 2.6` had no effect whatsoever.

  **The cause.** `make_material` built emission as `emission = Color(1,1,1)`
  with `EMISSION_OP_MULTIPLY`. Godot's MULTIPLY multiplies the emission
  colour by the material's **emission texture** — not by albedo, and not by
  the mesh's vertex colours. This pipeline has no textures anywhere (a `.vox`
  mesh carries colour per vertex), so the operand was absent and the product
  was zero. The comment beside those lines claimed the opposite
  ("multiplying by albedo keeps green reading as green"), which is what the
  author intended and what MULTIPLY would do if the operand were albedo. It
  isn't. Raising the energy to 50 rendered a **byte-identical** frame, which
  is how the bug was proved rather than argued.

  **Why the obvious fix is wrong.** `EMISSION_OP_ADD` lights the lamps
  instantly and lights them all **white**, which is the precise failure the
  original comment set out to avoid: a single `emissive` submesh often holds
  more than one lens colour — the crusher's `control_panel` puts
  `status_green` at one end and `status_amber` at the other — so one flat
  additive colour washes green, amber and red into the same signal, and
  UI_UX §7's "saturated colour reserved for meaning" is spent on nothing.

  **What it is now.** One spatial shader,
  `godot_project/voxel_emissive.gdshader`: `ALBEDO = lens * albedo_scale`,
  `EMISSION = lens * energy`, where `lens` is `COLOR.rgb` converted sRGB →
  linear with Godot's own curve — the conversion `vertex_color_is_srgb` does
  for the other classes. Albedo and emission are fed the SAME linear value,
  so a lamp emits the colour it is painted and never a second hue laid over
  the first. `emissive` is the only class that gets it; matte, metal and
  liquid stay `StandardMaterial3D`, which they need to be — `main.gd`
  duplicates and re-tints those for hopper fills and per-fluid pipe bodies.

  **The energy was retuned in the same pass, 2.6 → 1.2.** 2.6 was never a
  judgement: it was chosen while the value could not reach the screen, and
  any number rendered the same frame. Judged for the first time against the
  mixer's amber lamp, the cooking vat's ring and the dock's deck lamps, under
  the glow settings that were themselves tuned while nothing emitted
  (`factory_environment.gd`: `glow_hdr_threshold 0.85`, `glow_intensity
  0.7`), 2.6 burns a lit `status_amber` #FFB020 lens through to (255, 245,
  178) — a cream white. That is the hue-washing the shader exists to prevent,
  arrived at from the other direction. At 1.2 a lens is unmistakably lit
  against the hazard chevrons beside it and is still amber.

  **The OFF lamp keeps its albedo.** `lamp_idle` drops energy to zero AND
  darkens albedo by `LAMP_OFF_ALBEDO`, unchanged from the fix that made the
  dock's full-pool blink visible (same day, issue #23). Emission working does
  not make albedo redundant: it is what separates the two halves of the blink
  in a frame where the glow pass is doing the talking, and it is the half a
  headless test can assert.

- 2026-09-16: **Staffing is an establishment, not a throughput measure — and
  the crew gets a plate.** A playtest complaint that "the number of staff
  needed keeps fluctuating" turned out to be exactly true, and the cause was
  the design, not a bug.

  **What was happening.** `sim::machine` asks for a crew only when the plan
  is to work ("the crew is asked for only when the plan is to work"), and the
  dock claims its logistics head only when its buffer is non-empty. So
  `demand` measured THROUGHPUT, not staffing. A belt running dry dropped the
  requirement; a burst raised it. The `▲ SHORT-STAFFED` verdict was therefore
  computed from a momentary peak, and a player who acted on it over-hired,
  because the number that prompted them had already moved. GAMEPLAY.md §7
  stated this as the design — "*while actively running*" — so changing it
  needed the owner's agreement, which it got before any code was written.

  **What it is now.** `CategoryPool.required` is the sum of the crews of
  every PLACED machine, running or not. It is recomputed in exactly one
  place, `World::recompute_establishment`, called from
  `World::recompute_topology` — which is already the single choke point for
  `Place`, `Remove` and `Rotate` and nothing else. `short` on the wire became
  `required - headcount`: "hire this many", and it does not move unless the
  player moves it. The old per-tick figures are NOT deleted: `demand` and
  `assigned` remain, and the HUD still shows "3 of 4 working right now"
  beside the establishment. That is a genuinely interesting number; it just
  was never the hiring question.

  **Three things said the codebase already assumed this.** `data/staff.ron`
  ships manufacturing 4, packaging 2, logistics 1 — exactly crusher 1 +
  mixer 1 + vat 2, bottler 1 + packer 1, dock 1, and `tests/staff.rs`
  *asserts* that equality. The HUD was already summing placements in
  GDScript to draw its "asked by: crusher ×1, mixer ×1, cooking vat ×2"
  line — two sources of truth, and gameplay arithmetic on the wrong side of
  the boundary (CLAUDE.md: gameplay logic lives in Rust, never in GDScript).
  And GAMEPLAY §7's own starting numbers only made sense on the placement
  model. Moving the sum into Rust fixed the rule violation and the bug
  together.

  **What was deliberately NOT changed.** Per-tick claim gating stays as it
  was: machines still claim crew each tick in the topology's stable order.
  When `headcount >= required` every claim succeeds, so the two models agree
  whenever the factory is properly staffed, and when it is short the existing
  stable-order rule still lands the shortfall on the same machine every tick.
  The tick path is untouched, which is why this was a safe change to make to
  a system with a long-running determinism requirement.

  **`short` is computed against `headcount`, never `available`.** A head in
  arrears is on the books and not working. Counting them as missing would
  advise hiring a second head to cover the first — but the fix for a head in
  arrears is paying them, and hiring makes the next payday worse. The
  distinction only exists because notice pay exists.

  **No save-schema bump.** `required` is derived, so it is recomputed on
  load rather than stored. That nearly shipped broken:
  `save::load` rebuilds structure by replaying real `Place` commands (which
  recompute it correctly) and THEN calls `restore_staff`, which builds fresh
  pools and threw the derived state away. Every loaded world read as needing
  nobody. Caught by an existing round-trip test, fixed by calling
  `recompute_establishment` after the restore. Worth remembering as a shape:
  a restore that rebuilds a struct wholesale silently discards anything
  derived that was computed before it.

  **The balance consequence, stated plainly.** You are now told to hire for
  machines that are idle, where before an idle machine cost nothing.
  Expansion is more expensive up front and less surprising later. Because
  the starting roster exactly equals the shipped line's establishment, a
  fresh game still starts perfectly staffed. Whether a machine should be
  *mothballable* — switched off deliberately, releasing its crew — was left
  open here and in GAMEPLAY §11. **Decided 2026-09-20 (the record above):**
  yes, machines only, through `PlayerCommand::SetMothballed`.

  **The CREW plate, and what it displaced.** The second half of the same
  request: a permanent readout on the right-hand column showing who you
  have, what they are committed to, whether you must hire, the next payday's
  cost and whether you can cover it. Building it revealed that the clipboard
  was already saying two of those things — a "wages 10 / period · payday in
  18s" line and a two-line SHORT-STAFFED strip — four inches above a plate
  that now says both permanently and in more detail. At 1280x720 that was
  not merely untidy: those two lines were the difference between the column
  fitting and the plates overlapping. So the split is **alert vs readout**.
  The clipboard keeps only the wages WARNING, beside the balance it is
  about; the plate owns the figures; the alarm moved to the plate's heading
  and border, where the rows beneath already name which categories are short
  and by how many. Issue #13's "one hint line at a time" trade lapsed with
  it — the board hint and the alarm are on different plates now.

  **The right-hand column has three tenants, so the order of yielding is
  written down.** The clipboard scrolls its docket stack; the stock list
  scrolls its rows; the CREW plate, whose content is bounded at four staff
  categories, drops its "assigned to" detail line instead of growing a
  scrollbar — an at-a-glance readout you have to scroll is not one, and
  every category stays on screen either way, which a scroll could not
  promise. The plate's cap is measured off the viewport and the hotbar
  ALONE. Reading the clipboard or the stock plate there would close a loop,
  because both of those are sized against the plate's reserved height; that
  is the same circularity `_stock_reserved_height` documents from the
  two-plate era, and it was re-derived the hard way here. Two related
  Godot-specific traps: a wrapping Label reports its minimum height at its
  MINIMUM width, not the width it will be drawn at, so an unconstrained one
  measured three times its real height; and the assignment line is now a
  single ellipsised row rather than a wrapping one, so a big factory's
  machine names cannot silently push the plate past its cap.

  **The plan's per-row meter was dropped, and the plate carries numbers
  only.** A bar would have been legitimate under UI_UX §5e — `required` is a
  sim-defined ceiling, which is exactly what §5e asks for — but it does not
  fit: the column is 342px wide, and `icon 27 + "Manufacturing" ~120 + a
  50px track + "2 / 4" ~45 + "▲ HIRE 2" ~90` plus separations overruns it,
  while a full-width track under each row costs ~24px of height on a plate
  whose whole budget is 43% of the column and which already drops its detail
  line at 1280x720. `2 / 4` beside `▲ HIRE 2` says both things the bar would
  have said, in less room. The H panel keeps its meter, where there is width
  for one.

  **From the holistic review of the feature, also fixed here.**
  `maintenance` is a dead category — nothing in `machines.ron` asks for it
  and the roster starts it at zero — so it now takes no room on the plate
  until something asks or a head is hired into it, and the H panel says why.
  FIRE is gated on HEADS, never on the establishment: a category nothing
  needs is exactly where a player is most likely to be carrying heads they
  want to shed, and disabling the button there would bill them every payday
  with no way out. And notice pay — `fire()` drops `headcount` but leaves
  `owed`, and payday pays the heads who have already left FIRST — is now a
  tooltip on the FIRE button itself, not only a line in the legend at the
  foot of the panel. It is the one staffing rule whose consequence arrives
  after the decision, and FIRE is the button a player reaches for in exactly
  the payday it cannot rescue them from.
- 2026-09-16: **The hall is drawn in first person only; the isometric view
  gets its air. And the main menu is centred.** Two things the owner sent
  back after demoing the title-screen branch: "make the menu centred, at the
  moment the main menu is to the left of the screen", and "when in isometric
  view the skybox is messed up (it looks great in first person btw) so decide
  on what to do with the skybox when we are in the isometric view".

  **The artifact.** In the isometric view the hall's roof came out as a
  handful of hard, near-white stripes fanning from the corners of the frame,
  which slid sideways when the player panned. Rendered at two zooms
  (`--build --closeup 0,0,320` and `--closeup 0,0,160`) to see it properly.
  The cause is structural, not a bug in the shader: `factory_interior_sky.
  gdshader` finds every surface by intersecting a viewer-centred box against
  `EYEDIR`, and an **orthographic camera's rays are parallel**. The entire
  screen therefore samples barely a degree of direction, so two or three roof
  bays get magnified across 1600 pixels; and being a sky, it has no parallax
  to pin it to the floor, so panning slides it. A painted infinite room
  cannot be photographed with a parallel projection. Nothing about the first-
  person view is affected — its rays fan out, which is exactly why the same
  shader reads as a building there.

  **What was rejected.** *Pre-render a panorama of the hall and show that in
  isometric* — it goes stale the moment the voxel art or the lighting is
  re-tuned, which is the defect `factory_environment.gd` exists to prevent.
  *Fake a perspective `EYEDIR` from `SCREEN_UV` when the camera is
  orthographic* — a sky shader does not know the camera, so it would need
  per-frame uniforms pushed from `main.gd`, and it would paint a hall that
  disagrees with the projection of everything in front of it. *Swap in a
  different sky material in isometric* — that changes the sky-derived ambient
  and reflections, so the factory's whole look would shift on TAB, which is
  the one thing the shared environment is there to stop. *A giant slab quad
  behind the world* — real geometry has an edge and occludes the camera as
  the floor grows, which is why the backdrop is a sky in the first place.

  **The rule.** `factory_environment.gd` gained
  `set_backdrop_for(env, first_person)`, the single place that decides:
  `BG_SKY` in first person, `BG_COLOR` at `HAZE_COLOR` (`#939495` — the very
  colour every distant surface in the shader already fades to) in isometric.
  So the hall is drawn where it reads and stands up to being looked at, and
  the isometric view gets the same hall's *air*: the floor runs out into haze
  instead of into a starburst, and pressing TAB reveals the room that haze
  was standing in. `main.gd` holds the `WorldEnvironment` returned by
  `build_into` and calls `_apply_backdrop_mode()` at build and on every TAB.
  The title screen never calls it — its camera is perspective, so it keeps
  the hall.

  **Only the background changes**, and that is the point. The `Sky` resource
  stays on the environment and stays `AMBIENT_SOURCE_SKY` /
  `REFLECTION_SOURCE_SKY` in both modes. Verified rather than asserted: the
  same isometric frame was rendered with the branch forced to `BG_SKY` and
  then with the haze fill, and three sampled patches — floor tiles, the
  machine cluster, a belt's shadowed side — came out **byte-identical**
  ((155,154,144), (141,137,130), (123,122,118)); the only pixels that differ
  are the background itself and a little glow bleed at its edge.

  `test_presentation.gd` now asserts the whole rule, because it is a one-line
  property with a purely rendered symptom — swap the two branches and every
  other test still passes: a `Sky` with the interior shader and sky ambient
  in *both* modes, `BG_COLOR` at exactly `HAZE_COLOR` in isometric, `BG_SKY`
  after `_set_view_mode(FIRST_PERSON)`, the haze back on TAB, and the sky and
  ambient source untouched by the round trip.

  **The centred menu** reverses one clause of the title-screen record above,
  which put the column "to one side of the vanishing point" so the plate
  would not sit on the picture's subject. The owner looked at it and wanted
  it centred, which is decisive — it is a title screen, and a centred plate
  is what both reference games do. So the column and the plate are
  `SIZE_SHRINK_CENTER`, the wordmark is centre-aligned, and the scrim became
  a symmetric four-stop gradient (`SCRIM_CORE`, opaque across the middle 40%,
  falling to clear at both edges) instead of the one-sided ramp. That leaves
  the plate on the composition's middle, so the composition moved to suit it:
  `title_scene.gd`'s camera was re-framed to `Vector3(2.8 * TILE, 4.9,
  8.4 * TILE)` at yaw -6, which puts tomatoes riding into the crusher in the
  left third and bottles coming out of the bottler in the right third —
  "sugar in, sauce out" spread across the frame with the menu in the gap.
  Four framings were rendered and rejected first: pulled back at yaw -30 and
  -33 (one dead half of the screen; the foreground hopper cropped), and a
  distant broadside at yaw -8 (the chain too small to read).

  **And the title screen gets clear air** ("get rid of the haze in the splash
  screen"). `build_into` took a `haze_density` argument and `title_scene.gd`
  passes `TITLE_HAZE_DENSITY = 0.0`. The haze is doing real work in the game
  — the far wall is 132 units off and the player walks about in it, so
  distance has to read as distance — but a title screen is one still frame
  seen under a wordmark, where the same haze reads as a grey wash lying on
  the picture rather than as air. Without it the roof trusses, the skylight
  ribbons, the clerestory glazing and the far plant line all come up crisp
  behind the hand-placed chain, which is what makes the chain look like it is
  standing somewhere specific.

  This is the **one** parameter that differs between the game's environment
  and the menu's, and the header of `factory_environment.gd` now says so —
  the whole point of that file is that the menu cannot end up lit differently
  from the game. It is a property of the photograph, not of the lighting:
  surfaces, both lights, exposure and palette are unchanged, and measured in
  the rendered frame the models moved by at most three levels (the crusher
  (81,84,83) → (80,82,81), the bottler (72,72,73) → (69,69,69)).
  `test_main_menu.gd` asserts the title sky's `haze_density`, because it is a
  single shader parameter that anyone re-tuning the hall would naturally push
  back through both callers.


- 2026-09-16: **The main menu is a title screen: the factory itself, hand
  composed, behind the menu.** The owner asked for "a splash screen similar
  to minecraft/factorio using our assets". Both of those games open on the
  same thing: their own world, photographed, with the wordmark over it and a
  joke underneath. Ours opened on a flat charcoal plate with one hazard band.

  That plate was a decision, not an oversight, and its argument was good:
  `main.tscn` costs a full voxel load, a simulation and a lit 3D frame, and
  the player is on this screen for two seconds. Standing the real game up
  behind a menu to be looked at briefly is a poor trade. **The reversal
  answers that argument rather than overruling it**: `title_scene.gd` loads
  none of `main.tscn` and creates no `SimBridge`. It is about twenty
  hand-placed models — the shipped chain, in the order it runs — a floor, and
  the hall environment. A handful of `.vox` loads, not the game.

  Three things were weighed against this. **A pre-rendered panorama**
  (Minecraft's literal approach — six images) was rejected because the art
  here is generated: every re-render of the voxel models or re-tune of the
  lighting would silently leave the title screen showing an older game, and
  nothing would fail. **Running the real scene behind the menu** was rejected
  on the cost above, and because a simulated line means a title screen whose
  belts back up while nobody is watching. **Doing nothing** was the status
  quo and is what the owner asked to change.

  The rule that keeps this honest is that the poster is built from the game's
  own parts, extracted rather than copied: `voxel_assets.gd` (the `.vox`
  loading, the per-class materials, the mesh cache) and
  `factory_environment.gd` (the sky shader's uniforms, the environment, the
  two lights) came straight out of `main.gd`, which now calls them, and the
  floor helpers `_floor_variant_for` / `_floor_hash` / `_floor_yaw_steps_for`
  became `static` so the menu lays the same worn tiles in the same places. A
  second copy of any of that would be exactly the palette drift UI_UX.md §2a
  point 5 forbids. Re-render the voxels and this screen changes with them.

  **It is art, not a simulation, and that is stated in the file.** Nothing
  ticks; the tomatoes, bottles and boxes on the belts were put there by hand;
  the 2x2 vat is placed by the same footprint-centring rule the game uses but
  by nothing else. The sim's rules do not apply to a poster. The one clock is
  wall time — the camera's slow drift, 74 seconds a cycle — which is the
  single documented exception to "every animation clock is
  `sim.current_tick()`": there is no simulation on this screen to take a tick
  from, and nothing here is reproduced from a save. A perfectly still title
  screen reads as a paused game rather than as a place.

  The camera is **perspective**, where the game is orthographic. UI_UX.md §1
  is a rule about how the game is *played* — it keeps the factory readable
  and the grid honest. A title screen is a photograph of the hall, and the
  converging roof trusses and the line receding into the haze are the whole
  subject; an orthographic poster throws away the depth the backdrop exists
  to show. The framing was found by a camera-projection search rather than by
  eye: three earlier compositions were rendered and rejected — a background
  fluid tank swallowing the frame, the chain reduced to a distant strip of
  empty floor, and the chain entirely hidden behind the menu plate — before
  the broadside at tile (-3.1, 4.2), yaw -35°, pitch -3° put the mixer, vat,
  bottler, packer and dock across the open half of the screen.

  Over it, the menu was rearranged to suit a photograph. The wordmark left
  the plate, grew to 114pt hazard yellow, and took the subtitle and a tilted
  yellow splash line with it; the column stands to one side because a plate
  in the middle covers the vanishing point the whole composition is aimed at;
  a gradient scrim washes the left edge, because an outline alone cannot hold
  white text against both a lit skylight and a dark plinth as the camera
  drifts past. The **splash line is picked from a fixed list by the date**,
  not by `randf()` — it changes day to day like Minecraft's, and a review
  screenshot rendered twice in one session is still the same frame
  (`godot_project/CLAUDE.md`, "visual randomness is derived from an index").

  One regression came out of the rearrangement and is recorded as UI_UX.md
  §2b point 35: the save list's flat 450px cap fit under a title *inside* the
  plate and pushed BACK off the bottom edge under a 114pt one above it. It is
  now measured — the viewport less the wordmark, the plate's padding and the
  load view's own furniture, read off the nodes so it follows the fonts. The
  headless menu test asserts the list never exceeds that budget, which is the
  check that was missing when the constant stopped being right.

- 2026-09-16: **The backdrop is the inside of a factory hall, not the
  sky.** The owner asked for a skybox that makes the game look like it is
  always inside a massive factory. The backdrop had already been argued
  twice: a flat near-black `BG_COLOR` first (chosen to keep a hard horizon
  and a sun streak out of the tightly-framed isometric view), then an
  outdoor `ProceduralSkyMaterial` when first-person mode made the flat
  colour read as an empty void. The second answer fixed the void and left
  the factory standing on a concrete slab in the open air, which is not what
  UI_UX.md §3 describes.

  Three ways to build an interior were considered. **Real geometry** — an
  actual shell around the play area — was rejected first: the hall would be
  finite, so first-person mode could walk to a wall and find the edge of the
  world, and any shell tall enough to read as a factory would occlude the
  isometric camera the moment the floor grew. A **panorama texture** was
  rejected because it is not code-generated content, and because at 4K it
  still resolves into visible pixels under the first-person FOV. What
  shipped is a **sky shader** (`factory_interior_sky.gdshader`): analytic,
  resolution-independent, zero draw calls, and — being a sky — at infinity
  with no parallax, so the hall never resolves into a room whose size the
  player can pace out. The lack of parallax is the feature, not a
  compromise.

  The shape is one box intersected against `EYEDIR`: roof at 26 units, walls
  at 132, slab 5 below. Roof and walls share one structural grid
  (`bay_pitch`), in x and z alike, so a wall column always lands under a roof
  truss — that alignment is most of what makes it read as a building.
  Distance-proportional `soft` widths stand in for the derivatives a sky
  shader cannot take, and the per-bay variation in the distant plant line is
  hashed from the bay index, never `randf()`, so a review screenshot is
  repeatable.

  Two things cost real time and are worth writing down. **A near-parallel
  ray must not return a negative distance.** The first version guarded the
  reciprocal by clamping the magnitude of the direction and kept its sign,
  so a ray running a hair below parallel to the wall pair came back with a
  large *negative* `t`, won the `min()`, and made `exp(-t * density)`
  overflow. The infinity spread through the radiance map, and the symptom
  was that *every lit surface in the world rendered black* — nothing about
  the frame pointed at the sky. `plane_distance()` now returns `1e9` for a
  parallel ray and the haze is clamped. **An interior is a much darker and
  much warmer light source than an open sky.** Ambient energy went 0.8 ->
  1.2 to hold the same fill on the shadowed faces of a voxel model, and the
  key light lost the sun's warmth (0.95/0.88 -> 0.98/0.95) with the fill
  turned back towards daylight-blue: measured against the previous frame,
  the bare floor had gone from r-b = 5 to r-b = 21, i.e. every grey asset in
  the game was photographing beige. Exposure came back from 1.05 to 1.0.

  `test_presentation.gd` asserts the sky is that shader and that ambient
  still comes from it. The isometric camera shows the backdrop in one corner
  of the frame, so without that assertion a silent return to an outdoor sky
  would pass the entire suite.

- 2026-09-16: **Four playtest asks: a dock that can be full, pulp off the
  crusher, SHIFT-to-rebuy while placing, and the first-person camera's
  entry pose.** Four small, unrelated requests from the owner, done as one
  pass. Recorded together because three of them turned out to be the same
  argument at different sizes — *the simulation owns the fact, the
  presentation owns how it reads* — and because the fourth is a one-line
  fix whose whole value is the note about what caused it.

  1. **A dock can be full, and the ceiling went on the POOL, per item id.**
     `sim::economy::DELIVERY_STOCK_CAPACITY` is 48 units of any one item.
     The dock stays the pass-through it was designed as (GAMEPLAY.md §9);
     the bound is on the shared delivery pool it drains into, because that
     is the number the player already watches — a cap on the dock's own
     32-unit buffer would be invisible, since `ship_into` empties it every
     tick and it never holds more than one box.

     **Per item id, not one total,** and this is the load-bearing part: a
     single total can deadlock the game outright. One overproduced product
     would crowd every other item out of the pool, a contract whose mix
     names a crowded-out item could then never be satisfied again, and
     nothing in this game dumps excess stock (GAMEPLAY.md §11's
     spot-market question is still open) — so there would be no way out of
     it. Per item, every id always has its own room, a mix is always
     eventually fillable, and a clog is always a specific sentence ("the
     dock is full of boxed ketchup") rather than a mystery.

     **The number is a relationship, not a literal**, the rule
     `STARTING_MONEY` already follows: a contract settles only when the
     pool holds its whole mix at once, so the cap must comfortably exceed
     the largest single mix quantity on the board — `export_pallet` asks
     for 20, and 48 is 2.4x that, leaving the pallet order to build up
     while the small orders are skimmed off the same pool. `tests/dock.rs`
     pins the *relationship* against `data/contracts.ron`, so adding a
     bigger order fails a test instead of quietly making that contract
     unsatisfiable. Left running against no demand at all, a line reaches
     the ceiling in about 12 000 ticks — long enough not to nag, short
     enough to be a consequence.

     `Dock::try_absorb` now asks the economy for room before it takes a
     box off the belt, so a full pool backs the line up instead of
     swallowing product — the refusal path the dock's own bounded buffer
     had already built and tested, which is why the sim change is small.
     A save whose pool is over the cap is `SaveError::OverfullDeliveryPool`
     rather than a silent truncation: a save that loads to a state the
     live game cannot reach is the bug, not the rejection.

     **Presentation, and the "never colour alone" problem this exposed.**
     The clipboard's ON THE DOCK line was a bare figure under UI_UX.md
     §5e's rule that a bar needs a simulation-defined ceiling; now there is
     one, so it reads `20/48` with a meter, the meter turning the docket's
     one warning red at the cap with `⚠ FULL — belt backing up` beside it.
     On the floor, the obvious signal — light the dock red — is impossible
     here: `_make_material` gives emission `EMISSION_OP_MULTIPLY` over
     vertex colour (§2b point 8's status-lamp rule), so a red wash over a
     blue-grey deck is a dimmer deck, not a red one. The full dock
     **pulses** its deck lamps instead (`DOCK_FULL_PULSE_TICKS`, 12 ticks
     lit, 12 dark), clocked off `sim.current_tick()` like every other
     animation in the scene (§6a), so a pause freezes it and a screenshot
     repeats. A rhythm is also the one signal that survives this camera at
     any zoom, and cannot be mistaken for the steady lamp that already
     means "has stock". `_dock_lamp_lit(stocked, full, tick)` is split out
     pure so `test_presentation.gd` can assert the blink without pumping
     frames.

     **Amended 2026-09-21 — the lamp is per DOCK, the clipboard is per
     pool.** `_update_dock_stock` lit every dock off
     `Economy::delivery_pool_full()`, which is an `.any()` over the
     per-item ceilings above. That was indistinguishable from correct
     while one sauce shipped — the only capped item was the only item —
     and the BBQ branch (PR #142) made it wrong: a dock backing up on
     `boxed_bbq` pulsed the ketchup dock across the floor, which was
     still taking boxes with 38 units of room. A pulsing lamp on a
     working dock is worse than no lamp, because it sends the player to
     inspect the one machine that is fine.

     `Dock` now carries a `blocked` flag, raised at the refusal site in
     `try_absorb` — the one place that has both the refusal and the item
     in hand — and cleared for every dock at the top of `World::tick`'s
     consumer pass. Cleared per tick rather than assigned, because a dock
     may be fed by several belts and the pass visits it once per belt:
     any one of them jamming is the jam, the same `claim`/`reset` shape
     the staff pools use. It is derived, never persisted — `SavedDock` is
     unchanged, so no `schema_version` bump, and a load starts it clear.
     `DockSnapshot::full` carries it; `_blocked_dock_keys()` turns the
     snapshot poll into the set of tiles the lamp tests against.

     `EconomySnapshot::delivery_full` keeps the `.any()` and keeps the
     clipboard's `⚠ FULL` line. The two readings are different questions:
     the clipboard shows the per-item levels right beside it, so "some
     product is capped" is a complete sentence there, where on the
     factory floor it is a lamp pointing at the wrong building.
     `tests/bbq_branch.rs` pins it at world level (one dock full, the
     other clear, with the pool reporting full), and
     `screenshot-scene.sh --split-docks` runs the BBQ fixture on until
     the two docks disagree — a fixture that, under the old code, cannot
     build itself.

  2. **The crusher throws pulp — a BEAT, which is a third size of
     feedback.** §6a settled two sizes for two frequencies (a payout's
     celebration, a dock arrival's small cue). Crushing is neither: it is
     something a machine is *doing* for eighty ticks at a stretch. So it
     is a beat rather than a burst — five flecks of a new `crush_bit`
     voxel asset thrown out of the feed pocket every `CRUSH_BEAT_TICKS`
     (10) of the machine's own WORK ticks, arcing about a fifth of a tile
     over the mouth and shrinking out as they fall back into it.

     **Driving it off `_machine_work_elapsed` is the decision**, not an
     implementation detail: the spray inherits §2b point 28's narrow
     meaning of "working" for free, so a crusher starved of tomatoes or
     blocked on a full output throws nothing, and the pulp is another
     statement of the fact the mechanism and the status lamp already make.
     The beat index is compared with `!=` rather than `>` because a new
     batch resets the machine's elapsed count to zero and its first beat
     still has to fire. Heights and sizes vary from the fleck's own index,
     never `randf()` (§6a), and every distance is in `TILE`.

     **Two things came out of the render and not the plan** (§2b point
     12). The `crush_bit` model took three passes: pale seed pips read as
     a stalk — i.e. as a whole tomato, the opposite of crushed — and a
     cuboid torn corner rendered as a slab hanging off the lump; what
     works is three overlapping spheres and a single proud voxel. And the
     first spray, three flecks at 0.85 scale, read as three stray red
     pixels on the machine's roof rather than as a spit; five at varied
     sizes read as one. Thrown higher (an experiment at 2.4 tiles of
     launch) it stops reading as something coming out of the pocket and
     starts reading as debris flying off the floor. **No sound was added**:
     UI_UX.md §8 keeps "no idle machine loop" deliberately, and a cue
     every ten ticks per crusher is exactly that loop.

     `scripts/screenshot-scene.sh --crushing` is the review flag, built
     for the same reason `--payout` was and holding the same shape: it
     waits for the *state* (a crusher genuinely mid-batch with pulp in the
     air) rather than a tick count, then freezes the scene, because the
     flight is nine ticks and the shot's warm-up is longer. It exists
     because a headless test can prove a fleck was thrown from the mouth
     and reclaimed, and cannot say whether the throw reads as crushing —
     and because without it the obvious fixtures photograph a crusher that
     is blocked or starved at the frame, which is correct behaviour and a
     useless picture. The first `--crushing` run is also what caught a
     missing `cargo xtask` + double `--import`: the flecks were invisible
     because `crush_bit` had never been staged into `godot_project/voxels/`
     (§2b point 4 again).

  3. **SHIFT buys back what the click places.** A modifier on *placing*,
     not a second way to buy: the click still places a unit the player
     owns, and the purchase happens after it and only if the placement
     actually landed, so a blocked tile never charges. It exists because
     laying a run of belt otherwise costs a round trip to the build menu
     per handful. The prompt carries it in all three states, with the
     price on each (`SHIFT: buy a replacement as you place (5 each)` /
     `⟳ SHIFT: buying a replacement each click (-5)` / `⚠ SHIFT: cannot
     afford another (5) · placing from stock`), because a modifier nothing
     ever mentions is one nobody finds, and one that silently drains the
     balance is §7's "never encode state invisibly". The unaffordable case
     warns *before* the click rather than buzzing after it: nothing is
     refused, the placement stands, and the owned count simply starts
     falling.

  4. **Entering first person places the first-person camera.**
     `_set_view_mode` ended with an unconditional
     `_update_camera_transform()` — the isometric placement — after a
     branch that had already placed the camera for whichever mode was
     being entered. Entering first person therefore left the camera at the
     isometric angle and distance until the first mouse-look or WASD input
     called `_update_fp_camera_transform()` and snapped it into place,
     which is exactly the "weird place until I start moving" the owner
     reported. Each branch now places the camera and nothing runs
     afterwards. The rule worth keeping: **when two modes each own a
     transform, the switch must not have a third opinion.**
     `test_camera_nav.gd` asserts the camera *itself* is at `_fp_position`
     on entry, not merely that the mode flag flipped — the old test passed
     throughout.

- 2026-09-16: **Every pull request body embeds a screenshot, no
  exceptions.** The owner's instruction was that PRs always include visual
  elements, and the first question was whether to scope it to visual
  changes. It is not scoped, deliberately. This project is reviewed as a
  frame — the one check that catches a stale content-id lookup is a human
  looking at a PNG (UI_UX.md §2b point 12) — and a conditional rule turns
  into an author deciding their own change is not visual, which is exactly
  the session that also skipped `screenshot-scene.sh`. A change with no
  visual surface still has a state it affects: the panel whose numbers
  moved, the line that backs up, or `--build` as it stands at that commit.
  The screenshot does not replace the words; "How it was verified" still
  carries what the PNG showed, because an image is evidence of a frame and
  not of a reading of it.

  **The mechanism is forced by two facts about this repo.** `gh` cannot
  attach a file to a PR body — GitHub's upload endpoint needs a browser
  session, not a token — and the PNGs `screenshot-scene.sh` writes are
  gitignored, so there is nothing to link to. `scripts/pr-screenshot.sh`
  therefore pushes them through the contents API to **`pr-media`**, an
  orphan branch (no parents, no shared history) that is never merged into
  `main`, so review images never enter the history that feeds the
  changelog and can be pruned without touching source.

  **The URL form is the part that is easy to get wrong, and it was
  checked rather than assumed** (`gh api /markdown` shows what GitHub does
  to each link). This repository is private. A
  `raw.githubusercontent.com` link is a different origin, needs a token,
  and renders as a broken image for every reader; a
  `github.com/<owner>/<repo>/blob/<branch>/<path>?raw=true` link is
  same-origin, is left un-proxied by GitHub's renderer, and is fetched
  with the reader's own session, so it renders for anyone who can see the
  repo. Both CI and the script treat the raw form as an error.

  **It is enforced twice because a rule that only asks is a rule a hurried
  session skips.** A `PreToolUse` hook
  (`.claude/hooks/require-pr-screenshot.sh`) blocks a `gh pr create` or
  `gh pr edit` whose body embeds no image, which is also why `--fill` no
  longer opens a PR here: a commit message cannot hold an image. The hook
  parses rather than substring-matches, and that is not over-engineering —
  the first version blocked an edit to this very documentation because the
  heredoc contained the words `gh pr create`. It strips heredoc bodies
  before deciding whether the command *invokes* `gh`, then searches the
  whole command *including* heredocs for the image, since that is where a
  written-up body lives. The backstop is the `PR screenshot` CI job
  (`scripts/check-pr-screenshot.sh`), which fails a body with no image, a
  `raw.githubusercontent.com` link, or a link to a file that does not
  exist — the last one being what catches an invented URL.

- 2026-09-16: **Five HUD features the owner asked for, built in parallel:
  staffing made unmissable, buy-all in the market, a right-hand STOCK
  plate, a collapsible legend.** The brief was five sentences from the
  owner, two of which ("make it clear I need to hire staff", "more UI for
  the staff stuff because it's confusing") are the same complaint at two
  distances, and it was run as three parallel work packages in three
  worktrees — staff, market, HUD layout — merged in that order. **No Rust
  changed.** Everything the staff features needed was already on the wire
  from the quality pass (`short`, `can_hire`, `next_bill`, `shortfall`),
  and `buy_ingredient` already took an arbitrary quantity, so buy-all is
  one command rather than a loop of ten. That is the reusable half of the
  partition: **check the wire before splitting the work**, because a
  package that needs a sim change cannot run beside the package that
  consumes it.

  The partition rule that made three parallel sessions safe in one
  11,000-line file (`main.gd`) is that every package owned a *region* of
  the HUD — the staff panel, the market grid, the fixed plates — and no
  package was allowed to touch a `.md` file. Documentation is where three
  agents' edits genuinely collide, since every doc change lands in the same
  top-of-file decisions log; it is the orchestrator's job after the merge,
  which is what this entry is. The one real conflict was in the fixed-plate
  variable block, where the HUD package retired the larder strip that the
  market package's neighbours were declared beside.

  1. **Short-staffed is said where the player already is, not only where
     the answer is.** The staff panel had every number right and was still
     answering a question the player had to know to ask. The fix is three
     readouts at three distances rather than a better panel: a
     **SHORT-STAFFED strip on the always-visible clipboard** (`▲
     SHORT-STAFFED — hire 2 manufacturing · press H`), a **one-sentence
     verdict** above the panel's first row, and a **`▲ SHORT` chip plus
     warn border** on the row itself. The strip is a child of the
     clipboard's own body rather than a free plate, so it can never land on
     another plate (§2b point 33 costs nothing if the thing is parented to
     something already placed); it names only the categories actually
     short and hides entirely when none are, because a readout that reads
     `0 short` in the steady state is noise that trains the player to stop
     looking at it. It re-primes on a rebuild, so loading a save that is
     already short does not fire the refusal cue as though it had just
     happened — the same priming `_announce_contract_changes` and the
     payday notices needed, now three callers deep, which is the point at
     which it is a convention rather than a workaround.

  2. **A disabled button says why in words.** `HIRE +1 · 2 / period` and
     `FIRE −1` name the cost of the click on the click, and a HIRE the wire
     refuses reads `✗ cannot hire — 8 in hand, 14 due, 2 / period` rather
     than going grey. Grey says "not now"; only the words say "and here is
     what would change that". The three figures are the wire's own
     (`money`, `next_bill`, `wage`), never a Godot-side sum, for the reason
     the quality pass recorded: after a firing, notice pay makes an
     independently-computed bill disagree with the sim at the exact moment
     the player is watching it.

  3. **The payday rules are stated, inside the scroll.** Four lines of
     plain-words rules (`STAFF_RULES`) at the foot of the *scrolled* list,
     not under the footer, because `_fit_grid_scroll` takes its cap from
     the panel's available height (§2b point 32 again): four lines of
     chrome cost a whole category row, measured at 1600x900 as 2.5 visible
     rows dropping to 1.5. One always-visible footer line says the legend
     is down there. Also dropped: the old footer's `hiring is free`, which
     stopped being true when the quality pass made a hire require the next
     bill covered — a sentence that was right when written is not
     self-maintaining.

  4. **SHIFT+click buys every unit the balance covers, as ONE command.**
     Not a loop of ten single-crate purchases: `IngredientCatalog::cost_of`
     is all-or-nothing, so one command for the whole quantity means no
     partially-completed purchase exists to be reasoned about, and the unit
     price comes from the market snapshot rather than a Godot-side idea of
     one. The arithmetic is two pure helpers (`_units_affordable`,
     `_affordable_units`) so it can be asserted headlessly, and it settles
     the division by multiplying back within `MARKET_PRICE_EPSILON`, since
     money crosses the wire as a double and a fixed-point price that
     divides exactly in the sim does not always divide exactly here.
     **A gesture nobody can see is a gesture nobody uses**, so every slot
     prints what shift would fetch right now (`shift: buy 123`) under the
     price, dimmed to `shift: buy 0` when nothing is affordable. The slot
     is now recessed only when *neither* verb can act, where it used to
     disable the moment a whole crate was out of reach — which would have
     swallowed the shift gesture in exactly the case it was built for, a
     player who can afford four tomatoes but not ten. The card pays for its
     new line in height rather than by shrinking its icon, the opposite of
     the build card's choice in ticket 5, and legitimately so: §2b point
     32's icon-shrink is forced by a *capped grid*, and the market's single
     row of three is not capped.

  5. **Stock is a plate on the right edge, and a reserved height is what a
     plate wants, not what it has.** The larder strip lived inside the
     hotbar's container, so it was one line tall by construction and could
     never be more than a chip per ingredient. The replacement is a plate
     under the clipboard with a row per priced raw ingredient — icon,
     display name, right-aligned amount — and nothing on the hotbar shows
     stock any more, since two readouts of one fact drift. No bar (§5e:
     nothing defines "full" for a tomato), so zero is the word `none` on a
     dimmed row.

     The right-hand column now arbitrates in both directions: the
     clipboard's scroll cap subtracts the plate's reserved height, and the
     plate takes the band between the clipboard's actual bottom edge and
     the hotbar's top. Only one half of that was documented when it was
     built, and the other half bit immediately. Rendering the merged HUD at
     1280x720 showed one of three ingredients: the plate reported the
     height it had *settled at*, which the clipboard's cap is computed
     against, so the two caps chased each other down to a stable point of
     one row on a plate whose entire job is showing every ingredient.
     `_stock_reserved_height` now reports `_stock_wanted_height` — chrome
     plus the list's natural height, capped by the same share the placement
     uses, floored at one row. **Two constraints that read each other must
     not both read a settled value**; one of them has to state an
     intention. Here the clipboard is the right one to yield, because a
     docket stack scrolls by design and this readout does not. The plate
     also uses its own cap rather than `_fit_grid_scroll`, whose
     `_ui_px(110)` floor is taller than the whole list's natural height and
     so could never cap it.

  6. **A fold is not a modal, and it leaves a stub.** The legend's plate
     gains a heading row (`CONTROLS` and a button naming its key) and folds
     to exactly that, so what is left still says what it is and how to get
     it back. Two ways in — the button, which never takes focus so the
     number keys keep reaching the game, and **F1**, read in
     `_unhandled_input` beside TAB so it works in both camera modes. It is
     deliberately outside the one-modal-at-a-time rule: it neither closes
     an open panel nor is closed by ESC or X, because it is furniture, not
     a screen. State is session-only — a fold is "not right now", not a
     preference, and persisting it would need a settings file this project
     does not have. Everything that measures the legend reads `_legend_rect`
     (its *current* rect) rather than a constant, so folding genuinely
     returns the space to the fixed-plate avoid list of §2b point 33.

  Proofs: a new `test_hud_plates.gd` (`HUD_PLATES_OK`, registered in
  `verify-core-loop.sh`), new sections in `test_staff_panel.gd` and
  `test_chain_rework.gd`, and `--short-staffed` / `--legend-collapsed`
  screenshot flags for the two readouts that cannot be photographed any
  other way (one is hidden by every panel, the other only exists once the
  panel is gone). Reviewed at 1600x900 and 1280x720.
  **Reported, not fixed:** at 1600x900 the staff panel shows about 2.5 of
  its 4 category rows and the rest are scrolled to, and a busy clipboard
  standing beside the SHORT-STAFFED strip clips its last docket line — both
  are the same capped-grid arithmetic, and both leave the "am I short"
  question answered by the summary line and the strip, which is what the
  brief asked for.

- 2026-09-16: **GitHub is the workflow: every change lands by pull request,
  and every deferral becomes an issue.** The restructure recorded immediately
  below fixed what an agent *reads* on every turn. This one fixes what it does
  with finished work, and with the work it decides not to do.

  **The measurement.** The repo had been under active agent development for
  ten days, across a dozen worktrees and several orchestration Runs. GitHub
  held the ten stock labels, **zero issues and zero pull requests.** Every
  merge to date had been a local fast-forward or a manual conflict resolution
  followed by `git push origin HEAD:main` — including, an hour before this was
  written, the context restructure itself. Meanwhile at least four findings
  had been consciously deferred and written into prose: the `resolve_sink`
  inlet rule, the unwatched belt animation, `_read_world_layout`'s share of a
  rebuild, and the `--hover` harness drift.

  **The two failures, which have the same cause.** A change that lands by
  local merge has no reviewable artifact: no diff anyone can comment on, no
  CI run standing behind the merge (the commitlint workflow runs on push and
  PR, so a fast-forward of an already-pushed branch proves nothing new), and
  nothing afterwards to read but a subject line. And a finding that lives only
  in a session's closing message is gone the moment the session ends. The
  cause in both cases is that this project is built by agents in separate
  worktrees whose sessions share no memory — *the repository is the only thing
  they have in common*, so anything not written into it did not happen. That
  is the same argument the DEVLOG and the decisions log already won; this
  extends it to the backlog and to the act of merging.

  **The rule for landing work.** Work goes on a `<type>/<description>` branch
  whose prefix is the Conventional Commit type; `gh pr create` opens the PR;
  `gh pr checks` must be green; `gh pr merge --squash --delete-branch` lands
  it. `.githooks/pre-push` refuses a push to `main` and prints those steps.
  It **blocks**, where the `pre-commit` DEVLOG hook only reminds, and the
  difference is the point: whether a change deserves a DEVLOG entry is a
  judgement a script cannot make, whereas "is this ref `refs/heads/main`" is
  mechanical, so it is the rare case where blocking is honest. The escape
  hatch is `--no-verify`, and the hook says so and asks that using it be
  declared.

  **Squash by default** — a branch's "fix the test" and "wording" commits are
  noise in a history CONTRIBUTING.md treats as a changelog source, and the
  squashed subject becomes the single Conventional Commit for the change,
  linted by the same `scripts/lint-commit-msg.sh`. A real merge commit is kept
  only where the individual commits are each a reviewed unit, as in a ticketed
  epic. **Self-merging is allowed for routine work and refused for four
  cases**: red or still-running checks, a PR the owner asked to review,
  anything the design docs call a deviation, and a `gdext` bump or save-format
  change. The agent says it is ready, gives the URL, and stops.

  `.github/pull_request_template.md` asks for what a review here actually
  needs, which is not "what changed" — the diff says that — but **what was run
  and what it said**: the commands by name, the 30 headless markers and the
  `PASS`, and for a visual change what the PNG showed *in words*, because the
  PNG is gitignored and a headless run cannot attach it. It also asks which
  record-keeping calls were made, so the DEVLOG and decision-log questions are
  answered at review time rather than forgotten.

  **The rule for deferrals.** *(Amended 2026-09-21 — the low bar produced an
  unprioritised backlog; see the entry at the top of Part A. Everything below
  still holds for a finding that passes the new gates.)* Deciding something is
  out of scope is only half a decision; the issue is the other half, and the
  bar is deliberately low — if it was worth putting in a report, it is worth
  an issue. The body assumes
  a cold reader: **file and function** (not "somewhere in the Godot layer"),
  **how it was found** (the test name, the `screenshot-scene.sh` flags, the
  fixture — this is what turns a re-investigation into one command), **why it
  was deferred**, and **what the fix probably is** including what it would
  break. Labels are kept few on purpose, because a taxonomy nobody maintains
  is worse than none: `deferred`, `owner-decision` for anything blocked on a
  human call, and five `area:` labels. `owner-decision` earns its place
  because this repo keeps generating that exact shape — a fix that is known,
  small, and not an agent's to make because it changes what a player may
  build or invalidates saves.

  **What issues are NOT.** They are the backlog and nothing else. A rule that
  is now true goes to `docs/DECISIONS_LOG.md` and here; a milestone goes to
  `docs/DEVLOG.md`; a plan is a doc in `docs/`. An issue is only for work not
  yet done. Without that line the four records would start competing, and the
  one that is easiest to write would win.

  **Dogfooded**: this change itself was made on a branch and merged by PR
  rather than pushed, and the four written-down deferrals were filed as issues
  #1–#4, with `docs/PIPES_AND_CONVEYORS_PLAN.md`'s "Still open" section
  amended to cite them so there are not two backlogs. Skills `open-pr` and
  `file-issue` carry the procedures; `docs/WORKFLOW.md` has the rule;
  `CLAUDE.md` carries only the two sentences that apply on every turn.

  **Left undone, deliberately: branch protection on `main`.** A GitHub ruleset
  requiring a PR and a green commitlint check would enforce this server-side
  instead of relying on a hook that `--no-verify` bypasses and that a fresh
  clone does not even have until `git config core.hooksPath .githooks` is run.
  It is not enabled because it would equally stop the **owner's** own direct
  pushes to their own private repo, which is a workflow choice to make
  deliberately rather than a default to impose. The command is one line when
  wanted.


- 2026-09-16: **The agent context was restructured: `CLAUDE.md` is a map,
  not the manual.** Everything below in this file is a record of a decision
  about the *game*. This one is a decision about how the game gets built,
  and it was prompted by an outside argument rather than by a defect:
  "Why Your AI Generates Crap Code - And How to Fix It" (Web Dev Simplified),
  whose central claim is that `CLAUDE.md`/`AGENTS.md` is injected into every
  single request, so its size is not a one-off cost but a tax on every turn,
  and that an `@`-include is exactly equivalent to pasting the file's whole
  text inline.

  Measured against that, this repo was the worst case the video describes.
  `CLAUDE.md` was 631 lines, and it `@`-included ARCHITECTURE.md, UI_UX.md
  and GAMEPLAY.md, so the real always-loaded payload was about **2,000 lines
  / ~34,500 tokens** — roughly four times the largest example the video was
  willing to call broken. Three causes, each individually reasonable:

  1. **The decisions log grew a bullet per past problem.** 57 entries, 429
     lines, all of it loaded on every request including the ones that never
     touch a machine, a contract or a voxel. The log is genuinely valuable —
     it is what stops a rule being re-litigated — but "valuable" and "worth
     paying for on every turn" are different claims.
  2. **Project structure was documented file by file**, 131 lines naming
     nearly every source file and what it contains. Per-file detail is the
     part of a layout that goes stale first: the thing it describes is one
     rename away from being a lie, and a confident lie in the always-loaded
     file is worse than no description at all.
  3. **The three design docs were `@`-included.** They are authoritative and
     they are long, and most sessions need exactly one of them, if any.

  The restructure, following the video's own prescriptions:

  - **`CLAUDE.md` is 72 lines / ~950 tokens** (a 97% cut). It holds the
    project in a sentence, the rules that genuinely always apply — Rust owns
    gameplay logic, content is data, no floats in sim state, follow the
    design docs, small increments, plan mode, text scenes, Conventional
    Commits — a seven-line layout block at *directory* granularity, and a
    table mapping "what you are working on" to "which doc to read". No
    `@`-includes at all.
  - **The condensed decisions log moved to `docs/DECISIONS_LOG.md`**
    verbatim, all 57 entries. It is linked, not included: an agent reads it
    when it needs to know whether a question is already settled.
  - **Per-module `CLAUDE.md` files exist now** for `sim_core/`,
    `godot_project/` and `assets/voxels/` (ARCHITECTURE.md §10 said "once
    warranted" — three years of invariants in one root file is what
    warranted looks like). Each is ~100 lines of that area's non-obvious
    invariants: `sim_core/`'s determinism and topology rules,
    `godot_project/`'s rebuild/wire invariants, `assets/voxels/`'s geometry
    traps. Worst case inside a module is now ~2,400 tokens.
  - **`AGENTS.md` is a symlink to `CLAUDE.md`** at the root and in each of
    those three directories. This is not cosmetic: an untracked 160 KB
    `AGENTS.md` had been sitting at the root, and it turned out to be a
    flattened snapshot of `CLAUDE.md` with its three `@`-includes expanded —
    a copy that had already drifted. A symlink cannot drift.
  - **Workflow material moved to `docs/WORKFLOW.md`** (build, verification,
    the non-headless check, commits, the DEVLOG decision, Orca
    orchestration), and four **skills** were added under `.claude/skills/` —
    `voxel-asset`, `verify`, `screenshot-review`, `record-decision`. A
    skill's body is loaded only when it is invoked; only its name and
    description sit in context, which is the right shape for a procedure
    that matters on the rare turn it is needed and costs nothing otherwise.

  The rule this produces, and the one worth keeping: **only put something in
  an always-loaded file if it would be wrong for the agent not to know it on
  a turn that has nothing to do with it.** Everything else is a link. The
  corollary the video is blunter about than this repo had been: a rule added
  because the agent did something wrong once is not automatically worth its
  place forever — the always-loaded file is not a changelog of past
  mistakes.

  Five documentation defects were found in the process of verifying the
  move, all of them the kind of drift that had been invisible precisely
  because nobody re-reads a 631-line file:

  - **ARCHITECTURE.md §3.9 said the fixed timestep is 60 ticks/s. It has
    never been.** `main.gd`'s `TICK_HZ` has been 30 since milestone 3
    (`e7683c1`), and DECISIONS.md already described §3.9's figure as
    "nominal" against "the rate the scene actually runs". This is a
    correction, not a decision: every seconds-figure derived from the wrong
    rate was twice the real elapsed time, and two were — `hopper.rs`'s
    "roughly 21 seconds" (really 43) and a DEVLOG figure of "40 seconds"
    (really 80). §3.9 now names `TICK_HZ` as the one place the rate is set
    and says what it read until today; a dated DEVLOG entry from 2026-09-11
    was left alone, being accurate when written.
  - **`docs/DEVLOG.md`'s "Current status" and `scripts/verify-core-loop.sh`
    both still said nineteen headless Godot scripts.** There are thirty.
  - **`scripts/screenshot-scene.sh`'s worked example was broken**:
    `screenshot-scene.sh out.png --build …` silently loses `--build`,
    because the script reads `$2` as the resolution and forwards only
    `"${@:3}"`. The example now passes the resolution, and says why it must.
  - **UI_UX.md §2b had three duplicate point numbers** — two extra 27s and
    an extra 28 — in a list whose own header says point numbers are cited by
    code comments and must stay stable. Citations in the wild had resorted
    to disambiguating in prose ("points 27 in \"the floor\" and 28"). The
    three entries took the unused numbers **35** (a model whose only
    consumer is the icon pass is authored for vengi's camera), **36** (the
    floor is authored by its value range) and **37** (N models over
    thousands of tiles is still N stamps; per-instance rotation is the
    multiplier), in place rather than renumbered into sequence, so that
    every existing citation stays correct (34 was taken by ticket 7's
    ghost rule the day before). All seven citations plus
    Part B's own copies were updated.
  - **`main.gd`'s `_panel_position_near` docstring still described the
    behaviour ticket 5 replaced** — viewport-edge flipping only — directly
    above the four-quadrant, plate-avoiding body that superseded it.
- 2026-09-15: **Pipes & conveyors overhaul, ticket 7: the close-out, and
  what a close-out ticket may not decide.** The overhaul's last ticket was
  a bug hunt over the two galleries plus the docs. The galleries were
  clean — tickets 5 and 6 did derive every belt and pipe shape from
  `links`, at every facing and turn — so what the ticket actually found
  sat around the edges of the work, and the decisions worth keeping are
  these five.

  1. **`cargo xtask` runs Godot's importer itself, twice.** Staging voxel
     models deletes their `.import` sidecars, and the `.vox` importer is an
     editor plugin that registers only on the pass after the first scan
     (UI_UX.md §2b point 4), so after every `cargo xtask` the scene came up
     blank until someone ran `godot --headless --path godot_project
     --import` twice by hand. A step that must be run twice, by hand, by
     someone who already knows about it, is not a step — it is a trap, and
     it caught this ticket on its first run (`Nonexistent function
     'chunk_delta_json'`, from a stale staged library beside unimported
     voxels). `main()` now ends with both passes, printing `xtask: godot
     --import pass 1/2 ok`. It **degrades rather than fails** when godot is
     not runnable, because `cargo test` on a CI box with no Godot must
     stay green; `--no-import` opts out and prints the command to run.

  2. **The headless proofs' one known-benign log line was not benign.**
     `Parameter "material" is null` (from
     `material_get_instance_shader_parameters`) had a comment at the top of
     `test_placement.gd` telling the next reader not to chase it. Freeing
     an instance that still holds a `material_override` is what emits it
     under the headless dummy renderer. `main.gd`'s
     `_release_render_resources` walks a subtree and drops mesh and
     material off **both** kinds of visual instance — the
     `MultiMeshInstance3D`s `_build_multimesh_group` makes and the plain
     `MeshInstance3D`s `_asset_node` makes for anything with live
     per-instance state — immediately before it is freed, at every one of the
     six places the scene frees something that could hold a mesh. Clearing
     only the MultiMesh kind left the line at one occurrence, which is how
     the second kind was found; fixing only the two subtree frees left it
     in `test_pipe_shapes.gd`, which is how the ghost body (a lone
     `MeshInstance3D` whose tint is a `material_override`) was found. The
     debugging note is worth as much as the fix: **the line surfaces when
     the rendering server flushes, not synchronously at the `free()`**, so
     instrumenting the frees with marker prints points at whatever ran
     next, not at the culprit. The header comment now says the inverse —
     a `material` line there is a regression, not scenery. A "known benign"
     annotation is a standing invitation to ignore a real signal; it
     earns its keep only while nobody can afford to remove the cause.

  3. **The demo saves are regenerated by a committed script, from the same
     fixtures the tests use.** The four saves demoed since 2026-09-10 came
     from a throwaway script nobody kept, and a save is rejected, not
     migrated, on a `schema_version` mismatch (ARCHITECTURE.md §4) — so the
     next bump would have ended them with no way back.
     `godot_project/make_demo_saves.gd` and `scripts/make-demo-saves.sh`
     rebuild all four ("demo line", "full chain", "belt gallery", "pipe
     gallery") from `test_layouts.gd`'s own builders through real
     `Purchase`/`Place` commands, warm each 600 ticks, and **prove each one
     loads back before writing it** — the rule `game_session.gd` already
     applies before a scene change, for the same reason: a demo save that
     will not load is discovered in front of whoever the demo is for. The
     saves stay in `user://saves/`, not the repo: the committed fixture is
     the builder, not its output. The shell script requires the script's
     `DEMO_SAVES_OK` marker (Godot exits 0 on a parse failure), which is
     what caught the first version calling `Layouts.call(builder, …)` — a
     *static* function cannot be dispatched by name off the script class,
     so the pairing is a `match` now.

  4. **A ghost previews what the connection rules make certain, and never
     what the simulation still has to decide** (UI_UX.md §2b point 34).
     The belt ghost stays straight beside a belt that will corner it while
     the pipe ghost takes its shape from the pipes around it; that reads as
     an inconsistency and is not one. GAMEPLAY.md §9 splits the two models
     — solids respect faces, fluids connect by touching — so a pipe joins
     whatever it touches and its shape is knowable before the click, while
     a belt's hand-off is `resolve_sink`'s answer at placement time. A
     belt ghost that drew the corner would be promising a link the
     placement might not make. The same rule is why the pipe ghost still
     previews no machine or tank fitting: which tile of a footprint gets
     tapped is the sim's call. **No behaviour changed** — the rule was
     written down, and both ghosts already obeyed it.

  5. **A close-out ticket does not get to change the game.** `resolve_sink`
     lets a belt feed a machine through **any** side of an inlet-edge tile,
     and registers a belt sitting on a machine's **outlet** face as feeding
     it. Measured, not inferred: a crusher facing east crafts when fed from
     its north flank, and a belt on an outlet face appears in `links` as
     both `input from West kind Machine` and `output to West kind Machine`
     while carrying real `tomato_paste`. Both `world.rs`'s own module doc
     ("A belt pointing into a machine's flank does nothing") and
     GAMEPLAY.md §9's "a machine accepts input opposite its facing" say
     otherwise, so the **code is the outlier**, and
     `sim_core/tests/links.rs` pins the behaviour deliberately while naming
     this ticket as the owner of the decision. The fix is one condition
     (`belt.direction == sink.direction`). It was not taken, because it is
     a **gameplay change, not a bug fix**: it breaks `test_layouts.gd`'s
     `build_demo_line`, the shipped demo saves, and any layout a player has
     built against the current rule. Tightening a connection rule after
     players have built against it is the owner's call. Two more items are
     reported rather than fixed for the same class of reason: nobody has
     watched the belt animation run live at speed (`BELT_FRAMES` is a
     judgement no headless run can make), and `_read_world_layout` is ~40 ms
     of an 80 ms rebuild at 1500 tiles, which is milestone 8's. Every perf
     figure in the plan doc is off the debug `sim_core` that `cargo xtask`
     builds.

- 2026-09-15: **Quality pass, ticket 5: polish sweep and docs — closes out
  the quality pass.** The brief was a method rather than a defect list:
  render every screen the game has at 1600x900 through
  `scripts/screenshot-scene.sh` and *read* each PNG, on the standing
  argument of UI_UX.md §2b point 12 that the full-scene render catches what
  no headless test can. Five things read wrong. Two of them were not
  discoveries — ticket 3 found them, judged them out of its scope and
  named this ticket in its notes — which is worth recording as evidence
  that the hand-off worked: a bug a ticket declines to fix needs a named
  owner or it becomes permanent.

  1. **A panel's placement is checked against the plates it can land on,
     not only against the viewport's edges — and the check has to survive
     the clamp.** `_panel_position_near` places every hover tooltip and the
     hopper/tank config panel. It flipped the panel to the other side of
     the cursor when it ran past the viewport's right or bottom edge, and
     that is all it did. Three things it therefore ignored: the hotbar,
     which draws over the bottom of the viewport and is not part of its
     visible rect (the config panel's own clamp already subtracted
     `_hotbar_reserved_height()`, so the knowledge existed a few lines
     away); the legend in the top-left; and the contracts clipboard in the
     top-right. A tile low on screen put its tooltip through the hotbar,
     and one high on screen put it over the clipboard's own text. Both were
     found in rendered frames, one bug each.

     The replacement tries all four quadrants around the cursor — nearest
     first, so nothing moves in the common case — and takes the first that
     clears the viewport, the hotbar's reserved height and every avoided
     plate. Plates are read with `get_global_rect()` rather than
     `position`/`size`, since both sit inside their own layout containers
     while this panel is effectively in screen space.

     The fallback is where the real lesson is, and it cost two attempts. A
     tall tooltip (buffer rows, two meters) near the top middle can fail
     all four quadrants at once: down runs past the hotbar, up lands on the
     legend or the clipboard whichever side of the cursor it takes. The
     first fallback written clamped the panel on screen and then pushed it
     off the overlapping plate along whichever axis it penetrated least —
     and pushed it straight down off the bottom of the screen, where the
     same clamp snapped it back onto the plate it was fleeing. The rule
     that came out of it: **a positioning fix must be validated after every
     constraint has been applied, not at the moment it is chosen.**
     `_pushed_clear_of` now tries all four directions off a plate, clamps
     each candidate, and keeps only one that still clears the plate
     afterwards. Penetration depth only orders the attempts; it no longer
     decides.

  2. **The legend's panels line is cut whenever it grows, and it will grow
     again.** Ticket 3 reported the legend's third line running under the
     clipboard at 1600x900 on every screen. The cause is not a layout bug:
     the line reads `E: build menu  M: market  C: contracts  H: staff  P:
     production  click a hopper or tank: configure`, and it grew by one
     entry each time a panel got a key (H with the staff pools, P with
     production stats) with nothing capping its width. Splitting the
     configure hint onto its own line — five legend lines, not four — takes
     the longest line back clear of the clipboard's left edge. This is the
     second time that line has been cut for this reason, so it is recorded
     as a recurring cost rather than a fix: **the next panel that earns a
     key will overrun it again**, and the honest structural answer (wrap
     the legend against the clipboard's measured rect, or drop the
     panel keys into a single "E/M/C/H/P: panels" entry) was left alone as
     out of a no-new-features ticket's scope.

  3. **Only the second and later notices in a poll are queued; the first
     stays synchronous.** Ticket 3's other hand-off:
     `_announce_contract_changes` loops over what changed since the last
     poll and calls `_show_contract_notice` per change, and that function
     assigns `_contract_notice.text` outright. Two contracts lost on the
     same poll, or a loss and a signature together, therefore showed only
     the last — the earlier text overwritten before a single frame drew it.
     Ticket 3's note suggested a queue, and the first implementation was
     the obvious one: every notice appends, `_update_contract_notice`
     drains. It broke `test_contract_board.gd`, and the reason is worth
     keeping. Both of that file's notice assertions read
     `main._contract_notice.text` *synchronously*, immediately after
     `_sign_contract_offer` or after advancing ticks, with no frame pumped
     and no call to `_update_contract_notice` in between; and `.text` is
     never reset between the two cases, so the second one starts with the
     first's text already in the label. Gating on an empty label fails for
     the same reason; gating on wall-clock time since process start is
     unpredictable at test time.

     So the batching moved into `_announce_contract_changes`'s own loop: a
     local `raised` flag, the first change of a poll shown immediately
     exactly as before, the rest appended to `_contract_notice_queue` and
     drained by `_update_contract_notice` once the current notice's hold
     and fade have fully elapsed. `_show_contract_notice` itself is
     unchanged. **The bug was about a second call, so the fix is scoped to
     the second call** — which leaves every existing caller (including
     ticket 4's payday announcements, which share this seam) and every
     existing test behaving identically.

  4. **A card's new line is paid for from the card's own padding, because a
     taller card is not available.** Ticket 4 put a crew's running cost on
     the build-menu card joined to the crew line by ` · `, and at the
     card's real width that overran on every kind with a crew — painted
     over by the next card's stylebox, so it read as a truncated word
     rather than as overflow. The natural fix, growing `SHOP_SLOT_SIZE`,
     was implemented and reverted: `_fit_grid_scroll` caps the build grid's
     scroll against the *panel's* available height, not the grid's natural
     height, so a taller card does not raise the cap — it clips more of row
     two before the scroll starts, and past a point puts a scrollbar on
     screen at 1600x900, which UI_UX.md §5c explicitly rules out. Autowrap
     alone was tried second and broke the phrase in the wrong place
     ("manufacturing · 2 /" then "period"), still overrunning. What shipped
     is a literal newline in `_crew_cost_text` (bounded at two lines) and a
     new `SHOP_CARD_ICON_SIZE` shrinking the build card's icon by roughly
     what the line costs. The market's crate keeps the larger
     `SHOP_ICON_SIZE`, since its grid is one row of three rather than two
     of six and has the slack. This is UI_UX.md §2b point 32 applying to
     itself a second time, and both constants now carry the reasoning in
     comments so the revert is not re-attempted.

  5. **A hover line is never the wire's own representation.**
     `_splitter_output_text` printed a connected output as its tile
     coordinates (`12, 7`). UI_UX.md §5e's rule is about clocks and ids —
     seconds not ticks, display names not wire ids — and a tile coordinate
     is the same category of leak: it is the simulation's addressing scheme
     surfacing in a player-facing line, and it is not actionable, since the
     player is looking at the tile. It says `connected` / `not connected`
     now, which is the whole of the fact the player can act on.

  The ticket's other half was playing a new game from nothing to its first
  payout across the real command boundary, the way `test_layouts.gd` builds,
  looking for any point a player would not know what to do next. It
  produced no fixes, which is a result worth recording rather than an empty
  section: the build prompt, the larder strip, the docket and the payday
  line answer the run's questions in the order it raises them, and the
  earlier tickets' readouts (ticket 4's wage line especially) are what
  closed the gaps that would otherwise have shown up here. One oddity was
  found and deliberately not fixed: `scripts/screenshot-scene.sh --hover`
  can settle the cursor a tile away from the one requested. It is a
  property of the harness's own camera-to-tile math, not reachable in play,
  and fixing it is test-tooling work rather than polish.

- 2026-09-14: **Quality pass, ticket 4: staffing legibility — WHY, COST,
  BEFORE, AFTER.** The brief (`docs/QUALITY_PASS_PLAN.md`) named four gaps
  ticket 1's arrears rule left standing: a player could see a category was
  short without knowing what placed thing was asking for it, could see the
  bill only by opening H, had no warning before a payday it could not
  cover, and had no notice after one that it had actually hurt. Four
  sub-decisions, one per gap, plus the two supporting fixture/testing calls
  that fell out of building them.

  1. **WHY is answered by counting placements, not by asking the sim for
     a reason.** `_staff_demand_sources_for(category_id, placements,
     crews)` walks the world's own placements and crews (the same data
     the hover panel's crew line already reads) and returns one entry per
     *kind* asking for the category, in the order it was first asked,
     with a running count — `asked for by: crusher ×1, mixer ×1, cooking
     vat ×2`. The alternative, a wire field carrying pre-aggregated
     demand sources, was rejected: the sim already reports `demand` as a
     single number per category, and turning that into a breakdown by
     kind is a presentation question (which kinds exist, how their names
     read) with no simulation consequence — the same reasoning that keeps
     `_kind_display_name` and the hover's crew line on the Godot side.
     `STAFF_CATEGORY_BLURBS` (one sentence per category, a plain const
     dictionary) answers the case demand can't: a category nothing has
     been placed against yet still deserves an explanation of what it
     *would* do, so a new player reads `nothing placed asks for staff
     yet` under a sentence naming the job, not two blank lines.
  2. **COST moves to the clipboard, and both readouts quote the wire's
     `next_bill`/`shortfall` rather than recomputing them.** Ingredients
     have the larder strip and equipment has shop prices always on
     screen; wages, GAMEPLAY.md §5's third recurring cost, only existed
     inside the H modal. The always-visible line
     (`wages 14 / period  ·  payday in 6s`) sits under the money counter
     it draws from, and goes to the docket's one warning red with a glyph
     and words when the balance can't cover it — never colour alone
     (UI_UX.md §7). The candidate Godot-side arithmetic, `Σ headcount ×
     wage`, was rejected even though `_staff_bill` already computes it:
     since ticket 1's notice pay, the bill for the *next* period and the
     run-rate can genuinely differ (a head fired mid-period is still
     billed for it), so a second, disagreeing number would appear exactly
     when a player just fired someone and is watching the figure most
     closely. `_staff_bill` survives only as the fallback for a snapshot
     from before the field existed. The build-menu card gained the same
     treatment (`_crew_cost_text`) so a kind's running cost is visible
     before it's bought, not only after, in the same slack UI_UX.md §2b
     point 32 already recorded paying a card's new line from. *(Ticket 5
     amended the format: the cost is on its own line under the crew line,
     not joined to it by ` · ` — see ticket 5's record for why the joined
     form did not fit the card.)*
  3. **BEFORE replays the payday cascade client-side, against wire
     numbers, in wire order.** The wire has `shortfall` (a single
     number) but not "how many heads will actually go unpaid," because
     that requires walking `owed`/`wage` per category in the sim's own
     payment order — exactly the payday rule ticket 1 wrote in `staff.rs`.
     Two ways to get that count: add a wire field computed by the sim, or
     replay the rule in Godot against numbers already on the wire. The
     wire field was rejected for the same reason as point 1 — this is a
     read of the *next tick that hasn't happened*, not a fact about the
     current tick, and the sim already gives Godot everything the rule
     needs (`owed`, `wage`, category order) to derive it without a new
     round trip. `_staff_unpaid_count_if_payday_now` therefore duplicates
     the cascade rule (a zero-wage head is never counted unpaid even
     mid-cascade, matching `staff.rs`'s own treatment) and is asserted
     against both a literal cascade and the real fixture below, so a
     future change to the payday rule in Rust has a Godot-side test that
     would need deliberately updating to match, not one that silently
     drifts. The warning's wording is the ticket's own, verbatim
     (`_staff_shortfall_text`: `▲ next payday short by 6 — 3 heads go
     unpaid`, with a singular `1 head goes unpaid`), because the ticket
     text is itself an example a test can hold to exactly. HIRE reading
     the wire's `can_hire` per row (already computed by ticket 1's hire
     refusal, previously ignored by the row) closes a small legibility
     gap of its own: the button used to say ENABLED until the click was
     silently refused.
  4. **AFTER reuses the contract-loss notice mechanism rather than
     inventing a second one.** `_show_contract_notice` already exists to
     put a fading, one-line announcement along the clipboard's edge, and
     `_announce_contract_changes` already has the right priming idiom
     (the first poll after a rebuild seeds silently, so a loaded save
     doesn't replay history as new events). `_announce_staff_payday`
     copies that idiom exactly, keyed on `next_payday_tick` strictly
     advancing rather than diffing a set of ids, and
     `_staff_payday_notice_text` composes one line per affected category
     (`▲ PAYDAY: 3 packaging unpaid, idle until next payday`, `✗ PAYDAY: 1
     logistics left — unpaid two paydays running`) joined by newlines
     when more than one category is hit in the same payday. A second,
     staff-specific toast widget was rejected: a player who already knows
     what the clipboard's fading notice means for a lost contract does
     not need to learn a second UI convention for a lost head — same
     mechanism, same place, same idiom for reading it. Each row's own
     `status_text` (`_staff_status_text`, both cases sharing the `▲`
     glyph, joined with `  ·  ` when a row is both idled and down a head)
     gives the same fact a permanent home on the row, so the transient
     notice and the persistent row never disagree about what happened.

  **Fixture and testing calls.** `test_layouts.gd` gained
  `build_underfunded_line` (the shipped reference line funded to 8
  against a 14 bill, chosen so manufacturing's 4 heads at wage 2 exactly
  exhaust the balance on the cascade, leaving packaging's 2 and
  logistics' 1 unpaid — shortfall 6, 3 heads unpaid, the ticket's own
  numbers) rather than reusing `build_understaffed_line`: that fixture is
  short on *crew*, this one is short on *money*, and the panel's two
  warnings (short of staff vs. short of cash) needed to be provably
  distinct fixtures, not the same factory read two ways. Loading it
  mid-test disables every category's `can_hire`, which surfaced a real
  test-ordering bug: the pre-existing HIRE/FIRE command tests later in
  `test_staff_panel.gd` assumed the previously-loaded fixture's
  affordability and started failing once the underfunded fixture ran
  before them in the same script. The fix is to rebuild the funded
  reference line explicitly before those tests run, rather than assume
  world state a prior test section happened to leave behind — the same
  discipline `build_understaffed_line`'s section already followed for
  its own `new_game()` call, just not one this new section had inherited
  automatically. `--staff-shortfall` (`screenshot-scene.sh`) renders the
  fixture with the STAFF panel open so the BEFORE warning and disabled
  HIRE buttons are reviewed as pixels, not only as strings a headless
  test read back — both `--staff` and `--staff-shortfall` were rendered
  and read at 1600×900 confirming neither the WHY lines nor the shortfall
  warning clip against the clipboard or the hotbar.

- 2026-09-13: **Quality pass, ticket 1 (sim): staffing — no silent
  walk-out, no free dodge — and the money ceiling.** The brief
  (`docs/QUALITY_PASS_PLAN.md`) carried the diagnosis: the staff pools
  MVP's payday paid heads one at a time in `staff.ron` order and FIRED the
  first head it could not pay and every head after it, with nothing but
  `walked_out_last_payday` in a panel footer to say so. A player who spent
  down on equipment before a payday lost most of their crew and read
  `hired 0`. That is the "kept having to hire staff I never fired" report,
  and it was a design bug. Eight calls, each with the alternative it beat.

  1. **An unpaid head is idled, not fired; it leaves only if unpaid
     twice.** The line rule stays — the first head the money does not
     reach, and every head after it down the file, goes without — but
     "goes without" now means *unpaid*: still hired, not at its post until
     the next payday (`StaffPools::reset` hands out `headcount - unpaid`),
     so the machines it crewed starve with the existing `staff` stall and
     the panel can print `N unpaid`. A head unpaid at the next payday as
     well leaves; a covered payday clears the arrears. This is the contract
     board's strikes (a paid window resets the count; `strikes` misses in
     a row lose the contract) applied to people, with two strikes. The
     alternatives: keep firing but announce it (still sudden — a whole
     line stops with no chance to react); allow debt (GAMEPLAY §1 has one
     non-negative counter, and a negative balance is a second mechanic);
     three or more strikes (a factory that cannot pay for three periods is
     not a factory in trouble, it is a factory with no income, and every
     extra period of idle heads is a period the panel reads as broken).
     Arrears do not stack: a head in arrears is owed ONE wage at the next
     payday, not two — the period it was unpaid for is the period it did
     not work, so the debt is already settled in labour, and stacking
     would make the second payday harder to meet than the first, which is
     the cliff again one period later.
  2. **Back pay is settled first.** Within a category, the money reaches
     the fired heads' notice, then the heads in arrears, then the rest.
     So a factory that can pay some of its crew clears the heads it
     already owes before it lets a fresh one go unpaid, and "leaves if
     unpaid twice" means the money genuinely did not reach that head
     twice. The alternative — counting by position, so whoever is left
     unpaid is assumed to be the same head — lost a head in exactly the
     case where the player had found the money for it. A consequence,
     documented rather than fixed: a factory that can pay half its crew
     every period keeps everyone, half of them idle, indefinitely. That is
     half the labour for half the wage, which is fair, and the panel shows
     it.
  3. **A head is owed for any period it was employed in any part of**
     (`CategoryPool::owed`: set to headcount at each payday, `+1` on hire,
     unchanged on fire). The payday bills `owed`, not `headcount`. This is
     what closes "fire everyone the tick before payday, hire them back the
     tick after": the bill is byte-identical to not having fired, and the
     test says so. Hiring one tick before payday likewise bills that head
     in full — a period is the unit, and a head that worked any of it is
     paid for it. The alternative, pro-rating by ticks employed, is
     honest but is a second clock per head in a system whose whole design
     is four aggregate counters; and it still leaves the dodge half open
     (fire at tick 1, hire at tick 999, pay for two ticks). Notice pay is
     settled before the heads still on the books (call 2), so being broke
     does not dodge it either: the fired head's wage is the first thing
     the money reaches.
  4. **A hire is refused unless the balance covers the next bill plus the
     new head's wage** (`StaffPools::can_hire`; a refused hire is a silent
     no-op like every other refusal, and the wire's per-category
     `can_hire` says so ahead). Without this, calls 1 and 3 opened a worse
     hole than they closed: hire N heads with no money, let them work a
     period, let them go unpaid, fire them (still owed, but you cannot pay
     and they are gone), hire N fresh ones — free labour for the cost of
     clicking. "You cannot hire someone you could not pay" is one rule,
     legible on the button, and it is also the honest one: the panel
     already prints the bill and the balance side by side. What it does
     not prevent is hiring with the money and then spending it, which is
     the reported case — and that path now warns first (call 5) and idles
     rather than fires (call 1).
  5. **The wire warns before and announces after.** `staff_snapshot_json`
     gains `next_bill` (`Σ owed × wage` — what the payday will actually
     take, fired heads included), `shortfall` (how far `money` falls short
     of it, whole units rounded up, 0 when covered), `unpaid_last_payday`
     (hired heads the last payday's money did not reach), and per category
     `owed`, `unpaid`, `left_last_payday` (who left, by category — the
     announcement's text) and `can_hire`. `walked_out_last_payday` keeps
     its name and its meaning — how many LEFT — because every reader
     (`main.gd`'s footer, `test_staff_panel.gd`) means exactly that, and
     only the rule for leaving changed. Godot's `_staff_bill`
     (`Σ headcount × wage`) is now the run rate, not the bill: the
     staff-panel ticket had flagged a sim-side `next_bill` as the cleaner
     home "if the payday rule ever stops being every head at its wage",
     and it has.
  6. **Save: four fields under `#[serde(default)]`, schema 1, and an
     impossible state is refused.** `SavedStaff` gains `unpaid`, `owed`,
     `left_last_payday` (maps by category id, like `headcount`) and
     `unpaid_last_payday`. A save from the MVP loads with nobody in
     arrears and every head owed exactly its headcount — a paid-up crew,
     which is what an MVP save could only have been between paydays. More
     in arrears than hired, or fewer owed than hired, cannot come out of
     the tick path and is `SaveError::BadStaff` with a reason, the
     production ledger's precedent. The 1200-step lockstep test compares
     the staff snapshot every tick with a crew in arrears.
  7. **Money is `Money`, `fixed`'s `I48F16`, its own type beside `Fx`.**
     `I16F16` tops out around 32 767; `data/contracts.ron`'s board pays
     about 23 boxes' worth per 1000 ticks across five lines, which reaches
     the ceiling in minutes of play. A probe settled what happens there:
     the `fixed` crate's operators overflow like the primitive integers',
     so a payout at the top PANICS in a debug build — the profile
     `cargo xtask` ships by default — and wraps to a negative balance in
     release. The task allowed "make saturation visible on the wire" as a
     minimum; that would have left a game whose savings stop at a number
     it never explains. Widening is the fix, and the choice of `I48F16`
     over an integer or `I32F32` is deliberate: the SAME 16 fractional
     bits mean a saved `money_bits` written as `Fx` bits (an `i32`, which
     RON prints as a plain integer) reads back as the same balance through
     the new `MoneyBits(i64)`, `test_layouts.gd`'s `fund` patch
     (`amount * 65536`) is still exactly right, and every whole-number
     price and wage converts losslessly. Not widening `Fx` itself: belt
     positions and pipe volumes are fine at 16 bits and are the hot path.
     `Economy::tick` credits with `saturating_add` regardless of width,
     and a unit test pins a payout at `Money::MAX - 1` landing on
     `Money::MAX`, not a panic and not a wrap. `Shop::cost_of` and
     `Ingredients::cost_of` keep refusing an unrepresentable total, now
     via `checked_from_num` against the 2^47 ceiling (a `u32::MAX`
     purchase of a shipped price is representable and simply
     unaffordable, which the tests now say).
  8. **The audit's verdicts.** Checked and sound: the machine claim order
     is `(y, x, id)`-sorted at every topology rebuild, so removing and
     replacing a machine cannot move it up the queue; a removed machine
     loses its contents and its load window and refunds its price, a
     rearrangement, not a gain; a contract has one signature, a first
     window a full cadence from signing, no refund on loss and a
     re-signing fee, so sign/lose/re-sign loops only cost; auto-fill moves
     bought stock in whole units and never touches money; the dock is a
     bounded pass-through into an unbounded pool whose `Inventory::add`
     saturates. Balance, written into `data/staff.ron`'s header: the
     starting crew equals the reference line's summed crew (pinned); one
     kept-fed line clears about 60 a period on the starter tier before
     wages of 14 — a quarter of the margin, felt and survivable. Listed
     for later tickets, not changed here: a removed hopper or tank
     destroys the stock in it with no warning (ticket 5, a prompt);
     `test_layouts.gd`'s `fund` clamps at 32 767 (test-only, ticket 3's if
     a fixture needs more); the maintenance category is a pure cost until
     something corrodes (the panel already says so).

- 2026-09-13: **Belt item spacing is half a tile, and a belt's front item
  respects the next tile's footprint (quality pass, ticket 2).** The
  project owner's playtest note: items on conveyors are too close
  together.

  1. **The one number, and why a half.** `ITEM_MIN_SPACING_TILES` in
     `world.rs` is the only place the shipped spacing is set (every belt
     and splitter tile is built with it; the unit fixtures in `belt.rs`,
     `hopper.rs` and `dock.rs` carry their own local 0.25 and test the
     mechanism, not the number). It was 0.25 and is 0.5. Items are
     modelled at half a tile (`assets.rs`'s `ITEM_VOXELS` = 16; the widest,
     the box, is 12 voxels across), so at a quarter-tile spacing two
     neighbours overlapped by half their width and the baseline render of
     the bottleneck line's tomato lane was one continuous red bar with a
     stem every few voxels. The candidates were judged by rendering, not
     arithmetic: at a half tile the closeup shows every tomato and every
     sugar sack standing on its own patch of belt with a few voxels of
     rubber between each pair, the jammed sugar lane still reads as a
     queue, and at default zoom both feed lanes read as rows of separate
     dots. Three quarters would leave a jammed four-tile lane holding five
     items, which is a trickle, and 0.4 leaves the box 0.8 of a voxel from
     its neighbour, which does not fix the complaint. The item models and
     `main.gd`'s `_carried_item_transform` did not need to move.
  2. **Speed unchanged, so throughput per belt halves and the line's does
     not.** A tile is still crossed in 50 ticks; a belt now takes one item
     per 25 ticks (forty per thousand) instead of one per 12.5. The
     shipped line's slowest stage, the 240-tick vat, asks its tomato lane
     for about seventeen per thousand, so the reference line's steady
     output measured 41–42 boxes per 10 000 ticks before and after,
     against `data/contracts.ron`'s ~4.1 per thousand. No contract or
     recipe was touched.
  3. **The packing bug the old number hid.** A belt tile's front item
     stores its distance to the tile's END and ran right up to it, while
     the tile ahead could hold an item at its START — the same physical
     point, since items are drawn face to face. A jammed line therefore
     held five items per tile at a quarter spacing (0, ¼, ½, ¾ and 1) and
     drew one of them twice at every boundary. Invisible while the
     spacing was narrower than the items; at a half tile it would have
     been a doubled item every tile. So a belt's front item now respects
     the footprint of what stands on the tile ahead: `BeltSegment::
     tick_toward(speed, room_ahead)` holds it `min_spacing - room_ahead`
     short of the end while that is positive, where `room_ahead` is the
     downstream segment's new `room_at_start()` (the whole length when
     empty, else the back item's distance from the start — the very
     quantity `try_spawn` already measured). The hop is unchanged: the
     receiver still accepts at `min_spacing` of room, and now the sender
     reaches its end at exactly that moment. `tick(speed)` is
     `tick_toward(speed, min_spacing)`, and a belt into a machine or dock,
     or into nothing, uses it. A jammed tile holds exactly `1 / spacing`
     items.
  4. **Order-independent by construction.** `World::tick` reads every
     carrier's downstream room BEFORE any carrier moves, into a reused
     buffer (`World::room_ahead`), through `Topology.belt_ahead` — the
     `belt_to_belt` relation indexed like `belts`, recorded in the same
     loop that resolves it, because a per-belt search of `belt_to_belt`
     every tick would be quadratic in the line's length and the stress
     fixture is 1500 belts. A splitter uses the roomier of its two outputs
     (its item goes to whichever side takes it). Reading pre-tick state
     means a belt's result never depends on whether its receiver was
     visited first, which matters because topology order is placement
     order and a loaded save rebuilds in save order: the lockstep test in
     `tests/save_load.rs` still compares every snapshot every tick. The
     cost is one tick of latency the first time an item crosses a boundary
     in a moving queue, so the item behind settles `speed` (a fiftieth of
     a tile) looser than the spacing; each item pays it once and no
     throughput changes.
  5. **Knock-on: the crusher is belt-limited, and that is reported, not
     tuned.** The crusher's recipe takes four tomatoes per 80 ticks; one
     lane now delivers four per 100. Until the line backs up the crusher
     reads `starved · waiting for Tomato` about a fifth of the time and
     the mixer behind it gets one paste per 100 ticks against its 90-tick
     craft. The vat is the bottleneck so nothing downstream changes, but
     the line takes longer to back up (the crusher reads blocked from
     ~8300 ticks, was ~7000; `tests/machine_load.rs`'s `SETTLE_TICKS` and
     `test_layouts.gd`'s `BOTTLENECK_SETTLE_TICKS` are 10 000 and say
     why), and the staff pools' "short one manufacturing head idles the
     vat" now holds on the ticks the crusher and mixer both claim — on a
     tick the crusher waits for a tomato it claims nobody and the vat's
     two heads are free, which is the same first-come rule read the other
     way. `tests/staff.rs` asserts refused ticks outnumber run ticks and
     the crusher's only reason is ever its input; the paused-craft test
     fires three heads so the vat genuinely cannot run. The splitter
     fixtures count what a jammed run holds now (8 on four tiles, 6 on
     three), and `test_machine_load.gd`'s "a real status change, polled on
     the tick" section watches the vat instead of the crusher, because the
     vat is the one machine a fed line drives to `working` and holds
     there. Whether the crusher's recipe should be re-timed to the lane
     (100 ticks, or five tomatoes) is a balance question, ticket 1's,
     flagged in the report and in DEVLOG.
  6. **What did not change.** Belt speed, `BELT_TICKS_PER_TILE` and the
     four-frame surface animation, item scale, the corner arc, the save
     schema (positions are saved, not gaps), the wire.

- 2026-09-13: **Production stats — the P screen (both halves).**
  Factorio's production GUI. Sim: a `ProductionLedger` on `World`
  (`sim/stats.rs`) counts per item/fluid id what a recipe DEPOSITS at
  craft completion (made) and TAKES at craft start (used), plus what a
  paying contract takes from the delivery pool — the factory's exit, so
  the boxed product is used only when an order ships. Buying stock,
  filling a bin and a hopper metering onto a belt count NOTHING: the
  market is not the factory and moving a unit between bins is neither
  making nor using it. Four rolling windows (`STAT_WINDOWS`: 5s/1m/10m/1h,
  thirty buckets each, one clock advanced once per tick after everything
  that records) plus lifetime totals; integers only, fluids in the
  recipe's whole units. It never dirties a chunk —
  `production_snapshot_json` is polled — and the rings ride the save as
  `SavedProduction` (schema stays 1; an older save loads empty; a corrupt
  ledger is refused with a reason). Godot: **P** opens the PRODUCTION
  panel (`_build_production_panel`), the dark plate like H/C/E/M and one
  modal at a time with them. Godot SORTS and FORMATS and never counts:
  one row per id with anything in the window, biggest first (made + used,
  then id), icon, name, kind, and MADE / USED bars with the count and a
  per-minute rate printed on them — the rate divides by the window's
  OBSERVED span (the wire's `observed`), so a young factory reads its
  true rate; bars are shares of the largest count on the list so rows
  compare (Factorio's convention); a window selector built from the
  wire's `windows` plus "all time" (totals, no rate); an empty window
  says so in a sentence. Polled only while open, on the load's cadence,
  re-read on opening, frozen by the pause menu. `test_production_stats.gd`
  (pure rows and rates from a literal; the bottleneck line counting every
  stage, fluids as fluids, the exit rule; nothing for buying/filling;
  keys; polling; a loaded save's counts on the first frame) and
  `screenshot-scene.sh --production`. Merged into main behind the staff
  pools: `Machine::tick` takes the crew request AND the ledger, recording
  at the `Plan::Start`/`Plan::Deposit` points, and a crew refusal records
  nothing.

- 2026-09-13: **Quality pass, ticket 3 — presentation bugs and
  performance.** The brief (`docs/QUALITY_PASS_PLAN.md`): no new features;
  measure, fix what is slow, fix what is wrong, prove each fix headlessly.
  Everything was measured on `test_perf.gd`'s 1500-belt stress line, in
  the DEBUG `sim_core` that `cargo xtask` builds by default (the release
  flag exists and nothing in the repo uses it — worth knowing before
  comparing these numbers to anything else).

  1. **Where the time went.** A frame profile split `_process` into its
     calls: `_advance_simulation` was 7.2 ms of an 8.4 ms frame, and inside
     it `sim.tick()` was 1 ms and `_apply_dirty_chunks` 12.3 ms per tick.
     The chunk snapshot for a dirty chunk was 50 KB (960 belts) + 28 KB
     (540 belts) — every belt in the chunk, because a belt dirties its
     chunk whenever an item on it moves, and on a running line some belt in
     every chunk moves every tick. Of the 12 ms, 6 was Rust serialising
     (debug), 2.4 was `JSON.parse_string`, 3.5 was `_render_carried_items`
     over 1500 tiles of which ~80 had changed. Everything else `_process`
     does was under 1.5 ms combined, but all of it re-parsed snapshots
     (economy three times) on every frame, ticks or not.

  2. **The fix is a chunk delta, and it is still per-chunk.** ARCHITECTURE
     §4's rule — "delta granularity: per chunk, not per entity" — was
     written against copying one delta per entity across FFI every tick.
     What was actually happening was worse in the other direction: one
     call per chunk carrying every entity in it. `World::chunk_delta`
     keeps the call per chunk and makes what it carries proportional to
     what moved: the sim already calls `mark_dirty(tile)` for exactly the
     conveyor tiles whose contents changed and `mark_placement_dirty` for
     exactly the bodies whose state did, so the same call sites now also
     record `dirty_tiles` and `dirty_entities` (hash sets, membership only;
     the output is sorted by tile so a reading is deterministic and §3.9 is
     untouched). A structural change (`mark_tiles_dirty`, used only by
     place/remove/rotate) and a load (`mark_every_chunk_dirty`) put the
     chunk in `full_chunks`, and the delta of a full chunk IS
     `chunk_snapshot`: a renderer holds no last state for a tile that did
     not exist a moment ago. The same `ChunkSnapshot` struct comes back,
     so `_render_snapshot` did not fork; a tile absent from a delta is a
     tile that did not change, which is exactly the contract
     `_render_carried_items`'s per-tile hiding already had. Pipes are never
     in a delta (fluid is polled; a pipe's shape is on the layout). The
     other options the ticket listed were weighed: skipping off-screen
     chunks helps nothing on a framed factory and the camera can zoom;
     parsing on a cadence makes items visibly stutter; caching the parsed
     dictionary and diffing it in GDScript pays the parse anyway. Walking
     the changed set instead of the chunk (an O(changed) delta) took the
     Rust side from ~1 ms to ~0.3 ms per chunk on top.
     `sim_core/tests/chunk_delta.rs` pins the equivalence tile by tile (a
     belt is in the delta if and only if its snapshot differs from the
     previous tick's, and the entry is that snapshot), the empty delta of a
     jammed line, the full delta after a place, and one hopper in the delta
     when only one unloaded.

  3. **A revision counter gates the per-frame polls.** Every polled
     snapshot is a function of state that only `tick` and `issue_command`
     move, so `World::revision` counts both — a REFUSED command too,
     because a refusal is a fact the hotbar has to re-read (the sim is
     silent on refusal by design). `_wire_changed` compares it, plus the
     viewport size (every panel positions against it), the hotbar
     selection (a key press is not a command) and a bitmask of the open
     modals (a panel opening must read on that frame). At 60 Hz against a
     30 Hz tick that is every other frame. The wall-clock fades those
     functions also drove (the money flyout, the hotbar name flyout) run in
     the gate's `else` branch so they keep fading between ticks and under
     the pause menu, where nothing else moves. `_rebuild_world` resets the
     stamp because a new game's world starts at revision 0. The
     alternative — gating on `current_tick()` alone — was rejected because
     the larder's own comment asks for a purchase to show on the same
     frame, and the headless tests issue commands straight into `main.sim`
     and then call `_process`.

  4. **After: 8.4 → 2.1 ms per frame on the stress line**, 12.3 → 2.2 ms
     per tick for `_apply_dirty_chunks`, 78 KB → 12 KB across the wire per
     tick, items drawn still equal to items in flight (324/324). Draw
     calls and primitives are unchanged (188 / 4.56 M on the stress
     frame; the scene graph was not touched).

  5. **The rebuild re-reads every chunk — a bug the delta made me read
     for.** `_rebuild_world` empties the item pools; the
     `_apply_dirty_chunks` after it reads only the chunks the sim reports
     dirty, which after a place/remove/rotate is the one that changed. On
     a two-chunk factory every item on a belt in the OTHER chunk
     disappeared until it next moved, and a backed-up belt never moves.
     Not new — it predates this pass — but the first proof I wrote for it
     passed without the fix, because the branch belt in the fixture had
     moved on the last tick and was dirty anyway; the proof that bites
     (`test_splitters.gd`) builds a dead-end lane across the chunk seam,
     ticks until a tick dirties nothing, places a belt in the other chunk
     and looks. `_reread_every_chunk` parses every occupied chunk's full
     snapshot once per rebuild (chunks from the layout's `occupies`, on
     the sim's 32-tile grid, packed the way `sim::encode_chunk_id` does):
     +10 ms on the 1500-belt rebuild, paid once per click.

  6. **Input state, four fixes.** (a) A right-drag whose release landed on
     a Control with a mouse filter — the hotbar, the clipboard, a docket —
     or on the pause menu never reached `_unhandled_input`, so
     `_iso_dragging` stayed true and the camera panned with every mouse
     move until the next right click. `_input` (which sees events before
     the GUI) ends a drag on any right release; the press stays in
     `_unhandled_input` so a right click ON a panel starts nothing;
     `_set_pause_open(true)` clears it too. (b) TAB into first-person
     captured the mouse unless the BUILD MENU was open — the one check —
     so with the market, board, staff, production or configuration panel
     up the cursor was captured under a panel of buttons. One decision,
     `_fp_mouse_free`, now feeds `_set_view_mode`, `_set_pause_open` and
     the crosshair. Headless Godot ignores `Input.mouse_mode` (checked),
     so the proof asserts the decision. (c) R with a modal open rotated
     whatever stood behind the panel; `_rotate_tile` refuses under any
     modal. (d) X with the market/board/staff/production panel open entered
     remove mode under it and left both up; it closes every modal, as
     E/M/C/H/P close each other. Proofs in `test_camera_nav.gd` and
     `test_interaction.gd`.

  7. **Left deliberately, reported.** The legend's third line runs under
     the clipboard at 1600x900 on every screen (ticket 5 owns overlaps);
     `_read_world_layout` is ~40 ms of the 80 ms rebuild on 1500 tiles
     (`world_layout_json` is 336 KB; the honest fix is a leaner layout or
     an incremental rebuild, both bigger than this pass); placing a tile
     costs the sim ~5 ms on 1500 tiles (a full topology rebuild, ARCH
     §3.8's milestone-8 flip); `_announce_contract_changes` shows only the
     last of several notices raised in one poll; the staff panel was not
     touched (ticket 4).
- 2026-09-12: **Music and a full set of sound effects.** Requested
  directly ("add some nice music and sound effect"). It reverses one half
  of the 2026-09-11 audio decision — "no music" — and keeps the other
  three: feedback cues only fire on events, everything is synthesised in
  GDScript, and no `.wav` is committed.

  1. **Fourteen cues, one per thing that can happen to the player.**
     `audio.gd`'s `CUES` is the list. The design constraint was a
     *family*: the payout chime is a rising fourth, so the buy cue is a
     rising interval a fifth lower and half as long (a receipt, not an
     earning), the contract-lost cue is the same interval FALLING and held
     longest of all, hire/fire are a rising and a falling fifth, open/close
     are two notes up and the same two down. The place cue is a low thump
     with a bright tick a few milliseconds after it (what separates "a
     machine landed" from "something fell"); remove is its opposite,
     rising, because taking a thing back is not a failure. Refusal is ONE
     shared dull double buzz — the third harmonic on a low E — for every
     "no" the sim gives (unaffordable, blocked, already signed, fire at
     zero), so the player learns one sound for "that did not happen".
  2. **The handler that knows the outcome fires the cue, never the
     input.** Every command is a silent no-op on refusal (the `Purchase`
     convention), so the click cannot know. `_buy_slot` compares the owned
     count before and after; `_buy_ingredient_slot` and
     `_sign_contract_offer` compare `_current_money()` (read from
     `shop_snapshot_json`, the cheapest snapshot that carries the
     balance); `_confirm_build` looks at `_placement_at_tile` after the
     place; `_staff_move_head` compares `_headcount_of` from the staff
     poll it already re-runs. Menus cue inside the `_set_*_open` seams so
     ESC-closing cues too and an already-open menu cues nothing.
     `CONTRACT_LOST` rides the poll diff in `_announce_contract_changes`;
     `SIGN` deliberately does not (the board handler already played it,
     and a poll-side cue would play it twice).
  3. **One looping track, on an autoload.** `music_samples()` in
     `audio.gd`: D minor, 92 BPM, eight bars (i–VI–III–VII, then
     i–VI–iv–V so the loop resolves onto bar one rather than restarting),
     a detuned two-oscillator pad computed in one pass per chord (the
     expensive part), a plucked bass on a dotted pattern, a mallet
     arpeggio on a sixteenth grid with rests (two alternating bars, lifted
     an octave in the second half), kick on 1 and 3, rim on 2 and 4, hats
     on the eighths with one open hat every other bar. 22 050 Hz mono —
     half the cues' rate — because the loop is a thousand times longer
     than a cue and nothing in it wants the top octave; the dulled hat is
     the lo-fi character, not a loss. **Every note wraps at the loop
     end** (`_slot` with `wrap`): a pad released in bar eight is heard
     under bar one, so the seam is a fraction of a step and
     `test_audio.gd` pins it. The `Music` autoload owns the player for the
     same reason `GameSession` is an autoload: `change_scene_to_file`
     frees the menu before the game's `_ready`, and a player parented to
     either scene would stop at the cut. `start()` is idempotent (the
     menu and the game both call it).
  4. **Rendered lazily, on a thread, never headless — and encoded on the
     main thread.** Twenty seconds of PCM take ~0.7 s of GDScript
     (measured), a visible hitch on the menu's first frame, so a `Thread`
     renders the samples. `DisplayServer.get_name() == "headless"` skips
     it entirely: the 29 headless proofs instantiate the game dozens of
     times and can hear nothing. The worker renders only the
     `PackedFloat32Array`; the `AudioStreamWAV` is encoded on the main
     thread (25 ms), so the resource is made where it is used.
     **Quitting goes through `Music.quit_tree()`.** With the track playing
     at the instant the tree is torn down, `--verbose` reported its
     `AudioStreamPlaybackWAV` and `AudioStreamWAV` leaked at refcount 1 —
     nondeterministically, three runs in four of the same build, so it is
     a race between the audio mix thread releasing the playback and the
     engine's exit check, not a reference anyone holds. (It was first
     misread as a worker-thread resource problem; the encode split above
     was made for that reason and kept because it is the better shape.)
     Stopping the player and waiting two frames before `get_tree().quit()`
     made it zero in seven runs; the menu's QUIT, the pause menu's QUIT
     GAME and the window's close button all call the helper, and none
     calls `get_tree().quit()` itself any more.
  5. **Pause ducks, and each channel has its own switch.** The pause menu
     lowers the track by `DUCK_DB` (-10) rather than stopping it — the
     factory has stopped, and the score stepping back says so, where
     silence would read as a crash. MUSIC: ON/OFF sits under SOUND: ON/OFF
     and both are session-scoped, as the SOUND switch always was (a
     persisted setting belongs to a settings screen the project has not
     got). Levels: cues at `VOLUME_DB` -7, the bed at -13, ducked to -23.
     `VOICES` went 4 → 6 so a placement landing on a menu closing does not
     cut either off.
  6. **What did not change.** No idle machine loop, no ambient bed, no
     `.wav` in the repo. The screenshot script is unaffected (a windowed
     run renders the track once; the frame it captures is the same).

- 2026-09-12: **Staff pools MVP, ticket 3 — the Godot half.** The plan's
  "Presentation" section (`docs/STAFF_POOLS_PLAN.md`) built as written,
  with the coordinator's amendment that the andon bullet is void. The
  calls worth recording:

  1. **The panel reads; it does not compute.** Every figure on a row —
     `headcount`, `wage`, `demand`, `assigned`, `short` — is the wire's
     own, and the row's words and bar are a pure function of the category
     dictionary (`_staff_row_state`), so `test_staff_panel.gd` proves the
     panel against `staff_snapshot_json` with no viewport. The one
     exception is deliberate and was put to the coordinator first: the
     footer's `wages N / period` is the UPCOMING bill, which the wire does
     not carry (`last_bill` is what the last payday paid: 0 before the
     first, stale after a hire). It is `Σ headcount × wage`
     (`_staff_bill`) — the payday rule applied to numbers the snapshot
     already holds, the same kind of arithmetic as the shop meter's
     price ÷ money — and explicitly NOT a second opinion on any fact the
     sim reports. Recorded as such: if the payday rule ever stops being
     "every head at its wage" (a recruiting fee, a per-period bonus), a
     sim-side `next_bill` is the cleaner home and this sum is the thing to
     delete.
  2. **The meter is `assigned / demand`, red when short, with the two
     numbers printed on the track.** GAMEPLAY.md §10 asked for a gauge
     per category; §5e says a bar is drawn only where the sim defines the
     ceiling, and `demand` is that ceiling — what the factory asked for
     on the last tick. So a fully staffed idle line reads `0 / 0` on an
     empty bar (nobody asked), not `4 / 4` (a bar of headcount against an
     invented full mark would be the gauge that lies). The numbers sit ON
     the track (a Label child anchored full-rect over the fill) so the
     bar is never colour alone; `2 short` in words stands beside it in the
     warning colour, and the bar turns `METER_FILL_WARN` only while
     `short > 0` — a demand the player cannot meet is the §5e meaning of
     red.
  3. **"nothing to maintain yet" is keyed on the category id.** A row
     whose headcount and demand are both 0 could belong to any category;
     the sentence says what maintenance is FOR, so a generic rule would
     print it for a logistics crew the player had just fired.
     `STAFF_IDLE_NOTES` holds one entry and a fired-out logistics row
     keeps its (empty) `0 / 0` meter.
  4. **Polling is on the load's cadence but on its own stamp.** The plan
     says every `LOAD_POLL_TICKS` from `_advance_simulation` and "buttons
     re-poll at once". Sharing `_load_polled_tick` would make every HIRE
     shift the machine-load cadence; `_staff_polled_tick` is separate, and
     `_poll_staff` also runs once per `_rebuild_world` (a loaded save
     reads its crew before the first frame — the machine-load precedent)
     and once on open. The poll is cheap enough (four categories, one
     JSON) that it runs whether or not the panel is up, which is what lets
     the panel open on the last poll instead of an empty plate.
  5. **The crew is read once per rebuild from `query_tile_json`.**
     `_machine_states` is overwritten by every chunk snapshot, and
     `MachineSnapshot` does not carry `crew` (it is content, not state),
     so hover keeps a separate `_crews` map, seeded in `_seed_crews` for
     every recipe-running machine AND every dock, exactly as
     `_seed_machine_states` is. A category is printed as its id
     (`crew: 2 manufacturing`, `short of manufacturing staff`), a plain
     word, not through `_item_display_name`, which would capitalise it
     into a proper noun mid-sentence. The dock's "held · short of
     logistics staff" names the category from the DOCK'S crew: the wire's
     `understaffed` is a bare bool, and the crew on the same tile snapshot
     is the only place the sim says which category that is.
  6. **The panel had to lose a line per row, and the build-menu card had
     to lose its slack.** The first render of the panel scrolled at
     1600x900 with the fourth category cut off: three text lines per row
     plus stacked buttons. `hired 4  ·  wage 2 / head / period` on one
     line and HIRE beside FIRE brought it to four rows on one plate, the
     §5c lesson again (a scale costs layout). The card: `crew: 2
     manufacturing` at `_ui_font(-3)` CLIPPED at "manufacturi" on the
     128-wide card, and the fix could not be a taller card, because the
     build menu's two-row grid is capped against the hotbar and a taller
     row scrolls it. So the card is 144 wide (the text fits at the
     `test_ui_scale.gd` floor, `_ui_font(-4)`), 138 tall, and its column's
     inset went 8 → 2 and separation 2 → 0 — the crew line was paid for
     out of the card's own padding. `--staff` implies `--populate` for
     the same reason the first render taught: an unrun line reads `0 / 0`
     on every meter, which is true and useless as a review frame.
  7. **No click sound.** The plan says use an existing UI click if
     `audio.gd` has one; it has the payout chime and the delivery knock
     and nothing else, and synthesising a new cue was outside the ticket.
     Noted here so the next audio pass knows the buttons are silent by
     omission, not by decision.
  8. **The andon assertion became a hover assertion.** The stack lights
     were removed on `main` while this epic was in flight; the proof
     asserts the wire's `{kind: staff, item: manufacturing}` stall, the
     load record's `starved`, the hover's `▲ starved · 0% busy · short of
     manufacturing staff` and `crew: 2 manufacturing`, and the load bar's
     amber colour — every channel the removed light had, on the panel that
     replaced it.

- 2026-09-12: **Staff pools MVP, ticket 1 — the simulation half.**
  GAMEPLAY.md §7's staff pools and §5's third cost axis, built as
  `docs/STAFF_POOLS_PLAN.md` specifies (its "The data" and "The wire
  contract" sections are the spec; ticket 3 consumes that wire unchanged).
  Eleven calls, several of them the plan's and restated here because the
  code now depends on them.

  (1) **A plain field, not ECS.** `sim::staff::StaffPools` sits on `World`
  beside `Economy`: four aggregate counters (headcount, available, demand,
  assigned per category) and a payday clock. Nothing is per entity, so
  ECS machinery would be shape without content — the same call `Economy`
  made at milestone 5. This closes ARCHITECTURE.md §11's "`StaffPools`
  has no ECS shape" by saying it needs none.

  (2) **The claim lives inside `Machine::tick`, and is made only on a tick
  the machine would otherwise have worked.** `Machine::tick` first decides
  what the tick WOULD do (start, advance, deposit, blocked, idle) without
  touching a buffer or a tap, then asks for the crew only for the three
  working plans. That is what makes the plan's precedence rule true by
  construction rather than by a check: a machine with no sugar or a full
  output never asks, so a `staff` stall can never mask an `input` or
  `output` one, and `machine_stall` reports it strictly after the six
  older kinds find nothing. The signature grew by one argument,
  `crew: Option<CrewRequest<'_>>` (the kind's `staff` slice plus `&mut
  StaffPools`); `None` is "asks for nobody", which every unit test and
  every crew-less kind uses.

  (3) **A refused mid-craft machine pauses by moving `start_tick` forward
  one tick.** ARCHITECTURE.md §3.5 defines progress as `current_tick -
  start_tick`, and Godot's progress bar reads exactly those two numbers,
  so "elapsed stands still" is expressed in the one representation that
  already exists rather than a second `elapsed` field that could drift
  from it. Because the snapshot's `process` changes on every paused tick,
  the pause is reported as a change (the chunk is dirtied) — the same
  per-tick rate an auto-filling hopper already dirties at, and only while
  understaffed, which is a state the player is expected to fix. Progress
  is never lost; `tests/staff.rs` proves the craft finishes exactly
  `required - elapsed` ticks after the crew returns.

  (4) **Understaffed is a kind of starved.** The window records `Starved`
  and the andon shows amber; the new `StallKind::Staff` (`item` = the
  category id) is what distinguishes it. No fourth `WorkState`: a machine
  waiting on people is waiting on a resource. `Machine::refused_by` holds
  the category for exactly one tick, cleared at the top of the next, so it
  is a per-tick fact like every other stall — and Godot's hover already
  holds the last non-null reason (UI_UX §2b point 30).

  (5) **The first short category in the CREW's order is the one named.**
  A kind's `staff` list order is content (`machines.ron`), so a
  multi-category kind's refusal reason is the content author's choice, not
  the catalog's order or the pools' iteration order.

  (6) **A dock asks only when it holds something, and `understaffed` is
  the verdict of the last such tick.** An empty dock has nothing to ship
  and nobody to ask for, so it neither counts toward demand nor changes
  its flag; the flag therefore reads true while the boxes wait, through
  the empty ticks between arrivals, and flips back on the tick something
  ships. A refused dock keeps its buffer (32 per item), then `try_absorb`
  refuses and the belt backs up — mass conservation as before, pinned by
  a test that counts what waited on the dock and sees exactly that many
  ship the tick the head is back.

  (7) **Payday pays heads one at a time, in `staff.ron` order, through
  `Economy::try_spend`, and the first refusal fires that head and every
  wage-earning head after it.** `try_spend` stays the only debit and stays
  all-or-nothing per call; there is no debt (GAMEPLAY §1's one counter
  does not go negative) and no cliff at money 0 (the heads the balance
  covers stay). The walk-out is a line down the file, not a per-head
  affordability test — a cheaper head later in the file leaves too — so
  the panel can say "N left unpaid" as one fact. A zero-wage head never
  leaves. File order is therefore a design choice: manufacturing first so
  a factory that must lose someone loses its shipping clerk before its
  cook. `next_payday_tick` starts at one period (tick 1000) and the
  boundary test is `tick_count >= next_payday_tick`, `Economy::tick`'s
  own convention.

  (8) **Hire is free and never refused; fire is free and immediate.**
  GAMEPLAY §11's recruiting-fee and firing questions answered "no, for
  now", marked MVP-provisional in GAMEPLAY §9 and §11. The one staff figure
  on the ledger is the wage bill, so the panel shows one number. Neither
  command dirties a chunk (a head is not a tile); both are silent no-ops
  for an unknown category, and fire at zero — the `Purchase` convention.

  (9) **`machines.ron` is schema 2 with a REQUIRED `staff` list, and the
  version is checked off a header before the shape.** Required, not
  defaulted, because an omitted crew must be a loader error, not a silent
  zero (the `recipes.ron` v2 → v3 precedent). Checking the version first
  is new to this loader (save.rs already did it): a v1 file has no `staff`
  and would otherwise be reported as "missing field", which sends the
  reader to the wrong fix. A crew naming an unknown category, listing one
  twice, or asking for zero is a TEST failure
  (`StaffCatalog::validate_machines`, the `validate_covers` pattern), so a
  test fixture may build a registry the catalog does not cover.

  (10) **Save: `SaveData.staff: Option<SavedStaff>`, plus `refused_by` on
  the machine and `understaffed` on the dock, all `#[serde(default)]`,
  schema stays 1.** `None` is the pre-epic save, which loads with the
  starting crew and a payday one period from ITS tick (not from zero, or
  an old save would owe a year of wages on load). The per-tick tallies
  are deliberately not saved — the first tick after a load recomputes
  them exactly, like the topology — so the staff snapshot is compared in
  lockstep from the first tick on, while everything else is compared
  before it. A category the map omits keeps its starting headcount, so a
  category added to `staff.ron` after a save was written starts as a new
  game would; an id the catalog does not know is a corrupt save.

  (11) **A broke factory has no staff, and the test fixtures say so
  honestly.** Every long-running fixture used to stock up and run on
  exactly zero money; at the first payday that factory now loses its whole
  crew and stops, which is the mechanic working. Rather than soften payday,
  `tests/common` gained `WAGE_RESERVE` (300 periods of the starting crew's
  bill, a literal pinned against the data by `tests/staff.rs` the way
  `STOCK_COST` is pinned) and `wages_paid(world)`, read off the payday
  clock; fixtures fund themselves with the reserve and the assertions that
  said "money is exactly the payouts" now say "earnings are exactly the
  payouts", where earnings are the balance plus wages paid minus the
  reserve. The Godot proofs needed no change: a new game's 3000 covers
  wages for far longer than any proof runs, and nothing on the wire they
  read changed shape.
- 2026-09-12: **Staff pools, ticket 2 — the four staff-category icons.**
  The asset half of the staff-pools epic (`docs/STAFF_POOLS_PLAN.md`,
  "The icons"): four voxel models in `assets/voxels/generators/assets.rs`
  that exist only to be rendered by `voxicons` into
  `assets/voxels/icons/staff_<category>.png` for ticket 3's staff panel.
  Six calls.

  (1) **Each icon is the tool the category holds, never a person.**
  GAMEPLAY.md §7 is explicit that staff are an abstract capacity, not
  NPCs; a figure on the panel would promise a person who is not in the
  simulation. Manufacturing is a **hard hat** (the floor-worker's badge);
  maintenance a **double open-ended spanner**; packaging a **taped carton
  with the roll of tape lying on its lid**; logistics a **pallet jack**.
  The four were also chosen to have four *different outlines* — a low
  dome on a disc, a long flat bar with notched ends, a cube, a tall thing
  with two prongs — because the panel shows them in a row of four at 64 px
  and the reader tells them apart by silhouette before colour or detail
  (UI_UX.md §2 "silhouette-first"; the plan's own "silhouette test, not a
  detail test"). The carton is deliberately not just a box: a box alone is
  `item_box`, which is the *product*; the roll on the lid is what turns it
  into the people who seal it.

  (2) **Colour is the plan's rule, with one small extension and no new
  palette entry.** Hard hat and tape are `hazard_yellow` — the one signage
  colour, reserved for meaning (UI_UX.md §7), and also simply what a hard
  hat and a roll of packing tape are. Spanner and jack are class `metal`
  (`steel_light` bodies, a `steel` stamp band down the spanner's shaft,
  `steel_dark` for the jack's pump housing) so they catch the light like
  every other tool here (§4). The carton is `cardboard`/`cardboard_dark`
  like `item_box`, with a `label_cream` shipping label. The extension: the
  jack's four wheels and two grips are `rubber_dark` — a jack's wheels are
  polyurethane, not steel, and the dark accents under the forks are what
  make them read as *off the ground*. The test therefore pins "the jack has
  metal and no signage colour", not "the jack is all metal"; the spanner IS
  pinned all metal. Every colour needed already had a job in
  `palette.ron`, so nothing was appended and no other `.vox` re-emitted —
  the diff is exactly nine new model files, four icons and `index.json`.

  (3) **Authored for vengi's camera, not the game's.** UI_UX.md §2b point
  27's rule from the vinegar jug: a model whose only consumer is the icon
  pass is seen from `vengi-thumbnailer`'s **-X/+Y** corner, never from the
  scene's +X/+Z. So the hard hat's peak points at -X; the carton's tape
  turns down the +Y face and its label sits on the -X face; the jack's
  forks point at -X, where nothing stands in front of them, and its housing
  and tiller rise at the far +X end where a tall part hides nothing
  (point 16's depth rule, applied to the other camera). Nothing here is
  symmetric about a tile midline, on purpose — nothing ever spins these.

  (4) **Registered like the coin.** Entries in `ASSETS` after `coin`, each
  a `VoxModel::tile()`, no footprint, no `machines.ron`/`shop.ron` row, no
  `icons.rs` exclusion (they are exactly what the icon pass is for), no
  Godot change (ticket 3 reads `res://voxels/icons/staff_<id>.png` after
  `cargo xtask` stages `icons/`). The names are `staff_` + the
  `data/staff.ron` category id so ticket 3 can build the path from the
  wire's `id` and never needs a lookup table; a fifth category is a fifth
  generator with the same naming.

  (5) **The rework pass was needed, and by the hat.** The spanner, carton
  and jack read at 64 px on the dark plate on the first render. The hard
  hat had every part a hard hat has — brim, front peak, domed crown, a
  crest over the top — and read as a bell (a jelly mould with ribs): its
  crown stood 14 voxels high on a radius of 11 with a squared-off
  superellipse profile, and the vertical risers of the stacked slices
  became ribbing. Making the crown a plain ellipse 9 high on a radius of 12
  — wider than tall — and dropping the second crest fixed it with fewer
  voxels. The lesson is UI_UX.md §2b point 31: at icon scale an object is
  identified by the *proportion* of its parts, not their presence. The
  other rework was test-found before any render: the first spanner's jaw
  slot was as deep as the head was long, so the lower lip vanished and the
  head was a fork; `spanner_jaws_are_open` failed, the head was widened
  (radius 7.5) and the slot shortened to five voxels.

  (6) **What the tests pin.** `staff_models_are_non_empty_and_fit_one_tile`:
  exactly the four names, in `staff.ron` category order, each non-empty,
  each a 32³ volume, every voxel inside it (`VoxModel::set` clips silently,
  point 29, so an overreaching generator would otherwise lose voxels
  without a word). `staff_icons_use_the_planned_materials`: the classes
  and the yellow, per (2). `spanner_jaws_are_open`: the jaws are empty
  voxels, not a darker colour, for point 11's reason.
- 2026-09-12: **Machine load: the stack lights come down; the hover panel
  is the readout.** After playing with ticket 3's andon stack lights the
  owner did not like how the poles looked and asked for the signals "just
  on the UI elements (the hover overlays)". The in-world half of the epic
  is removed outright rather than hidden behind a toggle: the six
  `andon_*` models, the `andon()` generator and its voxgen tests, the
  `is_andon` icon rule, and every andon node, constant and function in
  `main.gd`. The four palette entries the lights added (`status_*_off`,
  `paint_grey`) are deliberately KEPT: removing a palette entry shifts
  every index after it and re-emits every `.vox` in the repo for no
  visible change. Two things the andon work changed stay because they are
  improvements on their own: `VoxModel::rotated_cw` turning an oblong
  footprint, and the crusher's motor and the packer's control panel at the
  positions the corner test moved them to.

  What replaces the light is a stronger hover: the load line leads with a
  status glyph (`●` working, `▲` starved, `■` blocked) as well as the
  word, so the status is a shape beside a colour (UI_UX.md §7) even in a
  monochrome screenshot, and the load bar under the amber progress bar is
  filled in the STATUS colour rather than a fixed green — green working,
  amber starved, red blocked, the three colours the lenses used — so the
  tooltip is the andon now. The sim half (ticket 1), the polling cadence,
  the remembered reason and the bottleneck fixture are untouched;
  `test_machine_load.gd` asserts records, glyphs and colours instead of
  lenses and pins that NO `andon_*` node exists under `World`, so the
  removal cannot quietly regress.

  Recorded honestly rather than papered over: GAMEPLAY.md §9's stated goal
  was to see the bottleneck "across a whole factory without hovering each
  machine", and a hover-only readout does not do that. It is an open
  thread again (a toggleable HUD badge per machine, a status tint on the
  machine's own lamp, a floor decal — not a pole), to be answered when the
  player asks for it, not before. UI_UX.md §2b point 29's lesson (an
  overlay that does not turn with its host is checked against the host
  turned all four ways) outlives its example.

- 2026-09-11: **Machine load, ticket 3 — the stack lights in the scene.**
  The Godot half of the machine-load epic (`docs/MACHINE_LOAD_PLAN.md`):
  ticket 1's numbers and ticket 2's models become a light beside every
  machine and a line in its hover. Eight calls.

  (1) **Where the light stands is derived from the camera, not picked.**
  `CAMERA_DIRECTION` is where the camera sits relative to its focus, so a
  footprint's visible corner is its far edge on every axis the camera is
  positive on. `ANDON_CORNER` is `(sign(x), sign(z))` of that vector on
  tile axes (`_footprint_centre` maps tile y to world z) — `(1, 1)` for the
  shipped camera — and `_andon_tile` applies it to the ROTATED footprint.
  The constant reads the camera so that the day UI_UX §9's camera question
  is settled the other way, the lights move with it and nobody has to
  remember they exist. The post's own offset into the corner of that tile
  is authored into the model (ticket 2: model +X / low Y, which is world
  +X / +Z); the node stands at the tile's centre at yaw 0 and is never
  rotated, per ticket 2's rule.

  (2) **Three nodes, siblings of the body, visibility-flipped.** Exactly
  the motion-overlay arrangement: `body.get_node_or_null("emissive")` must
  keep finding the machine's own lamp, and a status changes at most once a
  bucket, so building all three once and toggling `visible` is cheaper
  and simpler than swapping meshes. `_set_machine_andon` is a no-op on the
  same status and leaves the light as it was on a status no model exists
  for — a dark tower would say "not measured" about a machine that is.

  (3) **Height by footprint, as ticket 2 said.** `size == Vector2i.ONE`
  gets `_short`, anything else `_tall`. `size` is the rotated footprint,
  which is the right one: a turned 1x2 is still not 1x1.

  (4) **Polled on the sim's clock, from `_advance_simulation`, and once
  per rebuild.** `LOAD_POLL_TICKS` (30) equals the sim's bucket, the
  finest grain the verdict can change at, so polling faster would re-read
  the same answer. The due check is against `sim.current_tick()`, not a
  frame count: a burst of catch-up ticks in one frame still polls on
  schedule, and a paused game (no ticks, and the early return never reaches
  the poll) never polls at all — `test_machine_load.gd` moves the sim on
  underneath a "paused" scene and proves the lights stay put. The rebuild
  poll sits after `_seed_machine_states` and is what makes a loaded save
  light up on frame one: `SavedMachine.load` arrives full, and the
  `_ready` path is a rebuild. `test_screenshot.gd`'s populate loop bypasses
  `_advance_simulation` and so polls once at its end, for the same reason
  it advances the belt and motion frames itself.

  (5) **Godot never re-derives `status`, and hover remembers the last
  reason.** The lens is the wire's `status` string; the counts are never
  compared here. Ticket 1 warned that `stall` is a per-tick fact, `null`
  on most ticks of a blocked machine (it is mid-craft on most of them), so
  `_machine_load[key].reason` holds the last non-null stall while the
  status is unchanged and is forgotten on a status change, because it was
  the old status's reason. The test ticks a settled crusher 300 times and
  asserts its reason is never dropped. This is UI_UX §2b point 30.

  (6) **The hover line, in the plan's language, with one number the plan
  got wrong.** `flat out  ·  100% of the last 20s`, `starved  ·  38% busy
  ·  waiting for Sugar`, `blocked  ·  61% busy  ·  output full`. The
  seconds are `load.window / TICK_HZ`, as the plan required — and that is
  **20 s**, not the "10s" the plan wrote, because the sim's 600-tick window
  is ten seconds at ARCHITECTURE §3.9's nominal 60 Hz and twenty at the
  30 Hz this scene actually runs (§5e: player-facing clocks are at the
  rate the game runs). Six stall kinds have six phrasings; an unknown kind
  says nothing rather than guessing; items go through
  `_item_display_name`. A machine with a status and no remembered reason
  says only the status and the number.

  (7) **The hover panel gets a second bar, and it is green.** The plan
  made the bar optional if it did not crowd the progress bar; the render
  (`--bottleneck --hover`) shows an amber progress bar over a green load
  bar under three lines of text, and it reads. Green (`METER_FILL_LEVEL`)
  because §5e's rule is that a bar's colour is its meaning: the load is a
  level against a real ceiling (100 % of the window), not work under way
  (amber is the batch's progress, directly above it) and not a warning.
  The status word in the text says which way a low level is failing.
  `_hover_panel_state` returns it as `load_meter`, empty for anything but
  a machine, and `_apply_hover_panel_state` records the fraction as node
  meta like every other meter.

  (8) **The fixture is the reference line on auto-fill, settled for 8000
  ticks.** `build_bottleneck_line` funds itself to 8000, buys 400 of every
  raw ingredient plus 300 tomatoes (the crusher runs flat out for the first
  ~7000 ticks filling every buffer with paste, four tomatoes a craft, and a
  first render showed the larder down to 16), switches both hoppers and the
  tank to auto-fill through the real commands, and ticks
  `BOTTLENECK_SETTLE_TICKS` = 8000 — ticket 1's measured number, not the
  plan's 1500. `BOTTLENECK_MACHINES` pins red · red · green · amber ·
  amber by origin tile. The render verdict, stated for the coordinator:
  in the closeup the tall stack on the vat reads unambiguously (green lit,
  two dark lenses below, black bands between); at the whole-line
  auto-framing a lit lens is a 2–3 px dot whose colour is tellable apart
  (red on the crusher and mixer, green on the vat, amber on the bottler
  and packer) but whose position on the post needs one step of scroll
  zoom. Nothing on the Godot side would change that — the posts are
  unoccluded at every machine — so it is reported, not patched.

- 2026-09-11: **Machine load, ticket 2 — the andon stack-light models.**
  The machine-load epic (`docs/MACHINE_LOAD_PLAN.md`) shows a machine's
  load as an industrial andon beside it: a post carrying three lenses,
  green over amber over red, one lit. This ticket built the models in
  `voxgen` only; ticket 1 owns the number, ticket 3 stands the light in the
  scene. Seven calls, each made by a test or a render rather than a guess.

  (1) **Two heights, not one, and the render decided it.** The plan's
  default was a single post whose lens stack clears every machine body
  (the vat's steam stack reaches z 47, so a lens bottom of 48 and a 64-voxel
  model), with two heights as the fallback "if the render shows one tall
  post absurd beside a 1x1 crusher". Both were composed beside a crusher, a
  vat and a packer and thumbnailed. At 64 voxels the post is two and a half
  crushers tall and the stack floats a whole machine's height above the
  body it is bolted to — a lamp-post, not a machine's own light — while the
  same stack at a lens bottom of 32 (clearing the generic housing's
  exhaust at z 31, the tallest 1x1 feature) reads as part of the machine.
  So: `andon_<status>_short` for 1x1 kinds, `andon_<status>_tall` for any
  larger footprint, one generator `andon(p, lit, lens_bottom)` behind both,
  and Godot picks by footprint. The cost is stated honestly: the lights no
  longer sit in one band across the floor, and a short stack's RED lens
  (z 32..35) can be clipped by a vat standing directly camera-side of and
  adjacent to a 1x1 machine; the amber and green cannot, and the tall stack
  is never occluded. `the_andon_lenses_stand_above_every_machine_body` pins
  both bottoms above their bodies AND within one lens height of them, so
  neither can quietly grow into a mast again.

  (2) **The post is inset exactly one voxel, and that number is a test, not
  a taste.** A machine's plinth is flush with its tile edge
  (`machine_footing`), so a post ON the edge puts its outer faces on the
  plinth's and z-fights there (UI_UX §2b point 24). One voxel in, the post's
  column runs down THROUGH the plinth and the painted skirt — which is what
  "bolted to the machine's corner" looks like — and every voxel it shares
  with a body is buried, the post itself covering the one side the skirt
  does not. Two voxels in would put the post's +X face on the painted
  skirt's own +X face (`machine_footing_rect` stops the skirt at
  `xhi - 2`) and the fight comes back. The test
  `the_andon_post_shares_no_exposed_face_with_any_machine_body_at_any_facing`
  overlays the post's column on the +X/low-Y corner tile of every body
  (generic, crusher, mixer, vat, bottler, packer) turned all four ways and
  asserts that wherever a post voxel and a body voxel coincide, every face
  either would expose is covered by one of them. It also asserts the post
  DOES enter the plinth — a post that avoided every body would pass the
  first check by standing on bare floor.

  (3) **The test moved two machine features, by one voxel each.** The
  crusher's drive motor stood on its far (+Y) shoulder at y 25..30, one
  voxel from the edge — turned 180° that is the andon's corner at the
  andon's inset, and the motor's face lay on the post's face. It stops at
  29 now; the post passes through it. The packer's control panel sat at
  x 5..14, which put its green lamp at x 0..2; turned 90° the lamp lands
  one voxel in from +X, on the post. The panel is centred at (11, 20) like
  every other 1x1 kind's. Neither is visible in a preview; both were found
  only by the test. The general lesson is recorded as UI_UX §2b point 29.

  (4) **`VoxModel::rotated_cw` now turns an oblong model.** It asserted a
  square footprint; the mixer is 32x64 and is a body the post stands on.
  The generalisation swaps the X and Y extents on every odd turn — exactly
  what `machines.rs` says a rotated footprint does — and is the identity
  the old code was for a square.

  (5) **Every lens is a ring, the widest ring that fits.** The post is never
  rotated (a lamp on a rotated face is a lamp nobody sees, point 18), so a
  lens facing one way would be a lamp nobody sees from the other three. Each
  lens is `cylinder_z_f` of radius 2 about the post's centre — a rounded
  5x5 — and the radius is the largest possible, not a chosen one: the post
  centre is two voxels from the +X edge and two from the low-Y edge, and
  `VoxModel::set` silently clips anything past the volume, which would
  flatten the ring on exactly the two faces the camera sees. A test pins
  that every lens stands proud of the post on all four sides, is at least 4
  wide and 3 tall (4, as built), stays inside the tile, and has a
  `hazard_black` band between it and the next.

  (6) **Unlit lenses are dark matte versions of their colour, never black.**
  Three new palette entries, `status_green_off`/`status_amber_off`/
  `status_red_off`, each the hue of its `status_*` emissive taken down to a
  dark matte. A tower with three black bands and one lit one is a pole with
  a blob on it; with the dead lenses still faintly their colour the tower
  reads as green-amber-red top to bottom and the lit lens is told apart by
  its PLACE in that order as well as by its glow (UI_UX §7: never colour
  alone). Class `matte`, because an unlit lens that emitted would be a lamp
  half on. `an_andon_lights_exactly_one_lens_and_it_is_the_right_one_in_the_right_place`
  pins one emissive colour per model, in the band the model is named for,
  with the other two bands wholly their `_off` colour.

  (7) **The post is painted grey, and there is no icon.** `paint_grey` is a
  fourth new palette entry: structure is paint, not stainless (§2b point 8);
  not the housings' `paint_blue`, so a post bolted to a blue skirt stays a
  separate object; not black, because a dark column against a body's
  shadow-side face reads as a gap in that face (point 21). Nothing in
  `shop.ron` names an andon, so `icons.rs` skips `andon_*` the way it skips
  motion overlays (`is_andon`). Palette colours are APPENDED, so every
  `.vox` re-emits (only the index bytes change), and the icon pass
  refreshed five icons whose bodies had changed on earlier branches without
  a re-render (crusher, packer, mixer, bottler, floor tile) and wrote four
  that had never been committed (coin and the three floor variants) — all
  committed here, since `icons/` is committed build output.
- 2026-09-11: **Machine load, ticket 1 (sim) — the bottleneck readout the
  simulation owns.** The player asked to see, at a glance, which machines
  are flat out and which are waiting, so the bottleneck in a line is
  legible. `docs/MACHINE_LOAD_PLAN.md` is the plan; this ticket is its
  `sim_core` half. Eight calls worth recording.

  (1) **A window, not a flag, and the sim owns it.** `Machine::tick` clears
  `process` on the tick a batch lands and starts the next on the tick
  after, so an instantaneous "is it working" flickers once per cycle. The
  readout is therefore a `LoadWindow` on every `Machine`: a ring of
  `LOAD_BUCKETS` (20) buckets of `LOAD_BUCKET_TICKS` (30) ticks, a 600-tick
  window, integers only (ARCHITECTURE.md §3.9 — tick counts need no `Fx`).
  It lives in Rust because a rolling window in GDScript would be gameplay
  logic in the presentation layer, would not survive a save and could not
  be tested headlessly (ARCHITECTURE.md §1's hard rule).

  (2) **Exactly one `WorkState` per tick, and the completion tick counts as
  work.** Started, advanced or *deposited* → `Working`; nothing started →
  `Starved`; finished and held on a full output or pipe → `Blocked`. The
  deposit tick is the one that matters: with it counted as work, a machine
  fed and drained without interruption reads exactly 100 %, tick after
  tick, and `FULL_LOAD_PERCENT` (95) is a genuine threshold with a real
  margin for a line that hiccups, rather than a fudge that hides the
  mechanism's own one-tick gap. A unit test asserts no non-working tick is
  ever recorded on such a machine. (A machine whose recipe is missing —
  which `World` refuses to place — records `Starved`, so the "exactly one"
  rule has no exception.)

  (3) **`status` is derived in one place, and Godot reads the word.**
  `LoadWindow::status`: nothing observed → `starved`; working share ≥ 95 %
  → `working`; otherwise `starved` if `starved >= blocked` (a tie reads
  starved), else `blocked`. It is serialized as the string on the wire, and
  the presentation layer is told not to re-derive it from the counts, so
  the threshold has exactly one home. `percent` is `working * 100 /
  observed`, 0 when nothing has been observed.

  (4) **The ring clears a whole bucket the tick after it fills**, so a
  warmed-up window's `observed` oscillates between 571 and 600 rather than
  sitting at 600. That is the cheap ring (no per-tick subtraction of a
  departing tick), and it is what ticket 3 must expect: `observed` is a
  denominator, not a progress bar, and `window` (600) is on the wire beside
  it for the "of the last 10 s" wording.

  (5) **Load never dirties a chunk.** It changes every tick on every
  machine; a per-tick dirty would defeat ARCHITECTURE.md §4's per-chunk
  delta rule. It gets the treatment fluid movement and tank auto-fill get:
  a light polled endpoint, `SimBridge::machine_load_snapshots_json`,
  returning `[{tile, load, stall}]` — one per recipe-running machine, in the
  topology's stable tile order, nothing else — and the identical `load` and
  `stall` objects riding `MachineSnapshot` (chunk snapshots) and
  `query_tile_json`'s machine variant, which did *not* go through
  `machine_snapshot` and now carries the same fields explicitly. A test
  pins that a starving machine's growing window dirties nothing.

  (6) **The stall reason is computed at snapshot time, never per tick.** It
  is a question ("why is this machine waiting right now?"), not state, so
  `World::machine_stall` asks the questions `Machine::tick` asks — enough of
  each solid input? a tap that can supply each fluid? room for each output?
  — read-only, against the same `machine_taps`/`machine_output_taps` the
  tick would hand the machine. First shortfall only, in recipe order, solids
  before fluids: idle → `input`, `fluid_input` (tap exists, cannot supply),
  `no_supply_pipe` (recipe wants a fluid, no tap); complete-and-held →
  `output`, `fluid_output`, `no_outlet_pipe`; mid-craft, or idle with
  everything it needs (it starts next tick) → `null`. It describes *this
  tick*, independently of the window's `status`: a blocked crusher is
  mid-craft on some ticks and reports `null` on them. The kinds serialize
  in `snake_case` as the plan's exact words.

  (7) **Save: the raw ring, `#[serde(default)]`, schema stays 1.**
  `SavedMachine.load` is the bucket triples plus the cursor, verbatim, so a
  loaded window is bit-identical to the saved one — the 2600-tick lockstep
  test compares chunk snapshots, which now carry `load`, so a window that
  failed to survive a load would show up there with no new test. An absent
  or empty ring loads as a fresh window (a pre-window save still loads,
  like `SavedHopper::auto_fill`); a ring that is not exactly 20 buckets,
  has its cursor outside it, or holds an over-full bucket is refused as
  `SaveError::BadLoadWindow`, because `LoadWindow` cannot produce one.
  Restore goes through `LoadWindow::restore` (`pub(crate)`), never a public
  setter.

  (8) **The shipped line takes about 7000 ticks to back up to the crusher,
  not the plan's 1500.** The vat is flat out from ~1500 and the bottler and
  packer are starved from the start, but each machine holds 8 of its
  output and there is a belt or pipe run between every pair, and all of
  that fills at the vat's pace (one craft per 240 ticks) before the mixer
  (~3500) and then the crusher (~7000) stall. `tests/machine_load.rs`
  settles 8000 ticks, then *watches* one full window: status is asserted on
  every tick of it, and a stall kind is asserted as the only reason a
  machine ever gave over the window, never at one sampled tick — a stall is
  a per-tick fact, and a test that samples it once is a coin flip. Ticket
  3's `build_bottleneck_line` needs the same patience (≥ 8000 ticks), or
  it will photograph a green crusher.

- 2026-09-11: **Payout feedback — what earning money looks and sounds like,
  and the project's first audio.** Money arrives through exactly one channel
  (a contract settling at its cadence boundary, `sim/economy.rs`) and it
  arrived silently: a number on the clipboard went up between two frames. The
  brief was Crossy Road's kind of satisfaction, so the moment now gets a coin
  burst off every dock, a scale-and-amber punch on the money line, a rising
  "+N", a PAID stamp on the docket that paid, and a two-note chime; the far
  more frequent moment it leads to (a case landing on the dock) gets a
  seven-tick pop and a wooden knock. Eight calls worth recording.

  (1) **The moment is detected by POLLING per-contract `deliveries_made`
  deltas out of `economy_snapshot_json`, so the sim did not change at all.**
  That snapshot already carries `deliveries_made` beside `payout`, and
  `_update_contract_panel` already parses it every frame, so a rise in the
  counter between two polls IS a payout. Strictly better than watching
  `money`, for three reasons: money also moves when the player buys
  something; a money delta cannot name WHICH docket to stamp; and a rise of
  two means two windows settled and should read as `2 x payout` in ONE
  celebration. Nothing new crosses the `gdext` boundary, so ARCHITECTURE.md
  §4's wire contract is untouched. The first poll of a world SEEDS silently —
  loading a save whose contracts have delivered forty times must not fire
  forty celebrations — and a contract seen for the first time is seeded
  rather than treated as having just paid `deliveries_made` times.

  (2) **The animation clock is `sim.current_tick()`, never
  `Time.get_ticks_msec()`.** Three things follow and all three are wanted.
  It PAUSES WITH THE GAME, so UI_UX.md §5d's "the factory is stopped while
  this is open" is not contradicted by a coin arcing over a stopped factory.
  It stays inside `MAX_TICKS_PER_FRAME`'s existing byte-identical-screenshot
  guarantee (the visual-overhaul-r2 decision) instead of outside it, so
  `--payout` photographs the same burst twice. And it makes the whole effect
  a pure function of `current_tick - start_tick`, which is why the headless
  test drives it by TICKING rather than by sleeping. The cost is that motion
  updates at TICK_HZ (30) rather than at frame rate, which is not a
  compromise for a game whose belts already march at four frames per fifty
  ticks (UI_UX.md §6: "chunky, snappy ... no smooth organic motion").

  (3) **Every distance in the burst is measured in TILE, not in bare world
  units.** This is the mistake the first render caught and it is the entry's
  most transferable lesson. The ballistic constants were tuned in the
  abstract at 3.0 and 3.4, which put the peak a fifth of a tile up and the
  outward spread (1.1) BELOW the width of a coin (0.8) — so ten coins with
  ten distinct directions rendered as one gold lump sitting on the deck, and
  every headless assertion about them passed. There is even a floor set by
  arithmetic rather than by taste: N coins on a circle of radius r stay
  separate only while `TAU * r / N` exceeds the coin's own width. Any
  distance a player compares against the factory has to be in the factory's
  unit.

  (4) **The coin pool is fixed (24), hand-driven and deterministic — not
  `GPUParticles3D` and not `randf()`.** A hard pool means a factory running
  for an hour allocates nothing after its first payout, and an overflowing
  burst steals the OLDEST coin in flight rather than refusing to draw (a
  stolen coin at the end of a fade is invisible; a burst that silently
  shrinks under load is not). Directions are an evenly spaced fan whose whole
  rotation is derived from the tick, and each coin's launch varies by a
  function of its index — a fan thrown at one speed is a ring, a fountain has
  coins at different heights — so two payouts in a row are not the identical
  picture and a screenshot is still reproducible. The pool hangs off a
  scene-level `Feedback` node, NOT off `_world_root`, which is thrown away
  and rebuilt on every place/remove/rotate: a player who lays a belt while a
  payout is in the air keeps their burst.

  (5) **The "+N" is anchored to the money label, not to a dock.** One payment
  is one number; with two docks a per-dock flyout would read as two payouts
  of N. Coins can be per-dock because they are goods and every dock ships
  into the same shared pool, and a dock can legitimately be off screen or
  behind the player in first-person, where the clipboard never is. Its two
  bugs were both render-only: parented straight to the CanvasLayer, nothing
  ever set its size, so right-alignment used a width of zero and put "+90"
  off the side of the screen; and the clipboard is anchored to the top-right
  corner, so there is no `PAYOUT_FLYOUT_RISE` of room ABOVE the money figure
  and the flyout rises out THROUGH it instead, down to a fifth alpha by the
  time it crosses.

  (6) **Sound is synthesised in GDScript, not committed as `.wav` files.**
  This is UI_UX.md §2a's voxel argument applied to audio: that section's
  driving constraint was that every asset be creatable and editable by
  Claude and not only by a human in a GUI, and a recorded `.wav` is an opaque
  blob only its author can regenerate. A few hundred samples of arithmetic
  are diffable and tunable by changing a number, and `AudioStreamWAV` takes
  raw PCM so no file need exist. Noise transients run off an explicit LCG
  rather than `randf()`, so two runs of `verify-core-loop.sh` differ in
  nothing at all. The limit is stated rather than discovered later: this buys
  arcade tones and not recorded material, and a future music/ambience layer
  would want real files. **This resolves UI_UX.md §8's open "audio direction"
  question, narrowly:** sound here is FEEDBACK, not a score or an ambience
  bed — it fires on the moments the player is meant to feel and is silent
  otherwise, which is what keeps a factory that runs for an hour from
  becoming an hour of noise. The payout is two rising notes over a low thump
  (an interval reads as an event where a single tone reads as a
  notification); the delivery knock is a fifth of its length and well under
  half its level, because it fires every few seconds on a working line.

  (7) **`preload`, not `class_name`.** `audio.gd` is reached by
  `const FactoryAudio = preload("res://audio.gd")`, the way `game_session.gd`
  and `test_layouts.gd` already are. A `class_name` is only registered after
  an editor scan, so a headless `--script` run against a cold `.godot/` would
  not find it — which is precisely how every test in this project runs.

  (8) **What the two verifications each cover.**
  `godot_project/test_payout_feedback.gd` (`PAYOUT_FEEDBACK_OK`, in
  `verify-core-loop.sh`) proves the numbers end to end: a real payout on the
  shipped line at tick 2500 fires exactly one celebration for the paying
  docket's own payout, a two-window and a two-contract delta each celebrate
  ONCE for the summed amount, an unchanged board is silent, everything decays
  back to exactly nothing, dock arrivals get the small cue and never the big
  one, and the synthesis is byte-identical run to run.
  `scripts/screenshot-scene.sh --payout` is the eyes, and it earned its place
  immediately by finding (3) and (5) with every one of those assertions
  green — UI_UX.md §2b point 12, again. It waits for the EVENT rather than a
  tick count (a hardcoded 2500 would quietly photograph an empty apron the
  day a cadence moved) and then FREEZES the scene, because a burst is over in
  34 ticks and the shot's own warm-up is 45 frames.
- 2026-09-11: **Machine animations — every processing machine has a moving
  mechanism, and it moves only while the machine is actually working.**
  Six calls worth not re-litigating.

  (1) **A machine is animated by a separate MOTION MODEL in four frames,
  not by regenerating its body and not by rotating a node.** The belt family
  already answers "how does a voxel thing move" (UI_UX.md §2b point 23):
  emit the same asset in `MACHINE_FRAMES` arrangements, stand them all in
  the scene, show one. The alternatives were both worse. Spinning a child
  node in GDScript gives smooth organic motion, which §6 explicitly does not
  want, and it would put un-art-directed angles on screen at every frame —
  the same reason §1 forbids a free camera. Folding the mechanism into the
  body means five machine models become twenty, and every future edit to a
  machine's shell has to be made four times. So: `<machine>_motion_f0..f3`,
  cut on the machine's own footprint, and `main.gd` builds four sibling
  nodes at the body's position and yaw with `visible = k == 0`. Siblings
  rather than children for the reason the progress gauge is one —
  `body.get_node_or_null("emissive")` must keep finding the machine's own
  status lamp, not a lamp inside an overlay.

  (2) **The moving parts are CUT OUT of the body, not duplicated into the
  overlay.** This is the call that actually cost geometry: the crusher lost
  the outer skin of its rolls, the mixer its ribbon, the vat its agitator,
  the bottler its nozzles and chuck, the packer its taping head. The reason
  is point 24 — two voxel solids z-fight wherever a voxel of both has an
  outward neighbour in neither — and a z-fight at scene scale reads as a
  texture seam, not as a bug anyone goes looking for. Disjointness is
  arranged by construction (`cylinder_*` fills `d <= r` and `arm_cells`
  fills `inner < d <= outer`, so a core and an annulus about one axis are
  provably disjoint) and then PINNED: a voxgen test walks every voxel of
  every frame of every family against its body. It is the test most likely
  to be broken by a later tweak to either half, which is exactly why it
  exists.

  (3) **Every motion volume is an EVEN number of voxels tall.**
  `vox::encode` lifts a model by `model.size[2] / 2` in whole voxels, and
  that is integer division, so a 33-tall overlay sits half a voxel below a
  34-tall body and the two meet along a seam instead of a face. Found as a
  latent bug in the generic `machine_motion` before it ever rendered, and
  now stated as a rule with a test behind it rather than as a fact about one
  model.

  (4) **The four frames must differ CONSECUTIVELY, not pairwise.** The first
  version of the test asserted that no two of the four frames were the same
  model, and the bottler failed it: its plunge is a there-and-back offset
  table (`BOTTLER_PLUNGE = [0, -1, -2, -1]`), so f1 and f3 are the same pose
  — which is what a reciprocating part honestly does. The fix considered and
  rejected was to add a spinning capping chuck so all four differ; the
  generator's own doc already records that a bottling line plunges rather
  than spins, and inventing a rotation to satisfy a test is the test writing
  the art. So the rule is the one that actually matters — the mechanism
  never stands still for a frame while it is meant to be working — checked
  round the loop so the last frame is compared with frame 0 too, plus a
  second, stronger test (`a_turning_machine_shows_four_different_angles`)
  for the four `spin`-driven families, where four distinct angles genuinely
  must hold or the fold count and `MACHINE_FRAMES` divide each other.

  (5) **The gate is `process != null` AND `0 <= elapsed < required`, not
  `process != null`.** This is the single piece of simulation knowledge the
  whole feature turns on. `Machine::tick` (`sim_core/src/sim/machine.rs`)
  clears `self.process` only once the finished goods have somewhere to go: a
  machine that has completed its batch and is blocked on a full output
  buffer or a full pipe KEEPS its process and retries every tick. Reading
  that as "running" would leave a jammed factory full of machines cheerfully
  turning with nowhere to put anything — the opposite of what the player
  needs to see. Splitting it the other way gives a genuinely useful pair
  with the status lamp, which is unchanged: the lamp says "has a batch", the
  mechanism says "is turning", and **lamp lit with nothing turning means the
  thing downstream is full**. Neither signal says that alone. The elapsed
  count is derived from `current_tick()` rather than from a snapshot, for
  the reason `_machine_progress` already is: a machine dirties its chunk
  when its state CHANGES, so nothing arrives during a process.

  (6) **One pace for every kind: `MACHINE_TICKS_PER_CYCLE` = 30.** A
  machine's own process length is the obvious driver and is wrong — the
  crusher's 80 ticks against the vat's 240 reads as two factories running at
  different speeds rather than one factory with a slow cook in it, and the
  cook already states its length through the product backing up behind it.
  The pace says "working", not "working this fast"; the belt is what carries
  speed. Judged by the numbers rather than by eye, the way the belt's march
  was: 30 ticks at `TICK_HZ` 30 is four frame changes a second against the
  belt surface's 2.4 (`BELT_TICKS_PER_TILE` 50 over `BELT_FRAMES` 4), so a
  mechanism reads as busier than the rubber it feeds, and a fifth of a
  second a frame is the chunky end of UI_UX.md §6 rather than a blur. The
  frame comes from the machine's OWN elapsed ticks rather than from the
  global tick, so a mechanism starts its cycle at frame 0 the moment work
  starts instead of joining a factory-wide beat part way through.

  Four smaller things, recorded so they are not rediscovered.
  **`_update_machine_motion` is called only from `_advance_simulation`**,
  beside `_update_belt_frame` and for the same reason: the paused
  early-return never reaches it, so the pause menu stops the machinery for
  free and a headless test that never ticks sees frame 0.
  **The mixer's ribbon spans `MIXER_RIBBON_Y` (rows 20..43) rather than the
  drum's full length** — the voxgen disjointness test found that a flight at
  `MIXER_RIBBON_R` passes straight through two feed throats and the
  discharge nozzle. Shortening the ribbon rather than thinning it everywhere
  keeps the flights out at the drum wall where the sight window (y 24..39)
  actually shows them turning, so what the player can see is unchanged.
  **The crusher is excluded from the constant-mass test**: its teeth are
  culled below `CRUSHER_POCKET_FLOOR_Z`, so a tooth turning down into the
  nip leaves the model — which is exactly what it does to the player's view
  of it, since below that line it is inside the frame.
  **A `*_motion_fN` overlay gets no catalogue icon** (`icons.rs`'s
  `is_motion_overlay`, alongside the existing `_fN`-for-N≥1 skip): a spider
  of four spokes floating on nothing is not a picture of a machine.

  **No steam.** It was the obvious addition for the cooking vat and it is
  wrong twice over: a puff is mass a stopped vat should not have, and a
  frozen cloud over a cold vat states the opposite of what the lamp says.

  Two things the work turned up outside its own scope. **`test_screenshot.gd`
  never advanced the belt frame**, because its populate loop calls
  `sim.tick()` directly rather than going through `_advance_simulation` — so
  every scene screenshot ever taken has photographed belt frame 0, an
  idle-looking factory in the middle of a run, which is precisely the lie
  that loop's own `_apply_dirty_chunks`-every-tick comment is about. It now
  calls `_update_belt_frame()` and `_update_machine_motion()` there; both
  read the tick, so shots stay reproducible. And **the vengi preview of a
  rigid-translation frame is byte-identical to frame 0's** (the packer's),
  because the thumbnailer crops to the model's bounding box — a preview
  artefact, not a still mechanism; in the scene the volume is fixed and the
  carriage really travels. Point 22's warning about vengi's camera, in
  another form: judge a translation in the scene.
- 2026-09-10: **Belt arrows removed — the belt's moving mark is a plain
  cleat, and direction is said in words.** The pipes and conveyors
  overhaul's ticket 2 put a solid `hazard_yellow` arrow on every belt tile
  (Part B point 23 records why it was solid and raised). The player looked
  at the running game and asked for the arrows to go, so they are gone.
  Three things about how.

  **The mark was not simply deleted, because it was also the animation.**
  Frame `k` of a belt is frame 0 with the surface pattern advanced a
  quarter tile, and the arrow was the whole of that pattern; delete it and
  all four frames are byte-identical, so the belt stops running with every
  test still green — the exact-shift test only says frames must differ by
  the shift, and identical frames do. The arrow is therefore replaced by
  the least mark that keeps the frames distinct: a **cleat**, a bar of
  `rubber_dark` two voxels long across the lane, one voxel proud of the
  rubber (raised for the same camera reason as before), a voxel short of
  the rubber's edge on each side. It is what a real cleated conveyor belt
  wears and it is not signage. `BELT_ARROW_Z`/`arrow_covers` became
  `BELT_CLEAT_Z`/`cleat_covers`; the `SurfaceCell` machinery and the
  corner's stretched `u` are unchanged, so the cleat bends round a corner
  exactly as the arrow did.

  **Direction is said, not drawn.** UI_UX.md §7 wants every meaning
  carried by a shape, a word or a position, never colour alone; the arrow
  was the shape. Its job is already done in words in two places — the
  build prompt names the ghost's facing (`✔ PLACE  Belt  ·  facing east`)
  and a placed tile's hover says `carries east · turns left` — and a
  running line shows it by moving. So nothing in the Godot layer changed
  for this; only a comment that said the ghost showed its facing by the
  arrow. The one loss is that a lone, idle straight tile no longer shows
  which way it faces from its model alone; hover does, and R turns it.

  **The scope is the belt family only.** The splitter's inlaid next-output
  arrow is a different readout (which of two outputs takes the next item,
  a live fact per tile) and stays. `cargo test -p voxgen` now asserts no
  signage colour stands anywhere on a belt's running surface, so an arrow
  cannot quietly come back. Twelve `.vox` models and three icons changed
  and nothing else in `models/` or `icons/` did.
- 2026-09-11: **Vinegar looks like vinegar — a raw fluid gets a palette
  colour of its own, a model, and a route to the icon.** Vinegar shipped
  with neither: no entry in any model table, so `_market_asset_for` fell
  through to `PIPE_ASSET` and the ingredient market, the larder strip and
  the hopper picker all offered the player *a length of pipe* to buy; and
  no colour anywhere but a single `FLUID_TINTS` gold in `main.gd`, which at
  scene scale read as cooking oil. Four calls.

  (1) **A fluid's hue belongs in `palette.ron` even though a fluid model's
  hue does not.** §2b point 5's rule stands — the *pipe's* fluid body is
  near-white and tinted, because which liquid is in a pipe is live sim
  data — but the tint's authoritative value is now a palette entry
  (`vinegar_amber`, `#AD6B23`, class `liquid`), and `FLUID_TINTS["vinegar"]`
  is that RGB with a comment saying so. The moment a fluid has a still model
  as well, the two are seen together: the market icon sits a few centimetres
  from the pipe run it fills. Two independent definitions of one colour is a
  divergence waiting for someone to tweak one of them.

  (2) **Amber, not clear.** Real distilled vinegar is colourless, and
  colourless in a stainless pipe with an inspection window is indistinguish-
  able from an empty pipe — the player would read a working line as a broken
  one. Malt/cider vinegar is amber and real, so the honest option and the
  legible one coincide. The first attempt (`#C07A28`) read orange enough to
  be mistaken for the tomato stages one tile away, and was darkened after a
  scene render, not after looking at the swatch.

  (3) **`FLUID_MODELS` is a separate table from `ITEM_MODELS`, not a couple
  of extra keys in it.** `ITEM_MODELS` answers "what does this look like
  riding a belt", and no belt can carry a fluid; a fluid's model exists for
  the market slot, the larder chip and the icon pass. Keeping them apart
  means neither table's fallback can quietly answer the other's question —
  which is exactly the bug being fixed, `_market_asset_for` reaching
  `PIPE_ASSET` because the only table it consulted was about belts.
  `_market_asset_for` and `_icon_for` both check `ITEM_MODELS`, then
  `FLUID_MODELS`, then fall back.

  (4) **A model whose only consumer is the icon renderer is authored for
  vengi's camera** (new §2b point 35). The jug's label and moulded ear were
  first put on the faces the *game* camera sees, per point 11, and were
  invisible in the rendered icon: `vengi-thumbnailer` looks from the -X/+Y
  corner (point 22), the opposite side. Confirmed by rendering `item_box`,
  whose own labels are missing from its own icon for the same reason. Since
  vinegar is a fluid it can never appear in the scene at all, so the icon is
  its *only* consumer and the detail was re-authored for that corner. Two
  more shape lessons from the same render loop: a squat ovoid jug read as "a
  pale plate with a rim of amber" under the isometric icon angle and had to
  become tall and straight-sided, and the fill line has to sit well below the
  shoulder or the glass above it reads as a cap.

  Palette entries were **appended**, never reordered (that re-indexes every
  `.vox`), and the proof is in the diff: all 132 models show as modified
  because the palette is embedded in each, and `item_vinegar.png` is the only
  icon whose bytes changed.
- 2026-09-11: **The factory floor, redesigned — the one asset that is
  authored by its value range rather than by its detail.** The project
  owner's note was that the flooring tile did not fit the theme. It did not:
  a `concrete` slab with a `#33332F` saw-cut joint on all four edges and
  single-voxel `concrete_light` speckle scattered over it, identical on
  every tile of the world. Rendered wide, the joint read as bathroom grout
  and the speckle as television static — a tiled bathroom, not UI_UX.md §3's
  poured industrial food plant. Five calls, each of which was made by
  looking at a render and none of which is obvious from the generator.

  (1) **The floor gets its own concrete, and the three tones sit within
  eight points of each other.** `concrete_slab` / `concrete_mottle` /
  `concrete_shade` are new `palette.ron` entries rather than a change to the
  shared `concrete`, because the dock deck and the machine plinths want to
  read as part of their machine while a slab wants to read as the ground the
  machine stands on. `concrete_slab` is a shade darker than `concrete` so
  stainless and painted housings have something to pop against (§4). How
  CLOSE the three sit is the decision: a first pass put them a dozen points
  apart and the render came back reading as confetti scattered on the floor
  rather than as the floor.

  (2) **Variation is broad blotches, never per-voxel noise — and that is
  also the cheap option.** `floor_mottle` works on a coarse 8-voxel cell
  grid, leaves most cells bare slab, and builds each blotch as rows of
  differing length and inset so its outline wanders (a rectangle with its
  corners knocked off renders as a plus sign). Beyond looking like a
  trowelled pour instead of static, broad low-contrast areas greedy-mesh
  into far fewer triangles: the scene render went from 1 363 384 primitives
  to 798 272, a 41% cut, on the one asset whose triangle count is multiplied
  by the size of the world.

  (3) **Four models, and per-instance rotation is what makes them look like
  more than four.** `floor_tile`, `floor_tile_worn`, `floor_tile_patch` and
  `floor_tile_drain`, chosen in `main.gd` by `_floor_variant_for(tx, tz)`.
  A magnified crop of the first four-model version showed the obvious
  thing in hindsight: four stamps are still four stamps, every worn tile's
  scuff running the same way across the screen. `_floor_yaw_steps_for` lays
  each slab at a hash-chosen quarter turn, which fixes all of it at no asset
  cost. **The price is that the tile's edges must be four-fold symmetric**,
  so `floor_joint` keeps its saw cut on all four edges even though that
  makes an interior joint two voxels wide — and `floor_seam` was lightened
  `#33332F` → `#50504A` until the doubled line still read as a hairline. Two
  edges instead of four would have halved the line and broken every seam the
  moment a tile turned. **Both the variant and the turn are pure functions
  of (tx, tz)**, because `_build_floor` runs on every place and remove: a
  hashed-on-nothing floor would repave itself under the player and come back
  different from a save.

  (4) **The drain is laid in whole ROWS, not scattered.** A trench drain is
  the detail that says "a plant that is hosed down" rather than "warehouse",
  and it only says it if consecutive tiles join into one trench —
  `posmod(tz + 4, 9) == 0`, phased so no drain crosses tile row 0, where an
  empty world is framed and a player lays their first belt. It is the one
  model `_floor_yaw_steps_for` never turns, since a rotated tile in a row
  would put its channel across the trench. Its grating bars are the one part
  of this floor that WANT contrast (a grating is read by its stripes), so
  they stay pinned to `concrete` rather than to the mottle tone, which is
  pulled toward the slab whenever the floor is quietened. It also finally
  uses `drain_grate`, a palette colour that had been defined and never
  drawn.

  (5) **The repair patch is DARKER than the slab, and that direction was
  decided by measuring a render.** A patch should read as somebody having
  cut a rectangle out of this floor with a saw, and the real-world instinct
  is that a fresh pour is paler than a floor polished by traffic. Filled
  with `concrete_light` it read as white paving slabs; filled with
  `concrete`, one step up, it still measured +16 render units against a
  floor whose entire remaining variation was ±2 — the loudest thing on
  screen. It is `concrete_shade` now, about eleven units DOWN, with its saw
  cut in `concrete_dark` (which `floor_seam` would otherwise be too close
  to swallow). The governing rule is §4: metal is the deliberate bright
  exception, so the floor must never hold the brightest pixel in the frame.
  A dark cut is fine here precisely because there is one per ~125 tiles —
  the grout problem in (3) was a dark line on all four edges of *every*
  tile. Frequency was cut with it (`FLOOR_PATCH_IN` 41 → 97).

  Recorded as UI_UX.md §2b points 36 and 37. Three voxgen property tests
  pin what a render cannot re-check by itself: nothing on the floor stands
  proud of the slab's top surface (`main.gd`'s `FLOOR_Y` puts everything
  else on that plane, so one voxel too tall intersects every belt and
  machine the player builds, and it is invisible until something stands on
  the offending tile); every rotatable model carries the identical joint on
  all four edges; and the drain's channel crosses the whole tile,
  symmetric about the midline. The whole floor stays in the `matte`
  material class, because `_build_floor` sets `material_override` on the
  multimeshes.
- 2026-09-11: **The UI visualisation pass — every readout in the HUD was
  asked what its number MEANS, and the answers are one bar language.** The
  fill bar that shipped on the hopper configuration panel was the readout
  the player singled out as looking right, so this pass took that idiom
  across the hover tooltip, the contract clipboard, the build menu, the
  ingredient market and the money figure. Seven calls worth not
  re-litigating.

  (1) **A bar is drawn only where the SIMULATION defines the ceiling.**
  Hopper and tank level (capacity), a machine's cycle (its recipe's
  `time_ticks`), a pipe segment (its pressure against full), a contract's
  countdown (against its own cadence), a price (against the money in the
  player's pocket) — all of those are genuine fractions, and a fraction is a
  bar. Larder stock and the balance itself are **not**: nothing in the game
  says how many tomatoes is "full", so a bar for them would have to invent a
  maximum, and a gauge whose full mark is fiction teaches the player a scale
  that does not exist. They stay figures. This is the rule to apply to the
  next readout somebody adds, not the list of the ones that exist today.

  (2) **One mechanism, once.** `_build_meter(parent, width, fill, track,
  border, height)` builds a track Panel with a fill Panel anchored inside it
  and **returns the track**; `_set_meter(track, fraction, fill_color)` sizes
  and colours the fill. A width of 0 means "expand to fill the row". Two
  bars in the same HUD built two different ways would drift apart in height,
  border and idle appearance within a ticket, which is exactly what the
  voxel side learned about materials (UI_UX §2b point 2).

  (3) **`_set_meter` records the fraction it drew on the track's own node
  meta.** Godot lays nothing out in a headless SceneTree, so a test can
  never read a bar's pixels; without this, "the countdown drains" could only
  be asserted through the label beside it, which is a different value with
  its own formatting. The meta is the drawn number itself, so
  `test_contract_panel.gd` watches three countdowns fall independently as
  the real simulation runs, and `test_hover.gd` asserts which tiles get a
  meter at all. A presentation value that a headless proof can read is worth
  the two lines it costs.

  (4) **Three fills, each a meaning, never a mood.** `METER_FILL_LEVEL`
  (green) is how full something is, `METER_FILL_PROGRESS` (amber) is work
  under way, `METER_FILL_WARN` (red) is a cost that cannot be met or a clock
  nearly out. Every one of them is accompanied by a word, a glyph or a
  figure — the countdown says "due now", the missed tally says "✗ 2 missed",
  an unaffordable slot is dimmed and marked as well as red (UI_UX §7).

  (5) **The clipboard's bar is paper and ink, and its clock is in
  seconds.** UI_UX §5 gives a docket exactly two colours (the checkbox's
  paper tone plus a steel hairline) and this pass did not invent a third:
  the countdown is ink on paper inside the card's own hairline, with the
  docket's single warning red reserved for the last stretch and for a
  non-zero missed tally. The figure beside it reads **"8s to go" / "1m 15s
  to go" / "due now"**, never a tick count — ticks are an implementation
  detail of ARCHITECTURE.md §3.9's fixed timestep, and a board that prints
  them is asking the player to do the division. Two layout facts came out of
  building it: the countdown, the cadence and both tallies share ONE row,
  because a second row pushed the third docket off the bottom of a 1600x900
  board (UI_UX §5c says nothing scrolls there), and the bar needs
  `CONTRACT_METER_MIN_WIDTH` as well as `SIZE_EXPAND_FILL`, because on the
  docket whose cadence spells out longest the expanding bar was squeezed to
  zero — a gauge that vanishes when the text beside it grows is worse than
  no gauge.

  (6) **A price is drawn as the share of the balance it would take.** That
  is the honest relation the player is actually deciding on, and it means
  the same bar answers "can I afford it" (a full red bar when they cannot)
  and "what will this cost me" (a sliver at a healthy balance). It is
  floored at `SHOP_METER_MIN_SHARE` so that something buyable always draws
  something; the consequence, accepted, is that at a large balance several
  cheap kinds draw the same sliver and the numbers above them carry the
  difference. The alternative — scaling each price against the most
  expensive thing in the menu — makes a bar that never changes as the player
  earns and spends, which is the one thing this readout is for.

  (7) **The money figure announces what just changed it.** A contract
  settles on its own clock while the player is looking at their factory, so
  the only evidence of a payout was a balance that had quietly become a
  different number by the next time they glanced at it. `+90` in green and
  `-5` in the docket's warning ink now flash under the figure and fade, on
  the hotbar name flyout's own hold/fade timings, because two transient
  readouts that appear and vanish the same way are one idiom rather than
  two. It floats over the dock line below rather than reserving a row that
  would be empty almost always. Two traps: the deadline must start one whole
  fade **in the past**, since `Time.get_ticks_msec()` is milliseconds since
  the process started and a zero deadline is still inside the fade window
  for the first second of a session (the headless proof caught exactly
  that); and a change smaller than a whole unit must not flash, since the
  figure above is drawn whole and a `+0` is an event the player cannot see.

  One thing found rather than decided: the pass kept turning up the wire's
  own ids in player-facing text (`boxed_ketchup` as a checklist heading,
  `delivery dock` as a tooltip title). Everything the player reads now goes
  through `_kind_display_name`/`_item_display_name`, and the headless tests
  were updated to assert the display names — which is the point, because a
  test asserting the raw id is what let them survive this long.
- 2026-09-10: **The contract board — contracts are signed, struck and
  lost.** The request: start with one basic contract, buy bigger and more
  complex ones as the factory grows, lose a contract that is not fulfilled
  and have to buy it back, never hold two of one contract, and show on the
  clipboard how well each order is being filled and when it is in danger.
  Eight calls worth not re-litigating.

  (1) **`contracts.ron` is a board of offers, and the file went to schema
  2.** Three fields were added — `price`, `strikes`, `starter` — and a v1
  file is rejected, not defaulted: a file that says nothing about what a
  contract costs or when it is lost has no honest default for either, and
  the loader inventing one would put the answer to a design question in a
  `serde(default)`. `price` and `strikes` must be at least 1 (a free-to-
  recover contract cannot be lost in any sense that matters; a zero-strike
  contract can never be lost at all, which is v1's contract and v1 is gone
  on purpose), and at least one offer must be a `starter`, so a new game is
  never a factory with nobody to sell to.

  (2) **The economy owns the board and the active list is a subset of
  it.** `Economy` now holds a clone of the `ContractRegistry` beside
  `contracts: Vec<ActiveContract>`. Signing needs an offer's terms at
  command time and nothing else in `World` keeps a registry after
  construction; a clone of four structs is cheaper than threading a
  reference through every constructor. `World::new` activates the
  starters only.

  (3) **One signature per offer, enforced where money moves.**
  `Economy::try_sign` refuses an already-active id before it touches the
  balance. The alternative — let a player stack several copies of a
  lucrative order — is the "double a line's income for free" failure the
  allocation rule already closes for goods, opened again one level up.
  The command path drops the `SignRefusal` on the floor (every refused
  command is a silent no-op); the enum exists so tests and a UI can say
  why.

  (4) **Strikes are consecutive, a payout forgives, and losing is the
  whole penalty.** GAMEPLAY §11 had three candidates: immediate loss, a
  grace period, a reputation meter. Immediate loss makes the starter
  unplayable (a first line WILL miss a window while it is being built);
  a reputation meter is a second number to explain and a second thing to
  balance. A run of `strikes` missed windows is a grace period counted in
  the contract's own units. Consecutive rather than cumulative because a
  cumulative count is a fuse that only burns down, turning every long game
  into a certain loss; a factory that recovers should be forgiven and only
  one that keeps failing should lose the customer. No fine on top of the
  loss: the fee to sign again is already printed on the board, and a
  second penalty number would be one more thing the docket has to say.
  A re-signed contract remembers nothing (new agreement, zero counters);
  the board remembers how many times it was lost (`Economy::losses`), for
  the row's "lost twice before" note and nothing else.

  (5) **A signed contract takes its file-order rank.** The scarcity
  tiebreak is `contracts.ron` order (the contracts ticket 1 rule); if a
  signature appended to the list instead, priority would depend on the
  order a player happened to buy things in, and a save/load would have to
  preserve that history. `insert_in_board_order` is the one insertion, used
  by signing and by the restore path, so a loaded game is in the same
  order a played one is.

  (6) **`Economy::tick` returns events, not indices.** It used to return
  the paid contracts' indices; with removal inside the loop an index is
  meaningless a line later. `ContractEvent::{Paid, Missed, Lost}` in settle
  order, `Lost` always following the `Missed` that caused it. `World::tick`
  discards them — the snapshot is the wire and Godot diffs it — but the
  unit tests read them, and a future sound cue will.

  (7) **Save schema stays 1; the active list is restored exactly.** Two new
  fields are `#[serde(default)]` (the auto-fill flag's precedent): an old
  save's every other field means what it did, and an old save with all
  three contracts active is a legal state (they were signed before signing
  existed). The load path now CLEARS the empty world's starters before
  inserting the saved list, since "the starter is not in the save" now
  means "the player lost it", not "the board changed". Consequently
  `SaveError::MissingContract` is deleted: a board offer the save does not
  name is unsigned, not an error. The other direction — an active id the
  board does not offer — is still refused, and so is the same id active
  twice (`DuplicateContract`).

  (8) **Presentation: a menu for what is for sale, a docket for what is
  running, and danger said four ways.** The board is a dark-plate menu on
  **C** (the key the delivery-dock ticket freed; opening a menu changes
  nothing, which is what `test_interaction.gd` still asserts of it), with
  the clipboard's paper docket as its row and one SIGN button naming the
  price — SIGNED / ✗ SIGN · 150 / LOST — sign it again. It is not a section
  of the clipboard because a SIGN button beside running orders makes every
  docket look clickable. The docket gained a status line (✔ READY / ⚠ AT
  RISK / ⚠ LAST CHANCE, with "N short" and the window percentage), strike
  pips with the rule in words, and a heavier border in the
  state's colour, from one pure function (`_contract_risk_for`) the test
  drives directly. "At risk" is short with half the window gone
  (`CONTRACT_RISK_FRACTION`); "last chance" is short with the next miss
  losing it, however early. The docket keeps to paper, ink, its done-green
  and ONE warning red — at-risk and last-chance share the red and are told
  apart by words, border weight and pips — because the concurrent UI
  visualisation pass had just pinned "no third colour" on the card and a
  hazard amber would have been exactly that. The wire has no loss event,
  so the clipboard diffs active ids between polls and announces "✗ CONTRACT
  LOST: <name> — sign it again on the board (C)" (first poll only primes,
  so a loaded save is not read as a string of losses), and an empty
  clipboard says "press C" rather than showing money over nothing.

  Shipped content: four rungs — the unchanged starter (3 strikes, 40 to
  re-sign), the diner and stall orders (2 and 1 strikes, 150 and 60) and a
  new regional grocer (6 boxes / 4000 ticks / +210, 400 to sign, 2 strikes)
  whose demand one line cannot carry beside the other three, which is
  GAMEPLAY §6's pull toward a second line made concrete. All placeholders
  per GAMEPLAY §10. Built on branch `TomWhitticase/contract-board` in its
  own worktree because the `anthias` worktree had a live concurrent
  session in `main.gd`.

- 2026-09-11: **The contract board is sized in lines' worth of demand — a
  starter tier one line can serve, plus a growth tier for every line after
  it.** The trigger was a playtest that ran out of game: money arrived only
  through the three shipped contracts, which together asked for ~3.9
  boxes per 1000 ticks against a single line's ~4.1, so a first line
  earned enough to build a second and the second had nothing to sell to.
  Overproduction is a real loss here (an unordered box is ~11 of
  ingredients spent for nothing), so a second line was not merely useless
  but negative. Five calls.

  (1) **Demand is measured in the throughput of one line, and the board
  has to exceed it by several lines.** One line's steady state is one unit
  per bottleneck cycle (the 240-tick vat), so the unit is "boxes per 1000
  ticks" and the board is sized as tiers of it: starter ~3.9, growth
  ~19.2, whole board ~23.0 — nearly six lines. The number is not
  sacred; the RULE is that the board must always leave the next line an
  order to go after, and `contracts.rs` pins it against the slowest
  recipe's `time_ticks` so a tuning pass on the vat moves the expectation
  rather than breaking it. A content change, not a sim change.

  (2) **The starter tier is the original three contracts, unchanged.** A
  new player's first ten minutes are the most-tested part of the game and
  nothing about them moved; the growth tier is appended after them.

  (3) **Per-box rate rises with order size, on every tier.** The shipped
  three already did this (24/box for one box, 30/box for three); the
  growth tier continues it to 40/box for the 20-box pallet. A big order
  is the one a single line cannot serve — twelve or twenty boxes have to
  stand on the dock at once — so the better rate is the reward for having
  built the capacity, and the test pins "never worse per box for a bigger
  order" rather than the literal figures.

  (4) **Priority stays smallest first.** File order is the allocation
  tiebreak under scarcity, and every cadence on the board coincides every
  30 000 ticks. Listing the pallet order first would let it eat twenty
  boxes a beginner had accumulated for the corner shop; listing it last
  means a big order that CAN be filled still is, out of what the small
  ones left, and a player who cannot fill it loses nothing but that
  window's payout (a miss still has no other cost — GAMEPLAY.md §11).

  (5) **Two contracts may share a cadence.** The old panel test demanded
  every docket's cadence line be unique, which was a proxy for "each
  docket's countdown is its own"; with two orders at 1500 and two at 2000
  it checks the real property instead — each card names its own contract's
  cadence, and cards on different cadences print different lines.

  Consequences noted rather than fixed: the clipboard's docket stack now
  scrolls at 1600x900 (three and a bit dockets visible), so the cap that
  was built as a safety valve is the ordinary case and a compact board is
  the next UI step; and every order is still `boxed_ketchup`, so the
  mixed-sauce orders GAMEPLAY.md §6 describes still wait on a second
  sauce line. Money is `Fx` (I16F16, ceiling ~32 767) and a five-line
  board earns toward that ceiling in minutes rather than hours — a
  pre-existing limit this makes nearer.
- 2026-09-10: **Hopper/tank configuration, ticket 2 of 2 (the Godot half,
  plus the one sim deletion ticket 1 deferred to it) — a placed hopper or
  tank is CLICKED, and everything about it is on a panel beside it.** The
  three global keys are gone: **SPACE** (fill every hopper), **V** (fill
  every tank) and **Q** (cycle a hopper's contents), and with them
  `PlayerCommand::CycleHopperItem` — `SetHopperItem` is the one way a placed
  hopper changes its mind now, exactly as ticket 1's entry said it would be.
  Six calls worth not re-litigating.

  (1) **A left click outside build mode is the configuration click, and the
  build click wins where both could apply.** Until now a click on the floor
  with nothing in hand did nothing at all; it opens the panel of the hopper
  or tank under the cursor, from any tile of its footprint. In build or
  remove mode the click still places or removes — a click that placed a
  belt AND opened a panel would be two answers to one gesture. The pure half
  (`_config_click_at_tile`) is what the test drives.

  (2) **The click is a toggle, only one panel ever exists, and anything
  else closes it.** The same building again closes; a different hopper or
  tank retargets; empty floor, any other kind, or no tile at all closes
  outright — a panel describing something the player has stopped looking at
  is worse than none. It also closes itself, from the per-frame poll, when
  the build menu, the market, build mode or remove mode takes the screen, or
  when the building it describes is removed or stops being its kind — so no
  caller has to remember to close it. **ESC closes it FIRST** and returns:
  it is the innermost thing on screen, and "back out of what I am in" has to
  mean the panel rather than also dropping the ghost or reaching the pause
  menu.

  (3) **The panel reads the simulation, never a local copy.** Level, fill
  bar, stock line, the FILL button's enabled state and the AUTO-FILL
  switch's ON/OFF are all recomputed every frame from `hopper_snapshots_json`
  / `fluid_tank_snapshots_json` and the ingredient ledger, so the toggle can
  never claim a state the simulation does not have, and a level that moved
  because the factory ate a tomato moves in the same frame. Every command
  it issues is tile-addressed to the ONE building it is about; "fill on this
  panel loaded the other hopper" is an explicit test failure.

  (4) **FILL is disabled WITH A REASON IN WORDS**, not greyed out: "No
  Tomato in stock — buy some at the Ingredient Market (M)", "Full — nothing
  more will fit", and — the refusal a hopper cannot have — "Not connected to
  a pipe run" for a tank plumbed to nothing, which has nothing to be charged
  for. That is the DEVLOG 2026-09-09 "SPACE and V say why" lesson moved to
  the one place it can name the one missing ingredient. The switch says
  **AUTO-FILL: ON/OFF** in text as well as colour (UI_UX.md §7), and the
  picker marks the chosen ingredient with a tick and the hotbar's own
  selected-socket style, so "the thing I have chosen" looks the same
  wherever the game asks.

  (5) **It is machine-side factory UI, in the dark plate** — the build
  menu's and the hotbar's language, deliberately not the clipboard's paper,
  which UI_UX.md §5 reserves for orders. It stands beside the building (the
  footprint centre projected to the screen, so it follows the camera) and
  never on it; past the window's edge, or **past the contract clipboard's
  left edge where the two would overlap**, it flips to the building's other
  side — the first rendered frames put it squarely over the one readout
  that says whether the factory is paying, which a panel about one bin has
  no business hiding.

  (6) **The test pins the mechanic, not only the widget, and one of its
  expectations was wrong in an instructive way.** `test_config_panel.gd`
  switches auto-fill on and ticks; the first cut expected the tank to sit
  near full after 200 ticks and it sat at 24 of 120 — because the larder's
  one crate of vinegar had all gone into the tank and its run, and the
  top-ups had stopped exactly as ticket 1's call (3) says they must. The
  test now asserts BOTH halves: a dry larder stops the top-ups with the flag
  still armed, and a stocked one keeps the tank within a unit of full while
  the ledger goes down. **Also swept in:** the market footer still told
  players "then SPACE loads a hopper and V loads a tank" after both keys
  were deleted; `test_presentation.gd` and `scripts/screenshot-scene.sh`
  still drove SPACE and Q (`--fill-blocked` is replaced by `--config TX,TY`,
  and `--contents N` now presses the open panel's picker slot). A deleted
  key leaves stale copy in more places than its handler, and none of the
  headless tests could see the footer.

- 2026-09-10: **Pipes & conveyors overhaul, ticket 5 of 7 (belt
  presentation: shape from `links`, junctions at every facing, animation
  at sim speed, items through corners; `godot_project/` plus docs).** The
  plan fixed the wire contract, the frame scheme and the corner
  conventions; tickets 1, 2 and 4 built them; this ticket consumed them.
  Seven calls worth not re-litigating.

  1. **The shape rule is one pure function of `links` and facing.**
     `_belt_shape_for` returns one of six words — straight, corner_left,
     corner_right, merge, start, end — and every consumer (the frame set,
     the item path, the hover line, the tests) asks it rather than
     re-reading `links`. The order of the rule matters and is the plan's:
     a feed from BEHIND makes a tile straight whatever else feeds it, so a
     flank feed on top of one is a merge and not a corner (the stress
     line's flank hoppers, the gallery's ring hopper); exactly one flank
     feed with nothing behind is the corner; two flanks is a merge; then
     no feed is a START and no output an END. A lone tile is a start rather
     than an end because it is the start of the line being laid. A feed
     from the tile the belt points AT (a hopper facing a belt that faces it
     back) is neither behind nor a flank, is drawn straight and is said in
     hover; no model describes it and the sim does link it. Merge, start
     and end all draw the straight family: ticket 2 authored no separate
     models, and nothing on the render asked for one.

  2. **Which flank is "left" was decided by the models, and it agrees with
     the product's own left.** `belt_corner_left` is authored with its
     output on +X and its input on vox low Y, which the importer puts at
     world +Z, which is sim north — the side one quarter turn
     ANTICLOCKWISE from east. Product coming from the north heads south and
     turns to go east: facing south, east is on your left. So left corner =
     fed from `rotate_ccw(direction)`, right corner = fed from
     `rotate_cw(direction)`, and because a Y rotation carries that relation
     with the mesh, **a corner needs no yaw of its own** — every belt tile
     spins by its facing exactly as a straight does. Ticket 4's
     caller-supplied yaw seam is kept (it costs nothing) but unused.

  3. **Items are drawn face to face, and that was a half-tile bug fix, not
     a style change.** `BeltSnapshot` measures an item's distance from the
     tile's START (its input edge) to its END; `main.gd` had been drawing
     distance 0 at the tile's CENTRE and distance 1 at the next tile's
     centre. Every tile shared the offset, so a straight line looked
     continuous — an item just appeared mid-belt instead of at the hopper's
     mouth and vanished at a machine's centre instead of at its face — and
     nobody saw it. A corner cannot share it: the plan's arc runs from the
     input face's midpoint to the output face's, and drawn half a tile
     late it would have left the tile through the wrong face. So the
     straight run is `centre + facing * (distance - 0.5) * TILE`, the arc
     is a quarter circle of radius half a tile about the pivot corner (the
     tile corner the input and output faces share), and
     `test_belt_shapes.gd` pins that a corner's end is the next tile's
     start and the previous tile's end is the corner's start. The arc is
     the same curve the corner model's lane is built on, so items ride the
     rubber. The ticket said "straight tiles unchanged"; they are unchanged
     in shape (a straight line along the facing) and moved half a tile
     upstream in position, which the plan's own arc definition requires.

  4. **The frame is a function of the tick and nothing else.**
     `_belt_frame_for_tick(tick) = floor(tick * BELT_FRAMES /
     BELT_TICKS_PER_TILE) mod BELT_FRAMES`, computed once at the end of
     `_advance_simulation` from `sim.current_tick()`. Nothing is
     accumulated, so the pause menu (which never reaches that line) freezes
     the rubber with no extra state, a loaded save resumes on the right
     frame, and a headless test can drive it with the same function the
     game uses. `BELT_FRAMES` and `BELT_TICKS_PER_TILE` are mirrored as
     constants in `main.gd` with a "change both" note, the same way
     `BELT_SURFACE_VOXELS` mirrors the generator. **At `TICK_HZ` 30 that is
     2.4 frame changes a second**, which a PNG cannot judge; the coordinator
     asked for a judgement in the live scene and this ticket could not make
     one, so the number is recorded here and the frame count is left at
     the plan's four (a change is an `ask`). The "closest frame to the
     exact sub-tile phase" alternative changes when the flips happen, not
     how many there are, so it was not built either.

  5. **The build ghost stays a straight tile.** The alternative — computing
     a would-be shape for the ghost from its neighbours' `links` — means a
     Godot-side copy of `resolve_sink`: which neighbours feed a tile that
     does not exist yet, from which sides, including the inlet-edge rule
     ticket 1 flagged for ticket 7. That is precisely the second adjacency
     scan the `links` field was built to remove, and it would drift from
     the sim's rule the first time the rule changed. The would-be links of
     an unplaced tile are a fact the sim has not computed, and
     ARCHITECTURE.md §4's planned `preview_command` is the mechanism for
     asking it; when that lands the ghost reads its shape from the answer.
     Until then the ghost's job is the tile, the facing (the arrow on the
     straight tile) and whether the click will succeed, and the corner
     appears the instant the tile is placed, since placing rebuilds. The
     REMOVE ghost, which is over a placed tile, does show the real shape.

  6. **The retired names are deleted, not kept as aliases.** `belt`,
     `belt_inlet` and `belt_outlet` are out of `ASSETS`, their nine `.vox`
     files and three icons are removed, `SHOP_MODELS["belt"]` is `belt_f0`
     (which is what the hotbar icon and the ghost already were, by
     content), and `test_connections.gd` asserts the old models are NOT
     staged, so a stale lookup fails loudly instead of drawing frame 0
     under an old name. `voxgen`'s `belt()` function stays, as the
     reference the junction and splitter tests measure against, with a
     test pinning it to frame 0. `_belt_asset_for` and its east-only
     fallback are gone from `main.gd` along with every comment about
     belt junction art being +X only; `_is_building_tile` survives because
     the pipe-role pass still reads it until ticket 6.

  7. **Hover says it in words, from the same `links`.** A belt's tooltip
     is three lines: `belt · carries east · turns left` (the shape word only
     when it is a corner or a merge), `fed from the west (Hopper)` (the
     side always; the neighbour's display name when it is not another
     conveyor, looked up from the placement actually there since a link
     only says `machine`), and `hands to Crusher` / `hands east to the next
     belt` / `not connected`. "Fed by nothing" is said rather than left
     blank: an unfed line start is the commonest reason nothing is moving.

  The gallery (`test_layouts.gd`'s `build_belt_gallery`, `--belt-gallery`)
  is a clockwise ring (four right corners), an anticlockwise ring (four
  left), hopper → belt → crusher → belt → dock at each facing, a two-flank
  merge, a line end and a lone start; about 1,300 of the 3,000 balance.
  A closed ring is a legal placement and runs; fed from a flank hopper it
  fills and jams within the populate run, which is a real state and shows
  items nose to tail round both hands of corner.
- 2026-09-09: **Pipe presentation — "Pipes & Conveyors Overhaul" ticket 6
  (`godot_project/main.gd`, tests, the screenshot script; the deprecated
  `pipe_source`/`pipe_inlet` deleted from `assets/voxels/`).** Seven calls.

  (1) **A pipe tile is composed from `links.connections` and nothing else.**
  The old `_pipe_roles` read the fluid snapshot (`is_source`, tap indices)
  and then GUESSED — "is there a building south of this tile?" — to pick one
  of three one-arrangement models, and fell back to a plain tube for every
  arrangement it could not express, which after the chain rework was most
  of them. Ticket 1 put the exact joins on the wire, so the scene now
  derives the shape (end / straight / corner / tee / cross from how many
  sides are joined and how they lie), the turn, and a fitting per non-pipe
  side from that one field. No adjacency scan of Godot's own, per
  ARCHITECTURE.md §4/§5.

  (2) **The turn is arithmetic on `DIRECTIONS` indices, and the tests prove
  it on the models.** The shapes are authored where ticket 3 put them (an
  end open west, a corner on east and north, a tee closed south) and one
  clockwise quarter turn adds one to every index — the same turn
  `Direction::rotate_cw`, `_direction_yaw` and `VoxModel::rotated_cw` make.
  So "which turn lands the authored openings on the joined sides" is one
  subtraction per shape, and voxgen's `a_corner_turned_twice_opens_west_and_south`
  is the proof that the arithmetic and the models agree.

  (3) **A straight's turn is decided per AXIS, not per tile, and a pipe's
  `direction` decides only a lonely tile's axis.** Ticket 3 kept one joint
  flange per straight, on its low-X end, precisely so a join reads as one
  fitting. That holds only if every tile along a run puts its flange on the
  same side; the old code turned each straight by its own placed
  `direction`, so an east tile next to a west tile stacked two flanges on
  the boundary between them. Now an east-west straight is always turn 0 and
  a north-south one always turn 1, whatever the player pressed R to; the
  placement's facing only matters for a pipe joined to nothing (it lies
  along that axis), which is also the only case where R visibly does
  anything to a pipe. Ends have a dictated turn (their one open side), so
  an end open to the east would stack its flange against the straight east
  of it — `pipe_end` therefore carries no joint flange; its cap is its
  identity and the joint is the neighbour's.

  (4) **Every part of a fitting is a full voxel of radius larger than
  anything it can be overlaid on (`PIPE_FITTING_MARGIN` = 1.0).** Ticket 3
  widened a fitting's bore by 0.4 so the two meshes "never share a
  surface", which was true of the bore and of a fitting on a +X leg. It was
  not true of a fitting on a straight's LOW-X face — the first tile of the
  run leaving the mixer, and every tile a tank discharges into from the
  east — where the fitting lands on the straight's own joint flange (radius
  5.5) with a flange of its own at 6.0. Two voxel solids z-fight wherever a
  voxel of both has an outward neighbour in neither, because both then
  expose a face on the same plane; a margin of one full voxel of radius
  guarantees every axis-neighbour of an inner voxel lies inside the outer
  solid, so no face is shared. The machine fitting's flange went to 6.5;
  the tank fitting already cleared it. `a_fitting_overlaid_on_either_face_of_a_straight_shares_no_outer_surface`
  pins it for both fittings on both faces, exempting only the tile
  boundary plane (what stands beyond it is the render's job).

  (5) **Fittings are instances; fluid bodies stay nodes.** A fitting has no
  live state, so it goes through ticket 4's `_stage_tile_instance` under
  its own key (`fitting:x,y:side`) and costs nothing a tile does not. The
  liquid body is scaled per frame AND tinted per network, and a pool per
  fluid is a node with extra steps, so it stays a node — now one asset per
  shape (`PIPE_FLUID_ASSETS`), at the shape's turn, which is what let
  `FLUID_END_CAP_TRIM` go: each body is authored to reach only its shape's
  open faces.

  (6) **Hover names the joins in words, and "feeds"/"fed by" is read off
  the fluid snapshot, not guessed from geometry.** The wire says "machine,
  north"; the placement standing north says WHICH machine (`_kind_display_name`
  of its kind); whether this pipe feeds it or is fed by it is whether the
  segment index is in the network's `taps` (the machine draws from it) or
  `producers` (the machine deposits into it). A tank always feeds.

  (7) **The ghost previews its shape from the PIPE tiles beside it, and
  nothing else.** Any orthogonally adjacent pipe joins the same network
  (`sim::world`'s pipe-links doc), so that much of a hypothetical placement
  is certain before the sim is asked; which machine or tank it would tap is
  not (a machine taps ONE tile of a run and the sim picks it), so no
  fitting is previewed. With no pipe beside it the ghost lies along the
  chosen facing, exactly as the placed lonely tile will. `preview_command`
  (ARCHITECTURE.md §4) is still the right long-term answer. Decided here
  before ticket 5 chose the belt ghost's rule; flagged for ticket 7 to make
  the two agree.

  Also learned, for fixtures: **a tank is filled from bought stock, and only
  a RAW fluid can be bought**, so a gallery run tapped by a bottler (whose
  input is ketchup) stays empty however many times it is filled; the
  gallery's fed runs are tapped by mixers (vinegar) for that reason.

- 2026-09-09: **Pipes & conveyors overhaul, ticket 2 of 7 (the belt model
  family; `assets/voxels/` only, `godot_project/main.gd` deliberately
  untouched — ticket 5 wires the models in).** The plan
  (`docs/PIPES_AND_CONVEYORS_PLAN.md`) fixed the names, the frame scheme and
  the corner conventions; this ticket built them and made the calls the plan
  left open. Eight calls worth not re-litigating.

  1. **One generator, twelve models.** `belt_shape(p, shape, phase)` with
     `BeltShape::{Straight, Corner(Turn::{Left, Right})}` and `phase` in
     `0..BELT_FRAMES` (4) is the whole family; the named `belt_f0..f3`,
     `belt_corner_left_f0..f3` and `belt_corner_right_f0..f3` entries are a
     macro over it, because `ASSETS` is a table of plain `fn(&Palette)`
     pointers and a closure-per-frame would have changed every other entry
     for nothing. `belt`, `belt_inlet` and `belt_outlet` keep generating,
     byte-identical to `belt_f0` (pinned by a test), so the scene renders
     unchanged until ticket 5 switches names.
  2. **Frames differ only on the lane surface, and the test says so
     exactly.** Frame `k` is frame 0 with every voxel in the lane at or
     above the rubber shifted `k * BELT_FRAME_SHIFT` (8) voxels along +X,
     modulo the tile; everything else — legs, rails, hazard stripes, idlers,
     the return strand — is compared voxel for voxel across frames. The
     idlers do NOT shift: on a real conveyor the belt travels and the
     rollers stay put, and they are under the rubber in any case. The plan's
     wording ("cleat/roller pattern") was read as "the moving surface", and
     the surface is what moves.
  3. **The pattern's period is one tile, so there is one arrow per tile.**
     Four frames at a quarter tile each demand a period of exactly 32
     voxels: the old cleats every 8 would have made all four frames
     identical, a period of 16 makes frames 0 and 2 coincide, and any two
     identical marks 8 or 16 voxels apart give the wagon-wheel illusion (a
     mark that could have moved forward 8 or back 8). One mark a tile moves
     unambiguously forward. It also means the animation is a stepped 4-frame
     march at about 5 frames a second at sim speed — the plan's decision,
     accepted.
  4. **The direction mark is a solid arrow, raised, in `hazard_yellow` —
     decided by the render.** The first cut was a thin two-arm chevron, the
     obvious "direction chevron", and `scripts/screenshot-scene.sh
     --closeup` showed an `L`: under the fixed `(1, 1, 1)` camera a line
     along one tile diagonal is a full-length screen horizontal and a line
     along the other a short screen vertical, so a 45° `>` has no facing at
     which it reads as a `>`. A filled wedge keeps its apex from every
     angle; that is also why the splitter's readout is a filled arrow, and
     the shape family is now consistent. Raised one voxel rather than
     recessed because one edge of any recess lies along the sight line and
     shadows its own floor (the splitter's grooves get away with it because
     their arrow is emissive). The item-overlap cost — an item drawn at the
     surface stands one voxel into the plate — was checked in the populated
     close-up and is a sliver under the item's foot. `hazard_yellow`
     because it is the belt's own signage colour (the rails already wear
     it) and UI_UX §7 lists safety yellow in the base palette, not among the
     reserved status colours; the splitter's stubs carry no arrow at all so
     a matte yellow mark never sits beside its lit amber one. UI_UX §2b
     point 23 records the lesson.
  5. **A corner is a quarter annulus about the pivot corner, centre-line
     radius half a tile.** That makes the lane enter and leave at exactly
     `BELT_LANE` and the curved rails meet a straight tile's rails voxel for
     voxel at both open faces (tested), so a corner between two straights
     is one continuous line. `Turn::Left` is fed from the plan's north face,
     which is vox LOW Y (sim +Y is world +Z is vox -Y, the junction header's
     rule); `Turn::Right` is built by mirroring the finished left model in
     Y, never authored a second time. The tile's outer corner and its pivot
     corner are bare floor by design — a curved belt on a square tile has
     both.
  6. **A corner's surface pattern lives in the same `(u, v)` coordinates as
     a straight's, with `u` stretched over the quarter turn.** The arc is
     25 voxels long on the centre line but the sim crosses it in the same 50
     ticks as a 32-voxel straight and ticket 5's items follow the same 0..1
     parameter, so the pattern is defined per tile-fraction, not per voxel:
     the arrow reaches the exit face in the same frame a straight tile's
     would, and the surface and the items on it stay in step. Length along
     the path is in `u`, width in voxels, so the arrow is short on the
     inside of the bend and long on the outside — it bends with the belt.
     The corner frame test therefore checks structure equality plus the
     circular mean of the arrow's centre-line `u` advancing 8 per frame,
     not an exact voxel shift, which a 22.5° rotation cannot be.
  7. **The belt-side junction variants are retired.** `belt_inlet` and
     `belt_outlet` dressed the last tile before a machine with a static
     steel dead-plate and kerbs. Three reasons, any one sufficient: a static
     plate on an animated surface either freezes the pattern short of the
     machine (a belt that visibly stops early) or has to be carved out of
     the frame rule; point 18 already fell back to the plain tile on three
     facings in four, so the shipped game has shown the plain tile at most
     junctions with no complaint; and with corners a corner can be the
     tile that meets a machine, which multiplies the variant set by three.
     The machine-side pieces (`machine_infeed`'s hood with its brass
     threshold, `machine_outfeed`'s doorway and lip) already bring the
     junction to the tile boundary at `BELT_SURFACE_Z`, and the close-up of
     the crusher's hood with a plain arrowed tile running into it confirmed
     that they carry it. The names live on as aliases until ticket 5.
  8. **Icons only for frame 0.** `voxicons` skips any `name_fN` with N ≥ 1:
     an icon is a catalogue picture of a kind, and three near-identical
     PNGs per shape would be committed for nothing to load.

  Verification: `cargo test -p voxgen` grew from 9 to 15 tests (arrow on
  every frame and pointing along the path; exact shift for straights;
  structure + quarter-pattern advance for corners; a corner open on exactly
  its two connected faces with a straight's rails there; every belt face
  open above the lane; the retired aliases identical to `belt_f0`; the
  splitter's deck at the belt's height with plain stubs). The scene was
  rendered with `--build --populate --no-hud --closeup` for straights at
  three facings with product on them, and once more with a temporary,
  uncommitted `main.gd` hack drawing corner models in place of belts, which
  is the only way to see a corner in the scene before ticket 5. **Flagged
  for ticket 5**: re-check corners at all four facings once the scene draws
  them for real, and the south facing of everything (the demo fixture has
  none).
- 2026-09-09: **Pipe model family — "Pipes & Conveyors Overhaul" ticket 3
  (`assets/voxels/` only; `godot_project/main.gd` deliberately untouched,
  ticket 6 wires it).** The ticket began by rendering every pipe run in the
  chain, fluids and demo fixtures and listing what was wrong (the DEVLOG
  entry has the seven defects). Six calls worth not re-litigating.

  (1) **The pipe axis is on the tile MIDLINE, between voxels 15 and 16, not
  on voxel 16.** Every earlier pipe asset was centred on voxel 16 because
  the integer-centred cylinder primitives could not do otherwise, and it was
  invisible while every tile faced east. Godot spins a placement about its
  footprint centre, so an east tile's axis sat at y = 16, a west tile's at
  y = 15, a south tile's at x = 15 and a north tile's at x = 16 — a run at
  mixed facings stepped by one voxel at every joint, and a corner could never
  have met both of its neighbours. `vox.rs` gained fractional-centre
  primitives (`cylinder_x_f` and friends, centre in voxel-index units) and
  every shape is built on `PIPE_C` = 15.5. A side effect is a rounder tube:
  the old profile had three nine-wide rows and read as a square beam at
  scene scale. The machines' own nozzles (`machine_fluid_port`, the tank
  spout) still sit on voxel 16 — a radius-3 stub half a voxel off inside a
  six-wide bore is not a visible step, and moving them is machine art
  outside this ticket.

  (2) **The fluid body is NARROWER than the bore (14..17 in a 13..18 bore).**
  The most visible pipe bug in a rendered factory was a coloured stripe
  along the top of every closed tile: the slab's side faces coincided with
  the tube's OUTER wall where it curves in toward the crown, and the two
  z-fought. Two voxels in from each side keeps every face of a full column
  inside wall material except where the window has genuinely been carved.
  `main.gd`'s PIPE_BORE_FLOOR_VOXELS (12) and PIPE_BORE_HEIGHT_VOXELS (7)
  are unchanged; each body reaches every open face and stops short of a cap,
  so the Godot-side `FLUID_END_CAP_TRIM` is no longer needed.

  (3) **Face convention: vox low Y is north.** The plan's corner conventions
  name the corner's second face "+Y (north)" in TILE space; the vox importer
  maps tile north (world +Z) from vox LOW Y (the junction header's
  derivation, and the mixer's stub reaching its north neighbour through
  y = 0). So `pipe_corner` is open on x = 31 and y = 0, `pipe_tee` on x = 31,
  x = 0 and y = 0, `pipe_end` on x = 0 only. Both of a corner's openings are
  on camera-visible faces (§2b point 11). Stated once in the pipe section's
  header; `VoxModel::rotated_cw` applies the same `(dx, dy) -> (-dy, dx)`
  turn as `Direction::rotate_cw`, and a test proves a corner turned twice
  opens west and south.

  (4) **Fittings are overlays authored on +X, and hardware at tube height
  stays at x ≥ 27 (`PIPE_FITTING_X0`).** A fitting is the pipe's half of a
  joint with something that is not a pipe; Godot rotates one onto whichever
  face touches a machine or a tank, on ANY shape. That composability is why
  the junction shapes' window sits at 20..26 on the +X leg and the fittings
  never reach it; the tank valve's hand wheel, which stands well above the
  tube, may reach x = 25 because a sight line out of a window rises one
  voxel per voxel of travel and cannot meet it. A fitting's bore is carved
  0.4 wider than the shape's so an overlay's inner faces sit inside the
  shape's wall rather than coinciding with its bore surface. The machine
  fitting is a radius-6 brass flange on the boundary — one radius larger
  than the tube — because a plain tube end butting the machine's radius-4
  flange showed its own cut face around the smaller ring. The tank fitting
  is the heaviest flange on the line with the isolation valve standing ON
  it (a flanged valve bolted to a tank outlet is how a real tank is
  plumbed), the bonnet narrow in Y so a leg window's sight line passes it,
  and in `steel` rather than `steel_dark` because it stands in the tank's
  shadow.

  (5) **Junction shapes carry a cast hub and no end flange; the straight
  keeps one flange at low X; a cap is a bright bolted plate.** A flange on a
  corner's leg would stack against a straight neighbour's own flange half
  the time (a straight facing INTO a joint puts its flange on that boundary);
  the hub sphere is what says "fitting". The old cap put a brass disc in the
  middle of the plate and the render showed a black hole — the +X face is
  camera-facing but on the shadow side, and brass there reads near-black.
  Small brass bolt heads on a pale plate keep the accent without giving the
  eye a dark circle to read as a bore.

  (6) **`pipe_source` and `pipe_inlet` are deprecated, composed from the
  family, and kept generating.** They exist so the scene renders before
  ticket 6 and so both fittings could be reviewed in a REAL render now
  (`VoxModel::merge` + `rotated_cw`): the tank fitting at the tank, the
  machine fitting on the branch to the mixer. `pipe_inlet` is still wrong
  at every facing but one — that is the defect the family exists to remove,
  and `main.gd` already falls back around it. Voxgen tests pin: each shape
  open exactly on its declared faces (bore empty AND wall present on the
  boundary), each fluid body inside its bore and reaching only open faces,
  a genuinely empty window on every shape, and each fitting's flange on the
  boundary at `PIPE_AXIS_Z` with an open bore.
- 2026-09-09: **Pipes & conveyors overhaul, ticket 4 — instanced rendering
  for belts, pipes and items (`godot_project/` only; the sim and the assets
  deliberately untouched).** `_rebuild_world` used to free and rebuild one
  `Node3D` (with a `MeshInstance3D` per material class) per placement, and
  `_render_carried_items` kept one pooled `Node3D` per belt slot and moved it
  every tick. At the scale the overhaul is heading for — a few thousand
  conveyor tiles, animated — node count is the cost, and it is paid twice: on
  every place/remove/rotate (a full rebuild, by design) and every tick (a
  transform write per item node, each of which notifies its mesh children).
  Six calls.

  (1) **`MultiMesh` per (asset, material class), the same shape
  `_build_floor` already used.** A voxel asset is one mesh per material class
  (UI_UX.md §2b point 2), so an instanced tile is one `MultiMeshInstance3D`
  per class with the class material as `material_override`, exactly what the
  node had as a surface override. Lighting, shadows and SSAO are identical —
  `scripts/screenshot-scene.sh --populate` before and after differed only on
  the carried items and one clipboard digit, and the digit said why: the two
  runs were one simulation tick apart (the screenshot ticked by wall clock
  then, so two runs of the OLD renderer differed from each other the same
  way; #82 pinned that clock on 2026-09-25). With
  `--no-hud` on the splitter fixture the two PNGs were byte-identical.

  (2) **What stays a node is a rule, not a list: anything with live
  per-instance state.** A machine's status lamp swaps material; a hopper's,
  tank's and pipe segment's level body scales and retints; the splitter's
  arrow turns and hides; the dock's cases change model; the ghost is tinted.
  A `MultiMesh` instance has a transform and nothing else, so each of those
  would need a pool per material or a colour channel with its own design —
  and they are tens of things, not thousands. Pipe fluid bodies in
  particular stay nodes even though pipes are instanced: ticket 6 redraws
  them per shape and is the right place to decide whether the tint becomes a
  per-instance colour.

  (3) **Three seams built now for tickets 5 and 6, so animation and corners
  land ON this rather than beside it.** (a) A tile's asset is a FRAME SET:
  `_tile_frames_for` returns a list of asset names (one frame each), every
  frame gets its own per-class MultiMeshes with the SAME transforms, and
  `_set_tile_frame(k)` flips visibility — so the plan's belt animation costs
  a handful of visibility flips per frame change for any number of belts.
  Until ticket 5 every set has one frame. (b) `_stage_tile_instance` takes a
  position and a YAW from the caller; `_build_placement` passes the
  placement's direction yaw today, and a corner tile can pass something
  else. (c) `_carried_item_transform(tile, direction, distance)` is the only
  place an item's position is computed; the corner arc goes there with the
  same `distance` in. The basis is identity, as items never were rotated.

  (4) **A `MultiMesh` needs its count up front, so tiles are STAGED during
  the rebuild and built together after the placement loop.** Staged order is
  placement order (the sim's own tile order), so two rebuilds of one world
  produce the same instance indices — determinism for free.

  (5) **Items are a swap-remove pool per asset with a dense visible prefix**
  (`visible_instance_count`), not a hidden-by-zero-scale instance: hiding is
  one transform write plus a count. Growth doubles capacity and replays a
  mirror of the transforms, because setting `instance_count` clears a
  MultiMesh's buffer. The per-TILE hiding rule from the splitter ticket is
  kept verbatim (`_hide_carried_item` returns false at the first gap).

  (6) **Measure, and say what the number is.** `build_stress_line` is a
  1500-belt serpentine with hoppers on its free flanks and three tank-fed,
  mixer-tapped pipe runs. Its bill (~8,500) is past `STARTING_MONEY`, and
  the sim is off limits to this ticket, so `fund` sets the balance through
  the one door Godot has — `save_ron`, patch `money_bits`, `load_ron` —
  with `Fx`'s ~32 767 ceiling. Results: rendered objects in the stress frame
  7,772 → 910, draw calls 204 → 174, headless warm rebuild 57 → 38 ms. The
  per-frame headless figure (7.2 ms) did not move, and profiling says why:
  ~8 ms of every tick step is `JSON.parse_string` on ~80 KB of chunk
  snapshot for two dirty chunks, because the wire reports every belt in a
  dirty chunk every tick (ARCHITECTURE.md §4's per-chunk delta). That is a
  wire-format matter for milestone 8 and is recorded, not worked around.
  `test_perf.gd` asserts generous ceilings (an order-of-magnitude
  regression, not a ten-percent one) and that drawn items equal the sim's
  count; `screenshot-scene.sh --stress` prints the renderer's own
  object/primitive/draw-call counts so the GPU side can be compared without
  a profiler. Primitives rose ~15% on the stress shot: a MultiMesh is culled
  as one whole, which at a camera that frames the whole world is nothing.
- 2026-09-09: **Pipes & conveyors overhaul, ticket 1 of 7 (sim only) —
  connectivity goes on the wire as `links`, recorded by the topology pass
  and never derived by Godot.** The overhaul's tickets 5 and 6 draw a belt
  as a straight, a corner or a merge and a pipe as an end, a straight, a
  corner, a tee or a cross, with port fittings where a machine or a tank
  joins. Every one of those is a fact the sim already decides in
  `World::recompute_topology` — which belt hands to which, which pipe tile
  a machine taps — and the plan's ground rule is that Godot never derives a
  fact the sim already knows. So the sim says it. Seven calls.

  (1) **`links` is filled inside the topology pass, at the moment each
  relation is resolved, and stored on `Topology` (`links: BTreeMap<Entity,
  Links>`) — not computed on read from the relation lists.** Deriving it in
  `world_layout` by searching `belt_to_belt`, `belt_to_sink`,
  `source_to_belt`, `splitter_outputs`, the networks' segment links and the
  three tap lists for every placement would be a second pass over the same
  facts (quadratic at milestone 8's scale, and `world_layout` runs on every
  place/remove/rotate), and it would be a second place the linking rule
  lives. Written where the sim resolves a hand-off, the wire cannot disagree
  with the tick, and it costs nothing per tick because nothing per tick
  touches it. `world_layout` and `query_tile` look it up by entity.

  (2) **Every belt-like entity and every pipe has an entry, empty or not;
  no other kind has one, and on those the key is absent rather than
  `null`.** A lonely belt reports `{"inputs": [], "output": null}` and a
  lonely pipe `{"connections": []}`, so a consumer never special-cases a
  missing object on a kind that is supposed to carry one. Everything else
  serializes with `#[serde(skip_serializing_if = "Option::is_none")]`, so a
  machine's, hopper's, tank's or dock's JSON is byte-for-byte what it was
  before the field existed — pinned by a test that asserts the substring
  `links` does not appear.

  (3) **Entries are sorted by side — north, east, south, west,
  `Direction`'s own order — at the end of the pass.** At most one thing can
  stand on each side of a tile, so this is a total order and a property of
  the geometry rather than of which placement the pass happened to walk
  first. That is what lets `tests/links.rs` assert a merge built in two
  placement orders reads identically and pin the whole JSON byte for byte.

  (4) **A splitter's `output` is its straight side only.** The branch is
  already on the wire as `outputs[1]`; saying it twice in two shapes is two
  places to disagree. The receiving belts on both sides list the splitter
  as an input, since that is what feeds them.

  (5) **A pipe lists a machine or a tank on the one tile the sim actually
  taps, not on every tile that touches the machine.** A machine draws from
  one segment and deposits into one (`machine_taps`,
  `machine_output_taps`); a tank discharges into one (`tank_discharges`).
  A run laid along a machine's whole side therefore gets one port fitting,
  on the tile fluid actually crosses. Drawing a fitting on every touching
  tile would be art asserting a connection the sim does not make —
  exactly the kind of Godot-derived fact the overhaul forbids.

  (6) **`Links` is an untagged enum of two structs.** The two shapes share
  no key, and the consumer already knows which it is reading from the
  placement's `kind`, so a tag would be noise on the wire the plan does
  not specify.

  (7) **A sim rule was found that the docs contradict, and it was
  deliberately left alone.** `resolve_sink` links a belt into a machine or
  dock when *the tile it points at* is on the inlet edge — from whichever
  side the belt approaches. For a 1x1 machine every side is that edge, so
  a crusher is fed from all four sides, including through its outlet face;
  for the 1x2 mixer facing east both tiles are on both edges. `world.rs`'s
  module doc says "a belt pointing into a machine's flank does nothing";
  the code checks the tile, not the approach face. The fix is one
  condition (`belt.direction == sink.direction`, equivalently "the belt
  stands on an inlet neighbour"), but it moves product: a placed line that
  works today would stop, `godot_project/test_layouts.gd`'s demo line —
  the fixture every screenshot and most headless proofs build — feeds a
  west-facing crusher from the south and would break, and a save of such a
  line would load and then stall. This ticket adds a readout and must not
  change behaviour, so the wire reports the rule as it stands (a belt
  pointing sideways into a crusher shows `output: {kind: machine}`, which
  is true), `tests/links.rs` pins it in a test named for what it does, and
  the decision is ticket 7's. Note the asymmetry: the *input* side is
  already face-correct (`source_to_belt` uses `outlet_neighbours`), so a
  belt that both points at and stands off a 1x1 machine reads as fed by
  and feeding the same machine.

  **Topology gaps that had to be filled**, all in the same pass: the
  machine *input* tap kept only `(network, segment)` and was found with a
  `find` over footprint tiles × directions that discarded which direction
  hit — the direction and tile are now captured; the machine *output* tap
  and the tank discharge likewise kept only `(network, segment)` and now
  also yield the pipe tile; pipe-to-pipe adjacency existed only as `(i, j)`
  segment indices walked east and north, and is now recorded from both
  ends as it is found. Hopper→belt and machine→belt (`source_to_belt`),
  belt→machine/dock (`belt_to_sink`) and belt→belt were already recorded
  with enough to name the side. Nothing that existed changed shape;
  `cargo test --workspace` and every headless Godot proof pass unchanged.

- 2026-09-09: **Hopper/tank auto-fill, ticket 1 of 2 (the sim half;
  `godot_project/`, `data/` and `assets/voxels/` deliberately untouched) — a
  bin can keep itself full, and the keyboard stops being the feed
  mechanism.** Until this ticket, keeping a factory stocked was two global
  keybinds: **SPACE** (`FillHopper` with `tile: None`) filled every hopper in
  the world and **V** every tank, so an unattended line ran for exactly one
  hopper-load and then stopped. Ticket 2 replaces both with a per-building
  panel; this is the mechanics that panel drives. Six calls worth not
  re-litigating.

  (1) **Auto-fill runs FIRST in the tick — a new step 0, before the tank
  discharge and long before hoppers meter onto belts — and that placement is
  the mechanic rather than an ordering detail.** A topped-up tank therefore
  discharges into its run in the same tick it was filled, and a topped-up
  hopper hands a unit to its belt in the same tick. Running it last is the
  obvious alternative and is worse in a way that is invisible in a test and
  visible in a rendered factory: a hopper that emptied on tick *n* would feed
  nothing on tick *n*, so an auto-filled line would hiccup for one tick every
  time a bin bottomed out, for no reason a player could see or fix. The
  observable consequence is pinned rather than described —
  `tests/auto_fill.rs` asserts a draining hopper reads at or above
  `capacity - 1` at the end of **every** tick for four hundred ticks.

  (2) **To capacity, out of ONE shared fill helper per kind.** The tick path
  and the manual `FillHopper`/`FillFluidTank` arms both go through
  `World::fill_hopper_entity` / `fill_tank_entity`, so there is one fill rule
  per kind in `world.rs` rather than two that can drift: **two clamps in this
  order** — what the player owns, then what the buffer has room for — and the
  ledger debited by **exactly what `fill` reported it added**, never by what
  was asked for. That rule already existed and was already subtle (a
  nearly-full bin accepts less than it is offered); duplicating it into a
  per-tick path was the one thing most likely to leak stock a thousand ticks
  later.

  (3) **It spends STOCK, never MONEY** — the call the whole feature turns on.
  `BuyIngredient` stays an explicit player action, so auto-fill can only ever
  move units that were already paid for out of the larder and into a bin.
  Wiring it to buy what it needs would be one line and would delete
  GAMEPLAY.md §5's first cost axis as a decision: the recurring ingredient
  cost is supposed to be one of the three things the player is choosing
  between, and a toggle that quietly debits the balance turns it into a
  background leak the player cannot see, cannot budget for, and would only
  discover as an unexplained bankruptcy. When the larder runs dry the
  top-ups simply stop — **no error, and the flag is deliberately not flipped
  off**, so they resume by themselves the moment stock is bought again
  (turning it off would mean the player has to remember to re-arm every bin
  after every shortage, which is the chore the feature exists to remove).

  (4) **A tank asks only for its WHOLE-UNIT room, where the manual command
  bills a part unit as a whole one.** A tank's room is continuous while stock
  is bought and debited in whole units, so `FillFluidTank` has always rounded
  a fractional fill up. That is correct for one deliberate keypress and
  ruinous sixty times a second: a tank trickling a fraction of a unit per
  tick would be billed a whole unit per tick, and the larder would drain
  orders of magnitude faster than the factory consumed. `FluidTank::
  whole_unit_room` is the floor of its room, so an automatic top-up is billed
  for exactly what it added, always, and a tank with less than a unit of room
  is left alone until it has drained enough to take one. This is not asserted
  in prose: **vinegar is in `tests/auto_fill.rs`'s mass-conservation walk**
  precisely because it is the one raw ingredient with a continuous buffer, and
  a ceiling-rounded top-up makes that equality drift within a handful of
  ticks.

  (5) **A hopper's top-up dirties its chunk; a tank's does not**, and that
  asymmetry is inherited rather than invented. A hopper's level has always
  been a chunk-visible fact (`hopper_snapshots`' own doc says a chunk-driven
  renderer stays correct), and a hopper being topped up is a hopper feeding a
  belt, which dirties its chunk anyway — so nothing new is spent. A tank's
  level is already documented as poll-only, because the discharge pass that
  moves fluid every tick deliberately dirties nothing (ARCHITECTURE.md §4's
  delta granularity); an automatic top-up runs on exactly that cadence, so
  dirtying on it would flag a chunk every tick for a fact no chunk consumer
  reads. The discrete player command still dirties, as it always did.

  (6) **`SetHopperItem` is added and `CycleHopperItem` is kept, on purpose,
  for exactly one ticket.** A picker that already names an ingredient should
  not have to press "next" until it arrives there, so the panel gets an
  explicit form — refusing a non-raw item against the same `World::raw_items`
  set `Place` validates against, and treating "the item it already holds" as
  a **no-op that keeps the contents**, because a picker redraws its own
  current selection and re-selecting must not empty the bin. Cycling is not
  deleted here only because `main.gd` still binds **Q** to it and removing it
  would leave `main` red between the two halves of this epic; its doc comment
  says so and ticket 2 deletes both together. This is the same discipline the
  delivery dock followed with `CollectOutput` — do the rewrite, do not
  accumulate two ways to do one thing — with the deletion deferred by exactly
  one ticket rather than skipped. **The flag survives a retarget**, which is
  the small call inside this one: "keep this bin full" is a statement about
  the bin, not about what is in it, so a player who switches a hopper from
  tomato to sugar still wants it kept full, now of sugar. It survives a
  `Rotate` too (rotation is pure geometry and never reaches into a body) and
  is destroyed by a `Remove` along with everything else the body held, so a
  re-placed hopper starts manual. **`schema_version` stayed at 1**, exactly as
  the save ticket's own forward-compatibility note reserved: one
  `#[serde(default)] auto_fill: bool` on each of `SavedHopper` and
  `SavedTank`, and nothing has shipped with a save file yet.

- 2026-09-09: **Save/load, ticket 2 of 2 (the Godot half; `sim_core/`,
  `data/` and `assets/voxels/` deliberately untouched — every capability
  drawn on here was already on the boundary after ticket 1) — the game opens
  on a MAIN MENU, and there is a way back out of it.** Ticket 1 could write
  a world to text and put it back; nothing could reach either, because
  `project.godot` booted straight into `main.tscn` and the only way out of a
  session was the window's close button. Eight calls worth not
  re-litigating.

  (1) **An AUTOLOAD carries the choice between the two scenes, and what it
  is pending is STICKY rather than consumed.** `change_scene_to_file` frees
  the old scene before the new one's `_ready` runs, so there is nowhere in
  either scene to hand "the player picked Load Game, from this file" across;
  `game_session.gd` (registered as `GameSession`) lives outside both, which
  is Godot's own answer to exactly this. Reading `pending_action` does not
  clear it, for a reason that is not academic: **every headless test in this
  project calls `_ready()` a second time by hand** (`add_child` fires it
  once, the test fires it again), and a consumed action would silently
  revert a loaded world to a new game on the second pass. Every transition
  into `main.tscn` arms the action first, so nothing is ambiguous, and the
  default is `new_game` — which is what makes the eighteen tests that
  instantiate `main.tscn` directly and never touch the singleton behave
  exactly as they did before this ticket.

  (2) **Everything about save FILES lives in that same file, not split
  between the two screens.** `sim_core` produces and consumes the RON text
  and nothing else (ticket 1's call 7), so the directory, the listing, the
  sanitizer and the reserved-name rule are Godot's — and they are one set of
  static functions both scenes call, because the screen that writes a save
  and the screen that loads one must not be able to disagree about where a
  save lives or what it may be called. Saves are
  `user://saves/<sanitized-slot>.ron`, and `main.gd` prints
  `ProjectSettings.globalize_path` of that directory once per launch, since
  a developer looking for the files should not have to know Godot's
  per-platform `user://` layout.

  (3) **A save is proved to LOAD before the scene changes.** Selecting a
  slot reads the file and runs it through a throwaway `SimBridge.load_ron`
  — which builds a whole new world and only swaps it in once every step
  succeeded, so the probe costs a load and answers the question — and
  `load_ron`'s own message is shown verbatim beside the list on failure.
  Discovering a bad file inside `main.tscn` instead would put the message on
  a screen with no list, no Back button and the previous world already gone,
  which is precisely the "silently starts a fresh world" failure this ticket
  was told not to have. `main.gd` still checks the load again defensively
  (the file can be deleted between the two moments) and prints rather than
  failing silently, but the normal case is answered where the player can act
  on it.

  (4) **ESC gained a second meaning rather than the pause menu gaining a
  key.** ESC already closed whatever was open; it now opens the pause menu
  when nothing is. That reads as one escalating rule — ESC backs out of the
  thing you are in, and when you are in nothing it backs out of the game —
  it is Minecraft's convention, and it cost no new letter on a legend that
  already runs the width of the window. **Build mode counts as "open"**
  (`_anything_open`), so putting a ghost down is the first press and the
  menu is the second: a ghost following the cursor is a state the player is
  in.

  (5) **The simulation genuinely stops, and the stop is on the ACCUMULATOR
  as well as the tick loop.** `MAX_TICKS_PER_FRAME` caps how far one frame
  can catch up, but an accumulator that kept filling through a five-minute
  pause would still be paid back over the frames after it — so `_tick_accum`
  is cleared while the panel is up and again on the way out, and the
  autosave clock does not run either. The tick loop moved out of `_process`
  into `_advance_simulation` for one honest reason: `_process` needs a
  viewport for everything else it does, and a headless test asserting "ten
  seconds of paused time is zero ticks, and the next frame is exactly one"
  needs the half that does not.

  (6) **Autosave owns one slot and only that slot, in both directions.**
  `autosave.ron` on a three-minute real-time timer (Factorio's own order of
  magnitude — long enough never to be noticed, short enough that a crash
  costs minutes rather than an evening) and on quit-to-menu, quit-game and
  the window's close button. A manual save is **refused** the name
  `autosave`, out loud, rather than quietly renamed: a save that went
  somewhere other than where it was asked to go is worse than one that did
  not happen, and the thing being protected is that a timer must never be
  able to overwrite a save the player deliberately made. It is listed like
  any other save and labelled as the autosave, because falling back on it
  is legitimate and being surprised by it is not. Real seconds, not ticks,
  because what it protects against is the process going away and the process
  goes away in real time.

  (7) **`auto_accept_quit` is turned OFF, deliberately, and that is the
  riskiest line in the ticket.** Godot's default close handling quits
  immediately, which would take the last few minutes of building with it, so
  `main.gd` intercepts `NOTIFICATION_WM_CLOSE_REQUEST`, autosaves and quits
  itself. The failure mode of getting that wrong is an unclosable window, so
  it is verified two ways rather than reasoned about: a headless assertion
  that the notification writes the autosave, and a real windowed run that
  propagates the notification the way the OS does and confirms the process
  shuts down.

  (8) **A typed save name is player text and a file name is not, so the
  sanitizer is a WHITELIST.** Letters, digits, space, `-` and `_` survive
  and everything else becomes `_`; `../../etc/passwd` becomes
  `.._.._etc_passwd`, which is a file name in the saves directory and
  nothing else. A blacklist has to be right about every bad sequence; this
  has to be right about the good ones, of which there are five. A name with
  nothing usable left is a refusal rather than an invented name. **Two
  things came off a rendered frame and not off the code** (UI_UX.md §2b
  point 12, again): the whole menu was drawn in the BOTTOM-RIGHT CORNER,
  because `PRESET_CENTER` is computed from a control's minimum size at the
  moment it is applied and the labels inside it had no text yet — it is a
  `CenterContainer` now, which re-centres whenever its child resizes; and
  the backdrop's hazard band had no edge treatment, so it read as a
  rendering seam rather than as paint on a floor. **Flagged, not fixed:**
  the save list has no delete and no overwrite confirmation — saving over an
  existing slot just does it, which is what a name field implies but is
  worth a confirm step the day slots get long-lived.

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

---

## Part B — UI_UX.md §2b, full text ("How the pipeline actually turned out")

Point numbers are the ones code comments cite (`UI_UX.md §2b point N`).
There is no point 10 — it was skipped in the original numbering — and the
list runs 1-28, 36-37, 29-34: the factory-floor entries were written as a
second 27 and a second 28, and took the next free numbers on 2026-09-16
rather than renumber anything already cited. (UI_UX.md §2b also has a 35, the
icon-pass camera rule, which has no long form here.)

Everything above held up. Four things are worth writing down because they
were only discoverable by building it:

1. **No Rust crate can write the `.vox` files this pipeline needs**, so
   `generators/vox.rs` contains a small hand-written encoder — the fallback
   §2a already anticipated. `vox-format` is still parser-only; `vox_writer`
   0.1.9 does write `SIZE`/`XYZI`/`RGBA` but emits **no scene-graph chunks**,
   and the Godot importer below cannot read a file without them. The format
   is simple enough that encoding it directly is ~150 lines with no
   dependency risk, and it also lets us emit `MATL` material chunks.
2. **One asset becomes one mesh per material class.** A voxel mesh imported
   into Godot carries vertex colours but a single material, which cannot
   satisfy §4's rule that steel is *noticeably* shinier than concrete. So
   `voxgen` writes both a combined `name.vox` (for previews and for a human
   opening it in an editor) and `name.<class>.vox` per material class the
   asset uses; Godot rebuilds one `StandardMaterial3D` per class from
   `palette.ron`'s own numbers and reassembles the asset as a node with one
   `MeshInstance3D` per class.
3. **Reviewing a model headlessly works, with one wrinkle**:
   `vengi-thumbnailer -i model.vox -o out.png -s 512 --image --isometric` is
   the incantation. `vengi-voxconvert -o out.png` is *not* — that writes one
   PNG per Z-slice of the volume, which is not a thumbnail. And the
   thumbnailer renders through OpenGL, so forcing `SDL_VIDEODRIVER=dummy`
   makes it fail; it needs a real video driver even though nothing is
   displayed. `scripts/build-voxels.sh` wraps all of this.
4. **Godot must run `--import` twice on a cold cache.** The `.vox` importer
   is an editor *plugin*, and a plugin is only registered on the pass after
   the one that first scans the project — so the first import silently skips
   every `.vox` and the second imports them all. This bites any CI setup that
   assumes one import pass is enough.
5. **A liquid is its own material class, not a recoloured matte one
   (added 2026-09-07 with milestone 6's pipes).** `palette.ron`'s
   `materials` table gained a fourth class, `liquid` — a dielectric like
   matte, but far glossier (roughness 0.14) — because §4's "visible liquid
   surfaces in tanks/pipes with subtle shading/reflection" is a *material*
   property, and point (4) above already settled that material class is
   palette data rather than a per-asset decision made in GDScript. What is
   NOT in the palette is the fluid's hue: which liquid is in a pipe is live
   simulation data, so the fluid model's voxels are near-white and
   `main.gd` multiplies a per-fluid albedo tint over them (the same shape as
   its existing item-id → model mapping). One consequence worth knowing:
   the fluid model is deliberately **one voxel tall** and scaled vertically
   to the segment's live fill fraction — the encoder puts a model's origin
   at its footprint centre *at ground level* (see §2b point 1's encoder), so
   a one-voxel slab's base is exactly its node origin and a fill level needs
   no pivot arithmetic.
6. **The pipe run is stainless steel, and it is modelled hollow with an
   open inspection channel.** Steel rather than PVC because §3 names both
   but §4 makes metals the deliberate shiny exception so machinery pops —
   and because GAMEPLAY.md §2/§4 reserve non-standard piping material as a
   *later* progression gate (tier 5's acid-resistant equipment), which the
   baseline run shouldn't pre-empt. Hollow because the liquid has to be
   *seen*: the first cut opened only the top half of the wall, and a
   rendered-scene review showed that hid the level below roughly half fill
   — exactly the state a player most needs to read (a draining pipe means a
   stalling machine). The opening now starts just above the bore floor,
   leaving side rails standing.
7. **UI icons (§5) turned out to need a second, separate render pass from
   the review previews above — not a smaller copy of them (built
   2026-09-07).** The obvious-sounding shortcut was "reuse
   `previews/*.png`, just smaller": checking the actual files first (per
   this section's own norm of verifying rather than assuming) ruled that
   out. `vengi-thumbnailer`'s `--image` mode always fills the whole frame
   with an opaque solid colour — confirmed by reading its `--jsonconfig`
   cvar dump (no background/alpha/clear-colour cvar exists at all) and by
   rendering through both `--image --isometric` and its default
   camera-based path and getting an identically opaque result either way.
   A preview is also deliberately padded, full-frame, and large (512px) —
   right for "look at the whole model," wrong for "sit in an 18px UI slot
   next to a line of text." So icon generation is a second binary,
   `assets/voxels/generators/icons.rs` (`cargo run -p voxgen --bin
   voxicons`), that calls the same `vengi-thumbnailer` and then does the
   work the tool itself has no flag for: flood-fill the background to
   transparent from the image border inward (not a flat colour-key, so a
   future asset using a near-black *interior* colour can't be falsely
   keyed), crop to content, and downscale onto a small (64px) square
   canvas. This is still "extend the existing offline pipeline" (§2a option
   (a), not the live in-engine `SubViewport` alternative that section also
   named) — it reuses the same external tool and the same `cargo xtask`
   staging pattern as `models/`, just as a distinct output (`assets/voxels/
   icons/`, committed) rather than a rename of `previews/`. First concrete
   use: `godot_project/main.gd`'s `_icon_for`, showing the contract's
   target item's icon next to its name in the economy readout line.

8. **A machine's material split follows "food contact is stainless,
   structure is painted or poured" (the "Machines improvement" milestone's
   ticket 4).** The first four assets with a real machine identity — a
   crusher, a mixer, a 2x2 cooking vat and a bulk hopper — needed a rule for
   §4's metal exception that wasn't decided per asset, and this is it:
   anything the product actually touches (crushing rolls, mixing vessel and
   agitator, the vat's body and dome, the hopper's funnel and chute) is the
   `metal` class, because in a real food plant it is stainless for exactly
   that reason; anything that only holds those parts up (concrete plinths,
   painted housings and skirts, hazard striping) is `matte`. The point is not
   that steel is prettier — it is that an all-steel machine reads as flat as
   an all-concrete one, so the matte structure is what gives the metal
   something to pop against. Emissive stays where it already was: status
   lamps only (§7).
9. **Review the rendered asset, not the generator — and expect to rework
   it.** §2a point 3 says to render and look; what building four machines
   added is how often looking *changes the design*, not just catches a bug.
   The crusher's rolls were originally sunk in a throat too deep for the
   fixed isometric angle to see into (the whole reason to expose a crushing
   mechanism is that it can be seen); the mixer's top-mounted gearbox — the
   accurate choice — covered the agitator, so it moved to a side bracket with
   a belt drive, which is also accurate and also legible; the cooking vat's
   first proportions read as a sphere on a plinth. None of those are visible
   in the generator source. Budget a rework pass per asset rather than
   treating the first render as verification.
11. **Only two of a machine's four vertical faces can ever be seen, and
    that decides how a junction is drawn (the "Visual overhaul round 2"
    epic's ticket 3).** §1's camera is fixed at a `(1, 1, 1)` view
    direction, so the only vertical faces with any pixels on screen are the
    ones facing +X and +Z-world. The production chain runs +X, which means a
    machine's OUTLET face is dead-on camera-facing and its INLET face points
    directly away — carving an inlet doorway into it, the obvious move, is
    invisible in the only view the game ships. Nor can an inlet be saved by
    sinking an open-topped channel into that edge: the sight line out of one
    travels up and over the machine's own body, so anything as tall as a
    mixing drum occludes its own inlet. The resolution is asymmetric on
    purpose — **an outlet is a doorway on the +X face; an inlet is a hooded
    transfer housing whose CAMERA-FACING SIDE is cut away**, so the product
    path is shown from the side rather than through a mouth that cannot be
    seen. Both read by value contrast (palette-brightest frame, genuinely
    black opening, one brass threshold) rather than by detail, because at
    scene scale a junction is a dozen pixels; a mid-grey interior renders as
    an empty portal and a hazard stripe across the one camera-facing edge
    renders as a dark bar, both of which were built and rejected on the
    render. Corollary worth knowing before drawing any new junction: a
    machine big enough to stand between its own inlet and the camera (the
    2x2 cooking vat) needs its opening widened along the face until it
    reaches an unoccluded corner, and a *neighbouring building* can hide a
    correct junction outright — the vinegar tank stands one tile diagonally
    in front of the crusher's discharge and does exactly that.

12. **The full-scene render catches a class of bug no headless test can.**
    `main.gd`'s item-id → model mapping is presentation-only with a fallback,
    so when a milestone renamed a chain intermediate the mapping silently
    went stale and two stages of the chain shipped *cardboard boxes* down the
    belt where pails of sauce belonged — every structural test still passed,
    because a box is a perfectly valid mesh. `scripts/screenshot-scene.sh`
    is what found it. Any presentation-side lookup keyed by a content id has
    this failure mode; the rendered scene is the only check for it.

13. **A process an order of magnitude longer than a craft cycle needs a
    passive readout, and on this camera that readout has to emit its own
    light (ARCHITECTURE.md §9 milestone 7's aging rack).** Every machine
    already reports *running vs. idle* through its status lamp, and the hover
    tooltip already reports progress — which is enough while every process on
    the line is one to four seconds long. It stops being enough at 2400
    ticks: the only question a player has about a machine that will sit
    loaded for over a minute is "how far through is it", and an answer you
    have to park a cursor on is not an answer you can glance at. So the rack
    carries a **maturation gauge**: a slot in its corner post with an
    emissive amber bar scaled to the live process fraction — the fourth use
    of the same one-voxel-slab-scaled-every-frame mechanism the pipe liquid,
    the hopper fill and the tank level already use (point 5 above), not a
    fifth mechanism.

    Two things about it were only discoverable by rendering it. **The gauge
    face is permanently unlit.** §1's fixed camera means the only visible
    vertical faces are +X and +Z-world (point 11), and on this scene's key
    light *both* of those read as shadow side — every bright surface in a
    machine close-up is a horizontal one. A painted or brushed-metal gauge
    would therefore be a dark stripe on a dark column; emissive is not
    decoration here, it is the only thing that reads. **And an empty gauge
    has to show its own scale.** With the bar hidden below its sub-voxel
    threshold the slot is just black, so the player cannot see how far the
    bar still has to climb — the fix is a lit green line at the head of the
    scale, always on: a finish line, which is a *position* rather than a
    state encoded in colour (§7). Amber climbing toward green is the whole
    readout.

    Corollary for the next machine that wants one: this is keyed on machine
    *kind* (`main.gd`'s `MACHINE_PROGRESS_GAUGES`) and deliberately sparse. A
    bar tracking the crusher's 60-tick cycle would be a flicker, and five
    flickering bars along a line is noise, not information.

14. **Wood is the one deliberate exception to point 8's "food contact is
    stainless" rule.** The aging rack's casks are oak because in a real
    barrel-aging room the wood *is* the process, not a material someone
    settled for — and GAMEPLAY.md §2's tier 6 names "barrel-aged" outright.
    It is also, with the packer's cardboard, one of only two large warm
    surfaces in the factory, so its palette entry is deliberately browner and
    redder than `cardboard`; shape does most of the work of telling them
    apart (a round hooped cask against a folded flat case) and hue does the
    rest. Recorded here so a later pass doesn't "correct" the barrels to
    steel for consistency.

15. **The machine with no mechanism is a legitimate silhouette.** Every other
    kind on this line is identified by something that moves — flywheels, an
    agitator, a filling gantry, a taping head (points 8/9). The aging rack
    has none, on purpose: what distinguishes it from its neighbours is that
    it only *waits*, and a rack of sealed casks with nothing driving them is
    the honest shape for that. Casks stand on end rather than lying in a
    cradle for the same camera reason as everything else here — lying down,
    a cask presents one round head and then recedes into the tile as a long
    ridge, and four of them render as corrugation; standing, each is a
    separate round-topped object with a visible head and bung, and four of
    those read as four things resting.

16. **The one building that is a place rather than a machine, and the
    occlusion rule that decided its layout (the delivery-dock ticket).**
    Every other kind on the line is equipment; the delivery dock is
    architecture — a poured platform at trailer-bed height with a striped
    edge, rubber bumpers, a despatch door under a canopy — because that is
    what it *is* (GAMEPLAY.md §6's "somewhere product is sent to be sold"),
    and because it is the only silhouette on the line that could not be
    mistaken for another housing with a mechanism on top.

    Point 11 established which faces this camera can see. What building the
    dock added is the **corollary about depth**, which matters far more for a
    building with an open floor than for a machine that is solid: a sight
    line leaving any point travels `(+x, -y, +z)` in model space, so anything
    at HIGH x and LOW y stands in front of everything behind it. The obvious
    design — a truck bay carved into the +X face, dead-on to the camera —
    was built first and reads beautifully on its own, and it walls off the
    entire deck, because every deck voxel's sight line has to pass through
    those last few columns on its way out. The tall structure therefore went
    to the FAR (high-y) side, where its own -Y face is still square to the
    camera and it occludes nothing, and the near half of the deck was left
    clear for the goods that stand on it. **Rule of thumb for the next
    building with something inside it: put the height at high y, put the
    content at low y.**

    Two smaller things came off the render and not off the generator. The
    platform was `concrete_dark` for its whole mass on the first cut, which
    is right for a floor slab's seam and wrong for a block this size — it
    read as a black pit with a lit rail round it. And the canopy stood two
    voxels clear of the wall on thin posts, which at scene scale is not read
    as a gap, it is read as a white shelf floating in front of a blue wall;
    it is integral with the wall's head now.

    **Updated 2026-09-09: the dock is 2x2, and the corollary above is why.**
    A playtest said the one-tile version did not read as a loading dock, and
    it did not — everything above was true of it and all of it was crammed
    into one 32-voxel tile, which is also why the goods standing on the deck
    had to be capped at two cases. The extra tile went into DEPTH rather
    than width for exactly the rule this point states: the far tile is the
    building (despatch wall, two doors, one continuous canopy, a bay light
    over each door) and the near tile is open apron. A dock twice as wide
    and still one row deep would have left the height and the content
    fighting for the same row. One new wrinkle the bigger model turned up,
    worth knowing before drawing any multi-tile building: **a machine's
    infeed hood claims the corner of its own tile, and a wall drawn through
    it is a hole in the building** — the far tile's hood and the despatch
    wall wanted the same corner, and the wall now starts east of it, which
    is also what a real bay looks like (the conveyor comes in at the end of
    the building, not through the middle).

17. **What stands on the dock is drawn from the pool, not from the dock —
    and it is capped (at four since the dock became 2x2; two before that).** The dock's own buffer is a pass-through that
    empties every tick (`sim_core::sim::dock`), so a fill gauge driven from
    it would read zero forever: the same gauge-lies-about-the-mechanism
    failure the fluid tank refused when it chose constant head over its own
    fill fraction. The number that is real is the economy's shared
    `delivery_stock`, which is what the contract clipboard already labels ON
    THE DOCK, so the deck stands one case per unit of *that* — each drawn
    with the item's own model, through the same `ITEM_MODELS` table the belt
    uses, so what waits on the dock looks like what arrived on it.

    The cap was two because the deck was one tile and a case is a third of
    it: a rendered four-slot version buried the transfer housing, the
    despatch door and the canopy under a wall of cardboard. **That reason
    went away with the 2026-09-09 footprint change** (point 16), which left
    the whole near tile clear, so it is a two-by-two block of four pallet
    positions now, standing in front of the building rather than on top of
    it. The cap itself stays, and so does what it is for: this is a
    glance-readable "goods are waiting", not a readout — the exact figure is
    on the clipboard and in the dock's own hover, neither of which is
    capped.

18. **A rotated placement needs no new geometry — and the junction art is the
    one thing that does not survive the rotation (the "Real Placement" epic's
    ticket 2).** Every asset here is modelled with its product path along +X,
    because until that epic +X was the only direction anything could face. A
    placement facing another way is therefore just the same mesh spun about
    its own origin — the encoder puts that origin at the model's footprint
    centre (point 1), so the node is re-centred on the rotated rectangle and
    nothing else changes. That covers belts, pipes, machines and buildings
    alike.

    What it does *not* cover is point 11's junction pieces, and the reason is
    point 11 itself: an outlet is a doorway carved through the camera-facing
    +X wall and an inlet is a hood cut open on that same visible side, so
    spinning either 180° puts the carved side where nobody can see it and a
    blank wall where the product path is — **visibly worse than the plain,
    symmetric tile**. So the belt's `belt_inlet`/`belt_outlet` variants stay
    east-only and three of the four facings a player can choose fall back to
    the plain model. That is a known, tested, accepted gap, not an oversight:
    a four-directional junction pass is its own art ticket, and until it
    happens the fallback is what keeps a turned line honest rather than
    wrong. A machine turned away from the camera is a milder case of the
    same thing — its own doorway is still in the right place relative to its
    body, it is simply on the side you cannot see.

    **Amended 2026-09-09 (the pipes and conveyors overhaul, ticket 2):** the
    belt's `belt_inlet`/`belt_outlet` variants are retired — see point 23
    and Part A's entry of the same date. `pipe_inlet` is unchanged by that
    ticket.

19. **Two meshes that share a face z-fight, and the place it happens on a
    pipe is the fluid body against the OUTSIDE of the tube (ticket 3 of the
    Pipes & Conveyors Overhaul).** The one-voxel fluid slab (point 5) is
    scaled inside the bore, and the obvious width for it is the bore's
    width. But a round tube's outer wall curves in toward the crown, so at
    the rows just below the crown the outer wall is exactly as wide as the
    bore's equator — and a full column's side faces then lie ON the tube's
    outer surface. The render showed a coloured stripe running along the
    top of every closed tile, which no test and no model preview could
    have shown (a preview has no fluid in it). The rule: a live body scaled
    inside a hollow shape must be strictly narrower than the shape's
    outer wall at every height it can reach, not merely inside the cavity.

20. **Anything that will be spun by Godot must be symmetric about the
    tile's midline, which is BETWEEN voxels 15 and 16.** The integer-
    centred primitives put an axis on a voxel, half a voxel off the
    midline, and a placement rotated 90° or 180° about its footprint centre
    lands that axis on the other side of the midline. A pipe run at mixed
    facings stepped by a voxel at every joint. The fractional-centre
    primitives (`cylinder_x_f` etc., centre 15.5 in index units) are the
    fix; the rule applies to anything that has to meet its neighbour
    across a boundary — belts, pipes, junction art.

21. **A dark accent on a camera-facing shadow-side face reads as a hole.**
    The end cap's brass disc was meant as "a fitting"; on the +X face,
    which the camera looks straight at but the key light does not reach,
    it rendered as a black circle in a pale ring — the open mouth it
    existed to close. Point 11 already says an opening must be genuinely
    black; the converse holds too: a thing that is NOT an opening must not
    be a dark circle on that face. Small brass bolts on a bright plate keep
    the accent.

22. **A composable fitting is an overlay, and an overlay must keep out of
    the shape's windows.** With one fitting for every shape and every
    facing, the fitting cannot know which leg it lands on; so every shape
    puts its sight window in the same place relative to the +X leg
    (20..26) and every fitting keeps tube-height hardware east of 27. Its
    bore is carved a little wider than the shape's so the two meshes never
    share the bore surface (the same z-fight as point 19, from the inside).
    `vengi`'s isometric preview looks from the -X/+Y corner — the two faces
    the game camera never shows — so a fitting or a cap on the +X face can
    only be judged in the real scene; the deprecated `pipe_source`/
    `pipe_inlet` were kept as merged compositions for exactly that review.

---
23. **A thin diagonal mark on the tile plane does not survive this camera;
    a solid shape does (the pipes and conveyors overhaul, ticket 2).** The
    belt family needed a direction mark that reads at a glance at all four
    facings. The obvious mark is a chevron: two arms at 45° to the lane
    meeting at a point, two voxels thick, in the belt's own signage yellow.
    In the 512px preview it was a chevron. In `scripts/screenshot-scene.sh
    --closeup` it was an `L`, on every tile, at every facing.

    The reason is the camera, and it is worth stating as geometry because it
    applies to every mark anyone draws on a horizontal surface in this
    game: the isometric view direction is `(1, 1, 1)`, so it looks straight
    down one of the tile's two diagonals. A line along that diagonal
    projects to a short screen *vertical*; a line along the other diagonal
    projects to a full-length screen *horizontal*. A `>` whose arms are the
    two diagonals therefore becomes one long horizontal stroke and one
    short vertical stub meeting at a corner — an `L` — and an `L` points at
    nothing. Turning the tile does not help: whichever facing, one arm is
    always the long one and the other the stub. No thickness fixes it either;
    it is the arms' directions that are lost, not their width.

    What survives projection is *area*. A filled wedge is a wedge from any
    angle because its apex is its apex and its base is its base; the
    splitter's inlaid readout is a filled arrow for the same reason, though
    that was found by a different route (two thin arrows crossing into a
    blob). So the belt's mark is a solid arrowhead with a short shaft,
    sixteen voxels across the base and fourteen long, standing one voxel
    proud of the rubber, and the close-up shows an unmistakable arrow on
    every tile of an east-running and a west-running line side by side.

    Three smaller findings from the same renders. **Raised beats recessed
    for a mark on a running surface**: a recess is seen through its own
    walls, and one wall always runs along the sight line and shadows the
    floor beside it, so a recessed diagonal loses one edge entirely; a
    plate is lit from above at every facing. **A jammed belt hides its
    marks**: with product nose to tail only the arrow's fringe shows either
    side of each item, which is enough to give the direction and is the
    correct trade — the items are what the player is looking at. **The
    item overlap is real but negligible**: an item drawn at the surface
    height stands one voxel into a raised plate, and at scene scale that is
    a yellow sliver under the item's foot rather than a visible clip.

    Corners were checked the only way they could be before ticket 5 draws
    them: a temporary, uncommitted edit to `main.gd` that drew the corner
    models in place of straight belts in the demo fixture, at three
    facings. The quarter-annulus lane, the curved striped rails and the
    bent arrow all read as a belt turning a corner, and the bare floor at
    the tile's outer corner and pivot corner reads as floor, not as a hole.
    That render is not repeatable from a committed script, which is why the
    check is flagged for ticket 5 rather than claimed as done.

    **Amended 2026-09-10: the arrow itself is gone.** The player asked for
    the belt arrows to be removed, and the mark is now a plain cleat — a
    bar of dark rubber two voxels long across the lane, still raised one
    voxel for the reason above — whose only job is to make the four frames
    differ so the belt visibly runs (Part A, 2026-09-10 "Belt arrows
    removed"). Direction is given in words by the build prompt and by
    hover. Everything this point records about the camera still holds for
    the next mark anyone draws on a horizontal surface.

24. **A fitting overlaid on a shape must be a full voxel of radius larger
    than anything it can cover.** Ticket 3's "+0.4 on the bore" kept the two
    meshes apart in the bore and on a bare +X leg; it did nothing for the
    case ticket 6 hit first in a render — the machine fitting on the LOW-X
    face of the run's first tile, sitting on the straight's own joint
    flange. Two voxel solids z-fight wherever a voxel of both has an
    outward neighbour in neither: both then expose a face on the same
    plane. One full voxel of radius margin makes every axis-neighbour of an
    inner voxel part of the outer solid, so no face is ever shared. Pinned
    by `cargo test -p voxgen` for both fittings on both faces of a
    straight; the tile-boundary plane is exempt, because what stands beyond
    it (the machine's nozzle, the tank's spout) is what the render is for.

25. **A pipe tile's turn is a property of what it is joined to, never of
    the facing it was placed with.** The one joint flange per straight has
    to sit on the same side of every tile along an axis, or two neighbours
    stack theirs on a shared boundary; so an east-west straight is always
    turn 0 and a north-south one always turn 1, and R on a pipe visibly
    changes only a tile joined to nothing. An end's turn is dictated by its
    one open side, so it cannot obey the rule and carries no joint flange
    at all — the cap is its identity. Junction shapes never had one.
26. **A uniform offset in a path convention is invisible on a straight
    line and shows up the moment the line bends** (the pipes and conveyors
    overhaul, ticket 5).

    `BeltSnapshot` gives an item's distance from the tile's start — its
    input edge — to its end, in `[0, 1]`. `main.gd` had drawn distance 0
    at the tile's *centre* and distance 1 at the *next* tile's centre.
    Because every tile shared the half-tile shift, a belt-to-belt hop was
    continuous (the sender's 1 and the receiver's 0 landed on the same
    point), and every render of a straight line since the core-loop
    milestone looked right. What was actually wrong: a hopper's item
    appeared mid-belt instead of at the hopper's mouth, and a machine's
    item vanished at the machine's centre instead of at its face, both of
    which read as "the item went under the hood" and were never questioned.

    The corner exposed it. The plan's arc runs from the input face's
    midpoint to the output face's, and drawn half a tile late it would have
    left the tile through the wrong face — and a corner-shaped queue of
    tomatoes is what the gallery render shows, so the arc, not the offset,
    was the thing to keep. Items are drawn face to face on every tile now
    (`centre + facing * (distance - 0.5) * TILE` for a straight), and the
    populated demo render shows the first tomato standing in the hopper's
    mouth and the last one at the crusher's face.

    The general rule: check a path convention against the tile's own
    edges, not against its neighbours, because neighbours share the error
    and hide it. `test_belt_shapes.gd` pins the straight run's two ends to
    the tile's faces and a corner's ends to its neighbours' ends, which is
    both halves of that check.

    Two smaller findings from the same renders. **Both hands of corner
    read at all four facings** with no yaw of their own (the models are
    authored output-+X and the flank relation survives the spin), so the
    caller-supplied-yaw seam ticket 4 left is unused. **A closed ring of
    belts is a legal placement and runs**: fed from a flank hopper, the
    first items go all the way round and queue back at the merge tile,
    which is a real state and draws a smooth curve of items through the
    corner — the arc's best proof.

27. **A machine is animated by a separate motion model in `MACHINE_FRAMES`
    arrangements, and the moving parts are cut out of the body rather than
    duplicated into the overlay** (the machine-animations pass).

    Point 23 answered "how does a voxel thing move" for a tile; this is the
    same answer for something that is not one. `voxgen` emits
    `<machine>_motion_f0..f3` for each kind — the crusher's roll teeth and
    twin flywheels, the mixer's ribbon flights, the vat's agitator spider,
    the bottler's nozzles and capping chuck, the packer's taping carriage,
    and a drive spider on the generic housing's crown for any kind that has
    no model of its own. Each is cut on the machine's own footprint, so both
    models put their origin at footprint centre at ground level (point 1) and
    Godot needs no offset at all: four sibling nodes at the body's position
    and yaw, exactly one visible, and a frame change is two visibility flips.

    The moving parts are **removed from the body**, not copied. Point 24's
    rule is why: two voxel solids z-fight wherever a voxel of both has an
    outward neighbour in neither, and a z-fight at scene scale reads as a
    texture seam rather than as a bug. Disjointness is arranged by
    construction — `cylinder_*` fills `d <= r` while `arm_cells` fills
    `inner < d <= outer`, so a core and an annulus about one axis cannot
    share a cell — and then pinned by a test that walks every voxel of every
    frame of every family against its body. That test is what found the
    mixer's ribbon passing through both feed throats and the discharge
    nozzle; the fix was to run the ribbon only through rows 20..43, which are
    the rows the sight window (y 24..39) shows, rather than to thin the
    flights away from the drum wall where the window displays them.

    Two rules the models themselves have to obey. **Every motion volume is
    an even number of voxels tall**, because `vox::encode` lifts a model by
    `size[2] / 2` in *whole* voxels, so an odd height sinks the overlay half
    a voxel and it meets the body along a seam instead of a face. **The
    frames must differ consecutively, not pairwise**: a reciprocating part
    passes through the same pose on the way out and the way back
    (`BOTTLER_PLUNGE` is `[0, -1, -2, -1]`, so f1 and f3 coincide), and the
    property that actually matters is that the mechanism never stands still
    for a frame while it is meant to be working. Four distinct angles is a
    separate, stronger test, applied to the four families a rotation drives.

28. **Motion means "working", and working is narrower than "has a batch"**
    (the same pass).

    `Machine::tick` clears a machine's `process` only once the finished
    goods have somewhere to go, so a machine that has completed its batch and
    is blocked on a full output keeps its process and retries every tick.
    Animating on `process != null` would therefore leave a jammed factory
    full of machines cheerfully turning with nowhere to put anything. The
    mechanism turns on `0 <= elapsed < required` instead, which leaves the
    status lamp saying "has a batch" and the mechanism saying "is turning" —
    and **lamp lit with nothing turning is the readout for "the thing
    downstream is full"**, which neither signal gives on its own. §7's rule
    about never encoding state in one channel, arrived at from the other
    direction: two channels that disagree are themselves information.

    The pace is one cycle per 30 ticks for every kind. Driving it from a
    machine's own process length is the obvious choice and is wrong: the
    crusher's 80 ticks against the vat's 240 reads as two factories at
    different speeds rather than one factory with a slow cook in it, and how
    long a stage takes is already stated by the product backing up behind it.
    The pace says "working", not "working this fast"; the belt carries speed.
    By the numbers, the way point 23's march was judged: 30 ticks at 30 Hz is
    four frame changes a second against the belt surface's 2.4, so a
    mechanism reads as busier than the rubber it feeds, and a fifth of a
    second a frame is the chunky end of §6 rather than a blur. The frame
    comes from the machine's own elapsed ticks, not the global tick, so a
    mechanism starts at frame 0 when work starts instead of joining a
    factory-wide beat part way through.

    **No steam**, though the cooking vat is the obvious place for it: a puff
    is mass a stopped vat should not have, and a frozen cloud over a cold vat
    states the opposite of what the lamp says.

    One finding about the review tool itself, which belongs here because
    point 12 is about exactly this. **`test_screenshot.gd`'s populate loop
    calls `sim.tick()` directly**, not `_advance_simulation`, so it never
    advanced the belt frame — every scene screenshot ever taken photographed
    belt frame 0, an idle-looking factory in the middle of a run. It now
    advances both animations there. And a vengi preview of a frame that is a
    rigid translation of frame 0 is byte-identical to it, because the
    thumbnailer crops to the bounding box: judge a translation in the scene,
    which is point 22's warning about vengi's camera in another form.
36. **The floor is the one surface whose job is to be a background, so it
    is authored by its VALUE RANGE, not by its detail.**

    Every other asset in this pipeline is designed by looking at one copy
    of it. The floor is never seen that way: it is seen as thousands of
    copies filling the frame, with everything else standing on top. At that
    scale anything carrying contrast becomes the loudest thing in the
    picture, and three separate versions of this floor failed in exactly
    that way before one worked. The frame that found each of them was the
    EMPTY world — no fixture, no machines — which is both the state a new
    game starts in and the only render that is almost entirely floor.

    The near-black `#33332F` control joint, drawn on all four edges of
    every tile so that an interior joint was two voxels of it, read as
    grout: the factory looked like a tiled bathroom rather than a poured
    slab. Single-voxel `concrete_light` speckle, at a dozen points above
    the slab tone, read as television static. And the repair patch, filled
    with the shared `concrete` mid tone on the sound real-world theory that
    a fresh pour is paler than a floor polished by traffic, measured +16
    render units against a floor whose entire remaining variation was ±2 —
    a scatter of white paving slabs across the plant.

    What came out of it, in order of how often it will apply again:

    - Keep the surface's own tones within a few points of each other. §7's
      "neutral/industrial" is a *narrow* band here, not merely a desaturated
      one. `concrete_slab`/`concrete_mottle`/`concrete_shade` span eight
      points in total, and the first attempt at a dozen points read as
      confetti scattered on the floor rather than as the floor.
    - Make variation broad and low-contrast: blotches on a coarse cell grid,
      most cells left bare, each blotch built from rows of differing length
      and inset so its outline wanders. A rectangle with its corners knocked
      off renders as a plus sign, which is its own kind of pattern. This is
      also the cheap option — broad flat areas greedy-mesh into far fewer
      triangles than per-voxel noise, and this is the one asset whose
      triangle count is multiplied by the size of the world (1 363 384
      primitives before, 798 272 after).
    - **Never let the floor hold the brightest pixel in the frame.** §4
      reserves that for metal, and it is what makes machinery pop. When the
      render and the real-world instinct disagree about which way a feature
      should go, this rule breaks the tie: the repair patch is darker than
      the floor around it.
    - A feature that genuinely wants contrast is the exception, and it
      should be a real object rather than a texture. The trench drain's
      grating is read by its stripes, so its bars stay pinned to `concrete`
      instead of to the mottle tone, which gets pulled toward the slab every
      time the floor is quietened.

37. **N models laid over thousands of tiles is still only N stamps; the
    multiplier is per-instance rotation, and it is only free if the tile's
    edges are four-fold symmetric.**

    Four floor variants — plain, worn, patched, drained — looked varied in
    isolation and in the wide shot. A magnified crop said otherwise: every
    worn tile's scuff ran the same way across the screen, every plain tile
    carried its blotches in identical spots, and once seen it could not be
    unseen. Laying each instance at a quarter turn chosen by a hash of its
    tile coordinate breaks all of it at once, at zero asset cost, and it is
    the correction that made this floor work.

    The constraint it imposes is on the EDGE. Whatever a tile draws at its
    border has to survive a quarter turn, or the seam steps wherever two
    differently-turned tiles meet — the same class of failure point 20
    records for an integer-centred axis, and just as invisible until the
    exact pair of neighbours occurs. So the saw-cut joint stays on all four
    edges of every rotatable model even though that makes an interior joint
    two voxels wide, and the fix for the doubled line is its VALUE, not its
    geometry: `floor_seam` was lightened until two voxels of it still read
    as a hairline. Dropping to two edges to halve the line is the tempting
    trade and it is the wrong one.

    Two corollaries. **The variant and the turn must both be pure functions
    of the tile coordinate**, because the scene rebuilds its whole World
    subtree on every place and remove (ARCHITECTURE.md §5): anything hashed
    on iteration order or on a counter would repave the floor under the
    player mid-build and come back different from a save. **The one model
    that must not be turned is the one whose feature spans tiles** — a row
    of trench drains is a trench only while every tile lays its channel on
    the same axis, so the drain always returns turn 0.

    What a property test can carry from here, and a render cannot re-check
    by itself: that every rotatable model's four edges really are identical
    (`every_rotatable_floor_tile_carries_the_same_joint_on_all_four_edges`),
    and that nothing on the floor stands proud of the slab's top surface —
    `main.gd`'s `FLOOR_Y` puts every belt, machine and building on that
    plane, so a single voxel of floor detail one step taller intersects
    everything the player builds, and it is invisible in every render until
    something happens to stand on the offending tile.
29. **A free-standing thing at a fixed place on a tile meets every body at
    every facing, and a face one voxel from a tile edge is a face one voxel
    from EVERY edge once the body turns.** The andon post stands at the
    tile's +X/low-Y corner, inset one voxel, and never rotates; the
    machines it is bolted to do. Two features that had sat harmlessly at
    the far side of their bodies — the crusher's motor one voxel from its
    +Y edge, the packer's status lamp one voxel from its -X edge — landed
    exactly on the post's own faces when the body was turned, and z-fought
    it (point 24) in a way no preview could show, because a preview shows
    one facing. The rule: an overlay that does not turn with its host is
    checked against the host turned all four ways, by a test, not by eye;
    and the check is on co-located voxels with a face exposed by both, which
    is the whole of point 24 in one predicate. The corollary for the inset
    itself: on the tile edge it fights the flush plinth, two in it fights
    the painted skirt, so one voxel is a pinned number and not a choice —
    the post passes THROUGH a body and never fights one. The last lesson is
    the lens ring's size: `VoxModel::set` clips silently at the volume, so a
    ring about a point two voxels from two edges has radius two at most, or
    it is flat on exactly the faces the camera sees.
30. **A per-tick fact shown beside a windowed verdict has to be held across
    the ticks it is absent.** The machine-load wire carries two things about
    a machine: `status`, a verdict over the last 600 ticks, and `stall`, the
    reason it is waiting THIS tick — and the second is `null` on most ticks
    of a machine the first calls blocked, because a blocked machine is
    mid-craft on most ticks and is only "held" on the ones a finished batch
    has nowhere to go. A hover that printed exactly this poll's `stall`
    beside a `status` that had not changed would read "blocked · output
    full" one second and "blocked" the next, which looks like the reason
    going away when it has not. The scene keeps the last non-null reason
    while the status holds and forgets it when the status changes (it was
    the old status's reason), and a test ticks a settled crusher through
    hundreds of polls and asserts the reason is never dropped. The general
    form: when one polled field explains another, the explanation's
    lifetime is the explained thing's, not the poll's.
31. **At icon scale an object is identified by the proportion of its
    parts, not by their presence.** The staff-pools epic's hard hat icon:
    the first generator drew a brim, a longer peak at the front, a domed
    crown and a crest over the top — the complete inventory of a hard hat
    — and the rendered 64 px icon was a bell, or a jelly mould with ribs.
    The crown stood 14 voxels high on a radius of 11 with a squared-off
    superellipse profile, so the object was taller than it was wide, and
    the vertical risers of its stacked slices read as ribbing. Lowering the
    crown to a plain ellipse 9 high on a radius of 12 and dropping the
    second crest — fewer voxels, fewer features — made it a hard hat at
    once. The rule: below a certain size the eye reads an outline's
    proportions and nothing else, and every correct detail on a wrong ratio
    is wasted. The working corollary: look at the icon at its real size on
    the real dark plate BEFORE looking at the 512 px preview, because the
    preview shows the detail, which is what the eye lands on, and the
    detail is not what the icon is read by. Point 9 already budgets a
    rework pass per asset; this is what the pass is usually for.
32. **A fixed-size card clips a label without complaint, and a taller
    card is not always available to fix it.** The build-menu card gained
    one line (`crew: 2 manufacturing`) and the first render cut it at
    "manufacturi" on the right and pushed `Owned 0` off the bottom; every
    headless test read the full string and passed (point 12 again — a
    Label's text is intact whatever the card shows of it). The obvious fix,
    a taller card, made the two-row grid scroll, because the menu is capped
    against the hotbar (§5c), and a wider card alone left the bottom line
    clipped. What worked was paying for the line out of the card's own
    slack: two less inset top and bottom, no separation, the smallest font
    step the scale allows, and a card wide enough for the longest crew
    string. The rule: a new line on a card that lives in a capped grid is
    bought from the card's padding, not from the grid's height, and the
    check is the rendered frame at the shipped window size.

## Part C — ARCHITECTURE.md §9a, full text (mobile export spike findings, 2026-09-06)

Ran against the milestone-1 scaffold, no gameplay code involved. Dev environment: macOS/arm64, Godot 4.6.3-stable (matching §1a's pin), only Xcode **Command Line Tools** installed (no Xcode.app).

### Android — works

1. Installed a standalone Android SDK (cmdline-tools, platform-tools, `build-tools;34.0.0`, `platforms;android-34`, NDK **27.2.12479018**) plus `rustup target add aarch64-linux-android x86_64-linux-android` and `cargo install cargo-ndk` — no Android Studio needed, everything scriptable/headless.
2. `cargo ndk -t arm64-v8a -t x86_64 build -p sim_core` cross-compiled `sim_core` unmodified — zero source changes were needed to make the milestone-1 crate Android-buildable. Both `.so`s copied to `godot_project/bin/android/`; `sim_core.gdextension` got `android.debug.arm64`/`android.debug.x86_64` (+ release) entries alongside the existing macOS/Linux/Windows ones.
3. Godot's **Gradle-based** Android export (required for GDExtension — the prebuilt-APK export path can't embed extra native libraries) needed several pieces of one-time local machine setup beyond what's in the repo, none of it Android-specific-obscure but all of it undocumented by Godot itself for a from-scratch headless setup:
   - Editor Settings' `export/android/android_sdk_path` and `export/android/java_sdk_path` (both machine-local, correctly *not* repo-tracked).
   - The Android Gradle build template extracted from the export templates' `android_source.zip` into `godot_project/android/build/` (normally done by Godot's own `--install-android-build-template`, which — see below — hung in this environment; extracting it manually and writing the `.build_version` marker Godot itself expects there worked identically).
   - `rendering/textures/vram_compression/import_etc2_astc=true` in `project.godot`. **This is the one genuine surprise worth flagging for future sessions:** without it, export fails validation with a completely blank error message — Godot's own Android exporter has a config check (`ResourceImporterTextureSettings::should_import_etc2_astc()` in `has_valid_project_configuration`) that sets `valid = false` without ever appending anything to the error string it hands back, so the CLI just prints "due to configuration errors:" and nothing else. Confirmed by reading Godot's own `platform/android/export/export_plugin.cpp` at the 4.6.3-stable tag rather than guessing — this is a real (if narrow) upstream gap in the CLI export path's error reporting, not a mistake in this project's config.
   - A debug keystore (any `keytool`-generated one works) — machine-local by convention, so keep it out of `export_presets.cfg`; either point Editor Settings at one or export `GODOT_ANDROID_KEYSTORE_DEBUG_{PATH,USER,PASSWORD}` at export time.
4. **A real, undocumented headless-automation trap:** running `godot --headless --install-android-build-template` when the project's `export_presets.cfg` still has unresolved Android config errors makes the editor pop a **native blocking macOS alert dialog** — confirmed via a real on-screen window (`orca computer list-windows` found it) that never returns control even under `--headless`, because that flag doesn't suppress this particular alert path. There is no CLI flag to auto-dismiss it; the only way through in an environment with no one to click "OK" is to avoid triggering it in the first place — fix the underlying config errors first (per the bullets above), or extract the build template by hand instead of via that command, as this spike ended up doing.
5. Result: `godot --headless --export-debug "Android" build/android/sauce-factory.apk` produced a **373 MB, validly-signed** (`apksigner verify` ✅, APK Signature Scheme v2) APK with both `lib/arm64-v8a/libsim_core.arm64-v8a.so` and `lib/x86_64/libsim_core.x86_64.so` actually embedded (`unzip -l` confirmed), package `com.saucefactory.spike`, target SDK 36.
6. **What was *not* verified, and why:** actually installing/launching the APK on a device or emulator to confirm the extension loads and runs at runtime. This dev host is a shared multi-agent machine that was down to double-digit MB of free physical memory with other sessions' work running concurrently (one background process here was already killed for memory pressure mid-spike) — booting a full Android emulator on top of that risked destabilizing sibling sessions on the same machine, so it was deliberately skipped rather than risking a shared resource. The APK's structural/signature validity was confirmed instead (`apksigner verify`, `aapt2 dump badging`). **This is the one open item before Android export can be called fully proven, not just built** — install/launch on a real device or a properly-resourced CI runner and confirm the GDExtension actually initializes at runtime (e.g. via `adb logcat` for the `Initialize godot-rust` line this project's own scaffold already prints).

### iOS — blocked, not proven broken

Two independent checks both confirm the blocker is this environment, consistent with §0's flagged risk (which was about iOS being *buggy*, not *impossible* — worth distinguishing):

1. `xcodebuild -showsdks` and `xcrun --sdk iphoneos --show-sdk-path` both fail outright: `xcode-select`'s active developer directory is `/Library/Developer/CommandLineTools`, and the iOS SDK is only ever shipped inside full Xcode.app, never inside standalone Command Line Tools. Godot's iOS export generates and builds an actual Xcode project (`xcodebuild`) — there is no headless-CLT-only path around this.
2. Confirmed independently at the Rust layer too, so this isn't just "Godot's fault": `rustup target add aarch64-apple-ios` + `cargo build --target aarch64-apple-ios` **appeared to succeed** (no build error), but the linker fell back to the macOS SDK sysroot with an explicit warning (`ld: building for 'iOS', but linking in dylib ... built for 'macOS'`), and `vtool -show-build` on the resulting `.dylib` shows an internally inconsistent `LC_VERSION_MIN_IPHONEOS`/`sdk 15.5` (a macOS SDK version number) pairing. **This is exactly the kind of "looks done, isn't" result worth calling out explicitly: the build not erroring is not evidence the binary is a valid iOS artifact** — it almost certainly is not, and this was not treated as a pass.

Godot's own iOS export was not attempted with a real preset run to completion — the environment gates in (1) make the outcome deterministic (it would fail at the Xcode-project-build step) and attempting it burned into a separate, apparently-unrelated instability instead (see below), so the Xcode-absence finding stands on its own two-pronged evidence above rather than on a completed (and therefore misleadingly "further along") export log.

**Recommendation per this milestone's own framing (§9 item 2's "decide explicitly"): keep iOS as a stretch/deferred platform, not a committed milestone-10 launch target, until this spike is re-run to completion on a Mac with a full Xcode install** (Xcode.app, not just Command Line Tools) or on Apple-hosted CI (e.g. GitHub Actions `macos-*` runners, which ship full Xcode). That re-run should be quick — the Android half of this spike shows the `gdext`/GDExtension side of the pipeline needs no source changes for mobile, so the only unknown left for iOS is Godot's own Xcode-project-generation and code-signing path, which needs the real toolchain to observe.

### An unrelated instability surfaced late in this spike, worth flagging separately

Late in this session, this shared dev host (12 concurrently logged-in users, load average 6.5–8+, physical memory pinned with double-digit MB free) started failing to even load the **unmodified, checksum-verified-identical** `libsim_core.dylib` that had worked correctly across all eight-plus Android export runs earlier in the same session — every invocation (including the plain milestone-1 `scripts/verify-scaffold.sh` round-trip check, with no export involved at all) now aborts immediately with:

```
ERROR: [panic .../godot-ffi-0.5.5/src/toolbox.rs:462]
  unsafe precondition(s) violated: hint::unreachable_unchecked must never be reached
   at: godot_core::private::install_godot_rust_hook::{{closure}} (.../godot-core-0.5.5/src/private.rs:331)
thread caused non-unwinding panic. aborting.
```

This reproduced identically with a fully clean environment (`env -i`) and against a byte-identical dylib (`md5` checked before/after), which rules out anything this spike changed. It looks like a resource-exhaustion-triggered trip of one of `gdext`'s optional Rust UB-check assertions (its own error text says as much: "this Undefined Behavior check is optional, and cannot be relied on for safety") rather than a real bug in `sim_core` or in this project's changes — **but it means milestone-1's own round-trip check may currently be unreliable on this specific shared host under load**, independent of anything mobile-export-related. Re-run `scripts/verify-scaffold.sh` on a less-contended machine (or this same one once load drops) before treating any future failure of it as a real regression.

---

