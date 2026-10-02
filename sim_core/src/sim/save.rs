//! Saving a world to text and loading one back (the "Save/load + a real
//! main menu" epic's ticket 1).
//!
//! # What a save file is, and what it deliberately is not
//!
//! **A save is a full state snapshot, in RON, carrying its own
//! `schema_version`.** It is *not* ARCHITECTURE.md §4's "command log plus
//! periodic full-state snapshots" — that design is written for undo/redo and
//! for lockstep multiplayer replay, neither of which exists (no undo/redo is
//! implemented; multiplayer is an explicit stretch milestone), so building a
//! replay engine to answer "put my factory back" would be solving a problem
//! this project does not have. It is also the more fragile of the two here:
//! a command log has to be replayed through whatever game logic the *current*
//! build has, so any change to how a command behaves silently rewrites every
//! old save, where a snapshot of the resulting state cannot drift that way.
//! Layering a command log on top later, when undo/redo or replay actually
//! land, is still open — a snapshot format does not preclude one, and
//! ARCHITECTURE.md §4 has been amended to say exactly that.
//!
//! RON because it is already this project's data format (`data/*.ron`), so a
//! save is as readable and as diff-able as the content files beside it, and
//! `schema_version` is **rejected rather than migrated** on mismatch —
//! copying, deliberately, what every other loader in this crate does
//! ([`super::recipe`], [`super::machines`], [`super::contracts`],
//! [`super::shop`], [`super::ingredients`]).
//!
//! **Where a save file lives is not this crate's business.** Rust produces
//! and consumes the RON *string*; resolving `user://saves/<slot>.ron` is
//! Godot's, because `user://` is Godot's. There is deliberately no
//! filesystem code anywhere in `sim_core`.
//!
//! # The bar: running-state-identical, not layout-identical
//!
//! A save that restored only what is where would be visibly wrong the
//! moment it was loaded: machines would restart their batches, items would
//! jump back to tile boundaries, pipes would empty, contract clocks would
//! reset. So everything live is carried — a machine's in-flight
//! [`TimedProcess`](super::machine::TimedProcess) and both its buffers, each
//! item's exact sub-tile position on its belt, every hopper's level, every
//! tank's and every pipe tile's exact fluid volume, each splitter's
//! next-due side, each dock's stock, lifetime shipped total and last
//! payment tick, every contract's own cadence clock and delivered/missed
//! counters, money, owned equipment, owned ingredient stock, and the tick
//! count.
//!
//! **Every fixed-point value is stored as its raw bits** ([`FxBits`]), never
//! as decimal text: [`Fx`] is `I16F16`, and a decimal print-and-reparse can
//! land a fraction of a bit away — which is exactly the divergence a
//! lockstep comparison of a saved and a reloaded world would catch a
//! thousand ticks later. `fixed`'s own serde feature is deliberately not
//! enabled for this: raw bits is the representation this format wants to
//! promise, and it should not be a property of a dependency's formatting
//! choices.
//!
//! # How a load rebuilds a world
//!
//! In two halves, and the split is the whole design:
//!
//! 1. **Structure is rebuilt through the real player commands** —
//!    [`PlayerCommand::Purchase`] then [`PlayerCommand::Place`], one pair per
//!    placed thing, exactly as [`super::reference::reference_chain`] builds
//!    the shipped line. Nothing about placement, footprints, collision,
//!    hopper contents or topology is re-implemented here, so a load cannot
//!    build a world a player could not have built, and cannot drift from
//!    what `Place` means. A refused `Place` is a **corrupt save** (an
//!    [`Err`] naming the kind and the tile), never a silently missing tile.
//! 2. **Live state is written on top through restore-only methods** that are
//!    not commands and are not public — `pub(crate)` on the body types
//!    ([`BeltSegment::restore`](super::belt::BeltSegment), and its siblings
//!    on `Splitter`/`Dock`/`FluidTank`/`Inventory`) and `pub(super)` on
//!    `World` itself (see world.rs's "save/load seam" block). **That
//!    visibility is the rule, not an accident:** a player cannot set a
//!    machine's craft progress, a dock's lifetime total or a money balance,
//!    and none of those should become part of the surface that says what a
//!    player can do. A loader legitimately must, so it gets a door of its
//!    own that opens only from inside the crate.
//!
//! Money is the one value that has to be written twice. Each rebuild
//! purchase costs real money, so the loading world is funded one item at a
//! time and the **saved** balance is installed at the very end — the
//! rebuild's spending can therefore never leak into what the player has.
//! Funding per item rather than pricing the whole bill up front also kept
//! an arbitrarily large factory loadable while money was `Fx` (which
//! saturates around 32 767, so a single up-front bill for a few thousand
//! belt tiles would not have fit); money is [`Money`] now, 2^47 wide, and
//! the per-item funding stays because it is still the simplest correct
//! way to keep the rebuild's spending out of the saved balance.
//!
//! # What is deliberately not in the file
//!
//! Anything derived or constant: every buffer's capacity (a Rust
//! constant), a pipe network's shape or
//! which fluid it carries or which tile a tank discharges into (all
//! re-derived by `World::recompute_topology` from the placements, which is
//! the same pass a `Place` triggers), and the dirty-chunk set (a freshly
//! loaded world reports **every** chunk dirty, so a chunk-driven renderer
//! redraws all of it).
//!
//! A machine's **recipe** left that list in the BBQ epic. It used to be
//! derived — a kind named exactly one recipe — but a kind may now name
//! several and a placed machine picks whichever its inputs satisfy, so the
//! pick is live state. It has to be saved for the same reason `process` is:
//! reloading mid-craft must resume *that* craft, and re-deriving the pick
//! from a half-consumed buffer would resume a different one.
//!
//! # Forward compatibility
//!
//! **That window is closed.** It ran while nothing had shipped with a save
//! file: `schema_version` 1 was whatever the save batch merged with, and
//! four fields were added inside it on those terms — `auto_fill` on
//! `SavedHopper` and `SavedTank` (the auto-fill ticket), `SavedMachine::
//! recipe` (the BBQ epic) and [`SaveData::lifetime_deliveries`] (the
//! progression gates) — each a one-line `#[serde(default)]`, which is what
//! the per-kind body records are named-field structs for. The note that
//! reserved it said what ends it: "after this batch ships, a field addition
//! is a version bump like any other shape change".
//!
//! **`schema_version` is therefore 2**, and version 2 is this sweep's
//! batch: [`SavedTank::fluid`] is the field that moved it (issue #43: a
//! tank declares what it holds), and [`SavedMachine::mothballed`] and
//! [`SavedDock::mothballed`] (issue #19: a crewed building can be switched
//! off) joined the same number rather than each claiming one of their own.
//! Three shape changes reviewed and merged in the same sweep are one shape
//! change as far as a file is concerned — no build ever wrote a save with
//! one of them and not the others — so a third and a fourth version would
//! name states that never existed. A v1 file is **rejected**, not migrated — the rule ARCHITECTURE.md §4 sets
//! for this format and every other loader in this crate follows — and the
//! cost of that is bounded by the same fact the window rested on: no
//! release has ever written a save file, so the only v1 files in existence
//! are development ones. [`SaveError::UnsupportedSchemaVersion`] names both
//! numbers so the refusal reads as "this save predates a change" rather
//! than as a parse failure.
//!
//! **`schema_version` 3 is per-dock stock** (2026-09-26, `super::dock`'s
//! module doc). The economy's one shared delivery pool is gone, so
//! `SaveData::delivery_stock` went with it, and what waits to be sold is now
//! each [`SavedDock::contents`], checked against the dock's places. A v2 file
//! is rejected like a v1 file: its pool has no dock to stand on, and handing
//! it to whichever dock came first would invent a fact the file never said.
//!
//! **`schema_version` 4 is the pay cycle** (2026-09-26, `super::stats`'s
//! module doc): [`SavedProduction::cycles_closed`] and each
//! [`SavedItemStats`]'s `produced_this_cycle` / `produced_last_cycle`. No
//! ring can re-derive them — a payday falls mid-bucket — and a v3 file has
//! no honest value for them: zero this cycle would read as a factory that
//! made nothing since payday. Rejected like every older version. The same
//! unreleased bump carries [`SavedMachine::refusal_hold`] (the understaffed
//! marker's refusal window).
//!
//! A field addition from here on is a bump, for real this time. Adding one
//! `#[serde(default)]` and leaving the version alone would mean a v2 file
//! written by an older build and one written by a newer one are both "v2"
//! and are not the same shape, which is the exact ambiguity the version
//! number exists to prevent.
//!
//! # Version 5: a gate counts boxes, not delivery windows
//!
//! [`SaveData::lifetime_boxes_sold`] is why version 4 became version 5
//! (2026-09-26, the owner's change): the progression gates stopped keying
//! off satisfied delivery windows and started keying off boxes of a named
//! product sold. A file from before that has no such ledger, and this loader
//! will not write one for it: reconstructing the ledger means reading
//! `data/contracts.ron` to find out what a delivery window shipped, so the
//! answer moves with a rebalance, which is not something a load may depend
//! on. Outside the loader it is a different question, and
//! `scripts/migrate-save-v4-to-v5.py` answers it — it replays this crate's
//! own arithmetic over each contract's `deliveries_made` and checks the
//! result against the docks' `shipped_total`. A v4 file is still refused
//! here, by number.
//!
//! The tempting alternative was one `#[serde(default)]` and no bump, and it
//! was tried before it was rejected. It loads, but it loses a tier the
//! player paid for: gate state is *derived* from the ledger on every load,
//! so an older save with BBQ open comes back with both gates shut, the gated
//! raws unbuyable, the gated pages re-drawn as silhouettes, and any BBQ
//! contract already signed still active but impossible to restock — it
//! stands there striking out. Seeding the ledger from `lifetime_deliveries`
//! instead is no better: a delivery window says nothing about which product
//! was in it, so the seed would invent boxes of a sauce the factory may
//! never have made, and the two errors point in opposite directions.
//!
//! Refusing is the honest third answer and the one this crate's every other
//! loader already gives: the file is from before the rule changed, this
//! build cannot know what it sold, and it says so instead of guessing. No
//! release has ever written a save file, so every file in existence is a
//! development one.
//!
//! [`SaveError::UnsupportedSchemaVersion`] carries that reason in words for
//! the version it knows about, so the refusal reads as "this save predates
//! a change, and here is the change" rather than as a bare number mismatch.
//!
//! Nothing else about the change touches the file: `lifetime_deliveries` is
//! still written and still restored, because it still drives the HUD's
//! delivery count — it simply no longer opens anything.
//!
//! # Version 6: a rack remembers what it is reserved for
//!
//! [`SavedRack::filter`] is why version 5 became version 6 (2026-09-28, the
//! owner's request that a warehouse rack can be told to hold one
//! ingredient — `super::storage`'s module doc). A rack's body stopped being
//! an empty marker, so the `WarehouseRack` record changed shape, and a
//! field addition is a bump (above).
//!
//! Unlike version 5 there is nothing to reconstruct: every rack in a
//! version 5 file was open to every raw, which is exactly `filter: None`.
//! The loader still refuses the older file by number, as every loader in
//! this crate does, and `scripts/migrate-save-v5-to-v6.py` rewrites one
//! outside the game — a change of text, not of meaning.
//!
//! # Version 7: a splitter remembers which way it splits
//!
//! [`SavedSplitter::mode`] is why version 6 became version 7 (2026-09-29,
//! the owner's request that a splitter can split left, right, both ways as a
//! T, or three ways — `super::splitter`'s module doc). The same change
//! renamed the cursor's second side: `next_output: branch` is
//! `next_output: left` now, because a splitter has two sides it can branch
//! to and the old one is the left.
//!
//! Version 6's precedent holds exactly: every splitter in a version 6 file
//! was what is now `mode: left`, so there is nothing to reconstruct. The
//! loader refuses the older file by number, and
//! `scripts/migrate-save-v6-to-v7.py` rewrites one outside the game.
//!
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::belt::BeltSegment;
use super::contracts::ContractRegistry;
use super::dock::{DOCK_KIND, Dock};
use super::economy::ActiveContract;
use super::fluid::{FLUID_TANK_KIND, FluidTank};
use super::hopper::{HOPPER_KIND, Hopper};
use super::inventory::Inventory;
use super::machine::{LOAD_BUCKETS, LoadBucket, LoadWindow, Machine, TimedProcess};
use super::machines::MachineRegistry;
use super::recipe::RecipeRegistry;
use super::splitter::{SPLITTER_KIND, Side, SplitMode, Splitter};
use super::staff::{StaffPools, StaffRestore, StaffRestoreError};
use super::stats::{ItemStats, ProductionLedger, STAT_BUCKETS, STAT_WINDOW_COUNT, StatKind};
use super::storage::{BayPool, WAREHOUSE_RACK_KIND, WarehouseRack};
use super::types::{Direction, Fx, ItemId, Money, RecipeId, TileCoord};
use super::world::{
    BELT_KIND, CROSSING_KIND, PIPE_KIND, PlayerCommand, World, crossing_pipe_direction,
};

