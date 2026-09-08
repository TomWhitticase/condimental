//! Economy MVP (ARCHITECTURE.md §9 milestone 5; GAMEPLAY.md §1's core loop,
//! §6's contracts-as-demand-system, §9's implied data/systems).
//!
//! Deliberately minimal, per the milestone description: a single hardcoded
//! [`Contract`] (fixed sauce mix, fixed cadence, fixed payout) and a
//! fixed-point `money` counter — **not** GAMEPLAY.md §6/§9's full
//! `contracts.ron`/multi-contract/staff-pool design. That generalization
//! (data-file-loaded contracts, several concurrent contracts, reputation/
//! contract-loss on a miss) is explicitly future work; this module only
//! needs to prove "build factory -> deliver -> get paid" end to end.
//!
//! **This resolves the open architectural-home question both
//! ARCHITECTURE.md §11 and GAMEPLAY.md §9 flagged** ("money and in-flight
//! contract state don't have a designed ECS shape yet"): the answer turned
//! out to be a plain `Economy` struct (not a new ECS component) owned
//! directly by `World` and advanced once per tick alongside every other
//! system, the same way `World` owns its `stages` today. A full ECS
//! component wasn't warranted because there is exactly one of each thing
//! (one money counter, one active contract) at this milestone's scope —
//! see `World::tick`'s call to `Economy::tick` and `World::economy_snapshot`
//! for how this plugs into the rest of the sim.

use super::inventory::Inventory;
use super::types::{Fx, ItemId};

/// Which finished item the hardcoded contract wants — `data/recipes.ron`'s
/// final-stage output at milestone 4's scope. Named so a future
/// `contracts.ron` migration has one obvious place to read this milestone's
/// values from, instead of a magic string buried in `Contract::hardcoded`.
pub const CONTRACT_ITEM: &str = "boxed_ketchup";
/// How many units of `CONTRACT_ITEM` must be delivered per cadence window.
pub const CONTRACT_QUANTITY: u32 = 2;
/// Delivery cadence, in sim ticks (ARCHITECTURE.md §3.9: tick-counted, never
/// wall-clock) — GAMEPLAY.md §6's "recurring delivery cadence."
pub const CONTRACT_CADENCE_TICKS: u64 = 1_000;
/// Payout per successful delivery, in whole money units. Kept as a plain
/// integer here (not a `Fx` constant) purely because `Fx::from_num` isn't a
/// `const fn` in the `fixed` crate version this project pins — the actual
/// fixed-point value is constructed once in `Contract::hardcoded`.
pub const CONTRACT_PAYOUT_MONEY: u32 = 50;

/// A standing agreement: a fixed item mix, delivered on a recurring cadence,
/// for a fixed payout per successful delivery (GAMEPLAY.md §6). Milestone 5
/// hardcodes exactly one of these directly in Rust, per the milestone's own
/// scope note ("doesn't need the full contracts.ron ... design yet") — a
/// later milestone generalizes this into data loaded from `contracts.ron`
/// plus several concurrently-active contracts (GAMEPLAY.md §9).
#[derive(Debug, Clone)]
pub struct Contract {
    /// (item id, quantity) pairs required per delivery — GAMEPLAY.md §6's
    /// "mix," collapsed to a single item at this milestone's scope since
    /// `data/recipes.ron`'s chain only has one sellable final product so far.
    pub mix: Vec<(ItemId, u32)>,
    pub cadence_ticks: u64,
    pub payout: Fx,
}

impl Contract {
    fn hardcoded() -> Self {
        Self {
            mix: vec![(CONTRACT_ITEM.to_string(), CONTRACT_QUANTITY)],
            cadence_ticks: CONTRACT_CADENCE_TICKS,
            payout: Fx::from_num(CONTRACT_PAYOUT_MONEY),
        }
    }
}

/// Money plus the single active contract's delivery state. Owned directly by
/// `World` (see this module's doc for why no new ECS component was needed)
/// and advanced once per tick via [`Economy::tick`].
#[derive(Debug)]
pub struct Economy {
    /// Fixed-point money counter (ARCHITECTURE.md §3.9 — shared, persisted
    /// simulation state, so never a float).
    pub money: Fx,
    pub contract: Contract,
    /// Delivered product waiting to be checked against the contract at the
    /// next cadence tick — GAMEPLAY.md §6's "accumulating output buffer."
    /// Filled by `World::issue_command(PlayerCommand::CollectOutput)` (the
    /// chain's existing exit point) rather than a separate delivery
    /// building/entity, per this milestone's minimal scope — a real
    /// delivery-dock tile is future polish, not required to prove the loop.
    pub delivery_buffer: Inventory,
    /// Absolute tick at which the contract's mix is next checked.
    pub next_delivery_tick: u64,
}

/// A presentation-facing snapshot of economy state (ARCHITECTURE.md §4/§5) —
/// what Godot needs to render a money counter and basic contract status,
/// never the internal `Inventory` representation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EconomySnapshot {
    /// `f64` is fine here — same rule as `BeltSnapshot`'s item positions:
    /// this has already crossed from fixed-point sim state into a
    /// presentation-only value that never feeds back in (ARCHITECTURE.md
    /// §3.9).
    pub money: f64,
    pub contract_mix: Vec<(ItemId, u32)>,
    pub contract_progress: Vec<(ItemId, u32)>,
    pub cadence_ticks: u64,
    pub next_delivery_tick: u64,
}

