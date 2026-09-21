# Renders the real scene to a PNG so a visual change can actually be
# reviewed — by a person or by a Claude Code session reading the image.
#
# The rest of this project's verification is headless (ARCHITECTURE.md §7),
# and headless Godot has no framebuffer, so none of it can see the thing
# docs/UI_UX.md is actually about. This script closes that gap without
# needing a human at a screen: it runs main.tscn with real rendering, waits
# for the lighting/shadow/glow passes to settle, grabs the viewport, and
# exits.
#
# Run via scripts/screenshot-scene.sh, or directly:
#   godot --path godot_project --script test_screenshot.gd -- /abs/out.png [flags]
#
# **THE WORLD STARTS EMPTY.** Since the "Real Placement" epic a new game
# contains nothing at all, so most flags below begin by BUILDING something.
# What they can build is bounded by `sim::STARTING_MONEY`, which the
# inventory-hotbar ticket raised from 200 to 3000 — comfortably more than the
# shipped line's ~1,200 — so `--fixture chain` and `--contracts`
# can build and run the real line again. See test_layouts.gd's header.
#
# `--build` puts down the demo line (test_layouts.gd's `build_demo_line`): a
# hopper feeding belts that run east, TURN NORTH into a crusher facing WEST,
# which discharges back west, with a pipe run heading SOUTH beside it. Three
# of the four facings on belts and the fourth on the pipes, which is what
# makes it the right subject for reviewing per-tile rotation and the belt
# shapes the overhaul derives from `links` in one frame.
#
# `--fixture <name>` picks a different one: `machines` (hopper, machine and
# dock, i.e. one of each sort of building), `fluids` (a tank, a pipe run and
# the machine that taps it), `vat` (the 2x2 cooking vat, alone), `splitter` (a
# line that forks) or `chain` (THE WHOLE SHIPPED LINE, bought and placed out
# of a new game's own balance — test_layouts.gd's `build_reference_line`).
#
# `--contracts` builds that full line and then runs it until the contract
# board is in a genuinely mixed state — some deliveries made, some missed,
# goods standing on the dock — so the clipboard can be reviewed against a
# working economy rather than an idle one. It implies `--fixture chain`.
#
# `--stress` builds the STRESS LINE (test_layouts.gd's `build_stress_line`:
# a 1500-tile serpentine belt fed by hoppers plus three tank-fed pipe runs)
# and runs it, for reviewing the instanced tile/item rendering the pipes &
# conveyors overhaul's ticket 4 built, at the scale it exists for. It
# implies `--populate`, and the shot's frame prints the rendering server's
# object, primitive and draw-call counts (`SCREENSHOT_RENDER`) so the cost of
# a frame can be compared without a profiler.
#
# `--belt-gallery` builds the BELT GALLERY (test_layouts.gd's
# `build_belt_gallery`: every belt shape at every facing, every belt-to-
# building junction at every facing), the review subject for the pipes and
# conveyors overhaul's belt presentation.
#
# `--item-gallery` builds the ITEM GALLERY (test_layouts.gd's
# `build_item_gallery`) and stands one of each finished-goods model on it —
# the second-sauce epic's review frame, and the only one that shows a
# shipping case's LABEL, which the model previews cannot (they are rendered
# from the opposite corner).
#
# `--bbq` builds the BBQ LINE (test_layouts.gd's `build_bbq_running_line`:
# both sauces on one floor, the second one branching north off the ketchup
# line's outlet run, funded, crewed and run until boxes of both are moving).
# Implies `--populate`. With `--board --board-end` the shot is of the BBQ
# tier of the contracts board, mixed orders and all.
#
# `--bottleneck` builds the BOTTLENECK LINE (test_layouts.gd's
# `build_bottleneck_line`: the reference line on auto-fill, run until it has
# backed up behind the cooking vat), the fixture the machine-load epic's
# hover readout is reviewed on (with `--hover`). Implies `--populate`.
#
# `--staff` builds the reference line, runs it (implies `--populate`, so the
# meters read a line that is asking for its crew rather than `0 / 0`) and
# opens the STAFF panel (H) over it, the fixture the staff pools MVP's panel
# is reviewed on. `--understaffed`
# builds the UNDERSTAFFED LINE (test_layouts.gd's `build_understaffed_line`:
# the reference line on auto-fill with two manufacturing heads fired, run
# until the cooking vat is refused its crew) and parks the cursor on the
# vat, so its hover reads `starved · short of manufacturing staff` and
# `crew: 2 manufacturing`. Implies `--populate`.
#
# `--short-staffed` builds the same UNDERSTAFFED LINE and opens no panel and
# hovers nothing, so the always-visible SHORT-STAFFED strip on the clipboard
# is what the shot reviews. Implies `--populate`.
#
# `--production` builds the bottleneck line and opens the PRODUCTION panel
# (P) over it — the production-stats epic's screen, reviewed on a line
# with every stage counted and a contract paid. Implies `--populate`.
#
# `--full-dock` builds the FULL-DOCK LINE (test_layouts.gd's
# `build_full_dock_line`: the reference line on auto-fill, run until the
# shared delivery pool is at its 48-per-item ceiling) and freezes the frame
# on a lit half of the dock lamps' pulse, so the clipboard's `48/48` /
# `⚠ FULL` line, the belt standing still behind the dock and the pulsing
# deck lamps are all in one shot. `--dock-lamp dark` takes the other half of
# the pulse; two runs a `--dock-lamp` apart are the blink itself. Pair it
# with `--closeup 26,0,12` to judge the lamps, which are floor-level and
# small in a whole-line frame.
#
# `--populate` runs whatever has been built forward first so the screenshot
# shows product actually moving, not an idle factory.
#
# `--ghost <kind>` buys one of `kind` and enters placement mode with the
# cursor over open ground, so the VALID ghost and its prompt are in the shot.
# `--ghost-blocked <kind>` does the same with the cursor over something
# already placed, for the invalid state. `--remove-mode` enters remove mode
# with the cursor over a placed tile. These three are the only way to review
# build mode without a live interactive session — the same problem the hover
# flags solve for tooltips.
#
# `--first-person` switches to the first-person camera before rendering
# (looking slightly up, from height, so the sky/ground extent is actually in
# frame) — the only way to visually review that mode's lighting/skybox
# without a live interactive session.
#
# `--hover` warps the OS cursor over the first placed machine before
# rendering, so the hover tooltip (docs/UI_UX.md §5) actually shows up in the
# shot. `--hover-pipe`, `--hover-hopper`, `--hover-tank`, `--hover-dock` and
# `--hover-vat` do the same over the first placed pipe, hopper, fluid tank,
# dock, or the FAR CORNER of the first machine whose footprint is bigger than
# one tile (deliberately not its origin — "hovering anywhere on a multi-tile
# machine identifies it" is the claim, and hovering the origin would not show
# it). All of them share one implementation (`_hover_over_tile`) and differ
# only in how they pick the tile out of `world_layout_json` — which is
# `hover_targets.gd`, one table, read by the one loop in
# `_apply_hover_flags`.
#
# A HOVER FLAG THAT CANNOT FIND ITS TARGET REFUSES. Each of these used to be
# its own little function guarded by `if tile != HOVER_NONE`, so a flag whose
# target was not in the world aimed at nothing and the run photographed the
# parked cursor and said SCREENSHOT_OK: `--fixture full_dock
# --hover-splitter` handed back a clean frame of the dock, and plain `--hover`
# did it against the empty world. Nothing in the output said the flag had
# been dropped. That is a frame answering a question nobody asked at exit 0 —
# issue #70's class exactly, and the opposite of what the cursor guard below
# promises — so it is now one refusal for the whole set, naming the flag, the
# target and the fixture. `--ghost-blocked` and `--remove-mode` are the only
# other flags that aim at something the world may not contain, and they
# refuse on the same grounds.
#
# THE CURSOR IS GLOBAL, AND THIS SCRIPT OWNS IT FOR THE WHOLE RUN (issue #4).
# `Input.warp_mouse` moves the machine's ONE pointer, which outlives the
# process and is shared with every other window on the desktop — including a
# second `screenshot-scene.sh` running in a parallel worktree. Left alone that
# leaks both ways: a run asking for no hover at all inherits whatever tile the
# LAST run parked on and photographs its tooltip (`--fixture bottleneck
# --payout` came back with a dock tooltip from the `--hover-dock` run before
# it, reading numbers the frozen scene never refreshed, and cost a round of
# investigation to rule out as a game bug), and a run that does ask for a
# hover can have the pointer moved out from under it between the warp and the
# shutter 43 frames later. So: every run PARKS the cursor on empty ground
# first (`_park_cursor`), any --hover* flag then aims it, it is RE-ASSERTED a
# few frames before the shutter (`_settle_cursor`), and the frame is only
# written if the cursor really is over the tile it was aimed at
# (`_check_cursor`) — a mismatch REFUSES, since a tooltip belonging to the
# wrong thing is exactly the frame this script exists not to hand back.
#
# IN FIRST PERSON THE POINTER IS NOT WHAT AIMS THE TOOLTIP — THE CROSSHAIR
# IS. `main.gd`'s `_update_hover_tooltip` asks the SCREEN CENTRE what it is
# over when `_view_mode` is FIRST_PERSON and the mouse position otherwise, so
# every one of the three steps above asks the same question the frame does
# (`_hover_query_pos`), and a --hover* flag TURNS THE CAMERA rather than
# warping the pointer (`_aim_at_tile`). This is not a detail: checking the
# pointer in a view the pointer does not drive made `--first-person
# --hover-dock` fail at the shutter and blame a parallel worktree for it,
# which is the same class of confidently-wrong report as issues #4 and #70.
# The good news is that the leak itself cannot happen in first person — the
# query position is the middle of the window whatever the desktop's pointer
# did — so there the cursor work is a check rather than a cure.
#
# `--shop` opens the build menu (docs/UI_UX.md §5) before rendering.
#
# `--stock` buys an assorted starting kit (belts, pipes, a hopper, a tank and
# a machine or two) so the hotbar shows CARRIED sockets, multi-digit counts and
# EMPTY sockets side by side — the only way to review that distinction, since a
# new game carries nothing and every socket is empty. It buys uneven RAW STOCK
# for the same reason, so the STOCK plate shows a stocked row beside an
# out-of-stock one rather than three of a kind.
#
# `--hotbar N` presses hotbar key N (1..9, or 0 for the tenth slot) through
# the scene's real `_unhandled_input`, after buying one of that slot's kind so
# the socket is not empty — so the selected-slot state, the name flyout and
# the ghost that selection puts in the player's hand are all in the shot. It
# is the only way to review the hotbar's selected state without a live
# session, exactly as `--ghost` is for build mode.
#
# `--title` photographs the TITLE SCREEN — the menu exactly as the player
# first sees it, three buttons and the wordmark over the 3D factory backdrop
# (`title_scene.gd`). `--menu` is the same scene with its LOAD LIST open,
# which is the other half worth reviewing.
# `--menu` photographs the MAIN MENU (`menu.tscn`) instead of the game,
# having first written a save file so the load list has something real in it.
# It is the only flag here that renders a different scene, because the main
# menu IS a different scene — it is what `project.godot` opens on now.
#
# `--pause` builds a factory, runs it, and opens the in-game pause menu over
# it. Like `--ghost` and `--hotbar`, it is the only way to review a panel that
# only exists in response to a keypress.
#
# `--closeup TX,TY[,SIZE]` re-aims the ISOMETRIC camera at one simulation
# tile and zooms it right in (SIZE is the orthographic width in world units,
# default CLOSEUP_SIZE). The whole-factory shot is the right frame for "does
# the line read", and the wrong one for "does this belt actually meet this
# machine" — a junction is about four voxels of detail and at 1600x900 the
# full line gives it about six pixels.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const GameSaves = preload("res://game_session.gd")
const HoverTargets = preload("res://hover_targets.gd")

