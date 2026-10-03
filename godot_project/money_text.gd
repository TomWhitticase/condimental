## How money is WRITTEN, everywhere a player reads it: pounds sterling, with
## the symbol (the owner's rule, 2026-10-03 — "whenever we display money use
## a pound sign"). The one place the symbol lives, so the HUD, every panel
## and THE COOKBOOK cannot disagree about it.
##
## Formatting only: every amount still comes off the wire, and the
## simulation's `Money` has no currency at all. Reached by `preload`, never
## `class_name` (godot_project/CLAUDE.md).
extends RefCounted

const SYMBOL := "£"


## Money, rounded to whole pounds — "£1200", "-£30". No
## thousands separator: the HUD's columns are measured against plain digits.
static func amount(value: float) -> String:
	var whole := roundi(value)
	if whole < 0:
		return "-%s%d" % [SYMBOL, -whole]
	return "%s%d" % [SYMBOL, whole]


## A change of money, always signed: "+£50" for a gain, "-£30" for a spend.
static func change(value: float) -> String:
	var whole := roundi(value)
	return ("+%s%d" % [SYMBOL, whole]) if whole >= 0 else ("-%s%d" % [SYMBOL, -whole])
