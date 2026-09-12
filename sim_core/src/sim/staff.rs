//! Staff pools (GAMEPLAY.md §7, §5's third cost axis; the staff pools MVP
//! epic, `docs/STAFF_POOLS_PLAN.md`): the catalog of categories
//! (`data/staff.ron`), the per-category pools a running factory claims its
//! crews from, and payday.
//!
//! **A capacity check, not entities.** Staff are aggregate demand versus
//! capacity, the shape `PowerNetwork` will take — nothing per person,
//! nothing per tile, no pathfinding. `World` owns one [`StaffPools`] as a
//! plain field, the way it owns `Economy`.
//!
//! **Per tick, in stable order, whole crew or nothing.** Every tick the
//! pools [`reset`](StaffPools::reset) to headcount; then each machine, in
//! the topology's stable machine order, and each dock after them, asks for
//! its kind's whole crew ([`StaffPools::claim`]) *only on a tick it would
//! otherwise have worked*. A machine that cannot get its full crew takes
//! nothing from any category and does not advance. Because the order is the
//! same every tick, the same machine goes short every tick — no flicker, no
//! oscillation, deterministic (ARCHITECTURE.md §3.9). This is GAMEPLAY.md
//! §11's "first-come-first-served" option, chosen for the MVP because it
//! needs no player-facing control; player-set priority can replace it later
//! without touching the wire.
//!
//! **Wages spend money; hiring does not.** Hire and fire are free (§11,
//! answered provisionally), so the only staff figure on the ledger is the
//! wage bill. On payday ([`StaffPools::tick`]) the bill is paid one head at
//! a time in `staff.ron` order through `Economy::try_spend` — still the only
//! debit, still all-or-nothing per call — and the first head that cannot be
//! paid, and every head after it, leaves. Softer than the whole crew
//! walking out at money 0, stricter than debt, which GAMEPLAY.md §1's one
//! non-negative counter does not allow. A zero-wage head always stays.
//!
//! Integers throughout except the money itself, which is the economy's
//! `Fx` (ARCHITECTURE.md §3.9). Pools are a `Vec` in file order — the panel
//! order and the payday order — never a hash map.

use serde::{Deserialize, Serialize};

use super::economy::Economy;
use super::machines::MachineRegistry;
use super::types::Fx;

/// The `schema_version` this build reads. Content changes never bump it.
const CURRENT_SCHEMA_VERSION: u32 = 1;

const STAFF_RON: &str = include_str!("../../../data/staff.ron");

/// One category as `data/staff.ron` defines it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StaffCategory {
    pub id: String,
    pub name: String,
    /// Whole money units per head per pay period. Zero is legal: a
    /// volunteer category is a legitimate content choice.
    pub wage: u32,
    pub starting_headcount: u32,
}

#[derive(Debug, Deserialize)]
struct StaffFile {
    schema_version: u32,
    pay_period_ticks: u64,
    categories: Vec<StaffCategory>,
}

#[derive(Debug)]
pub enum StaffLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    NoCategories,
    DuplicateCategory {
        id: String,
    },
    EmptyName {
        id: String,
    },
    ZeroPayPeriod,
    /// `machines.ron` names a category this file does not define
    /// ([`StaffCatalog::validate_machines`]).
    UnknownCategoryInMachine {
        kind: String,
        category: String,
        known: Vec<String>,
    },
    /// One kind lists the same category twice.
    DuplicateCategoryInMachine {
        kind: String,
        category: String,
    },
    /// A crew line asking for nobody.
    ZeroCrewCount {
        kind: String,
        category: String,
    },
}

