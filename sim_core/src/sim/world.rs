//! The simulation `World`: an `hecs::World` (ARCHITECTURE.md §1/§3.8) of
//! **placed** things — belts, pipes, machines and buffer buildings the
//! player bought and put down — plus the topology derived from where they
//! happen to stand.
//!
//! # What the "Real Placement" epic changed, and why
//!
//! Until this epic `World::new` *built a factory*. It read
//! `data/recipes.ron`'s list order, laid one `(belt -> machine)` stage per
//! recipe west to east, stood a hopper at the head, a tank beside the pipe
//! run and a dock at the tail, and linked every stage to the next by index
//! at construction time. Everything existed unconditionally from tick 0, in
//! exactly one arrangement, facing exactly one way. The player could fill
//! the two buffers and buy equipment whose owned count nothing ever read
//! (the "Shop & Inventory" epic's ticket 1 built that ledger and said, in
//! as many words, that wiring it to real placement was a later epic).
//!
//! **This is that wiring. `World::new` now builds nothing at all:** an empty
//! grid, an empty adjacency graph, and [`Economy::money`] seeded from
//! [`STARTING_MONEY`]. Everything in a world is something a
//! [`PlayerCommand::Purchase`] paid for and a [`PlayerCommand::Place`] put
//! down, and both [`PlayerCommand::Rotate`] and [`PlayerCommand::Remove`]
//! can change it afterwards. GAMEPLAY.md §4's "equipment purchases *are*
//! the tech tree" is a real statement about the simulation for the first
//! time.
//!
//! Three things follow from that, and they are the whole of this module's
//! new shape:
//!
//! **1. Flow is derived from adjacency, not from a list.** There is no
//! `stages: Vec<Stage>` any more, and no chain order. A belt tile carries
//! items toward the tile its own [`Direction`] points at, and what happens
//! there depends entirely on what is standing there: another belt, a
//! machine's inlet face, a dock, or nothing. A machine discharges onto
//! whatever belt is adjacent to its outlet face. `recipes.ron`'s file order
//! still *describes* a chain — [`validate_chain`] still enforces that, since
//! it is what makes the shipped content a coherent line and what
//! [`super::reference::reference_chain`] builds — but nothing in the
//! simulation reads it as geometry any more.
//!
//! **2. Topology is recomputed on a topology change, and never per tick**
//! (ARCHITECTURE.md §3.8's union-find rule, made real for the first time —
//! see [`World::recompute_topology`]). `Place`, `Remove` and `Rotate` are
//! the only three things that can change what is adjacent to what, so they
//! are the only three that pay for the recompute. Item movement inside a
//! tick still uses the exact same [`BeltSegment`]/[`Machine`]/
//! [`FluidNetwork`] mechanics it always did; only how a pair of them is
//! *linked* changed. The recompute is a single-threaded pass — `rayon`
//! parallelism and multi-network partitioning stay ARCHITECTURE.md §9
//! milestone 8's scope, deliberately not half-built here.
//!
//! **3. The hopper, the fluid tank and the dock are ordinary placeable
//! kinds.** All three used to be `Option`-less fields constructed once at a
//! fixed relative position. They are now placed, rotated and removed like
//! anything else, and a world may hold **zero, one or several of each** —
//! each filling, discharging or shipping independently. That is not a
//! feature anyone asked for so much as the absence of an invented
//! restriction: nothing in the simulation ever assumed exactly one, so
//! forbidding a second would have been new code enforcing a rule no design
//! document states (CLAUDE.md: don't design for hypothetical requirements).
//! A second dock is simply a second place boxes leave the factory; both
//! ship into [`Economy`]'s one shared pool, which is where "how much is on
//! the dock" has always been one number.
//!
//! # The conventions this module depends on
//!
//! [`super::machines`]'s module doc owns the placement geometry — origin
//! corner, how a footprint rotates, and which edge is the inlet face and
//! which the outlet. [`Direction`]'s own doc owns the axis convention
//! (`East` is `+X`, `North` is `+Y`) and the fact that **`East` reproduces
//! every pre-placement behaviour exactly**, which is what makes
//! [`super::reference::reference_chain`] a genuine regression anchor: it
//! builds the old fixed layout, tile for tile, out of nothing but purchase
//! and placement commands.
//!
//! Three adjacency rules are decided *here*, because they are about linking
//! rather than geometry:
//!
//! - **Solids respect faces.** A belt feeds a machine only if the tile it
//!   points into is on that machine's inlet face, and a machine discharges
//!   only onto a belt standing off its outlet face. A belt pointing into a
//!   machine's flank does nothing. This is what makes rotating a machine a
//!   decision with a visible consequence.
//! - **A fluid is drawn by touching and pushed by facing.** A machine draws
//!   its recipe's `fluid_inputs` from any pipe tile orthogonally adjacent to
//!   any tile of its footprint, whatever either is facing, and pipe-to-pipe
//!   flow is undirected (see [`super::fluid`]'s module doc). That half is
//!   not a shortcut: the shipped mixer has always been fed from the row
//!   *above* it — neither its inlet face nor its outlet — because a pressure
//!   network is not a conveyor, and requiring a fluid inlet to face the
//!   right way would break the one plumbing arrangement the game has always
//!   had.
//!
//!   Its `fluid_outputs`, added by the chain-rework epic, are the other way
//!   round: **a fluid output leaves by the face the machine points at,
//!   exactly as a solid one does.** [`super::machines`]'s convention is that
//!   `direction` *is* "the side product leaves by", and a liquid product is
//!   still product. That asymmetry is what makes a machine plumbed on both
//!   sides unambiguous without the player having to label a run: the mixer
//!   pushes its ketchup mix east into the pipe it faces, and draws its
//!   vinegar from the run merely touching it. An input tap is therefore
//!   resolved as "the first adjacent pipe network that is **not** the output
//!   network" — so a loop of pipe touching a machine on both sides is read
//!   as its outlet and gives it no inlet at all, rather than as a machine
//!   quietly feeding itself.
//!
//!   A network's fluid id is still decided by content and not by the player:
//!   the topology pass claims an unclaimed network for whichever fluid the
//!   first machine to reach it names, whether that machine is drawing or
//!   depositing. Producer and consumer agree by naming the same id in
//!   `recipes.ron` — a run between the mixer and the vat resolves to
//!   `"ketchup_mix"` whichever end is walked first — and a run whose two
//!   ends disagree simply stalls one of them, the same way a machine tapping
//!   the wrong fluid always has.
//! - **One placement resolves to two downstream targets, and only one.** A
//!   belt looks at the single tile it points at; a
//!   [splitter](super::splitter) looks at two — the tile on its own facing
//!   and the tile one quarter turn clockwise from it — and deals its front
//!   item alternately between them (`splitter_outputs` below, and
//!   `splitter.rs`'s module doc for both rules). Everything else about it is
//!   belt-like: it holds what it carries in an ordinary [`BeltSegment`] and
//!   accepts a hand-off from anything a belt accepts one from, so it can be
//!   dropped into a line in place of a belt tile without changing how fast
//!   product moves through it.
//!
//! # Corners, finally
//!
//! This module's doc used to carry a long note explaining that corners were
//! out of scope because "there is no `direction` field anywhere in a
//! `BeltSnapshot`/`PipeSnapshot`, so a renderer has nowhere to even learn
//! that a tile turns," and that fixing it was "a topology change, not a
//! rendering one." That is exactly what this ticket did. Every belt tile,
//! pipe tile, machine and building now carries a real [`Direction`] on the
//! wire, a belt run may turn as often as the player likes, and the sim
//! links a corner the same way it links a straight — by asking what is in
//! the tile a belt points at. The other half that note named — rotated
//! voxel models, and a corner that is *drawn* as a corner — is presentation:
//! the Real Placement epic's ticket 2 spun every model by its facing, and
//! the Pipes & Conveyors Overhaul reads shape off [`Links`] (below) in
//! `main.gd` — ticket 6 a pipe tile's shape, turn and fittings, ticket 5 a
//! belt's shape (straight, left or right corner, merge, line start, line
//! end) at every facing, with no east-only junction art left on the belt.
//!
//! **And the wire now says which way a corner turns** (the Pipes &
//! Conveyors Overhaul, ticket 1 — `docs/PIPES_AND_CONVEYORS_PLAN.md`). Every
//! belt, splitter and pipe placement carries [`Links`]: what the topology
//! pass linked into it and on which side, what it hands to, or which sides
//! its pipe is joined on. It is recorded by [`World::recompute_topology`] at
//! the moment each relation is resolved — the same `belt_to_belt`,
//! `belt_to_sink`, `source_to_belt`, `splitter_outputs`, pipe adjacency and
//! tap lists the tick reads — so a renderer drawing a corner, a tee or a
//! port fitting from it is drawing where product actually goes, and it
//! costs nothing per tick.
//!
//! This is also where `SimBridge`'s pure-logic half lives (ARCHITECTURE.md
//! §4): `tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
//! `query_tile()`, `world_layout()`. The `gdext`-facing wrapper in `lib.rs`
//! is a thin translation layer on top of this — nothing here depends on
//! Godot, so it is fully exercisable from headless `cargo test`
//! (ARCHITECTURE.md §7).

use std::collections::{BTreeMap, HashMap};

use hecs::Entity;

use super::belt::BeltSegment;
use super::contracts::ContractRegistry;
use super::dock::{DOCK_CAPACITY, DOCK_KIND, Dock};
use super::economy::{ContractEvent, Economy, EconomySnapshot, STARTING_MONEY};
use super::fluid::{
    FLUID_TANK_CAPACITY, FLUID_TANK_FILL_AMOUNT, FLUID_TANK_KIND, FluidNetwork, FluidTank,
    FluidTap, PipeSegment,
};
use super::hopper::{HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, HOPPER_KIND, Hopper};
use super::ingredients::{
    DEFAULT_BUY_QUANTITY, Ingredients, IngredientsSnapshot, raw_ingredient_ids,
};
use super::machine::{LOAD_WINDOW_TICKS, LoadWindow, Machine, WorkState};
use super::machines::{Footprint, MachineRegistry};
use super::recipe::{Recipe, RecipeRegistry};
use super::shop::{DEFAULT_PURCHASE_QUANTITY, Shop, ShopSnapshot};
use super::splitter::{SPLITTER_KIND, Side, Splitter};
use super::stats::{ProductionLedger, ProductionSnapshot, StatKind};
use super::staff::{CrewEntry, CrewRequest, StaffPools, StaffSnapshot, crew_entries};
use super::types::{ChunkId, Direction, FluidId, Fx, ItemId, RecipeId, TileCoord, chunk_of};

/// The buildable-kind id (`data/shop.ron`) of one tile of conveyor.
///
/// Spelled out in Rust for the reason [`HOPPER_KIND`] and [`DOCK_KIND`] are,
/// with one difference worth noting: a belt is not in `data/machines.ron` at
/// all, because it is not a machine (that file's own header, and
/// `shop.ron`'s). Its footprint is therefore not content — it is one tile,
/// always, which is what "priced per tile" in the shop catalog means.
pub const BELT_KIND: &str = "belt";

/// The buildable-kind id of one tile of pipe — [`BELT_KIND`]'s fluid twin,
/// and not a machine either.
pub const PIPE_KIND: &str = "pipe";

/// Where an entity sits in the tile grid (ARCHITECTURE.md §3.1/§3.2). Always
/// the placement's **origin**: its minimum-X/minimum-Y corner, whatever it
/// is facing (see [`super::machines`]'s module doc).
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub tile: TileCoord,
}

/// What a placed entity is, how much room it takes and which way it faces —
/// the three facts every placement has, whether it is a machine, a belt
/// tile, or a dock.
///
/// `footprint` is the kind's **declared** (unrotated) footprint, straight
/// out of `data/machines.ron` — the rectangle it actually occupies is
/// `footprint.rotated(direction)` anchored at the entity's [`Position`].
/// Both are reported on the wire so presentation never has to re-derive
/// either.
#[derive(Debug, Clone)]
pub struct Placement {
    pub kind: String,
    pub footprint: Footprint,
    pub direction: Direction,
}

impl Placement {
    /// Every tile this placement claims.
    pub fn tiles(&self, origin: TileCoord) -> Vec<TileCoord> {
        self.footprint.tiles_facing(origin, self.direction)
    }
}

/// A placed tile of pipe. The fluid it holds is **not** here — it lives in
/// whichever [`FluidNetwork`] this tile currently belongs to, because
/// pressure is a property of a connected run rather than of one fitting
/// (ARCHITECTURE.md §3.4). What is here is the only thing a pipe tile owns
/// on its own: how much it can hold.
#[derive(Debug, Clone, Copy)]
struct PipeTile {
    capacity: Fx,
}

/// A command the player can issue (ARCHITECTURE.md §4's `PlayerCommand`).
///
/// **Three of these are new in the "Real Placement" epic** — [`Place`],
/// [`Remove`] and [`Rotate`] — and they are what turned the two that came
/// before them into something worth having: `Purchase` credited a count
/// nothing read, and the two fill commands supplied buildings the world
/// created for free.
///
/// [`Place`]: PlayerCommand::Place
/// [`Remove`]: PlayerCommand::Remove
/// [`Rotate`]: PlayerCommand::Rotate
///
/// Every variant follows the same rule, which is this crate's universal one:
/// **an impossible command is a silent, total no-op.** Filling a full
/// hopper, buying what you cannot afford, placing what you do not own or
/// where it will not fit, rotating into a collision, removing an empty tile
/// — all leave the world byte-identical rather than erroring. Presentation
/// decides what to say about it, using `shop_snapshot_json` and
/// `world_layout_json` to avoid offering the impossible action in the first
/// place.
#[derive(Debug, Clone)]
pub enum PlayerCommand {
    /// Loads raw ingredient into a hopper, **out of the stock the player has
    /// bought**. The *item* is not a parameter: each hopper holds exactly
    /// one item id, chosen when it was placed (see
    /// [`PlayerCommand::Place`]), so naming an item here could only ever
    /// name the wrong one.
    ///
    /// `tile` names **which** hopper (any tile of its footprint, like every
    /// other tile-addressed command here). `None` means *every* hopper in
    /// the world, each clamped at its own capacity — which is what a world
    /// with one hopper has always done on a keypress, and which is why the
    /// pre-placement wire format still works unchanged. A world with no
    /// hopper at all, or a `tile` holding something else, is a no-op.
    ///
    /// **The stock check is new with the ingredient economy** (GAMEPLAY.md
    /// §5): `amount` is clamped to how many of that hopper's item the player
    /// owns *before* [`Hopper::fill`] clamps it again to the room in the
    /// bin, and the owned ledger is debited by **exactly what `fill` reports
    /// it added** — never the requested amount, since a nearly-full hopper
    /// accepts less than it is offered and the difference must not vanish
    /// from the ledger. Filling with no stock is a silent no-op, like every
    /// other under-resourced command here.
    FillHopper {
        amount: u32,
        tile: Option<TileCoord>,
    },
    /// Loads fluid into a tank, in whole fluid units, out of bought stock.
    /// Exactly [`PlayerCommand::FillHopper`]'s shape for exactly its
    /// reasons, including `tile` and the clamp-then-debit-what-was-added
    /// rule.
    ///
    /// The *fluid* is not a parameter here either, but for a subtler reason
    /// than the hopper's: a tank deliberately holds no fluid id of its own
    /// (see [`FluidTank`]), so what it is being filled with is **the fluid
    /// of the network it discharges into** — which is what its own snapshot
    /// has always reported. A tank plumbed to nothing, or to a run no
    /// machine taps, therefore has no id to charge the player for and is a
    /// no-op: you cannot buy a drum of nothing.
    FillFluidTank {
        amount: u32,
        tile: Option<TileCoord>,
    },
    /// Buys `quantity` units of a raw ingredient — the consumable half of
    /// GAMEPLAY.md §5's cost axes, and the twin of
    /// [`PlayerCommand::Purchase`] in every respect: priced from
    /// `data/ingredients.ron`, paid out of the one [`Economy::money`]
    /// counter through `try_spend`, credited to a per-id ledger
    /// ([`super::ingredients::OwnedIngredients`]), and **all-or-nothing and
    /// silent about failure** — an unknown ingredient, a zero quantity, an
    /// unrepresentable total or simply not enough money all leave the world
    /// byte-identical.
    ///
    /// `item` names an item *or* a fluid: both are plain string ids and both
    /// are bought by the unit (see [`super::ingredients`]).
    BuyIngredient { item: String, quantity: u32 },
    /// Buys `quantity` of a buildable kind from the shop (the "Shop &
    /// Inventory" epic's ticket 1): debits `quantity * price` from
    /// [`Economy::money`] and credits `quantity` to the player's owned count
    /// of that kind (`sim::shop::OwnedEquipment`).
    ///
    /// **All or nothing, and silent about failure.** An unknown kind, a zero
    /// quantity, or a total the player can't afford leaves money *and* the
    /// owned count exactly as they were; nothing is ever partially applied.
    Purchase { kind: String, quantity: u32 },
    /// Puts one owned unit of `kind` on the floor at `tile`, facing
    /// `direction` (the "Real Placement" epic).
    ///
    /// **It spends inventory, not money** — the money went at
    /// [`PlayerCommand::Purchase`] time. One owned unit becomes one thing on
    /// the floor, and [`PlayerCommand::Remove`] gives it back, so the two
    /// ledgers (what you own, what you have placed) always add up to what
    /// you bought.
    ///
    /// A silent no-op, changing nothing at all, when: the player owns none
    /// of `kind`; `kind` is not a buildable kind at all; `kind` is a machine
    /// kind no recipe runs on (there is nothing to construct); or the
    /// rotated footprint would overlap **any** already-claimed tile — belt,
    /// pipe, machine or building. Overlap is checked against the whole
    /// rotated rectangle, not just `tile`.
    ///
    /// `tile` is the footprint's origin — its minimum-X/minimum-Y corner —
    /// exactly as it is everywhere else in this crate. Placing succeeds or
    /// fails as a unit; there is no partial placement.
    ///
    /// **`item` is the hopper's contents, and is only read for a hopper.**
    /// A hopper holds one item id for its whole life ([`super::hopper`]), and
    /// until the chain gained a second raw ingredient the simulation owned a
    /// sensible answer for which — there was exactly one raw item, so every
    /// hopper held it and the player was never asked. With `tomato` *and*
    /// `sugar` there is no such answer, and this field follows
    /// `PlayerCommandDto`'s own rule for that case: **optional while it is
    /// inferable, required once it is ambiguous.** Concretely, placing a
    /// hopper with `item: None` succeeds in a world whose recipes name
    /// exactly one raw item and is a silent no-op in one that names two, and
    /// naming an item that is not raw at all is likewise refused. For every
    /// other kind the field is ignored.
    Place {
        kind: String,
        tile: TileCoord,
        direction: Direction,
        item: Option<ItemId>,
    },
    /// Picks up whatever occupies `tile` and credits one back to the
    /// player's owned count of its kind.
    ///
    /// **Any tile of a multi-tile footprint works**, and the whole thing is
    /// removed — the same "hover anywhere on it" rule
    /// [`World::query_tile`] follows, for the same reason: the player is
    /// pointing at a machine, not at one of its corners. An empty tile is a
    /// silent no-op.
    ///
    /// **What it holds is destroyed, deliberately.** Items on a removed belt
    /// tile, fluid in a removed pipe, a batch mid-craft in a removed machine
    /// and a removed hopper's contents all go with it. That is the honest
    /// reading of picking a thing up, and pretending otherwise would need an
    /// inventory to put the spillage in — which is a mechanic nobody has
    /// asked for. It does mean a mass-conservation invariant only holds
    /// across a run in which nothing loaded is removed, which is what the
    /// tests that assert one actually do.
    Remove { tile: TileCoord },
    /// Turns whatever occupies `tile` 90° clockwise in place (see
    /// [`Direction::rotate_cw`]).
    ///
    /// Any tile of a multi-tile footprint works, like [`Self::Remove`]. A
    /// silent no-op on an empty tile, or when the *rotated* footprint would
    /// collide with something else — a 3x1 machine turned into a 1x3 claims
    /// tiles it did not before, and it may not take them from a neighbour.
    /// The thing being rotated is of course allowed to overlap its own old
    /// tiles.
    ///
    /// **What it holds is untouched.** A belt's gap array, a pipe's volume,
    /// a machine's buffers and its running [`super::machine::TimedProcess`]
    /// all carry on: none of them has any absolute-direction dependency, so
    /// turning a belt is a pure geometry change and turning a machine
    /// mid-craft does not restart the craft.
    Rotate { tile: TileCoord },
    /// Switches the hopper at `tile` to hold `item` — the ONE way a placed
    /// hopper changes its mind, and the one a panel with a list of
    /// ingredients in it wants: a picker that already names the item should
    /// not have to press "next" until it arrives there.
    ///
    /// It replaced a `CycleHopperItem` variant, deleted when the **Q**
    /// keybind that was its only caller went (the click-to-configure
    /// ticket). Cycling is what a key can express and a picker cannot
    /// usefully offer; two commands for one decision is exactly what the
    /// delivery dock refused to accumulate when it replaced `CollectOutput`.
    ///
    /// A silent no-op on an empty tile, on a tile that is not a hopper, and
    /// on an item that is not raw (the same set [`Self::Place`] validates a
    /// hopper's contents against — [`World::raw_items`]). Naming the item
    /// the hopper **already** holds is a no-op too, contents and all: a
    /// picker redraws its own current selection, and re-selecting it must
    /// not empty the bin. Otherwise it is exactly
    /// [`super::hopper::Hopper::retarget`] — the contents go, the
    /// [`super::hopper::Hopper::auto_fill`] flag stays.
    SetHopperItem { tile: TileCoord, item: ItemId },
    /// Signs the contract board's offer `id` (the contract-board epic; see
    /// [`Economy::try_sign`]): debits its price from the one money counter
    /// and activates it with a fresh clock. `data/contracts.ron` is a board
    /// of offers, and a new game starts with only its `starter`s running;
    /// this is how the player takes on more — and how they take back a
    /// contract they lost by missing its `strikes` windows in a row, at the
    /// same price again.
    ///
    /// **One signature per offer.** A silent no-op, changing nothing at
    /// all, on an id the board does not list, on an offer that is already
    /// active, and on a fee the balance does not cover — the universal
    /// all-or-nothing rule every other command here follows. No chunk is
    /// dirtied: a contract is not a tile.
    SignContract { id: String },
    /// Turns automatic top-up on or off for the hopper at `tile` (see
    /// [`super::hopper::Hopper::auto_fill`]): while it is on, `World::tick`
    /// refills that hopper to capacity out of the player's **bought stock**,
    /// every tick, spending no money.
    ///
    /// Tile-addressed like [`Self::Rotate`] — any tile of the footprint
    /// resolves — and a silent no-op on an empty tile or one holding
    /// something that is not a hopper. Setting the value it already has is a
    /// no-op that dirties nothing.
    SetHopperAutoFill { tile: TileCoord, enabled: bool },
    /// The fluid twin of [`Self::SetHopperAutoFill`], for the tank at
    /// `tile`. A tank plumbed to nothing can be *flagged*, and simply never
    /// tops up while there is no fluid to charge the player for — the same
    /// refusal [`Self::FillFluidTank`] already makes, rather than a second
    /// rule about when the toggle itself is allowed.
    SetTankAutoFill { tile: TileCoord, enabled: bool },
    /// Hires one head into a staff category (the staff pools MVP epic,
    /// GAMEPLAY.md §7). **Free, and never refused for a known category** —
    /// wages are the cost (§5), paid every pay period by `World::tick`, and
    /// the MVP answers §11's recruiting-fee question with "no, for now". An
    /// unknown category is a silent no-op. No chunk is dirtied: a head is
    /// not a tile, and Godot reads the pools off `staff_snapshot_json`.
    Hire { category: String },
    /// Lets one head go. Free and immediate; a silent no-op at headcount 0
    /// or for an unknown category (the `Purchase` convention). A machine
    /// mid-craft that loses its crew pauses on the next tick and resumes,
    /// with no progress lost, when one is free again. No chunk is dirtied.
    Fire { category: String },
}

