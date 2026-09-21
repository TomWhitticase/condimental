//! Raw-stock capacity: how much of each ingredient the player may hold, and
//! the warehouse racks that raise the ceiling (the warehouse-rack sprint,
//! ticket W1).
//!
//! # Why this module exists
//!
//! Until it did, [`super::ingredients::OwnedIngredients`] was **unbounded**.
//! `PlayerCommand::BuyIngredient` priced a quantity, spent the money and
//! credited the lot, and nothing anywhere said no. The only limit on stock
//! was the balance — and a balance is what a working factory produces, so
//! the limit evaporated at exactly the moment the player stopped noticing
//! it. GAMEPLAY.md §5 names ingredients as the cost axis that "scales
//! directly with production volume"; a one-off stockpile turns that into a
//! one-off purchase.
//!
//! A cap that is only a number would have been a worse answer than no cap:
//! the player would meet a refusal with nothing to do about it. So the
//! ceiling is a *building*. `data/storage.ron` gives every id a free
//! [`StorageCatalog::base_cap`], and everything above that occupies a
//! **bay** supplied by a placed warehouse rack — a thing you buy, carry a
//! footprint for, and can see filling up on the factory floor (requirement
//! R3 of the sprint plan, which is what W2's art and W3's render are for).
//!
//! # Bays are derived, never assigned
//!
//! The sprint plan's first shape bound each bay to an ingredient id and
//! persisted the binding, on the reasoning that "because the bays are the
//! state, there is no allocation function that can drift". This module
//! takes the stronger version of that argument: with **no** allocation
//! state at all, there is nothing to drift *from*. A holding's bay demand is
//! a pure function of the holding —
//!
//! ```text
//! bays_for(count) = ceil(max(0, count - base_cap) / slot_stack)
//! ```
//!
//! — so [`StorageCatalog::bays_used`] is a fold over the ledger and
//! [`StorageCatalog::cap_for`] is that fold minus the asking id's own share.
//! Three things fall out, each of which the stored form had to be careful
//! about:
//!
//! - **Nothing to save.** A rack carries no state, so `save.rs` gains an
//!   empty enum variant and `SAVE_SCHEMA_VERSION` does not move.
//! - **A bay frees itself.** Consume the tomato and the bay is available to
//!   sugar on the next command, rather than staying reserved for an
//!   ingredient the player holds one unit of.
//! - **Nothing can be locked out.** `base_cap` is outside the bay
//!   arithmetic entirely, so a warehouse packed to the roof with tomato
//!   still leaves every other raw its free allowance. The shared pool is
//!   the intended pressure; the base is what stops that pressure becoming a
//!   deadlock.
//!
//! What the derived form gives up is a *stable* home for a bay: which
//! physical bay draws which ingredient can change when a neighbour's stock
//! moves. That is a presentation concern and it is settled in one place —
//! `World::storage_snapshot` walks ids in sorted order and racks in tile
//! order — so the render is deterministic (ARCHITECTURE.md §3.9) even
//! though it is not sticky.
//!
//! # What a rack deliberately does not do
//!
//! **A rack has nothing to do with belts or pipes.** It is not a source, it
//! is not a sink, it has no output item and no facing rule; the tick never
//! looks at one. Requirement R6 of the sprint plan says so and the rest of
//! the plan contradicted itself about it, so it is worth stating where
//! someone will read it: the hopper remains the only thing that meters raw
//! onto a belt, and a rack that could discharge would simply *be* a hopper
//! with a bigger bin. Storage and feeding are separate jobs. `tests/
//! storage.rs` pins the absence.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::ingredients::OwnedIngredients;

/// The schema version this build understands. Rejected (not migrated) on
/// mismatch, for [`super::ingredients::CURRENT_SCHEMA_VERSION`]'s reason:
/// the file is bundled into the binary at compile time, so a mismatch means
/// data and binary have drifted apart.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The buildable kind id of the shipped warehouse rack — `data/shop.ron`'s
/// entry, `data/machines.ron`'s footprint and `data/storage.ron`'s bay
/// count all key off this.
///
/// A named constant for [`super::hopper::HOPPER_KIND`]'s reason: `world.rs`
/// matches on it when placing, and a string literal repeated across three
/// modules is a typo waiting to be a silent refusal.
pub const WAREHOUSE_RACK_KIND: &str = "warehouse_rack";

