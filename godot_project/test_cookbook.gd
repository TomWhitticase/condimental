# Headless proof of THE COOKBOOK's Godot half: the panel B opens, the sauce
# tree it draws out of `cookbook_json()`, the pages behind it, and the modal
# rules it shares with E/M/C/H/P.
#
# Run with:
#   godot --headless --path godot_project --script test_cookbook.gd
#
# Verifies:
#   * the tree IS the wire — one node per sauce, in the wire's tier column
#     with the wire's lock state, one labelled edge per "this sauce goes into
#     that one", every edge orthogonal and left to right, and all of it on
#     the canvas's `layout` meta, which is what a headless run can read and
#     therefore what `_draw()` is only allowed to render from,
#   * a locked family is a silhouette: the padlock, the word LOCKED and its
#     gate, its NAME nowhere on the screen, and a page that is the gate only
#     — no ingredient and no stage shown under the lock,
#   * an unlocked page is its wire entry: one row per input carrying the
#     wire's count, one plate per stage in wire order, every duration in
#     seconds and the word `tick` nowhere on any page,
#   * the pages link up — a stage opens its machine, an ingredient row opens
#     its ingredient, a row made here opens the sauce that makes it, and the
#     breadcrumb walks back to the tree,
#   * NOTHING is authored in GDScript: no id, display name or contract name
#     in the whole book appears in any string literal in `cookbook_panel.gd`,
#     which is the feature's binding requirement stated as an assertion,
#   * B opens it and closes the other five; each of E/M/C/H/P closes it; ESC
#     walks the pages back and only then closes it, without opening the pause
#     menu; both legends name B,
#   * the book is polled on the tick only while open, an unchanged book
#     redraws nothing, and the pause menu hosts the same panel as a page.
extends SceneTree

const PANEL_SOURCE := "res://cookbook_panel.gd"

var main
var _ran := false
var _failed := false


func _fail(msg: String) -> void:
	_failed = true
	printerr("COOKBOOK_FAIL: %s" % msg)


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


func _key(keycode: Key) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.pressed = true
	main._unhandled_input(event)


func _step() -> void:
	main._advance_simulation(1.0 / float(main.TICK_HZ))


func _wire() -> Dictionary:
	return JSON.parse_string(main.sim.cookbook_json())


func _panel():
	return main._cookbook


func _node(node_name: String) -> Node:
	if main._cookbook == null or main._cookbook.root == null:
		return null
	return main._cookbook.root.find_child(node_name, true, false)


## Every word the panel is showing, as one string. Labels and Buttons both,
## because a node's caption is a Button's text and a page's is a Label's.
func _texts(node: Node) -> Array:
	var out: Array = []
	if node == null:
		return out
	if node is Label:
		out.append((node as Label).text)
	elif node is Button:
		out.append((node as Button).text)
	for child in node.get_children():
		out.append_array(_texts(child))
	return out


func _screen() -> String:
	if main._cookbook == null:
		return ""
	return "\n".join(_texts(main._cookbook.root))


func _entry(key: String, id: String) -> Dictionary:
	for entry in _wire().get(key, []):
		if String(entry.get("id", "")) == id:
			return entry
	return {}


## The first sauce the wire says is locked, and the first it says is not.
func _sauce_where(locked: bool) -> Dictionary:
	for entry in _wire().get("sauces", []):
		if bool(entry.get("locked", false)) == locked:
			return entry
	return {}


func _run_checks() -> void:
	var book := _wire()
	if book.get("sauces", []).is_empty():
		_fail("cookbook_json carries no sauces; nothing below can be checked")
		_finish()
		return

	_check_tree(book)
	_check_sauce_page()
	_check_locked(book)
	_check_links()
	_check_nothing_authored(book)
	_check_keys()
	_check_polling()
	_check_pause_menu()
	_finish()


