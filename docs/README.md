# Docs index

Sauce Factory is a Factorio-like automation game, themed around sauce
production. Three docs cover the whole design, split by concern so each stays
useful even if the others change:

| Doc | Covers | Read it for |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Tech stack, engine choice, ECS/simulation design, data formats, testing, interop layer | How the game runs and is built |
| [UI_UX.md](UI_UX.md) | Camera, art medium, visual theme, materials/lighting, UI style | How the game looks and feels |
| [GAMEPLAY.md](GAMEPLAY.md) | Core loop, sauce/equipment progression, economy, contracts, staff | What the game actually is to play |
| [DEVLOG.md](DEVLOG.md) | Current project status + a dated log of what's been done | Catching up on progress without re-deriving it from `git log` |
| [DECISIONS_LOG.md](DECISIONS_LOG.md) | Every decision as one condensed rule, newest first | Checking whether a question is already settled before re-opening it |
| [DECISIONS.md](DECISIONS.md) | Full-text decision records (the long form of DECISIONS_LOG.md, UI_UX.md §2b and ARCHITECTURE.md §9a) | The *why* behind a rule, or the history of a change |
| [WORKFLOW.md](WORKFLOW.md) | Building, verifying, the one non-headless check, commits, branches/PRs/issues on GitHub, the DEVLOG decision, Orca orchestration | How work actually gets done, proven and shipped in this repo |
| [PIPES_AND_CONVEYORS_PLAN.md](PIPES_AND_CONVEYORS_PLAN.md) | The pipes & conveyors overhaul: goals, ticket DAG, and the wire/asset contracts every ticket shares | What that update is and how its tickets fit together |
| [STAFF_POOLS_PLAN.md](STAFF_POOLS_PLAN.md) | The staff pools MVP: goals, ticket DAG, the data shapes and the wire contract every ticket shares | What that epic is and how its tickets fit together |

The first three are the authoritative design docs. **Nothing in this folder
is loaded automatically** — the root [`CLAUDE.md`](../CLAUDE.md) links to
these rather than `@`-including them, since an `@`-include is inlining and is
paid for on every single request (the context restructure of 2026-09-16; see
DECISIONS_LOG.md's newest entry). `CLAUDE.md` carries a table mapping a task
to the doc that covers it, so read the one your work touches.

DEVLOG.md, DECISIONS_LOG.md and DECISIONS.md are different in kind — a
status/history log, a rule list and an archive of full records — and all
three grow without bound. Check them when you need status, history, or
whether a rule already exists; append to them when you finish meaningful work
or make a decision.

This index exists for quickly orienting on the `docs/` folder itself (e.g.
before adding a new doc, or when only skimming rather than reading
everything).

**Adding a new design doc:** keep it scoped to one concern (don't let
architecture, visuals, and gameplay bleed into each other — cross-reference
instead of duplicating), add a row here, add a row to `CLAUDE.md`'s "Where to
look" table, and state up front what it does and doesn't cover, the way the
existing three do. Do not `@`-include it.
