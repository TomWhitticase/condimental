## Every preference the player can change, and the one file they live in.
##
## An autoload, beside `GameSession` and `Music`, for `GameSession`'s reason:
## the menu and the factory are two scenes and `change_scene_to_file` frees
## one before the other's `_ready` runs, so a preference parented to either
## would not survive the cut. Both scenes read and write this singleton, and
## `settings_panel.gd` is the one screen that edits it.
##
## # Why none of this is in the save file, and none of it is in `sim_core`
##
## `main.gd`'s `_toggle_sound` has carried the answer as a comment since the
## game's first sound: *"a persisted audio setting belongs with the rest of a
## settings screen this project does not have yet, not smuggled into the save
## format."* A save is a state of the **factory** — what is on the floor, what
## is in the pipes, what the contracts owe. How loud the music is, and how
## fast the camera flies, are states of the **player's machine**: they should
## be the same across every save on this box and should not travel with a save
## file copied to another. So they live in `user://settings.cfg`, and
## `sim_core` never hears about any of them — not one value below changes a
## tick, so not one belongs behind the wire (ARCHITECTURE.md §3/§4).
##
## `ConfigFile`, not JSON or RON: it is Godot's own answer for exactly this
## file, it is plain text a developer can edit and diff, and an unreadable or
## half-written one degrades to the defaults below rather than to an error.
##
## # The headless seam
##
## `_ready` loads the file only on a real display, the same test
## `music.gd::_can_play` makes. Every headless `test_*.gd` instantiates
## `main.tscn`, and a developer who had set movement speed to x3 would
## otherwise fail `test_camera_nav.gd` on their machine and nowhere else —
## a settings file is per-developer state, and a gate that reads it is a gate
## that means something different on every box. `test_settings.gd` drives
## `load_file()` and `save_file()` by hand instead, so the round trip is still
## covered. `test_screenshot.gd` is NOT headless and so resets to defaults
## explicitly; see its header.
extends Node

## Where the file lives. `user://` resolves per-platform, exactly as
## `GameSession.SAVES_DIR` does — `resolved_path()` is the same courtesy
## `GameSession.resolved_saves_dir()` exists to provide.
const SETTINGS_PATH := "user://settings.cfg"
const SECTION := "settings"

# --- The knobs ------------------------------------------------------------
#
# Each one is a DEFINITION, not just a default: the min, the max and the step
# below are what `settings_panel.gd` steps through and what `_clamped` pins a
# loaded file to, so a hand-edited file can never put a value somewhere the
# panel cannot bring it back from. One dictionary per knob, in one list, in
# the order the panel draws them — a row is added by adding an entry here, not
# by editing the panel.
#
# `kind` is "toggle" or "step". A step's `format` names how the value reads in
# words, because a bare number on a settings row says nothing: a volume is a
# percentage, a speed is a multiplier, a field of view is degrees (UI_UX.md
# §7 — never encode state invisibly, and never leave a number unlabelled).
const FORMAT_PERCENT := "percent"
const FORMAT_MULTIPLIER := "multiplier"
const FORMAT_DEGREES := "degrees"

const KNOBS := [
	{
		"key": "music_enabled",
		"label": "MUSIC",
		"kind": "toggle",
		"default": true,
		"help": "the looping score",
	},
	{
		"key": "music_volume",
		"label": "MUSIC VOLUME",
		"kind": "step",
		"default": 1.0,
		"min": 0.0,
		"max": 1.0,
		"step": 0.1,
		"format": FORMAT_PERCENT,
		"help": "",
	},
	{
		"key": "sfx_enabled",
		"label": "SOUND",
		"kind": "toggle",
		"default": true,
		"help": "clunks, chimes and refusals",
	},
	{
		"key": "sfx_volume",
		"label": "SOUND VOLUME",
		"kind": "step",
		"default": 1.0,
		"min": 0.0,
		"max": 1.0,
		"step": 0.1,
		"format": FORMAT_PERCENT,
		"help": "",
	},
	{
		"key": "move_speed",
		"label": "MOVEMENT SPEED",
		"kind": "step",
		"default": 1.0,
		"min": 0.25,
		"max": 3.0,
		"step": 0.25,
		"format": FORMAT_MULTIPLIER,
		"help": "walking and panning alike",
	},
	{
		"key": "look_sensitivity",
		"label": "LOOK SENSITIVITY",
		"kind": "step",
		"default": 1.0,
		"min": 0.25,
		"max": 3.0,
		"step": 0.25,
		"format": FORMAT_MULTIPLIER,
		"help": "first-person mouse only",
	},
	{
		"key": "field_of_view",
		"label": "FIELD OF VIEW",
		"kind": "step",
		"default": 75.0,
		"min": 60.0,
		"max": 110.0,
		"step": 5.0,
		"format": FORMAT_DEGREES,
		"help": "first-person camera only",
	},
	{
		"key": "hall_scale",
		"label": "HALL SIZE",
		"kind": "step",
		"default": 1.0,
		"min": 0.5,
		"max": 2.5,
		"step": 0.25,
		"format": FORMAT_MULTIPLIER,
		"help": "how far the building goes",
	},
]

