//! Recipe content, loaded from `data/recipes.ron` (ARCHITECTURE.md §3.6).
//!
//! Recipes are data, never hardcoded in simulation code (§10's AI-agent
//! convention: "content changes should never require touching Rust"). This
//! module only knows how to parse and look recipes up; it has no opinion on
//! what any specific recipe contains.

use std::collections::HashMap;

use serde::Deserialize;

use super::types::{ItemId, RecipeId};

/// The schema version this build of sim_core understands. Bump this and
/// migrate/reject on mismatch (ARCHITECTURE.md §4) whenever `Recipe`'s shape
/// changes in a way older data can't satisfy.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
pub struct Recipe {
    pub id: RecipeId,
    /// (item id, quantity) pairs consumed per craft.
    pub inputs: Vec<(ItemId, u32)>,
    /// (item id, quantity) pairs produced per craft.
    pub outputs: Vec<(ItemId, u32)>,
    /// The machine kind this recipe runs on (e.g. "cooker"). Not yet
    /// validated against a `machines.ron` — that file doesn't exist yet
    /// (see CLAUDE.md's Project Structure section).
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
/// tick it needs to check what it's making.
#[derive(Debug, Default)]
pub struct RecipeRegistry {
    by_id: HashMap<RecipeId, Recipe>,
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
            .into_iter()
            .map(|r| (r.id.clone(), r))
            .collect();
        Ok(Self { by_id })
    }

    pub fn get(&self, id: &str) -> Option<&Recipe> {
        self.by_id.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
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
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 99", 1);
        let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            RecipeLoadError::UnsupportedSchemaVersion { found: 99, .. }
        ));
    }
}
