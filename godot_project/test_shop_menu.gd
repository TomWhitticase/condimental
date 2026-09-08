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
#   * affordability is a visible slot state, not just a JSON field: with no
#     money every slot is disabled and its price carries the "unaffordable"
#     mark,
#   * and a REAL purchase — money earned by running the actual chain against
#     the actual contract, then spent through `issue_command_json` — moves
#     the displayed owned count.
extends SceneTree

## Ticks to run while trying to earn a contract payout. The hardcoded
## contract (sim_core/src/sim/economy.rs) pays out at a 1000-tick cadence and
## the chain's own latency is several hundred ticks on top, so this is a
## generous ceiling, not an expected cost — the loop breaks the moment money
## appears.
const EARN_TICK_LIMIT := 20000


func _fail(msg: String) -> void:
	printerr("SHOP_MENU_FAIL: %s" % msg)
	quit(1)


func _shop_snapshot(main) -> Dictionary:
	return JSON.parse_string(main.sim.shop_snapshot_json())


## Runs the real chain until the contract has paid out at least once, so
## there is genuinely earned money to spend — mirroring
## sim_core/tests/shop.rs's own drive loop. `collect_output` is issued every
## tick rather than only when something is ready: it is a documented no-op on
## an empty machine, which keeps this loop from needing to inspect chunks.
func _earn_money(main) -> float:
	for i in range(EARN_TICK_LIMIT):
		main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
		main.sim.issue_command_json(JSON.stringify({"type": "fill_fluid_tank"}))
		main.sim.tick()
		main.sim.issue_command_json(JSON.stringify({"type": "collect_output"}))
		var money: float = float(_shop_snapshot(main).get("money", 0.0))
		if money > 0.0:
			return money
	return 0.0


func _press(main, keycode: int) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
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
	# A fresh world has no money at all, so every slot must read as locked.
	if float(snapshot.get("money", 0.0)) != 0.0:
		_fail("expected a fresh world to start with zero money, got %.2f" % float(snapshot.get("money", 0.0)))
		return
	for slot in main._shop_slots:
		if not slot["button"].disabled:
			_fail("with zero money, slot '%s' should be disabled" % slot["kind"])
			return
		if not String(slot["price_label"].text).contains("✗"):
			_fail("an unaffordable slot's price text must carry the unaffordable mark, not rely on colour alone (slot '%s' reads '%s')" % [
				slot["kind"], slot["price_label"].text,
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
	var money: float = _earn_money(main)
	if money <= 0.0:
		_fail("the chain earned no contract money within %d ticks, so a purchase cannot be tested" % EARN_TICK_LIMIT)
		return
	main._update_shop_menu()

	# Buy the cheapest affordable kind, whatever the catalog currently says
	# that is — nothing here hardcodes data/shop.ron's prices.
	var target := -1
	for index in range(main._shop_slots.size()):
		if not main._shop_slots[index]["button"].disabled:
			target = index
			break
	if target < 0:
		_fail("earned %.0f money but no slot became affordable" % money)
		return

	var kind: String = main._shop_slots[target]["kind"]
	var owned_before: int = int(_shop_snapshot(main)["entries"][target]["owned"])
	var text_before: String = main._shop_slots[target]["owned_label"].text

	# Emit the button's own signal rather than calling _buy_slot directly —
	# this is what a click does, so this is what should be proven.
	main._shop_slots[target]["button"].pressed.emit()

	var after: Dictionary = _shop_snapshot(main)
	var owned_after: int = int(after["entries"][target]["owned"])
	if owned_after != owned_before + 1:
		_fail("clicking slot '%s' should have bought exactly one (%d -> %d)" % [kind, owned_before, owned_after])
		return
	if float(after["money"]) >= money:
		_fail("buying '%s' did not spend money (%.0f -> %.0f)" % [kind, money, float(after["money"])])
		return
	var text_after: String = main._shop_slots[target]["owned_label"].text
	if text_after == text_before:
		_fail("the displayed owned count for '%s' did not change after buying (still '%s')" % [kind, text_after])
		return
	if not text_after.contains(str(owned_after)):
		_fail("the displayed owned count for '%s' reads '%s', expected it to show %d" % [kind, text_after, owned_after])
		return

	# And the number-key half of the interaction buys too.
	if target < main.SHOP_HOTKEY_SLOTS:
		_press(main, KEY_1 + target)
		var owned_hotkey: int = int(_shop_snapshot(main)["entries"][target]["owned"])
		if owned_hotkey != owned_after + 1:
			_fail("the number-key shortcut for slot %d did not buy one of '%s' (%d -> %d)" % [
				target + 1, kind, owned_after, owned_hotkey,
			])
			return

	print("SHOP_MENU_OK: ShopMenu is a distinct node with %d slots (one per catalog entry), B toggles it, unaffordable slots are disabled and marked, and buying '%s' moved the displayed owned count to '%s'" % [
		main._shop_slots.size(), kind, main._shop_slots[target]["owned_label"].text,
	])
	quit(0)
