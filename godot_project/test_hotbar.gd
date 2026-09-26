# Headless structural proof of the inventory hotbar (docs/UI_UX.md §5 — "a
# Minecraft-style hotbar/quick-access strip along one screen edge", the last
# of that section's named widgets to be built).
#
# Run with:
#   godot --headless --path godot_project --script test_hotbar.gd
#
# Like test_shop_menu.gd, this deliberately does NOT claim the strip *looks*
# right — that needs eyes on `scripts/screenshot-scene.sh --hotbar 1`. What it
# verifies is everything checkable without a framebuffer:
#
#   * the strip is its own always-visible node, built at `_ready` and never
#     hidden behind a menu, with one slot per `shop_snapshot_json` catalog
#     entry in the snapshot's own order and a real voxel icon in each,
#   * the number keys 1..9 and 0 label the first ten slots, and are the SAME
#     index the build menu prints on the same kind,
#   * counts are live: buying moves a slot's count text, and a kind owned in
#     quantity 0 shows no number at all (an empty socket, not a slot reading
#     zero),
#   * **the behaviour change this ticket is for** — a number key selects a
#     slot and enters placement mode WITH NO MENU OPEN, which is what was
#     impossible before: the keys used to do nothing unless the build menu
#     was up,
#   * pressing the selected slot's key again deselects, mirroring ESC, and
#     ESC/X/E leaving build mode by their own routes clears the strip's
#     highlight too (the selection is derived from `_build_kind`, not stored),
#   * an empty socket refuses selection rather than showing an unbuildable
#     ghost,
#   * clicking a slot does exactly what its key does,
#   * selection is signalled by more than colour (UI_UX.md §7): a distinct
#     slot style, a visible caret glyph, and the name flyout above the strip,
#   * buying is still reachable and is still a DIFFERENT verb — a bare number
#     key never spends money (covered end-to-end in test_shop_menu.gd), and
#     the control legend says how selection works in both view modes.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("HOTBAR_FAIL: %s" % msg)
	quit(1)


func _shop_snapshot(main) -> Dictionary:
	return JSON.parse_string(main.sim.shop_snapshot_json())


func _press(main, keycode: int, shift := false) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	event.shift_pressed = shift
	main._unhandled_input(event)


