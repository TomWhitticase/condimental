# Sauce Factory — Decisions log (condensed)

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
  bottles out on the right, plate over the middle).
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
  opens the PR, `gh pr checks` must be green, and `gh pr merge --squash
  --delete-branch` lands it (squash by default, so the squashed subject is the
  one Conventional Commit and is linted like any other; a real merge commit
  only where each commit is a reviewed unit). `.githooks/pre-push` **blocks** a
  direct push to `main` — mechanical, unlike the `pre-commit` DEVLOG reminder,
  which is a judgement call and only warns. Do not merge a PR that is red,
  still running, requested for review, or that deviates from a design doc,
  bumps `gdext` or changes the save format — say it is ready and stop.
  **Deciding something is out of scope is only half the decision; filing it is
  the other half**: `gh issue create --label deferred`, with file *and*
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
