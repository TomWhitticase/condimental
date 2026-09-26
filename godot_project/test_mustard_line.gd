# Headless proof of THE MUSTARD LINE — the third sauce, running end to end,
# and every id it puts on screen resolving to a model or a tint of its own
# rather than to a documented fallback.
#
# Run with:
#   godot --headless --path godot_project --script test_mustard_line.gd
#
# WHY IT EXISTS. Mustard is the first sauce that is a whole SECOND LINE from
# raw rather than a branch off an existing one, and it is the first whose
# middle is entirely WET: the crusher mills the seed with vinegar and
# discharges a fluid, so `mustard_mash` and `mustard_mix` never touch a belt.
# Between them those two facts mean the third sauce touches five lookup
# tables in `main.gd` — `ITEM_MODELS`, `FLUID_TINTS`, `PRODUCT_TINTS`,
# `HOPPER_FILL_TINTS`, `RACK_LOAD_TINTS` — and every one of them fails
# SILENTLY. A missing key is a cardboard box sliding down a belt, a pale blue
# pipe, an unfinished-looking vat or a pallet of the same sand as every other
# pallet, with the whole suite green (UI_UX.md §2b point 12).
#
# Verifies:
#   * THE LINE IS BUILT: every placement `build_mustard_line` asks for is
#     accepted (a refused purchase is silent, and would leave a gap nothing
#     else reports),
#   * THE PLUMBING IS NAMED: all four runs `carries` the fluid the layout
#     implies, and the mash is on the crusher's side of the mixer while the
#     mix is on the other. An unnamed run is not an error in `sim_core` — its
#     machines simply never draw — so it has to be asserted somewhere or it
#     photographs as a line that is idle for no stated reason,
#   * IT ACTUALLY RUNS: `boxed_mustard` is produced AND delivered. A line
#     that is plumbed correctly and starved looks identical in a screenshot,
#   * EVERY MUSTARD ID RESOLVES. Enumerated from the SIMULATION — the sauce's
#     own page in `cookbook_json`, which is `data/*.ron` as the game reads it
#     — and never from a list written here. That is the ruling PR #84's
#     review made and issue #108 recorded: a guard that can only be given
#     more entries has not been fixed. Add a fourth sauce and this test asks
#     about its ids without being edited,
#   * THE THREE FLUIDS ARE TELLABLE APART ALONG THE LINE, by the measure the
#     rest of the repo uses — see `MUSTARD_LUMA_STEP_MIN` below,
#   * AND THE VAT AGREES WITH ITS OWN PIPE: `PRODUCT_TINTS["mustard"]` equals
#     `FLUID_TINTS["mustard"]`, one cook and one colour, the rule
#     `test_bbq_branch.gd` pins for the other two sauces.
#
# The one-frame-before-checking dance is inherited from test_hover.gd — see
# that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## How far apart, in Rec.709 luma on 0..1, two CONSECUTIVE stages of the
## mustard run have to be drawn.
##
## This is the only thing in the suite that asserts mustard's three fluids
## are three fluids rather than one. `test_fluid_gallery.gd` already holds
## every pair in the game to a combined chroma-and-value floor, which the
## mustard trio clears at 0.450 — but that metric is dominated by summed
## |dRGB|, and three tints of ONE HUE can clear it while being a single
## colour to a player looking down a pipe. Value is the axis this camera
## reads by (UI_UX.md §7), and along a single-hue ladder it is the ONLY axis,
## so it gets a floor of its own.
##
## **This floor is in TINT space and the frame is what matters**, so it is
## set from a rendered measurement rather than from the tints. The pipe
## body's lit face maps tint luma to pixels compressively (the curve is
## written out in `main.gd`'s FLUID_TINTS block): the shipped ladder is
## 0.1354 then 0.1586 of tint luma — 34.5/255 and 40.4/255 — and
## photographs as **+18.2/255 and +14.1/255**, measured on a fixed 32x16
## mask over `--fluid-gallery --populate --closeup <tile>,5`. An earlier
## cut with EVEN 25.6/255 tint steps photographed as +15.3 and +10.4, and
## 10.4 is inside issue #113's 10.7-11.8 noise band for reading one fluid
## twice. So 0.120 is roughly a 10% margin under the smaller shipped step,
## the same deliberately-tight calibration `test_fluid_gallery.gd` and
## `test_ingredient_models.gd` are set by — and it is a floor a headless
## test CAN hold, not a claim about pixels it cannot see. A re-tint landing
## between 0.120 and 0.135 is one a human should re-photograph.
const MUSTARD_LUMA_STEP_MIN := 0.120