/// JSON wire shape for [`PlayerCommand`], used only at the `gdext` boundary
/// (`lib.rs`) — GDScript builds a `{"type": "place", "kind": "belt",
/// "tile": [3, 0], "direction": "east"}` dictionary, converts it to a JSON
/// string, and this is what parses it. Kept separate from `PlayerCommand`
/// itself so the pure sim type isn't coupled to a wire format.
///
/// Optional fields follow one rule: a field is optional when the simulation
/// already owns a sensible answer, and required when only the player can
/// know it. So `amount`/`quantity` default (to the simulation's own fill
/// amount and to one), `direction` defaults to [`Direction::East`] — the
/// facing that reproduces every pre-placement behaviour — and `kind` and
/// `tile` are required, because there is no sensible default for *what* to
/// build or *where*.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PlayerCommandDto {
    /// `{"type": "fill_hopper", "amount": 16, "tile": [-1, 0]}` — both
    /// optional. Omitting `tile` fills every hopper in the world.
    FillHopper {
        #[serde(default = "default_fill_amount")]
        amount: u32,
        #[serde(default)]
        tile: Option<TileCoord>,
    },
    /// `{"type": "fill_fluid_tank", "amount": 30, "tile": [5, 1]}` — both
    /// optional, exactly as `fill_hopper`'s are.
    FillFluidTank {
        #[serde(default = "default_fluid_fill_amount")]
        amount: u32,
        #[serde(default)]
        tile: Option<TileCoord>,
    },
    /// `{"type": "purchase", "kind": "belt", "quantity": 4}` — `quantity`
    /// optional, defaulting to [`DEFAULT_PURCHASE_QUANTITY`] (one).
    Purchase {
        kind: String,
        #[serde(default = "default_purchase_quantity")]
        quantity: u32,
    },
    /// `{"type": "buy_ingredient", "item": "tomato", "quantity": 40}` —
    /// `quantity` optional, defaulting to [`DEFAULT_BUY_QUANTITY`] (one),
    /// exactly as `purchase`'s does.
    BuyIngredient {
        item: String,
        #[serde(default = "default_buy_quantity")]
        quantity: u32,
    },
    /// `{"type": "place", "kind": "crusher", "tile": [4, 0],
    /// "direction": "east"}` — `direction` optional, defaulting to `"east"`.
    /// `item` is optional too and is only read when placing a hopper; see
    /// [`PlayerCommand::Place`] for when omitting it is a refusal.
    Place {
        kind: String,
        tile: TileCoord,
        #[serde(default)]
        direction: Direction,
        #[serde(default)]
        item: Option<ItemId>,
    },
    /// `{"type": "remove", "tile": [4, 0]}`.
    Remove { tile: TileCoord },
    /// `{"type": "rotate", "tile": [4, 0]}`.
    Rotate { tile: TileCoord },
    /// `{"type": "set_hopper_item", "tile": [-1, 0], "item": "sugar"}` —
    /// both required: only the player knows which hopper and which
    /// ingredient.
    SetHopperItem { tile: TileCoord, item: ItemId },
    /// `{"type": "sign_contract", "id": "diner_supply"}` — `id` required:
    /// only the player knows which offer they are taking.
    SignContract { id: String },
    /// `{"type": "set_hopper_auto_fill", "tile": [-1, 0], "enabled": true}`
    /// — `enabled` optional, defaulting to `true`, because a panel's toggle
    /// sends the state it wants and "turn it on" is the one this command
    /// exists for.
    SetHopperAutoFill {
        tile: TileCoord,
        #[serde(default = "default_auto_fill_enabled")]
        enabled: bool,
    },
    /// `{"type": "set_tank_auto_fill", "tile": [5, 1], "enabled": true}` —
    /// exactly `set_hopper_auto_fill`'s shape.
    SetTankAutoFill {
        tile: TileCoord,
        #[serde(default = "default_auto_fill_enabled")]
        enabled: bool,
    },
    Hire {
        category: String,
    },
    Fire {
        category: String,
    },
}

/// Turning auto-fill *on* is what the command is for; see
/// `PlayerCommandDto::SetHopperAutoFill`.
fn default_auto_fill_enabled() -> bool {
    true
}

fn default_fill_amount() -> u32 {
    HOPPER_FILL_AMOUNT
}

fn default_fluid_fill_amount() -> u32 {
    FLUID_TANK_FILL_AMOUNT
}

fn default_purchase_quantity() -> u32 {
    DEFAULT_PURCHASE_QUANTITY
}

fn default_buy_quantity() -> u32 {
    DEFAULT_BUY_QUANTITY
}

impl From<PlayerCommandDto> for PlayerCommand {
    fn from(dto: PlayerCommandDto) -> Self {
        match dto {
            PlayerCommandDto::FillHopper { amount, tile } => {
                PlayerCommand::FillHopper { amount, tile }
            }
            PlayerCommandDto::FillFluidTank { amount, tile } => {
                PlayerCommand::FillFluidTank { amount, tile }
            }
            PlayerCommandDto::Purchase { kind, quantity } => {
                PlayerCommand::Purchase { kind, quantity }
            }
            PlayerCommandDto::BuyIngredient { item, quantity } => {
                PlayerCommand::BuyIngredient { item, quantity }
            }
            PlayerCommandDto::Place {
                kind,
                tile,
                direction,
                item,
            } => PlayerCommand::Place {
                kind,
                tile,
                direction,
                item,
            },
            PlayerCommandDto::Remove { tile } => PlayerCommand::Remove { tile },
            PlayerCommandDto::Rotate { tile } => PlayerCommand::Rotate { tile },
            PlayerCommandDto::SetHopperItem { tile, item } => {
                PlayerCommand::SetHopperItem { tile, item }
            }
            PlayerCommandDto::SignContract { id } => PlayerCommand::SignContract { id },
            PlayerCommandDto::SetHopperAutoFill { tile, enabled } => {
                PlayerCommand::SetHopperAutoFill { tile, enabled }
            }
            PlayerCommandDto::SetTankAutoFill { tile, enabled } => {
                PlayerCommand::SetTankAutoFill { tile, enabled }
            }
            PlayerCommandDto::Hire { category } => PlayerCommand::Hire { category },
            PlayerCommandDto::Fire { category } => PlayerCommand::Fire { category },
        }
    }
}

/// Parses a JSON-encoded [`PlayerCommand`] (see `PlayerCommandDto`). Returns
/// `None` on malformed input rather than erroring — an invalid command from
/// Godot is treated the same as a no-op command per `issue_command`'s own
/// contract.
pub fn command_from_json(json: &str) -> Option<PlayerCommand> {
    serde_json::from_str::<PlayerCommandDto>(json)
        .ok()
        .map(PlayerCommand::from)
}

/// A snapshot of one chunk's contents, built fresh on request from live
/// simulation state — never a stored representation. Only presentation
/// wants this; Godot never sees a belt's internal gap array or a machine's
/// raw `TimedProcess` (ARCHITECTURE.md §4/§5).
///
/// Every array here can now be empty *and* can hold several entries: a
/// freshly-started world has nothing in any of them, and a player may place
/// as many hoppers, tanks or docks as they own (see this module's doc).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChunkSnapshot {
    pub belts: Vec<BeltSnapshot>,
    pub splitters: Vec<SplitterSnapshot>,
    pub machines: Vec<MachineSnapshot>,
    pub pipes: Vec<PipeSnapshot>,
    pub hoppers: Vec<HopperSnapshot>,
    pub fluid_tanks: Vec<FluidTankSnapshot>,
    pub docks: Vec<DockSnapshot>,
}

/// One belt **tile**, for rendering.
///
/// JSON shape (`snake_case`, as serialized by `serde_json`):
///
/// ```json
/// { "tile": [3, 0], "direction": "east",
///   "items": [["tomato", 0.62]] }
/// ```
///
/// **This shape is a breaking change from the pre-placement one**, which was
/// `{"start_tile", "length_tiles", "items"}`. A belt used to be a
/// multi-tile *segment* laid down by the chain builder — four tiles at a
/// time — because nothing could place one. A belt is now exactly one tile,
/// bought and placed one at a time and priced per tile in `data/shop.ron`,
/// so `start_tile` became `tile`, `length_tiles` is gone (it would always be
/// 1), and `direction` is new and load-bearing: it is which way this tile
/// carries, and therefore which neighbour it hands off to.
///
/// An item's position is its distance from this tile's start, in tiles, so
/// it is always in `[0, 1]`. `f64` is fine here — it has already crossed
/// from fixed-point simulation state into a presentation-only value that
/// never feeds back in (ARCHITECTURE.md §3.9).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BeltSnapshot {
    pub tile: TileCoord,
    pub direction: Direction,
    pub items: Vec<(ItemId, f64)>,
}

/// One conveyor splitter (Epic B), for rendering.
///
/// JSON shape (`snake_case`, as serialized by `serde_json`):
///
/// ```json
/// { "tile": [6, 0], "kind": "splitter", "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 1 },
///   "items": [["tomato", 0.44]],
///   "outputs": [[7, 0], [6, -1]],
///   "next_output": "straight" }
/// ```
///
/// A splitter is drawn like a belt tile that has something on its end, and
/// this snapshot is [`BeltSnapshot`] plus the three facts only a splitter
/// has:
///
/// - **`outputs` is always a two-element array, straight-through first**:
///   the tile on the placement's own `direction`, then the tile one quarter
///   turn clockwise from it (see [`super::splitter`], which owns that rule —
///   presentation should read these tiles rather than re-derive them). An
///   entry is `null` when nothing that can accept an item stands there,
///   which is exactly the state "this branch isn't built yet".
/// - **`next_output`** is `"straight"` or `"branch"` — which side gets first
///   refusal on the next item. It is the one piece of a splitter's state a
///   player cannot infer from the tiles around it, and the natural thing for
///   an arrow or a lamp to point at.
/// - `items` are positioned exactly as a belt tile's are, in `[0, 1]` from
///   this tile's start, because a splitter carries them in an ordinary
///   one-tile belt segment.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SplitterSnapshot {
    pub tile: TileCoord,
    pub kind: String,
    pub direction: Direction,
    pub footprint: Footprint,
    pub items: Vec<(ItemId, f64)>,
    pub outputs: [Option<TileCoord>; 2],
    pub next_output: Side,
}

/// One machine, for rendering.
///
/// JSON shape (`snake_case`, as serialized by `serde_json`):
///
/// ```json
/// {
///   "tile": [14, 0],
///   "kind": "cooking_vat",
///   "direction": "east",
///   "footprint": { "width_tiles": 2, "height_tiles": 2 },
///   "recipe": "ketchup",
///   "process": [120, 240],
///   "input": [["ketchup_mix", 1]],
///   "output": [["ketchup", 1]]
/// }
/// ```
///
/// **`tile` is the footprint's origin — its minimum-X/minimum-Y corner**
/// (see [`super::machines`]'s module doc), and `footprint` is the kind's
/// **declared** rectangle. `direction` (new in the "Real Placement" epic) is
/// what turns those into the tiles actually covered: at `"north"`/`"south"`
/// the two extents swap, so a renderer must apply the rotation rather than
/// reading `footprint` literally. It is also which face the machine
/// discharges from.
///
/// **Chunk-boundary caveat:** a machine whose footprint straddles a chunk
/// boundary (ARCHITECTURE.md §3.1's 32x32 chunks) appears in the snapshot of
/// *every* chunk it overlaps, each time with identical contents — including
/// the same origin `tile`, which may lie outside the chunk being asked
/// about. That is deliberate: it means a renderer that only loads visible
/// chunks still learns about a machine whose origin is off-screen. The cost
/// is that a consumer stitching several chunk snapshots together must key
/// machines by `tile` and de-duplicate, not blindly concatenate.
///
/// `load` and `stall` (the machine-load epic, 2026-09-11) are the bottleneck
/// readout — see [`MachineLoadSnapshot`] and [`Stall`]. **They change every
/// tick without dirtying the chunk**, so a chunk-driven renderer sees them
/// only when something else dirties the machine; poll
/// [`World::machine_load_snapshots`] for a live view.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MachineSnapshot {
    pub tile: TileCoord,
    pub kind: String,
    pub direction: Direction,
    pub footprint: Footprint,
    pub recipe: RecipeId,
    /// (start_tick, required_ticks) — Godot derives progress from this plus
    /// the current tick count rather than needing a snapshot every tick.
    pub process: Option<(u64, u64)>,
    pub input: Vec<(ItemId, u32)>,
    pub output: Vec<(ItemId, u32)>,
    pub load: MachineLoadSnapshot,
    pub stall: Option<Stall>,
}

/// What share of the recent past a machine spent working, starved or
/// blocked — the [`super::machine::LoadWindow`]'s totals, plus the verdict
/// the sim draws from them. The one fact behind the andon stack light.
///
/// ```json
/// {"working": 540, "starved": 60, "blocked": 0, "observed": 600,
///  "window": 600, "percent": 90, "status": "starved"}
/// ```
///
/// - `working`/`starved`/`blocked` are tick counts over the window;
///   `observed` is their sum, at most `window`
///   ([`super::machine::LOAD_WINDOW_TICKS`]). A machine placed a moment ago
///   has observed only the ticks since.
/// - `percent` is `working * 100 / observed` in whole percent (0 when
///   nothing has been observed).
/// - `status` is `"working"`, `"starved"` or `"blocked"` by
///   [`super::machine::LoadWindow::status`]'s rule. **Godot reads this word;
///   it never re-derives it from the counts**, so the threshold lives in
///   exactly one place.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MachineLoadSnapshot {
    pub working: u32,
    pub starved: u32,
    pub blocked: u32,
    pub observed: u32,
    pub window: u32,
    pub percent: u32,
    pub status: WorkState,
}

impl MachineLoadSnapshot {
    fn of(load: &LoadWindow) -> Self {
        Self {
            working: load.working(),
            starved: load.starved(),
            blocked: load.blocked(),
            observed: load.observed(),
            window: LOAD_WINDOW_TICKS,
            percent: load.percent(),
            status: load.status(),
        }
    }
}

/// Why a machine is waiting *right now* — a structured fact for Godot to
/// phrase, computed at snapshot time from the machine's buffers, its recipe
/// and its fluid taps (never per tick: it is a question, not state).
///
/// ```json
/// {"kind": "input",          "item": "sugar"}    // solid input short
/// {"kind": "fluid_input",    "item": "vinegar"}  // tap exists, cannot supply
/// {"kind": "no_supply_pipe", "item": "vinegar"}  // recipe wants fluid, no tap
/// {"kind": "output",         "item": "ketchup"}  // finished, solid output full
/// {"kind": "fluid_output",   "item": "ketchup"}  // finished, outlet pipe cannot accept
/// {"kind": "no_outlet_pipe", "item": "ketchup"}  // finished, deposits, no tap
/// {"kind": "staff",          "item": "manufacturing"} // refused its crew; item = category id
/// ```
///
/// It describes **this tick's** situation, independently of the window's
/// `status`: a machine may be `working` over the last ten seconds and
/// momentarily short of sugar. Only the first shortfall is reported, in
/// recipe order, solids before fluids. A machine mid-craft, or idle with
/// everything it needs (it starts next tick), has no stall — `null` —
/// **unless its crew was refused** (the staff pools MVP epic): a staff
/// refusal happens only on a tick the machine would otherwise have worked,
/// so it can never mask an input or output shortfall, and it is checked
/// after the six older kinds have found nothing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Stall {
    pub kind: StallKind,
    pub item: String,
}

/// The seven ways a machine can be waiting — see [`Stall`]. Serialized in
/// `snake_case`, exactly as listed there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StallKind {
    Input,
    FluidInput,
    NoSupplyPipe,
    Output,
    FluidOutput,
    NoOutletPipe,
    /// Refused its crew on the last tick; `item` is the category id.
    Staff,
}

/// One entry of [`World::machine_load_snapshots`]: the machine's origin
/// tile (its key in every other snapshot), its load and its stall.
///
/// ```json
/// {"tile": [4, 0], "load": {...}, "stall": {"kind": "input", "item": "tomato"}}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MachineLoadEntry {
    pub tile: TileCoord,
    pub load: MachineLoadSnapshot,
    pub stall: Option<Stall>,
}

/// One pipe tile, for rendering (milestone 6 — ARCHITECTURE.md §3.4).
///
/// ```json
/// { "tile": [8, 1], "direction": "east", "volume": 7.5, "capacity": 10.0,
///   "pressure": 0.75, "is_source": false }
/// ```
///
/// `volume`/`capacity` are in whole fluid units; `pressure` is the
/// dimensionless fill fraction `volume / capacity`, in `[0, 1]` — the value
/// to drive a fill-level visual from. All three are `f64` for the same
/// reason `BeltSnapshot`'s positions are.
///
/// `direction` (new in the "Real Placement" epic) is the rotation the player
/// gave this tile. **It is presentation only** — fluid flow is undirected
/// (see [`super::fluid`]'s module doc), so unlike a belt's direction it does
/// not decide where anything goes.
///
/// `is_source` marks a tile some fluid tank discharges into. Since
/// placement it is derived from where tanks actually stand, so it moves when
/// the player moves a tank, and a run with no tank has none at all.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipeSnapshot {
    pub tile: TileCoord,
    pub direction: Direction,
    pub volume: f64,
    pub capacity: f64,
    pub pressure: f64,
    pub is_source: bool,
}

/// One raw-ingredient hopper — placement plus live fill level. The *same*
/// struct is returned by [`World::hopper_snapshots`] and carried in
/// [`ChunkSnapshot::hoppers`], so a consumer only ever learns one shape.
///
/// ```json
/// {
///   "tile": [-1, 0], "kind": "hopper", "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 1 },
///   "item": "tomato", "quantity": 12, "capacity": 64, "fill": 0.1875,
///   "auto_fill": false,
///   "belt_tile": [0, 0]
/// }
/// ```
///
/// - `tile` is the footprint's origin, `direction` the way it discharges.
/// - `fill` is `quantity / capacity` in `[0, 1]`, precomputed for the same
///   reason `PipeSnapshot::pressure` is.
/// - **`belt_tile` is now nullable.** It is the belt this hopper currently
///   unloads onto — the tile off its outlet face — and it is `null` when
///   there is no belt there, which is a perfectly ordinary state for a
///   hopper the player has just put down. Before placement a hopper was
///   built attached to the chain's first belt and this could not be absent.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HopperSnapshot {
    pub tile: TileCoord,
    pub kind: String,
    pub direction: Direction,
    pub footprint: Footprint,
    pub item: ItemId,
    pub quantity: u32,
    pub capacity: u32,
    pub fill: f64,
    /// Whether `World::tick` keeps this hopper topped up out of bought
    /// stock — [`super::hopper::Hopper::auto_fill`].
    pub auto_fill: bool,
    pub belt_tile: Option<TileCoord>,
}

