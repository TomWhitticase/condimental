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
   progress gauge — do not invent another.
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
    any facing. The belt's direction mark is therefore a **filled** arrow
    (wedge head, short shaft) standing one voxel proud of the rubber in
    `hazard_yellow` (§7: a shape, in signage colour), and it reads at all
    four facings. **Raised, not recessed**: one edge of any recess runs along
    the sight line and shadows its own floor. Belts animate by **frames**:
    frame `k` is the same tile with the surface pattern advanced `k` quarter
    tiles along the path and nothing else changed; the pattern's period is
    one tile (a shorter one makes frames coincide or run backwards). A
    corner's surface is a quarter annulus about the tile corner between its
    two open faces, whose pattern is stretched to a tile's worth so it
    reaches the exit face in the same frame a straight would; its arrow
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

## 7. Color & readability

- **Base palette is neutral/industrial**: steel greys, concrete, safety
  yellow/black, muted materials.
- **Saturated colour is reserved for meaning**: status lights, alerts,
  highlights, selection.
- **Never encode critical state in colour alone**: pair every status colour
  with a glyph, word, shape or position.

## 8. What this doc does not cover

Gameplay and balance (GAMEPLAY.md); architecture, rendering pipeline, data
formats (ARCHITECTURE.md); audio direction (not decided).

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
