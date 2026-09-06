//! Pure simulation core (ARCHITECTURE.md §3) — no `godot` dependency
//! anywhere in this module tree. `lib.rs` is the only place that touches
//! `gdext`; everything here is exercisable from plain `cargo test`
//! (ARCHITECTURE.md §7).

pub mod belt;
pub mod inventory;
pub mod machine;
pub mod recipe;
pub mod types;
pub mod world;

pub use recipe::RecipeRegistry;
pub use types::{decode_chunk_id, encode_chunk_id};
pub use world::{ChunkSnapshot, PlayerCommand, TileSnapshot, World, command_from_json};
