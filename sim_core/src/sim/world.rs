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
//! description in data rather than "one stage per recipe in file order" —
//! flagged here rather than solved now, since nothing before milestone 8/9
//! (Scale-up, Logistics expansion) needs more than one line (CLAUDE.md:
//! "don't design for hypothetical future requirements").
//!
//! **Machine size is data, not a constant** (the "Machines improvement"
//! milestone). Each stage's machine occupies a rectangle of tiles given by
//! its kind's [`Footprint`] in `data/machines.ron`, anchored at an origin
//! tile that is the rectangle's minimum-X/minimum-Y corner and also the tile
//! the stage's belt feeds into — see [`super::machines`]'s module doc, which
//! is the single statement of that convention. The chain is laid out by
//! *accumulating* those widths (belt tiles, then this machine's full width,
//! then the next stage begins immediately after), not by multiplying a fixed
//! per-stage span, so making a kind wider pushes every downstream stage east
//! by exactly the extra tiles. Ticket 1 introduced the arithmetic with every
//! kind still 1x1 (where it reduces to the old fixed span exactly, moving no
//! tile); ticket 2 gave it a real user — the shipped `cooking_vat` is 2x2, so
//! every stage downstream of it sits one tile further east than the old
//! formula would have put it.
//!
//! This is also where `SimBridge`'s pure-logic half lives (ARCHITECTURE.md
//! §4): `tick()`, `dirty_chunks()`/`chunk_snapshot()`, `issue_command()`,
//! `query_tile()`, `chain_layout()`. The `gdext`-facing wrapper in `lib.rs`
//! is a thin translation layer on top of this — nothing here depends on
//! Godot, so it is fully exercisable from headless `cargo test`
//! (ARCHITECTURE.md §7).

use hecs::Entity;

use super::belt::BeltSegment;
use super::economy::{Economy, EconomySnapshot};
use super::fluid::{FluidNetwork, PipeSegment};
use super::machine::Machine;
use super::machines::{Footprint, MachineRegistry};
use super::recipe::{Recipe, RecipeRegistry};
use super::types::{ChunkId, FluidId, Fx, ItemId, RecipeId, TileCoord, chunk_of};

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

/// Which machine kind an entity is, and how many tiles it therefore covers
/// (the "Machines improvement" milestone). Both come straight from
/// `data/machines.ron` — this component is where a stage's entry in that
/// file lands in the ECS, so nothing has to re-look-it-up by recipe id.
///
/// The entity's `Position` is the footprint's **origin**: its minimum-X,
/// minimum-Y corner, and the tile its belt feeds into. See
/// [`super::machines`]'s module doc for the full convention.
#[derive(Debug, Clone)]
pub struct MachinePlacement {
    pub kind: String,
    pub footprint: Footprint,
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
    /// Pipe segments of the fluid network (milestone 6) whose tiles fall in
    /// this chunk. See `PipeSnapshot` for the JSON shape, and
    /// `World::fluid_snapshot` for why live fluid *levels* are better read
    /// from there than from here.
    pub pipes: Vec<PipeSnapshot>,
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

/// One machine, for rendering.
///
/// JSON shape (`snake_case`, as serialized by `serde_json`):
///
/// ```json
/// {
///   "tile": [14, 0],
///   "kind": "cooking_vat",
///   "footprint": { "width_tiles": 2, "height_tiles": 2 },
///   "recipe": "ketchup",
///   "process": [120, 240],
///   "input": [["ketchup_mix", 1]],
///   "output": [["ketchup", 1]]
/// }
/// ```
///
/// `kind`/`footprint` are new in the "Machines improvement" milestone.
/// **`tile` is the footprint's origin — its minimum-X, minimum-Y corner, and
/// the tile the belt feeds into** (see [`super::machines`]'s module doc); the
/// machine covers `x in [tile.x, tile.x + width_tiles)`, `y in [tile.y,
/// tile.y + height_tiles)`. A renderer that placed a mesh at `tile` and
/// assumed one tile of extent stays correct for every 1x1 kind and only
/// needs the footprint to size or centre a larger one.
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
    /// The footprint's origin tile — see this type's doc.
    pub tile: TileCoord,
    /// Machine kind id from `data/machines.ron` (e.g. "crusher").
    pub kind: String,
    /// How many tiles this machine covers, anchored at `tile`.
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
/// JSON shape (`snake_case` field names, as serialized by `serde_json`):
///
/// ```json
/// { "tile": [8, 1], "volume": 7.5, "capacity": 10.0, "pressure": 0.75, "is_source": false }
/// ```
///
/// `volume`/`capacity` are in whole fluid units; `pressure` is the
/// dimensionless fill fraction `volume / capacity`, in `[0, 1]` — the value
/// to drive a fill-level visual from. All three are `f64` for the same
/// reason `BeltSnapshot::items` positions are: they have already crossed
/// out of fixed-point simulation state into presentation-only values that
/// never feed back in (ARCHITECTURE.md §3.9). `is_source` marks the
/// off-map supply tap, which is a different thing visually from a plain
/// length of pipe.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PipeSnapshot {
    pub tile: TileCoord,
    pub volume: f64,
    pub capacity: f64,
    pub pressure: f64,
    pub is_source: bool,
}

