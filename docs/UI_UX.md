# CONDIMENTAL — UI/UX & Visual Style Plan

This doc covers **aesthetics and UI/UX only** — how the game looks and
behaves to the player. Mechanics are [GAMEPLAY.md](GAMEPLAY.md); how it is
built is [ARCHITECTURE.md](ARCHITECTURE.md). Full decision records, including
the long form of §2b, are in [DECISIONS.md](DECISIONS.md).

---

## 0. At a glance

| Aspect | Direction |
|---|---|
| Camera | Fixed orthographic isometric, tile-based grid; a first-person view on TAB, which is what a session opens in |
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
- **A first-person view on TAB, and it is the view a session opens in**
  (`main.gd`'s `DEFAULT_VIEW_MODE`, 2026-09-21). The factory is voxel art at
  a human scale standing in a hall the sky shader draws (§3), and the
  opening frame should be the player inside it rather than above it; laying
  a line out is the second thing they do, and TAB is there for it. This does
  **not** loosen the rule above: the isometric camera is still fixed, still
  the angle every asset is art-directed for, and the first-person camera
  still does not orbit it — it is a separate free-look perspective camera
  with its own position, not a rotation of the isometric one.
- **A panel takes the keyboard with the cursor.** While anything is up that
  hands the pointer back — one of the five modals, the configuration panel,
  the pause menu — held keys stop steering the camera and mouse motion stops
  swinging it (`main.gd`'s `_fp_mouse_free`/`_camera_keys_live`). The panels
  bind the same keys the camera does: SHIFT+click on a market crate buys
  every unit the balance covers, and while the camera still read held keys
  the one press also flew the player down, since SPACE/SHIFT are first
  person's up/down (2026-09-22).
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
   This is what keeps §4's material rule and §7's palette consistent. It
   carries material facts as well as colour: the class table, and the
   `lamp_states` table that says what a status lamp looks like lit and dark
   (§2b point 39). `voxgen` resolves both and exports them beside the models,
   so a second renderer reads them rather than restating them.

Layout (built exactly this way):

```
assets/voxels/
  Cargo.toml        # crate `voxgen` — `cargo run -p voxgen --bin voxgen`
  palette.ron       # shared palette: colour AND material class
  generators/       # one generator per asset, plus vox.rs, icons.rs
  models/           # generated .vox — committed build output
  icons/            # generated UI icons — committed
  previews/         # review PNGs — gitignored
```

### 2b. How the pipeline actually turned out — rules learned by building it

Point numbers are cited by code comments; keep them stable — a number is
never reused and never moves. Full narrative in DECISIONS.md Part B. There is
no point 10, and the list runs 1-28, 35-37, 29-34, 38-39: the three entries the
factory-floor work added were written as a second 27, a third 27 and a second
28, and were given the next free numbers on 2026-09-16 rather than renumbered
into sequence. They sit where they were discovered.

1. **The `.vox` encoder is hand-written** (`generators/vox.rs`, ~150 lines) —
   no Rust crate writes the chunks the Godot importer needs. It puts a
   model's origin at its footprint centre at ground level.
2. **One asset = one mesh per material class.** An imported voxel mesh has
   one material, so `voxgen` writes `name.<class>.vox` per class; Godot
   rebuilds one `StandardMaterial3D` per class from `materials.json`.
3. **Headless review** via `vengi-thumbnailer` (needs a real video driver;
   `SDL_VIDEODRIVER=dummy` fails), wrapped by `scripts/build-voxels.sh`.
4. **Godot needs `--import` twice on a cold cache** — the `.vox` importer
   plugin registers only on the pass after the first scan.
5. **A liquid is its own material class**, near-white so `main.gd` can tint
   it live; the fluid model is one voxel tall, scaled to fill fraction (the
   same mechanism draws hopper/tank level and progress gauges). The tint
   value itself IS palette data (`FLUID_TINTS[id]`), even though the
   model's hue is not, so a fluid with a still model (vinegar's jug, point
   35) doesn't show two different colours side by side.
6. **Pipes are stainless steel, hollow, with an open inspection channel**
   above the bore floor so level is visible below half fill.
7. **UI icons are a second render pass** (`icons.rs`), flood-filled
   transparent and downscaled to 64px — not shrunk previews.
8. **Material rule for machines: food contact is stainless, structure is
   painted or poured.** Emissive is for status lamps only — and it is the
   only class Godot builds as a `ShaderMaterial`, so that a lamp emits its
   own vertex colour (§4).
9. **Review the rendered asset, not the generator**, and budget a rework
   pass per asset — looking changes the design.
11. **Only a machine's +X and +Z-world faces are ever seen.** An outlet is
    a doorway on +X; an inlet is a hooded housing cut open on the
    camera-facing side (the true inlet face is invisible). Both read by
    value contrast, not colour. A tall machine (the 2x2 vat) needs its
    opening widened to an unoccluded corner.
12. **The full-scene render catches bugs no headless test can** — every
    content-id lookup needs a fallback, since a stale key ships silently
    with every test green. `scripts/screenshot-scene.sh` is the check.
13. **A process an order of magnitude longer than a craft cycle needs a
    passive, emissive gauge** showing its own full-scale mark when empty.
    `MACHINE_PROGRESS_GAUGES` is sparse — a bar on a short cycle flickers.
14. **Wood (`oak`) is the one exception to point 8** — in a barrel-aging
    room the wood IS the process.
15. **A machine with no mechanism is a legitimate silhouette** when it only
    waits (e.g. casks stand on end, not lying down).
16. **An open-floored building puts height at the far side, content at the
    near side** (sight line travels `(+x,-y,+z)`). The delivery dock is
    architecture on the far tile with an open near apron.
17. **What stands on the dock is drawn from `delivery_stock`**, not the
    dock's own buffer (which empties every tick) — one case per unit,
    capped at four.
18. **A rotated placement reuses the same mesh spun about its footprint
    origin**, except point 11's junction art — carved doorways don't
    survive a 180° spin, so belt/pipe junctions are carried by the plain
    tile plus overlaid fittings instead (point 23, 25).
19. **A live body inside a hollow shape must stay strictly narrower than
    the shape's outer wall at every height it reaches**, or the two
    z-fight — only visible with liquid actually in the run.
20. **Anything Godot spins must be symmetric about the tile midline**
    (voxel 15.5, not 16), or a run at mixed facings steps at each joint.
21. **A dark accent on a camera-facing shadow-side face reads as a hole**
    (converse of point 11) — keep accents on bright plates.
22. **A composable fitting overlay keeps clear of a shape's sight windows**
    and carves its bore wider than the shape's so the meshes never share a
    surface.
23. **A thin diagonal mark does not survive this camera; a solid shape
    does** — a chevron rendered as an `L`. The belt's direction mark is a
    plain raised cleat (not an arrow, since 2026-09-10), with direction
    given in words. Belts animate by frames of one tile's period; corners
    use a quarter-annulus pattern stretched to match.
24. **A fitting overlay must be a full voxel of radius larger than
    anything it covers** (`PIPE_FITTING_MARGIN`), or the two solids
    z-fight at shared-neighbour voxels.
25. **A pipe tile's turn is a property of what it's joined to, never of its
    facing** — one joint-flange side per axis, so rotating a pipe visibly
    changes only a tile joined to nothing.
26. **A uniform offset in a path convention is invisible on a straight run
    and shows up the moment it bends** — items are now drawn face to face
    (input midpoint to output midpoint), not centre to centre.
27. **A machine's moving parts are a separate motion overlay, cut out of
    the body rather than duplicated** (`<machine>_motion_f0..f3`, siblings,
    one visible) — two overlapping voxel solids z-fight. Every motion
    volume is an even voxel height (`vox::encode` lifts by half-height).
    Frames must differ consecutively; a reciprocating part repeats a pose.
28. **Motion means "working", narrower than "has a batch"** — a mechanism
    turns only while `0 <= elapsed < required`, so a finished-but-blocked
    machine stands still with its lamp still lit. One pace (30 ticks) for
    every kind. No steam, deliberately.
35. **A model whose only consumer is the icon pass must be authored for
    vengi's camera (the -X/+Y corner), not the game's** — the two cameras
    see opposite sides of the same model. A fluid is the pure case: it's
    never placed or belt-borne, so its whole detail budget goes to the
    icon-visible faces.
36. **The floor is a background surface, authored by its value range, not
    its detail** — judged as thousands of copies, so any contrast becomes
    the loudest thing in frame. Variation is broad, low-contrast blotches,
    never per-voxel speckle, and the floor must never hold the brightest
    pixel (§4 reserves that for metal).
37. **N models over thousands of tiles is still only N stamps; the
    multiplier is per-instance rotation, free only if the tile's edges are
    four-fold symmetric.** Variant and turn are a pure function of tile
    coordinate, so a reload never repaves. The one exception is a model
    whose feature spans tiles (trench drains), which must never turn.
29. **A free-standing overlay at a fixed tile position meets every host
    body at every facing** — check it against the host turned all four
    ways by a test, not by eye, since a preview shows only one facing.
30. **A per-tick fact shown beside a windowed verdict must be held across
    the ticks it's absent** — a hover explaining a status from a
    same-tick-only reason flickers; keep the last non-null reason while
    the status holds.
31. **At icon scale an object is identified by the proportion of its parts,
    not their presence** — correct detail on a wrong ratio reads as the
    wrong thing. Judge an icon at its real size on the real plate, not the
    full-size preview.
32. **A fixed-size card clips a label without complaint, and a taller card
    is not always available to fix it** — a new line on a card in a capped
    grid is paid for from the card's own padding, not the grid's height.
    The grid's scroll cap comes from the PANEL's available height, so a
    taller card raises nothing and clips more of the row below it; and a
    wrapped label is not a fix either, since autowrap breaks the phrase
    where the width falls rather than where the sense does (the build
    card's crew line takes a literal newline and a smaller icon).
33. **A panel placed near the cursor is positioned against the fixed HUD
    plates, not just the viewport's edges, and every candidate is judged
    AFTER the on-screen clamp rather than when it is chosen.** The hotbar,
    legend and clipboard are drawn over the viewport, so a flip that only
    respects the viewport's right and bottom edges lands the panel on top
    of them. Try all four quadrants around the cursor, nearest first; when
    a tall panel fails all four, clamp on screen and then push clear —
    keeping only a push that still clears the plate once the clamp has been
    re-applied, or the clamp snaps the panel back onto what it was
    avoiding.

    **That rule is for a cursor, and first person has none.** There the
    query point is the crosshair at the centre of the screen and the subject
    is the thing *behind* it, so a panel that follows the anchor stands on
    what it describes. A parked panel — the lower-left corner, above the
    hotbar — is the first-person rule instead: it is the one region no fixed
    plate claims and the crosshair never enters, and being fixed it also
    stops the panel sliding about the frame on every mouse-look. Covering
    the legend is preferred to covering the subject, so a position that
    stands on the crosshair is refused however little HUD it would cover
    (issue #105).

34. **A ghost previews what the game's connection rules already make
    certain, and never what the simulation still has to decide.** The two
    shapes drawn under the cursor look inconsistent and are not: a belt
    ghost stays straight at its facing even beside a belt that will corner
    it, while a pipe ghost takes its shape from the pipes already placed
    around it. GAMEPLAY.md §9 is the reason — solids respect faces, fluids
    connect by touching. A pipe joins everything it touches, so the shape
    the tile will have is knowable before the click; a belt's hand-off is
    the sim's own directional answer (`resolve_sink`), so a ghost that
    drew the corner would be promising a link the placement might not
    make. For the same reason the pipe ghost still previews no machine or
    tank fitting: which tile of a footprint is tapped is the simulation's
    call, not geometry's. Where a rule is certain, show it; where it is
    not, show the plain tile and let the prompt (§5a) say the rest in
    words.

35. **A list that scrolls is capped by what the window has left, measured,
    not by a constant.** The main menu's save list had a flat 450px cap,
    which fit while the title sat inside the plate. The wordmark then moved
    out of the plate and grew to 114pt for the title screen (§5d), and at
    1600x900 that same list pushed the BACK button clean off the bottom
    edge. `menu.gd`'s `_list_height_budget` now subtracts the wordmark, the
    plate's padding and the load view's own furniture — measured off the
    nodes with `get_combined_minimum_size`, so it follows the fonts — from
    the viewport, and the list takes what is left. A constant that was
    right once is only right until the layout around it moves; and only a
    rendered frame shows that it stopped being right (point 12).

38. **A destructive action names what it destroys, before the click.**
    Declaring what a fluid tank holds empties the tank *and* the pipe run it
    feeds — the sim destroys that volume rather than crediting or moving it
    (issue #43). The config picker's note therefore reads "switching destroys
    89.7 units — this tank and the run it feeds" whenever there is something
    to lose, and drops the number when there is not. The alternative,
    refusing the switch while the run is full, is worse: it strands a
    mis-declared tank with no way to drain its run. Destruction the player
    consented to is fine; destruction they discover afterwards is not.

39. **A status lamp has two states, and both are palette data.** A lamp is
    dark far more often than it is lit — `main.gd` dims one the moment a
    machine body is built and only lights it while the machine processes — so
    "the lamp" is two materials, not one. Both live in `palette.ron`'s
    `lamp_states` table, written as *modifiers* of the `emissive` class:
    `emission_scale` multiplies the class's energy rather than restating it,
    so retuning the class moves the lit lamp with it and there is no second
    copy to drift. `voxgen` resolves them and writes
    `models/lamp_states.json`; `voxel_assets.gd` and the asset browser both
    build their materials from that file. The dark state drops emission to
    zero **and** darkens albedo, because albedo is what keeps a dead lens
    reading as dead in a frame where the glow pass is doing the talking — the
    same judgement `palette.ron` makes outright in its `status_*_off` hues.
    Before this, the dim values existed only in GDScript, and the asset
    browser — which cannot drift from the pipeline precisely because it
    derives everything from repo files — had nothing to derive and drew every
    lamp lit (issue #81).

## 3. Theme & setting

**A grounded, semi-realistic 21st-century industrial food factory**:
stainless tanks, rubber belts, steel piping, poured concrete, hazard
striping, pallets, loading docks. Lean into real industrial visual language
(signage, cable runs, grates, walkways). Avoid candy-coloured or cutesy
factory tropes; the sauce theme comes through in what the factory processes.

**The player's line stands inside that factory, and the backdrop says so.**
The sky is `godot_project/factory_interior_sky.gdshader`: a hall with a
trussed roof, skylight ribbons, high-bay lamps, clad walls with clerestory
glazing and a line of other plant standing along them, all fading into air
haze. It is a *sky*, so it sits at infinity with no parallax — the hall is
the same size however far the player walks, which is what makes it read as
enormous rather than as a room. Never an outdoor sky (the factory then reads
as a slab of concrete in a field) and never real geometry (a finite shell has
a wall to reach and occludes the isometric camera).

**The hall is drawn in first person; the isometric view gets its air.** A sky
is found by intersecting a box against `EYEDIR`, which needs rays that fan
out, and the isometric camera's are parallel — the hall came out as hard
stripes fanning from the corners of the frame. So `factory_environment.gd`'s
`set_backdrop_for` fills the isometric background with `HAZE_COLOR`, the very
colour every distant surface in the shader fades to, and draws the hall
itself in first person. **Only the background changes**: the sky stays the
ambient and reflection source in both modes, so the factory is lit
identically either way and TAB changes what is *behind* the world, not the
world. Full record in `docs/DECISIONS.md` (2026-09-16).

## 4. Materials & lighting

- **Baseline materials are simple and mostly matte**: painted metal,
  concrete, plastic, rubber.
- **Metals are the deliberate exception**: stainless tanks, housings, pipes
  and fittings read noticeably shinier, so machinery pops.
- **Lighting**: real-time dynamic lighting, soft shadows, mild bloom on
  emissives, light SSAO. Reference: a modded-Minecraft shader pack, not
  photorealism. **"On emissives" is the whole of it**: the glow post-pass
  must mean *this is emitting*, never *this pixel is bright*. That is
  `glow_bloom = 0.0` (Godot's `glow_bloom` blooms everything *below* the
  threshold — frame-wide haze by definition), a threshold above the
  brightest lit white surface in the game, and **SCREEN**, never SOFTLIGHT,
  whose effect scales with the pixel beneath it and so cannot brighten a
  lens that is already bright (DECISIONS.md, 2026-09-20).
  **"Light SSAO" is a crease, not a shadow**: `ssao_radius` is in world
  units, so it must be set at the scale of *a fitting against a body*
  (0.45 units = 4.5 voxels), not of a machine against the floor. A wider
  radius reads as grime at eye level long before it reads as contact
  shading in isometric. Because the radius is world-space, the two cameras
  disagree about how strong the pass is — judge a change to it in
  isometric closeup, play zoom **and** first person standing beside the
  line (DECISIONS.md, 2026-09-20).
- **The light is indoor light**: a lamp-white key from above for the roof's
  high-bay fittings, a daylight-cool fill for the glazing, and ambient taken
  from the interior sky itself. Warming the key beiges every grey asset in
  the game, because there is no blue sky left to balance it.
- **Emissives**: status lights, screens and strobes glow — both aesthetic
  and a readability signal (§7). **A lit voxel glows in its OWN colour**, and
  that is the one place in the pipeline that needs a shader rather than a
  `StandardMaterial3D` (`godot_project/voxel_emissive.gdshader`): a single
  `emissive` submesh often holds two lens colours, so a flat emission would
  wash them together. `palette.ron`'s `emission_energy` is judged against a
  rendered lens, not picked — past about 1.2 an amber lens burns through to
  cream and the hue is gone (DECISIONS.md, 2026-09-16).
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
  category** (the staff pools MVP, ticket 3, 2026-09-12; extended by the
  quality pass's staffing-legibility ticket, 2026-09-14). Icon, name,
  `hired 4  ·  wage 2 / head / period`, then the §5e bar that this panel
  earns: `assigned / demand` — what the factory asked for on the last tick
  is the simulation's own ceiling, so the bar is honest, and it reads
  `0 / 0` on an idle line rather than headcount against an invented full
  mark. The two numbers are printed ON the bar, `2 short` stands beside it
  in the one warning red while the bar is red (§7: position, digits, word,
  colour), and HIRE / FIRE are the build menu's own buttons — FIRE disabled
  at zero, HIRE disabled the moment the wire's own `can_hire` says a head
  cannot be afforded, not only after the click is refused. A category
  says so in a sentence in place of an empty bar
  (`nothing to maintain yet`). **A row is two lines, not seven**
  (2026-09-18, issue #7): name and verdict chip over `hired 4  ·  wage 2 /
  head / period` in one column; the bar with its `hired / asked for`
  caption beside it — a caption on a line of its own cost four lines
  across the four rows to say four words — and after the bar the shortfall
  (`▲ hire 2`) while there is one, or the per-tick `2 of 4 working right
  now` when there is not, never both: an instruction and a flow figure
  side by side read as one number. Under the bar, **one sentence**, the
  worst news the row has, ranked *unpaid heads > cannot hire and why >
  what hiring would unblock > WHY > what the category does*; the rest are
  the row's hover. Seven stacked elements made a row 147px tall, the
  panel's scroll caps at 400px at 1600x900, and the fourth category sat
  below the cut — in the one panel whose whole job is comparison BETWEEN
  categories. Condensing the row is the fix; a taller panel or smaller
  type would have been §5c's last resort taken first. **WHY:**
  `asked for by: crusher ×1, mixer ×1, cooking vat ×2` counts placements
  by kind in the order the factory first asked, or `nothing placed asks
  for staff yet` on an empty line — before this a shortfall's cause lived
  only in a machine's hover, which a category with no shortfall never
  earned. **What it does:** one sentence per category
  (`STAFF_CATEGORY_BLURBS`), so a category nobody has needed yet is still
  explained. The footer carries the upcoming bill, the payday countdown in
  seconds (§5e: never ticks), and one of two futures: `balance after
  payday: N` when it is covered, or the ticket's own words when it is not
  — `▲ next payday short by 6 — 3 heads go unpaid` — glyph and words
  together, never colour alone (§7). Only when it happened, the footer
  also says `✗ 3 left unpaid last payday` / `✗ 2 idled unpaid last
  payday`, and a payday that idles or loses a head fires the clipboard's
  own fading notice (below) the moment it happens, naming category and
  count, so the news is not only found by opening H afterward.
  **Being short is said before H is opened, and again at the top of it**
  (2026-09-16, on the owner's verdict that it was not clear when staff were
  needed): a **SHORT-STAFFED** strip on the always-visible clipboard, under
  the wage line it belongs with, in the docket's one warning red with a
  glyph, the words and the key — `▲ SHORT-STAFFED — hire 2 manufacturing ·
  press H` — naming only the categories actually short and disappearing
  entirely when none are, never reading `0 short`. **The column shows one
  hint line at a time** (2026-09-16, issue #13): while the strip is up the
  board's own `C: contracts board — sign more orders` hint is hidden, because
  the two say the same kind of thing in the same place and the right-hand
  column cannot afford both — at 1600x900 those two lines were the
  difference between a whole docket plus the full stock plate and a docket
  cut through a sentence. The strip is the line that yields last: it carries
  a number and a key, the hint carries only a key. And the panel opening
  with a one-sentence verdict above its first row (`▲ 2 short — hire 1
  manufacturing, 1 packaging`, `✔ every machine has its crew`, or `nothing
  placed asks for staff yet`), three states in three inks, the green spent
  only on a verdict the factory actually earned. A short row is then
  obvious without reading numbers: a `▲ SHORT` chip beside its name, a
  warn-coloured border, and a line saying what hiring would unblock (`hire 2
  → crusher, mixer, cooking vat all get crew again`), drawn from the same
  count of placements the `asked for by:` line uses. **A button says what
  the click does and what it costs** — `HIRE +1 · 2 / period`, `FIRE −1` —
  and a disabled HIRE says *why* in words rather than only in grey (`✗
  cannot hire — 8 in hand, 14 due, 2 / period`). Last, the payday rules are
  stated in plain words at the foot of the scrolled list (`STAFF_RULES`):
  every head employed for any part of a period is billed for it; an unpaid
  head idles until the next payday and leaves only if unpaid twice. The
  legend sits **inside** the scroll rather than under the footer for §2b
  point 32's reason — `_fit_grid_scroll` takes its cap from the panel's
  height, so four lines of chrome cost a whole category row — with one
  always-visible footer line saying it is down there. It shares **one flat
  box** with the category rows (2026-09-20, issue #63): the scroll's cut
  snaps between the *children* of the box it is given, so with the rows
  nested a level down the only cut that was not "the whole list" was one
  through the explainer's own lines. Flat, every boundary a reader can see is
  a boundary the cut can land on, and the explainer is shown whole or not at
  all. A row
  carries the same fact permanently: `▲ 2 unpaid, idle until payday` / `▲
  1 left last payday`, joined when both apply. E, M, C and H are one
  modal at a time; ESC closes whichever is up. Being short is also read
  where it bites: a refused machine's hover says `▲ starved · … · short
  of manufacturing staff` over its `crew: 2 manufacturing` line, a
  refused dock says `held · short of logistics staff`, and a build-menu
  card says a kind's crew and its running cost under its price
  (`crew: 2 manufacturing`, with `4 / period` on its own second line —
  point 33 below). **The wage bill and payday
  countdown are also on the always-visible clipboard** (quality pass,
  2026-09-14), not only inside H: `wages 14 / period  ·  payday in 6s`
  sits directly under the money it draws from, taking the docket's one
  warning red with a glyph and words — `⚠ wages 16 due in 4s — short 6,
  unpaid heads leave` — when the balance cannot cover it, and hiding
  itself entirely with nobody hired rather than reading zero. Both the
  clipboard line and the H footer read the wire's own `next_bill`/
  `shortfall`, never a Godot-side sum: since the arrears rule a head
  fired mid-period is still billed for it, so an independently-computed
  figure could disagree with the sim at the one moment — just after a
  firing — a player is watching it closest. **A lost head is announced
  through the same mechanism a lost contract uses**, reusing
  `_show_contract_notice` rather than a second toast convention, and
  priming silently on the first poll after a rebuild so a loaded save
  never replays a stale payday as news.
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
actions: **left-click buys one, right-click places one you own**; a market
crate has two as well — **left-click buys one crate, SHIFT+left-click buys
every unit the balance covers** — and each slot prints what the shift
gesture would fetch *right now* (`shift: buy 123`) under the crate's price,
dimmed to `shift: buy 0` when nothing is affordable, because a modifier
nobody can see is a modifier nobody uses.

**What a hopper or a tank will hold is not chosen on the ghost.** The ghost
carries the *first* raw the simulation offers — `hopper_items[0]`, i.e.
`World::raw_items` in wire order — drawn as a tinted fill and named in the
prompt, so the click is still specific about what it is about to place. It
is changed afterwards, by **clicking the placed building** and using the
picker in its own configuration panel; the prompt says so on the
ghost, `click it once placed to change contents`, and says it only when
there is more than one raw to choose between, because a hint about a choice
that does not exist is noise. The fluid tank works the same way and through the
same picker (issue #43, 2026-09-20), so there is one gesture for "what does
this vessel hold" rather than one per vessel.

This replaced a **Q** key that cycled the ghost's contents before the click
(along with global **SPACE** and **V** for filling, and the sim's
`CycleHopperItem` command, all deleted). The reason is worth keeping: a
keybind on the ghost can only configure a thing that has not been built yet,
so the answer to "I placed the wrong one" was to remove it and place it
again. Configuring the *placed* building makes the bin retargetable in place
and puts every vessel-feeding verb — level, FILL, AUTO-FILL, contents — on
one panel reached one way. That panel has its own section, **§5g** —
where it stands, what it yields and in what order. `test_config_panel.gd`
pins it and `scripts/screenshot-scene.sh --config TX,TY` photographs it.

**SHIFT buys back what the click places** (2026-09-16). Laying a run of belt
otherwise costs two trips to the build menu per handful — buy ten, place
ten, buy ten more — so holding SHIFT while placing spends money to replace
the unit just placed, keeping the owned count level until the balance runs
out. It is a *modifier on placing*, not a second way to buy: the click still
places what the player owns, and the purchase happens after it and only if
it succeeded, so a blocked tile never charges. The prompt says which it is
before the click and after it, always with the price on it: **SHIFT: buy a
replacement as you place  (5 each)** unheld, **⟳ SHIFT: buying a replacement
each click  (-5)** while held, and **⚠ SHIFT: cannot afford another  (5)  ·
placing from stock** when the balance has run out — because a modifier that
silently drains the balance is exactly §7's "never encode state invisibly",
and the line has no colour of its own to say it with. The unaffordable case
warns *before* the click rather than buzzing after it: nothing was refused,
the placement still stands, and the owned count simply starts falling.
Placing gets its own cue and the replacement its purchase cue (§8), so two
sounds mean two things happened.

### 5b. The hotbar: what is in your hand

An always-visible strip along the **bottom** edge, centred (the other edges
hold the legend and the clipboard). One slot per `data/shop.ron` entry, in
file order, built from `shop_snapshot_json`. What raw stock has been bought
is **not** here — it is the right-hand STOCK plate (§5f), which replaced the
one-line larder strip that used to sit above the sockets.

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

**Width yields too, and before it is clipped** (2026-09-20, issue #62).
Height had yielded since this section was written; width had not, so at
1280x720 the staff panel (1350px), the build menu (1412px) and the contracts
board (1380px) were simply drawn off *both* edges of the window — taking the
FIRE and SIGN buttons with them. **A control the player cannot reach is the
worst thing a layout can do**, above every fault ranked in §5f. So a modal
measures itself against `_modal_available_width()` (the viewport less
`MODAL_EDGE_MARGIN`, in window pixels — it is the last gap before a control
is off the screen, not a design margin that scales) and gives width back in
this order:

1. **A sentence moves to the row's hover.** The staff row already ranks five
   sentences and shows one; at this width it shows none, and the hover
   carries all five along with the per-tick figure and the meter caption
   (`_fit_staff_width`, `_apply_staff_prose`). What stays is what can be
   clicked.
2. **A grid of cards takes fewer columns.** `_fit_grid_columns` asks the
   panel whether it fits, one column at a time, then balances the rows (six
   crates are 3+3, never 5+1). `SHOP_SLOT_COLUMNS` and `MARKET_SLOT_COLUMNS`
   are **the most** a grid ever takes, not the number it always takes.
3. **The list scrolls**, as above.

**A cursor panel with nowhere to stand yields lines, not the legend**
(2026-09-25, issue #99). At 1280x720 with the legend open, a full machine
hover (456x283) is 48px taller than the band between the legend and the
hotbar, and no gap beside the legend is wide enough, so every position
covered some plate. Folding the legend for the player would break §5f's "a
fold is the player's", so the hover sheds instead, fewest rungs first
(`_hover_yield_rungs`): **the `crew:` line**, which the CREW plate on the
same frame already names, then **the load bar**, whose `% busy` figure stays
in the text above it. The ladder is judged by the same placement rule that
then places the panel, from the full panel every frame. That means a bigger
window or a folded legend brings the lines back on its own.

**`autowrap_mode` is never put on a Label whose width a fit pass measures.**
A wrapping Label reports a 1px minimum width and a height shaped at whatever
width it was last *drawn* at — so under `--headless` it measures 1x1609 and
every panel sized around it is nonsense. This layout is measurement-driven end
to end and its regression proof is a headless test. A sentence that must get
narrower is therefore **broken at a join it already has** (the board's offer
is a mix label and a terms label) — a break at a fixed point, not a reflow,
and `_fit_contract_board_split` measures before it breaks so a window with the
room keeps the sentence whole.

It *is* used — 14 times in `main.gd` — wherever the width is **pinned** before
anything measures it, so the 1px minimum is never what a fit pass reads: inside
a fixed-size ancestor (`SHOP_SLOT_SIZE`, `MARKET_SLOT_SIZE`,
`CONFIG_PICKER_SLOT_SIZE`), or on a Label that sets its own
`custom_minimum_size.x` (`CONFIG_PANEL_WIDTH` for the configuration notes,
`CREW_TEXT_WIDTH` for the crew affordance line, `PAUSE_BUTTON_MIN_SIZE.x` for
the pause menu). The configuration panel *is* measured by a fit pass
(`_position_config_panel`), so escaping measurement is not the exemption — the
pin is. Pin the width or do not wrap.

**A panel that dodges the right-hand column consults one list, after its own
clamp** (2026-09-20, issue #88). The clipboard, STOCK and CREW plates are
three tenants of one edge, so a panel avoiding them takes them as a list
(`_contract_panel_rect`, `_stock_rect`, `_crew_rect`) rather than a branch
each — a branch each is how CREW came to be missing from
`_position_config_panel` for four days. The avoidance is judged **after** the
panel's own clamp, per §2b point 33, and a panel flipped aside lands its edge
**on** the bound rather than at a fixed offset from its anchor. The legend is
deliberately not in that list: at 1280x720 nothing clears both it and the
column, and a keybind reference the player can fold with F1 is the right thing
to cover in preference to a live readout.

**A panel that points at the world is followed in isometric and parked in
first person** (2026-09-21, issue #105). The hover tooltip and the placement
prompt are the two, and both go through one door (`_world_panel_position`) so
neither can be fixed and the other left behind — the tooltip was the one
reported and the prompt had the same fault. In first person they take the
lower-left corner (`_first_person_panel_position`), stepped clear of whatever
plate that corner still touches by the same scoring the cursor rule falls back
to, with the crosshair as a veto rather than a score. At 1600x900 the old flip
found room and the fault did not show; at 1280x720 every quadrant failed, the
clamp came back through the middle, and the dock a player had walked up to sat
entirely behind its own tooltip. The corner is chosen over "below the
crosshair" because there is not room below it: at 1280x720 the strip between
the crosshair and the hotbar is about 230px and a machine hover is 283 tall.

**A headless test cannot prove a width; only the render can.** Godot's
headless font metrics are narrower than a real window's: the staff panel
passed the headless assertion at the very size the rendered frame clipped.
Headless assertions are the regression net, `scripts/screenshot-scene.sh` is
the acceptance. Relatedly, `get_combined_minimum_size()` can *under*report
what is on screen (the machine hover: 299px reported, 545px drawn), so
anything positioning a panel takes `maxf(minimum, size)` on both axes.

### 5d. The screens around the game: the main menu and the pause menu

Both are built from §5's ingredients (dark plate, hazard-yellow rule,
outlined text), not a stock engine theme: the main menu is the first thing a
player sees.

**Main menu**: a **title screen** — the factory itself, in 3D, standing in
the §3 hall behind the menu (`title_scene.gd`), the way Minecraft's panorama
and Factorio's title art do. It is a hand-composed still life of the shipped
chain, not `main.tscn` and not a simulation: nothing ticks, the items on the
belts were put there by hand, and the camera drifts slowly so the screen
reads as a place rather than as a paused game. Its models, materials, floor
variants and lighting all come from the game's own pipeline
(`voxel_assets.gd`, `factory_environment.gd`, `main.gd`'s floor helpers are
shared, not copied), so the poster cannot go stale. The camera is
**perspective** here — the fixed orthographic rule (§1) governs how the game
is *played*, and depth is the whole subject of a photograph of the hall. The
air is clear here where the game's is hazy (`TITLE_HAZE_DENSITY`) — the one
parameter that differs between the two, because haze read as a grey wash
lying on a single still frame rather than as distance.

Over it: the wordmark, subtitle and a tilted yellow **splash line** (picked
from a fixed list by the date, so it changes day to day and a review
screenshot still repeats), then a dark plate with New Game / Load Game /
Quit, **centred in the viewport**. **Nothing is washed over the hall**: a
gradient scrim used to stand behind the column for legibility and read as a
shadow lying on the picture, so it is gone (the owner's call). The text
carries its own contrast instead — the wordmark and the strap take a heavier
outline than anything inside a plate needs (`TITLE_OUTLINE`,
`STRAP_OUTLINE`), and the button plate is near-opaque in its own right. (An
earlier cut stood the column to one side so the plate would not cover the
vanishing point; the owner asked for it centred, and the backdrop's framing
was moved to suit — the chain now reads either side of the plate, tomatoes
going in on the left and bottles coming out on the right.) Load Game **replaces** the buttons with the save list. A row shows the slot name and the file's mtime (not the tick
count, which would mean parsing every file). The autosave is listed and
labelled. An empty list says so in a sentence.

**Pause menu**: the same plate, more opaque, centred in the whole viewport,
with the line *"the factory is stopped while this is open"* (§7: never encode
state invisibly). **Quit to Menu asks first; Save & Quit does not.** The save
field is a **name**, prefilled with what this session loaded from.

**Settings** is one screen hosted twice — `settings_panel.gd`, built by the
main menu and by the pause menu alike, so a knob cannot exist in one and not
the other. In both it **replaces the button column in the plate it is in**
rather than opening a second plate over the first: that is what Load Game
already does on the menu and what the quit confirmation does on the pause
menu, and a modal over a modal is two things to dismiss. The heading says
*"kept on this computer, not in your save"*, because these are preferences
and not progress: they live in `user://settings.cfg` and a save carried to
another machine does not bring them (`docs/DECISIONS.md`).

Two row shapes, both built from §5's plates rather than from stock engine
controls:

- a **toggle**, one button captioned in words — `MUSIC: ON` / `MUSIC: OFF`,
  never a coloured light (§7);
- a **stepper**, `[ − ] LABEL   value [ + ]` over a thin fill bar. Steppers
  and not a slider because a drag cannot be asserted headlessly and because
  every one of these knobs is a short list of sensible values; the bar is
  **not** the §5e meter language — the sim defines no ceiling here, so it
  gets its own colours and its own `SettingRange_*` name.

The value is always **formatted, never bare**: `100%`, `×1.25`, `75°`. The
column **scrolls on a row boundary** (§5c's snap rule) — half a card of a
contract is half a decision, and half a settings row is a `+` button drawn
where it cannot be clicked.

On the main menu the **wordmark stands down while these rows are up** and
comes back with the buttons. It is the title, the tagline and the splash —
about 230px of the column at any window size — and at 1280x720 that height is
the difference between one row of eight and four. §5c already ranks a control
the player has to reach above everything else on the screen, and what is above
this plate is decoration; the player is one BACK press from all of it, and the
factory poster behind it never moves. Load Game keeps the wordmark: its list
is short and its rows are tall, so it has nothing to spend the height on.

**The development menu (F3)** is not part of the game and says so on itself:
a blue-edged plate, never the hazard yellow every in-game plate is bordered
with. It is **not a modal** — ESC leaves it alone, it closes nothing, and the
factory keeps running behind it, which is the entire point of watching a
cheat land. It sits bottom-left, under the legend and above the hotbar, the
one corner no plate owns. Money grants go over the real command wire (a cheat
that Godot kept to itself would mean a balance the sim disagrees with);
**time speed is presentation-side** and the panel says so — the clocks in the
game still count the simulation's seconds, not the wall's.

### 5e. Numbers into bars: the one meter language

A figure tells the player what a thing *is*; a bar tells them where it
*stands*. Every readout in the HUD that has somewhere to stand is drawn as
both — the number for the exact value, the bar for the shape of it at a
glance.

**A bar is drawn only where the simulation defines the ceiling.** Hopper and
tank level, a machine's cycle, a pipe's pressure, a contract's countdown
against its own cadence, a price against the money in the player's pocket:
those are real fractions. The balance itself is not — nothing says how much
money is "full" — so it stays a figure.
A gauge whose full mark is invented teaches a scale that does not exist.

All of them go through one pair of helpers in `main.gd` (`_build_meter` /
`_set_meter`), so there is one bar height, one track, one hairline border
and one empty state, and **three fills that each mean something**: green is
a level, amber is work under way, red is a cost that cannot be met or a
clock nearly out. Per §7 the colour is never the whole message — the
countdown says "due now", the missed tally reads "✗ 2 missed", an
unaffordable slot is dimmed and marked as well as red.

Two consequences worth keeping:

- **What's on the dock earned its bar the day the simulation gave it a
  ceiling** (the dock-capacity change, 2026-09-16). The clipboard's ON THE
  DOCK line was a bare figure under exactly this rule — nothing said how
  many boxes was "full" — and now the shared pool has a per-item cap
  (GAMEPLAY.md §9) the line reads `20/48` with the meter beside it, in the
  level green until it is full and the warning red at the ceiling, with
  `⚠ FULL` spelled out beside it (§7). The rule is unchanged, not bent: the
  bar appeared because the ceiling did, and with no `delivery_capacity` on
  the wire the line falls back to the bare figure rather than inventing a
  full mark.
- **The warehouse's ceiling is drawn where it is spent, and that is not the
  STOCK plate** (2026-09-21). Racks cap raw stock (GAMEPLAY.md), so "how many
  tomatoes is full" finally has an answer and the §5f plate could have had a
  bar under this rule. It does not, for the thing the rule is really about: a
  cap is `base + stack × free bays` and the bays are a pool every ingredient
  draws on, so the tomato row's full mark moves when the player buys sugar. A
  bar that slides while the player never touched the thing it measures
  teaches a scale as badly as an invented one does. So the ceiling is split
  three ways, each to the place that can hold it honestly: the plate says
  `FULL` in a word on the row that has reached it (§7), the market crate
  prints `owned / capacity` because the crate is where the ceiling actually
  stops something, and the bar goes on the one genuinely fixed fraction in
  the system — **bays used of bays built**, in the market's warehouse header.
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
hides every event that produced it. That fade, the hotbar name flyout's and the
contract notice's are the three clocks deliberately outside the simulation
tick, so they keep running with the pause menu open — but they are timed
against `main.gd`'s `_hud_now_msec()`, summed from the frames the scene was
actually given, never `Time.get_ticks_msec()`. For a player that is the wall
clock; for a review render it is what makes the fade the same in two
photographs of one command (issue #82).

### 5f. The right-hand column: what you have, and the legend that folds

Three plates are drawn over the game and never asked for: the legend
top-left, the contracts clipboard top-right, and — since 2026-09-16 — the
**STOCK** plate under the clipboard, on the same right edge. Between them
they answer "what do I press", "what do I owe" and "what do I have", and the
first and the third are the two the owner asked to be able to see and to get
out of the way.

**STOCK is a plate, not a strip.** Bought raw stock used to be a one-line
larder strip inside the hotbar's own container, centred over the sockets:
one line tall by construction, so it could only ever afford a chip per
ingredient and would not survive the ingredient list growing. It is now a
right-hand plate, a sibling of the clipboard, with **one row per raw
ingredient the simulation prices** — voxel icon, display name, amount
right-aligned so the numbers form a column a glance can run down. **One
readout, one fact, one place:** nothing on the hotbar shows stock any more.
Per §5e there is still no bar, though since the warehouse (2026-09-21) the
reason has changed: stock now has a ceiling, but it is drawn from a shared
bay pool, so a row's own full mark would move when a different ingredient was
bought. Zero is carried by the word `none` on a dimmed row rather than by a
colour (§7); the ceiling by a `FULL` chip on the row that has reached it; and
the pair of numbers by the market crate that spends it. The heading keeps the
`M: market` hint so the plate says where the numbers come from.

**The right-hand column arbitrates in both directions, and only one half of
that is obvious.** The clipboard's scroll cap subtracts the plate's reserved
height; the plate takes the band left between the clipboard's *actual*
bottom edge and the hotbar's top. The trap, found by rendering the merged
HUD at 1280x720: if the plate reserves the height it has *settled at*, the
two caps chase each other down — a frame where stock had shrunk to one row
reserved one row's worth, the clipboard grew into the slack, and stock
stayed at one row for good. **A reserved height is what a plate wants, not
what it currently has.** The docket stack scrolls by design (§5c's last
resort); this readout does not, so the clipboard is the one that yields.

**Where the yielding stops** (issue #13): both panels also have *floors* —
one whole docket, one whole stock row — and at 1280x720 with a full board
the two floors do not both fit in the column. Ranked worst to least worst,
the faults are: **a control the player cannot reach** (2026-09-20, §5c);
a panel drawn ON another panel; a card cut through the
middle of a line; a list showing less than it could. So the clipboard may
grow past what the plate reserved, taking the column back a docket at a
time, up to the line the plate stands on once it has given up all but one
row (`_stock_ceiling_y`) — and at that line the stack is cut short of a
whole docket rather than either panel overlapping the other.

**But "cut short" still lands on an edge** (2026-09-18, issue #17): where
the clipboard cannot keep a whole docket it keeps whole *lines* of one,
cutting under the last header, checklist line, status, strike row or
countdown that fits (`_line_boundary_height`). The ordering above is
unchanged — the clamp still wins, and the cut still goes down, never up.
It is the third rank that got sharper: "a list showing less than it could"
now means a docket showing fewer of its own lines, not a docket sliced
through one.

**Three tenants in one column, and the tie goes to what cannot be reached**
(2026-09-21, the warehouse work). The CREW plate made this column three
plates deep and the arithmetic stopped closing: at 1600x900 the column is
741px and the clipboard, STOCK and CREW wanted 796px between them. What gave
way was the stock list — down to ONE of three ingredients, behind a scroll
thumb drawn over the count — so the plate whose whole job is "what do I have"
was showing a third of it. Both candidates are rank-4 faults (a docket
showing fewer lines; a list showing fewer rows), so the tiebreak had to come
from the rank above: **reachability**. The docket stack is a scrollbar the
player can drag, and C shows every contract in full; the STOCK plate is
`MOUSE_FILTER_IGNORE` from the panel down, so a row cut off it cannot be
reached at all. The column pays the plate first now —
`STOCK_PANEL_HEIGHT_SHARE` 0.42 → 0.55, `CREW_PANEL_HEIGHT_SHARE` 0.43 → 0.32
(CREW spends its assignment line, which the H panel still carries in full),
and the clipboard is capped at the line where the whole list fits
(`_stock_list_ceiling_y`) but never below its own one-line floor. The shipped
1600x900 window shows whole dockets and the whole plate again.

**At 1280x720 the column still cannot seat all three whole, so the cut is
announced.** A plate silently showing one of three ingredients reads as a
factory that holds one ingredient, which is a worse lie than a short list.
The heading's hint becomes `+2 more · M: market`, and the row list reserves a
gutter so the scrollbar is never drawn over the count it is hiding. On an
IGNORE plate a scrollbar is a mark rather than a control — it can say "there
is more" and cannot be used to see it — so the words have to carry it.

**The case that would not snap is bought with the hint's line** (2026-09-20,
closing issue #17). At 1280x720 with every offer signed the column left the
clipboard 12px where the first line boundary inside a docket is 43px:
snapping there would have shown no docket at all, and a clipboard with
neither a docket nor the cue that there are dockets is worse than a ragged
edge — so that one case kept the ragged cut. The line it needed was already
on the panel: the board hint. **The hint is a line the docket stack may
spend.** `_update_contract_panel` — the one place that knows what the stack
actually got — hides it when, and only when, the cut falls below the stack's
first line boundary (`_first_line_boundary`), and re-fits; that line is
exactly what puts the cut back on a boundary. "Sign more orders" is also the
least useful sentence on a clipboard whose every order is signed. Issue #13
made the same trade against the SHORT-STAFFED strip; the difference now is
that there is exactly ONE owner of the hint's visibility, so
`_update_staff_alert` no longer touches it.

**The legend folds, and folding is not a modal.** Its plate gains a heading
row — `CONTROLS` and a button naming its key — and folds to that stub, so
what is left still says what it is and how to bring it back; a panel that
vanishes completely teaches the player nothing about getting it back. Two
ways in, the button (a generous hit target that never takes focus, so the
number keys keep reaching the game) and **F1**, the first function key the
project binds, read in `_unhandled_input` beside TAB so it works in
isometric and first person alike. It is deliberately **not** a modal: it
neither closes an open panel nor is closed by ESC or X, and the "one modal
at a time" rule is untouched. State is session-only — a fold is a "not right
now", not a preference. Everything that measures the legend (the fixed-plate
avoid list of §2b point 33, the contract notice) reads its *current* rect,
so folding really does hand the space back.

### 5g. The configuration panel: the one place a building is told what to do

Every other panel in §5 reports. This one **commands**, and it is the only
one that does: click a hopper or a fluid tank and it opens beside that
building carrying its level, a **FILL** button, an **AUTO-FILL** toggle and
a **CONTENTS** picker of everything the simulation will let that vessel
hold. Machines get the same panel with a different middle — a state line and
the **MOTHBALL** toggle. It had no section here for a long time and it
should have had the first one, because §5a's ghost paragraph was describing
a **Q** key that had already been deleted in its favour (issue #116).

**It is anchored to the building, not to the screen.** The panel's position
comes from the footprint's own centre projected through the camera — the
inverse of the ray/plane pick that resolved the click — so it pans and zooms
with the thing it configures rather than sitting at a fixed corner that
would stop being *about* anything. That makes it a cursor-following panel by
§2b point 33, and it obeys that rule's whole discipline: it clamps to the
window first and is judged against the fixed plates *afterwards*, at the
position it will actually be drawn at.

**The order it yields in is the order of §5c, applied to one panel.** It
flips to the building's other side to clear the right-hand column, landing
its edge *on* the bound rather than at a fixed offset from the anchor. Then,
where a clear position exists, it steps down or out to clear the legend.
Where none exists it takes the corner and the **legend steps aside for as
long as the panel is open** (2026-09-21, issue #107). That is a ranking, not
a preference: the legend is static reference text that the player can fold
and mostly already knows, and the panel is live, was just asked for, and
carries the warning that switching a vessel's contents destroys what is in
it. The yield is not a fold — `_legend_collapsed` is the player's and
nothing here writes it — and the legend keeps *measuring* while it is
yielded, so the panel cannot drift off the corner, find it free and hand it
back once a frame.

**Inside the panel, the type yields before a word does.** A CONTENTS slot is
58×78 and deliberately tall enough for two lines, which is the right answer
for "Spice Mix": a name with a space wraps at a join it already has. A name
with no join and no room had nowhere legal to break, and "Molasses" was
drawn as `Molasse` / `s` with the tail outside the slot's frame (issue
#106). The font now shrinks a point at a time until the widest single word
fits, down to a floor, and the slot clips what a floor cannot rescue — a
crop, never a spill. The measurement is taken from the **font** at build
time rather than from the Label: a wrapping Label reports a 1px minimum
width and will agree that anything fits.

**A choice the player has not unlocked stays in the picker, padlocked.**
`data/unlocks.ron` holds ingredients back until a tier opens, and the market
simply omits a locked row — but the CONTENTS picker keeps it, dimmed, with a
🔒 where the selected slot's ✔ would be and a line under the row naming the
gate and the distance to it ("🔒 Liquid Smoke — BBQ Sauce, 4 of 10
deliveries"). The two panels differ because they answer different questions.
The market is a list of what you may buy **now**, and an entry you cannot buy
is noise in it. The picker is the full set of things this vessel will ever
hold, and a player who never sees liquid smoke until the day it appears has
no way to anticipate it — a gate nobody can see coming is a gate nobody plans
for (GAMEPLAY §2). Pressing a padlocked slot is refused by the simulation and
sounds refused; the panel does not grey the button out and swallow the click,
because the glyph, the dimming and the cue together say more than a dead
control does (2026-09-21, issue #90).

**And the panel is never wider than the constant that names its width.** The
positioning arithmetic above is written against `CONFIG_PANEL_WIDTH`; a
single unpinned footer Label made the real panel 99px wider than that, which
is 99px of legend it could then not get off. Every label on this panel pins
its width and wraps — §5c's rule, and the reason that rule exists.

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

**A third size, for a thing that is not an event at all (2026-09-16).** A
payout happens once and a case lands once; *crushing* is something a machine
is doing for eighty ticks at a stretch, so it gets neither a burst nor a
cue but a **beat** — a few flecks of pulp thrown out of the crusher's feed
pocket every ten ticks of its own work, arcing about a fifth of a tile over
the mouth and shrinking away as they drop back into it. Everything above
still holds: the distances are in `TILE`, the clock is the simulation tick
(so a pause holds the pulp in the air), and the fan's heights and sizes are
derived from the fleck's index rather than from `randf()`. Two rules are
new, and both came out of the render rather than the plan (§2b point 12):

- **A continuous effect is driven by the machine's own work clock**, so it
  inherits §2b point 28's narrow meaning of "working" for free — a crusher
  starved of tomatoes or blocked on a full output throws nothing, and the
  spray is another statement of the same fact its mechanism and its status
  lamp already make, not a decoration that runs whenever a machine exists.
- **At this camera a beat of three flecks is three stray pixels.** The
  first render read as a rash on the machine's roof rather than as a spit;
  five flecks at varied sizes read as one. Thrown higher it stops reading
  as something coming out of the pocket and starts reading as debris flying
  off the floor, so the arc is deliberately small and the fleck's exit is
  its shrink, not its landing. `scripts/screenshot-scene.sh --crushing` is
  the review, the way `--payout` is for the celebration: it waits for the
  crusher to be genuinely mid-batch and freezes the frame with pulp in the
  air, because a headless test can prove a fleck was thrown and cannot say
  whether the throw reads as crushing.

The work-clock rule is not only about machines. A belt's rubber is a
continuous effect too, and it used to run off the *global* simulation tick,
so a line backed up solid against a full vat drew its surface racing under
items that had not moved in minutes — the decoration running because the
belt existed, which is exactly what the rule above forbids (issue #79). The
fix gives every belt tile its own work clock: `BeltSegment::moved_ticks`
counts the ticks that tile actually advanced product on, `BeltSnapshot` and
`SplitterSnapshot` carry it, and the frame is
`floor(moved_ticks * BELT_FRAMES / BELT_TICKS_PER_TILE) mod BELT_FRAMES`.
Three consequences worth knowing before changing it:

- **The count belongs to the simulation, not to Godot.** A belt that stops
  moving stops dirtying its chunk, so the presentation layer is never told
  "this tile stood still" — it cannot derive the clock, only be handed it.
- **It is a counter, not a "moving" flag.** A flag would leave the pattern
  to a global phase, and the rubber would jump a fraction of a tile at the
  moment a jam cleared. A counter resumes on the phase it stopped on, and
  the pattern crosses a tile in exactly the ticks an item does.
- **An empty belt therefore shows no motion at all**, the same way an idle
  crusher throws no pulp. That is the rule, not an omission: running rubber
  is the statement "product is moving here", and on an empty belt it is a
  lie about a belt that could be carrying and is not.

## 7. Color & readability

- **Base palette is neutral/industrial**: steel greys, concrete, safety
  yellow/black, muted materials.
- **Saturated colour is reserved for meaning**: status lights, alerts,
  highlights, selection.
- **Never encode critical state in colour alone**: pair every status colour
  with a glyph, word, shape or position.
- **A model that displays stock is never painted the colour of stock** (the
  stock shelving, 2026-09-21). Its bays are tinted per ingredient, so the
  structure around them has to stay out of that colour space: the load beams
  were `paint_red` #82372F, the family the commonest stock — tomatoes — is
  already in. They are `paint_blue` now, the one paint the rest of that
  structure already wears, which spends no hue on the frame at all and
  leaves the tinted goods as the only saturated thing the kind shows.
  **Structure may separate from structure by value alone**: the beams and
  the uprights are one colour, and a beam still reads as a beam because its
  lit top face renders at 134.7 where the upright's vertical faces beside it
  are 55.7 — the isometric key light does the work a second paint was doing.
  **And when a colour on a measured asset is replaced, keep its luma and
  move only hue**: everything such an asset was tuned by (goods against
  backdrop, `RACK_TINT_DEAD_BAND`) is a relationship in value, so a
  value-neutral swap costs no re-measuring. The shipped beams measure 76.4
  where the red ones measured 74.7. A brighter green cut (#3A6B33, +11.6)
  was rejected on exactly that.
- **A world-space warning is a rhythm, not a lamp** (the dock-capacity
  change, 2026-09-16). A full delivery dock has to say so on the floor, not
  only on the clipboard, and it cannot say it by turning red: machine
  emission in this scene MULTIPLIES over vertex colour (§2b point 8's
  status-lamp rule, `_make_material`), so a red wash over a blue-grey deck
  is a dimmer deck, not a red one. The dock's own deck lights instead
  **pulse** — lit, dark, lit — off the simulation tick like every other
  animation here (§6a), which reads across a floor at any zoom and cannot
  be confused with a steady lamp. The words are still where the detail is:
  the hover says `⚠ FULL`, and the clipboard's ON THE DOCK line reads
  `48/48` with its bar at the ceiling (§5e).

## 8. What this doc does not cover

Gameplay and balance (GAMEPLAY.md); architecture, rendering pipeline, data
formats (ARCHITECTURE.md).

**Audio direction — decided 2026-09-11, amended 2026-09-12.** The first
decision, made by building the first sounds (§6a), was that sound in this
game is **feedback, not a soundtrack**: a cue fires because something
happened that the player needs to know about. On 2026-09-12 the owner asked
for music, and the direction is now in two halves:

- **Effects are feedback.** Fourteen cues (`godot_project/audio.gd`): the
  payout chime and the dock knock, and since the audio pass one for every
  player action — a tile placed (a low clunk with a metal tick), picked up
  (two rising blips), rotated (a ratchet), a purchase (a short till, a
  fifth under the payout so it reads as a receipt rather than an earning),
  a signature (pen scratch and stamp), hire and fire (a rising and a
  falling fifth), a hotbar pick (one dry tick), a modal opening and closing
  (two notes up, the same two down) — and for the two things the game does
  to the player unasked: a **refused** command (a dull double buzz, the one
  cue shared by every "no": unaffordable, blocked, already signed) and a
  **lost contract** (two falling notes, held longest of all). The cue is
  fired by the handler that knows whether the simulation took the action,
  never by the click itself, so a click that did nothing sounds like
  nothing happened (§7: the sound is a channel of the same fact the prompt
  and the counters show). Still no idle machine loop and no ambient bed:
  a running factory is quiet apart from the dock.
- **Music is one looping track**, warm and unhurried (D minor, 92 BPM,
  eight bars: a detuned pad, a plucked bass, a mallet arpeggio and a soft
  kick, rim and hat), synthesised like the cues and owned by the `Music`
  autoload so it starts on the main menu and carries across the cut into
  the factory without restarting. It **ducks** under the pause menu rather
  than stopping — the factory has stopped, and the score stepping back says
  so, where silence would read as a crash — and it has its own MUSIC
  ON/OFF switch beside SOUND ON/OFF on the pause menu. It is a bed, not a
  signal: nothing in it ever tells the player anything, which is what makes
  it safe to switch off.

Every sound is **synthesised in GDScript at runtime** into an
`AudioStreamWAV` rather than committed as a `.wav` binary — a waveform
written as code is diffable, reviewable and tweakable the same way
`assets/voxels/generators/` makes the models code (§2a point 1), and it is
deterministic, so a test can assert two renders are byte-identical
(`test_audio.gd` does, for every cue and for the track). The track's twenty
seconds of PCM cost about 0.7 s of GDScript, so it is rendered on a thread
the first time it is needed and never under a headless run. What is still
undecided is whether a larger soundscape (machine loops, room tone, a second
track for a bigger factory) stays synthesised or moves to recorded samples;
the loop's lo-fi character is the ceiling of the current approach, and it
suits the voxels.

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
