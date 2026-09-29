# Headless structural proof of the contracts board and the docket risk
# readout (the contract-board epic, 2026-09-10; GAMEPLAY.md §6/§9/§11,
# UI_UX.md §5/§7).
#
# Run with:
#   godot --headless --path godot_project --script test_contract_board.gd
#
# Like every other headless script here this does NOT claim the board *looks*
# right — that needs eyes on `scripts/screenshot-scene.sh --board`. What it
# proves is the structural half, through the same entry points a real
# session reaches:
#
#   * a new game's clipboard carries NO docket — it says nothing is running
#     and names the key — and the board menu carries one row per offer, in
#     file order, every row's SIGN button priced and enabled;
#   * C opens and closes the board through `_unhandled_input`, E swaps it for
#     the build menu, ESC closes it, and opening it changes nothing in the
#     simulation;
#   * clicking SIGN (driven through `_sign_contract_offer`) debits the price,
#     adds a docket to the clipboard, flips the row to SIGNED, and a second
#     click is refused — one of each contract, never two;
#   * `_contract_risk_for` — the pure half of the docket's status — sorts a
#     contract into ready / ok / at risk / last chance from the snapshot's own
#     fields, and the docket shows each state as a glyph AND words AND strike
#     pips AND a border, never colour alone;
#   * a contract left unserved for its strikes is LOST for real: it leaves the
#     clipboard, the announcement line says so, the board offers it again
#     with a LOST note, and signing it again costs its price;
#   * with every contract lost the clipboard says so in a sentence that names
#     the key to press;
#   * the board is TWO columns — the running contracts' dockets on the left,
#     the offers on the right — and a signature moves the contract into the
#     left column on the same call, while an empty left column shows a paper
#     card saying nothing is running;
#   * at 1280x720 the board shows at least two WHOLE offers, on a new game and
#     with nothing running, and the empty card keeps the left column to its
#     pinned width;
#   * every offer is listed, and the ones a gate holds back are padlocked —
#     locked paper, a 🔒 on the badge, the button naming the gate over its
#     own meter and count — and a signature on one is refused by the sim;
#   * the SHOW and sauce filters show exactly the offers they name, every
#     chip counts what it would show given the other filter (checked against
#     the wire under every combination), only the chosen chip is ticked, a
#     sauce chip clears on a second click, a filter that matches nothing
#     shows a card saying so, and every filtered view at 1280x720 still shows
#     two whole offers when it has two.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("CONTRACT_BOARD_FAIL: %s" % msg)
	quit(1)


