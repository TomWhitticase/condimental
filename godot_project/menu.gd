## The main menu — the first thing the player sees (the "Save/load + a real
## main menu" epic, ticket 2). New Game, Load Game, Quit, modelled on the
## screen Minecraft and Factorio both open on.
##
## It is `project.godot`'s `run/main_scene` now; `main.tscn` is what this
## changes to. The handoff between the two is `game_session.gd`, an autoload
## that outlives both scenes — see that file for why a singleton rather than
## a scene parameter.
##
## **Style: this is the game's own industrial UI, not a default Godot theme**
## (UI_UX.md §5). Everything here is built in code, like every other panel in
## this project, out of the same three ingredients `main.gd`'s HUD uses: a
## near-opaque dark plate, a hazard-yellow rule along its top edge, and
## outlined sturdy text. A menu drawn in Godot's stock grey would be the one
## screen in the game that does not look like the game, and it is the screen
## that sets the expectation for everything after it.
##
## **A save is validated BEFORE the scene changes.** Selecting a slot reads
## the file and runs it through a throwaway `SimBridge.load_ron`, whose
## message is shown verbatim beside the list on failure — so a corrupt or
## older-build save is refused on the one screen still able to say why,
## rather than after `main.tscn` has already replaced this one. See
## `_arm_slot`.
extends Control

const GameSaves = preload("res://game_session.gd")
const TitleScene = preload("res://title_scene.gd")

# --- Plate palette, shared with main.gd's HUD ------------------------------
## Deliberately the same three colours `main.gd` builds every panel from, so
## the menu and the game read as one product. They are repeated rather than
## imported because `main.gd` builds them inline too and neither file should
## have to load the other just to draw a rectangle.
const PLATE_BG := Color(0.06, 0.06, 0.07, 0.94)
const PLATE_ACCENT := Color(0.85, 0.63, 0.11, 0.95)
const INK := Color(0.92, 0.93, 0.94)
const INK_DIM := Color(0.66, 0.68, 0.70)
const HAZARD := Color(0.98, 0.85, 0.42)
const ERROR_INK := Color(0.97, 0.55, 0.48)
## The splash line is the one yellow on this screen that is not the hazard
## stripe: brighter, so it reads as a sticker slapped on the poster.
const SPLASH_INK := Color(1.0, 0.94, 0.30)
const SPLASH_TILT_DEGREES := -7.0
const SPLASH_PULSE := 0.055
const SPLASH_BEATS_PER_SECOND := 0.9

## The lines themselves. In-jokes about this game and this codebase, the way
## Minecraft's are about that one — the point of a splash line is that it
## could only belong to the game it is on.
const SPLASH_LINES := [
	"Now with 100% more tomato!",
	"Deterministic to the last tick!",
	"Fixed-point, never float!",
	"Automate the condiment!",
	"The vinegar is load-bearing",
	"Mind the conveyor!",
	"Ketchup is a vegetable",
	"Sugar in, sauce out",
	"Belt, belt, corner, belt",
	"Pressure-equalized!",
	"Made of real voxels",
	"Bottles all the way down",
	"Also try the mayonnaise!",
	"The hopper is full, boss",
	"Every asset is a cube",
	"Contractually obliged",
	"Squeezy!",
	"No gameplay in GDScript!",
	"Shipping from the dock",
	"Batch number 4,096",
]