func _finish() -> void:
	if _failed:
		quit(1)
		return
	print("COOKBOOK_OK: the tree is cookbook_json — a node per sauce in its tier column, an orthogonal labelled edge per sauce-into-sauce, all on the canvas meta _draw() renders from; a locked family is a silhouette with its gate and a gate-only page; an unlocked page is its wire entry, counts, stages in order, seconds and never ticks; stages, rows and the breadcrumb link the pages; no id, name or contract in the book is written anywhere in cookbook_panel.gd; B opens it and closes E/M/C/H/P and each of them closes it, ESC walks back then closes without the pause menu, both legends name B; polled on the tick only while open, an unchanged book redraws nothing, and the pause menu hosts the same panel")
	quit(0)


# --------------------------------------------------------------------------
# The tree
# --------------------------------------------------------------------------

func _check_tree(book: Dictionary) -> void:
	main._set_cookbook_open(true)
	_panel().show_tree()

	if String(_panel().view().get("kind", "")) != "tree":
		_fail("opening the cookbook does not show the tree")

	var layout: Dictionary = _panel().tree_layout()
	var nodes: Array = layout.get("nodes", [])
	if nodes.size() != book.get("sauces", []).size():
		_fail("tree draws %d nodes for %d sauces" % [nodes.size(), book.get("sauces", []).size()])

	var drawn := {}
	for node in nodes:
		drawn[String(node.get("id", ""))] = node
	for entry in book.get("sauces", []):
		var id := String(entry.get("id", ""))
		if not drawn.has(id):
			_fail("the wire has sauce %s and the tree does not draw it" % id)
			continue
		var node: Dictionary = drawn[id]
		if int(node.get("tier", -1)) != int(entry.get("tier", -1)):
			_fail("%s is tier %d on the wire and column %d in the tree" % [id, int(entry.get("tier", -1)), int(node.get("tier", -1))])
		if bool(node.get("locked", false)) != bool(entry.get("locked", false)):
			_fail("%s lock state differs between wire and tree" % id)
		if _node("Node_%s" % id) == null:
			_fail("no clickable node for %s" % id)
		if _node("Tier_%d" % int(entry.get("tier", 0))) == null:
			_fail("no caption over tier %d" % int(entry.get("tier", 0)))

	# A tier column is one x, and a later tier is further right: the column
	# IS the claim "this far from raw", so two tiers sharing an x would be
	# the tree saying nothing.
	var column_of := {}
	for node in nodes:
		var tier := int(node.get("tier", 0))
		var x: float = (node.get("rect") as Rect2).position.x
		if column_of.has(tier) and not is_equal_approx(float(column_of[tier]), x):
			_fail("tier %d is drawn in two different columns" % tier)
		column_of[tier] = x
	var tiers: Array = column_of.keys()
	tiers.sort()
	for index in range(1, tiers.size()):
		if float(column_of[tiers[index]]) <= float(column_of[tiers[index - 1]]):
			_fail("tier %s is not drawn right of tier %s" % [tiers[index], tiers[index - 1]])

	var edges: Array = layout.get("edges", [])
	if edges.size() != book.get("edges", []).size():
		_fail("tree draws %d edges for %d on the wire" % [edges.size(), book.get("edges", []).size()])
	for edge in book.get("edges", []):
		var from_id := String(edge.get("from", ""))
		var to_id := String(edge.get("to", ""))
		var found := false
		for drawn_edge in edges:
			if String(drawn_edge.get("from", "")) != from_id or String(drawn_edge.get("to", "")) != to_id:
				continue
			found = true
			if String(drawn_edge.get("via", "")) != String(edge.get("via", "")):
				_fail("edge %s to %s carries the wrong item" % [from_id, to_id])
			var points: PackedVector2Array = drawn_edge.get("points", PackedVector2Array())
			if points.size() != 4:
				_fail("edge %s to %s is not a routed line" % [from_id, to_id])
				break
			if not (is_equal_approx(points[0].y, points[1].y)
					and is_equal_approx(points[1].x, points[2].x)
					and is_equal_approx(points[2].y, points[3].y)):
				_fail("edge %s to %s is not orthogonal" % [from_id, to_id])
			if points[0].x >= points[3].x:
				_fail("edge %s to %s does not run left to right" % [from_id, to_id])
			break
		if not found:
			_fail("the wire has edge %s to %s and the tree does not draw it" % [from_id, to_id])
		var label := _node("Edge_%s_%s" % [from_id, to_id])
		if label == null:
			_fail("edge %s to %s is unlabelled" % [from_id, to_id])
		elif (label as Label).text != main._kind_display_name(String(edge.get("via", ""))):
			_fail("edge %s to %s is not labelled with what travels along it" % [from_id, to_id])


