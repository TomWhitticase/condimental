# Headless structural proof of the build menu (docs/UI_UX.md §5 — "build/
# recipe menus as grids of voxel-rendered icon slots") — the "Shop &
# Inventory" epic's ticket 2, the presentation half of ticket 1's shop.
#
# Run with:
#   godot --headless --path godot_project --script test_shop_menu.gd
#
# Like test_contract_panel.gd, this deliberately does NOT claim the menu
# *looks* right — that needs eyes on scripts/screenshot-scene.sh --shop.
# What it verifies is everything that can be checked without a framebuffer:
#
#   * the menu is its own distinct node (`ShopMenu`), a direct child of the
#     HUD and separate from every other HUD panel, so nothing a sibling
#     ticket does to those disturbs it,
#   * it renders exactly one slot per `shop_snapshot_json` catalog entry, in
#     the snapshot's own order, each carrying a real icon plus price and
#     owned-count text that match the snapshot's numbers,
#   * B actually toggles it open and closed, driven through the scene's real
#     `_unhandled_input` (the approach test_interaction.gd establishes),
#   * affordability is a visible slot state, not just a JSON field: a new
#     world's STARTING_MONEY leaves every slot open AND covers one of
#     everything in the catalog twice over, and spending it locks the slots
#     and marks each price with the unaffordable glyph,
#   * a REAL purchase through `issue_command_json` moves the displayed owned
#     count, on both the click and the SHIFT+number paths,
#   * and an OWNED slot offers the second action the "Real Placement" epic
#     added — putting one in your hand — which enters placement mode and
#     closes the menu.
#
# WHAT THE INVENTORY-HOTBAR TICKET CHANGED HERE. The number keys used to buy
# while this menu was open; they are the hotbar's selection keys now, in every
# mode, menu or no menu (see test_hotbar.gd). Buying kept a keyboard path but
# moved onto SHIFT+number, and that key only buys WITH THIS MENU OPEN — the
# one screen where the price and the balance being spent are both visible.
# The checks below follow that split rather than being deleted with it.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("SHOP_MENU_FAIL: %s" % msg)
	quit(1)


func _shop_snapshot(main) -> Dictionary:
	return JSON.parse_string(main.sim.shop_snapshot_json())