/// One fluid tank — placement plus live level. Field-for-field the fluid
/// twin of [`HopperSnapshot`], deliberately, so presentation can drive both
/// from one code path: `fluid` stands where `item` does and `pipe_tile`
/// where `belt_tile` does.
///
/// ```json
/// {
///   "tile": [5, 1], "kind": "fluid_tank", "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 1 },
///   "fluid": "vinegar", "quantity": 30.0, "capacity": 120.0,
///   "fill": 0.25, "auto_fill": false, "pipe_tile": [6, 1]
/// }
/// ```
///
/// - `quantity`/`capacity` are **floating-point**, unlike the hopper's: a
///   fluid volume is continuous (pressure equalization moves partial units
///   around).
/// - **`fluid` is nullable**, and is the fluid of the network this tank
///   discharges into — which is decided by whichever machine taps that
///   network (see [`super::fluid`]). A tank connected to nothing, or to a
///   run no machine taps, reports `null`: it holds volume, and nothing has
///   yet said what that volume is.
/// - **`pipe_tile` is nullable** for [`HopperSnapshot::belt_tile`]'s reason:
///   it is the pipe tile off this tank's outlet face, and there may not be
///   one.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FluidTankSnapshot {
    pub tile: TileCoord,
    pub kind: String,
    pub direction: Direction,
    pub footprint: Footprint,
    pub fluid: Option<FluidId>,
    pub quantity: f64,
    pub capacity: f64,
    pub fill: f64,
    /// Whether `World::tick` keeps this tank topped up out of bought stock —
    /// [`FluidTank::auto_fill`].
    pub auto_fill: bool,
    pub pipe_tile: Option<TileCoord>,
}

/// One delivery dock — placement, what is on it, and how much it has
/// shipped. The exit-end counterpart of [`HopperSnapshot`].
///
/// ```json
/// {
///   "tile": [35, 0], "kind": "dock", "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 1 },
///   "belt_tile": [34, 0], "contents": [], "capacity_per_item": 32,
///   "shipped_total": 37
/// }
/// ```
///
/// Two things about it are unchanged by placement and worth re-reading:
///
/// 1. **There is no `fill` field.** The dock is a pass-through: it ships
///    everything it receives into the economy's shared delivery pool in the
///    same tick, so its own level is always zero between ticks. **The number
///    to draw a pallet stack from is `EconomySnapshot::delivery_stock`** —
///    the one shared "what's on the dock" figure every contract settles
///    against, and still one number however many docks the player builds.
/// 2. **`contents` is a list**, not one item and a count: a dock accepts
///    whatever the line delivers to it.
///
/// `shipped_total` is this dock's own cumulative output since it was placed.
/// **`belt_tile` replaces the old `belt_start_tile`/`belt_length_tiles`
/// pair**: a dock no longer owns a belt (it used to be built with one), so
/// what is reported is simply which belt tile currently feeds it, if any.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DockSnapshot {
    pub tile: TileCoord,
    pub kind: String,
    pub direction: Direction,
    pub footprint: Footprint,
    /// The belt tile that currently hands product to this dock, if any.
    pub belt_tile: Option<TileCoord>,
    pub contents: Vec<(ItemId, u32)>,
    pub capacity_per_item: u32,
    pub shipped_total: u32,
    /// Refused its logistics crew on the last tick it had something to
    /// ship (the staff pools MVP epic; `Dock::understaffed`). While true
    /// the buffer is held and, at capacity, the belt behind it backs up.
    pub understaffed: bool,
}

/// One machine's tap into a fluid network, for presentation. Used for both
/// roles — see [`FluidSnapshot::taps`] (machines drawing) and
/// [`FluidSnapshot::producers`] (machines depositing).
#[derive(Debug, Clone, serde::Serialize)]
pub struct FluidTapSnapshot {
    /// Origin tile of the machine on this end of the network.
    pub machine_tile: TileCoord,
    pub machine_recipe: RecipeId,
    /// How much the recipe moves per craft — consumed for a tap in `taps`,
    /// deposited for one in `producers`.
    pub units_per_craft: u32,
    /// Index into the network's `segments` of the tile this machine is
    /// plumbed to — the one whose level drops when a craft starts, or rises
    /// when one finishes.
    pub segment_index: usize,
}

/// One whole fluid network in one snapshot — what a HUD/overlay needs
/// without having to stitch together per-chunk pipe lists.
///
/// Read this the way `economy_snapshot` is read, **not** the way
/// `chunk_snapshot` is: fluid volume changes on almost every tick and
/// deliberately does **not** mark its chunk dirty (see [`World::tick`]), so
/// `dirty_chunks` will not tell you when fluid moved. Chunk snapshots still
/// carry pipes so a chunk-driven renderer can place the geometry; this is
/// where the live state lives.
///
/// ```json
/// {
///   "fluid": "vinegar",
///   "total_volume": 27.5, "total_capacity": 40.0, "pressure": 0.6875,
///   "segments": [ { "tile": [5, 1], "direction": "east", "volume": 10.0,
///                   "capacity": 10.0, "pressure": 1.0, "is_source": true } ],
///   "taps": [ { "machine_tile": [9, 0], "machine_recipe": "ketchup_mix",
///               "units_per_craft": 2, "segment_index": 3 } ],
///   "producers": [],
///   "tank_tiles": [[5, 1]]
/// }
/// ```
///
/// **This shape is a breaking change from the pre-placement one.** There
/// used to be exactly one network per world, so its snapshot named its one
/// machine inline (`machine_tile`/`machine_recipe`/`units_per_craft`) and
/// its one inlet (`inlet_index`), and `segments` was in flow order because
/// the run was a straight line built by the chain builder. Now there is one
/// of these per connected component of placed pipe, a component may be any
/// shape and may be tapped by several machines or none, so: `taps` is an
/// array (possibly empty), `tank_tiles` names the tanks feeding it (possibly
/// none), `fluid` is nullable, and **`segments` is in tile order, not flow
/// order** — adjacency is not "consecutive entries" any more, and a
/// consumer that needs it should read it off the tiles.
///
/// `total_volume`/`total_capacity`/`pressure` describe the **pipes alone**,
/// as they always have: a tank is a building with its own snapshot, not a
/// length of pipe.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FluidSnapshot {
    pub fluid: Option<FluidId>,
    pub total_volume: f64,
    pub total_capacity: f64,
    /// Network-wide fill fraction, `total_volume / total_capacity`.
    pub pressure: f64,
    pub segments: Vec<PipeSnapshot>,
    pub taps: Vec<FluidTapSnapshot>,
    /// Machines **depositing** into this network — the chain-rework epic's
    /// addition, and the reason a run can now have fluid in it with no tank
    /// anywhere. Without it a consumer reading `tank_tiles: []` alongside a
    /// rising volume would have no way to say where the liquid comes from.
    pub producers: Vec<FluidTapSnapshot>,
    pub tank_tiles: Vec<TileCoord>,
}

/// What occupies a single tile, for [`World::query_tile`].
///
/// Every variant gained a `direction` in the "Real Placement" epic, and
/// multi-tile things still resolve from **any** tile of their footprint, not
/// only the origin — which is what makes hovering, rotating and removing a
/// big machine work anywhere on it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TileSnapshot {
    /// `{"kind": "belt", "direction": "east", "items_here": ["tomato"],
    ///   "links": {"inputs": [{"from": "west", "kind": "belt"}],
    ///             "output": {"to": "east", "kind": "machine"}}}` —
    /// one tile, so `items_here` is simply everything on it. `links` is
    /// exactly the placement's [`ConveyorLinks`], so hover can say what
    /// feeds a tile and where it hands to without a second call.
    Belt {
        direction: Direction,
        items_here: Vec<ItemId>,
        links: ConveyorLinks,
    },
    /// `{"kind": "splitter", "direction": "east",
    ///   "items_here": ["tomato"], "outputs": [[7, 0], [6, -1]],
    ///   "next_output": "straight", "links": {...}}` — enough for a hover
    /// tooltip ("Splitter — next: straight"). `outputs` and `next_output`
    /// mean exactly what they do in [`SplitterSnapshot`]; `links` is the
    /// placement's [`ConveyorLinks`] (`output` = the straight side).
    Splitter {
        direction: Direction,
        items_here: Vec<ItemId>,
        outputs: [Option<TileCoord>; 2],
        next_output: Side,
        links: ConveyorLinks,
    },
    /// `{"kind": "machine", "machine_kind": "cooking_vat",
    ///   "direction": "east",
    ///   "footprint": {"width_tiles": 2, "height_tiles": 2},
    ///   "origin_tile": [14, 0], "recipe": "ketchup",
    ///   "process": [120, 240],
    ///   "load": {"working": 600, "starved": 0, "blocked": 0,
    ///            "observed": 600, "window": 600, "percent": 100,
    ///            "status": "working"},
    ///   "stall": null,
    ///   "crew": [{"category": "manufacturing", "count": 2}]}`
    ///
    /// `machine_kind` is spelled out rather than reusing `kind`, which this
    /// enum already spends on its own variant tag. `load` and `stall` are
    /// the same objects [`MachineSnapshot`] carries, so hover can say how
    /// loaded a machine is and why it waits without a second call. `crew`
    /// is the kind's `machines.ron` `staff` requirement (the staff pools
    /// MVP epic), for hover's "crew: 2 manufacturing".
    Machine {
        machine_kind: String,
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        recipe: RecipeId,
        process: Option<(u64, u64)>,
        load: MachineLoadSnapshot,
        stall: Option<Stall>,
        crew: Vec<CrewEntry>,
    },
    /// `{"kind": "pipe", "direction": "east", "fluid": "vinegar",
    ///   "volume": 7.5, "capacity": 10.0, "pressure": 0.75,
    ///   "is_source": false, "is_inlet": true,
    ///   "links": {"connections": [{"to": "west", "kind": "pipe"}]}}` —
    /// enough for a hover tooltip without a second call. `is_inlet` is
    /// whether some machine taps this exact tile; `links` is the
    /// placement's [`PipeLinks`].
    Pipe {
        direction: Direction,
        fluid: Option<FluidId>,
        volume: f64,
        capacity: f64,
        pressure: f64,
        is_source: bool,
        is_inlet: bool,
        links: PipeLinks,
    },
    /// `{"kind": "hopper", "direction": "east",
    ///   "footprint": {...}, "origin_tile": [-1, 0], "item": "tomato",
    ///   "quantity": 12, "capacity": 64, "fill": 0.1875,
    ///   "auto_fill": false}` — enough for a
    /// hover tooltip ("Hopper — 12/64 tomato"). There is no `machine_kind`
    /// field: this enum's own `kind` tag already *is* the kind id.
    Hopper {
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        item: ItemId,
        quantity: u32,
        capacity: u32,
        fill: f64,
        auto_fill: bool,
    },
    /// `{"kind": "fluid_tank", "direction": "east", "footprint": {...},
    ///   "origin_tile": [5, 1], "fluid": "vinegar", "quantity": 30.0,
    ///   "capacity": 120.0, "fill": 0.25, "auto_fill": false}`. `fluid` is
    /// nullable for
    /// [`FluidTankSnapshot`]'s reason.
    FluidTank {
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        fluid: Option<FluidId>,
        quantity: f64,
        capacity: f64,
        fill: f64,
        auto_fill: bool,
    },
    /// `{"kind": "dock", "direction": "east", "footprint": {...},
    ///   "origin_tile": [35, 0], "contents": [], "capacity_per_item": 32,
    ///   "shipped_total": 37}` — enough for a hover tooltip ("Loading dock —
    /// 37 shipped"). There is deliberately no `fill` here and `contents` is
    /// normally empty; see [`DockSnapshot`].
    Dock {
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        contents: Vec<(ItemId, u32)>,
        capacity_per_item: u32,
        shipped_total: u32,
        /// The kind's `staff` requirement and whether the dock was refused
        /// it on the last tick it held something (the staff pools MVP
        /// epic) — hover's "crew: 1 logistics" and "held · short of
        /// logistics staff".
        crew: Vec<CrewEntry>,
        understaffed: bool,
    },
}

/// What stands on the far end of a link, for [`Links`] — the seven kinds a
/// belt, splitter or pipe tile can be joined to, as the wire names them.
///
/// This is deliberately its own enum rather than a `String` kind id: a
/// link is about *what kind of thing* is there (a machine, whichever
/// machine), not which content id it runs, and a closed set is what lets a
/// renderer match on it exhaustively. `BodyKind` is the private twin;
/// the one difference in spelling is `tank` for `fluid_tank`, because the
/// plan's wire contract names it that way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Belt,
    Splitter,
    Hopper,
    Machine,
    Dock,
    Pipe,
    Tank,
}

impl From<BodyKind> for LinkKind {
    fn from(body: BodyKind) -> Self {
        match body {
            BodyKind::Belt => LinkKind::Belt,
            BodyKind::Splitter => LinkKind::Splitter,
            BodyKind::Pipe => LinkKind::Pipe,
            BodyKind::Machine => LinkKind::Machine,
            BodyKind::Hopper => LinkKind::Hopper,
            BodyKind::FluidTank => LinkKind::Tank,
            BodyKind::Dock => LinkKind::Dock,
        }
    }
}

/// Something the topology pass links **into** a belt-like tile:
/// `{"from": "west", "kind": "belt"}`. `from` is the absolute direction
/// from this tile toward the neighbour that feeds it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LinkInput {
    pub from: Direction,
    pub kind: LinkKind,
}

/// The sink a belt-like tile hands to: `{"to": "east", "kind": "machine"}`.
/// `to` is the absolute direction from this tile toward the sink.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LinkOutput {
    pub to: Direction,
    pub kind: LinkKind,
}

/// One thing a pipe tile is joined to: `{"to": "north", "kind": "pipe"}`.
/// `to` is the absolute direction from this tile toward it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PipeConnection {
    pub to: Direction,
    pub kind: LinkKind,
}

/// A belt's or splitter's links (the Pipes & Conveyors Overhaul, ticket 1
/// — `docs/PIPES_AND_CONVEYORS_PLAN.md`, "The wire contract").
///
/// ```json
/// { "inputs": [ {"from": "west", "kind": "belt"},
///               {"from": "south", "kind": "hopper"} ],
///   "output": {"to": "east", "kind": "machine"} }
/// ```
///
/// `inputs` is everything the topology pass links **into** this tile — a
/// belt or splitter pointing at it, a hopper or machine discharging onto it
/// — in a fixed order: north, east, south, west by `from`. At most one
/// thing can link in from each side, so that order is a property of the
/// geometry, not of which placement the pass happened to walk first.
/// `output` is the one sink this tile hands to, `null` when nothing that
/// can accept an item stands where it points. For a splitter this is the
/// **straight** output; the branch stays in [`SplitterSnapshot::outputs`].
///
/// These are the same links the tick uses (`Topology::belt_to_belt`,
/// `belt_to_sink`, `source_to_belt`, `splitter_outputs`), recorded once in
/// the same pass, so they are exact by construction: a renderer that
/// draws a corner from `inputs` is drawing where the item will actually
/// come from, and it costs nothing per tick.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct ConveyorLinks {
    pub inputs: Vec<LinkInput>,
    pub output: Option<LinkOutput>,
}

/// A pipe tile's links (same ticket).
///
/// ```json
/// { "connections": [ {"to": "north", "kind": "machine"},
///                    {"to": "east",  "kind": "pipe"},
///                    {"to": "south", "kind": "tank"},
///                    {"to": "west",  "kind": "pipe"} ] }
/// ```
///
/// One entry per thing this tile is joined to, in north, east, south, west
/// order: every orthogonally adjacent pipe tile in the **same** network
/// (which, since a network *is* a connected component of adjacent pipe,
/// is every orthogonally adjacent pipe tile); the tank whose discharge
/// this tile is; and a machine whose input tap or output tap this exact
/// tile is. **Only the tap tile gets the machine entry.** A machine draws
/// from one segment and deposits into one segment (`Topology::machine_taps`
/// / `machine_output_taps`), so a run laid along a machine's whole side
/// connects to it on one tile, and that is the tile a port fitting belongs
/// on — the wire reports where the fluid actually crosses, not every
/// place the two happen to touch.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct PipeLinks {
    pub connections: Vec<PipeConnection>,
}

/// The `links` object on a [`PlacementSnapshot`]: a belt or splitter
/// carries [`ConveyorLinks`], a pipe carries [`PipeLinks`], and every
/// other kind carries nothing (the field is omitted, so those placements
/// serialize byte-for-byte as they did before the field existed).
/// Serialized untagged — the two shapes share no field name, and a
/// consumer already knows which it is reading from the placement's `kind`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum Links {
    Conveyor(ConveyorLinks),
    Pipe(PipeLinks),
}

/// One placed thing, statically described — the unit
/// [`World::world_layout`] reports.
///
/// **This replaces `StageLayout`, and `world_layout` replaces
/// `chain_layout`** (a breaking change for presentation). A stage was a
/// belt-plus-machine pair in a fixed chain, and neither of those concepts
/// survives placement: there is no chain, there is no stage, and a belt is
/// no longer owned by the machine downstream of it. What a renderer
/// actually needs — and all it ever used `chain_layout` for — is "what is
/// on the floor, where, facing which way, and how big is it," which is what
/// this is, for every placement in the world rather than only the
/// recipe-driven ones.
///
/// ```json
/// {
///   "kind": "cooking_vat",
///   "tile": [14, 0],
///   "direction": "east",
///   "footprint": { "width_tiles": 2, "height_tiles": 2 },
///   "occupies": [[14, 0], [15, 0], [14, 1], [15, 1]],
///   "recipe": "ketchup"
/// }
/// ```
///
/// `footprint` is the kind's **declared** rectangle and `occupies` is what
/// it actually covers once rotated — both are given so a consumer can pick
/// whichever it needs without re-deriving the rotation rule. `recipe` is
/// `null` for everything that runs none: belts, pipes, hoppers, tanks and
/// docks.
///
/// **`links` (the Pipes & Conveyors Overhaul, ticket 1)** is present on
/// every belt, splitter and pipe and **absent** — not `null`, absent — on
/// every other kind, so a machine's or hopper's entry is byte-identical to
/// what it was before the field existed. See [`Links`] for both shapes:
///
/// ```json
/// { "kind": "belt", "tile": [0, 0], "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 1 },
///   "occupies": [[0, 0]], "recipe": null,
///   "links": { "inputs": [ {"from": "west", "kind": "hopper"} ],
///              "output": {"to": "east", "kind": "machine"} } }
/// ```
///
/// Unlike `chain_layout`, **this is not static for the life of the world** —
/// it changes whenever the player places, rotates or removes something. Call
/// it again after issuing any of those three commands (and after a
/// `dirty_chunks` report, if the renderer is chunk-driven). `links` changes
/// exactly when the rest does, since both come out of the same topology
/// pass.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlacementSnapshot {
    pub kind: String,
    pub tile: TileCoord,
    pub direction: Direction,
    pub footprint: Footprint,
    pub occupies: Vec<TileCoord>,
    pub recipe: Option<RecipeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
}

/// What kind of thing a placed entity is — the one component every
/// placement carries besides [`Position`] and [`Placement`], so the
/// topology pass can classify a tile's owner without probing for each
/// possible body type in turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind {
    Belt,
    /// A conveyor splitter (see [`super::splitter`]). Belt-*like*: it
    /// carries its contents in an ordinary one-tile [`BeltSegment`] and
    /// accepts a hand-off from anything a belt accepts one from, so every
    /// existing transfer function works on it verbatim. What makes it its
    /// own kind rather than a flag on `Belt` is the other end: it resolves
    /// to **two** downstream targets instead of one, which no other
    /// placement in this simulation does.
    Splitter,
    Pipe,
    Machine,
    Hopper,
    FluidTank,
    Dock,
}

