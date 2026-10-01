#!/usr/bin/env bash
# Renders godot_project/main.tscn to a PNG for visual review (docs/UI_UX.md).
#
# This is the one verification step in the project that is NOT headless, and
# deliberately so: everything else about this codebase is checked without a
# framebuffer (ARCHITECTURE.md §7), but "does the factory look like UI_UX.md
# describes" cannot be. It opens a real window (off-screen), waits for
# lighting to settle, writes the frame, and exits.
#
# **THE WINDOW IS OFF-SCREEN, AND FOCUS IS HANDED BACK.** These renders run on
# a laptop someone is using, so nothing here may land on top of their work or
# take their keystrokes. test_screenshot.gd moves its window off the desktop
# before drawing anything; its header says how. Godot still activates itself
# at launch, which nothing inside the process can prevent, so while it runs
# this script hides Godot the moment it is in front, and macOS gives focus
# back to whatever app had it (`hand_focus_back`). The cursor is not touched either: the --hover* flags aim
# a point inside the scene, not the machine's pointer. Pass --on-screen to
# watch a run in a normal window; that also leaves focus alone.
#
# **THE WORLD NOW STARTS EMPTY** (the "Real Placement" epic). With no flags
# at all this photographs exactly that — bare floor, a build menu key and
# nothing else — which is a real state worth reviewing, not a broken one.
#
# Pass --build (the same thing as --fixture demo) to lay down the demo line first: a hopper feeding belts that
# run east, TURN NORTH and then WEST into a crusher facing WEST which discharges back west,
# with a pipe run heading SOUTH beside it. Three of the four facings on belts
# and the fourth on the pipes, which makes it the right subject for reviewing
# per-tile rotation and the junction art's east-only fallback in one frame.
# **WHICH FIXTURE YOU GET, AND HOW TO KNOW** (issue #70). Every run prints
# `SCREENSHOT_FIXTURE: <name>` beside `SCREENSHOT_OK`, so a frame can be
# traced back to the world it came from. An explicit --fixture WINS over the
# convenience flags below, which each imply one: `--fixture understaffed
# --staff` is the understaffed line with the staff panel over it, and the run
# says so (`SCREENSHOT_NOTE: --fixture understaffed overrides --staff's own
# fixture`). Everything else the flag means — the panel it opens, the ticks
# it runs — still applies. A --fixture name that does not exist is REFUSED
# with the list of names and a non-zero status; it used to build the demo
# line and look plausible.
#
# Pass --fixture <machines|fluids|vat|splitter|splitter_modes|chain|stress|pipes|gallery|demo> for a different one:
# `machines` (a hopper, a machine and a dock), `fluids` (a tank, a pipe run
# and the machine tapping it), `vat` (the 2x2 cooking vat alone), `splitter`
# (a line that FORKS — one splitter with both outputs built and a second with
# only its straight one, so the next-output arrow's two behaviours are in one
# frame) or `chain` (THE WHOLE SHIPPED LINE — about 1,200 of a new game's
# 3,000 balance, bought and placed like anything else, with its piped middle
# and its two hoppers).
#
# Pass --contracts to build that full line and then run it until the contract
# board is genuinely mixed (some deliveries made, some missed, goods on the
# dock), so the clipboard can be reviewed against a working economy. It came
# back with `sim::STARTING_MONEY`'s rise from 200 to 3000: under the old
# balance no fixture could reach a factory that earns anything.
#
# Pass --payout to build that same line, run it until a contract genuinely
# pays out, and hold the frame partway through the celebration that fires:
# the coin burst over the dock, the rising "+N" over the money line, the
# money figure mid-punch and the paying docket reading PAID. This is the one
# check the payout work needs eyes on — test_payout_feedback.gd can prove
# every number and cannot say whether the burst reads as a fountain.
# It implies --fixture chain, and it FREEZES the scene once the frame is set
# up, since a burst is over in 34 ticks and the shot's warm-up is longer.
#
# Pass --crushing to build that same line, run it until the crusher is
# genuinely mid-batch, and hold the frame with its pulp in the air — the
# spray it throws on its own beat while it works. The same review --payout
# gets, one size down: test_machine_motion.gd can prove a fleck is thrown
# from the mouth and reclaimed, and cannot say whether the burst reads as
# satisfying crushing. It implies --fixture chain and --populate, and (like
# --payout) FREEZES the scene, since a fleck's flight is 9 ticks and the
# shot's warm-up is longer. A starved or blocked crusher throws nothing by
# design, which is why this waits for the state rather than a tick count.
#
# Pass --stock to buy an assorted kit (belts, pipes, a hopper, a tank, a
# machine or two) so the hotbar shows carried sockets, multi-digit counts and
# empty sockets side by side, plus uneven raw stock so the STOCK plate shows a
# stocked row beside an out-of-stock one, and --hotbar N to press hotbar key N for real —
# the only way to review the selected-slot state, the name flyout and the
# ghost it puts in the player's hand.
#
# Pass --legend-collapsed to fold the keybind legend away before the shot (the
# HUD ticket). It is the only way to review what the fold LEAVES: the plate's
# heading plus the button that names F1, and the corner of the floor the
# five lines of keybinds were covering.
#
# Pass --stress to build and run the STRESS LINE (a 1500-tile serpentine belt
# fed by hoppers, plus three tank-fed pipe runs — test_layouts.gd's
# `build_stress_line`), the fixture the instanced tile/item rendering is
# measured on. It implies --populate, and the run prints a SCREENSHOT_RENDER
# line with the frame's object, primitive and draw-call counts.
#
# Pass --pipe-gallery to build the PIPE GALLERY (test_layouts.gd's
# `build_pipe_gallery`): every pipe shape at every turn, a tank discharging
# into a pipe from each of four sides, a machine tapping a pipe from each
# side, and the shipped mixer -> vat -> bottler runs — the fixture the pipe
# presentation (the pipes & conveyors overhaul, ticket 6) is reviewed on.
# Add --populate to see liquid standing in the fed runs, and --closeup for
# any one joint; a whole-gallery frame is too small to judge a flange by.
# Pass --belt-gallery to build the BELT GALLERY (test_layouts.gd's
# `build_belt_gallery`): both hands of corner at all four facings, a belt
# meeting a hopper, a crusher and a dock at all four facings, a two-flank
# merge, a line start and a line end — every belt shape the pipes and
# conveyors overhaul draws, in one frame. Combine with --populate to see items
# following the corners' arcs, and with --closeup on one of its corners
# (they are at (0,0), (3,0), (3,3), (0,3) for right turns and (6,0), (9,0),
# (9,3), (6,3) for left) to judge one junction.
#
# Pass --fluid-gallery to build the FLUID GALLERY (test_layouts.gd's
# `build_fluid_gallery`): all SIX fluids standing in six full runs in one
# frame — vinegar, ketchup mix, ketchup, BBQ mix, liquid smoke and BBQ — so
# main.gd's FLUID_TINTS can be judged whole rather than three at a time,
# which is all the shipped line or the BBQ fixture ever shows (issue #72).
# It does not tick and does not need to: a run is NAMED by the recipes at
# its ends, and the volumes are set directly. --closeup on any of its runs
# reads one colour at size.
#
# Pass --warehouse to build the WAREHOUSE ROW (test_layouts.gd's
# `build_warehouse_row`): three stock racks holding three raws, two of them
# with every bay bound, the third with two of six, and a part-full load on
# the last bay of each ingredient. It is the frame that judges W3's readout:
# whether a bound bay is tellable from a bare shelf, whether a part load
# reads as part, and whether two ingredients in one rack are two colours.
# Fourteen of its eighteen bays are in use, so add --remove-mode for the
# second frame: W1's refusal, said in the placement prompt before the click. It frames itself close (the subject is one 1x2 tile) and keeps the
# HUD, and it does not tick — stock is bought, not made. The three BBQ raws
# are behind data/unlocks.ron's gate and cannot be bought in a fresh world,
# so they are not here; that every raw HAS a distinct rack tint is proved by
# test_warehouse_rack.gd instead.
#
# Pass --item-gallery to build the ITEM GALLERY (test_layouts.gd's
# `build_item_gallery`): a bare belt run with one of each FINISHED-GOODS
# model standing on it, ketchup bottle, BBQ bottle, ketchup case, BBQ case,
# at the size an item really renders on a belt. It is the only frame that
# can judge whether two sauces are tellable apart, and the only one that
# shows a case's shipping label at all — assets/voxels/previews/ is rendered
# from vengi's camera, the opposite corner to the game's (UI_UX.md §2b point
# 35), which is the face the labels are not on. Nothing feeds the run, so
# the items on it are exactly the ones the shot asked for; add --closeup
# 3,0,12 to fill the frame with them.
#
# Pass --bbq to build the BBQ LINE (test_layouts.gd's
# `build_bbq_running_line`): both sauces on one floor — the shipped ketchup
# line on row 0 and the BBQ branch three rows north of it, joined by the
# single pipe that taps finished ketchup out of the vat's outlet run — funded,
# fully crewed, auto-filled and run 15 000 ticks, so boxes of both are moving
# when the shutter opens. It implies --populate. Add --board --board-end to
# put the BBQ tier of the board over it, which is where the epic's mixed
# orders read.
#
# Pass --mustard to build the MUSTARD LINE (test_layouts.gd's
# `build_mustard_line`): the third sauce, and the first that is a whole
# SECOND line from raw rather than a branch off an existing one — seed hopper
# and vinegar tank into a crusher that discharges a FLUID, turmeric joining
# at the mixer, then vat, bottler, packer and its own dock — funded, fully
# crewed, auto-filled and run 15 000 ticks, so cases are shipping when the
# shutter opens. It implies --populate. It is the one fixture whose middle is
# entirely wet: mustard_mash and mustard_mix exist only in pipes, so this is
# the only frame that can show whether the three stages of one sauce are
# three colours. Add --closeup 7,0,14 to fill the frame with the piped half.
#
# Pass --stalled-run to build the STALLED RUN (test_layouts.gd's
# `build_stalled_pipe_run`): the BBQ branch with its liquid-smoke tank turned
# off its own run — issue #54's own mistake — so that run is one the
# simulation cannot name and every machine on it waits forever. It is the
# frame that shows a DEAD run (hazard clamps along it) beside the merely
# EMPTY runs either side of it, which used to look identical, with the cursor
# on the dead run so the hover gives the reason. Deliberately NOT populated:
# a factory run forward has no empty runs left to compare against.
#
# Pass --bottleneck to build the BOTTLENECK LINE (test_layouts.gd's
# `build_bottleneck_line`): the whole shipped line with its hoppers and tank
# on auto-fill, run until it has backed up behind the 240-tick cooking vat,
# so the machines read blocked · blocked · working · starved · starved
# along the line — the fixture the machine-load indicator is reviewed on.
# It implies --populate. Add --hover to put the crusher's load line (glyph,
# word, number, reason) and status-coloured load bar in the tooltip, which
# is where the indicator lives since the stack-light poles were removed.
#
# Pass --staff to build the shipped line, run it (implies --populate, so the
# meters read a line asking for its crew) and open the STAFF panel (H) over
# it — one row per category with its icon, hired count, assigned/asked-for
# meter, wage and HIRE/FIRE, and the wage bill and payday countdown in the
# footer (the staff pools MVP). Pass --understaffed to build the
# UNDERSTAFFED LINE (test_layouts.gd's `build_understaffed_line`: the same
# line on auto-fill with two manufacturing heads fired, run until the
# cooking vat is refused its crew) with the cursor on the vat, so the hover
# shows `▲ starved · … · short of manufacturing staff` over its crew line.
# It implies --populate.
#
# Pass --staff-shortfall to build the UNDERFUNDED LINE
# (test_layouts.gd's `build_underfunded_line`: the shipped line funded to
# less than its next payday's bill) and open the STAFF panel over it, so the
# footer's BEFORE-payday warning (`▲ next payday short by 6 — 3 heads go
# unpaid`) and the disabled HIRE buttons are what the shot reviews, rather
# than the meters --staff already covers (the quality pass, ticket 4). It
# implies --populate.
#
# Pass --payday-warning to photograph THE PAYDAY CONFIRMATION (2026-09-26) as a
# click raises it: the shipped line funded to its next bill plus a belt less
# one (test_layouts.gd's `build_payday_warning_line`), the build menu open, and
# a belt bought through `_buy_slot` — so the frame reviews the panel's heading,
# the figures the simulation answered with, the consequence sentence and both
# buttons, over the menu they interrupted. It REFUSES rather than shipping a
# frame with no panel, so a fixture funded wrong cannot pass unnoticed.
#
# Pass --short-staffed to build the same UNDERSTAFFED LINE with NO panel open
# and nothing hovered, so the always-visible SHORT-STAFFED strip on the
# clipboard (`▲ SHORT-STAFFED — hire 2 manufacturing  ·  press H`) is what the
# shot reviews — the one readout --staff and --understaffed cannot show,
# since both cover the screen with a panel or a tooltip. It implies
# --populate.
#
# Pass --mothballed to build the same UNDERSTAFFED LINE, throw the mothball
# switch on its cooking vat and open that vat's configuration panel — issue
# #19's escape hatch from the unpaid spiral, in the state a player reaches
# it in. The shot reviews the machine panel the click-to-configure epic's
# panel grew a third shape for: the switch reading MOTHBALLED: OFF against
# the hopper/tank rows being absent rather than disabled, the batch HELD
# where the switch caught it, and the sentence saying what the machine is
# now doing to the payroll. It implies --populate, so the rest of the line
# is running around the one machine that is not.
#
# Pass --unclerked-dock to build the reference line with its ONE logistics
# head FIRED, ticked only far enough to settle the scene. That is the window
# issue #87 was about: `understaffed` only moves on a tick the dock actually
# held something and was refused, so until a box reaches the dock — hundreds
# of ticks on a cold line, never on one starved upstream — the tooltip said
# nothing while the establishment could not crew the building. Pair it with
# --hover-dock; the subject of the frame is the "short of logistics staff"
# line now drawn from the STANDING `crewed` fact. The fixture refuses to
# build if that window has closed, so a sim change cannot hand back an
# ordinary-looking dock under this flag. --full-dock photographs the other,
# latched case.
#
# Pass --production to build the bottleneck line and open the PRODUCTION
# panel (P) over it: what the factory made and used per item over a
# window or all time, biggest first, with per-minute rates and bars (the
# production-stats epic — Factorio's P screen). It implies --populate.
#
# Pass --cookbook to open THE COOKBOOK (B) on its sauce tree over the
# reference line, --cookbook-page for the page behind the first unlocked
# node the tree draws — the ingredients, the machine process in seconds, the
# cost and what the contracts pay — or --cookbook-basics for the first of
# the book's prose pages, where the sentences come out of data/guide.ron and
# every number in them out of the sim. All three imply --populate. None of
# them names a sauce or a topic: the screen is data/*.ron rendered, and so
# is the flag.
#
# --cookbook-basics also takes an optional topic id (--cookbook-basics
# growing), for the same reason: the bare flag can only ever photograph the
# first topic data/guide.ron lists, so a change to any page below it has no
# frame. A topic id the guide does not ship is refused with the list it does.
#
# --cookbook-page also takes an optional id (--cookbook-page boxed_mustard),
# which opens that page instead. The bare form is still the default and is
# still wire-driven; the argument exists because "the first unlocked node"
# can only ever reach the FIRST family, and there are now pages behind it.
# Combine it with a fixture flag to choose the factory behind the plate:
# `--cookbook-page boxed_mustard --mustard`. An id the book does not hold is
# refused with the list of ids it does.
#
# Pass --split-docks to build the SPLIT-DOCK LINE (test_layouts.gd's
# `build_split_dock_line`): the BBQ branch, with its two docks, run on until
# ONE of them is backing up and the other is not. The fixture's mixed exit
# feeds BBQ onto the ketchup dock too, and that dock fills with the BBQ the
# orders cannot absorb, while the branch's own dock keeps taking boxes. Each
# dock holds its own stock, so only the jammed dock's lamp pulses and only
# it raises the floating "!". The clipboard still reads ⚠ FULL, correctly:
# a dock in the world is full.
# Honours --dock-lamp dark for the other half of the pulse, as --full-dock
# does, and refuses to claim the fixture if the docks never disagreed.
#
# Pass --full-dock to build the FULL-DOCK LINE (test_layouts.gd's
# `build_full_dock_line`): the shipped line on auto-fill, run until it has
# made boxes faster than its contracts ask for them and its dock has all 32
# places taken. That is the one state the
# dock-capacity change added and nothing else can reach on demand
# (--bottleneck leaves the dock part-filled): the clipboard's ON THE DOCK
# line reads 32/32 in warning red under `⚠ FULL — belt backing up`, the boxes
# stand still on the belt the dock has stopped taking from, and the dock's
# deck lamps PULSE instead of holding steady. The shot is FROZEN on a lit
# half of that pulse, since a pulse is a rhythm rather than a state and a
# stray frame would photograph a dark lamp; add --dock-lamp dark for the
# other half, and take both to see the blink. The lamps are floor-level, so
# --closeup 26,0,12 (the dock's tile) is the frame to judge them in.
#
# Pass --populate to run whatever was built forward first, so the shot shows
# product moving instead of an idle factory. Pass --first-person to review
# that camera mode's lighting/skybox. A shot is ISOMETRIC without it — this
# tool pins the view rather than inheriting main.gd's DEFAULT_VIEW_MODE (the
# view a SESSION opens in, first person since 2026-09-21), because every
# framing flag here steers the isometric camera and none of them can steer
# the other one. See test_screenshot.gd's `_enter_view_mode`.
# It combines with the --hover* flags, and there they TURN THE CAMERA rather
# than the pointer: in first person the tooltip is whatever the CROSSHAIR is
# over (main.gd queries the screen centre), so --first-person --hover-dock is
# "stand in the hall and look at the dock", and a --first-person shot with no
# hover flag keeps the pose it was given and simply reports what the
# crosshair found.
#
# Add --after-machine-hover to a --hover* flag to hover the first machine
# first and move on to the target a few frames later, so the shot shows a
# hover straight after a taller, wider one (issue #173).
#
# Pass --ghost KIND to enter placement mode with the ghost over open ground
# (the VALID state), or --ghost-blocked KIND with it over something already
# placed (the INVALID state), or --remove-mode to enter remove mode over a
# placed building. These are the only way to review build mode without a live
# interactive session — the same problem the hover flags solve for tooltips.
# Add --ghost-at TX,TY[,FACING] to hold that ghost over one chosen tile at one
# facing instead; the sim, not the flag, says whether it is valid there. The
# crossing's two frames are a pipe across the line's belt and along it:
#
#   scripts/screenshot-scene.sh valid.png --fixture crossing --ghost pipe --ghost-at 6,0,north
#   scripts/screenshot-scene.sh refused.png --fixture crossing --ghost pipe --ghost-at 6,0,east
#
# Pass one of --hover / --hover-pipe / --hover-vat / --hover-hopper /
# --hover-tank / --hover-dock / --hover-splitter to aim the cursor over the
# first placed machine, pipe, multi-tile machine's FAR CORNER, hopper, fluid
# tank, dock or conveyor splitter,
# so that tooltip is in the shot. (--hover-vat deliberately picks a
# non-origin tile: "any tile of a multi-tile machine identifies it" is the
# claim, and hovering the origin would not show it.)
#
# **A HOVER FLAG THAT FINDS NOTHING IS REFUSED**, with the flag, what it
# looked for and the world it looked in — no PNG, non-zero status. It used to
# aim at nothing at all, so `--fixture full_dock --hover-splitter` (that line
# has no splitter in it) came back as a clean frame of the dock with
# SCREENSHOT_OK on it, and plain `--hover` did the same against the empty
# world: a picture that answers a question nobody asked, which is the trap
# issue #70 was about. Which fixture contains what is above; the seven flags
# and their targets are one table, `godot_project/hover_targets.gd`, checked
# against every fixture headlessly by test_hover_targets.gd. --ghost-blocked
# and --remove-mode refuse on an empty world for the same reason: neither has
# anything to stand on, and --ghost-blocked's frame would be the VALID state.
#
# **THE CURSOR IS THIS RUN'S, NOT THE MACHINE'S** (issue #4). These flags
# used to warp the one real pointer, which outlived the process and leaked
# between runs. They now aim a point only the scene can see. Every run says
# where the cursor ended up beside SCREENSHOT_FIXTURE: `SCREENSHOT_CURSOR:
# hovering tile (26, 0)` for a hover flag, `parked on empty ground ...` for a
# run that asked for none. If the cursor is not over the tile it was aimed at
# when the shutter opens (the camera moved after the aim), the run is REFUSED
# with SCREENSHOT_FAIL and writes no PNG, because the tooltip in that frame
# belongs to the wrong thing.
#
# Parallel runs in several worktrees are fine. Each has its own window and
# its own cursor, and neither is on the desktop.
#
# Pass --title to photograph the TITLE SCREEN — the menu as the player first
# sees it: the wordmark, the splash line and the three buttons over the 3D
# factory backdrop that stands behind them (`title_scene.gd`). --menu is the
# same screen with its load list open.
#
# Pass --menu to photograph the MAIN MENU instead of the game — it renders
# menu.tscn, having first written a real save so its load list has a row in
# it, and removes that save again afterwards. Pass --pause to open the
# in-game pause menu (ESC) over whatever else was built.
#
# Pass --settings to open the SETTINGS page of whichever menu is being
# rendered — the pause menu's on its own, the main menu's with --menu. It is
# one screen hosted twice (`settings_panel.gd`), so both frames are worth
# taking. Pass --debug for the F3 DEBUG MENU over a running factory.
#
# EVERY run FORCES `Settings` BACK TO ITS DEFAULTS first, without writing
# anything: this script is the one check in the project with a real display,
# so it is the one that would otherwise render at whatever field of view,
# movement speed and hall size happen to be in the developer's own
# `user://settings.cfg` — and a review frame nobody else can reproduce is
# not a review frame.
#
# Pass --set KEY=VALUE (repeatable) to put one knob somewhere else for the
# render, which is the only way to photograph what a setting DOES:
#   --first-person --set hall_scale=2.5
# An unknown key is refused rather than ignored, like --fixture.
#
# Pass --shop to open the build menu (docs/UI_UX.md §5) before the shot, or
# --market to open the ingredient market — the stock half of the shop, where
# raw ingredients are bought (the chain-rework epic) — or --board to open the
# CONTRACTS BOARD, where contracts are signed and lost ones signed again (the
# contract-board epic). --contracts signs every offer before it runs the
# line, so the clipboard photographs the whole board competing. --board-end
# winds that board to its last offer: it lists contracts.ron in file order
# and shows about four at a time, so a tier appended to the end of the file
# is below the fold until something scrolls there.
# --board-filter presses one of the board's filter chips before the shot:
# all, open, signed or locked, or a product id such as boxed_mustard for its
# sauce chip. It refuses without --board, and refuses a value the board has
# no chip for.
#
# Pass --at-capacity to fill every unlocked raw ingredient to the ceiling the
# warehouse gives it (W1's cap, W4's words). With --market that is the frame
# where a crate reads AT CAPACITY, the header's bays meter is at its warning
# red with 0 free, and the STOCK plate marks each row FULL; the balance is
# left deliberately fat, because the point of the state is a purchase money
# cannot fix. Without the flag every fixture has room to spare and the crates
# read exactly as they did before the warehouse existed.
#
# Pass --unlocked to put the world past every progression gate in
# data/unlocks.ron (the shipped one opens after ten deliveries), so --market,
# --board and the larder photograph with the BBQ tier released — the "after"
# half of a before/after pair, where the same command without the flag is the
# "before". Pass --unlock-toast instead to cross the gate one frame before
# the shutter, which is the only way to catch the one-line announcement: it
# shows for six seconds and then fades.
#
# Pass --no-hud to hide the HUD entirely. Closeups are for reviewing whether
# a belt or a pipe actually meets a machine, and the contract clipboard and
# the hotbar cover the top-right and the bottom of the frame — which is where
# a junction lands about half the time.
#
# Pass --config TX,TY to click the building on that tile after the fixture
# is built — a hopper, a tank, a crewed machine or a warehouse rack — so the
# frame shows its configuration panel (the hopper/tank click-to-configure
# ticket). With an empty larder FILL is photographed blocked, with its
# reason; add --stock to see it enabled. --contents N on top presses the open
# panel's picker slot N, so a placed hopper is photographed retargeted to
# another raw ingredient, e.g.
#
#   scripts/screenshot-scene.sh out.png 1600x900 --build --config=-1,0 --contents 1
#
# On a rack, slot 0 is "Any" and slot 1 the first raw, so the warehouse row's
# tomato rack is photographed reserved for tomato with
#
#   scripts/screenshot-scene.sh out.png 1600x900 --warehouse --config=0,0 --contents 1
#
# --contents-refill is only needed when the picker slot *changes* the declared
# fluid, which is the destructive case: it buys the newly declared fluid and
# pumps it back through the run, so the picker's note is shot saying what a
# *second* switch would destroy ("switching destroys 89.7 units — this tank
# and the run it feeds"). Re-pressing the slot a tank already carries is a
# no-op in the sim since #128 — the tank keeps its volume — so that frame
# already shows the quantified note without this flag.
#
# (the demo line's hopper stands at (-1, 0); note the `=` form, since a
# value starting with `-` would otherwise be read as another flag. Note also
# that the RESOLUTION IS POSITIONAL and must be given whenever any flag is:
# this script reads $2 as the resolution and passes only "${@:3}" through, so
# `out.png --build` hands `--build` to `godot --resolution` and the flag is
# silently lost.)
#
# Pass --closeup TX,TY[,SIZE] to aim the isometric camera at one simulation
# tile and zoom right in (SIZE is the orthographic width in world units;
# fractional tile coordinates are allowed, because a junction sits ON a tile
# boundary). The whole-factory shot is the right frame for "does the line
# read" and the wrong one for "does this belt actually meet this machine",
# e.g.
#
#   scripts/screenshot-scene.sh out.png 1100x800 --build --closeup 0.5,0.3,10
#
# Usage: scripts/screenshot-scene.sh [output.png] [WIDTHxHEIGHT] [--build] [--fixture NAME] [--stress] [--pipe-gallery] [--belt-gallery] [--fluid-gallery] [--item-gallery] [--warehouse] [--bottleneck] [--bbq] [--mustard] [--stalled-run] [--staff] [--staff-shortfall] [--payday-warning] [--production] [--cookbook|--cookbook-page [ID]|--cookbook-basics [ID]] [--understaffed] [--short-staffed] [--mothballed] [--unclerked-dock] [--full-dock [--dock-lamp dark]] [--split-docks [--dock-lamp dark]] [--contracts] [--payout] [--crushing] [--populate] [--first-person] [--ghost KIND|--ghost-blocked KIND|--remove-mode] [--hover|--hover-pipe|--hover-vat|--hover-hopper|--hover-tank|--hover-dock|--hover-splitter] [--title] [--menu] [--pause] [--settings] [--debug] [--set KEY=VALUE] [--shop] [--market] [--board [--board-end] [--board-filter all|open|signed|locked|ITEM]] [--unlocked] [--unlock-toast] [--no-hud] [--stock] [--at-capacity] [--hotbar N] [--legend-collapsed] [--config TX,TY [--contents N [--contents-refill]]] [--closeup TX,TY[,SIZE]] [--allow-unhealthy] [--on-screen]
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Defaults into the same gitignored previews/ directory the voxel-model
# thumbnails go to — all of it is regenerable review output, none of it ships.
out="${1:-$repo_root/assets/voxels/previews/scene.png}"
resolution="${2:-1600x900}"
godot_bin="${GODOT_BIN:-godot}"

