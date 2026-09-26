//! `Dock` — the chain's finished-product exit (the "delivery dock" ticket).
//!
//! # What this replaces, and why it is a building rather than a command
//!
//! This is [`super::hopper`]'s argument, run at the other end of the line.
//! Milestones 4-7 emptied the chain with a debug affordance:
//! `PlayerCommand::CollectOutput` reached straight into the LAST machine's
//! own `output: Inventory`, drained it, and deposited the lot into a shared
//! delivery pool. Pressing **C** teleported finished boxes off the factory
//! floor and into a customer's hands out of nothing — no tile downstream of
//! the packer existed at all. That is fine as a "prove the economy pays out"
//! scaffold and wrong as a game mechanic, for the same three reasons the
//! `SpawnItem` command was wrong: nothing in the world held the product,
//! nothing moved it, and the player's action *was* the delivery rate.
//!
//! A loading dock is the real thing that does that job on a factory floor:
//! the end of the line, where product arrives on a conveyor and is staged
//! for a customer. So the last machine feeds an ordinary belt like every
//! other stage does, that belt runs into a dock building, and contracts
//! collect what is standing on the docks with no player action at all.
//!
//! # Each dock holds its own stock (2026-09-26)
//!
//! **What stands on a dock is that dock's**, in [`DOCK_CAPACITY`] places,
//! one case per place. Contracts settle against the SUM over every dock
//! that can ship this tick and take their mix from the docks in topology
//! order (`Economy::tick`, `DeliveryStock`), and the docks a payment
//! actually took from record it ([`Dock::last_paid_tick`]).
//!
//! This replaced a design where the dock was a pass-through: it shipped
//! everything it received, the same tick, into ONE pool on the economy,
//! and "what's on the dock" was that one number however many docks stood
//! on the floor. That kept the figure in one place, and it made every dock
//! a mirror of every other. A case landing on one dock popped a case on
//! all of them, a payment threw coins off all of them, and neither the
//! deck, the hover nor a "full" marker could say anything about the dock
//! the player was looking at, because there was nothing per dock to say.
//! The owner's playtest of 2026-09-26 called that out as docks being
//! "linked", and it was the model, not the drawing, that linked them.
//!
//! What survived the change is the rule that mattered: **one box can be
//! sold once.** Contracts still draw from one combined stock and still
//! consume their whole mix on payment, so two contracts are never credited
//! with the same box, and a second dock is still a second door out of the
//! factory rather than a second ledger. What changed is only WHERE the
//! boxes wait: on the dock they arrived at, until a contract takes them.
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
//! # Places, not a per-item ceiling
//!
//! The pool this replaced was capped per ITEM (48 of each), because one
//! total over a shared pool could deadlock the game: an overproduced
//! product would crowd every other item out, and a mix naming a
//! crowded-out item could never be filled again. A dock's places are one
//! TOTAL, and that is safe here for two reasons the pool did not have.
//! A dock is fed by the line in front of it, so one line's overproduction
//! fills only its own dock and never another line's. And a dock can be
//! removed, which destroys what stands on it (`PlayerCommand::Remove`'s
//! rule for every building), so a dock clogged with a product nobody orders
//! always has a way out. A total is also what the player can see: a grid
//! of places with cases in some of them.
//!
//! # Any item, unlike the hopper's one
//!
//! A [`Hopper`](super::hopper::Hopper) holds a single item id fixed at
//! construction, because a chain has exactly one raw ingredient and
//! `recipes.ron` says which. A dock is the opposite case: it accepts
//! whatever the line delivers to it, and is not commissioned for one SKU,
//! so it holds a plain multi-item [`Inventory`] whose slot order is the
//! order its places fill in.

use super::belt::BeltSegment;
use super::inventory::Inventory;
use super::types::ItemId;

/// The machine kind id (`data/machines.ron`) a dock is placed as.
///
/// `"dock"` rather than `"depot"`: the contract clipboard's own on-screen
/// line calls what is waiting to be sold "ON THE DOCK", so this is the word
/// the game already uses for the place finished goods wait to be sold.
/// Introducing a second noun for the same idea would cost terminology
/// consistency and buy nothing.
///
/// Spelled out in Rust for exactly the reason [`super::hopper::HOPPER_KIND`]
/// and [`super::fluid::FLUID_TANK_KIND`] are: the dock is *structural* —
/// it is where a chain ends — so unlike a processing stage it is not
/// derivable from `recipes.ron`'s list. Its footprint still comes from
/// `data/machines.ron` like any other kind; only the id is written here.
pub const DOCK_KIND: &str = "dock";

