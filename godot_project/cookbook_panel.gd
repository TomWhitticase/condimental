## THE COOKBOOK: what this factory can make, and how.
##
## The game has never explained itself. A player who has just bought a mixer
## has no way to learn what it makes, what feeds it, or what the thing it
## makes is worth — and every one of those facts already exists, in
## `data/*.ron`, where only the loader has ever read them. This screen is
## those files, rendered for a human.
##
## Reached by `preload`, never `class_name` (godot_project/CLAUDE.md).
##
## # Nothing here is authored
##
## The binding requirement the feature was asked for is that adding a sauce,
## rebalancing a recipe, renaming an item or swapping an asset changes this
## screen with no edit to this file. So this file contains no sauce name, no
## machine name, no ingredient, no tier, no price and no arrow: it draws
## `sim.cookbook_json()` and nothing else. The derivation — tiers, families,
## edges, cost per case, the pay range, and which pages a gate is still
## standing over — is `sim_core/src/sim/cookbook.rs`'s, because the sim
## decides and Godot renders (the one rule).
##
## The two consequences worth stating, since both look like omissions:
##
## - **There is no fallback content.** A sauce with no `blurb` gets no blurb
##   paragraph, not a sentence this file made up. Writing one here would be
##   the hardcoding the feature exists to avoid, and it would be a second
##   place to fix when the real one is written.
## - **No number is computed here.** Cost per case, the pay range and the
##   delivery count all arrive finished; seconds are the one exception, and
##   only because `TICK_HZ` is GDScript's constant (`main.gd`), so the wire
##   carries ticks and the host hands the divisor over.
##
## # It is a COLUMN, not a plate
##
## `build()` returns a `VBoxContainer` the host drops into the plate it has,
## exactly as `settings_panel.gd` does and for the same reason: the Cookbook
## is hosted twice, as the B modal and in place of the pause menu's button
## column. `ui_scale`/`font_base` are arguments rather than reads off
## `main.gd` because `main.gd` preloads this file and a preload cycle does
## not load.
##
## # The tree draws itself
##
## The sauce tree is the first `_draw()` in this project, and the discipline
## that makes it testable is `main.gd::_set_meter`'s: the layout is computed
## into a plain dictionary, put on the canvas with `set_meta("layout", ...)`,
## and `_draw()` renders only from that. A headless run never lays a Control
## out, so a layout that lived in laid-out rectangles would be invisible to
## every test — the meta is the thing `test_cookbook.gd` asserts against, and
## the pixels are downstream of it. The nodes themselves are real Buttons at
## explicit positions for the same reason: a click is then a `pressed`
## signal, not a hit test against a rectangle nobody measured.
extends RefCounted

## The plate's accent and inks — `settings_panel.gd`'s four, which are
## `menu.gd`'s `PLATE_ACCENT` and `main.gd`'s border colour. One hazard
## yellow, everywhere.
const ACCENT := Color(0.85, 0.63, 0.11, 0.95)
const HEADING := Color(0.98, 0.85, 0.42)
const INK := Color(0.92, 0.93, 0.94)
const INK_DIM := Color(0.66, 0.68, 0.70)

const PLATE_BG := Color(0.13, 0.13, 0.15, 0.96)
const PLATE_EDGE := Color(0.30, 0.30, 0.33, 0.95)
## A locked plate is edged and inked down, and says LOCKED beside a padlock —
## the dimming is never the only thing saying so (UI_UX.md §7).
const LOCKED_BG := Color(0.10, 0.10, 0.11, 0.92)
const LOCKED_EDGE := Color(0.26, 0.26, 0.28, 0.95)
## What a silhouetted icon is multiplied by. Grey rather than black: the
## icons are lit voxel renders on a dark plate, and multiplying towards black
## does not read as "hidden", it reads as "missing".
const SILHOUETTE := Color(0.40, 0.42, 0.46, 0.85)

## The arrow between two families, and the arrow between two process stages.
const WIRE := Color(0.85, 0.63, 0.11, 0.75)

## Design-space sizes, all through `_px`. `BODY_WIDTH` is the width every
## wrapped paragraph is PINNED to — godot_project/CLAUDE.md: a Label whose
## width a fit pass measures is pinned or it does not wrap.
const BODY_WIDTH := 560.0
const NODE_SIZE := Vector2(152.0, 108.0)
const NODE_GAP := Vector2(72.0, 24.0)
const TIER_CAPTION := 24.0
const STEP_SIZE := Vector2(98.0, 112.0)
const ARROW_WIDTH := 34.0
const NODE_ICON := 46.0
const TITLE_ICON := 56.0
const STEP_ICON := 40.0
const ROW_ICON := 30.0
const COUNT_WIDTH := 46.0

const VIEW_TREE := "tree"
const VIEW_SAUCE := "sauce"
const VIEW_MACHINE := "machine"
const VIEW_INGREDIENT := "ingredient"

var root: VBoxContainer
## The one node everything below the breadcrumb hangs off, emptied and
## rebuilt by `_rebuild()` on every navigation.
var body: VBoxContainer
## The tree view's canvas while the tree is showing, null on a page. What
## `tree_layout()` reads.
var canvas: Control
var back_button: Button
var trail_box: HBoxContainer

