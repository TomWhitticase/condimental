//! `Hopper` — the chain's raw-ingredient feeder (the "Machines improvement"
//! milestone's ticket 3).
//!
//! # What this replaces, and why it is a building rather than a command
//!
//! Milestones 3-6 fed the chain with a debug affordance: `PlayerCommand::
//! SpawnItem` reached straight into stage 0's `BeltSegment` and called
//! `try_spawn`, so pressing SPACE teleported a tomato onto the belt out of
//! nothing. That is fine as a "prove the belt renders" scaffold and wrong as
//! a game mechanic — nothing in the world held the raw ingredient, nothing
//! metered it onto the line, and the player's action *was* the production
//! rate.
//!
//! A hopper is the real thing that does that job on a factory floor: a bin
//! that is loaded in bulk and meters its contents onto the line one unit at
//! a time. So the player action becomes **filling** the hopper (a bounded
//! buffer, [`Hopper::fill`]), and the *simulation* does the unloading, one
//! unit per tick at most, and only when the belt actually has room
//! ([`Hopper::try_unload`]).
//!
//! # The shape this deliberately copies
//!
//! [`Hopper::try_unload`] is the same "gate on availability, consume on
//! success" pattern [`super::machine::Machine::tick`] already uses to start a
//! craft: check that the downstream/upstream resource is really available,
//! and only then commit the state change — never half-consume, never run
//! degraded. Concretely, the room check is not re-implemented here at all:
//! [`super::belt::BeltSegment::try_spawn`] already answers "is there room at
//! the belt's start" *and* places the item, and it is a no-op returning
//! `false` when there isn't — so `try_unload` calls it and only decrements
//! its own count if it succeeded. There is exactly one min-spacing rule in
//! the codebase and it stays in `belt.rs`.
//!
//! # One item, by design
//!
//! A `Hopper` holds a single item id, fixed at construction. The chain has
//! one raw-ingredient entry point (`world.rs`'s module doc), and *which*
//! item that is comes from `data/recipes.ron` — the first recipe's first
//! input — never from a constant here. Multi-item hoppers, several hoppers,
//! or a hopper the player can re-target are not needed by anything yet and
//! are not built (CLAUDE.md: don't design for hypothetical requirements).

use super::belt::BeltSegment;
use super::types::ItemId;

/// The machine kind id (`data/machines.ron`) a hopper is placed as.
///
/// This is the one content id this milestone spells out in Rust, for the
/// same reason [`super::economy`]'s contract constants are: the hopper is
/// structural — every world has exactly one, at the chain's head — so it is
/// not derivable from `recipes.ron`'s list the way every *processing* stage
/// is. Its footprint still comes from `data/machines.ron` like any other
/// kind; only the id is written here. When the player can place buildings
/// (a real build mode), this becomes just another entry in whatever
/// placement data that milestone introduces.
pub const HOPPER_KIND: &str = "hopper";

/// How many units a hopper holds when full.
///
/// Sized against the line it feeds rather than picked round: the shipped
/// crusher consumes 4 tomatoes every 80 ticks (`data/recipes.ron`), so 64
/// tomatoes is 16 crafts — roughly 43 seconds of unattended running at
/// ARCHITECTURE.md §3.9's 30 ticks/sec. That is the intended feel: enough
/// that filling it buys real breathing room, little enough that keeping the
/// line fed is an ongoing decision rather than something you do once and
/// forget.
///
/// It is a Rust constant rather than a `data/machines.ron` field on purpose,
/// matching how `world.rs` already sizes every machine's input/output buffer
/// (`MACHINE_INPUT_CAPACITY`). When buffer sizes become per-kind content,
/// they should all move to `machines.ron` together — one field for one kind
/// that nothing else can use would be a worse fit than the constant it
/// replaced.
pub const HOPPER_CAPACITY: u32 = 64;

/// How much one fill command adds when it doesn't say (see
/// `world::PlayerCommand::FillHopper`) — a crate of tomatoes, a quarter of
/// [`HOPPER_CAPACITY`], so a full hopper is four deliberate actions.
pub const HOPPER_FILL_AMOUNT: u32 = 16;

/// A bounded buffer of one raw item, feeding one belt.
#[derive(Debug, Clone)]
pub struct Hopper {
    /// The single item id this hopper holds — from `data/recipes.ron`, see
    /// the module doc.
    item: ItemId,
    quantity: u32,
    capacity: u32,
    auto_fill: bool,
}

