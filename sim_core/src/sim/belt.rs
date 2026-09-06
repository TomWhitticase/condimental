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
    pub fn tick(&mut self, speed: Fx) -> bool {
        if self.items.is_empty() {
            self.frontier = 0;
            return false;
        }
        let upper = (self.frontier + 1).min(self.items.len() - 1);
        let mut freed = Fx::ZERO;
        let mut last_moved = 0usize;
        let mut moved = false;
        for i in 0..=upper {
            let item = &mut self.items[i];
            let effective_gap = item.gap_ahead + freed;
            let advance = speed.min(effective_gap);
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
        let back_dist_from_start = self.length - self.dist_from_end(self.items.len() - 1);
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
}