func _buy(main, kind: String, quantity := 1) -> void:
	main.sim.issue_command_json(JSON.stringify({
		"type": "purchase", "kind": kind, "quantity": quantity,
	}))
	main._refresh_owned_counts()
	main._update_hotbar()


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	# --- The strip exists, is always on, and is its own node --------------
	var root_node: VBoxContainer = main._hotbar_root
	if root_node == null:
		_fail("main._hotbar_root was not built — there is no hotbar")
		return
	if root_node.get_parent() != hud:
		_fail("HotbarRoot must be a direct child of the HUD CanvasLayer")
		return
	if main._hotbar_panel == null or main._hotbar_panel.name != "Hotbar":
		_fail("expected the strip's plate to be a node named 'Hotbar'")
		return
	# The whole point of a hotbar: it is visible without opening anything.
	if not root_node.visible or not main._hotbar_panel.visible:
		_fail("the hotbar must be visible from the start, with no menu opened")
		return
	if main._shop_open:
		_fail("the build menu must still start closed — the hotbar does not open it")
		return
	if main._hotbar_panel == main._shop_menu or main._hotbar_panel == main._contract_panel:
		_fail("the hotbar must be a genuinely separate node from the build menu and the clipboard")
		return

	# --- One slot per catalog entry, in the snapshot's own order ----------
	var snapshot: Dictionary = _shop_snapshot(main)
	var entries: Array = snapshot.get("entries", [])
	if entries.is_empty():
		_fail("shop_snapshot_json reported an empty catalog — nothing to build a hotbar from")
		return
	if main._hotbar_slots.size() != entries.size():
		_fail("expected %d hotbar slots (one per catalog entry), found %d" % [
			entries.size(), main._hotbar_slots.size(),
		])
		return

	# The catalog OUTGREW the number row when Epic B priced the splitter (it is
	# twelve kinds against ten keys), so the overflow this constant was always
	# written for is now live rather than hypothetical. It has to degrade the
	# way the constant promises: the tail is keyless and click-only, and — the
	# part worth checking rather than assuming — no keycode resolves to a slot
	# that prints no key, which would be a number key silently selecting the
	# wrong kind.
	if entries.size() <= main.SHOP_HOTKEY_SLOTS:
		print("HOTBAR_NOTE: the catalog (%d) no longer overflows the %d hotkey slots" % [
			entries.size(), main.SHOP_HOTKEY_SLOTS,
		])
	for keycode in main.HOTBAR_KEYCODES:
		var index: int = main._hotbar_index_for_keycode(keycode)
		if index < 0 or index >= main.SHOP_HOTKEY_SLOTS:
			_fail("keycode %d resolves to hotbar slot %d, outside the %d keyed slots" % [
				keycode, index, main.SHOP_HOTKEY_SLOTS,
			])
			return
	for index in range(main.SHOP_HOTKEY_SLOTS, entries.size()):
		if main._hotbar_key_label(index) != "":
			_fail("slot %d is past the key row but still prints a key" % index)
			return
		if main.HOTBAR_KEYCODES.size() > index:
			_fail("slot %d is past the key row but a keycode exists for it" % index)
			return

	var strip: Node = main._hotbar_panel.find_child("HotbarSlots", true, false)
	if strip == null:
		_fail("no HotbarSlots container found under Hotbar")
		return
	if strip.get_child_count() != entries.size():
		_fail("HotbarSlots holds %d children, expected %d slot buttons" % [
			strip.get_child_count(), entries.size(),
		])
		return

	for index in range(entries.size()):
		var slot: Dictionary = main._hotbar_slots[index]
		if String(slot["kind"]) != String(entries[index]["kind"]):
			_fail("hotbar slot %d is '%s' but the catalog's entry %d is '%s' — the strip must follow the snapshot" % [
				index, slot["kind"], index, entries[index]["kind"],
			])
			return
		if not (slot["button"] is Button):
			_fail("hotbar slot '%s' is not a Button, so it cannot be clicked" % slot["kind"])
			return
		var icon: TextureRect = slot["icon"]
		if icon.texture == null or icon.texture.get_width() < 16:
			_fail("hotbar slot '%s' has no real generated icon" % slot["kind"])
			return
		# The strip and the build menu print the same key on the same kind,
		# because they are the same index into the same catalog.
		var expected_key := "" if index >= main.SHOP_HOTKEY_SLOTS else str((index + 1) % 10)
		if String(slot["key_label"].text) != expected_key:
			_fail("hotbar slot %d should print key '%s', prints '%s'" % [
				index, expected_key, slot["key_label"].text,
			])
			return
		var menu_key: Label = main._shop_slots[index]["button"].find_child("Hotkey", true, false)
		if menu_key != null and String(menu_key.text) != expected_key:
			_fail("build menu slot %d prints key '%s' but the hotbar prints '%s' — they must agree" % [
				index, menu_key.text, expected_key,
			])
			return

	# --- An unowned kind is an EMPTY SOCKET, not a slot reading zero ------
	# A new game owns nothing at all, so every slot starts empty.
	main._update_hotbar()
	for slot in main._hotbar_slots:
		if String(slot["count_label"].text) != "":
			_fail("slot '%s' owns none but shows a count of '%s' — an empty socket shows nothing" % [
				slot["kind"], slot["count_label"].text,
			])
			return
		if String(slot["state"]) != "empty":
			_fail("slot '%s' owns none but is in state '%s'" % [slot["kind"], slot["state"]])
			return
		if slot["icon"].modulate.a >= 1.0:
			_fail("slot '%s' owns none but its icon is drawn at full strength" % slot["kind"])
			return

	# --- An empty socket refuses selection --------------------------------
	_press(main, main.HOTBAR_KEYCODES[0])
	if main._in_build_mode():
		_fail("selecting a slot the player owns none of must not enter placement mode")
		return

	# --- Counts are live --------------------------------------------------
	var kind: String = String(main._hotbar_slots[0]["kind"])
	_buy(main, kind, 3)
	if String(main._hotbar_slots[0]["count_label"].text) != "3":
		_fail("after buying 3 '%s' the slot should read '3', reads '%s'" % [
			kind, main._hotbar_slots[0]["count_label"].text,
		])
		return
	if String(main._hotbar_slots[0]["state"]) != "stocked":
		_fail("a slot with stock should not still be an empty socket")
		return

	# --- THE TICKET: a number key selects, with no menu open --------------
	if main._shop_open:
		_fail("the build menu should still be closed at this point")
		return
	_press(main, main.HOTBAR_KEYCODES[0])
	if not main._in_build_mode():
		_fail("pressing a hotbar key with no menu open must enter placement mode — this is the whole ticket")
		return
	if main._build_kind != kind:
		_fail("pressing hotbar key 1 selected '%s', expected '%s'" % [main._build_kind, kind])
		return
	if main._shop_open:
		_fail("selecting from the hotbar must not open the build menu")
		return
	if main._hotbar_selected_index() != 0:
		_fail("the strip does not know slot 0 is selected")
		return

	# --- Selection is signalled by more than colour (UI_UX.md §7) ---------
	var selected_slot: Dictionary = main._hotbar_slots[0]
	if String(selected_slot["state"]) != "selected":
		_fail("the selected slot is in state '%s', expected 'selected'" % selected_slot["state"])
		return
	if not selected_slot["mark"].visible:
		_fail("the selected slot must carry its caret glyph, not rely on colour alone")
		return
	for index in range(1, main._hotbar_slots.size()):
		if main._hotbar_slots[index]["mark"].visible:
			_fail("slot %d shows the selection caret while slot 0 is selected" % index)
			return
	# The name flyout: Minecraft's, and the reason a player can read what is
	# in hand without looking down at a 64px icon.
	if main._hotbar_flyout == null or not main._hotbar_flyout.text.to_lower().contains(
			main._kind_display_name(kind).to_lower()):
		_fail("the name flyout should announce '%s', reads '%s'" % [
			main._kind_display_name(kind),
			"" if main._hotbar_flyout == null else main._hotbar_flyout.text,
		])
		return
	if main._hotbar_flyout.modulate.a <= 0.0:
		_fail("the name flyout should be visible immediately after a selection")
		return

	# --- Pressing the selected slot's key again deselects -----------------
	_press(main, main.HOTBAR_KEYCODES[0])
	if main._in_build_mode():
		_fail("pressing the selected slot's key again must leave placement mode, as ESC does")
		return
	if main._hotbar_selected_index() != -1:
		_fail("nothing should be selected after deselecting")
		return
	if main._hotbar_slots[0]["mark"].visible:
		_fail("the caret must clear when the slot is deselected")
		return

	# --- Clicking a slot does what its key does ---------------------------
	main._hotbar_slots[0]["button"].pressed.emit()
	if not main._in_build_mode() or main._build_kind != kind:
		_fail("clicking a hotbar slot should select it exactly as its key does")
		return
	main._hotbar_slots[0]["button"].pressed.emit()
	if main._in_build_mode():
		_fail("clicking the selected slot again should deselect it")
		return

	# --- Selection is DERIVED, so every exit route clears the strip -------
	_press(main, main.HOTBAR_KEYCODES[0])
	_press(main, KEY_ESCAPE)
	if main._hotbar_selected_index() != -1 or main._hotbar_slots[0]["mark"].visible:
		_fail("ESC left the hotbar still showing a selection")
		return
	_press(main, main.HOTBAR_KEYCODES[0])
	_press(main, KEY_X) # remove mode replaces placement mode
	if main._hotbar_selected_index() != -1 or main._hotbar_slots[0]["mark"].visible:
		_fail("entering remove mode left the hotbar still showing a selection")
		return
	_press(main, KEY_X) # back out of remove mode

	# --- Selecting a second kind moves the highlight, it does not add one -
	var second := -1
	for index in range(1, main._hotbar_slots.size()):
		if index < main.SHOP_HOTKEY_SLOTS:
			second = index
			break
	var second_kind: String = String(main._hotbar_slots[second]["kind"])
	_buy(main, second_kind, 1)
	_press(main, main.HOTBAR_KEYCODES[0])
	_press(main, main.HOTBAR_KEYCODES[second])
	if main._build_kind != second_kind:
		_fail("switching slots should put '%s' in hand, holding '%s'" % [second_kind, main._build_kind])
		return
	if main._hotbar_slots[0]["mark"].visible:
		_fail("switching slots must move the caret, not leave two slots marked")
		return
	main._exit_build_mode()

	# --- The mode tab beside the strip follows every mode change -----------
	# Driven through the real keys, and the expected mode is named here rather
	# than read back from `_build_mode()`, so a tab that printed whatever that
	# function said would still fail if the function stopped tracking the keys.
	var tab: PanelContainer = main._mode_indicator
	if tab == null or not main._hotbar_root.is_ancestor_of(tab):
		_fail("there is no mode tab in the hotbar's column")
		return
	var seen := {}
	for step in [
		[-1, "select"],
		[main.HOTBAR_KEYCODES[0], "place"],
		[KEY_X, "remove"],
		[KEY_X, "select"],
		[main.HOTBAR_KEYCODES[0], "place"],
		[KEY_ESCAPE, "select"],
	]:
		if step[0] != -1:
			_press(main, step[0])
		var mode: String = step[1]
		var entry: Dictionary = main.BUILD_MODES[mode]
		var text: String = main._mode_indicator_label.text
		if not text.contains(entry["glyph"]) or not text.contains(entry["word"]):
			_fail("in %s mode the tab should read '%s %s', reads '%s'" % [mode, entry["glyph"], entry["word"], text])
			return
		# UI_UX.md §7: the rule colour changes too, but never alone.
		var style: StyleBoxFlat = tab.get_theme_stylebox("panel")
		if style == null or style.border_color != entry["rule"]:
			_fail("in %s mode the tab's rule should be that mode's colour" % mode)
			return
		seen[text] = true
	if seen.size() != main.BUILD_MODES.size():
		_fail("every mode should read differently on the tab, saw %s" % [seen.keys()])
		return

	# --- The hover tooltip carries the two verbs --------------------------
	# This is what makes the rebinding discoverable without reading source.
	main._on_hotbar_slot_hovered(0)
	if main._hotbar_tooltip == null or not main._hotbar_tooltip.visible:
		_fail("hovering a hotbar slot should show its tooltip")
		return
	var body: String = main._hotbar_tooltip_body.text.to_lower()
	if not main._hotbar_tooltip_name.text.to_lower().contains(main._kind_display_name(kind).to_lower()):
		_fail("the hotbar tooltip should name the kind, reads '%s'" % main._hotbar_tooltip_name.text)
		return
	if not body.contains("select"):
		_fail("the hotbar tooltip must say how to select, reads '%s'" % body)
		return
	if not body.contains("buy"):
		_fail("the hotbar tooltip must say where to buy — selecting and buying are different verbs — reads '%s'" % body)
		return
	main._on_hotbar_slot_unhovered(0)
	if main._hotbar_tooltip.visible:
		_fail("the hotbar tooltip should hide when the cursor leaves the slot")
		return

	# --- The legend describes it, in BOTH view modes ----------------------
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		var lower: String = legend.to_lower()
		if not lower.contains("hotbar"):
			_fail("a control legend never mentions the hotbar: '%s'" % legend)
			return
		if not lower.contains("1-9"):
			_fail("a control legend never names the hotbar keys: '%s'" % legend)
			return

	print("HOTBAR_OK: an always-visible %d-slot strip built from the catalog, empty sockets show no count, a number key selects with no menu open and again to deselect, clicks match keys, every exit route clears the caret, the mode tab follows every mode, and the tooltip carries both verbs" % main._hotbar_slots.size())
	quit(0)
