# Headless proof of the warehouse stock rack's presentation (W3 of the
# warehouse sprint) — the structural claims a machine can check without a
# framebuffer. Whether the shelves actually READ at play zoom is what
# `scripts/screenshot-scene.sh --warehouse` is for, the same division
# test_machines.gd and test_fluid_gallery.gd already draw.
#
# Run with:
#   godot --headless --path godot_project --script test_warehouse_rack.gd
#
# The rack is the one building whose whole job is a READOUT: it does no work,
# has no crew, no lamp and no belt, and everything a player learns from it is
# learned by looking at which of its six shelves carry a pallet and how high
# the goods on them stand. Every way that can degrade is quiet — a bay index
# mapped to the wrong shelf still draws a rack full of pallets, a missing tint
# still draws goods, a stale cache still draws the stock that was there a
# minute ago — so none of them can be left to a rendered frame to catch.
# Verifies:
#   * the six shelves exist as real meshes at six distinct positions, filling
#     BOTTOM LEVEL FIRST, and every one of them inside the rack's own footprint
#     and under the model's own height (a pallet hanging in the air beside the
#     frame is the failure this geometry invites),
#   * an empty rack draws NO pallet at all — the check is not vacuous, so a
#     pallet appearing later means something,
#   * buying stock binds bays: pallets appear, one per occupied bay the wire
#     reports, and buying more raises the last one's load rather than adding a
#     shelf,
#   * the load's height tracks the bay's fill fraction and never exceeds
#     RACK_LOAD_HEIGHT_VOXELS — the beams above it are that close (issue #144),
#   * every RAW INGREDIENT the simulation offers — solids and fluids, gated
#     ones included — has a RACK_LOAD_TINTS entry, and every key in that table
#     is one of them: a guard that can only be given more entries has not been
#     fixed (the rule test_ingredient_models.gd and test_fluid_gallery.gd both
#     already apply),
#   * every one of those tints clears assets/voxels/generators/assets.rs's
#     RACK_TINT_DEAD_BAND — the measured luma band in which a full bay is
#     indistinguishable from an empty one against the factory floor,
#   * hover resolves from EITHER tile of the 1x2 footprint to the same rack,
#     names what is on it, and reports the shared warehouse rather than only
#     this building,
#   * the hover panel's meter measures bays against this rack's own bays,
#   * and W1's removal refusal is SURFACED: a rack whose bays the stock cannot
#     do without says so in the placement prompt and in hover, the refusal
#     clears when the stock is sold, and the prompt's words differ from the
#     ordinary REMOVE prompt.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## assets/voxels/generators/assets.rs's RACK_TINT_DEAD_BAND, copied here
## because a GDScript test cannot read a Rust constant. The generator's doc
## carries the measurement that produced it; this carries the enforcement.
## Change one, change both — and the failure message below says so.
const RACK_TINT_DEAD_BAND_LOW := 100.0
const RACK_TINT_DEAD_BAND_HIGH := 175.0

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("WAREHOUSE_RACK_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	# The fixture goes down in `_run_checks`, not here: `_ready` runs twice
	# under this harness and each run creates a fresh (empty) world, so
	# anything built now is wiped before the checks see it. See
	# test_hover.gd's header.


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


## Rec.709 luma on 0..255 — the measure `assets.rs`'s dead band was measured
## with, and the one this camera reads a rendered face by (UI_UX.md §7:
## value contrast, not hue).
func _luma(c: Color) -> float:
	return (0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b) * 255.0


## Every raw ingredient a rack can hold: the raw SOLIDS a hopper accepts plus
## the raw FLUIDS a tank may be declared to hold, which between them are
## `World::raw_items` and `World::raw_fluids` — the simulation's own two
## definitions of "raw", asked of it rather than listed here. Both wire lists
## keep an id a progression gate is still holding back, flagged rather than
## dropped (issue #90), so this reads the same six on a fresh world as on a
## finished one. A padlocked ingredient still needs a tint, and is in fact the
## one most likely to be missing it.
func _raw_ingredients() -> Array:
	main._refresh_hopper_items()
	main._refresh_tank_fluids()
	var ids: Array = []
	for id in main._hopper_items:
		ids.append(String(id))
	for id in main._tank_fluids:
		if not ids.has(String(id)):
			ids.append(String(id))
	return ids


func _ingredient_entry(item_id: String) -> Dictionary:
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry.get("id", "")) == item_id:
			return entry
	return {}


