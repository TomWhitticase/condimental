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
# **THE WINDOW IS REAL BUT OFF-SCREEN** (`_hide_window`). This runs on the
# laptop someone is working at, so the window must never land on top of what
# they are doing. It is made borderless and no-focus and moved far off the
# desktop in `_initialize`. macOS clamps a titled window back on-screen, and
# `--position` is clamped the same way, which is why this is done from here.
# Godot skips drawing a window macOS reports as occluded, so while it is off
# there `_process` draws each frame itself (`_keep_drawing`), and the frame
# written is identical to an on-screen one. Godot still takes focus when it
# starts, and nothing inside the process can stop that: `show_window`
# activates the app unconditionally. scripts/screenshot-scene.sh hands focus
# back to whatever was in front. `--on-screen` keeps the window where Godot
# put it, for watching a run.
#
# Run via scripts/screenshot-scene.sh, or directly:
#   godot --path godot_project --fixed-fps 60 --script test_screenshot.gd -- /abs/out.png [flags]
#
# THE SAME COMMAND TWICE IS THE SAME PICTURE, AND `--fixed-fps` IS WHY
# (issue #82). The fixtures below are deterministic — each ticks a fixed
# number of times — but the warm-up that follows them is counted in FRAMES,
# and main.gd advances the simulation on each frame's DELTA. On a free
# clock that delta is real elapsed time, so the ticks between "fixture
# built" and "shutter" are a property of the machine's load: two renders of
# `--bottleneck --closeup 4,0,10 --no-hud` came back one tick and several
# moved tomatoes apart, and six of eight glow variants failed to reproduce
# in the sweep that found it. `--fixed-fps` hands every frame the same
# delta instead, so the shot is a function of its arguments; this script
# REFUSES a run that arrives without it (`_clock_is_pinned`), and prints
# the tick it fired on (`SCREENSHOT_TICK`) so a pair that ever does drift
# says so in its own output.
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
# `--mustard` builds the MUSTARD LINE (test_layouts.gd's
# `build_mustard_line`: the third sauce, a whole SECOND line from raw rather
# than a branch, funded, crewed and run until cases are shipping). Implies
# `--populate`. It is the one fixture whose middle is entirely WET — the
# crusher mills seed with vinegar and discharges a fluid, so `mustard_mash`
# and `mustard_mix` are only ever seen in a pipe — which makes it the only
# frame that can show whether mustard's three stages are three colours.
# Parsed after `--bbq`, so `--cookbook-page mustard --mustard` is the book
# open on the new sauce over the line that made it.
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
# `--cookbook` opens THE COOKBOOK (B) on its sauce tree over the reference
# line, `--cookbook-page` opens the page behind the FIRST UNLOCKED node
# the tree draws, and `--cookbook-basics` opens the FIRST BASICS page the
# guide ships. Which sauce and which topic those are is deliberately not
# named here: the whole feature is that the screen comes off
# `cookbook_json`, and a flag that named either would be one more place to
# edit when the content files change. All three imply `--populate`, so the
# factory behind the plate is running.
#
# `--cookbook-page ID` opens that sauce's page instead. The bare form stays
# the default and stays wire-driven, for the reason above; the argument
# exists because "the first unlocked node" can only ever photograph the
# FIRST family, and from the third sauce on there are pages that flag can
# never reach. A reviewer asking for a specific page has already named it in
# the request, so naming it on the command line adds no place to edit — and
# an id the book does not hold is refused in words that list the ones it
# does, rather than photographing the wrong page.
#
# `--cookbook-basics ID` opens that topic's page instead, for the same reason
# and with the same refusal: "the first BASICS page" can only ever be the
# first topic in `data/guide.ron`, so a change to any page below it — the
# contracts page, the growing page — has no frame at all otherwise. An id the
# guide does not ship is refused with the list it does.
#
# `--full-dock` builds the FULL-DOCK LINE (test_layouts.gd's
# `build_full_dock_line`: the reference line on auto-fill, run until the
# dock has every one of its 32 places taken) and freezes the frame
# on a lit half of the dock lamps' pulse, so the clipboard's `32/32` /
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
# `--hover` aims the cursor over the first placed machine before
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
# `--after-machine-hover` puts the cursor over the first placed machine
# first, lets main.gd lay that tooltip out for a few frames, and only then
# aims the --hover* flag. So the frame shows a hover taken straight after a
# taller, wider one — the case issue #173 was about, which a hover from a
# fresh state cannot show. Isometric only, and it needs a --hover* flag and a
# machine in the world; without them it refuses.
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
# THE CURSOR IS THIS RUN'S OWN, NOT THE MACHINE'S (issue #4). It is
# `main.gd`'s `_pointer_override`, a screen point the scene reads in place of
# the mouse. It used to be `Input.warp_mouse`, which moves the machine's ONE
# pointer: it yanked the pointer out from under the person at the desk, and it
# leaked between runs. A run asking for no hover at all photographed the tile
# the LAST run had parked on (`--fixture bottleneck --payout` came back with
# a dock tooltip from the `--hover-dock` run before it), and a parallel
# worktree's run could move the pointer between the warp and the shutter. The
# override cannot leak in either direction. The procedure is kept anyway,
# because it is cheap and it also catches the scene moving under a fixed aim.
# Every run PARKS the cursor on empty ground first (`_park_cursor`). Any
# --hover* flag then aims it. It is RE-ASSERTED a few frames before the
# shutter (`_settle_cursor`), and the frame is only written if the cursor
# really is over the tile it was aimed at (`_check_cursor`). A mismatch
# REFUSES, since a tooltip belonging to the wrong thing is exactly the frame
# this script exists not to hand back.
#
# IN FIRST PERSON THE CURSOR IS NOT WHAT AIMS THE TOOLTIP — THE CROSSHAIR
# IS. `main.gd`'s `_update_hover_tooltip` asks the SCREEN CENTRE what it is
# over when `_view_mode` is FIRST_PERSON and `_pointer_pos` otherwise, so
# every one of the three steps above asks the same question the frame does
# (`_hover_query_pos`), and a --hover* flag TURNS THE CAMERA rather than
# moving the cursor (`_aim_at_tile`). This is not a detail: checking the
# pointer in a view the pointer does not drive made `--first-person
# --hover-dock` fail at the shutter and blame a parallel worktree for it,
# which is the same class of confidently-wrong report as issues #4 and #70.
#
# `--shop` opens the build menu (docs/UI_UX.md §5) before rendering.
# `--shop-search TEXT` opens it too, and types TEXT into its search box, so the
# frame shows the filter: matching slots only, empty category sections hidden.
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
# `--settings` opens the SETTINGS page of whichever menu this run is
# rendering: the pause menu's by itself, or the main menu's with `--menu`.
# One screen hosted twice (`settings_panel.gd`), so it is worth both frames.
#
# `--debug` opens the DEBUG MENU (F3) over a running factory — the panel
# that gives money and changes time speed. It is a development tool and it
# says so on its own heading, which is part of what the frame is reviewing.
#
# **`Settings` is reset to its shipped defaults before the scene is built.**
# Unlike every headless test, this run has a real display, so the autoload
# has already read whoever's `user://settings.cfg` is on this box — and a
# review frame rendered at someone's personal field of view, movement speed
# and hall size is a frame nobody else can reproduce. Nothing is written
# back: `persist_enabled` goes off first, so the developer's own file is
# untouched by the reset.
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