/// One buildable kind that supplies storage bays, and how many it supplies.
#[derive(Debug, Clone, Deserialize)]
pub struct RackKind {
    /// The buildable kind id, as written in `data/shop.ron`.
    pub id: String,
    /// Bays one placed building of this kind contributes. Must be at least
    /// one — a rack that stores nothing is a building with no reason to be
    /// bought.
    pub bays: u32,
}

#[derive(Debug, Deserialize)]
struct StorageFile {
    schema_version: u32,
    base_cap: u32,
    slot_stack: u32,
    racks: Vec<RackKind>,
}

#[derive(Debug)]
pub enum StorageLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// Two entries name the same kind — a duplicate silently shadows the
    /// other, exactly as it would in `shop.ron`.
    DuplicateKind {
        id: String,
    },
    /// A rack that supplies no bays.
    DegenerateBays {
        id: String,
    },
    /// A `base_cap` or `slot_stack` of zero. Either one makes the whole
    /// system degenerate in a way that is worse than having no cap:
    /// `base_cap: 0` means nothing can be bought until a rack is placed,
    /// and a rack cannot be placed without ingredients to justify it;
    /// `slot_stack: 0` means a rack adds no capacity however many bays it
    /// has.
    DegenerateCapacity {
        field: &'static str,
    },
}

impl std::fmt::Display for StorageLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageLoadError::Parse(e) => write!(f, "failed to parse storage.ron: {e}"),
            StorageLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "storage.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            StorageLoadError::DuplicateKind { id } => write!(
                f,
                "storage.ron lists '{id}' more than once — racks are looked up by kind, \
                 so a duplicate silently shadows the other"
            ),
            StorageLoadError::DegenerateBays { id } => write!(
                f,
                "storage.ron gives '{id}' zero bays — a rack that stores nothing is a \
                 building with nothing to sell it on; remove the entry instead"
            ),
            StorageLoadError::DegenerateCapacity { field } => write!(
                f,
                "storage.ron sets {field} to zero — see this module's doc for why both \
                 halves of the capacity rule must be positive"
            ),
        }
    }
}

impl std::error::Error for StorageLoadError {}

const STORAGE_RON: &str = include_str!("../../../data/storage.ron");

/// The stock-capacity rules, loaded from `data/storage.ron` — a `World`
/// field beside [`super::ingredients::Ingredients`], which it caps.
#[derive(Debug)]
pub struct StorageCatalog {
    base_cap: u32,
    slot_stack: u32,
    bays_by_kind: BTreeMap<String, u32>,
}

