//! The simulation `World`: an `hecs::World` (ARCHITECTURE.md §1/§3.8) tying
//! together belt/machine/recipe content into milestone 4's "first full
//! chain" (ARCHITECTURE.md §9 item 4) — a straight-line production line of
//! `N` `(BeltSegment -> Machine)` stages, one per recipe in `recipes.ron`.
//!
//! **Chain topology is derived entirely from `recipes.ron`'s list order**
//! (`RecipeRegistry::ordered`), not from any Rust-side layout description:
//! stage `i` runs `ordered()[i]`, its belt feeds its machine, and its
//! machine's finished output is handed to stage `i + 1`'s belt. This is what
//! makes "add a processing step" a pure data-file edit (ARCHITECTURE.md
//! §3.6/§10, CLAUDE.md's project convention) — appending a `Recipe` to the
//! file whose input item matches the previous entry's output item adds a
//! pipeline stage with zero Rust changes. `validate_chain` below enforces
//! that linkage at construction time (a clear panic, not a silently-stuck
//! item) exactly the way a compile error would for a Rust mistake.
//!
//! This is a deliberately narrow milestone-4 assumption, not a general
//! factory-layout engine: every recipe in the file is assumed to belong to
//! *one* straight chain, and each recipe (besides the first) is assumed to
//! consume exactly one unit of the previous stage's single output item.
//! Branching/multiple independent chains will need an explicit topology
//! description in data (e.g. a future `machines.ron` placement list) rather
//! than "one stage per recipe in file order" — flagged here rather than
//! solved now, since nothing before milestone 8/9 (Scale-up, Logistics
//! expansion) needs more than one line (CLAUDE.md: "don't design for
//! hypothetical future requirements").
//!
//! This is also where `SimBridge`'s pure-logic half lives (ARCHITECTURE.md
//! §4): `tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
//! `query_tile()`, `chain_layout()`. The `gdext`-facing wrapper in `lib.rs`
//! is a thin translation layer on top of this — nothing here depends on
//! Godot, so it is fully exercisable from headless `cargo test`
//! (ARCHITECTURE.md §7).

use hecs::Entity;

use super::belt::BeltSegment;
use super::machine::Machine;
use super::recipe::{Recipe, RecipeRegistry};
use super::types::{ChunkId, Fx, ItemId, RecipeId, TileCoord, chunk_of};

/// Where an entity sits in the tile grid (ARCHITECTURE.md §3.1/§3.2).
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub tile: TileCoord,
}

/// How many tiles a belt spans, laid out horizontally starting at its
/// entity's `Position`. Each stage still gets one straight segment;
/// corners/branches are a later milestone's concern.
#[derive(Debug, Clone, Copy)]
pub struct BeltSpan {
    pub length_tiles: i32,
}

/// A command the player can issue (ARCHITECTURE.md §4's `PlayerCommand`).
/// Kept intentionally small — the chain's single entry point (spawning raw
/// ingredient onto the first belt) and single exit point (collecting
/// finished output from the last machine), not a full build-mode yet.
#[derive(Debug, Clone)]
pub enum PlayerCommand {
    SpawnItem { item: ItemId },
    CollectOutput,
}

/// JSON wire shape for `PlayerCommand`, used only at the `gdext` boundary
/// (`lib.rs`) — GDScript builds a `{"type": "spawn_item", "item": "tomato"}`
/// dictionary, converts it to a JSON string, and this is what parses it.
/// Kept separate from `PlayerCommand` itself so the pure sim type isn't
/// coupled to a wire format.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum PlayerCommandDto {
    SpawnItem { item: ItemId },
    CollectOutput,
}

impl From<PlayerCommandDto> for PlayerCommand {
    fn from(dto: PlayerCommandDto) -> Self {
        match dto {
            PlayerCommandDto::SpawnItem { item } => PlayerCommand::SpawnItem { item },
            PlayerCommandDto::CollectOutput => PlayerCommand::CollectOutput,
        }
    }
}