/// The save format's version. Bumped whenever the shape below changes;
/// a file naming any other version is rejected, never migrated — see the
/// module doc, and every other loader in this crate.
pub const SAVE_SCHEMA_VERSION: u32 = 7;

/// A fixed-point value as its raw `I16F16` bits — see the module doc for why
/// a save never stores one as decimal text.
///
/// `#[serde(transparent)]` so it reads as a plain integer in the file; every
/// field holding one is named `*_bits` (or sits in a list this type's own
/// name labels) so nothing mistakes it for a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FxBits(pub i32);

impl FxBits {
    pub fn of(value: Fx) -> Self {
        Self(value.to_bits())
    }

    pub fn value(self) -> Fx {
        Fx::from_bits(self.0)
    }
}

/// The balance as its raw [`Money`] (`I48F16`) bits — [`FxBits`]'s rule for
/// the one value wider than `Fx` (the quality pass, 2026-09-13). `Money`
/// has the same 16 fractional bits, so a save written when money was `Fx`
/// (an `i32`, which RON prints as the same plain integer) reads back as the
/// same balance; and `test_layouts.gd`'s `fund` patch, which writes
/// `amount * 65536`, is still exactly right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MoneyBits(pub i64);

impl MoneyBits {
    pub fn of(value: Money) -> Self {
        Self(value.to_bits())
    }

    pub fn value(self) -> Money {
        Money::from_bits(self.0)
    }
}

/// One whole saved world. See the module doc for what is here and what is
/// deliberately derived instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveData {
    pub schema_version: u32,
    /// The absolute tick the world was saved on — every
    /// [`TimedProcess::start_tick`] below is measured against it.
    pub tick_count: u64,
    pub money_bits: MoneyBits,
    /// Everything on the floor, in tile order (the world's own placement
    /// order), so two saves of one world are byte-identical.
    pub placements: Vec<SavedPlacement>,
    /// What the player owns and has **not** placed, by kind — sorted.
    pub owned_equipment: Vec<(String, u32)>,
    /// Bought raw ingredient stock not yet loaded into anything — sorted.
    pub owned_ingredients: Vec<(String, u32)>,
    /// Every **active** contract's clock and counters, in the world's
    /// contract order. Only the *state* — a contract's mix, cadence, price
    /// and strikes are content (`data/contracts.ron`) and are matched by id
    /// on load. Since the contract-board epic this is a SUBSET of the board:
    /// what is not listed here is on the board unsigned, so a loaded world
    /// activates exactly these and nothing else — not the board's starters.
    pub contracts: Vec<SavedContract>,
    /// How many times each offer has been lost, by id (the board's "lost
    /// twice" note). Absent in saves from before the contract-board epic,
    /// which is the same as never.
    #[serde(default)]
    pub contracts_lost: Vec<(String, u32)>,
    /// Lifetime successful contract deliveries
    /// ([`super::economy::Economy::lifetime_deliveries`]) — the counter
    /// every progression gate in `data/unlocks.ron` is keyed off.
    ///
    /// **The counter and not the gates.** Which tiers are open is derived
    /// from this against the *current* data file every time it is asked, so
    /// a balance pass on `unlocks.ron` re-reads correctly for every existing
    /// save; storing "bbq_sauce is unlocked" would freeze a save against the
    /// numbers in that file on the day it was written.
    ///
    /// `#[serde(default)]` under the same rule as `auto_fill` and
    /// `SavedMachine::recipe` — see the module doc for what a pre-gate save
    /// therefore loads as, and why that is the right call.
    #[serde(default)]
    pub lifetime_deliveries: u32,
    /// **Lifetime boxes sold, per item id**
    /// ([`super::economy::Economy::lifetime_boxes_sold`]) — what every
    /// progression gate in `data/unlocks.ron` is keyed off since 2026-09-26.
    ///
    /// The ledger and not the gates, for `lifetime_deliveries`' reason: which
    /// tiers are open is derived from this against the *current* data file
    /// every time it is asked, so a balance pass on `unlocks.ron` — or a
    /// change of which product a tier waits on — re-reads correctly for every
    /// existing save.
    ///
    /// Sorted by item id, because it is a `BTreeMap` and the save text is
    /// part of the determinism contract.
    ///
    /// No `#[serde(default)]`: this field is why the format is at version 5,
    /// and a file that does not carry it is refused rather than read as a
    /// factory that has sold nothing — see "Version 5: a gate counts boxes,
    /// not delivery windows" in the module doc.
    ///
    /// **The guard is [`SAVE_SCHEMA_VERSION`], not the missing attribute.**
    /// A version-4 file is refused by number before serde ever looks at its
    /// fields, so putting `#[serde(default)]` back here breaks nothing and no
    /// test goes red for it (a reviewer tried, 2026-09-27). It stays off
    /// anyway: a default here would mean "this save sold nothing", and gate
    /// state is derived from this map on every load, so the day someone bumps
    /// the version for an unrelated reason the attribute would quietly start
    /// re-locking earned tiers again.
    pub lifetime_boxes_sold: BTreeMap<String, u32>,
    /// The production ledger (`sim::stats::ProductionLedger`), rings
    /// verbatim, so a loaded factory's P screen reads exactly what the
    /// saved one did. `#[serde(default)]` for the same reason as
    /// `SavedMachine::load`: an older save loads with an empty ledger,
    /// which is what a factory nothing has been counted for reads as.
    #[serde(default)]
    pub production: SavedProduction,
    /// The staff pools (the staff pools MVP epic). `#[serde(default)]`
    /// under the same rule as `auto_fill`: a pre-epic save has no pools and
    /// loads with the starting crew and a payday one period from its tick.
    /// The per-tick tallies (`demand`, `assigned`) are not here — the first
    /// tick after a load recomputes them, like the topology.
    #[serde(default)]
    pub staff: Option<SavedStaff>,
}

/// A [`ProductionLedger`]'s whole state: one clock per window, its age, and
/// every id's rings and totals in id order. Empty `cursor` is the "nothing
/// saved" default and restores as a fresh ledger; anything else must be a
/// state the tick path could have produced (`ProductionLedger::restore`),
/// or the save is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProduction {
    pub cursor: Vec<usize>,
    pub ticks_into_bucket: Vec<u32>,
    pub age: u64,
    /// Paydays the ledger has closed (`ProductionLedger::cycles_closed`).
    pub cycles_closed: u64,
    pub items: Vec<SavedItemStats>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedItemStats {
    pub id: ItemId,
    pub kind: StatKind,
    /// One ring per window, in `STAT_WINDOWS` order, each `STAT_BUCKETS`
    /// long.
    pub produced: Vec<Vec<u32>>,
    pub consumed: Vec<Vec<u32>>,
    pub produced_total: u64,
    pub consumed_total: u64,
    /// The pay cycle's two counts (`ItemStats`).
    pub produced_this_cycle: u32,
    pub produced_last_cycle: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedStaff {
    /// Every category's headcount, keyed by id. A category the map does not
    /// name keeps its starting headcount (`StaffPools::restore`); an id the
    /// catalog does not know is [`SaveError::UnknownStaffCategory`].
    pub headcount: BTreeMap<String, u32>,
    pub next_payday_tick: u64,
    pub last_bill: u32,
    pub walked_out_last_payday: u32,
    /// The quality pass's arrears rule (2026-09-13), all `#[serde(default)]`
    /// so a save from before it loads with nobody in arrears, every head
    /// owed and nobody just departed — the state a paid-up crew is in.
    /// Heads in arrears per category (hired, unpaid last payday, not
    /// working; gone if unpaid again). A category the map does not name
    /// has none.
    #[serde(default)]
    pub unpaid: BTreeMap<String, u32>,
    /// Heads the next payday bills per category — everyone employed for
    /// any part of the period. A category the map does not name is owed
    /// exactly its headcount. Below the headcount is a corrupt save.
    #[serde(default)]
    pub owed: BTreeMap<String, u32>,
    /// Who left at the last payday, per category.
    #[serde(default)]
    pub left_last_payday: BTreeMap<String, u32>,
    /// How many hired heads the last payday's money did not reach.
    #[serde(default)]
    pub unpaid_last_payday: u32,
}

/// One placed thing: what it is, where it stands, which way it faces, and
/// whatever it is holding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPlacement {
    pub kind: String,
    pub tile: TileCoord,
    pub direction: Direction,
    pub body: SavedBody,
}

/// The live contents of one placement, one variant per body a `Place` can
/// construct (`world.rs`'s `apply_place`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedBody {
    Belt(SavedBelt),
    Splitter(SavedSplitter),
    Pipe(SavedPipe),
    Machine(SavedMachine),
    Hopper(SavedHopper),
    FluidTank(SavedTank),
    Dock(SavedDock),
    /// A warehouse stock rack, and the one choice the player made about
    /// it — see [`SavedRack`].
    ///
    /// A rack holds no stock of its own: the bays it supplies are a
    /// property of its kind in `data/storage.ron`, and the stock standing
    /// in them is rows of `owned_ingredients`, which this file already
    /// saves. Which bay draws which ingredient is derived on read (see
    /// [`super::storage`]), so none of that is persisted and none of it
    /// could load back inconsistent with the ledger.
    ///
    /// It was an empty marker until version 6; the filter is what gave it a
    /// payload, and is why the version moved.
    WarehouseRack(SavedRack),
    /// A belt and a pipe crossing on one tile (`world.rs`'s
    /// `BodyKind::Crossing`): a belt tile's contents and a pipe tile's
    /// volume, because the one entity is both.
    ///
    /// A new variant, not a changed record, so it stays version 7: no
    /// file written before crossings existed can contain one, and every
    /// other variant's shape is untouched, so a version 7 file still means
    /// exactly one thing. The field-addition rule above is about a record
    /// two builds would read differently; nothing here is read differently.
    Crossing(SavedCrossing),
}

impl SavedBody {
    /// What this variant is, for an error message about a record whose body
    /// doesn't match its kind.
    fn label(&self) -> &'static str {
        match self {
            SavedBody::Belt(_) => "belt",
            SavedBody::Splitter(_) => "splitter",
            SavedBody::Pipe(_) => "pipe",
            SavedBody::Machine(_) => "machine",
            SavedBody::Hopper(_) => "hopper",
            SavedBody::FluidTank(_) => "fluid_tank",
            SavedBody::Dock(_) => "dock",
            SavedBody::WarehouseRack(_) => "warehouse_rack",
            SavedBody::Crossing(_) => "crossing",
        }
    }
}

/// A warehouse rack's own state: which raw ingredient it is reserved for
/// (`sim::storage::WarehouseRack::filter`), or `None` for a rack any raw
/// may stand in. No `#[serde(default)]`: the field is why the format is at
/// version 6, and the version is the guard (see the module doc).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedRack {
    pub filter: Option<ItemId>,
}

