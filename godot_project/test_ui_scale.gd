# Headless structural proof of the 2026-09-09 UI-scale pass (docs/UI_UX.md §5).
#
# Run with:
#   godot --headless --path godot_project --script test_ui_scale.gd
#
# The project owner's report was that the UI is "much" too small and
# "difficult to read". The fix was a real jump — half again — expressed as one
# knob (`main.gd`'s UI_SCALE) that every font size, padding, border,
# separation, icon square and slot size is measured in. Like every other
# headless script here this deliberately does NOT claim the result *looks*
# right; that needs eyes on `scripts/screenshot-scene.sh`. What it checks is
# the two things that can rot silently and that a screenshot at one
# resolution would not catch:
#
#   1. **The scale is really applied, everywhere.** A hardcoded pixel size
#      left behind in one panel is invisible until someone opens that panel,
#      and there were two dozen of them before this pass. So every Label in
#      the whole HUD subtree is walked and its size checked against the
#      scale, rather than a handful of named ones being spot-checked.
#   2. **The result still FITS.** Twelve hotbar sockets, a twelve-kind build
#      menu and a three-docket clipboard all have to sit inside
#      `project.godot`'s own 1280x720 window with the strip along the bottom.
#      That is arithmetic, it changes with the catalog and with the scale,
#      and it is exactly what broke first when the scale went up — the build
#      menu ran a clear 300px past the bottom of the screen.
extends SceneTree

## What the pass was for: a real jump, not a token bump. If someone later
## tunes UI_SCALE back down to something that does not answer the original
## report, this is the test that says so.
const MIN_BODY_FONT := 20
## No label anywhere in the HUD may be smaller than this. The smallest thing
## in the design is a docket's cadence footnote at `_ui_font(-3)`, which is
## 18 at the shipped scale.
const MIN_ANY_FONT := 16


func _fail(msg: String) -> void:
	printerr("UI_SCALE_FAIL: %s" % msg)
	quit(1)


