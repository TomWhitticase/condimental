//! Recipe content, loaded from `data/recipes.ron` (ARCHITECTURE.md §3.6).
//!
//! Recipes are data, never hardcoded in simulation code (§10's AI-agent
//! convention: "content changes should never require touching Rust"). This
//! module only knows how to parse and look recipes up; it has no opinion on
//! what any specific recipe contains.

use std::collections::HashMap;

use serde::Deserialize;

use super::types::{FluidId, ItemId, RecipeId};

/// The schema version this build of sim_core understands. Bump this and
/// migrate/reject on mismatch (ARCHITECTURE.md §4) whenever `Recipe`'s shape
/// changes in a way older data can't satisfy.
///
/// **v2 (milestone 6, Fluids)** added `Recipe::fluid_inputs`.
///
/// v1 data is **rejected, not migrated**, even though `fluid_inputs` is
/// `#[serde(default)]` and a v1 file would therefore parse cleanly as "no
/// fluid inputs." ARCHITECTURE.md §4 allows either, and rejection is the
/// right call here: `data/recipes.ron` is bundled into the binary at
/// compile time (`include_str!` in `lib.rs`), so there is exactly one such
/// file and it is updated in the same commit as this constant — a v1 file
/// reaching this loader means someone's data and binary have drifted
/// apart, which is worth a loud error rather than a silent default. A
/// migration path becomes worth writing when recipe data is loaded from
/// disk at runtime (i.e. when players or mods can supply their own), which
/// is not the case yet.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Deserialize, Clone)]
pub struct Recipe {
    pub id: RecipeId,
    /// (item id, quantity) pairs consumed per craft.
    pub inputs: Vec<(ItemId, u32)>,
    /// (fluid id, whole fluid units) consumed per craft from the machine's
    /// connected `fluid::FluidNetwork` — the liquid half of a recipe's
    /// ingredient list (ARCHITECTURE.md §9 milestone 6, GAMEPLAY.md §2's
    /// tier table, e.g. the ketchup mix needing vinegar).
    ///
    /// Defaults to empty, so every pre-milestone-6 recipe is unchanged and
    /// adding a fluid requirement stays a pure data-file edit
    /// (ARCHITECTURE.md §3.6/§10) — the same additive property `inputs`
    /// already has.
    ///
    /// Quantities are **whole units (`u32`), not `Fx`**, matching how
    /// `inputs` writes solid quantities: recipe content is authored in
    /// whole ingredient units, and keeping the data file integer-only means
    /// no float ever appears on the content path either (a RON float would
    /// have to be converted somewhere, and ARCHITECTURE.md §3.9 is happier
    /// if that conversion simply doesn't exist). The *network* stores
    /// volume as `Fx` because pressure equalization moves partial units
    /// around; the conversion happens once, at the point of use, in
    /// `Machine::tick`.
    #[serde(default)]
    pub fluid_inputs: Vec<(FluidId, u32)>,
    /// (item id, quantity) pairs produced per craft.
    pub outputs: Vec<(ItemId, u32)>,
    /// The machine kind this recipe runs on (e.g. "crusher"). Must name an
    /// entry in `data/machines.ron` — [`super::machines::MachineRegistry::validate_covers`]
    /// checks that at world-construction time, and that entry is what says
    /// how many tiles the machine occupies.
    pub machine: String,
    /// How long a single craft takes, in sim ticks (ARCHITECTURE.md §3.5's
    /// tick-counted `TimedProcess`, never wall-clock time).
    pub time_ticks: u64,
}

#[derive(Debug, Deserialize)]
struct RecipeFile {
    schema_version: u32,
    recipes: Vec<Recipe>,
}

#[derive(Debug)]
pub enum RecipeLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion { found: u32, supported: u32 },
}

impl std::fmt::Display for RecipeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecipeLoadError::Parse(e) => write!(f, "failed to parse recipes.ron: {e}"),
            RecipeLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "recipes.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
        }
    }
}

