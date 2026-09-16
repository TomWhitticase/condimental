# CONDIMENTAL

A Factorio-like automation game, themed around sauce production. Pre-alpha.
A deterministic Rust simulation core (`sim_core`) drives a Godot 4 / GDScript
presentation layer through `gdext`; all content is data in `data/*.ron`; the
art is code-generated voxel models.

The shipped chain: tomato + sugar → crusher → mixer → (pipe) → cooking vat →
(pipe) → bottler → packer → dock. Hopper and fluid tank feed it; contracts pay
for what the dock ships. The game boots to a main menu, the world starts
empty, and the player buys and places every tile.

## The rules that always apply

- **Gameplay logic lives in Rust, never in GDScript.** Godot renders,
  formats and sorts; the sim decides. Anything deterministic or persisted is
  `sim_core`'s.
- **Content is data, never code.** Recipes, machines, prices, contracts and
  staff are `data/*.ron`. If a content change needs a Rust change, a data
  field is missing.
- **No `f32`/`f64` in shared simulation state** — fixed-point only (`Fx`,
  `Money`). Determinism is a hard requirement.
- **Follow the design docs; do not deviate without discussing it first.**
  Concrete deviations to refuse: Unity/Unreal/Bevy, gameplay logic in
  GDScript, recipes hardcoded in Rust, flat-textured non-voxel assets, a
  free-rotating camera, a spot market beside contracts.
- **Prefer small, playable increments** over big upfront systems.
- **Use plan mode** for anything that touches architecture or adds a major
  system. Small fixes do not need it.
- Scenes stay text `.tscn`/`.tres`. Never binary, no visual scripting.
- Conventional Commits, enforced by a hook + CI. Once per clone:
  `git config core.hooksPath .githooks`.
- **Never commit or push to `main`.** Work on a branch and merge it through a
  pull request (`gh pr create`); a `pre-push` hook refuses the alternative.
- **Every PR body shows the change in a screenshot** — render with
  `scripts/screenshot-scene.sh`, read it, publish with
  `scripts/pr-screenshot.sh`, paste its markdown. A hook and CI both refuse a
  PR without an image; a change with no visual surface shows the state it
  affects.
- **Anything you notice and decide not to fix becomes a GitHub issue**
  (`gh issue create --label deferred`). A finding reported only in a final
  message is lost — the next session starts cold.

## Layout

```
data/           content: recipes, machines, ingredients, shop, contracts, staff
sim_core/       the Rust simulation (src/sim/ is pure, src/lib.rs is the gdext edge)
godot_project/  the Godot 4 presentation layer
assets/voxels/  voxel asset pipeline — generators are the source, models/ is output
xtask/          `cargo xtask`: build sim_core, stage the lib and models for Godot
scripts/        verification, voxel build, screenshot review
docs/           design docs and history
```

Each of `sim_core/`, `godot_project/` and `assets/voxels/` has its own
`CLAUDE.md` with that area's invariants, loaded when you work there.

## Where to look

Read the doc for the thing you are changing — these are authoritative, and
each explains *why* its choices were made. Read that reasoning before you
change something that seems to conflict with it.

| Working on | Read |
|---|---|
| Stack, ECS, interop, save format, milestones | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) |
| Anything visual: art, UI, camera, audio | [docs/UI_UX.md](docs/UI_UX.md) |
| Mechanics, progression, economy, contracts, staff | [docs/GAMEPLAY.md](docs/GAMEPLAY.md) |
| Verifying, committing, branches/PRs/issues, DEVLOG, Orca | [docs/WORKFLOW.md](docs/WORKFLOW.md) |
| Whether a question is already settled | [docs/DECISIONS_LOG.md](docs/DECISIONS_LOG.md) |
| Why a rule exists (full records) | [docs/DECISIONS.md](docs/DECISIONS.md) |
| Status and dated history | [docs/DEVLOG.md](docs/DEVLOG.md) |

Skills cover the repeatable procedures: `voxel-asset`, `verify`,
`screenshot-review`, `record-decision`, `open-pr`, `file-issue`.

## Not yet built, deliberately

Add these when a milestone needs them; do not improvise a different shape.
`rayon` parallelism and the tiered scheduler (ARCHITECTURE §3.7/§3.8,
milestone 8); `preview_command`/`undo`/`redo` (ARCHITECTURE §4).
