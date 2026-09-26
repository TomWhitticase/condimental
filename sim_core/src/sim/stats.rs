//! Production statistics — the simulation half of the **P** screen (the
//! production-stats epic, 2026-09-12; Factorio's production GUI): how many
//! of each item and fluid the factory MADE and USED over the last few
//! seconds, minute, ten minutes and hour, and over its whole life.
//!
//! **What counts, in one rule.** *Produced* is what a recipe deposits at
//! craft completion; *consumed* is what a recipe takes at craft start plus
//! what a contract takes off the docks when it pays — the
//! factory's exit, the way a rocket launch is consumption in Factorio.
//! Buying stock, filling a hopper or tank, and a hopper metering a unit
//! onto its belt are deliberately **not** counted: the market is not the
//! factory, and moving a unit between bins is neither making nor using it.
//! So a raw ingredient appears only on the consumption side, an
//! intermediate on both, and the boxed product on both once a contract
//! has paid.
//!
//! **Shape.** One [`ProductionLedger`] per world. Per item id, one ring of
//! [`STAT_BUCKETS`] buckets per window in [`STAT_WINDOWS`], for produced
//! and for consumed separately, plus two lifetime totals. All the windows
//! share one clock: every tick advances every window's `ticks_into_bucket`,
//! and when a window's bucket is full its cursor moves on and the bucket it
//! lands on is cleared for every item — so a window's sum always covers its
//! last `ticks` and the partial current bucket, never more. Integers only
//! (ARCHITECTURE.md §3.9); a fluid is counted in the whole units a recipe
//! names, never as a volume.
//!
//! **Why the sim owns it** (the same reasons `machine::LoadWindow` gives):
//! it is deterministic, it survives a save (`save.rs`'s `SavedProduction`),
//! it is tested headlessly here, and a Godot-side tally would be gameplay
//! bookkeeping in the presentation layer. It **never dirties a chunk**: it
//! changes every tick a machine is running, so it is a polled readout
//! (`World::production_snapshot`) like the load and the fluid network.
//!
//! **The pay cycle** (ticket 7, 2026-09-26). Beside the rolling windows,
//! every id keeps what it PRODUCED since the last payday and in the whole
//! period before it: the staff payday (`staff.ron`'s `pay_period_ticks`) is
//! the game's one global cycle, the one the wage bill and its countdown
//! already run on — a contract's own cadence is per contract, so it cannot
//! be the period a factory-wide tally rolls over on. `World::tick` calls
//! [`ProductionLedger::close_cycle`] on the tick payday lands. What the
//! active contracts ASK for over the same period is the economy's
//! (`Economy::demand_per_period`); [`ProductionLedger::cycle_lines`] puts
//! the two side by side. Neither ring can derive these — no bucket edge
//! lands on a payday — so they are saved (`SavedItemStats`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use fixed::types::I48F16;

use super::types::ItemId;

/// Buckets per window. Thirty, so every shipped window divides into a
/// whole number of ticks per bucket (5, 60, 600, 3600).
pub const STAT_BUCKETS: usize = 30;

/// One time window the ledger keeps: its player-facing id and its length
/// in ticks at Godot's 30 Hz tick rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatWindow {
    pub id: &'static str,
    pub ticks: u32,
}

impl StatWindow {
    /// Ticks per bucket — `ticks / STAT_BUCKETS`, which [`STAT_WINDOWS`]
    /// is chosen to make exact.
    pub const fn bucket_ticks(self) -> u32 {
        self.ticks / STAT_BUCKETS as u32
    }
}

/// The windows, shortest first: five seconds, a minute, ten minutes, an
/// hour. Factorio's 10h and 50h are dropped — no session here is that long
/// — and "all time" is the pair of lifetime totals, not a window.
pub const STAT_WINDOWS: [StatWindow; 4] = [
    StatWindow {
        id: "5s",
        ticks: 150,
    },
    StatWindow {
        id: "1m",
        ticks: 1_800,
    },
    StatWindow {
        id: "10m",
        ticks: 18_000,
    },
    StatWindow {
        id: "1h",
        ticks: 108_000,
    },
];

/// How many windows there are — the length of every per-window array.
pub const STAT_WINDOW_COUNT: usize = STAT_WINDOWS.len();

