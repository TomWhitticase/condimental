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
//! - **Fluids connect by touching.** A fluid-consuming machine taps any pipe
//!   tile orthogonally adjacent to any tile of its footprint, whatever
//!   either is facing, and pipe-to-pipe flow is undirected (see
//!   [`super::fluid`]'s module doc). That is not a shortcut: the shipped
//!   mixer has always been fed from the row *above* it — neither its inlet
//!   face nor its outlet — because a pressure network is not a conveyor, and
//!   requiring a fluid inlet to face the right way would break the one
//!   plumbing arrangement the game has always had.
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
//! the tile a belt points at. What is *not* done here is the other half
//! that note named: rotated voxel models. Ticket 2 (presentation) owns that.
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
use super::economy::{Economy, EconomySnapshot, STARTING_MONEY};
use super::fluid::{
    FLUID_TANK_CAPACITY, FLUID_TANK_FILL_AMOUNT, FLUID_TANK_KIND, FluidNetwork, FluidTank,
    FluidTap, PipeSegment,
};
use super::hopper::{HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, HOPPER_KIND, Hopper};
use super::machine::Machine;
use super::machines::{Footprint, MachineRegistry};
use super::recipe::{Recipe, RecipeRegistry};
use super::shop::{DEFAULT_PURCHASE_QUANTITY, Shop, ShopSnapshot};
use super::splitter::{SPLITTER_KIND, Side, Splitter};
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
    /// Loads raw ingredient into a hopper. The *item* is not a parameter: a
    /// hopper holds exactly one item id, decided by `data/recipes.ron` (see
    /// [`super::hopper`]), so naming an item here could only ever name the
    /// wrong one.
    ///
    /// `tile` names **which** hopper (any tile of its footprint, like every
    /// other tile-addressed command here). `None` means *every* hopper in
    /// the world, each clamped at its own capacity — which is what a world
    /// with one hopper has always done on a keypress, and which is why the
    /// pre-placement wire format still works unchanged. A world with no
    /// hopper at all, or a `tile` holding something else, is a no-op.
    FillHopper {
        amount: u32,
        tile: Option<TileCoord>,
    },
    /// Loads fluid into a tank, in whole fluid units. Exactly
    /// [`PlayerCommand::FillHopper`]'s shape for exactly its reasons,
    /// including `tile`: the *fluid* is not a parameter (a network carries
    /// the fluid its tapping machine's recipe names), and over-filling is
    /// clamped at the tank's capacity, silently.
    FillFluidTank {
        amount: u32,
        tile: Option<TileCoord>,
    },
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
    Place {
        kind: String,
        tile: TileCoord,
        direction: Direction,
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
    /// `{"type": "place", "kind": "crusher", "tile": [4, 0],
    /// "direction": "east"}` — `direction` optional, defaulting to `"east"`.
    Place {
        kind: String,
        tile: TileCoord,
        #[serde(default)]
        direction: Direction,
    },
    /// `{"type": "remove", "tile": [4, 0]}`.
    Remove { tile: TileCoord },
    /// `{"type": "rotate", "tile": [4, 0]}`.
    Rotate { tile: TileCoord },
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
            PlayerCommandDto::Place {
                kind,
                tile,
                direction,
            } => PlayerCommand::Place {
                kind,
                tile,
                direction,
            },
            PlayerCommandDto::Remove { tile } => PlayerCommand::Remove { tile },
            PlayerCommandDto::Rotate { tile } => PlayerCommand::Rotate { tile },
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
///   "fill": 0.25, "pipe_tile": [6, 1]
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
}