/// A belt tile's contents: each item and how far along the tile it is, front
/// to back, plus the scan boundary — see
/// [`BeltSegment::restore`](super::belt::BeltSegment) for why the boundary is
/// state and not a cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedBelt {
    pub items: Vec<(ItemId, FxBits)>,
    pub frontier: u32,
    /// The tile's surface clock (`BeltSegment::moved_ticks`) — the tick this
    /// belt last ran on, which is the current tick unless it is JAMMED
    /// (amended 2026-09-25; it used to be a count of the ticks it advanced
    /// product on). Presentation state, saved so a loaded factory's rubber
    /// stands where the saved one's did rather than snapping to a fresh
    /// phase under the same items. `#[serde(default)]` and added to
    /// `schema_version` 2 rather than bumping it, like
    /// `SavedHopper::auto_fill` and `SavedMachine::load`.
    ///
    /// **Only a belt with items on it is restored from this.** An empty one
    /// takes the save's own `tick_count` instead, because an empty belt is a
    /// running belt and a save written under the old rule holds the tick
    /// each idle tile last *carried* something — stale by construction, and
    /// never corrected afterwards, since an idle tile dirties no chunk. See
    /// `BeltSegment::restore`.
    #[serde(default)]
    pub moved_ticks: u64,
}

/// A splitter carries a belt tile's worth of product plus the two things a
/// belt has no equivalent of: which outputs it deals to, and which is due
/// next.
///
/// No `#[serde(default)]` on `mode`: the field is why the format is at
/// version 7, and the version is the guard (see the module doc).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSplitter {
    pub belt: SavedBelt,
    pub mode: SplitMode,
    pub next_output: Side,
}

/// A pipe tile holds only a volume. Which network it belongs to, what that
/// network carries and whether a tank discharges into it are all derived on
/// load (this module's doc).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPipe {
    pub volume_bits: FxBits,
}

/// A crossing's two halves. The placement's `direction` is the belt's; the
/// pipe's is derived from it on load, as it is in play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedCrossing {
    pub belt: SavedBelt,
    pub pipe: SavedPipe,
}

/// A machine's two buffers, its in-flight craft, and which recipe that
/// craft is. The recipe was *not* saved until the BBQ epic, because a kind
/// named exactly one and re-deriving it was free; a kind may now name
/// several (`RecipeRegistry::for_machine`), the pick is made from what the
/// machine holds when a craft starts, and it is frozen for that craft's
/// length — so it is live state, and a save mid-craft has to carry it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedMachine {
    pub input: Vec<(ItemId, u32)>,
    pub output: Vec<(ItemId, u32)>,
    /// `(start_tick, required_ticks)` of the batch currently running, if
    /// any — absolute ticks, so it survives against the restored
    /// [`SaveData::tick_count`] exactly.
    pub process: Option<(u64, u64)>,
    /// The machine's load window (`sim::machine::LoadWindow`), stored as
    /// its raw ring so a loaded window is identical to the saved one and
    /// the lockstep test's chunk snapshots — which now carry `load` —
    /// stay byte-equal across a round trip. `#[serde(default)]` because it
    /// was added to `schema_version` 1 rather than bumping it, like
    /// `SavedHopper::auto_fill`: a save without it loads with a fresh,
    /// empty window, which is what a machine that has observed nothing
    /// reads as anyway.
    #[serde(default)]
    pub load: SavedLoad,
    /// `Machine::refused_by` — the category that refused this machine its
    /// crew on the saved tick, if any. Saved because it shows in the
    /// machine's snapshot as the `staff` stall, which the lockstep test
    /// compares before the first post-load tick. `#[serde(default)]`.
    #[serde(default)]
    pub refused_by: Option<String>,
    /// `Machine::refusal_hold`, for the same reason as `refused_by`: it
    /// decides `MachineLoadEntry::understaffed`. Part of `schema_version` 4
    /// (see the module doc); `#[serde(default)]` like every other field of
    /// this record, so a machine record written without it reads as no
    /// recent refusal.
    #[serde(default)]
    pub refusal_hold: u32,
    /// Which of its kind's recipes this machine is running
    /// (`sim::machine::Machine::recipe`). `#[serde(default)]` and added to
    /// `schema_version` 1 rather than bumping it, like
    /// `SavedHopper::auto_fill` and `load` above: a save without it loads on
    /// the kind's first recipe in file order and re-picks on its next idle
    /// tick, which is exactly what a freshly placed machine does.
    ///
    /// A save written mid-craft is the case that needs it. `process` says a
    /// craft is 30 ticks from done; this says *which* craft, and without it
    /// a two-recipe kind could finish the reload making something the
    /// player never started.
    #[serde(default)]
    pub recipe: Option<RecipeId>,
    /// Whether the player had switched this machine OFF
    /// (`sim::machine::Machine::mothballed`, issue #19's escape hatch from
    /// the unpaid spiral). A mothballed machine is out of the establishment,
    /// so this is a payroll fact as much as a machine one: a factory
    /// reopened from a save must still be off where the player switched it
    /// off, or the load hands them back a wage bill they had deliberately
    /// cut.
    ///
    /// `#[serde(default)]`, but **not** under the closed field-addition
    /// window: it is part of what makes the format version 2, alongside
    /// `SavedTank::fluid` (see the module doc). The default still earns its
    /// keep between two version-2 builds in this sweep — a file written
    /// before this field existed loads with every machine in service, which
    /// is what it was written with — and a v1 file never reaches it, having
    /// already been refused.
    #[serde(default)]
    pub mothballed: bool,
}

/// A [`LoadWindow`](super::machine::LoadWindow)'s ring, verbatim: one
/// `(working, starved, blocked)` triple per bucket in storage order, and
/// the cursor. Empty `buckets` is the "no window saved" default and
/// restores as a fresh window; a non-empty ring that is not exactly
/// `LOAD_BUCKETS` long, or whose cursor is outside it, or with a bucket
/// over `LOAD_BUCKET_TICKS`, is refused as corrupt.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedLoad {
    pub buckets: Vec<(u32, u32, u32)>,
    pub cursor: usize,
}

/// A hopper's chosen contents and how much of them it holds. `item` is what
/// the placement is made *with* (a hopper holds one item for its whole
/// life), not something set afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedHopper {
    pub item: ItemId,
    pub quantity: u32,
    /// Whether the player has switched this hopper to automatic top-up
    /// (`sim::hopper::Hopper::auto_fill`). `#[serde(default)]` because it was
    /// added to `schema_version` 1 rather than bumping it — see this module's
    /// "Forward compatibility" note.
    #[serde(default)]
    pub auto_fill: bool,
}

/// A tank's exact volume. Fractional in general — a discharging tank holds a
/// volume no whole-unit `fill` could produce — hence bits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTank {
    pub volume_bits: FxBits,
    /// `SavedHopper::auto_fill`'s twin (`sim::fluid::FluidTank::auto_fill`).
    #[serde(default)]
    pub auto_fill: bool,
    /// What the player declared this tank holds, or `None` if they never
    /// did — `sim::fluid::FluidTank::declared`, issue #43. **The field that
    /// took this format to `schema_version` 2** (see the module doc).
    ///
    /// It has to be saved because it is the one fluid fact that is *said*
    /// rather than derived: everything else about a network — its shape,
    /// what it carries, which tile a tank discharges into — is rebuilt by
    /// `World::recompute_topology` from the placements, and a declaration
    /// is an input to that pass, not an output of it. Dropping it would
    /// reload the two-tank layout it exists for as two unnamed runs and a
    /// stalled machine.
    ///
    /// Still `Option`, and still defaulted, because *undeclared* is a real
    /// and ordinary state — every tank in a one-fluid line is in it — and
    /// not a missing value.
    #[serde(default)]
    pub fluid: Option<ItemId>,
}

/// What stands on a dock, in its own slot order (see [`Inventory::slots`]),
/// its lifetime shipped total and the tick it last paid on. `Dock::blocked`
/// is derived per tick and not saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedDock {
    pub contents: Vec<(ItemId, u32)>,
    pub shipped_total: u32,
    pub last_paid_tick: Option<u64>,
    /// `Dock::understaffed`, for the same reason as the machine's
    /// `refused_by`. `#[serde(default)]`.
    #[serde(default)]
    pub understaffed: bool,
    /// `Dock::mothballed` — the player's switch, same shape and same
    /// `#[serde(default)]` reason as `SavedMachine::mothballed`.
    #[serde(default)]
    pub mothballed: bool,
}

/// One contract's live state, matched to content by `id` on load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedContract {
    pub id: String,
    pub next_delivery_tick: u64,
    pub deliveries_made: u32,
    pub deliveries_missed: u32,
    /// Missed windows in a row (the strike count). Defaulted for saves from
    /// before the contract-board epic — `schema_version` stays 1 with a
    /// `#[serde(default)]`, the auto-fill flag's precedent, because an old
    /// save's every other field means exactly what it did.
    #[serde(default)]
    pub consecutive_misses: u32,
}

/// Why a save could not be loaded. Every variant names the thing that is
/// wrong and where, in the style every other loader in this crate uses —
/// and every one of them leaves the caller's existing world untouched,
/// because a load builds a *new* world and only hands it back on success.
#[derive(Debug)]
pub enum SaveError {
    Parse(ron::error::SpannedError),
    /// The save names a staff category `data/staff.ron` does not define.
    UnknownStaffCategory {
        id: String,
    },
    /// The save's pools are in a state the tick path could never have
    /// produced (more heads in arrears than hired, fewer owed than hired).
    BadStaff {
        category: String,
        reason: String,
    },
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// A placed kind this build has no idea how to construct — `data/shop.ron`
    /// does not price it, or `data/machines.ron` gives it no footprint.
    /// Content changed under the save.
    UnknownKind {
        kind: String,
        tile: TileCoord,
    },
    /// A record whose body is not the body its kind builds — a `"belt"`
    /// carrying a machine, say. Only a hand-edited or corrupted file gets
    /// here; [`World::to_save`] cannot produce one.
    BodyMismatch {
        kind: String,
        tile: TileCoord,
        body: &'static str,
    },
    /// Two placements claim the same tile — either the same origin twice, or
    /// two footprints overlapping.
    OverlappingPlacements {
        tile: TileCoord,
        first: String,
        second: String,
    },
    /// `Place` refused for a reason the checks above didn't catch. The one
    /// that reaches here in practice is a machine kind whose recipe this
    /// build no longer has.
    PlacementRefused {
        kind: String,
        tile: TileCoord,
    },
    /// An item id no recipe in this build mentions — a save written before a
    /// content change that removed it.
    UnknownItem {
        item: String,
        where_: String,
    },
    /// A hopper naming something that is not a raw ingredient of this
    /// build's recipes, so no hopper could legally hold it.
    UnknownHopperItem {
        item: String,
        tile: TileCoord,
        raw: Vec<String>,
    },
    /// A tank **declared** to hold something that is not a raw fluid of
    /// this build's recipes — `UnknownHopperItem`'s twin, and refused for
    /// its reason. `PlayerCommand::SetTankFluid` would not have accepted
    /// it either.
    UnknownTankFluid {
        fluid: String,
        tile: TileCoord,
        raw: Vec<String>,
    },
    /// A rack reserved for something that is not a raw ingredient of this
    /// build's recipes, solid or fluid — `UnknownHopperItem`'s twin, and
    /// refused for its reason. `PlayerCommand::SetRackFilter` would not
    /// have accepted it either.
    UnknownRackFilter {
        item: String,
        tile: TileCoord,
        raw: Vec<String>,
    },
    /// A machine whose in-flight craft names a recipe its *kind* cannot
    /// run — `data/recipes.ron` changed under this save, either dropping
    /// the recipe or moving it to another machine.
    UnknownMachineRecipe {
        recipe: String,
        kind: String,
        tile: TileCoord,
        known: Vec<String>,
    },
    /// A buildable kind in the owned-equipment ledger that `data/shop.ron`
    /// does not price.
    UnknownEquipmentKind {
        kind: String,
    },
    /// An ingredient in the owned-stock ledger that `data/ingredients.ron`
    /// does not price.
    UnknownIngredient {
        id: String,
    },
    /// The owned-stock ledger holds more than the saved racks have room
    /// for. The cross-file invariant between `owned_ingredients` and the
    /// `warehouse_rack` placements, and the one thing about storage a save
    /// *can* get wrong — `data/storage.ron` changing under a save, or a
    /// hand-edited ledger.
    ///
    /// Reported rather than clamped, for this module's standing reason: a
    /// loader that quietly drops stock the player paid for is worse than
    /// one that says the save and the content files no longer agree.
    StockOverCapacity {
        id: String,
        held: u32,
        bays: u32,
    },
    /// An active contract id this build's `data/contracts.ron` does not
    /// offer. Only this direction is an error: since the contract-board
    /// epic the active list is a subset of the board, so a board offer the
    /// save does not name is simply unsigned — a new offer added to the
    /// board is content the player has not bought yet, not a changed
    /// board. (Before the epic both directions were refused, because every
    /// board entry was always active.)
    UnknownContract {
        id: String,
    },
    /// The same active contract listed twice. One signature per offer is
    /// the rule the economy enforces, so a save that breaks it is corrupt.
    DuplicateContract {
        id: String,
    },
    /// A buffer, bin, tank or pipe holding more than it can. A hand-edited
    /// file, or content whose capacity constant shrank.
    OverCapacity {
        what: String,
        tile: TileCoord,
        held: String,
        capacity: String,
    },
    /// A machine's saved load ring is not one `LoadWindow` could have
    /// written: wrong length, cursor outside it, or an over-full bucket.
    BadLoadWindow {
        tile: TileCoord,
        buckets: usize,
        cursor: usize,
    },
    /// The saved production ledger is not one `ProductionLedger` could
    /// have written: a wrong number of windows or buckets, a cursor outside
    /// its ring, a clock at or past its bucket, or an id listed twice.
    BadProductionLedger {
        detail: String,
    },
}

