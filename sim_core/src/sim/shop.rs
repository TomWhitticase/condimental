//! The shop and the player's equipment inventory (GAMEPLAY.md §1's core
//! loop, §4's "equipment purchases *are* the tech tree") — the "Shop &
//! Inventory" epic's ticket 1.
//!
//! Two things live here, and they are deliberately one module because
//! neither is useful without the other:
//!
//! 1. [`ShopCatalog`] — what is buyable and what it costs, loaded and
//!    validated from `data/shop.ron` exactly the way [`super::recipe`] and
//!    [`super::machines`] load theirs (schema-version check, degenerate
//!    entry rejected with a clear error, bundled at compile time via
//!    `include_str!` so resolution is identical under `cargo test`, the
//!    Godot editor and an exported build).
//! 2. [`OwnedEquipment`] — how many of each kind the player currently owns,
//!    which starts at zero for everything and only ever grows through a
//!    purchase.
//!
//! [`Shop`] pairs them, and is owned by [`super::world::World`] as a plain
//! field for precisely the reason `Economy` and `FluidNetwork` are: there is
//! exactly one shop and one equipment inventory per world, so ECS machinery
//! would buy nothing. Money itself stays where it already lives, on
//! [`super::economy::Economy`] — GAMEPLAY.md §1 is emphatic that there is
//! no second currency, so there must not be a second money counter either;
//! a purchase debits the same `Economy::money` a contract delivery credits.
//! [`super::world::World::issue_command`] is what holds both sides at once
//! (the same way its `FillHopper` arm already holds the hopper and the
//! economy at once).
//!
//! # What this module does not do yet
//!
//! Nothing consumes an owned count. Every machine in the shipped chain is
//! still built unconditionally by `World::new`, whether or not it has been
//! bought, and a purchase's only observable effect is money going down and
//! a count going up. That is this ticket's deliberate scope: the economy
//! layer is real and fully simulated first, and a later epic rewires
//! placement to actually spend the inventory. Keeping those two apart means
//! this half is testable in complete isolation and moves no existing tile.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::staff::CrewEntry;
use super::types::Fx;

/// The schema version this build of sim_core understands. Bump this and
/// reject/migrate on mismatch (ARCHITECTURE.md §4) whenever [`ShopEntry`]'s
/// shape changes in a way older data can't satisfy.
///
/// v1 is this file's first version. Mismatched data is **rejected, not
/// migrated**, for exactly the reason [`super::recipe::CURRENT_SCHEMA_VERSION`]
/// and [`super::machines::CURRENT_SCHEMA_VERSION`] give: `shop.ron` is
/// bundled into the binary at compile time, so there is one such file and it
/// is updated in the same commit as this constant — a version mismatch means
/// data and binary have drifted apart, which deserves a loud error.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// How many of a kind a `Purchase` command buys when the JSON wire form
/// omits `quantity` — the "player clicked the buy button once" case, the
/// same convenience `fill_hopper`'s optional `amount` provides.
pub const DEFAULT_PURCHASE_QUANTITY: u32 = 1;

/// One purchasable kind and its price.
///
/// `price` is a plain integer, not an `Fx`, for the reason
/// [`super::recipe::Recipe::fluid_inputs`] gives for its own quantities:
/// content is authored in whole units, and keeping the data file
/// integer-only means no float ever appears on the content path at all
/// (ARCHITECTURE.md §3.9 is happier if that conversion simply doesn't
/// exist). The conversion to fixed-point money happens once, here, in
/// [`ShopEntry::price_money`].
///
/// It is `i64` rather than `u32` specifically so a negative price *parses*
/// and can then be rejected with a message naming the offending kind
/// ([`ShopLoadError::DegeneratePrice`]) — a `u32` field would fail as an
/// anonymous RON type error instead, which is a worse thing to hand someone
/// editing content.
#[derive(Debug, Clone, Deserialize)]
pub struct ShopEntry {
    /// The buildable kind id — every id in `data/machines.ron`, plus the
    /// belt and pipe tiles, which are infrastructure rather than machines.
    /// See `data/shop.ron`'s own header for that distinction.
    pub id: String,
    /// Price in whole money units, in the same money
    /// [`super::economy::Economy::money`] counts. Must be at least 1.
    pub price: i64,
}

