# Headless proof of the payday warning: the confirmation that interrupts a
# spend which would leave the next payday unpayable (2026-09-26).
#
# Run with:
#   godot --headless --path godot_project --script test_payday_warning.gd
#
# Verifies:
#   * the DECISION is the simulation's, never Godot's — `spend_preview_json`
#     answers `warn` for the exact command about to be issued, and every
#     figure the panel prints comes from that answer rather than from
#     arithmetic here (`godot_project/CLAUDE.md`'s one rule),
#   * a spend the payday can cover opens nothing and goes through at once,
#   * a spend that CROSSES the line raises the panel, and one made while
#     already short raises it too with the other heading — the two cases the
#     owner chose to warn about separately,
#   * CANCEL leaves the balance, the owned count and the world alone; SPEND
#     ANYWAY issues the command, and the money moves,
#   * ESC cancels and does NOT reach the panel underneath or the pause menu,
#     and the panel swallows every other key while it is up,
#   * all four spends are gated: equipment, ingredient stock, a contract fee
#     and a hire — and the hire is quoted as a recurring wage, not as a price,
#   * SHIFT's replacement purchase raises it per tile, and cancelling one
#     still ends build mode with nothing owned (the dead-ghost trap),
#   * a hire the payday cannot cover now WARNS: the staff row's HIRE button
#     stays live and says why in words,
#   * the panel counts as a modal for the cursor, for `_anything_open` and in
#     `_wire_changed`'s bitmask, and it does NOT pause the simulation,
#   * the subject is a display name, the payday is in seconds, and nothing on
#     the panel is a wire id.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false
var _failed := false
var _yes_calls := 0
var _cancel_calls := 0


func _fail(msg: String) -> void:
	_failed = true
	printerr("PAYDAY_WARNING_FAIL: %s" % msg)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	if _failed:
		quit(1)
	return true


func _note_yes() -> void:
	_yes_calls += 1


func _note_cancel() -> void:
	_cancel_calls += 1


func _step() -> void:
	main._advance_simulation(1.0 / float(main.TICK_HZ))