## The frame rate this harness insists on being run at, matching the
## `--fixed-fps` in `scripts/screenshot-scene.sh` (issue #82; the two are
## checked against each other by `scripts/check-invariants.sh`).
##
## WHY A SHOT NEEDS A PINNED CLOCK. The warm-up above is counted in FRAMES,
## and `main.gd` advances the simulation on the frame's DELTA — so with a
## free-running clock the number of ticks between "fixture built" and
## "shutter" is however much wall time those 45 frames happened to take,
## which is a property of the host's load and not of the command. Two runs
## of `--bottleneck --closeup 4,0,10 --no-hud` landed one tick apart and
## produced different pictures; nothing in the output said so.
const FIXED_FPS := 60.0

## How far a frame's delta may sit from `1.0 / FIXED_FPS` and still count as
## pinned. The engine hands back exactly that quotient under `--fixed-fps`,
## so this is a float-comparison tolerance rather than a real allowance — a
## free-running frame misses by tens of milliseconds, not by a microsecond.
const FIXED_DELTA_EPSILON := 1e-6

## The frame `_settle_cursor` re-asserts the cursor on — three before the
## shutter, so main.gd has frames left to lay the tooltip out with it.
const CURSOR_SETTLE_FRAME := WARMUP_FRAMES - 3

## The frame `--after-machine-hover` moves the cursor from the machine to the
## --hover* target on: enough frames over the machine for main.gd's layout
## pass to have grown the tooltip to the machine's size.
const AFTER_MACHINE_HOVER_FRAME := 12

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
	"stalled_run", "understaffed", "shortfall", "payday_warning", "full_dock",
	"unclerked_dock", "warehouse", "split_docks", "mustard",
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
const HEALTHY_FIXTURES: PackedStringArray = ["bbq", "bottleneck", "full_dock", "mustard"]

