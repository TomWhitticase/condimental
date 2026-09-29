//! `Splitter` — one belt in, two or three belts out (Epic B, ticket 1).
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
//! # Where its outputs are (the direction convention, and the four modes)
//!
//! A [`super::world::Placement`] carries exactly one `direction`, and a
//! splitter does not get a second one. Every output is derived from it:
//!
//! - **Straight** is the placement's own `direction`.
//! - **Left** is one quarter turn clockwise from it ([`Direction::rotate_cw`]
//!   — the same turn [`super::world::PlayerCommand`] `Rotate` applies).
//! - **Right** is one quarter turn the other way.
//!
//! **Why the sim's clockwise turn is the player's left.** The renderer lays
//! tile `(x, y)` at world `(x, 0, y)` with Godot's Y up, and that mapping is a
//! reflection: a turn that is clockwise in the sim's `x`-east, `y`-north plane
//! is anticlockwise on screen. So the output this module has always derived
//! with `rotate_cw` leaves by the traveller's *left* as the player sees it —
//! which is what the owner reported ("splitters only split to the left",
//! 2026-09-29) and why the sides are named for what is drawn rather than for
//! the sim's own turn.
//!
//! Which of those three a splitter actually deals to is its [`SplitMode`],
//! the one thing about it a player sets (`PlayerCommand::SetSplitterMode`,
//! from the splitter's configuration panel):
//!
//! | mode         | deals to                 |
//! |--------------|--------------------------|
//! | `left`       | straight, left (default) |
//! | `right`      | straight, right          |
//! | `t_junction` | left, right              |
//! | `three_way`  | straight, left, right    |
//!
//! Until 2026-09-29 there was no mode and every splitter was `left`; it is
//! still the default, so a splitter placed today behaves exactly as one
//! placed before the mode existed, and a migrated save is unchanged.
//!
//! Deriving every output from the one facing keeps the property the original
//! convention was chosen for: **rotating turns all of a splitter's outputs
//! together, for free**, and there is no second facing that could get out of
//! step with the first. The mode picks *which* derived sides are live; it
//! never stores a direction.
//!
//! **What a splitter accepts, and from where, is deliberately not gated by
//! its facing or its mode.** A splitter is belt-like: anything that hands to
//! a belt hands to a splitter — a belt pointing at it, a machine discharging
//! onto it, a hopper metering onto it. That is not laxness, it is the 1x1
//! rule this whole codebase already has (a 1x1 placement's inlet face *is*
//! its only tile, so a dock and a hopper behave the same way), and it keeps a
//! splitter usable as the corner piece of a line rather than only as a
//! straight one. The consequence worth knowing: an output may point back at
//! the belt that feeds it, and then items go round in a circle. So do two
//! ordinary belts pointing at each other — the simulation has never forbidden
//! that, and inventing a rule against it here would be new code enforcing
//! something no design doc states.
//!
//! # The distribution rule: round-robin, with fallback
//!
//! `data/machines.ron` and the shop make a splitter *content*; this is the
//! one piece of behaviour that is genuinely code, so it is stated here once.
//!
//! **A splitter offers its front item to the side that is due, and if that
//! side cannot take it, to each of its mode's other sides in turn, in the
//! same tick. Whichever side actually takes it, the side after that one (in
//! the mode's order, [`SplitMode::sides`]) becomes due.**
//!
//! - With every output clear it is a strict rotation — A, B, A, B for two
//!   sides, A, B, C, A, B, C for three — which is what "splits its output"
//!   has to mean for the throughput of a fed line to divide evenly.
//! - The fallback is what makes it a splitter rather than a metering valve.
//!   Factorio's splitters behave the same way, for the same reason: if one
//!   branch backs up, everything should go down the other one instead of the
//!   whole line stalling on a full buffer. A splitter that insisted on its
//!   turn would be a throughput bug the player could not see.
//! - Advancing the cursor **past whichever side took it** (rather than only
//!   past the side that was due) is what makes fairness resume by itself.
//!   With two sides, while one is blocked every item goes down the open one
//!   and the blocked one stays permanently due, so the first item after it
//!   clears goes to it. With three, a blocked side is due at least every
//!   other item, so it is never starved either. No catch-up counter.
//! - A side with nothing standing on it is simply a side that always
//!   refuses, so **a splitter with one connected output feeds that one** and
//!   **a splitter with none holds everything** and backs the line up behind
//!   it. Both fall out of the rule; neither is a special case. A side the
//!   mode does not deal to is not offered at all, whatever stands on it.
//!
//! At most one item leaves a splitter per tick, exactly as at most one item
//! crosses any other hand-off in this simulation per tick.
//!
//! # Why the buffer is a `BeltSegment`, not a bespoke one-slot type
//!
//! A splitter holds what it has accepted in an ordinary
//! [`BeltSegment`](super::belt::BeltSegment) of exactly one tile, with the
//! same item spacing as every other belt tile — the [`Splitter`] component
//! below holds *only* the mode and the round-robin cursor.
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
//! 3. **It costs nothing.** The mode and the cursor are two enums; the
//!    storage is a type that already exists and is already tested.

