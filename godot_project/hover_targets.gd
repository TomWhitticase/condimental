# What each `--hover*` screenshot flag aims at, as data rather than as one
# near-identical function per flag.
#
# **A FLAG THAT FINDS NOTHING MUST REFUSE.** That is the whole reason this is
# a table. Until it was one, every hover helper was guarded by `if tile !=
# NONE:` and so did nothing at all when the world had no such thing in it:
# `--fixture full_dock --hover-splitter` aimed at nothing, parked on empty
# ground, wrote the PNG and printed SCREENSHOT_OK, handing back a clean frame
# of the dock to a reviewer who had asked to see a splitter's tooltip. Plain
# `--hover` did the same against the empty world, which is the shortest
# command there is. That is issue #70's class of failure exactly — a
# plausible frame of the wrong thing at exit 0 — and it is the one thing
# `test_screenshot.gd`'s cursor guard exists to make impossible.
#
# So the hole is closed for the SET rather than for the seven: every flag
# has a row here and a `refusal` to print when its row finds nothing,
# `test_screenshot.gd` has one loop over `rows_for()` and no other way to aim
# a hover flag, `scripts/check-invariants.sh`
# fails if a `--hover*` string appears in that file outside the table, and
# `test_hover_targets.gd` drives every row against every fixture headlessly.
# An eighth flag is one row here; it inherits the refusal because there is
# nowhere else to put it.
#
# Nothing in here knows about Godot's scene, the cursor or the camera —
# it is `world_layout_json` in, a tile out — which is what lets the headless
# test check the same rows the windowed harness uses.
extends RefCounted

## No such thing in this world. Deliberately the same sentinel value
## `test_screenshot.gd` exports as `HOVER_NONE`, which is also what
## `main.gd`'s `_tile_under_screen_pos` returns for a ray that hits no floor:
## one "nowhere" throughout, rather than three that have to be told apart.
const NONE := Vector2i(-999999, -999999)


## Every `--hover*` flag there is, in **table order** — which is also the
## precedence, because `requested_flags()` and `rows_for()` both walk this
## list rather than the command line. So `--hover-dock --hover` aims at
## whichever appears first here, not at whichever was typed last. That is
## what the seven `if _frames == 2 and _hover_x` blocks this replaced did
## too, so the behaviour is unchanged; only this comment was ever wrong.
##
## `footprint_size` is `main.gd`'s own `_footprint_size`, passed in rather
## than copied: the multi-tile row needs to know how big a placement is, and
## a second implementation of that would be a second thing to keep in step.
static func all(footprint_size: Callable) -> Array:
	return [
		{
			"flag": "--hover",
			"looks_for": "a machine — anything placed that runs a recipe",
			"find": func(layout: Array) -> Vector2i: return first_machine(layout),
		},
		{
			"flag": "--hover-pipe",
			"looks_for": "a pipe",
			"find": func(layout: Array) -> Vector2i: return first_of_kind(layout, "pipe"),
		},
		{
			"flag": "--hover-vat",
			"looks_for": "a machine with a footprint bigger than one tile",
			"find": func(l: Array) -> Vector2i: return multi_tile_far_corner(l, footprint_size),
		},
		{
			"flag": "--hover-hopper",
			"looks_for": "a hopper",
			"find": func(layout: Array) -> Vector2i: return first_of_kind(layout, "hopper"),
		},
		{
			"flag": "--hover-tank",
			"looks_for": "a fluid tank",
			"find": func(layout: Array) -> Vector2i: return first_of_kind(layout, "fluid_tank"),
		},
		{
			"flag": "--hover-dock",
			"looks_for": "a delivery dock",
			"find": func(layout: Array) -> Vector2i: return first_of_kind(layout, "dock"),
		},
		{
			"flag": "--hover-splitter",
			"looks_for": "a conveyor splitter",
			"find": func(layout: Array) -> Vector2i: return first_of_kind(layout, "splitter"),
		},
	]


## Every flag in the table, as plain strings. The one place a caller can ask
## "is this a hover flag?" before it has a scene to ask about footprints with
## — `test_screenshot.gd` parses its command line long before `_main` exists.
## `test_hover_targets.gd` checks this list against `all()`, so the two
## cannot drift.
const FLAGS: PackedStringArray = [
	"--hover", "--hover-pipe", "--hover-vat", "--hover-hopper", "--hover-tank",
	"--hover-dock", "--hover-splitter",
]