var _frames := 0
var _on_screen := false
var _frames_drawn_seen := -1
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
## `--after-machine-hover`: see the header.
var _after_machine_hover := false
var _shop := false
var _shop_search := ""
## `--config TX,TY` clicks the building standing on that tile once
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
## `data/unlocks.ron` — the shipped first one takes a hundred boxes of ketchup
## sold — so the market, the larder and the board photograph with the BBQ tier
## released. Combine it with `--market` or `--board` to shoot the "after" half
## of a before/after pair; without it those panels show the "before" half,
## which is what a new game actually looks like.
##
## It sets the boxes-sold ledger through `test_layouts.gd`'s `sell`, i.e. the
## save file, rather than running a chain for the fifty thousand ticks it
## would take to sell them: this is a camera, and
## `sim_core/tests/unlocks.rs` is what earns the gate honestly.
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
## `--cookbook` opens THE COOKBOOK (B) on its sauce tree; `--cookbook-page`
## opens the page of the first unlocked sauce the tree drew. The panel is the
## same in any world — it is the content files rendered — so the fixture is
## chosen for what is BEHIND it, which is the line the game ships.
## `--cookbook-basics` opens the first prose page in `data/guide.ron`, which
## is the half of the book that has numbers substituted into sentences — the
## shot exists to prove no `{placeholder}` reaches the screen.
var _cookbook := false
var _cookbook_page := false
## `--cookbook-page ID`'s argument, or "" for the flag's wire-driven default
## (the first unlocked node the tree draws). See this file's header for why
## the argument exists and why the default is still the default.
var _cookbook_page_id := ""
var _cookbook_basics := false
## `--cookbook-basics ID`'s argument, or "" for the flag's wire-driven default
## (the first topic `data/guide.ron` lists). It exists for `--cookbook-page`'s
## reason: "the first page" can only ever be the first topic in the file, and a
## change to any page below it has no frame otherwise.
var _cookbook_basics_id := ""
var _understaffed := false
## `--staff-shortfall` opens the staff panel over a line funded to less than
## its next payday's bill (the quality pass, ticket 4's BEFORE warning) —
## `--staff`'s own dispatch, aimed at `build_underfunded_line` instead of the
## fed reference line, so the footer's shortfall sentence is what the shot
## reviews rather than the meters `--staff` already covers.
var _staff_shortfall := false
## `--payday-warning` (2026-09-26) opens the build menu over a line funded to
## one unit less than its bill plus a belt, then BUYS a belt through the real
## funnel (`_buy_slot`), so the frame is the confirmation panel as a click
## raises it: its heading, the figures the sim answered with, the consequence
## sentence and both buttons, over the menu it interrupted. Driving the click
## rather than showing the panel is the point — a widget made visible would
## prove nothing about the path a player takes to it.
var _payday_warning := false
## `--short-staffed` builds the same understaffed line as `--understaffed`
## but opens NOTHING and hovers nothing: the shot exists to prove the
## always-visible SHORT-STAFFED strip on the clipboard is unmissable with no
## panel up, which is the one thing a `--staff` or `--understaffed` shot
## cannot show (both put a panel or a tooltip over the screen).
var _short_staffed := false
var _hotbar_key := ""
var _stock := false
## `--at-capacity` fills every raw to its warehouse ceiling, so the market's
## crates and the STOCK plate photograph the full state rather than the
## roomy one.
var _at_capacity := false
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
## of the one dock's stock) reads as one docket satisfied and the rest part-
## filled: the checkbox mechanic and the competition for stock in one frame.
## A MINIMUM dock count rather than an exact one, because the contracts check
## every 1000/1500/2500 ticks and drain the dock about as fast as the line can
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
## `--full-dock` photographs a dock with every place taken: the clipboard's
## ON THE DOCK line reading `32/32` in warning red under `⚠ FULL — belt
## backing up`, the boxes standing still on the belt the dock has stopped
## taking from, and the dock's deck lamps PULSING rather than steady. Both
## states arrived with the dock-capacity change and neither could be
## photographed on demand: `--bottleneck` leaves the dock part-filled.
##
## `Layouts.build_full_dock_line` does the waiting (some 25 000 ticks against
## the signed corner-shop order's appetite); what is left here is the same freeze
## `--payout` and `--crushing` need, plus which HALF of the blink to stop on.
## A pulse is a rhythm rather than a state, so a single frame can land on the
## dark half and photograph a dock that looks like it holds nothing — this
## picks the half deliberately instead, and two runs a `--dock-lamp` apart
## are how a reviewer sees the blink itself.
var _full_dock := false
var _split_docks := false
## `--dock-lamp dark` for the other half of the pulse; lit by default, since
## a lit lamp over a full dock is the state worth reviewing and a dark one is
## indistinguishable from an empty dock's.
var _dock_lamp_dark := false
## How long to hunt for a tick that is both full AND on the wanted half of
## the blink. A whole pulse is DOCK_FULL_PULSE_TICKS (12), so this is only
## generous because a contract may ship mid-hunt and take the dock off its
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
## `--settings` opens the settings page of whichever menu is being rendered;
## `--debug` opens the F3 development panel over the game. `--set key=value`
## moves one knob off its default for the render (`_reset_settings`).
var _settings := false
var _debug := false
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
	_on_screen = args.has("--on-screen")
	if not _on_screen:
		_hide_window()
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
	if args.has("--warehouse"):
		_pick_fixture("warehouse", "--warehouse")
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
	_cookbook_page = args.has("--cookbook-page") or _string_arg(args, "--cookbook-page") != ""
	# `_string_arg` hands back whatever token follows, flag or not, so a bare
	# `--cookbook-page --populate` would ask the book for a page called
	# "--populate". Anything starting with `--` is the next flag, not an id.
	_cookbook_page_id = _string_arg(args, "--cookbook-page")
	if _cookbook_page_id.begins_with("--"):
		_cookbook_page_id = ""
	_cookbook_basics = (
		args.has("--cookbook-basics") or _string_arg(args, "--cookbook-basics") != ""
	)
	# Same trap as `--cookbook-page` above: a bare `--cookbook-basics --populate`
	# would otherwise ask the guide for a topic called "--populate".
	_cookbook_basics_id = _string_arg(args, "--cookbook-basics")
	if _cookbook_basics_id.begins_with("--"):
		_cookbook_basics_id = ""
	_cookbook = _cookbook_page or _cookbook_basics or args.has("--cookbook")
	if _cookbook:
		var flag := "--cookbook"
		if _cookbook_page:
			flag = "--cookbook-page"
		elif _cookbook_basics:
			flag = "--cookbook-basics"
		_pick_fixture("chain", flag)
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
	_after_machine_hover = args.has("--after-machine-hover")
	_shop_search = _string_arg(args, "--shop-search")
	_shop = args.has("--shop") or _shop_search != ""
	_payday_warning = args.has("--payday-warning")
	if _payday_warning:
		_pick_fixture("payday_warning", "--payday-warning")
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
		# only be more ticks against an already-full dock.
		_pick_fixture("full_dock", "--full-dock")
	# Last of the fixture flags on purpose: --contracts, --payout and
	# --crushing each force the plain shipped line, and --bbq is meant to be
	# combined with the first of those (the mixed orders are the half of this
	# epic the clipboard shows), so it has the final say.
	if args.has("--bbq"):
		_pick_fixture("bbq", "--bbq")
		_populate = true
	# After --bbq for the same reason --bbq is after --contracts: the third
	# sauce is the one a combined flag is most likely to be aimed at, so
	# `--cookbook-page boxed_mustard --mustard` opens the new page over the
	# line that made it rather than over the shipped ketchup line.
	if args.has("--mustard"):
		_pick_fixture("mustard", "--mustard")
		_populate = true
	_split_docks = args.has("--split-docks")
	if _split_docks:
		_pick_fixture("split_docks", "--split-docks")
	_stalled_run = args.has("--stalled-run")
	if _stalled_run:
		# NOT --populate: the shot compares a DEAD run with a merely EMPTY
		# one, and a factory run forward has no empty runs left in it (the
		# fixture's own doc says why).
		_pick_fixture("stalled_run", "--stalled-run")
	_dock_lamp_dark = _string_arg(args, "--dock-lamp") == "dark"
	_hotbar_key = _string_arg(args, "--hotbar")
	_stock = args.has("--stock")
	_at_capacity = args.has("--at-capacity")
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
	_settings = args.has("--settings")
	_debug = args.has("--debug")
	if not _reset_settings(args):
		return
	if not _parse_closeup(args):
		return
	_resolve_fixture()
	_apply_fluid_gallery_defaults(args)
	_apply_warehouse_defaults(args)
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


