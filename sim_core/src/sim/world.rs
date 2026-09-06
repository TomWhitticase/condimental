//! The simulation `World`: an `hecs::World` (ARCHITECTURE.md §1/§3.8) tying
//! together the belt, machine, and recipe registry for milestone 3's fixed
//! layout — one belt segment feeding one machine.
//!
//! This is also where `SimBridge`'s pure-logic half lives (ARCHITECTURE.md
//! §4): `tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
//! `query_tile()`. The `gdext`-facing wrapper in `lib.rs` is a thin
//! translation layer on top of this — nothing here depends on Godot, so it
//! is fully exercisable from headless `cargo test` (ARCHITECTURE.md §7).

use hecs::Entity;

use super::belt::BeltSegment;
use super::machine::Machine;
use super::recipe::RecipeRegistry;
use super::types::{ChunkId, Fx, ItemId, RecipeId, TileCoord, chunk_of};

/// Where an entity sits in the tile grid (ARCHITECTURE.md §3.1/§3.2).
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub tile: TileCoord,
}

/// How many tiles a belt spans, laid out horizontally starting at its
/// entity's `Position`. Milestone 3 only needs a single straight segment;
/// corners/branches are a later milestone's concern.
#[derive(Debug, Clone, Copy)]
pub struct BeltSpan {
    pub length_tiles: i32,
}

/// A command the player can issue (ARCHITECTURE.md §4's `PlayerCommand`).
/// Kept intentionally small — this milestone's "interactable" requirement
/// is spawning raw ingredient onto the belt and collecting finished output
/// from the machine, not a full build-mode yet.
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
/// wants this; Godot never sees the belt's internal gap array or the
/// machine's raw `TimedProcess` (ARCHITECTURE.md §4/§5).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ChunkSnapshot {
    pub belt: Option<BeltSnapshot>,
    pub machine: Option<MachineSnapshot>,
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

pub struct World {
    ecs: hecs::World,
    recipes: RecipeRegistry,
    tick_count: u64,
    belt_entity: Entity,
    machine_entity: Entity,
    belt_speed: Fx,
    /// Chunks touched since the start of the current `tick()` call. Cleared
    /// (not reallocated) at the start of every `tick()` — see `tick`'s doc
    /// comment and ARCHITECTURE.md §4's "Delta granularity" note.
    dirty: Vec<ChunkId>,
}

const BELT_LENGTH_TILES: i32 = 4;
const ITEM_MIN_SPACING_TILES: f64 = 0.25;
const BELT_SPEED_TILES_PER_TICK: f64 = 0.02;
const MACHINE_INPUT_CAPACITY: u32 = 8;
const MACHINE_OUTPUT_CAPACITY: u32 = 8;

impl World {
    /// Builds milestone 3's fixed layout: a `BELT_LENGTH_TILES`-tile belt
    /// starting at (0, 0), feeding a machine at the next tile over, running
    /// `recipe_id` (looked up in `recipes` — content, not hardcoded per
    /// ARCHITECTURE.md §3.6/§10).
    pub fn new(recipes: RecipeRegistry, recipe_id: RecipeId) -> Self {
        let mut ecs = hecs::World::new();

        let belt_entity = ecs.spawn((
            Position { tile: (0, 0) },
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
                tile: (BELT_LENGTH_TILES, 0),
            },
            Machine::new(recipe_id, MACHINE_INPUT_CAPACITY, MACHINE_OUTPUT_CAPACITY),
        ));

