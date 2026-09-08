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

- **Stage:** Pre-alpha, but the core MVP loop is real end to end and it looks like a game. Milestones 1 (Scaffold), 2 (Mobile export spike), 3 (Core loop proof), 4 (First full chain), and 5 (Economy MVP) are all done — a real multi-stage production chain (tomato -> tomato paste -> ketchup mix -> ketchup -> bottled ketchup -> boxed ketchup) that can deliver against several concurrent, data-driven contracts to earn money, headlessly verified end to end, Android export proven working. On top of that, a visual-polish pass built UI_UX.md's voxel asset pipeline and turned the flat 2D placeholder scene into an isometric, real-time-lit 3D factory.
- **Repo:** Private GitHub repo (`TomWhitticase/sauce-factory`), `main` branch. Conventional Commits enforced via a local `commit-msg` hook + CI backstop (see CONTRIBUTING.md).
- **Design docs:** ARCHITECTURE.md, UI_UX.md, and GAMEPLAY.md are all written and cross-referenced; docs/README.md indexes them.
- **Milestone progress:** milestones 1-5 (Scaffold through Economy MVP) were built as an Orca orchestration Run (`run_51e5cb271445`), one Task per milestone, dependency-chained. **All 5 are complete and merged.** Check `orca orchestration task-list --run run_51e5cb271445 --json` for the DAG's final state. There are 11 milestones total, not 9 — a milestone 5 "Economy MVP" was added since the loop wasn't a game without it. **Milestone 6 (Fluids) is done**: its simulation half (pipe network, pressure equalization, a recipe requiring a liquid, the `SimBridge` fluid snapshot) and its presentation half (pipe/supply-tap/liquid voxel assets, a pipe run rendered from the simulation's own segment list, a liquid body whose height tracks live pressure, and pipe hover info in both view modes) have both landed, built as a 2-ticket orchestration Run (`run_2867a25ae373`, sequential: sim core then presentation). Milestone 7 (Timed processes) is next. A separate **"Machines improvement"** epic (not an ARCHITECTURE.md §9 milestone — an infrastructure/content pass requested directly) is in progress: ticket 1 landed machine footprints as data (`data/machines.ron`), generalizing the chain's layout math so a machine can occupy more than one tile, and ticket 2 spent that on real content — the chain's front end is now a crusher, a mixer and a genuinely 2x2 cooking vat. Ticket 3 replaced the debug "spawn a tomato onto the belt" command with a real **hopper** — a building west of the chain's head that buffers raw tomatoes and meters them onto the first belt one per tick, filled by the player (SPACE) rather than teleporting items. **Ticket 4 finished the epic**: the Godot presentation of all three — a bespoke voxel model per front-end machine kind, multi-tile rendering so the cooking vat actually covers its 2x2 footprint, the hopper drawn with a product level that tracks its live fill, and hover resolving any tile of a multi-tile machine as well as the hopper. **The "Machines improvement" epic is complete.** A further **"Shop & Inventory"** epic (also requested directly, and a deliberate priority shift toward a real build-your-own-factory game) has started: **ticket 1** landed the economy half — `data/shop.ron` prices every buildable kind, `sim::shop` loads/validates it, the player owns a per-kind count, and a `Purchase` command spends contract money on it, all exposed as `shop_snapshot_json`. **Ticket 2 finished the epic**: a real build menu (**B**), a grid of voxel-rendered icon slots built entirely from `shop_snapshot_json`, where clicking a slot — or pressing its number key — buys one, and what you can't afford is disabled, dimmed and marked. **The "Shop & Inventory" epic is complete.** Nothing consumes an owned count yet: the shipped chain is still built unconditionally, so buying is a purchase ledger, and a following epic (real placement/rotation/pickup) is what makes a purchase visible on the floor.
- **Art/UI:** the voxel pipeline (UI_UX.md §2a/§2b) exists and is the source of every asset in the scene — including milestone 6's pipes and the liquid inside them, which added a fourth material class (`liquid`, a glossy dielectric) to `palette.ron` — Rust generators in `assets/voxels/generators/` produce `.vox` models, reviewed as isometric PNGs via `scripts/build-voxels.sh`, imported into Godot by a pinned community importer. UI_UX.md §5's real UI is now landed: the money/contract readout is a diegetic clipboard panel (paper background, steel clip bar, checkbox tick marks) with a voxel-rendered icon per required item, and per-machine detail (recipe/progress/buffers) is hover-only instead of dense always-on text, working in both view modes — a screen-centre crosshair stands in for the cursor in first-person — see the log entries below.
- **UI polish epic (done):** a Run (`run_cc6860507d36`) with 3 parallel tickets targeting UI_UX.md §5, all done and merged — hover tooltips for machines, a voxel-rendered icon pipeline, and the clipboard reskin. Check `orca orchestration task-list --run run_cc6860507d36 --json` for the DAG's final state.
- **Visual overhaul round 2 (done):** a second directly-requested pass — the first visual-overhaul attempt (below) closed a floor-level seam but left the actual complaint unfixed ("items disappear into the wall"), so this round diagnosed and fixed it properly, as a 3-ticket epic. **Ticket 1 (sim):** the pipe run's always-refilling off-map source is gone, replaced by a player-loadable `FluidTank` (one tile west of the run's head, loaded with the new **V** key) drained by the same pressure-equalization pass that moves fluid along the run — vinegar is now a consumable the player manages, exactly as tomatoes already were via the hopper. **Ticket 2 (assets):** every machine kind in `data/machines.ron` now has a bespoke model (the bottler and packer were still generic boxes), the tank is drawn with a live level gauge, the pipe run and the crusher were redesigned after diagnosing exactly what read wrong in a rendered close-up, and the three bulk stages travel the belt as visibly *liquid* product (a glossy surface in an open pail, not a solid block). **Ticket 3 (the actual connection fix):** every machine now has real, carved inlet and outlet openings sized and placed from the simulation's own layout — the key finding was that the fixed isometric camera can only ever see one side of a tile, so an outlet is a real doorway carved through the camera-facing wall while an inlet is a hooded housing cut open on that same visible side (cutting an opening on the actual inlet face, as the first attempt would have, is invisible to this camera and would have fixed nothing). The belt gained two junction-adjacent variants picked per tile from the sim's own layout, the pipe run's downstream end is properly terminated instead of stopping in mid-air, and a new `--closeup` flag on `scripts/screenshot-scene.sh` let every one of the twelve junctions be inspected individually. Corner handling for belts/pipes stays deliberately undone (documented in `sim_core/src/sim/world.rs`'s module doc) until the sim carries a per-tile direction field.
- **Visual overhaul, first attempt (superseded by round 2 above):** closed a floor-level seam between machine/hopper footings and belt/pipe edges, added a connector fitting for the fluid-fed stage, gave the hopper's status lamp real feedback, and fixed some contract-clipboard typography — real fixes, but not the one the player actually flagged, which is what round 2 above addressed.
- **Camera:** two view modes, toggled with TAB. Isometric (default): pan (WASD/right-drag/arrows) and zoom (scroll, trackpad, +/-); the fixed angle itself never changes, per UI_UX.md §1. First-person: a free-look "fly" camera (mouse-look, WASD move, SPACE/SHIFT up/down) — not a UI_UX.md concept, and not real player physics (no gravity/collision, this project has neither). 90° snap-rotation of the isometric angle is still not implemented (still a nice-to-have, not required).
- **Lighting:** the sky is the real background (`BG_SKY`), not a flat colour — see the "empty void" DEVLOG entry below for why that matters. `scripts/screenshot-scene.sh --first-person` reviews that mode's lighting specifically.
- **Contracts (sim half done, presentation pending):** contracts are now `data/contracts.ron` content and three run concurrently, each with its own mix/cadence/payout, all drawing from one shared pool of delivered stock (a box pays one contract, never two). `economy_snapshot_json` now returns an array of contracts — a breaking change the Godot clipboard panel has not yet been updated for, which is the follow-up ticket.
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

