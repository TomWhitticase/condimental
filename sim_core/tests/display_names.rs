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
//! What is under test here is the *map*, not any loader — the loaders have
//! their own unit tests. This is the join: three files feed it, the map is
//! what Godot reads, and an id that is absent from it is the signal to
//! title-case rather than a hole.
//!
//! **`data/machines.ron` is the third file**, added by the project owner's
//! 2026-09-21 call to call `warehouse_rack` "Stock Shelving". That is the
//! other reason a name lives in content: not an acronym the formatter
//! cannot spell, but a kind whose id is a key — written into every save,
//! and looked up by `shop.ron`, `storage.ron` and the voxel asset table —
//! and therefore not something to rename because a label changed.

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
        5,
        "only the four BBQ ids and the one renamed machine kind need naming today: {names:?}"
    );
}

/// The machine half of the map, and the case it was added for: the kind id
/// stays `warehouse_rack` because it is a key, and every label a player
/// reads comes from here instead. `main.gd`'s `_kind_display_name` asks
/// this map before it title-cases, so the build menu, the hotbar tooltip
/// and the hover panel all say "Stock Shelving" from this one entry.
#[test]
fn a_machine_kind_may_be_called_something_other_than_its_id() {
    let names = names();
    assert_eq!(
        names.get("warehouse_rack").map(String::as_str),
        Some("Stock Shelving")
    );
    for id in [
        "crusher",
        "mixer",
        "cooking_vat",
        "bottler",
        "packer",
        "dock",
    ] {
        assert!(
            !names.contains_key(id),
            "'{id}' title-cases to what the game calls it, so it stays out of the map"
        );
    }
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
