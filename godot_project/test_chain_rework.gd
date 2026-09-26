# Headless proof of the chain rework's presentation half (Epic A, ticket 2) —
# the structural claims a machine can check without a framebuffer. Whether any
# of it READS at scene scale is what scripts/screenshot-scene.sh is for, the
# same division test_machines.gd and test_splitters.gd already draw.
#
# Run with:
#   godot --headless --path godot_project --script test_chain_rework.gd
#
# It replaces test_timed_processes.gd, which went with the aging rack ticket 1
# deleted. What that file proved — a bespoke model at the tile the simulation
# reports, and a live gauge — is proved for every other kind by
# test_machines.gd; what is NEW and unproven is everything below.
#
# Verifies:
#   * THE AGING RACK IS GONE from the presentation layer: no model mapping, no
#     progress-gauge entry, and no kind in the shop catalog that fails to
#     resolve to a bespoke model (which is how ticket 1 left `main` red),
#   * A HOPPER CAN ACTUALLY BE PLACED. `_confirm_build` sends `item`, so the
#     placement lands; the same click with the field stripped is refused,
#     which is exactly the silent no-op the scene shipped with,
#   * A HOPPER IN HAND HOLDS THE FIRST RAW ITEM, from the simulation's own
#     `hopper_items` list rather than from a Godot literal — the ghost-time
#     **Q** cycle is gone (the click-to-configure ticket), so the placement
#     prompt names the default in words, says the contents are changed by
#     clicking the hopper once it is down, and the ghost carries a tinted
#     product body so the default is visible as well as written,
#   * TOMATO AND SUGAR BOTH REACH THE FLOOR: two hoppers placed from one
#     catalog slot, the second RETARGETED after placement through the
#     configuration panel's item picker, each drawing its own product tint,
#   * THE 1x2 MIXER renders one node covering both its tiles, and hover
#     resolves the mixer from either of them,
#   * THE TWO MACHINE-TO-MACHINE PIPE RUNS render at the tiles the simulation
#     reports, carry the fluid the recipe names, and draw plain straights
#     with a MACHINE FITTING on the side each machine stands (from `links`,
#     the pipes & conveyors overhaul's ticket 6), while the tank-fed vinegar
#     run turns down into the mixer through a CORNER with its fitting south,
#   * THE INGREDIENT MARKET is a real panel on M with one slot per priced
#     ingredient, a click buys a crate, and the larder strip follows the
#     stock it bought,
#   * A CRATE'S SECOND VERB: shift+click buys every unit the balance covers.
#     The arithmetic is pinned as a pure function, the slot prints what that
#     gesture would fetch right now and stays live while a crate is already
#     out of reach, a click spends the balance down to under one more unit,
#     and a shift+click with an empty wallet issues no command at all.
#
# The one-frame-before-checking dance and the "never hold your own camera
# reference" rule are inherited from test_hover.gd — see that file's header.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## Tiles of the shipped reference line this file asserts against. They mirror
## `sim_core/src/sim/reference.rs`'s `plan()`, which
## `sim_core/tests/machine_footprints.rs` pins — so a content change that
## moves the line fails there first, with a better message.
const MIXER := Vector2i(9, 0)
const MIXER_FAR := Vector2i(9, 1)
const MIX_RUN := [Vector2i(10, 0), Vector2i(11, 0)]
const KETCHUP_RUN := [Vector2i(14, 0), Vector2i(15, 0)]
const TOMATO_HOPPER := Vector2i(-1, 0)
const SUGAR_HOPPER := Vector2i(4, 1)

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("CHAIN_REWORK_FAIL: %s" % msg)
	quit(1)


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


func _meshes(node: Node, out: Array) -> Array:
	if node is MeshInstance3D:
		out.append(node)
	for child in node.get_children():
		_meshes(child, out)
	return out


func _placement_kind(tile: Vector2i) -> String:
	return String(main._placement_at_tile(tile).get("kind", ""))