## The backdrop is a REAL FACTORY, standing in the same hall the game is
## played in (`title_scene.gd`) — the thing Minecraft's panorama and
## Factorio's title art both are, asked for in those words.
##
## It replaces a flat industrial charcoal plate, which was chosen on the
## argument that `main.tscn` costs a full voxel-asset load and a lit 3D frame
## to stand behind a menu the player leaves in two seconds. That cost is real
## and is not paid: `title_scene.gd` loads none of `main.tscn` and builds no
## simulation — it is about twenty hand-placed models on a floor, in the same
## hall, lit by the same two lights. What it buys is the first screen telling
## the player what the game is, which a charcoal rectangle cannot.
##
## The scrim below is the price of putting text over a photograph: a dark
## wash across the side the menu column sits on, so the wordmark and the
## buttons keep their contrast wherever the drifting camera happens to be.
const SCRIM_INK := Color(0.03, 0.03, 0.04, 0.88)
## How far across the screen the scrim has faded out entirely.
const SCRIM_SPAN := 0.62
## **Every size here is measured in `main.gd`'s `UI_SCALE`**, read from that
## script rather than repeated, so the menu and the HUD cannot drift apart
## (the menu was written alongside the 2026-09-09 UI-scale pass and
## reconciled with it at merge). The sizes below are member variables rather
## than constants only because a constant cannot multiply by a value read
## from another script; they are set once and never written again.
const MainScript := preload("res://main.gd")
var UI_SCALE: float = MainScript.UI_SCALE

## How far in from the left edge the menu column stands. The column is on
## one side rather than centred because the backdrop is a photograph now:
## a panel in the middle of it covers the vanishing point the whole
## composition is aimed at.
var COLUMN_MARGIN: float = 86.0 * UI_SCALE

## The same 15px design base `main.gd`'s `UI_FONT_BASE` uses; every font
## size here is an offset from it, scaled — `main.gd`'s `_ui_font` rule.
const BASE_FONT_SIZE := 15
var BODY_FONT_SIZE: int = int(round(BASE_FONT_SIZE * UI_SCALE))
## The wordmark stands on its own over the scene now instead of inside the
## panel, so it is nearly twice the size it was.
var TITLE_FONT_SIZE: int = int(round((BASE_FONT_SIZE + 61) * UI_SCALE))
var SUBTITLE_FONT_SIZE: int = int(round((BASE_FONT_SIZE + 4) * UI_SCALE))
var SPLASH_FONT_SIZE: int = int(round((BASE_FONT_SIZE + 4) * UI_SCALE))
var BUTTON_FONT_SIZE: int = int(round((BASE_FONT_SIZE + 5) * UI_SCALE))
var ROW_FONT_SIZE: int = int(round((BASE_FONT_SIZE + 2) * UI_SCALE))
var BUTTON_MIN_SIZE: Vector2 = Vector2(260.0, 52.0) * UI_SCALE
var PANEL_MARGIN: float = 26.0 * UI_SCALE
var PANEL_MIN_WIDTH: float = 560.0 * UI_SCALE
## The width left inside the panel's own margins — what every full-width
## child (the list, the rows, the error line) is sized to.
var PANEL_INNER_WIDTH: float = PANEL_MIN_WIDTH - PANEL_MARGIN * 2.0
## The load list scrolls rather than pushing the panel off the screen. The
## cap is not a fixed number: the wordmark stands outside the panel now, so
## what is left for the list depends on the window — see
## `_list_height_budget`. These two only bound that measurement, so a very
## short window still shows a scrollable list rather than a sliver, and a
## very tall one does not stretch the plate down the whole screen.
var SAVE_LIST_MIN_HEIGHT: float = 110.0 * UI_SCALE
var SAVE_LIST_MAX_HEIGHT: float = 300.0 * UI_SCALE


## `n` design pixels at the current scale — `main.gd`'s `_ui_px`.
func _px(n: float) -> float:
	return round(n * UI_SCALE)


## `_px` for the properties Godot types as `int` — `main.gd`'s `_ui_i`.
func _i(n: float) -> int:
	return int(round(n * UI_SCALE))

## The 3D factory standing behind the menu (`title_scene.gd`).
var _title_scene: Node3D
## Minecraft's splash line, and the clock it beats on.
var _splash_label: Label
var _splash_beat := 0.0

var _root_box: VBoxContainer
var _button_box: VBoxContainer
var _load_box: VBoxContainer
var _save_rows_box: VBoxContainer
var _save_rows: Array = [] # Array[Dictionary]: the rows `list_saves` returned
var _empty_label: Label
var _error_label: Label
var _new_game_button: Button
var _load_button: Button
var _quit_button: Button
var _back_button: Button
## Which half of the menu is showing. The load list replaces the three
## buttons rather than opening beside them: this is one panel, and a modal
## list over a modal menu is two things to close.
var _load_open := false


