# sim_core

Invariants for the Rust simulation core. Reasoning is in `docs/DECISIONS.md`;
the plan is `docs/ARCHITECTURE.md` §3/§4/§6/§10.

## Boundaries

- **`src/lib.rs` is the only file that may touch `godot`.** `src/sim/` has zero
  `gdext` references and stays runnable under plain `cargo test`. Bridge
  methods are thin translation, never simulation logic.
- **The wire is JSON strings via `serde_json`**, never `Dictionary`/`Array`;
  Godot sees snapshot structs only, never internal representations.

## Determinism (§3.9 — the rule most easily broken)

- **No `f32`/`f64` in shared, persisted or replayable state.** `Fx` =
  `fixed::types::I16F16`. Floats appear only in `*Snapshot` structs at the
  presentation boundary; that conversion is one-way and never feeds back.
- **Money is `types::Money` = `I48F16`, not `Fx`** — same 16 fractional bits,
  48 integer. `Fx`'s ~32 767 ceiling is minutes of play away, and there a
  payout panics in debug / wraps negative in release. `Economy::tick` credits
  with a saturating add regardless.
- **Deterministic iteration order.** Prefer `BTreeMap`/`BTreeSet` over
  `HashMap` for anything walked in a tick or serialized; every `World::tick`
  pass walks a list the topology pass built in tile order.
- **The tick is the only clock**: no `std::time`, no wall-clock reads, no RNG.

## World, topology, placement

- **`World::new` builds nothing** — empty grid and graph, `STARTING_MONEY`.
  Everything arrives via real `Purchase` + `Place` commands.
- **`recompute_topology` runs only on `Place`/`Remove`/`Rotate`, never per
  tick**: a full single-threaded rebuild of occupancy, hand-off links and fluid
  networks (volumes carried across by tile). `rayon`/partitioning is milestone
  8, deliberately unbuilt.
- **Declaring a *different* fluid empties the tank *and* the run it feeds**,
  before the recompute (`try_set_tank_fluid` → `purge_runs_fed_by`). Carrying
  volumes across by tile would otherwise relabel the fluid already in the
  pipes; the purged volume is destroyed, never credited or moved. One run,
  not "every": the tank is 1×1 and discharges through the face it points at.
  Declaring the fluid the tank and its run already hold settles the name and
  destroys nothing (`declaration_settles_what_is_there`). That, not the
  switch, is the commoner click — reading this rule as being only about
  *changing* a declaration is what hid #121.
- **`sim::reference_chain` is the shared test fixture**, built only through the
  public command API — it can do nothing a player cannot; all placements `East`.
- **`Direction::East` = `+X` is the regression anchor** and `default()`;
  `North` = `+Y`; `rotate_cw` is the only rotation.
- **`machines.rs` owns the footprint origin convention** — min-X/min-Y corner,
  occupying `[x, x+width) × [y, y+height)`, and the tile a belt feeds into.
  Read its module doc before touching rotation.
- **A kind may name several recipes; the machine's *inputs* pick one.**
  `Machine::choose` starts the first fully-satisfied recipe
  `RecipeRegistry::for_machine` offers, so `recipes.ron` file order is the
  tiebreak; with none satisfied it holds the closest, which is what makes the
  stall readout name a raw the machine is really waiting for. **There is no
  player-facing picker and no command that sets a recipe** — do not add one.
  The in-flight recipe *is* saved (`SavedMachine::recipe`): a craft under way
  has already eaten the inputs it would be re-derived from.
- **A recipe may draw N fluid networks, matched by fluid id; it deposits into
  exactly one.** `validate_fluid_scope` refuses a recipe that names one fluid
  twice (that is one input with the units added up) and still caps
  `fluid_outputs` at one, off the outlet face. The N draw borrows plus the
  deposit are asserted disjoint in `world.rs`, not assumed.
- `validate_chain` panics at construction on a part of `recipes.ron` nothing
  can start — a component with no buyable machine or no obtainable input
  (solid or fluid): content errors fail at load, not silently at runtime.
  **Several independent production lines are fine**; it stopped demanding one
  connected component in issue #42, because mayonnaise shares nothing with
  tomato.

## Snapshots and dirtiness

- **Delta granularity is per chunk, not per entity.** `chunk_delta` narrows
  what a dirty chunk's read carries; `chunk_snapshot` stays the full read.
