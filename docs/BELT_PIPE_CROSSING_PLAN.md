# Plan: letting belts and pipes cross

Status: in progress, 2026-09-25. Integration branch
`TomWhitticase/belt-pipe-crossings`.

The design doc this implements lives in Drive —
["CONDIMENTAL — Design: letting belts and pipes cross"](https://docs.google.com/document/d/1lEemZC-R-blzAuKJdnadzs9aUC2TA3wxSJ0rpPvYn-A/edit).
**Read that first for why the other two options were rejected.** This file is
the build order and the corrections the visual spike forced on it.

## The shape, in one paragraph

A belt and a pipe that meet become **one entity on one tile**, of a new kind
`crossing`, carrying both a `BeltSegment` and a `PipeTile`. `topology.grid`
stays `BTreeMap<TileCoord, Entity>` — one occupant per tile, exactly as today
— so `Remove`, `Rotate`, hover, the ghost and `resolve_sink` never have to ask
"which layer?". That question is the whole cost of the layered-grid option
and the whole reason it was rejected; nothing in this plan may reintroduce it.

The player never buys a crossing and never picks one off the hotbar. They
place a pipe onto a belt (or a belt onto a pipe) and the tile becomes a
crossing. `Remove` takes the whole tile and credits back **both** a belt and
a pipe.

## What the spike corrected

The design doc claims a crossing is "one new voxel model: today's straight
pipe with the central stand swapped for two legs outside the lane", reasoning
that the belt's cleats top out at `BELT_CLEAT_Z = 10` and the pipe's tube
spans z 11–19. That is true about the belt and useless about the belt's
*traffic*: an item sits base-down on the running surface at z=10 and
`item_bottle` is 15 voxels tall, so what crosses the tile fills z 10–24,
inside a bore at z 12–19. Rendered, the flat bridge impales a box mid-tile.

The shape is the spike's `pipe_gantry` (commit `f1751d8`), which shipped
under the name `pipe_crossing` — T2 renamed it for what it had become. The
run lifts to exactly one tile above the floor (`GANTRY_AXIS_Z =
TILE_VOXELS`, 32), on risers of the run's own radius flush against either
side of the belt lane. Measured on the shipped model, its lowest voxel
*between the risers* — y 8..23, which contains `CROSSING_ITEM_Y` (10..21) —
is z = 28, three clear (25..27) of `CROSSING_ITEM_TOP_Z` (24); the spike's
figure of 27 was one voxel out. Not "anywhere over the lane": the risers
stand **on** the lane's boundary rows, y 7 and y 24 (elbows at y 3 and 28,
outer radius 4.5), and carry steel from z 13 up. Nothing travels those two
rows, which is why the clearance that matters is the one between them. Its two
open faces are ordinary bores on the pipe axis at `PIPE_AXIS_Z`, so a
crossing drops into an existing north–south run with no fitting and no
change to its neighbours.

## Decisions already taken

These are settled. A ticket that wants to revisit one escalates rather than
deviating.

- **Removal takes the whole tile and refunds both parts.** There is no
  "remove just the pipe". That is the layer ambiguity coming back at removal
  time; two clicks to rebuild with both pieces in hand is the cheaper price.
- **Rotation turns the whole assembly 90°.** The belt direction *is* the
  crossing's single `Direction` and the pipe axis is derived perpendicular to
  it, so `rotate_cw` stays the only rotation in the simulation
  (`types.rs`) and R cycles four states. Not a 180° flip, not a refusal.
- **A crossing is only ever perpendicular.** The pipe being placed already
  carries a `Direction` from the player's R presses, so a parallel attempt is
  refused with no guessing. Parallel is neither drawable nor meaningful.
- **No new shop entry, no new hotbar slot.** A crossing is assembled, never
  bought, so it teaches no new concept and costs no UI.
- **The crossing's pipe connects along its axis only.** See below — this is
  the one genuinely new rule in the feature.

## The open-sides mask

Pipe connectivity today is undirected and joins on all four sides regardless
of a pipe's own `Direction` — the BFS frontier at `world.rs:3191` walks
`Direction::ALL`, and the `segment_links` loop below it pairs east and north
neighbours. A crossing must not do that. Its pipe half runs along one axis
and physically spans the tile on that axis alone; if it joined on all four
sides it would weld itself to any pipe sitting on the belt-parallel side and
silently merge two runs that visibly do not touch.

So every pipe tile gains an **open-sides mask** — the set of directions on
which it will accept an adjacency. An ordinary pipe's mask is all four; a
crossing's is its axis (two). Adjacency is agreed only where **both** sides
are open, which keeps the relation symmetric and therefore keeps the flood
fill's components well defined. This is the one part of the feature that a
test must pin rather than a screenshot: a crossing between two perpendicular
runs must leave them as two networks, not one.

## Tickets

Every ticket lands its own branch by pull request into the integration branch
`TomWhitticase/belt-pipe-crossings`, with a screenshot in the PR body; the
integration branch reaches `main` by one PR of its own. No ticket pushes to
`main`, and no ticket merges another ticket's branch by hand.

### T1 — sim: the `crossing` kind

**Target:** `sim_core/src/sim/world.rs`, `save.rs`, `sim_core/tests/`. No
Godot, no GDScript, no art.

**Change:** a `BodyKind::Crossing` whose entity carries both a `BeltSegment`
and a `PipeTile`, following `Splitter` as the precedent for "belt-like enough
that every existing transfer function works on it verbatim". Specifically:

