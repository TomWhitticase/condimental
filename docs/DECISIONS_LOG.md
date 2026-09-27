# CONDIMENTAL — Decisions log (condensed)

Every entry here is **the rule**, newest first. The full reasoning behind each
one is in [DECISIONS.md](DECISIONS.md), Part A, under the same date and title.

This file used to live in the root `CLAUDE.md`, where it was injected into
every request. It moved out on 2026-09-16: a log grows without bound, and
almost every entry is irrelevant to any given task. Read it when you are
about to change something that looks settled, or when you need to know
whether a question has already been answered. **Do not re-litigate these
without a good reason.**

When you make a non-trivial design decision not covered by
[ARCHITECTURE.md](ARCHITECTURE.md), [UI_UX.md](UI_UX.md) or
[GAMEPLAY.md](GAMEPLAY.md): add the condensed rule here, the full reasoning to
[DECISIONS.md](DECISIONS.md), and a note to whichever design doc it affects.
See [WORKFLOW.md](WORKFLOW.md).

---

- 2026-09-26 **The review always runs; only the merge is ever held.** An
  amendment to the entry below, stated after PR #193 ended a session open
  and "waiting on review" that nobody had been asked for. An author with no
  `VERDICT:` yet commissions the reviewer instead of stopping, and waits for
  the verdict before ending the session, because the subagent does not
  outlive it. It commissions one even when the merge cannot happen: when the
  task forbids the author merging (an Orca worker's coordinator merges), or
  the change also needs the owner (a PR the owner asked to review, a
  design-doc deviation, a `gdext` bump, a save-format change), the PR ends up
  open *with* its verdict, naming the hold. Those held PRs, one handed over
  with its latest verdict named, and one for which no subagent could be
  launched are the only PRs that may end a session open — a `CHANGES
  REQUESTED` verdict is work still in hand, not a way to close. Binds
  `CLAUDE.md`, `docs/WORKFLOW.md` and the `open-pr` and `wrap-it-up` skills.
  Recorded in the same `docs/DECISIONS.md` record as the entry below.

- 2026-09-26 **A PR's reviewer must not share its author's context; it need
  not be another session.** The merge still gates on green checks plus a
  `VERDICT: APPROVED` review, but the reviewer is now typically a fresh
  subagent the author launches. Its whole prompt is the PR number and the
  instruction to follow the `review` skill, with none of the author's
  reasoning, summary, list of what was verified or hints at what is fine. A
  separate session that did not write the branch also qualifies. The author
  reviewing its own work, or a subagent briefed with the author's reasoning,
  never does. "Make a PR and merge it" now means: launch that reviewer, then
  merge on its verdict. This amends the 2026-09-25 rule below. Binds
  `CLAUDE.md`, `CONTRIBUTING.md`, `docs/WORKFLOW.md`, `.githooks/pre-push` and
  the `open-pr`, `review` and `wrap-it-up` skills. Full record in
  `docs/DECISIONS.md`.

- 2026-09-26 **The build menu is sectioned by category, has a search box,
  and may scroll at 1600x900.** The owner asked for categories plus a search
  box. Categories are content (`data/shop.ron` `categories`), three sections
  of five/five/two cards do not fit two rows at 1600px, so §5c's "nothing
  scrolls at 1600x900 but the clipboard" gains the build menu as its one
  exception. The search box sits outside the scroll as the way to what is
  below the fold, and the cut lands on a card-row or heading edge. Binds
  `docs/UI_UX.md` §5c, `godot_project/main.gd` `_position_shop_menu`.

- 2026-09-26 **A new game starts with no contract.** No offer in
  `data/contracts.ron` is `starter: true`, so `World::new` activates nothing
  and the player signs their first order from the contracts board (**C**)
  the same way they buy their first tile. This reverses the 2026-09-10 rule
  that at least one offer must be a starter, at the owner's request: the
  world starts empty, so a contract running before the first tile is placed
  only burns strikes. The loader no longer requires a starter
  (`ContractLoadError::NoStarter` is gone); `starter` stays an optional
  field, so the schema and saves are unchanged. The guard against "a factory
  with nobody to sell to" is now the empty clipboard, which names **C**.
  Fixtures that need an order sign one through `SignContract`
  (`sim::reference_chain`, `test_layouts.gd`'s `build_reference_line`).
  Full record in `docs/DECISIONS.md`.

- 2026-09-26 **CONDIMENTAL is GPL-3.0-only; the name and brand are
  reserved.** The code is licensed `GPL-3.0-only` (`LICENSE`, and `license =`
  in every workspace `Cargo.toml`) **with a GPLv3 §7 additional permission**
  (`LICENSE-EXCEPTION.md`) to link the Steamworks SDK, GodotSteam and other
  platform SDKs — plain GPL would forbid a Steam build using achievements or
  the overlay, and once outside contributions land the exception could no
  longer be added without every contributor's consent. The name "CONDIMENTAL", its logo and the
  official store art are **not** under that grant — `TRADEMARKS.md` says a
  distributed fork must be renamed. Distribution is two routes to the same
  game: free by building from source, and a small price on Steam for the
  packaged build. Every crate linked into a build must carry a licence on
  `deny.toml`'s allow-list (all GPL-3.0-compatible); `cargo deny check
  licenses` in `.github/workflows/audit.yml` refuses anything else, and
  widening the list is a licensing decision, not a build fix. A distributed
  build ships `LICENSE` and `THIRD_PARTY_NOTICES.md` (Godot's own
  `COPYRIGHT.txt` included). Contributions are inbound = outbound under the
  same licence and exception (`CONTRIBUTING.md`). Binds `LICENSE`,
  `LICENSE-EXCEPTION.md`, `TRADEMARKS.md`,
  `THIRD_PARTY_NOTICES.md`, `deny.toml`, `docs/ARCHITECTURE.md` §1b. Full
  record in `docs/DECISIONS.md`.

- 2026-09-26 **A spend that would leave the next payday unpayable is
  confirmed, not refused, and the simulation is what decides.** Every route
  that spends money — the build menu's buy (and the number keys), a market
  crate and its SHIFT bulk buy, a contract fee, a hire, and SHIFT's
  replacement purchase during a placement run — passes through
  `main.gd`'s `_spend_with_confirm`, which asks
  `SimBridge::spend_preview_json` about the **exact command dictionary it is
  about to issue** and obeys the `warn` it answers. `World::spend_preview` is
  read-only, prices each spend by calling the same function the command arm
  calls (`Shop::cost_of`, `Ingredients::cost_of`, the board offer's own
  `price`, the category's `wage` against `StaffPools::next_bill`) and mirrors
  each arm's own refusal predicate, so there is never a second copy of a price
  rule; it moves neither `revision()` nor the save. The panel prints that
  answer's figures — subject by display name, balance after, bill, gap, and
  payday in seconds — and computes none of them. It warns in **both** cases
  the preview distinguishes: the spend that crosses the line (`crosses`) and
  any spend made while already short, with a different heading for each. There
  is **no suppression** and no "don't ask again". The same decision turned
  `StaffPools`'s hire refusal into a warning: `hire` now refuses only an
  unknown category, the wire field `can_hire` became `hire_warns`, and the
  staff row's HIRE button is gated on `required > 0` alone and says why in
  words beside a live button. The confirmation does **not** pause the
  simulation — only the pause menu does — which is why the command is issued
  through `issue_command_json` when the player answers rather than replayed,
  and why the tail of an interrupted action (`_confirm_build`'s
  `_finish_build`) must run on either answer. **A spend the balance cannot
  cover raises no panel** (`affordable: false`): the economy refuses it, so it
  leaves the payday exactly as short as it already was and gets the ordinary
  refusal cue instead. Binds
  `sim_core/src/sim/world.rs`, `sim_core/src/sim/staff.rs`,
  `sim_core/src/lib.rs`, `sim_core/tests/spend_preview.rs`,
  `godot_project/main.gd`, `godot_project/test_payday_warning.gd`,
  `docs/UI_UX.md` §2b point 40 and §5, `docs/GAMEPLAY.md` §7/§11 and
  `docs/ARCHITECTURE.md` §4. Full record in `docs/DECISIONS.md`.

- 2026-09-26 **Each dock holds its own stock; contracts draw on the docks
  together.** A dock keeps what it takes off its belt in `DOCK_CAPACITY`
  (32) places, one case each, whatever the case holds — the ceiling is a
  per-dock total, not a per-item cap. At its ceiling `Dock::try_absorb`
  refuses and the belt backs up. `World::tick` step 7 hands `Economy::tick`
  a `DeliveryStock` over the docks able to ship this tick (not mothballed,
  not refused crew in step 6), in topology order; a contract is satisfied
  if the SUM covers its mix and consumes each line dock by dock in that
  order. `ContractEvent::Paid { id, from }` names the docks it drew on, and
  every such dock records `last_paid_tick` (saved). Step 6 claims a dock's
  logistics crew only while it holds something an active contract orders
  (`Economy::wanted`); a dock of unordered goods asks for nobody, so it never
  holds a head another dock's orders need. The coins rise over
  those docks only, and an arrival pops only the dock whose
  `shipped_total` rose. `EconomySnapshot::delivery_stock` /
  `delivery_capacity` / `delivery_full` are the docks summed / total places
  / "some dock is at capacity". Removing a dock destroys its stock (the
  ordinary `Remove` rule) — that is the valve for a dock clogged with a
  product nobody orders. Save schema 3; v2 saves are rejected, not
  migrated. **Supersedes** 2026-09-08 "Delivery dock, ticket 1" (the
  pass-through into one pool) and the pool half of 2026-09-16's full dock
  (`DELIVERY_STOCK_CAPACITY`, 48 per item, `OverfullDeliveryPool`).
  > Full record in `docs/DECISIONS.md`.

- 2026-09-26 **At most one RAW fluid may enter a machine kind, and at most one
  recipe on that kind may drink it.** A new sauce whose fluid wants into a kind
  that already drinks a bought fluid must take it as a *derived* fluid, made a
  stage earlier by a machine of another kind — or get a kind of its own.
  `World::resolve_network_fluids` names a pipe run by narrowing on FLUIDS
  ALONE: it seeds a run from every recipe of every kind that touches it,
  regardless of what those machines are actually running, and the only outside
  fact is "a run a tank feeds carries a raw fluid". So two recipes on one kind
  drinking the same raw leaves that kind's outlet run ambiguous, and one kind
  whose recipes drink two different raws leaves the TANK run ambiguous — and an
  ambiguous tank run cannot even be filled, because `World::tank_fluid` has
  nothing to name it with. Both are stalls on a line the player already built,
  and `RecipeRegistry::for_machine` is not gate-aware, so unreleased content
  breaks a shipped line in a new game. Mustard is the case that found it: its
  vinegar draws at the CRUSHER, not at the mixer beside turmeric, and the
  mustard mixer takes the derived `mustard_mash` down a pipe. Binds
  `data/recipes.ron` (the rule is stated in its header), and is fenced by
  `sim_core/src/sim/recipe.rs`'s `at_most_one_raw_fluid_enters_any_machine_kind`
  and by `sim_core/tests/mustard_line.rs`. Teaching the resolver about the
  solid side of a recipe would lift the rule; until then it is content's to
  obey. Full record in `docs/DECISIONS.md`.
- 2026-09-25 **THE COOKBOOK is derived from the content files, and nothing in
  it may be written in GDScript.** The in-game encyclopedia (B, and a page on
  the pause menu) draws `SimBridge::cookbook_json` and only that: the sauce
  tree, the tiers, the family edges, cost per case, the contract pay range and
  the gates are all computed in `sim_core/src/sim/cookbook.rs` from the
  registries, with no `World`. `godot_project/cookbook_panel.gd` contains no
  sauce name, machine name, ingredient, tier, price or arrow, and no fallback
  prose — a sauce with no `blurb` gets no blurb paragraph rather than a
  sentence GDScript made up. `test_cookbook.gd` enforces this by checking
  every id, display name and contract name in the book against the panel's own
  string literals, so a hardcoded word fails the suite. The one number the
  panel computes is seconds, because `TICK_HZ` is GDScript's. Prose that is
  not derivable lives in new optional `tagline`/`blurb`/`icon` fields in
  `data/{recipes,machines,ingredients}.ron`; a Cookbook that needs a fact
  those files cannot state is a missing data field, not a Rust change. A
  locked family stays in the tree as an unnamed silhouette carrying its gate,
  and its page is the gate only. Binds `sim_core/src/sim/cookbook.rs`,
  `sim_core/src/lib.rs`, `godot_project/cookbook_panel.gd`,
  `godot_project/test_cookbook.gd` and `docs/UI_UX.md` §5h. Full record in
  `docs/DECISIONS.md`.
- 2026-09-25 **Every PR is reviewed BEFORE it merges, with no exceptions.**
  A PR merges only when its checks are green AND it has a `VERDICT: APPROVED`
  comment from a session that did not write it (the `review` skill). Routine
  changes get no exemption, and a review after the merge does not count. An
  instruction to "make a PR and merge it" means merge after that verdict. With
  no verdict, the author stops with the PR open and says it is waiting on
  review. This replaces the old WORKFLOW.md wording that let authors merge
  their own routine work unreviewed. Binds `CLAUDE.md`, `docs/WORKFLOW.md`,
  `.githooks/pre-push` and the `open-pr` and `wrap-it-up` skills. Full record in `docs/DECISIONS.md`.
  **Amended 2026-09-26:** the reviewer need not be another session, only one
  that does not share the author's context — typically a fresh subagent given
  just the PR number (see that entry above).
- 2026-09-25 **The review render's clock is PINNED, so the same command twice
  is the same picture.** `scripts/screenshot-scene.sh` passes Godot's own
  `--fixed-fps 60`; `_advance_simulation` is untouched, so the tick loop a
  player runs is unchanged and the 45-frame warm-up always spends
  `floor(45 * 30 / 60)` = 22 ticks instead of however many the host's load
  allowed (two runs used to land a tick apart — 10621 vs 10622 — and the PNG
  hash tracked it exactly). The three HUD fades `--fixed-fps` cannot reach —
  money flyout, hotbar name flyout, contract notice, outside the tick on
  purpose so they keep fading under the pause menu — read `_hud_now_msec()`,
  a clock summed from the frame deltas the scene was actually given, NEVER
  `Time.get_ticks_msec()` directly. **The camera is sealed for the same reason the
  clock is pinned:** a render aims the camera and then runs 45 ordinary
  frames, which poll the real keyboard and see the real mouse, so
  `main.gd`'s `_camera_input_sealed` (off for every player, set by the
  harness on frame 1) shuts all five doors — key pan, right-drag, zoom,
  first-person look, first-person walk. It stays a rule of the harness rather
  than a side effect of PR #167's off-screen, no-focus window, which a later
  change to that window would repeal in silence, and which `--on-screen` does
  not get. `test_screenshot.gd` REFUSES a run
  whose frames did not arrive at `FIXED_FPS` rather than quietly
  photographing a load-dependent moment, `scripts/check-invariants.sh` keeps script and
  harness in step, and every render prints `SCREENSHOT_TICK`. A baseline
  rendered before this is a different moment, not a corrupt one: re-render
  it, do not diff across the fix. Full record in `docs/DECISIONS.md`.
- 2026-09-25 **A status lamp's two states are palette data, written as
  MODIFIERS of the `emissive` class, and every renderer reads the resolved
  JSON.** A lamp is lit or dark, and `main.gd` calls `_set_machine_running(key,
  false)` the moment a body is built, so dark is the state a machine is in for
  most of its life. `palette.ron` now carries a `lamp_states` table holding
  `roughness`, `emission_scale` and `albedo_scale` per state; `voxgen` resolves
  each against the `emissive` class and writes `models/lamp_states.json` beside
  `materials.json`, which `voxel_assets.gd` and the asset browser both read.
  The states are modifiers, not absolute numbers: `emission_scale` multiplies
  the class's `emission_energy`, so that energy stays written down exactly once
  and retuning the class retunes the lit lamp with it. Absolute values would
  have been a second copy that silently stops matching. Both dimensions are
  kept — a dead lens drops emission to zero AND darkens albedo — because
  albedo is what makes the state readable in a frame where the glow pass is
  doing the talking, and it is the same channel `palette.ron` reaches for in
  its own `status_*_off` entries. Adding the table is a shape change, so
  `palette.ron`'s `schema_version` went to 2. Full record in
  `docs/DECISIONS.md`.
- 2026-09-21 **Settings are PRESENTATION; a cheat that moves money is a real
  SIM COMMAND; F3 is ungated in pre-alpha.** Eight knobs (music/sfx on and
  level, movement speed, look sensitivity, FOV, hall size) live in a `Settings`
  autoload and in `user://settings.cfg` — never in `sim_core` and never in the
  save format, so a save carries to another machine without dragging that
  machine's preferences with it, and a schema that is rejected rather than
  migrated never has to carry a volume slider. One panel
  (`settings_panel.gd`) is hosted by both the main menu and the pause menu,
  replacing each plate's button column in place. Granting money is NOT the
  same kind of thing: money is simulation state, spent through `try_spend`
  and written by `save.rs`, so `PlayerCommand::DebugGrantMoney` goes over the
  real wire and refuses `amount <= 0` byte-identically like every other
  command. Time speed IS presentation — nothing in `sim_core` knows what a
  second is — so it scales `_advance_simulation`'s interval and its catch-up
  budget together, and the panel says the in-game clocks still count sim
  seconds. The F3 panel is available in every build rather than behind
  `OS.is_debug_build()`, so a review screenshot shows what an export does;
  it is deliberately not a modal. Full record in `docs/DECISIONS.md`.
- 2026-09-21 **A review verdict is a COMMENT whose first line is exactly
  `VERDICT: APPROVED` or `VERDICT: CHANGES REQUESTED`.** GitHub refuses
  `--approve` and `--request-changes` on a pull request owned by the same
  account, and every PR in this repo is authored as `TomWhitticase`: the API
  answers `Review Can not approve your own pull request`, and
  `gh pr view N --json reviewDecision` stays `""` however thorough the review
  was. So a reviewer posts `gh pr review N --comment` and the merge gates on
  that first line, not on GitHub's own state. The review standard behind it is
  the `review` skill — own worktree, rebuild, re-run the issue's own
  reproduction, revert each fix to prove its test fails without it, count the
  `_OK` markers by hand, read the PNG; a re-review re-runs the first review's
  "what I verified" rather than diffing the rework. Full record in
  `docs/DECISIONS.md`.
- 2026-09-21 **A content id is a KEY; what a player reads is a `display_name`
  in the same file.** Renaming a thing in front of the player never renames
  its id. A machine kind id is written into every save file and looked up by
  `data/shop.ron`, `data/storage.ron`, `main.gd`'s asset tables and the voxel
  generator, so moving it costs a `SAVE_SCHEMA_VERSION` bump — every existing
  save rejected, `sim::save`'s version rule being reject-never-migrate — to
  change a string no player can see the inside of. `MachineKind` therefore
  carries an optional `display_name`, exactly as `Recipe` and
  `IngredientKind` already did, and `World::display_names` gathers all three
  into the one map `_kind_display_name` asks before it title-cases an id. The
  shipped case: `warehouse_rack` is called **"Stock Shelving"**. Absence from
  that map stays the normal case and means "the title-case fallback is
  right". Full record in `docs/DECISIONS.md`.
- 2026-09-21 **Nothing on a storage model is painted the colour of a thing
  the game stores, and its structure wears ONE paint.** The stock shelving's
  load beams were `paint_red` #82372F, the two-tone real racking is sold in,
  and its bays hold tinted stock the player is meant to read at a glance —
  most often tomatoes. A structure sharing a colour family with the goods it
  frames fights the one readout the model exists for. The beams are
  `paint_blue` #2E5772 now — the uprights' own colour, so the whole frame is
  one paint and no hue at all is spent on structure. **A beam still reads as
  a beam by value, not by colour**: its lit top face renders at 134.7 where
  the upright's vertical faces beside it are 55.7, which is the isometric key
  light doing what a second paint used to. The swap is also **value-neutral**
  — the beam pixels measure 76.4 in the rendered warehouse row where the red
  ones measured 74.7 — because every relationship the rack was tuned by
  (goods against backdrop, `RACK_TINT_DEAD_BAND`) is a relationship in value.
  A green cut (#3A6B33) measured eleven luma up and was rejected for that
  alone. Full record in `docs/DECISIONS.md`.
- 2026-09-21 **A session opens in FIRST PERSON; isometric is a TAB away.**
  `main.gd`'s `DEFAULT_VIEW_MODE` is the one place that says so, applied by
  `_ready` through `_set_view_mode` *after* `_build_ui` (the crosshair and
  the legend must exist first). The isometric camera is unchanged and still
  fixed — UI_UX.md §1 governs the angle, not which view the game opens in —
  and nothing else in the scene branches on which mode came first.
  `test_screenshot.gd` deliberately does NOT follow this: a review shot is
  isometric unless `--first-person`, because `--closeup`, `--zoom`, `--pan`
  and every `--hover*` are isometric ideas and would otherwise be aimed at a
  camera they cannot steer. Full record in `docs/DECISIONS.md`.
- 2026-09-21 **The one wall-clock assertion in the gate is scaled by how
  loaded the machine is, measured in the same process.** (Issue #146.)
  `test_perf.gd` is the only test that asserts a time rather than an outcome,
  and several worktrees run headless Godot on this machine at once: the same
  120 frames measured 350.7 ms alone and 3733.1 ms beside a neighbour's
  verify, failing a 3000 ms ceiling with no code between the two. A red gate
  that reads as a regression is worse than no gate, because the next session
  goes looking for the regression in its own diff. Raising the ceiling is the
  wrong fix — it is generous precisely so it catches an order-of-magnitude
  regression, and a ceiling loose enough to survive four concurrent runs
  would let one through. Instead the test times a fixed pure-script loop
  before and after the measurements it brackets, takes the worse reading, and
  multiplies both ceilings by it, clamped to 1.0 below (a quiet box gets
  exactly the constants) and 6.0 above (past that the machine is too busy for
  the number to mean anything). The median of the samples, never the minimum:
  a starved process still gets a full slice now and then, so its fastest
  sample looks idle however loaded the box is.

- 2026-09-21 **Raw stock has a ceiling, a rack is pure storage, and its bays
  are derived.** (W1.) `data/storage.ron` caps a holding at `base_cap` (20,
  free and unconditional, so a new game works and nothing can be crowded out)
  plus `slot_stack` (50) per **bay**; a `warehouse_rack` supplies six, the
  model's own geometry, and costs 75. **R6 wins:** a rack has no output item,
  no picker, no topology links and no tick-step-5 arm — the hopper stays the
  only thing that meters raw onto a belt. Bays are computed from the ledger
  (`ceil(max(0, count − base_cap) / slot_stack)`), never stored: no allocation
  state to persist or drift, and a bay frees itself when its stock is used.
  Free bays are therefore a **shared pool**. `BuyIngredient` clamps silently
  and charges only for what it credited; the reason is derived by presentation
  from `IngredientSnapshot::capacity`, as in #143. Fixtures build the
  warehouse they need through real commands and top it up mid-run rather than
  stocking deep, and `*_PLACEMENTS` counts exclude it.
- 2026-09-21 **A voxel asset is judged against the factory floor, not
  against vengi's black.** The two backdrops give opposite verdicts, and the
  floor is the one the player has. Repaint the pure-black pad with
  `#9C9C90` (luma 155.1) before taking any mean. What actually disappears on
  this floor is a MID tint at the floor's own value, so a legibility verdict
  is a dead BAND, not a threshold — the warehouse rack's is 100..175 luma,
  recorded as `RACK_TINT_DEAD_BAND` in the generator for W3 to read rather
  than re-derive. Also: **pay the fixed camera with spacing, not with
  deletion.** The rack's first design dropped its longitudinal load beam
  because the beam hid the load behind it; the result read as a bike rack.
  The beam stays and the levels are spaced so the sight line clears it.
- 2026-09-21 **A panel that points at the world is FOLLOWED in isometric and
  PARKED in first person.** (Issue #105.) In first person there is no cursor:
  the query point is the crosshair at the centre of the screen and the subject
  is the thing behind it, so `_panel_position_near`'s flip stands the panel on
  what it describes — at 1280x720 all four quadrants fail, the clamp comes back
  through the middle, and the dock the player walked up to is entirely behind
  its own tooltip. The first-person rule is the lower-left corner, above the
  hotbar: the one region no fixed plate claims, the one the crosshair never
  enters, and fixed rather than followed so the panel also stops sliding about
  on every mouse-look. A position standing on the crosshair is refused however
  little HUD it covers — covering a keybind reference the player can fold with
  F1 beats covering the machine they walked over to look at. Both panels with a
  world anchor (the hover tooltip, the build placement prompt) go through one
  door, `_world_panel_position`, so neither can be fixed and the other left.

- 2026-09-21 **A gated ingredient is padlocked in the contents picker, not
  hidden from it — and pointing a vessel at one is REFUSED.** The market
  omits a locked row; the picker keeps it, dimmed, with a 🔒 and a line
  naming the unlock and the deliveries still to go. Different questions: the
  market lists what you may buy now, the picker is everything this vessel
  will ever hold, and a gate nobody sees coming is one nobody plans for. The
  refusal is the simulation's (`try_set_hopper_item`, `try_set_tank_fluid`,
  the tank's landing *before* the purge), the wire carries `locked`/
  `unlock_name`/`after_deliveries` per choice, and the button stays live so
  the refusal is heard rather than swallowed by a dead control. The rule
  stops at `Place`'s `item`: that seeds a bin (the ghost, the fixtures)
  rather than choosing against a gate, the ghost never offers a locked id,
  and a bin seeded with one can hold nothing anyway. Fixtures state their
  premise with `reference::open_every_gate` instead of the sim growing a
  back door.

- 2026-09-21 **An issue is for what you cannot fix now, and the bar is no
  longer low.** Amends the 2026-09-16 GitHub rule below, whose "if it was
  worth putting in a report, it is worth an issue" worked exactly as written
  and produced a backlog of small, true, unprioritised findings a human has
  to rank and nobody picks up — its longest-lived entries were comments
  citing thirty Godot scripts against 37 and 60 Hz against a 30 Hz tick, both
  cheaper to fix than to file. **An issue nobody will ever choose to work on
  costs more than it records.** Three gates now, in order: could you just fix
  it (a line or two in a file the branch already touches, or a small safe fix
  in the area you are in, is a fix and a line in the PR body — file only when
  fixing now would be *wrong*: an owner's call, the save format, a design-doc
  contradiction, real work, or a part of the tree this branch has no business
  in); would anyone pick it ahead of the next feature (polish with no defect,
  speculative hardening, systems CLAUDE.md lists as deliberately unbuilt, and
  restatements of a documented limit all fail); is it already filed (a second
  sighting is a comment). **Budget: roughly one issue per PR** — a smell
  test, not a quota; the rest go in the PR body where review can promote
  them. Unchanged: the cold-reader body, the label set, and that a finding
  which *does* pass the gates must still be filed rather than left in a
  closing message. `file-issue`'s first section is the gate; `open-pr`, the
  PR template and `CLAUDE.md` point at it.
- 2026-09-21 **A machine kind may name several recipes, and a placed machine
  runs whichever one its inputs satisfy — there is never a picker.**
  `recipes.ron` stopped being one line per kind in the BBQ epic;
  `RecipeRegistry::for_machine` hands a kind's recipes back in file order and
  `Machine::choose` starts the **first fully-satisfied** one, so file order is
  the deterministic tiebreak and that is the only role it plays in selection.
  When none is satisfied the machine *holds* the closest one (most
  requirements met, earliest wins a tie) — nothing in the tick turns on that,
  but the stall readout is taken from `Machine::recipe` and "waiting for
  molasses" must be a true sentence about a BBQ mixer. **No player-facing
  recipe picker, and no save field for a player's choice**: the plumbing
  already decides, and a machine-selection screen is a different game's
  affordance. The in-flight recipe *is* persisted (`SavedMachine::recipe`),
  because a craft half-finished has already eaten the inputs it would be
  re-derived from. Landed 2026-09-18 (PR #45); recorded here as T5 of the
  epic. Full record in `docs/DECISIONS.md`.
- 2026-09-21 **A recipe may draw several fluid networks, matched by fluid id;
  it still deposits into exactly one.** `validate_fluid_scope` no longer caps
  `fluid_inputs` at one. What it enforces now: a recipe may not name the same
  fluid twice (one fluid is one input with the units added up, because each
  input is matched to a *different* network), and `fluid_outputs` is still at
  most one, off the outlet face — that half of the rule is unchanged and is
  not up for discussion. Each fluid input is matched **by id** against the
  networks touching the footprint, greedy in recipe order, each network
  supplying at most one input; `world.rs` now hands `Machine::tick` N disjoint
  draw borrows plus one deposit and still asserts they do not alias rather
  than assuming it. Bent because BBQ's cook needs `bbq_mix` and `liquid_smoke`
  together and no layout dodges it, and every later sauce wants it (mayo is
  oil plus acid, hot sauce vinegar plus mash). The knock-on: a run's fluid can
  no longer be read off "the one recipe its machine names", so
  `resolve_network_fluids` narrows the whole graph at once instead. Landed
  2026-09-18 (PR #45). Full record in `docs/DECISIONS.md`.
- 2026-09-21 **A progression gate is a filter on two lists, keyed to lifetime
  deliveries, and it announces itself.** `data/unlocks.ron` (schema 1) is the
  first gate in the game: an `Unlock` names a threshold and exactly two lists,
  `ingredients` held back from the market and `contracts` held back from the
  board. **There is no way to lock a recipe and no field to add one with** —
  BBQ's recipes sit in `recipes.ron` from a new game, unbuildable because
  their raws cannot be bought and unsellable because nobody is asking, which
  keeps the whole mechanism off the tick path. Both lists are enforced in the
  simulation, not the panel: `BuyIngredient` is refused (`world.rs`) and
  `SignContract` returns `SignRefusal::Locked` whether or not a row was ever
  drawn. The trigger is lifetime **successful** deliveries
  (`Economy::lifetime_deliveries`, satisfied windows only, never decreasing);
  the **counter** is saved and every gate's state is derived from it against
  the file, so a balance pass re-reads correctly for every existing save. The
  group is named because the toast needs a subject — a gate that opens
  silently is one the player never experiences. Partly answers GAMEPLAY §11's
  contract-gating question: a delivery count, no reputation meter. Landed
  2026-09-19 (PR #66). Full record in `docs/DECISIONS.md`.
- 2026-09-21 **A belt's rubber runs on that tile's own work clock, and the
  simulation is what counts it.** §6a's machine rule applies to belts:
  `BeltSegment::moved_ticks` counts the ticks a tile actually advanced
  product on, rides `BeltSnapshot`/`SplitterSnapshot`, and is what the frame
  index is taken from — so a jammed line's surface stops with the items on
  it. It lives in Rust because a stopped belt stops dirtying its chunk, so
  Godot is never told a tile stood still. A **counter**, not a "moving"
  flag: a flag re-joins a global phase and makes the rubber jump when a jam
  clears. Saved (`#[serde(default)]`, no schema bump) so a load stands where
  the save did. **Amended 2026-09-25: an EMPTY belt is a running belt.**
  `tick_toward` counts an empty tick too, so the clock stops on a jam and on
  nothing else — a conveyor with no box on it is a motor turning, not a
  starved crusher, and while still rubber meant either "jammed" or "nothing
  here" the jam could not be read off the floor at all. An idle tile still
  dirties no chunk; `main.gd`'s `_belt_idle_phase` carries the clock forward
  in the scene instead, which is exact because the value is the tick itself.
  **Amended again 2026-09-25: it IS the tick, so every running belt is in
  phase.** `tick_toward` assigns `current_tick` rather than incrementing, so
  tiles laid minutes apart move as one conveyor and only a jam takes a tile
  out of step. A tile is **laid on a tick**, not on zero
  (`BeltSegment::new`'s `laid_on_tick`, and the save's `tick_count` for an
  empty belt at load): the scene reads a placed tile once and an idle tile
  dirties nothing after that, so a clock that starts wrong stays wrong. Same
  formula, same one-per-tick rate; the accepted cost is
  that a freed tile rejoins the phase instead of resuming its own, jumping
  once, which is cheaper than a drift that never resolves. Full record in
  `docs/DECISIONS.md`.
- 2026-09-20 **A width fix must be MEASURED at every window size, and
  `snap_rows` is for rows the player signs.** Two regressions from the width
  pass above, both from a fit applied unconditionally. `_build_contract_board_row`
  split the offer sentence onto its own line always: load-bearing at 1280x720
  (unsplit 1324px against 1264px available) but not at 1600x900 (1380px in a
  1600px window), where the taller 157px card needed 489px for three against a
  484px cap and the board lost a whole offer — five pixels. `_fit_contract_board_split`
  now measures the unsplit panel against `_modal_available_width()` and breaks
  only when it must, which is the doctrine that entry already states. Second:
  `snap_rows` costs height to buy a tidy edge, so it is **only** for a stack
  whose rows are decisions the player signs — an offer, a hire — where half a
  card is half a choice. A production row is a readout; the production panel
  drops it (it was 161px short of its column at 1600x900) and joins the build
  menu in letting the half-row past the cut say "scroll".
- 2026-09-20 **The asset browser may never look darker than the game's own
  strongest honest presentation of the same geometry.** The browser's baked
  AO exists so a form cannot be approved on shading the game will not give it,
  so `AO_STRENGTH` in `tools/asset-browser/web/ao.mjs` is *measured*, never
  chosen: `tools/asset-browser/calibrate-ao.py --write` renders the game's vat
  fixture with `factory_environment.gd`'s SSAO on and off and the browser on
  the same model with AO on and off, and divides one renderer's shading per
  machine pixel by the other's. **"The game" means the game at the pose where
  it presents that geometry most strongly — standing beside the line in first
  person — not at the isometric play camera**, because the bake is
  object-space and a 0.45 m world `ssao_radius` is a few pixels wide at the
  play camera: the same vat shades 67.8% of its pixels past 1% beside the line
  and 0.3% there. Calibrating at the play camera matches the *camera* and
  ships a pass under one 8-bit level. The gap is **published, not hidden** —
  `tools/asset-browser/README.md` and the PR say which pose the constant came
  from, at which settings, and that a play-camera screenshot looks flatter on
  purpose. Every render setting is ported, not a subset: `ssao_radius`,
  `ssao_intensity` **and `ssao_power`** (a gamma on the occlusion term — a
  missing one is a shape mismatch no strength can fix). The calibration
  **fails** rather than producing a number when the browser's coverage
  diverges from the game's, when the constants are stale against
  `factory_environment.gd`, when the strength will not settle, or when the
  screenshot harness hands back a frame captured before the fixture or the
  pose was applied. Full record in `docs/DECISIONS.md`.

- 2026-09-20 **Glow means "this is emitting", not "this is bright" — and
  SSAO means "this is a crease".** The
  hall's post-pass is `glow_intensity 1.0`, **`glow_bloom 0.0`**,
  `glow_hdr_threshold 1.2`, **SCREEN** (`factory_environment.gd`). Tune it
  only against rendered frames in which something actually emits, and judge it
  by *selectivity* — what the pass adds around a lit lens over what it adds to
  the rest of the frame. Two rules for measuring it, both learned the hard
  way: the null baseline is glow **enabled** with **`glow_bloom = 0.0` and a
  threshold of 3.0** (enabling the pass at all shifts the frame ~+4.5 mean, and
  a high threshold alone still leaves `glow_bloom`'s frame-wide haze in the
  baseline); and it could not be measured through
  `scripts/screenshot-scene.sh` at the time, whose sim state was not then
  reproducible run to run — since 2026-09-25 the render's clock is pinned and
  it can be. `glow_bloom` is frame-wide
  haze by definition and belongs at 0 in a game whose emissives are small
  lamps. **SOFTLIGHT can never make a halo** — its effect scales with the pixel
  beneath it, so its brightest pixel is +25..30 levels at any intensity or
  threshold, against SCREEN's +213. The old 0.7 / 0.15 / 0.85 / SOFTLIGHT was
  tuned while issue #25 meant nothing in the game emitted at all, and scored
  2.7x selectivity against the present 69x.

  The **SSAO** in the same block had the same defect and was retuned with it:
  `ssao_radius 0.45`, `ssao_intensity 16.0`, `ssao_power 1.6`, from
  `0.9 / 2.4 / 1.6`. At the old numbers the deepest crease in an isometric
  closeup lost 1.2 levels out of ~130 — invisible (measured by the asset
  browser's worker on issue #49, and confirmed here against
  `ssao_enabled = false`). **Intensity alone cannot fix it**: `ssao_radius` is
  in WORLD units, so an eye-level camera finds far more occlusion per pixel
  than the orthographic camera looking down from across the hall, and
  `0.9 / 16` reads well in isometric while swallowing the belt's legs in a
  smear in first person. Halving the radius is what reconciles the two views —
  0.45 world units is 4.5 voxels, *a fitting against a body*, not a machine
  against the floor. Judge a change to it in **three** framings (isometric
  closeup, play zoom, first person standing on the line), and watch the share
  of the frame darkened by more than 3%: that is where overshoot shows up
  first. A first-person shot aimed across the empty hall shows no SSAO at any
  setting and proves nothing — there is no geometry near the camera.
  Full record in `docs/DECISIONS.md`.
- 2026-09-20 **A tank declares what it holds, and the save format's
  field-addition window is closed.** `FluidTank` carries an optional declared
  fluid, set by `PlayerCommand::SetTankFluid` and **seeded into**
  `resolve_network_fluids` rather than recorded beside its answer: a declared
  tank's run is a singleton before any narrowing runs, which is the only thing
  that can tell two tank-fed raw runs into one two-fluid machine apart (issue
  #43). An undeclared tank is unchanged — its fluid is still derived from the
  run it discharges into — so the shipped chain needs no labelling. Two
  contradicting declarations on one run leave it `Ambiguous`; the resolver
  still never guesses. Because the declaration is an *input* to naming,
  `World::from_save_with_content` recomputes the topology after restoring
  bodies. The field bumped **`SAVE_SCHEMA_VERSION` to 2** and a v1 save is
  **refused, not migrated**, with both numbers in the message: the "a
  `#[serde(default)]` addition is not a shape change" window `save.rs` opened
  closed when that batch shipped, and this crate rejects rather than migrates
  everywhere else (ARCHITECTURE §4). Cost is bounded because no release has
  ever written a save. **Amended in review of the pull request:** changing a
  declaration destroys the **pipe run's** fluid as well as the tank's
  (`World::try_set_tank_fluid` purges every run the tank discharges into
  *before* the recompute, which carries volumes across by tile and would
  otherwise hand sixty bought units of vinegar back as liquid smoke). The
  fluid is destroyed, not credited or moved, and the picker's note names the
  total it will destroy before the click — declaring is a consented
  destructive act, not a refused one, because refusing it while the run is
  non-empty leaves a mis-declared tank with no way to drain its run. And a
  declaration no machine on the run has any recipe for makes that run read
  `Unnameable` rather than `Carries`, so the blame lands on the tank the
  player mis-declared instead of on the machine's outlet. Full record in
  `docs/DECISIONS.md`.
- 2026-09-20 **A modal panel yields WIDTH before it is clipped, and an
  unreachable control outranks every other layout rule.** Height has yielded
  since the scale change; nothing made a panel yield width, so at 1280x720 the
  staff panel (1350px), the build menu (1412px) and the contracts board
  (1380px) overran the window and were clipped on both edges with FIRE −1 and
  SIGN off the screen. UI_UX §5f's ranking gains a new worst fault above "a
  panel drawn ON another panel": **a control the player cannot reach**. Every
  modal now measures `_modal_available_width()` (the viewport less
  `MODAL_EDGE_MARGIN`, in window pixels, not `_ui_px`) and gives width back in
  this order: **(1) prose moves to the row's hover** (`_fit_staff_width`,
  `_apply_staff_prose` — the ranked sentence, the per-tick figure and the
  meter caption, all already readable on the tooltip); **(2) a grid of cards
  takes fewer columns** (`_fit_grid_columns`, measured by asking the panel one
  column at a time, then balanced so six crates are 3+3 and not 5+1;
  `SHOP_SLOT_COLUMNS`/`MARKET_SLOT_COLUMNS` are now the MOST a grid takes, not
  what it always takes); **(3) the list scrolls**, as it already did. A
  sentence too long for one line is **broken at a join it already has** (the
  board's mix and terms are two labels), never reflowed: **no `autowrap_mode`
  on a Label whose width a fit pass measures** — a wrapping Label reports a
  1px minimum width and a height shaped at whatever width it was last *drawn*
  at, and this layout is measurement-driven with a headless test for its
  proof. Where the width is pinned first — by a fixed-size ancestor or by the
  Label's own `custom_minimum_size.x` — the minimum decides nothing and wrapping
  is fine; that is the case at all 14 live sites. Both `x` and `y` are
  clamped everywhere a modal is positioned, and the market is pushed clear of
  the clipboard, STOCK and CREW plates (`_plate_rect`, `_pushed_clear_of`).
- 2026-09-20 **A headless layout test cannot prove a real width; only
  `scripts/screenshot-scene.sh` can.** Godot's headless font metrics are
  NARROWER than a real window's: the staff panel measured 1264px available
  against a 1339px minimum headless, passed `test_hud_plates.gd`, and was
  still clipped in the rendered frame at the same size. Headless assertions
  are the regression net; the render is the acceptance. For the same reason a
  Control's `get_combined_minimum_size()` may UNDERREPORT what is on screen
  (the machine hover reported 299x285 while drawn at 545x285, and was placed
  as if its right-hand 246px did not exist), so anything positioning a panel
  takes `maxf(minimum, panel.size)` on both axes. And where no position clears
  every fixed plate, `_panel_position_near` scores each candidate by total
  overlapped area (`_hud_overlap_area`) and keeps the smallest, ties going to
  the earlier preference — pushing off one plate at a time can land on the
  next, and did.
- 2026-09-20 **A panel that dodges the right-hand column consults ONE list of
  plates, after its own clamp.** `_position_config_panel`'s right-hand bound
  named the clipboard and STOCK in two hand-written branches; the CREW plate
  arrived on that column later and was never added, and the bound was computed
  against an unclamped `y`, so a tank's panel — taller than the band below the
  clipboard — was lifted by the clamp into rows a test against its old `y` had
  decided were nowhere near it (issue #88). It now clamps `y` first, loops
  `[_contract_panel_rect(), _stock_rect(), _crew_rect()]`, and flips the panel
  so its right edge lands ON the bound rather than at a fixed offset from the
  anchor, which is not the same as clearing anything. The legend is
  deliberately not on that list: at 1280x720 nothing clears both it and the
  column, and a static keybind reference the player can fold with F1 is the
  right thing to cover in preference to a live readout — the same ranking
  `_position_market_panel` makes. Asserted headlessly by `test_hud_plates.gd`
  with the camera looking AT the building, since a building off the side of
  the frame puts the panel on a window edge and proves nothing.
- 2026-09-20 **The clipboard's board hint is a line the docket stack may
  spend.** `_update_contract_panel` — the one place that knows what the stack
  got — hides `_contract_hint_label` when, and only when, the cut it made is
  below the stack's first line boundary (`_first_line_boundary`); that line is
  what buys the cut back onto a boundary at 1280x720 with every offer signed,
  the case issue #17 had left ragged. `_update_staff_alert` no longer touches
  the hint.

- 2026-09-20 **The bank at the moment of building decides whether the BBQ
  branch costs you the board.** `data/unlocks.ron`'s `after_deliveries: 10`
  stays where it is, and the reason is measured rather than argued — the
  file's own comment carries the table AND the protocol each row was run
  under, because "how many contracts were signed, and when" changes every
  number here. Building the BBQ branch stops boxed-ketchup output for ~8 500
  ticks while the new pipes and vat prime off the existing fluid network, and
  the contract board keeps counting windows through it. Whether that costs a
  contract is decided by **how many finished boxes are banked at the moment
  the player builds**, not by how many orders are running: three orders
  signed at the gate (bank 22) lose one of three, the same three signed at
  tick 0 (bank 7, because the early windows ate the stock) lose two to the
  branch, and one order (bank 22) loses none. **A delivery count controls
  that bank only by proxy, and badly** — signing more contracts is both how a
  player reaches the count sooner and what makes the transient cost more.
  Pushed far enough it does work: an emulated gate of 40 deliveries (bank 24)
  loses nothing to the branch with three orders signed. But 40 is not "inside
  a first session", which is the whole reason ten was chosen, so **do not
  answer "expansion starves the line" with a number in `unlocks.ron`** —
  issue #55 stays open, and the fixes it wants are a warning at placement
  time and a fluid network that will not hand a brand-new consumer 100% of a
  network the old one lives on (issues #92, #93). Full record in
  `docs/DECISIONS.md`.
- 2026-09-20 **A legibility mask must be proved to contain the pixels in
  question, before any number it produces is quoted.** The
  fixed-mask-mean-luma method (`voxel-legibility-measurement`) is only as
  honest as its silhouette mask: judging `box_of_ketchup` against
  `box_of_bbq` with the warm-pixel mask the earlier passes used
  (`r > g > b and r - b > 18`) silently dropped the printed panels, because
  `label_cream` renders near-neutral at (241, 239, 234), and reported a
  confident 5.3/255 for art that in fact separates by 39.5/255. **The mask is
  the case's whole silhouette with interior holes filled** — largest warm
  connected component, then flood the complement from the window border and
  keep what the flood cannot reach — because the kraft outline encloses the
  print on every face the camera sees. **Validate a mask by measuring a
  candidate that visibly changes the art; if it scores the same as shipped,
  the instrument is broken, not the candidate.** The measurement it now
  supports: the shipped lid print separates the two cartons on a shared belt
  by 39.5/255 luma (`bbq_mixed.png`, windows (964,561) and (1023,538), 26x26;
  independently reproduced at 42.5 by review), so **#40's ruling stands and
  the sauce cartons are not repainted** — and chroma is the weaker channel at
  this size, not the stronger one. **A control must be the same subject kind
  as the subject, and every window's coordinates must be logged.** The first
  pass judged that gap against a 12.6 "noise floor" measured on cases
  standing BACK TO BACK in the packer queue — a different pixel population
  from one case alone on dark belt — and quoted a ratio from it: that floor
  and that ratio are **withdrawn**. The same-kind control is one single case
  on the shared belt across 14 renders, which spreads 5.0/255. Full record in
  `docs/DECISIONS.md`.
- 2026-09-20 **Mothballing is the way out of arrears, and it STANDS CREW
  DOWN rather than only shrinking the establishment.** A placement can be
  switched off (`PlayerCommand::SetMothballed`, wire `set_mothballed`). It
  drops out of the ESTABLISHMENT, produces nothing, and HOLDS the batch it
  was part-way through rather than losing it (`Machine::tick` advances
  `start_tick` in step with the clock). **Crucially it also moves money**:
  `StaffPools` computes `stood_down = min(headcount - required, the
  mothballed requirement)` and `on_payroll() = headcount - stood_down`, and
  `bill_per_period` sums the heads ON THE PAYROLL, so the bill falls the
  tick the switch is thrown without firing anybody. `next_bill` follows
  `owed`, which `sync_payroll` only ever RAISES — a head already owed for
  this period stays owed, so mothballing on payday eve buys notice pay, not
  free labour — and only payday lowers it. **A stood-down head never walks
  out**: `left` is capped by `on_payroll()`. That is the whole reason to
  prefer mothballing to firing, which loses a trained crew the factory then
  has to re-hire.
- 2026-09-20 **A placement may be mothballed iff its kind asks for a crew.**
  The test is a non-empty `staff` list in `data/machines.ron`, not a Rust
  type check — content decides, as content decides everything else about a
  kind. This **reverses** the first round's "only a `Machine` can be
  mothballed": the DOCK asks for a logistics clerk, so it is switchable, and
  it has to be, because while it is on, `required("logistics")` never
  reaches 0 and a factory with no income can never get its bill to zero. A
  hopper, a tank, a belt, a pipe and a splitter ask for nobody and are
  refused byte-exactly. The same rule drives Godot: `_config_can_mothball`
  asks whether the polled crew list is non-empty rather than matching a
  hardcoded kind list. Mothballing is not a fourth `WorkState` — the window
  it did no work in is honestly a starved window, and `StallKind::Mothballed`
  carries the why. It persists as `SavedMachine::mothballed` and
  `SavedDock::mothballed`, both `#[serde(default)]`, **joining
  `schema_version` 2** with issue #43's `SavedTank::fluid` rather than
  claiming a version of their own. No new `data/*.ron` number: the figures
  that govern it (`wage`, `pay_period_ticks`) are already data. The surface
  is the click-to-configure panel; **not** a keybind — Q, SPACE and V were
  retired for acting on a building the player could not see. What
  mothballing does NOT do is conjure income: a standing order lost to three
  missed windows during a shutdown still costs money to re-sign, which is
  issue #85 and stays open.
  > Full record in `docs/DECISIONS.md`.

- 2026-09-20 **`crewed` on a placement is the ESTABLISHMENT's answer, not a
  replay of the last tick.** `PlacementSnapshot.crewed` and
  `MachineLoadEntry.crewed` come from `World::uncrewed_placements`, which
  hands each pool's `headcount` out in the order the tick claims in
  (`topology.machines`, then `topology.docks`, whole crew or nothing) and
  names whoever goes without. It deliberately does **not** read what the
  last tick did: the tick only claims for a machine that would otherwise
  have worked, so a per-tick reading flickers as the line ebbs — which is
  the exact flapping the 2026-09-16 establishment change existed to end.
  It is derived, so no save-schema change; `crewed` sits BEFORE `links` in
  `PlacementSnapshot`, because `links` stays last and `tests/links.rs` pins
  that shape byte for byte. Godot shows it in the hover's load line, in the
  same words a `staff` stall gives (`_uncrewed_reason_text` and
  `_stall_reason_text` share `_crew_categories_text`), and by outlining the
  load bar amber — the track's BORDER, not its fill, because an uncrewed
  machine does no work and has no fill left to colour. **No in-world pole**;
  the hover panel is still the one place status lives.
  > Full record in `docs/DECISIONS.md`.

- 2026-09-16 **The game is called CONDIMENTAL, and so is the repo.** All-caps
  wherever it is the title: the menu's `title.text`, `config/name`, the
  design-doc headings. The GitHub repo was renamed the same day —
  `TomWhitticase/condimental` — and every current-tense link was swept with it;
  GitHub redirects the old paths, so nothing held a dead URL in between.
  `sauce-factory` survives *deliberately* in three places, so **do not sweep
  them**: **dated history** in DEVLOG.md and DECISIONS.md, the archived
  `*_PLAN.md` files (including the worker prompts quoted verbatim), and the
  **local checkout and worktree paths** (`orca/projects/sauce-factory`,
  `orca/workspaces/sauce-factory/*`) — live worktrees and running sessions
  point at those, and renaming a directory out from under them breaks them for
  no gain. "Sauce" stays the theme and the noun in every recipe. `user://`
  resolves through `config/name`, so saves moved to `app_userdata/CONDIMENTAL/`
  and pre-rename saves are not migrated.
- 2026-09-16 **Nothing is washed over the title screen's hall.** The main
  menu's gradient scrim is **removed**: no full-screen tinted overlay may sit
  between `title_scene.gd`'s backdrop and the menu column, asserted in
  `test_main_menu.gd`. Legibility over the drifting 3D frame is bought by the
  text carrying its own edge — `menu.gd`'s `TITLE_OUTLINE` (14px at
  `UI_SCALE`) on the wordmark and `STRAP_OUTLINE` (8px) on the tagline and
  splash line — and by the button plate already being near-opaque
  (`PLATE_BG`). This reverses the scrim clause of the centred-menu entry
  below; the centring and the camera framing stand.
  Full record in `docs/DECISIONS.md`.

- 2026-09-16 **An emissive voxel emits its OWN colour, through the one
  shader in the pipeline.** `emissive` is built as a `ShaderMaterial`
  (`godot_project/voxel_emissive.gdshader`, `EMISSION = COLOR.rgb *
  energy`); every other material class stays a `StandardMaterial3D`. The
  reason is that colour in this pipeline lives ONLY in the mesh's vertex
  colours, and `StandardMaterial3D`'s emission can come from a flat colour or
  an emission texture and from nothing else — so the `EMISSION_OP_MULTIPLY`
  it was asked for multiplied by an absent texture and **nothing in the
  factory emitted at all** until this was fixed (issue #25). A flat additive
  colour is not the alternative: a single `emissive` submesh carries more
  than one lens colour (the crusher's green and amber), so one additive hue
  washes them together. `emission_energy` was retuned 2.6 → **1.2** at the
  same time, because 2.6 had never rendered anything and at 2.6 a lit amber
  lens burns through to cream — which loses exactly the hue the shader is
  for. A lamp's OFF state still moves **albedo** as well as energy
  (`LAMP_OFF_ALBEDO`); that is what carries the dock's full-pool blink in a
  frame where the glow pass is what the eye reads.

- 2026-09-16 **Staffing is an establishment, and the crew has a plate.**
  **The staffing requirement is the sum of the crews of every PLACED
  machine**, running or not — recomputed only in `World::recompute_topology`
  (place/remove/rotate) and again after a load, never per tick. `short` on
  the wire means `required - headcount`: "hire this many", and it holds
  still. The per-tick `demand`/`assigned` figures are kept and still shown,
  demoted to "what is working right now". This contradicts GAMEPLAY §7 as
  written ("*while actively running*"), which is amended with it: the old
  model made the hire advice move with the belts, so it had changed by the
  time the player acted on it. No save-schema bump — `required` is derived.
  Per-tick claim gating is untouched; the two models agree whenever the
  factory is properly staffed. The **CREW plate** on the right-hand column
  makes staffing legible with no key pressed, and it takes the SHORT-STAFFED
  alarm and the wages line OFF the clipboard: the clipboard keeps only the
  wages WARNING, beside the balance it is about. The right-hand column's
  order of yielding is now clipboard scrolls → stock scrolls → CREW drops
  its detail line (bounded content, so never a scrollbar). `short` is
  computed against `headcount`, never `available`: a head in arrears is not
  working, but hiring a second head to cover them is the wrong advice —
  paying them is.
  The plan's per-row meter is dropped: `required` is a legitimate §5e
  ceiling, but neither a 50px track nor a full-width one fits a 342px column
  already at its height cap — `2 / 4` beside `▲ HIRE 2` says the same thing.
- 2026-09-16 **The hall is drawn in first person only; the isometric view
  gets its air.** Owner's playtest ("when in isometric view the skybox is
  messed up… decide what to do with it"). A sky is found by intersecting a
  box against `EYEDIR`, which needs rays that fan out; an **orthographic
  camera's rays are parallel**, so the whole screen sampled barely a degree
  of direction and two or three roof bays were magnified into hard stripes
  fanning from the corners, sliding as the player panned. It is not a shader
  bug — a painted infinite room cannot be photographed with a parallel
  projection. `factory_environment.gd`'s `set_backdrop_for(env, first_person)`
  is now the one place that decides: `BG_SKY` in first person, `BG_COLOR` at
  `HAZE_COLOR` (the very colour every distant surface in the shader fades to)
  in isometric. **Only the background changes** — the `Sky` stays on the
  environment and stays the ambient and reflection source in both modes, so
  the factory is lit identically either way and TAB changes what is BEHIND
  the world, not the world. `main.gd` calls it from `_apply_backdrop_mode`,
  at build and on every TAB; the title screen is perspective and keeps the
  sky. **Also: the main menu's column is CENTRED**, reversing the previous
  entry's "a column set to one side of the vanishing point" — the owner asked
  for it centred, the scrim became a symmetric four-stop gradient, and
  `title_scene.gd`'s camera was re-framed around it (tomatoes in on the left,
  bottles out on the right, plate over the middle), and **the title screen
  gets clear air** — `build_into(parent, haze_density)` with
  `TITLE_HAZE_DENSITY = 0.0`, the one parameter that differs between the two
  callers. Haze is atmosphere over a play session; in a single still frame
  under a wordmark it is a grey wash on the picture. The game keeps
  `HAZE_DENSITY`.
  Full record in `docs/DECISIONS.md`.

- 2026-09-16 **The main menu is a title screen: the factory itself, hand
  composed, behind the menu.** Owner's brief ("a splash screen similar to
  minecraft/factorio using our assets"). This **reverses** the earlier "flat
  charcoal plate is enough" call, and answers its argument rather than
  ignoring it: `godot_project/title_scene.gd` loads NONE of `main.tscn` and
  creates no simulation — it is ~20 hand-placed models of the shipped chain
  on the game's own floor, in the §3 hall, with a perspective camera that
  drifts on wall time. It is art, not a sim: nothing ticks, and the items on
  the belts were placed by hand. To stop the poster drifting from the game,
  the shared parts were EXTRACTED rather than copied — `voxel_assets.gd`
  (loading, materials, meshes), `factory_environment.gd` (sky, environment,
  lights), and `main.gd`'s floor helpers made `static`. The wordmark,
  subtitle and a date-picked tilted splash line stand outside the plate, over
  a gradient scrim, in a column set to one side of the vanishing point. A
  list inside the menu is now capped by `_list_height_budget` — the window
  less the measured chrome — not by a constant.
  Full record in `docs/DECISIONS.md`.

- 2026-09-16 **The backdrop is the inside of a factory hall, not the
  sky.** Owner's brief. `godot_project/factory_interior_sky.gdshader` is a
  sky shader that ray-marches nothing and intersects one box centred on the
  viewer — roof deck, trusses, skylight ribbons and high-bay lamps overhead;
  clad walls with a pipe-rack gantry, clerestory glazing and a line of
  distant plant; a concrete slab below; all fading into air haze. It is at
  infinity and has no parallax, so the hall is the same size wherever the
  player walks: that is what makes it read as massive rather than as a room.
  Colours are `palette.ron`'s, passed in from `_build_environment` as
  `source_color` uniforms. The key light is now lamp-white and the fill
  daylight-cool, because with no blue sky left to balance the old warm sun
  every concrete surface in the game photographed beige. Do not put the
  outdoor sky back, and do not build this as geometry.
  Full record in `docs/DECISIONS.md`.

- 2026-09-16 **Four playtest asks: a full dock, pulp off the crusher,
  SHIFT-to-rebuy, and the first-person camera's entry pose.**
  **A dock can be full**, and the ceiling is on the shared delivery POOL,
  **per item id** — `DELIVERY_STOCK_CAPACITY` (48) is the number the player
  already watches, and a single total would deadlock the game (one
  overproduced item crowds the others out, a mix naming a crowded-out item
  becomes unsatisfiable, and nothing dumps stock). The number is pinned as
  a *relationship* by `tests/dock.rs` — comfortably over the board's
  largest single mix quantity — never as two literals. `try_absorb` asks
  the economy for room before taking a box off the belt, so a full pool
  backs the line up through the dock's existing refusal path; **the
  backed-up line IS the consequence**, no penalty on top. An over-cap save
  is `SaveError::OverfullDeliveryPool`, not a truncation. The clipboard's
  ON THE DOCK line now earns UI_UX §5e's bar (`20/48`, red plus `⚠ FULL —
  belt backing up` at the cap); on the floor the full dock **pulses** its
  deck lamps (`DOCK_FULL_PULSE_TICKS`, off `sim.current_tick()`), because
  emission MULTIPLIES vertex colour (§2b point 8) so a red lamp is
  impossible here — **a world-space warning is a rhythm, not a colour.**
  **Amended 2026-09-21:** that pulse is per **dock**, off
  `DockSnapshot::full` (`Dock::blocked` — this dock refused a box this
  tick), **not** per pool: `Economy::delivery_pool_full` is an `.any()`
  over the item ceilings, so one capped sauce pulsed every dock in the
  world, including ones shipping another sauce into room. The `.any()`
  stays for the clipboard, which shows the per-item levels beside it.
  **Superseded 2026-09-26:** the pool is gone; each dock holds 32 places
  of its own (see that entry). The lamp pulse and the clipboard bar stand.
  **The crusher throws pulp, and a BEAT is a third size of feedback**
  beside §6a's two: five `crush_bit` flecks every `CRUSH_BEAT_TICKS` of
  the machine's own WORK ticks, so a starved or blocked crusher throws
  nothing (§2b point 28's "working" for free) and the spray is another
  statement of what the mechanism and the lamp already say. Varied by
  index, never `randf()`; measured in `TILE`; **no sound** (§8 keeps "no
  idle machine loop", and a cue every ten ticks is that loop). Tuned by
  rendering, not by plan: three flecks read as stray pixels, five varied
  as a spit, and a higher arc as debris off the floor.
  `screenshot-scene.sh --crushing` waits for the *state* and freezes it,
  since the obvious fixtures photograph a blocked or starved crusher.
  **SHIFT buys back what the click places** — a modifier on *placing*, so
  the purchase happens after the placement and only if it landed; the
  prompt names it and its price in all three states, and an unaffordable
  SHIFT warns before the click rather than buzzing after it.
  **Entering first person places the first-person camera:**
  `_set_view_mode` ended with an unconditional `_update_camera_transform()`
  (the isometric placement) after a branch that had already placed the
  camera, so first person kept the isometric pose until the first input.
  **When two modes each own a transform, the switch must not have a third
  opinion.** Full record in `docs/DECISIONS.md`.

- 2026-09-16 **Every pull request body embeds a screenshot, no
  exceptions.** Owner's rule. A PR here reviews a frame, so it shows one;
  "visual changes only" was rejected because the conditional version rots
  into an author deciding their change is not visual. `gh` cannot attach a
  file and the rendered PNGs are gitignored, so
  `scripts/pr-screenshot.sh` pushes them to **`pr-media`**, an orphan
  branch never merged into `main`, and prints a
  `github.com/<owner>/<repo>/blob/pr-media/…?raw=true` link — the
  `raw.githubusercontent.com` form renders broken in a private repo
  because it needs a token. Enforced twice, since a rule that only asks is
  a rule a hurried session skips: a `PreToolUse` hook
  (`.claude/hooks/require-pr-screenshot.sh`) refuses `gh pr create` with
  no image in the body — so `--fill` alone no longer opens a PR — and the
  `PR screenshot` CI job re-checks the body and that every link resolves.

- 2026-09-16 **Five HUD features, built in parallel: staffing made
  unmissable, buy-all in the market, a right-hand STOCK plate, a
  collapsible legend.** Owner's brief, three parallel worktrees (staff /
  market / HUD layout), **no Rust change** — the quality pass had already
  put `short`, `can_hire`, `next_bill` and `shortfall` on the wire and
  `buy_ingredient` already took a quantity. Agents were forbidden `.md`
  files: three sessions' doc edits all land in the same logs, so docs are
  the orchestrator's job after the merge. **Being short is said at three
  distances:** a SHORT-STAFFED strip on the always-visible clipboard (`▲
  SHORT-STAFFED — hire 2 manufacturing · press H`, a child of the
  clipboard's body so it can never land on another plate, naming only the
  categories actually short and hidden entirely when none are — never `0
  short`), a one-sentence verdict above the panel's first row, and a `▲
  SHORT` chip plus warn border on the row, with a line saying what hiring
  would unblock. **A button names its cost and a disabled one says why in
  words** (`HIRE +1 · 2 / period`; `✗ cannot hire — 8 in hand, 14 due, 2 /
  period`), all three figures the wire's own. The payday rules are stated
  in plain words at the foot of the SCROLLED list, not under the footer —
  `_fit_grid_scroll` caps against the panel, so four lines of chrome cost a
  whole category row (UI_UX §2b point 32). **SHIFT+click a market crate
  buys every unit the balance covers as ONE command** (`cost_of` is
  all-or-nothing, so no partial purchase can exist), the quantity printed
  on every slot (`shift: buy 123`, dimmed `shift: buy 0`) because an
  invisible gesture is an unused one, and a slot is recessed only when
  NEITHER verb can act. **Stock is a right-hand plate, one row per priced
  raw ingredient**, replacing the one-line larder strip that was one line
  tall by construction; nothing on the hotbar shows stock any more, and
  zero is the word `none` on a dimmed row (UI_UX §5e: no bar, nothing
  defines "full"). The right-hand column arbitrates both ways, and **a
  reserved height must be what a plate WANTS, not what it has** —
  reporting the settled height let the clipboard's cap and the plate's cap
  chase each other down to one row at 1280x720; the clipboard yields
  because a docket stack scrolls by design and this readout does not. **A
  fold is not a modal:** the legend folds to a labelled stub (heading + a
  button naming F1), toggled by the button or F1 in `_unhandled_input` so
  it works in both camera modes, outside the one-modal rule, session-only
  state, and everything that avoids the legend reads its CURRENT rect so
  the space really comes back. Reported not fixed: the staff panel shows
  ~2.5 of 4 rows at 1600x900, and a busy clipboard beside the strip clips
  its last docket line. Full record in [DECISIONS.md](DECISIONS.md).

- 2026-09-16 **GitHub is the workflow: every change lands by pull request,
  every deferral becomes an issue.** Nothing is committed or pushed to `main`
  directly — work goes on a `<type>/<description>` branch, `gh pr create`
  opens the PR, `gh pr checks` must be green (and, since 2026-09-25, a
  `VERDICT: APPROVED` review must be posted — see that entry above), and `gh pr merge --squash
  --delete-branch` lands it (squash by default, so the squashed subject is the
  one Conventional Commit and is linted like any other; a real merge commit
  only where each commit is a reviewed unit). `.githooks/pre-push` **blocks** a
  direct push to `main` — mechanical, unlike the `pre-commit` DEVLOG reminder,
  which is a judgement call and only warns. Do not merge a PR that is red,
  still running, requested for review, or that deviates from a design doc,
  bumps `gdext` or changes the save format — say it is ready and stop.
  *(Amended 2026-09-26: the review still runs in those cases; only the merge
  stops — see the entry at the top of this log.)*
  **Deciding something is out of scope is only half the decision; filing it is
  the other half** *(the bar was raised on 2026-09-21 — see the entry at the
  top of this log)*: `gh issue create --label deferred`, with file *and*
  function, how it was found, why it was deferred and what the fix probably
  is, because the next agent starts cold and a finding reported only in a
  closing message is lost. Labels are deliberately few: `deferred`,
  `owner-decision` (blocked on a human call — a gameplay change, a design-doc
  deviation, anything invalidating saves) and `area:sim|godot|assets|docs|
  tooling`. Issues are the **backlog only** — a rule still goes to this log,
  a milestone to `docs/DEVLOG.md`, a plan to a doc in `docs/`. Skills
  `open-pr` and `file-issue` are the procedures; `docs/WORKFLOW.md` has the
  rule. Full record in `docs/DECISIONS.md`.

- 2026-09-16 **The agent context is a map, not the manual.** `CLAUDE.md`
  `@`-included the three design docs and carried this log plus a per-file
  project structure, so every single request paid for ~2,000 lines
  (~34,500 tokens) whether or not any of it applied. An `@`-include is
  inlining. The root file is now 72 lines: the project in a sentence, the
  rules that genuinely always apply, a directory-level layout, and a table
  from "what you are working on" to "which doc to read". This log, the
  workflow material and the design docs are **linked, not included**;
  `sim_core/`, `godot_project/` and `assets/voxels/` each have their own
  `CLAUDE.md` of local invariants, loaded only when work happens there;
  `AGENTS.md` is a **symlink** to `CLAUDE.md` in each of those places, so
  the two can never drift (an untracked 160 KB flattened copy had already
  drifted); and repeatable procedures are **skills** under
  `.claude/skills/`, whose bodies load only on invocation. The rule: put
  something in an always-loaded file only if it would be wrong for the
  agent not to know it on a turn that has nothing to do with it —
  everything else is a link, and a rule added because of one past mistake
  does not earn a permanent seat. Corrections found while verifying the
  move: ARCHITECTURE.md §3.9's tick rate is **30/s, not 60** (`TICK_HZ`
  has been 30 since milestone 3, and it is the one place the rate is set),
  which halved two derived seconds-figures; there are **thirty** headless
  Godot scripts, not nineteen; `screenshot-scene.sh`'s resolution argument
  is **positional**, so a flag in `$2` is silently eaten; UI_UX.md §2b's
  three duplicate point numbers became **35** (icon-pass camera), **36**
  (the floor's value range) and **37** (N stamps, per-instance rotation),
  taking the next free numbers so every existing citation stays correct. Full
  record in `docs/DECISIONS.md`.
- 2026-09-15 **Pipes & conveyors overhaul, ticket 7 — the close-out, and
  what a close-out may not decide.** The galleries were clean; what the
  bug hunt found was around the work. **`cargo xtask` now runs `godot
  --import` itself, twice** (staging deletes the `.import` sidecars and
  the `.vox` importer registers only on the second pass, UI_UX §2b point
  4), degrading to a printed reminder when godot is not runnable so a
  `cargo test`-only CI box stays green; `--no-import` opts out. A step
  that must be run twice by hand by someone who already knows about it is
  a trap, and it caught this ticket. **The "known benign" `Parameter
  "material" is null` line was real:** freeing an instance still holding a
  `material_override` emits it, so `_release_render_resources` drops mesh
  and material off BOTH instance kinds (`MultiMeshInstance3D` and the
  `MeshInstance3D`s `_asset_node` makes) at every one of the six places the
  scene frees something that could hold a mesh — clearing only the first
  kind, or only the two subtree frees, each left one occurrence behind
  (the last was the ghost body, whose tint is a `material_override`). The line surfaces on the
  rendering server's flush, not at the `free()`, so marker prints around
  the frees point at the wrong place; `test_placement.gd`'s header now
  calls it a regression, not scenery. **`scripts/make-demo-saves.sh` +
  `make_demo_saves.gd` rebuild the four demo saves** from
  `test_layouts.gd`'s builders through real Purchase/Place, each warmed
  600 ticks and proven to LOAD before it is written; they live in
  `user://saves/`, since the committed fixture is the builder, not its
  output, and a schema bump rejects (never migrates) every existing save.
  **One ghost rule, not two** (UI_UX §2b point 34): *a ghost previews what
  the connection rules already make certain and never what the simulation
  still has to decide* — the pipe ghost takes shape from adjacent pipes
  (fluids connect by touching), the belt ghost stays straight (a belt's
  hand-off is `resolve_sink`'s answer), and the pipe ghost still previews
  no machine/tank fitting. No behaviour changed. **Deliberately left to
  the owner:** `resolve_sink` feeds a machine from any side of an
  inlet-edge tile and counts a belt on its OUTLET face as feeding it —
  measured, and contrary to both `world.rs`'s module doc and GAMEPLAY §9,
  so the code is the outlier (`tests/links.rs` pins it on purpose). The
  fix is one condition (`belt.direction == sink.direction`) but it is a
  gameplay change that breaks `build_demo_line` and existing saves. Also
  reported not fixed: nobody has watched the belt animation live
  (`BELT_FRAMES`), `_read_world_layout` is ~40 of a rebuild's 80 ms
  (milestone 8), and every perf figure is off a debug build. Full record
  in `docs/DECISIONS.md`.
- 2026-09-15 **Quality pass, ticket 5: polish sweep and docs — closes out
  the quality pass.** Every screen walked at 1600x900 through
  `scripts/screenshot-scene.sh` and every PNG read; five fixes, of which
  the first two were left for this ticket by name in ticket 3's notes.
  **A panel never lands on another plate** (`_panel_position_near`,
  rewritten, plus `_pushed_clear_of`): it used to flip only off the right
  and bottom *viewport* edges, so a hover or config panel could sit
  squarely over the legend, over the contracts clipboard, or under the
  hotbar (which is not part of the visible rect). It now tries all four
  quadrants around the cursor and takes the first that clears the
  viewport, the hotbar's reserved height and every fixed plate, then
  falls back to clamping on screen and *pushing* clear — picking a
  direction that survives the clamp, since an earlier version chose by
  penetration depth alone and shoved the tooltip off the bottom, where
  the clamp snapped it straight back onto the plate it was fleeing.
  **The legend is five lines, not four:** H and P had joined E/M/C on the
  panels line and that line plus the configure hint ran back under the
  clipboard — the configure hint gets its own line
  (`ISO_HELP_TEXT`/`FP_HELP_TEXT`). **A poll that raises more than one
  contract notice no longer clobbers itself**
  (`_announce_contract_changes`/`_contract_notice_queue`): two contracts
  lost together, or a loss and a signature in the same poll, left only
  the last call's text visible, since `_show_contract_notice` overwrites
  `_contract_notice.text` synchronously. The fix is local to
  `_announce_contract_changes`'s own loop — the first notice in a poll
  still shows immediately, exactly as before (both
  `test_contract_board.gd` cases assert `.text` synchronously with no
  intervening frame pump, so a queue-everything design broke them), and
  only a second or later notice in the *same* poll queues, drained by
  `_update_contract_notice` once the current notice's hold+fade window
  has fully elapsed. **A build-menu card's crew cost is two lines**
  (`_crew_cost_text` returns `crew: 2 manufacturing\n4 / period`): joined
  by ` · ` it overran the card and was painted over by the next card's
  stylebox, and `AUTOWRAP_WORD_SMART` alone broke it mid-phrase into
  three. Per UI_UX.md §2b point 32 the line is paid for out of the card's
  own padding — a new `SHOP_CARD_ICON_SIZE` shrinks the build card's icon
  by what the line costs — because a taller card does NOT help:
  `_fit_grid_scroll`'s scroll cap comes from the panel's height, not the
  grid's, so growing the card only clips more of row two. **A splitter's
  hover says `connected` / `not connected`,** not the wire's raw tile
  coordinates (UI_UX.md §5e). Un-fixed and reported: a hover-drift quirk
  in the `--hover` screenshot harness itself, not reproducible in play.
  Full record in `docs/DECISIONS.md`.
- 2026-09-14 **Quality pass, ticket 4: staffing legibility (WHY, COST,
  BEFORE, AFTER).** The staff panel and HUD now answer four questions the
  MVP left to hover and hindsight. **WHY:** each row gets a demand line
  under its meter, `asked for by: crusher ×1, mixer ×1, cooking vat ×2`
  (`_staff_demand_sources_for` counts placements by kind in first-asked
  order, `_staff_demand_text` formats it; `nothing placed asks for staff
  yet` when empty) plus one sentence of what the category does
  (`STAFF_CATEGORY_BLURBS`). **COST, always visible:** the wage bill and
  payday countdown moved out of H onto the always-visible clipboard
  (`wages 14 / period  ·  payday in 6s`), reading the wire's own
  `next_bill`/`shortfall` rather than a Godot-side sum, since ticket 1's
  notice pay means those two can genuinely differ; the build-menu card
  gained a crew's running cost (`_crew_cost_text`; ticket 5 put the cost on
  its own line — `crew: 2 manufacturing` then `4 / period` — see ticket 5's
  entry below). **BEFORE:** the footer replays the payday cascade
  client-side against wire numbers in wire order
  (`_staff_unpaid_count_if_payday_now` — a zero-wage head is never unpaid
  mid-cascade) and warns in the ticket's own words when short
  (`_staff_shortfall_text`: `▲ next payday short by 6 — 3 heads go
  unpaid`, singular `1 head goes unpaid`), else prints `balance after
  payday: N`; HIRE now reads the wire's own `can_hire` per row instead of
  always being enabled. **AFTER:** a payday that idles or loses heads is
  announced through the same `_show_contract_notice` a lost contract uses
  (`_staff_payday_notice_text`/`_announce_staff_payday`, priming on the
  first poll exactly like `_announce_contract_changes`), naming category
  and count (`▲ PAYDAY: 3 packaging unpaid, idle until next payday`, `✗
  PAYDAY: 1 logistics left — unpaid two paydays running`); a row's own
  `status_text` carries the same fact (`▲ 2 unpaid, idle until payday`).
  New fixture `build_underfunded_line` (`test_layouts.gd`: the reference
  line funded to 8, short of its 14 bill by 6, 3 heads unpaid on cascade)
  backs both the pure-function tests and a `--staff-shortfall` screenshot;
  loading it mid-test disables every row's HIRE, so the pre-existing real
  HIRE/FIRE tests that follow it in `test_staff_panel.gd` now explicitly
  rebuild the funded reference line first rather than assume prior state.
  Full record in `docs/DECISIONS.md`.
- 2026-09-13 **Quality pass, ticket 1 (sim): no silent walk-out, no free
  dodge, no money overflow.** The payday rule (`staff.rs`) no longer fires
  the unpaid: a head the money does not reach is **unpaid** — hired, idle
  until the next payday (`reset` gives `headcount - unpaid`), gone only if
  unpaid again; a covered payday clears the arrears, back pay first. The
  line rule stands (the first refusal leaves every head after it unpaid).
  **Notice pay:** `CategoryPool::owed` counts every head employed for any
  part of the period (starts at headcount, `hire` adds, `fire` does not
  subtract), the payday bills `owed`, fired heads first, so firing before
  payday changes nothing. **A hire is refused unless
  `money >= next_bill + wage`** (`can_hire`) — you cannot hire someone you
  could not pay. Wire (`staff_snapshot_json`): `next_bill`, `shortfall`
  (0 when covered), `unpaid_last_payday`, per category `owed`/`unpaid`/
  `left_last_payday`/`can_hire`; `walked_out_last_payday` keeps its name
  and meaning (how many LEFT). Godot should read `next_bill` for the bill
  (its `Σ headcount × wage` is the run rate). Save: `SavedStaff` gains
  `unpaid`/`owed`/`left_last_payday`/`unpaid_last_payday` under
  `#[serde(default)]`, schema 1; more unpaid than hired or fewer owed than
  hired is `SaveError::BadStaff`. **Money is `types::Money` (`I48F16`)**,
  not `Fx`: same 16 fractional bits (old `money_bits` read unchanged,
  `MoneyBits(i64)`), 48 integer bits; `I16F16`'s 32 767 was minutes away
  and a payout there PANICS in debug / wraps in release; `Economy::tick`
  credits with `saturating_add`; `cost_of` refuses a total past `Money`
  via `checked_from_num`. Audit verdicts in the DEVLOG entry; full record
  in `docs/DECISIONS.md`.
- 2026-09-13 **Belt item spacing is half a tile, and a belt's front item
  respects the next tile's footprint (quality pass, ticket 2).**
  `ITEM_MIN_SPACING_TILES` in `world.rs` is **0.5** (was 0.25), chosen by
  rendering `--bottleneck` at default zoom and `--closeup`: items are
  half-tile models (the box 12 voxels wide), so at a quarter tile a fed
  lane drew as one bar of product; at a half tile every item stands on
  visible belt and a jam still reads as a queue; wider reads as a trickle.
  Item models and `_carried_item_transform` are unchanged. Belt speed is
  unchanged (one tile per 50 ticks), so a belt carries **one item per 25
  ticks, forty per thousand**; the reference line's steady output is the
  same 41–42 boxes per 10 000 ticks before and after (the vat is the
  bottleneck). **Packing rule found by widening it:** the front item used
  to run to the belt's end while the next tile held an item at its start,
  the same point — five per tile with one drawn twice at every boundary.
  `BeltSegment::tick_toward(speed, room_ahead)` holds the front item
  `min_spacing - room` short of the end; `World::tick` reads every
  carrier's downstream `room_at_start()` (`Topology.belt_ahead`, aligned
  with `belts`; a splitter uses its roomier side) BEFORE any carrier moves,
  so the result is order-independent and a rebuilt save stays in lockstep.
  A jammed tile holds exactly `1 / spacing` items (two); a moving queue
  settles `speed` looser across each boundary, once per item, and no
  throughput changes. **Knock-on, reported not tuned:** one lane delivers
  four tomatoes per 100 ticks against the crusher's 80-tick craft, so the
  crusher is belt-limited (~80 % busy, `starved · waiting for Tomato`) and
  the mixer runs at its pace until the line backs up (crusher blocked from
  ~8300 ticks, was ~7000; `SETTLE_TICKS`/`BOTTLENECK_SETTLE_TICKS` are
  10 000). The staffing rule "short one manufacturing head idles the vat"
  holds on the ticks the crusher and mixer both claim; on a tick the
  crusher waits for a tomato the vat gets its crew (`tests/staff.rs`
  asserts refused > ran; the paused-craft test fires three heads).
  Splitter fixtures count 8 on four tiles and 6 on three;
  `test_machine_load.gd`'s polled status change watches the VAT (the one
  machine a fed line drives to `working`). Whether the crusher's recipe
  should be re-timed is ticket 1's. Full record in `docs/DECISIONS.md`.
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
- 2026-09-13 **Quality pass, ticket 3 — presentation bugs and performance.**
  **The tick path reads a chunk DELTA, and per-frame polls are gated on
  the wire's revision.** `SimBridge::chunk_delta_json(chunk)` is the chunk
  snapshot cut down to the belts/splitters whose contents changed and the
  machines/hoppers/tanks/docks whose state did since the dirty set was
  last cleared (walked from the changed set, never the chunk; sorted so a
  reading is deterministic; never a pipe — fluid is polled), and the WHOLE
  snapshot after a place/remove/rotate/load touched the chunk
  (`World::chunk_delta`, `full_chunks`). `chunk_snapshot_json` is unchanged
  and stays the full read. Delta granularity is still per chunk (ARCH §4):
  what changed is what a dirty chunk's read CARRIES. `SimBridge::revision()`
  counts ticks plus commands, refused or not; `_process` parses its
  snapshots only when the revision, the viewport size, the hotbar
  selection or the set of open modals changed (`_wire_changed`), and the
  wall-clock fades run in the `else` branch so nothing freezes under the
  pause menu; a rebuild resets the gate. The economy is parsed once per
  frame (`_read_economy`) for the clipboard, deck and board. Stress line
  (`test_perf.gd`): 8.4 → 2.1 ms/frame, `_apply_dirty_chunks` 12.3 → 2.2
  ms per tick, 78 KB → 12 KB per tick; `_rebuild_world` 70 → 80 ms because
  **a rebuild now re-reads EVERY occupied chunk's full snapshot**
  (`_reread_every_chunk`, chunks computed from the layout on the sim's
  32-tile grid): before, a place/remove/rotate emptied the item pools and
  re-read only the dirty chunk, so every item on a belt in another chunk
  vanished until it next moved — for good on a backed-up belt.
  Input-state fixes, each with a headless proof: `_input` ends a right-drag
  on any right RELEASE (a release over a panel with a mouse filter or the
  pause menu never reached `_unhandled_input` and the camera followed the
  mouse until the next right click); first-person mouse capture is one
  decision, `_fp_mouse_free` (TAB with M/C/H/P or a config panel up used
  to capture the cursor under the panel); `_rotate_tile` refuses under any
  modal (R turned the tile behind the panel, unseen); X closes every modal
  like E/M/C/H/P do. The production panel's window buttons take the
  hotbar's cached styleboxes instead of five new ones per button per
  update. Deliberately left (reported): the legend's third line runs under
  the clipboard at 1600x900 on every screen (ticket 5); `_read_world_layout`
  is ~40 of the rebuild's 80 ms on 1500 tiles (a 336 KB layout parse per
  place); `cargo xtask` builds a DEBUG `sim_core` and every number above is
  measured on it. Full record in `docs/DECISIONS.md`.
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
  authored by its value range rather than its detail (UI_UX §2b points 36
  and 37): broad low-contrast blotches, never per-voxel
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
  starters — none since 2026-09-26). Missing `strikes` windows in a row
  loses the contract — losing IS the penalty, no fine on top; a paid
  window resets the count.
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
  deck instead shows delivery stock directly. (Since 2026-09-26 the deck
  shows the dock's OWN stock.)
- 2026-09-08 **Real Placement, ticket 1 (sim).** `World::new` builds an
  empty grid; `Place`/`Remove`/`Rotate` do everything else, and
  `sim::reference_chain` rebuilds the shipped line from real commands as
  the one test fixture every integration test shares. Flow is derived
  from adjacency, recomputed only on placement changes, never per tick.
- 2026-09-08 **Delivery dock, ticket 1 (sim).** The dock is a
  pass-through: it empties into `Economy`'s shared pool every tick rather
  than holding stock, so what's "on the dock" is always one number in one
  place. **Superseded 2026-09-26:** each dock holds its own stock.
  `CollectOutput` is removed — a supplied factory earns money with
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