impl StorageCatalog {
    pub fn from_ron_str(text: &str) -> Result<Self, StorageLoadError> {
        let file: StorageFile = ron::from_str(text).map_err(StorageLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(StorageLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if file.base_cap == 0 {
            return Err(StorageLoadError::DegenerateCapacity { field: "base_cap" });
        }
        if file.slot_stack == 0 {
            return Err(StorageLoadError::DegenerateCapacity {
                field: "slot_stack",
            });
        }
        let mut bays_by_kind = BTreeMap::new();
        for rack in &file.racks {
            if rack.bays == 0 {
                return Err(StorageLoadError::DegenerateBays {
                    id: rack.id.clone(),
                });
            }
            if bays_by_kind.insert(rack.id.clone(), rack.bays).is_some() {
                return Err(StorageLoadError::DuplicateKind {
                    id: rack.id.clone(),
                });
            }
        }
        Ok(Self {
            base_cap: file.base_cap,
            slot_stack: file.slot_stack,
            bays_by_kind,
        })
    }

    /// The shipped `data/storage.ron`. Panics only if that bundled file is
    /// itself invalid — a build-time content bug, not a runtime condition
    /// any caller could recover from.
    pub fn bundled() -> Self {
        Self::from_ron_str(STORAGE_RON)
            .unwrap_or_else(|e| panic!("bundled data/storage.ron must load: {e}"))
    }

    /// Free capacity per ingredient id, with no rack placed anywhere.
    pub fn base_cap(&self) -> u32 {
        self.base_cap
    }

    /// Units of one ingredient a single bay holds.
    pub fn slot_stack(&self) -> u32 {
        self.slot_stack
    }

    /// Bays one placed building of `kind` supplies — zero for every kind
    /// that is not a rack, which is most of them. `World::storage_bays`
    /// sums this over the topology.
    pub fn bays_of_kind(&self, kind: &str) -> u32 {
        self.bays_by_kind.get(kind).copied().unwrap_or(0)
    }

    /// Whether `kind` supplies any storage at all — what `try_remove` asks
    /// before it bothers checking whether a removal would strand stock.
    pub fn is_rack(&self, kind: &str) -> bool {
        self.bays_by_kind.contains_key(kind)
    }

    /// Every storage-supplying kind and its bay count, sorted by id.
    pub fn rack_kinds(&self) -> impl Iterator<Item = (&str, u32)> {
        self.bays_by_kind.iter().map(|(k, &n)| (k.as_str(), n))
    }

    /// How many bays a holding of `count` units occupies: none at all up to
    /// [`Self::base_cap`], then one per [`Self::slot_stack`] above it,
    /// rounded **up** — a part-full bay is still a bay taken.
    pub fn bays_for(&self, count: u32) -> u32 {
        let above = count.saturating_sub(self.base_cap);
        above.div_ceil(self.slot_stack)
    }

    /// Bays every current holding occupies between them.
    pub fn bays_used(&self, owned: &OwnedIngredients) -> u32 {
        owned
            .iter()
            .map(|(_, count)| self.bays_for(count))
            .fold(0, u32::saturating_add)
    }

    /// The most of `id` the player may hold right now, given everything
    /// else they hold and the `total_bays` their placed racks supply.
    ///
    /// The bays available to `id` are the ones nothing else is standing in,
    /// which is `total_bays` less what the *other* ids need — so an id's own
    /// stock never counts against its own ceiling, and this is what makes
    /// the invariant below true rather than merely usually true.
    ///
    /// # The invariant
    ///
    /// `cap_for(id) >= count(id)`, always, for every id. Every purchase is
    /// clamped so `bays_used <= total_bays`, which means the bays left for
    /// `id` are at least the ones `id` is already standing in, which means
    /// the cap is at least what is already held. Stock can therefore never
    /// become retroactively illegal — no removal, no purchase of something
    /// else and no load can leave a holding over its own ceiling.
    /// `tests/storage.rs` asserts it after every step of a scripted game.
    pub fn cap_for(&self, id: &str, owned: &OwnedIngredients, total_bays: u32) -> u32 {
        let used_by_others = self
            .bays_used(owned)
            .saturating_sub(self.bays_for(owned.count(id)));
        let free = total_bays.saturating_sub(used_by_others);
        self.base_cap
            .saturating_add(self.slot_stack.saturating_mul(free))
    }

    /// What stands in each occupied bay, as `(ingredient, units in this
    /// bay)` in flat bay order — the render's view of the warehouse.
    ///
    /// Ingredients in sorted id order (the order
    /// [`OwnedIngredients::iter`] walks), each taking consecutive bays, the
    /// last of them part full. The caller slices this run across racks in
    /// tile order, so the whole assignment is a pure function of the ledger
    /// and the floor plan and is therefore deterministic
    /// (ARCHITECTURE.md §3.9) — see this module's doc for the trade this
    /// makes against a bay that stays bound to one ingredient.
    ///
    /// Exactly [`Self::bays_used`] entries long, which
    /// `World::storage_snapshot` relies on to know the run fits.
    pub fn bay_contents(&self, owned: &OwnedIngredients) -> Vec<(String, u32)> {
        let mut bays = Vec::new();
        for (id, count) in owned.iter() {
            let mut above = count.saturating_sub(self.base_cap);
            while above > 0 {
                let units = above.min(self.slot_stack);
                bays.push((id.to_string(), units));
                above -= units;
            }
        }
        bays
    }

    /// How many more units of `id` will fit — [`Self::cap_for`] less what
    /// is already held. The number `BuyIngredient` clamps a request to.
    pub fn room_for(&self, id: &str, owned: &OwnedIngredients, total_bays: u32) -> u32 {
        self.cap_for(id, owned, total_bays)
            .saturating_sub(owned.count(id))
    }
}

impl Default for StorageCatalog {
    fn default() -> Self {
        Self::bundled()
    }
}

/// The body component of a placed warehouse rack.
///
/// Deliberately empty. A rack holds nothing of its own — capacity is a
/// property of *how many* racks stand on the floor, and the stock itself
/// stays in the one ledger it has always been in
/// ([`super::ingredients::OwnedIngredients`]). Splitting stock across rack
/// entities would give the simulation two places to be wrong about how much
/// tomato the player owns.
///
/// It exists because every placement carries a body the topology pass can
/// classify by, and because a marker component is how `hecs` answers "is the
/// thing on this tile a rack".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarehouseRack;

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        base_cap: 20,
        slot_stack: 50,
        racks: [ RackKind(id: "warehouse_rack", bays: 6) ],
    )"#;

