//! Raw ingredients: what the factory's consumables cost, and how many of
//! each the player currently owns (GAMEPLAY.md §5's first cost axis —
//! "ingredients — recurring, scales directly with production volume").
//!
//! # Why this module exists
//!
//! Until it did, ingredients were **free and infinite**.
//! `PlayerCommand::FillHopper` called `Hopper::fill` and
//! `PlayerCommand::FillFluidTank` called `FluidTank::fill`, each conjuring
//! material out of nothing at the player's keypress. GAMEPLAY.md §5 has
//! named ingredients as one of three cost axes since it was written, and
//! §1's core loop only closes if running the factory *costs* something
//! ongoing — otherwise the only decision money is involved in is which
//! machine to buy next, and a line, once built, prints money.
//!
//! # The shape it copies, deliberately
//!
//! Everything here is [`super::shop`]'s shape with one word changed:
//!
//! | shop | here |
//! |---|---|
//! | [`super::shop::ShopCatalog`] — buildable kinds, from `data/shop.ron` | [`IngredientCatalog`] — raw ids, from `data/ingredients.ron` |
//! | [`super::shop::OwnedEquipment`] — unplaced buildings | [`OwnedIngredients`] — unloaded stock |
//! | [`super::shop::Shop`] — the pair, a plain `World` field | [`Ingredients`] — the pair, a plain `World` field |
//! | `PlayerCommand::Purchase` | `PlayerCommand::BuyIngredient` |
//!
//! …including the parts that matter most: the loader rejects a mismatched
//! `schema_version`, a duplicate id and a price below 1; the file is
//! `include_str!`'d at compile time so resolution is identical under `cargo
//! test`, the Godot editor and an exported build; and **money still lives in
//! exactly one place**. A purchase debits [`super::economy::Economy::money`]
//! through `try_spend`, all-or-nothing, because GAMEPLAY.md §1 allows one
//! currency and therefore one counter.
//!
//! # What is deliberately *not* copied
//!
//! Two ledgers rather than one, and no shared type between them. An owned
//! *kind* is a building you can put down and pick back up; an owned
//! *ingredient* is stock that a machine eats and never returns. They have
//! the same shape — a `BTreeMap<String, u32>` — and not the same meaning,
//! which is the identical argument [`super::shop::OwnedEquipment`] makes for
//! not being an [`super::inventory::Inventory`]. Merging them would let a
//! kind id be spent where an item id was meant.
//!
//! What *is* shared, on the other hand, is this module's own two sides: a
//! solid item and a fluid sit in one catalog and one ledger. [`super::types::
//! ItemId`] and [`super::types::FluidId`] are both plain `String` aliases,
//! the ids never collide, and in both cases the player is buying stock
//! measured in whole units — a delivery of vinegar is not a different kind
//! of transaction from a delivery of sugar.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::recipe::RecipeRegistry;
use super::types::Money;

/// The schema version this build understands. Rejected (not migrated) on
/// mismatch, for the reason [`super::shop::CURRENT_SCHEMA_VERSION`] gives:
/// the file is bundled into the binary at compile time, so a mismatch means
/// data and binary have drifted apart.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// How many units a `BuyIngredient` command buys when the JSON wire form
/// omits `quantity` — one, exactly as `Purchase`'s omitted quantity is one.
pub const DEFAULT_BUY_QUANTITY: u32 = 1;

/// One purchasable raw ingredient and its price per unit.
///
/// `price` is a plain integer for [`super::shop::ShopEntry::price`]'s
/// reason (content is authored in whole units; no float appears on the
/// content path at all), and `i64` rather than `u32` for its reason too — so
/// a negative price *parses* and can be rejected with a message naming the
/// offending id, instead of failing as an anonymous RON type error.
#[derive(Debug, Clone, Deserialize)]
pub struct IngredientEntry {
    /// The item or fluid id, as written in `data/recipes.ron`.
    pub id: String,
    /// Price of **one unit**, in the same money
    /// [`super::economy::Economy::money`] counts. Must be at least 1.
    pub price: i64,
}

