# Headless proof of the build menu's category sections and its search box.
#
# Run with:
#   godot --headless --path godot_project --script test_shop_search.gd
#
# Categories are CONTENT — `data/shop.ron`'s `categories`, and a `category`
# on every entry, carried by `shop_snapshot_json`. So every expectation here
# is read off that snapshot, never written down: the headings the menu must
# print are the snapshot's category names, and the section a slot must sit
# in is the one its entry names. A category added, renamed or regrouped in
# the data file moves this test with it.
#
# What it checks:
#
#   * one headed section per snapshot category, in the snapshot's order,
#     its caption printing that category's name and its grid holding exactly
#     the slots whose entries name it,
#   * typing in the search box (the LineEdit's own `text_changed`, which is
#     what a keystroke emits) hides every slot whose display name does not
#     contain the text, case-insensitively, and hides a section left empty,
#   * text matching nothing hides every section and says so,
#   * clearing the box shows everything again, and so does closing the menu,
#   * with the box FOCUSED, real key events (`root.push_input`) type into it
#     and reach no hotkey — no digit selects a tool, no letter opens a panel
#     or closes the menu, TAB leaves the view alone — then the first ESC only
#     releases the focus and the second closes the menu and clears the text.
#     The hotkeys live in `_unhandled_input`, which a focused LineEdit gets
#     to first; this pins that, so moving one to `_input` or a shortcut goes
#     red here. A negative control (M with nothing focused opens the market)
#     proves the pushed events do reach the hotkeys.
extends SceneTree

## The keys typed into the focused box: each digit and letter a hotkey is
## bound to, plus SHIFT+1 (buy), `-` and SPACE. `[keycode, unicode, shift]`.
const TYPED_KEYS := [
	[KEY_1, 0x31, false], [KEY_E, 0x65, false], [KEY_M, 0x6d, false],
	[KEY_C, 0x63, false], [KEY_H, 0x68, false], [KEY_P, 0x70, false],
	[KEY_B, 0x62, false], [KEY_X, 0x78, false], [KEY_R, 0x72, false],
	[KEY_1, 0x21, true], [KEY_MINUS, 0x2d, false], [KEY_SPACE, 0x20, false],
]
const TYPED_TEXT := "1emchpbxr!- "

var _main
var _step := 0
var _summary := ""
var _view_before := 0


func _fail(msg: String) -> void:
	printerr("SHOP_SEARCH_FAIL: %s" % msg)
	quit(1)


## The slots showing, as kind ids.
func _shown(main) -> Array:
	var out := []
	for slot in main._shop_slots:
		if slot["button"].visible:
			out.append(String(slot["kind"]))
	return out


## Types `text` the way a player does: the box's text changes and it emits
## `text_changed` (which assigning `text` from code does not).
func _type(main, text: String) -> void:
	main._shop_search.text = text
	main._shop_search.text_changed.emit(text)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	main._set_shop_menu_open(true)

	var snapshot: Dictionary = JSON.parse_string(main.sim.shop_snapshot_json())
	var categories: Array = snapshot.get("categories", [])
	var entries: Array = snapshot.get("entries", [])
	if categories.size() < 2:
		_fail("the snapshot carries %d categories — a sectioned menu needs data/shop.ron to define them" % categories.size())
		return

	# --- One headed section per category, holding that category's slots ----
	var sections: Array = main._shop_section_list
	if sections.size() != categories.size():
		_fail("%d sections for %d snapshot categories" % [sections.size(), categories.size()])
		return
	for i in range(categories.size()):
		var category: Dictionary = categories[i]
		var id := String(category["id"])
		var section: Dictionary = sections[i]
		if String(section["id"]) != id:
			_fail("section %d is '%s', the snapshot's category %d is '%s' — headings follow the data's order" % [
				i, section["id"], i, id,
			])
			return
		var caption: Label = section["section"].find_child("Caption", true, false)
		if caption == null or caption.text.to_lower() != String(category["name"]).to_lower():
			_fail("section '%s' must be headed with its category name '%s', reads '%s'" % [
				id, category["name"], caption.text if caption != null else "<no caption>",
			])
			return
		var expected := []
		for entry in entries:
			if String(entry["category"]) == id:
				expected.append(String(entry["kind"]))
		var held := []
		for child in section["grid"].get_children():
			held.append(String(child.name).trim_prefix("ShopSlot_"))
		if held != expected:
			_fail("section '%s' holds %s, its entries are %s" % [id, held, expected])
			return
		if not section["section"].visible:
			_fail("with an empty search, section '%s' must be showing" % id)
			return

	if _shown(main).size() != entries.size():
		_fail("with an empty search every slot must show, %d of %d do" % [_shown(main).size(), entries.size()])
		return

	# --- Typing filters by display name, case-insensitively --------------
	# The query is a real display name's middle, in the wrong case, so the
	# check is about substring + case rather than about one known kind.
	var target := String(entries[entries.size() - 1]["kind"])
	var name: String = main._kind_display_name(target)
	var query := name.substr(1, maxi(3, name.length() - 2)).to_upper()
	_type(main, query)
	var shown := _shown(main)
	if not shown.has(target):
		_fail("searching '%s' hid '%s' (display name '%s')" % [query, target, name])
		return
	for entry in entries:
		var kind := String(entry["kind"])
		var matches: bool = main._kind_display_name(kind).to_lower().contains(query.to_lower())
		if shown.has(kind) != matches:
			_fail("searching '%s': '%s' (%s) is %s" % [
				query, kind, main._kind_display_name(kind), "shown" if shown.has(kind) else "hidden",
			])
			return
	if shown.size() >= entries.size():
		_fail("searching '%s' hid nothing — the filter is not applied" % query)
		return
	var hidden_sections := 0
	for section in sections:
		var has_match := false
		for child in section["grid"].get_children():
			has_match = has_match or shown.has(String(child.name).trim_prefix("ShopSlot_"))
		if section["section"].visible != has_match:
			_fail("searching '%s': section '%s' is %s with %s match" % [
				query, section["id"], "shown" if section["section"].visible else "hidden",
				"a" if has_match else "no",
			])
			return
		if not has_match:
			hidden_sections += 1
	if hidden_sections == 0:
		_fail("searching '%s' left every section up — a category with no match must hide" % query)
		return
	if main._shop_no_match.visible:
		_fail("the no-match line is up while '%s' matches something" % query)
		return

	# --- Nothing matches: every section hides, and the menu says so -------
	_type(main, "zzqqxx")
	if not _shown(main).is_empty():
		_fail("'zzqqxx' matched %s" % [_shown(main)])
		return
	for section in sections:
		if section["section"].visible:
			_fail("section '%s' is up with nothing in it" % section["id"])
			return
	if not main._shop_no_match.visible:
		_fail("a search that matches nothing must say so, not show an empty panel")
		return

	# --- Clearing brings everything back; so does closing ----------------
	_type(main, "")
	if _shown(main).size() != entries.size():
		_fail("clearing the search left %d of %d slots showing" % [_shown(main).size(), entries.size()])
		return
	_type(main, query)
	main._set_shop_menu_open(false)
	main._set_shop_menu_open(true)
	if main._shop_search.text != "" or _shown(main).size() != entries.size():
		_fail("reopening the menu must clear the search and show every slot")
		return

	_summary = "%d category sections headed from the snapshot, each holding its own entries; '%s' filtered %d slots to %d and hid %d section(s); a miss hid all and said so; clearing and reopening showed all %d" % [
		sections.size(), query, entries.size(), shown.size(), hidden_sections, entries.size(),
	]
	# The keyboard half needs frames: `push_input` is delivered through the
	# viewport's GUI, which a SceneTree only runs from `_process` on.
	_main = main


