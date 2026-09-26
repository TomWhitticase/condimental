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
//! whatever belt is adjacent to its outlet face. `recipes.ron` still
//! *describes* a line — [`validate_chain`] enforces that its recipes form
//! one connected graph, since that is what makes the shipped content a
//! coherent line and what [`super::reference::reference_chain`] builds —
//! but nothing in the simulation reads its order as geometry any more. Its
//! order survives as one thing only: the tiebreak when a machine kind could
//! run more than one of its recipes (see "Which recipe a machine runs",
//! below).
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
//! A second dock is simply a second place boxes leave the factory; each
//! holds what arrived at it, and contracts draw on all of them at once
//! (`super::dock`'s module doc).
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
//!   vinegar from the run merely touching it. A machine's input taps are
//!   therefore **every** adjacent pipe network that is *not* the output
//!   network — so a loop of pipe touching a machine on both sides is read
//!   as its outlet and gives it no inlet at all, rather than as a machine
//!   quietly feeding itself.
//!
//!   Plural since the BBQ epic: a recipe may draw several fluids (the
//!   cooking vat takes bbq mix and liquid smoke), and each is matched at
//!   tick time to a *distinct* touching run **by fluid id**, never by
//!   position. One run per input, first fit in tile order. A fluid output
//!   is still exactly one, off the outlet face ([`validate_fluid_scope`]).
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
//! # What a pipe run carries
//!
//! A network's fluid id is decided by content and not by the player, and
//! deriving it is the subtlest pass in this file ([`resolve_network_fluids`]
//! has the mechanism; this is the shape of the problem).
//!
//! It used to be one line of code. A machine kind named exactly one recipe,
//! so the first machine to reach an unclaimed run named it from that
//! recipe, drawing or depositing alike, and producer and consumer agreed by
//! naming the same id in `recipes.ron`. The BBQ epic ends that: a kind may
//! now run several recipes and a placed machine picks whichever its inputs
//! satisfy, so a mixer's outlet run is `ketchup_mix` or `bbq_mix` depending
//! on what is plumbed into the mixer — which is the very thing being
//! derived. Asking each machine in turn would make the answer depend on
//! which machine was walked first, and this simulation does not get to have
//! answers like that (ARCHITECTURE.md §3.9).
//!
//! So it is derived for the whole graph at once, by narrowing. Each run
//! starts with every id it could possibly hold — what a machine at its end
//! could deposit, or for a run nothing fills, what a machine could draw —
//! and one constraint from outside: **a run a tank discharges into can hold
//! only a raw fluid**, because a tank is the only way into the graph and a
//! raw ingredient is the only thing that comes in from outside. Then, while
//! anything still changes: a recipe is *feasible* for a machine when its
//! output still fits its outlet run and each of its inputs can be matched
//! by id to a distinct touching run, and where every feasible recipe of a
//! kind agrees about a run, that run cannot hold anything else. A run left
//! with exactly one id carries it; two or more is genuine ambiguity, none
//! is a contradiction, and either way the run stays **unnamed** and the
//! machines on it stall rather than anything panicking.
//!
//! The sets only shrink and each pass reads the last pass's answer, so it
//! terminates and it does not depend on any iteration order — the same
//! layout built in a different placement order gets the same naming, which
//! is what `pipe_naming_is_independent_of_placement_order` asserts.
//!
//! # Which recipe a machine runs
//!
//! Whichever of its kind's recipes its **current inputs** satisfy, re-picked
//! on every idle tick and frozen for the length of a craft
//! ([`super::machine::Machine::choose`]). `data/recipes.ron`'s file order
//! breaks a tie. There is no player-facing picker and deliberately so: the
//! plumbing and the hoppers *are* the mechanism, so a mixer on a vinegar
//! pipe beside a sugar hopper blends ketchup mix and one on a ketchup pipe
//! beside spice and molasses hoppers blends BBQ mix. Because the pick is
//! live state rather than a property of the kind, it is saved
//! ([`super::save::SavedMachine::recipe`]) — a reload mid-craft has to
//! finish the craft that was started.
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

use std::collections::{BTreeMap, BTreeSet, HashSet};

use hecs::Entity;

use super::belt::BeltSegment;
use super::contracts::ContractRegistry;
use super::dock::{DOCK_CAPACITY, DOCK_KIND, Dock};
use super::economy::{ContractEvent, DeliveryStock, Economy, EconomySnapshot, STARTING_MONEY};
use super::fluid::{
    FLUID_TANK_CAPACITY, FLUID_TANK_FILL_AMOUNT, FLUID_TANK_KIND, FluidNetwork, FluidTank,
    FluidTap, PipeNaming, PipeSegment,
};
use super::guide::GuideRegistry;
use super::hopper::{HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, HOPPER_KIND, Hopper};
use super::ingredients::{
    DEFAULT_BUY_QUANTITY, IngredientCatalog, Ingredients, IngredientsSnapshot, raw_ingredient_ids,
};
use super::inventory::Inventory;
use super::machine::{LOAD_WINDOW_TICKS, LoadWindow, Machine, WorkState};
use super::machines::{Footprint, MachineRegistry};
use super::recipe::{Recipe, RecipeRegistry};
use super::shop::{DEFAULT_PURCHASE_QUANTITY, Shop, ShopCatalog, ShopSnapshot};
use super::splitter::{SPLITTER_KIND, Side, Splitter};
use super::staff::{
    CrewEntry, CrewRequest, CrewSource, EstablishmentEntry, StaffPools, StaffSnapshot, crew_entries,
};
use super::stats::{ProductionLedger, ProductionSnapshot, StatKind};
use super::storage::{StorageCatalog, WAREHOUSE_RACK_KIND, WarehouseRack};
use super::types::{ChunkId, Direction, FluidId, Fx, ItemId, Money, RecipeId, TileCoord, chunk_of};

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
    /// Declares that the tank at `tile` holds `fluid` — the fluid twin of
    /// [`Self::SetHopperItem`], and the answer to issue #43.
    ///
    /// A tank's fluid used to be *derived*: whatever the run it discharges
    /// into turned out to carry. That works while the run can be worked
    /// out, and one shape defeats it — a machine drawing two raw fluids
    /// from two tank-fed runs, where both runs seed to the same pair and
    /// nothing tells them apart. Both stay unnamed, the machine stalls, and
    /// no command the player could issue says which drum is which. This is
    /// that command; see [`super::fluid::FluidTank`] for why the
    /// declaration is an input to the naming rather than a second copy of
    /// its answer.
    ///
    /// A silent no-op on an empty tile, on a tile that is not a tank, and
    /// on a fluid that is not raw ([`World::raw_fluids`] — a tank is how
    /// fluid enters the graph from outside, and only raws come in).
    /// Declaring what the tank **already** holds is a no-op, contents and
    /// all, for [`Self::SetHopperItem`]'s reason: a picker redraws its own
    /// selection. A real change **empties the tank** (that volume is the
    /// old fluid) and keeps the auto-fill flag, exactly as a hopper
    /// retarget does — and, unlike a hopper retarget, it re-runs the
    /// topology pass, because what a declared tank holds is one of the
    /// facts [`resolve_network_fluids`] starts from.
    SetTankFluid { tile: TileCoord, fluid: FluidId },

    /// **Switches the machine at `tile` off, or back on** — mothballing
    /// (issue #19, GAMEPLAY.md §11's open question answered).
    ///
    /// A mothballed machine asks for no crew, so the ESTABLISHMENT
    /// ([`World::recompute_establishment`]) falls by that machine's whole
    /// crew the instant this lands, and the player may then FIRE the heads
    /// it was holding on the books and shrink their wage bill to something
    /// their smaller factory can pay. It also produces nothing: it starts
    /// no craft, advances no timer and deposits nothing, and a craft it was
    /// in the middle of is PAUSED where it stood, not lost.
    ///
    /// **Machines only, and deliberately so.** A hopper, tank, belt, pipe
    /// or splitter asks for no crew, so switching one off would release
    /// nobody; the dock asks for one logistics head and is the only way
    /// product becomes money, so switching it off is never the move a
    /// player in arrears wants. A tile holding any of them — or an empty
    /// tile — is a silent no-op, the `Purchase` convention, as is setting
    /// the state the machine already has.
    ///
    /// Tile-addressed like [`Self::Rotate`]: any tile of a multi-tile
    /// footprint resolves. It dirties the machine's chunk (unlike `Hire`
    /// and `Fire`, this one IS about a tile) but does not rebuild the
    /// topology: nothing about what is adjacent to what has changed, only
    /// the establishment, which is recomputed on its own.
    SetMothballed { tile: TileCoord, enabled: bool },
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

    /// **A cheat.** Credits `amount` to the balance out of nowhere, for
    /// developing the game: reaching a state worth looking at otherwise means
    /// playing to it, and a screen full of machines is fifteen minutes away
    /// from a fresh world (the settings/debug menu branch, 2026-09-21).
    ///
    /// Named `Debug…` so that nothing in this enum can be mistaken for a
    /// mechanic. Godot's F3 panel is the only thing that sends it, and it is
    /// deliberately NOT gated on a debug build — this is pre-alpha, and a
    /// review screenshot should show what an export does.
    ///
    /// It is still a real command, on the real wire, applied by the real
    /// [`super::economy::Economy`] — because money is simulation state, and
    /// the alternative (a number Godot adds to what it renders) would mean a
    /// balance the sim disagrees with and a save that forgets the grant.
    ///
    /// Refused, leaving the world byte-identical, at `amount <= 0`: there is
    /// no negative grant. Taking money away would need a rule about what
    /// happens below zero — the one thing [`super::economy::Economy`] does
    /// not currently have, since `try_spend` simply refuses — and a cheat is
    /// the worst place to invent it. Dirties no chunk: money is not a tile,
    /// exactly as [`Self::Purchase`] documents.
    DebugGrantMoney { amount: Money },
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
    Remove {
        tile: TileCoord,
    },
    /// `{"type": "rotate", "tile": [4, 0]}`.
    Rotate {
        tile: TileCoord,
    },
    /// `{"type": "set_hopper_item", "tile": [-1, 0], "item": "sugar"}` —
    /// both required: only the player knows which hopper and which
    /// ingredient.
    SetHopperItem {
        tile: TileCoord,
        item: ItemId,
    },
    /// `{"type": "sign_contract", "id": "diner_supply"}` — `id` required:
    /// only the player knows which offer they are taking.
    SignContract {
        id: String,
    },
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
    /// `{"type": "set_tank_fluid", "tile": [5, 1], "fluid": "vinegar"}` —
    /// exactly `set_hopper_item`'s shape, and both required for its reason.
    SetTankFluid {
        tile: TileCoord,
        fluid: FluidId,
    },
    /// `{"type": "set_mothballed", "tile": [12, 0], "enabled": true}` —
    /// `enabled` optional, defaulting to `true` for the auto-fill toggles'
    /// reason: a toggle sends the state it wants, and "switch it off" is
    /// the state this command exists for.
    SetMothballed {
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
    /// `{"type": "debug_grant_money", "amount": 10000}` — **whole currency
    /// units**, required, and an `i64` rather than a float precisely because
    /// nothing with a fractional part should be anywhere near this wire
    /// (ARCHITECTURE.md §3.9). A grant bigger than [`Money`] can hold
    /// saturates rather than wrapping.
    DebugGrantMoney {
        amount: i64,
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
            PlayerCommandDto::SetTankFluid { tile, fluid } => {
                PlayerCommand::SetTankFluid { tile, fluid }
            }
            PlayerCommandDto::SetTankAutoFill { tile, enabled } => {
                PlayerCommand::SetTankAutoFill { tile, enabled }
            }
            PlayerCommandDto::SetMothballed { tile, enabled } => {
                PlayerCommand::SetMothballed { tile, enabled }
            }
            PlayerCommandDto::Hire { category } => PlayerCommand::Hire { category },
            PlayerCommandDto::Fire { category } => PlayerCommand::Fire { category },
            PlayerCommandDto::DebugGrantMoney { amount } => PlayerCommand::DebugGrantMoney {
                // Saturating, not `from_num`: `Money`'s 48 integer bits do
                // not hold every `i64`, and a debug fat-finger of twenty
                // digits should land on "as much as there can be" rather than
                // panic in debug and wrap negative in release.
                amount: Money::saturating_from_num(amount),
            },
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
///   "items": [["tomato", 0.62]], "moved_ticks": 418 }
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
    /// The tick this tile last RAN on (`BeltSegment::moved_ticks`) — the
    /// clock the renderer runs the belt's surface pattern from, so a jammed
    /// tile's rubber stops with the items on it (issue #79), and every tile
    /// that is running holds the same number, so a run of them is one
    /// conveyor (amended 2026-09-25; it used to count only the ticks the
    /// tile advanced product on).
    ///
    /// **A tile missing from a delta is not a tile that stood still.** An
    /// idle belt's clock advances every tick and it is deliberately absent
    /// from the delta, because there is no product on it to redraw and a
    /// delta per empty tile per tick is the cost this field exists to
    /// avoid. The renderer carries an idle tile's phase forward itself
    /// (`main.gd`'s `_belt_idle_phase`), which is exact because the value is
    /// the tick: the number it draws is the number the tile's next snapshot
    /// brings. What a missing delta means is "nothing on this tile moved",
    /// and only a tile with items on it can send one at all.
    pub moved_ticks: u64,
}

/// One conveyor splitter (Epic B), for rendering.
///
/// JSON shape (`snake_case`, as serialized by `serde_json`):
///
/// ```json
/// { "tile": [6, 0], "kind": "splitter", "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 1 },
///   "items": [["tomato", 0.44]], "moved_ticks": 418,
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
    /// Exactly [`BeltSnapshot::moved_ticks`]: a splitter carries its items
    /// in an ordinary one-tile belt segment and is drawn as one frame, but
    /// it still reports its own clock so the two kinds' wire shapes stay
    /// interchangeable.
    pub moved_ticks: u64,
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
    /// Whether the player has switched this machine OFF
    /// ([`PlayerCommand::SetMothballed`]). Unlike `load` and `stall` beside
    /// it this one DOES dirty the chunk when it changes — it moves only
    /// when the player toggles it, which is a discrete action, so the
    /// chunk-driven renderer learns about it the same tick.
    pub mothballed: bool,
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
/// {"kind": "unnameable_pipe","item": "liquid_smoke"} // a run is here, nothing can name it
/// {"kind": "ambiguous_pipe", "item": "liquid_smoke"} // a run is here, it could be several fluids
/// ```
///
/// The last two are issue #54's, and they replace a *wrong* answer rather
/// than filling a silence: a machine on a run the naming pass could not
/// name used to report `no_supply_pipe` — "you have not plumbed this in" —
/// while pointing at a pipe the player had plumbed in perfectly well. They
/// are reported in place of `no_supply_pipe`/`fluid_input` on the draw side
/// and of `fluid_output` on the deposit side, whenever the run that would
/// have served is unnamed (see [`PipeNaming`]).
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

/// The ten ways a machine can be waiting — see [`Stall`]. Serialized in
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
    /// A pipe run *is* attached for this fluid, but nothing in the factory
    /// can say what it carries ([`PipeNaming::Unnameable`]) — so it can
    /// neither supply nor accept, for good. `item` is the fluid the recipe
    /// wanted. Issue #54.
    UnnameablePipe,
    /// The attached run could be carrying any of several fluids
    /// ([`PipeNaming::Ambiguous`]) and the resolver will not guess, so it
    /// carries none. `item` is the fluid the recipe wanted. Issue #54, and
    /// #43 for the case that produces it.
    AmbiguousPipe,
    /// **Switched off by the player** — mothballed (issue #19). `item` is
    /// empty: nothing is being waited for, which is the point. Reported
    /// ahead of every other kind, because a machine that is off has no
    /// meaningful supply state and "waiting for Tomato" about one is a lie
    /// the player then has to debug.
    Mothballed,
}

