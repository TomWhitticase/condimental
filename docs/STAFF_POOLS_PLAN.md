# Staff Pools MVP — plan

Status: in progress (started 2026-09-12). Integration branch:
`TomWhitticase/staff-pools-mvp`, branched from `main` at `8cee800`. Built as
one Orca orchestration Run; the tickets below map 1:1 to Tasks.

This is the first cut of GAMEPLAY.md §7 ("Staff — a resource pool, not
NPCs") and the third cost axis of §5 ("staff wages — recurring upkeep").
It resolves ARCHITECTURE.md §11's open item "`StaffPools` has no ECS shape"
and two of GAMEPLAY.md §11's open questions (allocation under a shortfall;
recruiting cost and firing) *for the MVP*, and says so where it does.

## Goal

A factory needs people to run, and people cost money every period.

1. Four staff categories — **manufacturing, maintenance, packaging,
   logistics** — each with a hired headcount and a wage per head, all
   content in a new `data/staff.ron`.
2. Every machine kind in `data/machines.ron` declares the crew it needs
   *while running*; the dock needs logistics staff to ship. A machine (or
   dock) that cannot get its full crew this tick does not run this tick.
3. Wages are paid every pay period out of the one money counter. A head
   that cannot be paid leaves.
4. The player hires and fires from a staff panel (**H**) and can see, per
   category, how many are hired, how many the factory is asking for right
   now, and how many are short. A machine short of staff reads that way on
   hover and on its andon light.

Not in this MVP (deliberately): maintenance demand (nothing corrodes yet,
so the category exists with zero demand); a recruiting fee; a firing
penalty or delay; player-set priority between machines; a reputation
score. Each is a §11 open question this plan leaves open or answers only
provisionally, as marked.

## Why it is shaped this way

- **A capacity check, not entities** (GAMEPLAY §7). Staff are aggregate
  demand versus capacity, the same shape `PowerNetwork` will take. There
  is no pathfinding, no per-person state, nothing per tile.
- **Per tick, in stable order, whole crew or nothing.** Each tick the
  pools reset to headcount and machines claim in the simulation's
  existing stable machine order (`topology.machines`, then docks). A
  machine that cannot claim its full crew claims nothing and does not
  advance — its progress pauses, its inputs are untouched. Because the
  order is the same every tick, the *same* machine is always the one that
  goes short: no flicker, no oscillation, deterministic (ARCH §3.9). This
  is the "first-come-first-served" option of GAMEPLAY §11, chosen for the
  MVP because it needs no new player-facing control; player-set priority
  can replace it later without touching the wire.
- **Understaffed is a kind of starved.** The machine-load window (the
  machine-load epic) has three states; a machine waiting on people is
  waiting on a resource, exactly like one waiting on sugar, so it records
  `Starved` and the andon shows amber. What distinguishes it is the
  *stall reason*, which gains a `staff` kind. No fourth light.