/// The one-sentence reason a *named* older version stopped loading, appended
/// to [`SaveError::UnsupportedSchemaVersion`] so the refusal tells a player
/// what changed rather than only that a number differs.
///
/// A version this build has no story for adds nothing — the bare number
/// mismatch is already the whole truth for a file from the future, or from a
/// development build whose change nobody has to explain any more.
fn why_the_version_moved(found: u32) -> &'static str {
    match found {
        6 => {
            ". Version 7 lets a splitter split left, right, both ways or three ways, \
             which changed how a splitter is written. Nothing is lost: \
             scripts/migrate-save-v6-to-v7.py rewrites a version 6 save with every \
             splitter splitting left, which is what every splitter in it did"
        }
        5 => {
            ". Version 6 lets a warehouse rack be reserved for one ingredient, which \
             changed how a rack is written. Nothing is lost: \
             scripts/migrate-save-v5-to-v6.py rewrites a version 5 save with every rack \
             open to every ingredient, which is what every rack in it was"
        }
        4 => {
            ". Version 5 changed what unlocks a sauce: a tier used to open after a \
             number of delivery windows, and now opens after a number of boxes of a \
             named product sold. A version 4 save never recorded which products its \
             deliveries contained, so this build cannot tell what it had sold, and it \
             will not guess — start a new factory"
        }
        _ => "",
    }
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Parse(e) => write!(f, "failed to parse the save file: {e}"),
            SaveError::UnknownStaffCategory { id } => write!(
                f,
                "the save names staff category '{id}', which data/staff.ron does not define"
            ),
            SaveError::BadStaff { category, reason } => write!(
                f,
                "the save's staff pools are inconsistent for '{category}': {reason}"
            ),
            SaveError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "save schema_version {found} is not supported by this build (supports \
                 {supported}) — this save was written by a different version of the \
                 game{}",
                why_the_version_moved(*found)
            ),
            SaveError::UnknownKind { kind, tile } => write!(
                f,
                "the save places '{kind}' at {tile:?}, which this build cannot construct — \
                 data/shop.ron does not price it or data/machines.ron gives it no footprint"
            ),
            SaveError::BodyMismatch { kind, tile, body } => write!(
                f,
                "the save places '{kind}' at {tile:?} carrying a {body} body, which is not \
                 what that kind is built with"
            ),
            SaveError::OverlappingPlacements {
                tile,
                first,
                second,
            } => write!(
                f,
                "the save has two placements claiming tile {tile:?} ('{first}' and \
                 '{second}') — no world can be in that state"
            ),
            SaveError::PlacementRefused { kind, tile } => write!(
                f,
                "the save places '{kind}' at {tile:?} and this build refused it — most \
                 likely no recipe runs on that machine kind any more"
            ),
            SaveError::UnknownItem { item, where_ } => write!(
                f,
                "the save holds item '{item}' ({where_}), which no recipe in this build \
                 mentions — data/recipes.ron changed under this save"
            ),
            SaveError::UnknownHopperItem { item, tile, raw } => write!(
                f,
                "the hopper at {tile:?} holds '{item}', which is not a raw ingredient of \
                 this build's recipes (raw: {}) — a hopper can only ever hold one of those",
                raw.join(", ")
            ),
            SaveError::UnknownTankFluid { fluid, tile, raw } => write!(
                f,
                "the tank at {tile:?} is declared to hold '{fluid}', which is not a raw fluid \
                 of this build's recipes (raw: {}) — a tank is how fluid enters the graph from \
                 outside, and only raws come in",
                raw.join(", ")
            ),
            SaveError::UnknownRackFilter { item, tile, raw } => write!(
                f,
                "the warehouse rack at {tile:?} is reserved for '{item}', which is not a raw \
                 ingredient of this build's recipes (raw: {}) — only raws are held in the \
                 warehouse",
                raw.join(", ")
            ),
            SaveError::UnknownMachineRecipe {
                recipe,
                kind,
                tile,
                known,
            } => write!(
                f,
                "the {kind} at {tile:?} is running recipe '{recipe}', which this build's \
                 data/recipes.ron does not give that kind (it runs: {}) — the recipe file \
                 changed under this save",
                known.join(", ")
            ),
            SaveError::UnknownEquipmentKind { kind } => write!(
                f,
                "the save says the player owns '{kind}', which data/shop.ron does not price"
            ),
            SaveError::UnknownIngredient { id } => write!(
                f,
                "the save says the player owns '{id}', which data/ingredients.ron does not \
                 price"
            ),
            SaveError::StockOverCapacity { id, held, bays } => write!(
                f,
                "the save holds {held} of '{id}', which will not fit the {bays} storage \
                 bay(s) its warehouse racks supply — data/storage.ron changed under this \
                 save, or the ledger was edited by hand"
            ),
            SaveError::UnknownContract { id } => write!(
                f,
                "the save carries state for contract '{id}', which data/contracts.ron does \
                 not define in this build"
            ),
            SaveError::DuplicateContract { id } => write!(
                f,
                "the save lists contract '{id}' as active twice — a contract can be signed \
                 once at a time"
            ),
            SaveError::OverCapacity {
                what,
                tile,
                held,
                capacity,
            } => write!(
                f,
                "the {what} at {tile:?} holds {held}, more than its capacity of {capacity}"
            ),
            SaveError::BadLoadWindow {
                tile,
                buckets,
                cursor,
            } => write!(
                f,
                "the machine at {tile:?} has a load window of {buckets} bucket(s) with cursor \
                 {cursor}, which is not a ring of {LOAD_BUCKETS}"
            ),
            SaveError::BadProductionLedger { detail } => {
                write!(f, "the production ledger is corrupt: {detail}")
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl SaveData {
    /// The save as RON text — pretty-printed, because a save file in this
    /// project should be as readable and as diff-able as `data/*.ron` beside
    /// it.
    ///
    /// Deterministic: every list in [`SaveData`] is in a fixed order (tile
    /// order for placements, sorted for the two ledgers, slot order for the
    /// inventories), so two saves of one world are byte-identical.
    pub fn to_ron(&self) -> String {
        let config = ron::ser::PrettyConfig::new()
            .depth_limit(2)
            .indentor("  ".to_string());
        ron::ser::to_string_pretty(self, config)
            .expect("SaveData is plain data and must always serialize")
    }

    /// Parses RON text, checking `schema_version` **first** so a file from a
    /// different build reports that rather than whatever the shape change
    /// happens to break.
    pub fn from_ron(text: &str) -> Result<SaveData, SaveError> {
        #[derive(Deserialize)]
        struct Header {
            schema_version: u32,
        }
        // A version mismatch is far more likely than a corrupt file, and its
        // message is far more useful, so a file that does not parse is asked
        // again against a shape that only has the one field in it.
        //
        // Only once the real parse has failed: reading that shape skips every
        // other field through `deserialize_any`, and ron 0.12.2 scans the
        // whole rest of the text for `..` on each number it skips that way
        // (`Parser::next_bytes_is_float`), so on a good file it made every
        // load quadratic: 3 560 placements took 3.8 s with it, 23 ms without.
        let data: SaveData = match ron::from_str(text) {
            Ok(data) => data,
            Err(error) => {
                if let Ok(header) = ron::from_str::<Header>(text)
                    && header.schema_version != SAVE_SCHEMA_VERSION
                {
                    return Err(SaveError::UnsupportedSchemaVersion {
                        found: header.schema_version,
                        supported: SAVE_SCHEMA_VERSION,
                    });
                }
                return Err(SaveError::Parse(error));
            }
        };
        if data.schema_version != SAVE_SCHEMA_VERSION {
            return Err(SaveError::UnsupportedSchemaVersion {
                found: data.schema_version,
                supported: SAVE_SCHEMA_VERSION,
            });
        }
        Ok(data)
    }
}

// =====================================================================
// Saving
// =====================================================================

impl World {
    /// Everything about this world a load has to put back — see the module
    /// doc for the "running-state-identical" bar this is written against.
    pub fn to_save(&self) -> SaveData {
        let mut placements = Vec::new();
        for (tile, placement, entity) in self.saved_placements() {
            let kind = placement.kind.as_str();
            let body = match kind {
                BELT_KIND => SavedBody::Belt(saved_belt(self, entity)),
                SPLITTER_KIND => {
                    let splitter = *self
                        .ecs()
                        .get::<&Splitter>(entity)
                        .expect("a splitter placement carries a Splitter");
                    SavedBody::Splitter(SavedSplitter {
                        belt: saved_belt(self, entity),
                        mode: splitter.mode(),
                        next_output: splitter.due(),
                    })
                }
                PIPE_KIND => SavedBody::Pipe(SavedPipe {
                    volume_bits: FxBits::of(
                        self.pipe_volume_at(tile)
                            .expect("a pipe placement is always in the pipe index")
                            .0,
                    ),
                }),
                HOPPER_KIND => {
                    let hopper = self
                        .ecs()
                        .get::<&Hopper>(entity)
                        .expect("a hopper placement carries a Hopper");
                    SavedBody::Hopper(SavedHopper {
                        item: hopper.item().clone(),
                        quantity: hopper.quantity(),
                        auto_fill: hopper.auto_fill(),
                    })
                }
                FLUID_TANK_KIND => {
                    let tank = self
                        .ecs()
                        .get::<&FluidTank>(entity)
                        .expect("a tank placement carries a FluidTank");
                    SavedBody::FluidTank(SavedTank {
                        volume_bits: FxBits::of(tank.volume()),
                        auto_fill: tank.auto_fill(),
                        fluid: tank.declared().cloned(),
                    })
                }
                DOCK_KIND => {
                    let dock = self
                        .ecs()
                        .get::<&Dock>(entity)
                        .expect("a dock placement carries a Dock");
                    SavedBody::Dock(SavedDock {
                        contents: dock.slots().to_vec(),
                        shipped_total: dock.shipped_total(),
                        last_paid_tick: dock.last_paid_tick(),
                        understaffed: dock.understaffed(),
                        mothballed: dock.mothballed(),
                    })
                }
                WAREHOUSE_RACK_KIND => {
                    let rack = self
                        .ecs()
                        .get::<&WarehouseRack>(entity)
                        .expect("a rack placement carries a WarehouseRack");
                    SavedBody::WarehouseRack(SavedRack {
                        filter: rack.filter.clone(),
                    })
                }
                CROSSING_KIND => SavedBody::Crossing(SavedCrossing {
                    belt: saved_belt(self, entity),
                    pipe: SavedPipe {
                        volume_bits: FxBits::of(
                            self.pipe_volume_at(tile)
                                .expect("a crossing is always in the pipe index")
                                .0,
                        ),
                    },
                }),
                _ => {
                    let machine = self
                        .ecs()
                        .get::<&Machine>(entity)
                        .expect("a machine placement carries a Machine");
                    SavedBody::Machine(SavedMachine {
                        input: machine.input.slots().to_vec(),
                        output: machine.output.slots().to_vec(),
                        process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
                        load: SavedLoad {
                            buckets: machine
                                .load
                                .buckets()
                                .iter()
                                .map(|b| (b.working, b.starved, b.blocked))
                                .collect(),
                            cursor: machine.load.cursor(),
                        },
                        refused_by: machine.refused_by.clone(),
                        refusal_hold: machine.refusal_hold,
                        recipe: Some(machine.recipe.clone()),
                        mothballed: machine.mothballed,
                    })
                }
            };
            placements.push(SavedPlacement {
                kind: placement.kind.clone(),
                tile,
                direction: placement.direction,
                body,
            });
        }

        SaveData {
            schema_version: SAVE_SCHEMA_VERSION,
            tick_count: self.current_tick(),
            money_bits: MoneyBits::of(self.money()),
            placements,
            owned_equipment: self
                .shop()
                .owned
                .iter()
                .map(|(kind, count)| (kind.to_string(), count))
                .collect(),
            owned_ingredients: self
                .ingredients()
                .owned
                .iter()
                .map(|(id, count)| (id.to_string(), count))
                .collect(),
            contracts: self
                .economy()
                .contracts
                .iter()
                .map(|active| SavedContract {
                    id: active.contract.id.clone(),
                    next_delivery_tick: active.next_delivery_tick,
                    deliveries_made: active.deliveries_made,
                    deliveries_missed: active.deliveries_missed,
                    consecutive_misses: active.consecutive_misses,
                })
                .collect(),
            contracts_lost: self
                .economy()
                .losses
                .iter()
                .map(|(id, count)| (id.clone(), *count))
                .collect(),
            lifetime_deliveries: self.economy().lifetime_deliveries,
            lifetime_boxes_sold: self.economy().lifetime_boxes_sold.clone(),
            production: saved_production(self.production()),
            staff: Some(SavedStaff {
                headcount: self.staff().headcounts().into_iter().collect(),
                next_payday_tick: self.staff().next_payday_tick(),
                last_bill: self.staff().last_bill(),
                walked_out_last_payday: self.staff().walked_out_last_payday(),
                unpaid: self.staff().unpaid_counts().into_iter().collect(),
                owed: self.staff().owed_counts().into_iter().collect(),
                left_last_payday: self.staff().left_counts().into_iter().collect(),
                unpaid_last_payday: self.staff().unpaid_last_payday(),
            }),
        }
    }

    /// [`World::to_save`] straight to RON text — the whole save side of the
    /// `SimBridge` boundary in one call.
    pub fn to_save_ron(&self) -> String {
        self.to_save().to_ron()
    }
}

fn saved_belt(world: &World, entity: hecs::Entity) -> SavedBelt {
    let belt = world
        .ecs()
        .get::<&BeltSegment>(entity)
        .expect("a belt or splitter placement carries a BeltSegment");
    SavedBelt {
        items: belt
            .snapshot_positions()
            .into_iter()
            .map(|(item, position)| (item, FxBits::of(position)))
            .collect(),
        frontier: belt.frontier() as u32,
        moved_ticks: belt.moved_ticks(),
    }
}

// =====================================================================
// Loading
// =====================================================================

impl World {
    /// Rebuilds a world from a save, against the bundled content
    /// (`data/machines.ron`, `data/contracts.ron`) and the recipes passed in
    /// — the same content seams [`World::new`] has, for the same reason.
    ///
    /// See the module doc for the two-halves mechanism and for what makes a
    /// save corrupt. Nothing partial is ever returned: this builds a new
    /// world and hands it back only if every step succeeded, so a caller's
    /// existing world is untouched by a failed load.
    pub fn from_save(data: SaveData, recipes: RecipeRegistry) -> Result<World, SaveError> {
        World::from_save_with_content(
            data,
            recipes,
            MachineRegistry::bundled(),
            ContractRegistry::bundled(),
        )
    }

    /// [`World::from_save`] with explicit machine and contract content — the
    /// seam [`World::with_content`] already provides, carried through so a
    /// test can load a save against content the shipped files don't have.
    pub fn from_save_with_content(
        data: SaveData,
        recipes: RecipeRegistry,
        machines: MachineRegistry,
        contracts: ContractRegistry,
    ) -> Result<World, SaveError> {
        if data.schema_version != SAVE_SCHEMA_VERSION {
            return Err(SaveError::UnsupportedSchemaVersion {
                found: data.schema_version,
                supported: SAVE_SCHEMA_VERSION,
            });
        }

        // An empty world first, so every validation below can be asked of
        // the real content registries rather than of a second copy of the
        // rules. Its starting money is irrelevant and is overwritten.
        let mut world = World::with_content_and_money(recipes, machines, contracts, Money::ZERO);

        validate(&world, &data)?;
        rebuild_structure(&mut world, &data)?;
        restore_bodies(&mut world, &data)?;
        // A tank's declared fluid only just arrived, and pipe naming is
        // derived from it — see `World::recompute_topology_after_restore`.
        world.recompute_topology_after_restore();
        restore_ledgers(&mut world, &data);
        world.restore_production(restore_production(&data.production)?);
        restore_staff(&mut world, &data)?;
        // `restore_staff` builds fresh pools, so the establishment the
        // replayed `Place` commands computed in `rebuild_structure` is gone
        // by here. Sum it again, or every loaded world reads as needing
        // nobody and the hiring advice is silently wrong after a load.
        world.recompute_establishment();

        world.restore_tick_count(data.tick_count);
        world.restore_money(data.money_bits.value());
        world.mark_every_chunk_dirty();
        Ok(world)
    }

    /// Parses and loads in one call — the whole load side of the
    /// `SimBridge` boundary.
    pub fn from_save_ron(text: &str, recipes: RecipeRegistry) -> Result<World, SaveError> {
        World::from_save(SaveData::from_ron(text)?, recipes)
    }
}

/// Everything checkable before a single thing is placed: kinds this build
/// can construct, bodies matching their kinds, no two placements on one
/// tile, and every id naming something that still exists.
fn validate(world: &World, data: &SaveData) -> Result<(), SaveError> {
    let known_items = world.recipe_item_ids();

    let mut claimed: std::collections::BTreeMap<TileCoord, String> =
        std::collections::BTreeMap::new();
    for placement in &data.placements {
        let Some(footprint) = world.footprint_of_kind(&placement.kind) else {
            return Err(SaveError::UnknownKind {
                kind: placement.kind.clone(),
                tile: placement.tile,
            });
        };
        let expected = expected_body(&placement.kind);
        if placement.body.label() != expected {
            return Err(SaveError::BodyMismatch {
                kind: placement.kind.clone(),
                tile: placement.tile,
                body: placement.body.label(),
            });
        }
        for tile in footprint.tiles_facing(placement.tile, placement.direction) {
            if let Some(first) = claimed.insert(tile, placement.kind.clone()) {
                return Err(SaveError::OverlappingPlacements {
                    tile,
                    first,
                    second: placement.kind.clone(),
                });
            }
        }

        // Every item id the body carries has to still be content.
        match &placement.body {
            SavedBody::Belt(belt) => check_belt_items(belt, &known_items, placement.tile)?,
            SavedBody::Splitter(splitter) => {
                check_belt_items(&splitter.belt, &known_items, placement.tile)?
            }
            SavedBody::Crossing(crossing) => {
                check_belt_items(&crossing.belt, &known_items, placement.tile)?
            }
            SavedBody::Machine(machine) => {
                for (item, _) in machine.input.iter().chain(machine.output.iter()) {
                    check_item(item, &known_items, || {
                        format!("in the machine buffer at {:?}", placement.tile)
                    })?;
                }
                // The saved pick has to still be one this kind can run —
                // the same list `Machine::choose` picks from, so a restored
                // machine can never be on a recipe the live sim would not
                // have chosen for it.
                if let Some(recipe) = &machine.recipe {
                    let known = world.recipes_for_kind(&placement.kind);
                    if !known.iter().any(|id| id == recipe) {
                        return Err(SaveError::UnknownMachineRecipe {
                            recipe: recipe.clone(),
                            kind: placement.kind.clone(),
                            tile: placement.tile,
                            known,
                        });
                    }
                }
            }
            SavedBody::Dock(dock) => {
                for (item, _) in &dock.contents {
                    check_item(item, &known_items, || {
                        format!("on the dock at {:?}", placement.tile)
                    })?;
                }
            }
            SavedBody::Hopper(hopper) => {
                if !world.raw_items().contains(&hopper.item) {
                    return Err(SaveError::UnknownHopperItem {
                        item: hopper.item.clone(),
                        tile: placement.tile,
                        raw: world.raw_items().to_vec(),
                    });
                }
            }
            SavedBody::FluidTank(tank) => {
                // A declared fluid is validated exactly as a hopper's item
                // is, and rejected for the same reason: content changed
                // under the save, and this crate's loader convention is to
                // say so rather than to guess. An *undeclared* tank is not
                // an error — it is the ordinary state of every tank on a
                // one-fluid line.
                if let Some(fluid) = &tank.fluid
                    && !world.raw_fluids().contains(fluid)
                {
                    return Err(SaveError::UnknownTankFluid {
                        fluid: fluid.clone(),
                        tile: placement.tile,
                        raw: world.raw_fluids().iter().cloned().collect(),
                    });
                }
            }
            SavedBody::WarehouseRack(rack) => {
                if let Some(item) = &rack.filter
                    && !world.raw_items().contains(item)
                    && !world.raw_fluids().contains(item)
                {
                    return Err(SaveError::UnknownRackFilter {
                        item: item.clone(),
                        tile: placement.tile,
                        raw: world
                            .raw_items()
                            .iter()
                            .chain(world.raw_fluids().iter())
                            .cloned()
                            .collect(),
                    });
                }
            }
            SavedBody::Pipe(_) => {}
        }
    }

    for (kind, _) in &data.owned_equipment {
        if world.shop().catalog.get(kind).is_none() {
            return Err(SaveError::UnknownEquipmentKind { kind: kind.clone() });
        }
    }
    for (id, _) in &data.owned_ingredients {
        if world.ingredients().catalog.get(id).is_none() {
            return Err(SaveError::UnknownIngredient { id: id.clone() });
        }
    }
    // The saved stock has to fit the saved racks. `rebuild_structure`
    // replays real `Place` commands but `restore_ledgers` writes the ledger
    // straight in — it has to, since replaying `BuyIngredient` would charge
    // for stock the save already paid for — so this is the only place the
    // storage invariant gets checked on the load path. Counted off
    // `data.placements` rather than the half-built world because validation
    // runs before anything is placed.
    // A filtered rack's bays are counted as its filter's, exactly as
    // `World::bay_pool` counts them on a live floor.
    let mut pool = BayPool::default();
    for placement in &data.placements {
        let filter = match &placement.body {
            SavedBody::WarehouseRack(rack) => rack.filter.as_deref(),
            _ => None,
        };
        pool.add_rack(world.storage().bays_of_kind(&placement.kind), filter);
    }
    let bays = pool.total();
    let mut owned = super::ingredients::OwnedIngredients::new();
    for (id, count) in &data.owned_ingredients {
        owned.add(id, *count);
    }
    for (id, count) in &data.owned_ingredients {
        if *count > world.storage().cap_for(id, &owned, &pool) {
            return Err(SaveError::StockOverCapacity {
                id: id.clone(),
                held: *count,
                bays,
            });
        }
    }
    // Every active contract has to be an offer this build's board makes,
    // and none may be active twice — see `SaveError::UnknownContract` for
    // why the other direction (an offer the save does not name) is fine.
    for (i, saved) in data.contracts.iter().enumerate() {
        if world.economy().board().get(&saved.id).is_none() {
            return Err(SaveError::UnknownContract {
                id: saved.id.clone(),
            });
        }
        if data.contracts[..i].iter().any(|c| c.id == saved.id) {
            return Err(SaveError::DuplicateContract {
                id: saved.id.clone(),
            });
        }
    }
    for (id, _) in &data.contracts_lost {
        if world.economy().board().get(id).is_none() {
            return Err(SaveError::UnknownContract { id: id.clone() });
        }
    }
    let staff_catalog = world.staff().catalog();
    if let Some(staff) = &data.staff {
        for id in staff
            .headcount
            .keys()
            .chain(staff.unpaid.keys())
            .chain(staff.owed.keys())
            .chain(staff.left_last_payday.keys())
        {
            if staff_catalog.get(id).is_none() {
                return Err(SaveError::UnknownStaffCategory { id: id.clone() });
            }
        }
    }
    for placement in &data.placements {
        if let SavedBody::Machine(machine) = &placement.body
            && let Some(id) = &machine.refused_by
            && staff_catalog.get(id).is_none()
        {
            return Err(SaveError::UnknownStaffCategory { id: id.clone() });
        }
    }
    Ok(())
}

/// The pools: the save's headcounts, arrears, owed counts and payday
/// clock, or — for a pre-epic save — the starting crew with a payday one
/// period from the saved tick.
fn restore_staff(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
    let catalog = world.staff().catalog().clone();
    fn pairs(map: &BTreeMap<String, u32>) -> Vec<(String, u32)> {
        map.iter().map(|(id, count)| (id.clone(), *count)).collect()
    }
    let pools = match &data.staff {
        Some(saved) => StaffPools::restore(
            catalog,
            StaffRestore {
                headcount: &pairs(&saved.headcount),
                unpaid: &pairs(&saved.unpaid),
                owed: &pairs(&saved.owed),
                left_last_payday: &pairs(&saved.left_last_payday),
                next_payday_tick: saved.next_payday_tick,
                last_bill: saved.last_bill,
                unpaid_last_payday: saved.unpaid_last_payday,
                walked_out_last_payday: saved.walked_out_last_payday,
            },
        )
        .map_err(|e| match e {
            StaffRestoreError::UnknownCategory(id) => SaveError::UnknownStaffCategory { id },
            StaffRestoreError::Inconsistent { category, reason } => {
                SaveError::BadStaff { category, reason }
            }
        })?,
        None => {
            let period = catalog.pay_period_ticks();
            StaffPools::restore(
                catalog,
                StaffRestore {
                    headcount: &[],
                    unpaid: &[],
                    owed: &[],
                    left_last_payday: &[],
                    next_payday_tick: data.tick_count + period,
                    last_bill: 0,
                    unpaid_last_payday: 0,
                    walked_out_last_payday: 0,
                },
            )
            .expect("empty lists name no unknown category and break no invariant")
        }
    };
    world.restore_staff(pools);
    Ok(())
}

/// Which body a kind is built with (`world.rs`'s `apply_place`) — the one
/// place this module restates that mapping, so a mismatch is an error rather
/// than a silently wrong body.
fn expected_body(kind: &str) -> &'static str {
    match kind {
        BELT_KIND => "belt",
        SPLITTER_KIND => "splitter",
        PIPE_KIND => "pipe",
        HOPPER_KIND => "hopper",
        FLUID_TANK_KIND => "fluid_tank",
        DOCK_KIND => "dock",
        WAREHOUSE_RACK_KIND => "warehouse_rack",
        CROSSING_KIND => "crossing",
        _ => "machine",
    }
}

