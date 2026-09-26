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
#   * clearing the box shows everything again, and so does closing the menu.
extends SceneTree


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

	print("SHOP_SEARCH_OK: %d category sections headed from the snapshot, each holding its own entries; '%s' filtered %d slots to %d and hid %d section(s); a miss hid all and said so; clearing and reopening showed all %d" % [
		sections.size(), query, entries.size(), shown.size(), hidden_sections, entries.size(),
	])
	quit(0)
