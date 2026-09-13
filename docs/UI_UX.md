# Sauce Factory — UI/UX & Visual Style Plan

This doc covers **aesthetics and UI/UX only** — how the game looks and
behaves to the player. Mechanics are [GAMEPLAY.md](GAMEPLAY.md); how it is
built is [ARCHITECTURE.md](ARCHITECTURE.md). Full decision records, including
the long form of §2b, are in [DECISIONS.md](DECISIONS.md).

---

## 0. At a glance

| Aspect | Direction |
|---|---|
| Camera | Fixed orthographic isometric, tile-based grid |
| Art medium | Full 3D voxel models (not flat-textured cubes) |
| Asset scale | Every tile = a 32×32×32 voxel volume |
| Theme | Grounded, semi-realistic 21st-century industrial food factory |
| Lighting | Real-time lighting, soft shadows, light shaders — "modded Minecraft with shaders" |
| Materials | Mostly matte; metals deliberately shinier |
| UI | Clean, Minecraft-adjacent; diegetic clipboard/checklist motifs |

Reference points: **Crossy Road** (chunky voxel readability), **The Sims** /
**Townscaper** (fixed dimetric dollhouse camera), **modded Minecraft with a
shader pack** (voxel base + real lighting).

## 1. Camera & world presentation

- **Fixed orthographic isometric camera.** No perspective, no free orbit.
  The scene uses a true isometric `(1, 1, 1)` view direction, one constant
  in `main.gd` (`CAMERA_DIRECTION`).
- **Tile grid always legible**: the player can always tell which tile a
  thing occupies.
- **Rotation**: 90° snap only (not yet implemented). Assets are modelled for
  four fixed angles; free rotation would expose un-art-directed angles.
- **Zoom**: smooth, within a capped range.

## 2. Art medium: voxel models, not textured blocks

Assets are **full 3D voxel models** (MagicaVoxel-style), so machines and
props have real shapes, not cubes with a picture on each face.

- **Scale**: one tile = a 32×32×32 voxel volume. A kind's model size equals
  its `machines.ron` footprint × 32 (the 2x2 vat is 64×64, the 1x2 mixer
  32×64). Size is content.
- **Authoring**: palette-based; one shared palette keeps assets consistent.
- **Silhouette-first**: design to read from the fixed camera angles.

### 2a. Asset pipeline

Researched, not guessed. The driving constraint: every asset must be
creatable, editable and viewable by Claude, not only by a human in a GUI.