impl Hopper {
    pub fn new(item: ItemId, capacity: u32) -> Self {
        Self {
            item,
            quantity: 0,
            capacity,
            // A new hopper is loaded by hand until the player says
            // otherwise — see [`Hopper::auto_fill`].
            auto_fill: false,
        }
    }

    pub fn item(&self) -> &ItemId {
        &self.item
    }

    pub fn quantity(&self) -> u32 {
        self.quantity
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.quantity == 0
    }

    /// How much more this hopper can take — what an automatic top-up asks
    /// for (see `World::tick`'s auto-fill phase).
    pub fn room(&self) -> u32 {
        self.capacity.saturating_sub(self.quantity)
    }

    /// Whether `World::tick` keeps this hopper topped up to capacity out of
    /// the player's bought stock, every tick, without a command.
    ///
    /// **It spends stock, never money.** Buying ingredients stays an
    /// explicit `PlayerCommand::BuyIngredient` — GAMEPLAY.md §5 makes the
    /// recurring ingredient cost one of the three things the player is
    /// deciding about, and a toggle that quietly drained the balance would
    /// turn that decision into a background leak. When the larder runs out
    /// the hopper simply stops being topped up: no error, and this flag is
    /// **not** flipped off, so the top-ups resume by themselves the moment
    /// stock is bought again.
    ///
    /// **Changing the contents does not clear it** ([`Hopper::retarget`]).
    /// "Keep this bin full" is a statement about the bin, not about what is
    /// in it — a player who set it and then switched a hopper from tomato to
    /// sugar still wants it kept full, now of sugar.
    ///
    /// `false` on a newly placed hopper, and a removed-then-replaced hopper
    /// is a new hopper (`PlayerCommand::Remove` destroys the body with
    /// everything in it), so it comes back `false` too.
    pub fn auto_fill(&self) -> bool {
        self.auto_fill
    }

    pub fn set_auto_fill(&mut self, enabled: bool) {
        self.auto_fill = enabled;
    }

    /// Switches which item this hopper holds, discarding whatever it
    /// currently has — a hopper is single-item by design (this module's own
    /// doc), so there is no "half tomato, half sugar" state to preserve.
    /// This is the same honest reading `World::try_remove` already gives
    /// picking a building up ("what it holds is lost"), applied to changing
    /// a placed hopper's mind instead of removing it outright: the
    /// alternative, refusing to retarget a non-empty hopper, would leave a
    /// player with no way to switch a full tomato hopper to sugar short of
    /// removing and re-placing it, for no real benefit over just clearing it.
    ///
    /// [`Hopper::auto_fill`] is deliberately **not** reset — see its doc.
    pub fn retarget(&mut self, item: ItemId) {
        self.item = item;
        self.quantity = 0;
    }

    /// Adds up to `amount` units, **clamped at capacity**, and returns how
    /// many were actually added (0 when already full).
    ///
    /// Clamping is silent, never an error and never a panic: this mirrors
    /// [`super::inventory::Inventory::add`]'s contract exactly, and it is
    /// the same "an impossible player command is a no-op, not a failure"
    /// rule `World::issue_command` documents. Saturating arithmetic
    /// throughout, so even `fill(u32::MAX)` is just "fill it up".
    pub fn fill(&mut self, amount: u32) -> u32 {
        let room = self.capacity.saturating_sub(self.quantity);
        let added = amount.min(room);
        self.quantity += added;
        added
    }