func _key(main, keycode: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _econ(main) -> Dictionary:
	return JSON.parse_string(main.sim.economy_snapshot_json())


func _active_ids(main) -> Array:
	var out: Array = []
	for contract in _econ(main).get("contracts", []):
		out.append(String(contract["id"]))
	return out


func _card_ids(main) -> Array:
	var out: Array = []
	for card in main._contract_cards:
		out.append(String(card["id"]))
	return out


## The docket ids actually hanging in the board's LEFT column, read off the
## scene tree rather than the record array, so a card built into the wrong
## container cannot pass.
func _left_column_ids(main) -> Array:
	var out: Array = []
	var column: Node = main._board_panel.find_child("ActiveColumn", true, false)
	for card in main._board_active_cards:
		var panel: Node = card["panel"]
		if column != null and column.is_ancestor_of(panel):
			out.append(String(card["id"]))
	return out


## The smallest window the game supports (UI_UX.md §5f), where the board's
## two columns are tightest.
const SMALL := Vector2i(1280, 720)


## How many WHOLE offers the board's scroll shows, with the board positioned
## at SMALL exactly as a live frame positions it. Summed off the rows' own
## heights against the height `_position_contract_board` gave the scroll, so
## a card half in view does not count — half an offer is half a decision.
func _whole_offers_at_small(main) -> int:
	root.size = SMALL
	main._position_hotbar()
	main._update_contract_board()
	var room: float = main._board_scroll.custom_minimum_size.y
	var sep: float = float(main._board_box.get_theme_constant("separation"))
	var used := 0.0
	var whole := 0
	for row in main._board_rows:
		if not row["panel"].visible:
			continue
		used += row["panel"].get_combined_minimum_size().y + (sep if whole > 0 else 0.0)
		if used > room + 0.5:
			break
		whole += 1
	return whole


## The offer ids the filters leave showing, in board order.
func _shown_ids(main) -> Array:
	var out: Array = []
	for row in main._board_rows:
		if row["panel"].visible:
			out.append(String(row["id"]))
	return out


## The number a chip ends with ("✔ ALL  17" -> 17).
func _chip_count(chip: Button) -> int:
	var parts: PackedStringArray = chip.text.split("  ")
	return int(parts[parts.size() - 1])


## "" if every chip reads what the WIRE says it should under the filters as
## they stand — each status chip counting the offers the sauce filter lets
## through, each sauce chip the offers the status filter does — and only the
## chosen chip in each row pressed and ticked; otherwise what is wrong.
func _chip_error(main, board: Array) -> String:
	var status_want := {"all": 0, "open": 0, "signed": 0, "locked": 0}
	var sauce_want := {"": 0}
	for offer in board:
		var status := "signed" if bool(offer["active"]) else ("locked" if bool(offer["locked"]) else "open")
		var ids: Array = []
		for pair in offer["mix"]:
			if not ids.has(String(pair[0])):
				ids.append(String(pair[0]))
		if main._board_sauce_filter == "" or ids.has(main._board_sauce_filter):
			status_want["all"] += 1
			status_want[status] += 1
		if main._board_status_filter == "all" or main._board_status_filter == status:
			sauce_want[""] += 1
			for item_id in ids:
				sauce_want[item_id] = int(sauce_want.get(item_id, 0)) + 1
	for key in status_want:
		var chip: Button = main._board_status_chips[key]
		if _chip_count(chip) != status_want[key]:
			return "status chip '%s' reads '%s', the wire says %d" % [key, chip.text, status_want[key]]
		var chosen: bool = key == main._board_status_filter
		if chip.button_pressed != chosen or chip.text.begins_with("✔") != chosen:
			return "status chip '%s' reads '%s' pressed=%s, but chosen=%s" % [key, chip.text, chip.button_pressed, chosen]
	for key in main._board_sauce_chips:
		var chip: Button = main._board_sauce_chips[key]
		if _chip_count(chip) != int(sauce_want.get(key, 0)):
			return "sauce chip '%s' reads '%s', the wire says %d" % [key, chip.text, int(sauce_want.get(key, 0))]
		var chosen: bool = key == main._board_sauce_filter
		if chip.button_pressed != chosen or chip.text.begins_with("✔") != chosen:
			return "sauce chip '%s' reads '%s' pressed=%s, but chosen=%s" % [key, chip.text, chip.button_pressed, chosen]
	return ""


## "" if at 1280x720 every filtered view shows two whole offers (or all it
## has, if fewer) and leaves the board the width ALL gives it; otherwise
## what is wrong. Leaves both filters cleared.
func _filter_views_error(main) -> String:
	_whole_offers_at_small(main)
	# And under every filter: a padlocked card used to carry one more line
	# than an open one, and the LOCKED view at 1280x720 showed one (PR #208's
	# review). A filter showing fewer than two offers owes as many as it has.
	# The offer column also keeps ONE width through every filter, the
	# no-match card included: sized by the visible cards, the board jumped
	# sideways on each click. (The left column is measured apart: a running
	# docket's own status line changes width as its window drains, which no
	# filter touches.)
	var board_width: float = main._board_scroll.get_parent().get_combined_minimum_size().x
	var filter_views: Array = [["locked", ""], ["open", ""], ["signed", ""]]
	for item_id in main._board_sauce_ids:
		filter_views.append(["all", String(item_id)])
	for view in filter_views:
		main._set_board_status_filter(view[0])
		main._set_board_sauce_filter(view[1])
		var owed := mini(2, _shown_ids(main).size())
		var whole := _whole_offers_at_small(main)
		if whole < owed:
			return "at 1280x720 the %s + '%s' view shows %d whole offer(s) of %d — it owes %d" % [
				view[0], view[1], whole, _shown_ids(main).size(), owed]
		var view_width: float = main._board_scroll.get_parent().get_combined_minimum_size().x
		if absf(view_width - board_width) > 0.5:
			return "the %s + '%s' view makes the offer column %.0fpx wide, ALL makes it %.0fpx — a filter must not move it" % [
				view[0], view[1], view_width, board_width]
		if view[1] != "":
			main._set_board_sauce_filter(view[1])
	main._set_board_status_filter("all")
	return ""


func _row_for(main, id: String) -> Dictionary:
	for row in main._board_rows:
		if String(row["id"]) == id:
			return row
	return {}


var _main
var _ran := false


func _initialize() -> void:
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)


