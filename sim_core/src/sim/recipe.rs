//! Recipe content, loaded from `data/recipes.ron` (ARCHITECTURE.md §3.6).
//!
//! Recipes are data, never hardcoded in simulation code (§10's AI-agent
//! convention: "content changes should never require touching Rust"). This
//! module only knows how to parse and look recipes up; it has no opinion on
//! what any specific recipe contains.

use std::collections::{BTreeMap, HashMap};

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
    /// What to call this recipe's product in front of a player, when
    /// title-casing the id gets it wrong.
    ///
    /// **Optional, and usually absent.** The presentation layer titles
    /// itself from ids — `tomato_paste` → "Tomato Paste" — precisely so a
    /// new entry in this file needs no matching edit in GDScript
    /// (ARCHITECTURE.md §3.6/§10). That formatter is right for every id
    /// that is words, and wrong for an id that is an initialism: `bbq`
    /// title-cases to "Bbq", which is how the HUD came to call the second
    /// sauce "Boxed Bbq" (issue #60).
    ///
    /// So content names itself where it must, and stays quiet where the
    /// formatter is already right. A missing value is not a defect and
    /// never will be: it means "the fallback says it correctly", which is
    /// the answer for all but four of the shipped ids. That is what keeps
    /// this from becoming the hand-written label table the formatter's own
    /// doc warns about — such a table goes stale silently, while this one
    /// cannot, because the thing it labels is defined three lines above it.
    ///
    /// The id it labels is **this recipe's id**, which is also its
    /// product's id: every entry in this file is named after what it makes.
    /// [`RecipeRegistry::from_ron_str`] rejects a recipe whose
    /// `display_name` is blank, and `world.rs` gathers these into one map
    /// for the presentation layer ([`super::world::World::display_names`]).
    #[serde(default)]
    pub display_name: Option<String>,

    /// One line saying what this product *is*, for the Cookbook's page
    /// heading — "The cold-chain tier. Two new machines, and the first
    /// ingredient in the game that can go off."
    ///
    /// **Optional, like every field below.** The Cookbook is built entirely
    /// out of this file and its siblings, so a sauce added to `recipes.ron`
    /// gets a page with no Rust and no GDScript edit (ARCHITECTURE.md
    /// §3.6/§10). What it cannot do is *invent prose*: a page with no
    /// tagline simply omits the line rather than title-casing something to
    /// fill it, exactly as a missing `display_name` falls back rather than
    /// failing.
    ///
    /// Intermediate steps usually leave this out. A tagline belongs to the
    /// thing a player thinks of as a product — "BBQ", not "BBQ Mix" — and
    /// the page for a step in the middle of a chain is read as part of its
    /// sauce's page, not on its own.
    #[serde(default)]
    pub tagline: Option<String>,

    /// The paragraph on this product's Cookbook page: what is interesting
    /// about making it, what it introduces, what it costs you to run.
    ///
    /// Prose, deliberately, and deliberately **here rather than in
    /// GDScript**. `main.gd`'s `STAFF_CATEGORY_BLURBS` is the same idea
    /// written in the presentation layer, and it is the thing this field
    /// exists not to become: a table keyed by content id, living in a file
    /// that has no way to know when the content changed under it. A blurb
    /// three lines from the recipe it describes cannot go stale unnoticed.
    ///
    /// Newlines are honoured. The panel wraps at a pinned width
    /// (`godot_project/CLAUDE.md`'s rule about `autowrap_mode`), so this
    /// does not need hand-wrapping.
    #[serde(default)]
    pub blurb: Option<String>,

    /// Which voxel asset's icon stands for this product, when the asset is
    /// not simply named after the id.
    ///
    /// The presentation layer has carried this mapping since the shop epic,
    /// as `main.gd`'s `ITEM_MODELS` — a hand-written table from content id
    /// to asset name, in a file that ships green when it goes stale and is
    /// caught only by a rendered frame (`godot_project/CLAUDE.md`, UI_UX.md
    /// §2b point 12). That is the wrong place for it: renaming an asset or
    /// adding a sauce is a content change, and a content change must never
    /// need a code change.
    ///
    /// So content names its own asset here, `world.rs` gathers these into
    /// one map beside the display names, and `_icon_for` asks the wire
    /// before it asks its table. The table stays as the documented fallback
    /// rather than being deleted — it still answers for every id shipped
    /// before this field existed — but nothing new needs to be added to it.
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RecipeFile {
    schema_version: u32,
    recipes: Vec<Recipe>,
}

