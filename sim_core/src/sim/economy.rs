//! Money and the in-flight state of every active contract (GAMEPLAY.md §1's
//! core loop, §5's economy, §6's contracts-as-demand-system).
//!
//! **This resolves the open architectural-home question both
//! ARCHITECTURE.md §11 and GAMEPLAY.md §9 flagged** ("money and in-flight
//! contract state don't have a designed ECS shape yet"): the answer turned
//! out to be a plain [`Economy`] struct (not a new ECS component) owned
//! directly by `World` and advanced once per tick alongside every other
//! system, the same way `World` owns its shop and its topology. Several concurrent
//! contracts did not change that call: they are a `Vec` inside the one
//! `Economy`, not a `Vec` of entities — nothing about a contract has a
//! position, a footprint, or a component-wise update, which is what ECS
//! machinery would buy.
//!
//! # What changed from the Economy MVP
//!
//! Milestone 5 shipped exactly ONE contract, hardcoded in Rust from four
//! named constants, with its own delivery buffer. Its own scope note said
//! the generalization was deferred; this module is that generalization.
//! Contracts are now content ([`super::contracts`], loaded from
//! `data/contracts.ron`) and several run at once, each with its own mix,
//! its own cadence, and its own independently-tracked next-delivery tick.
//! That is what makes GAMEPLAY.md §6's central design pressure exist at
//! all: with one contract there is nothing to diversify *between*.
//!
//! # The delivery-allocation rule (decided here — GAMEPLAY.md left it open)
//!
//! **There is ONE shared pool of collected-but-undelivered stock. At its
//! own cadence boundary a contract is satisfied only if the pool holds its
//! full mix, and on payout it CONSUMES that mix from the pool. Goods are
//! never credited to two contracts at once.**
//!
//! The alternative considered — give every contract its own buffer and
//! credit one delivered unit to every contract that wants that item — is
//! simpler to write and wrong for this game. It duplicates goods: adding a
//! second contract for an item the factory already makes would double
//! income at zero production cost, so the optimal play becomes "stack as
//! many contracts as possible onto one product line," which is the exact
//! strategy GAMEPLAY.md §6 rejects a spot market in order to prevent. One
//! physical box of ketchup can be shipped to one customer; modelling that
//! is what makes throughput scarce, and scarce throughput is what makes a
//! second production line worth building.
//!
//! Two consequences fall out of that rule and are worth naming:
//!
//! 1. **Contracts compete, so ties need an order.** When several fall due
//!    on the same tick and stock is short, they are filled in
//!    `contracts.ron` file order — a deterministic tiebreak
//!    (ARCHITECTURE.md §3.9), documented in that file too. A player-set
//!    priority is the natural upgrade, and is the same open question
//!    GAMEPLAY.md §11 already has for staff allocation; there is no
//!    priority UI yet, so file order is the honest placeholder rather than
//!    an invented answer.
//! 2. **Undelivered stock is never destroyed.** A missed window leaves the
//!    pool untouched, so what was delivered still counts toward the next
//!    window (of that contract or any other). A miss costs that window's
//!    payout and nothing more — contract loss and reputation penalties are
//!    GAMEPLAY.md §11's explicitly open question, not decided here.

use super::contracts::{Contract, ContractRegistry};
use super::inventory::Inventory;
use super::types::{Fx, ItemId};

/// What a new game starts with in the bank (the "Real Placement" epic's
/// ticket 1).
///
/// This constant only became necessary when `World::new` stopped building a
/// factory: before it, the world arrived complete and free, and starting
/// money could honestly be zero because there was nothing to spend it on.
/// Now the world starts **empty** and everything in it is something the
/// player bought and placed, so a fresh world needs enough to make a first
/// move with.
///
/// **200, and it is a placeholder** — GAMEPLAY.md §10 puts exact numeric
/// balance ("ingredient costs, contract payouts, equipment prices") in a
/// later pass, and `data/shop.ron`'s own header says the same about the
/// prices this is measured against. What the number *does* encode, against
/// today's catalog, is the shape GAMEPLAY.md §8's walkthrough describes —
/// "start with a basic cooker and one small contract", not with a factory:
///
/// - It buys a hopper (40), a crusher (75) and seventeen belt tiles (85):
///   a raw feed, one processing machine and a line to run between them.
/// - It does **not** buy the reference chain, which costs a little over
///   1000 all told — so the shipped six-stage line is something a player
///   works up to through contract deliveries, which is the loop GAMEPLAY.md
///   §1 describes.
/// - It is a bit over two payouts of the shipped board's middle contract
///   (50), so the first expansion is a handful of deliveries away rather
///   than a grind.
///
/// It lives here, beside the contract bookkeeping, because this is the file
/// that owns economic state — the same reason the Economy MVP's contract
/// constants lived here before contracts became data. It is applied by
/// `World`'s constructors (through [`Economy::with_money`]), not by
/// [`Economy::new`], which starts an empty ledger at zero.
pub const STARTING_MONEY: u32 = 200;