var _scroll: ScrollContainer
var _book := {}
var _view := {"kind": VIEW_TREE, "id": ""}
## The views walked through to get here, oldest first. `back()` pops it; the
## breadcrumb draws it.
var _trail: Array = []

var _ui_scale := 1.0
var _font_base := 15
var _tick_hz := 30.0
var _icon_for := Callable()
var _display_name := Callable()
var _on_close := Callable()


## Builds the column.
##
## `icon_for` is called as `icon_for(id, asset)` and must return a real
## Texture2D — `main.gd::_cookbook_icon`, which is the project's one
## documented resolution chain (wire icon name, then the model tables, then
## the placeholder swatch). This file deliberately has no icon table of its
## own: a second one is a second thing to leave stale.
##
## `display_name` is `main.gd::_kind_display_name`, for the same reason —
## content that title-cases wrong names itself in `data/*.ron`, and there is
## one place that knows it.
##
## `tick_hz` is the host's `TICK_HZ`. The wire carries ticks because ticks
## are what the sim has; seconds are what a player reads (UI_UX.md §7).
##
## `on_close` is what the footer button does at the top of the trail. Inside
## a page it reads BACK and walks up instead, because a page deep in the book
## has somewhere to go that is not "gone".
func build(
	ui_scale: float,
	font_base: int,
	tick_hz: float,
	icon_for: Callable,
	display_name: Callable,
	on_close: Callable
) -> VBoxContainer:
	_ui_scale = ui_scale
	_font_base = font_base
	_tick_hz = tick_hz
	_icon_for = icon_for
	_display_name = display_name
	_on_close = on_close

	root = VBoxContainer.new()
	root.name = "CookbookPanel"
	root.add_theme_constant_override("separation", _i(10))

	var heading := Label.new()
	heading.name = "CookbookHeading"
	heading.label_settings = _label_settings(_font(5), HEADING)
	heading.text = "THE COOKBOOK"
	heading.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root.add_child(heading)

	trail_box = HBoxContainer.new()
	trail_box.name = "CookbookTrail"
	trail_box.alignment = BoxContainer.ALIGNMENT_CENTER
	trail_box.add_theme_constant_override("separation", _i(6))
	root.add_child(trail_box)

	root.add_child(_rule())

	_scroll = ScrollContainer.new()
	_scroll.name = "CookbookScroll"
	# Both directions: the tree grows sideways with every new tier, and
	# UI_UX.md §5c's last resort for a panel that will not fit is to scroll
	# rather than to draw where it cannot be clicked.
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	root.add_child(_scroll)

	body = VBoxContainer.new()
	body.name = "CookbookBody"
	body.add_theme_constant_override("separation", _i(12))
	_scroll.add_child(body)

	root.add_child(_rule())

	var footer := HBoxContainer.new()
	footer.name = "CookbookFooter"
	footer.add_theme_constant_override("separation", _i(12))
	root.add_child(footer)

	back_button = _button("BackButton", "CLOSE")
	back_button.custom_minimum_size = Vector2(_px(150.0), _px(36.0))
	back_button.pressed.connect(_on_back_pressed)
	footer.add_child(back_button)

	var hint := Label.new()
	hint.name = "CookbookHint"
	hint.label_settings = _label_settings(_font(-3), INK_DIM)
	# Short on purpose: this row is the footer, so whatever is longest in it
	# sets the panel's width, and a Label's own minimum under-reports what it
	# draws by about the outline (§5c, `_fit_config_picker_name`). A sentence
	# narrower than the header paragraph never gets to decide either.
	hint.text = "read from the recipe files — it changes when they do"
	hint.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	hint.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	footer.add_child(hint)

	_rebuild()
	return root


# --------------------------------------------------------------------------
# What the host drives
# --------------------------------------------------------------------------

## Hands over a fresh `cookbook_json()` and redraws whatever is showing. The
## lock half of the snapshot moves with `lifetime_deliveries`, so the host
## re-reads it on a cadence while the panel is up and a gate line counts up
## under the player's eyes.
##
## An unchanged book redraws NOTHING: a poll twice a second would otherwise
## throw away the scroll position and every Button on the page mid-read.
## Compared by `hash()`, because Godot's `Dictionary ==` is identity.
func set_book(book: Dictionary) -> void:
	if not _book.is_empty() and _book.hash() == book.hash():
		return
	_book = book
	_rebuild()


## Back to the tree, forgetting the trail. What opening the panel does.
func show_tree() -> void:
	_trail.clear()
	_view = {"kind": VIEW_TREE, "id": ""}
	_rebuild()


## Opens a page, remembering where it was opened from.
func open(kind: String, id: String) -> void:
	if id == "":
		return
	_trail.append(_view)
	_view = {"kind": kind, "id": id}
	_rebuild()


## Up one step. False at the top, which is the host's cue to close.
func back() -> bool:
	if _trail.is_empty():
		return false
	_view = _trail.pop_back()
	_rebuild()
	return true


## What is showing, as `{"kind": ..., "id": ...}`. A test asserts against
## this rather than against which nodes happen to exist.
func view() -> Dictionary:
	return _view.duplicate()


## The tree's computed nodes and edges, or an empty dictionary on a page.
## The pixels are drawn from exactly this — see the header.
func tree_layout() -> Dictionary:
	if canvas == null:
		return {}
	return canvas.get_meta("layout", {})