## The smallest icon that is not the 8x8 placeholder `_icon_for` hands back
## for a name no asset answers to. Same threshold, and the same reasoning, as
## `test_ingredient_models.gd`: a key pointing at an asset that does not
## exist is exactly as silent as no key at all.
const ICON_MIN_PIXELS := 16

var main: Node
var _ran := false
## kind id -> the tile that kind's single instance stands on.
var _kind_tiles: Dictionary = {}


func _fail(msg: String) -> void:
	printerr("MUSTARD_FAIL: %s" % msg)
	quit(1)


## Rec.709 relative luminance, the weights every legibility measurement in
## this repo is taken with (`docs/DECISIONS.md`, `test_fluid_gallery.gd`), so
## a number in a failure message here and a number read off a rendered frame
## mean the same thing.
func _luma(c: Color) -> float:
	return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	# The fixture goes down in `_run_checks`, not here: `_ready` runs twice
	# under this harness and each run creates a fresh (empty) world, so
	# anything built now is wiped before the checks see it.


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


func _fluid_on(tile: Vector2i) -> String:
	for network in JSON.parse_string(main.sim.fluid_snapshots_json()):
		for segment in network.get("segments", []):
			if int(segment["tile"][0]) == tile.x and int(segment["tile"][1]) == tile.y:
				return String(network.get("fluid", ""))
	return ""


func _naming_on(tile: Vector2i) -> String:
	for network in JSON.parse_string(main.sim.fluid_snapshots_json()):
		for segment in network.get("segments", []):
			if int(segment["tile"][0]) == tile.x and int(segment["tile"][1]) == tile.y:
				return String(network.get("naming", ""))
	return ""


func _made(item_id: String) -> int:
	for entry in JSON.parse_string(main.sim.production_snapshot_json()).get("items", []):
		if String(entry.get("id", "")) == item_id:
			return int(entry.get("produced_total", 0))
	return 0


func _delivered(item_id: String) -> int:
	for entry in JSON.parse_string(main.sim.economy_snapshot_json()).get("delivery_stock", []):
		if String(entry[0]) == item_id:
			return int(entry[1])
	return 0


## The sauce page `cookbook_json` holds for `id`, or an empty Dictionary.
func _cookbook_sauce(id: String) -> Dictionary:
	for sauce in JSON.parse_string(main.sim.cookbook_json()).get("sauces", []):
		if String(sauce.get("id", "")) == id:
			return sauce
	return {}


## Does this tile's machine have a `liquid` child mesh — i.e. does it SHOW
## its product? Asked of the built scene rather than answered from a list of
## kinds here, because the answer is a property of the voxel models
## (`machine_product` in assets/voxels/generators/assets.rs) and a generator
## change is exactly the thing a hardcoded list would miss.
func _shows_product(tile: Vector2i) -> bool:
	return main._machine_products.get(main._tile_key(tile), null) != null