/// One machine's tap into a fluid network, for presentation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FluidTapSnapshot {
    /// Origin tile of the machine drawing from this network.
    pub machine_tile: TileCoord,
    pub machine_recipe: RecipeId,
    /// How much the recipe consumes per craft.
    pub units_per_craft: u32,
    /// Index into the network's `segments` of the tile this machine draws
    /// from — the one whose level drops when a craft starts.
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
    /// `{"kind": "belt", "direction": "east", "items_here": ["tomato"]}` —
    /// one tile, so `items_here` is simply everything on it.
    Belt {
        direction: Direction,
        items_here: Vec<ItemId>,
    },
    /// `{"kind": "splitter", "direction": "east",
    ///   "items_here": ["tomato"], "outputs": [[7, 0], [6, -1]],
    ///   "next_output": "straight"}` — enough for a hover tooltip
    /// ("Splitter — next: straight"). `outputs` and `next_output` mean
    /// exactly what they do in [`SplitterSnapshot`].
    Splitter {
        direction: Direction,
        items_here: Vec<ItemId>,
        outputs: [Option<TileCoord>; 2],
        next_output: Side,
    },
    /// `{"kind": "machine", "machine_kind": "cooking_vat",
    ///   "direction": "east",
    ///   "footprint": {"width_tiles": 2, "height_tiles": 2},
    ///   "origin_tile": [14, 0], "recipe": "ketchup",
    ///   "process": [120, 240]}`
    ///
    /// `machine_kind` is spelled out rather than reusing `kind`, which this
    /// enum already spends on its own variant tag.
    Machine {
        machine_kind: String,
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        recipe: RecipeId,
        process: Option<(u64, u64)>,
    },
    /// `{"kind": "pipe", "direction": "east", "fluid": "vinegar",
    ///   "volume": 7.5, "capacity": 10.0, "pressure": 0.75,
    ///   "is_source": false, "is_inlet": true}` — enough for a hover tooltip
    /// without a second call. `is_inlet` is whether some machine taps this
    /// exact tile.
    Pipe {
        direction: Direction,
        fluid: Option<FluidId>,
        volume: f64,
        capacity: f64,
        pressure: f64,
        is_source: bool,
        is_inlet: bool,
    },
    /// `{"kind": "hopper", "direction": "east",
    ///   "footprint": {...}, "origin_tile": [-1, 0], "item": "tomato",
    ///   "quantity": 12, "capacity": 64, "fill": 0.1875}` — enough for a
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
    },
    /// `{"kind": "fluid_tank", "direction": "east", "footprint": {...},
    ///   "origin_tile": [5, 1], "fluid": "vinegar", "quantity": 30.0,
    ///   "capacity": 120.0, "fill": 0.25}`. `fluid` is nullable for
    /// [`FluidTankSnapshot`]'s reason.
    FluidTank {
        direction: Direction,
        footprint: Footprint,
        origin_tile: TileCoord,
        fluid: Option<FluidId>,
        quantity: f64,
        capacity: f64,
        fill: f64,
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
    },
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
/// Unlike `chain_layout`, **this is not static for the life of the world** —
/// it changes whenever the player places, rotates or removes something. Call
/// it again after issuing any of those three commands (and after a
/// `dirty_chunks` report, if the renderer is chunk-driven).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlacementSnapshot {
    pub kind: String,
    pub tile: TileCoord,
    pub direction: Direction,
    pub footprint: Footprint,
    pub occupies: Vec<TileCoord>,
    pub recipe: Option<RecipeId>,
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
    /// machine that touches a pipe.
    machine_taps: Vec<(Entity, usize, usize)>,
    /// `(tank, network index, segment index)` for every tank discharging
    /// into a pipe.
    tank_discharges: Vec<(Entity, usize, usize)>,
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
    /// The chain's raw ingredient — the first recipe's first input — which
    /// is what any hopper the player places will hold. `None` for a recipe
    /// file whose first recipe has no inputs, in which case a hopper cannot
    /// be placed at all (rather than the world refusing to be built).
    raw_item: Option<ItemId>,
    tick_count: u64,
    belt_speed: Fx,
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
    topology: Topology,
}

/// How far an item advances along a belt tile per tick. Unchanged from the
/// pre-placement chain — a tile is one unit long, so an item still crosses
/// one tile in 50 ticks and a four-tile run in 200, exactly as it did when
/// that run was a single four-tile `BeltSegment`.
const BELT_SPEED_TILES_PER_TICK: f64 = 0.02;
/// One placed belt tile is one unit of belt. (`belt.rs` requires
/// `speed <= min_spacing`, which the two constants above and below satisfy.)
const BELT_TILE_LENGTH: f64 = 1.0;
const ITEM_MIN_SPACING_TILES: f64 = 0.25;
const MACHINE_INPUT_CAPACITY: u32 = 8;
const MACHINE_OUTPUT_CAPACITY: u32 = 8;
/// Volume one pipe tile holds, in whole fluid units.
const PIPE_SEGMENT_CAPACITY: i32 = 10;

impl World {
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
            Fx::from_num(STARTING_MONEY),
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
        let raw_item = ordered[0].inputs.first().map(|(item, _)| item.clone());