/// How many places one dock has — how many cases, of any mix of items, can
/// stand on it at once.
///
/// **The number is a relationship, not a literal** (the rule
/// [`super::economy::STARTING_MONEY`] follows): a contract settles only when
/// the docks hold its whole mix at once, so ONE dock must comfortably hold
/// the largest single order on the board, or a factory with one dock could
/// never fill it. `export_pallet` asks for 20 boxes in one window, and 32 is
/// 1.6x that: the pallet order can be building up while a small order
/// skims a few off the same dock. `tests/dock.rs` pins the relationship
/// against `data/contracts.ron` rather than the literal, so adding a bigger
/// order fails a test instead of quietly breaking that contract.
///
/// Its tightness is deliberate too. A line running against no demand fills
/// 32 places in about 8 000 ticks (four minutes at 30 Hz), and that is when
/// the dock says it is full (the jam lamp, and the "!" over the deck) and
/// the belt behind it backs up. That is long enough not to nag and short
/// enough to be a consequence of building past the board's demand.
///
/// A Rust constant rather than a `data/machines.ron` field, matching
/// `HOPPER_CAPACITY` and `FLUID_TANK_CAPACITY`: when buffer sizes become
/// per-kind content they should all move to `machines.ron` together.
pub const DOCK_CAPACITY: u32 = 32;

/// The end of the line: the places where finished product waits on this
/// dock until a contract collects it.
#[derive(Debug)]
pub struct Dock {
    /// What is standing on the dock, in the order it arrived per item — the
    /// order a presentation fills its grid of places in.
    ///
    /// Capped per item at `places` as well as in total, so the total is the
    /// only ceiling that ever binds: [`Self::try_absorb`] checks it.
    stock: Inventory,
    places: u32,
    /// Every unit this dock has ever taken off its belt. Never decreases;
    /// saturates rather than wrapping.
    ///
    /// The name is the line's point of view: the line has shipped it to this
    /// dock. Presentation diffs it to pop a case on THIS dock and no other.
    shipped_total: u32,
    /// The last tick a paying contract took stock from this dock, or `None`
    /// if none has since the world was built or loaded. What the coin burst
    /// reads to throw its coins off the docks that paid, and only those.
    ///
    /// Persisted (`save.rs`), unlike [`Self::blocked`]: it is a fact about
    /// the past that no tick recomputes, and a loaded world has to report
    /// what the saved one did.
    last_paid_tick: Option<u64>,
    /// Whether this dock was refused its crew on the last tick it held
    /// something (the staff pools MVP epic). Updated only on such ticks —
    /// an empty dock keeps the verdict of its last loaded one — so hover can
    /// say "held · short of logistics staff". A contract does not draw from
    /// a dock that was refused its crew this tick. On the wire as
    /// `DockSnapshot::understaffed`; saved (`save.rs`).
    understaffed: bool,
    /// Whether the player has MOTHBALLED this dock (issue #19): switched
    /// off, so it claims no clerk, nothing is collected from it and it takes
    /// nothing off the belt behind it. Persisted (`save.rs`), and the reason
    /// a factory that cannot make payroll can get its wage bill all the way
    /// to zero — the dock is the last thing on the line still asking for a
    /// crew.
    mothballed: bool,
    /// Whether this dock refused a box off its belt this tick because it had
    /// no free place — THIS dock's jam.
    ///
    /// Derived, not persisted: [`World::tick`] clears it on every dock
    /// before step 4 and [`Self::try_absorb`] raises it again, so it means
    /// "is backing up right now" rather than "once did". A world restored
    /// from a save reports `false` until its first tick.
    ///
    /// On the wire as `DockSnapshot::full`, where it drives the lamp.
    /// [`Self::at_capacity`] is the related but different fact: every place
    /// is taken, whether or not a box is waiting on the belt.
    blocked: bool,
}

impl Dock {
    pub fn new(places: u32) -> Self {
        Self {
            stock: Inventory::new(places),
            places,
            shipped_total: 0,
            last_paid_tick: None,
            understaffed: false,
            mothballed: false,
            blocked: false,
        }
    }

