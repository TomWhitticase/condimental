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
//!    payout — and, since the contract-board epic, a **strike**.
//!
//! # Signing and losing contracts (the contract-board epic, 2026-09-10)
//!
//! This answers the question GAMEPLAY.md §11 had left open ("what exactly
//! happens when a delivery interval is missed"), and it answers it the
//! Factorio way rather than the reputation-meter way: **a contract is a
//! thing you buy, and a thing you can lose.**
//!
//! - `data/contracts.ron` is a **board of offers**. A new game activates
//!   only the offers flagged `starter`; everything else is signed by the
//!   player through [`Economy::try_sign`] (the `SignContract` command) for
//!   the offer's `price`, out of the same money counter everything else
//!   debits. **One signature per offer**: signing an active contract is
//!   refused, so a player can never hold two of one contract and double a
//!   line's income for free — the same "goods are never credited twice"
//!   principle as the allocation rule above, applied to the contracts
//!   themselves.
//! - A contract that misses `strikes` windows **in a row** is **lost**: it
//!   leaves the active list at that settle, its counters are forgotten,
//!   and the board offers it again at full price. A paid window resets
//!   the run of misses to zero, so a factory that recovers is forgiven;
//!   only a factory that keeps failing loses the customer. Consecutive
//!   rather than cumulative because a cumulative count is a fuse that can
//!   only ever burn down, which turns every long game into a certain loss.
//! - A signed contract takes its **file-order position** in the active
//!   list, whatever order it was signed in, so the scarcity tiebreak above
//!   is still `contracts.ron`'s and never depends on player history.
//! - Losing is the whole penalty. There is no fine on top: the fee to get
//!   the contract back IS the fine, and it is a number already printed on
//!   the board rather than a second one to explain. A reputation score
//!   gating higher tiers stays open (GAMEPLAY.md §11).
//!
//! The economy owns the board (a clone of the registry) because signing
//! needs the offer's terms at command time and nothing else in `World`
//! holds a registry after construction.

use std::collections::BTreeMap;

use super::contracts::{Contract, ContractRegistry};
use super::inventory::Inventory;
use super::types::{ItemId, Money};

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
/// **3000, and it is still a placeholder** — GAMEPLAY.md §10 puts exact
/// numeric balance ("ingredient costs, contract payouts, equipment prices")
/// in a later pass, and `data/shop.ron`'s own header says the same about the
/// prices this is measured against. What the number *does* encode, against
/// today's catalog, is that **the core loop closes from a cold start**:
///
/// - The shipped six-stage line — [`super::reference::reference_chain`],
///   which prices its own bill out of `data/shop.ron` rather than
///   hardcoding it — costs exactly **1087**. A new player can therefore
///   build the whole thing outright if that is what they want to do, and
///   still have about **1900** left for the mistakes, the second line and
///   the belt they laid the wrong way round.
/// - That is deliberately **more than 2.5x** the bill rather than exactly
///   it. A balance that only just covers the line is a puzzle with one
///   solution; the point of the margin is that a first factory can be built
///   wrong and rebuilt.
///
/// **Why it is not 200 any more.** 200 was this constant's first value, and
/// it was chosen to be "a first move, not a factory" — a hopper (40), a
/// crusher (75) and seventeen belt tiles (85). Against a 1087 bill and a
/// chain whose first *sellable* product is six machines downstream, that
/// turned out not to be a slow start but a dead one: money only ever arrives
/// through contract deliveries, deliveries need the line, and the line could
/// not be reached from 200 by any sequence of commands. GAMEPLAY.md §9
/// recorded exactly that as the balancing pass's first job. This is that
/// fix, taking the option that adds a real decision — the player *chooses*
/// what to build first — rather than the option that only removes one.
///
/// The two numbers are balanced against each other, so changing prices in
/// `data/shop.ron` means revisiting this constant; `tests/shop.rs` pins the
/// relationship (starting money against the chain's *dynamically priced*
/// bill) rather than the pair of literals, so a price change moves the
/// test's expectations with it instead of breaking them.
///
/// It lives here, beside the contract bookkeeping, because this is the file
/// that owns economic state — the same reason the Economy MVP's contract
/// constants lived here before contracts became data. It is applied by
/// `World`'s constructors (through [`Economy::with_money`]), not by
/// [`Economy::new`], which starts an empty ledger at zero.
pub const STARTING_MONEY: u32 = 3000;

