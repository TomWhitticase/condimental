# sauce-factory

Factorio-like automation game, themed around sauce production. Pre-alpha.

@docs/ARCHITECTURE.md

The file above is the authoritative architecture/tech-stack plan (Rust simulation
core + Godot/GDScript presentation via `gdext`, data-driven recipes, deterministic
tick simulation). Always follow it when writing code — it also explains *why*
each choice was made, which matters when a change seems to conflict with it.

## Working conventions

- Follow docs/ARCHITECTURE.md for stack/architecture decisions — don't deviate
  (e.g. don't reach for Unity/Unreal/Bevy, don't put gameplay logic in GDScript,
  don't hardcode recipes/ingredients in Rust) without discussing it first.
- This project is being built primarily by Claude Code. When a non-trivial design
  decision gets made that isn't already covered by docs/ARCHITECTURE.md, record
  it in the decisions log below so future sessions don't re-litigate it.
- Prefer small, playable increments over big upfront systems — this is a game, favor "can I run it and see something" early.
- Use plan mode for anything that touches architecture or adds a new major system; small fixes/tweaks don't need it.

## Commit messages

Conventional Commits, enforced by a local `commit-msg` hook + CI. See
[CONTRIBUTING.md](CONTRIBUTING.md). Run this once per clone/worktree so commits
made here don't get rejected:

```
git config core.hooksPath .githooks
```

## Decisions log

- 2026-09-06: Commit messages must follow Conventional Commits (enforced via
  git hook + CI) so history can generate release notes later. See CONTRIBUTING.md.
- 2026-09-06: Adopted the architecture plan in docs/ARCHITECTURE.md — Rust
  simulation core (ECS, deterministic ticks) + Godot 4/GDScript presentation via
  `gdext`, data-driven content (RON/JSON). Chosen specifically to be
  agent-friendly (text-native scenes, compiler-error feedback loop, mainstream
  languages). See that doc for full rationale and sources.
- 2026-09-06: Ran a critical review of that architecture doc specifically hunting
  for holes, then resolved every finding into firm decisions rather than leaving
  them open — see docs/ARCHITECTURE.md for details. Highlights: simulation state
  is fixed-point, not float (§3.9); ECS is `hecs` + `rayon`, not `bevy_ecs`
  (§1/§3.8); Godot/`gdext`/Rust versions are now pinned (§1a); `SimBridge` gained
  preview/undo/save-format design (§4); a mobile-export spike moved from
  milestone 8 to milestone 2 (§9); primary dev loop is headless `cargo test`, not
  editor hot-reload (§7). Don't re-litigate these without good reason.