impl IngredientEntry {
    pub fn price_money(&self) -> Money {
        Money::from_num(self.price)
    }
}

#[derive(Debug, Deserialize)]
struct IngredientFile {
    schema_version: u32,
    entries: Vec<IngredientEntry>,
}

#[derive(Debug)]
pub enum IngredientLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// Two entries name the same ingredient — a duplicate silently shadows
    /// the other, exactly as it would in `shop.ron`.
    DuplicateId {
        id: String,
    },
    /// A price of zero or less. Free ingredients are the state this module
    /// exists to end, and a negative price would *pay* the player to stock
    /// up.
    DegeneratePrice {
        id: String,
        price: i64,
    },
    /// `data/recipes.ron` needs an ingredient nothing here prices. See
    /// [`IngredientCatalog::validate_covers`] — reported, never a
    /// `World`-construction panic.
    UnpricedIngredient {
        ingredient: String,
        priced: Vec<String>,
    },
}

impl std::fmt::Display for IngredientLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngredientLoadError::Parse(e) => write!(f, "failed to parse ingredients.ron: {e}"),
            IngredientLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "ingredients.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            IngredientLoadError::DuplicateId { id } => write!(
                f,
                "ingredients.ron prices '{id}' more than once — entries are looked up by \
                 name, so a duplicate silently shadows the other"
            ),
            IngredientLoadError::DegeneratePrice { id, price } => write!(
                f,
                "ingredients.ron prices '{id}' at {price} — a price must be at least 1, \
                 since an ingredient that costs nothing is the free-and-infinite supply \
                 this file exists to replace (GAMEPLAY.md §5)"
            ),
            IngredientLoadError::UnpricedIngredient { ingredient, priced } => write!(
                f,
                "data/recipes.ron consumes '{ingredient}', which no recipe produces and \
                 data/ingredients.ron does not price (priced: {}) — a raw ingredient with \
                 no price can never be bought, so the hopper or tank holding it can never \
                 be filled",
                priced.join(", ")
            ),
        }
    }
}

impl std::error::Error for IngredientLoadError {}

const INGREDIENTS_RON: &str = include_str!("../../../data/ingredients.ron");

/// The purchasable raw-ingredient catalog, loaded from
/// `data/ingredients.ron`. Preserves the file's own order, for
/// [`super::shop::ShopCatalog`]'s reason: it is the order a buy menu should
/// list things in, and presentation shouldn't have to invent one.
#[derive(Debug, Default)]
pub struct IngredientCatalog {
    entries: Vec<IngredientEntry>,
    by_id: BTreeMap<String, usize>,
}

