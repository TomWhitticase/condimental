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
#   * every modal the player can open (E, M, C, P, H) asks for no more width
#     than the window has, sits wholly inside it, and leaves no button drawn
#     outside it — at 1600x900 AND at 1280x720, with the world past every
#     progression gate so the market and the board are as full as they get
#     (issue #62: the staff panel's minimum was 1350px, so at 1280x720 it was
#     clipped on both edges and FIRE −1 could not be clicked at all; issues
#     #69 and #17 are the same rule on the market row and the docket stack),
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

## A staff snapshot for the CREW plate, shaped like the wire's `categories`.
##
## `short` is deliberately INCONSISTENT with `required - headcount` here
## (2, not 1). The plate must print the wire's own figure: `short` is the
## sim's hiring advice and Godot recomputing it would be exactly the
## GDScript-side gameplay arithmetic the establishment change removed
## (CLAUDE.md: gameplay logic lives in Rust). A plate that printed "HIRE 1"
## from this fixture would be doing its own sums.
##
## `maintenance` is the dead category: nothing in `machines.ron` asks for it
## and nobody is hired into it, so its row must take no room at all.
const CREW_SNAPSHOT := {
	"money": 40.0,
	"next_payday_tick": 600,
	"next_bill": 52,
	"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "headcount": 3, "required": 4, "short": 2},
		{"id": "packaging", "name": "Packaging", "headcount": 2, "required": 2, "short": 0},
		{"id": "maintenance", "name": "Maintenance", "headcount": 0, "required": 0, "short": 0},
	],
}

## Every combination the right-hand column has to survive: the strip down and
## up, at the shipped window size and at the tight one.
const DOCKET_CASES := [
	[false, BIG], [false, SMALL], [true, BIG], [true, SMALL],
]
## Every modal the player can open: the key that opens it, the setter behind
## that key, and the panel it puts on the screen. Each one is checked at BOTH
## window sizes, because the rule they are checked against is the new first
## line of UI_UX.md §5f — a control the player cannot reach outranks every
## other layout rule (issue #62: the staff panel's own minimum width was
## 1350px, so at 1280x720 it hung off both edges and FIRE −1 could not be
## clicked at all).
const MODAL_CASES := [
	["E", "_set_shop_menu_open", "_shop_menu"],
	["M", "_set_market_open", "_market_panel"],
	["C", "_set_contract_board_open", "_board_panel"],
	["P", "_set_production_open", "_production_panel"],
	["H", "_set_staff_open", "_staff_panel"],
]

## The tile the docket sweep's own hopper stands on (`_run_checks` places it),
## reused by the configuration sweep: the panel that opens on a click has to
## clear the right-hand column like any other (issue #88).
const CONFIG_TILE := Vector2i(-6, 0)

## And a FLUID TANK, placed for the configuration sweep alone. Issue #88 is a
## tank at 1600x900, not a hopper, and the two panels are not the same panel:
## the tank's carries a fluid meter and a contents picker of fluids. A sweep
## of the hopper alone is a sweep that PASSES with `_position_config_panel`
## reverted to the version that shipped the bug, at the very size the issue
## was filed about.
const CONFIG_TANK_TILE := Vector2i(-3, 0)

## WHERE IN THE FRAME the building stands when its panel opens, as a
## fraction of the window. The panel is anchored to the BUILDING, so this is
## not decoration — it is the input the positioning is a function of, and a
## sweep that only ever frames the building in the middle only ever tests the
## middle.
##
## `CONFIG_LOW` is where issue #88 photographed its tank, and it is the half
## of the issue a centred sweep cannot see: a panel opening BELOW the
## right-hand plates is clamped back up above the hotbar, and the clamp can
## lift it into plates that were nowhere near it when the flip was decided.
const CONFIG_CENTRED := Vector2(0.5, 0.5)
const CONFIG_LOW := Vector2(0.52, 0.58)

## Every configuration case: which building, on which tile, at which window
## size, standing where in the frame.
const CONFIG_CASES := [
	["the hopper", CONFIG_TILE, BIG, CONFIG_CENTRED],
	["the hopper", CONFIG_TILE, SMALL, CONFIG_CENTRED],
	["the fluid tank", CONFIG_TANK_TILE, BIG, CONFIG_CENTRED],
	["the fluid tank", CONFIG_TANK_TILE, SMALL, CONFIG_CENTRED],
	["the fluid tank", CONFIG_TANK_TILE, BIG, CONFIG_LOW],
	["the fluid tank", CONFIG_TANK_TILE, SMALL, CONFIG_LOW],
]

