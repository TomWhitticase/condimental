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
3. **Viewing is a headless render step, not opening a GUI.** After generating or changing a model, render it with `vengi-voxconvert` in isometric thumbnail mode and read the resulting PNG. This is how a Claude Code session actually looks at the asset it just made and confirms it matches intent — no interactive 3D tool, and no dependency on computer-use-style GUI automation, which would be slow and imprecise for placing individual voxels anyway.
4. **Escape hatch for genuinely hand-sculpted assets.** Not everything is worth parametrizing — a one-off detailed hero prop may just be easier for a human to sculpt directly in `vengi-voxedit`. That's fine: commit the resulting `.vox` with no generator behind it, and mark it clearly (e.g. a note next to it) as hand-authored, so nobody later goes looking for a generator that doesn't exist.
5. **One shared, checked-in color palette**, referenced by every generator, rather than each asset inventing its own colors. This is what actually keeps the "shiny metal vs. flat everything else" material rule (§4) and the overall factory palette (§7) consistent across dozens of separately-generated assets instead of drifting per-asset.

**Proposed layout** (exact paths can shift once the Godot project exists, but the shape shouldn't):

```
assets/voxels/
  palette.ron       # shared color palette — every generator references this, none invent their own
  generators/        # Rust source: one generator per asset (or grouped by category)
  models/             # generated .vox files — build output; never hand-edited
  previews/            # generated PNG thumbnails for visual review — gitignored, regenerate on demand
```

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

- **Clean and simple, Minecraft-adjacent**: chunky panel borders, flat-shaded slot grids, a limited, consistent color palette per panel, generous hit targets. Avoid modern skeuomorphic-app UI (gradients, drop shadows everywhere, thin hairline fonts) — the UI should feel like it belongs in the same voxel world as the factory.
- **Icons are rendered from the real assets**: item/recipe/machine icons are small isometric renders of the actual voxel models (the same asset the player sees in the world, rendered to a thumbnail), not separately-drawn 2D icon art. This keeps the UI and the world visually consistent and halves the art workload per item.
- **Diegetic paper-and-metal widgets**: recipe lists, orders, and objectives are framed as **clipboards and checklists** — a paper texture on a metal clipboard clip, checkbox-style tick marks for completed items — rather than flat modern list UI. This is a deliberate physical/tactile framing choice that fits the industrial setting (a factory floor clipboard, not an app notification list).
- **Typography**: a simple, sturdy, slightly blocky sans-serif — readable at UI sizes, evokes stenciled factory signage/pixel fonts without tipping into illegible novelty pixel fonts.
- **Layout conventions**: a Minecraft-style hotbar/quick-access strip along one screen edge; build/recipe menus as grids of voxel-rendered icon slots; hover tooltips for recipe/machine detail rather than dense always-on text.

## 6. Feedback, motion & animation

- **Chunky, snappy animation** on voxel assets — simple rigs or blocky procedural motion (the same instinct as Minecraft item-entities bobbing, or Crossy Road's hopping animals) rather than smooth organic animation, which would clash with the voxel medium.
- **UI transitions are quick and simple** — panels slide/flip open rather than easing through elaborate modern-app motion curves. A clipboard "flipping open" is a good mental model for how a UI panel should transition.
- **Juice stays legible over spectacle**: particle effects (steam, sparks, liquid splashes, an item popping into existence on production) should read clearly even when a large factory floor has dozens of these happening at once — favor short, clear bursts over lingering elaborate effects.

## 7. Color & readability

- **Base palette is neutral/industrial**: steel greys, concrete tones, safety yellow/black striping, muted material colors — this is the "resting state" of the factory floor.
- **Saturated color is reserved for meaning**: status lights, alerts, UI highlights, and selection/hover states use bright, saturated color specifically so they stand out against the neutral base — don't dilute this by using bright color decoratively elsewhere.
- **Never encode critical state in color alone** — pair every status color with an icon or shape distinction too (a blocked machine gets both a red tint *and* a distinct icon, not just red), for basic colorblind accessibility.

## 8. What this doc does not cover

- **Gameplay mechanics, recipes, progression, balance** — a game-design concern, not an aesthetics one; not yet written up anywhere.
- **Simulation architecture, rendering pipeline, engine/tooling choices, data formats, performance** — see [docs/ARCHITECTURE.md](ARCHITECTURE.md).
- **Audio direction (music, SFX, ambience)** — not decided yet; would warrant its own doc when it is.

## 9. Open questions (not yet decided)

- Exact camera angle: true 2:1 isometric vs. a Sims-style ~3/4 dimetric — needs a side-by-side prototype before locking in, since it affects how voxel assets get modeled.
- Concrete palette values (exact hex ranges per material category), rather than the directional description in §7.
- Specific typeface.
- Whether player-visible characters (if any) share the 32×32×32 tile scale from §2 or use a different scale for facial/readability reasons — common in voxel games (e.g. characters are often modeled at a different fidelity than environment tiles).
- Which Rust `.vox`-writing approach (§2a) ends up used — re-check `vox-format`, `vox_writer`, and `create_vox`'s maturity at implementation time rather than assuming today's research still holds.
- Which specific Godot `.vox` import plugin (§2a) gets adopted — pick and pin one during milestone 1.

---

## 10. References (asset pipeline research, §2a)

- [MagicaVoxel .vox file format spec — ephtracy/voxel-model](https://github.com/ephtracy/voxel-model/blob/master/MagicaVoxel-file-format-vox.txt) — the community-maintained format spec
- [Kaitai Struct: MagicaVoxel File format](https://formats.kaitai.io/magicavoxel_vox/) — cross-language parser generator spec for `.vox`
- [`vox-format` — crates.io](https://crates.io/crates/vox-format) — Rust `.vox` reader/writer; writer support should be re-verified before relying on it (see open questions)
- [`vengi` — voxel-art tools (editor, thumbnailer, format converter)](https://github.com/vengi-voxel/vengi) — the headless CLI (`vengi-voxconvert`) this plan uses for format conversion and isometric PNG thumbnail rendering
- [Godot-MagicaVoxel-Importer-with-Extensions](https://github.com/qbradq/Godot-MagicaVoxel-Importer-with-Extensions) and [godot-vox-import](https://github.com/cschram/godot-vox-import) — candidate Godot 4 `.vox` import plugins; pick one at milestone 1
