# Headless proof of the fluid gallery (issue #72) — the structural claims a
# machine can check without a framebuffer. Whether the nine colours actually
# READ is what `scripts/screenshot-scene.sh --fluid-gallery` is for, the same
# division `test_pipe_shapes.gd` and `test_belt_shapes.gd` draw for their own
# galleries.
#
# Run with:
#   godot --headless --path godot_project --script test_fluid_gallery.gd
#
# The fixture exists because `bbq` and `bbq_mix` stood at zero pressure in
# every other fixture, so the tints issue #68 gave them could not be
# photographed at all. It is a claim about the SIMULATION as much as about a
# frame — the six runs are named by topology, not by a label the fixture
# writes — and every way it can degrade is quiet: move one pipe and two
# networks merge into a single unnameable run, add a machine and a run goes
# ambiguous, get the fill wrong and a run is named but invisible. In each
# case the render still comes back a perfectly good-looking factory, missing
# one colour the reader was told it held. Verifies:
#   * the checker is not vacuous: an EMPTY world is refused before anything
#     is placed, so a silent "" later means something,
#   * every placement the gallery asks for is accepted (a refused purchase
#     would drop a run and nothing else would say so),
#   * `check_fluid_gallery` passes on the shipped layout: nine runs, each
#     `carries` with one fluid named, each above the pressure whose colour
#     shows in a sight window,
#   * the nine named fluids are exactly `FLUID_GALLERY_FLUIDS`,
#   * `test_layouts.gd`'s hand-copied `FLUID_MIN_VISIBLE_PRESSURE` still
#     equals `main.gd`'s, the number it was copied from,
#   * every one of the nine has a `FLUID_TINTS` entry, no two alike, and none
#     of them near-grey (issue #100 — see `FLUID_TINT_MIN_SATURATION`),
#   * and, the other way round, every key in `FLUID_TINTS` is IN
#     `FLUID_GALLERY_FLUIDS` (issue #108). Without that the three checks
#     above police a hand-written list rather than the table they are
#     about, so a tenth fluid ships an unphotographed, unmeasured tint with
#     `FLUID_GALLERY_OK` and exit 0 — proved on `559641b` by adding one. A
#     guard that has merely been given more entries, rather than made to
#     refuse, has not been fixed; that ruling came out of PR #84's review and
#     is what `test_ingredient_models.gd` already does for the model tables.
#   * and — the check the two above only LOOK like — every fluid the
#     SIMULATION carries has a `FLUID_TINTS` row, and every recipe a machine
#     that shows its product can run has a `PRODUCT_TINTS` row, both ways
#     round. `FLUID_GALLERY_FLUIDS` and `FLUID_TINTS` are both hand-written
#     GDScript, so checking them against each other cannot see a fluid
#     missing from both: mustard's `mustard_mash`, `mustard_mix` and
#     `mustard` reached a review drawing in `FLUID_TINT_FALLBACK`, and its
#     three products in `PRODUCT_TINT_FALLBACK`, with this file green. The
#     expected set now comes off `cookbook_json` and off the machines' own
#     generated models — see `_wire_fluids()` and `_wire_products()` — so a
#     fourth sauce is covered by the content landing and nothing else.
#   * every DRAWN pipe body, after two polls, carries its own fluid's tint.
#     The checks above read `FLUID_TINTS`, not the materials on screen, and
#     since `_fluid_material_for` caches one material per fluid, a wrong
#     cache key would draw every run one colour with all of them green
#     (#220's review). That one material per fluid is SHARED is
#     `test_fluids.gd`'s check, not this one's.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## How much chroma a fluid tint must carry. A fluid body is drawn inside a
## grey steel bore, so a near-grey tint is the one colour that cannot be seen
## in a pipe however its value is tuned: `liquid_smoke` shipped at
## (0.36, 0.32, 0.26), saturation 0.278, and measured 11 luma from the steel
## either side of it — a full run nobody could find by eye at 1280x720 while
## the other five were obvious (issue #100). The floor sits above that and
## below the five that always read; it is a floor on SATURATION rather than
## on distance-from-steel because the failure is chromatic, not tonal —
## `vinegar` is no darker than the old smoke was and has never been hard to
## see.
const FLUID_TINT_MIN_SATURATION := 0.35

