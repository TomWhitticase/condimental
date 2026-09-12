# Headless proof of the machine-load indicator's Godot half (the machine-load
# epic, ticket 3; `docs/MACHINE_LOAD_PLAN.md`): every recipe-running machine
# carries an andon stack light on its camera-visible corner tile, lit to the
# simulation's own verdict, polled on the tick, and its hover text says the
# same thing in words.
#
# Run with:
#   godot --headless --path godot_project --script test_machine_load.gd
#
# Verifies:
#   * `ANDON_CORNER` is derived from `CAMERA_DIRECTION`, and `_andon_tile`
#     picks the max-X/max-Y tile of a footprint under it (the 2x2 vat's far
#     corner, the 1x2 mixer's north tile, a 1x1 machine's own tile),
#   * every machine of the settled bottleneck line gets exactly three andon
#     nodes (one per status model), at that tile, yaw 0, short for a 1x1
#     footprint and tall for anything larger; nothing that is not a
#     recipe-running machine gets one — not the hoppers, the tank, the dock,
#     a belt or a pipe,
#   * exactly one lens is visible per machine and it is the status the sim
#     reports on the wire — red · red · green · amber · amber along the
#     line, which is the whole point of the fixture,
#   * the hover line carries status, percentage and reason for all three
#     cases, in the plan's exact language, with the window in seconds from
#     `window / TICK_HZ` and items through `_item_display_name`; the hover
#     panel's load bar records the same fraction,
#   * a status change flips the lens (a real one, polled through
#     `_advance_simulation` as a fresh line starts up), and the pause menu
#     freezes polling — the sim may move on, the lights do not,
#   * a save loaded back shows its lights on the first frame, before any tick.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false
var _failed := false


## Prints the failure and keeps going, so one run lists every problem; the
## success marker is withheld at the end (`quit(1)` alone would be
## overridden by the final `quit(0)`).
func _fail(msg: String) -> void:
	_failed = true
	printerr("MACHINE_LOAD_FAIL: %s" % msg)


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


## One simulation step through the real per-frame path.
func _step() -> void:
	main._advance_simulation(1.0 / float(main.TICK_HZ))


## What the simulation itself says the machine at `key` is doing, straight
## off the wire — the reference every lens is checked against.
func _sim_status(key: String) -> String:
	var parsed = JSON.parse_string(main.sim.machine_load_snapshots_json())
	for entry in parsed:
		if main._tile_key(main._pair_to_tile(entry.get("tile"))) == key:
			return String(entry.get("load", {}).get("status", ""))
	return ""


## "x,y" back to a tile.
func _key_tile(key: String) -> Vector2i:
	var parts := key.split(",")
	return Vector2i(int(parts[0]), int(parts[1]))


## Which lens the machine at `key` is showing, or "" for none, failing on
## more than one.
func _lit_lens(key: String) -> String:
	if not main._machine_andons.has(key):
		return ""
	var lenses: Dictionary = main._machine_andons[key]["lenses"]
	var lit := ""
	for status in lenses.keys():
		var node: Node3D = lenses[status]
		if node.visible:
			if lit != "":
				_fail("the machine at %s shows two lenses at once: %s and %s" % [key, lit, status])
			lit = String(status)
	return lit