/// The whole fluid network in one snapshot — what a HUD/overlay needs
/// without having to stitch together per-chunk pipe lists.
///
/// Read this the way `economy_snapshot` is read, **not** the way
/// `chunk_snapshot` is: fluid volume changes on almost every tick, and
/// deliberately does **not** mark its chunk dirty (see `World::tick`), so
/// `dirty_chunks` will not tell you when fluid moved. There is exactly one
/// network per world, so polling this each frame you want to redraw is
/// cheap and always current — the same trade `EconomySnapshot` already
/// makes. Chunk snapshots still carry pipes so a chunk-driven renderer can
/// place the geometry; this is where the live state lives.
///
/// JSON shape — `null` when the loaded `data/recipes.ron` has no recipe
/// with `fluid_inputs` at all (no fluid-requiring recipe, no pipe run):
///
/// ```json
/// {
///   "fluid": "vinegar",
///   "total_volume": 27.5,
///   "total_capacity": 40.0,
///   "pressure": 0.6875,
///   "segments": [ { "tile": [5, 1], "volume": 10.0, "capacity": 10.0,
///                   "pressure": 1.0, "is_source": true }, ... ],
///   "inlet_index": 3,
///   "machine_tile": [8, 0],
///   "machine_recipe": "ketchup_mix",
///   "units_per_craft": 2
/// }
/// ```
///
/// `segments` is in flow order, source end first; adjacent entries are
/// physically adjacent pipes. `inlet_index` indexes into `segments` and
/// marks the segment the machine actually draws from — that is the one
/// whose level drops when a craft starts. `machine_tile`/`machine_recipe`
/// identify the machine being fed (its tile is adjacent to the inlet
/// segment's), and `units_per_craft` is how much the recipe consumes per
/// craft.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FluidSnapshot {
    pub fluid: FluidId,
    pub total_volume: f64,
    pub total_capacity: f64,
    /// Network-wide fill fraction, `total_volume / total_capacity`.
    pub pressure: f64,
    pub segments: Vec<PipeSnapshot>,
    pub inlet_index: usize,
    pub machine_tile: TileCoord,
    pub machine_recipe: RecipeId,
    pub units_per_craft: u32,
}

/// What occupies a single tile, for `query_tile`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TileSnapshot {
    Belt {
        items_here: Vec<ItemId>,
    },
    /// A tile occupied by a machine. **Any** tile inside the machine's
    /// footprint resolves to this, not only its origin — which is what makes
    /// hovering a multi-tile machine work anywhere on it.
    ///
    /// JSON:
    /// `{"kind": "machine", "machine_kind": "cooking_vat",
    ///   "footprint": {"width_tiles": 2, "height_tiles": 2},
    ///   "origin_tile": [14, 0], "recipe": "ketchup",
    ///   "process": [120, 240]}`
    ///
    /// `origin_tile` is the footprint's minimum-X/minimum-Y corner (see
    /// [`super::machines`]'s module doc), so a caller that hovered a
    /// non-origin tile can still identify *which* machine it hit and where
    /// that machine actually starts. `machine_kind` is spelled out rather
    /// than reusing `kind`, which this enum already spends on its own
    /// variant tag.
    Machine {
        machine_kind: String,
        footprint: Footprint,
        origin_tile: TileCoord,
        recipe: RecipeId,
        process: Option<(u64, u64)>,
    },
    /// A pipe tile (milestone 6). JSON:
    /// `{"kind": "pipe", "fluid": "vinegar", "volume": 7.5, "capacity": 10.0,
    ///   "pressure": 0.75, "is_source": false, "is_inlet": true}` —
    /// enough for a hover tooltip without a second call.
    Pipe {
        fluid: FluidId,
        volume: f64,
        capacity: f64,
        pressure: f64,
        is_source: bool,
        is_inlet: bool,
    },
}

