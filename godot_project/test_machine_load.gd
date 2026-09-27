# Headless proof of the machine-load indicator's Godot half (the machine-load
# epic, ticket 3; `docs/MACHINE_LOAD_PLAN.md`), as it stands since the
# in-world andon stack lights were REMOVED on 2026-09-12: a machine's load
# is polled from the simulation on the tick and read in the HOVER PANEL —
# a status glyph and word leading the load line, the number, the reason it
# is waiting, and a load bar filled in the status colour. Nothing about it
# stands on the floor.
#
# Run with:
#   godot --headless --path godot_project --script test_machine_load.gd
#
# Verifies:
#   * the load line carries a status glyph AND word for all three cases,
#     in the plan's exact language, with the window in seconds from
#     `window / TICK_HZ` and items through `_item_display_name`,
#   * the load bar's colour follows the status (green working, amber
#     starved, red blocked) and its fraction is the sim's percent,
#   * on the settled bottleneck line every recipe-running machine has a
#     load record that is the sim's own verdict — blocked · blocked ·
#     working · starved · starved along the line — and nothing that is not
#     a machine has one; and NO node on the floor is an andon,
#   * hover on any tile of a machine's footprint says its status, and a
#     blocked machine's remembered reason survives the null ticks between
#     held batches,
#   * a save loaded back reads its statuses on the first frame, before any
#     tick,
#   * a real status change is polled through `_advance_simulation` as a
#     fresh line starts up, the pause menu freezes polling, and the cadence
#     is LOAD_POLL_TICKS of simulation,
#   * the two staffing facts the same record carries (issues #19 and #20):
#     a machine going without its crew says so in the stall's own words and
#     turns its bar amber even on the frames it has no live stall, a
#     MOTHBALLED machine reads "switched off" with a fourth glyph and an
#     unlit bar rather than as a fault, and both really arrive over the
#     wire for every machine on the line,
#   * a machine the sim reports `understaffed` floats the red person over
#     its roof, and no other machine does — not a staffed one, not one
#     waiting on input, not one switched off — and it holds, poll after
#     poll, while the vat's per-tick `staff` stall blinks under it.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
## How long the understaffed fixture is run on to prove its marker holds:
## the review's probe length, some 200 load polls.
const STEADY_TICKS := 6000

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
## off the wire — the reference every record is checked against.
func _sim_status(key: String) -> String:
	var parsed = JSON.parse_string(main.sim.machine_load_snapshots_json())
	for entry in parsed:
		if main._tile_key(main._pair_to_tile(entry.get("tile"))) == key:
			return String(entry.get("load", {}).get("status", ""))
	return ""


## The status the scene holds for the machine at `key`, or "" for none.
func _shown_status(key: String) -> String:
	if not main._machine_load.has(key):
		return ""
	return String(main._machine_load[key].get("status", ""))


## The keys of every raised understaffed marker, sorted.
func _machine_markers() -> Array:
	var keys: Array = []
	for key in main._world_warnings.keys():
		if String(key).begins_with("machine:"):
			keys.append(key)
	keys.sort()
	return keys


## "x,y" back to a tile.
func _key_tile(key: String) -> Vector2i:
	var parts := key.split(",")
	return Vector2i(int(parts[0]), int(parts[1]))


