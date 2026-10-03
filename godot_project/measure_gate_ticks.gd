# HOW LONG A GATE TAKES, measured rather than guessed — a balance tool, not a
# test, and deliberately not named `test_*.gd` so `check-invariants.sh` does
# not demand a place for it in `verify-core-loop.sh`.
#
# Run with:
#   godot --headless --path godot_project --script measure_gate_ticks.gd
#
# WHY IT EXISTS. The gates' cost is the one thing about the 2026-09-26 trigger
# change that no test can pin: a test asserting "the mustard gate takes 155 000
# ticks" would fail on every balance pass, which is the opposite of useful. But
# an unmeasured claim in a PR body is worse — the first review of this change
# reproduced the ketchup figure and could not reproduce the mustard one,
# because every shipped fixture crosses that gate by patching the ledger
# (`Layouts.open_every_gate`) rather than earning it. This earns it.
#
# WHAT IT MEASURES. A line that never runs dry, never mis-builds and never goes
# broke — restocked and refunded every poll — so both numbers are a FLOOR, not
# a typical playthrough. Since 2026-10-03 the order is ketchup, mustard, BBQ:
# the ketchup line (`build_reference_line`) runs alone until a hundred boxes
# of ketchup open MUSTARD; the mustard line (`build_mustard_line`, on its own
# rows, gate earned rather than patched) is then built beside it, every order
# the board will take is signed, and the factory runs until a hundred boxes
# of mustard open BBQ.
#
# The history of what it printed under the old order (ketchup, BBQ, mustard)
# is in `data/unlocks.ron`'s notes and in this file's git history; the
# current numbers are in `data/unlocks.ron`, beside the gates they measure.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

func _init() -> void:
	var main = preload("res://main.gd").new()
	get_root().add_child(main)
	await process_frame
	var sim = main.sim
	var t0 := Time.get_ticks_msec()

	# 1. The ketchup line alone, kept fed, crewed and funded.
	Layouts.build_reference_line(sim)
	Layouts.fund(sim, 200000)
	Layouts.stock_larder(sim, Layouts.LARDER_STOCK)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true,
		}))
	sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true,
	}))
	Layouts.hire_until_staffed(sim)
	var first := _run_until_open(sim, "mustard_sauce", 300000)
	if first < 0:
		print("MEASURE_FAIL: the ketchup line never opened the mustard gate")
		quit(1)
		return
	print("MUSTARD gate opened at tick %d (%.1f min at 30 ticks/s; wall %.1fs)" % [
		first, first / 30.0 / 60.0, (Time.get_ticks_msec() - t0) / 1000.0])

	# 2. The mustard line beside it, its gate EARNED (open_gates = false).
	Layouts.build_mustard_line(sim, false)
	if Layouts.fixture_failure() != "":
		print("MEASURE_FAIL: %s" % Layouts.fixture_failure())
		quit(1)
		return
	var handed := int(sim.current_tick())
	print("mustard line built and settled, handed back at tick %d" % handed)

	# 3. Sign everything the board will now take. The keys are `board` and
	# `contracts`; a tool that asked for the wrong keys once signed nothing
	# and measured the wrong world, so this one REFUSES an empty answer.
	Layouts.fund(sim, 200000)
	var board: Array = JSON.parse_string(sim.economy_snapshot_json()).get("board", [])
	if board.is_empty():
		print("MEASURE_FAIL: the board is empty, so `board` is not the wire's key for it any more")
		quit(1)
		return
	for offer in board:
		if not bool(offer.get("active", false)):
			sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": offer.get("id", "")}))
	var signed: Array = JSON.parse_string(sim.economy_snapshot_json()).get("contracts", [])
	print("active contracts: %d of %d offers" % [signed.size(), board.size()])
	if signed.is_empty():
		print("MEASURE_FAIL: nothing is signed, so nothing orders mustard and the BBQ gate cannot open")
		quit(1)
		return

	var second := _run_until_open(sim, "bbq_sauce", 600000)
	if second < 0:
		print("MEASURE: the BBQ gate did not open within the ceiling")
	else:
		print("BBQ gate opened at tick %d (%.1f min at 30 ticks/s); %d ticks after the mustard gate, %d after the mustard line's hand-off" % [
			second, second / 30.0 / 60.0, second - first, second - handed])
	print("MEASURE_DONE")
	quit(0)


## Ticks the factory (restocked and refunded every poll) until gate `id` is
## open, and returns the tick it was seen open, or -1 past `ceiling`.
func _run_until_open(sim, id: String, ceiling: int) -> int:
	var start := int(sim.current_tick())
	while int(sim.current_tick()) - start < ceiling:
		if Layouts.unlock_open(sim, id):
			return int(sim.current_tick())
		Layouts.restock(sim)
		Layouts.fund(sim, 200000)
		for _i in range(250):
			sim.tick()
	return -1
