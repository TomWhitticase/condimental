# Sauce Factory — Development Log & Status Tracker

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
- **Repo:** Private GitHub repo (`TomWhitticase/sauce-factory`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
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
- **Contract board (DONE, both halves, 2026-09-10):** contracts are **signed and can be lost**. A new game runs one basic contract; the rest of `data/contracts.ron` (schema 2, now four offers with `price`, `strikes` and `starter`) is a board the player buys from on **C**, one of each contract only. A contract that misses its strikes in a row is lost — it leaves the clipboard, is announced, and goes back on the board to be signed again at its price. Each docket now says how it is doing and whether it is in danger: a status line (✔ READY / ⚠ AT RISK / ⚠ LAST CHANCE), a window bar, strike pips and a state border. The top offer exceeds one line's throughput beside the others, by design. Built on branch `TomWhitticase/contract-board`.
- **Chain rework (DONE, both halves):** the middle of the production line is **piped, not belted**, which is what a real sauce plant looks like. The mixer's output and the cooking vat's output are fluids (`recipes.ron` schema_version 3 gained `fluid_outputs`), so mix flows from the mixer to the vat and cooked ketchup from the vat to the bottler through pipe runs with no tank behind them; the bottler is where the product becomes a solid again, and the cooking vat is now the one machine on the line **no belt touches at all**. The mixer gained **sugar** as a second solid ingredient and grew to **1x2** for it — a machine accepts solid input on every tile of the edge opposite the way it points, so N solid ingredients need N tiles of inlet face or the two lanes merge onto one belt and deadlock. The aging rack is gone. **Raw ingredients cost money**: `data/ingredients.ron` prices tomato, sugar and vinegar, a `BuyIngredient` command buys them, and a hopper or a tank is loaded from that bought stock *(originally with the global `SPACE`/`V` keys; since the configuration panel below, from the building's own panel)* — so overproduction is a real loss rather than free accumulation. On the presentation side: a hopper's contents are chosen **when it is placed** (the ghost carries a tinted product body and the prompt names them in words; the **Q** key that cycled them is gone, replaced by the placed hopper's own picker), because with two raw solids the simulation can no longer infer which bin the player meant — until that landed, placing a hopper from the UI did nothing at all, silently. Buying stock has its own panel on **M** (the build menu was already as tall as the space above the hotbar allows), and a permanent larder strip above the hotbar shows what is carried so an unloadable hopper explains itself. The mixer, the cooking vat and the bottler were re-modelled with pipe nozzles where belts no longer meet them.
- **Hopper/tank configuration (DONE, both halves):** nothing about feeding a factory is on the keyboard any more. A placed hopper or tank is **clicked**, and a panel beside it shows its level, a **FILL** button (disabled with its reason in words when the larder is empty, the bin is full, or the tank is plumbed to nothing), an **AUTO-FILL** switch that keeps the bin topped up from bought stock every tick — spending stock, never money, and simply pausing while the larder is dry — and, for a hopper, an item picker that retargets it in place. The click is a toggle, only one panel ever exists, ESC closes it first, and it closes itself under any menu or mode. The global **SPACE**/**V**/**Q** keys and `CycleHopperItem` are deleted. Pinned by `test_config_panel.gd`; `scripts/screenshot-scene.sh --config TX,TY` photographs it.
- **Pipes & conveyors overhaul (IN PROGRESS — six of seven tickets merged, `docs/PIPES_AND_CONVEYORS_PLAN.md` has the ticket-7 hand-off notes):** seven tickets on one integration branch to make belts and pipes look right, read right, run fast and connect up. **Ticket 1 (sim) has landed:** every belt, splitter and pipe on the wire carries `links` — what feeds it, what it hands to, or what its pipe is joined to, per side — recorded by the topology pass so it is exact and free per tick. **Ticket 4 (Godot) has landed:** belts, splitters, pipes and the items riding them are `MultiMesh` instances (one per asset and material class) rather than a node each, with a 1500-tile stress fixture, a headless timing test (`test_perf.gd`) and `scripts/screenshot-scene.sh --stress`; machines and buildings keep their nodes, and the seams the belt-animation and corner tickets build on (frame sets, a caller-supplied yaw per tile, one item-positioning function) are in place in `main.gd`. **Ticket 3 (assets) has landed:** the pipe model family (straight, corner, tee, cross, end, machine/tank fittings, one fluid body per shape) on a 15.5 midline, with seven pipe rendering defects fixed at the generator. **Ticket 2 (assets) has landed:** the belt model family — straights and left/right corners in four animation frames with a solid raised direction arrow; the belt-side junction variants are retired (the plain tile carries the junction, the machine's hood does the rest). **Ticket 6 (Godot) has landed:** every pipe tile draws its shape (end, straight, corner, tee, cross), its turn, its machine/tank fittings and its per-shape liquid body from the wire's `links` alone; hover names a pipe's connections in words; the one-arrangement `pipe_source`/`pipe_inlet` tiles are deleted; `test_pipe_shapes.gd` and `scripts/screenshot-scene.sh --pipe-gallery` cover every shape at every turn. **Ticket 5 (Godot) has landed:** a belt's shape (straight, left/right corner, merge, start, end) comes from its `links`, the four-frame surface animates at item speed from the tick, items arc through corners (and are drawn face to face on every tile — they had been half a tile downstream), hover says feeds and hand-offs in words, the retired belt aliases are deleted, and a `--belt-gallery` fixture shows every shape and junction at every facing. Ticket 7 (gallery bug hunt and docs close-out) is to do; one sim rule discrepancy (a belt feeds a machine from any side of an inlet-edge tile) is flagged for ticket 7.
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
loses it), a window bar, one strike pip per strike the contract allows,
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
for **2400 ticks** (40 seconds at §3.9's 60 ticks/sec, exactly ten times the
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