/// How many units of any ONE item may stand on the dock at once — the
/// ceiling on the shared delivery pool, per item id.
///
/// **Why the pool has a ceiling at all** (the dock-capacity change,
/// 2026-09-16). Until now this was `u32::MAX` and its comment called the
/// pool "an accumulating ledger, not a physical-space-limited buffer". That
/// was true of the ledger and false of the fiction: what the player sees is
/// a loading dock with cases standing on it, and a loading dock fills up.
/// With no ceiling a line built far past the board's demand simply grew a
/// number forever — overproduction cost ingredients (`contracts.ron`'s own
/// closing note) but never showed up on the floor, so the one place the
/// player would actually notice it, the factory itself, stayed silent. A
/// full pool now refuses the box at the dock's belt end
/// ([`super::dock::Dock::try_absorb`]), the line backs up behind it exactly
/// the way a full machine buffer already backs one up, and a jam that says
/// "you are making more than anyone ordered" is visible from across the
/// floor. It also gives `delivery_stock` a real denominator, which is what
/// lets presentation draw it as a bar at all (UI_UX.md §5e: a bar is only
/// drawn where the simulation defines the ceiling).
///
/// **Why it is per item and not one total.** A single total can deadlock
/// the game: one overproduced product would crowd every other item out of
/// the pool, and a contract whose mix names an item that can no longer be
/// delivered could never be satisfied again — and nothing in the game can
/// dump excess stock (GAMEPLAY.md §11's spot-market question is still
/// open), so there would be no way out. A per-item ceiling cannot do that:
/// every item always has its own room, so a mix is always eventually
/// fillable, and a clog is always specific ("the dock is full of boxed
/// ketchup") rather than a mystery.
///
/// **The number is a relationship, not a literal** (the same rule
/// [`STARTING_MONEY`] follows): it must comfortably exceed the LARGEST
/// single mix quantity on the board, because a contract settles only when
/// the pool holds its whole mix at once — `export_pallet` asks for 20 boxes
/// in one window, so a ceiling of 20 or under would make it unsatisfiable.
/// 48 is 2.4x that: the pallet order can be building up while the small
/// orders keep being skimmed off the same pool. `tests/dock.rs` pins the
/// relationship against `data/contracts.ron` rather than the literal, so
/// adding a bigger order fails a test instead of quietly breaking that
/// contract. Left of that ceiling, a line running against no demand at all
/// reaches it in about 12 000 ticks (~6.5 minutes) — long enough not to
/// nag, short enough to be a consequence.
///
/// A Rust constant rather than a `data/` field for
/// [`super::dock::DOCK_CAPACITY`]'s reason, and to be moved when buffer
/// sizes become content.
pub const DELIVERY_STOCK_CAPACITY: u32 = 48;

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
    /// How many cadence windows passed with the mix unsatisfied, over the
    /// life of this signature.
    pub deliveries_missed: u32,
    /// Missed windows **in a row**. Zeroed by a paid window; reaching
    /// `contract.strikes` loses the contract (module doc). This, not
    /// `deliveries_missed`, is what "at risk" means.
    pub consecutive_misses: u32,
}

impl ActiveContract {
    /// A fresh signature at `tick`: first window ends one cadence later,
    /// every counter at zero. A re-signed contract is a new agreement and
    /// remembers nothing of the one that was lost.
    pub fn signed_at(contract: Contract, tick: u64) -> Self {
        Self {
            next_delivery_tick: tick + contract.cadence_ticks,
            contract,
            deliveries_made: 0,
            deliveries_missed: 0,
            consecutive_misses: 0,
        }
    }

    /// The tick this contract's current window began — the other end of
    /// the countdown, so a gauge can show elapsed as a fraction.
    pub fn window_start_tick(&self) -> u64 {
        self.next_delivery_tick
            .saturating_sub(self.contract.cadence_ticks)
    }
}

/// What settling did to one contract on one tick — the return of
/// [`Economy::tick`], so a caller (or a test) never has to diff the active
/// list by index to learn that something was paid or dropped. `Lost` always
/// follows the `Missed` that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractEvent {
    Paid { id: String },
    Missed { id: String, consecutive_misses: u32 },
    Lost { id: String },
}

/// Why [`Economy::try_sign`] did nothing. The command path drops this on
/// the floor (every refused command is a silent no-op), but the reason is
/// worth having for tests and for any UI that wants to say it in words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignRefusal {
    /// No such id on the board.
    Unknown,
    /// Already active — one signature per offer.
    AlreadySigned,
    /// The fee is more than the balance; nothing moved.
    CannotAfford,
}

