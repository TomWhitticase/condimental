//! Machine-kind content, loaded from `data/machines.ron` (ARCHITECTURE.md
//! §3.6) — the sibling of [`super::recipe`], built the same way for the same
//! reason: what a machine *is* is data, never hardcoded in simulation code
//! (ARCHITECTURE.md §10's "content changes should never require touching
//! Rust").
//!
//! Right now a kind carries exactly one property, its tile [`Footprint`],
//! because that is what [`super::world::World`]'s chain layout needs in
//! order to stop assuming every machine occupies exactly one tile. Adding
//! per-kind staff draw (GAMEPLAY.md §7), build cost (§4/§5) or an
//! equipment-tier gate later means adding fields to [`MachineKind`], not a
//! second data file.
//!
//! # The footprint origin convention
//!
//! This is the one thing every other layer (layout math, `query_tile`,
//! chunk snapshots, and eventually the Godot renderer) has to agree on, so
//! it is stated once, here, and nowhere contradicted:
//!
//! **A machine's origin tile is its footprint's minimum-X, minimum-Y
//! corner, and the footprint occupies `x in [origin.x, origin.x +
//! width_tiles)`, `y in [origin.y, origin.y + height_tiles)`.**
//!
//! Two consequences worth spelling out, because they are what make the
//! convention useful rather than arbitrary:
//!
//! 1. The origin is **the tile the stage's belt feeds into**. The production
//!    chain runs west to east along +X, so a stage's belt ends on the tile
//!    immediately west of the origin (`origin.x - 1`, same row). Widening a
//!    machine therefore never moves its input side — it grows eastward, into
//!    the space before the next stage.
//! 2. A single `TileCoord` plus these two numbers is a complete, unambiguous
//!    placement. Nothing anywhere needs to know a machine's centre, and no
//!    layer has to guess which corner a coordinate refers to.
//!
//! # Rotation: the same convention, plus a facing (the "Real Placement" epic)
//!
//! Until this epic every building faced the same way, because `World` laid
//! the chain out itself and the chain ran west to east. Now the player
//! places and rotates things, so a placement is `(origin, footprint,
//! direction)` and this module owns what those three mean together. It is
//! stated once, here, and every other layer — collision, adjacency, chunk
//! snapshots, `query_tile`, and the Godot renderer — refers to it:
//!
//! **1. The occupied rectangle is *always* anchored at `origin` as its
//! minimum-X/minimum-Y corner, whatever the direction.** Rotation never
//! moves the anchor; it only changes the rectangle's shape and which of its
//! edges is the inlet. This is what keeps every placement one tile plus two
//! numbers, and it is why [`Footprint::rotated`] is the whole of the
//! geometry.
//!
//! **2. A quarter turn swaps the extents.** At [`Direction::East`] or
//! `West` a kind occupies `width_tiles` x `height_tiles`; at `North` or
//! `South` it occupies `height_tiles` x `width_tiles`, because the machine's
//! own width now lies along the world's Y axis. A 1x1 kind is therefore
//! rotation-invariant in shape (though not in facing), and the shipped
//! content has exactly one kind for which this is observable — the 2x2
//! cooking vat is *square*, so in fact no shipped kind changes shape when
//! turned. That is content, not a limit: the mechanism is exercised by
//! deliberately oblong test kinds, exactly the way ticket 1 of the
//! "Machines improvement" epic exercised widths no shipped kind had.
//!
//! **3. `direction` is the side product LEAVES by.** The rectangle edge on
//! the direction's side is the **outlet face**
//! ([`Footprint::outlet_tiles`]); the edge opposite it is the **inlet
//! face** ([`Footprint::inlet_tiles`]) — the tiles a belt has to point into
//! to feed this machine. For `East` that is the old convention verbatim:
//! the inlet face is the minimum-X column (the origin's own column, "the
//! tile the stage's belt feeds into") and the outlet face is the
//! maximum-X column, so an east-facing world reproduces the pre-placement
//! chain tile for tile.
//!
//! **4. The two side faces are neither.** A belt pointing into a machine's
//! flank does not feed it, and a machine never discharges sideways. That is
//! a deliberate simplification, not an oversight: it is what makes "which
//! way is this machine facing" a decision the player can actually see the
//! consequences of, and it costs nothing today (no kind has more than one
//! input or output stream).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::{Direction, TileCoord};

