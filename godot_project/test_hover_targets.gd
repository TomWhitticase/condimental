# Headless proof that a `--hover*` screenshot flag cannot miss in silence.
#
# Run with:
#   godot --headless --path godot_project --script test_hover_targets.gd
#
# WHY THIS EXISTS. `scripts/screenshot-scene.sh` is the one check in this
# project that needs a window, so nothing about it runs in CI — and until
# this file there was nothing in CI that knew the hover flags existed. What
# that cost is the bug this test was written for: every hover helper was
# guarded by `if tile != NONE`, so a flag whose target was not in the world
# aimed at nothing, the run photographed the parked cursor, and
# `--fixture full_dock --hover-splitter` handed back a clean frame of the
# dock with SCREENSHOT_OK on it and exit 0. Plain `--hover` did the same
# against the empty world. A reviewer reading that frame is reading an
# answer to a question nobody asked (issue #70's class exactly), and the
# harness is what every other PR's evidence is produced with.
#
# WHAT IT CAN AND CANNOT SEE. `hover_targets.gd` is deliberately just
# `world_layout_json` in and a tile out, so everything about WHICH tile a
# flag means, and whether a world has one at all, is checkable here with no
# framebuffer. What needs the window — that the cursor really lands there,
# and that the refusal writes no PNG — stays where it has to be, in
# `test_screenshot.gd`'s own shutter guard and in the rendered shots on the
# pull request. `scripts/check-invariants.sh` closes the seam between them:
# it fails if `test_screenshot.gd` names a `--hover*` flag outside the table.
#
# Verifies:
#   * `FLAGS` and `all()` list the same flags in the same order, so the
#     command-line parser and the finders cannot drift apart,
#   * in an EMPTY world every flag finds nothing and produces a refusal that
#     says so in those words rather than blaming a missing dock,
#   * the reported case — `--fixture full_dock --hover-splitter` — finds
#     nothing, and its refusal names both the flag and the fixture,
#   * in a fixture that DOES contain the thing, every flag finds it, the
#     tile really belongs to a placement of that kind, and nothing refuses
#     (the half that would catch a refusal fired at the wrong time),
#   * `--hover-vat` picks a tile of a multi-tile machine that is NOT its
#     origin, which is the whole claim that flag photographs.

extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const HoverTargets = preload("res://hover_targets.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("HOVER_TARGETS_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	# Only for `_footprint_size`, which the table needs and which belongs to
	# main.gd: passing the REAL one is half of what makes `--hover-vat`
	# checkable here rather than against a second copy of that parsing.
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


## A world built by one of `test_layouts.gd`'s fixtures, as the table reads
## it. A fresh SimBridge each time, so fixtures cannot stack on one floor.
func _world(build: Callable) -> Array:
	var sim = SimBridge.new()
	build.call(sim)
	return JSON.parse_string(sim.world_layout_json())


## The kind of the placement covering `tile`, or "" — footprints included, so
## the far corner of a 2x2 vat resolves to the vat.
func _kind_at(layout: Array, tile: Vector2i) -> String:
	for placement in layout:
		var pair: Array = placement["tile"]
		var origin := Vector2i(int(pair[0]), int(pair[1]))
		var size: Vector2i = main._footprint_size(placement.get("footprint"))
		if Rect2i(origin, size).has_point(tile):
			return String(placement["kind"])
	return ""


func _row(flag: String) -> Dictionary:
	for row in HoverTargets.all(main._footprint_size):
		if row["flag"] == flag:
			return row
	_fail("no row in hover_targets.gd for %s" % flag)
	return {}


func _run_checks() -> void:
	var rows := HoverTargets.all(main._footprint_size)

	# --- the table describes itself ----------------------------------------
	var listed: PackedStringArray = []
	for row in rows:
		listed.append(row["flag"])
		if String(row["looks_for"]).is_empty():
			_fail("%s has nothing to say about what it looks for, so its \
refusal could not name it" % row["flag"])
			return
	if listed != HoverTargets.FLAGS:
		_fail("FLAGS is %s but all() lists %s — the parser and the finders \
have drifted" % [str(HoverTargets.FLAGS), str(listed)])
		return

	# --- the empty world: the shortest command there is --------------------
	# `--hover` with no fixture is the easiest way to reach this, and used to
	# be the most convincing: bare floor, a tooltip-less frame, exit 0.
	var empty: Array = _world(func(_sim): pass)
	for row in rows:
		if HoverTargets.tile_for(row, empty) != HoverTargets.NONE:
			_fail("%s found something in an empty world" % row["flag"])
			return
		var refusal := HoverTargets.refusal(row, "", "", empty)
		if not refusal.begins_with("SCREENSHOT_FAIL: "):
			_fail("%s's empty-world refusal does not start with the marker \
scripts read for a refusal: %s" % [row["flag"], refusal])
			return
		if not refusal.contains(row["flag"]):
			_fail("%s's refusal does not name the flag: %s"
				% [row["flag"], refusal])
			return
		if not refusal.contains("built no world at all"):
			_fail("%s's refusal blames a missing thing rather than the empty \
world it actually looked at: %s" % [row["flag"], refusal])
			return

	# --- the reported case, exactly ----------------------------------------
	# `scripts/screenshot-scene.sh out.png 1600x900 --fixture full_dock
	# --hover-splitter` — a full line, correctly built, with no splitter
	# anywhere in it.
	var full_dock := _world(Layouts.build_full_dock_line)
	if full_dock.is_empty():
		_fail("the full_dock fixture built nothing, so the reported case is \
not being reproduced here at all")
		return
	var splitter := _row("--hover-splitter")
	if HoverTargets.tile_for(splitter, full_dock) != HoverTargets.NONE:
		_fail("the full_dock line now contains a splitter, so this no longer \
reproduces the reported case — point it at a fixture that has none")
		return
	var reported := HoverTargets.refusal(splitter, "full_dock", "", full_dock)
	for expected in ["--hover-splitter", "full_dock", "conveyor splitter"]:
		if not reported.contains(expected):
			_fail("the refusal for the reported case does not mention '%s': %s"
				% [expected, reported])
			return

	# --- and a world that HAS one of each ----------------------------------
	# The other half of the guard: a refusal that fired when the thing was
	# there would break every hover shot in the project, so each flag is
	# pointed at a fixture that contains its target and has to find it.
	var found := {
		"--hover": [_world(Layouts.build_machines_fixture), "crusher"],
		"--hover-pipe": [_world(Layouts.build_fluid_run), "pipe"],
		"--hover-hopper": [_world(Layouts.build_machines_fixture), "hopper"],
		"--hover-tank": [_world(Layouts.build_fluid_run), "fluid_tank"],
		"--hover-dock": [_world(Layouts.build_machines_fixture), "dock"],
		"--hover-splitter": [_world(Layouts.build_splitter_line), "splitter"],
		"--hover-vat": [_world(Layouts.build_multi_tile), "cooking_vat"],
	}
	for flag in found:
		var layout: Array = found[flag][0]
		var kind: String = found[flag][1]
		var tile: Vector2i = HoverTargets.tile_for(_row(flag), layout)
		if tile == HoverTargets.NONE:
			_fail("%s found nothing in a fixture built to contain a %s"
				% [flag, kind])
			return
		if _kind_at(layout, tile) != kind:
			_fail("%s aimed at %s, which is a '%s' rather than a '%s'"
				% [flag, str(tile), _kind_at(layout, tile), kind])
			return

	# --- the multi-tile claim ----------------------------------------------
	# "Hovering ANY tile of a multi-tile machine identifies it" is what
	# --hover-vat is for, and the origin is the one tile that would not show
	# it — so a finder that quietly returned the origin would photograph a
	# true-looking frame that proves nothing.
	var vat_world := _world(Layouts.build_multi_tile)
	var vat_tile: Vector2i = HoverTargets.tile_for(_row("--hover-vat"), vat_world)
	var vat_origin_pair: Array = vat_world[0]["tile"]
	var vat_origin := Vector2i(int(vat_origin_pair[0]), int(vat_origin_pair[1]))
	if vat_tile == vat_origin:
		_fail("--hover-vat aimed at the vat's ORIGIN %s, which is the one \
tile that cannot show that any tile of a footprint identifies it" % str(vat_tile))
		return
	var vat_size: Vector2i = main._footprint_size(vat_world[0].get("footprint"))
	if vat_tile != vat_origin + vat_size - Vector2i.ONE:
		_fail("--hover-vat aimed at %s, which is not the far corner of the \
vat at %s (%s)" % [str(vat_tile), str(vat_origin), str(vat_size)])
		return

	print("HOVER_TARGETS_OK: %d hover flags, one table; every one of them \
finds nothing in an empty world and refuses in words that name the flag and \
the world, --hover-splitter still finds nothing in the %d-placement full_dock \
line (the reported case), every flag finds its own target in a fixture that \
has one and lands on a placement of that kind, and --hover-vat lands on the \
vat's far corner %s rather than its origin %s"
		% [rows.size(), full_dock.size(), str(vat_tile), str(vat_origin)])
	quit(0)