        Self {
            ecs,
            recipes,
            tick_count: 0,
            belt_entity,
            machine_entity,
            belt_speed: Fx::from_num(BELT_SPEED_TILES_PER_TICK),
            dirty: Vec::new(),
        }
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }

    fn mark_dirty(&mut self, tile: TileCoord) {
        let chunk = chunk_of(tile);
        if !self.dirty.contains(&chunk) {
            self.dirty.push(chunk);
        }
    }

    /// Advances the simulation by exactly one tick (ARCHITECTURE.md §3.9:
    /// fixed timestep, no wall-clock reads). Order: belt movement, machine
    /// process/timer, then attempt to hand the belt's front item to the
    /// machine's input buffer.
    pub fn tick(&mut self) {
        self.dirty.clear();
        self.tick_count += 1;

        let belt_tile = self.ecs.get::<&Position>(self.belt_entity).unwrap().tile;
        let machine_tile = self.ecs.get::<&Position>(self.machine_entity).unwrap().tile;

        let belt_changed = {
            let mut belt = self.ecs.get::<&mut BeltSegment>(self.belt_entity).unwrap();
            belt.tick(self.belt_speed)
        };
        let machine_changed = {
            let mut machine = self.ecs.get::<&mut Machine>(self.machine_entity).unwrap();
            machine.tick(self.tick_count, &self.recipes)
        };
        let transferred = self.try_transfer_belt_to_machine();

        if belt_changed || transferred {
            self.mark_dirty(belt_tile);
        }
        if machine_changed || transferred {
            self.mark_dirty(machine_tile);
        }
    }

    /// If the belt has an item waiting at its end and the machine has room
    /// for it, moves it across the interop-invisible boundary between the
    /// two components. Returns whether a transfer happened.
    fn try_transfer_belt_to_machine(&mut self) -> bool {
        let ready_item = {
            let belt = self.ecs.get::<&BeltSegment>(self.belt_entity).unwrap();
            belt.front_ready().cloned()
        };
        let Some(item_id) = ready_item else {
            return false;
        };

        let has_room = {
            let machine = self.ecs.get::<&Machine>(self.machine_entity).unwrap();
            machine.input.has_room_for(&item_id, 1)
        };
        if !has_room {
            return false;
        }

        let taken = {
            let mut belt = self.ecs.get::<&mut BeltSegment>(self.belt_entity).unwrap();
            belt.take_front_if_at_end()
        };
        let Some(taken) = taken else {
            return false;
        };
        let mut machine = self.ecs.get::<&mut Machine>(self.machine_entity).unwrap();
        machine.input.add(&taken.item, 1);
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

        let belt_tile = self.ecs.get::<&Position>(self.belt_entity).unwrap().tile;
        if chunk_of(belt_tile) == id {
            let belt = self.ecs.get::<&BeltSegment>(self.belt_entity).unwrap();
            let span = self.ecs.get::<&BeltSpan>(self.belt_entity).unwrap();
            let items = belt
                .snapshot_positions()
                .into_iter()
                .map(|(item, pos)| (item, pos.to_num::<f64>()))
                .collect();
            snapshot.belt = Some(BeltSnapshot {
                start_tile: belt_tile,
                length_tiles: span.length_tiles,
                items,
            });
        }

        let machine_tile = self.ecs.get::<&Position>(self.machine_entity).unwrap().tile;
        if chunk_of(machine_tile) == id {
            let machine = self.ecs.get::<&Machine>(self.machine_entity).unwrap();
            snapshot.machine = Some(MachineSnapshot {
                tile: machine_tile,
                recipe: machine.recipe.clone(),
                process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
                input: machine.input.iter().cloned().collect(),
                output: machine.output.iter().cloned().collect(),
            });
        }

        snapshot
    }

    pub fn query_tile(&self, pos: TileCoord) -> Option<TileSnapshot> {
        let belt_tile = self.ecs.get::<&Position>(self.belt_entity).unwrap().tile;
        let span = self.ecs.get::<&BeltSpan>(self.belt_entity).unwrap();
        if pos.1 == belt_tile.1 && pos.0 >= belt_tile.0 && pos.0 < belt_tile.0 + span.length_tiles {
            let belt = self.ecs.get::<&BeltSegment>(self.belt_entity).unwrap();
            let items_here = belt
                .snapshot_positions()
                .into_iter()
                .filter(|(_, dist)| dist.floor() == Fx::from_num(pos.0 - belt_tile.0))
                .map(|(item, _)| item)
                .collect();
            return Some(TileSnapshot::Belt { items_here });
        }

        let machine_tile = self.ecs.get::<&Position>(self.machine_entity).unwrap().tile;
        if pos == machine_tile {
            let machine = self.ecs.get::<&Machine>(self.machine_entity).unwrap();
            return Some(TileSnapshot::Machine {
                recipe: machine.recipe.clone(),
                process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
            });
        }

        None
    }

    /// Applies a player-issued command (ARCHITECTURE.md §4). Invalid/no-op
    /// commands (e.g. spawning onto a belt with no room) are silently
    /// ignored, same as `BeltSegment::try_spawn`'s own contract.
    pub fn issue_command(&mut self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::SpawnItem { item } => {
                let belt_tile = self.ecs.get::<&Position>(self.belt_entity).unwrap().tile;
                let spawned = {
                    let mut belt = self.ecs.get::<&mut BeltSegment>(self.belt_entity).unwrap();
                    belt.try_spawn(item)
                };
                if spawned {
                    self.mark_dirty(belt_tile);
                }
            }
            PlayerCommand::CollectOutput => {
                let machine_tile = self.ecs.get::<&Position>(self.machine_entity).unwrap().tile;
                let mut machine = self.ecs.get::<&mut Machine>(self.machine_entity).unwrap();
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