## Whether this frame arrived on the fixed clock the shot is a function of,
## and a refusal naming the missing flag if it did not.
##
## Checked on every frame rather than once, because "pinned" is a claim about
## all of them: `--fixed-fps` is what stops the engine synchronising to real
## time, and a single frame that drifts is a tick this run may or may not
## have spent. There is nothing to measure against but the delta itself —
## the engine does not report the flag back — and that is the quantity that
## matters anyway.
##
## It REFUSES rather than warning. A frame taken on a free clock is exactly
## as plausible as a good one and is the thing issue #82 was filed about: a
## whole glow sweep was judged against two renders of one baseline that
## disagreed with each other, at 98x and 8x selectivity, with nothing in
## either output saying which was which.
func _clock_is_pinned(delta: float) -> bool:
	var wanted := 1.0 / FIXED_FPS
	if absf(delta - wanted) <= FIXED_DELTA_EPSILON:
		return true
	printerr(
		"SCREENSHOT_FAIL: frame %d arrived %.1f ms after the last one, not the "
		% [_frames, delta * 1000.0]
		+ "%.1f ms this harness is pinned to. " % (wanted * 1000.0)
		+ "The engine is synchronising to real time, so how far the simulation "
		+ "runs before the shutter depends on what else this machine is doing "
		+ "and two runs of this command would photograph two different "
		+ "factories (issue #82). Run it through scripts/screenshot-scene.sh, "
		+ "or pass --fixed-fps %d to godot yourself." % int(FIXED_FPS)
	)
	return false


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


## Sells enough boxes to cross every gate in `data/unlocks.ron`, leaving the
## last one `offset` boxes away from its threshold — 0 to sit exactly on it,
## -1 to sit one box short. What each gate counts is read off the wire, never
## written out here, so a balance pass on that file cannot leave these shots
## photographing the wrong side of the gate.
func _cross_gates(offset: int) -> void:
	Layouts.cross_every_gate(_main.sim, offset, "SCREENSHOT_FAIL")
	# `cross_every_gate` goes through save/load, so the rendered world is
	# rebuilt from the reloaded one — identical by construction, but the scene
	# has to be told, exactly as `_build_fixture` tells it.
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
		"warehouse":
			# Refused rather than photographed when it comes up short, for
			# the same reason the fluid gallery is: a rack holding less than
			# the caption says looks exactly like a rack holding what it
			# says, and the shot is OF the contents.
			var short: String = Layouts.build_warehouse_row(_main.sim)
			if short != "":
				printerr("SCREENSHOT_FAIL: %s" % short)
				_refuse()
				return
		"items":
			Layouts.build_item_gallery(_main.sim)
		"bottleneck":
			Layouts.build_bottleneck_line(_main.sim)
		"bbq":
			Layouts.build_bbq_running_line(_main.sim)
		"mustard":
			Layouts.build_mustard_line(_main.sim)
		"stalled_run":
			Layouts.build_stalled_pipe_run(_main.sim)
		"understaffed":
			Layouts.build_understaffed_line(_main.sim)
		"shortfall":
			Layouts.build_underfunded_line(_main.sim)
		"payday_warning":
			Layouts.build_payday_warning_line(_main.sim)
		"full_dock":
			Layouts.build_full_dock_line(_main.sim)
		"split_docks":
			# Like `unclerked_dock`, this one can be built and still not BE
			# itself: the subject is two docks DISAGREEING, and a frame where
			# both agree shows nothing. Refuse rather than photograph it.
			Layouts.build_split_dock_line(_main.sim)
			if not Layouts.split_dock_lamps_found(_main.sim):
				printerr("SCREENSHOT_NOTE: --split-docks never found a tick with one dock backing up and one clear — nothing to photograph")
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
	#
	# Except the warehouse row, whose subject IS the larder. Stocking it
	# again buys 64 more of every raw and — since W1 — puts up whatever racks
	# that needs, so the three racks in frame would hold amounts the caption
	# does not claim while more of them stood off camera. A fixture that is
	# about the stock does its own.
	if _fixture != "warehouse":
		Layouts.stock_larder(_main.sim)
	# `--at-capacity` buys every unlocked raw up to the ceiling the warehouse
	# gives it. It is the one state the W4 words exist for and the one a
	# fixture cannot drift into on its own: with room to spare every crate
	# reads the same as it always did. The balance is deliberately left FAT
	# afterwards — the whole point of the frame is a purchase money cannot fix.
	if _at_capacity:
		Layouts.fund(_main.sim, 32767)
		for item in Layouts.stockable(_main.sim):
			var room: int = Layouts.stock_room_for(_main.sim, item)
			if room <= 0:
				continue
			_main.sim.issue_command_json(JSON.stringify({
				"type": "buy_ingredient", "item": item, "quantity": room,
			}))
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