## Enough frames for shadow atlases, SSAO and glow to be resolved — a
## screenshot on frame 1 shows an unlit, shadowless scene and is misleading.
const WARMUP_FRAMES := 45

## The frame `_settle_cursor` re-asserts the cursor on — three before the
## shutter, so main.gd has frames left to lay the tooltip out with it.
const CURSOR_SETTLE_FRAME := WARMUP_FRAMES - 3

## Spawn attempts, and simulation ticks between them, used by `--populate`.
## Driven straight through the scene's real input and tick path rather than
## poking at simulation state — this is a screenshot of the actual game.
const POPULATE_SPAWNS := 60
const POPULATE_TICKS_PER_SPAWN := 10

## EVERY fixture this harness can build, and the only names `--fixture`
## accepts. `_build_fixture`'s `match` has one arm per name and no fallback:
## a name here with no arm is a loud error rather than a quietly wrong
## frame, which is the whole of issue #70. Keep the two in step — they are
## the same list written twice, and there is no third place to check.
const FIXTURE_NAMES: PackedStringArray = [
	"demo", "machines", "fluids", "vat", "splitter", "chain", "stress",
	"pipes", "gallery", "items", "fluid_gallery", "bottleneck", "bbq",
	"stalled_run", "understaffed", "shortfall", "full_dock",
	"unclerked_dock",
]

## The fixtures whose whole claim is a factory that is WORKING, and which are
## therefore checked against the simulation before the shutter fires
## (issue #94). Each of these settles for thousands of ticks and is
## photographed as a going concern: a line that has jammed, gone unstaffed or
## run its hoppers dry is not the subject, it is a broken shot that looks
## exactly like a good one.
##
## Deliberately NOT every fixture. The galleries produce nothing by design,
## `stalled_run`, `understaffed`, `shortfall` and `unclerked_dock`
## photograph the failure itself, and an empty world has nothing to be
## healthy about — asserting on
## those would be asserting the opposite of what they are for. Fixtures
## outside this list still get the `SCREENSHOT_HEALTH:` line; they are just
## not failed on it.
const HEALTHY_FIXTURES: PackedStringArray = ["bbq", "bottleneck", "full_dock"]

var _frames := 0
var _out := "user://scene.png"
var _fixture := ""
## The fixture a convenience flag asked for, and the flag that asked. Kept
## apart from `_fixture` so an explicit `--fixture` can be checked against
## it: a flag used to overwrite `_fixture` outright, so
## `--fixture understaffed --staff` photographed the reference line and said
## nothing about it (issue #70).
var _flag_fixture := ""
var _flag_fixture_flag := ""
## Set when `_initialize` refused the command line. `quit()` only asks the
## tree to stop at the end of the current iteration, so `_process` still
## runs once after it — and that one pass would build a fixture and print a
## second, more confusing failure.
var _refused := false
## `--allow-unhealthy` waives the health check for a shot that means to
## photograph a factory in trouble. The check is the default and the waiver is
## typed out, because the failure it catches — a frame of a broken line,
## reported as a success — is invisible to the reviewer looking at the PNG.
var _allow_unhealthy := false
var _populate := false
var _first_person := false
## The --hover* flags this command line asked for, out of `hover_targets.gd`
## — not seven booleans, so there is exactly one list of hover flags in the
## project and no way to ask for one without a finder and a refusal behind it
## (see `_apply_hover_flags`).
var _hover_flags: PackedStringArray = []
## The tile the cursor is supposed to be over when the shutter opens, and
## whether a --hover* flag asked for it (as opposed to `_park_cursor` having
## put it somewhere harmless). Both are the cursor bookkeeping issue #4 needed
## — see this file's header.
var _cursor_tile := Vector2i(-999999, -999999)
var _cursor_hovered := false
## Whether the park had to settle for a tile that has something to say, and
## which tile that was. A flag rather than a sentinel tile, because the
## quietest park of all — first person looking at the sky — reports
## `HOVER_NO_TILE`, which is the same (-999999, -999999) any sentinel here
## would use. `_park_cursor` announces it once (SCREENSHOT_NOTE) and
## `_check_cursor` then lets that frame through, because a tooltip this run
## explained in its own output is not the leftover issue #4 is about.
var _cursor_park_noisy := false
var _cursor_park_noise_tile := Vector2i(-999999, -999999)
## `--stalled-run` (issue #54): the BBQ branch with its liquid-smoke tank
## turned off its own run, cursor parked on a tile of the run that leaves
## dead. The one frame that can show a DEAD run and a merely EMPTY one side
## by side, with the hover saying which is which.
var _stalled_run := false
var _shop := false
## `--config TX,TY` clicks the hopper or tank standing on that tile once
## the fixture is built, so the frame shows its configuration panel — the
## level bar, FILL with its reason when it is blocked, the AUTO-FILL switch
## and (hoppers) the item picker. Without `--stock` the larder is empty, so
## the panel is photographed in its blocked state, which is the more useful
## of the two to look at.
var _config := false
var _config_tile := Vector2i.ZERO
## `--mothballed` builds the UNDERSTAFFED LINE, throws the mothball switch
## on the cooking vat and opens that vat's own configuration panel — issue
## #19's escape hatch in the state a player reaches it in. The frame shows
## the switch reading MOTHBALLED: OFF, the batch held where the switch
## caught it, and the machine's hover words; it implies `--populate`, so the
## rest of the line is running around the one machine that is not.
var _mothballed := false
## `--unclerked-dock` builds the reference line with its one logistics head
## FIRED and ticks only far enough to settle the scene — the window where
## the dock cannot be crewed but has not yet been refused anything, because
## nothing has reached it (issue #87). Pair it with `--hover-dock`: the
## point of the frame is that the tooltip says "short of logistics staff"
## from the STANDING fact, where before it said nothing at all until a box
## arrived. `--full-dock` photographs the other, latched case.
var _unclerked_dock := false
## `--market` opens the ingredient market, and `--contents N` presses the
## open configuration panel's picker slot N (with `--config`), so a placed
## hopper is photographed after being retargeted to another ingredient.
var _market := false
var _contents := -1

## `--contents-refill` buys the fluid just declared and pumps it back into the
## tank and its run, so the picker's note is photographed saying what a
## *second* switch would destroy. Without it the frame necessarily shows the
## unquantified wording: pressing the picker slot is itself the destructive
## act, so the tank and its run are empty the instant the panel is shot.
var _contents_refill := false
## `--no-hud` hides the whole HUD layer. It exists for `--closeup`: the
## contract clipboard and the hotbar between them cover the top-right and the
## bottom of the frame, which is exactly where a junction lands about half
## the time — so reviewing "does this pipe actually meet this machine", the
## thing closeups are FOR, kept coming down to whether the subject happened
## to miss a panel. The HUD has its own screenshots (--shop, --market,
## --hover-*); a junction shot does not need it in frame.
var _no_hud := false
## `--unlocked` puts the world on the far side of every progression gate in
## `data/unlocks.ron` — the shipped one takes ten deliveries — so the market,
## the larder and the board photograph with the BBQ tier released. Combine it
## with `--market` or `--board` to shoot the "after" half of a before/after
## pair; without it those panels show the "before" half, which is what a new
## game actually looks like.
##
## It sets the counter through `test_layouts.gd`'s `deliver`, i.e. the save
## file, rather than running a chain for the ten thousand ticks it would take
## to earn them: this is a camera, and `sim_core/tests/unlocks.rs` is what
## earns the gate honestly.
var _unlocked := false
## `--unlock-toast` crosses the gate WHILE the HUD is watching, one frame
## before the shutter, so the announcement line is up in the frame. The
## notice is the only part of this feature that cannot be photographed from
## a settled state — it exists for one moment and then fades.
##
## The notice slot holds one line at a time and queues the rest, so this flag
## needs the slot EMPTY at the crossing — `_clear_build_notices` sees to that
## — and it REFUSES rather than writing a PNG of whatever else is in it
## (`_check_unlock_toast`, issue #141).
var _unlock_toast := false
var _contracts := false
## `--board` opens the contracts board menu (the contract-board epic).
var _board := false
## `--board-end` scrolls that board to its last offer. The board lists
## `contracts.ron` in file order and shows about four at a time, so a tier
## appended to the end of the file — the BBQ one — is off the bottom of an
## unscrolled board. A player reaches it with the wheel; this reaches it
## through the same ScrollContainer, and does nothing without `--board`.
var _board_end := false
var _staff := false
## `--production` opens the PRODUCTION panel (P) over the settled bottleneck
## line — the fixture with every stage counted and a contract paid, so the
## rows, rates, bars and the all-time exit show something.
var _production := false
var _understaffed := false
## `--staff-shortfall` opens the staff panel over a line funded to less than
## its next payday's bill (the quality pass, ticket 4's BEFORE warning) —
## `--staff`'s own dispatch, aimed at `build_underfunded_line` instead of the
## fed reference line, so the footer's shortfall sentence is what the shot
## reviews rather than the meters `--staff` already covers.
var _staff_shortfall := false
## `--short-staffed` builds the same understaffed line as `--understaffed`
## but opens NOTHING and hovers nothing: the shot exists to prove the
## always-visible SHORT-STAFFED strip on the clipboard is unmissable with no
## panel up, which is the one thing a `--staff` or `--understaffed` shot
## cannot show (both put a panel or a tooltip over the screen).
var _short_staffed := false
var _hotbar_key := ""
var _stock := false
## `--legend-collapsed` folds the keybind legend away before the shot (the
## HUD ticket), which is the half of that feature a headless test cannot
## judge: whether what is LEFT — the plate's heading and the button naming
## F1 — still reads as a thing to click rather than as a stray label, and
## whether the corner it frees actually looks freed (UI_UX.md §2b point 12).
var _legend_collapsed := false
## `--contracts` waits for a STATE, not a tick count: a fixed count drifts
## with any change to throughput or to the shipped cadences and quietly goes
## back to photographing three identical checklists. What it waits for is at
## least one unit standing on the dock plus at least one delivery AND one miss
## on the board — which against the shipped contracts (asks of 2, 3 and 1 out
## of one shared pool) reads as one docket satisfied and the rest part-filled:
## the checkbox mechanic and the shared-pool competition in a single frame.
## A MINIMUM dock count rather than an exact one, because the contracts check
## every 1000/1500/2500 ticks and drain the pool about as fast as the line can
## fill it.
const CONTRACTS_REVIEW_DOCK_UNITS := 1
const CONTRACTS_REVIEW_TICK_LIMIT := 20000
## `--payout` photographs the celebration a contract payout fires: the coin
## burst over the dock, the "+N" over the money line and the PAID stamp on the
## docket that paid. This is the one review the payout work actually needs —
## `test_payout_feedback.gd` can prove every number, and cannot say whether ten
## coins over a dock read as a fountain or as a mess (UI_UX.md §2b point 12).
##
## Like `--contracts` it waits for the EVENT, not for a tick count: the first
## payout lands at tick 2500 against today's cadences and process times, and a
## hardcoded count would quietly go back to photographing an empty apron the
## day one of those moves.
var _payout := false
const PAYOUT_REVIEW_TICK_LIMIT := 20000
## Where in the burst's arc to stop the clock, as a fraction of
## COIN_FLIGHT_TICKS. Past `COIN_GROW` so the coins are at full size, and near
## the top of the flight, which is where the fan reads as a fountain rather
## than as a pile on the deck.
const PAYOUT_REVIEW_PHASE := 0.5
## `--crushing` photographs the crusher's spray: the pulp a working crusher
## throws out of its mouth on its own beat. The same review `--payout` exists
## for, one size down — a headless test can prove a fleck is thrown, launched
## from the mouth and reclaimed, and cannot say whether the burst reads as
## satisfying crushing or as a rash of red pixels (UI_UX.md §2b point 12).
##
## Like `--payout` it waits for the STATE, not for a tick count: the spray
## runs off `_machine_work_elapsed`, so a crusher starved of tomatoes or
## blocked on a full output throws nothing by design, and about a third of
## the shipped line's frames catch it that way. It ticks until the crusher is
## genuinely mid-batch with pulp in the air, then freezes, for the same
## reason `--payout` does.
var _crushing := false
const CRUSHING_REVIEW_TICK_LIMIT := 2000
## How many ticks past the first fleck to stop the clock: far enough for a
## second beat to leave the mouth, so the shot shows a spray rather than one
## bit, and short of CRUSH_FLIGHT_TICKS so the first beat is still up.
const CRUSHING_REVIEW_HOLD_TICKS := 6
## `--full-dock` photographs a delivery pool at its ceiling: the clipboard's
## ON THE DOCK line reading `48/48` in warning red under `⚠ FULL — belt
## backing up`, the boxes standing still on the belt the dock has stopped
## taking from, and the dock's deck lamps PULSING rather than steady. Both
## states arrived with the dock-capacity change and neither could be
## photographed on demand: `--bottleneck` leaves the pool around 20 of 48.
##
## `Layouts.build_full_dock_line` does the waiting (some 25 000 ticks against
## the starter contract's appetite); what is left here is the same freeze
## `--payout` and `--crushing` need, plus which HALF of the blink to stop on.
## A pulse is a rhythm rather than a state, so a single frame can land on the
## dark half and photograph a dock that looks like it holds nothing — this
## picks the half deliberately instead, and two runs a `--dock-lamp` apart
## are how a reviewer sees the blink itself.
var _full_dock := false
## `--dock-lamp dark` for the other half of the pulse; lit by default, since
## a lit lamp over a full pool is the state worth reviewing and a dark one is
## indistinguishable from an empty dock's.
var _dock_lamp_dark := false
## How long to hunt for a tick that is both full AND on the wanted half of
## the blink. A whole pulse is DOCK_FULL_PULSE_TICKS (12), so this is only
## generous because a contract may ship mid-hunt and drop the pool off its
## ceiling for a cadence — in which case the line refills and the next
## crossing is taken.
const FULL_DOCK_HOLD_TICK_LIMIT := 2000
var _ghost_kind := ""
var _ghost_blocked := false
var _remove_mode := false
## `--menu` renders menu.tscn rather than main.tscn; `--pause` renders the
## in-game pause menu over a running factory. See the header.
var _menu := false
## `--title` is `--menu` as the player first sees it — the title screen, with
## its three buttons, over the 3D factory backdrop — rather than the load
## list `--menu` opens. Both are worth a frame: the list is the half with the
## typography in it, and the title is the half that is a poster.
var _title := false
var _pause := false
## The save this run writes so `--menu`'s list is not empty, and removes
## again on the way out. Named so it cannot collide with a real save.
const MENU_REVIEW_SLOT := "zz review shot"
var _menu_review_path := ""