    /// Rebuilds a dock holding `contents`, having already taken
    /// `shipped_total` off its belt and last paid on `last_paid_tick` — the save/load module's seam
    /// (`sim::save`), and `pub(crate)` for its reason: `shipped_total` is a
    /// cumulative counter no player action can set, so only a loader may.
    ///
    /// Contents keep their slot order, empties included, for
    /// [`Inventory::restore`]'s reason.
    pub(crate) fn restore(
        places: u32,
        contents: Vec<(ItemId, u32)>,
        shipped_total: u32,
        last_paid_tick: Option<u64>,
        understaffed: bool,
        mothballed: bool,
    ) -> Self {
        Self {
            stock: Inventory::restore(places, contents),
            places,
            shipped_total,
            last_paid_tick,
            understaffed,
            mothballed,
            blocked: false,
        }
    }

    /// Whether this dock is refusing boxes off its belt right now, so the
    /// line behind it is backing up — what the lamp pulses on.
    ///
    /// False for a MOTHBALLED dock, which also takes nothing off the belt:
    /// that is a switch the player threw, reported by [`Self::mothballed`],
    /// and conflating the two would light a jam lamp on a dock that is
    /// simply off.
    pub fn blocked(&self) -> bool {
        self.blocked
    }

    /// Clears the jam verdict so [`Self::try_absorb`] can raise it again —
    /// called on every dock once per tick, before the pass that pulls off
    /// belts, in the `claim`/`reset` shape the staff pools already use.
    ///
    /// Separate from `try_absorb` because a dock may be fed by SEVERAL
    /// belts: resetting here and only ever setting there makes `blocked`
    /// "any inbound belt was refused", which is what backing up means.
    pub(crate) fn clear_blocked(&mut self) {
        self.blocked = false;
    }

    /// Whether the player has switched this dock off (issue #19).
    pub fn mothballed(&self) -> bool {
        self.mothballed
    }

    /// `World::try_set_mothballed` throws the switch. Nothing else may:
    /// this is a player command, not a consequence of the line's state.
    pub(crate) fn set_mothballed(&mut self, off: bool) {
        self.mothballed = off;
    }

    pub fn understaffed(&self) -> bool {
        self.understaffed
    }

    /// `World::tick`'s step 6 records the crew verdict here on every tick
    /// the dock holds something: `true` when the claim was refused (nothing
    /// is collected from it this tick; it keeps taking boxes until its
    /// places are full, then the belt backs up), `false` when it was crewed.
    pub(crate) fn set_understaffed(&mut self, refused: bool) {
        self.understaffed = refused;
    }

    /// Every slot on the dock, empties included — the save side of
    /// [`Self::restore`], see [`Inventory::slots`] for why the empties
    /// matter.
    pub(crate) fn slots(&self) -> &[(ItemId, u32)] {
        self.stock.slots()
    }

    /// What is standing on the dock right now, `(item id, quantity)`,
    /// non-zero entries only, in the order the item first arrived.
    pub fn contents(&self) -> impl Iterator<Item = &(ItemId, u32)> {
        self.stock.iter()
    }

    /// How many places this dock has — [`DOCK_CAPACITY`] for every placed
    /// dock; a parameter only so a test can build a small one.
    pub fn places(&self) -> u32 {
        self.places
    }

    /// How many of those places hold a case.
    pub fn used(&self) -> u32 {
        self.stock
            .iter()
            .fold(0u32, |sum, (_, qty)| sum.saturating_add(*qty))
    }

    /// Whether every place holds a case — the dock is full, whether or not
    /// a box is waiting on the belt behind it. What the "!" over the deck
    /// shows (`DockSnapshot::at_capacity`).
    pub fn at_capacity(&self) -> bool {
        self.used() >= self.places
    }

    pub fn is_empty(&self) -> bool {
        self.stock.iter().next().is_none()
    }

    /// Every unit this dock has ever taken off its belt. Never decreases;
    /// saturates rather than wrapping.
    pub fn shipped_total(&self) -> u32 {
        self.shipped_total
    }

    /// The last tick a paying contract took stock from this dock (see the
    /// field).
    pub fn last_paid_tick(&self) -> Option<u64> {
        self.last_paid_tick
    }

    /// Lends this dock's stock out for settlement — `World::tick`'s step 7
    /// moves every shipping dock's stock into one `DeliveryStock`, lets the
    /// economy settle against it, and hands each back with
    /// [`Self::return_stock`]. A move rather than a borrow because several
    /// docks have to be lent at once and each lives in its own ECS entity.
    pub(crate) fn take_stock(&mut self) -> Inventory {
        std::mem::take(&mut self.stock)
    }

    /// The other half of [`Self::take_stock`]. `paid_on` is the tick a
    /// paying contract took something from this stock, if one did.
    pub(crate) fn return_stock(&mut self, stock: Inventory, paid_on: Option<u64>) {
        self.stock = stock;
        if paid_on.is_some() {
            self.last_paid_tick = paid_on;
        }
    }