fn check_belt_items(belt: &SavedBelt, known: &[String], tile: TileCoord) -> Result<(), SaveError> {
    for (item, _) in &belt.items {
        check_item(item, known, || format!("on the belt at {tile:?}"))?;
    }
    Ok(())
}

fn check_item(
    item: &str,
    known: &[String],
    where_: impl FnOnce() -> String,
) -> Result<(), SaveError> {
    if known.iter().any(|id| id == item) {
        return Ok(());
    }
    Err(SaveError::UnknownItem {
        item: item.to_string(),
        where_: where_(),
    })
}

/// Half one of a load: buy and place every saved thing through the real
/// commands (module doc). Funded one item at a time, so an arbitrarily large
/// factory loads whatever its total bill (which once had to fit in `Fx`).
fn rebuild_structure(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
    world.begin_bulk_place();
    let rebuilt = replay_placements(world, data);
    world.end_bulk_place();
    rebuilt
}

/// One `Purchase` + `Place` per saved placement, in save order — inside
/// [`World::begin_bulk_place`], so each costs its own tiles rather than a
/// pass over the whole floor.
fn replay_placements(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
    for (index, placement) in data.placements.iter().enumerate() {
        if placement.kind == CROSSING_KIND {
            rebuild_crossing(world, placement)?;
            if world.placement_count() != index + 1 {
                return Err(SaveError::PlacementRefused {
                    kind: placement.kind.clone(),
                    tile: placement.tile,
                });
            }
            continue;
        }
        let price = world
            .shop()
            .catalog
            .get(&placement.kind)
            .expect("validate() proved every kind is priced")
            .price_money();
        world.restore_money(price);
        world.issue_command(PlayerCommand::Purchase {
            kind: placement.kind.clone(),
            quantity: 1,
        });
        world.issue_command(PlayerCommand::Place {
            kind: placement.kind.clone(),
            tile: placement.tile,
            direction: placement.direction,
            item: match &placement.body {
                SavedBody::Hopper(hopper) => Some(hopper.item.clone()),
                _ => None,
            },
        });
        if world.placement_count() != index + 1 {
            return Err(SaveError::PlacementRefused {
                kind: placement.kind.clone(),
                tile: placement.tile,
            });
        }
    }
    Ok(())
}

