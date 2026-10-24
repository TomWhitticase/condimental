# Pipes & Conveyors Overhaul — plan

Status: **DONE** (started 2026-09-09; tickets 1–6 merged by 2026-09-10
morning; ticket 7, the close-out, landed 2026-09-15). Integration branch:
`TomWhitticase/pipes-and-conveyors-overhaul`, long since merged to `main`.
Built as one Orca orchestration Run; tickets below map 1:1 to Tasks.
See "What shipped, and what is still open" at the end of this doc before
picking anything up from it.

## Goal

Make belts and pipes look right, read right, run fast, and connect up. The
update must deliver:

1. Better belt and pipe models; pipe visual bugs fixed.
2. Orientation of a belt or pipe is obvious at a glance.
3. Belt animation that shows travel direction.
4. Belts and pipes connect visually when placed together: corners, tees,
   ends, and fittings into machines/tanks/hoppers/docks — at every facing.
5. General fixes and performance work around them.

## Ground rules for every ticket

- Follow CLAUDE.md and the three design docs. Read `docs/DECISIONS.md` Part B
  (UI_UX §2b long form) before drawing anything: it explains the camera's
  visible faces, why openings must be genuinely empty, and why the render is
  the test.
- The sim never learns about art. Godot never derives a fact the sim
  already knows; the sim puts it on the wire (ARCHITECTURE.md §4/§5).
- Assets are generators in `assets/voxels/generators/assets.rs`, regenerated
  with `scripts/build-voxels.sh`; the committed `.vox` files are build
  output. Every asset is authored with its product path along +X and is
  rotated in Godot by the placement's direction (UI_UX §2b point 18).
- Verify: `cargo test --workspace`, `scripts/verify-core-loop.sh`, and for
  anything visual `scripts/screenshot-scene.sh` — then READ the PNG.
- Conventional Commits; a DEVLOG entry per ticket; a condensed rule in
  CLAUDE.md's decisions log plus the full reasoning in `docs/DECISIONS.md`;
  new §2b points in UI_UX.md numbered from 19 upward.
- Do not merge into the integration branch yourself. Report the branch in
  `worker_done`; the coordinator merges.

## The wire contract (ticket 1 builds it; tickets 4/5 consume it)

Every belt, splitter and pipe placement in `world_layout_json` gains a
`links` object. Directions are absolute (`"north"|"east"|"south"|"west"`),
named from THIS tile toward the neighbour.

```json
// belt / splitter
"links": {
  "inputs":  [ {"from": "west",  "kind": "belt"},
               {"from": "south", "kind": "hopper"} ],
  "output":  {"to": "east", "kind": "machine"}      // or null
}
// pipe
"links": {
  "connections": [ {"to": "west",  "kind": "pipe"},
                   {"to": "east",  "kind": "pipe"},
                   {"to": "north", "kind": "machine"},   // a tap or a producer
                   {"to": "south", "kind": "tank"} ]
}
```

`kind` values: `belt`, `splitter`, `hopper`, `machine`, `dock`, `pipe`,
`tank`. `inputs` lists every thing the topology pass links INTO this tile;
`output` is the sink the tile hands to (null if nothing is linked). For a
splitter, `output` is the straight output; the branch stays in the existing
`outputs` field. These are the same links `World::recompute_topology` already
builds, so they cost nothing per tick and are exact.

Shape rules Godot derives from `links` (ticket 4/5 own the code):

- Belt: input from the tile behind (opposite its direction) → straight.
  Single input from a flank → **corner** (left or right). Two flank inputs or
  flank + behind → straight (a merge). No inputs → line start. No output →
  line end.
- Pipe: count and arrangement of `connections` → end (1), straight (2
  opposite), corner (2 adjacent), tee (3), cross (4). Machine and tank
  connections draw a port fitting on that face.

## Tickets

| # | Ticket | Layer | Depends on |
|---|---|---|---|
| 1 | Connectivity on the wire (`links`) + tests | sim | — |
| 2 | Belt model family: straight with direction chevrons, left/right corners, animation frames, facing-safe machine junctions | assets | — |
| 3 | Pipe model family: straight, corner, tee, cross, end, machine/tank port fittings, fluid bodies for each; pipe visual bugs fixed | assets | — |
| 4 | Instanced rendering for belts, pipes and items (performance) with a stress fixture and timings | godot | — |
| 5 | Belt presentation: shape from `links`, four-facing junctions, animation at sim speed, items curve through corners | godot | 1, 2, 4 |
| 6 | Pipe presentation: shape from `links`, fittings, fluid body per shape, hover | godot | 1, 3, 4 |
| 7 | Bug hunt and polish: render a layout gallery (every shape at every facing), fix what is wrong, docs and DEVLOG | all | 5, 6 |

Waves: {1, 2, 3, 4} in parallel → {5, 6} in parallel → 7 → human gate →
merge to `main`.

## Belt animation (decided here so tickets 2 and 5 agree)