use super::types::Direction;

/// The buildable-kind id (`data/machines.ron`, `data/shop.ron`) of a
/// conveyor splitter. Spelled out in Rust for the reason
/// [`HOPPER_KIND`](super::hopper::HOPPER_KIND) and
/// [`DOCK_KIND`](super::dock::DOCK_KIND) are: the simulation has to
/// construct a distinct body for it, which a kind id alone cannot express.
pub const SPLITTER_KIND: &str = "splitter";

/// Which of a splitter's outputs — see this module's doc for where each one
/// points.
///
/// An enum rather than an index so no call site can invent a fourth output
/// or confuse two of them. The wire and the save both spell these
/// `"straight"`, `"left"`, `"right"`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The output on the splitter's own facing — where a belt tile in its
    /// place would have carried.
    Straight,
    /// One quarter turn clockwise from the facing in the sim, which is the
    /// player's left on screen (see this module's doc for why).
    Left,
    /// One quarter turn anticlockwise from the facing: the player's right.
    Right,
}

impl Default for Side {
    /// `Straight` — a freshly placed splitter offers its first item to the
    /// way it is pointing, so a splitter with its side outputs not yet built
    /// behaves like the belt tile it replaced from its very first item.
    fn default() -> Self {
        Side::Straight
    }
}

impl Side {
    /// Every side, in the order a snapshot's `outputs` lists them.
    pub const ALL: [Side; 3] = [Side::Straight, Side::Left, Side::Right];

    /// Index into a three-element output array. `Straight` is 0, `Left` 1,
    /// `Right` 2 — [`Self::ALL`]'s order.
    pub fn index(self) -> usize {
        match self {
            Side::Straight => 0,
            Side::Left => 1,
            Side::Right => 2,
        }
    }

    /// The world direction this side leaves by, for a splitter facing
    /// `facing`. **This is the whole of the geometry** — every other layer
    /// (topology linking, snapshots, and the renderer's arrow) follows this
    /// rather than restating it.
    pub fn direction_from(self, facing: Direction) -> Direction {
        match self {
            Side::Straight => facing,
            Side::Left => facing.rotate_cw(),
            Side::Right => facing.rotate_cw().opposite(),
        }
    }
}

/// Which of its outputs a splitter deals to — the player's one setting on
/// it (see the table in this module's doc).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SplitMode {
    /// Straight on, and off to the left: what every splitter was before the
    /// mode existed.
    Left,
    /// Straight on, and off to the right.
    Right,
    /// Left and right, nothing straight on.
    TJunction,
    /// Straight on, left and right.
    ThreeWay,
}

impl Default for SplitMode {
    /// `Left` — the one behaviour a splitter had before it could be set, so
    /// a new one and a migrated one both carry on as they always did.
    fn default() -> Self {
        SplitMode::Left
    }
}

impl SplitMode {
    /// Every mode, in the order the configuration panel lists them.
    pub const ALL: [SplitMode; 4] = [
        SplitMode::Left,
        SplitMode::Right,
        SplitMode::TJunction,
        SplitMode::ThreeWay,
    ];

    /// The sides this mode deals to, **in round-robin order**. Always a
    /// subsequence of [`Side::ALL`], so "straight, then left, then right"
    /// is the one order a splitter ever deals in.
    pub fn sides(self) -> &'static [Side] {
        match self {
            SplitMode::Left => &[Side::Straight, Side::Left],
            SplitMode::Right => &[Side::Straight, Side::Right],
            SplitMode::TJunction => &[Side::Left, Side::Right],
            SplitMode::ThreeWay => &[Side::Straight, Side::Left, Side::Right],
        }
    }

    /// Whether this mode deals to `side` at all.
    pub fn deals_to(self, side: Side) -> bool {
        self.sides().contains(&side)
    }
}

