---
name: Deferred finding
about: Something noticed during other work and consciously not fixed
title: ''
labels: deferred
---

<!-- The next agent to read this starts cold, with no memory of the session
that filed it. Everything it needs to act must be in this body. -->

## What is wrong

<!-- One or two sentences. The current behaviour, not the fix. -->

## Where

<!-- File and function, e.g. `sim_core/src/sim/world.rs`, `resolve_sink`.
"Somewhere in the Godot layer" is not enough to act on. -->

## How it was found

<!-- Which test, which `screenshot-scene.sh` flag, which fixture, which run.
This is what lets someone reproduce it in one command instead of ten. -->

## Why it was deferred

<!-- Out of scope for the work in hand / needs a decision from the owner /
invalidates saves / too large for the session. If it is the owner's call,
add the `owner-decision` label and say exactly what the question is. -->

## What the fix probably is

<!-- If known, even roughly — "one condition in `resolve_sink`" is worth a
lot. Say what it would break: saves, the shipped demo line, a fixture,
a design doc that would need amending. Say "unknown" if it is unknown. -->