## Every Label in a subtree, so the checks below are about the whole HUD
## rather than about the panels someone remembered to name.
func _labels_under(node: Node, out: Array) -> Array:
	for child in node.get_children():
		if child is Label:
			out.append(child)
		_labels_under(child, out)
	return out


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	var hud: Node = main.get_node_or_null("HUD")
	if hud == null:
		_fail("no HUD CanvasLayer found")
		return

	# --- 1. One knob, and it is a real jump -------------------------------
	if main.UI_SCALE < 1.4:
		_fail("UI_SCALE is %.2f — the pass this test guards exists because the UI was too small to read" % main.UI_SCALE)
		return
	var body: int = main._ui_font()
	if body < MIN_BODY_FONT:
		_fail("the body font is %dpx, expected at least %d" % [body, MIN_BODY_FONT])
		return
	if body != int(round(float(main.UI_FONT_BASE) * main.UI_SCALE)):
		_fail("_ui_font() (%d) is not UI_FONT_BASE scaled by UI_SCALE" % body)
		return
	# The type hierarchy is offsets off the body size, so a bigger offset must
	# still be a bigger font — the property that makes `_ui_font(6)` mean "the
	# money figure" at any scale.
	if not (main._ui_font(-4) < body and body < main._ui_font(6)):
		_fail("_ui_font's offsets no longer order the type hierarchy")
		return
	# The three chrome helpers agree with each other and with the constants.
	if main._ui_px(10.0) != round(10.0 * main.UI_SCALE):
		_fail("_ui_px does not scale")
		return
	if main._ui_i(10) != int(round(10.0 * main.UI_SCALE)):
		_fail("_ui_i does not scale")
		return
	if not main._ui_v(Vector2(10, 20)).is_equal_approx(Vector2(main._ui_px(10), main._ui_px(20))):
		_fail("_ui_v does not scale both axes")
		return
	for pair in [
		["SHOP_SLOT_SIZE", main.SHOP_SLOT_SIZE, Vector2(128, 130)],
		["SHOP_ICON_SIZE", main.SHOP_ICON_SIZE, Vector2(46, 46)],
		["MARKET_SLOT_SIZE", main.MARKET_SLOT_SIZE, Vector2(128, 152)],
	]:
		var got: Vector2 = pair[1]
		var want: Vector2 = (pair[2] as Vector2) * main.UI_SCALE
		if not got.is_equal_approx(want):
			_fail("%s is %s, expected its design size scaled: %s" % [pair[0], str(got), str(want)])
			return
	# The hotbar socket is the one piece of chrome that deliberately takes
	# LESS than the full scale (see HOTBAR_STRIP_FIT) — but it must still be
	# genuinely bigger than the 64px it was before the pass.
	if main.HOTBAR_SLOT_SIZE.x <= 64.0:
		_fail("the hotbar socket did not grow at all: %.1f" % main.HOTBAR_SLOT_SIZE.x)
		return

	# --- 2. No label was left behind at a pre-scale size ------------------
	var labels := _labels_under(hud, [])
	if labels.size() < 10:
		_fail("only %d labels found under the HUD — the walk is not reaching the panels" % labels.size())
		return
	for label in labels:
		var settings: LabelSettings = label.label_settings
		if settings == null:
			continue
		if settings.font_size < MIN_ANY_FONT:
			_fail("label '%s' still reads at %dpx — a pre-scale size left behind (text: '%s')" % [
				label.name, settings.font_size, label.text,
			])
			return

	# --- 3. It all still fits the shipped window --------------------------
	# Measured against `project.godot`'s DECLARED window size, read from
	# ProjectSettings rather than from the live window: a headless run's root
	# window is 100x100 and `scripts/screenshot-scene.sh` renders at whatever
	# it is told, so neither is the size the game actually ships at. This is.
	#
	# The hotbar's reservation is taken from `get_combined_minimum_size()`
	# rather than from the live `size`, because nothing has laid itself out in
	# a headless run — the minimum is the honest number here and is what the
	# strip settles at anyway.
	var bounds := Vector2(
		float(ProjectSettings.get_setting("display/window/size/viewport_width", 0)),
		float(ProjectSettings.get_setting("display/window/size/viewport_height", 0))
	)
	if bounds.x <= 0.0 or bounds.y <= 0.0:
		_fail("project.godot declares no window size (%s), so nothing below can be checked" % str(bounds))
		return

	# The strip: twelve sockets plus separations plus the plate's margins.
	# This is the check HOTBAR_STRIP_FIT exists to satisfy, and the one a
	# thirteenth catalog kind will eventually fail.
	var strip: Vector2 = main._hotbar_root.get_combined_minimum_size()
	if strip.x > bounds.x:
		_fail("the hotbar strip is %.0fpx wide in a %.0fpx window — sockets or separations must shrink" % [
			strip.x, bounds.x,
		])
		return
	var reserved: float = strip.y + main.HOTBAR_BOTTOM_MARGIN * 2.0
	var available: float = bounds.y - reserved

	# Both shop panels and the clipboard: they must fit the window's width
	# outright, and their height must fit in what is left above the strip.
	# The build menu's grid and the clipboard's docket stack scroll if they
	# have to (a documented last resort — see `_fit_grid_scroll`), so what is
	# asserted is that the PANEL fits, which is what keeps the menu's footer
	# (the one line explaining the two mouse buttons) on screen.
	main._set_shop_menu_open(true)
	main._update_shop_menu()
	main._set_market_open(true)
	main._update_ingredient_market()
	main._update_contract_panel()
	main._fit_grid_scroll(main._shop_grid_scroll, main._shop_grid, available - main._ui_px(16.0))
	main._fit_grid_scroll(main._contract_cards_scroll, main._contract_cards_box, available - main._ui_px(24.0))
	for pair in [
		["the build menu", main._shop_menu],
		["the ingredient market", main._market_panel],
		["the contract clipboard", main._contract_panel],
	]:
		var panel: Control = pair[1]
		var size: Vector2 = panel.get_combined_minimum_size()
		if size.x > bounds.x:
			_fail("%s is %.0fpx wide in a %.0fpx window" % [pair[0], size.x, bounds.x])
			return
		if size.y > available:
			_fail("%s is %.0fpx tall but only %.0fpx is free above the hotbar" % [
				pair[0], size.y, available,
			])
			return

	print("UI_SCALE_OK: one knob at %.2f drives a %dpx body font and every panel's chrome, no label is left at a pre-scale size (%d checked), and the %.0f-wide hotbar plus all three panels fit a %.0fx%.0f window" % [
		main.UI_SCALE, body, labels.size(), strip.x, bounds.x, bounds.y,
	])
	quit(0)