/// One contract plus the bookkeeping that is *this world's* rather than the
/// content's: when it is next checked, and how it has done so far.
///
/// The split matters — [`Contract`] is loaded from `data/contracts.ron` and
/// is immutable content; everything else here is live simulation state.
#[derive(Debug, Clone)]
pub struct ActiveContract {
    pub contract: Contract,
    /// Absolute tick at which this contract's mix is next checked. Each
    /// contract advances its own, independently of every other — that is
    /// what lets one fall due while another has 900 ticks still to run.
    pub next_delivery_tick: u64,
    /// How many cadence windows this contract has been paid for.
    pub deliveries_made: u32,
    /// How many cadence windows passed with the mix unsatisfied. Counted
    /// but not yet acted on: GAMEPLAY.md §11 has not decided what a miss
    /// costs beyond the payout, so this is the number that mechanic will
    /// read when it lands, and presentation can already show it.
    pub deliveries_missed: u32,
}

impl ActiveContract {
    fn new(contract: Contract) -> Self {
        Self {
            next_delivery_tick: contract.cadence_ticks,
            contract,
            deliveries_made: 0,
            deliveries_missed: 0,
        }
    }
}

/// Money plus every active contract's delivery state. Owned directly by
/// `World` (see this module's doc for why no new ECS component was needed)
/// and advanced once per tick via [`Economy::tick`].
#[derive(Debug)]
pub struct Economy {
    /// Fixed-point money counter (ARCHITECTURE.md §3.9 — shared, persisted
    /// simulation state, so never a float).
    pub money: Fx,
    /// Every contract currently running, in `contracts.ron` file order,
    /// which is also the priority order under scarcity (module doc).
    pub contracts: Vec<ActiveContract>,
    /// The ONE shared pool of delivered-but-not-yet-shipped product that
    /// every contract draws from (module doc's allocation rule) — the
    /// number the UI calls "on the dock", and since the delivery-dock ticket
    /// that is literally true: it is filled by [`super::dock::Dock`], the
    /// building at the chain's tail, which ships whatever arrives on its
    /// belt into here every tick. (Until that ticket it was filled by
    /// `World::issue_command(PlayerCommand::CollectOutput)`, a keypress that
    /// reached into the last machine's own output buffer; that command is
    /// gone. This field's meaning and shape did not change — only who fills
    /// it.)
    pub delivery_buffer: Inventory,
}

/// A presentation-facing snapshot of economy state (ARCHITECTURE.md §4/§5) —
/// what Godot needs to render a money counter and a contract clipboard per
/// active contract, never the internal `Inventory` representation.
///
/// **Shape changed when contracts became data.** It used to carry one
/// contract inline (`contract_mix` / `contract_progress` / `cadence_ticks` /
/// `next_delivery_tick` as top-level fields). Those are gone; per-contract
/// fields now live in [`ContractSnapshot`] inside the `contracts` array.
/// `money` is unchanged and is still a single value.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EconomySnapshot {
    /// `f64` is fine here — same rule as `BeltSnapshot`'s item positions:
    /// this has already crossed from fixed-point sim state into a
    /// presentation-only value that never feeds back in (ARCHITECTURE.md
    /// §3.9).
    pub money: f64,
    /// The whole shared delivery pool, `(item id, quantity)`, non-zero
    /// entries only. This is the raw stock on hand — the number a UI should
    /// show for "what's on the dock," as opposed to a contract's own
    /// clamped `progress`.
    pub delivery_stock: Vec<(ItemId, u32)>,
    /// Every active contract, in priority order.
    pub contracts: Vec<ContractSnapshot>,
}

/// One active contract, as presentation sees it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractSnapshot {
    /// Stable content id — the right key for a UI to track a card by.
    pub id: String,
    /// Player-facing title from `contracts.ron`.
    pub name: String,
    /// What this contract requires per delivery, `(item id, quantity)`.
    pub mix: Vec<(ItemId, u32)>,
    /// How much of each mix line the shared pool can currently cover,
    /// **clamped to the required quantity** — so a checklist reads "2/2"
    /// rather than "7/2" when stock exceeds the ask. Same item order as
    /// `mix`, so the two zip positionally.
    pub progress: Vec<(ItemId, u32)>,
    pub cadence_ticks: u64,
    pub next_delivery_tick: u64,
    /// Payout per satisfied delivery. `f64` for the same presentation-only
    /// reason as `money`.
    pub payout: f64,
    pub deliveries_made: u32,
    pub deliveries_missed: u32,
}

