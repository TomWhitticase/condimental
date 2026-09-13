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
//! **Wages spend money; hiring and firing do not, but neither dodges a
//! wage** (the quality pass, 2026-09-13). On payday ([`StaffPools::tick`])
//! the bill is paid one head at a time in `staff.ron` order through
//! `Economy::try_spend` — still the only debit, still all-or-nothing per
//! call. **A head is owed for any period it was employed in any part of**
//! ([`CategoryPool::owed`]): a head fired mid-period is still billed at
//! the next payday (notice pay, and it is paid *first*, before the heads
//! still on the books), and a head hired mid-period is billed at that
//! payday — so firing everyone the tick before payday and hiring them
//! back after costs exactly what keeping them would have. Hiring is
//! refused while the balance cannot cover the next bill *plus* the new
//! head's wage ([`StaffPools::can_hire`]): you cannot hire someone you
//! could not pay.
//!
//! **A walk-out is never silent and never sudden.** The first head the
//! money does not reach — and every head after it down the file, the line
//! rule — is not fired: it is **unpaid** ([`CategoryPool::unpaid`]), still
//! hired, but it does not work the period it was not paid for (the pools
//! [`reset`](StaffPools::reset) to `headcount - unpaid`), so the factory
//! that spent its wages sees machines starve with a `staff` stall and a
//! panel that says how many are unpaid, not a crew that vanished. A head
//! unpaid at the NEXT payday as well leaves ([`CategoryPool::
//! left_last_payday`]); a paid payday clears the arrears. Two strikes,
//! mirroring the contract board. Before any of that, the wire says what
//! the next payday will cost ([`StaffSnapshot::next_bill`]) and how far
//! short the balance is ([`StaffSnapshot::shortfall`]), so the warning can
//! be shown before the payday, not after. Stricter than debt, which
//! GAMEPLAY.md §1's one non-negative counter does not allow; softer than
//! a cliff. A zero-wage head is never unpaid and never leaves.
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

/// One category's live numbers. `headcount`, `unpaid`, `owed` and
/// `left_last_payday` persist (and ride the save); `available`, `demand`
/// and `assigned` are this tick's tally, reset at the top of every tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CategoryPool {
    pub headcount: u32,
    /// Heads in arrears: hired, unpaid at the last payday, and **not
    /// working** until they are paid. Never more than `headcount`. Paid at
    /// the next payday they go back to work; unpaid again, they leave.
    pub unpaid: u32,
    /// Heads the NEXT payday bills: everyone employed for any part of the
    /// current period. Starts each period at `headcount`, rises with every
    /// hire, and does **not** fall with a fire (notice pay), so it is never
    /// less than `headcount`.
    pub owed: u32,
    /// Heads who left at the last payday — unpaid there and at the one
    /// before. Zero until a payday, and zero again after the next one.
    pub left_last_payday: u32,
    /// Heads not yet claimed this tick (`headcount - unpaid` at reset).
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
    /// How many hired heads the money did not reach (they stay, in
    /// arrears, and do not work next period — including the ones counted
    /// in `left`, who were already in arrears).
    pub unpaid: u32,
    /// How many heads left: unpaid at this payday and the one before.
    pub left: u32,
}

/// Why [`StaffPools::restore`] refused a save's pools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaffRestoreError {
    /// An id `data/staff.ron` does not define.
    UnknownCategory(String),
    /// A state the tick path could never have produced.
    Inconsistent { category: String, reason: String },
}

/// The persisted half of the pools, as [`StaffPools::restore`] takes it —
/// the save's `SavedStaff` without `serde`. Every per-category list is
/// keyed by id; a category a list does not name takes its default (its
/// starting headcount; no arrears; owed = its headcount; nobody left).
pub struct StaffRestore<'a> {
    pub headcount: &'a [(String, u32)],
    pub unpaid: &'a [(String, u32)],
    pub owed: &'a [(String, u32)],
    pub left_last_payday: &'a [(String, u32)],
    pub next_payday_tick: u64,
    pub last_bill: u32,
    pub unpaid_last_payday: u32,
    pub walked_out_last_payday: u32,
}

