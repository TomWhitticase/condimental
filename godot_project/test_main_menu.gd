# Headless proof of the MAIN MENU (the "Save/load + a real main menu" epic,
# ticket 2) — the first screen the game opens on, and the handoff from it
# into `main.tscn`.
#
# Run with:
#   godot --headless --path godot_project --script test_main_menu.gd
#
# `test_save_load.gd` already covers the `SimBridge` half (`save_ron` /
# `load_ron` / `new_game` across the real gdext boundary). What is new here
# is everything Godot owns because `sim_core` deliberately does no file I/O:
# the saves directory, the listing, the slot a player picks out of it, and
# the `GameSession` autoload that carries the choice across the scene change.
#
# Verifies:
#   * `menu.tscn` builds the three buttons the ticket names — New Game, Load
#     Game, Quit — plus a Back button in the load view,
#   * a real save file written to `user://saves/` appears in the list, with
#     its slot name and its last-modified time, newest first, and the empty
#     state hides itself when there is something to show,
#   * choosing that slot arms the session, and a FRESH `main.tscn` built from
#     that armed session comes up as the saved factory — same tick count,
#     same snapshots — rather than as a new game,
#   * a CORRUPT file is refused on the menu: the error `load_ron` produced is
#     shown, the session is left alone, and no scene change is attempted,
#   * New Game arms a new game, and the world that comes up is tick 0 and
#     empty,
#   * the autosave slot is listed and labelled as the autosave,
#   * the title screen behind it is the real factory: a lit backdrop in the
#     same hall, on the game's own floor, with a drifting perspective camera
#     and a splash line — and the save list still fits under it.
#
# It writes real files under `user://saves/`, because that is the thing being
# tested, and removes exactly the ones it wrote on the way out. It never
# touches `autosave.ron`.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const GameSaves = preload("res://game_session.gd")
const FactoryEnvironment = preload("res://factory_environment.gd")

## Distinctive enough that they cannot collide with a real save, and both
## removed at the end.
const FIXTURE_SLOT := "zz test fixture"
const CORRUPT_SLOT := "zz test corrupt"

var _cleanup_paths: Array = []


func _find_all(node: Node, type_name: String, out: Array) -> Array:
	if node.is_class(type_name):
		out.append(node)
	for child in node.get_children():
		_find_all(child, type_name, out)
	return out


func _fail(msg: String) -> void:
	_cleanup()
	printerr("MAIN_MENU_FAIL: %s" % msg)
	quit(1)


func _cleanup() -> void:
	for path in _cleanup_paths:
		if FileAccess.file_exists(path):
			DirAccess.remove_absolute(path)


## Every snapshot the scene redraws from, as one string — the same
## fingerprint `test_save_load.gd` compares two worlds with.
func _fingerprint(sim) -> String:
	var parts := PackedStringArray()
	parts.append("tick=%d" % sim.current_tick())
	for call_name in [
		"world_layout_json", "hopper_snapshots_json", "fluid_tank_snapshots_json",
		"fluid_snapshots_json", "dock_snapshots_json", "economy_snapshot_json",
		"shop_snapshot_json", "ingredients_snapshot_json",
	]:
		parts.append("%s=%s" % [call_name, sim.call(call_name)])
	return "\n".join(parts)


func _new_main() -> Node:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	return main


func _row_index(menu, slot: String) -> int:
	for i in range(menu._save_rows.size()):
		if String(menu._save_rows[i]["slot"]) == slot:
			return i
	return -1


