# Quality pass — plan (2026-09-13)

**No new features.** The project owner, after playing the game with other
people, asked for a pass that makes what exists correct, fast, fair and
legible. This document is the brief every ticket reads first: the
diagnosis, the rules that bind every ticket, and the ticket table.
Integration branch: `TomWhitticase/quality-pass` (worktree `quality-pass`);
workers branch from it as `qp-N-*`, the coordinator merges.

## What was reported

1. **Staffing is confusing.** Players "kept having to hire new staff
   despite not firing them", and could not tell why they needed staff at
   all or what staff cost.
2. **Items on belts are too close together.**
3. General: bugs, performance, exploits and balance, polish.

## Diagnosis (read before touching staffing)

`sim_core/src/sim/staff.rs` — on every payday (`pay_period_ticks`, 1000
ticks ≈ 33 s at `TICK_HZ` 30) the bill is paid one head at a time out of
the one money counter, in `staff.ron` order, and **the first head that
cannot be paid — and every head after it — leaves silently**. A player who
spends down to a few coins on equipment just before a payday loses most
of their crew, sees machines starve, and finds the staff panel reading
`hired 0`. Nothing announced it; the only trace is
`walked_out_last_payday` in the panel footer. That is the "kept having to
hire" report, and it is a design bug, not a misunderstanding.

Two things compound it:

- **Hiring is free and instant**, so the walk-out is also an *exploit*:
  fire everyone the tick before payday, hire them back the tick after,
  and wages are never paid. Nothing in the sim prevents it.
- **Why staff are needed is only on the build card and the hover.** Every
  kind in `data/machines.ron` declares a crew (`staff: [("manufacturing",
  1)]` …); the staff panel shows `assigned / demand` per category but
  never *which machines* make up the demand, so a player who has not read
  a card sees numbers with no cause.

Belts: `BeltSegment` (`sim_core/src/sim/belt.rs`) stores gaps as slack
beyond a fixed minimum item spacing; the shipped spacing is what `World`
passes when it builds a belt tile (see `world.rs`'s belt constants). The
presentation draws items at the sim's positions (`_carried_item_transform`
in `main.gd`). A wider minimum spacing is a sim constant change with
throughput consequences (fewer items per tile at full compression) that
the conservation and rate tests must follow.

## Ground rules for every ticket

- **No new features.** A ticket may add readouts, warnings, rules that
  close an exploit, and content changes, because those make existing
  features correct. It may not add a mechanic the docs do not describe.
- CLAUDE.md and the three design docs load automatically; follow them.
  `docs/DECISIONS.md` Part B has the long-form art/UI lessons.
- **Verify before reporting:** `cargo test --workspace` and
  `scripts/verify-core-loop.sh` green (set
  `PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`
  first — `cargo` is not on the default PATH). Anything visual:
  `scripts/screenshot-scene.sh` (after `cargo xtask`, run
  `godot --headless --path godot_project --import` TWICE or the scene draws
  no meshes), then actually read the PNG. The render is the test.
- Conventional Commits (`git config core.hooksPath .githooks` first). One
  DEVLOG entry per ticket. A condensed rule in CLAUDE.md's decisions log and
  the full reasoning appended to `docs/DECISIONS.md` Part A.
- Do not merge; report with the exact commit range. Do not touch
  `godot_project/project.godot` (Godot rewrites its comments) or the
  stray `AGENTS.md`.
- Every number you change (a wage, a spacing, a price) is content: change
  the prose that states it in the same commit, or do not change it.

## Tickets

| # | Ticket | Layer | Depends on |
|---|--------|-------|-----------|
| 1 | Staffing mechanics: no silent walk-out, no free dodge; exploit and balance audit of the whole sim | sim + data | — |
| 2 | Wider gaps between belt items | sim + Godot | — |
| 3 | Bug hunt and performance pass on the presentation layer | Godot | — |
| 4 | Staffing legibility: why, who, what it costs, and what is about to happen | Godot | 1 |
| 5 | Polish sweep and docs | Godot + docs | 2, 3, 4 |

### Ticket 1 — staffing mechanics, exploits, balance (sim)

Decide and implement, in `sim_core` and `data/`:

