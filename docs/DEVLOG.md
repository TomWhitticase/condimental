# CONDIMENTAL — Development Log & Status Tracker

This doc answers "what's actually been done, and what's the state of things
right now" for whoever (human or agent) picks this project up cold. It's a
**log and status tracker, not a design doc** — the reasoning and specifics
live in [ARCHITECTURE.md](ARCHITECTURE.md), [UI_UX.md](UI_UX.md), and
[GAMEPLAY.md](GAMEPLAY.md); this file indexes *when* and *why* things changed,
and what's true *now*. It is deliberately **not** `@`-imported into
`CLAUDE.md` (unlike those three) — it will grow without bound over the life
of the project, so it's meant to be checked when you need history/status, not
force-loaded into every session's context.

---

## Current status

*(This section reflects "now" — edit it in place as things change. The Log
below is history and should only ever be appended to.)*

- **Stage:** Pre-alpha, but the core MVP loop is real end to end and it looks like a game. Milestones 1 (Scaffold), 2 (Mobile export spike), 3 (Core loop proof), 4 (First full chain), and 5 (Economy MVP) are all done — a real multi-stage production chain (tomato + sugar -> tomato paste -> **piped** ketchup mix -> **piped** ketchup -> bottled ketchup -> boxed ketchup) that can deliver against several concurrent, data-driven contracts to earn money, headlessly verified end to end, Android export proven working. Raw ingredients are bought with money now, so the loop has a real recurring cost as well as an income. On top of that, a visual-polish pass built UI_UX.md's voxel asset pipeline and turned the flat 2D placeholder scene into an isometric, real-time-lit 3D factory.
- **Everything is in `main` (2026-09-11).** The six feature branches that were
  finished but unmerged — contract board, payout juice + audio, machine
  animations, floor redesign, vinegar assets, belt-arrow removal — were
  consolidated onto one integration branch and fast-forwarded into `main`.
  The union passed `scripts/verify-core-loop.sh` (19 Rust suites, 30 headless
  Godot proofs, no failures) and has been rendered and played; the same script
  is **26 Rust suites and 42 Godot proofs as of 2026-09-25** — a number the
  run now prints for itself. A fresh
  worktree still needs `cargo xtask` and **two** `godot --import` passes
  before the world is visible. See the log entry below.
- **CI runs the tests now (2026-09-16).** Nine workflows under
  `.github/workflows/`: `rust.yml` (sim_core + voxgen tests, `cargo xtask`,
  a fmt/clippy ratchet, and a check that `assets/voxels/models` still matches
  its generators), `invariants.yml` (the CLAUDE.md rules that can be grepped
  for, `shellcheck`, `actionlint`), the two older lint/screenshot backstops,
  `verify.yml` (the full Godot gate — nightly, on demand, or on a PR labelled
  `full-verify`), `audit.yml`, `labeler.yml`, and
  `claude.yml`/`claude-review.yml` (dormant
  until an `ANTHROPIC_API_KEY` secret exists). `docs/WORKFLOW.md`,
  "What CI runs", is the table. See the log entry below.
- **There are two sauces (2026-09-18).** `data/recipes.ron` forks: ketchup
  plus spice mix and molasses go into a mixer for `bbq_mix`, that plus liquid
  smoke into a cooking vat for `bbq`, then the same bottler and packer. No new
  machine kind — a BBQ line is a *branch* tapped off the pipe leaving the
  ketchup vat, so the two sauces compete for one vat's output, and the 600-tick
  BBQ cook against the vat's 240 splits it 3 ketchup to 2 BBQ. Two of the four
  new contracts name both sauces. **BBQ is gated now (2026-09-19):**
  `data/unlocks.ron` holds its three ingredients and four contracts back until
  **a hundred boxes of ketchup have been sold** (the trigger was ten lifetime
  delivery windows until 2026-09-26), so a new game's market and board show
  only what a one-sauce factory can use. See the log entries below.
- **The game explains itself (2026-09-25).** **B** opens THE COOKBOOK: a
  sauce tree with a node per family in a column that says how far from raw it
  is, an arrow wherever one sauce is an ingredient in another, and a page
  behind each node with the ingredients per case, the machine process in
  seconds, the cost, what the contracts pay and who buys it. All of it is
  `sim_core/src/sim/cookbook.rs` derived from `data/*.ron` — nothing in
  `godot_project/cookbook_panel.gd` names a sauce, a machine or a price, and
  `test_cookbook.gd` fails if anything ever does. A locked family is an
  unnamed silhouette carrying its gate. The same panel is a page on the pause
  menu, for reading with the factory stopped. **It has a BASICS section now
  (2026-09-26):** six pages of prose out of `data/guide.ron` explaining the
  loop, money, contracts, crew, building and growing — and the prose states
  no number of its own, every quantity in it being a `{placeholder}` the sim
  answers from the registries. See the log entries below.
- **Spending past the payday asks first (2026-09-26).** Any spend that would
  leave the balance below the next payday's bill — equipment, ingredient
  stock, a contract fee, or a hire — raises a confirmation naming the cost,
  the balance after, what payday needs, the gap and the seconds until it
  lands. `World::spend_preview` decides, off the exact command about to be
  issued and the same price functions that charge for it; Godot reads `warn`
  and never recomputes. One funnel, `_spend_with_confirm`, gates every spend
  site. It is advice, not a gate: the old hard refusal on hiring while short
  became a warning too, and `can_hire` on the wire is `hire_warns`. There is
  deliberately no "don't ask again". See the log entry below.
- **Raw stock has a ceiling (2026-09-21).** `data/storage.ron` caps every
  ingredient at `base_cap` 20 plus 50 per **bay**, and a `warehouse_rack`
  (1×2, six bays, 75) is the only thing that supplies bays. `BuyIngredient`
  clamps silently and charges for what it credited; the reason is read off
  `IngredientSnapshot::capacity`. Bays are derived from the ledger, so free
  bays are a shared pool and a bay frees itself when its stock is used. A rack
  is pure storage — it never touches a belt. Any fixture that wants more than
  20 of something has to build the room first. **A rack draws what it holds
  (W3):** six shelves, a pallet per bound bay from the bottom level up, goods
  scaled to the bay's fill and tinted per raw, hover off either tile of the
  1×2, and W1's removal refusal said in words before the click.
  `scripts/screenshot-scene.sh --warehouse` photographs it. **The market
  reads the ceiling (W4):** a crate prints `owned / capacity`, buys only what
  fits, says `AT CAPACITY` when nothing does, and a header meters the racks'
  shared bays; the STOCK plate marks a full row `FULL` and says how many rows
  it is hiding when the column cuts its list. See the log entries below.
- **The lighting post-passes mean something now (2026-09-20).** Glow is
  `1.0 / 0.0 / 1.2 / SCREEN` and SSAO is `0.45 / 16.0 / 1.6`
  (`factory_environment.gd`). Both sets of old numbers were chosen where they
  could not be seen — the glow while issue #25 meant nothing emitted, the
  SSAO at a radius sized for a machine against the floor rather than a
  fitting against a body. Retune either only against rendered frames, and the
  SSAO in three framings: its radius is in world units, so the isometric and
  first-person cameras disagree about how strong it is. See the log entry
  below and `docs/DECISIONS.md`.
- **Agent context restructured (2026-09-16).** `CLAUDE.md` is 72 lines and
  `@`-includes nothing; it maps a task to the doc that covers it. The
  condensed decisions log lives in `docs/DECISIONS_LOG.md`, workflow in
  `docs/WORKFLOW.md`, and `sim_core/`, `godot_project/` and `assets/voxels/`
  each carry their own `CLAUDE.md` of local invariants with `AGENTS.md`
  symlinked beside it. Four skills under `.claude/skills/`. Always-loaded
  context went from ~34,500 tokens to ~950. See the log entry below for the
  five doc defects the move turned up, including the tick rate.
- **The world is indoors (2026-09-16).** The backdrop is a factory-interior
  sky shader, not weather, and the scene's lighting is the hall's rather than
  a sun's. See the log entry below.
- **The hall is drawn in first person only (2026-09-16).** The isometric view
  fills its background with the hall's own haze colour instead — a sky is
  intersected against `EYEDIR` and an orthographic camera's rays are
  parallel, so the roof came out as stripes fanning from the corners. The sky
  stays the ambient and reflection source in both modes, so the factory is
  lit identically either way (`factory_environment.gd`'s `set_backdrop_for`).
- **The menu is a title screen (2026-09-16).** The game opens on the factory
  itself — a hand-composed 3D still life of the shipped chain in the same
  hall, with a drifting camera, a big wordmark and a daily splash line — not
  on a flat plate. It loads none of `main.tscn` and runs no simulation, and
  it is built from the game's own assets, lighting and floor helpers
  (`title_scene.gd`, `voxel_assets.gd`, `factory_environment.gd`). See the
  log entry below.
- **An emissive voxel glows in its own colour (2026-09-16, issue #25 fixed).**
  `emissive` is the one material class Godot builds as a `ShaderMaterial`
  (`godot_project/voxel_emissive.gdshader`, `EMISSION = COLOR.rgb * energy`);
  matte, metal and liquid stay `StandardMaterial3D`, which they must, because
  `main.gd` duplicates and re-tints those. `StandardMaterial3D` could not
  express it: its emission comes from a texture this pipeline does not have.
  `emission_energy` was retuned 2.6 -> 1.2 in the same pass — the old value
  was picked when nothing could reach the screen, and it burns a lit amber
  lens to cream. Status lamps still report their state through **albedo** as
  well as emission, so an OFF lamp reads off in any lighting. See the log
  entry below.
- **The sim can express a branching chain (2026-09-17).** A machine kind may
  have several recipes and a placed machine runs whichever its current inputs
  satisfy (file order breaks a tie, and there is deliberately no player-facing
  picker); a recipe may draw several fluid networks, matched by fluid id. A
  pipe run's fluid is now derived by shrinking possibility sets rather than by
  whichever machine reached it first, and which recipe a machine is on is
  saved. `data/*.ron` is untouched — the BBQ chain itself is the next ticket.
  **Amended 2026-09-20 (below):** `validate_chain` no longer demands one
  connected component, so `recipes.ron` may hold independent production lines;
  and where the graph cannot name a pipe run, a **tank declares what it
  holds** and its declaration replaces the run's candidate set. The save is
  schema 2 for it, and a v1 save is refused rather than migrated. See the log
  entries below.
- **Repo:** Private GitHub repo (`TomWhitticase/condimental`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
- **Work reaches `main` through pull requests, and deferrals become issues
  (2026-09-16).** Nothing is pushed to `main` directly — a `pre-push` hook
  refuses it — and anything noticed but consciously not fixed is filed with
  `gh issue create --label deferred` instead of being left in a closing
  message. The open backlog is `gh issue list`, not this file: this doc stays
  the status and history, `docs/DECISIONS_LOG.md` stays the rules. Full rule
  in `docs/WORKFLOW.md`; the `open-pr` and `file-issue` skills are the
  procedures.
- **Every PR body embeds a screenshot (2026-09-16).** Owner's rule, and it
  is not scoped to visual changes. `scripts/pr-screenshot.sh` publishes a
  rendered PNG to the orphan `pr-media` branch and prints the markdown to
  paste; a `PreToolUse` hook and the `PR screenshot` CI job both refuse a
  PR whose body has no image, so `gh pr create --fill` alone no longer
  opens one.
- **Design docs:** ARCHITECTURE.md, UI_UX.md, and GAMEPLAY.md are all written and cross-referenced; docs/README.md indexes them.
- **Milestone progress:** milestones 1-5 (Scaffold through Economy MVP) were built as an Orca orchestration Run (`run_51e5cb271445`), one Task per milestone, dependency-chained. **All 5 are complete and merged.** Check `orca orchestration task-list --run run_51e5cb271445 --json` for the DAG's final state. There are 11 milestones total, not 9 — a milestone 5 "Economy MVP" was added since the loop wasn't a game without it. **Milestone 6 (Fluids) is done**: its simulation half (pipe network, pressure equalization, a recipe requiring a liquid, the `SimBridge` fluid snapshot) and its presentation half (pipe/supply-tap/liquid voxel assets, a pipe run rendered from the simulation's own segment list, a liquid body whose height tracks live pressure, and pipe hover info in both view modes) have both landed, built as a 2-ticket orchestration Run (`run_2867a25ae373`, sequential: sim core then presentation). **Milestone 7 (Timed processes) is DONE, and the stage that proved it has since been deleted**: its simulation half added an `aging_rack` holding a batch for 2400 ticks (40 real seconds, 10x the cooking vat) on the *existing* generic `TimedProcess`, needing zero changes under `sim_core/src/`, which was the milestone's actual point; its presentation half drew the rack with a live maturation gauge. The **chain rework** (below) then removed ageing from the game outright, so nothing shipped exercises the long timescale any more — `TimedProcess` itself is untouched and runs every craft cycle, and the gauge mechanism is kept as an empty `MACHINE_PROGRESS_GAUGES` table waiting for the next long process. A separate **"Machines improvement"** epic (not an ARCHITECTURE.md §9 milestone — an infrastructure/content pass requested directly) is in progress: ticket 1 landed machine footprints as data (`data/machines.ron`), generalizing the chain's layout math so a machine can occupy more than one tile, and ticket 2 spent that on real content — the chain's front end is now a crusher, a mixer and a genuinely 2x2 cooking vat. Ticket 3 replaced the debug "spawn a tomato onto the belt" command with a real **hopper** — a building west of the chain's head that buffers raw tomatoes and meters them onto the first belt one per tick, filled by the player (SPACE) rather than teleporting items. **Ticket 4 finished the epic**: the Godot presentation of all three — a bespoke voxel model per front-end machine kind, multi-tile rendering so the cooking vat actually covers its 2x2 footprint, the hopper drawn with a product level that tracks its live fill, and hover resolving any tile of a multi-tile machine as well as the hopper. **The "Machines improvement" epic is complete.** A further **"Shop & Inventory"** epic (also requested directly, and a deliberate priority shift toward a real build-your-own-factory game) has started: **ticket 1** landed the economy half — `data/shop.ron` prices every buildable kind, `sim::shop` loads/validates it, the player owns a per-kind count, and a `Purchase` command spends contract money on it, all exposed as `shop_snapshot_json`. **Ticket 2 finished the epic**: a real build menu (**B**), a grid of voxel-rendered icon slots built entirely from `shop_snapshot_json`, where clicking a slot — or pressing its number key — buys one, and what you can't afford is disabled, dimmed and marked. **The "Shop & Inventory" epic is complete.** The **"Real Placement"** epic that followed it is what made a purchase visible on the floor: **ticket 1 (sim) has landed** — `World::new` now builds *nothing*, every belt tile, pipe tile, machine and building is placed by the player out of what they own, everything carries a real four-way `Direction` it can be rotated through, and flow is derived from adjacency (recomputed only when something is placed, removed or rotated) rather than from `recipes.ron`'s file order. Ticket 2 (the Godot build/ghost/rotate UI, and rendering per-tile rotation) is still to do.
- **Art/UI:** the voxel pipeline (UI_UX.md §2a/§2b) exists and is the source of every asset in the scene — including milestone 6's pipes and the liquid inside them, which added a fourth material class (`liquid`, a glossy dielectric) to `palette.ron` — Rust generators in `assets/voxels/generators/` produce `.vox` models, reviewed as isometric PNGs via `scripts/build-voxels.sh`, imported into Godot by a pinned community importer. UI_UX.md §5's real UI is now landed: the money/contract readout is a diegetic clipboard panel (paper background, steel clip bar, checkbox tick marks) with a voxel-rendered icon per required item, and per-machine detail (recipe/progress/buffers) is hover-only instead of dense always-on text, working in both view modes — a screen-centre crosshair stands in for the cursor in first-person — see the log entries below.
- **UI polish epic (done):** a Run (`run_cc6860507d36`) with 3 parallel tickets targeting UI_UX.md §5, all done and merged — hover tooltips for machines, a voxel-rendered icon pipeline, and the clipboard reskin. Check `orca orchestration task-list --run run_cc6860507d36 --json` for the DAG's final state.
- **Visual overhaul round 2 (done):** a second directly-requested pass — the first visual-overhaul attempt (below) closed a floor-level seam but left the actual complaint unfixed ("items disappear into the wall"), so this round diagnosed and fixed it properly, as a 3-ticket epic. **Ticket 1 (sim):** the pipe run's always-refilling off-map source is gone, replaced by a player-loadable `FluidTank` (one tile west of the run's head, loaded with the new **V** key) drained by the same pressure-equalization pass that moves fluid along the run — vinegar is now a consumable the player manages, exactly as tomatoes already were via the hopper. **Ticket 2 (assets):** every machine kind in `data/machines.ron` now has a bespoke model (the bottler and packer were still generic boxes), the tank is drawn with a live level gauge, the pipe run and the crusher were redesigned after diagnosing exactly what read wrong in a rendered close-up, and the three bulk stages travel the belt as visibly *liquid* product (a glossy surface in an open pail, not a solid block). **Ticket 3 (the actual connection fix):** every machine now has real, carved inlet and outlet openings sized and placed from the simulation's own layout — the key finding was that the fixed isometric camera can only ever see one side of a tile, so an outlet is a real doorway carved through the camera-facing wall while an inlet is a hooded housing cut open on that same visible side (cutting an opening on the actual inlet face, as the first attempt would have, is invisible to this camera and would have fixed nothing). The belt gained two junction-adjacent variants picked per tile from the sim's own layout, the pipe run's downstream end is properly terminated instead of stopping in mid-air, and a new `--closeup` flag on `scripts/screenshot-scene.sh` let every one of the twelve junctions be inspected individually. Corner handling for belts/pipes stays deliberately undone (documented in `sim_core/src/sim/world.rs`'s module doc) until the sim carries a per-tile direction field.
- **Visual overhaul, first attempt (superseded by round 2 above):** closed a floor-level seam between machine/hopper footings and belt/pipe edges, added a connector fitting for the fluid-fed stage, gave the hopper's status lamp real feedback, and fixed some contract-clipboard typography — real fixes, but not the one the player actually flagged, which is what round 2 above addressed.
- **Camera:** two view modes, toggled with TAB. Isometric (default): pan (WASD/right-drag/arrows) and zoom (scroll, trackpad, +/-); the fixed angle itself never changes, per UI_UX.md §1. First-person: a free-look "fly" camera (mouse-look, WASD move, SPACE/SHIFT up/down) — not a UI_UX.md concept, and not real player physics (no gravity/collision, this project has neither). 90° snap-rotation of the isometric angle is still not implemented (still a nice-to-have, not required).
- **Lighting:** the sky is the real background (`BG_SKY`), not a flat colour — see the "empty void" DEVLOG entry below for why that matters. `scripts/screenshot-scene.sh --first-person` reviews that mode's lighting specifically.
- **Delivery dock (done, both halves):** the chain has a real exit, and you can see it. One more belt runs past the last machine into a `dock` building, which ships finished product into the shared delivery pool automatically every tick — `PlayerCommand::CollectOutput` (the **C** key) is **removed**, so a supplied factory now earns money with no player action beyond keeping the hopper and vinegar tank topped up. The dock is drawn as the one piece of *architecture* on the line rather than another machine: a raised, hazard-striped loading platform with rubber bumpers, a hooded transfer housing where the belt runs in, and a despatch door under a canopy. It has **no fill gauge**, deliberately — its own buffer empties every tick, so a bar driven from it would read zero forever; what stands on its deck instead is one case per unit of the economy's shared `delivery_stock`, the same figure the clipboard labels ON THE DOCK. The shipped line now spans three chunks, (-1, 0) through (1, 0).
- **Real placement (DONE, both halves):** the world starts **empty**, with `STARTING_MONEY` (200) and a bare floor, and the player builds everything on it. `Place`/`Remove`/`Rotate` commands put things down out of the shop inventory, take them back, and turn them 90° clockwise; a belt carries toward the way it faces, a machine accepts on the face opposite the way it faces and discharges on the face it points at, and a line may turn corners. The hopper, the fluid tank and the dock are ordinary placeable kinds now, and a world may hold any number of each. The scene rebuilds itself from `world_layout_json` after every place/remove/rotate, draws every tile at the facing the player gave it, and has a real build mode: right-click an owned build-menu slot (or shift+number) to pick it up, a translucent ghost follows the cursor and says **✔ PLACE** / **✗ BLOCKED** / **✗ NONE OWNED** in a glyph and a word as well as a colour, **R** rotates the ghost or whatever is under the cursor, **X** toggles remove mode, **ESC** cancels, left-click confirms. The belt junction art stays east-only and falls back to the plain tile for the other three facings — a known, tested, accepted gap; a four-directional art pass is a separate ticket. *(The balance caveat this entry used to carry — 200 could not reach a first payout — was fixed by the inventory-hotbar work below; `STARTING_MONEY` is 3000 now.)*
- **Inventory hotbar (done):** an always-visible strip along the bottom edge, one slot per `data/shop.ron` kind, showing what the player is carrying. **Number keys 1-9 and 0 select a slot and put that kind in your hand** — in either view mode, with or without a menu open, which is the behaviour change: until now nothing at all was reachable without opening the build menu first. Pressing the selected slot's key again puts it down, mirroring ESC. Buying stays a separate verb in the build menu (**E**), on a slot's left click or SHIFT+number. A kind you own none of is an empty socket rather than a disabled button, selection is signalled four ways at once (bigger slot, hazard-yellow frame, caret, brightened key digit) plus Minecraft's fading name flyout, and the whole strip is driven from `shop_snapshot_json` like the menu it now fronts. **Starting money went from 200 to 3000 in the same ticket** — the shipped six-stage line costs 1087, so a new save can now build all of it outright with ~1900 to spare, and the core loop finally closes from a cold start.

- **Conveyor splitters (DONE, both halves):** a line can fork. A `splitter` is a 1x1 buildable kind that carries product exactly like a belt tile and deals it alternately between two outputs — the tile it faces, and the tile one quarter turn clockwise from it — so one `rotate` turns both outputs together and no new command was needed. If the side that is due is blocked (or was never built) the item goes down the other one in the same tick, so a full branch never stalls the whole line and "only one output connected" needs no special case. It reaches every sink a belt's hand-off does: another belt, another splitter, a machine's inlet face, a dock. It is **drawn** as a machined diverter table set into the belt run — a bright steel plate where the rest of the line is rubber, with three open lanes and one closed rail — carrying a **next-output arrow inlaid flush in the deck**: both directions are cut into the table as grooves and one of them lights up, so which way the next item goes is a pair of positions rather than a colour, and nothing stands proud of the surface the product slides over. The arrow shows where the item will **actually** go, not the raw `next_output`: a splitter whose branch was never built reports "branch" forever while everything goes straight, and an arrow driven off that field alone would point at bare floor. Items cross a splitter drawn exactly as they cross a belt, and hover names both outputs and says "not connected" about one that was never laid.
- **Contracts (done, both halves):** contracts are `data/contracts.ron` content and three run concurrently, each with its own mix/cadence/payout, all drawing from one shared pool of delivered stock (a box pays one contract, never two). The clipboard panel shows all of them — one paper docket card per contract on the one board, each with its own checklist, payout, countdown and delivered/missed tally, above a single shared money figure and a single "on the dock" line naming the pool they compete for.
- **Contract board (DONE, both halves, 2026-09-10):** contracts are **signed and can be lost**. A new game runs one basic contract; the rest of `data/contracts.ron` (schema 2, nine offers since the 2026-09-11 growth tier — a starter tier one line can serve, then six larger, slower orders for every line after it — with `price`, `strikes` and `starter`) is a board the player buys from on **C**, one of each contract only. A contract that misses its strikes in a row is lost — it leaves the clipboard, is announced, and goes back on the board to be signed again at its price. Each docket now says how it is doing and whether it is in danger: a status line (✔ READY / ⚠ AT RISK / ⚠ LAST CHANCE), strike pips and a state border beside the countdown bar. The top offer exceeds one line's throughput beside the others, by design. Built on branch `TomWhitticase/contract-board`.
- **Chain rework (DONE, both halves):** the middle of the production line is **piped, not belted**, which is what a real sauce plant looks like. The mixer's output and the cooking vat's output are fluids (`recipes.ron` schema_version 3 gained `fluid_outputs`), so mix flows from the mixer to the vat and cooked ketchup from the vat to the bottler through pipe runs with no tank behind them; the bottler is where the product becomes a solid again, and the cooking vat is now the one machine on the line **no belt touches at all**. The mixer gained **sugar** as a second solid ingredient and grew to **1x2** for it — a machine accepts solid input on every tile of the edge opposite the way it points, so N solid ingredients need N tiles of inlet face or the two lanes merge onto one belt and deadlock. The aging rack is gone. **Raw ingredients cost money**: `data/ingredients.ron` prices tomato, sugar and vinegar, a `BuyIngredient` command buys them, and a hopper or a tank is loaded from that bought stock *(originally with the global `SPACE`/`V` keys; since the configuration panel below, from the building's own panel)* — so overproduction is a real loss rather than free accumulation. On the presentation side: a hopper's contents are chosen **when it is placed** (the ghost carries a tinted product body and the prompt names them in words; the **Q** key that cycled them is gone, replaced by the placed hopper's own picker), because with two raw solids the simulation can no longer infer which bin the player meant — until that landed, placing a hopper from the UI did nothing at all, silently. Buying stock has its own panel on **M** (the build menu was already as tall as the space above the hotbar allows), and a permanent larder strip above the hotbar shows what is carried so an unloadable hopper explains itself. The mixer, the cooking vat and the bottler were re-modelled with pipe nozzles where belts no longer meet them.
- **Hopper/tank configuration (DONE, both halves):** nothing about feeding a factory is on the keyboard any more. A placed hopper or tank is **clicked**, and a panel beside it shows its level, a **FILL** button (disabled with its reason in words when the larder is empty, the bin is full, or the tank is plumbed to nothing), an **AUTO-FILL** switch that keeps the bin topped up from bought stock every tick — spending stock, never money, and simply pausing while the larder is dry — and, for a hopper, an item picker that retargets it in place. The click is a toggle, only one panel ever exists, ESC closes it first, and it closes itself under any menu or mode. The global **SPACE**/**V**/**Q** keys and `CycleHopperItem` are deleted. Pinned by `test_config_panel.gd`; `scripts/screenshot-scene.sh --config TX,TY` photographs it.
- **Machine animations (done):** every processing machine has a visible moving mechanism — the crusher's toothed rolls and flywheels, the mixer's ribbon, the cooking vat's agitator spider, the bottler's plunging nozzles and capping chuck, the packer's taping carriage, and a drive spider on the generic housing for any kind without a model of its own. Drawn as a four-frame motion overlay on the machine's own footprint (the moving parts are cut out of the bodies, not duplicated, so nothing z-fights), advanced from the tick beside the belt animation so the pause menu freezes it. **It runs only while the machine is actually working**: `Machine::tick` keeps the process of a machine that has finished but is blocked on a full output, so the gate is `elapsed < required` rather than `process != null` — which makes a lit status lamp over a still machine mean "the thing downstream is full". One pace for every kind (a cycle per 30 ticks); the belt carries speed.
- **Music and sound effects (done, 2026-09-12):** the game has a score and a sound for everything the player does. One looping synthesised track (D minor, 92 BPM, eight bars — pad, bass, mallet arpeggio, soft drums) starts on the main menu, carries across into the factory on the `Music` autoload, ducks under the pause menu and has its own MUSIC ON/OFF switch there. `audio.gd` grew from two cues to fourteen: place, remove, rotate, buy, sign, hire, fire, a hotbar pick, menus opening and closing, a shared "refused" buzz for every command the simulation declines, and a falling cue for a lost contract — each fired by the handler that knows whether the action went through. Still synthesised in GDScript, still no `.wav` files; `test_audio.gd` proves all of it headlessly.
- **Payout juice, and the project's first audio (done):** earning money is now an event rather than a number quietly going up. When a contract settles, coins fountain up over every dock, the money counter punches up in scale and flashes amber, a "+N" rises out of it and the docket that paid is stamped **PAID**; a case landing on the dock gets a much smaller cue (a pop and a quiet knock), because that happens constantly and a payout happens every thousand-odd ticks. It needed **no simulation change at all** — Godot notices a payout by watching each contract's `deliveries_made` for a rise in the snapshot it already polls. The whole celebration is clocked off `sim.current_tick()`, so it pauses with the game and a screenshot run stays reproducible. **Sound exists now**, and it is synthesised in GDScript at runtime (`godot_project/audio.gd`) rather than shipped as `.wav` files — which settles UI_UX.md §8's open audio question as *feedback-only, procedurally generated*, with a SOUND switch on the pause menu.
- **Pipes & conveyors overhaul (DONE, `docs/PIPES_AND_CONVEYORS_PLAN.md` has the what-shipped/what-is-open section):** seven tickets on one integration branch to make belts and pipes look right, read right, run fast and connect up. **Ticket 1 (sim) has landed:** every belt, splitter and pipe on the wire carries `links` — what feeds it, what it hands to, or what its pipe is joined to, per side — recorded by the topology pass so it is exact and free per tick. **Ticket 4 (Godot) has landed:** belts, splitters, pipes and the items riding them are `MultiMesh` instances (one per asset and material class) rather than a node each, with a 1500-tile stress fixture, a headless timing test (`test_perf.gd`) and `scripts/screenshot-scene.sh --stress`; machines and buildings keep their nodes, and the seams the belt-animation and corner tickets build on (frame sets, a caller-supplied yaw per tile, one item-positioning function) are in place in `main.gd`. **Ticket 3 (assets) has landed:** the pipe model family (straight, corner, tee, cross, end, machine/tank fittings, one fluid body per shape) on a 15.5 midline, with seven pipe rendering defects fixed at the generator. **Ticket 2 (assets) has landed:** the belt model family — straights and left/right corners in four animation frames with a solid raised direction arrow *(replaced by a plain rubber cleat on 2026-09-10 at the player's request — see the log)*; the belt-side junction variants are retired (the plain tile carries the junction, the machine's hood does the rest). **Ticket 6 (Godot) has landed:** every pipe tile draws its shape (end, straight, corner, tee, cross), its turn, its machine/tank fittings and its per-shape liquid body from the wire's `links` alone; hover names a pipe's connections in words; the one-arrangement `pipe_source`/`pipe_inlet` tiles are deleted; `test_pipe_shapes.gd` and `scripts/screenshot-scene.sh --pipe-gallery` cover every shape at every turn. **Ticket 5 (Godot) has landed:** a belt's shape (straight, left/right corner, merge, start, end) comes from its `links`, the four-frame surface animates at item speed from the tick, items arc through corners (and are drawn face to face on every tile — they had been half a tile downstream), hover says feeds and hand-offs in words, the retired belt aliases are deleted, and a `--belt-gallery` fixture shows every shape and junction at every facing. **Ticket 7 (close-out) has landed:** the galleries were walked shape by shape and the defects found were not in the shapes — `cargo xtask` now runs Godot's importer twice itself (a staged voxel whose `.import` sidecar had been deleted left the scene blank until someone knew to run `--import` by hand); the headless proofs' standing `Parameter "material" is null` line is gone, freed instances now drop their mesh and material first (`_release_render_resources`), so that line is a real regression from here rather than scenery; `scripts/make-demo-saves.sh` rebuilds the four demo saves from `test_layouts.gd`'s own builders, each proven to load back before it is written, which a save-schema bump had otherwise made unrepeatable; and the two ghost rules are reconciled as one (UI_UX.md §2b point 34) with no behaviour change. **Left open deliberately, not missed:** the sim lets a belt feed a machine from any side of an inlet-edge tile — and out of its outlet face — which contradicts `world.rs`'s own module doc and GAMEPLAY.md §9; the fix is one condition in `resolve_sink`, but it is a gameplay change that invalidates existing saves and the shipped demo line, so it is the owner's call. Nobody has watched the belt animation run live at speed, and every perf figure in the plan doc is off a debug build.
- **Machine load indicator (DONE, all three tickets, 2026-09-11; plan: `docs/MACHINE_LOAD_PLAN.md`):** the player can see which machines are flat out and which are waiting, and so where the bottleneck is, across the whole floor. The simulation measures each machine's load over a rolling 600-tick window and reports `working`/`starved`/`blocked` with a percentage and the reason it is waiting, on the wire (`machine_load_snapshots_json`, plus the same fields on chunk and tile snapshots) and in the save (ticket 1). Six `andon_*` stack-light models — three statuses in a short and a tall height — are pinned by voxgen tests against every machine body at every facing (ticket 2). Every recipe-running machine now stands one on its camera-visible corner, lit to the sim's own verdict, polled on the tick (a pause freezes it; a loaded save lights up on its first frame); hover gains a line in words (`blocked · 61% busy · output full`) and a load bar; `test_layouts.gd`'s `build_bottleneck_line` settles the shipped line behind the cooking vat so it reads red · red · green · amber · amber, and `scripts/screenshot-scene.sh --bottleneck` photographs it (ticket 3). **Amended 2026-09-12: the stack-light poles are gone.** After a playtest the owner did not like a pole beside every machine, so the six `andon_*` models and the scene nodes were removed; the indicator is the hover panel alone — a status glyph and word leading the load line and the load bar filled in the status colour. The simulation half and the polling are unchanged.
- **Staff pools MVP (DONE, all three tickets, 2026-09-12; plan: `docs/STAFF_POOLS_PLAN.md`):** the factory needs people, and people cost money every period (GAMEPLAY.md §5, §7). **Ticket 1 (sim) has landed:** four staff categories in `data/staff.ron`, a required crew per kind in `data/machines.ron` (now schema 2), per-tick whole-crew claims in stable machine order (a machine short of its crew pauses where it is and reads *starved* with a `staff` reason; a dock without its logistics head holds), payday out of the one money counter in file order — **amended by the quality pass's ticket 1 (2026-09-13, below): an unpaid head is idled, not fired, and leaves only if the next payday is unpaid too**, free `hire`/`fire` commands, `staff_snapshot_json`, `crew` on tile and shop snapshots, `understaffed` on the dock, and all of it in the save. **Ticket 2 (icons) has landed:** four voxel props rendered through the icon pass, one per category (hard hat, spanner, taped carton, pallet jack). **Ticket 3 (Godot) has landed:** the **H** staff panel — one row per category with its icon, hired count, an assigned/asked-for meter with the numbers printed on it, the shortfall in words, wage and HIRE/FIRE, and the wage bill and payday countdown in the footer — polled on the tick like the machine load; hover gains `crew: 2 manufacturing`, an understaffed machine reads `▲ starved · short of manufacturing staff` and an understaffed dock `held · short of logistics staff`; the build-menu card shows a kind's crew under its price; `test_staff_panel.gd` proves it on a new `build_understaffed_line` fixture and `scripts/screenshot-scene.sh --staff` / `--understaffed` photograph it.
- **Quality pass (DONE, all five tickets, 2026-09-13/14; plan: `docs/QUALITY_PASS_PLAN.md`, decisions in CLAUDE.md's log):** no new features — correct, fast, fair, legible. **Ticket 1 (sim):** the payday rule no longer fires the unpaid silently — an unpaid head is idled for a period and leaves only if unpaid again, the wire says the next bill and the shortfall ahead of time and who left afterwards (`staff_snapshot_json`: `next_bill`, `shortfall`, `unpaid_last_payday`, per-category `owed`/`unpaid`/`left_last_payday`/`can_hire`); a fired head is still owed for its period and a hire needs the balance to cover the next bill, so neither HIRE nor FIRE dodges a wage; money is `Money` (`I48F16`) so a payout can no longer panic or wrap at 32 767; the rest of the sim was audited for exploits (see the log entry). **Ticket 2 (belt gaps):** `ITEM_MIN_SPACING_TILES` is half a tile, not a quarter — a fed lane now reads as a visible queue of boxes instead of one solid bar, at the same steady throughput as before (the vat is still the bottleneck); widening it exposed and fixed a packing bug where the belt's front item and the next tile's item occupied the same point at every tile boundary. **Ticket 3 (Godot bugs/perf):** the tick path reads a chunk *delta* (`chunk_delta_json`, only what actually changed) instead of the whole chunk snapshot, and per-frame polls are gated on the wire's own `revision()` so an unchanged frame parses nothing — the stress line dropped from 8.4 to 2.1 ms/frame; fixed a right-drag that never released when the mouse came up over a panel, first-person mouse capture fighting the pause/config panels, `R` rotating a tile hidden behind an open panel, and `X` not closing every modal like E/M/C/H do. **Ticket 4 (staffing legibility):** the wage bill and payday countdown are on the always-visible clipboard and in the H footer, drawn from the wire's own `next_bill`/`shortfall`; each row names what asks for its category and what the category does; the footer warns before a payday it cannot cover in the ticket's own words (`▲ next payday short by 6 — 3 heads go unpaid`) and disables HIRE per the wire's `can_hire`; a payday that idles or loses heads is announced through the same mechanism a lost contract uses. **Ticket 5 (polish sweep and docs):** every screen walked at 1600x900 and every PNG read, five things fixed. A hover or config panel could land squarely on the legend, on the contracts clipboard or under the hotbar, because it only ever flipped off the right and bottom edges of the viewport — it now tries all four quadrants around the cursor and, failing those, clamps on screen and pushes clear of whichever plate it still overlaps (`_panel_position_near`/`_pushed_clear_of`). The legend is five lines instead of four, so the panels line (E/M/C/H/P) and the configure hint no longer run back under the clipboard — ticket 3 left that one by name. `_announce_contract_changes` shows two notices raised in the same poll (two contracts lost at once, or a loss and a signature together) in turn, instead of the second silently overwriting the first before either was ever read (`_contract_notice_queue`) — ticket 3 left that one by name too. A build-menu card's crew cost is two lines (`crew: 2 manufacturing` / `4 / period`), paid for out of the card's own padding by a smaller build-card icon, because joined on one line it overran the card and a taller card does not help (the scroll cap comes from the panel's height, not the grid's). A splitter's hover says `connected` / `not connected` rather than the wire's raw tile coordinates. A new game was also played through to its first payout by driving the real command boundary, and nothing in that run needed a wording or ordering fix. Finally this doc, CLAUDE.md's decisions log and `docs/QUALITY_PASS_PLAN.md` were reconciled across the whole pass. `cargo test --workspace` and `scripts/verify-core-loop.sh` are green.
- **Dock capacity, crusher pulp, SHIFT auto-buy, first-person camera (DONE, 2026-09-16):** four playtest asks in one pass. The shared delivery pool has a per-item ceiling (`DELIVERY_STOCK_CAPACITY`, 48) so a dock with nowhere to put a box refuses it and the belt backs up — read on the clipboard as `20/48` with a bar and `⚠ FULL`, and on the floor as the dock's deck lamps pulsing, since emission multiplies vertex colour here and a red lamp is impossible. A working crusher throws pulp on its own beat (five flecks every ten *work* ticks, so a starved or blocked crusher throws nothing), reviewed through the new `screenshot-scene.sh --crushing` flag. Holding SHIFT while placing buys a replacement after the placement lands, with the price named in the prompt in all three states. And entering first person no longer leaves the camera at the isometric pose until the first input — `_set_view_mode` had a third opinion after each branch had already placed the camera.
- **Production stats — the P screen (DONE, both halves, 2026-09-13):** Factorio's production GUI. The simulation keeps a per-item ledger of what recipes made and used over four rolling windows (5s/1m/10m/1h) and for all time — a paid contract counts as using what it ships, buying and filling count nothing — polled over `production_snapshot_json` and saved with the world. **P** opens the PRODUCTION panel: rows biggest first with the item's icon, MADE and USED bars with counts and per-minute rates printed on them (over the observed span, scaled against the largest count), a window selector plus all time, one modal at a time with E/M/C/H. `test_production_stats.gd` proves it; `scripts/screenshot-scene.sh --production` photographs it. The sim half was written on 2026-09-12 and left uncommitted in a worktree; it was picked up, merged behind the staff pools (a craft records at the same points a crew is claimed for) and finished with the Godot half in one pass.
- **HUD features from the owner's playtest (DONE, 2026-09-16):** five
  requests, three parallel worktrees, no Rust change. Being short-staffed
  is now said on the always-visible clipboard (a **SHORT-STAFFED** strip
  naming the categories and the key), at the top of the staff panel (a
  one-sentence verdict) and on the row itself (a `▲ SHORT` chip, a warn
  border and a line saying what hiring would unblock); HIRE/FIRE name
  their cost and a refused HIRE says why in words; the payday rules are
  written out in the panel. **SHIFT+click a market crate** buys every unit
  the balance covers in one command, with the quantity printed on the
  slot. Raw stock moved off the hotbar onto a **right-hand STOCK plate**,
  one row per priced ingredient. The keybind legend **folds to a labelled
  stub** (its button, or F1).
- **The HUD fits 1280x720 (DONE, 2026-09-20; issues #62, #63, #69, #17).**
  Every panel knew how to yield height and none knew how to yield width, so
  at the smallest supported window the staff panel, build menu and contracts
  board were clipped on both edges with `FIRE −1` and `SIGN` off the screen.
  **A control the player cannot reach now outranks every other layout rule**
  (UI_UX §5c/§5f): a modal measures the window and gives width back by moving
  a row's prose to its hover, then taking fewer grid columns, then scrolling.
  The market wraps into a grid and dodges the right-hand plates, the staff
  explainer is shown whole or not at all, and the clipboard spends its hint's
  line to keep the docket cut on a line boundary. `test_hud_plates.gd` sweeps
  all five modals at 1280x720. Two findings deferred as issues: the machine
  hover cannot clear an open legend at that size, and headless font metrics
  are narrower than a real window's — so only a render can prove a width.
  Issue #88 (a building's configuration panel over the STOCK, CREW and
  CONTRACTS plates) was folded in, being the same fix.
- **Settings and a development menu (DONE, 2026-09-21).** The game has a
  settings screen, reachable from the main menu and from the pause menu — the
  same eight rows hosted twice, replacing the button column in whichever one
  opened it. Music and sound effects each have an on/off and a level;
  movement speed, look sensitivity and field of view tune the camera; hall
  size scales the room the factory stands in, live, including behind the main
  menu. They are **presentation state**: a `Settings` autoload and
  `user://settings.cfg`, never `sim_core` and never the save format, so a
  save written before this branch still loads. The pause menu's old
  SOUND/MUSIC buttons are gone — they are rows of this screen now. **F3**
  opens a development menu over a running factory: GIVE MONEY (a real
  `debug_grant_money` command, because money is simulation state), TIME SPEED
  FROZEN/×1/×2/×5/×10 with a STEP button, and a live readout of tick,
  revision, speed, FPS, money and where `user://` really is. It is ungated in
  pre-alpha and deliberately not a modal: ESC does not take it and it pauses
  nothing.
- **Open threads:** each doc's own "Open questions" section (ARCHITECTURE.md §11, UI_UX.md §9, GAMEPLAY.md §11) is the real TODO list at this stage — nothing is blocked, these are just undecided-and-known-undecided.

---

## How to use this doc

- **Newest entries at the top of the Log.**
- **Keep it high-level.** A few sentences per entry: what changed in general terms and why it matters for where the project is at, not a blow-by-blow of every file/decision — that detail already lives in the docs' own decisions/changes and in `git log`/commit messages. If you're tempted to write a bullet per commit, zoom out instead.
- One entry per meaningful chunk of work (a session, a design pass, a milestone), not one per commit.
- **Update "Current status" above whenever it goes stale.** That section is what anyone should read first; the log is for "how did we get here," briefly.
- **Not every commit needs an entry.** Small/trivial changes (typo fixes, a tweaked wording, a minor config change) don't warrant one — see CLAUDE.md's commit convention for the actual rule on when to add one.

---

## Log

### 2026-09-26 — The P screen weighs output against the pay cycle's orders

The production screen (P) opens on a pay-cycle block: one row per product an
active contract orders, with REQUIRED per payday period (each mix scaled by
period over cadence), THIS CYCLE and LAST CYCLE. A finished cycle that
fell short of the orders running now takes the staff panel's short border
and prints `▲ short` in the warning colour. The cycle is the staff payday,
not a new clock; `ProductionLedger` rolls this cycle into last on the payday
tick. **Save schema 4:** those counts cannot be re-derived, so a v3 save is
refused by number rather than migrated (re-run `make-demo-saves.sh`). Review
caught two Godot checks that could never fail and a test loop that hung
instead of failing; both now go red under the mutation they guard (PR #193).
### 2026-09-26 — A sauce is earned by selling the sauce before it

The two progression gates count boxes now. `data/unlocks.ron` used to say
`after_deliveries: 10`; it says `after_sold: (item: "boxed_ketchup",
boxes: 100)`, and mustard says the same about `boxed_bbq`. "Sold" is a box
delivered into a satisfied contract window — the dock being paid — tallied per
item id in a saved ledger (`Economy::lifetime_boxes_sold`).

The old trigger was lifetime satisfied delivery *windows*, and the reason to
change it is that a window counted the same whether it carried one box or
eight, of anything at all. It therefore did not measure output: it measured how
many contract clocks happened to be ticking while the dock was stocked, so a
player who signed three orders crossed the gate roughly three times faster than
one with the starter order alone and the same factory — and mustard's gate could
be satisfied entirely by ketchup. Counting boxes of a named product makes each
tier depend on the line before it actually running, which is what the tier table
in GAMEPLAY §2 always claimed.

`lifetime_deliveries` still counts and is still shown; no gate reads it. A
gate nobody can ever open is worse than a badly tuned one, so five mistakes
are now refused — but in two different places, deliberately. A zero threshold
is refused by the loader itself (`UnlockRegistry::from_ron_str`, so it is a
startup panic); the four cross-file ones — an unknown item id, an item no
recipe produces, an item no contract's `mix` orders, and two gates on one
product whose thresholds fall — live in `validate_covers`, which only the test
suite calls, against the shipped files. That split is the 2026-09-08 rule for
`IngredientCatalog::validate_covers`, kept: a bad gate in `data/unlocks.ron`
fails `cargo test`, and a world built from fabricated content in some other
test is not dragged down with it. Tier order is `unlocks.ron`'s file order,
since 100 boxes of ketchup and 100 boxes of BBQ are not comparable numbers. The wire publishes the
product, the requirement and the count, so the padlock and the cookbook read
"34 of 100 boxes of Boxed Ketchup" with the name resolved by Godot's existing
display-name map and nothing about the rule written in GDScript.

**Existing saves stop loading, deliberately.** `SaveData` gains
`lifetime_boxes_sold` and the format goes to **version 5**; a v4 file is
refused with a sentence saying what changed. The first cut of this branch
tried one `#[serde(default)]` instead, and review found what that costs: gate
state is derived from the ledger on every load, so an old save came back with
both tiers re-locked, its gated raws unbuyable and a signed BBQ contract
stranded mid-run. A delivery window never recorded what was in it, so nothing
in a v4 file can be turned into a boxes-sold ledger without inventing it.
No release has ever written a save; the local demo saves are rebuilt by
`scripts/make-demo-saves.sh`, as after any version move.

**The gates are much later, and that is a flag rather than a tuning
decision.** The numbers are the owner's. Measured headless at 30 ticks/sec:
the first gate arrives at tick 52 000 (~29 min, 50 delivery windows) with
`corner_shop` signed alone, tick 33 000 (~18 min, 44 windows) with three orders
signed at tick 0 — where ten windows used to be a couple of minutes. A new game
signs nothing for you, so 50 is the floor a real run pays. The second gate is
measured from zero by `godot_project/measure_gate_ticks.gd` and lands at tick
141 000 — 78 minutes, and 89 000 ticks (~49 min) after the BBQ gate opened at
tick 52 000 — because the BBQ branch puts about 1.2 boxes per 1 000 ticks into
paid windows against the ~4.5 the four BBQ orders ask for. The 2026-09-20
measurement behind issue #55 found its trap stopped biting around 40 windows,
so at 44-50 it mostly does not fire any more — by side
effect, not by design, and #92/#93 are still the right fixes. Full record:
`docs/DECISIONS.md`, 2026-09-26.

**One thing is left for the owner.** GAMEPLAY §2's tier table now has the
mustard row it never got, with every cell filled from the content or the
measurement — except the tier *number*, which reads `?`. Mustard is a
recipe-complexity peer of BBQ (no new machine, no new handling axis, one more
recipe on the line that exists), so it is either a second tier 3 beside BBQ or
a renumber that pushes mayonnaise, hot sauce and the aged sauces down one. No
code reads the column.

### 2026-09-26 — Open-source licence and alpha-release groundwork

CONDIMENTAL has a licence: **GPL-3.0-only** plus a §7 linking exception for
Steamworks and other platform SDKs (`LICENSE-EXCEPTION.md`), with the name
and brand reserved in `TRADEMARKS.md` — free to build from source, and a paid Steam build for
convenience. `THIRD_PARTY_NOTICES.md` lists what a distributed build must
carry, and `cargo deny check licenses` (new job in `audit.yml`, allow-list in
`deny.toml`) keeps every linked crate GPL-compatible. Ahead of the repository
going public, the `@claude` workflow now answers only owners, members and
collaborators; `SECURITY.md`, `CODE_OF_CONDUCT.md` and a player bug-report
form were added; `project.godot` has a version (`0.1.0`) and a player-facing
description. Not done here, and still needed before a Steam build: desktop
export presets, a tagged release workflow, macOS signing, and the Steamworks
side. Decision record: `docs/DECISIONS_LOG.md`, 2026-09-26.

### 2026-09-26 — Mustard can be seen, and the guard that could not see it

Mustard now draws. `main.gd` gained the five tables the content needed:
`ITEM_MODELS` for the four solids, `FLUID_TINTS` for the three fluids,
`PRODUCT_TINTS` for the three recipes a product-showing machine runs, and
`HOPPER_FILL_TINTS`/`RACK_LOAD_TINTS` for the two raws. `test_mustard_line.gd`
stands the whole second line from raw through 31 real purchase-and-place
commands and proves it ships, and `test_screenshot.gd --mustard` photographs
it. That much was the ticket.

**The interesting part is that nothing went red.** `test_fluid_gallery.gd`
exists to catch precisely a fluid with no tint — issue #108's ruling, that a
guard must be made to REFUSE rather than merely be given more entries — and
it passed green on a branch where all three mustard fluids drew in
`FLUID_TINT_FALLBACK` and all three products in `PRODUCT_TINT_FALLBACK`. It
checked `FLUID_GALLERY_FLUIDS` against `FLUID_TINTS` in both directions,
which looks complete and is not: both are hand-written GDScript, so a fluid
missing from *both* is invisible to it. Two sauces had gone by under that.
The expected set is now derived — the fluids off `cookbook_json`'s recipe
graph, the product recipes off the same graph crossed with which machines
emit a `liquid` material class in their own generated models — so neither
side of the check is a copy of the other and a fourth sauce is covered by its
content landing. Deleting any one of the six mustard rows now fails by name.

**And the tints were measured, not reasoned.** The first cut spaced the three
fluids evenly in tint space, 25.6/255 a stage, which photographed as +15.3
then **+10.4** — inside issue #113's 10.7-11.8 noise band for reading one
fluid twice. The pipe body's lit face maps tint luma to pixels compressively,
and the frame is already crowded: measured on the gallery, `bbq` sits at 97.6,
`ketchup` 125.2, `bbq_mix` 128.3, `ketchup_mix` 141.0, `liquid_smoke` 159.6
and `vinegar` 178.2 of 255, leaving one free band. Mustard's three landed in
it at 203.6 / 221.8 / 235.9 — +18.2 then +14.1, clearing `vinegar` by 25.4,
which matters because vinegar is the fluid in the pipe on the *other* side of
the same crusher. Mustard's tightest pair is wider than two pairs the game
already ships. The band is narrow because the palette is nearly full; a
fourth sauce will have to take value off the older two.

One number the review added, recorded here because the fourth sauce will
need it: the **pipe's own steel casing reads 209.65** in these frames, and
every fluid is measured against its neighbours rather than against the
casing it sits in. `mustard_mash` is 6.09 *below* the casing and
`mustard_mix` 12.14 above it, both inside the anchor the legibility work
uses — so those two read as fluid because an empty pipe draws no liquid body
at all, a geometry cue, not because their tint stands off the metal. That is
fine for three sauces and it is not a rule to keep leaning on.

**One bug fell out that predates mustard.** THE COOKBOOK's sauce page read
its nullable wire fields with `String(entry.get("tagline", ""))`. An
`Option<String>` serialises as JSON `null`, `Dictionary.get` returns the null
rather than the default, and `String(null)` is a hard script error in Godot 4
— it aborts `_build_sauce_page` mid-build and the engine exits 0. Boxed BBQ
has been shipping a blank page. Every nullable string in `cookbook_panel.gd`
now goes through one `_text()` helper, and `test_cookbook.gd` opens *every*
page the book holds and fails unless each puts its own display name on
screen. Keying that assertion on a node rather than on content does not work:
`queue_free` is deferred to end of frame, so within one `_process` the
previous page's nodes are still findable.
### 2026-09-26 — A floating "!" over a full dock

A dock with all 32 places taken now shows a glowing amber "!" hanging over
it, so a floor with a stuck dock shows where the problem is before you hover
anything. The marker is a general one. Racks and machines can raise it under
their own key, through the same `_set_world_warning` call, when they need
the player's attention. Together with the hover's places grid, this closes
the owner's report on the docks.

### 2026-09-26 — a spend that starves the next payday asks first

Money leaves a CONDIMENTAL factory three ways, and wages are the only
recurring one with a cliff behind it: a head the payday cannot pay is idled,
and unpaid twice running it walks out. Nothing stood on the spending side of
that cliff. The shop took the money in silence, and the consequence arrived a
thousand ticks later at the payday, with the belt already laid and the balance
already gone. The staff panel's shortfall figure warned after the fact, on a
screen the player has to be looking at.

Now every spend that would leave the balance below the next payday's bill
raises a confirmation first: what it costs, what the balance would be, what
payday needs, the gap, how many seconds away it is, and what being unpaid
actually does. Two buttons — SPEND ANYWAY or CANCEL, ESC cancels — and it
covers all four ways to spend: equipment, ingredient stock, a contract fee and
a hire. It is a warning, not a gate; the player is told and then decides.

**The simulation is what decides.** `World::spend_preview` takes the exact
command Godot is about to issue and prices it with the same functions the
command arms charge with — `Shop::cost_of`, `Ingredients::cost_of` after the
same warehouse clamp, the offer's own price behind the same already-signed
filter, the category's wage against the payday bill — so there is no second
copy of a price rule to drift. Godot reads one boolean. The panel is one funnel
(`_spend_with_confirm`) rather than a check at each of the seven spend sites,
because the version written per site is the feature missing wherever someone
later forgets it.

It does not pause the world: only the pause menu does that, so the command is
*issued* on YES rather than replayed from the preview, and the tick that takes
the money is still the one entitled to refuse it. The tail of the interrupted
action runs on either answer — a CANCEL that skipped it used to leave build
mode holding a ghost of something no longer owned.

The hire's old hard refusal became a warning in the same pass, so all four
spends behave alike; `hire_warns` replaces `can_hire` on the wire, since a
field saying `can_hire: false` beside a hire that succeeds is a lie.
GAMEPLAY §7/§11, UI_UX §2b point 40 and §5, and ARCHITECTURE §4 are amended;
`docs/DECISIONS.md` carries the full record, including the four scope calls
the owner made and the two I argued against.

Noticed while adding point 40: §2b had two points numbered 35 — the vengi
camera rule the voxel generators cite, and the menu list's height budget. The
second took the next free number, 41, the way the 2026-09-16 entries did;
only the two records pointing at it needed the correction, since every code
comment citing 35 means the camera.

### 2026-09-26 — Each dock holds its own stock

The owner saw the docks behave as if they were wired together. A payout threw
coins over every dock, and one dock taking a case made every dock pop one.
The design was working as written: since 2026-09-08 a dock had been a
pass-through into one shared delivery pool, so the only fact in the world was
"what is on the docks". There was nothing per-dock for the presentation to
draw.

Each dock now keeps what it takes off its belt, in 32 places. Contracts draw
on the docks that can ship this tick, summed, taking from each dock in
topology order. Every dock a payout drew on records it. The coins now rise
over only those docks, and a case pops only on the dock that took it. The
dock hover names what stands on that dock and counts its places.

This supersedes the pool and its 48-per-item cap (DECISIONS_LOG
2026-09-26). The save schema is now 3, and older saves are refused rather
than migrated. This is the first of three PRs for the owner's report. The
hover's places grid and a reusable floating "!" over a full dock follow on
this model.

### 2026-09-26 — Mustard, the third sauce, and the fluid it could not drink

Mustard ships as content: two raws (`mustard_seed`, `turmeric`), five stages,
four contracts and a 25-delivery gate, all in `data/*.ron`, with no simulation
change behind it. The interesting part is the one shape that had to change,
and why.

The obvious recipe drank `vinegar` at the mixer, beside turmeric, the way
`ketchup_mix` does — which is the whole point of the tier, since vinegar is
the first raw two sauces compete for out of one budget. It also broke the
ketchup line in a brand-new game, long before a player could unlock mustard.
`World::resolve_network_fluids` works out what a pipe run carries by narrowing
candidates seeded from *every* recipe of *every* kind that touches the run,
and `RecipeRegistry::for_machine` knows nothing about unlock gates. Two mixer
recipes drinking one raw leaves nothing to narrow with: the vinegar tank's run
goes ambiguous, every machine on it stalls with `AmbiguousPipe`, and because
`World::tank_fluid` returns `None` on an ambiguous run the tank cannot even be
filled. Content that is still locked can stall a line that already ships.

So mustard's vinegar moved a stage upstream, to the crusher, which drank
nothing: `mustard_mash` is 4 mustard seed milled wet with 2 vinegar, and the
mustard mixer takes the derived mash down a pipe the way the BBQ mixer takes
derived ketchup. The bill per box is unchanged at 17 — only the machine that
drinks moved — and it restored full per-kind input disjointness, so the older
and stronger `recipes_on_a_kind_take_disjoint_inputs` rule survives untouched.

The rule that came out of it is recorded (2026-09-26, `docs/DECISIONS_LOG.md`)
and fenced two ways: `at_most_one_raw_fluid_enters_any_machine_kind` reads the
data file and fails on the shape, and `sim_core/tests/mustard_line.rs` builds
both lines out of real commands and proves every run resolves and a box of
mustard reaches the dock. Two more cases pin the near misses the crusher's new
fluid recipe creates — a dry crusher whose outlet touches a `ketchup_mix` run
does not poison it, and one vinegar run may feed a crusher and a mixer at
once. The principled fix, a resolver that reads the solid side of a recipe
too, is issue #184 rather than something improvised here.

Review caught the gate's own tests asserting a list against itself: they looped
over `data/unlocks.ron`'s `mustard_sauce` entry, so deleting an id from it made
the loop vacuous and the suite stayed green with an unfulfillable mustard order
on a new game's board. What a tier must hold back is now derived from the
content — every contract whose mix names `boxed_mustard`, every raw the recipe
graph feeds to mustard alone — and compared with the gate in both directions.
The general lesson for the next sauce: a gate list is only guarded by something
that does not read the gate.

Mustard's voxels are ticket M2 (PR #179); registering them in `main.gd`'s
model and tint tables is M3, so `test_ingredient_models.gd` and
`test_warehouse_rack.gd` fail on the content branch alone by design.

### 2026-09-26 — THE COOKBOOK gets a BASICS section, and the prose states no numbers

The Cookbook's tree answers "what can this factory make". It cannot answer
"what is the loop", "what is money for" or "what happens when a contract
expires" — none of that is derivable from the recipe graph, because it is
prose. The owner asked for the how-to-play half, under the same binding
requirement as the rest of the book: dynamic, never hardcoded.

Prose is content, so it went in a content file: `data/guide.ron`, six topics
(the loop, money, contracts, crew, building, growing), each a title, a
tagline and three or four paragraphs. The interesting part is the rule the
file states about itself: **a paragraph never writes a number down.** Every
quantity is a `{placeholder}` — `{starting_money}`, `{payday_ticks}`,
`{cheapest_buyable}`, `{first_gate_deliveries}` — and the sim answers all
sixteen of them from the registries on every poll. A price change rewrites
the sentence that mentions it; nobody edits a word of prose.

That only holds if it is enforced, so it is enforced from both ends.
`sim_core/src/sim/guide.rs` refuses at load to accept a title, a tagline or a
paragraph naming a placeholder the sim cannot answer, or an unclosed brace —
a typo is a load error, never a `{` on screen. A unit test strips the placeholders out of
every shipped paragraph and fails if an ASCII digit survives where a fact
exists, which is what stops the next author from typing "3000" because it
was quicker. `guide_facts` always emits the whole vocabulary, so a page can
never find a gap. `cookbook.rs` carries the topics with their placeholders
still in them and the answers alongside, and the panel's `_fill()` is the
only thing that puts the two together — formatting each kind the way the
book already formats it, so a tick fact reads as seconds and a content id
goes through the display-name map rather than being titled in Rust.

**Godot gained a view, not a vocabulary.** A row of chips at the top of the
tree, one per topic, titled from the file; a page per topic; the other
topics offered at the foot of each page. `test_cookbook.gd` checks the chips
against the wire, that every paragraph reaches the screen, that the value
shown for each placeholder is the value the wire answered with, that no
brace survives, that a tick fact reads as seconds, and that no sentence of
the prose appears anywhere in `cookbook_panel.gd`. Both halves were
mutation-tested before being believed: neutering `_fill()` and dropping the
chip row each turned the suite red in the way it should.

`{machine_count}` and `{cheapest_machine}` were renamed `{buyable_count}`
and `{cheapest_buyable}` while writing the prose — the shelf holds belts,
pipes and the dock too, so the cheapest thing on it is a belt, and a
placeholder called "machine" would have invited a future author to write a
sentence that was wrong by four.

New screenshot flag `--cookbook-basics`, which opens the first topic the
guide ships without naming it, for the same reason `--cookbook-page` does
not name a sauce. The screen is `docs/UI_UX.md` §5h.

Review caught four ways the first cut was still lying, and the fixes are
worth recording because three of them are the same mistake in different
clothes — a sentence that is true of the *file* read as if it were true of
the *game*. "There are {raw_count} raw ingredients you can buy" counts the
ones behind a gate too, and so did "{contract_count} orders on the board".
A count that means "right now" is therefore its own fact:
`{open_raw_count}` and `{open_contract_count}` join the vocabulary, asked of
the market and of `contract_available` at the current delivery total, and the
two file counts keep their own sentences ("of the {contract_count} orders in
the file"). The second review pass caught the sequel to that, which is worth
knowing because the fix for the first mistake walks straight into it: the
right fact in a sentence with the wrong tense. "Nine are on the board of a
brand-new factory" is true at zero deliveries and false from the tenth on,
because the number moves and the clause around it does not — so the prose has
to carry the same tense as the fact it asks for, and no loader or test can
see that half of it. A third paragraph claimed the machines could already make a sauce
this build does not ship; it now says what is actually true and durable —
what a machine makes is decided by what is touching it.

The fourth was structural. A title was validated at load but **not**
substituted on screen, so `{dock_capacity}` in a title would have passed the
loader and then shown its braces to the player: the loader read only the
tagline and the body, and the panel filled only the tagline and the body.
Both now treat all three the same, and the Godot test doctors a title with a
placeholder in it to prove it. `KEEP READING` also grew a trail crumb per
topic read, so a reader who browsed ten of them had ten crumbs running off
the modal; reading on sideways replaces the last crumb instead of stacking,
and the breadcrumb shows at most three plus a `…` back to the tree. The test
asserts the crumb **count**, not a width — a headless run lays nothing out.
The screen test now also compares each whole filled line against the wire's
own substitution rather than checking the pieces separately, which is what
catches a value landing in the wrong sentence. Every one of these was
mutation-tested before being believed: uncapping the trail, stacking the
crumbs, leaving a title unfilled and blinding the loader to titles each
turned the suite red.


### 2026-09-25 — THE COOKBOOK: the game explains itself, out of the data files

CONDIMENTAL had never explained itself. A player who bought a mixer had no
way to learn what it makes, what feeds it, or what the thing it makes is
worth — and every one of those facts already existed in `data/*.ron`, where
only the loader had ever read them. The owner, reading a design doc whose
sauce tree and recipe cards were generated from those files, asked for them
in game and set the binding requirement: dynamic, never hardcoded, and as
maintainable as possible, because sauces, recipes, balance, assets and names
are all going to keep changing.

So the feature is not really a screen; it is the property that the screen
tracks the content files, and the screen is what that property looks like.

**The sim decides.** New pure module `sim_core/src/sim/cookbook.rs` builds
the whole book from the registries with no `World`: the obtainability
fixpoint lifted out of `validate_chain` gives each recipe its tier (its depth
from raw, which is the tree's column), families root at each item a contract
demands and walk back through producing recipes, an edge is one family's
product being an input to another's, cost per case recurses the graph from
raw ingredient prices, the pay range is min/max of `payout / boxes` across
every contract naming the terminal item, and lockedness is derived — a family
is locked while any raw in its transitive inputs is still gated. One new
bridge call, `cookbook_json()`, read on open and on the `LOAD_POLL_TICKS`
cadence while open, because the lock half moves with lifetime deliveries.

**Godot renders.** `godot_project/cookbook_panel.gd` is a `RefCounted`
returning a column, hosted twice: as the B modal and as a pause-menu page,
because a modal that does not pause is wrong for a long read. The sauce tree
is the project's first `_draw()`, and it follows `_set_meter`'s discipline —
the layout is computed into a dictionary, put on the canvas with
`set_meta("layout", ...)`, and `_draw()` renders only from that, so a
headless test can read a layout no headless run ever lays out. The nodes are
real Buttons at explicit positions, so a click is a `pressed` signal.

**The property is a test, not a comment.** `test_cookbook.gd` pulls every id,
display name and contract name out of `cookbook_json` and fails if any of
them appears in a string literal in `cookbook_panel.gd`. It also checks the
drawn tree against the wire node for node and edge for edge, that a locked
family is a silhouette whose page is the gate and nothing else, that every
duration is seconds and the word `tick` appears nowhere, that the pages link
and the breadcrumb walks back, that B closes the other five and each of them
closes B, and that the book is polled on the tick only while open.

**What the data files gained.** Optional `tagline`/`blurb`/`icon` on
`Recipe`, `MachineKind` and `IngredientEntry`, blank rejected at load as
`BlankDisplayName` already was, plus a sibling `icon_names_json()` that
`_icon_for` now consults ahead of `ITEM_MODELS` — which is what makes
swapping an asset a data edit. The rule going forward: a Cookbook that needs
a fact the data files cannot state is a missing data field, not a Rust
change.

**Three consequences that look like omissions and are not.** No fallback
prose — a sauce with no `blurb` gets no blurb paragraph rather than a
sentence GDScript made up. No number computed in Godot except seconds, and
only because `TICK_HZ` is GDScript's. And no machine named even in prose: an
ingredient row says "solid, by belt", not "hopper, then belt", because a
machine name in a sentence is still content authored in GDScript.

B was free — it was the build menu's key until E took it over and was left
dead rather than aliased — so the legend grew a sixth panel key and was split
across two lines rather than widened a third time, which `DECISIONS.md` had
already called ("the next panel that earns a key will overrun it again").
Decision record: `docs/DECISIONS.md`, 2026-09-25; the screen is
`docs/UI_UX.md` §5h.

### 2026-09-25 — An empty belt is a running belt

Play-test note from the owner: on a long run of belt, only the tile under
the box moved its rubber, so a single item crossing the floor looked like a
spotlight walking down a dead line. The surface clock from issue #79 counted
only the ticks a tile advanced product, and that was one analogy too far —
a crusher with no tomato is not doing its job, a conveyor with no box is a
motor turning. Worse, still rubber meant both "jammed" and "nothing on it",
and the empty case is far the commoner, so the jam signal the clock was
built for could not be read off the floor.

`BeltSegment::tick_toward` now counts an idle tick too, so the clock stops
on a jam and on nothing else. The silence on the wire is unchanged and
deliberate — an idle tile dirties no chunk, because a delta per empty belt
per tick is the cost the counter exists to avoid — and Godot carries the
phase forward itself between snapshots (`_belt_idle_phase`). That is exact
rather than a guess: the increment is one per tick, so the phase the scene
draws while a tile is idle is the number the tile's next snapshot brings,
and the rubber does not jump when a box lands on it. UI_UX §6a's rule is
kept where it bites, and the 2026-09-21 decision record is amended in place
with what the play test refuted.

Playing that build turned up the other half: "the belts need to all be
synced up, they are a bit out of sync". A per-tile count of running ticks
drifts — belts are laid minutes apart, and one spell of back-pressure
leaves a tile a fraction of a tile behind for good — so a run read as a row
of separate machines. `moved_ticks` is now the TICK the tile last ran on
rather than a count of its own, which is the same formula at the same rate
and puts every running belt on one phase. A jam still freezes it; the
accepted cost is that a freed tile rejoins the phase instead of resuming its
own, jumping once under items that are just starting to move.

The review of that branch found the half neither test covered: a belt is laid
on a tick now as well. A freshly placed segment started its clock at 0, and
since the scene reads a placed tile exactly once — on the rebuild the
placement triggers — and an idle tile dirties no chunk afterwards, the one
tile the player had just bought was the one tile permanently out of step. The
same hole closed at load, where an empty belt now takes the save's own tick
rather than whatever a pre-rule save recorded.

### 2026-09-25 — A hover sheds a line only when that helps (PR #163 review)

PR #163 merged unreviewed, and its review after the fact found that the
yield ladder shed both lines whenever no rung cleared the HUD. It assumed a
shorter panel always covers less. It does not, because the placement search
is not exhaustive. At 1600x900, 147 of 720 cursor positions between the
legend and the clipboard dropped the crew line and the load bar and still
covered exactly as much as the full panel did. A rung is now taken only if
it clears, or if it covers strictly less than the full panel. Otherwise the
panel keeps its lines. `test_hud_plates.gd` sweeps a 32px grid of cursor
positions at 1600x900 and fails on the old ladder at (968, 72). The review's
stale figures are also fixed: the DEVLOG overlap (about 15,500 px², not
20,000) and the old "550x283, one pixel short" notes in DECISIONS.md and
the test.

### 2026-09-25 — No PR merges before it is reviewed

PR #163 merged on green CI with no review. The docs allowed that at the
time ("merging your own work unreviewed is fine for routine changes"). The
owner has ruled it out: every PR now needs a `VERDICT: APPROVED` comment
from a session that did not write it before it merges, and a review after
the merge does not count. CLAUDE.md, WORKFLOW.md, the `open-pr` and
`wrap-it-up` skills and the `pre-push` hook now say this, and the merge
step reads the latest `VERDICT:` line across a PR's reviews and comments.
The decision is recorded in DECISIONS_LOG.md and DECISIONS.md.

### 2026-09-25 — The same screenshot command twice is the same picture (issue #82)

The review render was not reproducible, and this project judges its art by
rendered frames — so every measurement taken through one was standing on
sand. `scripts/screenshot-scene.sh` builds its fixture and then lets the
scene's own `_process` run 45 frames before the shutter, and
`_advance_simulation` spends **real** frame deltas against the 30 Hz tick, so
the number of ticks between the fixture and the photograph was a function of
how busy the machine happened to be. Two runs of one command landed on ticks
10621 and 10622; the PNG hash and the draw-call counts tracked the tick
exactly. Worse than the drift was its shape: on a quiet machine the same
command can give the same hash four times running and look fixed.

The fix is Godot's own `--fixed-fps 60`, passed by the script. Nothing about
how the game ticks for a player changes — `_advance_simulation` is untouched
— the engine simply hands every frame the same delta, so the warm-up always
spends `floor(45 * 30 / 60)` = 22 ticks. The three HUD fades a fixed frame
rate cannot reach (money flyout, hotbar name flyout, contract notice, which
sit outside the tick on purpose so they keep fading with the pause menu open)
now read `_hud_now_msec()`, a clock `main.gd` sums from the deltas it is
actually given, instead of `Time.get_ticks_msec()`. For a player that is the
wall clock; for a render it is the pinned delta; and a frozen scene now
freezes its fades with it.

The clock was only half of it. A render aims the camera — `--closeup`,
`--first-person`, a fixture's framing — and then runs 45 ordinary frames, and
those frames poll the real keyboard in `_pan_camera` and see the real mouse in
`_input`. A key held in another window on the host panned the shot a tile and
a half between the aim and the shutter, about one run in four here, and
nothing said so: the tick was identical and only the picture moved.
`main.gd` has a `_camera_input_sealed` flag now, off for every player and set
by the harness on frame 1, and it shuts all five doors live input comes
through (key pan, right-drag, zoom, first-person look, first-person walk).
Measured with the engine made to report a pan key as held: on the same tick,
unsealed the command rendered `972e7d6bbea3` with the closeup framing undone,
sealed it rendered the canonical `52493c1c3da8`. PR #167 landed under this
branch while it was in flight and moved the render's window off-screen and
no-focus, which as a side effect stops the host's keystrokes reaching the
default path at all — instrumented, `window_is_focused()` is false on all 45
warm-up frames. The seal stays because `--on-screen` does take focus (true
from frame 20 of the same warm-up), and because a guarantee resting on a
window flag is one a later change to the window repeals in silence.

`test_screenshot.gd` refuses a run whose frames did not arrive at that rate
rather than photographing a load-dependent moment, prints `SCREENSHOT_TICK`
so two frames can be told apart by reading rather than guessing, and
`scripts/check-invariants.sh` keeps the script and the harness in step
(neither half of a non-headless renderer can be caught by the headless
suite). `test_camera_seal.gd` (`CAMERA_SEAL_OK`) proves each of the five
guards, and fails if any one is deleted; `test_frame_clock.gd`
(`FRAME_CLOCK_OK`) holds the clock rule: no frames means no HUD time, all three fades finish
on frames alone, and the warm-up spends the same 22 ticks twice running.

Measured at 1280x720 on the tip: before, `--bottleneck --closeup 4,0,10
--no-hud` gave four different hashes in four runs; `--fixture chain
--populate` gave one hash four times on a quiet machine and three different
ones under a six-way CPU load. After, the closeup is one hash across six runs
including two under an eight-way load, and the chain fixture one hash across
eight. **Re-render any baseline taken before this** — a pinned warm-up stops
the world on a different tick than a loaded one did, so an old frame is a
different moment rather than a broken one.

### 2026-09-25 — Screenshot renders stop interrupting the person at the laptop

`scripts/screenshot-scene.sh` used to open a 1600x900 window on top of
whatever the owner was doing, take focus, and (for any `--hover*` flag)
move the machine's real mouse pointer. Its window is now borderless,
no-focus and far off the desktop. Godot skips drawing a window macOS
reports as occluded, so the harness draws each frame itself. An
off-screen frame was compared pixel for pixel with an on-screen one and
was identical. The hover flags set `main.gd`'s `_pointer_override`
instead of calling `Input.warp_mouse`, which also ends issue #4's
cross-worktree cursor leak. Godot activates itself at launch and nothing
inside the process can stop it, so the script hides the Godot app as soon
as it comes to the front, and macOS gives focus back to the previous app
about half a second later. A container was considered and rejected:
Docker on macOS has no GPU, and a software-rendered frame is not the
game's frame. `--on-screen` restores the old window for watching a run.
### 2026-09-25 — The asset browser learns that a lamp is usually dark

The browser drew every `*.emissive.vox` mesh at full emission. The game does
the opposite: `main.gd` dims a machine's status lamp the moment its body is
built and lights it only while it processes, so a cooking vat's rim is a dark
brown ring for most of its life. Reviewing a lamp in the browser therefore
meant approving the state a machine spends least of its time in — and the
browser's whole point is that it cannot drift from the pipeline (issue #81).

The dim values had nowhere to come from: they were three numbers hardcoded in
`voxel_assets.gd`, the one part of the voxel pipeline with no representation in
`palette.ron`. They are palette data now. `palette.ron` carries a `lamp_states`
table written as *modifiers* of the `emissive` class — `emission_scale`
multiplies the class's energy rather than restating it — `voxgen` resolves them
and writes `models/lamp_states.json` beside `materials.json`, and
`voxel_assets.gd` and the browser both build their materials from that file.
Adding the table is a shape change, so `palette.ron`'s `schema_version` is 2.

The browser has a `running`/`idle` toggle on `l` and a `?lamp=idle` parameter
on its automation seam, so a screenshot can ask for the dark state. Measured
over the vat's lamp ring, divided by the vat's own stainless in the same frame:
the browser draws an idle ring at 0.193 against the game's 0.235, where before
it drew 1.006. Full record in `docs/DECISIONS.md`; the rule is UI_UX.md §2b
point 39.

### 2026-09-25 — A session now has a defined ending

`wrap-it-up` joins the skills. Sessions were ending on "that looks
done", which is not the same question as "can this worktree be deleted
without losing anything" — the one that actually matters, since an Orca
worktree is removed as a matter of course and takes any un-landed branch
with it. The skill is the sweep that answers it: take stock out loud,
finish or deliberately drop each item, land every branch through a
merged PR, prove `git status`, unpushed commits, the shared stash and
open PRs are all empty, and then report with an all-caps green banner —
or a red one naming exactly what is unfinished and where it lives.
`docs/WORKFLOW.md` gains a "Closing out a session" section stating the
two rules the skill enforces: the end of a session is not a reason to
file an issue, and a branch left in a worktree about to be removed is
deleted work. It defers to the `review` skill that landed on the 21st:
since nobody can review what they wrote, a session's own PR waiting on
its verdict is the one legitimate way to end with a PR still open —
which is also why the skill separates "is the work finished" from "would
deleting the tree lose anything". They are different questions and a
wrap-up answers both.

### 2026-09-25 — A machine hover gives up lines before it covers the HUD (issue #99)

At 1280x720 with the controls legend open, a machine hover always sat on
top of the legend. The best position the placement search could find still
covered about 15,500 px² of it. The issue said the panel was one pixel too
tall. Measuring showed it was 48px too tall: the panel is 456x283, and the
band between the legend and the hotbar is 235px.

The hover now drops lines until it fits (`_hover_yield_rungs`). It drops
the `crew:` line first, then the load bar, and never more than it needs.
Both are already shown elsewhere on the same frame: the CREW plate lists
every crew category, and the `% busy` figure stays in the text. The panel
is measured at full size every frame, so at 1600x900 or with the legend
folded (F1) nothing is dropped. We never fold the legend for the player,
because UI_UX §5f says the fold is theirs. The rule is written up in UI_UX
§5c. `test_hud_plates.gd` covers the ladder in both view modes, and
`test_staff_panel.gd` checks that the first rung removes the crew line and
nothing else.


### 2026-09-21 — A settings screen, and an F3 menu for cheating during development

Two asks in one branch, and the line between them is the project's own rule
about where things live.

**Settings are presentation.** Eight knobs — music on/off and level, sound
effects on/off and level, movement speed, look sensitivity, field of view,
hall size — in a `Settings` autoload and a `user://settings.cfg`, never in
`sim_core` and never in the save format. A save is a full state snapshot with
a version that is rejected rather than migrated; a volume slider in it would
mean a preference change could invalidate a save, and a save carried to
another machine would bring that machine's field of view with it. So a save
written before this branch still loads, which is the acceptance test for the
whole choice. It also settles the note `_toggle_sound` had carried since the
pause menu grew SOUND and MUSIC buttons: those two are gone, and are rows of
this screen.

One panel, hosted twice — the main menu and the pause menu build the same
rows and each swaps them in place of its own button column, the shape the
Load Game view and the quit confirmation already have. The rows are steppers
rather than sliders, because a drag cannot be asserted headlessly and each of
these is a short list of sensible values; every value is formatted (`100%`,
`×1.25`, `75°`) and the bar under a stepper is deliberately not the sim's
meter language. Hall size scales three shader uniforms together, so the room
keeps its proportions, and the title screen takes the same scale — the row
changes the room behind the menu as it is pressed.

**A cheat that moves money is a real command.** `PlayerCommand::DebugGrantMoney`
travels the same JSON wire everything else does, refused at zero or below
leaving the world byte-identical, because money is spent, saved and read off
one counter and a grant Godot kept to itself would be a balance the sim
disagrees with. Time speed is the opposite case: nothing in the sim knows
what a second is, so it scales the tick interval and the catch-up budget in
`_advance_simulation`. **F3** opens the panel over a running factory — it is
ungated in pre-alpha and is not a modal, so ESC does not take it and it
pauses nothing, which is the entire point of watching a cheat land. FROZEN
stops the ticks without pausing the panels or the camera, and STEP spends
exactly one.

`test_screenshot.gd` now forces the defaults before every render and takes
`--set key=value` to move one knob for the frame, so a review shot is the
same on any developer's machine and a screenshot of what a setting *does* is
reproducible rather than a temporary source edit.

### 2026-09-21 — A dock's lamp is about that dock

The "backing up" pulse on a dock's deck lamps was read off the shared
delivery pool, which caps **per item id**. `Economy::delivery_pool_full()`
is an `.any()` over those ceilings, so the moment one sauce filled its 48
units every dock in the world started pulsing — including a dock shipping
the other sauce into 38 units of room. That was invisible while the game
shipped one product, and the BBQ branch made it a real misread: the lamp
sent the player to inspect the line that was working.

Docks do share the pool, and that part is deliberate (a second dock is
another door out of the factory, not a second ledger). What is not shared
is *jamming*: `Dock` now records `blocked` at the moment `try_absorb`
refuses a box, `World::tick` clears it for every dock at the top of the
consumer pass, and `DockSnapshot::full` carries it to the lamp. It is
derived, not saved, so the save format is untouched. The clipboard's
`⚠ FULL` line still reads the pool, on purpose — it shows the per-item
levels next to it, so there it names something.

Covered by two sim tests plus a world-level one on the BBQ fixture, a
presentation test that only the named dock blinks, and
`screenshot-scene.sh --split-docks`, which runs the BBQ branch on until
its two docks disagree — a fixture the old code could not have built.

### 2026-09-21 — The adversarial review standard is a skill, not prompt text

The review bar this repo actually holds work to lived nowhere in the repo: it
was prompt text one coordinator session held during the 2026-09-20 sweep, so
every later session either reinvented it or reviewed to a weaker bar without
noticing the bar had moved (issue #124). It is now `.claude/skills/review/`,
beside `verify`, `screenshot-review` and `open-pr`, listed in `CLAUDE.md` and
pointed at from `docs/WORKFLOW.md` where merging is described.

What it says: review in your own worktree off your own `cargo xtask` build;
re-run the *issue's* reproduction rather than the author's test; reproduce the
failure on the pre-fix commit and show it gone on the tip, naming both; and —
the load-bearing step — **revert each fix in turn and confirm the new test
fails**, because a test that passes without its fix keeps passing through the
regression. Count the `_OK` markers by hand and read the PNG yourself, since
Godot exits 0 on a parse error and `assert()` does not stop the run. A
re-review is not a diff of the rework: the first review's "what I verified"
is stale the moment the tip moves, so it is re-run.

Every claim in it was re-measured before it was written down, which is the
rule the skill itself argues for. `gh pr review --approve` on a PR of this
account answers `Review Can not approve your own pull request`, and
`--request-changes` the equivalent, so the verdict convention — a `--comment`
review whose first line is exactly `VERDICT: APPROVED` or
`VERDICT: CHANGES REQUESTED` — is recorded as a decision in
`docs/DECISIONS_LOG.md` and `docs/DECISIONS.md`. The `assert()` claim was
re-probed on Godot 4.6.3: it prints `SCRIPT ERROR: Assertion failed`, abandons
the rest of *that function*, and the caller runs on to exit 0. One stale
number fixed in passing — `docs/WORKFLOW.md` said 19 integration test files
under `sim_core/tests/`, there are 27 — and one that looked stale and was not:
the status section's 38 Godot proofs is right, because
`grep -c '^run_godot_script'` answers 39 by matching the function definition
too, the same off-by-one issue #138 recorded.
### 2026-09-21 — The warehouse rack loses its red, and the game calls it Stock Shelving

Two owner asks on one kind. **The red first.** The rack's load beams were
`paint_red`, picked in the W2 pass because blue-and-red is what real racking
is sold in. What that choice missed is that this kind's entire readout is
colour: its bays hold stock tinted per ingredient, and the stock a factory
holds most of is tomatoes — so the structure was painted the same family as
the thing it was framing. The beams went green first — a value-matched
#32602F, since every reading the rack was tuned by (the goods against their
backdrop, the tint dead band) is a reading in value, and a brighter #3A6B33
cut measured +11.6 luma and was rejected for it. Then, on the owner's second
call, they went `paint_blue`: the uprights' own colour, so the frame is one
paint and no hue at all is spent on structure, leaving the tinted goods as
the only saturated thing the kind shows. The beams still read as beams —
under the fixed isometric key their lit top faces render at 134.7 against the
upright faces beside them at 55.7, shading doing what the second paint did.
Value held across both swaps: the beam pixels measure 76.4 where the red ones
measured 74.7. `assets/voxels/icons/warehouse_rack.png`
was re-rendered with it: that icon is committed art the build menu and hotbar
load at runtime, so a model recolour that skips `voxgen --bin voxicons` leaves
the old colour sitting in the UI.

**Then the name.** "Warehouse Rack" was never written down anywhere: the
build menu, the hotbar and the hover panel all title-case the kind id, and
the id is `warehouse_rack`. Renaming the id would have meant a save-format
break (ids are save keys and this format rejects rather than migrates) plus
`shop.ron`, `storage.ron`, the voxel asset and some 440 lines of measured
design prose, to change one string in three panels. So `MachineKind` gained
the optional `display_name` that `Recipe` and `IngredientKind` have carried
since issue #60, `World::display_names` now gathers all three files, and
`data/machines.ron` says `display_name: "Stock Shelving"`. The id stayed put.
The rule that came out of it is in DECISIONS_LOG: **an id is a key, a label
is content** — and it is now the documented way to rename anything the player
reads.

### 2026-09-21 — The spice mix is a sack, and a machine shows the sauce it is really making

Two asks in one branch. The first is art: `item_spice_mix` was a tin, which
at icon size read as a drum next to the molasses jar, so it is now a sack cut
from `item_sugar`'s pattern — smaller in every dimension (a voxgen test pins
that, and that neither sack ever borrows the other's cloth), in a new
`sack_spice` brown, with its blend showing at the mouth above both the cloth
and the label.

The second is the real fix. The bottler and the packer drew their sauce in
`sauce_red` no matter what they were running, so a BBQ line bottled ketchup
on screen; with more sauces coming, one model per sauce is not a shape worth
starting. Instead **a machine's `liquid` material class now means "this is
the product"**: `machine_product` paints it in a neutral `fluid_pale`,
`voxel_assets.gd` already names each class's mesh, and `main.gd` tints that
one mesh from `PRODUCT_TINTS[recipe_id]` whenever the machine's live recipe
changes. A new sauce is a row in that table, not a new asset. The tint is
keyed by *recipe* rather than by output buffer because mid-craft the buffer
is empty; an unregistered sauce falls back to untinted pale, so it reads as
unfinished rather than as the wrong sauce. The cooking vat's sight glass had
the same bug one stage upstream and is fixed by the same mechanism, which
leaves `sauce_red` with no consumer — it stays in `palette.ron` with a note,
because indices are file order and removing one renumbers everything below.

Rendering it found the rest of the defect was in the sim, not the renderer.
`Machine::choose` breaks a tie on `recipes.ron` file order, so a BBQ bottler
that ran dry silently adopted `bottled_ketchup` — visible as a red machine,
and as a stall readout that named a raw it was not waiting for. A tie now
goes to the recipe the machine is already holding
(`a_bbq_machine_that_runs_dry_keeps_its_own_recipe`). `test_bbq_branch.gd`
samples each machine's live recipe *during* the run and asserts the two lines
draw different colours, which is what catches a regression here at all — the
end-of-run state had already fallen back.

### 2026-09-21 — The market says what will fit, and the plate stops hiding rows (W4)

The last of the warehouse tickets, and the half the player reads. W1 put a
ceiling on raw stock and W3 drew the racks; the shop was still offering
crates of ten into a larder with room for four, and refusing them in silence.

**The crate carries the ceiling, because the crate is where it stops
something.** Each market slot now prints `In stock 180 / 320`, buys the number
it prints (`min(crate, capacity − owned)`), and when there is no room at all
says `AT CAPACITY ✗` in amber rather than wearing the unaffordable price's
`✗` — the two refusals have different cures, earn money or build a rack, and
one word for both sends the player to the wrong one. Shift-buy is clamped by
the same room, a click with nowhere to put anything issues no command at all
(it cues the shared refusal instead), and every figure is the wire's:
`IngredientSnapshot::capacity`, never a Godot-side `base + stack × bays`,
which is exactly the arithmetic only the simulation can do honestly while the
bays are a shared pool.

**A header for the pool itself**: `Warehouse: 1 rack · bays 2 / 6 · 4 free`
under the balance, with a meter beside it. That meter is the first bar the
warehouse has earned under §5e — bays used of bays built is a fixed fraction
the simulation defines — and it is the only one: a per-ingredient bar would
slide whenever a *different* ingredient took a bay, so the STOCK plate keeps
figures plus a `FULL` chip on any row at its ceiling. A purchase dirties no
tile, so `_buy_ingredient_slot` re-reads the warehouse itself rather than
waiting a frame; the pallets on the racks fill as the crates are clicked.

**The plate defect the W1 and W3 screenshots showed.** Both PRs photographed a
STOCK plate listing one of three ingredients behind a scroll thumb drawn over
the count. The cause was not the plate: the CREW plate made this column three
tenants deep, and at 1600x900 they wanted 796px of a 741px column, with
`_stock_ceiling_y` inviting the clipboard to grow until the plate was down to
its one-row floor. Two rank-4 faults were in play, so the tiebreak came from
the rank above — **reachability**. A docket stack is a scrollbar the player
can drag and C shows every contract in full; the STOCK plate is
`MOUSE_FILTER_IGNORE` from the panel down, so a row cut off it cannot be
reached at all. The shares are now 0.55 stock / 0.32 crew (CREW spends its
assignment line, which the H panel still shows), and the clipboard is capped
at the line where the whole list fits (`_stock_list_ceiling_y`) but never
below its own one-line floor. 1600x900 shows three rows and no scrollbar.

720p genuinely cannot seat three whole plates, so there the cut is
*announced*: the heading reads `+2 more · M: market` and the list reserves a
gutter so the thumb is never drawn on a count. A scrollbar on an IGNORE plate
is a mark, not a control.

`test_warehouse_rack.gd` proves the crate's pair of numbers against the wire's
own cap, the clamped quantity (four units of room buys four and charges for
four), the AT CAPACITY word and glyph, the dead click, and the header's
figures and meter. `test_hud_plates.gd` proves the FULL chip, the whole list
at 1600x900, and that a cut list says how many rows it is hiding. UI_UX.md
§5e/§5f carry the reasoning.

### 2026-09-21 — The rack shows what it is holding, bay by bay (W3)

The presentation half of the warehouse stock racks. W1 made a rack the thing
that raises the ceiling and W2 gave it a model; until now it rendered as a
handsome empty frame whatever was in it. `main.gd` now builds six pallet/goods
pairs per rack from W2's own geometry constants (`RACK_BAY_OFFSETS`,
`RACK_LEVEL_VOXELS`, mirrored from `assets.rs` with the generator named beside
them) and drives them off `storage_snapshot_json`, polled with the other
buffers: a bound bay shows its pallet, the goods on it are scaled to the bay's
fill, and the tint says which raw it is. Bays fill **bottom level first, near
bay before far** — the sim assigns bay indices rather than storing them, so
which shelf a bay is, is this file's choice, and it is the one that makes a
part-used rack load from the floor up.

The six tints are the ones W2 measured, not the hopper's: a hopper's heap is
seen down its own dark throat and a rack's pallet is seen against the factory
floor, so vinegar and liquid smoke are carried further from the floor's value
than their hopper twins. `test_warehouse_rack.gd` holds that line — it
enumerates the raws from the sim (`hopper_items ∪ tank_fluids`, so a gated
ingredient still needs a tint), refuses a table entry that is not one of them,
and fails any tint whose Rec.709 luma lands inside `assets.rs`'s measured
`RACK_TINT_DEAD_BAND` of 100..175.

**W1's refusal is spoken now.** A rack whose bays the stock cannot do without
will not come up, and that was silent: the simulation declines, and there is
no generic refusal channel to carry a reason (the #143 precedent). So the
prompt derives it from the three fields the wire already carries —
`used_bays > total_bays − this rack's bays` — and says `✗ REMOVE … its bays
are the only room the stock has` before the click, hover says `⚠ HOLDING`, and
`_confirm_build` measures whether the placement actually went before choosing
between the REMOVE and REFUSED cues. Hover resolves from either tile of the
1×2 and reads back the bays used, what is on them and the shared warehouse
total, because a rack standing empty beside a full one is not a fault.

No config panel and no status lamp: a rack has no crew and no contents to
declare, so `_config_click_at_tile` already closes on it, and the model has no
emissive voxel because a building that neither works nor stalls has nothing to
light. How many of its six shelves carry a pallet **is** the readout.

New screenshot fixture `--warehouse` (`Layouts.build_warehouse_row`): three
racks holding three raws — two with every bay bound, one with two of six, a
part-full load on the last bay of each ingredient, and four bare shelves to
compare the bound ones against; refused rather than photographed if the order
was clamped. Fourteen of its eighteen bays are in use, so `--warehouse
--remove-mode` photographs the refusal too. It is the one fixture exempt from
the harness's blanket `stock_larder`, because its subject is the larder.

### 2026-09-21 — The game opens in first person

The view a session starts in is first person now, not isometric: you arrive
standing in the hall the sky shader draws, among machines built at a human
scale, and TAB takes you up to the isometric view to lay a line out. One
constant says so (`main.gd`'s `DEFAULT_VIEW_MODE`), applied by `_ready` after
the HUD exists — the crosshair, the legend and the mouse capture all come
with the mode. Neither camera changed, and the isometric one is still the
fixed angle UI_UX.md §1 specifies.

`scripts/screenshot-scene.sh` deliberately does not follow the new default:
its framing flags (`--closeup`, `--zoom`, `--pan`, every `--hover*`) all
steer the isometric camera, so a review shot stays isometric unless
`--first-person` asks otherwise. Reasoning in `docs/DECISIONS.md`.

### 2026-09-21 — The perf gate stops failing on the neighbours (issue #146)

`test_perf.gd` is the only test in the suite that asserts a wall clock, and
this machine runs up to six worktrees' headless Godot at once. The same 120
frames measured 350.7 ms alone and 3733.1 ms beside a neighbour's verify,
against a 3000 ms ceiling — a red gate on a branch that had touched only
panel placement, reading exactly like a regression.

Both ceilings are now multiplied by a load factor the test measures itself:
a fixed 400k-iteration script loop, timed once before the measurements and
once after, worse of the two, `measured / 12 ms`, clamped to [1.0, 6.0]. A
quiet box therefore gets exactly the old constants (measured after the
change: 1.04x, ceilings 1567/3135 ms, frames 275.1 ms). The median of nine
samples, not the minimum — under sixteen spinning processes the fastest
sample was 11.9 ms, indistinguishable from idle, while the median was
38.3 ms. Raw milliseconds are still printed unscaled, because the ticket-4
before/after numbers quoted elsewhere in this log come off those lines.
Reasoning in [DECISIONS.md](DECISIONS.md) under today's date.

### 2026-09-21 — Raw stock has a ceiling, and a rack is what raises it (W1)

The simulation half of the warehouse stock racks feature. Until now
`BuyIngredient` credited `OwnedIngredients` with no limit but money, and money
is the thing a running factory produces — so GAMEPLAY.md §5's recurring
ingredient cost was one early purchase and then nothing. A holding is now
capped at `data/storage.ron`'s `base_cap` (20, free and unconditional) plus
`slot_stack` (50) for every **bay** the placed warehouse supplies, and
`warehouse_rack` — W2's model, 1×2, six bays, 75 in `shop.ron` — is the thing
that supplies them. New module `sim_core/src/sim/storage.rs` owns the
arithmetic and the loader; `World` gained `storage_bays`, `stock_cap_for`,
`stock_room_for` and `storage_snapshot`, `IngredientSnapshot` gained
`capacity`, and `SavedBody::WarehouseRack` carries no fields, so
`schema_version` stays 2. `godot_project/main.gd` registers the model and
nothing else: pallets, tints and the guidance plate are W3, the market plate
is W4.

Three rulings are in [DECISIONS.md](DECISIONS.md) under today's date. **R6
wins** — a rack is pure storage, with no output item, no picker, no topology
links and no tick-step-5 arm, so the hopper stays the only thing that meters
raw onto a belt; the sprint plan asked for all three readings of this and they
were three different buildings. **Bays are derived from the ledger, not
assigned and persisted** — nothing to migrate, nothing to drift, and a bay
frees itself the instant its stock is used, at the price that free bays are a
shared pool. **Six bays at 50, priced 75** — six is the model's own geometry,
and the price moves with it.

`sim_core/tests/storage.rs` is ten tests over the whole system, including the
headline invariant (a holding is never over its own ceiling, however the
warehouse changes), a removal that is refused because it would strand stock —
the first removal in the game that can fail — a save holding more than its
racks can, and the R6 pin. The cap's real cost was to the fixtures: every one
that ran a line bought a deep larder up front and none of them could any more.
Both layers now build the warehouse through the real commands, one rack at a
time, re-asking the sim for the room rather than restating its arithmetic
(`common::make_room`, `Layouts.make_room`), and the long runs top up mid-run
instead of stocking deep (`restock`) — 3 000 of one ingredient is sixty bays,
and `--full-dock` would have photographed a wall of racks instead of a dock.
`*_PLACEMENTS` counts now exclude the warehouse (`Layouts.line_placements`):
the layout under test is the line.

### 2026-09-21 — The warehouse rack has its art (W2)

First ticket of the warehouse stock racks feature, and assets only — nothing
is placeable yet. Three generators in `assets/voxels/generators/assets.rs`:
`warehouse_rack` (32×64×46, a 1×2 footprint, **two bays × three levels = six
slots**), `warehouse_pallet` (18×22×2) and `warehouse_pallet_load`, a
one-voxel `bulk_pale` slab for `main.gd` to scale and tint per item the way
`hopper_fill` already works. One palette colour appended, `tote_grey`. Five
new voxgen tests assert the bays are genuinely empty volumes, that a full
slot stands clear of the frame at every level, that the load beams span the
whole run, that the rack keeps to its footprint and stays under the cooking
vat's height, and that every bay is the same size.

Two design rulings came out of it and are in
[DECISIONS.md](DECISIONS.md) under today's date: **pay the fixed camera with
spacing, not with deletion** (the first design dropped its longitudinal load
beam to stop it hiding the loads, and read as a bike rack), and **judge a
voxel asset against the factory floor rather than vengi's black pad** (the
two backdrops give opposite verdicts; the rack's unusable tint band is
100..175 luma, in `RACK_TINT_DEAD_BAND` for W3 to read). Measuring the second
one found a real defect: the bottom bay was backed by the dark plinth instead
of by floor, and decking it in `steel_dark` like the other two narrowed the
band from 55..175. Issues #144 and #147 both closed by doing the work.
### 2026-09-21 — In first person the hover panel is parked, not followed (issue #105)

A panel that follows the cursor is the right idea right up until there is no
cursor. In first person the mouse is captured, the query point is the
crosshair at the centre of the screen, and the thing being described is
directly behind it — so `_panel_position_near`'s flip, which exists to keep a
panel off the tile it names, had nothing to flip away from. At 1600x900 it
found room to the right and looked fine. At 1280x720 the legend and the
right-hand column leave no quadrant that fits, the fallback clamps the panel
back on screen, and it lands on the crosshair: PR #78's own published evidence
frame shows a dock, the belt feeding it and the crosshair all behind the panel
describing them, and nobody read the frame that way at the time.

The fix is a different rule rather than a tuned one. First person parks the
panel in the lower-left corner, above the hotbar — the one region no fixed
plate claims and the crosshair never enters — and steps it clear of whatever
plate that corner still touches by the scoring the cursor rule already falls
back to. A candidate that contains the crosshair is dropped before it is
scored: covering the legend, which F1 folds away, beats covering the machine
the player walked over to look at. Being parked rather than followed also
stops the panel sliding about the frame on every mouse-look, which is the
other half of why following was wrong here.

The placement prompt anchors on the crosshair in first person as well and had
the same fault with nobody reporting it, so both panels now go through one
door (`_world_panel_position`) and the plates they dodge are one list
(`_hud_avoid_rects`). `test_hud_plates.gd` drives the rule with sizes of its
own — a headless font metric cannot prove a width, so what the test pins is
the rule given a size, and the 1280x720 render is the acceptance.

### 2026-09-21 — A locked ingredient is padlocked in the picker, and refused (issue #90)

`data/unlocks.ron` holds the three BBQ raws back until ten deliveries, and
until now it held them back from the market and from nowhere else. The
configuration panel's CONTENTS picker listed every raw the content files
define, the simulation accepted a click on any of them, and the hopper then
sat empty with no explanation beyond "in stock: 0" — the player had aimed a
bin at something they could not buy and nothing in the game said so.

The gate is now a rule on that click too: `try_set_hopper_item` and
`try_set_tank_fluid` refuse a locked id, the tank's refusal landing *before*
the purge so a refused declaration does not cost the player the vinegar
standing in the drum. The wire carries the verdict rather than the scene
re-deriving it — `IngredientsSnapshot`'s two lists became `ContentsChoice`
records (`id`, `locked`, `unlock_name`, `after_deliveries`) beside the
lifetime delivery count, and the market's `entries` still omits the locked
rows.

The picker keeps them. A locked slot is dimmed, wears a 🔒 in the corner the
✔ would have taken, is never marked as chosen, and the line under the row
names its gate and the distance to it — "🔒 Liquid Smoke — BBQ Sauce, 0 of 10
deliveries". The button is not disabled: pressing it issues the command,
the simulation refuses it, and the cue is read back off what the building
holds afterwards, so a refusal sounds refused. The build ghost's default
skips locked ids for the same reason.

Where the rule deliberately stops is `Place`'s `item`. That field is how the
ghost and the layout fixtures *seed* a bin, not a choice made against a
gate; refusing it would close nothing a player can reach and would cost
every BBQ fixture ten deliveries of real play before it could place a
hopper, which `test_layouts.gd` has no honest way to pay. The gate still
bites the bin — the raw cannot be bought, so it can never hold a unit — and
`unlocks.rs` pins exactly that. Twelve cases there in all, plus a new
section of `test_config_panel.gd` and `reference::open_every_gate`, which
lets a fixture state the premise that its factory belongs to a player who
earned it.

### 2026-09-21 — The bar for filing an issue is no longer low

The 2026-09-16 workflow rule said that if a finding was worth putting in a
report it was worth an issue. It worked as written, and the result was a
backlog of small, true, unprioritised findings — its longest-lived entries
were a script header claiming thirty Godot scripts where it runs 37, and a
`recipes.ron` comment citing 60 ticks/sec against a 30 Hz tick. Both were
one-line edits, and both cost more to file, read and rank than to fix.

The rule is now a gate rather than a reflex, in `docs/WORKFLOW.md` ("Fix it,
or file it — and prefer fixing it") and in the `file-issue` skill's first
section: fix it in the branch at hand when it is a line or two or a small safe
fix in the area you are already in; file only when fixing now would be wrong
(an owner's call, the save format, a design-doc contradiction, real work, or a
part of the tree this branch has no business touching) *and* someone would
pick it off the board ahead of the next feature. Roughly one issue per PR, as
a smell test; further deferrals go in the PR body, where review can promote
them. Nothing changes for a finding that passes the gate — same cold-reader
body, same labels. Recorded in `docs/DECISIONS_LOG.md` and `docs/DECISIONS.md`
Part A, which also annotate the 2026-09-16 entry this amends.

Dogfooded on the two issues the new rule cites as examples of what should
never have been filed: `data/recipes.ron`'s cook-time comment now says 30
ticks/sec (issue #137 — 80 ticks is under three seconds, not "a little over a
second"), and `scripts/verify-core-loop.sh` no longer claims a script count in
its header. It **counts and prints** the scripts it ran instead (issue #138,
whose own count of 38 was one too many — `grep -c '^run_godot_script'` matches
the function definition as well as its 37 calls, which is the argument for
printing the number rather than writing it down),
and the five live copies of "30 headless Godot scripts" that had drifted with
it — the `verify` and `open-pr` skills, WORKFLOW.md's command table, the PR
template's checklist and `verify.yml`'s header — now name the property rather
than the number. Dated entries elsewhere in this log keep their numbers: they
describe a moment, not the present. The other 15 open issues were left alone;
pruning them is the owner's call.
### 2026-09-21 — The BBQ epic gets the docs it shipped without (T5)

The second sauce has been playable since 2026-09-18 and gated since
2026-09-19, but the epic's last ticket — fold it into the authoritative docs —
was never done, and three days of sessions have been reading docs that
described a game with one sauce in it.

**Two invariants were bent and neither was written down.** `DECISIONS.md` now
carries a full record for each. *A machine kind may name several recipes, and
a placed machine runs whichever one its inputs satisfy* — first
fully-satisfied wins, `recipes.ron` file order is the tiebreak, an unsatisfied
machine holds the closest recipe so the stall readout can name a raw it is
really waiting for, and **there is no player-facing picker**, because the
plumbing already decides. *A recipe may draw several fluid networks, matched
by fluid id, and still deposits into exactly one* — the output half of the
2026-09-08 rule is untouched, which is what keeps a both-ways-plumbed machine
readable from the layout alone. Both were previously stated as deliberate
invariants, in a panic message and in a comment, so without a record the next
session finds a rule, a reason, and no note that it was overturned.

**The knock-on is recorded with them, because it is the real price.** Once a
kind runs several recipes, a mixer's outlet run is `ketchup_mix` or `bbq_mix`
depending on what is plumbed *into* the mixer — the thing being derived. That
is why `resolve_network_fluids` narrows the whole graph at once instead of
asking each machine in turn, and it is the root of issues #43 and #53.

**The gate got a record too.** `data/unlocks.ron` is the first progression
gate in the game: a threshold of lifetime successful deliveries releasing two
named lists, enforced in the simulation rather than hidden by a panel, with no
way to lock a recipe and no field to add one with — so progression costs the
tick nothing. The counter is saved and the gate derived from it, which is what
lets a balance pass on the file re-read correctly for every existing save.

**GAMEPLAY.md was wrong in two tables, not merely incomplete.** §2 and §4 both
said BBQ is gated behind "a smoker or low-and-slow reduction unit" — a machine
that was deliberately never built, because the 600-tick cook buys the same
identity with a number. Both rows now say what shipped, §4's "equipment
purchases *are* the tech tree" claim is qualified by the sauce that is not
bought, and §2 gains the third kind of gate the table did not have a slot for:
a tier that costs no capital still needs a reason not to arrive in the first
minute. §3 records BBQ as the game's first shipped *derived* sauce and why the
3:2 ratio is load-bearing; §9 gains the recipe-selection and progression-gate
bullets; §11's contract-gating question is now half-struck — a delivery count,
no reputation meter, and what remains open said plainly.

`docs/BBQ_SAUCE_PLAN.md` is marked superseded with a table pointing at where
each of its sections now lives, the way `QUALITY_PASS_PLAN.md` was before it,
and `sim_core/CLAUDE.md` gains the two invariants plus the `unlocks` module it
had never listed.

**Rendering the frame for this entry found the harness broken.**
`test_screenshot.gd`'s forward run still called `main.gd::_update_belt_frame`,
deleted by issue #79 the day before, so every frame of every forward run died
on a nonexistent method — and since a GDScript runtime error unwinds only the
call, it took the machine mechanisms, the dirty chunks, the buffers and the
fluid refresh with it. The harness wrote a PNG and exited 0 regardless: an
idle-looking factory photographed in the middle of a run, which is precisely
the lie those lines were added to prevent. Fixed here. A second finding from
the same render is filed rather than fixed — `--unlock-toast` photographs the
`SIGNED` notice instead of the unlock announcement it exists for, because the
toast is queued behind a showing notice on a wall-clock dwell the shutter
beats (issue #141).

One thing this pass deliberately did not do: close the gap the epic left in
the *game*. Building the BBQ branch still stops boxed-ketchup output for
~8 500 ticks and can cost a contract, with no warning at placement — issue #55
is closed `NOT_PLANNED` and its successors #92 and #93 are open and unstarted.

### 2026-09-21 — The README catches up with the second sauce, and every frame in it is fresh

The README had not moved since 2026-09-16 and was describing a one-sauce
game: it said in as many words that only one sauce existed, counted the
verify script at 19 Rust suites and 30 Godot proofs, and its gallery
pre-dated the glow/SSAO retune, the HUD refit, the hopper tints and the
config panel's corner. Every claim in it has now been re-derived from
`data/` and `sim_core/` rather than edited in place — the shipped line's
cost is 959, not "a little over 1,000"; the board holds 13 contracts of
which 9 are ungated; the market sells 6 ingredients of which 3 wait for the
tenth delivery. The chain section is two tables now, because BBQ is a
branch off the ketchup line and not a second line, and a table that pretends
otherwise is the wrong shape regardless of the words in it.

All fourteen gallery images were re-rendered rather than kept: a screenshot
of a build from five days ago is a screenshot of a different game's
lighting. Three are new — the two-sauce factory that is now the hero shot,
a mothballed building standing its crew down, and a close-up of the BBQ
branch taking finished ketchup in as a fluid. The captions say which world
state each frame shows, since "nine offers" and "three ingredients" are
true of a new game and misleading of an unlocked one.
### 2026-09-21 — A jammed belt's rubber stands still with the items on it (issue #79)

UI_UX.md §6a's rule — a continuous effect is driven by the thing's own work
clock — predates the belt animation and the belt animation broke it. The
frame index was `floor(tick * BELT_FRAMES / BELT_TICKS_PER_TILE) mod
BELT_FRAMES` off the global simulation tick, one number for every belt in
the world, so a line backed up solid behind the 240-tick cooking vat drew
its surface racing under items that had not moved in minutes.

Every belt tile now has its own work clock. `BeltSegment::moved_ticks`
counts the ticks that tile actually advanced product on — incremented by
`tick_toward` exactly when it reports movement — and `BeltSnapshot` and
`SplitterSnapshot` carry it to Godot, which takes the frame from it instead
of from the tick. `SavedBelt::moved_ticks` is `#[serde(default)]` at the
current `schema_version`, like `SavedHopper::auto_fill` before it, so a
reload stands where the save did and an older save loads at phase 0.

Two things about the shape of it, both written up in DECISIONS.md. It is in
Rust because Godot cannot work it out: a belt that stops moving stops
dirtying its chunk, so the presentation layer is never told a tile stood
still — and that same fact makes the fix free, since a jammed tile is simply
absent from the delta and Godot keeps the count it already has. And it is a
counter rather than a "moving" flag, because a flag would rejoin a global
phase and make the rubber jump the moment a jam cleared.

On the Godot side the frame is now per tile rather than per scene, which
`MultiMesh` has no per-instance visibility for; a tile not standing on a
frame is kept in that frame's group with a zero-scale basis **at its own
position**, so the group's AABB is unchanged and a frame change costs two
transform writes for the tile that changed and nothing for the rest.

The empty belt shows no motion at all now, deliberately — the idle crusher
throws no pulp for the same reason. New `sim_core/tests/belt_surface.rs`
(five cases, read through `chunk_snapshot`) and a rewritten section 3 of
`test_belt_shapes.gd`. What the headless test cannot reach is the transform
write itself: the dummy RenderingServer reads every `MultiMesh` instance
transform back as the identity. `scripts/screenshot-scene.sh --bottleneck
--closeup 1.5,0,20` is that half, and it is visible — four consecutive tiles
of one four-frame set stand on four different phases, where before the whole
world stood on one.

### 2026-09-21 — Clippy's backlog is gone and the baseline is zero (issue #29)

The lint ratchet has held `CLIPPY_BASELINE=28` since it was introduced; it
is now 0. The remaining warnings were six sites, none of them wrong code:
two six-deep tuple types that are now named `type`s with a doc comment
saying what the columns are, an index loop that only indexed one slice,
four deliberately kept-but-unused generator items that already explained
themselves in prose and now say the same thing to the compiler, and the
tick's step "2b" — which is not a list marker any markdown parser
recognises, and could not simply be renumbered because `dock.rs` and the
tick body both cite these steps by number.

`scripts/check-rust-lint.sh` stays a **ratchet** rather than becoming
`-D warnings`, now with both baselines at zero. A toolchain bump can lint
code nobody touched, and the right answer then is one reviewed line raising
a baseline with a reason beside it — not a red-by-default CI that everybody
learns to ignore. The rule is unchanged: lower freely, never raise without
saying why.
### 2026-09-21 — The dock says it is short of a clerk before anything is refused (issue #87)

The dock's hover drew its staffing line from `DockSnapshot::understaffed`,
which is a **per-tick** fact: it only moves on a tick the dock actually held
something and was refused it. Between firing the clerk and the first box
arriving — hundreds of ticks on a cold line, and never at all on one that is
also starved upstream — the tooltip said nothing was wrong while the
establishment plainly could not crew the building. This is the dock's half
of issue #20, which the machines got in the same pass and the dock did not.

The fix is a channel choice, which is why it was deferred rather than
one-lined. `crewed` — the establishment's standing answer, already computed
by `World::uncrewed_placements` — existed on `TileSnapshot::Dock`, but Godot
reads that only when the scene is *rebuilt*, and hiring and firing
deliberately dirty no chunk. A `crewed` taken off the rebuild channel would
have gone stale the moment a clerk was let go, which is worse than the blink
it replaced. So it rides `DockSnapshot` instead, on the POLLED
`dock_snapshots_json` the hover already reads every frame.

The tooltip now prefers the live refusal where there is one, because "held"
says the stronger thing, and falls back to the standing shortage in the
**same words** the machine hover uses, through the one helper both share
(`_uncrewed_reason_text`) — so the two readings cannot drift apart.

The silent window is now photographable: `scripts/screenshot-scene.sh
--unclerked-dock --hover-dock` builds the reference line with its one
logistics head fired and refuses to render at all if the window has closed,
so a sim change cannot hand back an ordinary-looking dock under that flag.

### 2026-09-21 — The configuration panel gets a corner, a section, and a name that is not cut in half (issues #106, #107, #116)

Three tickets about the one panel in the HUD that *commands* rather than
reports, and they turned out to share a cause.

The panel declares itself `CONFIG_PANEL_WIDTH` wide. It was not: a single
footer Label with neither a width pin nor a wrap made it 99px wider than the
constant every line of the positioning arithmetic is written against. That
is what left it with nowhere to stand — at 1600x900 it is 571px tall in a
698px band, and once it flips off the right-hand column it lands in the
legend's corner. The footer is pinned and wrapped now, like every other
label on the panel.

That alone does not fix it, because at neither shipped window size does a
position exist that clears both the legend and the right-hand column. So the
panel tries to step down or out first, and where it cannot, **the legend
steps aside for as long as the panel is open**. It is a ranking the panel's
own comment had stated for months without acting on: the legend is static
keybind reference the player can fold, the panel is live and carries the
warning that switching a vessel's contents destroys what is in it. The yield
is not a fold — the player's `_legend_collapsed` is untouched — and the
legend keeps *measuring* while it is yielded, so the panel cannot drift off
the free corner and hand it straight back, which is the oscillation issue
#87 records elsewhere.

Inside the panel, "Molasses" was drawn as `Molasse` / `s`, tail outside the
slot. A name with a space wraps at a join it already has; one without had
nowhere legal to break, so the font now shrinks a point at a time until the
widest single word fits, and the slot clips what the floor cannot rescue.
The measurement is taken off the **font**, not off the Label: turning
wrapping off to measure a Label only re-shapes it when the TEXT also
changes, so every single-word name answered "1px, it fits" and was never
shrunk — the first cut of this fix shipped nothing at all and the rendered
PNG is what caught it.

And `docs/UI_UX.md` finally has **§5g** for the panel, which had no section
at all despite owning every verb that feeds the factory.
### 2026-09-21 — Liquid smoke is lifted out of BBQ mix's shadow, and every hopper heap is now measured against every other (issues #113, #115)

`liquid_smoke` and `bbq_mix` stand next to each other in the BBQ branch — the
tank's run and the mixer's run feed the same vat — and until now they were
9.3/255 apart in the rendered frame, against 10.7–11.8/255 of ordinary
same-fluid shading noise. Two fluids a metre apart were, measurably, the same
colour. The fix is a **lift, not a darkening** (issue #100's rule: raise the
value, do not deepen the hue): `smoke_slate` goes #4D4521 → #706430 in
`palette.ron`, with `main.gd`'s `FLUID_TINTS` entry moved to match, and the
pair now renders **20.4/255 apart** — comfortably outside the noise.

It does not reach the 26/255 the ticket asked for, deliberately. Getting
there needs a luma of about 0.419 (#796C33), which lands liquid smoke
**9.1/255 from vinegar** — the gap is closed at one end by opening it at the
other. The six fluids are crowded on the value axis and no single tint can
fix that; issue #131 carries the real problem.

Alongside it, the hopper heaps get the guard the fluids already had.
`test_fluid_gallery.gd` has checked every fluid pair against a floor for a
while; `test_ingredient_models.gd` only ever checked that a raw solid *had* a
tint, so two indistinguishable heaps were legal. It now runs the same
pairwise `sum |dRGB| + 2·|dLuma|` metric over `HOPPER_FILL_TINTS` with a
floor of 0.31, calibrated ~10% under the worst shipped pair, and reports the
worst pair it found in its success line so the margin is visible without
failing. The fluid floor moves 0.24 → 0.26 for the same reason — the lift
made the old floor slack.

### 2026-09-21 — Seams that tell the truth: five stale claims and three flags that lied

A docs-and-guards pass over five issues that shared one shape: something in
the tree *asserted* a fact that the code no longer backed, or accepted an
instruction it could not carry out and reported success anyway.

**Three flags that were accepted and dropped (#123).** The rule this
settles, for the next flag as much as these: a flag the harness accepts but
cannot honour must stop the run, not decorate the log. `--contents-refill`
with no `--contents` reached `_refill_declared_tank()` only inside
`if _contents >= 0`, so it was dropped on the floor and the run still printed
`SCREENSHOT_OK`; `--contents` with no `--config` had nothing to press a
picker slot in; and a picker slot naming no fluid printed a `SCREENSHOT:`
note and carried on. All three now `_refuse()` — non-zero exit, no PNG.
Same class as #110 (`BBQ_FIXTURE_FAIL` printed and ignored) and #114
(`--closeup` silently ignored).

**A fourth, found by looking at the frame rather than the log.** Rendering
the PR's own screenshot with `--config 5,2` — the coordinate issue #129
quoted — came back a perfectly ordinary factory with no panel in it and
`SCREENSHOT_OK` beside it, because the fluid run's tank is at (4, 2) and
`_open_config_at` printed "no hopper or tank at (5, 2) to configure" and
returned. That is the same defect in a fifth place, and it is in this change
because the shot was read, not because the exit code was.

**The invariant only had teeth in one quote style (#127).**
`check-invariants.sh`'s hover section greped `'"--hover[a-z-]*"'` with the
double quotes baked in, while its own comment claimed it fails for any
`--hover*` string outside the table. GDScript takes either quote, so
`args.has('--hover-belt')` was invisible to it. Widened to both, then
break-tested: injecting a single-quoted eighth flag now fails the run
naming it, and the tree passes clean afterwards.

**Two comments asserting a shape the code cannot produce (#122, #129).**
`purge_runs_fed_by` said it walks `tank_discharges` because "a tank facing a
junction may legitimately have more than one entry" — but `fluid_tank` is
`Footprint(1, 1)` in `data/machines.ron` and discharges through the one face
it points at, so the topology pass cannot produce a second entry. The
`BTreeSet` is defensive, and now says so. `sim_core/CLAUDE.md` carried the
same claim plus the wording that hid #121 — "*Changing* a tank's
declaration" describes only the switch, which is why the first-declaration
case went unexamined for so long. `screenshot-scene.sh`'s
`--contents-refill` paragraph was left stale by #128 in the same way and is
reworded. These matter more than their size: `sim_core/CLAUDE.md` is loaded
into every session working in that crate, so a false sentence there is one
the next agent reasons *from* instead of measuring.

**A precedence docstring describing an order the code does not use (#127).**
`hover_targets.gd` said its flags are listed "in the order a command line
applies them (the last one given wins)". `requested_flags()` and `rows_for()`
both iterate the table, so `--hover-dock --hover` aims at whichever sits
first in the table. The seven `if _frames == 2 and _hover_x` blocks it
replaced did the same, so only the comment was ever wrong.

**`.claude/settings.local.json` is untracked again (#125).** It was
committed by #112. It is the per-clone list of tool permissions a local
session happened to approve, so tracking it ships one worker's approvals to
every clone and churns on any branch where a session approves something new.
`git rm --cached` plus a `.gitignore` line; the shared `.claude/settings.json`
stays tracked.

**And a duplicated paragraph in this file, removed.** The entry below for the
tooling pass carried both the superseded "43 combinations" sweep and the
corrected "48 combinations — 41 that must render, 7 that must refuse",
inside one entry, contradicting itself. A keep-both merge resolution put it
there: the conflicting commit was a *rewrite* of that paragraph, not a new
one. Keep-both is right for an append-only list and wrong for a revision,
and the difference is not visible in the conflict markers.

### 2026-09-21 — Confirming what a tank already holds stopped costing a tankful (issue #121)

Declaring a fluid tank's fluid empties the tank and purges every pipe run it
feeds. That rule is right and stays: carrying volumes across a recompute
would relabel fluid the player bought as something else, so a declaration
that renames a run has to destroy what the old name was attached to. What was
wrong is that the rule fired on declarations that renamed nothing. A tank
ships **undeclared** — it has no fluid id of its own and reads its name off
the run it discharges into — so the single most ordinary click in the panel,
ticking the vinegar the panel is already showing you, was billed at the full
price of a fluid swap. On the shipped chain that is 22 units in the tank and
40 in the run, gone, for pressing "yes" to the status quo.

`World::try_set_tank_fluid` now asks one question before the declaration
lands: does this only write down what is already resolved? Two halves, both
required. The tank's own contents must be that fluid or there must be none —
a tank is allowed to be empty, since there is nothing there to destroy. And
every run it discharges into must resolve to `PipeNaming::Carries(x)` for the
same x. A tank that discharges into no run at all is *not* settled, and
neither is one whose run is `Ambiguous` (two tanks disagreeing) or
`Unnameable` (no machine on it can say). Those keep the old destructive path
exactly. The settled case takes a new `FluidTank::settle`, which records the
declaration and skips `purge_runs_fed_by` — a sibling of `declare` rather
than a flag on it, because the two mean different things and only `World`
can tell them apart: the tank type cannot see a run.

The panel half was the same bug wearing the other hat. The picker's note
quantified what a click would destroy only on a tank that was **already**
declared, which is the one state the shipped chain is never in. Both
undeclared branches warn now: "another fluid destroys 62.0 units" on a run
something can name, where the carried fluid itself has just become free, and
"declaring destroys 62.0 units" where nothing can name the run and every pick
purges. That is UI_UX.md point 38 applied to the state the game actually
boots in, which the pull request that wrote the point had skipped.

Four new tests, three in `tests/tank_fluid.rs` around a new
shipped-single-tank fixture and one in `test_config_panel.gd`. The sim ones
were checked for vacuity by disabling the guard and watching the settled case
fail. `scripts/verify-core-loop.sh` is green.

### 2026-09-20 — The width pass, measured: a fit that was not conditional cost the board an offer

PR #102 taught every modal to yield width at 1280x720 and landed on main as
`c62aae9`. An adversarial review of it finished after the merge and its
verdict never reached anyone, so its three blocking findings and one request
have been live defects in shipped code since. This is that review, worked
through. Nothing here is new behaviour; all four are regressions or
false statements introduced by that PR.

**The contracts board lost a whole offer at 1600x900 — by five pixels.**
`_build_contract_board_row` put the offer's terms on a second line
*unconditionally*. That split is load-bearing at 1280x720, where the unsplit
grid wants 1324px against 1264px available. At 1600x900 it is not: unsplit,
the panel is 1380px in a 1600px window with 220px to spare. What the split
costs there is vertical — a card goes 120px to 157px, so three of them need
489px against a 484px cap and `snap_rows` cuts the list to TWO. Before the
width pass the board showed three offers at that size; after it, two. The
fix is the doctrine the PR's own decision record states and then did not
apply to itself: `_fit_contract_board_split` measures the panel unsplit
against `_modal_available_width()` and reparents the terms label into the mix
row when it fits, into the text column when it does not. At 1600x900 the
board is back to three whole offers with the sentence on one line; at
1280x720 the rendered frame is byte-identical to before.

**`snap_rows` is a cost, so it now needs a reason.** Moving the cut down to
the last whole row buys a tidy edge and pays height. The build menu was
already exempt on the grounds that the half-row past the cap is what says
"scroll". The production panel asked for it and paid 161px at 1600x900 —
four rows where main had shown four plus a readable fifth. The rule the two
share, and the one recorded: **`snap_rows` is for a stack whose rows are
DECISIONS the player signs** — an offer, a hire — where half a card is half a
choice. A production row is a readout; nothing is signed and nothing is lost
by cutting one, so the production panel drops it and the half-row does its
job (367px to 434px at 1600x900, 179px to 254px at 1280x720).

**"`autowrap_mode` is banned and unused" was false the day it was written.**
`docs/UI_UX.md`, `godot_project/CLAUDE.md` and `docs/DECISIONS.md` all stated
it as an absolute while 14 live assignments sat in `main.gd`. All four
statements are rescoped to what the code does, against every site checked
rather than assumed: seven shop and market labels sit in a button pinned by
`SHOP_SLOT_SIZE` (`main.gd:5719`) or `MARKET_SLOT_SIZE` (6312), the
contents-picker label under `CONFIG_PICKER_SLOT_SIZE` (10177), and six set
their own `custom_minimum_size.x` — `CONFIG_PANEL_WIDTH`, `CREW_TEXT_WIDTH`,
`PAUSE_BUTTON_MIN_SIZE.x`. The first draft of this correction said the
configuration panel escapes the rule by not being sized by a fit pass; it
does not — `_position_config_panel` reads `get_combined_minimum_size()` and
places the panel against the right-hand plates from it. The pin is the
exemption, not the absence of measurement, and the true rule is **pin the
width or do not wrap**. It matters out of proportion to its size because
`godot_project/CLAUDE.md` loads into every session in that directory: an
absolute stated there costs the next agent either working code or a session
spent proving the file wrong.

**Issue #88 was closed behind a test that could not catch it.** #88 is a
FLUID TANK's configuration panel drawn over STOCK and CREW at 1600x900;
`test_hud_plates.gd` opened a HOPPER. Reverting `_position_config_panel` to
its pre-#102 body failed only the 1280x720 case — the size the issue is
about passed with the fix removed, while a real render of the issue's own
repro (`--fixture fluids --stock --config=4,2`) put the tank's panel squarely
on both plates. The test now runs six cases: hopper and tank, 1600x900 and
1280x720, and the tank again low in the frame where #88 photographed it,
framed through the real drag-pan rather than a teleport. It was proved by
reverting: `the fluid tank's configuration panel at (1600, 900), standing 52%
across and 58% down the frame covers the STOCK plate by 168x100px`. The cause
is worth recording too, because the first guess was wrong: both the old and
new bodies measure `get_combined_minimum_size()`. The difference is ORDER —
the new one clamps `pos.y` *before* consulting the plates, the old one tested
them against the pre-clamp `y`, found no overlap for a panel opening low, and
then the final clamp lifted it into them. A bound has to be judged against
where the panel will be DRAWN, which is the rule `_panel_position_near`
learned the same way (UI_UX §2b point 33).

**And the branch per plate the same commit forbade.** `_panel_position_near`
avoided the legend, the contract panel and `_stock_rect` — but not
`_crew_rect`, in the very commit that created `_crew_rect` and wrote into
`godot_project/CLAUDE.md` that the right-hand column is ONE list and never a
branch per plate. A tooltip near the cursor landed on the wage bill. The
entry is added and the test now loops the column instead of naming one plate,
so the next tenant of that edge is caught by a test that already exists; it
was proved by reverting the three lines.

Not fixed here and still open: issue #106 (a mid-word break in the contents
picker) and issue #99 (the legend), neither of which is this review's
subject.
### 2026-09-20 — Two hopper tints, a guard that refuses, and a distance metric that can see value

Three issues sharing one shape: a lookup table keyed by content id whose
fallback makes a missing entry invisible. The same class as #67's market
icons and #68's fluid colours.

**Two raws were drawn the same colour (#80).** `HOPPER_FILL_TINTS` had
`tomato` and `sugar` and nothing else, so `spice_mix` and `molasses` both
came out of `HOPPER_FILL_TINT_FALLBACK` — measured on the `--bbq` fixture
their fill tops were 213.2 and 214.4 luma, 1.2/255 apart, which is the same
object to a player. They now carry their own palette colours verbatim
rather than lifted: unlike a fluid sitting in a shadowed steel bore there is
nothing to lift a hopper's upward-facing heap against, and `tomato` and
`sugar` were already their palette colours unaltered.

`molasses` ships nearly black on purpose, after looking at it in a frame
rather than reasoning about it. #100 is the precedent for a dark tint
reading as an empty vessel, but an empty hopper is a **pale** bin with its
ridged floor in plain view, so the failure mode is the opposite of a pipe's:
full it measures 57.4 luma against the empty bin's 106.1, and at 22% fill
— deepest in the bin, worst-lit by SSAO — 57.3 against 111.1. Lifting it
would have moved it *toward* the empty bin.

**The guard is the point, not the two colours.** Adding entries to a table
that silently tolerates missing ones fixes today and not tomorrow.
`test_ingredient_models.gd` now refuses a raw solid with no tint, and
enumerates the raws from the simulation's own `hopper_items`
(`World::raw_items`, deliberately unfiltered by the unlock gates) rather
than from a second hand-written list.

**The fluid gallery was policing a list, not the table (#108).** Its
saturation and distance checks walked `FLUID_GALLERY_FLUIDS`, a hand-written
six, so a seventh entry in `FLUID_TINTS` would ship unphotographed and
unmeasured with `FLUID_GALLERY_OK` and exit 0. Proved by adding a near-grey
`worcester`: green before the change, red after. The reverse loop closes it.

**The distance floor could not see the axis the renderer uses.**
`FLUID_TINT_MIN_DISTANCE` was 0.08 on summed `|dRGB|`, calibrated against
nothing, and summed `|dRGB|` cannot tell a brightness difference from a hue
difference of the same size — only the first is what this camera reads by.
Raising it would only have moved where it was blind, so the metric gained a
luma term (`sum|dRGB| + 2*|dLuma|`) and the floor is now 0.24, a stated 10%
margin below the measured worst shipped pair.

That pair is reported rather than excused. `bbq_mix` and `liquid_smoke`
render 9.3/255 of luma apart while one fluid photographed on two tiles of
its own run spreads 10.7 and 11.8 — the difference between the liquids is
smaller than the noise of measuring one of them twice, so they separate by
hue alone. Filed as #113 with the windows. Incidentally #108's own figure
did not reproduce: it named `bbq` <-> `liquid_smoke` at "dLuma 0.6", which
measures 19.8-27/255 depending on zoom, and 0.6 looks like a same-fluid
control.

**And the doc still described a deleted keybind (#89).** UI_UX §5a said a
hopper's contents are chosen on the ghost with **Q**. That key, the global
SPACE/V fill keys and the sim's `CycleHopperItem` command all went when the
configuration panel landed; the ghost now carries `hopper_items[0]` and
contents are retargeted by clicking the placed building, the fluid tank
included since #43. §5a has no numbered points and every `§N point M`
citation in the tree refers to §2a/§2b, so nothing was renumbered.

Deferred out of this pass: #113 (the value collision above), #114 (the
screenshot harness silently ignores `--closeup=X,Y,S` and renders a
whole-factory frame with exit 0), #115 (`HOPPER_FILL_TINTS` has no pairwise
distance guard at all; `tomato` <-> `spice_mix` is 12.7/255), #116 (UI_UX
has no section for the configuration panel, so #89's replacement text is
lodged in §5a).

### 2026-09-20 — The hall's two post-passes, judged against frames for the first time

Issues #32 and #2 closed; #3 measured and left open. All three were the same
kind of problem: a number nobody had ever checked against a rendered frame.

**Glow (issue #32).** `glow_intensity 0.7 / glow_bloom 0.15 /
glow_hdr_threshold 0.85 / SOFTLIGHT` was chosen for UI_UX §4's "mild bloom on
emissives" while, because of issue #25, **nothing in the game emitted at
all** — it was tuned against an input that did not exist. Retuned to
`1.0 / 0.0 / 1.2 / SCREEN` by scoring *selectivity*: what the pass adds around
a lit status lens against what it adds to the rest of the frame. 2.7x -> 69x.
The status lamp goes from a cream cube with a visible edge to an amber lamp
with a halo; in first person the roof fittings stop being white rectangles
indistinguishable from the skylight panels. Exposure is unmoved and
`SCREENSHOT_RENDER` is identical — a full-screen post-pass costs the same
whatever it is tuned to.

**SSAO (issue #32, extra scope).** The asset browser's work on issue #49
measured the game's SSAO while calibrating a baked AO pass against it and
found it near-inert; confirmed here — at `ssao_radius 0.9 /
ssao_intensity 2.4` the deepest crease in an isometric closeup lost 1.7
levels out of 133, which is nothing. **Raising the intensity is not the fix.**
`ssao_radius` is in world units, so an eye-level camera finds far more
occlusion per pixel than the orthographic one looking down from across the
hall: `0.9 / 16` reads well in isometric and, in first person, swallows the
belt's legs in a smear. Halving the radius reconciles them — `0.45 / 16.0`,
where 0.45 units is 4.5 voxels, *a fitting against a body* rather than a
machine against the floor. Judged in three framings, not two.

**The belt animation (issue #2).** Nobody had watched it run. `BELT_FRAMES = 4`
is confirmed correct, from a frame sequence rather than an opinion: on a
*running* belt at the repo's own closeup, a cleat-advance tick changes the
frame 3.81-3.88 against 3.33-3.56 for an ordinary tick — only +12%, because
the quarter-tile cleat step is buried in the continuous glide of the goods. A
live wall-clock run showed the phase sequence 1->2->3->0 advancing every 12-13
ticks with nothing skipped or repeated. A separate defect fell out of it and
is filed, not fixed: a **backed-up** belt keeps running its rubber (issue #79),
which both strobes and states the opposite of the truth.

**`_read_world_layout` (issue #3), measured and left open.** The issue quotes
40-80 ms per rebuild, off a **debug** `sim_core` — and `cargo xtask` builds
debug unless passed `--release`. On release it is **14.8-15.5 ms, 24-28% of a
55.4-61.6 ms warm rebuild** at 1539 placements; even debug is only 30.6 ms
(38%). Since it is a quarter of the rebuild, fixing it alone cannot remove the
hitch, and place/remove/rotate all bump `sim.revision()` so there is no cheap
cache. Numbers posted on #3; it stays open for milestone 8's tiered scheduler
rather than being improvised now. `test_perf.gd` reports the figure every run
so it cannot drift back into folklore.

Also filed: issue #82 — `scripts/screenshot-scene.sh` renders are not
reproducible run to run, because the scene ticks on the wall clock for 45
frames before it saves. That defeated the first glow sweep entirely (two
renders of the same settings differed more than two different settings did),
and both post-pass retunes above were measured through a tick-pinned harness
instead.
### 2026-09-20 — A tank says what it holds, and a recipe file may fork

Two fluid-identity issues off the board, both latent: neither bites the
shipped chain today, and both would have bitten the next content addition.

**A recipe file may hold independent production lines (issue #42).**
`validate_chain` demanded that every recipe be reachable from recipe 0 by a
shared ingredient id — a proxy for "nothing in this file is unreachable" that
happened to hold while every recipe descended from tomato. Mayonnaise (eggs +
oil, GAMEPLAY §2 tier 4) shares nothing with ketchup, so adding it to
`data/recipes.ron` would have panicked the game at load with a message about
connectivity. The rule is now per component and about obtainability, not
about the graph's shape: every component must contain at least one recipe
whose machine the player can buy and whose inputs are all raw or made
upstream, and every recipe must be reachable within its own component from
one of those. A file with two self-sufficient lines constructs; a component
with no buyable machine, or one whose only inputs nothing produces, still
panics at load. `machines.validate_covers` runs first so a missing machine is
named as a missing machine rather than surfacing as an unstartable component.

**A tank declares its fluid (issue #43).** Pipe runs are named by what the
machines at their ends claim, so two tank-fed runs feeding one machine that
draws two *raw* fluids are indistinguishable — the graph offers no way to say
which tank is the vinegar. `resolve_network_fluids` takes a declaration set
per run now, seeded from the tanks on it: a declared tank *replaces* its run's
candidate set with a singleton rather than intersecting, so the player's
statement wins outright and a contradiction between two tanks on one run is
reported as `Ambiguous` instead of silently picked. Only a raw fluid may be
declared, and switching a declared tank empties it, exactly as a hopper's
retarget does. The player-facing half is the tank's config panel: it now
offers the raw fluids in the same picker the hopper uses, marks the declared
one, and tells you in a line beneath whether the name was read from the pipe
run or stated by you.

`tests/tank_fluid.rs` builds the issue's exact case — one mixer drawing two
raw fluids, two tank-fed runs — and proves both that the graph alone leaves
it `Ambiguous` and that two declarations resolve it and the machine runs.
The two-fluid machine could not be photographed: no machine in
`data/recipes.ron` draws two raw fluids yet (that is why the bug is latent),
and a Godot fixture cannot inject a recipe.

**The save format's field-addition window is closed.** A declared fluid is
persisted state, so `SAVE_SCHEMA_VERSION` is 2 and a v1 save is refused by
number rather than migrated, per ARCHITECTURE §4. Declarations restore before
topology is re-derived, because pipe naming is *derived* from them — a loaded
world would otherwise be named as though nothing had been declared. The main
menu's demo saves are v1 and stop loading until someone runs
`scripts/make-demo-saves.sh`, which is what that script is for; they live in
Godot's user directory, not the repo, so this pass cannot do it for you.
`test_save_load.gd` reads the version out of the save it just wrote instead of
spelling it, so the next bump does not quietly turn its refusal case into
"loads fine".

**Declaring is destructive, and says so first.** Review caught that emptying
only the tank was not enough: `recompute_topology` carries pipe volumes
across by tile, so the run the tank feeds was silently *relabelled* — sixty
bought units of vinegar came back as liquid smoke. A declaration change now
purges the tank *and* every run it discharges into, before the recompute, and
the fluid is destroyed rather than credited or moved; the config picker names
the total it is about to destroy so the player consents to it instead of
discovering it. Refusing the switch while the run holds something was the
other option and was rejected: it strands a mis-declared tank with no way to
drain its run. A declaration no machine on the run has a recipe for now makes
that run read `Unnameable` rather than `Carries`, so the blame lands on the
tank the player just mis-declared instead of on the machine's outlet.

### 2026-09-20 — The BBQ ingredients get faces, and all six fluids stand in one frame

Two more of the BBQ branch's presentation gaps, both of the same family as
the last pass: a lookup with a fallback quietly covering for content that
had never been drawn.

**Spice mix, molasses and liquid smoke are real objects now (issue #67).**
The three ingredients the second sauce added had no voxel model, so
`_market_asset_for` fell through to its pipe fallback and the shop offered
three identical lengths of pipe — and the liquid-smoke tank's panel titled
itself with one. Three generators join `assets.rs`: a wide steel tin heaped
with paprika, a narrow tall jar of near-black syrup under a kraft label and
a steel cap, and a hooped slate drum with a brass bung. They were designed as
a *row* against tomato, sugar and vinegar rather than one at a time, because
the market plate shows all six at 64px side by side — each takes a silhouette
and a value none of the others has, which is what separates them at that
size. The jar took three rework passes: as first drawn its tapered neck and
wide cap ate 40% of the height and it read as a tin, exactly like its new
neighbour. `palette.ron` gains `spice_paprika`, `molasses_black` and
`smoke_slate`; the last is held equal to `main.gd`'s `liquid_smoke` tint, the
same rule `vinegar_amber` already follows. `item_liquid_smoke` is icon-only,
so like the vinegar jug it is built for vengi's camera and not the game's
(§2b point 35) — its bung sits where the *opposite* corner will see it.

The fallback is the real fix, though, and it is a test: `test_ingredient_models.gd`
walks every id `data/ingredients.ron` unlocks and fails if any of them
resolves to the pipe, if a fluid is missing from `FLUID_MODELS` or a solid
from `ITEM_MODELS`, or if a named asset has no icon. It was proved to fail by
removing an entry, not only to pass. The next ingredient someone adds gets
told, rather than shipping green with a pipe for a face.

**A frame that shows all six fluids at once (issue #72).** `bbq` and
`bbq_mix` sat at zero pressure in every fixture, so the tints #68 added could
not be photographed and had to be argued for in prose. `--fluid-gallery`
stands one chain of tanks, pipes, mixers and vats whose topology *names* all
six runs — vinegar, ketchup mix, ketchup, BBQ mix, liquid smoke, BBQ — and
then fills them. Naming is topology alone, so no ticking is needed; pressure
comes through the save-RON door `fund()` and `deliver()` already use, patching
every `volume_bits` to 10 units, which is within both a pipe segment's
capacity and a tank's. The fixture checks itself, and — after review — actually
refuses: if a run comes back unnameable, ambiguous, or at or below the
pressure `main.gd` needs to draw a body at all, the harness writes NO PNG and
exits non-zero. The first cut only pushed an error and then photographed the
degraded gallery anyway, exit 0, which is #70 and #75's failure mode for the
third time and was caught by the reviewer emptying a run. `test_fluid_gallery.gd`
runs the same check headlessly in `verify-core-loop.sh`, so the fixture cannot
rot quietly between renders; it also refuses an empty world first, so its
silence on the real gallery is evidence rather than a default. The frame
frames itself too — `_framing_bounds` pads a flat layout to 14 tiles on both
axes and shrank the subject to nothing, so the fixture carries its own focus
and ortho size as overridable defaults, and hides the HUD.

**And the gallery immediately earned its keep (issue #100).** With all six
runs standing side by side it was obvious that `liquid_smoke` could not be
seen at all: the tint #68 gave it was a near-grey, and the body it paints
stands inside a grey steel bore — measured at 1:1 it was 11 luma from the
steel either side of it, a full pipe nobody could find by eye. It is an
olive-brown now (53 luma and 170 summed-RGB from the same steel), the drum in
the market moves with it by the rule that keeps a substance one colour, and
the new test holds every fluid tint to a SATURATION floor rather than a
contrast one — the failure was chromatic, not tonal, and `vinegar` is no
darker than the old smoke was.

### 2026-09-20 — The HUD fits a 1280x720 window (issues #62, #63, #69, #17, #88)

Four layout issues, one root cause: every panel in the game knew how to yield
HEIGHT and none of them knew how to yield WIDTH. At 1600x900 that never
showed. At 1280x720 — the smallest window the project claims to support — the
staff panel wanted 1350px, the build menu 1412px and the contracts board
1380px, and a centred panel wider than its window is clipped on *both* edges.
What went was the outermost column of each: the icons on the left, and on the
right `FIRE −1`, the only way to undo a hire, and `SIGN`, the only way to take
a contract. The panels looked fine in the middle. You could not play the game
from them.

**The rule this pass adds** is that **a control the player cannot reach is the
worst thing a layout can do** — above every fault UI_UX §5f already ranked.
Width is now a budget like height (`_modal_available_width`), and a modal too
wide for its window gives width back in a fixed order: a row's prose moves to
its hover (`_fit_staff_width` — the staff row's ranked sentence, the per-tick
figure and the meter caption, all already on the tooltip); then a grid of
cards takes fewer columns and balances them (`_fit_grid_columns`, so six
crates are 3+3 and never 5+1, and `SHOP_SLOT_COLUMNS`/`MARKET_SLOT_COLUMNS`
now mean *the most* a grid takes); then the list scrolls, as it already did.
`autowrap_mode` is deliberately **not** used — a wrapping Label reports a 1px
minimum width and a height shaped at whatever width it was last *drawn* at, so
it measures as nonsense under `--headless`, where this layout's proof lives. A
sentence that must be narrower is broken at a join it already has: the board's
offer is now a mix line and a terms line.

**The other three.** The ingredient market was a fixed `HBoxContainer` that
neither wrapped nor scrolled — six crates measured 1260px in a 1280px window,
and a seventh ingredient would simply have been off the edge; it is a
`GridContainer` in a scroll now, clamped on `x` as well as `y` and pushed
clear of the clipboard, STOCK and CREW plates rather than reprinting `Money:`
a few pixels from theirs (#69). The staff panel's scroll box is flat instead
of nested, because the cut snaps between the *children* of the box it is
given and the nesting left it nothing to snap to but the explainer's own
lines (#63). And the clipboard's board hint is now a line the docket stack may
spend: at 1280x720 with every offer signed the stack was down to 12px against
a 43px first line boundary, which is the one case §5f had conceded to #17, and
dropping "sign more orders" from a clipboard whose every order is signed buys
exactly the line that puts the cut back on a boundary.

**Two things worth knowing before trusting a measurement.** Godot's headless
font metrics are **narrower** than a real window's: the staff panel passed the
new headless assertion at the very size the rendered frame clipped, and the
missing 262px was only found by instrumenting the fit during a real
`screenshot-scene.sh` run. A headless assertion is the regression net; the
render is the acceptance. And `get_combined_minimum_size()` can *under*report
what is on screen — the machine hover reported 299px wide while being drawn at
545px, and was positioned as if its right-hand 246px did not exist — so
anything placing a panel now takes `maxf(minimum, size)`. Where no position
clears every plate at all, `_panel_position_near` scores every candidate by
total overlapped area and keeps the least bad one rather than pushing off one
plate onto the next.

**And the panel a click opens, folded in (issue #88).** The configuration
panel's documented bound was failing three ways at once, all of them this
pass's own lesson: it named the clipboard and STOCK but not the CREW plate
that joined that column later; it tested an unclamped `y`, so the clamp lifted
a tank's panel into rows the test had just cleared it of; and its flip was a
fixed displacement from the anchor rather than a landing on the bound, which
left 15px of panel on the clipboard at 1280x720. One list of three rects,
clamped first, flipped onto the bound. The legend stays off that list
deliberately — at 1280x720 nothing clears both it and the column, and a
keybind reference the player can fold with F1 is the right thing to cover in
preference to a live readout.

`test_hud_plates.gd` sweeps all five modals (E, M, C, P, H) at 1280x720 and
asserts no panel is wider than the window, none is drawn off it, and no button
is unreachable — excusing only a button under a scroll's fold. A sixth case
opens a building's configuration panel with the camera looking at the building
and asserts it covers none of the three plates. Rendered and
read at both sizes. Two findings are deferred rather than fixed: the machine
hover still cannot clear an open legend at 1280x720 (the geometry leaves
nowhere to stand; the answer is a self-folding legend or a shorter tooltip,
which is a content decision), and the headless-metrics gap itself.

### 2026-09-20 — Two balance calls, both measured, neither one a number change

A pass over two BBQ balance/legibility issues that ended with the shipped
values kept — and with the evidence for keeping them written down, because in
both cases the cheap fix is the wrong one and nothing in the repo said so.
**Both sets of numbers were then corrected the same day under review**, which
is the more useful half of this entry.

**The bank at the moment of building decides whether the BBQ branch costs you
the board (issue #55, still OPEN).** Building the BBQ branch off a running
ketchup line stops boxed-ketchup output for about 8 500 ticks: the new pipe
runs and the second vat fill from the *existing* fluid network, roughly 35
boxes' worth of throughput into priming pipework before any comes back.
Delivery windows keep counting; three misses lose a contract. The issue's
cheap fix is to raise `after_deliveries` in `data/unlocks.ron` until the
player has slack by the time BBQ lands, so the number was swept — with the
protocol written down this time, because "when were the extra contracts
signed" turned out to move every number in the sweep. One order at the shipped
gate banks 22 and loses nothing; three orders signed **at the gate** bank the
same 22 and lose one; the same three signed **at tick 0** bank 7 and lose two.
An emulated gate of 40 banks 24 and loses nothing at all. So **ten stays** —
not because no number can work, but because 40 deliveries is nowhere near the
first session ten exists to sit inside — and #55 stays open with the
measurements posted on it. The two real fixes are filed: a warning at
placement time (#92) and a fluid network that will not hand a new consumer
100% of a network the old one lives on (#93).

The first version of this entry claimed the whole board is lost at *every*
gate, including a 50 000-tick run. It does not reproduce. Two mistakes:
contracts signed at different moments were reported as one experiment, and
`market_stall`'s death at tick 1 500 was counted against the branch when the
no-branch control loses it too. The corrected table and the protocol are in
`data/unlocks.ron`'s comment.

`test_layouts.gd`'s `--bbq` fixture was rebuilt to prove the setup: a splitter
merges the BBQ branch onto the ketchup exit belt so both cartons ride one
line, and the re-sign loop that used to paper over a lost contract is gone.
The fixture now also **fails loudly** rather than photographing a dead branch:
if the unlock gate has not opened by its tick ceiling, or if a snapshot will
not parse, it prints a `BBQ_FIXTURE_FAIL:` marker and asserts.

**The two sauce cartons read apart, and the first measurement that said
otherwise was wrong (issue #57, closed).** With both boxes on one belt at
whole-factory framing, `boxed_bbq` reads 117.2 mean luma against a
`boxed_ketchup` on the same belt in the same frame at 156.7 — a **39.5/255**
gap, independently reproduced at 42.5 by review. Against the method's anchors
(1.2/255 = invisible, ~26/255 = reads reliably) that clears the threshold by
half again. So the lid print introduced on 2026-09-18 does its job and **the
art is unchanged**.

The useful part is how nearly it went the other way, twice. The first pass
measured a 5.3/255 gap and concluded the art fails, because the silhouette
mask used the warm-pixel test earlier passes used (`r > g > b and r - b > 18`)
and `label_cream` renders near-neutral at (241, 239, 234) — the mask excluded
the printed panels, the exact pixels under test, and scored the cardboard
around them. It was caught when the one candidate that visibly repaints the
art measured *identical* to shipped. The corrected mask is the case's whole
silhouette with interior holes filled. The second miss was the control: the
gap was first judged against a 12.6 "noise floor" measured on cases standing
back to back in the packer queue — a different pixel population from a single
case on dark belt — and those six windows' coordinates were never written
down anywhere. **That floor and the ratio built on it are withdrawn.** The
same-kind control is one case on the shared belt across 14 renders, spread
5.0/255, with every window's coordinates in the record. Rules now written
down: **validate a legibility mask by measuring a candidate that visibly
changes the art before quoting any number it produces**, and **a control must
be the same subject kind as the subject, with its windows logged.** A chroma
metric was tested at the same time and came out negative — dC separates the
cartons by 3.7 against a same-carton 4.0 — because a coloured panel is a
handful of pixels and light-against-dark is the whole lid.

Also filed along the way: a screenshot fixture is never asserted to be
actually running before the shutter (#94), and the clipboard shows one docket
at 1600x900 with three contracts signed (#95).

### 2026-09-20 — The asset browser spins a model, and its shadows are measured

Two asset-browser issues, and the second turned into a measurement.

**A facing control (issue #36).** The browser drew every model once, in the
facing it was authored in, so the two things UI_UX.md §2b asserts about
rotation could be read in the doc and not seen: a carved doorway does not
survive a 180° spin (point 18), and an overlay that lines up on its host does
so in one facing out of four (point 29). There are **E S W N** buttons and an
`f` key now, in the sim's own clockwise order, and they spin the model the
way a placement does — a yaw about the **footprint origin**, which is what
the `.vox` origin already is (the footprint's centre at ground level), not
about the mesh's centre. The details panel's footprint line swaps its extents
with it, as `_rotated_footprint` does. The dock is the asset that makes the
point: at east and south you are looking into its loading bays, at west they
are a blank blue wall.

**Ambient occlusion, at the game's own strength (issue #49).** Baked per
voxel rather than screen-space: the occupancy grid the mesh is built from is
box-blurred to a near-Gaussian at Godot's own radius, sampled along each
face's normal, and the flat-half-space baseline is subtracted so an open
flank keeps the colour it had. It dims indirect diffuse only, as Godot's SSAO
does with `ssao_light_affect` at 0. The three.js `SSAOPass` was not the route
— it does not define `PERSPECTIVE_CAMERA=0`, this viewer's camera is
orthographic, and `web/vendor/` is fetched rather than vendored, so patching
it means vendoring a copy; and the voxel grid the occlusion is *about* is
already in hand at mesh-build time, which is cheaper and more exact than
inferring that grid back out of a depth buffer.

The first cut of it shipped a bug that review caught and the change is worth
recording for the shape of it. The sample was counted off the face's corner
index — which is the first EMPTY cell on a positive normal but the last SOLID
one on a negative normal, so the three negative normals sampled a voxel
nearer the surface than the baseline was derived at, the subtraction
under-cancelled, and half of every model's flat faces came back 2.9% dark
with nothing whatever in front of them. On the vat that was 48.8% of the
painted deck darkened where the game darkens 0.2% of it. The fix is to count
from the first empty cell on both signs; the lesson is that the bake is
arithmetic, so `tools/asset-browser/test-ao.mjs` now runs the viewer's own
module under node and asserts a solid cube reads fully open on all six face
normals, that each axis reads its two faces alike, and that an inner corner
is still occluded. It fails six assertions on the code that shipped.

Nor did the calibration catch it, and that is the more interesting half. The
strength is derived by measurement rather than chosen, but the first metric
was the deep end of the pixels each renderer darkened *at all* — which is
area-normalised, so "how much of the model got shaded" divides straight out
of it. A browser darkening 11x more of the machine than the game therefore
derived a constant that reproduced to three decimal places. `calibrate-ao.py`
now measures per MACHINE pixel — mean shading, the fraction of the machine
darkened past 1% and 3%, and the 99th percentile over the machine rather than
over the touched set — and **fails loudly** when the browser's coverage
diverges from the game's, because no strength can fix a coverage gap: it
scales every darkened pixel alike. The machine's pixels are found rather than
guessed, by rendering the same camera over the empty world (the world starts
empty, so that is just another `screenshot-scene.sh` run) and taking what
differs. Run against the frames that shipped, the new metric calls it at
49.7x coverage.

Two more of the same class turned up while re-deriving. The port had taken
`ssao_radius` and `ssao_intensity` out of `factory_environment.gd` and left
`ssao_power` behind, and power is a **gamma on the occlusion term**: it is the
difference between a crease finder and a wash, and no strength can stand in
for it, because a strength scales every darkened pixel alike. Porting it cut
the browser's coverage from 24.8% of the machine to 9.1% on its own. And
`test-ao.mjs` had assumed `AO_RADIUS` was a whole number, so a fractional
radius indexed the field at fractional coordinates and three assertions read
`undefined` rather than failing on the geometry. Two of three settings ported,
and a test that passed by rounding — the next person porting a render setting
will do the same unless it is written down.

Which pose to calibrate against turned out to be the real question. The bake
is object-space, so it is a property of the model, and the model is what the
browser exists to show; but the game's play camera is isometric, and a 0.45 m
world radius is a few pixels wide there. The same vat at the same settings has
67.8% of its pixels darkened by more than 1% when you stand beside it and
0.3% at the play camera. Matching the play camera would therefore have matched
the *camera*, not the model, and shipped a pass under one 8-bit level — issue
#49 closed on paper with nothing visible. The constant is derived beside the
line instead, and the gap is published rather than hidden: the browser removes
**3.49% of the machine's light against the game's 4.36%**, over 0.75x its
coverage and 1.26x its depth, at `AO_STRENGTH` **5.067** against PR #104's
`ssao_radius 0.45 / ssao_intensity 16.0 / ssao_power 1.6` — which is why #104
merges first. The rule it is held to is now stated precisely: never darker
than the game's own strongest *honest* presentation of the same geometry.

The frames themselves needed a gate too. `screenshot-scene.sh` occasionally
returns a frame captured before the fixture is built or before the pose is
applied — a plausible PNG of an empty hall at the default framing, which once
got as far as writing a constant. The calibration now refuses a before/after
unless the machine covers a sane share of the frame, the pass-on frame keeps
95% of the pass-off frame's machine pixels, and the pass only removes light;
it re-shoots three times and then fails. It caught two bad frames on the run
that produced the number above, one of them a "pass on" frame that came back
*brighter*, 112.1 to 155.2 mean luma.

A claim this work made along the way was wrong, and the correction belongs
here because it fed issue #103. Calibrating against the game needs a frame
the game's SSAO visibly changes, and the first-person pose looked like it was
not one: switching SSAO off moved 1.3% of that frame. But
`test_screenshot.gd`'s first-person camera stands 4 m above the focus and
pitches *up*, so whatever fixture is built it is aimed across an empty hall —
a frame with no near geometry is supposed to be unmoved by an occlusion pass.
Re-measured from a pose standing beside the line instead, the shipped
0.9/2.4 settings darken **9.80% of the frame by more than 1%, 2.84% by more
than 3%, p99 6.29%, darkest pixel 33.1%** (a re-render of the control differs
by 0.000%, so that is signal, not noise). The pass was never inert; the
camera was pointed at nothing.

Two seams came with it, both narrow. The viewer takes its switches from the
query string as well as its buttons (`?view=game&facing=2&ao=0&bare=1`), and
`serve.py` grows a `POST /api/shot` that exists **only** when `--shot-dir` is
passed on the command line — headless Chrome captures on a load heuristic
that lands before a model has been fetched, so the page hands back its own
canvas instead of being photographed.

### 2026-09-20 — Tooling: the build signs itself, the hook can read a path, the cursor belongs to the run

Three tooling defects off the deferred queue (#77, #53, #4). None of them is
in the game; all three are in the machinery the game is verified with, and
each one had already cost somebody a round of investigation.

**`cargo xtask` re-signs the library it stages (issue #77).** The sighting
behind the issue is a headless run dying with **exit 137, no stderr, no
marker**, while `godot --version` still exited 0 and `codesign --verify`
called the library valid — it read as a broken engine, and re-signing by hand
cured it. xtask is what replaces the file, so xtask now runs
`codesign -s - --force` on it and prints that it did, before the two
`--import` passes that are themselves the first thing to load it.

**Said plainly, because review asked the right question of it: that kill was
not reproduced to order.** Comment the call out, force a relink, run headless
Godot — and it passes, because the Rust linker already emits
`flags=0x20002(adhoc,linker-signed)`, so the staged copy is normally valid
before xtask touches it. Neither a `com.apple.quarantine` xattr nor a byte
flipped in a signed library brought the kill back either. What IS reproducible
is the shape of the failure when the signature really is gone:
`codesign --remove-signature` on the staged copy gives
`dlopen(...): Trying to load an unsigned library`, no GDExtension, a
`SimBridge` parse error out of `main.gd` — and **exit 0**. So the status code
is useless in both directions (137 with no output, or 0 with no marker), which
is the argument for keeping a call this cheap rather than for claiming it is
proven load-bearing.
Two calls settled in the issue: a signing failure **warns and leaves the
library staged** rather than failing the build (a machine with no Xcode
command line tools must still be able to build; CI's `cargo test` never loads
the dylib), and it applies **only to the loaded dylib, not the staged `.vox`
output** — the kill is about code the dynamic linker maps in, and signing
thousands of data files would cost seconds a build to change nothing.

**The PR-screenshot hook can open the body file it is given (issue #53).**
`.claude/hooks/require-pr-screenshot.sh` `open()`s any `--body-file` path in
the command, but it is not a shell: `--body-file $SCRATCH/body.md` was opened
as the literal characters `$SCRATCH`, failed, and counted as "no image" —
refusing a body whose first line was the screenshot. It now expands `$VARS`
over the environment the hook was actually given, plus `~`, and nothing else;
no subshell, so `--body "$(cat body.md)"` is still refused. Both halves are
tested: `$SCRATCH/body.md` and `~/body.md` pass, while a missing file, an
undefined variable, a command substitution and an expanded file with no image
in it are all still blocked (25 cases in
`.claude/hooks/test_require_pr_screenshot.py`).

**The `--hover` cursor drift, diagnosed (issue #4).** The cause was not the
tile→screen mapping, which is exact: it is that `Input.warp_mouse` moves the
machine's ONE pointer, which outlives the process and is shared with every
other window on the desktop. That leaks two ways, both reproduced. A run
asking for no hover inherited the tile the last run parked on — `--fixture
bottleneck --payout` came back wearing the previous `--hover-dock` run's
tooltip, over a frozen scene that had stopped refreshing the numbers in it,
which is the sighting in the issue's comment. And two `screenshot-scene.sh`
runs at once (a wave of parallel worktrees makes this ordinary) fight over
that pointer: run A aimed at the dock, run B warped to a hopper, and A's frame
would have carried the hopper's tooltip while claiming to be the dock shot.
So the script owns the cursor for the whole run now: every run **parks** it on
empty ground first (asking the scene's own `_hover_text_at_tile` where is
quiet), a `--hover*` flag then aims it, it is **re-asserted** three frames
before the shutter (which is also what makes `--closeup` safe, since that
re-aims the camera after the parking), and the frame is only written if the
cursor really is over what it was aimed at. A mismatch is `SCREENSHOT_FAIL`
and no PNG, the same refusal #70 gave a wrong fixture: a tooltip belonging to
the wrong thing is precisely the frame this script exists not to hand back.
Every run now prints `SCREENSHOT_CURSOR:` beside `SCREENSHOT_FIXTURE`.

The sweep that came with it found one more of the same kind, and it is fixed
too: a hover target the camera is **not showing** — `--fixture chain
--closeup 26,0,8 --hover-hopper` — could not be reached, because the window
server clamps a warp to the window, so the run failed at the shutter blaming
a hand on the mouse for a tile that was nine thousand pixels off the left
edge. The aim now checks the projection against the viewport first and says
exactly that. What the sweep did NOT find was a spurious refusal: 48
combinations — 41 that must render, 7 that must refuse — and every one did
what it should, before the merge with main and again after it. Across two sweeps one run each
refused at the shutter naming a tile nobody asked for — a different case
each time, both while sibling worktrees were running Godot, and both passing
six times in a row afterwards — which is the parallel-pointer case the
message already lists, measured rather than assumed, and now recorded in
`screenshot-scene.sh`'s own header.

Two corrections from review, both about a guard being confidently wrong —
which is the thing this ticket is against, so they are the ticket, not a
footnote. **A guard must check what is actually in the frame.** In first
person `main.gd` reads the tooltip from the SCREEN CENTRE, not the pointer,
so asserting on the pointer there failed `--first-person --hover-dock` at the
shutter and blamed a parallel worktree for a pointer nothing had touched. The
harness now asks the same question the frame does (`_hover_query_pos`), and a
`--hover*` flag in first person **turns the camera** instead of warping the
pointer — `--first-person --hover-dock` is "stand in the hall and look at the
dock", which is a review shot worth having. **And a guard must not invent a
cause.** The "nowhere on screen is empty ground" fallback printed a NOTE
explaining the tooltip and then refused anyway, with a message about a
pointer that had been moved when nothing had moved it; the explained frame is
now written, as its own docstring always promised, and every refusal lists
its candidate causes as candidates.

A third correction, from the re-review, and the largest of the three: **a
flag that cannot do what it was asked must refuse, not shrug.** Every one of
the seven `--hover*` helpers was guarded by `if tile != HOVER_NONE`, so a
flag whose target was not in the fixture aimed at nothing at all and the run
went on to photograph the parked cursor and report success. `--fixture
full_dock --hover-splitter` — that line has no splitter — handed back a clean
frame of the dock with `SCREENSHOT_OK` on it and exit 0, and plain `--hover`
did the same against the empty world, which is the shortest command the
script takes. Nothing in the output said the flag had been dropped. That is
issue #70's failure wearing another hat: a plausible frame of the wrong thing
at a zero exit status, handed to a reviewer as evidence.

Fixed for the SET rather than for the seven, because a hole this shape is
worth closing once. The flags are one table in `godot_project/hover_targets.gd`
— flag, what it looks for in words, and a finder over `world_layout_json` —
`test_screenshot.gd` has a single loop over it and no other way to aim a
hover flag, and a finder that comes back empty REFUSES with the flag, the
target and the world it looked in (with "you built no world at all" for the
empty case, rather than a confusing complaint about a missing dock). Three
things keep the next one honest: `test_hover_targets.gd` drives every row
against every fixture **headlessly**, which is new — the screenshot harness
needs a window, so until now nothing in CI knew the hover flags existed;
`scripts/check-invariants.sh` fails if `test_screenshot.gd` names a `--hover*`
flag the table does not have; and `--ghost-blocked` and `--remove-mode`, the
only other flags that aim at something the world may not contain, refuse on
the same grounds (an empty world made `--ghost-blocked` photograph the VALID
state).

### 2026-09-20 — Mothballing, and naming the machine that is going without

The two halves of one complaint about the staff pools: there was no way out
of the unpaid spiral (#19), and no way to tell which machine was the one
short of crew (#20).

**The spiral had no exit.** Unpaid heads idle the machines they crew, which
cuts the income that would have paid them, so a factory that missed one
payday missed the next one harder. The only lever was `Remove`, which
refunds the machine to the shed — a fine answer for a machine you regret
buying, a terrible one for a line you want back next period.

**Mothballing.** `set_mothballed` switches a building off: it drops out of
the establishment, produces nothing, and HOLDS the batch it was part-way
through rather than losing it or finishing it, so switching it back on
resumes. The switch lives on the click-to-configure panel, which grew a
third shape — not a keybind, for the same reason Q, SPACE and V were
retired. It persists across a save, joining `schema_version` 2 alongside a
tank's declared fluid rather than claiming a version of its own.

**Standing crew down is what makes it worth anything.** Review caught the
first attempt doing nothing: it shrank `required` but never touched
`bill_per_period` or `next_bill`, so an A/B over four paydays came back
bit-identical. It now stands crew down — a mothballed building's heads come
off the payroll, the bill falls on the tick the switch is thrown, nobody is
fired and nobody walks out. Mothballing on payday eve buys notice pay, not
free labour. The four-payday A/B that used to tie now ends 44 against 86.

**Anything that asks for a crew can be switched off**, decided by the
kind's `staff` list in `data/machines.ron` rather than by a Rust type — so
the **dock** can be, and has to be able to be: while it is on,
`required("logistics")` never reaches zero and a factory with no income
cannot get its bill down to nothing. A hopper, a tank, a belt, a pipe and a
splitter ask for nobody and are refused. The same rule now drives the
panel, which asks the content instead of consulting a kind list kept in
GDScript.

What mothballing does not do is conjure income: a standing order lost to
three missed windows while the factory was dark costs money to re-sign.
The acceptance test says so out loud rather than claiming a recovery it did
not see. That is issue #85, still open.

**Naming the machine.** Every placement now reports `crewed` — the
establishment's stable answer to *if everything placed wanted to run at
once, who goes without?* Deliberately not a replay of the last tick, which
flickers as the line ebbs; that flapping is what the establishment model
existed to end. The hover says it in the same words a `staff` stall gives,
through the one helper both go through so they cannot drift.

The load bar needed one new idea. An uncrewed machine does no work, so its
window decays to 0% and there is no fill left to turn amber at exactly the
moment the player needs telling. So the track's **border** carries the
colour instead: the fill is the quantity, the border is the state, and the
words above it still say which state it is. A mothballed machine beats both
and reads `○ switched off` on an unlit grey bar — the player chose it, and
nothing about it is a fault. No in-world pole; the hover panel is still the
one place status lives.

The dock's own held line still flickers the old way, for a channel reason —
issue #87.


### 2026-09-20 — Three issues off the board: names, fluid colour, honest frames

A no-new-features pass over the deferred queue. Nothing here changes what
the game does; all three were cases of the presentation layer saying
something untrue.

**Content names itself (issue #60).** The HUD titled an item by title-casing
its id, which is right for every id the game shipped with and wrong the
moment content contained an acronym: the second sauce read as "Boxed Bbq" on
the contract board, in the production panel and in every tooltip. A recipe or
an ingredient entry may now carry an optional `display_name` in `data/*.ron`;
`World::display_names` gathers both files' into one map, `display_names_json`
carries it, and `main.gd`'s `_kind_display_name` consults it before falling
back to title-casing. The fallback is still the rule — only the four BBQ ids
name themselves, and a test asserts that every id title-casing gets right
stays *out* of the map, so no label has a second copy to keep in step. A
blank name is refused at load beside the other content errors. `schema_version`
is unchanged: an optional field leaves old data valid. Both data files enable
RON's `implicit_some`, so a content file reads like content
(`display_name: "BBQ"`, not `Some("BBQ")`).

**The BBQ branch's three fluids each have a colour (issue #68).** `bbq_mix`,
`bbq` and `liquid_smoke` had no `FLUID_TINTS` entry, so all three drew in the
neutral fallback — three runs of pale blue on a factory whose whole point is
that it makes a second, browner sauce. The tints are picked against
`palette.ron`'s `bbq_brown` and lifted for the shadowed bore, and
`test_bbq_branch.gd` now fails if a run the fixture names has no tint, or if
two tints land within a summed-RGB distance of each other. `bbq` and
`bbq_mix` sit at zero pressure in every existing fixture and so could not be
photographed — stated in the PR rather than glossed, and filed as #72.

**The screenshot harness stopped swapping fixtures behind your back (issue
#70).** `--fixture NAME` was parsed first and then overwritten unconditionally
by any of sixteen convenience flags, so `--fixture understaffed --staff`
photographed the *reference* line and reported a fully-crewed factory, with
nothing on stdout and a zero exit status. An explicit `--fixture` wins now
and the override is announced; only the fixture is overridden, so the panel
the flag opens and the ticks it runs still apply. An unknown `--fixture` name
is refused with the list of known names instead of falling through to the
demo line and looking plausible, and every run prints
`SCREENSHOT_FIXTURE: <name>` beside `SCREENSHOT_OK`. This one matters beyond
tooling: a rendered frame is the evidence every PR is reviewed on.

Also closed **#58** — `cargo run -p voxgen` was fixed on `main` by #64
(`default-run = "voxgen"` plus `--bin voxgen` spelled out in every doc), and
the issue had simply outlived it.

### 2026-09-18 — Two HUD panels that ran out of room

**The staff panel shows all four categories at 1600x900 again (issue #7).**
It was showing about two and a half: a row was seven stacked elements tall
(147px), the scroll caps at 400px there, and the fourth category was below
the cut — in the one panel whose job is comparing categories. The fix is the
row, not the panel: the bar's caption moved onto the bar's own line
(`hired / asked for`, four words that were costing four lines), the per-tick
`2 of 4 working right now` moved into the slot after the bar and shows only
when there is no shortfall to put there, and the five sentences a row can
say — unpaid heads, why HIRE is disabled, what hiring would unblock, what
asks for the category, what the category does — became **one** slot that
shows the worst of them and hands the rest to the row's hover
(`_staff_row_sentence`, `STAFF_SENTENCE_ORDER`). 147px → 85px, four rows in
367px of a 400px cap, in every state including the underfunded fixture. No
panel grew and no type shrank. `✗ cannot hire —` also lost its long form for
the one UI_UX.md has documented since the staffing ticket (`8 in hand, 14
due, 2 / period`).

**A clipboard cut short now stops on a line (issue #17, partly).** When the
column has no room for a whole docket, `_fit_grid_scroll` keeps whole *lines*
of the one it is cutting instead of stopping wherever the clamp fell
(`_line_boundary_height` / `_row_line_boundary`). UI_UX.md §5's ordering is
untouched — never overlap, then cut on a boundary, then show as much as fits;
the clamp still wins and the cut still only moves down. At 1600x900 the cut
went from 102px (through a checklist row) to 81px; with the SHORT-STAFFED
strip up, 146px. **Not fixed:** 1280x720 with the strip down and a full
board, where the column leaves the clipboard 12px and the first line boundary
inside a docket is 43px. Snapping there shows no docket at all, which loses
the "there is more below" cue as well as the docket, so that one case keeps
its ragged edge and #17 stays open with the numbers on it. The answer there
is a shorter docket — a content decision, not a layout one.
`test_hud_plates.gd` kept its assertion about what set the cut and gained the
line-boundary one for the clamped case it used to skip; `test_staff_panel.gd`
gained the four-rows-fit-at-1600x900 proof and a one-sentence-per-row proof.

### 2026-09-18 — BBQ sauce exists, and the board asks for both

The second sauce is content. `data/recipes.ron` grew four entries, all on
machine kinds the player already owns — ketchup + spice mix + molasses into
`bbq_mix` in a **mixer**, `bbq_mix` + liquid smoke into `bbq` in a **cooking
vat**, then the same bottler and packer as before. No new machine, no new
Rust behaviour: every capability this needed landed with the previous ticket.
A BBQ line is a *branch*, tapped off the pipe run leaving the ketchup vat, so
the two sauces compete for one vat's output and a player's first real
decision is how to split it.

**The cook time is a ratio.** 600 ticks in the BBQ vat against the ketchup
vat's 240 means the branch draws one ketchup per 600 ticks and leaves 2.5 per
1000 for bottling — exactly 3 boxed ketchup for every 2 boxed BBQ, which is
exactly what `grill_pub`, the first mixed order, asks for. Measured on a
running factory over 24 000 ticks, not just arithmetic:
`the_two_sauces_split_one_vat_three_to_two` in `sim_core/tests/bbq_branch.rs`
fails loudly if either number moves without the other.

**Two of the four new contracts name both sauces**, which is the first time
GAMEPLAY.md §6's central claim — a mix is all-or-nothing, so diversify — has
ever been in front of a player. It was implemented and tested from the start
and had nothing to point at.

**Nothing in `sim_core/src/` changed behaviour.** What did change is the test
fixture. `sim::reference_chain` walked *every* recipe in the file and laid
them out in a line, which quietly worked only because the file was a line;
the moment it forked, the fixture parked the BBQ mixer's hopper on the
ketchup packer and took about a hundred and forty tests down. It now walks
`linear_prefix` — the recipes that actually chain, one after another — and
the branch gets a fixture of its own, `sim::bbq_branch`. Both are pinned by
an exact placement list (`reference.rs`'s own `mod tests`), so the next
content change cannot shove the shipped layout sideways in silence.

`data/recipes.ron`'s header described rules that stopped being true last
ticket (issue #44) and is rewritten: it now carries the table of which
recipes share a kind, and `recipes_on_a_kind_take_disjoint_inputs` pins the
property that makes file order a tiebreak that never actually breaks a tie.

Two things noticed and deferred: a pipe run that stays ambiguous stalls its
machines with nothing said to the player (#54), and bolting this branch onto
a single ketchup line starves the starter contract for the few thousand ticks
the new pipes take to prime (#55).

`data/unlocks.ron` is untouched — BBQ ships ungated and the next ticket gates
it.

### 2026-09-18 — A case's print moves to its lid, and the two sauces separate

Issue #40: `boxed_ketchup` and `boxed_bbq` were tellable apart closeup and
not at the framing a player actually plays at. Measured rather than argued —
on the item-gallery frame at default zoom, where a case is about 30px, the
two cases' mean brightness was **1.2/255 apart**, i.e. nothing. The two
label patches were 5x6px each and on the camera's shadow-side face.

The fix is the same print on the **lid**, one panel either side of the tape
seam, in each sauce's own label colour — no new palette entry, no new voxel,
no change to either case's silhouette, since every voxel it paints was
already carton. The lid is the face this camera lights and sees most of, so
the same two colours now measure **26.5/255 apart** on the same frame, about
twenty times the separation. **The carton is untouched** (the project owner's
ruling): kraft margins all round the panels and the tape seam still crossing
between them. Print on the two short faces as well measured better still
(43.2/255) and was rejected — at closeup the kraft stopped being the thing
you see, so the case read as coloured *packaging* rather than a printed box.

The lesson, which is why the numbers are here: at this camera a mark on the
lit top face is worth several of the same mark on a side face. `item_box`
changes too, deliberately — the pair has to contrast, and both sauces' cases
gain a positive identity instead of one being "the marked one". The
distinction is value, not hue, so it survives colour blindness. Followed on
as #57 (re-check on a real mixed line once BBQ is in `data/`) and #58 (the
stale `cargo run -p voxgen` invocation in docs and the CI hint).

### 2026-09-18 — The PR-screenshot hook looks at the pull request now

`.claude/hooks/require-pr-screenshot.sh` decided whether a command needed a
screenshot by reading the command's own text and nothing else, which misfired
both ways (issue #50, both halves hit for real while landing PR #45).

A `gh pr edit <n> --add-label full-verify` was refused: it carries no body, so
it carries no image, so the hook blocked it — even though the PR's body already
embedded one. That is the worst possible case to block, because `full-verify` is
the label that switches on the Godot gate and the changes that deserve the gate
are the ones you label *after* opening the PR; the workaround it taught was
`gh api .../labels`, which routes around the guard entirely. An edit with no
body-bearing flag now asks GitHub for the body that is already there. If that
lookup fails — offline, not logged in, an unparseable selector — the hook
**fails open**, deliberately: such an edit cannot remove an image, and CI's
`body-embeds-an-image` check still sees the PR. `gh pr create`, and any edit
that rewrites the body, stay fail-closed.

The other half: writing *about* the rule tripped the rule. The invocation
scan split the raw command on `;`/`&&`/newline, so `gh pr edit` quoted inside
a `--body` counted as a command — filing issue #50 was itself blocked, from a
plain `gh issue create`. The scan tokenises with `shlex` now, so a quoted
argument is one word and `gh` has to be in command position; newline stays a
separator, so two commands on two lines are still two commands.

The hook has a test for the first time (`.claude/hooks/test_require_pr_screenshot.py`,
19 cases, `gh` stubbed so it needs no network), run by `invariants.yml`. Half
the cases are the bug and half are the rule — a guard that stopped guarding
would be a worse outcome than the bug.

### 2026-09-18 — A browser for the voxel assets

`scripts/asset-browser.sh` (`tools/asset-browser/`) opens every asset in
`assets/voxels/` as a searchable list beside a 3D viewer you can turn. It
exists because reviewing an asset had two speeds and nothing in between: a
single isometric preview PNG per model, or the whole factory through
`screenshot-scene.sh`. Neither answers "what does this thing look like from
the other side, what variations does it have, and what is it made of".

Everything on screen is read live from files already in the repo — the
`.vox` models themselves, `index.json`, `materials.json`, `palette.ron`, the
`///` doc comment on the generator function, `main.gd`'s model tables and
`data/machines.ron`'s footprints — so the browser cannot drift from the
pipeline, and a `cargo run -p voxgen` shows up **without a reload**: the page
polls a stamp of those files' mtimes every 1.5s and rebuilds what changed,
toasting the assets that are new. The 79 models fold
into 44 assets by the naming the pipeline already uses (`_f0..f3` frames,
`_motion` overlays cut out of a body, `_fill`), with motion drawn over its
host and `play` cycling the frames; `pipe_fluid*` and `splitter_arrow` are
offered on top of the host they are meaningless without.

It is Python 3 stdlib plus three.js fetched once into a gitignored
`web/vendor/`: no build step, no npm, nothing new for CI to run.

The viewer is lit like the game, which closes **issue #37**: the two
directional lights of `factory_environment.gd` at their Godot angles and
energies, the factory-interior sky as an environment map, ACES tonemapping and
a bloom pass in place of Godot's glow, so a status lamp glows in its own colour
here as it does on the factory floor. Godot splits one thing three.js does not:
`ambient_light_energy` scales sky *irradiance* only, while
`REFLECTION_SOURCE_SKY` reflections arrive at full sky radiance — three.js has
a single `environmentIntensity` for both. The split is restored as
`environmentIntensity` (answering to the metal) plus a `HemisphereLight` (the
diffuse ambient, which reflects in nothing), and both numbers were set by
measurement rather than by eye: render the frame, re-render with a
`MeshNormalMaterial` override, classify each pixel by the world direction its
face points, and compare luma percentiles per material class against pixels
sampled from `screenshot-scene.sh --fixture vat`. Stainless lands at #d4d4d3
on top against the game's #d2d3d4, #666c6f on the camera-facing flank against
#646969, painted blue #153d57 against #173b58.

That test is what found the real bug. Every model was being drawn from the
*inside*: `buildClassMesh` emitted its two triangles in the order the corner
lists read, which is clockwise seen from outside, i.e. back-facing — so with
`side: FrontSide` three.js culled the near surface of every model and shaded
the far one. It looked like a lighting problem (washed out, no contrast, both
directional lights apparently inert) and was a winding problem; a vat read as
a hollow white shell. Worth remembering as a class of bug: when *every* light
seems to do nothing, check which way the faces point before touching a single
light.

There is still no ambient occlusion, which Godot has on — three.js's
`SSAOPass` assumes a perspective camera and this viewer is orthographic
(**issue #49**). So the browser supplements the render checks in UI_UX.md §2a
point 3 and does not replace them.
### 2026-09-17 — the sim can express a branching chain

The three sim costs the BBQ plan named are paid, against a test fixture
rather than against `data/recipes.ron` — the shipped file keeps its five
entries and its behaviour is byte-identical, because the sauce itself is the
next ticket.

**A machine kind may run several recipes.** `RecipeRegistry` keeps a
per-kind list in file order; a placed machine runs whichever of its kind's
recipes its *current inputs* satisfy, re-picked on each idle tick, with file
order as the tiebreak when more than one fits. There is deliberately no
player-facing picker: no command, no UI, no save field for a choice. Two
cooking vats on one line, same kind and same facing, now run different
recipes because they are plumbed into different things.

**A recipe may draw several fluids.** The one-fluid-input cap is gone and
each input is matched to a touching pipe run **by fluid id** at tick time,
not by position; fluid *outputs* stay capped at one, off the outlet face.
The N draws plus one deposit are borrowed as disjoint mutable slices.

**`validate_chain` reads a graph.** It floods from the first recipe and
panics on any recipe sharing no ingredient id with the rest, so a fork is
legal and a structural break still fails at construction. A typo'd id that
merely invents an unpriced raw is still caught by
`IngredientCatalog::validate_covers` instead; `validate_chain`'s doc records
which net catches which. That rule expires the day a sauce has an
independent raw chain — mayonnaise, GAMEPLAY.md §2 tier 4 — which is issue
#42.

**The subtle part was naming pipe runs.** A network's fluid used to come
from the single recipe its machine named, which multi-recipe kinds destroy.
It is now derived by **shrinking possibility sets**: a run seeds to what the
machine whose outlet faces it could make, or failing that to what the
machines drawing on it could want; a run with a tank on it is intersected
with the raw fluids, because a tank is the only way in from outside and only
raws come in; then every machine whose feasible recipes *all* use a run
narrows that run to what they name it, repeated until nothing changes. One
possibility left means the run carries it; none or several means the run
stays unnamed and its machines stall, never a panic. Shrinkage is monotone,
so it terminates, and a sweep is computed from the previous sweep's state
and applied at once, so the answer does not depend on the order machines are
walked — `pipe_naming_is_independent_of_placement_order` builds one layout
forwards and backwards and asserts identical naming. `world.rs`'s module doc
carries the full explanation. The case it cannot resolve — two *tank-fed*
runs feeding one machine that draws two raw fluids — is issue #43.

**Save format.** Which recipe a machine is on is live state now, so
`SavedMachine` carries it: a `#[serde(default)] recipe: Option<RecipeId>`
added **without** bumping `SAVE_SCHEMA_VERSION`, on the standing rule that
an additive optional field is not a shape change. A save taken mid-craft
comes back on the same craft; a save written before the field loads on the
kind's first recipe. An unknown recipe id for a kind is a `SaveError`.

This supersedes DECISIONS.md decision (7), "a placed machine runs the one
recipe its kind names" — the *no player-facing picker* half of that decision
stands and is in fact what made this possible; only the "one" is gone.
`data/recipes.ron`'s header still describes the old rules and is issue #44,
left alone because the next ticket of this epic owns that file.

### 2026-09-17 — The fmt backlog is cleared

`cargo fmt --all` landed as one commit (#28): 32 of the repo's 46 Rust files
were unformatted, purely accumulated drift, no behavior change. Chosen to
land at a quiet moment (no other Rust-touching branch open) since a
repo-wide reformat conflicts with anything in flight. `FMT_BASELINE` in
`scripts/check-rust-lint.sh` drops to 0 — the ratchet mechanism stays, but at
zero it now behaves like an ordinary `cargo fmt --all --check` gate.
`CLIPPY_BASELINE` (issue #29) is untouched; clippy's backlog is a separate,
judgment-heavy burn-down.

### 2026-09-16 — The repo is `condimental` too

The name change reached GitHub: `TomWhitticase/sauce-factory` →
[`TomWhitticase/condimental`](https://github.com/TomWhitticase/condimental).
Every current-tense link was swept with it — the clone instructions and issues
link in README.md, the two `ISSUE_TEMPLATE/config.yml` URLs, `claude.yml`'s
line about which repo the agent is in, and the **Repo:** line in the status
section above.

Nothing was ever pointing at a dead URL, because GitHub redirects a renamed
repo's old paths indefinitely. That is also why the old links are left alone
where they are history: the `#1`–`#4` issue links in the log below, the ones
in `PIPES_AND_CONVEYORS_PLAN.md`, and — the ones that would have been genuinely
annoying to fix — the `pr-media` screenshot URLs baked into every past PR body.
They all still resolve.

The tooling needed no edit: `pr-screenshot.sh` and `check-pr-screenshot.sh`
both ask `gh repo view --json nameWithOwner` rather than spelling the repo out,
so they picked up the new name on their own. Worth keeping that way.

**The local directories are deliberately NOT renamed.** `orca/projects/sauce-factory`
and `orca/workspaces/sauce-factory/*` stay as they are. A worktree's `.git` is
an absolute path into the parent checkout's `.git/worktrees/`, and there were
three live worktrees with sessions running in them at the time; renaming the
directory breaks all of them and git does not care what a checkout is called.

### 2026-09-16 — The game is called CONDIMENTAL

`sauce-factory` was always a working title. The game's name is **CONDIMENTAL**,
styled all-caps — it is what the title screen says, what `project.godot` calls
the application, and what the design docs are titled.

The rename stops at the name. "Sauce" is still the theme and still the noun in
every recipe, ingredient and plan; only the title changed. Three things keep
the old name deliberately:

- **The repo and the worktree paths** are still `sauce-factory`, so every
  `github.com/TomWhitticase/sauce-factory/...` link in the docs is left exactly
  as it is. Rewriting them before the repo is renamed would 404 them; after a
  rename GitHub redirects the old paths, so there is no hurry either way.
  Renaming the GitHub repo is the owner's to do.
- **This log and DECISIONS.md** are dated history. Past entries say
  "Sauce Factory" because that is what it was called at the time.
- **The archived `*_PLAN.md` files**, for the same reason — including the
  worker prompts quoted verbatim inside them.

One consequence worth knowing: `user://` resolves through `config/name`, so
saves now live in `app_userdata/CONDIMENTAL/` rather than
`app_userdata/Sauce Factory/`. Any pre-existing local save is not migrated —
it is simply in the old directory, and the game will not list it. That is
acceptable at pre-alpha, where the schema is not migrated either.

The gdext entry struct is `CondimentalExtension` now; the crate is still
`sim_core` and `entry_symbol` is still gdext's default `gdext_rust_init`, so
no library filename or binding moved. Android/iOS ids went from
`com.saucefactory.spike` to `com.condimental.spike`, which is free because
nothing has ever been published under either.

### 2026-09-16 — BBQ sauce has a bottle and a case

The second sauce's finished goods, drawn ahead of the content that makes them
(`docs/BBQ_SAUCE_PLAN.md`'s ticket 4; `bottled_bbq` and `boxed_bbq` do not
exist in `data/` yet and will not until ticket 2 lands). `item_bottle` and
`item_box` are now thin callers of a shared `bottle`/`box_of_sauce`
parameterised by livery, with `item_bottle_bbq` and `item_box_bbq` beside
them and both ids mapped in `main.gd`'s `ITEM_MODELS`.

The distinction is carried by VALUE, not hue, in both directions: a bottle of
BBQ is a deep molasses `bbq_brown` where ketchup is vivid `ketchup_red`, and
a case of BBQ has DARK labels where ketchup's are near-white cream. At the
size an item renders on a belt — half a tile — two warm colours a shade apart
are not a difference a player can act on; reversing the contrast direction
survives any zoom. The carton itself does not change, per the project owner:
a case is a cardboard box whichever sauce is in it, and only the print on it
says which.

Two things fell out of rendering it. The model previews could not review a
case at all — they are shot from vengi's camera, the opposite corner to the
game's (UI_UX.md §2b point 35), which is the face the labels are not on — so
`scripts/screenshot-scene.sh --item-gallery` now stands one of each
finished-goods model on a bare belt run, through the scene's own item
drawing. And for the same reason **`boxed_ketchup`'s icon has never shown its
label**: a case is labelled on both long faces now, which changes `item_box`
on a face the isometric camera cannot see, and is what stops the contract
clipboard showing the same blank carton for two different items.

### 2026-09-16 — BBQ sauce, the second sauce, is planned

Every one of the nine contracts on the board asks for `boxed_ketchup`, so
GAMEPLAY.md §6's central claim — that contracts demand a mix and therefore
force a diversified factory — has never once been exercised in play. The
second sauce exists to turn that pillar on rather than to add content.

BBQ won it because it needs no mechanic §11 still lists as undecided (mayo
needs spoilage, hot sauce needs corrosion), because it fits on the machines
already shipped, and because it consumes **finished ketchup** — making the
sauce list a branching dependency tree from the second entry onward. Its
mixer taps the same ketchup pipe the bottler drinks from, so every box of
BBQ is a box of ketchup that was not shipped.

The plan is `docs/BBQ_SAUCE_PLAN.md`, with the numbers five worker tickets
depend on. Three of them cost something in the sim: a machine kind may run
several recipes (chosen by what its inputs satisfy, not by a player-facing
picker), a recipe may draw more than one fluid network, and `validate_chain`
becomes a graph check rather than a line check. Nothing is built yet.

### 2026-09-16 — the title screen loses its scrim

The main menu laid a dark gradient band across the middle of the viewport
behind its text — 82% black at full strength, fading to clear at both edges —
on the argument that text over a photograph needs a backing. The owner did not
want it, and the rendered frame backs that up: it dimmed the skylight ribbons
and the far plant line that the same day's `TITLE_HAZE_DENSITY = 0.0` decision
had gone out of its way to make crisp, and its soft edges read as a shadow
lying on the picture rather than as anything standing in the hall.

It is gone, and the contrast it was buying is bought by the text itself: the
wordmark's outline goes 9px → 14px (`TITLE_OUTLINE`) and the tagline and
splash line 5px → 8px (`STRAP_OUTLINE`), both named constants now rather than
numbers at the call site, because they are load-bearing. The button plate was
never relying on the scrim — it is α 0.94 in its own right. `test_main_menu.gd`
now refuses any full-rect overlay child of the menu, since restoring the scrim
would otherwise leave every check green.

### 2026-09-16 — The factory emits light

The follow-up to the entry below, and the close of **issue #25**. `emissive`
is now the one material class Godot builds as a `ShaderMaterial`
(`godot_project/voxel_emissive.gdshader`): `ALBEDO = lens * albedo_scale`,
`EMISSION = lens * energy`, where `lens` is the mesh's own vertex colour
converted sRGB → linear. Everything else stays `StandardMaterial3D`, which
matte, metal and liquid need to be — `main.gd` duplicates and re-tints those
for hopper fills and per-fluid pipe bodies.

`StandardMaterial3D` could not do this. Its emission comes from a flat colour
and an emission texture, never from vertex colours, and `EMISSION_OP_ADD` —
the only setting that lights anything at all — lights every lens the same
white, which is the failure the original MULTIPLY was reaching for. A single
`emissive` submesh routinely carries two lens colours: `control_panel` puts
`status_green` at one end and `status_amber` at the other.

`palette.ron`'s `emission_energy` went **2.6 → 1.2** in the same pass. 2.6 was
never a judgement — it was picked while the value could not reach the screen,
so every value rendered the same frame. Judged for the first time against the
mixer's amber lamp, the vat's ring and the dock's deck lamps under the glow
settings that were themselves tuned while nothing emitted, 2.6 burns a lit
#FFB020 lens through to (255, 245, 178), a cream white — the hue-washing the
shader exists to prevent, reached from the other side. At 1.2 a lens reads
unmistakably lit against the hazard chevrons beside it and is still amber.

The OFF lamp keeps its dark **albedo** as well as zero energy: that is what
separates the two halves of the dock's full-pool blink in a frame where the
glow pass is what the eye reads, and it is the half a headless test can hold.
`test_presentation.gd` asserts both, plus that the `emissive` material is the
shader and no other class is.

### 2026-09-16 — CI grew up: the checks a session skips now run anyway

The repository had two workflows — commit-message linting and the
PR-screenshot backstop — and neither of them ran a single test. Everything
that actually proves this project works was, until today, only ever run by
whoever happened to remember to run it. That is a bad bet in a repo built
mostly by agents in separate worktrees, and it had already been taken: the
workspace has 32 unformatted files and 28 clippy warnings, and nothing had
ever said so.

Seven workflows now. `rust.yml` runs `cargo test -p sim_core`, `cargo test -p
voxgen` and `cargo xtask` on every pull request, plus a regenerate-and-diff of
`assets/voxels/models` — voxgen's output turns out to be byte-reproducible, so
an edited generator with a stale `.vox` beside it is now caught instead of
quietly rendering the old model in the screenshot the PR is judged on.
`invariants.yml` checks the rules that fail silently: a `test_*.gd` nobody
wired into `verify-core-loop.sh`, a marker the runner greps for that the
script never prints, a `res://` path into a deleted file, a doc link into a
renamed one, plus `shellcheck` and `actionlint`. `verify.yml` runs the full
Godot gate nightly, on demand, or on a PR labelled `full-verify` — not on
every push, because it costs minutes and a 100MB download. `audit.yml` is
`cargo audit`. `claude.yml` and `claude-review.yml` answer `@claude` and
post a first-pass review against CLAUDE.md's own rules; both no-op until an
API-key secret exists. Dependabot, a path labeller and CODEOWNERS fill in the
rest.

The fmt/clippy backlog is handled as a **ratchet** rather than a gate:
`scripts/check-rust-lint.sh` holds a high-water mark, so new code cannot add
an unformatted file or a warning, while the existing 32 and 28 are burned down
under issues #28 and #29. A gate that is red on day one is a gate that is
ignored by day three. The same reasoning kept the full Godot verify off the
PR path until it has been seen green on Linux — it has only ever run on the
macOS machines this project is developed on.
### 2026-09-16 — The README is the front door, with pictures

The root `README.md` had described a milestone-1 scaffold with "no gameplay
yet" for months. It is now a real front page: what the game is, eleven
rendered screenshots and the voxel icon set as artwork, prerequisites and a
build/run/verify walkthrough anyone can follow from a cold clone, a controls
table and an eight-step "your first factory" tutorial, the four architectural
rules, how the project is developed by agents, the roadmap and the release
position (there are none; build from source).

The screenshots are committed under `docs/media/`, downscaled and quantised
(2.7 MB for eleven), because the repository is private: a
`raw.githubusercontent.com` link renders as a broken image for a reader
without a token, which is the same constraint `scripts/pr-screenshot.sh`
documents for PR bodies. `docs/README.md` says so and says how to re-render
one. Nothing outside the three doc files changed.

### 2026-09-16 — Nothing in the factory emits light

The dock's "delivery pool is full" signal is a pulse on its deck lamps.
`--full-dock` (added the same day) made it possible to photograph either half
of that pulse on demand, and the two halves turned out to be **2/255 apart** —
a signal correct on every count a headless test could check, and invisible on
screen.

The cause is bigger than the dock. `voxel_assets.gd` built emission with
`EMISSION_OP_MULTIPLY` and a white emission colour, on the stated assumption
that MULTIPLY multiplies by albedo and so keeps each lamp's own hue. It
multiplies by the emission **texture**, and this pipeline has no textures at
all — colour is per-vertex, straight from `palette.ron`. The product is zero.
Raising the lit lamp from 2.6 to 50 renders a byte-identical PNG; switching to
ADD blows the same pixels to white immediately. **No `emissive`-class voxel in
the game has ever emitted** — not the status lamps, the vat ring, the payout
coin or the pilot lights. What has been shipping is ordinary painted amber,
green and red, and the glow settings in `factory_environment.gd` were tuned
with nothing feeding them.

Fixing that properly means emission that follows each voxel's own colour,
which `StandardMaterial3D` cannot express — it needs a small spatial shader
and a relight of every machine. That is its own review pass, so it is
**issue #25**, not a rider on a bug fix.

What did change: an off lamp is now told by **albedo**, the channel that does
reach the screen, taken down to a little over a third — which is where
`palette.ron` put its `status_*_off` trio back when the andon stack lights
let the colour be chosen outright. The dock's lamp reads (204, 154, 46) lit
against (66, 42, 16) dark, and the same swap serves every hopper, tank and
machine lamp. `test_presentation.gd` now asserts the two lamp materials differ
in **albedo specifically**, since an emission-only difference passes every
other assertion in that section and is still invisible.

### 2026-09-16 — Staffing holds still, and the crew is on screen

A playtest note said the staffing requirement "keeps fluctuating", and it
did. `demand` was a THROUGHPUT measure: a machine asks for a crew only when
it plans to work, so a belt running dry dropped the requirement and a burst
raised it. The `▲ SHORT-STAFFED` verdict was a momentary peak, and acting on
it meant over-hiring.

The requirement is now the **establishment**: the sum of the crews of every
PLACED machine, running or not, recomputed only when a machine is placed,
removed or rotated (and again after a load). `short` means "hire this many"
and holds still. The per-tick figures are kept and still shown, demoted to
"3 of 4 working right now" — a true and interesting number that was never
the hiring question. Per-tick claim gating is untouched, so the tick path
and its determinism guarantees did not move. No save-schema bump; `required`
is derived.

This contradicted GAMEPLAY §7 as written, so §7 is amended and the reasoning
is in DECISIONS.md. The owner agreed the change before any code was written.
Three things in the tree already assumed it: `staff.ron`'s starting roster
equals the shipped chain's placed crews (and a test asserts it), the HUD was
summing placements in GDScript to draw its "asked by" line, and §7's own
starting numbers only worked on the placement model.

The second half was a **CREW plate** on the right-hand column: who you have,
what they are committed to, whether you must hire, the next payday's cost
and whether you can cover it — all without a key pressed. Building it showed
the clipboard was already saying two of those things, so the clipboard is
now the ALERT (the wages warning, beside the balance it is about) and the
plate is the READOUT. The SHORT-STAFFED strip moved onto the plate's heading
and border. At 1280x720 that move was load-bearing: the strip's two lines
were the difference between the column fitting and the plates overlapping.

The column has three tenants now and the order of yielding is written down
and tested — clipboard scrolls its dockets, stock scrolls its rows, CREW
drops its detail line rather than growing a scrollbar, because an
at-a-glance readout you have to scroll is not one.

From a holistic review of the feature: `maintenance` is a dead category
(nothing asks for it) so it takes no room until something does; FIRE is
gated on heads and never on the establishment, or a player could be billed
for staff they cannot shed; and notice pay is now a tooltip on the FIRE
button, which is the one rule whose consequence arrives after the decision.

Two deferrals filed as issues: the unpaid spiral has no designed escape
hatch, and there is still no per-machine "who is unstaffed" at a glance.

The review shot earned its place again: with the panel open on an underfunded
line, the verdict read "nothing placed asks for staff yet" over four hired
heads and a `4 / 4` bar. It was summing `demand`, which a stalled factory
reports as zero. It sums `required` now, and the stalled case is a fixture.
The plate carries numbers and no bar: a 342px column has no room for a track
beside `2 / 4` and `▲ HIRE 2`, and `docs/DECISIONS.md` says so.

### 2026-09-16 — the hall in first person, its air in isometric; menu centred

Two notes back from the owner's demo of the title-screen branch.

**"When in isometric view the skybox is messed up (it looks great in first
person btw)."** It was, and for a reason worth writing down. The hall is a
*sky*: every surface of it is found by intersecting a box against `EYEDIR`,
which needs rays that fan out. An **orthographic camera's rays are
parallel**, so the entire screen sampled barely a degree of direction and two
or three roof bays were magnified into hard stripes fanning from the corners
— sliding sideways as the player panned, since a sky has no parallax to pin
it to the floor. Not a bug in the shader; a painted infinite room simply
cannot be photographed with a parallel projection.

So the hall is drawn where it reads and stands up to being looked at — first
person — and the isometric view gets the same hall's **air**:
`factory_environment.gd`'s new `set_backdrop_for` fills the background with
`HAZE_COLOR`, the very colour every distant surface in the shader fades to,
so the floor runs out into haze rather than into a starburst. Pressing TAB
then reveals the room that haze was standing in, which is the right way
round. **Only the background changes** — the sky stays the ambient and
reflection source in both modes, checked rather than assumed: the same
isometric frame rendered both ways came out byte-identical on the floor
tiles, the machine cluster and a belt's shadowed side.
`test_presentation.gd` now asserts the whole rule, because it is a one-line
property whose only symptom is a rendered frame.

**"Make the menu centred."** Done — the column and the plate are
`SIZE_SHRINK_CENTER`, the wordmark is centre-aligned and the scrim became a
symmetric four-stop gradient. That put the plate over the composition's
middle, so the composition moved: `title_scene.gd`'s camera was re-framed
(four framings rendered and rejected first) to put tomatoes riding into the
crusher in the left third and bottles coming out of the bottler in the right
— "sugar in, sauce out" spread across the frame with the menu in the gap.
This reverses the previous entry's "column set to one side of the vanishing
point", which is why it is argued in `docs/DECISIONS.md` rather than quietly
replaced.

**"Get rid of the haze in the splash screen."** Done, and it is the right
call: haze is atmosphere you read over a play session, but in one still frame
under a wordmark it is a grey wash lying on the picture. `build_into` takes a
`haze_density` now and the title screen passes `TITLE_HAZE_DENSITY = 0.0`, so
the roof trusses, the skylight ribbons, the glazing band and the far plant
line all come up crisp behind the chain. It is the one parameter that differs
between the game's hall and the menu's, and it is a property of the
photograph rather than of the lighting — measured in the frame, the models
moved by at most three levels.

### 2026-09-16 — a title screen: the factory behind the menu

The game opened on a flat charcoal plate. It now opens on a **title screen**
in the sense Minecraft and Factorio mean it: the factory itself, standing in
the hall, photographed. The shipped chain runs across the frame — hopper,
crusher, mixer, the 2x2 cooking vat, bottler, packer, dock, with tomatoes on
the belts going in and boxes coming out — a second line of plant stands off
in the haze behind it, and the camera drifts slowly along the line so the
screen reads as a place rather than as a paused game. Over it: a 114pt
hazard-yellow wordmark, the tagline, and a tilted yellow **splash line**
picked from a fixed list by the date.

**It is art, not a simulation.** None of `main.tscn` is loaded and no
`SimBridge` is created: `godot_project/title_scene.gd` is about twenty
hand-placed models on a floor, which costs a handful of `.vox` loads rather
than the game. That is what answers the argument the original flat plate was
chosen on. The parts it shares with the game were **extracted, not copied** —
`voxel_assets.gd` (the `.vox` loading, the per-class materials, the mesh
cache) and `factory_environment.gd` (the sky shader's uniforms, the
environment, the lights) both came out of `main.gd`, which now calls them,
and the floor helpers became `static` — so re-rendering the voxels or
re-tuning the lighting changes the title screen with the game instead of
leaving a stale poster behind.

The camera here is perspective, not the game's orthographic view: §1's rule
governs how the game is *played*, and the converging roof and the receding
line are the whole subject of a photograph. Its framing came out of a
camera-projection search after three rendered compositions were rejected.

The menu was rearranged around it — the wordmark out of the plate, the column
set to one side of the vanishing point, a gradient scrim holding the text
against whatever the drifting camera is pointing at. That turned up one real
regression, now UI_UX.md §2b point 41: the save list's flat 450px cap pushed
the BACK button off the bottom edge under the taller wordmark. The list is
now sized from the window less its measured chrome, and `test_main_menu.gd`
asserts both that budget and the backdrop itself (camera, hall, floor,
models, drift, splash).

Full reasoning in `docs/DECISIONS.md`; the rule is in `docs/DECISIONS_LOG.md`
and UI_UX.md §5d.

### 2026-09-16 — The backdrop is now the inside of a factory

The game used to stand on a concrete slab under an open blue sky. It now
stands in the middle of a factory hall far larger than anything the player
can build: a trussed roof with skylight ribbons and high-bay lamps overhead,
clad walls with a pipe-rack gantry, clerestory glazing and a line of other
plant along them, and air haze swallowing all of it towards the far end.
Press TAB and look around and the answer to "where am I" is the setting
UI_UX.md §3 describes, rather than a field.

It is one sky shader (`godot_project/factory_interior_sky.gdshader`), not
geometry and not a texture: a box intersected against the view direction,
with the roof trusses and the wall columns sharing one structural grid so
they line up. Being a sky, it has no parallax — the hall stays the same size
however far the player flies, which is exactly why it reads as massive
instead of as a room with walls to reach.

Two findings worth carrying forward. A near-parallel ray that returns a
*negative* distance overflows the haze term, and the resulting infinity in
the radiance map renders **every lit surface in the world black** — a
symptom that points nowhere near the sky. And an interior is a far darker,
far warmer light source than daylight: ambient went 0.8 -> 1.2 and the key
light lost the sun's warmth, because measured against the old frame the bare
concrete had gone from r-b = 5 to r-b = 21, i.e. every grey asset in the game
was photographing beige. Full record in DECISIONS.md.

Two things came back from the owner's playthrough. The hall's slab was set
to the same `concrete` the floor tiles use, which in the frame read about 40
levels darker than them — the tiles are lit and the backdrop is not — so it
is now a warmer, lighter `#979489` that measures within 4 levels of the game
floor at the seam, and the haze lifted with it. And walking the hall at 6
world units/second took most of a minute to cross it; first-person movement
is 15 now, about 4.7 tiles a second, which is a walk in a building this size
rather than a crawl.

### 2026-09-16 — Four playtest asks: a dock that fills up, pulp off the crusher, SHIFT to rebuy, and the first-person camera's first frame

Four unrelated things the owner asked for after playing, done as one pass.
Three of them turned out to be the same argument at different sizes: the
simulation owns the fact, the presentation owns how it reads.

**A dock can be full now, and the ceiling went on the pool, per item id.**
The dock itself stays the pass-through GAMEPLAY.md §9 designed — it empties
into the shared delivery pool every tick, so a cap on its own buffer would
be invisible. The cap is on the pool: `DELIVERY_STOCK_CAPACITY`, 48 units
of any *one* item. Per item, not one total, because a single total can
deadlock the game outright — one overproduced product crowds every other
item out of the pool, a contract whose mix names a crowded-out item can
then never settle again, and nothing in this game dumps excess stock. Per
item, every id always has its own room and a clog is always a specific
sentence rather than a mystery. The 48 is a *relationship*, not a literal,
pinned the way `STARTING_MONEY` is: `tests/dock.rs` asserts it comfortably
exceeds the board's largest single mix quantity (20, the export pallet), so
adding a bigger order fails a test instead of quietly making that contract
unsatisfiable. `Dock::try_absorb` asks the economy for room before taking a
box off the belt, which reuses the refusal path the dock's bounded buffer
already had, so the sim change is small. A save whose pool is over the cap
is now `SaveError::OverfullDeliveryPool` rather than a silent truncation.

Reading it is two things. The clipboard's ON THE DOCK line finally has a
denominator, so under UI_UX.md §5e's rule it earns a bar — `20/48` with a
meter that takes the docket's one warning red and `⚠ FULL — belt backing
up` at the cap. On the floor the obvious move, lighting the dock red, is
impossible here: `_make_material` gives emission `EMISSION_OP_MULTIPLY` over
vertex colour, so a red wash over a blue-grey deck is just a dimmer deck.
The full dock **pulses** its deck lamps instead, clocked off
`sim.current_tick()` like everything else in the scene, so a pause freezes
it and a screenshot repeats. A rhythm also survives every zoom this camera
has, and cannot be confused with the steady lamp that already means "has
stock". Worth keeping as a rule: **a world-space warning here is a rhythm,
not a colour.** And the full dock is not a penalty on top of anything — the
belt backing up behind it *is* the consequence.

**The crusher throws pulp.** UI_UX.md §6a had settled two sizes of feedback
for two frequencies — a payout's celebration, a dock arrival's small cue.
Crushing is neither: it is something a machine is *doing* for eighty ticks
at a stretch, so it is a beat rather than a burst. Five flecks of a new
`crush_bit` model out of the feed pocket every ten of the machine's own
*work* ticks, arcing about a fifth of a tile over the mouth and shrinking
out as they fall back in. Driving it off `_machine_work_elapsed` is the
decision rather than a detail: the spray inherits §2b point 28's narrow
meaning of "working", so a crusher starved of tomatoes or blocked on a full
output throws nothing, and the pulp says the same thing the mechanism and
the status lamp already say. No sound — §8 keeps "no idle machine loop"
deliberately, and a cue every ten ticks per crusher is exactly that loop.

Two things came out of the render rather than the plan, which is §2b point
12 again. The model took three passes: pale seed pips read as a stalk, i.e.
as a whole tomato, the opposite of crushed; a torn cuboid corner read as a
slab hanging off the lump; three overlapping spheres and one proud voxel
work. And the first spray — three flecks, 0.85 scale — read as three stray
red pixels on the machine's roof; five at varied sizes read as a spit;
thrown higher it stopped reading as something leaving the pocket and
started reading as debris flying off the floor. `scripts/screenshot-scene.sh
--crushing` is the review flag that made judging any of that possible: it
waits for the *state* (a crusher genuinely mid-batch with pulp in the air)
rather than a tick count, then freezes, because the flight is nine ticks
and the shot's warm-up is longer. Without it the obvious fixtures
photograph a crusher that is blocked or starved at that frame — correct
behaviour, useless picture. Its first run also caught the flecks being
invisible for the oldest reason in this repo: `crush_bit` had never been
staged with `cargo xtask` and imported twice, so the mesh silently did not
exist.

**SHIFT buys back what the click places**, so a run of belt no longer costs
a round trip to the build menu per handful. It is a modifier on *placing*,
not a second way to buy: the click still places a unit the player owns, and
the purchase happens after it and only if the placement actually landed, so
a blocked tile never charges. The prompt carries it in all three states
with the price on each, because a modifier nothing mentions is one nobody
finds and one that silently drains the balance is UI_UX.md §7's "never
encode state invisibly". Unaffordable warns *before* the click rather than
buzzing after it — nothing is refused, the placement stands, and the owned
count simply starts falling.

**Entering first person put the camera somewhere odd until you moved.**
`_set_view_mode` ended with an unconditional `_update_camera_transform()`
— the isometric placement — after a branch that had already placed the
camera for whichever mode was being entered, so first person kept the
isometric angle and distance until the first mouse-look or WASD input
called `_update_fp_camera_transform()` and snapped it into place. Each
branch now places the camera and nothing runs afterwards. The rule worth
keeping: **when two modes each own a transform, the switch must not have a
third opinion.** `test_camera_nav.gd` now asserts the camera itself is at
`_fp_position` on entry rather than that the mode flag flipped, which is
why the old test passed throughout.

`scripts/verify-core-loop.sh` is green.

### 2026-09-16 — Pull requests have to show the change

The owner's rule: a PR description always includes visual elements. It is
unconditional — not "for visual changes" — because that condition is decided
by the author of the change least likely to render anything, and the PNG is
the only check in this repo that a human has to look at (UI_UX.md §2b point
12). A sim-only change shows the state it affects instead: the panel whose
numbers moved, or the demo line at that commit.

Two things stood in the way and shaped the result. `gh` cannot attach a file
to a PR body, and the screenshots are gitignored, so there is nothing to link
to — `scripts/pr-screenshot.sh` now pushes a PNG to **`pr-media`**, an orphan
branch that shares no history with `main` and is never merged into it. And
the repo is private, which decides the link form: `raw.githubusercontent.com`
needs a token and renders broken for readers, while
`github.com/<owner>/<repo>/blob/pr-media/…?raw=true` is same-origin and
renders with the reader's own session. That was checked against
`gh api /markdown` rather than assumed.

Enforced in two places, because an unenforced process rule is a rule that
lasts until the first hurried session: `.claude/hooks/require-pr-screenshot.sh`
(a `PreToolUse` hook) blocks `gh pr create`/`gh pr edit` with no image in the
body — `--fill` alone therefore no longer opens a PR — and
`scripts/check-pr-screenshot.sh` runs in CI as the `PR screenshot` job,
failing a body with no image, a raw.githubusercontent link, or a link to a
file that does not exist.

One thing worth keeping from building it: the hook's first version matched the
substring `gh pr create` and promptly blocked the edit that was *documenting*
the rule, because the heredoc contained those words. It now strips heredoc
bodies before deciding whether the command really invokes `gh`, and searches
the whole command including heredocs when looking for the image.

### 2026-09-16 — Five things the owner asked for after playing it

Not a pass with a theme — a list of five sentences from the owner after a
session with the game, four of them about not being able to see something:
make it obvious when staff need hiring, let SHIFT+click buy as much as the
money allows, show what is in stock on the right-hand side, let the
instructions panel be closed, and "more UI for the staff stuff because it's
confusing to users". The first and the last are the same complaint at two
distances, which is why they were built together.

It was run as **three parallel work packages in three worktrees** — staff,
market, HUD layout — and the interesting part of that is how it was made
safe in a single 11,877-line file. Two rules did it. Each package owned a
*region* of the HUD rather than a feature list, so the diffs barely
overlapped; and **no package was allowed to touch a `.md` file**, because
three agents documenting their own work all write into the same
top-of-file decisions log and would conflict every time. Docs are the
orchestrator's job after the merge, which is what this entry and the
records beside it are. The only real conflict in three merges was in the
fixed-plate variable block, where the HUD package retired the larder strip
that the market package's new constant had been declared beside.

**No Rust changed, and that was checked first.** The quality pass had
already put `short`, `can_hire`, `next_bill` and `shortfall` on
`staff_snapshot_json`, and `buy_ingredient` has always taken an arbitrary
quantity — so buy-all is one command, not a loop of ten, and every staff
readout below is the simulation's own number formatted differently. Worth
recording as a partition rule: **check what the wire already carries before
splitting the work**, since a package that needs a sim change cannot run
beside the package that would consume it.

**Staffing is now said at three distances.** The old panel had every figure
right and still answered a question the player had to know to ask. So: a
**SHORT-STAFFED** strip on the always-visible clipboard, under the wage
line it belongs with (`▲ SHORT-STAFFED — hire 2 manufacturing · press H`),
naming only the categories actually short and hiding entirely when none
are — a readout that says `0 short` in the steady state trains the player
to stop looking at it. Then the panel opens with a one-sentence verdict
above its first row instead of four rows of meters. Then the short row
wears a `▲ SHORT` chip, a warn border and a line saying what hiring would
unblock. The buttons say what a click costs (`HIRE +1 · 2 / period`), and a
HIRE the wire refuses says *why* in words rather than going grey. Finally
the payday rules are written out in plain words — inside the scrolled list,
not under the footer, because `_fit_grid_scroll` caps against the panel's
height and four lines of chrome cost a whole category row (UI_UX.md §2b
point 32, for the third time in this project).

**SHIFT+click in the market buys everything the balance covers**, as a
single command — `cost_of` is all-or-nothing, so there is never a
partially-completed purchase to reason about. The half that matters for
whether anyone finds it: every slot prints what the gesture would fetch
right now (`shift: buy 123`) under the price, dimmed to `shift: buy 0` when
nothing is affordable. A slot is also now recessed only when *neither* verb
can act, where it used to go dead the moment a whole crate was out of
reach — which would have disabled the gesture in exactly the case it exists
for, a player who can afford four tomatoes but not ten.

**Stock got a plate.** It used to be a one-line larder strip inside the
hotbar's own container — one line tall by construction, a chip per
ingredient, and no room to grow. It is now a right-hand plate under the
clipboard with a row per priced raw ingredient (icon, display name,
right-aligned amount), and nothing on the hotbar shows stock any more,
since two readouts of one fact drift apart. The bug that came out of it is
the entry's best lesson. The right-hand column arbitrates in both
directions: the clipboard's scroll cap subtracts the plate's reserved
height, and the plate takes the band left under the clipboard. Rendering
the merged HUD at 1280x720 showed *one* of three ingredients — because the
plate reported the height it had **settled at**, the two caps chased each
other down, and the stable point was one row on a plate whose entire job is
showing every ingredient. **Two constraints that read each other must not
both read a settled value**; one has to state an intention. The plate now
reports what it wants, and the clipboard yields, because a docket stack
scrolls by design and this readout does not.

**The legend folds, and a fold is not a modal.** It collapses to a labelled
stub — its heading plus a button naming F1 — rather than vanishing, so what
is left still says how to get it back. Two ways in (the button, which never
takes focus so the number keys keep reaching the game, and F1, read beside
TAB so it works in both camera modes), deliberately outside the
one-modal-at-a-time rule: it neither closes an open panel nor is closed by
ESC. State is session-only; a fold is "not right now", not a preference.

Proofs: a new `test_hud_plates.gd` (`HUD_PLATES_OK`) in
`verify-core-loop.sh`, new sections in `test_staff_panel.gd` and
`test_chain_rework.gd`, and two new screenshot flags for the readouts that
cannot be photographed any other way — `--short-staffed` (the strip is
hidden by every panel that would otherwise be open) and
`--legend-collapsed` (the stub only exists once the legend is gone).
`scripts/verify-core-loop.sh` is green; reviewed at 1600x900 and 1280x720.
**Reported, not fixed:** the staff panel shows about 2.5 of its 4 rows at
1600x900, and a busy clipboard standing beside the SHORT-STAFFED strip
clips its last docket line. Both are the same capped-grid arithmetic, and
both leave "am I short?" answered by the strip and the summary line, which
is what was asked for.

### 2026-09-16 — GitHub is the workflow now: PRs in, issues out

The context restructure (below) fixed what an agent *reads*. This fixes what
an agent *does with finished work and with what it chooses not to do*.

Two things were going wrong, both invisible from inside a session. Changes
were reaching `main` by local merge and direct push — this very repo had a
commit fast-forwarded onto `main` an hour before this entry was written — so
there was no reviewable artifact, no CI run standing behind the merge, and
nothing to read afterwards except a `git log` subject. And findings that a
session consciously decided not to fix were being reported in its closing
message and nowhere else. GitHub had four years of stock labels, **zero
issues and zero pull requests** while the repo carried at least four known,
written-down deferrals.

**Nothing lands on `main` except by merging a PR.** `.githooks/pre-push`
refuses a direct push and prints the five commands that do it properly; unlike
the `pre-commit` DEVLOG reminder it blocks, because "am I pushing to main" is
mechanical, not a judgement call. Squash-merge is the default so the squashed
subject is the one Conventional Commit for the change and is linted like any
other. `.github/pull_request_template.md` asks for the thing reviews here
actually need: the verification commands **actually run and what they said**,
not "tests pass" — and, for a visual change, what the PNG showed in words,
since it is gitignored and cannot be attached from a headless run.

**A deferral is not finished until the issue exists.** `gh issue create
--label deferred`, with a body that assumes the reader starts cold: file *and
function*, how it was found (which test, which `screenshot-scene.sh` flag),
why it was deferred, and what the fix probably is. Seven labels, kept few:
`deferred`, `owner-decision` (blocked on a human — a gameplay change, a
design-doc deviation, anything invalidating saves) and five `area:` labels.

The four deferrals already written down in prose were filed as the first
issues, and `docs/PIPES_AND_CONVEYORS_PLAN.md`'s "Still open" section now
points at them so there are not two competing backlogs: the `resolve_sink`
inlet rule that contradicts both `world.rs`'s module doc and GAMEPLAY.md §9
([#1](https://github.com/TomWhitticase/sauce-factory/issues/1),
`owner-decision`, because it invalidates saves and the demo line), the belt
animation nobody has watched live
([#2](https://github.com/TomWhitticase/sauce-factory/issues/2)),
`_read_world_layout`'s ~40 ms of an 80 ms rebuild
([#3](https://github.com/TomWhitticase/sauce-factory/issues/3)), and the
`--hover` screenshot harness's cursor drift
([#4](https://github.com/TomWhitticase/sauce-factory/issues/4)).

Issues are the **backlog** and nothing else: a rule still goes through the
three-place decision ritual, a milestone still comes here, a plan is still a
doc. Two new skills, `open-pr` and `file-issue`, are the procedures; the full
rule is `docs/WORKFLOW.md`'s new GitHub section, with `CLAUDE.md` carrying
only the two lines that always apply. Not done, and the owner's call: branch
protection on `main`, which would enforce this server-side but would also stop
the owner's own direct pushes.


### 2026-09-16 — The agent context restructured: CLAUDE.md is a map now

Prompted by an outside argument rather than a bug: "Why Your AI Generates
Crap Code - And How to Fix It" (Web Dev Simplified). Its central point is
that `CLAUDE.md`/`AGENTS.md` is injected into **every** request, so its size
is a tax on every turn, and that an `@`-include is exactly equivalent to
pasting the included file's whole text inline.

By that measure this repo was the worst case the video describes. `CLAUDE.md`
was 631 lines and `@`-included ARCHITECTURE.md, UI_UX.md and GAMEPLAY.md, so
the real always-loaded payload was about **2,000 lines / ~34,500 tokens** —
several times the largest example the video was willing to call broken. Three
causes: a 429-line decisions log (a bullet per past problem, all of it loaded
on requests that never touch a machine or a contract), a 131-line **per-file**
project structure (the part of a layout that goes stale first, and a
confident lie in the always-loaded file is worse than no description), and
the three `@`-included design docs, of which most sessions need one at most.

What it is now:

- **`CLAUDE.md`: 72 lines, ~950 tokens** — a 97% cut, no `@`-includes. The
  project in a sentence, the rules that genuinely always apply, a
  directory-level layout, and a table from "what you are working on" to
  "which doc to read".
- **`docs/DECISIONS_LOG.md`** holds the condensed log, all 57 entries moved
  verbatim. Linked, read on demand.
- **`docs/WORKFLOW.md`** holds build/verify/commit/DEVLOG/Orca material.
- **Per-module `CLAUDE.md`** for `sim_core/`, `godot_project/` and
  `assets/voxels/` — ~100 lines each of that area's non-obvious invariants,
  loaded only when work happens there. Worst case inside a module is now
  ~2,400 tokens. ARCHITECTURE.md §10 had called for these "once warranted";
  this is what warranted looks like.
- **`AGENTS.md` is a symlink to `CLAUDE.md`** at the root and beside each
  module file. Not cosmetic: an untracked 160 KB `AGENTS.md` had been sitting
  at the root, and it turned out to be a flattened snapshot of `CLAUDE.md`
  with its `@`-includes expanded — a copy that had already drifted. A symlink
  cannot.
- **Four skills** under `.claude/skills/`: `voxel-asset`, `verify`,
  `screenshot-review`, `record-decision`. Only a skill's name and description
  sit in context; the body loads when it is invoked.

The rule, in `docs/DECISIONS_LOG.md` and ARCHITECTURE.md §10: put something
in an always-loaded file only if it would be wrong for the agent not to know
it on a turn that has nothing to do with it. Everything else is a link.

**Five documentation defects fell out of verifying the move**, all of them
drift that had been invisible precisely because nobody re-reads a 631-line
file:

1. **ARCHITECTURE.md §3.9 said the fixed timestep is 60 ticks/s. It never
   was.** `main.gd`'s `TICK_HZ` has been 30 since milestone 3 (`e7683c1`),
   and DECISIONS.md already called §3.9's figure "nominal" against "the rate
   the scene actually runs". A factual correction, not a new decision — but
   every seconds-figure derived from it was twice the real elapsed time, and
   two were: `hopper.rs`'s "roughly 21 seconds" (43) and a DEVLOG figure of
   "40 seconds" (80). §3.9 now names `TICK_HZ` as the single place the rate
   is set and records what it used to say. A dated 2026-09-11 entry keeps its
   original figure — it was accurate when written.
2. **"Nineteen headless Godot scripts"** in this file's Current status and in
   `scripts/verify-core-loop.sh`'s header. There are **thirty**.
3. **`scripts/screenshot-scene.sh`'s worked example was broken.** It read
   `screenshot-scene.sh out.png --build --config=…`, which silently loses
   `--build`: the script takes `$2` as the resolution and forwards only
   `"${@:3}"`, so the flag goes to `godot --resolution`. The example now
   passes the resolution and says why it has to.
4. **UI_UX.md §2b had three duplicate point numbers** — two extra 27s and an
   extra 28 — in a list whose own header says the numbers are cited by code
   comments and must stay stable. Citations had resorted to disambiguating in
   prose (`points 27 in "the floor" and 28`). The three entries took the
   next free numbers — **35** (a model whose only consumer is the icon pass
   is authored for vengi's camera), **36** (the floor is authored by its
   value range) and **37** (N models over thousands of tiles is still N
   stamps; per-instance rotation is the multiplier) — in place rather than
   renumbered into sequence, so every existing citation stays correct. 34
   was taken by ticket 7's ghost rule the day before. All seven
   citations and DECISIONS.md Part B's own copies were updated.
5. **`main.gd`'s `_panel_position_near` docstring** still described the
   viewport-edge-flipping behaviour the quality pass's ticket 5 replaced,
   sitting directly above the four-quadrant, plate-avoiding body that
   superseded it.

`scripts/verify-core-loop.sh` passes (19 Rust suites, 30 headless Godot
proofs). Nothing under `sim_core/src/`, `godot_project/*.tscn`, `data/` or
`assets/voxels/models/` changed behaviour — the only code touched was
comments and one docstring.
### 2026-09-15 — Pipes & conveyors overhaul closed out (ticket 7)

The last ticket of the overhaul: walk the two galleries shape by shape,
fix what the six build tickets left behind, and write the whole thing
down. The galleries themselves were clean — every belt shape at every
facing, every pipe shape at every turn, drawn from `links` as tickets 5
and 6 intended. What the walk actually found were four things around the
edges of the work, which is the more useful answer.

**The scene came up blank, and the code was fine.** Running the proofs
after a `cargo xtask` gave `Nonexistent function 'chunk_delta_json' in
base 'SimBridge'` — a stale library and unimported voxels, because
`xtask` stages models and deletes their `.import` sidecars, and Godot's
`.vox` importer is an editor plugin that only registers on the pass
after the first scan (UI_UX.md §2b point 4). So the models needed
`godot --headless --path godot_project --import` run **twice**, by hand,
by someone who already knew that. `xtask` now runs those two passes
itself at the end of `main()`, printing `godot --import pass 1/2 ok` as
it goes, and degrades to a printed reminder rather than a failure when
godot is not on PATH, so a CI box that only runs `cargo test` is
unaffected. `--no-import` opts out.

**The headless proofs' standing error line is gone.** `Parameter
"material" is null`, from `material_get_instance_shader_parameters`, had
been written off as scenery — `test_placement.gd`'s header told the next
reader not to chase it. It was real: freeing a `MultiMeshInstance3D` or
a `MeshInstance3D` that still holds a `material_override` makes the
headless dummy renderer complain. `main.gd`'s new
`_release_render_resources` walks a subtree and drops mesh and material
off both instance kinds just before it is freed, at every one of the six
places the scene frees something that could hold a mesh — including the
ghost body, a lone `MeshInstance3D` whose tint is a `material_override`,
which kept `test_pipe_shapes.gd` printing the line after the two subtree
frees were fixed. Worth recording the debugging dead end: the line is
emitted when the rendering server flushes, not synchronously at the
`free()`, so instrumenting the frees with marker prints points at
whatever ran next rather than at the culprit. Clearing on the way out of
both kinds is what actually silenced it, and the header comment now says
the opposite of what it used to — a `material` line there is a
regression now.

**The demo saves can be rebuilt.** The four saves used for demos since
2026-09-10 came from a throwaway script nobody kept, and a save is
rejected, not migrated, on a `schema_version` mismatch (ARCHITECTURE.md
§4) — so the next bump would have quietly ended them.
`scripts/make-demo-saves.sh` plus `godot_project/make_demo_saves.gd`
rebuild all four from `test_layouts.gd`'s own builders through real
Purchase/Place commands, warm each for 600 ticks, and prove each one
loads back before writing it. The shell script requires the script's
`DEMO_SAVES_OK` marker, since Godot exits 0 on a parse failure — which
is exactly how the first version's `Layouts.call(builder, ...)` was
caught (a static function cannot be dispatched by name off the script
class; it is a `match` now).

**The two ghost rules are one rule.** A belt ghost stays straight beside
a belt that will corner it; a pipe ghost takes its shape from the pipes
around it. That looked like an inconsistency ticket 7 was meant to
settle, and the settlement is that both are already right for the same
reason: GAMEPLAY.md §9's split, solids respect faces and fluids connect
by touching, means a pipe's shape is knowable before the click and a
belt's hand-off is the simulation's own answer (`resolve_sink`). A
ghost previews what the connection rules make certain and never what the
sim still has to decide — which is also why the pipe ghost still shows
no machine or tank fitting. Recorded as UI_UX.md §2b point 34; no
behaviour changed.

**Deliberately left, with evidence.** `resolve_sink` lets a belt feed a
machine through any side of an inlet-edge tile, and registers a belt
sitting on a machine's *outlet* face as feeding it. This is not
theoretical: a crusher facing east crafts when fed from its north flank,
and a belt on an outlet face shows up in `links` as both an input from
and an output to the same machine while carrying real product. Both
`world.rs`'s module doc ("A belt pointing into a machine's flank does
nothing") and GAMEPLAY.md §9 say otherwise, so the code is the outlier
and `sim_core/tests/links.rs` pins the current behaviour on purpose. The
fix is one condition (`belt.direction == sink.direction`), but it is a
gameplay change, not a bug fix: it breaks `test_layouts.gd`'s
`build_demo_line` and every save built against the current rule. That is
the owner's call to make, not a close-out ticket's. Also open: nobody
has watched the belt animation run live at speed (`BELT_FRAMES` is a
judgement a headless run cannot make), `_read_world_layout` is still
~40 ms of an 80 ms rebuild at 1500 tiles and belongs to milestone 8, and
every perf number in the plan doc is measured on the debug `sim_core`
`cargo xtask` builds.

### 2026-09-15 — Every screen read once, on purpose (quality pass, ticket 5)

The last ticket of the quality pass, and the only one whose method was
looking rather than measuring: render every screen the game has at
1600x900 through `scripts/screenshot-scene.sh` — menu, pause, build menu,
market, contracts board, staff panel (funded and short), production panel,
a configured hopper, every hover (machine, hopper, tank, dock, pipe,
splitter, belt), the bottleneck line, the belt gallery, a closeup, first
person, a payout — and actually read each PNG rather than confirming it
rendered. Five things read wrong. Two of them ticket 3 had already found
and left here by name, which is the argument for the ticket existing.

**A panel could land on another panel.** `_panel_position_near` places
every hover tooltip and the hopper/tank config panel, and it flipped only
off the right and bottom edges of the *viewport*. The hotbar is not part of
the viewport's edges, and neither is the legend or the contracts clipboard,
so a tile low on screen put its tooltip through the hotbar and a tile high
on screen put it over the clipboard's text. It now tries all four quadrants
around the cursor and takes the first that clears the viewport, the
hotbar's reserved height and every fixed plate. The fallback is where the
real lesson is: a tall tooltip near the top middle fails all four
quadrants at once, and the first fallback written — clamp on screen, then
push off the overlapping plate along whichever axis it penetrated least —
pushed the tooltip straight down off the bottom of the screen, where the
same clamp snapped it back onto the plate it was fleeing. `_pushed_clear_of`
now tries all four directions off a plate and keeps only one that survives
being clamped back. **A position fix has to be checked after the clamp, not
before it.**

**The legend ran under the clipboard.** Ticket 3 reported this on every
screen and left it. The cause is that the legend's panels line has been
quietly growing — E and M, then C, then H, then P, plus the "click a hopper
or tank" hint on the same line — and nothing caps its width. Splitting the
configure hint onto its own line makes the legend five lines and takes the
longest line back under the clipboard's left edge. Worth saying plainly:
this is the second time that line has been cut for exactly this reason, and
it will happen a third time when the next panel gets a key.

**Two notices in one poll, one of them never seen.** Also ticket 3's, also
left here. `_announce_contract_changes` calls `_show_contract_notice` in a
loop, and that function assigns `_contract_notice.text` synchronously — so
two contracts lost on the same poll, or a loss and a signature together,
showed only the last one, the earlier text having been overwritten before a
single frame drew it. The fix queues the *second and later* notices in a
poll and drains them from `_update_contract_notice` once the current one's
hold and fade have fully elapsed. It is deliberately not a queue-everything
design: `test_contract_board.gd` asserts `.text` immediately after signing
and after a loss, with no frame pumped in between, so anything that defers
the first notice of a poll fails both. Keeping the first synchronous makes
the fix invisible to every existing caller and to the tests, which is the
right shape for a bug about a second call.

**A card clipped its own new line.** Ticket 4 added the running cost to the
build menu's crew line, joined with ` · `, and on the card's real width
that runs past the edge on every kind with a crew — painted over by the
next card's stylebox, so it read as a truncated word rather than as
overflow. The obvious fix, a taller card, does not work here and it is
worth recording why: `_fit_grid_scroll`'s scroll cap is set by the *panel's*
available height, not the grid's natural height, so a taller card does not
raise the cap — it clips more of row two before the scroll begins, and past
a point puts a scrollbar on screen at 1600x900, which UI_UX.md §5c rules
out. So the second line is paid for out of the card's own padding (UI_UX.md
§2b point 32): `_crew_cost_text` returns a literal two lines and a new
`SHOP_CARD_ICON_SIZE` shrinks the build card's icon by what the line costs.
Autowrap alone was tried first and broke the phrase in the wrong place
("manufacturing · 2 /" then "period"), still overrunning.

**A hover read out the wire.** A splitter's hover printed its outputs as
tile coordinates (`12, 7`), which is the wire's representation, not the
player's — UI_UX.md §5e's rule that a player-facing line never shows the
wire's own ids or units. It says `connected` / `not connected` now, which
is the whole of what the player can act on.

The other half of the ticket was playing a new game from nothing to its
first payout, driving the real command boundary the way `test_layouts.gd`
does, watching for anywhere a player would not know what to do next. That
turned up nothing needing a fix: the build prompt, the larder, the
clipboard's docket and the payday line between them answer the questions
the run raises, in that order. The one oddity found is in the screenshot
harness rather than the game — `--hover` can settle the cursor a tile off
what it was asked for — and it is reported, not fixed, since it is not
reachable in play.

Docs: this file's Current status now covers all five tickets of the pass
with one clause each, the staff-pools bullet is amended where ticket 1
superseded it, CLAUDE.md's decisions log gains ticket 5's entry and has
ticket 4's crew-cost line corrected to the two-line format, and
`docs/QUALITY_PASS_PLAN.md` is closed out with what each ticket decided.
`cargo test --workspace` and `scripts/verify-core-loop.sh` are green.

### 2026-09-14 — Staffing legibility finished: why, before, and after (quality pass, ticket 4, part 2)

Part 1 put the wage bill on the clipboard and left three things open: the
per-category demand line, the before-payday warning, and an after-payday
announcement. All three are in now, closing ticket 4.

**WHY.** Each staff-panel row gains a line under its meter naming what is
actually asking for that category: `asked for by: crusher ×1, mixer ×1,
cooking vat ×2`, one kind per placement counted once with a running total,
in the order the factory first asked. A category nothing has placed yet
says so in a full sentence, `nothing placed asks for staff yet`, matching
the idle note's own register rather than an empty bar. Every row also gets
one sentence of what the category does (`STAFF_CATEGORY_BLURBS`) — hovering
a machine already explains a shortfall in the moment; the panel now
explains the categories themselves, for a player who has never been short.

**BEFORE.** The footer replays the payday cascade against the wire's own
numbers (`_staff_unpaid_count_if_payday_now`) and, when the balance falls
short, prints the warning in the ticket's own words: `▲ next payday short
by 6 — 3 heads go unpaid` (singular `1 head goes unpaid` where it applies).
Covered, it prints `balance after payday: N` instead — the player always
sees which of the two futures they are in. HIRE now reads the wire's own
`can_hire` per row (ticket 1's hire-refusal rule) rather than always being
enabled, so a row that cannot afford another head says so before the click
is refused, not after.

**AFTER.** A payday that idles or loses heads is announced through the same
mechanism a lost contract already used (`_show_contract_notice`), naming
category and count: `▲ PAYDAY: 3 packaging unpaid, idle until next payday`
/ `✗ PAYDAY: 1 logistics left — unpaid two paydays running`. Detection
mirrors the contract-loss idiom exactly — the first poll after a rebuild
primes the known payday clock without announcing, so loading a save mid-
period does not fire a stale notice for a payday that already happened. A
row also gains its own `status_text` (`▲ 2 unpaid, idle until payday`, `▲ 1
left last payday`, both at once joined with a separator) so the same fact
is legible on the row, not only in the transient toast.

Two things worth keeping.

**A pure-function replay beats reading the last cascade off the wire.**
`_staff_unpaid_count_if_payday_now` does not read any "would be unpaid"
field — none exists — it replays payday's own arrears rule client-side
against `owed`/`wage` per category in wire order, the same order the sim
pays in. It had to reproduce the rule exactly (a zero-wage head is never
unpaid even mid-cascade) rather than approximate it, because the ticket's
own example number (short by 6, 3 heads unpaid) is a real assertion, not a
plausible one.

**A fixture that intentionally breaks affordability has to hand it back.**
`test_layouts.gd` gained `build_underfunded_line` (the reference line funded
to 8, short of its next 14-bill by 6) for the panel-against-wire tests and
the new `--staff-shortfall` screenshot flag. Loading it mid-test disables
every row's HIRE via `can_hire`, which broke the pre-existing real
HIRE/FIRE command tests that ran afterward in the same script — they now
explicitly rebuild the funded reference line before running, rather than
assuming the previous section's world state.

Reviewed both `--staff` and the new `--staff-shortfall` screenshots at
1600x900: the WHY lines and blurbs read cleanly under each meter, the
footer's bill/countdown and the shortfall warning read without clipping the
clipboard or the hotbar, and HIRE reads visibly disabled in the shortfall
shot. Ticket 5 (a polish sweep) is next; nothing here should need revisiting
from it.

### 2026-09-14 — The wage bill is on the clipboard (quality pass, ticket 4, part 1)

Wages are GAMEPLAY.md §5's third recurring cost axis, and until now the only
place the bill appeared was inside the H panel. Ingredients have the larder
strip, equipment has its prices in the shop; a charge that is levied every
pay period whether or not anyone is looking had to be gone and looked for.
It now sits on the always-visible clipboard, directly under the money it is
drawn from: `wages 14 / period  ·  payday in 6s`. When the balance cannot
cover the next bill the line takes the docket's one warning red and names
the gap in figures and in words beside a glyph — `⚠ wages 16 due in 4s —
short 6, unpaid heads leave` — per UI_UX.md §7, colour is never the whole
message. With nobody hired the line hides itself rather than reading zero.

Two things worth keeping from building it.

**Both readouts now quote the wire, not their own arithmetic.** The clipboard
line and the H panel's footer take `next_bill` and `shortfall` straight from
`staff_snapshot_json`. Since ticket 1's notice pay those are genuinely not
the same as Σ headcount × wage — a head fired part-way through a period is
still owed for it — so a Godot-side sum would have been a second opinion
that disagreed with the sim exactly when someone had been let go, which is
the one moment the player is watching the number. `_staff_bill` survives as
the fallback for a snapshot older than the field, and as what it honestly
is: the run-rate, not the next charge.

**A shared `LabelSettings` is shared state.** The first cut recoloured
`_contract_meta_settings` to go red — the object every docket's footnote
draws from — so one bad payday turned the whole board red. The wages line
owns its own settings object, duplicated from the meta ink, and
`test_staff_panel.gd` asserts it is not the same object rather than only
checking the colour.

Still open on this ticket: the per-category "asked by: crusher ×1, mixer ×1"
lists, the walk-out announcement through `_update_contract_notice`'s
mechanism, and a `--staff` screenshot flag to review the panel itself.

### 2026-09-14 — The always-loaded docs were cut down again

The three `@`-included files had grown past the 40k-char budget a second
time (73k, 45k, 43k) — the same failure mode as the 2026-09-09 pass below,
this time led by `CLAUDE.md`'s decisions log (54 entries, ~850 lines) and
`UI_UX.md` §2b (32 points) rather than by `ARCHITECTURE.md`. Same fix,
same rule: every entry's full text was confirmed present in
`docs/DECISIONS.md` (two entries — the 2026-09-13 production-stats log
entry and §2b's point 32 — had been added to the loaded docs but never
backfilled, and were copied over first so nothing was lost), then each
entry in `CLAUDE.md`'s log and each point in `UI_UX.md` §2b was condensed
to its rule in 1-3 sentences; two fully-superseded log entries (the andon
Godot/asset tickets, since the poles they built were deleted) were dropped
outright rather than condensed, their text still intact in DECISIONS.md.
`GAMEPLAY.md` §9 got a different treatment: its dated epic/ticket
narrative substantially duplicated this DEVLOG's own entries, so it was
rewritten as current-state design rules only, with a pointer here for
history, rather than needing a DECISIONS.md Part D. The three files are
now 29k, 32k and 28k.

### 2026-09-13 — Quality pass, ticket 1: nobody walks out silently, nobody works for free, and money cannot overflow

The first ticket of the quality pass (`docs/QUALITY_PASS_PLAN.md`), all
in `sim_core/` and `data/`. The report was "I kept having to hire staff I
never fired", and the diagnosis was the MVP's payday rule: the first head
the money did not reach, and every head after it, was fired on the spot
with nothing but a footer count to say so. A player who bought a machine
just before payday lost most of their crew and found `hired 0`.

**The rule now mirrors the contract board's strikes.** A head the money
does not reach is *unpaid*: still hired, not at its post until the next
payday (the pools reset to headcount minus arrears, so its machines starve
with a `staff` stall and the panel can say `N unpaid`). Unpaid at the next
payday as well, it leaves; a covered payday clears the arrears, back pay
first. Before any of that the wire says what the next payday will charge
and how far short the balance is, so the panel can warn ahead. Two dodges
went with it: a head is owed for any period it was employed in any part
of, so firing everyone the tick before payday bills exactly what keeping
them would (notice pay, settled before the heads still on the books); and
a hire is refused while the balance could not cover the next bill plus
the new wage, so nobody works a period for free. `sim_core/tests/staff.rs`
reproduces the report end to end and pins every rule. The save carries
the arrears under `#[serde(default)]`; schema stays 1.

**Money is `Money` now, `fixed`'s `I48F16`** — the same 16 fractional bits
as `Fx`, 48 integer bits instead of 16. `I16F16` topped out around 32 767,
which a five-line board earns in minutes, and at the top a payout
*panicked* in a debug build (the one `cargo xtask` ships) and wrapped to a
negative balance in release; a probe proved both. Every price, payout,
wage and balance is the wider type; a payout is a saturating add on top;
old saves read unchanged because the raw bits mean the same thing.

**The audit** of the rest of the simulation found no other exploit that
pays: the machine claim order is tile-sorted, so remove-and-replace cannot
promote a machine; a removed machine loses its contents and its load
window and refunds its price, which is a rearrangement, not a gain; a
contract can be signed once, its first window is a full cadence away, and
losing it costs the re-signing fee; auto-fill spends stock and never
money; the dock is a bounded pass-through into an unbounded pool that
saturates rather than wraps. Two balance facts were checked and written
into `data/staff.ron`'s header: the starting crew is exactly the reference
line's demand (pinned), and wages are about a quarter of a first line's
margin. Listed, not changed: a removed hopper destroys the stock in it
without saying so (a legibility item for ticket 5); a factory that can pay
only half its crew keeps everyone at half labour indefinitely (fair, and
documented); `test_layouts.gd`'s `fund` still clamps at 32 767 (test-only,
ticket 3's if it wants it). Full record: `docs/DECISIONS.md` Part A.
### 2026-09-13 — Items on belts stand apart (quality pass, ticket 2)

The project owner's playtest note was that items on conveyors were too
close together. They were: an item is modelled at half a tile and the
belt's minimum spacing was a quarter, so a fed lane drew as one continuous
bar of product. The spacing is now **half a tile** — the one constant in
`world.rs`, chosen by rendering the bottleneck line at default zoom and
close up: at a half tile every tomato and sack stands on visible belt and a
jammed lane still reads as a queue; wider read as a trickle. Belt speed is
unchanged, so a belt now carries one item per 25 ticks instead of one per
12.5, still more than twice what the shipped line's slowest stage asks of
it, and the reference line's steady output is the same 41–42 boxes per
10 000 ticks before and after.

Widening the spacing exposed a packing bug the old number had hidden: a
belt's front item ran right up to its end while the next tile could hold
an item at its start, the same point, so a jammed line held five items per
tile with one of them drawn twice at every boundary. The front item now
keeps the spacing behind whatever stands at the start of the tile ahead
(`BeltSegment::tick_toward`, fed a pre-tick snapshot of every downstream
room so the outcome is independent of visiting order), and a jammed tile
holds exactly two.

One knock-on worth the coordinator's eye: the crusher wants four tomatoes
per 80 ticks and one lane now delivers four per 100, so the crusher is
belt-limited (reads `starved · waiting for Tomato` about a fifth of the
time until the line backs up, which now takes ~8300 ticks instead of
~7000), and the mixer behind it runs at the crusher's pace. Nothing
downstream notices, since the vat is the bottleneck, but the staffing
rule "the line short one manufacturing head idles the vat" is now "on the
ticks the crusher and mixer both claim"; on a tick the crusher waits for a
tomato, the vat gets its crew. The tests say so; whether the crusher's
recipe should be re-timed is ticket 1's call and was not made here.
### 2026-09-13 — Quality pass, ticket 3: presentation bugs and performance

No new features (`docs/QUALITY_PASS_PLAN.md`). Measured first on the
1500-belt stress line (`test_perf.gd`, debug `sim_core`): 8.4 ms a frame,
of which the tick path was 7.2 — each dirty chunk's snapshot re-sent every
belt in the chunk (78 KB a tick, most of it empty belts) and the scene
parsed and re-drew all of them. The sim now offers a **chunk delta**
(`chunk_delta_json`: only what changed, the whole chunk after a structural
change) and a **revision counter** (`revision()`: ticks plus commands), and
`_process` polls its snapshots only on frames where the revision, the
viewport, the selection or the open modal moved. After: 2.1 ms a frame,
2.2 ms a tick for `_apply_dirty_chunks`, 12 KB a tick. Draw calls
unchanged (188 on the stress frame).

| stress line | before | after |
|---|---|---|
| `_process` per frame | 8.4 ms | 2.1 ms |
| `_apply_dirty_chunks` per tick | 12.3 ms | 2.2 ms |
| wire bytes per tick | 78 KB | 12 KB |
| warm `_rebuild_world` | 70 ms | 80 ms (re-reads every chunk, see below) |
| objects / primitives / draw calls | 867 / 4.56 M / 187 | 873 / 4.56 M / 188 |

Bugs fixed, each with a headless proof: items on belts in another chunk
vanished after any place/remove/rotate until they next moved (a rebuild
now re-reads every occupied chunk; `test_splitters.gd`); a right-drag
whose release landed on a panel or the pause menu panned the camera
forever (`_input` ends it; `test_camera_nav.gd`); TAB into first-person
captured the mouse under the market/board/staff/production/config panel
(one `_fp_mouse_free` decision; same test); R rotated the tile behind an
open panel and X entered remove mode under one (`test_interaction.gd`);
the production panel rebuilt five styleboxes per window button on every
update. Every screenshot flag was rendered and read. Left for ticket 5:
the legend's third line runs under the clipboard on every screen. Left
and reported: the 336 KB layout parse that is half of a rebuild on 1500
tiles, and that every number here is on a debug build of the sim.

### 2026-09-13 — The P screen: what the factory made and used

Factorio's production statistics, both halves. The simulation half had
been started the day before and left uncommitted in a worktree: a
production ledger on the world counting, per item and fluid, what every
recipe made (at craft completion) and used (at craft start) over four
rolling windows and for all time, with a paid contract counting as using
the boxes it ships and buying or filling a bin counting nothing at all.
It was committed as it stood, merged behind the staff pools — a craft now
records at exactly the points a crew is claimed for, and a refused crew
records nothing — and its tests came through green.

The Godot half is the **P** panel, in the same dark plate as the staff,
contracts and shop menus and one modal at a time with them. Rows are
sorted biggest first and formatted here, never counted here: an icon, the
name, and MADE / USED bars with the count and a per-minute rate printed on
each, scaled against the largest count on the list so a glance says which
item moves most. The rate divides by how much of the window has actually
been observed, so a factory thirty seconds old reads its real rate rather
than a twentieth of it. A selector picks the window or all time; an empty
window says so in a sentence. Polled only while the panel is up, frozen by
the pause menu, and a loaded save reads its counts on the first frame.
### 2026-09-12 — Music, and a sound for everything (audio pass)

Requested directly: "add some nice music and sound effect". The
2026-09-11 audio decision had said *feedback only, no music*; the owner's
ask reverses that half and the rest stands (synthesised in GDScript, no
`.wav` binaries, cues fire on events). What landed:

- **A looping track** — `audio.gd`'s `music_samples()`, twenty seconds of
  D minor at 92 BPM built from the same sine/noise primitives as the cues
  (a detuned pad, a plucked bass, a mallet arpeggio, kick/rim/hat), every
  note wrapping at the loop end so the seam is inaudible. It lives on a
  new `Music` autoload so it starts on the main menu and does not restart
  when the scene changes to the factory; it is rendered on a thread the
  first time it is needed (0.7 s of GDScript) and never under a headless
  run. Every quit path now goes through the autoload, which stops the
  track two frames before the tree is torn down — a stream still playing
  at that instant raced the engine's exit check and reported its playback
  leaked, three runs in four.
- **Twelve new cues** beside the payout chime and the dock knock: place,
  remove, rotate, buy, sign, hire, fire, select, menu open/close, one
  shared *refused* buzz, and a lost-contract cue. Each is fired from the
  handler that can tell whether the simulation took the action (owned
  count, balance, occupancy or headcount compared before and after),
  because every command is a silent no-op on refusal and a click cannot
  know.
- **Pause ducks the track** rather than stopping it, and the pause menu
  gains MUSIC: ON/OFF beside SOUND: ON/OFF.
- `test_audio.gd` (`AUDIO_OK`) proves every cue's stream is present,
  deterministic and unclipped, the loop's length and seam, the duck, and
  every handler's cue and refusal; `scripts/verify-core-loop.sh` runs it
  (29 headless proofs now). UI_UX.md §8 and the decisions log record the
  amended direction.

### 2026-09-12 — The staff panel: hire, fire, and see who is short (staff pools, ticket 3)

The staff pools have a face now. **H** opens a plain dark plate with one
row per category: ticket 2's icon, the name, `hired 4 · wage 2 / head /
period`, a bar of how many the factory asked for on the last tick against
how many it got — `2 / 4`, printed on the bar, with `2 short` beside it in
red when the two differ — and HIRE and FIRE, each moving one head through
the real command with the panel re-reading the wire on the same click.
The footer says what the next payday will cost (every head at its wage)
and how many seconds away it is, and only when it happened, how many
people left unpaid last time. Maintenance, which nothing asks for yet,
says so in a sentence instead of showing an empty bar.

Being short shows where it bites, too. A machine that was refused its
crew reads `▲ starved · 0% busy · short of manufacturing staff` in its
hover, over a line saying what its crew is (`crew: 2 manufacturing`); a
dock without its logistics head reads `held · short of logistics staff`;
and the build menu's card says under a machine's price what people it
will need. None of it is worked out in Godot — demand, assigned, short and
the stall reason are the simulation's own last-tick figures, polled once a
second of sim time like the machine load and frozen with the pause menu.
The one sum Godot does, the upcoming bill, was put to the coordinator
first and is recorded as arithmetic on wire numbers rather than a derived
fact.

Two things the renders taught (recorded in `docs/DECISIONS.md`): the
panel's rows had to lose a line each to fit four categories above the
hotbar, and the build-menu card had to pay for its new crew line out of
its own padding, because a taller card scrolls the two-row grid. The
andon light the plan asked the proof to check no longer exists; the proof
checks the hover panel that replaced it.

### 2026-09-12 — The factory needs people (staff pools, ticket 1)

The simulation half of the staff pools MVP. Staff are GAMEPLAY.md §7's
aggregate pools, not people on the floor: `data/staff.ron` defines four
categories with a wage and a starting headcount, every kind in
`data/machines.ron` now declares the crew it needs while running (the
file went to schema 2 so an omitted crew is a loader error), and each
tick every machine claims its whole crew in the simulation's stable order
before it works. A machine that cannot get its crew takes nothing, pauses
exactly where it is and reads *starved* with a new `staff` stall naming
the short category — so the shipped line one manufacturing head short
idles the cooking vat, last in order, and nothing else. The dock needs a
logistics head to ship and holds, without losing a box, without one.
Wages are paid every pay period out of the one money counter, one head at
a time in file order, and the first head that cannot be paid — and every
one after it — leaves; hiring and firing are free commands. The starting
crew is exactly the reference line's summed demand, pinned by a test
against the data rather than by literals, so a first factory runs without
a visit to the staff panel and wages bite from the first payday.

One consequence reached every long-running test fixture: a factory with
no money now has no staff, so fixtures that used to stock up and run
broke fund themselves with a wage reserve and assert what they *earned*
rather than what they hold. Nothing on the wire Godot already reads
changed shape — it only gained keys — and the Godot proofs still pass.
Tickets 2 (icons) and 3 (the staff panel) follow.

### 2026-09-12 — The staff categories have faces (staff pools, ticket 2)

The staff-pools epic's asset half (`docs/STAFF_POOLS_PLAN.md`): four voxel
props, one per category, rendered through the icon pass so ticket 3's
staff panel has a picture beside each name — a hard hat for manufacturing,
a double open-ended spanner for maintenance, a taped carton with the roll
of tape on its lid for packaging, a pallet jack for logistics. Each is the
tool the category holds rather than a person, because GAMEPLAY.md §7's
staff are a capacity, not NPCs, and the four were picked to have four
different outlines. Registered like the coin — in `ASSETS`, never placed,
no footprint — and authored for vengi's camera, since the icon is their
only consumer. No palette change, so nothing else re-emitted.

Three read at 64 px on the dark plate first time; the hard hat read as a
bell until its crown was made wider than tall — UI_UX §2b point 9's
budgeted rework pass earning its keep, and the lesson is the new point 31.
A voxgen test pins the four names in category order, non-empty, inside
32³, the plan's material classes, and that the spanner's jaws are open.
Full record in `docs/DECISIONS.md`.
### 2026-09-12 — The stack lights come off the floor; the hover panel is the indicator

A playtest verdict on the machine-load epic: the andon poles beside every
machine were not liked, on looks alone, and they are gone — the six
`andon_*` models, their generator and voxel tests, and the three sibling
nodes the scene stood per machine. What the epic was for survives intact:
the simulation still measures every machine's load and stall, the scene
still polls it once a second of sim time (frozen by the pause menu, read
on the first frame of a load), and the hover panel says it. The hover's
load line now leads with a status glyph and word — ● flat out, ▲ starved,
■ blocked — and its load bar is filled in the status colour, so the
tooltip is the andon: shape, word and colour, never colour alone.

What is lost is floor-wide legibility — the bottleneck used to be visible
from across the factory as the one green light in a row of red and amber,
and now it takes a hover per machine. That is recorded as an open thread
rather than patched: if a whole-floor readout is wanted it should be a
different visual answer (an overlay, a HUD list), not the poles back.

### 2026-09-11 — The bottleneck is visible (machine load, ticket 3)

The machine-load epic is complete. Every processing machine stands an
andon stack light on the corner of its footprint the camera can see — a
corner derived from `CAMERA_DIRECTION`, not chosen — and lights the one
lens the simulation's verdict names: green flat out, amber starved, red
blocked. The lights are polled from the simulation once a second of sim
time, so the pause menu freezes them, and once on every rebuild, so a
loaded save shows its lights before its first tick. Hover says the same
thing in words with the number and the reason (`starved · 38% busy ·
waiting for Sugar`), keeping the last reason across the ticks a blocked
machine has none, and draws the load as a second bar under the progress
bar. A new fixture runs the shipped line on auto-fill for 8000 ticks until
it has backed up behind the cooking vat, which is what
`scripts/screenshot-scene.sh --bottleneck` photographs: red, red, green,
amber, amber along the line. One number moved in the making: the hover's
"of the last 10s" is 20 s, because the sim's window is 600 ticks and the
scene runs at 30 Hz. The render verdict on the asset at the whole-line
zoom is recorded in the decisions log for the coordinator.

### 2026-09-11 — The andon stack lights exist (machine load, ticket 2)

The machine-load epic's asset half: six voxel models of an industrial
andon — a painted post with a green, an amber and a red lens ring, one lit —
that ticket 3 will stand at the camera-visible corner of every processing
machine so the bottleneck can be read across the floor. Two heights rather
than the plan's one, because the render said so: a single post tall enough
to clear the cooking vat was two and a half crushers high, so 1x1 kinds get
a short stack and larger footprints the tall one, picked by footprint.

The interesting work was in the tests, not the model. The post is never
rotated but the machines are, so a voxgen test overlays the post on every
machine body at all four facings and fails on any face the two would draw
in the same place; it found two body features (the crusher's motor, the
packer's lamp) sitting exactly one voxel from a far edge — invisible in any
preview, and a z-fight the moment that machine was turned. Both moved a
voxel. `VoxModel::rotated_cw` learned to turn an oblong model for the
mixer's sake. Full record in `docs/DECISIONS.md`; the camera lesson is
UI_UX.md §2b point 29.
### 2026-09-11 — Machine load, ticket 1: the simulation measures every machine's load

The first half of the machine-load epic (`docs/MACHINE_LOAD_PLAN.md`):
every recipe-running machine now keeps a rolling ten-second window of what
it did each tick — working, starved, or blocked on its output — and the
world reports a percentage, a `working`/`starved`/`blocked` verdict and the
concrete reason it is waiting (`waiting for sugar`, `outlet pipe full`,
`no supply pipe for vinegar`…) as structured data. It rides the existing
chunk and tile snapshots and a new polled `machine_load_snapshots_json`,
survives a save with no schema bump, and never dirties a chunk. The
counting rule was chosen so a saturated machine reads exactly 100 % rather
than dipping once per cycle. The integration test found that the shipped
line takes about 7000 ticks — not the plan's 1500 — to back up all the way
to the crusher, which ticket 3's fixture and screenshot need to know.
Tickets 2 (the andon stack-light models) and 3 (lights in the scene, hover,
fixture) are what make it visible.

### 2026-09-11 — Six feature branches consolidated into one playable main

Six finished-but-unmerged branches were sitting in separate worktrees, each
green on its own and none of them in `main`: the contract board, the payout
juice and its audio, machine animations, the floor redesign, the vinegar
assets, and the belt-arrow removal. They are now one history, merged onto
`integration/all-features` and fast-forwarded into `main`.

Most of the conflicts were not disagreements, and resolving them by picking
a side would have lost work. Three kinds, with the rule each one wants:

- **Both sides prepended to the same newest-first list** (this log, the
  decisions log, the two design docs). There is no conflict here at all —
  the file is a stack and both entries belong on it. Keep both, theirs
  first. Never `--ours`/`--theirs` on an append-only document.
- **About 130 conflicting `.vox` binaries.** No side was right. Two branches
  had each added colours to `palette.ron`, which shifts every palette index
  after them, so every committed model disagreed. `.vox` files are **build
  output** (UI_UX.md §2a point 2): merge `palette.ron` and the generators,
  then regenerate. 138 files became 189.
- **One real semantic clash**, which no file-level conflict showed. The
  payout test read the two contract shapes it needs from the ACTIVE
  contract list — true until the contract board made only the one
  `starter: true` contract active at tick 0. Both branches were correct
  alone and wrong together; the test now takes its second shape from the
  unsigned offers on the board.

`scripts/verify-core-loop.sh` passes on the union: 19 Rust suites, 26
headless Godot proofs including all six features' own markers, no failures.
A rendered frame shows the whole set at once — the new floor under an
animated line, coins over the dock, the strikes-and-offers board, a signed
toast and the money flash.

**Lesson worth keeping:** a green branch and a green `main` do not make a
green merge, and the thing that catches it is running the whole suite on
the union, not re-running each branch's own tests. Also, the voxel import
cache is per-worktree and cold: a fresh checkout needs `cargo xtask` and
**two** `godot --import` passes (ARCHITECTURE.md §2b point 4) or the game
boots into an invisible world with nothing but warnings in the log.

### 2026-09-11 — Earning money is an event now (and the game makes noise)

A directly-requested pass with a simple brief: *"whenever we earn money i
want some cool animation or particle effect and maybe a sound effect too. i
want this game to be super satisfying to play like how crossy road is."*

Money arrives through exactly one channel — a contract settling at its
cadence boundary — and at cadences of 1000-2500 ticks that is rare enough to
deserve a real celebration. A payout now fires four things at once: a
fountain of coins arcing up over every dock, the money counter punching up in
scale and flashing amber, a "+N" rising out of the money line, and a **PAID**
stamp on the docket that paid. The frequent moment underneath it — a case
landing on the dock — gets a deliberately small cue instead: the case pops in
with a short overshoot and a quiet knock plays. Two sizes of feedback for two
frequencies.

**No simulation change was needed, and none was made.** Godot notices a
payout by watching each contract's `deliveries_made` for a rise in the
`economy_snapshot_json` it already polls every frame; the first poll after a
rebuild or a load seeds silently, so loading a save with forty deliveries
behind it does not throw forty celebrations. The whole thing is clocked off
`sim.current_tick()` rather than wall time, so it freezes when the game is
paused and a fixed-tick screenshot run is still reproducible.

**The game has sound for the first time**, and the sounds are *code*: a small
`audio.gd` synthesises each cue into an `AudioStreamWAV` at runtime instead
of the repo carrying `.wav` binaries, the same way `assets/voxels/generators/`
makes the models code. That is deterministic enough for a test to assert two
renders of a cue are byte-identical, and it settles UI_UX.md §8's long-open
"audio direction (not decided)" as **feedback-only sound, procedurally
generated** — no music, no ambience, with a SOUND switch on the pause menu.

Two bugs were found by the rendered frame that every headless assertion had
passed straight through (UI_UX.md §2b point 12 again). The coin arc was
scaled in raw world units while a tile is 3.2 of them, so its peak was a
fifth of a tile and its spread was narrower than a coin: ten coins drew as
one gold lump sitting on the deck. And the "+N" flyout is parented to the HUD
`CanvasLayer`, so nothing ever set its size, and right-aligning a zero-width
label put the text off the edge of the screen — then, once fixed, off the top
of it, because a top-right clipboard leaves no headroom above the money
figure to rise into. Both are recorded as rules: every distance in a 3D
effect is measured in `TILE`, and a flyout rises *out of* the thing it
announces.

`test_payout_feedback.gd` proves the structure (seeding, one event per poll
however many windows settled, the totals, the decay of every element back to
nothing, the dock cue, audio determinism) and `scripts/screenshot-scene.sh
--payout` freezes the scene partway through a real burst so the look can be
judged. Full record in `docs/DECISIONS.md`.
### 2026-09-11 — The machines move, and only while they are working

Until now a running factory and a jammed one looked the same: the product on
the belts moved, and every machine it passed through stood perfectly still.
Each of the six processing kinds now has a **motion overlay** — a second
voxel model on the machine's own footprint, emitted in four frames, standing
beside the body in the scene with one frame visible at a time. The crusher
turns twin toothed rolls behind a pair of flywheels, the mixer turns its
ribbon flights inside the drum where the sight window shows them, the
cooking vat turns an agitator spider on its crown, the bottler plunges two
filling nozzles and a capping chuck, the packer runs a taping carriage along
the case, and any kind with no model of its own gets a drive spider on the
generic housing — so a machine added to `data/machines.ron` visibly *runs*
before anyone edits a Godot file, which is the same fallback rule the
machine models themselves follow.

The parts that move were **cut out of the bodies** rather than copied into
the overlays, because two voxel solids that overlap z-fight and a z-fight at
scene scale reads as a texture seam rather than as a bug. A voxgen test
walks every voxel of every frame against its body to keep it that way, and
it is what found the mixer's ribbon passing straight through both feed
throats and the discharge nozzle.

The other half of the request is the more interesting one. A mechanism turns
only while the machine is **genuinely working** — not merely while it holds
a batch. `Machine::tick` clears a machine's process only once the finished
goods have somewhere to go, so a machine that has finished and is blocked on
a full output keeps its process and retries every tick; animating on that
would have left a jammed factory full of machines cheerfully turning with
nowhere to put anything. Gating the motion on `elapsed < required` instead
leaves the status lamp saying "has a batch" and the mechanism saying "is
turning", and **a lit lamp over a still machine now means the thing
downstream is full** — a diagnosis the player could not previously read off
the floor at all. Everything runs at one pace (a cycle per 30 ticks) whatever
its process length, because the pace is meant to say "working", not "working
this fast"; the belt is what carries speed. The frame comes from the
machine's own elapsed ticks, so a mechanism starts from rest when work
starts, and the whole thing is advanced from `_advance_simulation` beside the
belt, so the pause menu freezes it for free.

Two things turned up along the way. `test_screenshot.gd`'s populate loop
ticks the simulation directly rather than going through the per-frame path,
so it had **never** advanced the belt frame either — every scene screenshot
ever taken has photographed an idle-looking factory in the middle of a run.
It drives both animations now. And the vengi preview of a frame that is a
rigid translation of frame 0 is byte-identical to it, because the thumbnailer
crops to the bounding box: a translation has to be judged in the scene, which
is UI_UX.md §2b point 22's warning about vengi's camera wearing a different
hat. Proved by `test_machine_motion.gd` (frames per kind, the frame from the
machine's own elapsed ticks, and motion only while it is actually working)
and reviewed in a rendered scene. Rules in UI_UX.md §2b points 27-28.
### 2026-09-10 — The belt arrows are gone

The player asked for the yellow direction arrows on the conveyor belts to
be removed, and they are. What replaces them is deliberately almost
nothing: a plain **cleat**, a bar of dark rubber two voxels long across
the lane, one voxel proud of the surface, because the arrow was also the
belt's entire animation pattern — with it simply deleted all four frames
would have been identical and every belt would have stood still with every
test green. The cleat keeps the frames distinct so a belt visibly runs and
a corner's cleat still bends with the bend. Direction is in words where it
already was: the build prompt's `facing east` and hover's `carries east`.
The splitter's inlaid next-output arrow is a different readout and stays.
Twelve belt models and three belt icons regenerated; `cargo test -p
voxgen` now refuses any signage colour on a belt's running surface.
Checked in `scripts/screenshot-scene.sh --belt-gallery --populate` at
scene zoom and in a corner closeup. Decision record in `docs/DECISIONS.md`
Part A (2026-09-10) and an amendment to UI_UX.md §2b point 23.
### 2026-09-11 — Vinegar looks like vinegar

A small directly-requested art pass with one real bug inside it. Vinegar is
one of the three raw ingredients the player buys, and it had no model at all
— so `_market_asset_for`'s fallback answered for it, and the ingredient
market, the larder strip and the hopper picker all showed the player **a
length of pipe** where the thing they were buying should be. Its colour
existed only as one `FLUID_TINTS` gold in `main.gd`, which at scene scale
read as cooking oil rather than an acid.

Now: `palette.ron` has `vinegar_amber` (`#AD6B23`, the `liquid` class) and
`vinegar_glass`, appended so no existing model re-indexes; `main.gd`'s tint
for vinegar *is* that RGB, so the pipe run, the tank's sight window and the
market icon are one colour with one home. `item_vinegar` is a tall,
straight-sided jug with a cream wrap-around label, a steel cap and a moulded
ear, routed through a new `FLUID_MODELS` table — deliberately not extra keys
in `ITEM_MODELS`, which is about what rides a belt, and a fluid never does.

Amber rather than the honest colourless, because a clear fluid behind a
stainless inspection window is indistinguishable from an empty pipe; malt
vinegar is amber and real, so legibility and honesty agreed. The one
genuinely new pipeline rule came out of the render loop (UI_UX §2b point 35):
the jug's label was authored on the faces the game camera sees and was
invisible in its own icon, because `vengi-thumbnailer` looks from the
opposite (-X/+Y) corner — and a fluid never appears in the scene at all, so
the icon is this model's only consumer. `item_box` turns out to have had the
same invisible-label problem all along. Verified with `cargo test -p voxgen`,
`scripts/verify-core-loop.sh` and three scene renders (market panel, pipe
run, tank).
### 2026-09-11 — The factory floor, redesigned

The project owner said the flooring tile did not fit the theme, and it did
not: one `concrete` slab with a near-black saw-cut joint on all four edges
and single-voxel bright speckle over it, identical on every tile in the
world. Rendered wide, the joint read as grout and the speckle as television
static — a tiled bathroom rather than UI_UX.md §3's poured industrial food
plant.

It is a poured floor now. Its own three concrete tones, deliberately within
eight points of each other; broad ragged blotches on a coarse cell grid
instead of per-voxel noise; a hairline control joint that still draws the
tile grid §1 asks for; four models — plain, worn, patched, drained — each
laid at a quarter turn chosen from its own coordinates, which is what stops
four models from reading as four stamps; and trench drains in continuous
rows, the detail that says a plant which is hosed down. The scene got
cheaper as well as better: 1 363 384 primitives to 798 272, because broad
low-contrast areas greedy-mesh far better than speckle, and the floor is the
one asset whose triangle count is multiplied by the size of the world.

Three iterations, each judged by looking. Two lessons general enough to keep
(UI_UX.md §2b points 36 and 37). **The floor is authored by its value range,
not its detail** — it is the one asset seen as thousands of copies filling
the frame, so it must never hold the brightest pixel, which §4 reserves for
metal. That rule broke a tie: a repair patch "should" be a paler fresh pour,
and every paler version read as white paving slabs dropped at random, so the
patch is darker than the floor around it. **And N models over thousands of
tiles is still N stamps** — per-instance rotation is the free multiplier,
and its price is that the tile's edges must be four-fold symmetric, which is
why the joint keeps all four of its edges and was fixed by lightening its
colour rather than by dropping two of them.

The frame that found every one of these was the EMPTY world, not the demo
chain: it is the state a new game starts in, and the only render that is
almost entirely floor.
### 2026-09-11 — Numbers into bars: a UI visualisation pass over every readout

The hopper panel's fill bar, added with the configuration panel, was the
best-liked thing on screen — a level you *see* rather than a level you read
— so this pass took that idiom everywhere the HUD was printing a raw figure
and asked, per readout, what the figure actually means to the player.

The rule it settled on is the part worth keeping: **a quantity gets a bar
only if the simulation defines a ceiling for it.** A hopper against its
capacity, a machine against its cycle, a countdown against its cadence, a
price against the balance in your pocket — those are fractions, and a
fraction is a bar. Raw stock in the larder and the balance itself are not:
nothing says how much tomato is "full", and a bar drawn against an invented
maximum would be a gauge that lies. They stay figures.

All of it goes through one mechanism (`_build_meter`/`_set_meter`) with
three fills that mean things rather than set moods: green is a level, amber
is work under way, red is a cost you cannot meet or a clock nearly out.
Where they landed: hover now draws a hopper's or tank's level, a pipe's
pressure, a machine's cycle and its input/output buffers instead of
listing them; each contract docket has a draining countdown in the
clipboard's own paper and ink, reading **"8s to go"** and **"due now"**
rather than a tick count, with ✓/✗ tallies beside it; every build-menu and
market slot shows its price as the share of the balance it would take, and
a full red bar when you cannot afford it; and the money figure flashes
**+90** or **-5** under itself when something changes it, so a payout that
lands while you are looking at the factory is an event rather than a number
that was different last time you glanced.

Two things fell out of doing it. Everything the player reads is now named
the way a person would say it (`Boxed Ketchup`, `Fluid Tank`) — the pass
kept finding raw wire ids like `boxed_ketchup` in headings, because a
tooltip that echoes the JSON is invisible until you look at it as art. And
a meter records the fraction it drew on its own node, which is how the
headless proofs assert that a bar *moved* without ever laying out a
viewport: `test_contract_panel.gd` watches three countdowns drain
independently, and `test_hover.gd` checks which tiles get a meter at all.
### 2026-09-10 — Contracts are signed, struck and lost

The contracts system stopped being three orders that exist from tick 0
and can never go away. `data/contracts.ron` is a **board of offers**
(schema 2): each contract has a signing `price`, a number of `strikes`,
and optionally `starter`. A new game runs the one starter; the player
opens the **contracts board** on **C** and signs the rest out of the same
money everything else spends — one of each, never two. Miss a contract's
strikes **in a row** and it is lost: it leaves the clipboard on that tick,
a line along the top of the screen says so, and the board offers it again
at full price with a "lost before" note. A paid window clears the strikes,
so recovering is forgiven. This decides GAMEPLAY §11's open question about
missed deliveries (a strike-counted grace period; no reputation meter yet).

On the clipboard each docket now answers the two questions a player asks
of an order: a status line in words with a glyph (READY, AT RISK with the
shortfall and the share of the window gone, LAST CHANCE when the next miss
loses it), one strike pip per strike the contract allows,
and a heavier border in the state's colour — never colour alone. An empty
clipboard says "press C". The shipped board gained a fourth rung, a
regional grocer whose six boxes every 4000 ticks one line cannot carry
beside the other three, which is the pull toward a second production line
GAMEPLAY §6 always wanted.

Sim: `Economy` owns the board, `try_sign`, `ContractEvent`s from `tick`,
file-order insertion, `losses`; the save format keeps schema 1 with
defaulted fields and restores the active subset exactly. Tests:
`sim_core/tests/contract_board.rs`, `test_contract_board.gd`, and the
clipboard test now signs every offer first. Built in its own worktree on
`TomWhitticase/contract-board` because the `anthias` tree had a live
concurrent session editing `main.gd`; it needs merging with that session's
UI visualisation pass (both touch the docket).
### 2026-09-11 — The contract board grows a tier for the second line onward

A playtest stalled at the point where a factory-builder should take off:
the first line paid for a second line, and the second line had nothing to
sell to, because the three shipped contracts absorbed one line's output
and not a box more. `data/contracts.ron` now ships **nine** contracts.
The original three are unchanged and still form a *starter tier* one
kept-fed line can just serve; below them a *growth tier* of six larger,
slower orders (four boxes every 1500 ticks up to a twenty-box pallet
every 5000) asks for about four more lines' worth, at per-box rates that
rise with the size of the order. Priority order stays smallest first, so
a beginner's income is never eaten by a pallet order they have not built
for. No sim change: a content edit, a loader test that pins the sizing
rule against the chain's own bottleneck, and one Godot test loosened to
allow two contracts on one cadence. The clipboard's docket stack now
scrolls at the default window size — the cap that was a safety valve is
the ordinary case, and a compact board is the next UI step if the board
grows again. Every order is still `boxed_ketchup`: a mixed order needs a
second sauce, which is content this pass did not add.

### 2026-09-10 — Click a hopper or a tank to configure it

The presentation half of the previous entry, and the end of feeding a
factory from the keyboard. A left click on a placed hopper or tank — outside
build mode, from any tile of its footprint — opens a small dark-plate panel
beside the building: the level and a fill bar, **FILL** (one load of *this*
bin, disabled with the reason in words when it cannot happen), the
**AUTO-FILL** switch (ON/OFF in text as well as colour, read back off the
simulation's own snapshot every frame), and for a hopper an item picker that
retargets it in place. Clicking the building again closes it, clicking
another retargets it, anything else — floor, a machine, ESC, a menu, a mode,
the building being removed — closes it. **SPACE**, **V** and **Q** are gone
from the legend and the handler, and `CycleHopperItem` is gone from the
simulation with them.

Two things came out of finishing it rather than building it. The headless
proof's first expectation for auto-fill was wrong in exactly the way the
mechanic is right: a tank switched to automatic sat at a fifth full after two
hundred ticks because the larder's single crate of vinegar had all gone into
the tank and its run, and the top-ups had stopped — spending stock and never
buying is the design, so the test now checks the dry case and the stocked
case both. And a deleted key leaves stale copy in more places than its
handler: the market's own footer still told players to press SPACE and V,
and two scripts still drove the dead keys. The panel also had to learn to
step aside for the contract clipboard, which the first rendered frames showed
it sitting squarely on top of.

**With this, the whole save/load + configuration + tuning plan is landed:**
a main menu with save slots and an autosave, a factory that comes back
mid-craft, a slower crusher, a 2x2 dock, a UI at 1.5x, and buildings that
are configured by clicking them.
### 2026-09-10 — Pipes & conveyors overhaul, ticket 5: belts turn corners, run, and connect at every facing

The first presentation ticket of the overhaul, consuming tickets 1, 2 and 4.
Every belt tile now takes its SHAPE from the `links` the simulation puts on
the wire — straight, left corner, right corner, merge, line start, line end
— through one pure function (`_belt_shape_for`), draws the matching
four-frame model family, and spins by its facing like every other asset (a
corner needs no yaw of its own). The `belt`/`belt_inlet`/`belt_outlet`
aliases, models and icons are deleted and every east-only belt comment in
`main.gd`, `world.rs`, `machines.ron` and ARCHITECTURE.md is gone: a belt
meeting a machine, a hopper or a dock is the plain tile at all four
facings, and the machine's hood does the rest.

The surface animates at item speed: frame `floor(tick × 4 / 50) mod 4`,
read from the tick at the end of each simulation step, so the pause menu
freezes it with no extra state. Items follow a quarter-circle arc through a
corner, parameterised by the sim's own 0..1 distance — and fitting that arc
exposed that items had always been drawn half a tile downstream (tile
centre to next centre rather than face to face); they are face to face on
every tile now, UI_UX.md §2b point 26. Hover says it in words (`carries
east · turns left / fed from the west (Hopper) / hands to Crusher`, `not
connected` where null). The build ghost stays a straight tile, on purpose:
the would-be links of an unplaced tile are the sim's to compute
(ARCHITECTURE.md §4's planned `preview_command`), not a Godot-side copy of
`resolve_sink`.

New: `test_belt_shapes.gd` (the shape rule at every facing, the gallery's
eight corners at their facing's yaw, frame index and pause freeze, arc
positions and continuity, hover), `test_layouts.gd`'s `build_belt_gallery`
and `scripts/screenshot-scene.sh --belt-gallery` — both hands of corner and
hopper → belt → crusher → belt → dock at all four facings, a two-flank
merge, a line end and a lone start. Renders read at scene zoom and in
closeups on both rings and all four machine lines, populated and idle: the
corners' rails meet the straights, the arrows bend with the belt, a queue
of tomatoes curves round a corner, and the first tomato stands in the
hopper's mouth. `test_connections.gd` now asserts the retired models are
NOT staged and that turning a fed belt re-shapes it into a corner.
`verify-core-loop.sh` runs 22 Godot proofs; the stress line's rebuild
went from 38 ms to 64 ms for four frame sets per belt family (still one
draw call per set per class per frame).

**Flagged, not judged:** at `TICK_HZ` 30 the four-frame march is 2.4 frame
changes a second. A PNG cannot show stutter and this ticket had no live
session; the number is recorded and `BELT_FRAMES` is left at four. If it
reads as stepping in play, raising the frame count is a coordinator call
(it multiplies MultiMeshes per belt family, not per-frame cost).
### 2026-09-09 — Pipes & Conveyors Overhaul, ticket 6: a pipe is drawn from what it is joined to

The Godot half of the pipe family (`docs/PIPES_AND_CONVEYORS_PLAN.md`,
ticket 6), built on tickets 1 (`links`), 3 (the models) and 4 (instancing).
`main.gd` no longer decides a pipe tile's model from the fluid snapshot's
`is_source`/tap indices plus a "is there a building south of it" guess
(`_pipe_roles`, which could only ever draw one arrangement and fell back
to a plain tube everywhere else). Every pipe placement's `links.connections`
now decides three things at once: the SHAPE (one side joined is an end, two
opposite a straight, two adjacent a corner, three a tee, four a cross), the
quarter-TURN that lands the authored openings on the joined sides, and a
machine or tank FITTING on every side whose connection is one — overlaid
as its own instance, turned to that side. The liquid body is the one
authored for that shape, at the same turn, so a corner's liquid turns with
its bore and an end's stops at its cap; the Godot-side end trim is gone.
Hover says it in words: *connects west · feeds Mixer*, *connects east ·
fed by Fluid Tank*. The deprecated `pipe_source`/`pipe_inlet` generators,
models and icons are deleted.

Two calls the render forced. (1) A straight's turn is decided per AXIS,
not per tile: its one joint flange must sit on the same side of every tile
along a run or two neighbours stack flanges on a shared boundary, so a
pipe's `direction` now decides only which way a pipe joined to nothing
lies. The end, whose turn is dictated by its single open side, would stack
too, so `pipe_end` lost its joint flange. (2) A fitting landing on a
straight's low-X face (the first tile leaving the mixer) covered the
straight's own joint flange and the two voxel solids shared boundary
faces; every part of a fitting is now a full voxel of radius larger than
anything it can cover (`PIPE_FITTING_MARGIN`), pinned by a voxgen test
that overlays both fittings on both faces of a straight.

Verified with `cargo test --workspace`, `scripts/verify-core-loop.sh`
(including the new `test_pipe_shapes.gd`: 52 pipe tiles, every shape at
every turn, 18 fittings, one body per shape, hover, the ghost), and about
twenty closeups of the new `--pipe-gallery` fixture plus the demo, fluids
and chain fixtures read at scene zoom and up close. Flagged for ticket 7:
the pipe ghost previews its shape from neighbouring PIPE tiles only (no
fitting — the sim picks the tap tile), decided here before ticket 5 chose
the belt ghost's rule; the two should agree.

### 2026-09-09 — Pipes & conveyors overhaul, ticket 2: the belt family, drawn and animated

The first asset ticket of the overhaul (`docs/PIPES_AND_CONVEYORS_PLAN.md`).
One parameterised generator now produces the whole belt family the plan
names: a straight tile and a left and a right corner, each in four
animation frames that differ only by the surface pattern advancing a
quarter tile along the product path. Direction is a **solid raised yellow
arrow** on the rubber, one per tile — a thin chevron was built first and
the scene render showed it collapsing into an `L` under the isometric
camera, which is now UI_UX.md §2b point 23. A corner's lane is a quarter
annulus whose rails meet a straight's voxel for voxel, and its arrow bends
with the belt. The belt-side junction variants (`belt_inlet`/`belt_outlet`)
are retired — the plain tile carries the junction and the machine's hood
does the rest — and survive only as aliases of frame 0 until ticket 5 wires
the framed models in. The splitter's stubs lost their old cleats to match.
`cargo test -p voxgen` grew to 15 tests; nothing under `godot_project/`
changed.
### 2026-09-09 — Pipes & Conveyors Overhaul, ticket 3: the pipe model family

Assets only (`assets/voxels/`), per `docs/PIPES_AND_CONVEYORS_PLAN.md`;
ticket 6 wires the new models into `main.gd`. Started by rendering the
current pipe runs and listing what was actually wrong, then fixed each:

1. **The liquid bled through the closed pipe as a coloured stripe** along
   the top of every tile. The fluid slab was as wide as the bore, so its
   side faces coincided with the tube's outer wall where the wall curves in
   toward the crown. Fixed: the body is two voxels narrower each side
   (`PIPE_FLUID_Y`), and a voxgen test pins every body inside its bore.
2. **The axis was half a voxel off the tile midline** (centred on voxel 16),
   so tiles at mixed facings stepped by a voxel at every joint and no
   corner could have met both neighbours. Fixed: every shape is built on
   `PIPE_C` = 15.5 with new fractional-centre primitives in `vox.rs`; the
   profile is rounder for it (a two-voxel flat instead of three nine-wide
   rows, which is what made the tube read as a square beam).
3. **A run's free end was an open bore with the floor visible through it**
   (the demo's south run, both ends). Fixed by `pipe_end`; the demo still
   shows it until ticket 6 picks that shape from `links`.
4. **`pipe_inlet`'s blind flange capped the +X end whatever the run did**,
   and its branch pointed south whatever the machine did — in the fluids
   fixture (a west-facing run) the branch stuck out over bare floor while
   the mixer's stub met the tube's flank. The shape family replaces it
   (corner or tee + `pipe_fitting_machine`); the deprecated tile is kept
   generating, composed from the new parts, so the scene still renders.
5. **The cap read as a hole.** Its brass disc faced the camera on the
   shadow side and rendered near-black — a cap indistinguishable from the
   open mouth it closed. Now a bright plate with brass bolt heads.
6. **A plain tube end butting a machine's radius-4 flange showed its own
   cut face** around the smaller ring. `pipe_fitting_machine` puts a
   radius-6 brass flange on the pipe side that swallows the machine's.
7. **No corner, tee or cross existed**, so neither the one-flange-per-tile
   rule nor the sight window had anywhere to go on a turned run.

Shipped: `pipe`, `pipe_corner`, `pipe_tee`, `pipe_cross`, `pipe_end`;
`pipe_fitting_machine`, `pipe_fitting_tank` (authored on +X, overlaid by
Godot on any open leg); one fluid body per shape. `PIPE_BORE_FLOOR_VOXELS`
(12) and `PIPE_BORE_HEIGHT_VOXELS` (7) in `main.gd` are unchanged; the end
body stops short of its cap, so `FLUID_END_CAP_TRIM` can go in ticket 6.
Face convention, stated once in the pipe section header: vox low-Y is
north. Verified with `cargo test --workspace`, `verify-core-loop.sh`, the
model previews and closeups of every pipe run in the chain, fluids and demo
fixtures before and after.
### 2026-09-09 — Pipes & conveyors overhaul, ticket 4: a thousand belts is one draw call

Every belt, splitter and pipe tile used to be its own `Node3D` with a
`MeshInstance3D` per material class, and every item on a belt a pooled
`Node3D` moved each tick. Now every tile drawing one asset is an instance
in one `MultiMesh` per material class (the way the floor was already drawn),
and items live in a swap-remove pool per item asset. Machines, buildings,
level bodies, the splitter arrow and the ghost stay nodes: they carry live
per-instance state (a lamp swap, a scaled fill, a turning arrow) that a
`MultiMesh` instance cannot hold. Nothing observable changed — every asset
choice, hover, ghost, removal, per-tile item hiding and the fluid bodies are
as they were, and `scripts/screenshot-scene.sh --populate` before and after
is pixel-identical on the world (the only differing pixels were one
simulation tick's worth of item travel, which two runs of the OLD renderer
also show against each other).

Measured on the new 1500-belt stress fixture (`test_layouts.gd`'s
`build_stress_line`, which funds itself through `save_ron`/`load_ron` since
its bill is past the starting balance):

| | before | after |
|---|---|---|
| rendered objects in frame (`--stress`) | 7,772 | 910 |
| draw calls in frame (`--stress`) | 204 | 174 |
| headless `_rebuild_world`, warm | 57 ms | 38 ms |
| headless 120 frames of `_process`, running | 867 ms (7.2 ms/frame) | 862 ms (7.2 ms/frame) |

The per-frame figure did not move because it is not the scene's: profiling
the stress line puts ~8 ms of each tick step into parsing the ~80 KB of
chunk-snapshot JSON two dirty chunks produce, ~1 ms into the sim tick, and
~2.4 ms into the scene's own belt loop. That is the wire's per-chunk delta
granularity (ARCHITECTURE.md §4) reporting every belt in a chunk every tick,
and it is milestone 8's problem, flagged here rather than worked around.
Primitives in frame rose ~15% on the stress shot because a `MultiMesh` is
culled as one whole; at this camera nothing is off-screen anyway.

Seams for tickets 5 and 6, documented in `main.gd`'s "Instanced tiles and
items" section: a tile's asset is a FRAME SET with `_set_tile_frame` flipping
per-frame visibility; a tile's yaw is handed in by the caller, not derived
from its direction; an item's position comes from one function of (tile,
direction, distance). Tests read the scene through `_tile_instance_at`,
`_carried_item` and `_carried_item_count` instead of node names.
### 2026-09-09 — Pipes & conveyors overhaul, ticket 1: the sim says what is joined to what

The first ticket of the overhaul planned in `docs/PIPES_AND_CONVEYORS_PLAN.md`.
Every belt, splitter and pipe in `world_layout_json` (and in
`query_tile_json`, for hover) now carries a `links` object: which sides
feed a belt and what kind of thing stands there, which side it hands to,
or which sides a pipe is joined on and whether that is another pipe, a
machine's tap or a tank. Directions are absolute, named from the tile
outward, and listed north/east/south/west so the shape is canonical. It is
recorded inside `World::recompute_topology` at the moment each hand-off,
tap and pipe adjacency is resolved, so it is exactly what the tick uses,
never a second scan, and free per tick; every other kind's JSON is
byte-identical to before. Tickets 5 and 6 read this to pick a straight, a
corner, a merge, an end, a tee, a cross or a port fitting.

Topology gaps filled to make it exact: the machine input tap now keeps
the direction and tile it was found through (it discarded both), the
output tap and the tank discharge now keep the pipe tile (they kept only
network and segment), and pipe-to-pipe adjacency is recorded from both
ends as it is found. `sim_core/tests/links.rs` covers every shape the plan
names, links after `Remove`/`Rotate`, a mirror-symmetry invariant over the
whole reference chain, and the JSON byte for byte.

**Found, not fixed (for ticket 7):** the sim links a belt into a machine
or dock when the tile it points at is on the inlet *edge*, from any side —
so a 1x1 crusher is fed from all four sides, including through its outlet
face, against `world.rs`'s own doc. `test_layouts.gd`'s demo line (a
west-facing crusher fed from the south) relies on it, and fixing it would
change what placed lines do, so the wire reports the rule as it stands
and the links test pins it.

### 2026-09-09 — The always-loaded docs were cut to a third of their size

`CLAUDE.md`, `ARCHITECTURE.md` and `UI_UX.md` had all grown past the 40k-char
per-file budget (160k, 52k and 52k). Every decision record that lived in
`CLAUDE.md`'s log, the long form of `UI_UX.md` §2b and `ARCHITECTURE.md`
§9a moved verbatim into a new `docs/DECISIONS.md`, which is not
`@`-included. What stays loaded is the **rule** each record produced, in
condensed form, under the same section and point numbers code comments
cite. The three files are now 28k, 19.5k and 18k. New decisions: condensed
rule in the loaded doc, full reasoning in DECISIONS.md.

### 2026-09-09 — A hopper or a tank can keep itself topped up

The sim half of moving hopper/tank configuration off the keyboard and onto
the buildings themselves. Until now keeping a factory fed was a key you held
down: **SPACE** filled every hopper in the world and **V** every tank, so an
unattended line ran for one hopper-load and stopped. A placed hopper or tank
can now be switched to **automatic**, after which the simulation tops it up
to capacity out of the player's bought stock every tick, before anything else
in the tick consumes anything — so a refilled bin feeds its belt in the same
tick rather than stuttering.

**It spends stock, never money.** Buying ingredients stays an explicit
purchase (GAMEPLAY.md §5's recurring ingredient cost is one of the three
things the player is deciding about, and a toggle that quietly drained the
balance would turn that decision into a background leak). An empty larder
just stops the top-ups — no error, and the flag is not flipped off, so they
resume by themselves the moment stock is bought again.

Three new tile-addressed commands go with it: the two toggles, plus a
`set_hopper_item` that names an ingredient outright, which is what the item
picker in a placed hopper's panel wants (the **Q**-key `cycle_hopper_item` is
kept only until the presentation ticket removes that key). The flags ride on
the snapshots presentation already reads and in the save file, and the whole
thing is pinned by a new `sim_core/tests/auto_fill.rs` plus a save round-trip
that keeps topping up after the load. The panel that fronts all of this is
the following ticket.

### 2026-09-09 — The game opens on a menu, and there is a way out of it

The other half of save/load. Until this the project booted straight into a
factory and the only way out of a session was the window's close button —
so the save format that landed the same day had nothing to reach it.

There is a **main menu** now (`menu.tscn`, and it is what `project.godot`
opens on): New Game, Load Game, Quit, drawn in the game's own dark-plate and
hazard-yellow language rather than in a stock engine theme, because it is the
screen that sets the expectation for everything after it. Load Game lists
what is actually in `user://saves/`, newest first, each row naming its slot
and when it was written. Choosing one **proves it loads before the scene
changes** — the file goes through a throwaway `SimBridge` first — so a
corrupt or older-build save is refused on the one screen still able to show
`load_ron`'s own reason, with the list still up, rather than after the
previous world has already been thrown away.

In the game, **ESC now opens a pause menu when nothing else is open**, which
is one escalating rule rather than a new key: ESC backs out of the thing you
are in, and when you are in nothing, it backs out of the game. It carries
Resume, Save into a named slot, Save & Quit to Menu, Quit to Menu (which
asks first, since it is the one button that throws work away) and Quit Game
— and the factory genuinely stops while it is up, including the tick
accumulator, so reading the panel for five minutes costs nothing and gains
nothing on resume.

**Autosave** writes `user://saves/autosave.ron` every three minutes and on
every way out, including the window's close button. It owns that slot in
both directions: a timer can never overwrite a save the player made, and a
manual save is refused the name `autosave` out loud rather than quietly
renamed.

Two new headless scripts (`test_main_menu.gd`, `test_pause_menu.gd`) cover
all of it — including a genuinely corrupt file and the three-minute timer,
neither of which needs three minutes to test — and the real windowed scene
chain (boot → menu → New Game / a chosen save → gameplay → ESC) was driven
end to end with a framebuffer, because `change_scene_to_file` is the one
thing a headless test cannot exercise.

### 2026-09-09 — A factory can be saved, and comes back still running

Until this ticket nothing a player did survived closing the window: the
scene built a brand-new empty world on every launch. There is a save format
now — a full state snapshot in RON, with its own `schema_version`, rejected
rather than migrated on a mismatch like every `data/*.ron` loader — plus
`save_ron()` / `load_ron()` / `new_game()` on the `SimBridge` boundary.

The bar it is built to is that a saved-and-reloaded factory is
**running-state-identical**, not layout-identical: a machine mid-craft does
not restart its batch, an item part way across a belt tile comes back at the
same fraction of it, every pipe tile and tank holds the exact volume it did,
and every contract keeps counting on its own clock. That is proven by
running the saved world and the loaded world side by side for longer than
the longest contract cadence and comparing every snapshot the boundary
exposes at every single tick.

Two design calls worth knowing about. **ARCHITECTURE.md §4's "command log
plus periodic snapshots" save format is amended**, because the two things it
was meant to share a mechanism with — undo/redo and lockstep multiplayer —
do not exist, and a command log is the more fragile of the two anyway (it
has to be replayed through whatever game logic the current build has, so a
change to what a command does silently rewrites every old save). And **file
I/O stays out of `sim_core`**: Rust produces and consumes the RON string,
Godot decides what file it becomes, because `user://` is Godot's.

The main menu, save slots, autosave and an in-game way out are ticket 2.

### 2026-09-09 — A tuning pass: a slower crusher, a dock that reads as a dock, and a UI you can read

Three directly-requested changes after a positive playtest, all of them
things you only find by playing.

**The crusher takes 80 ticks instead of 60.** At 60 the crush read as
instant beside the mixer's 90 and the vat's 240 — a stage you never see
working. 80 keeps it the quickest of the three front-end stages and keeps
it out-producing the mixer it feeds, so the line's bottleneck stays at the
vat. The ordering of those three times is stated in words in three
different files, so the rule now lives in `data/recipes.ron`'s header:
change the numbers and the words together or not at all.

**The delivery dock is 2x2.** It did not read as a loading dock at one
tile, and it could not: the raised deck, the striped edge, the bumpers, the
despatch door under its canopy and the apron for goods waiting to go out
were all competing for one 32-voxel tile — which is also why the stack of
cases on it had to be capped at two. The extra tile is spent on depth
rather than on making the old design bigger, because on this camera
anything tall has to stand on the far row: the far tile is the building
now, the near tile is open apron with room for four cases. Two real
consequences beyond how it looks — its inlet face is two tiles long, so
two lines can deliver into one dock, and a square footprint claims the same
tiles whichever way it is turned, which is what you want of the building at
the end of a line. Its price deliberately did not move with it.

**The UI is half again as big.** The report was that it is "much" too small
and "difficult to read", and it was — the body font had been 15px since
milestone 5 and a couple of dozen pixel literals had grown up around it.
There are two numbers now, `UI_SCALE` and `UI_FONT_BASE`, and everything
else is derived from them; explicit constants rather than the engine's own
scale knob, because this scene maps 3D to screen coordinates in half a
dozen places that would all then need the factor taken back out. Half again
does not fit a 1280x720 window unchanged, so the build menu became six
columns of two shorter cards, all three panels put their heading and
balance on one line, the control legend became four grouped lines, and the
clipboard and the build grid now size themselves against the space actually
left above the hotbar rather than against a guessed fraction of the window.
At 1600x900 nothing scrolls; at 720 the build grid gives up a sliver, which
is much better than a menu whose footer hides under the hotbar — and the
project's default window is 1600x900 now (it was 1280x720), so the default
is the size at which nothing has to.

### 2026-09-09 — SPACE and V say why, instead of doing nothing

More real playtesting fallout: with a hopper's item choice and raw-ingredient
purchasing both landed, pressing SPACE (load hoppers) or V (load tanks) with
no stock bought — or no hopper/tank placed at all — did *exactly nothing*,
silently. That is `issue_command`'s correct contract for the simulation (an
impossible command changes nothing), but a player watching a keypress do
nothing has no way to tell "you own no sugar" from "there is no hopper" from
"it's already full," and reported it as "how am I meant to load the hopper
for sugar?" — a controls problem, not a missing-feature one.

`_hopper_fill_blocker`/`_tank_fill_blocker` (`main.gd`) check, BEFORE the
command ever reaches the sim, whether anything is actually placed and
whether at least one raw item it needs is in stock — and if not, name the
exact missing thing in the same flyout label the hotbar already uses ("No
hopper placed — buy one in the build menu (E)" / "No tomato, sugar in stock
— buy some at the Ingredient Market (M)"). A tank not yet feeding any pipe
run is deliberately not blocked on stock at all — that is a plumbing gap,
not a stock one, and this message does not try to tell the two apart.
Already-full is left silent on purpose too: the fill bar sitting at max is
its own answer, unlike the other two cases, which have nothing visible to
look at.

New `test_fill_feedback.gd` (marker `FILL_FEEDBACK_OK`) proves both messages
appear at the right moments, both clear the instant real stock is bought
(not just that the strings exist), and a real fill actually moves stock once
unblocked — full `scripts/verify-core-loop.sh` green.

### 2026-09-08 — A placed hopper can change its mind

A real gap surfaced by actually playing the demo after the chain-rework epic
landed: a hopper's contents are chosen at placement time (Q, while the ghost
follows the cursor), but nothing let a player change an *already-placed*
hopper's item afterward — a tomato hopper stayed a tomato hopper forever, with
no discoverable path to a sugar one short of picking it up and re-placing
while remembering to press Q during that one brief window.

Fixed with a new `PlayerCommand::CycleHopperItem { tile }`, deliberately
shaped like `Rotate` rather than `Place`'s explicit-item form: no target to
name, just "advance to the next raw item, wrapping around" — the set of legal
values is small, fixed, and the simulation's own (`World::raw_items`), so the
player doesn't need to name one. [`Hopper::retarget`] clears whatever the
hopper held on a switch, the same honest "what it holds is lost" rule
`Remove` already has — there's no half-tomato-half-sugar state to preserve.
On the Godot side, **Q now works exactly the way R already does**: it cycles
the ghost's choice while placing, and — outside placement mode — cycles
whatever hopper is under the cursor, in place, no pickup required. The hover
tooltip gained a "Q: change contents" line, shown only when there's more than
one raw item to change it to.

`sim_core/tests/hopper.rs` gained five new tests (cycling switches the item
and clears contents; a full lap through every raw item returns to the start;
an empty or non-hopper tile refuses quietly; a single-raw-item world is a
true no-op that keeps its contents; the wire form parses) — 205 tests green.
Full `scripts/verify-core-loop.sh` green.

### 2026-09-08 — The chain rework, ticket 2 of 2: the piped middle, drawn

The presentation half of Epic A. Ticket 1 changed what the factory *is* —
the aging rack deleted, the middle of the line piped instead of belted, sugar
added as a second raw ingredient (making the mixer 1x2), and raw ingredients
made something you buy — and deliberately left `godot_project/` and
`assets/voxels/` untouched. That left `main` in a state worth recording: its
own headless suite was red, and, less visibly, **a new game could not be
played at all**. Placing a hopper from the UI was a silent no-op, because
`_confirm_build` sent no `item` field and a world with two raw solids refuses
a hopper that does not say which one it holds.

Fixing that properly was the ticket's real work, and it is an interaction
design rather than a rendering one: the hopper keeps one catalog slot, one
price and one owned count, and *what it will hold* is a property of the
placement, cycled with **Q** while the ghost is up. Splitting it into
"Hopper — Tomato" and "Hopper — Sugar" catalog entries was the obvious
alternative and would have renumbered every hotbar slot after the hopper the
day a third ingredient ships — the exact property fixed slots exist to
protect. The choices come from the simulation's own `raw_items`, newly on the
wire, so a third ingredient needs no Godot edit.

Buying stock got its own panel on **M** rather than a section of the build
menu, which was already as tall as the space above the hotbar allows, plus a
permanent larder strip above the hotbar — because `SPACE` and `V` now draw
from bought stock and do nothing without it, and a control that silently does
nothing needs the number that explains why on screen.

On the art side the mixer was rebuilt as a 32x64 horizontal ribbon blender
(the footprint doubled, and a round vessel centred in a 1x2 plinth would have
put solid shell across both feed lanes), and the mixer, the vat and the
bottler grew pipe nozzles on the faces a belt no longer meets. The vat's
charging gallery and discharge doorway are gone: a covered belt-width mouth
with no conveyor in front of it is a fitting describing a mechanism the
machine does not have. Sugar got an item model of its own — without one it
would have travelled the belt as a cardboard box through `ITEM_MODELS`'s
fallback, with every structural test still passing, which is UI_UX.md §2b
point 12's lesson for the third time.

Verified with the full headless suite green plus rendered frames of the
rebuilt chain running to a contract payout, the mixer's two feed junctions,
both pipe runs meeting their machines, the hopper-contents ghost in use, and
the market panel. `test_timed_processes.gd` was deleted with the rack and
replaced by `test_chain_rework.gd`.

### 2026-09-08 — A performance/code-quality audit, run alongside the fluid-pipes rework

A dedicated audit (not milestone work) hunting for real, measured waste in
today's small-factory scale — explicitly not milestone 8 (Scale-up) territory.
Two genuine, low-risk fixes landed; several other candidates were measured and
deliberately left alone, which is itself the useful result in those cases.

**Fixed:** `sim_core/tests/shop.rs`'s `world_with_earned_money` fixture ran
the *whole* reference chain (aging rack and all) until the bank held 200 —
24,000 ticks, redundantly, once per test in that file (7 of them, ~168,000
ticks total), and it was measurably the most expensive part of
`cargo test -p sim_core`'s debug-mode run. 200 was never load-bearing (the
hungriest test in the file only ever needed 36); dropping the threshold to 50
keeps a real margin over that while cutting the file's own runtime by more
than half (serial: 6.48s -> 2.88s). Also converted three `debug_assert!`s in
`assets/voxels/generators/assets.rs` (gauge/tank-fill/hopper-fill geometry
bounds) that clippy flagged as compile-time-constant into real `const { assert!
(..) }`s — same check, but it now fails the build instead of only a debug
test run if a future geometry tweak breaks the invariant.

**Measured and left alone, on purpose:**
`sim_core/src/sim/world.rs`'s `tick()`/`recompute_topology` already match
ARCHITECTURE.md §3.8's "recompute on topology change, not per tick" — no
change needed. `godot_project/main.gd`'s `_rebuild_world` (flagged in the
"Real Placement" `worker_done` report as a possible concern) fires only on a
discrete Place/Remove/Rotate action, never per tick or per frame, which
matches the project's own already-recorded decision that a full rebuild is
the right call at this scale — confirmed by reading the call sites, not
re-litigated. `sim_core/tests/fluids.rs` has a real version of the same
redundant-work pattern the shop.rs fix addressed — two test helpers
(`count_item_anywhere`, `mixer_is_starved`) each rebuild a full multi-chunk
snapshot on *every tick* of a ~5,000–10,000-tick loop, and that one test
alone is ~1.7s of the suite — but `tests/fluids.rs` sits directly next to
`fluid.rs`, which the concurrent mixer/cooking-vat/bottler fluid-pipes rework
is actively rewriting, so fixing it now would almost certainly be thrown away
or conflict; left as a flagged finding for whoever picks up that file next
instead of fixed. `cargo clippy -p sim_core` came back otherwise clean
(a couple of cosmetic `collapsible_if`/`needless_borrow` nits, mostly in
files the same rework already owns) — not fixed, for the same reason.

### 2026-09-08 — The chain rework, ticket 1: piped through the middle, and ingredients you have to buy (sim half)

A directly-requested content-and-mechanics pass, and the largest change to
`data/recipes.ron`'s shape since it was written. Four things at once, because
each needs the others to make sense. `godot_project/` and `assets/voxels/` are
deliberately untouched — a follow-up presentation ticket owns those.

**The aging rack is gone.** Milestone 7 inserted a 2400-tick `aged_ketchup`
stage to prove the generic `TimedProcess` at a qualitatively different
timescale; the project owner has decided ageing isn't a mechanic this game
wants, so the stage, its machine kind, its price and its dedicated test file
are deleted rather than disabled. `TimedProcess` itself is untouched and still
runs every craft cycle in the game.

**The middle of the line is piped, not belted.** Until now a machine could
*consume* a fluid and nothing could produce one, so every stage-to-stage link
had to be a conveyor even where a real plant uses pipe. `Recipe` gained
`fluid_outputs` (mirroring `fluid_inputs`), `FluidNetwork` gained the
producer-side twin of its draw API, and `Machine::tick` now takes two taps —
one to draw from, one to deposit into. The shipped chain uses it: the mixer's
ketchup mix and the cooking vat's finished ketchup are both **liquids**, so
the vat has no belt touching it at all, and the bottler is where the product
becomes a solid again. `recipes.ron`'s `schema_version` is 3.

The rule that makes a machine plumbed on both sides unambiguous is geometric,
and worth knowing: **a fluid output leaves by the face the machine points at,
exactly as a solid one does; a fluid input may come from any other pipe
touching the footprint.** So the player never has to label a run, and a loop
of pipe wrapped around one machine reads as its outlet rather than as a
machine quietly feeding itself.

**Sugar, and a rule about inlet faces.** The mixer's recipe gained sugar, the
chain's second raw ingredient — which meant two hoppers, which meant `Place`
had to start asking *what* a hopper holds (optional while there is one raw
ingredient to infer, required once there are two). The unanticipated finding
was geometric: a 1x1 machine has **one** inlet tile, so two solid ingredients
would have to merge onto one belt, and a merge with no splitter behind it
deadlocks — whichever ingredient arrives faster fills the shared belt and
starves the other out. That was built and measured, not theorised. The mixer
is **1x2** now, giving sugar its own lane into its own inlet tile, and
`data/machines.ron` records the general rule for the next multi-ingredient
kind: a machine that consumes N solid ingredients needs an inlet face at least
N tiles long.

**Ingredients cost money.** GAMEPLAY.md §5 has named ingredients as one of
three recurring cost axes since it was written and it had never been
implemented — `FillHopper` and `FillFluidTank` conjured material out of
nothing. There is now a `data/ingredients.ron` catalog and an owned-stock
ledger (`sim/ingredients.rs`, `shop.rs`'s shape with one word changed), a
`buy_ingredient` command, and both fill commands draw from that stock and are
silent no-ops without it. One consequence is a real design change rather than
a mechanical one: **overproduction is now a loss**, since a box no contract
has ordered is ingredients converted into unsold stock — which is exactly the
pressure GAMEPLAY.md §6 chooses contracts over a spot market to create.

`cargo test -p sim_core` is green at 228 tests across 11 binaries, including a
rebuilt full-chain mass-conservation check that now accounts for product
standing in pipes as well as on belts, and runs once per raw ingredient
rather than once in "tomato equivalents".

### 2026-09-08 — Factorio's controls: E opens the build menu, ESC closes everything

Two small control changes, both about matching what a player arriving from
the genre already has in their fingers. The build menu moved off **B** onto
**E** — Factorio's own inventory key — and B was removed rather than kept as
an alias, so the legend prints the one key that works. And **ESC** now closes
*everything* that is open: it used to leave placement/remove mode only, so
with the build menu up the one key a player reaches for to dismiss things
left the biggest panel on screen standing. That was a real gap, not a
preference; `_close_all_overlays` is the ESC-only wider verb, deliberately
separate from `_exit_build_mode` because the hotbar's deselect and the
"you placed your last one" exit must *not* sweep the menu away with them.

### 2026-09-08 — The fork, drawn: the splitter's diverter table and its lit arrow

Epic B's ticket 2, and the presentation half of the entry below. `sim_core`
and `data/` were untouched — every fact drawn was already on the wire — and
the ticket's minimum bar was that `main` had been left failing its own
headless suite: `test_machines.gd` asserts every priced kind resolves to a
bespoke model, and the splitter had none.

It is drawn as the one piece of **machined table** in a run of rubber: a
stainless diverter plate with three open lanes and one closed rail, so the
tile reads as "the line forks here" from across the factory without reading
as another machine. The state that needed showing is which output takes the
next item, and that is an **arrow inlaid flush in the deck** — both
directions cut as grooves, one of them lit — rather than a gate or a signal
arm, because items are drawn sliding across this tile and anything standing
proud of it gets driven through. Two grooves and one glow means the readout
is a pair of *positions*, which survives a screenshot and a player who cannot
tell amber from steel.

The arrow points where the item will **actually** go rather than at the raw
`next_output` field, and that distinction is the ticket's one real design
call: a splitter whose branch was never built reports "branch" for ever while
everything goes straight past it, so an arrow driven off that field alone
would sit pointing at bare floor — a readout lying about its own mechanism,
which this project has now refused three times.

Two bugs came out of it, both found by looking at a rendered frame rather
than by a test. Items on a belt in a **second chunk** were being erased by the
first chunk's render, which a splitter branching a line across a chunk
boundary is what finally exposed: the simulation held eight tomatoes on the
branch belt and the scene drew none of them. And
`--populate`, the flag that runs a factory forward before photographing it,
was batching thousands of ticks without reading the dirty set the simulation
clears every tick, so it had been photographing whatever the last tick
happened to touch. Both are fixed, and the first is now a regression test.

### 2026-09-08 — A line can fork: the conveyor splitter (sim half)

Epic B's ticket 1. Until now every hand-off in the simulation was
one-to-one — a belt looks at the single tile it points at — so a production
line could turn corners but never branch. A **splitter** is the first
placement that resolves to two downstream targets: a 1x1 buildable kind
(`data/machines.ron`, priced at 25 in `data/shop.ron`) that carries product
like a belt tile and deals it alternately between the tile it faces and the
tile one quarter turn clockwise from it. One `rotate` turns both outputs
together, because the second output is derived from the placement's single
`direction` rather than stored — so no new command and no second facing that
could drift out of step with the first.

The distribution rule is Factorio's: strict alternation while both sides can
take, falling back to the other side in the same tick when one is blocked, so
a full branch never stalls the whole line. Both degenerate wirings fall out
of that loop rather than being special-cased — one output connected feeds
that one, no outputs connected holds everything and backs the line up. What
a splitter is *holding* lives in an ordinary one-tile belt segment, which is
what makes an item cross it at belt speed and lets every existing transfer
function feed it unchanged.

Sim and data only: `godot_project/` and `assets/voxels/` were deliberately
untouched, and the splitter's wire form (`chunk_snapshot`'s new `splitters`
array, `query_tile`'s new variant) is on the boundary waiting for ticket 2 to
draw it. One thing flagged in passing: the shop catalog has outgrown the
hotbar's ten number keys, so the last two kinds are now keyless — a
presentation problem for that ticket, not a simulation one. See CLAUDE.md's
decisions log for the five design calls.

### 2026-09-08 — A real hotbar, and a starting balance the loop can close from

Two directly-requested changes in one ticket, bundled because the second
breaks the tests the first has to touch.

**The hotbar.** UI_UX.md §5 has asked for "a Minecraft-style hotbar/quick-access
strip along one screen edge" since it was written, and it was the last named
widget in that section still unbuilt. Until now **nothing** in the game was
reachable without opening the full build menu first — no glanceable view of
what you were carrying, and a modal grid to navigate every time you wanted to
place something. There is now a strip along the bottom edge, one fixed slot
per `data/shop.ron` kind, driven from the same `shop_snapshot_json` the menu
uses. Number keys 1-9 and 0 select a slot and enter placement mode directly,
in either view mode, menu open or closed; pressing the selected slot's key
again puts it down. Buying stays a different verb in a different place — the
build menu (**B**), on a slot's left click or SHIFT+number — because a key
that spends money with no prices on screen is exactly the accident worth
designing out. Fixed slots rather than a packed list of what is owned, so the
keys stay muscle memory; a kind you own none of is drawn as an empty socket
rather than a disabled button. Selection is signalled four ways at once and
the strip animates (a scale pop on select, a pulse on click, and Minecraft's
name flyout above the bar). Two problems only the rendered frame could show
got fixed on the way: the strip's plate was translucent enough that the
floor's tiling showed through and made the row noisy, and the build menu grew
tall enough to bury its own footer — the one line explaining how to buy —
under the new strip.

**The balance.** `STARTING_MONEY` went from 200 to **3000**. The "Real
Placement" epic had flagged the hole and left it for the balancing pass:
money enters only through contract deliveries, a delivery needs a line that
reaches the dock, and that line costs 1087 — so at 200 a new save could not be
played forward to a first payout at all. Not a slow game, a dead one. 3000 is
deliberately more than 2.5x the bill, so a first factory can be built wrong
and rebuilt. Every test that argued from the old insufficiency was reworked
rather than patched, and coverage that the old balance had *deleted* came
back: a Godot fixture that builds the entire shipped six-stage line out of
ordinary purchase/place commands, and the `--contracts` screenshot flag on top
of it.

### 2026-09-08 — You build the factory now (the presentation half)

The world has been empty since this morning's sim ticket; today it is empty
*on screen*, and the player is the one who fills it. This is the largest
presentation change the project has had, and together with that ticket it
completes the shop/inventory/building pivot.

The scene used to learn its shape once, from `chain_layout_json`, because a
factory arrived pre-built and nothing could ever move it. It now re-reads
`world_layout_json` and rebuilds its whole "World" subtree after every place,
remove and rotate — a full rebuild on a player action, never per tick, which
is deliberately the same trade the simulation makes when it recomputes its
adjacency graph rather than maintaining an incremental one. Everything drawn
carries the facing the player gave it: a belt turned north is drawn turned
north, a machine facing west discharges to the west, and a rotated footprint
is re-centred on the rectangle it actually claims.

**Build mode.** An owned slot in the build menu is right-clicked (or
shift+number) to enter placement mode; left-click still buys, so the two
actions are two buttons rather than one that changes meaning with the owned
count. A translucent tinted ghost of the real voxel model then follows the
tile under the cursor — the same ray/plane pick the hover tooltip already
used — and says whether the click will work, in a glyph and a word as well as
a colour: **✔ PLACE** on free ground, **✗ BLOCKED** over anything already
placed, **✗ NONE OWNED** when the inventory runs out. Validity is answered
entirely on the Godot side, footprint-aware, from the occupancy map the
layout already carries, so aiming costs no simulation round trip. **R**
turns the ghost, and — outside placement mode — turns whatever is under the
cursor where it stands. **X** toggles remove mode, which ghosts the target in
red and picks it up on click. **ESC** leaves either.

**The junction art is deliberately not fixed, and now it is checked.** The
belt's inlet/outlet variants are authored for a camera that can only see two
faces, so they are east-only; three of the four facings a player can now
choose fall back to the plain tile rather than draw a doorway pointing away
from the camera. A four-directional art pass is its own ticket. What this one
adds is that the fallback is asserted rather than assumed, and a rendered
frame confirms nothing floats or clips at a turned junction.

**One thing this ticket could not do, and it is worth knowing.** A new game
has 200 in the bank and the shipped six-stage line costs about 1,087, so
nothing on the Godot side — tests or screenshots — can reach that factory any
more; it stays asserted in `cargo test` against `sim::reference_chain`, which
starts its world with exactly the money to pay for it. Every headless Godot
test now builds what 200 buys, through real purchase and place commands,
which is a better test of the boundary and a smaller factory to look at. It
also means the game as it stands cannot be *played* forward from a new save:
200 buys a hopper, a few belts and one machine, and the first sellable
product is six machines away. That is a balance question for whoever picks up
the economy next, not a rendering one.

### 2026-09-08 — The delivery dock you can actually see (presentation half)

The dock existed in simulation but not on screen: the tile past the packer
rendered as bare floor, the belt feeding it was not drawn at all, and the
control legend still offered a **C: collect product** key for a command that
no longer exists. All three are fixed, and `sim_core`/`data/` were not
touched — every fact drawn here was already on the wire.

The dock is deliberately the one building on the line that is a *place*
rather than a machine: a poured platform at trailer-bed height, its edge
hazard-striped, with rubber bumpers, a despatch door under a canopy, and the
same hooded transfer housing every other machine is fed through. That last
part matters — the packer had never had a discharge doorway, because until
this ticket it was the end of the line, so it grew one and the dock's belt
now leaves and arrives through real junctions like every other hop.

**No fill gauge, and that was the decision worth making carefully.** The dock
ships everything it receives in the same tick, so its own level is always
zero and a bar reading it would be a gauge lying about its mechanism. The
number that is real is the economy's shared delivery pool, so the deck stands
one case on it per unit waiting to be sold — drawn with the item's own model,
capped at two because a third case starts hiding the building. The exact
figure stays where it already was: the clipboard's ON THE DOCK line, and the
dock's own hover, which reports what it has shipped over the factory's whole
life alongside what is standing on it now. **This completes the delivery-dock
work.**

### 2026-09-08 — The world starts empty: real placement, rotation and pickup (sim half)

This is the largest single change the project has had, and the one that
turns it from a tech demo of a factory into a game about building one.

Until today `World::new` *built a factory*. It read `data/recipes.ron`'s list
order, laid one belt-and-machine stage per recipe west to east, stood a
hopper at the head, a vinegar tank beside the pipe run and a dock at the
tail, and wired every stage to the next by index at construction time. It
all existed unconditionally from tick 0, in exactly one arrangement, facing
exactly one way. The player could buy equipment — the "Shop & Inventory"
epic built that ledger — and nothing anywhere read the result.

Now `World::new` builds nothing at all: an empty grid and 200 in the bank.
Everything in a factory is something a `purchase` paid for and a `place` put
down, `remove` picks it back up into inventory, and `rotate` turns it 90°
clockwise. GAMEPLAY.md §4's "equipment purchases *are* the tech tree" is a
statement about the simulation for the first time.

Three consequences are worth knowing beyond the headline. **Flow is derived
from adjacency**, not from a chain: a belt carries toward the way it faces
and hands off to whatever is standing there, a machine accepts on the face
opposite the way it points and discharges on the face it points at, and the
whole graph is recomputed only when something is placed, removed or rotated
— ARCHITECTURE.md §3.8's "once per topology change, not every tick" rule,
made real for the first time. A line can therefore turn corners, which the
old fixed layout could not express and which `world.rs`'s module doc had
been carrying a long "deliberately out of scope" note about; that note is
now a description of what shipped. **The hopper, the tank and the dock are
ordinary placeable kinds**, so a world may hold none, one, or several of
each — not because anyone asked for two docks, but because nothing in the
simulation ever assumed one and inventing that restriction would have been
new code enforcing a rule no design doc states. And **the shipped six-stage
line still exists**, as `sim::reference_chain`: a fixture that rebuilds it
tile-for-tile out of nothing but purchase and placement commands, which is
simultaneously what every existing integration test now builds its world
from (so mass conservation, contract payouts, fluid consumption and
footprint geometry are all still asserted against the same factory) and the
proof that placement can reach every configuration the game already
guaranteed.

The wire format changed a lot, and ticket 2 (the Godot half — ghost preview,
place/rotate/remove input, and drawing per-tile rotation) is where that
lands: `chain_layout_json` is replaced by `world_layout_json`, the hopper /
tank / dock / fluid snapshots return arrays instead of single objects, a
belt snapshot is one tile rather than a run, and everything placed carries a
`direction` string. `godot_project/` is deliberately untouched here, so the
scene will need that ticket before it runs again.

### 2026-09-08 — The chain has an end: a real delivery dock (sim half)

Until now the factory's product left the world on a keypress. `C` ran
`PlayerCommand::CollectOutput`, which reached into the last machine's own
output buffer and dropped its contents into the economy's shared delivery
pool — there was no tile downstream of the packer at all, so finished boxes
were teleported off the floor and sold. That is exactly the shape the
"Machines improvement" epic's hopper removed at the *feed* end, and
GAMEPLAY.md §9 and `economy.rs` had both been carrying a "a real
delivery-dock tile is future polish" note ever since.

It is no longer future. `data/machines.ron` has a `dock` kind (priced in
`shop.ron` like everything else buildable), `World` lays one more ordinary
belt past the last machine and stands the dock at its end, and the dock takes
arriving boxes off that belt and deposits them into the shared pool **every
tick, by itself**. The old command is gone rather than kept beside it — two
ways to sell the same box, one of which teleports, is the contradiction the
hopper existed to remove. Feeding the factory is still the player's job;
emptying it is now the machinery's.

The one decision worth knowing before building anything on top of this: the
dock is a **pass-through, not a store**. It ships everything it receives in
the same tick, so its own buffer reads empty between ticks. That keeps
"what's on the dock" exactly one number in exactly one place — the shared
delivery pool every contract settles against — instead of two numbers where
the one the player can see is the one contracts cannot use. A visible hold
becomes worth having the day something gates the release (a truck on a
cadence, GAMEPLAY.md §7's logistics staff); a hold with nothing gating it is
latency, not a mechanic.

Two consequences. First, this is a **breaking wire-format change**:
`{"type": "collect_output"}` no longer parses (it is a silent no-op, like any
malformed command, so nothing crashes), and there is a new
`dock_snapshot_json` plus a `docks` array on chunk snapshots and a `dock`
variant on tile queries. Ticket 2 (the Godot dock asset, hover, and control
legend) consumes those and has to drop the now-dead **C** keybind. Second,
the shipped line now runs x = 0..35 and therefore genuinely spans three
chunks rather than two — handled, and pinned by tests, rather than avoided.

Verified the way this project verifies things: `core_loop.rs`'s
mass-conservation invariant still holds every tick across the longer chain,
now counting the dock's buffer as mass and reading "shipped" off the dock
instead of tallying a command — and *no test issues any command to empty the
chain any more*, which is the property the ticket exists to create.

### 2026-09-08 — Contracts, ticket 2: the clipboard shows all of them

The presentation half of "contracts are data." Ticket 1 made
`economy_snapshot_json` report an array of contracts; the Godot clipboard was
still drawing exactly one of them, which meant GAMEPLAY.md §6's whole point —
several orders competing for one factory's throughput — existed in the
simulation and nowhere the player could see it.

The clipboard is now a **board of dockets**: one sheet of paper under the same
steel clip, carrying the shared money figure once, then one printed docket
card per contract below it with its own name, payout, checklist, cadence
countdown and delivered/missed tally. A clipboard *per* contract was the other
obvious option and was rejected — it repeats the frame and the clip bar for no
new information — and a single flat list of checklist rows was rejected too,
because it loses which rows belong to which order the moment two contracts
want the same item, which all three shipped contracts do. The docket is the
checkbox's own paper tone plus a steel hairline, so this adds separation
without adding a visual language. Beyond a capped height the stack scrolls
rather than growing off the bottom of the screen; three contracts is nowhere
near it, and that cap is a safety valve for a content change, not a UI anyone
is expected to drive.

One deliberate addition beyond "show them all": a small **"on the dock"** line
above the dockets, showing the shared delivery pool. It is the player-visible
face of ticket 1's allocation rule — every checklist is clamped to its own
ask, so three dockets can all read "ready" while the dock holds fewer boxes
than they collectively want, and without the pool figure that reads as three
imminent payouts instead of a competition.

`sim_core` was not touched. Verified headlessly (`test_contract_panel.gd` now
asserts one docket per reported contract in priority order, per-contract
checkbox counts and distinct nodes for the same item id across contracts, one
money readout and per-contract countdowns that diverge as the sim runs, the
dock line tracking a real delivery, and — the regression this work could
cause — that a one-contract board still renders) and visually, via a new
`scripts/screenshot-scene.sh --contracts` flag that runs the real chain until
the board is in a genuinely mixed state (one order satisfied, one part-filled,
one already missed) rather than photographing three identical empty
checklists. That review pass also caught something no headless test could:
the checkbox tick mark was body-text sized inside a 14px box and had been
rendering as a clipped diagonal in dark ink on dark green ever since the
clipboard landed.

### 2026-09-08 — Milestone 7, ticket 2: the aging rack, and a process you can watch

The presentation half of ARCHITECTURE.md §9's milestone 7, and the end of
that milestone. Ticket 1 put a 2400-tick `aging_rack` into the chain between
the cooking vat and the bottler; until now it rendered as the generic housing
with no way to tell what it was doing. It is now a rack of four oak casks on a
stainless frame — deliberately the one machine on the line with **no moving
part**, because what makes it different from its neighbours is that it only
waits — and it carries a **maturation gauge**: an emissive amber bar in a slot
on its corner post, scaled every frame to the live process fraction, climbing
toward an always-lit green finish line. `sim_core` and `data/` were not
touched; every fact the gauge draws was already on the wire.

The gauge is the ticket's real content. A machine only dirties its chunk when
its *state* changes, so across a forty-second wait no snapshot arrives at all
— the bar has to be recomputed from `current_tick()` against the machine's own
`[start_tick, required_ticks]` pair, which is what makes a long process
animate instead of jumping at the end. Two design calls came straight off the
rendered scene rather than off the generator: on this camera the gauge's face
is *permanently* in shadow, so the bar had to emit its own light rather than
be painted; and an empty slot showed no scale at all, so a lit green line
marks where "done" is. The hover tooltip grew a remaining time beside its
percentage for the same reason — "40%" of a duration the player was never told
is not an answer to "how long until this is out".

Found and fixed while getting a screenshot at all, on the branch this ticket
was built on: since contracts became data and several started running at
once, `main.gd`'s contract panel read a `next_delivery_tick` field that no
longer existed, and **the resulting per-frame GDScript error aborted
`_process` at that line** — so fluid levels, the hopper level, hover
tooltips and camera panning had all silently stopped updating in the live
scene. A narrow tolerate-both-shapes patch fixed that live-blocking bug at
the time; by the time this branch rebased onto `main`, the real fix — the
N-contract clipboard rebuild from the entry above — had already landed
there, so the narrow patch did not survive the rebase and both previously-red
scripts (`test_core_loop.gd`, `test_contract_panel.gd`) are green again.

Verified headlessly (`cargo test -p sim_core` unchanged and green,
`cargo test -p voxgen` gained a check that the gauge slot is a genuinely
carved opening rather than a painted one, and a new `test_timed_processes.gd`
drives the real chain until the rack loads and asserts the bar grows on live
ticks with no snapshot applied) and visually, through a new
`scripts/screenshot-scene.sh --aging` flag that runs the chain until a batch is
half aged — a gauge photographed at 0% being a picture of an empty slot.

**This completes ARCHITECTURE.md §9 milestone 7 (Timed processes).**

### 2026-09-08 — Shop & Inventory, ticket 2: a build menu you can actually buy from

The presentation half of the shop, and the first piece of genuine player
agency in the game — everything before it was either automatic or a single
fixed action ("load the hopper"). **B** opens a build menu over the scene:
a grid of voxel-rendered icon slots, one per `data/shop.ron` entry, each
showing the kind's icon, its price and how many the player owns. Clicking a
slot buys one; so does pressing the slot's number key, which is how it works
in first-person where the mouse would otherwise be captured. Nothing about
the menu's contents is written down in GDScript — the catalog, its order,
prices, owned counts and affordability all arrive in ticket 1's
`shop_snapshot_json`, so pricing a new kind in `data/shop.ron` puts a slot in
the menu with no Godot edit, the same way adding a recipe already extends the
chain.

Two things worth knowing. What you cannot afford is signalled three ways at
once — the slot is disabled and visibly recessed, its icon dims, and its
price gains a "✗" — because UI_UX.md §7 forbids encoding state in colour
alone. And the menu wears the scene's existing dark plate with the
hazard-yellow rule rather than the contract clipboard's paper: the clipboard
motif is for orders and checklists, a build menu is machine-side factory UI,
and there is deliberately no third visual language here.

Verified headlessly (`test_shop_menu.gd`: the menu is a distinct node, one
slot per catalog entry with the snapshot's own numbers, B toggles it, every
slot reads locked at zero money, and a purchase driven through the real
`issue_command_json` — with money genuinely earned by running the chain
against the contract — moves the displayed owned count) and visually, via a
new `scripts/screenshot-scene.sh --shop` flag that earns a payout and opens
the menu so a shot shows both affordable and unaffordable slots. `sim_core`
was not touched.

**This completes the "Shop & Inventory" epic.** Buying is still a purchase
ledger and nothing more — the chain is built unconditionally at tick 0, and
actually placing what you bought is the next, separate epic.

### 2026-09-08 — Shop & Inventory, ticket 1: money can now buy something

The start of a directly-requested priority shift — turning this from a fixed,
always-built production line into a factory game where the player *buys* the
equipment and puts it down. That is two epics: this one (a real
purchasable-equipment economy) and, after it, real placement/rotation/pickup.
This ticket is the first half of the first epic, and it is Rust and data
only; the Godot build menu is ticket 2.

GAMEPLAY.md §1 has always said the loop is "money buys equipment, equipment
unlocks sauces, sauces satisfy contracts, contracts pay money," and until now
the first clause was the missing one — money accumulated and had nothing to
be spent on. There is now a `data/shop.ron` pricing every buildable kind
(every machine kind in `data/machines.ron`, plus the belt and pipe tiles,
which are infrastructure rather than machines), a `sim::shop` module that
loads and validates it the way the recipe and machine loaders already do, a
per-kind owned count, and a `Purchase` player command that debits the *same*
money counter a contract delivery credits — deliberately one currency, per
that section, not a shop with its own wallet. Prices are placeholders
(GAMEPLAY.md §10 puts real balance in a later pass) but they encode a shape:
a belt tile is small change, and the 2x2 cooking vat that runs the chain's
longest process is the thing you save up for.

**What this ticket deliberately does not do is change the factory.** Every
machine still exists unconditionally from tick 0, exactly as before; buying
something credits a number and nothing else. Keeping the economy layer and
the placement rewiring apart means this half is fully testable in isolation
and moved no existing tile — the whole pre-existing test suite passes
untouched. The one cross-file invariant worth knowing about: adding a machine
kind to `data/machines.ron` without pricing it in `data/shop.ron` is now a
test failure, not a thing the player can never buy.

### 2026-09-08 — Contracts are content, and there are three of them

Simulation and data only (Rust + `data/`); `godot_project/` deliberately
untouched, and this is ticket 1 of 2 — the presentation half (contract cards
for a board rather than a single clipboard) is its own ticket. Closes
GAMEPLAY.md §9's long-standing "contracts need their own data file"
(`contracts.ron`) and the multi-contract half of §6.

Milestone 5's Economy MVP shipped exactly **one** contract, hardcoded in Rust
from four named constants. That was honest for a milestone whose only job was
"prove build factory -> get paid," but it meant the game's central design
pressure — GAMEPLAY.md §6's whole argument for contracts over a spot market —
did not exist: with one contract there is nothing to diversify *between*.
Contracts are now content in `data/contracts.ron` (schema_version 1, loaded
and validated by a new `sim/contracts.rs` alongside `recipe.rs`/`machines.rs`),
three of them ship, and each runs its own mix, its own cadence and its own
independently-tracked next-delivery tick.

**The decision worth not re-litigating is how a delivered box is allocated,
which GAMEPLAY.md left open.** There is one shared pool of collected stock;
a contract that settles *consumes* its mix from it, and goods are never
credited to two contracts at once. The tempting alternative — credit every
contract that wants the item — duplicates goods, so stacking contracts onto
one product line would double income for free, which is the exact strategy
the contract system exists to prevent. Contracts therefore compete for one
line's throughput, and file order is the (documented, deterministic)
tiebreak until a player-set priority exists. The shipped three are balanced
just inside the chain's real throughput, so a factory kept fed serves all
three and one left to run dry starts missing windows.

`economy_snapshot_json` changed shape as a result — the four top-level
contract fields are gone, replaced by a `contracts` array plus a
`delivery_stock` pool; `money` is unchanged. That breaks the existing Godot
clipboard panel, which is exactly what ticket 2 is for.

### 2026-09-08 — Milestone 7, ticket 1: the sauce has to wait

ARCHITECTURE.md §9 milestone 7 ("Timed processes") — the simulation half.
The chain gained a sixth stage: cooked ketchup now rests on an **aging rack**
for **2400 ticks** (80 seconds at §3.9's 30 ticks/sec, exactly ten times the
cooking vat's cook) before it can be bottled.

**The interesting result is how little there was to do.** `TimedProcess`
(§3.5) already existed and every machine's craft cycle already ran on it —
what had never been tested was whether it holds up at a *qualitatively*
different timescale, the kind GAMEPLAY.md §2's tier 6 (aged/fermented sauces)
is built on rather than the tens-of-ticks craft cycles the chain had so far.
It holds: this ticket changed `data/recipes.ron` and `data/machines.ron` and
**nothing whatsoever under `sim_core/src/`**. No change to `TimedProcess`, no
change to `Machine`, no change to `World`, no new field on the `SimBridge`
JSON surface (a machine's `process` is already a generic
`(start_tick, required_ticks)` pair, which is all a progress bar needs), and
— because the stage was inserted *mid*-chain so the chain's final output id
never moved — no change to `economy.rs`'s hardcoded contract either. That
negative result is the milestone, so it is pinned by tests rather than
asserted in prose: a new `sim_core/tests/timed_processes.rs` checks the
duration is genuinely an order of magnitude beyond every other process (read
out of the data file, not hardcoded twice), that the batch completes on
exactly its boundary tick and not one earlier, that the economy module
contains no mention of the new ids, and that the rack's progress is readable
through the *unchanged* snapshot surface.

**Tick-counted, not wall-clock, is now something we've measured rather than
something the doc says.** Two identical worlds age a batch; one is stopped
dead partway through for a quarter-second of real time with no ticks issued,
then resumed. Both finish on the identical simulation tick, the in-flight
process is byte-identical across the pause, and the test also asserts the
paused run really did take longer in real time — otherwise it would quietly
degrade into comparing two identical runs. That is §3.5's central claim,
checked.

Two things worth knowing. The aging rack is **1x1**, deliberately: an aging
stage's cost is time, and the footprint field cannot express time — and the
chain now runs x = 0..30, one tile short of chunk (0, 0)'s east edge, so
growing it (or anything upstream) would push the last stage into a second
chunk. That is handled correctly but is a deliberate change, not an accident,
and several tests pin the chain's chunk list. And ageing *ketchup* is a
placeholder: GAMEPLAY.md §2 puts ageing at tier 6, which has no production
line yet, so the mechanism is proven on the line that exists — mid-chain
specifically, so a long process is exercised with real stages back-pressured
above it and starved below it. Ticket 2 (the Godot presentation of the rack)
is next.

### 2026-09-08 — Visual overhaul round 2, ticket 3: the belts and pipes now go somewhere

Presentation-only, and the last ticket of the epic. `sim_core` gained one
doc-comment clarification and nothing else; `data/` was untouched. It answers
the sharpest complaint from the project owner's live-demo review — *"a
conveyor belt going into a mixer seems to just go straight into the side
there is no connecting so it looks like items just disappear into the wall"*
— which the previous pass's flush-footing fix had not: that closed a
floor-level crease, and a flush wall is still a wall.

**The finding that decided everything came from rendering the junctions one
at a time, and it contradicts the obvious fix.** The camera is fixed at a
`(1, 1, 1)` view direction, so the only vertical faces it can ever see are
the ones facing +X and +Z. The chain runs +X — which means a machine's
*outlet* face is dead-on camera-facing, and its *inlet* face points directly
away and can never be seen at all. Carving an inlet doorway, the thing this
ticket was written to ask for, would have been invisible. So the two halves
of a junction are deliberately different pieces: an outlet is a real doorway
cut through the +X wall with a lip reaching the tile edge, and an inlet is a
hooded transfer housing whose *camera-facing side* is cut away, showing a
brass threshold at the lip and darkness beyond it. Both read by value
contrast rather than by detail, because at scene scale a junction is a dozen
pixels — a pale interior renders as an empty portal and a hazard stripe on
the one camera-facing edge renders as a dark bar, and both were built,
rendered and thrown away before the current version.

**The belt has two junction-adjacent variants now**, and which tile gets one
is derived from the layout the simulation already publishes: whether the
neighbour along that stage's own travel direction is covered by a machine
footprint, or by the hopper, which discharges the same way. That is the
buildable half of *"assets should look different depending on their
orientation and what's around them"* — what a tile is next to is on the wire;
which way it points still is not, so the variants are authored for a +X run
and the renderer falls back to the plain tile for anything else. The corner
limitation in `world.rs` is unchanged, and was re-stated there rather than
quietly weakened.

Three things turned up on the way. The pipe run's downstream end was an open
bore — the run literally stopped in mid-air — and its supply end was
*plugged*, because the flange drawn for the tank joint filled back in the
hole the tank discharges through. And the per-frame tick catch-up was
unbounded, so one slow frame silently fast-forwarded the whole factory:
two identical screenshot runs photographed visibly different states, which
makes a rendered scene useless as a before/after check. Clamped; two runs are
now byte-identical, which is the property this kind of review depends on.

Verified the way this project verifies visual work, plus one new layer. The
generators now have tests of their own (`cargo test -p voxgen`) asserting
that each machine's opening is a genuinely *empty* volume at the belt's own
lane and height — a mouth painted dark meshes as a flat rectangle and renders
as a decal, and no screenshot at scene scale can tell the difference. A new
headless `test_connections.gd` checks that the per-tile belt choice really
comes from the layout. Then the part that actually decided the design:
close-up renders of all twelve junctions in the factory, iterated through
five rounds of look-change-look.

### 2026-09-08 — Visual overhaul round 2, ticket 2: every machine has a face, and the sauce is liquid

Presentation-only. `sim_core` and `data/` were not touched at all — every fact
this ticket draws was already on the JSON wire, ticket 1 included. It answers
four specific complaints from the project owner's live-demo review, and the
method for all four was the one this project keeps re-learning: render the
whole factory, look at it, change something, look again.

**Two machines had no face.** `bottler` and `packer` had no entry in
`main.gd`'s model table, so the chain's last two stages both drew the generic
housing and the end of the line looked like the same machine twice. Both now
have models built around what actually identifies them — bottles standing
under a filling gantry with a capping head; a packing table with a stack of
case blanks, an open case and a taping head. The packer is deliberately low
and mostly *cardboard*, the one large warm matte surface in the factory, so
it reads apart from its stainless neighbours at a glance.

**The pipes and the crusher were redesigned, not tweaked.** Rendering them
alone was not enough to see what was wrong; rendering the factory was. Each
pipe tile carried four oversized rings, so a join stacked two of them and the
run read as a row of bobbins rather than one pipe; the sight opening removed
almost half the top wall, so the tube had no round silhouette left; and the
dark support legs out-massed the run they carried. It is now one flange per
tile, a short framed sight window, one slim centred stand — and the tube is
the *brightest* metal in the palette with the fittings darker, which is
backwards from convention and is the thing that finally let the eye follow
the run from the tank to the mixer. The crusher's flared lip was, at scene
scale, a flat plate laid over the machine. Replacing it with the obviously
correct shape — a converging feed mouth — made it worse, because a shallow
taper seen from a fixed oblique angle *is* a plate; that failed attempt is
recorded in the generator so the next session doesn't reach for it. The
version that works identifies the machine from the side instead: a squared
frame with an open feed pocket you can see product sitting in, and big driven
flywheels on the camera-facing face.

**The tank ticket 1 built is now drawn**, with a level gauge slot down its
front and a liquid column behind it scaled to the live fill — the same
one-poll/one-scaled-body pattern the pipe run's liquid and the hopper's
product level already use, deliberately reused a third time rather than
reinvented. It also gained the thing ticket 1 didn't: a **key to fill it**.
The shipped scene had no way to load the tank at all, so the chain silently
stalled at the mixer with every belt past it running empty; **V** is now the
fluid twin of SPACE, and the screenshot harness loads both consumables.

**And the sauce is liquid.** Tomato paste, ketchup mix and finished ketchup
travelled the belt as pails capped by a flat matte disc, which read as a
painted lid. The pails are hollow now, with the product surface a voxel below
the rim, and that surface takes `palette.ron`'s glossy `liquid` class — the
material milestone 6 added for exactly this. Three new palette entries carry
their own hue rather than being tinted in GDScript, because unlike a pipe's
contents these are fixed by the recipe chain.

One rename while in there: `item_sauce_pail` became `item_paste_pail`. The old
name still referred to `tomato_sauce`, an item id deleted two epics ago — the
exact drift that once shipped cardboard boxes down the belt.

Verified headlessly by the full `scripts/verify-core-loop.sh` suite (`cargo
test -p sim_core` unchanged and green, plus nine Godot scripts), with
`test_machines.gd` extended to fail if *any* stage falls through to the
generic model and `test_fluids.gd` extended to cover the tank's mesh,
placement, live level and hover. Then reviewed the way it has to be: repeated
rendered screenshots of the running factory, zoomed in, four iterations deep.

### 2026-09-07 — Visual overhaul round 2, ticket 1: the vinegar comes from a tank the player fills

Simulation-only (Rust + `data/machines.ron`), the first of three tickets in a
second visual-overhaul epic; tickets 2 (new/redone voxel assets, including a
tank model) and 3 (belt/pipe-to-machine connections) both build on it.

Milestone 6's fluid supply was an off-map tap: one pipe segment flagged
`is_source` that refilled itself to capacity every tick and could never run
out. That was the smallest honest thing at the time, but the project owner's
review of the live demo named the gap exactly — *"add a fluid equivalent of a
hopper for the vinegar"* — because there was nothing there for a player to
manage. There is now a **`FluidTank`**: a capped, player-loadable buffer
standing one tile west of the pipe run's head, filled with a `FillFluidTank`
command that mirrors `FillHopper` down to its clamping and its optional
`amount`, and drained by the *same* per-tick pressure-equalization pass that
already moved fluid along the run. Vinegar is a consumable now — it runs out,
the mixer stalls, and refilling starts the line again.

Two decisions worth not re-litigating. **The tank is not a pipe segment.** It
is a placed building with a footprint in `data/machines.ron`, like the hopper,
kept out of `FluidNetwork::segments` so the run's existing snapshot geometry
did not change shape at all; what it shares with a segment is the flow rule,
not the type. **It presents constant full head, not its own fill fraction.**
It stands beside the run as a raised vessel, so it discharges downhill: a
tank that equalized on fill fraction would keep three quarters of every
remaining unit inside itself forever, and the player would watch the mixer
starve while the gauge still read a quarter full. Full head also makes flow
strictly one-way with no special case — the shared rule already clamps a
transfer to what the donor has, so an empty tank simply moves nothing.

Placement followed the hopper's precedent exactly (one tile outside the
existing geometry, so introducing it moved no tile); capacity is 120 units,
sized against the line the way the hopper's 64 was, which is about three and
a half hoppers of tomatoes per fill. The tank is on the wire as
`fluid_tank_snapshot_json`, a `fluid_tanks` array in the chunk snapshot, and a
`"fluid_tank"` hover tile — all shaped as the fluid twin of the hopper's, so
tickets 2/3 can drive both from one code path.

### 2026-09-07 — Visual overhaul: closing the belt/pipe/machine seams, hopper feedback, clipboard typography

A dedicated presentation-only pass requested directly by the project owner
("things don't connect properly to adjacent tiles... we also need to improve
our UI elements"), on top of everything below. `sim_core`'s only change is a
doc comment (see the corner note below); no recipe, machine, or economy
behaviour moved.

**Root cause of the connection issue, found by actually rendering and
zooming into the scene rather than guessing from code:** `belt()` and
`pipe_body()` (`assets/voxels/generators/assets.rs`) have always run their
frames/bores flush to all four edges of their own tile, but every machine's
footing (`machine()`, the shared `machine_footing()` crusher/mixer/vat build
on, and the hopper's own plinth) was inset 1-2 voxels from its tile's edges.
That left a small but real, and completely consistent, seam of bare floor —
darkened further by SSAO into a visible crease — at every single belt-to-
machine and pipe-to-machine junction in the factory. Fixed by making every
one of those footings flush with its tile boundary, the same rule belts and
pipes already followed.

**The pipe run itself had a second, more specific gap**: a plain `pipe`
tile's solid header sits mid-tile, so the run's last segment (the one
`FluidNetwork::inlet_index` actually feeds a machine from) left real open
floor between its own bore and the mixer's inlet nozzle — and the two were
modelled 4 voxels apart in height on top of that. New `pipe_inlet` asset (a
branch fitting reaching to the machine-facing tile edge at the mixer's own
inlet height) closes it; a new `test_fluids.gd` assertion checks the inlet
segment actually renders as this distinct fitting, not a recoloured plain
pipe.

**Corners:** checked, not built. Every belt and pipe tile only ever runs
+X — there's no `direction` field in any snapshot for a renderer to even
read — so drawing a turn needs a sim-side topology change first, not just a
rotated asset. Documented in `world.rs`'s module doc rather than
speculatively built against a corner nothing places yet.

**UI polish**, scoped to real, specific gaps rather than a broad reskin: the
hopper's one status lamp used to sit a fixed colour regardless of state —
now it swaps between the same idle/running materials a machine's lamp uses
based on whether the hopper actually holds stock, so filling it gives
feedback beyond the level slowly rising. The contract clipboard's title and
money figure shared the body text's exact size, so nothing on the panel told
the eye what mattered most; both now step up in a small, consistent type
scale.

Verified the established way: `cargo test -p sim_core` and the full
`scripts/verify-core-loop.sh` suite both green (including the new fluids
assertion), and `scripts/screenshot-scene.sh` rendered and reviewed multiple
times — populated isometric, hover-pipe, hover-hopper, and first-person —
comparing tight crops of the same regions before and after the footing fix
to confirm it actually changed the generated geometry (voxel counts moved,
e.g. `machine_mixer` 11675 -> 12877) even where the visual delta reads as
subtle at screenshot resolution rather than dramatic.

### 2026-09-07 — Machines improvement, ticket 4: you can see all of it now

The presentation half, and the last ticket of the epic — tickets 1-3 built
machine footprints, three real front-end machines and a hopper, and none of
it had ever been drawn. `sim_core` is untouched: everything here comes out of
JSON those tickets already exposed.

Four new voxel assets, each with a silhouette that says what the machine
does: a **crusher** with two counter-rotating rolls visible in an open feed
throat, a **mixer** with a paddle agitator in an open drum (its drive motor
moved to a side bracket precisely so it doesn't hide the agitator), the 2x2
**cooking vat** with an access platform, ladder, steam stack and a sight
glass — the first asset in the pipeline bigger than one tile — and a
**hopper** that is a real funnel: a box section over a converging cone,
narrowing to a chute that steps down over the head of the first belt. The
material rule they follow is "food contact is stainless, structure is painted
or poured": the parts product touches are `metal`, everything holding them up
is `matte`, which is what gives UI_UX.md §4's shiny exception something to be
an exception *to*. All four went through the pipeline's own review loop and
all four were reworked after looking at the render rather than after writing
the generator — the crusher's rolls were originally sunk in a well too deep
for the isometric angle to see into, and the vat read as a sphere.

Rendering a machine now reads its **kind** and **footprint** from the
simulation, so the vat genuinely covers its four tiles (a node sits at its
footprint's centre, which for every 1x1 kind is still exactly its tile), and
a kind with no bespoke model still draws with the generic one — adding to
`data/machines.ron` never needs a Godot edit first. The hopper's product
level is a body scaled to its live fill every frame, deliberately the same
mechanism milestone 6 built for liquid in a pipe rather than a second visual
language, and SPACE's help text now says what it actually does. The one
hover mechanism grew a third branch: any tile of a multi-tile machine
resolves to it, not just the origin, and so does the hopper.

The rendered-scene review caught something no headless test could: the
item-model mapping still named `tomato_sauce`, an item ticket 2 deleted, so
two whole stages of the chain had been shipping cardboard boxes down the belt
instead of pails of paste and mix. Fixed, with a third pail livery for the
mix so the vat's long cook visibly changes something. That is the argument
for `scripts/screenshot-scene.sh` existing, made concrete.

### 2026-09-07 — Machines improvement, ticket 3: a hopper feeds the chain

The chain's raw ingredient no longer appears out of nothing. Since milestone
3, pressing SPACE ran a command that reached straight into the first belt and
teleported a tomato onto it — a fine "prove the belt renders" scaffold and a
bad mechanic, because nothing in the world held the ingredient and the
player's key-press *was* the production rate. There is now a **hopper**: a
real building, standing immediately west of the chain's first belt, that
holds a bounded buffer of tomatoes and meters them onto that belt one unit
per tick, and only when the belt has room. Filling it is the player action;
the rate is the machinery's.

Three calls worth not re-litigating. (1) **The hopper is its own stage type,
not a recipe-driven one.** `data/recipes.ron`'s list is the chain of
*processes*, and a hopper doesn't consume-and-produce — forcing it through
`Recipe` would have meant inventing an entry that creates matter from no
inputs, which would lie in the data file and break the mass-conservation
reading the tests depend on. So `recipes.ron` is untouched by this ticket;
`data/machines.ron` gains a `hopper` kind purely for its footprint, since a
footprint is a footprint whether or not a recipe names the kind. (2) **It is
placed at negative x**, west of the chain's start rather than at it, so
introducing a whole new building moved not one existing tile — the same
discipline ticket 1 followed. The cost is that the world now spans two
chunks (the hopper is in `(-1, 0)`, everything else in `(0, 0)`), which
`all_chunks` reports and a renderer must honour. (3) **Capacity 64, one
unit per tick**, both Rust constants rather than data: 64 tomatoes is 16
crusher crafts, enough that filling buys real breathing room and little
enough that keeping the line fed stays a decision. In practice the belt's
own spacing rule is the binding constraint, so a full hopper feeds at exactly
the rate the belt accepts and stops dead when the line backs up.

`PlayerCommand::SpawnItem` became `FillHopper` — a rename, not a third
command, since keeping both would leave two contradictory ways to feed the
chain. New `hopper_snapshot_json` carries placement plus live fill, and
hoppers also appear in `chunk_snapshot_json` and `query_tile_json`;
`chain_layout_json` is deliberately unchanged, because it is defined as one
object per recipe stage and the hopper is not one. Rendering the hopper and
its fill UI is ticket 4.

### 2026-09-07 — Machines improvement, ticket 2: crusher, mixer, cooking vat

Content, riding on ticket 1's mechanism. The chain used to open with two
abstract steps — a "cooker" that turned raw tomatoes straight into tomato
sauce, then a "reducer" that turned that into ketchup — which is not a
process anyone would recognise from a real plant. It now opens with the
three steps a ketchup line actually has: a **crusher** makes paste, a
**mixer** blends that paste with vinegar into a ketchup mix, and a **cooking
vat** cooks the mix down into finished ketchup. Bottling and boxing are
unchanged, so the line went from four stages to five.

The cooking vat is **2x2 — the first machine in the game bigger than one
tile**, and deliberately the only one. Size is meant to say something: a
crusher and an in-line blender are small units, a batch vat that holds the
chain's longest process (the cook is four times the crush) is not. That
makes ticket 1's accumulate-widths layout do visible work for the first
time — every stage downstream of the vat sits one tile further east than the
old fixed-span formula would have put it, and the vat claims a second row of
the factory floor.

The other half of the change is a quiet proof that the data-driven design
holds. The vinegar requirement moved from the cooked stage to the *mixing*
stage, which is where a real line adds it — and that took no Rust change at
all, because `World` builds its pipe run for "whichever recipe declares a
fluid input" rather than for a named stage. Neither data file's
`schema_version` moved either: their shapes didn't change, only their
content. `sim_core`'s whole test suite was updated to the new five-stage
chain rather than trimmed — mass conservation across the extra hand-offs,
the fluid gate stalling and then running at its new stage, and `query_tile`
resolving the vat correctly from all four of its tiles.

### 2026-09-07 — Machines improvement, ticket 1: a machine can be bigger than one tile

Infrastructure, not content. Until now `World` hardcoded that every machine
occupies exactly one tile, so a stage's width was the constant
`BELT_LENGTH_TILES + 1` and stage `i` always started at `i * 5`. Machine size
is now **data**: a new `data/machines.ron` (the file CLAUDE.md's Project
Structure section has been anticipating alongside `recipes.ron`) maps each
machine kind to a tile footprint, loaded and validated by a new
`sim_core::sim::machines` module built the same way `recipe.rs` is —
`schema_version` checked, degenerate footprints and duplicate ids rejected,
and a recipe naming a kind the file doesn't define reported as a clear error
instead of a stage that silently never appears.

The chain now lays itself out by *accumulating* each stage's own machine
width rather than multiplying a fixed span, so a wider machine pushes
everything downstream east by exactly its extra tiles. A footprint's origin
is its minimum-X/minimum-Y corner, which is also the tile its belt feeds
into — stated once in `machines.rs`'s module doc and referenced everywhere
else, so a later renderer never has to guess which corner a coordinate
means. `chain_layout_json`, `chunk_snapshot_json` and `query_tile_json` all
carry the footprint now, and crucially `query_tile` resolves a machine from
*any* tile it covers, not just its origin — that is what the hover system
needs before a multi-tile machine can exist. A footprint straddling a 32x32
chunk boundary is handled rather than rejected: the machine belongs to every
chunk it overlaps, so a renderer loading only visible chunks still finds it.

The point of the ticket was to change nothing visible. Every shipped kind
(cooker, reducer, bottler, packer) declares 1x1, and a new test asserts the
four-stage chain's tile coordinates against literals copied from the old
formula rather than recomputed from the new code — plus tests that drive
genuinely 2- and 3-wide machines, since "every real kind is 1x1" would
otherwise make a width bug unobservable. The whole existing suite passes
untouched, Godot side included. Tickets 2-4 of this milestone are what
actually make a machine bigger.

### 2026-09-07 — Milestone 6, ticket 2: the pipe run you can actually see

The presentation half of ARCHITECTURE.md §9 item 6, on top of ticket 1's
simulation work below. `sim_core` was not touched at all: everything here is
drawn from the JSON that ticket already exposed. Three new voxel assets
(`pipe`, `pipe_source`, `pipe_fluid`) join the pipeline, and `main.gd` builds
the run from `fluid_snapshot_json`'s own segment list — how many pipe tiles
there are, where they sit, and which one is the supply tap are all the
simulation's answer, never a constant in GDScript, the same rule the belt
chain already follows.

The liquid is real geometry, not a HUD number: a fluid body sits in each
pipe's bore and is rescaled every frame to that segment's live pressure, so
you watch the run fill from the source and watch the inlet drop when the
machine takes a draw. That is UI_UX.md §4's "visible liquid surfaces in
tanks/pipes" taken literally — the pipe is modelled hollow with an open
inspection channel along its top so the level is legible from the fixed
isometric angle at any fill, not just a full one. Two decisions worth not
re-litigating: the run is **stainless steel** (UI_UX.md §4's shiny metal
exception; acid-resistant piping is GAMEPLAY.md tier 5's *later* gate, so
the baseline shouldn't pre-empt it), and palette.ron gained a fourth
material class, `liquid` — a glossy dielectric — so the wet look stays a
palette fact rather than a number hardcoded in GDScript. Only the per-fluid
*tint* is decided Godot-side, because which fluid is in the pipe is live
data.

Hovering a pipe now reports its fluid, level and pressure, through the exact
same screen-ray/plane-intersection path machines use — the tile-to-text
lookup was generalized (`_hover_text_at_tile`) rather than given a second
mechanism, so it worked in first-person via the crosshair with no extra
code. New `godot_project/test_fluids.gd` (wired into
`scripts/verify-core-loop.sh`) asserts the structural half headlessly, and
`scripts/screenshot-scene.sh --hover-pipe` was added so the tooltip and the
liquid could be reviewed as an actual image, which is how the sight channel's
height got fixed — the first cut hid the liquid below half fill, which is
exactly the state worth seeing.

### 2026-09-07 — Milestone 6, ticket 1: fluids in the simulation core

The sim half of ARCHITECTURE.md §9 item 6 (Fluids). New
`sim_core::sim::fluid` module implements §3.4's model literally — a
`FluidNetwork` is a run of pipe segments each holding a *volume* of one
fluid, with pressure (fill fraction) equalizing between adjacent segments
every tick; there is no droplet, no per-item entity, and all state is
fixed-point `Fx`. A recipe can now declare `fluid_inputs`, and a machine
gates and consumes them exactly the way it already gates and consumes solid
input: no fluid, no craft, and crucially no half-consumed solid input
either. `data/recipes.ron` moves to `schema_version: 2` and hooks the
ketchup stage up to vinegar, which is what GAMEPLAY.md §2's tier table
always said ketchup needed — an existing stage gained an ingredient, the
belt chain's shape is untouched.

Two scope decisions worth not re-litigating. First, the pipe run is
**fixed** — built once in `World::new`, an always-refilling off-map source
at one end feeding one machine at the other — the same deliberately-minimal
stance milestone 5 took with its single hardcoded contract; player-placed
pipes and §3.8's union-find topology recompute stay out until placement is
real. Second, **which** stage gets plumbed is still a pure data question:
`World` picks whatever recipe declares `fluid_inputs` and carries whatever
fluid it names, so no recipe id, fluid id or stage index is written down in
Rust. Fluid state is exposed to Godot as a new `fluid_snapshot_json`
(plus pipes in chunk snapshots and `query_tile`), polled per-frame like the
economy snapshot rather than dirty-tracked, since fluid moves nearly every
tick. Ticket 2 renders it.

### 2026-09-07 — First-person crosshair hover

Follow-up after the diegetic-UI epic landed and got a live review: hover
tooltips only worked in ISOMETRIC mode (FIRST_PERSON unconditionally hid
them, since the mouse is captured there and has no meaningful screen
position). Added a small screen-centre crosshair (`_crosshair`, visible only
in FIRST_PERSON) and pointed `_update_hover_tooltip` at the viewport centre
instead of the live cursor whenever `_view_mode` is FIRST_PERSON — the same
pure `_tile_under_screen_pos` math drives both, since `project_ray_origin`/
`project_ray_normal` work identically for an orthogonal or a perspective
camera. That did surface one real gap the fixed isometric camera never
could: looking up at the sky in first-person points the ray away from the
belt-surface plane, and the old math would have happily returned a phantom
tile "behind" the camera — `_tile_under_screen_pos` now checks `t < 0` and
returns `HOVER_NO_TILE` in that case. `test_hover.gd` gained checks for both
the crosshair-driven resolution and the new upward-look guard, verified by
aiming the first-person camera at a known machine tile the same way the
existing isometric checks aim the cursor, no live Input needed.

### 2026-09-07 — Machine hover tooltips (UI_UX.md §5, one piece of the diegetic-UI epic)

Ticket A of the three-ticket parallel push below. Per-stage recipe/progress/
input/output detail used to sit as always-on text in the HUD panel;
UI_UX.md §5 explicitly calls for that as a hover tooltip instead. The
always-on panel is now gone entirely — hovering a machine in ISOMETRIC mode
shows a small panel near the cursor with that machine's live data, and the
tooltip never appears in FIRST_PERSON (the mouse is captured there for
looking around, so "hover" has no meaning).

Follows this file's established pure-function/thin-Input-wrapper split
(`_apply_pan`/`_pan_input_direction` etc.): `_tile_under_screen_pos` casts
the camera's ray for an explicit screen position and intersects it with the
belt-surface plane — no physics engine, matching this project's zero physics
dependency elsewhere — and is directly unit-testable; `_update_hover_tooltip`
is the thin, untested half that reads the live mouse position each frame.
New `godot_project/test_hover.gd`, wired into `scripts/verify-core-loop.sh`.
Also gave `test_screenshot.gd` a `--hover` flag (warps the cursor over a
known machine before rendering) so the tooltip could actually be reviewed as
an image, not just asserted headlessly — confirmed visually as well as via
the new test. Merged last of the three tickets, on top of the icon and
clipboard panel work below — reconciling meant deleting the old
`_machine_label`/`_economy_label` HUD code entirely (both superseded, by the
hover tooltip and the clipboard panel respectively) and factoring the hover
tooltip's construction into its own `_build_hover_tooltip()` alongside
`_build_contract_panel()`.

Worth flagging for whoever next touches the headless Godot test scripts: all
of them (not just the new one) call `main._ready()` manually right after
`root.add_child(main)`, but in a `--script`-mode SceneTree that add_child
*also* schedules the engine's own automatic `_ready()` call, deferred to the
first processed frame — so `_ready()` actually runs twice, silently building
two of everything under `main`. Harmless everywhere it's currently used
(every rebuild is deterministic and only `main`'s own fields get read
afterward), but it broke a naively-cached `Camera3D` reference while writing
this ticket's test — see test_hover.gd's own header comment for the full
story. Not fixed here; a pre-existing harness quirk shared by every existing
test in this family.

### 2026-09-07 — UI polish epic dispatched: hover info + diegetic clipboard UI

Started closing the gap UI_UX.md §5 flags and the DEVLOG has been tracking
as not-built: hover tooltips instead of dense always-on text, real icons
rendered from the voxel models instead of hand-drawn art, and the clipboard/
checklist motif GAMEPLAY.md §6 calls out contracts as the natural fit for.
Broken into 3 independently-dispatchable tickets (a new Run,
`run_cc6860507d36`) run in parallel: hover tooltips for machines, a voxel-
icon rendering pipeline, and a clipboard-styled reskin of the contract
panel. All three touch `main.gd`'s HUD-building code to some degree, so a
three-way merge conflict is expected and will be reconciled by hand, the
same way milestone 5 and the visual-polish pass were.

### 2026-09-07 — Voxel-rendered UI icons (UI_UX.md §5, one piece of the diegetic-UI epic)

Ticket B of the three-ticket parallel push above. Added a reusable way to
get a small icon texture for any item, generated from the real voxel models
rather than hand-drawn — `_icon_for(item_id)` in `main.gd`. Chose to extend
the existing offline pipeline (§2a's `vengi-thumbnailer` step) rather than a
live in-engine `SubViewport` renderer, but it turned out the existing
review-preview PNGs couldn't just be reused smaller — see the new UI_UX.md
§2b point 5 for why (short version: they're opaque and padded for "look at
the whole model," an icon needs a transparent, tightly-cropped render, and
`vengi-thumbnailer` has no flag for that, so a new `voxicons` binary
post-processes its output instead). New committed build output
(`assets/voxels/icons/`, staged into Godot by `cargo xtask` same as
`models/`) plus a headless test (`test_icons.gd`) covering the cached-lookup
and unknown-id-fallback behavior. Merged on top of ticket C below, so the
icon actually landed inside the new `ContractPanel`'s checklist row rather
than the old plain economy Label it was originally built against — the
hover-tooltip ticket is still separate, not touched here.

### 2026-09-07 — Contract readout reskinned as a diegetic clipboard (UI_UX.md §5)

First real piece of §5's UI style, dispatched as one of three parallel
tickets against the same commit (hover tooltips and a voxel-icon pipeline
being the other two — see above for how the icon merge reconciled against
this). The money/contract readout — previously one plain white Label folded
into the generic dark HUD panel — is now its own `PanelContainer`
("ContractPanel"): a warm paper-cream background framed in steel grey, with
a thick top border standing in for the clipboard's metal clip. The
contract's mix requirement (read live from `economy_snapshot_json`, never
hardcoded) renders as one small checkbox square per required unit, filling
in with a saturated "done" green and a tick mark as `contract_progress`
reaches each unit — the numeric "0/2" text stays alongside it, not replaced.
`godot_project/test_contract_panel.gd` (wired into
`scripts/verify-core-loop.sh`) checks this headlessly: the panel is a
genuinely separate node from the machine-line panel, one checkbox per
required unit exists, and a checkbox's style/tick-mark visibility actually
flips when driven through `_set_checkbox_checked`/`_update_contract_panel`.
Confirmed visually too, both empty and fully checked, via
`scripts/screenshot-scene.sh`.

### 2026-09-07 — Fixed the "empty void" lighting, remapped first-person up/down

Two follow-ups after actually using first-person mode. First, up/down is now
SPACE (up) and SHIFT (down) instead of E/Q, per request — SPACE only spawns
a tomato in isometric mode now, so there's no conflict with flying up.
Second, and bigger: the sky was configured but never actually shown — the
visible backdrop was a flat near-black colour, chosen specifically to avoid
a hard horizon line behind the tightly-framed isometric camera. That reads
as an empty void the moment first-person mode lets you look around freely,
which is exactly what got reported. Fixed by making the sky the real
background (`BG_SKY`), brightening it and the ambient light for a friendlier
daytime feel while staying grounded-industrial (UI_UX.md §3/§7 — no visible
sun disk added, to avoid the glow-streak problem that motivated the original
choice), and separating the floor's *visual* extent from the isometric
camera's auto-framing math, so the ground can stretch far past the chain
without changing the default isometric zoom. `test_presentation.gd` now
asserts the sky is actually the background, and `test_screenshot.gd` gained
a `--first-person` flag so this mode's lighting can be reviewed as an image
at all, not just eyeballed live.

### 2026-09-07 — Two view modes: isometric and first-person, toggled with TAB

Added a second way to look at the factory: TAB switches between the
existing UI_UX.md §1 isometric camera (which also gained right-click-drag
panning alongside WASD/scroll) and a new first-person "fly" camera — mouse
look, WASD to move, E/Q for up/down (not Space/Shift, since Space already
spawns a tomato and reusing it would fire both on one keypress). Both modes
share the one `Camera3D` node, reconfigured rather than swapped, so "exactly
one camera" stays true either way.

First-person is explicitly not a UI_UX.md concept and not real player
movement — there's no player entity, no gravity, no collision on the voxel
meshes, so it's a fly-cam, not a walk-cam. Movement is yaw-only horizontal
(pressing forward never climbs or dives just because you're looking up or
down), the same rule Minecraft's own creative-flight controls use, verified
headlessly by pitching the camera hard over and confirming forward movement
still has zero vertical component. Also fixed along the way: macOS trackpad
scroll wasn't zooming at all before this — it arrives as a different Godot
event type (`InputEventPanGesture`) than an actual mouse wheel
(`InputEventMouseButton`), which only the latter was handled. +/- keys now
zoom too. `test_camera_nav.gd` covers all of it: mode switching, look/move
math, drag-pan direction, and the trackpad/key zoom paths.

### 2026-09-07 — Camera pan and zoom

Added the navigation UI_UX.md §1 always allowed (it only forbids free
rotation of the fixed isometric angle): scroll wheel zooms, WASD/arrows pan,
clamped so you can't zoom past the point of losing the factory or so far out
it's a speck. The fixed angle itself never changes — pan moves where the
camera looks *at*, zoom changes the orthographic view size, orientation is
recomputed the same way either time. Verified headlessly
(`test_camera_nav.gd`, wired into `scripts/verify-core-loop.sh`): zoom
clamps at both ends, panning moves along the screen-relative axes (not raw
world X/Z), and orientation is provably untouched by either.

### 2026-09-07 — Visual polish: the voxel pipeline, and a factory you can look at

Built docs/UI_UX.md's asset pipeline for real and replaced the flat 2D
placeholder scene with the isometric 3D one that doc describes. Nothing in
the simulation changed — `sim_core`, `SimBridge` and `data/` are untouched,
and Godot still learns the chain's entire shape from `chain_layout_json` plus
chunk snapshots. This was purely "render what the sim already reports, far
better."

The pipeline is the substantial half. Voxel models are now generated by small
Rust functions (`assets/voxels/generators/`) against one shared, checked-in
palette, exported as MagicaVoxel `.vox`, and reviewed by rendering isometric
PNG thumbnails headlessly — no GUI voxel editor anywhere in the loop, which
was the whole constraint driving UI_UX.md §2a's design. Two of that section's
open questions closed in the process, and neither the way it expected: no
current Rust crate can write the `.vox` files needed (so the encoder is
hand-written, the fallback the doc itself anticipated), and the "which of two
Godot importers" question turned out to have one answer because the two
candidates are now the same repository.

The scene itself: concrete factory floor, rubber-and-steel conveyors, a
stainless processing vessel per stage, and voxel models for every product
from raw tomato to boxed ketchup — lit in real time with shadows, ambient
occlusion and a glow pass, under a fixed orthographic isometric camera. The
one rule that shaped the pipeline more than any other is UI_UX.md §4's
insistence that metals look deliberately shinier than everything else; that
is why an asset is emitted as one mesh *per material class* rather than one
mesh, and why the palette file owns material as well as colour.

Verification grew a piece it was missing: a headless test asserting the
structural claims of the look (camera really is orthographic, lights really
cast shadows, meshes really imported, metal really is shinier than matte),
plus a deliberately non-headless script that renders the actual scene to a
PNG — because no amount of headless testing can tell you whether a game looks
right. See CLAUDE.md's decisions log for the specifics worth not
re-litigating.

### 2026-09-06 — Milestone 5: Economy MVP

The loop stops being a production-chain tech demo and starts being a game:
`sim_core::sim::World` now owns a `money` counter (fixed-point, never a
float) and a single hardcoded contract — a fixed sauce mix (2x
`boxed_ketchup`), a fixed delivery cadence (every 1,000 ticks), and a fixed
payout — that the milestone-4 chain can actually deliver against. Collecting
finished product (`CollectOutput`) now deposits it into an accumulating
delivery buffer instead of discarding it; on each cadence boundary the
buffer is checked against the contract's mix, and a satisfied mix pays out
and consumes the delivered stock, while a miss just skips that window
(no contract-loss penalty yet — GAMEPLAY.md §11's still-open question).
Money and contract status are exposed to Godot as a JSON snapshot and
rendered as a minimal HUD label (money + per-item progress + ticks to next
check) — deliberately plain, not UI_UX.md's clipboard/checklist styling,
which is later polish. Verified headlessly two ways: a `cargo test`
integration test drives the whole chain end-to-end (spawn -> travel every
stage -> collect -> get paid) and asserts the exact payout lands only once
the mix is satisfied, plus `Economy`'s own unit tests cover cadence-boundary
edge cases; and the existing gdext-boundary Godot test now also round-trips
the new JSON snapshot. This resolves the money/contract architectural-home
question flagged as open since the cross-doc consistency pass (see
CLAUDE.md's decisions log and GAMEPLAY.md §9 for the actual shape used).
Explicitly deferred, per this milestone's own scope: `contracts.ron` data
loading, multiple concurrent contracts, and `StaffPools` — all still future
work.

### 2026-09-06 — Milestone 4: First full chain

Extended milestone 3's single belt+machine into a real multi-stage
production chain: raw tomato -> tomato sauce -> ketchup -> bottled ketchup ->
boxed ketchup, four sequential belt+machine stages, fully data-driven from
`data/recipes.ron`. The core design call: `sim_core::sim::World` now builds
one stage per recipe *in the data file's own list order*, chaining each
stage's machine output into the next stage's belt automatically — no
Rust code names which recipe follows which. This was proven, not just
claimed: after building and testing a working 3-stage chain, a 4th recipe
(`boxed_ketchup`) was appended to `data/recipes.ron` alone (verified via a
SHA-256 hash check that no `.rs`/`.gd` file changed), and the full headless
test suite — the Rust conservation test plus the real `gdext` JSON boundary
through Godot — passed against the extended 4-stage chain with zero code
changes. `sim_core/tests/core_loop.rs` now walks the whole chain generically
(via a recursive "how many raw tomatoes is this item worth" helper) rather
than asserting fixed counts, so it keeps working as the chain grows. The
Godot side (`main.gd`) reads a new `chain_layout_json` call once at startup
to build its `TileMap` sized to however many stages currently exist, instead
of hardcoding stage geometry — keeping intact milestone 3's "Godot draws
what the sim says exists" principle. See CLAUDE.md's decisions log for the
`ChunkSnapshot` shape change (`belt`/`machine` singular -> `belts`/`machines`
plural, since a chunk can now hold more than one stage) and the deliberate
scope limit (every recipe past the first has exactly one input/output, so
this is one straight line, not yet a branching factory).

### 2026-09-06 — Milestone 2: Mobile export spike

Deliberately run early (ARCHITECTURE.md §9) so mobile-export risk surfaces
now rather than at platform polish. Full details in ARCHITECTURE.md §9a;
headline result: **Android works, iOS doesn't in this environment.**

Android: stood up a standalone Android SDK/NDK (no Android Studio needed),
cross-compiled the milestone-1 `sim_core` crate for `arm64-v8a`/`x86_64` via
`cargo-ndk` with zero source changes, and got Godot's Gradle-based export to
produce a validly-signed APK with the extension actually embedded (confirmed
via `apksigner verify` and `unzip -l`). One real Godot gap found along the
way: its Android export CLI can fail with a **blank** error message if
`rendering/textures/vram_compression/import_etc2_astc` isn't set in
`project.godot` — tracked down by reading Godot's own export-plugin source
rather than guessing, now fixed in this project's settings. Installing and
actually launching the APK on a device/emulator was skipped this round — the
dev host was down to double-digit MB of free memory with sibling sessions
running concurrently, and booting an emulator on top of that felt like the
wrong risk to take. That's the one remaining unverified step before Android
export counts as fully proven, not just built.

iOS: could not be attempted for real. This dev environment has no Xcode.app,
only Command Line Tools — confirmed both via `xcodebuild`/`xcrun` failing to
find an iOS SDK at all, and via a Rust `aarch64-apple-ios` build that
compiled without error but linked against the wrong (macOS) SDK sysroot, so
even that partial result wasn't treated as a pass. Per this milestone's own
instruction to decide explicitly if iOS looks broken: **it stays a
stretch/deferred platform, not a milestone-10 launch commitment**, until
someone re-runs this spike on a Mac with real Xcode or Apple-hosted CI —
that's recorded as a genuinely open question (ARCHITECTURE.md §11), not a
settled "iOS is broken."

Also surfaced, unrelated to mobile export itself: late in this session the
shared dev host (many concurrent users, physical memory nearly exhausted)
started failing to load the project's own unmodified `sim_core` extension at
all — even the plain milestone-1 round-trip check — with a Rust
undefined-behavior-check panic inside `gdext`. Checksum-verified as the exact
same binary that worked fine earlier in the session, and reproduced even in
a clean environment, so this reads as host resource exhaustion tripping an
optional safety check, not a project regression — but it means
`scripts/verify-scaffold.sh` should be re-run on a calmer host before trusting
its result if it ever fails unexpectedly like this again.

### 2026-09-06 — Milestone 3: Core loop proof

First real simulation code: one belt, one machine, one recipe (tomato →
tomato sauce), rendered and interactable in Godot. `sim_core/` now has a
pure-Rust `sim/` module tree (belt/machine/recipe/inventory/world, no `gdext`
dependency) sitting under a thin JSON-based `SimBridge` translation layer —
`tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
`query_tile()`, per ARCHITECTURE.md §4. The belt implements real gap-based
item storage (§3.3, Factorio's FFF #176 trick), the machine runs the generic
tick-counted `TimedProcess` pattern (§3.5), and all shared simulation state
uses fixed-point (`fixed::I16F16`), never float, per §3.9. The recipe itself
lives in a new top-level `data/recipes.ron` — resolving §11's data-file-path
open question — not hardcoded in Rust.

Verified in three layers, cheapest first (`scripts/verify-core-loop.sh`
automates all three): a headless `cargo test` integration test ticks the
belt+machine for thousands of ticks and asserts strict item conservation
(no duplication/loss) every single tick; a headless Godot script drives the
real `gdext` JSON command/snapshot boundary end to end (spawn 4 tomatoes →
1 tomato sauce → collect); and a second headless Godot script simulates an
actual keypress through the interactive scene's input handler to confirm
the "interactable" requirement is real, not assumed. The interactive scene
(`main.tscn`) renders via a genuine `TileMap` + moving item sprites, driven
only by `SimBridge`'s JSON snapshots — it never touches the belt's internal
gap array. Visuals are flat placeholder textures, explicitly not a UI_UX.md
style decision (see `godot_project/placeholder_art/README.md`); the real
voxel asset pipeline is still unbuilt. See CLAUDE.md's decisions log for two
smaller implementation calls worth knowing about: the belt's gap
representation and why the `gdext` boundary is JSON strings instead of
`Dictionary`/`Array` values.

### 2026-09-06 — Milestone 1: Scaffold

First real code. Set up the Rust workspace (`sim_core/` cdylib crate + an
`xtask` build-script crate) and a Godot 4 project (`godot_project/`), wired
together via `gdext` — a trivial `SimBridge.ping()` callable from GDScript
proves the Rust↔Godot round trip end to end. `cargo xtask` builds and copies
the compiled extension into place per ARCHITECTURE.md §7; `sim_core.gdextension`
is checked in. Pinned the exact versions ARCHITECTURE.md §1a had left as
placeholders (Godot 4.6.3-stable, `gdext` 0.5.5, Rust edition 2024, `hecs`
0.11.1) — see CLAUDE.md's decisions log for the reasoning.

Verified entirely headlessly, no editor GUI: `scripts/verify-scaffold.sh`
builds, primes Godot's local import cache (working around a known, benign
Godot 4.6.3 editor-shutdown crash — see the script's own comments), then runs
`godot --headless --script test_round_trip.gd`, which calls `SimBridge.ping()`
and asserts the known return value, exiting non-zero on failure. This is now
the project's headless "does the interop boundary still work" check.

### 2026-09-06 — Milestones 1-5 dispatched via Orca orchestration

Adopted Orca orchestration (see CLAUDE.md's Orchestration section) as how
milestone work actually gets built, then used it for real: created a Run and
a 5-task DAG covering milestones 1-5 (Scaffold, Mobile export spike, Core
loop proof, First full chain, Economy MVP), dependency-chained to match
ARCHITECTURE.md §9. Milestone 1 (above) was the first task dispatched and is
now complete; 2-5 are queued behind their dependencies and will be dispatched
as each prior milestone completes. This is the actual start of writing game
code — everything before this was design docs only.

### 2026-09-06 — Repo set up, full design-doc pass complete

Set up the repo (private, Conventional Commits enforced via hook + CI) and
wrote all three core design docs: **ARCHITECTURE.md** (Rust sim core +
Godot/`gdext` presentation, chosen to be agent-friendly, then hardened via a
critical-review pass that turned several open options into firm decisions),
**UI_UX.md** (isometric voxel art at a 32×32×32 tile scale, Minecraft-adjacent
UI, plus a researched asset pipeline so voxel models can be authored and
reviewed by an agent), and **GAMEPLAY.md** (tiered sauce/equipment
progression, a contracts-based economy designed to force a diversified
factory, staff as an abstract resource pool). Still no game code — the docs
are the whole project so far. Next up: milestone 1 (scaffold) from
ARCHITECTURE.md. For specifics, see the docs themselves, CLAUDE.md's
decisions log, or `git log`.

### 2026-09-06 — Cross-doc consistency pass

Read back through all three design docs together looking for gaps and
contradictions now that they've each been written somewhat independently.
Found and fixed a real one: the milestone list never actually scheduled the
economy/contracts system, even though the gameplay doc defines the core loop
as needing it — added an "Economy MVP" milestone. Also gave the architecture
doc an "Open questions" section (its siblings already had one) and flagged a
couple of things that don't have a designed home yet (money/contract state,
mobile touch input). Tightened up repo config (`.editorconfig`, a fuller
`.gitignore`) while at it. Nothing structural changed — still docs-only,
still pre-milestone-1.

