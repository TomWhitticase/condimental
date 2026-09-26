# Headless proof of the production-stats epic's Godot half (Factorio's P
# screen): the PRODUCTION panel on P, its rows read straight off
# `production_snapshot_json`, the window selector, the rates, and the modal
# rules it shares with E/M/C/H. Godot sorts and formats; every count on the
# panel is checked back against the wire, never derived here.
#
# Run with:
#   godot --headless --path godot_project --script test_production_stats.gd
#
# Verifies:
#   * the rows are a pure function of a snapshot: one per id with anything
#     made or used in the window, biggest first, each count as a share of
#     the largest on the list, rates per minute over what was OBSERVED (a
#     young window divides by its age, not its length), all-time totals
#     with no rate, and nothing for a window nothing has happened in,
#   * the window caption says `last 1m`, `(Ns so far)` while young, and
#     `all time`,
#   * a running line (the bottleneck fixture) counts: every stage's product
#     is made and used, fluids are listed as fluids, the boxed product is
#     USED only because a contract paid, raw ingredients are used and never
#     made, and buying stock or filling a hopper counts nothing,
#   * the panel's rows match the wire in sorted order with the item's own
#     icon, the selected window button is styled as selected, switching
#     windows re-reads, and an empty world says so in a sentence,
#   * P opens it and closes the other modals; each of E/M/C/H closes it;
#     ESC closes it without the pause menu; both legends name P,
#   * the pay-cycle block (ticket 7) prints the wire's `cycle` lines in its
#     order as REQUIRED / THIS CYCLE / LAST CYCLE, a fraction to one place,
#     `—` before the first payday, the short verdict in words and the warn
#     colour, and a sentence in place of the rows with no contract running,
#   * the ledger is polled on the tick only while open, the pause menu
#     freezes it, opening re-reads at once, and a save loaded back reads
#     its counts on the first frame.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false
var _failed := false


func _fail(msg: String) -> void:
	_failed = true
	printerr("PRODUCTION_STATS_FAIL: %s" % msg)


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


func _wire() -> Dictionary:
	return JSON.parse_string(main.sim.production_snapshot_json())


func _wire_item(id: String) -> Dictionary:
	for item in _wire().get("items", []):
		if String(item.get("id", "")) == id:
			return item
	return {}


func _row_by_id(rows: Array, id: String) -> Dictionary:
	for row in rows:
		if String(row["id"]) == id:
			return row
	return {}