- **Walk-out is announced ahead and never silent.** Recommended: a head
  that cannot be paid on a payday is *unpaid* (stays hired, does not work
  — its pool claims nothing that period) and leaves only if the NEXT
  payday cannot pay it either; a paid payday clears arrears. Mirror the
  contract board's strikes. Put on the wire (`staff_snapshot_json`): the
  next payday's bill, the current balance's shortfall against it, the
  per-category unpaid count, and a `StaffEvent`-style record of who left
  at the last payday, so ticket 4 can warn BEFORE and announce AFTER.
- **Close the fire-before-payday dodge.** Recommended: a head is owed for
  the whole period it was employed in any part of — firing mid-period still
  bills at the next payday (notice pay); hiring mid-period bills at that
  payday. Pin it with a test that fires everyone the tick before payday and
  shows the bill unchanged.
- Audit the rest of the simulation for exploits and balance holes, fix the
  clear ones, list the rest: contract sign/lose loops, remove-and-replace
  to reset claims or timers, auto-fill edge cases, dock/pool behaviour, the
  `Fx` money ceiling (~32 767 — a five-line board reaches it; decide
  whether to widen the type or at least make saturation visible and
  tested), starting crew vs. shipped line, wages vs. a first line's income.
- Save format: the new fields follow `save.rs`'s `#[serde(default)]` rule;
  `schema_version` stays 1 unless a shape genuinely breaks.
- `godot_project/` untouched except where a headless script must change to
  keep the suite green (say so in the report).

### Ticket 2 — wider gaps between belt items (sim + Godot)

Make items on a belt visibly separated. Change the ONE spacing constant
the sim uses, decide the number by rendering it (the shipped
`scripts/screenshot-scene.sh --build --populate --closeup …`), keep the
tile transit time unchanged, update the tests whose expectations encode
items-per-tile (conservation, throughput, splitter fairness), and record
in `data`/prose wherever the old number was stated. If the presentation
needs to move (item scale, `_carried_item_transform`), do it here. Report
the before/after throughput of the reference line.

### Ticket 3 — bug hunt and performance (Godot)

`godot_project/main.gd` is ~10 k lines. Find and fix real bugs (play the
scene through `scripts/screenshot-scene.sh` flags and the headless
scripts; read the per-frame paths). Measure with `test_perf.gd` and the
stress fixture before and after. Known hotspot from the instancing ticket:
`JSON.parse_string` of ~80 KB chunk snapshots per dirty chunk per tick step
(~8 ms) — decide whether the wire can send less (a sim change is allowed
if it is the honest fix; coordinate through the report), whether parsing
can be skipped for chunks off-screen, or whether snapshots can be diffed.
Also: per-frame panel rebuilds that could be guarded, allocations in
`_process`, tooltip/meter work while nothing changed. Do not restructure
the staff panel (ticket 4 owns it).

### Ticket 4 — staffing legibility (Godot; after ticket 1)

Make staff understandable without reading a doc:

- **Why:** in the staff panel, under each category, list what asks for it —
  `asked for by: crusher ×1, mixer ×1, cooking vat ×2` — derived from the
  placements' crews (`_crews` / `query_tile_json`), and a one-line
  explanation of what the category does (`data/staff.ron` names or a
  short table in `main.gd`).
- **Cost, always visible:** the wage bill and the next payday countdown on
  the always-visible HUD (the money line or the larder strip), not only
  inside H. The build menu's card already shows a kind's crew; make sure
  the PRICE of running it (wage per period) reads beside it.
- **Before it happens:** when the balance cannot cover the next bill, a
  warning in words and a glyph, wherever money is shown, and on the panel.
- **After it happens:** a walk-out is announced like a lost contract
  (`_update_contract_notice`'s mechanism), naming the category and count,
  and the panel row says `unpaid` / `left` in words.
- Update `test_staff_panel.gd` and `--staff` screenshots; read the PNGs.

### Ticket 5 — polish sweep and docs (after 2, 3, 4)

Walk every screen with the screenshot flags (`--menu`, `--pause`,
`--shop`, `--market`, `--board`, `--staff`, `--production`, `--config`,
`--hover*`, `--bottleneck`, `--closeup`) at 1600x900 and fix what reads
wrong: overlaps, clipping, stale wording (any legend/footer/tooltip that
names a removed key or an old number), inconsistent capitalisation,
panels that cover each other. Then refresh `docs/DEVLOG.md`'s Current
status for the whole pass and reconcile CLAUDE.md's decisions log.

## Coordinator notes

(Appended per merged ticket.)
