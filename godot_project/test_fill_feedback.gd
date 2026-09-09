# Headless proof that SPACE (load hoppers) and V (load fluid tanks) say why
# they did nothing, when they would have done nothing — the exact silent
# dead-keypress gap a player hit directly: "how am I meant to load the
# hopper for sugar?" with no stock bought and no hint that stock was the
# missing thing. `_hopper_fill_blocker`/`_tank_fill_blocker` (main.gd) are
# checked before either command reaches the sim, and this proves both
# messages appear at the right moments and clear once the real fix (buying
# stock) is applied — not just that the strings exist somewhere.
#
# Run with:
#   godot --headless --path godot_project --script test_fill_feedback.gd
#
# Verifies:
#   * WITH NOTHING PLACED, both blockers name the missing BUILDING ("no
#     hopper placed" / "no fluid tank placed"), not a generic failure,
#   * WITH A HOPPER AND A TANK PLACED BUT NO STOCK BOUGHT (the real
#     reference line, freshly built, before `stock_larder`), both blockers
#     point at the Ingredient Market by name,
#   * ONCE STOCK IS BOUGHT for every raw item the placed buildings actually
#     need, both blockers clear to "" — the message is tied to the real
#     state, not a one-time flag that never resets,
#   * A TANK NOT YET FEEDING ANY PIPE RUN has nothing to check stock for and
#     is not blocked at all — the placement gap that message deliberately
#     does not try to diagnose.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("FILL_FEEDBACK_FAIL: %s" % msg)
	quit(1)


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


func _run_checks() -> void:
	# --- 1. A fresh, empty world: nothing is placed at all -----------------
	var hopper_blocker: String = main._hopper_fill_blocker()
	if hopper_blocker == "" or not hopper_blocker.contains("hopper"):
		_fail("with no hopper placed, SPACE should say so by name: got '%s'" % hopper_blocker)
		return
	var tank_blocker: String = main._tank_fill_blocker()
	if tank_blocker == "" or not tank_blocker.contains("fluid tank"):
		_fail("with no tank placed, V should say so by name: got '%s'" % tank_blocker)
		return

	# --- 2. The real reference line, placed but not yet stocked -----------
	Layouts.build_reference_line(main.sim)
	main._rebuild_world()
	if main._hopper_states.is_empty():
		_fail("test setup expects the reference line to place at least one hopper")
		return
	if main._fluid_tank_states.is_empty():
		_fail("test setup expects the reference line to place a fluid tank")
		return

	var unstocked_hopper_blocker: String = main._hopper_fill_blocker()
	if unstocked_hopper_blocker == "" or not unstocked_hopper_blocker.contains("Ingredient Market"):
		_fail("with zero stock, SPACE's message should point at the Ingredient Market: got '%s'" % unstocked_hopper_blocker)
		return
	var unstocked_tank_blocker: String = main._tank_fill_blocker()
	if unstocked_tank_blocker == "" or not unstocked_tank_blocker.contains("Ingredient Market"):
		_fail("with zero vinegar, V's message should point at the Ingredient Market: got '%s'" % unstocked_tank_blocker)
		return

	# --- 3. Buying stock clears both blockers -------------------------------
	Layouts.stock_larder(main.sim)
	if main._hopper_fill_blocker() != "":
		_fail("buying stock for every raw item must clear the SPACE blocker, got '%s'" % main._hopper_fill_blocker())
		return
	if main._tank_fill_blocker() != "":
		_fail("buying vinegar must clear the V blocker, got '%s'" % main._tank_fill_blocker())
		return

	# And a real fill must now actually move stock into the hopper — proving
	# the blocker being clear and the command actually working are the same
	# fact, not two things that happened to agree here.
	var before: Dictionary = main._hopper_states.values()[0]
	main.sim.issue_command_json(JSON.stringify({"type": "fill_hopper"}))
	main._refresh_buffers()
	var after: Dictionary = main._hopper_states.get(main._tile_key(main._pair_to_tile(before.get("tile"))), {})
	if int(after.get("quantity", 0)) <= int(before.get("quantity", 0)):
		_fail("SPACE cleared its blocker but did not actually fill the hopper (%s -> %s)" % [
			str(before.get("quantity")), str(after.get("quantity"))
		])
		return

	# --- 4. An unconnected tank has nothing to check stock for --------------
	# A second, freshly-placed tank feeding no pipe run at all must not be
	# blocked on stock it cannot even name a fluid for — that is a placement
	# gap, not a stock gap, and this message deliberately does not try to
	# tell those two apart.
	Layouts.place(main.sim, "fluid_tank", Vector2i(50, 50), "east")
	main._rebuild_world()
	if main._tank_fill_blocker() != "":
		_fail("an unconnected tank should not be blocked on stock: got '%s'" % main._tank_fill_blocker())
		return

	print("FILL_FEEDBACK_OK: SPACE and V both name what is missing (no building, or no stock) before ever reaching the sim, and both clear the moment the real fix (buying stock) is applied")
	quit(0)
