//! Shared primitive types for the simulation core.
//!
//! ARCHITECTURE.md §3.9: any value that is shared across systems, persisted,
//! or could ever be replayed/compared across machines must be fixed-point,
//! never float. `Fx` is that one fixed-point type, threaded through belts
//! (§3.3) and timed processes (§3.5). Floats only ever appear at the very
//! edge of the simulation, when a snapshot is built for Godot to render
//! (see `world::ChunkSnapshot`) — that conversion is one-way and never feeds
//! back into simulation state.

/// Fixed-point type for simulation state: 16 fractional bits is enough
/// sub-tile precision for belt item positions at this project's tile scale,
/// with headroom (16 integer bits) far beyond any world size this milestone
/// needs.
pub type Fx = fixed::types::I16F16;

/// A raw or finished item's content id (e.g. "tomato", "tomato_sauce").
/// Matches the string ids used in `recipes.ron` (ARCHITECTURE.md §3.6).
pub type ItemId = String;

/// A recipe's content id (e.g. "tomato_sauce"), matching `Recipe.id` in
/// `recipes.ron`.
pub type RecipeId = String;

/// Tile coordinate in the world grid (ARCHITECTURE.md §3.1).
pub type TileCoord = (i32, i32);

/// Chunk grid coordinate — tiles are grouped into 32x32 chunks per
/// ARCHITECTURE.md §3.1.
pub type ChunkId = (i32, i32);

pub const CHUNK_SIZE: i32 = 32;

/// Which chunk a tile belongs to.
pub fn chunk_of(tile: TileCoord) -> ChunkId {
    (tile.0.div_euclid(CHUNK_SIZE), tile.1.div_euclid(CHUNK_SIZE))
}

/// Packs a `ChunkId` into a single `i64` so it can cross the `gdext`
/// boundary as a plain integer instead of a second compound type.
pub fn encode_chunk_id(id: ChunkId) -> i64 {
    ((id.0 as i64) << 32) | (id.1 as u32 as i64)
}

pub fn decode_chunk_id(packed: i64) -> ChunkId {
    ((packed >> 32) as i32, (packed & 0xFFFF_FFFF) as u32 as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_id_roundtrips_through_encoding() {
        for id in [(0, 0), (-1, 5), (12345, -6789)] {
            assert_eq!(decode_chunk_id(encode_chunk_id(id)), id);
        }
    }
}
