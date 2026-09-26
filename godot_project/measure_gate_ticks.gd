# HOW LONG A GATE TAKES, measured rather than guessed — a balance tool, not a
# test, and deliberately not named `test_*.gd` so `check-invariants.sh` does
# not demand a place for it in `verify-core-loop.sh`.
#
# Run with:
#   godot --headless --path godot_project --script measure_gate_ticks.gd
#
# WHY IT EXISTS. The gates' cost is the one thing about the 2026-09-26 trigger
# change that no test can pin: a test asserting "the mustard gate takes 181 000
# ticks" would fail on every balance pass, which is the opposite of useful. But
# an unmeasured claim in a PR body is worse — the first review of this change
# reproduced the ketchup figure and could not reproduce the mustard one,
# because every shipped fixture crosses that gate by patching the ledger
# (`Layouts.open_every_gate`) rather than earning it. This earns it.
#
# WHAT IT MEASURES. A line that never runs dry, never mis-builds and never goes
# broke — restocked and refunded every poll — so both numbers are a FLOOR, not
# a typical playthrough.
#
# WHAT IT PRINTED on 2026-09-27, at `data/unlocks.ron`'s shipped 100/100:
#
#   BBQ gate crossed at tick 67000 (detected on a 2 000-tick poll, 130 sold;
#     the hundredth box itself sells at tick ~52 000 — `test_layouts.gd`'s
#     `build_bbq_running_line` ceiling comment carries that tighter figure)
#   MUSTARD gate crossed at tick 181000 — 100.6 minutes at 30 ticks/s,
#     114 000 ticks (~63 min) after the BBQ gate
#
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

func _init() -> void:
	var main = preload("res://main.gd").new()
	get_root().add_child(main)
	await process_frame
	var sim = main.sim

	var t0 := Time.get_ticks_msec()
	Layouts.build_bbq_running_line(sim)
	if Layouts.fixture_failure() != "":
		print("MEASURE_FAIL: %s" % Layouts.fixture_failure())
		quit(1)
		return
	var economy = JSON.parse_string(sim.economy_snapshot_json())
	print("BBQ gate crossed at tick %d (wall %.1fs)" % [int(sim.current_tick()), (Time.get_ticks_msec() - t0) / 1000.0])
	for gate in economy.get("unlocks", []):
		print("  gate %s: item=%s sold=%d required=%d open=%s" % [
			gate.get("id", "?"), gate.get("item", "?"),
			int(gate.get("sold", 0)), int(gate.get("required", 0)), str(gate.get("open", false))])

	# Sign everything the board will now take, so BBQ demand exists at all.
	Layouts.fund(sim, 200000)
	for contract in JSON.parse_string(sim.economy_snapshot_json()).get("available_contracts", []):
		sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": contract.get("id", "")}))
	var signed: Array = JSON.parse_string(sim.economy_snapshot_json()).get("active_contracts", [])
	print("active contracts after signing: %d" % signed.size())

	var start_tick := int(sim.current_tick())
	var ceiling := 600000
	var crossed := -1
	while true:
		Layouts.restock(sim)
		Layouts.fund(sim, 200000)
		for _i in range(2000):
			sim.tick()
		var snap = JSON.parse_string(sim.economy_snapshot_json())
		var tick := int(sim.current_tick())
		var bbq_sold := 0
		var mustard_open := false
		for gate in snap.get("unlocks", []):
			if String(gate.get("item", "")) == "boxed_bbq":
				bbq_sold = int(gate.get("sold", 0))
				mustard_open = bool(gate.get("open", false))
		if tick % 20000 < 2000:
			print("  tick %d: boxed_bbq sold %d" % [tick, bbq_sold])
		if mustard_open:
			crossed = tick
			break
		if tick - start_tick > ceiling:
			print("MEASURE: ceiling %d ticks reached, boxed_bbq sold %d" % [ceiling, bbq_sold])
			break
	if crossed >= 0:
		print("MUSTARD gate crossed at tick %d (%.1f min at 30 ticks/s); %d ticks after the BBQ gate" % [
			crossed, crossed / 30.0 / 60.0, crossed - start_tick])
	print("MEASURE_DONE")
	quit(0)