func _ready() -> void:
	# `_ready` runs twice under the headless test scripts (once from
	# `add_child`, once called directly, which is this project's established
	# test shape). Rebuilding the whole menu the second time would leave two
	# of everything parented to the same node, so the second pass just
	# refreshes the list.
	if _root_box != null:
		_refresh_saves()
		return
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_build_backdrop()
	_build_panel()
	_refresh_saves()
	# The list's height is a function of the window's, so it is re-measured
	# when the window changes rather than only at build time. The headless
	# tests call `_ready` on a menu that is not in a tree, and a menu with no
	# window has nothing to follow.
	if is_inside_tree():
		get_viewport().size_changed.connect(_apply_list_height)
	# The score starts here, on the first screen, and carries into the
	# factory: the `Music` autoload outlives this scene (see music.gd).
	var music := _music()
	if music != null:
		music.start()


func _build_backdrop() -> void:
	# The factory itself, in 3D, behind everything. It is added first so the
	# whole Control tree draws over it: Godot renders a viewport's 3D world
	# under its 2D, whatever the order of the nodes.
	_title_scene = TitleScene.new()
	add_child(_title_scene)

	# A dark wash from the left edge, fading out before the middle of the
	# screen. Without it the wordmark sits over whatever the drifting camera
	# is pointing at — a lit skylight one minute, a dark plinth the next —
	# and an outline alone cannot hold text against both.
	var gradient := Gradient.new()
	gradient.set_color(0, SCRIM_INK)
	gradient.set_color(1, Color(SCRIM_INK.r, SCRIM_INK.g, SCRIM_INK.b, 0.0))
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.width = 256
	texture.height = 1
	texture.fill_from = Vector2(0.0, 0.0)
	texture.fill_to = Vector2(1.0, 0.0)

	var scrim := TextureRect.new()
	scrim.name = "Scrim"
	scrim.texture = texture
	scrim.stretch_mode = TextureRect.STRETCH_SCALE
	scrim.set_anchors_and_offsets_preset(Control.PRESET_LEFT_WIDE)
	scrim.anchor_right = SCRIM_SPAN
	scrim.offset_right = 0.0
	scrim.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(scrim)