## `--item-gallery` — one of each FINISHED-GOODS model standing on a bare
## belt run (`test_layouts.gd`'s `build_item_gallery`), ketchup beside BBQ,
## at the size an item really renders. The second-sauce epic's one review
## that nothing else can give: a preview render is a single model blown up to
## fill the frame, and the model previews are shot from vengi's camera — the
## opposite corner to the game's (UI_UX.md §2b point 35) — which is the very
## face a shipping label is NOT on.
##
## Ketchup first, then its BBQ counterpart, so the pair to compare is
## adjacent. The items are stood through `main.gd`'s own
## `_show_carried_item`/`_carried_item_transform`, not posed here, so what
## this photographs is a belt item and not an approximation of one; nothing
## ticks in this fixture, so they stay where they were put.
const ITEM_GALLERY_ASSETS := [
	"item_bottle", "item_bottle_bbq", "item_box", "item_box_bbq"
]
var _item_gallery := false

## Where `--fluid-gallery` puts the camera when nothing else says: the
## centre of `build_fluid_gallery`'s 17x3 strip, at the widest zoom that
## still has all six runs inside 1600x900 with a margin.
const FLUID_GALLERY_FOCUS := Vector2(8.0, 1.0)
const FLUID_GALLERY_SIZE := 64.0

## `--closeup` target tile and orthographic width; `_closeup` gates both.
const CLOSEUP_SIZE := 9.0
var _closeup := false
var _closeup_tile := Vector2.ZERO
var _closeup_size := CLOSEUP_SIZE
var _main: Node


func _initialize() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() > 0:
		_out = args[0]
	_fixture = _string_arg(args, "--fixture")
	if _fixture != "" and not FIXTURE_NAMES.has(_fixture):
		# Refused rather than fallen back on. `_build_fixture`'s `_:` arm used
		# to build the demo line for a name it did not know, so a typo came
		# back as a plausible frame of the wrong world with a zero exit
		# status (issue #70).
		printerr(
			"SCREENSHOT_FAIL: no fixture named '%s'. Known fixtures: %s"
			% [_fixture, ", ".join(FIXTURE_NAMES)]
		)
		_refuse()
		return
	if args.has("--build"):
		_pick_fixture("demo", "--build")
	_populate = args.has("--populate")
	if args.has("--stress"):
		_pick_fixture("stress", "--stress")
		_populate = true
	if args.has("--pipe-gallery"):
		_pick_fixture("pipes", "--pipe-gallery")
	if args.has("--belt-gallery"):
		_pick_fixture("gallery", "--belt-gallery")
	if args.has("--fluid-gallery"):
		_pick_fixture("fluid_gallery", "--fluid-gallery")
	_item_gallery = args.has("--item-gallery")
	if _item_gallery:
		_pick_fixture("items", "--item-gallery")
	if args.has("--bottleneck"):
		_pick_fixture("bottleneck", "--bottleneck")
		_populate = true
	_staff = args.has("--staff")
	if _staff:
		_pick_fixture("chain", "--staff")
		_populate = true
	_production = args.has("--production")
	if _production:
		_pick_fixture("bottleneck", "--production")
		_populate = true
	_understaffed = args.has("--understaffed")
	if _understaffed:
		_pick_fixture("understaffed", "--understaffed")
		_populate = true
	_staff_shortfall = args.has("--staff-shortfall")
	if _staff_shortfall:
		_pick_fixture("shortfall", "--staff-shortfall")
		_populate = true
	_short_staffed = args.has("--short-staffed")
	if _short_staffed:
		_pick_fixture("understaffed", "--short-staffed")
		_populate = true
	_unclerked_dock = args.has("--unclerked-dock")
	if _unclerked_dock:
		_pick_fixture("unclerked_dock", "--unclerked-dock")
	_mothballed = args.has("--mothballed")
	if _mothballed:
		_pick_fixture("understaffed", "--mothballed")
		_populate = true
	_first_person = args.has("--first-person")
	_hover_flags = HoverTargets.requested_flags(args)
	_shop = args.has("--shop")
	var config_arg := _string_arg(args, "--config")
	if config_arg != "":
		var parts: PackedStringArray = config_arg.split(",")
		if parts.size() == 2:
			_config = true
			_config_tile = Vector2i(int(parts[0]), int(parts[1]))
	_market = args.has("--market")
	_board = args.has("--board")
	_board_end = args.has("--board-end")
	_unlocked = args.has("--unlocked")
	_unlock_toast = args.has("--unlock-toast")
	_no_hud = args.has("--no-hud")
	_contents = int(_string_arg(args, "--contents")) if _string_arg(args, "--contents") != "" else -1
	_contents_refill = args.has("--contents-refill")
	# A flag the harness accepts but cannot honour must stop the run, not
	# decorate the log (issue #123, same class as #110 and #114). --contents
	# needs a panel to press the slot in, and --contents-refill needs a slot
	# to have been pressed; without those the flag is silently dropped and
	# the run still reports SCREENSHOT_OK on a frame nobody asked for.
	if _contents >= 0 and not _config:
		printerr(
			"SCREENSHOT_FAIL: --contents %d needs --config TX,TY — there is no "
			% _contents
			+ "configuration panel open for it to press a picker slot in."
		)
		_refuse()
		return
	if _contents_refill and _contents < 0:
		printerr(
			"SCREENSHOT_FAIL: --contents-refill needs --config TX,TY and "
			+ "--contents N — it refills the fluid a picker slot just declared, "
			+ "so with no slot pressed there is nothing for it to refill."
		)
		_refuse()
		return
	_contracts = args.has("--contracts")
	if _contracts:
		_pick_fixture("chain", "--contracts")
	_payout = args.has("--payout")
	if _payout:
		_pick_fixture("chain", "--payout")
	_crushing = args.has("--crushing")
	if _crushing:
		_pick_fixture("chain", "--crushing")
		_populate = true
	_full_dock = args.has("--full-dock")
	if _full_dock:
		# NOT --populate: the fixture runs itself to the state it is named
		# for, and `_run_chain_forward`'s own spawns on top of that would
		# only be more ticks against an already-full pool.
		_pick_fixture("full_dock", "--full-dock")
	# Last of the fixture flags on purpose: --contracts, --payout and
	# --crushing each force the plain shipped line, and --bbq is meant to be
	# combined with the first of those (the mixed orders are the half of this
	# epic the clipboard shows), so it has the final say.
	if args.has("--bbq"):
		_pick_fixture("bbq", "--bbq")
		_populate = true
	_stalled_run = args.has("--stalled-run")
	if _stalled_run:
		# NOT --populate: the shot compares a DEAD run with a merely EMPTY
		# one, and a factory run forward has no empty runs left in it (the
		# fixture's own doc says why).
		_pick_fixture("stalled_run", "--stalled-run")
	_dock_lamp_dark = _string_arg(args, "--dock-lamp") == "dark"
	_hotbar_key = _string_arg(args, "--hotbar")
	_stock = args.has("--stock")
	_legend_collapsed = args.has("--legend-collapsed")
	_ghost_kind = _string_arg(args, "--ghost")
	if _ghost_kind == "":
		_ghost_kind = _string_arg(args, "--ghost-blocked")
		_ghost_blocked = _ghost_kind != ""
	_allow_unhealthy = args.has("--allow-unhealthy")
	_remove_mode = args.has("--remove-mode")
	_title = args.has("--title")
	_menu = args.has("--menu") or _title
	_pause = args.has("--pause")
	if not _parse_closeup(args):
		return
	_resolve_fixture()
	_apply_fluid_gallery_defaults(args)
	# The world this frame actually came from, printed whether or not
	# anything asked for one — a frame that cannot be traced back to its
	# fixture is not evidence (issue #70).
	print("SCREENSHOT_FIXTURE: %s" % (_fixture if _fixture != "" else "none (empty world)"))
	if _menu:
		# The load list is the half of this screen worth reviewing, so give it
		# something to list. Written before the scene is built, because
		# `menu.gd` reads the directory in `_ready`.
		_write_menu_review_save()
		_main = load("res://menu.tscn").instantiate()
		root.add_child(_main)
		return
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	# NOT here: in a real (non-headless) run, add_child()'s _ready() is
	# deferred to the engine's own next-frame processing, unlike the pure
	# SceneTree tests that call _ready() directly — _main._camera doesn't
	# exist yet at this point. Anything that touches it has to wait for
	# _process()'s first frame instead. (Learned the hard way: this used to
	# live here and crashed with "base object of type Nil".)


## Stops the run with a non-zero status. `quit()` alone leaves one more
## `_process` pass to come, so this also latches `_refused`.
func _refuse() -> void:
	_refused = true
	quit(1)