func _hopper_item(tile: Vector2i) -> String:
	for hopper in JSON.parse_string(main.sim.hopper_snapshots_json()):
		if int(hopper["tile"][0]) == tile.x and int(hopper["tile"][1]) == tile.y:
			return String(hopper.get("item", ""))
	return ""


## How much of a raw ingredient the player owns, straight off the wire.
func _stock_of(item_id: String) -> int:
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry["id"]) == item_id:
			return int(entry["owned"])
	return 0


## Answers the payday warning (2026-09-26) with SPEND ANYWAY if it is up.
##
## A new game already owes a starting crew a wage, so any spend made from a
## balance funded down below that bill raises the confirmation — and the market
## section below deliberately funds itself down to two units of sugar to test
## the "buy all you can afford" arithmetic. The market is what this script is
## about, so it says yes and carries on; `test_payday_warning.gd` is what
## proves the panel, both its answers and its numbers.
func _spend_anyway() -> void:
	if main._payday_confirm_open:
		main._on_payday_confirm_yes()


## Which fluid the network covering `tile` carries, or "" if no network does.
func _fluid_at(tile: Vector2i) -> String:
	for network in JSON.parse_string(main.sim.fluid_snapshots_json()):
		for segment in network.get("segments", []):
			if int(segment["tile"][0]) == tile.x and int(segment["tile"][1]) == tile.y:
				return String(network.get("fluid", ""))
	return ""


