# Sauce Factory — UI/UX & Visual Style Plan

This doc is about **aesthetics and UI/UX only** — what the game looks and feels
like to look at and interact with. It deliberately does not cover gameplay
mechanics, recipes, progression, or how any of this is built/rendered
under the hood — that's [docs/ARCHITECTURE.md](ARCHITECTURE.md). The two
should be read as a pair: ARCHITECTURE.md is "how it runs and is built,"
this doc is "how it looks and behaves to the player."

---

## 0. At a glance

| Aspect | Direction |
|---|---|
| Camera | Fixed orthographic isometric/dimetric, tile-based grid |
| Art medium | Full 3D voxel models (not flat-textured cubes) |
| Asset scale | Every tile = a 32×32×32 voxel volume, consistent across all assets |
| Theme | Grounded, semi-realistic 21st-century industrial food factory |
| Lighting | Real-time dynamic lighting, soft shadows, light shaders — "modded Minecraft with shaders," not photoreal |
| Materials | Mostly matte/simple voxel materials; metals are deliberately shinier |
| UI | Clean, simple, Minecraft-adjacent; diegetic clipboard/checklist motifs |

Reference points for the overall feel: **Crossy Road** (chunky, readable voxel simplicity), isometric city/sim games like **The Sims** and **Townscaper**-style titles (fixed dimetric camera, dollhouse readability), **modded Minecraft with a shader pack** (voxel base + real lighting, not vanilla-flat).

---

## 1. Camera & world presentation

