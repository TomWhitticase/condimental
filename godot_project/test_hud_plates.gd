# Headless proof of the two HUD plates the "stock on the right, legend that
# folds" ticket built (docs/UI_UX.md §5, §5b, §5e, §7 and §2b point 33):
#
#   * the STOCK plate — a permanent right-hand column under the contracts
#     clipboard, one row per RAW ingredient the simulation prices, replacing
#     the one-line larder strip that used to sit on the hotbar, and
#   * the collapsible CONTROLS legend — the keybind plate, folded to a
#     labelled stub by its own button or by F1, and brought back the same
#     two ways.
#
# Run with:
#   godot --headless --path godot_project --script test_hud_plates.gd
#
# What it asserts:
#
#   * the stock plate is a HUD plate of its own, a sibling of the clipboard
#     and NOT a child of the hotbar's column, with no second copy of the
#     stock left behind on the hotbar (one readout of one fact in one place),
#   * one row per ingredient on `ingredients_snapshot_json`, each with an
#     icon, the DISPLAY name (never the wire id) and a count,
#   * counts follow bought stock, an unbought ingredient reads the WORD
#     `none` and its row dims, and a row's colour is its OWN — the strip
#     shared one `LabelSettings` across every chip, so one out-of-stock
#     ingredient greyed the counts beside it,
#   * the plate clears the clipboard above it and the hotbar below it at
#     1600x900 AND at 1280x720, on the right-hand edge in both,
#   * both capped lists in that column — the clipboard's docket stack and the
#     plate's own rows — are cut BETWEEN two rows and never through one, at
#     both window sizes and with the SHORT-STAFFED strip up as well as down
#     (issue #8: a card sliced through a line of text reads as a rendering
#     fault, not as "there is more, scroll"),
#   * the legend folds and unfolds from the button and from F1 (through the
#     real `_unhandled_input`), in isometric and in first person, and the
#     stub names the key that brings it back,
#   * folding is not a modal: it neither closes an open menu nor is closed
#     by ESC, and F1 works with a menu open,
#   * everything that measures the legend reads its CURRENT rect — a panel
#     near the cursor may use the space a folded legend gave up, and still
#     never lands on the stub or on the stock plate (§2b point 33).
#
# What it does NOT assert is how any of it looks — that is
# `scripts/screenshot-scene.sh --stock --populate` (and its new
# `--legend-collapsed`) and reading the PNGs.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

# The shipped window, and the small one §5c calls the tight case.
const BIG := Vector2i(1600, 900)
const SMALL := Vector2i(1280, 720)