impl std::fmt::Display for StaffLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StaffLoadError::Parse(e) => write!(f, "failed to parse staff.ron: {e}"),
            StaffLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "staff.ron schema_version {found} is not supported by this build (supports \
                 {supported}) — migrate the data file or update sim_core"
            ),
            StaffLoadError::NoCategories => write!(
                f,
                "staff.ron defines no categories — a factory with nobody to hire could \
                 never run a machine that asks for a crew"
            ),
            StaffLoadError::DuplicateCategory { id } => write!(
                f,
                "staff.ron defines category '{id}' more than once — categories are looked \
                 up by id, so a duplicate silently shadows the other"
            ),
            StaffLoadError::EmptyName { id } => write!(
                f,
                "staff.ron category '{id}' has an empty name — the name is what the staff \
                 panel prints, and an id is not a player-facing word"
            ),
            StaffLoadError::ZeroPayPeriod => write!(
                f,
                "staff.ron pay_period_ticks must be at least 1 — a zero period is a payday \
                 every tick, which is a wage per tick, not a wage per period"
            ),
            StaffLoadError::UnknownCategoryInMachine {
                kind,
                category,
                known,
            } => write!(
                f,
                "machines.ron kind '{kind}' asks for staff category '{category}', which \
                 data/staff.ron does not define (known categories: {}) — add it there, or \
                 fix the kind's `staff` list",
                known.join(", ")
            ),
            StaffLoadError::DuplicateCategoryInMachine { kind, category } => write!(
                f,
                "machines.ron kind '{kind}' lists staff category '{category}' twice — \
                 merge the two lines into one count"
            ),
            StaffLoadError::ZeroCrewCount { kind, category } => write!(
                f,
                "machines.ron kind '{kind}' asks for 0 of staff category '{category}' — \
                 drop the line rather than asking for nobody"
            ),
        }
    }
}

impl std::error::Error for StaffLoadError {}

/// The loaded `data/staff.ron`: the categories in file order and the pay
/// period. Content only; the live headcounts live on [`StaffPools`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffCatalog {
    pay_period_ticks: u64,
    categories: Vec<StaffCategory>,
}