## The checks run on the first frame, not in `_initialize`: until then the
## scene has no viewport, and `_position_contract_board` — which the 1280x720
## checks below need — lays nothing out without one.
func _process(_delta: float) -> bool:
	if not _ran:
		_ran = true
		_run_checks(_main)
	return false


func _run_checks(main) -> void:

	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	# --- The board exists, hidden, one row per offer in file order --------
	var board_panel: PanelContainer = main._board_panel
	if board_panel == null or board_panel.name != "ContractBoard" or board_panel.get_parent() != hud:
		_fail("the contracts board must be a HUD child named 'ContractBoard'")
		return
	if board_panel.visible or main._board_open:
		_fail("the board must start closed")
		return
	var econ := _econ(main)
	var board: Array = econ.get("board", [])
	if board.size() < 2:
		_fail("economy_snapshot_json reported %d offer(s) — the shipped board must have several" % board.size())
		return
	if main._board_rows.size() != board.size():
		_fail("expected %d board row(s), found %d" % [board.size(), main._board_rows.size()])
		return
	for i in range(board.size()):
		if String(main._board_rows[i]["id"]) != String(board[i]["id"]):
			_fail("board row %d is '%s' but the snapshot's offer %d is '%s' — rows must follow file order" % [
				i, main._board_rows[i]["id"], i, board[i]["id"]])
			return
		if String(main._board_rows[i]["name_label"].text).replace("\n", " ") != String(board[i]["name"]):
			_fail("row '%s' shows the wrong name" % board[i]["id"])
			return
		var terms: String = String(main._board_rows[i]["terms_label"].text)
		for fragment in ["every %s" % main._format_remaining(int(board[i]["cadence_ticks"])), "%d strike" % int(board[i]["strikes"])]:
			if not terms.contains(fragment):
				_fail("row '%s' terms '%s' do not state '%s'" % [board[i]["id"], terms, fragment])
				return
		if terms.contains("tick"):
			_fail("row '%s' terms '%s' speak in ticks — a player-facing clock is seconds" % [board[i]["id"], terms])
			return
		if not String(main._board_rows[i]["payout_label"].text).contains("+%d" % int(board[i]["payout"])):
			_fail("row '%s' does not state its payout, got '%s'" % [board[i]["id"], main._board_rows[i]["payout_label"].text])
			return

	# --- A new game: nothing running, every open offer for sale -----------
	# The board lists EVERY offer; the ones a gate holds back are padlocked
	# (2026-09-29), so "for sale" is the unlocked ones.
	var offers: Array = []
	var locked_offers: Array = []
	for offer in board:
		if bool(offer["starter"]):
			_fail("the shipped board must sign nothing for a new player, '%s' is a starter" % offer["id"])
			return
		if bool(offer["locked"]):
			locked_offers.append(String(offer["id"]))
		else:
			offers.append(String(offer["id"]))
	if offers.is_empty():
		_fail("the shipped board must offer at least one contract to buy")
		return
	if locked_offers.is_empty():
		_fail("a new game's board must show some padlocked offers, or the padlock checks prove nothing")
		return
	if not _card_ids(main).is_empty() or not _active_ids(main).is_empty():
		_fail("a new game's clipboard should carry no docket, got %s" % str(_card_ids(main)))
		return
	if not main._contract_empty_label.visible or not main._contract_empty_label.text.contains("press C"):
		_fail("a new game's empty clipboard should say so and name the key, got visible=%s '%s'" % [
			str(main._contract_empty_label.visible), main._contract_empty_label.text])
		return
	if not main._contract_hint_label.text.contains("C"):
		_fail("the clipboard should tell the player which key opens the board")
		return
	main._update_contract_board()
	for id in offers:
		var row := _row_for(main, id)
		var offer_price := 0
		for offer in board:
			if String(offer["id"]) == id:
				offer_price = int(offer["price"])
		if row["sign_button"].disabled:
			_fail("offer '%s' should be signable with the starting balance" % id)
			return
		if String(row["sign_button"].text) != "SIGN · %d" % offer_price:
			_fail("offer '%s' button should name the price, got '%s'" % [id, row["sign_button"].text])
			return
		if row["lock_row"].visible or row["badge_lock"].visible:
			_fail("open offer '%s' must not carry a padlock" % id)
			return

	# --- A locked offer: padlocked four ways, and refused if signed --------
	for id in locked_offers:
		var row := _row_for(main, id)
		var offer: Dictionary = {}
		for o in board:
			if String(o["id"]) == id:
				offer = o
		if not row["sign_button"].disabled or not String(row["sign_button"].text).begins_with("🔒"):
			_fail("locked offer '%s' must have a disabled 🔒 button, got '%s'" % [id, row["sign_button"].text])
			return
		if not row["lock_row"].visible or not row["badge_lock"].visible:
			_fail("locked offer '%s' must show its padlock line and the badge padlock" % id)
			return
		if String(row["sign_button"].text) != "🔒 %s" % String(offer["unlock_name"]):
			_fail("locked offer '%s' button must name its gate '%s', got '%s'" % [id, offer["unlock_name"], row["sign_button"].text])
			return
		if row["status_label"].visible:
			_fail("locked offer '%s' shows its gate count in the status's place, not both" % id)
			return
		# Faded ONCE: a modulate multiplies down the tree, and the terms sit
		# inside the faded mix row unless a split moved them out.
		var alpha := 1.0
		var node: Node = row["terms_label"]
		while node != row["panel"]:
			alpha *= (node as CanvasItem).modulate.a
			node = node.get_parent()
		if absf(alpha - 0.55) > 0.01:
			_fail("locked offer '%s' terms are drawn at %.2f alpha, not faded once to 0.55" % [id, alpha])
			return
		if not String(row["lock_count"].text).contains("%d of %d" % [int(offer["unlock_sold"]), int(offer["unlock_boxes"])]):
			_fail("locked offer '%s' must say how far along its gate is, got '%s'" % [id, row["lock_count"].text])
			return
		if not row["lock_meter"].has_meta("fraction"):
			_fail("locked offer '%s' has no gate meter driven by _set_meter" % id)
			return
		if row["panel"].get_theme_stylebox("panel") != main._contract_card_style_locked:
			_fail("locked offer '%s' must be on the locked paper" % id)
			return
	var econ_locked: String = main.sim.economy_snapshot_json()
	main._do_sign_contract(String(locked_offers[0]))
	if _active_ids(main).has(locked_offers[0]) or main.sim.economy_snapshot_json() != econ_locked:
		_fail("signing locked '%s' must be refused by the sim" % locked_offers[0])
		return

	# --- The filters: status and sauce, counted, never silently empty ------
	main._update_contract_board()
	var all_count: int = main._board_rows.size()
	var chip_error := _chip_error(main, board)
	if chip_error != "":
		_fail("ALL/ALL: " + chip_error)
		return
	main._set_board_status_filter("locked")
	chip_error = _chip_error(main, board)
	if chip_error != "":
		_fail("LOCKED: " + chip_error)
		return
	if _shown_ids(main) != locked_offers:
		_fail("LOCKED should show exactly the padlocked offers %s, shows %s" % [str(locked_offers), str(_shown_ids(main))])
		return
	var locked_chip: Button = main._board_status_chips["locked"]
	if not locked_chip.button_pressed or not locked_chip.text.ends_with(str(locked_offers.size())):
		_fail("the LOCKED chip should be pressed and count %d, reads '%s'" % [locked_offers.size(), locked_chip.text])
		return
	main._set_board_status_filter("open")
	if _shown_ids(main) != offers:
		_fail("OPEN should show exactly the unlocked offers %s, shows %s" % [str(offers), str(_shown_ids(main))])
		return
	main._set_board_status_filter("signed")
	if not _shown_ids(main).is_empty() or not main._board_no_match.visible:
		_fail("SIGNED on a new game matches nothing, and the board must say so")
		return
	main._set_board_status_filter("all")
	if _shown_ids(main).size() != all_count or main._board_no_match.visible:
		_fail("ALL should show every offer again")
		return
	# The sauce chips come from the wire's mixes — one per product, none typed
	# here — and each shows exactly the offers whose mix names it.
	if main._board_sauce_ids.size() < 2:
		_fail("the shipped board asks for several products, so there should be several sauce chips, got %s" % str(main._board_sauce_ids))
		return
	for item_id in main._board_sauce_ids:
		var expected: Array = []
		for offer in board:
			for pair in offer["mix"]:
				if String(pair[0]) == item_id and not expected.has(String(offer["id"])):
					expected.append(String(offer["id"]))
		main._set_board_sauce_filter(item_id)
		if _shown_ids(main) != expected:
			_fail("the '%s' chip should show %s, shows %s" % [item_id, str(expected), str(_shown_ids(main))])
			return
		# Every chip's count under this sauce, and under it with each status:
		# a count is what the chip WOULD show given the other filter.
		for status_key in ["all", "open", "signed", "locked"]:
			main._set_board_status_filter(status_key)
			chip_error = _chip_error(main, board)
			if chip_error != "":
				_fail("%s + '%s': %s" % [status_key, item_id, chip_error])
				return
		main._set_board_status_filter("all")
	var last_sauce: String = main._board_sauce_ids[-1]
	main._set_board_sauce_filter(last_sauce)
	if main._board_sauce_filter != "" or _shown_ids(main).size() != all_count:
		_fail("clicking the chosen sauce chip again should clear the sauce filter")
		return
	# The two combine: OPEN + a product only gated orders ask for is empty.
	var gated_only := ""
	for item_id in main._board_sauce_ids:
		var any_open := false
		for offer in board:
			for pair in offer["mix"]:
				if String(pair[0]) == item_id and not bool(offer["locked"]):
					any_open = true
		if not any_open:
			gated_only = item_id
	if gated_only == "":
		_fail("a new game should have a product only padlocked orders ask for")
		return
	main._set_board_status_filter("open")
	main._set_board_sauce_filter(gated_only)
	chip_error = _chip_error(main, board)
	if chip_error != "":
		_fail("OPEN + '%s': %s" % [gated_only, chip_error])
		return
	if _chip_count(main._board_sauce_chips[gated_only]) != 0:
		_fail("under OPEN the '%s' chip must count 0, reads '%s'" % [gated_only, main._board_sauce_chips[gated_only].text])
		return
	if not _shown_ids(main).is_empty() or not main._board_no_match.visible:
		_fail("OPEN + '%s' on a new game should match nothing, shows %s" % [gated_only, str(_shown_ids(main))])
		return
	main._set_board_sauce_filter(gated_only)
	main._set_board_status_filter("all")

	# --- C opens and closes it, changing nothing in the simulation --------
	var econ_before: String = main.sim.economy_snapshot_json()
	_key(main, KEY_C)
	if not main._board_open or not board_panel.visible:
		_fail("C should open the contracts board")
		return
	if main.sim.economy_snapshot_json() != econ_before:
		_fail("opening the board changed the economy")
		return
	# --- Two columns: running on the left, for sale on the right ---------
	var active_col: Node = board_panel.find_child("ActiveColumn", true, false)
	var offer_col: Node = board_panel.find_child("OfferColumn", true, false)
	if active_col == null or offer_col == null or active_col.get_parent() != offer_col.get_parent():
		_fail("the board must be two side-by-side columns, 'ActiveColumn' and 'OfferColumn'")
		return
	if not (active_col.get_parent() is HBoxContainer) or active_col.get_index() > offer_col.get_index():
		_fail("the running contracts must be the LEFT column, the offers the right")
		return
	for row in main._board_rows:
		if not offer_col.is_ancestor_of(row["panel"]):
			_fail("offer '%s' is not in the right-hand column" % row["id"])
			return
	if _left_column_ids(main) != _active_ids(main):
		_fail("the left column should hold exactly the active contracts %s, holds %s" % [
			str(_active_ids(main)), str(_left_column_ids(main))])
		return
	if main._board_active_empty.visible == not _active_ids(main).is_empty():
		_fail("the left column's empty card should show only when nothing is running")
		return
	# At 1280x720 a new game's board must let two offers be COMPARED: beside
	# the running dockets the offer card used to break to five lines, and the
	# window showed one offer at a time (PR #195's review).
	var fresh_whole := _whole_offers_at_small(main)
	if fresh_whole < 2:
		_fail("at 1280x720 a new game's board shows %d whole offer(s) — it must show at least 2 to compare" % fresh_whole)
		return
	var views_error := _filter_views_error(main)
	if views_error != "":
		_fail("new game: " + views_error)
		return
	_key(main, KEY_C)
	if main._board_open or board_panel.visible:
		_fail("a second C should close the board")
		return
	_key(main, KEY_C)
	_key(main, KEY_E)
	if main._board_open or not main._shop_open:
		_fail("E should swap the board for the build menu (one modal at a time)")
		return
	_key(main, KEY_C)
	if main._shop_open or not main._board_open:
		_fail("C should swap the build menu for the board")
		return
	_key(main, KEY_ESCAPE)
	if main._board_open or main._pause_open:
		_fail("ESC should close the board (and not reach the pause menu while it is open)")
		return
	for legend in [main.ISO_HELP_TEXT, main.FP_HELP_TEXT]:
		if not legend.contains("C: contracts"):
			_fail("the control legend does not name the contracts key: %s" % legend)
			return

	# --- SIGN buys an offer, once -----------------------------------------
	var first_offer: String = offers[0]
	var first_price := 0
	for offer in board:
		if String(offer["id"]) == first_offer:
			first_price = int(offer["price"])
	var money_before: float = float(_econ(main)["money"])
	var first_index: int = main._board_row_ids.find(first_offer)
	if _left_column_ids(main).has(first_offer):
		_fail("'%s' is in the left column before it was signed" % first_offer)
		return
	main._sign_contract_offer(first_index)
	if not _left_column_ids(main).has(first_offer) or _left_column_ids(main) != _active_ids(main):
		_fail("signing '%s' should put it in the board's left column at once; left %s, active %s" % [
			first_offer, str(_left_column_ids(main)), str(_active_ids(main))])
		return
	for card in main._board_active_cards:
		if String(card["id"]) == first_offer and String(card["status_label"].text) == "":
			_fail("the left column's docket for '%s' was never driven" % first_offer)
			return
	var money_after: float = float(_econ(main)["money"])
	if not is_equal_approx(money_before - money_after, float(first_price)):
		_fail("signing '%s' should cost %d, money went %.0f -> %.0f" % [first_offer, first_price, money_before, money_after])
		return
	if not _active_ids(main).has(first_offer):
		_fail("'%s' should be active after signing" % first_offer)
		return
	if not _card_ids(main).has(first_offer):
		_fail("the clipboard should show a docket for '%s' on the same frame it was signed" % first_offer)
		return
	if _card_ids(main) != _active_ids(main):
		_fail("dockets %s must be exactly the active contracts %s, in order" % [str(_card_ids(main)), str(_active_ids(main))])
		return
	var signed_row := _row_for(main, first_offer)
	if not signed_row["sign_button"].disabled or String(signed_row["sign_button"].text) != "SIGNED":
		_fail("a signed offer's row must flip to SIGNED")
		return
	main._sign_contract_offer(first_index)
	if not is_equal_approx(float(_econ(main)["money"]), money_after):
		_fail("signing an already-signed offer must be refused — money moved")
		return
	if _active_ids(main).count(first_offer) != 1:
		_fail("one of each contract, never two")
		return
	if main._contract_notice.text.find("SIGNED") < 0 or main._contract_notice.modulate.a < 0.99:
		_fail("signing should announce itself on the notice line, got '%s'" % main._contract_notice.text)
		return
	# With a docket running the left column is at its docket width, not the
	# empty card's, and the offers get what is left: the filtered views owe
	# the same two whole offers there.
	views_error = _filter_views_error(main)
	if views_error != "":
		_fail("with '%s' running: %s" % [first_offer, views_error])
		return

	# --- The pure risk function -------------------------------------------
	var base := {
		"cadence_ticks": 1000, "window_start_tick": 1000, "next_delivery_tick": 2000,
		"mix": [["boxed_ketchup", 2]], "progress": [["boxed_ketchup", 1]],
		"satisfied": false, "consecutive_misses": 0, "strikes": 3,
	}
	var ok: Dictionary = main._contract_risk_for(base, 1200)
	if ok["level"] != "ok" or ok["short"] != 1 or not is_equal_approx(ok["elapsed"], 0.2):
		_fail("early and one short should be 'ok' with elapsed 0.2, got %s" % str(ok))
		return
	var risk: Dictionary = main._contract_risk_for(base, 1700)
	if risk["level"] != "risk":
		_fail("one short with 70%% of the window gone should be 'risk', got %s" % str(risk))
		return
	var last := base.duplicate()
	last["consecutive_misses"] = 2
	if main._contract_risk_for(last, 1100)["level"] != "last_chance":
		_fail("one miss from the strike limit should be 'last_chance' however early it is")
		return
	var one_strike := base.duplicate()
	one_strike["strikes"] = 1
	if main._contract_risk_for(one_strike, 1100)["level"] != "last_chance":
		_fail("a one-strike contract is always one miss from lost")
		return
	var ready := base.duplicate()
	ready["progress"] = [["boxed_ketchup", 2]]
	ready["satisfied"] = true
	ready["consecutive_misses"] = 2
	var r: Dictionary = main._contract_risk_for(ready, 1900)
	if r["level"] != "ready" or r["short"] != 0:
		_fail("a satisfied mix is 'ready' whatever the strikes, got %s" % str(r))
		return
	if main._contract_risk_for(base, 5000)["elapsed"] != 1.0 or main._contract_risk_for(base, 0)["elapsed"] != 0.0:
		_fail("elapsed must clamp to [0, 1]")
		return

	# --- The docket shows the state four ways ------------------------------
	main._update_contract_panel()
	for card in main._contract_cards:
		if String(card["status_label"].text) == "":
			_fail("docket '%s' has no status line" % card["id"])
			return
		if not card["countdown_meter"].has_meta("fraction"):
			_fail("docket '%s' has no countdown bar driven by _set_meter" % card["id"])
			return
		var strikes := 0
		for contract in _econ(main)["contracts"]:
			if String(contract["id"]) == String(card["id"]):
				strikes = int(contract["strikes"])
		if card["strike_pips"].size() != strikes:
			_fail("docket '%s' should carry %d strike pip(s), has %d" % [card["id"], strikes, card["strike_pips"].size()])
			return
		if String(card["state"]) != "ok":
			_fail("a fresh, unfilled docket early in its window should read 'ok', '%s' reads '%s'" % [card["id"], card["state"]])
			return
	# Drive one docket through the states by hand — the card updater is pure
	# given a contract dictionary, exactly like the risk function.
	var card0: Dictionary = main._contract_cards[0]
	var late := base.duplicate()
	late["id"] = card0["id"]
	late["name"] = "x"
	late["payout"] = 50
	main._update_contract_card(card0, late, 1800)
	if String(card0["state"]) != "risk" or not String(card0["status_label"].text).begins_with("⚠ AT RISK"):
		_fail("70%% through and short should read '⚠ AT RISK', got '%s'" % card0["status_label"].text)
		return
	if not is_equal_approx(float(card0["countdown_meter"].get_meta("fraction")), 0.2):
		_fail("the countdown bar should have drained to 0.2, reads %s" % str(card0["countdown_meter"].get_meta("fraction")))
		return
	if card0["panel"].get_theme_stylebox("panel") != main._contract_card_style_risk:
		_fail("an at-risk docket should carry the at-risk border")
		return
	late["consecutive_misses"] = 2
	main._update_contract_card(card0, late, 1800)
	if String(card0["state"]) != "last_chance" or not String(card0["status_label"].text).begins_with("⚠ LAST CHANCE"):
		_fail("one miss from lost should read '⚠ LAST CHANCE', got '%s'" % card0["status_label"].text)
		return
	if not bool(card0["strike_pips"][0].get_meta("struck")) or not bool(card0["strike_pips"][1].get_meta("struck")) or bool(card0["strike_pips"][2].get_meta("struck")):
		_fail("two of three strike pips should be struck")
		return
	if not String(card0["strike_label"].text).contains("one more miss"):
		_fail("the strike line should say the next miss loses it, got '%s'" % card0["strike_label"].text)
		return
	if card0["panel"].get_theme_stylebox("panel") != main._contract_card_style_last_chance:
		_fail("a last-chance docket should carry the last-chance border")
		return
	late["progress"] = [["boxed_ketchup", 2]]
	late["satisfied"] = true
	main._update_contract_card(card0, late, 1800)
	if String(card0["state"]) != "ready" or not String(card0["status_label"].text).begins_with("✔ READY"):
		_fail("a filled docket should read '✔ READY', got '%s'" % card0["status_label"].text)
		return

	# --- A contract is really lost, announced, and re-signable ------------
	# The quickest offer to lose: smallest cadence x strikes. Nothing is ever
	# delivered in this scene, so every window is a miss.
	var quickest := ""
	var quickest_ticks := 0
	var quickest_price := 0
	for offer in board:
		if bool(offer["locked"]):
			continue
		var ticks := int(offer["cadence_ticks"]) * int(offer["strikes"])
		if quickest == "" or ticks < quickest_ticks:
			quickest = String(offer["id"])
			quickest_ticks = ticks
			quickest_price = int(offer["price"])
	if not _active_ids(main).has(quickest):
		main._sign_contract_offer(main._board_row_ids.find(quickest))
	if not _active_ids(main).has(quickest):
		_fail("could not sign '%s' to lose it" % quickest)
		return
	main._update_contract_panel()
	var signed_at: int = main.sim.current_tick()
	while main.sim.current_tick() < signed_at + quickest_ticks - 1:
		main.sim.tick()
	main._update_contract_panel()
	if not _active_ids(main).has(quickest):
		_fail("'%s' should survive until its last window closes" % quickest)
		return
	var doomed: Dictionary = {}
	for card in main._contract_cards:
		if String(card["id"]) == quickest:
			doomed = card
	if String(doomed["state"]) != "last_chance":
		_fail("one tick before its last window closes, unfilled, '%s' should read last_chance, reads '%s'" % [quickest, doomed["state"]])
		return
	main.sim.tick()
	main._update_contract_panel()
	if _active_ids(main).has(quickest) or _card_ids(main).has(quickest):
		_fail("'%s' should be lost once its strikes are used up" % quickest)
		return
	if main._contract_notice.text.find("CONTRACT LOST") < 0 or main._contract_notice.text.find("C") < 0:
		_fail("losing a contract should be announced, naming the key to get it back; got '%s'" % main._contract_notice.text)
		return
	if main._contract_notice.modulate.a < 0.99:
		_fail("the loss notice should be fully visible when it fires")
		return
	main._update_contract_board()
	var lost_row := _row_for(main, quickest)
	if lost_row["sign_button"].disabled or not String(lost_row["sign_button"].text).begins_with("SIGN"):
		_fail("a lost contract must be signable again, row reads '%s'" % lost_row["sign_button"].text)
		return
	if not String(lost_row["status_label"].text).contains("LOST"):
		_fail("a lost contract's row should say it was lost, got '%s'" % lost_row["status_label"].text)
		return
	var before_resign: float = float(_econ(main)["money"])
	main._sign_contract_offer(main._board_row_ids.find(quickest))
	if not is_equal_approx(before_resign - float(_econ(main)["money"]), float(quickest_price)):
		_fail("re-signing should cost the listed price again")
		return
	if not _card_ids(main).has(quickest):
		_fail("a re-signed contract should be back on the clipboard")
		return

	# --- Nothing left: the clipboard says so --------------------------------
	var guard := 0
	while not _active_ids(main).is_empty() and guard < 40000:
		main.sim.tick()
		guard += 1
	main._update_contract_panel()
	if not _active_ids(main).is_empty():
		_fail("could not run every contract out of strikes within %d ticks" % guard)
		return
	if not main._contract_cards.is_empty():
		_fail("with nothing active there should be no dockets")
		return
	if not main._contract_empty_label.visible or not main._contract_empty_label.text.contains("press C"):
		_fail("an empty clipboard should say there is nothing running and name the key, got visible=%s '%s'" % [
			str(main._contract_empty_label.visible), main._contract_empty_label.text])
		return
	main._update_contract_board()
	if not _left_column_ids(main).is_empty() or not main._board_active_empty.visible or main._board_active_scroll.visible:
		_fail("with nothing running the board's left column should show its empty card and no dockets")
		return
	# The empty card is held to the column's pinned width: its sentence on one
	# line widened the column ~90px, all of it taken from the offers, in the
	# state a new game opens the board to. And the offers still show two whole.
	var empty_whole := _whole_offers_at_small(main)
	var active_col_width: float = main._board_panel.find_child("ActiveColumn", true, false).get_combined_minimum_size().x
	if active_col_width > main.BOARD_ACTIVE_COLUMN_WIDTH + 0.5:
		_fail("the empty left column is %.0fpx wide, past its pinned %.0fpx — the empty card's sentence must wrap inside the pin" % [
			active_col_width, main.BOARD_ACTIVE_COLUMN_WIDTH])
		return
	if empty_whole < 2:
		_fail("at 1280x720 with nothing running the board shows %d whole offer(s) — it must show at least 2 to compare" % empty_whole)
		return
	main._sign_contract_offer(main._board_row_ids.find(offers[0]))
	if main._board_active_empty.visible or _left_column_ids(main) != [String(offers[0])]:
		_fail("signing from an empty board should replace the empty card with that docket, left %s" % str(_left_column_ids(main)))
		return
	if main._contract_empty_label.visible or main._contract_cards.size() != 1:
		_fail("signing an offer again should bring one docket back and hide the empty line")
		return

	print("CONTRACT_BOARD_OK: no docket on a new game, %d offers on the board in file order, C/E/ESC modal rules, SIGN debits once and refuses twice, ready/ok/risk/last-chance read four ways, a lost contract leaves the clipboard, is announced, and is re-signed at its price; an empty clipboard names the key; the board's left column holds the running dockets, gains a signature at once and has an empty card held to its pinned width; at 1280x720 the board shows %d whole offers on a new game and %d with nothing running" % [board.size(), fresh_whole, empty_whole])
	quit(0)