    /// Takes **exactly one** unit off the end of `belt`, if a unit has
    /// arrived there and this dock has a free place for it. Returns whether
    /// one moved.
    ///
    /// The mirror image of [`super::hopper::Hopper::try_unload`], and gated
    /// the same way: `front_ready` is the availability check,
    /// `take_front_if_at_end` is the commit, and nothing is half-moved in
    /// between. One unit per call (and `World::tick` calls it once per tick
    /// per inbound belt) keeps the dock's intake rate a property of the
    /// machinery rather than of how often anything asks.
    pub fn try_absorb(&mut self, belt: &mut BeltSegment) -> bool {
        let Some(item) = belt.front_ready().cloned() else {
            return false;
        };
        // Checked before the item is taken, so a full dock leaves the box
        // standing at the belt's end and the line backs up behind it —
        // exactly what a machine with a full input buffer already does.
        if self.at_capacity() || !self.stock.has_room_for(&item, 1) {
            // The jam is a fact about THIS dock: recorded here, where the
            // refusal is in hand, rather than re-derived by a snapshot.
            self.blocked = true;
            return false;
        }
        let Some(taken) = belt.take_front_if_at_end() else {
            return false;
        };
        self.stock.add(&taken.item, 1);
        self.shipped_total = self.shipped_total.saturating_add(1);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::types::Fx;

    const BELT_SPEED: f64 = 0.02;

    fn belt() -> BeltSegment {
        BeltSegment::new(Fx::from_num(4.0), Fx::from_num(0.25), 0)
    }

    fn dock() -> Dock {
        Dock::new(DOCK_CAPACITY)
    }

    /// Runs `belt` until its front item has reached the end, so the dock has
    /// something to take. Panics rather than silently doing nothing if the
    /// belt never delivers — a broken fixture must not read as a pass.
    fn run_to_end(belt: &mut BeltSegment) {
        for _ in 0..1_000 {
            if belt.front_ready().is_some() {
                return;
            }
            belt.tick(Fx::from_num(BELT_SPEED), 0);
        }
        panic!("the fixture belt never delivered an item to its end");
    }

    /// Puts `item` on a fresh belt, runs it down and has `d` take it.
    fn deliver(d: &mut Dock, item: &str) -> bool {
        let mut b = belt();
        assert!(b.try_spawn(item.to_string()));
        run_to_end(&mut b);
        d.try_absorb(&mut b)
    }

    #[test]
    fn a_dock_starts_empty_and_has_shipped_nothing() {
        let d = dock();
        assert!(d.is_empty());
        assert_eq!(d.shipped_total(), 0);
        assert_eq!(d.contents().count(), 0);
        assert_eq!(d.places(), DOCK_CAPACITY);
        assert_eq!(d.used(), 0);
        assert!(!d.at_capacity());
        assert_eq!(d.last_paid_tick(), None);
    }

    /// Nothing on the belt, or nothing that has arrived yet, must never
    /// produce a unit out of thin air — the exit-end twin of the hopper's
    /// `an_empty_hopper_never_puts_anything_on_the_belt`.
    #[test]
    fn an_empty_belt_never_puts_anything_on_the_dock() {
        let mut d = dock();
        let mut b = belt();
        for _ in 0..500 {
            assert!(!d.try_absorb(&mut b));
            b.tick(Fx::from_num(BELT_SPEED), 0);
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
        assert!(b.try_spawn("boxed_ketchup".to_string()));

        assert!(
            !d.try_absorb(&mut b),
            "a box that just entered the belt has not reached the dock"
        );
        assert_eq!(b.len(), 1, "and it must still be on the belt");

        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b));
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
            b.tick(Fx::from_num(BELT_SPEED), 0);
        }
        assert!(b.try_spawn("boxed_ketchup".to_string()));