## How much a luma difference counts for, against the same magnitude of
## difference spread over R, G and B. It is >1 because this camera reads by
## VALUE contrast rather than by colour (`assets/voxels/CLAUDE.md`, UI_UX §7),
## so a pair that moves in brightness is further apart to a player than a
## pair that merely moves round the hue wheel by the same amount. Two is the
## weight at which the measured worst pair below stops being the one summed
## |dRGB| alone names, which is the whole reason the term is here.
const FLUID_TINT_LUMA_WEIGHT := 2.0

## How far apart two tints have to be to count as different liquids, on
## `sum |dRGB| + FLUID_TINT_LUMA_WEIGHT * |dLuma|`.
##
## **It used to be 0.08 on summed |dRGB| alone, and that number was
## calibrated against nothing** (issue #108). The shipped six's worst pair on
## that metric is 0.148, so the floor passed a pair at half the worst case
## the art had ever been reviewed at, and — the real defect — summed |dRGB|
## cannot tell "these two differ in brightness" from "these two differ in hue
## at the same brightness". Only the first of those is the axis the renderer
## reads by, so the metric was blind on the axis that matters and raising it
## would only have moved where it was blind.
##
## Measured on the shipped six with the luma term in: the worst pair is now
## `ketchup_mix` <-> `bbq_mix` at **0.291**, then `bbq_mix` <-> `liquid_smoke`
## at 0.412 and `vinegar` <-> `liquid_smoke` at 0.452; everything else is
## 0.45 and up. **0.26 is a 10% margin below the worst pair** — deliberately
## tight, where the old 0.08's was 46%, because a seventh fluid landing
## between 0.26 and 0.29 is one a human should look at rather than one a
## constant should wave through.
##
## It was 0.24 below a worst pair of 0.269, which was `bbq_mix` <->
## `liquid_smoke` — the value collision issue #113 reported. That is fixed
## rather than tolerated now: `smoke_slate` was lifted from #4D4521 to
## #706430 in `palette.ron`, taking the pair from 6.2/255 of tint luma to
## 24.4/255, so the floor could follow the art up. See `main.gd`'s
## `FLUID_TINTS["liquid_smoke"]` for why the move is a lift and not a
## darkening.
##
## **The floor sitting below a pair a ticket reports as too close is a
## record that somebody should look, not an endorsement of it.** The pair
## that inherits that role is `ketchup_mix` <-> `bbq_mix`: 0.291 overall but
## only 7.7/255 of luma between them, so they are told apart by hue much as
## the old pair was. Red against brown is a wider hue gap than terracotta
## against olive and both are mid-chain fluids that never share a frame in
## the shipped fixtures, which is why this is filed rather than fixed here.
##
## **Re-measured over NINE when mustard landed, and the floor did not move.**
## The three new tints are a value ladder on one hue, so their own two gaps
## and the gap from `vinegar` to `mustard_mash` all come out at 0.432-0.450 —
## the third, fourth and fifth worst pairs in the table, and every one of
## them above `bbq_mix` <-> `liquid_smoke`'s 0.412. The worst pair is still
## `ketchup_mix` <-> `bbq_mix` at 0.291, so 0.26's 10% margin is still a
## margin below the same pair it was calibrated against. A third sauce that
## had needed this number raised would have been a third sauce whose colours
## were picked badly.
##
## `test_bbq_branch.gd` keeps its own, looser 0.08 on plain summed |dRGB|,
## and the two are deliberately NOT the same number any more: that one is
## documented as a copied-entry catcher over the four runs its fixture
## names, this one is a calibrated judgement over every tint the game has.
const FLUID_TINT_MIN_DISTANCE := 0.26