/// Money plus every active contract's delivery state. Owned directly by
/// `World` (see this module's doc for why no new ECS component was needed)
/// and advanced once per tick via [`Economy::tick`].
#[derive(Debug)]
pub struct Economy {
    /// Fixed-point money counter (ARCHITECTURE.md §3.9 — shared, persisted
    /// simulation state, so never a float).
    pub money: Money,
    /// The whole board of offers, in `contracts.ron` order (module doc:
    /// "Signing and losing contracts"). Content, so read-only.
    board: ContractRegistry,
    /// Every contract currently running, in `contracts.ron` file order,
    /// which is also the priority order under scarcity (module doc). A
    /// subset of `board`; never holds one id twice.
    pub contracts: Vec<ActiveContract>,
    /// The ONE shared pool of delivered-but-not-yet-shipped product that
    /// every contract draws from (module doc's allocation rule) — the
    /// number the UI calls "on the dock", and since the delivery-dock ticket
    /// that is literally true: it is filled by [`super::dock::Dock`], the
    /// building at the chain's tail, which ships whatever arrives on its
    /// belt into here every tick.
    pub delivery_buffer: Inventory,
    /// How many times each offer has been lost, by id — absent means never.
    /// A `BTreeMap` so iteration (and the save) is deterministic. Kept so
    /// the board can say "lost twice" beside an offer; nothing in the
    /// settle logic reads it.
    pub losses: BTreeMap<String, u32>,
}

/// A presentation-facing snapshot of economy state (ARCHITECTURE.md §4/§5) —
/// what Godot needs to render a money counter, a contract clipboard per
/// active contract and the board of offers, never the internal `Inventory`
/// representation.
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
    /// The ceiling each of those entries stands against
    /// ([`DELIVERY_STOCK_CAPACITY`]) — on the wire so presentation can draw
    /// the level as a bar without knowing the constant, and so a change to
    /// it never needs a matching edit in GDScript.
    pub delivery_capacity: u32,
    /// Whether any item has reached that ceiling, i.e. whether the dock is
    /// refusing boxes and the line behind it is backing up
    /// ([`Economy::delivery_pool_full`]).
    pub delivery_full: bool,
    /// Every **active** contract, in priority order.
    pub contracts: Vec<ContractSnapshot>,
    /// Every offer on the board, in file order, active or not — what the
    /// contracts board menu is built from, the way `shop_snapshot_json`'s
    /// `entries` build the build menu.
    pub board: Vec<ContractOfferSnapshot>,
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
    /// Whether the pool holds the whole mix right now — i.e. whether the
    /// window would pay if it ended this tick. Derived here rather than in
    /// Godot so the two can never disagree about "all-or-nothing".
    pub satisfied: bool,
    pub cadence_ticks: u64,
    /// The tick the current window began; with `next_delivery_tick` this
    /// is the whole countdown, both ends.
    pub window_start_tick: u64,
    pub next_delivery_tick: u64,
    /// Payout per satisfied delivery. `f64` for the same presentation-only
    /// reason as `money`.
    pub payout: f64,
    /// What it cost to sign — and will cost again if it is lost.
    pub price: f64,
    pub deliveries_made: u32,
    pub deliveries_missed: u32,
    /// Missed windows in a row; the contract is lost when this reaches
    /// `strikes`.
    pub consecutive_misses: u32,
    pub strikes: u32,
}

/// One offer on the board, active or not.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractOfferSnapshot {
    pub id: String,
    pub name: String,
    pub mix: Vec<(ItemId, u32)>,
    pub cadence_ticks: u64,
    pub payout: f64,
    pub price: f64,
    pub strikes: u32,
    pub starter: bool,
    /// Currently signed. An active offer cannot be signed again.
    pub active: bool,
    /// Whether the fee is within the current balance — the same "one is
    /// buyable right now" flag `shop_snapshot_json` gives each kind.
    pub affordable: bool,
    /// How many times this offer has been lost and gone back on the board.
    pub times_lost: u32,
}

impl Economy {
    /// Builds the live economy from a loaded contract board: the board's
    /// `starter` contracts are active from tick 0, everything else waits to
    /// be signed. Starts at **zero** money — an `Economy` is a ledger, and
    /// an empty ledger is the honest default. A new *game* starts at
    /// [`STARTING_MONEY`], which `World` applies through
    /// [`Economy::with_money`]; keeping that decision one layer up is what
    /// lets a test build an economy in isolation without a starting balance
    /// appearing in its arithmetic.
    pub fn new(contracts: &ContractRegistry) -> Self {
        Self::with_money(contracts, Money::ZERO)
    }

    /// [`Economy::new`], but starting from a chosen balance instead of
    /// [`STARTING_MONEY`]. The seam a test fixture uses to start rich (see
    /// [`super::reference::reference_chain`], which buys a whole factory) or
    /// deliberately broke, and where a loaded save will restore its balance.
    pub fn with_money(contracts: &ContractRegistry, money: Money) -> Self {
        Self {
            money,
            contracts: contracts
                .starters()
                .cloned()
                .map(|c| ActiveContract::signed_at(c, 0))
                .collect(),
            board: contracts.clone(),
            // Bounded per item since the dock-capacity change — see
            // DELIVERY_STOCK_CAPACITY for why the dock fills up and why the
            // ceiling is per item rather than one total.
            delivery_buffer: Inventory::new(DELIVERY_STOCK_CAPACITY),
            losses: BTreeMap::new(),
        }
    }

