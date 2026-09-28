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
//! - **Nothing to save** about where stock stands. (A rack's *filter* is
//!   saved since version 6 — see the next section — but that is a choice
//!   the player made, not an allocation.)
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
//! # A rack may be told to hold one ingredient
//!
//! The owner asked for it on 2026-09-28: click a rack, pick one ingredient,
//! and that rack holds nothing else. It is the one piece of per-rack state
//! the warehouse has, and it is a **reservation of room, not a binding of
//! stock** — which is what keeps everything above true.
//!
//! A rack filtered to `id` puts its bays in a pool of their own that only
//! `id` may stand in. Every unfiltered rack's bays stay in the shared pool.
//! An ingredient's bay demand is still [`StorageCatalog::bays_for`] its
//! holding; it stands in its own reserved bays first, and only what is left
//! over draws on the shared pool ([`StorageCatalog::shared_need`]). The rule
//! is then the old one with the shared pool in place of the whole floor:
//!
//! ```text
//! cap(id) = base_cap + slot_stack * (reserved(id) + shared - shared bays other ids need)
//! ```
//!
//! Still derived, still a fold over the ledger, still nothing stored about
//! which bay holds what. The filter itself is saved (`SavedRack::filter`),
//! because it is a choice the player made and nothing can re-derive it.
//!
//! A change of filter is **refused when it would strand stock** — when the
//! bays it moves out of the shared pool are ones the stock is standing in —
//! for exactly the reason a rack holding stock up cannot be picked up (see
//! `World::try_remove`). Clearing a filter never strands anything: its bays
//! all return to the shared pool, which can only make room.
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
use super::types::ItemId;

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

/// The bays the placed racks supply, split by what may stand in them: the
/// **shared** pool every ingredient draws on, and the bays **reserved** for
/// one ingredient by a rack filtered to it (see this module's doc).
///
/// Built fresh from the floor whenever it is asked for
/// (`World::bay_pool`), never kept: it is a fold over the racks and their
/// filters, so a stored copy could only ever be a stale one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BayPool {
    shared: u32,
    reserved: BTreeMap<ItemId, u32>,
}

impl BayPool {
    /// A floor of `bays` bays, none of them reserved — the whole warehouse
    /// before any rack was given a filter.
    pub fn unfiltered(bays: u32) -> Self {
        Self {
            shared: bays,
            reserved: BTreeMap::new(),
        }
    }

    /// Adds one rack's `bays` to the pool: to the shared pool when it has
    /// no filter, to `filter`'s reservation when it has one.
    pub fn add_rack(&mut self, bays: u32, filter: Option<&str>) {
        match filter {
            None => self.shared = self.shared.saturating_add(bays),
            Some(id) => {
                let entry = self.reserved.entry(id.to_string()).or_insert(0);
                *entry = entry.saturating_add(bays);
            }
        }
    }

    /// Takes one rack's `bays` back out — the pool as it would be without
    /// that rack, which is what a removal or a change of filter is judged
    /// against.
    pub fn remove_rack(&mut self, bays: u32, filter: Option<&str>) {
        match filter {
            None => self.shared = self.shared.saturating_sub(bays),
            Some(id) => {
                if let Some(entry) = self.reserved.get_mut(id) {
                    *entry = entry.saturating_sub(bays);
                    if *entry == 0 {
                        self.reserved.remove(id);
                    }
                }
            }
        }
    }

    /// Bays any ingredient may stand in.
    pub fn shared(&self) -> u32 {
        self.shared
    }

    /// Bays only `id` may stand in.
    pub fn reserved_for(&self, id: &str) -> u32 {
        self.reserved.get(id).copied().unwrap_or(0)
    }