# --- The values -----------------------------------------------------------
#
# Plain members rather than a Dictionary so every read is typed and a typo in
# a key is a parse error rather than a silent `null`. `KNOBS` above is what
# maps one to the other, through `get_value`/`set_value`.
#
# Each setter re-applies through `changed`, so a caller never has to remember
# to. `changed` is what `main.gd` and `menu.gd` connect to: one signal for
# every knob, because every listener re-reads the two or three values it owns
# anyway and a signal per knob would be eight connections to keep in step.

signal changed

var music_enabled := true:
	set(value):
		music_enabled = value
		_emit()
var music_volume := 1.0:
	set(value):
		music_volume = value
		_emit()
var sfx_enabled := true:
	set(value):
		sfx_enabled = value
		_emit()
var sfx_volume := 1.0:
	set(value):
		sfx_volume = value
		_emit()
var move_speed := 1.0:
	set(value):
		move_speed = value
		_emit()
var look_sensitivity := 1.0:
	set(value):
		look_sensitivity = value
		_emit()
var field_of_view := 75.0:
	set(value):
		field_of_view = value
		_emit()
var hall_scale := 1.0:
	set(value):
		hall_scale = value
		_emit()

## Suppresses `changed` while a batch of values is being written (a file load,
## a reset), so eight setters fire one signal between them rather than eight.
var _quiet := false
## Whether `_ready` actually read the file. False under `--headless` and in a
## test that never asked.
var loaded := false

## Whether this node may touch `user://settings.cfg` at all. **Off under
## `--headless`**, and that is the whole reason it exists: every one of this
## project's tests instantiates the real scene, presses real buttons and runs
## against the developer's own `user://`, so a test that flips the sound off
## would otherwise flip the sound off on the machine it ran on — and a
## developer who had set movement speed to x3 would fail `test_camera_nav.gd`
## on their box and nowhere else. A headless run neither reads the file nor
## writes it; it runs on the shipped defaults, which is what a test should be
## asserting against anyway. `test_settings.gd` turns it back on deliberately,
## around a backup of the real file, to prove the round-trip.
##
## Decided in the MEMBER INITIALIZER rather than in `_ready`, because an
## autoload's `_ready` has not run yet when a `--script` test's
## `_initialize()` does: the node is on the root and answers to `get_node`,
## but anything `_ready` was going to set is still at its declared value.
## A test that read `persist_enabled` there would have been told `true` on a
## headless run, which is the exact opposite of what this flag is for.
var persist_enabled := DisplayServer.get_name() != "headless"


func _ready() -> void:
	if not persist_enabled:
		return
	load_file()


## The absolute path `SETTINGS_PATH` resolves to, for the line `main.gd`
## prints at startup — a developer looking for the file should not have to
## know how `user://` resolves. `GameSession.resolved_saves_dir()`'s twin.
static func resolved_path() -> String:
	return ProjectSettings.globalize_path(SETTINGS_PATH)


# --------------------------------------------------------------------------
# Reading and writing one knob
# --------------------------------------------------------------------------

## The definition of `key`, or `{}`. The panel walks `KNOBS`, so this is for
## the callers that hold a key and want its range.
static func knob(key: String) -> Dictionary:
	for entry in KNOBS:
		if entry["key"] == key:
			return entry
	return {}


func get_value(key: String):
	return get(key)


## Writes `key`, clamped to its own definition. **The one door**: the panel,
## a file load and a reset all come through here, so a value that reached the
## members can never be outside the range the panel can step back through.
func set_value(key: String, value) -> void:
	var entry := knob(key)
	if entry.is_empty():
		return
	set(key, _clamped(entry, value))


## `value` pinned to `entry`'s own shape: a toggle to a bool, a step to its
## range and snapped to its grid.
##
## Snapping matters as much as clamping. The steps are tenths and quarters,
## neither of which is exact in binary, so eight presses of `+` from 0.0 land
## on 0.7999999 rather than 0.8 — and a row that reads "80%" while the file
## says 0.7999999 is a row that will one day read "79%". Snapping to the grid
## on every write means the stored value is always the nearest legal one.
static func _clamped(entry: Dictionary, value):
	if entry["kind"] == "toggle":
		return bool(value)
	var low: float = entry["min"]
	var high: float = entry["max"]
	var step: float = entry["step"]
	var pinned: float = clampf(float(value), low, high)
	var steps: float = round((pinned - low) / step)
	return clampf(low + steps * step, low, high)


