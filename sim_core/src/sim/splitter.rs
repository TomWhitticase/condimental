//! `Splitter` — one belt in, two belts out (Epic B, ticket 1).
//!
//! The first placeable kind in this game whose whole purpose is that a
//! production line stops being a *line*. Until now every hand-off in the
//! simulation was one-to-one: a belt looks at the single tile its own
//! [`Direction`] points at and gives its front item to whatever is standing
//! there ([`super::world::World::recompute_topology`]'s `belt_to_belt` /
//! `belt_to_sink` links). A splitter is the one thing that resolves to
//! **two** downstream targets, which is what lets one crusher feed two
//! mixers, or one line be tapped without being cut.
//!
//! It is an ordinary placeable kind, not a special-cased belt variant: a
//! 1x1 [`Footprint`](super::machines::Footprint) in `data/machines.ron` and
//! a price in `data/shop.ron`, bought with `purchase`, put down with
//! `place`, turned with `rotate` and picked up with `remove`, exactly like
//! every other building in the game. It has no recipe, which puts it in the
//! same category as the hopper, the fluid tank and the dock (see
//! `machines.ron`'s own header) — it moves product, it does not change it.
//!
//! # Where its two outputs are (the direction convention)
//!
//! A [`super::world::Placement`] carries exactly one `direction`, and a
//! splitter does not get a second field. Instead:
//!
//! **The placement's own `direction` is the STRAIGHT-THROUGH output, and
//! the second output is one quarter turn clockwise from it**
//! ([`Direction::rotate_cw`] — the same turn [`super::world::PlayerCommand`]
//! `Rotate` applies, so "clockwise" means one thing in this codebase).
//!
//! Three reasons this is the rule rather than, say, "both flanks" or a
//! stored pair of directions:
//!
//! 1. **A splitter drops into an existing line without re-planning it.**
//!    Placed facing the way the belts around it already run, it carries
//!    product exactly where a belt tile would have; the branch is the
//!    addition. The player never has to work out which of two stored
//!    facings is "the main one".
//! 2. **Rotating turns both outputs together, for free.** A quarter turn
//!    moves the straight output to where the branch was and the branch to
//!    the next side round, because both are derived from the one field
//!    `Rotate` already changes. There is no second facing that could get out
//!    of step with the first, and no new command.
//! 3. **The four facings give all four L-shapes.** Straight+clockwise at
//!    each of the four directions covers every "carry on, and also branch to
//!    one side" arrangement; a player who wants the branch on the other side
//!    faces the splitter the other way and lets the straight run take the
//!    corner. Nothing is unreachable.
//!
//! **What a splitter accepts, and from where, is deliberately not gated by
//! its facing.** A splitter is belt-like: anything that hands to a belt
//! hands to a splitter — a belt pointing at it, a machine discharging onto
//! it, a hopper metering onto it. That is not laxness, it is the 1x1 rule
//! this whole codebase already has (a 1x1 placement's inlet face *is* its
//! only tile, so a dock and a hopper behave the same way), and it keeps a
//! splitter usable as the corner piece of a line rather than only as a
//! straight one. The consequence worth knowing: a splitter's branch output
//! may point back at the belt that feeds it, and then items go round in a
//! circle. So do two ordinary belts pointing at each other — the simulation
//! has never forbidden that, and inventing a rule against it here would be
//! new code enforcing something no design doc states.
//!
//! # The distribution rule: round-robin, with fallback
//!
//! `data/machines.ron` and the shop make a splitter *content*; this is the
//! one piece of behaviour that is genuinely code, so it is stated here once.
//!
//! **A splitter offers its front item to the side that is due, and if that
//! side cannot take it, to the other side in the same tick. Whichever side
//! actually takes it, the other becomes due.**
//!
//! - With both outputs clear it is a strict alternation — A, B, A, B — which
//!   is what "splits its output across two outputs" has to mean for the
//!   throughput of a fed line to divide in half.
//! - The fallback is what makes it a splitter rather than a metering valve.
//!   Factorio's splitters behave the same way, for the same reason: if one
//!   branch backs up, everything should go down the other one instead of the
//!   whole line stalling on a full buffer. A splitter that insisted on its
//!   turn would be a throughput bug the player could not see.
//! - Advancing the cursor **on whichever side took it** (rather than only on
//!   the side that was due) is what makes fairness resume by itself: while
//!   one branch is blocked, every item goes down the open one and the
//!   blocked one stays permanently due, so the first item after it clears
//!   goes to it. No catch-up counter, no starvation.
//! - A side with nothing standing on it is simply a side that always
//!   refuses, so **a splitter with one output feeds that one** and **a
//!   splitter with no outputs holds everything** and backs the line up
//!   behind it. Both fall out of the rule; neither is a special case.
//!
//! At most one item leaves a splitter per tick, exactly as at most one item
//! crosses any other hand-off in this simulation per tick.
//!
//! # Why the buffer is a `BeltSegment`, not a bespoke one-slot type
//!
//! A splitter holds what it has accepted in an ordinary
//! [`BeltSegment`](super::belt::BeltSegment) of exactly one tile, with the
//! same item spacing as every other belt tile — the [`Splitter`] component
//! below holds *only* the round-robin cursor.
//!
//! 1. **A splitter is a belt tile with a choice at the end of it.** Reusing
//!    the belt's own gap-based storage (ARCHITECTURE.md §3.3) means an item
//!    crosses a splitter in the same 50 ticks it would cross a belt, so
//!    inserting one into a line changes where product goes and never how
//!    fast it travels. A one-slot buffer with instant transit would make a
//!    line with a splitter measurably *quicker* than the same line without
//!    one, which is a physics rule nobody asked for.
//! 2. **Every existing hand-off works on it verbatim.** Belt-to-splitter is
//!    `try_transfer_belt_to_belt`, machine-to-splitter is
//!    `try_transfer_machine_output_to_belt`, hopper-to-splitter is
//!    `Hopper::try_unload` — all three already take a `&mut BeltSegment`,
//!    and all three inherit the "spawn first, then take" ordering that makes
//!    a refused hop leave the item where it was instead of annihilating it.
//!    A bespoke buffer would mean writing that ordering a fourth time, in a
//!    place the mass-conservation tests would only catch by luck.
//! 3. **It costs nothing.** The cursor is one enum; the storage is a type
//!    that already exists and is already tested.