/// Everything derived from *where things stand*, rebuilt whenever that
/// changes and never otherwise (ARCHITECTURE.md §3.8's "once per topology
/// change, not every tick").
///
/// Every list here is in a deterministic order — placements sorted by origin
/// tile, links in the order of their source placement — because tick order
/// is simulation behaviour and ARCHITECTURE.md §3.9 requires it to be
/// reproducible. Nothing in this struct is ever iterated out of a `HashMap`.
#[derive(Debug, Default)]
struct Topology {
    /// Every occupied tile and the entity claiming it. A multi-tile
    /// placement appears once per tile it covers, all mapping to the same
    /// entity — which is what makes "any tile of it works" true for
    /// `query_tile`, `Remove` and `Rotate` alike.
    grid: BTreeMap<TileCoord, Entity>,
    /// Every placement, in tile order — the order `world_layout` reports and
    /// `all_chunks` walks.
    all: Vec<Entity>,
    belts: Vec<Entity>,
    splitters: Vec<Entity>,
    pipes: Vec<Entity>,
    machines: Vec<Entity>,
    hoppers: Vec<Entity>,
    tanks: Vec<Entity>,
    docks: Vec<Entity>,
    /// A belt hands off to the belt (or splitter) it points at.
    belt_to_belt: Vec<(Entity, Entity)>,
    /// The same relation as `belt_to_belt`, indexed like `belts`: entry
    /// `i` is what `belts[i]` hands to if that is a belt or a splitter,
    /// else `None`. The belt tick reads the receiver's room at its start
    /// through this (`belt.rs`'s "the footprint does not stop at the belt's
    /// end"), and a per-belt search of `belt_to_belt` every tick would be
    /// quadratic in the line's length.
    belt_ahead: Vec<Option<Entity>>,
    /// **The one one-to-many relation in this simulation**: a splitter and
    /// the two things standing on its output tiles — straight-through
    /// first, then the branch one quarter turn clockwise (see
    /// [`super::splitter`]). Either entry is `None` when nothing that can
    /// accept an item is there, which is what makes "a splitter with one
    /// output feeds that one" and "a splitter with none holds everything"
    /// fall out of the ordinary offer loop rather than being special cases.
    splitter_outputs: Vec<(Entity, [Option<Entity>; 2])>,
    /// A belt hands off into a machine's inlet face, or into a dock.
    belt_to_sink: Vec<(Entity, Entity)>,
    /// A machine or hopper discharges onto the belt off its outlet face.
    source_to_belt: Vec<(Entity, Entity)>,
    /// One per connected component of placed pipe.
    networks: Vec<FluidNetwork>,
    /// Which network and which segment each pipe tile is, so a tile query
    /// doesn't have to search every network.
    pipe_index: BTreeMap<TileCoord, (usize, usize)>,
    /// `(machine, network index, segment index)` for every fluid-consuming
    /// machine that touches a pipe — the network it **draws** from.
    machine_taps: Vec<(Entity, usize, usize)>,
    /// The same, for the network a fluid-producing machine **deposits**
    /// into: the first pipe off its outlet face. Kept as a separate list
    /// rather than a second field on `machine_taps` because the two are
    /// derived by different rules (touching vs. facing — see this module's
    /// doc) and a machine may have either, both or neither.
    machine_output_taps: Vec<(Entity, usize, usize)>,
    /// `(tank, network index, segment index)` for every tank discharging
    /// into a pipe.
    tank_discharges: Vec<(Entity, usize, usize)>,
    /// **The wire's view of every relation above**, keyed on the belt,
    /// splitter or pipe it describes: which sides feed it, which side it
    /// hands to, or which sides its pipe is joined on (see [`Links`]).
    /// Filled in the same pass, at the same moment each relation is
    /// resolved, so it can never disagree with the lists the tick reads —
    /// and never rescanned: `world_layout` and `query_tile` look it up.
    /// Every belt-like entity and every pipe has an entry, empty or not.
    links: BTreeMap<Entity, Links>,
}

pub struct World {
    ecs: hecs::World,
    recipes: RecipeRegistry,
    machines: MachineRegistry,
    /// Which recipe a machine *kind* runs — the 1:1 inverse of
    /// `Recipe.machine`, built once at construction. A placed machine runs
    /// this and nothing else: choosing a recipe per machine is a mechanic
    /// nobody has asked for, and `data/recipes.ron` has always had exactly
    /// one recipe per kind (construction panics if that stops being true,
    /// rather than silently picking one).
    recipe_for_kind: HashMap<String, RecipeId>,
    /// Every **raw** item the chain consumes: an item some recipe takes as a
    /// solid input and no recipe produces
    /// ([`super::ingredients::raw_ingredient_ids`], filtered to solids
    /// here), in order of first appearance.
    ///
    /// This replaced a single `raw_item: Option<ItemId>` computed as "the
    /// first recipe's first input, and nothing else is ever considered raw."
    /// That was a correct reading of a chain with exactly one raw
    /// ingredient and stopped being one the moment the mixer gained sugar:
    /// it would have named `tomato` alone, leaving `sugar` unbuyable, no
    /// hopper able to hold it, and the line unable to run.
    ///
    /// Empty for a recipe file with no raw solid at all, in which case a
    /// hopper cannot be placed (rather than the world refusing to be built).
    raw_items: Vec<ItemId>,
    tick_count: u64,
    belt_speed: Fx,
    /// Scratch for the belt tick: how much room each carrier's downstream
    /// carrier has at its start, read for every belt and splitter BEFORE
    /// any of them moves (`BeltSegment::tick_toward`'s order-independence
    /// rule). Indexed like `topology.belts` then `topology.splitters`;
    /// cleared, not reallocated, every tick, like `dirty`.
    room_ahead: Vec<Fx>,
    /// Chunks touched since the start of the current `tick()` call. Cleared
    /// (not reallocated) at the start of every `tick()` — see `tick`'s doc
    /// comment and ARCHITECTURE.md §4's "Delta granularity" note. A
    /// `Place`/`Remove`/`Rotate` between ticks accumulates into it too.
    dirty: Vec<ChunkId>,
    /// Money + every active contract's delivery state (GAMEPLAY.md §6/§9).
    economy: Economy,
    /// What the player can buy and what they own of it. Since this epic an
    /// owned count is real inventory: `Place` spends one and `Remove`
    /// credits one back.
    shop: Shop,
    /// The raw-ingredient market and the stock the player has bought out of
    /// it (GAMEPLAY.md §5). A plain field beside `shop` and `economy` for
    /// their reason: one per world.
    ingredients: Ingredients,
    /// The staff pools (the staff pools MVP epic, GAMEPLAY.md §7): a plain
    /// field like `economy`, for the same reason — four aggregate counters
    /// are not a reason for ECS machinery.
    staff: StaffPools,
    topology: Topology,
    /// What the factory made and used, per item, over several rolling
    /// windows and for all time (the production-stats epic, `stats.rs`).
    /// Written by `Machine::tick` and by a contract payout in step 7 of
    /// [`World::tick`]; read by [`World::production_snapshot`]. Polled,
    /// never dirtying a chunk, like the load window.
    production: ProductionLedger,
}

/// How far an item advances along a belt tile per tick. Unchanged from the
/// pre-placement chain — a tile is one unit long, so an item still crosses
/// one tile in 50 ticks and a four-tile run in 200, exactly as it did when
/// that run was a single four-tile `BeltSegment`.
const BELT_SPEED_TILES_PER_TICK: f64 = 0.02;
/// One placed belt tile is one unit of belt. (`belt.rs` requires
/// `speed <= min_spacing`, which the two constants above and below satisfy.)
const BELT_TILE_LENGTH: f64 = 1.0;
/// The closest two items on a belt can stand, centre to centre, and so how
/// many a fully compressed tile holds: **two**, one every half tile, since
/// the quality pass of 2026-09-13 (it was a quarter tile, four to five per
/// tile). Chosen by rendering: items are modelled at half a tile
/// (`assets.rs`'s `ITEM_VOXELS`, the widest — the box — 12 of 32 voxels),
/// so a quarter-tile spacing drew a jammed lane as one continuous bar of
/// product, and at a half tile a jammed lane reads as a queue of separate
/// objects with a few voxels of belt showing between each pair. Wider still
/// read as a trickle. The belt's speed is unchanged — only the spacing moved
/// — so a belt now carries `BELT_SPEED_TILES_PER_TICK / this` items per
/// tick at most: one every 25 ticks, forty per thousand, still well over
/// what the shipped line's slowest stage asks of it (`tests/core_loop.rs`
/// pins that the line's rate is the vat's, not the belt's). Exact in `Fx`.
/// Stated in prose in CLAUDE.md's decisions log and `docs/DECISIONS.md`;
/// change them together.
const ITEM_MIN_SPACING_TILES: f64 = 0.5;
const MACHINE_INPUT_CAPACITY: u32 = 8;
const MACHINE_OUTPUT_CAPACITY: u32 = 8;
/// Volume one pipe tile holds, in whole fluid units.
const PIPE_SEGMENT_CAPACITY: i32 = 10;

impl World {
    /// [`ITEM_MIN_SPACING_TILES`] as the `Fx` every belt is built with.
    fn item_min_spacing(&self) -> Fx {
        Fx::from_num(ITEM_MIN_SPACING_TILES)
    }

    /// An **empty** world: no belts, no machines, no buildings, and
    /// [`STARTING_MONEY`] in the bank. Everything in a factory is something
    /// the player buys ([`PlayerCommand::Purchase`]) and places
    /// ([`PlayerCommand::Place`]) — see this module's doc for why that
    /// replaced the fixed recipe-order chain.
    ///
    /// Machine sizes come from the shipped `data/machines.ron` and the
    /// contract board from the shipped `data/contracts.ron`; use
    /// [`World::with_machines`] or [`World::with_content`] to supply a
    /// different set of either.
    pub fn new(recipes: RecipeRegistry) -> Self {
        Self::with_machines(recipes, MachineRegistry::bundled())
    }

    /// [`World::new`], but with an explicit machine-kind registry instead of
    /// the bundled `data/machines.ron`. This is the seam that lets a test
    /// exercise a footprint no shipped kind has — every shipped kind is 1x1
    /// or square, so rotation's extent-swap would otherwise be unobservable
    /// on real content.
    pub fn with_machines(recipes: RecipeRegistry, machines: MachineRegistry) -> Self {
        Self::with_content(recipes, machines, ContractRegistry::bundled())
    }

    /// [`World::with_machines`], but with an explicit contract board instead
    /// of the bundled `data/contracts.ron` — the seam a test uses to drive
    /// cadences and mixes the shipped content doesn't have.
    pub fn with_content(
        recipes: RecipeRegistry,
        machines: MachineRegistry,
        contracts: ContractRegistry,
    ) -> Self {
        Self::with_content_and_money(recipes, machines, contracts, Fx::from_num(STARTING_MONEY))
    }

