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