impl Economy {
    /// Builds the live economy from loaded contract content. The registry's
    /// order is preserved, because it is the scarcity tiebreak.
    /// Starts at **zero** money — an `Economy` is a ledger, and an empty
    /// ledger is the honest default. A new *game* starts at
    /// [`STARTING_MONEY`], which `World` applies through
    /// [`Economy::with_money`]; keeping that decision one layer up is what
    /// lets a test build an economy in isolation without a starting balance
    /// appearing in its arithmetic.
    pub fn new(contracts: &ContractRegistry) -> Self {
        Self::with_money(contracts, Fx::ZERO)
    }

    /// [`Economy::new`], but starting from a chosen balance instead of
    /// [`STARTING_MONEY`]. The seam a test fixture uses to start rich (see
    /// [`super::reference::reference_chain`], which buys a whole factory) or
    /// deliberately broke, and where a loaded save will restore its balance.
    pub fn with_money(contracts: &ContractRegistry, money: Fx) -> Self {
        Self {
            money,
            contracts: contracts
                .ordered()
                .iter()
                .cloned()
                .map(ActiveContract::new)
                .collect(),
            // Unbounded per-item capacity: this is an accumulating ledger of
            // "delivered, awaiting a contract check," not a
            // physical-space-limited buffer like a machine's input/output.
            delivery_buffer: Inventory::new(u32::MAX),
        }
    }

    /// The shipped `data/contracts.ron`. Convenience for tests and for
    /// `World`, which has no reason to hold a registry after construction.
    pub fn bundled() -> Self {
        Self::new(&ContractRegistry::bundled())
    }

    /// Adds finished product into the shared delivery pool — called from
    /// [`super::dock::Dock::ship_into`], once per tick, for whatever the
    /// chain's delivery dock received that tick. (It was the
    /// `CollectOutput` command's handler before the delivery-dock ticket;
    /// the call is identical, it just isn't a player action any more.)
    pub fn deposit(&mut self, item: &str, qty: u32) {
        self.delivery_buffer.add(item, qty);
    }

    /// Spends `amount` if there is enough money, and reports whether it
    /// did. **All or nothing:** on `false` not a single unit has moved, so
    /// a caller can check-and-spend in one step without ever leaving a
    /// half-applied transaction behind ("Shop & Inventory" epic, ticket 1 —
    /// `World::issue_command`'s `Purchase` arm is the only caller).
    ///
    /// This is the *only* way money leaves the economy, and it lives here
    /// rather than on the shop for the reason `sim::shop`'s module doc
    /// gives: GAMEPLAY.md §1 allows exactly one currency, so there is
    /// exactly one counter, and a purchase debits the same field a contract
    /// delivery credits. A non-positive `amount` is refused rather than
    /// treated as a free purchase.
    pub fn try_spend(&mut self, amount: Fx) -> bool {
        if amount <= Fx::ZERO || self.money < amount {
            return false;
        }
        self.money -= amount;
        true
    }

    /// Advances every active contract's own cadence clock and settles any
    /// that fall due on `current_tick`. For each such contract:
    ///
    /// - **Satisfied** (the shared pool holds its full mix): consumes the
    ///   mix from the pool, pays out, advances that contract's
    ///   `next_delivery_tick` by its own cadence.
    /// - **Not satisfied:** a missed window — no money changes hands and
    ///   nothing is consumed, but the cadence still advances rather than
    ///   re-checking every tick forever.
    ///
    /// Contracts are settled in priority order (module doc), so when two
    /// fall due together and stock covers only one, the earlier-listed one
    /// takes it. Returns the indices of the contracts that were paid, in
    /// that same order — empty when nothing fell due or nothing was
    /// satisfiable.
    pub fn tick(&mut self, current_tick: u64) -> Vec<usize> {
        let mut paid = Vec::new();
        for i in 0..self.contracts.len() {
            if current_tick < self.contracts[i].next_delivery_tick {
                continue;
            }
            self.contracts[i].next_delivery_tick += self.contracts[i].contract.cadence_ticks;

            let satisfied = self.contracts[i]
                .contract
                .mix
                .iter()
                .all(|(item, qty)| self.delivery_buffer.has_at_least(item, *qty));
            if !satisfied {
                self.contracts[i].deliveries_missed += 1;
                continue;
            }

            for (item, qty) in &self.contracts[i].contract.mix {
                self.delivery_buffer.remove(item, *qty);
            }
            self.money += self.contracts[i].contract.payout;
            self.contracts[i].deliveries_made += 1;
            paid.push(i);
        }
        paid
    }

