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

### Tickets 1 and 2 merged (1cd331a) — read before ticket 4 or 5

**Ticket 1 (staffing sim, e32e0f1..53a5f91).** The payday rule is the
contract board's two strikes applied to people: a head the money does not
reach is UNPAID — still hired, not at its post until the next payday (its
machines starve with the existing `staff` stall) — and LEAVES only if unpaid
at the next payday too; a covered payday clears arrears, back pay settles
first, arrears never stack. Notice pay closes the dodge: a head employed for
any part of a period is owed for it (fired heads' notice is billed first),
so firing everyone the tick before payday bills identically. **Hiring is
refused unless `money >= next_bill + wage`** (otherwise hire-work-fire on an
empty balance is free labour); a refused hire is a silent no-op on the wire.
Money is `types::Money = I48F16` now (a payout at the old I16F16 ceiling
panicked in debug builds); save `money_bits` reads unchanged. NEW on
`staff_snapshot_json`: top-level `next_bill`, `shortfall` (0 when covered),
`unpaid_last_payday`; per category `owed`, `unpaid`, `left_last_payday`,
`can_hire`. `walked_out_last_payday` keeps its meaning (how many LEFT).

For ticket 4, verbatim from the worker: read `next_bill` for the footer's
bill (drop `_staff_bill`, which is now the run rate and differs after a
fire); warn on `shortfall > 0` before `next_payday_tick`; a row with
`unpaid > 0` should say `N unpaid · not working` (its meter shows the
shortfall as short); announce `left_last_payday` per category when
`next_payday_tick` moves, and `unpaid_last_payday` as "N went unpaid";
mark HIRE from `can_hire` and SAY WHY it is refused ("next payday needs 14,
you have 10"); `test_staff_panel.gd`'s literal snapshots need the new keys.

**Ticket 2 (belt gaps, 3f22340).** `ITEM_MIN_SPACING_TILES` is 0.5 (was
0.25). Widening it exposed a packing bug — a belt's front item stood on
the tile boundary where the next tile's first item also stood, drawn twice
at every boundary — fixed with a pre-tick snapshot of downstream room
(`Topology.belt_ahead`). Belt speed and models untouched. Reference-line
throughput unchanged (vat-limited). **Consequence, accepted by the
coordinator, not retuned:** one hopper lane now delivers 4 tomatoes per
100 ticks against the crusher's 80-tick craft, so a single-lane crusher is
belt-limited (~80 % busy, reads `▲ starved · waiting for Tomato` the rest
of the time). That is a true readout of a real constraint — a lane has a
throughput, as in Factorio — and the vat is still the line's bottleneck.
The recipe-ordering rule (crusher quickest, vat slowest) stays; the
crusher was NOT slowed to 100 ticks. Ticket 5: the hover already says why;
check nothing in the docs still claims a lane saturates a crusher.

### Ticket 3 merged — read before ticket 4's merge and ticket 5

**Ticket 3 (bugs + perf, 35859dd..ca0bed2).** The wire gained
`SimBridge::chunk_delta_json` (a chunk snapshot filtered to what changed;
the whole chunk after place/remove/rotate/load; `chunk_snapshot_json`
unchanged) and `SimBridge::revision` (ticks + commands, refused ones too),
and `main.gd` gates every per-frame snapshot parse on revision / viewport /
selection / open-modal change. Tick path 12 → 2 ms, wire 78 → 12 KB per
tick. Five bugs fixed with proofs: items in other chunks vanishing after a
rebuild; the camera panning after a right-drag released over a panel; TAB
into first-person capturing the mouse under a modal (`_fp_mouse_free` is
now the ONE decision every capture seam and the crosshair use — ticket 4:
use it, do not add another `not _x_open` chain); R rotating a tile under a
panel; X entering remove mode under M/C/H/P. Left for ticket 5: the
legend's third line runs under the clipboard at 1600x900 on every screen;
`_announce_contract_changes` shows only the last of several notices from
one poll (ticket 4's walk-out announcements go through the same seam —
prefer a queue). Left for later: `_read_world_layout` is ~40 of an 80 ms
rebuild on 1500 tiles; all perf numbers are on a debug `sim_core`.

### Coordinator notes — reconciling a parallel manual merge into main

Between ticket 4's merge and ticket 5's dispatch, a separate session (the
project owner, working directly, commits authored `tomwhitticase` /
co-authored `Claude Opus 5`) merged part of this pass into `main` by hand:
commit `365358c` brought in tickets 1 and 2, plus only ticket 4's FIRST
commit (the wage bill on the clipboard), by merging `qp-4-staffing-ux`'s
tip directly rather than the up-to-date `TomWhitticase/quality-pass`
branch — so **ticket 3 (bugs/perf) never reached `main`**, and ticket 4's
second half (the "asked for by"/blurb text, `98f4417`/`6a88c50`) was never
pushed to origin at all, only sitting in the local worker worktree.

Reconciled in a scratch branch off `origin/main`: merged
`origin/TomWhitticase/quality-pass` (bringing ticket 3 in; two conflicts
in `CLAUDE.md`/`docs/DECISIONS.md`, both just condensed-vs-full duplicate
decision-log entries, resolved by keeping `main`'s condensed versions and
adding quality-pass's new ticket-3 entry) then merged the LOCAL
`qp-4-staffing-ux` branch (bringing in the rest of ticket 4; clean, no
conflicts). Verified `cargo test --workspace` and
`scripts/verify-core-loop.sh` green (27 `_OK` markers) on the result, and
pushed straight to `main` as a fast-forward (`365358c..4c1058b`) — **`main`
now has every ticket 1-4 change**, tested and pushed.

**Consequence for ticket 5: branch from `main`, not
`TomWhitticase/quality-pass`.** The `quality-pass` integration branch is
now behind `main` (missing ticket 4 entirely) and should be treated as
retired — do not merge into it. Ticket 5's own merge target is `main`
directly, since it is the last ticket and everything else is already
there.
