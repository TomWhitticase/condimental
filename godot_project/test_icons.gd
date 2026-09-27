# Headless proof of the icon-loading helper (UI_UX.md §5: icons are small
# isometric renders of the real voxel assets, not hand-drawn 2D art) —
# ticket B of the diegetic hover-info/clipboard-UI epic.
#
# Run with:
#   godot --headless --path godot_project --script test_icons.gd
#
# Verifies, without a framebuffer:
#   * a known item id (data/recipes.ron's final stage, also the hardcoded
#     contract's target item — sim_core/src/sim/economy.rs's CONTRACT_ITEM)
#     resolves to a real, non-empty Texture2D loaded from the generated
#     icon PNG — not the 8x8 placeholder fallback,
#   * the same id returns the exact same Texture2D instance on a second
#     call, proving the result is cached rather than re-loaded/re-rendered
#     every time,
#   * an unknown item id still returns a non-null Texture2D (the documented
#     placeholder fallback) instead of crashing or returning null,
#   * project.godot's config/icon (the window/dock icon) loads and is at
#     least 256px, because a bad path there is silent: the game boots with
#     Godot's default icon and logs nothing.
extends SceneTree


func _fail(msg: String) -> void:
	printerr("ICONS_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()

	var known_id := "boxed_ketchup"
	var icon: Texture2D = main._icon_for(known_id)
	if icon == null:
		_fail("_icon_for('%s') returned null" % known_id)
		return
	if icon.get_width() < 16 or icon.get_height() < 16:
		_fail("_icon_for('%s') looks like the placeholder fallback (%dx%d) — expected the real generated icon" % [
			known_id, icon.get_width(), icon.get_height(),
		])
		return

	var icon_again: Texture2D = main._icon_for(known_id)
	if icon_again != icon:
		_fail("_icon_for('%s') should return the cached instance on a second call" % known_id)
		return

	var unknown_icon: Texture2D = main._icon_for("not_a_real_item_id")
	if unknown_icon == null:
		_fail("_icon_for on an unknown id should fall back, not return null")
		return
	if unknown_icon.get_width() <= 0 or unknown_icon.get_height() <= 0:
		_fail("the fallback texture should still have real pixel dimensions")
		return

	var app_icon_path: String = ProjectSettings.get_setting("application/config/icon", "")
	if app_icon_path == "" or not ResourceLoader.exists(app_icon_path):
		_fail("application/config/icon ('%s') does not name a loadable file — the window would show Godot's default icon" % app_icon_path)
		return
	var app_icon: Texture2D = load(app_icon_path)
	if app_icon == null or app_icon.get_width() < 256 or app_icon.get_height() < 256:
		_fail("application/config/icon ('%s') should be a texture of at least 256x256" % app_icon_path)
		return

	print("ICONS_OK: known item id resolves to a cached, real generated icon; unknown id falls back to a non-null placeholder without crashing; the app icon loads at %dx%d" % [app_icon.get_width(), app_icon.get_height()])
	quit(0)