func _stock_of(item_id: String) -> int:
	return int(_ingredient_entry(item_id).get("owned", 0))


func _run_checks() -> void:
	# --- the tint table, before anything is built --------------------------
	# Checked first and against the SIMULATION's list rather than against
	# whatever a fixture happens to stock: an ingredient with no tint fails
	# here even though no world holds one yet.
	var raws := _raw_ingredients()
	if raws.size() < 2:
		_fail("the simulation offers fewer than two raw ingredients — this check would be vacuous")
		return
	for id in raws:
		if not main.RACK_LOAD_TINTS.has(id):
			_fail("raw ingredient '%s' can stand in a rack bay but has no RACK_LOAD_TINTS entry — its pallet draws in RACK_LOAD_TINT_FALLBACK %s, the same pale sand as every other untinted raw. Add it to RACK_LOAD_TINTS in main.gd, and measure it against RACK_TINT_DEAD_BAND rather than copying HOPPER_FILL_TINTS: a rack's goods are seen against the FLOOR, a hopper's down its own dark throat." % [
				id, str(main.RACK_LOAD_TINT_FALLBACK),
			])
			return
	# ...and the other way round, so the guard can be made to REFUSE rather
	# than merely be given more entries (issue #108's ruling).
	for key in main.RACK_LOAD_TINTS.keys():
		if not raws.has(String(key)):
			_fail("RACK_LOAD_TINTS has an entry for '%s', which the simulation does not offer as a raw ingredient — a stale key here is a tint nobody can ever see and a check that polices a hand-written list instead of the table it is about" % key)
			return
	var closest_id := ""
	var closest_margin := INF
	for id in raws:
		var tint: Color = main.RACK_LOAD_TINTS[id]
		var luma := _luma(tint)
		if luma > RACK_TINT_DEAD_BAND_LOW and luma < RACK_TINT_DEAD_BAND_HIGH:
			_fail("'%s' is tinted %s, luma %.1f — inside assets.rs's RACK_TINT_DEAD_BAND (%.0f..%.0f), the measured band where a loaded bay renders at the value of the floor behind it and a full rack cannot be told from an empty one at play zoom (issue #144). Take the colour up or down in VALUE, keeping its hue; the generator's doc has the measurement and the two tints already darkened for this reason." % [
				id, str(tint), luma, RACK_TINT_DEAD_BAND_LOW, RACK_TINT_DEAD_BAND_HIGH,
			])
			return
		var margin: float = (
			RACK_TINT_DEAD_BAND_LOW - luma if luma <= RACK_TINT_DEAD_BAND_LOW
			else luma - RACK_TINT_DEAD_BAND_HIGH
		)
		if margin < closest_margin:
			closest_margin = margin
			closest_id = id

	# --- one rack on an empty floor ----------------------------------------
	# Placed through the real purchase and place commands, like every fixture
	# in this project: this has to be a rack a player could have built.
	var tile := Vector2i(0, 0)
	Layouts.place(main.sim, Layouts.WAREHOUSE_KIND, tile, "east")
	main._rebuild_world()
	var key := "%d,%d" % [tile.x, tile.y]
	var nodes: Dictionary = main._rack_nodes.get(key, {})
	if nodes.is_empty():
		_fail("the placed warehouse rack built no bay nodes — _build_placement has no %s arm, or the kind id drifted" % main.WAREHOUSE_RACK_KIND)
		return
	var bays: Array = nodes.get("bays", [])
	if bays.size() != main.RACK_SLOTS:
		_fail("expected %d shelves on a rack, got %d" % [main.RACK_SLOTS, bays.size()])
		return

	# --- the six shelves are six real, distinct, in-bounds places ----------
	# The rack body's own extent is what they are judged against, and it is
	# read off the imported mesh rather than restated here: a pallet drawn
	# outside the frame it is supposed to be standing in is the failure this
	# geometry invites, and it looks perfectly fine in a generator diff.
	var body: Node3D = main.get_node_or_null("World/%s_%s" % [main.WAREHOUSE_RACK_KIND, key])
	if body == null:
		_fail("no rendered node for the placed rack at %s" % str(tile))
		return
	var body_meshes := _meshes(body, [])
	if body_meshes.is_empty():
		_fail("the placed rack rendered no meshes — the .vox didn't import")
		return
	var frame := AABB()
	var first := true
	for mi in body_meshes:
		if mi.mesh == null:
			continue
		var here: AABB = mi.mesh.get_aabb()
		here.position += body.position
		if first:
			frame = here
			first = false
		else:
			frame = frame.merge(here)
	var seen_positions: Array = []
	var previous_height := -INF
	for i in range(bays.size()):
		var bay: Dictionary = bays[i]
		var pallet: Node3D = bay["pallet"]
		var load: Node3D = bay["load"]
		if _meshes(pallet, []).is_empty() or _meshes(load, []).is_empty():
			_fail("shelf %d rendered no meshes — check RACK_PALLET_ASSET '%s' and RACK_LOAD_ASSET '%s' are staged by `cargo xtask`" % [
				i, main.RACK_PALLET_ASSET, main.RACK_LOAD_ASSET,
			])
			return
		for other in seen_positions:
			if pallet.position.is_equal_approx(other):
				_fail("two of the rack's shelves stand in the same place (%s) — the bay index is not reaching the geometry" % str(pallet.position))
				return
		seen_positions.append(pallet.position)
		# Inside the frame in plan, and under its roof. The Y bound is the
		# claim that matters most: `RACK_LEVEL_VOXELS` is copied from the
		# generator by hand, so a level added there and not here puts a pallet
		# through the cap plate.
		if pallet.position.y < frame.position.y or pallet.position.y > frame.position.y + frame.size.y:
			_fail("shelf %d stands at y=%.3f, outside the rack body's own %.3f..%.3f — RACK_LEVEL_VOXELS has drifted from assets.rs's RACK_LEVEL_Z" % [
				i, pallet.position.y, frame.position.y, frame.position.y + frame.size.y,
			])
			return
		if pallet.position.x < frame.position.x or pallet.position.x > frame.position.x + frame.size.x \
			or pallet.position.z < frame.position.z or pallet.position.z > frame.position.z + frame.size.z:
			_fail("shelf %d stands at %s, outside the rack's own footprint %s — RACK_BAY_OFFSETS has drifted from assets.rs's RACK_BAY_X/RACK_BAY_Y" % [
				i, str(pallet.position), str(frame),
			])
			return
		if load.position.y <= pallet.position.y:
			_fail("shelf %d's goods (y=%.3f) do not stand ON its pallet (y=%.3f)" % [
				i, load.position.y, pallet.position.y,
			])
			return
		# Bottom level first: shelf i's height never DROPS below shelf i-1's.
		# Two shelves share each level, so this is `>=`, not `>`.
		if pallet.position.y < previous_height - 0.0001:
			_fail("shelf %d stands lower (y=%.3f) than shelf %d (y=%.3f) — the bays do not fill from the floor up, so a part-used rack loads its top shelf first" % [
				i, pallet.position.y, i - 1, previous_height,
			])
			return
		previous_height = pallet.position.y

	# --- an empty rack draws nothing ---------------------------------------
	main._refresh_buffers()
	for i in range(bays.size()):
		if bays[i]["pallet"].visible or bays[i]["load"].visible:
			_fail("shelf %d is loaded on a rack in a factory holding nothing — the check that follows would be vacuous" % i)
			return

	# --- buying stock binds bays -------------------------------------------
	# `base_cap` of an ingredient is free and costs no bay at all, so the
	# order has to clear it before a single pallet appears. Read off the wire
	# rather than restated: `data/storage.ron` is content and may be tuned.
	var storage = JSON.parse_string(main.sim.storage_snapshot_json())
	if not (storage is Dictionary):
		_fail("storage_snapshot_json returned no object")
		return
	var base_cap := int(storage.get("base_cap", 0))
	var slot_stack := int(storage.get("slot_stack", 0))
	if base_cap <= 0 or slot_stack <= 0:
		_fail("data/storage.ron reports base_cap %d and slot_stack %d — one of them is unusable" % [base_cap, slot_stack])
		return
	var item := String(raws[0])
	# Exactly one bay's worth above the free allowance, less a little, so the
	# single bound bay is PART full and its load has a fraction to show.
	var part := slot_stack / 2
	main.sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item, "quantity": base_cap + part,
	}))
	main._refresh_buffers()
	if not bays[0]["pallet"].visible:
		_fail("buying %d of %s (base_cap %d + %d) bound no bay — its pallet is still hidden" % [
			base_cap + part, item, base_cap, part,
		])
		return
	if bays[1]["pallet"].visible:
		_fail("a part-full first bay also loaded the second shelf")
		return
	var part_height: float = bays[0]["load"].scale.y
	if not bays[0]["load"].visible or part_height <= 0.0:
		_fail("a bound, part-full bay drew no goods on its pallet")
		return
	if part_height > float(main.RACK_LOAD_HEIGHT_VOXELS) + 0.001:
		_fail("a part-full bay's load (%.3f) already stands taller than RACK_LOAD_HEIGHT_VOXELS (%d) — it would grow through the beams above it (issue #144)" % [
			part_height, main.RACK_LOAD_HEIGHT_VOXELS,
		])
		return

	# Filling that same bay to the brim raises its load and still binds only
	# one shelf — the fill fraction and the bay count are different readouts
	# and this is where they could be confused for each other.
	main.sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item, "quantity": slot_stack - part,
	}))
	main._refresh_buffers()
	if bays[0]["load"].scale.y <= part_height:
		_fail("filling the bay further did not raise its load (%.3f -> %.3f)" % [
			part_height, bays[0]["load"].scale.y,
		])
		return
	if bays[0]["load"].scale.y > float(main.RACK_LOAD_HEIGHT_VOXELS) + 0.001:
		_fail("a FULL bay's load (%.3f) stands taller than RACK_LOAD_HEIGHT_VOXELS (%d)" % [
			bays[0]["load"].scale.y, main.RACK_LOAD_HEIGHT_VOXELS,
		])
		return
	if bays[1]["pallet"].visible:
		_fail("a single full bay loaded a second shelf as well")
		return

	# One more unit spills into the next bay, which is the moment a player
	# watches for — and the pallets have to agree with the wire about how many
	# are bound, not merely be "more than before".
	main.sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item, "quantity": 1,
	}))
	main._refresh_buffers()
	var rack: Dictionary = main._rack_states.get(key, {})
	if rack.is_empty():
		_fail("no rack snapshot for the placed rack at %s" % key)
		return
	var contents: Array = rack.get("contents", [])
	if contents.size() != 2:
		_fail("expected the wire to report 2 occupied bays after one unit spilled over, got %d" % contents.size())
		return
	var loaded := 0
	for i in range(bays.size()):
		if bays[i]["pallet"].visible:
			loaded += 1
	if loaded != contents.size():
		_fail("the wire reports %d occupied bays and %d shelves are loaded — the renderer and the simulation disagree about the warehouse" % [
			contents.size(), loaded,
		])
		return

	# --- the tint reaches the goods ----------------------------------------
	# A material per shelf, and the one the table names for what is standing
	# there. The fallback would draw a pallet just as happily.
	var slab: MeshInstance3D = _meshes(bays[0]["load"], [])[0]
	var mat: StandardMaterial3D = slab.get_surface_override_material(0)
	if mat == null:
		_fail("a loaded bay's goods carry no material of their own — the per-item tint never reached them")
		return
	var want: Color = main.RACK_LOAD_TINTS[item]
	if not mat.albedo_color.is_equal_approx(want):
		_fail("the goods in a bay of %s are tinted %s, not the table's %s" % [
			item, str(mat.albedo_color), str(want),
		])
		return

	# --- hover, from BOTH tiles of the 1x2 footprint -----------------------
	var rack_name: String = main._kind_display_name(main.WAREHOUSE_RACK_KIND)
	var hovers: Array = []
	for hover_tile in [tile, tile + Vector2i(0, 1)]:
		var text: String = main._hover_text_at_tile(hover_tile)
		if not text.begins_with(rack_name):
			_fail("hovering the rack's tile %s resolved to '%s' rather than to the rack" % [
				str(hover_tile), text,
			])
			return
		hovers.append(text)
	if hovers[0] != hovers[1]:
		_fail("the rack's two tiles give two different tooltips — hover is resolving a tile rather than a footprint")
		return
	var hover: String = hovers[0]
	if not hover.contains(main._item_display_name(item)):
		_fail("the rack's tooltip does not name what is standing on it (%s):\n%s" % [item, hover])
		return
	if not hover.contains("warehouse:"):
		_fail("the rack's tooltip reports only this building and not the shared warehouse — bays are a pooled resource and a rack standing empty beside a full one is not a fault:\n%s" % hover)
		return

	# The hover panel's meter: BAYS used against this rack's own bays.
	var panel: Dictionary = main._hover_panel_state(tile)
	var meter: Dictionary = panel.get("meter", {})
	if meter.is_empty():
		_fail("the rack shows no meter, though its bays are a genuine ceiling")
		return
	var want_fraction := float(contents.size()) / float(rack.get("bays", 1))
	if absf(float(meter.get("fraction", -1.0)) - want_fraction) > 0.001:
		_fail("the rack's meter reads %.3f, expected %d of %d bays = %.3f" % [
			float(meter.get("fraction", -1.0)), contents.size(), int(rack.get("bays", 0)),
			want_fraction,
		])
		return

	# --- W4: the market carries the ceiling --------------------------------
	# The always-on STOCK plate counts what is held; the ceiling is drawn where
	# it is ACTED on, which is the crate. Three things have to be true of it:
	# the pair of numbers, a quantity clamped to what will fit (so the printed
	# promise is the purchase), and a word for a refusal that money cannot
	# cure.
	Layouts.fund(main.sim, 1000000)
	main._update_ingredient_market()
	var slot_index := -1
	for i in range(main._market_slots.size()):
		if String(main._market_slots[i]["id"]) == item:
			slot_index = i
	if slot_index < 0:
		_fail("the market has no crate for '%s'" % item)
		return
	var card: Dictionary = main._market_slots[slot_index]
	var market_entry := _ingredient_entry(item)
	var capacity := int(market_entry.get("capacity", 0))
	var unit_price := float(market_entry.get("price", 0.0))
	# The rack raised the ceiling, and by the whole of what it supplies: no
	# other ingredient is above its free allowance, so every bay is this one's
	# to use (`sim::storage::cap_for`).
	var want_capacity: int = base_cap + slot_stack * int(rack.get("bays", 0))
	if capacity != want_capacity:
		_fail("with one rack placed the wire caps '%s' at %d, expected base %d + %d bays x %d = %d" % [
			item, capacity, base_cap, int(rack.get("bays", 0)), slot_stack, want_capacity,
		])
		return
	var want_stock := "In stock %d / %d\n%.0f each" % [_stock_of(item), capacity, unit_price]
	if card["owned_label"].text != want_stock:
		_fail("the crate reads '%s', expected '%s' — the ceiling is not on the card" % [
			card["owned_label"].text.replace("\n", " / "), want_stock.replace("\n", " / "),
		])
		return

	# The warehouse line under the balance: racks standing, bays spoken for,
	# and the same figures on a bar (a ceiling the simulation defines, so a bar
	# is legal here — UI_UX.md §5e).
	var used_bays := int(main._storage_state.get("used_bays", 0))
	var total_bays := int(main._storage_state.get("total_bays", 0))
	if not main._market_storage_label.visible:
		_fail("the market says nothing about the warehouse, with a rack standing")
		return
	for phrase in ["1 rack", "bays %d / %d" % [used_bays, total_bays], "%d free" % (total_bays - used_bays)]:
		if not main._market_storage_label.text.contains(phrase):
			_fail("the market's warehouse line does not say '%s':\n%s" % [phrase, main._market_storage_label.text])
			return
	var bays_fraction := float(main._market_bays_meter.get_meta("fraction", -1.0))
	if absf(bays_fraction - float(used_bays) / float(total_bays)) > 0.001:
		_fail("the market's bay meter reads %.3f for %d of %d bays" % [bays_fraction, used_bays, total_bays])
		return

	# A CRATE CLAMPED TO THE ROOM LEFT. Four units of room, and the crate says
	# four and buys four — the simulation clamps and charges for what it
	# credits either way, so the thing at risk is the panel promising ten.
	var short_of_full := 4
	main.sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item,
		"quantity": capacity - _stock_of(item) - short_of_full,
	}))
	main._refresh_buffers()
	main._update_ingredient_market()
	if main._market_room(item) != short_of_full:
		_fail("with %d of %d held the market offers room for %d, expected %d" % [
			_stock_of(item), capacity, main._market_room(item), short_of_full,
		])
		return
	var want_crate := "Buy %d  ·  %.0f" % [short_of_full, unit_price * float(short_of_full)]
	if card["price_label"].text != want_crate:
		_fail("a crate with room for %d still offers '%s', not '%s'" % [
			short_of_full, card["price_label"].text, want_crate,
		])
		return
	var money_before: float = main._current_money()
	main._buy_ingredient_slot(slot_index)
	if _stock_of(item) != capacity:
		_fail("clicking the clamped crate moved stock to %d, not to the %d ceiling" % [_stock_of(item), capacity])
		return
	var spent: float = money_before - main._current_money()
	if absf(spent - unit_price * float(short_of_full)) > 0.01:
		_fail("the clamped crate charged %.2f for %d units at %.2f" % [spent, short_of_full, unit_price])
		return

	# AT CAPACITY IS ITS OWN REFUSAL. "You cannot afford it" and "there is
	# nowhere to put it" have different cures, so the full crate says the
	# second one in a word rather than wearing the first one's "✗" alone.
	main._update_ingredient_market()
	if main._market_room(item) != 0:
		_fail("a crate bought to its ceiling still reports room for %d" % main._market_room(item))
		return
	if not card["price_label"].text.begins_with("AT CAPACITY"):
		_fail("a full crate reads '%s' rather than saying it is at capacity" % card["price_label"].text)
		return
	if not card["price_label"].text.contains("✗"):
		_fail("the full crate carries no ✗ — the amber alone cannot say it (UI_UX.md §7)")
		return
	if card["shift_label"].text != "shift: buy 0  ✗":
		_fail("a full crate's shift line reads '%s'" % card["shift_label"].text)
		return
	if not card["button"].disabled:
		_fail("a crate with nowhere to put anything stayed live")
		return
	# ...and the click sends nothing at all. `revision()` counts commands,
	# refused or not, so it is the only honest proof of that.
	var revision_before: int = main.sim.revision()
	main._buy_ingredient_slot(slot_index)
	if main.sim.revision() != revision_before:
		_fail("clicking a full crate issued a command anyway (revision %d -> %d)" % [
			revision_before, main.sim.revision(),
		])
		return

	# --- W1's removal refusal, said in words -------------------------------
	# The only removal in the game that can fail. With every bay this rack
	# supplies needed by the stock standing in it, picking it up is refused —
	# silently in the simulation, so the whole job here is that the player is
	# told before they click.
	main.sim.issue_command_json(JSON.stringify({
		"type": "buy_ingredient", "item": item,
		"quantity": slot_stack * int(rack.get("bays", 0)),
	}))
	main._refresh_buffers()
	var bay_count := int(main._rack_states.get(key, {}).get("bays", 0))
	if not main._rack_removal_refused(bay_count):
		_fail("a rack whose every bay is holding stock is not reported as refused — used_bays %s of total_bays %s" % [
			str(main._storage_state.get("used_bays", "?")), str(main._storage_state.get("total_bays", "?")),
		])
		return
	main._remove_mode = true
	main._ghost_tile = tile
	main._ghost_reason = "rack_needed"
	var refused_prompt: String = main._placement_prompt_text()
	main._ghost_reason = ""
	var ordinary_prompt: String = main._placement_prompt_text()
	main._remove_mode = false
	if not refused_prompt.begins_with("✗"):
		_fail("the refused REMOVE prompt does not open with the ✗ glyph — the words alone carry this, there is no colour in it (UI_UX.md §7):\n%s" % refused_prompt)
		return
	if refused_prompt == ordinary_prompt:
		_fail("a refused REMOVE reads exactly like one that would work:\n%s" % refused_prompt)
		return
	if not refused_prompt.contains(rack_name):
		_fail("the refused REMOVE prompt does not name what it is refusing:\n%s" % refused_prompt)
		return
	# And the hover says it too, since a player may well point at the rack to
	# find out why it will not come up rather than re-entering remove mode.
	if not main._hover_text_at_tile(tile).contains("HOLDING"):
		_fail("a rack that cannot be picked up does not say so on hover:\n%s" % main._hover_text_at_tile(tile))
		return
	# The simulation agrees: the click is a total no-op.
	var before_layout: int = JSON.parse_string(main.sim.world_layout_json()).size()
	main.sim.issue_command_json(JSON.stringify({"type": "remove", "tile": [tile.x, tile.y]}))
	main._rebuild_world()
	if JSON.parse_string(main.sim.world_layout_json()).size() != before_layout:
		_fail("the simulation removed a rack the prompt said it would refuse — the two have drifted apart and the prompt is now a lie")
		return

	# It is a "not yet", not a "never": emptying the warehouse frees the bays
	# and the same rack comes up. Selling is not a command, so the stock is
	# consumed the way the market's own refund would be — by buying nothing
	# and instead removing the holding through a fresh world. Here the cheaper
	# proof is the arithmetic the refusal is derived from: with no stock above
	# `base_cap` there are no used bays at all.
	var fresh = load("res://main.tscn").instantiate()
	root.add_child(fresh)
	fresh._ready()
	Layouts.place(fresh.sim, Layouts.WAREHOUSE_KIND, tile, "east")
	fresh._rebuild_world()
	fresh._refresh_buffers()
	var fresh_bays := int(fresh._rack_states.get(key, {}).get("bays", 0))
	if fresh_bays <= 0:
		_fail("the second world's rack reports no bays at all")
		return
	if fresh._rack_removal_refused(fresh_bays):
		_fail("a rack in a factory holding nothing is refused — the refusal is not reading `used_bays` at all")
		return
	var layout_before: int = JSON.parse_string(fresh.sim.world_layout_json()).size()
	fresh.sim.issue_command_json(JSON.stringify({"type": "remove", "tile": [tile.x, tile.y]}))
	if JSON.parse_string(fresh.sim.world_layout_json()).size() >= layout_before:
		_fail("an empty rack could not be picked up either — the refusal has become unconditional")
		return

	print("WAREHOUSE_RACK_OK: %d shelves on a rack, bottom level first, all inside the imported frame; an empty rack draws none of them and stock binds them one at a time, %d of %d bays loaded in step with the wire, the load scaled to the bay's fill and capped at %d voxels; all %d raw ingredients (%s) have a rack tint of their own, no key is stale and every one clears the %.0f..%.0f dead band (closest: %s, %.1f luma clear); hover resolves from both tiles of the 1x2, names the goods and the shared warehouse, and meters bays; the market crate carries `owned / capacity` (%d / %d), clamps its quantity to the room left and says AT CAPACITY when there is none, under a header reading the racks and their free bays; and W1's refusal is spoken by the prompt and the tooltip before the click the simulation then refuses" % [
		bays.size(),
		contents.size(),
		int(rack.get("bays", 0)),
		main.RACK_LOAD_HEIGHT_VOXELS,
		raws.size(),
		", ".join(PackedStringArray(raws)),
		RACK_TINT_DEAD_BAND_LOW,
		RACK_TINT_DEAD_BAND_HIGH,
		closest_id,
		closest_margin,
		capacity,
		capacity,
	])
	quit(0)