## Records the fixture a convenience flag implies. The LAST caller wins,
## which is the precedence this file has always had and which
## `scripts/screenshot-scene.sh` documents (`--bbq` is deliberately parsed
## last, so `--contracts --bbq` is the BBQ line with the contracts board
## over it). What changed with issue #70 is only that this no longer
## overwrites an explicit `--fixture`; `_resolve_fixture` decides that.
func _pick_fixture(fixture: String, flag: String) -> void:
	_flag_fixture = fixture
	_flag_fixture_flag = flag


## Settles `--fixture` against the convenience flags.
##
## **An explicit `--fixture` wins, and the override is announced.** A flag
## used to overwrite it in silence, so `--fixture understaffed --staff`
## photographed the reference line and reported a fully-crewed factory —
## the opposite of what was asked for, with a zero exit status (issue #70).
##
## Only the FIXTURE is overridden. Everything else `--staff` means (open the
## staff panel, run the line forward) still happens, which is what makes the
## combination worth typing: the understaffed line with the staff panel over
## it is a frame no single flag can ask for.
func _resolve_fixture() -> void:
	if _flag_fixture == "" or _fixture == _flag_fixture:
		if _fixture == "":
			_fixture = _flag_fixture
		return
	if _fixture == "":
		_fixture = _flag_fixture
		return
	print(
		"SCREENSHOT_NOTE: --fixture %s overrides %s's own fixture ('%s'); everything else \
%s does still applies"
		% [_fixture, _flag_fixture_flag, _flag_fixture, _flag_fixture_flag]
	)


## `--flag value` (or `--flag=value`, for a value that starts with `-` and
## would otherwise be read as another flag — a negative tile coordinate) as
## one look-up, since every option below that takes an argument takes
## exactly one.
func _string_arg(args: Array, flag: String) -> String:
	for i in range(args.size()):
		var arg := String(args[i])
		if arg == flag and i + 1 < args.size():
			return String(args[i + 1])
		if arg.begins_with(flag + "="):
			return arg.substr(flag.length() + 1)
	return ""


## Moves the lifetime delivery counter to `offset` away from the last gate
## in `data/unlocks.ron` — 0 to sit exactly on the threshold, -1 to sit one
## delivery short of it. The threshold is read off the wire, never written
## out here, so a balance pass on that file cannot leave these shots
## photographing the wrong side of the gate.
func _cross_gates(offset: int) -> void:
	var econ: Dictionary = JSON.parse_string(_main.sim.economy_snapshot_json())
	var threshold := 0
	for unlock in econ.get("unlocks", []):
		threshold = maxi(threshold, int(unlock.get("after_deliveries", 0)))
	if threshold == 0:
		push_error("_cross_gates(): data/unlocks.ron gates nothing")
		return
	Layouts.deliver(_main.sim, maxi(0, threshold + offset))
	# `deliver` goes through save/load, so the rendered world is rebuilt from
	# the reloaded one — identical by construction, but the scene has to be
	# told, exactly as `_build_fixture` tells it.
	_main._rebuild_world()
	_main._refresh_owned_counts()


## The glyph `main.gd::_announce_unlocks` opens its announcement with. The
## other two notices are `✔ SIGNED:` and `✗ CONTRACT LOST:`, so the first
## character is enough to tell them apart without pinning the wording.
const UNLOCK_NOTICE_GLYPH := "★"


## Drops the "what just changed" banner the fixture's own construction raised,
## and the queue behind it.
##
## A fixture buys, places, hires, signs and runs tens of thousands of ticks
## inside frame 1, so main.gd's contract notices fire for things that — in the
## world this shot claims to photograph — happened long before the shutter.
## `--bbq` signs two orders as it builds, and its frames came back with
## `✔ SIGNED: smokehouse_trial` hanging over a factory that had been running
## for 27,000 ticks.
##
## It costs more than a wart. A notice holds for `CONTRACT_NOTICE_HOLD_MSEC`
## of WALL CLOCK and everything raised while it is up queues behind it, so the
## stale banner swallowed the one thing `--unlock-toast` exists to photograph
## (issue #141). Clearing it is not the harness overruling that queue — the
## queueing rule is right, an unlock really does land on a delivery, and
## `test_unlocks.gd` still proves the announcement fires. It is the harness
## declining to photograph its own setup.
func _clear_build_notices() -> void:
	if _main == null or _main._contract_notice == null:
		return
	_main._contract_notice_queue.clear()
	_main._contract_notice_until_msec = 0
	_main._contract_notice.text = ""
	_main._contract_notice.modulate = Color(1, 1, 1, 0)


## Proves the announcement is the notice in frame, rather than trusting it.
##
## `--unlock-toast` wrote a perfectly good-looking PNG of the wrong banner for
## a day and still printed `SCREENSHOT_OK:` (issue #141); nothing but a
## reviewer's eye caught it. A shot of the wrong thing is worse than no shot,
## because a PR body presents it as evidence — so refuse, like the fluid
## gallery does when its runs have merged.
func _check_unlock_toast() -> bool:
	var notice: Label = _main._contract_notice
	if notice == null:
		printerr("SCREENSHOT_FAIL: --unlock-toast, but the HUD has no notice line")
		return false
	if not notice.text.begins_with(UNLOCK_NOTICE_GLYPH):
		var wrong := (
			"SCREENSHOT_FAIL: --unlock-toast crossed the gate, but the notice "
			+ "in frame reads \"%s\" — the announcement is queued behind that, "
			+ "not photographed"
		)
		printerr(wrong % notice.text)
		return false
	if notice.modulate.a < 0.99:
		var faded := (
			"SCREENSHOT_FAIL: --unlock-toast raised the announcement, but it "
			+ "had faded to %.2f alpha by the shutter — the run took longer "
			+ "than the notice holds"
		)
		printerr(faded % notice.modulate.a)
		return false
	return true


## Builds whatever the flags asked for, through the same purchase/place
## commands a player uses (test_layouts.gd). The camera is re-framed
## afterwards, because it frames the placed world and there was nothing to
## frame when `_ready` ran.
func _build_fixture() -> void:
	match _fixture:
		"machines":
			Layouts.build_machines_fixture(_main.sim)
		"fluids":
			Layouts.build_fluid_run(_main.sim)
		"vat":
			Layouts.build_multi_tile(_main.sim)
		"splitter":
			Layouts.build_splitter_line(_main.sim)
		"chain":
			Layouts.build_reference_line(_main.sim)
		"stress":
			Layouts.build_stress_line(_main.sim)
		"pipes":
			Layouts.build_pipe_gallery(_main.sim)
		"gallery":
			Layouts.build_belt_gallery(_main.sim)
		"fluid_gallery":
			# The one fixture that can be built and still not BE itself:
			# move a pipe and two runs merge, and the frame comes back a
			# perfectly good-looking factory missing a colour the reviewer
			# was told it held. Refuse instead — no PNG, non-zero status.
			var complaint: String = Layouts.build_fluid_gallery(_main.sim)
			if complaint != "":
				printerr("SCREENSHOT_FAIL: %s" % complaint)
				_refuse()
				return
		"items":
			Layouts.build_item_gallery(_main.sim)
		"bottleneck":
			Layouts.build_bottleneck_line(_main.sim)
		"bbq":
			Layouts.build_bbq_running_line(_main.sim)
		"stalled_run":
			Layouts.build_stalled_pipe_run(_main.sim)
		"understaffed":
			Layouts.build_understaffed_line(_main.sim)
		"shortfall":
			Layouts.build_underfunded_line(_main.sim)
		"full_dock":
			Layouts.build_full_dock_line(_main.sim)
		"unclerked_dock":
			# Like the fluid gallery, this one can be built and still not BE
			# itself: the whole subject is the window where `crewed` is
			# false and `understaffed` is not yet true, and a sim change
			# that closes that window would hand back a frame of a
			# perfectly ordinary dock. Refuse instead of photographing it.
			Layouts.build_unclerked_dock(_main.sim)
			if not Layouts.unclerked_dock_is_silent(_main.sim):
				printerr(
					"SCREENSHOT_FAIL: the unclerked dock is not in the silent "
					+ "window (issue #87) — it reads crewed, or it has already "
					+ "latched understaffed"
				)
				_refuse()
				return
		"demo":
			Layouts.build_demo_line(_main.sim)
		_:
			# Unreachable: `_initialize` refuses a name that is not in
			# `FIXTURE_NAMES`. If it ever fires, the two lists have drifted
			# — say so rather than photograph the demo line, which is how
			# a typo'd `--fixture` used to come back looking plausible.
			printerr(
				"SCREENSHOT_FAIL: fixture '%s' is in FIXTURE_NAMES with no arm here"
				% _fixture
			)
			_refuse()
			return
	# Every fixture is stocked, whether or not this run will tick: a
	# screenshot of a factory with an empty larder is a screenshot of a
	# factory that cannot start, and the larder strip would say so.
	Layouts.stock_larder(_main.sim)
	_main._rebuild_world()
	_main._refresh_owned_counts()
	# A fixture that TICKS its way to a state — the bottleneck, understaffed
	# and full-dock lines all do — banks a whole run's earnings and settles a
	# whole run's contract windows between two polls of the clipboard. Left
	# alone, the next poll announces the lot at once: a "+9308" money flyout
	# and a "+1000" payout burst, both of them sitting over the dock line the
	# shot came for, for twenty payments the player never watched arrive.
	# Both counters seed themselves on their first poll (`main.gd`'s
	# `_flash_money_change` and `_detect_payouts`: nothing to compare against
	# yet) and this asks for that same seeding again, now that the world has
	# moved. Deliberately NOT done for `--populate`/`--payout`, which tick
	# from here on with the panel polled as they go, so their celebrations
	# are of payments that happened in frame.
	_main._last_money = INF
	_main._payout_seen.clear()
	_reframe()


## Re-runs the isometric camera's own auto-framing against the world as it now
## stands. The scene frames once at startup, deliberately (re-framing on every
## placement would yank the view around while the player builds), so a
## screenshot that builds a factory after `_ready` has to ask for it.
func _reframe() -> void:
	_main._frame_bounds = _main._framing_bounds()
	_main._camera_focus = Vector3(
		(float(_main._frame_bounds.position.x) + float(_main._frame_bounds.size.x) * 0.5) * _main.TILE,
		_main.TILE * 0.35,
		(float(_main._frame_bounds.position.y) + float(_main._frame_bounds.size.y) * 0.5) * _main.TILE
	)
	_main._camera_base_size = _main._framing_size(_main._camera.transform.basis)
	_main._camera_size = _main._camera_base_size
	_main._update_camera_transform()


## `--closeup TX,TY[,SIZE]` — the argument is one comma-separated token so
## it survives the `--` passthrough without an argument-order convention.
## Both `--closeup 9,-2.5,14` and `--closeup=9,-2.5,14` are accepted, because
## `_string_arg` takes the `=` form and two argument styles in one script
## disagreeing about it is a trap (issue #114).
##
## **A `--closeup` this cannot read stops the run.** It used to fall through
## and render a whole-factory frame at default zoom with `SCREENSHOT_OK:` and
## exit 0, so the reviewer measured a picture nobody asked for — the one
## failure a screenshot review cannot catch by looking. Returns false when it
## refused.
func _parse_closeup(args: Array) -> bool:
	for i in range(args.size()):
		var arg := String(args[i])
		if not arg.begins_with("--closeup"):
			continue
		var value := ""
		if arg == "--closeup":
			if i + 1 < args.size():
				value = String(args[i + 1])
		elif arg.begins_with("--closeup="):
			value = arg.substr("--closeup=".length())
		else:
			printerr(
				"SCREENSHOT_FAIL: unknown flag '%s'. Did you mean --closeup TX,TY[,SIZE]?" % arg
			)
			_refuse()
			return false
		var parts: PackedStringArray = value.split(",")
		if value == "" or parts.size() < 2:
			printerr(
				"SCREENSHOT_FAIL: --closeup wants TX,TY[,SIZE] as one comma-separated token, got '%s'"
				% value
			)
			_refuse()
			return false
		_closeup = true
		# Fractional on purpose: a junction sits ON a tile boundary, so the
		# frame worth photographing is usually "half a tile past the belt".
		_closeup_tile = Vector2(float(parts[0]), float(parts[1]))
		if parts.size() >= 3:
			_closeup_size = float(parts[2])
		return true
	return true


