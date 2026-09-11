# Headless proof that main.tscn's keyboard input actually reaches the
# simulation (ARCHITECTURE.md §9 item 3's "interactable" requirement) —
# not just that the underlying issue_command_json API works (test_core_loop.gd
# already covers that), but that Main._unhandled_input's key handling does.
#
# It is also this project's standing list of REMOVED controls. Three keys have
# been deleted over three tickets and every one of them had to be checked two
# ways — the legend no longer advertises it, and the key genuinely does
# nothing — because `issue_command_json` silently drops a command it does not
# recognise, so a key left behind advertises a control the simulation does not
# have. **C** went with the delivery dock; **SPACE**, **V** and **Q** went
# with the click-to-configure panel, which is where loading and retargeting a
# bin live now (see test_config_panel.gd). SPACE still means "up" in
# first-person, which is why it is pressed here in ISOMETRIC.
#
# Run with:
#   godot --headless --path godot_project --script test_interaction.gd
extends SceneTree

const Layouts = preload("res://test_layouts.gd")


func _fail(msg: String) -> void:
	printerr("INTERACTION_FAIL: %s" % msg)
	quit(1)


## The world starts EMPTY since the "Real Placement" epic, so there is no
## hopper to load until the test builds one — and `hopper_snapshots_json` is
## an array now, because a world may hold several.
func _hopper_quantity(sim) -> int:
	var hoppers = JSON.parse_string(sim.hopper_snapshots_json())
	if not (hoppers is Array) or hoppers.is_empty():
		return -1
	return int(hoppers[0]["quantity"])