func _run_checks() -> void:
	# --- 1. The aging rack left no scar tissue ----------------------------
	if main.MACHINE_MODELS.has("aging_rack"):
		_fail("MACHINE_MODELS still names the deleted 'aging_rack' kind")
		return
	if main.MACHINE_PROGRESS_GAUGES.has("aging_rack"):
		_fail("MACHINE_PROGRESS_GAUGES still names the deleted 'aging_rack' kind")
		return
	# The check ticket 1 actually left failing: every kind the shop prices has
	# to resolve to a real model, and a stale mapping is how that broke.
	var shop: Dictionary = JSON.parse_string(main.sim.shop_snapshot_json())
	for entry in shop.get("entries", []):
		var kind := String(entry.get("kind", ""))
		if kind == main.BELT_KIND or kind == main.PIPE_KIND:
			continue
		if main._shop_asset_for(kind) == main.MACHINE_MODEL_FALLBACK:
			_fail("priced kind '%s' has no bespoke model" % kind)
			return

	# --- 2. The list of things a hopper may hold is the SIMULATION'S ------
	main._refresh_hopper_items()
	if main._hopper_items.size() < 2:
		_fail("the shipped chain has two raw solids; hopper_items reports %s" % str(main._hopper_items))
		return
	if not (main._hopper_items.has("tomato") and main._hopper_items.has("sugar")):
		_fail("hopper_items should be the chain's raw solids, got %s" % str(main._hopper_items))
		return
	# ...and it does NOT include the raw FLUID, which goes in a tank.
	if main._hopper_items.has("vinegar"):
		_fail("vinegar is a fluid and cannot be a hopper's contents: %s" % str(main._hopper_items))
		return

	# --- 3. Placing a hopper through the real build click ------------------
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "hopper", "quantity": 2}))
	main._refresh_owned_counts()
	main._enter_build_mode("hopper")
	if main._build_kind != "hopper":
		_fail("entering placement mode for an owned hopper failed")
		return
	var first_item: String = main._selected_hopper_item()
	if first_item == "":
		_fail("a hopper in hand must have chosen contents")
		return
	# The prompt names the contents in words — UI_UX.md §7's never-colour-
	# alone half — and says where they can be changed. That used to be "Q:";
	# the contents key is gone (the click-to-configure ticket), so what a
	# hopper in hand holds is `raw_items[0]` and the prompt points at the
	# panel that changes it after the thing is on the floor.
	main._ghost_reason = ""
	var prompt: String = main._placement_prompt_text()
	if not prompt.contains(main._item_display_name(first_item)):
		_fail("the placement prompt does not name the hopper's contents: %s" % prompt)
		return
	if prompt.contains("Q:"):
		_fail("the placement prompt still advertises the deleted Q key: %s" % prompt)
		return
	if not prompt.contains("click it once placed"):
		_fail("the placement prompt does not say where contents are changed: %s" % prompt)
		return
	# ...and the ghost carries a tinted product body, so the choice is visible.
	main._rebuild_ghost_body()
	if main._ghost_root.get_node_or_null("ghost_fill") == null:
		_fail("the hopper ghost has no product body, so its contents are invisible")
		return

	main._ghost_tile = TOMATO_HOPPER
	main._ghost_valid = true
	main._confirm_build()
	if _placement_kind(TOMATO_HOPPER) != "hopper":
		_fail("the build click placed no hopper at %s — the `item` field is missing again" % str(TOMATO_HOPPER))
		return
	if _hopper_item(TOMATO_HOPPER) != first_item:
		_fail("the placed hopper holds '%s', expected '%s'" % [_hopper_item(TOMATO_HOPPER), first_item])
		return

	# The same command WITHOUT the item is a silent refusal — which is what
	# the scene shipped with and what this whole mechanism exists to fix.
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "hopper"}))
	main.sim.issue_command_json(JSON.stringify({
		"type": "place", "kind": "hopper", "tile": [-1, 5], "direction": "east",
	}))
	main._rebuild_world()
	if _placement_kind(Vector2i(-1, 5)) != "":
		_fail("a hopper placed with no `item` should be refused in a two-ingredient world")
		return

	# --- 4. A second hopper, RETARGETED after placement --------------------
	# Both hoppers now leave the hotbar holding the same default (there is no
	# ghost-time cycle any more), so the second one is switched the way a
	# player switches one: click it, then click the other ingredient in the
	# configuration panel's picker, which issues `set_hopper_item`.
	main._refresh_owned_counts()
	main._enter_build_mode("hopper")
	main._ghost_tile = Vector2i(-1, 3)
	main._ghost_valid = true
	main._confirm_build()
	if _hopper_item(Vector2i(-1, 3)) != first_item:
		_fail("a freshly placed hopper should default to '%s', got '%s'" % [
			first_item, _hopper_item(Vector2i(-1, 3))
		])
		return
	main._exit_build_mode()

	main._config_click_at_tile(Vector2i(-1, 3))
	if not main._config_open:
		_fail("clicking the second hopper did not open its configuration panel")
		return
	var other_index := -1
	for index in range(main._config_picker_slots.size()):
		if String(main._config_picker_slots[index]["item"]) != first_item:
			other_index = index
			break
	if other_index < 0:
		_fail("the picker offers only one ingredient: %s" % str(main._hopper_items))
		return
	main._on_config_picker_pressed(other_index)
	var second_item: String = _hopper_item(Vector2i(-1, 3))
	if second_item == first_item:
		_fail("the picker did not change the hopper's contents (still '%s')" % second_item)
		return
	main._close_config_panel()
	# Two hoppers, two different contents, out of ONE catalog slot — which is
	# why the hotbar did not have to grow a slot per ingredient.
	if _hopper_item(TOMATO_HOPPER) == second_item:
		_fail("both hoppers ended up holding the same thing")
		return
	# Each draws its own product tint, so they are tellable apart on the floor.
	var tints := {}
	for tile in [TOMATO_HOPPER, Vector2i(-1, 3)]:
		tints[main.HOPPER_FILL_TINTS.get(_hopper_item(tile), main.HOPPER_FILL_TINT_FALLBACK)] = true
	if tints.size() != 2:
		_fail("two hoppers holding different things drew the same product tint")
		return

	# --- 5. The market and the larder --------------------------------------
	if main._market_panel == null or main._market_slots.is_empty():
		_fail("no ingredient market was built")
		return
	var priced: Array = JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", [])
	if main._market_slots.size() != priced.size():
		_fail("the market has %d slots for %d priced ingredients" % [main._market_slots.size(), priced.size()])
		return
	main._set_market_open(true)
	if not main._market_panel.visible:
		_fail("opening the market did not show its panel")
		return
	var before_stock := 0
	for entry in priced:
		if String(entry["id"]) == "sugar":
			before_stock = int(entry["owned"])
	var sugar_slot := -1
	for index in range(main._market_slots.size()):
		if String(main._market_slots[index]["id"]) == "sugar":
			sugar_slot = index
	if sugar_slot < 0:
		_fail("the market has no slot for sugar")
		return
	main._buy_ingredient_slot(sugar_slot)
	var after_stock := 0
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry["id"]) == "sugar":
			after_stock = int(entry["owned"])
	if after_stock != before_stock + main.MARKET_CRATE:
		_fail("buying a crate moved sugar stock %d -> %d, expected +%d" % [before_stock, after_stock, main.MARKET_CRATE])
		return
	main._update_larder()
	var larder: Label = main._larder_entries.get("sugar")
	if larder == null or larder.text != str(after_stock):
		_fail("the larder reads '%s' for %d sugar" % ["<none>" if larder == null else larder.text, after_stock])
		return
	# An ingredient the player has none of says so in WORDS, not as a grey
	# zero — that line is the only thing that explains a SPACE that did
	# nothing (UI_UX.md §7).
	var unstocked: Label = null
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if int(entry["owned"]) == 0:
			unstocked = main._larder_entries.get(String(entry["id"]))
	if unstocked != null and unstocked.text != "none":
		_fail("an out-of-stock ingredient reads '%s', not 'none'" % unstocked.text)
		return

	# --- 5a. SHIFT+click buys all you can afford ---------------------------
	# The arithmetic first, as a pure function. It is a PROMISE THE SLOT
	# PRINTS ("shift: buy 123") as well as what the click sends, so the two
	# can never be allowed to disagree — and the fixed-point balance crossing
	# the wire as a double makes the exact-multiple case (3000 / 3) the one
	# worth pinning.
	for pure_case in [
		[0.0, 1.0, 0],       # no money buys nothing
		[7.0, 3.0, 2],       # the ordinary case: change left over
		[9.0, 3.0, 3],       # exact, small
		[3000.0, 3.0, 1000], # exact, and the one a double can round either way
		[2.0, 3.0, 0],       # not even one
		[5.0, 0.0, 0],       # an unpriced ingredient is not infinitely free
		[-1.0, 2.0, 0],      # a negative balance is not a purchase
	]:
		var got_units: int = main._units_affordable(float(pure_case[0]), float(pure_case[1]))
		if got_units != int(pure_case[2]):
			_fail("_units_affordable(%s, %s) = %d, expected %d" % [
				pure_case[0], pure_case[1], got_units, pure_case[2]
			])
			return
	var sugar_price := 0.0
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry["id"]) == "sugar":
			sugar_price = float(entry["price"])
	if sugar_price <= 0.0:
		_fail("sugar has no unit price on the wire to divide a balance by")
		return

	# A BALANCE THAT COVERS A FEW UNITS BUT NOT A CRATE. The slot must say so
	# and stay clickable: the crate is out of reach (that is what the "✗" and
	# the dimmed icon are for) but shift+click is not, and a disabled Button
	# would swallow the gesture.
	var short_money := int(sugar_price) * 2
	Layouts.fund(main.sim, short_money)
	main._update_ingredient_market()
	var sugar_stock := 0
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry["id"]) == "sugar":
			sugar_stock = int(entry["owned"])
	if main._affordable_units("sugar") != 2:
		_fail("with %d in hand and sugar at %.0f, the market offers %d units, expected 2" % [
			short_money, sugar_price, main._affordable_units("sugar")
		])
		return
	var sugar_card: Dictionary = main._market_slots[sugar_slot]
	if sugar_card["shift_label"].text != "shift: buy 2":
		_fail("the sugar crate's shift line reads '%s', not 'shift: buy 2'" % sugar_card["shift_label"].text)
		return
	if sugar_card["button"].disabled:
		_fail("a slot the player can still shift-buy two of was recessed as unbuyable")
		return

	# THE CLICK ITSELF: one command, the whole affordable quantity, and the
	# balance spent down to less than one more unit.
	main._buy_all_from_slot(sugar_slot)
	_spend_anyway()
	var bought_stock := 0
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry["id"]) == "sugar":
			bought_stock = int(entry["owned"])
	if bought_stock != sugar_stock + 2:
		_fail("shift+click moved sugar stock %d -> %d, expected +2" % [sugar_stock, bought_stock])
		return
	if main._affordable_units("sugar") != 0:
		_fail("buying all it could afford left the balance able to buy %d more" % main._affordable_units("sugar"))
		return
	# ...and the slot now says the wallet is out, in a glyph as well as a
	# number (UI_UX.md §7), with the square finally recessed.
	if sugar_card["shift_label"].text != "shift: buy 0  ✗":
		_fail("a spent-out crate's shift line reads '%s'" % sugar_card["shift_label"].text)
		return
	if not sugar_card["button"].disabled:
		_fail("a crate with neither verb left to offer stayed live")
		return

	# AN EMPTY WALLET ISSUES NOTHING AT ALL. `revision()` counts ticks plus
	# commands, refused or not — so it is the one honest test of "no command
	# was sent", which a stock count alone could not tell from a command the
	# simulation happened to refuse.
	Layouts.fund(main.sim, 0)
	main._update_ingredient_market()
	var revision_before: int = main.sim.revision()
	main._buy_all_from_slot(sugar_slot)
	if main.sim.revision() != revision_before:
		_fail("a shift+click with an empty wallet issued a command anyway (revision %d -> %d)" % [
			revision_before, main.sim.revision()
		])
		return
	var broke_stock := 0
	for entry in JSON.parse_string(main.sim.ingredients_snapshot_json()).get("entries", []):
		if String(entry["id"]) == "sugar":
			broke_stock = int(entry["owned"])
	if broke_stock != bought_stock:
		_fail("a shift+click with an empty wallet moved sugar stock %d -> %d" % [bought_stock, broke_stock])
		return

	# THE LINE FITS THE CARD it is printed in, at a balance rich enough to
	# make the quantity five digits — UI_UX.md §2b point 32: a fixed card
	# clips a label without complaint, and a market crate is a fixed square.
	Layouts.fund(main.sim, 32767)
	main._update_ingredient_market()
	var cheapest: Label = main._market_slots[0]["shift_label"]
	var inner_width: float = main.MARKET_SLOT_SIZE.x - 2.0 * main._ui_px(8)
	var drawn_width: float = cheapest.get_theme_default_font().get_string_size(
		cheapest.text, HORIZONTAL_ALIGNMENT_LEFT, -1, cheapest.label_settings.font_size
	).x
	if drawn_width > inner_width:
		_fail("the shift line '%s' draws %.0fpx wide inside a %.0fpx card" % [
			cheapest.text, drawn_width, inner_width
		])
		return

	# THE GESTURE, through the handler the Button hands its raw events to.
	# A plain left press belongs to the crate (it is `pressed`'s, and this
	# handler has to keep its hands off it); only SHIFT+left is buy-all.
	Layouts.fund(main.sim, int(sugar_price) * 3)
	main._update_ingredient_market()
	var plain_click := InputEventMouseButton.new()
	plain_click.button_index = MOUSE_BUTTON_LEFT
	plain_click.pressed = true
	var quiet_revision: int = main.sim.revision()
	main._on_market_slot_gui_input(plain_click, sugar_slot)
	if main.sim.revision() != quiet_revision:
		_fail("an unmodified left-click reached the buy-all handler and spent money")
		return
	var shift_click := InputEventMouseButton.new()
	shift_click.button_index = MOUSE_BUTTON_LEFT
	shift_click.pressed = true
	shift_click.shift_pressed = true
	var gesture_stock := _stock_of("sugar")
	main._on_market_slot_gui_input(shift_click, sugar_slot)
	_spend_anyway()
	if _stock_of("sugar") != gesture_stock + 3:
		_fail("SHIFT+left-click moved sugar stock %d -> %d, expected +3" % [
			gesture_stock, _stock_of("sugar")
		])
		return

	# BOTH VERBS ARE NAMED where the player looks for them (UI_UX.md §5b).
	var market_footer := ""
	for child in main._market_panel.find_children("*", "Label", true, false):
		if String(child.text).find("M/ESC: close") >= 0:
			market_footer = String(child.text)
	if market_footer.find("buy %d" % main.MARKET_CRATE) < 0 or market_footer.find("shift+click") < 0:
		_fail("the market footer names only one of its two verbs: '%s'" % market_footer)
		return
	main._set_market_open(false)

	# --- 6. The shipped line: the 1x2 mixer and the piped middle ----------
	# A fresh scene, because the hoppers above stand where the line goes.
	var line = load("res://main.tscn").instantiate()
	root.add_child(line)
	line._ready()
	Layouts.build_reference_line(line.sim)
	Layouts.stock_larder(line.sim)
	line._rebuild_world()

	var placed := Layouts.line_placements(line.sim)
	if placed != Layouts.REFERENCE_LINE_PLACEMENTS:
		_fail("the reference line placed %d of %d pieces — something was refused" % [
			placed, Layouts.REFERENCE_LINE_PLACEMENTS
		])
		return

	# One node, covering both of the mixer's tiles, and hover resolves from
	# either — the multi-tile machinery the 2x2 vat already proved, now
	# exercised by the only OBLONG kind in the file.
	var mixer_node: Node3D = line.get_node_or_null("World/mixer_%d,%d" % [MIXER.x, MIXER.y])
	if mixer_node == null:
		_fail("no rendered node for the mixer at %s" % str(MIXER))
		return
	if _meshes(mixer_node, []).is_empty():
		_fail("the mixer rendered no meshes — its .vox didn't import")
		return
	if line._kind_footprint("mixer") != Vector2i(1, 2):
		_fail("the mixer's footprint reads %s, expected 1x2" % str(line._kind_footprint("mixer")))
		return
	# The node sits at the CENTRE of the 1x2 rectangle, which is half a tile
	# north of its origin — the coupling between a kind's footprint and its
	# model's size (`machine_mixer` is 32x64 voxels for exactly this reason).
	var expected_centre: Vector3 = line._footprint_centre(MIXER, Vector2i(1, 2), line.FLOOR_Y)
	if not mixer_node.position.is_equal_approx(expected_centre):
		_fail("the mixer sits at %s, expected its footprint centre %s" % [
			str(mixer_node.position), str(expected_centre)
		])
		return
	for tile in [MIXER, MIXER_FAR]:
		var hover: String = line._hover_text_at_tile(tile)
		if not hover.contains("Ketchup Mix"):
			_fail("hover at %s should resolve the mixer's recipe by name, got '%s'" % [str(tile), hover])
			return

	# Both machine-to-machine runs exist, carry the right fluid, and draw two
	# plain straights: the first with the machine fitting on its WEST side
	# (the machine it leaves), the second with one on its EAST (the machine
	# it feeds) — each decided from the tile's own `links`.
	for pair in [[MIX_RUN, "ketchup_mix"], [KETCHUP_RUN, "ketchup"]]:
		var run: Array = pair[0]
		for i in range(run.size()):
			var tile: Vector2i = run[i]
			if _placement_kind_on(line, tile) != "pipe":
				_fail("no pipe at %s — the piped middle of the chain is missing" % str(tile))
				return
			if line._tile_instance_at(tile).is_empty():
				_fail("the pipe at %s was not rendered" % str(tile))
				return
			if line._pipe_asset_for(tile) != line.PIPE_ASSET:
				_fail("the pipe at %s drew '%s'; a machine-to-machine run is straight" % [
					str(tile), line._pipe_asset_for(tile)
				])
				return
			var fittings: Array = line._pipe_fittings_at(tile)
			var side := "west" if i == 0 else "east"
			if fittings.size() != 1 or String(fittings[0]["asset"]) != line.PIPE_FITTING_MACHINE_ASSET \
					or String(fittings[0]["to"]) != side:
				_fail("the pipe at %s should carry one machine fitting on its %s side, got %s" % [
					str(tile), side, str(fittings)
				])
				return
		if _fluid_on(line, run[0]) != pair[1]:
			_fail("the run at %s carries '%s', expected '%s'" % [
				str(run[0]), _fluid_on(line, run[0]), pair[1]
			])
			return

	# ...while the TANK-fed vinegar run, which lies north of the mixer and
	# turns down into it, ends in a corner with the machine fitting south.
	var vinegar_tap := Vector2i(9, 2)
	if line._pipe_asset_for(vinegar_tap) != line.PIPE_CORNER_ASSET:
		_fail("the vinegar run's tapped tile drew '%s', expected a corner" % line._pipe_asset_for(vinegar_tap))
		return
	var tap_fittings: Array = line._pipe_fittings_at(vinegar_tap)
	if tap_fittings.size() != 1 or String(tap_fittings[0]["to"]) != "south":
		_fail("the vinegar tap should carry one fitting on its south side, got %s" % str(tap_fittings))
		return

	# Neither the vat nor the bottler has a feed belt any more, which is the
	# visible consequence of the whole rework.
	for tile in [Vector2i(11, 0), Vector2i(15, 0)]:
		if _placement_kind_on(line, tile) == line.BELT_KIND:
			_fail("a belt still runs into a machine that is fed by pipe, at %s" % str(tile))
			return

	# --- 7. Both feed lanes are belted, and both hoppers stand -------------
	for tile in [TOMATO_HOPPER, SUGAR_HOPPER]:
		if _placement_kind_on(line, tile) != "hopper":
			_fail("no hopper at %s — the mixer's second solid ingredient has no feed" % str(tile))
			return
	if _hopper_item_on(line, SUGAR_HOPPER) != "sugar":
		_fail("the hopper at %s holds '%s', expected sugar" % [
			str(SUGAR_HOPPER), _hopper_item_on(line, SUGAR_HOPPER)
		])
		return
	for lane_y in [0, 1]:
		var feed := Vector2i(8, lane_y)
		if _placement_kind_on(line, feed) != line.BELT_KIND:
			_fail("the mixer's inlet tile at row %d has no feed belt" % lane_y)
			return

	print("CHAIN_REWORK_OK: the aging rack is gone from every mapping, a hopper places holding the first raw item (the prompt names it and points at the panel that changes it, the ghost shows it) while the same click without an item is refused, a placed hopper is retargeted through the panel's picker, the market buys crates and the larder follows, shift+click buys every unit the balance covers and an empty wallet sends nothing, and the shipped line renders a 1x2 mixer hovering from both tiles, two machine-to-machine pipe runs carrying ketchup_mix and ketchup on the plain tile, the tank-fed run still on the branch fitting, and no belt into either piped machine")
	quit(0)


func _placement_kind_on(scene, tile: Vector2i) -> String:
	return String(scene._placement_at_tile(tile).get("kind", ""))


func _hopper_item_on(scene, tile: Vector2i) -> String:
	for hopper in JSON.parse_string(scene.sim.hopper_snapshots_json()):
		if int(hopper["tile"][0]) == tile.x and int(hopper["tile"][1]) == tile.y:
			return String(hopper.get("item", ""))
	return ""


func _fluid_on(scene, tile: Vector2i) -> String:
	for network in JSON.parse_string(scene.sim.fluid_snapshots_json()):
		for segment in network.get("segments", []):
			if int(segment["tile"][0]) == tile.x and int(segment["tile"][1]) == tile.y:
				return String(network.get("fluid", ""))
	return ""
