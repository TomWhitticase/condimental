//! Pure simulation core (ARCHITECTURE.md §3) — no `godot` dependency
//! anywhere in this module tree. `lib.rs` is the only place that touches
//! `gdext`; everything here is exercisable from plain `cargo test`
//! (ARCHITECTURE.md §7).

pub mod belt;
pub mod contracts;
pub mod dock;
pub mod economy;
pub mod fluid;
pub mod hopper;
pub mod ingredients;
pub mod inventory;
pub mod machine;
pub mod machines;
pub mod recipe;
pub mod reference;
pub mod save;
pub mod shop;
pub mod splitter;
pub mod types;
pub mod world;

pub use contracts::{Contract, ContractRegistry};
pub use dock::{DOCK_CAPACITY, DOCK_KIND, Dock};
pub use economy::{
    ActiveContract, ContractEvent, ContractOfferSnapshot, ContractSnapshot, Economy,
    EconomySnapshot, STARTING_MONEY, SignRefusal,
};
pub use fluid::{
    FLUID_TANK_CAPACITY, FLUID_TANK_FILL_AMOUNT, FLUID_TANK_KIND, FluidNetwork, FluidTank,
    FluidTap, PipeSegment,
};
pub use hopper::{HOPPER_CAPACITY, HOPPER_FILL_AMOUNT, HOPPER_KIND, Hopper};
pub use ingredients::{
    DEFAULT_BUY_QUANTITY, IngredientCatalog, IngredientEntry, IngredientSnapshot, Ingredients,
    IngredientsSnapshot, OwnedIngredients, raw_ingredient_ids,
};
pub use machines::{Footprint, MachineKind, MachineRegistry};
pub use recipe::{Recipe, RecipeRegistry};
pub use reference::{
    reference_chain, reference_chain_with, reference_chain_with_budget,
    reference_chain_with_content_and_budget,
};
pub use save::{SAVE_SCHEMA_VERSION, SaveData, SaveError};
pub use shop::{
    DEFAULT_PURCHASE_QUANTITY, OwnedEquipment, Shop, ShopCatalog, ShopEntry, ShopEntrySnapshot,
    ShopSnapshot,
};
pub use splitter::{SPLITTER_KIND, Side, Splitter};
pub use types::{Direction, Fx, decode_chunk_id, encode_chunk_id};
pub use machine::{
    FULL_LOAD_PERCENT, LOAD_BUCKET_TICKS, LOAD_BUCKETS, LOAD_WINDOW_TICKS, LoadWindow, WorkState,
};
pub use world::{
    BELT_KIND, BeltSnapshot, ChunkSnapshot, ConveyorLinks, DockSnapshot, FluidSnapshot,
    FluidTankSnapshot, FluidTapSnapshot, HopperSnapshot, LinkInput, LinkKind, LinkOutput, Links,
    MachineLoadEntry, MachineLoadSnapshot, MachineSnapshot, PIPE_KIND, PipeConnection, PipeLinks,
    PipeSnapshot, PlacementSnapshot, PlayerCommand, SplitterSnapshot, Stall, StallKind,
    TileSnapshot, World, command_from_json,
};