### 2026-09-08 — Shop & Inventory, ticket 2: a build menu you can actually buy from

The presentation half of the shop, and the first piece of genuine player
agency in the game — everything before it was either automatic or a single
fixed action ("load the hopper"). **B** opens a build menu over the scene:
a grid of voxel-rendered icon slots, one per `data/shop.ron` entry, each
showing the kind's icon, its price and how many the player owns. Clicking a
slot buys one; so does pressing the slot's number key, which is how it works
in first-person where the mouse would otherwise be captured. Nothing about
the menu's contents is written down in GDScript — the catalog, its order,
prices, owned counts and affordability all arrive in ticket 1's
`shop_snapshot_json`, so pricing a new kind in `data/shop.ron` puts a slot in
the menu with no Godot edit, the same way adding a recipe already extends the
chain.

Two things worth knowing. What you cannot afford is signalled three ways at
once — the slot is disabled and visibly recessed, its icon dims, and its
price gains a "✗" — because UI_UX.md §7 forbids encoding state in colour
alone. And the menu wears the scene's existing dark plate with the
hazard-yellow rule rather than the contract clipboard's paper: the clipboard
motif is for orders and checklists, a build menu is machine-side factory UI,
and there is deliberately no third visual language here.

Verified headlessly (`test_shop_menu.gd`: the menu is a distinct node, one
slot per catalog entry with the snapshot's own numbers, B toggles it, every
slot reads locked at zero money, and a purchase driven through the real
`issue_command_json` — with money genuinely earned by running the chain
against the contract — moves the displayed owned count) and visually, via a
new `scripts/screenshot-scene.sh --shop` flag that earns a payout and opens
the menu so a shot shows both affordable and unaffordable slots. `sim_core`
was not touched.