        Self {
            ecs: hecs::World::new(),
            recipes,
            machines,
            recipe_for_kind,
            raw_item,
            tick_count: 0,
            belt_speed: Fx::from_num(BELT_SPEED_TILES_PER_TICK),
            dirty: Vec::new(),
            economy: Economy::with_money(&contracts, money),
            shop: Shop::bundled(),
            topology: Topology::default(),
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

    /// The shop catalog, the player's owned counts and their current money
    /// in one object — see [`ShopSnapshot`] for the exact JSON shape.
    pub fn shop_snapshot(&self) -> ShopSnapshot {
        self.shop.snapshot(self.economy.money)
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
            HOPPER_KIND => self.raw_item.is_some(),
            other => self.recipe_for_kind.contains_key(other),
        }
    }

    /// Places one owned unit of `kind`, returning whether it happened. See
    /// [`PlayerCommand::Place`] for the refusal rules — every one of them
    /// leaves the world byte-identical.
    fn try_place(&mut self, kind: &str, tile: TileCoord, direction: Direction) -> bool {
        if self.shop.owned.count(kind) == 0 || !self.can_construct(kind) {
            return false;
        }
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
                let item = self
                    .raw_item
                    .clone()
                    .expect("can_construct checked the chain has a raw ingredient");
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
        for &belt in &topology.belts {
            let placement = self.placement_of(belt);
            let target = placement.direction.step(self.position_of(belt));
            let Some(occupant) = resolve_sink(&topology.grid, target) else {
                continue;
            };
            if is_belt_like(body_of(occupant)) {
                topology.belt_to_belt.push((belt, occupant));
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
                let target = side.direction_from(placement.direction).step(origin);
                resolve_sink(&topology.grid, target)
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
                (self.position_of(entity), (entity, capacity, placement.direction))
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
            let mut links = Vec::new();
            for (i, &tile) in component.iter().enumerate() {
                for dir in [Direction::East, Direction::North] {
                    if let Some(&j) = index_of.get(&dir.step(tile)) {
                        links.push((i, j));
                    }
                }
            }
            let network_index = topology.networks.len();
            for (segment_index, &tile) in component.iter().enumerate() {
                topology
                    .pipe_index
                    .insert(tile, (network_index, segment_index));
            }
            topology.networks.push(FluidNetwork::new(None, segments, links));
        }

        // --- who taps what. A machine touching a pipe taps it (fluids
        // connect by touching, not by facing — this module's doc), and the
        // first fluid-consuming machine to reach a network is what decides
        // the fluid that network carries.
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
            let Some((fluid_id, _)) = recipe.fluid_inputs.first().cloned() else {
                continue;
            };
            let tap = placement
                .tiles(origin)
                .into_iter()
                .flat_map(|tile| Direction::ALL.map(|dir| dir.step(tile)))
                .find_map(|neighbour| topology.pipe_index.get(&neighbour).copied());
            let Some((network, segment)) = tap else {
                continue;
            };
            if topology.networks[network].fluid.is_none() {
                topology.networks[network].fluid = Some(fluid_id);
            }
            topology.machine_taps.push((machine, network, segment));
        }
        for &tank in &topology.tanks {
            let placement = self.placement_of(tank);
            let origin = self.position_of(tank);
            let discharge = placement
                .footprint
                .outlet_neighbours(origin, placement.direction)
                .into_iter()
                .find_map(|tile| topology.pipe_index.get(&tile).copied());
            if let Some((network, segment)) = discharge {
                topology.networks[network].mark_source(segment);
                topology.tank_discharges.push((tank, network, segment));
            }
        }

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
    /// 3. **Machines run** — start a craft, or deposit a finished one.
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
    ///    contracts settle against below.
    /// 7. **Contracts settle**, one cadence check per active contract.
    ///
    /// Steps 4 and 5 are the same two phases the fixed chain always had
    /// ("belt to machine, then everything onto a belt"); what changed is
    /// that the pairs come from adjacency rather than from a stage index.
    pub fn tick(&mut self) {
        self.dirty.clear();
        self.tick_count += 1;

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

        // 2. Belts — and splitters, which carry their contents in an
        // ordinary belt segment (`splitter.rs`) so that product crosses one
        // in exactly the time it would cross the belt tile it replaced.
        let belts = std::mem::take(&mut self.topology.belts);
        let splitters = std::mem::take(&mut self.topology.splitters);
        for &carrier in belts.iter().chain(splitters.iter()) {
            let moved = {
                let mut segment = self
                    .ecs
                    .get::<&mut BeltSegment>(carrier)
                    .expect("a belt or splitter placement carries a BeltSegment");
                segment.tick(self.belt_speed)
            };
            if moved {
                let tile = self.position_of(carrier);
                self.mark_dirty(tile);
            }
        }
        self.topology.splitters = splitters;

        // 3. Machines.
        let machines = std::mem::take(&mut self.topology.machines);
        let taps = std::mem::take(&mut self.topology.machine_taps);
        for &machine in &machines {
            let tapped = taps
                .iter()
                .find(|&&(entity, _, _)| entity == machine)
                .map(|&(_, network, inlet)| (network, inlet));
            let changed = {
                let mut body = self
                    .ecs
                    .get::<&mut Machine>(machine)
                    .expect("a machine placement carries a Machine");
                let tap = tapped.map(|(network, inlet)| FluidTap {
                    network: &mut self.topology.networks[network],
                    inlet,
                });
                body.tick(self.tick_count, &self.recipes, tap)
            };
            if changed {
                self.mark_placement_dirty(machine);
            }
        }
        self.topology.machine_taps = taps;

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
            let shipped = {
                let mut body = self
                    .ecs
                    .get::<&mut Dock>(dock)
                    .expect("a dock placement carries a Dock");
                body.ship_into(&mut self.economy)
            };
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
        // always-fresh snapshot instead.
        self.economy.tick(self.tick_count);
    }

    fn body_kind(&self, entity: Entity) -> Option<BodyKind> {
        self.ecs.get::<&BodyKind>(entity).ok().map(|body| *body)
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
                    // Clamped at capacity, never an error — see `Hopper::fill`.
                    let added = {
                        let mut body = self.ecs.get::<&mut Hopper>(hopper).unwrap();
                        body.fill(amount)
                    };
                    if added > 0 {
                        self.mark_placement_dirty(hopper);
                    }
                }
            }
            PlayerCommand::FillFluidTank { amount, tile } => {
                let tanks = self.targets(&self.topology.tanks.clone(), tile);
                for tank in tanks {
                    // Clamped at capacity, never an error — see `FluidTank::fill`.
                    let added = {
                        let mut body = self.ecs.get::<&mut FluidTank>(tank).unwrap();
                        body.fill(amount)
                    };
                    if added > Fx::ZERO {
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
            PlayerCommand::Place {
                kind,
                tile,
                direction,
            } => {
                self.try_place(&kind, tile, direction);
            }
            PlayerCommand::Remove { tile } => {
                self.try_remove(tile);
            }
            PlayerCommand::Rotate { tile } => {
                self.try_rotate(tile);
            }
        }
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
                snapshot
                    .pipes
                    .push(pipe_snapshot(&self.topology.networks[network].segments()[segment]));
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
                    taps: self
                        .topology
                        .machine_taps
                        .iter()
                        .filter(|&&(_, net, _)| net == index)
                        .filter_map(|&(machine, _, segment)| {
                            let body = self.ecs.get::<&Machine>(machine).ok()?;
                            let units_per_craft = self
                                .recipes
                                .get(&body.recipe)
                                .and_then(|r| r.fluid_inputs.first().map(|(_, units)| *units))
                                .unwrap_or(0);
                            Some(FluidTapSnapshot {
                                machine_tile: self.position_of(machine),
                                machine_recipe: body.recipe.clone(),
                                units_per_craft,
                                segment_index: segment,
                            })
                        })
                        .collect(),
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
                }
            }
            BodyKind::Machine => {
                let body = self.ecs.get::<&Machine>(entity).unwrap();
                TileSnapshot::Machine {
                    machine_kind: placement.kind,
                    direction: placement.direction,
                    footprint: placement.footprint,
                    origin_tile: origin,
                    recipe: body.recipe.clone(),
                    process: body.process.map(|p| (p.start_tick, p.required_ticks)),
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
                }
            }
        };
        Some(snapshot)
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
        let links = next
            .inputs
            .iter()
            .any(|(item, _)| prev.outputs.iter().any(|(out_item, _)| out_item == item));
        assert!(
            links,
            "recipes.ron chain error: recipe '{}' (output {:?}) does not feed recipe '{}' \
             (inputs {:?}) — the file describes one production line, so consecutive recipes \
             must chain by a shared item id",
            prev.id, prev.outputs, next.id, next.inputs
        );
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
             exactly one network",
            recipe.id,
            recipe.fluid_inputs.len()
        );
    }
}