## Sizes the scrolled area against the room the host has for the WHOLE
## column, measured with the scroll zeroed (`settings_panel.gd::fit`'s trick,
## for its reason: the scroll's own size is what is being solved for).
## `available <= 0` on an axis means no cap — the headless tests, which have
## no window.
func fit(available: Vector2) -> void:
	if _scroll == null or body == null or root == null:
		return
	var keep := _scroll.custom_minimum_size
	_scroll.custom_minimum_size = Vector2.ZERO
	var chrome := root.get_combined_minimum_size()
	_scroll.custom_minimum_size = keep

	var wanted := body.get_combined_minimum_size()
	var height := wanted.y
	if available.y > 0.0:
		height = minf(height, maxf(_px(160.0), available.y - chrome.y))

	var width := wanted.x
	# A vertical scrollbar is drawn INSIDE the container, and `body`'s own
	# minimum has never heard of it: ask for the content width alone and every
	# row loses its last column to the bar. An ingredient row ended "solid, by
	# bel" at 1600x900 — right-aligned text, so the bar ate the one word that
	# says whether it arrives by belt or by pipe.
	if height < wanted.y:
		var bar := _scroll.get_v_scroll_bar()
		if bar != null:
			width += bar.get_combined_minimum_size().x
	if available.x > 0.0:
		width = minf(width, maxf(_px(240.0), available.x))
	_scroll.custom_minimum_size = Vector2(width, height)


# --------------------------------------------------------------------------
# Navigation
# --------------------------------------------------------------------------

func _on_back_pressed() -> void:
	if back():
		return
	if _on_close.is_valid():
		_on_close.call()


func _rebuild() -> void:
	if body == null:
		return
	for child in body.get_children():
		body.remove_child(child)
		child.queue_free()
	canvas = null

	_rebuild_trail()

	var kind := String(_view.get("kind", VIEW_TREE))
	var id := String(_view.get("id", ""))
	if kind == VIEW_SAUCE:
		_build_sauce_page(id)
	elif kind == VIEW_MACHINE:
		_build_machine_page(id)
	elif kind == VIEW_INGREDIENT:
		_build_ingredient_page(id)
	else:
		_build_tree()

	back_button.text = "BACK" if not _trail.is_empty() else "CLOSE"
	fit(Vector2.ZERO)


## The breadcrumb: every view walked through, each one clickable, then the
## one showing as plain text. A trail of one (the tree) draws nothing —
## "THE COOKBOOK ›" over the tree is the heading said twice.
func _rebuild_trail() -> void:
	for child in trail_box.get_children():
		trail_box.remove_child(child)
		child.queue_free()
	if _trail.is_empty():
		trail_box.visible = false
		return
	trail_box.visible = true
	for depth in range(_trail.size()):
		var crumb := _button("Crumb_%d" % depth, _view_title(_trail[depth]))
		crumb.add_theme_font_size_override("font_size", _font(-3))
		crumb.pressed.connect(_on_crumb_pressed.bind(depth))
		trail_box.add_child(crumb)
		var caret := Label.new()
		caret.label_settings = _label_settings(_font(-3), INK_DIM)
		caret.text = "›"
		caret.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		trail_box.add_child(caret)
	var here := Label.new()
	here.name = "CrumbHere"
	here.label_settings = _label_settings(_font(-3), HEADING)
	here.text = _view_title(_view)
	here.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	trail_box.add_child(here)


func _on_crumb_pressed(depth: int) -> void:
	_view = _trail[depth]
	_trail.resize(depth)
	_rebuild()


## A view's name for the breadcrumb. A locked sauce is not named, here or
## anywhere — that is what "silhouette until unlocked" means.
##
## The tree's crumb is not "THE COOKBOOK": that is the heading directly above
## it, and a trail whose first word repeats the title spends a line saying
## nothing. It is what the view actually is.
func _view_title(entry: Dictionary) -> String:
	var kind := String(entry.get("kind", VIEW_TREE))
	var id := String(entry.get("id", ""))
	if kind == VIEW_TREE:
		return "SAUCE TREE"
	if kind == VIEW_SAUCE and bool(_sauce(id).get("locked", false)):
		return "LOCKED"
	return _name(id)


# --------------------------------------------------------------------------
# The tree
# --------------------------------------------------------------------------