# --------------------------------------------------------------------------
# A page
# --------------------------------------------------------------------------

func _check_sauce_page() -> void:
	var entry := _sauce_where(false)
	if entry.is_empty():
		_fail("no unlocked sauce to open")
		return
	var id := String(entry.get("id", ""))

	var node := _node("Node_%s" % id) as Button
	if node == null:
		_fail("no node to click for %s" % id)
		return
	node.pressed.emit()

	var view: Dictionary = _panel().view()
	if String(view.get("kind", "")) != "sauce" or String(view.get("id", "")) != id:
		_fail("clicking %s does not open its page" % id)
		return
	if _node("CrumbHere") == null:
		_fail("a page shows no breadcrumb")

	var title := _node("CookbookTitle")
	var title_text := "\n".join(_texts(title))
	if title_text.find(main._kind_display_name(id)) < 0:
		_fail("the page for %s is not titled with its name" % id)
	var tagline := String(entry.get("tagline", ""))
	if tagline != "" and title_text.find(tagline) < 0:
		_fail("the page for %s drops the tagline the data file gives it" % id)

	for line in entry.get("inputs", []):
		var input_id := String(line.get("id", ""))
		var row := _node("Input_%s" % input_id)
		if row == null:
			_fail("%s needs %s and the page does not list it" % [id, input_id])
			continue
		var row_text := "\n".join(_texts(row))
		if row_text.find(main._kind_display_name(input_id)) < 0:
			_fail("the %s row does not name it" % input_id)
		var quantity := float(line.get("quantity", 0.0))
		var count := "%d" % int(round(quantity)) if is_equal_approx(quantity, round(quantity)) else "%.2f" % quantity
		if row_text.find("×%s" % count) < 0:
			_fail("the %s row does not carry the wire's count of %s" % [input_id, count])

	var steps: Array = entry.get("steps", [])
	var process := _node("Process")
	if process == null:
		_fail("the page for %s shows no process" % id)
		return
	var order: Array = []
	for child in process.get_children():
		if String(child.name).begins_with("Step_"):
			order.append(String(child.name).substr(5))
	if order.size() != steps.size():
		_fail("%s takes %d stages and the page draws %d" % [id, steps.size(), order.size()])
	for index in range(mini(order.size(), steps.size())):
		var step: Dictionary = steps[index]
		var recipe := String(step.get("recipe", ""))
		if order[index] != recipe:
			_fail("stage %d of %s is %s, drawn as %s" % [index, id, recipe, order[index]])
			continue
		var plate := _node("Step_%s" % recipe)
		var plate_text := "\n".join(_texts(plate))
		var machine := String(step.get("machine", ""))
		if plate_text.find(main._kind_display_name(machine)) < 0:
			_fail("stage %s does not name the machine that runs it" % recipe)
		var seconds := "%.1fs" % (float(int(step.get("time_ticks", 0))) / float(main.TICK_HZ))
		if plate_text.find(seconds) < 0:
			_fail("stage %s does not say %s" % [recipe, seconds])

	var screen := _screen()
	if screen.to_lower().find("tick") >= 0:
		_fail("a cookbook page says tick; player-facing clocks are seconds")
	if screen.find("%.0f" % float(entry.get("cost_per_unit", 0.0))) < 0:
		_fail("the page for %s does not carry the wire's cost per case" % id)
	var blurb := String(entry.get("blurb", ""))
	if blurb != "" and screen.find(blurb) < 0:
		_fail("the page for %s drops the blurb the data file gives it" % id)
	for name in entry.get("contracts", []):
		if screen.find(String(name)) < 0:
			_fail("%s is bought by %s and the page does not say so" % [id, String(name)])