## `key` moved `direction` (+1 or -1) steps, clamped. What the panel's two
## buttons call; a toggle flips instead.
func nudge(key: String, direction: int) -> void:
	var entry := knob(key)
	if entry.is_empty():
		return
	if entry["kind"] == "toggle":
		set_value(key, not bool(get_value(key)))
		return
	set_value(key, float(get_value(key)) + float(entry["step"]) * float(direction))


## How `key`'s current value reads on the row. Presentation formatting, in the
## one place both hosts share, so the panel and a test agree on the string.
func format_value(key: String) -> String:
	var entry := knob(key)
	if entry.is_empty():
		return ""
	var value = get_value(key)
	if entry["kind"] == "toggle":
		return "ON" if bool(value) else "OFF"
	match entry["format"]:
		FORMAT_PERCENT:
			return "%d%%" % int(round(float(value) * 100.0))
		FORMAT_DEGREES:
			return "%d°" % int(round(float(value)))
		_:
			return "x%.2f" % float(value)


## Where `key` sits in its range, 0..1 — the fill the row's bar draws. A
## toggle has no range, so it answers 1 or 0 and the panel draws no bar.
func fraction(key: String) -> float:
	var entry := knob(key)
	if entry.is_empty():
		return 0.0
	if entry["kind"] == "toggle":
		return 1.0 if bool(get_value(key)) else 0.0
	var low: float = entry["min"]
	var high: float = entry["max"]
	if high <= low:
		return 0.0
	return clampf((float(get_value(key)) - low) / (high - low), 0.0, 1.0)


# --------------------------------------------------------------------------
# The whole set
# --------------------------------------------------------------------------

## Every knob back to its `KNOBS` default, and one `changed` for the lot.
func reset() -> void:
	_quiet = true
	for entry in KNOBS:
		set_value(entry["key"], entry["default"])
	_quiet = false
	_emit()


## Whether every knob is at its default — what `test_screenshot.gd` asserts
## after resetting, and what the panel could one day grey its RESET out on.
func is_default() -> bool:
	for entry in KNOBS:
		if not is_same(get_value(entry["key"]), _clamped(entry, entry["default"])):
			return false
	return true


## Reads `SETTINGS_PATH`. **A missing or unreadable file is not an error** —
## it is a player who has never opened this screen, which is every player the
## first time, so it resets to the defaults and says so by returning `false`.
## A file missing one key keeps that key's default for the same reason; the
## file is a record of what was changed, not a schema.
##
## Unlike a save file, this one is never version-rejected. There is no state
## here that a future build could misread: a knob that no longer exists is
## ignored, and a knob that is new takes its default.
func load_file() -> bool:
	if not persist_enabled:
		return false
	var config := ConfigFile.new()
	var error := config.load(SETTINGS_PATH)
	_quiet = true
	for entry in KNOBS:
		var key: String = entry["key"]
		var stored = config.get_value(SECTION, key, entry["default"]) if error == OK else entry["default"]
		set_value(key, stored)
	_quiet = false
	loaded = error == OK
	_emit()
	return loaded


## Writes every knob. Returns "" or the reason it failed, `GameSession`'s
## convention, so a caller can put the reason on screen instead of guessing.
## Writes nothing, successfully, while `persist_enabled` is false.
func save_file() -> String:
	if not persist_enabled:
		return ""
	var config := ConfigFile.new()
	for entry in KNOBS:
		config.set_value(SECTION, entry["key"], get_value(entry["key"]))
	var error := config.save(SETTINGS_PATH)
	if error != OK:
		return "Could not write %s (error %d)." % [SETTINGS_PATH, error]
	return ""


## The score is applied HERE rather than by a listener, because `Music` is an
## autoload this node can always find and nothing else owns it — the menu has
## no sound of its own, so without this the track would be unsettable until a
## factory was loaded. Everything else (the cues, the camera, the hall) is
## owned by the scene that built it and is applied by `main.gd`'s listener.
func _apply_music() -> void:
	var music := music_node()
	if music == null:
		return
	music.enabled = music_enabled
	music.volume = music_volume


## The `Music` autoload, or null. Looked up off the tree root rather than as a
## global identifier for the reason `main.gd::_music` gives: the headless test
## scripts run outside the active scene tree, and an absolute node path
## resolved from there is an engine error rather than a null.
func music_node() -> Node:
	var loop := Engine.get_main_loop()
	if not (loop is SceneTree):
		return null
	var tree_root: Node = (loop as SceneTree).root
	return tree_root.get_node_or_null("Music") if tree_root != null else null


func _emit() -> void:
	if _quiet:
		return
	_apply_music()
	changed.emit()
