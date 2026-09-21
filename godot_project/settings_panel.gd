## The settings screen: one column of rows, built once and hosted twice.
##
## The main menu and the pause menu both offer SETTINGS, and they must be the
## same screen — a volume slider that exists on one and not the other, or two
## that drift a row apart, is the defect `factory_environment.gd` exists to
## prevent for the hall and this file prevents for the settings. Both hosts
## build their UI in code, both already scale off `main.gd`'s `UI_SCALE`
## (`menu.gd` preloads `main.gd` for exactly that), so one builder serves both.
##
## Reached by `preload`, never `class_name` (godot_project/CLAUDE.md).
##
## # It is a COLUMN, not a plate
##
## `build()` returns a `VBoxContainer` the host drops into the plate it
## already has, and the host hides its own button column while it is showing.
## Both files already do this with something: `menu.gd`'s LOAD GAME replaces
## the three buttons with the save list, and `main.gd`'s `_pause_confirm_box`
## replaces `_pause_buttons_box` with the quit question — for the reason
## stated there, that "a second modal on top of a modal is two things to
## dismiss". A settings screen is a whole page of that argument.
##
## # Why steppers and not sliders
##
## There is no `HSlider` anywhere in this project, and adding one here would
## put a stock engine control in the middle of a UI built entirely out of
## `StyleBoxFlat` plates. It would also be untestable: a drag cannot be
## asserted under `--headless`, and `godot_project/CLAUDE.md` is explicit that
## a headless test cannot prove a width, which is the only thing a slider's
## position means. A row is therefore `[-] LABEL  value [+]`, and every press
## is a discrete, assertable command against `Settings.nudge`.
##
## Under a stepped row is a thin bar showing where in its range the value
## sits. **That bar is not a meter.** The meter language
## (`main.gd::_build_meter`/`_set_meter`) is reserved for numbers where the
## simulation defines the ceiling, and none of these come from the simulation
## at all — so this one has its own node name (`SettingRange`), its own
## colours and its own builder, and it is purely decorative: the value beside
## it is always spelled out in words (`100%`, `x1.25`, `75°`), per UI_UX.md
## §7's rule that a bar is never the only thing saying what a number is.
extends RefCounted

## The plate's accent, `menu.gd`'s `PLATE_ACCENT` and `main.gd`'s border
## colour — the one hazard yellow every panel in this game is edged with.
const ACCENT := Color(0.85, 0.63, 0.11, 0.95)
const HEADING := Color(0.98, 0.85, 0.42)
const INK := Color(0.92, 0.93, 0.94)
const INK_DIM := Color(0.66, 0.68, 0.70)

## The range bar under a stepped row. Deliberately NOT any of
## `main.gd`'s `METER_FILL_*` colours — see the header.
const RANGE_FILL := Color(0.62, 0.66, 0.72, 0.95)
const RANGE_TRACK := Color(0.16, 0.16, 0.18, 0.95)
const RANGE_HEIGHT := 4.0
## Wide enough that the longest label ("LOOK SENSITIVITY") and its value do
## not shove the `+` button off the row at either host's scale.
const ROW_WIDTH := 330.0
const STEP_BUTTON_SIZE := Vector2(40.0, 30.0)

var settings: Node
var root: VBoxContainer
## key -> the Label showing its value (a stepped row) or the Button whose
## caption carries it (a toggle). What `refresh()` writes and what a test
## reads, instead of walking the tree for a node path.
var value_controls := {}
## key -> the range bar under a stepped row. Toggles have no entry.
var range_bars := {}
var reset_button: Button
## The scrolled column and the box inside it, kept so `fit()` can re-measure
## them once the host has this panel in its tree.
var _scroll: ScrollContainer
var _rows: VBoxContainer
var back_button: Button

var _ui_scale := 1.0
var _font_base := 15


