# Headless proof of the payout celebration — the "make earning money feel
# like something" ticket (docs/UI_UX.md §6, "juice stays legible over
# spectacle"; §8's audio direction, decided by `audio.gd`).
#
# Run with:
#   godot --headless --path godot_project --script test_payout_feedback.gd
#
# Like every other test_*.gd here, this deliberately does NOT claim to verify
# that the burst *looks* good — that needs eyes on
# `scripts/screenshot-scene.sh --payout` (UI_UX.md §2b point 12). What it does
# verify is everything a structural test can, and there is more of that than
# usual because the whole effect was deliberately built as a pure function of
# `sim.current_tick()` rather than of the wall clock:
#
#   * the coin pool is allocated ONCE and hangs off a scene-level node, so a
#     `_rebuild_world` (every place/remove/rotate) cannot blink a burst out,
#   * the first poll of a world SEEDS silently — loading a save whose
#     contracts have already delivered forty times must not fire forty
#     celebrations,
#   * a real running factory's real contract payout fires it, end to end
#     across the real `gdext` JSON boundary,
#   * a delta of two windows, and a delta across two contracts, each
#     celebrate ONCE for the summed amount (`MAX_TICKS_PER_FRAME` clamps
#     catch-up to 8 ticks, but a single frame can still cross two boundaries,
#     and a player who was paid once must not hear two chimes),
#   * every timed piece decays back to exactly nothing — coins invisible,
#     flyout transparent, money label back to scale 1, the docket's payout
#     figure back from PAID to its own number,
#   * the frequent moment (a case landing on the dock) is detected separately
#     and gets the small cue, not the burst,
#   * the audio is deterministic (an explicit LCG, not `randf()`) and the
#     mute switch says ON or OFF in words (UI_UX.md §7).
#
# Because a dummy audio driver makes "was a sound heard" unanswerable, the
# sound assertions run against `audio.gd`'s `plays` counter — the seam that
# exists for exactly this reason — plus the generated PCM itself.

extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## Generous: the probe that sized this found the first payout at tick 2500
## (the `diner_supply` docket). The ceiling is a multiple of that so a tuning
## change to a cadence or a process time moves the answer without breaking
## the test.
const TICK_BUDGET := 6000


