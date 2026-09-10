# Pipes & Conveyors Overhaul — plan

Status: in progress (started 2026-09-09; tickets 1–6 merged by 2026-09-10 morning, ticket 7 not started). Integration branch:
`TomWhitticase/pipes-and-conveyors-overhaul`. Built as one Orca
orchestration Run; tickets below map 1:1 to Tasks.

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
  `BELT_FRAMES` = 4 and the arrow mark stands at `BELT_SURFACE_Z + 1`.
  Corners were render-checked only through a temporary `main.gd` hack —
  ticket 5 must look at them at all four facings once it draws them.
  `voxicons` renders icons for frame 0 only.

## Coordinator notes (appended as wave 1 lands)

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
- The straight tile's arrow reads at all four facings in ticket 2's
  close-ups; the south facing was never rendered — include it in the belt
  gallery.

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
Ticket 4's finding that ~8 ms of every tick step is chunk-snapshot JSON
parsing on a 1500-belt line is milestone 8 scope, not this update's.

**State at the end of 2026-09-10's first session:** tickets 1–6 are merged
and verified on the integration branch (`cargo test --workspace`,
`scripts/verify-core-loop.sh` with 24 headless proofs, models and icons
regenerated with no drift). Ticket 7 is `ready` in the Run and not
dispatched.

**For ticket 7, gathered from tickets 5 and 6 and the merges:**

- UI_UX §2b numbering after the merges: 23 belt arrow/frames (ticket 2),
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
- The `resolve_sink` inlet-edge rule from ticket 1 is still untouched.
- Every worktree grows an untracked `AGENTS.md` (a CLAUDE.md copy) from the
  environment; nobody has committed it. Decide whether to `.gitignore` it.
