# Sauce Factory — Development Log & Status Tracker

This doc answers "what's actually been done, and what's the state of things
right now" for whoever (human or agent) picks this project up cold. It's a
**log and status tracker, not a design doc** — the reasoning and specifics
live in [ARCHITECTURE.md](ARCHITECTURE.md), [UI_UX.md](UI_UX.md), and
[GAMEPLAY.md](GAMEPLAY.md); this file indexes *when* and *why* things changed,
and what's true *now*. It is deliberately **not** `@`-imported into
`CLAUDE.md` (unlike those three) — it will grow without bound over the life
of the project, so it's meant to be checked when you need history/status, not
force-loaded into every session's context.

---

## Current status

*(This section reflects "now" — edit it in place as things change. The Log
below is history and should only ever be appended to.)*

- **Stage:** Pre-alpha, but the core MVP loop is real end to end and it looks like a game. Milestones 1 (Scaffold), 2 (Mobile export spike), 3 (Core loop proof), 4 (First full chain), and 5 (Economy MVP) are all done — a real multi-stage production chain (tomato -> tomato sauce -> ketchup -> bottled ketchup -> boxed ketchup) that can deliver against a hardcoded contract to earn money, headlessly verified end to end, Android export proven working. On top of that, a visual-polish pass built UI_UX.md's voxel asset pipeline and turned the flat 2D placeholder scene into an isometric, real-time-lit 3D factory.
- **Repo:** Private GitHub repo (`TomWhitticase/sauce-factory`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
- **Design docs:** ARCHITECTURE.md, UI_UX.md, and GAMEPLAY.md are all written and cross-referenced; docs/README.md indexes them.
- **Milestone progress:** milestones 1-5 (Scaffold through Economy MVP) were built as an Orca orchestration Run (`run_51e5cb271445`), one Task per milestone, dependency-chained. **All 5 are complete and merged.** Check `orca orchestration task-list --run run_51e5cb271445 --json` for the DAG's final state. There are 11 milestones total, not 9 — a milestone 5 "Economy MVP" was added since the loop wasn't a game without it. **Milestone 6 (Fluids) is done**: its simulation half (pipe network, pressure equalization, a recipe requiring a liquid, the `SimBridge` fluid snapshot) and its presentation half (pipe/supply-tap/liquid voxel assets, a pipe run rendered from the simulation's own segment list, a liquid body whose height tracks live pressure, and pipe hover info in both view modes) have both landed.
- **Art/UI:** the voxel pipeline (UI_UX.md §2a/§2b) exists and is the source of every asset in the scene — including milestone 6's pipes and the liquid inside them, which added a fourth material class (`liquid`, a glossy dielectric) to `palette.ron` — Rust generators in `assets/voxels/generators/` produce `.vox` models, reviewed as isometric PNGs via `scripts/build-voxels.sh`, imported into Godot by a pinned community importer. UI_UX.md §5's real UI is now landed: the money/contract readout is a diegetic clipboard panel (paper background, steel clip bar, checkbox tick marks) with a voxel-rendered icon per required item, and per-machine detail (recipe/progress/buffers) is hover-only instead of dense always-on text, working in both view modes — a screen-centre crosshair stands in for the cursor in first-person — see the log entries below.
- **UI polish epic (done):** a Run (`run_cc6860507d36`) with 3 parallel tickets targeting UI_UX.md §5, all done and merged — hover tooltips for machines, a voxel-rendered icon pipeline, and the clipboard reskin. Check `orca orchestration task-list --run run_cc6860507d36 --json` for the DAG's final state.
- **Camera:** two view modes, toggled with TAB. Isometric (default): pan (WASD/right-drag/arrows) and zoom (scroll, trackpad, +/-); the fixed angle itself never changes, per UI_UX.md §1. First-person: a free-look "fly" camera (mouse-look, WASD move, SPACE/SHIFT up/down) — not a UI_UX.md concept, and not real player physics (no gravity/collision, this project has neither). 90° snap-rotation of the isometric angle is still not implemented (still a nice-to-have, not required).
- **Lighting:** the sky is the real background (`BG_SKY`), not a flat colour — see the "empty void" DEVLOG entry below for why that matters. `scripts/screenshot-scene.sh --first-person` reviews that mode's lighting specifically.
- **Open threads:** each doc's own "Open questions" section (ARCHITECTURE.md §11, UI_UX.md §9, GAMEPLAY.md §11) is the real TODO list at this stage — nothing is blocked, these are just undecided-and-known-undecided.

---

## How to use this doc

- **Newest entries at the top of the Log.**
- **Keep it high-level.** A few sentences per entry: what changed in general terms and why it matters for where the project is at, not a blow-by-blow of every file/decision — that detail already lives in the docs' own decisions/changes and in `git log`/commit messages. If you're tempted to write a bullet per commit, zoom out instead.
- One entry per meaningful chunk of work (a session, a design pass, a milestone), not one per commit.
- **Update "Current status" above whenever it goes stale.** That section is what anyone should read first; the log is for "how did we get here," briefly.
- **Not every commit needs an entry.** Small/trivial changes (typo fixes, a tweaked wording, a minor config change) don't warrant one — see CLAUDE.md's commit convention for the actual rule on when to add one.

---

## Log

### 2026-09-07 — Milestone 6, ticket 2: the pipe run you can actually see

The presentation half of ARCHITECTURE.md §9 item 6, on top of ticket 1's
simulation work below. `sim_core` was not touched at all: everything here is
drawn from the JSON that ticket already exposed. Three new voxel assets
(`pipe`, `pipe_source`, `pipe_fluid`) join the pipeline, and `main.gd` builds
the run from `fluid_snapshot_json`'s own segment list — how many pipe tiles
there are, where they sit, and which one is the supply tap are all the
simulation's answer, never a constant in GDScript, the same rule the belt
chain already follows.

The liquid is real geometry, not a HUD number: a fluid body sits in each
pipe's bore and is rescaled every frame to that segment's live pressure, so
you watch the run fill from the source and watch the inlet drop when the
machine takes a draw. That is UI_UX.md §4's "visible liquid surfaces in
tanks/pipes" taken literally — the pipe is modelled hollow with an open
inspection channel along its top so the level is legible from the fixed
isometric angle at any fill, not just a full one. Two decisions worth not
re-litigating: the run is **stainless steel** (UI_UX.md §4's shiny metal
exception; acid-resistant piping is GAMEPLAY.md tier 5's *later* gate, so
the baseline shouldn't pre-empt it), and palette.ron gained a fourth
material class, `liquid` — a glossy dielectric — so the wet look stays a
palette fact rather than a number hardcoded in GDScript. Only the per-fluid
*tint* is decided Godot-side, because which fluid is in the pipe is live
data.

Hovering a pipe now reports its fluid, level and pressure, through the exact
same screen-ray/plane-intersection path machines use — the tile-to-text
lookup was generalized (`_hover_text_at_tile`) rather than given a second
mechanism, so it worked in first-person via the crosshair with no extra
code. New `godot_project/test_fluids.gd` (wired into
`scripts/verify-core-loop.sh`) asserts the structural half headlessly, and
`scripts/screenshot-scene.sh --hover-pipe` was added so the tooltip and the
liquid could be reviewed as an actual image, which is how the sight channel's
height got fixed — the first cut hid the liquid below half fill, which is
exactly the state worth seeing.

### 2026-09-07 — Milestone 6, ticket 1: fluids in the simulation core

The sim half of ARCHITECTURE.md §9 item 6 (Fluids). New
`sim_core::sim::fluid` module implements §3.4's model literally — a
`FluidNetwork` is a run of pipe segments each holding a *volume* of one
fluid, with pressure (fill fraction) equalizing between adjacent segments
every tick; there is no droplet, no per-item entity, and all state is
fixed-point `Fx`. A recipe can now declare `fluid_inputs`, and a machine
gates and consumes them exactly the way it already gates and consumes solid
input: no fluid, no craft, and crucially no half-consumed solid input
either. `data/recipes.ron` moves to `schema_version: 2` and hooks the
ketchup stage up to vinegar, which is what GAMEPLAY.md §2's tier table
always said ketchup needed — an existing stage gained an ingredient, the
belt chain's shape is untouched.

Two scope decisions worth not re-litigating. First, the pipe run is
**fixed** — built once in `World::new`, an always-refilling off-map source
at one end feeding one machine at the other — the same deliberately-minimal
stance milestone 5 took with its single hardcoded contract; player-placed
pipes and §3.8's union-find topology recompute stay out until placement is
real. Second, **which** stage gets plumbed is still a pure data question:
`World` picks whatever recipe declares `fluid_inputs` and carries whatever
fluid it names, so no recipe id, fluid id or stage index is written down in
Rust. Fluid state is exposed to Godot as a new `fluid_snapshot_json`
(plus pipes in chunk snapshots and `query_tile`), polled per-frame like the
economy snapshot rather than dirty-tracked, since fluid moves nearly every
tick. Ticket 2 renders it.

### 2026-09-07 — First-person crosshair hover

Follow-up after the diegetic-UI epic landed and got a live review: hover
tooltips only worked in ISOMETRIC mode (FIRST_PERSON unconditionally hid
them, since the mouse is captured there and has no meaningful screen
position). Added a small screen-centre crosshair (`_crosshair`, visible only
in FIRST_PERSON) and pointed `_update_hover_tooltip` at the viewport centre
instead of the live cursor whenever `_view_mode` is FIRST_PERSON — the same
pure `_tile_under_screen_pos` math drives both, since `project_ray_origin`/
`project_ray_normal` work identically for an orthogonal or a perspective
camera. That did surface one real gap the fixed isometric camera never
could: looking up at the sky in first-person points the ray away from the
belt-surface plane, and the old math would have happily returned a phantom
tile "behind" the camera — `_tile_under_screen_pos` now checks `t < 0` and
returns `HOVER_NO_TILE` in that case. `test_hover.gd` gained checks for both
the crosshair-driven resolution and the new upward-look guard, verified by
aiming the first-person camera at a known machine tile the same way the
existing isometric checks aim the cursor, no live Input needed.

### 2026-09-07 — Machine hover tooltips (UI_UX.md §5, one piece of the diegetic-UI epic)

Ticket A of the three-ticket parallel push below. Per-stage recipe/progress/
input/output detail used to sit as always-on text in the HUD panel;
UI_UX.md §5 explicitly calls for that as a hover tooltip instead. The
always-on panel is now gone entirely — hovering a machine in ISOMETRIC mode
shows a small panel near the cursor with that machine's live data, and the
tooltip never appears in FIRST_PERSON (the mouse is captured there for
looking around, so "hover" has no meaning).

Follows this file's established pure-function/thin-Input-wrapper split
(`_apply_pan`/`_pan_input_direction` etc.): `_tile_under_screen_pos` casts
the camera's ray for an explicit screen position and intersects it with the
belt-surface plane — no physics engine, matching this project's zero physics
dependency elsewhere — and is directly unit-testable; `_update_hover_tooltip`
is the thin, untested half that reads the live mouse position each frame.
New `godot_project/test_hover.gd`, wired into `scripts/verify-core-loop.sh`.
Also gave `test_screenshot.gd` a `--hover` flag (warps the cursor over a
known machine before rendering) so the tooltip could actually be reviewed as
an image, not just asserted headlessly — confirmed visually as well as via
the new test. Merged last of the three tickets, on top of the icon and
clipboard panel work below — reconciling meant deleting the old
`_machine_label`/`_economy_label` HUD code entirely (both superseded, by the
hover tooltip and the clipboard panel respectively) and factoring the hover
tooltip's construction into its own `_build_hover_tooltip()` alongside
`_build_contract_panel()`.

Worth flagging for whoever next touches the headless Godot test scripts: all
of them (not just the new one) call `main._ready()` manually right after
`root.add_child(main)`, but in a `--script`-mode SceneTree that add_child
*also* schedules the engine's own automatic `_ready()` call, deferred to the
first processed frame — so `_ready()` actually runs twice, silently building
two of everything under `main`. Harmless everywhere it's currently used
(every rebuild is deterministic and only `main`'s own fields get read
afterward), but it broke a naively-cached `Camera3D` reference while writing
this ticket's test — see test_hover.gd's own header comment for the full
story. Not fixed here; a pre-existing harness quirk shared by every existing
test in this family.

### 2026-09-07 — UI polish epic dispatched: hover info + diegetic clipboard UI

Started closing the gap UI_UX.md §5 flags and the DEVLOG has been tracking
as not-built: hover tooltips instead of dense always-on text, real icons
rendered from the voxel models instead of hand-drawn art, and the clipboard/
checklist motif GAMEPLAY.md §6 calls out contracts as the natural fit for.
Broken into 3 independently-dispatchable tickets (a new Run,
`run_cc6860507d36`) run in parallel: hover tooltips for machines, a voxel-
icon rendering pipeline, and a clipboard-styled reskin of the contract
panel. All three touch `main.gd`'s HUD-building code to some degree, so a
three-way merge conflict is expected and will be reconciled by hand, the
same way milestone 5 and the visual-polish pass were.

### 2026-09-07 — Voxel-rendered UI icons (UI_UX.md §5, one piece of the diegetic-UI epic)

Ticket B of the three-ticket parallel push above. Added a reusable way to
get a small icon texture for any item, generated from the real voxel models
rather than hand-drawn — `_icon_for(item_id)` in `main.gd`. Chose to extend
the existing offline pipeline (§2a's `vengi-thumbnailer` step) rather than a
live in-engine `SubViewport` renderer, but it turned out the existing
review-preview PNGs couldn't just be reused smaller — see the new UI_UX.md
§2b point 5 for why (short version: they're opaque and padded for "look at
the whole model," an icon needs a transparent, tightly-cropped render, and
`vengi-thumbnailer` has no flag for that, so a new `voxicons` binary
post-processes its output instead). New committed build output
(`assets/voxels/icons/`, staged into Godot by `cargo xtask` same as
`models/`) plus a headless test (`test_icons.gd`) covering the cached-lookup
and unknown-id-fallback behavior. Merged on top of ticket C below, so the
icon actually landed inside the new `ContractPanel`'s checklist row rather
than the old plain economy Label it was originally built against — the
hover-tooltip ticket is still separate, not touched here.

### 2026-09-07 — Contract readout reskinned as a diegetic clipboard (UI_UX.md §5)

First real piece of §5's UI style, dispatched as one of three parallel
tickets against the same commit (hover tooltips and a voxel-icon pipeline
being the other two — see above for how the icon merge reconciled against
this). The money/contract readout — previously one plain white Label folded
into the generic dark HUD panel — is now its own `PanelContainer`
("ContractPanel"): a warm paper-cream background framed in steel grey, with
a thick top border standing in for the clipboard's metal clip. The
contract's mix requirement (read live from `economy_snapshot_json`, never
hardcoded) renders as one small checkbox square per required unit, filling
in with a saturated "done" green and a tick mark as `contract_progress`
reaches each unit — the numeric "0/2" text stays alongside it, not replaced.
`godot_project/test_contract_panel.gd` (wired into
`scripts/verify-core-loop.sh`) checks this headlessly: the panel is a
genuinely separate node from the machine-line panel, one checkbox per
required unit exists, and a checkbox's style/tick-mark visibility actually
flips when driven through `_set_checkbox_checked`/`_update_contract_panel`.
Confirmed visually too, both empty and fully checked, via
`scripts/screenshot-scene.sh`.

### 2026-09-07 — Fixed the "empty void" lighting, remapped first-person up/down

Two follow-ups after actually using first-person mode. First, up/down is now
SPACE (up) and SHIFT (down) instead of E/Q, per request — SPACE only spawns
a tomato in isometric mode now, so there's no conflict with flying up.
Second, and bigger: the sky was configured but never actually shown — the
visible backdrop was a flat near-black colour, chosen specifically to avoid
a hard horizon line behind the tightly-framed isometric camera. That reads
as an empty void the moment first-person mode lets you look around freely,
which is exactly what got reported. Fixed by making the sky the real
background (`BG_SKY`), brightening it and the ambient light for a friendlier
daytime feel while staying grounded-industrial (UI_UX.md §3/§7 — no visible
sun disk added, to avoid the glow-streak problem that motivated the original
choice), and separating the floor's *visual* extent from the isometric
camera's auto-framing math, so the ground can stretch far past the chain
without changing the default isometric zoom. `test_presentation.gd` now
asserts the sky is actually the background, and `test_screenshot.gd` gained
a `--first-person` flag so this mode's lighting can be reviewed as an image
at all, not just eyeballed live.

### 2026-09-07 — Two view modes: isometric and first-person, toggled with TAB

Added a second way to look at the factory: TAB switches between the
existing UI_UX.md §1 isometric camera (which also gained right-click-drag
panning alongside WASD/scroll) and a new first-person "fly" camera — mouse
look, WASD to move, E/Q for up/down (not Space/Shift, since Space already
spawns a tomato and reusing it would fire both on one keypress). Both modes
share the one `Camera3D` node, reconfigured rather than swapped, so "exactly
one camera" stays true either way.

First-person is explicitly not a UI_UX.md concept and not real player
movement — there's no player entity, no gravity, no collision on the voxel
meshes, so it's a fly-cam, not a walk-cam. Movement is yaw-only horizontal
(pressing forward never climbs or dives just because you're looking up or
down), the same rule Minecraft's own creative-flight controls use, verified
headlessly by pitching the camera hard over and confirming forward movement
still has zero vertical component. Also fixed along the way: macOS trackpad
scroll wasn't zooming at all before this — it arrives as a different Godot
event type (`InputEventPanGesture`) than an actual mouse wheel
(`InputEventMouseButton`), which only the latter was handled. +/- keys now
zoom too. `test_camera_nav.gd` covers all of it: mode switching, look/move
math, drag-pan direction, and the trackpad/key zoom paths.

### 2026-09-07 — Camera pan and zoom

Added the navigation UI_UX.md §1 always allowed (it only forbids free
rotation of the fixed isometric angle): scroll wheel zooms, WASD/arrows pan,
clamped so you can't zoom past the point of losing the factory or so far out
it's a speck. The fixed angle itself never changes — pan moves where the
camera looks *at*, zoom changes the orthographic view size, orientation is
recomputed the same way either time. Verified headlessly
(`test_camera_nav.gd`, wired into `scripts/verify-core-loop.sh`): zoom
clamps at both ends, panning moves along the screen-relative axes (not raw
world X/Z), and orientation is provably untouched by either.

### 2026-09-07 — Visual polish: the voxel pipeline, and a factory you can look at

Built docs/UI_UX.md's asset pipeline for real and replaced the flat 2D
placeholder scene with the isometric 3D one that doc describes. Nothing in
the simulation changed — `sim_core`, `SimBridge` and `data/` are untouched,
and Godot still learns the chain's entire shape from `chain_layout_json` plus
chunk snapshots. This was purely "render what the sim already reports, far
better."

The pipeline is the substantial half. Voxel models are now generated by small
Rust functions (`assets/voxels/generators/`) against one shared, checked-in
palette, exported as MagicaVoxel `.vox`, and reviewed by rendering isometric
PNG thumbnails headlessly — no GUI voxel editor anywhere in the loop, which
was the whole constraint driving UI_UX.md §2a's design. Two of that section's
open questions closed in the process, and neither the way it expected: no
current Rust crate can write the `.vox` files needed (so the encoder is
hand-written, the fallback the doc itself anticipated), and the "which of two
Godot importers" question turned out to have one answer because the two
candidates are now the same repository.

The scene itself: concrete factory floor, rubber-and-steel conveyors, a
stainless processing vessel per stage, and voxel models for every product
from raw tomato to boxed ketchup — lit in real time with shadows, ambient
occlusion and a glow pass, under a fixed orthographic isometric camera. The
one rule that shaped the pipeline more than any other is UI_UX.md §4's
insistence that metals look deliberately shinier than everything else; that
is why an asset is emitted as one mesh *per material class* rather than one
mesh, and why the palette file owns material as well as colour.

Verification grew a piece it was missing: a headless test asserting the
structural claims of the look (camera really is orthographic, lights really
cast shadows, meshes really imported, metal really is shinier than matte),
plus a deliberately non-headless script that renders the actual scene to a
PNG — because no amount of headless testing can tell you whether a game looks
right. See CLAUDE.md's decisions log for the specifics worth not
re-litigating.

### 2026-09-06 — Milestone 5: Economy MVP

The loop stops being a production-chain tech demo and starts being a game:
`sim_core::sim::World` now owns a `money` counter (fixed-point, never a
float) and a single hardcoded contract — a fixed sauce mix (2x
`boxed_ketchup`), a fixed delivery cadence (every 1,000 ticks), and a fixed
payout — that the milestone-4 chain can actually deliver against. Collecting
finished product (`CollectOutput`) now deposits it into an accumulating
delivery buffer instead of discarding it; on each cadence boundary the
buffer is checked against the contract's mix, and a satisfied mix pays out
and consumes the delivered stock, while a miss just skips that window
(no contract-loss penalty yet — GAMEPLAY.md §11's still-open question).
Money and contract status are exposed to Godot as a JSON snapshot and
rendered as a minimal HUD label (money + per-item progress + ticks to next
check) — deliberately plain, not UI_UX.md's clipboard/checklist styling,
which is later polish. Verified headlessly two ways: a `cargo test`
integration test drives the whole chain end-to-end (spawn -> travel every
stage -> collect -> get paid) and asserts the exact payout lands only once
the mix is satisfied, plus `Economy`'s own unit tests cover cadence-boundary
edge cases; and the existing gdext-boundary Godot test now also round-trips
the new JSON snapshot. This resolves the money/contract architectural-home
question flagged as open since the cross-doc consistency pass (see
CLAUDE.md's decisions log and GAMEPLAY.md §9 for the actual shape used).
Explicitly deferred, per this milestone's own scope: `contracts.ron` data
loading, multiple concurrent contracts, and `StaffPools` — all still future
work.

### 2026-09-06 — Milestone 4: First full chain

Extended milestone 3's single belt+machine into a real multi-stage
production chain: raw tomato -> tomato sauce -> ketchup -> bottled ketchup ->
boxed ketchup, four sequential belt+machine stages, fully data-driven from
`data/recipes.ron`. The core design call: `sim_core::sim::World` now builds
one stage per recipe *in the data file's own list order*, chaining each
stage's machine output into the next stage's belt automatically — no
Rust code names which recipe follows which. This was proven, not just
claimed: after building and testing a working 3-stage chain, a 4th recipe
(`boxed_ketchup`) was appended to `data/recipes.ron` alone (verified via a
SHA-256 hash check that no `.rs`/`.gd` file changed), and the full headless
test suite — the Rust conservation test plus the real `gdext` JSON boundary
through Godot — passed against the extended 4-stage chain with zero code
changes. `sim_core/tests/core_loop.rs` now walks the whole chain generically
(via a recursive "how many raw tomatoes is this item worth" helper) rather
than asserting fixed counts, so it keeps working as the chain grows. The
Godot side (`main.gd`) reads a new `chain_layout_json` call once at startup
to build its `TileMap` sized to however many stages currently exist, instead
of hardcoding stage geometry — keeping intact milestone 3's "Godot draws
what the sim says exists" principle. See CLAUDE.md's decisions log for the
`ChunkSnapshot` shape change (`belt`/`machine` singular -> `belts`/`machines`
plural, since a chunk can now hold more than one stage) and the deliberate
scope limit (every recipe past the first has exactly one input/output, so
this is one straight line, not yet a branching factory).

### 2026-09-06 — Milestone 2: Mobile export spike

Deliberately run early (ARCHITECTURE.md §9) so mobile-export risk surfaces
now rather than at platform polish. Full details in ARCHITECTURE.md §9a;
headline result: **Android works, iOS doesn't in this environment.**

Android: stood up a standalone Android SDK/NDK (no Android Studio needed),
cross-compiled the milestone-1 `sim_core` crate for `arm64-v8a`/`x86_64` via
`cargo-ndk` with zero source changes, and got Godot's Gradle-based export to
produce a validly-signed APK with the extension actually embedded (confirmed
via `apksigner verify` and `unzip -l`). One real Godot gap found along the
way: its Android export CLI can fail with a **blank** error message if
`rendering/textures/vram_compression/import_etc2_astc` isn't set in
`project.godot` — tracked down by reading Godot's own export-plugin source
rather than guessing, now fixed in this project's settings. Installing and
actually launching the APK on a device/emulator was skipped this round — the
dev host was down to double-digit MB of free memory with sibling sessions
running concurrently, and booting an emulator on top of that felt like the
wrong risk to take. That's the one remaining unverified step before Android
export counts as fully proven, not just built.

iOS: could not be attempted for real. This dev environment has no Xcode.app,
only Command Line Tools — confirmed both via `xcodebuild`/`xcrun` failing to
find an iOS SDK at all, and via a Rust `aarch64-apple-ios` build that
compiled without error but linked against the wrong (macOS) SDK sysroot, so
even that partial result wasn't treated as a pass. Per this milestone's own
instruction to decide explicitly if iOS looks broken: **it stays a
stretch/deferred platform, not a milestone-10 launch commitment**, until
someone re-runs this spike on a Mac with real Xcode or Apple-hosted CI —
that's recorded as a genuinely open question (ARCHITECTURE.md §11), not a
settled "iOS is broken."

Also surfaced, unrelated to mobile export itself: late in this session the
shared dev host (many concurrent users, physical memory nearly exhausted)
started failing to load the project's own unmodified `sim_core` extension at
all — even the plain milestone-1 round-trip check — with a Rust
undefined-behavior-check panic inside `gdext`. Checksum-verified as the exact
same binary that worked fine earlier in the session, and reproduced even in
a clean environment, so this reads as host resource exhaustion tripping an
optional safety check, not a project regression — but it means
`scripts/verify-scaffold.sh` should be re-run on a calmer host before trusting
its result if it ever fails unexpectedly like this again.

### 2026-09-06 — Milestone 3: Core loop proof

First real simulation code: one belt, one machine, one recipe (tomato →
tomato sauce), rendered and interactable in Godot. `sim_core/` now has a
pure-Rust `sim/` module tree (belt/machine/recipe/inventory/world, no `gdext`
dependency) sitting under a thin JSON-based `SimBridge` translation layer —
`tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
`query_tile()`, per ARCHITECTURE.md §4. The belt implements real gap-based
item storage (§3.3, Factorio's FFF #176 trick), the machine runs the generic
tick-counted `TimedProcess` pattern (§3.5), and all shared simulation state
uses fixed-point (`fixed::I16F16`), never float, per §3.9. The recipe itself
lives in a new top-level `data/recipes.ron` — resolving §11's data-file-path
open question — not hardcoded in Rust.

Verified in three layers, cheapest first (`scripts/verify-core-loop.sh`
automates all three): a headless `cargo test` integration test ticks the
belt+machine for thousands of ticks and asserts strict item conservation
(no duplication/loss) every single tick; a headless Godot script drives the
real `gdext` JSON command/snapshot boundary end to end (spawn 4 tomatoes →
1 tomato sauce → collect); and a second headless Godot script simulates an
actual keypress through the interactive scene's input handler to confirm
the "interactable" requirement is real, not assumed. The interactive scene
(`main.tscn`) renders via a genuine `TileMap` + moving item sprites, driven
only by `SimBridge`'s JSON snapshots — it never touches the belt's internal
gap array. Visuals are flat placeholder textures, explicitly not a UI_UX.md
style decision (see `godot_project/placeholder_art/README.md`); the real
voxel asset pipeline is still unbuilt. See CLAUDE.md's decisions log for two
smaller implementation calls worth knowing about: the belt's gap
representation and why the `gdext` boundary is JSON strings instead of
`Dictionary`/`Array` values.

### 2026-09-06 — Milestone 1: Scaffold

First real code. Set up the Rust workspace (`sim_core/` cdylib crate + an
`xtask` build-script crate) and a Godot 4 project (`godot_project/`), wired
together via `gdext` — a trivial `SimBridge.ping()` callable from GDScript
proves the Rust↔Godot round trip end to end. `cargo xtask` builds and copies
the compiled extension into place per ARCHITECTURE.md §7; `sim_core.gdextension`
is checked in. Pinned the exact versions ARCHITECTURE.md §1a had left as
placeholders (Godot 4.6.3-stable, `gdext` 0.5.5, Rust edition 2024, `hecs`
0.11.1) — see CLAUDE.md's decisions log for the reasoning.

Verified entirely headlessly, no editor GUI: `scripts/verify-scaffold.sh`
builds, primes Godot's local import cache (working around a known, benign
Godot 4.6.3 editor-shutdown crash — see the script's own comments), then runs
`godot --headless --script test_round_trip.gd`, which calls `SimBridge.ping()`
and asserts the known return value, exiting non-zero on failure. This is now
the project's headless "does the interop boundary still work" check.

### 2026-09-06 — Milestones 1-5 dispatched via Orca orchestration

Adopted Orca orchestration (see CLAUDE.md's Orchestration section) as how
milestone work actually gets built, then used it for real: created a Run and
a 5-task DAG covering milestones 1-5 (Scaffold, Mobile export spike, Core
loop proof, First full chain, Economy MVP), dependency-chained to match
ARCHITECTURE.md §9. Milestone 1 (above) was the first task dispatched and is
now complete; 2-5 are queued behind their dependencies and will be dispatched
as each prior milestone completes. This is the actual start of writing game
code — everything before this was design docs only.

### 2026-09-06 — Repo set up, full design-doc pass complete

Set up the repo (private, Conventional Commits enforced via hook + CI) and
wrote all three core design docs: **ARCHITECTURE.md** (Rust sim core +
Godot/`gdext` presentation, chosen to be agent-friendly, then hardened via a
critical-review pass that turned several open options into firm decisions),
**UI_UX.md** (isometric voxel art at a 32×32×32 tile scale, Minecraft-adjacent
UI, plus a researched asset pipeline so voxel models can be authored and
reviewed by an agent), and **GAMEPLAY.md** (tiered sauce/equipment
progression, a contracts-based economy designed to force a diversified
factory, staff as an abstract resource pool). Still no game code — the docs
are the whole project so far. Next up: milestone 1 (scaffold) from
ARCHITECTURE.md. For specifics, see the docs themselves, CLAUDE.md's
decisions log, or `git log`.

### 2026-09-06 — Cross-doc consistency pass

Read back through all three design docs together looking for gaps and
contradictions now that they've each been written somewhat independently.
Found and fixed a real one: the milestone list never actually scheduled the
economy/contracts system, even though the gameplay doc defines the core loop
as needing it — added an "Economy MVP" milestone. Also gave the architecture
doc an "Open questions" section (its siblings already had one) and flagged a
couple of things that don't have a designed home yet (money/contract state,
mobile touch input). Tightened up repo config (`.editorconfig`, a fuller
`.gitignore`) while at it. Nothing structural changed — still docs-only,
still pre-milestone-1.