**This completes the "Shop & Inventory" epic.** Buying is still a purchase
ledger and nothing more — the chain is built unconditionally at tick 0, and
actually placing what you bought is the next, separate epic.

### 2026-09-08 — Shop & Inventory, ticket 1: money can now buy something

The start of a directly-requested priority shift — turning this from a fixed,
always-built production line into a factory game where the player *buys* the
equipment and puts it down. That is two epics: this one (a real
purchasable-equipment economy) and, after it, real placement/rotation/pickup.
This ticket is the first half of the first epic, and it is Rust and data
only; the Godot build menu is ticket 2.

GAMEPLAY.md §1 has always said the loop is "money buys equipment, equipment
unlocks sauces, sauces satisfy contracts, contracts pay money," and until now
the first clause was the missing one — money accumulated and had nothing to
be spent on. There is now a `data/shop.ron` pricing every buildable kind
(every machine kind in `data/machines.ron`, plus the belt and pipe tiles,
which are infrastructure rather than machines), a `sim::shop` module that
loads and validates it the way the recipe and machine loaders already do, a
per-kind owned count, and a `Purchase` player command that debits the *same*
money counter a contract delivery credits — deliberately one currency, per
that section, not a shop with its own wallet. Prices are placeholders
(GAMEPLAY.md §10 puts real balance in a later pass) but they encode a shape:
a belt tile is small change, and the 2x2 cooking vat that runs the chain's
longest process is the thing you save up for.

**What this ticket deliberately does not do is change the factory.** Every
machine still exists unconditionally from tick 0, exactly as before; buying
something credits a number and nothing else. Keeping the economy layer and
the placement rewiring apart means this half is fully testable in isolation
and moved no existing tile — the whole pre-existing test suite passes
untouched. The one cross-file invariant worth knowing about: adding a machine
kind to `data/machines.ron` without pricing it in `data/shop.ron` is now a
test failure, not a thing the player can never buy.

### 2026-09-08 — Contracts are content, and there are three of them

Simulation and data only (Rust + `data/`); `godot_project/` deliberately
untouched, and this is ticket 1 of 2 — the presentation half (contract cards
for a board rather than a single clipboard) is its own ticket. Closes
GAMEPLAY.md §9's long-standing "contracts need their own data file"
(`contracts.ron`) and the multi-contract half of §6.

Milestone 5's Economy MVP shipped exactly **one** contract, hardcoded in Rust
from four named constants. That was honest for a milestone whose only job was
"prove build factory -> get paid," but it meant the game's central design
pressure — GAMEPLAY.md §6's whole argument for contracts over a spot market —
did not exist: with one contract there is nothing to diversify *between*.
Contracts are now content in `data/contracts.ron` (schema_version 1, loaded
and validated by a new `sim/contracts.rs` alongside `recipe.rs`/`machines.rs`),
three of them ship, and each runs its own mix, its own cadence and its own
independently-tracked next-delivery tick.