    fn catalog() -> StorageCatalog {
        StorageCatalog::from_ron_str(EXAMPLE).expect("test storage must parse")
    }

    fn owned(pairs: &[(&str, u32)]) -> OwnedIngredients {
        let mut owned = OwnedIngredients::new();
        for (id, count) in pairs {
            owned.add(id, *count);
        }
        owned
    }

    #[test]
    fn loads_the_bundled_file() {
        let catalog = StorageCatalog::bundled();
        assert!(catalog.base_cap() > 0);
        assert!(catalog.slot_stack() > 0);
        assert_eq!(catalog.bays_of_kind(WAREHOUSE_RACK_KIND), 6);
        assert!(catalog.is_rack(WAREHOUSE_RACK_KIND));
        assert!(!catalog.is_rack("hopper"));
        assert_eq!(catalog.bays_of_kind("hopper"), 0);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 9", 1);
        assert!(matches!(
            StorageCatalog::from_ron_str(&text),
            Err(StorageLoadError::UnsupportedSchemaVersion { found: 9, .. })
        ));
    }

    #[test]
    fn rejects_a_duplicate_kind() {
        let text = EXAMPLE.replacen(
            r#"RackKind(id: "warehouse_rack", bays: 6)"#,
            r#"RackKind(id: "warehouse_rack", bays: 6), RackKind(id: "warehouse_rack", bays: 2)"#,
            1,
        );
        assert!(matches!(
            StorageCatalog::from_ron_str(&text),
            Err(StorageLoadError::DuplicateKind { .. })
        ));
    }

    #[test]
    fn rejects_a_rack_with_no_bays() {
        let text = EXAMPLE.replacen("bays: 6", "bays: 0", 1);
        assert!(matches!(
            StorageCatalog::from_ron_str(&text),
            Err(StorageLoadError::DegenerateBays { .. })
        ));
    }

    #[test]
    fn rejects_a_degenerate_capacity() {
        let no_base = EXAMPLE.replacen("base_cap: 20", "base_cap: 0", 1);
        assert!(matches!(
            StorageCatalog::from_ron_str(&no_base),
            Err(StorageLoadError::DegenerateCapacity { field: "base_cap" })
        ));
        let no_stack = EXAMPLE.replacen("slot_stack: 50", "slot_stack: 0", 1);
        assert!(matches!(
            StorageCatalog::from_ron_str(&no_stack),
            Err(StorageLoadError::DegenerateCapacity {
                field: "slot_stack"
            })
        ));
    }

    #[test]
    fn stock_up_to_the_base_needs_no_bay() {
        let catalog = catalog();
        assert_eq!(catalog.bays_for(0), 0);
        assert_eq!(catalog.bays_for(1), 0);
        assert_eq!(catalog.bays_for(20), 0);
    }

    #[test]
    fn a_part_full_bay_is_still_a_bay() {
        let catalog = catalog();
        assert_eq!(catalog.bays_for(21), 1);
        assert_eq!(catalog.bays_for(70), 1);
        assert_eq!(catalog.bays_for(71), 2);
        assert_eq!(catalog.bays_for(120), 2);
    }