## A real key press and release, through the viewport the way a keyboard's
## are — so it goes through GUI focus, then `_unhandled_input`.
func _key(code: int, unicode := 0, shift := false) -> void:
	for pressed in [true, false]:
		var ev := InputEventKey.new()
		ev.keycode = code
		ev.physical_keycode = code
		ev.unicode = unicode if pressed else 0
		ev.pressed = pressed
		ev.shift_pressed = shift
		root.push_input(ev)


## What a leaked hotkey would have changed, or "" when nothing did.
func _leak(main) -> String:
	if not main._shop_menu.visible:
		return "the build menu closed"
	for panel_name in ["_market_panel", "_board_panel", "_staff_panel", "_production_panel"]:
		var panel: Control = main.get(panel_name)
		if panel != null and panel.visible:
			return "%s opened" % panel_name
	if main._cookbook_open:
		return "the cookbook opened"
	if main._build_kind != "":
		return "tool '%s' was selected" % main._build_kind
	if main._remove_mode:
		return "remove mode came on"
	if main._view_mode != _view_before:
		return "the view mode changed %d -> %d" % [_view_before, main._view_mode]
	return ""


func _process(_delta: float) -> bool:
	if _main == null:
		return false
	_step += 1
	var main = _main
	var search: LineEdit = main._shop_search
	match _step:
		3:
			# Closed and reopened, not just opened: the engine runs `_ready`
			# itself once frames start, after the explicit call above, and
			# that rebuilds the HUD — `_shop_menu` and `_shop_search` are new
			# nodes, hidden, under a flag that still says open.
			main._set_shop_menu_open(false)
			main._set_shop_menu_open(true)
			search = main._shop_search
			search.grab_focus()
			_view_before = main._view_mode
			for key in TYPED_KEYS:
				_key(key[0], key[1], key[2])
			_key(KEY_TAB)
		5:
			if search.text != TYPED_TEXT:
				_fail("typed '%s' into the focused box, it reads '%s'" % [TYPED_TEXT, search.text])
				return true
			var leak := _leak(main)
			if leak != "":
				_fail("keys typed into the focused search box leaked to the hotkeys: %s" % leak)
				return true
			if not search.has_focus():
				_fail("the search box lost focus while being typed into")
				return true
			_key(KEY_ESCAPE)
		7:
			if search.has_focus():
				_fail("the first ESC must release the search box's focus")
				return true
			if not main._shop_menu.visible or search.text != TYPED_TEXT:
				_fail("the first ESC must only release focus — menu %s, text '%s'" % [
					"open" if main._shop_menu.visible else "closed", search.text,
				])
				return true
			_key(KEY_ESCAPE)
		9:
			if main._shop_menu.visible:
				_fail("the second ESC, with nothing focused, must close the build menu")
				return true
			main._set_shop_menu_open(true)
			if search.text != "":
				_fail("closing with ESC must clear the search, it reopened with '%s'" % search.text)
				return true
			main._set_shop_menu_open(false)
			# Negative control: with nothing focused the same kind of event
			# DOES reach a hotkey, so a quiet run above is not a deaf probe.
			_key(KEY_M, 0x6d)
		11:
			if main._market_panel == null or not main._market_panel.visible:
				_fail("negative control: M with nothing focused did not open the market — the pushed keys reach no hotkey, so the checks above prove nothing")
				return true
			print("SHOP_SEARCH_OK: %s; typed '%s' into the focused box with no hotkey firing, the first ESC released it and the second closed and cleared the menu; M unfocused opened the market" % [
				_summary, TYPED_TEXT,
			])
			quit(0)
			return true
	return false