func _run_checks() -> void:
	# --- 1. the words, pure -------------------------------------------------
	var glyphs: Dictionary = main.LOAD_STATUS_GLYPHS
	for status in ["working", "starved", "blocked"]:
		if not glyphs.has(status) or String(glyphs[status]).is_empty():
			_fail("LOAD_STATUS_GLYPHS has no glyph for %s" % status)
	if glyphs.values().size() != 3 or glyphs["working"] == glyphs["starved"] or glyphs["starved"] == glyphs["blocked"] or glyphs["working"] == glyphs["blocked"]:
		_fail("the three status glyphs must be three different shapes, got %s" % str(glyphs))

	var window: int = 600
	var probe := "77,77"
	main._machine_load[probe] = {
		"status": "working", "reason": null,
		"load": {"percent": 100, "window": window, "status": "working"},
	}
	var expected_seconds := int(round(float(window) / main.TICK_HZ))
	var flat_out: String = main._machine_load_text(probe)
	if flat_out != "%s flat out  ·  100%% of the last %ds" % [glyphs["working"], expected_seconds]:
		_fail("a working machine reads '● flat out · 100%% of the last Ns', got '%s'" % [flat_out])
	if flat_out.find("%ds" % expected_seconds) < 0:
		_fail("the window must be window / TICK_HZ seconds (%d), got '%s'" % [expected_seconds, flat_out])
	main._machine_load[probe] = {
		"status": "starved", "reason": {"kind": "input", "item": "sugar"},
		"load": {"percent": 38, "window": window, "status": "starved"},
	}
	var starved: String = main._machine_load_text(probe)
	if starved != "%s starved  ·  38%% busy  ·  waiting for %s" % [glyphs["starved"], main._item_display_name("sugar")]:
		_fail("a starved machine reads '▲ starved · 38%% busy · waiting for Sugar', got '%s'" % [starved])
	if starved.find("sugar") >= 0 and main._item_display_name("sugar") != "sugar":
		_fail("the item must go through _item_display_name, got '%s'" % starved)
	main._machine_load[probe] = {
		"status": "blocked", "reason": {"kind": "output", "item": "tomato_paste"},
		"load": {"percent": 61, "window": window, "status": "blocked"},
	}
	var blocked: String = main._machine_load_text(probe)
	if blocked != "%s blocked  ·  61%% busy  ·  output full" % glyphs["blocked"]:
		_fail("a blocked machine reads '■ blocked · 61%% busy · output full', got '%s'" % [blocked])
	# No remembered reason: the status and the number alone, nothing invented.
	main._machine_load[probe] = {
		"status": "blocked", "reason": null,
		"load": {"percent": 61, "window": window, "status": "blocked"},
	}
	if main._machine_load_text(probe) != "%s blocked  ·  61%% busy" % glyphs["blocked"]:
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

	# --- 2. the colour, pure -------------------------------------------------
	# Three statuses, three colours, and they are the panel's own meter
	# colours rather than three new ones (UI_UX.md §5e/§7).
	var colors := {
		"working": main._load_status_color("working"),
		"starved": main._load_status_color("starved"),
		"blocked": main._load_status_color("blocked"),
	}
	if colors["working"] != main.METER_FILL_LEVEL:
		_fail("a working machine's load bar is the level colour")
	if colors["starved"] != main.METER_FILL_PROGRESS:
		_fail("a starved machine's load bar is the amber colour")
	if colors["blocked"] != main.METER_FILL_WARN:
		_fail("a blocked machine's load bar is the warning colour")
	if colors["working"] == colors["starved"] or colors["starved"] == colors["blocked"] or colors["working"] == colors["blocked"]:
		_fail("the three status colours must differ")

	# --- 3. the settled line: records, and no poles ---------------------------
	Layouts.build_bottleneck_line(main.sim)
	main._rebuild_world()

	var expected: Dictionary = Layouts.BOTTLENECK_MACHINES
	if main._machine_load.size() != expected.size():
		_fail("%d machines on the line, %d load records" % [expected.size(), main._machine_load.size()])
	for key in expected.keys():
		if not main._machine_load.has(key):
			_fail("the machine at %s has no load record" % key)
			continue
		var sim_status := _sim_status(key)
		if sim_status != expected[key]:
			_fail("the settled line's %s reads %s, the fixture expects %s" % [key, sim_status, expected[key]])
		if _shown_status(key) != sim_status:
			_fail("_machine_load[%s] says %s, the sim says %s" % [key, _shown_status(key), sim_status])
	# Nothing else gets a record: hoppers, the tank, the dock, a belt, a pipe.
	for key in ["-1,0", "4,1", "5,2", "26,0", "0,0", "10,0"]:
		if main._machine_load.has(key):
			_fail("%s is not a recipe-running machine and must have no load record" % key)
	# And nothing stands on the floor for it: the andon poles are gone, and
	# a machine's body still owns its own status lamp.
	for child in main._world_root.get_children():
		if String(child.name).begins_with("andon_"):
			_fail("an andon node %s is standing in the world — the poles were removed" % child.name)
	for key in expected.keys():
		var lamp = main._machine_lamps.get(key)
		if lamp == null or not String(lamp.get_parent().name).ends_with("_%s" % key):
			_fail("the machine at %s has no status lamp on its own body" % key)

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
		var status: String = expected[key]
		if text.find(String(glyphs[status])) < 0:
			_fail("hover on %s must lead its load line with the %s glyph %s, got:\n%s" % [key, status, glyphs[status], text])
		var percent := int(main._machine_load[key]["load"]["percent"])
		if text.find("%d%%" % percent) < 0:
			_fail("hover on %s must carry the sim's %d%%, got:\n%s" % [key, percent, text])
		var state: Dictionary = main._hover_panel_state(origin)
		var load_meter: Dictionary = state.get("load_meter", {})
		if load_meter.is_empty():
			_fail("the hover panel on %s has no load bar" % key)
			continue
		if absf(float(load_meter["fraction"]) - float(percent) / 100.0) > 0.001:
			_fail("the load bar on %s is at %f, the sim's percent is %d" % [key, load_meter["fraction"], percent])
		if load_meter["color"] != colors[status]:
			_fail("the load bar on %s (%s) is not in the %s colour" % [key, status, status])
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
		if _shown_status("4,0") != "blocked":
			_fail("the crusher stopped reading blocked during the settled run")

	# --- 5. a save loaded back reads right on the first frame ----------------
	var save_text: String = main.sim.save_ron()
	main.sim.new_game()
	main._rebuild_world()
	if not main._machine_load.is_empty():
		_fail("an empty world has no load records")
	var refusal: String = main.sim.load_ron(save_text)
	if refusal != "":
		_fail("load_ron refused the settled line's save: %s" % refusal)
	main._rebuild_world()
	for key in expected.keys():
		if _shown_status(key) != expected[key]:
			_fail("after a load, before any tick, %s reads '%s', expected %s" % [key, _shown_status(key), expected[key]])
		if main._hover_text_at_tile(_key_tile(key)).find(expected[key] if expected[key] != "working" else "flat out") < 0:
			_fail("after a load, hover on %s does not carry its status" % key)

	# --- 6. a real status change, polled on the tick; pause freezes it ------
	# The machine watched is the COOKING VAT: it is the line's bottleneck,
	# so once the mixer's run is charged it reads flat out and stays there.
	# The crusher used to be the subject, but since the half-tile item
	# spacing (the quality pass, 2026-09-13) one lane delivers four tomatoes
	# per 100 ticks against its 80-tick craft, so a fed crusher never
	# reaches the 95 % that reads `working`.
	main.sim.new_game()
	Layouts.build_reference_line(main.sim)
	Layouts.stock_larder(main.sim, 200)
	for tile in [Vector2i(-1, 0), Vector2i(4, 1)]:
		main.sim.issue_command_json(JSON.stringify({"type": "set_hopper_auto_fill", "tile": [tile.x, tile.y], "enabled": true}))
	main.sim.issue_command_json(JSON.stringify({"type": "set_tank_auto_fill", "tile": [5, 2], "enabled": true}))
	main._rebuild_world()
	var vat := "12,0"
	if _shown_status(vat) != "starved":
		_fail("a machine that has not run yet reads starved (nothing observed), got '%s'" % _shown_status(vat))
	if _sim_status(vat) != "starved":
		_fail("the sim itself should read starved before any tick")
	if main._machine_load_poll_due():
		_fail("a rebuild polls; nothing is due straight after it")
	# The pause menu: the sim can be moved on underneath (as a test may), and
	# the records must not follow until the game runs again. The run is the
	# time the front of the line takes to charge the vat's inlet run (the
	# vat reads flat out from about tick 1500) plus a whole window, so the
	# idle start has scrolled out of it.
	for i in range(2200):
		main.sim.tick()
	if _sim_status(vat) != "working":
		_fail("a fed vat with room downstream should read working after 2200 ticks, sim says %s" % _sim_status(vat))
	var polled_at: int = main._load_polled_tick
	main._pause_open = true
	for i in range(int(main.LOAD_POLL_TICKS) * 3):
		_step()
	main._pause_open = false
	if main._load_polled_tick != polled_at:
		_fail("the load was polled with the pause menu up")
	if _shown_status(vat) != "starved":
		_fail("the vat's status changed with the pause menu up")
	# Running again: the next frame's step is due and the record follows.
	_step()
	if _shown_status(vat) != "working":
		_fail("after resuming, the vat must follow the sim to working, reads '%s'" % _shown_status(vat))
	if main._hover_text_at_tile(_key_tile(vat)).find("flat out") < 0:
		_fail("after resuming, hover on the vat must read flat out")
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

	# --- 7. the staffing readout (issues #19 and #20) ------------------------
	# Two things the same polled record now carries that are NOT statuses,
	# and both of them beat the status when they are true.
	var staff_probe := "88,88"
	# #20: a machine going without its crew says so on every frame, not just
	# on the ticks it got as far as asking and being refused. The words are
	# the ones a `staff` stall gives, so the two readings cannot contradict.
	main._crews[staff_probe] = [{"category": "manufacturing", "count": 2}]
	if main._uncrewed_reason_text(staff_probe) != "short of manufacturing staff":
		_fail("an uncrewed machine's words are '%s'" % main._uncrewed_reason_text(staff_probe))
	if main._stall_reason_text({"kind": "staff", "item": "manufacturing"}) \
			!= main._uncrewed_reason_text(staff_probe):
		_fail("the stall and the establishment say different things about the same shortage")
	main._machine_load[staff_probe] = {
		"status": "starved", "reason": null, "crewed": false, "mothballed": false,
		"load": {"percent": 12, "window": window, "status": "starved"},
	}
	var uncrewed_line: String = main._machine_load_text(staff_probe)
	if uncrewed_line != "%s starved  ·  12%% busy  ·  short of manufacturing staff" % glyphs["starved"]:
		_fail("an uncrewed machine with no live stall reads '%s'" % uncrewed_line)
	if main._load_bar_color(main._machine_load[staff_probe]) != main.METER_FILL_PROGRESS:
		_fail("an uncrewed machine's load bar is not amber")
	# A live stall is the more specific answer and keeps its place.
	main._machine_load[staff_probe]["reason"] = {"kind": "input", "item": "sugar"}
	if main._machine_load_text(staff_probe).find("waiting for") < 0:
		_fail("the uncrewed words displaced a live stall reason")
	# #19: mothballed beats everything, because it is not a fault.
	main._machine_load[staff_probe] = {
		"status": "starved", "reason": {"kind": "mothballed", "item": ""},
		"crewed": true, "mothballed": true,
		"load": {"percent": 12, "window": window, "status": "starved"},
	}
	if main._stall_reason_text({"kind": "mothballed", "item": ""}) != "switched off":
		_fail("the mothballed stall kind reads '%s'" % main._stall_reason_text({"kind": "mothballed", "item": ""}))
	var off_line: String = main._machine_load_text(staff_probe)
	if off_line.find("switched off") < 0 or off_line.find(main.LOAD_OFF_GLYPH) < 0:
		_fail("a mothballed machine's line is '%s' — it needs the glyph and the words" % off_line)
	if off_line.find("starved") >= 0:
		_fail("a machine the player switched off must not be reported as starved: '%s'" % off_line)
	if main._load_bar_color(main._machine_load[staff_probe]) != main.METER_FILL_OFF:
		_fail("a mothballed machine's load bar is not the unlit fill")
	if main.LOAD_OFF_GLYPH in glyphs.values():
		_fail("the switched-off glyph must be a fourth shape, not one of the three statuses")
	main._machine_load.erase(staff_probe)
	main._crews.erase(staff_probe)

	# And the two facts really do come off the wire, for every machine on
	# the line, rather than being defaults this file supplies.
	for key in expected.keys():
		var record: Dictionary = main._machine_load.get(key, {})
		if not record.has("crewed") or not record.has("mothballed"):
			_fail("the polled record for %s carries no staffing facts: %s" % [key, str(record)])
			continue
		if not bool(record["crewed"]):
			_fail("a funded line's %s reads uncrewed" % key)
		if bool(record["mothballed"]):
			_fail("nothing on this line was switched off, but %s reads mothballed" % key)

	# --- the wire itself, on a line where the answer is NOT the default ----------
	#
	# Everything above this point runs on a fully crewed line, where
	# `crewed: true` is also what a cut wire would produce — so none of it
	# can tell the two apart. THIS can: a fixture built by real commands,
	# genuinely short of manufacturing heads, where the sim's answer for
	# one machine is FALSE. Hardwire `record["crewed"]` in
	# `_poll_machine_load` and this is the test that goes red.
	main.sim.new_game()
	Layouts.build_understaffed_line(main.sim)
	main._rebuild_world()
	var vat_key: String = main._tile_key(Layouts.UNDERSTAFFED_VAT)
	if Layouts.understaffed_stall(main.sim, Layouts.UNDERSTAFFED_VAT) != Layouts.UNDERSTAFFED_CATEGORY:
		_fail("the fixture's vat must be refused its crew on the wire before this proves anything")
	var uncrewed_keys: Array = []
	for key in main._machine_load.keys():
		if not bool(main._machine_load[key].get("crewed", true)):
			uncrewed_keys.append(key)
	if uncrewed_keys != [vat_key]:
		_fail("the sim says exactly the vat goes without its crew; the panel says %s" % str(uncrewed_keys))

	# The red person over it, and over nothing else on the line: every
	# other machine there is crewed, whatever else it is waiting for.
	var marked := _machine_markers()
	if marked != ["machine:" + vat_key]:
		_fail("only the vat refused its crew should carry the understaffed marker, got %s" % str(marked))
	else:
		var person: Node3D = main._world_warnings["machine:" + vat_key]
		if not String(person.name).begins_with(main.UNDERSTAFFED_MARKER_ASSET):
			_fail("the vat's marker is '%s', not the red person" % person.name)
		if person.get_parent() != main._world_root:
			_fail("the understaffed marker is not in the world")
		var roof: float = main.FLOOR_Y + main._mesh_for("machine_cooking_vat").get_aabb().end.y
		if person.position.y <= roof:
			_fail("the vat's marker stands at %.2f, inside a roof at %.2f" % [person.position.y, roof])

	# ...and it HOLDS. The vat is handed its crew on any tick the crusher or
	# the mixer claims nothing, so its `staff` stall blinks off about a
	# quarter of the time while it stays short; a marker read off that
	# stall toggled 43 times in 201 polls here (the PR #192 review). Run
	# the fixture on through the real tick path, which polls on its own
	# cadence, and fail on any poll the person is missing while the sim
	# still says the vat is short.
	var person_node = main._world_warnings.get("machine:" + vat_key)
	var marker_polled_at: int = main._load_polled_tick
	var polls := 0
	var stall_flips := 0
	var last_staff := Layouts.understaffed_stall(main.sim, Layouts.UNDERSTAFFED_VAT) != ""
	for i in range(STEADY_TICKS):
		if i % Layouts.RESTOCK_EVERY == 0:
			Layouts.restock(main.sim)
		main._advance_simulation(1.0 / main.TICK_HZ)
		var staff := Layouts.understaffed_stall(main.sim, Layouts.UNDERSTAFFED_VAT) != ""
		if staff != last_staff:
			stall_flips += 1
		last_staff = staff
		if main._load_polled_tick == marker_polled_at:
			continue
		marker_polled_at = main._load_polled_tick
		polls += 1
		if bool(main._machine_load[vat_key].get("crewed", true)):
			_fail("the vat was crewed by tick %d; the steady-marker check proves nothing" % marker_polled_at)
			break
		if _machine_markers() != ["machine:" + vat_key]:
			_fail("poll at tick %d: the vat is still short but the markers are %s"
				% [marker_polled_at, str(_machine_markers())])
			break
		if main._world_warnings["machine:" + vat_key] != person_node:
			_fail("poll at tick %d: the vat's person was freed and re-made" % marker_polled_at)
			break
	if polls < STEADY_TICKS / main.LOAD_POLL_TICKS / 2:
		_fail("only %d polls in %d ticks: the tick path did not poll" % [polls, STEADY_TICKS])
	# The reason the check exists: the stall under it really did blink.
	if stall_flips < 10:
		_fail("the vat's staff stall flipped only %d times; the fixture no longer blinks" % stall_flips)

	# And the same for the switch, thrown by a real command rather than
	# written into a dictionary here — through the config panel's own path,
	# checked straight after the click with no tick or rebuild between, so
	# it is that path's own re-poll that clears the person, not the next.
	var vat_tile := [Layouts.UNDERSTAFFED_VAT.x, Layouts.UNDERSTAFFED_VAT.y]
	main._issue_config_command({"type": "set_mothballed", "tile": vat_tile, "enabled": true})
	if not _machine_markers().is_empty():
		_fail("switching the vat off must clear its person with the click, got %s" % str(_machine_markers()))
	main._issue_config_command({"type": "set_mothballed", "tile": vat_tile, "enabled": false})
	if _machine_markers() != ["machine:" + vat_key]:
		_fail("switching the vat back on must restore its person with the click, got %s" % str(_machine_markers()))
	main._issue_config_command({"type": "set_mothballed", "tile": vat_tile, "enabled": true})
	main._rebuild_world()
	if not bool(main._machine_load.get(vat_key, {}).get("mothballed", false)):
		_fail("a machine switched off by command must poll back as mothballed")
	var still_uncrewed: Array = []
	for key in main._machine_load.keys():
		if not bool(main._machine_load[key].get("crewed", true)):
			still_uncrewed.append(key)
	if not still_uncrewed.is_empty():
		_fail("with the vat switched off nothing goes without a crew, but %s does" % str(still_uncrewed))
	# Off is idle for a reason of the player's own: no person over it.
	if not _machine_markers().is_empty():
		_fail("a switched-off machine still floats the understaffed marker: %s" % str(_machine_markers()))

	if _failed:
		quit(1)
		return
	print("MACHINE_LOAD_OK")
	quit(0)
