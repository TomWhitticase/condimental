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


## The luma term's weight, and the floor, for telling two hopper heaps apart.
## Same metric as `test_fluid_gallery.gd`'s `FLUID_TINT_MIN_DISTANCE`:
## `sum |dRGB| + HOPPER_FILL_TINT_LUMA_WEIGHT * |dLuma|`. Summed |dRGB| alone
## cannot tell "these two differ in brightness" from "these two differ in hue
## at the same brightness", and only the first is an axis the renderer reads
## by, so a metric without the luma term is blind on the axis that matters.
##
## Issue #80 added `spice_mix` and `molasses` here and a guard that every raw
## a hopper accepts HAS an entry. Nothing checked the entries against each
## other, which is the same failure mode one level up: two raws could be
## given the same colour, or near enough, and every test would stay green
## (issue #115).
##
## Measured on the shipped four: the worst pair is `tomato` <-> `spice_mix`
## at **0.347**, then `tomato` <-> `molasses` at 1.138 and `spice_mix` <->
## `molasses` at 1.243; `sugar` against anything is 2.9 and up. **0.31 is a
## ~10% margin below the worst pair**, the same deliberately-tight rule
## `test_fluid_gallery.gd` is calibrated by, so a fifth raw landing between
## 0.31 and 0.35 is one a human should look at.
##
## **The floor sitting below a pair a ticket reports as too close is a
## record, not an endorsement.** `tomato` <-> `spice_mix` is that pair here:
## rendered on `--bbq --no-hud` at 1600x900 their heap tops are 142.4 and
## 155.0 luma, **12.7/255 apart**, against the ~26/255 anchor for "reads
## reliably". They separate on hue instead — red against orange, R-G of 116
## against 41 — and in the shipped fixture they sit at opposite ends of the
## line, so the side-by-side case is the one nobody has looked at. Fixing it
## is an art change and belongs in `palette.ron`, not here: `spice_paprika`
## is the substance's own colour and the palette's rule is one colour per
## substance everywhere it appears, so moving it for the bin would split it
## from the market crate (issue #115).
const HOPPER_FILL_TINT_LUMA_WEIGHT := 2.0
const HOPPER_FILL_TINT_MIN_DISTANCE := 0.31


## Rec. 709 luminance, the same weights as the legibility measurements in
## `docs/DECISIONS.md` and as `test_fluid_gallery.gd`, so a number in a
## failure message here and a number in a rendered-frame mean are the same
## thing.
func _luma(c: Color) -> float:
	return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b


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
##
## Read through the scene's own `_hopper_items` rather than off the JSON:
## since issue #90 each entry on the wire is a CHOICE record carrying its
## gate, and `_refresh_hopper_items` is the one place that unpacks it.
func _is_fluid(id: String) -> bool:
	return not _raw_solids().has(id)


## The raw solid ids, unpacked from the wire's contents choices. Gated ones
## are included — a padlocked ingredient still needs a model and a tint, and
## is in fact the one most likely to be missing them.
func _raw_solids() -> Array:
	_main._refresh_hopper_items()
	return _main._hopper_items


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
	# the loop above. `hopper_items` is `World::raw_items` and keeps the ids a
	# gate holds back, flagged rather than dropped (issue #90), so this reads
	# the same four ids on a fresh world as on a finished one — the gates
	# crossed above are for the ingredient entries, not for this.
	var hopper_items: Array = _raw_solids()
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

	# Every entry against every other, not just presence. Keyed off
	# `hopper_items` rather than the table's own keys so a tint left behind
	# for an ingredient the sim no longer offers cannot fail the build.
	var seen := {}
	var worst_distance := INF
	var worst_pair := ["", ""]
	for item in hopper_items:
		var raw_id := String(item)
		var tint: Color = _main.HOPPER_FILL_TINTS[raw_id]
		for other_id in seen:
			var other: Color = seen[other_id]
			var chroma: float = (
				absf(tint.r - other.r) + absf(tint.g - other.g) + absf(tint.b - other.b)
			)
			var value: float = absf(_luma(tint) - _luma(other))
			var distance: float = chroma + HOPPER_FILL_TINT_LUMA_WEIGHT * value
			if distance < HOPPER_FILL_TINT_MIN_DISTANCE:
				_fail(
					"raw solids '%s' and '%s' are tinted too close to be told apart in a hopper (distance %.3f, floor %.2f) — sum |dRGB| %.3f plus %.1f x |dLuma| %.3f. A player looking at two bins side by side sees one substance. Move them apart in BRIGHTNESS for preference, since that is the axis the heap's lit top face reads by; a pair separated on hue alone can clear this floor and still read the same at play zoom (issue #115). The colours belong to assets/voxels/palette.ron, so move them there and let the bin and the market crate travel together." % [
						raw_id,
						other_id,
						distance,
						HOPPER_FILL_TINT_MIN_DISTANCE,
						chroma,
						HOPPER_FILL_TINT_LUMA_WEIGHT,
						value,
					]
				)
				return
			if distance < worst_distance:
				worst_distance = distance
				worst_pair = [other_id, raw_id]
		seen[raw_id] = tint

	print("INGREDIENT_MODELS_OK: all %d ingredients (%s) resolve to a real model icon, none to the pipe fallback; all %d raw solids a hopper accepts (%s) have a fill tint of their own, none the fallback, and no two of those tints are closer than %.2f (worst pair %s <-> %s at %.3f)" % [
		ids.size(),
		", ".join(ids),
		hopper_items.size(),
		", ".join(PackedStringArray(hopper_items)),
		HOPPER_FILL_TINT_MIN_DISTANCE,
		worst_pair[0],
		worst_pair[1],
		worst_distance,
	])
	quit(0)
