# Headless proof that EVERY raw ingredient in `data/ingredients.ron` has a
# voxel model of its own — issue #67.
#
# Run with:
#   godot --headless --path godot_project --script test_ingredient_models.gd
#
# This exists because of the exact failure it now catches. `main.gd`'s
# `_market_asset_for` ends in `return PIPE_ASSET`, a documented fallback of
# the kind UI_UX.md §2b point 12 requires — and a fallback of that kind is
# silent by construction. When the BBQ branch added `spice_mix`, `molasses`
# and `liquid_smoke` to `data/ingredients.ron`, all three fell straight
# through it: the market offered three different shipments as three
# identical pipe nozzles, a fluid tank titled "Liquid Smoke" was captioned
# with a picture of a pipe, and every test in the suite stayed green. A
# fallback is the right thing to have and the wrong thing to rely on, so the
# fallback stays and this test asserts nothing reaches it.
#
# Verifies:
#   * every id the simulation reports as an ingredient resolves through
#     ITEM_MODELS or FLUID_MODELS, never through the pipe fallback,
#   * the model each one names is a real asset with a loadable icon, not a
#     key pointing at nothing (a typo'd value is as silent as a missing one),
#   * a fluid's entry lives in FLUID_MODELS and a solid's in ITEM_MODELS —
#     the split those two tables document, since no belt can carry a fluid,
#   * every raw solid a hopper may legally be SET TO has a HOPPER_FILL_TINTS
#     entry (issue #80). That is the same silent per-id fallback one table
#     over: `HOPPER_FILL_TINT_FALLBACK` is a beige, so `spice_mix` and
#     `molasses` shipped as two different raws drawn the same pale sand.
#     The set is the simulation's own `hopper_items` — the list the placed
#     hopper's picker is built from — and never a second hand-written one,
#     because a guard that has to be updated by hand is the bug rather than
#     the fix (the ruling PR #84's review made, recorded in issue #108).
#
# **It crosses the unlock gates first.** `ingredients_snapshot_json` is
# filtered by what the player has unlocked, so on a fresh world it reports
# only the ketchup raws and the three ids this test was written for would be
# invisible to it. `Layouts.deliver` patches the delivery counter through the
# save door the way `test_screenshot.gd`'s `_cross_gates` does.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var _main


func _fail(msg: String) -> void:
	printerr("INGREDIENT_MODELS_FAIL: %s" % msg)
	quit(1)


## Every ingredient id, with the gates crossed so the late ones are in it.
func _all_ingredient_ids() -> Array:
	var econ: Dictionary = JSON.parse_string(_main.sim.economy_snapshot_json())
	var threshold := 0
	for unlock in econ.get("unlocks", []):
		threshold = maxi(threshold, int(unlock.get("after_deliveries", 0)))
	if threshold == 0:
		_fail("data/unlocks.ron gates nothing — this test cannot tell locked from unlocked")
		return []
	Layouts.deliver(_main.sim, threshold)

	var snapshot: Dictionary = _main._ingredients_snapshot()
	var ids := []
	for entry in snapshot.get("entries", []):
		var id := String(entry.get("id", ""))
		if id != "":
			ids.append(id)
	return ids


## Is this id a fluid? The simulation already answers it: `hopper_items` is
## `World::raw_items`, the raw SOLIDS a hopper may legally hold, so a raw
## ingredient absent from that list is a fluid. Asking the sim beats a
## hardcoded list here, which would be the same staleness one layer down.
func _is_fluid(id: String) -> bool:
	var hopper_items: Array = _main._ingredients_snapshot().get("hopper_items", [])
	return not hopper_items.has(id)


func _initialize() -> void:
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	_main._ready()

	var ids := _all_ingredient_ids()
	if ids.is_empty():
		_fail("no ingredients in the snapshot at all — the gates did not open")
		return

	for id in ids:
		var asset: String = _main._market_asset_for(id)
		if asset == _main.PIPE_ASSET:
			_fail("ingredient '%s' has no model — it falls through _market_asset_for to '%s'. Add it to ITEM_MODELS (solids) or FLUID_MODELS (fluids) in main.gd and author the generator in assets/voxels/generators/assets.rs." % [
				id, _main.PIPE_ASSET,
			])
			return

		var fluid := _is_fluid(id)
		var table := "FLUID_MODELS" if fluid else "ITEM_MODELS"
		var in_fluids: bool = _main.FLUID_MODELS.has(id)
		var in_items: bool = _main.ITEM_MODELS.has(id)
		if fluid and not in_fluids:
			_fail("'%s' is a fluid but its model is registered in ITEM_MODELS — no belt can carry it, so it belongs in %s" % [id, table])
			return
		if not fluid and not in_items:
			_fail("'%s' rides a belt but its model is only in FLUID_MODELS — it belongs in %s" % [id, table])
			return

		# A key pointing at an asset that does not exist is exactly as
		# silent as no key at all: `_icon_for` would hand back the 8x8
		# placeholder and the market would still draw *something*.
		var icon: Texture2D = _main._icon_for(id)
		if icon == null or icon.get_width() < 16 or icon.get_height() < 16:
			_fail("ingredient '%s' maps to asset '%s' but its icon is missing or is the placeholder (%s) — check the name is registered in the ASSETS table and that assets/voxels/icons/%s.png is committed" % [
				id,
				asset,
				"null" if icon == null else "%dx%d" % [icon.get_width(), icon.get_height()],
				asset,
			])
			return

	# The hopper's fill tint, from the same source the GAME asks. `ITEM_MODELS`
	# above is keyed by every item a BELT can carry; this table is keyed only
	# by what a hopper can be set to hold, which is a strictly smaller set and
	# a different question, so it gets its own pass rather than a branch in
	# the loop above. `hopper_items` is `World::raw_items` and is deliberately
	# NOT filtered by the unlock gates (see its field doc), so this reads the
	# same four ids on a fresh world as on a finished one — the gates crossed
	# above are for the ingredient entries, not for this.
	var hopper_items: Array = _main._ingredients_snapshot().get("hopper_items", [])
	if hopper_items.is_empty():
		_fail("the simulation offers a hopper NO contents at all — this check would be vacuous")
		return
	for item in hopper_items:
		var raw_id := String(item)
		if not _main.HOPPER_FILL_TINTS.has(raw_id):
			_fail("raw solid '%s' can be put in a hopper but has no HOPPER_FILL_TINTS entry — its heap draws in HOPPER_FILL_TINT_FALLBACK %s, so it is the same pale sand as every other untinted raw. Add it to HOPPER_FILL_TINTS in main.gd, using that ingredient's own colour from assets/voxels/palette.ron." % [
				raw_id, str(_main.HOPPER_FILL_TINT_FALLBACK),
			])
			return

	print("INGREDIENT_MODELS_OK: all %d ingredients (%s) resolve to a real model icon, none to the pipe fallback; all %d raw solids a hopper accepts (%s) have a fill tint of their own, none the fallback" % [
		ids.size(), ", ".join(ids), hopper_items.size(), ", ".join(PackedStringArray(hopper_items)),
	])
	quit(0)