/// A placed splitter's state: its mode, and which of that mode's sides is
/// due next.
///
/// What it is *holding* is not here — that is the [`BeltSegment`] this
/// entity also carries (see this module's doc for why it is a belt segment
/// rather than a bespoke slot).
///
/// **`due` is always one of `mode`'s sides.** Every constructor and
/// [`Self::set_mode`] keep it so, which is what lets [`Self::offer_order`]
/// start from it without a check.
///
/// [`BeltSegment`]: super::belt::BeltSegment
#[derive(Debug, Clone, Copy, Default)]
pub struct Splitter {
    mode: SplitMode,
    due: Side,
}

impl Splitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// What this splitter deals to.
    pub fn mode(&self) -> SplitMode {
        self.mode
    }

    /// The side that gets first refusal on the next item.
    pub fn due(&self) -> Side {
        self.due
    }

    /// Switches the mode. The cursor stays where it is if the new mode still
    /// deals to that side, and otherwise starts from the new mode's first
    /// side — so going from `left` to `three_way` does not disturb the
    /// rotation, and going to `t_junction` never leaves `straight` due on a
    /// splitter that no longer deals straight on.
    pub fn set_mode(&mut self, mode: SplitMode) {
        self.mode = mode;
        if !mode.deals_to(self.due) {
            self.due = mode.sides()[0];
        }
    }

    /// The order to offer this tick's item in: the due side, then the rest
    /// of the mode's sides in rotation. The caller stops at the first side
    /// that takes it — every entry after the first *is* the fallback
    /// described in this module's doc.
    pub fn offer_order(&self) -> impl Iterator<Item = Side> {
        let sides = self.mode.sides();
        let start = Self::position(sides, self.due);
        (0..sides.len()).map(move |i| sides[(start + i) % sides.len()])
    }

    /// Rebuilds a splitter in `mode` whose next-due side is `due` — the
    /// save/load module's seam (`sim::save`), and the only way to set the
    /// cursor other than by actually handing an item over. `pub(crate)` on
    /// purpose: a player chooses a splitter's mode, never which way it deals
    /// next. A `due` the mode does not deal to (a hand-edited file) starts
    /// from the mode's first side, [`Self::set_mode`]'s rule.
    pub(crate) fn restore(mode: SplitMode, due: Side) -> Self {
        let mut splitter = Self { mode, due };
        splitter.set_mode(mode);
        splitter
    }

    /// Records that `side` actually took an item, whether or not it was the
    /// side that was due. The side after it in the rotation becomes due,
    /// which is what makes a blocked side one of the next served the moment
    /// it clears.
    pub fn record_output(&mut self, side: Side) {
        let sides = self.mode.sides();
        self.due = sides[(Self::position(sides, side) + 1) % sides.len()];
    }

    fn position(sides: &[Side], side: Side) -> usize {
        sides
            .iter()
            .position(|&s| s == side)
            .expect("a splitter's cursor is always one of its mode's sides")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The direction convention this module exists to state: straight is the
    /// facing, left is a quarter turn clockwise in the sim, right the other
    /// quarter turn, and no two sides are ever the same tile.
    #[test]
    fn the_three_sides_are_the_facing_and_its_two_quarter_turns() {
        for facing in Direction::ALL {
            assert_eq!(Side::Straight.direction_from(facing), facing);
            assert_eq!(Side::Left.direction_from(facing), facing.rotate_cw());
            assert_eq!(
                Side::Right.direction_from(facing).rotate_cw(),
                facing,
                "right is one quarter turn anticlockwise"
            );
            for a in Side::ALL {
                for b in Side::ALL {
                    if a != b {
                        assert_ne!(a.direction_from(facing), b.direction_from(facing));
                    }
                }
            }
        }
    }

    /// Rotating the placement rotates every output together — the property
    /// that means a splitter needs no second direction field.
    #[test]
    fn rotating_the_placement_turns_every_output() {
        for facing in Direction::ALL {
            let turned = facing.rotate_cw();
            for side in Side::ALL {
                assert_eq!(
                    side.direction_from(turned),
                    side.direction_from(facing).rotate_cw()
                );
            }
        }
    }

    fn deal(splitter: &mut Splitter, items: usize) -> Vec<Side> {
        (0..items)
            .map(|_| {
                let side = splitter.offer_order().next().unwrap();
                splitter.record_output(side);
                side
            })
            .collect()
    }

    /// With every side taking, each mode rotates strictly through its own
    /// sides and never offers one it does not deal to.
    #[test]
    fn every_mode_rotates_through_exactly_its_sides() {
        use Side::*;
        let cases = [
            (SplitMode::Left, vec![Straight, Left, Straight, Left]),
            (SplitMode::Right, vec![Straight, Right, Straight, Right]),
            (SplitMode::TJunction, vec![Left, Right, Left, Right]),
            (
                SplitMode::ThreeWay,
                vec![Straight, Left, Right, Straight, Left, Right],
            ),
        ];
        for (mode, expected) in cases {
            let mut splitter = Splitter::new();
            splitter.set_mode(mode);
            assert_eq!(deal(&mut splitter, expected.len()), expected, "{mode:?}");
            for side in Side::ALL {
                let offered = splitter.offer_order().any(|s| s == side);
                assert_eq!(offered, mode.deals_to(side), "{mode:?} {side:?}");
            }
        }
    }

    /// The fallback: when the due side refuses, the item goes down the other
    /// one — and the refusing side stays due, so it is served first the
    /// moment it clears.
    #[test]
    fn a_blocked_side_stays_due_and_is_served_the_moment_it_clears() {
        let mut splitter = Splitter::new();
        // Straight is due, but blocked. Six items all go left.
        for _ in 0..6 {
            let order: Vec<Side> = splitter.offer_order().collect();
            assert_eq!(order, vec![Side::Straight, Side::Left]);
            // The caller's loop: straight refuses, left takes it.
            splitter.record_output(order[1]);
        }
        // Straight clears. The next item is its.
        assert_eq!(splitter.offer_order().next(), Some(Side::Straight));
    }

    /// Three ways, one blocked: the other two share every item and the
    /// blocked one is still offered first every other item, so it is served
    /// within one item of clearing.
    #[test]
    fn a_blocked_side_of_three_is_never_starved() {
        let mut splitter = Splitter::new();
        splitter.set_mode(SplitMode::ThreeWay);
        let mut firsts = Vec::new();
        let mut taken = Vec::new();
        for _ in 0..6 {
            let order: Vec<Side> = splitter.offer_order().collect();
            firsts.push(order[0]);
            let side = *order.iter().find(|&&s| s != Side::Straight).unwrap();
            splitter.record_output(side);
            taken.push(side);
        }
        assert!(!taken.contains(&Side::Straight));
        assert_eq!(
            taken.iter().filter(|&&s| s == Side::Left).count(),
            taken.iter().filter(|&&s| s == Side::Right).count()
        );
        assert!(
            firsts.chunks(2).all(|pair| pair.contains(&Side::Straight)),
            "straight is due at least every other item: {firsts:?}"
        );
    }

    /// A fresh splitter is `left` and points its first item straight
    /// through, so one dropped into a line before its branch exists behaves
    /// like the belt tile it replaced — and like every splitter did before
    /// the mode existed.
    #[test]
    fn a_fresh_splitter_is_left_and_offers_straight_first() {
        assert_eq!(Splitter::new().mode(), SplitMode::Left);
        assert_eq!(Splitter::new().due(), Side::Straight);
        assert_eq!(Side::default(), Side::Straight);
        assert_eq!(SplitMode::default(), SplitMode::Left);
    }

    /// Switching mode never leaves the cursor on a side the new mode does not
    /// deal to, and keeps it where it was when it can.
    #[test]
    fn switching_mode_keeps_the_cursor_on_a_live_side() {
        let mut splitter = Splitter::new();
        splitter.set_mode(SplitMode::TJunction);
        assert_eq!(splitter.due(), Side::Left, "straight is not dealt to");
        splitter.record_output(Side::Left);
        assert_eq!(splitter.due(), Side::Right);
        splitter.set_mode(SplitMode::ThreeWay);
        assert_eq!(splitter.due(), Side::Right, "still live, so undisturbed");
        splitter.set_mode(SplitMode::Left);
        assert_eq!(splitter.due(), Side::Straight);
        for mode in SplitMode::ALL {
            for due in Side::ALL {
                let restored = Splitter::restore(mode, due);
                assert!(mode.deals_to(restored.due()), "{mode:?} {due:?}");
            }
        }
    }

    #[test]
    fn sides_and_modes_have_stable_wire_names() {
        assert_eq!(Side::ALL, [Side::Straight, Side::Left, Side::Right]);
        for (i, side) in Side::ALL.into_iter().enumerate() {
            assert_eq!(side.index(), i);
        }
        let names: Vec<String> = Side::ALL
            .iter()
            .map(|s| serde_json::to_string(s).unwrap())
            .collect();
        assert_eq!(names, ["\"straight\"", "\"left\"", "\"right\""]);
        let modes: Vec<String> = SplitMode::ALL
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect();
        assert_eq!(
            modes,
            ["\"left\"", "\"right\"", "\"t_junction\"", "\"three_way\""]
        );
    }
}
