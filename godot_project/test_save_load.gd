# Headless proof of save/load across the real `gdext` boundary (the
# "Save/load + a real main menu" epic, ticket 1).
#
# Run with:
#   godot --headless --path godot_project --script test_save_load.gd
#
# `sim_core`'s own `cargo test` suite already proves the hard part in pure
# Rust: that a saved-and-reloaded factory is running-state-identical, tick for
# tick, for longer than a contract cadence (`sim_core/tests/save_load.rs`).
# What this script exercises is the part cargo test cannot — the three new
# `SimBridge` methods as GDScript actually calls them, with a whole RON save
# crossing the boundary as a `String` in both directions.
#
# Verifies:
#   * `save_ron()` returns real save text from a factory built over this same
#     wire, and calling it twice returns the identical string,
#   * `load_ron()` puts an earlier moment back: the world is ticked PAST the
#     save, the save is loaded, and the tick count and the snapshots the
#     scene renders from all go back to where they were,
#   * a loaded world reports every chunk it occupies as dirty, which is what
#     lets a chunk-driven renderer redraw all of it,
#   * `load_ron("garbage")` returns a NON-EMPTY message (not a bool, and not
#     silence — DEVLOG 2026-09-09's "say why instead of doing nothing") and
#     leaves the live world completely untouched,
#   * `new_game()` empties the world back to a fresh start.
#
# File I/O is deliberately absent here as it is everywhere in this crate: Rust
# hands back a string and Godot decides what file it becomes, because
# `user://` is Godot's. That split is the ticket's own boundary decision and
# the reason this test can run without touching a disk at all.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")


func _fail(msg: String) -> void:
	printerr("SAVE_LOAD_FAIL: %s" % msg)
	quit(1)


## Every snapshot the scene redraws from, as one string — the same
## "if these match, presentation cannot tell the two apart" fingerprint
## `sim_core/tests/save_load.rs` compares worlds with.
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


func _initialize() -> void:
	var sim = SimBridge.new()

	# --- Build a real factory over the wire and run it into mid-flight. -----
	Layouts.build_machines_fixture(sim)
	Layouts.stock_larder(sim)
	for i in range(400):
		sim.issue_command_json(JSON.stringify({"type": "fill_hopper", "amount": 64}))
		sim.tick()

	var saved_tick: int = sim.current_tick()
	var save_text: String = sim.save_ron()
	if save_text.length() < 100 or not save_text.contains("schema_version"):
		_fail("save_ron() must return real save text, got %d chars" % save_text.length())
		return
	if save_text != sim.save_ron():
		_fail("two saves of one unchanged world must be identical")
		return
	var saved_fingerprint := _fingerprint(sim)

	# --- Run PAST the save, so loading it has to actually go back. ----------
	for i in range(250):
		sim.tick()
	if sim.current_tick() == saved_tick:
		_fail("the world must have moved on before the load is tested")
		return
	if _fingerprint(sim) == saved_fingerprint:
		_fail("250 ticks must have changed something, or the load proves nothing")
		return

	# --- A refused load says why, and changes nothing. ----------------------
	var before_refusal := _fingerprint(sim)
	var refusal: String = sim.load_ron("this is not a save file")
	if refusal == "":
		_fail("a garbage save must be refused, not loaded")
		return
	if not refusal.to_lower().contains("parse"):
		_fail("the refusal must say what was wrong, got '%s'" % refusal)
		return
	if _fingerprint(sim) != before_refusal:
		_fail("a refused load must leave the live world completely untouched")
		return

	# A save from another build is refused the same way, by version rather
	# than by whatever the shape change happens to break. The version this
	# build writes is read out of the save rather than spelled here, so a
	# schema bump does not quietly turn this case into "loads fine".
	var version_line := RegEx.new()
	version_line.compile("schema_version: (\\d+)")
	var found := version_line.search(save_text)
	if found == null:
		_fail("a save must carry a schema_version")
		return
	var from_the_future := save_text.replace(
		found.get_string(), "schema_version: %d" % (int(found.get_string(1)) + 1)
	)
	var version_refusal: String = sim.load_ron(from_the_future)
	if not version_refusal.contains("schema_version"):
		_fail("a save from another build must be refused by version, got '%s'" % version_refusal)
		return
	if _fingerprint(sim) != before_refusal:
		_fail("a version-refused load must leave the live world untouched too")
		return

	# --- The real thing: the earlier moment comes back. ---------------------
	var error: String = sim.load_ron(save_text)
	if error != "":
		_fail("the save must load, got '%s'" % error)
		return
	if sim.current_tick() != saved_tick:
		_fail("current_tick must go back to %d, got %d" % [saved_tick, sim.current_tick()])
		return
	if _fingerprint(sim) != saved_fingerprint:
		_fail("every snapshot must go back to what it was when the world was saved")
		return
	if sim.save_ron() != save_text:
		_fail("re-saving a freshly loaded world must produce the identical file")
		return

	# --- A load announces itself to a chunk-driven renderer. ----------------
	var dirty: Array = Array(sim.dirty_chunk_ids())
	if dirty.is_empty():
		_fail("a loaded world must report its chunks dirty so the scene redraws")
		return
	for chunk_id in dirty:
		var snapshot = JSON.parse_string(sim.chunk_snapshot_json(chunk_id))
		if not (snapshot is Dictionary):
			_fail("a dirty chunk from a loaded world must have a readable snapshot")
			return

	# --- New Game throws it all away. ---------------------------------------
	sim.new_game()
	if sim.current_tick() != 0:
		_fail("new_game() must reset the tick count, got %d" % sim.current_tick())
		return
	var layout: Array = JSON.parse_string(sim.world_layout_json())
	if not layout.is_empty():
		_fail("new_game() must leave an empty floor, got %d placement(s)" % layout.size())
		return
	var shop: Dictionary = JSON.parse_string(sim.shop_snapshot_json())
	if float(shop["money"]) <= 0.0:
		_fail("new_game() must restore the starting balance")
		return

	print("SAVE_LOAD_OK")
	quit(0)