impl ShopEntry {
    /// This entry's price as fixed-point money, ready to compare against
    /// [`super::economy::Economy::money`].
    pub fn price_money(&self) -> Fx {
        Fx::from_num(self.price)
    }
}

#[derive(Debug, Deserialize)]
struct ShopFile {
    schema_version: u32,
    entries: Vec<ShopEntry>,
}

#[derive(Debug)]
pub enum ShopLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// Two entries name the same kind. Entries are looked up by id, so a
    /// duplicate silently shadows the other — the same reasoning
    /// [`super::machines::MachineLoadError::DuplicateKind`] uses.
    DuplicateKind {
        id: String,
    },
    /// A price of zero or less. Free equipment would make GAMEPLAY.md §1's
    /// whole loop vacuous, and a negative price would *pay* the player to
    /// buy — caught here so no purchase path downstream has to defend
    /// against either.
    DegeneratePrice {
        id: String,
        price: i64,
    },
    /// `data/machines.ron` defines a kind that `data/shop.ron` doesn't
    /// price. Reported rather than silently defaulted, for the same reason
    /// [`super::machines::MachineLoadError::UnknownKindInRecipe`] is: a kind
    /// with no price is a kind the player can never buy, which is a content
    /// mistake, not a design statement.
    UnpricedMachineKind {
        machine: String,
        priced: Vec<String>,
    },
}

impl std::fmt::Display for ShopLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShopLoadError::Parse(e) => write!(f, "failed to parse shop.ron: {e}"),
            ShopLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "shop.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            ShopLoadError::DuplicateKind { id } => write!(
                f,
                "shop.ron prices kind '{id}' more than once — entries are looked up by \
                 name, so a duplicate silently shadows the other"
            ),
            ShopLoadError::DegeneratePrice { id, price } => write!(
                f,
                "shop.ron prices kind '{id}' at {price} — a price must be at least 1, \
                 since buying equipment is what money is for (GAMEPLAY.md §1)"
            ),
            ShopLoadError::UnpricedMachineKind { machine, priced } => write!(
                f,
                "data/machines.ron defines machine kind '{machine}', which data/shop.ron \
                 does not price (priced kinds: {}) — every buildable kind needs a price, or \
                 the player can never buy it",
                priced.join(", ")
            ),
        }
    }
}

impl std::error::Error for ShopLoadError {}

/// The purchasable catalog, loaded from `data/shop.ron`.
///
/// Preserves the data file's own order (see [`Self::entries`]), unlike
/// [`super::machines::MachineRegistry`] — not because order carries
/// simulation meaning the way `recipes.ron`'s does, but because it is the
/// order a build menu should list things in, and presentation shouldn't have
/// to invent one.
#[derive(Debug, Default)]
pub struct ShopCatalog {
    entries: Vec<ShopEntry>,
    by_id: BTreeMap<String, usize>,
}

/// The bundled shop content, embedded at compile time for exactly the reason
/// [`super::machines`] embeds `machines.ron`: resolving it must be identical
/// under `cargo test`, the Godot editor and an exported build, none of which
/// share a working directory.
const SHOP_RON: &str = include_str!("../../../data/shop.ron");

impl ShopCatalog {
    pub fn from_ron_str(text: &str) -> Result<Self, ShopLoadError> {
        let file: ShopFile = ron::from_str(text).map_err(ShopLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ShopLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }

        let mut by_id = BTreeMap::new();
        for (index, entry) in file.entries.iter().enumerate() {
            if entry.price < 1 {
                return Err(ShopLoadError::DegeneratePrice {
                    id: entry.id.clone(),
                    price: entry.price,
                });
            }
            if by_id.insert(entry.id.clone(), index).is_some() {
                return Err(ShopLoadError::DuplicateKind {
                    id: entry.id.clone(),
                });
            }
        }
        Ok(Self {
            entries: file.entries,
            by_id,
        })
    }