    /// The active contract with the given content id, if any — the lookup
    /// tests and future command handling ("deliver to *this* contract")
    /// want, without exposing the `Vec`'s indices as an API.
    pub fn contract(&self, id: &str) -> Option<&ActiveContract> {
        self.contracts.iter().find(|c| c.contract.id == id)
    }

    pub fn snapshot(&self) -> EconomySnapshot {
        EconomySnapshot {
            money: self.money.to_num::<f64>(),
            delivery_stock: self.delivery_buffer.iter().cloned().collect(),
            contracts: self
                .contracts
                .iter()
                .map(|active| ContractSnapshot {
                    id: active.contract.id.clone(),
                    name: active.contract.name.clone(),
                    mix: active.contract.mix.clone(),
                    progress: active
                        .contract
                        .mix
                        .iter()
                        .map(|(item, required)| {
                            (item.clone(), self.delivery_buffer.count(item).min(*required))
                        })
                        .collect(),
                    cadence_ticks: active.contract.cadence_ticks,
                    next_delivery_tick: active.next_delivery_tick,
                    payout: active.contract.payout.to_num::<f64>(),
                    deliveries_made: active.deliveries_made,
                    deliveries_missed: active.deliveries_missed,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two contracts on deliberately different cadences, both wanting the
    /// same item, plus a third wanting something the fixture never
    /// delivers. Written inline rather than read from `contracts.ron` so
    /// these unit tests keep testing `Economy`'s behaviour rather than the
    /// current balance of shipped content.
    const FIXTURE: &str = r#"(
        schema_version: 1,
        contracts: [
            Contract(id: "fast", name: "Fast", mix: [("boxed_ketchup", 2)], cadence_ticks: 100, payout: 50),
            Contract(id: "slow", name: "Slow", mix: [("boxed_ketchup", 3)], cadence_ticks: 250, payout: 90),
            Contract(id: "impossible", name: "Impossible", mix: [("chilli_mayo", 1)], cadence_ticks: 100, payout: 500),
        ],
    )"#;

    fn fixture() -> Economy {
        Economy::new(&ContractRegistry::from_ron_str(FIXTURE).expect("fixture must load"))
    }

    #[test]
    fn each_contract_starts_with_its_own_first_boundary_at_its_own_cadence() {
        let econ = fixture();
        assert_eq!(econ.contract("fast").unwrap().next_delivery_tick, 100);
        assert_eq!(econ.contract("slow").unwrap().next_delivery_tick, 250);
    }

    #[test]
    fn does_not_pay_out_before_the_cadence_boundary_arrives() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 10);
        assert!(econ.tick(99).is_empty());
        assert_eq!(econ.money, Fx::from_num(0));
    }

    #[test]
    fn a_contract_pays_out_exactly_its_own_payout_and_consumes_exactly_its_own_mix() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 5);

        // t=100: only "fast" is due ("slow" runs to 250).
        let paid = econ.tick(100);

