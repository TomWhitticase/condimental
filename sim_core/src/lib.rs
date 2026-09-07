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
    ///
    /// Each entry of the `machines` array carries `kind` and `footprint`
    /// alongside its origin `tile` (see `sim::MachineSnapshot`). A machine
    /// whose footprint straddles a chunk boundary is reported in *every*
    /// chunk it overlaps, identically — so a caller merging several chunks'
    /// snapshots must key machines by `tile` and de-duplicate.
    #[func]
    fn chunk_snapshot_json(&self, chunk_id: i64) -> GString {
        let snapshot = self.world.chunk_snapshot(decode_chunk_id(chunk_id));
        let json = serde_json::to_string(&snapshot).expect("ChunkSnapshot must serialize");
        GString::from(json.as_str())
    }

    /// A JSON-serialized `sim::TileSnapshot`, or an empty string if the
    /// tile is unoccupied.
    ///
    /// A machine resolves from **any** tile inside its footprint, not only
    /// its origin — the returned `machine` variant carries `machine_kind`,
    /// `footprint` (`{"width_tiles", "height_tiles"}`) and `origin_tile`, so
    /// a hover on the far corner of a multi-tile machine still identifies
    /// which machine it hit and where that machine starts. See
    /// `sim::TileSnapshot` for the field-by-field shape.
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
    /// `{"type": "fill_hopper", "amount": 16}` or
    /// `{"type": "collect_output"}`). Malformed/invalid commands are
    /// silently ignored, same as an out-of-range command would be.
    ///
    /// **`fill_hopper` replaced `spawn_item`** (the "Machines improvement"
    /// milestone's ticket 3): raw ingredient is now loaded into the chain's
    /// hopper, which meters it onto the first belt itself, one unit per
    /// tick, instead of the command teleporting an item onto that belt.
    /// `amount` may be omitted (`{"type": "fill_hopper"}`) to add the
    /// simulation's own default load; over-filling is clamped at the
    /// hopper's capacity, never an error. Read the result through
    /// `hopper_snapshot_json`.
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
    ///
    /// Each stage also reports `machine_kind` and `machine_footprint`
    /// (`{"width_tiles", "height_tiles"}`) from `data/machines.ron`:
    /// `machine_tile` is the footprint's minimum-X/minimum-Y **origin**
    /// corner — the tile the belt feeds into — and the machine covers
    /// `width_tiles` x `height_tiles` tiles growing east and north from it.
    /// `sim::StageLayout` documents the full shape.
    #[func]
    fn chain_layout_json(&self) -> GString {
        let layout = self.world.chain_layout();
        let json = serde_json::to_string(&layout).expect("StageLayout list must serialize");
        GString::from(json.as_str())
    }

    /// A JSON-serialized `sim::FluidSnapshot` — the pipe run's fluid type,
    /// per-segment volume/pressure, and which machine it feeds
    /// (ARCHITECTURE.md §9 milestone 6, §3.4). Returns an **empty string**
    /// when the world has no pipe run at all (no recipe in
    /// `data/recipes.ron` declares `fluid_inputs`), the same "empty string
    /// means nothing there" convention `query_tile_json` already uses —
    /// GDScript should check `is_empty()` before parsing.
    ///
    /// Read this every frame you want to redraw fluid, like
    /// `economy_snapshot_json` and unlike `chunk_snapshot_json`: fluid
    /// volume changes on nearly every tick and deliberately does *not*
    /// dirty its chunk (that would defeat dirty tracking), so
    /// `dirty_chunk_ids()` will never tell you fluid moved. Pipe
    /// *geometry* is also carried in `chunk_snapshot_json`'s new `pipes`
    /// array for a chunk-driven renderer; the live levels are here. The
    /// full field-by-field shape is documented on `sim::FluidSnapshot`.
    #[func]
    fn fluid_snapshot_json(&self) -> GString {
        match self.world.fluid_snapshot() {
            Some(snapshot) => {
                let json = serde_json::to_string(&snapshot).expect("FluidSnapshot must serialize");
                GString::from(json.as_str())
            }
            None => GString::new(),
        }
    }

    /// A JSON-serialized `sim::HopperSnapshot` — where the chain's raw-
    /// ingredient hopper stands, which item it holds, and how full it is
    /// (the "Machines improvement" milestone's ticket 3):
    ///
    /// ```json
    /// {"tile": [-1, 0], "kind": "hopper",
    ///  "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///  "item": "tomato", "quantity": 12, "capacity": 64, "fill": 0.1875,
    ///  "belt_tile": [0, 0]}
    /// ```
    ///
    /// Always returns a value (every world has exactly one hopper), so
    /// unlike `fluid_snapshot_json` there is no empty-string case. Poll it
    /// whenever the fill UI redraws, like `economy_snapshot_json` — though
    /// unlike fluid, a hopper's level also dirties its chunk when it
    /// changes, and `chunk_snapshot_json` carries the identical object in a
    /// new `hoppers` array, so a chunk-driven renderer works too. Note the
    /// shipped hopper sits at x = -1, i.e. in chunk (-1, 0) rather than the
    /// (0, 0) that holds the rest of the chain. `sim::HopperSnapshot`
    /// documents the full shape; `query_tile_json` answers with a `"hopper"`
    /// tile carrying the same fill/capacity for hover.
    #[func]
    fn hopper_snapshot_json(&self) -> GString {
        let snapshot = self.world.hopper_snapshot();
        let json = serde_json::to_string(&snapshot).expect("HopperSnapshot must serialize");
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