func _run_checks() -> void:
	Layouts.build_mustard_line(main.sim)
	if Layouts.fixture_failure() != "":
		_fail("the fixture refused to build: %s" % Layouts.fixture_failure())
		return
	main._rebuild_world()

	# --- 1. the whole line is on the floor --------------------------------
	var layout: Array = JSON.parse_string(main.sim.world_layout_json())
	var line_placed: int = Layouts.line_placements(main.sim)
	if line_placed != Layouts.MUSTARD_LINE_PLACEMENTS:
		_fail(
			"the line placed %d things, expected %d — a refused purchase is silent, so a stage is missing"
			% [line_placed, Layouts.MUSTARD_LINE_PLACEMENTS]
		)
		return
	for entry in layout:
		var kind := String(entry.get("kind", ""))
		var origin = entry.get("tile", null)
		if origin == null:
			continue
		if not _kind_tiles.has(kind):
			_kind_tiles[kind] = Vector2i(int(origin[0]), int(origin[1]))
	for pair in [
		[Layouts.MUSTARD_CRUSHER, "crusher"], [Layouts.MUSTARD_MIXER, "mixer"],
		[Layouts.MUSTARD_VAT, "cooking_vat"], [Layouts.MUSTARD_BOTTLER, "bottler"],
		[Layouts.MUSTARD_PACKER, "packer"], [Layouts.MUSTARD_DOCK, "dock"],
	]:
		var kind_here := String(main._placement_at_tile(pair[0]).get("kind", ""))
		if kind_here != pair[1]:
			_fail("tile %s must carry a %s, not '%s'" % [pair[0], pair[1], kind_here])
			return

	# --- 2. all four runs named, and named the RIGHT way round ------------
	# `carries` and not merely non-empty: `naming` is how `sim_core` says
	# whether it narrowed to one id or gave up, and an ambiguous run reads
	# as "" here exactly as an unnameable one does.
	for entry in Layouts.MUSTARD_RUNS:
		var tile: Vector2i = entry[0]
		var want := String(entry[1])
		var naming := _naming_on(tile)
		if naming != "carries":
			_fail(
				"the run at %s is '%s' rather than carrying one fluid — two networks have merged, or a machine went ambiguous, and every machine on it will stall in silence"
				% [tile, naming]
			)
			return
		if _fluid_on(tile) != want:
			_fail(
				"the run at %s must carry '%s', not '%s' — the mash belongs on the crusher's side of the mixer and the mix on the other, and a layout that swapped them would still report both ids somewhere in the snapshot"
				% [tile, want, _fluid_on(tile)]
			)
			return

	# --- 3. it made mustard, and shipped it -------------------------------
	var made := _made("boxed_mustard")
	var shipped := _delivered("boxed_mustard")
	if made <= 0:
		_fail(
			"no case of mustard was made in %d ticks — the line is plumbed and starved, which photographs exactly like a working one"
			% Layouts.MUSTARD_SETTLE_TICKS
		)
		return
	if shipped <= 0:
		_fail(
			"%d case(s) of mustard were made but none reached the delivery pool — the dock, its belt or its clerk is the missing stage"
			% made
		)
		return

	# --- 4. every id the sauce is made of resolves ------------------------
	# Enumerated from the simulation's own content graph. `boxed_mustard` is
	# named once, as the thing the line was just proved to SHIP, and
	# everything else is read out of the cookbook page for it.
	var sauce := _cookbook_sauce("boxed_mustard")
	if sauce.is_empty():
		_fail("cookbook_json has no page for 'boxed_mustard' — the sauce this line just shipped is not in the book, so nothing below is enumerated from anything")
		return
	var steps: Array = sauce.get("steps", [])
	var inputs: Array = sauce.get("inputs", [])
	if steps.size() < 5 or inputs.is_empty():
		_fail(
			"the mustard page lists %d step(s) and %d input(s) — a five-stage line with three raws should not read that short, and a truncated page would make every check below vacuous"
			% [steps.size(), inputs.size()]
		)
		return

	var fluids_in_order: Array = []
	for step in steps:
		var recipe := String(step.get("recipe", ""))
		var carries := String(step.get("carries", ""))
		var is_fluid := bool(step.get("carries_fluid", false))
		var machine := String(step.get("machine", ""))
		if is_fluid:
			if not main.FLUID_TINTS.has(carries):
				_fail(
					"'%s' leaves the %s down a pipe but has no FLUID_TINTS entry — it draws in FLUID_TINT_FALLBACK %s, a neutral pale blue, so a full run of it reads as water. Mustard's middle is ALL pipe, so this is most of the sauce."
					% [carries, machine, str(main.FLUID_TINT_FALLBACK)]
				)
				return
			fluids_in_order.append(carries)
		else:
			if not main.ITEM_MODELS.has(carries):
				_fail(
					"'%s' rides a belt out of the %s but has no ITEM_MODELS entry — it draws as ITEM_MODEL_FALLBACK '%s', a sealed cardboard box, which is what two whole stages of the ketchup chain shipped as when a key drifted"
					% [carries, machine, main.ITEM_MODEL_FALLBACK]
				)
				return
			# The ASSET the table names, loaded directly. NOT `_icon_for(id)`:
			# that consults `icon_names_json` first and returns the content's
			# own icon when there is one, so it comes back with a picture
			# whatever ITEM_MODELS holds — including a key pointing at
			# nothing, which is the case this line exists for.
			var asset: String = main.ITEM_MODELS[carries]
			var icon: Texture2D = main._icon_for_asset(asset)
			if icon == null or icon.get_width() < ICON_MIN_PIXELS:
				_fail(
					"'%s' maps to '%s', which has no real icon (%s) — a key pointing at a missing asset is as silent as a missing key"
					% [
						carries, asset,
						"null" if icon == null else "%dx%d" % [icon.get_width(), icon.get_height()],
					]
				)
				return
		# A machine that SHOWS its product paints the recipe it is running,
		# so every recipe such a machine can run needs a row. Which machines
		# those are comes off the built scene, not off a list here.
		var tile = _kind_tiles.get(machine, null)
		if tile != null and _shows_product(tile) and not main.PRODUCT_TINTS.has(recipe):
			_fail(
				"the %s at %s shows its product and runs '%s', which has no PRODUCT_TINTS entry — it draws in PRODUCT_TINT_FALLBACK, so a finished sauce reads as pale and unmade"
				% [machine, tile, recipe]
			)
			return

	for input in inputs:
		var id := String(input.get("id", ""))
		var fluid := bool(input.get("fluid", false))
		if fluid:
			if not main.FLUID_MODELS.has(id):
				_fail(
					"the raw fluid '%s' has no FLUID_MODELS entry — the market crate and the larder chip both fall through to a picture of a pipe (issue #67)"
					% id
				)
				return
		else:
			if not main.ITEM_MODELS.has(id):
				_fail("the raw solid '%s' has no ITEM_MODELS entry — it rides the feed belt as a cardboard box" % id)
				return
			if _hopper_items().has(id) and not main.HOPPER_FILL_TINTS.has(id):
				_fail(
					"'%s' can be put in a hopper but has no HOPPER_FILL_TINTS entry — its heap is the same pale sand as every other untinted raw"
					% id
				)
				return
		if not main.RACK_LOAD_TINTS.has(id):
			_fail(
				"'%s' can stand in a warehouse bay but has no RACK_LOAD_TINTS entry — its pallet draws in the fallback beige, and a rack's goods are seen against the FLOOR, so it cannot borrow the hopper's number"
				% id
			)
			return

	# --- 5. the three stages are three colours ----------------------------
	# In LINE ORDER, off the steps above, so this asserts the direction of
	# travel and not merely that three numbers differ. Mustard brightens
	# where ketchup and BBQ cook down — the pigment is turmeric and it does
	# not go in until the mixer — and that claim is only worth making if the
	# frame can show it.
	if fluids_in_order.size() < 3:
		_fail(
			"the mustard page names %d piped stage(s); its middle is supposed to be all fluid, so there is nothing here to compare"
			% fluids_in_order.size()
		)
		return
	var worst_step := INF
	for i in range(1, fluids_in_order.size()):
		var before: Color = main.FLUID_TINTS[fluids_in_order[i - 1]]
		var after: Color = main.FLUID_TINTS[fluids_in_order[i]]
		var step_luma := _luma(after) - _luma(before)
		if step_luma <= 0.0:
			_fail(
				"'%s' (luma %.4f) is not brighter than '%s' (luma %.4f) — mustard's run is a value LADDER, and a stage that goes the wrong way makes the line unreadable in the one direction a player watches it"
				% [
					fluids_in_order[i], _luma(after),
					fluids_in_order[i - 1], _luma(before),
				]
			)
			return
		if absf(step_luma) < MUSTARD_LUMA_STEP_MIN:
			_fail(
				"'%s' and '%s' are consecutive stages of one run and only %.4f of luma apart (%.1f/255, floor %.3f) — they are one hue, so VALUE is the only axis telling them apart and a player watching the line never sees the product change"
				% [
					fluids_in_order[i - 1], fluids_in_order[i], absf(step_luma),
					absf(step_luma) * 255.0, MUSTARD_LUMA_STEP_MIN,
				]
			)
			return
		worst_step = minf(worst_step, absf(step_luma))

	# --- 6. the vat and the pipe it fills are one colour ------------------
	var cooked := String(fluids_in_order[fluids_in_order.size() - 1])
	if main.PRODUCT_TINTS.get(cooked, main.PRODUCT_TINT_FALLBACK) != main.FLUID_TINTS[cooked]:
		_fail(
			"the vat draws '%s' as %s while the pipe it fills draws it as %s — one cook, one colour"
			% [cooked, str(main.PRODUCT_TINTS.get(cooked, main.PRODUCT_TINT_FALLBACK)), str(main.FLUID_TINTS[cooked])]
		)
		return

	print(
		"MUSTARD_OK: %d placements stand a whole second line from raw, all four runs named by "
		% line_placed
		+ "topology alone (%s), %d case(s) of mustard made and %d delivered in %d ticks; every id "
		% [
			", ".join(PackedStringArray([
				_fluid_on(Layouts.MUSTARD_RUNS[0][0]), _fluid_on(Layouts.MUSTARD_RUNS[1][0]),
				_fluid_on(Layouts.MUSTARD_RUNS[2][0]), _fluid_on(Layouts.MUSTARD_RUNS[3][0]),
			])),
			made, shipped, Layouts.MUSTARD_SETTLE_TICKS,
		]
		+ "on the sauce's own cookbook page (%d steps, %d inputs) resolves to a model or a tint of "
		% [steps.size(), inputs.size()]
		+ "its own and none to a fallback; and the run BRIGHTENS %s by at least %.1f/255 a stage "
		% [" -> ".join(PackedStringArray(fluids_in_order)), worst_step * 255.0]
		+ "of TINT luma (floor %.1f; photographs as +18.2 then +14.1), with the vat drawing '%s' "
		% [MUSTARD_LUMA_STEP_MIN * 255.0, cooked]
		+ "the same colour as the pipe it fills"
	)
	quit(0)


## The raw ids a hopper may legally be set to, off the simulation.
##
## `main._hopper_items` is the same list, but it is filled by
## `_refresh_hopper_items()` — which runs when a hopper is placed or its
## picker opens, and neither happens in here. Reading it instead left the
## hopper-tint guard walking an empty array, which is a guard that passes
## whatever the table holds; caught by deleting `HOPPER_FILL_TINTS["turmeric"]`
## and watching this file stay green.
func _hopper_items() -> Array:
	var snapshot = JSON.parse_string(main.sim.ingredients_snapshot_json())
	if not (snapshot is Dictionary):
		return []
	# Entries, not ids: each is `{id, locked, unlock_name, after_deliveries}`,
	# so an `Array.has("turmeric")` over the raw list is false for every raw
	# there is. That was the second way this one guard managed to pass
	# without looking at anything.
	var ids := []
	for entry in snapshot.get("hopper_items", []):
		ids.append(String(entry.get("id", "")))
	return ids