    /// Every bay on the floor, shared and reserved alike.
    pub fn total(&self) -> u32 {
        self.reserved
            .values()
            .fold(self.shared, |sum, &bays| sum.saturating_add(bays))
    }
}

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

    /// Bays every current holding occupies between them, reserved and
    /// shared alike — the warehouse's "bays in use".
    pub fn bays_used(&self, owned: &OwnedIngredients) -> u32 {
        owned
            .iter()
            .map(|(_, count)| self.bays_for(count))
            .fold(0, u32::saturating_add)
    }

    /// How many **shared** bays a holding of `count` units of `id` needs:
    /// its bay demand less the bays reserved for it, which it stands in
    /// first.
    pub fn shared_need(&self, id: &str, count: u32, pool: &BayPool) -> u32 {
        self.bays_for(count).saturating_sub(pool.reserved_for(id))
    }

    /// Shared bays every current holding needs between them.
    pub fn shared_used(&self, owned: &OwnedIngredients, pool: &BayPool) -> u32 {
        owned
            .iter()
            .map(|(id, count)| self.shared_need(id, count, pool))
            .fold(0, u32::saturating_add)
    }

    /// Whether every holding has a bay to stand in on this floor — the
    /// condition every purchase, removal, change of filter and load keeps
    /// true, and so the premise of [`Self::cap_for`]'s invariant.
    pub fn fits(&self, owned: &OwnedIngredients, pool: &BayPool) -> bool {
        self.shared_used(owned, pool) <= pool.shared()
    }

    /// The most of `id` the player may hold right now, given everything
    /// else they hold and the bays in `pool`.
    ///
    /// The bays available to `id` are the ones reserved for it plus the
    /// shared ones nothing else is standing in — so an id's own stock never
    /// counts against its own ceiling, and this is what makes the invariant
    /// below true rather than merely usually true. With no rack filtered,
    /// `pool` is all shared and this is the rule the warehouse shipped with.
    ///
    /// # The invariant
    ///
    /// `cap_for(id) >= count(id)`, always, for every id, **while
    /// [`Self::fits`] holds**. Every purchase is clamped to this cap, which
    /// keeps `fits` true; every removal and change of filter that would
    /// break `fits` is refused; a load that breaks it is rejected. And if
    /// the shared bays everything needs fit the shared pool, the shared bays
    /// left for `id` are at least the ones `id` already needs, so the cap is
    /// at least what is already held. Stock can therefore never become
    /// retroactively illegal. `tests/storage.rs` asserts it after every step
    /// of a scripted game.
    pub fn cap_for(&self, id: &str, owned: &OwnedIngredients, pool: &BayPool) -> u32 {
        let used_by_others = self
            .shared_used(owned, pool)
            .saturating_sub(self.shared_need(id, owned.count(id), pool));
        let free = pool
            .reserved_for(id)
            .saturating_add(pool.shared().saturating_sub(used_by_others));
        self.base_cap
            .saturating_add(self.slot_stack.saturating_mul(free))
    }

    /// What stands in each occupied bay of each rack — the render's view of
    /// the warehouse. `racks` is every rack's bay count and filter, in tile
    /// order; the answer is one list per rack, in the same order, of
    /// `(ingredient, units in this bay)` for its occupied bays in bay order.
    ///
    /// Each ingredient's bays are its holding above [`Self::base_cap`] cut
    /// into [`Self::slot_stack`]s, the last usually part full. They fill the
    /// racks filtered to that ingredient first, in tile order; whatever is
    /// left joins the shared run — ingredients in sorted id order (the
    /// order [`OwnedIngredients::iter`] walks) — which fills the unfiltered
    /// racks in tile order. So the whole assignment is a pure function of
    /// the ledger, the floor plan and the filters, and is therefore
    /// deterministic (ARCHITECTURE.md §3.9) — see this module's doc for the
    /// trade this makes against a bay that stays bound to one ingredient.
    ///
    /// When [`Self::fits`] holds, every bay of stock appears exactly once.
    /// When it does not (no state a player can reach), the stock with
    /// nowhere to stand is simply not drawn.
    pub fn rack_contents(
        &self,
        owned: &OwnedIngredients,
        racks: &[(u32, Option<&str>)],
    ) -> Vec<Vec<(ItemId, u32)>> {
        let mut out: Vec<Vec<(ItemId, u32)>> = vec![Vec::new(); racks.len()];
        let mut shared_run = Vec::new();
        for (id, count) in owned.iter() {
            let mut bays = Vec::new();
            let mut above = count.saturating_sub(self.base_cap);
            while above > 0 {
                let units = above.min(self.slot_stack);
                bays.push(units);
                above -= units;
            }
            let mut bays = bays.into_iter();
            for (rack, &(capacity, filter)) in racks.iter().enumerate() {
                if filter != Some(id) {
                    continue;
                }
                while (out[rack].len() as u32) < capacity {
                    let Some(units) = bays.next() else { break };
                    out[rack].push((id.to_string(), units));
                }
            }
            shared_run.extend(bays.map(|units| (id.to_string(), units)));
        }
        let mut shared_run = shared_run.into_iter();
        for (rack, &(capacity, filter)) in racks.iter().enumerate() {
            if filter.is_some() {
                continue;
            }
            while (out[rack].len() as u32) < capacity {
                let Some(bay) = shared_run.next() else { break };
                out[rack].push(bay);
            }
        }
        out
    }

    /// How many more units of `id` will fit — [`Self::cap_for`] less what
    /// is already held. The number `BuyIngredient` clamps a request to.
    pub fn room_for(&self, id: &str, owned: &OwnedIngredients, pool: &BayPool) -> u32 {
        self.cap_for(id, owned, pool)
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
/// A rack holds no stock of its own — capacity is a property of *how many*
/// racks stand on the floor, and the stock itself stays in the one ledger it
/// has always been in ([`super::ingredients::OwnedIngredients`]). Splitting
/// stock across rack entities would give the simulation two places to be
/// wrong about how much tomato the player owns.
///
/// What it does carry is the player's one choice about it: the ingredient
/// its bays are reserved for, if any (see this module's doc). `None` is a
/// rack any raw may stand in, which is how every rack is placed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WarehouseRack {
    pub filter: Option<ItemId>,
}

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
        assert_eq!(
            catalog.cap_for("tomato", &owned, &BayPool::unfiltered(0)),
            20
        );
        assert_eq!(
            catalog.cap_for("sugar", &owned, &BayPool::unfiltered(0)),
            20
        );
        assert_eq!(
            catalog.cap_for("never_bought", &owned, &BayPool::unfiltered(0)),
            20
        );
        assert_eq!(
            catalog.room_for("tomato", &owned, &BayPool::unfiltered(0)),
            0
        );
        assert_eq!(
            catalog.room_for("sugar", &owned, &BayPool::unfiltered(0)),
            15
        );
    }

    #[test]
    fn a_rack_raises_every_ids_ceiling_at_once() {
        let catalog = catalog();
        let owned = OwnedIngredients::new();
        // Six bays of fifty, on top of the free twenty.
        assert_eq!(
            catalog.cap_for("tomato", &owned, &BayPool::unfiltered(6)),
            320
        );
        assert_eq!(
            catalog.cap_for("sugar", &owned, &BayPool::unfiltered(6)),
            320
        );
    }

    #[test]
    fn free_bays_are_a_shared_pool() {
        let catalog = catalog();
        // Tomato at 320 stands in all six bays.
        let owned = owned(&[("tomato", 320)]);
        assert_eq!(catalog.bays_used(&owned), 6);
        // …so sugar is back to its free allowance, and tomato's own ceiling
        // is unchanged by its own occupancy.
        assert_eq!(
            catalog.cap_for("sugar", &owned, &BayPool::unfiltered(6)),
            20
        );
        assert_eq!(
            catalog.cap_for("tomato", &owned, &BayPool::unfiltered(6)),
            320
        );
        assert_eq!(
            catalog.room_for("tomato", &owned, &BayPool::unfiltered(6)),
            0
        );
    }

    #[test]
    fn a_freed_bay_is_available_immediately() {
        let catalog = catalog();
        // One unit over the base takes a whole bay…
        let over = owned(&[("tomato", 21)]);
        assert_eq!(catalog.bays_used(&over), 1);
        assert_eq!(
            catalog.cap_for("sugar", &over, &BayPool::unfiltered(6)),
            20 + 50 * 5
        );
        // …and giving that unit back returns it, with no unbinding step.
        let at_base = owned(&[("tomato", 20)]);
        assert_eq!(catalog.bays_used(&at_base), 0);
        assert_eq!(
            catalog.cap_for("sugar", &at_base, &BayPool::unfiltered(6)),
            20 + 50 * 6
        );
    }

    #[test]
    fn rack_contents_fills_whole_bays_then_a_part_full_one() {
        let catalog = catalog();
        // 20 free, then 50 + 50 + 5.
        let owned = owned(&[("tomato", 125)]);
        let racks = catalog.rack_contents(&owned, &[(6, None)]);
        assert_eq!(
            racks,
            vec![vec![
                ("tomato".to_string(), 50),
                ("tomato".to_string(), 50),
                ("tomato".to_string(), 5),
            ]]
        );
        assert_eq!(racks[0].len() as u32, catalog.bays_used(&owned));
    }

    #[test]
    fn rack_contents_is_sorted_and_skips_stock_under_the_base() {
        let catalog = catalog();
        let owned = owned(&[("tomato", 30), ("sugar", 20), ("vinegar", 21)]);
        // Sugar sits entirely in its free allowance, so it takes no bay.
        assert_eq!(
            catalog.rack_contents(&owned, &[(6, None)]),
            vec![vec![("tomato".to_string(), 10), ("vinegar".to_string(), 1)]]
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
                    catalog.cap_for(id, &owned, &BayPool::unfiltered(bays)) >= owned.count(id),
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
        assert_eq!(
            catalog.cap_for("sugar", &owned, &BayPool::unfiltered(0)),
            20
        );
        assert_eq!(
            catalog.room_for("sugar", &owned, &BayPool::unfiltered(0)),
            20
        );
    }

    fn pool(racks: &[(u32, Option<&str>)]) -> BayPool {
        let mut pool = BayPool::default();
        for &(bays, filter) in racks {
            pool.add_rack(bays, filter);
        }
        pool
    }

    #[test]
    fn a_pool_splits_shared_from_reserved_and_takes_a_rack_back_out() {
        let mut pool = pool(&[(6, None), (6, Some("tomato")), (6, Some("tomato"))]);
        assert_eq!(pool.shared(), 6);
        assert_eq!(pool.reserved_for("tomato"), 12);
        assert_eq!(pool.reserved_for("sugar"), 0);
        assert_eq!(pool.total(), 18);
        pool.remove_rack(6, Some("tomato"));
        pool.remove_rack(6, Some("tomato"));
        assert_eq!(pool, BayPool::unfiltered(6));
    }

    #[test]
    fn a_filtered_rack_raises_only_its_own_ingredients_ceiling() {
        let catalog = catalog();
        let owned = OwnedIngredients::new();
        let pool = pool(&[(6, Some("tomato"))]);
        assert_eq!(catalog.cap_for("tomato", &owned, &pool), 320);
        assert_eq!(catalog.cap_for("sugar", &owned, &pool), 20);
    }

    #[test]
    fn stock_stands_in_its_reserved_bays_before_the_shared_ones() {
        let catalog = catalog();
        let pool = pool(&[(6, None), (6, Some("tomato"))]);
        // Tomato at 320 needs six bays: all six of its own, none shared…
        let owned = owned(&[("tomato", 320)]);
        assert_eq!(catalog.shared_used(&owned, &pool), 0);
        assert_eq!(catalog.cap_for("sugar", &owned, &pool), 320);
        // …and it may still spill into the shared pool above that.
        assert_eq!(catalog.cap_for("tomato", &owned, &pool), 620);
        let more = owned_with(&owned, "tomato", 100);
        assert_eq!(catalog.shared_used(&more, &pool), 2);
        assert_eq!(catalog.cap_for("sugar", &more, &pool), 220);
        assert!(catalog.fits(&more, &pool));
    }

    #[test]
    fn a_reserved_bay_is_never_room_for_anything_else() {
        let catalog = catalog();
        // Nothing held at all, and still sugar gets only its free allowance.
        let owned = owned(&[("sugar", 20)]);
        let pool = pool(&[(6, Some("tomato"))]);
        assert_eq!(catalog.room_for("sugar", &owned, &pool), 0);
    }

    #[test]
    fn fits_is_false_exactly_when_shared_stock_has_nowhere_to_stand() {
        let catalog = catalog();
        // Tomato in all six shared bays: reserving that rack for sugar would
        // leave six bays of tomato standing nowhere.
        let owned = owned(&[("tomato", 320)]);
        assert!(catalog.fits(&owned, &pool(&[(6, None)])));
        assert!(!catalog.fits(&owned, &pool(&[(6, Some("sugar"))])));
        // Reserving it for tomato itself is fine: the same bays, the same
        // stock in them.
        assert!(catalog.fits(&owned, &pool(&[(6, Some("tomato"))])));
    }

    #[test]
    fn rack_contents_puts_reserved_stock_on_its_own_rack_and_the_rest_in_tile_order() {
        let catalog = catalog();
        // Tomato 20 + 50*7 + 10: eight bays. Sugar 20 + 30: one bay.
        let owned = owned(&[("tomato", 380), ("sugar", 50)]);
        let racks = catalog.rack_contents(&owned, &[(6, None), (6, Some("tomato")), (6, None)]);
        let tomato = |units| ("tomato".to_string(), units);
        // The filtered rack takes tomato's first six bays…
        assert_eq!(racks[1], vec![tomato(50); 6]);
        // …and the shared run (sugar before tomato, sorted) fills the
        // unfiltered racks in tile order.
        assert_eq!(
            racks[0],
            vec![("sugar".to_string(), 30), tomato(50), tomato(10)]
        );
        assert!(racks[2].is_empty());
    }

    #[test]
    fn a_holding_is_never_over_its_own_cap_on_any_fitting_floor() {
        let catalog = catalog();
        let owned = owned(&[("tomato", 320), ("sugar", 120), ("vinegar", 1)]);
        let floors = [
            pool(&[(6, Some("tomato")), (6, None)]),
            pool(&[(6, None), (6, Some("sugar"))]),
            pool(&[(6, Some("tomato")), (6, Some("sugar")), (6, None)]),
            pool(&[(6, None), (6, None)]),
        ];
        for floor in &floors {
            assert!(catalog.fits(&owned, floor), "{floor:?} should fit");
            for id in ["tomato", "sugar", "vinegar", "never_bought"] {
                assert!(
                    catalog.cap_for(id, &owned, floor) >= owned.count(id),
                    "{id} over its cap on {floor:?}"
                );
            }
        }
    }

    fn owned_with(base: &OwnedIngredients, id: &str, extra: u32) -> OwnedIngredients {
        let mut owned = OwnedIngredients::new();
        for (held, count) in base.iter() {
            owned.add(held, count);
        }
        owned.add(id, extra);
        owned
    }
}
