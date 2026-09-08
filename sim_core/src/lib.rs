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
    /// snapshots must key machines by `tile` and de-duplicate. The same rule
    /// applies to the `hoppers` and (since the "Visual overhaul round 2"
    /// epic's ticket 1) `fluid_tanks` arrays.
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
    ///
    /// **`fill_fluid_tank` is its fluid twin** (the "Visual overhaul round
    /// 2" epic's ticket 1): `{"type": "fill_fluid_tank", "amount": 30}`, or
    /// `{"type": "fill_fluid_tank"}` for the simulation's own default load,
    /// adds whole fluid units to the pipe run's tank — which is the run's
    /// only supply since that ticket removed the always-refilling off-map
    /// tap. Over-filling is clamped at the tank's capacity, silently, and a
    /// world with no pipe run (no recipe declares `fluid_inputs`) has no
    /// tank, so the command simply does nothing there. Read the result
    /// through `fluid_tank_snapshot_json`.
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
    ///
    /// **`total_volume`/`total_capacity`/`pressure` cover the pipe run
    /// alone**, and always have — the tank that supplies it (the "Visual
    /// overhaul round 2" epic's ticket 1) has its own
    /// `fluid_tank_snapshot_json`. What changed with that ticket is that
    /// these can now legitimately read zero: the run has no self-refilling
    /// source any more, so an unfilled or exhausted tank means dry pipes and
    /// a stalled machine, which is a state presentation should be able to
    /// show rather than treat as a bug. `segments[i].is_source` likewise
    /// still marks the run's supply end, but that segment is now an ordinary
    /// pipe tile (the one the tank connects to) rather than an always-full
    /// tap.
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

    /// A JSON-serialized `sim::FluidTankSnapshot` — where the pipe run's
    /// fluid tank stands, which fluid it holds, and how full it is (the
    /// "Visual overhaul round 2" epic's ticket 1):
    ///
    /// ```json
    /// {"tile": [5, 1], "kind": "fluid_tank",
    ///  "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///  "fluid": "vinegar", "quantity": 30.0, "capacity": 120.0,
    ///  "fill": 0.25, "pipe_tile": [6, 1]}
    /// ```
    ///
    /// Field-for-field the fluid twin of `hopper_snapshot_json`: `fluid`
    /// stands where `item` does, `pipe_tile` where `belt_tile` does, and
    /// `tile`/`kind`/`footprint`/`fill` mean exactly what they mean there —
    /// so a scene can drive both from one code path. The one shape
    /// difference: `quantity`/`capacity` are **floating-point**, not
    /// integers, because a fluid volume is continuous (pressure
    /// equalization moves partial units around).
    ///
    /// Returns an **empty string** when the world has no pipe run at all
    /// (no recipe in `data/recipes.ron` declares `fluid_inputs`, so there is
    /// nothing for a tank to supply) — the same "empty string means nothing
    /// there" convention `fluid_snapshot_json` and `query_tile_json` use,
    /// and the one way this differs from `hopper_snapshot_json`, which
    /// always answers because every world has a hopper. GDScript should
    /// check `is_empty()` before parsing.
    ///
    /// **Poll this every frame you want to redraw the tank**, like
    /// `fluid_snapshot_json` and unlike `chunk_snapshot_json`: the tank is
    /// drained by the same equalization pass the pipe run is, which
    /// deliberately does not dirty its chunk, so `dirty_chunk_ids()` will
    /// never tell you the level fell. Only a `fill_fluid_tank` command
    /// dirties it. `chunk_snapshot_json` carries the identical object in a
    /// new `fluid_tanks` array for a chunk-driven renderer to place the
    /// geometry from, and `query_tile_json` answers with a `"fluid_tank"`
    /// tile carrying the same fill/capacity for hover.
    /// `sim::FluidTankSnapshot` documents the full shape.
    #[func]
    fn fluid_tank_snapshot_json(&self) -> GString {
        match self.world.fluid_tank_snapshot() {
            Some(snapshot) => {
                let json =
                    serde_json::to_string(&snapshot).expect("FluidTankSnapshot must serialize");
                GString::from(json.as_str())
            }
            None => GString::new(),
        }
    }

    /// A JSON-serialized `sim::EconomySnapshot` — money, the shared delivery
    /// pool, and an ARRAY of every active contract (GAMEPLAY.md §6/§9).
    /// Unlike the chunk snapshots above there's no dirty-tracking to check
    /// first: there's exactly one economy per world, so Godot can just call
    /// this every frame it wants to redraw the HUD.
    ///
    /// **This shape is a BREAKING CHANGE from the Economy MVP's**, made when
    /// contracts became `data/contracts.ron` content and several started
    /// running at once. It was:
    ///
    /// ```json
    /// { "money": 0.0, "contract_mix": [["boxed_ketchup", 2]],
    ///   "contract_progress": [["boxed_ketchup", 0]],
    ///   "cadence_ticks": 1000, "next_delivery_tick": 1000 }
    /// ```
    ///
    /// and is now:
    ///
    /// ```json
    /// {
    ///   "money": 0.0,
    ///   "delivery_stock": [["boxed_ketchup", 3]],
    ///   "contracts": [
    ///     {
    ///       "id": "corner_shop",
    ///       "name": "Corner Shop Standing Order",
    ///       "mix": [["boxed_ketchup", 2]],
    ///       "progress": [["boxed_ketchup", 2]],
    ///       "cadence_ticks": 1000,
    ///       "next_delivery_tick": 1000,
    ///       "payout": 50.0,
    ///       "deliveries_made": 0,
    ///       "deliveries_missed": 0
    ///     }
    ///   ]
    /// }
    /// ```
    ///
    /// The four old top-level contract fields are gone: their per-contract
    /// equivalents live inside each `contracts` entry (`contract_mix` ->
    /// `mix`, `contract_progress` -> `progress`). `money` is unchanged and
    /// is still a single value. `progress` is clamped to the mix's required
    /// quantity — a checklist reads "2/2", never "7/2"; the unclamped pool
    /// is `delivery_stock`. `contracts` is in `contracts.ron` file order,
    /// which is also the priority order when two contracts fall due
    /// together and stock covers only one (see `sim::economy`'s module doc
    /// for the full allocation rule).
    #[func]
    fn economy_snapshot_json(&self) -> GString {
        let snapshot = self.world.economy_snapshot();
        let json = serde_json::to_string(&snapshot).expect("EconomySnapshot must serialize");
        GString::from(json.as_str())
    }

    /// A JSON-serialized `sim::ShopSnapshot` — the whole build menu in one
    /// object: what is buyable, what each thing costs, how many the player
    /// owns, and how much money they have (GAMEPLAY.md §1/§4, the "Shop &
    /// Inventory" epic's ticket 1):
    ///
    /// ```json
    /// {"money": 120.0,
    ///  "entries": [
    ///    {"kind": "belt",        "price": 5.0,   "owned": 3, "affordable": true},
    ///    {"kind": "pipe",        "price": 8.0,   "owned": 0, "affordable": true},
    ///    {"kind": "hopper",      "price": 40.0,  "owned": 0, "affordable": true},
    ///    {"kind": "fluid_tank",  "price": 60.0,  "owned": 0, "affordable": true},
    ///    {"kind": "crusher",     "price": 75.0,  "owned": 1, "affordable": true},
    ///    {"kind": "mixer",       "price": 100.0, "owned": 0, "affordable": true},
    ///    {"kind": "cooking_vat", "price": 200.0, "owned": 0, "affordable": false},
    ///    {"kind": "bottler",     "price": 120.0, "owned": 0, "affordable": false},
    ///    {"kind": "packer",      "price": 90.0,  "owned": 0, "affordable": true}
    ///  ]}
    /// ```
    ///
    /// Field by field: `kind` is a `data/shop.ron` buildable-kind id (every
    /// `data/machines.ron` machine kind, plus `belt` and `pipe`, which are
    /// infrastructure priced per tile); `price` is the **unit** price;
    /// `owned` is how many the player has bought and not yet placed — zero
    /// for everything in a fresh world; `affordable` is whether **one** is
    /// buyable at the current `money` (a menu buying several at once should
    /// compare `money` against `price * quantity` itself). `money` is the
    /// same value `economy_snapshot_json` reports, repeated so a build menu
    /// is one call rather than two.
    ///
    /// `entries` lists **every** priced kind, owned or not, in
    /// `data/shop.ron`'s own file order — which is the intended menu order,
    /// so a UI should render it as it arrives rather than re-sorting.
    ///
    /// Always returns a value (every world has a shop), like
    /// `hopper_snapshot_json` and unlike `fluid_snapshot_json` — there is no
    /// empty-string case. **Poll it whenever the build menu redraws**, like
    /// `economy_snapshot_json`: a purchase changes money and a count, not a
    /// tile, so `dirty_chunk_ids()` will never report one, and no chunk
    /// snapshot carries this.
    ///
    /// Buying is `issue_command_json` with
    /// `{"type": "purchase", "kind": "belt", "quantity": 4}` — `quantity`
    /// optional, defaulting to 1. An unaffordable or unknown purchase is a
    /// silent no-op that changes nothing at all (the same contract every
    /// other command has); re-read this snapshot to see the result either
    /// way.
    ///
    /// **What `owned` does not mean yet:** nothing in the simulation
    /// consumes an owned count. The shipped chain is still built
    /// unconditionally at tick 0 whether or not anything was bought, so a
    /// count here is a purchase ledger, not a statement about what is on the
    /// factory floor. Spending it on real placement is a later epic.
    #[func]
    fn shop_snapshot_json(&self) -> GString {
        let snapshot = self.world.shop_snapshot();
        let json = serde_json::to_string(&snapshot).expect("ShopSnapshot must serialize");
        GString::from(json.as_str())
    }
}
