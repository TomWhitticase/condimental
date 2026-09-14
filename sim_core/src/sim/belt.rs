//! `BeltSegment` (ARCHITECTURE.md §3.2, §3.3) — gap-based item storage.
//!
//! Directly applies Factorio's FFF #176 trick: instead of storing each
//! item's absolute position, store the *gap* between it and the item ahead
//! of it (or the belt's end, for the frontmost item). `tick()` only ever
//! has to update the boundary where slack still exists — once a run of
//! items is fully compressed (every gap == 0), nothing behind that boundary
//! needs to be touched at all.
//!
//! Two things make the "gap" representation exact rather than approximate:
//!
//! - **Item footprint.** Two items can never be closer than `min_spacing`
//!   (their physical footprint) — for an item at index >= 1, `gap_ahead`
//!   stores the *slack beyond* `min_spacing`, not the raw distance, so
//!   `gap_ahead == 0` means "touching, fully compressed," never
//!   "overlapping." The frontmost item (index 0) has no item ahead of it,
//!   only the belt's end, so its `gap_ahead` is a plain distance with no
//!   footprint offset.
//! - **The index-0/index>=1 boundary shifts when an item exits.** When the
//!   front item is removed (handed to whatever is downstream), the new
//!   front item's stored gap must gain back the `min_spacing` it no longer
//!   needs to reserve against a leader — see `take_front_if_at_end`.
//!
//! This module also makes one physical assumption, same as Factorio's own
//! belts: **`speed` (the max an item advances in one tick) must never
//! exceed `min_spacing`.** Under that assumption, freed space can ripple
//! back by at most one item per tick, which is what makes the `frontier`
//! bookkeeping in `tick()` sound.
//!
//! # The footprint does not stop at the belt's end
//!
//! A placed belt is one tile, and a line is a chain of them handing items
//! across their shared boundary (`world.rs`'s belt-to-belt hop). The
//! frontmost item's `gap_ahead` is its distance to the belt's END, so left
//! alone it runs right up to the boundary — while the tile ahead may be
//! holding an item at ITS start, which is the very same point. On a jammed
//! line that put two items on every tile boundary (five per tile at a
//! quarter-tile spacing: 0, ¼, ½, ¾ and 1, with the 1 of one tile the 0 of
//! the next). Invisible while the spacing was narrower than the items
//! themselves; the moment items were spaced to be told apart, every
//! boundary drew a doubled item. So the front item respects the footprint
//! of what is on the tile ahead too: [`BeltSegment::tick_toward`] takes
//! the downstream segment's [`BeltSegment::room_at_start`] and holds the
//! front item `min_spacing - room` short of the end while that room is
//! under `min_spacing`. The hop itself is unchanged — `try_spawn` already
//! refused until the receiver had `min_spacing` of room, and now the sender
//! reaches its end at exactly the moment the receiver can take it. A fully
//! compressed line therefore holds `1 / min_spacing` items per tile, and
//! an item at a tile's start is never also drawn at the previous tile's end.

use std::collections::VecDeque;

use super::types::{Fx, ItemId};

/// One item currently on the belt.
#[derive(Debug, Clone)]
pub struct BeltItem {
    pub item: ItemId,
    /// For index 0: distance to the belt's end. For index >= 1: slack
    /// beyond `min_spacing` to the item ahead of it. See the module doc.
    pub gap_ahead: Fx,
}

#[derive(Debug)]
pub struct BeltSegment {
    /// Belt length in tile-units.
    pub length: Fx,
    /// Minimum physical distance between two items' centers.
    pub min_spacing: Fx,
    /// Front (index 0) is closest to the belt's end/output.
    items: VecDeque<BeltItem>,
    /// Index of the last item known to have a nonzero gap (or to be
    /// adjacent to freed space) as of the last `tick()`. Items beyond
    /// `frontier + 1` are guaranteed to have `gap_ahead == 0` and to not be
    /// reachable by this tick's freed space, so `tick()` only rescans
    /// `0..=frontier+1` instead of the whole belt — the actual FFF #176
    /// payoff, not just the storage shape.
    frontier: usize,
}