## The two framing choices `--fluid-gallery` makes for itself, applied after
## `_resolve_fixture` because that is the first point at which the settled
## fixture name is known (`_pick_fixture` only records a REQUEST, so that an
## explicit `--fixture` can still win and say so). Both are defaults: an
## explicit `--closeup` was already parsed above and is left alone, and
## `--no-hud` cannot be un-asked-for anyway.
##
##   * The scene's auto-framing pads a world out to EMPTY_WORLD_FRAME_TILES
##     on both axes, which is right for a factory and wrong for this: the
##     gallery is 17 tiles long and 3 deep, so the padding is most of the
##     frame and the six colours the shot exists to compare come back a few
##     pixels wide each.
##   * The HUD goes, for the reason `_no_hud` already records one step
##     further on: the subject is a line of pipe running corner to corner,
##     and the legend and the clipboard sit on both ends of that diagonal.
##     There is no framing that shows six runs and misses both panels.
func _apply_fluid_gallery_defaults(args: Array) -> void:
	if _fixture != "fluid_gallery":
		return
	if not _closeup:
		_closeup = true
		_closeup_tile = FLUID_GALLERY_FOCUS
		_closeup_size = FLUID_GALLERY_SIZE
	_no_hud = true


## Aims the isometric camera at one tile, close in. Deliberately writes
## `_camera_focus`/`_camera_size` and calls the scene's own
## `_update_camera_transform` rather than posing the Camera3D directly, so
## what this photographs is the same camera the game uses (UI_UX.md §1's
## fixed angle included) at a zoom the player could also reach.
func _enter_closeup() -> void:
	_main._camera_focus = Vector3(
		_closeup_tile.x * _main.TILE, _main.TILE * 0.35, _closeup_tile.y * _main.TILE
	)
	_main._camera_size = _closeup_size
	_main._update_camera_transform()


func _enter_first_person() -> void:
	_main._set_view_mode(_main.ViewMode.FIRST_PERSON)
	# Higher up and looking a bit further up than the default entry pose, so
	# the sky/horizon/ground extent — the thing being reviewed — is actually
	# in frame, not just a close-up of one machine.
	_main._fp_position += Vector3(0.0, 4.0, 0.0)
	_main._fp_pitch = deg_to_rad(5.0)
	_main._update_fp_camera_transform()


## Warps the real OS cursor over one world tile, using the same tile ->
## world -> screen math `_tile_under_screen_pos` inverts (via the camera's own
## `unproject_position`, which needs the camera actually inside the tree — see
## the `_ready()` timing note in `_initialize` above, which is exactly why
## this can't run any earlier than `_process`). Every --hover* flag is this
## one function plus a different way of picking the tile.
##
## The tile is REMEMBERED (`_cursor_tile`), because a warp is a request to the
## window server rather than a fact: `_settle_cursor` makes it again just
## before the shutter and `_check_cursor` proves it stuck. See the cursor note
## in this file's header.
##
## Returns false having REFUSED if the tile is not in frame. In isometric the
## pointer cannot be put outside the window — the window server clamps the
## warp to the edge — so the shutter check would fire a few frames later and
## blame a hand on the mouse for a tile the camera simply is not showing
## (`--fixture chain --closeup 26,0,8 --hover-hopper` did exactly that). A
## cause this run CAN see, it names.
func _hover_over_tile(tile: Vector2i) -> bool:
	if not _in_first_person():
		var world_point: Vector3 = _main._tile_origin(
			tile.x, tile.y, _main.BELT_SURFACE_Y
		)
		var point: Vector2 = _main._camera.unproject_position(world_point)
		var frame: Rect2 = root.get_visible_rect()
		if not frame.has_point(point):
			printerr(
				"SCREENSHOT_FAIL: tile %s is not in frame — it projects to %s in a \
%dx%d window, and the pointer cannot be put outside that, so the tooltip would \
belong to whatever the edge of the frame is over. Widen the shot (a bigger \
--closeup size, or drop it), raise the resolution, or aim at something the \
camera is showing."
				% [str(tile), str(point.round()), int(frame.size.x), int(frame.size.y)]
			)
			_refuse()
			return false
	_cursor_tile = tile
	_cursor_hovered = true
	_aim_at_tile(tile)
	return true


## Puts whatever this view's tooltip is driven by over `tile`: the pointer in
## isometric, the camera itself in first person. One function so no caller has
## to know which view it is in, and so `_settle_cursor` re-asserts the same
## thing `_hover_over_tile` asserted.
func _aim_at_tile(tile: Vector2i) -> void:
	if _in_first_person():
		_aim_crosshair_at_tile(tile)
	else:
		_warp_to_tile(tile)


func _warp_to_tile(tile: Vector2i) -> void:
	var world_point: Vector3 = _main._tile_origin(tile.x, tile.y, _main.BELT_SURFACE_Y)
	Input.warp_mouse(_main._camera.unproject_position(world_point))


## Turns the first-person camera until the tile is dead centre, which is the
## only way to aim a tooltip in that view — warping the pointer there does
## nothing at all, and while the mouse is captured Godot does not even move it.
##
## The maths is the inverse of `main.gd`'s `_fp_look_basis`: the camera looks
## down its own -Z through `Basis(UP, yaw) * Basis(RIGHT, pitch)`, so a
## direction `d` is centred by `yaw = atan2(-d.x, -d.z)` and
## `pitch = asin(d.y / |d|)` — the same pair `_set_view_mode` computes when it
## enters the view. Pitch is clamped exactly where mouse-look clamps it, so a
## tile almost directly underfoot cannot be centred; this aims as close as the
## clamp allows and leaves `_check_cursor` to notice, rather than quietly
## handing back a frame aimed near the thing that was asked for.
func _aim_crosshair_at_tile(tile: Vector2i) -> void:
	var target: Vector3 = _main._tile_origin(tile.x, tile.y, _main.BELT_SURFACE_Y)
	var to_target: Vector3 = target - _main._fp_position
	if to_target.length() < 0.001:
		return
	_main._fp_yaw = atan2(-to_target.x, -to_target.z)
	_main._fp_pitch = clampf(
		asin(clampf(to_target.y / to_target.length(), -1.0, 1.0)),
		-_main.FP_PITCH_LIMIT,
		_main.FP_PITCH_LIMIT
	)
	_main._update_fp_camera_transform()


## Where the tooltip in THIS frame is queried from — the screen centre in
## first person, the pointer otherwise.
##
## It mirrors `main.gd`'s `_update_hover_tooltip` (the `_view_mode ==
## ViewMode.FIRST_PERSON` branch), and mirroring it is the whole point: a
## harness that asserts on a position the frame does not use can only ever be
## confidently wrong. If that branch in main.gd ever changes, this is the
## other half that has to change with it — there is no third copy.
func _hover_query_pos() -> Vector2:
	if _in_first_person():
		return root.get_visible_rect().size / 2.0
	return root.get_mouse_position()


func _in_first_person() -> bool:
	return _main != null and _main._view_mode == _main.ViewMode.FIRST_PERSON


## "Nowhere" — `hover_targets.gd`'s own sentinel, re-exported under the name
## this file has always used for it. Everything the hover flags need comes
## out of `world_layout_json`, so none of them types a tile in, which is what
## keeps them meaningful when a fixture moves its dock.
const HOVER_NONE := HoverTargets.NONE


## Every placement in the world, as the hover table reads it.
func _layout() -> Array:
	return JSON.parse_string(_main.sim.world_layout_json())


func _first_tile_of(predicate: Callable) -> Vector2i:
	return HoverTargets.first_matching(_layout(), predicate)


func _first_machine_tile() -> Vector2i:
	return HoverTargets.first_machine(_layout())


## Aims at whatever each --hover* flag on the command line asked for, and
## REFUSES if one of them cannot find its target.
##
## The refusal is the point, and it is why this is a loop over a table
## instead of seven `_hover_over_x()` functions. Each of those was guarded by
## `if tile != HOVER_NONE`, so a flag whose target was not in the fixture
## aimed at nothing at all and the run went on to photograph the parked
## cursor and report success: `--fixture full_dock --hover-splitter` gave
## back a clean frame of the dock, `SCREENSHOT_OK`, exit 0, and no line
## anywhere saying the flag had been dropped. A frame that answers a
## different question than the one asked, at a zero exit status, is issue
## #70's failure wearing another hat — and this file's own header promises
## the opposite (a frame is written only if the cursor really is over what it
## aimed at). One loop, one refusal, and an eighth flag is a row in
## `hover_targets.gd` that cannot be added without both.
##
## Order is the table's, so when more than one is given the last one wins the
## cursor — the precedence these flags have always had.
func _apply_hover_flags() -> void:
	var layout := _layout()
	for row in HoverTargets.rows_for(_hover_flags, _main._footprint_size):
		var tile: Vector2i = HoverTargets.tile_for(row, layout)
		if tile == HOVER_NONE:
			printerr(HoverTargets.refusal(row, _fixture, _flag_fixture_flag, layout))
			_refuse()
			return
		if not _hover_over_tile(tile):
			# Out of frame; `_hover_over_tile` has said so and refused. Named
			# here too, because which FLAG asked is only knowable from here.
			printerr("SCREENSHOT_NOTE: that tile is what %s asked for — %s"
				% [row["flag"], row["looks_for"]])
			return


## Screen positions `_park_cursor` tries, in this order, as fractions of the
## viewport: the four corners first, then the edge midpoints. Corners first
## because the factory is framed in the middle, so a corner is the likeliest
## patch of empty floor — and because a parked cursor near the middle of the
## frame would sit over the machines the shot came for the moment anything is
## built one tile further out.
const PARK_POINTS: Array[Vector2] = [
	Vector2(0.03, 0.04), Vector2(0.97, 0.04), Vector2(0.03, 0.96), Vector2(0.97, 0.96),
	Vector2(0.5, 0.04), Vector2(0.5, 0.96), Vector2(0.03, 0.5), Vector2(0.97, 0.5),
]

## How far around a candidate point the ground must also be empty, in pixels.
## A warp lands on whole pixels and `unproject_position` does not, so a point
## chosen right on a tile boundary could park a pixel over into the machine
## next door.
const PARK_CLEARANCE_PX := 4.0


