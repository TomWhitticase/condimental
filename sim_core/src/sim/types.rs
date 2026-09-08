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

/// A fluid's content id (e.g. "vinegar", "water") — the exact same
/// string-id pattern as `ItemId`, kept as its own alias because a fluid is
/// never interchangeable with a solid item: fluids live in a
/// `fluid::FluidNetwork` as a volume (ARCHITECTURE.md §3.4), items live in
/// an `Inventory` or on a belt as discrete units, and no code path should
/// be able to pass one where the other is meant without it reading wrong.
pub type FluidId = String;

/// Tile coordinate in the world grid (ARCHITECTURE.md §3.1).
pub type TileCoord = (i32, i32);

/// Which way a placed thing faces (the "Real Placement" epic's ticket 1).
///
/// Four-way, matching UI_UX.md §1's 90°-snap camera: there is no diagonal
/// and no free angle anywhere in this game, so a `Direction` is exactly the
/// four values a snap-rotation can produce and [`Direction::rotate_cw`] is
/// the only way the player ever changes one.
///
/// # Which way is which
///
/// [`TileCoord`] is `(x, y)`, and the two axes already had fixed meanings
/// before this type existed, so the names are chosen to match them rather
/// than the other way round:
///
/// - **`East` is `+X`** — the direction the production chain has always run
///   (`world.rs`'s layout, `machines.rs`'s "the belt ends on the tile
///   immediately west of the origin").
/// - **`North` is `+Y`** — the direction a machine's footprint grows in
///   height, and the row the pipe run has always sat on relative to the
///   machine it feeds ("the row immediately *north* of the machine",
///   `World::new` before this ticket).
///
/// **`East` is therefore the regression anchor for this whole epic**: every
/// behaviour the fixed chain had is what a world built entirely out of
/// east-facing placements does, tile for tile. It is also
/// [`Direction::default`], so a command that omits a direction gets the
/// behaviour the game had before directions existed.
///
/// # What a direction means on a thing
///
/// It is the side product *leaves* by — a belt tile carries items toward its
/// direction, a machine discharges from the face on its direction's side and
/// accepts input on the opposite face, a hopper unloads that way and a dock
/// is fed from that side. See [`super::machines`]'s module doc for the
/// footprint half of that convention (which edge of a rotated rectangle
/// counts as the inlet and which as the outlet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Default for Direction {
    /// `East` — see the type doc: it is the direction that reproduces every
    /// pre-placement behaviour exactly, so it is what an omitted direction
    /// on the JSON wire means.
    fn default() -> Self {
        Direction::East
    }
}

impl Direction {
    /// Every direction, in a fixed order — for tests and for anything that
    /// needs to enumerate them deterministically (ARCHITECTURE.md §3.9).
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// The unit tile offset this direction points along.
    pub fn offset(self) -> TileCoord {
        match self {
            Direction::North => (0, 1),
            Direction::East => (1, 0),
            Direction::South => (0, -1),
            Direction::West => (-1, 0),
        }
    }

    /// The tile one step this way from `tile`.
    pub fn step(self, tile: TileCoord) -> TileCoord {
        let (dx, dy) = self.offset();
        (tile.0 + dx, tile.1 + dy)
    }

    /// The next direction 90° clockwise — the one and only rotation the
    /// player can apply (`PlayerCommand::Rotate`), so four of these is a
    /// full turn back to where it started.
    ///
    /// Clockwise *on screen*, i.e. with `+X` right and `+Y` up: east turns
    /// to south, not to north.
    pub fn rotate_cw(self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    /// 180° — the face opposite this one. A building's inlet face is its
    /// direction's opposite (see [`super::machines`]'s module doc).
    pub fn opposite(self) -> Direction {
        self.rotate_cw().rotate_cw()
    }

    /// Whether this direction runs along the X axis. A 90° turn is exactly
    /// "this flipped", which is why a rotated footprint swaps its extents
    /// (see [`super::machines::Footprint::rotated`]).
    pub fn is_horizontal(self) -> bool {
        matches!(self, Direction::East | Direction::West)
    }
}

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

    /// Four clockwise turns is a full circle, from every starting point —
    /// the property `PlayerCommand::Rotate` relies on to be reversible
    /// without an "anticlockwise" command.
    #[test]
    fn four_clockwise_turns_return_to_the_start() {
        for dir in Direction::ALL {
            assert_eq!(dir.rotate_cw().rotate_cw().rotate_cw().rotate_cw(), dir);
            assert_eq!(dir.rotate_cw().rotate_cw(), dir.opposite());
            assert_ne!(dir.rotate_cw(), dir);
        }
    }

    /// The axis convention this whole epic is anchored on: `East` is `+X`
    /// (the direction the production chain has always run) and `North` is
    /// `+Y` (the direction a footprint grows in height).
    #[test]
    fn east_is_plus_x_and_north_is_plus_y() {
        assert_eq!(Direction::East.offset(), (1, 0));
        assert_eq!(Direction::North.offset(), (0, 1));
        assert_eq!(Direction::West.offset(), (-1, 0));
        assert_eq!(Direction::South.offset(), (0, -1));
        assert_eq!(Direction::default(), Direction::East);
        assert_eq!(Direction::East.step((4, 7)), (5, 7));
    }

    /// Clockwise on screen, with +X right and +Y up.
    #[test]
    fn clockwise_turns_east_toward_south() {
        assert_eq!(Direction::East.rotate_cw(), Direction::South);
        assert_eq!(Direction::South.rotate_cw(), Direction::West);
        assert_eq!(Direction::West.rotate_cw(), Direction::North);
        assert_eq!(Direction::North.rotate_cw(), Direction::East);
    }

    #[test]
    fn a_direction_serializes_as_its_lowercase_name() {
        assert_eq!(
            serde_json::to_string(&Direction::East).unwrap(),
            "\"east\""
        );
        assert_eq!(
            serde_json::from_str::<Direction>("\"south\"").unwrap(),
            Direction::South
        );
    }

    #[test]
    fn chunk_id_roundtrips_through_encoding() {
        for id in [(0, 0), (-1, 5), (12345, -6789)] {
            assert_eq!(decode_chunk_id(encode_chunk_id(id)), id);
        }
    }
}