func _check_locked(book: Dictionary) -> void:
	var entry := _sauce_where(true)
	if entry.is_empty():
		# Every family unlocked is a legitimate state of the content; there
		# is then nothing to prove about a silhouette.
		return
	var id := String(entry.get("id", ""))
	var display: String = main._kind_display_name(id)

	_panel().show_tree()
	var tree_text := _screen()
	if tree_text.find(display) >= 0:
		_fail("the tree names the locked sauce %s; it is meant to be a silhouette" % id)
	var node := _node("Node_%s" % id)
	var node_text := "\n".join(_texts(node))
	if node_text.find("LOCKED") < 0:
		_fail("the silhouette for %s does not say LOCKED" % id)
	var gate = entry.get("gate")
	if gate is Dictionary:
		var progress := "%d of %d" % [int(gate.get("deliveries", 0)), int(gate.get("after_deliveries", 0))]
		if node_text.find(progress) < 0:
			_fail("the silhouette for %s does not carry its gate (%s)" % [id, progress])

	(node as Button).pressed.emit()
	var page := _screen()
	if page.find(display) >= 0:
		_fail("the page for locked %s names it" % id)
	if gate is Dictionary and page.find(String(gate.get("name", ""))) < 0:
		_fail("the page for locked %s does not name the gate standing over it" % id)
	for line in entry.get("inputs", []):
		var input_id := String(line.get("id", ""))
		if _node("Input_%s" % input_id) != null:
			_fail("the locked page for %s lists its ingredients" % id)
		if page.find(main._kind_display_name(input_id)) >= 0:
			_fail("the locked page for %s names %s" % [id, input_id])
	for step in entry.get("steps", []):
		if _node("Step_%s" % String(step.get("recipe", ""))) != null:
			_fail("the locked page for %s shows its process" % id)

	if not _panel().back():
		_fail("a page cannot be walked back to the tree")
	if String(_panel().view().get("kind", "")) != "tree":
		_fail("back from a page does not land on the tree")
	if _panel().back():
		_fail("the tree claims somewhere further back to go")


func _check_links() -> void:
	var entry := _sauce_where(false)
	if entry.is_empty():
		return
	var id := String(entry.get("id", ""))
	_panel().show_tree()
	(_node("Node_%s" % id) as Button).pressed.emit()

	var steps: Array = entry.get("steps", [])
	if not steps.is_empty():
		var step: Dictionary = steps[0]
		var machine := String(step.get("machine", ""))
		(_node("Step_%s" % String(step.get("recipe", ""))) as Button).pressed.emit()
		var view: Dictionary = _panel().view()
		if String(view.get("kind", "")) != "machine" or String(view.get("id", "")) != machine:
			_fail("a stage does not open the machine that runs it")
		var machine_entry := _entry("machines", machine)
		var page := _screen()
		if page.find(main._kind_display_name(machine)) < 0:
			_fail("the %s page does not name it" % machine)
		if page.find("%d × %d" % [int(machine_entry.get("width_tiles", 0)), int(machine_entry.get("height_tiles", 0))]) < 0:
			_fail("the %s page does not carry the footprint the wire gives it" % machine)
		for recipe in machine_entry.get("makes", []):
			if _node("Makes_%s" % String(recipe)) == null and page.find(main._kind_display_name(String(recipe))) < 0:
				_fail("the %s page does not say it makes %s" % [machine, String(recipe)])
		_panel().back()

	for line in entry.get("inputs", []):
		var input_id := String(line.get("id", ""))
		if bool(line.get("locked", false)):
			continue
		(_node("Input_%s" % input_id) as Button).pressed.emit()
		var view: Dictionary = _panel().view()
		var from_sauce = line.get("from_sauce")
		if from_sauce == null:
			if String(view.get("kind", "")) != "ingredient" or String(view.get("id", "")) != input_id:
				_fail("the %s row does not open its ingredient" % input_id)
			elif _screen().find("%.0f" % float(_entry("ingredients", input_id).get("price", 0.0))) < 0:
				_fail("the %s page does not carry its price" % input_id)
		elif String(view.get("kind", "")) != "sauce" or String(view.get("id", "")) != String(from_sauce):
			_fail("the %s row is made here and does not open the sauce that makes it" % input_id)
		_panel().back()
		if String(_panel().view().get("id", "")) != id:
			_fail("back from a linked page does not return to where it was opened from")