    /// The shipped `data/shop.ron`. Panics only if that bundled file is
    /// itself invalid, which is a build-time content bug, not a runtime
    /// condition any caller could recover from — same contract as
    /// [`super::machines::MachineRegistry::bundled`].
    pub fn bundled() -> Self {
        Self::from_ron_str(SHOP_RON)
            .unwrap_or_else(|e| panic!("bundled data/shop.ron must load: {e}"))
    }

    /// Every entry, in the data file's order — the order a build menu lists
    /// them in.
    pub fn entries(&self) -> &[ShopEntry] {
        &self.entries
    }

    pub fn get(&self, kind: &str) -> Option<&ShopEntry> {
        self.by_id.get(kind).map(|&i| &self.entries[i])
    }

    /// Priced kind ids, sorted — a stable order for error messages and
    /// tests, independent of file order.
    pub fn kind_ids(&self) -> Vec<String> {
        self.by_id.keys().cloned().collect()
    }

    /// Checks that every machine kind is priced here — the cross-file
    /// invariant between `machines.ron` and `shop.ron`, the mirror of
    /// [`super::machines::MachineRegistry::validate_covers`]. Returns the
    /// first unpriced kind rather than a list, matching how every other
    /// loader in `sim/` reports the first problem it hits.
    ///
    /// Note the direction: every *machine* kind must be priced, but not
    /// every priced kind is a machine — `belt` and `pipe` are priced
    /// infrastructure with no entry in `machines.ron` at all.
    pub fn validate_covers(
        &self,
        machines: &super::machines::MachineRegistry,
    ) -> Result<(), ShopLoadError> {
        for machine in machines.kind_ids() {
            if !self.by_id.contains_key(&machine) {
                return Err(ShopLoadError::UnpricedMachineKind {
                    machine,
                    priced: self.kind_ids(),
                });
            }
        }
        Ok(())
    }
}

/// How many of each buildable kind the player owns and has not yet placed.
///
/// A `BTreeMap`, not a `HashMap`, so iteration order is deterministic
/// (ARCHITECTURE.md §3.9) — this is shared simulation state that a snapshot
/// is built from, and a snapshot whose field order wobbles between runs is
/// exactly the kind of thing that makes a rendered scene useless as a
/// before/after check. A kind absent from the map owns zero; nothing is
/// pre-seeded, so a fresh world owns nothing at all.
///
/// Deliberately *not* [`super::inventory::Inventory`]: that type is a
/// capacity-bounded store of item ids, and this is an unbounded ledger of
/// buildable-kind ids. They share a shape but not a meaning, and conflating
/// them would let a kind id be passed where an item id is meant.
#[derive(Debug, Default)]
pub struct OwnedEquipment {
    counts: BTreeMap<String, u32>,
}

impl OwnedEquipment {
    pub fn new() -> Self {
        Self::default()
    }

    /// How many of `kind` the player owns — zero for a kind never bought,
    /// and for a kind that doesn't exist.
    pub fn count(&self, kind: &str) -> u32 {
        self.counts.get(kind).copied().unwrap_or(0)
    }

    /// Credits `quantity` of `kind`. Saturating rather than wrapping: at the
    /// count where this could matter the player has bigger problems, and a
    /// wrap would turn a purchase into a loss.
    pub fn add(&mut self, kind: &str, quantity: u32) {
        if quantity == 0 {
            return;
        }
        let slot = self.counts.entry(kind.to_string()).or_insert(0);
        *slot = slot.saturating_add(quantity);
    }

    /// Debits **exactly one** of `kind`, returning whether there was one to
    /// debit (the "Real Placement" epic's ticket 1: placing something spends
    /// it out of this ledger, and removing it credits one back through
    /// [`Self::add`]).
    ///
    /// All-or-nothing and silent about failure, like every other refusal in
    /// this crate: an unowned kind leaves the ledger byte-identical, and
    /// `World` turns that into the same no-op an unaffordable purchase gets.
    /// A count that falls to zero is erased rather than left as an explicit
    /// zero, so [`Self::iter`] never reports a kind the player has none of.
    pub fn take_one(&mut self, kind: &str) -> bool {
        let Some(slot) = self.counts.get_mut(kind) else {
            return false;
        };
        if *slot == 0 {
            return false;
        }
        *slot -= 1;
        if *slot == 0 {
            self.counts.remove(kind);
        }
        true
    }