/// Static (non-per-tick) description of one pipeline stage's placement —
/// enough for Godot to build its `TileMap` generically from however many
/// stages `recipes.ron` currently defines, instead of hardcoding stage
/// geometry client-side (mirrors the same "Godot draws what the sim says
/// exists" principle `chunk_snapshot` already follows for dynamic state).
///
/// JSON shape (`snake_case`, one object per stage in chain order):
///
/// ```json
/// {
///   "recipe": "tomato_paste",
///   "belt_start_tile": [0, 0],
///   "belt_length_tiles": 4,
///   "machine_tile": [4, 0],
///   "machine_kind": "crusher",
///   "machine_footprint": { "width_tiles": 1, "height_tiles": 1 }
/// }
/// ```
///
/// `machine_kind`/`machine_footprint` are new in the "Machines improvement"
/// milestone; every field that existed before is unchanged in both name and
/// meaning, so an existing consumer keeps working untouched.
/// **`machine_tile` is the footprint's origin** — its minimum-X/minimum-Y
/// corner, and the tile the belt feeds into (see [`super::machines`]'s module
/// doc). The machine covers `x in [machine_tile.x, machine_tile.x +
/// width_tiles)`, `y in [machine_tile.y, machine_tile.y + height_tiles)`, and
/// the next stage's `belt_start_tile` is the tile immediately east of that
/// rectangle.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageLayout {
    pub recipe: RecipeId,
    pub belt_start_tile: TileCoord,
    pub belt_length_tiles: i32,
    /// Origin tile of this stage's machine — see this type's doc.
    pub machine_tile: TileCoord,
    /// Machine kind id from `data/machines.ron` (e.g. "crusher").
    pub machine_kind: String,
    /// How many tiles that machine covers, anchored at `machine_tile`.
    pub machine_footprint: Footprint,
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
    /// Money + the hardcoded contract's delivery state (ARCHITECTURE.md §9
    /// milestone 5, GAMEPLAY.md §6) — see `economy` module doc for why this
    /// lives here as a plain field rather than a new ECS component.
    economy: Economy,
    /// Milestone 6's single fixed pipe run (ARCHITECTURE.md §9 item 6,
    /// §3.4), or `None` when no recipe in `recipes.ron` declares
    /// `fluid_inputs`. A plain field, not an ECS component, for exactly the
    /// same reason `economy` is one: there is precisely one of these at
    /// this milestone's scope, so ECS machinery would buy nothing. It also
    /// keeps `Machine::tick`'s `&mut FluidNetwork` argument borrow-checkable
    /// alongside a `&mut Machine` fetched out of `ecs` (disjoint fields).
    fluid: Option<FluidNetwork>,
    /// Index into `stages` of the machine the pipe run feeds — `Some` iff
    /// `fluid` is.
    fluid_stage: Option<usize>,
}

const BELT_LENGTH_TILES: i32 = 4;
const ITEM_MIN_SPACING_TILES: f64 = 0.25;
const BELT_SPEED_TILES_PER_TICK: f64 = 0.02;
const MACHINE_INPUT_CAPACITY: u32 = 8;
const MACHINE_OUTPUT_CAPACITY: u32 = 8;

/// Milestone 6's fixed pipe run (ARCHITECTURE.md §9 item 6): how many pipe
/// tiles, including the source at one end and the machine inlet at the
/// other. Long enough that pressure equalization is doing visible work
/// carrying fluid down the run, short enough to stay a demo.
const FLUID_PIPE_RUN_TILES: i32 = 4;
/// Which tile row the production chain itself sits on. Every belt tile and
/// every machine footprint's *origin* row is this; a machine taller than one
/// tile grows northward from here.
const CHAIN_ROW_Y: i32 = 0;
/// Volume one pipe tile holds, in whole fluid units.
const PIPE_SEGMENT_CAPACITY: i32 = 10;

impl World {
    /// Builds one stage per recipe in `recipes.ordered()`, in file order,
    /// laid out left to right — see the module doc for why this order (not
    /// any Rust-side description) is what defines the chain. Machine sizes
    /// come from the shipped `data/machines.ron`; use
    /// [`World::with_machines`] to supply a different set.
    pub fn new(recipes: RecipeRegistry) -> Self {
        Self::with_machines(recipes, MachineRegistry::bundled())
    }