use super::types::Direction;

/// The buildable-kind id (`data/machines.ron`, `data/shop.ron`) of a
/// conveyor splitter. Spelled out in Rust for the reason
/// [`HOPPER_KIND`](super::hopper::HOPPER_KIND) and
/// [`DOCK_KIND`](super::dock::DOCK_KIND) are: the simulation has to
/// construct a distinct body for it, which a kind id alone cannot express.
pub const SPLITTER_KIND: &str = "splitter";

/// Which of a splitter's two outputs — see this module's doc for where each
/// one points.
///
/// An enum rather than an index so no call site can invent a third output or
/// confuse the two: `Straight` is always the placement's own facing and
/// `Branch` is always one quarter turn clockwise from it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The output on the splitter's own facing — where a belt tile in its
    /// place would have carried.
    Straight,
    /// The output one quarter turn clockwise from the facing.
    Branch,
}

impl Default for Side {
    /// `Straight` — a freshly placed splitter offers its first item to the
    /// way it is pointing, so a splitter with one branch not yet built
    /// behaves like the belt tile it replaced from its very first item.
    fn default() -> Self {
        Side::Straight
    }
}

impl Side {
    /// Both sides, in the order a snapshot lists them: straight first.
    pub const BOTH: [Side; 2] = [Side::Straight, Side::Branch];

    /// Index into a two-element output array. `Straight` is 0.
    pub fn index(self) -> usize {
        match self {
            Side::Straight => 0,
            Side::Branch => 1,
        }
    }

    /// The other side.
    pub fn other(self) -> Side {
        match self {
            Side::Straight => Side::Branch,
            Side::Branch => Side::Straight,
        }
    }

    /// The world direction this side leaves by, for a splitter facing
    /// `facing`. **This is the whole of the geometry** — every other layer
    /// (topology linking, snapshots, and eventually the renderer) calls this
    /// rather than restating the `rotate_cw` rule.
    pub fn direction_from(self, facing: Direction) -> Direction {
        match self {
            Side::Straight => facing,
            Side::Branch => facing.rotate_cw(),
        }
    }
}

