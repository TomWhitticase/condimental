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

Point numbers are cited by code comments; keep them stable — a number is
never reused and never moves. Full narrative in DECISIONS.md Part B. There is
no point 10, and the list runs 1-28, 35-37, 29-34: the three entries the
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
   painted or poured.** Emissive is for status lamps only.
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
  (`nothing to maintain yet`), plain-sentence company for two more lines:
  what asks for that category, and what the category does. **WHY:**
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
  entirely when none are, never reading `0 short`; and the panel opening
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
  always-visible footer line saying it is down there. A row
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
nobody can see is a modifier nobody uses. A hopper's
contents are chosen on the ghost (**Q** cycles raw items; the prompt names
it; the ghost shows a tinted fill).

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
those are real fractions. Stock on hand (the §5f plate) and the balance
itself are not — nothing in the game says how many tomatoes is "full" — so
they stay figures.
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
Per §5e there is no bar (nothing says how many tomatoes is "full"), so zero
is carried by the word `none` on a dimmed row rather than by a colour (§7),
and the heading keeps the `M: market` hint so the plate says where the
numbers come from.

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