## Where `--warehouse` puts the camera when nothing else says: the middle of
## `build_warehouse_row`'s three racks, close enough in that a single BAY is
## a readable part of the frame. The racks are the smallest subject any shot
## in this file has — six shelves inside one 1x2 tile — and the auto-framing
## that suits a factory renders a loaded pallet about four pixels tall.
const WAREHOUSE_FOCUS := Vector2(2.5, 0.5)
const WAREHOUSE_SIZE := 26.0


## `--warehouse`'s framing, on the same terms as the fluid gallery's above:
## a default that an explicit `--closeup` overrides. The HUD STAYS — unlike
## the gallery, this fixture's subject sits in the middle of an otherwise
## empty floor with nothing in the corners, and the hotbar and legend are
## worth having in frame as the scale the racks are read against.
func _apply_warehouse_defaults(_args: Array) -> void:
	if _fixture != "warehouse":
		return
	if not _closeup:
		_closeup = true
		_closeup_tile = WAREHOUSE_FOCUS
		_closeup_size = WAREHOUSE_SIZE


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


## Where the window goes: far enough left of any desktop that it can never be
## on a screen, whatever the display arrangement.
const OFF_SCREEN_POSITION := Vector2i(-20000, 0)


## Takes the window off the desktop before anything is drawn in it. See the
## window note in this file's header. Borderless first, because macOS clamps a
## window that has a title bar back onto a screen.
func _hide_window() -> void:
	DisplayServer.window_set_flag(DisplayServer.WINDOW_FLAG_NO_FOCUS, true)
	DisplayServer.window_set_flag(DisplayServer.WINDOW_FLAG_BORDERLESS, true)
	DisplayServer.window_set_position(OFF_SCREEN_POSITION)


## Draws this frame by hand if the engine skipped the last one. Godot does
## not draw a window macOS reports as occluded, and an off-screen window is
## occluded, so without this the viewport would hold whatever was drawn before
## the move. `Engine.get_frames_drawn` counts only the loop's own draws, so a
## count that has stopped moving means the loop has stopped drawing.
func _keep_drawing(delta: float) -> void:
	if _on_screen:
		return
	var drawn := Engine.get_frames_drawn()
	if drawn == _frames_drawn_seen:
		RenderingServer.force_draw(true, delta)
	_frames_drawn_seen = drawn


## The view this run photographs, which is NOT `main.gd`'s
## `DEFAULT_VIEW_MODE` — a review shot is isometric unless `--first-person`
## asked otherwise. Every other framing flag this tool has (`--closeup`,
## `--zoom`, `--pan`, and every `--hover*`, which aims the cursor at
## where a tile projects) is an isometric idea, so following the game's
## opening view would have silently re-aimed every command line in
## `docs/`, `scripts/` and CI at a first-person camera those flags cannot
## steer. The game's default is exercised by `test_camera_nav.gd` instead,
## and photographed by passing `--first-person` here.
func _enter_view_mode() -> void:
	if not _first_person:
		_main._set_view_mode(_main.ViewMode.ISOMETRIC)
		return
	_main._set_view_mode(_main.ViewMode.FIRST_PERSON)
	# Higher up and looking a bit further up than the default entry pose, so
	# the sky/horizon/ground extent — the thing being reviewed — is actually
	# in frame, not just a close-up of one machine.
	_main._fp_position += Vector3(0.0, 4.0, 0.0)
	_main._fp_pitch = deg_to_rad(5.0)
	_main._update_fp_camera_transform()


## Aims the cursor (`main.gd`'s `_pointer_override`) over one world tile,
## using the same tile ->
## world -> screen math `_tile_under_screen_pos` inverts (via the camera's own
## `unproject_position`, which needs the camera actually inside the tree — see
## the `_ready()` timing note in `_initialize` above, which is exactly why
## this can't run any earlier than `_process`). Every --hover* flag is this
## one function plus a different way of picking the tile.
##
## The tile is REMEMBERED (`_cursor_tile`), not the screen point, because the
## camera can still move after the aim: `_settle_cursor` aims again just
## before the shutter and `_check_cursor` proves it stuck. See the cursor note
## in this file's header.
##
## Returns false having REFUSED if the tile is not in frame. A tooltip for a
## point outside the window has nowhere to be drawn, and when this aimed the
## real pointer the window server clamped it to the edge, so the shutter check
## fired a few frames later and blamed a hand on the mouse for a tile the
## camera simply was not showing
## (`--fixture chain --closeup 26,0,8 --hover-hopper` did exactly that). A
## cause this run CAN see, it names.
## `--cookbook-page`: the page behind the first node the TREE draws that is
## not a silhouette. Read off `cookbook_json` rather than written down, for
## the reason the whole feature exists — a flag naming a sauce would be a
## place to edit every time the content files change. It refuses in words if
## every family is still locked, rather than photographing the tree twice.
##
## `--cookbook-page ID` asks for that page by name instead. "The first
## unlocked node" is a fine default and a dead end as the only one: it can
## only ever open the FIRST family, so from the third sauce on there are
## pages no flag can reach, and a reviewer asking for one of those has
## already named it. An id the book does not hold is refused with the list of
## ids it does — the alternative is a frame of the wrong sauce that looks
## exactly like a frame of the right one, which is the trap `--fixture`'s own
## no-fallback `match` was built to close (issue #70).
func _open_first_unlocked_sauce() -> bool:
	var book = JSON.parse_string(_main.sim.cookbook_json())
	if not (book is Dictionary):
		printerr("SCREENSHOT_FAIL: --cookbook-page could not read cookbook_json")
		_refuse()
		return false
	var sauces: Array = (book as Dictionary).get("sauces", [])
	if _cookbook_page_id != "":
		var known := PackedStringArray()
		for entry in sauces:
			var id := String(entry.get("id", ""))
			known.append(id)
			if id != _cookbook_page_id:
				continue
			# A LOCKED page is photographed on request without complaint: the
			# silhouette is a real screen, and reviewing a new sauce before
			# its gate opens is exactly what it is for.
			_main._cookbook.open("sauce", id)
			_main._position_cookbook_panel()
			return true
		printerr(
			"SCREENSHOT_FAIL: --cookbook-page %s — the book has no page with that id. It holds: %s"
			% [_cookbook_page_id, ", ".join(known)]
		)
		_refuse()
		return false
	for entry in sauces:
		if bool(entry.get("locked", false)):
			continue
		_main._cookbook.open("sauce", String(entry.get("id", "")))
		_main._position_cookbook_panel()
		return true
	printerr(
		"SCREENSHOT_FAIL: --cookbook-page found no unlocked sauce in this world, so \
there is no page to open. Every family is still behind its gate — ship enough to \
release one, or use --cookbook for the tree."
	)
	_refuse()
	return false