func _key(keycode: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _money() -> float:
	return float(main._read_economy().get("money", 0.0))


func _bill() -> int:
	main._poll_staff()
	return int(main._staff_snapshot.get("next_bill", 0))


## Every word the panel is showing, as one string — its heading, its subject
## line, its two-column figures and the consequence sentence.
func _panel_text() -> String:
	var out: Array = []
	_collect_text(main._payday_confirm_panel, out)
	return "\n".join(PackedStringArray(out))


func _collect_text(node: Node, out: Array) -> void:
	if node == null:
		return
	if node is Label:
		out.append(String(node.text))
	elif node is Button:
		out.append(String(node.text))
	for child in node.get_children():
		_collect_text(child, out)


func _slot_index(kind: String) -> int:
	for i in range(main._shop_slots.size()):
		if String(main._shop_slots[i]["kind"]) == kind:
			return i
	return -1


func _market_index(item: String) -> int:
	for i in range(main._market_slots.size()):
		if String(main._market_slots[i]["id"]) == item:
			return i
	return -1


func _staff_index(id: String) -> int:
	for i in range(main._staff_rows.size()):
		if String(main._staff_rows[i]["id"]) == id:
			return i
	return -1


## Is this offer on the clipboard? The economy wire's own active list, never a
## count kept here.
func _signed(id: String) -> bool:
	for contract in main._read_economy().get("contracts", []):
		if String(contract.get("id", "")) == id:
			return true
	return false


func _category(id: String) -> Dictionary:
	for category in main._staff_snapshot.get("categories", []):
		if String(category.get("id", "")) == id:
			return category
	return {}


## The staffed reference line, with the balance SET to `money` — `fund` sets
## rather than adds, which is the only way to put the factory exactly one unit
## short of a bill it is about to spend against.
func _staffed_line(money: int) -> void:
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	Layouts.hire_until_staffed(main.sim)
	Layouts.fund(main.sim, money)
	main._rebuild_world()
	main._refresh_owned_counts()
	main._poll_staff()


func _run_checks() -> void:
	# The bill this whole test spends against. Read once from a funded line, so
	# every balance below is expressed in terms of the sim's own figure rather
	# than a wage table copied in here.
	_staffed_line(4000)
	var bill := _bill()
	if bill <= 0:
		_fail("the staffed reference line owes nothing at its next payday; nothing here can warn")
		return
	var belt_price := int(main._price_of("belt"))
	if belt_price <= 0:
		_fail("the shop prices a belt at %d" % belt_price)
		return

	_check_the_sim_decides(bill, belt_price)
	_check_crossing_and_already_short(bill, belt_price)
	_check_cancel_and_confirm(bill, belt_price)
	_check_keys_and_modal_rules(bill, belt_price)
	_check_ingredient_and_contract(bill)
	_check_hire(bill)
	_check_shift_rebuy(bill, belt_price)
	_check_a_refusal_is_not_a_dilemma(belt_price)

	if _failed:
		return
	print("PAYDAY_WARNING_OK: the sim answers `warn` for the command about to be issued and Godot only renders it; a spend that crosses the next payday's bill and a spend made while already short both raise the confirmation with their own heading; CANCEL leaves money, stock and world untouched and SPEND ANYWAY issues the command; ESC cancels without reaching the panel behind it; equipment, ingredient stock, a contract fee, a hire and SHIFT's replacement purchase are all gated; a hire the payday cannot cover warns beside a live button; and a spend the balance cannot cover raises nothing at all, because the economy refuses it")
	quit(0)


## THE DECISION IS THE SIM'S. Nothing here recomputes it, and the wire answers
## for the exact dictionary that is about to be issued.
func _check_the_sim_decides(bill: int, belt_price: int) -> void:
	_staffed_line(bill + belt_price * 10)
	var command := {"type": "purchase", "kind": "belt"}
	var preview: Dictionary = main._spend_preview(command)
	if preview.is_empty():
		_fail("spend_preview_json answered nothing parseable")
		return
	for field in [
		"spends", "subject", "cost", "money_now", "money_after", "next_bill",
		"bill_after", "shortfall_now", "shortfall_after", "affordable",
		"crosses", "warn",
	]:
		if not preview.has(field):
			_fail("the wire's preview carries no `%s`" % field)
	if not bool(preview.get("spends", false)):
		_fail("a belt purchase spends something")
	if int(preview.get("cost", 0)) != belt_price:
		_fail("the preview prices a belt at %s, the shop at %d" % [str(preview.get("cost")), belt_price])
	if int(preview.get("next_bill", 0)) != bill:
		_fail("the preview's bill is %s, the staff wire's is %d" % [str(preview.get("next_bill")), bill])
	if bool(preview.get("warn", true)):
		_fail("a balance of ten belts over the bill must not warn")

	# A preview mutates NOTHING: it is a question, and asking it must not move
	# the world a command would have moved.
	var revision_before: int = main.sim.revision()
	var save_before: String = main.sim.save_ron()
	main._spend_preview({"type": "purchase", "kind": "cooking_vat"})
	main._spend_preview({"type": "hire", "category": "manufacturing"})
	if main.sim.revision() != revision_before:
		_fail("asking the preview moved the revision")
	if main.sim.save_ron() != save_before:
		_fail("asking the preview changed the world")

	# ...and the funnel spends AT ONCE when there is no warning, with no panel.
	_yes_calls = 0
	main._spend_with_confirm(command, _note_yes, _note_cancel)
	if _yes_calls != 1:
		_fail("an unwarned spend must go through at once, got %d calls" % _yes_calls)
	if main._payday_confirm_open:
		_fail("an unwarned spend raised the confirmation")
	if _cancel_calls != 0:
		_fail("an unwarned spend called the cancel callback")


## The two cases the owner chose to warn about, and the heading that tells
## them apart.
func _check_crossing_and_already_short(bill: int, belt_price: int) -> void:
	# ONE UNIT short of covering the bill afterwards: the balance pays for the
	# belt and then cannot pay the wages.
	_staffed_line(bill + belt_price - 1)
	var command := {"type": "purchase", "kind": "belt"}
	var preview: Dictionary = main._spend_preview(command)
	if not bool(preview.get("warn", false)):
		_fail("a purchase that leaves the payday 1 short must warn: %s" % JSON.stringify(preview))
		return
	if not bool(preview.get("crosses", false)):
		_fail("this purchase IS the cause, so the preview must say it crosses")
	if int(preview.get("shortfall_now", -1)) != 0:
		_fail("the payday is covered before the spend, so shortfall_now is 0")
	if int(preview.get("shortfall_after", 0)) != 1:
		_fail("one unit short, got %s" % str(preview.get("shortfall_after")))

	_yes_calls = 0
	main._spend_with_confirm(command, _note_yes, _note_cancel)
	if not main._payday_confirm_open:
		_fail("a warned spend must raise the confirmation")
		return
	if _yes_calls != 0:
		_fail("a warned spend must not have spent yet")
	if not main._payday_confirm_panel.visible:
		_fail("the confirmation is open but not visible")

	# EVERY FIGURE ON THE PLATE IS THE PREVIEW'S. Checked against the wire's
	# own numbers, so a Godot-side recomputation cannot pass this.
	var words := _panel_text()
	if words.find("THIS WILL LEAVE YOUR STAFF UNPAID") < 0:
		_fail("the crossing heading names the cause, got:\n%s" % words)
	if words.find("▲") < 0:
		_fail("the warning leads with a glyph, never colour alone (UI_UX §7)")
	if words.find(main._kind_display_name("belt")) < 0:
		_fail("the panel does not name the subject by display name, got:\n%s" % words)
	if words.find("belt") >= 0:
		_fail("the panel prints the wire id `belt` rather than its display name:\n%s" % words)
	for figure in [
		"%d" % roundi(float(preview.get("money_after", 0.0))),
		"%d" % int(preview.get("bill_after", 0)),
		"%d" % int(preview.get("shortfall_after", 0)),
	]:
		if words.find(figure) < 0:
			_fail("the panel omits the preview's own figure %s, got:\n%s" % [figure, words])
	# WHEN, in seconds off the staff wire's own payday clock — the same figure
	# `_payday_seconds_away` gives the footer, never a tick count.
	var due_in: int = main._payday_seconds_away()
	if due_in < 0:
		_fail("the staff wire carries no payday clock for the panel to read")
	if words.find("payday in") < 0 or words.find("%ds" % due_in) < 0:
		_fail("the panel does not say payday lands in %ds, got:\n%s" % [due_in, words])
	if words.to_lower().find("tick") >= 0:
		_fail("a player-facing panel must not say `tick`, got:\n%s" % words)
	if words.find("idled") < 0:
		_fail("the panel does not say what going unpaid actually does, got:\n%s" % words)
	main._on_payday_confirm_no()

	# ALREADY SHORT: the same purchase, made from a balance the bill already
	# beats. It warns (the owner's call), and says so differently — the click
	# is not the cause.
	_staffed_line(maxi(0, bill - 1))
	if _money() < float(belt_price):
		_fail("this section needs the belt to be affordable while the bill is not")
		return
	var short_preview: Dictionary = main._spend_preview(command)
	if not bool(short_preview.get("warn", false)):
		_fail("a purchase made while already short must warn")
	if bool(short_preview.get("crosses", true)):
		_fail("the payday was already short, so this purchase does not cross the line")
	if int(short_preview.get("shortfall_now", 0)) <= 0:
		_fail("the preview must report the shortfall that was already there")
	main._spend_with_confirm(command, _note_yes, _note_cancel)
	if not main._payday_confirm_open:
		_fail("a purchase made while already short must raise the confirmation")
		return
	var short_words := _panel_text()
	if short_words.find("ALREADY GOING UNPAID") < 0:
		_fail("the already-short heading must not blame this click, got:\n%s" % short_words)
	main._on_payday_confirm_no()


## The two answers, through the real shop slot the player clicks.
func _check_cancel_and_confirm(bill: int, belt_price: int) -> void:
	var index := _slot_index("belt")
	if index < 0:
		_fail("the build menu has no belt slot to click")
		return

	_staffed_line(bill + belt_price - 1)
	var owned_before: int = main._owned_of("belt")
	var money_before := _money()
	var revision_before: int = main.sim.revision()
	main._buy_slot(index)
	if not main._payday_confirm_open:
		_fail("the build menu's own buy is not gated")
		return
	main._on_payday_confirm_no()
	if main._payday_confirm_open:
		_fail("CANCEL leaves the confirmation up")
	if main._owned_of("belt") != owned_before or not is_equal_approx(_money(), money_before):
		_fail("CANCEL spent anyway: owned %d -> %d, money %f -> %f" % [
			owned_before, main._owned_of("belt"), money_before, _money(),
		])
	if main.sim.revision() != revision_before:
		_fail("CANCEL issued a command to the simulation")

	main._buy_slot(index)
	if not main._payday_confirm_open:
		_fail("the second click does not re-raise the confirmation")
		return
	main._on_payday_confirm_yes()
	if main._payday_confirm_open:
		_fail("SPEND ANYWAY leaves the confirmation up")
	if main._owned_of("belt") != owned_before + 1:
		_fail("SPEND ANYWAY did not buy the belt: owned %d -> %d" % [owned_before, main._owned_of("belt")])
	if _money() >= money_before:
		_fail("SPEND ANYWAY did not pay for the belt: %f -> %f" % [money_before, _money()])
	# ...and the payday is genuinely short afterwards, which is the whole point
	# of having been asked.
	if _bill() <= int(floor(_money())):
		_fail("after spending anyway the balance still covers the bill; the warning was empty")

	# The BUTTONS do the same as the handlers a test can call directly — the
	# player only ever has the buttons.
	main._buy_slot(index)
	if not main._payday_confirm_open:
		_fail("the balance is short, so every further purchase warns (the owner's call)")
		return
	var no_button: Button = main._payday_confirm_panel.find_child("PaydayConfirmNoButton", true, false)
	if no_button == null:
		_fail("the confirmation has no CANCEL button")
		return
	no_button.pressed.emit()
	if main._payday_confirm_open:
		_fail("the CANCEL button closes nothing")
	if main._payday_confirm_yes == null or main._payday_confirm_yes.text.find("SPEND ANYWAY") < 0:
		_fail("the confirming button must say what it does, got '%s'" % (
			"" if main._payday_confirm_yes == null else main._payday_confirm_yes.text
		))


## ESC, the keys the panel swallows, and the flags it has to appear in.
func _check_keys_and_modal_rules(bill: int, belt_price: int) -> void:
	_staffed_line(bill + belt_price - 1)
	main._set_shop_menu_open(true)
	main._buy_slot(_slot_index("belt"))
	if not main._payday_confirm_open:
		_fail("a purchase from the open build menu is not gated")
		return

	# The flags. All three, because each one answers a different question and
	# only the bitmask keeps the panel's own poll alive.
	if not main._fp_mouse_free():
		_fail("the confirmation must hand the cursor back in first person")
	if not main._anything_open():
		_fail("`_anything_open` must count the confirmation")
	main._polled_modals = -1
	if not main._wire_changed():
		_fail("the confirmation must move `_wire_changed`'s modal bitmask")

	# THE SIMULATION KEEPS RUNNING. Only the pause menu stops ticks, which is
	# why the command is re-issued rather than replayed on confirm.
	var tick_before: int = main.sim.current_tick()
	_step()
	if main.sim.current_tick() <= tick_before:
		_fail("the confirmation must not pause the simulation")

	# Every other key is swallowed: the panel is waiting on an answer, and a
	# key that reached the world would be acting on a purchase nobody settled.
	_key(KEY_E)
	if not main._payday_confirm_open:
		_fail("E closed the confirmation")
	_key(KEY_TAB)
	if main._view_mode != main.ViewMode.FIRST_PERSON and main._payday_confirm_open:
		pass # TAB is swallowed; the mode is whatever the session opened in.
	_key(KEY_P)
	if main._production_open:
		_fail("a key reached the world through the confirmation")

	# ESC cancels, and reaches NEITHER the panel behind it NOR the pause menu.
	_key(KEY_ESCAPE)
	if main._payday_confirm_open:
		_fail("ESC must cancel the confirmation")
	if main._pause_open:
		_fail("ESC opened the pause menu behind the confirmation")
	if not main._shop_open:
		_fail("ESC closed the build menu the purchase was clicked in")
	main._set_shop_menu_open(false)


## The other two spends of the four: ingredient stock and a contract fee.
func _check_ingredient_and_contract(bill: int) -> void:
	# --- ingredient stock ---
	_staffed_line(4000)
	Layouts.make_room(main.sim, "tomato", 16)
	var tomato_price := 0.0
	for entry in main._ingredients_snapshot().get("entries", []):
		if String(entry.get("id", "")) == "tomato":
			tomato_price = float(entry.get("price", 0.0))
	if tomato_price <= 0.0:
		_fail("the market prices tomato at %f" % tomato_price)
		return
	var room := Layouts.stock_room_for(main.sim, "tomato")
	if room <= 0:
		_fail("this section needs warehouse room for tomato, got %d" % room)
		return
	var crate: int = mini(int(main.MARKET_CRATE), room)
	var crate_cost := int(round(tomato_price * float(crate)))
	main._poll_staff()
	Layouts.fund(main.sim, _bill() + crate_cost - 1)
	main._refresh_owned_counts()
	var market_index := _market_index("tomato")
	if market_index < 0:
		_fail("the market has no tomato slot")
		return
	var held_before := Layouts.stock_held(main.sim, "tomato")
	main._buy_ingredient_slot(market_index)
	if not main._payday_confirm_open:
		_fail("an ingredient buy is not gated")
		return
	var market_words := _panel_text()
	if market_words.find(main._item_display_name("tomato")) < 0:
		_fail("the panel does not name the ingredient, got:\n%s" % market_words)
	if market_words.find("%d" % crate_cost) < 0:
		_fail("the panel quotes the crate's cost (%d), got:\n%s" % [crate_cost, market_words])
	main._on_payday_confirm_no()
	if Layouts.stock_held(main.sim, "tomato") != held_before:
		_fail("CANCEL bought stock anyway")
	main._buy_ingredient_slot(market_index)
	main._on_payday_confirm_yes()
	if Layouts.stock_held(main.sim, "tomato") <= held_before:
		_fail("SPEND ANYWAY bought no stock: %d -> %d" % [
			held_before, Layouts.stock_held(main.sim, "tomato"),
		])

	# --- a contract fee ---
	_staffed_line(4000)
	main._set_contract_board_open(true)
	main._update_contract_board()
	var offer_index := -1
	var fee := 0
	for i in range(main._board_rows.size()):
		for offer in main._read_economy().get("board", []):
			if String(offer.get("id", "")) != String(main._board_rows[i]["id"]):
				continue
			if bool(offer.get("active", false)):
				continue
			if offer_index < 0 or int(offer.get("price", 0)) < fee:
				offer_index = i
				fee = int(offer.get("price", 0))
	if offer_index < 0 or fee <= 0:
		_fail("the board offers nothing unsigned with a fee to warn about")
		main._set_contract_board_open(false)
		return
	Layouts.fund(main.sim, _bill() + fee - 1)
	main._update_contract_board()
	var offer_id := String(main._board_rows[offer_index]["id"])
	main._sign_contract_offer(offer_index)
	if not main._payday_confirm_open:
		_fail("a contract fee is not gated")
		main._set_contract_board_open(false)
		return
	var board_words := _panel_text()
	if board_words.find(offer_id) >= 0:
		_fail("the panel prints the contract's wire id, got:\n%s" % board_words)
	main._on_payday_confirm_no()
	if _signed(offer_id):
		_fail("CANCEL signed the contract anyway")
	main._sign_contract_offer(offer_index)
	main._on_payday_confirm_yes()
	if not _signed(offer_id):
		_fail("SPEND ANYWAY did not sign the contract")
	main._set_contract_board_open(false)


## The hire: the one spend that takes nothing out of the balance today, and
## the one whose rule this change relaxed from a refusal to a warning.
func _check_hire(_bill_unused: int) -> void:
	_staffed_line(4000)
	main._set_staff_open(true)
	main._poll_staff()
	var id := ""
	var wage := 0
	for category in main._staff_snapshot.get("categories", []):
		if int(category.get("required", 0)) > 0 and int(category.get("wage", 0)) > 0:
			id = String(category.get("id", ""))
			wage = int(category.get("wage", 0))
			break
	if id == "":
		_fail("the staffed line has no waged category with an establishment to hire into")
		main._set_staff_open(false)
		return
	Layouts.fund(main.sim, _bill() + wage - 1)
	main._poll_staff()
	main._update_staff_panel()

	var preview: Dictionary = main._spend_preview({"type": "hire", "category": id})
	if not bool(preview.get("warn", false)):
		_fail("a hire the next payday cannot cover must warn: %s" % JSON.stringify(preview))
	if int(preview.get("cost", -1)) != 0:
		_fail("a hire costs nothing TODAY, got %s" % str(preview.get("cost")))
	if int(preview.get("bill_after", 0)) != int(preview.get("next_bill", 0)) + wage:
		_fail("a hire raises the BILL by its wage: %s -> %s" % [
			str(preview.get("next_bill")), str(preview.get("bill_after")),
		])

	# The row itself: the button is LIVE and the warning is in words beside it.
	var row_index := _staff_index(id)
	if row_index < 0:
		_fail("the staff panel has no %s row" % id)
		main._set_staff_open(false)
		return
	var category := _category(id)
	if not bool(category.get("hire_warns", false)):
		_fail("the wire must report `hire_warns` for a hire the payday cannot cover")
	if main._staff_rows[row_index]["hire_button"].disabled:
		_fail("a warned hire must leave the HIRE button live (2026-09-26)")
	var why := String(main._staff_rows[row_index]["blocked_label"].text)
	if why.find("would go short") < 0:
		_fail("the row does not say why the hire is warned, got '%s'" % why)

	var headcount_before := int(category.get("headcount", 0))
	main._staff_hire(row_index)
	if not main._payday_confirm_open:
		_fail("a hire is not gated")
		main._set_staff_open(false)
		return
	var hire_words := _panel_text()
	if hire_words.find(String(category.get("name", id))) < 0:
		_fail("the panel does not name the category, got:\n%s" % hire_words)
	if hire_words.find("every payday") < 0:
		_fail("a hire must be quoted as a RECURRING wage, not a price, got:\n%s" % hire_words)
	main._on_payday_confirm_no()
	main._poll_staff()
	if int(_category(id).get("headcount", 0)) != headcount_before:
		_fail("CANCEL hired anyway")
	main._staff_hire(row_index)
	main._on_payday_confirm_yes()
	main._poll_staff()
	if int(_category(id).get("headcount", 0)) != headcount_before + 1:
		_fail("the hire the payday cannot cover must HAPPEN when confirmed (it used to be refused)")
	main._set_staff_open(false)


## SHIFT's replacement purchase, which is the interruption the owner chose to
## take per tile, and the dead-ghost trap a cancel could leave behind.
func _check_shift_rebuy(bill: int, belt_price: int) -> void:
	_staffed_line(bill + belt_price * 2 - 1)
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "belt"}))
	main._refresh_owned_counts()
	main._auto_buy_forced = true
	main._enter_build_mode("belt")
	main._build_direction = "east"
	main._ghost_tile = Vector2i(4, 14)
	main._ghost_valid = true
	main._ghost_reason = ""
	main._confirm_build()
	if main._placement_at_tile(Vector2i(4, 14)).is_empty():
		_fail("the tile must still be PLACED; only the rebuy was in question")
	if not main._payday_confirm_open:
		_fail("SHIFT's replacement purchase is not gated")
		main._auto_buy_forced = false
		main._exit_build_mode()
		return
	var owned_before: int = main._owned_of("belt")
	main._on_payday_confirm_no()
	if main._owned_of("belt") != owned_before:
		_fail("CANCEL bought the replacement anyway")
	# THE DEAD-GHOST TRAP: the placement spent the last belt, so cancelling the
	# rebuy has to end build mode. A cancel that skipped `_finish_build` would
	# leave a ghost that can never be built and a prompt reading ✗ NONE OWNED.
	if main._owned_of("belt") <= 0 and main._in_build_mode():
		_fail("cancelling the rebuy left build mode on with nothing owned")
	main._auto_buy_forced = false
	main._exit_build_mode()