- `try_place` grows the upgrade path. Where it today refuses any occupied
  tile (`world.rs:2551`), a pipe placed onto a belt — or a belt onto a pipe —
  with a perpendicular direction instead despawns the occupant and spawns a
  crossing. **The existing `BeltSegment` moves across verbatim**, items and
  all: upgrading a live line must not destroy what is on the tile. The
  newly placed kind comes out of `owned`; the consumed one is *not* credited
  back, because it is still standing there.
- `try_remove` credits back both `belt` and `pipe`.
- `try_rotate` turns the crossing as one piece.
- `recompute_topology` puts a crossing in both `topology.belts` and
  `topology.pipes`, and `resolve_sink` accepts one as a solid sink exactly as
  it accepts a belt. The fluid flood fill honours the open-sides mask above.
- `SavedBody::Crossing` — a new variant, added the way `WarehouseRack` was,
  with **no `SAVE_SCHEMA_VERSION` bump**: an older save has no crossings and
  loads unchanged.

**Constraints:** no `f32`/`f64` in anything shared or persisted;
`BTreeMap`/`BTreeSet` for anything walked in a tick; the tick is the only
clock. `sim/` stays free of `gdext`. Content stays in `data/*.ron` — if a
field is missing, add the field rather than a Rust constant.

**Acceptance:** `cargo test -p sim_core` green, with new tests that prove at
minimum: an item already on the belt survives the upgrade; a crossing hands
solids on and accepts them exactly as a belt tile does; two perpendicular
pipe runs meeting at a crossing stay **two** networks; `Remove` returns one
belt and one pipe to `owned`; four `Rotate`s return the tile to where it
started; and a world with a crossing saves and loads identically.

### T2 — art: finish the gantry

**Target:** `assets/voxels/generators/assets.rs`, `assets/voxels/models/`,
`assets/voxels/icons/`. No sim, no Godot.

**Change:** promote the spike's `pipe_gantry` into the shipping asset —
name it for what it is now that it is a real kind, give it an icon, and add
the fluid body at the raised axis so a crossing shows what it is carrying
the way every other pipe tile does (`pipe_fluid_gantry` exists but is
unfinished: nothing positions it at `GANTRY_AXIS_Z`). Decide the fate of
`pipe_bridge_flat` — it exists only as the counter-example in
`only_the_gantry_clears_what_the_belt_is_carrying`. Keeping a rejected model
in `ASSETS` forever is clutter; deleting it costs the regression guard.
Either is defensible, but say which and why in the PR.

**Constraints:** the generator is the source, the `.vox` is committed build
output. Colours only from `palette.ron` via `p.idx`, indices append-only.
Symmetry about voxel 15.5 for anything Godot rotates — a crossing rotates
four ways, so this one matters. Read `assets/voxels/CLAUDE.md` first.

**Acceptance:** `cargo test -p voxgen` green; the asset read at play zoom and
at 1:1 in a rendered frame, not judged from the generator; the icon judged at
its real 64px on the real plate.

**What shipped** (PR #169): `pipe_gantry` became `pipe_crossing`, with
`pipe_fluid_crossing` as its raised liquid body and a separate icon-only
`crossing` model with a frozen belt and bottle baked in. `pipe_bridge_flat`
was **deleted**; `only_the_gantry_clears_what_the_belt_is_carrying` keeps its
counter-example by turning an ordinary straight across the lane instead, so
the guard survived without a rejected model living in `ASSETS` forever.
Neither name below outlives this note — grep for `pipe_crossing`.

### T3 — godot: draw it and let the player build it

**Target:** `godot_project/main.gd`, `test_layouts.gd`, `test_screenshot.gd`.
Depends on T1 and T2 being merged into the integration branch.

**Change:** the presentation half. The kind `crossing` maps to the gantry
model with a documented fallback; belt items pass under it and the fluid fill
overlay tracks the raised axis. The placement UX has to teach the mechanic
with no words: holding a pipe over a perpendicular belt must read as a
**valid** placement that will produce a crossing, not as the refused-red the
player gets for every other occupied tile — and holding it over a *parallel*
belt must still read as refused. Hover on a crossing names both halves.

**Constraints:** Godot renders, formats and sorts; the sim decides. Nothing
here may recompute a number the wire already carries, and the upgrade rule
itself lives in Rust — GDScript only asks. Every content-id lookup gets a
documented fallback. `.tscn`/`.tres` stay text.

**Acceptance:** a `crossing` fixture in `test_layouts.gd` built through real
purchase/place commands (the spike's hand-placed `_stand_crossing_spike`
harness in `test_screenshot.gd` is **deleted** by this ticket — it stands the
model up by hand and is a lie once the sim can do it); every headless
`test_*.gd` green; and a `scripts/screenshot-scene.sh` frame, read, showing a
belt running under a live pipe with the ghost in both its valid and refused
states.

### T4 — docs and the decision record

**Target:** `docs/`. Depends on T1–T3.

**Change:** record the feature where the repo keeps such things —
`GAMEPLAY.md` for the mechanic, `ARCHITECTURE.md` for the crossing entity and
the open-sides mask, a `DECISIONS.md` entry plus its `DECISIONS_LOG.md` line
for "one entity per tile, not a layered grid", and a `DEVLOG.md` note. Fold
this plan's corrections back into the Drive doc so the two do not disagree.

**Acceptance:** `scripts/verify-core-loop.sh` green on the integration branch
with every ticket in, and the decision reachable from `DECISIONS_LOG.md`.