    /// The shipped `data/contracts.ron`. Convenience for tests and for
    /// `World`, which has no reason to hold a registry after construction.
    pub fn bundled() -> Self {
        Self::new(&ContractRegistry::bundled())
    }

    /// The board of offers this economy was built from, in file order.
    pub fn board(&self) -> &ContractRegistry {
        &self.board
    }

    /// Adds finished product into the shared delivery pool — called from
    /// [`super::dock::Dock::ship_into`], once per tick, for whatever the
    /// chain's delivery dock received that tick.
    ///
    /// **Returns how many units were actually accepted**, which is fewer
    /// than `qty` once the pool is at [`DELIVERY_STOCK_CAPACITY`] for that
    /// item. The caller must keep the remainder rather than assume the whole
    /// deposit landed: the dock does, so a box is never destroyed by a full
    /// pool (`tests/core_loop.rs`'s mass-conservation check is the property
    /// this protects). In practice the remainder is almost always zero,
    /// because [`super::dock::Dock::try_absorb`] refuses to take a box off
    /// the belt for a full pool in the first place — this is the second
    /// gate, for the tick where one dock fills the pool that another dock
    /// had already taken a box for.
    pub fn deposit(&mut self, item: &str, qty: u32) -> u32 {
        self.delivery_buffer.add(item, qty)
    }

    /// Whether the shared pool could take `qty` more of `item` — the gate
    /// [`super::dock::Dock::try_absorb`] asks before it takes a box off the
    /// belt, and therefore the definition of "this dock is full" that the
    /// wire and the HUD both report.
    pub fn has_delivery_room(&self, item: &str, qty: u32) -> bool {
        self.delivery_buffer.has_room_for(item, qty)
    }

    /// Whether any item in the pool has reached the ceiling — "the dock is
    /// full", as the dock's lamp and the clipboard say it. Deliberately
    /// *any* rather than *all*: one item at the ceiling is already enough to
    /// stop that product's line dead, which is the fact worth showing.
    pub fn delivery_pool_full(&self) -> bool {
        self.delivery_buffer
            .iter()
            .any(|(_, qty)| *qty >= DELIVERY_STOCK_CAPACITY)
    }

    /// Spends `amount` if there is enough money, and reports whether it
    /// did. **All or nothing:** on `false` not a single unit has moved, so
    /// a caller can check-and-spend in one step without ever leaving a
    /// half-applied transaction behind ("Shop & Inventory" epic, ticket 1).
    ///
    /// This is the *only* way money leaves the economy, and it lives here
    /// rather than on the shop for the reason `sim::shop`'s module doc
    /// gives: GAMEPLAY.md §1 allows exactly one currency, so there is
    /// exactly one counter, and a purchase — or a contract signature —
    /// debits the same field a contract delivery credits. A non-positive
    /// `amount` is refused rather than treated as a free purchase.
    pub fn try_spend(&mut self, amount: Money) -> bool {
        if amount <= Money::ZERO || self.money < amount {
            return false;
        }
        self.money -= amount;
        true
    }

    /// Signs the board's offer `id` at `current_tick`: debits its price,
    /// activates it with a fresh clock (first window ends one cadence from
    /// now) and zero counters, and slots it into the active list at its
    /// **file-order** position so priority under scarcity stays the
    /// board's. Refuses — changing nothing — an unknown id, an offer that is
    /// already active (one signature per offer, module doc) and a fee the
    /// balance does not cover.
    pub fn try_sign(&mut self, id: &str, current_tick: u64) -> Result<(), SignRefusal> {
        let Some(contract) = self.board.get(id).cloned() else {
            return Err(SignRefusal::Unknown);
        };
        if self.is_active(id) {
            return Err(SignRefusal::AlreadySigned);
        }
        if !self.try_spend(contract.price) {
            return Err(SignRefusal::CannotAfford);
        }
        self.insert_in_board_order(ActiveContract::signed_at(contract, current_tick));
        Ok(())
    }

    /// Whether the offer `id` is currently signed.
    pub fn is_active(&self, id: &str) -> bool {
        self.contracts.iter().any(|c| c.contract.id == id)
    }

    /// How many times the offer `id` has been lost.
    pub fn times_lost(&self, id: &str) -> u32 {
        self.losses.get(id).copied().unwrap_or(0)
    }