## The material class a generator emits for a machine's product mesh
## (`machine_product` in `assets/voxels/generators/assets.rs`). Its presence
## on a machine's model is how section 5c decides that machine shows what it
## is making, instead of holding a list of the kinds that do.
const PRODUCT_MESH_CLASS := "liquid"

## A floor on how many piped ids `_wire_fluids()` may find before section 5b
## calls the derivation broken rather than the tables complete. Three is
## structural and not a count of the shipped content: one sauce is a mix, a
## cook, and at least one raw fluid bought to make them, so anything less
## means the walk is not reaching the recipe graph at all.
##
## The real non-vacuity guard is the loop UNDER it, which walks `FLUID_TINTS`
## and demands the wire knows each key: an empty derivation fails there
## whatever this number says. This exists so that it fails saying "the
## derivation is broken" rather than "vinegar is not a fluid".
const WIRE_FLUIDS_FLOOR := 3

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("FLUID_GALLERY_FAIL: %s" % msg)
	quit(1)


## Rec. 709 relative luminance, the same weights the legibility measurements
## in `docs/DECISIONS.md` are taken with, so the number in a failure message
## and the number in a rendered-frame review mean the same thing.
func _luma(c: Color) -> float:
	return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b


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
	# --- 1. the checker refuses an empty world ----------------------------
	# Run before anything is placed, so that the "" further down is evidence
	# rather than a default. A check that cannot fail is the thing this whole
	# fixture exists to avoid.
	if Layouts.check_fluid_gallery(main.sim) == "":
		_fail(
			"check_fluid_gallery() passed a world with nothing in it — the check is vacuous, "
			+ "so its silence on the real gallery would mean nothing"
		)
		return

	# --- 2. the gallery builds, and says so itself ------------------------
	var complaint: String = Layouts.build_fluid_gallery(main.sim)
	if complaint != "":
		_fail("build_fluid_gallery() refused its own layout: %s" % complaint)
		return
	Layouts.stock_larder(main.sim)
	main._rebuild_world()
	main._update_fluid_network()

	var layout: Array = JSON.parse_string(main.sim.world_layout_json())
	# `line_placements`, not the layout's size: stocking a larder now builds
	# warehouse racks, and they are the fixture's storage rather than part of
	# the gallery being counted.
	var placed := Layouts.line_placements(main.sim)
	if placed != Layouts.FLUID_GALLERY_PLACEMENTS:
		_fail(
			"the gallery placed %d things, expected %d — a refused placement, and a run short"
			% [placed, Layouts.FLUID_GALLERY_PLACEMENTS]
		)
		return

	# --- 3. nine runs, each carrying one of the nine ----------------------
	var snapshots: Array = JSON.parse_string(main.sim.fluid_snapshots_json())
	var carried := {}
	for run in snapshots:
		carried[String(run.get("fluid", ""))] = true
	var expected := {}
	for fluid in Layouts.FLUID_GALLERY_FLUIDS:
		expected[fluid] = true
	if carried.keys().size() != expected.keys().size():
		_fail(
			"the gallery's runs carry %s, and the fixture is written against %s"
			% [str(carried.keys()), str(expected.keys())]
		)
		return
	for fluid in expected:
		if not carried.has(fluid):
			_fail("no run carries '%s'" % fluid)
			return

	# --- 4. the hand-copied floor has not drifted -------------------------
	# `test_layouts.gd` restates `main.gd`'s number because its helpers are
	# static and have no scene to ask. Two copies of a constant drift
	# silently in the direction that matters: if main's floor RISES, the
	# gallery's check goes quietly weaker rather than louder.
	if not is_equal_approx(Layouts.FLUID_MIN_VISIBLE_PRESSURE, main.FLUID_MIN_VISIBLE_PRESSURE):
		_fail(
			"test_layouts.gd's FLUID_MIN_VISIBLE_PRESSURE is %s and main.gd's is %s — "
			% [Layouts.FLUID_MIN_VISIBLE_PRESSURE, main.FLUID_MIN_VISIBLE_PRESSURE]
			+ "the copy has drifted, and the gallery is now checked against the wrong floor"
		)
		return

	# --- 5. the gallery's hand-list covers the whole table ----------------
	# Everything from here down walks `FLUID_GALLERY_FLUIDS`, so on its own it
	# is a check on a list rather than on `FLUID_TINTS`. This loop is what
	# ties the two together: a fluid cannot get a tint without also being
	# stood in the frame that proves the tint reads (issue #108).
	for fluid in main.FLUID_TINTS:
		if not Layouts.FLUID_GALLERY_FLUIDS.has(fluid):
			_fail(
				"'%s' has a FLUID_TINTS entry but the gallery never stands it in a frame, " % fluid
				+ "so nothing below can see it: add it to test_layouts.gd's "
				+ "FLUID_GALLERY_FLUIDS and give it a run in build_fluid_gallery()"
			)
			return

	# --- 5b. and the whole table is derived from the SIMULATION -----------
	# Sections 4 and 5 check `FLUID_GALLERY_FLUIDS` against `FLUID_TINTS` in
	# both directions, which sounds complete and is not: both are
	# hand-written GDScript, so a fluid absent from BOTH is invisible to
	# them. That is how mustard's three stages reached a review drawing in
	# `FLUID_TINT_FALLBACK` with every script in the suite green. The set
	# below is read off `cookbook_json` — the simulation's own recipe graph,
	# which is `data/*.ron` and never a list here — so a fourth sauce's
	# fluids arrive in this check the moment the content does.
	var wire_fluids := _wire_fluids()
	if wire_fluids.size() < WIRE_FLUIDS_FLOOR:
		_fail(
			"cookbook_json names only %d piped id(s) (%s) — the shipped content has at least %d, "
			% [wire_fluids.size(), ", ".join(wire_fluids.keys()), WIRE_FLUIDS_FLOOR]
			+ "so the derivation is broken and every check below it is vacuous"
		)
		return
	for fluid in wire_fluids:
		if not main.FLUID_TINTS.has(fluid):
			_fail(
				"'%s' flows down a pipe in the content (%s) but has no FLUID_TINTS entry in "
				% [fluid, wire_fluids[fluid]]
				+ "main.gd — it draws in FLUID_TINT_FALLBACK %s, the neutral pale blue, so a "
				% [str(main.FLUID_TINT_FALLBACK)]
				+ "whole run of it reads as water. Add the row, then a run in "
				+ "build_fluid_gallery() so the colour is photographed as well as declared."
			)
			return
	for fluid in main.FLUID_TINTS:
		if not wire_fluids.has(fluid):
			_fail(
				"FLUID_TINTS has a row for '%s', which no recipe in the content carries — " % fluid
				+ "either the key is a typo (and the fluid it was meant for is falling through "
				+ "the fallback unnoticed) or the row outlived its sauce"
			)
			return

	# --- 5c. and so is PRODUCT_TINTS --------------------------------------
	# The same table one surface over: a machine whose model carries a
	# `liquid` material class SHOWS what it is making, tinted by the recipe
	# id. Which machines those are is read off the generated models rather
	# than listed here, and which recipes they run is read off the wire, so
	# neither side of this is a copy of the other.
	var wire_products := _wire_products()
	if wire_products.is_empty():
		_fail(
			"no machine in the content both shows a product and runs a recipe — either "
			+ "assets/voxels stopped emitting the '%s' class or cookbook_json stopped " % PRODUCT_MESH_CLASS
			+ "naming the machine each step runs on; either way this check now proves nothing"
		)
		return
	for recipe in wire_products:
		if not main.PRODUCT_TINTS.has(recipe):
			_fail(
				"the %s runs '%s' and shows its product, but '%s' has no PRODUCT_TINTS entry "
				% [wire_products[recipe], recipe, recipe]
				+ "in main.gd — the mesh draws in PRODUCT_TINT_FALLBACK %s, so the sauce "
				% [str(main.PRODUCT_TINT_FALLBACK)]
				+ "standing in that machine reads as white and unmade"
			)
			return
	for recipe in main.PRODUCT_TINTS:
		if not wire_products.has(recipe):
			_fail(
				"PRODUCT_TINTS has a row for '%s', which is not a recipe run by any machine " % recipe
				+ "that shows its product — a key nothing reads, and if it is a typo the "
				+ "recipe it was meant for is drawing in the fallback"
			)
			return

	# --- 6. nine tints, all different, none of them grey ------------------
	var seen := {}
	for fluid in Layouts.FLUID_GALLERY_FLUIDS:
		if not main.FLUID_TINTS.has(fluid):
			_fail(
				"'%s' has no FLUID_TINTS entry — it renders in the neutral fallback, " % fluid
				+ "which is the silent-fallback failure issue #68 closed"
			)
			return
		var tint: Color = main.FLUID_TINTS[fluid]
		if tint.s < FLUID_TINT_MIN_SATURATION:
			_fail(
				"'%s' is tinted (%.3f, %.3f, %.3f), saturation %.3f — below the %.2f floor, " % [
					fluid, tint.r, tint.g, tint.b, tint.s, FLUID_TINT_MIN_SATURATION
				]
				+ "so it reads as an empty steel pipe rather than a full one (issue #100)"
			)
			return
		for other_id in seen:
			var other: Color = seen[other_id]
			var chroma: float = (
				absf(tint.r - other.r) + absf(tint.g - other.g) + absf(tint.b - other.b)
			)
			var value: float = absf(_luma(tint) - _luma(other))
			var distance: float = chroma + FLUID_TINT_LUMA_WEIGHT * value
			if distance < FLUID_TINT_MIN_DISTANCE:
				_fail(
					"'%s' and '%s' are tinted the same liquid (distance %.3f, of which "
					% [fluid, other_id, distance]
					+ "%.3f is summed |dRGB| and %.3f is |dLuma| x %s) — the gallery stands "
					% [chroma, value, FLUID_TINT_LUMA_WEIGHT * value, FLUID_TINT_LUMA_WEIGHT]
					+ "them in one frame, so two of its runs would be one liquid. Move them "
					+ "apart in BRIGHTNESS by preference: this camera reads by value, and a "
					+ "pair separated by hue alone clears the floor while still reading as "
					+ "one colour at play zoom (issue #108)."
				)
				return
		seen[fluid] = tint

	# --- 7. the DRAWN liquid is each run's own tint ----------------------
	# The checks above read `FLUID_TINTS`, not what the pipes are drawn with.
	# Pipe bodies share one cached material per fluid (`_fluid_material_for`),
	# so a wrong cache key would draw every run one colour with all of the
	# above still green. Poll twice, as the frame loop does, then read every
	# body's material back.
	main._update_fluid_network()
	main._update_fluid_network()
	var materials := {} # fluid id -> a material its bodies draw with
	for n in range(main._last_fluid.size()):
		var network: Dictionary = main._last_fluid[n]
		var fluid: String = main._fluid_id_of(network)
		var tint: Color = main.FLUID_TINTS.get(fluid, main.FLUID_TINT_FALLBACK)
		for s in range(network.get("segments", []).size()):
			var body: Node = main._pipe_fluid_nodes.get("%d:%d" % [n, s], null)
			if body == null:
				continue
			for child in body.get_children():
				if not (child is MeshInstance3D):
					continue
				var drawn: StandardMaterial3D = child.get_surface_override_material(0)
				if drawn == null or not drawn.albedo_color.is_equal_approx(tint):
					_fail(
						"a '%s' pipe body is drawn %s, not its own tint %s"
						% [fluid, str(drawn.albedo_color if drawn != null else null), str(tint)]
					)
					return
				materials[fluid] = drawn
	if materials.size() != expected.size():
		_fail(
			"liquid is drawn for %s, but the gallery's runs carry %s"
			% [str(materials.keys()), str(expected.keys())]
		)
		return

	print(
		"FLUID_GALLERY_OK: %d placements stand %d runs, each named by topology alone, each above "
		% [placed, snapshots.size()]
		+ "the %s pressure at which a sight window shows colour, each with a tint of its own "
		% [Layouts.CROSSING_MIN_DRAWN_PRESSURE]
		+ "(%s); every fluid " % [", ".join(Layouts.FLUID_GALLERY_FLUIDS)]
		+ "the CONTENT carries (%d, off cookbook_json) has a FLUID_TINTS row and every row has a "
		% [wire_fluids.size()]
		+ "fluid, and every recipe the %d product-showing machine kind(s) run (%d recipes, off "
		% [_product_machines(wire_products).size(), wire_products.size()]
		+ "the '%s' class in their own models) has a PRODUCT_TINTS row and every row has a "
		% PRODUCT_MESH_CLASS
		+ "recipe — and the check refuses an empty world, so that is a pass and not a default"
	)
	quit(0)