impl BeltSegment {
    pub fn new(length: Fx, min_spacing: Fx) -> Self {
        Self {
            length,
            min_spacing,
            items: VecDeque::new(),
            frontier: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Advances every item by up to `speed` tile-units, constrained by
    /// gaps. Returns whether anything actually moved (used for
    /// dirty-chunk tracking — ARCHITECTURE.md §4).
    ///
    /// Nothing is assumed about what lies beyond the belt's end: the front
    /// item may run right up to it. A belt that hands to another belt
    /// should use [`Self::tick_toward`] instead (the module doc's last
    /// section says why).
    pub fn tick(&mut self, speed: Fx) -> bool {
        self.tick_toward(speed, self.min_spacing)
    }

    /// [`Self::tick`] for a belt whose end meets the start of another one
    /// holding `room_ahead` of clear belt at that start (its
    /// [`Self::room_at_start`]). The front item stops
    /// `min_spacing - room_ahead` short of the end while that is positive,
    /// so it keeps its footprint clear of the item on the tile ahead exactly
    /// as it would of one on its own tile; with `room_ahead >= min_spacing`
    /// this is plain `tick`.
    ///
    /// The caller passes the room as it stood BEFORE any belt ticked this
    /// tick (`World::tick` snapshots every room first), so the result does
    /// not depend on the order belts are visited in. The cost is one tick
    /// of latency the first time an item crosses a boundary in a moving
    /// queue — the item behind settles `speed` further back than
    /// `min_spacing` — which is a fiftieth of a tile at the shipped speed
    /// and changes no throughput: every item pays it once.
    pub fn tick_toward(&mut self, speed: Fx, room_ahead: Fx) -> bool {
        if self.items.is_empty() {
            self.frontier = 0;
            return false;
        }
        let reserve = (self.min_spacing - room_ahead).max(Fx::ZERO);
        let upper = (self.frontier + 1).min(self.items.len() - 1);
        let mut freed = Fx::ZERO;
        let mut last_moved = 0usize;
        let mut moved = false;
        for i in 0..=upper {
            let item = &mut self.items[i];
            let effective_gap = item.gap_ahead + freed;
            // Only the front item has the belt's end (and whatever stands
            // just past it) ahead; everything behind it has an item ahead
            // and `gap_ahead` already reserves the footprint against that.
            let ceiling = if i == 0 {
                (effective_gap - reserve).max(Fx::ZERO)
            } else {
                effective_gap
            };
            let advance = speed.min(ceiling);
            item.gap_ahead = effective_gap - advance;
            freed = advance;
            if advance > Fx::ZERO {
                moved = true;
                last_moved = i;
            }
        }
        self.frontier = last_moved;
        moved
    }

    /// The item id at the front of the belt, if it has reached the belt's
    /// end (`gap_ahead == 0`) and is ready to be handed downstream.
    pub fn front_ready(&self) -> Option<&ItemId> {
        self.items
            .front()
            .filter(|front| front.gap_ahead == Fx::ZERO)
            .map(|front| &front.item)
    }

    /// If the frontmost item has reached the belt's end (`gap_ahead == 0`),
    /// removes and returns it. The caller (the world's per-tick update)
    /// decides whether it can actually be handed to whatever is
    /// downstream.
    pub fn take_front_if_at_end(&mut self) -> Option<BeltItem> {
        let ready = matches!(self.items.front(), Some(front) if front.gap_ahead == Fx::ZERO);
        if !ready {
            return None;
        }
        let item = self.items.pop_front();
        // The new front item no longer needs to reserve `min_spacing`
        // against a leader that's gone — fold that slack back in so its
        // gap becomes a plain index-0 distance-to-end again.
        if let Some(new_front) = self.items.front_mut() {
            new_front.gap_ahead += self.min_spacing;
        }
        self.frontier = 0;
        item
    }

    /// Literal distance from the belt's end to item `idx`, accounting for
    /// the footprint offset every item past index 0 carries.
    fn dist_from_end(&self, idx: usize) -> Fx {
        let mut cum = Fx::ZERO;
        for (k, it) in self.items.iter().enumerate().take(idx + 1) {
            cum += it.gap_ahead;
            if k >= 1 {
                cum += self.min_spacing;
            }
        }
        cum
    }

    /// How much clear belt there is at the start: the whole length when
    /// empty, otherwise the back-most item's distance from the start. This
    /// is what [`Self::try_spawn`] measures against `min_spacing`, and what
    /// the belt BEHIND this one passes to [`Self::tick_toward`] so its front
    /// item stops short of an item standing here (module doc).
    pub fn room_at_start(&self) -> Fx {
        match self.items.len() {
            0 => self.length,
            n => self.length - self.dist_from_end(n - 1),
        }
    }

    /// Attempts to place a new item at the belt's start. Fails (returns
    /// `false`, no-op) if the current back-most item hasn't moved at least
    /// `min_spacing` away from the start yet.
    pub fn try_spawn(&mut self, item: ItemId) -> bool {
        if self.items.is_empty() {
            self.items.push_back(BeltItem {
                item,
                gap_ahead: self.length,
            });
            self.frontier = 0;
            return true;
        }
        let back_dist_from_start = self.room_at_start();
        if back_dist_from_start < self.min_spacing {
            return false;
        }
        self.items.push_back(BeltItem {
            item,
            gap_ahead: back_dist_from_start - self.min_spacing,
        });
        self.frontier = self.items.len() - 1;
        true
    }

    /// How far back `tick` still has to scan — **restore-only state**, see
    /// [`BeltSegment::restore`] for why a save has to carry it.
    pub(crate) fn frontier(&self) -> usize {
        self.frontier
    }

    /// Rebuilds a belt from what [`Self::snapshot_positions`] reported, for
    /// the save/load module and nothing else (`sim::save`).
    ///
    /// `positions` is front to back, i.e. descending distance-from-start,
    /// exactly as `snapshot_positions` emits it; the gap representation this
    /// module is built on is derived back out of it here, which is what
    /// makes the round trip exact rather than approximate (`positions` is
    /// the only view of a belt anything outside this file has, and a save
    /// storing the internal gaps instead would be storing a representation
    /// the format is not allowed to depend on).
    ///
    /// **`frontier` is a parameter and not recomputed**, because it is
    /// genuinely behavioural rather than a cache: the module doc's
    /// "freed space ripples back by at most one item per tick" property is
    /// enforced *by* the scan bound, so a belt restored with a wider
    /// frontier would let a compressed run advance further in its first tick
    /// than the belt it was saved from. It is clamped on the way in, so a
    /// hand-edited save cannot push it out of range.
    pub(crate) fn restore(
        length: Fx,
        min_spacing: Fx,
        positions: &[(ItemId, Fx)],
        frontier: usize,
    ) -> Self {
        let mut items: VecDeque<BeltItem> = VecDeque::with_capacity(positions.len());
        for (index, (item, position)) in positions.iter().enumerate() {
            // Index 0 stores its plain distance to the belt's end; every
            // item behind it stores the slack beyond `min_spacing` to the
            // one ahead (the module doc's two cases, inverted).
            let gap_ahead = if index == 0 {
                length - *position
            } else {
                (positions[index - 1].1 - *position) - min_spacing
            };
            items.push_back(BeltItem {
                item: item.clone(),
                gap_ahead,
            });
        }
        let frontier = frontier.min(items.len().saturating_sub(1));
        Self {
            length,
            min_spacing,
            items,
            frontier,
        }
    }

    /// (item, distance-from-belt-start) for every item currently on the
    /// belt, front to back — presentation-only, computed on demand for
    /// snapshots (ARCHITECTURE.md §4). Never stored as simulation state.
    pub fn snapshot_positions(&self) -> Vec<(ItemId, Fx)> {
        (0..self.items.len())
            .map(|i| {
                (
                    self.items[i].item.clone(),
                    self.length - self.dist_from_end(i),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(v: f64) -> Fx {
        Fx::from_num(v)
    }

    const SPACING: f64 = 0.25;

    #[test]
    fn single_item_advances_by_speed_each_tick() {
        let mut belt = BeltSegment::new(fx(4.0), fx(SPACING));
        belt.try_spawn("tomato".into());
        let moved = belt.tick(fx(0.1));
        assert!(moved);
        let positions = belt.snapshot_positions();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].1, fx(0.1));
    }

    #[test]
    fn item_stops_exactly_at_belt_end_and_does_not_overshoot() {
        let mut belt = BeltSegment::new(fx(0.3), fx(SPACING));
        belt.try_spawn("tomato".into());
        for _ in 0..10 {
            belt.tick(fx(0.1));
        }
        let positions = belt.snapshot_positions();
        assert_eq!(positions[0].1, fx(0.3));
        assert!(belt.take_front_if_at_end().is_some());
        assert!(belt.is_empty());
    }

    #[test]
    fn following_item_compresses_against_a_jammed_leader_without_overlap() {
        let mut belt = BeltSegment::new(fx(10.0), fx(SPACING));
        belt.try_spawn("a".into());
        // Advance the leader most of the way, then leave it there forever
        // (nobody calls take_front_if_at_end) to simulate a full
        // downstream machine.
        for _ in 0..80 {
            belt.tick(fx(0.1));
        }
        assert!(belt.try_spawn("b".into()));
        for _ in 0..200 {
            belt.tick(fx(0.1));
        }
        let positions = belt.snapshot_positions();
        assert_eq!(positions.len(), 2);
        let gap_between = positions[0].1 - positions[1].1;
        assert!(
            gap_between >= fx(SPACING),
            "items must never overlap closer than min spacing, got gap {gap_between}"
        );
    }

    /// The save/load round trip's floor: whatever `snapshot_positions`
    /// reports must rebuild a belt that reports exactly the same thing —
    /// exactly, not nearly, since `Fx` is fixed-point and a save is expected
    /// to come back running-state-identical (`sim::save`).
    #[test]
    fn restoring_from_snapshot_positions_is_exact() {
        let mut belt = BeltSegment::new(fx(4.0), fx(SPACING));
        // An item has to have travelled `min_spacing` from the start before
        // the next one fits behind it, which at this speed is 13 ticks.
        assert!(belt.try_spawn("a".into()));
        for _ in 0..21 {
            belt.tick(fx(0.02));
        }
        assert!(belt.try_spawn("b".into()));
        for _ in 0..13 {
            belt.tick(fx(0.02));
        }
        assert!(belt.try_spawn("c".into()));
        for _ in 0..11 {
            belt.tick(fx(0.02));
        }

        let positions = belt.snapshot_positions();
        assert_eq!(positions.len(), 3);
        let restored =
            BeltSegment::restore(belt.length, belt.min_spacing, &positions, belt.frontier());
        assert_eq!(restored.snapshot_positions(), positions);
        assert_eq!(restored.frontier(), belt.frontier());

        // ...and it must keep behaving identically, not just read the same.
        let mut original = belt;
        let mut copy = restored;
        for _ in 0..50 {
            assert_eq!(original.tick(fx(0.02)), copy.tick(fx(0.02)));
            assert_eq!(original.snapshot_positions(), copy.snapshot_positions());
        }
    }

    /// An empty belt round-trips too — the case a freshly placed tile is in.
    #[test]
    fn restoring_an_empty_belt_yields_an_empty_belt() {
        let belt = BeltSegment::restore(fx(1.0), fx(SPACING), &[], 0);
        assert!(belt.is_empty());
        assert_eq!(belt.snapshot_positions(), Vec::new());
    }

    #[test]
    fn cannot_spawn_on_top_of_an_unmoved_item() {
        let mut belt = BeltSegment::new(fx(4.0), fx(SPACING));
        assert!(belt.try_spawn("a".into()));
        assert!(!belt.try_spawn("b".into()));
    }

    #[test]
    fn exiting_the_front_folds_min_spacing_back_into_the_new_front() {
        let mut belt = BeltSegment::new(fx(1.0), fx(SPACING));
        belt.try_spawn("a".into());
        // Run the leader all the way to the end (but don't remove it yet),
        // then queue a follower right behind it.
        for _ in 0..10 {
            belt.tick(fx(0.1));
        }
        assert!(belt.try_spawn("b".into()));
        let dist_to_end_before = belt.length - belt.snapshot_positions()[1].1;

        belt.take_front_if_at_end();

        // "b" is now the front item; its literal distance to the belt's
        // end must be unchanged by "a"'s removal.
        let dist_to_end_after = belt.length - belt.snapshot_positions()[0].1;
        assert_eq!(dist_to_end_after, dist_to_end_before);

        for _ in 0..10 {
            belt.tick(fx(0.1));
        }
        assert!(
            belt.take_front_if_at_end().is_some(),
            "the follower must be able to reach the end on its own"
        );
    }

    #[test]
    fn room_at_start_is_the_whole_belt_when_empty_and_the_back_items_distance_otherwise() {
        let mut belt = BeltSegment::new(fx(1.0), fx(SPACING));
        assert_eq!(belt.room_at_start(), fx(1.0));
        belt.try_spawn("a".into());
        assert_eq!(belt.room_at_start(), fx(0.0));
        // Eighths, which `Fx` holds exactly (0.1 does not sum to 0.3).
        belt.tick(fx(0.125));
        belt.tick(fx(0.125));
        assert_eq!(belt.room_at_start(), fx(0.25));
        // A second item behind the first: the room is measured to the
        // BACK-most one, whatever the first has done since.
        belt.try_spawn("b".into());
        assert_eq!(belt.room_at_start(), fx(0.0));
    }

    /// The module doc's boundary rule: an item on the tile ahead standing
    /// at its start is `min_spacing` of footprint this belt's front item
    /// must keep clear of, exactly as it would of an item on its own tile.
    #[test]
    fn the_front_item_stops_min_spacing_short_of_an_item_at_the_next_belts_start() {
        let mut belt = BeltSegment::new(fx(1.0), fx(SPACING));
        belt.try_spawn("a".into());
        // The tile ahead has no room at all at its start.
        for _ in 0..40 {
            belt.tick_toward(fx(0.1), fx(0.0));
        }
        let positions = belt.snapshot_positions();
        assert_eq!(
            positions[0].1,
            fx(1.0 - SPACING),
            "held one footprint short of the end, not on the boundary"
        );
        assert!(
            belt.front_ready().is_none(),
            "an item held short of the end is not ready to hand over"
        );
        // Room opens up ahead by less than a footprint: it creeps forward by
        // exactly that much...
        belt.tick_toward(fx(0.1), fx(0.1));
        assert_eq!(belt.snapshot_positions()[0].1, fx(1.0 - SPACING + 0.1));
        // ...and with a whole footprint of room it reaches the end, which is
        // the same moment `try_spawn` on the tile ahead would accept it.
        for _ in 0..5 {
            belt.tick_toward(fx(0.1), fx(SPACING));
        }
        assert_eq!(belt.snapshot_positions()[0].1, fx(1.0));
        assert!(belt.front_ready().is_some());
    }

    /// The reserve is about the tile AHEAD only: items behind the front one
    /// still close up against it as far as their own footprint allows.
    #[test]
    fn the_reserve_holds_only_the_front_item_and_the_rest_close_up_behind_it() {
        let mut belt = BeltSegment::new(fx(1.0), fx(SPACING));
        belt.try_spawn("a".into());
        for _ in 0..3 {
            belt.tick_toward(fx(0.1), fx(0.0));
        }
        belt.try_spawn("b".into());
        for _ in 0..40 {
            belt.tick_toward(fx(0.1), fx(0.0));
        }
        let positions = belt.snapshot_positions();
        assert_eq!(positions[0].1, fx(1.0 - SPACING));
        assert_eq!(positions[1].1, fx(1.0 - 2.0 * SPACING));
    }

    /// `tick` is `tick_toward` with nothing to reserve against — the two
    /// must agree exactly, or a belt into a machine would move differently
    /// from one into a belt with room.
    #[test]
    fn tick_is_tick_toward_with_a_whole_footprint_of_room() {
        let mut plain = BeltSegment::new(fx(2.0), fx(SPACING));
        let mut toward = BeltSegment::new(fx(2.0), fx(SPACING));
        for belt in [&mut plain, &mut toward] {
            belt.try_spawn("a".into());
            belt.tick(fx(0.1));
            belt.tick(fx(0.1));
            belt.tick(fx(0.1));
            belt.try_spawn("b".into());
        }
        for _ in 0..30 {
            assert_eq!(plain.tick(fx(0.1)), toward.tick_toward(fx(0.1), fx(SPACING)));
            assert_eq!(plain.snapshot_positions(), toward.snapshot_positions());
        }
        // ...and more room than a footprint changes nothing either.
        for _ in 0..30 {
            assert_eq!(plain.tick(fx(0.1)), toward.tick_toward(fx(0.1), fx(1.0)));
            assert_eq!(plain.snapshot_positions(), toward.snapshot_positions());
        }
    }
}