    /// Puts `active` into the list at the board's rank for its id — after
    /// every active contract listed earlier in the file, before every one
    /// listed later. Also the restore path's insertion, so a loaded save
    /// ends up in the same order a played game would.
    pub(super) fn insert_in_board_order(&mut self, active: ActiveContract) {
        let rank = self.rank(&active.contract.id);
        let at = self
            .contracts
            .iter()
            .position(|c| self.rank(&c.contract.id) > rank)
            .unwrap_or(self.contracts.len());
        self.contracts.insert(at, active);
    }

    fn rank(&self, id: &str) -> usize {
        self.board.position(id).unwrap_or(usize::MAX)
    }

    /// Whether the shared pool holds every line of `contract`'s mix.
    fn mix_satisfied(&self, contract: &Contract) -> bool {
        contract
            .mix
            .iter()
            .all(|(item, qty)| self.delivery_buffer.has_at_least(item, *qty))
    }

    /// Advances every active contract's own cadence clock and settles any
    /// that fall due on `current_tick`. For each such contract:
    ///
    /// - **Satisfied** (the shared pool holds its full mix): consumes the
    ///   mix from the pool, pays out, advances that contract's
    ///   `next_delivery_tick` by its own cadence, and clears its run of
    ///   misses.
    /// - **Not satisfied:** a missed window — no money changes hands and
    ///   nothing is consumed, but the cadence still advances rather than
    ///   re-checking every tick forever — and one more consecutive miss.
    ///   If that reaches the contract's `strikes`, the contract is **lost**:
    ///   removed from the active list before this call returns, counted in
    ///   `losses`, back on the board.
    ///
    /// Contracts are settled in priority order (module doc), so when two
    /// fall due together and stock covers only one, the earlier-listed one
    /// takes it. Returns what happened, in that same order — empty when
    /// nothing fell due.
    pub fn tick(&mut self, current_tick: u64) -> Vec<ContractEvent> {
        let mut events = Vec::new();
        let mut lost: Vec<usize> = Vec::new();
        for i in 0..self.contracts.len() {
            if current_tick < self.contracts[i].next_delivery_tick {
                continue;
            }
            self.contracts[i].next_delivery_tick += self.contracts[i].contract.cadence_ticks;
            let id = self.contracts[i].contract.id.clone();

            if !self.mix_satisfied(&self.contracts[i].contract) {
                let active = &mut self.contracts[i];
                active.deliveries_missed += 1;
                active.consecutive_misses += 1;
                events.push(ContractEvent::Missed {
                    id: id.clone(),
                    consecutive_misses: active.consecutive_misses,
                });
                if active.consecutive_misses >= active.contract.strikes {
                    lost.push(i);
                    events.push(ContractEvent::Lost { id });
                }
                continue;
            }

            for (item, qty) in &self.contracts[i].contract.mix {
                self.delivery_buffer.remove(item, *qty);
            }
            // Saturating, never wrapping (and never panicking in a debug
            // build): see `types::Money`.
            self.money = self.money.saturating_add(self.contracts[i].contract.payout);
            self.contracts[i].deliveries_made += 1;
            self.contracts[i].consecutive_misses = 0;
            events.push(ContractEvent::Paid { id });
        }
        // Highest index first, so earlier removals never shift a later one.
        for &i in lost.iter().rev() {
            let gone = self.contracts.remove(i);
            *self.losses.entry(gone.contract.id).or_insert(0) += 1;
        }
        events
    }

    /// The active contract with the given content id, if any — the lookup
    /// tests and command handling want, without exposing the `Vec`'s
    /// indices as an API.
    pub fn contract(&self, id: &str) -> Option<&ActiveContract> {
        self.contracts.iter().find(|c| c.contract.id == id)
    }

