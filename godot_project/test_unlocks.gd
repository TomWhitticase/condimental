# Headless structural proof of the first progression gate (the BBQ epic,
# ticket 3; GAMEPLAY.md §6, UI_UX.md §5/§7, `data/unlocks.ron`).
#
# Run with:
#   godot --headless --path godot_project --script test_unlocks.gd
#
# `sim_core/tests/unlocks.rs` owns the rule itself — the counter, the
# threshold, the refusals, the save round trip — earned by running the real
# chain. This file owns the presentation half, and only that: given a
# simulation on either side of the gate, does the HUD show what the wire
# says? It proves, through the same entry points a real session reaches:
#
#   * a new game's market and larder list only the ungated ingredients, and
#     the board only the ungated offers — no panel re-derives the filter,
#     they simply draw what `ingredients_snapshot_json` and
#     `economy_snapshot_json` gave them;
#   * crossing the threshold GROWS all three lists in place, so a panel
#     built at `_ready()` does not stay three crates wide forever
#     (`_market_slots_match` / `_larder_rows_match` / `_board_rows_match`);
#   * the crossing is announced once, on the existing one-line notice, in
#     words that name the tier and both halves of the game that changed;
#   * a save loaded already past the gate is NOT announced — the priming
#     rule the contract notice follows, for the same reason.
extends SceneTree

const Layouts := preload("res://test_layouts.gd")

## Read from the wire rather than written out: this file must not be a second
## copy of `data/unlocks.ron`.
var _gate: Dictionary = {}


func _fail(msg: String) -> void:
	printerr("UNLOCKS_FAIL: %s" % msg)
	quit(1)


func _econ(main) -> Dictionary:
	return JSON.parse_string(main.sim.economy_snapshot_json())


func _market_ids(main) -> Array:
	var out: Array = []
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		out.append(String(entry.get("id", "")))
	return out


func _board_ids(main) -> Array:
	var out: Array = []
	for offer in _econ(main).get("board", []):
		out.append(String(offer.get("id", "")))
	return out