/// The schema version this build of sim_core understands. Bump this and
/// reject/migrate on mismatch (ARCHITECTURE.md §4) whenever [`MachineKind`]'s
/// shape changes in a way older data can't satisfy.
///
/// v1 is this file's first version, added by the "Machines improvement"
/// milestone. Mismatched data is **rejected, not migrated**, for exactly the
/// reason [`super::recipe::CURRENT_SCHEMA_VERSION`] gives: `machines.ron` is
/// bundled into the binary at compile time, so there is one such file and it
/// is updated in the same commit as this constant — a version mismatch means
/// data and binary have drifted apart, which deserves a loud error.
///
/// v2 (the staff pools MVP epic, 2026-09-12) added the **required** `staff`
/// list to every kind. Bumped rather than defaulted because an omitted crew
/// should be a loader error, not a silent zero — the precedent `recipes.ron`
/// set at v2 → v3 for `fluid_outputs`. A v1 file is rejected.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// How many tiles a machine kind occupies, in each axis. See this module's
/// doc for where the origin sits relative to this rectangle.
///
/// `i32` (not `u32`) to match [`super::types::TileCoord`]'s own component
/// type, so footprint arithmetic never needs a cast — the loader rejects
/// non-positive values (see [`MachineLoadError::DegenerateFootprint`]) so
/// nothing downstream has to defend against a zero or negative extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Footprint {
    pub width_tiles: i32,
    pub height_tiles: i32,
}

impl Footprint {
    /// The one-tile footprint every machine kind implicitly had before this
    /// milestone. Handy in tests; not a fallback — a kind missing from
    /// `machines.ron` is an error, never silently 1x1.
    pub const ONE_BY_ONE: Footprint = Footprint {
        width_tiles: 1,
        height_tiles: 1,
    };

    /// Whether `tile` falls inside this footprint placed at `origin` — the
    /// single definition of "is this tile part of that machine," used by
    /// `query_tile` and by the overlap validation in `world.rs`.
    pub fn contains(&self, origin: (i32, i32), tile: (i32, i32)) -> bool {
        tile.0 >= origin.0
            && tile.0 < origin.0 + self.width_tiles
            && tile.1 >= origin.1
            && tile.1 < origin.1 + self.height_tiles
    }

    /// Every tile this footprint covers when placed at `origin`, in
    /// row-major order (all of row `origin.y` west to east, then the next
    /// row north). Deterministic order, per ARCHITECTURE.md §3.9.
    ///
    /// This is the **unrotated** rectangle — the shape a kind has facing
    /// [`Direction::East`]. Anything the player placed should go through
    /// [`Footprint::tiles_facing`] instead, which is this applied to
    /// [`Footprint::rotated`].
    pub fn tiles(&self, origin: (i32, i32)) -> Vec<(i32, i32)> {
        let mut tiles = Vec::with_capacity((self.width_tiles * self.height_tiles) as usize);
        for dy in 0..self.height_tiles {
            for dx in 0..self.width_tiles {
                tiles.push((origin.0 + dx, origin.1 + dy));
            }
        }
        tiles
    }

    /// This footprint as it actually sits on the grid when facing `dir`: the
    /// extents swap on a quarter turn, and are unchanged on a half turn.
    /// See this module's doc, point 2 — this function *is* the whole of the
    /// rotation geometry, and every other rotation-aware helper below is
    /// written in terms of it.
    ///
    /// `East` (and `West`) return `self` unchanged, which is what makes an
    /// east-facing world byte-identical to the pre-placement chain.
    pub fn rotated(&self, dir: Direction) -> Footprint {
        if dir.is_horizontal() {
            *self
        } else {
            Footprint {
                width_tiles: self.height_tiles,
                height_tiles: self.width_tiles,
            }
        }
    }

    /// Every tile this footprint covers when placed at `origin` facing
    /// `dir` — the rotation-aware [`Footprint::tiles`], and the single
    /// definition of "which tiles does this placement claim" that collision
    /// checking, chunk fan-out and `query_tile` all use.
    pub fn tiles_facing(&self, origin: TileCoord, dir: Direction) -> Vec<TileCoord> {
        self.rotated(dir).tiles(origin)
    }

    /// Whether `tile` falls inside this footprint placed at `origin` facing
    /// `dir` — the rotation-aware [`Footprint::contains`].
    pub fn contains_facing(&self, origin: TileCoord, dir: Direction, tile: TileCoord) -> bool {
        self.rotated(dir).contains(origin, tile)
    }