        run_to_end(&mut b);
        assert!(d.try_absorb(&mut b));
        assert_eq!(b.len(), 1, "exactly one box moved, not the whole belt");
        assert_eq!(d.used(), 1);
    }

    /// What arrives STAYS — the dock is a store now, not a pass-through —
    /// and every arrival counts toward `shipped_total`, the counter
    /// presentation diffs to pop a case on this dock alone.
    #[test]
    fn what_arrives_stays_on_the_dock_and_counts_as_shipped_to_it() {
        let mut d = dock();
        for _ in 0..5 {
            assert!(deliver(&mut d, "boxed_ketchup"));
        }
        assert_eq!(
            d.contents().cloned().collect::<Vec<_>>(),
            vec![("boxed_ketchup".to_string(), 5)]
        );
        assert_eq!(d.used(), 5);
        assert_eq!(d.shipped_total(), 5);
    }

    /// A dock accepts whatever the line hands it, unlike the hopper's single
    /// fixed item, and keeps the kinds apart in arrival order.
    #[test]
    fn several_item_kinds_share_the_places() {
        let mut d = dock();
        for item in ["boxed_ketchup", "boxed_mayo", "boxed_ketchup"] {
            assert!(deliver(&mut d, item));
        }
        assert_eq!(
            d.contents().cloned().collect::<Vec<_>>(),
            vec![
                ("boxed_ketchup".to_string(), 2),
                ("boxed_mayo".to_string(), 1)
            ]
        );
        assert_eq!(d.used(), 3);
    }

    /// The gate that makes a full dock back the line up instead of eating
    /// the box, mirroring the hopper's `holds_its_contents_when_the_belt_has
    /// _no_room` — and the places are ONE total, so a dock full of one item
    /// refuses a different one too.
    #[test]
    fn a_full_dock_refuses_any_box_and_leaves_it_on_the_belt() {
        let mut d = Dock::new(2);
        assert!(deliver(&mut d, "boxed_ketchup"));
        assert!(deliver(&mut d, "boxed_ketchup"));
        assert!(d.at_capacity());

        let mut b = belt();
        assert!(b.try_spawn("boxed_bbq".to_string()));
        run_to_end(&mut b);
        for _ in 0..200 {
            assert!(
                !d.try_absorb(&mut b),
                "a dock with every place taken must not accept another box"
            );
            b.tick(Fx::from_num(BELT_SPEED), 0);
        }
        assert_eq!(b.len(), 1, "the refused box is still on the belt, not lost");
        assert_eq!(d.used(), 2);
        assert_eq!(d.shipped_total(), 2);
    }

    /// `blocked` is raised by the refusal itself and says nothing until one
    /// happens — a full dock with an empty belt behind it is at capacity but
    /// is not backing anything up.
    #[test]
    fn blocked_tracks_the_refusal_and_at_capacity_tracks_the_places() {
        let mut d = Dock::new(1);
        assert!(deliver(&mut d, "boxed_ketchup"));
        assert!(d.at_capacity());
        assert!(
            !d.blocked(),
            "a full dock alone is not backing up — nothing has been refused"
        );

        let mut b = belt();
        assert!(b.try_spawn("boxed_ketchup".to_string()));
        run_to_end(&mut b);
        assert!(!d.try_absorb(&mut b));
        assert!(
            d.blocked(),
            "the box was refused, so the line is backing up"
        );

        // A contract collects the case, and the same dock takes the box it
        // was refusing.
        let mut stock = d.take_stock();
        stock.remove("boxed_ketchup", 1);
        d.return_stock(stock, Some(40));
        d.clear_blocked();
        assert!(!d.at_capacity());
        assert!(d.try_absorb(&mut b));
        assert!(!d.blocked());
        assert_eq!(d.last_paid_tick(), Some(40));
    }

    /// Lending the stock out and back is lossless, and only a lend that paid
    /// moves `last_paid_tick` — a settlement that took nothing from this
    /// dock must not throw coins off it.
    #[test]
    fn a_lend_that_took_nothing_leaves_last_paid_tick_alone() {
        let mut d = dock();
        assert!(deliver(&mut d, "boxed_ketchup"));
        let stock = d.take_stock();
        d.return_stock(stock, Some(10));
        let stock = d.take_stock();
        d.return_stock(stock, None);
        assert_eq!(d.last_paid_tick(), Some(10));
        assert_eq!(d.used(), 1);
    }

    /// A restored dock is the saved one: same stock in the same order, same
    /// counters, and the derived jam flag starts clear.
    #[test]
    fn restore_round_trips_the_stock_and_clears_the_derived_flag() {
        let d = Dock::restore(
            DOCK_CAPACITY,
            vec![
                ("boxed_bbq".to_string(), 3),
                ("boxed_ketchup".to_string(), 0),
            ],
            17,
            Some(40),
            true,
            false,
        );
        assert_eq!(d.used(), 3);
        assert_eq!(d.shipped_total(), 17);
        assert!(d.understaffed());
        assert!(!d.blocked());
        assert_eq!(d.last_paid_tick(), Some(40));
        assert_eq!(
            d.slots().len(),
            2,
            "empties survive, for Inventory::restore's reason"
        );
    }
}