## One column per tier, in tier order, and one arrow per "this family's sauce
## is an ingredient in that one". Both come off the wire whole; the only
## thing decided here is where the rectangles go.
func _build_tree() -> void:
	var sauces: Array = _book.get("sauces", [])
	if sauces.is_empty():
		# A sentence, not an empty box (UI_UX.md §5c). It is also the honest
		# reading: with no snapshot yet there is nothing to say.
		_paragraph(body, "CookbookEmpty", "no recipes loaded yet", INK_DIM)
		return

	_paragraph(
		body,
		"CookbookIntro",
		"Every sauce this factory knows how to make. A column is how far from raw it is; an arrow is a sauce that goes into another sauce. Click one for its page.",
		INK
	)

	var columns := {}
	var tiers: Array = []
	for entry in sauces:
		var tier := int(entry.get("tier", 0))
		if not columns.has(tier):
			columns[tier] = []
			tiers.append(tier)
		columns[tier].append(entry)
	tiers.sort()

	canvas = TreeCanvas.new()
	canvas.name = "CookbookTree"
	canvas.mouse_filter = Control.MOUSE_FILTER_IGNORE
	canvas.wire = WIRE
	canvas.line_width = _px(3.0)
	canvas.arrow = _px(9.0)

	var node_size := Vector2(_px(NODE_SIZE.x), _px(NODE_SIZE.y))
	var step := node_size + Vector2(_px(NODE_GAP.x), _px(NODE_GAP.y))
	var top := _px(TIER_CAPTION)

	var nodes: Array = []
	var placed := {}
	var extent := Vector2.ZERO
	for column in range(tiers.size()):
		var tier: int = tiers[column]
		var caption := Label.new()
		caption.name = "Tier_%d" % tier
		caption.label_settings = _label_settings(_font(-4), INK_DIM)
		caption.text = "TIER %d" % tier
		caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		caption.position = Vector2(column * step.x, 0.0)
		caption.size = Vector2(node_size.x, top)
		canvas.add_child(caption)

		var rows: Array = columns[tier]
		for row in range(rows.size()):
			var entry: Dictionary = rows[row]
			var id := String(entry.get("id", ""))
			var rect := Rect2(Vector2(column * step.x, top + row * step.y), node_size)
			placed[id] = rect
			canvas.add_child(_build_node(entry, rect))
			nodes.append({
				"id": id,
				"tier": tier,
				"locked": bool(entry.get("locked", false)),
				"rect": rect,
			})
			extent = Vector2(maxf(extent.x, rect.end.x), maxf(extent.y, rect.end.y))

	var edges: Array = []
	for edge in _book.get("edges", []):
		var from_id := String(edge.get("from", ""))
		var to_id := String(edge.get("to", ""))
		if not placed.has(from_id) or not placed.has(to_id):
			continue
		var from_rect: Rect2 = placed[from_id]
		var to_rect: Rect2 = placed[to_id]
		var start := Vector2(from_rect.end.x, from_rect.position.y + from_rect.size.y * 0.5)
		var finish := Vector2(to_rect.position.x, to_rect.position.y + to_rect.size.y * 0.5)
		# Orthogonal, out of the right face and into the left one, turning
		# once in the gutter between the columns. Diagonals read as "roughly
		# related"; a right angle reads as a pipe, which is what it is.
		var turn := (start.x + finish.x) * 0.5
		var points := PackedVector2Array([
			start, Vector2(turn, start.y), Vector2(turn, finish.y), finish,
		])
		edges.append({"from": from_id, "to": to_id, "via": String(edge.get("via", "")), "points": points})

		# The arrow is labelled with what travels along it, because "ketchup
		# goes into BBQ" is the whole content of the edge and an unlabelled
		# line only says "these two are connected".
		var via := Label.new()
		via.name = "Edge_%s_%s" % [from_id, to_id]
		via.label_settings = _label_settings(_font(-4), HEADING)
		via.text = _name(String(edge.get("via", "")))
		via.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		via.size = Vector2(_px(NODE_GAP.x), _px(18.0))
		via.position = Vector2(turn - _px(NODE_GAP.x) * 0.5, (start.y + finish.y) * 0.5 - _px(24.0))
		canvas.add_child(via)

	canvas.custom_minimum_size = extent
	canvas.set_meta("layout", {"nodes": nodes, "edges": edges})
	canvas.queue_redraw()
	body.add_child(canvas)

	for entry in sauces:
		if bool(entry.get("locked", false)):
			_paragraph(
				body,
				"CookbookLockedNote",
				"🔒 A locked sauce shows its gate instead of its name. It opens itself as you ship.",
				INK_DIM
			)
			break


## One sauce in the tree. Unlocked it is its icon, its name and how many
## stages it takes; locked it is a silhouette, the padlock, the word LOCKED
## and the gate — never the name, and never the shape of the chain.
func _build_node(entry: Dictionary, rect: Rect2) -> Button:
	var id := String(entry.get("id", ""))
	var locked := bool(entry.get("locked", false))

	var button := _plate_button("Node_%s" % id, locked)
	button.position = rect.position
	button.size = rect.size
	button.custom_minimum_size = rect.size
	button.pressed.connect(open.bind(VIEW_SAUCE, id))

	var column := VBoxContainer.new()
	column.name = "Contents"
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.add_theme_constant_override("separation", _i(2))
	column.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	button.add_child(column)

	var icon := _icon(id, String(entry.get("icon", "")), _px(NODE_ICON), locked)
	icon.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	column.add_child(icon)

	var title := Label.new()
	title.name = "Title"
	title.label_settings = _label_settings(_font(-2), INK_DIM if locked else HEADING)
	title.text = "🔒 LOCKED" if locked else _name(id)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	# Pinned before it wraps, godot_project/CLAUDE.md's rule — the node's own
	# width, which is a number this file chose.
	title.custom_minimum_size = Vector2(rect.size.x - _px(12.0), 0.0)
	title.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(title)

	var note := Label.new()
	note.name = "Note"
	note.label_settings = _label_settings(_font(-4), INK_DIM)
	note.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	note.custom_minimum_size = Vector2(rect.size.x - _px(12.0), 0.0)
	note.mouse_filter = Control.MOUSE_FILTER_IGNORE
	note.text = _gate_progress(entry.get("gate")) if locked else _stage_count(entry)
	column.add_child(note)

	return button