    /// [`World::new`], but with an explicit machine-kind registry instead of
    /// the bundled `data/machines.ron`. This is the seam that lets a test
    /// exercise a footprint no shipped kind has (every shipped kind is 1x1,
    /// so the width math would otherwise be unobservable) without inventing
    /// fake content in the real data file.
    pub fn with_machines(recipes: RecipeRegistry, machines: MachineRegistry) -> Self {
        let ordered = recipes.ordered();
        assert!(
            !ordered.is_empty(),
            "recipes.ron must define at least one recipe for World::new to build a chain from"
        );
        validate_chain(ordered);
        validate_fluid_scope(ordered);
        // Every recipe's machine kind must exist in machines.ron before any
        // layout arithmetic runs — a missing kind has no footprint, and
        // guessing one (1x1) would silently mis-place the entire rest of the
        // chain. Same construction-time-panic rule as `validate_chain`.
        if let Err(e) = machines.validate_covers(&recipes) {
            panic!("machine content error: {e}");
        }

        let mut ecs = hecs::World::new();
        let mut stages = Vec::with_capacity(ordered.len());

        // Stage placement accumulates each machine's own width rather than
        // multiplying a fixed per-stage span: belt tiles, then this stage's
        // machine footprint, then the next stage starts on the very next
        // tile. With every kind 1x1 this reproduces the old
        // `i * (BELT_LENGTH_TILES + 1)` exactly.
        let mut next_stage_x = 0;
        for recipe in ordered.iter() {
            let footprint = machines
                .footprint_of(&recipe.machine)
                .expect("validate_covers ran above, so every kind is known");

            let belt_start_x = next_stage_x;
            let machine_origin_x = belt_start_x + BELT_LENGTH_TILES;
            next_stage_x = machine_origin_x + footprint.width_tiles;

            let belt_entity = ecs.spawn((
                Position {
                    tile: (belt_start_x, CHAIN_ROW_Y),
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
                // The origin tile: the footprint's min-X/min-Y corner, and
                // the tile this stage's belt feeds into.
                Position {
                    tile: (machine_origin_x, CHAIN_ROW_Y),
                },
                MachinePlacement {
                    kind: recipe.machine.clone(),
                    footprint,
                },
                Machine::new(recipe.id.clone(), MACHINE_INPUT_CAPACITY, MACHINE_OUTPUT_CAPACITY),
            ));

            stages.push(Stage {
                belt_entity,
                machine_entity,
            });
        }

        // The fluid half of the world (milestone 6), also derived purely
        // from `recipes.ron`: whichever recipe declares `fluid_inputs` gets
        // a pipe run built up to its machine, carrying the fluid that
        // recipe asks for. No fluid id, no recipe id and no stage index is
        // written down in Rust — same data-driven rule the belt chain
        // already follows (ARCHITECTURE.md §3.6/§10). A file with no fluid
        // recipe simply has no pipes.
        let fluid_stage = ordered.iter().position(|r| !r.fluid_inputs.is_empty());
        let fluid = fluid_stage.map(|i| {
            let machine_entity = stages[i].machine_entity;
            let machine_origin = ecs.get::<&Position>(machine_entity).unwrap().tile;
            let machine_footprint = ecs.get::<&MachinePlacement>(machine_entity).unwrap().footprint;
            // The pipe run sits on the row immediately *north* of the
            // machine it feeds, so its inlet is always edge-adjacent to that
            // machine's footprint no matter how tall the machine is. For a
            // 1x1 machine on row 0 that is row 1 — exactly where milestone
            // 6 put it — and for a taller one it rises with the machine
            // instead of being buried inside it.
            let pipe_row_y = machine_origin.1 + machine_footprint.height_tiles;
            let capacity = Fx::from_num(PIPE_SEGMENT_CAPACITY);
            // Laid out left to right ending directly beside the machine it
            // feeds: the source is the far end, the inlet is the tile
            // adjacent to the machine.
            let segments: Vec<PipeSegment> = (0..FLUID_PIPE_RUN_TILES)
                .map(|n| {
                    let x = machine_origin.0 - (FLUID_PIPE_RUN_TILES - 1 - n);
                    PipeSegment::new((x, pipe_row_y), capacity, n == 0)
                })
                .collect();
            let inlet = segments.len() - 1;
            FluidNetwork::new(ordered[i].fluid_inputs[0].0.clone(), segments, inlet)
        });

        let world = Self {
            ecs,
            recipes,
            tick_count: 0,
            stages,
            belt_speed: Fx::from_num(BELT_SPEED_TILES_PER_TICK),
            dirty: Vec::new(),
            economy: Economy::new(),
            fluid,
            fluid_stage,
        };
        world.validate_placement();
        world
    }

    /// Construction-time check that the laid-out world is physically
    /// coherent: no machine footprint overlaps another machine, a belt tile,
    /// or a pipe tile. With the accumulate-widths layout above this can't
    /// happen for the shipped content, but a future change to the layout
    /// rule or the pipe run would otherwise fail as a subtly wrong render
    /// rather than an error — so it panics here, at construction, the same
    /// way `validate_chain` does for a broken recipe chain.
    fn validate_placement(&self) {
        let mut claims: Vec<(TileCoord, String)> = Vec::new();

        for (i, stage) in self.stages.iter().enumerate() {
            let belt_tile = self.position_of(stage.belt_entity);
            let span = self.ecs.get::<&BeltSpan>(stage.belt_entity).unwrap();
            for dx in 0..span.length_tiles {
                claims.push(((belt_tile.0 + dx, belt_tile.1), format!("stage {i}'s belt")));
            }

            let placement = self.placement_of(stage.machine_entity);
            let origin = self.position_of(stage.machine_entity);
            for tile in placement.footprint.tiles(origin) {
                claims.push((tile, format!("stage {i}'s '{}' machine", placement.kind)));
            }
        }

        if let Some(fluid) = &self.fluid {
            for segment in fluid.segments() {
                claims.push((segment.tile, "the pipe run".to_string()));
            }
        }

        assert_no_tile_conflicts(&claims);
    }

    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }

    /// Current money balance (ARCHITECTURE.md §3.9: fixed-point, never a
    /// float, since it's shared/persisted simulation state).
    pub fn money(&self) -> Fx {
        self.economy.money
    }

    /// Presentation-facing money + contract-status snapshot (ARCHITECTURE.md
    /// §4/§5) — always current, unlike `chunk_snapshot` there's no dirty-
    /// tracking to consult first, since there's exactly one of this per
    /// world rather than one per chunk.
    pub fn economy_snapshot(&self) -> EconomySnapshot {
        self.economy.snapshot()
    }

    /// The fluid network's live state, or `None` if this world has no pipe
    /// run (no recipe declares `fluid_inputs`). Always current, never
    /// dirty-tracked — see [`FluidSnapshot`]'s doc for the full JSON shape
    /// and for why presentation should poll this rather than wait for a
    /// dirty chunk.
    pub fn fluid_snapshot(&self) -> Option<FluidSnapshot> {
        let fluid = self.fluid.as_ref()?;
        let stage = self.stages[self.fluid_stage.expect("fluid_stage is Some iff fluid is")];
        let machine = self.ecs.get::<&Machine>(stage.machine_entity).unwrap();
        let units_per_craft = self
            .recipes
            .get(&machine.recipe)
            .and_then(|r| r.fluid_inputs.first().map(|(_, units)| *units))
            .unwrap_or(0);
        let total_capacity = fluid.total_capacity();
        Some(FluidSnapshot {
            fluid: fluid.fluid.clone(),
            total_volume: fluid.total_volume().to_num::<f64>(),
            total_capacity: total_capacity.to_num::<f64>(),
            pressure: if total_capacity == Fx::ZERO {
                0.0
            } else {
                (fluid.total_volume() / total_capacity).to_num::<f64>()
            },
            segments: fluid.segments().iter().map(pipe_snapshot).collect(),
            inlet_index: fluid.inlet_index(),
            machine_tile: self.position_of(stage.machine_entity),
            machine_recipe: machine.recipe.clone(),
            units_per_craft,
        })
    }

    fn position_of(&self, entity: Entity) -> TileCoord {
        self.ecs.get::<&Position>(entity).unwrap().tile
    }

    /// A machine entity's kind + footprint (`data/machines.ron`), cloned out
    /// of the ECS so the caller isn't holding a borrow on `self.ecs`.
    fn placement_of(&self, entity: Entity) -> MachinePlacement {
        (*self.ecs.get::<&MachinePlacement>(entity).unwrap()).clone()
    }

    fn mark_dirty(&mut self, tile: TileCoord) {
        let chunk = chunk_of(tile);
        if !self.dirty.contains(&chunk) {
            self.dirty.push(chunk);
        }
    }