## A SPEND THE BALANCE CANNOT COVER IS A REFUSAL, NOT A DILEMMA. The economy
## declines it, the balance does not move, so the payday is left exactly as
## short as it already was — there is nothing to confirm, and the click is owed
## the plain "no" every refused command gets (`test_audio.gd` pins the cue).
func _check_a_refusal_is_not_a_dilemma(belt_price: int) -> void:
	_staffed_line(maxi(0, belt_price - 1))
	var command := {"type": "purchase", "kind": "belt"}
	var preview: Dictionary = main._spend_preview(command)
	if bool(preview.get("affordable", true)):
		_fail("a belt at %d called affordable on a balance of %s" % [
			belt_price, str(preview.get("money_now"))
		])
		return
	if int(preview.get("shortfall_after", 0)) <= 0:
		_fail("this balance is under the bill, so the preview must still report the gap")
	if bool(preview.get("warn", true)):
		_fail("a purchase the balance cannot cover must not warn")

	# THROUGH THE FUNNEL: no panel, and the spending handler runs so it can
	# sound the refusal the simulation is about to give.
	_yes_calls = 0
	_cancel_calls = 0
	main._spend_with_confirm(command, _note_yes, _note_cancel)
	if main._payday_confirm_open:
		_fail("an unaffordable purchase raised the confirmation")
		main._on_payday_confirm_no()
		return
	if _yes_calls != 1 or _cancel_calls != 0:
		_fail("an unaffordable spend must reach its own handler once, got %d yes / %d cancel" % [
			_yes_calls, _cancel_calls
		])

	# A HIRE IS ALWAYS AFFORDABLE — it debits nothing today — so the same
	# empty balance still warns about the payday it would raise.
	var hire: Dictionary = main._spend_preview({"type": "hire", "category": "manufacturing"})
	if not bool(hire.get("affordable", false)):
		_fail("a hire costs nothing today, so no balance makes it unaffordable")
	if not bool(hire.get("warn", false)):
		_fail("a hire on an empty balance leaves the payday short and must warn")