/// Whether an id is a solid item or a fluid. Recorded with the first
/// entry and never re-derived: a recipe knows which list it took the id
/// from, and Godot draws the two differently (a fluid is named as one and
/// its icon is the jug it is bought in).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatKind {
    Item,
    Fluid,
}

impl StatKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StatKind::Item => "item",
            StatKind::Fluid => "fluid",
        }
    }
}

/// One id's counts: a produced ring and a consumed ring per window, and
/// two lifetime totals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStats {
    pub kind: StatKind,
    pub produced: [[u32; STAT_BUCKETS]; STAT_WINDOW_COUNT],
    pub consumed: [[u32; STAT_BUCKETS]; STAT_WINDOW_COUNT],
    pub produced_total: u64,
    pub consumed_total: u64,
    /// Produced since the last payday.
    pub produced_this_cycle: u32,
    /// Produced in the whole pay period before that one; zero until the
    /// first payday.
    pub produced_last_cycle: u32,
}

impl ItemStats {
    fn new(kind: StatKind) -> Self {
        Self {
            kind,
            produced: [[0; STAT_BUCKETS]; STAT_WINDOW_COUNT],
            consumed: [[0; STAT_BUCKETS]; STAT_WINDOW_COUNT],
            produced_total: 0,
            consumed_total: 0,
            produced_this_cycle: 0,
            produced_last_cycle: 0,
        }
    }
}

/// The world's production ledger. See the module doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionLedger {
    items: BTreeMap<ItemId, ItemStats>,
    /// The current bucket of each window, in [`STAT_WINDOWS`] order.
    cursor: [usize; STAT_WINDOW_COUNT],
    /// How many ticks the current bucket of each window has absorbed.
    ticks_into_bucket: [u32; STAT_WINDOW_COUNT],
    /// Ticks this ledger has observed, so a window younger than itself can
    /// report a true rate (`observed = min(age, window.ticks)`).
    age: u64,
    /// Paydays this ledger has closed — zero means there is no "last
    /// cycle" yet, so a zero there is not a shortfall.
    cycles_closed: u64,
}

impl Default for ProductionLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductionLedger {
    pub fn new() -> Self {
        Self {
            items: BTreeMap::new(),
            cursor: [0; STAT_WINDOW_COUNT],
            ticks_into_bucket: [0; STAT_WINDOW_COUNT],
            age: 0,
            cycles_closed: 0,
        }
    }

    /// Adds `qty` to `id`'s produced count in every window and its total.
    pub fn record_produced(&mut self, id: &str, kind: StatKind, qty: u32) {
        if qty == 0 {
            return;
        }
        let cursor = self.cursor;
        let stats = self.entry(id, kind);
        for (w, &c) in cursor.iter().enumerate() {
            stats.produced[w][c] = stats.produced[w][c].saturating_add(qty);
        }
        stats.produced_total = stats.produced_total.saturating_add(u64::from(qty));
        stats.produced_this_cycle = stats.produced_this_cycle.saturating_add(qty);
    }

    /// Adds `qty` to `id`'s consumed count in every window and its total.
    pub fn record_consumed(&mut self, id: &str, kind: StatKind, qty: u32) {
        if qty == 0 {
            return;
        }
        let cursor = self.cursor;
        let stats = self.entry(id, kind);
        for (w, &c) in cursor.iter().enumerate() {
            stats.consumed[w][c] = stats.consumed[w][c].saturating_add(qty);
        }
        stats.consumed_total = stats.consumed_total.saturating_add(u64::from(qty));
    }

    fn entry(&mut self, id: &str, kind: StatKind) -> &mut ItemStats {
        if !self.items.contains_key(id) {
            self.items.insert(id.to_string(), ItemStats::new(kind));
        }
        self.items.get_mut(id).expect("inserted above")
    }

    /// Advances every window's clock by one tick. Called exactly once per
    /// `World::tick`, after everything that records — so a tick's events
    /// land in the bucket that was current while they happened, and a
    /// bucket that fills is left behind, not cleared under them.
    pub fn tick(&mut self) {
        self.age = self.age.saturating_add(1);
        for (w, window) in STAT_WINDOWS.iter().enumerate() {
            self.ticks_into_bucket[w] += 1;
            if self.ticks_into_bucket[w] >= window.bucket_ticks() {
                self.ticks_into_bucket[w] = 0;
                self.cursor[w] = (self.cursor[w] + 1) % STAT_BUCKETS;
                let c = self.cursor[w];
                for stats in self.items.values_mut() {
                    stats.produced[w][c] = 0;
                    stats.consumed[w][c] = 0;
                }
            }
        }
    }

