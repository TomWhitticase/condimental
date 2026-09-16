## The one thing that survives a scene change, plus everything this project
## knows about save FILES (the "Save/load + a real main menu" epic, ticket 2).
##
## Two jobs, and they are one file because they are the same seam.
##
## **1. The handoff.** `menu.tscn` and `main.tscn` are separate scenes, and
## `change_scene_to_file` frees the old one before the new one's `_ready`
## runs — so there is nowhere in either scene to hand "the player chose Load
## Game, from this file" across. An autoload lives outside both, which is
## Godot's own answer to exactly this, so the menu arms `pending_action`
## before it changes scene and `main.gd._ready()` reads it.
##
## The pending action is deliberately **sticky, not consumed**: reading it
## does not clear it. Every transition into `main.tscn` sets it first, so
## nothing is ever ambiguous, and a `_ready()` that runs twice (which every
## headless test does — `add_child` calls it once and the test calls it
## again) starts the same world both times instead of silently reverting to a
## new game on the second pass. The default is `new_game`, which is what
## makes a test that instantiates `main.tscn` directly and never touches this
## singleton behave exactly as it always did.
##
## **2. The files.** `sim_core` deliberately does no file I/O at all — it
## produces and consumes the RON *text* and nothing else, because resolving
## `user://saves/…` is Godot's job (see `SimBridge::save_ron`'s doc, and
## ARCHITECTURE.md §4's amended save-format paragraph). This is that job, in
## one place rather than half in the menu and half in the game: both scenes
## list, read and write saves through the static functions below, so the
## rules about where a save lives and what it may be called cannot drift
## between the screen that loads one and the screen that writes one.
extends Node

## Where saves live. `user://` resolves per-platform (macOS:
## `~/Library/Application Support/Godot/app_userdata/CONDIMENTAL/`), which
## is why `resolved_saves_dir()` exists and why `main.gd` prints it once at
## startup — a developer looking for the files should not have to know that.
const SAVES_DIR := "user://saves"
const SAVE_EXTENSION := "ron"

## The slot autosave owns, and the one name a manual save may not use.
##
## Reserved in BOTH directions on purpose. Autosave never writes a named slot
## (so a timer can never overwrite the save a player deliberately made), and
## a manual save is refused this name (so a player cannot put a save
## somewhere the next autosave will silently destroy three minutes later).
## The refusal is loud — `main.gd` says so in its flyout — rather than
## silently renaming, because a save that went somewhere other than where it
## was asked to go is worse than one that did not happen.
const AUTOSAVE_SLOT := "autosave"

## What `main.tscn` should do when it starts: `"new_game"` or `"load"`.
var pending_action := "new_game"
## The save file `pending_action == "load"` refers to. Absolute `user://` path.
var pending_path := ""
## The slot name the live game was loaded from (or last saved to), so the
## pause menu's save field can prefill with it. Empty for a brand-new game.
var current_slot := ""


## Arms a fresh start. Called by the menu's New Game and by Quit to Menu, so
## that a subsequent New Game cannot inherit a stale `load`.
func arm_new_game() -> void:
	pending_action = "new_game"
	pending_path = ""
	current_slot = ""


## Arms a load of one file. The caller is expected to have VALIDATED it first
## (the menu does, by running the text through a throwaway `SimBridge`), so
## that a corrupt file is refused on the screen that can still show a message
## rather than after the scene has already changed.
func arm_load(path: String, slot: String) -> void:
	pending_action = "load"
	pending_path = path
	current_slot = slot


# --------------------------------------------------------------------------
# Save files
# --------------------------------------------------------------------------

## The OS path `user://saves` actually resolves to — printed once at startup
## so the files are findable without knowing Godot's per-platform layout.
static func resolved_saves_dir() -> String:
	return ProjectSettings.globalize_path(SAVES_DIR)


## Creates `user://saves` if it isn't there. Every write goes through this;
## a listing tolerates its absence instead (an empty list is the right answer
## for a player who has never saved).
static func ensure_saves_dir() -> void:
	DirAccess.make_dir_recursive_absolute(SAVES_DIR)


static func slot_path(slot: String) -> String:
	return "%s/%s.%s" % [SAVES_DIR, slot, SAVE_EXTENSION]