## Everything the HUD has to repoll after the simulation moved — the same
## three calls `_poll_*` makes every frame, so nothing here is a state a
## real session could not reach.
func _repoll(main) -> void:
	main._update_contract_panel()
	main._update_larder()
	main._update_ingredient_market()
	main._update_contract_board()


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	# --- The gate is on the wire, resolved by the simulation --------------
	var unlocks: Array = _econ(main).get("unlocks", [])
	if unlocks.is_empty():
		_fail("economy_snapshot_json carried no `unlocks` — the HUD has nothing to announce")
		return
	_gate = unlocks[0]
	for field in ["id", "name", "open", "deliveries", "after_deliveries", "ingredients", "contracts"]:
		if not _gate.has(field):
			_fail("an unlock snapshot must carry '%s', got %s" % [field, str(_gate.keys())])
			return
	var threshold: int = int(_gate["after_deliveries"])
	if threshold < 1:
		_fail("a gate open in a new game is not a gate (after_deliveries = %d)" % threshold)
		return
	if bool(_gate["open"]) or int(_gate["deliveries"]) != 0:
		_fail("a new game must be on the closed side of the gate")
		return

	# --- Closed: three lists, all short -----------------------------------
	var locked_market := _market_ids(main)
	var locked_board := _board_ids(main)
	var locked_larder: Array = main._larder_ids.duplicate()
	if locked_larder != locked_market:
		_fail("the larder must list exactly the market's ingredients, got %s vs %s" % [
			str(locked_larder), str(locked_market)])
		return
	if main._market_slots.size() != locked_market.size():
		_fail("expected %d market crate(s), found %d" % [locked_market.size(), main._market_slots.size()])
		return
	if main._board_rows.size() != locked_board.size():
		_fail("expected %d board row(s), found %d" % [locked_board.size(), main._board_rows.size()])
		return

	# The notice starts empty, and the first poll only primes the diff.
	_repoll(main)
	if main._known_unlocks_primed != true:
		_fail("the first poll must prime the unlock diff")
		return
	if main._contract_notice.text.contains("unlocked"):
		_fail("a new game must not announce a gate it has not crossed: '%s'" % main._contract_notice.text)
		return

	# --- Cross it ---------------------------------------------------------
	# The counter moves under a HUD that has already polled once, which is
	# how a playing session reaches it: the gate opens on a delivery.
	Layouts.deliver(main.sim, threshold)
	_repoll(main)
	if not bool(_econ(main)["unlocks"][0]["open"]):
		_fail("the gate must be open at %d deliveries" % threshold)
		return

	var open_market := _market_ids(main)
	var open_board := _board_ids(main)
	if open_market.size() <= locked_market.size():
		_fail("crossing the gate must put ingredients on sale, %s -> %s" % [
			str(locked_market), str(open_market)])
		return
	if open_board.size() <= locked_board.size():
		_fail("crossing the gate must put offers on the board, %d -> %d" % [
			locked_board.size(), open_board.size()])
		return
	if int(_gate["ingredients"]) != open_market.size() - locked_market.size():
		_fail("the gate says it releases %d ingredient(s), the market grew by %d" % [
			int(_gate["ingredients"]), open_market.size() - locked_market.size()])
		return
	if int(_gate["contracts"]) != open_board.size() - locked_board.size():
		_fail("the gate says it releases %d contract(s), the board grew by %d" % [
			int(_gate["contracts"]), open_board.size() - locked_board.size()])
		return

	# --- The panels grew with it, in place --------------------------------
	if main._market_slots.size() != open_market.size():
		_fail("the market panel still shows %d crate(s) for %d ingredient(s) — the slot guard did not fire" % [
			main._market_slots.size(), open_market.size()])
		return
	for i in range(open_market.size()):
		if String(main._market_slots[i]["id"]) != open_market[i]:
			_fail("market crate %d is '%s', the wire's entry %d is '%s'" % [
				i, main._market_slots[i]["id"], i, open_market[i]])
			return
	if main._larder_ids != open_market:
		_fail("the larder still lists %s for a market of %s" % [str(main._larder_ids), str(open_market)])
		return
	if main._board_rows.size() != open_board.size():
		_fail("the board still shows %d row(s) for %d offer(s)" % [
			main._board_rows.size(), open_board.size()])
		return
	for i in range(open_board.size()):
		if String(main._board_rows[i]["id"]) != open_board[i]:
			_fail("board row %d is '%s', the wire's offer %d is '%s'" % [
				i, main._board_rows[i]["id"], i, open_board[i]])
			return

	# --- ...and it was announced, once, in words --------------------------
	var notice: String = String(main._contract_notice.text)
	if not notice.contains(String(_gate["name"])) or not notice.contains("unlocked"):
		_fail("the crossing must be announced by name, got '%s'" % notice)
		return
	for half in ["new ingredients in the shop", "new orders on the board"]:
		if not notice.contains(half):
			_fail("the announcement must say '%s', got '%s'" % [half, notice])
			return
	var announced_once := notice
	_repoll(main)
	_repoll(main)
	if String(main._contract_notice.text) != announced_once:
		_fail("a gate already open must not be announced again on every poll")
		return

	# --- A save loaded past the gate is not announced ----------------------
	# Loading a save builds the scene afresh, and `_build_contract_panel`
	# clears exactly these two — so a session that opens a world already
	# past the gate primes its diff with the tier open and says nothing.
	main._known_open_unlock_ids.clear()
	main._known_unlocks_primed = false
	main._contract_notice.text = ""
	main._contract_notice_queue.clear()
	_repoll(main)
	if main._contract_notice.text.contains("unlocked"):
		_fail("loading a save already past the gate must not re-announce it, got '%s'" % main._contract_notice.text)
		return
	if _market_ids(main).size() != open_market.size():
		_fail("a save loaded past the gate must still have the tier on sale")
		return

	print("UNLOCKS_OK: '%s' gates %d ingredient(s) and %d contract(s) behind %d deliveries; a new game lists %d/%d market, %d/%d board, crossing grows market, larder and board in place and is announced once by name; a save already past it is not re-announced" % [
		String(_gate["name"]), int(_gate["ingredients"]), int(_gate["contracts"]), threshold,
		locked_market.size(), open_market.size(), locked_board.size(), open_board.size()])
	quit(0)
