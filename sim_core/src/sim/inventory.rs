//! Generic slot-based storage (ARCHITECTURE.md §3.2's `Inventory`
//! component) — used here as a `Machine`'s input/output buffer, the same
//! shape a chest or player inventory would eventually use.

use super::types::ItemId;

#[derive(Debug, Default, Clone)]
pub struct Inventory {
    /// (item id, count) pairs. A `Vec` rather than a `HashMap` — the small,
    /// fixed number of distinct item kinds a single machine buffer holds
    /// doesn't need hashing, and iteration order stays deterministic
    /// (ARCHITECTURE.md §3.9) for free.
    slots: Vec<(ItemId, u32)>,
    pub capacity_per_item: u32,
}

impl Inventory {
    pub fn new(capacity_per_item: u32) -> Self {
        Self {
            slots: Vec::new(),
            capacity_per_item,
        }
    }

    pub fn count(&self, item: &str) -> u32 {
        self.slots
            .iter()
            .find(|(id, _)| id == item)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    }

    /// Adds `amount` of `item`, capped at `capacity_per_item`. Returns the
    /// amount actually added.
    pub fn add(&mut self, item: &str, amount: u32) -> u32 {
        let existing = self.slots.iter_mut().find(|(id, _)| id == item);
        let slot_count = match existing {
            Some((_, n)) => n,
            None => {
                self.slots.push((item.to_string(), 0));
                &mut self.slots.last_mut().unwrap().1
            }
        };
        let room = self.capacity_per_item.saturating_sub(*slot_count);
        let added = amount.min(room);
        *slot_count += added;
        added
    }

    /// Removes up to `amount` of `item`. Returns the amount actually
    /// removed.
    pub fn remove(&mut self, item: &str, amount: u32) -> u32 {
        if let Some(slot) = self.slots.iter_mut().find(|(id, _)| id == item) {
            let removed = amount.min(slot.1);
            slot.1 -= removed;
            removed
        } else {
            0
        }
    }

    pub fn has_at_least(&self, item: &str, amount: u32) -> bool {
        self.count(item) >= amount
    }

    pub fn has_room_for(&self, item: &str, amount: u32) -> bool {
        self.count(item) + amount <= self.capacity_per_item
    }

    pub fn iter(&self) -> impl Iterator<Item = &(ItemId, u32)> {
        self.slots.iter().filter(|(_, n)| *n > 0)
    }

    /// Every slot in its own order, **including ones currently at zero** —
    /// the save/load module's seam (`sim::save`) and the reason it is not
    /// [`Self::iter`].
    ///
    /// Slot order is observable: it is the order a `MachineSnapshot`'s
    /// `input`/`output` arrays come out in, and a slot that has been emptied
    /// keeps its place so the next unit of that item lands back where it
    /// was. A save that dropped empty slots would therefore reload a machine
    /// that reports its buffers in a different order the moment the missing
    /// item arrives again — running-state-*nearly*-identical, which is not
    /// the bar.
    pub(crate) fn slots(&self) -> &[(ItemId, u32)] {
        &self.slots
    }

    /// Rebuilds an inventory with `slots` exactly as [`Self::slots`]
    /// reported them, order and empties included. Counts are clamped at
    /// `capacity_per_item` the same way [`Self::add`] clamps, so a
    /// hand-edited save cannot over-fill a buffer; `sim::save` rejects such
    /// a save before it reaches here rather than relying on the clamp.
    pub(crate) fn restore(capacity_per_item: u32, slots: Vec<(ItemId, u32)>) -> Self {
        Self {
            slots: slots
                .into_iter()
                .map(|(item, count)| (item, count.min(capacity_per_item)))
                .collect(),
            capacity_per_item,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_caps_at_capacity() {
        let mut inv = Inventory::new(10);
        assert_eq!(inv.add("tomato", 7), 7);
        assert_eq!(inv.add("tomato", 7), 3);
        assert_eq!(inv.count("tomato"), 10);
    }

    #[test]
    fn remove_caps_at_available() {
        let mut inv = Inventory::new(10);
        inv.add("tomato", 4);
        assert_eq!(inv.remove("tomato", 9), 4);
        assert_eq!(inv.count("tomato"), 0);
    }
}
