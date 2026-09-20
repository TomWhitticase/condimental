# Headless proof of the fluid gallery (issue #72) — the structural claims a
# machine can check without a framebuffer. Whether the six colours actually
# READ is what `scripts/screenshot-scene.sh --fluid-gallery` is for, the same
# division `test_pipe_shapes.gd` and `test_belt_shapes.gd` draw for their own
# galleries.
#
# Run with:
#   godot --headless --path godot_project --script test_fluid_gallery.gd
#
# The fixture exists because `bbq` and `bbq_mix` stood at zero pressure in
# every other fixture, so the tints issue #68 gave them could not be
# photographed at all. It is a claim about the SIMULATION as much as about a
# frame — the six runs are named by topology, not by a label the fixture
# writes — and every way it can degrade is quiet: move one pipe and two
# networks merge into a single unnameable run, add a machine and a run goes
# ambiguous, get the fill wrong and a run is named but invisible. In each
# case the render still comes back a perfectly good-looking factory, missing
# one colour the reader was told it held. Verifies:
#   * the checker is not vacuous: an EMPTY world is refused before anything
#     is placed, so a silent "" later means something,
#   * every placement the gallery asks for is accepted (a refused purchase
#     would drop a run and nothing else would say so),
#   * `check_fluid_gallery` passes on the shipped layout: six runs, each
#     `carries` with one fluid named, each above the pressure main.gd needs,
#   * the six named fluids are exactly `FLUID_GALLERY_FLUIDS`,
#   * `test_layouts.gd`'s hand-copied `FLUID_MIN_VISIBLE_PRESSURE` still
#     equals `main.gd`'s, the number it was copied from,
#   * every one of the six has a `FLUID_TINTS` entry, no two alike, and none
#     of them near-grey (issue #100 — see `FLUID_TINT_MIN_SATURATION`).
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

## How much chroma a fluid tint must carry. A fluid body is drawn inside a
## grey steel bore, so a near-grey tint is the one colour that cannot be seen
## in a pipe however its value is tuned: `liquid_smoke` shipped at
## (0.36, 0.32, 0.26), saturation 0.278, and measured 11 luma from the steel
## either side of it — a full run nobody could find by eye at 1280x720 while
## the other five were obvious (issue #100). The floor sits above that and
## below the five that always read; it is a floor on SATURATION rather than
## on distance-from-steel because the failure is chromatic, not tonal —
## `vinegar` is no darker than the old smoke was and has never been hard to
## see.
const FLUID_TINT_MIN_SATURATION := 0.35

## How far apart two tints have to be, summed over RGB, to count as
## different liquids. The same number and the same reason as
## `test_bbq_branch.gd`'s check of the runs its own fixture names; this one
## covers all six at once, which is what the gallery is for.
const FLUID_TINT_MIN_DISTANCE := 0.08

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("FLUID_GALLERY_FAIL: %s" % msg)
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


func _run_checks() -> void:
	# --- 1. the checker refuses an empty world ----------------------------
	# Run before anything is placed, so that the "" further down is evidence
	# rather than a default. A check that cannot fail is the thing this whole
	# fixture exists to avoid.
	if Layouts.check_fluid_gallery(main.sim) == "":
		_fail(
			"check_fluid_gallery() passed a world with nothing in it — the check is vacuous, "
			+ "so its silence on the real gallery would mean nothing"
		)
		return

	# --- 2. the gallery builds, and says so itself ------------------------
	var complaint: String = Layouts.build_fluid_gallery(main.sim)
	if complaint != "":
		_fail("build_fluid_gallery() refused its own layout: %s" % complaint)
		return
	Layouts.stock_larder(main.sim)
	main._rebuild_world()
	main._update_fluid_network()

	var layout: Array = JSON.parse_string(main.sim.world_layout_json())
	if layout.size() != Layouts.FLUID_GALLERY_PLACEMENTS:
		_fail(
			"the gallery placed %d things, expected %d — a refused placement, and a run short"
			% [layout.size(), Layouts.FLUID_GALLERY_PLACEMENTS]
		)
		return

	# --- 3. six runs, each carrying one of the six ------------------------
	var snapshots: Array = JSON.parse_string(main.sim.fluid_snapshots_json())
	var carried := {}
	for run in snapshots:
		carried[String(run.get("fluid", ""))] = true
	var expected := {}
	for fluid in Layouts.FLUID_GALLERY_FLUIDS:
		expected[fluid] = true
	if carried.keys().size() != expected.keys().size():
		_fail(
			"the gallery's runs carry %s, and the fixture is written against %s"
			% [str(carried.keys()), str(expected.keys())]
		)
		return
	for fluid in expected:
		if not carried.has(fluid):
			_fail("no run carries '%s'" % fluid)
			return

	# --- 4. the hand-copied floor has not drifted -------------------------
	# `test_layouts.gd` restates `main.gd`'s number because its helpers are
	# static and have no scene to ask. Two copies of a constant drift
	# silently in the direction that matters: if main's floor RISES, the
	# gallery's check goes quietly weaker rather than louder.
	if not is_equal_approx(Layouts.FLUID_MIN_VISIBLE_PRESSURE, main.FLUID_MIN_VISIBLE_PRESSURE):
		_fail(
			"test_layouts.gd's FLUID_MIN_VISIBLE_PRESSURE is %s and main.gd's is %s — "
			% [Layouts.FLUID_MIN_VISIBLE_PRESSURE, main.FLUID_MIN_VISIBLE_PRESSURE]
			+ "the copy has drifted, and the gallery is now checked against the wrong floor"
		)
		return

	# --- 5. six tints, all different, none of them grey -------------------
	var seen := {}
	for fluid in Layouts.FLUID_GALLERY_FLUIDS:
		if not main.FLUID_TINTS.has(fluid):
			_fail(
				"'%s' has no FLUID_TINTS entry — it renders in the neutral fallback, " % fluid
				+ "which is the silent-fallback failure issue #68 closed"
			)
			return
		var tint: Color = main.FLUID_TINTS[fluid]
		if tint.s < FLUID_TINT_MIN_SATURATION:
			_fail(
				"'%s' is tinted (%.3f, %.3f, %.3f), saturation %.3f — below the %.2f floor, " % [
					fluid, tint.r, tint.g, tint.b, tint.s, FLUID_TINT_MIN_SATURATION
				]
				+ "so it reads as an empty steel pipe rather than a full one (issue #100)"
			)
			return
		for other_id in seen:
			var other: Color = seen[other_id]
			var distance: float = (
				absf(tint.r - other.r) + absf(tint.g - other.g) + absf(tint.b - other.b)
			)
			if distance < FLUID_TINT_MIN_DISTANCE:
				_fail(
					"'%s' and '%s' are tinted the same liquid (distance %.3f) — the gallery "
					% [fluid, other_id, distance]
					+ "stands them in one frame, so two of its six would be one colour"
				)
				return
		seen[fluid] = tint

	print(
		"FLUID_GALLERY_OK: %d placements stand %d runs, each named by topology alone, each above "
		% [layout.size(), snapshots.size()]
		+ "main.gd's %s visible-pressure floor, each with a tint of its own (%s) — and the check "
		% [main.FLUID_MIN_VISIBLE_PRESSURE, ", ".join(Layouts.FLUID_GALLERY_FLUIDS)]
		+ "refuses an empty world, so that is a pass and not a default"
	)
	quit(0)