    /// The tiles of this placement's **outlet face**: the rectangle edge on
    /// `dir`'s side, which is the side product leaves by (module doc, point
    /// 3). Deterministic order, minimum coordinate first.
    ///
    /// For a 1x1 placement this is always the single tile itself, whatever
    /// the direction — which is why a belt tile, a hopper and a dock all
    /// work out of the same two functions a multi-tile machine does.
    pub fn outlet_tiles(&self, origin: TileCoord, dir: Direction) -> Vec<TileCoord> {
        self.edge_tiles(origin, dir, dir)
    }

    /// The tiles of this placement's **inlet face**: the rectangle edge
    /// opposite `dir`, which is the face a belt has to point into to feed
    /// this machine (module doc, point 3). Deterministic order, minimum
    /// coordinate first.
    pub fn inlet_tiles(&self, origin: TileCoord, dir: Direction) -> Vec<TileCoord> {
        self.edge_tiles(origin, dir, dir.opposite())
    }

    /// The tiles immediately *outside* the outlet face — where a receiving
    /// belt has to stand for this placement to discharge onto it.
    pub fn outlet_neighbours(&self, origin: TileCoord, dir: Direction) -> Vec<TileCoord> {
        self.outlet_tiles(origin, dir)
            .into_iter()
            .map(|tile| dir.step(tile))
            .collect()
    }

    /// The tiles immediately *outside* the inlet face — where a feeding
    /// belt has to stand, pointing at this placement.
    pub fn inlet_neighbours(&self, origin: TileCoord, dir: Direction) -> Vec<TileCoord> {
        let back = dir.opposite();
        self.inlet_tiles(origin, dir)
            .into_iter()
            .map(|tile| back.step(tile))
            .collect()
    }

    /// The tiles of the rectangle's `edge` side, once placed at `origin`
    /// facing `dir`. Split out so the inlet and the outlet are provably the
    /// same computation on opposite sides rather than two hand-written
    /// cases that could drift apart.
    fn edge_tiles(&self, origin: TileCoord, dir: Direction, edge: Direction) -> Vec<TileCoord> {
        let rect = self.rotated(dir);
        let (w, h) = (rect.width_tiles, rect.height_tiles);
        match edge {
            Direction::East => (0..h).map(|dy| (origin.0 + w - 1, origin.1 + dy)).collect(),
            Direction::West => (0..h).map(|dy| (origin.0, origin.1 + dy)).collect(),
            Direction::North => (0..w).map(|dx| (origin.0 + dx, origin.1 + h - 1)).collect(),
            Direction::South => (0..w).map(|dx| (origin.0 + dx, origin.1)).collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineKind {
    /// Content id, matching the `machine` field of a `Recipe` in
    /// `data/recipes.ron` (e.g. "crusher").
    pub id: String,
    pub footprint: Footprint,
    /// The crew this kind needs **while running**, as `(category, count)`
    /// lines in the categories `data/staff.ron` defines (GAMEPLAY.md §7;
    /// `sim::staff`). Required since schema 2; an empty list is a kind that
    /// never asks — a hopper, a tank, a splitter. Each tick a machine claims
    /// the whole list or nothing (`StaffPools::claim`), and the list's own
    /// order is the order a refusal is reported in. Checked against the
    /// catalog by `StaffCatalog::validate_machines`, a test.
    pub staff: Vec<(String, u32)>,
    /// What to call this kind in front of a player, when title-casing the
    /// id gets it wrong — the machine twin of [`super::recipe::Recipe::
    /// display_name`], and everything that doc says applies here.
    ///
    /// **Optional, and usually absent**: the build menu, the hotbar and
    /// every hover tooltip title themselves from the id (`cooking_vat` →
    /// "Cooking Vat"), which is what keeps a new `data/shop.ron` entry from
    /// needing a matching edit in GDScript.
    ///
    /// Where the recipe field exists for initialisms the formatter cannot
    /// spell, this one exists for the other case: **a kind whose id is not
    /// what the game calls it.** `warehouse_rack` is the shipped example —
    /// the project owner's 2026-09-21 call renamed it to "Stock Shelving"
    /// in front of the player, and the id stayed where it was because a
    /// kind id is a key (it is written into every save file and looked up
    /// by `data/storage.ron`, `data/shop.ron` and the voxel asset table),
    /// while a name is content. Renaming the key to follow the label would
    /// have cost a `SAVE_SCHEMA_VERSION` bump — every existing save
    /// rejected — for a change no player can see the inside of.
    #[serde(default)]
    pub display_name: Option<String>,
}

/// The file's shape once its version has been checked (see
/// `from_ron_str`): `schema_version` is read off a header first, so it is
/// not repeated here.
#[derive(Debug, Deserialize)]
struct MachineFile {
    machines: Vec<MachineKind>,
}

#[derive(Debug)]
pub enum MachineLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    DuplicateKind {
        id: String,
    },
    /// A footprint with a zero or negative extent — a machine that occupies
    /// no tiles, or a nonsensical negative rectangle. Caught here so no
    /// layout, snapshot or hover code downstream has to handle it.
    DegenerateFootprint {
        id: String,
        footprint: Footprint,
    },
    /// A `display_name` that is present but empty or whitespace — the same
    /// refusal [`super::recipe::RecipeLoadError::BlankDisplayName`] makes,
    /// for the same reason: absent means "title-case the id", which is a
    /// real answer, and blank means a build-menu card captioned with
    /// nothing, which would show up only as a gap in a rendered frame.
    BlankDisplayName {
        id: String,
    },
    /// A recipe names a machine kind that `machines.ron` doesn't define.
    /// Reported as an error rather than a silent default, so a typo'd kind
    /// surfaces the way a compile error would (the same reasoning
    /// `world::validate_chain` uses for a broken recipe chain).
    UnknownKindInRecipe {
        recipe: String,
        machine: String,
        known: Vec<String>,
    },
}

impl std::fmt::Display for MachineLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineLoadError::Parse(e) => write!(f, "failed to parse machines.ron: {e}"),
            MachineLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "machines.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            MachineLoadError::DuplicateKind { id } => write!(
                f,
                "machines.ron defines machine kind '{id}' more than once — kind ids are \
                 looked up by name, so a duplicate silently shadows the other"
            ),
            MachineLoadError::DegenerateFootprint { id, footprint } => write!(
                f,
                "machines.ron machine kind '{id}' has a degenerate footprint \
                 ({}x{}) — both width_tiles and height_tiles must be at least 1, since a \
                 machine occupies the tile its belt feeds into",
                footprint.width_tiles, footprint.height_tiles
            ),
            MachineLoadError::BlankDisplayName { id } => write!(
                f,
                "machines.ron machine kind '{id}' has a blank display_name — omit the \
                 field to title-case the id instead"
            ),
            MachineLoadError::UnknownKindInRecipe {
                recipe,
                machine,
                known,
            } => write!(
                f,
                "recipes.ron recipe '{recipe}' runs on machine kind '{machine}', which \
                 data/machines.ron does not define (known kinds: {}) — add it there, or fix \
                 the recipe's `machine` field",
                known.join(", ")
            ),
        }
    }
}