    #[test]
    fn without_a_rack_every_id_gets_the_base_and_no_more() {
        let catalog = catalog();
        let owned = owned(&[("tomato", 20), ("sugar", 5)]);
        assert_eq!(catalog.cap_for("tomato", &owned, 0), 20);
        assert_eq!(catalog.cap_for("sugar", &owned, 0), 20);
        assert_eq!(catalog.cap_for("never_bought", &owned, 0), 20);
        assert_eq!(catalog.room_for("tomato", &owned, 0), 0);
        assert_eq!(catalog.room_for("sugar", &owned, 0), 15);
    }

    #[test]
    fn a_rack_raises_every_ids_ceiling_at_once() {
        let catalog = catalog();
        let owned = OwnedIngredients::new();
        // Six bays of fifty, on top of the free twenty.
        assert_eq!(catalog.cap_for("tomato", &owned, 6), 320);
        assert_eq!(catalog.cap_for("sugar", &owned, 6), 320);
    }

    #[test]
    fn free_bays_are_a_shared_pool() {
        let catalog = catalog();
        // Tomato at 320 stands in all six bays.
        let owned = owned(&[("tomato", 320)]);
        assert_eq!(catalog.bays_used(&owned), 6);
        // …so sugar is back to its free allowance, and tomato's own ceiling
        // is unchanged by its own occupancy.
        assert_eq!(catalog.cap_for("sugar", &owned, 6), 20);
        assert_eq!(catalog.cap_for("tomato", &owned, 6), 320);
        assert_eq!(catalog.room_for("tomato", &owned, 6), 0);
    }

    #[test]
    fn a_freed_bay_is_available_immediately() {
        let catalog = catalog();
        // One unit over the base takes a whole bay…
        let over = owned(&[("tomato", 21)]);
        assert_eq!(catalog.bays_used(&over), 1);
        assert_eq!(catalog.cap_for("sugar", &over, 6), 20 + 50 * 5);
        // …and giving that unit back returns it, with no unbinding step.
        let at_base = owned(&[("tomato", 20)]);
        assert_eq!(catalog.bays_used(&at_base), 0);
        assert_eq!(catalog.cap_for("sugar", &at_base, 6), 20 + 50 * 6);
    }

    #[test]
    fn bay_contents_fills_whole_bays_then_a_part_full_one() {
        let catalog = catalog();
        // 20 free, then 50 + 50 + 5.
        let owned = owned(&[("tomato", 125)]);
        assert_eq!(
            catalog.bay_contents(&owned),
            vec![
                ("tomato".to_string(), 50),
                ("tomato".to_string(), 50),
                ("tomato".to_string(), 5),
            ]
        );
        assert_eq!(
            catalog.bay_contents(&owned).len() as u32,
            catalog.bays_used(&owned)
        );
    }

    #[test]
    fn bay_contents_is_sorted_and_skips_stock_under_the_base() {
        let catalog = catalog();
        let owned = owned(&[("tomato", 30), ("sugar", 20), ("vinegar", 21)]);
        // Sugar sits entirely in its free allowance, so it takes no bay.
        assert_eq!(
            catalog.bay_contents(&owned),
            vec![("tomato".to_string(), 10), ("vinegar".to_string(), 1)]
        );
    }

    /// The property `world.rs` relies on: a holding is never over its own
    /// ceiling, whatever else is held and however few bays exist.
    #[test]
    fn a_holding_is_never_over_its_own_cap() {
        let catalog = catalog();
        let owned = owned(&[("tomato", 320), ("sugar", 20), ("vinegar", 1)]);
        for bays in [6, 7, 12] {
            for id in ["tomato", "sugar", "vinegar"] {
                assert!(
                    catalog.cap_for(id, &owned, bays) >= owned.count(id),
                    "{id} over its cap at {bays} bays"
                );
            }
        }
    }

    /// Losing racks cannot make a holding illegal *if* the removal was
    /// legal — but the arithmetic must not panic or wrap when asked about
    /// a state that should be impossible, so the saturating path is pinned.
    #[test]
    fn fewer_bays_than_are_used_saturates_rather_than_wrapping() {
        let catalog = catalog();
        let owned = owned(&[("tomato", 320)]);
        assert_eq!(catalog.cap_for("sugar", &owned, 0), 20);
        assert_eq!(catalog.room_for("sugar", &owned, 0), 20);
    }
}