func _initialize() -> void:
	var session: Node = root.get_node_or_null("GameSession")
	if session == null:
		_fail("the GameSession autoload is not registered — check project.godot's [autoload]")
		return
	session.arm_new_game()

	# --- A real save file, written the way the pause menu writes one. ------
	var builder := _new_main()
	Layouts.build_machines_fixture(builder.sim)
	Layouts.stock_larder(builder.sim)
	for i in range(300):
		builder.sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
		builder.sim.tick()
	var saved_tick: int = builder.sim.current_tick()
	var saved_fingerprint := _fingerprint(builder.sim)
	var save_text: String = builder.sim.save_ron()

	var fixture_path: String = GameSaves.slot_path(FIXTURE_SLOT)
	var corrupt_path: String = GameSaves.slot_path(CORRUPT_SLOT)
	_cleanup_paths = [fixture_path, corrupt_path]

	var write_error: String = GameSaves.write_slot(FIXTURE_SLOT, save_text)
	if write_error != "":
		_fail("could not write the fixture save: %s" % write_error)
		return
	if not FileAccess.file_exists(fixture_path):
		_fail("write_slot reported success but %s is not there" % fixture_path)
		return
	# Written second so it is also the NEWEST, which is what proves the
	# listing's ordering rather than assuming the directory hands them back
	# in a helpful order.
	var corrupt_error: String = GameSaves.write_slot(CORRUPT_SLOT, "this is not a save file")
	if corrupt_error != "":
		_fail("could not write the corrupt fixture: %s" % corrupt_error)
		return

	# --- The menu itself. --------------------------------------------------
	var menu = load("res://menu.tscn").instantiate()
	root.add_child(menu)
	menu._ready()

	for button_name in ["NewGameButton", "LoadGameButton", "QuitButton"]:
		var button: Button = menu.find_child(button_name, true, false)
		if button == null:
			_fail("menu.tscn has no %s — the three named buttons are the menu" % button_name)
			return
		if button.text.strip_edges() == "":
			_fail("%s has no label on it" % button_name)
			return
	if menu.find_child("BackButton", true, false) == null:
		_fail("the load view needs a way back to the three buttons")
		return
	if menu.find_child("Title", true, false) == null:
		_fail("the menu needs a title — it is the first thing the player sees")
		return

	# --- The title screen: a real factory behind the menu ------------------
	# The backdrop is a hand-composed 3D scene (title_scene.gd), not a flat
	# plate, and it is built from the SAME voxel assets, floor helpers and
	# hall environment the game uses so the two cannot drift. None of that is
	# visible to any other check: the menu would still open, still list saves
	# and still start a game with an empty grey rectangle behind it.
	var backdrop: Node3D = menu._title_scene
	if backdrop == null:
		_fail("the menu has no TitleScene backdrop — the title screen is the factory itself")
		return
	# As with `main` above: `_ready` only fires on the engine's own next-frame
	# processing, which this headless script-mode SceneTree never runs.
	backdrop._ready()
	if backdrop.name != "TitleScene":
		_fail("the backdrop should name itself, found '%s'" % backdrop.name)
		return
	var title_cameras := _find_all(backdrop, "Camera3D", [])
	if title_cameras.size() != 1:
		_fail("the title backdrop needs exactly one camera, found %d" % title_cameras.size())
		return
	var title_camera: Camera3D = title_cameras[0]
	if title_camera.projection != Camera3D.PROJECTION_PERSPECTIVE:
		_fail("the title camera is a photograph of the hall, so it is perspective, not the game's orthographic view (see title_scene.gd's note)")
		return
	var title_env := _find_all(backdrop, "WorldEnvironment", [])
	if title_env.is_empty() or title_env[0].environment == null:
		_fail("the title backdrop has no WorldEnvironment — it must stand in the same lit hall the game does")
		return
	var title_sky: Environment = title_env[0].environment
	if title_sky.background_mode != Environment.BG_SKY or title_sky.sky == null \
			or not (title_sky.sky.sky_material is ShaderMaterial):
		_fail("the title backdrop must use the factory-interior sky, the same one main.gd builds (factory_environment.gd is shared for exactly this reason)")
		return
	# ...but with clear air. Haze is atmosphere over a play session; in one
	# still frame under a wordmark it is a grey wash sitting on the picture.
	# Asserted because it is a single shader parameter that anyone re-tuning
	# the hall would naturally push back through both callers.
	var title_haze = (title_sky.sky.sky_material as ShaderMaterial).get_shader_parameter("haze_density")
	if not is_equal_approx(float(title_haze), FactoryEnvironment.TITLE_HAZE_DENSITY):
		_fail("the title screen's air must be clear (haze_density %f), was %f — the poster wants the roof and the far wall crisp" % [FactoryEnvironment.TITLE_HAZE_DENSITY, float(title_haze)])
		return
	if backdrop.get_node_or_null("Floor") == null:
		_fail("the title backdrop has no Floor — the poster stands on the game's own floor tiles")
		return
	var plant := backdrop.get_node_or_null("Plant")
	if plant == null or plant.get_child_count() < 10:
		_fail("the title backdrop should show the whole chain; found %d models" % (0 if plant == null else plant.get_child_count()))
		return
	if _find_all(backdrop.get_node("Floor"), "MultiMeshInstance3D", []).is_empty():
		_fail("the title floor must be instanced the way the game's is")
		return
	# The camera drifts, which is the difference between a title screen and a
	# paused game. Wall time, deliberately: there is no simulation here to
	# take a tick from (title_scene.gd's `_process`).
	var drift_from := title_camera.position.x
	backdrop._process(6.0)
	if is_equal_approx(title_camera.position.x, drift_from):
		_fail("the title camera must drift — a still frame reads as a paused game")
		return

	# NOTHING IS WASHED OVER THE HALL. The menu used to lay a dark gradient
	# band across the middle of the screen behind its text; the owner did not
	# want it, and the whole point of standing a real factory back there is
	# that it is seen. This is a purely rendered symptom — put the scrim back
	# and every other check here still passes — so it is asserted: no
	# full-screen tinted overlay may sit between the backdrop and the column.
	# Instead the standalone labels carry their own outline (menu.gd's
	# `TITLE_OUTLINE`/`STRAP_OUTLINE`), which is checked below.
	for wash in menu.get_children():
		if not (wash is ColorRect or wash is TextureRect or wash is Panel):
			continue
		if wash.anchor_right < 1.0 or wash.anchor_bottom < 1.0:
			continue
		_fail("'%s' covers the whole menu — the hall behind it is the screen, not a backing for text" % wash.name)
		return
	# The wordmark is the lit marquee sign hanging in the hall, a voxel model
	# from the asset pipeline (`title_sign`), not a Label. A missing model
	# builds an empty node and every other check here stays green, so its
	# meshes are counted — the board, the letters and a bulb frame.
	var sign_rig: Node3D = backdrop.get_node_or_null("TitleCamera/TitleSign")
	if sign_rig == null:
		_fail("the title sign must hang from the title camera, so it stays framed while the hall drifts")
		return
	if _find_all(sign_rig, "MeshInstance3D", []).size() < 3:
		_fail("the title sign has no meshes — is `title_sign` staged? (`cargo xtask`)")
		return
	# It hangs in the slot the column keeps for it: framing it into a rect
	# moves it, and the settings page stands it down with the slot.
	backdrop.frame_sign(Rect2(100, 40, 800, 150), Vector2(1280, 720))
	var framed_at := sign_rig.position
	backdrop.frame_sign(Rect2(300, 300, 400, 75), Vector2(1280, 720))
	if framed_at.is_equal_approx(sign_rig.position) or sign_rig.scale.x >= 1.0e3:
		_fail("frame_sign must follow the rect it is handed")
		return

	# The poster RUNS: belts march, product slides, and the intro can be
	# skipped to its settled end. Wall time, so driven here by hand.
	if backdrop._belt_sets.is_empty() or backdrop._motion_sets.is_empty() or backdrop._flow_items.is_empty():
		_fail("the title line must have belt frames, machine motion and product on it")
		return
	# Any item, not one: an item off the end of its run is parked, hidden,
	# inside the next machine until its slot comes round.
	var items_from: Array = []
	for flow_item in backdrop._flow_items:
		items_from.append((flow_item["node"] as Node3D).position.x)
	backdrop._process(0.4)
	var moved := false
	for k in range(items_from.size()):
		if not is_equal_approx((backdrop._flow_items[k]["node"] as Node3D).position.x, items_from[k]):
			moved = true
	if not moved:
		_fail("product on the title line must move along the belt")
		return
	backdrop.finish_intro()
	if backdrop.intro_time < backdrop.INTRO_LENGTH:
		_fail("finish_intro must leave the intro at its settled end")
		return

	# The splash line, the one piece of pure decoration on the screen.
	var splash: Label = menu.find_child("SplashText", true, false)
	if splash == null or splash.text.strip_edges() == "":
		_fail("the splash line is missing or blank")
		return
	if is_zero_approx(splash.rotation):
		_fail("the splash line is tilted — that tilt is the whole Minecraft reference")
		return

	# The load view is closed to begin with, and the three buttons are what
	# is showing.
	if menu._load_open:
		_fail("the menu must open on its buttons, not on the save list")
		return
	if not menu._button_box.visible or menu._load_box.visible:
		_fail("the load view must start hidden")
		return

	# --- Load Game lists what is on disk, newest first. --------------------
	menu._on_load_pressed()
	if not menu._load_open or not menu._load_box.visible or menu._button_box.visible:
		_fail("Load Game must swap the buttons for the save list")
		return
	if menu._save_rows.is_empty():
		_fail("the save list is empty even though two files were just written")
		return
	if menu._empty_label.visible:
		_fail("the empty-state line must hide itself once there are saves")
		return
	# The list is capped by what the WINDOW has left after the wordmark, the
	# plate and BACK — not by a flat number. A flat one put BACK off the
	# bottom edge once the wordmark moved out of the panel, and no headless
	# check noticed (menu.gd's `_list_height_budget`).
	var save_scroll: ScrollContainer = menu.find_child("SaveScroll", true, false)
	if save_scroll == null:
		_fail("the save list has no scroll container")
		return
	var list_ceiling := maxf(menu._list_height_budget(), menu.SAVE_LIST_MIN_HEIGHT)
	if save_scroll.custom_minimum_size.y > list_ceiling + 1.0:
		_fail("the save list is %.0fpx tall in a window that leaves it %.0fpx — BACK would fall off the bottom" % [
			save_scroll.custom_minimum_size.y, list_ceiling,
		])
		return
	for i in range(1, menu._save_rows.size()):
		if int(menu._save_rows[i - 1]["modified_unix"]) < int(menu._save_rows[i]["modified_unix"]):
			_fail("the save list must be newest first")
			return

	var fixture_index := _row_index(menu, FIXTURE_SLOT)
	var corrupt_index := _row_index(menu, CORRUPT_SLOT)
	if fixture_index < 0 or corrupt_index < 0:
		_fail("both written saves must appear in the list (fixture=%d corrupt=%d)" % [
			fixture_index, corrupt_index,
		])
		return
	var fixture_row: Button = menu.find_child("SaveRow_%d" % fixture_index, true, false)
	if fixture_row == null:
		_fail("no button was built for the fixture save's row")
		return
	if not fixture_row.text.contains(FIXTURE_SLOT):
		_fail("a save's row must name its slot, got '%s'" % fixture_row.text)
		return
	if not fixture_row.text.contains(String(menu._save_rows[fixture_index]["modified"])):
		_fail("a save's row must say when it was written, got '%s'" % fixture_row.text)
		return
	# An autosave, if this machine has one, is labelled rather than hidden.
	for row in menu._save_rows:
		if bool(row["autosave"]):
			var index := _row_index(menu, String(row["slot"]))
			var node: Button = menu.find_child("SaveRow_%d" % index, true, false)
			if node == null or not node.text.contains("autosave"):
				_fail("the autosave slot must be labelled as the autosave in the list")
				return

	# --- A CORRUPT save is refused HERE, with the reason, and nothing moves.
	var before_action := String(session.pending_action)
	var before_path := String(session.pending_path)
	menu._on_save_row_pressed(corrupt_index)
	if not menu._error_label.visible or menu._error_label.text.strip_edges() == "":
		_fail("a corrupt save must put load_ron's reason on screen")
		return
	if not menu._error_label.text.contains(CORRUPT_SLOT):
		_fail("the error must name the save it is about, got '%s'" % menu._error_label.text)
		return
	if String(session.pending_action) != before_action \
			or String(session.pending_path) != before_path:
		_fail("a refused save must leave the session exactly as it was")
		return
	if not menu._load_open:
		_fail("a refused save must leave the load list up, not walk away from it")
		return
	if current_scene != null:
		_fail("a refused save must not have changed scene")
		return

	# --- The real one arms the session, and the world comes back. ----------
	var arm_error: String = menu._arm_slot(fixture_index)
	if arm_error != "":
		_fail("the fixture save must load, got '%s'" % arm_error)
		return
	if String(session.pending_action) != "load":
		_fail("choosing a save must arm a load, got '%s'" % session.pending_action)
		return
	if String(session.pending_path) != fixture_path:
		_fail("armed the wrong file: %s" % session.pending_path)
		return
	if String(session.current_slot) != FIXTURE_SLOT:
		_fail("the loaded slot's name must survive into the game, for the save field")
		return

	var loaded := _new_main()
	if loaded.sim.current_tick() != saved_tick:
		_fail("a game started from the menu's Load must be at tick %d, got %d" % [
			saved_tick, loaded.sim.current_tick(),
		])
		return
	if _fingerprint(loaded.sim) != saved_fingerprint:
		_fail("every snapshot must come back as it was when the save was written")
		return
	# The slot the pause menu will prefill with is the one that was loaded.
	if String(loaded._save_slot) != FIXTURE_SLOT:
		_fail("the loaded game must remember its slot name, got '%s'" % loaded._save_slot)
		return

	# --- ...and the SCENE is the loaded factory on its first frame. --------
	# `_ready` runs the whole rebuild plus every per-frame updater against
	# the loaded world, so nothing draws a new game's empty floor (or a new
	# game's inventory counts) for a frame before catching up.
	var world_root: Node = loaded.get_node_or_null("World")
	if world_root == null:
		_fail("a loaded game has no World subtree — the scene never rebuilt")
		return
	var placements: Array = JSON.parse_string(loaded.sim.world_layout_json())
	if placements.is_empty():
		_fail("the fixture save should have placements in it")
		return
	for placement in placements:
		var pair: Array = placement["tile"]
		var kind := String(placement["kind"])
		var origin := Vector2i(int(pair[0]), int(pair[1]))
		var key := "%s_%d,%d" % [kind, origin.x, origin.y]
		# A belt or pipe is a MultiMesh instance rather than a node (the
		# pipes & conveyors overhaul's ticket 4), so it is looked up the
		# way the scene records it.
		if loaded._is_instanced_kind(kind):
			if loaded._tile_instance_at(origin).is_empty():
				_fail("the loaded world's %s was not drawn" % key)
				return
			continue
		if world_root.get_node_or_null(key) == null:
			_fail("the loaded world's %s was not drawn" % key)
			return
	# Inventory counts too: these come off a different snapshot than the
	# layout does, and a new game's would be visibly wrong.
	var loaded_shop: Dictionary = JSON.parse_string(loaded.sim.shop_snapshot_json())
	for entry in loaded_shop.get("entries", []):
		var kind := String(entry["kind"])
		if int(loaded._owned_counts.get(kind, 0)) != int(entry["owned"]):
			_fail("the hotbar's owned count for %s does not match the loaded world" % kind)
			return

	# --- New Game arms an empty world. -------------------------------------
	menu._on_new_game_pressed()
	if String(session.pending_action) != "new_game" or String(session.pending_path) != "":
		_fail("New Game must arm a fresh start, got '%s'/'%s'" % [
			session.pending_action, session.pending_path,
		])
		return
	var fresh := _new_main()
	if fresh.sim.current_tick() != 0:
		_fail("New Game must start at tick 0, got %d" % fresh.sim.current_tick())
		return
	var layout: Array = JSON.parse_string(fresh.sim.world_layout_json())
	if not layout.is_empty():
		_fail("New Game must start on an empty floor, got %d placement(s)" % layout.size())
		return
	if String(fresh._save_slot) != fresh.DEFAULT_SLOT:
		_fail("a new game must offer its default slot name, got '%s'" % fresh._save_slot)
		return

	_cleanup()
	print("MAIN_MENU_OK: three buttons, a save list newest-first with its slot and time,")
	print("              a chosen save coming back as the same factory, a corrupt one")
	print("              refused with load_ron's own reason and nothing moved, and New")
	print("              Game arming a tick-0 empty world, behind a drifting factory")
	quit(0)