    /// [`World::with_content`], but starting from a chosen balance instead
    /// of [`STARTING_MONEY`].
    ///
    /// The seam [`super::reference::reference_chain`] uses to buy a whole
    /// factory in one go, and where a loaded save will restore its balance.
    /// Deliberately a constructor argument rather than a "give me money"
    /// method: money only ever moves through [`Economy::try_spend`] and a
    /// contract payout once a world exists, and adding a second way in would
    /// be a cheat surface on the real API.
    pub fn with_content_and_money(
        recipes: RecipeRegistry,
        machines: MachineRegistry,
        contracts: ContractRegistry,
        money: Fx,
    ) -> Self {
        let ordered = recipes.ordered();
        assert!(
            !ordered.is_empty(),
            "recipes.ron must define at least one recipe"
        );
        // Still enforced, though nothing lays out a chain any more: it is
        // what makes the shipped file a coherent production line, what
        // `reference_chain` builds against, and what makes "the first
        // recipe's first input is the raw ingredient" true.
        validate_chain(ordered);
        validate_fluid_scope(ordered);
        // Every recipe's machine kind must exist in machines.ron — a
        // missing kind has no footprint, so a machine of that kind could
        // never be placed and its recipe would be silently unreachable.
        // Same construction-time-panic rule as `validate_chain`.
        if let Err(e) = machines.validate_covers(&recipes) {
            panic!("machine content error: {e}");
        }

        let mut recipe_for_kind: HashMap<String, RecipeId> = HashMap::new();
        for recipe in ordered {
            if let Some(existing) = recipe_for_kind.get(&recipe.machine) {
                panic!(
                    "recipes.ron content error: machine kind '{}' runs both '{existing}' and \
                     '{}' — a placed machine runs the one recipe its kind names (world.rs's \
                     `recipe_for_kind`), so two recipes on one kind would need a \
                     recipe-selection mechanic that does not exist",
                    recipe.machine, recipe.id
                );
            }
            recipe_for_kind.insert(recipe.machine.clone(), recipe.id.clone());
        }
        // Raw = consumed somewhere, produced nowhere. The fluid raws
        // (vinegar) are bought the same way but are not hopper contents, so
        // only the solids are kept here.
        let solid_inputs: Vec<&str> = ordered
            .iter()
            .flat_map(|r| r.inputs.iter().map(|(item, _)| item.as_str()))
            .collect();
        let raw_items: Vec<ItemId> = raw_ingredient_ids(&recipes)
            .into_iter()
            .filter(|id| solid_inputs.contains(&id.as_str()))
            .collect();

        Self {
            ecs: hecs::World::new(),
            recipes,
            machines,
            recipe_for_kind,
            raw_items,
            tick_count: 0,
            belt_speed: Fx::from_num(BELT_SPEED_TILES_PER_TICK),
            room_ahead: Vec::new(),
            dirty: Vec::new(),
            economy: Economy::with_money(&contracts, money),
            shop: Shop::bundled(),
            ingredients: Ingredients::bundled(),
            staff: StaffPools::bundled(),
            topology: Topology::default(),
            production: ProductionLedger::new(),
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }

    /// Current money balance (ARCHITECTURE.md §3.9: fixed-point, never a
    /// float, since it's shared/persisted simulation state).
    pub fn money(&self) -> Fx {
        self.economy.money
    }

    /// The live economy — money, the shared delivery pool, and every active
    /// contract's own cadence state. Read-only; contracts advance through
    /// [`World::tick`], never by a caller poking at them.
    pub fn economy(&self) -> &Economy {
        &self.economy
    }

    /// Presentation-facing money + contract-status snapshot (ARCHITECTURE.md
    /// §4/§5) — always current, unlike `chunk_snapshot` there's no
    /// dirty-tracking to consult first.
    pub fn economy_snapshot(&self) -> EconomySnapshot {
        self.economy.snapshot()
    }

    /// The production ledger — what the factory has made and used, per
    /// item, over the rolling windows in `stats::STAT_WINDOWS` and for all
    /// time. Read-only; it is written by the tick, never by a caller.
    pub fn production(&self) -> &ProductionLedger {
        &self.production
    }

    /// The presentation-facing view of [`World::production`] — see
    /// [`ProductionSnapshot`] for the JSON shape. **Poll this**; it changes
    /// every tick a machine runs and deliberately dirties nothing.
    pub fn production_snapshot(&self) -> ProductionSnapshot {
        self.production.snapshot()
    }

    /// The shop catalog, the player's owned counts and their current money
    /// in one object — see [`ShopSnapshot`] for the exact JSON shape.
    pub fn shop_snapshot(&self) -> ShopSnapshot {
        self.shop
            .snapshot(self.economy.money, |kind| crew_entries(self.crew_of_kind(kind)))
    }

    /// The staff pools: headcounts, the last tick's demand and the payday
    /// clock (the staff pools MVP epic).
    pub fn staff(&self) -> &StaffPools {
        &self.staff
    }

    /// The staff panel's one poll — see [`StaffSnapshot`]. `money` is
    /// repeated here as `shop_snapshot` repeats it.
    pub fn staff_snapshot(&self) -> StaffSnapshot {
        self.staff.snapshot(self.economy.money)
    }

    /// The crew `kind` needs while running: its `machines.ron` `staff`
    /// list, empty for a kind the registry does not know (belt, pipe).
    pub fn crew_of_kind(&self, kind: &str) -> &[(String, u32)] {
        self.machines
            .get(kind)
            .map_or(&[], |k| k.staff.as_slice())
    }

    /// The raw-ingredient market plus the stock the player owns and their
    /// money, in one object — see [`IngredientsSnapshot`].
    pub fn ingredients_snapshot(&self) -> IngredientsSnapshot {
        self.ingredients
            .snapshot(self.economy.money, &self.raw_items)
    }

    /// How many units of a raw ingredient the player owns and has **not**
    /// loaded. The ingredient twin of [`Self::owned_count`].
    pub fn owned_ingredient(&self, item: &str) -> u32 {
        self.ingredients.owned.count(item)
    }

    /// Every raw solid item this world's recipes need, in order of first
    /// appearance — the set a hopper's contents must be drawn from. See
    /// [`Self::raw_items`]'s field doc for what "raw" means.
    pub fn raw_items(&self) -> &[ItemId] {
        &self.raw_items
    }

    /// How many of `kind` the player owns and has **not** placed. A thin
    /// read-only accessor for tests and for a build menu that wants one
    /// number; presentation should use [`Self::shop_snapshot`], which
    /// answers for every kind at once.
    pub fn owned_count(&self, kind: &str) -> u32 {
        self.shop.owned.count(kind)
    }

    /// How many placements exist in the world at all — the cheap "is
    /// anything built yet" question a fresh world answers with zero.
    pub fn placement_count(&self) -> usize {
        self.topology.all.len()
    }

    // -----------------------------------------------------------------
    // Placement
    // -----------------------------------------------------------------

    /// The footprint a kind is placed with: `data/machines.ron`'s for
    /// anything that file defines, one tile for the belt and the pipe, which
    /// are infrastructure rather than machines (see [`BELT_KIND`]) — and
    /// `None` for a kind that is neither, which is what makes an unknown
    /// kind an unplaceable one.
    fn footprint_for(&self, kind: &str) -> Option<Footprint> {
        if kind == BELT_KIND || kind == PIPE_KIND {
            return Some(Footprint::ONE_BY_ONE);
        }
        self.machines.footprint_of(kind)
    }

    /// Whether `kind` is something this world knows how to construct at all
    /// — a buildable kind in the shop catalog, with a footprint, and (for a
    /// machine) a recipe to run.
    fn can_construct(&self, kind: &str) -> bool {
        if self.shop.catalog.get(kind).is_none() || self.footprint_for(kind).is_none() {
            return false;
        }
        match kind {
            BELT_KIND | PIPE_KIND | SPLITTER_KIND | DOCK_KIND | FLUID_TANK_KIND => true,
            HOPPER_KIND => !self.raw_items.is_empty(),
            other => self.recipe_for_kind.contains_key(other),
        }
    }

    /// Which item a hopper being placed will hold, given what the player
    /// asked for (if anything). `None` means "refuse the placement".
    ///
    /// This is where [`PlayerCommand::Place`]'s optional-when-inferable rule
    /// is actually decided, in one place, so the wire form and the pure
    /// command agree by construction:
    ///
    /// - a named item is honoured **only if it is genuinely raw** — a hopper
    ///   full of bottled ketchup is not a thing to allow by omission of a
    ///   check;
    /// - an omitted item falls back to the world's single raw item when
    ///   there is exactly one, which is what every pre-sugar world had and
    ///   why nothing about that world's behaviour changed;
    /// - an omitted item in a world with **several** raw items is a refusal,
    ///   because the simulation genuinely does not know which bin the player
    ///   meant and picking the first would silently choose for them.
    fn hopper_item_for(&self, requested: Option<&str>) -> Option<ItemId> {
        match requested {
            Some(item) => self.raw_items.iter().find(|raw| *raw == item).cloned(),
            None => match self.raw_items.as_slice() {
                [only] => Some(only.clone()),
                _ => None,
            },
        }
    }

    /// Places one owned unit of `kind`, returning whether it happened. See
    /// [`PlayerCommand::Place`] for the refusal rules — every one of them
    /// leaves the world byte-identical.
    fn try_place(
        &mut self,
        kind: &str,
        tile: TileCoord,
        direction: Direction,
        item: Option<&str>,
    ) -> bool {
        if self.shop.owned.count(kind) == 0 || !self.can_construct(kind) {
            return false;
        }
        // Resolved before anything is spawned, so an ambiguous or bogus
        // hopper item is a refusal that changes nothing rather than a
        // half-built placement.
        let hopper_item = if kind == HOPPER_KIND {
            match self.hopper_item_for(item) {
                Some(resolved) => Some(resolved),
                None => return false,
            }
        } else {
            None
        };
        let footprint = self
            .footprint_for(kind)
            .expect("can_construct checked the footprint exists");
        let tiles = footprint.tiles_facing(tile, direction);
        if tiles.iter().any(|t| self.topology.grid.contains_key(t)) {
            return false;
        }

        let placement = Placement {
            kind: kind.to_string(),
            footprint,
            direction,
        };
        let position = Position { tile };
        let entity = match kind {
            BELT_KIND => self.ecs.spawn((
                position,
                placement,
                BodyKind::Belt,
                BeltSegment::new(
                    Fx::from_num(BELT_TILE_LENGTH),
                    Fx::from_num(ITEM_MIN_SPACING_TILES),
                ),
            )),
            // A splitter is a one-tile belt segment plus a cursor saying
            // which output is due — see `splitter.rs`'s module doc for why
            // the buffer is a `BeltSegment` and not a bespoke slot. Same
            // length and spacing as a belt tile, deliberately: dropping a
            // splitter into a line must not change how fast product moves
            // through it.
            SPLITTER_KIND => self.ecs.spawn((
                position,
                placement,
                BodyKind::Splitter,
                BeltSegment::new(
                    Fx::from_num(BELT_TILE_LENGTH),
                    Fx::from_num(ITEM_MIN_SPACING_TILES),
                ),
                Splitter::new(),
            )),
            PIPE_KIND => self.ecs.spawn((
                position,
                placement,
                BodyKind::Pipe,
                PipeTile {
                    capacity: Fx::from_num(PIPE_SEGMENT_CAPACITY),
                },
            )),
            HOPPER_KIND => {
                let item =
                    hopper_item.expect("the hopper's item was resolved before anything spawned");
                self.ecs.spawn((
                    position,
                    placement,
                    BodyKind::Hopper,
                    Hopper::new(item, HOPPER_CAPACITY),
                ))
            }
            FLUID_TANK_KIND => self.ecs.spawn((
                position,
                placement,
                BodyKind::FluidTank,
                FluidTank::new(Fx::from_num(FLUID_TANK_CAPACITY)),
            )),
            DOCK_KIND => self.ecs.spawn((
                position,
                placement,
                BodyKind::Dock,
                Dock::new(DOCK_CAPACITY),
            )),
            machine_kind => {
                let recipe = self.recipe_for_kind[machine_kind].clone();
                self.ecs.spawn((
                    position,
                    placement,
                    BodyKind::Machine,
                    Machine::new(recipe, MACHINE_INPUT_CAPACITY, MACHINE_OUTPUT_CAPACITY),
                ))
            }
        };
        debug_assert!(self.ecs.contains(entity));

        let taken = self.shop.owned.take_one(kind);
        debug_assert!(taken, "owned count was checked above");
        self.recompute_topology();
        self.mark_tiles_dirty(&tiles);
        true
    }

    /// Removes whatever occupies `tile`, crediting one back to the owned
    /// count of its kind. See [`PlayerCommand::Remove`].
    fn try_remove(&mut self, tile: TileCoord) -> bool {
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        let placement = self.placement_of(entity);
        let origin = self.position_of(entity);
        let tiles = placement.tiles(origin);
        self.ecs
            .despawn(entity)
            .expect("the grid only ever names live entities");
        self.shop.owned.add(&placement.kind, 1);
        self.recompute_topology();
        self.mark_tiles_dirty(&tiles);
        true
    }

    /// Turns whatever occupies `tile` 90° clockwise in place. See
    /// [`PlayerCommand::Rotate`].
    fn try_rotate(&mut self, tile: TileCoord) -> bool {
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        let placement = self.placement_of(entity);
        let origin = self.position_of(entity);
        let old_tiles = placement.tiles(origin);
        let turned = placement.direction.rotate_cw();
        let new_tiles = placement.footprint.tiles_facing(origin, turned);
        // Its own tiles are its own to reclaim; anybody else's are not.
        let blocked = new_tiles.iter().any(|t| {
            self.topology
                .grid
                .get(t)
                .is_some_and(|&occupant| occupant != entity)
        });
        if blocked {
            return false;
        }
        self.ecs
            .get::<&mut Placement>(entity)
            .expect("a placed entity always has a Placement")
            .direction = turned;
        self.recompute_topology();
        self.mark_tiles_dirty(&old_tiles);
        self.mark_tiles_dirty(&new_tiles);
        true
    }

    // -----------------------------------------------------------------
    // Topology
    // -----------------------------------------------------------------

    /// Rebuilds everything derived from where things stand: the occupancy
    /// grid, the ordered per-kind entity lists, the hand-off links, and the
    /// fluid networks.
    ///
    /// **Called only from `Place`, `Remove` and `Rotate`** — the three
    /// commands that can change what is adjacent to what — and never from
    /// [`Self::tick`]. That is ARCHITECTURE.md §3.8's rule ("compute
    /// connected-component grouping via union-find, once per topology
    /// change ... not every tick") applied at the scale this project
    /// actually has: a full rebuild of a few hundred tiles is cheaper than
    /// the bookkeeping an incremental union-find would need, and it cannot
    /// drift out of sync with the grid the way an incremental one could.
    /// When a world gets big enough for that trade to flip (milestone 8's
    /// "thousands of entities"), this one function is what changes.
    ///
    /// Fluid *volumes* are carried across the rebuild by tile, so a
    /// topology change never spills the run; a pipe that is removed takes
    /// its contents with it, which is [`PlayerCommand::Remove`]'s documented
    /// behaviour.
    fn recompute_topology(&mut self) {
        let carried_volumes: BTreeMap<TileCoord, Fx> = self
            .topology
            .networks
            .iter()
            .flat_map(|n| n.segments())
            .map(|s| (s.tile, s.volume))
            .collect();

        let mut placements: Vec<(TileCoord, u32, Entity, BodyKind)> = self
            .ecs
            .iter()
            .filter_map(|entity_ref| {
                let position = entity_ref.get::<&Position>()?;
                let body = entity_ref.get::<&BodyKind>()?;
                Some((
                    position.tile,
                    entity_ref.entity().id(),
                    entity_ref.entity(),
                    *body,
                ))
            })
            .collect();
        // Deterministic order: by origin tile, then by entity id for the
        // (impossible today, but cheap to guarantee) tie.
        placements.sort_by_key(|&(tile, id, _, _)| (tile.1, tile.0, id));

        let mut topology = Topology::default();
        for &(_, _, entity, body) in &placements {
            topology.all.push(entity);
            match body {
                BodyKind::Belt => topology.belts.push(entity),
                BodyKind::Splitter => topology.splitters.push(entity),
                BodyKind::Pipe => topology.pipes.push(entity),
                BodyKind::Machine => topology.machines.push(entity),
                BodyKind::Hopper => topology.hoppers.push(entity),
                BodyKind::FluidTank => topology.tanks.push(entity),
                BodyKind::Dock => topology.docks.push(entity),
            }
            let placement = self.placement_of(entity);
            let origin = self.position_of(entity);
            for tile in placement.tiles(origin) {
                let previous = topology.grid.insert(tile, entity);
                debug_assert!(
                    previous.is_none(),
                    "two placements claim tile {tile:?} — Place/Rotate must have refused one"
                );
            }
        }

        let body_of = |entity: Entity| -> BodyKind {
            *self
                .ecs
                .get::<&BodyKind>(entity)
                .expect("every placement carries a BodyKind")
        };

        // --- the wire's `links`: one entry per belt-like and pipe entity,
        // seeded empty here and filled in below at the exact point each
        // relation is resolved, so a lonely belt still reports
        // `{"inputs": [], "output": null}` and a lonely pipe
        // `{"connections": []}`. Sorted by side at the end of the pass.
        let mut links: BTreeMap<Entity, Links> = BTreeMap::new();
        for &entity in topology.belts.iter().chain(topology.splitters.iter()) {
            links.insert(entity, Links::Conveyor(ConveyorLinks::default()));
        }
        for &entity in &topology.pipes {
            links.insert(entity, Links::Pipe(PipeLinks::default()));
        }
        fn conveyor_links(
            links: &mut BTreeMap<Entity, Links>,
            entity: Entity,
        ) -> &mut ConveyorLinks {
            match links.get_mut(&entity) {
                Some(Links::Conveyor(conveyor)) => conveyor,
                _ => unreachable!("every belt-like entity was seeded with ConveyorLinks"),
            }
        }
        fn pipe_links(links: &mut BTreeMap<Entity, Links>, entity: Entity) -> &mut PipeLinks {
            match links.get_mut(&entity) {
                Some(Links::Pipe(pipe)) => pipe,
                _ => unreachable!("every pipe entity was seeded with PipeLinks"),
            }
        }

        // --- solid hand-offs: a belt hands to whatever it points at, and a
        // machine/hopper discharges onto whatever belt is off its outlet
        // face. Faces matter here (see this module's doc).
        //
        // What may receive a solid item handed *into* `target`. Written once
        // because two relations now ask it — a belt's single downstream tile
        // and each of a splitter's two — and they must never diverge on what
        // counts as a valid sink.
        let resolve_sink = |grid: &BTreeMap<TileCoord, Entity>, target: TileCoord| {
            let &occupant = grid.get(&target)?;
            match body_of(occupant) {
                // Belt-like: takes anything, whatever it is facing, exactly
                // as a belt tile always has.
                BodyKind::Belt | BodyKind::Splitter => Some(occupant),
                // A machine or a dock only accepts on its inlet face.
                BodyKind::Machine | BodyKind::Dock => {
                    let sink = self.placement_of(occupant);
                    let sink_origin = self.position_of(occupant);
                    sink.footprint
                        .inlet_tiles(sink_origin, sink.direction)
                        .contains(&target)
                        .then_some(occupant)
                }
                BodyKind::Pipe | BodyKind::Hopper | BodyKind::FluidTank => None,
            }
        };
        topology.belt_ahead = vec![None; topology.belts.len()];
        for (index, &belt) in topology.belts.iter().enumerate() {
            let placement = self.placement_of(belt);
            let target = placement.direction.step(self.position_of(belt));
            let Some(occupant) = resolve_sink(&topology.grid, target) else {
                continue;
            };
            let occupant_body = body_of(occupant);
            conveyor_links(&mut links, belt).output = Some(LinkOutput {
                to: placement.direction,
                kind: occupant_body.into(),
            });
            if is_belt_like(occupant_body) {
                topology.belt_to_belt.push((belt, occupant));
                topology.belt_ahead[index] = Some(occupant);
                conveyor_links(&mut links, occupant).inputs.push(LinkInput {
                    from: placement.direction.opposite(),
                    kind: LinkKind::Belt,
                });
            } else {
                topology.belt_to_sink.push((belt, occupant));
            }
        }
        // A splitter's two outputs: its own facing, then one quarter turn
        // clockwise from it (`splitter.rs` owns that rule and this is its
        // only call site). A side with nothing valid on it stays `None` and
        // simply always refuses at tick time.
        for &splitter in &topology.splitters {
            let placement = self.placement_of(splitter);
            let origin = self.position_of(splitter);
            let outputs = Side::BOTH.map(|side| {
                let direction = side.direction_from(placement.direction);
                let sink = resolve_sink(&topology.grid, direction.step(origin));
                if let Some(occupant) = sink {
                    let occupant_body = body_of(occupant);
                    // The wire's `output` is the straight side only; the
                    // branch is already on the wire as `outputs[1]`.
                    if side == Side::Straight {
                        conveyor_links(&mut links, splitter).output = Some(LinkOutput {
                            to: direction,
                            kind: occupant_body.into(),
                        });
                    }
                    if is_belt_like(occupant_body) {
                        conveyor_links(&mut links, occupant).inputs.push(LinkInput {
                            from: direction.opposite(),
                            kind: LinkKind::Splitter,
                        });
                    }
                }
                sink
            });
            topology.splitter_outputs.push((splitter, outputs));
        }
        for &source in topology.machines.iter().chain(topology.hoppers.iter()) {
            let placement = self.placement_of(source);
            let origin = self.position_of(source);
            // The first belt (or splitter) off the outlet face, in the
            // face's own deterministic order. A machine hands over one unit
            // per tick, so a second belt against the same face would need a
            // round-robin rule nobody has asked for — which is exactly what
            // a splitter standing there gives the player instead.
            let outlet_belt = placement
                .footprint
                .outlet_neighbours(origin, placement.direction)
                .into_iter()
                .filter_map(|tile| topology.grid.get(&tile).copied())
                .find(|&occupant| is_belt_like(body_of(occupant)));
            if let Some(belt) = outlet_belt {
                topology.source_to_belt.push((source, belt));
                conveyor_links(&mut links, belt).inputs.push(LinkInput {
                    from: placement.direction.opposite(),
                    kind: body_of(source).into(),
                });
            }
        }

        // --- fluid networks: one per connected component of pipe tiles,
        // adjacency undirected (see `fluid.rs`'s module doc).
        let pipe_tiles: BTreeMap<TileCoord, (Entity, Fx, Direction)> = topology
            .pipes
            .iter()
            .map(|&entity| {
                let placement = self.placement_of(entity);
                let capacity = self
                    .ecs
                    .get::<&PipeTile>(entity)
                    .expect("a pipe placement carries a PipeTile")
                    .capacity;
                (
                    self.position_of(entity),
                    (entity, capacity, placement.direction),
                )
            })
            .collect();
        let mut unvisited: Vec<TileCoord> = pipe_tiles.keys().copied().collect();
        unvisited.reverse(); // pop() then yields ascending tile order
        let mut seen: BTreeMap<TileCoord, ()> = BTreeMap::new();
        while let Some(start) = unvisited.pop() {
            if seen.contains_key(&start) {
                continue;
            }
            // Breadth-first over the component, then sorted — so a
            // network's segment order is its tile order regardless of which
            // tile happened to be found first.
            let mut component: Vec<TileCoord> = Vec::new();
            let mut frontier = vec![start];
            seen.insert(start, ());
            while let Some(tile) = frontier.pop() {
                component.push(tile);
                for dir in Direction::ALL {
                    let next = dir.step(tile);
                    if pipe_tiles.contains_key(&next) && seen.insert(next, ()).is_none() {
                        frontier.push(next);
                    }
                }
            }
            component.sort_by_key(|&(x, y)| (y, x));

            let index_of: BTreeMap<TileCoord, usize> = component
                .iter()
                .enumerate()
                .map(|(i, &tile)| (tile, i))
                .collect();
            let segments: Vec<PipeSegment> = component
                .iter()
                .map(|&tile| {
                    let (_, capacity, direction) = pipe_tiles[&tile];
                    let mut segment = PipeSegment::new(tile, direction, capacity);
                    segment.volume = carried_volumes.get(&tile).copied().unwrap_or(Fx::ZERO);
                    segment
                })
                .collect();
            // Each adjacency once: only look east and north, so `(i, j)`
            // never appears alongside `(j, i)`.
            let mut segment_links = Vec::new();
            for (i, &tile) in component.iter().enumerate() {
                for dir in [Direction::East, Direction::North] {
                    let neighbour = dir.step(tile);
                    if let Some(&j) = index_of.get(&neighbour) {
                        segment_links.push((i, j));
                        // The same adjacency, once from each end, for the
                        // wire.
                        let (this, _, _) = pipe_tiles[&tile];
                        let (that, _, _) = pipe_tiles[&neighbour];
                        pipe_links(&mut links, this)
                            .connections
                            .push(PipeConnection {
                                to: dir,
                                kind: LinkKind::Pipe,
                            });
                        pipe_links(&mut links, that)
                            .connections
                            .push(PipeConnection {
                                to: dir.opposite(),
                                kind: LinkKind::Pipe,
                            });
                    }
                }
            }
            let network_index = topology.networks.len();
            for (segment_index, &tile) in component.iter().enumerate() {
                topology
                    .pipe_index
                    .insert(tile, (network_index, segment_index));
            }
            topology
                .networks
                .push(FluidNetwork::new(None, segments, segment_links));
        }

        // --- who taps what.
        //
        // Two rules, and the asymmetry between them is deliberate (this
        // module's doc): a machine **deposits** into the pipe off its outlet
        // face, exactly as it discharges a solid, and **draws** from any
        // other pipe merely touching its footprint. Resolving the output
        // first is what makes the input's "any other network" well defined,
        // and it is why a loop of pipe wrapped around one machine reads as
        // its outlet rather than as a machine feeding itself.
        //
        // Whichever machine reaches an unclaimed network first names it,
        // producer or consumer alike; machines are walked in tile order, so
        // that is deterministic. Two machines naming the same run agree by
        // naming the same id in `recipes.ron`, which is what makes a
        // mixer -> pipe -> vat run resolve to one fluid from either end.
        for &machine in &topology.machines {
            let placement = self.placement_of(machine);
            let origin = self.position_of(machine);
            let Some(recipe) = self
                .ecs
                .get::<&Machine>(machine)
                .ok()
                .and_then(|m| self.recipes.get(&m.recipe).cloned())
            else {
                continue;
            };

            let mut output_network: Option<usize> = None;
            if let Some((fluid_id, _)) = recipe.fluid_outputs.first().cloned() {
                let tap = placement
                    .footprint
                    .outlet_neighbours(origin, placement.direction)
                    .into_iter()
                    .find_map(|tile| topology.pipe_index.get(&tile).map(|&index| (tile, index)));
                if let Some((tile, (network, segment))) = tap {
                    if topology.networks[network].fluid.is_none() {
                        topology.networks[network].fluid = Some(fluid_id);
                    }
                    output_network = Some(network);
                    topology
                        .machine_output_taps
                        .push((machine, network, segment));
                    // The pipe stands off the outlet face, so from the pipe
                    // the machine is back the way the machine points.
                    let (pipe, _, _) = pipe_tiles[&tile];
                    pipe_links(&mut links, pipe)
                        .connections
                        .push(PipeConnection {
                            to: placement.direction.opposite(),
                            kind: LinkKind::Machine,
                        });
                }
            }

            if let Some((fluid_id, _)) = recipe.fluid_inputs.first().cloned() {
                let tap = placement
                    .tiles(origin)
                    .into_iter()
                    .flat_map(|tile| Direction::ALL.map(|dir| (dir, dir.step(tile))))
                    .filter_map(|(dir, neighbour)| {
                        topology
                            .pipe_index
                            .get(&neighbour)
                            .map(|&index| (dir, neighbour, index))
                    })
                    // A machine never draws from the run it fills: that is
                    // not plumbing, it is a machine feeding itself.
                    .find(|&(_, _, (network, _))| Some(network) != output_network);
                if let Some((dir, tile, (network, segment))) = tap {
                    if topology.networks[network].fluid.is_none() {
                        topology.networks[network].fluid = Some(fluid_id);
                    }
                    topology.machine_taps.push((machine, network, segment));
                    // `dir` stepped from the footprint to the pipe; the
                    // wire names the side from the pipe's point of view.
                    let (pipe, _, _) = pipe_tiles[&tile];
                    pipe_links(&mut links, pipe)
                        .connections
                        .push(PipeConnection {
                            to: dir.opposite(),
                            kind: LinkKind::Machine,
                        });
                }
            }
        }
        for &tank in &topology.tanks {
            let placement = self.placement_of(tank);
            let origin = self.position_of(tank);
            let discharge = placement
                .footprint
                .outlet_neighbours(origin, placement.direction)
                .into_iter()
                .find_map(|tile| topology.pipe_index.get(&tile).map(|&index| (tile, index)));
            if let Some((tile, (network, segment))) = discharge {
                topology.networks[network].mark_source(segment);
                topology.tank_discharges.push((tank, network, segment));
                let (pipe, _, _) = pipe_tiles[&tile];
                pipe_links(&mut links, pipe)
                    .connections
                    .push(PipeConnection {
                        to: placement.direction.opposite(),
                        kind: LinkKind::Tank,
                    });
            }
        }

        // A side order that is a property of the geometry (north, east,
        // south, west — `Direction`'s own order), not of which placement the
        // pass walked first. At most one thing can stand on each side of a
        // tile, so this is a total order and the wire shape is canonical.
        for entry in links.values_mut() {
            match entry {
                Links::Conveyor(conveyor) => conveyor.inputs.sort_by_key(|input| input.from),
                Links::Pipe(pipe) => pipe.connections.sort_by_key(|connection| connection.to),
            }
        }
        topology.links = links;

        self.topology = topology;
    }

    fn position_of(&self, entity: Entity) -> TileCoord {
        self.ecs
            .get::<&Position>(entity)
            .expect("a placed entity always has a Position")
            .tile
    }

    /// A placed entity's kind + footprint + facing, cloned out of the ECS so
    /// the caller isn't holding a borrow on `self.ecs`.
    fn placement_of(&self, entity: Entity) -> Placement {
        (*self
            .ecs
            .get::<&Placement>(entity)
            .expect("a placed entity always has a Placement"))
        .clone()
    }

    fn mark_dirty(&mut self, tile: TileCoord) {
        let chunk = chunk_of(tile);
        if !self.dirty.contains(&chunk) {
            self.dirty.push(chunk);
        }
    }

    fn mark_tiles_dirty(&mut self, tiles: &[TileCoord]) {
        for &tile in tiles {
            self.mark_dirty(tile);
        }
    }

    /// Marks every chunk a placement touches, not just the one its origin
    /// tile is in — a footprint straddling a chunk boundary changes what
    /// both chunks' snapshots contain, so both have to be re-read.
    fn mark_placement_dirty(&mut self, entity: Entity) {
        let placement = self.placement_of(entity);
        let origin = self.position_of(entity);
        for chunk in footprint_chunks(origin, placement.footprint.rotated(placement.direction)) {
            if !self.dirty.contains(&chunk) {
                self.dirty.push(chunk);
            }
        }
    }
}

impl World {
    /// Advances the simulation by exactly one tick (ARCHITECTURE.md §3.9:
    /// fixed timestep, no wall-clock reads).
    ///
    /// The passes, in order — every one of them walking a list the topology
    /// pass built in tile order, so the whole tick is deterministic:
    ///
    /// 0. **Auto-fill.** Every hopper and tank the player has switched to
    ///    automatic ([`super::hopper::Hopper::auto_fill`]) is topped up to
    ///    capacity out of bought stock. **First**, before anything consumes
    ///    anything, and that placement is the mechanic: a topped-up tank
    ///    discharges into its run in step 1 of the same tick, and a topped-up
    ///    hopper meters a unit onto its belt in step 5 of the same tick — so
    ///    an auto-filled line never stutters for a tick between "the stock
    ///    exists" and "the machine gets it". Running it last would mean a
    ///    hopper that emptied this tick fed nothing this tick, which is a
    ///    one-tick hiccup with no reason behind it.
    /// 1. **Fluid.** Every tank discharges into the pipe it faces, then
    ///    every network equalizes across its own links. First, so a machine
    ///    starting a craft below sees this tick's pressure rather than last
    ///    tick's (ARCHITECTURE.md §3.4). Deliberately marks **no** chunk
    ///    dirty: fluid moves on nearly every tick, and dirtying a chunk
    ///    every tick would defeat the point of dirty tracking (§4's delta
    ///    granularity). Presentation polls `fluid_snapshots` instead.
    /// 2. **Belts move**, each by one tick's travel — splitters too, which
    ///    are belt tiles with a choice at the end (see
    ///    [`super::splitter`]), so product crosses one at belt speed.
    /// 2b. **Staff pools reset** (the staff pools MVP epic): every category
    ///    is back at its headcount with nothing asked for yet.
    /// 3. **Machines run** — start a craft, or deposit a finished one. In
    ///    `topology.machines` order each machine that would work this tick
    ///    first claims its kind's whole crew from the pools; a refused
    ///    machine takes nothing, does nothing and reads `starved` with a
    ///    `staff` stall (see `Machine::tick`). Stable order is what makes
    ///    the *same* machine the one that goes short every tick.
    /// 4. **Consumers pull** off a belt's end: into a machine's input
    ///    buffer, or onto a dock.
    /// 5. **Producers push** onto a belt's start: a hopper meters one raw
    ///    unit, a machine hands over one finished unit, a belt hands its
    ///    front item to the belt ahead of it, and **a splitter deals its
    ///    front item to one of its two outputs**. All after every belt has
    ///    already moved this tick, so nothing crosses two tiles in one tick
    ///    — a splitter included, since an item handed *onto* it this tick
    ///    lands at the start of its segment and is not at the end yet.
    /// 6. **Docks ship** whatever they took in step 4 into [`Economy`]'s
    ///    shared delivery pool — in the same tick it arrived, because a dock
    ///    is a pass-through (see [`super::dock`]) and the pool is what
    ///    contracts settle against below. A dock holding something first
    ///    claims its crew like a machine; refused, it holds (at capacity the
    ///    belt backs up; nothing is lost) and reads `understaffed`.
    /// 7. **Contracts settle**, one cadence check per active contract. A
    ///    contract that pays records its mix in the production ledger as
    ///    consumed — the factory's exit — and the ledger's clock advances
    ///    after it, so everything this tick recorded sits in the bucket that
    ///    was current while it happened.
    /// 8. **Payday** (`StaffPools::tick`): on the tick the clock reaches
    ///    `next_payday_tick`, wages are paid one head at a time out of the
    ///    one money counter, and the first head that cannot be paid — and
    ///    every one after it — leaves.
    ///
    /// Steps 4 and 5 are the same two phases the fixed chain always had
    /// ("belt to machine, then everything onto a belt"); what changed is
    /// that the pairs come from adjacency rather than from a stage index.
    pub fn tick(&mut self) {
        self.dirty.clear();
        self.tick_count += 1;

        // 0. Auto-fill, before anything draws down a buffer.
        self.auto_fill_buffers();

        // 1. Fluid.
        let tank_discharges = std::mem::take(&mut self.topology.tank_discharges);
        for &(tank, network, segment) in &tank_discharges {
            if let Ok(mut tank_body) = self.ecs.get::<&mut FluidTank>(tank) {
                self.topology.networks[network].discharge_from_tank(&mut tank_body, segment);
            }
        }
        self.topology.tank_discharges = tank_discharges;
        for network in &mut self.topology.networks {
            network.tick();
        }

        // 2b. Staff pools: everyone back at their post, nobody asked for.
        self.staff.reset();

        // 2. Belts — and splitters, which carry their contents in an
        // ordinary belt segment (`splitter.rs`) so that product crosses one
        // in exactly the time it would cross the belt tile it replaced.
        //
        // Every carrier's front item stops short of its end by whatever
        // footprint the item at the start of the NEXT carrier still claims
        // (`belt.rs`, "the footprint does not stop at the belt's end").
        // The rooms are all read before any carrier moves, so the outcome
        // is the same whatever order the carriers are visited in — which is
        // what lets a save that was rebuilt in a different placement order
        // stay in lockstep with the world it was taken from.
        let belts = std::mem::take(&mut self.topology.belts);
        let splitters = std::mem::take(&mut self.topology.splitters);
        let belt_ahead = std::mem::take(&mut self.topology.belt_ahead);
        let splitter_outputs = std::mem::take(&mut self.topology.splitter_outputs);
        let room_at_start = |world: &Self, carrier: Option<Entity>| -> Option<Fx> {
            let carrier = carrier?;
            world
                .ecs
                .get::<&BeltSegment>(carrier)
                .ok()
                .map(|segment| segment.room_at_start())
        };
        self.room_ahead.clear();
        for index in 0..belts.len() {
            let room = room_at_start(self, belt_ahead[index]);
            self.room_ahead.push(room.unwrap_or(self.item_min_spacing()));
        }
        for (index, _) in splitters.iter().enumerate() {
            // A splitter's item goes to whichever side takes it, so the
            // room that matters is the roomier side's; neither side being
            // a carrier is the "nothing past the end" case, like a belt
            // into a machine.
            let (_, outputs) = splitter_outputs[index];
            let room = outputs
                .iter()
                .filter_map(|&output| room_at_start(self, output))
                .max();
            self.room_ahead.push(room.unwrap_or(self.item_min_spacing()));
        }
        for (index, &carrier) in belts.iter().chain(splitters.iter()).enumerate() {
            let moved = {
                let mut segment = self
                    .ecs
                    .get::<&mut BeltSegment>(carrier)
                    .expect("a belt or splitter placement carries a BeltSegment");
                segment.tick_toward(self.belt_speed, self.room_ahead[index])
            };
            if moved {
                let tile = self.position_of(carrier);
                self.mark_dirty(tile);
            }
        }
        self.topology.splitter_outputs = splitter_outputs;
        self.topology.belt_ahead = belt_ahead;
        self.topology.splitters = splitters;

        // 3. Machines.
        let machines = std::mem::take(&mut self.topology.machines);
        let taps = std::mem::take(&mut self.topology.machine_taps);
        let output_taps = std::mem::take(&mut self.topology.machine_output_taps);
        for &machine in &machines {
            let kind = self.placement_of(machine).kind;
            let tapped = taps
                .iter()
                .find(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, inlet)| (network, inlet));
            let pushing = output_taps
                .iter()
                .find(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, outlet)| (network, outlet));
            let changed = {
                let mut body = self
                    .ecs
                    .get::<&mut Machine>(machine)
                    .expect("a machine placement carries a Machine");
                // The topology pass guarantees the two networks are
                // different (see `recompute_topology`), which is what makes
                // holding both mutable borrows at once sound — and what
                // `two_networks_mut` asserts rather than assumes.
                let (fluid_in, fluid_out) = match (tapped, pushing) {
                    (Some((a, inlet)), Some((b, outlet))) => {
                        let (net_a, net_b) = two_networks_mut(&mut self.topology.networks, a, b);
                        (
                            Some(FluidTap {
                                network: net_a,
                                inlet,
                            }),
                            Some(FluidTap {
                                network: net_b,
                                inlet: outlet,
                            }),
                        )
                    }
                    (Some((a, inlet)), None) => (
                        Some(FluidTap {
                            network: &mut self.topology.networks[a],
                            inlet,
                        }),
                        None,
                    ),
                    (None, Some((b, outlet))) => (
                        None,
                        Some(FluidTap {
                            network: &mut self.topology.networks[b],
                            inlet: outlet,
                        }),
                    ),
                    (None, None) => (None, None),
                };
                // The crew is the kind's `machines.ron` list; the claim is
                // made inside `tick`, only on a tick the machine would work.
                let crew = CrewRequest {
                    crew: self.machines.get(&kind).map_or(&[], |k| k.staff.as_slice()),
                    pools: &mut self.staff,
                };
                body.tick(
                    self.tick_count,
                    &self.recipes,
                    fluid_in,
                    fluid_out,
                    Some(crew),
                    &mut self.production,
                )
            };
            if changed {
                self.mark_placement_dirty(machine);
            }
        }
        self.topology.machine_taps = taps;
        self.topology.machine_output_taps = output_taps;