        assert_eq!(paid, vec![0], "only the 100-tick contract should have settled");
        assert_eq!(econ.money, Fx::from_num(50));
        assert_eq!(
            econ.delivery_buffer.count("boxed_ketchup"),
            3,
            "exactly the paid contract's mix must leave the shared pool"
        );
        assert_eq!(econ.contract("fast").unwrap().next_delivery_tick, 200);
        assert_eq!(
            econ.contract("slow").unwrap().next_delivery_tick,
            250,
            "an untouched contract's own cadence must not move"
        );
        assert_eq!(econ.contract("fast").unwrap().deliveries_made, 1);
    }

    #[test]
    fn one_contract_hitting_its_boundary_while_another_has_not_pays_only_the_due_one() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 100); // plenty for anyone

        for t in [100u64, 200] {
            econ.tick(t);
        }
        assert_eq!(
            econ.money,
            Fx::from_num(100),
            "two 'fast' windows and no 'slow' window should have paid"
        );
        assert_eq!(econ.contract("slow").unwrap().deliveries_made, 0);

        econ.tick(250);
        assert_eq!(
            econ.money,
            Fx::from_num(190),
            "'slow' now falls due and adds its own, different payout"
        );
        assert_eq!(econ.contract("slow").unwrap().deliveries_made, 1);
    }

    /// The documented allocation rule, stated as a test: one delivered unit
    /// is spent, not credited twice. Both contracts are due at t=500 and
    /// stock covers only the first — the second must go unpaid.
    #[test]
    fn delivered_stock_is_consumed_not_credited_to_every_matching_contract() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 2); // enough for "fast" alone

        // Settled in one call at t=500, which is past both contracts' first
        // boundaries — so both are due on the same tick, competing for the
        // same two boxes.
        econ.tick(500);

        assert_eq!(
            econ.money,
            Fx::from_num(50),
            "only 'fast' could be filled from two boxes — 'slow' must not be paid from the \
             same stock"
        );
        assert_eq!(econ.delivery_buffer.count("boxed_ketchup"), 0);
        assert_eq!(econ.contract("fast").unwrap().deliveries_made, 1);
        assert_eq!(econ.contract("slow").unwrap().deliveries_made, 0);
        assert!(econ.contract("slow").unwrap().deliveries_missed >= 1);
    }

    /// The other half of the same rule: with enough stock for both, both
    /// are paid — so the test above is showing scarcity, not a bug where a
    /// second contract can never settle on a shared boundary.
    #[test]
    fn two_contracts_due_together_both_settle_when_stock_covers_both() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 5); // 2 for "fast" + 3 for "slow"

        econ.tick(500);

        assert_eq!(
            econ.money,
            Fx::from_num(140),
            "both contracts should have settled at the shared boundary"
        );
        assert_eq!(econ.delivery_buffer.count("boxed_ketchup"), 0);
        assert_eq!(econ.contract("slow").unwrap().deliveries_made, 1);
    }

    #[test]
    fn a_contract_that_is_never_satisfiable_never_pays_while_the_others_still_do() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 1000);

        for t in 1..=1000u64 {
            econ.tick(t);
        }

        let impossible = econ.contract("impossible").unwrap();
        assert_eq!(impossible.deliveries_made, 0);
        assert_eq!(
            impossible.deliveries_missed, 10,
            "a 100-tick contract must have missed every one of its ten windows"
        );
        assert!(
            econ.contract("fast").unwrap().deliveries_made > 0,
            "an unsatisfiable contract must not stall the ones that can be filled"
        );
    }

    #[test]
    fn a_missed_delivery_can_still_pay_out_once_enough_accumulates_by_a_later_boundary() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 1); // short by one for "fast"
        assert!(econ.tick(100).is_empty());
        assert_eq!(econ.money, Fx::from_num(0));
        assert_eq!(
            econ.delivery_buffer.count("boxed_ketchup"),
            1,
            "a miss must not destroy what was delivered"
        );
        assert_eq!(econ.contract("fast").unwrap().deliveries_missed, 1);

        econ.deposit("boxed_ketchup", 1); // the missing unit arrives later
        assert_eq!(econ.tick(200), vec![0]);
        assert_eq!(econ.money, Fx::from_num(50));
    }

    #[test]
    fn snapshot_reports_one_entry_per_contract_with_progress_clamped_to_the_ask() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 7); // more than any single mix wants

        let snap = econ.snapshot();
        assert_eq!(snap.contracts.len(), 3);
        assert_eq!(snap.delivery_stock, vec![("boxed_ketchup".to_string(), 7)]);

        let fast = &snap.contracts[0];
        assert_eq!(fast.id, "fast");
        assert_eq!(fast.name, "Fast");
        assert_eq!(fast.mix, vec![("boxed_ketchup".to_string(), 2)]);
        assert_eq!(
            fast.progress,
            vec![("boxed_ketchup".to_string(), 2)],
            "progress must be clamped to the required quantity, not the raw stock"
        );
        assert_eq!(fast.cadence_ticks, 100);
        assert_eq!(fast.next_delivery_tick, 100);
        assert_eq!(fast.payout, 50.0);

        let impossible = &snap.contracts[2];
        assert_eq!(
            impossible.progress,
            vec![("chilli_mayo".to_string(), 0)],
            "an item with no stock reads zero, and still appears in progress"
        );
    }

    #[test]
    fn the_bundled_contracts_build_a_working_economy() {
        let econ = Economy::bundled();
        assert!(
            econ.contracts.len() >= 2,
            "shipped content must have several concurrent contracts"
        );
        assert_eq!(econ.money, Fx::from_num(0));
        for active in &econ.contracts {
            assert_eq!(
                active.next_delivery_tick, active.contract.cadence_ticks,
                "each contract's first window ends one cadence in"
            );
        }
    }
}
