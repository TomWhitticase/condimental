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
//! next-due side, each dock's buffer and lifetime shipped total, the shared
//! delivery pool, every contract's own cadence clock and delivered/missed
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
//! Funding per item rather than pricing the whole bill up front also keeps
//! an arbitrarily large factory loadable: `Fx` saturates around 32 767, so a
//! single up-front bill for a few thousand belt tiles would not fit in the
//! money counter even though every individual price does.
//!
//! # What is deliberately not in the file
//!
//! Anything derived or constant: a machine's recipe (its kind names it),
//! every buffer's capacity (a Rust constant), a pipe network's shape or
//! which fluid it carries or which tile a tank discharges into (all
//! re-derived by `World::recompute_topology` from the placements, which is
//! the same pass a `Place` triggers), and the dirty-chunk set (a freshly
//! loaded world reports **every** chunk dirty, so a chunk-driven renderer
//! redraws all of it).
//!
//! # Forward compatibility
//!
//! Nothing has shipped with a save file yet, so **`schema_version` 1 is
//! whatever this batch of work merges with**. The auto-fill ticket landing
//! alongside this one adds an `auto_fill` flag to hoppers and tanks: the
//! per-kind body records are named-field structs precisely so that is one
//! `#[serde(default)]` field each, and it must **not** bump the schema
//! version. After this batch ships, a field addition is a version bump like
//! any other shape change.

use serde::{Deserialize, Serialize};

use super::belt::BeltSegment;
use super::contracts::ContractRegistry;
use super::dock::Dock;
use super::fluid::FluidTank;
use super::hopper::Hopper;
use super::inventory::Inventory;
use super::machine::{Machine, TimedProcess};
use super::machines::MachineRegistry;
use super::recipe::RecipeRegistry;
use super::splitter::{SPLITTER_KIND, Side, Splitter};
use super::types::{Direction, Fx, ItemId, TileCoord};
use super::world::{BELT_KIND, PIPE_KIND, PlayerCommand, World};
use super::dock::DOCK_KIND;
use super::fluid::FLUID_TANK_KIND;
use super::hopper::HOPPER_KIND;

/// The save format's version. Bumped whenever the shape below changes;
/// a file naming any other version is rejected, never migrated — see the
/// module doc, and every other loader in this crate.
pub const SAVE_SCHEMA_VERSION: u32 = 1;

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

/// One whole saved world. See the module doc for what is here and what is
/// deliberately derived instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveData {
    pub schema_version: u32,
    /// The absolute tick the world was saved on — every
    /// [`TimedProcess::start_tick`] below is measured against it.
    pub tick_count: u64,
    pub money_bits: FxBits,
    /// Everything on the floor, in tile order (the world's own placement
    /// order), so two saves of one world are byte-identical.
    pub placements: Vec<SavedPlacement>,
    /// What the player owns and has **not** placed, by kind — sorted.
    pub owned_equipment: Vec<(String, u32)>,
    /// Bought raw ingredient stock not yet loaded into anything — sorted.
    pub owned_ingredients: Vec<(String, u32)>,
    /// The economy's one shared delivery pool, in its own slot order (see
    /// [`Inventory::slots`]).
    pub delivery_stock: Vec<(ItemId, u32)>,
    /// Per-contract clocks and counters, in the world's contract order.
    /// Only the *state* — a contract's mix, cadence and payout are content
    /// (`data/contracts.ron`) and are matched by id on load.
    pub contracts: Vec<SavedContract>,
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
/// construct (`world.rs`'s `try_place`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedBody {
    Belt(SavedBelt),
    Splitter(SavedSplitter),
    Pipe(SavedPipe),
    Machine(SavedMachine),
    Hopper(SavedHopper),
    FluidTank(SavedTank),
    Dock(SavedDock),
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
        }
    }
}

/// A belt tile's contents: each item and how far along the tile it is, front
/// to back, plus the scan boundary — see
/// [`BeltSegment::restore`](super::belt::BeltSegment) for why the boundary is
/// state and not a cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedBelt {
    pub items: Vec<(ItemId, FxBits)>,
    pub frontier: u32,
}