/// A crossing is never bought, so it is rebuilt the way a player builds
/// one: buy and place a belt facing the saved direction, then buy a pipe
/// and place it across it. Both halves go through the real commands, so a
/// crossing a load puts back is one the live `Place` rule would allow.
fn rebuild_crossing(world: &mut World, placement: &SavedPlacement) -> Result<(), SaveError> {
    let halves = [
        (BELT_KIND, placement.direction),
        (PIPE_KIND, crossing_pipe_direction(placement.direction)),
    ];
    for (kind, direction) in halves {
        let price = world
            .shop()
            .catalog
            .get(kind)
            .ok_or_else(|| SaveError::UnknownKind {
                kind: placement.kind.clone(),
                tile: placement.tile,
            })?
            .price_money();
        world.restore_money(price);
        world.issue_command(PlayerCommand::Purchase {
            kind: kind.to_string(),
            quantity: 1,
        });
        world.issue_command(PlayerCommand::Place {
            kind: kind.to_string(),
            tile: placement.tile,
            direction,
            item: None,
        });
    }
    let assembled = world
        .entity_at(placement.tile)
        .is_some_and(|entity| world.kind_of(entity) == CROSSING_KIND);
    if !assembled {
        return Err(SaveError::PlacementRefused {
            kind: placement.kind.clone(),
            tile: placement.tile,
        });
    }
    Ok(())
}

/// Half two: write the live contents onto the placements half one built.
fn restore_bodies(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
    for placement in &data.placements {
        let entity = world
            .entity_at(placement.tile)
            .expect("rebuild_structure placed something on every saved tile");
        match &placement.body {
            SavedBody::Belt(belt) => restore_belt(world, entity, belt, data.tick_count),
            SavedBody::Splitter(splitter) => {
                restore_belt(world, entity, &splitter.belt, data.tick_count);
                world
                    .ecs_mut()
                    .insert_one(
                        entity,
                        Splitter::restore(splitter.mode, splitter.next_output),
                    )
                    .expect("the entity was just placed");
            }
            SavedBody::Pipe(pipe) => restore_pipe(world, placement.tile, pipe)?,
            SavedBody::Crossing(crossing) => {
                restore_belt(world, entity, &crossing.belt, data.tick_count);
                restore_pipe(world, placement.tile, &crossing.pipe)?;
            }
            SavedBody::Machine(saved) => {
                let (input_capacity, output_capacity) = {
                    let machine = world
                        .ecs()
                        .get::<&Machine>(entity)
                        .expect("a machine placement carries a Machine");
                    (
                        machine.input.capacity_per_item,
                        machine.output.capacity_per_item,
                    )
                };
                check_capacity(
                    &saved.input,
                    input_capacity,
                    "machine input",
                    placement.tile,
                )?;
                check_capacity(
                    &saved.output,
                    output_capacity,
                    "machine output",
                    placement.tile,
                )?;
                let load = restore_load(&saved.load, placement.tile)?;
                let mut machine = world
                    .ecs_mut()
                    .get::<&mut Machine>(entity)
                    .expect("a machine placement carries a Machine");
                machine.input = Inventory::restore(input_capacity, saved.input.clone());
                machine.output = Inventory::restore(output_capacity, saved.output.clone());
                machine.process = saved
                    .process
                    .map(|(start_tick, required_ticks)| TimedProcess {
                        start_tick,
                        required_ticks,
                    });
                machine.load = load;
                machine.refused_by = saved.refused_by.clone();
                machine.refusal_hold = saved.refusal_hold;
                // The establishment is recomputed once after every body is
                // restored (see `load`), so setting this here is enough to
                // put a mothballed machine's crew back out of the payroll.
                machine.mothballed = saved.mothballed;
                // `validate` already checked this names one of the kind's
                // recipes; a save without the field keeps the recipe the
                // `Place` above started the machine on.
                if let Some(recipe) = &saved.recipe {
                    machine.recipe = recipe.clone();
                }
            }
            SavedBody::Hopper(saved) => {
                let mut hopper = world
                    .ecs_mut()
                    .get::<&mut Hopper>(entity)
                    .expect("a hopper placement carries a Hopper");
                if saved.quantity > hopper.capacity() {
                    return Err(SaveError::OverCapacity {
                        what: "hopper".to_string(),
                        tile: placement.tile,
                        held: saved.quantity.to_string(),
                        capacity: hopper.capacity().to_string(),
                    });
                }
                hopper.fill(saved.quantity);
                hopper.set_auto_fill(saved.auto_fill);
            }
            SavedBody::FluidTank(saved) => {
                let capacity = world
                    .ecs()
                    .get::<&FluidTank>(entity)
                    .expect("a tank placement carries a FluidTank")
                    .capacity();
                let volume = saved.volume_bits.value();
                if volume > capacity || volume < Fx::ZERO {
                    return Err(SaveError::OverCapacity {
                        what: "fluid tank".to_string(),
                        tile: placement.tile,
                        held: volume.to_string(),
                        capacity: capacity.to_string(),
                    });
                }
                world
                    .ecs_mut()
                    .insert_one(
                        entity,
                        FluidTank::restore(capacity, volume, saved.auto_fill, saved.fluid.clone()),
                    )
                    .expect("the entity was just placed");
            }
            SavedBody::Dock(saved) => {
                let places = world
                    .ecs()
                    .get::<&Dock>(entity)
                    .expect("a dock placement carries a Dock")
                    .places();
                // Places are one total across items, so the sum is what
                // binds; the per-item check would pass a dock holding 32 of
                // two items each.
                let used = saved
                    .contents
                    .iter()
                    .fold(0u32, |sum, (_, qty)| sum.saturating_add(*qty));
                if used > places {
                    return Err(SaveError::OverCapacity {
                        what: "dock".to_string(),
                        tile: placement.tile,
                        held: used.to_string(),
                        capacity: places.to_string(),
                    });
                }
                world
                    .ecs_mut()
                    .insert_one(
                        entity,
                        Dock::restore(
                            places,
                            saved.contents.clone(),
                            saved.shipped_total,
                            saved.last_paid_tick,
                            saved.understaffed,
                            saved.mothballed,
                        ),
                    )
                    .expect("the entity was just placed");
            }
            // Only the filter to write back: `rebuild_structure`'s `Place`
            // spawned the rack open to every raw, and its contents are
            // `owned_ingredients`, restored by `restore_ledgers` further
            // down. Written straight in rather than through
            // `SetRackFilter`, which would refuse a filter whose gate has
            // not opened in the half-restored world (the ledger that opens
            // it is restored after this) and would judge the room against
            // a ledger that is still empty.
            SavedBody::WarehouseRack(saved) => {
                world
                    .ecs_mut()
                    .insert_one(
                        entity,
                        WarehouseRack {
                            filter: saved.filter.clone(),
                        },
                    )
                    .expect("the entity was just placed");
            }
        }
    }
    Ok(())
}