        // 4. Consumers pull off a belt's end.
        let belt_to_sink = std::mem::take(&mut self.topology.belt_to_sink);
        for &(belt, sink) in &belt_to_sink {
            let moved = match self.body_kind(sink) {
                Some(BodyKind::Machine) => self.try_transfer_belt_to_machine(belt, sink),
                Some(BodyKind::Dock) => self.try_absorb_belt_into_dock(belt, sink),
                _ => false,
            };
            if moved {
                let tile = self.position_of(belt);
                self.mark_dirty(tile);
                self.mark_placement_dirty(sink);
            }
        }
        self.topology.belt_to_sink = belt_to_sink;

        // 5. Producers push onto a belt's start.
        let source_to_belt = std::mem::take(&mut self.topology.source_to_belt);
        for &(source, belt) in &source_to_belt {
            let moved = match self.body_kind(source) {
                Some(BodyKind::Machine) => self.try_transfer_machine_output_to_belt(source, belt),
                Some(BodyKind::Hopper) => self.try_unload_hopper_onto_belt(source, belt),
                _ => false,
            };
            if moved {
                self.mark_placement_dirty(source);
                let tile = self.position_of(belt);
                self.mark_dirty(tile);
            }
        }
        self.topology.source_to_belt = source_to_belt;

        let belt_to_belt = std::mem::take(&mut self.topology.belt_to_belt);
        for &(from, to) in &belt_to_belt {
            if self.try_transfer_belt_to_belt(from, to) {
                let (from_tile, to_tile) = (self.position_of(from), self.position_of(to));
                self.mark_dirty(from_tile);
                self.mark_dirty(to_tile);
            }
        }
        self.topology.belt_to_belt = belt_to_belt;

        // ...and a splitter deals its front item to one of its two outputs,
        // alternating (`splitter.rs` owns the rule). After the belt-to-belt
        // hop rather than before it only so the whole of step 5 reads in one
        // order; the two passes are independent, because an item handed onto
        // a splitter this tick lands at the start of its segment and cannot
        // also leave it.
        let splitter_outputs = std::mem::take(&mut self.topology.splitter_outputs);
        for &(splitter, outputs) in &splitter_outputs {
            if let Some(target) = self.try_split(splitter, outputs) {
                let tile = self.position_of(splitter);
                self.mark_dirty(tile);
                self.mark_placement_dirty(target);
            }
        }
        self.topology.splitter_outputs = splitter_outputs;

        // 6. Docks ship.
        let docks = std::mem::take(&mut self.topology.docks);
        for &dock in &docks {
            let kind = self.placement_of(dock).kind;
            let (shipped, flag_flipped) = {
                let mut body = self
                    .ecs
                    .get::<&mut Dock>(dock)
                    .expect("a dock placement carries a Dock");
                if body.is_empty() {
                    // Nothing to ship, nobody asked for; the verdict of the
                    // last loaded tick stands.
                    (0, false)
                } else {
                    let crew: &[(String, u32)] =
                        self.machines.get(&kind).map_or(&[], |k| k.staff.as_slice());
                    let was = body.understaffed();
                    match self.staff.claim(crew) {
                        Ok(()) => {
                            body.set_understaffed(false);
                            (body.ship_into(&mut self.economy), was)
                        }
                        Err(_) => {
                            // Held, not lost: the buffer keeps it, and at
                            // capacity `try_absorb` refuses and the belt
                            // backs up.
                            body.set_understaffed(true);
                            (0, !was)
                        }
                    }
                }
            };
            if flag_flipped {
                // `understaffed` rides the dock's chunk snapshot.
                self.mark_placement_dirty(dock);
            }
            if shipped > 0 {
                // The dock's own snapshot changed (`shipped_total` at
                // least), so a chunk-driven renderer has to re-read it. At
                // most once per box, not once per tick, so it does not
                // defeat dirty tracking the way fluid movement would.
                self.mark_placement_dirty(dock);
            }
        }
        self.topology.docks = docks;
        self.topology.belts = belts;
        self.topology.machines = machines;

        // 7. Contracts. Not tied to any tile, so it doesn't participate in
        // dirty-chunk tracking; Godot reads economy state via its own
        // always-fresh snapshot instead. A contract lost here (its strikes
        // used up) simply stops appearing in that snapshot's `contracts` and
        // reads `active: false` on its `board`; the events are not kept —
        // the snapshot is the wire, and a UI diffs it. What IS kept from the
        // events is the one thing no snapshot can reconstruct afterwards:
        // a payout took its mix out of the pool, which is the factory's
        // exit and so its consumption of the finished product.
        let events = self.economy.tick(self.tick_count);
        for event in &events {
            if let ContractEvent::Paid { id } = event {
                if let Some(contract) = self.economy.board().get(id) {
                    for (item, qty) in &contract.mix {
                        self.production.record_consumed(item, StatKind::Item, *qty);
                    }
                }
            }
        }
        self.production.tick();

        // 8. Payday. Not tied to any tile either; Godot polls
        // `staff_snapshot`. A head who leaves unpaid simply is not there
        // when the pools reset next tick.
        let _ = self.staff.tick(self.tick_count, &mut self.economy);
    }

    /// Step 0 of [`World::tick`]: tops every auto-filling hopper and tank up
    /// **to capacity** out of the player's bought stock.
    ///
    /// Three rules, all of them deliberate and all of them stated in
    /// [`super::hopper::Hopper::auto_fill`]'s doc as well:
    ///
    /// - **To capacity, not by a fill-command's worth.** The mechanic is
    ///   "keep this bin full", so it asks for the whole room every tick; in
    ///   practice a running hopper is one unit down and takes one unit back.
    /// - **It spends stock, never money.** [`PlayerCommand::BuyIngredient`]
    ///   stays the only thing that moves the balance (GAMEPLAY.md §5). An
    ///   empty larder simply means nothing is added — no error, no flag flip,
    ///   and top-ups resume by themselves once stock is bought.
    /// - **The same fill rule as the manual command**, because it is
    ///   literally the same function ([`World::fill_hopper_entity`] /
    ///   [`World::fill_tank_entity`]): two clamps, owned stock then room, and
    ///   the ledger debited by what was actually added.
    ///
    /// A tank asks only for [`FluidTank::whole_unit_room`] and therefore
    /// **marks no chunk dirty**, both for the same reason: this runs every
    /// tick, so rounding a part unit up would bill sixty whole units a second
    /// for a trickle, and dirtying the chunk every tick would defeat
    /// ARCHITECTURE.md §4's delta granularity exactly the way the discharge
    /// pass below would if it dirtied. A tank's level is already documented
    /// as poll-only ([`World::fluid_tank_snapshots`]); a hopper's is not, so
    /// a hopper top-up does dirty, as it always has.
    fn auto_fill_buffers(&mut self) {
        let hoppers = std::mem::take(&mut self.topology.hoppers);
        for &hopper in &hoppers {
            let room = match self.ecs.get::<&Hopper>(hopper) {
                Ok(body) if body.auto_fill() => body.room(),
                _ => 0,
            };
            if room > 0 {
                self.fill_hopper_entity(hopper, room);
            }
        }
        self.topology.hoppers = hoppers;

        let tanks = std::mem::take(&mut self.topology.tanks);
        for &tank in &tanks {
            let room = match self.ecs.get::<&FluidTank>(tank) {
                Ok(body) if body.auto_fill() => body.whole_unit_room(),
                _ => 0,
            };
            if room > 0 {
                self.fill_tank_entity(tank, room);
            }
        }
        self.topology.tanks = tanks;
    }

    fn body_kind(&self, entity: Entity) -> Option<BodyKind> {
        self.ecs.get::<&BodyKind>(entity).ok().map(|body| *body)
    }

    /// The wire's links for a belt or splitter — looked up, never derived,
    /// since the topology pass recorded them (see `Topology::links`).
    fn conveyor_links_of(&self, entity: Entity) -> ConveyorLinks {
        match self.topology.links.get(&entity) {
            Some(Links::Conveyor(conveyor)) => conveyor.clone(),
            _ => unreachable!("every belt-like entity has ConveyorLinks in the topology"),
        }
    }

    /// If `belt` has an item waiting at its end and `machine` has room for
    /// it, moves it across the interop-invisible boundary between the two
    /// components. Returns whether a transfer happened.
    fn try_transfer_belt_to_machine(&mut self, belt: Entity, machine: Entity) -> bool {
        let ready_item = {
            let segment = self.ecs.get::<&BeltSegment>(belt).unwrap();
            segment.front_ready().cloned()
        };
        let Some(item_id) = ready_item else {
            return false;
        };
        let has_room = {
            let body = self.ecs.get::<&Machine>(machine).unwrap();
            body.input.has_room_for(&item_id, 1)
        };
        if !has_room {
            return false;
        }
        let taken = {
            let mut segment = self.ecs.get::<&mut BeltSegment>(belt).unwrap();
            segment.take_front_if_at_end()
        };
        let Some(taken) = taken else {
            return false;
        };
        let mut body = self.ecs.get::<&mut Machine>(machine).unwrap();
        body.input.add(&taken.item, 1);
        true
    }

    /// If `machine` is holding a finished item and `belt` has room to accept
    /// one at its start, hands one unit across. Returns whether a transfer
    /// happened.
    fn try_transfer_machine_output_to_belt(&mut self, machine: Entity, belt: Entity) -> bool {
        let ready_item = {
            let body = self.ecs.get::<&Machine>(machine).unwrap();
            body.output
                .iter()
                .find(|(_, qty)| *qty > 0)
                .map(|(id, _)| id.clone())
        };
        let Some(item_id) = ready_item else {
            return false;
        };
        let spawned = {
            let mut segment = self.ecs.get::<&mut BeltSegment>(belt).unwrap();
            segment.try_spawn(item_id.clone())
        };
        if !spawned {
            return false;
        }
        let mut body = self.ecs.get::<&mut Machine>(machine).unwrap();
        body.output.remove(&item_id, 1);
        true
    }

    /// Hands the item at `from`'s end to `to`'s start — the belt-to-belt
    /// hop, which is new with placement (a belt used to be a whole
    /// multi-tile run, so there was nothing to hop between).
    ///
    /// Spawn first, then take: [`BeltSegment::try_spawn`] is both the room
    /// check and the commit, so committing it before removing the item is
    /// what makes a refused hop leave the item exactly where it was rather
    /// than annihilating it. That ordering is the same one
    /// [`Self::try_transfer_machine_output_to_belt`] has always used, and it
    /// is what the mass-conservation tests actually check.
    fn try_transfer_belt_to_belt(&mut self, from: Entity, to: Entity) -> bool {
        debug_assert_ne!(from, to, "a belt cannot point at itself");
        let ready_item = {
            let segment = self.ecs.get::<&BeltSegment>(from).unwrap();
            segment.front_ready().cloned()
        };
        let Some(item_id) = ready_item else {
            return false;
        };
        let spawned = {
            let mut segment = self.ecs.get::<&mut BeltSegment>(to).unwrap();
            segment.try_spawn(item_id)
        };
        if !spawned {
            return false;
        }
        let mut segment = self.ecs.get::<&mut BeltSegment>(from).unwrap();
        let taken = segment.take_front_if_at_end();
        debug_assert!(
            taken.is_some(),
            "the item was ready one statement ago and nothing else can have moved it"
        );
        true
    }

    /// Offers `splitter`'s front item to its two outputs in the order its
    /// cursor says (due side first, then the other — see
    /// [`super::splitter`]'s module doc for why the fallback exists and why
    /// the cursor advances on whichever side actually took it). Returns the
    /// entity that took it, so the caller can dirty its chunk.
    ///
    /// One item per tick at most, like every other hand-off here. Nothing at
    /// all happens when both sides refuse: the item stays exactly where it
    /// is and the line backs up behind the splitter, which is what a
    /// splitter with no outputs — or two full ones — has to do.
    fn try_split(&mut self, splitter: Entity, outputs: [Option<Entity>; 2]) -> Option<Entity> {
        {
            // Cheap early out, and the reason `record_output` below can
            // trust that a successful hand-off moved *this* item.
            let segment = self
                .ecs
                .get::<&BeltSegment>(splitter)
                .expect("a splitter placement carries a BeltSegment");
            segment.front_ready()?;
        }
        let order = self
            .ecs
            .get::<&Splitter>(splitter)
            .expect("a splitter placement carries a Splitter")
            .offer_order();
        for side in order {
            let Some(target) = outputs[side.index()] else {
                continue;
            };
            if self.try_handoff_from_belt_like(splitter, target) {
                self.ecs
                    .get::<&mut Splitter>(splitter)
                    .expect("a splitter placement carries a Splitter")
                    .record_output(side);
                return Some(target);
            }
        }
        None
    }

    /// Hands `from`'s front item to whatever `target` is — the three
    /// existing belt-side transfers behind one dispatch, so a splitter's
    /// output reaches exactly the sinks a belt's does and by exactly the
    /// same code. `target` has already been validated by
    /// `recompute_topology`'s `resolve_sink` (a machine's inlet face, in
    /// particular, was checked there); this only picks the transfer.
    fn try_handoff_from_belt_like(&mut self, from: Entity, target: Entity) -> bool {
        match self.body_kind(target) {
            Some(BodyKind::Belt) | Some(BodyKind::Splitter) => {
                self.try_transfer_belt_to_belt(from, target)
            }
            Some(BodyKind::Machine) => self.try_transfer_belt_to_machine(from, target),
            Some(BodyKind::Dock) => self.try_absorb_belt_into_dock(from, target),
            _ => false,
        }
    }

    fn try_unload_hopper_onto_belt(&mut self, hopper: Entity, belt: Entity) -> bool {
        let mut body = self.ecs.get::<&mut Hopper>(hopper).unwrap();
        let mut segment = self.ecs.get::<&mut BeltSegment>(belt).unwrap();
        body.try_unload(&mut segment)
    }

    fn try_absorb_belt_into_dock(&mut self, belt: Entity, dock: Entity) -> bool {
        let mut body = self.ecs.get::<&mut Dock>(dock).unwrap();
        let mut segment = self.ecs.get::<&mut BeltSegment>(belt).unwrap();
        body.try_absorb(&mut segment)
    }

    /// Applies a player-issued command (ARCHITECTURE.md §4). Invalid/no-op
    /// commands are silently ignored — see [`PlayerCommand`] for the full
    /// list of refusals, every one of which leaves the world byte-identical.
    ///
    /// Note what the two fill commands deliberately do *not* do: they never
    /// touch a belt or a pipe. Getting raw ingredient onto the line is a
    /// hopper's job, one unit per tick from [`Self::tick`]; getting fluid
    /// down a run is the pressure-equalization pass's. The player sets the
    /// *supply*, the machinery sets the *rate*.
    pub fn issue_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::FillHopper { amount, tile } => {
                let hoppers = self.targets(&self.topology.hoppers.clone(), tile);
                for hopper in hoppers {
                    self.fill_hopper_entity(hopper, amount);
                }
            }
            PlayerCommand::FillFluidTank { amount, tile } => {
                let tanks = self.targets(&self.topology.tanks.clone(), tile);
                for tank in tanks {
                    if self.fill_tank_entity(tank, amount) > Fx::ZERO {
                        // A discrete player action, so a chunk-driven
                        // renderer is told; the per-tick auto-fill path
                        // deliberately is not (see `auto_fill_buffers`).
                        self.mark_placement_dirty(tank);
                    }
                }
            }
            PlayerCommand::Purchase { kind, quantity } => {
                // Priced first, then paid for, then credited — and every
                // step that can fail does so before anything has changed, so
                // an unknown kind, a zero quantity, an unrepresentable total
                // (see `Shop::cost_of`) or simply not enough money all leave
                // the world byte-identical. `Economy::try_spend` is itself
                // all-or-nothing for the same reason.
                //
                // No chunk is marked dirty: a purchase changes money and an
                // owned count, neither of which is a tile. *Placing* what was
                // bought is what dirties a chunk.
                let Some(cost) = self.shop.cost_of(&kind, quantity) else {
                    return;
                };
                if !self.economy.try_spend(cost) {
                    return;
                }
                self.shop.owned.add(&kind, quantity);
            }
            PlayerCommand::BuyIngredient { item, quantity } => {
                // `Purchase`'s arm, verbatim, against the other catalog and
                // the other ledger: priced first, then paid for, then
                // credited, with every failure landing before anything has
                // changed. No chunk is dirtied — stock is not a tile.
                let Some(cost) = self.ingredients.cost_of(&item, quantity) else {
                    return;
                };
                if !self.economy.try_spend(cost) {
                    return;
                }
                self.ingredients.owned.add(&item, quantity);
            }
            PlayerCommand::Place {
                kind,
                tile,
                direction,
                item,
            } => {
                self.try_place(&kind, tile, direction, item.as_deref());
            }
            PlayerCommand::Remove { tile } => {
                self.try_remove(tile);
            }
            PlayerCommand::Rotate { tile } => {
                self.try_rotate(tile);
            }
            PlayerCommand::SignContract { id } => {
                // Every refusal (`SignRefusal`) is the same silent no-op as
                // an unaffordable `Purchase`; the economy already spends
                // all-or-nothing. No chunk is dirtied — a contract is not a
                // tile, and Godot reads the board off `economy_snapshot`.
                let _ = self.economy.try_sign(&id, self.tick_count);
            }
            PlayerCommand::SetHopperItem { tile, item } => {
                self.try_set_hopper_item(tile, &item);
            }
            PlayerCommand::SetHopperAutoFill { tile, enabled } => {
                self.try_set_hopper_auto_fill(tile, enabled);
            }
            PlayerCommand::SetTankAutoFill { tile, enabled } => {
                self.try_set_tank_auto_fill(tile, enabled);
            }
            // Neither dirties a chunk: a head is not a tile. Both are silent
            // about an unknown category, and `Fire` about an empty pool,
            // the `Purchase` convention.
            PlayerCommand::Hire { category } => {
                self.staff.hire(&category);
            }
            PlayerCommand::Fire { category } => {
                self.staff.fire(&category);
            }
        }
    }

    /// **The one hopper fill rule in this file.** Adds up to `amount` units
    /// of this hopper's own item out of bought stock, and returns how many
    /// actually landed.
    ///
    /// Both the manual `PlayerCommand::FillHopper` arm and `World::tick`'s
    /// automatic top-up go through here, so the two cannot drift on the part
    /// that matters: **two clamps, in this order** — what the player owns,
    /// then what the bin has room for — and the ledger debited by what
    /// [`Hopper::fill`] reports it actually took, never by what was asked
    /// for.
    ///
    /// It marks the placement dirty itself when something moved, because
    /// both callers want that: a hopper's level is a chunk-visible fact
    /// (see [`World::hopper_snapshots`]), and a hopper being topped up is
    /// also a hopper feeding a belt, which dirties its chunk anyway.
    fn fill_hopper_entity(&mut self, hopper: Entity, amount: u32) -> u32 {
        let item = self.ecs.get::<&Hopper>(hopper).unwrap().item().clone();
        let allowed = amount.min(self.ingredients.owned.count(&item));
        if allowed == 0 {
            return 0;
        }
        let added = {
            let mut body = self.ecs.get::<&mut Hopper>(hopper).unwrap();
            body.fill(allowed)
        };
        self.ingredients.owned.take(&item, added);
        if added > 0 {
            self.mark_placement_dirty(hopper);
        }
        added
    }

    /// **The one tank fill rule in this file**, and [`World::
    /// fill_hopper_entity`]'s twin — same two clamps in the same order, same
    /// debit-what-was-added rule, shared by the manual command and the
    /// per-tick top-up.
    ///
    /// Two things are specific to fluid. A tank holds no fluid id of its own,
    /// so what is being bought is the fluid of the run it discharges into —
    /// **plumbed to nothing is a no-op**, because there is nothing to charge
    /// for. And a tank's room is continuous while stock is whole units, so a
    /// part unit is billed as a whole one, capped at what the ownership clamp
    /// already allowed. (The automatic path never hits that rounding: it asks
    /// only for [`FluidTank::whole_unit_room`].)
    ///
    /// Unlike the hopper's, this does **not** mark the placement dirty —
    /// its callers do, or deliberately do not. See `auto_fill_buffers`.
    fn fill_tank_entity(&mut self, tank: Entity, amount: u32) -> Fx {
        let Some(fluid) = self.tank_fluid(tank) else {
            return Fx::ZERO;
        };
        let allowed = amount.min(self.ingredients.owned.count(&fluid));
        if allowed == 0 {
            return Fx::ZERO;
        }
        // Clamped at capacity, never an error — see `FluidTank::fill`.
        let added = {
            let mut body = self.ecs.get::<&mut FluidTank>(tank).unwrap();
            body.fill(allowed)
        };
        let debit = added.ceil().to_num::<i64>().max(0) as u32;
        self.ingredients.owned.take(&fluid, debit.min(allowed));
        added
    }

    /// [`PlayerCommand::SetHopperItem`]. Silent no-op on an empty tile, a
    /// tile that is not a hopper, an item that is not raw, or the item the
    /// hopper already holds.
    fn try_set_hopper_item(&mut self, tile: TileCoord, item: &str) -> bool {
        if !self.raw_items.iter().any(|raw| raw == item) {
            return false;
        }
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        let Ok(mut hopper) = self.ecs.get::<&mut Hopper>(entity) else {
            return false;
        };
        if hopper.item() == item {
            // Re-selecting the current item must not empty the bin.
            return false;
        }
        hopper.retarget(item.to_string());
        drop(hopper);
        self.mark_placement_dirty(entity);
        true
    }

    /// [`PlayerCommand::SetHopperAutoFill`]. Silent no-op on an empty tile
    /// or one that is not a hopper; setting the value it already has changes
    /// and dirties nothing.
    fn try_set_hopper_auto_fill(&mut self, tile: TileCoord, enabled: bool) -> bool {
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        let Ok(mut hopper) = self.ecs.get::<&mut Hopper>(entity) else {
            return false;
        };
        if hopper.auto_fill() == enabled {
            return false;
        }
        hopper.set_auto_fill(enabled);
        drop(hopper);
        self.mark_placement_dirty(entity);
        true
    }

    /// [`PlayerCommand::SetTankAutoFill`] — `try_set_hopper_auto_fill`'s
    /// twin, against a tank.
    fn try_set_tank_auto_fill(&mut self, tile: TileCoord, enabled: bool) -> bool {
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        let Ok(mut tank) = self.ecs.get::<&mut FluidTank>(entity) else {
            return false;
        };
        if tank.auto_fill() == enabled {
            return false;
        }
        tank.set_auto_fill(enabled);
        drop(tank);
        self.mark_placement_dirty(entity);
        true
    }

    /// Which of `candidates` a tile-addressed command applies to: the one
    /// whose footprint covers `tile`, or — when no tile was given — all of
    /// them. See [`PlayerCommand::FillHopper`] for why "all of them" is the
    /// default.
    fn targets(&self, candidates: &[Entity], tile: Option<TileCoord>) -> Vec<Entity> {
        match tile {
            None => candidates.to_vec(),
            Some(tile) => match self.topology.grid.get(&tile) {
                Some(&entity) if candidates.contains(&entity) => vec![entity],
                _ => Vec::new(),
            },
        }
    }
}

