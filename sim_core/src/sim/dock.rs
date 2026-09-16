//! `Dock` — the chain's finished-product exit (the "delivery dock" ticket).
//!
//! # What this replaces, and why it is a building rather than a command
//!
//! This is [`super::hopper`]'s argument, run at the other end of the line.
//! Milestones 4-7 emptied the chain with a debug affordance:
//! `PlayerCommand::CollectOutput` reached straight into the LAST machine's
//! own `output: Inventory`, drained it, and deposited the lot into
//! [`super::economy::Economy`]'s shared delivery pool. Pressing **C**
//! teleported finished boxes off the factory floor and into a customer's
//! hands out of nothing — no tile downstream of the packer existed at all.
//! That is fine as a "prove the economy pays out" scaffold and wrong as a
//! game mechanic, for the same three reasons the `SpawnItem` command was
//! wrong: nothing in the world held the product, nothing moved it, and the
//! player's action *was* the delivery rate.
//!
//! A loading dock is the real thing that does that job on a factory floor:
//! the end of the line, where product arrives on a conveyor and is staged
//! for a customer. So the last machine now feeds an ordinary belt like every
//! other stage does, that belt runs into a dock building, and the dock
//! deposits what it receives into the shared pool by itself, every tick,
//! with no player action at all. GAMEPLAY.md §6/§9 and `economy.rs`'s own
//! module doc both flagged "a real delivery-dock tile is future polish";
//! this is that tile, and `CollectOutput` is gone rather than kept beside
//! it — two ways to sell the same box is exactly the contradiction ticket 3
//! removed at the hopper end.
//!
//! # The shape this deliberately copies
//!
//! [`Dock::try_absorb`] is [`super::hopper::Hopper::try_unload`] mirrored:
//! the same "gate on availability, consume on success" pattern, reading off
//! a belt's end instead of writing to its start. The belt-side rules are not
//! re-implemented here either — [`super::belt::BeltSegment::front_ready`]
//! answers "is a unit waiting at the end" and `take_front_if_at_end` is the
//! commit, so a dock with no room simply doesn't take the item and the belt
//! backs up behind it, exactly the way a machine with a full input buffer
//! already makes one back up (`World::try_transfer_belt_to_machine`).
//!
//! # Why the dock drains completely, every tick
//!
//! The dock's own [`Inventory`] is a **transfer buffer, not a store**:
//! [`Dock::ship_into`] moves everything in it into the shared delivery pool
//! in the same tick it arrived, so a snapshot taken between ticks reads
//! empty (all but the one case [`Dock::ship_into`] names — a pool filled by
//! another dock later in the same tick). That was a real choice, and the alternative — hold a visible
//! stack on the dock and release it on some schedule — was rejected for two
//! reasons.
//!
//! 1. **There would be two different numbers both honestly called "what's on
//!    the dock," and the one the player could see would be the one contracts
//!    cannot use.** `Economy`'s shared pool already *is* the staged-goods
//!    number (`EconomySnapshot::delivery_stock`; the clipboard panel already
//!    labels it "ON THE DOCK"), and it is what every contract settles
//!    against. A second buffer in front of it would show boxes standing on
//!    the dock tile that no contract can count yet — a gauge reading one
//!    thing while the mechanism does another, which is the same failure
//!    [`super::fluid::FluidTank`] rejected when it decided to present
//!    constant head rather than its own fill fraction.
//! 2. **Nothing would gate the release, so the hold would be latency with no
//!    decision attached to it.** Contracts are already what meter the pool
//!    out, at their own cadences (`economy.rs`'s allocation rule); a dock
//!    that also held stock back would slow the loop down without giving the
//!    player anything to do about it, and a buffer you cannot act on is not
//!    a mechanic. When there *is* something to gate on — a truck that leaves
//!    on a cadence, or GAMEPLAY.md §7's logistics staff capacity — the hold
//!    becomes a real decision, and this is the one line that changes.
//!
//! So the visible "stock on the dock" stays exactly one number in exactly
//! one place, the same shape money already has (GAMEPLAY.md §1 allows
//! exactly one currency, and this file does not invent a second ledger of
//! goods either). Presentation should draw a pallet stack from
//! `EconomySnapshot::delivery_stock`, not from this buffer — see
//! `world::DockSnapshot`, which deliberately has no `fill` field for that
//! reason.
//!
//! # ...and why it can still be full (the dock-capacity change, 2026-09-16)
//!
//! A pass-through with an unbounded sink can never fill up, and for a year
//! this one could not: the player could point a line at a dock nobody had
//! ordered from and watch a number grow. The ceiling went on the pool it
//! drains into rather than on this buffer
//! ([`super::economy::DELIVERY_STOCK_CAPACITY`], per item id, and that
//! constant's doc carries the argument), because that is the number the
//! player already sees. Everything above survives it: there is still one
//! figure called "what's on the dock", it is still the one contracts settle
//! against, and this buffer still drains. What changed is that
//! [`Dock::try_absorb`] now asks the economy for room before it takes a box
//! off the belt, so a dock with nowhere to put a box refuses it and the
//! line backs up — the same refusal a full machine buffer already makes,
//! and the reason the "full" state is visible on the factory floor rather
//! than only in a readout. `DockSnapshot::full` reports it.
//!
//! # Any item, unlike the hopper's one
//!
//! A [`Hopper`](super::hopper::Hopper) holds a single item id fixed at
//! construction, because a chain has exactly one raw ingredient and
//! `recipes.ron` says which. A dock is the opposite case: it accepts
//! whatever the line delivers to it. That costs nothing today (the shipped
//! chain makes one sellable product) and is the honest model of the
//! building — a dock is not commissioned for one SKU — so it holds a plain
//! multi-item [`Inventory`] rather than one item id and a count.