impl std::error::Error for RecipeLoadError {}

/// Recipes keyed by id, looked up by `Machine` (ARCHITECTURE.md §3.2) each
/// tick it needs to check what it's making. Also preserves the data file's
/// own list order (`ordered()`) — milestone 4's `World` treats that order as
/// its demo production chain's stage sequence (see `world.rs`'s module doc),
/// which is exactly what makes "add a recipe to extend the chain" a
/// data-file-only change (ARCHITECTURE.md §3.6/§10): no Rust code encodes
/// which recipe comes after which, the file's own ordering does.
#[derive(Debug, Default)]
pub struct RecipeRegistry {
    recipes: Vec<Recipe>,
    by_id: HashMap<RecipeId, usize>,
}

impl RecipeRegistry {
    pub fn from_ron_str(text: &str) -> Result<Self, RecipeLoadError> {
        let file: RecipeFile = ron::from_str(text).map_err(RecipeLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(RecipeLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let by_id = file
            .recipes
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        Ok(Self {
            recipes: file.recipes,
            by_id,
        })
    }

    pub fn get(&self, id: &str) -> Option<&Recipe> {
        self.by_id.get(id).map(|&i| &self.recipes[i])
    }

    /// Recipes in data-file order — see the struct doc for why this order
    /// matters, not just recipe content.
    pub fn ordered(&self) -> &[Recipe] {
        &self.recipes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 2,
        recipes: [
            Recipe(
                id: "tomato_sauce",
                inputs: [("tomato", 4)],
                outputs: [("tomato_sauce", 1)],
                machine: "cooker",
                time_ticks: 180,
            ),
        ],
    )"#;

    #[test]
    fn loads_recipe_by_id() {
        let registry = RecipeRegistry::from_ron_str(EXAMPLE).unwrap();
        let recipe = registry.get("tomato_sauce").expect("recipe present");
        assert_eq!(recipe.inputs, vec![("tomato".to_string(), 4)]);
        assert_eq!(recipe.outputs, vec![("tomato_sauce".to_string(), 1)]);
        assert_eq!(recipe.time_ticks, 180);
    }

    #[test]
    fn a_recipe_without_fluid_inputs_defaults_to_none() {
        let registry = RecipeRegistry::from_ron_str(EXAMPLE).unwrap();
        let recipe = registry.get("tomato_sauce").expect("recipe present");
        assert!(
            recipe.fluid_inputs.is_empty(),
            "fluid_inputs must be additive — omitting it means 'this recipe needs no liquid'"
        );
    }

    #[test]
    fn loads_a_recipe_that_requires_a_fluid() {
        const WITH_FLUID: &str = r#"(
            schema_version: 2,
            recipes: [
                Recipe(
                    id: "ketchup",
                    inputs: [("tomato_sauce", 1)],
                    fluid_inputs: [("vinegar", 2)],
                    outputs: [("ketchup", 1)],
                    machine: "reducer",
                    time_ticks: 120,
                ),
            ],
        )"#;
        let registry = RecipeRegistry::from_ron_str(WITH_FLUID).unwrap();
        let recipe = registry.get("ketchup").expect("recipe present");
        assert_eq!(recipe.fluid_inputs, vec![("vinegar".to_string(), 2)]);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 2", "schema_version: 99", 1);
        let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            RecipeLoadError::UnsupportedSchemaVersion { found: 99, .. }
        ));
    }

    /// See `CURRENT_SCHEMA_VERSION`'s doc: pre-fluids data is rejected
    /// rather than migrated-with-defaults, on purpose.
    #[test]
    fn rejects_pre_fluids_v1_data_rather_than_silently_defaulting_it() {
        let text = EXAMPLE.replacen("schema_version: 2", "schema_version: 1", 1);
        let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            RecipeLoadError::UnsupportedSchemaVersion { found: 1, supported: 2 }
        ));
    }
}