func _stage_count(entry: Dictionary) -> String:
	var steps: int = (entry.get("steps", []) as Array).size()
	return "1 stage" if steps == 1 else "%d stages" % steps


## `4 of 10 shipped`, or "" with no gate. The numbers are both the wire's —
## the distance is deliberately not subtracted here, because two counts say
## the same thing and neither is a number this file invented.
func _gate_progress(gate) -> String:
	if not (gate is Dictionary):
		return ""
	return "%d of %d shipped" % [int(gate.get("deliveries", 0)), int(gate.get("after_deliveries", 0))]


# --------------------------------------------------------------------------
# The sauce page
# --------------------------------------------------------------------------

func _build_sauce_page(id: String) -> void:
	var entry := _sauce(id)
	if entry.is_empty():
		_paragraph(body, "CookbookMissing", "no page for %s" % id, INK_DIM)
		return

	var locked := bool(entry.get("locked", false))
	if locked:
		_build_locked_page(entry)
		return

	_title_row(id, String(entry.get("icon", "")), _name(id), String(entry.get("tagline", "")))

	var inputs := _section("INGREDIENTS", "per case, all the way back to raw")
	for line in entry.get("inputs", []):
		inputs.add_child(_input_row(line))

	var process := _section("PROCESS", "left to right; NEW STAGE marks a machine no earlier sauce needed")
	process.add_child(_process_chain(entry))

	var numbers := _section("THE NUMBERS", "")
	_line(numbers, "Cost", "%.0f per case, in ingredients" % float(entry.get("cost_per_unit", 0.0)))
	_line(numbers, "Contracts pay", _pay_text(entry))

	var contracts: Array = entry.get("contracts", [])
	if not contracts.is_empty():
		var board := _section("WHO BUYS IT", "%d contract%s on the board name this sauce" % [contracts.size(), "" if contracts.size() == 1 else "s"])
		var names: Array[String] = []
		for name in contracts:
			names.append(String(name))
		_paragraph(board, "CookbookContracts", "  ·  ".join(names), INK)

	var blurb := String(entry.get("blurb", ""))
	if blurb != "":
		_paragraph(body, "CookbookBlurb", blurb, INK_DIM)


## A locked sauce's page. It is the gate and the gate only: the tree already
## refuses to name it, and a page that named its ingredients under a padlock
## would make the silhouette decorative.
func _build_locked_page(entry: Dictionary) -> void:
	var id := String(entry.get("id", ""))
	_title_row(id, String(entry.get("icon", "")), "🔒 LOCKED", "", true)

	var gate = entry.get("gate")
	var block := _section("THE GATE", "")
	if gate is Dictionary:
		_line(block, String(gate.get("name", "")), _gate_progress(gate))
		_paragraph(
			block,
			"CookbookGate",
			"This page writes itself at %d deliveries. Nothing needs buying and nothing needs choosing — keep shipping." % int(gate.get("after_deliveries", 0)),
			INK_DIM
		)
	else:
		_paragraph(block, "CookbookGate", "not yours yet", INK_DIM)


func _pay_text(entry: Dictionary) -> String:
	var low = entry.get("pay_low")
	var high = entry.get("pay_high")
	if low == null or high == null:
		return "nothing on the board asks for it yet"
	if float(low) == float(high):
		return "%.0f per case" % float(low)
	return "%.0f to %.0f per case, by contract" % [float(low), float(high)]


## One ingredient line. The whole row is the button, so the click target is
## the thing being read rather than a word inside it.
func _input_row(line: Dictionary) -> Button:
	var id := String(line.get("id", ""))
	var locked := bool(line.get("locked", false))
	var from_sauce = line.get("from_sauce")

	var button := _plate_button("Input_%s" % id, locked)
	button.custom_minimum_size = Vector2(_px(BODY_WIDTH), _px(38.0))
	if from_sauce == null:
		button.pressed.connect(open.bind(VIEW_INGREDIENT, id))
	else:
		button.pressed.connect(open.bind(VIEW_SAUCE, String(from_sauce)))

	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", _i(10))
	row.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	button.add_child(row)

	row.add_child(_icon(id, String(line.get("icon", "")), _px(ROW_ICON), locked))

	var count := Label.new()
	count.name = "Count"
	count.label_settings = _label_settings(_font(0), HEADING)
	count.text = "×%s" % _quantity(float(line.get("quantity", 0.0)))
	count.custom_minimum_size = Vector2(_px(COUNT_WIDTH), 0.0)
	count.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	count.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(count)

	var name_label := Label.new()
	name_label.name = "Name"
	name_label.label_settings = _label_settings(_font(0), INK)
	name_label.text = _name(id)
	name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(name_label)

	var tag := Label.new()
	tag.name = "Tag"
	tag.label_settings = _label_settings(_font(-3), INK_DIM)
	tag.text = _input_tag(line)
	tag.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	tag.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	tag.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(tag)

	return button