impl IngredientCatalog {
    pub fn from_ron_str(text: &str) -> Result<Self, IngredientLoadError> {
        let file: IngredientFile = ron::from_str(text).map_err(IngredientLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(IngredientLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let mut by_id = BTreeMap::new();
        for (index, entry) in file.entries.iter().enumerate() {
            if entry.price < 1 {
                return Err(IngredientLoadError::DegeneratePrice {
                    id: entry.id.clone(),
                    price: entry.price,
                });
            }
            if by_id.insert(entry.id.clone(), index).is_some() {
                return Err(IngredientLoadError::DuplicateId {
                    id: entry.id.clone(),
                });
            }
        }
        Ok(Self {
            entries: file.entries,
            by_id,
        })
    }

    /// The shipped `data/ingredients.ron`. Panics only if that bundled file
    /// is itself invalid — a build-time content bug, not a runtime condition
    /// any caller could recover from.
    pub fn bundled() -> Self {
        Self::from_ron_str(INGREDIENTS_RON)
            .unwrap_or_else(|e| panic!("bundled data/ingredients.ron must load: {e}"))
    }

    /// Every entry, in the data file's order.
    pub fn entries(&self) -> &[IngredientEntry] {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&IngredientEntry> {
        self.by_id.get(id).map(|&i| &self.entries[i])
    }

    /// Priced ids, sorted — a stable order for error messages and tests.
    pub fn ids(&self) -> Vec<String> {
        self.by_id.keys().cloned().collect()
    }

    /// Checks that every **raw** id in `recipes` is priced here — the
    /// cross-file invariant between `recipes.ron` and `ingredients.ron`, the
    /// mirror of [`super::shop::ShopCatalog::validate_covers`].
    ///
    /// **Deliberately not called from `World`'s construction**, unlike that
    /// mirror, and for the reason [`super::contracts`] gives about its own
    /// cross-check: a machine kind with no footprint would make the layout
    /// silently wrong, so it must panic; an unpriced ingredient leaves the
    /// world perfectly coherent — the hopper simply can never be filled —
    /// and every test in this crate that builds a world out of fabricated
    /// recipes would otherwise have to price fabricated ingredients it never
    /// buys. It is run against the shipped files by this module's own test
    /// instead.
    pub fn validate_covers(&self, recipes: &RecipeRegistry) -> Result<(), IngredientLoadError> {
        for id in raw_ingredient_ids(recipes) {
            if !self.by_id.contains_key(&id) {
                return Err(IngredientLoadError::UnpricedIngredient {
                    ingredient: id,
                    priced: self.ids(),
                });
            }
        }
        Ok(())
    }
}

/// Every **raw** ingredient id in a recipe file: an item or fluid that some
/// recipe consumes and no recipe produces, in order of first appearance.
///
/// This is the definition of "raw" the whole crate uses — `world.rs` reads
/// it to decide what a hopper may hold, and this module reads it to decide
/// what has to be priced. Writing it once, here, is what keeps those two
/// from drifting: the alternative ("the first recipe's first input") was
/// true only while the chain had exactly one raw ingredient.
///
/// Order of first appearance rather than sorted, so it is deterministic
/// (ARCHITECTURE.md §3.9) *and* reads as the production line does — the
/// ingredient the front of the line eats comes first.
pub fn raw_ingredient_ids(recipes: &RecipeRegistry) -> Vec<String> {
    let mut produced: Vec<&str> = Vec::new();
    for recipe in recipes.ordered() {
        for (item, _) in &recipe.outputs {
            produced.push(item);
        }
        for (fluid, _) in &recipe.fluid_outputs {
            produced.push(fluid);
        }
    }
    let mut raw: Vec<String> = Vec::new();
    for recipe in recipes.ordered() {
        for (id, _) in recipe.inputs.iter().chain(recipe.fluid_inputs.iter()) {
            if !produced.contains(&id.as_str()) && !raw.contains(id) {
                raw.push(id.clone());
            }
        }
    }
    raw
}

/// How many units of each raw ingredient the player owns and has not yet
/// loaded into a hopper or a tank.
///
/// A `BTreeMap` for [`super::shop::OwnedEquipment`]'s reason — deterministic
/// iteration for a snapshot (ARCHITECTURE.md §3.9). An id absent from the
/// map owns zero; a fresh world owns nothing at all.
#[derive(Debug, Default)]
pub struct OwnedIngredients {
    counts: BTreeMap<String, u32>,
}

impl OwnedIngredients {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self, id: &str) -> u32 {
        self.counts.get(id).copied().unwrap_or(0)
    }

    /// Credits `quantity`. Saturating, never wrapping — a wrap would turn a
    /// purchase into a loss.
    pub fn add(&mut self, id: &str, quantity: u32) {
        if quantity == 0 {
            return;
        }
        let slot = self.counts.entry(id.to_string()).or_insert(0);
        *slot = slot.saturating_add(quantity);
    }

    /// Debits **up to** `quantity`, returning how much was actually taken.
    ///
    /// Unlike [`super::shop::OwnedEquipment::take_one`] this is a clamp
    /// rather than an all-or-nothing refusal, and the difference is the
    /// point: placing a building is one indivisible act, while loading a
    /// hopper is a *quantity* that is already clamped twice over (by what
    /// the player owns, and by the room in the hopper). The caller's rule is
    /// in `World::issue_command`: clamp the request to what is owned, let
    /// `fill` clamp again to its own capacity, then debit **exactly what
    /// `fill` reports it added** — never the requested amount.
    ///
    /// A count that falls to zero is erased rather than left as an explicit
    /// zero, so [`Self::iter`] never reports stock the player has none of.
    pub fn take(&mut self, id: &str, quantity: u32) -> u32 {
        let Some(slot) = self.counts.get_mut(id) else {
            return 0;
        };
        let taken = quantity.min(*slot);
        *slot -= taken;
        if *slot == 0 {
            self.counts.remove(id);
        }
        taken
    }

    /// Every owned ingredient and its count, sorted by id. Only ids actually
    /// bought appear.
    pub fn iter(&self) -> impl Iterator<Item = (&str, u32)> {
        self.counts.iter().map(|(k, &n)| (k.as_str(), n))
    }
}

/// The catalog plus what the player owns of it — `World`'s single
/// ingredients field. Money is not here, for [`super::shop::Shop`]'s reason.
#[derive(Debug)]
pub struct Ingredients {
    pub catalog: IngredientCatalog,
    pub owned: OwnedIngredients,
}

impl Ingredients {
    /// Selling the bundled `data/ingredients.ron`, owning nothing.
    pub fn bundled() -> Self {
        Self::new(IngredientCatalog::bundled())
    }

    pub fn new(catalog: IngredientCatalog) -> Self {
        Self {
            catalog,
            owned: OwnedIngredients::new(),
        }
    }

    /// What `quantity` units of `id` costs, or `None` if the id isn't priced
    /// (an unknown ingredient), the quantity is zero (nothing to buy), or
    /// the total would exceed what [`Money`] can represent.
    ///
    /// That last case is [`super::shop::Shop::cost_of`]'s, verbatim and for
    /// its reason: a quantity arriving from JSON is a `u32`, and a total
    /// past what [`Money`] holds must be refused rather than saturated into
    /// something that could look affordable. `checked_from_num` keeps a
    /// silly quantity a plain no-op.
    pub fn cost_of(&self, id: &str, quantity: u32) -> Option<Money> {
        if quantity == 0 {
            return None;
        }
        let entry = self.catalog.get(id)?;
        let total = entry.price.checked_mul(i64::from(quantity))?;
        Money::checked_from_num(total)
    }

    /// The presentation snapshot — see [`IngredientsSnapshot`].
    ///
    /// `hopper_items` is passed in rather than derived here because it is a
    /// property of the WORLD's recipe set, not of the catalog: an ingredient
    /// is priced whether it is a solid or a fluid, and only a solid can go
    /// in a hopper. [`super::world::World::ingredients_snapshot`] hands over
    /// its own [`super::world::World::raw_items`], which is the single
    /// definition of that set.
    pub fn snapshot(&self, money: Money, hopper_items: &[String]) -> IngredientsSnapshot {
        IngredientsSnapshot {
            money: money.to_num::<f64>(),
            hopper_items: hopper_items.to_vec(),
            entries: self
                .catalog
                .entries()
                .iter()
                .map(|entry| IngredientSnapshot {
                    id: entry.id.clone(),
                    price: entry.price_money().to_num::<f64>(),
                    owned: self.owned.count(&entry.id),
                    affordable: money >= entry.price_money(),
                })
                .collect(),
        }
    }
}

/// A presentation-facing snapshot of the ingredient market (ARCHITECTURE.md
/// §4/§5) — everything a buy panel needs in one object, deliberately
/// field-for-field [`super::shop::ShopSnapshot`] so presentation can drive
/// both from one code path.
///
/// ```json
/// {"money": 2913.0,
///  "hopper_items": ["tomato", "sugar"],
///  "entries": [
///    {"id": "tomato", "price": 1.0, "owned": 40, "affordable": true},
///    {"id": "sugar", "price": 3.0, "owned": 0, "affordable": true}
///  ]}
/// ```
///
/// `hopper_items` is `entries` minus the fluids: the ids a hopper may
/// legally be placed holding (see [`super::world::PlayerCommand::Place`]'s
/// `item` field). It is on the wire because presentation genuinely cannot
/// work it out — an id's price says nothing about whether it is a solid —
/// and because the alternative is a Godot-side list of literals that a
/// third raw ingredient would silently make wrong. Its order is
/// `data/recipes.ron`'s own order of first appearance, so a UI cycling
/// through it presents them in the order the production line uses them.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IngredientsSnapshot {
    pub money: f64,
    pub hopper_items: Vec<String>,
    pub entries: Vec<IngredientSnapshot>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IngredientSnapshot {
    pub id: String,
    pub price: f64,
    pub owned: u32,
    pub affordable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        entries: [
            IngredientEntry(id: "tomato", price: 1),
            IngredientEntry(id: "sugar", price: 3),
        ],
    )"#;

    fn recipes(text: &str) -> RecipeRegistry {
        RecipeRegistry::from_ron_str(text).expect("test recipes must parse")
    }

    #[test]
    fn loads_and_prices_by_id() {
        let catalog = IngredientCatalog::from_ron_str(EXAMPLE).unwrap();
        assert_eq!(catalog.get("sugar").unwrap().price, 3);
        assert!(catalog.get("nothing_like_this").is_none());
        assert_eq!(
            catalog.ids(),
            vec!["sugar".to_string(), "tomato".to_string()]
        );
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 7", 1);
        assert!(matches!(
            IngredientCatalog::from_ron_str(&text).unwrap_err(),
            IngredientLoadError::UnsupportedSchemaVersion {
                found: 7,
                supported: 1
            }
        ));
    }

    #[test]
    fn rejects_a_duplicate_id() {
        let text = EXAMPLE.replacen(r#"id: "sugar""#, r#"id: "tomato""#, 1);
        assert!(matches!(
            IngredientCatalog::from_ron_str(&text).unwrap_err(),
            IngredientLoadError::DuplicateId { .. }
        ));
    }

    #[test]
    fn rejects_a_free_or_negative_ingredient() {
        for price in ["0", "-4"] {
            let text = EXAMPLE.replacen("price: 3", &format!("price: {price}"), 1);
            assert!(matches!(
                IngredientCatalog::from_ron_str(&text).unwrap_err(),
                IngredientLoadError::DegeneratePrice { .. }
            ));
        }
    }

    /// The definition every other module leans on: consumed somewhere,
    /// produced nowhere — across solids and fluids alike.
    #[test]
    fn raw_ingredients_are_what_nothing_produces() {
        let registry = recipes(
            r#"(
                schema_version: 3,
                recipes: [
                    Recipe(id: "paste", inputs: [("tomato", 4)], outputs: [("paste", 1)],
                           machine: "crusher", time_ticks: 10),
                    Recipe(id: "mix", inputs: [("paste", 1), ("sugar", 1)],
                           fluid_inputs: [("vinegar", 2)], outputs: [],
                           fluid_outputs: [("mix", 1)], machine: "mixer", time_ticks: 10),
                    Recipe(id: "sauce", inputs: [], fluid_inputs: [("mix", 1)],
                           outputs: [("sauce", 1)], machine: "bottler", time_ticks: 10),
                ],
            )"#,
        );
        assert_eq!(
            raw_ingredient_ids(&registry),
            vec![
                "tomato".to_string(),
                "sugar".to_string(),
                "vinegar".to_string()
            ],
            "paste and the liquid mix are produced by the chain, so neither is raw"
        );
    }

    /// The shipped files against each other — the check `validate_covers`
    /// deliberately isn't a `World` panic for. This is what would have caught
    /// `sugar` being added to `recipes.ron` with no price beside it.
    #[test]
    fn the_shipped_catalog_prices_every_shipped_raw_ingredient() {
        let registry = recipes(include_str!("../../../data/recipes.ron"));
        IngredientCatalog::bundled()
            .validate_covers(&registry)
            .expect("data/ingredients.ron must price every raw ingredient data/recipes.ron needs");
        assert_eq!(
            raw_ingredient_ids(&registry),
            vec![
                "tomato".to_string(),
                "sugar".to_string(),
                "vinegar".to_string(),
                "spice_mix".to_string(),
                "molasses".to_string(),
                "liquid_smoke".to_string(),
            ],
            "the shipped chain's raw ingredients — if this changed deliberately, price the \
             new one in data/ingredients.ron"
        );
        // Finished ketchup is the case worth stating outright: the BBQ mixer
        // consumes it, so it reads like an input, but the cooking vat
        // produces it, so it is NOT raw — it has no price and no hopper, and
        // a BBQ branch gets it by building enough ketchup line to make it.
        assert!(
            !raw_ingredient_ids(&registry).contains(&"ketchup".to_string()),
            "ketchup is produced by the chain, so a derived sauce consuming it must not make \
             it a purchasable raw"
        );
    }

    #[test]
    fn validate_covers_names_the_ingredient_it_cannot_price() {
        let registry = recipes(
            r#"(
                schema_version: 3,
                recipes: [
                    Recipe(id: "paste", inputs: [("saffron", 1)], outputs: [("paste", 1)],
                           machine: "crusher", time_ticks: 10),
                ],
            )"#,
        );
        let err = IngredientCatalog::bundled()
            .validate_covers(&registry)
            .unwrap_err();
        assert!(
            format!("{err}").contains("saffron"),
            "the error must name the unpriced ingredient, got: {err}"
        );
    }

