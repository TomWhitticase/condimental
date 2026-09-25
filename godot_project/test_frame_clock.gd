# Headless proof that the HUD's fades are timed by FRAMES, not by the
# operating system's clock — the half of issue #82 that a fixed frame rate
# alone does not reach.
#
# Run with:
#   godot --headless --path godot_project --script test_frame_clock.gd
#
# WHY THIS EXISTS. `scripts/screenshot-scene.sh` pins the engine's frame
# delta (`--fixed-fps`), which pins how far the simulation runs before the
# shutter. It does not pin `Time.get_ticks_msec()`, and three HUD elements
# were timed off that: the money flyout, the hotbar's name flyout and the
# contract notice. They sit outside the simulation tick on purpose — they
# have to keep fading under the pause menu, where there are no ticks at all
# — so they were the last thing in a review frame whose state depended on
# how long the render took. They now read `main.gd`'s `_hud_now_msec()`,
# which is the sum of the frame deltas this scene has been given.
#
# Nothing headless can render, so this cannot prove a PNG. What it proves is
# the property the PNG rests on, and it fails on the regression that matters
# — a fade put back on the wall clock:
#   * `_hud_now_msec()` does not move while real milliseconds pass with no
#     frames (a sleep), and does move, by the delta it was given, when they
#     are handed over,
#   * all three fades run to completion on frames alone, in a test that
#     takes microseconds of real time to advance seconds of theirs — on the
#     wall clock they would still be at full ink,
#   * the warm-up arithmetic the screenshot harness depends on: at
#     `test_screenshot.gd`'s own FIXED_FPS, its WARMUP_FRAMES frames always
#     advance the simulation by the same number of ticks.
extends SceneTree

const Screenshot = preload("res://test_screenshot.gd")

## Real milliseconds to let pass without handing the scene a single frame.
## Short on purpose: the claim is that NO amount of it moves the HUD clock,
## and a quarter of a second is enough to catch a clock that follows the
## machine instead of the frames.
const REAL_SLEEP_MSEC := 250

var main
var _ran := false


func _fail(msg: String) -> void:
	printerr("FRAME_CLOCK_FAIL: %s" % msg)
	quit(1)


func _initialize() -> void:
	main = load("res://main.tscn").instantiate()
	root.add_child(main)
	main._ready()


func _process(_delta: float) -> bool:
	if _ran:
		return true
	_ran = true
	_run_checks()
	return true


## One frame of the real per-frame path, at the rate a review render is
## pinned to — `_process`, not `_advance_simulation`, because the HUD clock
## is a property of a FRAME and the fades are updated from `_process`.
func _frame() -> void:
	main._process(1.0 / Screenshot.FIXED_FPS)


## Frames enough to cover `msec` of the HUD's own time, plus one, so a
## deadline exactly on the boundary is past rather than level with it.
func _frames_worth(msec: int) -> void:
	var count := int(ceil(float(msec) * Screenshot.FIXED_FPS / 1000.0)) + 1
	for i in range(count):
		_frame()


func _alpha_of(label) -> float:
	return label.modulate.a


