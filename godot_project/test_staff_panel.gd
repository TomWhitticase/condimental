# Headless proof of the staff pools MVP's Godot half (ticket 3;
# `docs/STAFF_POOLS_PLAN.md`, "Presentation"): the STAFF panel on H, its
# polling, HIRE/FIRE through the real commands, and the hover lines an
# understaffed machine and dock gain. Godot derives nothing here — every
# figure on the panel is read back against `staff_snapshot_json`.
#
# Run with:
#   godot --headless --path godot_project --script test_staff_panel.gd
#
# Verifies:
#   * a row's words and bar are pure functions of the wire's category
#     (`hired N`, `assigned / demand` printed on the meter, `N short` in
#     the warning colour, FIRE disabled at 0, the maintenance idle note),
#   * the footer says the bill (every head at its wage), the payday
#     countdown in seconds from `current_tick()`, and the unpaid line only
#     when someone left,
#   * on the reference line the panel's rows match the snapshot in wire
#     order, with ticket 2's icon (not the fallback) on each,
#   * H opens the panel and closes the market; opening the market closes
#     it; ESC closes it without opening the pause menu; the legend names H,
#   * HIRE and FIRE move `headcount` on the wire by one and the row follows
#     on the same call; FIRE at 0 is disabled and a no-op,
#   * the understaffed fixture's vat reads `staff` on the wire and starved
#     in its load record; its hover leads with the starved glyph, names the
#     category (`short of manufacturing staff`) and its crew, and its load
#     bar is the amber one; the panel's manufacturing row reads `2 short`
#     on a red bar,
#   * a dock refused its logistics head says `held · short of logistics
#     staff` under `crew: 1 logistics`, and says neither while staffed,
#   * the build-menu card shows a machine's crew under its price and
#     nothing for a belt,
#   * the panel polls on the tick at LOAD_POLL_TICKS, the pause menu
#     freezes it, and a rebuild reads the wire before the first frame.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false
var _failed := false


## Prints the failure and keeps going, so one run lists every problem; the
## success marker is withheld at the end (see test_machine_load.gd).
func _fail(msg: String) -> void:
	_failed = true
	printerr("STAFF_PANEL_FAIL: %s" % msg)


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