impl World {
    // -----------------------------------------------------------------
    // Snapshots and queries
    // -----------------------------------------------------------------

    /// Chunks touched since the start of the current `tick()` call
    /// (ARCHITECTURE.md §4), plus any touched by a `Place`/`Remove`/`Rotate`
    /// issued since. Godot only needs to re-read chunks that both changed
    /// AND are currently visible.
    pub fn dirty_chunks(&self) -> &[ChunkId] {
        &self.dirty
    }

    /// Every chunk anything currently placed occupies, in a deterministic
    /// order. Not part of `SimBridge`'s surface (Godot only ever needs
    /// `dirty_chunks`) — this exists so tooling/tests can get a complete
    /// view of the whole world without hardcoding how many chunks it spans.
    /// Empty for a fresh world, which now contains nothing.
    pub fn all_chunks(&self) -> Vec<ChunkId> {
        let mut ids = Vec::new();
        for &tile in self.topology.grid.keys() {
            let chunk = chunk_of(tile);
            if !ids.contains(&chunk) {
                ids.push(chunk);
            }
        }
        ids
    }

    pub fn chunk_snapshot(&self, id: ChunkId) -> ChunkSnapshot {
        let mut snapshot = ChunkSnapshot::default();
        for &belt in &self.topology.belts {
            let tile = self.position_of(belt);
            if chunk_of(tile) == id {
                snapshot.belts.push(self.belt_snapshot(belt));
            }
        }
        for &splitter in &self.topology.splitters {
            let tile = self.position_of(splitter);
            if chunk_of(tile) == id {
                snapshot.splitters.push(self.splitter_snapshot(splitter));
            }
        }
        // Anything with a footprint is reported for every chunk it
        // overlaps, not only the chunk holding its origin tile — see
        // `MachineSnapshot`'s doc for the de-duplication that implies for a
        // consumer stitching several chunks together.
        for &machine in &self.topology.machines {
            if self.overlaps_chunk(machine, id) {
                snapshot.machines.push(self.machine_snapshot(machine));
            }
        }
        for (&tile, &(network, segment)) in &self.topology.pipe_index {
            if chunk_of(tile) == id {
                snapshot.pipes.push(pipe_snapshot(
                    &self.topology.networks[network].segments()[segment],
                ));
            }
        }
        for &hopper in &self.topology.hoppers {
            if self.overlaps_chunk(hopper, id) {
                snapshot.hoppers.push(self.hopper_snapshot(hopper));
            }
        }
        for &tank in &self.topology.tanks {
            if self.overlaps_chunk(tank, id) {
                snapshot.fluid_tanks.push(self.fluid_tank_snapshot(tank));
            }
        }
        for &dock in &self.topology.docks {
            if self.overlaps_chunk(dock, id) {
                snapshot.docks.push(self.dock_snapshot(dock));
            }
        }
        snapshot
    }

    fn overlaps_chunk(&self, entity: Entity, id: ChunkId) -> bool {
        let placement = self.placement_of(entity);
        let origin = self.position_of(entity);
        footprint_chunks(origin, placement.footprint.rotated(placement.direction)).contains(&id)
    }

    /// Everything currently placed, in tile order — see
    /// [`PlacementSnapshot`], and note this **replaces `chain_layout`**.
    pub fn world_layout(&self) -> Vec<PlacementSnapshot> {
        self.topology
            .all
            .iter()
            .map(|&entity| {
                let placement = self.placement_of(entity);
                let origin = self.position_of(entity);
                PlacementSnapshot {
                    recipe: self
                        .ecs
                        .get::<&Machine>(entity)
                        .ok()
                        .map(|machine| machine.recipe.clone()),
                    links: self.topology.links.get(&entity).cloned(),
                    occupies: placement.tiles(origin),
                    kind: placement.kind,
                    tile: origin,
                    direction: placement.direction,
                    footprint: placement.footprint,
                }
            })
            .collect()
    }

    /// Every raw-ingredient hopper in the world, in tile order — placement
    /// plus live fill level. Empty in a world with none, several in a world
    /// with several (see this module's doc).
    ///
    /// Read this the way `economy_snapshot` is read rather than the way
    /// `chunk_snapshot` is: polling it whenever the fill UI redraws is cheap
    /// and always current. Unlike fluid, a hopper's level *does* dirty its
    /// chunk when it changes (at most once per tick, not every tick), so a
    /// purely chunk-driven renderer stays correct too — `ChunkSnapshot`
    /// carries the identical struct.
    pub fn hopper_snapshots(&self) -> Vec<HopperSnapshot> {
        self.topology
            .hoppers
            .iter()
            .map(|&hopper| self.hopper_snapshot(hopper))
            .collect()
    }

    /// Every recipe-running machine's load and stall, in the topology's
    /// stable (tile) order — one [`MachineLoadEntry`] per machine, keyed by
    /// origin tile; never a hopper, tank or dock, which run no recipe.
    ///
    /// **Poll this; do not wait for a dirty chunk.** A load window changes
    /// every tick on every machine, so dirtying for it would defeat the
    /// per-chunk delta rule (ARCHITECTURE.md §4) — the same treatment fluid
    /// movement gets. It carries nothing but the readout, so polling it
    /// every few ticks is cheap. `ChunkSnapshot` and `query_tile` carry the
    /// identical `load`/`stall` objects for a consumer that already has one.
    pub fn machine_load_snapshots(&self) -> Vec<MachineLoadEntry> {
        self.topology
            .machines
            .iter()
            .map(|&machine| {
                let body = self.ecs.get::<&Machine>(machine).unwrap();
                let load = MachineLoadSnapshot::of(&body.load);
                drop(body);
                MachineLoadEntry {
                    tile: self.position_of(machine),
                    load,
                    stall: self.machine_stall(machine),
                }
            })
            .collect()
    }

    /// Every fluid tank in the world, in tile order — placement plus live
    /// level.
    ///
    /// **Poll this every frame you want to redraw a tank — do not wait for a
    /// dirty chunk.** A tank is drained by the same per-tick equalization
    /// pass the pipes are, which deliberately does not dirty anything, so a
    /// falling level is invisible to `dirty_chunks`. Only `FillFluidTank`
    /// dirties it, since that is a discrete player action a chunk-driven
    /// renderer would otherwise miss entirely.
    pub fn fluid_tank_snapshots(&self) -> Vec<FluidTankSnapshot> {
        self.topology
            .tanks
            .iter()
            .map(|&tank| self.fluid_tank_snapshot(tank))
            .collect()
    }

    /// Every delivery dock in the world, in tile order. See
    /// [`DockSnapshot`] for why `contents` is normally empty and there is no
    /// `fill` field.
    pub fn dock_snapshots(&self) -> Vec<DockSnapshot> {
        self.topology
            .docks
            .iter()
            .map(|&dock| self.dock_snapshot(dock))
            .collect()
    }

    /// Every fluid network in the world, in the order the topology pass
    /// found them (which is the tile order of their first segment). Empty
    /// when no pipe is placed. See [`FluidSnapshot`] — and note this
    /// returns a **list** now, where the pre-placement world had at most one.
    pub fn fluid_snapshots(&self) -> Vec<FluidSnapshot> {
        self.topology
            .networks
            .iter()
            .enumerate()
            .map(|(index, network)| {
                let total_capacity = network.total_capacity();
                FluidSnapshot {
                    fluid: network.fluid.clone(),
                    total_volume: network.total_volume().to_num::<f64>(),
                    total_capacity: total_capacity.to_num::<f64>(),
                    pressure: if total_capacity == Fx::ZERO {
                        0.0
                    } else {
                        (network.total_volume() / total_capacity).to_num::<f64>()
                    },
                    segments: network.segments().iter().map(pipe_snapshot).collect(),
                    taps: self.tap_snapshots(&self.topology.machine_taps, index, false),
                    producers: self.tap_snapshots(&self.topology.machine_output_taps, index, true),
                    tank_tiles: self
                        .topology
                        .tank_discharges
                        .iter()
                        .filter(|&&(_, net, _)| net == index)
                        .map(|&(tank, _, _)| self.position_of(tank))
                        .collect(),
                }
            })
            .collect()
    }

    /// One end's worth of [`FluidTapSnapshot`]s for network `index` —
    /// shared by `taps` and `producers`, which differ only in which list
    /// they walk and which side of the recipe they report a quantity from.
    fn tap_snapshots(
        &self,
        links: &[(Entity, usize, usize)],
        index: usize,
        producing: bool,
    ) -> Vec<FluidTapSnapshot> {
        links
            .iter()
            .filter(|&&(_, net, _)| net == index)
            .filter_map(|&(machine, _, segment)| {
                let body = self.ecs.get::<&Machine>(machine).ok()?;
                let units_per_craft = self
                    .recipes
                    .get(&body.recipe)
                    .and_then(|r| {
                        let side = if producing {
                            &r.fluid_outputs
                        } else {
                            &r.fluid_inputs
                        };
                        side.first().map(|(_, units)| *units)
                    })
                    .unwrap_or(0);
                Some(FluidTapSnapshot {
                    machine_tile: self.position_of(machine),
                    machine_recipe: body.recipe.clone(),
                    units_per_craft,
                    segment_index: segment,
                })
            })
            .collect()
    }

    fn belt_snapshot(&self, belt: Entity) -> BeltSnapshot {
        let segment = self.ecs.get::<&BeltSegment>(belt).unwrap();
        BeltSnapshot {
            tile: self.position_of(belt),
            direction: self.placement_of(belt).direction,
            items: segment
                .snapshot_positions()
                .into_iter()
                .map(|(item, pos)| (item, pos.to_num::<f64>()))
                .collect(),
        }
    }

    fn splitter_snapshot(&self, splitter: Entity) -> SplitterSnapshot {
        let placement = self.placement_of(splitter);
        let segment = self.ecs.get::<&BeltSegment>(splitter).unwrap();
        SplitterSnapshot {
            tile: self.position_of(splitter),
            kind: placement.kind,
            direction: placement.direction,
            footprint: placement.footprint,
            items: segment
                .snapshot_positions()
                .into_iter()
                .map(|(item, pos)| (item, pos.to_num::<f64>()))
                .collect(),
            outputs: self.splitter_output_tiles(splitter),
            next_output: self
                .ecs
                .get::<&Splitter>(splitter)
                .expect("a splitter placement carries a Splitter")
                .due(),
        }
    }

    /// The tile each of a splitter's outputs currently hands to, read off
    /// the topology's own links rather than re-derived — so what is reported
    /// can never disagree with where an item actually goes (the same rule
    /// [`Self::outbound_belt_tile`] follows).
    fn splitter_output_tiles(&self, splitter: Entity) -> [Option<TileCoord>; 2] {
        self.topology
            .splitter_outputs
            .iter()
            .find(|&&(entity, _)| entity == splitter)
            .map(|&(_, outputs)| outputs.map(|target| target.map(|t| self.position_of(t))))
            .unwrap_or([None, None])
    }

    fn machine_snapshot(&self, machine: Entity) -> MachineSnapshot {
        let placement = self.placement_of(machine);
        let body = self.ecs.get::<&Machine>(machine).unwrap();
        MachineSnapshot {
            tile: self.position_of(machine),
            kind: placement.kind,
            direction: placement.direction,
            footprint: placement.footprint,
            recipe: body.recipe.clone(),
            process: body.process.map(|p| (p.start_tick, p.required_ticks)),
            input: body.input.iter().cloned().collect(),
            output: body.output.iter().cloned().collect(),
            load: MachineLoadSnapshot::of(&body.load),
            stall: self.machine_stall(machine),
        }
    }

    /// Why `machine` is waiting at this moment, if it is — see [`Stall`] for
    /// the rule. Read-only: it asks the same questions `Machine::tick` asks
    /// (enough input? a tap that can supply? room to deposit?) against the
    /// same taps `tick` would hand it, without moving anything.
    fn machine_stall(&self, machine: Entity) -> Option<Stall> {
        let body = self.ecs.get::<&Machine>(machine).ok()?;
        // The six older kinds first; only when none of them applies can a
        // crew refusal be the reason (it is asked for only on a tick the
        // machine would otherwise have worked, so the two never compete).
        self.machine_supply_stall(machine, &body).or_else(|| {
            body.refused_by.as_ref().map(|category| Stall {
                kind: StallKind::Staff,
                item: category.clone(),
            })
        })
    }