/// A splitter carries a belt tile's worth of product plus the one thing a
/// belt has no equivalent of: which output is due next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSplitter {
    pub belt: SavedBelt,
    pub next_output: Side,
}

/// A pipe tile holds only a volume. Which network it belongs to, what that
/// network carries and whether a tank discharges into it are all derived on
/// load (this module's doc).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPipe {
    pub volume_bits: FxBits,
}

/// A machine's two buffers and its in-flight craft. The recipe is not here:
/// a placed machine runs the one recipe its *kind* names
/// (`world.rs`'s `recipe_for_kind`), so saving it would be saving content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedMachine {
    pub input: Vec<(ItemId, u32)>,
    pub output: Vec<(ItemId, u32)>,
    /// `(start_tick, required_ticks)` of the batch currently running, if
    /// any — absolute ticks, so it survives against the restored
    /// [`SaveData::tick_count`] exactly.
    pub process: Option<(u64, u64)>,
}

/// A hopper's chosen contents and how much of them it holds. `item` is what
/// the placement is made *with* (a hopper holds one item for its whole
/// life), not something set afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedHopper {
    pub item: ItemId,
    pub quantity: u32,
}

/// A tank's exact volume. Fractional in general — a discharging tank holds a
/// volume no whole-unit `fill` could produce — hence bits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedTank {
    pub volume_bits: FxBits,
}

/// A dock's buffer (normally empty — it is a pass-through, see
/// [`super::dock`]) and its lifetime shipped total, which is the number
/// worth keeping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedDock {
    pub contents: Vec<(ItemId, u32)>,
    pub shipped_total: u32,
}

/// One contract's live state, matched to content by `id` on load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedContract {
    pub id: String,
    pub next_delivery_tick: u64,
    pub deliveries_made: u32,
    pub deliveries_missed: u32,
}

