# sauce-factory

Factorio-like automation game, themed around sauce production. Pre-alpha — no tech stack chosen yet.

## Working conventions

- No stack decided yet. Don't assume a language/engine/framework — ask or propose options before scaffolding anything.
- This project is being built primarily by Claude Code. When a non-trivial design decision gets made (stack, architecture, core mechanics), record it here so future sessions don't re-litigate it.
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