impl std::error::Error for MachineLoadError {}

/// Machine kinds keyed by id — the `machines.ron` counterpart of
/// [`super::recipe::RecipeRegistry`].
///
/// Unlike `RecipeRegistry`, file order carries no meaning here: `recipes.ron`
/// order *is* the production chain (see `world.rs`'s module doc), whereas
/// this file is a pure lookup table. Nothing should depend on the order kinds
/// appear in.
#[derive(Debug, Default)]
pub struct MachineRegistry {
    by_id: HashMap<String, MachineKind>,
}

/// The bundled machine content, embedded at compile time for exactly the
/// reason `lib.rs` embeds `recipes.ron`: resolving it must be identical
/// under `cargo test`, the Godot editor and an exported build, none of
/// which share a working directory.
///
/// This one lives in `sim/` rather than at the `gdext` boundary in `lib.rs`
/// so [`super::world::World::new`] can keep its single-argument signature —
/// every existing caller and test builds a `World` from recipes alone, and
/// footprints are infrastructure those call sites have no opinion about.
/// Tests that need a non-shipping footprint use
/// [`super::world::World::with_machines`] instead.
const MACHINES_RON: &str = include_str!("../../../data/machines.ron");

impl MachineRegistry {
    pub fn from_ron_str(text: &str) -> Result<Self, MachineLoadError> {
        // The version is checked FIRST, off a header that ignores every
        // other field, so a file from another schema is reported as such
        // rather than as whatever parse error its old shape happens to
        // produce (a v1 file has no `staff` list, which would otherwise
        // read as "missing field", not "wrong version").
        #[derive(Deserialize)]
        struct Header {
            schema_version: u32,
        }
        let header: Header = ron::from_str(text).map_err(MachineLoadError::Parse)?;
        if header.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(MachineLoadError::UnsupportedSchemaVersion {
                found: header.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let file: MachineFile = ron::from_str(text).map_err(MachineLoadError::Parse)?;

        let mut by_id = HashMap::with_capacity(file.machines.len());
        for kind in file.machines {
            if kind.footprint.width_tiles < 1 || kind.footprint.height_tiles < 1 {
                return Err(MachineLoadError::DegenerateFootprint {
                    id: kind.id,
                    footprint: kind.footprint,
                });
            }
            if by_id.contains_key(&kind.id) {
                return Err(MachineLoadError::DuplicateKind { id: kind.id });
            }
            if kind
                .display_name
                .as_deref()
                .is_some_and(|name| name.trim().is_empty())
            {
                return Err(MachineLoadError::BlankDisplayName { id: kind.id });
            }
            by_id.insert(kind.id.clone(), kind);
        }
        Ok(Self { by_id })
    }

    /// The shipped `data/machines.ron`. Panics only if that bundled file is
    /// itself invalid, which is a build-time content bug, not a runtime
    /// condition any caller could recover from.
    pub fn bundled() -> Self {
        Self::from_ron_str(MACHINES_RON)
            .unwrap_or_else(|e| panic!("bundled data/machines.ron must load: {e}"))
    }

    pub fn get(&self, id: &str) -> Option<&MachineKind> {
        self.by_id.get(id)
    }

    /// A kind's footprint, or `None` if the kind is unknown. Prefer calling
    /// [`Self::validate_covers`] once up front and then treating a missing
    /// kind as impossible, rather than defaulting a lookup miss to 1x1
    /// somewhere downstream.
    pub fn footprint_of(&self, id: &str) -> Option<Footprint> {
        self.by_id.get(id).map(|k| k.footprint)
    }

    /// Every kind that carries a name of its own, as `(id, display name)`
    /// pairs — see [`MachineKind::display_name`]. Kinds that leave the field
    /// out are absent, which is the presentation layer's cue to title-case
    /// the id. Sorted, because the backing map is a `HashMap` and this feeds
    /// a snapshot the GDScript side may compare against.
    pub fn display_names(&self) -> Vec<(&str, &str)> {
        let mut names: Vec<(&str, &str)> = self
            .by_id
            .values()
            .filter_map(|kind| {
                kind.display_name
                    .as_deref()
                    .map(|name| (kind.id.as_str(), name))
            })
            .collect();
        names.sort_unstable();
        names
    }

    /// Known kind ids, sorted — a stable order for error messages and tests
    /// (the backing map's iteration order is not deterministic).
    pub fn kind_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.by_id.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Checks that every recipe's `machine` kind is defined here — the
    /// cross-file invariant between `recipes.ron` and `machines.ron`.
    /// Returns the first offending recipe rather than a list, matching how
    /// the RON loaders above report the first structural problem they hit.
    pub fn validate_covers(
        &self,
        recipes: &super::recipe::RecipeRegistry,
    ) -> Result<(), MachineLoadError> {
        for recipe in recipes.ordered() {
            if !self.by_id.contains_key(&recipe.machine) {
                return Err(MachineLoadError::UnknownKindInRecipe {
                    recipe: recipe.id.clone(),
                    machine: recipe.machine.clone(),
                    known: self.kind_ids(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::recipe::RecipeRegistry;

    /// The east-facing case must be the old, pre-rotation geometry exactly —
    /// this is the regression anchor the whole "Real Placement" epic rests
    /// on (see [`Direction`]'s own doc).
    #[test]
    fn facing_east_is_the_unrotated_footprint_verbatim() {
        let fp = Footprint {
            width_tiles: 3,
            height_tiles: 2,
        };
        assert_eq!(fp.rotated(Direction::East), fp);
        assert_eq!(fp.tiles_facing((4, 0), Direction::East), fp.tiles((4, 0)));
        // The inlet is the origin's own column and the outlet the far one —
        // "the origin is the tile the stage's belt feeds into", verbatim.
        assert_eq!(
            fp.inlet_tiles((4, 0), Direction::East),
            vec![(4, 0), (4, 1)]
        );
        assert_eq!(
            fp.outlet_tiles((4, 0), Direction::East),
            vec![(6, 0), (6, 1)]
        );
        assert_eq!(
            fp.inlet_neighbours((4, 0), Direction::East),
            vec![(3, 0), (3, 1)]
        );
        assert_eq!(
            fp.outlet_neighbours((4, 0), Direction::East),
            vec![(7, 0), (7, 1)]
        );
    }

    /// A quarter turn swaps the extents and keeps the anchor; a half turn
    /// keeps the shape and only swaps which edge is the inlet.
    #[test]
    fn a_quarter_turn_swaps_the_extents_around_the_same_anchor() {
        let fp = Footprint {
            width_tiles: 3,
            height_tiles: 2,
        };
        assert_eq!(
            fp.rotated(Direction::North),
            Footprint {
                width_tiles: 2,
                height_tiles: 3
            }
        );
        assert_eq!(fp.rotated(Direction::South), fp.rotated(Direction::North));
        assert_eq!(fp.rotated(Direction::West), fp);

        let tiles = fp.tiles_facing((4, 0), Direction::North);
        assert_eq!(tiles.len(), 6);
        assert_eq!(
            tiles,
            vec![(4, 0), (5, 0), (4, 1), (5, 1), (4, 2), (5, 2)],
            "the anchor is still the min-X/min-Y corner"
        );
        // Facing north, the outlet is the top row and the inlet the bottom.
        assert_eq!(
            fp.outlet_tiles((4, 0), Direction::North),
            vec![(4, 2), (5, 2)]
        );
        assert_eq!(
            fp.inlet_tiles((4, 0), Direction::North),
            vec![(4, 0), (5, 0)]
        );
        assert_eq!(
            fp.outlet_neighbours((4, 0), Direction::North),
            vec![(4, 3), (5, 3)]
        );
        assert_eq!(
            fp.inlet_neighbours((4, 0), Direction::North),
            vec![(4, -1), (5, -1)]
        );
    }

    /// A half turn is where the inlet and outlet swap ends without the
    /// rectangle moving at all — the case a "rotation is just a mesh
    /// transform" reading would get wrong.
    #[test]
    fn a_half_turn_swaps_the_faces_without_moving_a_tile() {
        let fp = Footprint {
            width_tiles: 3,
            height_tiles: 2,
        };
        assert_eq!(
            fp.tiles_facing((4, 0), Direction::West),
            fp.tiles_facing((4, 0), Direction::East)
        );
        assert_eq!(
            fp.inlet_tiles((4, 0), Direction::West),
            fp.outlet_tiles((4, 0), Direction::East)
        );
        assert_eq!(
            fp.outlet_tiles((4, 0), Direction::West),
            fp.inlet_tiles((4, 0), Direction::East)
        );
    }

    /// A 1x1 placement — every belt tile, every pipe tile, the hopper, the
    /// tank and the dock — has one tile that is simultaneously its whole
    /// body, its inlet face and its outlet face, whichever way it faces.
    #[test]
    fn a_one_by_one_placement_is_its_own_inlet_and_outlet_in_every_direction() {
        let fp = Footprint::ONE_BY_ONE;
        for dir in Direction::ALL {
            assert_eq!(fp.tiles_facing((2, 3), dir), vec![(2, 3)]);
            assert_eq!(fp.inlet_tiles((2, 3), dir), vec![(2, 3)]);
            assert_eq!(fp.outlet_tiles((2, 3), dir), vec![(2, 3)]);
            assert_eq!(fp.outlet_neighbours((2, 3), dir), vec![dir.step((2, 3))]);
            assert_eq!(
                fp.inlet_neighbours((2, 3), dir),
                vec![dir.opposite().step((2, 3))]
            );
        }
    }

    #[test]
    fn contains_facing_follows_the_rotated_rectangle() {
        let fp = Footprint {
            width_tiles: 3,
            height_tiles: 1,
        };
        assert!(fp.contains_facing((0, 0), Direction::East, (2, 0)));
        assert!(!fp.contains_facing((0, 0), Direction::East, (0, 2)));
        assert!(fp.contains_facing((0, 0), Direction::North, (0, 2)));
        assert!(!fp.contains_facing((0, 0), Direction::North, (2, 0)));
    }

    const EXAMPLE: &str = r#"(
        schema_version: 2,
        machines: [
            MachineKind(id: "cooker", footprint: Footprint(width_tiles: 1, height_tiles: 1), staff: [("manufacturing", 1)]),
            MachineKind(id: "crusher", footprint: Footprint(width_tiles: 2, height_tiles: 3), staff: []),
        ],
    )"#;

    fn recipe_on(machine: &str) -> RecipeRegistry {
        let text = format!(
            r#"(
                schema_version: 3,
                recipes: [
                    Recipe(
                        id: "tomato_sauce",
                        inputs: [("tomato", 4)],
                        outputs: [("tomato_sauce", 1)],
                        machine: "{machine}",
                        time_ticks: 180,
                    ),
                ],
            )"#
        );
        RecipeRegistry::from_ron_str(&text).expect("test recipe text must parse")
    }

    #[test]
    fn loads_footprints_by_kind_id() {
        let registry = MachineRegistry::from_ron_str(EXAMPLE).unwrap();
        assert_eq!(registry.footprint_of("cooker"), Some(Footprint::ONE_BY_ONE));
        assert_eq!(
            registry.footprint_of("crusher"),
            Some(Footprint {
                width_tiles: 2,
                height_tiles: 3
            })
        );
        assert_eq!(registry.footprint_of("nonexistent"), None);
    }

    #[test]
    fn the_bundled_file_defines_every_shipped_kind_with_a_usable_footprint() {
        let registry = MachineRegistry::bundled();
        let recipes = RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("bundled recipes must parse");
        registry
            .validate_covers(&recipes)
            .expect("every shipped recipe's machine kind must be defined");
        for recipe in recipes.ordered() {
            let footprint = registry
                .footprint_of(&recipe.machine)
                .expect("validate_covers passed, so every kind resolves");
            assert!(
                footprint.width_tiles >= 1 && footprint.height_tiles >= 1,
                "kind '{}' has a degenerate footprint {footprint:?}",
                recipe.machine
            );
        }
    }

    /// Ticket 1 shipped every kind as 1x1 so that generalizing the layout
    /// math moved no tile; ticket 2 is the content change that made one kind
    /// genuinely bigger. This asserts that the *mechanism* has a real user in
    /// shipped content — without it, every width/height code path would only
    /// ever be exercised by test-only fixtures.
    #[test]
    fn the_shipped_content_actually_uses_a_multi_tile_footprint() {
        let registry = MachineRegistry::bundled();
        let recipes = RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("bundled recipes must parse");
        let biggest = recipes
            .ordered()
            .iter()
            .filter_map(|r| registry.footprint_of(&r.machine))
            .max_by_key(|f| f.width_tiles * f.height_tiles)
            .expect("the shipped chain has at least one stage");
        assert!(
            biggest != Footprint::ONE_BY_ONE,
            "no shipped machine kind is larger than one tile — the footprint mechanism would \
             then have no real content behind it (the cooking vat is meant to be 2x2)"
        );
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 2", "schema_version: 99", 1);
        let err = MachineRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            MachineLoadError::UnsupportedSchemaVersion {
                found: 99,
                supported: 2
            }
        ));
    }

    /// v1 had no `staff` list; a v1 file is rejected by version before its
    /// shape is ever looked at (the staff pools MVP epic).
    #[test]
    fn rejects_a_v1_file() {
        let text = r#"(schema_version: 1, machines: [
            MachineKind(id: "cooker", footprint: Footprint(width_tiles: 1, height_tiles: 1)),
        ])"#;
        assert!(matches!(
            MachineRegistry::from_ron_str(text).unwrap_err(),
            MachineLoadError::UnsupportedSchemaVersion {
                found: 1,
                supported: 2
            }
        ));
    }

    /// A v2 file with the `staff` list missing on a kind is a parse error,
    /// not a silent zero — the reason the version was bumped.
    #[test]
    fn a_kind_without_a_staff_list_is_a_loader_error() {
        let text = r#"(schema_version: 2, machines: [
            MachineKind(id: "cooker", footprint: Footprint(width_tiles: 1, height_tiles: 1)),
        ])"#;
        assert!(matches!(
            MachineRegistry::from_ron_str(text).unwrap_err(),
            MachineLoadError::Parse(_)
        ));
    }

