# Headless structural proof of the contract "clipboard" panel (docs/UI_UX.md
# §5 — "a contract IS a clipboard"), which reskinned the plain money/contract
# Label into its own diegetic panel with checkbox-style tick marks, and now
# shows EVERY active contract at once (GAMEPLAY.md §6: several concurrent
# orders competing for one factory's throughput is the whole design pressure,
# and it only reaches the player if they are all on screen).
#
# Run with:
#   godot --headless --path godot_project --script test_contract_panel.gd
#
# Like test_presentation.gd, this deliberately does NOT claim to verify that
# the panel *looks* like a clipboard — that needs eyes on
# scripts/screenshot-scene.sh's output. What it verifies is the structural
# half:
#
#   * the economy/contract readout lives in its own distinct node
#     (`ContractPanel`), separate from whatever the general HUD panel above
#     it contains,
#   * there is exactly one docket card per contract `economy_snapshot_json`
#     reports, in the same (priority) order, each named after its contract id,
#   * every card carries a checkbox per required unit of ITS OWN mix — the
#     case that broke the old single-contract panel is that all three shipped
#     contracts ask for the same item, so per-item state cannot be shared,
#   * a checkbox's visual state (its stylebox override + tick-mark
#     visibility) actually changes when progress changes — driven directly
#     through `_set_checkbox_checked`, the same "call the method directly, no
#     live mouse/window needed" approach test_camera_nav.gd uses,
#   * money appears ONCE (a single shared counter, GAMEPLAY.md §1) while each
#     card's countdown is its OWN — verified by driving the real simulation
#     forward and asserting the cards' countdowns diverge,
#   * and the single-contract case still renders correctly, driven through
#     `_rebuild_contract_cards` with a one-contract array so the regression
#     the multi-contract work could plausibly cause is actually covered.
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

	# --- One docket per reported contract, in the same order --------------
	var econ: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	var contracts: Array = econ.get("contracts", [])
	if contracts.size() < 2:
		_fail("economy_snapshot_json reported %d contract(s) — this test needs the shipped multi-contract data/contracts.ron to prove anything" % contracts.size())
		return
	if main._contract_cards.size() != contracts.size():
		_fail("expected %d docket card(s), found %d" % [contracts.size(), main._contract_cards.size()])
		return

	for i in range(contracts.size()):
		var card: Dictionary = main._contract_cards[i]
		var contract: Dictionary = contracts[i]
		var contract_id: String = String(contract["id"])
		if String(card["id"]) != contract_id:
			_fail("docket %d is for '%s' but the snapshot's contract %d is '%s' — order must match, it is the scarcity priority" % [
				i, String(card["id"]), i, contract_id
			])
			return
		var panel: PanelContainer = card["panel"]
		if panel.get_parent() != main._contract_cards_box:
			_fail("docket '%s' is not inside the panel's card list" % contract_id)
			return
		if not String(panel.name).ends_with(contract_id):
			_fail("docket node '%s' should be named after its contract id '%s'" % [panel.name, contract_id])
			return
		if String(card["name_label"].text) != String(contract["name"]):
			_fail("docket '%s' shows the name '%s', expected '%s'" % [
				contract_id, card["name_label"].text, contract["name"]
			])
			return

		# --- Checkboxes: one per required unit of THIS contract's mix -----
		var mix: Array = contract["mix"]
		if mix.is_empty():
			_fail("contract '%s' reported an empty mix — nothing to check against" % contract_id)
			return
		for pair in mix:
			var item_id: String = String(pair[0])
			var required: int = int(pair[1])
			var row: Dictionary = card["rows"].get(item_id, {})
			if row.is_empty():
				_fail("docket '%s' has no checklist row for '%s'" % [contract_id, item_id])
				return
			var boxes: Array = row["boxes"]
			var expected_box_count: int = required if required <= main.CONTRACT_CHECKLIST_MAX_BOXES else 1
			if boxes.size() != expected_box_count:
				_fail("docket '%s': expected %d checkbox(es) for '%s' (required=%d), found %d" % [
					contract_id, expected_box_count, item_id, required, boxes.size()
				])
				return
			for box in boxes:
				if not (box is Panel):
					_fail("checklist entries must be Panel nodes (the checkbox squares)")
					return

	# Every shipped contract asks for the same item, which is exactly the case
	# a per-item (rather than per-contract) node dictionary would collapse.
	# Assert the dockets really do own distinct nodes.
	var first_boxes: Array = main._contract_cards[0]["rows"].values()[0]["boxes"]
	var second_boxes: Array = main._contract_cards[1]["rows"].values()[0]["boxes"]
	if first_boxes[0] == second_boxes[0]:
		_fail("two dockets share the same checkbox node — per-contract state got collapsed by item id")
		return

	# --- A checkbox's visual state actually changes -----------------------
	var box: Panel = first_boxes[0]
	var mark: Label = box.get_node_or_null("Mark")
	if mark == null:
		_fail("checkbox '%s' has no 'Mark' tick-mark child" % box.name)
		return

	var style_before = box.get_theme_stylebox("panel")
	main._set_checkbox_checked(box, true)
	if box.get_theme_stylebox("panel") == style_before:
		_fail("checking a box did not change its stylebox override")
		return
	if not mark.visible:
		_fail("checking a box did not reveal its tick mark")
		return
	if box.get_theme_stylebox("panel") != main._checkbox_style_checked:
		_fail("a checked box must use the checked stylebox resource")
		return

	main._set_checkbox_checked(box, false)
	if box.get_theme_stylebox("panel") != main._checkbox_style_unchecked:
		_fail("unchecking a box did not restore the unchecked stylebox")
		return
	if mark.visible:
		_fail("unchecking a box did not hide its tick mark")
		return

	# --- The full update path (economy_snapshot_json -> box state) --------
	main._update_contract_panel()
	for card in main._contract_cards:
		for item_id in card["rows"]:
			for b in card["rows"][item_id]["boxes"]:
				if bool(b.get_meta("checked", false)):
					_fail("with an empty delivery pool, no checkbox should read as checked yet")
					return

	# --- Money once, countdowns per contract ------------------------------
	var money_labels := _count_money_labels(contract_panel)
	if money_labels != 1:
		_fail("expected exactly one money readout on the clipboard, found %d — money is a single shared counter (GAMEPLAY.md §1)" % money_labels)
		return

	var meta_texts: Dictionary = {}
	for card in main._contract_cards:
		meta_texts[String(card["meta_label"].text)] = true
	if meta_texts.size() < main._contract_cards.size():
		_fail("dockets share an identical cadence line — each contract clocks independently, so its countdown must be its own")
		return

	# Countdowns must actually count DOWN, independently, as the sim runs.
	var before: Array = _countdowns(main)
	for i in range(250):
		main.sim.tick()
	main._update_contract_panel()
	var after: Array = _countdowns(main)
	for i in range(before.size()):
		if after[i] >= before[i]:
			_fail("docket %d's countdown did not advance with the simulation (%d -> %d)" % [i, before[i], after[i]])
			return

	# --- The shared dock pool is surfaced, and tracks real deliveries -----
	if main._contract_dock_row == null:
		_fail("no shared delivery-pool readout on the clipboard")
		return
	if not main._contract_dock_empty_label.visible:
		_fail("with nothing delivered, the dock line should read as empty")
		return
	var delivered_item: String = _run_until_delivery(main)
	if delivered_item.is_empty():
		_fail("could not get any product onto the dock — cannot verify the shared-pool readout")
		return
	main._update_contract_panel()
	if main._contract_dock_empty_label.visible:
		_fail("stock is on the dock but the readout still says empty")
		return
	if not main._contract_dock_entries.has(delivered_item):
		_fail("delivered '%s' but the dock readout has no entry for it" % delivered_item)
		return

	# --- The single-contract case still renders --------------------------
	# The regression this ticket could plausibly cause is the OPPOSITE of the
	# one it fixes, so drive the builder with a one-contract array directly.
	var single: Array = [contracts[0]]
	main._rebuild_contract_cards(single)
	if main._contract_cards.size() != 1:
		_fail("a one-contract snapshot must render exactly one docket, got %d" % main._contract_cards.size())
		return
	var only_card: Dictionary = main._contract_cards[0]
	if String(only_card["id"]) != String(contracts[0]["id"]):
		_fail("the single docket is for the wrong contract")
		return
	var only_item: String = String(contracts[0]["mix"][0][0])
	var only_required: int = int(contracts[0]["mix"][0][1])
	var only_boxes: Array = only_card["rows"][only_item]["boxes"]
	var expected_single: int = only_required if only_required <= main.CONTRACT_CHECKLIST_MAX_BOXES else 1
	if only_boxes.size() != expected_single:
		_fail("single-contract docket: expected %d checkbox(es), found %d" % [expected_single, only_boxes.size()])
		return
	main._update_contract_card(only_card, contracts[0], main.sim.current_tick())
	if not String(only_card["meta_label"].text).contains("next check in"):
		_fail("single-contract docket has no countdown line")
		return

	print("CONTRACT_PANEL_OK: ContractPanel is a distinct node with %d docket(s) — one per contract, per-contract checkboxes and countdowns, one shared money readout and one shared dock line; the single-contract case still renders" % contracts.size())
	quit(0)


## Every Label on the clipboard whose text is the money figure. Guards
## against money creeping onto each docket as contracts multiplied.
func _count_money_labels(node: Node) -> int:
	var found := 0
	if node is Label and String(node.text).begins_with("Money:"):
		found += 1
	for child in node.get_children():
		found += _count_money_labels(child)
	return found


func _countdowns(main) -> Array:
	var out: Array = []
	for card in main._contract_cards:
		var text: String = String(card["meta_label"].text)
		# "next check in N ticks  ·  ..." — N is the second-to-last token of
		# the first segment.
		out.append(int(text.split(" ticks")[0].split("in ")[1]))
	return out


## Runs the real chain until something lands in the shared delivery pool,
## exactly the way test_shop_menu.gd earns money: real commands, real ticks.
## Returns the item id delivered, or "" if nothing arrived.
func _run_until_delivery(main) -> String:
	for i in range(20000):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		main.sim.tick()
		main.sim.issue_command_json(JSON.stringify({"type": "collect_output"}))
		var stock: Array = JSON.parse_string(main.sim.economy_snapshot_json()).get("delivery_stock", [])
		if not stock.is_empty():
			return String(stock[0][0])
	return ""