## Builds the column. `on_back` is called by BACK; the host decides what
## "back" means (the menu's three buttons, or the pause menu's).
##
## `ui_scale` and `font_base` are passed in rather than read from `main.gd`,
## even though that is where both live: `main.gd` preloads THIS file, so
## preloading it back would be a cycle. The two hosts are the ones that
## already hold the knob (`main.gd`'s `UI_SCALE`/`UI_FONT_BASE`, `menu.gd`'s
## `UI_SCALE`/`BASE_FONT_SIZE`, which is the same 15px design base), so they
## hand it over and the "UI scale is two constants" rule still holds.
##
## `rows_height_budget` is how much room the host has for the rows — eight of
## them at 1.5x do not fit a 720-tall window, and a panel taller than the
## screen puts its BACK button where it cannot be clicked (UI_UX.md §5c). Over
## the budget the rows scroll, exactly as the menu's save list does; under it
## the scroll box shrinks to the rows and no empty band appears. Pass 0 for no
## cap (the headless tests, which have no window to measure).
func build(
	settings_node: Node,
	ui_scale: float,
	font_base: int,
	rows_height_budget: float,
	on_back: Callable
) -> VBoxContainer:
	settings = settings_node
	_ui_scale = ui_scale
	_font_base = font_base

	root = VBoxContainer.new()
	root.name = "SettingsPanel"
	root.add_theme_constant_override("separation", _i(10))

	var heading := Label.new()
	heading.name = "SettingsHeading"
	heading.label_settings = _label_settings(_font(4), HEADING)
	heading.text = "SETTINGS"
	heading.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root.add_child(heading)

	# The line that says where these live. A player who has just turned the
	# music down wants to know it will still be down tomorrow, and a developer
	# wants to know which file to delete — UI_UX.md §7 again: the pause menu
	# says the factory is stopped, and this says the setting is kept.
	var note := Label.new()
	note.name = "SettingsNote"
	note.label_settings = _label_settings(_font(-2), INK_DIM)
	note.text = "kept on this computer, not in your save"
	note.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root.add_child(note)

	root.add_child(_rule())

	_scroll = ScrollContainer.new()
	var scroll := _scroll
	scroll.name = "SettingsScroll"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	root.add_child(scroll)

	_rows = VBoxContainer.new()
	var rows := _rows
	rows.name = "SettingsRows"
	rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	rows.add_theme_constant_override("separation", _i(9))
	scroll.add_child(rows)

	for entry in _knobs():
		rows.add_child(_build_row(entry))

	# Measured AFTER the rows exist, and only the height: every control on a
	# row is pinned, so this is a sum of numbers this file chose rather than a
	# question about font metrics — which is the part a headless run cannot
	# answer (godot_project/CLAUDE.md).
	scroll.custom_minimum_size.x = _px(ROW_WIDTH)
	fit(rows_height_budget)

	root.add_child(_rule())

	var footer := HBoxContainer.new()
	footer.name = "SettingsFooter"
	footer.add_theme_constant_override("separation", _i(8))
	root.add_child(footer)

	reset_button = _button("ResetButton", "RESET")
	reset_button.custom_minimum_size = Vector2(_px(ROW_WIDTH * 0.36), _px(38.0))
	reset_button.pressed.connect(_on_reset_pressed)
	footer.add_child(reset_button)

	back_button = _button("BackButton", "BACK")
	back_button.custom_minimum_size = Vector2(_px(ROW_WIDTH * 0.6), _px(38.0))
	back_button.pressed.connect(on_back)
	footer.add_child(back_button)

	refresh()
	return root


## The knob definitions, or none. A null `Settings` means the autoload was
## never registered, which is a broken `project.godot` rather than a state
## worth drawing — the panel comes up empty and says nothing false.
func _knobs() -> Array:
	return settings.KNOBS if settings != null else []


## One row, toggle or stepped. The `key` is on every node's name so a
## rendered frame and a failing test both name the setting rather than a row
## index.
func _build_row(entry: Dictionary) -> Control:
	var key: String = entry["key"]
	var column := VBoxContainer.new()
	column.name = "Row_%s" % key
	column.add_theme_constant_override("separation", _i(3))

	if entry["kind"] == "toggle":
		# One wide button captioned in WORDS, which is exactly what the pause
		# menu's SOUND and MUSIC buttons have always been (UI_UX.md §7: a
		# switch says ON or OFF, it does not merely look pressed). Those two
		# buttons moved in here; this is them.
		var toggle := _button("Toggle_%s" % key, "")
		toggle.custom_minimum_size = Vector2(_px(ROW_WIDTH), _px(34.0))
		toggle.pressed.connect(_on_nudge_pressed.bind(key, 1))
		column.add_child(toggle)
		value_controls[key] = toggle
	else:
		var line := HBoxContainer.new()
		line.name = "Line_%s" % key
		line.add_theme_constant_override("separation", _i(8))
		column.add_child(line)

		var minus := _button("Minus_%s" % key, "-")
		minus.custom_minimum_size = STEP_BUTTON_SIZE * _ui_scale
		minus.pressed.connect(_on_nudge_pressed.bind(key, -1))
		line.add_child(minus)

		var label := Label.new()
		label.name = "Label_%s" % key
		label.label_settings = _label_settings(_font(0), INK)
		label.text = entry["label"]
		label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		line.add_child(label)

		var value := Label.new()
		value.name = "Value_%s" % key
		value.label_settings = _label_settings(_font(0), HEADING)
		value.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		value.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		# Pinned so the row does not jog sideways as "100%" becomes "90%" —
		# and pinned rather than wrapped, per godot_project/CLAUDE.md: a
		# Label whose width something measures is pinned or it does not wrap.
		value.custom_minimum_size = Vector2(_px(64.0), 0.0)
		line.add_child(value)

		var plus := _button("Plus_%s" % key, "+")
		plus.custom_minimum_size = STEP_BUTTON_SIZE * _ui_scale
		plus.pressed.connect(_on_nudge_pressed.bind(key, 1))
		line.add_child(plus)

		value_controls[key] = value
		range_bars[key] = _build_range(column, key)

	var help: String = entry["help"]
	if help != "":
		var help_label := Label.new()
		help_label.name = "Help_%s" % key
		help_label.label_settings = _label_settings(_font(-3), INK_DIM)
		help_label.text = help
		column.add_child(help_label)
	return column


