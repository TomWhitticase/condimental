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
# a typical playthrough.
#
# WHAT IT PRINTED on 2026-10-02, at `data/unlocks.ron`'s shipped 100/100,
# with a case of eight bottles and the doubled 36-offer contract board (the
# run before the board doubled, 15 of 17 offers signed and 13 active, printed
# 131000 / 72.8 min / 64 000 / 79 000; the 2026-09-29 run, at one bottle a
# case, printed 155000 / 86.1 min / 88 000 / 103 000):
#
#   BBQ fixture handed back at tick 67000, 142 boxes of ketchup sold — the tick
#     `build_bbq_running_line` RETURNS at, not the tick the gate opened: it
#     spots the open gate on a 250-tick poll at about tick 52 000, then builds
#     the BBQ branch and runs `BBQ_SETTLE_TICKS` (15 000) more before handing
#     back, and the ketchup line keeps selling throughout
#   active contracts after signing 34 of 36 offers: 25
#   MUSTARD gate crossed at tick 125000 — 69.4 minutes at 30 ticks/s,
#     58 000 ticks after the line above; the BBQ gate itself opened at tick
#     52 000, so 73 000 ticks (~41 min) of the run follow it
#
# It printed 141000 until 2026-09-29 (PR #209), when `build_bbq_running_line`
# stopped signing `smokehouse_trial`: the hand-off now carries only 8 boxes
# of BBQ sold, so this number moves with what that fixture signs during its
# settle.
#
# IT PRINTED 181000 UNTIL 2026-09-27, and that number is in no file any more:
# the signing step read `available_contracts`/`active_contracts`, which are not
# the wire's keys, so `.get(…, [])` handed it an empty array, nothing was
# signed and only the two orders `build_bbq_running_line` signs for itself were
# ever asking for BBQ. It reported "active contracts after signing: 0" while
# two were running (then `smokehouse_trial` and `grill_pub`), which is what gave the
# mistake away. Hence the two
# refusals below: a measuring tool that skips a step silently is worse than no
# tool.
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
	# NOT the tick the gate opened: the fixture spots it on a 250-tick poll and
	# then builds the BBQ branch and settles `BBQ_SETTLE_TICKS` more, so this is
	# the hand-off and the `sold` count below is already past 100.
	print("BBQ fixture handed back at tick %d, %d ticks after its gate opened (wall %.1fs)" % [
		int(sim.current_tick()), Layouts.BBQ_SETTLE_TICKS, (Time.get_ticks_msec() - t0) / 1000.0])
	for gate in economy.get("unlocks", []):
		print("  gate %s: item=%s sold=%d required=%d open=%s" % [
			gate.get("id", "?"), gate.get("item", "?"),
			int(gate.get("sold", 0)), int(gate.get("required", 0)), str(gate.get("open", false))])

	# Sign everything the board will now take, so the whole of the authored BBQ
	# demand is actually running — not just the two orders the BBQ fixture
	# signs for itself. The keys are `board` (every offer) and `contracts`
	# (the active ones), which is what `main.gd` reads; an earlier cut of this
	# tool asked for `available_contracts`/`active_contracts`, got the `.get`
	# default `[]` for both, signed nothing and reported "0 active" while two
	# orders were running. A silently-skipped step in a measuring tool is worse
	# than no tool, so this one REFUSES rather than measuring the wrong world.
	Layouts.fund(sim, 200000)
	var board: Array = JSON.parse_string(sim.economy_snapshot_json()).get("board", [])
	if board.is_empty():
		print("MEASURE_FAIL: the board is empty, so `board` is not the wire's key for it any more")
		quit(1)
		return
	var asked := 0
	for offer in board:
		if bool(offer.get("active", false)):
			continue
		asked += 1
		sim.issue_command_json(JSON.stringify({"type": "sign_contract", "id": offer.get("id", "")}))
	var signed: Array = JSON.parse_string(sim.economy_snapshot_json()).get("contracts", [])
	var names: Array[String] = []
	for contract in signed:
		names.append(String(contract.get("id", "?")))
	print("active contracts after signing %d of %d offers: %d (%s)" % [
		asked, board.size(), signed.size(), ", ".join(names)])
	if signed.is_empty():
		print("MEASURE_FAIL: nothing is signed, so nothing orders BBQ and the second gate cannot open")
		quit(1)
		return

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
		print("MUSTARD gate crossed at tick %d (%.1f min at 30 ticks/s); %d ticks after this measurement started at the fixture's hand-off" % [
			crossed, crossed / 30.0 / 60.0, crossed - start_tick])
	print("MEASURE_DONE")
	quit(0)