impl Economy {
    pub fn new() -> Self {
        let contract = Contract::hardcoded();
        Self {
            money: Fx::from_num(0),
            next_delivery_tick: contract.cadence_ticks,
            // Unbounded per-item capacity: this is an accumulating ledger of
            // "delivered, awaiting the next contract check," not a
            // physical-space-limited buffer like a machine's input/output.
            delivery_buffer: Inventory::new(u32::MAX),
            contract,
        }
    }

    /// Adds finished product into the delivery buffer — called from
    /// `World::issue_command`'s `CollectOutput` handler.
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

    /// Checks the contract at its cadence boundary. Once `current_tick`
    /// reaches `next_delivery_tick`, tests whether the delivery buffer holds
    /// the full required mix:
    ///
    /// - **Satisfied:** consumes the mix from the buffer, pays out, advances
    ///   `next_delivery_tick` by the cadence, returns `true`.
    /// - **Not satisfied:** a missed delivery — no money changes hands, but
    ///   the cadence still advances rather than re-checking every tick
    ///   forever (contract loss/reputation penalties for a miss are
    ///   GAMEPLAY.md §11's explicitly deferred open question, not this
    ///   milestone's scope). Returns `false`.
    ///
    /// Returns `false` without touching `next_delivery_tick` if the cadence
    /// boundary hasn't arrived yet.
    pub fn tick(&mut self, current_tick: u64) -> bool {
        if current_tick < self.next_delivery_tick {
            return false;
        }
        self.next_delivery_tick += self.contract.cadence_ticks;

        let satisfied = self
            .contract
            .mix
            .iter()
            .all(|(item, qty)| self.delivery_buffer.has_at_least(item, *qty));
        if !satisfied {
            return false;
        }

        for (item, qty) in &self.contract.mix {
            self.delivery_buffer.remove(item, *qty);
        }
        self.money += self.contract.payout;
        true
    }

    pub fn snapshot(&self) -> EconomySnapshot {
        EconomySnapshot {
            money: self.money.to_num::<f64>(),
            contract_mix: self.contract.mix.clone(),
            contract_progress: self
                .contract
                .mix
                .iter()
                .map(|(item, _)| (item.clone(), self.delivery_buffer.count(item)))
                .collect(),
            cadence_ticks: self.contract.cadence_ticks,
            next_delivery_tick: self.next_delivery_tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_pay_out_before_the_cadence_boundary_arrives() {
        let mut econ = Economy::new();
        econ.deposit(CONTRACT_ITEM, CONTRACT_QUANTITY);
        // One tick short of the boundary: mix is fully stocked, but the
        // cadence hasn't elapsed yet, so nothing should happen.
        let paid = econ.tick(econ.next_delivery_tick - 1);
        assert!(!paid);
        assert_eq!(econ.money, Fx::from_num(0));
    }

    #[test]
    fn pays_out_exactly_the_contract_payout_when_mix_is_satisfied_at_the_boundary() {
        let mut econ = Economy::new();
        econ.deposit(CONTRACT_ITEM, CONTRACT_QUANTITY);
        let boundary = econ.next_delivery_tick;

        let paid = econ.tick(boundary);

        assert!(paid);
        assert_eq!(econ.money, Fx::from_num(CONTRACT_PAYOUT_MONEY));
        assert_eq!(econ.delivery_buffer.count(CONTRACT_ITEM), 0, "the mix must be consumed on payout");
        assert_eq!(econ.next_delivery_tick, boundary + CONTRACT_CADENCE_TICKS, "cadence must advance");
    }

    #[test]
    fn does_not_pay_out_when_the_delivered_mix_falls_short_at_the_boundary() {
        let mut econ = Economy::new();
        econ.deposit(CONTRACT_ITEM, CONTRACT_QUANTITY - 1); // short by one unit
        let boundary = econ.next_delivery_tick;

        let paid = econ.tick(boundary);

        assert!(!paid);
        assert_eq!(econ.money, Fx::from_num(0));
        // A miss doesn't destroy what was delivered — it just wasn't enough
        // yet; it should still count toward the next cadence window.
        assert_eq!(econ.delivery_buffer.count(CONTRACT_ITEM), CONTRACT_QUANTITY - 1);
        assert_eq!(econ.next_delivery_tick, boundary + CONTRACT_CADENCE_TICKS, "cadence must still advance on a miss");
    }

    #[test]
    fn a_missed_delivery_can_still_pay_out_once_enough_accumulates_by_a_later_boundary() {
        let mut econ = Economy::new();
        econ.deposit(CONTRACT_ITEM, CONTRACT_QUANTITY - 1);
        let first_boundary = econ.next_delivery_tick;
        assert!(!econ.tick(first_boundary));

        econ.deposit(CONTRACT_ITEM, 1); // the missing unit arrives later
        let second_boundary = econ.next_delivery_tick;
        assert!(econ.tick(second_boundary));
        assert_eq!(econ.money, Fx::from_num(CONTRACT_PAYOUT_MONEY));
    }
}
