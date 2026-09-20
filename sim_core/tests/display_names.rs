//! Headless integration test for issue #60: **the HUD called the second
//! sauce "Boxed Bbq".**
//!
//! The presentation layer titles an item by title-casing its id
//! (`main.gd`'s `_kind_display_name`), which is right for every id the game
//! shipped with and wrong for the moment content contained an acronym. The
//! fix keeps the formatter as the fallback and lets content name itself: an
//! optional `display_name` on a recipe or an ingredient entry, gathered by
//! [`World::display_names`] into the one map the wire carries.
//!
//! What is under test here is the *map*, not either loader — the loaders
//! have their own unit tests. This is the join: both files feed one id
//! space, the map is what Godot reads, and an id that is absent from it is
//! the signal to title-case rather than a hole.

use std::collections::BTreeMap;

use sim_core::sim::{RecipeRegistry, World};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("data/recipes.ron must parse")
}

/// An empty world is enough: the map is content read back, not state. That
/// is the point of asserting it on `World::new` — it is available before
/// anything is placed, which is what lets Godot read it once at load.
fn names() -> BTreeMap<String, String> {
    World::new(recipes()).display_names()
}

#[test]
fn every_bbq_id_reaches_the_wire_spelled_the_way_a_player_would_write_it() {
    let names = names();
    assert_eq!(names.get("bbq_mix").map(String::as_str), Some("BBQ Mix"));
    assert_eq!(names.get("bbq").map(String::as_str), Some("BBQ"));
    assert_eq!(
        names.get("bottled_bbq").map(String::as_str),
        Some("Bottled BBQ")
    );
    assert_eq!(
        names.get("boxed_bbq").map(String::as_str),
        Some("Boxed BBQ")
    );
}

/// The map is deliberately small. Every id whose title-case is already
/// right stays out of it, so there is exactly one place each of those
/// labels comes from.
#[test]
fn an_id_that_title_cases_correctly_is_absent_rather_than_repeated() {
    let names = names();
    for id in [
        "tomato",
        "sugar",
        "vinegar",
        "spice_mix",
        "molasses",
        "liquid_smoke",
        "tomato_paste",
        "ketchup_mix",
        "ketchup",
        "bottled_ketchup",
        "boxed_ketchup",
    ] {
        assert!(
            !names.contains_key(id),
            "'{id}' title-cases correctly, so naming it in data would be a second copy of \
             the label to keep in step with the first"
        );
    }
    assert_eq!(
        names.len(),
        4,
        "only the four BBQ ids need naming today: {names:?}"
    );
}

/// Both halves of the id space are in one map. No shipped raw names itself,
/// so this asserts the joining rather than a name: an ingredient id the
/// market sells and a recipe id it does not must both be answerable from
/// the same lookup, with absence meaning "title-case it".
#[test]
fn ingredients_and_recipes_share_one_lookup() {
    let names = names();
    assert!(names.contains_key("bbq"), "a recipe id is in the map");
    assert!(
        !names.contains_key("liquid_smoke"),
        "a raw id is answerable from the same map — absent here, and absence is the \
         instruction to title-case, not a missing entry"
    );
}
