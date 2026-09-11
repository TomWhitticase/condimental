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
#   * a new game's clipboard carries ONE docket (the board's starter) and the
#     board menu carries one row per offer, in file order, the starter's row
#     reading SIGNED and every other row's SIGN button priced and enabled;
#   * C opens and closes the board through `_unhandled_input`, E swaps it for
#     the build menu, ESC closes it, and opening it changes nothing in the
#     simulation;
#   * clicking SIGN (driven through `_sign_contract_offer`) debits the price,
#     adds a docket to the clipboard, flips the row to SIGNED, and a second
#     click is refused — one of each contract, never two;
#   * `_contract_risk_for` — the pure half of the docket's status — sorts a
#     contract into ready / ok / at risk / last chance from the snapshot's own
#     fields, and the docket shows each state as a glyph AND words AND a bar
#     position AND a border, never colour alone;
#   * a contract left unserved for its strikes is LOST for real: it leaves the
#     clipboard, the announcement line says so, the board offers it again
#     with a LOST note, and signing it again costs its price;
#   * with every contract lost the clipboard says so in a sentence that names
#     the key to press.
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


func _row_for(main, id: String) -> Dictionary:
	for row in main._board_rows:
		if String(row["id"]) == id:
			return row
	return {}


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

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
		if String(main._board_rows[i]["name_label"].text) != String(board[i]["name"]):
			_fail("row '%s' shows the wrong name" % board[i]["id"])
			return
		var terms: String = String(main._board_rows[i]["terms_label"].text)
		for fragment in ["every %d ticks" % int(board[i]["cadence_ticks"]), "+%d" % int(board[i]["payout"]), "%d missed in a row" % int(board[i]["strikes"])]:
			if not terms.contains(fragment):
				_fail("row '%s' terms '%s' do not state '%s'" % [board[i]["id"], terms, fragment])
				return

	# --- A new game: one starter running, everything else for sale --------
	var starters: Array = []
	var offers: Array = []
	for offer in board:
		if bool(offer["starter"]):
			starters.append(String(offer["id"]))
		else:
			offers.append(String(offer["id"]))
	if starters.size() != 1:
		_fail("the shipped board must start a new game with exactly one contract, found %d starter(s)" % starters.size())
		return
	if offers.is_empty():
		_fail("the shipped board must offer at least one contract to buy")
		return
	if _card_ids(main) != starters:
		_fail("a new game's clipboard should carry the starter only, got %s" % str(_card_ids(main)))
		return
	if not main._contract_hint_label.text.contains("C"):
		_fail("the clipboard should tell the player which key opens the board")
		return
	main._update_contract_board()
	var starter_row := _row_for(main, starters[0])
	if not starter_row["sign_button"].disabled or String(starter_row["sign_button"].text) != "SIGNED":
		_fail("the starter's row must read SIGNED and be disabled, got '%s'" % starter_row["sign_button"].text)
		return
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

	# --- C opens and closes it, changing nothing in the simulation --------
	var econ_before: String = main.sim.economy_snapshot_json()
	_key(main, KEY_C)
	if not main._board_open or not board_panel.visible:
		_fail("C should open the contracts board")
		return
	if main.sim.economy_snapshot_json() != econ_before:
		_fail("opening the board changed the economy")
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
	main._sign_contract_offer(first_index)
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
		if not card["window_track"].has_meta("fraction"):
			_fail("docket '%s' has no window bar driven by _set_contract_bar" % card["id"])
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
	if not is_equal_approx(float(card0["window_track"].get_meta("fraction")), 0.8):
		_fail("the window bar should stand at 0.8, reads %s" % str(card0["window_track"].get_meta("fraction")))
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
	main._sign_contract_offer(main._board_row_ids.find(starters[0]))
	if main._contract_empty_label.visible or main._contract_cards.size() != 1:
		_fail("signing the starter again should bring one docket back and hide the empty line")
		return

	print("CONTRACT_BOARD_OK: one starter docket on a new game, %d offers on the board in file order, C/E/ESC modal rules, SIGN debits once and refuses twice, ready/ok/risk/last-chance read four ways, a lost contract leaves the clipboard, is announced, and is re-signed at its price; an empty clipboard names the key" % board.size())
	quit(0)
