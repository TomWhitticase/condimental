# Sauce Factory — Decision records (full history)

This file holds the **full, verbatim decision records** that used to live in
`CLAUDE.md`'s decisions log, `docs/UI_UX.md` §2b and `docs/ARCHITECTURE.md`
§9a. They were moved here on 2026-09-09 because the three `@`-included docs
had grown past the context budget. This file is deliberately **not**
`@`-included into `CLAUDE.md`.

What stays loaded every session is the **rule** each record produced, in
condensed form:

- `CLAUDE.md` → "Decisions log (condensed)"
- `docs/UI_UX.md` → §2b (same point numbers as below)
- `docs/ARCHITECTURE.md` → §9a (summary)

Read this file when you need the *why* behind a rule, or the history of a
change. New decisions: add the condensed rule to the loaded doc, and the full
reasoning here (newest first within each part). Point/entry numbering here
matches what code comments and data files cite.

---

## Part A — CLAUDE.md decisions log (verbatim, newest first)

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
  RISK / ⚠ LAST CHANCE, with "N short" and the window percentage), a window
  bar, strike pips with the rule in words, and a heavier border in the
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
  runs were one simulation tick apart (the screenshot ticks by wall clock;
  two runs of the OLD renderer differ from each other the same way). With
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
There is no point 10 — it was skipped in the original numbering.

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