**The decision worth not re-litigating is how a delivered box is allocated,
which GAMEPLAY.md left open.** There is one shared pool of collected stock;
a contract that settles *consumes* its mix from it, and goods are never
credited to two contracts at once. The tempting alternative — credit every
contract that wants the item — duplicates goods, so stacking contracts onto
one product line would double income for free, which is the exact strategy
the contract system exists to prevent. Contracts therefore compete for one
line's throughput, and file order is the (documented, deterministic)
tiebreak until a player-set priority exists. The shipped three are balanced
just inside the chain's real throughput, so a factory kept fed serves all
three and one left to run dry starts missing windows.

`economy_snapshot_json` changed shape as a result — the four top-level
contract fields are gone, replaced by a `contracts` array plus a
`delivery_stock` pool; `money` is unchanged. That breaks the existing Godot
clipboard panel, which is exactly what ticket 2 is for.

### 2026-09-08 — Visual overhaul round 2, ticket 3: the belts and pipes now go somewhere

Presentation-only, and the last ticket of the epic. `sim_core` gained one
doc-comment clarification and nothing else; `data/` was untouched. It answers
the sharpest complaint from the project owner's live-demo review — *"a
conveyor belt going into a mixer seems to just go straight into the side
there is no connecting so it looks like items just disappear into the wall"*
— which the previous pass's flush-footing fix had not: that closed a
floor-level crease, and a flush wall is still a wall.

**The finding that decided everything came from rendering the junctions one
at a time, and it contradicts the obvious fix.** The camera is fixed at a
`(1, 1, 1)` view direction, so the only vertical faces it can ever see are
the ones facing +X and +Z. The chain runs +X — which means a machine's
*outlet* face is dead-on camera-facing, and its *inlet* face points directly
away and can never be seen at all. Carving an inlet doorway, the thing this
ticket was written to ask for, would have been invisible. So the two halves
of a junction are deliberately different pieces: an outlet is a real doorway
cut through the +X wall with a lip reaching the tile edge, and an inlet is a
hooded transfer housing whose *camera-facing side* is cut away, showing a
brass threshold at the lip and darkness beyond it. Both read by value
contrast rather than by detail, because at scene scale a junction is a dozen
pixels — a pale interior renders as an empty portal and a hazard stripe on
the one camera-facing edge renders as a dark bar, and both were built,
rendered and thrown away before the current version.

**The belt has two junction-adjacent variants now**, and which tile gets one
is derived from the layout the simulation already publishes: whether the
neighbour along that stage's own travel direction is covered by a machine
footprint, or by the hopper, which discharges the same way. That is the
buildable half of *"assets should look different depending on their
orientation and what's around them"* — what a tile is next to is on the wire;
which way it points still is not, so the variants are authored for a +X run
and the renderer falls back to the plain tile for anything else. The corner
limitation in `world.rs` is unchanged, and was re-stated there rather than
quietly weakened.

Three things turned up on the way. The pipe run's downstream end was an open
bore — the run literally stopped in mid-air — and its supply end was
*plugged*, because the flange drawn for the tank joint filled back in the
hole the tank discharges through. And the per-frame tick catch-up was
unbounded, so one slow frame silently fast-forwarded the whole factory:
two identical screenshot runs photographed visibly different states, which
makes a rendered scene useless as a before/after check. Clamped; two runs are
now byte-identical, which is the property this kind of review depends on.

Verified the way this project verifies visual work, plus one new layer. The
generators now have tests of their own (`cargo test -p voxgen`) asserting
that each machine's opening is a genuinely *empty* volume at the belt's own
lane and height — a mouth painted dark meshes as a flat rectangle and renders
as a decal, and no screenshot at scene scale can tell the difference. A new
headless `test_connections.gd` checks that the per-tile belt choice really
comes from the layout. Then the part that actually decided the design:
close-up renders of all twelve junctions in the factory, iterated through
five rounds of look-change-look.

### 2026-09-08 — Visual overhaul round 2, ticket 2: every machine has a face, and the sauce is liquid

