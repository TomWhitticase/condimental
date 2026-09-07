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
#   * _machine_hover_text returns the live recipe/progress/input/output text
#     for a hovered stage, and "" for a recipe id that isn't hovering
#     anything (the sentinel _update_hover_tooltip checks to decide whether
#     to show the tooltip at all),
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
	var camera: Camera3D = main._camera
	if camera == null:
		_fail("main._camera was not set by _build_camera()")
		return
	if main._stage_order.is_empty():
		_fail("chain layout produced no stages — nothing to hover")
		return

	# --- A screen position exactly on a known machine's tile resolves to it --
	var recipe_id: String = main._stage_order[0]
	var machine_tile: Vector2i = main._machine_tiles[recipe_id]
	var world_point: Vector3 = main._tile_origin(machine_tile.x, machine_tile.y, main.BELT_SURFACE_Y)
	var screen_pos: Vector2 = camera.unproject_position(world_point)

	var resolved_tile: Vector2i = main._tile_under_screen_pos(screen_pos)
	if resolved_tile != machine_tile:
		_fail("screen pos over %s's machine tile %s resolved to %s instead" % [
			recipe_id, str(machine_tile), str(resolved_tile),
		])
		return

	var resolved_recipe: String = main._machine_recipe_at_tile(resolved_tile)
	if resolved_recipe != recipe_id:
		_fail("tile %s should resolve to recipe '%s', got '%s'" % [
			str(resolved_tile), recipe_id, resolved_recipe,
		])
		return

	var hover_text: String = main._machine_hover_text(resolved_recipe)
	if hover_text == "" or not hover_text.begins_with(recipe_id):
		_fail("_machine_hover_text('%s') should start with the recipe id, got: %s" % [recipe_id, hover_text])
		return
	if not hover_text.contains("in:") or not hover_text.contains("out:"):
		_fail("_machine_hover_text should include input/output buffer contents, got: %s" % hover_text)
		return

	# --- A screen position clearly off in open floor finds no machine -------
	# Far outside the chain's footprint (FLOOR_GROUND_MARGIN_Z alone is 24
	# tiles) but still on the floor plane, so this is a real "open floor, no
	# machine" case, not an off-screen/degenerate ray.
	var open_floor_world: Vector3 = main._tile_origin(0, 20, main.BELT_SURFACE_Y)
	var open_floor_screen: Vector2 = camera.unproject_position(open_floor_world)
	var open_floor_tile: Vector2i = main._tile_under_screen_pos(open_floor_screen)
	if open_floor_tile != Vector2i(0, 20):
		_fail("screen pos over open floor tile (0, 20) resolved to %s instead" % str(open_floor_tile))
		return
	var no_recipe: String = main._machine_recipe_at_tile(open_floor_tile)
	if no_recipe != "":
		_fail("open floor tile (0, 20) should have no machine, found '%s'" % no_recipe)
		return
	if main._machine_hover_text(no_recipe) != "":
		_fail("_machine_hover_text('') (no recipe) should return empty text")
		return

	# --- FIRST_PERSON: the crosshair (screen centre) resolves to a machine --
	# it's aimed at, exactly like the isometric cursor does above. Aim by
	# setting _fp_position/_fp_yaw/_fp_pitch directly (same fields
	# _apply_look/_fp_camera_move drive) and deriving yaw/pitch from the
	# desired look direction, then read back via _update_fp_camera_transform
	# — no live Input needed, matching _tile_under_screen_pos's own
	# Input-free contract.
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
		_fail("FIRST_PERSON crosshair aimed at %s's machine tile %s resolved to %s instead" % [
			recipe_id, str(machine_tile), str(fp_resolved_tile),
		])
		return

	# --- FIRST_PERSON: looking steeply up finds nothing (the t < 0 guard) ---
	# Same eye position, pitched up toward the sky instead of at the machine.
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

	print("HOVER_OK: screen-to-tile resolution finds the right machine in both view modes, open floor finds none, looking up in first-person finds no tile, hover text carries live recipe/progress/buffer data, crosshair visibility tracks view mode")
	quit(0)