    /// Every owned kind and its count, in sorted-by-kind order. Only kinds
    /// actually bought appear; use [`Self::count`] for a specific one.
    pub fn iter(&self) -> impl Iterator<Item = (&str, u32)> {
        self.counts.iter().map(|(k, &n)| (k.as_str(), n))
    }
}

/// The catalog plus what the player owns of it — `World`'s single shop
/// field. Money is not here; see this module's doc for why it stays on
/// `Economy`.
#[derive(Debug)]
pub struct Shop {
    pub catalog: ShopCatalog,
    pub owned: OwnedEquipment,
}

impl Shop {
    /// A shop selling the bundled `data/shop.ron`, owning nothing.
    pub fn bundled() -> Self {
        Self::new(ShopCatalog::bundled())
    }

    pub fn new(catalog: ShopCatalog) -> Self {
        Self {
            catalog,
            owned: OwnedEquipment::new(),
        }
    }

    /// What `quantity` of `kind` costs, or `None` if the kind isn't priced
    /// (an unknown kind), the quantity is zero (nothing to buy), or the
    /// total would exceed what [`Fx`] can represent.
    ///
    /// That last case is not paranoia: `Fx` is `I16F16`, so money saturates
    /// somewhere around 32767, and a purchase quantity is a `u32` arriving
    /// from JSON. Computing the total in `i64` first and refusing to
    /// represent an out-of-range one keeps a silly quantity a plain no-op
    /// instead of a saturated total that could look affordable.
    pub fn cost_of(&self, kind: &str, quantity: u32) -> Option<Fx> {
        if quantity == 0 {
            return None;
        }
        let entry = self.catalog.get(kind)?;
        let total = entry.price.checked_mul(i64::from(quantity))?;
        if total > i64::from(Fx::MAX.int().to_num::<i32>()) {
            return None;
        }
        Some(Fx::from_num(total))
    }

    /// Builds the presentation snapshot — see [`ShopSnapshot`] for the exact
    /// JSON shape. `money` is passed in rather than read from a field
    /// because this type deliberately doesn't own one.
    /// `crew_of` answers "what crew does this kind need" — `World` passes
    /// its machine registry's `staff` list; a kind the registry does not
    /// know (belt, pipe) answers empty.
    pub fn snapshot(&self, money: Fx, crew_of: impl Fn(&str) -> Vec<CrewEntry>) -> ShopSnapshot {
        ShopSnapshot {
            money: money.to_num::<f64>(),
            entries: self
                .catalog
                .entries()
                .iter()
                .map(|entry| ShopEntrySnapshot {
                    kind: entry.id.clone(),
                    price: entry.price_money().to_num::<f64>(),
                    owned: self.owned.count(&entry.id),
                    affordable: money >= entry.price_money(),
                    crew: crew_of(&entry.id),
                })
                .collect(),
        }
    }
}