    /// Ends the pay cycle: what was produced this cycle becomes last
    /// cycle's, and this cycle starts again at zero. Called by
    /// `World::tick` on the tick payday lands, after everything that
    /// records, so that tick's crafts belong to the cycle it closes.
    pub fn close_cycle(&mut self) {
        self.cycles_closed = self.cycles_closed.saturating_add(1);
        for stats in self.items.values_mut() {
            stats.produced_last_cycle = stats.produced_this_cycle;
            stats.produced_this_cycle = 0;
        }
    }

    /// Paydays closed so far.
    pub fn cycles_closed(&self) -> u64 {
        self.cycles_closed
    }

    pub fn produced_this_cycle(&self, id: &str) -> u32 {
        self.items
            .get(id)
            .map(|s| s.produced_this_cycle)
            .unwrap_or(0)
    }

    pub fn produced_last_cycle(&self, id: &str) -> u32 {
        self.items
            .get(id)
            .map(|s| s.produced_last_cycle)
            .unwrap_or(0)
    }

    /// One line per product the active contracts ask for, in the order
    /// `demand` gives (the economy's: active-list order, so contract
    /// priority): what one pay period requires, what was made this cycle
    /// and last. `short` is last cycle's verdict — a finished period that
    /// made less than it required — and is never set before the first
    /// payday, when there is no finished period to judge. An empty
    /// `demand` (no active contract) is an empty list.
    pub fn cycle_lines(&self, demand: &[(ItemId, I48F16)]) -> Vec<CycleLineSnapshot> {
        demand
            .iter()
            .map(|(id, required)| {
                let last = self.produced_last_cycle(id);
                CycleLineSnapshot {
                    id: id.clone(),
                    required: required.to_num::<f64>(),
                    produced_this_cycle: self.produced_this_cycle(id),
                    produced_last_cycle: last,
                    short: self.cycles_closed > 0 && I48F16::from_num(last) < *required,
                }
            })
            .collect()
    }

    /// Ticks this ledger has observed in total.
    pub fn age(&self) -> u64 {
        self.age
    }

    /// How much of window `w` has actually been observed: the window's
    /// full length once the ledger is older than it, the ledger's age
    /// before that. What a per-minute rate divides by.
    pub fn observed(&self, w: usize) -> u32 {
        let full = u64::from(STAT_WINDOWS[w].ticks);
        self.age.min(full) as u32
    }

    pub fn produced_in(&self, id: &str, w: usize) -> u32 {
        self.items
            .get(id)
            .map(|s| s.produced[w].iter().sum())
            .unwrap_or(0)
    }

    pub fn consumed_in(&self, id: &str, w: usize) -> u32 {
        self.items
            .get(id)
            .map(|s| s.consumed[w].iter().sum())
            .unwrap_or(0)
    }

    pub fn produced_total(&self, id: &str) -> u64 {
        self.items.get(id).map(|s| s.produced_total).unwrap_or(0)
    }

    pub fn consumed_total(&self, id: &str) -> u64 {
        self.items.get(id).map(|s| s.consumed_total).unwrap_or(0)
    }

    pub fn kind_of(&self, id: &str) -> Option<StatKind> {
        self.items.get(id).map(|s| s.kind)
    }