    /// Marks every chunk a machine's footprint touches, not just the one its
    /// origin tile is in — a machine straddling a chunk boundary changes
    /// what both chunks' snapshots contain, so both have to be re-read.
    fn mark_machine_dirty(&mut self, origin: TileCoord, footprint: Footprint) {
        for chunk in footprint_chunks(origin, footprint) {
            if !self.dirty.contains(&chunk) {
                self.dirty.push(chunk);
            }
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

        // Fluid first, so a machine starting a craft below sees this tick's
        // equalized pressure rather than last tick's (ARCHITECTURE.md §3.4).
        // Note this deliberately does NOT mark any chunk dirty: fluid moves
        // on nearly every tick, and dirtying a chunk every tick would defeat
        // the point of dirty tracking (§4's delta granularity). Presentation
        // polls `fluid_snapshot` instead, exactly as it polls
        // `economy_snapshot` — see `FluidSnapshot`'s own doc.
        if let Some(fluid) = self.fluid.as_mut() {
            fluid.tick();
        }

        for i in 0..self.stages.len() {
            let stage = self.stages[i];
            let belt_tile = self.position_of(stage.belt_entity);
            let machine_tile = self.position_of(stage.machine_entity);
            let machine_footprint = self.placement_of(stage.machine_entity).footprint;

            let belt_changed = {
                let mut belt = self.ecs.get::<&mut BeltSegment>(stage.belt_entity).unwrap();
                belt.tick(self.belt_speed)
            };
            let machine_changed = {
                let mut machine = self.ecs.get::<&mut Machine>(stage.machine_entity).unwrap();
                // Only the one plumbed-in stage sees the network; every
                // other machine is handed `None` and behaves exactly as it
                // did before milestone 6.
                let fluid = if self.fluid_stage == Some(i) {
                    self.fluid.as_mut()
                } else {
                    None
                };
                machine.tick(self.tick_count, &self.recipes, fluid)
            };
            let transferred =
                self.try_transfer_belt_to_machine(stage.belt_entity, stage.machine_entity);

            if belt_changed || transferred {
                self.mark_dirty(belt_tile);
            }
            if machine_changed || transferred {
                self.mark_machine_dirty(machine_tile, machine_footprint);
            }
        }

        for i in 0..self.stages.len().saturating_sub(1) {
            let from_machine = self.stages[i].machine_entity;
            let to_belt = self.stages[i + 1].belt_entity;
            let machine_tile = self.position_of(from_machine);
            let machine_footprint = self.placement_of(from_machine).footprint;
            let belt_tile = self.position_of(to_belt);

            let transferred = self.try_transfer_machine_output_to_belt(from_machine, to_belt);
            if transferred {
                self.mark_machine_dirty(machine_tile, machine_footprint);
                self.mark_dirty(belt_tile);
            }
        }

        // Contract cadence check (GAMEPLAY.md §6) — not tied to any tile/
        // chunk, so it doesn't participate in dirty-chunk tracking; Godot
        // reads economy state via its own always-fresh snapshot instead
        // (see `economy_snapshot`).
        self.economy.tick(self.tick_count);
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

            // A machine is reported for every chunk its footprint overlaps,
            // not only the chunk holding its origin tile — see
            // `MachineSnapshot`'s doc for the de-duplication that implies
            // for a consumer stitching several chunks together.
            let machine_tile = self.position_of(stage.machine_entity);
            let placement = self.placement_of(stage.machine_entity);
            if footprint_chunks(machine_tile, placement.footprint).contains(&id) {
                let machine = self.ecs.get::<&Machine>(stage.machine_entity).unwrap();
                snapshot.machines.push(MachineSnapshot {
                    tile: machine_tile,
                    kind: placement.kind,
                    footprint: placement.footprint,
                    recipe: machine.recipe.clone(),
                    process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
                    input: machine.input.iter().cloned().collect(),
                    output: machine.output.iter().cloned().collect(),
                });
            }
        }

        if let Some(fluid) = &self.fluid {
            for segment in fluid.segments() {
                if chunk_of(segment.tile) == id {
                    snapshot.pipes.push(pipe_snapshot(segment));
                }
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
            let mut chunks = vec![chunk_of(self.position_of(stage.belt_entity))];
            // Every chunk the machine's footprint reaches, so a wide machine
            // straddling a boundary doesn't leave the far chunk unlisted.
            chunks.extend(footprint_chunks(
                self.position_of(stage.machine_entity),
                self.placement_of(stage.machine_entity).footprint,
            ));
            for chunk in chunks {
                if !ids.contains(&chunk) {
                    ids.push(chunk);
                }
            }
        }
        if let Some(fluid) = &self.fluid {
            for segment in fluid.segments() {
                let chunk = chunk_of(segment.tile);
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
                let placement = self.placement_of(stage.machine_entity);
                let recipe = self.ecs.get::<&Machine>(stage.machine_entity).unwrap().recipe.clone();
                StageLayout {
                    recipe,
                    belt_start_tile: belt_tile,
                    belt_length_tiles: span.length_tiles,
                    machine_tile,
                    machine_kind: placement.kind,
                    machine_footprint: placement.footprint,
                }
            })
            .collect()
    }

    /// What occupies `pos`, if anything (ARCHITECTURE.md §4). A machine
    /// resolves from **any** tile in its footprint, not just its origin —
    /// see [`TileSnapshot::Machine`] for the shape that returns and why a
    /// hover system needs it.
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
            let placement = self.placement_of(stage.machine_entity);
            if placement.footprint.contains(machine_tile, pos) {
                let machine = self.ecs.get::<&Machine>(stage.machine_entity).unwrap();
                return Some(TileSnapshot::Machine {
                    machine_kind: placement.kind,
                    footprint: placement.footprint,
                    origin_tile: machine_tile,
                    recipe: machine.recipe.clone(),
                    process: machine.process.map(|p| (p.start_tick, p.required_ticks)),
                });
            }
        }