## What the row says about where the ingredient comes from. A fluid and a
## solid arrive along different things, and that is the single most useful
## thing a page can tell a player who is about to build the line.
##
## It names no MACHINE, deliberately: a machine name written here would be
## content authored in GDScript, and `test_cookbook.gd` checks every id and
## every display name in the book against this file's own text.
func _input_tag(line: Dictionary) -> String:
	var from_sauce = line.get("from_sauce")
	if from_sauce != null:
		return "made here, from %s" % _name(String(from_sauce))
	var where := "fluid, by pipe" if bool(line.get("fluid", false)) else "solid, by belt"
	if bool(line.get("locked", false)):
		return "🔒 locked  ·  %s" % where
	return "raw  ·  %s" % where


## The machine chain, one plate per stage with an arrow carrying that
## stage's output between them. Horizontally scrolled inside its own box so
## a long chain lengthens the scroll rather than the panel (§5c).
func _process_chain(entry: Dictionary) -> ScrollContainer:
	var scroll := ScrollContainer.new()
	scroll.name = "ProcessScroll"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.custom_minimum_size = Vector2(_px(BODY_WIDTH), _px(STEP_SIZE.y) + _px(26.0))

	var row := HBoxContainer.new()
	row.name = "Process"
	row.add_theme_constant_override("separation", _i(2))
	scroll.add_child(row)

	var steps: Array = entry.get("steps", [])
	for index in range(steps.size()):
		var step: Dictionary = steps[index]
		row.add_child(_step_plate(step))
		row.add_child(_step_arrow(step))
	return scroll


func _step_plate(step: Dictionary) -> Button:
	var machine := String(step.get("machine", ""))
	var button := _plate_button("Step_%s" % String(step.get("recipe", "")), false)
	button.custom_minimum_size = Vector2(_px(STEP_SIZE.x), _px(STEP_SIZE.y))
	button.pressed.connect(open.bind(VIEW_MACHINE, machine))

	var column := VBoxContainer.new()
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.add_theme_constant_override("separation", _i(2))
	column.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	button.add_child(column)

	var icon := _icon(machine, String(step.get("machine_icon", "")), _px(STEP_ICON), false)
	icon.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	column.add_child(icon)

	var name_label := Label.new()
	name_label.name = "Machine"
	name_label.label_settings = _label_settings(_font(-3), INK)
	name_label.text = _name(machine)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	name_label.custom_minimum_size = Vector2(_px(STEP_SIZE.x) - _px(10.0), 0.0)
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(name_label)

	var time_label := Label.new()
	time_label.name = "Time"
	time_label.label_settings = _label_settings(_font(-2), HEADING)
	time_label.text = _seconds(int(step.get("time_ticks", 0)))
	time_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	time_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(time_label)

	var tag := Label.new()
	tag.name = "Stage"
	tag.label_settings = _label_settings(_font(-4), HEADING if bool(step.get("new_machine", false)) else INK_DIM)
	tag.text = "NEW STAGE" if bool(step.get("new_machine", false)) else "SHARED"
	tag.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	tag.mouse_filter = Control.MOUSE_FILTER_IGNORE
	column.add_child(tag)

	return button


## The arrow after a stage, carrying that stage's output. Belt or pipe is
## said in words, because it decides what the player has to build between
## the two machines and the arrow itself cannot say it. It stops there: where
## the LAST stage's output goes is not in the snapshot, and a sentence about
## it would be this file claiming something no data file said.
func _step_arrow(step: Dictionary) -> VBoxContainer:
	var carries := String(step.get("carries", ""))
	var column := VBoxContainer.new()
	column.name = "Arrow_%s" % carries
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.custom_minimum_size = Vector2(_px(ARROW_WIDTH), 0.0)
	column.add_theme_constant_override("separation", _i(1))

	var glyph := Label.new()
	glyph.label_settings = _label_settings(_font(2), WIRE)
	glyph.text = "→"
	glyph.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(glyph)

	var what := Label.new()
	what.name = "Carries"
	what.label_settings = _label_settings(_font(-4), INK_DIM)
	what.text = "pipe" if bool(step.get("carries_fluid", false)) else "belt"
	what.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	what.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	what.custom_minimum_size = Vector2(_px(ARROW_WIDTH), 0.0)
	column.add_child(what)

	return column


# --------------------------------------------------------------------------
# The machine and ingredient pages
# --------------------------------------------------------------------------

func _build_machine_page(id: String) -> void:
	var entry := _machine(id)
	if entry.is_empty():
		_paragraph(body, "CookbookMissing", "no page for %s" % id, INK_DIM)
		return

	_title_row(id, String(entry.get("icon", "")), _name(id), "")

	var facts := _section("THE MACHINE", "")
	_line(facts, "Footprint", "%d × %d tiles" % [int(entry.get("width_tiles", 1)), int(entry.get("height_tiles", 1))])
	var price = entry.get("price")
	_line(facts, "Price", "not for sale" if price == null else "%.0f" % float(price))
	var crew: Array = entry.get("crew", [])
	if crew.is_empty():
		_line(facts, "Crew", "none — it runs unattended")
	else:
		for line in crew:
			_line(facts, String(line.get("name", "")), "×%d" % int(line.get("count", 0)))

	var makes: Array = entry.get("makes", [])
	if not makes.is_empty():
		var recipes := _section("WHAT IT MAKES", "the machine's inputs pick which one; there is no recipe picker")
		var names: Array[String] = []
		for recipe in makes:
			names.append(_name(String(recipe)))
		_paragraph(recipes, "CookbookMakes", "  ·  ".join(names), INK)

	var blurb := String(entry.get("blurb", ""))
	if blurb != "":
		_paragraph(body, "CookbookBlurb", blurb, INK_DIM)