    /// Every id the ledger has ever recorded, in id order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.items.keys().map(String::as_str)
    }

    /// The raw per-id record, in id order — for `save.rs`, which stores
    /// the rings verbatim so a loaded ledger is bit-identical.
    pub fn items(&self) -> &BTreeMap<ItemId, ItemStats> {
        &self.items
    }

    pub fn cursor(&self) -> [usize; STAT_WINDOW_COUNT] {
        self.cursor
    }

    pub fn ticks_into_bucket(&self) -> [u32; STAT_WINDOW_COUNT] {
        self.ticks_into_bucket
    }

    /// Rebuilds a ledger from saved parts. `None` if any cursor is outside
    /// its ring or any window's `ticks_into_bucket` is at or past its
    /// bucket length — states the tick path can never produce, so states
    /// only a corrupt or hand-edited save can. Restore-only.
    pub(crate) fn restore(
        items: BTreeMap<ItemId, ItemStats>,
        cursor: [usize; STAT_WINDOW_COUNT],
        ticks_into_bucket: [u32; STAT_WINDOW_COUNT],
        age: u64,
        cycles_closed: u64,
    ) -> Option<Self> {
        for (w, window) in STAT_WINDOWS.iter().enumerate() {
            if cursor[w] >= STAT_BUCKETS || ticks_into_bucket[w] >= window.bucket_ticks() {
                return None;
            }
        }
        Some(Self {
            items,
            cursor,
            ticks_into_bucket,
            age,
            cycles_closed,
        })
    }

    /// The presentation-facing view — see [`ProductionSnapshot`]. `cycle`
    /// is the world's to fill: the ledger does not know the payday clock or
    /// the contracts.
    pub fn snapshot(&self, cycle: CycleSnapshot) -> ProductionSnapshot {
        ProductionSnapshot {
            cycle,
            windows: STAT_WINDOWS
                .iter()
                .enumerate()
                .map(|(w, window)| WindowSnapshot {
                    id: window.id,
                    ticks: window.ticks,
                    observed: self.observed(w),
                })
                .collect(),
            items: self
                .items
                .iter()
                .map(|(id, stats)| ItemStatsSnapshot {
                    id: id.clone(),
                    kind: stats.kind,
                    produced: stats
                        .produced
                        .iter()
                        .map(|ring| ring.iter().sum())
                        .collect(),
                    consumed: stats
                        .consumed
                        .iter()
                        .map(|ring| ring.iter().sum())
                        .collect(),
                    produced_total: stats.produced_total,
                    consumed_total: stats.consumed_total,
                })
                .collect(),
        }
    }
}

/// What `production_snapshot_json` carries:
///
/// ```json
/// {"windows": [{"id": "5s", "ticks": 150, "observed": 150}, ...],
///  "items": [{"id": "tomato", "kind": "item",
///             "produced": [0, 0, 0, 0], "consumed": [8, 96, 96, 96],
///             "produced_total": 0, "consumed_total": 96}, ...]}
/// ```
///
/// `windows` is [`STAT_WINDOWS`] in order with how much of each has been
/// observed (for a rate); every count array is in that same order. `items`
/// lists every id ever recorded, sorted by id — Godot sorts by count and
/// formats; the sim only counts. `cycle` is the pay-cycle view (see the
/// module doc):
///
/// ```json
/// "cycle": {"period_ticks": 1000, "next_tick": 3000, "cycles_closed": 2,
///           "lines": [{"id": "boxed_ketchup", "required": 6.666656494140625,
///                      "produced_this_cycle": 3, "produced_last_cycle": 5,
///                      "short": true}]}
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductionSnapshot {
    pub cycle: CycleSnapshot,
    pub windows: Vec<WindowSnapshot>,
    pub items: Vec<ItemStatsSnapshot>,
}

/// The pay cycle: its length, the tick it next closes on, how many have
/// closed, and one [`CycleLineSnapshot`] per product an active contract
/// orders — none with no contract running.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CycleSnapshot {
    pub period_ticks: u64,
    pub next_tick: u64,
    pub cycles_closed: u64,
    pub lines: Vec<CycleLineSnapshot>,
}

