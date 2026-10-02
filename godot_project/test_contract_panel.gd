# Headless structural proof of the contracts plate (docs/UI_UX.md §5) and of
# the docket cards the C board's YOUR CONTRACTS column draws.
#
# Since 2026-10-02 the always-visible plate is a dark plate in the right-hand
# column, one compact row per signed contract; the full docket (checklist,
# strikes, cadence, tallies) is the board's. Both halves are proved here,
# because GAMEPLAY.md §6's design pressure — several concurrent orders
# competing for one factory — only reaches the player if every order is on
# screen at once, and the details of each are one key away.
#
# Run with:
#   godot --headless --path godot_project --script test_contract_panel.gd
#
# Like test_presentation.gd, this deliberately does NOT claim to verify how
# the plate LOOKS — that needs eyes on scripts/screenshot-scene.sh's output.
# What it verifies is the structural half:
#
#   * the plate is its own node (`ContractPanel`), the column's width, and
#     shows the balance ONCE (a single shared counter, GAMEPLAY.md §1),
#   * there is exactly one row per contract `economy_snapshot_json` reports,
#     in the same (priority) order, each named after its contract id and
#     showing the contract's name, a status, a countdown in seconds and a
#     draining bar that counts down independently as the sim runs,
#   * a row's status says its state in a glyph AND a word, from the same
#     `_contract_risk_for` the docket reads (UI_UX.md §7),
#   * the docks are mentioned only when full, and in words,
#   * the money figure flashes what just CHANGED it,
#   * and on the board, every docket carries a checkbox per required unit of
#     ITS OWN mix — all the shipped contracts ask for the same item, so
#     per-item state cannot be shared — whose visual state actually changes.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("CONTRACT_PANEL_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# --- The panel is its own distinct node, not co-mingled Labels --------
	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	var contract_panel: PanelContainer = main._contract_panel
	if contract_panel == null:
		_fail("main._contract_panel was not built")
		return
	if contract_panel.name != "ContractPanel":
		_fail("expected the contract panel's node name to be 'ContractPanel', was '%s'" % contract_panel.name)
		return
	if contract_panel.get_parent() != hud:
		_fail("ContractPanel must be a direct child of the HUD CanvasLayer, not nested inside another panel")
		return

	# It must be a genuinely separate node from the general HUD panel (the
	# one holding the per-stage machine lines).
	var other_panels := 0
	for child in hud.get_children():
		if child is PanelContainer and child != contract_panel:
			other_panels += 1
	if other_panels < 1:
		_fail("expected at least one other HUD panel besides ContractPanel — did the general HUD panel disappear?")
		return

	# --- One row per reported contract, in the same order -----------------
	# Since 2026-09-26 a new game runs no contract and every one is signed;
	# this proof is about several orders at once, so sign every offer first,
	# through the real command, the way the board menu does
	# (test_contract_board.gd covers the menu itself).
	for offer in JSON.parse_string(main.sim.economy_snapshot_json()).get("board", []):
		if not bool(offer.get("active", false)):
			main.sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": String(offer["id"])}))
	main._update_contract_panel()
	# The signatures just spent money, which the money flyout announces; let
	# that flash expire so the flyout assertions further down start from
	# nothing showing, as they were written to.
	main._contract_money_change_until_msec = main._hud_now_msec() - main.MONEY_FLYOUT_HOLD_MSEC - main.MONEY_FLYOUT_FADE_MSEC - 1
	main._update_contract_panel()
	var econ: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	var contracts: Array = econ.get("contracts", [])
	if contracts.size() < 2:
		_fail("economy_snapshot_json reported %d contract(s) after signing every offer — this test needs the shipped multi-contract data/contracts.ron to prove anything" % contracts.size())
		return
	if main._contract_cards.size() != contracts.size():
		_fail("expected %d contract row(s), found %d" % [contracts.size(), main._contract_cards.size()])
		return

	# The plate is the column's width, so its edges line up with STOCK and
	# CREW under it — the owner's complaint about the clipboard it replaced.
	if contract_panel.custom_minimum_size.x != main.STOCK_PANEL_MIN_WIDTH:
		_fail("the contracts plate should be the column's width (%.0f), is pinned at %.0f" % [
			main.STOCK_PANEL_MIN_WIDTH, contract_panel.custom_minimum_size.x
		])
		return

	for i in range(contracts.size()):
		var row: Dictionary = main._contract_cards[i]
		var contract: Dictionary = contracts[i]
		var contract_id: String = String(contract["id"])
		if String(row["id"]) != contract_id:
			_fail("row %d is for '%s' but the snapshot's contract %d is '%s' — order must match, it is the scarcity priority" % [
				i, String(row["id"]), i, contract_id
			])
			return
		var panel: PanelContainer = row["panel"]
		if panel.get_parent() != main._contract_cards_box:
			_fail("row '%s' is not inside the plate's list" % contract_id)
			return
		if not String(panel.name).ends_with(contract_id):
			_fail("row node '%s' should be named after its contract id '%s'" % [panel.name, contract_id])
			return
		if String(row["name_label"].text) != String(contract["name"]):
			_fail("row '%s' shows the name '%s', expected '%s'" % [
				contract_id, row["name_label"].text, contract["name"]
			])
			return
		# Only what a glance needs: no checklist and no tallies on the plate.
		if row.has("rows") or row.has("done_label") or row.has("strike_pips"):
			_fail("row '%s' carries docket detail — the checklist, tallies and strikes are the C board's" % contract_id)
			return
		# The countdown is a DRAINING BAR as well as a figure, and the figure
		# is in seconds, not ticks — this plate is the player's clock.
		var countdown_text: String = String(row["countdown_label"].text)
		if countdown_text == "" or countdown_text.contains("tick"):
			_fail("row '%s' should show its time left in seconds, got: '%s'" % [contract_id, countdown_text])
			return
		if row["countdown_meter"] == null or not row["countdown_meter"].has_meta("fraction"):
			_fail("row '%s' has no countdown meter driven by _set_meter" % contract_id)
			return
		if String(row["status_label"].text) == "":
			_fail("row '%s' has no status" % contract_id)
			return

	# --- Money once -------------------------------------------------------
	var money_labels := _count_money_labels(contract_panel, main._contract_money_label.text)
	if money_labels != 1:
		_fail("expected exactly one money readout on the plate, found %d — money is a single shared counter (GAMEPLAY.md §1)" % money_labels)
		return
	if String(main._contract_money_label.text) != "%d" % int(float(econ.get("money", 0.0))):
		_fail("the balance should read the wire's money, got '%s'" % main._contract_money_label.text)
		return

	# --- A row's status: a glyph and a word for every state ---------------
	var base := {
		"id": "x", "name": "x", "payout": 50, "mix": [["boxed_ketchup", 2]],
		"progress": [["boxed_ketchup", 1]], "satisfied": false,
		"cadence_ticks": 1000, "next_delivery_tick": 1000, "strikes": 3,
		"consecutive_misses": 0,
	}
	var calm: Dictionary = main._contract_row_state(base, 100)
	if calm["level"] != "ok" or calm["status_text"] != "1/2":
		_fail("a calm row reads its tally, got %s" % str(calm))
		return
	if calm["time_text"] != main._format_remaining(900):
		_fail("a row's time left is `_format_remaining`'s, got '%s'" % calm["time_text"])
		return
	var risky: Dictionary = main._contract_row_state(base, 800)
	if risky["level"] != "risk" or not String(risky["status_text"]).begins_with("⚠"):
		_fail("an at-risk row carries the warning glyph, got %s" % str(risky))
		return
	if not risky["due"] and risky["fraction"] <= main.CONTRACT_DUE_FRACTION:
		_fail("a row this close to its check should read as due")
		return
	var last := base.duplicate()
	last["consecutive_misses"] = 2
	var last_state: Dictionary = main._contract_row_state(last, 100)
	if last_state["level"] != "last_chance" or not String(last_state["status_text"]).contains("LAST CHANCE"):
		_fail("one miss from lost reads LAST CHANCE in words, got %s" % str(last_state))
		return
	var full := base.duplicate()
	full["progress"] = [["boxed_ketchup", 2]]
	full["satisfied"] = true
	var ready: Dictionary = main._contract_row_state(full, 100)
	if ready["level"] != "ready" or not String(ready["status_text"]).begins_with("✔"):
		_fail("a filled row reads READY with its tick, got %s" % str(ready))
		return
	var paid: Dictionary = main._contract_row_state(base, 100, true)
	if not String(paid["status_text"]).contains("PAID"):
		_fail("a row that just paid reads PAID, got %s" % str(paid))
		return
	var due_now: Dictionary = main._contract_row_state(base, 1000)
	if due_now["time_text"] != "due now":
		_fail("a row at its check reads 'due now', got '%s'" % due_now["time_text"])
		return
	# The row's border takes the state's style.
	var row0: Dictionary = main._contract_cards[0]
	main._update_contract_row(row0, last, 100)
	if row0["panel"].get_theme_stylebox("panel") != main._contract_row_styles["last_chance"]:
		_fail("a last-chance row should carry the last-chance rule")
		return
	if row0["status_label"].label_settings != main._contract_row_status_settings["last_chance"]:
		_fail("a last-chance row's status should take the warning ink")
		return

	# Countdowns must actually count DOWN, independently, as the sim runs.
	main._update_contract_panel()
	var before: Array = _countdowns(main)
	for i in range(250):
		main.sim.tick()
	main._update_contract_panel()
	var after: Array = _countdowns(main)
	for i in range(before.size()):
		if after[i] >= before[i]:
			_fail("row %d's countdown did not advance with the simulation (%f -> %f)" % [i, before[i], after[i]])
			return

	# --- The docks: said only when full, and in words ---------------------
	if main._contract_dock_full_label.visible:
		_fail("the docks are empty but the plate already says they are full")
		return
	var full_econ: Dictionary = econ.duplicate()
	full_econ["delivery_full"] = true
	main._update_contract_panel(full_econ)
	if not main._contract_dock_full_label.visible or not String(main._contract_dock_full_label.text).contains("FULL"):
		_fail("full docks must say so in words on the plate (UI_UX.md 7)")
		return
	main._update_contract_panel()
	if main._contract_dock_full_label.visible:
		_fail("the docks are no longer full but the plate still says FULL")
		return

	# --- The money figure announces what it just did ----------------------
	# A payout lands on its own clock while the player is looking at the
	# factory, so a balance that only ever shows its current value hides
	# every event that changed it. Spending is the half a test can drive
	# deterministically: buy one belt and the flyout must say so.
	if main._contract_money_change.modulate.a > 0.0:
		_fail("the money flyout is showing with nothing having changed the balance")
		return
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	main._update_contract_panel()
	var flyout_text: String = String(main._contract_money_change.text)
	if not flyout_text.begins_with("-"):
		_fail("a purchase should flash a debit on the money figure, got: %s" % flyout_text)
		return
	if main._contract_money_change.modulate.a <= 0.0:
		_fail("the money flyout flashed nothing visible for a purchase")
		return
	if main._contract_money_change.label_settings != main._contract_spend_settings:
		_fail("a debit must be inked as a debit, not as a payout")
		return

	# --- The single-contract case still renders --------------------------
	var single: Array = [contracts[0]]
	main._rebuild_contract_cards(single)
	if main._contract_cards.size() != 1:
		_fail("a one-contract snapshot must render exactly one row, got %d" % main._contract_cards.size())
		return
	if String(main._contract_cards[0]["id"]) != String(contracts[0]["id"]):
		_fail("the single row is for the wrong contract")
		return

	# --- The board's dockets: one checkbox per unit of THEIR OWN mix ------
	main._rebuild_board_active_cards(contracts)
	var dockets: Array = main._board_active_cards
	if dockets.size() != contracts.size():
		_fail("expected %d docket(s) on the board, found %d" % [contracts.size(), dockets.size()])
		return
	for i in range(contracts.size()):
		var card: Dictionary = dockets[i]
		var contract: Dictionary = contracts[i]
		var contract_id: String = String(contract["id"])
		main._update_contract_card(card, contract, main.sim.current_tick())
		var mix: Array = contract["mix"]
		if mix.is_empty():
			_fail("contract '%s' reported an empty mix — nothing to check against" % contract_id)
			return
		for pair in mix:
			var item_id: String = String(pair[0])
			var required: int = int(pair[1])
			var checklist: Dictionary = card["rows"].get(item_id, {})
			if checklist.is_empty():
				_fail("docket '%s' has no checklist row for '%s'" % [contract_id, item_id])
				return
			var boxes: Array = checklist["boxes"]
			var expected_box_count: int = required if required <= main.CONTRACT_CHECKLIST_MAX_BOXES else 1
			if boxes.size() != expected_box_count:
				_fail("docket '%s': expected %d checkbox(es) for '%s' (required=%d), found %d" % [
					contract_id, expected_box_count, item_id, required, boxes.size()
				])
				return
			# The checklist names its item the way the rest of the UI names it.
			if not String(checklist["label"].text).begins_with(main._item_display_name(item_id)):
				_fail("the checklist should name its item, got: %s" % checklist["label"].text)
				return
		var expected_cadence := "every %s" % main._format_remaining(int(contract.get("cadence_ticks", 0)))
		if String(card["meta_label"].text) != expected_cadence:
			_fail("docket '%s' cadence line is '%s', expected '%s'" % [contract_id, card["meta_label"].text, expected_cadence])
			return
		if not String(card["done_label"].text).begins_with("✓") or not String(card["missed_label"].text).begins_with("✗"):
			_fail("the docket's tallies should be a glyph and a word")
			return

	# Every shipped contract asks for the same item, which is exactly the case
	# a per-item (rather than per-contract) node dictionary would collapse.
	var first_boxes: Array = dockets[0]["rows"].values()[0]["boxes"]
	var second_boxes: Array = dockets[1]["rows"].values()[0]["boxes"]
	if first_boxes[0] == second_boxes[0]:
		_fail("two dockets share the same checkbox node — per-contract state got collapsed by item id")
		return

	var box: Panel = first_boxes[0]
	var mark: Label = box.get_node_or_null("Mark")
	if mark == null:
		_fail("checkbox '%s' has no 'Mark' tick-mark child" % box.name)
		return
	main._set_checkbox_checked(box, true)
	if box.get_theme_stylebox("panel") != main._checkbox_style_checked or not mark.visible:
		_fail("checking a box must swap in the checked style and reveal its tick")
		return
	main._set_checkbox_checked(box, false)
	if box.get_theme_stylebox("panel") != main._checkbox_style_unchecked or mark.visible:
		_fail("unchecking a box must restore the unchecked style and hide its tick")
		return

	print("CONTRACT_PANEL_OK: ContractPanel is a column-width plate with %d row(s) — one per contract in priority order, each a name, a glyph-and-word status from the docket's own risk level, a countdown in seconds and a draining bar; one shared balance that flashes what it just did; full docks said in words; and the board's dockets keep per-contract checkboxes" % contracts.size())
	quit(0)


## Every Label on the plate whose text is the money figure. Guards against
## money creeping onto each row as contracts multiplied.
func _count_money_labels(node: Node, figure: String) -> int:
	var found := 0
	if node is Label and String(node.text) == figure:
		found += 1
	for child in node.get_children():
		found += _count_money_labels(child, figure)
	return found


## How full each row's countdown bar stands. Read from the meter rather
## than parsed back out of a label: the bar IS the countdown now, and a
## fraction is the number the panel was actually asked to draw.
func _countdowns(main) -> Array:
	var out: Array = []
	for card in main._contract_cards:
		out.append(float(card["countdown_meter"].get_meta("fraction", 0.0)))
	return out
