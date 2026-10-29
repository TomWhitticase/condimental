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

struct CondimentalExtension;

#[gdextension]
unsafe impl ExtensionLibrary for CondimentalExtension {}

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
    /// The [`sim::world::StorageKey`] `storage_revision` last saw, and how
    /// many times it has seen it move — presentation-side change detection
    /// for a snapshot that is polled, not dirtied.
    storage_seen: Option<sim::world::StorageKey>,
    storage_revision: i64,
    /// How many commands this bridge has taken and how many times its world
    /// has been replaced — `revision` without the ticks. See
    /// [`Self::command_revision`].
    command_revision: i64,
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

    /// `chunk_snapshot_json` cut down to what CHANGED in that chunk since the
    /// dirty set was last cleared — the belts and splitters whose contents
    /// moved, the machines/hoppers/tanks/docks whose live state did, never
    /// a pipe — and the WHOLE snapshot after a place/remove/rotate/load
    /// touched the chunk. Same JSON shape as `chunk_snapshot_json`, so a
    /// renderer keyed by tile applies either with the same code; a belt
    /// absent from a delta is a belt that did not change. This is the read
    /// to pair with `dirty_chunk_ids()` on the tick path (the quality pass,
    /// 2026-09-13: the full snapshot re-sent every belt in a chunk, most of
    /// them empty, thirty times a second). See `sim::World::chunk_delta`.
    #[func]
    fn chunk_delta_json(&self, chunk_id: i64) -> GString {
        let snapshot = self.world.chunk_delta(decode_chunk_id(chunk_id));
        let json = serde_json::to_string(&snapshot).expect("ChunkSnapshot must serialize");
        GString::from(json.as_str())
    }

    /// Ticks plus commands taken so far (`sim::World::revision`). Two equal
    /// readings mean no polled snapshot can differ between them, so a
    /// presentation layer can skip its per-frame parses on the frames
    /// between ticks; a refused command still bumps it.
    #[func]
    fn revision(&self) -> i64 {
        self.world.revision() as i64
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
    ///
    /// **`sign_contract` is new with the contract-board epic**:
    /// `{"type": "sign_contract", "id": "diner_supply"}` takes an offer off
    /// `economy_snapshot_json`'s `board` for its `price`, out of the same
    /// money everything else spends. A silent no-op on an unknown id, an
    /// offer that is already active (one signature per offer), or a fee the
    /// balance does not cover.
    ///
    /// **`hire` and `fire` are new with the staff pools MVP epic**:
    /// `{"type": "hire", "category": "manufacturing"}` adds one head, free
    /// and never refused for a known category; `{"type": "fire", ...}`
    /// removes one, free, a silent no-op at headcount 0. Both are silent
    /// no-ops for an unknown category and neither dirties a chunk — poll
    /// `staff_snapshot_json`.
    ///
    /// **Two changes from the chain-rework epic**, both of which a caller
    /// has to consume:
    ///
    /// - **`place` takes an optional `item`, and a hopper now needs it**:
    ///   `{"type": "place", "kind": "hopper", "tile": [0, -1],
    ///   "direction": "north", "item": "sugar"}`. A hopper holds one item
    ///   for its whole life, and the shipped chain has two raw ingredients
    ///   (tomato and sugar), so omitting it is a **silent refusal** rather
    ///   than a default — there is no longer one sensible answer for the
    ///   simulation to supply. `ingredients_snapshot_json` lists the ids a
    ///   hopper may legally be given. The field is ignored for every other
    ///   kind.
    /// - **`buy_ingredient` is new**: `{"type": "buy_ingredient",
    ///   "item": "tomato", "quantity": 40}` (`quantity` optional, default
    ///   one). Raw ingredients are no longer free — `fill_hopper` and
    ///   `fill_fluid_tank` now draw from bought stock and are a silent no-op
    ///   when there is none, so a UI that offers "fill" must also offer
    ///   "buy".
    #[func]
    fn issue_command_json(&mut self, command_json: GString) {
        // Counted before it is parsed, refused or not — the same rule
        // `World::issue_command` applies to `revision`: a reader cannot tell
        // a refusal from a no-op without re-reading.
        self.command_revision += 1;
        if let Some(cmd) = sim::command_from_json(&command_json.to_string()) {
            self.world.issue_command(cmd);
        }
    }

    /// What a command would do to the **next payday**, without issuing it —
    /// `sim::SpendPreview` as JSON (ARCHITECTURE.md §4, §5's wire table).
    /// It takes the **same JSON string** the caller is about to hand
    /// [`Self::issue_command_json`], so there is no way to preview one thing
    /// and buy another.
    ///
    /// ```json
    /// {"spends": true, "subject": "crusher", "cost": 120.0,
    ///  "money_now": 1300.0, "money_after": 1180.0,
    ///  "next_bill": 1600, "bill_after": 1600,
    ///  "shortfall_now": 0, "shortfall_after": 420,
    ///  "crosses": true, "warn": true}
    /// ```
    ///
    /// **`warn` is the decision, and it is the sim's** — presentation raises
    /// its confirmation on `warn`, and never recomputes the test from the
    /// other fields (`godot_project/CLAUDE.md`'s one rule). The rest of the
    /// fields exist so the panel can say *why* in the sim's own numbers.
    ///
    /// **It mutates nothing**: no tick, no command, and `revision()` does
    /// not move — so calling it on every hover or keypress cannot make the
    /// renderer re-poll, and a preview can never change what the following
    /// `issue_command_json` does. An unparseable command, or one that costs
    /// nothing (a `place`, a `rotate`, a hopper setting), answers
    /// `spends: false` with zeroed money rather than erroring: a caller may
    /// preview anything it is about to issue.
    ///
    /// Four commands can warn — `purchase`, `buy_ingredient`,
    /// `sign_contract` and `hire`. `hire` is the odd one: it spends nothing
    /// today, so its `cost` is 0 and its `bill_after` is what rises.
    #[func]
    fn spend_preview_json(&self, command_json: GString) -> GString {
        // An unparseable command spends nothing, for the same reason
        // `issue_command_json` ignores it: the caller learns the answer by
        // reading, not from an error it has no way to handle.
        let cmd = sim::command_from_json(&command_json.to_string());
        let preview = self.world.spend_preview(cmd.as_ref());
        let json = serde_json::to_string(&preview).expect("SpendPreview must serialize");
        GString::from(json.as_str())
    }

    /// Which pipe runs a `place` command would draw from that a placed
    /// machine already draws from — the build ghost's sharing warning (issue
    /// #92). Takes exactly what `preview_command_json` takes, so the ghost
    /// asks both questions of one dictionary; changes nothing. JSON array of
    /// `sim::SharedRunPreview`:
    ///
    /// ```json
    /// [{"pipe": [13, 1], "fluid": "ketchup", "drawn_by": ["bottler"]}]
    /// ```
    ///
    /// `[]` for anything but a `place`, and for malformed JSON: no warning.
    #[func]
    fn placement_fluid_preview_json(&self, command_json: GString) -> GString {
        let shared = match sim::command_from_json(&command_json.to_string()) {
            Some(sim::PlayerCommand::Place {
                kind,
                tile,
                direction,
                ..
            }) => self.world.placement_fluid_preview(&kind, tile, direction),
            _ => Vec::new(),
        };
        let json = serde_json::to_string(&shared).expect("SharedRunPreview must serialize");
        GString::from(json.as_str())
    }

    /// Would `command_json` succeed if issued now, and what would its tile
    /// become? Takes exactly what `issue_command_json` takes and changes
    /// nothing — `sim::World::preview_command`, which asks the same rule
    /// functions the command acts on, so a ghost cannot go green on a
    /// placement the sim then refuses. JSON-serialized `sim::PreviewResult`:
    ///
    /// ```json
    /// {"valid": true, "reason": "", "result_kind": "crossing"}
    /// {"valid": false, "reason": "A pipe can only cross a belt at right angles.",
    ///  "result_kind": ""}
    /// ```
    ///
    /// `reason` is a sentence to show, empty when valid; `result_kind` is
    /// the kind the tile would end up as, empty when nothing is placed.
    /// `place` and `purchase` are previewed; any other command, and
    /// malformed JSON, comes back `valid: false` with a reason saying so.
    #[func]
    fn preview_command_json(&self, command_json: GString) -> GString {
        let result = match sim::command_from_json(&command_json.to_string()) {
            Some(cmd) => self.world.preview_command(&cmd),
            None => sim::PreviewResult::malformed(),
        };
        let json = serde_json::to_string(&result).expect("PreviewResult must serialize");
        GString::from(json.as_str())
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
    ///    "occupies": [[14, 0], [15, 0], [14, 1], [15, 1]], "recipe": "ketchup",
    ///    "machine": true}
    /// ]
    /// ```
    ///
    /// **`recipe: null` does not mean "not a machine"** (2026-10-02): a
    /// machine with no recipe chosen carries `null` too (2026-10-03).
    /// `machine: true`, present on machines only, is the test.
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

    /// [`Self::world_layout_json`] with each placement serialized on its
    /// own: entry `i` is exactly the JSON object `world_layout_json`'s array
    /// holds at `i`, in the same order.
    ///
    /// It exists for the scene's rebuild (issue #3). Parsing the whole
    /// array is most of what a re-read costs at scale — ~12 ms of
    /// `JSON.parse_string` for a 1500-tile line, against ~1 ms to write it —
    /// and a place, remove or rotate leaves nearly every entry
    /// byte-identical. One string per entry lets `main.gd` keep what it
    /// parsed last time keyed by that string, and parse only the entries
    /// that changed. The key is the entry's whole text, so a change the
    /// cache did not anticipate (a neighbour's links, `crewed` flipping
    /// factory-wide) is a new key and is parsed — nothing has to say what
    /// changed.
    #[func]
    fn world_layout_entries(&self) -> PackedStringArray {
        self.world
            .world_layout()
            .iter()
            .map(|placement| {
                let json = serde_json::to_string(placement).expect("world layout must serialize");
                GString::from(json.as_str())
            })
            .collect()
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
    /// [{"fluid": "vinegar", "naming": "carries", "candidates": ["vinegar"],
    ///   "total_volume": 27.5, "total_capacity": 40.0,
    ///   "pressure": 0.6875,
    ///   "segments": [{"tile": [6, 1], "direction": "east", "volume": 10.0,
    ///                 "capacity": 10.0, "pressure": 1.0, "is_source": true,
    ///                 "naming": "carries"}],
    ///   "taps": [{"machine_tile": [9, 0], "machine_recipe": "ketchup_mix",
    ///             "units_per_craft": 2, "segment_index": 3}],
    ///   "tank_tiles": [[5, 1]]}]
    /// ```
    ///
    /// **`naming` is why `fluid` is what it is** (issue #54), and it is the
    /// field to drive a fault treatment from: `carries` works, `unplumbed`
    /// is pipe nobody has connected yet, and `unnameable`/`ambiguous` are
    /// the two ways a run stalls everything on it for good — nothing can
    /// name it, or it could be any of `candidates` and the sim will not
    /// guess. Rendering `fluid: null` as one state draws those three as
    /// one, which is exactly the bug. Every segment repeats its run's
    /// `naming` so the `pipes` array of a chunk snapshot can be drawn
    /// without stitching the networks back together.
    ///
    /// Fluid volume changes on nearly every tick and deliberately does *not*
    /// dirty its chunk, so `dirty_chunk_ids()` will never tell you fluid
    /// moved — unlike `chunk_snapshot_json`, this is polled. But poll it only
    /// when `command_revision` has moved: everything here except the levels
    /// is the plumbing's SHAPE — which networks, their segments' tiles and
    /// directions, what each is named — and the shape is recomputed only by
    /// a command or a load (`recompute_topology`, sim_core/CLAUDE.md). On the
    /// ticks between, `fluid_levels` carries the four numbers per network and
    /// per segment that do move, in this snapshot's own order. Pipe
    /// *geometry* is also carried in `chunk_snapshot_json`'s `pipes` array
    /// for a chunk-driven renderer; the live levels are here.
    #[func]
    fn fluid_snapshots_json(&self) -> GString {
        let snapshots = self.world.fluid_snapshots();
        let json = serde_json::to_string(&snapshots).expect("FluidSnapshot list must serialize");
        GString::from(json.as_str())
    }

    /// What `fluid_snapshots_json` carries that a TICK moves, and nothing
    /// else: for each network in that snapshot's order, its `total_volume`
    /// and `pressure`, then for each of its segments in order the segment's
    /// `volume` and `pressure` — one flat array, `2 + 2 × segments` numbers
    /// per network. The same `f64`s the JSON would print, read off the same
    /// `World::fluid_snapshots`, so a reader that already holds the shape
    /// writes these into it and draws exactly what a full read would have
    /// drawn. A reader whose shape does not fit the array (the sizes
    /// disagree) has a stale shape and takes the full read.
    ///
    /// Why a second wire for the same facts: on a 650-placement floor the
    /// full snapshot is ~50 KB of JSON, built, converted and parsed thirty
    /// times a second to redraw 276 liquid bodies whose tiles never moved.
    #[func]
    fn fluid_levels(&self) -> PackedFloat64Array {
        self.world
            .fluid_snapshots()
            .iter()
            .flat_map(|network| {
                [network.total_volume, network.pressure].into_iter().chain(
                    network
                        .segments
                        .iter()
                        .flat_map(|segment| [segment.volume, segment.pressure]),
                )
            })
            .collect()
    }

    /// How many commands this bridge has taken (`issue_command_json`,
    /// parsed or not, refused or not) plus how many times its world was
    /// replaced (`load_ron`, `new_game`) — `revision` with the ticks taken
    /// out. Two equal readings mean nothing has recomputed the topology in
    /// between: the floor's layout, the fluid networks and their naming are
    /// what they were, because every recompute sits in a command handler or
    /// a load (sim_core/CLAUDE.md, "recompute_topology runs only when the
    /// factory changes"). What a tick moves — levels, items, processes — is
    /// not covered, which is the point: it lets a poll tell a tick from a
    /// command and read only what a tick can change.
    #[func]
    fn command_revision(&self) -> i64 {
        self.command_revision
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

    /// Every recipe-running machine's load and stall, JSON-serialized as an
    /// **array** of `sim::MachineLoadEntry` — one per machine (never a
    /// hopper, tank or dock), keyed by origin tile, in the topology's
    /// stable order:
    ///
    /// ```json
    /// [{"tile": [4, 0],
    ///   "load": {"working": 540, "starved": 60, "blocked": 0,
    ///            "observed": 600, "window": 600, "percent": 90,
    ///            "status": "starved"},
    ///   "stall": {"kind": "input", "item": "sugar"}}]
    /// ```
    ///
    /// `load` is the machine's rolling window over the last `window` ticks
    /// (`sim::LOAD_WINDOW_TICKS`), `percent` its working share, and
    /// `status` the sim's verdict — `"working"`, `"starved"` or
    /// `"blocked"` — which Godot reads as a word and never re-derives from
    /// the counts. `stall` is why it is waiting *this tick* (`null` when it
    /// is not): `kind` is one of `input`, `fluid_input`, `no_supply_pipe`,
    /// `output`, `fluid_output`, `no_outlet_pipe`, and `item` names the
    /// item or fluid. See `docs/MACHINE_LOAD_PLAN.md`, "The wire contract".
    /// Each entry also carries `crewed`, `mothballed` and `understaffed` —
    /// the last is the stable "held back for want of a crew" the red person over
    /// a machine reads, where a `staff` stall is only this tick's
    /// (`sim::MachineLoadEntry::understaffed`).
    ///
    /// **Poll it — do not wait for a dirty chunk.** A load window changes
    /// every tick on every machine and deliberately dirties nothing, the
    /// same rule as fluid movement; polling every few ticks is cheap, since
    /// this carries nothing else. `chunk_snapshot_json`'s `machines` and
    /// `query_tile_json`'s `"machine"` tile carry the identical `load` and
    /// `stall` objects for hover.
    #[func]
    fn machine_load_snapshots_json(&self) -> GString {
        let snapshots = self.world.machine_load_snapshots();
        let json = serde_json::to_string(&snapshots).expect("MachineLoadEntry list must serialize");
        GString::from(json.as_str())
    }

    /// What the factory has MADE and USED, per item and fluid, over the
    /// rolling windows in `sim::STAT_WINDOWS` and for all time — the
    /// production-stats epic's wire (the **P** screen). One
    /// `sim::ProductionSnapshot`:
    ///
    /// ```json
    /// {"windows": [{"id": "5s", "ticks": 150, "observed": 150},
    ///              {"id": "1m", "ticks": 1800, "observed": 1800}, ...],
    ///  "items": [{"id": "tomato", "kind": "item",
    ///             "produced": [0, 0, 0, 0], "consumed": [8, 96, 96, 96],
    ///             "produced_total": 0, "consumed_total": 96}, ...],
    ///  "cycle": {"period_ticks": 1000, "next_tick": 3000, "cycles_closed": 2,
    ///            "lines": [{"id": "boxed_ketchup", "required": 2.0,
    ///                       "produced_this_cycle": 1, "produced_last_cycle": 3,
    ///                       "short": false}]}}
    /// ```
    ///
    /// `cycle` is the pay cycle (the staff payday period): one line per
    /// product an active contract orders, in priority order — what one
    /// period requires (a fraction when a cadence does not divide it), what
    /// was made since payday and in the period before, and whether that
    /// finished period made less than the contracts active now require.
    /// `lines` is empty with no contract.
    ///
    /// `windows` is in shortest-first order with how much of each has
    /// actually been observed (a rate divides by that, not by the window);
    /// every `produced`/`consumed` array is in the same order. `items`
    /// lists every id ever recorded, sorted by id — Godot sorts by count
    /// and formats; the sim only counts. `kind` is `"item"` or `"fluid"`.
    ///
    /// **Poll it — do not wait for a dirty chunk.** It changes on every
    /// craft and deliberately dirties nothing, like the load window.
    #[func]
    fn production_snapshot_json(&self) -> GString {
        let snapshot = self.world.production_snapshot();
        let json = serde_json::to_string(&snapshot).expect("ProductionSnapshot must serialize");
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
    ///   "belt_tile": [34, 0], "contents": [["boxed_ketchup", 3]],
    ///   "places": 32, "used": 3, "at_capacity": false,
    ///   "shipped_total": 37, "last_paid_tick": 1200, ...}]
    /// ```
    ///
    /// The exit-end counterpart of `hopper_snapshots_json`, and an array for
    /// the same reason: a dock is an ordinary placed building, so a world
    /// may have none or several. **`contents` is what stands on THAT dock**
    /// (2026-09-26): draw its deck, its hover grid (`places`, `used`) and
    /// its "full" marker (`at_capacity`) from it. `economy_snapshot_json`'s
    /// `delivery_stock` is the sum a contract settles against, not any one
    /// dock. A dock is polled, not dirty-tracked, so `shipped_total` rising
    /// is how to tell a box just arrived HERE, and `last_paid_tick` equal to
    /// the tick a payout landed is how to tell this dock paid it.
    ///
    /// **`belt_tile` replaces `belt_start_tile`/`belt_length_tiles`**: a dock
    /// no longer owns a belt (it used to be built with one), so what is
    /// reported is simply which belt tile currently feeds it, or `null`.
    /// `shipped_total` is that dock's own lifetime output — the number worth
    /// putting on a hover ("Loading dock — 37 shipped").
    ///
    /// **The two staffing flags are not interchangeable** (issue #87).
    /// `understaffed` is per-tick and only moves on a tick the dock had
    /// something to ship, so it blinks on a line that ebbs; `crewed` is the
    /// establishment's standing answer and is the one to draw a lasting
    /// "short of logistics staff" from. `crewed` rides this POLLED channel
    /// precisely because hiring and firing dirty no chunk, so the copy on
    /// `query_tile_json` goes stale between scene rebuilds.
    #[func]
    fn dock_snapshots_json(&self) -> GString {
        let snapshots = self.world.dock_snapshots();
        let json = serde_json::to_string(&snapshots).expect("DockSnapshot list must serialize");
        GString::from(json.as_str())
    }

    /// A JSON-serialized `sim::EconomySnapshot` — money, the stock every
    /// contract can draw on (summed over the docks), and an ARRAY of every active contract (GAMEPLAY.md §6/§9).
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
    ///   "delivery_capacity": 32, "delivery_used": 3, "delivery_full": false,
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
    /// quantity — a checklist reads "2/2", never "7/2"; the unclamped sum
    /// is `delivery_stock`. `contracts` is in `contracts.ron` file order,
    /// which is also the priority order when two contracts fall due
    /// together and stock covers only one (see `sim::economy`'s module doc
    /// for the full allocation rule).
    ///
    /// **The contract-board epic (2026-09-10) added to this, without
    /// removing anything.** `contracts` now lists only the ACTIVE contracts
    /// — a new game has none (the shipped board has no starter) — and each entry also
    /// carries `"satisfied": false` (would the window pay if it ended now),
    /// `"window_start_tick": 0` (the other end of the countdown),
    /// `"price": 40.0` (what it costs to sign, or re-sign after losing it),
    /// `"consecutive_misses": 0` and `"strikes": 3` (it is LOST when the
    /// first reaches the second). Beside `contracts` sits `"board"`: every
    /// offer in `data/contracts.ron`, in file order, active or not —
    ///
    /// ```json
    /// "board": [
    ///   {"id": "corner_shop", "name": "Corner Shop Standing Order",
    ///    "mix": [["boxed_ketchup", 2]], "cadence_ticks": 1000,
    ///    "payout": 50.0, "price": 40.0, "strikes": 3, "starter": false,
    ///    "active": false, "affordable": true, "times_lost": 0,
    ///    "locked": false, "unlock_name": "", "unlock_item": "",
    ///    "unlock_boxes": 0, "unlock_sold": 0},
    ///   ...
    /// ]
    /// ```
    ///
    /// — which is what a contracts menu is built from, the way
    /// `shop_snapshot_json`'s `entries` build the build menu. A contract
    /// that is lost simply stops appearing in `contracts` and reads
    /// `"active": false` (with `times_lost` bumped) on the board; there is
    /// no event on the wire, so a UI that wants to announce a loss diffs
    /// the ids between two polls.
    ///
    /// **The BBQ epic (2026-09-18) added progression gates, again without
    /// removing anything.** Two new top-level fields sit beside `board`:
    ///
    /// ```json
    /// "lifetime_deliveries": 7,
    /// "unlocks": [
    ///   {"id": "bbq_sauce", "name": "BBQ Sauce", "open": false,
    ///    "item": "boxed_ketchup", "sold": 34, "required": 100,
    ///    "ingredients": 3, "contracts": 4}
    /// ]
    /// ```
    ///
    /// `lifetime_deliveries` is every delivery window this save has ever
    /// satisfied — it never decreases, and losing a contract does not take
    /// its past deliveries back. **Nothing gates off it**: since 2026-09-26 a
    /// gate counts boxes of a named product sold, and each entry carries its
    /// own. `unlocks` is `data/unlocks.ron` in file order (which is the tier
    /// order — two gates counting different products have no comparable
    /// numbers), each entry already resolved: `open` is the answer,
    /// `sold`/`required` are the progress a HUD can draw as "34 of 100", and
    /// `item` is the product's **id**, to be titled with
    /// [`Self::display_names_json`] like every other id on the wire. `name`
    /// is player-facing prose, so it is what an announcement says, and
    /// `ingredients`/`contracts` are how many ids it puts on sale and on the
    /// board — counts, not lists, so the announcement can name the halves of
    /// the game that changed without assuming every gate touches both.
    ///
    /// **`board` padlocks what those gates hold back** (since 2026-09-29;
    /// before that it left it out): an offer behind a closed gate is listed
    /// with `"locked": true` and the gate's `unlock_name`, `unlock_item`,
    /// `unlock_boxes` and `unlock_sold` — the contents picker's four fields,
    /// meaning the same thing — so the board can show what is coming and
    /// what opens it. `ingredients_snapshot_json`'s `entries` is still
    /// filtered: a locked ingredient is simply absent from the market. Do
    /// not re-derive either answer in GDScript — the simulation refuses
    /// `sign_contract` and `buy_ingredient` for a locked id, so the wire and
    /// the rules already agree. Like a contract loss, an unlock raises no
    /// event: a UI that wants to announce one diffs `unlocks[].open` between
    /// two polls.
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
    ///  "categories": [
    ///    {"id": "logistics", "name": "Logistics",
    ///     "note": "belts, pipes and the bins that feed them"}, ...],
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
    /// Every entry also carries `category` (a `categories` id — the loader
    /// refuses a file where it is not) and `crew`, left out of the rows above
    /// for width. `categories` are the build menu's headings, in file order.
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

    /// The staff pools (the staff pools MVP epic, GAMEPLAY.md §7) as one
    /// JSON object (`sim::StaffSnapshot`) — everything the staff panel
    /// draws, in one poll:
    ///
    /// ```json
    /// {"pay_period_ticks": 1000, "next_payday_tick": 3000,
    ///  "next_bill": 14, "shortfall": 0,
    ///  "last_bill": 14, "unpaid_last_payday": 0, "walked_out_last_payday": 0,
    ///  "money": 2986.0,
    ///  "categories": [
    ///    {"id": "manufacturing", "name": "Manufacturing", "headcount": 4,
    ///     "wage": 2, "owed": 4, "unpaid": 0, "left_last_payday": 0,
    ///     "hire_warns": false, "demand": 4, "assigned": 4, "short": 0},
    ///    {"id": "engineering",   "name": "Engineering",   "headcount": 0,
    ///     "wage": 2, "owed": 0, "unpaid": 0, "left_last_payday": 0,
    ///     "hire_warns": false, "demand": 0, "assigned": 0, "short": 0},
    ///    {"id": "packaging",     "name": "Packaging",     "headcount": 2,
    ///     "wage": 2, "owed": 2, "unpaid": 0, "left_last_payday": 0,
    ///     "hire_warns": false, "demand": 2, "assigned": 2, "short": 0},
    ///    {"id": "logistics",     "name": "Logistics",     "headcount": 1,
    ///     "wage": 2, "owed": 1, "unpaid": 0, "left_last_payday": 0,
    ///     "hire_warns": false, "demand": 1, "assigned": 1, "short": 0}]}
    /// ```
    ///
    /// `categories` is in `data/staff.ron` order (the panel order and the
    /// payday order). `demand`, `assigned` and `short` (= `demand -
    /// assigned`) describe the LAST completed tick: every head machines
    /// and docks asked for, how many they got, how many they went without.
    /// Godot reads these; it never recomputes them from machine counts.
    /// `money` is repeated here as `shop_snapshot_json` repeats it.
    /// `next_payday_tick` is in sim ticks; the panel turns it into seconds
    /// against `current_tick()`.
    ///
    /// **The quality pass's fields (2026-09-13), for warning BEFORE a
    /// payday and announcing AFTER it.** `next_bill` is what the next
    /// payday will charge — every head employed for any part of the
    /// period, fired or not (notice pay), `Σ owed × wage` — and
    /// `shortfall` is how far `money` falls short of it (0 when covered):
    /// while it is non-zero, heads will go unpaid at the payday. After a
    /// payday, `unpaid_last_payday` is how many hired heads the money did
    /// not reach and `walked_out_last_payday` how many of them LEFT
    /// (unpaid there and at the payday before). Per category: `owed`
    /// (heads the next payday bills), `unpaid` (heads in arrears — still
    /// hired, NOT working, gone if unpaid again; the row should say so in
    /// words), `left_last_payday` (who left, for the announcement), and
    /// `hire_warns` (whether a hire would leave the next payday short:
    /// the balance under `next_bill` plus the category's wage — read it to
    /// WARN, not to disable, since 2026-09-26; the hire goes through if the
    /// player confirms it, and [`Self::spend_preview_json`] is the same rule
    /// asked per command). Read `next_bill` for the footer's bill: the
    /// `Σ headcount × wage` Godot used to compute is the run rate, not
    /// what the payday will take.
    ///
    /// **Poll it — do not wait for a dirty chunk.** The tallies change
    /// every tick and dirty nothing, the `machine_load_snapshots_json`
    /// rule. The commands are `{"type": "hire", "category": "manufacturing"}`
    /// and `{"type": "fire", "category": "manufacturing"}`; a machine short
    /// of its crew reads `"stall": {"kind": "staff", "item": "<category>"}`
    /// with status `starved`, a dock reads `"understaffed": true` on the
    /// tick it was refused and `"crewed": false` for as long as the
    /// shortage stands (issue #87 — the per-tick flag alone blinks), and
    /// `query_tile_json`'s machine and dock variants plus every
    /// `shop_snapshot_json` entry carry `"crew": [{"category": ...,
    /// "count": ...}]`. See `docs/STAFF_POOLS_PLAN.md`, "The wire contract".
    #[func]
    fn staff_snapshot_json(&self) -> GString {
        let snapshot = self.world.staff_snapshot();
        let json = serde_json::to_string(&snapshot).expect("StaffSnapshot must serialize");
        GString::from(json.as_str())
    }

    /// The raw-ingredient market — what a unit of each consumable costs,
    /// how much of it the player is carrying, and the same money figure
    /// `shop_snapshot_json` reports — as one JSON object
    /// (`sim::IngredientsSnapshot`).
    ///
    /// ```json
    /// {"money": 2913.0,
    ///  "entries": [
    ///    {"id": "tomato", "price": 1.0, "owned": 40, "capacity": 320,
    ///     "affordable": true},
    ///    {"id": "sugar", "price": 3.0, "owned": 0, "capacity": 20,
    ///     "affordable": true},
    ///    {"id": "vinegar", "price": 2.0, "owned": 12, "capacity": 20,
    ///     "affordable": true}
    ///  ]}
    /// ```
    ///
    /// Deliberately the same shape as `shop_snapshot_json`, so a buy panel
    /// can be one widget driven by either — but a **different call**,
    /// because the two price different kinds of thing: a shop entry is a
    /// building you place and can pick back up, an entry here is stock a
    /// machine eats (GAMEPLAY.md §5). `entries` is in
    /// `data/ingredients.ron`'s own order and lists every priced
    /// ingredient, including ones the player currently has none of —
    /// **except any a progression gate is still holding back**
    /// (`data/unlocks.ron`; the BBQ epic, 2026-09-18). A locked ingredient
    /// is absent rather than flagged, because there is nothing a panel
    /// could usefully do with it: `buy_ingredient` is refused for it in the
    /// simulation, so listing it greyed out would only advertise a price
    /// nobody can pay. `economy_snapshot_json`'s `unlocks` is where a HUD
    /// reads what is still to come, and how close it is.
    ///
    /// Beside it, `hopper_items` is the list of ids a hopper may be placed
    /// holding — see `issue_command_json` on `place`'s `item` field — which
    /// is `entries` minus any that are fluids, plus any a gate is holding
    /// back. It is deliberately **not** gate-filtered: it is a placement
    /// rule the simulation actually enforces on `place`, not a shop
    /// listing, and a hopper set to an ingredient nobody can buy yet is
    /// simply a hopper that stays empty.
    #[func]
    fn ingredients_snapshot_json(&self) -> GString {
        let snapshot = self.world.ingredients_snapshot();
        let json = serde_json::to_string(&snapshot).expect("IngredientsSnapshot must serialize");
        GString::from(json.as_str())
    }

    /// The warehouse: how much raw stock may be held, and what is standing
    /// in the racks right now, as one JSON object (`sim::StorageSnapshot`).
    ///
    /// ```json
    /// {"base_cap": 20, "slot_stack": 50, "total_bays": 12, "used_bays": 7,
    ///  "free_bays": 1,
    ///  "racks": [
    ///    {"tile": [0, -8], "kind": "warehouse_rack", "direction": "east",
    ///     "footprint": {"width_tiles": 1, "height_tiles": 2}, "bays": 6,
    ///     "contents": [{"bay": 0, "item": "tomato", "units": 50,
    ///                   "capacity": 50},
    ///                  {"bay": 1, "item": "tomato", "units": 30,
    ///                   "capacity": 50}],
    ///     "filter": "tomato", "blocked_filters": [],
    ///     "removal_refused": false}
    ///  ]}
    /// ```
    ///
    /// **Polled, not pushed.** Like the fluid and staff reads, a change to
    /// what is in a bay dirties no chunk: stock moves whenever a bin is
    /// filled or a contract is signed, and marking the warehouse's tiles
    /// dirty for that would make every purchase a structural update. The
    /// warehouse panel reads this while it is open, and the renderer reads
    /// it after a `place`/`remove`.
    ///
    /// `contents` holds one entry per **occupied** bay, in bay order, so an
    /// empty rack reports `[]` rather than six nulls. A bay's `units` is at
    /// most `capacity` and the last bay an item occupies is usually part
    /// full. Which bay holds what is **derived** from the ledger rather
    /// than stored — a reserved rack's own raw fills it first, then the
    /// rest fills the open racks in sorted id order, racks in tile order —
    /// so it can change under a renderer that caches it; re-read after any
    /// buy or fill.
    ///
    /// `filter` is the one raw a rack is reserved for (`set_rack_filter`),
    /// `null` when any raw may stand in it. `blocked_filters` lists the raws
    /// that command would refuse because some stock would be left with no
    /// bay, and `removal_refused` whether `remove` would refuse the rack for
    /// the same reason — both answered here so the panel dims a choice
    /// rather than redoing the arithmetic.
    ///
    /// `base_cap` is the free, unconditional ceiling every raw ingredient
    /// has with no warehouse at all, and `free_bays` the room above it that
    /// any raw may still take. With a rack reserved, `total_bays - used_bays`
    /// is more than that by the reserved racks' empty bays, which only their
    /// own raw may use (above: tomato stands in two of its rack's six, so
    /// four of the five empty bays are tomato's). Per-ingredient capacity is on
    /// `ingredients_snapshot_json`'s entries, because it moves with what
    /// else is held.
    #[func]
    fn storage_snapshot_json(&self) -> GString {
        let snapshot = self.world.storage_snapshot();
        let json = serde_json::to_string(&snapshot).expect("StorageSnapshot must serialize");
        GString::from(json.as_str())
    }

    /// A number that moves whenever `storage_snapshot_json` would answer
    /// differently from the last time this was asked, and only then
    /// (`World::storage_key`). Godot's warehouse poll reads this every frame
    /// and the snapshot only when it has moved: a big warehouse's snapshot is
    /// hundreds of kilobytes, and its stock changes on a hopper refill, not
    /// on every tick.
    #[func]
    fn storage_revision(&mut self) -> i64 {
        let key = self.world.storage_key();
        if self.storage_seen.as_ref() != Some(&key) {
            self.storage_seen = Some(key);
            self.storage_revision += 1;
        }
        self.storage_revision
    }

    /// The content ids that carry a name of their own, as one flat JSON
    /// object of `id -> display name`:
    ///
    /// ```json
    /// {"bbq": "BBQ", "bbq_mix": "BBQ Mix", "bottled_bbq": "Bottled BBQ"}
    /// ```
    ///
    /// **Read this once, at load, not per frame** — it is `data/*.ron`
    /// read back and cannot change while a game is running.
    ///
    /// An id that is absent has no name in the data, which is the caller's
    /// cue to title-case the id itself (`main.gd`'s `_kind_display_name`).
    /// That is the common case by a wide margin: the object holds only the
    /// handful of ids title-casing gets wrong, acronyms being the reason it
    /// exists (`bbq` -> "Bbq"). See [`sim::World::display_names`].
    #[func]
    fn display_names_json(&self) -> GString {
        let names = self.world.display_names();
        let json = serde_json::to_string(&names).expect("display names must serialize");
        GString::from(json.as_str())
    }

    /// The content ids that name their own voxel asset, as one flat JSON
    /// object of `id -> asset name`:
    ///
    /// ```json
    /// {"tomato": "item_tomato", "mixer": "machine_mixer"}
    /// ```
    ///
    /// **Read this once, at load**, for [`Self::display_names_json`]'s
    /// reason — it is `data/*.ron` read back and cannot move while a game
    /// runs.
    ///
    /// It exists so that swapping or adding an asset is a content edit.
    /// `main.gd` has carried this mapping by hand since the shop epic, as
    /// `ITEM_MODELS` / `MACHINE_MODELS` — tables in a file that has no way
    /// to know when the content changed under it, and that ship green when
    /// they go stale (UI_UX.md §2b point 12). `_icon_for` now asks this
    /// first and those tables second, so the fallback chain is wire ->
    /// table -> `ITEM_MODEL_FALLBACK` and nothing new has to be added to
    /// the tables again.
    ///
    /// An absent id is the normal case and means "no opinion" — not "no
    /// icon". See [`sim::World::icon_names`].
    #[func]
    fn icon_names_json(&self) -> GString {
        let icons = self.world.icon_names();
        let json = serde_json::to_string(&icons).expect("icon names must serialize");
        GString::from(json.as_str())
    }

    /// Every recipe a machine of `kind` can be set to, in `data/recipes.ron`
    /// order — what the machine's picker lists (`PlayerCommand::SetMachineRecipe`)
    /// — as `{"id", "locked", "unlock_name", "unlock_item", "unlock_boxes",
    /// "unlock_sold"}` entries, the CONTENTS pickers' shape. `[]` for a kind
    /// no recipe runs on.
    #[func]
    fn machine_recipes_json(&self, kind: GString) -> GString {
        let choices = self.world.machine_recipe_choices(&kind.to_string());
        let json = serde_json::to_string(&choices).expect("recipe choices must serialize");
        GString::from(json.as_str())
    }

    /// THE COOKBOOK: the whole of `data/*.ron` as the in-game encyclopedia
    /// reads it — a sauce tree, and a page per sauce, machine and
    /// ingredient. Shape and reasoning: [`sim::cookbook`].
    ///
    /// ```json
    /// {
    ///   "sauces": [{
    ///     "id": "boxed_ketchup", "tier": 0, "locked": false,
    ///     "tagline": "The first sauce. Everything else in the book is built on it.",
    ///     "steps": [{"recipe": "tomato_paste", "machine": "crusher",
    ///                "time_ticks": 80, "carries": "tomato_paste",
    ///                "carries_fluid": false, "new_machine": true}],
    ///     "inputs": [{"id": "tomato", "quantity": 4.0, "fluid": false,
    ///                 "from_sauce": null, "locked": false}],
    ///     "cost_per_unit": 11.0, "pay_low": 24.0, "pay_high": 40.0
    ///   }],
    ///   "edges": [{"from": "boxed_ketchup", "to": "boxed_bbq", "via": "ketchup"}],
    ///   "machines": [], "ingredients": []
    /// }
    /// ```
    ///
    /// **Not folded into `economy_snapshot_json`.** That one is polled
    /// every frame and this is a static graph: the only part of it that
    /// moves is the padlocks, which move with the boxes-sold ledger. A locked
    /// page carries its own `gate` — `{"id", "name", "item", "required",
    /// "sold"}` — so the padlock reads "34 of 100 boxes of Boxed Ketchup"
    /// from the page it is drawn on. Read it on open, and again on the
    /// panel's own poll cadence while it is open.
    ///
    /// **Ids go out as ids.** Title them with the map
    /// [`Self::display_names_json`] returns, the way every other id on the
    /// wire is titled; a second title-caser in Rust would be a second thing
    /// to keep in step (`godot_project/CLAUDE.md`'s one rule). Likewise a
    /// step carries `time_ticks` and the panel divides by `TICK_HZ`.
    #[func]
    fn cookbook_json(&self) -> GString {
        let cookbook = self.world.cookbook();
        let json = serde_json::to_string(&cookbook).expect("cookbook must serialize");
        GString::from(json.as_str())
    }

    /// The whole world as a RON save file, as text
    /// (`sim::save::SaveData` — see that module's doc for the format and
    /// for why it is a full snapshot rather than ARCHITECTURE.md §4's
    /// command log).
    ///
    /// **Writing it somewhere is the caller's job, deliberately.** Nothing
    /// in `sim_core` touches the filesystem: resolving `user://saves/…` is
    /// Godot's, so this hands back a string and GDScript decides what file
    /// it becomes. Two saves of one unchanged world are byte-identical, so
    /// a caller may compare them.
    ///
    /// Always succeeds — every world is saveable.
    #[func]
    fn save_ron(&self) -> GString {
        GString::from(self.world.to_save_ron().as_str())
    }

    /// Replaces the live world with the one in `ron`, and reports what
    /// happened: **an empty string means it loaded**, anything else is a
    /// human-readable reason it did not.
    ///
    /// A message rather than a `bool` for the reason DEVLOG's "SPACE and V
    /// say why, instead of doing nothing" entry gives: a control that fails
    /// silently leaves the player unable to tell a corrupt file from a save
    /// written by an older build from one they never made. The text is
    /// suitable for showing verbatim (e.g. *"save schema_version 2 is not
    /// supported by this build (supports 1)"*).
    ///
    /// **On failure the live world is untouched** — not partially loaded,
    /// not emptied. The load builds a whole new world and only swaps it in
    /// once every step has succeeded, so a refused load is exactly as
    /// eventless as a refused command.
    ///
    /// On success the world reports **every** chunk it occupies as dirty,
    /// so a chunk-driven renderer redraws all of it. Re-read
    /// `world_layout_json` too: a load changes what is on the floor exactly
    /// the way a `place`/`remove`/`rotate` does.
    #[func]
    fn load_ron(&mut self, ron: GString) -> GString {
        self.command_revision += 1;
        match World::from_save_ron(&ron.to_string(), load_bundled_recipes()) {
            Ok(world) => {
                self.world = world;
                GString::new()
            }
            Err(error) => GString::from(error.to_string().as_str()),
        }
    }

    /// Throws the live world away and starts a fresh one — an empty floor
    /// and `sim::STARTING_MONEY`, exactly what a newly constructed
    /// `SimBridge` has.
    ///
    /// It exists so a main menu's *New Game* and its *Load* are the same
    /// shape: both replace the world inside one long-lived bridge, so
    /// nothing else in the scene has to know which of the two happened.
    /// Constructing a new `SimBridge` would do the same job, so this is a
    /// convenience rather than a capability — but a scene that keeps the
    /// bridge in one place and swaps worlds under it has one fewer thing
    /// that can go stale.
    ///
    /// Like a load, this reports every chunk of the new world dirty (there
    /// are none), and `world_layout_json` is empty afterwards.
    #[func]
    fn new_game(&mut self) {
        self.command_revision += 1;
        self.world = World::new(load_bundled_recipes());
    }
}
