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
//! **The way out of arrears is MOTHBALLING** (issue #19). Unpaid heads idle
//! the machines they crew, which cuts the income that would have paid them,
//! so before mothballing existed a factory that missed one payday could
//! only watch the next one miss harder, and the only lever it had was
//! FIRING — which takes the trained crew away as the price of keeping the
//! factory, and leaves a broke factory unable to hire them back
//! ([`can_hire`](StaffPools::can_hire) refuses while the balance cannot
//! cover the next bill). Mothballing is the lever that costs nothing
//! permanent.
//!
//! A mothballed machine (`PlayerCommand::SetMothballed`) asks for no crew,
//! so it drops out of the ESTABLISHMENT ([`StaffPools::set_required`]) —
//! and the heads it was holding are **STOOD DOWN**
//! ([`CategoryPool::stood_down`]): still hired, still there, but OFF THE
//! PAYROLL. That is the whole mechanism, and it is the only thing in the
//! game that lowers a wage bill without losing a head.
//!
//! Standing down bills exactly like firing, minus the loss:
//!
//! * [`bill_per_period`](StaffPools::bill_per_period) — the run rate —
//!   falls the instant the switch is thrown, so the plate shows the player
//!   the relief they just bought.
//! * [`next_bill`](StaffPools::next_bill) does NOT, because `owed` does not
//!   fall mid-period. A head stood down halfway through a period still
//!   worked half of it. This is the same notice-pay rule a fired head gets,
//!   and it is what stops mothballing-on-payday-eve being a free period of
//!   labour, exactly as `owed` stops fire-then-rehire being one.
//! * From the NEXT payday on, the stood-down heads are not billed at all,
//!   and a head that is not billed cannot go unpaid — so a factory that
//!   stands its whole crew down settles at a bill of zero, clears its
//!   arrears, keeps every head, and switches back on. It pays for that
//!   with the periods of production it did not have.
//! * **A stood-down head never walks out.** Without that the two-strike
//!   rule would take the whole crew at the payday BEFORE the relief
//!   lands, since the period the switch was thrown in is still billed —
//!   the escape would close on the player's fingers. A head whose line has
//!   been switched off is not being asked to turn up.
//!
//! Switching the machine back on puts its crew straight back on the payroll
//! (`owed` rises again at once — no re-hiring, no [`can_hire`](StaffPools::can_hire) gate, no
//! lost training) and resumes the batch it was mid-way through.
//!
//! Only the crew of MOTHBALLED machinery is stood down, never a merely
//! surplus head: [`CategoryPool::stood_down`] is capped both by the heads
//! the mothballed machinery would need and by the heads left over once the
//! running machinery is crewed, so over-hiring costs what it always cost.
//!
//! Integers throughout except the money itself, which is the economy's
//! `Money` (ARCHITECTURE.md §3.9). Pools are a `Vec` in file order — the panel
//! order and the payday order — never a hash map.

use serde::{Deserialize, Serialize};

use super::economy::Economy;
use super::machines::MachineRegistry;
use super::types::Money;

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

/// WHO ASKS for a category: one placed kind and how many heads of that
/// category every placement of it needs, totalled — a second crusher
/// doubles the crusher's own count rather than adding a second line. The
/// wire form of the establishment's sources
/// ([`StaffCategorySnapshot::sources`]), so the HUD can name the machines
/// a hiring figure comes from without re-deriving it from the placements:
///
/// ```json
/// {"kind": "cooking_vat", "count": 2}
/// ```
///
/// MOTHBALLED machinery is not in here, for the same reason it is not in
/// `required` — it is not asking for anybody.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrewSource {
    pub kind: String,
    pub count: u32,
}

/// One category's whole establishment line, as
/// `World::recompute_establishment` hands it to
/// [`StaffPools::set_required`]: how many heads the running machinery
/// needs, how many more the mothballed machinery would need, and which
/// kinds the first figure comes from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EstablishmentEntry {
    pub category: String,
    /// Heads the machinery that is switched ON needs.
    pub required: u32,
    /// Heads the MOTHBALLED machinery would need if every switch went back
    /// on — the ceiling on how many heads may be stood down.
    pub mothballed: u32,
    /// The kinds behind `required`, in the world's own tile order.
    pub sources: Vec<CrewSource>,
}