use super::belt::BeltSegment;
use super::economy::Economy;
use super::inventory::Inventory;
use super::types::ItemId;

/// The machine kind id (`data/machines.ron`) a dock is placed as.
///
/// `"dock"` rather than `"depot"`: `economy.rs`'s shared pool is already
/// described as what is "on the dock" throughout this codebase (its
/// `delivery_stock` field doc, and the contract clipboard's own on-screen
/// line), so this is the word the game already uses for the place finished
/// goods wait to be sold. Introducing a second noun for the same idea would
/// cost terminology consistency and buy nothing.
///
/// Spelled out in Rust for exactly the reason [`super::hopper::HOPPER_KIND`]
/// and [`super::fluid::FLUID_TANK_KIND`] are: the dock is *structural* —
/// every chain has exactly one, at its tail — so unlike a processing stage
/// it is not derivable from `recipes.ron`'s list. Its footprint still comes
/// from `data/machines.ron` like any other kind; only the id is written
/// here.
pub const DOCK_KIND: &str = "dock";

/// How many units of any one item the dock can hold at once.
///
/// Sized against what a delivery actually is, the way
/// [`super::hopper::HOPPER_CAPACITY`] is sized against what a craft is: the
/// largest single order on the shipped board asks for 3 boxes and the whole
/// board asks for 6 across its three contracts (`data/contracts.ron`), so 32
/// is roughly five boards' worth — the staging area is never the reason a
/// delivery is short. It is half a hopper, which is also about right for the
/// relative bulk of a boxed pallet against loose tomatoes.
///
/// **It is still not a constraint the shipped world can reach**, and that is
/// stated rather than hidden: [`Dock::ship_into`] empties the buffer every
/// tick and [`Dock::try_absorb`] fills it at most one unit per tick, so it
/// never holds more than one. The bound exists because the absorb has to be
/// able to say *no* — that is what makes a full dock back the belt up
/// instead of annihilating the item, which is the property the
/// mass-conservation test in `tests/core_loop.rs` actually checks.
///
/// The ceiling the player actually meets is the shared pool's
/// ([`super::economy::DELIVERY_STOCK_CAPACITY`], added 2026-09-16): the
/// drain IS gated now, exactly as this paragraph used to predict, and it
/// was the one-line change it predicted — `try_absorb` asks the economy for
/// room beside asking this buffer, and the refusal path this bound had
/// already built and tested is what carried it.
///
/// A Rust constant rather than a `data/machines.ron` field, matching
/// `HOPPER_CAPACITY` and `FLUID_TANK_CAPACITY`: when buffer sizes become
/// per-kind content they should all move to `machines.ron` together.
pub const DOCK_CAPACITY: u32 = 32;

/// The end of the line: a bounded buffer that takes finished product off a
/// belt and ships it into the economy's shared delivery pool.
#[derive(Debug)]
pub struct Dock {
    buffer: Inventory,
    shipped_total: u32,
    /// Whether this dock was refused its crew on the last tick it held
    /// something to ship (the staff pools MVP epic). Updated only on such
    /// ticks — an empty dock keeps the verdict of its last loaded one — so
    /// hover can say "held · short of logistics staff" while the belt
    /// behind it backs up. On the wire as `DockSnapshot::understaffed`;
    /// saved (`save.rs`).
    understaffed: bool,
}