    pub fn snapshot(&self) -> EconomySnapshot {
        EconomySnapshot {
            money: self.money.to_num::<f64>(),
            delivery_stock: self.delivery_buffer.iter().cloned().collect(),
            delivery_capacity: self.delivery_buffer.capacity_per_item,
            delivery_full: self.delivery_pool_full(),
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
                            (
                                item.clone(),
                                self.delivery_buffer.count(item).min(*required),
                            )
                        })
                        .collect(),
                    satisfied: self.mix_satisfied(&active.contract),
                    cadence_ticks: active.contract.cadence_ticks,
                    window_start_tick: active.window_start_tick(),
                    next_delivery_tick: active.next_delivery_tick,
                    payout: active.contract.payout.to_num::<f64>(),
                    price: active.contract.price.to_num::<f64>(),
                    deliveries_made: active.deliveries_made,
                    deliveries_missed: active.deliveries_missed,
                    consecutive_misses: active.consecutive_misses,
                    strikes: active.contract.strikes,
                })
                .collect(),
            board: self
                .board
                .ordered()
                .iter()
                .map(|offer| ContractOfferSnapshot {
                    id: offer.id.clone(),
                    name: offer.name.clone(),
                    mix: offer.mix.clone(),
                    cadence_ticks: offer.cadence_ticks,
                    payout: offer.payout.to_num::<f64>(),
                    price: offer.price.to_num::<f64>(),
                    strikes: offer.strikes,
                    starter: offer.starter,
                    active: self.is_active(&offer.id),
                    affordable: self.money >= offer.price,
                    times_lost: self.times_lost(&offer.id),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two starter contracts on deliberately different cadences, both
    /// wanting the same item, a third starter wanting something the fixture
    /// never delivers (with enough strikes that it is never lost inside a
    /// test that is not about losing), and one unsigned offer. Written
    /// inline rather than read from `contracts.ron` so these unit tests keep
    /// testing `Economy`'s behaviour rather than the current balance of
    /// shipped content.
    const FIXTURE: &str = r#"(
        schema_version: 2,
        contracts: [
            Contract(id: "fast", name: "Fast", mix: [("boxed_ketchup", 2)], cadence_ticks: 100, payout: 50, price: 10, strikes: 2, starter: true),
            Contract(id: "offer", name: "Offer", mix: [("boxed_ketchup", 1)], cadence_ticks: 150, payout: 20, price: 30, strikes: 1),
            Contract(id: "slow", name: "Slow", mix: [("boxed_ketchup", 3)], cadence_ticks: 250, payout: 90, price: 20, strikes: 3, starter: true),
            Contract(id: "impossible", name: "Impossible", mix: [("chilli_mayo", 1)], cadence_ticks: 100, payout: 500, price: 5, strikes: 1000, starter: true),
        ],
    )"#;

    fn fixture() -> Economy {
        Economy::new(&ContractRegistry::from_ron_str(FIXTURE).expect("fixture must load"))
    }

    fn active_ids(econ: &Economy) -> Vec<&str> {
        econ.contracts
            .iter()
            .map(|c| c.contract.id.as_str())
            .collect()
    }

    fn paid(events: &[ContractEvent]) -> Vec<&str> {
        events
            .iter()
            .filter_map(|e| match e {
                ContractEvent::Paid { id } => Some(id.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn only_starters_are_active_in_a_new_economy_and_the_rest_wait_on_the_board() {
        let econ = fixture();
        assert_eq!(active_ids(&econ), vec!["fast", "slow", "impossible"]);
        assert!(!econ.is_active("offer"));
        assert_eq!(econ.board().len(), 4, "the board still lists every offer");
    }

    #[test]
    fn each_contract_starts_with_its_own_first_boundary_at_its_own_cadence() {
        let econ = fixture();
        assert_eq!(econ.contract("fast").unwrap().next_delivery_tick, 100);
        assert_eq!(econ.contract("slow").unwrap().next_delivery_tick, 250);
        assert_eq!(econ.contract("slow").unwrap().window_start_tick(), 0);
    }

    /// A payout at the top of the money type saturates: never a panic (a
    /// debug build's `+=` overflow), never a wrap to a negative balance.
    /// `Money` is 2^47 wide, so play never gets here; the guarantee is what
    /// matters (the quality pass).
    #[test]
    fn a_payout_at_the_ceiling_saturates_rather_than_wrapping() {
        let mut econ = fixture();
        econ.money = Money::MAX - Money::from_num(1);
        econ.deposit("boxed_ketchup", 10);
        let events = econ.tick(100);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ContractEvent::Paid { .. }))
        );
        assert_eq!(econ.money, Money::MAX);
        assert!(econ.money > Money::ZERO, "it did not wrap");
    }

    #[test]
    fn does_not_pay_out_before_the_cadence_boundary_arrives() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 10);
        assert!(econ.tick(99).is_empty());
        assert_eq!(econ.money, Money::from_num(0));
    }

    #[test]
    fn a_contract_pays_out_exactly_its_own_payout_and_consumes_exactly_its_own_mix() {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 5);

        // t=100: "fast" is due and paid; "impossible" is due and missed;
        // "slow" runs to 250.
        let events = econ.tick(100);

        assert_eq!(
            paid(&events),
            vec!["fast"],
            "only the 100-tick contract should have settled"
        );
        assert_eq!(econ.money, Money::from_num(50));
        assert_eq!(
            econ.delivery_buffer.count("boxed_ketchup"),
            3,
            "exactly the paid contract's mix must leave the shared pool"
        );
        assert_eq!(econ.contract("fast").unwrap().next_delivery_tick, 200);
        assert_eq!(econ.contract("fast").unwrap().window_start_tick(), 100);
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
            Money::from_num(100),
            "two 'fast' windows and no 'slow' window should have paid"
        );
        assert_eq!(econ.contract("slow").unwrap().deliveries_made, 0);

        econ.tick(250);
        assert_eq!(
            econ.money,
            Money::from_num(190),
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

        econ.tick(500);

        assert_eq!(
            econ.money,
            Money::from_num(50),
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
            Money::from_num(140),
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
        assert_eq!(impossible.consecutive_misses, 10);
        assert!(
            econ.contract("fast").unwrap().deliveries_made > 0,
            "an unsatisfiable contract must not stall the ones that can be filled"
        );
    }

    #[test]
    fn a_missed_delivery_can_still_pay_out_once_enough_accumulates_and_a_paid_window_clears_the_strike()
     {
        let mut econ = fixture();
        econ.deposit("boxed_ketchup", 1); // short by one for "fast"
        let events = econ.tick(100);
        assert!(paid(&events).is_empty());
        assert!(events.contains(&ContractEvent::Missed {
            id: "fast".to_string(),
            consecutive_misses: 1
        }));
        assert_eq!(econ.money, Money::from_num(0));
        assert_eq!(
            econ.delivery_buffer.count("boxed_ketchup"),
            1,
            "a miss must not destroy what was delivered"
        );
        assert_eq!(econ.contract("fast").unwrap().deliveries_missed, 1);
        assert_eq!(econ.contract("fast").unwrap().consecutive_misses, 1);
        assert!(
            econ.is_active("fast"),
            "one miss is one short of 'fast''s two strikes"
        );

        econ.deposit("boxed_ketchup", 1); // the missing unit arrives later
        assert_eq!(paid(&econ.tick(200)), vec!["fast"]);
        assert_eq!(econ.money, Money::from_num(50));
        assert_eq!(
            econ.contract("fast").unwrap().consecutive_misses,
            0,
            "a paid window forgives the run of misses"
        );
        assert_eq!(
            econ.contract("fast").unwrap().deliveries_missed,
            1,
            "the lifetime tally stays"
        );
    }

    #[test]
    fn missing_strikes_windows_in_a_row_loses_the_contract_and_puts_it_back_on_the_board() {
        let mut econ = fixture();
        // Nothing delivered: "fast" (2 strikes) misses at 100 and is lost at 200.
        let first = econ.tick(100);
        assert!(!first.contains(&ContractEvent::Lost {
            id: "fast".to_string()
        }));
        assert!(econ.is_active("fast"));

        let second = econ.tick(200);
        let lost_at = second
            .iter()
            .position(|e| {
                *e == ContractEvent::Lost {
                    id: "fast".to_string(),
                }
            })
            .expect("'fast' must be lost on its second consecutive miss");
        assert_eq!(
            second[lost_at - 1],
            ContractEvent::Missed {
                id: "fast".to_string(),
                consecutive_misses: 2
            },
            "Lost always follows the Missed that caused it"
        );
        assert!(
            !econ.is_active("fast"),
            "a lost contract leaves the active list on that tick"
        );
        assert_eq!(active_ids(&econ), vec!["slow", "impossible"]);
        assert_eq!(econ.times_lost("fast"), 1);
        assert_eq!(econ.times_lost("slow"), 0);

        let snap = econ.snapshot();
        let offer = snap.board.iter().find(|o| o.id == "fast").unwrap();
        assert!(!offer.active);
        assert_eq!(offer.times_lost, 1);

        // Later ticks must not keep settling a contract that is gone.
        for t in 201..=1000u64 {
            for e in econ.tick(t) {
                assert!(
                    !matches!(&e, ContractEvent::Missed { id, .. } | ContractEvent::Paid { id } if id == "fast"),
                    "a lost contract must not settle again: {e:?}"
                );
            }
        }
    }

    #[test]
    fn a_contract_with_one_strike_is_lost_on_its_first_miss() {
        let mut econ = fixture();
        econ.money = Money::from_num(100);
        econ.try_sign("offer", 0).unwrap();
        assert!(econ.is_active("offer"));
        let events = econ.tick(150);
        assert!(events.contains(&ContractEvent::Lost {
            id: "offer".to_string()
        }));
        assert!(!econ.is_active("offer"));
    }

    #[test]
    fn signing_debits_the_price_starts_a_fresh_clock_and_takes_the_boards_position() {
        let mut econ = fixture();
        econ.money = Money::from_num(100);

        assert_eq!(econ.try_sign("offer", 40), Ok(()));
        assert_eq!(
            econ.money,
            Money::from_num(70),
            "the fee is the offer's price"
        );
        let signed = econ.contract("offer").expect("now active");
        assert_eq!(
            signed.next_delivery_tick, 190,
            "first window ends one cadence after signing"
        );
        assert_eq!(signed.window_start_tick(), 40);
        assert_eq!(signed.deliveries_made, 0);
        assert_eq!(signed.consecutive_misses, 0);
        assert_eq!(
            active_ids(&econ),
            vec!["fast", "offer", "slow", "impossible"],
            "a contract signed last still sits at its file-order rank — priority is the board's"
        );
    }

    #[test]
    fn an_offer_can_be_signed_once_only_while_it_is_active() {
        let mut econ = fixture();
        econ.money = Money::from_num(1000);
        assert_eq!(econ.try_sign("offer", 0), Ok(()));
        assert_eq!(
            econ.try_sign("offer", 0),
            Err(SignRefusal::AlreadySigned),
            "one signature per offer"
        );
        assert_eq!(
            econ.money,
            Money::from_num(970),
            "a refused signature costs nothing"
        );
        assert_eq!(
            econ.contracts
                .iter()
                .filter(|c| c.contract.id == "offer")
                .count(),
            1
        );
        assert_eq!(
            econ.try_sign("fast", 0),
            Err(SignRefusal::AlreadySigned),
            "a starter is active too, and just as unsignable"
        );
    }

    #[test]
    fn signing_is_refused_whole_when_unaffordable_or_unknown() {
        let mut econ = fixture();
        econ.money = Money::from_num(29);
        assert_eq!(econ.try_sign("offer", 0), Err(SignRefusal::CannotAfford));
        assert_eq!(econ.money, Money::from_num(29));
        assert!(!econ.is_active("offer"));
        assert_eq!(
            econ.try_sign("midnight_kebab_van", 0),
            Err(SignRefusal::Unknown)
        );
        assert_eq!(econ.money, Money::from_num(29));
    }

    #[test]
    fn a_lost_contract_is_signed_again_at_full_price_and_remembers_nothing() {
        let mut econ = fixture();
        econ.money = Money::from_num(100);
        // Lose "fast": two misses in a row.
        econ.tick(100);
        econ.tick(200);
        assert!(!econ.is_active("fast"));

        assert_eq!(econ.try_sign("fast", 250), Ok(()));
        assert_eq!(
            econ.money,
            Money::from_num(90),
            "the starter's price applies when it is re-signed"
        );
        let again = econ.contract("fast").unwrap();
        assert_eq!(again.next_delivery_tick, 350);
        assert_eq!(
            again.deliveries_missed, 0,
            "a new signature is a new agreement"
        );
        assert_eq!(again.consecutive_misses, 0);
        assert_eq!(
            active_ids(&econ),
            vec!["fast", "slow", "impossible"],
            "back at its file rank"
        );
        assert_eq!(
            econ.times_lost("fast"),
            1,
            "but the board remembers it was lost"
        );
    }

    #[test]
    fn snapshot_reports_active_contracts_and_the_whole_board() {
        let mut econ = fixture();
        econ.money = Money::from_num(25);
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
        assert!(fast.satisfied);
        assert_eq!(fast.cadence_ticks, 100);
        assert_eq!(fast.window_start_tick, 0);
        assert_eq!(fast.next_delivery_tick, 100);
        assert_eq!(fast.payout, 50.0);
        assert_eq!(fast.price, 10.0);
        assert_eq!(fast.strikes, 2);
        assert_eq!(fast.consecutive_misses, 0);

        let impossible = &snap.contracts[2];
        assert_eq!(
            impossible.progress,
            vec![("chilli_mayo".to_string(), 0)],
            "an item with no stock reads zero, and still appears in progress"
        );
        assert!(!impossible.satisfied);

        let ids: Vec<&str> = snap.board.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["fast", "offer", "slow", "impossible"],
            "the board is in file order"
        );
        let offer = &snap.board[1];
        assert!(!offer.active);
        assert!(!offer.starter);
        assert_eq!(offer.price, 30.0);
        assert!(!offer.affordable, "25 in the bank does not cover a 30 fee");
        assert!(snap.board[0].active);
        assert!(snap.board[0].starter);
        assert!(
            snap.board[0].affordable,
            "affordability is reported for active offers too"
        );
        assert_eq!(offer.times_lost, 0);
    }

    #[test]
    fn the_bundled_contracts_build_a_working_economy() {
        let econ = Economy::bundled();
        assert_eq!(
            econ.contracts.len(),
            1,
            "a new game starts with exactly one contract — the starter — and buys the rest"
        );
        assert!(
            econ.board().len() > econ.contracts.len(),
            "there must be something to buy"
        );
        assert_eq!(econ.money, Money::from_num(0));
        for active in &econ.contracts {
            assert_eq!(
                active.next_delivery_tick, active.contract.cadence_ticks,
                "each contract's first window ends one cadence in"
            );
        }
    }
}
