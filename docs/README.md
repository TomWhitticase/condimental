# Docs index

Sauce Factory is a Factorio-like automation game, themed around sauce
production. Three docs cover the whole design, split by concern so each stays
useful even if the others change:

| Doc | Covers | Read it for |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Tech stack, engine choice, ECS/simulation design, data formats, testing, interop layer | How the game runs and is built |
| [UI_UX.md](UI_UX.md) | Camera, art medium, visual theme, materials/lighting, UI style | How the game looks and feels |
| [GAMEPLAY.md](GAMEPLAY.md) | Core loop, sauce/equipment progression, economy, contracts, staff | What the game actually is to play |

All three are authoritative and are `@`-included directly into the root
[`CLAUDE.md`](../CLAUDE.md), so any Claude Code session in this repo already
has them loaded in full — this index exists for quickly orienting on the
`docs/` folder itself (e.g. before adding a new doc, or when only skimming
rather than reading everything).

**Adding a new doc:** keep it scoped to one concern (don't let architecture,
visuals, and gameplay bleed into each other — cross-reference instead of
duplicating), add a row here, add it to the `@`-include list and decisions log
in `CLAUDE.md`, and state up front what it does and doesn't cover, the way the
existing three do.