## The hover flags on a command line, in table order.
static func requested_flags(args) -> PackedStringArray:
	var found: PackedStringArray = []
	for flag in FLAGS:
		if args.has(flag):
			found.append(flag)
	return found


## The rows for a set of flags, in table order.
static func rows_for(flags, footprint_size: Callable) -> Array:
	var rows := []
	for row in all(footprint_size):
		if flags.has(row["flag"]):
			rows.append(row)
	return rows


## The tile `row`'s flag aims at in `layout`, or `NONE`.
static func tile_for(row: Dictionary, layout: Array) -> Vector2i:
	return row["find"].call(layout)


## What the run says before it refuses, when a flag's target is not there.
##
## It names the three things the person at the terminal needs and cannot see
## from a missing PNG: WHICH flag came up empty, WHAT it was looking for, and
## WHICH world it looked in — including the case where there is no world,
## since `--hover-dock` with no fixture at all is the easiest way to hit this
## and "the dock is missing" would be a confusing way to say "you built
## nothing". It does not guess at a remedy beyond the two that always apply,
## because which fixture the shot wanted is not knowable from here.
static func refusal(row: Dictionary, fixture: String, fixture_flag: String,
		layout: Array) -> String:
	var world := ""
	if layout.is_empty():
		world = "this run built no world at all — with no --fixture (and no \
flag that implies one) every tile is bare floor, so there is nothing anywhere \
to hover"
	elif fixture == "":
		world = "the world this run built has none"
	elif fixture_flag == "":
		world = "--fixture %s builds %d placement(s) and none of them is one" \
			% [fixture, layout.size()]
	else:
		world = "%s's fixture ('%s') builds %d placement(s) and none of them is one" \
			% [fixture_flag, fixture, layout.size()]
	return (
		"SCREENSHOT_FAIL: %s found nothing to aim at. It looks for %s, and %s. \
Nothing was aimed at, so the frame would have shown the empty ground the cursor \
was parked on and still reported SCREENSHOT_OK — a plausible picture of the \
wrong thing, which is what this harness exists to refuse. Ask for a fixture \
that has one (scripts/screenshot-scene.sh says what each builds), or drop the flag."
		% [row["flag"], row["looks_for"], world]
	)


## The tile of the first placement `predicate` accepts, or `NONE`.
##
## "First" is `world_layout_json` order, which is the sim's placement order:
## stable for a given fixture, so a shot of the same fixture is a shot of the
## same tile, and derived from the world rather than typed in, so a fixture
## that moves its dock does not quietly leave the flag pointing at floor.
static func first_matching(layout: Array, predicate: Callable) -> Vector2i:
	for placement in layout:
		if predicate.call(placement):
			var pair: Array = placement["tile"]
			return Vector2i(int(pair[0]), int(pair[1]))
	return NONE


static func first_of_kind(layout: Array, kind: String) -> Vector2i:
	return first_matching(layout, func(p): return String(p["kind"]) == kind)


## The first placement that RUNS A RECIPE — a machine, as opposed to a belt,
## a pipe or one of the buffer buildings, none of which carry one.
static func first_machine(layout: Array) -> Vector2i:
	return first_matching(layout, func(p): return bool(p.get("machine", false)))


## The FAR CORNER of the first multi-tile machine — NOT its origin. "Hovering
## anywhere on a multi-tile machine identifies it" is the claim the flag is
## there to photograph, and the origin is the one tile that would not show it.
static func multi_tile_far_corner(layout: Array, footprint_size: Callable) -> Vector2i:
	for placement in layout:
		# A MACHINE, as the flag's `looks_for` says: the 1x2 warehouse rack is
		# multi-tile too, and a line whose racks come first in the layout had
		# --hover-vat photographing a shelf.
		if not bool(placement.get("machine", false)):
			continue
		var size: Vector2i = footprint_size.call(placement.get("footprint"))
		if size == Vector2i.ONE:
			continue
		var pair: Array = placement["tile"]
		return Vector2i(int(pair[0]), int(pair[1])) + size - Vector2i.ONE
	return NONE