/// One category's live numbers. `headcount`, `unpaid`, `owed` and
/// `left_last_payday` persist (and ride the save); `available`, `demand`
/// and `assigned` are this tick's tally, reset at the top of every tick;
/// `required` is the ESTABLISHMENT, derived from what is placed and
/// changed only by a topology change (see [`StaffPools::set_required`]).
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
    /// The ESTABLISHMENT: every head the placed machinery that is SWITCHED
    /// ON needs to be fully crewed, whether or not any of it is running
    /// this tick. Summed from the placements by
    /// `World::recompute_establishment`, so it moves only when a machine is
    /// placed, removed, rotated or mothballed — never with production flow.
    /// This is the number "do I need to hire?" is answered from; `demand`
    /// remains the honest per-tick activity figure beside it.
    pub required: u32,
    /// The establishment the MOTHBALLED machinery would add back if every
    /// switch went on again. Summed the same way, from the same walk, and
    /// the ceiling on [`stood_down`](Self::stood_down).
    pub mothballed_required: u32,
    /// Heads STOOD DOWN: on the books, not on the payroll, because the
    /// machinery they crew is mothballed (issue #19; see this module's
    /// doc). Derived, never commanded —
    /// `min(headcount - required, mothballed_required)` — so it is bounded
    /// twice over: it can never stand down a head the running machinery
    /// still needs, and it can never stand down more heads than the
    /// mothballed machinery was holding, which is what keeps a merely
    /// surplus head costing full wage.
    pub stood_down: u32,
}