case "$out" in
  /*) ;;
  *) out="$repo_root/$out" ;;
esac
mkdir -p "$(dirname "$out")"
# An older frame at this path would pass the "was it written" check below
# after a refused or crashed render, and be reported as this run's output.
rm -f "$out"

# THE CLOCK IS PINNED, AND THAT IS WHAT MAKES THIS REPRODUCIBLE (issue #82).
# `--fixed-fps` tells the engine to hand every frame the same delta and stop
# synchronising to real time, so the scene's warm-up spends a fixed amount of
# SIMULATED time before the shutter however long those frames actually took.
# Without it the number of ticks between "fixture built" and "shutter" is
# whatever the host's load allowed, and two runs of one command came back one
# tick — and several moved tomatoes — apart, which is enough to swamp the
# change a before/after pair is being read for.
#
# 60, matching `test_screenshot.gd`'s FIXED_FPS, which REFUSES a run whose
# frames did not arrive at that rate: this flag is the only thing standing
# between a review render and a lottery, so a hand-rolled `godot --script
# test_screenshot.gd` that forgets it is told rather than quietly obeyed.
fixed_fps=60

# Godot activates itself at launch, and its no-focus window then leaves no
# app in front at all. So whenever Godot is the front app, HIDE it (what
# Cmd-H does). macOS then gives focus back to the app that had it, which is
# the one thing a script cannot reliably name. Handing focus to a named app
# instead was tried: Godot took it back each time. Polled, because the
# activation lands a moment after launch, at a time this script cannot see.
# Nothing here needs the window to be visible; test_screenshot.gd draws it by
# hand.
hand_focus_back() {
  local godot_pid=$1 front
  while kill -0 "$godot_pid" 2>/dev/null; do
    front=$(lsappinfo info -only pid "$(lsappinfo front)" 2>/dev/null \
      | sed -n 's/.*"pid"=\([0-9]*\).*/\1/p')
    if [ "$front" = "$godot_pid" ]; then
      osascript -e "tell application \"System Events\" to set visible of \
        (first process whose unix id is $godot_pid) to false" >/dev/null 2>&1
    fi
    sleep 0.05
  done
}

"$godot_bin" --path godot_project --resolution "$resolution" \
  --fixed-fps "$fixed_fps" \
  --script test_screenshot.gd -- "$out" "${@:3}" &
godot_pid=$!
# Backgrounded for the watcher, so an interrupt has to reach Godot by hand.
trap 'kill "$godot_pid" 2>/dev/null' INT TERM
watcher_pid=""
if command -v lsappinfo >/dev/null && [[ " ${*:3} " != *" --on-screen "* ]]; then
  hand_focus_back "$godot_pid" &
  watcher_pid=$!
fi
wait "$godot_pid"
status=$?
[ -n "$watcher_pid" ] && wait "$watcher_pid" 2>/dev/null
if [ ! -f "$out" ]; then
  echo "error: no screenshot written to $out (godot exited $status)" >&2
  exit 1
fi
echo "wrote $out"
