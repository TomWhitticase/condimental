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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

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
    pub fn tiles(&self, origin: (i32, i32)) -> Vec<(i32, i32)> {
        let mut tiles = Vec::with_capacity((self.width_tiles * self.height_tiles) as usize);
        for dy in 0..self.height_tiles {
            for dx in 0..self.width_tiles {
                tiles.push((origin.0 + dx, origin.1 + dy));
            }
        }
        tiles
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineKind {
    /// Content id, matching the `machine` field of a `Recipe` in
    /// `data/recipes.ron` (e.g. "crusher").
    pub id: String,
    pub footprint: Footprint,
}

#[derive(Debug, Deserialize)]
struct MachineFile {
    schema_version: u32,
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
        let file: MachineFile = ron::from_str(text).map_err(MachineLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(MachineLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }

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

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        machines: [
            MachineKind(id: "cooker", footprint: Footprint(width_tiles: 1, height_tiles: 1)),
            MachineKind(id: "crusher", footprint: Footprint(width_tiles: 2, height_tiles: 3)),
        ],
    )"#;

    fn recipe_on(machine: &str) -> RecipeRegistry {
        let text = format!(
            r#"(
                schema_version: 2,
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
        assert_eq!(
            registry.footprint_of("cooker"),
            Some(Footprint::ONE_BY_ONE)
        );
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
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 99", 1);
        let err = MachineRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            MachineLoadError::UnsupportedSchemaVersion {
                found: 99,
                supported: 1
            }
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
        assert!(matches!(
            err,
            MachineLoadError::DegenerateFootprint { .. }
        ));
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
            vec![
                (10, 5),
                (11, 5),
                (12, 5),
                (10, 6),
                (11, 6),
                (12, 6)
            ]
        );
    }
}