func _run_checks() -> void:
	# --- 1. the rows, pure ---------------------------------------------------
	var minute_ticks := int(main.TICK_HZ) * 60
	var snapshot := {
		"windows": [
			{"id": "5s", "ticks": 150, "observed": 150},
			{"id": "1m", "ticks": minute_ticks, "observed": minute_ticks},
			{"id": "10m", "ticks": minute_ticks * 10, "observed": 900},
		],
		"items": [
			{"id": "tomato", "kind": "item", "produced": [0, 0, 0], "consumed": [8, 96, 96],
				"produced_total": 0, "consumed_total": 500},
			{"id": "tomato_paste", "kind": "item", "produced": [2, 24, 24], "consumed": [2, 24, 24],
				"produced_total": 120, "consumed_total": 118},
			{"id": "ketchup_mix", "kind": "fluid", "produced": [0, 12, 12], "consumed": [0, 12, 12],
				"produced_total": 60, "consumed_total": 60},
			{"id": "boxed_ketchup", "kind": "item", "produced": [0, 0, 3], "consumed": [0, 0, 2],
				"produced_total": 9, "consumed_total": 4},
		],
	}
	var minute: Array = main._production_rows_for(snapshot, 1)
	if minute.size() != 3:
		_fail("the minute window lists the three ids with anything in it, got %d" % minute.size())
	if minute.size() >= 3 and (String(minute[0]["id"]) != "tomato" or String(minute[1]["id"]) != "tomato_paste" or String(minute[2]["id"]) != "ketchup_mix"):
		_fail("rows are biggest first (made + used): tomato 96, paste 48, mix 24 — got %s" % str(minute.map(func(r): return r["id"])))
	var tomato := _row_by_id(minute, "tomato")
	if tomato.is_empty() or int(tomato["made"]) != 0 or int(tomato["used"]) != 96:
		_fail("tomato over the minute is made 0 / used 96, got %s" % str(tomato))
	if tomato.get("used_rate", "") != "96.0/min":
		_fail("96 over a full minute is '96.0/min', got '%s'" % tomato.get("used_rate", ""))
	if tomato.get("made_rate", "") != "0.0/min":
		_fail("nothing made over a minute still reads '0.0/min', got '%s'" % tomato.get("made_rate", ""))
	if absf(float(tomato["used_fraction"]) - 1.0) > 0.001:
		_fail("the largest count on the list is a full bar, got %s" % str(tomato["used_fraction"]))
	var paste := _row_by_id(minute, "tomato_paste")
	if absf(float(paste["made_fraction"]) - 0.25) > 0.001:
		_fail("24 against a largest of 96 is a quarter bar, got %s" % str(paste["made_fraction"]))
	if String(paste["name"]) != main._item_display_name("tomato_paste"):
		_fail("a row's name goes through _item_display_name")
	if String(_row_by_id(minute, "ketchup_mix")["kind"]) != "fluid":
		_fail("a fluid row carries its kind from the wire")
	# A young window: 900 of 18 000 ticks observed is 30 seconds, so 96
	# over it is 192/min, not 9.6/min.
	var young := _row_by_id(main._production_rows_for(snapshot, 2), "tomato")
	if young.get("used_rate", "") != "192.0/min":
		_fail("a rate divides by the OBSERVED span (30s), expected '192.0/min', got '%s'" % young.get("used_rate", ""))
	if main._production_rows_for(snapshot, 2).size() != 4:
		_fail("the ten-minute window lists the boxed product too, got %d rows" % main._production_rows_for(snapshot, 2).size())
	# All time: totals, no rate, boxed ketchup made 9 used 4.
	var all_time: Array = main._production_rows_for(snapshot, main.PRODUCTION_ALL_TIME)
	var boxed := _row_by_id(all_time, "boxed_ketchup")
	if boxed.is_empty() or int(boxed["made"]) != 9 or int(boxed["used"]) != 4:
		_fail("all time reads the lifetime totals, got %s" % str(boxed))
	if boxed.get("made_rate", "x") != "" or boxed.get("used_rate", "x") != "":
		_fail("all time has no rate")
	if String(all_time[0]["id"]) != "tomato":
		_fail("all time sorts by lifetime totals (tomato 500 first), got %s" % all_time[0]["id"])
	# The five-second window drops the ids with nothing in it.
	var five: Array = main._production_rows_for(snapshot, 0)
	if five.size() != 2 or not _row_by_id(five, "ketchup_mix").is_empty():
		_fail("a window with nothing for an id leaves the id out, got %s" % str(five.map(func(r): return r["id"])))
	if not main._production_rows_for(snapshot, 7).is_empty() or not main._production_rows_for({}, 0).is_empty():
		_fail("an unknown window, or no snapshot, is no rows")
	# Rates and captions, pure.
	if main._production_rate_text(30, minute_ticks) != "30.0/min":
		_fail("30 over a minute is '30.0/min', got '%s'" % main._production_rate_text(30, minute_ticks))
	if main._production_rate_text(5, 0) != "":
		_fail("nothing observed is no rate")
	if main._production_count_text(96, "96.0/min") != "96  ·  96.0/min" or main._production_count_text(9, "") != "9":
		_fail("a count prints with its rate after it, or alone")
	if main._production_window_text(snapshot, 1) != "last 1m":
		_fail("a full window's caption is 'last 1m', got '%s'" % main._production_window_text(snapshot, 1))
	var so_far: String = main._production_window_text(snapshot, 2)
	if so_far.find("last 10m") < 0 or so_far.find("(%s so far)" % main._format_remaining(900)) < 0:
		_fail("a young window's caption says how much has been seen, got '%s'" % so_far)
	if main._production_window_text(snapshot, main.PRODUCTION_ALL_TIME) != "all time":
		_fail("the lifetime caption is 'all time'")

	# --- 2. a running line counts ----------------------------------------------
	# Buying stock and filling bins first: none of that is production.
	Layouts.build_reference_line(main.sim)
	Layouts.stock_larder(main.sim, 64)
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "tile": [-1, 0]}))
	main._rebuild_world()
	if not _wire().get("items", []).is_empty():
		_fail("buying stock and filling a hopper count nothing, the wire lists %s" % str(_wire().get("items")))
	main._set_production_open(true)
	if not main._production_empty_label.visible or main._production_scroll.visible:
		_fail("an empty ledger shows its sentence in place of the list")
	if main._production_cycle_rows_for(main._production_snapshot).is_empty() != main._production_cycle_empty_label.visible:
		_fail("the no-orders sentence shows exactly when the wire lists no cycle line")
	if main._production_empty_label.text.find("nothing made or used") < 0:
		_fail("the empty sentence says nothing was made or used, got '%s'" % main._production_empty_label.text)
	main._set_production_open(false)
	main.sim.new_game()
	Layouts.build_bottleneck_line(main.sim)
	main._rebuild_world()
	var wire := _wire()
	var windows: Array = wire.get("windows", [])
	if windows.size() < 2:
		_fail("the wire carries the sim's windows, got %d" % windows.size())
	if String(windows[0].get("id", "")) != "5s" or int(windows[0].get("ticks", 0)) != 150:
		_fail("the first window is the five-second one, got %s" % str(windows[0]))
	var expected_kinds := {
		"tomato": "item", "sugar": "item", "vinegar": "fluid", "tomato_paste": "item",
		"ketchup_mix": "fluid", "ketchup": "fluid", "bottled_ketchup": "item", "boxed_ketchup": "item",
	}
	for id in expected_kinds.keys():
		var item := _wire_item(id)
		if item.is_empty():
			_fail("the settled line counted nothing for %s" % id)
			continue
		if String(item.get("kind", "")) != expected_kinds[id]:
			_fail("%s is a %s on the wire, expected %s" % [id, item.get("kind"), expected_kinds[id]])
	for raw in ["tomato", "sugar", "vinegar"]:
		var item := _wire_item(raw)
		if int(item.get("produced_total", -1)) != 0 or int(item.get("consumed_total", 0)) <= 0:
			_fail("a raw ingredient is used and never made, %s reads %s" % [raw, str(item)])
	for mid in ["tomato_paste", "ketchup_mix", "ketchup", "bottled_ketchup"]:
		var item := _wire_item(mid)
		if int(item.get("produced_total", 0)) <= 0 or int(item.get("consumed_total", 0)) <= 0:
			_fail("an intermediate is both made and used, %s reads %s" % [mid, str(item)])
	var boxed_wire := _wire_item("boxed_ketchup")
	if int(boxed_wire.get("produced_total", 0)) <= 0:
		_fail("the line packed boxes")
	var paid := 0
	for contract in JSON.parse_string(main.sim.economy_snapshot_json()).get("contracts", []):
		paid += int(contract.get("deliveries_made", 0))
	if paid <= 0:
		_fail("the settled line must have paid a contract for the exit rule to be tested")
	if int(boxed_wire.get("consumed_total", 0)) <= 0:
		_fail("a paid contract USES the boxes it ships; boxed_ketchup consumed_total is 0")
	if int(boxed_wire.get("consumed_total", 0)) > int(boxed_wire.get("produced_total", 0)):
		_fail("nothing is shipped that was not made")
	# The tomato count is the crusher's recipe quantity per craft, so it is
	# a multiple of four — the ledger records the recipe's own numbers.
	if int(_wire_item("tomato").get("consumed_total", 1)) % 4 != 0:
		_fail("tomato is used four a craft, total %d is not a multiple of four" % int(_wire_item("tomato").get("consumed_total", 0)))

	# --- 2b. the pay-cycle rows, pure ------------------------------------------
	var cycle_snapshot := {"cycle": {"period_ticks": 1000, "next_tick": 3000, "cycles_closed": 2, "lines": [
		{"id": "boxed_mustard", "required": 6.666656494140625, "produced_this_cycle": 3, "produced_last_cycle": 5, "short": true},
		{"id": "boxed_ketchup", "required": 2.0, "produced_this_cycle": 1, "produced_last_cycle": 4, "short": false},
	]}}
	var cycle_rows: Array = main._production_cycle_rows_for(cycle_snapshot)
	if cycle_rows.size() != 2 or String(cycle_rows[0]["id"]) != "boxed_mustard":
		_fail("the cycle rows are the wire's lines in the wire's order, got %s" % str(cycle_rows.map(func(r): return r["id"])))
	else:
		if cycle_rows[0]["required"] != "6.7" or cycle_rows[1]["required"] != "2":
			_fail("required prints a fraction to one place and a whole number whole, got '%s' / '%s'" % [cycle_rows[0]["required"], cycle_rows[1]["required"]])
		if cycle_rows[0]["this_cycle"] != "3" or cycle_rows[1]["last_cycle"] != "4":
			_fail("this and last cycle print the wire's counts, got %s" % str(cycle_rows))
		if not bool(cycle_rows[0]["short"]) or String(cycle_rows[0]["last_cycle"]).find("short") < 0:
			_fail("a short line says so in words, got '%s'" % cycle_rows[0]["last_cycle"])
		if bool(cycle_rows[1]["short"]) or String(cycle_rows[1]["last_cycle"]).find("short") >= 0:
			_fail("a covered line is not short")
	if main._production_required_text(10.99998474121093) != "11":
		_fail("a fixed-point whole number prints whole, got '%s'" % main._production_required_text(10.99998474121093))
	cycle_snapshot["cycle"]["cycles_closed"] = 0
	if String(main._production_cycle_rows_for(cycle_snapshot)[1]["last_cycle"]) != "—":
		_fail("before the first payday there is no last cycle to print")
	if not main._production_cycle_rows_for({"cycle": {"lines": []}}).is_empty() or not main._production_cycle_rows_for({}).is_empty():
		_fail("no contract, or no snapshot, is no cycle rows")

	# --- 3. the panel reads the wire ---------------------------------------------
	main._set_production_open(true)
	if main._production_snapshot.is_empty():
		_fail("opening the panel polls the ledger")
	if main._production_window != 1:
		_fail("the panel opens on the minute window, got %d" % main._production_window)
	var rows: Array = main._production_rows_for(main._production_snapshot, main._production_window)
	if rows.is_empty():
		_fail("the settled line has rows over the last minute")
	if main._production_rows.size() != rows.size():
		_fail("%d rows on the wire, %d on the panel" % [rows.size(), main._production_rows.size()])
	var fallback: Texture2D = main._fallback_icon_texture()
	for i in range(min(rows.size(), main._production_rows.size())):
		var state: Dictionary = rows[i]
		var row: Dictionary = main._production_rows[i]
		if String(row["id"]) != String(state["id"]):
			_fail("row %d is %s, the sorted wire's is %s" % [i, row["id"], state["id"]])
		if row["name_label"].text != String(state["name"]):
			_fail("row %s is named '%s', expected '%s'" % [state["id"], row["name_label"].text, state["name"]])
		if row["made_label"].text != main._production_count_text(int(state["made"]), String(state["made_rate"])):
			_fail("row %s's MADE prints '%s', expected '%s'" % [state["id"], row["made_label"].text, main._production_count_text(int(state["made"]), String(state["made_rate"]))])
		if row["used_label"].text != main._production_count_text(int(state["used"]), String(state["used_rate"])):
			_fail("row %s's USED prints the wire's count and rate" % state["id"])
		if absf(float(row["made_meter"].get_meta("fraction")) - float(state["made_fraction"])) > 0.001:
			_fail("row %s's MADE bar is at %s, expected %s" % [state["id"], str(row["made_meter"].get_meta("fraction")), str(state["made_fraction"])])
		if absf(float(row["used_meter"].get_meta("fraction")) - float(state["used_fraction"])) > 0.001:
			_fail("row %s's USED bar follows the wire" % state["id"])
		var icon: TextureRect = row["panel"].find_child("Icon", true, false)
		if icon == null or icon.texture == null or icon.texture == fallback:
			_fail("row %s carries the item's own icon, not the fallback" % state["id"])
	if main._production_empty_label.visible or not main._production_scroll.visible:
		_fail("a counted line shows the list, not the empty sentence")
	if main._production_window_label.text != main._production_window_text(main._production_snapshot, 1):
		_fail("the caption under the list is the selected window's")
	if main._production_footer_label.text.find("P/ESC") < 0:
		_fail("the footer names the close keys")
	if main._production_panel.name != "ProductionPanel":
		_fail("the panel node is named ProductionPanel")
	# The pay-cycle block: one row per line the wire's `cycle` carries,
	# each figure printed from it. The settled line has run past a payday.
	var cycle: Dictionary = main._production_snapshot.get("cycle", {})
	var lines: Array = cycle.get("lines", [])
	if int(cycle.get("period_ticks", 0)) <= 0 or int(cycle.get("cycles_closed", 0)) < 1:
		_fail("the settled line's wire carries a pay cycle past its first payday, got %s" % str(cycle))
	if lines.is_empty() or String(lines[0].get("id", "")) != "boxed_ketchup":
		_fail("the running contract orders boxed ketchup, so the cycle lists it, got %s" % str(lines))
	if main._production_cycle_rows.size() != lines.size():
		_fail("%d cycle lines on the wire, %d on the panel" % [lines.size(), main._production_cycle_rows.size()])
	for i in range(min(lines.size(), main._production_cycle_rows.size())):
		var line: Dictionary = lines[i]
		var row: Dictionary = main._production_cycle_rows[i]
		if row["required"].text != main._production_required_text(float(line.get("required", 0.0))):
			_fail("%s REQUIRED prints '%s', the wire says %s" % [line["id"], row["required"].text, str(line.get("required"))])
		if row["this_cycle"].text != str(int(line.get("produced_this_cycle", -1))):
			_fail("%s THIS CYCLE prints '%s', the wire says %s" % [line["id"], row["this_cycle"].text, str(line.get("produced_this_cycle"))])
		if not row["last_cycle"].text.begins_with(str(int(line.get("produced_last_cycle", -1)))):
			_fail("%s LAST CYCLE prints '%s', the wire says %s" % [line["id"], row["last_cycle"].text, str(line.get("produced_last_cycle"))])
		var warned: bool = row["last_cycle"].label_settings.font_color == main.METER_FILL_WARN
		if warned != bool(line.get("short", false)):
			_fail("%s's last-cycle figure is %scoloured as short, the wire says short=%s" % [line["id"], "" if warned else "not ", str(line.get("short"))])
	# The settled line is never short, so the check above only ever compares
	# false with false. Drive the live block through a short line and back:
	# the warn colour AND the staff panel's short border go on, then come off.
	if not lines.is_empty():
		var real_snapshot: Dictionary = main._production_snapshot
		for short in [true, false]:
			var forced: Dictionary = real_snapshot.duplicate(true)
			forced["cycle"]["lines"][0]["short"] = short
			main._production_snapshot = forced
			main._update_production_cycle_block()
			var row: Dictionary = main._production_cycle_rows[0]
			var colour: Color = row["last_cycle"].label_settings.font_color
			if (colour == main.METER_FILL_WARN) != short:
				_fail("a %s line's LAST CYCLE is coloured %s" % ["short" if short else "covered", str(colour)])
			if row["panel"].get_theme_stylebox("panel") != main._staff_row_style(short):
				_fail("a %s line's row does not wear the staff panel's %s border" % ["short" if short else "covered", "short" if short else "plain"])
		main._production_snapshot = real_snapshot
		main._update_production_cycle_block()
	if main._production_cycle_empty_label.visible:
		_fail("a running contract shows its rows, not the no-orders sentence")
	if main._production_cycle_caption.text.find("PAY CYCLE") < 0 or main._production_cycle_caption.text.find("payday in") < 0:
		_fail("the cycle caption names the pay cycle and its countdown, got '%s'" % main._production_cycle_caption.text)
	# The selector: five buttons (four windows + all time), one selected.
	if main._production_window_buttons.size() != windows.size() + 1:
		_fail("one button per window plus all time, got %d" % main._production_window_buttons.size())
	var selected_style: StyleBoxFlat = main._hotbar_slot_style("selected")
	for entry in main._production_window_buttons:
		var button: Button = entry["button"]
		var style: StyleBoxFlat = button.get_theme_stylebox("normal")
		var is_selected: bool = int(entry["index"]) == main._production_window
		if (style.border_color == selected_style.border_color) != is_selected:
			_fail("window button %s is %sstyled as selected" % [button.name, "" if is_selected else "not "])
	# Switching windows: all time shows totals, and the caption follows.
	main._set_production_window(main.PRODUCTION_ALL_TIME)
	if main._production_window != main.PRODUCTION_ALL_TIME or main._production_window_label.text != "all time":
		_fail("selecting all time redraws the caption, got '%s'" % main._production_window_label.text)
	var all_rows: Array = main._production_rows_for(main._production_snapshot, main.PRODUCTION_ALL_TIME)
	var boxed_row: Dictionary = _row_by_id(all_rows, "boxed_ketchup")
	if boxed_row.is_empty() or int(boxed_row["made"]) != int(boxed_wire.get("produced_total", -1)):
		_fail("all time shows the wire's lifetime totals")
	var panel_boxed: Dictionary = {}
	for row in main._production_rows:
		if String(row["id"]) == "boxed_ketchup":
			panel_boxed = row
	if panel_boxed.is_empty() or panel_boxed["made_label"].text != str(int(boxed_wire.get("produced_total", -1))):
		_fail("the all-time row prints the total alone, got '%s'" % (panel_boxed["made_label"].text if not panel_boxed.is_empty() else "no row"))
	main._set_production_window(99)
	if main._production_window != main.PRODUCTION_ALL_TIME:
		_fail("an unknown window index is refused")
	main._set_production_window(0)
	if main._production_window != 0:
		_fail("the five-second window can be selected")
	main._set_production_open(false)

	# --- 4. keys: P, one modal at a time, ESC, the legend ----------------------
	if not String(main.ISO_HELP_TEXT).contains("P: production") or not String(main.FP_HELP_TEXT).contains("P: production"):
		_fail("both legends name P")
	main._set_market_open(true)
	_key(KEY_P)
	if not main._production_open or not main._production_panel.visible:
		_fail("P opens the production panel")
	if main._market_open:
		_fail("opening the production panel closes the market")
	for pair in [[KEY_M, "_market_open"], [KEY_C, "_board_open"], [KEY_E, "_shop_open"], [KEY_H, "_staff_open"]]:
		_key(pair[0])
		if main._production_open or not main.get(pair[1]):
			_fail("opening another modal (%s) closes the production panel" % pair[1])
		_key(pair[0])
		_key(KEY_P)
	if not main._anything_open():
		_fail("an open production panel counts as something for ESC to close")
	_key(KEY_ESCAPE)
	if main._production_open:
		_fail("ESC closes the production panel")
	if main._pause_open:
		_fail("ESC with the panel open closes the panel, not the game")
	_key(KEY_P)
	_key(KEY_P)
	if main._production_open:
		_fail("P again closes the panel")
	main._set_production_open(true)
	if not main._anything_open():
		_fail("_anything_open must include the production panel")
	main._set_production_open(false)

	# --- 5. polling: only while open, on the tick, frozen by pause -------------
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	Layouts.stock_larder(main.sim, 200)
	main.sim.issue_command_json(JSON.stringify({"type": "set_hopper_auto_fill", "tile": [-1, 0], "enabled": true}))
	main._rebuild_world()
	var closed_at: int = main._production_polled_tick
	for i in range(int(main.LOAD_POLL_TICKS) * 2):
		_step()
	if main._production_polled_tick != closed_at:
		_fail("a closed panel does not poll the ledger")
	main._set_production_open(true)
	var opened_at: int = main._production_polled_tick
	if opened_at != main.sim.current_tick():
		_fail("opening re-reads the ledger at once")
	# Run until the crusher has crafted, so the ledger has something to say
	# the panel does not yet know.
	for i in range(400):
		main.sim.tick()
	if _wire_item("tomato").is_empty():
		_fail("a fed crusher uses tomatoes within 400 ticks")
	main._pause_open = true
	for i in range(int(main.LOAD_POLL_TICKS) * 3):
		_step()
	main._pause_open = false
	if main._production_polled_tick != opened_at:
		_fail("the ledger was polled with the pause menu up")
	if not main._production_snapshot.get("items", []).is_empty():
		_fail("the snapshot is the last poll, not the live wire, while paused")
	_step()
	if main._production_polled_tick == opened_at:
		_fail("running again, the next step polls")
	if main._production_snapshot.get("items", []).is_empty():
		_fail("after a poll the snapshot follows the wire")
	var after_poll: int = main._production_polled_tick
	for i in range(int(main.LOAD_POLL_TICKS) - 1):
		_step()
	if main._production_polled_tick != after_poll:
		_fail("polled again before LOAD_POLL_TICKS")
	_step()
	if main._production_polled_tick == after_poll:
		_fail("LOAD_POLL_TICKS after a poll, the next one is due")
	main._set_production_open(false)
	# A save loaded back reads its counts on the first frame, before any tick.
	var total_before := int(_wire_item("tomato").get("consumed_total", 0))
	var save_text: String = main.sim.save_ron()
	main.sim.new_game()
	main._rebuild_world()
	var refusal: String = main.sim.load_ron(save_text)
	if refusal != "":
		_fail("load_ron refused the save: %s" % refusal)
	main._rebuild_world()
	main._set_production_open(true)
	if int(_wire_item("tomato").get("consumed_total", -1)) != total_before:
		_fail("a loaded ledger carries its totals")
	var loaded_rows: Array = main._production_rows_for(main._production_snapshot, main.PRODUCTION_ALL_TIME)
	if _row_by_id(loaded_rows, "tomato").is_empty() or int(_row_by_id(loaded_rows, "tomato")["used"]) != total_before:
		_fail("after a load, before any tick, the panel reads the saved counts")
	main._set_production_open(false)

	if _failed:
		quit(1)
		return
	print("PRODUCTION_STATS_OK: the production panel (P) sorts and formats production_snapshot_json — rows biggest first with per-minute rates over the observed span and bars against the largest count, a window selector with all time, the running line counting every stage with fluids as fluids and a paid order as the exit, nothing counted for buying or filling, the pay-cycle block's REQUIRED / THIS CYCLE / LAST CYCLE off the wire, one modal at a time with E/M/C/H, ESC closes, polled on the tick only while open and frozen by pause, and a loaded save reads its counts first frame")
	quit(0)