impl Dock {
    pub fn new(capacity_per_item: u32) -> Self {
        Self {
            buffer: Inventory::new(capacity_per_item),
            shipped_total: 0,
            understaffed: false,
        }
    }

    /// Rebuilds a dock holding `contents` and having already shipped
    /// `shipped_total` — the save/load module's seam (`sim::save`), and
    /// `pub(crate)` for its reason: `shipped_total` is a cumulative
    /// counter no player action can set, so only a loader may.
    ///
    /// Contents keep their slot order, empties included, for
    /// [`Inventory::restore`]'s reason.
    pub(crate) fn restore(
        capacity_per_item: u32,
        contents: Vec<(ItemId, u32)>,
        shipped_total: u32,
        understaffed: bool,
    ) -> Self {
        Self {
            buffer: Inventory::restore(capacity_per_item, contents),
            shipped_total,
            understaffed,
        }
    }

    pub fn understaffed(&self) -> bool {
        self.understaffed
    }

    /// `World::tick`'s step 6 records the crew verdict here on every tick
    /// the dock held something to ship: `true` when the claim was refused
    /// (the buffer is held; at capacity the belt backs up, so nothing is
    /// lost), `false` when it shipped.
    pub(crate) fn set_understaffed(&mut self, refused: bool) {
        self.understaffed = refused;
    }

    /// Every slot on the dock, empties included — the save side of
    /// [`Self::restore`], see [`Inventory::slots`] for why the empties
    /// matter.
    pub(crate) fn slots(&self) -> &[(ItemId, u32)] {
        self.buffer.slots()
    }

    /// What is sitting on the dock right now, `(item id, quantity)`,
    /// non-zero entries only. Between ticks this is always empty — see the
    /// module doc's drain rule; it is non-empty only *inside* a tick,
    /// between [`Self::try_absorb`] and [`Self::ship_into`].
    pub fn contents(&self) -> impl Iterator<Item = &(ItemId, u32)> {
        self.buffer.iter()
    }

    pub fn capacity_per_item(&self) -> u32 {
        self.buffer.capacity_per_item
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.iter().next().is_none()
    }

    /// Every unit this dock has ever shipped into the delivery pool — the
    /// line's cumulative output, and the one number about a dock that is
    /// worth looking at (its live level is a pass-through, see the module
    /// doc). Never decreases; saturates rather than wrapping.
    pub fn shipped_total(&self) -> u32 {
        self.shipped_total
    }

    /// Takes **exactly one** unit off the end of `belt`, if a unit has
    /// arrived there and this dock has room for it. Returns whether one
    /// moved.
    ///
    /// The mirror image of [`super::hopper::Hopper::try_unload`], and gated
    /// the same way: `front_ready` is the availability check,
    /// `take_front_if_at_end` is the commit, and nothing is half-moved in
    /// between. One unit per call (and `World::tick` calls it once per tick)
    /// keeps the dock's intake rate a property of the machinery rather than
    /// of how often anything asks.
    pub fn try_absorb(&mut self, belt: &mut BeltSegment, economy: &Economy) -> bool {
        let Some(item) = belt.front_ready().cloned() else {
            return false;
        };
        // Checked before the item is taken, so a full dock leaves the box
        // standing at the belt's end and the line backs up behind it —
        // exactly what a machine with a full input buffer already does.
        //
        // TWO ceilings, and the second is the one the player meets (the
        // dock-capacity change, 2026-09-16). This buffer is a pass-through
        // and empties every tick, so it is the SHARED POOL that actually
        // fills up: `has_delivery_room` is what makes "the dock is full" a
        // reachable state rather than the unreachable one `DOCK_CAPACITY`'s
        // doc describes. Asking the economy here rather than letting
        // `ship_into` drop the overflow is what keeps the refusal on the
        // belt, where it backs the line up instead of destroying a box.
        if !self.buffer.has_room_for(&item, 1) || !economy.has_delivery_room(&item, 1) {
            return false;
        }
        let Some(taken) = belt.take_front_if_at_end() else {
            return false;
        };
        self.buffer.add(&taken.item, 1);
        true
    }