/// The live pools: one [`CategoryPool`] per catalog category, in file
/// order, plus the payday clock. Owned by `World` as a plain field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffPools {
    catalog: StaffCatalog,
    pools: Vec<CategoryPool>,
    next_payday_tick: u64,
    last_bill: u32,
    /// Heads the last payday's money did not reach (sum of that payday's
    /// per-category `unpaid`, before anyone left).
    unpaid_last_payday: u32,
    /// Heads who LEFT at the last payday (unpaid twice running). The name
    /// is the MVP's and means what it meant — how many walked out — only
    /// the rule for walking out changed.
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
                unpaid: 0,
                owed: c.starting_headcount,
                left_last_payday: 0,
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
            unpaid_last_payday: 0,
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

    /// How many heads LEFT at the last payday — unpaid there and at the
    /// payday before it.
    pub fn walked_out_last_payday(&self) -> u32 {
        self.walked_out_last_payday
    }

    /// How many hired heads the last payday's money did not reach.
    pub fn unpaid_last_payday(&self) -> u32 {
        self.unpaid_last_payday
    }

    /// Heads currently in arrears across every category.
    pub fn unpaid(&self) -> u32 {
        self.pools
            .iter()
            .map(|p| p.unpaid)
            .fold(0u32, u32::saturating_add)
    }

    /// The run rate: what one payday costs at the current headcounts,
    /// `sum(headcount * wage)`. What the NEXT payday will actually charge
    /// is [`next_bill`](Self::next_bill), which also counts notice pay.
    pub fn bill_per_period(&self) -> u32 {
        self.catalog
            .categories
            .iter()
            .zip(&self.pools)
            .map(|(c, p)| c.wage.saturating_mul(p.headcount))
            .fold(0u32, u32::saturating_add)
    }

    /// What the next payday will charge: `sum(owed * wage)` — every head
    /// employed for any part of this period, fired or not.
    pub fn next_bill(&self) -> u32 {
        self.catalog
            .categories
            .iter()
            .zip(&self.pools)
            .map(|(c, p)| c.wage.saturating_mul(p.owed))
            .fold(0u32, u32::saturating_add)
    }

    /// How far `money` falls short of [`next_bill`](Self::next_bill), in
    /// whole units rounded up; zero when the bill is covered. The number
    /// the panel warns with BEFORE a payday.
    pub fn shortfall(&self, money: Fx) -> u32 {
        let bill = Fx::saturating_from_num(self.next_bill());
        if money >= bill {
            0
        } else {
            (bill - money).ceil().saturating_to_num::<u32>()
        }
    }

    /// Whether `hire(category)` would be accepted with `money` in hand:
    /// the category is known and the balance covers the next bill plus
    /// the new head's wage. You cannot hire someone you could not pay.
    pub fn can_hire(&self, category: &str, money: Fx) -> bool {
        let Some(c) = self.catalog.get(category) else {
            return false;
        };
        let needed = Fx::saturating_from_num(self.next_bill().saturating_add(c.wage));
        money >= needed
    }

    /// Step 2 of `World::tick`: everyone who is being paid is back at
    /// their post, nobody has asked for anyone yet. A head in arrears is
    /// not at their post: `available` is `headcount - unpaid`.
    pub fn reset(&mut self) {
        for pool in &mut self.pools {
            pool.available = pool.headcount.saturating_sub(pool.unpaid);
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

    /// `PlayerCommand::Hire`: one more head, free to hire, billed at the
    /// next payday whenever in the period it arrives (`owed` rises with
    /// `headcount`). Refused — `false`, nothing changed — for an unknown
    /// category or when [`can_hire`](Self::can_hire) says the balance
    /// could not pay it. A new head is at work from the next tick.
    pub fn hire(&mut self, category: &str, money: Fx) -> bool {
        if !self.can_hire(category, money) {
            return false;
        }
        let i = self
            .catalog
            .index_of(category)
            .expect("can_hire is false for an unknown category");
        self.pools[i].headcount = self.pools[i].headcount.saturating_add(1);
        self.pools[i].owed = self.pools[i].owed.saturating_add(1);
        true
    }

    /// `PlayerCommand::Fire`: one head fewer, at once. `false` (nothing
    /// changed) at headcount 0 or for an unknown category. A head in
    /// arrears goes before a working one (it was not working anyway).
    /// **The fired head is still owed** for this period — `owed` does not
    /// move, so the next payday bills it (notice pay). A machine mid-craft
    /// that loses its crew pauses on the next tick and resumes when one is
    /// free again.
    pub fn fire(&mut self, category: &str) -> bool {
        match self.catalog.index_of(category) {
            Some(i) if self.pools[i].headcount > 0 => {
                let pool = &mut self.pools[i];
                pool.headcount -= 1;
                if pool.unpaid > 0 {
                    pool.unpaid -= 1;
                }
                true
            }
            _ => false,
        }
    }

    /// Payday, run after the economy's own tick. When `current_tick` has
    /// reached `next_payday_tick`, pays every OWED head one at a time in
    /// catalog order through [`Economy::try_spend`] — within a category the
    /// heads that have already left (notice pay) first, then the heads in
    /// arrears, then the rest. The first refusal leaves that head and every
    /// wage-earning head after it, across every remaining category,
    /// **unpaid** (the line rule: the money stops where it stops). Then,
    /// per category: a head unpaid now that was already in arrears
    /// **leaves**; a head unpaid for the first time stays, in arrears, and
    /// will not work next period; a head that was paid is clear. `owed`
    /// restarts at the new headcount. Records `last_bill` (what was
    /// actually paid), `unpaid_last_payday` and `walked_out_last_payday`,
    /// moves the clock one period on, and returns the report; `None` off
    /// the boundary. A zero-wage category is paid in full by definition.
    pub fn tick(&mut self, current_tick: u64, economy: &mut Economy) -> Option<PaydayReport> {
        if current_tick < self.next_payday_tick {
            return None;
        }
        let mut paid = 0u32;
        let mut unpaid_total = 0u32;
        let mut left_total = 0u32;
        let mut refused = false;
        for (category, pool) in self.catalog.categories.iter().zip(self.pools.iter_mut()) {
            pool.left_last_payday = 0;
            if category.wage == 0 {
                // A volunteer is never unpaid, so never leaves.
                pool.unpaid = 0;
                pool.owed = pool.headcount;
                continue;
            }
            let wage = Fx::from_num(category.wage);
            let mut paid_heads = 0u32;
            for _ in 0..pool.owed {
                if !refused && economy.try_spend(wage) {
                    paid = paid.saturating_add(category.wage);
                    paid_heads += 1;
                } else {
                    refused = true;
                }
            }
            // Notice pay went out first; what reached the heads still on
            // the books is the rest.
            let departed = pool.owed.saturating_sub(pool.headcount);
            let current_paid = paid_heads.saturating_sub(departed);
            let unpaid_now = pool.headcount.saturating_sub(current_paid);
            // Back pay first: the money that reached the current heads went
            // to the ones in arrears before anyone else, so an arrears head
            // it did NOT reach was unpaid twice and leaves; a clear head it
            // did not reach is in arrears for the first time and stays.
            let left = pool.unpaid.saturating_sub(current_paid);
            pool.headcount -= left;
            pool.unpaid = unpaid_now - left;
            pool.left_last_payday = left;
            pool.owed = pool.headcount;
            unpaid_total = unpaid_total.saturating_add(unpaid_now);
            left_total = left_total.saturating_add(left);
        }
        self.last_bill = paid;
        self.unpaid_last_payday = unpaid_total;
        self.walked_out_last_payday = left_total;
        self.next_payday_tick += self.catalog.pay_period_ticks;
        Some(PaydayReport {
            paid,
            unpaid: unpaid_total,
            left: left_total,
        })
    }

    /// The wire's view — see [`StaffSnapshot`].
    pub fn snapshot(&self, money: Fx) -> StaffSnapshot {
        StaffSnapshot {
            pay_period_ticks: self.catalog.pay_period_ticks,
            next_payday_tick: self.next_payday_tick,
            next_bill: self.next_bill(),
            shortfall: self.shortfall(money),
            last_bill: self.last_bill,
            unpaid_last_payday: self.unpaid_last_payday,
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
                    owed: p.owed,
                    unpaid: p.unpaid,
                    left_last_payday: p.left_last_payday,
                    can_hire: self.can_hire(&c.id, money),
                    demand: p.demand,
                    assigned: p.assigned,
                    short: p.demand.saturating_sub(p.assigned),
                })
                .collect(),
        }
    }

    /// The save seam (`save.rs`): the persisted half of the pools. A
    /// category a list does not name takes its default (see
    /// [`StaffRestore`]), so a category added to `staff.ron` after a save
    /// was written starts as a new game would, and a save from before the
    /// quality pass — no arrears, no `owed` — loads with nobody in arrears
    /// and every head owed. An id the catalog does not know, arrears above
    /// the headcount, or `owed` below it (states the tick path cannot
    /// produce) are refused. The per-tick tallies are not restored: the
    /// first tick after a load recomputes them, like the topology.
    pub(crate) fn restore(
        catalog: StaffCatalog,
        saved: StaffRestore<'_>,
    ) -> Result<Self, StaffRestoreError> {
        let mut pools = Self::new(catalog);
        let index_of = |pools: &Self, id: &String| {
            pools
                .catalog
                .index_of(id)
                .ok_or_else(|| StaffRestoreError::UnknownCategory(id.clone()))
        };
        for (id, count) in saved.headcount {
            let i = index_of(&pools, id)?;
            pools.pools[i].headcount = *count;
            pools.pools[i].owed = *count;
        }
        for (id, count) in saved.unpaid {
            let i = index_of(&pools, id)?;
            pools.pools[i].unpaid = *count;
        }
        for (id, count) in saved.owed {
            let i = index_of(&pools, id)?;
            pools.pools[i].owed = *count;
        }
        for (id, count) in saved.left_last_payday {
            let i = index_of(&pools, id)?;
            pools.pools[i].left_last_payday = *count;
        }
        for (c, p) in pools.catalog.categories.iter().zip(&pools.pools) {
            if p.unpaid > p.headcount {
                return Err(StaffRestoreError::Inconsistent {
                    category: c.id.clone(),
                    reason: format!("{} unpaid of {} hired", p.unpaid, p.headcount),
                });
            }
            if p.owed < p.headcount {
                return Err(StaffRestoreError::Inconsistent {
                    category: c.id.clone(),
                    reason: format!("{} owed, fewer than the {} hired", p.owed, p.headcount),
                });
            }
        }
        for p in &mut pools.pools {
            p.available = p.headcount.saturating_sub(p.unpaid);
        }
        pools.next_payday_tick = saved.next_payday_tick;
        pools.last_bill = saved.last_bill;
        pools.unpaid_last_payday = saved.unpaid_last_payday;
        pools.walked_out_last_payday = saved.walked_out_last_payday;
        Ok(pools)
    }

    /// The save seam's other half: every headcount, in catalog order.
    pub(crate) fn headcounts(&self) -> Vec<(String, u32)> {
        self.per_category(|p| p.headcount)
    }

    /// Every category's arrears, in catalog order.
    pub(crate) fn unpaid_counts(&self) -> Vec<(String, u32)> {
        self.per_category(|p| p.unpaid)
    }

    /// Every category's owed count, in catalog order.
    pub(crate) fn owed_counts(&self) -> Vec<(String, u32)> {
        self.per_category(|p| p.owed)
    }

    /// Every category's leavers at the last payday, in catalog order.
    pub(crate) fn left_counts(&self) -> Vec<(String, u32)> {
        self.per_category(|p| p.left_last_payday)
    }

    fn per_category(&self, f: impl Fn(&CategoryPool) -> u32) -> Vec<(String, u32)> {
        self.catalog
            .categories
            .iter()
            .zip(&self.pools)
            .map(|(c, p)| (c.id.clone(), f(p)))
            .collect()
    }
}

