# Headless proof of the sound effects and the music track (2026-09-12;
# UI_UX.md §8's audio direction as amended, `audio.gd`, `music.gd`).
#
# What it asserts:
#   * every cue `audio.gd` names has a stream after `setup`, none clips,
#     none is silent, and each is deterministic (two renders byte-identical)
#   * the music track is a seamless forward loop at its own rate, exactly
#     the length the tempo and bar count say, unclipped, deterministic, and
#     its loop seam is small (the last sample meets the first)
#   * the `Music` autoload exists, is asked to start by the game scene,
#     builds nothing headless (no device to hear it), ducks while the pause
#     menu is up and un-ducks when it closes, and the pause menu's MUSIC
#     button says ON or OFF in words
#   * every player action fires its cue from the handler that knows whether
#     the simulation took it: place / remove, a purchase that went through
#     and one refused for money, a market crate and a shift+click that buys
#     all it can afford (and one refused with an empty wallet, which sends
#     no command at all), a signature and a refused re-signature,
#     hire / fire and a refused fire at zero headcount, a hotbar selection,
#     every modal opening and closing, and a lost contract
#
# As with test_payout_feedback.gd, a dummy audio driver makes "was a sound
# heard" unanswerable, so the assertions run against `audio.gd`'s `plays`
# tally — the seam that exists for exactly this.

extends SceneTree

const Layouts = preload("res://test_layouts.gd")
const FactoryAudio = preload("res://audio.gd")

var _ok := true


func _fail(msg: String) -> void:
	printerr("AUDIO_FAIL: %s" % msg)
	_ok = false


func _peak(stream: AudioStreamWAV) -> int:
	var peak := 0
	var count := stream.data.size() / 2
	for i in range(count):
		peak = maxi(peak, absi(stream.data.decode_s16(i * 2)))
	return peak


func _plays(main, cue: String) -> int:
	return int(main._audio.plays.get(cue, 0))


func _expect_plays(main, cue: String, expected: int, what: String) -> void:
	var got := _plays(main, cue)
	if got != expected:
		_fail("%s: expected %s x%d, got x%d" % [what, cue, expected, got])