impl StaffCatalog {
    pub fn from_ron_str(text: &str) -> Result<Self, StaffLoadError> {
        let file: StaffFile = ron::from_str(text).map_err(StaffLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(StaffLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if file.pay_period_ticks == 0 {
            return Err(StaffLoadError::ZeroPayPeriod);
        }
        if file.categories.is_empty() {
            return Err(StaffLoadError::NoCategories);
        }
        for (i, category) in file.categories.iter().enumerate() {
            if category.name.trim().is_empty() {
                return Err(StaffLoadError::EmptyName {
                    id: category.id.clone(),
                });
            }
            if file.categories[..i].iter().any(|c| c.id == category.id) {
                return Err(StaffLoadError::DuplicateCategory {
                    id: category.id.clone(),
                });
            }
        }
        Ok(Self {
            pay_period_ticks: file.pay_period_ticks,
            categories: file.categories,
        })
    }

    /// The shipped `data/staff.ron`. Panics only if that bundled file is
    /// itself invalid — a build-time content bug.
    pub fn bundled() -> Self {
        Self::from_ron_str(STAFF_RON)
            .unwrap_or_else(|e| panic!("bundled data/staff.ron must load: {e}"))
    }

    pub fn pay_period_ticks(&self) -> u64 {
        self.pay_period_ticks
    }

    /// Every category, in file order — the panel order and the payday
    /// order.
    pub fn categories(&self) -> &[StaffCategory] {
        &self.categories
    }

    pub fn get(&self, id: &str) -> Option<&StaffCategory> {
        self.categories.iter().find(|c| c.id == id)
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.categories.iter().position(|c| c.id == id)
    }

    pub fn category_ids(&self) -> Vec<String> {
        self.categories.iter().map(|c| c.id.clone()).collect()
    }

    /// Checks every kind's `staff` list against this catalog: an unknown
    /// category, a category listed twice on one kind, or a line asking for
    /// nobody is an error. **A test, not a construction panic** — the
    /// `validate_covers` pattern: the shipped files are pinned green by
    /// `tests/staff.rs`, and a test fixture is free to build a registry
    /// this catalog does not cover.
    pub fn validate_machines(&self, machines: &MachineRegistry) -> Result<(), StaffLoadError> {
        let mut kinds = machines.kind_ids();
        kinds.sort();
        for kind in kinds {
            let staff = &machines
                .get(&kind)
                .expect("kind_ids() names only kinds the registry holds")
                .staff;
            for (i, (category, count)) in staff.iter().enumerate() {
                if self.get(category).is_none() {
                    return Err(StaffLoadError::UnknownCategoryInMachine {
                        kind: kind.clone(),
                        category: category.clone(),
                        known: self.category_ids(),
                    });
                }
                if staff[..i].iter().any(|(c, _)| c == category) {
                    return Err(StaffLoadError::DuplicateCategoryInMachine {
                        kind: kind.clone(),
                        category: category.clone(),
                    });
                }
                if *count == 0 {
                    return Err(StaffLoadError::ZeroCrewCount {
                        kind: kind.clone(),
                        category: category.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// One line of a kind's crew requirement, as the wire spells it
/// (`query_tile_json`'s machine and dock variants, `shop_snapshot_json`):
///
/// ```json
/// {"category": "manufacturing", "count": 2}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrewEntry {
    pub category: String,
    pub count: u32,
}

/// A kind's `staff` list in wire form.
pub fn crew_entries(staff: &[(String, u32)]) -> Vec<CrewEntry> {
    staff
        .iter()
        .map(|(category, count)| CrewEntry {
            category: category.clone(),
            count: *count,
        })
        .collect()
}

/// One category's live numbers. `headcount` persists; the other three are
/// this tick's tally, reset at the top of every tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CategoryPool {
    pub headcount: u32,
    /// Heads not yet claimed this tick.
    pub available: u32,
    /// Every head asked for this tick, whether or not the ask succeeded.
    pub demand: u32,
    /// Heads actually handed to a machine or dock this tick.
    pub assigned: u32,
}

/// What a machine or dock hands `Machine::tick` / `World::tick` to ask for
/// its crew: the kind's requirement and the pools to claim it from. `None`
/// in place of one of these means "asks for nothing", which is how a unit
/// test — or a kind whose `staff` is empty — runs a machine unstaffed.
pub struct CrewRequest<'a> {
    pub crew: &'a [(String, u32)],
    pub pools: &'a mut StaffPools,
}

/// What one payday did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaydayReport {
    /// What was actually paid, in whole money units.
    pub paid: u32,
    /// How many heads left unpaid.
    pub walked_out: u32,
}

/// The live pools: one [`CategoryPool`] per catalog category, in file
/// order, plus the payday clock. Owned by `World` as a plain field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffPools {
    catalog: StaffCatalog,
    pools: Vec<CategoryPool>,
    next_payday_tick: u64,
    last_bill: u32,
    walked_out_last_payday: u32,
}

impl StaffPools {
    /// The starting crew, with the first payday one period from tick 0.
    pub fn new(catalog: StaffCatalog) -> Self {
        let pools = catalog
            .categories
            .iter()
            .map(|c| CategoryPool {
                headcount: c.starting_headcount,
                available: c.starting_headcount,
                demand: 0,
                assigned: 0,
            })
            .collect();
        let next_payday_tick = catalog.pay_period_ticks;
        Self {
            catalog,
            pools,
            next_payday_tick,
            last_bill: 0,
            walked_out_last_payday: 0,
        }
    }

    pub fn bundled() -> Self {
        Self::new(StaffCatalog::bundled())
    }

    pub fn catalog(&self) -> &StaffCatalog {
        &self.catalog
    }

    /// The pools in catalog order, parallel to
    /// [`StaffCatalog::categories`].
    pub fn pools(&self) -> &[CategoryPool] {
        &self.pools
    }

    pub fn pool(&self, category: &str) -> Option<&CategoryPool> {
        self.catalog.index_of(category).map(|i| &self.pools[i])
    }

    pub fn headcount(&self, category: &str) -> u32 {
        self.pool(category).map_or(0, |p| p.headcount)
    }

    pub fn next_payday_tick(&self) -> u64 {
        self.next_payday_tick
    }

    pub fn last_bill(&self) -> u32 {
        self.last_bill
    }

    pub fn walked_out_last_payday(&self) -> u32 {
        self.walked_out_last_payday
    }

    /// What one payday would cost at the current headcounts:
    /// `sum(headcount * wage)`.
    pub fn bill_per_period(&self) -> u32 {
        self.catalog
            .categories
            .iter()
            .zip(&self.pools)
            .map(|(c, p)| c.wage.saturating_mul(p.headcount))
            .fold(0u32, u32::saturating_add)
    }

    /// Step 2 of `World::tick`: everyone is back at their post, nobody has
    /// asked for anyone yet.
    pub fn reset(&mut self) {
        for pool in &mut self.pools {
            pool.available = pool.headcount;
            pool.demand = 0;
            pool.assigned = 0;
        }
    }

    /// Asks for a whole crew. `demand` counts every head asked for whether
    /// or not the ask succeeds; if every category can supply, `available`
    /// drops and `assigned` rises for each, and `Ok(())`. If any cannot,
    /// **nothing is taken from any category** and the first short category
    /// *in the crew's own order* (the kind's `machines.ron` order) is
    /// returned, for the stall reason. A category the catalog does not know
    /// can never supply and refuses the same way. An empty crew is `Ok`.
    pub fn claim(&mut self, crew: &[(String, u32)]) -> Result<(), String> {
        let mut short: Option<String> = None;
        for (category, count) in crew {
            match self.catalog.index_of(category) {
                Some(i) => {
                    self.pools[i].demand = self.pools[i].demand.saturating_add(*count);
                    if self.pools[i].available < *count && short.is_none() {
                        short = Some(category.clone());
                    }
                }
                None => {
                    if short.is_none() {
                        short = Some(category.clone());
                    }
                }
            }
        }
        if let Some(category) = short {
            return Err(category);
        }
        for (category, count) in crew {
            let i = self
                .catalog
                .index_of(category)
                .expect("every category was found in the pass above");
            self.pools[i].available -= *count;
            self.pools[i].assigned += *count;
        }
        Ok(())
    }

    /// `PlayerCommand::Hire`: one more head, free, never refused for a
    /// known category. `false` for an unknown one.
    pub fn hire(&mut self, category: &str) -> bool {
        match self.catalog.index_of(category) {
            Some(i) => {
                self.pools[i].headcount = self.pools[i].headcount.saturating_add(1);
                true
            }
            None => false,
        }
    }

    /// `PlayerCommand::Fire`: one head fewer, free. `false` (nothing
    /// changed) at headcount 0 or for an unknown category. The head is gone
    /// at once; a machine mid-craft that loses its crew pauses on the next
    /// tick and resumes when one is free again.
    pub fn fire(&mut self, category: &str) -> bool {
        match self.catalog.index_of(category) {
            Some(i) if self.pools[i].headcount > 0 => {
                self.pools[i].headcount -= 1;
                true
            }
            _ => false,
        }
    }

    /// Payday, run after the economy's own tick. When `current_tick` has
    /// reached `next_payday_tick`, pays heads one at a time in catalog
    /// order through [`Economy::try_spend`]; the first refusal fires that
    /// head and every wage-earning head after it across every remaining
    /// category. Records `last_bill` (what was actually paid) and
    /// `walked_out_last_payday`, then moves the clock one period on.
    /// Returns the report on a payday tick, `None` otherwise.
    pub fn tick(&mut self, current_tick: u64, economy: &mut Economy) -> Option<PaydayReport> {
        if current_tick < self.next_payday_tick {
            return None;
        }
        let mut paid = 0u32;
        let mut walked_out = 0u32;
        let mut refused = false;
        for (category, pool) in self.catalog.categories.iter().zip(self.pools.iter_mut()) {
            if category.wage == 0 {
                // A volunteer is never unpaid, so never leaves.
                continue;
            }
            let wage = Fx::from_num(category.wage);
            let mut kept = 0u32;
            for _ in 0..pool.headcount {
                if !refused && economy.try_spend(wage) {
                    paid = paid.saturating_add(category.wage);
                    kept += 1;
                } else {
                    refused = true;
                    walked_out += 1;
                }
            }
            pool.headcount = kept;
        }
        self.last_bill = paid;
        self.walked_out_last_payday = walked_out;
        self.next_payday_tick += self.catalog.pay_period_ticks;
        Some(PaydayReport { paid, walked_out })
    }

    /// The wire's view — see [`StaffSnapshot`].
    pub fn snapshot(&self, money: Fx) -> StaffSnapshot {
        StaffSnapshot {
            pay_period_ticks: self.catalog.pay_period_ticks,
            next_payday_tick: self.next_payday_tick,
            last_bill: self.last_bill,
            walked_out_last_payday: self.walked_out_last_payday,
            money: money.to_num::<f64>(),
            categories: self
                .catalog
                .categories
                .iter()
                .zip(&self.pools)
                .map(|(c, p)| StaffCategorySnapshot {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    headcount: p.headcount,
                    wage: c.wage,
                    demand: p.demand,
                    assigned: p.assigned,
                    short: p.demand.saturating_sub(p.assigned),
                })
                .collect(),
        }
    }

    /// The save seam (`save.rs`): the persisted half of the pools. A
    /// category the map does not name keeps its starting headcount, so a
    /// category added to `staff.ron` after a save was written starts as a
    /// new game would. An id the catalog does not know is the caller's
    /// error to report. The per-tick tallies are not restored: the first
    /// tick after a load recomputes them, like the topology.
    pub(crate) fn restore(
        catalog: StaffCatalog,
        headcount: &[(String, u32)],
        next_payday_tick: u64,
        last_bill: u32,
        walked_out_last_payday: u32,
    ) -> Result<Self, String> {
        let mut pools = Self::new(catalog);
        for (id, count) in headcount {
            let Some(i) = pools.catalog.index_of(id) else {
                return Err(id.clone());
            };
            pools.pools[i].headcount = *count;
            pools.pools[i].available = *count;
        }
        pools.next_payday_tick = next_payday_tick;
        pools.last_bill = last_bill;
        pools.walked_out_last_payday = walked_out_last_payday;
        Ok(pools)
    }

    /// The save seam's other half: every headcount, in catalog order.
    pub(crate) fn headcounts(&self) -> Vec<(String, u32)> {
        self.catalog
            .categories
            .iter()
            .zip(&self.pools)
            .map(|(c, p)| (c.id.clone(), p.headcount))
            .collect()
    }
}

/// `SimBridge::staff_snapshot_json`'s object — polled, since `demand`,
/// `assigned` and `short` change every tick and dirty nothing:
///
/// ```json
/// {"pay_period_ticks": 1000, "next_payday_tick": 3000, "last_bill": 14,
///  "walked_out_last_payday": 0, "money": 2986.0,
///  "categories": [
///    {"id": "manufacturing", "name": "Manufacturing", "headcount": 4,
///     "wage": 2, "demand": 4, "assigned": 4, "short": 0}, ...]}
/// ```
///
/// `demand`, `assigned` and `short` (= `demand - assigned`) describe the
/// LAST completed tick. `money` is repeated here, as `shop_snapshot_json`
/// does, so the staff panel needs one poll.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StaffSnapshot {
    pub pay_period_ticks: u64,
    pub next_payday_tick: u64,
    pub last_bill: u32,
    pub walked_out_last_payday: u32,
    pub money: f64,
    pub categories: Vec<StaffCategorySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StaffCategorySnapshot {
    pub id: String,
    pub name: String,
    pub headcount: u32,
    pub wage: u32,
    pub demand: u32,
    pub assigned: u32,
    pub short: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        pay_period_ticks: 100,
        categories: [
            StaffCategory(id: "cooks", name: "Cooks", wage: 3, starting_headcount: 2),
            StaffCategory(id: "volunteers", name: "Volunteers", wage: 0, starting_headcount: 1),
            StaffCategory(id: "drivers", name: "Drivers", wage: 5, starting_headcount: 1),
        ],
    )"#;

    fn catalog() -> StaffCatalog {
        StaffCatalog::from_ron_str(EXAMPLE).unwrap()
    }

    fn crew(lines: &[(&str, u32)]) -> Vec<(String, u32)> {
        lines.iter().map(|(c, n)| (c.to_string(), *n)).collect()
    }

    fn economy_with(money: u32) -> Economy {
        Economy::with_money(
            &crate::sim::contracts::ContractRegistry::bundled(),
            Fx::from_num(money),
        )
    }

    #[test]
    fn the_bundled_file_loads() {
        let catalog = StaffCatalog::bundled();
        assert_eq!(catalog.categories().len(), 4);
        assert!(catalog.pay_period_ticks() > 0);
    }

    #[test]
    fn loads_categories_in_file_order() {
        let c = catalog();
        assert_eq!(c.category_ids(), vec!["cooks", "volunteers", "drivers"]);
        assert_eq!(c.pay_period_ticks(), 100);
        assert_eq!(c.get("drivers").unwrap().wage, 5);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 2", 1);
        assert!(matches!(
            StaffCatalog::from_ron_str(&text),
            Err(StaffLoadError::UnsupportedSchemaVersion {
                found: 2,
                supported: 1
            })
        ));
    }

    #[test]
    fn rejects_no_categories() {
        let text = "(schema_version: 1, pay_period_ticks: 100, categories: [])";
        assert!(matches!(
            StaffCatalog::from_ron_str(text),
            Err(StaffLoadError::NoCategories)
        ));
    }

    #[test]
    fn rejects_a_duplicate_id() {
        let text = EXAMPLE.replacen(r#"id: "drivers""#, r#"id: "cooks""#, 1);
        match StaffCatalog::from_ron_str(&text) {
            Err(StaffLoadError::DuplicateCategory { id }) => assert_eq!(id, "cooks"),
            other => panic!("expected DuplicateCategory, got {other:?}"),
        }
    }

    #[test]
    fn rejects_an_empty_name() {
        let text = EXAMPLE.replacen(r#"name: "Cooks""#, r#"name: "  ""#, 1);
        match StaffCatalog::from_ron_str(&text) {
            Err(StaffLoadError::EmptyName { id }) => assert_eq!(id, "cooks"),
            other => panic!("expected EmptyName, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_zero_pay_period() {
        let text = EXAMPLE.replacen("pay_period_ticks: 100", "pay_period_ticks: 0", 1);
        assert!(matches!(
            StaffCatalog::from_ron_str(&text),
            Err(StaffLoadError::ZeroPayPeriod)
        ));
    }

    #[test]
    fn a_zero_wage_is_allowed() {
        assert_eq!(catalog().get("volunteers").unwrap().wage, 0);
    }

    #[test]
    fn a_new_pool_holds_the_starting_crew_and_a_payday_one_period_out() {
        let pools = StaffPools::new(catalog());
        assert_eq!(pools.headcount("cooks"), 2);
        assert_eq!(pools.headcount("volunteers"), 1);
        assert_eq!(pools.headcount("drivers"), 1);
        assert_eq!(pools.headcount("nobody"), 0);
        assert_eq!(pools.next_payday_tick(), 100);
        assert_eq!(pools.bill_per_period(), 2 * 3 + 0 + 5);
    }

    #[test]
    fn a_claim_takes_the_whole_crew_or_nothing() {
        let mut pools = StaffPools::new(catalog());
        pools.reset();
        // Two cooks and a driver available; this crew wants both cooks and
        // two drivers: refused on drivers, and NOTHING is taken from cooks.
        let refused = pools.claim(&crew(&[("cooks", 2), ("drivers", 2)]));
        assert_eq!(refused, Err("drivers".to_string()));
        assert_eq!(pools.pool("cooks").unwrap().available, 2);
        assert_eq!(pools.pool("cooks").unwrap().assigned, 0);
        // ...but the ask is counted as demand on both.
        assert_eq!(pools.pool("cooks").unwrap().demand, 2);
        assert_eq!(pools.pool("drivers").unwrap().demand, 2);

        assert_eq!(pools.claim(&crew(&[("cooks", 1), ("drivers", 1)])), Ok(()));
        let cooks = pools.pool("cooks").unwrap();
        assert_eq!((cooks.available, cooks.assigned, cooks.demand), (1, 1, 3));
        let drivers = pools.pool("drivers").unwrap();
        assert_eq!(
            (drivers.available, drivers.assigned, drivers.demand),
            (0, 1, 3)
        );

        // The next driver ask is refused; the first short category in the
        // CREW's order is what is named.
        assert_eq!(
            pools.claim(&crew(&[("drivers", 1), ("cooks", 5)])),
            Err("drivers".to_string())
        );
        assert_eq!(
            pools.claim(&crew(&[("cooks", 5), ("drivers", 1)])),
            Err("cooks".to_string())
        );
    }

    #[test]
    fn an_empty_crew_and_a_zero_count_always_succeed_and_an_unknown_category_never_does() {
        let mut pools = StaffPools::new(catalog());
        pools.reset();
        assert_eq!(pools.claim(&[]), Ok(()));
        assert_eq!(pools.claim(&crew(&[("cooks", 0)])), Ok(()));
        assert_eq!(pools.pool("cooks").unwrap().demand, 0);
        assert_eq!(
            pools.claim(&crew(&[("cooks", 1), ("plumbers", 1)])),
            Err("plumbers".to_string())
        );
        assert_eq!(
            pools.pool("cooks").unwrap().assigned,
            0,
            "a refusal on the unknown category takes nothing from the known one"
        );
    }

    #[test]
    fn reset_restores_everyone_and_forgets_the_tally() {
        let mut pools = StaffPools::new(catalog());
        pools.reset();
        pools.claim(&crew(&[("cooks", 2)])).unwrap();
        pools.claim(&crew(&[("cooks", 1)])).unwrap_err();
        pools.reset();
        let cooks = pools.pool("cooks").unwrap();
        assert_eq!((cooks.available, cooks.demand, cooks.assigned), (2, 0, 0));
    }

    #[test]
    fn hire_and_fire_move_one_head_and_refuse_the_impossible() {
        let mut pools = StaffPools::new(catalog());
        assert!(pools.hire("cooks"));
        assert_eq!(pools.headcount("cooks"), 3);
        assert!(!pools.hire("plumbers"));
        assert!(pools.fire("drivers"));
        assert_eq!(pools.headcount("drivers"), 0);
        assert!(!pools.fire("drivers"), "fire at 0 is a no-op");
        assert!(!pools.fire("plumbers"));
        assert_eq!(pools.headcount("drivers"), 0);
    }

    #[test]
    fn payday_fires_only_on_the_boundary_and_debits_the_whole_bill() {
        let mut pools = StaffPools::new(catalog());
        let mut economy = economy_with(100);
        assert_eq!(pools.tick(99, &mut economy), None);
        assert_eq!(economy.money, Fx::from_num(100));
        let report = pools.tick(100, &mut economy).expect("payday at 100");
        assert_eq!(
            report,
            PaydayReport {
                paid: 11,
                walked_out: 0
            }
        );
        assert_eq!(economy.money, Fx::from_num(89));
        assert_eq!(pools.last_bill(), 11);
        assert_eq!(pools.walked_out_last_payday(), 0);
        assert_eq!(pools.next_payday_tick(), 200);
        assert_eq!(pools.tick(101, &mut economy), None);
        assert!(pools.tick(200, &mut economy).is_some());
        assert_eq!(economy.money, Fx::from_num(78));
    }

    #[test]
    fn the_first_unpaid_head_and_every_wage_earner_after_it_leave() {
        let mut pools = StaffPools::new(catalog());
        // Enough for one cook (3) and part of the second: the second cook
        // cannot be paid, and neither can the driver after him even though
        // 1 is left — the walk-out is a line, not a per-head affordability
        // test. The volunteer stays.
        let mut economy = economy_with(4);
        let report = pools.tick(100, &mut economy).unwrap();
        assert_eq!(
            report,
            PaydayReport {
                paid: 3,
                walked_out: 2
            }
        );
        assert_eq!(pools.headcount("cooks"), 1);
        assert_eq!(pools.headcount("volunteers"), 1);
        assert_eq!(pools.headcount("drivers"), 0);
        assert_eq!(economy.money, Fx::from_num(1));
        assert_eq!(pools.walked_out_last_payday(), 2);
        assert_eq!(pools.last_bill(), 3);
    }

    #[test]
    fn a_zero_wage_head_stays_even_when_everyone_else_goes() {
        let mut pools = StaffPools::new(catalog());
        let mut economy = economy_with(0);
        let report = pools.tick(100, &mut economy).unwrap();
        assert_eq!(report.paid, 0);
        assert_eq!(report.walked_out, 3);
        assert_eq!(pools.headcount("volunteers"), 1);
        assert_eq!(pools.bill_per_period(), 0);
    }

    #[test]
    fn the_snapshot_reports_the_last_tick_and_repeats_money() {
        let mut pools = StaffPools::new(catalog());
        pools.reset();
        pools.claim(&crew(&[("cooks", 2), ("drivers", 1)])).unwrap();
        pools.claim(&crew(&[("cooks", 1)])).unwrap_err();
        let snap = pools.snapshot(Fx::from_num(42));
        assert_eq!(snap.money, 42.0);
        assert_eq!(snap.pay_period_ticks, 100);
        assert_eq!(snap.next_payday_tick, 100);
        let cooks = &snap.categories[0];
        assert_eq!(cooks.id, "cooks");
        assert_eq!(cooks.name, "Cooks");
        assert_eq!(
            (
                cooks.headcount,
                cooks.wage,
                cooks.demand,
                cooks.assigned,
                cooks.short
            ),
            (2, 3, 3, 2, 1)
        );
        let json = serde_json::to_value(&snap).unwrap();
        let keys: Vec<&str> = json["categories"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        for key in [
            "id",
            "name",
            "headcount",
            "wage",
            "demand",
            "assigned",
            "short",
        ] {
            assert!(keys.contains(&key), "{key} missing from {keys:?}");
        }
    }

    #[test]
    fn restore_keeps_the_starting_count_for_a_category_the_save_omits() {
        let restored =
            StaffPools::restore(catalog(), &[("cooks".to_string(), 7)], 450, 9, 2).unwrap();
        assert_eq!(restored.headcount("cooks"), 7);
        assert_eq!(restored.headcount("drivers"), 1);
        assert_eq!(restored.next_payday_tick(), 450);
        assert_eq!(restored.last_bill(), 9);
        assert_eq!(restored.walked_out_last_payday(), 2);
        assert_eq!(
            StaffPools::restore(catalog(), &[("plumbers".to_string(), 1)], 0, 0, 0),
            Err("plumbers".to_string())
        );
    }
}