## Frames a case is given before it is read — one to apply it, the rest for
## Godot's deferred minimum-size invalidation (`_sweep_step`). It has always
## settled by the second; the third is slack.
const SETTLE_FRAMES := 3

var main
var _ran := false
## What `_run_checks` proved, held for the success line the sweep prints.
var _summary := ""
## The docket sweep's cursor: which case, and how many frames it has had to
## settle in (see `_sweep_step`).
var _case := 0
var _frame := 0
## The modal sweep's own cursor, a case per (modal, window) pair, and whether
## the docket sweep before it has been put away (`_finish_sweep`).
var _modal := 0
var _swept := false
## The configuration sweep's cursor, an index into `CONFIG_CASES`.
var _cfg := 0


func _fail(msg: String) -> void:
	printerr("HUD_PLATES_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if not _ran:
		_ran = true
		_run_checks()
		return false
	if _case < DOCKET_CASES.size():
		return _sweep_step()
	if not _swept:
		_swept = true
		_finish_sweep()
		return false
	return _modal_step()


## The window is 64x64 under `--script`, so every geometry check sets a real
## size first and then re-runs the three positioning passes a live frame
## runs, in the same order.
func _resize(to: Vector2i) -> void:
	root.size = to
	main._position_hotbar()
	# CREW before STOCK before the clipboard — the column's order of
	# yielding, and the order `_update_larder` runs them in every frame.
	main._position_crew_panel()
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
	Layouts.place(main.sim, "hopper", CONFIG_TILE, "east", bought)
	# The configuration sweep's second building — issue #88's own kind.
	Layouts.place(main.sim, "fluid_tank", CONFIG_TANK_TILE, "east")
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

	# --- The CREW plate: the staffing readout that needs no key pressed ----
	# It is the bottom tenant of the right-hand column, so the order of
	# yielding it completes is: the clipboard scrolls its dockets, the stock
	# list scrolls its rows, and this plate — bounded at four categories —
	# takes its natural height. All three must therefore be disjoint, at both
	# window sizes, on the right-hand edge.
	for window in [BIG, SMALL]:
		_resize(window)
		if main._crew_panel == null or not main._crew_panel.visible:
			_fail("there is no CREW plate at %s" % str(window))
			return
		var crew := Rect2(main._crew_panel.global_position, main._crew_panel.size)
		if crew.size.x <= 0.0 or crew.size.y <= 0.0:
			_fail("the CREW plate has no rect at %s" % str(window))
			return
		if crew.position.x < 0.0 or crew.end.x > float(window.x) + 0.5:
			_fail("the CREW plate runs off the %s window horizontally (%s)" % [str(window), str(crew)])
			return
		if crew.position.y < 0.0 or crew.end.y > float(window.y) + 0.5:
			_fail("the CREW plate runs off the %s window vertically (%s)" % [str(window), str(crew)])
			return
		if crew.position.x < float(window.x) * 0.5:
			_fail("the CREW plate is not on the right-hand half of the %s window (%s)" % [str(window), str(crew)])
			return
		for other in [
			["the contracts clipboard", main._contract_panel.get_global_rect(), main._contract_panel.visible],
			["the stock plate", main._stock_rect(), true],
			["the hotbar", main._hotbar_panel.get_global_rect(), true],
		]:
			if bool(other[2]) and crew.intersects(other[1]):
				_fail("the CREW plate lands on %s at %s (crew %s, other %s)" % [
					String(other[0]), str(window), str(crew), str(other[1])
				])
				return
		# Below the stock list, never above it: the bounded plate holds the
		# bottom so the unbounded one above it can do the scrolling.
		if crew.position.y < main._stock_rect().end.y:
			_fail("the CREW plate sits above the stock plate at %s (crew %s, stock %s)" % [
				str(window), str(crew), str(main._stock_rect())
			])
			return

	# Tight window: the plate sheds its DETAIL line, never a category and
	# never behind a scrollbar. Every category the player might have to act
	# on stays on screen at 1280x720 (`_crew_fit_detail`) — an at-a-glance
	# readout you have to scroll is not one.
	_resize(SMALL)
	for id in main._crew_row_nodes:
		var nodes = main._crew_row_nodes[id]
		if nodes["wrap"].visible and nodes["assigned_label"].visible:
			_fail("the CREW plate kept the '%s' assignment line at 1280x720" % id)
			return

	_resize(BIG)
	var named := 0
	for id in main._crew_row_nodes:
		var nodes = main._crew_row_nodes[id]
		if not nodes["wrap"].visible:
			continue
		if not nodes["assigned_label"].visible:
			_fail("the CREW plate dropped the '%s' assignment line at 1600x900" % id)
			return
		if String(nodes["assigned_label"].text) != "":
			named += 1
	if named == 0:
		_fail("no CREW row names what its category is assigned to")
		return

	# Every figure on the plate is the wire's, read through the one pure
	# function that builds it — no window size involved, so it is asserted
	# once rather than per window.
	var crew_state: Dictionary = main._crew_plate_state(CREW_SNAPSHOT, 0)
	var crew_rows: Array = crew_state["rows"]
	if crew_rows.size() != 3:
		_fail("the CREW plate built %d rows for 3 categories" % crew_rows.size())
		return
	if String(crew_rows[0]["count_text"]) != "3 / 4":
		_fail("the CREW plate reads '%s' for 3 hired of 4 required" % crew_rows[0]["count_text"])
		return
	if String(crew_rows[0]["chip_text"]) != "▲ HIRE 2":
		_fail("the CREW plate recomputed the shortfall instead of reading it: '%s'" % crew_rows[0]["chip_text"])
		return
	if String(crew_rows[1]["chip_text"]) != "":
		_fail("a fully crewed category still shows a HIRE chip ('%s')" % crew_rows[1]["chip_text"])
		return
	if not bool(crew_rows[0]["visible"]) or not bool(crew_rows[1]["visible"]):
		_fail("a category with an establishment is hidden from the CREW plate")
		return
	if bool(crew_rows[2]["visible"]):
		_fail("the dead `maintenance` category takes room on the CREW plate")
		return
	# Seconds, never ticks (UI_UX.md): 600 ticks away is not "600".
	var payday := String(crew_state["payday_text"])
	if not payday.begins_with("payday in") or payday.contains("600"):
		_fail("the CREW plate's payday line reads in ticks, not seconds ('%s')" % payday)
		return
	if not payday.contains("52"):
		_fail("the CREW plate's payday line does not name the bill ('%s')" % payday)
		return
	# 40 in the bank against a bill of 52: the warning comes BEFORE the money
	# is gone, and carries a glyph and words, never colour alone (§7).
	if not bool(crew_state["afford_warn"]) or String(crew_state["afford_text"]) == "":
		_fail("40 in hand against a bill of 52 raised no affordability warning")
		return
	# And the live plate hides its own dead row, not just the state dictionary.
	var maintenance = main._crew_row_nodes.get("maintenance", null)
	if maintenance is Dictionary and maintenance["wrap"].visible:
		_fail("the live CREW plate still draws the empty `maintenance` row")
		return

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

	# And the new plates join the list: a cursor over the right-hand column
	# pushes the panel off it, not onto it. The column is ONE list
	# (godot_project/CLAUDE.md), so every tenant of it is checked the same
	# way — CREW was added to the column and left out of `_panel_position_near`,
	# and a machine hovered beside it covered the whole plate.
	for plate in [
		["the stock plate", main._stock_rect()],
		["the CREW plate", main._crew_rect()],
	]:
		var plate_rect: Rect2 = plate[1]
		if plate_rect.size.y <= 0.0:
			_fail("%s has no rect to avoid — the fixture is wrong" % plate[0])
			return
		var pos: Vector2 = main._panel_position_near(probe, plate_rect.position + plate_rect.size * 0.5)
		if Rect2(pos, probe_size).intersects(plate_rect):
			_fail("a panel near the cursor lands on %s (%s over %s)" % [
				plate[0], str(Rect2(pos, probe_size)), str(plate_rect)
			])
			return
	probe.queue_free()

	# --- First person parks a world panel, it does not follow (issue #105) -
	# In first person there is no cursor: the query point is the crosshair at
	# the centre of the screen, and the subject is the thing BEHIND it. The
	# flip above is right for a mouse and wrong here — at 1280x720 all four
	# quadrants fail, the fallback clamps back through the middle, and the
	# panel lands squarely on the crosshair with the dock the player walked
	# over to look at entirely behind it.
	#
	# The sizes are the test's own, not a headless measurement: a headless
	# font metric cannot prove a width (godot_project/CLAUDE.md), so what is
	# pinned here is the RULE, given a size, and the render is the acceptance.
	# 550x283 is the machine hover as it is actually drawn at 1280x720, the
	# size the bug was found at.
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	if main._legend_collapsed:
		_press_key(KEY_F1)
	for window in [BIG, SMALL]:
		_resize(window)
		var crosshair := Vector2(window) * 0.5
		var lower_bound: float = float(window.y) - main._hotbar_reserved_height()
		for fp_size in [Vector2(550, 283), Vector2(480, 150), Vector2(300, 90)]:
			var placed := Rect2(main._first_person_panel_position(fp_size), fp_size)
			if placed.has_point(crosshair):
				_fail("at %s the first-person panel stands on the crosshair (%s over %s)" % [
					str(window), str(placed), str(crosshair)
				])
				return
			if placed.position.x < 0.0 or placed.end.x > float(window.x):
				_fail("at %s the first-person panel hangs off the side (%s)" % [str(window), str(placed)])
				return
			if placed.position.y < 0.0 or placed.end.y > lower_bound + 0.5:
				_fail("at %s the first-person panel runs into the hotbar's strip (%s, bound %d)" % [
					str(window), str(placed), int(lower_bound)
				])
				return
			# The right-hand column is never the answer: it is a plate at
			# every resolution, and the corner is on the other side.
			for plate in [
				["the clipboard", main._contract_panel.get_global_rect()],
				["the stock plate", main._stock_rect()],
				["the CREW plate", main._crew_rect()],
			]:
				if placed.intersects(plate[1]):
					_fail("at %s the first-person panel lands on %s (%s over %s)" % [
						str(window), plate[0], str(placed), str(plate[1])
					])
					return
	# And the two rules are chosen by the view mode, not by the caller: one
	# door (`_world_panel_position`) that both the hover tooltip and the
	# placement prompt go through, so neither can be fixed and the other left.
	var fp_probe := PanelContainer.new()
	var fp_label := Label.new()
	fp_label.text = "Dock  ·  facing east\nwaiting: boxed_ketchup x48"
	fp_probe.add_child(fp_label)
	root.add_child(fp_probe)
	var fp_anchor := Vector2(SMALL) * 0.5
	var fp_drawn: Vector2 = main._panel_draw_size(fp_probe)
	if main._world_panel_position(fp_probe, fp_anchor) != main._first_person_panel_position(fp_drawn):
		_fail("in first person a world panel is still being placed near the crosshair")
		return
	main._set_view_mode(main.ViewMode.ISOMETRIC)
	if main._world_panel_position(fp_probe, fp_anchor) != main._panel_position_near(fp_probe, fp_anchor):
		_fail("isometric lost the cursor-following rule")
		return
	fp_probe.queue_free()
	_resize(BIG)

	# --- The clipboard's docket stack is cut BETWEEN dockets (issue #8) ----
	# The clipboard is the panel that yields the right-hand column (see
	# `_position_stock_panel`), so its docket stack is the one that gets
	# capped — and the cut has to land on a boundary between two dockets. A
	# card cut through the middle of a line of text reads as a rendering
	# fault, not as "there is more, scroll".
	#
	# This needs more dockets than fit, so sign every offer on the board
	# through the real command. The sweep itself runs a case per frame from
	# `_sweep_step`, for the reason written there.
	for offer in JSON.parse_string(main.sim.economy_snapshot_json()).get("board", []):
		if not bool(offer.get("active", false)):
			main.sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": String(offer["id"])}))
	_summary = "%d stock rows on a right-hand plate clear of the clipboard and hotbar at 1600x900 and 1280x720, each with its own colour and a 'none' zero state; the legend folds %s -> %s from its button and from F1 in both view modes, is not a modal, and every cursor panel reads its current rect; a world panel is parked in the lower-left corner in first person rather than following the crosshair, clear of the crosshair, the hotbar's strip and the whole right-hand column at both sizes; the CREW plate holds the bottom of that column clear of all three at both sizes, prints the wire's own shortfall, hides the dead maintenance row and warns before payday" % [
		ids.size(), str(open_rect.size), str(stub_rect.size)
	]


## One case of the docket sweep per call, and SEVERAL FRAMES per case.
##
## Raising the SHORT-STAFFED strip changes the clipboard's minimum size, and
## Godot invalidates a cached minimum size in a DEFERRED call — which a
## `--script` SceneTree run flushes between `_process` calls and never inside
## one. Asserting in the frame that raised the strip reads the height the
## clipboard had before the strip existed (it read 1584px, the whole unfitted
## stack); by the next frame it has settled, and it never moves again. So the
## case is applied, given `SETTLE_FRAMES` to settle, and only then read.
##
## The strip is driven off the staff snapshot the clipboard already polls,
## rather than by firing crew and running the line for a few thousand ticks —
## this is a LAYOUT check, and what it needs is the two extra lines in the
## body (test_staff_panel.gd owns the real understaffed fixture). It is
## re-applied every frame because `main._process` re-polls the real one.
func _sweep_step() -> bool:
	var strip: bool = bool(DOCKET_CASES[_case][0])
	var window: Vector2i = DOCKET_CASES[_case][1]
	main._staff_snapshot = SHORT_STAFFED_SNAPSHOT if strip else {}
	_resize(window)
	_frame += 1
	if _frame < SETTLE_FRAMES:
		return false
	if not _check_docket_case(strip, window):
		return true
	_case += 1
	_frame = 0
	return false


## False when the case failed (`_fail` has already asked to quit).
func _check_docket_case(strip: bool, window: Vector2i) -> bool:
	var where := "%s (strip %s)" % [str(window), "up" if strip else "down"]
	# The alarm is on the CREW plate now (2026-09-16), not on the clipboard,
	# so the clipboard's height no longer moves with it — which is what made
	# the right-hand column fit at 1280x720 with three plates in it. Both
	# cases still run: the snapshot drives the plate's heading and border,
	# and the column must survive either.
	if (main._crew_hint.text.find("SHORT-STAFFED") >= 0) != strip:
		_fail("the CREW alarm should be %s for this case at %s (heading '%s')" % [
			"up" if strip else "down", str(window), main._crew_hint.text
		])
		return false
	# Issue #13's trade is off against the SHORT-STAFFED strip: the board hint
	# no longer competes with an alarm that is not on its plate. It does still
	# yield its line to the docket stack, but only for the one thing worth a
	# line of advice — a stack cut ABOVE the first docket's first line, which
	# is a heading sliced through its own type (issue #17). So when the hint is
	# down, the line it gave up has to be a line the stack got back.
	if not main._contract_hint_label.visible:
		var first_line: float = main._first_line_boundary(main._contract_cards_box)
		var bought: float = main._contract_cards_scroll.custom_minimum_size.y
		if bought < first_line - 0.5:
			_fail("at %s the board hint is down and the stack still shows no whole docket line (%.0f < %.0f)" % [
				where, bought, first_line
			])
			return false

	# Neither plate is drawn ON the other. This is the fault that outranks
	# every cut below: the earlier section checks it too, but only with the
	# strip DOWN, which is how a taller clipboard came to overlap the stock
	# plate at 1280x720 with every test green.
	var stock: Rect2 = main._stock_rect()
	var clipboard: Rect2 = main._contract_panel.get_global_rect()
	if main._contract_panel.visible and stock.intersects(clipboard):
		_fail("at %s the stock plate lands on the clipboard (stock %s, clipboard %s)" % [
			where, str(stock), str(clipboard)
		])
		return false
	if stock.intersects(main._hotbar_panel.get_global_rect()):
		_fail("at %s the stock plate lands on the hotbar (stock %s)" % [where, str(stock)])
		return false
	if clipboard.end.y > float(window.y):
		_fail("at %s the clipboard runs off the bottom of the window (%s)" % [where, str(clipboard)])
		return false

	# The cut lands on a docket boundary — unless the no-overlap clamp above
	# is what set it, which is the 1280x720 case with a full board: there the
	# stack is cut wherever it falls, because the alternative is the plate
	# below having nowhere to stand (`_stock_ceiling_y`).
	var cut: float = main._contract_cards_scroll.custom_minimum_size.y
	var whole: Array = _docket_boundaries()
	if whole.size() < 2:
		_fail("this check needs several dockets to have a boundary to land on, found %d" % whole.size())
		return false
	var clamped: bool = cut < float(whole[0]) - 0.5
	if not clamped:
		var lands_on_boundary := false
		for boundary in whole:
			if absf(cut - float(boundary)) <= 0.5:
				lands_on_boundary = true
		if not lands_on_boundary:
			_fail("at %s the docket stack is cut at %.0fpx, which is inside a docket — whole dockets end at %s" % [
				where, cut, str(whole)
			])
			return false
	else:
		# The clamp has overruled the docket boundary, which is allowed — a
		# plate with nowhere to stand is the worse fault, and that ordering
		# is the one UI_UX.md §5 fixes. But "cut ragged" was never the whole
		# of it: the cut still has to land on an edge the reader can see was
		# meant, so inside the docket it falls in it goes up to the bottom of
		# one of that docket's own LINES (issue #17).
		var lines: Array = _docket_line_boundaries()
		var lands_on_line := false
		for boundary in lines:
			if absf(cut - float(boundary)) <= 0.5:
				lands_on_line = true
		# There is no escape hatch under the first line any more: the
		# clipboard buys that line back by dropping the board hint
		# (`_update_contract_panel`), so a cut between zero and the first line
		# means that trade did not happen.
		if not lands_on_line:
			_fail("at %s the clamped stack is cut at %.0fpx, inside a line — lines end at %s" % [
				where, cut, str(lines)
			])
			return false

	# The plate under it obeys the same rule, and this is where it matters:
	# the clipboard's floor is a whole docket now, so this is the tightest
	# the stock list ever gets — and a count sliced through its digits is the
	# same fault in the other panel.
	var rows_cut: float = main._stock_scroll.custom_minimum_size.y
	var row_height: float = main._stock_row_height()
	var separation: float = float(main._stock_rows.get_theme_constant("separation"))
	var rows_shown: float = (rows_cut + separation) / (row_height + separation)
	if absf(rows_shown - roundf(rows_shown)) > 0.02:
		_fail("at %s the stock list is cut at %.0fpx — %.2f rows, not a whole number of them" % [
			where, rows_cut, rows_shown
		])
		return false
	if rows_cut < row_height - 0.5:
		_fail("at %s the stock list is capped below one whole row (%.0f < %.0f)" % [
			where, rows_cut, row_height
		])
		return false
	return true


## Every height the stack can be cut at without slicing through a line: a
## docket boundary, or — inside a docket — the bottom of one of its own lines,
## measured through the card's top padding. `_docket_boundaries`' finer
## sibling, for the cut the no-overlap clamp forces inside a card, and
## measured off the real cards for the same reason. Starts at 0.0: the top
## edge of the first docket is a boundary too.
func _docket_line_boundaries() -> Array:
	var separation: float = float(main._contract_cards_box.get_theme_constant("separation"))
	var out: Array = [0.0]
	var running: float = 0.0
	var cards: int = 0
	for child in main._contract_cards_box.get_children():
		var card := child as Control
		if card == null or not card.visible:
			continue
		if cards > 0:
			running += separation
		var card_top: float = running
		var body := card.get_child(0) as Control
		if body != null:
			var style: StyleBox = (card as PanelContainer).get_theme_stylebox("panel")
			var inner: float = card_top + (style.get_margin(SIDE_TOP) if style != null else 0.0)
			var body_separation: float = float(body.get_theme_constant("separation"))
			var lines: int = 0
			for line_child in body.get_children():
				var line := line_child as Control
				if line == null or not line.visible:
					continue
				if lines > 0:
					inner += body_separation
				inner += line.get_combined_minimum_size().y
				out.append(inner)
				lines += 1
		running = card_top + card.get_combined_minimum_size().y
		out.append(running)
		cards += 1
	return out


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


## One case of the modal sweep per call, and SEVERAL FRAMES per case, for the
## reason `_sweep_step` writes down: a modal's minimum size is invalidated in a
## deferred call, so the frame that opened it still reports the size it had
## while it was closed.
##
## The first call also puts the world past every progression gate, the way
## `test_screenshot.gd --unlocked` does. Without it the market has three crates
## and fits anything; issue #69 is about the six the unlocked game sells.
func _modal_step() -> bool:
	if _modal == 0 and _frame == 0:
		_unlock_everything()
	if _modal >= MODAL_CASES.size() * 2:
		return _config_step()
	var case: Array = MODAL_CASES[_modal / 2]
	var window: Vector2i = BIG if _modal % 2 == 0 else SMALL
	_open_only(String(case[1]))
	_resize(window)
	_reposition_modals()
	_frame += 1
	if _frame < SETTLE_FRAMES:
		return false
	if not _check_modal_case(String(case[0]), String(case[2]), window):
		return true
	_modal += 1
	_frame = 0
	return false


## The last sweep: the panel a CLICK on a building opens (issue #88). It is
## not a modal — it is positioned beside the building it belongs to, and it
## keeps clear of the right-hand column by flipping to the building's other
## side (`_position_config_panel`). The CREW plate arrived on that column
## after the flip was written and was never added to it, so a panel that
## cleared the clipboard and STOCK was drawn straight over the wage bill.
##
## The legend is checked too, but as "never covered while drawn" rather than
## "never overlapped" (issue #107). At neither shipped size does a position
## exist that clears both the legend and the right-hand column — the panel is
## 571px tall in a 698px band at 1600x900, and 426px wide where the gap
## between the legend and the clipboard is smaller than that — so a plain
## "must not overlap" would be a rule nothing can satisfy. The panel takes
## the corner when it must and the legend steps aside for as long as it does
## (`_set_legend_yielded`), which is the ranking `_position_config_panel` has
## always stated, now carried out rather than merely written down.
func _config_step() -> bool:
	if _cfg >= CONFIG_CASES.size():
		_open_only("")
		_resize(BIG)
		print("HUD_PLATES_OK: %s; every modal (%s) fits inside the window with every button reachable, at 1600x900 and at 1280x720; and a hopper's and a FLUID TANK's configuration panel keeps clear of all three right-hand plates at both sizes — and never drawn on top of the CONTROLS legend, which steps aside at a size where nothing can clear both (issue #107) — with the building centred and standing low in the frame where issue #88 photographed it" % [
			_summary, ", ".join(_modal_keys())
		])
		quit(0)
		return true
	var what: String = CONFIG_CASES[_cfg][0]
	var tile: Vector2i = CONFIG_CASES[_cfg][1]
	var window: Vector2i = CONFIG_CASES[_cfg][2]
	var across: Vector2 = CONFIG_CASES[_cfg][3]
	_open_only("")
	_resize(window)
	_open_config(tile, window, across)
	main._position_config_panel()
	_frame += 1
	if _frame < SETTLE_FRAMES:
		return false
	if not _check_config_case(window, tile, what, across):
		return true
	_close_config(tile)
	_cfg += 1
	_frame = 0
	return false


## Opens it with the camera LOOKING AT the building, which is what a player
## who is about to configure something has just done. It matters: the panel is
## anchored to the building's projected position, so a building off the left of
## the frame puts the panel on the left edge and proves nothing — centred, the
## panel opens over the right-hand column and has to flip to clear it, which is
## the case issue #88 was about.
func _open_config(tile: Vector2i, window: Vector2i, across: Vector2) -> void:
	var world: Vector3 = main._tile_origin(tile.x, tile.y, 0.0)
	main._camera_focus = world
	main._update_camera_transform()
	# Then pan until the building is drawn where this case wants it, through
	# the real drag-pan the player uses. Screen pixels to world is linear, so
	# the correction converges immediately; it is ASSERTED afterwards rather
	# than trusted (`_check_config_case`), because a case that quietly framed
	# the building somewhere else would quietly prove something else.
	var want := Vector2(float(window.x), float(window.y)) * across
	for _i in range(32):
		var delta: Vector2 = want - main._camera.unproject_position(world)
		if delta.length() <= 0.25:
			break
		main._apply_drag_pan(delta)
	if not main._config_open:
		main._config_click_at_tile(tile)


func _close_config(tile: Vector2i) -> void:
	if main._config_open:
		main._config_click_at_tile(tile)


## False when the case failed (`_fail` has already asked to quit).
func _check_config_case(window: Vector2i, tile: Vector2i, what: String, across: Vector2) -> bool:
	var where := "%s's configuration panel at %s, standing %d%% across and %d%% down the frame" % [
		what, str(window), int(round(across.x * 100.0)), int(round(across.y * 100.0))
	]
	if not main._config_open or main._config_panel == null or not main._config_panel.visible:
		_fail("%s never opened at %s" % [where, str(tile)])
		return false
	# The framing this case asked for is the case: assert it rather than
	# assume the pan landed (see `_open_config`).
	var drawn: Vector2 = main._camera.unproject_position(main._tile_origin(tile.x, tile.y, 0.0))
	var wanted := Vector2(float(window.x), float(window.y)) * across
	if drawn.distance_to(wanted) > 1.0:
		_fail("%s: the building is drawn at %s, not the %s this case asked for" % [
			where, str(drawn), str(wanted)
		])
		return false
	var rect: Rect2 = main._config_panel.get_global_rect()
	if rect.position.x < -0.5 or rect.end.x > float(window.x) + 0.5:
		_fail("%s is off the side of the window (%s)" % [where, str(rect)])
		return false
	var plates := [
		["the contracts clipboard", main._contract_panel_rect()],
		["the STOCK plate", main._stock_rect()],
		["the CREW plate", main._crew_rect()],
	]
	# The legend joins the list only while it is DRAWN. The panel is allowed
	# to take its corner when no position clears both it and the column —
	# what it is not allowed to do is take the corner and leave the legend
	# underneath (issue #107). `_legend_rect()` still reports a rect while
	# the plate has yielded, so this has to ask the plate and not the rect.
	if main._legend_panel != null and main._legend_panel.visible:
		plates.append(["the CONTROLS legend", main._legend_rect()])
	for named in plates:
		var plate: Rect2 = named[1]
		if plate.size.x <= 0.0 or plate.size.y <= 0.0:
			continue
		var overlap: Rect2 = rect.intersection(plate)
		if overlap.size.x > 0.5 and overlap.size.y > 0.5:
			_fail("%s covers %s by %.0fx%.0fpx" % [
				where, String(named[0]), overlap.size.x, overlap.size.y
			])
			return false
	return true


## Between the two sweeps: the docket sweep's fixture is put away, so the
## modal sweep starts from the clean staff snapshot and the window it sets
## itself.
func _finish_sweep() -> void:
	main._staff_snapshot = {}
	_resize(BIG)
	_summary += "; the docket stack is cut between dockets and the board hint buys back the first docket line when the stack cannot show one, and neither plate lands on the other, at both sizes with the strip up and down"
	_frame = 0


func _modal_keys() -> Array:
	var keys: Array = []
	for case in MODAL_CASES:
		keys.append(String(case[0]))
	return keys


## Past every progression gate, so the market holds every crate the shop sells
## and the board every offer. Lifted from `test_screenshot.gd --unlocked`:
## `deliver` goes through save/load, so the rendered world is rebuilt from the
## reloaded one.
func _unlock_everything() -> void:
	var econ: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	var threshold := 0
	for unlock in econ.get("unlocks", []):
		threshold = maxi(threshold, int(unlock.get("after_deliveries", 0)))
	if threshold == 0:
		_fail("data/unlocks.ron gates nothing — the market case would prove nothing")
		return
	Layouts.deliver(main.sim, threshold)
	main._rebuild_world()
	main._refresh_owned_counts()


## Opens exactly one modal, by the name of its setter, and closes the rest.
## One at a time is the game's own rule (`_set_staff_open` and its siblings
## close each other), and it is also what makes a width failure readable.
func _open_only(setter: String) -> void:
	for case in MODAL_CASES:
		main.call(String(case[1]), String(case[1]) == setter)


## The positioning passes a live frame runs for the modals, after `_resize`
## has run the ones for the fixed plates.
func _reposition_modals() -> void:
	main._position_shop_menu()
	main._position_market_panel()
	main._position_contract_board()
	main._position_production_panel()
	main._position_staff_panel()


## False when the case failed (`_fail` has already asked to quit).
func _check_modal_case(key: String, panel_name: String, window: Vector2i) -> bool:
	var where := "%s at %s" % [key, str(window)]
	var panel: Control = main.get(panel_name) as Control
	if panel == null or not panel.visible:
		_fail("%s did not open the panel it owns (%s)" % [where, panel_name])
		return false
	# The width the panel ASKS for, not the width it got: a Control clamped to
	# a window narrower than its minimum still reports the clamped size, while
	# its children lay out at the minimum and hang off the edge. The minimum is
	# the number the player feels.
	var wanted: Vector2 = panel.get_combined_minimum_size()
	if wanted.x > float(window.x) + 0.5:
		_fail("%s asks for %.0fpx of width in a %dpx window — it is clipped on both edges" % [
			where, wanted.x, window.x
		])
		return false
	var rect: Rect2 = panel.get_global_rect()
	if rect.position.x < -0.5 or rect.end.x > float(window.x) + 0.5:
		_fail("%s is off the side of the window (%s)" % [where, str(rect)])
		return false
	if rect.position.y < -0.5 or rect.end.y > float(window.y) + 0.5:
		_fail("%s is off the top or bottom of the window (%s)" % [where, str(rect)])
		return false
	# And the rule the panel's rect does not cover: a button is only pressed
	# where it is DRAWN, so every one of them has to be inside the window even
	# when the panel that holds them is not.
	var stray: Button = _unreachable_button(panel, window)
	if stray != null:
		_fail("%s puts %s ('%s') outside the window at %s — the player cannot click it" % [
			where, stray.name, stray.text, str(stray.get_global_rect())
		])
		return false
	return true


## The first visible button of this panel that is not wholly inside the
## window and cannot be brought into it, or null. Walks the whole subtree: the
## one that started issue #62 was FIRE −1, four containers down a scrolling
## row.
##
## A button below the fold of a ScrollContainer is NOT unreachable — it is
## what a scroll bar is for — so an enclosing scroll that still scrolls on the
## axis the button is off on excuses it. A scroll with that axis DISABLED does
## not: that is the staff panel's case, where the rows could not be pulled
## sideways and FIRE −1 was simply gone.
func _unreachable_button(node: Node, window: Vector2i, scrolls_x := false, scrolls_y := false) -> Button:
	for child in node.get_children():
		var control := child as Control
		if control == null or not control.visible:
			continue
		var button := control as Button
		if button != null:
			var rect: Rect2 = button.get_global_rect()
			var off_x: bool = rect.position.x < -0.5 or rect.end.x > float(window.x) + 0.5
			var off_y: bool = rect.position.y < -0.5 or rect.end.y > float(window.y) + 0.5
			if rect.size.x > 0.0 and ((off_x and not scrolls_x) or (off_y and not scrolls_y)):
				return button
		var scroll := control as ScrollContainer
		var down_x: bool = scrolls_x
		var down_y: bool = scrolls_y
		if scroll != null:
			down_x = down_x or scroll.horizontal_scroll_mode != ScrollContainer.SCROLL_MODE_DISABLED
			down_y = down_y or scroll.vertical_scroll_mode != ScrollContainer.SCROLL_MODE_DISABLED
		var found: Button = _unreachable_button(control, window, down_x, down_y)
		if found != null:
			return found
	return null