    /// Moves everything on the dock into the economy's shared delivery pool
    /// and returns how many units went. This is the call that replaces
    /// `PlayerCommand::CollectOutput`'s handler — the same
    /// [`Economy::deposit`], made every tick by the building instead of on a
    /// keypress by the player.
    ///
    /// Unthrottled, and complete except against a full pool: nothing is
    /// held back on purpose (contracts already meter the pool out at their
    /// own cadences, so a hold here would be latency with no decision
    /// attached to it — see the module doc), but the pool has had a ceiling
    /// since the dock-capacity change and **only what the pool accepts
    /// leaves this buffer**. The remainder stays on the dock and is offered
    /// again next tick, so a box is never destroyed by a full pool.
    ///
    /// That is the one case where a between-ticks snapshot of a dock reads
    /// non-empty, and it is rare by construction: [`Self::try_absorb`]
    /// refuses to take a box for a pool with no room, so the buffer only
    /// ever holds an unshippable unit when ANOTHER dock filled the pool
    /// later in the same tick.
    pub fn ship_into(&mut self, economy: &mut Economy) -> u32 {
        let held: Vec<(ItemId, u32)> = self.buffer.iter().cloned().collect();
        let mut shipped = 0u32;
        for (item, qty) in held {
            let accepted = economy.deposit(&item, qty);
            self.buffer.remove(&item, accepted);
            shipped = shipped.saturating_add(accepted);
        }
        self.shipped_total = self.shipped_total.saturating_add(shipped);
        shipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::economy::DELIVERY_STOCK_CAPACITY;
    use crate::sim::types::Fx;

    const BELT_SPEED: f64 = 0.02;

    fn belt() -> BeltSegment {
        BeltSegment::new(Fx::from_num(4.0), Fx::from_num(0.25))
    }

    fn dock() -> Dock {
        Dock::new(DOCK_CAPACITY)
    }

    /// The shipped contract board, never ticked — so nothing ever consumes
    /// out of the shared pool and these tests observe exactly what the dock
    /// put there. (Settling contracts is `Economy`'s own tested job; this
    /// file is about what reaches the pool, not what leaves it.)
    fn economy() -> Economy {
        Economy::bundled()
    }

    /// Runs `belt` until its front item has reached the end, so the dock has
    /// something to take. Panics rather than silently doing nothing if the
    /// belt never delivers — a broken fixture must not read as a pass.
    fn run_to_end(belt: &mut BeltSegment) {
        for _ in 0..1_000 {
            if belt.front_ready().is_some() {
                return;
            }
            belt.tick(Fx::from_num(BELT_SPEED));
        }
        panic!("the fixture belt never delivered an item to its end");
    }

    #[test]
    fn a_dock_starts_empty_and_has_shipped_nothing() {
        let d = dock();
        assert!(d.is_empty());
        assert_eq!(d.shipped_total(), 0);
        assert_eq!(d.contents().count(), 0);
        assert_eq!(d.capacity_per_item(), DOCK_CAPACITY);
    }

    /// Nothing on the belt, or nothing that has arrived yet, must never
    /// produce a unit out of thin air — the exit-end twin of the hopper's
    /// `an_empty_hopper_never_puts_anything_on_the_belt`.
    #[test]
    fn an_empty_belt_never_puts_anything_on_the_dock() {
        let mut d = dock();
        let mut b = belt();
        let econ = economy();
        for _ in 0..500 {
            assert!(!d.try_absorb(&mut b, &econ));
            b.tick(Fx::from_num(BELT_SPEED));
        }
        assert!(d.is_empty());
        assert_eq!(d.shipped_total(), 0);
    }

    /// An item that is on the belt but has not travelled to its end yet is
    /// not the dock's to take — `front_ready` is the gate, and it is the
    /// belt's own rule, not a second copy of it here.
    #[test]
    fn an_item_still_travelling_is_not_absorbed_until_it_arrives() {
        let mut d = dock();
        let mut b = belt();
        let econ = economy();
        assert!(b.try_spawn("boxed_ketchup".to_string()));

        assert!(
            !d.try_absorb(&mut b, &econ),
            "a box that just entered the belt has not reached the dock"
        );
        assert_eq!(b.len(), 1, "and it must still be on the belt");

        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b, &econ));
        assert!(b.is_empty());
        assert_eq!(
            d.contents().cloned().collect::<Vec<_>>(),
            vec![("boxed_ketchup".to_string(), 1)]
        );
    }

    #[test]
    fn absorbs_exactly_one_unit_per_call() {
        let mut d = dock();
        let mut b = belt();
        assert!(b.try_spawn("boxed_ketchup".to_string()));
        // Space the second box out behind the first, then let both run down.
        for _ in 0..20 {
            b.tick(Fx::from_num(BELT_SPEED));
        }
        assert!(b.try_spawn("boxed_ketchup".to_string()));

        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b, &economy()));
        assert_eq!(b.len(), 1, "exactly one box moved, not the whole belt");
        assert_eq!(d.contents().map(|(_, n)| *n).sum::<u32>(), 1);
    }

    /// The gate that makes a full dock back the line up instead of eating
    /// the box, mirroring the hopper's `holds_its_contents_when_the_belt_has
    /// _no_room`. A capacity-1 dock is used because the shipped
    /// `DOCK_CAPACITY` is deliberately unreachable while the drain runs
    /// every tick (see its doc) — the *rule* still has to be tested.
    #[test]
    fn a_full_dock_refuses_the_box_and_leaves_it_on_the_belt() {
        let mut d = Dock::new(1);
        let mut b = belt();
        assert!(b.try_spawn("boxed_ketchup".to_string()));
        for _ in 0..20 {
            b.tick(Fx::from_num(BELT_SPEED));
        }
        assert!(b.try_spawn("boxed_ketchup".to_string()));
        run_to_end(&mut b);

        let econ = economy();
        assert!(d.try_absorb(&mut b, &econ));
        for _ in 0..500 {
            b.tick(Fx::from_num(BELT_SPEED));
            assert!(
                !d.try_absorb(&mut b, &econ),
                "a full dock must not accept a second box"
            );
        }
        assert_eq!(b.len(), 1, "the refused box is still on the belt, not lost");
        assert_eq!(d.contents().map(|(_, n)| *n).sum::<u32>(), 1);
    }

    /// The whole point of the building: what lands on it goes into the
    /// shared delivery pool, completely, and the dock is empty again.
    #[test]
    fn shipping_moves_everything_into_the_delivery_pool_and_empties_the_dock() {
        let mut d = dock();
        let mut econ = economy();
        let mut b = belt();

        assert!(b.try_spawn("boxed_ketchup".to_string()));
        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b, &econ));

        assert_eq!(d.ship_into(&mut econ), 1);
        assert!(d.is_empty(), "the dock is a pass-through, not a store");
        assert_eq!(econ.delivery_buffer.count("boxed_ketchup"), 1);
        assert_eq!(d.shipped_total(), 1);
    }

    #[test]
    fn shipping_an_empty_dock_is_a_no_op() {
        let mut d = dock();
        let mut econ = economy();
        assert_eq!(d.ship_into(&mut econ), 0);
        assert_eq!(d.shipped_total(), 0);
        assert!(econ.delivery_buffer.iter().next().is_none());
    }

    /// A dock accepts whatever the line hands it, unlike the hopper's single
    /// fixed item — and ships all of it, not just the first kind.
    #[test]
    fn several_item_kinds_are_all_held_and_all_shipped() {
        let mut d = dock();
        let mut econ = economy();
        let mut b = belt();

        for item in ["boxed_ketchup", "boxed_mayo", "boxed_ketchup"] {
            assert!(b.try_spawn(item.to_string()));
            run_to_end(&mut b);
            assert!(d.try_absorb(&mut b, &econ));
        }
        assert_eq!(d.contents().map(|(_, n)| *n).sum::<u32>(), 3);

        assert_eq!(d.ship_into(&mut econ), 3);
        assert_eq!(econ.delivery_buffer.count("boxed_ketchup"), 2);
        assert_eq!(econ.delivery_buffer.count("boxed_mayo"), 1);
        assert_eq!(d.shipped_total(), 3);
        assert!(d.is_empty());
    }

    /// The dock-capacity change's central rule: once the shared pool is at
    /// its per-item ceiling, the dock stops taking boxes off the belt. Not
    /// "takes it and drops it" and not "takes it and holds it forever" —
    /// the box stays on the belt, which is what backs the line up.
    #[test]
    fn a_full_delivery_pool_leaves_the_box_on_the_belt() {
        let mut d = dock();
        let mut econ = economy();
        let mut b = belt();

        econ.deposit("boxed_ketchup", DELIVERY_STOCK_CAPACITY);
        assert!(econ.delivery_pool_full());

        assert!(b.try_spawn("boxed_ketchup".to_string()));
        run_to_end(&mut b);
        for _ in 0..200 {
            assert!(
                !d.try_absorb(&mut b, &econ),
                "a dock with nowhere to ship must refuse the box"
            );
            b.tick(Fx::from_num(BELT_SPEED));
        }
        assert_eq!(b.len(), 1, "the refused box is still on the belt, not lost");
        assert!(d.is_empty());
        assert_eq!(
            econ.delivery_buffer.count("boxed_ketchup"),
            DELIVERY_STOCK_CAPACITY,
            "and nothing was added to a pool that had no room"
        );
    }

    /// The ceiling is per item id, so a pool full of one product still takes
    /// another — the property that keeps a contract's mix fillable and the
    /// game out of a deadlock (`DELIVERY_STOCK_CAPACITY`'s doc).
    #[test]
    fn a_pool_full_of_one_item_still_accepts_another() {
        let mut d = dock();
        let econ = {
            let mut e = economy();
            e.deposit("boxed_ketchup", DELIVERY_STOCK_CAPACITY);
            e
        };
        let mut b = belt();

        assert!(b.try_spawn("boxed_mayo".to_string()));
        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b, &econ));
        assert_eq!(d.contents().map(|(_, n)| *n).sum::<u32>(), 1);
    }

    /// The second gate: a dock that absorbed a box before the pool filled
    /// (another dock shipped first, in the same tick) KEEPS it rather than
    /// shipping it into nothing. Mass conservation is the invariant here,
    /// not the drain.
    #[test]
    fn shipping_into_a_full_pool_holds_the_box_instead_of_destroying_it() {
        let mut d = dock();
        let mut econ = economy();
        let mut b = belt();

        assert!(b.try_spawn("boxed_ketchup".to_string()));
        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b, &econ));

        // The pool fills between the absorb and the ship.
        econ.deposit("boxed_ketchup", DELIVERY_STOCK_CAPACITY);

        assert_eq!(d.ship_into(&mut econ), 0, "nothing could be shipped");
        assert_eq!(
            d.contents().map(|(_, n)| *n).sum::<u32>(),
            1,
            "so the box is still standing on the dock"
        );
        assert_eq!(d.shipped_total(), 0);
        assert_eq!(
            econ.delivery_buffer.count("boxed_ketchup"),
            DELIVERY_STOCK_CAPACITY
        );

        // And it goes the moment room appears.
        econ.delivery_buffer.remove("boxed_ketchup", 1);
        assert_eq!(d.ship_into(&mut econ), 1);
        assert!(d.is_empty());
    }

    /// A partial ship: the pool has room for some of what the dock holds,
    /// so some goes and the rest stays. The one case where `ship_into`'s
    /// return value and the buffer's contents have to agree unit for unit.
    #[test]
    fn a_partly_full_pool_takes_what_fits_and_leaves_the_rest() {
        let mut d = dock();
        let mut econ = economy();
        let mut b = belt();

        for _ in 0..3 {
            assert!(b.try_spawn("boxed_ketchup".to_string()));
            run_to_end(&mut b);
            assert!(d.try_absorb(&mut b, &econ));
        }
        assert_eq!(d.contents().map(|(_, n)| *n).sum::<u32>(), 3);

        econ.deposit("boxed_ketchup", DELIVERY_STOCK_CAPACITY - 1);
        assert_eq!(d.ship_into(&mut econ), 1);
        assert_eq!(d.contents().map(|(_, n)| *n).sum::<u32>(), 2);
        assert_eq!(d.shipped_total(), 1);
        assert!(econ.delivery_pool_full());
    }

    /// `shipped_total` is cumulative across ships, which is what makes it
    /// the number a hover/HUD shows and the number a test can assert
    /// automatic delivery against.
    #[test]
    fn shipped_total_accumulates_across_ticks() {
        let mut d = dock();
        let mut econ = economy();
        let mut b = belt();

        for _ in 0..5 {
            assert!(b.try_spawn("boxed_ketchup".to_string()));
            run_to_end(&mut b);
            assert!(d.try_absorb(&mut b, &econ));
            d.ship_into(&mut econ);
        }
        assert_eq!(d.shipped_total(), 5);
        assert_eq!(econ.delivery_buffer.count("boxed_ketchup"), 5);
    }
}