## Puts the cursor somewhere that says nothing, and records where.
##
## Called on every run before anything opens (and again from
## `_settle_cursor`), because the pointer this script inherits is the one the
## LAST run left behind — see the cursor note in this file's header. Without
## it, `--payout` photographs the dock tooltip of the `--hover-dock` run
## before it.
##
## "Says nothing" is asked of the scene itself (`_hover_text_at_tile`, the
## same query the tooltip is drawn from) rather than derived here, so a kind
## of building that starts talking in future is covered without this knowing
## about it. The first candidate that is clear WITH ITS SURROUNDINGS wins.
##
## If the factory covers every one of them the run SAYS SO and parks on the
## first anyway (`_cursor_park_noise`), and the shutter check then lets that
## frame through: a tooltip this run named in its own output is explained,
## not the leftover from the last run that issue #4 is about. Refusing there
## instead would turn a dense fixture into a hard failure on EVERY
## screenshot, blamed on a pointer nothing had touched. The eight shipped
## points do not reach it today — `--stress` at both review resolutions still
## parks in a corner — but the frames only get busier.
func _park_cursor() -> void:
	if _menu:
		return
	_cursor_hovered = false
	# First person parks nothing: the crosshair is the pointer there, and it
	# is wherever the pose this run was asked for is looking. Moving the
	# camera to quieten the tooltip would be rewriting the shot.
	if _in_first_person():
		_cursor_tile = _main._tile_under_screen_pos(_hover_query_pos())
		if _main._hover_text_at_tile(_cursor_tile) == "":
			_cursor_park_noisy = false
		else:
			_note_park(
				_cursor_tile,
				"the first-person crosshair is looking at tile %s" % _cursor_tile
			)
		return
	var size: Vector2 = root.get_visible_rect().size
	for fraction in PARK_POINTS:
		var point := Vector2(fraction.x * size.x, fraction.y * size.y)
		if not _ground_is_quiet_around(point):
			continue
		Input.warp_mouse(point)
		_cursor_tile = _main._tile_under_screen_pos(point)
		_cursor_park_noisy = false
		return
	var fallback := Vector2(PARK_POINTS[0].x * size.x, PARK_POINTS[0].y * size.y)
	Input.warp_mouse(fallback)
	_cursor_tile = _main._tile_under_screen_pos(fallback)
	_note_park(
		_cursor_tile,
		"nowhere on screen is empty ground, so the cursor is parked on tile %s" % _cursor_tile
	)


## Records — and, the first time it is true of a given tile, announces — that
## the park had to settle for somewhere that talks. Said once rather than
## once per park, since `_settle_cursor` parks a second time and two
## identical NOTEs read like two different problems.
func _note_park(tile: Vector2i, why: String) -> void:
	if not _cursor_park_noisy or tile != _cursor_park_noise_tile:
		printerr(
			"SCREENSHOT_NOTE: %s; the tooltip in this frame is that tile's, " % why
			+ "put there by this run and not left over from the last one"
		)
	_cursor_park_noisy = true
	_cursor_park_noise_tile = tile


func _ground_is_quiet_around(point: Vector2) -> bool:
	for offset in [Vector2.ZERO, Vector2(PARK_CLEARANCE_PX, 0.0), Vector2(-PARK_CLEARANCE_PX, 0.0),
			Vector2(0.0, PARK_CLEARANCE_PX), Vector2(0.0, -PARK_CLEARANCE_PX)]:
		var tile: Vector2i = _main._tile_under_screen_pos(point + offset)
		if _main._hover_text_at_tile(tile) != "":
			return false
	return true


## Makes the cursor's placement true again a few frames before the shutter.
##
## The warp happens on frame 2 and the frame is written on WARMUP_FRAMES; in
## between, anything that moves the machine's one pointer moves this shot —
## a parallel worktree taking its own screenshot, a hand on the mouse, the
## window server settling. Re-asserting costs a warp and removes the whole
## class of "sometimes it points at the wrong thing".
##
## Re-PARKING (rather than re-warping to the remembered tile) is what makes
## `--closeup` safe: the closeup re-aims the camera on frame 2, after the
## parking on frame 1, so the tile that was empty floor in the wide shot can
## be a machine filling half the zoomed frame.
##
## The tooltip is then refreshed by hand because `--payout`, `--crushing` and
## `--full-dock` FREEZE the scene (`_main.set_process(false)`), so main.gd is
## not polling anything by now and would keep drawing the tooltip it last
## drew — which is the stale-numbers half of issue #4.
func _settle_cursor() -> void:
	if _menu:
		return
	if _cursor_hovered:
		_aim_at_tile(_cursor_tile)
	else:
		_park_cursor()
	_main._update_hover_tooltip()


## Whether the frame about to be written is of the cursor it was asked for.
##
## The three claims are different, and each is checked as itself. A --hover*
## flag claims a particular tile, so the tile the tooltip is queried from
## must BE that tile. A quiet park claims only that nothing is under it, so
## what is checked is that the scene has nothing to say there — which is also
## immune to the pixel of rounding a warp introduces, where an exact tile
## comparison on a tile boundary would fail for no reason. A park that had to
## settle for a tile that talks claims nothing beyond the NOTE it already
## printed, and is allowed through on that tile alone.
##
## What gets asked is `_hover_query_pos`, not the pointer: in first person
## main.gd queries the screen centre, and a check on any other quantity is a
## check on something that is not in the frame.
##
## When it refuses it reports WHAT it saw and stops there. The causes it used
## to name (a parallel worktree, a hand on the mouse) are guesses this
## process cannot distinguish, and a harness that states a guess as the cause
## is the defect issues #4 and #70 were both filed about; the candidates are
## listed as candidates.
func _check_cursor() -> bool:
	if _menu:
		return true
	var tile: Vector2i = _main._tile_under_screen_pos(_hover_query_pos())
	var aimed := "crosshair" if _in_first_person() else "cursor"
	if _cursor_hovered:
		if tile != _cursor_tile:
			printerr(
				"SCREENSHOT_FAIL: the hover flag aimed the %s at tile %s, but when the "
				% [aimed, _cursor_tile]
				+ "shutter opened the tooltip was being read from %s — so this frame's "
				% tile
				+ "tooltip is not the one that was asked for (issue #4). Candidates, none "
				+ "of them checked here: " + _drift_candidates() + "."
			)
			return false
		print("SCREENSHOT_CURSOR: hovering tile %s" % _cursor_tile)
		return true
	if _cursor_park_noisy and tile == _cursor_park_noise_tile:
		print(
			"SCREENSHOT_CURSOR: parked on tile %s, whose tooltip is in the frame " % tile
			+ "on purpose — see the SCREENSHOT_NOTE above"
		)
		return true
	if _main._hover_text_at_tile(tile) != "":
		printerr(
			"SCREENSHOT_FAIL: nothing asked for a hover, but the tooltip is being read "
			+ "from tile %s, which has something to say, and this run parked on %s "
			% [tile, _cursor_tile]
			+ "(issue #4). Candidates, none of them checked here: "
			+ _drift_candidates() + "."
		)
		return false
	# FIRST_PERSON looking at the sky is the ordinary way to have no tile at
	# all under the cursor, and it is the quietest park there is.
	if tile == _main.HOVER_NO_TILE:
		print("SCREENSHOT_CURSOR: parked off the floor, no tile under it")
	else:
		print("SCREENSHOT_CURSOR: parked on empty ground at tile %s" % tile)
	return true


## The things that are known to move a shot's aim out from under it, offered
## as a list to check rather than as a finding. Which list depends on what
## does the aiming in this view — in first person the desktop's pointer is
## not in the picture at all, and saying it was is how this harness came to
## blame a parallel worktree for a camera it had aimed itself.
func _drift_candidates() -> String:
	if _in_first_person():
		return (
			"the pitch needed to centre that tile is steeper than mouse-look's own "
			+ "%d-degree clamp, " % int(round(rad_to_deg(_main.FP_PITCH_LIMIT)))
			+ "or something moved the first-person camera after the aim"
		)
	return (
		"a hand on the mouse, a second screenshot-scene.sh warping the shared pointer "
		+ "from another worktree, or a camera move after the aim"
	)


## Loads both consumables and ticks, so the shot shows product moving rather
## than an idle factory. Whatever was built is what runs; a fixture with no
## hopper simply has nothing to meter out.
func _run_chain_forward() -> void:
	for i in range(POPULATE_SPAWNS):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		for j in range(POPULATE_TICKS_PER_SPAWN):
			_main.sim.tick()
			# Applied EVERY tick, not once at the end, because `World::tick`
			# clears its dirty set at the START of each tick — so a batched
			# run reports only whatever the very last tick happened to touch
			# and the shot is drawn from that. It photographed a branch belt
			# as empty while the simulation held eight tomatoes on it, which
			# is exactly the kind of lie this whole script exists to catch.
			_main._apply_dirty_chunks()
			# The machine mechanisms. `main.gd` advances them at the end of
			# `_advance_simulation`, which this loop deliberately does not go
			# through (it wants ticks without a frame's worth of input, camera
			# and UI work), so without this line every screenshot ever taken
			# shows every machine's mechanism at rest — an idle-looking factory
			# photographed in the middle of a run, which is the same lie the
			# comment above is about. It reads the tick, so the shot stays
			# reproducible.
			#
			# The belt surface needs no line here, and calling one is an ERROR:
			# `main.gd::_update_belt_frame` was deleted by issue #79, which put
			# each tile's rubber on its own clock off the wire, inside the
			# `_apply_dirty_chunks()` above. This loop kept calling it until
			# 2026-09-21 and every frame of a forward run died on the
			# nonexistent method — silently, because Godot's runtime error only
			# unwinds the call — taking `_update_machine_motion`,
			# `_refresh_buffers` and `_update_fluid_network` with it.
			_main._update_machine_motion()
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()
	# The load records are polled from `_advance_simulation` too, which this
	# loop bypasses; read them once here so a hover in the shot shows the
	# settled verdicts, not the ones the rebuild saw before the run.
	_main._poll_machine_load()


## Enters placement mode with the cursor parked somewhere the ghost reads as
## valid (open ground) or invalid (on top of something already placed) — the
## two states `_update_ghost` distinguishes, and the only way to photograph
## either without a live session.
func _enter_ghost_mode() -> void:
	_main.sim.issue_command_json(JSON.stringify({
		"type": "purchase", "kind": _ghost_kind, "quantity": 1,
	}))
	_main._refresh_owned_counts()
	if _main._owned_of(_ghost_kind) <= 0:
		# The fixture spent the starting balance and this kind could not be
		# afforded, so there is nothing to ghost. Said out loud rather than
		# photographed as an empty frame — see this file's header on what a
		# starting balance buys.
		printerr("SCREENSHOT_NOTE: could not afford one '%s' after building the fixture — no ghost to show" % _ghost_kind)
		return
	_main._enter_build_mode(_ghost_kind)
	var target := _open_ground_tile()
	if _ghost_blocked:
		var occupied := _first_machine_tile()
		if occupied == HOVER_NONE:
			occupied = _first_tile_of(func(_p): return true)
		if occupied == HOVER_NONE:
			# The --hover* hole, in the one other place a flag aims at
			# something the world may not contain. With nothing placed there
			# is nothing for the ghost to overlap, and what used to come back
			# was a frame of the VALID state under the flag that asks for the
			# INVALID one — plausible, wrong, and exit 0.
			printerr(
				"SCREENSHOT_FAIL: --ghost-blocked needs something already placed for \
the ghost to sit on, and this world is empty. Ask for a fixture (--fixture, or a \
flag that implies one); --ghost %s on its own is the valid state."
				% _ghost_kind
			)
			_refuse()
			return
		target = occupied
	if not _hover_over_tile(target):
		return
	_main._update_ghost()


## A tile next to what has been built but not on it, so the valid ghost is in
## frame beside the factory rather than off in empty floor.
func _open_ground_tile() -> Vector2i:
	var bounds: Rect2i = _main._world_tile_bounds()
	return Vector2i(bounds.position.x + bounds.size.x + 1, bounds.position.y)


## Remove mode, with the cursor over something removable.
func _enter_remove_mode() -> void:
	_main._toggle_remove_mode()
	var tile := _first_machine_tile()
	if tile == HOVER_NONE:
		tile = _first_tile_of(func(_p): return true)
	if tile == HOVER_NONE:
		# As above: remove mode over an empty world photographs the mode
		# with nothing under the cursor to remove, which is not the shot the
		# flag asks for.
		printerr(
			"SCREENSHOT_FAIL: --remove-mode has nothing to stand over — this world is \
empty, so the frame would show remove mode with no target under the cursor. Ask \
for a fixture (--fixture, or a flag that implies one)."
		)
		_refuse()
		return
	if not _hover_over_tile(tile):
		return
	_main._update_ghost()


func _open_shop_menu() -> void:
	_main._set_shop_menu_open(true)