    #[test]
    fn owned_stock_credits_and_debits_by_clamped_quantity() {
        let mut owned = OwnedIngredients::new();
        assert_eq!(owned.count("tomato"), 0);
        owned.add("tomato", 10);
        assert_eq!(owned.take("tomato", 4), 4);
        assert_eq!(owned.count("tomato"), 6);
        assert_eq!(
            owned.take("tomato", 100),
            6,
            "taking more than owned is a clamp"
        );
        assert_eq!(owned.count("tomato"), 0);
        assert_eq!(owned.take("tomato", 1), 0, "an exhausted id takes nothing");
        assert_eq!(
            owned.iter().count(),
            0,
            "an exhausted id is erased, not left at zero"
        );
        assert_eq!(owned.take("never_bought", 1), 0);
    }

    #[test]
    fn cost_refuses_a_zero_quantity_an_unknown_id_and_an_unrepresentable_total() {
        let ingredients = Ingredients::new(IngredientCatalog::from_ron_str(EXAMPLE).unwrap());
        assert_eq!(ingredients.cost_of("tomato", 5), Some(Money::from_num(5)));
        assert_eq!(ingredients.cost_of("tomato", 0), None);
        assert_eq!(ingredients.cost_of("saffron", 1), None);
        // A huge but representable total is a price like any other
        // (unaffordable, not refused); past `Money`'s 2^47 it is refused.
        assert_eq!(
            ingredients.cost_of("tomato", u32::MAX),
            Some(Money::from_num(i64::from(u32::MAX)))
        );
        let dear = Ingredients::new(
            IngredientCatalog::from_ron_str(
                r#"(schema_version: 1, entries: [IngredientEntry(id: "saffron", price: 40000)])"#,
            )
            .unwrap(),
        );
        assert_eq!(dear.cost_of("saffron", u32::MAX), None);
    }

    #[test]
    fn the_snapshot_reports_every_priced_ingredient_including_unowned_ones() {
        let mut ingredients = Ingredients::new(IngredientCatalog::from_ron_str(EXAMPLE).unwrap());
        ingredients.owned.add("tomato", 12);
        let hopper_items = ["tomato".to_string(), "sugar".to_string()];
        let snapshot = ingredients.snapshot(Money::from_num(2), &hopper_items);
        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(
            snapshot.hopper_items, hopper_items,
            "the hopper-legal ids are reported verbatim, not re-derived from the catalog"
        );
        assert_eq!(snapshot.entries[0].id, "tomato");
        assert_eq!(snapshot.entries[0].owned, 12);
        assert!(snapshot.entries[0].affordable);
        assert_eq!(snapshot.entries[1].id, "sugar");
        assert_eq!(snapshot.entries[1].owned, 0);
        assert!(
            !snapshot.entries[1].affordable,
            "3 each, with 2 in the bank"
        );
    }
}