func _run_checks() -> void:
	# --- 1. the corner, pure -----------------------------------------------
	var expected_corner := Vector2i(
		int(signf(main.CAMERA_DIRECTION.x)), int(signf(main.CAMERA_DIRECTION.z))
	)
	if main.ANDON_CORNER != expected_corner:
		_fail("ANDON_CORNER must be the sign of CAMERA_DIRECTION on the tile axes, got %s" % str(main.ANDON_CORNER))
	if main.ANDON_CORNER != Vector2i(1, 1):
		_fail("with the shipped camera the visible corner is the far one on both axes, got %s" % str(main.ANDON_CORNER))
	if main._andon_tile(Vector2i(4, 0), Vector2i(1, 1)) != Vector2i(4, 0):
		_fail("a 1x1 machine's andon stands on its own tile")
	if main._andon_tile(Vector2i(12, 0), Vector2i(2, 2)) != Vector2i(13, 1):
		_fail("the 2x2 vat's andon stands on its far corner (13,1), got %s" % str(main._andon_tile(Vector2i(12, 0), Vector2i(2, 2))))
	if main._andon_tile(Vector2i(9, 0), Vector2i(1, 2)) != Vector2i(9, 1):
		_fail("the 1x2 mixer's andon stands on its north tile (9,1)")

	# --- 2. the words, pure -------------------------------------------------
	var window: int = 600
	var probe := "77,77"
	main._machine_load[probe] = {
		"status": "working", "reason": null,
		"load": {"percent": 100, "window": window, "status": "working"},
	}
	var expected_seconds := int(round(float(window) / main.TICK_HZ))
	var flat_out: String = main._machine_load_text(probe)
	if flat_out != "flat out  ·  100%% of the last %ds" % [expected_seconds]:
		_fail("a working machine reads 'flat out · 100%% of the last Ns', got '%s'" % [flat_out])
	if flat_out.find("%ds" % expected_seconds) < 0:
		_fail("the window must be window / TICK_HZ seconds (%d), got '%s'" % [expected_seconds, flat_out])
	main._machine_load[probe] = {
		"status": "starved", "reason": {"kind": "input", "item": "sugar"},
		"load": {"percent": 38, "window": window, "status": "starved"},
	}
	var starved: String = main._machine_load_text(probe)
	if starved != "starved  ·  38%% busy  ·  waiting for %s" % [main._item_display_name("sugar")]:
		_fail("a starved machine reads 'starved · 38%% busy · waiting for Sugar', got '%s'" % [starved])
	if starved.find("sugar") >= 0 and main._item_display_name("sugar") != "sugar":
		_fail("the item must go through _item_display_name, got '%s'" % starved)
	main._machine_load[probe] = {
		"status": "blocked", "reason": {"kind": "output", "item": "tomato_paste"},
		"load": {"percent": 61, "window": window, "status": "blocked"},
	}
	var blocked: String = main._machine_load_text(probe)
	if blocked != "blocked  ·  61% busy  ·  output full":
		_fail("a blocked machine reads 'blocked · 61%% busy · output full', got '%s'" % [blocked])
	# No remembered reason: the status and the number alone, nothing invented.
	main._machine_load[probe] = {
		"status": "blocked", "reason": null,
		"load": {"percent": 61, "window": window, "status": "blocked"},
	}
	if main._machine_load_text(probe) != "blocked  ·  61% busy":
		_fail("a blocked machine with no reason yet says only status and number, got '%s'" % [main._machine_load_text(probe)])
	main._machine_load.erase(probe)
	if main._machine_load_text(probe) != "":
		_fail("an unpolled machine has no load line")
	# Every reason the wire can spell.
	var reasons := {
		"fluid_input": "waiting for %s in the pipe" % main._item_display_name("ketchup"),
		"no_supply_pipe": "no supply pipe for %s" % main._item_display_name("ketchup"),
		"fluid_output": "outlet pipe full",
		"no_outlet_pipe": "no outlet pipe",
	}
	for kind in reasons.keys():
		var text: String = main._stall_reason_text({"kind": kind, "item": "ketchup"})
		if text != reasons[kind]:
			_fail("stall kind %s reads '%s', expected '%s'" % [kind, text, reasons[kind]])
	if main._stall_reason_text(null) != "":
		_fail("a null stall is no reason")
	if main._stall_reason_text({"kind": "something_new", "item": "x"}) != "":
		_fail("an unknown stall kind says nothing rather than guessing")

	# --- 3. the lights on the settled line -----------------------------------
	Layouts.build_bottleneck_line(main.sim)
	main._rebuild_world()

	var expected: Dictionary = Layouts.BOTTLENECK_MACHINES
	var heights := {
		"4,0": main.ANDON_HEIGHT_SHORT, "9,0": main.ANDON_HEIGHT_TALL,
		"12,0": main.ANDON_HEIGHT_TALL, "16,0": main.ANDON_HEIGHT_SHORT,
		"21,0": main.ANDON_HEIGHT_SHORT,
	}
	var corners := {
		"4,0": Vector2i(4, 0), "9,0": Vector2i(9, 1), "12,0": Vector2i(13, 1),
		"16,0": Vector2i(16, 0), "21,0": Vector2i(21, 0),
	}
	if main._machine_andons.size() != expected.size():
		_fail("%d machines on the line, %d andons built" % [expected.size(), main._machine_andons.size()])
	for key in expected.keys():
		if not main._machine_andons.has(key):
			_fail("the machine at %s has no andon" % key)
		var andon: Dictionary = main._machine_andons[key]
		var lenses: Dictionary = andon["lenses"]
		if lenses.size() != main.ANDON_STATUSES.size():
			_fail("the machine at %s has %d lens nodes, expected %d" % [key, lenses.size(), main.ANDON_STATUSES.size()])
		if String(andon["height"]) != heights[key]:
			_fail("the machine at %s stands a %s andon, expected %s" % [key, andon["height"], heights[key]])
		if andon["tile"] != corners[key]:
			_fail("the machine at %s stands its andon on %s, expected %s" % [key, str(andon["tile"]), str(corners[key])])
		var want_position: Vector3 = main._footprint_centre(corners[key], Vector2i.ONE, main.FLOOR_Y)
		for status in main.ANDON_STATUSES:
			if not lenses.has(status):
				_fail("the machine at %s has no %s lens node" % [key, status])
			var node: Node3D = lenses[status]
			if node == null or not is_instance_valid(node) or node.get_parent() != main._world_root:
				_fail("the %s lens of %s is not a node under World" % [status, key])
			if node.name != "andon_%s_%s" % [key, status]:
				_fail("the %s lens of %s is named %s" % [status, key, node.name])
			if node.position.distance_to(want_position) > 0.001:
				_fail("the %s lens of %s stands at %s, expected the corner tile's centre %s" % [
					status, key, str(node.position), str(want_position),
				])
			if absf(node.rotation.y) > 0.001:
				_fail("an andon is never rotated; %s's is at yaw %f" % [key, node.rotation.y])
			# The model is really there: an asset name that fell through to
			# nothing would be an empty Node3D with every test green.
			var meshes := 0
			for child in node.get_children():
				if child is MeshInstance3D and child.mesh != null:
					meshes += 1
			if meshes == 0:
				_fail("the %s lens of %s (%s) has no mesh — is andon_%s_%s staged?" % [
					status, key, node.name, status, andon["height"],
				])
		# One lens lit, and it is the sim's word.
		var lit := _lit_lens(key)
		var sim_status := _sim_status(key)
		if lit == "":
			_fail("the machine at %s shows no lens" % key)
		if lit != sim_status:
			_fail("the machine at %s shows %s, the sim says %s" % [key, lit, sim_status])
		if sim_status != expected[key]:
			_fail("the settled line's %s reads %s, the fixture expects %s" % [key, sim_status, expected[key]])
		if String(main._machine_load[key]["status"]) != sim_status:
			_fail("_machine_load[%s] says %s, the sim says %s" % [key, main._machine_load[key]["status"], sim_status])
	# The body's own lamp is still the body's: the lenses are siblings.
	for key in expected.keys():
		var lamp = main._machine_lamps.get(key)
		if lamp == null or not String(lamp.get_parent().name).ends_with("_%s" % key):
			_fail("the machine at %s lost its own status lamp to the andon" % key)

	# Nothing else gets one: hoppers, the tank, the dock, a belt, a pipe.
	for key in ["-1,0", "4,1", "5,2", "26,0", "0,0", "10,0"]:
		if main._machine_andons.has(key):
			_fail("%s is not a recipe-running machine and must have no andon" % key)
		if main._machine_load.has(key):
			_fail("%s is not a recipe-running machine and must have no load record" % key)
	for child in main._world_root.get_children():
		if String(child.name).begins_with("andon_"):
			var owner_key := String(child.name).trim_prefix("andon_").rsplit("_", true, 1)[0]
			if not expected.has(owner_key):
				_fail("an andon node %s stands on nothing the fixture expects one on" % child.name)

	# --- 4. hover on the settled line ----------------------------------------
	var hover_cases := {
		"4,0": ["blocked", "output full"],
		"9,0": ["blocked", "outlet pipe full"],
		"12,0": ["flat out", "of the last %ds" % expected_seconds],
		"16,0": ["starved", "waiting for %s in the pipe" % main._item_display_name("ketchup")],
		"21,0": ["starved", "waiting for %s" % main._item_display_name("bottled_ketchup")],
	}
	for key in hover_cases.keys():
		var origin: Vector2i = _key_tile(key)
		var text: String = main._hover_text_at_tile(origin)
		for needle in hover_cases[key]:
			if text.find(needle) < 0:
				_fail("hover on %s must say '%s', got:\n%s" % [key, needle, text])
		var percent := int(main._machine_load[key]["load"]["percent"])
		if text.find("%d%%" % percent) < 0:
			_fail("hover on %s must carry the sim's %d%%, got:\n%s" % [key, percent, text])
		var state: Dictionary = main._hover_panel_state(origin)
		var load_meter: Dictionary = state.get("load_meter", {})
		if load_meter.is_empty():
			_fail("the hover panel on %s has no load bar" % key)
		if absf(float(load_meter["fraction"]) - float(percent) / 100.0) > 0.001:
			_fail("the load bar on %s is at %f, the sim's percent is %d" % [key, load_meter["fraction"], percent])
		main._apply_hover_panel_state(state)
		if not main._hover_load_meter.visible:
			_fail("the load bar must be shown for a machine")
		if absf(float(main._hover_load_meter.get_meta("fraction")) - float(load_meter["fraction"])) > 0.001:
			_fail("the load track did not record the fraction it was set to")
	# Any tile of the vat answers, and a hopper has no load bar.
	if main._hover_text_at_tile(Vector2i(13, 1)).find("flat out") < 0:
		_fail("hovering the vat's far corner must carry its load line")
	if not main._hover_panel_state(Vector2i(-1, 0)).get("load_meter", {}).is_empty():
		_fail("a hopper has no load bar")
	main._apply_hover_panel_state(main._hover_panel_state(Vector2i(-1, 0)))
	if main._hover_load_meter.visible:
		_fail("the load bar must hide on a tile with no load")
	# A blocked machine's reason survives the null ticks between held batches:
	# hover never flickers to nothing while the status holds.
	for i in range(300):
		main.sim.tick()
		main._poll_machine_load()
		if main._machine_load["4,0"]["reason"] == null:
			_fail("the crusher's remembered reason was dropped on a tick with a null stall")
		if String(main._machine_load["4,0"]["status"]) != "blocked":
			_fail("the crusher stopped reading blocked during the settled run")

	# --- 5. a save loaded back lights up on the first frame ------------------
	var save_text: String = main.sim.save_ron()
	main.sim.new_game()
	main._rebuild_world()
	if not main._machine_andons.is_empty():
		_fail("an empty world has no andons")
	var refusal: String = main.sim.load_ron(save_text)
	if refusal != "":
		_fail("load_ron refused the settled line's save: %s" % refusal)
	main._rebuild_world()
	for key in expected.keys():
		if _lit_lens(key) != expected[key]:
			_fail("after a load, before any tick, %s shows '%s', expected %s" % [key, _lit_lens(key), expected[key]])
		if main._hover_text_at_tile(_key_tile(key)).find(expected[key] if expected[key] != "working" else "flat out") < 0:
			_fail("after a load, hover on %s does not carry its status" % key)

	# --- 6. a real status change, polled on the tick; pause freezes it ------
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	Layouts.stock_larder(main.sim, 200)
	main.sim.issue_command_json(JSON.stringify({"type": "set_hopper_auto_fill", "tile": [-1, 0], "enabled": true}))
	main._rebuild_world()
	var crusher := "4,0"
	if _lit_lens(crusher) != "starved":
		_fail("a machine that has not run yet reads starved (nothing observed), got '%s'" % _lit_lens(crusher))
	if _sim_status(crusher) != "starved":
		_fail("the sim itself should read starved before any tick")
	if main._machine_load_poll_due():
		_fail("a rebuild polls; nothing is due straight after it")
	# The pause menu: the sim can be moved on underneath (as a test may), and
	# the lights must not follow until the game runs again. The run is the
	# belt's transit (~200 ticks for the first tomato to reach the crusher)
	# plus a whole window, so the idle start has scrolled out of it.
	for i in range(1300):
		main.sim.tick()
	if _sim_status(crusher) != "working":
		_fail("a fed crusher with room downstream should read working after 1300 ticks, sim says %s" % _sim_status(crusher))
	var polled_at: int = main._load_polled_tick
	main._pause_open = true
	for i in range(int(main.LOAD_POLL_TICKS) * 3):
		_step()
	main._pause_open = false
	if main._load_polled_tick != polled_at:
		_fail("the lights were polled with the pause menu up")
	if _lit_lens(crusher) != "starved":
		_fail("the crusher's light changed with the pause menu up")
	# Running again: the next frame's step is due and the lens flips — one
	# visible, the old one hidden.
	_step()
	if _lit_lens(crusher) != "working":
		_fail("after resuming, the crusher's light must follow the sim to working, shows '%s'" % _lit_lens(crusher))
	if main._machine_andons[crusher]["lenses"]["starved"].visible:
		_fail("the old lens must be hidden when the status changes")
	# The cadence: the next poll is LOAD_POLL_TICKS of simulation away, no
	# sooner, and a step that ticks is what brings it.
	var after_poll: int = main._load_polled_tick
	if main._machine_load_poll_due():
		_fail("nothing is due straight after a poll")
	for i in range(int(main.LOAD_POLL_TICKS) - 1):
		_step()
	if main._load_polled_tick != after_poll:
		_fail("polled again %d ticks after the last poll, before LOAD_POLL_TICKS" % (main.sim.current_tick() - after_poll))
	_step()
	if main._load_polled_tick == after_poll:
		_fail("LOAD_POLL_TICKS ticks after a poll, the next one is due")
	# Setting the same status again is a no-op on the nodes.
	main._set_machine_andon(crusher, "working")
	if _lit_lens(crusher) != "working":
		_fail("re-applying the shown status must change nothing")

	if _failed:
		quit(1)
		return
	print("MACHINE_LOAD_OK")
	quit(0)