    /// Moves **exactly one** unit onto `belt`, if this hopper has one and
    /// the belt has room for it at its start. Returns whether a unit moved.
    ///
    /// One unit per call (and `World::tick` calls it once per tick) is the
    /// whole point of the hopper: the line's feed rate becomes a property of
    /// the machinery, not of how fast the player can press a key. In
    /// practice the belt's own min-spacing rule is the binding constraint —
    /// a unit can only be placed once the previous one has moved clear — so
    /// a full hopper feeds the belt at exactly the rate the belt can accept,
    /// and stops dead when the line backs up.
    pub fn try_unload(&mut self, belt: &mut BeltSegment) -> bool {
        if self.quantity == 0 {
            return false;
        }
        // `try_spawn` is both the room check and the commit, and is a no-op
        // when there's no room — so this is a single source of truth for
        // belt spacing, not a duplicate of it.
        if !belt.try_spawn(self.item.clone()) {
            return false;
        }
        self.quantity -= 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::types::Fx;

    fn belt() -> BeltSegment {
        BeltSegment::new(Fx::from_num(4.0), Fx::from_num(0.25))
    }

    fn hopper() -> Hopper {
        Hopper::new("tomato".to_string(), HOPPER_CAPACITY)
    }

    #[test]
    fn fill_reports_what_it_added_and_clamps_at_capacity() {
        let mut h = hopper();
        assert_eq!(h.fill(10), 10);
        assert_eq!(h.quantity(), 10);
        assert_eq!(
            h.fill(HOPPER_CAPACITY),
            HOPPER_CAPACITY - 10,
            "only the remaining room is accepted"
        );
        assert_eq!(h.quantity(), HOPPER_CAPACITY);
        assert_eq!(h.fill(1), 0, "filling a full hopper adds nothing");
        assert_eq!(h.quantity(), HOPPER_CAPACITY);
    }

    /// Overfilling must be a clamp, never a panic or a wrapped count — the
    /// command that reaches this is player input, and `issue_command`'s
    /// contract is that an impossible command is a silent no-op.
    #[test]
    fn filling_far_past_capacity_is_clamped_not_overflowed() {
        let mut h = hopper();
        assert_eq!(h.fill(u32::MAX), HOPPER_CAPACITY);
        assert_eq!(h.quantity(), HOPPER_CAPACITY);
        assert_eq!(h.fill(u32::MAX), 0);
        assert_eq!(h.quantity(), HOPPER_CAPACITY);
    }

    #[test]
    fn an_empty_hopper_never_puts_anything_on_the_belt() {
        let mut h = hopper();
        let mut b = belt();
        for _ in 0..100 {
            assert!(!h.try_unload(&mut b));
            b.tick(Fx::from_num(0.02));
        }
        assert!(b.is_empty());
    }

    #[test]
    fn unloads_exactly_one_unit_per_tick_while_the_belt_has_room() {
        let mut h = hopper();
        h.fill(3);
        let mut b = belt();

        // First unit goes straight on; the next has to wait out the belt's
        // min-spacing, so "one per tick" is an upper bound the belt gates.
        assert!(h.try_unload(&mut b));
        assert_eq!(h.quantity(), 2);
        assert_eq!(b.len(), 1, "exactly one unit moved, not the whole buffer");

        let mut moved = 1;
        for _ in 0..200 {
            b.tick(Fx::from_num(0.02));
            if h.try_unload(&mut b) {
                moved += 1;
            }
        }
        assert_eq!(moved, 3, "all three units eventually reached the belt");
        assert!(h.is_empty());
    }

    #[test]
    fn auto_fill_is_off_on_a_new_hopper_and_survives_a_retarget() {
        let mut h = hopper();
        assert!(!h.auto_fill(), "a new hopper is loaded by hand");
        h.set_auto_fill(true);
        h.fill(10);

        h.retarget("sugar".to_string());
        assert!(
            h.auto_fill(),
            "\"keep this bin full\" is about the bin, not about what is in it"
        );
        assert_eq!(h.quantity(), 0, "retargeting still clears the contents");
    }

    #[test]
    fn room_is_what_a_top_up_asks_for() {
        let mut h = hopper();
        assert_eq!(h.room(), HOPPER_CAPACITY);
        h.fill(10);
        assert_eq!(h.room(), HOPPER_CAPACITY - 10);
        h.fill(HOPPER_CAPACITY);
        assert_eq!(h.room(), 0, "a full hopper asks for nothing");
    }

    /// The belt-full case, mirroring `belt.rs`'s own
    /// `cannot_spawn_on_top_of_an_unmoved_item` test: nothing ticks the
    /// belt, so the first unit never clears the start and the hopper must
    /// simply hold everything else.
    #[test]
    fn holds_its_contents_when_the_belt_has_no_room() {
        let mut h = hopper();
        h.fill(5);
        let mut b = belt();

        assert!(h.try_unload(&mut b));
        for _ in 0..500 {
            assert!(
                !h.try_unload(&mut b),
                "a blocked belt must not accept a second unit"
            );
        }
        assert_eq!(h.quantity(), 4, "nothing was consumed while blocked");
        assert_eq!(b.len(), 1);
    }
}