/// `SimBridge::staff_snapshot_json`'s object — polled, since `demand`,
/// `assigned` and `short` change every tick and dirty nothing:
///
/// ```json
/// {"pay_period_ticks": 1000, "next_payday_tick": 3000,
///  "next_bill": 14, "shortfall": 0,
///  "last_bill": 14, "unpaid_last_payday": 0, "walked_out_last_payday": 0,
///  "money": 2986.0,
///  "categories": [
///    {"id": "manufacturing", "name": "Manufacturing", "headcount": 4,
///     "wage": 2, "owed": 4, "unpaid": 0, "left_last_payday": 0,
///     "can_hire": true, "demand": 4, "assigned": 4, "short": 0}, ...]}
/// ```
///
/// `demand`, `assigned` and `short` (= `demand - assigned`) describe the
/// LAST completed tick. `money` is repeated here, as `shop_snapshot_json`
/// does, so the staff panel needs one poll.
///
/// The quality pass's additions (2026-09-13), so a panel can warn BEFORE
/// a payday and announce AFTER it: `next_bill` is what the next payday
/// will charge (every head employed for any part of the period, fired or
/// not: `Σ owed × wage`); `shortfall` is how far `money` falls short of
/// it, 0 when covered; `unpaid_last_payday` is how many hired heads the
/// last payday's money did not reach; `walked_out_last_payday` how many
/// of them LEFT (unpaid there and at the payday before). Per category:
/// `owed` (heads the next payday bills), `unpaid` (heads in arrears —
/// hired, not working, gone if unpaid again), `left_last_payday` (who
/// left, by category), and `can_hire` (whether a `hire` would be accepted
/// with this balance: it covers `next_bill` plus this category's wage).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StaffSnapshot {
    pub pay_period_ticks: u64,
    pub next_payday_tick: u64,
    pub next_bill: u32,
    pub shortfall: u32,
    pub last_bill: u32,
    pub unpaid_last_payday: u32,
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
    pub owed: u32,
    pub unpaid: u32,
    pub left_last_payday: u32,
    pub can_hire: bool,
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
        let rich = Fx::from_num(1000);
        assert!(pools.hire("cooks", rich));
        assert_eq!(pools.headcount("cooks"), 3);
        assert!(!pools.hire("plumbers", rich));
        assert!(pools.fire("drivers"));
        assert_eq!(pools.headcount("drivers"), 0);
        assert!(!pools.fire("drivers"), "fire at 0 is a no-op");
        assert!(!pools.fire("plumbers"));
        assert_eq!(pools.headcount("drivers"), 0);
    }

    /// You cannot hire someone you could not pay: a hire needs the balance
    /// to cover the next bill PLUS the new head's wage.
    #[test]
    fn a_hire_is_refused_when_the_balance_could_not_pay_the_next_bill_with_it() {
        let mut pools = StaffPools::new(catalog());
        // Bill 11; a cook is 3 more: 14 is the line.
        assert_eq!(pools.next_bill(), 11);
        assert!(!pools.can_hire("cooks", Fx::from_num(13)));
        assert!(!pools.hire("cooks", Fx::from_num(13)));
        assert_eq!(
            pools.headcount("cooks"),
            2,
            "a refused hire changes nothing"
        );
        assert!(pools.can_hire("cooks", Fx::from_num(14)));
        assert!(pools.hire("cooks", Fx::from_num(14)));
        assert_eq!(pools.next_bill(), 14);
        // A volunteer costs nothing extra, but the bill itself must still
        // be covered.
        assert!(!pools.can_hire("volunteers", Fx::from_num(13)));
        assert!(pools.can_hire("volunteers", Fx::from_num(14)));
        assert!(!pools.can_hire("plumbers", Fx::from_num(1_000)));
        assert_eq!(pools.shortfall(Fx::from_num(14)), 0);
        assert_eq!(pools.shortfall(Fx::from_num(13)), 1);
        assert_eq!(
            pools.shortfall(Fx::from_num(12.5)),
            2,
            "a part unit short is a whole unit short"
        );
        assert_eq!(pools.shortfall(Fx::ZERO), 14);
    }

    /// Notice pay: a head is owed for any period it was employed in any
    /// part of, so firing everyone the tick before payday bills exactly
    /// what keeping them would have, and a head hired one tick before
    /// payday is billed at that payday.
    #[test]
    fn a_fired_head_is_still_owed_and_a_hired_one_is_owed_at_once() {
        let mut pools = StaffPools::new(catalog());
        let mut economy = economy_with(100);
        assert_eq!(pools.next_bill(), 11);
        assert!(pools.fire("cooks") && pools.fire("cooks") && pools.fire("drivers"));
        assert_eq!(pools.bill_per_period(), 0, "nobody is on the books...");
        assert_eq!(pools.next_bill(), 11, "...but everybody is owed");
        assert_eq!(pools.pool("cooks").unwrap().owed, 2);
        let report = pools.tick(100, &mut economy).unwrap();
        assert_eq!(report.paid, 11, "the bill is unchanged by the firing");
        assert_eq!(economy.money, Fx::from_num(89));
        assert_eq!((report.unpaid, report.left), (0, 0));
        assert_eq!(
            pools.next_bill(),
            0,
            "the notice is paid; nothing is owed now"
        );

        // Hired one tick before the next payday: owed in full.
        assert!(pools.hire("drivers", economy.money));
        assert_eq!(pools.next_bill(), 5);
        let report = pools.tick(200, &mut economy).unwrap();
        assert_eq!(report.paid, 5);
        assert_eq!(economy.money, Fx::from_num(84));
    }

    /// Notice pay is paid FIRST, so a broke factory cannot dodge it by
    /// having fired: with money for one head only, the departed cook's
    /// notice takes it and the two still hired are the ones unpaid.
    #[test]
    fn notice_pay_comes_before_the_heads_still_on_the_books() {
        let mut pools = StaffPools::new(catalog());
        assert!(pools.fire("cooks"));
        // One cook hired, two owed (3 + 3), one driver owed (5): 11.
        let mut economy = economy_with(3);
        let report = pools.tick(100, &mut economy).unwrap();
        assert_eq!(report.paid, 3);
        assert_eq!(report.unpaid, 2, "the hired cook and the driver");
        assert_eq!(report.left, 0, "a first miss is arrears, not a walk-out");
        assert_eq!(pools.pool("cooks").unwrap().unpaid, 1);
        assert_eq!(pools.pool("drivers").unwrap().unpaid, 1);
        assert_eq!(pools.headcount("cooks"), 1);
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
                unpaid: 0,
                left: 0,
            }
        );
        assert_eq!(economy.money, Fx::from_num(89));
        assert_eq!(pools.last_bill(), 11);
        assert_eq!(pools.unpaid_last_payday(), 0);
        assert_eq!(pools.walked_out_last_payday(), 0);
        assert_eq!(pools.next_payday_tick(), 200);
        assert_eq!(pools.tick(101, &mut economy), None);
        assert!(pools.tick(200, &mut economy).is_some());
        assert_eq!(economy.money, Fx::from_num(78));
    }

    /// The first unpaid head and every wage-earner after it are UNPAID,
    /// not fired: still hired, not working, and gone only if the next
    /// payday cannot pay them either. A paid payday clears the arrears.
    #[test]
    fn the_first_unpaid_head_and_every_wage_earner_after_it_go_into_arrears_then_leave() {
        let mut pools = StaffPools::new(catalog());
        // Enough for one cook (3) and part of the second: the second cook
        // cannot be paid, and neither can the driver after him even though
        // 1 is left — the line stops where it stops, it is not a per-head
        // affordability test. The volunteer is never unpaid.
        let mut economy = economy_with(4);
        let report = pools.tick(100, &mut economy).unwrap();
        assert_eq!(
            report,
            PaydayReport {
                paid: 3,
                unpaid: 2,
                left: 0,
            }
        );
        assert_eq!(pools.headcount("cooks"), 2, "nobody left");
        assert_eq!(pools.headcount("drivers"), 1);
        assert_eq!(pools.pool("cooks").unwrap().unpaid, 1);
        assert_eq!(pools.pool("drivers").unwrap().unpaid, 1);
        assert_eq!(pools.pool("volunteers").unwrap().unpaid, 0);
        assert_eq!(pools.unpaid(), 2);
        assert_eq!(economy.money, Fx::from_num(1));
        assert_eq!(pools.unpaid_last_payday(), 2);
        assert_eq!(pools.walked_out_last_payday(), 0);
        assert_eq!(pools.last_bill(), 3);
        assert_eq!(
            pools.next_bill(),
            11,
            "arrears do not stack: one wage per head"
        );

        // The unpaid do not work: only one cook and no driver at their post.
        pools.reset();
        assert_eq!(pools.pool("cooks").unwrap().available, 1);
        assert_eq!(pools.pool("drivers").unwrap().available, 0);
        assert_eq!(
            pools.claim(&crew(&[("cooks", 2)])),
            Err("cooks".to_string())
        );
        assert_eq!(pools.claim(&crew(&[("cooks", 1)])), Ok(()));

        // Still broke at the next payday: the two in arrears leave, and
        // the cook the money did reach stays. Back pay is settled first,
        // so the 3 pays the cook who was in arrears — the OTHER cook is
        // now the unpaid one, and nobody has been unpaid twice.
        economy.money = Fx::from_num(3);
        let report = pools.tick(200, &mut economy).unwrap();
        assert_eq!(report.paid, 3);
        assert_eq!(report.unpaid, 2, "the other cook and the driver");
        assert_eq!(report.left, 1, "the driver, unpaid twice");
        assert_eq!(pools.headcount("drivers"), 0);
        assert_eq!(pools.pool("drivers").unwrap().left_last_payday, 1);
        assert_eq!(pools.headcount("cooks"), 2);
        assert_eq!(pools.pool("cooks").unwrap().unpaid, 1);
        assert_eq!(pools.pool("cooks").unwrap().left_last_payday, 0);
        assert_eq!(pools.walked_out_last_payday(), 1);

        // Nothing at all at the third: the cook in arrears leaves, the
        // other goes into arrears; `left_last_payday` is this payday's.
        let report = pools.tick(300, &mut economy).unwrap();
        assert_eq!((report.paid, report.unpaid, report.left), (0, 2, 1));
        assert_eq!(pools.headcount("cooks"), 1);
        assert_eq!(pools.pool("cooks").unwrap().unpaid, 1);
        assert_eq!(pools.pool("drivers").unwrap().left_last_payday, 0);

        // Paid in full at the fourth: the arrears clear and everyone works.
        economy.money = Fx::from_num(100);
        let report = pools.tick(400, &mut economy).unwrap();
        assert_eq!((report.paid, report.unpaid, report.left), (3, 0, 0));
        assert_eq!(pools.unpaid(), 0);
        assert_eq!(pools.walked_out_last_payday(), 0);
        pools.reset();
        assert_eq!(pools.pool("cooks").unwrap().available, 1);
    }

    #[test]
    fn a_zero_wage_head_is_never_unpaid_and_never_leaves() {
        let mut pools = StaffPools::new(catalog());
        let mut economy = economy_with(0);
        let report = pools.tick(100, &mut economy).unwrap();
        assert_eq!((report.paid, report.unpaid, report.left), (0, 3, 0));
        let report = pools.tick(200, &mut economy).unwrap();
        assert_eq!((report.paid, report.unpaid, report.left), (0, 3, 3));
        assert_eq!(pools.headcount("volunteers"), 1);
        assert_eq!(pools.pool("volunteers").unwrap().unpaid, 0);
        assert_eq!(pools.bill_per_period(), 0);
        pools.reset();
        assert_eq!(pools.pool("volunteers").unwrap().available, 1);
    }

    /// Firing takes a head in arrears before a working one, and the fired
    /// head stays owed.
    #[test]
    fn fire_takes_an_unpaid_head_first_and_keeps_it_owed() {
        let mut pools = StaffPools::new(catalog());
        let mut economy = economy_with(3);
        pools.tick(100, &mut economy).unwrap();
        assert_eq!(pools.pool("cooks").unwrap().unpaid, 1);
        assert!(pools.fire("cooks"));
        let cooks = pools.pool("cooks").unwrap();
        assert_eq!((cooks.headcount, cooks.unpaid, cooks.owed), (1, 0, 2));
        pools.reset();
        assert_eq!(pools.pool("cooks").unwrap().available, 1);
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
        assert_eq!(snap.next_bill, 11);
        assert_eq!(snap.shortfall, 0);
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
        assert_eq!(
            (cooks.owed, cooks.unpaid, cooks.left_last_payday),
            (2, 0, 0)
        );
        assert!(cooks.can_hire, "42 covers 11 + 3");
        let poor = pools.snapshot(Fx::from_num(10));
        assert_eq!(poor.shortfall, 1);
        assert!(!poor.categories[0].can_hire);
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
            "owed",
            "unpaid",
            "left_last_payday",
            "can_hire",
            "demand",
            "assigned",
            "short",
        ] {
            assert!(keys.contains(&key), "{key} missing from {keys:?}");
        }
        for key in ["next_bill", "shortfall", "unpaid_last_payday"] {
            assert!(json.get(key).is_some(), "{key} missing");
        }
    }

    fn saved<'a>(
        headcount: &'a [(String, u32)],
        unpaid: &'a [(String, u32)],
        owed: &'a [(String, u32)],
    ) -> StaffRestore<'a> {
        StaffRestore {
            headcount,
            unpaid,
            owed,
            left_last_payday: &[],
            next_payday_tick: 450,
            last_bill: 9,
            unpaid_last_payday: 3,
            walked_out_last_payday: 2,
        }
    }

    #[test]
    fn restore_keeps_the_starting_count_for_a_category_the_save_omits() {
        let cooks7 = [("cooks".to_string(), 7)];
        let restored = StaffPools::restore(catalog(), saved(&cooks7, &[], &[])).unwrap();
        assert_eq!(restored.headcount("cooks"), 7);
        assert_eq!(restored.headcount("drivers"), 1);
        assert_eq!(restored.next_payday_tick(), 450);
        assert_eq!(restored.last_bill(), 9);
        assert_eq!(restored.unpaid_last_payday(), 3);
        assert_eq!(restored.walked_out_last_payday(), 2);
        // A save with no arrears and no owed list: nobody in arrears, every
        // head owed, everyone at their post.
        assert_eq!(restored.pool("cooks").unwrap().owed, 7);
        assert_eq!(restored.pool("cooks").unwrap().unpaid, 0);
        assert_eq!(restored.pool("cooks").unwrap().available, 7);
        let plumbers = [("plumbers".to_string(), 1)];
        assert_eq!(
            StaffPools::restore(catalog(), saved(&plumbers, &[], &[])),
            Err(StaffRestoreError::UnknownCategory("plumbers".to_string()))
        );
    }

    #[test]
    fn restore_takes_arrears_and_owed_and_refuses_an_impossible_state() {
        let cooks3 = [("cooks".to_string(), 3)];
        let unpaid2 = [("cooks".to_string(), 2)];
        let owed5 = [("cooks".to_string(), 5)];
        let restored = StaffPools::restore(catalog(), saved(&cooks3, &unpaid2, &owed5)).unwrap();
        let cooks = restored.pool("cooks").unwrap();
        assert_eq!((cooks.headcount, cooks.unpaid, cooks.owed), (3, 2, 5));
        assert_eq!(
            cooks.available, 1,
            "the two in arrears are not at their post"
        );
        assert_eq!(restored.next_bill(), 5 * 3 + 5);

        let unpaid4 = [("cooks".to_string(), 4)];
        assert!(matches!(
            StaffPools::restore(catalog(), saved(&cooks3, &unpaid4, &[])),
            Err(StaffRestoreError::Inconsistent { ref category, .. }) if category == "cooks"
        ));
        let owed2 = [("cooks".to_string(), 2)];
        assert!(matches!(
            StaffPools::restore(catalog(), saved(&cooks3, &[], &owed2)),
            Err(StaffRestoreError::Inconsistent { ref category, .. }) if category == "cooks"
        ));
    }
}
