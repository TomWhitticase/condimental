# Headless proof of the staff pools MVP's Godot half (ticket 3;
# `docs/STAFF_POOLS_PLAN.md`, "Presentation") and the quality pass's ticket 4
# (staffing legibility): the STAFF panel on H, its polling, HIRE/FIRE through
# the real commands, the hover lines an understaffed machine and dock gain,
# the WHY line under each row, the always-visible wage bill, the BEFORE
# warning when the balance cannot cover the next bill, and the AFTER notice
# a payday that idled or lost heads fires. Godot derives nothing here —
# every figure on the panel is read back against `staff_snapshot_json`.
#
# Run with:
#   godot --headless --path godot_project --script test_staff_panel.gd
#
# Verifies:
#   * a row's words and bar are pure functions of the wire's category
#     (`hired N`, `assigned / demand` printed on the meter, `N short` in
#     the warning colour, FIRE disabled at 0, the engineering idle note),
#   * HIRE is gated on the establishment alone and the wire's own
#     `hire_warns` only warns beside it (2026-09-26 — it used to refuse),
#     and a row's `status_text` names heads idled or lost by a payday,
#   * the WHY line under a row (`_staff_demand_sources_for`/
#     `_staff_demand_text`): kind ×count per placement asking for a
#     category, lowercase space-separated kind names, and the empty
#     factory's `'nothing placed asks for staff yet'`,
#   * every category in `STAFF_CATEGORY_BLURBS` has a sentence,
#   * the footer says the bill (the wire's `next_bill`), the payday
#     countdown in seconds from `current_tick()`, the BEFORE warning
#     (`_staff_shortfall_text`, singular and plural) when the balance
#     cannot cover it, `balance after payday: N` when it can, and the
#     `idled`/`left` unpaid lines only when they happened,
#   * `_staff_unpaid_count_if_payday_now` replays the payday cascade
#     against wire numbers, a zero-wage head never counted unpaid,
#   * the clipboard's always-visible wages line (`_contract_wages_state`)
#     matches the footer's bill and shortfall,
#   * the AFTER-payday notice (`_staff_payday_notice_text`,
#     `_announce_staff_payday`) names category and count, primes on the
#     first poll without announcing, and fires through the same
#     `_show_contract_notice` a lost contract uses only once
#     `next_payday_tick` has actually advanced,
#   * on the reference line the panel's rows match the snapshot in wire
#     order, with ticket 2's icon (not the fallback) on each,
#   * the underfunded fixture (`build_underfunded_line`) is short by
#     exactly 6, leaves exactly 3 heads unpaid, disables every row's HIRE,
#     carries the ticket's own wording on the footer, and announces the
#     categories a payday it actually reaches idles,
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
#   * the build-menu card shows a machine's crew and its cost per period
#     under the price, and nothing for a belt,
#   * the panel polls on the tick at LOAD_POLL_TICKS, the pause menu
#     freezes it, and a rebuild reads the wire before the first frame,
#   * all four category rows fit the panel's scroll at 1600x900 (issue #7),
#     each showing exactly one of its five sentences with the rest on the
#     row's hover, the bar captioned on its own line, and the shortfall and
#     the working figure never sharing the slot after the bar.
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
	# Every check below reads the panel at the size UI_UX.md §5c designs the
	# wide case for. It is set HERE rather than at the one check that says
	# 1600x900 in its name because the panel's content now depends on the
	# window: at 1280x720 a row's prose is on its hover instead of on the row
	# (`_fit_staff_width`, issue #62), which is a layout rule test_hud_plates.gd
	# owns. A `--script` run starts at the project's own window size.
	root.size = Vector2i(1600, 900)
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
		"demand": 4, "assigned": 4, "required": 4, "short": 0,
	})
	if full["hired_text"] != "hired 4":
		_fail("a row reads 'hired N', got '%s'" % full["hired_text"])
	if full["wage_text"].find("2") < 0 or full["wage_text"].find("wage") < 0:
		_fail("a row names the wage per head, got '%s'" % full["wage_text"])
	if absf(float(full["meter"]["fraction"]) - 1.0) > 0.001:
		_fail("4 hired of 4 required is a full bar, got %s" % str(full["meter"]["fraction"]))
	if full["meter"]["color"] != main.METER_FILL_LEVEL:
		_fail("a fully staffed row's bar is the level colour")
	if full["meter_text"] != "4 / 4":
		_fail("the meter prints hired / required, got '%s'" % full["meter_text"])
	if full["short_text"] != "":
		_fail("nothing short says nothing, got '%s'" % full["short_text"])
	if full["note"] != "":
		_fail("a staffed category has no idle note")
	if not bool(full["fire_enabled"]):
		_fail("FIRE is enabled with heads to fire")
	var short: Dictionary = main._staff_row_state({
		"id": "manufacturing", "name": "Manufacturing", "headcount": 2, "wage": 2,
		"demand": 4, "assigned": 2, "required": 4, "short": 2,
	})
	if absf(float(short["meter"]["fraction"]) - 0.5) > 0.001:
		_fail("2 hired of 4 required is a half bar, got %s" % str(short["meter"]["fraction"]))
	if short["meter"]["color"] != main.METER_FILL_WARN:
		_fail("a short row's bar is the warning colour")
	if short["meter_text"] != "2 / 4":
		_fail("a short row's meter reads '2 / 4', got '%s'" % short["meter_text"])
	if short["short_text"] != "▲ hire 2":
		_fail("a short row says '▲ hire 2' in words (glyph + word + colour, UI_UX §7), got '%s'" % short["short_text"])
	if short["state_text"] != "▲ SHORT" or short["state_color"] != main.METER_FILL_WARN:
		_fail("a short row's name carries a ▲ SHORT chip in the warning colour, got '%s'" % short["state_text"])
	if not bool(short["short_row"]):
		_fail("a short row is flagged for the heavier border")
	if full["state_text"] != "✔ CREWED" or full["state_color"] != main.METER_FILL_LEVEL:
		_fail("a crewed row's chip reads '✔ CREWED', got '%s'" % full["state_text"])
	if bool(full["short_row"]):
		_fail("a crewed row is not flagged short")
	var idle: Dictionary = main._staff_row_state({
		"id": "engineering", "name": "Engineering", "headcount": 0, "wage": 2,
		"demand": 0, "assigned": 0, "required": 0, "short": 0,
	})
	if idle["note"] != "nothing needs engineering yet — nothing to hire for":
		_fail("engineering with nothing hired and nothing asked reads 'nothing needs engineering yet — nothing to hire for', got '%s'" % idle["note"])
	if bool(idle["fire_enabled"]):
		_fail("FIRE is disabled at headcount 0")
	if idle["state_text"] != "":
		_fail("a category nothing asks for gets no chip at all, got '%s'" % idle["state_text"])
	var idle_logistics: Dictionary = main._staff_row_state({
		"id": "logistics", "name": "Logistics", "headcount": 0, "wage": 2,
		"demand": 0, "assigned": 0, "required": 0, "short": 0,
	})
	if idle_logistics["note"] != "" or idle_logistics["meter_text"] != "0 / 0":
		_fail("an emptied logistics crew keeps its (empty) meter, not engineering's note")
	var nobody_asked: Dictionary = main._staff_row_state({
		"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 2,
		"demand": 0, "assigned": 0, "required": 0, "short": 0,
	})
	if float(nobody_asked["meter"]["fraction"]) != 0.0 or nobody_asked["meter_text"] != "2 / 0":
		_fail("two heads nothing needs is an empty bar reading '2 / 0', got %s '%s'" % [str(nobody_asked["meter"]["fraction"]), nobody_asked["meter_text"]])
	if not bool(nobody_asked["fire_enabled"]):
		_fail("heads nothing needs must still be fireable — otherwise the payroll is a trap")
	if bool(nobody_asked["hire_enabled"]):
		_fail("HIRE is closed on a category with no establishment (the engineering money sink)")

	# --- 1b. HIRE is gated on the ESTABLISHMENT ALONE, and a payday that
	# cannot cover the hire warns beside a button that still works
	# (2026-09-26); the row's AFTER-payday status line (ticket 4) ----------
	var hireable: Dictionary = main._staff_row_state({
		"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2,
		"demand": 4, "assigned": 4, "required": 4, "short": 0, "hire_warns": false,
	})
	if not bool(hireable["hire_enabled"]):
		_fail("a category the machinery asks for has HIRE enabled")
	var unhireable: Dictionary = main._staff_row_state({
		"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2,
		"demand": 4, "assigned": 4, "required": 4, "short": 0, "hire_warns": true,
	})
	if not bool(unhireable["hire_enabled"]):
		_fail("hire_warns on the wire is a WARNING, not a disabled button (2026-09-26)")
	if main._staff_row_state({"id": "manufacturing", "required": 1})["hire_enabled"] != true:
		_fail("an establishment with no hire_warns field on it still hires")
	if bool(main._staff_row_state({"id": "manufacturing", "required": 0})["hire_enabled"]):
		_fail("a category with no establishment still closes HIRE")
	if unhireable["status_text"] != "":
		_fail("nothing unpaid or left says nothing, got '%s'" % unhireable["status_text"])
	var idled: Dictionary = main._staff_row_state({
		"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 2,
		"demand": 2, "assigned": 2, "required": 2, "short": 0, "unpaid": 2,
	})
	if idled["status_text"] != "▲ 2 unpaid, idle until payday":
		_fail("an idled row says '▲ 2 unpaid, idle until payday', got '%s'" % idled["status_text"])
	var left_row: Dictionary = main._staff_row_state({
		"id": "logistics", "name": "Logistics", "headcount": 0, "wage": 2,
		"demand": 0, "assigned": 0, "required": 0, "short": 0, "left_last_payday": 1,
	})
	if left_row["status_text"] != "▲ 1 left last payday":
		_fail("a row that lost a head says '▲ 1 left last payday', got '%s'" % left_row["status_text"])
	var both_status: Dictionary = main._staff_row_state({
		"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 2,
		"demand": 2, "assigned": 2, "required": 2, "short": 0, "unpaid": 1, "left_last_payday": 1,
	})
	if both_status["status_text"] != "▲ 1 unpaid, idle until payday  ·  ▲ 1 left last payday":
		_fail("both at once join with a separator, got '%s'" % both_status["status_text"])

	# --- 1c. the WHY line: what asks for a category (ticket 4) ---------------
	if main._staff_demand_text([]) != "nothing placed asks for staff yet":
		_fail("an empty factory's WHY line reads 'nothing placed asks for staff yet', got '%s'" % main._staff_demand_text([]))
	var one_each: Array = [{"kind": "crusher", "count": 1}, {"kind": "mixer", "count": 1}]
	if main._staff_demand_text(one_each) != "asked for by: crusher ×1, mixer ×1":
		_fail("the WHY line lists kind ×count, got '%s'" % main._staff_demand_text(one_each))
	var vats: Array = [{"kind": "cooking_vat", "count": 2}]
	if main._staff_demand_text(vats) != "asked for by: cooking vat ×2":
		_fail("a multi-word kind id reads as lowercase words, not the wire's underscore id, got '%s'" % main._staff_demand_text(vats))
	# The sources are the SIM's, read straight off the category row rather
	# than re-totalled here (issue #19 broke the Godot-side sum: it went on
	# counting a machine the player had switched off). Section 7b runs the
	# real wire; this pins that nothing is derived on the way through.
	var wire_row := {
		"id": "manufacturing",
		"sources": [{"kind": "crusher", "count": 2}, {"kind": "mixer", "count": 1}],
	}
	if main._staff_demand_sources(wire_row) != wire_row["sources"]:
		_fail("the WHY line must hand the wire's own sources through untouched")
	if main._staff_demand_sources({"id": "logistics"}) != []:
		_fail("a category the wire lists no sources for asks for nobody")
	if main._staff_demand_sources(null) != []:
		_fail("a missing row is no sources, not a crash")

	# --- 1d. what each category does, one sentence per id (ticket 4) --------
	for id in ["manufacturing", "engineering", "packaging", "logistics"]:
		if String(main.STAFF_CATEGORY_BLURBS.get(id, "")) == "":
			_fail("category %s has no blurb in STAFF_CATEGORY_BLURBS" % id)

	# --- 1e. the HIRE/FIRE buttons say what a click does and costs, and a
	# disabled HIRE says WHY in words (the staffing ticket, requirement 3) ----
	if main._staff_hire_button_text({"wage": 2}) != "HIRE +1 · £2/period":
		_fail("HIRE names the per-period cost of the head it adds, got '%s'" % main._staff_hire_button_text({"wage": 2}))
	if main._staff_hire_button_text({"wage": 0}) != "HIRE +1":
		_fail("a volunteer category's HIRE quotes no cost, got '%s'" % main._staff_hire_button_text({"wage": 0}))
	if main._staff_row_state({"id": "manufacturing", "wage": 2})["hire_text"] != "HIRE +1 · £2/period":
		_fail("the row state carries the button's own text")
	if main._staff_row_state({"id": "manufacturing"})["fire_text"] != "FIRE −1":
		_fail("FIRE says it removes one head")
	# Notice pay is the one staffing rule whose consequence lands after the
	# decision, and FIRE is the button a player reaches for in exactly the
	# payday it will not rescue them from. The warning has to be ON the
	# button, not only in the legend at the foot of the panel.
	var fire_notice := String(main._staff_row_state({"id": "manufacturing"})["fire_tooltip"])
	if not fire_notice.to_lower().contains("notice pay"):
		_fail("FIRE does not warn about notice pay ('%s')" % fire_notice)
	if String(_row("manufacturing")["fire_button"].tooltip_text) != fire_notice:
		_fail("the FIRE button carries no notice-pay tooltip")
	if main._staff_hire_blocked_text({"hire_warns": false, "wage": 2}, {}) != "":
		_fail("a row whose payday covers the hire says nothing extra")
	var blocked_snapshot := {
		"money": 8.0,
		"categories": [
			{"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2, "owed": 4},
			{"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 2, "owed": 2},
			{"id": "logistics", "name": "Logistics", "headcount": 1, "wage": 2, "owed": 1},
		],
	}
	var blocked_text: String = main._staff_hire_blocked_text(
		{"id": "manufacturing", "wage": 2, "hire_warns": true}, blocked_snapshot)
	for needle in ["would go short", "8 in hand", "%d due" % main._staff_next_bill(blocked_snapshot), "2 / period"]:
		if blocked_text.find(needle) < 0:
			_fail("a warned HIRE says WHY in words ('%s'), got '%s'" % [needle, blocked_text])
	if blocked_text.find("▲") != 0:
		_fail("the warning leads with a glyph, not colour alone (UI_UX §7), got '%s'" % blocked_text)

	# --- 1f. what hiring would unblock, from the same demand sources the WHY
	# line names (the staffing ticket, requirement 5) -------------------------
	if main._staff_hire_unblocks_text(0, [{"kind": "cooking_vat", "count": 1}]) != "":
		_fail("nothing short has nothing to unblock")
	if main._staff_hire_unblocks_text(2, []) != "hire 2 and nothing goes short":
		_fail("a shortfall with no named source still says what hiring achieves")
	var one_kind: String = main._staff_hire_unblocks_text(2, [{"kind": "cooking_vat", "count": 1}])
	if one_kind != "hire 2 → the cooking vat gets its crew again":
		_fail("one asking kind is named in display words, got '%s'" % one_kind)
	var many_kinds: String = main._staff_hire_unblocks_text(
		3, [{"kind": "crusher", "count": 1}, {"kind": "cooking_vat", "count": 2}])
	if many_kinds != "hire 3 → crusher, cooking vat all get their crew again":
		_fail("several asking kinds are all named, got '%s'" % many_kinds)

	# --- 1g. the panel leads with a verdict (requirement 2) ------------------
	var verdict_short: Dictionary = main._staff_summary_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "required": 5, "demand": 4, "short": 1},
		{"id": "engineering", "name": "Engineering", "required": 0, "demand": 0, "short": 0},
		{"id": "packaging", "name": "Packaging", "required": 3, "demand": 2, "short": 1},
	]})
	if verdict_short["text"] != "▲ 2 short — hire 1 manufacturing, 1 packaging":
		_fail("the verdict names every short category and the heads to hire, got '%s'" % verdict_short["text"])
	if int(verdict_short["short"]) != 2:
		_fail("the verdict carries the wire's own summed shortfall")
	var verdict_ok: Dictionary = main._staff_summary_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "required": 4, "demand": 4, "short": 0},
	]})
	if verdict_ok["text"] != "✔ every machine has its crew" or int(verdict_ok["short"]) != 0:
		_fail("a fully crewed factory is told so in one sentence, got '%s'" % verdict_ok["text"])
	# The regression a screenshot caught and no fixture did: a line that is
	# fully crewed but STALLED (no ingredients, or its heads unpaid and not
	# working) reports `demand` 0 for every category. Read against `demand`,
	# the verdict greeted four hired heads with "nothing placed asks for
	# staff yet". The establishment is what asks, and it does not stall.
	var verdict_stalled: Dictionary = main._staff_summary_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "required": 4, "demand": 0, "short": 0},
	]})
	if verdict_stalled["text"] != "✔ every machine has its crew":
		_fail("a crewed but stalled factory still reads as crewed, got '%s'" % verdict_stalled["text"])
	var verdict_empty: Dictionary = main._staff_summary_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "required": 0, "demand": 0, "short": 0},
	]})
	if verdict_empty["text"] != "nothing placed asks for staff yet":
		_fail("an empty factory is not congratulated on its staffing, got '%s'" % verdict_empty["text"])
	if main._staff_summary_state({})["text"] != "nothing placed asks for staff yet":
		_fail("an unpolled snapshot reads as nothing asking, not as short")
	if String(verdict_short["state"]) != "short" or String(verdict_ok["state"]) != "crewed" \
			or String(verdict_empty["state"]) != "idle":
		_fail("the verdict names its own state, so the ink is not a green/red pair")

	# --- 1h. the always-visible SHORT-STAFFED alert (requirement 1) ----------
	if not main._staff_alert_state({}).is_empty():
		_fail("an unpolled snapshot raises no alert")
	if not main._staff_alert_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "demand": 4, "short": 0},
	]}).is_empty():
		_fail("nothing short means NO strip at all, never a strip reading '0 short'")
	var alert: Dictionary = main._staff_alert_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "demand": 4, "short": 2},
		{"id": "packaging", "name": "Packaging", "demand": 2, "short": 0},
	]})
	if int(alert["short"]) != 2:
		_fail("the alert carries the wire's summed shortfall")
	for needle in ["▲", "SHORT-STAFFED", "hire 2 manufacturing", "press H"]:
		if String(alert["text"]).find(needle) < 0:
			_fail("the alert says '%s' (glyph + words + key), got '%s'" % [needle, alert["text"]])
	if String(alert["text"]).find("packaging") >= 0:
		_fail("a category that is not short is not named in the alert")
	var alert_two: Dictionary = main._staff_alert_state({"categories": [
		{"id": "manufacturing", "name": "Manufacturing", "demand": 4, "short": 2},
		{"id": "logistics", "name": "Logistics", "demand": 1, "short": 1},
	]})
	if String(alert_two["text"]).find("hire 2 manufacturing, 1 logistics") < 0:
		_fail("every short category is named, in wire order, got '%s'" % alert_two["text"])

	# --- 1i. the model explained in plain words in the panel (requirement 4) --
	if main.STAFF_RULES.size() < 3:
		_fail("the panel explains the three rules a player can only otherwise learn by losing heads")
	var rules: String = main._staff_rules_text()
	if rules.find("how staffing works") != 0:
		_fail("the legend is headed, got '%s'" % rules)
	for rule in main.STAFF_RULES:
		if rules.find(String(rule)) < 0:
			_fail("the legend prints every rule; '%s' is missing" % String(rule))
	for needle in ["WHOLE crew", "firing", "unpaid"]:
		if rules.find(needle) < 0:
			_fail("the legend covers '%s', got:\n%s" % [needle, rules])
	if rules.split("\n").size() != main.STAFF_RULES.size() + 1:
		_fail("the legend is one line per rule under one heading, got:\n%s" % rules)

	# --- 2. the footer, pure -------------------------------------------------
	var snapshot := {
		"pay_period_ticks": 1000, "next_payday_tick": 2000, "last_bill": 14,
		"walked_out_last_payday": 0, "money": 500.0,
		"categories": [
			{"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2, "demand": 4, "assigned": 4, "short": 0},
			{"id": "engineering", "name": "Engineering", "headcount": 0, "wage": 2, "demand": 0, "assigned": 0, "short": 0},
			{"id": "packaging", "name": "Packaging", "headcount": 2, "wage": 3, "demand": 2, "assigned": 2, "short": 0},
			{"id": "logistics", "name": "Logistics", "headcount": 1, "wage": 2, "demand": 1, "assigned": 1, "short": 0},
		],
	}
	if main._staff_bill(snapshot) != 4 * 2 + 2 * 3 + 1 * 2:
		_fail("the bill is every head at its category's wage, got %d" % main._staff_bill(snapshot))
	var footer: String = main._staff_footer_text(snapshot, 1700)
	if footer.find("wages £16 / period") < 0:
		_fail("the footer says 'wages £16 / period', got:\n%s" % footer)
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
	snapshot["walked_out_last_payday"] = 0
	snapshot["unpaid_last_payday"] = 2
	var idled_footer: String = main._staff_footer_text(snapshot, 1700)
	if idled_footer.find("✗ 2 idled unpaid last payday") < 0:
		_fail("two idled last payday reads '✗ 2 idled unpaid last payday' (distinct from 'left'), got:\n%s" % idled_footer)
	snapshot["unpaid_last_payday"] = 0

	# --- 2a. the BEFORE-payday warning and the covered-payday balance,
	# pure (ticket 4) ----------------------------------------------------------
	if main._staff_footer_text(snapshot, 1700).find("balance after payday: £484") < 0:
		_fail("a covered payday's footer says the balance it would leave (500 - 16 = 484), got:\n%s" % main._staff_footer_text(snapshot, 1700))
	if main._staff_shortfall_text(6, 3) != "▲ next payday short by £6 — 3 heads go unpaid":
		_fail("the ticket's own example wording, got '%s'" % main._staff_shortfall_text(6, 3))
	if main._staff_shortfall_text(2, 1) != "▲ next payday short by £2 — 1 head goes unpaid":
		_fail("one head is singular ('head goes'), got '%s'" % main._staff_shortfall_text(2, 1))
	var broke_snapshot: Dictionary = snapshot.duplicate(true)
	broke_snapshot["money"] = 8.0
	broke_snapshot["shortfall"] = 6
	if main._staff_shortfall(broke_snapshot, 8.0) != 6:
		_fail("the wire's own shortfall wins, got %d" % main._staff_shortfall(broke_snapshot, 8.0))
	var broke_footer: String = main._staff_footer_text(broke_snapshot, 1700)
	if broke_footer.find("▲ next payday short by £6") < 0:
		_fail("an uncovered payday's footer leads with the BEFORE warning, got:\n%s" % broke_footer)
	if broke_footer.find("balance after payday") >= 0:
		_fail("a short footer does not also claim a balance after payday, got:\n%s" % broke_footer)

	# `_staff_unpaid_count_if_payday_now` replays the cascade against wire
	# numbers: manufacturing 4 heads at wage 2 eat the whole balance of 8,
	# so packaging's 2 and logistics' 1 (3 total) go unpaid — the same
	# numbers `build_underfunded_line` is built to reproduce end to end.
	var cascade_snapshot := {
		"money": 8.0,
		"categories": [
			{"id": "manufacturing", "wage": 2, "owed": 4},
			{"id": "engineering", "wage": 2, "owed": 0},
			{"id": "packaging", "wage": 2, "owed": 2},
			{"id": "logistics", "wage": 2, "owed": 1},
		],
	}
	if main._staff_unpaid_count_if_payday_now(cascade_snapshot) != 3:
		_fail("the cascade leaves 3 unpaid (2 packaging + 1 logistics), got %d" % main._staff_unpaid_count_if_payday_now(cascade_snapshot))
	var zero_wage_snapshot := {
		"money": 0.0,
		"categories": [{"id": "engineering", "wage": 0, "owed": 5}],
	}
	if main._staff_unpaid_count_if_payday_now(zero_wage_snapshot) != 0:
		_fail("a zero-wage head is never unpaid, even mid-cascade, got %d" % main._staff_unpaid_count_if_payday_now(zero_wage_snapshot))

	# --- 2b. the clipboard's wages WARNING, pure ------------------------------
	# The clipboard is the ALARM now, not the readout: since the CREW plate
	# (2026-09-16) the bill and the countdown are permanently on screen four
	# inches below this line, so printing them here too would be the same
	# sentence twice in one frame. What the clipboard keeps is the half the
	# plate cannot do as well — the warning, beside the balance it is about.
	if not main._contract_wages_state(snapshot, 1700, 500.0).is_empty():
		_fail("a balance that covers the bill leaves the clipboard silent; the CREW plate has the figure")

	# Short of the bill: the warning names the gap in figures AND words beside
	# a glyph, because colour is never the whole message (UI_UX.md §7).
	var broke: Dictionary = main._contract_wages_state(snapshot, 1700, 10.0)
	if not bool(broke["short"]):
		_fail("10 cannot cover a bill of 16, so the line warns")
	if String(broke["text"]).find("short £6") < 0:
		_fail("the warning names the shortfall, got '%s'" % broke["text"])
	if String(broke["text"]).find("⚠") < 0 or String(broke["text"]).find("leave") < 0:
		_fail("the warning carries a glyph and says what happens, got '%s'" % broke["text"])

	# A part unit cannot pay a whole head: 15.9 against 16 is short by one.
	var part: Dictionary = main._contract_wages_state(snapshot, 1700, 15.9)
	if not bool(part["short"]) or String(part["text"]).find("short £1") < 0:
		_fail("a fractional balance floors, so 15.9 against 16 is short 1, got '%s'" % part["text"])

	# The wire's own figures win over Godot's sum. Since ticket 1's notice pay
	# a head fired part-way through a period is still owed for it, so
	# `next_bill` deliberately exceeds `Σ headcount × wage` here — and both
	# readouts must quote the wire, not recompute it.
	var notice: Dictionary = snapshot.duplicate(true)
	notice["next_bill"] = 20
	notice["shortfall"] = 4
	if main._staff_next_bill(notice) != 20:
		_fail("the bill is the wire's next_bill, got %d" % main._staff_next_bill(notice))
	var owed: Dictionary = main._contract_wages_state(notice, 1700, 16.0)
	if String(owed["text"]).find("wages £20") < 0:
		_fail("notice pay is billed, so the line reads 'wages 20', got '%s'" % owed["text"])
	if not bool(owed["short"]) or String(owed["text"]).find("short £4") < 0:
		_fail("the wire's shortfall is quoted verbatim, got '%s'" % owed["text"])
	if main._staff_footer_text(notice, 1700).find("wages £20 / period") < 0:
		_fail("the H panel footer quotes the same bill as the clipboard warning")
	# And so does the plate, which is the readout the warning replaced.
	var plate_footer := String(main._crew_plate_state(notice, 1700)["payday_text"])
	if plate_footer.find("wages £20") < 0:
		_fail("the CREW plate quotes the wire's bill, got '%s'" % plate_footer)
	if plate_footer.find("payday in %s" % expected_wait) < 0:
		_fail("the CREW plate counts down in seconds, got '%s'" % plate_footer)

	# Nothing honest to say beats a cost that does not exist.
	if not main._contract_wages_state({}, 1700, 500.0).is_empty():
		_fail("before the first staff poll the wages line stays hidden")
	var nobody: Dictionary = snapshot.duplicate(true)
	for category in nobody["categories"]:
		category["headcount"] = 0
	if not main._contract_wages_state(nobody, 1700, 500.0).is_empty():
		_fail("with nobody hired there is no bill, so the line hides rather than reading 'wages 0'")

	# The wages line owns its ink. It recolours when wages cannot be met, and
	# the plate's other captions read from `_contract_plate_hint_settings`, so
	# sharing that object would turn the whole plate red on one bad payday.
	if main._contract_wages_label.label_settings != main._contract_wages_settings:
		_fail("the wages line draws with its own LabelSettings")
	if main._contract_wages_settings == main._contract_plate_hint_settings:
		_fail("the wages line must not share the plate's caption settings object")

	# --- 2c. the AFTER-payday notice, pure, then wired through the same
	# fading notice a lost contract uses (ticket 4) ----------------------------
	var payday_snapshot := {
		"next_payday_tick": 3000,
		"categories": [
			{"id": "packaging", "name": "Packaging", "unpaid": 3},
			{"id": "logistics", "name": "Logistics", "left_last_payday": 1},
		],
	}
	var notice_text: String = main._staff_payday_notice_text(payday_snapshot)
	if notice_text.find("▲ PAYDAY: 3 packaging unpaid, idle until next payday") < 0:
		_fail("the notice names the category and count for unpaid heads, got:\n%s" % notice_text)
	if notice_text.find("✗ PAYDAY: 1 logistics left — unpaid two paydays running") < 0:
		_fail("the notice names the category and count for a lost head, got:\n%s" % notice_text)
	var covered_payday := {"next_payday_tick": 3000, "categories": [{"id": "manufacturing", "name": "Manufacturing"}]}
	if main._staff_payday_notice_text(covered_payday) != "":
		_fail("a covered payday announces nothing")

	main._staff_known_payday_tick = -1
	main._contract_notice.text = ""
	main._announce_staff_payday(payday_snapshot)
	if main._contract_notice.text != "":
		_fail("the first poll after a rebuild only primes the payday clock, it must not announce")
	if main._staff_known_payday_tick != 3000:
		_fail("priming remembers the wire's next_payday_tick")
	main._announce_staff_payday(payday_snapshot)
	if main._contract_notice.text != "":
		_fail("the same next_payday_tick is not a new payday")
	var advanced_payday: Dictionary = payday_snapshot.duplicate(true)
	advanced_payday["next_payday_tick"] = 4000
	main._announce_staff_payday(advanced_payday)
	if main._contract_notice.text != main._staff_payday_notice_text(advanced_payday):
		_fail("a payday that idled or lost heads is announced through _show_contract_notice, got '%s'" % main._contract_notice.text)
	main._staff_known_payday_tick = -1

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
		if row["hire_button"].disabled == bool(state["hire_enabled"]):
			_fail("row %s's HIRE button must be %s (the establishment alone gates it)" % [id, "enabled" if state["hire_enabled"] else "disabled"])
	if main._staff_money_label.text != "Money: £%.0f" % float(live.get("money", 0.0)):
		_fail("the panel's balance is the snapshot's money, got '%s'" % main._staff_money_label.text)
	if main._staff_footer_label.text != main._staff_footer_text(main._staff_snapshot, main.sim.current_tick()):
		_fail("the footer is drawn from the last poll and the current tick")
	if main._staff_panel.name != "StaffPanel":
		_fail("the panel node is named StaffPanel")
	main._set_staff_open(false)

	# --- 3b. the underfunded line: the BEFORE warning against the wire, and
	# the AFTER notice on the payday it actually causes (ticket 4) ------------
	main.sim.new_game()
	Layouts.build_underfunded_line(main.sim)
	main._rebuild_world()
	main._set_staff_open(true)
	if main._staff_shortfall(main._staff_snapshot, float(main._staff_snapshot.get("money", 0.0))) != 6:
		_fail("the underfunded line is short by exactly 6, got %d" % main._staff_shortfall(main._staff_snapshot, float(main._staff_snapshot.get("money", 0.0))))
	if main._staff_unpaid_count_if_payday_now(main._staff_snapshot) != 3:
		_fail("the cascade leaves exactly 3 heads unpaid, got %d" % main._staff_unpaid_count_if_payday_now(main._staff_snapshot))
	if main._staff_footer_label.text.find("▲ next payday short by £6 — 3 heads go unpaid") < 0:
		_fail("the panel's footer carries the ticket's own wording, got:\n%s" % main._staff_footer_label.text)
	# A payday it cannot pay WARNS and leaves the button alone (2026-09-26):
	# the row says why in words, and `_staff_hire` puts the confirmation up.
	for row in main._staff_rows:
		var category: Dictionary = _category(String(row["id"]))
		if not bool(category.get("hire_warns", false)):
			continue
		if row["hire_button"].disabled and int(category.get("required", 0)) > 0:
			_fail("row %s warns about the payday; it must not disable HIRE" % row["id"])
		if row["blocked_label"].text.find("would go short") < 0:
			_fail("row %s warns in words, got '%s'" % [row["id"], row["blocked_label"].text])
	main._set_staff_open(false)
	var payday_tick := int(main._staff_snapshot.get("next_payday_tick", 0))
	while main.sim.current_tick() < payday_tick + 1:
		_step()
	main._poll_staff()
	if main._contract_notice.text.find("packaging") < 0 and main._contract_notice.text.find("logistics") < 0:
		_fail("the payday that actually ran announces the categories it idled, got '%s'" % main._contract_notice.text)

	# Restore the funded reference line: sections 4/5 hire and fire real heads,
	# and the underfunded fixture would put a payday confirmation over each.
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	Layouts.stock_larder(main.sim, 64)
	main._rebuild_world()
	main._staff_known_payday_tick = -1
	main._contract_notice.text = ""

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
	var engineering_index: int = main._staff_row_ids.find("engineering")
	if engineering_index >= 0 and int(_category("engineering").get("headcount", 0)) == 0:
		if not _row("engineering")["fire_button"].disabled:
			_fail("FIRE is disabled on a category with nobody hired")
		main._staff_fire(engineering_index)
		if int(_category("engineering").get("headcount", 0)) != 0:
			_fail("FIRE at 0 is a no-op")
		main._staff_hire(engineering_index)
		if int(_category("engineering").get("headcount", 0)) != 1:
			_fail("HIRE on engineering adds a head")
		if _row("engineering")["fire_button"].disabled:
			_fail("FIRE is enabled once a head is hired")
		if not _row("engineering")["meter"].visible or _row("engineering")["note_label"].visible:
			_fail("a hired engineering head replaces the idle note with the meter")
		main._staff_fire(engineering_index)
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
	# The hover's first yield rung (issue #99) drops the crew line and ONLY
	# it: the shortage it explains is still said, just not the headcount.
	var yielded_hover: String = main._hover_text_at_tile(vat, main.HOVER_YIELD_CREW)
	if yielded_hover != hover.replace("\ncrew: 2 manufacturing", ""):
		_fail("a hover yielding its crew line should lose that line and nothing else, got:\n%s" % yielded_hover)
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
	if short_row["short_label"].text != "▲ hire 2" or not short_row["short_label"].visible:
		_fail("the manufacturing row says '▲ hire 2', got '%s'" % short_row["short_label"].text)
	if short_row["state_label"].text != "▲ SHORT" or not short_row["state_label"].visible:
		_fail("the short row wears its ▲ SHORT chip, got '%s'" % short_row["state_label"].text)
	var short_border: StyleBoxFlat = short_row["panel"].get_theme_stylebox("panel")
	if short_border.border_color != main.METER_FILL_WARN:
		_fail("a short row is bordered in the warning colour")
	if _row("engineering")["state_label"].visible:
		_fail("a category with nothing asking for it wears no chip")
	if _row("engineering")["panel"].get_theme_stylebox("panel").border_color == main.METER_FILL_WARN:
		_fail("a row that is not short keeps the plain hairline border")
	if short_row["meter_label"].text != "%d / %d" % [int(manufacturing.get("headcount", 0)), int(manufacturing.get("required", 0))]:
		_fail("the manufacturing meter prints the wire's hired / required, got '%s'" % short_row["meter_label"].text)
	var fill: Panel = short_row["meter"].get_node("Fill")
	var fill_style: StyleBoxFlat = fill.get_theme_stylebox("panel")
	if fill_style.bg_color != main.METER_FILL_WARN:
		_fail("the short row's bar is drawn in the warning colour")
	# And the CREW plate says it with NO panel open, which is the whole point
	# of it (the staffing ticket, requirement 1). The alarm is the heading
	# and the border; which categories and how many is the rows beneath.
	main._update_staff_panel()
	main._update_staff_alert()
	main._update_crew_panel()
	if main._crew_hint.text.find("SHORT-STAFFED") < 0:
		_fail("an understaffed factory wears the alarm with no panel open, got '%s'" % main._crew_hint.text)
	if main._crew_hint.label_settings != main._crew_hint_warn:
		_fail("the alarm is not in the warning ink")
	if main._crew_plate_style.border_color != main.METER_FILL_WARN:
		_fail("the CREW plate's border does not carry the alarm")
	if main._crew_row_nodes["manufacturing"]["chip_label"].text.find("HIRE") < 0:
		_fail("the short row names the heads to hire, got '%s'" % main._crew_row_nodes["manufacturing"]["chip_label"].text)
	# The panel's own verdict is the same fact, one sentence, under the heading.
	if main._staff_summary_label.text.find("\u25b2 2 short") < 0:
		_fail("the panel leads with its verdict, got '%s'" % main._staff_summary_label.text)
	if main._staff_rules_label.text.find("how staffing works") < 0:
		_fail("the panel carries the staffing rules in plain words")
	# The legend is INSIDE the scroll (UI_UX §2b point 32: chrome added under
	# the footer is rows clipped off the top of a capped grid).
	var rules_parent: Node = main._staff_rules_label.get_parent()
	while rules_parent != null and not (rules_parent is ScrollContainer):
		rules_parent = rules_parent.get_parent()
	if rules_parent != main._staff_scroll:
		_fail("the rules legend scrolls with the rows rather than eating the panel's height")
	if main._staff_footer_label.text.find("scroll for how staffing works") < 0:
		_fail("the always-visible footer says the legend is down there")
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
	main._poll_staff()
	main._update_staff_alert()
	if main._crew_hint.text.find("SHORT-STAFFED") >= 0:
		_fail("the alarm clears entirely once nothing is short — it never reads '0 short'")
	if main._crew_plate_style.border_color != main.CREW_PANEL_ACCENT:
		_fail("the CREW plate's border stays red after the crew is hired back")
	main._set_staff_open(true)
	main._update_staff_panel()
	if main._staff_summary_label.text.find("\u2714") < 0:
		_fail("the verdict flips to the crewed sentence, got '%s'" % main._staff_summary_label.text)
	if _row("manufacturing")["state_label"].text != "\u2714 CREWED":
		_fail("the row's chip flips back to CREWED once the crew is hired")
	main._set_staff_open(false)

	# --- 8. a dock refused its logistics head --------------------------------------
	# The shipped line, so the dock holds boxed ketchup the signed
	# corner-shop order wants: a dock asks for its logistics crew only while it holds
	# something a contract orders (`World::tick` step 6), and a dock of
	# goods nobody ordered asks for nobody and is never "held".
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	Layouts.fund(main.sim, 30000)
	Layouts.stock_larder(main.sim, 320)
	for feed in [Vector2i(-1, 0), Vector2i(4, 1)]:
		main.sim.issue_command_json(JSON.stringify({"type": "set_hopper_auto_fill", "tile": [feed.x, feed.y], "enabled": true}))
	main.sim.issue_command_json(JSON.stringify({"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true}))
	Layouts.hire_until_staffed(main.sim)
	var dock_snaps = JSON.parse_string(main.sim.dock_snapshots_json())
	var dock := Vector2i(int(dock_snaps[0]["tile"][0]), int(dock_snaps[0]["tile"][1]))
	main._rebuild_world()
	var staffed_hover: String = main._hover_text_at_tile(dock)
	if staffed_hover.find("crew: 1 logistics") < 0:
		_fail("a dock's hover names its crew, got:\n%s" % staffed_hover)
	if staffed_hover.find("held") >= 0:
		_fail("a staffed dock is not held")
	main.sim.issue_command_json(JSON.stringify({"type": "fire", "category": "logistics"}))
	var refused := false
	for i in range(30000):
		# The order strikes out while the line primes; sign it again so the
		# dock's stock stays wanted.
		if i % 500 == 0:
			main.sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": "corner_shop"}))
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
			if label.text != "crew: 1 manufacturing\n£2 / period":
				_fail("the crusher's card says 'crew: 1 manufacturing\n2 / period' under the price, got '%s'" % label.text)
		if slot["kind"] == "belt":
			saw_belt = true
			if label.text != "":
				_fail("a belt's card shows no crew line")
		if slot["kind"] == "cooking_vat" and label.text != "crew: 2 manufacturing\n£4 / period":
			_fail("the vat's card says 'crew: 2 manufacturing\n4 / period', got '%s'" % label.text)
		if slot["kind"] == "dock" and label.text != "crew: 1 logistics\n£2 / period":
			_fail("the dock's card says 'crew: 1 logistics\n2 / period', got '%s'" % label.text)
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

	# --- 11. four rows at once at 1600x900, one sentence each (issue #7) -----
	# The panel's whole job is comparison BETWEEN categories, so the four have
	# to be on screen together at the smallest size UI_UX.md §5c designs for.
	# They were not: seven stacked elements made a row 147px tall, the scroll
	# caps at 400px there, and the fourth category sat below the cut.
	main.sim.new_game()
	Layouts.build_understaffed_line(main.sim)
	main._rebuild_world()
	main._set_staff_open(true)
	root.size = Vector2i(1600, 900)
	main._position_hotbar()
	main._position_crew_panel()
	main._update_contract_panel()
	main._position_stock_panel()
	main._update_staff_panel()
	if main._staff_rows.size() != 4:
		_fail("the wire's four categories are four rows, got %d" % main._staff_rows.size())
	var scroll_cap: float = main._staff_scroll.custom_minimum_size.y
	var rows_height := 0.0
	var row_separation := float(main._staff_rows_box.get_theme_constant("separation"))
	for row in main._staff_rows:
		if rows_height > 0.0:
			rows_height += row_separation
		rows_height += (row["panel"] as Control).get_combined_minimum_size().y
	if rows_height > scroll_cap + 0.5:
		_fail("all four rows must fit the scroll at 1600x900: %.1f of rows, %.1f of cap" % [
			rows_height, scroll_cap,
		])
	# The height came off the row by ranking its five sentences, not by
	# shrinking the type: exactly one of them shows, and the rest are the
	# row's hover, so nothing a row used to print is actually gone from it.
	for row in main._staff_rows:
		var in_slot: Array = []
		for key in main.STAFF_SENTENCE_ORDER:
			if (row[key] as Control).visible:
				in_slot.append(key)
		if in_slot.size() > 1:
			_fail("row %s shows %d sentences at once (%s); the slot holds one" % [
				row["id"], in_slot.size(), ", ".join(PackedStringArray(in_slot)),
			])
		var tooltip := String((row["panel"] as Control).tooltip_text)
		for key in main.STAFF_SENTENCE_ORDER:
			var text := String(row[key].text)
			if text == "" or in_slot.has(key):
				continue
			if tooltip.find(text) < 0:
				_fail("row %s drops '%s' instead of moving it to the hover" % [row["id"], text])
	# A short row leads with what hiring would unblock, not with its blurb.
	var ranked := _row("manufacturing")
	if not ranked["unblock_label"].visible:
		_fail("a short row's sentence is what hiring it would unblock, got '%s'" % [
			ranked["blurb_label"].text if ranked["blurb_label"].visible else "nothing",
		])
	if ranked["blurb_label"].visible:
		_fail("the blurb yields the slot to the row's own news")
	# And the bar keeps its caption beside it rather than on a line of its own.
	if not ranked["meter_caption"].visible or ranked["meter_caption"].get_parent() != ranked["meter_row"]:
		_fail("the meter's caption reads as its label, on the bar's own line")
	if ranked["meter"].get_parent() != ranked["meter_row"]:
		_fail("the bar and its caption share a line")
	# The shortfall and the per-tick working figure never share that line:
	# one is an instruction, the other a flow figure that moves under it.
	if ranked["short_label"].visible and ranked["working_label"].visible:
		_fail("'%s' and '%s' are both in the slot after the bar" % [
			ranked["short_label"].text, ranked["working_label"].text,
		])
	main._set_staff_open(false)

	# --- 12. the switch moves BOTH numbers, not just the sim's (issue #19) ---
	#
	# The blocker this section exists for: mothballing dropped the sim's
	# `required` from 4 to 2 while the CREW caption and the H panel's WHY
	# line went on saying 4, because Godot re-totalled the crews of every
	# PLACED machine instead of reading the establishment the sim had just
	# recomputed. One number, one source. Cut `sources` out of
	# `staff_snapshot_json` or put the GDScript walk back and this fails.
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	main._rebuild_world()
	main._update_staff_panel()
	var plate_before: Dictionary = main._crew_plate_state(main._staff_snapshot, main.sim.current_tick())
	var manufacturing_before := {}
	for row in plate_before["rows"]:
		if String(row["id"]) == "manufacturing":
			manufacturing_before = row
	if manufacturing_before.is_empty() or String(manufacturing_before["count_text"]).find("/ 4") < 0:
		_fail("the shipped line asks for 4 manufacturing, the caption says '%s'" % str(manufacturing_before.get("count_text", "")))
	if String(manufacturing_before["assigned_text"]).find("cooking vat") < 0:
		_fail("the vat is one of the things asking, got '%s'" % String(manufacturing_before["assigned_text"]))

	var vat_tile: Vector2i = Layouts.UNDERSTAFFED_VAT
	main.sim.issue_command_json(JSON.stringify({
		"type": "set_mothballed", "tile": [vat_tile.x, vat_tile.y], "enabled": true,
	}))
	main._rebuild_world()
	main._update_staff_panel()
	var plate_after: Dictionary = main._crew_plate_state(main._staff_snapshot, main.sim.current_tick())
	var manufacturing_after := {}
	for row in plate_after["rows"]:
		if String(row["id"]) == "manufacturing":
			manufacturing_after = row
	if String(manufacturing_after.get("count_text", "")).find("/ 2") < 0:
		_fail("a switched-off vat takes its 2 cooks off the CREW caption, got '%s'" % str(manufacturing_after.get("count_text", "")))
	if String(manufacturing_after.get("assigned_text", "")).find("cooking vat") >= 0:
		_fail("a switched-off vat is not one of the things asking, got '%s'" % String(manufacturing_after.get("assigned_text", "")))
	var why: String = main._staff_demand_text(main._staff_demand_sources(_category("manufacturing")))
	if why.find("cooking vat") >= 0:
		_fail("the H panel's WHY line still names the switched-off vat: '%s'" % why)
	if why.find("crusher") < 0:
		_fail("but it still names what IS switched on: '%s'" % why)

	if _failed:
		quit(1)
		return
	print("STAFF_PANEL_OK: the staff panel (H) reads staff_snapshot_json row for row with ticket 2's icons, HIRE/FIRE move one head through the real commands and the row follows at once, the understaffed line's vat reads starved with 'short of manufacturing staff' and its crew in hover over an amber load bar, a dock without its logistics head reads held, the build-menu card shows a crew, one modal at a time with E/M/C, ESC closes, polling is on the tick and frozen by pause")
	quit(0)