/// Why a save could not be loaded. Every variant names the thing that is
/// wrong and where, in the style every other loader in this crate uses —
/// and every one of them leaves the caller's existing world untouched,
/// because a load builds a *new* world and only hands it back on success.
#[derive(Debug)]
pub enum SaveError {
    Parse(ron::error::SpannedError),
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
    /// A contract id this build's `data/contracts.ron` does not have.
    UnknownContract {
        id: String,
    },
    /// A contract this build has that the save says nothing about. Both
    /// directions are errors for one reason: a contract board that has
    /// changed under a save means its economy state cannot be faithfully
    /// restored, and quietly starting the new contract from scratch would
    /// hide that.
    MissingContract {
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
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Parse(e) => write!(f, "failed to parse the save file: {e}"),
            SaveError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "save schema_version {found} is not supported by this build (supports \
                 {supported}) — this save was written by a different version of the game"
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
            SaveError::UnknownEquipmentKind { kind } => write!(
                f,
                "the save says the player owns '{kind}', which data/shop.ron does not price"
            ),
            SaveError::UnknownIngredient { id } => write!(
                f,
                "the save says the player owns '{id}', which data/ingredients.ron does not \
                 price"
            ),
            SaveError::UnknownContract { id } => write!(
                f,
                "the save carries state for contract '{id}', which data/contracts.ron does \
                 not define in this build"
            ),
            SaveError::MissingContract { id } => write!(
                f,
                "the save carries no state for contract '{id}', which this build's \
                 data/contracts.ron defines — the contract board changed under this save"
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
        // message is far more useful, so it is checked against a shape that
        // only has the one field in it.
        if let Ok(header) = ron::from_str::<Header>(text) {
            if header.schema_version != SAVE_SCHEMA_VERSION {
                return Err(SaveError::UnsupportedSchemaVersion {
                    found: header.schema_version,
                    supported: SAVE_SCHEMA_VERSION,
                });
            }
        }
        let data: SaveData = ron::from_str(text).map_err(SaveError::Parse)?;
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
                SPLITTER_KIND => SavedBody::Splitter(SavedSplitter {
                    belt: saved_belt(self, entity),
                    next_output: self
                        .ecs()
                        .get::<&Splitter>(entity)
                        .expect("a splitter placement carries a Splitter")
                        .due(),
                }),
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
                    })
                }
                FLUID_TANK_KIND => SavedBody::FluidTank(SavedTank {
                    volume_bits: FxBits::of(
                        self.ecs()
                            .get::<&FluidTank>(entity)
                            .expect("a tank placement carries a FluidTank")
                            .volume(),
                    ),
                }),
                DOCK_KIND => {
                    let dock = self
                        .ecs()
                        .get::<&Dock>(entity)
                        .expect("a dock placement carries a Dock");
                    SavedBody::Dock(SavedDock {
                        contents: dock.slots().to_vec(),
                        shipped_total: dock.shipped_total(),
                    })
                }
                _ => {
                    let machine = self
                        .ecs()
                        .get::<&Machine>(entity)
                        .expect("a machine placement carries a Machine");
                    SavedBody::Machine(SavedMachine {
                        input: machine.input.slots().to_vec(),
                        output: machine.output.slots().to_vec(),
                        process: machine
                            .process
                            .map(|p| (p.start_tick, p.required_ticks)),
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
            money_bits: FxBits::of(self.money()),
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
            delivery_stock: self.economy().delivery_buffer.slots().to_vec(),
            contracts: self
                .economy()
                .contracts
                .iter()
                .map(|active| SavedContract {
                    id: active.contract.id.clone(),
                    next_delivery_tick: active.next_delivery_tick,
                    deliveries_made: active.deliveries_made,
                    deliveries_missed: active.deliveries_missed,
                })
                .collect(),
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
        let mut world =
            World::with_content_and_money(recipes, machines, contracts, Fx::ZERO);

        validate(&world, &data)?;
        rebuild_structure(&mut world, &data)?;
        restore_bodies(&mut world, &data)?;
        restore_ledgers(&mut world, &data)?;

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
            SavedBody::Machine(machine) => {
                for (item, _) in machine.input.iter().chain(machine.output.iter()) {
                    check_item(item, &known_items, || {
                        format!("in the machine buffer at {:?}", placement.tile)
                    })?;
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
                if !world.raw_items().iter().any(|raw| *raw == hopper.item) {
                    return Err(SaveError::UnknownHopperItem {
                        item: hopper.item.clone(),
                        tile: placement.tile,
                        raw: world.raw_items().to_vec(),
                    });
                }
            }
            SavedBody::Pipe(_) | SavedBody::FluidTank(_) => {}
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
    for (item, _) in &data.delivery_stock {
        check_item(item, &known_items, || {
            "in the shared delivery pool".to_string()
        })?;
    }

    // The contract board has to match this build's, both ways — see
    // `SaveError::MissingContract` for why one direction is not enough.
    for saved in &data.contracts {
        if world.economy().contract(&saved.id).is_none() {
            return Err(SaveError::UnknownContract {
                id: saved.id.clone(),
            });
        }
    }
    for active in &world.economy().contracts {
        if !data.contracts.iter().any(|c| c.id == active.contract.id) {
            return Err(SaveError::MissingContract {
                id: active.contract.id.clone(),
            });
        }
    }
    Ok(())
}

/// Which body a kind is built with (`world.rs`'s `try_place`) — the one
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
        _ => "machine",
    }
}

fn check_belt_items(
    belt: &SavedBelt,
    known: &[String],
    tile: TileCoord,
) -> Result<(), SaveError> {
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
/// factory loads even though its total bill would not fit in `Fx`.
fn rebuild_structure(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
    for (index, placement) in data.placements.iter().enumerate() {
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

/// Half two: write the live contents onto the placements half one built.
fn restore_bodies(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
    for placement in &data.placements {
        let entity = world
            .entity_at(placement.tile)
            .expect("rebuild_structure placed something on every saved tile");
        match &placement.body {
            SavedBody::Belt(belt) => restore_belt(world, entity, belt),
            SavedBody::Splitter(splitter) => {
                restore_belt(world, entity, &splitter.belt);
                world
                    .ecs_mut()
                    .insert_one(entity, Splitter::restore(splitter.next_output))
                    .expect("the entity was just placed");
            }
            SavedBody::Pipe(pipe) => {
                let (_, capacity) = world
                    .pipe_volume_at(placement.tile)
                    .expect("a placed pipe is always in the pipe index");
                let volume = pipe.volume_bits.value();
                if volume > capacity || volume < Fx::ZERO {
                    return Err(SaveError::OverCapacity {
                        what: "pipe".to_string(),
                        tile: placement.tile,
                        held: volume.to_string(),
                        capacity: capacity.to_string(),
                    });
                }
                world.restore_pipe_volume(placement.tile, volume);
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
                check_capacity(&saved.input, input_capacity, "machine input", placement.tile)?;
                check_capacity(
                    &saved.output,
                    output_capacity,
                    "machine output",
                    placement.tile,
                )?;
                let mut machine = world
                    .ecs_mut()
                    .get::<&mut Machine>(entity)
                    .expect("a machine placement carries a Machine");
                machine.input = Inventory::restore(input_capacity, saved.input.clone());
                machine.output = Inventory::restore(output_capacity, saved.output.clone());
                machine.process = saved.process.map(|(start_tick, required_ticks)| {
                    TimedProcess {
                        start_tick,
                        required_ticks,
                    }
                });
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
                    .insert_one(entity, FluidTank::restore(capacity, volume))
                    .expect("the entity was just placed");
            }
            SavedBody::Dock(saved) => {
                let capacity = world
                    .ecs()
                    .get::<&Dock>(entity)
                    .expect("a dock placement carries a Dock")
                    .capacity_per_item();
                check_capacity(&saved.contents, capacity, "dock", placement.tile)?;
                world
                    .ecs_mut()
                    .insert_one(
                        entity,
                        Dock::restore(capacity, saved.contents.clone(), saved.shipped_total),
                    )
                    .expect("the entity was just placed");
            }
        }
    }
    Ok(())
}

fn restore_belt(world: &mut World, entity: hecs::Entity, saved: &SavedBelt) {
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
            BeltSegment::restore(length, min_spacing, &positions, saved.frontier as usize),
        )
        .expect("the entity was just placed");
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

/// The world-level ledgers: unplaced equipment, bought ingredient stock, the
/// shared delivery pool, and every contract's own clock.
///
/// Owned equipment is credited **after** the rebuild, not before: half one
/// buys exactly one of each thing it places and spends it immediately, so
/// the ledger is empty again by the time this runs and the saved counts are
/// the player's genuine unplaced inventory.
fn restore_ledgers(world: &mut World, data: &SaveData) -> Result<(), SaveError> {
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

    let delivery_stock = data.delivery_stock.clone();
    let contracts = data.contracts.clone();
    let economy = world.economy_mut();
    economy.delivery_buffer = Inventory::restore(u32::MAX, delivery_stock);
    for saved in &contracts {
        let active = economy
            .contracts
            .iter_mut()
            .find(|active| active.contract.id == saved.id)
            .expect("validate() proved every saved contract id exists");
        active.next_delivery_tick = saved.next_delivery_tick;
        active.deliveries_made = saved.deliveries_made;
        active.deliveries_missed = saved.deliveries_missed;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let text = a_save().to_ron().replace("schema_version: 1", "schema_version: 99");
        match SaveData::from_ron(&text) {
            Err(SaveError::UnsupportedSchemaVersion { found, supported }) => {
                assert_eq!((found, supported), (99, SAVE_SCHEMA_VERSION));
            }
            other => panic!("expected a version error, got {other:?}"),
        }
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
        });
        let message = refusal(data);
        assert!(message.contains("9999"), "{message}");
        assert!(message.contains("capacity"), "{message}");
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
    fn a_contract_board_that_has_changed_under_the_save_is_refused_both_ways() {
        let mut data = a_save();
        data.contracts[0].id = "midnight_kebab_van".to_string();
        let message = refusal(data);
        assert!(message.contains("midnight_kebab_van"), "{message}");

        let mut data = a_save();
        let dropped = data.contracts.remove(0);
        let message = refusal(data);
        assert!(message.contains(&dropped.id), "{message}");
        assert!(message.contains("carries no state"), "{message}");
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