## Every id the content moves down a PIPE, as `id -> where it was seen`.
##
## Off `cookbook_json`, which is `data/recipes.ron` and `data/ingredients.ron`
## read back: a step that hands on a fluid names it in `carries` with
## `carries_fluid` set, and a raw fluid a sauce buys is an input with `fluid`
## set. Locked families are in the book too, so this needs no gate crossed —
## which matters, because the fluids a fresh save has never heard of are
## exactly the ones a hand-written list forgets.
func _wire_fluids() -> Dictionary:
	var found := {}
	var book = JSON.parse_string(main.sim.cookbook_json())
	if not (book is Dictionary):
		return found
	for sauce in book.get("sauces", []):
		for step in sauce.get("steps", []):
			if bool(step.get("carries_fluid", false)):
				var carried := String(step.get("carries", ""))
				if carried != "":
					found[carried] = "out of the %s" % String(step.get("machine", "?"))
		for input in sauce.get("inputs", []):
			if bool(input.get("fluid", false)):
				var id := String(input.get("id", ""))
				if id != "":
					found[id] = "bought as a raw for %s" % String(sauce.get("id", "?"))
	return found


## Every recipe run by a machine that SHOWS its product, as
## `recipe -> machine`.
##
## Both halves are derived. The recipes and the machine each runs on come off
## `cookbook_json`; whether that machine shows anything comes off the
## machine's own generated model, which splits into material classes and
## emits a `liquid` one only for the kinds whose generator called
## `machine_product` (`assets/voxels/generators/assets.rs`). So adding a
## product mesh to the crusher would widen this check on its own, and a
## GDScript list of "the machines that show a product" — which is what this
## would otherwise need — cannot go stale because there is not one.
func _wire_products() -> Dictionary:
	var found := {}
	var book = JSON.parse_string(main.sim.cookbook_json())
	if not (book is Dictionary):
		return found
	var shows := {}
	for sauce in book.get("sauces", []):
		for step in sauce.get("steps", []):
			var machine := String(step.get("machine", ""))
			var recipe := String(step.get("recipe", ""))
			if machine == "" or recipe == "":
				continue
			if not shows.has(machine):
				var asset: String = main._shop_asset_for(machine)
				shows[machine] = main._asset_class_names(asset).has(PRODUCT_MESH_CLASS)
			if shows[machine]:
				found[recipe] = machine
	return found


## The distinct machine kinds in a `_wire_products()` result, for the count
## in the success line. Named rather than inlined so the line says how many
## KINDS show a product as well as how many recipes they run.
func _product_machines(products: Dictionary) -> Dictionary:
	var kinds := {}
	for recipe in products:
		kinds[products[recipe]] = true
	return kinds