func _run_checks() -> void:
	if main._contract_money_change == null or main._hotbar_flyout == null \
			or main._contract_notice == null:
		_fail("the HUD did not build its flyouts — nothing to time")
		return

	# --- 1. The clock is the frames, and only the frames -------------------
	var before: int = main._hud_now_msec()
	OS.delay_msec(REAL_SLEEP_MSEC)
	if main._hud_now_msec() != before:
		_fail(
			"%d real milliseconds moved _hud_now_msec() from %d to %d without a single "
			% [REAL_SLEEP_MSEC, before, main._hud_now_msec()]
			+ "frame — the HUD is back on the wall clock, so a review render's fades "
			+ "again depend on how long the render took (issue #82)"
		)
		return
	for i in range(60):
		_frame()
	var after: int = main._hud_now_msec()
	var expected: int = before + 1000
	if absf(float(after - expected)) > 2.0:
		_fail(
			"60 frames at %d fps should have moved the HUD clock to about %d ms, not %d"
			% [int(Screenshot.FIXED_FPS), expected, after]
		)
		return

	# --- 2. Every fade completes on frames alone ---------------------------
	# Money: a balance that moves flashes "+N" under the figure. The scene's
	# own poll is left seeing NO change either side of the flash — seeded
	# with the real balance, and put back to it afterwards — because a
	# `_flash_money_change` from `_update_contract_panel` on one of the
	# frames below would restart the very fade being timed.
	var money: float = float(JSON.parse_string(main.sim.economy_snapshot_json()).get("money", 0.0))
	main._last_money = money
	main._flash_money_change(money + 60.0)
	main._last_money = money
	main._update_money_flyout()
	if _alpha_of(main._contract_money_change) < 1.0:
		_fail("the money flyout was not at full ink on the frame it flashed")
		return
	_frames_worth(main.MONEY_FLYOUT_HOLD_MSEC + main.MONEY_FLYOUT_FADE_MSEC)
	# Asked for explicitly, because `_process` only runs the fades on a frame
	# the wire did NOT change (`godot_project/CLAUDE.md`) and the frames above
	# are ticking a simulation. What is under test is the clock the fade
	# reads, not which branch of `_process` last called it.
	main._update_money_flyout()
	if _alpha_of(main._contract_money_change) > 0.0:
		_fail(
			"the money flyout was still showing (alpha %.2f) after %d ms of FRAMES — "
			% [_alpha_of(main._contract_money_change),
				main.MONEY_FLYOUT_HOLD_MSEC + main.MONEY_FLYOUT_FADE_MSEC]
			+ "it is timed off something other than _hud_now_msec()"
		)
		return

	# Hotbar: the selected slot's name.
	main._show_hotbar_flyout("conveyor belt")
	main._update_hotbar_flyout()
	if _alpha_of(main._hotbar_flyout) < 1.0:
		_fail("the hotbar flyout was not at full ink on the frame it was shown")
		return
	_frames_worth(main.HOTBAR_FLYOUT_HOLD_MSEC + main.HOTBAR_FLYOUT_FADE_MSEC)
	main._update_hotbar_flyout()
	if _alpha_of(main._hotbar_flyout) > 0.0:
		_fail(
			"the hotbar flyout was still showing (alpha %.2f) after its whole hold and "
			% _alpha_of(main._hotbar_flyout)
			+ "fade in frames — it is timed off something other than _hud_now_msec()"
		)
		return

	# The contract notice, which holds for six seconds: the one that would
	# take a seven-second sleep to prove on a wall clock, and takes 361
	# frames here.
	main._show_contract_notice("★ BBQ unlocked")
	main._update_contract_notice()
	if _alpha_of(main._contract_notice) < 1.0:
		_fail("the contract notice was not at full ink on the frame it was raised")
		return
	_frames_worth(main.CONTRACT_NOTICE_HOLD_MSEC + main.CONTRACT_NOTICE_FADE_MSEC)
	main._update_contract_notice()
	if _alpha_of(main._contract_notice) > 0.0:
		_fail(
			"the contract notice was still showing (alpha %.2f) after its whole hold "
			% _alpha_of(main._contract_notice)
			+ "and fade in frames — it is timed off something other than _hud_now_msec()"
		)
		return

	# --- 3. The warm-up the screenshot harness counts on -------------------
	# `test_screenshot.gd` waits WARMUP_FRAMES frames before the shutter and
	# the engine hands each of them 1/FIXED_FPS. That makes the ticks spent
	# in between arithmetic rather than weather — this asserts the number,
	# and that two identical warm-ups spend the same one.
	var spent := _warmup_ticks()
	var wanted := int(floor(float(Screenshot.WARMUP_FRAMES) * main.TICK_HZ / Screenshot.FIXED_FPS))
	if spent != wanted:
		_fail(
			"%d frames at %d fps advanced the simulation %d tick(s), not the %d that "
			% [Screenshot.WARMUP_FRAMES, int(Screenshot.FIXED_FPS), spent, wanted]
			+ "TICK_HZ %d says — a screenshot's settle is no longer a function of its "
			% int(main.TICK_HZ)
			+ "frame count"
		)
		return
	var again := _warmup_ticks()
	if again != spent:
		_fail(
			"two identical warm-ups spent %d and %d ticks — a review render is back to "
			% [spent, again]
			+ "photographing whatever moment the host's load allowed (issue #82)"
		)
		return

	print("FRAME_CLOCK_OK: HUD fades and the screenshot warm-up run on frames, not the wall clock")
	quit(0)


## Ticks the simulation spends over one screenshot warm-up, driven through
## `_advance_simulation` at the pinned delta. The accumulator is cleared
## first so the count is the warm-up's own and not the leftovers of the
## frames above it, exactly as a fresh run's would be.
func _warmup_ticks() -> int:
	main._tick_accum = 0.0
	var start: int = main.sim.current_tick()
	for i in range(Screenshot.WARMUP_FRAMES):
		main._advance_simulation(1.0 / Screenshot.FIXED_FPS)
	return main.sim.current_tick() - start
