//! `gdext` boundary (ARCHITECTURE.md §4). This is the *only* file in the
//! crate that depends on `godot` — everything simulation-related lives in
//! `sim::` and is plain, headlessly-testable Rust (ARCHITECTURE.md §7).
//!
//! Per §4's "interop-layer training-data gap" note, this file's own surface
//! is kept deliberately small: every method here is a thin translation
//! (JSON string / plain int in and out) over `sim::World`, not a place
//! where any simulation logic lives.

pub mod sim;

use godot::prelude::*;

use sim::{RecipeRegistry, World, decode_chunk_id, encode_chunk_id};

struct SauceFactoryExtension;

#[gdextension]
unsafe impl ExtensionLibrary for SauceFactoryExtension {}

/// The bundled recipe content (ARCHITECTURE.md §3.6) — `data/recipes.ron`
/// is the single source of truth; embedding it at compile time keeps
/// resolving it identical whether running under `cargo test`, the Godot
/// editor, or an exported build, none of which share a working directory.
const RECIPES_RON: &str = include_str!("../../data/recipes.ron");

fn load_bundled_recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(RECIPES_RON).expect("bundled data/recipes.ron must parse")
}

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
struct SimBridge {
    base: Base<RefCounted>,
    #[init(val = World::new(load_bundled_recipes()))]
    world: World,
}

#[godot_api]
impl SimBridge {
    /// Milestone-1 round-trip proof — kept so `test_round_trip.gd` still
    /// passes; milestone 3 doesn't remove existing verified behavior.
    #[func]
    fn ping(&self) -> i64 {
        42
    }

    /// Advances the simulation by one tick (ARCHITECTURE.md §3.9: fixed
    /// timestep, driven by Godot's `_process`/`_physics_process`, never
    /// wall-clock time read on the Rust side).
    #[func]
    fn tick(&mut self) {
        self.world.tick();
    }

    #[func]
    fn current_tick(&self) -> i64 {
        self.world.current_tick() as i64
    }

    /// Chunk ids touched since the start of the last `tick()` call, each
    /// packed into a single `i64` (see `sim::encode_chunk_id`).
    #[func]
    fn dirty_chunk_ids(&self) -> PackedInt64Array {
        self.world
            .dirty_chunks()
            .iter()
            .map(|&id| encode_chunk_id(id))
            .collect()
    }

    /// A JSON-serialized `sim::ChunkSnapshot` for the given packed chunk
    /// id. Godot never sees the belt's gap array or the machine's raw
    /// `TimedProcess` — only this rendering-facing snapshot
    /// (ARCHITECTURE.md §4/§5).
    #[func]
    fn chunk_snapshot_json(&self, chunk_id: i64) -> GString {
        let snapshot = self.world.chunk_snapshot(decode_chunk_id(chunk_id));
        let json = serde_json::to_string(&snapshot).expect("ChunkSnapshot must serialize");
        GString::from(json.as_str())
    }

    /// A JSON-serialized `sim::TileSnapshot`, or an empty string if the
    /// tile is unoccupied.
    #[func]
    fn query_tile_json(&self, x: i32, y: i32) -> GString {
        match self.world.query_tile((x, y)) {
            Some(tile) => {
                let json = serde_json::to_string(&tile).expect("TileSnapshot must serialize");
                GString::from(json.as_str())
            }
            None => GString::new(),
        }
    }

    /// Applies a player command, JSON-encoded per
    /// `sim::world::PlayerCommandDto` (e.g.
    /// `{"type": "spawn_item", "item": "tomato"}` or
    /// `{"type": "collect_output"}`). Malformed/invalid commands are
    /// silently ignored, same as an out-of-range command would be.
    #[func]
    fn issue_command_json(&mut self, command_json: GString) {
        if let Some(cmd) = sim::command_from_json(&command_json.to_string()) {
            self.world.issue_command(cmd);
        }
    }

    /// The chain's static per-stage layout (belt/machine tile positions,
    /// which recipe each stage runs), JSON-serialized. Unlike
    /// `chunk_snapshot_json`, this never changes at runtime — Godot calls it
    /// once, at startup, to build a `TileMap` sized to however many stages
    /// `data/recipes.ron` currently defines, instead of hardcoding stage
    /// geometry client-side (ARCHITECTURE.md §4/§5).
    #[func]
    fn chain_layout_json(&self) -> GString {
        let layout = self.world.chain_layout();
        let json = serde_json::to_string(&layout).expect("StageLayout list must serialize");
        GString::from(json.as_str())
    }

    /// A JSON-serialized `sim::EconomySnapshot` — money plus the hardcoded
    /// contract's mix/progress/cadence (ARCHITECTURE.md §9 milestone 5,
    /// GAMEPLAY.md §6). Unlike the chunk snapshots above there's no
    /// dirty-tracking to check first: there's exactly one economy per world,
    /// so Godot can just call this every frame it wants to redraw the HUD.
    #[func]
    fn economy_snapshot_json(&self) -> GString {
        let snapshot = self.world.economy_snapshot();
        let json = serde_json::to_string(&snapshot).expect("EconomySnapshot must serialize");
        GString::from(json.as_str())
    }
}