func _key(keycode: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _step() -> void:
	main._advance_simulation(1.0 / float(main.TICK_HZ))


func _staff() -> Dictionary:
	return JSON.parse_string(main.sim.staff_snapshot_json())


func _category(id: String) -> Dictionary:
	for category in _staff().get("categories", []):
		if String(category.get("id", "")) == id:
			return category
	return {}


func _row(id: String) -> Dictionary:
	for row in main._staff_rows:
		if String(row["id"]) == id:
			return row
	return {}


func _run_checks() -> void:
	# --- 1. a row's state, pure --------------------------------------------
	var full: Dictionary = main._staff_row_state({
		"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2,
		"demand": 4, "assigned": 4, "short": 0,
	})
	if full["hired_text"] != "hired 4":
		_fail("a row reads 'hired N', got '%s'" % full["hired_text"])
	if full["wage_text"].find("2") < 0 or full["wage_text"].find("wage") < 0:
		_fail("a row names the wage per head, got '%s'" % full["wage_text"])
	if absf(float(full["meter"]["fraction"]) - 1.0) > 0.001:
		_fail("4 assigned of 4 asked is a full bar, got %s" % str(full["meter"]["fraction"]))
	if full["meter"]["color"] != main.METER_FILL_LEVEL:
		_fail("a fully staffed row's bar is the level colour")
	if full["meter_text"] != "4 / 4":
		_fail("the meter prints assigned / demand, got '%s'" % full["meter_text"])
	if full["short_text"] != "":
		_fail("nothing short says nothing, got '%s'" % full["short_text"])
	if full["note"] != "":
		_fail("a staffed category has no idle note")
	if not bool(full["fire_enabled"]):
		_fail("FIRE is enabled with heads to fire")
	var short: Dictionary = main._staff_row_state({
		"id": "manufacturing", "name": "Manufacturing", "headcount": 2, "wage": 2,
		"demand": 4, "assigned": 2, "short": 2,
	})
	if absf(float(short["meter"]["fraction"]) - 0.5) > 0.001:
		_fail("2 assigned of 4 asked is a half bar, got %s" % str(short["meter"]["fraction"]))
	if short["meter"]["color"] != main.METER_FILL_WARN:
		_fail("a short row's bar is the warning colour")
	if short["meter_text"] != "2 / 4":
		_fail("a short row's meter reads '2 / 4', got '%s'" % short["meter_text"])
	if short["short_text"] != "2 short":
		_fail("a short row says '2 short' in words, got '%s'" % short["short_text"])
	var idle: Dictionary = main._staff_row_state({
		"id": "maintenance", "name": "Maintenance", "headcount": 0, "wage": 2,
		"demand": 0, "assigned": 0, "short": 0,
	})
	if idle["note"] != "nothing to maintain yet":
		_fail("maintenance with nothing hired and nothing asked reads 'nothing to maintain yet', got '%s'" % idle["note"])
	if bool(idle["fire_enabled"]):
		_fail("FIRE is disabled at headcount 0")
	var idle_logistics: Dictionary = main._staff_row_state({
		"id": "logistics", "name": "Logistics", "headcount": 0, "wage": 2,
		"demand": 0, "assigned": 0, "short": 0,
	})
	if idle_logistics["note"] != "" or idle_logistics["meter_text"] != "0 / 0":
		_fail("an emptied logistics crew keeps its (empty) meter, not maintenance's note")
	var nobody_asked: Dictionary = main._staff_row_state({
		"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 2,
		"demand": 0, "assigned": 0, "short": 0,
	})
	if float(nobody_asked["meter"]["fraction"]) != 0.0 or nobody_asked["meter_text"] != "0 / 2".replace("2", "0"):
		_fail("a crew nobody asked for is an empty bar reading '0 / 0', got %s '%s'" % [str(nobody_asked["meter"]["fraction"]), nobody_asked["meter_text"]])

	# --- 2. the footer, pure -------------------------------------------------
	var snapshot := {
		"pay_period_ticks": 1000, "next_payday_tick": 2000, "last_bill": 14,
		"walked_out_last_payday": 0, "money": 500.0,
		"categories": [
			{"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2, "demand": 4, "assigned": 4, "short": 0},
			{"id": "maintenance", "name": "Maintenance", "headcount": 0, "wage": 2, "demand": 0, "assigned": 0, "short": 0},
			{"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 3, "demand": 2, "assigned": 2, "short": 0},
			{"id": "logistics", "name": "Logistics", "headcount": 1, "wage": 2, "demand": 1, "assigned": 1, "short": 0},
		],
	}
	if main._staff_bill(snapshot) != 4 * 2 + 2 * 3 + 1 * 2:
		_fail("the bill is every head at its category's wage, got %d" % main._staff_bill(snapshot))
	var footer: String = main._staff_footer_text(snapshot, 1700)
	if footer.find("wages 16 / period") < 0:
		_fail("the footer says 'wages 16 / period', got:\n%s" % footer)
	var expected_wait: String = main._format_remaining(300)
	if footer.find("next payday in %s" % expected_wait) < 0:
		_fail("the footer says 'next payday in %s' (300 ticks at TICK_HZ), got:\n%s" % [expected_wait, footer])
	if footer.find("unpaid") >= 0:
		_fail("nobody left unpaid: the footer must not mention it")
	if footer.find("H/ESC") < 0:
		_fail("the footer names the close keys")
	snapshot["walked_out_last_payday"] = 3
	var unpaid: String = main._staff_footer_text(snapshot, 1700)
	if unpaid.find("✗ 3 left unpaid last payday") < 0:
		_fail("three left unpaid reads '✗ 3 left unpaid last payday', got:\n%s" % unpaid)

	# --- 3. the reference line: rows match the wire ---------------------------
	Layouts.build_reference_line(main.sim)
	Layouts.stock_larder(main.sim, 64)
	main._rebuild_world()
	if main._staff_snapshot.is_empty():
		_fail("a rebuild polls the staff pools before the first frame")
	var live := _staff()
	var categories: Array = live.get("categories", [])
	if categories.size() < 2:
		_fail("the wire lists the staff.ron categories, got %d" % categories.size())
	main._set_staff_open(true)
	if main._staff_rows.size() != categories.size():
		_fail("%d categories on the wire, %d rows on the panel" % [categories.size(), main._staff_rows.size()])
	var fallback: Texture2D = main._fallback_icon_texture()
	for i in range(min(categories.size(), main._staff_rows.size())):
		var category: Dictionary = categories[i]
		var row: Dictionary = main._staff_rows[i]
		var id := String(category.get("id", ""))
		if String(row["id"]) != id:
			_fail("row %d is %s, the wire's %dth category is %s (wire order)" % [i, row["id"], i, id])
		if row["name_label"].text != String(category.get("name", "")):
			_fail("row %s is named '%s', the wire says '%s'" % [id, row["name_label"].text, category.get("name")])
		if row["hired_label"].text != "hired %d" % int(category.get("headcount", 0)):
			_fail("row %s reads '%s', the wire's headcount is %d" % [id, row["hired_label"].text, int(category.get("headcount", 0))])
		var icon: TextureRect = row["panel"].find_child("Icon", true, false)
		if icon == null or icon.texture == null or icon.texture == fallback:
			_fail("row %s must carry ticket 2's staff_%s icon, not the fallback" % [id, id])
		if not ResourceLoader.exists(main.ICON_DIR + main.STAFF_ICON_PREFIX + id + ".png"):
			_fail("no icon file for %s at %s" % [id, main.ICON_DIR + main.STAFF_ICON_PREFIX + id + ".png"])
		var state: Dictionary = main._staff_row_state(category)
		if state["note"] != "":
			if row["meter"].visible or not row["note_label"].visible or row["note_label"].text != state["note"]:
				_fail("row %s shows its note in place of the meter" % id)
		else:
			if not row["meter"].visible or row["note_label"].visible:
				_fail("row %s shows its meter" % id)
			if absf(float(row["meter"].get_meta("fraction")) - float(state["meter"]["fraction"])) > 0.001:
				_fail("row %s's meter is at %s, expected %s" % [id, str(row["meter"].get_meta("fraction")), str(state["meter"]["fraction"])])
			if row["meter_label"].text != state["meter_text"]:
				_fail("row %s's meter prints '%s', expected '%s'" % [id, row["meter_label"].text, state["meter_text"]])
		if row["fire_button"].disabled == bool(state["fire_enabled"]):
			_fail("row %s's FIRE button must be %s" % [id, "enabled" if state["fire_enabled"] else "disabled"])
		if row["hire_button"].disabled:
			_fail("HIRE is never disabled (it is free and never refused)")
	if main._staff_money_label.text != "Money: %.0f" % float(live.get("money", 0.0)):
		_fail("the panel's balance is the snapshot's money, got '%s'" % main._staff_money_label.text)
	if main._staff_footer_label.text != main._staff_footer_text(main._staff_snapshot, main.sim.current_tick()):
		_fail("the footer is drawn from the last poll and the current tick")
	if main._staff_panel.name != "StaffPanel":
		_fail("the panel node is named StaffPanel")
	main._set_staff_open(false)

	# --- 4. keys: H, one modal at a time, ESC, the legend --------------------
	if not String(main.ISO_HELP_TEXT).contains("H: staff") or not String(main.FP_HELP_TEXT).contains("H: staff"):
		_fail("both legends name H")
	main._set_market_open(true)
	_key(KEY_H)
	if not main._staff_open:
		_fail("H opens the staff panel")
	if main._market_open:
		_fail("opening the staff panel closes the market")
	if not main._staff_panel.visible:
		_fail("an open panel is visible")
	_key(KEY_M)
	if main._staff_open or not main._market_open:
		_fail("opening the market closes the staff panel")
	_key(KEY_M)
	_key(KEY_H)
	_key(KEY_C)
	if main._staff_open or not main._board_open:
		_fail("opening the contracts board closes the staff panel")
	_key(KEY_C)
	_key(KEY_H)
	_key(KEY_E)
	if main._staff_open or not main._shop_open:
		_fail("opening the build menu closes the staff panel")
	_key(KEY_E)
	_key(KEY_H)
	if not main._anything_open():
		_fail("an open staff panel counts as something for ESC to close")
	_key(KEY_ESCAPE)
	if main._staff_open:
		_fail("ESC closes the staff panel")
	if main._pause_open:
		_fail("ESC with the panel open closes the panel, not the game")
	if main._staff_panel.visible:
		_fail("a closed panel is hidden")
	_key(KEY_H)
	_key(KEY_H)
	if main._staff_open:
		_fail("H again closes the panel")
	# The panel is a modal like the board: the hover tooltip stays down.
	main._set_staff_open(true)
	main._config_open = false
	if not main._anything_open():
		_fail("_anything_open must include the staff panel")
	main._set_staff_open(false)

	# --- 5. HIRE and FIRE through the real command -------------------------------
	main._set_staff_open(true)
	var before := int(_category("manufacturing").get("headcount", 0))
	var row_index: int = main._staff_row_ids.find("manufacturing")
	if row_index < 0:
		_fail("no manufacturing row")
	main._staff_hire(row_index)
	if int(_category("manufacturing").get("headcount", 0)) != before + 1:
		_fail("HIRE adds one manufacturing head on the wire")
	if _row("manufacturing")["hired_label"].text != "hired %d" % (before + 1):
		_fail("the row follows a HIRE on the same call, reads '%s'" % _row("manufacturing")["hired_label"].text)
	main._staff_fire(row_index)
	if int(_category("manufacturing").get("headcount", 0)) != before:
		_fail("FIRE removes one manufacturing head on the wire")
	if _row("manufacturing")["hired_label"].text != "hired %d" % before:
		_fail("the row follows a FIRE on the same call")
	var maintenance_index: int = main._staff_row_ids.find("maintenance")
	if maintenance_index >= 0 and int(_category("maintenance").get("headcount", 0)) == 0:
		if not _row("maintenance")["fire_button"].disabled:
			_fail("FIRE is disabled on a category with nobody hired")
		main._staff_fire(maintenance_index)
		if int(_category("maintenance").get("headcount", 0)) != 0:
			_fail("FIRE at 0 is a no-op")
		main._staff_hire(maintenance_index)
		if int(_category("maintenance").get("headcount", 0)) != 1:
			_fail("HIRE on maintenance adds a head")
		if _row("maintenance")["fire_button"].disabled:
			_fail("FIRE is enabled once a head is hired")
		if not _row("maintenance")["meter"].visible or _row("maintenance")["note_label"].visible:
			_fail("a hired maintenance head replaces the idle note with the meter")
		main._staff_fire(maintenance_index)
	main._staff_fire(-1)
	main._staff_hire(99)
	main._set_staff_open(false)

	# --- 6. the stall reason and the crew line, pure --------------------------
	if main._stall_reason_text({"kind": "staff", "item": "manufacturing"}) != "short of manufacturing staff":
		_fail("a staff stall reads 'short of manufacturing staff', got '%s'" % main._stall_reason_text({"kind": "staff", "item": "manufacturing"}))
	if main._crew_text([{"category": "manufacturing", "count": 2}]) != "crew: 2 manufacturing":
		_fail("a crew reads 'crew: 2 manufacturing', got '%s'" % main._crew_text([{"category": "manufacturing", "count": 2}]))
	if main._crew_text([{"category": "manufacturing", "count": 1}, {"category": "packaging", "count": 1}]) != "crew: 1 manufacturing, 1 packaging":
		_fail("a two-category crew lists both")
	if main._crew_text([]) != "" or main._crew_text(null) != "":
		_fail("no crew is no line")
	if main._crew_categories_text([{"category": "logistics", "count": 1}]) != "logistics":
		_fail("the dock's category is named from its crew")

	# --- 7. the understaffed line ------------------------------------------------
	main.sim.new_game()
	Layouts.build_understaffed_line(main.sim)
	main._rebuild_world()
	var vat: Vector2i = Layouts.UNDERSTAFFED_VAT
	var vat_key: String = main._tile_key(vat)
	if Layouts.understaffed_stall(main.sim, vat) != Layouts.UNDERSTAFFED_CATEGORY:
		_fail("the fixture's vat must be refused its %s crew on the wire" % Layouts.UNDERSTAFFED_CATEGORY)
	if not main._machine_load.has(vat_key) or String(main._machine_load[vat_key].get("status", "")) != "starved":
		_fail("an understaffed machine reads starved (understaffed is a kind of starved)")
	var reason = main._machine_load.get(vat_key, {}).get("reason")
	if not (reason is Dictionary) or String(reason.get("kind", "")) != "staff":
		_fail("the vat's remembered reason is the staff stall, got %s" % str(reason))
	var hover: String = main._hover_text_at_tile(vat)
	var glyph := String(main.LOAD_STATUS_GLYPHS["starved"])
	for needle in ["%s starved" % glyph, "short of manufacturing staff", "crew: 2 manufacturing"]:
		if hover.find(needle) < 0:
			_fail("hover on the understaffed vat must say '%s', got:\n%s" % [needle, hover])
	if main._hover_text_at_tile(vat + Vector2i(1, 1)).find("short of manufacturing staff") < 0:
		_fail("any tile of the vat says it")
	var state: Dictionary = main._hover_panel_state(vat)
	var load_meter: Dictionary = state.get("load_meter", {})
	if load_meter.is_empty() or load_meter["color"] != main.METER_FILL_PROGRESS:
		_fail("the understaffed vat's load bar is the amber (starved) one")
	if load_meter.get("color") != main._load_status_color("starved"):
		_fail("the load bar colour is the starved status colour")
	# The other machines on the line are not short: the crusher and mixer
	# took the two heads left, so their reason is never staff.
	for key in ["4,0", "9,0"]:
		var other = main._machine_load.get(key, {}).get("reason")
		if other is Dictionary and String(other.get("kind", "")) == "staff":
			_fail("the %s was refused its crew; only the vat (last in stable order) should be" % key)
	# And the panel's row says the same thing the hover does.
	main._set_staff_open(true)
	var short_row := _row("manufacturing")
	var manufacturing := _category("manufacturing")
	if int(manufacturing.get("short", 0)) != 2 or int(manufacturing.get("headcount", 0)) != 2:
		_fail("the fixture's manufacturing pool reads headcount 2, short 2, got %s" % str(manufacturing))
	if short_row["short_label"].text != "2 short" or not short_row["short_label"].visible:
		_fail("the manufacturing row says '2 short', got '%s'" % short_row["short_label"].text)
	if short_row["meter_label"].text != "%d / %d" % [int(manufacturing.get("assigned", 0)), int(manufacturing.get("demand", 0))]:
		_fail("the manufacturing meter prints the wire's assigned / demand, got '%s'" % short_row["meter_label"].text)
	var fill: Panel = short_row["meter"].get_node("Fill")
	var fill_style: StyleBoxFlat = fill.get_theme_stylebox("panel")
	if fill_style.bg_color != main.METER_FILL_WARN:
		_fail("the short row's bar is drawn in the warning colour")
	main._set_staff_open(false)
	# Hiring the two back un-starves it: the panel is the fix as well as the
	# readout. (Two commands, then a tick for the claim to run.)
	main._staff_hire(row_index)
	main._staff_hire(row_index)
	for i in range(int(main.LOAD_POLL_TICKS) * 2):
		_step()
	if Layouts.understaffed_stall(main.sim, vat) != "":
		_fail("with its crew hired back the vat is no longer refused")
	if _category("manufacturing").get("short", 0) != 0:
		_fail("nobody is short once the crew is hired back")

	# --- 8. a dock refused its logistics head --------------------------------------
	# A short line straight into a dock: a crusher's paste reaches it in a
	# few hundred ticks. (`build_machines_fixture` stands its dock off the
	# crusher's outlet, so nothing ever arrives there.)
	main.sim.new_game()
	Layouts.place(main.sim, "hopper", Vector2i(-1, 0), "east", "tomato")
	Layouts.place(main.sim, "belt", Vector2i(0, 0), "east")
	Layouts.place(main.sim, "crusher", Vector2i(1, 0), "east")
	Layouts.place(main.sim, "belt", Vector2i(2, 0), "east")
	Layouts.place(main.sim, "belt", Vector2i(3, 0), "east")
	var dock := Vector2i(4, 0)
	Layouts.place(main.sim, "dock", dock, "east")
	Layouts.stock_larder(main.sim, 64)
	main.sim.issue_command_json(JSON.stringify({"type": "set_hopper_auto_fill", "tile": [-1, 0], "enabled": true}))
	main._rebuild_world()
	var staffed_hover: String = main._hover_text_at_tile(dock)
	if staffed_hover.find("crew: 1 logistics") < 0:
		_fail("a dock's hover names its crew, got:\n%s" % staffed_hover)
	if staffed_hover.find("held") >= 0:
		_fail("a staffed dock is not held")
	main.sim.issue_command_json(JSON.stringify({"type": "fire", "category": "logistics"}))
	var refused := false
	for i in range(1500):
		main.sim.tick()
		var snapshot_dock = JSON.parse_string(main.sim.query_tile_json(dock.x, dock.y))
		if snapshot_dock is Dictionary and bool(snapshot_dock.get("understaffed", false)):
			refused = true
			break
	if not refused:
		_fail("a dock with no logistics head and something to ship reads understaffed on the wire")
	main._rebuild_world()
	var held_hover: String = main._hover_text_at_tile(dock)
	for needle in ["crew: 1 logistics", "held  ·  short of logistics staff"]:
		if held_hover.find(needle) < 0:
			_fail("hover on the understaffed dock must say '%s', got:\n%s" % [needle, held_hover])
	if main._hover_text_at_tile(dock + Vector2i(1, 1)).find("held") < 0:
		_fail("any tile of the dock says it")

	# --- 9. the build-menu card ----------------------------------------------------
	main._set_shop_menu_open(true)
	var saw_machine := false
	var saw_belt := false
	var shop_entries: Array = JSON.parse_string(main.sim.shop_snapshot_json()).get("entries", [])
	# The first slot per entry: `_ready()` runs twice under this harness and
	# the second set of slots is never driven (test_shop_menu.gd's caveat).
	for slot in main._shop_slots.slice(0, shop_entries.size()):
		var label: Label = slot["crew_label"]
		if slot["kind"] == "crusher":
			saw_machine = true
			if label.text != "crew: 1 manufacturing":
				_fail("the crusher's card says 'crew: 1 manufacturing' under the price, got '%s'" % label.text)
		if slot["kind"] == "belt":
			saw_belt = true
			if label.text != "":
				_fail("a belt's card shows no crew line")
		if slot["kind"] == "cooking_vat" and label.text != "crew: 2 manufacturing":
			_fail("the vat's card says 'crew: 2 manufacturing', got '%s'" % label.text)
		if slot["kind"] == "dock" and label.text != "crew: 1 logistics":
			_fail("the dock's card says 'crew: 1 logistics', got '%s'" % label.text)
	if not saw_machine or not saw_belt:
		_fail("the build menu lists a crusher and a belt")
	main._set_shop_menu_open(false)

	# --- 10. polling on the tick; pause freezes it; a rebuild reads first -----------
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	main._rebuild_world()
	if main._staff_poll_due():
		_fail("a rebuild polls; nothing is due straight after it")
	var polled_at: int = main._staff_polled_tick
	main._pause_open = true
	for i in range(int(main.LOAD_POLL_TICKS) * 3):
		_step()
	main._pause_open = false
	if main._staff_polled_tick != polled_at:
		_fail("the staff pools were polled with the pause menu up")
	# Move the sim underneath (as a test may) and hire behind the panel's
	# back: the snapshot must not follow until a tick polls it.
	main.sim.issue_command_json(JSON.stringify({"type": "hire", "category": "packaging"}))
	var shown := int(main._staff_snapshot["categories"][2]["headcount"])
	if shown != int(_category("packaging").get("headcount", 0)) - 1:
		_fail("the snapshot is the last poll, not the live wire, between polls")
	for i in range(int(main.LOAD_POLL_TICKS)):
		_step()
	if main._staff_polled_tick == polled_at:
		_fail("LOAD_POLL_TICKS of simulation polls the staff pools")
	if int(main._staff_snapshot["categories"][2]["headcount"]) != int(_category("packaging").get("headcount", 0)):
		_fail("after a poll the snapshot follows the wire")
	var after_poll: int = main._staff_polled_tick
	for i in range(int(main.LOAD_POLL_TICKS) - 1):
		_step()
	if main._staff_polled_tick != after_poll:
		_fail("polled again before LOAD_POLL_TICKS")
	_step()
	if main._staff_polled_tick == after_poll:
		_fail("LOAD_POLL_TICKS after a poll, the next one is due")
	# A save loaded back reads its crew on the first frame, before any tick.
	var save_text: String = main.sim.save_ron()
	main.sim.new_game()
	main._rebuild_world()
	if int(main._staff_snapshot["categories"][2]["headcount"]) != int(_category("packaging").get("headcount", 0)):
		_fail("a new game's snapshot is the new game's")
	var refusal: String = main.sim.load_ron(save_text)
	if refusal != "":
		_fail("load_ron refused the save: %s" % refusal)
	main._rebuild_world()
	if int(main._staff_snapshot["categories"][2]["headcount"]) != int(_category("packaging").get("headcount", 0)):
		_fail("after a load, before any tick, the panel reads the saved crew")

	if _failed:
		quit(1)
		return
	print("STAFF_PANEL_OK: the staff panel (H) reads staff_snapshot_json row for row with ticket 2's icons, HIRE/FIRE move one head through the real commands and the row follows at once, the understaffed line's vat reads starved with 'short of manufacturing staff' and its crew in hover over an amber load bar, a dock without its logistics head reads held, the build-menu card shows a crew, one modal at a time with E/M/C, ESC closes, polling is on the tick and frozen by pause")
	quit(0)