/// A placed splitter's only state: which output is due next.
///
/// What it is *holding* is not here — that is the [`BeltSegment`] this
/// entity also carries (see this module's doc for why it is a belt segment
/// rather than a bespoke slot).
///
/// [`BeltSegment`]: super::belt::BeltSegment
#[derive(Debug, Clone, Copy, Default)]
pub struct Splitter {
    due: Side,
}

impl Splitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// The side that gets first refusal on the next item.
    pub fn due(&self) -> Side {
        self.due
    }

    /// The order to offer this tick's item in: the due side, then the other
    /// one. The caller stops at the first side that takes it — that second
    /// entry *is* the fallback described in this module's doc.
    pub fn offer_order(&self) -> [Side; 2] {
        [self.due, self.due.other()]
    }

    /// Rebuilds a splitter whose next-due side is `due` — the save/load
    /// module's seam (`sim::save`), and the only way to set this field
    /// other than by actually handing an item over. `pub(crate)` on purpose:
    /// a player cannot choose which way a splitter deals next, only a
    /// loader legitimately can.
    pub(crate) fn restore(due: Side) -> Self {
        Self { due }
    }

    /// Records that `side` actually took an item, whether or not it was the
    /// side that was due. The other side becomes due, which is what makes a
    /// blocked branch the next one served the moment it clears.
    pub fn record_output(&mut self, side: Side) {
        self.due = side.other();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The direction convention this module exists to state: straight is the
    /// facing, the branch is a quarter turn clockwise, and four facings
    /// therefore reach all four L-shapes.
    #[test]
    fn the_branch_is_one_clockwise_turn_from_the_facing() {
        for facing in Direction::ALL {
            assert_eq!(Side::Straight.direction_from(facing), facing);
            assert_eq!(Side::Branch.direction_from(facing), facing.rotate_cw());
            assert_ne!(
                Side::Straight.direction_from(facing),
                Side::Branch.direction_from(facing),
                "a splitter's two outputs are never the same tile"
            );
        }
    }

    /// Rotating the placement rotates both outputs together — the property
    /// that means a splitter needs no second direction field.
    #[test]
    fn rotating_the_placement_turns_both_outputs() {
        for facing in Direction::ALL {
            let turned = facing.rotate_cw();
            assert_eq!(
                Side::Straight.direction_from(turned),
                Side::Branch.direction_from(facing),
                "the branch becomes the straight-through after one turn"
            );
        }
    }

    /// With both sides taking, the cursor alternates strictly.
    #[test]
    fn a_fed_splitter_alternates() {
        let mut splitter = Splitter::new();
        let mut taken = Vec::new();
        for _ in 0..6 {
            let side = splitter.offer_order()[0];
            taken.push(side);
            splitter.record_output(side);
        }
        assert_eq!(
            taken,
            vec![
                Side::Straight,
                Side::Branch,
                Side::Straight,
                Side::Branch,
                Side::Straight,
                Side::Branch,
            ]
        );
    }

    /// The fallback: when the due side refuses, the item goes down the other
    /// one — and the refusing side stays due, so it is served first the
    /// moment it clears.
    #[test]
    fn a_blocked_side_stays_due_and_is_served_the_moment_it_clears() {
        let mut splitter = Splitter::new();
        // Straight is due, but blocked. Six items all go down the branch.
        for _ in 0..6 {
            let order = splitter.offer_order();
            assert_eq!(order[0], Side::Straight, "the blocked side stays due");
            // The caller's loop: straight refuses, the branch takes it.
            splitter.record_output(order[1]);
        }
        // Straight clears. The next item is its.
        assert_eq!(splitter.offer_order()[0], Side::Straight);
    }

    /// A fresh splitter points its first item straight through, so one
    /// dropped into a line before its branch exists behaves like the belt
    /// tile it replaced.
    #[test]
    fn a_fresh_splitter_offers_straight_first() {
        assert_eq!(Splitter::new().due(), Side::Straight);
        assert_eq!(Side::default(), Side::Straight);
    }

    #[test]
    fn sides_are_a_stable_two_element_set() {
        assert_eq!(Side::BOTH, [Side::Straight, Side::Branch]);
        assert_eq!(Side::Straight.index(), 0);
        assert_eq!(Side::Branch.index(), 1);
        assert_eq!(Side::Straight.other(), Side::Branch);
        assert_eq!(Side::Branch.other(), Side::Straight);
        assert_eq!(
            serde_json::to_string(&Side::Straight).unwrap(),
            "\"straight\""
        );
    }
}