func _press(main, keycode: int, shift := false) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	event.shift_pressed = shift
	main._unhandled_input(event)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# --- The menu is its own distinct node -------------------------------
	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	var menu: PanelContainer = main._shop_menu
	if menu == null:
		_fail("main._shop_menu was not built")
		return
	if menu.name != "ShopMenu":
		_fail("expected the build menu's node name to be 'ShopMenu', was '%s'" % menu.name)
		return
	if menu.get_parent() != hud:
		_fail("ShopMenu must be a direct child of the HUD CanvasLayer, not nested in another panel")
		return
	if menu == main._contract_panel:
		_fail("the build menu must be a genuinely separate node from the contract clipboard")
		return
	var other_panels := 0
	for child in hud.get_children():
		if child is PanelContainer and child != menu:
			other_panels += 1
	if other_panels < 2:
		_fail("expected the HUD to still hold its other panels (help strip, contract clipboard) alongside ShopMenu, found %d" % other_panels)
		return
	if menu.visible:
		_fail("the build menu must start closed")
		return

	# --- One slot per catalog entry, with the snapshot's own numbers ------
	var snapshot: Dictionary = _shop_snapshot(main)
	var entries: Array = snapshot.get("entries", [])
	if entries.is_empty():
		_fail("shop_snapshot_json reported an empty catalog — nothing to build a menu from")
		return
	if main._shop_slots.size() != entries.size():
		_fail("expected %d slots (one per catalog entry), found %d" % [entries.size(), main._shop_slots.size()])
		return

	var grid: Node = menu.find_child("ShopGrid", true, false)
	if grid == null:
		_fail("no ShopGrid container found under ShopMenu")
		return
	if grid.get_child_count() != entries.size():
		_fail("ShopGrid holds %d children, expected %d slot buttons" % [grid.get_child_count(), entries.size()])
		return

	main._update_shop_menu()
	for index in range(entries.size()):
		var entry: Dictionary = entries[index]
		var slot: Dictionary = main._shop_slots[index]
		if String(slot["kind"]) != String(entry["kind"]):
			_fail("slot %d is '%s' but the catalog's entry %d is '%s' — menu order must follow the snapshot" % [
				index, slot["kind"], index, entry["kind"],
			])
			return
		if not (slot["button"] is Button):
			_fail("slot '%s' is not a Button, so it cannot be clicked" % slot["kind"])
			return
		var icon: TextureRect = slot["icon"]
		if icon.texture == null:
			_fail("slot '%s' has no icon texture" % slot["kind"])
			return
		if icon.texture.get_width() < 16 or icon.texture.get_height() < 16:
			_fail("slot '%s' fell back to the placeholder icon (%dx%d) — expected the real generated icon for asset '%s'" % [
				slot["kind"], icon.texture.get_width(), icon.texture.get_height(),
				main._shop_asset_for(String(slot["kind"])),
			])
			return
		var price_text: String = slot["price_label"].text
		if not price_text.contains("%.0f" % float(entry["price"])):
			_fail("slot '%s' price text '%s' does not show its catalog price %.0f" % [
				slot["kind"], price_text, float(entry["price"]),
			])
			return
		var owned_text: String = slot["owned_label"].text
		if not owned_text.contains(str(int(entry["owned"]))):
			_fail("slot '%s' owned text '%s' does not show its owned count %d" % [
				slot["kind"], owned_text, int(entry["owned"]),
			])
			return

	# --- Affordability is a visible slot state, not just a JSON field -----
	# A new world starts with `sim::STARTING_MONEY`, and every slot should
	# read as available. (Before the "Real Placement" epic a fresh world had
	# zero and this test asserted the opposite — the starting balance is what
	# makes a first move possible at all.)
	var starting_money: float = float(snapshot.get("money", 0.0))
	if starting_money <= 0.0:
		_fail("expected a new world to start with STARTING_MONEY, got %.2f" % starting_money)
		return
	for slot in main._shop_slots:
		if slot["button"].disabled:
			_fail("with %.0f in the bank, slot '%s' should not be locked" % [starting_money, slot["kind"]])
			return

	# And it is not merely "the cheapest thing is affordable". The starting
	# balance was raised from 200 to 3000 precisely because a new game has to
	# be able to reach a first contract payout, which means building a line
	# that ends at a dock — so it must cover a whole factory's worth of
	# catalog, not a first move. Priced off the SNAPSHOT rather than off
	# `data/shop.ron`'s literals, so a balancing pass moves this with it;
	# `sim_core/tests/shop.rs` pins the exact relationship (starting money
	# against the reference chain's own dynamically-priced bill), and this is
	# the wire-side shadow of it.
	var one_of_everything := 0.0
	for entry in entries:
		one_of_everything += float(entry["price"])
	if starting_money <= one_of_everything * 2.0:
		_fail("a starting balance of %.0f must comfortably cover a factory, not a first move: one of every catalog kind is %.0f" % [
			starting_money, one_of_everything,
		])
		return

	# --- B toggles the menu, through the real input path ------------------
	_press(main, KEY_B)
	if not menu.visible or not main._shop_open:
		_fail("pressing B did not open the build menu")
		return
	_press(main, KEY_B)
	if menu.visible or main._shop_open:
		_fail("pressing B a second time did not close the build menu")
		return
	_press(main, KEY_B) # leave it open for the purchase below

	# --- A real purchase moves the displayed owned count ------------------
	# Buy the cheapest affordable kind, whatever the catalog currently says
	# that is — nothing here hardcodes data/shop.ron's prices.
	var target := -1
	for index in range(main._shop_slots.size()):
		if not main._shop_slots[index]["button"].disabled:
			target = index
			break
	if target < 0:
		_fail("no slot is affordable at all with %.0f in the bank" % starting_money)
		return

	var kind: String = main._shop_slots[target]["kind"]
	var owned_before: int = int(_shop_snapshot(main)["entries"][target]["owned"])
	var text_before: String = main._shop_slots[target]["owned_label"].text

	# Emit the button's own signal rather than calling _buy_slot directly —
	# this is what a LEFT click does, so this is what should be proven.
	main._shop_slots[target]["button"].pressed.emit()

	var after: Dictionary = _shop_snapshot(main)
	var owned_after: int = int(after["entries"][target]["owned"])
	if owned_after != owned_before + 1:
		_fail("clicking slot '%s' should have bought exactly one (%d -> %d)" % [kind, owned_before, owned_after])
		return
	if float(after["money"]) >= starting_money:
		_fail("buying '%s' did not spend money (%.0f -> %.0f)" % [kind, starting_money, float(after["money"])])
		return
	var text_after: String = main._shop_slots[target]["owned_label"].text
	if text_after == text_before:
		_fail("the displayed owned count for '%s' did not change after buying (still '%s')" % [kind, text_after])
		return
	if not text_after.contains(str(owned_after)):
		_fail("the displayed owned count for '%s' reads '%s', expected it to show %d" % [kind, text_after, owned_after])
		return

	# --- An OWNED slot offers the second action: put one in your hand -----
	# The affordance is on the owned line, because that is the line whose
	# number decides whether placing is possible at all.
	if not text_after.to_lower().contains("right-click"):
		_fail("an owned slot should say how to take one in hand, reads '%s'" % text_after)
		return
	main._place_from_slot(target)
	if not main._in_build_mode() or main._build_kind != kind:
		_fail("placing from an owned slot did not enter placement mode for '%s'" % kind)
		return
	if main._shop_open:
		_fail("entering placement mode should close the build menu")
		return
	main._exit_build_mode()
	_press(main, KEY_B) # reopen for the hotkey check below

	# And the keyboard half of BUYING is SHIFT+number, while this menu is up.
	# (Its plain-number half became the hotbar's selection key — checked
	# immediately below, and in full in test_hotbar.gd.)
	if target < main.SHOP_HOTKEY_SLOTS:
		var buy_key: int = main.HOTBAR_KEYCODES[target]
		_press(main, buy_key, true)
		var owned_hotkey: int = int(_shop_snapshot(main)["entries"][target]["owned"])
		if owned_hotkey != owned_after + 1:
			_fail("SHIFT+%s with the menu open did not buy one of '%s' (%d -> %d)" % [
				main._hotbar_key_label(target), kind, owned_after, owned_hotkey,
			])
			return
		if not main._shop_open:
			_fail("buying with SHIFT+number should leave the build menu open — it is where the prices are")
			return

		# The same key WITHOUT shift must not spend a thing: it selects.
		var money_before_select: float = float(_shop_snapshot(main)["money"])
		_press(main, buy_key)
		var after_select: Dictionary = _shop_snapshot(main)
		if float(after_select["money"]) != money_before_select:
			_fail("a bare number key must never spend money (%.0f -> %.0f)" % [
				money_before_select, float(after_select["money"]),
			])
			return
		if int(after_select["entries"][target]["owned"]) != owned_hotkey:
			_fail("a bare number key must not change what is owned")
			return
		if not main._in_build_mode() or main._build_kind != kind:
			_fail("a bare number key should have selected '%s' for placement" % kind)
			return
		main._exit_build_mode()
		_press(main, KEY_B) # reopen for the spend-down below

	# --- Spend the balance: now unaffordability has to be VISIBLE ---------
	# Buy the most expensive kind until nothing is affordable, then check the
	# locked state is signalled by more than colour (UI_UX.md §7).
	for i in range(200):
		if float(_shop_snapshot(main).get("money", 0.0)) <= 0.0:
			break
		var dearest := -1
		var dearest_price := 0.0
		var live: Array = _shop_snapshot(main)["entries"]
		for index in range(live.size()):
			if bool(live[index]["affordable"]) and float(live[index]["price"]) > dearest_price:
				dearest = index
				dearest_price = float(live[index]["price"])
		if dearest < 0:
			break
		main._buy_slot(dearest)
	main._update_shop_menu()
	var locked := 0
	for slot in main._shop_slots:
		if slot["button"].disabled:
			locked += 1
			if not String(slot["price_label"].text).contains("✗"):
				_fail("an unaffordable slot's price text must carry the unaffordable mark, not rely on colour alone (slot '%s' reads '%s')" % [
					slot["kind"], slot["price_label"].text,
				])
				return
	if locked == 0:
		_fail("spending the whole balance left every slot affordable — the locked state was never exercised")
		return

	print("SHOP_MENU_OK: ShopMenu is a distinct node with %d slots (one per catalog entry), B toggles it, buying '%s' moved the displayed owned count, an owned slot enters placement mode, and %d slot(s) lock and mark themselves once the balance is spent" % [
		main._shop_slots.size(), kind, locked,
	])
	quit(0)