func _build_panel() -> void:
	var style := StyleBoxFlat.new()
	style.bg_color = PLATE_BG
	style.border_color = PLATE_ACCENT
	style.border_width_top = _i(4)
	style.border_width_left = _i(2)
	style.border_width_right = _i(2)
	style.border_width_bottom = _i(2)
	style.set_content_margin_all(PANEL_MARGIN)

	# One column down the left of the screen: the wordmark, the splash line
	# and the panel, vertically centred, with the factory filling the rest of
	# the frame. A MarginContainer holds it off the edge and a VBox that
	# SHRINKS to its contents keeps the column as narrow as the panel, rather
	# than stretching a plate across a backdrop that is the point of the
	# screen. (It used to be a CenterContainer: that preset is computed from
	# the panel's minimum size at the moment it is applied, and at build time
	# the labels inside have no text yet — the first cut put the whole menu
	# in the bottom-right corner, which is the kind of thing only a rendered
	# frame shows, UI_UX.md §2b point 12. A container re-lays its child
	# whenever the child resizes, which is why one is still used here.)
	var margins := MarginContainer.new()
	margins.name = "MenuMargins"
	margins.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	margins.add_theme_constant_override("margin_left", int(COLUMN_MARGIN))
	margins.add_theme_constant_override("margin_right", int(COLUMN_MARGIN))
	margins.add_theme_constant_override("margin_top", _i(28))
	margins.add_theme_constant_override("margin_bottom", _i(28))
	margins.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(margins)

	var column := VBoxContainer.new()
	column.name = "MenuColumn"
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	column.add_theme_constant_override("separation", _i(10))
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	margins.add_child(column)

	_build_wordmark(column)

	var panel := PanelContainer.new()
	panel.name = "MenuPanel"
	panel.add_theme_stylebox_override("panel", style)
	panel.custom_minimum_size = Vector2(PANEL_MIN_WIDTH, 0.0)
	panel.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	column.add_child(panel)

	_root_box = VBoxContainer.new()
	_root_box.name = "MenuBody"
	_root_box.add_theme_constant_override("separation", _i(14))
	panel.add_child(_root_box)

	_button_box = VBoxContainer.new()
	_button_box.name = "MenuButtons"
	_button_box.add_theme_constant_override("separation", _i(10))
	_root_box.add_child(_button_box)

	_new_game_button = _menu_button("NewGameButton", "NEW GAME")
	_new_game_button.pressed.connect(_on_new_game_pressed)
	_button_box.add_child(_new_game_button)

	_load_button = _menu_button("LoadGameButton", "LOAD GAME")
	_load_button.pressed.connect(_on_load_pressed)
	_button_box.add_child(_load_button)

	_quit_button = _menu_button("QuitButton", "QUIT")
	_quit_button.pressed.connect(_on_quit_pressed)
	_button_box.add_child(_quit_button)

	_build_load_view()

	# The error line lives OUTSIDE the load view, at the bottom of the panel,
	# because it has to survive the view it was produced in: a refused load
	# leaves the list up, and a refused anything-else would otherwise have
	# nowhere to print.
	_error_label = Label.new()
	_error_label.name = "MenuError"
	_error_label.label_settings = _settings(BODY_FONT_SIZE, ERROR_INK, _i(4))
	_error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_error_label.custom_minimum_size = Vector2(PANEL_INNER_WIDTH, 0.0)
	_error_label.visible = false
	_root_box.add_child(_error_label)


## The wordmark, the tagline and the splash line — the top of the column,
## outside the panel. Minecraft and Factorio both put the game's name over
## the art rather than inside a box, and that is the whole difference between
## a title screen and a dialog.
func _build_wordmark(column: VBoxContainer) -> void:
	var title := Label.new()
	title.name = "Title"
	# A heavier outline than anything else in the game: this text stands over
	# a lit 3D frame, not over a plate, and the scrim behind it is a wash
	# rather than a background.
	title.label_settings = _settings(TITLE_FONT_SIZE, HAZARD, _i(9))
	title.text = "SAUCE FACTORY"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	column.add_child(title)

	# The tagline and the splash share one row, so the splash sits off the
	# wordmark's bottom-right corner the way Minecraft's does rather than
	# hanging under the middle of everything.
	var strap := HBoxContainer.new()
	strap.name = "Strap"
	strap.custom_minimum_size = Vector2(PANEL_MIN_WIDTH, 0.0)
	strap.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(strap)

	var subtitle := Label.new()
	subtitle.name = "Subtitle"
	subtitle.label_settings = _settings(SUBTITLE_FONT_SIZE, INK, _i(5))
	subtitle.text = "condiment production, automated"
	subtitle.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT
	subtitle.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	subtitle.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	strap.add_child(subtitle)

	# The splash line: yellow, tilted, and beating slowly — Minecraft's, and
	# the one piece of pure decoration on the screen. It is tilted by
	# transforming the Label INSIDE a plain `Control`, because a container
	# lays its children out axis-aligned and would undo the rotation.
	var splash_slot := Control.new()
	splash_slot.name = "SplashSlot"
	splash_slot.custom_minimum_size = Vector2(PANEL_MIN_WIDTH * 0.62, _px(44.0))
	splash_slot.mouse_filter = Control.MOUSE_FILTER_IGNORE
	splash_slot.clip_contents = false
	strap.add_child(splash_slot)

	_splash_label = Label.new()
	_splash_label.name = "SplashText"
	_splash_label.label_settings = _settings(SPLASH_FONT_SIZE, SPLASH_INK, _i(5))
	_splash_label.text = _splash_line()
	_splash_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_splash_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_splash_label.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_splash_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_splash_label.rotation = deg_to_rad(SPLASH_TILT_DEGREES)
	splash_slot.add_child(_splash_label)
	# The pivot has to follow the slot's real size, which is not known until
	# the container has laid out at least once.
	splash_slot.resized.connect(_recentre_splash.bind(splash_slot))
	_recentre_splash(splash_slot)


