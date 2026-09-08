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
/// **v3 (the chain-rework epic)** added its mirror, `Recipe::fluid_outputs`.
///
/// Older data is **rejected, not migrated**, even though both fluid fields
/// are `#[serde(default)]` and an older file would therefore parse cleanly
/// as "no fluids." ARCHITECTURE.md §4 allows either, and rejection is the
/// right call here: `data/recipes.ron` is bundled into the binary at
/// compile time (`include_str!` in `lib.rs`), so there is exactly one such
/// file and it is updated in the same commit as this constant — an old file
/// reaching this loader means someone's data and binary have drifted
/// apart, which is worth a loud error rather than a silent default. The v3
/// bump makes that concrete rather than theoretical: a v2 file's chain is
/// belt-connected end to end, and defaulting it into a build that expects
/// the middle of the line to be piped would produce a factory that loads,
/// renders, and never runs. A migration path becomes worth writing when
/// recipe data is loaded from disk at runtime (i.e. when players or mods can
/// supply their own), which is not the case yet.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

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
    ///
    /// May be **empty**, which is what a stage whose whole output is a fluid
    /// looks like (the shipped mixer and cooking vat). An empty list is
    /// vacuously satisfiable, so such a machine is never blocked on its
    /// solid output buffer.
    pub outputs: Vec<(ItemId, u32)>,
    /// (fluid id, whole fluid units) **deposited** per craft into the
    /// `fluid::FluidNetwork` off this machine's outlet face — the exact
    /// mirror of [`Recipe::fluid_inputs`], added by the chain-rework epic so
    /// a machine can put liquid into a pipe rather than only take it out.
    ///
    /// Until this existed, the only thing that could add volume to the
    /// system was a player filling a [`super::fluid::FluidTank`]; a recipe
    /// could consume a fluid and nothing could make one, so the middle of a
    /// production line had to be belt-connected even where a real plant
    /// pipes it.
    ///
    /// Which network it goes into is **geometric, not positional in this
    /// list**: a fluid output leaves by the face the machine points at,
    /// exactly as a solid one does, while a fluid input may come from any
    /// other pipe touching the footprint. That asymmetry is what lets one
    /// machine be plumbed on both sides at once without the player having to
    /// name which run is which. See `world.rs`'s module doc.
    ///
    /// Defaults to empty and is authored in whole units, for the same two
    /// reasons [`Recipe::fluid_inputs`] gives.
    #[serde(default)]
    pub fluid_outputs: Vec<(FluidId, u32)>,
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
        schema_version: 3,
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
            schema_version: 3,
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
        let text = EXAMPLE.replacen("schema_version: 3", "schema_version: 99", 1);
        let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            RecipeLoadError::UnsupportedSchemaVersion { found: 99, .. }
        ));
    }

    /// See `CURRENT_SCHEMA_VERSION`'s doc: older data is rejected rather
    /// than migrated-with-defaults, on purpose — both the pre-fluids v1
    /// shape and the input-fluids-only v2 one.
    #[test]
    fn rejects_older_data_rather_than_silently_defaulting_it() {
        for (older, expected) in [(1u32, 1u32), (2, 2)] {
            let text = EXAMPLE.replacen("schema_version: 3", &format!("schema_version: {older}"), 1);
            let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
            assert!(matches!(
                err,
                RecipeLoadError::UnsupportedSchemaVersion { found, supported: 3 } if found == expected
            ));
        }
    }

    #[test]
    fn a_recipe_without_fluid_outputs_defaults_to_none() {
        let registry = RecipeRegistry::from_ron_str(EXAMPLE).unwrap();
        let recipe = registry.get("tomato_sauce").expect("recipe present");
        assert!(
            recipe.fluid_outputs.is_empty(),
            "fluid_outputs must be additive — omitting it means 'this recipe makes no liquid'"
        );
    }

    /// The shape the chain rework needs: a stage whose entire output is a
    /// fluid, so its solid `outputs` list is empty.
    #[test]
    fn loads_a_recipe_that_produces_a_fluid_and_no_solid() {
        const FLUID_OUT: &str = r#"(
            schema_version: 3,
            recipes: [
                Recipe(
                    id: "ketchup_mix",
                    inputs: [("tomato_paste", 1)],
                    fluid_inputs: [("vinegar", 2)],
                    outputs: [],
                    fluid_outputs: [("ketchup_mix", 1)],
                    machine: "mixer",
                    time_ticks: 90,
                ),
            ],
        )"#;
        let registry = RecipeRegistry::from_ron_str(FLUID_OUT).unwrap();
        let recipe = registry.get("ketchup_mix").expect("recipe present");
        assert!(recipe.outputs.is_empty());
        assert_eq!(recipe.fluid_outputs, vec![("ketchup_mix".to_string(), 1)]);
    }
}