# --------------------------------------------------------------------------
# The requirement, as an assertion
# --------------------------------------------------------------------------

## Every string the panel can put on screen, taken out of its own source.
## Comment lines are skipped: a comment naming an example is not content the
## player is ever shown.
func _panel_literals() -> Array:
	var out: Array = []
	for raw in FileAccess.get_file_as_string(PANEL_SOURCE).split("\n"):
		var line := raw.strip_edges()
		if line.begins_with("#"):
			continue
		var index := 0
		while true:
			var start := line.find("\"", index)
			if start < 0:
				break
			var finish := line.find("\"", start + 1)
			if finish < 0:
				break
			out.append(line.substr(start + 1, finish - start - 1))
			index = finish + 1
	return out


## The whole point of the feature: a new sauce, a renamed item or a
## rebalanced recipe must change the screen with no edit to the panel. That
## holds exactly as long as the panel never writes a piece of content down,
## so this checks the panel's own strings against every id, display name and
## contract name the book carries.
func _check_nothing_authored(book: Dictionary) -> void:
	var words := {}
	for key in ["sauces", "machines", "ingredients"]:
		for entry in book.get(key, []):
			var id := String(entry.get("id", ""))
			words[id] = true
			words[main._kind_display_name(id)] = true
	for entry in book.get("sauces", []):
		for step in entry.get("steps", []):
			words[String(step.get("recipe", ""))] = true
		for name in entry.get("contracts", []):
			words[String(name)] = true

	var literals := _panel_literals()
	if literals.is_empty():
		_fail("could not read %s to check it authors no content" % PANEL_SOURCE)
		return
	for word in words:
		var text := String(word)
		if text.length() < 3:
			continue
		for literal in literals:
			if String(literal).find(text) >= 0:
				_fail("cookbook_panel.gd writes \"%s\" into the string \"%s\"; content belongs in data/*.ron" % [text, String(literal)])


# --------------------------------------------------------------------------
# Keys and the modal rules
# --------------------------------------------------------------------------

