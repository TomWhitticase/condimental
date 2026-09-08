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
    /// applies to the `hoppers`, `fluid_tanks` and (since the delivery-dock
    /// ticket) `docks` arrays.
    ///
    /// **A fresh world is empty**, so every array here is `[]` until the
    /// player places something. Each `belts` entry is now one *tile* with a
    /// `direction`, not a multi-tile run (`start_tile`/`length_tiles` are
    /// gone), and machines, hoppers, tanks, pipes and docks all carry a
    /// `direction` too.
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
    /// which machine it hit and where that machine starts. That is also the
    /// tile to pass to `remove`/`rotate`: they resolve it the same way.
    ///
    /// **Every variant now carries a `direction`** (`"north"`/`"east"`/
    /// `"south"`/`"west"`), and `footprint` is the kind's *declared*
    /// rectangle — at `"north"`/`"south"` its two extents swap on the grid.
    /// See `sim::TileSnapshot` for the field-by-field shape.
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
    /// `sim::world::PlayerCommandDto`. Malformed/invalid commands are
    /// silently ignored — and so is every *valid but impossible* one, which
    /// is this simulation's universal contract: nothing ever half-applies.
    ///
    /// **Three commands are new in the "Real Placement" epic**, and they are
    /// what a build mode is made of:
    ///
    /// - `{"type": "place", "kind": "crusher", "tile": [4, 0],
    ///   "direction": "east"}` — puts one **owned** unit of `kind` down with
    ///   its footprint's minimum-X/minimum-Y corner on `tile`. It spends
    ///   inventory, not money (the money went at `purchase` time), so
    ///   `shop_snapshot_json`'s `owned` count for that kind drops by one.
    ///   `direction` is optional and defaults to `"east"`. It does nothing
    ///   at all when the player owns none of that kind, when the kind is not
    ///   buildable, or when the rotated footprint would overlap any
    ///   already-occupied tile.
    /// - `{"type": "remove", "tile": [4, 0]}` — picks up whatever occupies
    ///   `tile` and credits one back to that kind's owned count. **Any tile
    ///   of a multi-tile footprint works** and the whole thing goes, the
    ///   same "point at the machine, not at its corner" rule
    ///   `query_tile_json` follows. What it was holding — items on a belt,
    ///   fluid in a pipe, a batch mid-craft — is destroyed with it.
    /// - `{"type": "rotate", "tile": [4, 0]}` — turns it 90° clockwise in
    ///   place (east -> south -> west -> north). Any tile of the footprint
    ///   works; a no-op if the rotated footprint would collide with a
    ///   *different* placement. Contents are untouched: rotating a belt does
    ///   not spill it and rotating a machine does not restart its craft.
    ///
    /// Directions are the four strings `"north"`, `"east"`, `"south"`,
    /// `"west"`, everywhere one is read or written. **`"east"` is `+X` and
    /// `"north"` is `+Y`**, and east-facing is what reproduces the layout
    /// the game had before placement existed.
    ///
    /// `{"type": "purchase", "kind": "belt", "quantity": 4}` is unchanged
    /// (`quantity` optional, defaulting to 1) but now means something
    /// concrete: what it credits is what `place` spends.
    ///
    /// **`fill_hopper` and `fill_fluid_tank` gained an optional `tile`**
    /// naming *which* building to load, since a world may hold several of
    /// each: `{"type": "fill_hopper", "amount": 16, "tile": [-1, 0]}`.
    /// Omitting it fills every hopper (or every tank) in the world, which is
    /// what the pre-placement one-hopper world always did on a keypress — so
    /// the old wire form still behaves identically. `amount` is still
    /// optional too, and over-filling is still clamped, silently.
    ///
    /// **`collect_output` and `spawn_item` do not exist** and have not for
    /// two tickets: product leaves the factory through a dock building and
    /// arrives through a hopper. Sending either is a silent no-op.
    #[func]
    fn issue_command_json(&mut self, command_json: GString) {
        if let Some(cmd) = sim::command_from_json(&command_json.to_string()) {
            self.world.issue_command(cmd);
        }
    }

    /// Everything currently placed in the world, JSON-serialized as an
    /// array of `sim::PlacementSnapshot` — kind, origin tile, facing,
    /// declared footprint, the tiles it actually occupies once rotated, and
    /// (for a machine) the recipe it runs.
    ///
    /// **This replaces `chain_layout_json`, and is a breaking change.** That
    /// call reported an ordered list of belt-plus-machine *stages*, because
    /// the world was a fixed chain built from `data/recipes.ron`'s file
    /// order. Since the "Real Placement" epic there is no chain and no
    /// stage: the player places every belt tile, machine and building
    /// individually, so what a renderer needs is "what is on the floor,
    /// where, facing which way, how big" — for everything, not only the
    /// recipe-driven half.
    ///
    /// ```json
    /// [
    ///   {"kind": "hopper", "tile": [-1, 0], "direction": "east",
    ///    "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///    "occupies": [[-1, 0]], "recipe": null},
    ///   {"kind": "belt", "tile": [0, 0], "direction": "east",
    ///    "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///    "occupies": [[0, 0]], "recipe": null},
    ///   {"kind": "cooking_vat", "tile": [14, 0], "direction": "east",
    ///    "footprint": {"width_tiles": 2, "height_tiles": 2},
    ///    "occupies": [[14, 0], [15, 0], [14, 1], [15, 1]], "recipe": "ketchup"}
    /// ]
    /// ```
    ///
    /// **Unlike `chain_layout_json` this is not static** — call it again
    /// after any `place`, `remove` or `rotate` command, and it is empty for
    /// a fresh world, which now contains nothing at all.
    #[func]
    fn world_layout_json(&self) -> GString {
        let layout = self.world.world_layout();
        let json = serde_json::to_string(&layout).expect("world layout must serialize");
        GString::from(json.as_str())
    }

    /// Every fluid network in the world, JSON-serialized as an **array** of
    /// `sim::FluidSnapshot` — each one's fluid, per-segment volume/pressure,
    /// which machines tap it and which tanks feed it (ARCHITECTURE.md §9
    /// milestone 6, §3.4). Empty (`[]`) when no pipe is placed.
    ///
    /// **Breaking change from `fluid_snapshot_json`**, which returned one
    /// object or an empty string: a world used to have at most one hardcoded
    /// pipe run, and now has one network per connected component of placed
    /// pipe. Per-network the shape also changed — `machine_tile`/
    /// `machine_recipe`/`units_per_craft`/`inlet_index` became a `taps`
    /// array (a component may be tapped by several machines or none),
    /// `tank_tiles` is new, `fluid` is nullable, each segment carries a
    /// `direction`, and **`segments` is in tile order, not flow order**
    /// (adjacency is no longer "consecutive entries" — a run can turn).
    ///
    /// ```json
    /// [{"fluid": "vinegar", "total_volume": 27.5, "total_capacity": 40.0,
    ///   "pressure": 0.6875,
    ///   "segments": [{"tile": [6, 1], "direction": "east", "volume": 10.0,
    ///                 "capacity": 10.0, "pressure": 1.0, "is_source": true}],
    ///   "taps": [{"machine_tile": [9, 0], "machine_recipe": "ketchup_mix",
    ///             "units_per_craft": 2, "segment_index": 3}],
    ///   "tank_tiles": [[5, 1]]}]
    /// ```
    ///
    /// Read this every frame you want to redraw fluid, like
    /// `economy_snapshot_json` and unlike `chunk_snapshot_json`: fluid volume
    /// changes on nearly every tick and deliberately does *not* dirty its
    /// chunk, so `dirty_chunk_ids()` will never tell you fluid moved. Pipe
    /// *geometry* is also carried in `chunk_snapshot_json`'s `pipes` array
    /// for a chunk-driven renderer; the live levels are here.
    #[func]
    fn fluid_snapshots_json(&self) -> GString {
        let snapshots = self.world.fluid_snapshots();
        let json = serde_json::to_string(&snapshots).expect("FluidSnapshot list must serialize");
        GString::from(json.as_str())
    }

    /// Every raw-ingredient hopper in the world, JSON-serialized as an
    /// **array** of `sim::HopperSnapshot` — where each stands, which way it
    /// faces, what it holds and how full it is:
    ///
    /// ```json
    /// [{"tile": [-1, 0], "kind": "hopper", "direction": "east",
    ///   "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///   "item": "tomato", "quantity": 12, "capacity": 64, "fill": 0.1875,
    ///   "belt_tile": [0, 0]}]
    /// ```
    ///
    /// **Breaking change from `hopper_snapshot_json`**, which returned one
    /// object and always had one to return. A hopper is now an ordinary
    /// placed building, so a world may have none (a fresh one does) or
    /// several, each filled and unloading independently. `direction` is new,
    /// and **`belt_tile` is nullable** — it is whichever belt this hopper
    /// currently discharges onto, and a hopper facing empty ground has none.
    ///
    /// Poll it whenever the fill UI redraws, like `economy_snapshot_json` —
    /// though unlike fluid, a hopper's level also dirties its chunk when it
    /// changes, and `chunk_snapshot_json` carries the identical objects in
    /// its `hoppers` array, so a chunk-driven renderer works too.
    /// `query_tile_json` answers with a `"hopper"` tile carrying the same
    /// fill/capacity for hover.
    #[func]
    fn hopper_snapshots_json(&self) -> GString {
        let snapshots = self.world.hopper_snapshots();
        let json = serde_json::to_string(&snapshots).expect("HopperSnapshot list must serialize");
        GString::from(json.as_str())
    }

    /// Every fluid tank in the world, JSON-serialized as an **array** of
    /// `sim::FluidTankSnapshot`:
    ///
    /// ```json
    /// [{"tile": [5, 1], "kind": "fluid_tank", "direction": "east",
    ///   "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///   "fluid": "vinegar", "quantity": 30.0, "capacity": 120.0,
    ///   "fill": 0.25, "pipe_tile": [6, 1]}]
    /// ```
    ///
    /// Field-for-field the fluid twin of `hopper_snapshots_json`: `fluid`
    /// stands where `item` does and `pipe_tile` where `belt_tile` does, so a
    /// scene can drive both from one code path. Two shape differences:
    /// `quantity`/`capacity` are **floating-point** (a fluid volume is
    /// continuous), and **`fluid` is nullable** — a tank's fluid is decided
    /// by whichever machine taps the network it discharges into, so a tank
    /// connected to nothing reports `null`.
    ///
    /// **Poll this every frame you want to redraw a tank**, like
    /// `fluid_snapshots_json`: a tank is drained by the same equalization
    /// pass the pipes are, which deliberately does not dirty its chunk, so
    /// `dirty_chunk_ids()` will never tell you the level fell. Only a
    /// `fill_fluid_tank` command dirties it.
    #[func]
    fn fluid_tank_snapshots_json(&self) -> GString {
        let snapshots = self.world.fluid_tank_snapshots();
        let json =
            serde_json::to_string(&snapshots).expect("FluidTankSnapshot list must serialize");
        GString::from(json.as_str())
    }

    /// Every delivery dock in the world, JSON-serialized as an **array** of
    /// `sim::DockSnapshot`:
    ///
    /// ```json
    /// [{"tile": [35, 0], "kind": "dock", "direction": "east",
    ///   "footprint": {"width_tiles": 1, "height_tiles": 1},
    ///   "belt_tile": [34, 0], "contents": [], "capacity_per_item": 32,
    ///   "shipped_total": 37}]
    /// ```
    ///
    /// The exit-end counterpart of `hopper_snapshots_json`, and an array for
    /// the same reason: a dock is an ordinary placed building now, so a
    /// world may have none or several — and several docks all ship into the
    /// one shared delivery pool, so "what is waiting to be sold" is still
    /// exactly one number. Two shape notes that have not changed:
    ///
    /// 1. **No `fill` field.** A dock ships everything it receives into the
    ///    shared pool in the same tick, so its own level is always zero
    ///    between ticks and `contents` is normally `[]`. **Draw a pallet
    ///    stack from `economy_snapshot_json`'s `delivery_stock` instead.**
    /// 2. **`contents` is a list**, not one item plus a count: a dock
    ///    accepts whatever the line delivers.
    ///
    /// **`belt_tile` replaces `belt_start_tile`/`belt_length_tiles`**: a dock
    /// no longer owns a belt (it used to be built with one), so what is
    /// reported is simply which belt tile currently feeds it, or `null`.
    /// `shipped_total` is that dock's own lifetime output — the number worth
    /// putting on a hover ("Loading dock — 37 shipped").
    #[func]
    fn dock_snapshots_json(&self) -> GString {
        let snapshots = self.world.dock_snapshots();
        let json = serde_json::to_string(&snapshots).expect("DockSnapshot list must serialize");
        GString::from(json.as_str())
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
    ///    {"kind": "packer",      "price": 90.0,  "owned": 0, "affordable": true},
    ///    {"kind": "dock",        "price": 70.0,  "owned": 0, "affordable": true}
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
    /// **What `owned` means, as of the "Real Placement" epic:** how many of
    /// that kind the player has bought and **not yet placed**. It is real
    /// inventory now, not a ledger of past purchases — `place` spends one
    /// and `remove` credits one back — so a build menu should offer to place
    /// a kind exactly when `owned > 0`, and offer to buy one when it isn't.
    /// A world starts with nothing placed and nothing owned, and
    /// `sim::STARTING_MONEY` in the bank.
    #[func]
    fn shop_snapshot_json(&self) -> GString {
        let snapshot = self.world.shop_snapshot();
        let json = serde_json::to_string(&snapshot).expect("ShopSnapshot must serialize");
        GString::from(json.as_str())
    }
}
