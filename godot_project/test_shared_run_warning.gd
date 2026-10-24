# Headless check for the build ghost's SHARING WARNING (issue #92).
#
# Building the BBQ branch onto a running ketchup line used to give no warning
# at any point: the shop sold the mixer, the ghost went green, and the first
# the player knew was a CONTRACT LOST banner thousands of ticks later. Since
# issue #93 two machines on one run take turns rather than one starving, but
# a turn is still a share, and a line serving the three starter orders still
# loses two of them to the branch. So the ghost says so before the click:
#
#   * a mixer ghost on the branch's tap reads "▲ shares the Ketchup run with
#     Bottler" — the fluid and the machine named from the WIRE
#     (`placement_fluid_preview_json` and the display names), not from here,
#   * built in the issue's own order — mixer first, joint pipe second — the
#     PIPE's ghost reads "▲ joins Bottler, Mixer on the Ketchup run", since
#     it is that click which puts the two on one run,
#   * the same mixer away from any pipe, and a first consumer on a run
#     nobody draws from yet, read no warning at all,
#   * and the warning is the sim's answer, not GDScript's: the text is built
#     from `_ghost_shared_runs`, which only the bridge fills.
#
# Run: godot --headless --path godot_project --script test_shared_run_warning.gd
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## `sim::bbq_branch`'s joint 1, as `test_layouts.gd`'s `build_bbq_branch`
## lays it: one pipe touching the ketchup run below and the mixer above.
const BBQ_TAP := Vector2i(14, -1)
const BBQ_MIXER := Vector2i(14, -3)

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("SHARED_RUN_WARNING_FAIL: %s" % msg)
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


## The PLACE prompt for `kind` at `tile`, through the ghost's own door
## (`_preview_place`) and with the ghost's state set as `_update_ghost` sets
## it for a valid placement.
func _prompt_for(kind: String, tile: Vector2i) -> String:
	main._build_kind = kind
	main._remove_mode = false
	var preview: Dictionary = main._preview_place(kind, tile, "east")
	main._ghost_valid = bool(preview.get("valid", false))
	main._ghost_reason = ""
	main._ghost_result_kind = String(preview.get("result_kind", ""))
	return main._placement_prompt_text()


func _run_checks() -> void:
	Layouts.build_reference_line(main.sim)
	Layouts.fund(main.sim, 5000)
	Layouts.place(main.sim, "pipe", BBQ_TAP, "east")
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "mixer"}))
	main._rebuild_world()

	# --- 1. the branch's mixer is warned, in words from the wire -----------
	var prompt: String = _prompt_for("mixer", BBQ_MIXER)
	if not prompt.begins_with("✔ PLACE"):
		_fail("the BBQ mixer's tile must be a valid placement, got:\n%s" % prompt)
		return
	var expected := "▲ shares the %s run with %s" % [
		main._item_display_name("ketchup"), main._kind_display_name("bottler")
	]
	if not prompt.contains(expected):
		_fail("the mixer on the ketchup run must read '%s', got:\n%s" % [expected, prompt])
		return
	if main._ghost_shared_runs.size() != 1:
		_fail("one shared run off the bridge, got %s" % str(main._ghost_shared_runs))
		return

	# --- 2. and nothing is warned where nothing is shared -----------------
	if _prompt_for("mixer", Vector2i(40, 40)).contains("▲ shares"):
		_fail("a mixer touching no pipe shares nothing")
		return
	# The vat's outlet run before its bottler: the bottler is its FIRST
	# consumer, which is how every line is built.
	var bottler := Vector2i(-999, -999)
	for placement in JSON.parse_string(main.sim.world_layout_json()):
		if String(placement.get("kind", "")) == "bottler":
			bottler = Vector2i(int(placement["tile"][0]), int(placement["tile"][1]))
	main.sim.issue_command_json(JSON.stringify({"type": "remove", "tile": [bottler.x, bottler.y]}))
	main._rebuild_world()
	var first: String = _prompt_for("bottler", bottler)
	if not first.begins_with("✔ PLACE"):
		_fail("the lifted bottler's tile must be placeable again, got:\n%s" % first)
		return
	if first.contains("▲ shares"):
		_fail("a first consumer shares with nobody, got:\n%s" % first)
		return

	# --- 2b. the issue's own order: the mixer first, then the joint pipe --
	# A fresh scene, so the pipe laid in section 1 is not already there.
	main.queue_free()
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	Layouts.build_reference_line(main.sim)
	Layouts.fund(main.sim, 5000)
	Layouts.place(main.sim, "mixer", BBQ_MIXER, "east", "", "bbq_mix")
	main.sim.issue_command_json(JSON.stringify({"type": "purchase", "kind": "pipe"}))
	main._rebuild_world()
	var joint: String = _prompt_for("pipe", BBQ_TAP)
	if not joint.begins_with("✔ PLACE"):
		_fail("the joint pipe's tile must be a valid placement, got:\n%s" % joint)
		return
	var joins := "▲ joins %s, %s on the %s run" % [
		main._kind_display_name("mixer"), main._kind_display_name("bottler"),
		main._item_display_name("ketchup"),
	]
	var joins_other := "▲ joins %s, %s on the %s run" % [
		main._kind_display_name("bottler"), main._kind_display_name("mixer"),
		main._item_display_name("ketchup"),
	]
	if not (joint.contains(joins) or joint.contains(joins_other)):
		_fail("the joint pipe laid after the mixer must say it joins the two, got:\n%s" % joint)
		return

	# --- 3. the words are the wire's: a two-machine run counts them ------
	var two: String = main._shared_run_text({"pipe": [0, 0], "fluid": "vinegar", "drawn_by": ["mixer", "mixer"]})
	if two != "▲ shares the %s run with 2 × %s" % [main._item_display_name("vinegar"), main._kind_display_name("mixer")]:
		_fail("two machines of one kind read as a count, got '%s'" % two)
		return
	if not main._shared_run_text({"pipe": [0, 0], "fluid": null, "drawn_by": ["bottler"]}).contains("an unnamed run"):
		_fail("a run nothing has named is not given a guessed fluid")
		return

	print(
		"SHARED_RUN_WARNING_OK: a mixer ghost on the BBQ branch's tap reads '%s', " % expected
		+ "and built mixer-first the joint pipe's ghost says it joins the two — "
		+ "the run and the machine named off placement_fluid_preview_json and the display names — "
		+ "while a mixer touching no pipe and a first consumer on an undrawn run read no warning"
	)
	quit(0)