/// One entry of [`World::machine_load_snapshots`]: the machine's origin
/// tile (its key in every other snapshot), its load and its stall.
///
/// ```json
/// {"tile": [4, 0], "load": {...}, "stall": {"kind": "input", "item": "tomato"},
///  "crewed": true, "mothballed": false}
/// ```
///
/// `crewed` and `mothballed` ride the POLLED entry rather than only the
/// layout, because both can change with no tile dirtied: `crewed` moves on
/// a `Hire`, a `Fire` or a payday's walk-out, none of which touches a chunk
/// (`staff.rs`: "a head is not a tile"), and this is the channel Godot's
/// machine-load readout already reads every frame it redraws.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MachineLoadEntry {
    pub tile: TileCoord,
    pub load: MachineLoadSnapshot,
    pub stall: Option<Stall>,
    /// Whether the hired headcount covers this machine's crew — issue #20.
    /// See [`World::uncrewed_placements`]; `false` is "this is the machine
    /// standing idle for want of a head".
    pub crewed: bool,
    /// Whether the player has switched this machine off — issue #19.
    pub mothballed: bool,
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
///
/// `naming` is the whole *run*'s state ([`PipeNaming`]), repeated on every
/// one of its tiles — a per-run fact on a per-tile record, deliberately, so
/// a renderer walking a chunk's pipes can draw a faulted run without first
/// stitching the chunk back into networks (issue #54). It is the same value
/// [`FluidSnapshot::naming`] carries for the run these tiles belong to.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipeSnapshot {
    pub tile: TileCoord,
    pub direction: Direction,
    pub volume: f64,
    pub capacity: f64,
    pub pressure: f64,
    pub is_source: bool,
    pub naming: PipeNaming,
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
///   "fluid": "vinegar", "declared": "vinegar", "quantity": 30.0,
///   "capacity": 120.0, "fill": 0.25, "auto_fill": false,
///   "pipe_tile": [6, 1]
/// }
/// ```
///
/// - `quantity`/`capacity` are **floating-point**, unlike the hopper's: a
///   fluid volume is continuous (pressure equalization moves partial units
///   around).
/// - **`fluid` is nullable**, and is what this tank holds: its `declared`
///   fluid if it has one, otherwise the fluid of the network it discharges
///   into — which is decided by whichever machine taps that network (see
///   [`super::fluid`]). An *undeclared* tank connected to nothing, or to a
///   run no machine taps, reports `null`: it holds volume, and nothing has
///   yet said what that volume is. A declared one always answers.
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
    /// What the player has **said** this tank holds, or `null` if they have
    /// not said — [`PlayerCommand::SetTankFluid`], issue #43.
    ///
    /// Deliberately a second field rather than a flag on `fluid`: a picker
    /// needs to show its own current selection (which is this, and is
    /// blank until the player chooses), while the panel's "holding:" line
    /// wants the effective answer (which is `fluid`, and may come from the
    /// run). Where both are set they are equal, because the declaration is
    /// what seeded the run's name.
    pub declared: Option<FluidId>,
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
///   "belt_tile": [34, 0], "contents": [["boxed_ketchup", 3]],
///   "places": 32, "used": 3, "at_capacity": false,
///   "shipped_total": 37, "last_paid_tick": 1200, ...
/// }
/// ```
///
/// **`contents` is what stands on THIS dock** (2026-09-26, `super::dock`'s
/// module doc): the number to draw this dock's deck, hover and "full"
/// marker from. Until then a dock was a pass-through into one shared pool
/// and its own contents were always empty between ticks, which is why every
/// dock used to draw the same stack. It is a list, not one item and a
/// count: a dock accepts whatever the line delivers to it.
///
/// The two counters are how presentation, which only polls, tells which
/// dock something happened at: `shipped_total` rises on the dock a box
/// arrived at, and `last_paid_tick` becomes the current tick on every dock a
/// contract payment took from, and on no other.
///
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
    /// How many places the dock has ([`super::dock::DOCK_CAPACITY`]) — the
    /// size of the grid a hover draws, carried so GDScript never knows it.
    pub places: u32,
    /// How many of those places hold a case: the sum of `contents`.
    pub used: u32,
    /// Every place is taken ([`super::dock::Dock::at_capacity`]) — the
    /// dock can take nothing more until a contract collects. Not the same
    /// as `full`, which is a box actually being refused.
    pub at_capacity: bool,
    /// Every unit this dock has ever taken off its belt.
    pub shipped_total: u32,
    /// The last tick a contract payment took stock from this dock, `null`
    /// if none has since the world was built or loaded.
    pub last_paid_tick: Option<u64>,
    /// Refused its logistics crew on the last tick it had something to
    /// ship (the staff pools MVP epic; `Dock::understaffed`). While true
    /// no contract draws on its stock; it keeps taking boxes until every
    /// place is taken, and then the belt behind it backs up.
    ///
    /// This is a PER-TICK fact and only moves on a tick the dock actually
    /// had something to ship, so on a line that ebbs it is false on most
    /// ticks of a dock that is permanently short — read `crewed` for the
    /// standing answer and this one for the live refusal (issue #87).
    pub understaffed: bool,
    /// Whether the establishment could crew this dock if everything it owns
    /// ran at once ([`World::uncrewed_placements`]) — the same stable,
    /// per-placement fact a machine's polled load entry carries (issue #20).
    ///
    /// It rides THIS snapshot, and not only `TileSnapshot::Dock`, because
    /// `dock_snapshots` is polled every frame while the tile snapshot is
    /// read per rebuild: hiring and firing deliberately dirty no chunk
    /// (`sim_core/CLAUDE.md`), so a `crewed` taken off the rebuild channel
    /// would go stale the moment the player fired the clerk. Polled is the
    /// only channel on which a staffing fact stays true.
    pub crewed: bool,
    /// Switched off by the player (issue #19): claims no clerk, ships
    /// nothing, takes nothing off the belt. Distinct from `understaffed`,
    /// which is the crew being refused rather than never asked for.
    pub mothballed: bool,
    /// Whether THIS dock is currently refusing a box off its belt, so the
    /// line behind it is backing up ([`super::dock::Dock::blocked`]) — the
    /// dock-capacity change, 2026-09-16, made per-dock by the second sauce.
    ///
    /// It used to be a property of the one shared delivery pool that every
    /// dock reported identically, which cried jam on a BBQ dock taking
    /// boxes perfectly happily beside a pool capped on ketchup. It is the
    /// building's own state now, and the board's summary stays where it
    /// belongs — `EconomySnapshot::delivery_full`, "some dock is at
    /// capacity", read by the clipboard.
    pub full: bool,
}

/// One occupied bay of a warehouse rack, for presentation (the
/// warehouse-rack sprint, ticket W3 draws a pallet per entry).
///
/// ```json
/// { "bay": 0, "item": "tomato", "units": 50, "capacity": 50 }
/// ```
///
/// `bay` indexes the building's own bays, `0..bays`, and is what decides
/// which pallet in the model this stock stands on. **It is assigned, not
/// stored**: [`World::storage_snapshot`] lays every ingredient's bays out in
/// sorted id order across the racks in tile order, so the same ledger and
/// the same floor plan always give the same answer, but the bay a given
/// ingredient occupies can move when a neighbour's stock does. See
/// [`super::storage`] for why that trade was taken.
///
/// `capacity` is the same `slot_stack` for every bay, carried here so a
/// renderer can size a pallet load without a second lookup — the reason
/// [`HopperSnapshot`] carries its capacity too.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RackBay {
    pub bay: u32,
    pub item: ItemId,
    /// Units standing in **this** bay. The last bay an ingredient occupies
    /// is usually part full; every earlier one holds exactly `capacity`.
    pub units: u32,
    pub capacity: u32,
}

/// One placed warehouse rack, for presentation.
///
/// ```json
/// {
///   "tile": [4, 2], "kind": "warehouse_rack", "direction": "east",
///   "footprint": { "width_tiles": 1, "height_tiles": 2 },
///   "bays": 6, "contents": [{ "bay": 0, "item": "tomato", "units": 50,
///                             "capacity": 50 }]
/// }
/// ```
///
/// There is no `crew`, no `understaffed` and no `mothballed`: a rack does no
/// work, asks for nobody and cannot be switched off, so the three fields
/// every working placement carries would all be constants here. There is no
/// `belt_tile` either — a rack is not wired to anything (sprint requirement
/// R6).
#[derive(Debug, Clone, serde::Serialize)]
pub struct RackSnapshot {
    pub tile: TileCoord,
    pub kind: String,
    pub direction: Direction,
    pub footprint: Footprint,
    /// Bays this building supplies, from `data/storage.ron`.
    pub bays: u32,
    /// Only the **occupied** bays, in bay order. Shorter than `bays`
    /// whenever the warehouse is not full, and empty for a rack in a
    /// factory holding nothing above its free allowance.
    pub contents: Vec<RackBay>,
}