- **Wages spend money; hiring does not.** §5 wants wages as a recurring
  upkeep separate from ingredients and equipment. The MVP charges nothing
  to hire and nothing to fire (GAMEPLAY §11, answered provisionally: "no,
  for now — wages are the cost"), so the only staff number on the ledger
  is the wage bill, and the panel can show it as one figure.
- **A head you cannot pay leaves.** On payday the bill is paid one head
  at a time, in `staff.ron` category order, through `Economy::try_spend`
  (still the only debit, still all-or-nothing per call). The first head
  that cannot be paid, and every head after it, leaves. Softer than "the
  whole crew walks out" (a cliff at money 0) and stricter than debt
  (which §1 does not allow — there is one money counter and it does not
  go negative). The panel says how many left.
- **A new game starts with a crew sized to the shipped line**, the same
  way `STARTING_MONEY` is sized to the reference chain's bill. A first
  factory therefore runs without a visit to the staff panel, wages bite
  from the first payday, and the *second* line is what makes the player
  hire. The relationship is pinned by a test, not the literals.
- **All content.** Categories, wages, pay period and starting crew are
  `staff.ron`; crew per kind is `machines.ron`. Adding a category or
  re-crewing a machine is a data edit (ARCH §3.6, §10).

## Ground rules for every ticket

- Follow CLAUDE.md and the three design docs. Read `docs/DECISIONS.md`
  Part B (UI_UX §2b long form) before drawing anything.
- The sim never learns about art. Godot never derives a fact the sim
  already knows — it reads `demand`, `assigned`, `short` and the stall
  reason from the wire; it never recomputes them from machine counts.
- Verify: `cargo test --workspace`, `scripts/verify-core-loop.sh`, and for
  anything visual `scripts/screenshot-scene.sh` — then READ the PNG.
- `cargo` is at `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo`
  (not on PATH). After `cargo xtask`, run
  `godot --headless --path godot_project --import` TWICE or the scene draws
  no meshes. Godot rewrites `godot_project/project.godot` (comment loss):
  never stage that. An untracked `AGENTS.md` may appear: leave it. A
  rejected commit (the commit-msg lint) leaves files STAGED. Godot's
  `test_*.gd.uid` files are tracked: commit the new one beside a new test.
- Conventional Commits; a DEVLOG entry per ticket; a condensed rule in
  CLAUDE.md's decisions log plus the full reasoning in `docs/DECISIONS.md`
  Part A; a new UI_UX §2b point for any presentation lesson, numbered
  after the last one present.
- Do not merge into the integration branch yourself. Commit on your worker
  branch and report it in `worker_done`; the coordinator merges.
- If a decision would change the wire contract below, use
  `orca orchestration ask` before deviating.

## The data (ticket 1 builds it)

### `data/staff.ron` (new, `schema_version` 1)

```ron
(
    schema_version: 1,
    pay_period_ticks: 1000,
    categories: [
        StaffCategory(id: "manufacturing", name: "Manufacturing", wage: 2, starting_headcount: 4),
        StaffCategory(id: "maintenance",   name: "Maintenance",   wage: 2, starting_headcount: 0),
        StaffCategory(id: "packaging",     name: "Packaging",     wage: 2, starting_headcount: 2),
        StaffCategory(id: "logistics",     name: "Logistics",     wage: 2, starting_headcount: 1),
    ],
)
```

`wage` is whole money units per head per pay period. The loader
(`sim_core/src/sim/staff.rs`, `StaffCatalog`) rejects a mismatched
`schema_version`, no categories, a duplicate id, an empty name, a zero
`pay_period_ticks`. A zero wage is allowed (a volunteer category is a
legitimate content choice). File order is the payday order and the panel
order. Embedded with `include_str!` like the other data files.

### `data/machines.ron` (`schema_version` 1 → 2)

Every `MachineKind` gains a required `staff` list of `(category, count)`:

| kind | staff |
|---|---|
| hopper, fluid_tank, splitter | `[]` |
| crusher | `[("manufacturing", 1)]` |
| mixer | `[("manufacturing", 1)]` |
| cooking_vat | `[("manufacturing", 2)]` |
| bottler | `[("packaging", 1)]` |
| packer | `[("packaging", 1)]` |
| dock | `[("logistics", 1)]` |

Bumped because the field is required and explicit (an omitted crew should
be a loader error, not a silent zero — the recipes file set this
precedent at v2 → v3). A `staff` category that is not in `staff.ron`, or a
duplicate category within one kind, is a **test** failure
(`StaffCatalog::validate_machines`, the `validate_covers` pattern), not a
construction panic. The shipped starting crew (4/0/2/1) is exactly the
reference chain's demand; `sim_core/tests/staff.rs` pins *that
relationship* (sum the chain's kinds from `machines.ron`), not the
literals — the `tests/shop.rs` pattern.

## The wire contract (ticket 1 builds it; ticket 3 consumes it)

### Commands

```json
{"type": "hire", "category": "manufacturing"}
{"type": "fire", "category": "manufacturing"}
```

Each moves one head. `hire` is free and never refused for a known
category. `fire` at headcount 0, and either with an unknown category, is a
silent no-op (the `Purchase` convention). Neither dirties a chunk. (Match
the exact `type` spelling to how `command_from_json` names the existing
commands — `snake_case` of the variant.)

### The tick

Order, inside `World::tick`:

1. Existing step 0 (auto-fill) and step 1 (fluid) unchanged.
2. **Pools reset**: `assigned = 0`, `demand = 0` for every category;
   `available = headcount`.
3. Step 3 (machines), in `topology.machines` order: a machine that would
   otherwise record `Working` this tick (start a craft, advance, or
   deposit) first asks for its kind's whole crew. `demand` counts every
   head asked for. If every category can supply, `available` drops,
   `assigned` rises, and the tick proceeds as today. If any category
   cannot, nothing is taken from any category, the machine records
   `Starved`, remembers which category refused it (the first short one in
   `machines.ron` order), and does nothing else this tick. A machine that
   would *not* have worked this tick (no inputs, output full) asks for
   nothing and does not count toward `demand`.
4. Step 6 (docks), in `topology.docks` order: `ship_into` asks for the
   dock kind's crew the same way, but only when the dock holds something
   to ship. A refused dock holds its buffer (32 per item, then the belt
   backs up — mass conservation as today) and remembers it was refused.
5. **Payday**, after the economy's own tick: when `tick_count` reaches
   `next_payday_tick`, pay heads one at a time in `staff.ron` order via
   `Economy::try_spend(wage)`; the first refusal fires that head and every
   head after it across all remaining categories. Record `last_bill` (what
   was actually paid) and `walked_out_last_payday`. Then
   `next_payday_tick += pay_period_ticks`. A zero-wage head always stays.

The claim is per tick and per machine; a machine mid-craft that loses its
crew (the player fired someone) pauses where it is and resumes when a crew
is free again. Progress is never lost.

### On the wire

New `SimBridge::staff_snapshot_json()`, polled (it changes every tick and
dirties nothing, like `machine_load_snapshots_json`):

```json
{
  "pay_period_ticks": 1000,
  "next_payday_tick": 3000,
  "last_bill": 14,
  "walked_out_last_payday": 0,
  "money": 2986.0,
  "categories": [
    {"id": "manufacturing", "name": "Manufacturing", "headcount": 4, "wage": 2,
     "demand": 4, "assigned": 4, "short": 0},
    {"id": "maintenance",   "name": "Maintenance",   "headcount": 0, "wage": 2,
     "demand": 0, "assigned": 0, "short": 0},
    {"id": "packaging",     "name": "Packaging",     "headcount": 2, "wage": 2,
     "demand": 2, "assigned": 2, "short": 0},
    {"id": "logistics",     "name": "Logistics",     "headcount": 1, "wage": 2,
     "demand": 1, "assigned": 1, "short": 0}
  ]
}
```

`demand`, `assigned`, `short` (= `demand - assigned`) describe the LAST
completed tick. `money` is repeated here (as `shop_snapshot_json` does) so
the panel needs one poll.

`MachineSnapshot` (chunk snapshots, `query_tile_json`,
`machine_load_snapshots_json`) — `stall` gains one kind:

```json
"stall": {"kind": "staff", "item": "manufacturing"}   // refused its crew; item = category id
```

The stall-reason precedence: a staff refusal is reported only when the
machine would otherwise have worked, so it can never mask an input or
output shortfall; check it *after* the existing six kinds find nothing.

`query_tile_json`'s machine and dock variants gain `crew`, the kind's
requirement, for hover:

```json
"crew": [{"category": "manufacturing", "count": 2}]
```

`shop_snapshot_json` entries gain the same `crew` (for the build-menu
card). `DockSnapshot` gains `"understaffed": true|false` (refused its crew
on the last tick it had something to ship).

### Save

`SavedWorld` gains `staff: SavedStaff { headcount: BTreeMap<String, u32>,
next_payday_tick, last_bill, walked_out_last_payday }` with
`#[serde(default)]` (a pre-epic save loads with the starting crew and a
payday `pay_period_ticks` from its tick). `schema_version` stays 1, like
`auto_fill` and the load ring. The machine's "refused category" and the
dock's `understaffed` flag are saved too (`#[serde(default)]`), because
the 2600-tick lockstep test compares every snapshot every tick and both
appear in snapshots.

## The icons (ticket 2 builds them; ticket 3 places them)

Four UI icons, one per category, rendered from voxel models through the
existing icon pass (UI_UX §5: icons are rendered from real voxel assets,
never separately drawn 2D art; §2b point 7 for the pipeline). Models are
UI props, never placed in the world, so they need no footprint rule and
no camera-corner rule; they must read as a silhouette at 64 px:

| model | reads as | category |
|---|---|---|
| `staff_manufacturing` | a hard hat | manufacturing |
| `staff_maintenance` | an open-ended spanner | maintenance |
| `staff_packaging` | a taped carton with a roll of tape on it | packaging |
| `staff_logistics` | a pallet jack (two forks, a handle) | logistics |

Rules: `palette.ron` colours only; the hard hat and the tape are
`hazard_yellow` (signage colour for people, §7); the spanner and the jack
are `metal` so they catch the light; the carton is `matte`. Follow the
`coin` asset (the payout epic) for how a UI-only model is registered in
`assets.rs`, rendered by `voxicons` to `assets/voxels/icons/<model>.png`,
and staged. Review each PNG. A `cargo test -p voxgen` test asserts every
staff model is non-empty and fits a 32³ volume.

## Presentation (ticket 3)

- **H** toggles the STAFF panel. Plain dark plate (UI_UX §5: the clipboard
  is reserved for orders and checklists). Heading and balance on one
  line, like the market. One row per category in wire order: the ticket-2
  icon, the name, `hired N`, a **meter** of `assigned / demand` (GAMEPLAY
  §10 suggests a gauge per category; a filled bar with the two numbers
  printed on it — position and digits, never colour alone, §7), the wage
  per head, and two buttons: **HIRE** and **FIRE** (FIRE disabled at
  headcount 0). A row whose `short > 0` says so in words beside the
  meter: `2 short`. Footer: `wages 14 / period`, `next payday in 12 s`
  (sim ticks, 60 per second, from `current_tick()`), and, when non-zero,
  `✗ 3 left unpaid last payday` (glyph + word, §7). Maintenance's row
  reads `nothing to maintain yet` in place of its meter while its
  `demand` is 0 and its headcount is 0.
- The panel polls `staff_snapshot_json` every `LOAD_POLL_TICKS` (30) of sim
  time from the end of `_advance_simulation`, the machine-load pattern;
  a pause polls nothing. Buttons issue `hire`/`fire` and re-poll at once.
- ESC closes it; opening it closes the build menu, market, contracts
  board and config panel (follow `_toggle_contract_board`). Legend gains
  the H line. Play the existing UI click sound on the buttons if
  `audio.gd` has one; do not synthesise a new sound.
- **Hover**: a machine's hover gains `crew: 2 manufacturing` (from
  `query_tile_json`'s `crew`) and, when its stall is `staff`, the load
  line reads `starved · 38% busy · short of manufacturing staff`. A dock
  hover gains `crew: 1 logistics` and `held · short of logistics staff`
  when `understaffed`. The build menu card shows `crew: 1 manufacturing`
  under the price.
- **Andon**: nothing new to draw — the sim says `starved`. The test asserts
  an understaffed machine's light is the amber one and its hover names
  the category.
- **Fixture**: `test_layouts.gd` gains `build_understaffed_line`: the
  reference line on auto-fill, then `fire manufacturing` twice, run until
  the vat has been refused (the vat needs 2; the crusher and mixer, earlier
  in order, take the remaining 2). `scripts/screenshot-scene.sh` gains
  `--staff` (panel open on the reference line) and `--understaffed` (that
  fixture, hover on the vat). Look at both PNGs.
- **Proof**: `godot_project/test_staff_panel.gd`, headless, prints its own
  success marker, and is added to `scripts/verify-core-loop.sh`: the panel
  rows match the snapshot; HIRE/FIRE change `headcount` through the real
  command; the understaffed fixture's vat reads `staff` on the wire, amber
  on the andon, and names the category in hover; ESC closes.

## Tickets

| # | Ticket | Layer | Depends on |
|---|---|---|---|
| 1 | `staff.ron`, `machines.ron` v2, pools, per-tick claim, payday, commands, wire, save, tests | `sim_core` | — |
| 2 | Four staff-category icons | `voxgen` | — |
| 3 | Staff panel (H), polling, hover, fixture, screenshot flags, proof | Godot | 1, 2 |

Tickets 1 and 2 run in parallel. Ticket 3 starts from the integration
branch once both are merged. The coordinator verifies the union
(`scripts/verify-core-loop.sh` on the merge, not on the branches), writes
DEVLOG's status line, and merges to `main`.

## Decisions made by tickets (read before consuming their output)

### Ticket 1 (sim) — landed 2026-09-12

- Everything in "The data" and "The wire contract" above is built as
  written; the real `staff_snapshot_json` at tick 1200 on the reference
  line is byte-for-byte the plan's shape (`money` included).
- `Machine::tick` gained `crew: Option<CrewRequest>`; a refused machine
  records `Starved`, sets `refused_by` (the category), and, mid-craft,
  **pauses by shifting `start_tick` forward one tick**, which is reported
  as a change — so a paused machine dirties its chunk every tick (same
  rate as an auto-filling hopper; Godot's progress bar reads `start_tick`).
- `DockSnapshot.understaffed`, `SavedMachine.refused_by`,
  `SavedDock.understaffed` and `SaveData.staff: Option<SavedStaff>` are all
  `#[serde(default)]`; `demand`/`assigned` are NOT saved (recomputed on the
  first tick after load), so the staff snapshot is compared from tick 1.
- `machines.ron`'s loader reads the version off a header first, so a v1
  file fails as "wrong version", not "missing field `staff`".
- **A broke factory loses its crew at the first payday**, which changed
  the long zero-money test fixtures: `tests/common/mod.rs` has
  `WAGE_RESERVE` (300 periods of the shipped bill, pinned against the data
  by `tests/staff.rs`) and `wages_paid()`; money assertions there are
  expressed as earnings. Godot fixtures start at 3000 and were untouched.
- Stall precedence: `staff` is reported only after the six older kinds
  find nothing, so it never masks an input or output shortfall.
- Docs landed with the ticket: DEVLOG, CLAUDE.md, DECISIONS Part A (eleven
  calls), GAMEPLAY §9/§11, ARCHITECTURE §4/§11.

### Ticket 2 (icons) — landed 2026-09-12

- Icon files are `assets/voxels/icons/staff_<category id>.png`, so ticket 3
  builds the path from the wire's `id` with no lookup table.
- Four UI-only props in `assets.rs` ("Staff category icons", registered
  after `coin`, never placed): hard hat (`hazard_yellow`), double
  open-ended spanner (`metal`), carton with `hazard_yellow` tape and a
  tape roll on the lid (`matte`), pallet jack (steel with `rubber_dark`
  wheels and grips, so it is `metal` + `matte`). No palette change.
- Reviewed at 1x and 3x on a dark plate; the hard hat was reworked once
  (its first crown read as a bell). UI_UX §2b point 31 records the lesson.

### Ticket 3 (Godot) — landed 2026-09-12

- Built as "Presentation" specifies, with the coordinator's andon
  amendment (below). `main.gd`: `_build_staff_panel` and friends, `H`,
  `_staff_open` in every modal guard, `_poll_staff` on its own stamp at
  `LOAD_POLL_TICKS`, `_crews` seeded from `query_tile_json` per rebuild
  for machines and docks, `staff` in `_stall_reason_text`, the crew line
  on machine and dock hover and on the build-menu card.
- The footer's `wages N / period` is `Σ headcount × wage` in GDScript
  (`_staff_bill`) — confirmed with the coordinator as arithmetic on wire
  numbers (the shop-meter precedent), not a derived fact. A sim-side
  `next_bill` is the cleaner home if the payday rule ever changes.
- `audio.gd` has no UI click; the buttons are silent (nothing synthesised).
- Fixture `test_layouts.gd`'s `build_understaffed_line` +
  `understaffed_stall`; proof `test_staff_panel.gd` (`STAFF_PANEL_OK`);
  `screenshot-scene.sh --staff` (implies `--populate`) and
  `--understaffed`. The panel's rows were compacted and the build-menu
  card resized (`SHOP_SLOT_SIZE` 144x138, tighter inset) after the first
  renders — UI_UX.md §2b point 32.

## Coordinator notes

- 2026-09-12: `main` removed the in-world andon stack lights (commit
  `da292b6`, "the hover panel is the indicator") while tickets 1 and 2 were
  in flight; merged into the integration branch before ticket 3 started
  in earnest. **Ticket 3 amendment:** the "Presentation" section's andon
  bullet is void — there is no floor light to assert. The understaffed
  proof asserts the hover panel's load line (`starved` status, the
  `staff` reason naming the category) and the wire, nothing on the floor.