Presentation-only. `sim_core` and `data/` were not touched at all — every fact
this ticket draws was already on the JSON wire, ticket 1 included. It answers
four specific complaints from the project owner's live-demo review, and the
method for all four was the one this project keeps re-learning: render the
whole factory, look at it, change something, look again.

**Two machines had no face.** `bottler` and `packer` had no entry in
`main.gd`'s model table, so the chain's last two stages both drew the generic
housing and the end of the line looked like the same machine twice. Both now
have models built around what actually identifies them — bottles standing
under a filling gantry with a capping head; a packing table with a stack of
case blanks, an open case and a taping head. The packer is deliberately low
and mostly *cardboard*, the one large warm matte surface in the factory, so
it reads apart from its stainless neighbours at a glance.

**The pipes and the crusher were redesigned, not tweaked.** Rendering them
alone was not enough to see what was wrong; rendering the factory was. Each
pipe tile carried four oversized rings, so a join stacked two of them and the
run read as a row of bobbins rather than one pipe; the sight opening removed
almost half the top wall, so the tube had no round silhouette left; and the
dark support legs out-massed the run they carried. It is now one flange per
tile, a short framed sight window, one slim centred stand — and the tube is
the *brightest* metal in the palette with the fittings darker, which is
backwards from convention and is the thing that finally let the eye follow
the run from the tank to the mixer. The crusher's flared lip was, at scene
scale, a flat plate laid over the machine. Replacing it with the obviously
correct shape — a converging feed mouth — made it worse, because a shallow
taper seen from a fixed oblique angle *is* a plate; that failed attempt is
recorded in the generator so the next session doesn't reach for it. The
version that works identifies the machine from the side instead: a squared
frame with an open feed pocket you can see product sitting in, and big driven
flywheels on the camera-facing face.

**The tank ticket 1 built is now drawn**, with a level gauge slot down its
front and a liquid column behind it scaled to the live fill — the same
one-poll/one-scaled-body pattern the pipe run's liquid and the hopper's
product level already use, deliberately reused a third time rather than
reinvented. It also gained the thing ticket 1 didn't: a **key to fill it**.
The shipped scene had no way to load the tank at all, so the chain silently
stalled at the mixer with every belt past it running empty; **V** is now the
fluid twin of SPACE, and the screenshot harness loads both consumables.

**And the sauce is liquid.** Tomato paste, ketchup mix and finished ketchup
travelled the belt as pails capped by a flat matte disc, which read as a
painted lid. The pails are hollow now, with the product surface a voxel below
the rim, and that surface takes `palette.ron`'s glossy `liquid` class — the
material milestone 6 added for exactly this. Three new palette entries carry
their own hue rather than being tinted in GDScript, because unlike a pipe's
contents these are fixed by the recipe chain.

One rename while in there: `item_sauce_pail` became `item_paste_pail`. The old
name still referred to `tomato_sauce`, an item id deleted two epics ago — the
exact drift that once shipped cardboard boxes down the belt.

Verified headlessly by the full `scripts/verify-core-loop.sh` suite (`cargo
test -p sim_core` unchanged and green, plus nine Godot scripts), with
`test_machines.gd` extended to fail if *any* stage falls through to the
generic model and `test_fluids.gd` extended to cover the tank's mesh,
placement, live level and hover. Then reviewed the way it has to be: repeated
rendered screenshots of the running factory, zoomed in, four iterations deep.

### 2026-09-07 — Visual overhaul round 2, ticket 1: the vinegar comes from a tank the player fills

Simulation-only (Rust + `data/machines.ron`), the first of three tickets in a
second visual-overhaul epic; tickets 2 (new/redone voxel assets, including a
tank model) and 3 (belt/pipe-to-machine connections) both build on it.

Milestone 6's fluid supply was an off-map tap: one pipe segment flagged
`is_source` that refilled itself to capacity every tick and could never run
out. That was the smallest honest thing at the time, but the project owner's
review of the live demo named the gap exactly — *"add a fluid equivalent of a
hopper for the vinegar"* — because there was nothing there for a player to
manage. There is now a **`FluidTank`**: a capped, player-loadable buffer
standing one tile west of the pipe run's head, filled with a `FillFluidTank`
command that mirrors `FillHopper` down to its clamping and its optional
`amount`, and drained by the *same* per-tick pressure-equalization pass that
already moved fluid along the run. Vinegar is a consumable now — it runs out,
the mixer stalls, and refilling starts the line again.