/// The whole storage picture in one object — what a capacity readout needs
/// without walking the racks itself (ARCHITECTURE.md §4/§5).
///
/// Polled, like the fluid and machine-load snapshots, and for their reason:
/// stock moves when a *hopper* is filled, which dirties the hopper's tile
/// and not the warehouse's, and a rack is redrawn from here rather than from
/// a chunk delta.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageSnapshot {
    /// Free capacity per ingredient with no rack placed at all.
    pub base_cap: u32,
    /// Units of one ingredient a single bay holds.
    pub slot_stack: u32,
    /// Bays every placed rack supplies between them.
    pub total_bays: u32,
    /// Bays the current holdings stand in — never more than `total_bays`,
    /// which is the invariant [`super::storage::StorageCatalog::cap_for`]
    /// documents.
    pub used_bays: u32,
    /// Every placed rack, in tile order.
    pub racks: Vec<RackSnapshot>,
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
///   "naming": "carries", "candidates": ["vinegar"],
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
    /// Why `fluid` is what it is — and, when it is `null`, which of the
    /// three very different reasons for that applies. See [`PipeNaming`];
    /// `candidates` below is the list an `ambiguous` run is ambiguous
    /// between.
    pub naming: PipeNaming,
    /// Every id the naming pass could not rule out, in id order: one entry
    /// when the run is named, none when nothing can name it, several when
    /// it is ambiguous.
    pub candidates: Vec<FluidId>,
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
    ///   "stall": null, "mothballed": false,
    ///   "crew": [{"category": "manufacturing", "count": 2}],
    ///   "crewed": true}`
    ///
    /// `machine_kind` is spelled out rather than reusing `kind`, which this
    /// enum already spends on its own variant tag. `load` and `stall` are
    /// the same objects [`MachineSnapshot`] carries, so hover can say how
    /// loaded a machine is and why it waits without a second call. `crew`
    /// is the kind's `machines.ron` `staff` requirement (the staff pools
    /// MVP epic), for hover's "crew: 2 manufacturing"; `crewed` is whether
    /// it actually got those people this tick (issue #20), so hover can say
    /// "short of manufacturing staff" about *this* machine rather than
    /// leaving the player to diff the staff screen against the floor.
    Machine {
        machine_kind: String,
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        recipe: RecipeId,
        process: Option<(u64, u64)>,
        load: MachineLoadSnapshot,
        mothballed: bool,
        stall: Option<Stall>,
        crew: Vec<CrewEntry>,
        crewed: bool,
    },
    /// `{"kind": "pipe", "direction": "east", "fluid": "vinegar",
    ///   "naming": "carries", "candidates": ["vinegar"],
    ///   "volume": 7.5, "capacity": 10.0, "pressure": 0.75,
    ///   "is_source": false, "is_inlet": true,
    ///   "links": {"connections": [{"to": "west", "kind": "pipe"}]}}` —
    /// enough for a hover tooltip without a second call. `is_inlet` is
    /// whether some machine taps this exact tile; `links` is the
    /// placement's [`PipeLinks`].
    Pipe {
        direction: Direction,
        fluid: Option<FluidId>,
        /// The run's naming state and what it narrowed to — the same two
        /// fields [`FluidSnapshot`] carries, so a hover panel can say *why*
        /// a run is unnamed from the one call it already makes.
        naming: PipeNaming,
        candidates: Vec<FluidId>,
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
    ///   "origin_tile": [35, 0], "contents": [["boxed_ketchup", 3]],
    ///   "places": 32, "used": 3, "at_capacity": false,
    ///   "shipped_total": 37, "last_paid_tick": null, ...}` — what stands on
    /// this dock and how many of its places that fills; the same fields
    /// [`DockSnapshot`] carries, and documented there.
    Dock {
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        contents: Vec<(ItemId, u32)>,
        places: u32,
        used: u32,
        at_capacity: bool,
        shipped_total: u32,
        last_paid_tick: Option<u64>,
        /// The kind's `staff` requirement and whether the dock was refused
        /// it on the last tick it held something (the staff pools MVP
        /// epic) — hover's "crew: 1 logistics" and "held · short of
        /// logistics staff". `crewed` is the same per-placement fact every
        /// other placement carries (issue #20); for a dock holding nothing
        /// it is the only one of the two that is meaningful, since
        /// `understaffed` only moves on a tick the dock had something to
        /// ship.
        crew: Vec<CrewEntry>,
        crewed: bool,
        understaffed: bool,
        /// Switched off by the player (issue #19) — the same field a
        /// machine tile carries, because the dock is the other placement
        /// whose kind asks for a crew and so the other one that can be
        /// switched off.
        mothballed: bool,
    },
    /// `{"kind": "warehouse_rack", "direction": "east", "footprint": {...},
    ///   "origin_tile": [4, 2], "bays": 6,
    ///   "contents": [{"bay": 0, "item": "tomato", "units": 50,
    ///                 "capacity": 50}]}` — enough for a hover tooltip
    /// ("Warehouse rack — 2/6 bays"). Only *occupied* bays appear in
    /// `contents`, and the bay a given ingredient stands in is assigned by
    /// [`World::storage_snapshot`] rather than stored (see
    /// [`super::storage`]). There is no `fill` and no crew: a rack does no
    /// work and asks for nobody.
    WarehouseRack {
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        bays: u32,
        contents: Vec<RackBay>,
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
            // A rack is never on either end of a link, so this arm is only
            // reachable if one day something asks a rack's tile what it is
            // wired to. `Dock` is the nearest honest answer — a terminus —
            // and inventing a `LinkKind::Rack` nothing produces would put a
            // dead variant on the wire for presentation to handle.
            BodyKind::WarehouseRack => LinkKind::Dock,
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
    /// Whether the hired headcount covers this placement's crew (issue
    /// #20) — see [`World::uncrewed_placements`] for the rule and why it is
    /// derived here rather than stored. `true` for everything that asks for
    /// nobody, and for a mothballed machine.
    ///
    /// Before `links`, which stays last: it is the only optional key here,
    /// and `tests/links.rs` pins that shape byte for byte.
    pub crewed: bool,
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
    /// A warehouse stock rack (see [`super::storage`]). The only body here
    /// that the *tick* never touches: a rack supplies storage bays, and
    /// storage is a property of the ledger rather than of a tile. It is a
    /// `BodyKind` at all because it is placed, removed, rotated and drawn
    /// like everything else, and because its absence from every hand-off in
    /// `recompute_topology` is the thing that has to stay deliberate —
    /// sprint requirement R6, "belts and pipes do not interact with racks".
    WarehouseRack,
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
    /// Every placed warehouse rack, in tile order. Read by
    /// [`World::storage_bays`] and by the storage snapshot, and by nothing
    /// in `tick` at all — see [`BodyKind::WarehouseRack`].
    racks: Vec<Entity>,
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
    /// The same, for fluids: a fluid some recipe draws and no recipe
    /// produces — vinegar today. **This is the root of the whole
    /// pipe-naming propagation** (`resolve_network_fluids`): a raw fluid is
    /// the only thing that can enter the graph from outside it, so a run a
    /// tank discharges into can carry one of these and nothing else, and
    /// every other run is named from what a machine at one of its ends
    /// could make or take. A `BTreeSet` because that propagation walks it
    /// (ARCHITECTURE.md §3.9's iteration-order rule).
    raw_fluids: BTreeSet<FluidId>,
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
    /// The tiles whose conveyor contents changed since `dirty` was last
    /// cleared, and the machines/hoppers/tanks/docks whose live state did —
    /// what [`World::chunk_delta`] filters a dirty chunk's snapshot down to.
    /// Membership only; iteration order comes from `topology`, never from
    /// these sets, so a `HashSet` does not threaten determinism (§3.9).
    dirty_tiles: HashSet<TileCoord>,
    dirty_entities: HashSet<Entity>,
    /// Chunks whose STRUCTURE changed since `dirty` was last cleared — a
    /// `Place`/`Remove`/`Rotate` or a load — for which a delta is the whole
    /// snapshot: a tile that was just placed has no "last state" for a
    /// consumer to have kept.
    full_chunks: Vec<ChunkId>,
    /// Bumped by every `tick` and every `issue_command`, accepted or not:
    /// the one number a presentation layer compares to know whether ANY
    /// polled snapshot could have changed since it last read one. See
    /// [`World::revision`].
    revision: u64,
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
    /// How much raw stock the player may hold and what raises the ceiling
    /// (the warehouse-rack sprint, `storage.rs`). A plain field beside
    /// `ingredients` because it is that ledger's rules: the numbers live in
    /// `data/storage.ron`, the bays are counted off `topology.racks`, and
    /// nothing about capacity is stored per entity.
    storage: StorageCatalog,
    /// The staff pools (the staff pools MVP epic, GAMEPLAY.md §7): a plain
    /// field like `economy`, for the same reason — four aggregate counters
    /// are not a reason for ECS machinery.
    staff: StaffPools,
    /// THE COOKBOOK's prose pages (`data/guide.ron`). Held like `storage`
    /// rather than loaded per read: [`World::cookbook`] is polled while the
    /// book is open, and re-parsing a content file on a poll is a parse per
    /// poll for a file that cannot change while the game runs. No world
    /// state touches it — it is content, and it is here because this is
    /// where the registries live.
    guide: GuideRegistry,
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
/// Stated in prose in `docs/DECISIONS_LOG.md` and `docs/DECISIONS.md`;
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
        Self::with_content_and_money(
            recipes,
            machines,
            contracts,
            Money::from_num(STARTING_MONEY),
        )
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
        money: Money,
    ) -> Self {
        let ordered = recipes.ordered();
        assert!(
            !ordered.is_empty(),
            "recipes.ron must define at least one recipe"
        );
        // Raw = consumed somewhere, produced nowhere. Computed before the
        // content checks because `validate_chain` asks whether each part of
        // the file has a *priced* way in, and "raw" is what decides which
        // ids have to be priced at all.
        let raws = raw_ingredient_ids(&recipes);
        let shop = Shop::bundled();
        let ingredients = Ingredients::bundled();
        // Every recipe's machine kind must exist in machines.ron — a
        // missing kind has no footprint, so a machine of that kind could
        // never be placed and its recipe would be silently unreachable.
        // Same construction-time-panic rule as `validate_chain`, and
        // checked *first* because no footprint is the more basic fault:
        // otherwise a kind missing from both files fails as "nothing can
        // start this line" when the plainer truth is "this machine does
        // not exist".
        if let Err(e) = machines.validate_covers(&recipes) {
            panic!("machine content error: {e}");
        }
        // Still enforced, though nothing lays out a chain any more: every
        // part of the file must be content the player can reach, which is
        // what `reference_chain` builds against and what makes "the first
        // recipe's first input is the raw ingredient" true.
        validate_chain(ordered, &raws, &shop.catalog, &ingredients.catalog);
        validate_fluid_scope(ordered);

        // The two halves of raw are split by where they arrive: a raw
        // *solid* is what a hopper holds, a raw *fluid* is what a
        // player-filled tank discharges. Both are bought from
        // `data/ingredients.ron`.
        let solid_inputs: Vec<&str> = ordered
            .iter()
            .flat_map(|r| r.inputs.iter().map(|(item, _)| item.as_str()))
            .collect();
        let fluid_inputs: Vec<&str> = ordered
            .iter()
            .flat_map(|r| r.fluid_inputs.iter().map(|(fluid, _)| fluid.as_str()))
            .collect();
        let raw_items: Vec<ItemId> = raws
            .iter()
            .filter(|id| solid_inputs.contains(&id.as_str()))
            .cloned()
            .collect();
        let raw_fluids: BTreeSet<FluidId> = raws
            .iter()
            .filter(|id| fluid_inputs.contains(&id.as_str()))
            .cloned()
            .collect();

        Self {
            ecs: hecs::World::new(),
            recipes,
            machines,
            raw_items,
            raw_fluids,
            tick_count: 0,
            belt_speed: Fx::from_num(BELT_SPEED_TILES_PER_TICK),
            room_ahead: Vec::new(),
            dirty: Vec::new(),
            dirty_tiles: HashSet::new(),
            dirty_entities: HashSet::new(),
            full_chunks: Vec::new(),
            revision: 0,
            economy: Economy::with_money(&contracts, money),
            shop,
            ingredients,
            storage: StorageCatalog::bundled(),
            staff: StaffPools::bundled(),
            guide: GuideRegistry::bundled(),
            topology: Topology::default(),
            production: ProductionLedger::new(),
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }

    /// Current money balance (ARCHITECTURE.md §3.9: fixed-point, never a
    /// float, since it's shared/persisted simulation state).
    pub fn money(&self) -> Money {
        self.economy.money
    }

    /// The live economy — money and every active
    /// contract's own cadence state. Read-only; contracts advance through
    /// [`World::tick`], never by a caller poking at them.
    pub fn economy(&self) -> &Economy {
        &self.economy
    }

    /// Presentation-facing money + contract-status snapshot (ARCHITECTURE.md
    /// §4/§5) — always current, unlike `chunk_snapshot` there's no
    /// dirty-tracking to consult first.
    /// Settles nothing: `progress` and `satisfied` are read against the
    /// stock [`World::tick`]'s step 7 would lend the economy if a window
    /// ended now, so the clipboard and the payout can never disagree.
    pub fn economy_snapshot(&self) -> EconomySnapshot {
        let mut stock = DeliveryStock::default();
        for &dock in &self.topology.docks {
            let body = self
                .ecs
                .get::<&Dock>(dock)
                .expect("a dock placement carries a Dock");
            stock.count_places(body.places(), body.at_capacity());
            if !body.mothballed() && !body.understaffed() {
                let mut held = Inventory::new(body.places());
                for (item, qty) in body.contents() {
                    held.add(item, *qty);
                }
                stock.push(held);
            }
        }
        self.economy.snapshot(&stock)
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
        self.shop.snapshot(self.economy.money, |kind| {
            crew_entries(self.crew_of_kind(kind))
        })
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
        self.machines.get(kind).map_or(&[], |k| k.staff.as_slice())
    }

    /// The raw-ingredient market plus the stock the player owns and their
    /// money, in one object — see [`IngredientsSnapshot`].
    ///
    /// **An ingredient behind a progression gate that has not opened is
    /// absent from `entries`** (`data/unlocks.ron`, [`super::unlocks`]) but
    /// **present and flagged in `hopper_items`/`tank_fluids`** — see
    /// [`super::ingredients::ContentsChoice`] for why the market hides what
    /// the contents picker shows.
    pub fn ingredients_snapshot(&self) -> IngredientsSnapshot {
        let deliveries = self.economy.lifetime_deliveries;
        let bays = self.storage_bays();
        self.ingredients.snapshot(
            self.economy.money,
            &self.raw_items,
            self.raw_fluids.iter().cloned().collect(),
            deliveries,
            |id| self.storage.cap_for(id, &self.ingredients.owned, bays),
            |id| {
                self.economy
                    .unlocks()
                    .gate_for_ingredient(id)
                    .filter(|gate| !gate.is_open(deliveries))
                    .map(|gate| (gate.name.clone(), gate.after_deliveries))
            },
        )
    }

    /// The stock-capacity rules this world was built with
    /// (`data/storage.ron`). Read-only: capacity is content, and the only
    /// thing that changes at runtime is how many racks stand on the floor.
    pub fn storage(&self) -> &StorageCatalog {
        &self.storage
    }

    /// How many storage bays the placed racks supply between them —
    /// `total_bays` everywhere else in this module.
    ///
    /// Summed off `topology.racks` rather than kept as a counter, so it
    /// cannot drift from the floor: `recompute_topology` runs on every
    /// `Place`, `Remove` and `Rotate`, which is exactly when this answer can
    /// change.
    pub fn storage_bays(&self) -> u32 {
        self.topology
            .racks
            .iter()
            .map(|&entity| self.storage.bays_of_kind(&self.placement_of(entity).kind))
            .fold(0, u32::saturating_add)
    }

    /// The most of `id` the player may currently hold — the number a market
    /// panel shows beside what they own, and the number `BuyIngredient`
    /// clamps against. See [`StorageCatalog::cap_for`] for the rule and its
    /// invariant.
    pub fn stock_cap_for(&self, id: &str) -> u32 {
        self.storage
            .cap_for(id, &self.ingredients.owned, self.storage_bays())
    }

    /// How many more units of `id` will fit right now.
    pub fn stock_room_for(&self, id: &str) -> u32 {
        self.storage
            .room_for(id, &self.ingredients.owned, self.storage_bays())
    }

    /// The flat run of occupied bays, `(ingredient, units)` in bay order —
    /// [`StorageCatalog::bay_contents`] against this world's ledger. Sliced
    /// across the racks by [`World::storage_snapshot`].
    fn bay_runs(&self) -> Vec<(ItemId, u32)> {
        self.storage.bay_contents(&self.ingredients.owned)
    }

    /// One rack's snapshot, given the flat bay run [`World::bay_runs`]
    /// built.
    ///
    /// The slice this rack gets is decided by **how many bays the racks
    /// before it in tile order supply**, which is why the run is passed in
    /// rather than recomputed: a rack cannot know its own offset.
    fn rack_snapshot(&self, entity: Entity, runs: &[(ItemId, u32)]) -> RackSnapshot {
        let placement = self.placement_of(entity);
        let bays = self.storage.bays_of_kind(&placement.kind);
        let mut offset = 0u32;
        for &other in &self.topology.racks {
            if other == entity {
                break;
            }
            offset =
                offset.saturating_add(self.storage.bays_of_kind(&self.placement_of(other).kind));
        }
        let capacity = self.storage.slot_stack();
        let contents = (0..bays)
            .filter_map(|bay| {
                let (item, units) = runs.get((offset + bay) as usize)?;
                Some(RackBay {
                    bay,
                    item: item.clone(),
                    units: *units,
                    capacity,
                })
            })
            .collect();
        RackSnapshot {
            tile: self.position_of(entity),
            kind: placement.kind.clone(),
            direction: placement.direction,
            footprint: placement.footprint,
            bays,
            contents,
        }
    }

    /// Everything a capacity readout and a warehouse render need: the two
    /// content numbers, the bay totals, and every placed rack with the stock
    /// standing in it (ARCHITECTURE.md §4/§5).
    ///
    /// **Ingredients fill the racks in sorted id order, racks in tile
    /// order** — the one place the bay assignment is decided, so it is
    /// deterministic (§3.9) even though nothing stores it.
    pub fn storage_snapshot(&self) -> StorageSnapshot {
        let runs = self.bay_runs();
        StorageSnapshot {
            base_cap: self.storage.base_cap(),
            slot_stack: self.storage.slot_stack(),
            total_bays: self.storage_bays(),
            used_bays: self.storage.bays_used(&self.ingredients.owned),
            racks: self
                .topology
                .racks
                .iter()
                .map(|&entity| self.rack_snapshot(entity, &runs))
                .collect(),
        }
    }

    /// Every content id that names itself, as `id -> display name`.
    ///
    /// The presentation layer title-cases an id when it wants a label
    /// (`main.gd`'s `_kind_display_name`), which is right for almost
    /// everything and wrong for an id that is an acronym: `bbq` came out
    /// as "Bbq". Rather than teach the formatter a list of exceptions —
    /// which would put content in GDScript — a recipe or an ingredient may
    /// carry a `display_name` in `data/*.ron`, and this is the one map of
    /// them. An id that is absent is absent on purpose: the title-case
    /// fallback says it correctly.
    ///
    /// Recipes and ingredients share one id space (a recipe's id is its
    /// product's), so one map serves both. A recipe wins a collision,
    /// which cannot happen today: no shipped raw declares a name.
    ///
    /// **Machine kinds are in here too**, from `data/machines.ron`, and
    /// they are the other half of the reason this map exists: a kind whose
    /// id is a key rather than a label (`warehouse_rack`, which the game
    /// calls "Stock Shelving") names itself in content exactly the way a
    /// sauce does. Kind ids and item ids are separate spaces that have
    /// never collided; if one ever does, the machine's name wins, since a
    /// kind label is only ever asked for by `_kind_display_name`.
    pub fn display_names(&self) -> BTreeMap<String, String> {
        self.ingredients
            .catalog
            .display_names()
            .chain(self.recipes.display_names())
            .chain(self.machines.display_names())
            .map(|(id, name)| (id.to_string(), name.to_string()))
            .collect()
    }

    /// THE COOKBOOK: `data/*.ron` read back as a sauce tree and a page per
    /// sauce, machine and ingredient. See [`super::cookbook`] for what is
    /// derived and why none of it is authored.
    ///
    /// **Nothing about this world is in the answer except the delivery
    /// count.** The book is the same for every save at the same point in
    /// progression, which is what lets the panel read it on open and then
    /// only when the padlocks could have moved, rather than per frame.
    pub fn cookbook(&self) -> super::cookbook::CookbookSnapshot {
        super::cookbook::build(&super::cookbook::CookbookInput {
            recipes: &self.recipes,
            machines: &self.machines,
            ingredients: &self.ingredients.catalog,
            contracts: self.economy.board(),
            shop: &self.shop.catalog,
            staff: self.staff.catalog(),
            unlocks: self.economy.unlocks(),
            guide: &self.guide,
            deliveries: self.economy.lifetime_deliveries,
        })
    }

    /// Every id in `data/*.ron` that names its own voxel asset, as
    /// `id -> asset`. [`Self::display_names`]'s twin, read once at load for
    /// its reason, and absence means the same thing: the presentation layer
    /// falls back to its own table (`main.gd`'s `ITEM_MODELS`).
    pub fn icon_names(&self) -> BTreeMap<String, String> {
        self.ingredients
            .catalog
            .icons()
            .chain(self.recipes.icons())
            .chain(self.machines.icons())
            .map(|(id, asset)| (id.to_string(), asset.to_string()))
            .collect()
    }

    /// Lifetime successful contract deliveries — the counter every
    /// progression gate is keyed off. Read-only; it is written by
    /// [`Economy::tick`] and by a load, never by a caller.
    pub fn lifetime_deliveries(&self) -> u32 {
        self.economy.lifetime_deliveries
    }

    /// Whether the market may sell `id` right now. The same answer
    /// `ingredients_snapshot` filters on and `BuyIngredient` refuses
    /// against — one rule, asked in one place.
    pub fn ingredient_unlocked(&self, id: &str) -> bool {
        self.economy.ingredient_unlocked(id)
    }

    /// Whether the board may offer `id` right now. [`Economy::try_sign`]
    /// refuses anything this says no to.
    pub fn contract_unlocked(&self, id: &str) -> bool {
        self.economy.contract_unlocked(id)
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

    /// The fluid half of [`Self::raw_items`]: every fluid consumed by some
    /// recipe and produced by none, so the only ones that can enter the
    /// graph through a tank. The set [`PlayerCommand::SetTankFluid`]
    /// validates a declaration against, and the list a tank's fluid picker
    /// offers (`IngredientsSnapshot::tank_fluids`).
    pub fn raw_fluids(&self) -> &BTreeSet<FluidId> {
        &self.raw_fluids
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
            // A rack needs nothing of the content files beyond a price
            // and a footprint: it runs no recipe and holds no declared
            // ingredient, so there is nothing here that could be missing.
            WAREHOUSE_RACK_KIND => true,
            HOPPER_KIND => !self.raw_items.is_empty(),
            other => self.recipes.for_machine(other).next().is_some(),
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
            // Raw, and DELIBERATELY not gated (issue #90). `SetHopperItem`
            // on a placed hopper is refused for a locked ingredient, because
            // that command is the picker the player clicks; the `item` a
            // Place carries is not a choice made against a gate at all — the
            // ghost seeds it (`_selected_hopper_item` skips locked ids), and
            // a layout fixture names it to build a factory to photograph.
            // Refusing it would buy nothing a player can reach and cost
            // every BBQ layout fixture ten deliveries of real play before it
            // could place a bin, which `test_layouts.gd` has no honest way
            // to pay. The gate still bites the bin it seeds: the raw behind
            // it cannot be bought, so neither a manual fill nor auto-fill
            // can put a single unit in it.
            Some(item) => self.raw_items.iter().find(|raw| *raw == item).cloned(),
            // An omitted item is unchanged: it falls back only where there
            // is exactly ONE raw item in the world at all.
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
                    // Laid on THIS tick, not on tick 0: a belt dropped into
                    // a floor that has been running for an hour is in phase
                    // with it from the frame it is first drawn in.
                    self.tick_count,
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
                    self.tick_count,
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
            // A rack's body is a marker and nothing else: the bays it
            // supplies are a property of its kind (`data/storage.ron`) and
            // the stock in them stays in the one ingredient ledger. See
            // [`WarehouseRack`].
            WAREHOUSE_RACK_KIND => {
                self.ecs
                    .spawn((position, placement, BodyKind::WarehouseRack, WarehouseRack))
            }
            machine_kind => {
                // A freshly placed machine starts on its kind's first recipe
                // in file order; from its first idle tick onwards it re-picks
                // whichever one its inputs satisfy. See
                // `RecipeRegistry::for_machine`.
                let recipe = self
                    .recipes
                    .for_machine(machine_kind)
                    .next()
                    .expect("can_construct checked this kind has a recipe")
                    .id
                    .clone();
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
        // **The one removal in this simulation that can be refused.**
        // Everything else picks up cleanly because its contents go with it
        // — a hopper keeps its bin, a belt's items are destroyed with the
        // tile. A rack's contents are not its own: they are rows in the
        // shared ingredient ledger, and taking the rack away would leave
        // stock standing in bays that no longer exist. Rather than delete
        // the overflow (the player paid for it) or let the ledger sit over
        // its own ceiling (which would make
        // `StorageCatalog::cap_for`'s invariant a lie), the building simply
        // will not come up until enough has been used. Presentation says so
        // from `storage_snapshot`; there is no refusal channel and this does
        // not add one (see `BuyIngredient`).
        if self.storage.is_rack(&placement.kind) {
            let bays = self.storage.bays_of_kind(&placement.kind);
            let remaining = self.storage_bays().saturating_sub(bays);
            if self.storage.bays_used(&self.ingredients.owned) > remaining {
                return false;
            }
        }
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

    /// Re-derives the whole topology once `save.rs` has finished restoring
    /// live state — the load-time counterpart of the `Place` commands that
    /// drive it during play.
    ///
    /// Loading rebuilds structure through real `Purchase`/`Place`, and each
    /// of those recomputes the topology on its own. A tank's **declared**
    /// fluid, though, is restored afterwards with the bodies, and a
    /// declaration is an *input* to pipe naming
    /// ([`resolve_network_fluids`]) rather than a record of its output. So
    /// without this pass a loaded world is named as though every tank were
    /// undeclared, and the very layout issue #43 exists for — two tank-fed
    /// raw runs into one machine — comes back ambiguous and stuck however
    /// it was saved. Nothing else is disturbed: the pass carries pipe
    /// volumes across by tile, which is what lets it run on a live world at
    /// all.
    pub(super) fn recompute_topology_after_restore(&mut self) {
        self.recompute_topology();
    }

    /// Sums the ESTABLISHMENT — every head the placed machinery needs to be
    /// fully crewed, whether or not any of it is running — and hands it to
    /// [`StaffPools::set_required`], together with the two things that
    /// figure is useless without: what the MOTHBALLED machinery would need,
    /// and which kinds are asking.
    ///
    /// Called from [`Self::recompute_topology`], which is reached only from
    /// `Place`, `Remove` and `Rotate`, so the staffing requirement moves
    /// when the FACTORY changes and at no other time. **Also called by the
    /// save loader after it restores the pools**: `StaffPools::restore`
    /// builds fresh pools, which would otherwise drop the establishment the
    /// replayed `Place` commands had just computed and load every world
    /// needing nobody. And by `try_set_mothballed`, which is a change to
    /// the factory that moves no tile.
    ///
    /// The per-tick `demand` beside it cannot do this job: a machine asks
    /// for its crew only when its plan is to work (`machine.rs`, "The crew
    /// is asked for only when the plan is to work"), and the dock only when
    /// its buffer has something in it, so `demand` falls whenever a belt
    /// runs dry. The player saw a hiring figure that had already changed by
    /// the time they clicked HIRE. Establishment against headcount is the
    /// stable question, and `staff.ron`'s starting roster was always the
    /// shipped chain's establishment — `tests/staff.rs` asserts that
    /// equality (docs/DECISIONS.md, the 2026-09-16 record).
    ///
    /// One entry per placement, so a 2x2 cooking vat is one crew and not
    /// four: the ECS holds one entity per placement, however many tiles it
    /// covers.
    ///
    /// **A MOTHBALLED machine is not in the establishment** (issue #19) —
    /// it is counted into `mothballed` instead, which is how many heads may
    /// be STOOD DOWN with it. The requirement is what is placed *and
    /// switched on*, so a player who cannot make payroll can switch a
    /// machine off, watch `required` fall below `headcount`, and see the
    /// wage bill fall with it without losing anybody
    /// (`StaffPools::sync_payroll`). The kinds behind `required` ride along
    /// as `sources` so the HUD can name them without a second walk of its
    /// own — the walk is here, once, in the sim's own tile order.
    pub(super) fn recompute_establishment(&mut self) {
        let mut required: BTreeMap<String, u32> = BTreeMap::new();
        let mut mothballed: BTreeMap<String, u32> = BTreeMap::new();
        // Kinds in the order the world first met them, per category, so
        // the HUD's `asked for by:` line is stable across a poll.
        let mut sources: BTreeMap<String, Vec<CrewSource>> = BTreeMap::new();
        for &entity in &self.topology.all {
            if self.ecs.get::<&Position>(entity).is_err() {
                continue;
            }
            let kind = self.placement_of(entity).kind;
            let Some(machine) = self.machines.get(&kind) else {
                continue;
            };
            let off = self.is_mothballed(entity);
            for (category, count) in &machine.staff {
                if off {
                    *mothballed.entry(category.clone()).or_insert(0) += *count;
                    continue;
                }
                *required.entry(category.clone()).or_insert(0) += *count;
                let lines = sources.entry(category.clone()).or_default();
                match lines.iter_mut().find(|line| line.kind == kind) {
                    Some(line) => line.count = line.count.saturating_add(*count),
                    None => lines.push(CrewSource {
                        kind: kind.clone(),
                        count: *count,
                    }),
                }
            }
        }
        let mut establishment: Vec<EstablishmentEntry> = Vec::new();
        for category in required.keys().chain(mothballed.keys()) {
            if establishment.iter().any(|e| &e.category == category) {
                continue;
            }
            establishment.push(EstablishmentEntry {
                category: category.clone(),
                required: required.get(category).copied().unwrap_or(0),
                mothballed: mothballed.get(category).copied().unwrap_or(0),
                sources: sources.get(category).cloned().unwrap_or_default(),
            });
        }
        self.staff.set_required(&establishment);
    }

    /// Whether `entity` is a placement the player has switched off.
    /// `false` for everything that cannot be mothballed — which is to say
    /// everything that never asks for a crew (see
    /// [`PlayerCommand::SetMothballed`]).
    fn is_mothballed(&self, entity: Entity) -> bool {
        if let Ok(machine) = self.ecs.get::<&Machine>(entity) {
            return machine.mothballed;
        }
        if let Ok(dock) = self.ecs.get::<&Dock>(entity) {
            return dock.mothballed();
        }
        false
    }

    /// **Which placements the hired headcount CANNOT crew** — issue #20's
    /// per-placement fact, the one the HUD could not say. The game already
    /// said *that* the factory was short and *how many* heads to hire; what
    /// it never said was WHICH machine was standing idle for want of one,
    /// so a player on a large line had to hunt tile by tile. Every entity
    /// in this set is such a machine.
    ///
    /// Derived, never stored: it costs no save-schema change, and it cannot
    /// go stale the way a cached flag would, because the two things it
    /// depends on — what is placed and how many heads are hired — move for
    /// completely different reasons (a `Place`, and a `Hire`/`Fire`/payday
    /// that dirties no tile at all). Recomputing it at snapshot time is
    /// what keeps it honest; a walk of a few hundred placements is nothing
    /// beside the chunk read it rides along with.
    ///
    /// **The ESTABLISHMENT, handed out in the order the tick claims in** —
    /// `topology.machines`, then `topology.docks`, whole crew or nothing
    /// (`staff.rs`: "per tick, in stable order, whole crew or nothing"), so
    /// the machine this names is the one the first-come rule really leaves
    /// short.
    ///
    /// It is deliberately NOT a replay of the last tick. The tick claims
    /// only for a machine that would otherwise have worked, so a per-tick
    /// reading flickers as the line ebbs — the vat reads crewed on the
    /// ticks the crusher happened to be waiting for a tomato — and that is
    /// the exact flapping the 2026-09-16 establishment change existed to
    /// end. This asks the stable question instead: *if everything that is
    /// placed wanted to run at once, who would go without?* A machine
    /// placed and idle still holds its place in the queue here, because it
    /// still holds its place in `required`. What happened on one particular
    /// tick is already on the wire as the `staff` stall
    /// (`Machine::refused_by`), and the two are meant to be read together.
    ///
    /// It allocates against the heads that would actually turn up —
    /// `headcount - unpaid` — and **not** against `headcount`. A head in
    /// arrears is on the books but is not at their post
    /// (`StaffPools::reset`), so a factory that missed a payday has
    /// machines standing idle for want of a crew, which is precisely what
    /// this set exists to name; budgeting against `headcount` reported
    /// every one of them as crewed through the whole of an arrears (issue
    /// #20's headline case, found in review). That is still an
    /// establishment-stable figure and not a per-tick one: arrears move
    /// only at a payday, hire or fire, never with production flow, so
    /// nothing here flickers.
    ///
    /// `StaffPools::short` keeps the other reading deliberately — it
    /// answers "how many must I HIRE", and hiring a second head to cover
    /// an unpaid one is the wrong advice, paying them is. The two numbers
    /// differ on purpose, and the HUD says which is which in words
    /// (`_uncrewed_reason_text`).
    ///
    /// A placement whose kind asks for nobody — a belt, a pipe, a hopper, a
    /// tank — is crewed vacuously, as is a MOTHBALLED machine: it needs no
    /// crew, so it is not waiting for one. Reported as the set of the
    /// UNCREWED, which is the short list on any factory worth looking at
    /// and makes "everything else is fine" the default.
    fn uncrewed_placements(&self) -> BTreeSet<Entity> {
        let mut budget: BTreeMap<&str, u32> = self
            .staff
            .catalog()
            .categories()
            .iter()
            .zip(self.staff.pools())
            .map(|(category, pool)| {
                (
                    category.id.as_str(),
                    pool.headcount.saturating_sub(pool.unpaid),
                )
            })
            .collect();
        let mut uncrewed = BTreeSet::new();
        let ordered = self.topology.machines.iter().chain(&self.topology.docks);
        for &entity in ordered {
            if self.is_mothballed(entity) {
                continue;
            }
            let kind = self.placement_of(entity).kind;
            let crew: &[(String, u32)] =
                self.machines.get(&kind).map_or(&[], |k| k.staff.as_slice());
            let affordable = crew.iter().all(|(category, count)| {
                budget.get(category.as_str()).copied().unwrap_or(0) >= *count
            });
            if affordable {
                for (category, count) in crew {
                    if let Some(left) = budget.get_mut(category.as_str()) {
                        *left -= *count;
                    }
                }
            } else {
                uncrewed.insert(entity);
            }
        }
        uncrewed
    }

    /// Whether one placement has the heads it needs on the books — see
    /// [`Self::uncrewed_placements`], which this is the single-entity
    /// reading of. Used by the hover query, which asks about one tile.
    fn is_crewed(&self, entity: Entity) -> bool {
        !self.uncrewed_placements().contains(&entity)
    }

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
                BodyKind::WarehouseRack => topology.racks.push(entity),
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
                // A rack is not a sink, deliberately (R6): an item on a
                // belt pointed at one simply stops, the way it does
                // against a tank.
                BodyKind::Pipe
                | BodyKind::Hopper
                | BodyKind::FluidTank
                | BodyKind::WarehouseRack => None,
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
        // This pass is pure **geometry**: it records the one network a
        // machine could deposit into and every network it could draw from,
        // and names nothing. Which of a kind's recipes runs is decided per
        // tick from what the machine actually holds, so the pipe ids cannot
        // be read off a machine's assignment any more — they are resolved
        // afterwards, for the whole graph at once, by
        // [`resolve_network_fluids`]. See this module's doc, "What a pipe
        // run carries".
        let mut plumbing: Vec<Plumbing> = Vec::with_capacity(topology.machines.len());
        for &machine in &topology.machines {
            let placement = self.placement_of(machine);
            let origin = self.position_of(machine);
            let kind = placement.kind.clone();
            // Whether this kind plumbs at all is a property of the *kind*,
            // not of whichever recipe it happens to be running: a machine
            // that can draw a fluid under any of its recipes gets its taps,
            // so switching recipe never has to rebuild the topology.
            let (mut draws, mut deposits) = (false, false);
            for recipe in self.recipes.for_machine(&kind) {
                draws |= !recipe.fluid_inputs.is_empty();
                deposits |= !recipe.fluid_outputs.is_empty();
            }

            let mut output_network: Option<usize> = None;
            if deposits {
                let tap = placement
                    .footprint
                    .outlet_neighbours(origin, placement.direction)
                    .into_iter()
                    .find_map(|tile| topology.pipe_index.get(&tile).map(|&index| (tile, index)));
                if let Some((tile, (network, segment))) = tap {
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

            // Every network touching the footprint, not just the first:
            // a recipe may draw several fluids, and which tap supplies
            // which is matched by fluid id in `Machine::tick`. One entry
            // per *network*, so the taps a machine is handed are pairwise
            // distinct and can be borrowed mutably all at once.
            let mut input_networks: Vec<usize> = Vec::new();
            if draws {
                let touching = placement
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
                    .filter(|&(_, _, (network, _))| Some(network) != output_network);
                for (dir, tile, (network, segment)) in touching {
                    if input_networks.contains(&network) {
                        continue;
                    }
                    input_networks.push(network);
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

            plumbing.push(Plumbing {
                kind,
                output: output_network,
                inputs: input_networks,
            });
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

        // --- what each run carries. Every tank discharge is marked by now,
        // which is what gives the propagation somewhere to start — and the
        // declared ones are gathered here, one set per run, because a run
        // two tanks discharge into is named by both of them at once.
        let mut declared: Vec<BTreeSet<FluidId>> = vec![BTreeSet::new(); topology.networks.len()];
        for &(tank, network, _) in &topology.tank_discharges {
            if let Some(fluid) = self
                .ecs
                .get::<&FluidTank>(tank)
                .ok()
                .and_then(|body| body.declared().cloned())
            {
                declared[network].insert(fluid);
            }
        }
        resolve_network_fluids(
            &mut topology.networks,
            &plumbing,
            &self.recipes,
            &self.raw_fluids,
            &declared,
        );

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
        // Last, and only here: the establishment walks `topology.all`, so
        // it has to see the topology this pass just built rather than the
        // one it replaced.
        self.recompute_establishment();
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
        self.dirty_tiles.insert(tile);
    }

    /// A STRUCTURAL change (place, remove, rotate): every chunk these tiles
    /// touch is dirty, and its next delta is the whole chunk.
    fn mark_tiles_dirty(&mut self, tiles: &[TileCoord]) {
        for &tile in tiles {
            self.mark_dirty(tile);
            let chunk = chunk_of(tile);
            if !self.full_chunks.contains(&chunk) {
                self.full_chunks.push(chunk);
            }
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
        self.dirty_entities.insert(entity);
        // A belt handed an item by a splitter is marked this way too, and a
        // conveyor's delta membership is by tile.
        self.dirty_tiles.insert(origin);
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
    ///
    ///    **2b. Staff pools reset** (the staff pools MVP epic): every
    ///    category is back at its headcount less its heads in arrears (the
    ///    quality pass: an unpaid head stays hired but does not work), with
    ///    nothing asked for yet. It reads as a paragraph of step 2 rather
    ///    than a list item of its own because `2b.` is not a list marker
    ///    any markdown parser recognises — and renumbering is not the way
    ///    out: `dock.rs` and the tick body itself cite these steps by
    ///    number.
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
    /// 6. **Docks crew up.** A dock holding something an active contract
    ///    orders ([`Economy::wanted`]) claims its crew like a machine;
    ///    refused, it reads `understaffed` and cannot ship this tick (it
    ///    keeps taking boxes until its places are full, then the belt backs
    ///    up; nothing is lost). A dock holding only goods nobody has ordered
    ///    asks for nobody and is not `understaffed` — it has no work — so a
    ///    clog of an unordered product never holds a head another dock's
    ///    orders need. A mothballed dock claims nobody.
    /// 7. **Contracts settle**, one cadence check per active contract,
    ///    against what stands on every dock that can ship this tick — not
    ///    mothballed, not refused its crew in step 6 — lent to the economy
    ///    in `topology.docks` order (see [`super::dock`] and
    ///    [`super::economy::DeliveryStock`]). Every dock a payment took from
    ///    records the tick. A contract that pays records its mix in the
    ///    production ledger as consumed — the factory's exit — and the
    ///    ledger's clock advances after it, so everything this tick recorded
    ///    sits in the bucket that was current while it happened.
    /// 8. **Payday** (`StaffPools::tick`): on the tick the clock reaches
    ///    `next_payday_tick`, every head owed for the period — fired ones
    ///    included, notice pay first — is paid one at a time out of the one
    ///    money counter. The first head the money does not reach, and every
    ///    one after it, is UNPAID: still hired, idle until the next payday,
    ///    and gone if that one cannot pay it either (the quality pass; see
    ///    `staff.rs`'s module doc).
    ///
    /// Steps 4 and 5 are the same two phases the fixed chain always had
    /// ("belt to machine, then everything onto a belt"); what changed is
    /// that the pairs come from adjacency rather than from a stage index.
    pub fn tick(&mut self) {
        self.clear_dirty();
        self.tick_count += 1;
        self.revision += 1;

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
        for &ahead in belt_ahead.iter().take(belts.len()) {
            let room = room_at_start(self, ahead);
            self.room_ahead
                .push(room.unwrap_or(self.item_min_spacing()));
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
            self.room_ahead
                .push(room.unwrap_or(self.item_min_spacing()));
        }
        for (index, &carrier) in belts.iter().chain(splitters.iter()).enumerate() {
            let moved = {
                let mut segment = self
                    .ecs
                    .get::<&mut BeltSegment>(carrier)
                    .expect("a belt or splitter placement carries a BeltSegment");
                segment.tick_toward(self.belt_speed, self.room_ahead[index], self.tick_count)
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
            // Every network this machine can draw from, in the tile order
            // the topology pass found them; `Machine::tick` matches them to
            // its recipe's fluid inputs by id.
            let tapped: Vec<(usize, usize)> = taps
                .iter()
                .filter(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, inlet)| (network, inlet))
                .collect();
            let pushing = output_taps
                .iter()
                .find(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, outlet)| (network, outlet));
            let changed = {
                let mut body = self
                    .ecs
                    .get::<&mut Machine>(machine)
                    .expect("a machine placement carries a Machine");
                // The topology pass guarantees these networks are pairwise
                // different (see `recompute_topology`), which is what makes
                // holding every mutable borrow at once sound — and what
                // `disjoint_networks_mut` asserts rather than assumes. The
                // deposit borrow goes last, so popping it off the end
                // leaves the draws in tap order.
                let mut wanted: Vec<usize> = tapped.iter().map(|&(network, _)| network).collect();
                wanted.extend(pushing.map(|(network, _)| network));
                let mut borrowed = disjoint_networks_mut(&mut self.topology.networks, &wanted);
                let fluid_out = pushing.map(|(_, outlet)| FluidTap {
                    network: borrowed.pop().expect("pushing added the last borrow"),
                    inlet: outlet,
                });
                let fluid_in: Vec<FluidTap<'_>> = borrowed
                    .into_iter()
                    .zip(tapped.iter())
                    .map(|(network, &(_, inlet))| FluidTap { network, inlet })
                    .collect();
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
        //
        // Every dock's jam verdict is cleared first so the pass below can
        // only ever raise it: a dock fed by two belts is backing up if
        // EITHER was refused, and last-write-wins would report whichever
        // link the topology happened to visit last.
        // Taken and put back rather than cloned, as the two passes below do:
        // this runs every tick, and a fresh Vec per tick to read a list the
        // loop does not modify is an allocation for nothing.
        let docks = std::mem::take(&mut self.topology.docks);
        for &dock in &docks {
            if let Ok(mut body) = self.ecs.get::<&mut Dock>(dock) {
                body.clear_blocked();
            }
        }
        self.topology.docks = docks;
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

        // 6. Docks crew up.
        let docks = std::mem::take(&mut self.topology.docks);
        let wanted = self.economy.wanted();
        for &dock in &docks {
            let kind = self.placement_of(dock).kind;
            let flag_flipped = {
                let mut body = self
                    .ecs
                    .get::<&mut Dock>(dock)
                    .expect("a dock placement carries a Dock");
                let was = body.understaffed();
                if body.mothballed() {
                    // Switched off: it claims no clerk and ships nothing,
                    // and it is not "understaffed" either — nobody was
                    // asked for, so there is no shortage to report. The
                    // held goods stay held; `try_absorb_belt_into_dock`
                    // refuses, so the belt behind it backs up.
                    body.set_understaffed(false);
                } else if body.contents().any(|(item, _)| wanted.contains(item)) {
                    // Crew is claimed every tick the dock holds something a
                    // contract orders: that stock is the crew's work.
                    // Claiming for ANY stock let a dock clogged with a
                    // product nobody orders hold the only head forever and
                    // starve the dock whose stock was ordered (PR #182
                    // review).
                    let crew: &[(String, u32)] =
                        self.machines.get(&kind).map_or(&[], |k| k.staff.as_slice());
                    body.set_understaffed(self.staff.claim(crew).is_err());
                } else if !body.is_empty() {
                    // Holding only what nobody ordered: no work, nobody
                    // asked for, no shortage to report — as for a mothball.
                    body.set_understaffed(false);
                }
                // An empty dock asks for nobody either, and the verdict of
                // the last tick it held work stands.
                body.understaffed() != was
            };
            if flag_flipped {
                // `understaffed` rides the dock's chunk snapshot.
                self.mark_placement_dirty(dock);
            }
        }
        self.topology.belts = belts;
        self.topology.machines = machines;

        // 7. Contracts. Not tied to any tile, so it doesn't participate in
        // dirty-chunk tracking; Godot reads economy state via its own
        // always-fresh snapshot instead. A contract lost here (its strikes
        // used up) simply stops appearing in that snapshot's `contracts` and
        // reads `active: false` on its `board`; the events are not kept —
        // the snapshot is the wire, and a UI diffs it. What IS kept from the
        // events is what no snapshot can reconstruct afterwards: which docks
        // a payout took its mix from, and that it took it — the factory's
        // exit, and so its consumption of the finished product.
        //
        // The shipping docks' stock is lent out and handed back, so a dock
        // lent here is the dock at the same position in `lent`.
        let mut stock = DeliveryStock::default();
        let mut lent = Vec::new();
        for &dock in &docks {
            let mut body = self
                .ecs
                .get::<&mut Dock>(dock)
                .expect("a dock placement carries a Dock");
            if body.mothballed() || body.understaffed() {
                continue;
            }
            stock.push(body.take_stock());
            lent.push(dock);
        }
        let events = self.economy.tick(self.tick_count, &mut stock);
        let mut paid = vec![false; lent.len()];
        for event in &events {
            if let ContractEvent::Paid { id, from } = event {
                for &at in from {
                    paid[at] = true;
                }
                if let Some(contract) = self.economy.board().get(id) {
                    for (item, qty) in &contract.mix {
                        self.production.record_consumed(item, StatKind::Item, *qty);
                    }
                }
            }
        }
        for ((dock, held), paid) in lent.into_iter().zip(stock.into_docks()).zip(paid) {
            self.ecs
                .get::<&mut Dock>(dock)
                .expect("a dock placement carries a Dock")
                .return_stock(held, paid.then_some(self.tick_count));
            if paid {
                // Its contents and `last_paid_tick` ride its chunk snapshot.
                self.mark_placement_dirty(dock);
            }
        }
        self.topology.docks = docks;
        self.production.tick();

        // 8. Payday. Not tied to any tile either; Godot polls
        // `staff_snapshot`. A head left unpaid is simply not available when
        // the pools reset next tick; one who leaves is not there at all.
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
        if body.mothballed() {
            // A switched-off dock takes nothing off the belt, exactly as a
            // switched-off machine refuses its input: the box stays where
            // it is and the line backs up behind it.
            return false;
        }
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
        // Refused or not: a consumer cannot tell a refusal from a no-op
        // without re-reading, and one extra poll is cheaper than a stale
        // hotbar after a refusal it never heard about.
        self.revision += 1;
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
                //
                // One check `Purchase` has no equivalent of: an ingredient
                // behind a progression gate that has not opened is REFUSED
                // here, not merely hidden from the market panel
                // (`data/unlocks.ron`). A gate is a rule in the simulation;
                // the snapshot's filtered `entries` is presentation agreeing
                // with it, and a command forged by hand lands here.
                if !self.economy.ingredient_unlocked(&item) {
                    return;
                }
                // A second check `Purchase` has no equivalent of: stock has
                // a CEILING, and the ceiling is a building
                // (`storage.rs`). The request is clamped to the room that
                // is actually there rather than refused outright, for
                // `OwnedIngredients::take`'s reason — a quantity is not an
                // indivisible act the way placing a building is, and a
                // player who asks for fifty with room for ten meant "fill
                // me up". Clamped FIRST, so the price is the price of what
                // arrives: charging for what was asked and crediting what
                // fits would be a silent tax.
                let quantity = quantity.min(self.stock_room_for(&item));
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
            PlayerCommand::SetTankFluid { tile, fluid } => {
                self.try_set_tank_fluid(tile, &fluid);
            }
            PlayerCommand::SetMothballed { tile, enabled } => {
                self.try_set_mothballed(tile, enabled);
            }
            // Neither dirties a chunk: a head is not a tile. Both are silent
            // about an unknown category, `Hire` about a balance that could
            // not pay the next bill plus the new wage (the quality pass:
            // you cannot hire someone you could not pay), and `Fire` about
            // an empty pool — the `Purchase` convention.
            PlayerCommand::Hire { category } => {
                self.staff.hire(&category, self.economy.money);
            }
            PlayerCommand::Fire { category } => {
                self.staff.fire(&category);
            }
            // The cheat. One line, in the economy rather than on the field,
            // and no chunk dirtied — money is not a tile.
            PlayerCommand::DebugGrantMoney { amount } => {
                self.economy.grant(amount);
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
        // And a progression gate that has not opened refuses it too
        // (issue #90). `data/unlocks.ron` states the rule for
        // `BuyIngredient` and `SignContract` — a gate is a rule, not a
        // decoration on a panel — and these two commands had escaped it:
        // a hopper could be pointed at an ingredient nobody could buy and
        // then sit empty forever, saying only "in stock: 0".
        if !self.ingredient_unlocked(item) {
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

    /// [`PlayerCommand::SetTankFluid`] — `try_set_hopper_item`'s twin,
    /// against a tank, with two extra steps at the end.
    ///
    /// The last is the topology recompute. `sim_core/CLAUDE.md` says the
    /// pass runs on `Place`/`Remove`/`Rotate` and never per tick, and this
    /// does not weaken that rule: it is the same rule, applied to the same
    /// kind of thing. A declared fluid is a *seed* of
    /// [`resolve_network_fluids`], so changing it changes what the runs in
    /// this world carry exactly as moving a machine does, and a discrete
    /// player action is precisely what the pass is allowed to cost.
    ///
    /// **The one before it is what makes the emptying honest.**
    /// [`FluidTank::declare`] zeroes the tank so that the old fluid is not
    /// relabelled as the new one — and on its own that is only true of the
    /// tank. The recompute carries pipe volumes across by tile (it must:
    /// a player laying one more tile of pipe would otherwise drain the run),
    /// so whatever the tank had already discharged into the run survived the
    /// rebuild and came back under the new name. Measured on shipped
    /// content through the shipped picker: sixty units of vinegar became
    /// sixty units of liquid smoke, never bought, on one click.
    ///
    /// So the emptying is carried through: every run this tank discharges
    /// into is [purged](FluidNetwork::purge) in the same breath, *before*
    /// the recompute, so the rebuild carries zeroes across rather than a
    /// lie. The fluid is destroyed, not moved — nothing is credited, no
    /// stock returns, and `ProductionLedger` is not told a thing was used,
    /// because nothing used it.
    ///
    /// That the destruction is **larger than the tank** is why
    /// `main.gd`'s picker says what will be lost before the click rather
    /// than after: declaring is a destructive act, and the player consents
    /// to it. Refusing the switch while the run still holds the old fluid
    /// was the alternative, and it is the worse game — it leaves a player
    /// who mis-declared a tank with no way to drain the run they no longer
    /// want.
    ///
    /// **And a declaration that changes no fluid destroys nothing** (issue
    /// #121). Everything above is about a *switch*; a tank ships
    /// undeclared, so the commonest use of this command is not a switch at
    /// all but the player agreeing with the name the graph already derived,
    /// and that used to empty the tank and purge the run exactly as a
    /// switch does — 120 units of vinegar destroyed to replace vinegar with
    /// vinegar, on a panel whose note invited the click. See
    /// [`Self::declaration_settles_what_is_there`] for what "changes no
    /// fluid" means precisely; when it holds, the declaration is recorded
    /// through [`FluidTank::settle`] and nothing is purged.
    fn try_set_tank_fluid(&mut self, tile: TileCoord, fluid: &str) -> bool {
        if !self.raw_fluids.iter().any(|raw| raw == fluid) {
            return false;
        }
        // Locked is refused, exactly as for a hopper — see
        // `try_set_hopper_item`. The two pickers are built from one piece of
        // Godot code and must not behave differently (issue #90, which is
        // issue #43's own note that whatever was decided applied to both).
        if !self.ingredient_unlocked(fluid) {
            return false;
        }
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        if self.ecs.get::<&FluidTank>(entity).is_err() {
            return false;
        }
        // Asked before the declaration lands, because both halves of it
        // read state the declaration is about to change.
        let settles = self.declaration_settles_what_is_there(entity, fluid);
        let mut tank = self
            .ecs
            .get::<&mut FluidTank>(entity)
            .expect("the tile was a tank one statement ago");
        let changed = if settles {
            tank.settle(fluid.to_string())
        } else {
            tank.declare(fluid.to_string())
        };
        drop(tank);
        if !changed {
            return false;
        }
        if !settles {
            self.purge_runs_fed_by(entity);
        }
        self.mark_placement_dirty(entity);
        self.recompute_topology();
        true
    }

    /// Whether declaring `fluid` on `tank` would only write down what that
    /// tank and its run are already resolved to hold — the case
    /// [`Self::try_set_tank_fluid`] must not destroy anything for.
    ///
    /// Both halves are needed, and neither is "the run also carries
    /// `fluid`" on its own:
    ///
    /// * **the tank's own contents are `fluid`, or there are none.** A tank
    ///   holds no id of its own, so its contents are whatever
    ///   [`Self::tank_fluid`] resolves — the declaration if it has one, the
    ///   run's name otherwise. A tank declared vinegar and still holding
    ///   some of it is switching, however its run reads.
    /// * **every run it discharges into already resolves to
    ///   `Carries(fluid)`.** A run that reads `Ambiguous` or `Unnameable`
    ///   becomes something else the moment this declaration seeds it, and
    ///   whatever is standing in it was never `fluid` by any answer the
    ///   simulation had — so that declaration *is* a change and the purge
    ///   is right. A tank facing a junction may discharge into more than
    ///   one run; one of them changing is enough to make this a change.
    ///
    /// A tank discharging into no run at all is not settled either. It has
    /// no downstream to lose, so the distinction costs nothing there: the
    /// purge finds nothing and the tank it empties can only be empty, since
    /// [`Self::fill_tank_entity`] refuses a tank with no run to name what
    /// it is pouring.
    fn declaration_settles_what_is_there(&self, tank: Entity, fluid: &str) -> bool {
        let Ok(body) = self.ecs.get::<&FluidTank>(tank) else {
            return false;
        };
        let holds_only_this = body.is_empty();
        drop(body);
        if !holds_only_this && self.tank_fluid(tank).as_deref() != Some(fluid) {
            return false;
        }
        let mut runs = self
            .topology
            .tank_discharges
            .iter()
            .filter(|&&(entity, _, _)| entity == tank)
            .map(|&(_, network, _)| &self.topology.networks[network])
            .peekable();
        runs.peek().is_some()
            && runs.all(|run| {
                run.naming() == PipeNaming::Carries && run.fluid.as_deref() == Some(fluid)
            })
    }

    /// Destroys what every run `tank` discharges into is holding, and marks
    /// each drained pipe tile so the change reaches the screen.
    ///
    /// Walks `tank_discharges` rather than the tank's neighbours because
    /// that list is already the answer to "which network, which segment",
    /// computed by the topology pass. It does *not* walk it because a tank
    /// can have several entries: `fluid_tank` is `Footprint(1, 1)` in
    /// `data/machines.ron` and discharges through the one face it points at,
    /// so the topology pass cannot currently produce more than one. The
    /// `BTreeSet` below is defensive against a multi-outlet tank, not a
    /// shape that occurs today. See [`Self::try_set_tank_fluid`] for why
    /// this exists at all.
    fn purge_runs_fed_by(&mut self, tank: Entity) {
        let networks: BTreeSet<usize> = self
            .topology
            .tank_discharges
            .iter()
            .filter(|&&(entity, _, _)| entity == tank)
            .map(|&(_, network, _)| network)
            .collect();
        for network in networks {
            if self.topology.networks[network].purge() > Fx::ZERO {
                let tiles: Vec<TileCoord> = self.topology.networks[network]
                    .segments()
                    .iter()
                    .map(|segment| segment.tile)
                    .collect();
                for tile in tiles {
                    if let Some(&pipe) = self.topology.grid.get(&tile) {
                        self.mark_placement_dirty(pipe);
                    }
                }
            }
        }
    }

    /// [`PlayerCommand::SetMothballed`]. Silent no-op on an empty tile, on
    /// the state it already has, and on **anything whose kind asks for no
    /// crew**.
    ///
    /// That last one is the whole rule, and it is asked of the content
    /// rather than of the Rust type: a placement may be switched off iff
    /// `machines.ron` gives its kind a non-empty `staff` list. Mothballing
    /// exists to take wages off the bill (issue #19), so switching off a
    /// belt, a pipe, a hopper or a tank would cost the player production
    /// and save nothing — while the DOCK, which does draw a clerk, must be
    /// switchable or the bill can never reach zero and a factory with no
    /// income can never climb out of arrears. Spelling the rule as "only a
    /// `Machine`" hard-codes in Rust an answer the data already holds.
    ///
    /// **[`Self::recompute_establishment`], not
    /// [`Self::recompute_topology`].** Switching a placement off changes
    /// exactly one derived thing — how many heads the placed machinery
    /// needs — and none of the things the topology pass exists to rebuild:
    /// the occupancy grid, the hand-off links and the fluid networks are
    /// all still what they were, and a mothballed machine keeps its taps so
    /// that returning it to service needs no rebuild either. Calling the
    /// narrow one keeps the rule that a full rebuild happens on
    /// `Place`/`Remove`/`Rotate` and nowhere else.
    fn try_set_mothballed(&mut self, tile: TileCoord, enabled: bool) -> bool {
        let Some(&entity) = self.topology.grid.get(&tile) else {
            return false;
        };
        if self.ecs.get::<&Position>(entity).is_err() {
            return false;
        }
        let kind = self.placement_of(entity).kind;
        let asks_for_a_crew = self
            .machines
            .get(&kind)
            .is_some_and(|def| !def.staff.is_empty());
        if !asks_for_a_crew || self.is_mothballed(entity) == enabled {
            return false;
        }
        if let Ok(mut machine) = self.ecs.get::<&mut Machine>(entity) {
            machine.mothballed = enabled;
        } else if let Ok(mut dock) = self.ecs.get::<&mut Dock>(entity) {
            dock.set_mothballed(enabled);
        } else {
            return false;
        }
        self.recompute_establishment();
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

    /// How many ticks and commands this world has taken, in total. Two
    /// equal readings mean NOTHING on the wire has changed between them —
    /// every polled snapshot (economy, shop, hoppers, tanks, fluid,
    /// ingredients, staff, production) is a function of the state those two
    /// operations mutate — so a presentation layer that parses snapshots
    /// every frame can skip the frames between ticks, and a refused command
    /// still bumps it (a refusal is a fact the hotbar has to re-read).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn clear_dirty(&mut self) {
        self.dirty.clear();
        self.dirty_tiles.clear();
        self.dirty_entities.clear();
        self.full_chunks.clear();
    }

    /// [`World::chunk_snapshot`] cut down to what CHANGED since the dirty
    /// set was last cleared: the belts and splitters whose contents moved,
    /// and the machines, hoppers, tanks and docks whose live state did.
    /// Pipes are never in it (fluid is polled, its shape is on the layout).
    /// After a structural change to the chunk — a place, remove, rotate or a
    /// load — it IS the whole snapshot, because a consumer holds no last
    /// state for a tile that did not exist a moment ago.
    ///
    /// **Why this exists (the quality pass, 2026-09-13).** A belt tile
    /// dirties its chunk when an item on it moves, and on a running line
    /// some belt in every chunk moves every tick, so the per-chunk
    /// snapshot was re-sending EVERY belt in the chunk — 960 tiles, most of
    /// them empty — thirty times a second, and the presentation layer
    /// parsed and re-drew all of them: about 12 ms of a 33 ms tick against
    /// the sim's own 1 ms. The granularity of *what is reported dirty*
    /// stays per chunk (ARCHITECTURE.md §4); what a dirty chunk's read
    /// carries is now proportional to what moved. A jammed line sends
    /// nothing at all. The same-shape [`ChunkSnapshot`] is returned so a
    /// consumer's render path does not fork.
    pub fn chunk_delta(&self, id: ChunkId) -> ChunkSnapshot {
        if self.full_chunks.contains(&id) {
            return self.chunk_snapshot(id);
        }
        // Walk what CHANGED, not the chunk: on a big floor the changed set
        // is a small fraction of the chunk, and this runs per dirty chunk
        // per tick. Sorted so the output order is the topology's own (by
        // tile, y then x) and a reading is deterministic.
        let mut snapshot = ChunkSnapshot::default();
        let mut tiles: Vec<TileCoord> = self
            .dirty_tiles
            .iter()
            .copied()
            .filter(|&tile| chunk_of(tile) == id)
            .collect();
        tiles.sort_by_key(|&(x, y)| (y, x));
        for tile in tiles {
            let Some(&entity) = self.topology.grid.get(&tile) else {
                continue;
            };
            match self.body_kind(entity) {
                Some(BodyKind::Belt) => snapshot.belts.push(self.belt_snapshot(entity)),
                Some(BodyKind::Splitter) => snapshot.splitters.push(self.splitter_snapshot(entity)),
                _ => {}
            }
        }
        let mut entities: Vec<(TileCoord, Entity)> = self
            .dirty_entities
            .iter()
            .filter(|&&entity| self.ecs.contains(entity))
            .map(|&entity| (self.position_of(entity), entity))
            .collect();
        entities.sort_by_key(|&((x, y), entity)| (y, x, entity.id()));
        for (_, entity) in entities {
            if !self.overlaps_chunk(entity, id) {
                continue;
            }
            match self.body_kind(entity) {
                Some(BodyKind::Machine) => snapshot.machines.push(self.machine_snapshot(entity)),
                Some(BodyKind::Hopper) => snapshot.hoppers.push(self.hopper_snapshot(entity)),
                Some(BodyKind::FluidTank) => {
                    snapshot.fluid_tanks.push(self.fluid_tank_snapshot(entity))
                }
                Some(BodyKind::Dock) => snapshot.docks.push(self.dock_snapshot(entity)),
                _ => {}
            }
        }
        snapshot
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
                let run = &self.topology.networks[network];
                snapshot
                    .pipes
                    .push(pipe_snapshot(run, &run.segments()[segment]));
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
        // Computed once for the whole walk rather than per placement: it is
        // one answer about the whole factory, not a property of a tile.
        let uncrewed = self.uncrewed_placements();
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
                    crewed: !uncrewed.contains(&entity),
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
        let uncrewed = self.uncrewed_placements();
        self.topology
            .machines
            .iter()
            .map(|&machine| {
                let body = self.ecs.get::<&Machine>(machine).unwrap();
                let load = MachineLoadSnapshot::of(&body.load);
                let mothballed = body.mothballed;
                drop(body);
                MachineLoadEntry {
                    tile: self.position_of(machine),
                    load,
                    stall: self.machine_stall(machine),
                    crewed: !uncrewed.contains(&machine),
                    mothballed,
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
                    naming: network.naming(),
                    candidates: network.candidates().to_vec(),
                    segments: network
                        .segments()
                        .iter()
                        .map(|segment| pipe_snapshot(network, segment))
                        .collect(),
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
                // Which side of the recipe this tap is on is the caller's
                // business; *which entry* of that side is this run's, since
                // a recipe may draw several fluids from several runs. Match
                // by the id the network carries, and report 0 for a tap on
                // a run the current recipe has no use for — the machine is
                // plumbed into it but is not drawing from it this craft.
                let fluid = self.topology.networks[index].fluid.as_deref();
                let units_per_craft = self
                    .recipes
                    .get(&body.recipe)
                    .and_then(|r| {
                        let side = if producing {
                            &r.fluid_outputs
                        } else {
                            &r.fluid_inputs
                        };
                        side.iter()
                            .find(|(id, _)| Some(id.as_str()) == fluid)
                            .map(|(_, units)| *units)
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
            moved_ticks: segment.moved_ticks(),
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
            moved_ticks: segment.moved_ticks(),
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
            mothballed: body.mothballed,
            stall: self.machine_stall(machine),
        }
    }

    /// Why `machine` is waiting at this moment, if it is — see [`Stall`] for
    /// the rule. Read-only: it asks the same questions `Machine::tick` asks
    /// (enough input? a tap that can supply? room to deposit?) against the
    /// same taps `tick` would hand it, without moving anything.
    fn machine_stall(&self, machine: Entity) -> Option<Stall> {
        let body = self.ecs.get::<&Machine>(machine).ok()?;
        if body.mothballed {
            // Ahead of everything: a machine that is off is not waiting for
            // anything, and its buffers say nothing about why it is idle.
            return Some(Stall {
                kind: StallKind::Mothballed,
                item: String::new(),
            });
        }
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
        // Every tap on one side, in the order `tick` would be handed them.
        // Plural now: a recipe may draw several fluids, each from whichever
        // touching run carries it.
        let taps_of = |taps: &[(Entity, usize, usize)]| -> Vec<(usize, usize)> {
            taps.iter()
                .filter(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, segment)| (network, segment))
                .collect()
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
                let taps = taps_of(&self.topology.machine_taps);
                let mut claimed: Vec<usize> = Vec::new();
                for (fluid, units) in &recipe.fluid_inputs {
                    // The same first-fit matching `Machine::tick` does, so
                    // the reason shown is the reason it stalled. A run
                    // carrying the right fluid but short of it reads as
                    // `FluidInput`; nothing touching that carries it at all
                    // reads as `NoSupplyPipe`, which is the "you have not
                    // plumbed this in" message and covers no pipe at all.
                    let supplier = taps.iter().find(|(network, segment)| {
                        !claimed.contains(network)
                            && self.topology.networks[*network].can_supply_at(
                                *segment,
                                fluid,
                                Fx::from_num(*units),
                            )
                    });
                    match supplier {
                        Some(&(network, _)) => claimed.push(network),
                        None if taps.iter().any(|&(network, _)| {
                            !claimed.contains(&network)
                                && self.topology.networks[network].fluid.as_deref()
                                    == Some(fluid.as_str())
                        }) =>
                        {
                            return stall(StallKind::FluidInput, fluid);
                        }
                        // A run IS here and it is unnamed: say so, rather
                        // than "no supply pipe" about a pipe the player can
                        // see they have laid (issue #54). Only reached once
                        // no touching run carries the fluid at all, so a
                        // working line beside a broken one still reports the
                        // working line's own shortfall.
                        None => {
                            return match taps.iter().find_map(|(network, _)| {
                                let naming = self.topology.networks[*network].naming();
                                naming.is_fault().then_some(naming)
                            }) {
                                Some(PipeNaming::Ambiguous) => {
                                    stall(StallKind::AmbiguousPipe, fluid)
                                }
                                Some(_) => stall(StallKind::UnnameablePipe, fluid),
                                None => stall(StallKind::NoSupplyPipe, fluid),
                            };
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
                let tap = taps_of(&self.topology.machine_output_taps).first().copied();
                for (fluid, units) in &recipe.fluid_outputs {
                    match tap {
                        None => return stall(StallKind::NoOutletPipe, fluid),
                        Some((network, segment)) => {
                            // The outlet run's naming first, for the draw
                            // side's reason: an unnamed run refuses a
                            // deposit however empty it is, and "outlet pipe
                            // full" about an empty pipe is the answer that
                            // sent issue #54's author to a test harness.
                            let naming = self.topology.networks[network].naming();
                            if naming.is_fault() {
                                return stall(
                                    match naming {
                                        PipeNaming::Ambiguous => StallKind::AmbiguousPipe,
                                        _ => StallKind::UnnameablePipe,
                                    },
                                    fluid,
                                );
                            }
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
            fluid: body.declared().cloned().or_else(|| {
                discharge.and_then(|(network, _)| self.topology.networks[network].fluid.clone())
            }),
            declared: body.declared().cloned(),
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
        // The body's fields are lifted out before `is_crewed`, which walks
        // every placement and would otherwise be called while this dock's
        // component is still borrowed.
        let (contents, places, used, at_capacity, shipped_total, last_paid_tick) = {
            let body = self.ecs.get::<&Dock>(dock).unwrap();
            (
                body.contents().cloned().collect(),
                body.places(),
                body.used(),
                body.at_capacity(),
                body.shipped_total(),
                body.last_paid_tick(),
            )
        };
        let (understaffed, mothballed, blocked) = {
            let body = self.ecs.get::<&Dock>(dock).unwrap();
            (body.understaffed(), body.mothballed(), body.blocked())
        };
        DockSnapshot {
            tile: self.position_of(dock),
            kind: placement.kind,
            direction: placement.direction,
            footprint: placement.footprint,
            belt_tile: self.inbound_belt_tile(dock),
            contents,
            places,
            used,
            at_capacity,
            shipped_total,
            last_paid_tick,
            understaffed,
            crewed: self.is_crewed(dock),
            mothballed,
            full: blocked,
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

    /// The fluid a tank is currently holding: **what it declares**, and
    /// failing that the fluid of the network it discharges into.
    ///
    /// The fallback is what keeps every tank that predates issue #43
    /// behaving as it did — an undeclared tank is still named by its run,
    /// which is right for the common case of one tank feeding one line. The
    /// declaration wins where there is one because it is also what seeded
    /// that run ([`resolve_network_fluids`]), so the two agree wherever the
    /// run resolved at all, and the declaration is the only answer where it
    /// did not. Read off the topology's own links, so it can never disagree
    /// with [`FluidTankSnapshot::fluid`], which is derived the same way.
    fn tank_fluid(&self, tank: Entity) -> Option<FluidId> {
        if let Some(declared) = self
            .ecs
            .get::<&FluidTank>(tank)
            .ok()
            .and_then(|body| body.declared().cloned())
        {
            return Some(declared);
        }
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
                    mothballed: body.mothballed,
                    stall: {
                        drop(body);
                        self.machine_stall(entity)
                    },
                    crew,
                    crewed: self.is_crewed(entity),
                }
            }
            BodyKind::Pipe => {
                let &(network_index, segment_index) = self.topology.pipe_index.get(&pos)?;
                let network = &self.topology.networks[network_index];
                let segment = &network.segments()[segment_index];
                TileSnapshot::Pipe {
                    direction: placement.direction,
                    fluid: network.fluid.clone(),
                    naming: network.naming(),
                    candidates: network.candidates().to_vec(),
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
            BodyKind::WarehouseRack => {
                let snapshot = self.rack_snapshot(entity, &self.bay_runs());
                TileSnapshot::WarehouseRack {
                    direction: snapshot.direction,
                    footprint: snapshot.footprint,
                    origin_tile: snapshot.tile,
                    bays: snapshot.bays,
                    contents: snapshot.contents,
                }
            }
            BodyKind::Dock => {
                let snapshot = self.dock_snapshot(entity);
                TileSnapshot::Dock {
                    direction: snapshot.direction,
                    footprint: snapshot.footprint,
                    origin_tile: snapshot.tile,
                    contents: snapshot.contents,
                    places: snapshot.places,
                    used: snapshot.used,
                    at_capacity: snapshot.at_capacity,
                    shipped_total: snapshot.shipped_total,
                    last_paid_tick: snapshot.last_paid_tick,
                    crew: crew_entries(self.crew_of_kind(&placement.kind)),
                    crewed: self.is_crewed(entity),
                    understaffed: snapshot.understaffed,
                    mothballed: snapshot.mothballed,
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
    /// Sets the lifetime delivery counter outright — the seam a load uses
    /// to put back what [`Economy::tick`] counted, and the one
    /// [`super::reference::bbq_branch`] uses to say "this factory belongs to
    /// a player who has already earned the BBQ tier".
    ///
    /// [`restore_money`](Self::restore_money)'s rule, for its reason: the
    /// counter otherwise only ever moves through a satisfied delivery
    /// window, and a second public way to raise it would be a cheat surface
    /// on the real API. It reaches a state a player genuinely reaches, and
    /// `pub(super)` keeps it out of reach of anything but this module tree.
    pub(super) fn restore_lifetime_deliveries(&mut self, deliveries: u32) {
        self.economy.lifetime_deliveries = deliveries;
    }

    pub(super) fn restore_money(&mut self, money: Money) {
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
    /// Every recipe id a machine of `kind` can run, in `data/recipes.ron`
    /// order — the list [`super::machine::Machine::choose`] picks from, and
    /// what [`super::save`] checks a saved in-flight craft against. Empty
    /// for a kind no recipe names, which is a kind that cannot be placed.
    pub(super) fn recipes_for_kind(&self, kind: &str) -> Vec<String> {
        self.recipes
            .for_machine(kind)
            .map(|recipe| recipe.id.clone())
            .collect()
    }

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
        self.full_chunks = self.dirty.clone();
    }
}

/// Enforces that every part of `data/recipes.ron` is content a player can
/// actually reach: each connected component of the recipe graph must hold
/// at least one recipe that can be **started** — its machine is for sale in
/// `data/shop.ron`, and everything it consumes is obtainable. A clear panic
/// here, at construction time before any tick runs, is the data-file
/// equivalent of a compiler error.
///
/// "Obtainable" is a closure, not a one-step test: an id is obtainable when
/// `data/ingredients.ron` prices it, or when some startable recipe makes it.
/// The set grows outwards from what the market sells until it stops growing.
/// That is what makes a whole line fed by an unpriced raw fail here even
/// though its *last* stage, read on its own, runs on a machine the shop
/// sells and consumes only what the stage before it makes.
///
/// The graph's nodes are recipes and its edges join any two sharing an id —
/// solid or fluid, in either direction. What this used to demand is that
/// the graph be *connected*: one component, every recipe reachable from the
/// first by a shared id. That rule replaced an even stricter "recipe `i + 1`
/// consumes something recipe `i` produces" in the BBQ epic, when the file
/// became a *branching* chain (ketchup feeds both the bottler and the BBQ
/// mixer) and consecutive entries stopped chaining.
///
/// **Connectivity is gone because the claim under it expired** (issue #42).
/// "One component" says every sauce shares an ancestor with every other,
/// which was true only while BBQ derived from ketchup, which derives from
/// tomato. Mayonnaise is eggs and oil and is tier 4 in GAMEPLAY.md §2: it
/// shares nothing with the tomato line, and the day its recipes land a
/// perfectly valid file would have panicked here with a message reading
/// like a typo report. A second production line is content, not a fault.
///
/// What survives is the half that was never about connectivity: **a
/// component nobody can build or feed is dead weight**, and a stage joined
/// to nothing is exactly that. An island of recipes with no purchasable
/// machine can never be placed; an island whose supply bottoms out in an
/// unpriced raw can never be started, which is what a typo'd input id
/// produces (it is consumed-nowhere-produced, i.e. a new raw ingredient,
/// and nobody priced it). Either way the player would watch a machine that
/// never runs with no clue why — the error this check has always existed to
/// turn into a load-time message.
///
/// **Which net catches what.** These two cover the content errors between
/// them, and neither does alone:
///
/// - *This check* catches a **structural break**: a fragment of a file that
///   no purchase and no ingredient can ever set in motion.
/// - [`super::ingredients::IngredientCatalog::validate_covers`] catches a
///   **typo'd id**, as a test rather than a panic, and catches it even when
///   the typo sits in a component some *other* recipe anchors. A misspelled
///   input is by definition consumed-nowhere-produced, i.e. a new raw
///   ingredient, and raw ingredients must be priced — so the unpriced
///   `tomatoe` fails there. `data/ingredients.ron`'s header says why that
///   one is a test: pricing is a content judgement, not a load-time
///   invariant, and *every* raw being priced is a stronger claim than this
///   one makes. This check asks only that each component have **one** way
///   in, which is the difference between "your content is unbalanced" and
///   "this half of your file is unreachable".
fn validate_chain(
    ordered: &[Recipe],
    raws: &[String],
    shop: &ShopCatalog,
    ingredients: &IngredientCatalog,
) {
    fn ids_of(recipe: &Recipe) -> BTreeSet<&str> {
        recipe
            .inputs
            .iter()
            .chain(recipe.outputs.iter())
            .map(|(item, _)| item.as_str())
            .chain(
                recipe
                    .fluid_inputs
                    .iter()
                    .chain(recipe.fluid_outputs.iter())
                    .map(|(fluid, _)| fluid.as_str()),
            )
            .collect()
    }
    // Union-find would be overkill for a file this size: flood out from each
    // recipe not yet in a component, which labels every one of them exactly
    // once, in file order.
    let touched: Vec<BTreeSet<&str>> = ordered.iter().map(ids_of).collect();
    let mut component: Vec<usize> = vec![usize::MAX; ordered.len()];
    let mut components = 0usize;
    for seed in 0..ordered.len() {
        if component[seed] != usize::MAX {
            continue;
        }
        component[seed] = components;
        let mut frontier = vec![seed];
        while let Some(index) = frontier.pop() {
            for (other, ids) in touched.iter().enumerate() {
                if component[other] == usize::MAX && !ids.is_disjoint(&touched[index]) {
                    component[other] = components;
                    frontier.push(other);
                }
            }
        }
        components += 1;
    }

    // A recipe can be *started* when the player can buy the machine it runs
    // on and can actually get hold of everything it consumes. "Get hold of"
    // is a closure rather than a one-step test: a priced raw is obtainable,
    // and so is anything a startable recipe makes. So the set grows from
    // what the market sells outwards, and a recipe whose only supplier is
    // itself unstartable never enters it — which is what makes a line
    // fed by an unpriced raw fail here even though its *last* stage, read
    // alone, looks perfectly buildable.
    let mut obtainable: BTreeSet<&str> = raws
        .iter()
        .map(String::as_str)
        .filter(|id| ingredients.get(id).is_some())
        .collect();
    let mut startable: Vec<bool> = vec![false; ordered.len()];
    loop {
        let mut grew = false;
        for (index, recipe) in ordered.iter().enumerate() {
            if startable[index] || shop.get(&recipe.machine).is_none() {
                continue;
            }
            let fed = recipe
                .inputs
                .iter()
                .chain(recipe.fluid_inputs.iter())
                .all(|(id, _)| obtainable.contains(id.as_str()));
            if !fed {
                continue;
            }
            startable[index] = true;
            grew = true;
            for (id, _) in recipe.outputs.iter().chain(recipe.fluid_outputs.iter()) {
                obtainable.insert(id.as_str());
            }
        }
        if !grew {
            break;
        }
    }
    for index in 0..components {
        let anchored = component
            .iter()
            .zip(&startable)
            .any(|(&label, &started)| label == index && started);
        if anchored {
            continue;
        }
        let ids: Vec<&str> = ordered
            .iter()
            .zip(&component)
            .filter(|&(_, &label)| label == index)
            .map(|(recipe, _)| recipe.id.as_str())
            .collect();
        panic!(
            "recipes.ron chain error: recipe(s) {ids:?} form a part of the file nothing can set \
             in motion — not one of them runs on a machine `data/shop.ron` sells and is fed by \
             ingredients the player can get, whether bought from `data/ingredients.ron` or made \
             by another recipe that can itself be started. Several independent production lines \
             are fine; a line with no way into it is not. (A misspelled id usually shows up here \
             as an unpriced raw ingredient, and otherwise in \
             IngredientCatalog::validate_covers.)"
        );
    }
}

/// One machine's plumbing as the topology pass found it — pure **geometry**,
/// with no recipe attached to it. `output` is the network off the outlet
/// face (the only one a machine may deposit into, and `None` if no pipe
/// stands there); `inputs` is every *other* network touching the footprint,
/// in tile order, one entry per network.
///
/// Kept separate from `Topology::machine_taps` because that list is keyed
/// for the tick's per-machine lookups, while this one is what
/// [`resolve_network_fluids`] walks: the same facts, shaped for the two
/// different questions.
struct Plumbing {
    kind: String,
    output: Option<usize>,
    inputs: Vec<usize>,
}

/// Decides what fluid each network carries, for the whole graph at once.
///
/// This is the subtlest thing in this file; the *why* is in the module doc
/// under "What a pipe run carries". In short: a machine kind may run
/// several recipes, so a run's id can no longer be read off the one recipe
/// a machine at its end was assigned. It is derived instead, by narrowing:
///
/// 1. **What could reach it.** Each run starts with the set of fluids that
///    can physically get into it. A run a tank discharges into can hold a
///    raw fluid — nothing else enters the graph from outside. A run off a
///    machine's outlet face can hold anything that kind's recipes make.
///    Every other run starts empty, which is why a length of pipe joined to
///    nothing stays unnamed. A run whose tank **declares** what it holds
///    (`PlayerCommand::SetTankFluid`, issue #43) starts as that one id and
///    nothing else: the player has said, and no amount of geometry
///    outranks that.
/// 2. **Narrowing.** A recipe is *feasible* for a machine when its fluid
///    output is still possible on the outlet run and each of its fluid
///    inputs can be matched, by id, to a distinct run touching the
///    footprint that could hold it. If **every** feasible recipe of a
///    machine's kind puts the same few ids on a run, that run cannot hold
///    anything else, so its set is intersected down to them. A machine with
///    no feasible recipe at all says nothing rather than emptying its runs:
///    a mixer plumbed so it can never draw still names what it would fill.
/// 3. **Naming.** A run whose set has narrowed to exactly one id carries
///    it. Two or more left is genuine ambiguity; none left is a
///    contradiction; either way the run stays **unnamed** — but it records
///    *which* of the two it was, and what it narrowed to, in
///    [`FluidNetwork::naming`] and `candidates`. That is the whole of issue
///    #54: the two have different fixes (turn a source round; separate two
///    merged runs), and before this pass said which, neither reached the
///    player at all.
///
/// Step 2 repeats until a sweep changes nothing. Sets only ever shrink and
/// each sweep is computed from the previous sweep's sets and applied all at
/// once, so it terminates (every productive sweep drops at least one id
/// from one run) and the answer does not depend on the order machines or
/// runs are walked in — build the same layout in a different placement
/// order and the naming is identical
/// (`pipe_naming_is_independent_of_placement_order`).
///
/// **Why a declaration had to exist.** Steps 1 and 2 are both about what
/// the *layout* implies, and one layout implies nothing: a machine drawing
/// two raw fluids from two tank-fed runs seeds both runs to the same pair,
/// and the narrowing step intersects both with that same pair, so neither
/// ever reaches one id. Both runs stay unnamed and the machine stalls.
/// There was no bug to fix in the propagation — the information simply was
/// not in the world, and the only place it exists is the player's head.
///
/// **An unnamed run is not an error.** `can_supply_at`/`can_accept_at`
/// reject one, so its machines stall — the same graceful "not plumbed yet"
/// a player sees for most of the time they spend laying a run, and never a
/// panic. It is, however, **said out loud** since issue #54: an unnamed run
/// something is plumbed to reports [`PipeNaming::Unnameable`] or
/// [`PipeNaming::Ambiguous`] rather than being indistinguishable from a run
/// that is merely empty, and the machines on it stall with
/// [`StallKind::UnnameablePipe`]/[`StallKind::AmbiguousPipe`] instead of the
/// misleading "no supply pipe" they used to report while pointing at a pipe.
fn resolve_network_fluids(
    networks: &mut [FluidNetwork],
    plumbing: &[Plumbing],
    recipes: &RecipeRegistry,
    raw_fluids: &BTreeSet<FluidId>,
    declared: &[BTreeSet<FluidId>],
) {
    // 1. Only a *machine* claims an id for a run: what it would deposit
    // there, or — for a run no machine fills — what it would draw from
    // there. The second half is what makes a run a player has laid but not
    // yet given a source read as the line it is going to be rather than as
    // nothing. A run no machine touches at all is claimed by nobody and
    // stays unnamed however much pipe it is, which is the pre-BBQ
    // behaviour and the reason a tank plumbed to nothing loads nothing.
    let mut filled_by: Vec<BTreeSet<FluidId>> = vec![BTreeSet::new(); networks.len()];
    let mut drawn_by: Vec<BTreeSet<FluidId>> = vec![BTreeSet::new(); networks.len()];
    for machine in plumbing {
        for recipe in recipes.for_machine(&machine.kind) {
            if let Some(network) = machine.output {
                for (fluid, _) in &recipe.fluid_outputs {
                    filled_by[network].insert(fluid.clone());
                }
            }
            for &network in &machine.inputs {
                for (fluid, _) in &recipe.fluid_inputs {
                    drawn_by[network].insert(fluid.clone());
                }
            }
        }
    }
    let mut possible: Vec<BTreeSet<FluidId>> = filled_by
        .into_iter()
        .zip(drawn_by)
        .map(|(filled, drawn)| if filled.is_empty() { drawn } else { filled })
        .collect();
    // A tank that has not been asked names nothing by itself — its own
    // fluid is whatever the run it discharges into carries. What it
    // contributes is the opposite of a claim: it is the only way into the
    // graph from outside, and the only thing that comes in from outside is
    // a **raw** fluid, so a run a tank feeds can hold nothing else. That is
    // what makes the propagation terminate somewhere sensible instead of
    // guessing: one end of every line is pinned by what the player buys.
    for (network, ids) in networks.iter().zip(possible.iter_mut()) {
        if network.segments().iter().any(|segment| segment.is_source) {
            ids.retain(|fluid| raw_fluids.contains(fluid));
        }
    }
    // A tank that HAS been asked (`PlayerCommand::SetTankFluid`, issue #43)
    // pins its run outright, and that is the whole of the fix: two runs
    // into one two-fluid machine are told apart by the player saying which
    // drum is which, because nothing about the geometry ever could.
    //
    // It **replaces** the set rather than intersecting with it, so a
    // declared tank names its run even when no machine is plumbed to it
    // yet — a player can fill a tank before building what drinks from it,
    // as they could not before. Disagreement is not silently resolved:
    // two tanks declaring different fluids into one run leave two ids on
    // the run and it reads `Ambiguous`. A declaration no machine on the run
    // can use is caught by step 2b below, which is where a statement the
    // layout contradicts is turned back into an unnamed run.
    for (ids, declared) in possible.iter_mut().zip(declared) {
        if !declared.is_empty() {
            *ids = declared.clone();
        }
    }

    // 2. One id dropped per productive sweep, at the very least.
    let bound: usize = possible.iter().map(BTreeSet::len).sum();
    for _ in 0..=bound {
        let mut narrowed = possible.clone();
        for machine in plumbing {
            let feasible: Vec<Vec<(usize, FluidId)>> = recipes
                .for_machine(&machine.kind)
                .filter_map(|recipe| feasible_naming(recipe, machine, &possible))
                .collect();
            if feasible.is_empty() {
                continue;
            }
            let mut claims: BTreeMap<usize, BTreeSet<FluidId>> = BTreeMap::new();
            for naming in &feasible {
                for (network, fluid) in naming {
                    claims.entry(*network).or_default().insert(fluid.clone());
                }
            }
            for (network, ids) in claims {
                // Only a run *every* feasible recipe uses is narrowed. One
                // that some of them leave alone — an unused tap, or an
                // outlet a fluidless recipe never fills — is left as it is,
                // because that recipe running would put nothing there.
                let used_by_all = feasible
                    .iter()
                    .all(|naming| naming.iter().any(|&(used, _)| used == network));
                if used_by_all {
                    narrowed[network].retain(|fluid| ids.contains(fluid));
                }
            }
        }
        if narrowed == possible {
            break;
        }
        possible = narrowed;
    }

    // 2b. A declaration the machines on the run cannot use is a
    // contradiction, and it is reported on the run the player declared.
    //
    // Step 1 replaces the set and step 2 never touches it again — a machine
    // with no feasible recipe `continue`s rather than emptying its runs — so
    // a tank declared as something the machine it feeds cannot drink came
    // out `Carries`, confidently named, while the *outlet* run took the
    // blame for the stall. That is the wrong tile: the mistake is the
    // declaration, and issue #54's rule is that a run the simulation cannot
    // make sense of says so rather than pointing somewhere else.
    //
    // The test is about the machine's **kind**, not about today's
    // half-built layout: a run is contradicted only when no recipe of any
    // machine drawing from it takes the declared fluid at all. A recipe
    // that is merely infeasible right now — because the player has not
    // finished plumbing its other input — must not condemn the
    // declaration, or the run would flicker Unnameable while a line is
    // being laid. A run nothing drinks from is left alone for the same
    // reason: declaring a tank before building is explicitly allowed.
    for network in 0..possible.len() {
        if declared[network].is_empty() || possible[network].is_empty() {
            continue;
        }
        let drinkers: Vec<&Plumbing> = plumbing
            .iter()
            .filter(|machine| machine.inputs.contains(&network))
            .collect();
        // A run some machine fills is not the player's to contradict: the
        // disagreement there is between a declaration and a product, which
        // the narrowing step already has its own answer for.
        if drinkers.is_empty() || plumbing.iter().any(|m| m.output == Some(network)) {
            continue;
        }
        let usable = drinkers.iter().any(|machine| {
            recipes.for_machine(&machine.kind).any(|recipe| {
                recipe
                    .fluid_inputs
                    .iter()
                    .any(|(fluid, _)| possible[network].contains(fluid))
            })
        });
        if !usable {
            possible[network].clear();
        }
    }

    // 3. Naming, with its reason (issue #54). Exactly one id left names the
    // run; none or several leaves it unnamed, and those are two different
    // mistakes with two different fixes, so the run records which — and
    // whether anything is actually plumbed to it, because a run nobody has
    // connected to yet is the ordinary state of pipe a player is still
    // laying and must not read as a fault.
    let mut attached: Vec<bool> = networks
        .iter()
        .map(|network| network.segments().iter().any(|segment| segment.is_source))
        .collect();
    for machine in plumbing {
        if let Some(network) = machine.output {
            attached[network] = true;
        }
        for &network in &machine.inputs {
            attached[network] = true;
        }
    }
    for ((network, ids), attached) in networks.iter_mut().zip(possible).zip(attached) {
        network.set_naming(ids.into_iter().collect(), attached);
    }
}

/// Which runs `recipe` would put which fluid into, if the machine plumbed
/// as `machine` ran it — or `None` when it could not run at all as this
/// machine is plumbed, and so has nothing to say about these runs.
///
/// The input matching is the same first-fit, one-run-per-input rule
/// [`super::machine::Machine::tick`] uses to actually draw, so a naming
/// this produces is a naming the tick can honour.
fn feasible_naming(
    recipe: &Recipe,
    machine: &Plumbing,
    possible: &[BTreeSet<FluidId>],
) -> Option<Vec<(usize, FluidId)>> {
    let mut naming = Vec::with_capacity(recipe.fluid_inputs.len() + recipe.fluid_outputs.len());
    for (fluid, _) in &recipe.fluid_outputs {
        // No pipe off the outlet face means this recipe cannot run here at
        // all, so it does not get to narrow this machine's input runs
        // either.
        let network = machine.output?;
        if !possible[network].contains(fluid) {
            return None;
        }
        naming.push((network, fluid.clone()));
    }
    let mut claimed: Vec<usize> = Vec::new();
    for (fluid, _) in &recipe.fluid_inputs {
        let supplier = machine
            .inputs
            .iter()
            .copied()
            .find(|network| !claimed.contains(network) && possible[*network].contains(fluid))?;
        claimed.push(supplier);
        naming.push((supplier, fluid.clone()));
    }
    Some(naming)
}

/// Several distinct networks out of one `Vec`, as mutable borrows, in the
/// order asked for.
///
/// Exists so [`World::tick`] can hand a machine every tap it might draw
/// from and the one it deposits into, all at the same time. That the
/// indices are pairwise distinct is an invariant of the topology pass — one
/// entry per network per machine, and a machine never draws from the run it
/// fills (see [`World::recompute_topology`]) — and this asserts it rather
/// than trusting it, because getting it wrong would be aliasing rather than
/// a wrong answer.
fn disjoint_networks_mut<'a>(
    networks: &'a mut [FluidNetwork],
    indices: &[usize],
) -> Vec<&'a mut FluidNetwork> {
    for (position, index) in indices.iter().enumerate() {
        assert!(
            !indices[position + 1..].contains(index),
            "a machine's fluid taps must be pairwise distinct networks — the topology pass keeps \
             one tap per network and excludes the outlet run when choosing the inputs, so {index} \
             appearing twice in {indices:?} means that pass is wrong"
        );
    }
    // Peel the slice in ascending index order — each `split_at_mut` hands
    // back one element and the untouched tail — then put the borrows back
    // into the caller's order.
    let mut ascending: Vec<usize> = (0..indices.len()).collect();
    ascending.sort_by_key(|&slot| indices[slot]);
    let mut borrows: Vec<(usize, &'a mut FluidNetwork)> = Vec::with_capacity(indices.len());
    let mut rest = networks;
    let mut consumed = 0usize;
    for slot in ascending {
        let (_, tail) = rest.split_at_mut(indices[slot] - consumed);
        let (one, tail) = tail.split_at_mut(1);
        consumed = indices[slot] + 1;
        rest = tail;
        borrows.push((slot, &mut one[0]));
    }
    borrows.sort_by_key(|&(slot, _)| slot);
    borrows.into_iter().map(|(_, network)| network).collect()
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

fn pipe_snapshot(network: &FluidNetwork, segment: &PipeSegment) -> PipeSnapshot {
    PipeSnapshot {
        tile: segment.tile,
        direction: segment.direction,
        volume: segment.volume.to_num::<f64>(),
        capacity: segment.capacity.to_num::<f64>(),
        pressure: segment.pressure().to_num::<f64>(),
        is_source: segment.is_source,
        naming: network.naming(),
    }
}

/// Enforces what stays asymmetric about plumbing: a machine **deposits**
/// into exactly one network — the pipe off the face it points at, exactly
/// as it discharges a solid — so a recipe may declare at most one fluid
/// output. A data file asking for two would get the second silently
/// ignored, with the machine simply never finishing and no clue why.
///
/// A recipe may declare **several fluid inputs** (the BBQ epic: the cooking
/// vat draws bbq mix and liquid smoke), because drawing has never been
/// tied to one face — a machine draws from every run touching its
/// footprint, and `Machine::tick` matches each input to a distinct one of
/// them by fluid id. Two inputs naming the *same* fluid are still a content
/// error: they would ask for two runs of the same thing, which is a single
/// larger draw written twice.
///
/// What this *stopped* enforcing, earlier, is the milestone-6 restriction
/// that a whole recipe file could contain at most one fluid-using recipe.
/// That existed because `World` built one hardcoded pipe run; a world now
/// holds one network per connected component of placed pipe, so several
/// fluid recipes are simply several runs the player may or may not have
/// built.
fn validate_fluid_scope(ordered: &[Recipe]) {
    for recipe in ordered {
        let mut named: BTreeSet<&str> = BTreeSet::new();
        for (fluid, _) in &recipe.fluid_inputs {
            assert!(
                named.insert(fluid.as_str()),
                "recipes.ron fluid error: recipe '{}' draws '{fluid}' twice — a machine matches \
                 each fluid input to a different network, so one fluid means one input with the \
                 units added up",
                recipe.id,
            );
        }
        assert!(
            recipe.fluid_outputs.len() <= 1,
            "recipes.ron fluid error: recipe '{}' declares {} fluid outputs, but a machine \
             deposits into exactly one network — the one off its outlet face",
            recipe.id,
            recipe.fluid_outputs.len()
        );
    }
}
