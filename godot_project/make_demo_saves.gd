# Writes one save file per demo layout into `user://saves/`, so Load Game on
# the main menu has something in it on a fresh machine.
#
# Run with:
#   scripts/make-demo-saves.sh
#   godot --headless --path godot_project --script make_demo_saves.gd
#
# Why this exists as a committed script: the four demo saves that have been
# used for demos since 2026-09-10 were made by a throwaway script that was
# never kept, so nobody could regenerate them after a save-schema bump — and a
# save is REJECTED, not migrated, on a `schema_version` mismatch
# (ARCHITECTURE.md §4). Rebuilding them has to be one command.
#
# Each layout is built the only way anything is ever built here — real
# `Purchase`/`Place` commands through `test_layouts.gd`, the same fixture
# file the headless proofs and the screenshot runs use (CLAUDE.md's "every
# headless Godot test builds its factory through real placement commands").
# So a demo save cannot contain a factory a player could not have built, and
# these files exercise the real `save_ron` boundary rather than a hand-written
# RON blob that could drift from the schema.
#
# Slot names are the player-facing display names from the plan doc's
# hand-off notes. `GameSession.write_slot` owns sanitizing and the reserved
# `autosave` slot, so this script never touches a path itself.
extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const GameSessionScript = preload("res://game_session.gd")

## The demo slots, in the order Load Game will list them. Each name is paired
## with its `test_layouts.gd` builder by `_build`, which is a match rather than
## a `call()` by name: the builders are STATIC, and a static function cannot be
## called dynamically off the script class.
const DEMOS := ["demo line", "full chain", "belt gallery", "pipe gallery"]


## Builds `slot`'s layout into `sim`. Returns false for a name with no builder,
## so a typo in DEMOS fails loudly instead of writing an empty factory.
func _build(slot: String, sim) -> bool:
	match slot:
		"demo line":
			Layouts.build_demo_line(sim)
		"full chain":
			Layouts.build_reference_line(sim)
		"belt gallery":
			Layouts.build_belt_gallery(sim)
		"pipe gallery":
			Layouts.build_pipe_gallery(sim)
		_:
			return false
	return true

## Ticks run before saving, so a loaded demo opens with product already on the
## belts rather than a cold factory the player has to wait on.
const WARMUP_TICKS := 600

var _main
var _ran := false


func _fail(msg: String) -> void:
	printerr("DEMO_SAVES_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	_main = load("res://main.tscn").instantiate()
	root.add_child(_main)
	_main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true

	GameSessionScript.ensure_saves_dir()
	var written := []

	for slot in DEMOS:
		# A fresh world per demo: `new_game` is the same reset the main menu's
		# New Game does, so one demo's factory can never leak into the next.
		_main.sim.new_game()
		if not _build(slot, _main.sim):
			_fail("no builder for demo slot '%s'" % slot)
			return true
		Layouts.stock_larder(_main.sim)
		for _i in range(WARMUP_TICKS):
			_main.sim.tick()

		var ron: String = _main.sim.save_ron()
		if ron.strip_edges() == "":
			_fail("save_ron returned nothing for '%s'" % slot)
			return true

		# Proven to LOAD before it is written, for the reason
		# `game_session.gd` proves a save before changing scene: a demo save
		# that cannot be loaded is worse than no demo save, because it is only
		# discovered in front of whoever the demo is for.
		var reason: String = _main.sim.load_ron(ron)
		if reason != "":
			_fail("'%s' saved but would not load back: %s" % [slot, reason])
			return true

		var error: String = GameSessionScript.write_slot(slot, ron)
		if error != "":
			_fail("could not write '%s': %s" % [slot, error])
			return true
		written.append("%s (%d bytes)" % [slot, ron.length()])

	print("DEMO_SAVES_OK: wrote %d demo saves to %s — %s" % [
		written.size(), GameSessionScript.resolved_saves_dir(), ", ".join(written)
	])
	quit(0)
	return true