Two decisions worth not re-litigating. **The tank is not a pipe segment.** It
is a placed building with a footprint in `data/machines.ron`, like the hopper,
kept out of `FluidNetwork::segments` so the run's existing snapshot geometry
did not change shape at all; what it shares with a segment is the flow rule,
not the type. **It presents constant full head, not its own fill fraction.**
It stands beside the run as a raised vessel, so it discharges downhill: a
tank that equalized on fill fraction would keep three quarters of every
remaining unit inside itself forever, and the player would watch the mixer
starve while the gauge still read a quarter full. Full head also makes flow
strictly one-way with no special case — the shared rule already clamps a
transfer to what the donor has, so an empty tank simply moves nothing.

Placement followed the hopper's precedent exactly (one tile outside the
existing geometry, so introducing it moved no tile); capacity is 120 units,
sized against the line the way the hopper's 64 was, which is about three and
a half hoppers of tomatoes per fill. The tank is on the wire as
`fluid_tank_snapshot_json`, a `fluid_tanks` array in the chunk snapshot, and a
`"fluid_tank"` hover tile — all shaped as the fluid twin of the hopper's, so
tickets 2/3 can drive both from one code path.

### 2026-09-07 — Visual overhaul: closing the belt/pipe/machine seams, hopper feedback, clipboard typography