/// One demanded product. `required` is a float only here, at the
/// presentation edge; the sim holds it fixed-point.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CycleLineSnapshot {
    pub id: ItemId,
    pub required: f64,
    pub produced_this_cycle: u32,
    pub produced_last_cycle: u32,
    pub short: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WindowSnapshot {
    pub id: &'static str,
    pub ticks: u32,
    pub observed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemStatsSnapshot {
    pub id: ItemId,
    pub kind: StatKind,
    pub produced: Vec<u32>,
    pub consumed: Vec<u32>,
    pub produced_total: u64,
    pub consumed_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_window_divides_into_whole_buckets() {
        for window in STAT_WINDOWS {
            assert_eq!(
                window.bucket_ticks() * STAT_BUCKETS as u32,
                window.ticks,
                "{} must be a whole number of buckets",
                window.id
            );
            assert!(window.bucket_ticks() >= 1);
        }
        // Shortest first, strictly — Godot's tabs and the "all ≥ 1m ≥ 5s"
        // assertions both rely on it.
        for pair in STAT_WINDOWS.windows(2) {
            assert!(pair[0].ticks < pair[1].ticks);
        }
    }

    #[test]
    fn a_record_lands_in_every_window_and_the_total() {
        let mut ledger = ProductionLedger::new();
        ledger.record_produced("paste", StatKind::Item, 3);
        ledger.record_consumed("tomato", StatKind::Item, 12);
        for w in 0..STAT_WINDOW_COUNT {
            assert_eq!(ledger.produced_in("paste", w), 3);
            assert_eq!(ledger.consumed_in("tomato", w), 12);
            assert_eq!(ledger.consumed_in("paste", w), 0);
        }
        assert_eq!(ledger.produced_total("paste"), 3);
        assert_eq!(ledger.consumed_total("tomato"), 12);
        assert_eq!(ledger.kind_of("paste"), Some(StatKind::Item));
        assert_eq!(ledger.kind_of("nothing"), None);
        assert_eq!(ledger.ids().collect::<Vec<_>>(), vec!["paste", "tomato"]);
    }

    #[test]
    fn a_zero_record_creates_no_entry() {
        let mut ledger = ProductionLedger::new();
        ledger.record_produced("paste", StatKind::Item, 0);
        assert_eq!(ledger.ids().count(), 0);
    }

    #[test]
    fn a_count_falls_out_of_a_window_after_its_length_and_never_out_of_the_total() {
        let mut ledger = ProductionLedger::new();
        ledger.record_produced("paste", StatKind::Item, 1);
        let five_s = STAT_WINDOWS[0];
        // Inside the window it is still there...
        for _ in 0..(five_s.ticks - five_s.bucket_ticks()) {
            ledger.tick();
        }
        assert_eq!(ledger.produced_in("paste", 0), 1);
        // ...and within one more bucket it has rolled off the 5s ring,
        // while the longer rings and the total keep it.
        for _ in 0..five_s.bucket_ticks() {
            ledger.tick();
        }
        assert_eq!(ledger.produced_in("paste", 0), 0);
        assert_eq!(ledger.produced_in("paste", 1), 1);
        assert_eq!(ledger.produced_total("paste"), 1);
    }

    #[test]
    fn observed_grows_with_age_and_saturates_at_the_window() {
        let mut ledger = ProductionLedger::new();
        assert_eq!(ledger.observed(0), 0);
        for _ in 0..100 {
            ledger.tick();
        }
        assert_eq!(ledger.observed(0), 100);
        assert_eq!(ledger.observed(1), 100);
        for _ in 0..100 {
            ledger.tick();
        }
        assert_eq!(ledger.observed(0), STAT_WINDOWS[0].ticks);
        assert_eq!(ledger.observed(1), 200);
        assert_eq!(ledger.age(), 200);
    }

    #[test]
    fn the_snapshot_sums_rings_in_window_order() {
        let mut ledger = ledger_with_history();
        ledger.record_consumed("vinegar", StatKind::Fluid, 2);
        let snap = ledger.snapshot(no_cycle());
        assert_eq!(snap.windows.len(), STAT_WINDOW_COUNT);
        assert_eq!(snap.windows[0].id, "5s");
        assert_eq!(snap.windows[1].ticks, 1_800);
        let ids: Vec<&str> = snap.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["paste", "vinegar"]);
        let paste = &snap.items[0];
        assert_eq!(paste.kind, StatKind::Item);
        assert_eq!(paste.produced, vec![1, 2, 2, 2]);
        assert_eq!(paste.produced_total, 2);
        let vinegar = &snap.items[1];
        assert_eq!(vinegar.kind, StatKind::Fluid);
        assert_eq!(vinegar.consumed, vec![2, 2, 2, 2]);
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"kind\":\"fluid\""), "{json}");
        assert!(json.contains("\"windows\":[{\"id\":\"5s\""), "{json}");
    }

    /// One paste made, a 5s window ago; one made now.
    fn ledger_with_history() -> ProductionLedger {
        let mut ledger = ProductionLedger::new();
        ledger.record_produced("paste", StatKind::Item, 1);
        for _ in 0..STAT_WINDOWS[0].ticks {
            ledger.tick();
        }
        ledger.record_produced("paste", StatKind::Item, 1);
        ledger
    }

    #[test]
    fn restore_refuses_a_cursor_or_clock_the_tick_path_cannot_produce() {
        let ledger = ledger_with_history();
        let items = ledger.items().clone();
        assert!(
            ProductionLedger::restore(
                items.clone(),
                ledger.cursor(),
                ledger.ticks_into_bucket(),
                ledger.age(),
                ledger.cycles_closed()
            )
            .is_some()
        );
        let mut bad_cursor = ledger.cursor();
        bad_cursor[0] = STAT_BUCKETS;
        assert!(
            ProductionLedger::restore(items.clone(), bad_cursor, ledger.ticks_into_bucket(), 0, 0)
                .is_none()
        );
        let mut bad_clock = ledger.ticks_into_bucket();
        bad_clock[1] = STAT_WINDOWS[1].bucket_ticks();
        assert!(ProductionLedger::restore(items, ledger.cursor(), bad_clock, 0, 0).is_none());
    }

    #[test]
    fn a_restored_ledger_is_the_saved_one() {
        let mut ledger = ledger_with_history();
        ledger.close_cycle();
        ledger.record_produced("paste", StatKind::Item, 4);
        let restored = ProductionLedger::restore(
            ledger.items().clone(),
            ledger.cursor(),
            ledger.ticks_into_bucket(),
            ledger.age(),
            ledger.cycles_closed(),
        )
        .unwrap();
        assert_eq!(restored, ledger);
    }

    fn no_cycle() -> CycleSnapshot {
        CycleSnapshot {
            period_ticks: 1_000,
            next_tick: 1_000,
            cycles_closed: 0,
            lines: Vec::new(),
        }
    }

    #[test]
    fn closing_a_cycle_moves_this_cycle_to_last_and_starts_again() {
        let mut ledger = ProductionLedger::new();
        ledger.record_produced("boxed_ketchup", StatKind::Item, 3);
        ledger.record_consumed("boxed_ketchup", StatKind::Item, 9);
        assert_eq!(ledger.produced_this_cycle("boxed_ketchup"), 3);
        assert_eq!(ledger.produced_last_cycle("boxed_ketchup"), 0);

        ledger.close_cycle();
        assert_eq!(ledger.cycles_closed(), 1);
        assert_eq!(ledger.produced_this_cycle("boxed_ketchup"), 0);
        assert_eq!(ledger.produced_last_cycle("boxed_ketchup"), 3);

        ledger.record_produced("boxed_ketchup", StatKind::Item, 5);
        assert_eq!(ledger.produced_this_cycle("boxed_ketchup"), 5);
        ledger.close_cycle();
        assert_eq!(ledger.produced_last_cycle("boxed_ketchup"), 5);
        // An idle cycle is a zero, not a carry-over of the one before.
        ledger.close_cycle();
        assert_eq!(ledger.produced_last_cycle("boxed_ketchup"), 0);
        // Consumption is not a cycle count, and the rolling windows and
        // lifetime total never see a payday.
        assert_eq!(ledger.produced_total("boxed_ketchup"), 8);
        assert_eq!(ledger.produced_in("boxed_ketchup", 0), 8);
    }

    #[test]
    fn a_cycle_line_is_short_only_after_a_finished_cycle_made_too_little() {
        let mut ledger = ProductionLedger::new();
        let demand = vec![
            ("boxed_ketchup".to_string(), I48F16::from_num(6)),
            (
                "boxed_bbq".to_string(),
                I48F16::from_num(2) / I48F16::from_num(3),
            ),
        ];
        ledger.record_produced("boxed_ketchup", StatKind::Item, 2);
        // No cycle has finished, so nothing can be short yet.
        let lines = ledger.cycle_lines(&demand);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].id, "boxed_ketchup", "demand order, not id order");
        assert_eq!(lines[0].produced_this_cycle, 2);
        assert!(!lines[0].short && !lines[1].short);

        ledger.record_produced("boxed_bbq", StatKind::Item, 1);
        ledger.close_cycle();
        let lines = ledger.cycle_lines(&demand);
        assert!(lines[0].short, "2 made of 6 required");
        assert!(!lines[1].short, "1 made covers two-thirds");
        assert!((lines[1].required - 2.0 / 3.0).abs() < 1e-4);
        assert!(ledger.cycle_lines(&[]).is_empty());
    }
}