        if let Some(fluid) = &self.fluid {
            for (i, segment) in fluid.segments().iter().enumerate() {
                if segment.tile == pos {
                    return Some(TileSnapshot::Pipe {
                        fluid: fluid.fluid.clone(),
                        volume: segment.volume.to_num::<f64>(),
                        capacity: segment.capacity.to_num::<f64>(),
                        pressure: segment.pressure().to_num::<f64>(),
                        is_source: segment.is_source,
                        is_inlet: i == fluid.inlet_index(),
                    });
                }
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
                let machine_footprint = self.placement_of(last.machine_entity).footprint;
                let mut machine = self.ecs.get::<&mut Machine>(last.machine_entity).unwrap();
                let held: Vec<(ItemId, u32)> = machine.output.iter().cloned().collect();
                if held.is_empty() {
                    return;
                }
                for (item, qty) in &held {
                    machine.output.remove(item, *qty);
                }
                drop(machine);
                // Collected product becomes delivered stock, checked against
                // the active contract at the next cadence boundary
                // (GAMEPLAY.md §6) — this is the "delivery mechanism"
                // milestone 5 adds on top of milestone 4's plain collect.
                for (item, qty) in &held {
                    self.economy.deposit(item, *qty);
                }
                self.mark_machine_dirty(machine_tile, machine_footprint);
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

/// Panics if two `(tile, description)` claims name the same tile, naming
/// both claimants. Split out of `World::validate_placement` so the rule can
/// be tested against a deliberately-broken layout: the accumulate-widths
/// arithmetic in `World::with_machines` makes a real conflict unreachable
/// today, which is precisely why the check needs its own test rather than
/// relying on some data file eventually tripping it.
fn assert_no_tile_conflicts(claims: &[(TileCoord, String)]) {
    for (i, (tile, what)) in claims.iter().enumerate() {
        if let Some((_, other)) = claims[..i].iter().find(|(t, _)| t == tile) {
            panic!(
                "world layout error: tile {tile:?} is claimed by both {other} and {what} — \
                 a machine footprint (data/machines.ron) must not overlap another stage, \
                 its own chain's belts, or the pipe run"
            );
        }
    }
}

/// Every chunk (ARCHITECTURE.md §3.1's 32x32 grid) a machine footprint
/// placed at `origin` overlaps, in a deterministic row-major order with the
/// origin's own chunk first.
///
/// A footprint smaller than a chunk usually yields exactly one id; a
/// footprint straddling a chunk boundary yields two or four. This is the one
/// place that fan-out is computed, so dirty tracking, `all_chunks` and
/// `chunk_snapshot` cannot disagree about which chunks a machine belongs to.
/// The multi-chunk case is *handled*, not rejected — see
/// [`MachineSnapshot`]'s doc for what that means for a consumer.
fn footprint_chunks(origin: TileCoord, footprint: Footprint) -> Vec<ChunkId> {
    let far = (
        origin.0 + footprint.width_tiles - 1,
        origin.1 + footprint.height_tiles - 1,
    );
    let (min_cx, min_cy) = chunk_of(origin);
    let (max_cx, max_cy) = chunk_of(far);
    debug_assert!(min_cx <= max_cx && min_cy <= max_cy, "footprint extents must be positive");
    let mut chunks = Vec::with_capacity(
        (((max_cx - min_cx + 1) as i64) * ((max_cy - min_cy + 1) as i64)) as usize,
    );
    for cy in min_cy..=max_cy {
        for cx in min_cx..=max_cx {
            chunks.push((cx, cy));
        }
    }
    // The origin is the footprint's min-X/min-Y corner, so row-major from
    // the minimum chunk already puts the origin's own chunk first.
    debug_assert_eq!(chunks[0], chunk_of(origin));
    chunks
}

fn pipe_snapshot(segment: &PipeSegment) -> PipeSnapshot {
    PipeSnapshot {
        tile: segment.tile,
        volume: segment.volume.to_num::<f64>(),
        capacity: segment.capacity.to_num::<f64>(),
        pressure: segment.pressure().to_num::<f64>(),
        is_source: segment.is_source,
    }
}

/// Enforces milestone 6's deliberately narrow fluid scope, the same way
/// `validate_chain` enforces milestone 4's: `World` builds exactly *one*
/// fixed pipe run, feeding exactly one machine, carrying exactly one fluid.
/// A data file that asks for more than that would otherwise get it silently
/// ignored (a second fluid recipe's machine would simply never start, with
/// no clue why) — so it panics at construction time instead, with the
/// reason and what would have to change.
fn validate_fluid_scope(ordered: &[Recipe]) {
    let with_fluid: Vec<&Recipe> = ordered
        .iter()
        .filter(|r| !r.fluid_inputs.is_empty())
        .collect();
    assert!(
        with_fluid.len() <= 1,
        "recipes.ron fluid error: {} recipes declare fluid_inputs ({:?}), but milestone 6 \
         (ARCHITECTURE.md §9 item 6) builds exactly one fixed pipe run feeding one machine — \
         supporting a second one needs multiple FluidNetworks and a real placement description, \
         not another hardcoded run",
        with_fluid.len(),
        with_fluid.iter().map(|r| &r.id).collect::<Vec<_>>()
    );
    if let Some(recipe) = with_fluid.first() {
        assert_eq!(
            recipe.fluid_inputs.len(),
            1,
            "recipes.ron fluid error: recipe '{}' declares {} fluid inputs, but a machine is \
             plumbed into exactly one network at this milestone's scope",
            recipe.id,
            recipe.fluid_inputs.len()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footprint_chunks_is_one_chunk_when_the_footprint_fits_inside_one() {
        assert_eq!(
            footprint_chunks((4, 0), Footprint::ONE_BY_ONE),
            vec![(0, 0)]
        );
        assert_eq!(
            footprint_chunks(
                (29, 0),
                Footprint {
                    width_tiles: 3,
                    height_tiles: 1
                }
            ),
            vec![(0, 0)],
            "29..=31 is still inside chunk 0 — the boundary is at x = 32"
        );
    }

    #[test]
    fn footprint_chunks_fans_out_across_a_straddled_boundary() {
        // 30..=33 crosses x = 32; 30..=33 x 31..=32 crosses both axes.
        assert_eq!(
            footprint_chunks(
                (30, 0),
                Footprint {
                    width_tiles: 4,
                    height_tiles: 1
                }
            ),
            vec![(0, 0), (1, 0)]
        );
        assert_eq!(
            footprint_chunks(
                (30, 31),
                Footprint {
                    width_tiles: 4,
                    height_tiles: 2
                }
            ),
            vec![(0, 0), (1, 0), (0, 1), (1, 1)],
            "row-major, origin's own chunk first"
        );
    }

    /// The real layout math can't produce an overlap (see
    /// `assert_no_tile_conflicts`'s doc), so the rule is checked directly
    /// against a hand-built conflicting claim list.
    #[test]
    #[should_panic(expected = "claimed by both")]
    fn overlapping_tile_claims_panic_with_both_claimants_named() {
        assert_no_tile_conflicts(&[
            ((4, 0), "stage 0's 'crusher' machine".to_string()),
            ((5, 0), "stage 1's belt".to_string()),
            ((4, 0), "the pipe run".to_string()),
        ]);
    }

    #[test]
    fn non_overlapping_tile_claims_are_accepted() {
        assert_no_tile_conflicts(&[
            ((4, 0), "stage 0's 'crusher' machine".to_string()),
            ((4, 1), "the pipe run".to_string()),
            ((5, 0), "stage 1's belt".to_string()),
        ]);
    }
}