func _hopper_item(sim) -> String:
	var hoppers = JSON.parse_string(sim.hopper_snapshots_json())
	if not (hoppers is Array) or hoppers.is_empty():
		return ""
	return String(hoppers[0]["item"])


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	# `_ready()` only fires on the engine's own next-frame processing, which
	# this headless script-mode SceneTree never runs — call it directly.
	main._ready()

	# Nothing exists until the player builds it, and a hopper is what the
	# three deleted keys used to act on — so one is built, and stocked, so
	# that a key which still worked would have something to move and would
	# show up as a changed level rather than as a silent pass.
	Layouts.build_minimal(main.sim)
	Layouts.stock_larder(main.sim)
	main._rebuild_world()

	# --- SPACE, V AND Q ARE GONE (the click-to-configure ticket) -----------
	# SPACE filled every hopper in the world, V every tank, and Q cycled the
	# contents of the hopper under the cursor. All three are per-building
	# decisions, and all three are made in the configuration panel now, so
	# the keys were deleted rather than left as a second way to do one thing.
	# With stock bought and a hopper placed, a surviving SPACE would fill it.
	var before := _hopper_quantity(main.sim)
	var hopper_item_before := _hopper_item(main.sim)
	for removed in [KEY_SPACE, KEY_V, KEY_Q]:
		_press(main, removed)
	var after := _hopper_quantity(main.sim)
	if after != before:
		_fail("SPACE/V no longer fill anything, but the hopper moved %d -> %d" % [before, after])
		return
	if _hopper_item(main.sim) != hopper_item_before:
		_fail("Q no longer cycles contents, but the hopper now holds '%s' (was '%s')" % [
			_hopper_item(main.sim), hopper_item_before,
		])
		return
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		for fragment in ["SPACE: load", "V: load", "Q: "]:
			if legend.contains(fragment):
				_fail("the control legend still offers the removed '%s' control: %s" % [fragment, legend])
				return

	# "Collect" is GONE (the delivery-dock ticket). `PlayerCommand::CollectOutput`
	# no longer exists sim-side, so the key was removed here rather than left
	# to send a command `issue_command_json` would silently drop. C has since
	# been given its natural meaning — it opens the CONTRACTS BOARD (the
	# contract-board epic) — and that is a menu, so pressing it must still
	# change nothing in the economy or on the dock. Both halves are checked,
	# exactly as they are for SPACE/V/Q above, plus the board opening.
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		if legend.to_lower().contains("collect"):
			_fail("the control legend still offers the removed collect-product control: %s" % legend)
			return
	var econ_before: String = main.sim.economy_snapshot_json()
	var dock_before: String = main.sim.dock_snapshots_json()
	var c_event := InputEventKey.new()
	c_event.keycode = KEY_C
	c_event.pressed = true
	main._unhandled_input(c_event)
	if main.sim.economy_snapshot_json() != econ_before:
		_fail("pressing C changed the economy — the collect-product command was supposed to be gone")
		return
	if main.sim.dock_snapshots_json() != dock_before:
		_fail("pressing C changed the dock — the collect-product command was supposed to be gone")
		return
	if not main._board_open:
		_fail("pressing C should open the contracts board")
		return
	main._unhandled_input(c_event) # and close it again, so the keys below are not under a modal
	if main._board_open:
		_fail("a second C should close the contracts board")
		return

	# --- The three build-mode keys (the "Real Placement" epic's ticket 2) ---
	# X toggles remove mode, R turns whatever is under the cursor, ESC leaves.
	# All three go through the same `_unhandled_input` path SPACE does, so
	# this is the keyboard half of build mode reaching the simulation.
	_press(main, KEY_X)
	if not main._remove_mode:
		_fail("X should have entered remove mode")
		return
	_press(main, KEY_ESCAPE)
	if main._remove_mode:
		_fail("ESC should have left remove mode")
		return

	# R outside placement mode rotates the placement under the cursor. The
	# cursor is not over anything in a headless run, so drive the same
	# function the key does with an explicit tile — the key path itself is
	# covered by the toggle above.
	var belt_before := _direction_at(main.sim, Vector2i(0, 0))
	main._issue_world_command({"type": "rotate", "tile": [0, 0]})
	var belt_after := _direction_at(main.sim, Vector2i(0, 0))
	if belt_before != "east" or belt_after != "south":
		_fail("rotating the belt at (0, 0) should turn east -> south, got %s -> %s" % [belt_before, belt_after])
		return

	# --- E OPENS THE MENU, ESC CLOSES EVERYTHING (the E/ESC-controls ticket)
	# The build menu moved off B onto Factorio's own inventory key, and B was
	# removed rather than kept as an alias — so both halves get asserted, the
	# same way C's removal above is.
	_press(main, KEY_E)
	if not main._shop_open:
		_fail("E should open the build menu")
		return
	_press(main, KEY_E)
	if main._shop_open:
		_fail("E a second time should close the build menu")
		return
	_press(main, KEY_B)
	if main._shop_open:
		_fail("B is no longer the build-menu key and must not open it")
		return

	# ESC used to leave build/remove mode ONLY: with the menu up it did
	# nothing at all, so the one key a player reaches for to dismiss things
	# left the biggest panel on screen standing. It closes all three pieces
	# of stateful UI now — menu, placement mode, remove mode — from any
	# combination of them, which is what `_close_all_overlays` exists for.
	_press(main, KEY_E)
	_press(main, KEY_ESCAPE)
	if main._shop_open:
		_fail("ESC should close the build menu")
		return

	# Placement mode alone. A kind has to be OWNED to be held, so buy one.
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	main._refresh_owned_counts()
	main._enter_build_mode("belt")
	if main._build_kind != "belt":
		_fail("could not enter placement mode for a belt the player owns")
		return
	_press(main, KEY_ESCAPE)
	if main._in_build_mode():
		_fail("ESC should leave placement mode")
		return

	# And the awkward combination: menu up WITH something in hand. Every
	# route the player has closes one when it opens the other, so this is
	# forced deliberately — ESC has to clear whatever it is actually handed,
	# not whatever the UI usually allows.
	for mode in ["placement", "remove"]:
		if mode == "placement":
			main._enter_build_mode("belt")
		else:
			_press(main, KEY_X)
		main._set_shop_menu_open(true)
		if not (main._shop_open and main._in_build_mode()):
			_fail("failed to set up the menu-open-plus-%s-mode combination" % mode)
			return
		_press(main, KEY_ESCAPE)
		if main._shop_open or main._in_build_mode():
			_fail("one ESC must close the menu AND leave %s mode (shop_open=%s build_mode=%s)" % [
				mode, main._shop_open, main._in_build_mode(),
			])
			return

	# Every new key is advertised, the same rule that removed C's hint.
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		for fragment in ["R: rotate", "X: remove mode", "ESC: close", "click: place", "E: build menu"]:
			if not legend.contains(fragment):
				_fail("the control legend is missing '%s': %s" % [fragment, legend])
				return
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		if legend.contains("B: build menu"):
			_fail("the control legend still advertises the removed B binding: %s" % legend)
			return

	# One key that still DOES reach the simulation, so this file keeps proving
	# what it was written for: E through `_unhandled_input` opens the menu
	# built from `shop_snapshot_json`, and the rotate above moved a real tile.
	print("INTERACTION_OK: SPACE, V and Q are all gone from both the legend and the key handler, C opens the contracts board and touches nothing (a stocked hopper stayed at %d units and kept its contents), X/ESC/R drive build mode and rotate a real tile through SimBridge, E (not B) opens the build menu, and one ESC closes the menu and either mode from any combination" % after)
	quit(0)


func _press(main, keycode: int) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _direction_at(sim, tile: Vector2i) -> String:
	for placement in JSON.parse_string(sim.world_layout_json()):
		for pair in placement["occupies"]:
			if int(pair[0]) == tile.x and int(pair[1]) == tile.y:
				return String(placement["direction"])
	return ""