A dedicated presentation-only pass requested directly by the project owner
("things don't connect properly to adjacent tiles... we also need to improve
our UI elements"), on top of everything below. `sim_core`'s only change is a
doc comment (see the corner note below); no recipe, machine, or economy
behaviour moved.

**Root cause of the connection issue, found by actually rendering and
zooming into the scene rather than guessing from code:** `belt()` and
`pipe_body()` (`assets/voxels/generators/assets.rs`) have always run their
frames/bores flush to all four edges of their own tile, but every machine's
footing (`machine()`, the shared `machine_footing()` crusher/mixer/vat build
on, and the hopper's own plinth) was inset 1-2 voxels from its tile's edges.
That left a small but real, and completely consistent, seam of bare floor —
darkened further by SSAO into a visible crease — at every single belt-to-
machine and pipe-to-machine junction in the factory. Fixed by making every
one of those footings flush with its tile boundary, the same rule belts and
pipes already followed.

**The pipe run itself had a second, more specific gap**: a plain `pipe`
tile's solid header sits mid-tile, so the run's last segment (the one
`FluidNetwork::inlet_index` actually feeds a machine from) left real open
floor between its own bore and the mixer's inlet nozzle — and the two were
modelled 4 voxels apart in height on top of that. New `pipe_inlet` asset (a
branch fitting reaching to the machine-facing tile edge at the mixer's own
inlet height) closes it; a new `test_fluids.gd` assertion checks the inlet
segment actually renders as this distinct fitting, not a recoloured plain
pipe.

**Corners:** checked, not built. Every belt and pipe tile only ever runs
+X — there's no `direction` field in any snapshot for a renderer to even
read — so drawing a turn needs a sim-side topology change first, not just a
rotated asset. Documented in `world.rs`'s module doc rather than
speculatively built against a corner nothing places yet.

**UI polish**, scoped to real, specific gaps rather than a broad reskin: the
hopper's one status lamp used to sit a fixed colour regardless of state —
now it swaps between the same idle/running materials a machine's lamp uses
based on whether the hopper actually holds stock, so filling it gives
feedback beyond the level slowly rising. The contract clipboard's title and
money figure shared the body text's exact size, so nothing on the panel told
the eye what mattered most; both now step up in a small, consistent type
scale.

Verified the established way: `cargo test -p sim_core` and the full
`scripts/verify-core-loop.sh` suite both green (including the new fluids
assertion), and `scripts/screenshot-scene.sh` rendered and reviewed multiple
times — populated isometric, hover-pipe, hover-hopper, and first-person —
comparing tight crops of the same regions before and after the footing fix
to confirm it actually changed the generated geometry (voxel counts moved,
e.g. `machine_mixer` 11675 -> 12877) even where the visual delta reads as
subtle at screenshot resolution rather than dramatic.

### 2026-09-07 — Machines improvement, ticket 4: you can see all of it now

The presentation half, and the last ticket of the epic — tickets 1-3 built
machine footprints, three real front-end machines and a hopper, and none of
it had ever been drawn. `sim_core` is untouched: everything here comes out of
JSON those tickets already exposed.

Four new voxel assets, each with a silhouette that says what the machine
does: a **crusher** with two counter-rotating rolls visible in an open feed
throat, a **mixer** with a paddle agitator in an open drum (its drive motor
moved to a side bracket precisely so it doesn't hide the agitator), the 2x2
**cooking vat** with an access platform, ladder, steam stack and a sight
glass — the first asset in the pipeline bigger than one tile — and a
**hopper** that is a real funnel: a box section over a converging cone,
narrowing to a chute that steps down over the head of the first belt. The
material rule they follow is "food contact is stainless, structure is painted
or poured": the parts product touches are `metal`, everything holding them up
is `matte`, which is what gives UI_UX.md §4's shiny exception something to be
an exception *to*. All four went through the pipeline's own review loop and
all four were reworked after looking at the render rather than after writing
the generator — the crusher's rolls were originally sunk in a well too deep
for the isometric angle to see into, and the vat read as a sphere.

Rendering a machine now reads its **kind** and **footprint** from the
simulation, so the vat genuinely covers its four tiles (a node sits at its
footprint's centre, which for every 1x1 kind is still exactly its tile), and
a kind with no bespoke model still draws with the generic one — adding to
`data/machines.ron` never needs a Godot edit first. The hopper's product
level is a body scaled to its live fill every frame, deliberately the same
mechanism milestone 6 built for liquid in a pipe rather than a second visual
language, and SPACE's help text now says what it actually does. The one
hover mechanism grew a third branch: any tile of a multi-tile machine
resolves to it, not just the origin, and so does the hopper.

The rendered-scene review caught something no headless test could: the
item-model mapping still named `tomato_sauce`, an item ticket 2 deleted, so
two whole stages of the chain had been shipping cardboard boxes down the belt
instead of pails of paste and mix. Fixed, with a third pail livery for the
mix so the vat's long cook visibly changes something. That is the argument
for `scripts/screenshot-scene.sh` existing, made concrete.

### 2026-09-07 — Machines improvement, ticket 3: a hopper feeds the chain

The chain's raw ingredient no longer appears out of nothing. Since milestone
3, pressing SPACE ran a command that reached straight into the first belt and
teleported a tomato onto it — a fine "prove the belt renders" scaffold and a
bad mechanic, because nothing in the world held the ingredient and the
player's key-press *was* the production rate. There is now a **hopper**: a
real building, standing immediately west of the chain's first belt, that
holds a bounded buffer of tomatoes and meters them onto that belt one unit
per tick, and only when the belt has room. Filling it is the player action;
the rate is the machinery's.

Three calls worth not re-litigating. (1) **The hopper is its own stage type,
not a recipe-driven one.** `data/recipes.ron`'s list is the chain of
*processes*, and a hopper doesn't consume-and-produce — forcing it through
`Recipe` would have meant inventing an entry that creates matter from no
inputs, which would lie in the data file and break the mass-conservation
reading the tests depend on. So `recipes.ron` is untouched by this ticket;
`data/machines.ron` gains a `hopper` kind purely for its footprint, since a
footprint is a footprint whether or not a recipe names the kind. (2) **It is
placed at negative x**, west of the chain's start rather than at it, so
introducing a whole new building moved not one existing tile — the same
discipline ticket 1 followed. The cost is that the world now spans two
chunks (the hopper is in `(-1, 0)`, everything else in `(0, 0)`), which
`all_chunks` reports and a renderer must honour. (3) **Capacity 64, one
unit per tick**, both Rust constants rather than data: 64 tomatoes is 16
crusher crafts, enough that filling buys real breathing room and little
enough that keeping the line fed stays a decision. In practice the belt's
own spacing rule is the binding constraint, so a full hopper feeds at exactly
the rate the belt accepts and stops dead when the line backs up.

`PlayerCommand::SpawnItem` became `FillHopper` — a rename, not a third
command, since keeping both would leave two contradictory ways to feed the
chain. New `hopper_snapshot_json` carries placement plus live fill, and
hoppers also appear in `chunk_snapshot_json` and `query_tile_json`;
`chain_layout_json` is deliberately unchanged, because it is defined as one
object per recipe stage and the hopper is not one. Rendering the hopper and
its fill UI is ticket 4.

### 2026-09-07 — Machines improvement, ticket 2: crusher, mixer, cooking vat

Content, riding on ticket 1's mechanism. The chain used to open with two
abstract steps — a "cooker" that turned raw tomatoes straight into tomato
sauce, then a "reducer" that turned that into ketchup — which is not a
process anyone would recognise from a real plant. It now opens with the
three steps a ketchup line actually has: a **crusher** makes paste, a
**mixer** blends that paste with vinegar into a ketchup mix, and a **cooking
vat** cooks the mix down into finished ketchup. Bottling and boxing are
unchanged, so the line went from four stages to five.

The cooking vat is **2x2 — the first machine in the game bigger than one
tile**, and deliberately the only one. Size is meant to say something: a
crusher and an in-line blender are small units, a batch vat that holds the
chain's longest process (the cook is four times the crush) is not. That
makes ticket 1's accumulate-widths layout do visible work for the first
time — every stage downstream of the vat sits one tile further east than the
old fixed-span formula would have put it, and the vat claims a second row of
the factory floor.

The other half of the change is a quiet proof that the data-driven design
holds. The vinegar requirement moved from the cooked stage to the *mixing*
stage, which is where a real line adds it — and that took no Rust change at
all, because `World` builds its pipe run for "whichever recipe declares a
fluid input" rather than for a named stage. Neither data file's
`schema_version` moved either: their shapes didn't change, only their
content. `sim_core`'s whole test suite was updated to the new five-stage
chain rather than trimmed — mass conservation across the extra hand-offs,
the fluid gate stalling and then running at its new stage, and `query_tile`
resolving the vat correctly from all four of its tiles.

### 2026-09-07 — Machines improvement, ticket 1: a machine can be bigger than one tile

Infrastructure, not content. Until now `World` hardcoded that every machine
occupies exactly one tile, so a stage's width was the constant
`BELT_LENGTH_TILES + 1` and stage `i` always started at `i * 5`. Machine size
is now **data**: a new `data/machines.ron` (the file CLAUDE.md's Project
Structure section has been anticipating alongside `recipes.ron`) maps each
machine kind to a tile footprint, loaded and validated by a new
`sim_core::sim::machines` module built the same way `recipe.rs` is —
`schema_version` checked, degenerate footprints and duplicate ids rejected,
and a recipe naming a kind the file doesn't define reported as a clear error
instead of a stage that silently never appears.

The chain now lays itself out by *accumulating* each stage's own machine
width rather than multiplying a fixed span, so a wider machine pushes
everything downstream east by exactly its extra tiles. A footprint's origin
is its minimum-X/minimum-Y corner, which is also the tile its belt feeds
into — stated once in `machines.rs`'s module doc and referenced everywhere
else, so a later renderer never has to guess which corner a coordinate
means. `chain_layout_json`, `chunk_snapshot_json` and `query_tile_json` all
carry the footprint now, and crucially `query_tile` resolves a machine from
*any* tile it covers, not just its origin — that is what the hover system
needs before a multi-tile machine can exist. A footprint straddling a 32x32
chunk boundary is handled rather than rejected: the machine belongs to every
chunk it overlaps, so a renderer loading only visible chunks still finds it.

The point of the ticket was to change nothing visible. Every shipped kind
(cooker, reducer, bottler, packer) declares 1x1, and a new test asserts the
four-stage chain's tile coordinates against literals copied from the old
formula rather than recomputed from the new code — plus tests that drive
genuinely 2- and 3-wide machines, since "every real kind is 1x1" would
otherwise make a width bug unobservable. The whole existing suite passes
untouched, Godot side included. Tickets 2-4 of this milestone are what
actually make a machine bigger.

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