Findings: `.vox` (MagicaVoxel) is the de facto voxel format with an open
spec; Godot 4 has community `.vox` importers; [`vengi`](https://github.com/vengi-voxel/vengi)
has a headless CLI that renders a model to an isometric PNG; Rust `.vox`
writing support is immature.

**The pipeline:**

1. **Source of truth is code.** Each asset is a small parameterized Rust
   generator function built from primitives. Rust, not a second scripting
   language.
2. **The `.vox` file is a generated build artifact, never hand-edited.**
   Change the generator and re-run it.
3. **Viewing is a headless render**: render to an isometric PNG with vengi
   and read the image. No GUI, no computer-use loop.
4. **Escape hatch**: a one-off hand-sculpted `.vox` (in `vengi-voxedit`) may
   be committed with no generator, clearly marked as hand-authored.
5. **One shared, checked-in colour palette** referenced by every generator.
   This is what keeps §4's material rule and §7's palette consistent.

Layout (built exactly this way):

```
assets/voxels/
  Cargo.toml        # crate `voxgen` — `cargo run -p voxgen`
  palette.ron       # shared palette: colour AND material class
  generators/       # one generator per asset, plus vox.rs, icons.rs
  models/           # generated .vox — committed build output
  icons/            # generated UI icons — committed
  previews/         # review PNGs — gitignored
```

### 2b. How the pipeline actually turned out — rules learned by building it

Point numbers are cited by code comments; keep them stable. Full narrative in
DECISIONS.md Part B. There is no point 10.

1. **The `.vox` encoder is hand-written** (`generators/vox.rs`). No Rust
   crate writes the scene-graph chunks the Godot importer needs. ~150 lines,
   and it also emits `MATL` chunks. The encoder puts a model's origin at its
   footprint centre at ground level.
2. **One asset = one mesh per material class.** An imported voxel mesh has
   one material, so `voxgen` writes `name.vox` (preview) plus
   `name.<class>.vox` per class; Godot rebuilds one `StandardMaterial3D` per
   class from `materials.json` (exported from `palette.ron`).
3. **Headless review:** `vengi-thumbnailer -i model.vox -o out.png -s 512
   --image --isometric`. `vengi-voxconvert -o out.png` writes one PNG per
   Z-slice, not a thumbnail. The thumbnailer needs a real video driver
   (`SDL_VIDEODRIVER=dummy` fails). `scripts/build-voxels.sh` wraps it.
4. **Godot needs `--import` twice on a cold cache.** The `.vox` importer is
   an editor plugin registered only on the pass after the first scan.
5. **A liquid is its own material class** (`liquid`, glossy dielectric,
   roughness 0.14). The fluid's hue is NOT palette data: which liquid is in a
   pipe is live sim data, so the fluid model is near-white and `main.gd`
   tints it. The fluid model is **one voxel tall**, scaled vertically to the
   live fill fraction; because of point 1's origin rule no pivot maths is
   needed. The same mechanism draws the hopper fill, the tank level and any
   progress gauge — do not invent another. **The tint value itself is
   palette data even though the model's hue is not:** a fluid that also has
   a still model (vinegar's jug, point 27) would otherwise be one colour in
   `palette.ron` and another in `main.gd`, and the two are seen side by side
   the moment a market icon sits above a pipe. `FLUID_TINTS[id]` is the
   palette entry's RGB, written as such in a comment beside it.
6. **Pipes are stainless steel, hollow, with an open inspection channel**
   starting just above the bore floor, so the level is visible below half
   fill. Steel, not PVC: §4 makes metal the shiny exception, and
   GAMEPLAY.md tier 5 reserves special piping as a later gate.
7. **UI icons are a second render pass** (`generators/icons.rs`,
   `cargo run -p voxgen --bin voxicons`), not shrunk previews: the
   thumbnailer has no transparent-background option, so icons are rendered,
   flood-filled to transparent from the border, cropped and downscaled to a
   64px square. Output `assets/voxels/icons/`, committed.
8. **Material rule for machines: food contact is stainless (`metal`),
   structure is painted or poured (`matte`).** An all-steel machine reads as
   flat; matte structure gives the metal something to pop against. Emissive
   is for status lamps only.
9. **Review the rendered asset, not the generator, and budget a rework
   pass per asset.** Looking changes the design (a crusher throat too deep
   to see into; a gearbox that hid the agitator).
11. **Only two of a machine's four vertical faces can ever be seen: +X and
    +Z-world.** So an OUTLET is a doorway carved through the +X face, and an
    INLET is a hooded transfer housing whose CAMERA-FACING side is cut away
    (a doorway on the true inlet face is invisible). Both read by value
    contrast (brightest frame, genuinely black opening, one brass
    threshold); a mid-grey interior or a hazard stripe fails at scene scale.
    A machine tall enough to stand between its inlet and the camera (the 2x2
    vat) needs its opening widened to an unoccluded corner. A neighbouring
    building can hide a correct junction.
12. **The full-scene render catches bugs no headless test can.** Any
    presentation lookup keyed by a content id (`ITEM_MODELS`,
    `MACHINE_MODELS`) has a fallback, so a stale key ships the wrong model
    with every test green. `scripts/screenshot-scene.sh` is the check.
13. **A process an order of magnitude longer than a craft cycle needs a
    passive gauge, and on this camera it must be emissive** (both visible
    faces are the shadow side). An empty gauge must show its own scale: a
    lit finish line at the top, a position rather than a colour (§7).
    `MACHINE_PROGRESS_GAUGES` is keyed on kind and sparse: a bar on a 60-tick
    cycle is a flicker.
14. **Wood (`palette.ron` `oak`) is the one exception to point 8.** In a
    barrel-aging room the wood IS the process. Do not "fix" it to steel.
15. **A machine with no mechanism is a legitimate silhouette** when what
    distinguishes it is that it only waits. Casks stand on end: lying down
    they recede into the tile and read as corrugation.
16. **A building with an open floor follows the depth rule: put the height
    at high y, put the content at low y.** A sight line travels
    `(+x, -y, +z)` in model space, so a tall structure on the near side walls
    off everything behind it. The delivery dock is architecture (raised
    platform, striped edge, bumpers, despatch doors under a canopy), with
    the building on the FAR tile and the near tile open apron. It is 2x2
    since 2026-09-09; the extra tile went into depth. A machine's infeed
    hood claims its tile corner, and a wall drawn through it is a hole, so
    the despatch wall starts east of that corner. Lessons: a large block in
    `concrete_dark` reads as a pit; a canopy on thin posts reads as a
    floating shelf.
17. **What stands on the dock is drawn from `delivery_stock`, not from the
    dock**, whose own buffer empties every tick. One case per unit, with the
    item's own model, capped at four (a 2x2 block). The exact figure is on
    the clipboard and in hover.
18. **A rotated placement needs no new geometry: the same mesh is spun about
    its footprint-centre origin.** The exception is point 11's junction
    art: spinning a carved +X doorway 180° puts the opening where nobody can
    see it, visibly worse than the plain tile. The belt's
    `belt_inlet`/`belt_outlet` variants were **retired** by the pipes and
    conveyors overhaul (point 23): the plain belt tile carries the junction
    and the machine's own hood and doorway do the rest, at every facing.
    Pipes no longer have this gap either: a pipe tile is a shape plus
    overlaid fittings, each turned to the side its `links` name (point 25).
19. **A live body scaled inside a hollow shape must be strictly narrower
    than the shape's OUTER wall at every height it can reach.** The pipe's
    fluid slab was as wide as the bore; where the tube curves in toward
    the crown its outer wall is that wide too, and the two z-fought as a
    coloured stripe along every closed tile. Only a scene render with
    liquid in the run shows it.
20. **Anything Godot spins must be symmetric about the tile midline,
    between voxels 15 and 16.** An integer-centred axis (voxel 16) lands
    on the other side of the midline after a turn, so a run at mixed
    facings steps at every joint. Use the `*_f` primitives with centre
    15.5 for pipes, belts and junction art.
21. **A dark accent on a camera-facing shadow-side face reads as a hole**
    (the converse of point 11). The end cap's brass disc rendered as the
    open mouth it closed; bolts on a bright plate keep the accent.
22. **A composable fitting is an overlay: it keeps out of every shape's
    sight window (tube-height hardware at x ≥ 27; leg windows at 20..26)
    and carves its bore a little wider than the shape's so the two meshes
    never share a surface.** `vengi` previews look from the -X/+Y corner,
    which the game never shows, so +X-face hardware is judged only in the
    scene.
23. **A thin diagonal mark on the tile plane does not survive this camera;
    a solid shape does.** The isometric view looks straight down one tile
    diagonal, so a line along one diagonal projects to a full-length screen
    horizontal and a line along the other to a short screen vertical: a
    two-arm `>` chevron rendered as an `L`, and could not point anywhere at
    any facing. The belt's mark was therefore a **filled** arrow (wedge
    head, short shaft) in `hazard_yellow` until 2026-09-10, when the player
    asked for the arrows to go: the mark is now a **plain cleat**, a bar of
    dark rubber two voxels long across the lane, and the belt's direction is
    given in words (the build prompt's `facing east`, hover's `carries
    east`) and by the running surface itself. The camera lesson stands for
    any mark anyone draws next. **Raised, not recessed**: one edge of any
    recess runs along the sight line and shadows its own floor. Belts
    animate by **frames**:
    frame `k` is the same tile with the surface pattern advanced `k` quarter
    tiles along the path and nothing else changed; the pattern's period is
    one tile (a shorter one makes frames coincide or run backwards). A
    corner's surface is a quarter annulus about the tile corner between its
    two open faces, whose pattern is stretched to a tile's worth so it
    reaches the exit face in the same frame a straight would; its cleat
    bends with the belt. Idlers, rails and legs are structure and never
    move. The plain tile carries every belt-to-machine junction (point 18).
24. **A fitting overlaid on a shape must be a full voxel of radius larger
    than anything it can cover** (`PIPE_FITTING_MARGIN`). Two voxel solids
    z-fight wherever a voxel of both has an outward neighbour in neither;
    a one-voxel margin puts every axis-neighbour of an inner voxel inside
    the outer solid. Found where the machine fitting lands on a straight's
    own joint flange; pinned by `cargo test -p voxgen` on both faces.
25. **A pipe tile's turn is a property of what it is joined to, never of
    its facing.** One joint flange per straight only reads as one join if
    every tile along an axis keeps it on the same side (east-west turn 0,
    north-south turn 1); an end's turn is dictated by its open side, so it
    carries no joint flange at all. R on a pipe visibly changes only a tile
    joined to nothing.
26. **A uniform offset in a path convention is invisible on a straight
    line and shows up the moment the line bends.** Items were drawn from a
    tile's centre to the next tile's centre — half a tile downstream of the
    simulation's own input-face-to-output-face distance — and every render
    of a straight line looked continuous, because every tile shared the
    shift. A corner cannot share it: the arc has to leave through the
    output face, and drawn half a tile late it left through the wrong one.
    Items are now drawn face to face on every tile, so an item appears at a
    hopper's mouth and vanishes at a machine's face. The general rule: check
    a path convention against the tile's edges, not against its neighbours,
    because neighbours hide a shared error.
27. **A machine is animated by a separate motion model in `MACHINE_FRAMES`
    arrangements, and the moving parts are CUT OUT of the body rather than
    duplicated into the overlay.** Point 23's frame mechanism applied to
    something that is not a tile: the crusher's roll teeth and flywheels,
    the mixer's ribbon, the vat's agitator spider, the bottler's nozzles and
    capping chuck and the packer's taping carriage all live in
    `<machine>_motion_f0..f3`, cut on the machine's own footprint so the
    overlay and the body share an origin and Godot needs no offset. All four
    stand beside the body as siblings and exactly one is visible. Two voxel
    solids that overlap z-fight (point 24), so the body cannot keep a copy
    of what moves; `cargo test -p voxgen` compares every overlay voxel
    against its body. Every motion volume is an EVEN number of voxels tall,
    because `vox::encode` lifts a model by `size[2] / 2` in whole voxels and
    an odd height sinks the overlay half a voxel into the body it sits on.
    The frames must differ CONSECUTIVELY, not pairwise: a part that
    reciprocates passes through the same pose on the way out and on the way
    back, and only a turning part has to show four distinct angles.
28. **Motion means "working", and working is narrower than "has a batch".**
    A mechanism turns only while `0 <= elapsed < required`, so a machine
    that has finished and is blocked on a full output stands still while its
    status lamp stays lit — lamp on and nothing turning is the readout for
    "the thing downstream is full", which neither signal gives alone. The
    pace is one cycle per 30 ticks for EVERY kind: driving it from a
    machine's own process length would make the 240-tick vat read as a
    different factory from the 80-tick crusher, and how long a stage takes
    is already stated by the product backing up behind it. **No steam,
    deliberately** — a puff is mass a stopped vat should not have, and a
    frozen cloud over a cold vat states the opposite of what the lamp says.
27. **A model whose only consumer is the icon pass must be authored for
    vengi's camera, not the game's.** Point 11 ("only +X and +Z-world are
    ever seen") is a rule about the *scene*; point 7's icon render goes
    through `vengi-thumbnailer`, which looks from the **-X/+Y** corner
    (point 22) — the two cameras see opposite sides of the same model. An
    asset that appears in both places has to read from both: `item_box`'s
    label, authored on the faces the game shows, is invisible in its own
    icon, and nobody noticed because the icon still read as a box. A
    **fluid** is the pure case of the other extreme: a fluid never rides a
    belt and is never placed, so its model exists only to be an icon, and
    its label, handle and fill line belong on the faces vengi shows. Decide
    which cameras a new model will actually meet before deciding where its
    detail goes, and render it to check — a face nobody sees is the same
    cost as one nobody drew.
27. **The floor is the one surface whose job is to be a background, so it
    is authored by its VALUE RANGE, not by its detail.** Every other asset
    is judged alone; the floor is judged as thousands of copies filling the
    frame, and at that scale anything with contrast becomes the loudest
    thing in the picture. Three failures, each found only by rendering an
    empty world (the state a new game starts in, and almost entirely
    floor): a near-black joint on all four tile edges read as bathroom
    grout, not a poured slab; single-voxel `concrete_light` speckle read as
    television static; and a repair patch filled with the shared `concrete`
    tone measured +16 render units against a floor whose whole variation
    was ±2, so it read as white paving slabs dropped at random. The rules
    that came out of it: keep the pour's own tones within a few points of
    each other (§7's neutral base is a *narrow* band here, not just a
    desaturated one); make variation BROAD and low-contrast — blotches with
    wandering outlines, never per-voxel noise, which also greedy-meshes into
    far fewer triangles on the one asset whose triangle count multiplies by
    the size of the world; and **never let the floor hold the brightest
    pixel in the frame**, because §4 reserves that for metal. A feature that
    genuinely wants contrast (a drain grating read by its stripes) is the
    exception and should be a real object, not a texture.
28. **N models laid over thousands of tiles is still only N stamps; the
    multiplier is per-instance rotation, and it is only free if the tile's
    edges are four-fold symmetric.** Four floor variants looked varied in
    isolation and repeated visibly in a magnified render — every worn tile's
    scuff ran the same way across the screen, every plain tile carried its
    blotches in the same spots. Laying each instance at a quarter turn chosen
    by a hash of its coordinates breaks all of that at once, at no asset
    cost. The constraint it imposes is on the EDGE: whatever the tile draws
    at its border must survive a quarter turn, or the seam steps wherever two
    differently-turned tiles meet — the same failure point 20 records for an
    integer-centred axis. So the saw-cut joint stays on all four edges even
    though that doubles an interior line, and the joint's colour was
    lightened until the doubled line still read as a hairline. The choice of
    variant and of turn must be a pure function of the tile coordinate, so a
    reloaded save is not repaved and a rebuild does not reshuffle the floor
    under the player. The one model that must NOT be turned is the one whose
    feature spans tiles: a row of trench drains is a trench only while every
    tile lays its channel on the same axis.
29. **A free-standing thing at a fixed place on a tile meets every body at
    every facing, and a face one voxel from a tile edge is a face one voxel
    from EVERY edge once the body turns.** The andon post (removed on
    2026-09-12 — the lesson stands) stood at the tile's +X/low-Y corner,
    inset one voxel, and never rotated; the machines it was bolted to do. Two features that had sat harmlessly at
    the far side of their bodies — the crusher's motor one voxel from its
    +Y edge, the packer's status lamp one voxel from its -X edge — landed
    exactly on the post's own faces when the body was turned, and z-fought
    it (point 24) in a way no preview could show, because a preview shows
    one facing. The rule: an overlay that does not turn with its host is
    checked against the host turned all four ways, by a test, not by eye;
    and the check is on co-located voxels with a face exposed by both, which
    is the whole of point 24 in one predicate. The corollary for the inset
    itself: on the tile edge it fights the flush plinth, two in it fights
    the painted skirt, so one voxel is a pinned number and not a choice —
    the post passes THROUGH a body and never fights one. The last lesson is
    the lens ring's size: `VoxModel::set` clips silently at the volume, so a
    ring about a point two voxels from two edges has radius two at most, or
    it is flat on exactly the faces the camera sees.
30. **A per-tick fact shown beside a windowed verdict has to be held across
    the ticks it is absent.** The load wire carries a `status` judged over
    the last 600 ticks and a `stall` that is this tick's reason — `null` on
    most ticks of a blocked machine, which is mid-craft on most of them. A
    hover printing exactly this poll's `stall` beside an unchanged `status`
    flickers between "blocked · output full" and "blocked", which reads as
    the reason going away. The scene keeps the last non-null reason while
    the status holds and forgets it when the status changes, and a test
    asserts it is never dropped across hundreds of polls. When one polled
    field explains another, the explanation's lifetime is the explained
    thing's, not the poll's.
31. **At icon scale an object is identified by the proportion of its
    parts, not by their presence.** The hard hat's first cut had every
    part a hard hat has — brim, front peak, domed crown, a crest over the
    top — and its 64 px icon was a bell, because the crown stood taller
    than it was wide; correct detail on a wrong ratio reads as the wrong
    thing. A low crown, wider than tall, fixed it with fewer voxels and
    fewer features. So judge an icon at its real size on the real plate
    before judging the preview: the preview shows detail, which is what the
    eye lands on and not what the icon is read by.

32. **A fixed-size card clips a label without complaint, and a taller
    card is not always available to fix it.** The build-menu card gained
    one line (`crew: 2 manufacturing`) and the first render cut it at
    "manufacturi" on the right and pushed `Owned 0` off the bottom; every
    headless test read the full string and passed (point 12 again — a
    Label's text is intact whatever the card shows of it). The obvious fix,
    a taller card, made the two-row grid scroll, because the menu is capped
    against the hotbar (§5c), and a wider card alone left the bottom line
    clipped. What worked was paying for the line out of the card's own
    slack: two less inset top and bottom, no separation, the smallest font
    step the scale allows, and a card wide enough for the longest crew
    string. The rule: a new line on a card that lives in a capped grid is
    bought from the card's padding, not from the grid's height, and the
    check is the rendered frame at the shipped window size.

## 3. Theme & setting

**A grounded, semi-realistic 21st-century industrial food factory**:
stainless tanks, rubber belts, steel piping, poured concrete, hazard
striping, pallets, loading docks. Lean into real industrial visual language
(signage, cable runs, grates, walkways). Avoid candy-coloured or cutesy
factory tropes; the sauce theme comes through in what the factory processes.

## 4. Materials & lighting

- **Baseline materials are simple and mostly matte**: painted metal,
  concrete, plastic, rubber.
- **Metals are the deliberate exception**: stainless tanks, housings, pipes
  and fittings read noticeably shinier, so machinery pops.
- **Lighting**: real-time dynamic lighting, soft shadows, mild bloom on
  emissives, light SSAO. Reference: a modded-Minecraft shader pack, not
  photorealism.
- **Emissives**: status lights, screens and strobes glow — both aesthetic
  and a readability signal (§7).
- **Atmosphere**: steam/smoke from cooking equipment, visible liquid
  surfaces with subtle reflection.

## 5. UI style

> **Status: built.** The contract readout is a diegetic clipboard; machine
> detail is hover-only; the build menu (E) and market (M) are grids of
> voxel-rendered icon slots; the hotbar is the primary way anything is
> picked up (§5b); the main and pause menus are §5d. Other readouts use the
> plain dark plate: the clipboard motif is reserved for orders and
> checklists.

- **Clean, simple, Minecraft-adjacent**: chunky panel borders, flat-shaded
  slot grids, limited palette per panel, generous hit targets. No modern
  skeuomorphic-app UI (gradients, drop shadows, hairline fonts).
- **Icons are rendered from the real voxel assets** (§2b point 7), never
  separately-drawn 2D art.
- **Several contracts are one board of dockets, not one clipboard each.**
  One sheet, one clip bar, one printed docket card per contract (the
  checkbox's paper tone plus a steel hairline, no third colour). Shared
  facts (money, the delivery pool) appear once above the dockets;
  per-contract facts (checklist, payout, countdown, delivered/missed) on
  each docket. Past a capped height the stack scrolls.
- **Diegetic paper-and-metal widgets**: orders and objectives are clipboards
  and checklists. A contract IS a clipboard.
- **The contracts board (C) is a dark-plate menu of paper offers.** What is
  running lives on the clipboard; what is for sale lives on the board, a
  menu like the build menu and the market (centred above the hotbar, one
  modal at a time), whose rows are the clipboard's own docket card with the
  terms in words and one SIGN button that names the price. A SIGN button on
  the clipboard would make every running order look like a thing to click.
  The button says what a click will do or why it will not (**SIGNED**, **✗
  SIGN · 150**), and a lost offer says **LOST — sign it again**.
- **A docket says how it is doing and whether it is in danger, four ways at
  once** (§7): a status line with a glyph and words (**✔ READY**, **⚠ AT
  RISK — 1 short, 70% of the window gone**, **⚠ LAST CHANCE — miss this
  check and it is LOST**), the draining countdown bar of §5e for the time
  half, strike pips (one per strike the contract allows, filled as misses
  run, the rule in words beside them) and a heavier border in the state's
  colour. The
  docket stays paper, ink, its done-green and its one warning red; at-risk
  and last-chance share the red and are told apart by their words, border
  weight and pips. A lost or signed contract is announced in one line along
  the top edge, and an empty clipboard says "press C" rather than showing a
  money figure over nothing.
- **The production panel (P) is the same dark-plate menu, one row per
  item the factory has touched** (the production-stats epic, 2026-09-13 —
  Factorio's P screen, and P for the same reason). A window selector in
  the heading (the sim's own windows, `last 5s` to `last 1h`, plus `all
  time`), then rows biggest first: the item's own icon and name, and two
  §5e bars side by side — MADE in the level green, USED in the amber — each
  with its count and per-minute rate printed on it, scaled against the
  largest count on the list so a glance across rows says which item moves
  most. A caption under the list names the window and, while the factory
  is younger than it, how much has actually been seen (`last 10m (32s so
  far)`), because the rate is over that. An empty window is a sentence,
  not an empty box. The footer says what counts: a craft starting is
  "used", a batch landing is "made", a paid order uses what it ships —
  buying and filling are not production and never appear here.
- **The staff panel (H) is the same dark-plate menu, one row per
  category** (the staff pools MVP, ticket 3, 2026-09-12). Icon, name,
  `hired 4  ·  wage 2 / head / period`, then the §5e bar that this panel
  earns: `assigned / demand` — what the factory asked for on the last tick
  is the simulation's own ceiling, so the bar is honest, and it reads
  `0 / 0` on an idle line rather than headcount against an invented full
  mark. The two numbers are printed ON the bar, `2 short` stands beside it
  in the one warning red while the bar is red (§7: position, digits, word,
  colour), and HIRE / FIRE are the build menu's own buttons, FIRE disabled
  at zero. The footer carries the upcoming bill, the payday countdown in
  seconds (§5e: never ticks) and, only when it happened, `✗ 3 left unpaid
  last payday`. A category nothing asks for yet says so in a sentence
  (`nothing to maintain yet`) in place of an empty bar. E, M, C and H are
  one modal at a time; ESC closes whichever is up. Being short is also
  read where it bites: a refused machine's hover says `▲ starved · … ·
  short of manufacturing staff` over its `crew: 2 manufacturing` line, a
  refused dock says `held · short of logistics staff`, and a build-menu
  card says a kind's crew under its price.
- **A machine's load is read from its hover, and only there** (the
  machine-load epic, 2026-09-11; **amended 2026-09-12**). The in-world
  andon stack lights this bullet was written about were removed after a
  playtest — the owner did not like a pole beside every machine — and
  the hover panel is the whole indicator now: the load line leads with a
  status GLYPH and word (● flat out, ▲ starved, ■ blocked — three shapes,
  §7) and the load bar under it is filled in the status colour (green,
  amber, red). What follows describes the removed lights as they were
  built, kept because the reasoning about corners and colours still
  applies to whatever floor-wide readout replaces them, if one does.
  *Originally:* every
  recipe-running machine has an andon on its camera-visible corner: green
  lit is flat out, amber is starved, red is blocked — colour AND position
  on the post (§7), the industrial convention (§3), never rotated with the
  machine. A line reads as a line: the machine whose light is green while
  the lights after it are amber and the lights before it are red is the
  bottleneck, and that is visible across the whole floor without hovering
  anything. Hover adds the number and the reason, in one line under the
  progress line — `flat out · 100% of the last 20s`, `starved · 38% busy ·
  waiting for Sugar`, `blocked · 61% busy · output full` — and a second bar
  in the §5e language: green, the share of the window spent working, under
  the amber progress bar. The words are the readout; the light and the bar
  are the same fact at two distances.
- **Typography**: a sturdy, slightly blocky sans-serif, readable at UI
  sizes; not a novelty pixel font.
- **Layout**: hotbar along the bottom (§5b); build/recipe menus as icon
  grids; hover tooltips rather than dense always-on text.

### 5a. Build mode: the ghost, and what the next click will do

The world starts empty and the player places every tile, so "what will this
click do" is answered continuously by two things at once:

- **A translucent tinted ghost of the real voxel model** on the tile under
  the cursor, at the chosen facing, including the belt's context variant.
  The tint **multiplies** over vertex colours (a flat wash loses which thing).
- **A prompt beside the cursor** with a glyph and a word: **✔ PLACE**,
  **✗ BLOCKED**, **✗ NONE OWNED**, **✔ REMOVE** (§7: never colour alone).

Conventions: **green means "this click builds"; red means "this click
destroys, or refuses"**, so a valid removal is red. **Removal is a mode (X),
not a modifier-click**: it is the one destructive action, and a mode
announces itself in the legend and the prompt. A build-menu slot has two
actions: **left-click buys one, right-click places one you own**. A hopper's
contents are chosen on the ghost (**Q** cycles raw items; the prompt names
it; the ghost shows a tinted fill).

### 5b. The hotbar: what is in your hand

An always-visible strip along the **bottom** edge, centred (the other edges
hold the legend and the clipboard). One slot per `data/shop.ron` entry, in
file order, built from `shop_snapshot_json`. Above it, a permanent larder
strip shows bought raw stock; zero reads "none".

1. **Fixed slots, not a packed list of what is owned.** Slot 5 must still be
   the crusher after the last crusher is placed.
2. **A kind you own none of is an empty socket**, not a disabled button:
   ghosted icon, no count, recessed.
3. **Selecting and buying are different verbs.** Number keys 1-9 and 0
   select in any mode; the same key again puts it down (mirrors ESC).
   Buying stays in the build menu (**E**) on left-click or SHIFT+number, and
   SHIFT+number works **only while the menu is open**, where price and
   balance are on screen.

Borrowed patterns: a selector bigger than its slot and a fading name flyout
(Minecraft); a framed hover spec sheet with kind, count, price and both verbs
(Terraria); a slot you have none of stays where it was (Factorio).
**Selection is signalled four ways** (§7): the slot grows (a scale, not a
position, since the container owns positions), hazard-yellow frame, a caret,
a brightened key digit. The hotbar plate is nearly opaque (the floor showed
through). The build menu is centred in the space **above** the hotbar so its
footer is never buried.

### 5c. Scale: how big the UI is

Everything is measured in two numbers in `main.gd`: `UI_SCALE` (1.5) and
`UI_FONT_BASE` (15). Font sizes are offsets from the base; chrome multiplies
through the same knob (`_ui_px/_ui_i/_ui_v/_ui_font`). **Explicit constants,
not Godot's `content_scale_factor`**: the scene maps 3D to screen coordinates
in half a dozen places, and each would need the factor divided back out.

What a scale change costs is **layout**, not type size: the build menu is six
columns with a shorter card; panels put heading and balance on one line; the
legend is four grouped lines; the clipboard and build grid cap themselves
against `_hotbar_reserved_height()`; the hotbar socket takes 0.88 of the
scale so twelve fit. `project.godot`'s window is 1600x900, where nothing
scrolls; at 1280x720 the build grid scrolls slightly, a deliberate last
resort over burying the footer.

### 5d. The screens around the game: the main menu and the pause menu

Both are built from §5's ingredients (dark plate, hazard-yellow rule,
outlined text), not a stock engine theme: the main menu is the first thing a
player sees.

**Main menu**: one centred plate (a `CenterContainer`, so it re-centres when
labels resize) with title, subtitle and New Game / Load Game / Quit over a
backdrop with one edged hazard band. Load Game **replaces** the buttons with
the save list. A row shows the slot name and the file's mtime (not the tick
count, which would mean parsing every file). The autosave is listed and
labelled. An empty list says so in a sentence.

**Pause menu**: the same plate, more opaque, centred in the whole viewport,
with the line *"the factory is stopped while this is open"* (§7: never encode
state invisibly). **Quit to Menu asks first; Save & Quit does not.** The save
field is a **name**, prefilled with what this session loaded from.

### 5e. Numbers into bars: the one meter language

A figure tells the player what a thing *is*; a bar tells them where it
*stands*. Every readout in the HUD that has somewhere to stand is drawn as
both — the number for the exact value, the bar for the shape of it at a
glance.

**A bar is drawn only where the simulation defines the ceiling.** Hopper and
tank level, a machine's cycle, a pipe's pressure, a contract's countdown
against its own cadence, a price against the money in the player's pocket:
those are real fractions. Larder stock and the balance itself are not —
nothing in the game says how many tomatoes is "full" — so they stay figures.
A gauge whose full mark is invented teaches a scale that does not exist.

All of them go through one pair of helpers in `main.gd` (`_build_meter` /
`_set_meter`), so there is one bar height, one track, one hairline border
and one empty state, and **three fills that each mean something**: green is
a level, amber is work under way, red is a cost that cannot be met or a
clock nearly out. Per §7 the colour is never the whole message — the
countdown says "due now", the missed tally reads "✗ 2 missed", an
unaffordable slot is dimmed and marked as well as red.

Two consequences worth keeping:

- **A docket's bar stays paper and ink.** §5 gives a contract card two
  colours; the countdown is drawn in ink on the checkbox's paper tone, and
  the card's single warning red is spent only on the last stretch of a
  countdown and on a non-zero missed tally.
- **Player-facing clocks are in seconds, never ticks**, and player-facing
  names are display names, never the wire's ids. "1m 15s to go" and "Boxed
  Ketchup", not "2250 ticks" and `boxed_ketchup`.

One readout is a bar's opposite number: **the money figure flashes what just
changed it** (`+90`, `-5`) beneath itself and fades, on the hotbar name
flyout's timings. A payout arrives on a contract's clock while the player is
looking at the factory, and a balance that only ever shows its current value
hides every event that produced it.

## 6. Feedback, motion & animation

- **Chunky, snappy animation** on voxel assets; no smooth organic motion.
- **UI transitions are quick and simple** — a clipboard flipping open.
- **Juice stays legible over spectacle**: short clear bursts, readable with
  dozens at once.

### 6a. Earning money — the one moment the game celebrates (built 2026-09-11)

Money arrives through exactly one channel: a contract settling at its cadence
boundary. That is rare (cadences are 1000-2500 ticks at 30 Hz), so it gets a
real celebration, while the frequent moment beneath it — a case landing on
the dock — gets a small cue. Two sizes of feedback for two frequencies; the
rare one would be exhausting at the frequent one's rate, and the frequent
one's cue would be an anticlimax for a payout.

- **A payout fires four things at once** (§7: never one channel alone): a
  fountain of coins over every dock, the money counter punching up in scale
  and flashing amber, a "+N" rising out of the money line, and a **PAID**
  stamp on the docket that paid. Say it in a shape, a position, a colour and
  a word, and it still reads when the player was looking elsewhere.
- **A dock arrival gets one small cue**: the arriving case pops in with a
  short overshoot, and a quiet knock plays. Nothing that competes.
- **The animation clock is the simulation tick, never wall time.** A pause
  freezes the celebration for free (§5d: the factory is stopped while this is
  open), and a screenshot run of a fixed tick count is still reproducible.
- **Every distance in a 3D burst is measured in `TILE`.** The first render of
  this had the coin arc in raw world units: a peak a fifth of a tile high and
  a spread narrower than a coin, so ten coins drew as one lump on the deck.
  Anything the player compares against the factory is measured in the
  factory's own unit (§2b point 12 again — headless tests all passed).
- **Randomness is derived from an index, not from `randf()`.** A fan of
  coins needs varied heights to read as a fountain rather than a ring, and
  a deterministic variation gives that without making a review screenshot
  unrepeatable.

## 7. Color & readability

- **Base palette is neutral/industrial**: steel greys, concrete, safety
  yellow/black, muted materials.
- **Saturated colour is reserved for meaning**: status lights, alerts,
  highlights, selection.
- **Never encode critical state in colour alone**: pair every status colour
  with a glyph, word, shape or position.

## 8. What this doc does not cover

Gameplay and balance (GAMEPLAY.md); architecture, rendering pipeline, data
formats (ARCHITECTURE.md).

**Audio direction — decided 2026-09-11**, by building the first sounds (§6a).
Sound in this game is **feedback, not a soundtrack**: a cue fires because
something happened that the player needs to know about, and there is no
ambient bed, no music and no idle machine loop. Every sound is
**synthesised in GDScript at runtime** into an `AudioStreamWAV`
(`godot_project/audio.gd`) rather than committed as a `.wav` binary — a
waveform written as code is diffable, reviewable and tweakable the same way
`assets/voxels/generators/` makes the models code (§2a point 1), and it is
deterministic, so a test can assert two renders of a cue are byte-identical.
A SOUND ON/OFF switch lives on the pause menu. What is still undecided is
whether a later, larger soundscape (machine loops, room tone) stays
synthesised or moves to recorded samples; that question lands when there is
something to loop.

## 9. Open questions

- Exact camera angle (true 2:1 isometric vs Sims-style dimetric): in
  practice true isometric via `CAMERA_DIRECTION`; a side-by-side is cheap.
- Concrete palette hex ranges per material category.
- Specific typeface.
- Whether characters (if any) share the 32³ tile scale.
- ~~`.vox` writer~~ — resolved: hand-written encoder (§2b point 1).
- ~~`.vox` importer~~ — resolved: see ARCHITECTURE.md §1a.
- **Touch/mobile input is a real gap.** Hover, small slot grids and
  keyboard camera controls do not transfer to touch. Resolve well before
  ARCHITECTURE.md milestone 10.

## 10. References (asset pipeline research, §2a)

- [MagicaVoxel .vox format spec](https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt)
- [Kaitai Struct: MagicaVoxel](https://formats.kaitai.io/magicavoxel_vox/)
- [`vox-format` crate](https://crates.io/crates/vox-format) (parser-only in practice)
- [`vengi` voxel tools](https://github.com/vengi-voxel/vengi)
- [Godot-MagicaVoxel-Importer-with-Extensions](https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions) (the other candidate, `godot-vox-import`, redirects here)