/// Parses a JSON-encoded `PlayerCommand` (see `PlayerCommandDto`). Returns
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
/// Plural since milestone 4: a chunk can hold more than one stage's belt or
/// machine (32x32-tile chunks, ARCHITECTURE.md §3.1, comfortably fit several
/// 5-tile stages), so this is a `Vec` of whichever stages' entities fall in
/// the requested chunk, not a single optional pair like milestone 3's
/// fixed one-belt-one-machine layout.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChunkSnapshot {
    pub belts: Vec<BeltSnapshot>,
    pub machines: Vec<MachineSnapshot>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BeltSnapshot {
    pub start_tile: TileCoord,
    pub length_tiles: i32,
    /// (item id, distance from belt start in tiles). `f64` is fine here —
    /// this has already crossed from fixed-point simulation state into a
    /// presentation-only value that never feeds back in (ARCHITECTURE.md
    /// §3.9).
    pub items: Vec<(ItemId, f64)>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MachineSnapshot {
    pub tile: TileCoord,
    pub recipe: RecipeId,
    /// (start_tick, required_ticks) — Godot derives progress from this plus
    /// the current tick count rather than needing a snapshot every tick.
    pub process: Option<(u64, u64)>,
    pub input: Vec<(ItemId, u32)>,
    pub output: Vec<(ItemId, u32)>,
}

/// What occupies a single tile, for `query_tile`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TileSnapshot {
    Belt {
        items_here: Vec<ItemId>,
    },
    Machine {
        recipe: RecipeId,
        process: Option<(u64, u64)>,
    },
}

/// Static (non-per-tick) description of one pipeline stage's placement —
/// enough for Godot to build its `TileMap` generically from however many
/// stages `recipes.ron` currently defines, instead of hardcoding stage
/// geometry client-side (mirrors the same "Godot draws what the sim says
/// exists" principle `chunk_snapshot` already follows for dynamic state).
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageLayout {
    pub recipe: RecipeId,
    pub belt_start_tile: TileCoord,
    pub belt_length_tiles: i32,
    pub machine_tile: TileCoord,
}

#[derive(Debug, Clone, Copy)]
struct Stage {
    belt_entity: Entity,
    machine_entity: Entity,
}

pub struct World {
    ecs: hecs::World,
    recipes: RecipeRegistry,
    tick_count: u64,
    stages: Vec<Stage>,
    belt_speed: Fx,
    /// Chunks touched since the start of the current `tick()` call. Cleared
    /// (not reallocated) at the start of every `tick()` — see `tick`'s doc
    /// comment and ARCHITECTURE.md §4's "Delta granularity" note.
    dirty: Vec<ChunkId>,
}

const BELT_LENGTH_TILES: i32 = 4;
/// Tile x-offset between one stage's belt start and the next — belt tiles,
/// then one tile for the machine, then the next stage begins immediately
/// after (matches milestone 3's original single-stage numbers exactly when
/// there's only one stage).
const STAGE_SPAN_TILES: i32 = BELT_LENGTH_TILES + 1;
const ITEM_MIN_SPACING_TILES: f64 = 0.25;
const BELT_SPEED_TILES_PER_TICK: f64 = 0.02;
const MACHINE_INPUT_CAPACITY: u32 = 8;
const MACHINE_OUTPUT_CAPACITY: u32 = 8;