## `--cookbook-basics`: the first page `data/guide.ron` lists, opened off the
## wire for the same reason as `--cookbook-page` — the flag must not know the
## topic's name. It refuses in words if the guide ships nothing, because a
## shot of the tree under a basics flag reviews the wrong screen.
func _open_first_basics_topic() -> bool:
	var book = JSON.parse_string(_main.sim.cookbook_json())
	if not (book is Dictionary):
		printerr("SCREENSHOT_FAIL: --cookbook-basics could not read cookbook_json")
		_refuse()
		return false
	var known: Array[String] = []
	for topic in (book as Dictionary).get("guide", []):
		var id := String(topic.get("id", ""))
		if id == "":
			continue
		known.append(id)
		if _cookbook_basics_id != "" and id != _cookbook_basics_id:
			continue
		_main._cookbook.open("topic", id)
		_main._position_cookbook_panel()
		return true
	if _cookbook_basics_id != "":
		printerr(
			"SCREENSHOT_FAIL: --cookbook-basics %s — data/guide.ron has no topic with \
that id. It ships: %s"
			% [_cookbook_basics_id, ", ".join(known)]
		)
		_refuse()
		return false
	printerr(
		"SCREENSHOT_FAIL: --cookbook-basics found no topics in cookbook_json, so there \
is no page to open. data/guide.ron ships the BASICS pages — check it loaded."
	)
	_refuse()
	return false


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
%dx%d window, so its tooltip could not be in the frame. Widen the shot (a bigger \
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
		_point_at_tile(tile)


func _point_at_tile(tile: Vector2i) -> void:
	var world_point: Vector3 = _main._tile_origin(tile.x, tile.y, _main.BELT_SURFACE_Y)
	_main._pointer_override = _main._camera.unproject_position(world_point)


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
	return _main._pointer_pos(root)


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


## `--after-machine-hover`: the cursor over the first machine until
## `AFTER_MACHINE_HOVER_FRAME`, when `_apply_hover_flags` moves it on. Refuses
## rather than photographing a fresh hover under a flag that promised a
## second one.
func _hover_machine_first() -> void:
	if _hover_flags.is_empty() or _first_person:
		printerr("SCREENSHOT_FAIL: --after-machine-hover needs a --hover* flag to move on to, in the isometric view")
		_refuse()
		return
	var tile := _first_machine_tile()
	if tile == HOVER_NONE:
		printerr("SCREENSHOT_FAIL: --after-machine-hover found no machine in this world to hover first")
		_refuse()
		return
	_hover_over_tile(tile)


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
## A point chosen right on a tile boundary is one rounding error from the
## machine next door; this was measured when the park was a warp of the real
## pointer, which lands on whole pixels, and is kept as a margin.
const PARK_CLEARANCE_PX := 4.0


## Puts the cursor somewhere that says nothing, and records where.
##
## Called on every run before anything opens (and again from
## `_settle_cursor`), because a scene that has drawn a tooltip before the
## park keeps it until something asks again. When the cursor was the real
## pointer, `--payout` photographed the dock tooltip of the `--hover-dock`
## run before it. See the cursor note in this file's header.
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
		_main._pointer_override = point
		_cursor_tile = _main._tile_under_screen_pos(point)
		_cursor_park_noisy = false
		return
	var fallback := Vector2(PARK_POINTS[0].x * size.x, PARK_POINTS[0].y * size.y)
	_main._pointer_override = fallback
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
## The aim happens on frame 2 and the frame is written on WARMUP_FRAMES. The
## cursor is this run's own now, so nothing outside can move it in between,
## but the camera can, and re-asserting costs one projection.
##
## Re-PARKING (rather than re-aiming at the remembered tile) is what makes
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
## immune to rounding at a tile edge, where an exact tile
## comparison on a tile boundary would fail for no reason. A park that had to
## settle for a tile that talks claims nothing beyond the NOTE it already
## printed, and is allowed through on that tile alone.
##
## What gets asked is `_hover_query_pos`, not the pointer: in first person
## main.gd queries the screen centre, and a check on any other quantity is a
## check on something that is not in the frame.
##
## When it refuses it reports WHAT it saw and stops there. The causes it used
## to name (a parallel worktree, a hand on the mouse) were guesses this
## process could not distinguish, and a harness that states a guess as the cause
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
## does the aiming in this view: in first person it is the camera, and
## naming anything else is how this harness came to blame a parallel
## worktree for a camera it had aimed itself.
func _drift_candidates() -> String:
	if _in_first_person():
		return (
			"the pitch needed to centre that tile is steeper than mouse-look's own "
			+ "%d-degree clamp, " % int(round(rad_to_deg(_main.FP_PITCH_LIMIT)))
			+ "or something moved the first-person camera after the aim"
		)
	return "a camera move after the aim, or a fixture that rebuilt the world under it"


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
	if _shop_search != "":
		# What a keystroke does: the box's text changes and it says so.
		_main._shop_search.text = _shop_search
		_main._shop_search.text_changed.emit(_shop_search)


## `--payday-warning`: the build menu, then a real belt purchase through
## `_buy_slot` — the same call the slot's button makes — so the confirmation in
## the frame is one a click produced. It refuses rather than shooting a frame
## with no panel in it: a fixture funded wrong, or a rule that stopped warning,
## would otherwise ship a green screenshot of the build menu and nothing else
## (issue #70's lesson, and `--config`'s guard above).
func _raise_payday_warning() -> void:
	_main._set_shop_menu_open(true)
	var index := -1
	for i in range(_main._shop_slots.size()):
		if String(_main._shop_slots[i]["kind"]) == "belt":
			index = i
	if index < 0:
		printerr("SCREENSHOT_FAIL: --payday-warning found no belt slot in the build menu to buy from.")
		_refuse()
		return
	_main._buy_slot(index)
	if not _main._payday_confirm_open:
		printerr(
			"SCREENSHOT_FAIL: --payday-warning bought a belt and no confirmation "
			+ "opened — the balance covers the next payday, or the warning is gone."
		)
		_refuse()


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
			"SCREENSHOT_FAIL: --config %s has nothing configurable on it to "
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
## it exactly the way a player does. What puts product on the dock the
## contracts draw from is the belt at the end of the line, not a command
## issued from here.
func _run_contract_cycles() -> void:
	# A new game runs no contract (since 2026-09-26); the review frame
	# wants the whole board competing, so sign every offer first — the
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
		# No belt-surface call here: `main.gd::_update_belt_frame` was deleted
		# by issue #79 and `_apply_dirty_chunks` above drives the rubber now.
		# Calling it raised a runtime error that silently unwound the REST of
		# this loop body — see `_run_chain_forward`, fixed 2026-09-21.
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
		# No `_update_belt_frame` — deleted by issue #79; see `_run_chain_forward`.
		_main._update_machine_motion()
		_main._update_payout_feedback()
	_main._refresh_buffers()
	_main._update_fluid_network()
	# Frozen for `--payout`'s reason: a fleck's whole flight is
	# CRUSH_FLIGHT_TICKS (9) and the warm-up frames that follow are longer.
	_main.set_process(false)


## Stops the clock on a tick where a dock is at its ceiling AND
## the dock's lamps are on the wanted half of their pulse, then freezes.
##
## `Layouts.build_full_dock_line` has already run the line to full by the
## time this is called; the loop here is the alignment, and it re-checks
## fullness on every step rather than assuming it, because a contract that
## ships mid-hunt takes the dock off its ceiling for a cadence and would
## otherwise be photographed as a dock that merely holds some stock.
func _run_to_full_dock() -> void:
	var held := false
	for i in range(FULL_DOCK_HOLD_TICK_LIMIT):
		if _a_dock_reports_full() and _dock_lamp_on_wanted_half():
			held = true
			break
		_main.sim.tick()
		_main._apply_dirty_chunks()
		# No `_update_belt_frame` — deleted by issue #79; see `_run_chain_forward`.
		_main._update_machine_motion()
	if not held:
		printerr("SCREENSHOT_NOTE: --full-dock never found a %s tick with a dock backing up — the shot is of whatever the line reached"
			% ("dark" if _dock_lamp_dark else "lit"))
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()
	# Frozen for `--payout`'s reason, and one step more so: the lamps read
	# `sim.current_tick()`, so eight more ticks a frame across the warm-up
	# would hand back a coin-flip on which half of the pulse is in frame —
	# the exact thing this flag exists to make deterministic.
	_main.set_process(false)


## Lands the shot on a tick where the two docks still disagree AND the pulse
## is on the half `--dock-lamp` asked for.
##
## That second condition is the whole shot. A dock merely HOLDING stock is
## lit steady, and a dock backing up PULSES, so on the lit half of the pulse
## both lamps are lit and the frame shows nothing: it is the DARK half that
## separates them, with the jammed dock's lamp out and the clear dock's
## still burning. `--dock-lamp dark` is therefore the interesting frame here,
## which is the opposite way round from `--full-dock`.
func _run_to_split_docks() -> void:
	var held := false
	for i in range(FULL_DOCK_HOLD_TICK_LIMIT):
		if Layouts.split_dock_lamps_found(_main.sim) and _dock_lamp_on_wanted_half():
			held = true
			break
		_main.sim.tick()
		_main._apply_dirty_chunks()
		# No `_update_belt_frame` — deleted by issue #79; see `_run_chain_forward`.
		_main._update_machine_motion()
	if not held:
		printerr("SCREENSHOT_NOTE: --split-docks never found a %s tick with the docks disagreeing — the shot is of whatever the lines reached"
			% ("dark" if _dock_lamp_dark else "lit"))
	_main._refresh_buffers()
	_main._update_fluid_network()
	_main._update_contract_panel()
	# Frozen for `_run_to_full_dock`'s reason: the lamps read
	# `sim.current_tick()`, so letting the scene run on would hand back a
	# coin-flip on which half of the pulse is in frame.
	_main.set_process(false)


## Whether any dock says IT is backing up this tick.
##
## Read off `dock_snapshots_json` rather than `Layouts.delivery_full`: that
## is a summary over every dock, and it is one dock's own verdict this shot
## is of.
## Read from the sim directly because the hunting loop below does not call
## `_refresh_buffers`, so `_main._dock_states` is a frame behind here.
func _a_dock_reports_full() -> bool:
	var docks = JSON.parse_string(_main.sim.dock_snapshots_json())
	if not (docks is Array):
		return false
	for dock in docks:
		if dock is Dictionary and bool(dock.get("full", false)):
			return true
	return false


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


## Puts `Settings` back to its shipped defaults for the duration of this run,
## then applies any `--set key=value` this run asked for, without writing
## anything. See the header: this is the one script in the
## suite that renders with a real display, so it is the one that would
## otherwise photograph the developer's own preferences.
func _reset_settings(args: Array) -> bool:
	var loop := Engine.get_main_loop()
	if not (loop is SceneTree):
		return true
	var settings: Node = (loop as SceneTree).root.get_node_or_null("Settings")
	if settings == null:
		return true
	# Off FIRST. Every press on the panel this run is about to photograph
	# persists, and a review render must not be able to change the machine
	# it ran on.
	settings.persist_enabled = false
	settings.reset()
	# `--set key=value`, after the reset, so the frame is the defaults plus
	# exactly what was asked for. Refused rather than ignored on an unknown
	# key, for `--fixture`'s reason: a typo that comes back as a plausible
	# frame of the wrong thing is worse than no frame at all.
	for i in range(args.size()):
		var arg := String(args[i])
		# `--set key=value` and `--set=key=value` both, like every other flag
		# here that takes one (`_string_arg`) — repeatable, so this is a scan
		# of the whole list rather than one lookup.
		var assignment := ""
		if arg == "--set" and i + 1 < args.size():
			assignment = String(args[i + 1])
		elif arg.begins_with("--set="):
			assignment = arg.substr(6)
		else:
			continue
		var pair := assignment.split("=", true, 1)
		if pair.size() != 2 or settings.knob(pair[0]) == null:
			printerr(
				"SCREENSHOT_FAIL: --set wants key=value and a key Settings knows: '%s'"
				% assignment
			)
			_refuse()
			return false
		settings.set_value(String(pair[0]), float(pair[1]))
	return true


func _process(delta: float) -> bool:
	if _refused:
		return true
	_keep_drawing(delta)
	_frames += 1
	# Before the fixture, before the flags, and on EVERY frame: a run whose
	# clock is not pinned cannot hand back the picture its arguments name,
	# and that is worth more as a refusal than as a caveat (issue #82).
	if not _clock_is_pinned(delta):
		_refuse()
		return true
	# Build first: every flag below is about looking at something, and since
	# the "Real Placement" epic there is nothing to look at until one of the
	# fixtures has been placed.
	if _menu:
		if _frames == 1 and _settings:
			_main._on_settings_pressed()
		elif _frames == 1 and not _title:
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
	# Before the view mode is chosen: from here to the shutter the camera is
	# only ever what this script aims it at. The hover flags stopped warping
	# the OS pointer when the window went off-screen, but the scene still
	# polls live input — a key held in another window pans, a stray wheel
	# notch zooms, a drag looks. None of that is in the command, so none of
	# it may be in the picture (issue #82).
	if _frames == 1 and _main != null:
		_main._camera_input_sealed = true
	if _frames == 1:
		_enter_view_mode()
	# Before main.gd has drawn a single tooltip, and before any --hover* flag
	# on frame 2 aims it somewhere on purpose, so no tooltip is drawn from
	# wherever the real mouse happens to be (issue #4).
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
	# The closeup is applied BEFORE any hover flag: a hover aims the cursor
	# at where a tile currently projects on screen, so re-aiming the camera
	# afterwards would leave the cursor pointing at whatever moved under it.
	if _frames == 2 and _closeup:
		_enter_closeup()
	if _frames == 2 and _item_gallery:
		_stand_item_gallery()
	if _frames == 2 and _after_machine_hover:
		_hover_machine_first()
		if _refused:
			return true
	var hover_frame := AFTER_MACHINE_HOVER_FRAME if _after_machine_hover else 2
	if _frames == hover_frame and not _hover_flags.is_empty():
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
	if _frames == 2 and _payday_warning:
		_raise_payday_warning()
		if _refused:
			return true
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
	if _frames == 2 and _cookbook:
		_main._set_cookbook_open(true)
		if _cookbook_page and not _open_first_unlocked_sauce():
			return true
		if _cookbook_basics and not _open_first_basics_topic():
			return true
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
	if _frames == 2 and _split_docks:
		_run_to_split_docks()
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
	if _frames == 3 and (_pause or _settings):
		_open_pause_menu()
		if _settings:
			_main._on_pause_settings_pressed()
	# After the pause menu, because it is allowed to be up beside it and the
	# frame that proves that is the one worth having.
	if _frames == 3 and _debug:
		_main._set_debug_open(true)
		_main._update_debug_readout()
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
	# `board`, not `contracts`: only a board entry carries `active` and
	# `times_lost`, so reading them off `contracts` counted 0 and 0 for every
	# frame and the lost-contract refusal below could never fire.
	for offer in economy.get("board", []):
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

	# WHEN, in the only clock the picture is made of. Two runs of one command
	# print the same number here, and a pair that ever stops printing it is a
	# pair whose difference is the harness rather than the change under
	# review — which is the whole of issue #82, and was invisible in this
	# output while it was true. The menu scene has no simulation to ask.
	if not _menu and _main != null and _main.sim != null:
		print("SCREENSHOT_TICK: %d" % _main.sim.current_tick())

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