fn restore_pipe(world: &mut World, tile: TileCoord, saved: &SavedPipe) -> Result<(), SaveError> {
    let (_, capacity) = world
        .pipe_volume_at(tile)
        .expect("a placed pipe is always in the pipe index");
    let volume = saved.volume_bits.value();
    if volume > capacity || volume < Fx::ZERO {
        return Err(SaveError::OverCapacity {
            what: "pipe".to_string(),
            tile,
            held: volume.to_string(),
            capacity: capacity.to_string(),
        });
    }
    world.restore_pipe_volume(tile, volume);
    Ok(())
}

/// `tick_count` is the save's own tick and not `world.current_tick()`, which
/// is still 0 here — `restore_tick_count` runs after every body is written.
/// An empty belt takes it as its surface clock (`BeltSegment::restore`).
fn restore_belt(world: &mut World, entity: hecs::Entity, saved: &SavedBelt, tick_count: u64) {
    let (length, min_spacing) = {
        let belt = world
            .ecs()
            .get::<&BeltSegment>(entity)
            .expect("a belt or splitter placement carries a BeltSegment");
        (belt.length, belt.min_spacing)
    };
    let positions: Vec<(ItemId, Fx)> = saved
        .items
        .iter()
        .map(|(item, bits)| (item.clone(), bits.value()))
        .collect();
    world
        .ecs_mut()
        .insert_one(
            entity,
            BeltSegment::restore(
                length,
                min_spacing,
                &positions,
                saved.frontier as usize,
                saved.moved_ticks,
                tick_count,
            ),
        )
        .expect("the entity was just placed");
}

/// A saved ring back into a [`LoadWindow`]: empty means "never saved",
/// which is a fresh window; anything else must be a ring `LoadWindow` could
/// have produced (see [`SavedLoad`]).
fn restore_load(saved: &SavedLoad, tile: TileCoord) -> Result<LoadWindow, SaveError> {
    if saved.buckets.is_empty() {
        return Ok(LoadWindow::new());
    }
    let buckets: Vec<LoadBucket> = saved
        .buckets
        .iter()
        .map(|&(working, starved, blocked)| LoadBucket {
            working,
            starved,
            blocked,
        })
        .collect();
    LoadWindow::restore(&buckets, saved.cursor).ok_or(SaveError::BadLoadWindow {
        tile,
        buckets: saved.buckets.len(),
        cursor: saved.cursor,
    })
}

fn saved_production(ledger: &ProductionLedger) -> SavedProduction {
    SavedProduction {
        cursor: ledger.cursor().to_vec(),
        ticks_into_bucket: ledger.ticks_into_bucket().to_vec(),
        age: ledger.age(),
        cycles_closed: ledger.cycles_closed(),
        items: ledger
            .items()
            .iter()
            .map(|(id, stats)| SavedItemStats {
                id: id.clone(),
                kind: stats.kind,
                produced: stats.produced.iter().map(|ring| ring.to_vec()).collect(),
                consumed: stats.consumed.iter().map(|ring| ring.to_vec()).collect(),
                produced_total: stats.produced_total,
                consumed_total: stats.consumed_total,
                produced_this_cycle: stats.produced_this_cycle,
                produced_last_cycle: stats.produced_last_cycle,
            })
            .collect(),
    }
}

fn restore_production(saved: &SavedProduction) -> Result<ProductionLedger, SaveError> {
    if saved.cursor.is_empty() {
        return Ok(ProductionLedger::new());
    }
    let bad = |detail: String| SaveError::BadProductionLedger { detail };
    let cursor: [usize; STAT_WINDOW_COUNT] = saved.cursor.as_slice().try_into().map_err(|_| {
        bad(format!(
            "{} window cursor(s), not {STAT_WINDOW_COUNT}",
            saved.cursor.len()
        ))
    })?;
    let ticks_into_bucket: [u32; STAT_WINDOW_COUNT] =
        saved.ticks_into_bucket.as_slice().try_into().map_err(|_| {
            bad(format!(
                "{} window clock(s), not {STAT_WINDOW_COUNT}",
                saved.ticks_into_bucket.len()
            ))
        })?;
    let mut items = std::collections::BTreeMap::new();
    for item in &saved.items {
        let rings = |rings: &Vec<Vec<u32>>,
                     what: &str|
         -> Result<[[u32; STAT_BUCKETS]; STAT_WINDOW_COUNT], SaveError> {
            if rings.len() != STAT_WINDOW_COUNT {
                return Err(bad(format!(
                    "'{}' has {} {what} ring(s), not {STAT_WINDOW_COUNT}",
                    item.id,
                    rings.len()
                )));
            }
            let mut out = [[0u32; STAT_BUCKETS]; STAT_WINDOW_COUNT];
            for (w, ring) in rings.iter().enumerate() {
                out[w] = ring.as_slice().try_into().map_err(|_| {
                    bad(format!(
                        "'{}' {what} ring {w} has {} bucket(s), not {STAT_BUCKETS}",
                        item.id,
                        ring.len()
                    ))
                })?;
            }
            Ok(out)
        };
        let stats = ItemStats {
            kind: item.kind,
            produced: rings(&item.produced, "produced")?,
            consumed: rings(&item.consumed, "consumed")?,
            produced_total: item.produced_total,
            consumed_total: item.consumed_total,
            produced_this_cycle: item.produced_this_cycle,
            produced_last_cycle: item.produced_last_cycle,
        };
        if items.insert(item.id.clone(), stats).is_some() {
            return Err(bad(format!("'{}' is listed twice", item.id)));
        }
    }
    ProductionLedger::restore(
        items,
        cursor,
        ticks_into_bucket,
        saved.age,
        saved.cycles_closed,
    )
    .ok_or_else(|| {
        bad(format!(
            "cursor {:?} / clock {:?} is not a state the ledger can be in",
            saved.cursor, saved.ticks_into_bucket
        ))
    })
}

fn check_capacity(
    slots: &[(ItemId, u32)],
    capacity: u32,
    what: &str,
    tile: TileCoord,
) -> Result<(), SaveError> {
    for (item, count) in slots {
        if *count > capacity {
            return Err(SaveError::OverCapacity {
                what: format!("{what} slot '{item}'"),
                tile,
                held: count.to_string(),
                capacity: capacity.to_string(),
            });
        }
    }
    Ok(())
}