impl World {
    /// Builds one stage per recipe in `recipes.ordered()`, in file order,
    /// laid out left to right — see the module doc for why this order (not
    /// any Rust-side description) is what defines the chain.
    pub fn new(recipes: RecipeRegistry) -> Self {
        let ordered = recipes.ordered();
        assert!(
            !ordered.is_empty(),
            "recipes.ron must define at least one recipe for World::new to build a chain from"
        );
        validate_chain(ordered);

        let mut ecs = hecs::World::new();
        let mut stages = Vec::with_capacity(ordered.len());

        for (i, recipe) in ordered.iter().enumerate() {
            let stage_start_x = i as i32 * STAGE_SPAN_TILES;

            let belt_entity = ecs.spawn((
                Position {
                    tile: (stage_start_x, 0),
                },
                BeltSpan {
                    length_tiles: BELT_LENGTH_TILES,
                },
                BeltSegment::new(
                    Fx::from_num(BELT_LENGTH_TILES),
                    Fx::from_num(ITEM_MIN_SPACING_TILES),
                ),
            ));

            let machine_entity = ecs.spawn((
                Position {
                    tile: (stage_start_x + BELT_LENGTH_TILES, 0),
                },
                Machine::new(recipe.id.clone(), MACHINE_INPUT_CAPACITY, MACHINE_OUTPUT_CAPACITY),
            ));

            stages.push(Stage {
                belt_entity,
                machine_entity,
            });
        }

        Self {
            ecs,
            recipes,
            tick_count: 0,
            stages,
            belt_speed: Fx::from_num(BELT_SPEED_TILES_PER_TICK),
            dirty: Vec::new(),
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }

    fn position_of(&self, entity: Entity) -> TileCoord {
        self.ecs.get::<&Position>(entity).unwrap().tile
    }

    fn mark_dirty(&mut self, tile: TileCoord) {
        let chunk = chunk_of(tile);
        if !self.dirty.contains(&chunk) {
            self.dirty.push(chunk);
        }
    }

    /// Advances the simulation by exactly one tick (ARCHITECTURE.md §3.9:
    /// fixed timestep, no wall-clock reads). Two passes over the chain, in
    /// stage order, so behavior stays deterministic:
    ///
    /// 1. Per stage: belt movement, machine process/timer, then the belt's
    ///    front item hands off to that same stage's machine input.
    /// 2. Per adjacent pair: a finished item in stage `i`'s machine output
    ///    hands off onto stage `i + 1`'s belt (the inter-stage conveyor
    ///    hop) — done as a second pass so every stage's own belt/machine
    ///    tick has already happened this tick before anything crosses a
    ///    stage boundary.
    pub fn tick(&mut self) {
        self.dirty.clear();
        self.tick_count += 1;

        for i in 0..self.stages.len() {
            let stage = self.stages[i];
            let belt_tile = self.position_of(stage.belt_entity);
            let machine_tile = self.position_of(stage.machine_entity);

            let belt_changed = {
                let mut belt = self.ecs.get::<&mut BeltSegment>(stage.belt_entity).unwrap();
                belt.tick(self.belt_speed)
            };
            let machine_changed = {
                let mut machine = self.ecs.get::<&mut Machine>(stage.machine_entity).unwrap();
                machine.tick(self.tick_count, &self.recipes)
            };
            let transferred =
                self.try_transfer_belt_to_machine(stage.belt_entity, stage.machine_entity);

            if belt_changed || transferred {
                self.mark_dirty(belt_tile);
            }
            if machine_changed || transferred {
                self.mark_dirty(machine_tile);
            }
        }

        for i in 0..self.stages.len().saturating_sub(1) {
            let from_machine = self.stages[i].machine_entity;
            let to_belt = self.stages[i + 1].belt_entity;
            let machine_tile = self.position_of(from_machine);
            let belt_tile = self.position_of(to_belt);

            let transferred = self.try_transfer_machine_output_to_belt(from_machine, to_belt);
            if transferred {
                self.mark_dirty(machine_tile);
                self.mark_dirty(belt_tile);
            }
        }
    }

    /// If `belt_entity` has an item waiting at its end and `machine_entity`
    /// has room for it, moves it across the interop-invisible boundary
    /// between the two components. Returns whether a transfer happened.
    fn try_transfer_belt_to_machine(&mut self, belt_entity: Entity, machine_entity: Entity) -> bool {
        let ready_item = {
            let belt = self.ecs.get::<&BeltSegment>(belt_entity).unwrap();
            belt.front_ready().cloned()
        };
        let Some(item_id) = ready_item else {
            return false;
        };

        let has_room = {
            let machine = self.ecs.get::<&Machine>(machine_entity).unwrap();
            machine.input.has_room_for(&item_id, 1)
        };
        if !has_room {
            return false;
        }

        let taken = {
            let mut belt = self.ecs.get::<&mut BeltSegment>(belt_entity).unwrap();
            belt.take_front_if_at_end()
        };
        let Some(taken) = taken else {
            return false;
        };
        let mut machine = self.ecs.get::<&mut Machine>(machine_entity).unwrap();
        machine.input.add(&taken.item, 1);
        true
    }

    /// If `machine_entity` is holding a finished item and `belt_entity` has
    /// room to accept a new item at its start, hands one unit across —
    /// the inter-stage conveyor hop from one stage's machine to the next
    /// stage's belt. Returns whether a transfer happened.
    fn try_transfer_machine_output_to_belt(&mut self, machine_entity: Entity, belt_entity: Entity) -> bool {
        let ready_item = {
            let machine = self.ecs.get::<&Machine>(machine_entity).unwrap();
            machine.output.iter().find(|(_, qty)| *qty > 0).map(|(id, _)| id.clone())
        };
        let Some(item_id) = ready_item else {
            return false;
        };

        let spawned = {
            let mut belt = self.ecs.get::<&mut BeltSegment>(belt_entity).unwrap();
            belt.try_spawn(item_id.clone())
        };
        if !spawned {
            return false;
        }

        let mut machine = self.ecs.get::<&mut Machine>(machine_entity).unwrap();
        machine.output.remove(&item_id, 1);
        true
    }

    /// Chunks touched since the start of the current `tick()` call
    /// (ARCHITECTURE.md §4). Godot only needs to re-read chunks that both
    /// changed AND are currently visible.
    pub fn dirty_chunks(&self) -> &[ChunkId] {
        &self.dirty
    }

    pub fn chunk_snapshot(&self, id: ChunkId) -> ChunkSnapshot {
        let mut snapshot = ChunkSnapshot::default();

        for stage in &self.stages {
            let belt_tile = self.position_of(stage.belt_entity);
            if chunk_of(belt_tile) == id {
                let belt = self.ecs.get::<&BeltSegment>(stage.belt_entity).unwrap();
                let span = self.ecs.get::<&BeltSpan>(stage.belt_entity).unwrap();
                let items = belt
                    .snapshot_positions()
                    .into_iter()
                    .map(|(item, pos)| (item, pos.to_num::<f64>()))
                    .collect();
                snapshot.belts.push(BeltSnapshot {
                    start_tile: belt_tile,
                    length_tiles: span.length_tiles,
                    items,
                });
            }

            let machine_tile = self.position_of(stage.machine_entity);
            if chunk_of(machine_tile) == id {
                let machine = self.ecs.get::<&Machine>(stage.machine_entity).unwrap();
                snapshot.machines.push(MachineSnapshot {
                    tile: machine_tile,
                    recipe: machine.recipe.clone(),
                    process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
                    input: machine.input.iter().cloned().collect(),
                    output: machine.output.iter().cloned().collect(),
                });
            }
        }

        snapshot
    }

    /// All chunk ids any current stage's belt or machine occupies. Not part
    /// of `SimBridge`'s surface (Godot only ever needs `dirty_chunks`) —
    /// this exists so tooling/tests can get a complete view of the whole
    /// chain without hardcoding how many chunks it happens to span.
    pub fn all_chunks(&self) -> Vec<ChunkId> {
        let mut ids = Vec::new();
        for stage in &self.stages {
            for tile in [
                self.position_of(stage.belt_entity),
                self.position_of(stage.machine_entity),
            ] {
                let chunk = chunk_of(tile);
                if !ids.contains(&chunk) {
                    ids.push(chunk);
                }
            }
        }
        ids
    }

    /// Static layout of every stage in the chain, in order — see
    /// `StageLayout`'s doc for why Godot needs this rather than hardcoding
    /// stage geometry itself.
    pub fn chain_layout(&self) -> Vec<StageLayout> {
        self.stages
            .iter()
            .map(|stage| {
                let belt_tile = self.position_of(stage.belt_entity);
                let span = self.ecs.get::<&BeltSpan>(stage.belt_entity).unwrap();
                let machine_tile = self.position_of(stage.machine_entity);
                let recipe = self.ecs.get::<&Machine>(stage.machine_entity).unwrap().recipe.clone();
                StageLayout {
                    recipe,
                    belt_start_tile: belt_tile,
                    belt_length_tiles: span.length_tiles,
                    machine_tile,
                }
            })
            .collect()
    }

    pub fn query_tile(&self, pos: TileCoord) -> Option<TileSnapshot> {
        for stage in &self.stages {
            let belt_tile = self.position_of(stage.belt_entity);
            let span = self.ecs.get::<&BeltSpan>(stage.belt_entity).unwrap();
            if pos.1 == belt_tile.1 && pos.0 >= belt_tile.0 && pos.0 < belt_tile.0 + span.length_tiles {
                let belt = self.ecs.get::<&BeltSegment>(stage.belt_entity).unwrap();
                let items_here = belt
                    .snapshot_positions()
                    .into_iter()
                    .filter(|(_, dist)| dist.floor() == Fx::from_num(pos.0 - belt_tile.0))
                    .map(|(item, _)| item)
                    .collect();
                return Some(TileSnapshot::Belt { items_here });
            }

            let machine_tile = self.position_of(stage.machine_entity);
            if pos == machine_tile {
                let machine = self.ecs.get::<&Machine>(stage.machine_entity).unwrap();
                return Some(TileSnapshot::Machine {
                    recipe: machine.recipe.clone(),
                    process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
                });
            }
        }

        None
    }

    /// Applies a player-issued command (ARCHITECTURE.md §4). Invalid/no-op
    /// commands (e.g. spawning onto a belt with no room) are silently
    /// ignored, same as `BeltSegment::try_spawn`'s own contract.
    /// `SpawnItem` always targets the chain's first belt (the raw-ingredient
    /// entry point); `CollectOutput` always targets the chain's last
    /// machine (the finished-product exit point) — a single entry/exit pair
    /// per chain, matching this milestone's "one straight line" scope.
    pub fn issue_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::SpawnItem { item } => {
                let first = self.stages[0];
                let belt_tile = self.position_of(first.belt_entity);
                let spawned = {
                    let mut belt = self.ecs.get::<&mut BeltSegment>(first.belt_entity).unwrap();
                    belt.try_spawn(item)
                };
                if spawned {
                    self.mark_dirty(belt_tile);
                }
            }
            PlayerCommand::CollectOutput => {
                let last = *self.stages.last().unwrap();
                let machine_tile = self.position_of(last.machine_entity);
                let mut machine = self.ecs.get::<&mut Machine>(last.machine_entity).unwrap();
                let held: Vec<(ItemId, u32)> = machine.output.iter().cloned().collect();
                if held.is_empty() {
                    return;
                }
                for (item, qty) in &held {
                    machine.output.remove(item, *qty);
                }
                drop(machine);
                self.mark_dirty(machine_tile);
            }
        }
    }
}

/// Enforces the module doc's chain-linkage assumption: recipe `i + 1` must
/// consume (as one of its inputs) an item that recipe `i` produces. A clear
/// panic here — at chain-construction time, before any tick runs — is the
/// data-file equivalent of a compiler error: it catches a data-entry mistake
/// (typo'd item id, recipes added out of order) immediately, instead of the
/// symptom being "a machine three stages down silently never starts."
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
             (inputs {:?}) — World builds one stage per recipe in file order, so consecutive \
             recipes must chain by a shared item id",
            prev.id, prev.outputs, next.id, next.inputs
        );
    }
}