- **An empty belt's `moved_ticks` advances every tick and dirties nothing.**
  Its rubber runs (`BeltSegment::tick_toward`, 2026-09-25) but there is no
  product on it to redraw, and a delta per idle tile per tick is what this
  counter exists to avoid; Godot extrapolates the phase off the one-per-tick
  rule instead (`main.gd`'s `_belt_idle_phase`). Do not "fix" the silence by
  marking it dirty. A JAM is what stops the clock, and the only thing that
  does — the field is the TICK the tile last ran on, so every running belt
  in the world is in phase and a freed one rejoins it. **A belt is laid on a
  tick** (`BeltSegment::new`'s `laid_on_tick`; an empty one takes the save's
  `tick_count` at load): Godot reads a placed tile once, and an idle tile
  dirties nothing afterwards, so a clock that starts at 0 is drawn at 0 for
  as long as the tile stays empty.
- **A preview mutates nothing and does not move `revision()`.**
  `World::spend_preview` answers what a spend would do to the next payday for
  the command Godot is about to issue, by calling the same price functions the
  command arms call (`Shop::cost_of`, `Ingredients::cost_of`, the offer's own
  `price`, the category's `wage`) and mirroring each arm's own refusal
  predicate — never a second copy of a price rule. **An unaffordable spend does
  not warn** (`affordable: false`): the economy would refuse it, so there is no
  consequence to confirm and the click keeps its ordinary refusal. It is a
  question, so
  `tests/spend_preview.rs` pins `revision()` and `to_save_ron()` byte-identical
  across one. **The payday is a WARNING, not a gate** since 2026-09-26:
  `StaffPools::hire` refuses only an unknown category, and `hire_warns` is
  advice on the wire (it was `can_hire`, a refusal, from 2026-09-13).
- **Fluid movement, tank drain, machine load, production stats and staff state
  deliberately dirty nothing** — all polled (`fluid_snapshots_json`,
  `fluid_tank_snapshots_json`, `machine_load_snapshots_json`,
  `production_snapshot_json`, `staff_snapshot_json`). Only a discrete player
  action dirties those tiles; do not "fix" it by marking them dirty.

## Data files

- Content changes must never require a Rust change; if they do, a data field is
  missing. `data/*.ron` is `include_str!`'d, so resolution is identical under
  `cargo test`, the editor and an exported build.
- Every loader **rejects** a `schema_version` mismatch, never migrates; content
  edits do not bump it, a shape change does.

## Save/load (`save.rs`)

- A **full RON state snapshot** with `schema_version`, rejected not migrated.
  Not a command log (that would replay through *current* logic).
- **Fixed-point is stored as raw bits** (`FxBits`), never decimal text.
- **No filesystem code in this crate** — Rust produces/consumes the string;
  `godot_project/game_session.gd` owns `user://saves/`.
- Load rebuilds structure through real `Purchase`/`Place`, then restores live
  state through `pub(crate)`/`pub(super)` `restore_*` methods — keep those
  restore-only and non-public.

## Tests

- Primary loop is headless: `cargo test -p sim_core`, no Godot. Every sim
  change needs coverage before merging.
- `tests/` is one file per concern. `tests/common/mod.rs` keeps hoppers and the
  tank fed and funds `WAGE_RESERVE` so a long run does not go broke and lose
  its crew; its `STOCK_COST`/`WAGE_RESERVE` literals are pinned against
  `data/*.ron` and fail loudly on a price change.

## Module map (`src/sim/`)

- `types` — `Fx`, `Money`, ids, `TileCoord`, `Direction`, chunk encoding
- `world` — `hecs::World` of placed things; commands, topology, tick, snapshots
- `machines` — `MachineRegistry` from `machines.ron`; footprint + rotation
- `machine` — `Machine`, `TimedProcess`, `LoadWindow`
- `recipe` — `RecipeRegistry` from `recipes.ron`
- `belt` — `BeltSegment`, gap-based item storage; `splitter` — round-robin diverter
- `fluid` — `FluidNetwork`, `FluidTank`, pressure equalization
- `hopper` — raw-solid feeder + auto-fill; `inventory` — slot-based storage
- `dock` — a dock's own stock in `DOCK_CAPACITY` places; contracts draw from it
- `economy` — money, contract board, active contracts, `DeliveryStock` (the docks summed)
- `contracts` — `ContractRegistry` from `contracts.ron`
- `shop` — `ShopCatalog` + `OwnedEquipment` (money is not here)
- `ingredients` — prices, owned stock, the definition of "raw"
- `staff` — `StaffCatalog` + `StaffPools`: crew claims, payday, hire/fire
- `unlocks` — `UnlockRegistry` from `unlocks.ron`: progression gates, off the tick path
- `stats` — `ProductionLedger`: made/used over rolling windows (the P screen)
- `save` — RON snapshot save/load
- `reference` — `reference_chain`: the shipped layout as a test fixture