## Everything on this column EXCEPT the rows: the heading, the note, the two
## rules and the RESET/BACK footer. A host that knows how tall the whole
## column may be takes this off to get the budget `fit()` wants.
##
## Measured with the scroll zeroed — `menu.gd::_apply_list_height`'s trick —
## because the scroll's own height is what is being solved for, and asking a
## container how big it is while the answer is still inside the question gives
## the last answer rather than the right one.
func chrome_height() -> float:
	if root == null or _scroll == null:
		return 0.0
	var rows_height: float = _scroll.custom_minimum_size.y
	_scroll.custom_minimum_size.y = 0.0
	var chrome: float = root.get_combined_minimum_size().y
	_scroll.custom_minimum_size.y = rows_height
	return chrome


## Re-measures the scrolled column against `budget` pixels of room. Called by
## `build()` and again by each host every time it SHOWS the panel, which is
## the only moment the two numbers this depends on are both true: the window
## size (the host reads it) and the rows' real laid-out heights.
##
## Both matter. `build()` runs while this column is still outside any tree —
## the host adds it afterwards — and a Control measured outside a tree
## answers with the DEFAULT theme's metrics rather than the one it will be
## drawn with, so the boundary `build()` picks is close but not exact. A
## window resized between two openings makes it wrong by more than that.
##
## `budget <= 0` means "no cap" — the headless tests, which have no window to
## measure and should see every row at its full height.
func fit(budget: float) -> void:
	if _scroll == null or _rows == null:
		return
	var wanted: float = _rows.get_combined_minimum_size().y
	var height: float = wanted
	if budget > 0.0:
		height = minf(wanted, _whole_rows_height(_rows, budget))
	_scroll.custom_minimum_size.y = height


## The tallest height not over `cap` that ends BETWEEN two rows, so a scrolled
## column is cut on a boundary rather than through the middle of one.
##
## `main.gd::_fit_grid_scroll` calls this snapping and reserves it for "a
## stack whose rows are DECISIONS the player signs — where half a card is
## half a choice". A settings row qualifies for a blunter reason: the row IS
## its `-` and `+` buttons, and half of one is a control drawn where it
## cannot be clicked (UI_UX.md §5c). The build menu's deliberate half-row,
## which is what says "scroll" there, would here be a button you can press
## the top of.
##
## Restated rather than shared, like `_button`'s styling: this file cannot
## preload `main.gd`, which preloads it. Rows are MEASURED rather than divided
## out of one constant for `main.gd`'s reason too — a toggle row, a stepped
## row and a stepped row with no help line are three different heights.
##
## One whole row is the floor: a column showing nothing at all is worse than
## a column showing its first row and a scrollbar.
func _whole_rows_height(box: VBoxContainer, cap: float) -> float:
	if box == null or box.get_child_count() == 0:
		return cap
	var separation: float = float(box.get_theme_constant("separation"))
	var used := 0.0
	var last_boundary := 0.0
	for child in box.get_children():
		if not (child is Control):
			continue
		if used > 0.0:
			used += separation
		used += (child as Control).get_combined_minimum_size().y
		if used > cap:
			break
		last_boundary = used
	return last_boundary if last_boundary > 0.0 else cap


## The range bar. Sized explicitly and recording its own fraction, the way
## `main.gd::_set_meter` does, so a headless test can assert it with no layout
## pass — but under its own name, because it is not a meter (see the header).
func _build_range(parent: Node, key: String) -> Panel:
	var track_style := StyleBoxFlat.new()
	track_style.bg_color = RANGE_TRACK

	var track := Panel.new()
	track.name = "SettingRange_%s" % key
	track.mouse_filter = Control.MOUSE_FILTER_IGNORE
	track.custom_minimum_size = Vector2(_px(ROW_WIDTH), _px(RANGE_HEIGHT))
	track.add_theme_stylebox_override("panel", track_style)
	parent.add_child(track)

	var fill_style := StyleBoxFlat.new()
	fill_style.bg_color = RANGE_FILL

	var fill := Panel.new()
	fill.name = "Fill"
	fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
	fill.add_theme_stylebox_override("panel", fill_style)
	track.add_child(fill)
	return track


