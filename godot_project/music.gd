## The one looping music track, and the node that keeps it playing across
## the menu → game scene change.
##
## An autoload (`project.godot`'s `[autoload]`, beside `GameSession`) for
## the same reason `GameSession` is one: `change_scene_to_file` frees the
## menu before the game's `_ready` runs, so a player parented to either
## scene would stop dead at the cut. Here it is parented to the root and the
## track carries straight through — the menu and the factory are one place
## with one score, not two rooms with two.
##
## **The track itself is `audio.gd`'s `music_stream()`**, synthesised like
## every cue in this project (see that file's header for why the waveforms
## are code). It is built LAZILY, the first time something asks it to play
## while a real audio device exists: twenty seconds of PCM take a noticeable
## fraction of a second to render in GDScript, and the headless test scripts
## — which instantiate the game dozens of times per verify run and can hear
## nothing — must not pay it. A headless run therefore records the request
## (`starts`) and builds nothing; a test that wants the stream calls
## `FactoryAudio.music_stream()` itself.
##
## Two switches, both session-scoped like the pause menu's SOUND switch:
## `enabled` (the MUSIC: ON/OFF button) and `ducked` (set by the pause menu
## while it is up — the factory has stopped, and the score stepping back a
## few decibels says so without going silent, which would read as a crash).
extends Node

const FactoryAudio = preload("res://audio.gd")

## The track's level. Under the cues' `VOLUME_DB` by a clear margin: music
## is a bed, and a bed that competes with the payout chime has failed.
const VOLUME_DB := -13.0
## How far the bed steps back while the pause menu is up.
const DUCK_DB := -10.0

var enabled := true:
	set(value):
		enabled = value
		_apply()
var ducked := false:
	set(value):
		ducked = value
		_apply()

## The player's own level, 0..1, from the settings screen. Rides on top of
## `VOLUME_DB` and composes with `DUCK_DB` in `_apply` — `audio.gd`'s `volume`
## and for its reason: the constants are the mix, this is the listener.
var volume := 1.0:
	set(value):
		volume = clampf(value, 0.0, 1.0)
		_apply()

## How many times `start()` has been asked for — the headless seam.
var starts := 0

var _player: AudioStreamPlayer
var _stream: AudioStreamWAV
var _render: Thread


func _ready() -> void:
	_player = AudioStreamPlayer.new()
	_player.name = "music"
	_player.bus = "Master"
	_player.volume_db = VOLUME_DB
	add_child(_player)


## Starts the track if it is not already going. Idempotent: the menu calls
## it, then the game scene calls it again, and the second call must not
## restart the loop.
func start() -> void:
	starts += 1
	# A scene that starts the score is never paused on its first frame: a
	# quit-to-menu from the pause menu would otherwise carry the duck out
	# with it.
	ducked = false
	if not _can_play():
		return
	if _stream == null:
		if _render == null:
			_render = Thread.new()
			_render.start(FactoryAudio.music_samples)
		return
	_apply()


func _process(_delta: float) -> void:
	if _render == null or _render.is_alive():
		return
	var samples: PackedFloat32Array = _render.wait_to_finish()
	_render = null
	# ~25 ms on the main thread (measured 2026-09-12) against ~700 for the
	# samples: one frame, once, and the resource is made where it is used.
	_stream = FactoryAudio._encode(samples, FactoryAudio.MUSIC_MIX_RATE, true)
	_player.stream = _stream
	_apply()


## The way the game quits: put the track down, give the audio mix thread
## a couple of frames to release its playback, then quit the tree. Every
## quit path (the menu's QUIT, the pause menu's QUIT GAME, the window's
## close button) comes through here rather than calling `get_tree().quit()`
## itself, because a stream still playing at the instant the tree is torn
## down races the engine's exit check and, three runs in four (measured
## 2026-09-12 with `--verbose`), reports its `AudioStreamPlaybackWAV` and
## `AudioStreamWAV` leaked at refcount 1. Harmless — the process is ending
## — but a warning on every quit is a warning nobody reads, and stopping
## ten frames earlier made it zero in four.
func quit_tree() -> void:
	if _player != null:
		_player.stop()
		_player.stream = null
	_stream = null
	if not is_inside_tree():
		return
	var tree := get_tree()
	await tree.process_frame
	await tree.process_frame
	tree.quit()


## A thread left running at quit is an engine error; join it first.
func _exit_tree() -> void:
	if _render != null:
		_render.wait_to_finish()
		_render = null


func is_playing() -> bool:
	return _player != null and _player.playing


## The renderer names the display server; a headless run ("headless") has
## no audio output either, so building a track for it would be pure cost.
func _can_play() -> bool:
	return _player != null and _player.is_inside_tree() and DisplayServer.get_name() != "headless"


func _apply() -> void:
	if _player == null:
		return
	# `linear_to_db(0.0)` is `-INF` and must not be added to anything — see
	# `FactoryAudio.SILENCE_DB`, whose floor this shares so the two channels
	# agree on what "off" sounds like.
	if volume <= 0.0:
		_player.volume_db = FactoryAudio.SILENCE_DB
	else:
		_player.volume_db = VOLUME_DB + linear_to_db(volume) + (DUCK_DB if ducked else 0.0)
	if not enabled and _player.playing:
		_player.stop()
	elif enabled and not _player.playing and _stream != null and _can_play():
		_player.play()