/// A presentation-facing snapshot of the shop (ARCHITECTURE.md §4/§5) —
/// everything a build menu needs to draw itself in one object, so it never
/// has to correlate two calls. Serialized by
/// `SimBridge::shop_snapshot_json`, whose doc comment carries the same
/// shape.
///
/// ```json
/// {"money": 120.0,
///  "entries": [
///    {"kind": "belt", "price": 5.0, "owned": 3, "affordable": true},
///    {"kind": "cooking_vat", "price": 200.0, "owned": 0, "affordable": false}
///  ]}
/// ```
///
/// `entries` is in `data/shop.ron`'s own order (the intended menu order) and
/// lists **every** priced kind, including ones the player owns none of —
/// a build menu shows what you could buy, not only what you have.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ShopSnapshot {
    /// The player's money, repeated here so a build menu is one call. Same
    /// value, and same `f64`-at-the-boundary rule, as
    /// [`super::economy::EconomySnapshot::money`]: this has already crossed
    /// from fixed-point sim state into a presentation-only value that never
    /// feeds back in (ARCHITECTURE.md §3.9).
    pub money: f64,
    pub entries: Vec<ShopEntrySnapshot>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ShopEntrySnapshot {
    /// The buildable kind id, e.g. "belt" or "cooking_vat".
    pub kind: String,
    /// Unit price. `f64` so it compares directly against `money` above
    /// without presentation having to know the data file stores an integer.
    pub price: f64,
    /// How many of this kind the player owns right now — zero until bought.
    pub owned: u32,
    /// Whether **one** of this kind is affordable at the current money.
    /// Computed here rather than in GDScript so the affordability rule lives
    /// in exactly one place; a menu buying several at once should compare
    /// `money` against `price * quantity` itself.
    pub affordable: bool,
    /// The crew this kind needs while running (`machines.ron`'s `staff`,
    /// the staff pools MVP epic), for the build-menu card. Empty for a kind
    /// that asks for nobody — and for the belt and pipe, which are not
    /// machine kinds at all.
    pub crew: Vec<CrewEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::machines::MachineRegistry;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        entries: [
            ShopEntry(id: "belt", price: 5),
            ShopEntry(id: "crusher", price: 75),
        ],
    )"#;

    #[test]
    fn loads_prices_by_kind_id_in_file_order() {
        let catalog = ShopCatalog::from_ron_str(EXAMPLE).unwrap();
        assert_eq!(catalog.get("belt").unwrap().price, 5);
        assert_eq!(catalog.get("crusher").unwrap().price, 75);
        assert!(catalog.get("nonexistent").is_none());
        let ids: Vec<&str> = catalog.entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["belt", "crusher"]);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 99", 1);
        let err = ShopCatalog::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            ShopLoadError::UnsupportedSchemaVersion {
                found: 99,
                supported: 1
            }
        ));
    }

    #[test]
    fn rejects_a_zero_price() {
        let text = EXAMPLE.replacen("price: 75", "price: 0", 1);
        let err = ShopCatalog::from_ron_str(&text).unwrap_err();
        match err {
            ShopLoadError::DegeneratePrice { ref id, price } => {
                assert_eq!(id, "crusher");
                assert_eq!(price, 0);
            }
            other => panic!("expected DegeneratePrice, got {other:?}"),
        }
        assert!(err.to_string().contains("at least 1"));
    }

    #[test]
    fn rejects_a_negative_price() {
        let text = EXAMPLE.replacen("price: 5", "price: -5", 1);
        let err = ShopCatalog::from_ron_str(&text).unwrap_err();
        match err {
            ShopLoadError::DegeneratePrice { ref id, price } => {
                assert_eq!(id, "belt");
                assert_eq!(price, -5);
            }
            other => panic!("expected DegeneratePrice, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_duplicated_kind_id() {
        let text = EXAMPLE.replacen(r#"id: "crusher""#, r#"id: "belt""#, 1);
        let err = ShopCatalog::from_ron_str(&text).unwrap_err();
        match err {
            ShopLoadError::DuplicateKind { ref id } => assert_eq!(id, "belt"),
            other => panic!("expected DuplicateKind, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_ron() {
        let err = ShopCatalog::from_ron_str("not a shop file at all").unwrap_err();
        assert!(matches!(err, ShopLoadError::Parse(_)));
    }

    #[test]
    fn a_machine_kind_with_no_price_is_an_error_not_a_default() {
        let catalog = ShopCatalog::from_ron_str(EXAMPLE).unwrap();
        // The bundled machine registry defines mixer/cooking_vat/etc., none
        // of which this two-entry test catalog prices.
        let err = catalog
            .validate_covers(&MachineRegistry::bundled())
            .unwrap_err();
        match err {
            ShopLoadError::UnpricedMachineKind { ref machine, .. } => {
                assert!(
                    MachineRegistry::bundled().get(machine).is_some(),
                    "the error must name a real machine kind, got '{machine}'"
                );
            }
            other => panic!("expected UnpricedMachineKind, got {other:?}"),
        }
    }

    /// The shipped content's own cross-file invariant: nothing the player
    /// could ever place is missing a price. This is what makes adding a
    /// machine kind to `data/machines.ron` without pricing it a test
    /// failure rather than a mystery in the build menu.
    #[test]
    fn the_bundled_shop_prices_every_bundled_machine_kind() {
        let catalog = ShopCatalog::bundled();
        catalog
            .validate_covers(&MachineRegistry::bundled())
            .expect("every shipped machine kind must be priced in data/shop.ron");
        // And the two pieces of infrastructure that are not machines.
        for infra in ["belt", "pipe"] {
            assert!(
                catalog.get(infra).is_some(),
                "data/shop.ron must price '{infra}'"
            );
        }
    }

    #[test]
    fn a_fresh_inventory_owns_nothing() {
        let shop = Shop::bundled();
        for entry in shop.catalog.entries() {
            assert_eq!(shop.owned.count(&entry.id), 0, "kind '{}'", entry.id);
        }
        assert_eq!(shop.owned.iter().count(), 0);
    }

    #[test]
    fn owned_counts_accumulate_per_kind() {
        let mut owned = OwnedEquipment::new();
        owned.add("belt", 3);
        owned.add("belt", 2);
        owned.add("mixer", 1);
        assert_eq!(owned.count("belt"), 5);
        assert_eq!(owned.count("mixer"), 1);
        assert_eq!(owned.count("packer"), 0);
        // Deterministic (sorted) iteration order, per this type's doc.
        let listed: Vec<(&str, u32)> = owned.iter().collect();
        assert_eq!(listed, vec![("belt", 5), ("mixer", 1)]);
    }

    #[test]
    fn adding_zero_does_not_create_an_entry() {
        let mut owned = OwnedEquipment::new();
        owned.add("belt", 0);
        assert_eq!(owned.count("belt"), 0);
        assert_eq!(owned.iter().count(), 0);
    }

    #[test]
    fn cost_of_multiplies_the_unit_price() {
        let shop = Shop::new(ShopCatalog::from_ron_str(EXAMPLE).unwrap());
        assert_eq!(shop.cost_of("belt", 1), Some(Fx::from_num(5)));
        assert_eq!(shop.cost_of("belt", 4), Some(Fx::from_num(20)));
        assert_eq!(shop.cost_of("crusher", 2), Some(Fx::from_num(150)));
    }

    #[test]
    fn cost_of_refuses_an_unknown_kind_a_zero_quantity_and_an_unrepresentable_total() {
        let shop = Shop::new(ShopCatalog::from_ron_str(EXAMPLE).unwrap());
        assert_eq!(shop.cost_of("fermenter", 1), None);
        assert_eq!(shop.cost_of("belt", 0), None);
        // 5 * u32::MAX is far past what I16F16 can hold, so it is refused
        // rather than saturated into something that might look affordable.
        assert_eq!(shop.cost_of("belt", u32::MAX), None);
    }

    #[test]
    fn snapshot_lists_every_entry_in_file_order_with_affordability() {
        let mut shop = Shop::new(ShopCatalog::from_ron_str(EXAMPLE).unwrap());
        shop.owned.add("belt", 2);
        let snapshot = shop.snapshot(Fx::from_num(10), |_| Vec::new());

        assert_eq!(snapshot.money, 10.0);
        assert_eq!(snapshot.entries.len(), 2);

        assert_eq!(snapshot.entries[0].kind, "belt");
        assert_eq!(snapshot.entries[0].price, 5.0);
        assert_eq!(snapshot.entries[0].owned, 2);
        assert!(snapshot.entries[0].affordable);

        // Unowned kinds are still listed — a build menu shows what you could
        // buy, not only what you have.
        assert_eq!(snapshot.entries[1].kind, "crusher");
        assert_eq!(snapshot.entries[1].owned, 0);
        assert!(!snapshot.entries[1].affordable);
    }

    #[test]
    fn affordability_is_inclusive_of_exactly_enough_money() {
        let shop = Shop::new(ShopCatalog::from_ron_str(EXAMPLE).unwrap());
        let snapshot = shop.snapshot(Fx::from_num(75), |_| Vec::new());
        let crusher = snapshot
            .entries
            .iter()
            .find(|e| e.kind == "crusher")
            .unwrap();
        assert!(
            crusher.affordable,
            "exactly the price must count as affordable"
        );
    }
}