func _recentre_splash(slot: Control) -> void:
	if _splash_label == null:
		return
	_splash_label.pivot_offset = slot.size * 0.5


## Which splash line is showing. Derived from the DATE, never `randf()`:
## godot_project/CLAUDE.md requires visual randomness to come from an index
## so a review screenshot repeats, and "a different one every day" is as
## close to Minecraft's per-launch pick as that rule allows.
func _splash_line() -> String:
	var date := Time.get_date_dict_from_system()
	var index: int = (int(date.get("year", 0)) * 372 + int(date.get("month", 1)) * 31
			+ int(date.get("day", 1)))
	return SPLASH_LINES[index % SPLASH_LINES.size()]


## The splash line's slow beat, and the only thing on this screen that is a
## function of wall time. The 3D backdrop drifts on its own clock
## (`title_scene.gd`); there is no simulation on this screen to take a tick
## count from.
func _process(delta: float) -> void:
	if _splash_label == null:
		return
	_splash_beat += delta
	var pulse := 1.0 + SPLASH_PULSE * absf(sin(_splash_beat * SPLASH_BEATS_PER_SECOND * PI))
	_splash_label.scale = Vector2(pulse, pulse)


func _build_load_view() -> void:
	_load_box = VBoxContainer.new()
	_load_box.name = "LoadView"
	_load_box.add_theme_constant_override("separation", _i(8))
	_load_box.visible = false
	_root_box.add_child(_load_box)

	var heading := Label.new()
	heading.label_settings = _settings(BUTTON_FONT_SIZE, INK, _i(4))
	heading.text = "LOAD GAME"
	_load_box.add_child(heading)

	_empty_label = Label.new()
	_empty_label.name = "NoSavesLabel"
	_empty_label.label_settings = _settings(BODY_FONT_SIZE, INK_DIM, _i(4))
	_empty_label.text = "No saves yet — start a new game and the factory will autosave itself."
	_empty_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_empty_label.custom_minimum_size = Vector2(PANEL_INNER_WIDTH, 0.0)
	_load_box.add_child(_empty_label)

	var scroll := ScrollContainer.new()
	scroll.name = "SaveScroll"
	scroll.custom_minimum_size = Vector2(PANEL_INNER_WIDTH, 0.0)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_load_box.add_child(scroll)

	_save_rows_box = VBoxContainer.new()
	_save_rows_box.name = "SaveRows"
	_save_rows_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_save_rows_box.add_theme_constant_override("separation", _i(6))
	scroll.add_child(_save_rows_box)

	_load_box.add_child(_rule())

	_back_button = _menu_button("BackButton", "BACK")
	_back_button.pressed.connect(_on_back_pressed)
	_load_box.add_child(_back_button)


# --------------------------------------------------------------------------
# Widgets
# --------------------------------------------------------------------------

func _settings(size: int, colour: Color, outline: int) -> LabelSettings:
	var s := LabelSettings.new()
	s.font_size = size
	s.font_color = colour
	s.outline_size = outline
	s.outline_color = Color(0, 0, 0, 0.85)
	return s


## A hazard-yellow hairline, the divider the rest of the UI already uses as
## the top border of every plate — here as a rule inside one.
func _rule() -> Panel:
	var rule := Panel.new()
	rule.custom_minimum_size = Vector2(0.0, _px(2.0))
	rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var style := StyleBoxFlat.new()
	style.bg_color = Color(PLATE_ACCENT.r, PLATE_ACCENT.g, PLATE_ACCENT.b, 0.55)
	rule.add_theme_stylebox_override("panel", style)
	return rule