    #[test]
    fn rejects_a_zero_width_footprint() {
        let text = EXAMPLE.replacen("width_tiles: 2", "width_tiles: 0", 1);
        let err = MachineRegistry::from_ron_str(&text).unwrap_err();
        match err {
            MachineLoadError::DegenerateFootprint { ref id, footprint } => {
                assert_eq!(id, "crusher");
                assert_eq!(footprint.width_tiles, 0);
            }
            other => panic!("expected DegenerateFootprint, got {other:?}"),
        }
        assert!(err.to_string().contains("degenerate footprint"));
    }

    #[test]
    fn rejects_a_negative_height_footprint() {
        let text = EXAMPLE.replacen("height_tiles: 3", "height_tiles: -2", 1);
        let err = MachineRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(err, MachineLoadError::DegenerateFootprint { .. }));
    }

    /// The field that lets a kind be called something other than its id, and
    /// the shipped reason it exists: `warehouse_rack` is the key every save
    /// and every content file uses, and "Stock Shelving" is what the player
    /// reads. A kind that leaves it out is absent from this map, which is
    /// the presentation layer's cue to title-case the id itself.
    #[test]
    fn a_kind_may_name_itself_and_most_do_not() {
        let text = EXAMPLE.replacen(
            r#"staff: []),"#,
            r#"staff: [], display_name: "The Crusher"),"#,
            1,
        );
        let registry = MachineRegistry::from_ron_str(&format!(
            "#![enable(implicit_some)]\n{text}"
        ))
        .unwrap();
        assert_eq!(registry.display_names(), vec![("crusher", "The Crusher")]);

        let bundled = MachineRegistry::bundled();
        assert_eq!(
            bundled.display_names(),
            vec![("warehouse_rack", "Stock Shelving")],
            "the shipped content names exactly one kind; the rest title-case correctly"
        );
    }

    /// Blank is not "no name" — it is a build-menu card captioned with
    /// nothing, and it would only show up in a rendered frame.
    #[test]
    fn a_blank_display_name_is_rejected_rather_than_drawn_as_an_empty_label() {
        let text = EXAMPLE.replacen(r#"staff: []),"#, r#"staff: [], display_name: "  "),"#, 1);
        let err =
            MachineRegistry::from_ron_str(&format!("#![enable(implicit_some)]\n{text}")).unwrap_err();
        match err {
            MachineLoadError::BlankDisplayName { ref id } => assert_eq!(id, "crusher"),
            other => panic!("expected BlankDisplayName, got {other:?}"),
        }
        assert!(err.to_string().contains("blank display_name"));
    }

    #[test]
    fn rejects_a_duplicated_kind_id() {
        let text = EXAMPLE.replacen(r#"id: "crusher""#, r#"id: "cooker""#, 1);
        let err = MachineRegistry::from_ron_str(&text).unwrap_err();
        match err {
            MachineLoadError::DuplicateKind { ref id } => assert_eq!(id, "cooker"),
            other => panic!("expected DuplicateKind, got {other:?}"),
        }
    }

    #[test]
    fn a_recipe_naming_an_undefined_kind_is_an_error_not_a_default() {
        let registry = MachineRegistry::from_ron_str(EXAMPLE).unwrap();
        let err = registry
            .validate_covers(&recipe_on("fermenter"))
            .unwrap_err();
        match err {
            MachineLoadError::UnknownKindInRecipe {
                ref recipe,
                ref machine,
                ref known,
            } => {
                assert_eq!(recipe, "tomato_sauce");
                assert_eq!(machine, "fermenter");
                assert_eq!(known, &["cooker".to_string(), "crusher".to_string()]);
            }
            other => panic!("expected UnknownKindInRecipe, got {other:?}"),
        }
        // The message has to name both sides of the mismatch, since the fix
        // could be in either file.
        let message = err.to_string();
        assert!(message.contains("tomato_sauce"), "{message}");
        assert!(message.contains("fermenter"), "{message}");
    }

    #[test]
    fn a_recipe_naming_a_defined_kind_validates() {
        let registry = MachineRegistry::from_ron_str(EXAMPLE).unwrap();
        registry.validate_covers(&recipe_on("crusher")).unwrap();
    }

    #[test]
    fn footprint_contains_covers_the_min_corner_rectangle() {
        let fp = Footprint {
            width_tiles: 2,
            height_tiles: 3,
        };
        let origin = (4, 0);
        // Inside: the full 2x3 block anchored at the min-x/min-y corner.
        for tile in [(4, 0), (5, 0), (4, 1), (5, 1), (4, 2), (5, 2)] {
            assert!(fp.contains(origin, tile), "{tile:?} must be inside");
        }
        // Outside: one tile past every edge, including *behind* the origin
        // (the belt side), which must never be part of the machine.
        for tile in [(3, 0), (6, 0), (4, -1), (4, 3), (6, 3)] {
            assert!(!fp.contains(origin, tile), "{tile:?} must be outside");
        }
    }

    #[test]
    fn footprint_tiles_is_row_major_and_complete() {
        let fp = Footprint {
            width_tiles: 3,
            height_tiles: 2,
        };
        assert_eq!(
            fp.tiles((10, 5)),
            vec![(10, 5), (11, 5), (12, 5), (10, 6), (11, 6), (12, 6)]
        );
    }
}