#[derive(Debug)]
pub enum RecipeLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// A `display_name` that is present but empty or whitespace. Absent
    /// means "title-case the id", which is a real answer; blank means a
    /// panel captioned with nothing, which is not — and it would show up
    /// only as a gap in a rendered frame, so it is caught here instead.
    BlankDisplayName {
        id: String,
    },
    /// A `tagline`, `blurb` or `icon` that is present but empty or
    /// whitespace — the same defect as [`Self::BlankDisplayName`] and
    /// caught for the same reason. Absent means "this product has no
    /// tagline", which the Cookbook renders by leaving the line out;
    /// blank means a line of nothing, or an icon lookup for the empty
    /// asset name, neither of which shows up until someone reads a frame.
    BlankText {
        id: String,
        field: &'static str,
    },
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
            RecipeLoadError::BlankDisplayName { id } => write!(
                f,
                "recipe '{id}' has a blank display_name — omit the field to title-case \
                 the id instead"
            ),
            RecipeLoadError::BlankText { id, field } => write!(
                f,
                "recipe '{id}' has a blank {field} — omit the field entirely rather than \
                 giving it an empty value"
            ),
        }
    }
}

impl std::error::Error for RecipeLoadError {}

/// Recipes keyed by id, looked up by `Machine` (ARCHITECTURE.md §3.2) each
/// tick it needs to check what it's making. Also preserves the data file's
/// own list order (`ordered()`), which is what makes "add a recipe to extend
/// the chain" a data-file-only change (ARCHITECTURE.md §3.6/§10): no Rust
/// code encodes which recipe comes after which, the file's own ordering
/// does.
///
/// **That order is a tiebreak, not a sequence, since the BBQ epic.** The
/// file used to describe exactly one production line, one stage per entry,
/// and `world.rs` enforced that consecutive entries chained. It now
/// describes a production *graph* — ketchup and BBQ share a mixer, a
/// cooking vat, a bottler and a packer — so what the order means is
/// narrower and sharper: when a machine's current inputs satisfy more than
/// one of its kind's recipes, **the earlier entry wins**. See
/// [`Self::for_machine`].
#[derive(Debug, Default)]
pub struct RecipeRegistry {
    recipes: Vec<Recipe>,
    by_id: HashMap<RecipeId, usize>,
    /// Every recipe a machine *kind* can run, in file order — the 1:many
    /// inverse of [`Recipe::machine`]. `BTreeMap` rather than `HashMap`
    /// because `world.rs` walks it when it decides what a network carries
    /// (ARCHITECTURE.md §3.9's iteration-order rule).
    by_machine: BTreeMap<String, Vec<usize>>,
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
        for recipe in &file.recipes {
            if recipe
                .display_name
                .as_ref()
                .is_some_and(|name| name.trim().is_empty())
            {
                return Err(RecipeLoadError::BlankDisplayName {
                    id: recipe.id.clone(),
                });
            }
            for (field, value) in [
                ("tagline", &recipe.tagline),
                ("blurb", &recipe.blurb),
                ("icon", &recipe.icon),
            ] {
                if value.as_ref().is_some_and(|text| text.trim().is_empty()) {
                    return Err(RecipeLoadError::BlankText {
                        id: recipe.id.clone(),
                        field,
                    });
                }
            }
        }
        let by_id = file
            .recipes
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        let mut by_machine: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, recipe) in file.recipes.iter().enumerate() {
            by_machine
                .entry(recipe.machine.clone())
                .or_default()
                .push(i);
        }
        Ok(Self {
            recipes: file.recipes,
            by_id,
            by_machine,
        })
    }

    pub fn get(&self, id: &str) -> Option<&Recipe> {
        self.by_id.get(id).map(|&i| &self.recipes[i])
    }

    /// Every recipe `kind` can run, **in file order** — empty for a kind no
    /// recipe names, which is what makes such a kind unplaceable
    /// (`world.rs`'s `can_construct`).
    ///
    /// A kind used to name exactly one recipe, and a second was a
    /// construction-time panic. Several are ordinary content since the BBQ
    /// epic: a mixer blends ketchup mix or BBQ mix, a cooking vat cooks
    /// either, and which one a *placed* machine runs is decided by what is
    /// in its buffers and its pipes at the moment a craft starts, never by
    /// the player (`machine.rs`'s `Machine::choose`). This order is the
    /// tiebreak when more than one is satisfiable.
    pub fn for_machine(&self, kind: &str) -> impl Iterator<Item = &Recipe> {
        self.by_machine
            .get(kind)
            .map_or(&[][..], |v| v.as_slice())
            .iter()
            .map(|&i| &self.recipes[i])
    }

    /// Recipes in data-file order — see the struct doc for why this order
    /// matters, not just recipe content.
    pub fn ordered(&self) -> &[Recipe] {
        &self.recipes
    }

    /// Every id in this file that carries a name of its own, as
    /// `(id, display name)` pairs in file order — see
    /// [`Recipe::display_name`]. Recipes that leave the field out are
    /// absent, which is the presentation layer's cue to title-case the id.
    pub fn display_names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.recipes.iter().filter_map(|recipe| {
            recipe
                .display_name
                .as_deref()
                .map(|name| (recipe.id.as_str(), name))
        })
    }

    /// Every id in this file that names its own voxel asset, as
    /// `(id, asset)` pairs in file order — see [`Recipe::icon`]. Absent
    /// ids are the presentation layer's cue to fall back to its own table.
    pub fn icons(&self) -> impl Iterator<Item = (&str, &str)> {
        self.recipes.iter().filter_map(|recipe| {
            recipe
                .icon
                .as_deref()
                .map(|asset| (recipe.id.as_str(), asset))
        })
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

    /// The file the game actually ships, loaded the way `lib.rs` loads it.
    fn shipped() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("bundled data/recipes.ron must parse")
    }

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
            let text =
                EXAMPLE.replacen("schema_version: 3", &format!("schema_version: {older}"), 1);
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

    /// No two recipes on one machine kind take the same set of inputs — in
    /// fact no two of them share a single input id.
    ///
    /// This is what makes `data/recipes.ron`'s file order a tiebreak that
    /// never actually decides anything, and it is the reason there is no
    /// player-facing recipe picker. A machine re-picks, every idle tick,
    /// whichever of its kind's recipes its *current inputs* satisfy
    /// (`super::machine::Machine::choose`), so the plumbing and the hoppers
    /// are the picker: a mixer on the vinegar pipe beside a sugar hopper can
    /// only ever blend ketchup mix, and one on the ketchup pipe beside spice
    /// and molasses hoppers can only ever blend BBQ mix. Neither can be fed
    /// the other's ingredients by accident, because no ingredient is in both
    /// lists.
    ///
    /// Break this and the tiebreak starts mattering: two recipes a machine
    /// could both run means reordering the file silently changes what a
    /// factory makes, and a player would have no way to ask for the other
    /// one. If a future sauce genuinely needs two recipes with overlapping
    /// inputs on one kind, that is the moment the picker has to be designed
    /// — not the moment to quietly reorder this file.
    #[test]
    fn recipes_on_a_kind_take_disjoint_inputs() {
        let registry = RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("data/recipes.ron must parse");
        let mut kinds: Vec<&str> = registry
            .ordered()
            .iter()
            .map(|r| r.machine.as_str())
            .collect();
        kinds.sort_unstable();
        kinds.dedup();
        for kind in kinds {
            let recipes: Vec<&Recipe> = registry.for_machine(kind).collect();
            for (index, recipe) in recipes.iter().enumerate() {
                for other in &recipes[index + 1..] {
                    let inputs = |r: &Recipe| -> Vec<String> {
                        r.inputs
                            .iter()
                            .chain(r.fluid_inputs.iter())
                            .map(|(id, _)| id.clone())
                            .collect()
                    };
                    let mine = inputs(recipe);
                    let shared: Vec<String> = inputs(other)
                        .into_iter()
                        .filter(|id| mine.contains(id))
                        .collect();
                    assert!(
                        shared.is_empty(),
                        "'{}' and '{}' both run on the {kind} and both consume {shared:?}. \
                         Recipes sharing a machine kind must take disjoint inputs, so that \
                         what a machine is plumbed to *is* which recipe it runs and file \
                         order never decides anything. See this test's doc comment.",
                        recipe.id,
                        other.id
                    );
                }
            }
        }
    }

    /// The reason [`Recipe::display_name`] exists: the presentation layer
    /// title-cases an id to label it, and an acronym comes out of that
    /// mangled ("Bbq"). These four ids are the ones in the shipped file
    /// that name themselves, so the assertion is on the names rather than
    /// only on the mechanism.
    #[test]
    fn the_bbq_ids_carry_their_own_names_because_title_casing_mangles_an_acronym() {
        let registry = shipped();
        let names: BTreeMap<&str, &str> = registry.display_names().collect();
        assert_eq!(names.get("bbq_mix"), Some(&"BBQ Mix"));
        assert_eq!(names.get("bbq"), Some(&"BBQ"));
        assert_eq!(names.get("bottled_bbq"), Some(&"Bottled BBQ"));
        assert_eq!(names.get("boxed_bbq"), Some(&"Boxed BBQ"));
    }

    /// The other half of the same rule: a recipe whose id title-cases
    /// correctly must NOT carry a name, or the file grows a second copy of
    /// every label to keep in step with the first.
    #[test]
    fn a_recipe_whose_id_title_cases_correctly_stays_out_of_the_map() {
        let registry = shipped();
        let names: BTreeMap<&str, &str> = registry.display_names().collect();
        for id in ["tomato_paste", "ketchup_mix", "ketchup", "boxed_ketchup"] {
            assert!(
                !names.contains_key(id),
                "'{id}' title-cases correctly, so naming it in data would be a duplicate                  label to keep in step — omit the field instead"
            );
        }
    }

    #[test]
    fn a_blank_display_name_is_rejected_rather_than_drawn_as_an_empty_label() {
        let text = EXAMPLE.replacen(
            "id: \"tomato_sauce\",",
            "id: \"tomato_sauce\", display_name: Some(\"  \"),",
            1,
        );
        let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
        assert!(
            matches!(err, RecipeLoadError::BlankDisplayName { ref id } if id == "tomato_sauce"),
            "a blank name would ship as an item with no label at all, got {err:?}"
        );
    }

    /// The Cookbook's three prose fields get the same refusal for the same
    /// reason: absent is a real answer the page renders by leaving the line
    /// out, and blank is a line of nothing that only shows up in a frame.
    #[test]
    fn a_blank_cookbook_field_is_rejected_the_way_a_blank_name_is() {
        for field in ["tagline", "blurb", "icon"] {
            let text = EXAMPLE.replacen(
                "id: \"tomato_sauce\",",
                &format!("id: \"tomato_sauce\", {field}: Some(\"  \"),"),
                1,
            );
            let err = RecipeRegistry::from_ron_str(&text).unwrap_err();
            assert!(
                matches!(
                    err,
                    RecipeLoadError::BlankText { ref id, field: got }
                        if id == "tomato_sauce" && got == field
                ),
                "a blank {field} should be refused by name, got {err:?}"
            );
            assert!(err.to_string().contains(field));
        }
    }

    /// The shipped file is the content this feature exists to render, so
    /// what it says is worth pinning here as well as in
    /// `tests/cookbook.rs`: every sauce a player can sell has prose, and
    /// the middle of a chain is allowed not to.
    #[test]
    fn the_shipped_recipes_carry_the_prose_the_cookbook_prints() {
        let registry = shipped();
        for id in ["boxed_ketchup", "boxed_bbq"] {
            let recipe = registry.get(id).expect("shipped");
            assert!(recipe.blurb.is_some(), "{id} has no blurb");
            assert!(recipe.icon.is_some(), "{id} has no icon");
        }
        assert!(
            registry
                .get("tomato_paste")
                .expect("shipped")
                .tagline
                .is_none(),
            "a stage in the middle of a chain does not need a tagline"
        );
    }
}