func _check_keys() -> void:
	if main.ISO_HELP_TEXT.find("B: cookbook") < 0:
		_fail("the isometric legend does not name B")
	if main.FP_HELP_TEXT.find("B: cookbook") < 0:
		_fail("the first-person legend does not name B")

	main._close_all_overlays()
	_key(KEY_B)
	if not main._cookbook_open:
		_fail("B does not open the cookbook")
	if not main._cookbook_plate.visible:
		_fail("an open cookbook is not visible")
	_key(KEY_B)
	if main._cookbook_open:
		_fail("B does not close the cookbook")

	# One modal at a time, both ways round, for each of the five.
	var others := {KEY_E: "_shop_open", KEY_M: "_market_open", KEY_C: "_board_open", KEY_H: "_staff_open", KEY_P: "_production_open"}
	for keycode in others:
		var flag: String = others[keycode]
		main._close_all_overlays()
		_key(keycode)
		if not bool(main.get(flag)):
			_fail("%s did not open before the cookbook test" % flag)
		_key(KEY_B)
		if not main._cookbook_open:
			_fail("B does not open the cookbook over %s" % flag)
		if bool(main.get(flag)):
			_fail("opening the cookbook leaves %s open" % flag)
		_key(keycode)
		if main._cookbook_open:
			_fail("%s does not close the cookbook" % flag)

	main._close_all_overlays()
	_key(KEY_B)
	var entry := _sauce_where(false)
	if not entry.is_empty():
		_panel().open("sauce", String(entry.get("id", "")))
		_key(KEY_ESCAPE)
		if not main._cookbook_open:
			_fail("ESC on a page closes the whole cookbook instead of walking back")
		if String(_panel().view().get("kind", "")) != "tree":
			_fail("ESC on a page does not walk back to the tree")
	_key(KEY_ESCAPE)
	if main._cookbook_open:
		_fail("ESC at the tree does not close the cookbook")
	if main._pause_open:
		_fail("ESC closing the cookbook also opened the pause menu")

	# Entering a mode closes every modal — the rule X and the build menu
	# already share (main.gd::_toggle_remove_mode).
	_key(KEY_B)
	main._toggle_remove_mode()
	if main._cookbook_open:
		_fail("entering remove mode leaves the cookbook open")
	main._close_all_overlays()


# --------------------------------------------------------------------------
# Polling and the pause menu
# --------------------------------------------------------------------------

func _check_polling() -> void:
	main._close_all_overlays()
	var closed: int = main._cookbook_polled_tick
	for _index in range(main.LOAD_POLL_TICKS * 3):
		_step()
	if main._cookbook_polled_tick != closed:
		_fail("the cookbook is polled while it is closed")

	_key(KEY_B)
	if main._cookbook_book.is_empty():
		_fail("opening the cookbook does not read the book at once")
	for _index in range(main.LOAD_POLL_TICKS * 2):
		_step()
		if main._cookbook_open and main._cookbook_poll_due():
			main._poll_cookbook()
	if main._cookbook_polled_tick == closed:
		_fail("an open cookbook is never polled")

	# An unchanged book must not throw the page away mid-read.
	var sauce := _sauce_where(false)
	if not sauce.is_empty():
		_panel().open("sauce", String(sauce.get("id", "")))
		var row := _node("Input_%s" % String((sauce.get("inputs", [])[0] as Dictionary).get("id", "")))
		_panel().set_book(_wire())
		if _node("Input_%s" % String((sauce.get("inputs", [])[0] as Dictionary).get("id", ""))) != row:
			_fail("an unchanged book rebuilds the page under the player")
	main._close_all_overlays()


func _check_pause_menu() -> void:
	main._set_pause_open(true)
	if main._pause_cookbook == null:
		_fail("the pause menu hosts no cookbook")
		main._set_pause_open(false)
		return
	main._on_pause_cookbook_pressed()
	if not main._pause_cookbook_open:
		_fail("the pause menu's COOKBOOK button opens nothing")
	if main._pause_buttons_box.visible:
		_fail("the cookbook page does not replace the pause menu's buttons")
	if not main._pause_cookbook_box.visible:
		_fail("the pause menu's cookbook page is not shown")
	if String(main._pause_cookbook.view().get("kind", "")) != "tree":
		_fail("the pause menu's cookbook does not open on the tree")
	if main._pause_cookbook.root.find_child("Node_%s" % String(_wire().get("sauces", [])[0].get("id", "")), true, false) == null:
		_fail("the pause menu's cookbook draws no tree")
	main._on_pause_cookbook_back_pressed()
	if main._pause_cookbook_open:
		_fail("BACK does not leave the pause menu's cookbook")
	if not main._pause_buttons_box.visible:
		_fail("leaving the cookbook page does not restore the pause menu's buttons")
	main._set_pause_open(false)
	if main._pause_cookbook_open:
		_fail("closing the pause menu leaves its cookbook page open")