## Re-reads every row from `Settings`. Called on build, after every press, and
## by the host whenever something else could have changed a value (a RESET, or
## the debug menu's own audio fiddling) — cheap enough to be unconditional.
func refresh() -> void:
	if settings == null:
		return
	for entry in _knobs():
		var key: String = entry["key"]
		var control = value_controls.get(key)
		if control == null:
			continue
		if entry["kind"] == "toggle":
			(control as Button).text = "%s: %s" % [entry["label"], settings.format_value(key)]
		else:
			(control as Label).text = settings.format_value(key)
			_set_range(range_bars.get(key), settings.fraction(key))


func _set_range(track, fraction: float) -> void:
	if track == null:
		return
	var fill: Panel = (track as Panel).get_node_or_null("Fill")
	if fill == null:
		return
	var width: float = maxf((track as Panel).size.x, (track as Panel).custom_minimum_size.x)
	var height: float = maxf((track as Panel).size.y, (track as Panel).custom_minimum_size.y)
	var clamped: float = clampf(fraction, 0.0, 1.0)
	fill.position = Vector2.ZERO
	fill.size = Vector2(width * clamped, height)
	fill.visible = clamped > 0.0
	# The fraction lives on the track as well as in the pixels, because the
	# pixels are a layout result and a headless run has no layout pass —
	# `main.gd::_set_meter` records it for the same reason.
	(track as Panel).set_meta("fraction", clamped)


func _on_nudge_pressed(key: String, direction: int) -> void:
	if settings == null:
		return
	settings.nudge(key, direction)
	_persist()
	refresh()


func _on_reset_pressed() -> void:
	if settings == null:
		return
	settings.reset()
	_persist()
	refresh()


## Written on every press rather than on BACK. A settings screen that loses
## the change when the game is closed from the pause menu — or crashes — has
## failed at the one thing it is for, and the file is eight keys of text.
func _persist() -> void:
	var error: String = settings.save_file()
	if error != "":
		printerr("CONDIMENTAL: %s" % error)


# --------------------------------------------------------------------------
# Chrome. `main.gd`'s `_ui_px`/`_ui_i`/`_ui_font`, against the scale the host
# handed over.
# --------------------------------------------------------------------------

func _px(n: float) -> float:
	return round(n * _ui_scale)


func _i(n: float) -> int:
	return int(round(n * _ui_scale))


func _font(delta: int) -> int:
	return maxi(8, int(round(float(_font_base + delta) * _ui_scale)))


func _label_settings(size: int, colour: Color) -> LabelSettings:
	var s := LabelSettings.new()
	s.font_size = size
	s.font_color = colour
	s.outline_size = _i(4)
	s.outline_color = Color(0, 0, 0, 0.85)
	return s


func _rule() -> Panel:
	var rule := Panel.new()
	rule.custom_minimum_size = Vector2(0.0, _px(2.0))
	rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var style := StyleBoxFlat.new()
	style.bg_color = Color(ACCENT.r, ACCENT.g, ACCENT.b, 0.55)
	rule.add_theme_stylebox_override("panel", style)
	return rule


## `menu.gd::_menu_button`'s styling, restated here rather than reached for:
## this file cannot preload `main.gd` (cycle) and must not depend on `menu.gd`
## (which is a scene, not a library). The three sets agree by construction —
## same plate, same accent, same states.
func _button(node_name: String, text: String) -> Button:
	var button := Button.new()
	button.name = node_name
	button.text = text
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_size_override("font_size", _font(1))
	button.add_theme_color_override("font_color", INK)
	button.add_theme_color_override("font_hover_color", HEADING)
	button.add_theme_color_override("font_pressed_color", HEADING)
	for state in ["normal", "hover", "pressed", "focus", "disabled"]:
		button.add_theme_stylebox_override(state, _button_style(state))
	return button


func _button_style(state: String) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.13, 0.13, 0.15, 0.96)
	style.border_color = Color(0.30, 0.30, 0.33, 0.95)
	match state:
		"hover":
			style.bg_color = Color(0.19, 0.18, 0.14, 0.98)
			style.border_color = ACCENT
		"pressed":
			style.bg_color = Color(0.24, 0.21, 0.12, 0.98)
			style.border_color = ACCENT
		"disabled":
			style.bg_color = Color(0.10, 0.10, 0.11, 0.85)
	style.set_border_width_all(_i(2))
	style.set_content_margin_all(_px(7.0))
	return style