## `--config TX,TY`: the click that opens a hopper's or tank's panel, driven
## through the same tile-resolved entry point a real click reaches
## (`_config_click_at_tile`), so the shot proves the panel a click actually
## produces — positioned beside the building, with whichever FILL reason the
## current larder gives — rather than a widget merely made visible.
func _open_config_at(tile: Vector2i) -> void:
	_main._config_click_at_tile(tile)
	if not _main._config_open:
		# Same rule as the --contents guards above (issue #123): --config
		# aimed at a tile with nothing configurable on it used to print this
		# and still ship a SCREENSHOT_OK frame with no panel in it.
		printerr(
			"SCREENSHOT_FAIL: --config %s has no hopper or tank on it to "
			% str(tile)
			+ "configure, so no panel opened."
		)
		_refuse()
		return
	if _contents >= 0:
		_main._on_config_picker_pressed(_contents)
		if _contents_refill:
			_refill_declared_tank()


## Fills a tank back up after `--contents` declared what it holds, through the
## panel's own FILL button and the sim's own ticks — one crate is 30 units and
## the run drinks, so it takes several presses and a few ticks for the pipe run
## to carry anything worth warning about.
func _refill_declared_tank() -> void:
	var fluid: String = String(_main._tank_fluids[_contents]) if _contents < _main._tank_fluids.size() else ""
	if fluid == "":
		printerr(
			"SCREENSHOT_FAIL: --contents-refill: picker slot %d names no fluid "
			% _contents
			+ "(the panel has %d slot(s)) — nothing to buy or pump back."
			% _main._tank_fluids.size()
		)
		_refuse()
		return
	Layouts.fund(_main.sim, 5000)
	Layouts.stock(_main.sim, fluid, 256)
	for i in range(4):
		_main._on_config_fill_pressed()
	for i in range(5):
		_main.sim.tick()
	_main._apply_dirty_chunks()
	_main._update_fluid_network()


## `--mothballed`: switches the understaffed line's cooking vat off through
## the real command, then opens its panel through the real click — so the
## frame is of the state a player reaches by clicking the machine that is
## going without its crew and throwing the one switch that is there, not of
## a widget this script posed.
##
## **The level bar in this frame is empty, and that is correct.** Review
## asked for a HELD batch here; this fixture cannot produce one, because the
## vat is precisely the machine the understaffed line denies a crew, so it
## has never started a craft to hold. Delaying this call by any number of
## frames does not change that — nothing about the vat advances while it is
## uncrewed. What the frame is evidence for is the other half: `required`
## falls from 4 to 2 and the CREW plate follows it, naming `crusher, mixer`
## and no vat. The held-batch promise is pinned headlessly instead, by
## `sim_core/tests/mothball.rs`, which asserts `elapsed` stands still across
## a mothballing and the same batch finishes on the other side.
func _mothball_the_vat() -> void:
	var tile: Vector2i = Layouts.UNDERSTAFFED_VAT
	_main._config_click_at_tile(tile)
	if not _main._config_open:
		printerr("SCREENSHOT: no machine at %s to mothball" % str(tile))
		return
	_main._on_config_mothball_pressed()
	_main._refresh_buffers()
	_main._poll_machine_load()
	_main._update_config_panel()


## Drives the real chain across several contract cadence boundaries, feeding
## it exactly the way a player does. What puts product in the shared delivery
## pool the contracts draw from is the delivery dock at the end of the line,
## not a command issued from here.
func _run_contract_cycles() -> void:
	# A new game runs the starter only (the contract-board epic); the review
	# frame wants the whole board competing, so sign every offer first — the
	# full line costs 1087 of the 3000 start, which covers the signatures.
	for offer in JSON.parse_string(_main.sim.economy_snapshot_json()).get("board", []):
		if not bool(offer.get("active", false)):
			_main.sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": String(offer["id"])}))
	for i in range(CONTRACTS_REVIEW_TICK_LIMIT):
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		_main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		_main.sim.tick()
		if _contract_board_is_mixed():
			break
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()


## Runs the shipped line until a contract genuinely pays, then stops the clock
## partway through the burst it fires.
##
## The line is put on AUTOMATIC rather than hand-fed a command a tick: the two
## raw solids and the raw fluid all have to keep arriving for 2500 ticks, and
## `stock_larder` buys only the solids, so the vinegar is bought explicitly
## here. A line that stalls halfway photographs as a factory with nothing on
## its dock and no payout at all.
func _run_to_payout() -> void:
	Layouts.fund(_main.sim, 30000)
	Layouts.stock(_main.sim, "vinegar", 1000)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		_main.sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true}))
	_main.sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true}))

	for i in range(PAYOUT_REVIEW_TICK_LIMIT):
		_main.sim.tick()
		_main._update_contract_panel()
		if _main.payout_events > 0:
			break
	if _main.payout_events == 0:
		printerr("SCREENSHOT_NOTE: --payout ran %d ticks without a contract paying"
			% PAYOUT_REVIEW_TICK_LIMIT)

	# Advance to the chosen point in the arc. The whole effect is a function of
	# `sim.current_tick()`, so "hold the frame mid-burst" is ticking the
	# simulation, not sleeping — which is also why two runs of this flag give
	# the identical picture.
	for i in range(int(_main.COIN_FLIGHT_TICKS * PAYOUT_REVIEW_PHASE)):
		_main.sim.tick()
	_main._apply_dirty_chunks()
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()
	_main._update_payout_feedback()
	# Then FREEZE the scene. Every other flag sets up a state that persists
	# (a board, an open menu, a hovered tile), so the WARMUP_FRAMES the shot
	# waits out cost them nothing; a burst lasts COIN_FLIGHT_TICKS (34) and
	# `main._process` advances up to MAX_TICKS_PER_FRAME (8) a frame, so 43
	# more frames of it would leave an empty apron to photograph. Holding the
	# scene still is what "hold the frame mid-burst" means here, and the
	# review wants a still anyway.
	_main.set_process(false)


## Ticks until the crusher is working and has thrown pulp, then holds the
## frame in the air. `_update_payout_feedback` is what drives the spray (it
## carries `_update_crush_spray`), and `_run_chain_forward`'s loop does not
## go through it, so it is called per tick here exactly as the belt and
## mechanism animations are there.
func _run_to_crushing() -> void:
	var thrown := false
	for i in range(CRUSHING_REVIEW_TICK_LIMIT):
		_main.sim.tick()
		_main._apply_dirty_chunks()
		_main._update_belt_frame()
		_main._update_machine_motion()
		_main._update_payout_feedback()
		if _main._active_crush_count() > 0:
			thrown = true
			break
	if not thrown:
		printerr("SCREENSHOT_NOTE: --crushing ran %d ticks without the crusher throwing"
			% CRUSHING_REVIEW_TICK_LIMIT)
	for i in range(CRUSHING_REVIEW_HOLD_TICKS):
		_main.sim.tick()
		_main._apply_dirty_chunks()
		_main._update_belt_frame()
		_main._update_machine_motion()
		_main._update_payout_feedback()
	_main._refresh_buffers()
	_main._update_fluid_network()
	# Frozen for `--payout`'s reason: a fleck's whole flight is
	# CRUSH_FLIGHT_TICKS (9) and the warm-up frames that follow are longer.
	_main.set_process(false)


## Stops the clock on a tick where the delivery pool is at its ceiling AND
## the dock's lamps are on the wanted half of their pulse, then freezes.
##
## `Layouts.build_full_dock_line` has already run the line to full by the
## time this is called; the loop here is the alignment, and it re-checks
## fullness on every step rather than assuming it, because a contract that
## ships mid-hunt takes the pool off its ceiling for a cadence and would
## otherwise be photographed as a dock that merely holds some stock.
func _run_to_full_dock() -> void:
	var held := false
	for i in range(FULL_DOCK_HOLD_TICK_LIMIT):
		if Layouts.delivery_full(_main.sim) and _dock_lamp_on_wanted_half():
			held = true
			break
		_main.sim.tick()
		_main._apply_dirty_chunks()
		_main._update_belt_frame()
		_main._update_machine_motion()
	if not held:
		printerr("SCREENSHOT_NOTE: --full-dock never found a %s tick with the pool full — the shot is of whatever the line reached"
			% ("dark" if _dock_lamp_dark else "lit"))
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()
	# Frozen for `--payout`'s reason, and one step more so: the lamps read
	# `sim.current_tick()`, so eight more ticks a frame across the warm-up
	# would hand back a coin-flip on which half of the pulse is in frame —
	# the exact thing this flag exists to make deterministic.
	_main.set_process(false)


## Whether the dock's lamps are lit or dark this tick, matched against what
## `--dock-lamp` asked for. The rule itself is `main.gd`'s — this asks the
## scene rather than re-deriving the phase, so the threshold stays in one
## place (`godot_project/CLAUDE.md`: Godot renders, the sim decides, and a
## test script re-deriving a presentation rule is the same mistake).
func _dock_lamp_on_wanted_half() -> bool:
	var lit: bool = _main._dock_lamp_lit(true, true, _main.sim.current_tick())
	return lit != _dock_lamp_dark


func _contract_board_is_mixed() -> bool:
	var econ: Dictionary = JSON.parse_string(_main.sim.economy_snapshot_json())
	var stock := 0
	for pair in econ.get("delivery_stock", []):
		stock += int(pair[1])
	if stock < CONTRACTS_REVIEW_DOCK_UNITS:
		return false
	var made := 0
	var missed := 0
	for contract in econ.get("contracts", []):
		made += int(contract["deliveries_made"])
		missed += int(contract["deliveries_missed"])
	return made >= 1 and missed >= 1


## Presses a hotbar key for real, having first bought one of that slot's kind
## so the socket has something in it — an empty socket refuses selection, by
## design, and would photograph as nothing happening.
func _select_hotbar_key() -> void:
	var index: int = _main.HOTBAR_KEYCODES.find(_hotbar_key_code())
	if index < 0 or index >= _main._hotbar_slots.size():
		printerr("SCREENSHOT_NOTE: --hotbar '%s' is not a hotbar key" % _hotbar_key)
		return
	var kind: String = String(_main._hotbar_slots[index]["kind"])
	_main.sim.issue_command_json(JSON.stringify({
		"type": "purchase", "kind": kind, "quantity": 1,
	}))
	_main._refresh_owned_counts()
	var event := InputEventKey.new()
	event.keycode = _hotbar_key_code()
	event.pressed = true
	_main._unhandled_input(event)
	# Park the cursor on open ground so the ghost selection put in the
	# player's hand reads as valid rather than blocked.
	if not _hover_over_tile(_open_ground_tile()):
		return
	_main._update_ghost()
	_main._update_hotbar()


## An assorted kit, so a review frame shows every socket state at once. The
## quantities are picked to include a two-digit count (belts), single-digit
## counts, and several kinds left at zero.
func _stock_the_hotbar() -> void:
	for pair in [["belt", 24], ["pipe", 6], ["hopper", 1], ["fluid_tank", 1], ["crusher", 2]]:
		_main.sim.issue_command_json(JSON.stringify({
			"type": "purchase", "kind": pair[0], "quantity": pair[1],
		}))
	# Raw stock too, and deliberately UNEVEN: the STOCK plate (the HUD ticket)
	# is reviewed for whether a stocked row and an out-of-stock one can be
	# told apart in the same glance, which a plate of three equal numbers —
	# or, after a fixture has run its larder dry, three identical `none`s —
	# cannot show. Vinegar is left unbought on purpose; it is the zero.
	Layouts.stock(_main.sim, "tomato", 42)
	Layouts.stock(_main.sim, "sugar", 7)
	_main._refresh_owned_counts()
	_main._update_hotbar()
	_main._update_larder()


func _hotbar_key_code() -> int:
	if _hotbar_key == "0":
		return KEY_0
	if _hotbar_key.length() == 1 and _hotbar_key >= "1" and _hotbar_key <= "9":
		return KEY_1 + (_hotbar_key.to_int() - 1)
	return 0


