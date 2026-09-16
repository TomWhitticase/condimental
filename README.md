<div align="center">

# CONDIMENTAL

**Condiment production, automated.** A Factorio-like factory-automation game about
making sauce — tomatoes and sugar in one end, boxed ketchup out of the loading dock
at the other, against standing orders that pay you for keeping them fed.

[![status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-orange)](docs/DEVLOG.md)
[![sim: Rust](https://img.shields.io/badge/simulation-Rust-b7410e)](sim_core/)
[![engine: Godot 4.6](https://img.shields.io/badge/engine-Godot%204.6-478cbf)](godot_project/)
[![art: voxel](https://img.shields.io/badge/art-code--generated%20voxels-8a9a5b)](assets/voxels/)

![The shipped production line, running](docs/media/factory-line.png)

</div>

---

## What it is

A deterministic **Rust** simulation core drives a **Godot 4 / GDScript** presentation
layer through `gdext`. The world starts as an empty factory hall and 3,000 in the
bank: you buy every belt, pipe and machine, lay them out, feed them raw ingredients,
sign contracts, hire the crew that runs the machines, and try to make the income
outpace the ingredients and the wage bill.

- **A real production chain, not a façade.** Tomatoes are crushed into paste, blended
  with sugar and vinegar into a liquid mix, cooked in a 2×2 vat, bottled, boxed, and
  shipped off a loading dock — five recipes, each a stage with its own throughput.
- **Belts *and* pipes.** Solids ride conveyors (with corners, merges and splitters);
  the middle of the line is plumbed, because that is what a sauce plant looks like.
  Pressure equalises along a pipe run, and the cooking vat is the one machine no belt
  ever touches.
- **Contracts, not a spot market.** Standing orders for a quantity of goods on a
  recurring cadence, signed off a board for a price. Miss a window too many times in a
  row and the order is *lost* — it goes back on the board to be signed again. (Orders
  that demand a *mix* of different sauces are the design; one sauce exists so far.)
- **Staff as a capacity constraint.** Four categories (manufacturing, maintenance,
  packaging, logistics). Machines need a crew to run at all, wages come out of the
  same money counter every payday, and an unpaid head idles before it walks.
- **Production statistics.** A Factorio-style production screen: what the factory made
  and used per item over rolling windows, with per-minute rates.
- **Code-generated voxel art.** Every model in the game is emitted by a Rust generator
  from one shared palette — no hand-authored `.vox` files, no flat textures.
- **Deterministic and saveable.** Fixed-point maths only (no `f32`/`f64`) in shared
  simulation state, so a tick is reproducible; the whole world round-trips to a save
  file, and the game autosaves every three minutes.

## Gallery

<table>
<tr>
<td width="50%"><img src="docs/media/title-screen.png" alt="Title screen"><br><sub><b>The title screen</b> — a hand-composed still life of the line, in the same hall you play in.</sub></td>
<td width="50%"><img src="docs/media/build-mode.png" alt="Build mode with a placement ghost"><br><sub><b>Build mode</b> — a translucent ghost, a facing, and a prompt that says <code>✔ PLACE</code> or why it will not.</sub></td>
</tr>
<tr>
<td width="50%"><img src="docs/media/build-menu.png" alt="Build menu"><br><sub><b>Build menu (E)</b> — every buildable kind, its price, and the crew it will ask for.</sub></td>
<td width="50%"><img src="docs/media/ingredient-market.png" alt="Ingredient market"><br><sub><b>Market (M)</b> — raw tomato, sugar and vinegar, bought by the crate.</sub></td>
</tr>
<tr>
<td width="50%"><img src="docs/media/contracts-board.png" alt="Contracts board"><br><sub><b>Contracts board (C)</b> — nine offers, from a corner shop to an export pallet.</sub></td>
<td width="50%"><img src="docs/media/staff-panel.png" alt="Staff panel"><br><sub><b>Staff (H)</b> — hired against asked-for, per category, with the wage bill and payday countdown.</sub></td>
</tr>
<tr>
<td width="50%"><img src="docs/media/production-panel.png" alt="Production statistics panel"><br><sub><b>Production (P)</b> — made and used per item, over 5s to 1h windows or all time.</sub></td>
<td width="50%"><img src="docs/media/closeup-crusher.png" alt="Close-up of the crusher"><br><sub><b>The crusher</b> — whole tomatoes in, pails of paste out, beside the sugar hopper.</sub></td>
</tr>
<tr>
<td width="50%"><img src="docs/media/closeup-mixer.png" alt="Close-up of the mixer"><br><sub><b>The mixer</b> — paste and sugar in on belts, ketchup mix out into a pipe.</sub></td>
<td width="50%"><img src="docs/media/closeup-cooking-vat.png" alt="Close-up of the cooking vat"><br><sub><b>The cooking vat</b> — 2×2, piped in and piped out, agitator turning while it works.</sub></td>
</tr>
</table>

## The chain, as shipped

Every model below is a real in-game asset, rendered straight out of the voxel pipeline:

<div align="center">

| <img src="assets/voxels/icons/item_tomato.png" width="44"> <img src="assets/voxels/icons/item_sugar.png" width="44"> <img src="assets/voxels/icons/item_vinegar.png" width="44"> | <img src="assets/voxels/icons/machine_crusher.png" width="44"> | <img src="assets/voxels/icons/machine_mixer.png" width="44"> | <img src="assets/voxels/icons/machine_cooking_vat.png" width="44"> | <img src="assets/voxels/icons/machine_bottler.png" width="44"> | <img src="assets/voxels/icons/machine_packer.png" width="44"> | <img src="assets/voxels/icons/machine_dock.png" width="44"> |
|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| tomato · sugar · vinegar | **crusher**<br>4 tomato → paste | **mixer**<br>paste + sugar + 2 vinegar → mix | **cooking vat**<br>mix → ketchup<br>*(240 ticks)* | **bottler**<br>ketchup → bottle | **packer**<br>bottle → box | **dock**<br>ships every tick |

</div>

Nothing above is hardcoded. Recipes, machine footprints, crews, prices, ingredients,
contracts and staff categories are all data in [`data/*.ron`](data/) — a new sauce is a
content change, never a code change.

## Install and play

No binary releases yet (see [Releases](#releases)); you build it from source. It takes
about five minutes, most of which is one Rust compile.

### 1. Prerequisites

| You need | Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs/) | 1.98.1 or newer, edition 2024 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| [Godot](https://godotengine.org/download/) | **4.6.3-stable**, standard build | Not the .NET/C# build. The version is pinned — see [ARCHITECTURE.md §1a](docs/ARCHITECTURE.md) |
| `git` | any | |

Put the Godot binary on your `PATH` as `godot` (on macOS:
`ln -s "/Applications/Godot.app/Contents/MacOS/Godot" /usr/local/bin/godot`), or point
every script at it with `export GODOT_BIN=/path/to/godot`.

Platforms: developed and played on macOS (Apple Silicon); Linux and Windows build the
same way. An Android export has been proven working; iOS is blocked (see
[ARCHITECTURE.md §9a](docs/ARCHITECTURE.md)).

### 2. Build

```bash
git clone https://github.com/TomWhitticase/condimental.git
cd condimental
cargo xtask
```

`cargo xtask` is the whole build. It compiles `sim_core`, copies the compiled library
into `godot_project/bin/` where `sim_core.gdextension` expects it, stages the voxel
models and UI icons into `godot_project/voxels/`, and then runs Godot's asset importer
**twice** (the `.vox` importer is an editor plugin that only registers on the pass after
the first scan — one pass leaves you with an invisible world).

The default is a debug build; `cargo xtask --release` builds the optimized one (slower
to compile, and the only build worth reading a performance number off).

### 3. Play

```bash
godot --path godot_project
```

That opens the title screen. **NEW GAME** starts you on a bare factory floor with
3,000 in the bank — the world is empty by design, and you buy and place every tile.

Want a factory to poke at instead of building one? Write the demo saves and pick one
from **LOAD GAME**:

```bash
scripts/make-demo-saves.sh
```

Saves live in Godot's per-user data directory, not in the repo (on macOS:
`~/Library/Application Support/Godot/app_userdata/CONDIMENTAL/saves`). The running
game prints the resolved path once at startup.

### 4. Check it works (optional)

```bash
scripts/verify-core-loop.sh
```

19 Rust test suites and 32 headless Godot proofs, including the real `gdext` JSON
boundary, save/load round trips and the interactive scene's input path. For a picture
instead of a pass/fail:

```bash
scripts/screenshot-scene.sh out.png 1600x900 --fixture chain --populate
```

<details>
<summary><b>Troubleshooting</b></summary>

- **The world is empty and no machines appear in a demo save.** The asset import did
  not complete. Run `godot --headless --path godot_project --import` twice, or just
  `cargo xtask` again.
- **`cargo: command not found`.** `~/.cargo/env` is not sourced in this shell; open a
  new terminal after installing rustup.
- **`git status` shows `godot_project/project.godot` modified after you ran the game.**
  Godot rewrites that file in canonical form, stripping its comments. Don't commit it —
  `git checkout godot_project/project.godot`.
- **A script can't find Godot.** `export GODOT_BIN=/absolute/path/to/godot`.
- **Failed extension load / "can't open dynamic library".** The library is gitignored
  build output, so a fresh clone has none until `cargo xtask` runs. Re-run it.

</details>

## How to play

### Controls

| Key | Does |
|---|---|
| `1`–`9`, `0` | Pick that hotbar slot up — puts one in your hand, ready to place |
| left click | Place what is in your hand (or click a hopper/tank to configure it) |
| `R` | Rotate — the ghost while placing, or whatever is under the cursor |
| `X` | Remove mode — click to pick a placed tile back up into your inventory |
| `SHIFT` while placing | Buy a replacement as each one goes down |
| `E` | Build menu — buy belts, pipes, machines, buildings |
| `M` | Ingredient market — buy raw tomato, sugar, vinegar |
| `C` | Contracts board — sign orders, re-sign lost ones |
| `H` | Staff — hire and fire per category |
| `P` | Production statistics |
| `F1` | Fold the on-screen keybind legend away |
| `TAB` | Swap between the isometric view and a first-person walk-through |
| `WASD` / drag / arrows, scroll / `+` `-` | Pan and zoom (isometric); move and look (first person) |
| hover a machine | Its recipe, buffers, crew, and whether it is working, starved or blocked |
| `ESC` | Close what is open — else the pause menu (save, sound, music, quit) |

### Your first factory

1. **Buy the line.** Press `E`. The shipped five-stage chain — hopper, belts, crusher,
   a second hopper for sugar, mixer, fluid tank, pipes, cooking vat, bottler, packer,
   dock — costs a little over 1,000, so a new game can afford all of it outright with
   change to spare for ingredients.
2. **Lay it out.** Pick a slot with a number key and click to place. Two rules decide
   whether things connect: **a belt carries toward the way it faces**, and **a machine
   takes input on the face opposite the way it points and discharges out the face it
   points at**. `R` rotates. The mixer is 1×2 and needs *two* feed tiles (paste on one
   lane, sugar on the other) — two ingredients merged onto one belt will deadlock.
3. **Plumb the middle.** The mixer's output is a fluid, so run **pipes**, not belts,
   from mixer → cooking vat → bottler. Vinegar comes from a fluid tank into the mixer
   the same way. The bottler is where the product becomes solid again.
4. **Buy stock and feed it.** Press `M`, buy tomato, sugar and vinegar (`SHIFT`+click
   buys everything you can afford). Then **click** a hopper or a tank: its panel has
   **FILL**, an **AUTO-FILL** switch that keeps it topped up from bought stock every
   tick, and — for a hopper — a picker for which ingredient it meters onto the belt.
5. **Sign orders.** Press `C`. A new game runs one starter order; the board has eight
   more, each with a price to sign, a quantity, a cadence and a number of strikes. The
   clipboard in the corner tracks every signed order: checklist, payout, countdown,
   and whether it is `✔ READY`, `⚠ AT RISK` or `⚠ LAST CHANCE`.
6. **Watch the money.** The dock ships finished boxes into a shared delivery pool
   automatically — one box pays one contract, never two, and the pool caps at 48 per
   item, at which point the dock refuses deliveries and the belt backs up. Ingredients
   are roughly a third of revenue, so overproducing into a pool nobody is buying from
   is a real loss.
7. **Mind the crew.** Press `H`. Every machine you place adds to what a category asks
   for, whether or not it is running; wages are paid every 1,000 ticks (about 33
   seconds at the 30 Hz tick). A payday you cannot cover idles heads for a period, and
   they leave if the next payday is short too.
8. **Find the bottleneck.** Hover any machine for `working` / `starved` / `blocked`,
   a busy percentage and the reason it is waiting — or press `P` for the whole
   factory's throughput. The cooking vat's 240-tick cycle is the longest stage on the
   shipped line, so that is where a single line backs up first.

## Under the hood

```
data/           content: recipes, machines, ingredients, shop, contracts, staff
sim_core/       the Rust simulation (src/sim/ is pure, src/lib.rs is the gdext edge)
godot_project/  the Godot 4 presentation layer
assets/voxels/  voxel asset pipeline — generators are the source, models/ is output
xtask/          `cargo xtask`: build sim_core, stage the library and models for Godot
scripts/        verification, voxel build, screenshot review
docs/           design docs and history
```

Four rules hold the project together, and every one of them is load-bearing:

- **Gameplay logic lives in Rust, never in GDScript.** Godot renders, formats and
  sorts; the simulation decides. Anything deterministic or persisted belongs to
  `sim_core`, behind one `SimBridge` interface of JSON commands and snapshots.
- **Content is data, never code.** If a content change needs a Rust change, a data
  field is missing.
- **No `f32`/`f64` in shared simulation state** — fixed-point (`Fx`, `Money`) only.
  Determinism is a hard requirement, not an aspiration.
- **Art is generated, not drawn.** One `palette.ron`, one Rust generator per asset,
  and the same models feed both the world and the UI icons.

The design docs are authoritative and explain *why* each choice was made:
[ARCHITECTURE.md](docs/ARCHITECTURE.md) (stack, ECS, interop, save format, milestones),
[UI_UX.md](docs/UI_UX.md) (camera, voxel pipeline, UI style, feedback),
[GAMEPLAY.md](docs/GAMEPLAY.md) (loop, progression, economy, contracts, staff),
[WORKFLOW.md](docs/WORKFLOW.md) (build, verify, ship), plus a dated
[DEVLOG](docs/DEVLOG.md) and a [decisions log](docs/DECISIONS_LOG.md).

## How this project is built

CONDIMENTAL is developed almost entirely by AI coding agents working in parallel
worktrees, with a human owner directing and playtesting. That only works with real
engineering discipline around it, so the repository is set up to be legible to an agent
that starts every session with no memory of the last one:

- **A context map, not a context dump.** The root [`AGENTS.md`](AGENTS.md) (the
  [agents.md](https://agents.md/) convention) is a short list of invariants that points
  at the doc covering each task, with per-area files in `sim_core/`, `godot_project/`
  and `assets/voxels/`. Always-loaded context is under a thousand tokens; everything
  else is read on demand, which is cheaper *and* keeps the agent reading the reasoning
  rather than a summary of it.
- **Repeatable procedures are skills**, not prose: adding a voxel asset, verifying,
  reviewing a screenshot, recording a decision, opening a PR, filing an issue.
- **Verification is headless and deterministic**, so an agent can prove its work
  without a human watching a screen — one script runs the Rust suites and every Godot
  proof through the real interop boundary. The one check that cannot be headless
  (*does it look right?*) is a scripted render the agent has to open and read.
- **Every pull request shows the change in a screenshot.** A local hook and a CI job
  both refuse a PR body without an image.
- **[Conventional Commits](https://www.conventionalcommits.org/)**, enforced by a
  `commit-msg` hook and re-checked in CI, so the history stays usable as a changelog
  source.
- **Decisions are recorded** in the style of an
  [architecture decision record](https://adr.github.io/) — a one-line rule in the
  condensed log, the full reasoning in the archive, and a note in the doc it affects —
  so a settled question is not re-litigated by the next session.
- **Anything noticed and not fixed becomes a GitHub issue**, labelled `deferred`. A
  finding mentioned only in a closing message is lost.

[CONTRIBUTING.md](CONTRIBUTING.md) has the mechanics: commit format, branch and PR
flow, issue templates, and when the DEVLOG needs an entry.

## Roadmap

Milestones 1–7 are done: the scaffold, a mobile export spike, the core loop, a full
multi-stage chain, the economy, fluids and timed processes — plus the epics that made
it a game rather than a demo (shop and inventory, real placement, splitters, the piped
chain rework, the delivery dock, contracts as data, the staff pool, production
statistics, save/load, and a music and sound pass).

**Next, roughly in order:**

| | Plan |
|---|---|
| **Scale-up** (milestone 8) | Power as a network, a tiered update scheduler, chunk activity tracking; stress-tested into the thousands of entities |
| **Logistics expansion** (milestone 9) | Transport between factory sites |
| **Platform polish** (milestone 10) | Mobile touch controls and export, packaging, a full UI pass |
| **More sauce** (content) | BBQ (a long slow-cook stage), mayo (refrigeration and a cold chain), hot sauce (acid-resistant equipment, fermentation), then aged and derived sauces — chilli mayo needs mayo *and* hot sauce, so the sauce list becomes a dependency tree |
| **Multiplayer** (stretch) | Designed for in the architecture, deliberately deferred |

The current open backlog is [GitHub issues](https://github.com/TomWhitticase/condimental/issues);
`docs/DEVLOG.md` is what has actually been built, dated.

## Releases

None yet. CONDIMENTAL is **pre-alpha** — the core loop closes and it is genuinely
playable, but there is no packaged build, no balance pass on any of the placeholder
numbers, and save files are rejected rather than migrated when the schema moves. Build
from source as above.

Tagged builds are planned for the platform-polish milestone, at which point the
Conventional Commit history becomes the release notes.

---

<div align="center">
<sub>Pre-alpha · private repository · no licence granted yet</sub>
</div>