## A staff snapshot with something short in it, to raise the SHORT-STAFFED
## strip on the clipboard. Shaped like the wire's own `categories` array and
## read the same way (`_staff_alert_state`) — Godot never recomputes `short`.
const SHORT_STAFFED_SNAPSHOT := {
	"categories": [{"id": "manufacturing", "name": "Manufacturing", "short": 2}],
}

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("HUD_PLATES_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


## The window is 64x64 under `--script`, so every geometry check sets a real
## size first and then re-runs the three positioning passes a live frame
## runs, in the same order.
func _resize(to: Vector2i) -> void:
	root.size = to
	main._position_hotbar()
	main._update_contract_panel()
	main._position_stock_panel()


func _press_key(keycode: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _ingredient_ids() -> Array:
	var ids: Array = []
	for entry in main._ingredients_snapshot().get("entries", []):
		ids.append(String(entry.get("id", "")))
	return ids


func _descends_from(node: Node, ancestor: Node) -> bool:
	var walk := node
	while walk != null:
		if walk == ancestor:
			return true
		walk = walk.get_parent()
	return false


func _run_checks() -> void:
	# --- The plate exists, on its own, beside the clipboard ----------------
	if main._stock_panel == null:
		_fail("there is no stock plate")
		return
	if main._hotbar_root == null or main._contract_panel == null:
		_fail("the hotbar or the clipboard is missing — the fixture is wrong")
		return
	if _descends_from(main._stock_panel, main._hotbar_root):
		_fail("the stock plate is still built inside the hotbar's column")
		return
	if main._stock_panel.get_parent() != main._contract_panel.get_parent():
		_fail("the stock plate is not a sibling of the contracts clipboard (parent %s vs %s)" % [
			main._stock_panel.get_parent(), main._contract_panel.get_parent()
		])
		return
	if not main._stock_panel.visible:
		_fail("the stock plate is not visible — it is a permanent readout")
		return
	if main._stock_hint == null or main._stock_hint.text != "M: market":
		_fail("the plate does not keep the 'M: market' hint (got '%s')" % [
			"<none>" if main._stock_hint == null else main._stock_hint.text
		])
		return

	# --- One row per ingredient, with a display name, and nothing else -----
	var ids := _ingredient_ids()
	if ids.size() < 3:
		_fail("the shipped content prices %d raw ingredients — too few to prove a column" % ids.size())
		return
	if main._larder_entries.size() != ids.size():
		_fail("the plate has %d count labels for %d priced ingredients (%s)" % [
			main._larder_entries.size(), ids.size(), str(ids)
		])
		return
	if main._stock_rows.get_child_count() != ids.size():
		_fail("the list holds %d rows for %d ingredients" % [
			main._stock_rows.get_child_count(), ids.size()
		])
		return
	for item_id in ids:
		if not main._larder_entries.has(item_id):
			_fail("'%s' is priced but has no row on the stock plate" % item_id)
			return
		var row: Control = main._stock_row_nodes.get(item_id)
		if row == null:
			_fail("'%s' has a count but no row node" % item_id)
			return
		var icon := row.get_node_or_null("StockIcon_%s" % item_id) as TextureRect
		if icon == null or icon.texture == null:
			_fail("'%s' has no voxel icon on its row" % item_id)
			return
		var name_label := row.get_node_or_null("StockName_%s" % item_id) as Label
		if name_label == null:
			_fail("'%s' has no name label on its row" % item_id)
			return
		var display: String = main._item_display_name(item_id)
		if name_label.text != display:
			_fail("'%s' reads '%s' on the plate, expected the display name '%s'" % [
				item_id, name_label.text, display
			])
			return
		if name_label.text == item_id and display == item_id:
			_fail("'%s' has no display name to show — the wire id would be printed" % item_id)
			return
		var count: Label = main._larder_entries[item_id]
		if not _descends_from(count, main._stock_panel):
			_fail("'%s' count label is not on the stock plate" % item_id)
			return

	# --- One readout of one fact in one place ------------------------------
	# Nothing under the hotbar's column may still be showing stock: the strip
	# was removed rather than kept, so the two can never disagree.
	for node in main._hotbar_root.find_children("*", "Label", true, false):
		for item_id in ids:
			if node.name == "Stock_%s" % item_id or node.name.begins_with("Larder"):
				_fail("a second stock readout ('%s') is still on the hotbar" % node.name)
				return

	# --- Every count label owns its OWN LabelSettings ----------------------
	# A LabelSettings is a Resource: one instance assigned to three Labels is
	# one object with three references, and `_update_larder` recolours it per
	# label. The strip did exactly that, so whichever ingredient was written
	# last decided the colour of all of them.
	var seen: Array = []
	for item_id in ids:
		var settings: LabelSettings = main._larder_entries[item_id].label_settings
		if settings == null:
			_fail("'%s' count label has no LabelSettings" % item_id)
			return
		if seen.has(settings.get_instance_id()):
			_fail("'%s' shares its LabelSettings with another row — recolouring one recolours both" % item_id)
			return
		seen.append(settings.get_instance_id())

	# --- Counts follow bought stock; zero is a word, dimmed ----------------
	main._update_larder()
	for item_id in ids:
		var label: Label = main._larder_entries[item_id]
		if label.text != "none":
			_fail("'%s' reads '%s' with nothing bought, expected the word 'none'" % [item_id, label.text])
			return
		if label.label_settings.font_color != main.STOCK_NONE_COLOR:
			_fail("'%s' reads 'none' without the dim colour" % item_id)
			return
		var row: Control = main._stock_row_nodes[item_id]
		if not is_equal_approx(row.modulate.a, main.STOCK_NONE_ROW_ALPHA):
			_fail("'%s' row is not dimmed at zero (alpha %.2f)" % [item_id, row.modulate.a])
			return

	var bought: String = ids[0]
	var untouched: String = ids[ids.size() - 1]
	Layouts.stock(main.sim, bought, 42)
	main._refresh_owned_counts()
	main._update_larder()
	if main._larder_entries[bought].text != "42":
		_fail("buying 42 %s left the plate reading '%s'" % [bought, main._larder_entries[bought].text])
		return
	if main._larder_entries[bought].label_settings.font_color != main.STOCK_COUNT_COLOR:
		_fail("a stocked ingredient's count is not in the plate's own count colour")
		return
	if not is_equal_approx(float(main._stock_row_nodes[bought].modulate.a), 1.0):
		_fail("a stocked ingredient's row is still dimmed")
		return
	# The bug the per-row LabelSettings fixes: writing the LAST row's `none`
	# must not reach back and grey the first row's 42.
	if main._larder_entries[untouched].text != "none":
		_fail("'%s' was never bought but reads '%s'" % [untouched, main._larder_entries[untouched].text])
		return
	if main._larder_entries[bought].label_settings.font_color \
			== main._larder_entries[untouched].label_settings.font_color:
		_fail("the stocked and the empty rows share one colour — the LabelSettings are shared again")
		return

	# Spending it moves the number back down in the same refresh.
	Layouts.place(main.sim, "hopper", Vector2i(-6, 0), "east", bought)
	main._issue_world_command({"type": "fill_hopper", "tile": [-6, 0]})
	main._refresh_owned_counts()
	main._update_larder()
	var left := int(main._owned_ingredient_counts().get(bought, 0))
	if left >= 42:
		_fail("filling a hopper with %s spent nothing (%d left)" % [bought, left])
		return
	var expected := "%d" % left if left > 0 else "none"
	if main._larder_entries[bought].text != expected:
		_fail("after the fill the plate reads '%s' for %d owned" % [main._larder_entries[bought].text, left])
		return

	# --- It clears the clipboard and the hotbar, at both window sizes ------
	for window in [BIG, SMALL]:
		_resize(window)
		var stock: Rect2 = main._stock_rect()
		if stock.size.x <= 0.0 or stock.size.y <= 0.0:
			_fail("the stock plate has no rect at %s" % str(window))
			return
		if stock.position.x < 0.0 or stock.end.x > float(window.x) + 0.5:
			_fail("the stock plate runs off the %s window horizontally (%s)" % [str(window), str(stock)])
			return
		if stock.position.y < 0.0 or stock.end.y > float(window.y) + 0.5:
			_fail("the stock plate runs off the %s window vertically (%s)" % [str(window), str(stock)])
			return
		# Right-hand edge, the ticket's own words, not merely on screen.
		if stock.position.x < float(window.x) * 0.5:
			_fail("the stock plate is not on the right-hand half of the %s window (%s)" % [str(window), str(stock)])
			return
		var clipboard: Rect2 = main._contract_panel.get_global_rect()
		if main._contract_panel.visible and stock.intersects(clipboard):
			_fail("the stock plate lands on the contracts clipboard at %s (stock %s, clipboard %s)" % [
				str(window), str(stock), str(clipboard)
			])
			return
		var hotbar: Rect2 = main._hotbar_panel.get_global_rect()
		if stock.intersects(hotbar):
			_fail("the stock plate lands on the hotbar at %s (stock %s, hotbar %s)" % [
				str(window), str(stock), str(hotbar)
			])
			return
		# The list yields rather than the plate growing: at least one whole
		# row is always readable, however tight the column gets.
		if stock.size.y < main._stock_row_height():
			_fail("the stock plate is capped below one row's height at %s" % str(window))
			return

	_resize(BIG)

	# --- The legend folds, from the button and from F1 ---------------------
	if main._legend_panel == null or main._legend_toggle == null:
		_fail("the legend has no plate or no toggle button")
		return
	if main._legend_collapsed or not main._help_label.visible:
		_fail("the legend starts folded — it should start open")
		return
	if main._legend_toggle.text.find("F1") < 0:
		_fail("the open legend's button does not name the key: '%s'" % main._legend_toggle.text)
		return
	if main._legend_toggle.focus_mode != Control.FOCUS_NONE:
		_fail("the legend button takes keyboard focus — the number keys would stop reaching the game")
		return
	if main._legend_toggle.custom_minimum_size.x < 48.0 or main._legend_toggle.custom_minimum_size.y < 24.0:
		_fail("the legend button is smaller than a generous hit target (%s)" % str(main._legend_toggle.custom_minimum_size))
		return

	var open_rect: Rect2 = main._legend_rect()
	main._legend_toggle.pressed.emit()
	if not main._legend_collapsed or main._help_label.visible:
		_fail("the button did not fold the legend")
		return
	if not main._legend_panel.visible:
		_fail("folding hid the whole plate — a stub must be left behind (§7)")
		return
	if main._legend_toggle.text.find("F1") < 0:
		_fail("the folded stub does not say how to get the legend back: '%s'" % main._legend_toggle.text)
		return
	var heading := main._legend_panel.find_child("LegendHeading", true, false) as Label
	if heading == null or not heading.visible or heading.text == "":
		_fail("the folded stub is not labelled")
		return
	var stub_rect: Rect2 = main._legend_rect()
	if stub_rect.size.y >= open_rect.size.y:
		_fail("the folded legend is no shorter than the open one (%s vs %s)" % [str(stub_rect), str(open_rect)])
		return

	# F1 through the real handler brings it back, and folds it again.
	_press_key(KEY_F1)
	if main._legend_collapsed or not main._help_label.visible:
		_fail("F1 did not bring the legend back")
		return
	_press_key(KEY_F1)
	if not main._legend_collapsed:
		_fail("F1 did not fold the legend again")
		return

	# --- Both view modes ---------------------------------------------------
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	if main._help_label.text != main.FP_HELP_TEXT:
		_fail("first person did not swap the legend's text")
		return
	if not main._legend_collapsed or main._help_label.visible:
		_fail("switching to first person un-folded the legend")
		return
	_press_key(KEY_F1)
	if main._legend_collapsed or not main._help_label.visible:
		_fail("F1 does not work in first person")
		return
	_press_key(KEY_F1)
	main._set_view_mode(main.ViewMode.ISOMETRIC)
	if not main._legend_collapsed:
		_fail("going back to isometric un-folded the legend")
		return
	_press_key(KEY_F1)

	# --- Folding is not a modal -------------------------------------------
	main._set_shop_menu_open(true)
	_press_key(KEY_F1)
	if not main._legend_collapsed:
		_fail("F1 is swallowed while the build menu is open")
		return
	if not main._shop_open:
		_fail("folding the legend closed the build menu — it is not a modal")
		return
	_press_key(KEY_ESCAPE)
	if main._shop_open:
		_fail("ESC did not close the build menu")
		return
	if not main._legend_collapsed:
		_fail("ESC un-folded the legend — the fold is not something ESC undoes")
		return
	_press_key(KEY_X)
	if not main._legend_collapsed:
		_fail("X (close every modal) un-folded the legend")
		return
	main._exit_build_mode()

	# --- §2b point 33: the avoid list reads the CURRENT rect ---------------
	var probe := PanelContainer.new()
	var probe_label := Label.new()
	probe_label.text = "hover panel\ntwo lines"
	probe.add_child(probe_label)
	root.add_child(probe)
	var probe_size: Vector2 = probe.get_combined_minimum_size()

	# Folded: a panel asked for near where the open legend used to be may
	# take that space back, and must still clear the stub that is there.
	var anchor: Vector2 = open_rect.position + open_rect.size * 0.5
	var folded_pos: Vector2 = main._panel_position_near(probe, anchor)
	if Rect2(folded_pos, probe_size).intersects(main._legend_rect()):
		_fail("a panel near the cursor lands on the folded legend's stub")
		return
	if not Rect2(folded_pos, probe_size).intersects(open_rect):
		_fail("a folded legend still reserves the whole open plate's rect — the avoid list is measuring the wrong thing")
		return

	# Open again: the same anchor must now be pushed clear of the full plate.
	_press_key(KEY_F1)
	if main._legend_collapsed:
		_fail("F1 did not re-open the legend for the avoid-list check")
		return
	var open_pos: Vector2 = main._panel_position_near(probe, anchor)
	if Rect2(open_pos, probe_size).intersects(main._legend_rect()):
		_fail("a panel near the cursor lands on the open legend (%s over %s)" % [
			str(Rect2(open_pos, probe_size)), str(main._legend_rect())
		])
		return

	# And the new plate joins the list: a cursor over the stock column pushes
	# the panel off it, not onto it.
	var stock_rect: Rect2 = main._stock_rect()
	var stock_pos: Vector2 = main._panel_position_near(probe, stock_rect.position + stock_rect.size * 0.5)
	if Rect2(stock_pos, probe_size).intersects(stock_rect):
		_fail("a panel near the cursor lands on the stock plate (%s over %s)" % [
			str(Rect2(stock_pos, probe_size)), str(stock_rect)
		])
		return
	probe.queue_free()

	# --- The clipboard's docket stack is cut BETWEEN dockets (issue #8) ----
	# The clipboard is the panel that yields the right-hand column (see
	# `_position_stock_panel`), so its docket stack is the one that gets
	# capped — and the cut has to land on a boundary between two dockets. A
	# card cut through the middle of a line of text reads as a rendering
	# fault, not as "there is more, scroll".
	#
	# This needs more dockets than fit, so sign every offer on the board
	# through the real command, and it needs the SHORT-STAFFED strip, which
	# is what made the ragged cut visible: the strip lives inside the
	# clipboard's own body and takes two lines out of the stack's share.
	for offer in JSON.parse_string(main.sim.economy_snapshot_json()).get("board", []):
		if not bool(offer.get("active", false)):
			main.sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": String(offer["id"])}))
	for strip in [false, true]:
		# The strip is driven off the staff snapshot the clipboard already
		# polls, rather than by firing crew and running the line for a few
		# thousand ticks — this is a LAYOUT check, and what it needs is the
		# two extra lines in the body (test_staff_panel.gd owns the real
		# understaffed fixture).
		main._staff_snapshot = SHORT_STAFFED_SNAPSHOT if strip else {}
		for window in [BIG, SMALL]:
			_resize(window)
			if main._staff_alert_panel.visible != strip:
				_fail("the SHORT-STAFFED strip should be %s for this case at %s" % [
					"up" if strip else "down", str(window)
				])
				return
			var cut: float = main._contract_cards_scroll.custom_minimum_size.y
			var whole: Array = _docket_boundaries()
			if whole.size() < 2:
				_fail("this check needs several dockets to have a boundary to land on, found %d" % whole.size())
				return
			var lands_on_boundary := false
			for boundary in whole:
				if absf(cut - float(boundary)) <= 0.5:
					lands_on_boundary = true
			if not lands_on_boundary:
				_fail("at %s (strip %s) the docket stack is cut at %.0fpx, which is inside a docket — whole dockets end at %s" % [
					str(window), "up" if strip else "down", cut, str(whole)
				])
				return
			# And at least one whole docket, however tight the column is —
			# the stack's own floor, the stock plate's `_stock_row_height`.
			if cut < float(whole[0]) - 0.5:
				_fail("the docket stack is capped below one whole docket at %s (%.0f < %.0f)" % [
					str(window), cut, float(whole[0])
				])
				return
			# The plate under it obeys the same rule, and this is where it
			# matters: the clipboard's floor is a whole docket now, so this
			# is the tightest the stock list ever gets — and a count sliced
			# through its digits is the same fault in the other panel.
			var rows_cut: float = main._stock_scroll.custom_minimum_size.y
			var row_height: float = main._stock_row_height()
			var separation: float = float(main._stock_rows.get_theme_constant("separation"))
			var rows_shown: float = (rows_cut + separation) / (row_height + separation)
			if absf(rows_shown - roundf(rows_shown)) > 0.02:
				_fail("at %s (strip %s) the stock list is cut at %.0fpx — %.2f rows, not a whole number of them" % [
					str(window), "up" if strip else "down", rows_cut, rows_shown
				])
				return
	main._staff_snapshot = {}
	_resize(BIG)

	print("HUD_PLATES_OK: %d stock rows on a right-hand plate clear of the clipboard and hotbar at 1600x900 and 1280x720, each with its own colour and a 'none' zero state; the legend folds %s -> %s from its button and from F1 in both view modes, is not a modal, and every cursor panel reads its current rect" % [
		ids.size(), str(open_rect.size), str(stub_rect.size)
	])
	quit(0)


## The heights at which the docket stack has a boundary: the first docket,
## the first two, and so on up to all of them. Measured off the real cards,
## because a docket's height varies with its checklist, its status line and
## its strike pips — there is no one row height to divide by here.
func _docket_boundaries() -> Array:
	var separation: float = float(main._contract_cards_box.get_theme_constant("separation"))
	var out: Array = []
	var running: float = 0.0
	for child in main._contract_cards_box.get_children():
		var card := child as Control
		if card == null or not card.visible:
			continue
		if not out.is_empty():
			running += separation
		running += card.get_combined_minimum_size().y
		out.append(running)
	return out