## Writes one real save so `--menu` photographs a list with a row in it
## rather than the empty state. Removed again once the frame is written.
func _write_menu_review_save() -> void:
	var sim = SimBridge.new()
	Layouts.build_machines_fixture(sim)
	Layouts.stock_larder(sim)
	for i in range(200):
		sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
		sim.tick()
	if GameSaves.write_slot(MENU_REVIEW_SLOT, String(sim.save_ron())) == "":
		_menu_review_path = GameSaves.slot_path(MENU_REVIEW_SLOT)


## `--menu` shows the LOAD view, not the three buttons: the buttons are the
## easy half and the list is the half with layout worth checking.
func _open_menu_load_view() -> void:
	_main._on_load_pressed()


## `--item-gallery`: one finished-goods model per gallery slot, standing
## mid-tile on the bare belt run the fixture placed.
##
## Straight through the scene's own item drawing — the same pooled multimesh,
## the same transform, the same materials a belt item gets — because the
## question this shot answers is "can a player tell these apart on a belt",
## and a model posed by the test would be answering a different one.
func _stand_item_gallery() -> void:
	for slot in range(ITEM_GALLERY_ASSETS.size()):
		_main._show_carried_item(
			"item_gallery_%d" % slot,
			String(ITEM_GALLERY_ASSETS[slot]),
			_main._carried_item_transform(Layouts.item_gallery_tile(slot), "east", 0.5)
		)


## `--pause`: the in-game menu, over a factory that has been running.
func _open_pause_menu() -> void:
	_main._set_pause_open(true)


func _process(_delta: float) -> bool:
	if _refused:
		return true
	_frames += 1
	# Build first: every flag below is about looking at something, and since
	# the "Real Placement" epic there is nothing to look at until one of the
	# fixtures has been placed.
	if _menu:
		if _frames == 1 and not _title:
			_open_menu_load_view()
		if _frames < WARMUP_FRAMES:
			return false
		return _write_frame()
	if _frames == 1 and _fixture != "":
		_build_fixture()
		# `_refuse()` inside the build latches and calls `quit(1)`, but quit
		# only takes effect between frames — without this the rest of THIS
		# pass would go on to open panels and, at the last frame, write the
		# PNG that was just refused.
		if _refused:
			return true
		# A fixture that could not build what it promised. `Layouts` latches
		# rather than asserting, because `assert()` does not halt the calling
		# chain here — the marker printed, the fixture ran on and the run
		# reported SCREENSHOT_OK with a dead branch in frame (issue #110).
		var complaint: String = Layouts.fixture_failure()
		if complaint != "":
			printerr(
				"SCREENSHOT_FAIL: the '%s' fixture did not build what it promises: %s"
				% [_fixture, complaint]
			)
			_refuse()
			return true
	# Straight after the fixture and before anything opens a panel: the gate
	# decides what the market, the larder and the board have in them, so it
	# has to be settled before they are drawn.
	if _frames == 1 and (_unlocked or _unlock_toast):
		_cross_gates(-1 if _unlock_toast else 0)
	if _frames == 1 and _first_person:
		_enter_first_person()
	# Before main.gd has drawn a single tooltip, and before any --hover* flag
	# on frame 2 aims it somewhere on purpose: the pointer this run starts
	# with is the one the run before it left behind (issue #4).
	if _frames == 1:
		_park_cursor()
	# Before any flag below acts, and in particular before --unlock-toast
	# crosses the gate: frame 1 built the fixture, and every banner main.gd
	# raised while it did is about the build, not about the world in frame
	# (issue #141).
	if _frames == 2 and _fixture != "":
		_clear_build_notices()
	if _frames == 2 and _populate:
		_run_chain_forward()
	# The closeup is applied BEFORE any hover flag: a hover warps the OS cursor
	# to where a tile currently projects on screen, so re-aiming the camera
	# afterwards would leave the cursor pointing at whatever moved under it.
	if _frames == 2 and _closeup:
		_enter_closeup()
	if _frames == 2 and _item_gallery:
		_stand_item_gallery()
	if _frames == 2 and not _hover_flags.is_empty():
		_apply_hover_flags()
		# Straight out, rather than letting the rest of this frame set up a
		# shot nothing will be written from: a refusal's own message is the
		# only output a refused run should have to be read for.
		if _refused:
			return true
	if _frames == 2 and _ghost_kind != "":
		_enter_ghost_mode()
		if _refused:
			return true
	if _frames == 2 and _remove_mode:
		_enter_remove_mode()
		if _refused:
			return true
	if _frames == 2 and _shop:
		_open_shop_menu()
	if _frames == 2 and _no_hud:
		var hud := _main.get_node_or_null("HUD")
		if hud != null:
			hud.visible = false
	# The crossing itself, with the HUD already primed one delivery short —
	# which is the only way the announcement fires, and is exactly what a
	# playing session does when its tenth delivery settles.
	if _frames == 2 and _unlock_toast:
		_cross_gates(0)
		_main._update_contract_panel()
		_main._update_larder()
		# Checked at the moment it is raised, not only at the shutter, so a
		# run that lost it says which banner took the slot instead.
		if not _check_unlock_toast():
			_refuse()
			return true
	if _frames == 2 and _market:
		_main._set_market_open(true)
	if _frames == 2 and _board:
		_main._set_contract_board_open(true)
	# A frame later than the open, deliberately: the offers are built and the
	# ScrollContainer sized in the same update that opens the board, and its
	# scroll ceiling is whatever the layout pass has settled on by then.
	if _frames == 3 and _board and _board_end:
		_scroll_board_to_end()
	if _frames == 2 and _staff:
		_main._set_staff_open(true)
	if _frames == 2 and _production:
		_main._set_production_open(true)
	if _frames == 2 and _understaffed:
		if not _hover_over_tile(Layouts.UNDERSTAFFED_VAT):
			return true
	if _frames == 2 and _stalled_run:
		if not _hover_over_tile(Layouts.STALLED_RUN_TILE):
			return true
	if _frames == 2 and _staff_shortfall:
		_main._set_staff_open(true)
	if _frames == 2 and _contracts:
		_run_contract_cycles()
	if _frames == 2 and _payout:
		_run_to_payout()
	if _frames == 2 and _crushing:
		_run_to_crushing()
	if _frames == 2 and _full_dock:
		_run_to_full_dock()
	if _frames == 2 and _stock:
		_stock_the_hotbar()
	# Through the real setter, not by hiding the label: the point of the shot
	# is what the collapse leaves behind, and only the setter resizes the
	# plate and rewrites the button.
	if _frames == 2 and _legend_collapsed:
		_main._set_legend_collapsed(true)
	if _frames == 2 and _hotbar_key != "":
		_select_hotbar_key()
	# After --stock, so a stocked larder photographs FILL enabled and an
	# empty one photographs it blocked with its reason.
	if _frames == 2 and _config:
		_open_config_at(_config_tile)
	if _frames == 2 and _mothballed:
		_mothball_the_vat()
	# Last, so it draws over whatever else the flags set up — which is
	# exactly what a review of it wants to see.
	if _frames == 3 and _pause:
		_open_pause_menu()
	# Late enough that everything above has settled (the closeup has re-aimed
	# the camera, a frozen scene has stopped), early enough that main.gd's own
	# processing gets frames to redraw with.
	if _frames == CURSOR_SETTLE_FRAME:
		_settle_cursor()
	if _frames < WARMUP_FRAMES:
		return false
	if not _check_cursor():
		_refuse()
		return true
	# Again here: the notice's dwell is wall clock (issue #82), so a slow
	# machine could fade the announcement out between frame 2 and the shutter
	# and leave the PNG showing an empty strip.
	if _unlock_toast and not _check_unlock_toast():
		_refuse()
		return true
	return _write_frame()


## Winds the board's own scroll container to the bottom — the same control a
## player's wheel drives, so the shot is of a board a player could be looking
## at rather than of a panel this script rearranged.
func _scroll_board_to_end() -> void:
	var scroll: ScrollContainer = _main._board_scroll
	if scroll == null:
		printerr("SCREENSHOT_FAIL: --board-end, but the board has no scroll container")
		return
	var bar := scroll.get_v_scroll_bar()
	if bar == null or bar.max_value <= bar.page:
		printerr(
			"SCREENSHOT_FAIL: --board-end, but the board already fits on one "
			+ "screen — there is nothing below the fold to scroll to"
		)
		return
	scroll.scroll_vertical = int(bar.max_value)


## Reads the factory in frame out of the simulation and prints it as one
## greppable `SCREENSHOT_HEALTH:` line, returning a complaint when the shot is
## of something broken (issue #94).
##
## Every screenshot in a PR body is evidence, and the evidence used to be
## unchecked: the `--bbq` fixture spent a wave rendering a frame with a
## CONTRACT LOST banner in it, `SCREENSHOT_OK:` printed and the harness exited
## 0 — only reading the PNG caught it. A lost contract and a line that made
## nothing are the two states that mean "this factory is not running", and
## both are cheap to read off the snapshots the HUD already uses.
##
## Reported for every fixture, enforced for `HEALTHY_FIXTURES` and waived by
## `--allow-unhealthy`.
func _frame_health() -> String:
	if _menu or _fixture == "" or _main == null or _main.sim == null:
		return ""
	var economy = JSON.parse_string(_main.sim.economy_snapshot_json())
	var production = JSON.parse_string(_main.sim.production_snapshot_json())
	if not (economy is Dictionary) or not (production is Dictionary):
		return "the economy/production snapshot did not parse"
	var active := 0
	var lost := 0
	for offer in economy.get("contracts", []):
		if bool(offer.get("active", false)):
			active += 1
		lost += int(offer.get("times_lost", 0))
	var made := PackedStringArray()
	var produced := 0
	for item in production.get("items", []):
		var total := int(item.get("produced_total", 0))
		if total > 0:
			produced += total
			made.append("%s:%d" % [String(item.get("id", "?")), total])
	print("SCREENSHOT_HEALTH: contracts=%d lost=%d made=%s" % [
		active, lost, ",".join(made) if made.size() > 0 else "nothing",
	])
	if _allow_unhealthy or not HEALTHY_FIXTURES.has(_fixture):
		return ""
	if lost > 0:
		return (
			"the '%s' fixture lost %d contract(s) before the shutter — " % [_fixture, lost]
			+ "the frame has a factory in trouble in it, not the working line this shot claims"
		)
	if produced == 0:
		return "the '%s' fixture produced nothing at all during its settle" % _fixture
	return ""


func _write_frame() -> bool:
	var complaint := _frame_health()
	if complaint != "":
		printerr("SCREENSHOT_FAIL: %s" % complaint)
		printerr("  (--allow-unhealthy photographs it anyway, for a shot that means to.)")
		_refuse()
		return true
	var image := root.get_texture().get_image()
	if _menu_review_path != "" and FileAccess.file_exists(_menu_review_path):
		DirAccess.remove_absolute(_menu_review_path)
	if image == null:
		printerr("SCREENSHOT_FAIL: no viewport image (is this running with --headless?)")
		return true
	var err := image.save_png(_out)
	if err != OK:
		printerr("SCREENSHOT_FAIL: could not write %s (error %d)" % [_out, err])
		return true

	# What the frame cost the renderer, in the renderer's own terms. Objects
	# is what instancing collapses (one MultiMesh is one object however many
	# tiles it draws); draw calls follow it; primitives do not move, since
	# the same voxels are drawn either way.
	print("SCREENSHOT_RENDER: objects %d, primitives %d, draw calls %d, in frame %d" % [
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_OBJECTS_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		_frames,
	])
	print("SCREENSHOT_OK: %s (%dx%d)" % [_out, image.get_width(), image.get_height()])
	return true