impl CategoryPool {
    /// Heads ON THE PAYROLL: hired, minus the ones stood down with the
    /// machinery they crew. The run-rate figure
    /// ([`StaffPools::bill_per_period`]) counts these and nothing else.
    pub fn on_payroll(&self) -> u32 {
        self.headcount.saturating_sub(self.stood_down)
    }
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
    /// Parallel to `pools`: who asks for that category, from the same walk
    /// that set `required`. Kept beside the pools rather than on
    /// [`CategoryPool`] so that stays a `Copy` bag of counters.
    sources: Vec<Vec<CrewSource>>,
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
                required: 0,
                mothballed_required: 0,
                stood_down: 0,
            })
            .collect();
        let sources = catalog.categories.iter().map(|_| Vec::new()).collect();
        let next_payday_tick = catalog.pay_period_ticks;
        Self {
            catalog,
            pools,
            sources,
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

    /// The run rate: what a payday costs at the current headcounts once
    /// the line is settled, `sum((headcount - stood_down) * wage)`.
    ///
    /// **This is the figure mothballing moves at once** (issue #19): a head
    /// whose machine has been switched off is off the payroll from that
    /// tick, so throwing the switch shows the player the relief they just
    /// bought, in the same breath as the machine going quiet. What the NEXT
    /// payday will actually charge is [`next_bill`](Self::next_bill), which
    /// still counts the period already half-worked — see this module's doc.
    pub fn bill_per_period(&self) -> u32 {
        self.catalog
            .categories
            .iter()
            .zip(&self.pools)
            .map(|(c, p)| c.wage.saturating_mul(p.on_payroll()))
            .fold(0u32, u32::saturating_add)
    }

    /// What the next payday will charge: `sum(owed * wage)` — every head
    /// employed for any part of this period, fired or not, stood down or
    /// not. `owed` is the memory that makes both notice pay and
    /// mothballing cost the period they happened in
    /// ([`Self::sync_payroll`]).
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
    pub fn shortfall(&self, money: Money) -> u32 {
        let bill = Money::saturating_from_num(self.next_bill());
        if money >= bill {
            0
        } else {
            (bill - money).ceil().saturating_to_num::<u32>()
        }
    }

    /// Whether `hire(category)` would be accepted with `money` in hand:
    /// the category is known and the balance covers the next bill plus
    /// the new head's wage. You cannot hire someone you could not pay.
    pub fn can_hire(&self, category: &str, money: Money) -> bool {
        let Some(c) = self.catalog.get(category) else {
            return false;
        };
        let needed = Money::saturating_from_num(self.next_bill().saturating_add(c.wage));
        money >= needed
    }

    /// Sets the ESTABLISHMENT — how many heads of each category the placed
    /// machinery needs to be fully crewed — from one line per category.
    ///
    /// Called from `World::recompute_establishment`, which is reached by
    /// `Place`, `Remove`, `Rotate` and `SetMothballed` — all four of them
    /// deliberate player actions — so this figure is **stable during
    /// play**: it cannot move because a belt ran dry or a vat finished a
    /// batch. That stability is the whole point. `demand` (what was asked
    /// for this tick) breathes with production flow, which made
    /// "do I need to hire?" unanswerable from the HUD — the number that
    /// prompted a hire had already gone by the time the player acted
    /// (docs/DECISIONS.md, the 2026-09-16 establishment record).
    ///
    /// Each entry carries three things the HUD must not re-derive: the
    /// heads the RUNNING machinery needs, the heads the MOTHBALLED
    /// machinery would need (the stand-down ceiling), and which kinds the
    /// first figure comes from. Categories the caller does not mention are
    /// zeroed on all three, so a factory whose last crusher was removed —
    /// or MOTHBALLED, which is the same thing to the payroll (issue #19) —
    /// stops asking for cooks. An unknown category id is ignored rather
    /// than rejected: the catalog is the authority on what exists, and
    /// `validate_machines` has already proved every machine's crew names a
    /// real category.
    ///
    /// Ends in [`sync_payroll`](Self::sync_payroll), so the wage bill moves
    /// in the same call the establishment does and the two can never be
    /// read out of step.
    pub fn set_required(&mut self, establishment: &[EstablishmentEntry]) {
        for pool in &mut self.pools {
            pool.required = 0;
            pool.mothballed_required = 0;
        }
        for sources in &mut self.sources {
            sources.clear();
        }
        for entry in establishment {
            if let Some(i) = self.catalog.index_of(&entry.category) {
                self.pools[i].required = self.pools[i].required.saturating_add(entry.required);
                self.pools[i].mothballed_required = self.pools[i]
                    .mothballed_required
                    .saturating_add(entry.mothballed);
                self.sources[i].extend(entry.sources.iter().cloned());
            }
        }
        self.sync_payroll();
    }

    /// Recomputes who is STOOD DOWN and what that does to the payroll, from
    /// the establishment and the headcounts. Called from every path that
    /// can move either — [`set_required`](Self::set_required),
    /// [`hire`](Self::hire), [`fire`](Self::fire) and payday itself — so
    /// `stood_down` is never a cached number that can go stale.
    ///
    /// `stood_down` is the surplus the mothballed machinery accounts for
    /// and no more (see [`CategoryPool::stood_down`]). `owed` then rises to
    /// cover everyone back on the payroll but **never falls here**: that is
    /// the notice-pay rule, and it is why standing a crew down costs you
    /// the period you did it in and spares you the ones after. Only payday
    /// lets `owed` fall, in [`tick`](Self::tick).
    fn sync_payroll(&mut self) {
        for pool in &mut self.pools {
            pool.stood_down = pool
                .headcount
                .saturating_sub(pool.required)
                .min(pool.mothballed_required);
            pool.owed = pool.owed.max(pool.on_payroll());
        }
    }

    /// Who asks for a category, by kind, in the world's own tile order —
    /// the establishment's own sources, as `set_required` was given them.
    /// Empty for an unknown category and for one nothing placed asks for.
    pub fn sources(&self, category: &str) -> &[CrewSource] {
        self.catalog
            .index_of(category)
            .map_or(&[], |i| self.sources[i].as_slice())
    }

    /// The establishment for one category — what the placed machinery needs.
    pub fn required(&self, category: &str) -> u32 {
        self.catalog
            .index_of(category)
            .map_or(0, |i| self.pools[i].required)
    }

    /// How many heads of `category` the player must HIRE for the placed
    /// machinery to be fully crewed: `required - headcount`, floored at 0.
    ///
    /// Deliberately measured against `headcount` and not `available`: a head
    /// in arrears is not working, but hiring a second head to cover them is
    /// the wrong advice — paying them is. The unpaid count is reported
    /// separately and reads as its own warning.
    pub fn short(&self, category: &str) -> u32 {
        self.catalog.index_of(category).map_or(0, |i| {
            self.pools[i]
                .required
                .saturating_sub(self.pools[i].headcount)
        })
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
    pub fn hire(&mut self, category: &str, money: Money) -> bool {
        if !self.can_hire(category, money) {
            return false;
        }
        let i = self
            .catalog
            .index_of(category)
            .expect("can_hire is false for an unknown category");
        self.pools[i].headcount = self.pools[i].headcount.saturating_add(1);
        self.pools[i].owed = self.pools[i].owed.saturating_add(1);
        self.sync_payroll();
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
                // A head fired out of a stood-down surplus is one fewer
                // head to stand down, not a second saving.
                self.sync_payroll();
                true
            }
            _ => false,
        }
    }

    /// Payday, run after the economy's own tick. When `current_tick` has
    /// reached `next_payday_tick`, pays every OWED head — which is every
    /// head that was on the payroll for any part of the period, and so
    /// excludes the ones stood down for the whole of it (issue #19) — one
    /// at a time in
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
                pool.owed = pool.on_payroll();
                continue;
            }
            let wage = Money::from_num(category.wage);
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
            // the books is the rest. `owed` can sit BELOW the headcount
            // when heads are stood down (issue #19), so the heads this
            // payday actually billed is the smaller of the two — a head
            // nobody billed cannot go unpaid, which is exactly how a
            // shutdown clears arrears instead of deepening them.
            let departed = pool.owed.saturating_sub(pool.headcount);
            let billed = pool.owed.min(pool.headcount);
            let current_paid = paid_heads.saturating_sub(departed);
            let unpaid_now = billed.saturating_sub(current_paid);
            // Back pay first: the money that reached the current heads went
            // to the ones in arrears before anyone else, so an arrears head
            // it did NOT reach was unpaid twice and leaves; a clear head it
            // did not reach is in arrears for the first time and stays.
            //
            // **A STOOD-DOWN HEAD NEVER WALKS OUT** (issue #19), which is
            // what makes mothballing an escape and not a slower death. The
            // period a stand-down is thrown in is still billed in full
            // (`owed`, notice pay), so a factory that mothballs the day
            // after it missed payroll misses the next one too — and
            // without this clause the two-strike rule would take its whole
            // crew at exactly the payday the relief was about to arrive.
            // A head whose line the player has switched off is not being
            // asked to work; they are owed their notice pay, and then they
            // wait. Only the heads the factory is still asking to turn up
            // — `on_payroll` — can walk.
            let left = pool
                .unpaid
                .saturating_sub(current_paid)
                .min(unpaid_now)
                .min(pool.on_payroll());
            pool.headcount -= left;
            pool.unpaid = unpaid_now.saturating_sub(left);
            pool.left_last_payday = left;
            // The new period starts everyone who is ON THE PAYROLL owed;
            // the stood-down are not billed until they are back on it.
            pool.owed = pool.on_payroll();
            unpaid_total = unpaid_total.saturating_add(unpaid_now);
            left_total = left_total.saturating_add(left);
        }
        // Walkouts moved headcounts, so the stand-down is re-derived from
        // what is left; `owed` was just set per category above and
        // `sync_payroll` only ever raises it, never lowers it.
        self.sync_payroll();
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
    pub fn snapshot(&self, money: Money) -> StaffSnapshot {
        StaffSnapshot {
            pay_period_ticks: self.catalog.pay_period_ticks,
            next_payday_tick: self.next_payday_tick,
            next_bill: self.next_bill(),
            bill_per_period: self.bill_per_period(),
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
                    required: p.required,
                    short: p.required.saturating_sub(p.headcount),
                    stood_down: p.stood_down,
                    on_payroll: p.on_payroll(),
                    sources: self.sources(&c.id).to_vec(),
                })
                .collect(),
        }
    }

    /// The save seam (`save.rs`): the persisted half of the pools. A
    /// category a list does not name takes its default (see
    /// [`StaffRestore`]), so a category added to `staff.ron` after a save
    /// was written starts as a new game would, and a save from before the
    /// quality pass — no arrears, no `owed` — loads with nobody in arrears
    /// and every head owed. An id the catalog does not know, or arrears
    /// above the headcount (a state the tick path cannot produce), is
    /// refused. `owed` BELOW the headcount is not: since standing down
    /// (issue #19) that is what a save taken during a shutdown looks like,
    /// and the establishment that justifies it is rebuilt a moment later by
    /// `World::recompute_establishment`, which repairs any `owed` the
    /// stand-down does not account for. The per-tick tallies are not
    /// restored: the first tick after a load recomputes them, like the
    /// topology.
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
        }
        for p in &mut pools.pools {
            p.available = p.headcount.saturating_sub(p.unpaid);
        }
        // The establishment is deliberately NOT synced here. It is not in
        // the save: `World::recompute_establishment` re-sums it from the
        // replayed placements a moment later and calls `sync_payroll` with
        // it. Doing it here, with nothing yet known to be mothballed,
        // would raise a shutdown's `owed` back to the full headcount and
        // bill a factory for the period it had stood its crew down in.
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
///  "next_bill": 14, "bill_per_period": 14, "shortfall": 0,
///  "last_bill": 14, "unpaid_last_payday": 0, "walked_out_last_payday": 0,
///  "money": 2986.0,
///  "categories": [
///    {"id": "manufacturing", "name": "Manufacturing", "headcount": 4,
///     "wage": 2, "owed": 4, "unpaid": 0, "left_last_payday": 0,
///     "can_hire": true, "required": 4, "demand": 4, "assigned": 4,
///      "short": 0, "stood_down": 0, "on_payroll": 4,
///      "sources": [{"kind": "crusher", "count": 1}]}, ...]}
/// ```
///
/// `demand` and `assigned` describe the LAST completed tick — what was
/// asked for and what was handed out — and both breathe with production
/// flow. `required` and `short` do NOT: `required` is the establishment
/// the PLACED machinery needs (`StaffPools::set_required`, recomputed only
/// on a topology change) and `short` is `required - headcount`, the heads
/// the player must HIRE. A panel answering "do I need staff?" reads
/// `short`; one showing "what is working right now" reads `assigned`.
/// Before 2026-09-16 `short` meant `demand - assigned`, which moved every
/// tick and so could not be acted on. `money` is repeated here, as `shop_snapshot_json`
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
    /// The RUN RATE: what a payday costs once the current stand-down has
    /// been in place for a whole period
    /// ([`StaffPools::bill_per_period`]). Equal to `next_bill` on a
    /// factory with nothing mothballed; below it the moment a switch is
    /// thrown, which is how the HUD shows the relief before the payday
    /// that delivers it.
    pub bill_per_period: u32,
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
    pub required: u32,
    pub short: u32,
    /// Heads stood down with mothballed machinery — off the payroll,
    /// still on the books (issue #19).
    pub stood_down: u32,
    /// `headcount - stood_down`: what the run rate is charged for.
    pub on_payroll: u32,
    /// Which placed kinds `required` comes from, in the world's tile
    /// order — the HUD's `asked for by:` line and the CREW plate's
    /// caption, so neither re-derives a number the wire already carries
    /// (`godot_project/CLAUDE.md`). Mothballed machinery is not in here,
    /// which is what keeps the caption from naming a machine that has
    /// left the establishment.
    pub sources: Vec<CrewSource>,
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

    /// An establishment line with nothing mothballed — what
    /// `recompute_establishment` hands over for a factory with every
    /// switch on.
    fn running(category: &str, required: u32) -> EstablishmentEntry {
        EstablishmentEntry {
            category: category.to_string(),
            required,
            mothballed: 0,
            sources: vec![CrewSource {
                kind: "crusher".to_string(),
                count: required,
            }],
        }
    }

    /// An establishment line for machinery that is placed and SWITCHED
    /// OFF: it needs nobody, and `mothballed` heads may be stood down.
    fn switched_off(category: &str, required: u32, mothballed: u32) -> EstablishmentEntry {
        EstablishmentEntry {
            category: category.to_string(),
            required,
            mothballed,
            sources: if required == 0 {
                Vec::new()
            } else {
                vec![CrewSource {
                    kind: "crusher".to_string(),
                    count: required,
                }]
            },
        }
    }

    fn economy_with(money: u32) -> Economy {
        Economy::with_money(
            &crate::sim::contracts::ContractRegistry::bundled(),
            Money::from_num(money),
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
        assert_eq!(pools.bill_per_period(), (2 * 3) + 5);
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
        let rich = Money::from_num(1000);
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
        assert!(!pools.can_hire("cooks", Money::from_num(13)));
        assert!(!pools.hire("cooks", Money::from_num(13)));
        assert_eq!(
            pools.headcount("cooks"),
            2,
            "a refused hire changes nothing"
        );
        assert!(pools.can_hire("cooks", Money::from_num(14)));
        assert!(pools.hire("cooks", Money::from_num(14)));
        assert_eq!(pools.next_bill(), 14);
        // A volunteer costs nothing extra, but the bill itself must still
        // be covered.
        assert!(!pools.can_hire("volunteers", Money::from_num(13)));
        assert!(pools.can_hire("volunteers", Money::from_num(14)));
        assert!(!pools.can_hire("plumbers", Money::from_num(1_000)));
        assert_eq!(pools.shortfall(Money::from_num(14)), 0);
        assert_eq!(pools.shortfall(Money::from_num(13)), 1);
        assert_eq!(
            pools.shortfall(Money::from_num(12.5)),
            2,
            "a part unit short is a whole unit short"
        );
        assert_eq!(pools.shortfall(Money::ZERO), 14);
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
        assert_eq!(economy.money, Money::from_num(89));
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
        assert_eq!(economy.money, Money::from_num(84));
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
        assert_eq!(economy.money, Money::from_num(100));
        let report = pools.tick(100, &mut economy).expect("payday at 100");
        assert_eq!(
            report,
            PaydayReport {
                paid: 11,
                unpaid: 0,
                left: 0,
            }
        );
        assert_eq!(economy.money, Money::from_num(89));
        assert_eq!(pools.last_bill(), 11);
        assert_eq!(pools.unpaid_last_payday(), 0);
        assert_eq!(pools.walked_out_last_payday(), 0);
        assert_eq!(pools.next_payday_tick(), 200);
        assert_eq!(pools.tick(101, &mut economy), None);
        assert!(pools.tick(200, &mut economy).is_some());
        assert_eq!(economy.money, Money::from_num(78));
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
        assert_eq!(economy.money, Money::from_num(1));
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
        economy.money = Money::from_num(3);
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
        economy.money = Money::from_num(100);
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
        let snap = pools.snapshot(Money::from_num(42));
        assert_eq!(snap.money, 42.0);
        assert_eq!(snap.pay_period_ticks, 100);
        assert_eq!(snap.next_payday_tick, 100);
        assert_eq!(snap.next_bill, 11);
        assert_eq!(snap.shortfall, 0);
        let cooks = &snap.categories[0];
        assert_eq!(cooks.id, "cooks");
        assert_eq!(cooks.name, "Cooks");
        // `demand`/`assigned` are the tick's tally: three cooks asked for,
        // two handed out. `short` is NOT drawn from them — nothing has been
        // placed, so these pools have no establishment and there is nobody
        // to hire, however busy the tick was.
        assert_eq!(
            (
                cooks.headcount,
                cooks.wage,
                cooks.demand,
                cooks.assigned,
                cooks.required,
                cooks.short
            ),
            (2, 3, 3, 2, 0, 0)
        );
        assert_eq!(
            (cooks.owed, cooks.unpaid, cooks.left_last_payday),
            (2, 0, 0)
        );
        assert!(cooks.can_hire, "42 covers 11 + 3");
        // Give the pools an establishment — what placed machinery would
        // need — and `short` becomes the hiring advice: three cooks' worth
        // of machines standing, two cooks hired, so hire one. It does not
        // matter what this or any other tick asked for.
        pools.set_required(&[running("cooks", 3)]);
        let staffed = pools.snapshot(Money::from_num(42));
        assert_eq!(
            (staffed.categories[0].required, staffed.categories[0].short),
            (3, 1)
        );
        pools.reset();
        let idle = pools.snapshot(Money::from_num(42));
        assert_eq!(
            (idle.categories[0].demand, idle.categories[0].short),
            (0, 1),
            "a tick that asked for nobody does not mean nobody is needed"
        );

        let poor = pools.snapshot(Money::from_num(10));
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
        // `owed` BELOW the headcount is NOT impossible since standing
        // down (issue #19) — it is what a save taken during a shutdown
        // looks like — so it loads, and the establishment that lands a
        // moment later decides how much of it survives. With nothing
        // mothballed, `sync_payroll` puts every head back on the payroll.
        let owed2 = [("cooks".to_string(), 2)];
        let shutdown = StaffPools::restore(catalog(), saved(&cooks3, &[], &owed2)).unwrap();
        assert_eq!(shutdown.pool("cooks").unwrap().owed, 2);
        let mut shutdown = shutdown;
        shutdown.set_required(&[switched_off("cooks", 0, 3)]);
        let cooks = shutdown.pool("cooks").unwrap();
        assert_eq!(
            (cooks.stood_down, cooks.on_payroll(), cooks.owed),
            (3, 0, 2),
            "three cooks stood down with the machinery they crew are off the payroll,              and the shutdown the save was taken in is not billed over again"
        );
        let mut running_again =
            StaffPools::restore(catalog(), saved(&cooks3, &[], &owed2)).unwrap();
        running_again.set_required(&[running("cooks", 3)]);
        assert_eq!(
            running_again.pool("cooks").unwrap().owed,
            3,
            "nothing is mothballed, so a saved `owed` below the headcount is repaired"
        );
    }
}