    /// The input/output half of [`Self::machine_stall`]: the six kinds that
    /// predate staff, in their original precedence.
    fn machine_supply_stall(&self, machine: Entity, body: &Machine) -> Option<Stall> {
        let recipe = self.recipes.get(&body.recipe)?;
        let tap_of = |taps: &[(Entity, usize, usize)]| {
            taps.iter()
                .find(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, segment)| (network, segment))
        };
        let stall = |kind: StallKind, item: &str| {
            Some(Stall {
                kind,
                item: item.to_string(),
            })
        };
        match body.process {
            None => {
                for (item, qty) in &recipe.inputs {
                    if !body.input.has_at_least(item, *qty) {
                        return stall(StallKind::Input, item);
                    }
                }
                let tap = tap_of(&self.topology.machine_taps);
                for (fluid, units) in &recipe.fluid_inputs {
                    match tap {
                        None => return stall(StallKind::NoSupplyPipe, fluid),
                        Some((network, segment)) => {
                            if !self.topology.networks[network].can_supply_at(
                                segment,
                                fluid,
                                Fx::from_num(*units),
                            ) {
                                return stall(StallKind::FluidInput, fluid);
                            }
                        }
                    }
                }
                None
            }
            Some(process) => {
                if !process.is_complete(self.tick_count) {
                    return None;
                }
                for (item, qty) in &recipe.outputs {
                    if !body.output.has_room_for(item, *qty) {
                        return stall(StallKind::Output, item);
                    }
                }
                let tap = tap_of(&self.topology.machine_output_taps);
                for (fluid, units) in &recipe.fluid_outputs {
                    match tap {
                        None => return stall(StallKind::NoOutletPipe, fluid),
                        Some((network, segment)) => {
                            if !self.topology.networks[network].can_accept_at(
                                segment,
                                fluid,
                                Fx::from_num(*units),
                            ) {
                                return stall(StallKind::FluidOutput, fluid);
                            }
                        }
                    }
                }
                None
            }
        }
    }

    fn hopper_snapshot(&self, hopper: Entity) -> HopperSnapshot {
        let placement = self.placement_of(hopper);
        let body = self.ecs.get::<&Hopper>(hopper).unwrap();
        HopperSnapshot {
            tile: self.position_of(hopper),
            kind: placement.kind,
            direction: placement.direction,
            footprint: placement.footprint,
            item: body.item().clone(),
            quantity: body.quantity(),
            capacity: body.capacity(),
            fill: if body.capacity() == 0 {
                0.0
            } else {
                body.quantity() as f64 / body.capacity() as f64
            },
            auto_fill: body.auto_fill(),
            belt_tile: self.outbound_belt_tile(hopper),
        }
    }

    fn fluid_tank_snapshot(&self, tank: Entity) -> FluidTankSnapshot {
        let placement = self.placement_of(tank);
        let body = self.ecs.get::<&FluidTank>(tank).unwrap();
        let discharge = self
            .topology
            .tank_discharges
            .iter()
            .find(|&&(entity, _, _)| entity == tank)
            .map(|&(_, network, segment)| (network, segment));
        FluidTankSnapshot {
            tile: self.position_of(tank),
            kind: placement.kind,
            direction: placement.direction,
            footprint: placement.footprint,
            fluid: discharge.and_then(|(network, _)| self.topology.networks[network].fluid.clone()),
            quantity: body.volume().to_num::<f64>(),
            capacity: body.capacity().to_num::<f64>(),
            // `FluidTank::pressure` is the tank's own fill fraction, which is
            // exactly what this field means — not its `discharge_pressure`,
            // which is a flow-model quantity and is always 1.
            fill: body.pressure().to_num::<f64>(),
            auto_fill: body.auto_fill(),
            pipe_tile: discharge
                .map(|(network, segment)| self.topology.networks[network].segments()[segment].tile),
        }
    }

    fn dock_snapshot(&self, dock: Entity) -> DockSnapshot {
        let placement = self.placement_of(dock);
        let body = self.ecs.get::<&Dock>(dock).unwrap();
        DockSnapshot {
            tile: self.position_of(dock),
            kind: placement.kind,
            direction: placement.direction,
            footprint: placement.footprint,
            belt_tile: self.inbound_belt_tile(dock),
            contents: body.contents().cloned().collect(),
            capacity_per_item: body.capacity_per_item(),
            shipped_total: body.shipped_total(),
            understaffed: body.understaffed(),
        }
    }

    /// The tile of the belt this placement currently discharges onto, if
    /// any — read off the topology's own links rather than re-derived, so it
    /// can never disagree with what the simulation actually does.
    fn outbound_belt_tile(&self, source: Entity) -> Option<TileCoord> {
        self.topology
            .source_to_belt
            .iter()
            .find(|&&(entity, _)| entity == source)
            .map(|&(_, belt)| self.position_of(belt))
    }

    /// The fluid a tank is currently holding, which is the fluid of the
    /// network it discharges into — read off the topology's own links, so it
    /// can never disagree with [`FluidTankSnapshot::fluid`], which is
    /// derived the same way.
    fn tank_fluid(&self, tank: Entity) -> Option<FluidId> {
        self.topology
            .tank_discharges
            .iter()
            .find(|&&(entity, _, _)| entity == tank)
            .and_then(|&(_, network, _)| self.topology.networks[network].fluid.clone())
    }

    /// The tile of the belt currently feeding this placement, if any.
    fn inbound_belt_tile(&self, sink: Entity) -> Option<TileCoord> {
        self.topology
            .belt_to_sink
            .iter()
            .find(|&&(_, entity)| entity == sink)
            .map(|&(belt, _)| self.position_of(belt))
    }

    /// What occupies `pos`, if anything (ARCHITECTURE.md §4). A multi-tile
    /// placement resolves from **any** tile of its footprint, not just its
    /// origin — see [`TileSnapshot`].
    pub fn query_tile(&self, pos: TileCoord) -> Option<TileSnapshot> {
        let &entity = self.topology.grid.get(&pos)?;
        let placement = self.placement_of(entity);
        let origin = self.position_of(entity);
        let snapshot = match self.body_kind(entity)? {
            BodyKind::Belt => {
                let segment = self.ecs.get::<&BeltSegment>(entity).unwrap();
                TileSnapshot::Belt {
                    direction: placement.direction,
                    items_here: segment
                        .snapshot_positions()
                        .into_iter()
                        .map(|(item, _)| item)
                        .collect(),
                    links: self.conveyor_links_of(entity),
                }
            }
            BodyKind::Splitter => {
                let segment = self.ecs.get::<&BeltSegment>(entity).unwrap();
                TileSnapshot::Splitter {
                    direction: placement.direction,
                    items_here: segment
                        .snapshot_positions()
                        .into_iter()
                        .map(|(item, _)| item)
                        .collect(),
                    outputs: self.splitter_output_tiles(entity),
                    next_output: self
                        .ecs
                        .get::<&Splitter>(entity)
                        .expect("a splitter placement carries a Splitter")
                        .due(),
                    links: self.conveyor_links_of(entity),
                }
            }
            BodyKind::Machine => {
                let body = self.ecs.get::<&Machine>(entity).unwrap();
                let crew = crew_entries(self.crew_of_kind(&placement.kind));
                TileSnapshot::Machine {
                    machine_kind: placement.kind,
                    direction: placement.direction,
                    footprint: placement.footprint,
                    origin_tile: origin,
                    recipe: body.recipe.clone(),
                    process: body.process.map(|p| (p.start_tick, p.required_ticks)),
                    load: MachineLoadSnapshot::of(&body.load),
                    stall: {
                        drop(body);
                        self.machine_stall(entity)
                    },
                    crew,
                }
            }
            BodyKind::Pipe => {
                let &(network_index, segment_index) = self.topology.pipe_index.get(&pos)?;
                let network = &self.topology.networks[network_index];
                let segment = &network.segments()[segment_index];
                TileSnapshot::Pipe {
                    direction: placement.direction,
                    fluid: network.fluid.clone(),
                    volume: segment.volume.to_num::<f64>(),
                    capacity: segment.capacity.to_num::<f64>(),
                    pressure: segment.pressure().to_num::<f64>(),
                    is_source: segment.is_source,
                    is_inlet: self
                        .topology
                        .machine_taps
                        .iter()
                        .any(|&(_, net, seg)| net == network_index && seg == segment_index),
                    links: match self.topology.links.get(&entity) {
                        Some(Links::Pipe(pipe)) => pipe.clone(),
                        _ => unreachable!("every pipe has PipeLinks in the topology"),
                    },
                }
            }
            BodyKind::Hopper => {
                let snapshot = self.hopper_snapshot(entity);
                TileSnapshot::Hopper {
                    direction: snapshot.direction,
                    footprint: snapshot.footprint,
                    origin_tile: snapshot.tile,
                    item: snapshot.item,
                    quantity: snapshot.quantity,
                    capacity: snapshot.capacity,
                    fill: snapshot.fill,
                    auto_fill: snapshot.auto_fill,
                }
            }
            BodyKind::FluidTank => {
                let snapshot = self.fluid_tank_snapshot(entity);
                TileSnapshot::FluidTank {
                    direction: snapshot.direction,
                    footprint: snapshot.footprint,
                    origin_tile: snapshot.tile,
                    fluid: snapshot.fluid,
                    quantity: snapshot.quantity,
                    capacity: snapshot.capacity,
                    fill: snapshot.fill,
                    auto_fill: snapshot.auto_fill,
                }
            }
            BodyKind::Dock => {
                let snapshot = self.dock_snapshot(entity);
                TileSnapshot::Dock {
                    direction: snapshot.direction,
                    footprint: snapshot.footprint,
                    origin_tile: snapshot.tile,
                    contents: snapshot.contents,
                    capacity_per_item: snapshot.capacity_per_item,
                    shipped_total: snapshot.shipped_total,
                    crew: crew_entries(self.crew_of_kind(&placement.kind)),
                    understaffed: snapshot.understaffed,
                }
            }
        };
        Some(snapshot)
    }
}

// =====================================================================
// The save/load seam (`super::save`)
// =====================================================================

impl World {
    /// Every placement, in the topology's own tile order, as
    /// `(origin tile, placement, entity)`.
    ///
    /// This block, and only this block, is what [`super::save`] reaches
    /// through: a save has to read state this module otherwise keeps to
    /// itself, and a load has to write state no [`PlayerCommand`] can
    /// express (a machine's craft progress, a dock's lifetime total, an
    /// exact money balance). Everything here is therefore `pub(super)` —
    /// visible to the save module, invisible outside the crate, and never
    /// part of the player-facing API. See `save.rs`'s module doc for the
    /// rule that keeps it that way.
    pub(super) fn saved_placements(&self) -> Vec<(TileCoord, Placement, Entity)> {
        self.topology
            .all
            .iter()
            .map(|&entity| (self.position_of(entity), self.placement_of(entity), entity))
            .collect()
    }

    /// Whatever claims `tile`, if anything — any tile of a footprint, like
    /// every other tile-addressed lookup here.
    pub(super) fn entity_at(&self, tile: TileCoord) -> Option<Entity> {
        self.topology.grid.get(&tile).copied()
    }

    pub(super) fn ecs(&self) -> &hecs::World {
        &self.ecs
    }

    pub(super) fn ecs_mut(&mut self) -> &mut hecs::World {
        &mut self.ecs
    }

    /// How much fluid the pipe tile at `tile` holds, and how much it could —
    /// `None` when no pipe stands there. Volume lives on whichever network
    /// currently owns the tile (`fluid.rs`), which is why this is addressed
    /// by tile rather than by network index: a network is derived and its
    /// indices are not stable across a rebuild, but a tile is exactly what
    /// the player placed.
    pub(super) fn pipe_volume_at(&self, tile: TileCoord) -> Option<(Fx, Fx)> {
        let &(network, segment) = self.topology.pipe_index.get(&tile)?;
        let segment = self.topology.networks[network].segments().get(segment)?;
        Some((segment.volume, segment.capacity))
    }

    /// Sets a pipe tile's volume outright, for a load. Returns whether a
    /// pipe was actually there.
    pub(super) fn restore_pipe_volume(&mut self, tile: TileCoord, volume: Fx) -> bool {
        let Some(&(network, segment)) = self.topology.pipe_index.get(&tile) else {
            return false;
        };
        self.topology.networks[network].restore_volume_at(segment, volume);
        true
    }

    /// Sets the money balance outright. Money otherwise only ever moves
    /// through [`Economy::try_spend`] and a contract payout, deliberately
    /// ([`World::with_content_and_money`]'s own doc) — a load is the one
    /// caller that legitimately states a balance rather than earning it, and it
    /// has to do so twice: once to fund each rebuild purchase, and once at
    /// the end to install the saved balance, so those purchases cannot leak
    /// into it.
    pub(super) fn restore_money(&mut self, money: Fx) {
        self.economy.money = money;
    }

    /// Installs a loaded production ledger wholesale. Restore-only, like
    /// every other `restore_*`: the tick path records, it never replaces.
    pub(super) fn restore_production(&mut self, ledger: ProductionLedger) {
        self.production = ledger;
    }

    pub(super) fn restore_tick_count(&mut self, tick: u64) {
        self.tick_count = tick;
    }

    pub(super) fn restore_staff(&mut self, staff: StaffPools) {
        self.staff = staff;
    }

    pub(super) fn economy_mut(&mut self) -> &mut Economy {
        &mut self.economy
    }

    pub(super) fn shop(&self) -> &Shop {
        &self.shop
    }

    pub(super) fn shop_mut(&mut self) -> &mut Shop {
        &mut self.shop
    }

    pub(super) fn ingredients(&self) -> &Ingredients {
        &self.ingredients
    }

    pub(super) fn ingredients_mut(&mut self) -> &mut Ingredients {
        &mut self.ingredients
    }

    /// The footprint `kind` is placed with, or `None` for a kind this world
    /// cannot construct at all — `try_place`'s own first question, asked by
    /// [`super::save`] before it places anything so a save naming a kind
    /// this build no longer has fails with a message about *that* rather
    /// than as a generic refusal.
    pub(super) fn footprint_of_kind(&self, kind: &str) -> Option<Footprint> {
        self.footprint_for(kind)
    }

    /// Every solid item id this world's recipes mention, consumed or
    /// produced, sorted and deduplicated. There is no `items.ron`, so this
    /// is the closest thing to an item registry the crate has — and it is
    /// what [`super::save`] checks a loaded belt, buffer, dock or delivery
    /// pool against, since an item id that no recipe names any more is a
    /// content change the save cannot survive.
    pub(super) fn recipe_item_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .recipes
            .ordered()
            .iter()
            .flat_map(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .chain(recipe.outputs.iter())
                    .map(|(item, _)| item.clone())
            })
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Marks every chunk the world occupies dirty, so a presentation layer
    /// driven by `dirty_chunks()` redraws all of a freshly loaded world
    /// rather than only whatever the next tick happens to touch. Each
    /// individual `Place` during a load already dirties what it touched;
    /// this is what makes the load *as a whole* announce itself.
    pub(super) fn mark_every_chunk_dirty(&mut self) {
        self.dirty = self.all_chunks();
    }
}

/// Enforces the chain-linkage assumption `data/recipes.ron` is written
/// under: recipe `i + 1` must consume (as one of its inputs) an item that
/// recipe `i` produces. A clear panic here — at construction time, before
/// any tick runs — is the data-file equivalent of a compiler error.
///
/// **`World` no longer lays a chain out from that order** (the "Real
/// Placement" epic — the player does), so this is no longer load-bearing for
/// geometry. It is kept because it is still load-bearing for *content*: the
/// file describes one production line, "the first recipe's first input is
/// the chain's raw ingredient" (which is what a placed hopper holds) is only
/// true if it does, and [`super::reference::reference_chain`] builds that
/// line in that order.
fn validate_chain(ordered: &[Recipe]) {
    for pair in ordered.windows(2) {
        let (prev, next) = (&pair[0], &pair[1]);
        let links_by_item = next
            .inputs
            .iter()
            .any(|(item, _)| prev.outputs.iter().any(|(out_item, _)| out_item == item));
        // The fluid half, added by the chain-rework epic. Two stages joined
        // by a pipe share no solid id at all — the mixer's whole output is a
        // liquid and the cooking vat's whole input is one — so without this
        // the middle of the shipped chain would look like a break in the
        // line. A typo'd *fluid* id is exactly as much a content error as a
        // typo'd item id, and gets exactly the same construction-time panic.
        let links_by_fluid = next.fluid_inputs.iter().any(|(fluid, _)| {
            prev.fluid_outputs
                .iter()
                .any(|(out_fluid, _)| out_fluid == fluid)
        });
        assert!(
            links_by_item || links_by_fluid,
            "recipes.ron chain error: recipe '{}' (outputs {:?}, fluid_outputs {:?}) does not \
             feed recipe '{}' (inputs {:?}, fluid_inputs {:?}) — the file describes one \
             production line, so consecutive recipes must chain by a shared id, solid or fluid",
            prev.id,
            prev.outputs,
            prev.fluid_outputs,
            next.id,
            next.inputs,
            next.fluid_inputs
        );
    }
}

/// Two distinct networks out of one `Vec`, as a pair of mutable borrows.
///
/// Exists so [`World::tick`] can hand a machine an input tap and an output
/// tap at the same time. `a != b` is an invariant of the topology pass —
/// a machine never draws from the run it fills (see
/// [`World::recompute_topology`]) — and this asserts it rather than trusting
/// it, because getting it wrong would be aliasing rather than a wrong
/// answer.
fn two_networks_mut(
    networks: &mut [FluidNetwork],
    a: usize,
    b: usize,
) -> (&mut FluidNetwork, &mut FluidNetwork) {
    assert_ne!(
        a, b,
        "a machine's input and output taps must be different networks — the topology pass          resolves the output first and excludes it when choosing the input"
    );
    let (lo, hi) = (a.min(b), a.max(b));
    let (left, right) = networks.split_at_mut(hi);
    if a < b {
        (&mut left[lo], &mut right[0])
    } else {
        (&mut right[0], &mut left[lo])
    }
}

/// Every chunk (ARCHITECTURE.md §3.1's 32x32 grid) a **rotated** footprint
/// placed at `origin` overlaps, in a deterministic row-major order with the
/// origin's own chunk first.
///
/// Callers pass `footprint.rotated(direction)`: this function is pure
/// rectangle arithmetic and deliberately knows nothing about facing, so
/// there is exactly one place the rotation rule is applied
/// ([`Footprint::rotated`]) and exactly one place the chunk fan-out is
/// computed (here). The multi-chunk case is *handled*, not rejected — see
/// [`MachineSnapshot`]'s doc for what that means for a consumer.
fn footprint_chunks(origin: TileCoord, footprint: Footprint) -> Vec<ChunkId> {
    let far = (
        origin.0 + footprint.width_tiles - 1,
        origin.1 + footprint.height_tiles - 1,
    );
    let (min_cx, min_cy) = chunk_of(origin);
    let (max_cx, max_cy) = chunk_of(far);
    debug_assert!(
        min_cx <= max_cx && min_cy <= max_cy,
        "footprint extents must be positive"
    );
    let mut chunks = Vec::with_capacity(
        (((max_cx - min_cx + 1) as i64) * ((max_cy - min_cy + 1) as i64)) as usize,
    );
    for cy in min_cy..=max_cy {
        for cx in min_cx..=max_cx {
            chunks.push((cx, cy));
        }
    }
    debug_assert_eq!(chunks[0], chunk_of(origin));
    chunks
}

/// Whether a body carries solid items in a [`BeltSegment`] — a belt tile or
/// a splitter. The single answer to "can this thing be handed an item the
/// way a belt can", so belt linking, machine/hopper discharge and a
/// splitter's own outputs cannot disagree about it.
fn is_belt_like(body: BodyKind) -> bool {
    matches!(body, BodyKind::Belt | BodyKind::Splitter)
}

fn pipe_snapshot(segment: &PipeSegment) -> PipeSnapshot {
    PipeSnapshot {
        tile: segment.tile,
        direction: segment.direction,
        volume: segment.volume.to_num::<f64>(),
        capacity: segment.capacity.to_num::<f64>(),
        pressure: segment.pressure().to_num::<f64>(),
        is_source: segment.is_source,
    }
}

/// Enforces the one fluid assumption that survived placement: a machine is
/// plumbed into exactly **one** network, so a recipe may declare at most one
/// fluid input. A data file asking for two would otherwise get the second
/// silently ignored, with the machine simply never starting and no clue why.
///
/// What this *stopped* enforcing is the milestone-6 restriction that a whole
/// recipe file could contain at most one fluid-using recipe. That existed
/// because `World` built one hardcoded pipe run; a world now holds one
/// network per connected component of placed pipe, so several fluid recipes
/// are simply several runs the player may or may not have built.
fn validate_fluid_scope(ordered: &[Recipe]) {
    for recipe in ordered {
        assert!(
            recipe.fluid_inputs.len() <= 1,
            "recipes.ron fluid error: recipe '{}' declares {} fluid inputs, but a machine taps \
             exactly one network to draw from",
            recipe.id,
            recipe.fluid_inputs.len()
        );
        assert!(
            recipe.fluid_outputs.len() <= 1,
            "recipes.ron fluid error: recipe '{}' declares {} fluid outputs, but a machine \
             deposits into exactly one network — the one off its outlet face",
            recipe.id,
            recipe.fluid_outputs.len()
        );
    }
}
