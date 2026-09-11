# Headless proof of machine hover tooltips (docs/UI_UX.md §5: "hover
# tooltips for recipe/machine detail rather than dense always-on text"),
# including the FIRST_PERSON crosshair variant added after initial review —
# the mouse is captured/hidden there, so a screen-centre crosshair
# (_crosshair, drawn by _build_crosshair) stands in for the cursor.
#
# Run with:
#   godot --headless --path godot_project --script test_hover.gd
#
# Verifies, without a framebuffer:
#   * a screen position that lands exactly on a known machine's tile
#     resolves (via _tile_under_screen_pos) to that tile, and the tile ->
#     recipe lookup (_machine_recipe_at_tile) finds that stage's machine,
#   * a screen position clearly off in open floor (far outside the chain's
#     footprint) resolves to a tile with no machine on it,
#   * _machine_hover_text returns the live recipe/facing/progress text for a
#     hovered stage, and "" for a tile that isn't hovering anything (the
#     sentinel _update_hover_tooltip checks to decide whether to show the
#     tooltip at all),
#   * _hover_panel_state — the WIDGET half, added by the UI visualisation
#     pass — reports a progress meter and both buffers for a machine, a
#     level meter for a hopper, and neither for a belt or open floor,
#   * the SAME _tile_under_screen_pos math also resolves correctly through a
#     FIRST_PERSON perspective camera aimed at a known machine — proving the
#     crosshair-driven hover works, not just the isometric cursor-driven one,
#   * looking steeply upward in FIRST_PERSON (a ray pointing away from the
#     belt-surface plane) resolves to HOVER_NO_TILE rather than a phantom
#     tile behind the camera — the `t < 0` guard `_tile_under_screen_pos`
#     added specifically for this case,
#   * `_crosshair`'s visibility tracks `_view_mode` (shown in FIRST_PERSON,
#     hidden in ISOMETRIC).
#
# `_tile_under_screen_pos` calls `Camera3D.project_ray_origin`/
# `project_ray_normal`, which (like `unproject_position`) require the camera
# to actually be inside the scene tree — true during normal gameplay from
# `_ready()` onward, but NOT true yet at the point `main._ready()` returns
# when this test drives it directly the same way test_camera_nav.gd does,
# because a `--script`-mode SceneTree's `root` only finishes entering the
# tree on the first processed frame, one tick after `_initialize()` runs.
# So this test — uniquely among this project's headless Godot tests —
# processes one frame before touching anything that needs the ray-projection
# camera calls. `_tile_under_screen_pos` itself stays pure/Input-free either
# way; this is purely a scene-tree-bootstrap quirk of the test harness, not
# something a real running game ever has to work around (its `_camera` is
# already inside the tree by the time `_process` first runs).
#
# One consequence of that same bootstrap quirk, discovered while writing
# this test and worth flagging for whoever next touches this file: `root`
# entering the tree on the first processed frame (not synchronously inside
# `_initialize()`) means the engine's OWN automatic `_ready()` call for
# `main` — queued the moment `root.add_child(main)` runs — is still pending
# at that point, and only fires once frame processing begins. Every existing
# headless test in this project calls `main._ready()` manually right after
# `root.add_child(main)` (to get a working scene before any frame has
# processed), which means `_ready()` actually runs TWICE: once manually,
# once automatically-but-deferred — building two of everything (two
# Camera3Ds, two HUDs, etc.) under `main`, with `main`'s own fields (like
# `_camera`) left pointing at whichever build ran last. This has been
# harmless for every existing test, because they only ever read `main`'s
# fields (never a separately-captured local reference) and every rebuild is
# deterministic. It stops being harmless here specifically because the
# *second* (automatic) `_ready()` call happens to build its camera with a
# genuinely different `_camera_base_size` than the first (the viewport
# becomes queryable in between, so `_framing_size`'s aspect-ratio fallback
# no longer applies the second time) — so a `Camera3D` reference captured
# early, before that second build replaces `main._camera`, silently goes
# stale. This test avoids that entirely by never holding its own `camera`
# reference: every check below reads `main._camera` fresh, exactly like
# `_tile_under_screen_pos` itself does, so it is always looking at whichever
# camera instance is actually live. Not fixing the underlying double-`_ready`
# behavior here — it's a pre-existing harness quirk shared by every test in
# this file's family, out of this ticket's scope, and every other test
# happens not to care about it.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("HOVER_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


## `_process` (SceneTree's per-frame callback, distinct from a Node's) fires
## once the tree has actually processed a frame — see the module doc comment
## above for why that's the point project_ray_* calls become safe to use,
## and `main._camera` stable.
func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