/// The world-level ledgers: unplaced equipment, bought ingredient stock and
/// every contract's own clock.
///
/// Owned equipment is credited **after** the rebuild, not before: half one
/// buys exactly one of each thing it places and spends it immediately, so
/// the ledger is empty again by the time this runs and the saved counts are
/// the player's genuine unplaced inventory.
fn restore_ledgers(world: &mut World, data: &SaveData) {
    debug_assert_eq!(
        world.shop().owned.iter().count(),
        0,
        "every unit bought during a rebuild must have been spent by the Place that followed it"
    );
    for (kind, count) in &data.owned_equipment {
        world.shop_mut().owned.add(kind, *count);
    }
    for (id, count) in &data.owned_ingredients {
        world.ingredients_mut().owned.add(id, *count);
    }

    let contracts = data.contracts.clone();
    let economy = world.economy_mut();
    // The active list is EXACTLY the save's. The empty world was built with
    // the board's starters running (that is what a new game is); a save
    // that lost a starter, or signed an offer, says so by listing exactly
    // what is active, so the starters are cleared rather than merged.
    economy.contracts.clear();
    for saved in &contracts {
        let contract = economy
            .board()
            .get(&saved.id)
            .cloned()
            .expect("validate() proved every saved contract id is on the board");
        let mut active = ActiveContract::signed_at(contract, 0);
        active.next_delivery_tick = saved.next_delivery_tick;
        active.deliveries_made = saved.deliveries_made;
        active.deliveries_missed = saved.deliveries_missed;
        active.consecutive_misses = saved.consecutive_misses;
        // Board order, not save order — the same insertion signing uses,
        // so the loaded list's priority is the board's whatever the file
        // says.
        economy.insert_in_board_order(active);
    }
    economy.losses = data.contracts_lost.iter().cloned().collect();
    // Restored, never recomputed from `deliveries_made`: that sum would
    // count only the contracts still active, so a save that lost one — or
    // that was written before this field existed — would quietly re-lock a
    // tier the player had earned. It is its own number for that reason.
    economy.lifetime_deliveries = data.lifetime_deliveries;
    economy.lifetime_boxes_sold = data.lifetime_boxes_sold.clone();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::contracts::Contract;
    use crate::sim::reference::reference_chain;

    const RECIPES_RON: &str = include_str!("../../../data/recipes.ron");

    fn recipes() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(RECIPES_RON).expect("bundled recipes must parse")
    }

    /// A small, real world to mutate saves of. The reference chain rather
    /// than an empty world, so every refusal below is exercised against a
    /// file that would otherwise load.
    fn a_save() -> SaveData {
        reference_chain(recipes()).to_save()
    }

    fn load(data: SaveData) -> Result<World, SaveError> {
        World::from_save(data, recipes())
    }

    fn refusal(data: SaveData) -> String {
        match load(data) {
            Ok(_) => panic!("this save should have been refused"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn the_unmodified_save_loads_so_every_refusal_below_is_about_its_own_change() {
        assert!(load(a_save()).is_ok());
    }

    #[test]
    fn a_different_schema_version_is_rejected_not_migrated() {
        let mut data = a_save();
        data.schema_version = SAVE_SCHEMA_VERSION + 1;
        let message = refusal(data);
        assert!(message.contains("schema_version"), "{message}");

        // ...and the text path reports it as a version problem rather than
        // as whatever the shape change happens to break.
        let text = a_save().to_ron().replace(
            &format!("schema_version: {SAVE_SCHEMA_VERSION}"),
            "schema_version: 99",
        );
        match SaveData::from_ron(&text) {
            Err(SaveError::UnsupportedSchemaVersion { found, supported }) => {
                assert_eq!((found, supported), (99, SAVE_SCHEMA_VERSION));
            }
            other => panic!("expected a version error, got {other:?}"),
        }
    }

    /// The v1 refusal issue #43 created, the v2 refusal per-dock stock
    /// created, the v3 refusal the pay cycle created and the v4 refusal the
    /// boxes-sold gates created, said out
    /// loud. A v1 file predates a tank declaring what it holds, a v2 file
    /// predates stock standing on its own dock, a v3 file predates the pay
    /// cycle, a v4 file predates a gate counting boxes of a named product;
    /// this build will load none of them,
    /// and the thing that matters is that it says *why* — both version
    /// numbers, so the message reads as "this save predates a change" rather
    /// than as a corrupt file. Migration is deliberately not on the table
    /// (ARCHITECTURE.md §4, and this module's doc).
    #[test]
    fn an_older_save_is_refused_with_both_numbers_in_the_message() {
        for old in [1, 2, 3, 4] {
            let text = a_save().to_ron().replace(
                &format!("schema_version: {SAVE_SCHEMA_VERSION}"),
                &format!("schema_version: {old}"),
            );
            let error =
                SaveData::from_ron(&text).expect_err("an older save is not loadable by this build");
            assert!(
                matches!(
                    error,
                    SaveError::UnsupportedSchemaVersion { found, supported }
                        if found == old && supported == SAVE_SCHEMA_VERSION
                ),
                "expected a version refusal naming {old} and {SAVE_SCHEMA_VERSION}, got {error:?}"
            );
            let message = error.to_string();
            assert!(
                message.contains(&format!("schema_version {old}"))
                    && message.contains(&format!("supports {SAVE_SCHEMA_VERSION}")),
                "the refusal must name both versions so it does not read as a parse failure: \
                 {message}"
            );
        }
    }

    /// The version this build *just* moved past is the one a developer is
    /// most likely to meet, so its refusal explains the change in words
    /// instead of leaving two numbers to be interpreted. A version with no
    /// story attached stays bare — the helper must not pad every message.
    #[test]
    fn the_version_that_just_moved_explains_itself_and_no_other_does() {
        let message = SaveError::UnsupportedSchemaVersion {
            found: 4,
            supported: SAVE_SCHEMA_VERSION,
        }
        .to_string();
        for phrase in ["delivery windows", "boxes of a", "will not guess"] {
            assert!(
                message.contains(phrase),
                "a v4 save is refused because the gate trigger changed, and the message \
                 has to say so — `{phrase}` missing from: {message}"
            );
        }

        let bare = SaveError::UnsupportedSchemaVersion {
            found: SAVE_SCHEMA_VERSION + 1,
            supported: SAVE_SCHEMA_VERSION,
        }
        .to_string();
        assert!(
            !bare.contains("delivery windows"),
            "a version this build has no story for gets the numbers and nothing else: {bare}"
        );
    }

    #[test]
    fn malformed_ron_is_a_parse_error() {
        match SaveData::from_ron("(schema_version: 1, tick_count: ") {
            Err(SaveError::Parse(_)) => {}
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn a_kind_this_build_cannot_construct_is_refused() {
        let mut data = a_save();
        data.placements[0].kind = "flux_capacitor".to_string();
        let message = refusal(data);
        assert!(message.contains("flux_capacitor"), "{message}");
        assert!(message.contains("cannot construct"), "{message}");
    }

    #[test]
    fn a_body_that_is_not_its_kinds_body_is_refused() {
        let mut data = a_save();
        let belt = data
            .placements
            .iter_mut()
            .find(|p| p.kind == BELT_KIND)
            .expect("the reference chain has belts");
        belt.body = SavedBody::Pipe(SavedPipe {
            volume_bits: FxBits(0),
        });
        let message = refusal(data);
        assert!(message.contains("pipe body"), "{message}");
    }

    #[test]
    fn a_hopper_holding_something_that_is_not_raw_is_refused() {
        let mut data = a_save();
        let hopper = data
            .placements
            .iter_mut()
            .find(|p| p.kind == HOPPER_KIND)
            .expect("the reference chain has hoppers");
        hopper.body = SavedBody::Hopper(SavedHopper {
            item: "boxed_ketchup".to_string(),
            quantity: 3,
            auto_fill: false,
        });
        let message = refusal(data);
        assert!(message.contains("boxed_ketchup"), "{message}");
        assert!(message.contains("raw ingredient"), "{message}");
    }

    #[test]
    fn two_placements_on_one_tile_are_refused() {
        let mut data = a_save();
        let duplicate = data.placements[0].clone();
        data.placements.push(duplicate);
        let message = refusal(data);
        assert!(message.contains("two placements"), "{message}");
    }

    /// Not the same case as the one above: these two origins differ, and it
    /// is the *rotated footprints* that collide — which is why the check
    /// walks every tile a placement claims rather than comparing origins.
    #[test]
    fn overlapping_footprints_are_refused() {
        let mut data = a_save();
        let vat = data
            .placements
            .iter()
            .find(|p| p.kind == "cooking_vat")
            .expect("the reference chain has a 2x2 cooking vat")
            .clone();
        // The vat's far corner, which no *origin* in the file names.
        let corner = (vat.tile.0 + 1, vat.tile.1 + 1);
        data.placements.push(SavedPlacement {
            kind: BELT_KIND.to_string(),
            tile: corner,
            direction: Direction::East,
            body: SavedBody::Belt(SavedBelt {
                items: Vec::new(),
                frontier: 0,
                moved_ticks: 0,
            }),
        });
        let message = refusal(data);
        assert!(message.contains("two placements"), "{message}");
        assert!(message.contains(&format!("{corner:?}")), "{message}");
    }

    #[test]
    fn an_item_no_recipe_mentions_any_more_is_refused() {
        let mut data = a_save();
        let belt = data
            .placements
            .iter_mut()
            .find(|p| p.kind == BELT_KIND)
            .expect("the reference chain has belts");
        belt.body = SavedBody::Belt(SavedBelt {
            items: vec![("brine".to_string(), FxBits::of(Fx::from_num(0.5)))],
            frontier: 0,
            moved_ticks: 0,
        });
        let message = refusal(data);
        assert!(message.contains("'brine'"), "{message}");
        assert!(message.contains("no recipe"), "{message}");
    }

    #[test]
    fn a_buffer_holding_more_than_it_can_is_refused() {
        let mut data = a_save();
        let machine = data
            .placements
            .iter_mut()
            .find(|p| p.kind == "crusher")
            .expect("the reference chain has a crusher");
        machine.body = SavedBody::Machine(SavedMachine {
            input: vec![("tomato".to_string(), 9_999)],
            output: Vec::new(),
            process: None,
            load: SavedLoad::default(),
            refused_by: None,
            refusal_hold: 0,
            recipe: None,
            mothballed: false,
        });
        let message = refusal(data);
        assert!(message.contains("9999"), "{message}");
        assert!(message.contains("capacity"), "{message}");
    }

    #[test]
    fn a_load_ring_that_is_not_a_ring_is_refused_and_a_missing_one_is_fresh() {
        let mut data = a_save();
        let crusher = data
            .placements
            .iter_mut()
            .find(|p| p.kind == "crusher")
            .expect("the reference chain has a crusher");
        let SavedBody::Machine(machine) = &mut crusher.body else {
            panic!("a crusher saves as a machine");
        };
        machine.load = SavedLoad {
            buckets: vec![(1, 0, 0); 3],
            cursor: 0,
        };
        let message = refusal(data);
        assert!(message.contains("load window"), "{message}");
        assert!(message.contains("3 bucket"), "{message}");

        // A save written before the window existed carries no `load` at
        // all; it must still parse (`#[serde(default)]`) and come back with
        // an empty window rather than being refused.
        let legacy: SavedMachine = ron::from_str("(input: [], output: [], process: None)")
            .expect("a pre-load-window machine record still parses");
        assert_eq!(legacy.load, SavedLoad::default());
        assert_eq!(
            legacy.recipe, None,
            "and the same record predates the recipe field, which defaults the same way"
        );
        let mut data = a_save();
        for placement in &mut data.placements {
            if let SavedBody::Machine(machine) = &mut placement.body {
                machine.load = SavedLoad::default();
            }
        }
        let world = load(data).expect("an absent ring is a fresh window, not an error");
        assert!(
            world
                .machine_load_snapshots()
                .iter()
                .all(|entry| entry.load.observed == 0),
            "every machine comes back with nothing observed"
        );
    }

    #[test]
    fn a_pipe_holding_more_than_it_can_is_refused() {
        let mut data = a_save();
        let pipe = data
            .placements
            .iter_mut()
            .find(|p| p.kind == PIPE_KIND)
            .expect("the reference chain has pipes");
        pipe.body = SavedBody::Pipe(SavedPipe {
            volume_bits: FxBits::of(Fx::from_num(9_999)),
        });
        let message = refusal(data);
        assert!(message.contains("capacity"), "{message}");
    }

    #[test]
    fn an_active_contract_the_board_does_not_offer_is_refused_but_an_unsigned_offer_is_not() {
        let mut data = a_save();
        data.contracts[0].id = "midnight_kebab_van".to_string();
        let message = refusal(data);
        assert!(message.contains("midnight_kebab_van"), "{message}");

        let mut data = a_save();
        data.contracts_lost
            .push(("midnight_kebab_van".to_string(), 1));
        let message = refusal(data);
        assert!(message.contains("midnight_kebab_van"), "{message}");

        // The other direction: a save that names NO active contract is a
        // player who lost their starter and has not re-signed it — it loads,
        // and it loads with nothing active rather than with the starters
        // quietly re-activated.
        let mut data = a_save();
        data.contracts.clear();
        let world = load(data).expect("an empty active list is a legal state");
        assert!(world.economy().contracts.is_empty());
        assert!(
            !world.economy().board().is_empty(),
            "the board is content and is still there to sign from"
        );
    }

    #[test]
    fn the_same_contract_active_twice_is_refused() {
        let mut data = a_save();
        let dup = data.contracts[0].clone();
        data.contracts.push(dup);
        let message = refusal(data);
        assert!(message.contains("twice"), "{message}");
    }

    #[test]
    fn signed_lost_and_strike_state_survives_a_round_trip_in_board_order() {
        let mut world = reference_chain(recipes());
        // Sign every offer that is SIGNABLE, so the active list is longer
        // than the starters. Funded from those offers' own signing fees
        // rather than a literal, so a bigger (or dearer) board cannot
        // silently leave offers unsigned and turn the assertion below into a
        // comparison of two short lists.
        //
        // Signable and not simply "every offer": since `data/unlocks.ron`,
        // part of the shipped board is behind a progression gate and
        // `try_sign` refuses it at zero lifetime deliveries. What this test
        // is about is that a signed contract keeps its FILE-ORDER rank
        // through a save, and a gated offer that never became active has no
        // rank to keep.
        let signable: Vec<Contract> = world
            .economy()
            .board()
            .ordered()
            .iter()
            .filter(|c| world.contract_unlocked(&c.id))
            .cloned()
            .collect();
        let ids: Vec<String> = signable.iter().map(|c| c.id.clone()).collect();
        let signing_bill = signable
            .iter()
            .fold(Money::from_num(0), |sum, c| sum + c.price);
        world.restore_money(signing_bill + Money::from_num(100));
        for id in ids.iter().rev() {
            let _ = world.economy_mut().try_sign(id, 0);
        }
        let board_order: Vec<String> = world
            .economy()
            .contracts
            .iter()
            .map(|c| c.contract.id.clone())
            .collect();
        assert_eq!(
            board_order, ids,
            "signing in reverse still yields board order"
        );
        // Nothing is ever delivered, so every contract is missing windows.
        // Run until exactly the quickest-to-lose offer is gone (the one
        // with the smallest cadence x strikes), which leaves the others
        // active — some of them with a strike on the record.
        let last = signable
            .iter()
            .min_by_key(|c| c.cadence_ticks * u64::from(c.strikes))
            .unwrap()
            .clone();
        for _ in 0..(last.cadence_ticks * u64::from(last.strikes) + 1) {
            world.tick();
        }
        assert!(
            !world.economy().is_active(&last.id),
            "the fixture must have lost one"
        );
        assert_eq!(world.economy().times_lost(&last.id), 1);
        assert!(
            !world.economy().contracts.is_empty(),
            "the fixture must have kept at least one contract active"
        );
        let struck: Vec<(String, u32)> = world
            .economy()
            .contracts
            .iter()
            .map(|c| (c.contract.id.clone(), c.consecutive_misses))
            .collect();
        assert!(
            struck.iter().any(|(_, misses)| *misses > 0),
            "against the shipped board a surviving contract has a strike by now: {struck:?}"
        );

        let loaded = World::from_save(world.to_save(), recipes()).expect("must load");
        let loaded_ids: Vec<String> = loaded
            .economy()
            .contracts
            .iter()
            .map(|c| c.contract.id.clone())
            .collect();
        let expected: Vec<String> = ids.iter().filter(|id| **id != last.id).cloned().collect();
        assert_eq!(
            loaded_ids, expected,
            "exactly the saved active set, in board order"
        );
        assert_eq!(loaded.economy().times_lost(&last.id), 1);
        let loaded_struck: Vec<(String, u32)> = loaded
            .economy()
            .contracts
            .iter()
            .map(|c| (c.contract.id.clone(), c.consecutive_misses))
            .collect();
        assert_eq!(loaded_struck, struck, "strike counts come back exactly");
        assert_eq!(
            loaded.to_save(),
            world.to_save(),
            "and a second save is identical"
        );
    }

    #[test]
    fn an_owned_kind_or_ingredient_this_build_does_not_price_is_refused() {
        let mut data = a_save();
        data.owned_equipment.push(("hoverbelt".to_string(), 3));
        let message = refusal(data);
        assert!(message.contains("hoverbelt"), "{message}");

        let mut data = a_save();
        data.owned_ingredients.push(("saffron".to_string(), 1));
        let message = refusal(data);
        assert!(message.contains("saffron"), "{message}");
    }

    /// Every refusal above has to be a refusal *before* anything is
    /// written, so a caller's own world is untouched. `from_save` builds a
    /// new world and hands it back only on success, so this is structural —
    /// but it is worth pinning that the failing load does not, for example,
    /// consume the recipes it was given in a way that leaves the caller
    /// without a world at all.
    #[test]
    fn a_refused_load_leaves_the_caller_with_their_own_world() {
        let mut world = reference_chain(recipes());
        world.tick();
        let before = world.to_save_ron();

        let mut data = a_save();
        data.placements[0].kind = "flux_capacitor".to_string();
        assert!(World::from_save(data, recipes()).is_err());

        assert_eq!(world.to_save_ron(), before);
    }
}