## What a typed slot name becomes on disk. A save name is player text and a
## file name is not, so this is a whitelist rather than a blacklist: letters,
## digits, space, `-` and `_` survive, everything else becomes `_`, runs
## collapse, and the ends are trimmed. That makes `../../etc/passwd` into
## `.._.._etc_passwd`, which is a file name in the saves directory and
## nothing else — the property worth having, since the alternative (stripping
## known-bad sequences) has to be right about every sequence.
##
## Returns "" for a name with nothing usable left in it, which callers treat
## as a refusal rather than inventing a name of their own.
static func sanitize_slot(name: String) -> String:
	var out := ""
	for i in range(name.length()):
		var c := name[i]
		var code := name.unicode_at(i)
		var alphanumeric := (
			(code >= 48 and code <= 57)   # 0-9
			or (code >= 65 and code <= 90)   # A-Z
			or (code >= 97 and code <= 122)  # a-z
		)
		if alphanumeric or c == " " or c == "-" or c == "_":
			out += c
		else:
			out += "_"
	# Collapse runs of the substitute so a name of pure punctuation doesn't
	# become a long unreadable line of underscores.
	while out.contains("__"):
		out = out.replace("__", "_")
	out = out.strip_edges()
	while out.begins_with("_"):
		out = out.substr(1)
	while out.ends_with("_"):
		out = out.substr(0, out.length() - 1)
	return out.strip_edges().substr(0, MAX_SLOT_LENGTH)


## Long enough for a descriptive name, short enough to render in one row of
## the load list without eliding.
const MAX_SLOT_LENGTH := 40


## True when `slot` is the autosave's own name, however it was capitalized —
## the reservation is about the FILE, and file names are case-insensitive on
## both platforms this ships to today.
static func is_reserved_slot(slot: String) -> bool:
	return slot.to_lower() == AUTOSAVE_SLOT


## Writes `text` to `slot`'s file. Returns "" on success, or a
## human-readable reason — the same convention `SimBridge::load_ron` uses,
## and for its reason: a control that can fail for reasons the player cannot
## see has to be able to say which one happened.
##
## `allow_reserved` is how autosave writes the one name a manual save may
## not. It is a parameter rather than a second function because everything
## else about the write is identical, and two nearly-identical writers is how
## the two paths would drift.
static func write_slot(slot: String, text: String, allow_reserved := false) -> String:
	if slot == "":
		return "That save needs a name."
	if is_reserved_slot(slot) and not allow_reserved:
		return "'%s' is the autosave's own slot — pick another name." % AUTOSAVE_SLOT
	ensure_saves_dir()
	var file := FileAccess.open(slot_path(slot), FileAccess.WRITE)
	if file == null:
		return "Could not write %s (error %d)." % [slot_path(slot), FileAccess.get_open_error()]
	file.store_string(text)
	file.close()
	return ""


## The RON text of one save file, or "" if it could not be read at all. A
## caller distinguishes "unreadable" from "read but not a valid save" by
## asking `SimBridge::load_ron` about the text it gets back.
static func read_save(path: String) -> String:
	var file := FileAccess.open(path, FileAccess.READ)
	if file == null:
		return ""
	var text := file.get_as_text()
	file.close()
	return text


## Every save on disk, NEWEST FIRST, as
## `{"slot", "path", "modified_unix", "modified", "autosave"}`.
##
## What each row says about a save beyond its name is its **last-modified
## time**, which is honest and free. The save's own tick count is not read:
## it would mean parsing every file in the directory to draw a list, and a
## tick number is a worse answer to "which save is this" than a date is
## anyway. (`sim_core` has no cheap header to peek at, and inventing one
## would have been a boundary change this ticket had no need to make.)
static func list_saves() -> Array:
	var rows: Array = []
	var dir := DirAccess.open(SAVES_DIR)
	if dir == null:
		return rows
	for name in dir.get_files():
		if not String(name).ends_with(".%s" % SAVE_EXTENSION):
			continue
		var slot := String(name).substr(0, String(name).length() - SAVE_EXTENSION.length() - 1)
		var path := "%s/%s" % [SAVES_DIR, name]
		var stamp := FileAccess.get_modified_time(path)
		rows.append({
			"slot": slot,
			"path": path,
			"modified_unix": stamp,
			"modified": format_stamp(stamp),
			"autosave": is_reserved_slot(slot),
		})
	rows.sort_custom(func(a, b): return int(a["modified_unix"]) > int(b["modified_unix"]))
	return rows


## A last-modified time as the load list prints it. Local time, because the
## question it answers is "is this the one I was playing an hour ago".
static func format_stamp(unix_seconds: int) -> String:
	if unix_seconds <= 0:
		return "unknown"
	var bias: int = int(Time.get_time_zone_from_system().get("bias", 0))
	var d := Time.get_datetime_dict_from_unix_time(unix_seconds + bias * 60)
	return "%04d-%02d-%02d  %02d:%02d" % [d["year"], d["month"], d["day"], d["hour"], d["minute"]]