func _build_ingredient_page(id: String) -> void:
	var entry := _ingredient(id)
	if entry.is_empty():
		_paragraph(body, "CookbookMissing", "no page for %s" % id, INK_DIM)
		return

	var locked := bool(entry.get("locked", false))
	_title_row(id, String(entry.get("icon", "")), _name(id), "", locked)

	var facts := _section("THE INGREDIENT", "")
	_line(facts, "Price", "%.0f each" % float(entry.get("price", 0.0)))
	_line(facts, "Arrives by", "pipe" if bool(entry.get("fluid", false)) else "belt")
	if locked:
		_line(facts, "🔒 Locked", _gate_line(entry.get("gate")))

	var used_in: Array = entry.get("used_in", [])
	if not used_in.is_empty():
		var uses := _section("WHAT IT GOES INTO", "")
		for sauce_id in used_in:
			uses.add_child(_sauce_link(String(sauce_id)))

	var blurb := String(entry.get("blurb", ""))
	if blurb != "":
		_paragraph(body, "CookbookBlurb", blurb, INK_DIM)


func _gate_line(gate) -> String:
	if not (gate is Dictionary):
		return "locked"
	return "%s — %s" % [String(gate.get("name", "")), _gate_progress(gate)]


## A link back to a sauce page, which is a locked sauce's silhouette again:
## an ingredient page must not be the back door that names it.
func _sauce_link(id: String) -> Button:
	var entry := _sauce(id)
	var locked := bool(entry.get("locked", false))
	var button := _plate_button("Uses_%s" % id, locked)
	button.custom_minimum_size = Vector2(_px(BODY_WIDTH), _px(40.0))
	button.pressed.connect(open.bind(VIEW_SAUCE, id))

	var row := HBoxContainer.new()
	row.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_theme_constant_override("separation", _i(10))
	row.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	button.add_child(row)

	row.add_child(_icon(id, String(entry.get("icon", "")), _px(ROW_ICON), locked))

	var name_label := Label.new()
	name_label.name = "Name"
	name_label.label_settings = _label_settings(_font(0), INK_DIM if locked else INK)
	name_label.text = "🔒 LOCKED" if locked else _name(id)
	name_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(name_label)
	return button


# --------------------------------------------------------------------------
# Wire lookups. Small lists, read on a click — a dictionary index would be a
# second copy of the snapshot to keep in step with it.
# --------------------------------------------------------------------------

func _sauce(id: String) -> Dictionary:
	return _find(_book.get("sauces", []), id)


func _machine(id: String) -> Dictionary:
	return _find(_book.get("machines", []), id)


func _ingredient(id: String) -> Dictionary:
	return _find(_book.get("ingredients", []), id)


func _find(entries: Array, id: String) -> Dictionary:
	for entry in entries:
		if entry is Dictionary and String(entry.get("id", "")) == id:
			return entry
	return {}


func _name(id: String) -> String:
	if _display_name.is_valid():
		return String(_display_name.call(id))
	return id


## Ticks into the words a player reads. The ONE number this file computes,
## and only because `TICK_HZ` is GDScript's (godot_project/CLAUDE.md:
## "seconds, never ticks").
func _seconds(ticks: int) -> String:
	return "%.1fs" % (float(ticks) / _tick_hz)


## `4`, or `1.5` when a recipe ever asks for a fraction of something. The
## wire carries a real number because cost per case is one; a count that is
## whole should not print a decimal point it does not have.
func _quantity(value: float) -> String:
	if is_equal_approx(value, round(value)):
		return "%d" % int(round(value))
	return "%.2f" % value


# --------------------------------------------------------------------------
# Chrome. `main.gd`'s `_ui_px`/`_ui_i`/`_ui_font` against the host's scale,
# and `settings_panel.gd`'s plate — see that file's note on why the styling
# is restated rather than reached for.
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


## A titled block on the column: caption, an optional note BESIDE it, and
## the box everything in the section goes into.
##
## Beside rather than under, because a page is four or five of these and a
## line of its own for each note pushed the process chain — the thing a
## player opened the page to see — under the fold at 1600x900.
func _section(title: String, note: String) -> VBoxContainer:
	var column := VBoxContainer.new()
	column.name = "Section_%s" % title.replace(" ", "_")
	column.add_theme_constant_override("separation", _i(5))
	body.add_child(column)

	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", _i(10))
	column.add_child(header)

	var caption := Label.new()
	caption.name = "Caption"
	caption.label_settings = _label_settings(_font(-2), ACCENT)
	caption.text = title
	caption.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
	header.add_child(caption)

	if note != "":
		var sub := Label.new()
		sub.name = "Note"
		sub.label_settings = _label_settings(_font(-4), INK_DIM)
		sub.text = note
		sub.vertical_alignment = VERTICAL_ALIGNMENT_BOTTOM
		sub.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		sub.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		# No wrap and no pin: it sits in a row the fit pass measures, and
		# godot_project/CLAUDE.md's rule is pin the width or do not wrap.
		header.add_child(sub)
	return column