func _run_checks() -> void:
	# Built HERE, not in `_initialize`. `_ready` runs twice under this harness
	# (see the header) and each run creates a fresh `SimBridge` — i.e. a fresh,
	# EMPTY world — so anything built before the second run is wiped. That was
	# invisible while every world arrived pre-built; since the "Real Placement"
	# epic it is the difference between a factory and bare floor.
	Layouts.build_demo_line(main.sim)
	main._rebuild_world()

	var camera: Camera3D = main._camera
	if camera == null:
		_fail("main._camera was not set by _build_camera()")
		return
	if main._placements.is_empty():
		_fail("the demo line produced no placements — nothing to hover")
		return

	# --- A screen position exactly on the crusher's tile resolves to it -----
	# Hover is keyed on the placement's ORIGIN TILE now, not on a recipe id:
	# the player can place several machines running the same recipe, so
	# "which machine is this" is answered by which footprint the cursor is
	# over. `_hover_text_at_tile` is the one entry point for every kind.
	var machine_tile := Vector2i(3, 2)
	var world_point: Vector3 = main._tile_origin(machine_tile.x, machine_tile.y, main.BELT_SURFACE_Y)
	var screen_pos: Vector2 = camera.unproject_position(world_point)

	var resolved_tile: Vector2i = main._tile_under_screen_pos(screen_pos)
	if resolved_tile != machine_tile:
		_fail("screen pos over the crusher's tile %s resolved to %s instead" % [
			str(machine_tile), str(resolved_tile),
		])
		return

	var placement: Dictionary = main._placement_at_tile(resolved_tile)
	if String(placement.get("kind", "")) != "crusher":
		_fail("tile %s should resolve to the crusher, got %s" % [str(resolved_tile), str(placement)])
		return

	# The tooltip names the recipe the way every other panel in the game names
	# an item — "Tomato Paste", not the wire format's `tomato_paste` — since
	# the UI visualisation pass (docs/UI_UX.md §5).
	var hover_text: String = main._hover_text_at_tile(resolved_tile)
	if hover_text == "" or not hover_text.begins_with("Tomato Paste"):
		_fail("hover text over the crusher should start with its recipe's display name, got: %s" % hover_text)
		return
	# Which way it faces is a fact only placement created, and one the player
	# can change — so the tooltip has to carry it.
	if not hover_text.contains("facing west"):
		_fail("machine hover text should report the facing the player gave it, got: %s" % hover_text)
		return

	# --- The WIDGET half: buffers and the progress bar -----------------------
	# The buffers left the sentence and became icon chips under a bar, so what
	# is asserted is the pure state `_apply_hover_panel_state` draws from, not
	# a substring of the text. A machine ALWAYS reports `has_buffers` (an
	# empty crusher and a pipe are different facts, and only the flag says
	# which), and always offers a meter — an idle machine's reads zero rather
	# than vanishing.
	var machine_state: Dictionary = main._hover_panel_state(resolved_tile)
	if not bool(machine_state.get("has_buffers", false)):
		_fail("a machine's hover state should report buffers, got: %s" % str(machine_state))
		return
	if not (machine_state.get("inputs") is Array) or not (machine_state.get("outputs") is Array):
		_fail("a machine's hover state should carry both buffers as arrays, got: %s" % str(machine_state))
		return
	var machine_meter: Dictionary = machine_state.get("meter", {})
	if machine_meter.is_empty():
		_fail("a machine's hover state should always carry a progress meter, got: %s" % str(machine_state))
		return
	var machine_fraction := float(machine_meter.get("fraction", -1.0))
	if machine_fraction < 0.0 or machine_fraction > 1.0:
		_fail("a machine's progress fraction should be a 0..1 level, got: %s" % str(machine_meter))
		return

	# A hopper is metered against its real capacity; a belt has no ceiling to
	# measure and deliberately gets no bar and no buffer rows at all.
	var hopper_state: Dictionary = main._hover_panel_state(Vector2i(-1, 0))
	if hopper_state.get("meter", {}).is_empty():
		_fail("a hopper's hover state should meter its level against capacity, got: %s" % str(hopper_state))
		return
	if bool(hopper_state.get("has_buffers", false)):
		_fail("a hopper has no machine buffers to show, got: %s" % str(hopper_state))
		return
	var belt_state: Dictionary = main._hover_panel_state(Vector2i(1, 0))
	if not belt_state.get("meter", {}).is_empty() or bool(belt_state.get("has_buffers", false)):
		_fail("a belt has neither a metered quantity nor buffers, got: %s" % str(belt_state))
		return
	if not main._hover_panel_state(Vector2i(0, 20)).get("meter", {}).is_empty():
		_fail("open floor should produce no meter at all")
		return

	# --- Every other placed kind answers through the same one entry point ---
	# A pipe's tooltip leads with the fluid it carries (or "empty run"), so
	# these are `contains` rather than `begins_with` — the point is that each
	# kind identifies itself, not that they all use one sentence shape.
	var kinds := {
		Vector2i(1, 0): "Belt",
		Vector2i(-1, 0): "Hopper",
		Vector2i(5, 2): "Pipe",
	}
	for tile in kinds.keys():
		var text: String = main._hover_text_at_tile(tile)
		if not text.contains(String(kinds[tile])):
			_fail("hover over %s should describe a %s, got: %s" % [str(tile), str(kinds[tile]), text])
			return

	# A ROTATED tile still identifies itself — the belt at (3, 1) faces north
	# and hands into the crusher's flank, and hover must say its facing
	# whatever shape `_belt_shape_for` gives it.
	var north_belt: String = main._hover_text_at_tile(Vector2i(3, 1))
	if not north_belt.contains("north"):
		_fail("hover over the north-facing belt should say so, got: %s" % north_belt)
		return

	# --- A screen position clearly off in open floor finds nothing ---------
	var open_floor_world: Vector3 = main._tile_origin(0, 20, main.BELT_SURFACE_Y)
	var open_floor_screen: Vector2 = camera.unproject_position(open_floor_world)
	var open_floor_tile: Vector2i = main._tile_under_screen_pos(open_floor_screen)
	if open_floor_tile != Vector2i(0, 20):
		_fail("screen pos over open floor tile (0, 20) resolved to %s instead" % str(open_floor_tile))
		return
	if main._hover_text_at_tile(open_floor_tile) != "":
		_fail("open floor should produce no tooltip text at all")
		return

	# --- FIRST_PERSON: the crosshair (screen centre) resolves to a machine --
	# it's aimed at, exactly like the isometric cursor does above.
	main._set_view_mode(main.ViewMode.FIRST_PERSON)
	if main._crosshair == null or not main._crosshair.visible:
		_fail("crosshair should be visible in FIRST_PERSON mode")
		return

	var eye: Vector3 = world_point + Vector3(main.TILE * 2.0, main.TILE * 3.0, main.TILE * 4.0)
	main._fp_position = eye
	var to_target: Vector3 = (world_point - eye).normalized()
	main._fp_yaw = atan2(-to_target.x, -to_target.z)
	main._fp_pitch = clampf(asin(clampf(to_target.y, -1.0, 1.0)), -main.FP_PITCH_LIMIT, main.FP_PITCH_LIMIT)
	main._update_fp_camera_transform()

	# A point exactly on the camera's forward axis always projects to the
	# viewport's centre regardless of the viewport's actual (possibly
	# headlessly-zero) size, unlike computing "screen centre" from
	# get_visible_rect().size directly — sidesteps that flakiness entirely.
	var crosshair_screen_pos: Vector2 = camera.unproject_position(world_point)
	var fp_resolved_tile: Vector2i = main._tile_under_screen_pos(crosshair_screen_pos)
	if fp_resolved_tile != machine_tile:
		_fail("FIRST_PERSON crosshair aimed at the crusher's tile %s resolved to %s instead" % [
			str(machine_tile), str(fp_resolved_tile),
		])
		return

	# --- FIRST_PERSON: looking steeply up finds nothing (the t < 0 guard) ---
	main._fp_pitch = main.FP_PITCH_LIMIT
	main._update_fp_camera_transform()
	var ahead: Vector3 = eye + (-main._camera.transform.basis.z) * (main.TILE * 10.0)
	var sky_screen_pos: Vector2 = camera.unproject_position(ahead)
	var sky_tile: Vector2i = main._tile_under_screen_pos(sky_screen_pos)
	if sky_tile != main.HOVER_NO_TILE:
		_fail("looking steeply up in FIRST_PERSON should find no tile (t < 0), got %s" % str(sky_tile))
		return

	main._set_view_mode(main.ViewMode.ISOMETRIC)
	if main._crosshair.visible:
		_fail("crosshair should be hidden in ISOMETRIC mode")
		return

	print("HOVER_OK: screen-to-tile resolution finds the right machine in both view modes, every placed kind answers through one entry point and names itself the way the rest of the UI does, a machine reports buffers and a progress meter while a belt reports neither, a rotated tile reports its facing, open floor finds none, looking up in first-person finds no tile, crosshair visibility tracks view mode")
	quit(0)