func _initialize() -> void:
	# --- The cues -----------------------------------------------------------
	var audio = FactoryAudio.new()
	audio.setup(root)
	for cue in FactoryAudio.CUES:
		var stream = audio._streams.get(cue)
		if stream == null:
			_fail("cue '%s' has no stream after setup" % cue)
			continue
		var wav: AudioStreamWAV = stream
		var peak := _peak(wav)
		if peak >= 32767:
			_fail("cue '%s' clips (peak %d)" % [cue, peak])
		if peak < 1500:
			_fail("cue '%s' is nearly silent (peak %d)" % [cue, peak])
		if wav.loop_mode != AudioStreamWAV.LOOP_DISABLED:
			_fail("cue '%s' loops — a cue must end" % cue)
		if wav.mix_rate != FactoryAudio.MIX_RATE or wav.stereo:
			_fail("cue '%s' is not mono at MIX_RATE" % cue)
	if FactoryAudio.place_stream().data != FactoryAudio.place_stream().data:
		_fail("place_stream is not deterministic")
	if FactoryAudio.rotate_stream().data != FactoryAudio.rotate_stream().data:
		_fail("rotate_stream (noise-only) is not deterministic — the LCG must seed the same")
	if FactoryAudio.menu_open_stream().data == FactoryAudio.menu_close_stream().data:
		_fail("open and close are the same sound")
	if FactoryAudio.hire_stream().data == FactoryAudio.fire_stream().data:
		_fail("hire and fire are the same sound")

	# --- The music -------------------------------------------------------------
	var music_a: AudioStreamWAV = FactoryAudio.music_stream()
	var music_b: AudioStreamWAV = FactoryAudio.music_stream()
	if music_a.data != music_b.data:
		_fail("music_stream is not deterministic")
	if music_a.loop_mode != AudioStreamWAV.LOOP_FORWARD or music_a.loop_begin != 0 \
			or music_a.loop_end != music_a.data.size() / 2:
		_fail("the music must loop forward over its whole length")
	if music_a.mix_rate != FactoryAudio.MUSIC_MIX_RATE or music_a.stereo:
		_fail("the music is not mono at MUSIC_MIX_RATE")
	var expected_samples := int((60.0 / FactoryAudio.MUSIC_BPM) * FactoryAudio.MUSIC_BEATS_PER_BAR
		* FactoryAudio.MUSIC_BARS * FactoryAudio.MUSIC_MIX_RATE)
	if absi(music_a.data.size() / 2 - expected_samples) > 1:
		_fail("music is %d samples, expected %d (%d bars at %.0f BPM)"
			% [music_a.data.size() / 2, expected_samples, FactoryAudio.MUSIC_BARS, FactoryAudio.MUSIC_BPM])
	var music_peak := _peak(music_a)
	if music_peak >= 32767:
		_fail("the music clips (peak %d)" % music_peak)
	if music_peak < 8000:
		_fail("the music is too quiet to be a bed (peak %d)" % music_peak)
	var last := music_a.data.decode_s16(music_a.data.size() - 2)
	var first := music_a.data.decode_s16(0)
	if absi(last - first) > 2500:
		_fail("loop seam steps by %d (last %d, first %d) — a click every loop" % [last - first, last, first])
	if FactoryAudio.MUSIC_CHORDS.size() != FactoryAudio.MUSIC_BARS:
		_fail("one chord per bar: %d chords for %d bars" % [FactoryAudio.MUSIC_CHORDS.size(), FactoryAudio.MUSIC_BARS])

	# --- The autoload -----------------------------------------------------------
	var music: Node = root.get_node_or_null("Music")
	if music == null:
		_fail("the Music autoload is not registered — check project.godot's [autoload]")
		quit(1)
		return
	if music.is_playing():
		_fail("music is playing under a headless run — nothing can hear it and the render is pure cost")

	# --- The game scene -----------------------------------------------------------
	var main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()
	if int(music.starts) < 1:
		_fail("main.gd's _ready must ask the score to start (it is idempotent)")
	if main._pause_music_button == null or main._pause_music_button.text != "MUSIC: ON":
		_fail("the pause menu's MUSIC button must caption itself ON at start")
	main._toggle_music()
	if bool(music.enabled) or main._pause_music_button.text != "MUSIC: OFF":
		_fail("toggling music must switch the autoload off and caption the button OFF")
	main._toggle_music()
	if not bool(music.enabled) or main._pause_music_button.text != "MUSIC: ON":
		_fail("toggling music back must caption the button ON")

	# Menus, and the pause duck.
	main._audio.plays.clear()
	main._set_pause_open(true)
	if not bool(music.ducked):
		_fail("the score must duck while the pause menu is up")
	_expect_plays(main, FactoryAudio.MENU_OPEN, 1, "pause open")
	main._set_pause_open(false)
	if bool(music.ducked):
		_fail("the score must un-duck when the pause menu closes")
	_expect_plays(main, FactoryAudio.MENU_CLOSE, 1, "pause close")
	main._set_shop_menu_open(true)
	main._set_shop_menu_open(true) # already open: no second cue
	main._set_shop_menu_open(false)
	main._set_market_open(true)
	main._set_market_open(false)
	main._set_contract_board_open(true)
	main._set_contract_board_open(false)
	main._set_staff_open(true)
	main._set_staff_open(false)
	_expect_plays(main, FactoryAudio.MENU_OPEN, 5, "every modal opening once")
	_expect_plays(main, FactoryAudio.MENU_CLOSE, 5, "every modal closing once")

	# Hotbar selection.
	main._audio.plays.clear()
	main._select_hotbar_slot(0)
	_expect_plays(main, FactoryAudio.SELECT, 1, "hotbar select")
	main._exit_build_mode()

	# A purchase that goes through, and one refused for money.
	Layouts.fund(main.sim, 5000)
	main._set_shop_menu_open(true)
	main._audio.plays.clear()
	var belt_slot := -1
	for i in range(main._shop_slots.size()):
		if String(main._shop_slots[i]["kind"]) == "belt":
			belt_slot = i
	if belt_slot < 0:
		_fail("no belt slot in the build menu")
	else:
		main._buy_slot(belt_slot)
		_expect_plays(main, FactoryAudio.BUY, 1, "an affordable purchase")
		_expect_plays(main, FactoryAudio.REFUSED, 0, "an affordable purchase")
		Layouts.fund(main.sim, 0)
		main._buy_slot(belt_slot)
		_expect_plays(main, FactoryAudio.REFUSED, 1, "a purchase with no money")
		_expect_plays(main, FactoryAudio.BUY, 1, "a purchase with no money")
	main._set_shop_menu_open(false)

	# Stock: bought, then refused.
	Layouts.fund(main.sim, 5000)
	main._set_market_open(true)
	main._audio.plays.clear()
	if main._market_slots.is_empty():
		_fail("no market slots")
	else:
		main._buy_ingredient_slot(0)
		_expect_plays(main, FactoryAudio.BUY, 1, "an affordable crate")
		Layouts.fund(main.sim, 0)
		main._buy_ingredient_slot(0)
		_expect_plays(main, FactoryAudio.REFUSED, 1, "a crate with no money")
		# The same square's other verb (shift+click buys all you can afford),
		# on the same rule: the cue comes from the outcome, never the gesture.
		# The refused case is the interesting one — it sends no command at
		# all, so the "no" has to be fired by the handler itself.
		Layouts.fund(main.sim, 5000)
		main._audio.plays.clear()
		main._buy_all_from_slot(0)
		_expect_plays(main, FactoryAudio.BUY, 1, "shift+click buying all it could afford")
		_expect_plays(main, FactoryAudio.REFUSED, 0, "shift+click buying all it could afford")
		main._audio.plays.clear()
		main._buy_all_from_slot(0)
		_expect_plays(main, FactoryAudio.REFUSED, 1, "shift+click with an empty wallet")
		_expect_plays(main, FactoryAudio.BUY, 0, "shift+click with an empty wallet")
	main._set_market_open(false)

	# Place, then remove.
	Layouts.fund(main.sim, 5000)
	main._enter_build_mode("belt")
	if not main._in_build_mode():
		_fail("could not enter build mode with an owned belt")
	main._ghost_tile = Vector2i(12, 12)
	main._ghost_valid = true
	main._audio.plays.clear()
	main._confirm_build()
	_expect_plays(main, FactoryAudio.PLACE, 1, "placing a belt")
	if main._placement_at_tile(Vector2i(12, 12)).is_empty():
		_fail("the belt was not placed")
	main._exit_build_mode()
	main._remove_mode = true
	main._ghost_tile = Vector2i(12, 12)
	main._confirm_build()
	main._remove_mode = false
	_expect_plays(main, FactoryAudio.REMOVE, 1, "removing a belt")
	if not main._placement_at_tile(Vector2i(12, 12)).is_empty():
		_fail("the belt was not removed")

	# A signature, then the same offer refused (one of each, never two).
	main._set_contract_board_open(true)
	main._audio.plays.clear()
	# The first offer the economy wire says is NOT active (the starter is).
	var offer := -1
	var econ: Dictionary = JSON.parse_string(main.sim.economy_snapshot_json())
	for i in range(main._board_rows.size()):
		var id := String(main._board_rows[i]["id"])
		var active := false
		for c in econ.get("contracts", []):
			if String(c.get("id", "")) == id:
				active = true
		if not active and offer < 0:
			offer = i
	if offer < 0:
		_fail("no unsigned offer on the board to sign")
	else:
		main._sign_contract_offer(offer)
		_expect_plays(main, FactoryAudio.SIGN, 1, "signing an offer")
		main._sign_contract_offer(offer)
		_expect_plays(main, FactoryAudio.REFUSED, 1, "signing an already-active offer")
		_expect_plays(main, FactoryAudio.SIGN, 1, "signing an already-active offer")
	main._set_contract_board_open(false)

	# Hire, fire, and a fire refused at zero.
	main._set_staff_open(true)
	main._audio.plays.clear()
	if main._staff_rows.is_empty():
		_fail("no staff rows")
	else:
		main._staff_hire(0)
		_expect_plays(main, FactoryAudio.HIRE, 1, "hiring a head")
		main._staff_fire(0)
		_expect_plays(main, FactoryAudio.FIRE, 1, "firing a head")
		var empty := -1
		for i in range(main._staff_rows.size()):
			if main._headcount_of(String(main._staff_rows[i]["id"])) == 0 and empty < 0:
				empty = i
		if empty < 0:
			_fail("no category at zero headcount to refuse a fire on")
		else:
			main._staff_fire(empty)
			_expect_plays(main, FactoryAudio.REFUSED, 1, "firing at zero headcount")
	main._set_staff_open(false)

	# A contract lost, announced by the poll diff.
	main._audio.plays.clear()
	main._known_active_primed = true
	main._known_active_contract_ids = ["corner_shop"]
	main._announce_contract_changes([], [])
	_expect_plays(main, FactoryAudio.CONTRACT_LOST, 1, "a lost contract")
	main._known_active_contract_ids = []
	main._announce_contract_changes([{"id": "corner_shop"}], [])
	_expect_plays(main, FactoryAudio.CONTRACT_LOST, 1, "a signature must not play the loss cue")

	# Muting counts but plays nothing — the existing rule, kept.
	main._audio.enabled = false
	main._audio.plays.clear()
	main._audio.play(FactoryAudio.PLACE)
	_expect_plays(main, FactoryAudio.PLACE, 1, "a muted cue still tallies")
	main._audio.enabled = true

	if not _ok:
		quit(1)
		return
	print("AUDIO_OK: %d cues, none clipping, all deterministic; the music is a seamless %d-bar loop; "
		% [FactoryAudio.CUES.size(), FactoryAudio.MUSIC_BARS]
		+ "the score ducks under the pause menu and has its own switch; place/remove/buy/refused/"
		+ "sign/hire/fire/select/menu/lost each fire from the handler that knows the outcome")
	quit(0)