func _fail(msg: String) -> void:
	printerr("PAYOUT_FEEDBACK_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# --- The pool exists, once, and in the right place --------------------
	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	var feedback: Node = main.get_node_or_null("Feedback")
	if feedback == null:
		_fail("no scene-level 'Feedback' node — the coin pool must not hang off _world_root")
		return
	# The crusher's spray shares this root (it outlives a rebuild for the same
	# reason a coin does), so Feedback holds the coins plus that one subtree.
	var crush_root: Node = feedback.get_node_or_null("CrushSpray")
	if crush_root == null:
		_fail("no 'CrushSpray' under Feedback — the crusher's pulp must survive _rebuild_world too")
		return
	if crush_root.get_child_count() != main.CRUSH_POOL:
		_fail("expected a fixed pool of %d crush bits, found %d children of CrushSpray"
			% [main.CRUSH_POOL, crush_root.get_child_count()])
		return
	if feedback.get_child_count() != main.COIN_POOL + 1:
		_fail("expected a fixed pool of %d coins beside CrushSpray, found %d children of Feedback"
			% [main.COIN_POOL, feedback.get_child_count()])
		return
	if main._coins.size() != main.COIN_POOL:
		_fail("_coins tracks %d entries but the pool is %d nodes"
			% [main._coins.size(), main.COIN_POOL])
		return
	for coin in main._coins:
		if (coin["node"] as Node3D).visible:
			_fail("a coin is visible before anything has been earned")
			return
		if int(coin["start_tick"]) >= 0:
			_fail("a coin is in flight before anything has been earned")
			return

	if main._payout_flyout == null:
		_fail("the '+N' flyout Label was not built")
		return
	if main._payout_flyout.get_parent() != hud:
		_fail("the flyout must be a direct child of the HUD, not nested in a panel")
		return
	if hud.get_child(hud.get_child_count() - 1) != main._payout_flyout:
		_fail("the flyout must be the LAST child of the HUD so it draws over every permanent panel")
		return
	if main._payout_flyout.modulate.a != 0.0:
		_fail("the flyout is visible before anything has been earned")
		return

	# --- A world to earn money in -----------------------------------------
	# The recipe the probe proved: the shipped line, funded, with BOTH raw
	# solids in the larder and the raw FLUID bought separately (stock_larder
	# only buys the hopper items, so a missing `vinegar` line stalls the mixer
	# silently and nothing ever reaches the dock), then both hoppers and the
	# tank switched to automatic so the line keeps running unattended.
	Layouts.build_reference_line(main.sim)
	Layouts.fund(main.sim, 30000)
	Layouts.stock_larder(main.sim, 1000)
	Layouts.stock(main.sim, "vinegar", 1000)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		main.sim.issue_command_json(JSON.stringify({
			"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true}))
	main.sim.issue_command_json(JSON.stringify({
		"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true}))
	main._rebuild_world()

	if main._dock_nodes.is_empty():
		_fail("the reference line has no dock — the burst would have nowhere to come from")
		return
	for key in main._dock_nodes.keys():
		if not (main._dock_nodes[key] as Dictionary).has("anchor"):
			_fail("dock %s has no 'anchor' — _burst_coins would throw its coins from the world origin" % key)
			return

	# Rebuilding the world must NOT take the pool with it: a player who
	# places a belt while a payout is in the air keeps their burst.
	if main.get_node_or_null("Feedback") == null:
		_fail("_rebuild_world destroyed the coin pool — it is parented to _world_root, not to the scene")
		return

	# --- The first poll seeds silently ------------------------------------
	main._update_contract_panel()
	if main.payout_events != 0:
		_fail("the first poll of a world celebrated %d payouts — loading a save with delivery history must be silent"
			% main.payout_events)
		return
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 0:
		_fail("the payout chime played on the first poll")
		return
	if main._payout_seen.is_empty():
		_fail("the first poll did not seed _payout_seen, so the NEXT poll would celebrate all history")
		return

	# --- A real factory, a real payout, end to end ------------------------
	var payout_tick := -1
	for i in range(TICK_BUDGET):
		main.sim.tick()
		# The two polls `_process` makes, in its order: the clipboard notices
		# payouts, the buffers notice cases arriving.
		main._update_contract_panel()
		main._refresh_buffers()
		if main.payout_events > 0:
			payout_tick = main.sim.current_tick()
			break
	if payout_tick < 0:
		_fail("no contract paid out within %d ticks — the fixture stopped supplying the factory" % TICK_BUDGET)
		return
	if main.payout_events != 1:
		_fail("one payment should be celebrated once, saw %d events" % main.payout_events)
		return

	# The amount celebrated is the paying docket's own payout, not a guess.
	# Read it back from the snapshot rather than hardcoding 90, so a
	# rebalance of contracts.ron moves the expectation with it.
	if main._payout_stamp_ticks.size() != 1:
		_fail("expected exactly one docket to be stamped PAID, %d were" % main._payout_stamp_ticks.size())
		return
	var paid_id := String(main._payout_stamp_ticks.keys()[0])
	var econ: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	var expected_payout := -1
	for entry in econ.get("contracts", []):
		if String((entry as Dictionary).get("id", "")) == paid_id:
			expected_payout = int(round(float(entry.get("payout", 0))))
	if expected_payout < 0:
		_fail("the stamped docket '%s' is not on the contract board at all" % paid_id)
		return
	if main.payout_total != expected_payout:
		_fail("celebrated %d but contract '%s' pays %d" % [main.payout_total, paid_id, expected_payout])
		return

	var coins_in_flight: int = main._active_coin_count()
	if coins_in_flight < main.COIN_MIN:
		_fail("a payout of %d launched only %d coins (COIN_MIN is %d)"
			% [main.payout_total, coins_in_flight, main.COIN_MIN])
		return
	if coins_in_flight > main.COIN_POOL:
		_fail("%d coins in flight from a pool of %d" % [coins_in_flight, main.COIN_POOL])
		return
	# Every coin leaves from the dock that paid, read off its own
	# `last_paid_tick` — the one dock of this line.
	var paying_docks: Array = main._docks_paid_since(payout_tick - 1)
	if paying_docks.size() != 1:
		_fail("one dock paid, but %d report paying on tick %d" % [paying_docks.size(), payout_tick])
		return
	var paid_anchor: Vector3 = main._dock_nodes[paying_docks[0]]["anchor"]
	for coin in main._coins:
		if int(coin["start_tick"]) >= 0 and (coin["origin"] as Vector3) != paid_anchor:
			_fail("a coin left from %s, not the paying dock at %s" % [coin["origin"], paid_anchor])
			return
	if main._payout_flyout.text != "+£%d" % expected_payout:
		_fail("the flyout reads '%s', expected '+£%d'" % [main._payout_flyout.text, expected_payout])
		return
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 1:
		_fail("expected exactly one payout chime, got %d"
			% int(main._audio.plays.get(main._audio.PAYOUT, 0)))
		return
	# A line that reached the dock must have knocked on the way.
	if int(main._audio.plays.get(main._audio.DELIVERY, 0)) < 1:
		_fail("no delivery cue fired, yet cases reached the dock")
		return

	# --- The PAID stamp, and its own reversal ------------------------------
	# On the plate's row, as its status...
	var paid_row: Dictionary = {}
	var paid_contract: Dictionary = {}
	for i in range(min(main._contract_cards.size(), main._contract_card_ids.size())):
		if String(main._contract_card_ids[i]) == paid_id:
			paid_row = main._contract_cards[i]
	for contract in JSON.parse_string(main.sim.economy_snapshot_json()).get("contracts", []):
		if String(contract["id"]) == paid_id:
			paid_contract = contract
	if paid_row.is_empty() or paid_contract.is_empty():
		_fail("the paying contract '%s' has no row on the contracts plate" % paid_id)
		return
	main._update_contract_row(paid_row, paid_contract, payout_tick)
	if not String(paid_row["status_label"].text).contains("PAID"):
		_fail("the paid row reads '%s', expected PAID" % paid_row["status_label"].text)
		return
	main._update_contract_row(paid_row, paid_contract, payout_tick + main.PAYOUT_STAMP_TICKS)
	if String(paid_row["status_label"].text).contains("PAID"):
		_fail("after %d ticks the row should stop reading PAID" % main.PAYOUT_STAMP_TICKS)
		return
	# ...and on the board's docket, as its payout figure.
	main._rebuild_board_active_cards(JSON.parse_string(main.sim.economy_snapshot_json()).get("contracts", []))
	var stamped_label: Label = null
	for i in range(min(main._board_active_cards.size(), main._board_active_ids.size())):
		if String(main._board_active_ids[i]) == paid_id:
			stamped_label = (main._board_active_cards[i] as Dictionary).get("payout_label")
	if stamped_label == null:
		_fail("the paying contract '%s' has no docket on the board" % paid_id)
		return
	main._update_payout_stamps(payout_tick)
	if stamped_label.text != "PAID":
		_fail("the paid docket's payout figure reads '%s', expected 'PAID'" % stamped_label.text)
		return
	main._update_payout_stamps(payout_tick + main.PAYOUT_STAMP_TICKS)
	if stamped_label.text != "+£%d" % expected_payout:
		_fail("after %d ticks the stamp should revert to '+%d', reads '%s'"
			% [main.PAYOUT_STAMP_TICKS, expected_payout, stamped_label.text])
		return
	if not main._payout_stamp_ticks.is_empty():
		_fail("an expired stamp was left in _payout_stamp_ticks — it would leak one entry per payout forever")
		return

	# --- Everything decays back to exactly nothing -------------------------
	# Driven by advancing the CLOCK, not by sleeping: that is the whole point
	# of anchoring the effect to `sim.current_tick()`.
	main._update_coins(payout_tick + main.COIN_FLIGHT_TICKS)
	if main._active_coin_count() != 0:
		_fail("%d coins were still in flight %d ticks after launch"
			% [main._active_coin_count(), main.COIN_FLIGHT_TICKS])
		return
	for coin in main._coins:
		if (coin["node"] as Node3D).visible:
			_fail("a coin node was left visible after its flight ended")
			return

	main._update_payout_flyout(payout_tick + main.PAYOUT_FLYOUT_TICKS)
	if main._payout_flyout.modulate.a != 0.0:
		_fail("the flyout is still visible %d ticks after the payout" % main.PAYOUT_FLYOUT_TICKS)
		return

	# Mid-flight first, and only then the settle: the punch RELEASES itself
	# (it sets `_money_punch_tick` back to -1 the moment it is done), so
	# asserting the settled state first would leave nothing to grow.
	main._update_money_punch(payout_tick + 1)
	if main._contract_money_label.scale.x <= 1.0:
		_fail("the money label never grew — the punch is invisible")
		return
	if main._contract_money_label.modulate == Color(1, 1, 1, 1):
		_fail("the money label never flashed — the punch is colour-free")
		return

	main._update_money_punch(payout_tick + main.MONEY_PUNCH_TICKS)
	if main._contract_money_label.scale != Vector2.ONE:
		_fail("the money label was left at scale %s instead of settling back to 1"
			% main._contract_money_label.scale)
		return
	if main._contract_money_label.modulate != Color(1, 1, 1, 1):
		_fail("the money label was left tinted %s instead of settling back to white"
			% main._contract_money_label.modulate)
		return

	# --- Two windows, and two contracts, are each ONE celebration ----------
	# `_detect_payouts` is a pure function of (snapshot, tick), so these are
	# driven with hand-made snapshots rather than by contriving a factory that
	# happens to cross two boundaries in one frame.
	# Since the contract board landed, only some contracts are ACTIVE and the
	# rest wait as offers (a new game signs none), so the two shapes this
	# case needs come from the active list first and the board behind it.
	# Only the id and the payout are read either way — the snapshots below
	# are hand-made, which is the whole point of the comment above.
	var board: Array = (econ.get("contracts", []) as Array).duplicate()
	for offer in econ.get("board", []):
		if not bool((offer as Dictionary).get("active", false)):
			board.append(offer)
	if board.size() < 2:
		_fail("contracts.ron has fewer than two contracts and no unsigned offers, so the multi-contract case cannot be tested")
		return

	main._payout_seen = {}
	main.payout_events = 0
	main.payout_total = 0
	main._audio.plays.clear()

	var first_id := String((board[0] as Dictionary).get("id", ""))
	var second_id := String((board[1] as Dictionary).get("id", ""))
	var first_pay := int(round(float((board[0] as Dictionary).get("payout", 0))))
	var second_pay := int(round(float((board[1] as Dictionary).get("payout", 0))))

	main._detect_payouts(_board([[first_id, first_pay, 0], [second_id, second_pay, 0]]), 100)
	if main.payout_events != 0:
		_fail("re-seeding an empty _payout_seen celebrated something")
		return

	# Two windows of one contract.
	main._detect_payouts(_board([[first_id, first_pay, 2], [second_id, second_pay, 0]]), 101)
	if main.payout_events != 1:
		_fail("a two-window delta fired %d times, expected one celebration" % main.payout_events)
		return
	if main.payout_total != 2 * first_pay:
		_fail("a two-window delta celebrated %d, expected %d" % [main.payout_total, 2 * first_pay])
		return
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 1:
		_fail("a two-window delta played the chime twice")
		return

	# Two contracts settling on the same tick.
	main._detect_payouts(_board([[first_id, first_pay, 3], [second_id, second_pay, 1]]), 102)
	if main.payout_events != 2:
		_fail("a two-contract delta should be the second celebration, events is %d" % main.payout_events)
		return
	if main.payout_total != 2 * first_pay + first_pay + second_pay:
		_fail("a two-contract delta celebrated %d in total, expected %d"
			% [main.payout_total, 2 * first_pay + first_pay + second_pay])
		return
	if main._payout_stamp_ticks.size() != 2:
		_fail("two contracts paid but %d dockets were stamped" % main._payout_stamp_ticks.size())
		return
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 2:
		_fail("two celebrations played %d chimes"
			% int(main._audio.plays.get(main._audio.PAYOUT, 0)))
		return

	# A board that does not move is silent.
	var before: int = main.payout_events
	main._detect_payouts(_board([[first_id, first_pay, 3], [second_id, second_pay, 1]]), 103)
	if main.payout_events != before:
		_fail("an unchanged contract board celebrated a payout")
		return

	# A contract appearing for the first time is seeded, not treated as
	# having just paid `deliveries_made` times.
	main._detect_payouts(_board([
		[first_id, first_pay, 3], [second_id, second_pay, 1], ["newcomer", 500, 40]]), 104)
	if main.payout_events != before:
		_fail("a contract seen for the first time with 40 deliveries behind it celebrated a payout")
		return

	# --- Coins leave from the docks that paid, and from no other ----------
	# The owner's report of 2026-09-26: a payout threw coins over EVERY dock.
	# A second dock is stood beside the line's one, by hand, so the two-dock
	# case needs no second line to be built.
	var real_dock: String = main._dock_nodes.keys()[0]
	var idle_anchor := Vector3(40.0, 0.0, 40.0)
	main._dock_nodes["99,99"] = {"anchor": idle_anchor, "slots": []}
	var saved_states: Dictionary = main._dock_states
	main._dock_states = {
		real_dock: {"last_paid_tick": 300},
		"99,99": {"last_paid_tick": 120},
	}
	var paid_now: Array = main._docks_paid_since(299)
	if paid_now != [real_dock]:
		_fail("only the dock that paid on tick 300 paid since 299, got %s" % [paid_now])
		return
	main._update_coins(1 << 20) # every coin still in flight lands
	main._burst_coins(first_pay, 300, paid_now)
	if main._active_coin_count() < main.COIN_MIN:
		_fail("the paying dock threw only %d coins" % main._active_coin_count())
		return
	for coin in main._coins:
		if int(coin["start_tick"]) >= 0 and (coin["origin"] as Vector3) == idle_anchor:
			_fail("a dock that gave nothing to the payment threw a coin")
			return
	main._update_coins(1 << 20)
	main._burst_coins(first_pay, 301, [])
	if main._active_coin_count() != 0:
		_fail("a payment no dock paid threw %d coins from somewhere" % main._active_coin_count())
		return
	main._dock_nodes.erase("99,99")

	# --- The frequent moment: a case landing on a dock ---------------------
	# Per dock, off each dock's own `shipped_total` — a case arriving at one
	# dock pops that dock and no other.
	main._audio.plays.clear()
	main._dock_received_seen = {}
	main._dock_pops = {}
	main._dock_states = {
		"5,0": {"shipped_total": 7, "used": 2},
		"9,0": {"shipped_total": 4, "used": 1},
	}
	main._detect_dock_arrivals(200)
	if int(main._audio.plays.get(main._audio.DELIVERY, 0)) != 0:
		_fail("the first dock poll knocked — a loaded save with a full dock must be silent")
		return
	main._dock_states["5,0"] = {"shipped_total": 8, "used": 3}
	main._detect_dock_arrivals(201)
	if int(main._audio.plays.get(main._audio.DELIVERY, 0)) != 1:
		_fail("a case arriving did not fire the delivery cue")
		return
	if int(main._dock_pops.get("5,0", {}).get("slot", -1)) != 2:
		_fail("the third case should pop slot index 2 of its dock, pops are %s" % main._dock_pops)
		return
	if main._dock_pops.has("9,0"):
		_fail("a case arriving at one dock popped another")
		return
	# A contract consuming a dock's stock is not an arrival.
	main._dock_states["5,0"] = {"shipped_total": 8, "used": 1}
	main._detect_dock_arrivals(202)
	if int(main._audio.plays.get(main._audio.DELIVERY, 0)) != 1:
		_fail("a dock's stock going DOWN fired a delivery cue")
		return
	# A dock placed after the first poll is seeded, not knocked.
	main._dock_states["12,0"] = {"shipped_total": 30, "used": 30}
	main._detect_dock_arrivals(203)
	if int(main._audio.plays.get(main._audio.DELIVERY, 0)) != 1:
		_fail("a dock seen for the first time knocked for its whole history")
		return
	# A dock arrival must never fire the big celebration.
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 0:
		_fail("a case landing on the dock played the payout chime")
		return
	main._update_dock_pop(201 + main.DOCK_POP_TICKS)
	if not main._dock_pops.is_empty():
		_fail("the dock pop did not finish and clear itself")
		return
	main._dock_states = saved_states

	# --- The sound itself --------------------------------------------------
	var stream_a: AudioStreamWAV = main.FactoryAudio.payout_stream()
	var stream_b: AudioStreamWAV = main.FactoryAudio.payout_stream()
	if stream_a.data != stream_b.data:
		_fail("two payout streams differ — the synthesis is not deterministic (a stray randf()?)")
		return
	if stream_a.data.is_empty():
		_fail("the payout stream is silent")
		return
	if stream_a.format != AudioStreamWAV.FORMAT_16_BITS or stream_a.stereo:
		_fail("expected 16-bit mono PCM")
		return
	var delivery: AudioStreamWAV = main.FactoryAudio.delivery_stream()
	if delivery.data.size() >= stream_a.data.size():
		_fail("the delivery knock is not shorter than the payout chime — it fires far more often")
		return
	if _peak(delivery.data) >= _peak(stream_a.data):
		_fail("the delivery knock is not quieter than the payout chime")
		return

	# The mute switch says which way it is set, in words (UI_UX.md §7). It is
	# the SOUND row of the settings screen now, reached from the pause menu.
	var sound_switch: Button = main._pause_settings_panel.value_controls.get("sfx_enabled")
	if sound_switch == null:
		_fail("the settings screen has no SOUND switch")
		return
	if sound_switch.text != "SOUND: ON":
		_fail("the sound switch starts reading '%s'" % sound_switch.text)
		return
	sound_switch.pressed.emit()
	if main._audio.enabled:
		_fail("pressing the sound switch did not mute")
		return
	if sound_switch.text != "SOUND: OFF":
		_fail("a muted game's switch reads '%s'" % sound_switch.text)
		return
	# Muting must not change bookkeeping, only whether a stream starts.
	main._audio.plays.clear()
	main._audio.play(main._audio.PAYOUT)
	if int(main._audio.plays.get(main._audio.PAYOUT, 0)) != 1:
		_fail("muting stopped play() bookkeeping, so mute can change timing or state")
		return
	sound_switch.pressed.emit()
	if not main._audio.enabled or sound_switch.text != "SOUND: ON":
		_fail("the sound switch does not toggle back on")
		return

	print("PAYOUT_FEEDBACK_OK: first payout at tick %d paid %d by '%s' -> %d coins, '+%d', one chime; "
		% [payout_tick, expected_payout, paid_id, coins_in_flight, expected_payout]
		+ "seeded silently, multi-window and multi-contract deltas celebrate once, "
		+ "everything decays to nothing, coins leave only the docks that paid, "
		+ "dock arrivals pop only their own dock, audio is deterministic")
	quit(0)


## A contracts array in the shape `economy_snapshot_json` reports, from
## `[id, payout, deliveries_made]` triples.
func _board(rows: Array) -> Array:
	var out: Array = []
	for row in rows:
		out.append({"id": row[0], "payout": float(row[1]), "deliveries_made": row[2]})
	return out


## The loudest sample in a 16-bit PCM buffer, as a fraction of full scale.
func _peak(data: PackedByteArray) -> float:
	var highest := 0
	for i in range(0, data.size(), 2):
		highest = maxi(highest, absi(data.decode_s16(i)))
	return float(highest) / 32767.0
