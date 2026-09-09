# Pipes & Conveyors Overhaul — plan

Status: in progress (started 2026-09-09). Integration branch:
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