## `Label: value`, as two columns so a stack of them lines up.
func _line(parent: Node, caption: String, value: String) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", _i(10))
	parent.add_child(row)

	var left := Label.new()
	left.label_settings = _label_settings(_font(-1), INK_DIM)
	left.text = caption
	left.custom_minimum_size = Vector2(_px(150.0), 0.0)
	row.add_child(left)

	var right := Label.new()
	right.label_settings = _label_settings(_font(-1), INK)
	right.text = value
	right.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	right.custom_minimum_size = Vector2(_px(BODY_WIDTH - 160.0), 0.0)
	row.add_child(right)


## A paragraph of prose. Every one of these is PINNED to `BODY_WIDTH` before
## it wraps — godot_project/CLAUDE.md is explicit that a Label whose width a
## fit pass measures is pinned or it does not wrap, and `fit()` measures this
## column.
func _paragraph(parent: Node, node_name: String, text: String, colour: Color) -> Label:
	var label := Label.new()
	label.name = node_name
	label.label_settings = _label_settings(_font(-2), colour)
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(_px(BODY_WIDTH), 0.0)
	parent.add_child(label)
	return label


## The heading of any page: the thing's own icon, its name, its tagline.
func _title_row(id: String, asset: String, title: String, tagline: String, locked: bool = false) -> void:
	var row := HBoxContainer.new()
	row.name = "CookbookTitle"
	row.add_theme_constant_override("separation", _i(14))
	body.add_child(row)

	row.add_child(_icon(id, asset, _px(TITLE_ICON), locked))

	var column := VBoxContainer.new()
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.add_theme_constant_override("separation", _i(3))
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(column)

	var name_label := Label.new()
	name_label.name = "Name"
	name_label.label_settings = _label_settings(_font(3), INK_DIM if locked else HEADING)
	name_label.text = title
	column.add_child(name_label)

	if tagline != "":
		var sub := Label.new()
		sub.name = "Tagline"
		sub.label_settings = _label_settings(_font(-2), INK_DIM)
		sub.text = tagline
		sub.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		sub.custom_minimum_size = Vector2(_px(BODY_WIDTH - TITLE_ICON - 20.0), 0.0)
		column.add_child(sub)


## An icon for a content id. `asset` is what the snapshot's own `icon` field
## said, "" when the file named none; the host's callable owns the whole
## resolution chain and the fallback, here as everywhere else.
func _icon(id: String, asset: String, size: float, locked: bool) -> TextureRect:
	var rect := TextureRect.new()
	rect.name = "Icon"
	rect.custom_minimum_size = Vector2(size, size)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if _icon_for.is_valid():
		rect.texture = _icon_for.call(id, asset)
	if locked:
		rect.modulate = SILHOUETTE
	return rect


func _plate_button(node_name: String, locked: bool) -> Button:
	var button := Button.new()
	button.name = node_name
	button.focus_mode = Control.FOCUS_NONE
	button.clip_contents = true
	for state in ["normal", "hover", "pressed", "focus", "disabled"]:
		button.add_theme_stylebox_override(state, _plate_style(state, locked))
	return button


func _button(node_name: String, text: String) -> Button:
	var button := Button.new()
	button.name = node_name
	button.text = text
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_size_override("font_size", _font(0))
	button.add_theme_color_override("font_color", INK)
	button.add_theme_color_override("font_hover_color", HEADING)
	button.add_theme_color_override("font_pressed_color", HEADING)
	for state in ["normal", "hover", "pressed", "focus", "disabled"]:
		button.add_theme_stylebox_override(state, _plate_style(state, false))
	return button


func _plate_style(state: String, locked: bool) -> StyleBoxFlat:
	var style := StyleBoxFlat.new()
	style.bg_color = LOCKED_BG if locked else PLATE_BG
	style.border_color = LOCKED_EDGE if locked else PLATE_EDGE
	match state:
		"hover":
			style.bg_color = Color(0.19, 0.18, 0.14, 0.98)
			style.border_color = ACCENT
		"pressed":
			style.bg_color = Color(0.24, 0.21, 0.12, 0.98)
			style.border_color = ACCENT
	style.set_border_width_all(_i(2))
	style.set_content_margin_all(_px(6.0))
	return style


## The tree's lines. Everything it draws comes off `layout` — see the file
## header: a headless run has no layout pass, so the rectangles this renders
## from are the ones a test can read, not the ones the engine measured.
class TreeCanvas extends Control:
	var wire := Color(1, 1, 1, 1)
	var line_width := 3.0
	var arrow := 9.0

	func _draw() -> void:
		var layout: Dictionary = get_meta("layout", {})
		for edge in layout.get("edges", []):
			var points: PackedVector2Array = edge["points"]
			if points.size() < 2:
				continue
			draw_polyline(points, wire, line_width, true)
			var tip: Vector2 = points[points.size() - 1]
			draw_colored_polygon(
				PackedVector2Array([
					tip,
					tip + Vector2(-arrow, -arrow * 0.6),
					tip + Vector2(-arrow, arrow * 0.6),
				]),
				wire
			)
