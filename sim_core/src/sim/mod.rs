//! Pure simulation core (ARCHITECTURE.md §3) — no `godot` dependency
//! anywhere in this module tree. `lib.rs` is the only place that touches
//! `gdext`; everything here is exercisable from plain `cargo test`
//! (ARCHITECTURE.md §7).

pub mod belt;
pub mod economy;
pub mod fluid;
pub mod hopper;
pub mod inventory;
pub mod machine;
pub mod machines;
pub mod recipe;
pub mod types;
pub mod world;

pub use economy::{Contract, Economy, EconomySnapshot};
pub use fluid::{
    FLUID_TANK_CAPACITY, FLUID_TANK_FILL_AMOUNT, FLUID_TANK_KIND, FluidNetwork, FluidTank,
    PipeSegment,
};
pub use hopper::{HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, HOPPER_KIND, Hopper};
pub use machines::{Footprint, MachineKind, MachineRegistry};
pub use recipe::{Recipe, RecipeRegistry};
pub use types::{decode_chunk_id, encode_chunk_id};
pub use world::{
    BeltSnapshot, ChunkSnapshot, FluidSnapshot, FluidTankSnapshot, HopperSnapshot, MachineSnapshot,
    PipeSnapshot, PlayerCommand, StageLayout, TileSnapshot, World, command_from_json,
};