- **Fixed orthographic isometric (dimetric) camera** — no perspective distortion, consistent tile footprint on screen regardless of position. This is the classic "isometric factory/city sim" look, not a free 3D camera.
- **Tile-based world grid**, always visually legible as a grid — the player should always be able to tell exactly which tile something occupies.
- **Rotation**: snap-rotate the camera in 90° increments rather than free-orbit. Voxel assets are modeled with clean silhouettes from four fixed viewing angles; free rotation would expose angles that weren't art-directed for and would fight the low-poly/voxel read.
- **Zoom**: smooth zoom is fine (it doesn't break the voxel read the way free rotation does), within a capped range so the factory stays readable at both ends.

## 2. Art medium: voxel models, not textured blocks

This is a step up from vanilla Minecraft's approach (simple cubes with a 2D texture per face) — assets here are **full 3D voxel models**, sculpted in voxel space the way a MagicaVoxel model or a voxel-art character is, so machines, props, and scenery can have real shapes (rounded tanks, angled pipes, chamfered edges) rather than being blocky cubes with a picture on them.

- **Tile/asset scale**: one world tile corresponds to a **32×32×32 voxel volume**. This is the shared unit every asset is built against, so a machine spanning one tile, a pipe segment, and a crate all read as consistently-scaled objects sitting in the same world — the same role Minecraft's 16×16 texture grid plays for scale consistency, just applied to full voxel models instead of flat textures.
- **Authoring**: voxel-model tools with a palette-based workflow (e.g. MagicaVoxel-style: a fixed color palette per model rather than photo-textures) keep assets visually consistent and cheap to produce/iterate on — palette swaps can restyle a whole asset without remodeling it.
- **Silhouette-first modeling**: because the camera is fixed-angle (§1), assets should be designed to read clearly from the four snap-rotation angles specifically, not sculpted for arbitrary viewing angles.

### 2a. Asset pipeline: file format, tools, and how assets actually get made

Neither of us had prior experience with voxel-asset pipelines, so this was researched rather than guessed — findings and the resulting decision below. The extra constraint driving the decision, beyond "what do other voxel games do": **every asset needs to be creatable, editable, and viewable by Claude**, not just by a human artist in a GUI. That constraint rules out a few otherwise-reasonable options and points at a specific pipeline.

**Findings:**

- **`.vox` (MagicaVoxel) is the de facto standard voxel-model format** across the voxel-art/voxel-game ecosystem. It's a simple, openly-documented RIFF-style format (a full community spec exists at [ephtracy/voxel-model](https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt), plus a [Kaitai Struct spec](https://formats.kaitai.io/magicavoxel_vox/) that auto-generates parsers for most languages), with open-source encoder/decoder libraries in Rust, Python, and JS.
- **Godot 4 already has community import plugins for `.vox`** — e.g. [Godot-MagicaVoxel-Importer-with-Extensions](https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions) and [godot-vox-import](https://github.com/cschram/godot-vox-import) — which import a `.vox` file directly as a mesh or `MeshLibrary`, hierarchy and all, and are maintained against current Godot 4.x. This means `.vox` can be the actual asset format Godot imports — no bespoke conversion step needed on the engine side. Pick one specific plugin during milestone 1 and pin it like any other dependency (ARCHITECTURE.md §1a).
- **[`vengi`](https://github.com/vengi-voxel/vengi)** is a free, open-source, actively-maintained (2026 releases, macOS/Linux/Windows builds) suite of voxel tools: an interactive editor (`vengi-voxedit`) and, more importantly for this project, a fully **headless, scriptable CLI** (`vengi-voxconvert`) that converts between ~30 voxel formats and can **render a model to a PNG thumbnail from the command line** — including an `--isometric` render mode, which happens to match this doc's fixed isometric camera (§1) exactly. Zero GUI interaction required for either job.
- Rust `.vox` **reading** support is solid (the `vox-format` crate). Rust `.vox` **writing** support is less mature as of this research — `vox-format`'s own writer is flagged work-in-progress, with smaller alternatives (`vox_writer`, `create_vox`) also existing. This should be re-checked at implementation time rather than assumed; the format itself is simple enough (a handful of RIFF chunks) that writing a minimal encoder directly is a realistic fallback if no library is in good enough shape.

**Decision — the pipeline:**

1. **Source of truth is code, not the binary model file.** Each voxel asset is a small, parameterized Rust generator function (e.g. `fn simmer_pot() -> VoxModel`), built from composable primitives (cuboid, cylinder, etc.). Rust, not a second scripting language, because it's already this project's primary language (ARCHITECTURE.md §1) — no reason to add Lua or anything else just for asset generation. A generator is plain text, fully diffable, and squarely in what an LLM agent is good at writing — this is the same "text is the interface" principle the rest of the project is built on, applied to art assets instead of code or scene files.
2. **The `.vox` file is a generated build artifact, never hand-edited.** If a shape needs to change, the generator changes and gets re-run — a `.vox` file that's silently diverged from the generator that supposedly produced it is a bug waiting to bite the next session that regenerates it and wipes someone's manual tweak.
3. **Viewing is a headless render step, not opening a GUI.** After generating or changing a model, render it to an isometric PNG thumbnail with vengi and read the resulting image (see §2b point 3 for the exact tool and flags). This is how a Claude Code session actually looks at the asset it just made and confirms it matches intent — no interactive 3D tool, and no dependency on computer-use-style GUI automation, which would be slow and imprecise for placing individual voxels anyway.
4. **Escape hatch for genuinely hand-sculpted assets.** Not everything is worth parametrizing — a one-off detailed hero prop may just be easier for a human to sculpt directly in `vengi-voxedit`. That's fine: commit the resulting `.vox` with no generator behind it, and mark it clearly (e.g. a note next to it) as hand-authored, so nobody later goes looking for a generator that doesn't exist.
5. **One shared, checked-in color palette**, referenced by every generator, rather than each asset inventing its own colors. This is what actually keeps the "shiny metal vs. flat everything else" material rule (§4) and the overall factory palette (§7) consistent across dozens of separately-generated assets instead of drifting per-asset.

**Layout** (proposed here originally; built exactly this way during the
visual-polish pass, so this is now descriptive, not aspirational):

```
assets/voxels/
  Cargo.toml        # the `voxgen` crate — a workspace member, run with `cargo run -p voxgen`
  palette.ron       # shared color palette — every generator references this, none invent their own
  generators/        # Rust source: one generator per asset (or grouped by category)
  models/             # generated .vox files — build output; never hand-edited
  previews/            # generated PNG thumbnails for visual review — gitignored, regenerate on demand
```

### 2b. How the pipeline actually turned out (built 2026-09-07)

Everything above held up. Four things are worth writing down because they
were only discoverable by building it:

1. **No Rust crate can write the `.vox` files this pipeline needs**, so
   `generators/vox.rs` contains a small hand-written encoder — the fallback
   §2a already anticipated. `vox-format` is still parser-only; `vox_writer`
   0.1.9 does write `SIZE`/`XYZI`/`RGBA` but emits **no scene-graph chunks**,
   and the Godot importer below cannot read a file without them. The format
   is simple enough that encoding it directly is ~150 lines with no
   dependency risk, and it also lets us emit `MATL` material chunks.
2. **One asset becomes one mesh per material class.** A voxel mesh imported
   into Godot carries vertex colours but a single material, which cannot
   satisfy §4's rule that steel is *noticeably* shinier than concrete. So
   `voxgen` writes both a combined `name.vox` (for previews and for a human
   opening it in an editor) and `name.<class>.vox` per material class the
   asset uses; Godot rebuilds one `StandardMaterial3D` per class from
   `palette.ron`'s own numbers and reassembles the asset as a node with one
   `MeshInstance3D` per class.
3. **Reviewing a model headlessly works, with one wrinkle**:
   `vengi-thumbnailer -i model.vox -o out.png -s 512 --image --isometric` is
   the incantation. `vengi-voxconvert -o out.png` is *not* — that writes one
   PNG per Z-slice of the volume, which is not a thumbnail. And the
   thumbnailer renders through OpenGL, so forcing `SDL_VIDEODRIVER=dummy`
   makes it fail; it needs a real video driver even though nothing is
   displayed. `scripts/build-voxels.sh` wraps all of this.
4. **Godot must run `--import` twice on a cold cache.** The `.vox` importer
   is an editor *plugin*, and a plugin is only registered on the pass after
   the one that first scans the project — so the first import silently skips
   every `.vox` and the second imports them all. This bites any CI setup that
   assumes one import pass is enough.
5. **A liquid is its own material class, not a recoloured matte one
   (added 2026-09-07 with milestone 6's pipes).** `palette.ron`'s
   `materials` table gained a fourth class, `liquid` — a dielectric like
   matte, but far glossier (roughness 0.14) — because §4's "visible liquid
   surfaces in tanks/pipes with subtle shading/reflection" is a *material*
   property, and point (4) above already settled that material class is
   palette data rather than a per-asset decision made in GDScript. What is
   NOT in the palette is the fluid's hue: which liquid is in a pipe is live
   simulation data, so the fluid model's voxels are near-white and
   `main.gd` multiplies a per-fluid albedo tint over them (the same shape as
   its existing item-id → model mapping). One consequence worth knowing:
   the fluid model is deliberately **one voxel tall** and scaled vertically
   to the segment's live fill fraction — the encoder puts a model's origin
   at its footprint centre *at ground level* (see §2b point 1's encoder), so
   a one-voxel slab's base is exactly its node origin and a fill level needs
   no pivot arithmetic.
6. **The pipe run is stainless steel, and it is modelled hollow with an
   open inspection channel.** Steel rather than PVC because §3 names both
   but §4 makes metals the deliberate shiny exception so machinery pops —
   and because GAMEPLAY.md §2/§4 reserve non-standard piping material as a
   *later* progression gate (tier 5's acid-resistant equipment), which the
   baseline run shouldn't pre-empt. Hollow because the liquid has to be
   *seen*: the first cut opened only the top half of the wall, and a
   rendered-scene review showed that hid the level below roughly half fill
   — exactly the state a player most needs to read (a draining pipe means a
   stalling machine). The opening now starts just above the bore floor,
   leaving side rails standing.
7. **UI icons (§5) turned out to need a second, separate render pass from
   the review previews above — not a smaller copy of them (built
   2026-09-07).** The obvious-sounding shortcut was "reuse
   `previews/*.png`, just smaller": checking the actual files first (per
   this section's own norm of verifying rather than assuming) ruled that
   out. `vengi-thumbnailer`'s `--image` mode always fills the whole frame
   with an opaque solid colour — confirmed by reading its `--jsonconfig`
   cvar dump (no background/alpha/clear-colour cvar exists at all) and by
   rendering through both `--image --isometric` and its default
   camera-based path and getting an identically opaque result either way.
   A preview is also deliberately padded, full-frame, and large (512px) —
   right for "look at the whole model," wrong for "sit in an 18px UI slot
   next to a line of text." So icon generation is a second binary,
   `assets/voxels/generators/icons.rs` (`cargo run -p voxgen --bin
   voxicons`), that calls the same `vengi-thumbnailer` and then does the
   work the tool itself has no flag for: flood-fill the background to
   transparent from the image border inward (not a flat colour-key, so a
   future asset using a near-black *interior* colour can't be falsely
   keyed), crop to content, and downscale onto a small (64px) square
   canvas. This is still "extend the existing offline pipeline" (§2a option
   (a), not the live in-engine `SubViewport` alternative that section also
   named) — it reuses the same external tool and the same `cargo xtask`
   staging pattern as `models/`, just as a distinct output (`assets/voxels/
   icons/`, committed) rather than a rename of `previews/`. First concrete
   use: `godot_project/main.gd`'s `_icon_for`, showing the contract's
   target item's icon next to its name in the economy readout line.

8. **A machine's material split follows "food contact is stainless,
   structure is painted or poured" (the "Machines improvement" milestone's
   ticket 4).** The first four assets with a real machine identity — a
   crusher, a mixer, a 2x2 cooking vat and a bulk hopper — needed a rule for
   §4's metal exception that wasn't decided per asset, and this is it:
   anything the product actually touches (crushing rolls, mixing vessel and
   agitator, the vat's body and dome, the hopper's funnel and chute) is the
   `metal` class, because in a real food plant it is stainless for exactly
   that reason; anything that only holds those parts up (concrete plinths,
   painted housings and skirts, hazard striping) is `matte`. The point is not
   that steel is prettier — it is that an all-steel machine reads as flat as
   an all-concrete one, so the matte structure is what gives the metal
   something to pop against. Emissive stays where it already was: status
   lamps only (§7).
9. **Review the rendered asset, not the generator — and expect to rework
   it.** §2a point 3 says to render and look; what building four machines
   added is how often looking *changes the design*, not just catches a bug.
   The crusher's rolls were originally sunk in a throat too deep for the
   fixed isometric angle to see into (the whole reason to expose a crushing
   mechanism is that it can be seen); the mixer's top-mounted gearbox — the
   accurate choice — covered the agitator, so it moved to a side bracket with
   a belt drive, which is also accurate and also legible; the cooking vat's
   first proportions read as a sphere on a plinth. None of those are visible
   in the generator source. Budget a rework pass per asset rather than
   treating the first render as verification.
11. **Only two of a machine's four vertical faces can ever be seen, and
    that decides how a junction is drawn (the "Visual overhaul round 2"
    epic's ticket 3).** §1's camera is fixed at a `(1, 1, 1)` view
    direction, so the only vertical faces with any pixels on screen are the
    ones facing +X and +Z-world. The production chain runs +X, which means a
    machine's OUTLET face is dead-on camera-facing and its INLET face points
    directly away — carving an inlet doorway into it, the obvious move, is
    invisible in the only view the game ships. Nor can an inlet be saved by
    sinking an open-topped channel into that edge: the sight line out of one
    travels up and over the machine's own body, so anything as tall as a
    mixing drum occludes its own inlet. The resolution is asymmetric on
    purpose — **an outlet is a doorway on the +X face; an inlet is a hooded
    transfer housing whose CAMERA-FACING SIDE is cut away**, so the product
    path is shown from the side rather than through a mouth that cannot be
    seen. Both read by value contrast (palette-brightest frame, genuinely
    black opening, one brass threshold) rather than by detail, because at
    scene scale a junction is a dozen pixels; a mid-grey interior renders as
    an empty portal and a hazard stripe across the one camera-facing edge
    renders as a dark bar, both of which were built and rejected on the
    render. Corollary worth knowing before drawing any new junction: a
    machine big enough to stand between its own inlet and the camera (the
    2x2 cooking vat) needs its opening widened along the face until it
    reaches an unoccluded corner, and a *neighbouring building* can hide a
    correct junction outright — the vinegar tank stands one tile diagonally
    in front of the crusher's discharge and does exactly that.

12. **The full-scene render catches a class of bug no headless test can.**
    `main.gd`'s item-id → model mapping is presentation-only with a fallback,
    so when a milestone renamed a chain intermediate the mapping silently
    went stale and two stages of the chain shipped *cardboard boxes* down the
    belt where pails of sauce belonged — every structural test still passed,
    because a box is a perfectly valid mesh. `scripts/screenshot-scene.sh`
    is what found it. Any presentation-side lookup keyed by a content id has
    this failure mode; the rendered scene is the only check for it.

13. **A process an order of magnitude longer than a craft cycle needs a
    passive readout, and on this camera that readout has to emit its own
    light (ARCHITECTURE.md §9 milestone 7's aging rack).** Every machine
    already reports *running vs. idle* through its status lamp, and the hover
    tooltip already reports progress — which is enough while every process on
    the line is one to four seconds long. It stops being enough at 2400
    ticks: the only question a player has about a machine that will sit
    loaded for over a minute is "how far through is it", and an answer you
    have to park a cursor on is not an answer you can glance at. So the rack
    carries a **maturation gauge**: a slot in its corner post with an
    emissive amber bar scaled to the live process fraction — the fourth use
    of the same one-voxel-slab-scaled-every-frame mechanism the pipe liquid,
    the hopper fill and the tank level already use (point 5 above), not a
    fifth mechanism.

    Two things about it were only discoverable by rendering it. **The gauge
    face is permanently unlit.** §1's fixed camera means the only visible
    vertical faces are +X and +Z-world (point 11), and on this scene's key
    light *both* of those read as shadow side — every bright surface in a
    machine close-up is a horizontal one. A painted or brushed-metal gauge
    would therefore be a dark stripe on a dark column; emissive is not
    decoration here, it is the only thing that reads. **And an empty gauge
    has to show its own scale.** With the bar hidden below its sub-voxel
    threshold the slot is just black, so the player cannot see how far the
    bar still has to climb — the fix is a lit green line at the head of the
    scale, always on: a finish line, which is a *position* rather than a
    state encoded in colour (§7). Amber climbing toward green is the whole
    readout.

    Corollary for the next machine that wants one: this is keyed on machine
    *kind* (`main.gd`'s `MACHINE_PROGRESS_GAUGES`) and deliberately sparse. A
    bar tracking the crusher's 60-tick cycle would be a flicker, and five
    flickering bars along a line is noise, not information.

14. **Wood is the one deliberate exception to point 8's "food contact is
    stainless" rule.** The aging rack's casks are oak because in a real
    barrel-aging room the wood *is* the process, not a material someone
    settled for — and GAMEPLAY.md §2's tier 6 names "barrel-aged" outright.
    It is also, with the packer's cardboard, one of only two large warm
    surfaces in the factory, so its palette entry is deliberately browner and
    redder than `cardboard`; shape does most of the work of telling them
    apart (a round hooped cask against a folded flat case) and hue does the
    rest. Recorded here so a later pass doesn't "correct" the barrels to
    steel for consistency.

15. **The machine with no mechanism is a legitimate silhouette.** Every other
    kind on this line is identified by something that moves — flywheels, an
    agitator, a filling gantry, a taping head (points 8/9). The aging rack
    has none, on purpose: what distinguishes it from its neighbours is that
    it only *waits*, and a rack of sealed casks with nothing driving them is
    the honest shape for that. Casks stand on end rather than lying in a
    cradle for the same camera reason as everything else here — lying down,
    a cask presents one round head and then recedes into the tile as a long
    ridge, and four of them render as corrugation; standing, each is a
    separate round-topped object with a visible head and bung, and four of
    those read as four things resting.

16. **The one building that is a place rather than a machine, and the
    occlusion rule that decided its layout (the delivery-dock ticket).**
    Every other kind on the line is equipment; the delivery dock is
    architecture — a poured platform at trailer-bed height with a striped
    edge, rubber bumpers, a despatch door under a canopy — because that is
    what it *is* (GAMEPLAY.md §6's "somewhere product is sent to be sold"),
    and because it is the only silhouette on the line that could not be
    mistaken for another housing with a mechanism on top.

    Point 11 established which faces this camera can see. What building the
    dock added is the **corollary about depth**, which matters far more for a
    building with an open floor than for a machine that is solid: a sight
    line leaving any point travels `(+x, -y, +z)` in model space, so anything
    at HIGH x and LOW y stands in front of everything behind it. The obvious
    design — a truck bay carved into the +X face, dead-on to the camera —
    was built first and reads beautifully on its own, and it walls off the
    entire deck, because every deck voxel's sight line has to pass through
    those last few columns on its way out. The tall structure therefore went
    to the FAR (high-y) side, where its own -Y face is still square to the
    camera and it occludes nothing, and the near half of the deck was left
    clear for the goods that stand on it. **Rule of thumb for the next
    building with something inside it: put the height at high y, put the
    content at low y.**

    Two smaller things came off the render and not off the generator. The
    platform was `concrete_dark` for its whole mass on the first cut, which
    is right for a floor slab's seam and wrong for a block this size — it
    read as a black pit with a lit rail round it. And the canopy stood two
    voxels clear of the wall on thin posts, which at scene scale is not read
    as a gap, it is read as a white shelf floating in front of a blue wall;
    it is integral with the wall's head now.

17. **What stands on the dock is drawn from the pool, not from the dock —
    and it is capped at two.** The dock's own buffer is a pass-through that
    empties every tick (`sim_core::sim::dock`), so a fill gauge driven from
    it would read zero forever: the same gauge-lies-about-the-mechanism
    failure the fluid tank refused when it chose constant head over its own
    fill fraction. The number that is real is the economy's shared
    `delivery_stock`, which is what the contract clipboard already labels ON
    THE DOCK, so the deck stands one case per unit of *that* — each drawn
    with the item's own model, through the same `ITEM_MODELS` table the belt
    uses, so what waits on the dock looks like what arrived on it.

    The cap is two because the deck is one tile and a case is a third of it:
    a rendered four-slot version buried the transfer housing, the despatch
    door and the canopy under a wall of cardboard. This is a glance-readable
    "goods are waiting", not a readout — the exact figure is on the clipboard
    and in the dock's own hover, neither of which is capped.

18. **A rotated placement needs no new geometry — and the junction art is the
    one thing that does not survive the rotation (the "Real Placement" epic's
    ticket 2).** Every asset here is modelled with its product path along +X,
    because until that epic +X was the only direction anything could face. A
    placement facing another way is therefore just the same mesh spun about
    its own origin — the encoder puts that origin at the model's footprint
    centre (point 1), so the node is re-centred on the rotated rectangle and
    nothing else changes. That covers belts, pipes, machines and buildings
    alike.

    What it does *not* cover is point 11's junction pieces, and the reason is
    point 11 itself: an outlet is a doorway carved through the camera-facing
    +X wall and an inlet is a hood cut open on that same visible side, so
    spinning either 180° puts the carved side where nobody can see it and a
    blank wall where the product path is — **visibly worse than the plain,
    symmetric tile**. So the belt's `belt_inlet`/`belt_outlet` variants stay
    east-only and three of the four facings a player can choose fall back to
    the plain model. That is a known, tested, accepted gap, not an oversight:
    a four-directional junction pass is its own art ticket, and until it
    happens the fallback is what keeps a turned line honest rather than
    wrong. A machine turned away from the camera is a milder case of the
    same thing — its own doorway is still in the right place relative to its
    body, it is simply on the side you cannot see.

## 3. Theme & setting

**A grounded, semi-realistic 21st-century industrial food factory** — stainless steel tanks, rubber conveyor belts, PVC/steel piping, poured concrete floors, hazard-stripe signage, factory floor lighting, pallets and crates, loading docks. The tone is closer to "a real bottling/food-processing plant you could actually walk into," rendered in voxels, than a whimsical toy-factory aesthetic.

- Lean into real industrial visual language (safety signage, cable runs, drainage grates, steel walkways) — the voxel medium provides the charm, the setting itself should still feel plausible and researched rather than cartoonish.
- Avoid candy-colored or overtly cutesy factory tropes — the sauce theme should come through in what the factory *produces and processes*, not in exaggerating the factory itself into a toy.

## 4. Materials & lighting

- **Baseline materials are simple and mostly matte**: painted metal, concrete, plastic, rubber — flat-shaded or close to it, in keeping with a voxel-art look.
- **Metals get a deliberate exception**: stainless steel tanks, machine housings, pipes, and fittings should read as noticeably shinier/more reflective than the rest of the palette. This contrast is intentional — it's what makes the "important machinery" pop visually against the plainer floors/walls/crates, and it's also just accurate to what a real steel-and-chrome factory floor looks like.
- **Lighting**: real-time dynamic lighting with soft shadows, mild bloom on bright highlights/emissives, and light ambient occlusion in voxel crevices for depth. Reference point is a modded-Minecraft shader pack (e.g. the Complementary/BSL family) — noticeably better and more atmospheric than flat vanilla lighting, but not aiming for path-traced photorealism.
- **Emissive/readability elements**: status lights, indicator screens, warning strobes on machines should glow/emit — both an aesthetic touch and a functional readability signal (see §7).
- **Atmosphere**: steam and smoke particles from cooking/heating equipment, visible liquid surfaces in tanks/pipes with subtle shading/reflection — these sell the "working factory" feeling more than static geometry alone.

## 5. UI style

> **Status:** most of §5 is built. The contract readout is a real diegetic
> clipboard (paper panel, steel clip bar, checkbox tick marks, voxel-rendered
> item icons), machine/pipe detail is hover-only rather than dense always-on
> text, and the build menu is a grid of voxel-rendered icon slots that now
> also *places* what it sells (see §5a). The scene's other readouts still use
> the plain dark plate, deliberately — that is machine-side factory UI, and
> the clipboard motif is reserved for orders and checklists. Not built: the
> hotbar/quick-access strip.

- **Clean and simple, Minecraft-adjacent**: chunky panel borders, flat-shaded slot grids, a limited, consistent color palette per panel, generous hit targets. Avoid modern skeuomorphic-app UI (gradients, drop shadows everywhere, thin hairline fonts) — the UI should feel like it belongs in the same voxel world as the factory.
- **Icons are rendered from the real assets**: item/recipe/machine icons are small isometric renders of the actual voxel models (the same asset the player sees in the world, rendered to a thumbnail), not separately-drawn 2D icon art. This keeps the UI and the world visually consistent and halves the art workload per item.
- **Several concurrent contracts are one board of dockets, not one clipboard each** (built alongside `data/contracts.ron`'s arrival). GAMEPLAY.md §6's design pressure is that several standing orders compete for one factory's throughput, so all of them have to be legible at once. The panel keeps its single sheet and single clip bar and prints one **docket card** per contract on it — the checkbox's own paper tone plus a steel hairline, i.e. no third colour. A separate clipboard per contract repeats the frame and the clip for no new information; a single flat list of checklist rows loses which rows belong to which order as soon as two contracts want the same item. What is shared appears once, above the dockets — the money figure (there is exactly one currency, GAMEPLAY.md §1) and the delivery pool every contract draws from; what is per-contract (checklist, payout, cadence countdown, delivered/missed tally) appears on its own docket. Past a capped height the stack scrolls rather than growing off-screen.
- **Diegetic paper-and-metal widgets**: recipe lists, orders, and objectives are framed as **clipboards and checklists** — a paper texture on a metal clipboard clip, checkbox-style tick marks for completed items — rather than flat modern list UI. This is a deliberate physical/tactile framing choice that fits the industrial setting (a factory floor clipboard, not an app notification list). [GAMEPLAY.md](GAMEPLAY.md) §6's contracts (a standing order for a sauce mix, delivered on a cadence) are the concrete, driving use case for this motif — a contract *is* a clipboard.
- **Typography**: a simple, sturdy, slightly blocky sans-serif — readable at UI sizes, evokes stenciled factory signage/pixel fonts without tipping into illegible novelty pixel fonts.
- **Layout conventions**: a Minecraft-style hotbar/quick-access strip along one screen edge; build/recipe menus as grids of voxel-rendered icon slots; hover tooltips for recipe/machine detail rather than dense always-on text.

### 5a. Build mode: the ghost, and what the next click will do

The world starts empty and the player places every tile in it (the "Real
Placement" epic), so "what will this click do" became a question the UI has to
answer continuously. It is answered by two things at once, deliberately:

- **A translucent tinted ghost of the real voxel model**, standing on the tile
  under the cursor, turned to the facing the player has chosen. It is the
  asset that will actually be built — including the belt's context-dependent
  variant — so the ghost is a preview rather than a marker. The tint
  *multiplies* over the model's own vertex colours rather than replacing them:
  a flat wash reads as "a tile is involved" and loses which thing.
- **A prompt beside the cursor** that names the kind, its facing and the
  action in words and a glyph: **✔ PLACE**, **✗ BLOCKED**, **✗ NONE OWNED**,
  **✔ REMOVE**. This is §7's never-colour-alone rule doing real work — the
  glyph and the wording alone are enough to know whether the click will
  succeed, with the tint and the panel's own border as reinforcement.

Two conventions worth keeping. **Green means "this click builds"; red means
"this click destroys, or refuses"** — so a valid removal is framed in red,
not green, even though the click will succeed. And **removal is a mode (X),
not a modifier-click**: it is the one destructive action in the game (what a
removed thing was holding is destroyed), and a mode announces itself in the
control legend and in the prompt, where a held modifier cannot.

A build-menu slot carries two actions on the two mouse buttons — left-click
buys one, right-click places one you own — rather than one button that
changes meaning with the owned count. The latter is shorter to explain and
worse to use: it makes buying a second of anything impossible without placing
the first.

## 6. Feedback, motion & animation

- **Chunky, snappy animation** on voxel assets — simple rigs or blocky procedural motion (the same instinct as Minecraft item-entities bobbing, or Crossy Road's hopping animals) rather than smooth organic animation, which would clash with the voxel medium.
- **UI transitions are quick and simple** — panels slide/flip open rather than easing through elaborate modern-app motion curves. A clipboard "flipping open" is a good mental model for how a UI panel should transition.
- **Juice stays legible over spectacle**: particle effects (steam, sparks, liquid splashes, an item popping into existence on production) should read clearly even when a large factory floor has dozens of these happening at once — favor short, clear bursts over lingering elaborate effects.

## 7. Color & readability

- **Base palette is neutral/industrial**: steel greys, concrete tones, safety yellow/black striping, muted material colors — this is the "resting state" of the factory floor.
- **Saturated color is reserved for meaning**: status lights, alerts, UI highlights, and selection/hover states use bright, saturated color specifically so they stand out against the neutral base — don't dilute this by using bright color decoratively elsewhere.
- **Never encode critical state in color alone** — pair every status color with an icon or shape distinction too (a blocked machine gets both a red tint *and* a distinct icon, not just red), for basic colorblind accessibility.

## 8. What this doc does not cover

- **Gameplay mechanics, recipes, progression, balance** — a game-design concern, not an aesthetics one; see [docs/GAMEPLAY.md](GAMEPLAY.md).
- **Simulation architecture, rendering pipeline, engine/tooling choices, data formats, performance** — see [docs/ARCHITECTURE.md](ARCHITECTURE.md).
- **Audio direction (music, SFX, ambience)** — not decided yet; would warrant its own doc when it is.

## 9. Open questions (not yet decided)

- Exact camera angle: true 2:1 isometric vs. a Sims-style ~3/4 dimetric — needs a side-by-side prototype before locking in, since it affects how voxel assets get modeled. **Still open, but no longer undecided in practice:** the scene currently uses a true isometric view (an equal-parts `(1, 1, 1)` view direction), chosen because it keeps voxel cubes perfectly regular on screen. It is one constant in `main.gd` (`CAMERA_DIRECTION`), so the side-by-side prototype is cheap whenever someone wants it.
- Concrete palette values (exact hex ranges per material category), rather than the directional description in §7.
- Specific typeface.
- Whether player-visible characters (if any) share the 32×32×32 tile scale from §2 or use a different scale for facial/readability reasons — common in voxel games (e.g. characters are often modeled at a different fidelity than environment tiles).
- ~~Which Rust `.vox`-writing approach (§2a) ends up used~~ — **resolved
  2026-09-07**: none of them. All three were re-checked at implementation
  time exactly as this question asked, and none can write a file the Godot
  importer accepts; `assets/voxels/generators/vox.rs` is a hand-written
  encoder instead. See §2b point 1 for the specifics.
- ~~Which specific Godot `.vox` import plugin (§2a) gets adopted~~ —
  **resolved 2026-09-07**:
  [qbradq/Godot-MagicaVoxel-Importer-with-Extensions](https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions),
  vendored and pinned at commit `288f1d8` (plugin v1.2) under
  `godot_project/addons/`. The choice turned out not to be a choice: the
  other candidate this doc named (`cschram/godot-vox-import`) now redirects
  to that same repository, so there is one maintained Godot 4 `.vox`
  importer, not two.
- **Touch/mobile input is a real gap in this doc, flagged during review rather than designed here.** ARCHITECTURE.md targets PC/Mac/mobile, but §1, §5, and §6 above all implicitly assume mouse+keyboard (hover tooltips, precise clicks on hotbar slots, camera-rotation as a keypress/button). None of that transfers to touch automatically — hover has no touch equivalent, small slot grids need larger touch targets, and camera rotate/zoom need an explicit gesture mapping. Not designing this now since it's not needed until ARCHITECTURE.md milestone 10 (platform polish), but it should be resolved well before then, not discovered at that milestone.

---

## 10. References (asset pipeline research, §2a)

- [MagicaVoxel .vox file format spec — ephtracy/voxel-model](https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt) — the community-maintained format spec
- [Kaitai Struct: MagicaVoxel File format](https://formats.kaitai.io/magicavoxel_vox/) — cross-language parser generator spec for `.vox`
- [`vox-format` — crates.io](https://crates.io/crates/vox-format) — Rust `.vox` reader/writer; writer support should be re-verified before relying on it (see open questions)
- [`vengi` — voxel-art tools (editor, thumbnailer, format converter)](https://github.com/vengi-voxel/vengi) — the headless CLI (`vengi-voxconvert`) this plan uses for format conversion and isometric PNG thumbnail rendering
- [Godot-MagicaVoxel-Importer-with-Extensions](https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions) and [godot-vox-import](https://github.com/cschram/godot-vox-import) — candidate Godot 4 `.vox` import plugins; pick one at milestone 1