func _menu_button(node_name: String, text: String) -> Button:
	var button := Button.new()
	button.name = node_name
	button.text = text
	button.custom_minimum_size = BUTTON_MIN_SIZE
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_size_override("font_size", BUTTON_FONT_SIZE)
	button.add_theme_color_override("font_color", INK)
	button.add_theme_color_override("font_hover_color", HAZARD)
	button.add_theme_color_override("font_pressed_color", HAZARD)
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
			style.border_color = PLATE_ACCENT
		"pressed":
			style.bg_color = Color(0.24, 0.21, 0.12, 0.98)
			style.border_color = PLATE_ACCENT
		"disabled":
			style.bg_color = Color(0.10, 0.10, 0.11, 0.85)
	style.set_border_width_all(_i(2))
	style.set_content_margin_all(_px(10.0))
	return style


# --------------------------------------------------------------------------
# The save list
# --------------------------------------------------------------------------

## Rebuilds the list from disk. Called on `_ready` and every time the load
## view opens, so a save written by a session that has since quit to this
## menu shows up without a restart.
func _refresh_saves() -> void:
	_save_rows = GameSaves.list_saves()
	if _save_rows_box == null:
		return
	for child in _save_rows_box.get_children():
		child.queue_free()
		_save_rows_box.remove_child(child)
	for index in range(_save_rows.size()):
		_save_rows_box.add_child(_build_save_row(index))
	if _empty_label != null:
		_empty_label.visible = _save_rows.is_empty()
	_apply_list_height()


func _apply_list_height() -> void:
	var scroll := _load_box.get_node_or_null("SaveScroll") if _load_box != null else null
	if scroll == null:
		return
	# Zeroed first so the load box reports what it needs WITHOUT the list,
	# which is what `_list_height_budget` measures against.
	(scroll as ScrollContainer).custom_minimum_size = Vector2(PANEL_INNER_WIDTH, 0.0)
	var wanted := float(_save_rows.size()) * (BUTTON_MIN_SIZE.y + _px(6.0))
	var height := clampf(
		minf(wanted, _list_height_budget()), SAVE_LIST_MIN_HEIGHT, SAVE_LIST_MAX_HEIGHT
	)
	(scroll as ScrollContainer).custom_minimum_size = Vector2(PANEL_INNER_WIDTH, height)


## How tall the save list may grow before it scrolls: the window, less
## everything else on the load view. Measured off the nodes rather than
## restated as numbers, so it keeps following the fonts and paddings if
## either changes.
##
## This exists because the wordmark left the panel when the backdrop became a
## real scene. A list capped at a flat 450px used to fit under a title inside
## the plate; under a 114pt one standing above it, at 1600x900, it pushed
## BACK clean off the bottom edge — a frame nobody had rendered since the
## column was rearranged (UI_UX.md §2b point 12).
func _list_height_budget() -> float:
	if not is_inside_tree():
		# No window to measure against (the headless tests build the menu
		# outside a tree); the flat cap is the honest answer.
		return SAVE_LIST_MAX_HEIGHT
	var used := _px(28.0) * 2.0 # the screen margins, top and bottom
	used += PANEL_MARGIN * 2.0 # the plate's own padding
	var column := get_node_or_null("MenuMargins/MenuColumn")
	if column != null:
		var separation := float(column.get_theme_constant("separation"))
		for child in column.get_children():
			# Everything standing above the plate: the wordmark and its strap.
			if child is Control and child.name != "MenuPanel":
				used += (child as Control).get_combined_minimum_size().y + separation
	if _load_box != null:
		# The heading, the rule and BACK — the list itself is zeroed by the
		# caller before this is read.
		used += _load_box.get_combined_minimum_size().y
	return float(get_viewport_rect().size.y) - used