Voxel meshes carry no UVs, so the belt is animated by **frames**: the
generator takes a phase `k` in `0..BELT_FRAMES` (4) and shifts the cleat and
roller pattern by `k * 32 / BELT_FRAMES` voxels along +X. Ticket 2 emits
`belt_f0..f3`, `belt_corner_left_f0..f3`, `belt_corner_right_f0..f3` (and
whatever junction variants it keeps, likewise framed). Ticket 5 shows frame
`floor(current_tick * BELT_FRAMES / BELT_TICKS_PER_TILE) mod BELT_FRAMES`
(a belt crosses a tile in 50 ticks, `sim_core/src/sim/belt.rs`), so the
surface moves at exactly item speed, stops when the sim is paused, and moves
along the placement's facing because the frames are authored along +X. Under
instancing (ticket 4) a frame change swaps which of the four per-frame
`MultiMeshInstance3D`s is visible, so animating a thousand belts costs four
visibility flips.

**Superseded on 2026-09-21 (issue #79), in both halves.** The clock is each
tile's own `BeltSegment::moved_ticks`, carried by `BeltSnapshot` — not
`current_tick`, because off the global tick a jammed line's rubber races
under items that are not moving (UI_UX.md §6a). Pause still freezes it, for
the same reason: no tick, no count. **Amended 2026-09-25:** the field is the
TICK the tile last ran on, not a count of ticks it advanced product on. An
empty tile is a running tile and keeps taking the new tick, so a line of
belts reads as one conveyor; only a JAM stops it taking the tick, and a
freed tile rejoins the world's one phase. And since tiles of one frame set are now on different frames, the
swap is per tile rather than per set: a tile not standing on a frame stays
in that frame's `MultiMesh` with a zero-scale basis at its own position, so
a frame change costs two transform writes for the tile that changed.

## Corner conventions (decided here so tickets 2, 3, 5, 6 agree)

- A belt corner model is authored for a tile whose OUTPUT is +X (east) and
  whose INPUT comes from the flank: `belt_corner_left` is fed from the NORTH
  side (+Y in tile space, i.e. product turns left when viewed from above with
  +X ahead), `belt_corner_right` is fed from the SOUTH side. Godot rotates by
  the tile's own direction as for every other asset.
- A pipe corner model is authored connecting the +X face and the +Y (north)
  face. Godot picks the rotation that maps that pair onto the tile's two
  connected faces. A tee is authored open on +X, -X and +Y; a cross on all
  four; an end cap open on -X only.
- Items on a belt corner follow a quarter-circle arc from the input face's
  midpoint to the output face's midpoint, parameterised by the sim's
  existing 0..1 distance, so nothing in the sim changes.

## Decisions made by tickets (read before consuming their output)

- **Ticket 2 (belts):** no belt-side junction variants. `belt_inlet` and
  `belt_outlet` are byte-identical aliases of `belt_f0` and ticket 5 should
  stop requesting them; the plain tile carries the junction and the
  machine-side hood/doorway do the rest at every facing. Frame models are
  `belt_f{0..3}`, `belt_corner_left_f{0..3}`, `belt_corner_right_f{0..3}`;
  `BELT_FRAMES` = 4 and the moving mark stands at `BELT_SURFACE_Z + 1`
  (an arrow until 2026-09-10, a plain rubber cleat since, at the player's
  request).
  Corners were render-checked only through a temporary `main.gd` hack —
  ticket 5 must look at them at all four facings once it draws them.
  `voxicons` renders icons for frame 0 only.

## Coordinator notes (appended as wave 1 lands)

> **Historical.** This section is the running hand-off as the tickets landed,
> kept because it records what each worker was told and what it reported.
> Everything it lists as outstanding was ticket 7's, and ticket 7 is done —
> read "What shipped, and what is still open" below for what is true now.

**Merged so far:** tickets 1 (`links`), 4 (instancing), 3 (pipe models) and
2 (belt models). Read their DEVLOG entries.

**For ticket 5, from ticket 2's worker and the merge:**

- The belt lesson ticket 2 wrote up is UI_UX.md §2b **point 23** (it was
  numbered 19 on its branch; ticket 3 had already taken 19–22, so every
  reference was renumbered at merge). Ticket 2's own decisions are in the
  "Decisions made by tickets" section above: no belt-side junction variants
  (`belt_inlet`/`belt_outlet` are aliases of `belt_f0` — stop requesting
  them and delete the aliases, their models and icons in this ticket), frame
  models `belt_f{0..3}` / `belt_corner_left_f{0..3}` /
  `belt_corner_right_f{0..3}`, `BELT_FRAMES` = 4.
- Corner face convention as authored: output +X; `belt_corner_left` is fed
  from **vox low Y = north** (the same convention ticket 3 used for pipes);
  right is the Y-mirror. Verify by render at all four facings — no
  committed script has drawn a corner yet.
- Ticket 2 flagged that the 4-frame march at sim speed is ~5 fps; judge it
  at default zoom in the real scene, and if it reads as stutter, prefer
  showing the frame closest to the exact sub-tile phase over changing the
  frame count (a change to BELT_FRAMES is an `ask`).
- The straight tile's arrow read at all four facings in ticket 2's
  close-ups; the south facing was never rendered — include it in the belt
  gallery. (The arrow was removed on 2026-09-10; the cleat that replaced it
  is what to look at now.)

**For ticket 6, from ticket 3's worker (verbatim facts, verify by render):**

- Face convention for the pipe shapes: **vox LOW Y = north.** `pipe_corner`
  is open on x=31 (+X) and y=0 (north); `pipe_tee` on x=31, x=0 and y=0;
  `pipe_end` on x=0 only (capped on +X). Fittings are authored on the +X
  face. Every shape is centred on the 15.5 midline, so a rotation lands
  exactly on its neighbours.
- `PIPE_BORE_FLOOR_VOXELS` (12) and `PIPE_BORE_HEIGHT_VOXELS` (7) in
  `main.gd` are unchanged. `pipe_fluid_end` already stops before the cap, so
  `FLUID_END_CAP_TRIM` can be deleted.
- Still visibly wrong until ticket 6 wires `links`: the demo's free-ended
  south run shows open bores (needs `pipe_end`); the fluids fixture's
  west-facing `pipe_inlet` points its branch north over bare floor (needs a
  corner/tee plus `pipe_fitting_machine`); vat→bottler shows a bare tube end
  at the bottler (needs the machine fitting overlay).
- Two straight tiles facing INTO a shared boundary stack two joint flanges;
  choose each straight tile's rotation so flanges alternate and never stack.
- Machine nozzles (`machine_fluid_port`) are still on voxel 16, half a voxel
  off the pipe axis; not visible, leave unless a render disagrees.
- Ticket 4's instancing keeps pipe fluid bodies as nodes so ticket 6 can
  decide per-shape bodies and per-instance tint; either is acceptable.

**For ticket 7:** ticket 1's `resolve_sink` finding (a belt feeds a machine
through any face of its inlet edge, including the outlet side; the demo
layout in `test_layouts.gd` relies on it; one-condition fix
`belt.direction == sink.direction`; `tests/links.rs` pins the current rule).
*Since decided (2026-10-01, #1): that fix is taken, and the demo line now
feeds the crusher through its inlet face.*
Ticket 4's finding that ~8 ms of every tick step is chunk-snapshot JSON
parsing on a 1500-belt line is milestone 8 scope, not this update's.

**State at the end of 2026-09-10's first session:** tickets 1–6 are merged
and verified on the integration branch (`cargo test --workspace`,
`scripts/verify-core-loop.sh` with 24 headless proofs, models and icons
regenerated with no drift). Ticket 7 is `ready` in the Run and not
dispatched.

**For ticket 7, gathered from tickets 5 and 6 and the merges:**

- UI_UX §2b numbering after the merges: 23 belt mark/frames (ticket 2),
  24 fitting margin and 25 pipe turn (ticket 6), 26 items drawn face to face
  (ticket 5). Every cross-reference was renumbered at merge; re-check.
- **Ghost rules disagree and need reconciling:** ticket 5 keeps the belt
  ghost a straight tile (a would-be corner is `preview_command`'s job, not a
  Godot-side copy of `resolve_sink`); ticket 6 previews a pipe ghost's shape
  from adjacent placed pipes. Pick one rule for both and record it.
- Headless runs print `ERROR: Parameter "material" is null` (from
  `material_get_instance_shader_parameters` in the dummy renderer) once per
  `_rebuild_world` that frees a World subtree holding `MultiMeshInstance3D`s
  — present since ticket 4, harmless, every proof passes. Silence it
  properly (clear the multimesh material before freeing, or equivalent) so
  the suite's output is clean.
- Belt animation is a 4-frame march at sim speed (2.4 frame changes/s).
  Nobody has watched it live; a PNG cannot judge stutter. Look at it in the
  running scene at default zoom and decide whether `BELT_FRAMES` stays 4.
- Ticket 5 fixed items being drawn half a tile downstream (only a corner
  exposed it). Ticket 4's perf rebuild went 38 → 64 ms with four frame sets;
  confirm `test_perf.gd`'s numbers with shapes + animation on.
- A closed belt ring is legal and jams at its merge (used by the gallery).
- ~~The `resolve_sink` inlet-edge rule from ticket 1 is still untouched.~~
  **Decided 2026-10-01 (#1):** a belt feeds only through the inlet face.
- Every worktree grows an untracked `AGENTS.md` (a CLAUDE.md copy) from the
  environment; nobody has committed it. Decide whether to `.gitignore` it.
- **Launching the real game after `cargo xtask` needs Godot's two `--import`
  passes** (`godot --headless --path godot_project --import`, twice): xtask
  restages `godot_project/voxels/` and drops every `.import` sidecar, so a
  windowed launch straight after it draws no meshes at all
  (`floor_tile mesh missing`). The headless scripts already do this; make
  `cargo xtask` do it too (or print the reminder) so a demo cannot start
  blank. Demo saves for Load Game (`demo line`, `full chain`,
  `belt gallery`, `pipe gallery`) were generated on 2026-09-10 with a
  throwaway script built on `test_layouts.gd`; a committed
  `scripts/make-demo-saves.sh` would be worth having.

## What shipped, and what is still open

Written at ticket 7's close-out (2026-09-15). Read this instead of inferring
the state of the update from the ticket table above.

### Shipped

- **The wire carries connectivity** (ticket 1). Every belt, splitter and pipe
  in `world_layout_json` carries a `links` object recorded by the topology
  pass itself, so Godot draws where product actually goes rather than
  re-deriving adjacency.
- **Belt and pipe model families** (tickets 2, 3) on the tile's true 15.5
  midline, in four animation frames, with composable fittings.
- **Instanced rendering** (ticket 4): belts, splitters, pipes and carried
  items are `MultiMesh` instances, guarded by a 1500-tile stress fixture and
  `test_perf.gd`.
- **Shape from links, for both** (tickets 5, 6): a belt's straight/corner/
  merge/start/end and a pipe's end/straight/corner/tee/cross, its turn and
  its fittings all come from `links` alone. `pipe_source`/`pipe_inlet` are
  deleted — **no asset in the project is authored for a single facing any
  more.** Items are drawn face to face, so a corner's arc leaves through the
  right face.
- **Galleries**: `--belt-gallery` and `--pipe-gallery` on
  `scripts/screenshot-scene.sh`, backed by `build_belt_gallery` /
  `build_pipe_gallery` in `test_layouts.gd`, cover every shape and junction
  at every facing. These replaced ticket 7's proposed single `--gallery`
  flag: one frame holding both families is too small to judge a flange in,
  which is the reason `--closeup` exists.
- **Close-out fixes** (ticket 7): the headless suite no longer prints
  `Parameter "material" is null` (`main.gd`'s `_release_render_resources`
  drops each instance's mesh and material before it is freed, at all six
  places the scene frees something that could hold one — the last found was
  the ghost body, whose tint is a `material_override` — so the dummy
  renderer has nothing to resolve — an ERROR line in a proof's
  output means something is wrong again); `cargo xtask` now runs Godot's two
  `--import` passes itself, because it is what deleted the `.import`
  sidecars and a launch straight after it used to draw no meshes at all;
  `scripts/make-demo-saves.sh` regenerates the four Load Game demo saves
  through real Purchase/Place commands, each proven to load before it is
  written; the stray `AGENTS.md` the agent environment drops in every
  worktree is `.gitignore`d.
- **One ghost rule, named** (ticket 7). The belt ghost (always straight) and
  the pipe ghost (shaped by adjacent placed pipes) looked like two rules
  because two tickets wrote them separately. They are one:
  **a ghost previews what the game's connection rules already make certain,
  and never what the simulation still has to decide** — which is
  GAMEPLAY.md §9's own split. A fluid connects by *touching*, so a pipe's
  shape is knowable from what stands beside it; a solid respects *faces* and
  the sim resolves which sink, so a belt's is not. The pipe ghost still
  previews no machine or tank fitting, because which tile of a run a machine
  taps is the sim's choice. Recorded as UI_UX.md §2b point 34; no behaviour
  changed.

### Still open

Each of these is now a GitHub issue as well, so it is findable without
reading this doc; the prose here stays as the evidence behind it.

- **Decided 2026-10-01 (#1): the owner chose the documented rule.** A belt
  now feeds a machine or dock only when it points through the inlet face;
  see `docs/DECISIONS_LOG.md`. The evidence that prompted it follows.
  **The `resolve_sink` inlet rule contradicts both design docs, and it moves
  real product.** A belt feeds a machine or dock when the tile it points
  into is on the inlet EDGE, *from whichever side it approaches*. For a 1x1
  machine every side is that edge. Measured at close-out, not inferred: a
  crusher facing east, fed from its north flank, crafts normally; and a belt
  standing on a machine's own OUTLET face is linked as feeding it, so the
  same belt both receives the machine's output and is registered to deliver
  into it. Today that only jams (no recipe consumes its own output), but it
  is a loop waiting for one that does.
  `sim_core/src/sim/world.rs`'s module doc says the opposite in as many
  words ("A belt pointing into a machine's flank does nothing. This is what
  makes rotating a machine a decision with a visible consequence"), and so
  does GAMEPLAY.md §9 ("A machine accepts input opposite its facing"). The
  code is the outlier. The fix is one condition in `resolve_sink`
  (`belt.direction == sink.direction`), but it is a **gameplay change, not a
  bug fix**: it stops flank feeding working, and `test_layouts.gd`'s
  `build_demo_line` is built on flank feeding (a north-going belt into a
  west-facing crusher), as is any player's save that used it.
  `sim_core/tests/links.rs` pins the current behaviour deliberately so the
  change is a visible diff. **Left for the owner to decide** — it was not
  taken unilaterally because it changes what a player is allowed to build.
  ([#1](https://github.com/TomWhitticase/sauce-factory/issues/1), `owner-decision`.)
- **Nobody has watched the belt animation live.** It is a 4-frame march at
  sim speed (~2.4 frame changes/s); whether `BELT_FRAMES` should stay 4 is a
  judgement a PNG cannot make and a headless run cannot make either. Needs
  somebody at a windowed build.
  ([#2](https://github.com/TomWhitticase/sauce-factory/issues/2).)
- **`_read_world_layout` is ~40 ms of an 80 ms rebuild on 1500 tiles** (a
  336 KB layout parse per placement). Milestone 8 scope, with the rest of
  the scale-up work.
  ([#3](https://github.com/TomWhitticase/sauce-factory/issues/3).) **Fixed
  2026-10-23:** a re-read parses only the placements whose JSON changed,
  ~4 ms after a rotate on a release build; the rest of the rebuild (node
  construction) is still milestone 8's.
- All performance figures anywhere in this doc are measured on a **debug**
  `sim_core`, which is what `cargo xtask` builds.

## Appendix — the ticket specs as dispatched

Exported from Orca Run `run_63d3ba781e83` on 2026-09-10 so the specs live in git, not only in the local orchestration database. Every ticket was prefixed with this shared context:

> CONTEXT (read first). You are a worker on the sauce-factory repo, in a child worktree branched from `TomWhitticase/pipes-and-conveyors-overhaul` — the integration branch of the "Pipes & Conveyors Overhaul" update. Read `docs/PIPES_AND_CONVEYORS_PLAN.md` before anything else: it is the plan for the whole update, it fixes the wire contract, the belt-animation frame scheme and the corner authoring conventions that other tickets rely on, and this ticket is one row of its table. CLAUDE.md and the three design docs load automatically; `docs/DECISIONS.md` Part B has the long-form art lessons (camera-visible faces, openings must be genuinely empty, the render is the test). The plan's "Ground rules for every ticket" apply: verify with `cargo test --workspace` and `scripts/verify-core-loop.sh` (and `scripts/screenshot-scene.sh` for anything visual — then actually read the PNG); Conventional Commits (`git config core.hooksPath .githooks` first); a DEVLOG entry; a condensed rule in CLAUDE.md's decisions log plus the full reasoning appended to `docs/DECISIONS.md` Part A; new UI_UX.md §2b points numbered from 19 upward if you learn an art lesson. Do NOT merge into the integration branch and do NOT touch files outside your ticket's scope beyond what it needs. Do not start work that belongs to another ticket in the table. If a decision would change the shared wire contract or the shared asset conventions in the plan, use `orca orchestration ask` before deviating. When done, commit everything on your branch and send worker_done containing: the branch name, the commit range, what changed, exactly what you verified (test summary, which PNGs you looked at and what you saw), and anything left or flagged.

### Ticket 1 — `task_8a968707b497` (completed)

TICKET 1 — Connectivity on the wire (`links`). Layer: sim_core only (plus any headless .gd test that asserts world_layout_json's exact keys, if one breaks).

Build the `links` field exactly as `docs/PIPES_AND_CONVEYORS_PLAN.md` "The wire contract" specifies, on every belt, splitter and pipe placement in `world_layout_json` (`PlacementSnapshot` in `sim_core/src/sim/world.rs`), and on the matching `query_tile_json` variants so hover can use it. Serialize `#[serde(skip_serializing_if = "Option::is_none")]` so kinds without links are byte-identical to today. Directions are absolute, named from the tile toward the neighbour. `kind` values: belt, splitter, hopper, machine, dock, pipe, tank.

Derive it from the links `World::recompute_topology` already builds — never a second adjacency scan. Where the Topology struct does not currently record something needed (hopper->belt, machine outlet->belt, belt->dock, pipe<->pipe adjacency within a network, tank->pipe, machine tap/producer->pipe), extend Topology to record it once in that pass, so `links` is exact and costs nothing per tick. For pipes, a connection is listed for every orthogonally adjacent pipe tile in the SAME network, for the tank tile a source segment draws from, and for a machine footprint tile a tap or producer touches via that face. For a splitter, `output` is the straight output and `inputs` follows the belt rule; the existing `outputs` field stays.

Tests: new `sim_core/tests/links.rs` built with Purchase+Place commands (see `sim::reference_chain` and `tests/placement.rs` for the idiom): a straight line; a left corner and a right corner; a merge (flank + behind); hopper feeding a belt on each of the four facings; machine outlet into a belt; belt into a dock; splitter inputs/output; a pipe end, straight, corner, tee and cross; a tank connection; a machine tap and a machine producer connection; links updating correctly after Remove and Rotate; and one byte-exact JSON assertion of the documented shape. Run every existing test too — nothing else may change behaviour. Note in DEVLOG which topology gaps you had to fill. If you find real sim bugs around belts/pipes while in there (hand-off, corner, chunk boundary, pipe adjacency), fix them if small and tested; otherwise report them in worker_done for ticket 7.

### Ticket 2 — `task_d7c27a84e0da` (completed)

TICKET 2 — Belt model family. Layer: assets/voxels only (generators, regenerated models, icons, voxgen tests), plus UI_UX.md/DEVLOG/decisions notes. Do NOT edit godot_project/main.gd; ticket 5 wires these in.

Deliver, in `assets/voxels/generators/assets.rs`'s conveyor section, a parameterised belt generator producing the family the plan names: `belt_f0..belt_f3` (straight), `belt_corner_left_f0..f3`, `belt_corner_right_f0..f3`, using the plan's "Belt animation" frame scheme (phase k shifts the cleat/roller pattern by k*32/4 voxels along +X; frames must differ ONLY by that shift) and its "Corner conventions" (output +X; left fed from north/+Y, right fed from south/-Y; a corner's running surface sweeps a quarter turn so it visibly connects the two faces at the belt's running height `BELT_SURFACE_Z`). Every model is authored for a +X product path and must read correctly when rotated to any of the four facings under the fixed (1,1,1) isometric camera — no east-only design is acceptable in this family.

Orientation must be obvious at a glance: put direction chevrons on the rubber running surface, as raised or recessed SHAPES in a contrasting palette entry (UI_UX §7: never colour alone), sized to read at the real scene zoom, not only in a 512px preview. Check that with a temporary render of the real scene (`scripts/screenshot-scene.sh --closeup` after pointing `belt` at your frame 0 — the existing `belt`, `belt_inlet` and `belt_outlet` names MUST keep generating, as frame 0 of the new design, so the scene still renders until ticket 5). Decide the fate of the east-only `belt_inlet`/`belt_outlet` junction variants: either redesign them so they are correct at every facing under UI_UX §2b points 11 and 18, or make the plain tile carry the junction and let the machine-side hoods do the rest — write the decision down (CLAUDE.md condensed + DECISIONS.md full + a new §2b point). If you keep junction variants, frame them too (`belt_inlet_f0..f3` etc.). Bring `machine_splitter` visually into line with the new belt (same surface height, frame style, rollers) without changing its arrow mechanism; do not re-architect it.

voxgen tests (`cargo test -p voxgen`): chevron cells are present on the surface of every frame and point +X; the four frames of a shape differ only by the phase shift; a corner's lanes are open on exactly its two connected faces; every junction opening is genuinely empty voxels. Run `scripts/build-voxels.sh` and regenerate icons (`cargo run -p voxgen --bin voxicons`); commit the models and icons. Read the preview PNGs for every new model and the closeup scene render; describe what you saw in worker_done. Add a UI_UX §2b point recording what only the render could tell you.

### Ticket 3 — `task_660d729ab175` (completed)

TICKET 3 — Pipe model family and pipe visual bugs. Layer: assets/voxels only (generators, regenerated models, voxgen tests), plus UI_UX.md/DEVLOG/decisions notes. Do NOT edit godot_project/main.gd; ticket 6 wires these in.

Start by finding what is actually wrong: run `scripts/build-voxels.sh` and `scripts/screenshot-scene.sh --closeup` (and `--no-hud`) on the current build and read the PNGs of the pipe runs (tank-fed vinegar run into the mixer; mixer->vat and vat->bottler runs). Known suspects: `pipe_inlet` and `pipe_source` are authored for one arrangement and fall back elsewhere; the blind flange caps the wrong end; the bore is open at run ends so daylight shows through; the one-flange-per-tile rule and the sight window do not survive a corner; fittings meet machines badly. List every defect you see in DEVLOG and fix each in this ticket.

Deliver, per the plan's "Corner conventions", the shape family in `assets.rs`'s pipe section: `pipe` (straight, keep the name, improved), `pipe_corner` (open on +X and +Y), `pipe_tee` (open on +X, -X, +Y), `pipe_cross`, `pipe_end` (open on -X only, capped on +X), and composable per-face fittings authored on the +X face that Godot can overlay on any shape: `pipe_fitting_machine` (the nozzle/flange that meets a machine's `machine_fluid_port`) and `pipe_fitting_tank` (the tank connection, with the hand wheel if you keep it). Each shape needs a matching one-voxel-tall fluid body (`pipe_fluid`, `pipe_fluid_corner`, `pipe_fluid_tee`, `pipe_fluid_cross`, `pipe_fluid_end`) that fills the bore's footprint so Godot can scale it to the live level (UI_UX §2b point 5, and `main.gd`'s PIPE_BORE_* constants — if you change the bore floor or height, say so loudly in worker_done because ticket 6 must update those). Every model must read correctly at every rotation under the fixed camera; the sight window must exist on every shape in a place the camera can see. Keep `pipe_source` and `pipe_inlet` generating for now (mark them deprecated in their doc comments; ticket 6 deletes them) so the scene still renders.

voxgen tests: each shape's bore is open exactly on its declared faces and closed on the others (assert the boundary voxels); each fluid body lies inside its shape's bore; a fitting's flange sits on the tile boundary at PIPE_AXIS height. Read the preview PNG of every new model and a closeup of the real scene with the improved `pipe`; describe what you saw in worker_done. Add UI_UX §2b points for what only the render could tell you.

### Ticket 4 — `task_89c69e73b6f1` (completed)

TICKET 4 — Instanced rendering for belts, pipes and items (performance). Layer: godot_project only (main.gd, tests, screenshot script), plus DEVLOG/decisions notes. Do NOT change the sim or assets. Do not add corners, shapes or animation — tickets 5/6 do that ON TOP of what you build, so build the seams they need.

Today `_rebuild_world` frees and rebuilds one Node3D (with a MeshInstance3D per material class) per placement, and `_render_carried_items` keeps one pooled Node3D per belt slot and moves it every frame. `_build_floor` already draws the floor as one `MultiMeshInstance3D`. Make belt tiles, splitter tiles, pipe tiles and carried items instanced the same way: one MultiMesh per (asset, material class), transforms set per tile; machines and buildings keep their nodes (they carry lamps, gauges, fill bodies). Preserve every observable behaviour: `_asset_for_placement` still decides the asset per tile; hover, occupancy, ghost, removal highlighting, splitter arrow, pipe fluid bodies, hopper/tank fill bodies all keep working; the per-TILE item hiding rule (see CLAUDE.md's splitter ticket-2 entry) is preserved; a place/remove/rotate still fully rebuilds (that is fine once it is cheap).

Build these seams for later tickets and document them in main.gd: (a) a tile asset may be a FRAME SET — a list of asset names — with `_set_tile_frame(frame: int)` that toggles which per-frame MultiMeshInstance3D set is visible, so animating a thousand belts is a few visibility flips (see the plan's "Belt animation"); until ticket 5 the set has one frame. (b) per-tile transform = position plus yaw supplied by the caller, so a corner tile can be rotated independently of the placement's nominal direction. (c) items are positioned through one function taking (tile, direction, distance) that ticket 5 will extend with corner arcs — keep the straight-line maths there.

Measure. Add a stress fixture to `test_layouts.gd` (a serpentine belt line of about 1500 tiles fed by hoppers, plus a few multi-tile pipe runs, running) and a `--stress` flag on `scripts/screenshot-scene.sh`; add a headless `test_perf.gd` that builds it and times `_rebuild_world` and 120 frames of `_process` with `Time.get_ticks_usec`, printing the numbers and asserting a generous ceiling. Record before/after numbers in DEVLOG. Verify every existing `test_*.gd` passes via `scripts/verify-core-loop.sh` and that `scripts/screenshot-scene.sh --populate` before and after is visually identical (read both PNGs). Record the decision (why MultiMesh, what stays a node) in CLAUDE.md condensed + DECISIONS.md.

### Ticket 5 — `task_e8b808641bf2` (completed)

TICKET 5 — Belt presentation: shape from `links`, four-facing junctions, animation, items through corners. Layer: godot_project (main.gd, tests, screenshot script) plus docs. Tickets 1 (the `links` wire field), 2 (the belt model family) and 4 (instanced tile/item rendering with frame sets) are MERGED on the integration branch you were branched from — read their DEVLOG entries and CLAUDE.md decisions before starting, and build on ticket 4's seams rather than around them.

Do: (1) In `_rebuild_world`/`_asset_for_placement`, derive each belt tile's shape from `links` per the plan's "Shape rules" (straight / corner left / corner right / merge / line start / line end) and pick the asset and the yaw accordingly; splitters keep their own model. Delete the `direction != "east"` fallback and every east-only comment about belts in main.gd; whatever ticket 2 decided about junction variants, apply it so a belt meeting a machine, hopper or dock looks connected at ALL four facings. (2) Animation: each belt shape is a 4-frame set; every frame, show frame `floor(current_tick * 4 / 50) mod 4` (BELT_TICKS_PER_TILE is 50, `sim_core/src/sim/belt.rs`) via ticket 4's `_set_tile_frame`, so the surface moves at item speed, along each tile's facing, and freezes when the pause menu is up. (3) Items on a corner tile follow a quarter-circle arc from the input face midpoint to the output face midpoint parameterised by the sim's 0..1 distance (extend ticket 4's item-positioning function; straight tiles unchanged). (4) Hover text for a belt names its inputs and output in words ("fed from the west · hands to Crusher"; "not connected" where null). (5) The build ghost: compute the would-be shape from the neighbours' `links` on the Godot side for the ghost only, or keep a straight ghost — decide, and record why. 

Tests: a new headless `test_belt_shapes.gd` (built with real commands via `test_layouts.gd`): corner-left and corner-right picks at all four facings with the expected yaw; merge and line-end shapes; frame index for given ticks and frozen while paused; item arc positions at distance 0, 0.5, 1 on a corner; hover text. Keep `test_connections.gd` honest — update the assertions that pinned the east-only fallback. Add `--belt-gallery` to `scripts/screenshot-scene.sh` (a layout with corners and machine junctions at every facing) and READ the PNGs at scene zoom and closeup; fix what reads wrong (if the fix is in a generator, make it, regenerate, commit the models). DEVLOG entry, CLAUDE.md condensed decisions + DECISIONS.md, UI_UX §2b points; update `world.rs`/`machines.ron` doc comments that still say junction art is east-only.

### Ticket 6 — `task_617a8113533e` (completed)

TICKET 6 — Pipe presentation: shape from `links`, fittings, fluid body per shape, hover. Layer: godot_project (main.gd, tests, screenshot script) plus docs, and deletion of the deprecated `pipe_source`/`pipe_inlet` generators and models in assets/voxels. Tickets 1 (the `links` wire field), 3 (the pipe model family) and 4 (instanced tile rendering) are MERGED on the integration branch you were branched from — read their DEVLOG entries and CLAUDE.md decisions first, and build on ticket 4's seams.

Do: (1) Replace `_pipe_roles` entirely: derive each pipe tile's shape from `links.connections` per the plan's "Shape rules" (end / straight / corner / tee / cross) and compute the yaw that maps the authored faces (corner: +X,+Y; tee: +X,-X,+Y; end: open -X) onto the tile's connected faces; overlay `pipe_fitting_machine` / `pipe_fitting_tank` on each face whose connection kind is machine / tank, rotated to that face. Instance pipes through ticket 4's tile renderer. (2) Fluid bodies: use the matching `pipe_fluid_*` body per shape, positioned and scaled by the live level exactly as today (PIPE_BORE_* constants — reconcile them with whatever ticket 3 reported). (3) Delete `pipe_source`, `pipe_inlet`, every reference to them, their generators, their model files and index entries; `machine_fluid_port` on machines stays. (4) Hover text for a pipe names its connections in words ("connects west and north · fed by Vinegar Tank · feeds Mixer"). (5) The ghost for a pipe: same decision as ticket 5 made for belts — read its DEVLOG entry and match it.

Tests: a new headless `test_pipe_shapes.gd`: each shape and rotation from a real placement (end, straight both axes, all four corner rotations, four tee rotations, cross), machine and tank fittings on the right faces, fluid body asset per shape, hover text; update `test_fluids.gd`/`test_chain_rework.gd` where they assert the old roles. Add `--pipe-gallery` to `scripts/screenshot-scene.sh` (every shape at every rotation, a tank feeding from each of four sides, a machine tapped from each side, the shipped mixer->vat->bottler runs) and READ the PNGs at scene zoom and closeup; fix what reads wrong (generator fixes are in scope — regenerate and commit models). DEVLOG entry, CLAUDE.md condensed + DECISIONS.md, UI_UX §2b points; update main.gd/world.rs comments that still describe the old fallback.

### Ticket 7 — `task_15907b4d0158` (completed)

TICKET 7 — Bug hunt, polish and documentation close-out for the Pipes & Conveyors Overhaul. Layer: anything the defects need. Tickets 1–6 are MERGED on the integration branch you were branched from; read `docs/PIPES_AND_CONVEYORS_PLAN.md`, every 2026-09-09/10 DEVLOG entry from this update and the worker notes the coordinator passes in this spec, then hunt.

Do: (1) Add `build_gallery` to `test_layouts.gd` and `--gallery` to `scripts/screenshot-scene.sh`: every belt shape at every facing; hopper->belt, belt->each machine kind, belt->dock and belt->splitter branches at every facing; every pipe shape at every rotation; tank->pipe from four sides; machine taps and producers from four sides; the whole shipped chain running (`build_reference_line`) plus one deliberately awkward serpentine line with corners in both hands. Render the gallery at scene zoom and with `--closeup` per region, including with the sim populated and mid-animation, and in first-person (`--first-person`). READ every PNG. (2) List every visual defect (z-fighting, seams, floating voxels, openings that are not empty, wrong rotations, items clipping a corner, fluid bodies poking through walls, chevrons unreadable, frames popping, fittings misaligned, junction hoods facing the wrong way, splitter mismatch with the new belt) in a checklist in DEVLOG and fix each — generator, main.gd or sim as appropriate, with a test where one is possible. (3) Performance sanity: run `test_perf.gd` and `--stress` and confirm the numbers ticket 4 recorded still hold with shapes and animation on; fix regressions. (4) Run the whole suite: `cargo test --workspace`, `scripts/verify-core-loop.sh`, `scripts/build-voxels.sh`, `scripts/screenshot-scene.sh` default/--populate/--stress/--gallery. (5) Docs close-out: DEVLOG "Current status" updated with a "Pipes & conveyors overhaul (DONE)" bullet; the plan doc's status line set to done with a short "what shipped / what is still open" section; ARCHITECTURE.md §11 and UI_UX.md §2b point 18 updated so they no longer describe junction art as east-only if that is now false; CLAUDE.md condensed decision + DECISIONS.md full entry for the overhaul as a whole; any stale comments in main.gd/world.rs/assets.rs/machines.ron that describe the pre-overhaul fallbacks. Report the remaining known gaps explicitly in worker_done.