## One save, as a button: its slot name, and — the honest second fact, read
## from the file's own mtime rather than from its contents — when it was last
## written. The autosave is labelled as such rather than hidden or sorted
## specially: it is a save like any other and the player should be able to
## fall back on it, but they should also be able to see that it is the one
## the game overwrites by itself.
func _build_save_row(index: int) -> Button:
	var row: Dictionary = _save_rows[index]
	var button := _menu_button("SaveRow_%d" % index, "")
	button.custom_minimum_size = Vector2(PANEL_INNER_WIDTH, BUTTON_MIN_SIZE.y)
	button.add_theme_font_size_override("font_size", ROW_FONT_SIZE)
	var label := String(row["slot"])
	if bool(row["autosave"]):
		label = "%s  (autosave)" % label
	button.text = "%s          %s" % [label, String(row["modified"])]
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.pressed.connect(_on_save_row_pressed.bind(index))
	return button


# --------------------------------------------------------------------------
# Actions
# --------------------------------------------------------------------------

## The `GameSession` autoload, or null. Reached through the main loop's own
## root rather than through an absolute node path, because the headless test
## scripts instantiate this scene OUTSIDE the active scene tree and an
## absolute path resolved from there is an engine error rather than a null.
## `main.gd._session` is the same function for the same reason.
func _session() -> Node:
	var loop := Engine.get_main_loop()
	if not (loop is SceneTree):
		return null
	var tree_root: Node = (loop as SceneTree).root
	return tree_root.get_node_or_null("GameSession") if tree_root != null else null


func _show_error(text: String) -> void:
	if _error_label == null:
		return
	_error_label.text = text
	_error_label.visible = text != ""


func _on_new_game_pressed() -> void:
	_show_error("")
	var session := _session()
	if session != null:
		session.arm_new_game()
	_start_game()


func _on_load_pressed() -> void:
	_show_error("")
	_set_load_open(true)


func _on_back_pressed() -> void:
	_show_error("")
	_set_load_open(false)


func _set_load_open(open: bool) -> void:
	_load_open = open
	if _button_box != null:
		_button_box.visible = not open
	if _load_box != null:
		_load_box.visible = open
	# After the swap, not before: `_refresh_saves` sizes the list against the
	# space the visible load view leaves.
	if open:
		_refresh_saves()


func _on_quit_pressed() -> void:
	if is_inside_tree():
		# Through the score, which stops itself first — see music.gd's
		# `quit_tree`.
		var music := _music()
		if music != null:
			music.quit_tree()
		else:
			get_tree().quit()


func _on_save_row_pressed(index: int) -> void:
	var error := _arm_slot(index)
	if error != "":
		_show_error(error)
		return
	_show_error("")
	_start_game()


## Reads one save, PROVES it loads, and arms the session with it. Returns ""
## on success or the reason it was refused.
##
## The proof is a throwaway `SimBridge`: `load_ron` builds a whole new world
## and only swaps it in once every step succeeded, so running the text
## through one here costs a load and answers the question this screen has to
## answer before it can leave. Doing it in `main.tscn` instead would mean
## discovering a bad file on a screen with no list, no Back button and a
## world already thrown away — which is exactly the "silently starts a fresh
## world" failure this ticket is told not to have.
func _arm_slot(index: int) -> String:
	if index < 0 or index >= _save_rows.size():
		return "That save is no longer there."
	var row: Dictionary = _save_rows[index]
	var path := String(row["path"])
	var text := GameSaves.read_save(path)
	if text == "":
		return "Could not read %s." % path
	var probe = SimBridge.new()
	var refusal: String = probe.load_ron(text)
	if refusal != "":
		return "Could not load '%s': %s" % [String(row["slot"]), refusal]
	var session := _session()
	if session != null:
		session.arm_load(path, String(row["slot"]))
	return ""


## Hands over to the game. The `is_inside_tree` guard is for the headless
## test scripts, which instantiate this scene outside the active tree — there
## is no tree to change there, and the assertions those tests make are about
## what was ARMED on the session, which has already happened by this point.
## The `Music` autoload, or null — reached the same way as `_session()`.
func _music() -> Node:
	var loop := Engine.get_main_loop()
	if not (loop is SceneTree):
		return null
	var tree_root: Node = (loop as SceneTree).root
	return tree_root.get_node_or_null("Music") if tree_root != null else null


func _start_game() -> void:
	if is_inside_tree():
		get_tree().change_scene_to_file("res://main.tscn")
