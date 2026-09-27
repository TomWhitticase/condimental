//! Progression gates: which tiers of content a game has reached, and what
//! each one releases when it opens (docs/BBQ_SAUCE_PLAN.md §5).
//!
//! # Why this module exists
//!
//! Until it did, the game gated nothing. Everything in `shop.ron`,
//! `ingredients.ron` and `contracts.ron` was available in a new game, so
//! the only thing that grew as you played was the balance. That was the
//! right call while the chain made one sauce — there was nothing to hold
//! back. The second sauce is what changes it: handing a player the whole
//! two-sauce board in their first minute makes the ketchup line read as a
//! chore standing between them and the interesting one.
//!
//! # The shape, and the one thing it deliberately cannot express
//!
//! An [`Unlock`] names a threshold and **two lists of ids**: ingredients
//! held back from the market, and contracts held back from the board.
//! There is no third list, and in particular **no way to lock a recipe**.
//! The BBQ recipes sit in `data/recipes.ron` from a new game and are simply
//! unbuildable (their raws cannot be bought) and unsellable (nobody is
//! asking).
//!
//! That is not a simplification, it is the design. A locked recipe would be
//! a state every placed machine consults on every tick — the hottest path
//! in the simulation — to answer a question whose answer changes at most a
//! handful of times in a session. Gating two lists instead keeps the whole
//! mechanism off the tick path: nothing here is read by [`super::machine`],
//! [`super::belt`] or [`super::fluid`] at all. It is read when the market
//! is drawn, when the board is drawn, and when the player tries to buy or
//! sign something.
//!
//! # Why the group is named
//!
//! The obvious alternative — an `unlock_after: 10` field on each
//! `ingredients.ron` and `contracts.ron` entry — spreads one decision over
//! seven entries in two files, and leaves nothing to announce. **The
//! announcement is the point of a gate**: one that opens silently is one
//! the player never experiences. A toast needs a subject, so the group has
//! a [`Unlock::name`] and that name is player-facing prose.
//!
//! # The trigger
//!
//! **Lifetime boxes of a named product sold** — a hundred `boxed_ketchup`
//! opens BBQ, a hundred `boxed_bbq` opens mustard
//! ([`super::economy::Economy::lifetime_boxes_sold`]). "Sold" means shipped
//! as part of a *satisfied* delivery window, because a dock being paid is
//! what selling is in this game: a missed window sells nothing, and a lost
//! contract does not un-sell the boxes it already paid for. It never
//! decreases, so a gate, once open, stays open.
//!
//! Not money (which can be sat on), not elapsed ticks (which pass while the
//! keybinds are read), not machines placed (which need never run) — and
//! since 2026-09-26 not a count of satisfied delivery *windows* either.
//! That trigger did not measure output at all: a window counted the same
//! whether it carried one box or eight, so the tier arrived at the same
//! count for a player running one small starter order as for a player
//! running three, and the count said nothing about how much ketchup either
//! of them had actually made. Boxes of a named product do say it, and they
//! say something the old trigger could not express at all: each tier now
//! waits on **the line before it running**, not on any line running.
//! Mustard waits on BBQ boxes, so the second sauce has to be a going
//! concern before the third one arrives.
//!
//! # What is saved
//!
//! The ledger, and nothing else. `save.rs` stores `lifetime_boxes_sold` —
//! boxes per item id — and every gate's state is re-derived from it against
//! the *current* `data/unlocks.ron`. Saving "bbq_sauce is unlocked" would
//! freeze a save against the numbers in the file on the day it was written;
//! deriving it means a balance pass on the data file re-reads correctly for
//! every existing save.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::contracts::ContractRegistry;
use super::ingredients::IngredientCatalog;
use super::recipe::RecipeRegistry;

/// The schema version this build understands. Rejected (not migrated) on
/// mismatch, for the reason [`super::ingredients::CURRENT_SCHEMA_VERSION`]
/// gives: the file is bundled into the binary at compile time, so a
/// mismatch means data and binary have drifted apart.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Lifetime boxes sold, per item id — the ledger every gate is measured
/// against ([`super::economy::Economy::lifetime_boxes_sold`]). An id absent
/// from it has sold none, which is what every id in a new game is.
///
/// A `BTreeMap` for the reason everything walked or serialized in this crate
/// is one (ARCHITECTURE.md §3.9): the order is part of the save text and of
/// every snapshot built from it.
pub type BoxesSold = BTreeMap<String, u32>;

/// "This many boxes of this item, sold" — a gate's whole trigger.
///
/// One field rather than two siblings (`sold_item` beside `sold_boxes`)
/// because a count with no product and a product with no count are both
/// meaningless, and a single term cannot be half-written: RON says
/// `after_sold: (item: "boxed_ketchup", boxes: 100)`.
#[derive(Debug, Clone, Deserialize)]
pub struct SoldRequirement {
    /// The item id whose boxes count — `data/recipes.ron`'s id for a
    /// finished packed product (`boxed_ketchup`), never a display name and
    /// never a recipe id. [`UnlockRegistry::validate_covers`] refuses one no
    /// data file names and one no recipe *produces*: neither could ever be
    /// sold, so a gate keyed off either would never open.
    pub item: String,
    /// How many of them, over the life of the save. At least 1 — zero is
    /// open in a new game and therefore not a gate.
    pub boxes: u32,
}

/// A shut gate as the two panels that draw a padlock need it: what is
/// waiting, what it is waiting *for*, and how far along the player is.
///
/// A struct rather than a four-tuple because it crosses two module
/// boundaries on its way to the wire ([`super::ingredients::ContentsChoice`]
/// and [`super::cookbook::GateLine`]), and the player-facing sentence is
/// built from all four parts at the far end of both.
#[derive(Debug, Clone)]
pub struct GateProgress {
    /// The gate's player-facing name, as the toast says it.
    pub name: String,
    /// The item id whose boxes it counts. An **id**: titling it is
    /// presentation's job (`main.gd`'s `_item_display_name`).
    pub item: String,
    /// Boxes of `item` the gate wants.
    pub required: u32,
    /// Boxes of `item` sold so far.
    pub sold: u32,
}

/// One progression gate: what it takes to open, what it releases, and what
/// to call it when it does.
#[derive(Debug, Clone, Deserialize)]
pub struct Unlock {
    /// Stable content id. The save format does not key on it (only the
    /// counter is saved — see the module doc), but the announcement's diff
    /// and every test do.
    pub id: String,
    /// Player-facing name, as the toast says it: "BBQ Sauce unlocked — …".
    /// Prose, never an id.
    pub name: String,
    /// What it takes to open: this many boxes of this item, sold. Both
    /// halves are content, so a third sauce gated on a fourth product is an
    /// edit to `data/unlocks.ron` and no Rust change at all.
    pub after_sold: SoldRequirement,
    /// `data/ingredients.ron` ids held back from the market until it opens.
    #[serde(default)]
    pub ingredients: Vec<String>,
    /// `data/contracts.ron` ids held back from the board until it opens.
    #[serde(default)]
    pub contracts: Vec<String>,
}

impl Unlock {
    /// How many boxes of the product this gate watches have been sold.
    /// Absent from the ledger is none, which is what a new game is.
    pub fn sold(&self, sold: &BoxesSold) -> u32 {
        sold.get(&self.after_sold.item).copied().unwrap_or(0)
    }

    /// Whether this gate is open against the ledger `sold`.
    ///
    /// `>=`, so [`SoldRequirement::boxes`] is the box that opens it rather
    /// than the one after it: a threshold of 100 opens on the hundredth box,
    /// which is what `data/unlocks.ron`'s comment promises and what a player
    /// counting cases will expect.
    pub fn is_open(&self, sold: &BoxesSold) -> bool {
        self.sold(sold) >= self.after_sold.boxes
    }

    /// This gate as a padlock caption's four facts — see [`GateProgress`].
    pub fn progress(&self, sold: &BoxesSold) -> GateProgress {
        GateProgress {
            name: self.name.clone(),
            item: self.after_sold.item.clone(),
            required: self.after_sold.boxes,
            sold: self.sold(sold),
        }
    }
}

#[derive(Debug, Deserialize)]
struct UnlockFile {
    schema_version: u32,
    unlocks: Vec<Unlock>,
}

#[derive(Debug)]
pub enum UnlockLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// Two unlocks share an id — a duplicate silently shadows the other,
    /// exactly as it would in any other data file.
    DuplicateId {
        id: String,
    },
    /// `after_sold`'s `boxes: 0` — open in a new game, so not a gate at
    /// all. Rejected rather than shipped, because the shape of the mistake
    /// (a field left at its default) looks exactly like a gate that works.
    OpenFromTheStart {
        id: String,
    },
    /// A gate keyed off an item id no data file names — [`Self::UnknownEntry`]
    /// on the TRIGGER side rather than the release side, and worse: an id
    /// that cannot be sold is a gate that never opens, so the tier it holds
    /// back is unreachable rather than merely ungated. Checked by
    /// [`UnlockRegistry::validate_covers`].
    UnknownSoldItem {
        item: String,
        unlock: String,
        known: Vec<String>,
    },
    /// A gate keyed off a real item that no recipe in `data/recipes.ron`
    /// **produces** — a bought raw, say. It is a perfectly good id, and no
    /// dock will ever ship a box of it, so the gate is shut forever.
    SoldItemNothingProduces {
        item: String,
        unlock: String,
    },
    /// A gate keyed off an item every recipe can make but **no contract ever
    /// orders**. One step further than [`Self::SoldItemNothingProduces`] and
    /// the same invisible failure: only a paid window's `mix` feeds the
    /// ledger, so a product nobody orders is sold zero times however much of
    /// it the factory makes, and the tier never opens. Four of the seven
    /// solid ids `recipes.ron` outputs are in this class — the three bottles
    /// and `tomato_paste`, everything that is not a boxed case — so this is
    /// an easy gate to author by accident.
    SoldItemNoContractOrders {
        item: String,
        unlock: String,
        ordered: Vec<String>,
    },
    /// Two gates counting the **same** product, where the later one in the
    /// file needs fewer boxes than the earlier one. `data/unlocks.ron`'s
    /// header states that file order is tier order and the fixtures rely on
    /// it; this pair breaks that claim in the one case where the thresholds
    /// are comparable at all, because the file's second tier would open
    /// first. Two tiers of one product are fine — they just have to climb.
    TiersOutOfOrder {
        item: String,
        earlier: String,
        earlier_boxes: u32,
        later: String,
        later_boxes: u32,
    },
    /// Both lists empty: an unlock that releases nothing. The announcement
    /// would still fire, promising content that does not arrive.
    ReleasesNothing {
        id: String,
    },
    /// One id gated by two unlocks. Which one announces it would then
    /// depend on file order, and the later gate would be dead: the id is
    /// already released by the earlier one.
    DoublyGated {
        entry: String,
        first: String,
        second: String,
    },
    /// A gated id that no other data file defines. The gate is then a
    /// no-op, which is the failure mode this file is least able to show:
    /// the market and the board look exactly right, because the id was
    /// never in them. See [`UnlockRegistry::validate_covers`].
    UnknownEntry {
        entry: String,
        unlock: String,
        known: Vec<String>,
    },
    /// A gate holding back a `starter: true` contract. A starter is active
    /// from tick 0 without being signed (`Economy::with_money`), so the gate
    /// would hide from the board an order the player is already running and
    /// already being struck for.
    GatedStarter {
        entry: String,
        unlock: String,
    },
}

impl std::fmt::Display for UnlockLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnlockLoadError::Parse(e) => write!(f, "failed to parse unlocks.ron: {e}"),
            UnlockLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "unlocks.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            UnlockLoadError::DuplicateId { id } => write!(
                f,
                "unlocks.ron defines '{id}' more than once — unlocks are looked up by \
                 name, so a duplicate silently shadows the other"
            ),
            UnlockLoadError::OpenFromTheStart { id } => write!(
                f,
                "unlocks.ron gives '{id}' after_sold boxes 0, which is open in a new \
                 game and therefore not a gate — use at least 1, or delete the unlock \
                 and ship its content ungated"
            ),
            UnlockLoadError::UnknownSoldItem {
                item,
                unlock,
                known,
            } => write!(
                f,
                "unlocks.ron's '{unlock}' opens after selling '{item}', which no data \
                 file defines (known: {}) — nothing can ever sell it, so the gate would \
                 never open and its tier would be unreachable",
                known.join(", ")
            ),
            UnlockLoadError::SoldItemNothingProduces { item, unlock } => write!(
                f,
                "unlocks.ron's '{unlock}' opens after selling '{item}', which no recipe \
                 in recipes.ron produces — a dock can only ship what the factory made, \
                 so the gate would never open"
            ),
            UnlockLoadError::SoldItemNoContractOrders {
                item,
                unlock,
                ordered,
            } => write!(
                f,
                "unlocks.ron's '{unlock}' opens after selling '{item}', which no contract in \
                 contracts.ron ever orders (ordered: {}) — only a paid delivery counts as a \
                 sale, so the gate would never open",
                ordered.join(", ")
            ),
            UnlockLoadError::TiersOutOfOrder {
                item,
                earlier,
                earlier_boxes,
                later,
                later_boxes,
            } => write!(
                f,
                "unlocks.ron gates '{earlier}' on {earlier_boxes} boxes of '{item}' and then \
                 '{later}' on only {later_boxes} of the same product — file order is tier \
                 order, so a later gate cannot be cheaper than an earlier one on the same \
                 count"
            ),
            UnlockLoadError::ReleasesNothing { id } => write!(
                f,
                "unlocks.ron's '{id}' names no ingredients and no contracts — it would \
                 announce itself and release nothing"
            ),
            UnlockLoadError::DoublyGated {
                entry,
                first,
                second,
            } => write!(
                f,
                "unlocks.ron gates '{entry}' behind both '{first}' and '{second}' — the \
                 earlier gate releases it, so the later one is dead and which of them \
                 announces it would depend on file order"
            ),
            UnlockLoadError::UnknownEntry {
                entry,
                unlock,
                known,
            } => write!(
                f,
                "unlocks.ron's '{unlock}' gates '{entry}', which no data file defines \
                 (known: {}) — the gate would hold back nothing and open silently",
                known.join(", ")
            ),
            UnlockLoadError::GatedStarter { entry, unlock } => write!(
                f,
                "unlocks.ron's '{unlock}' gates '{entry}', which contracts.ron marks \
                 starter: true — a starter is active from tick 0 without being signed, \
                 so the gate would hide an order the player is already running"
            ),
        }
    }
}

impl std::error::Error for UnlockLoadError {}

const UNLOCKS_RON: &str = include_str!("../../../data/unlocks.ron");

/// Every progression gate, loaded from `data/unlocks.ron`.
///
/// Preserves the file's own order, for [`super::shop::ShopCatalog`]'s
/// reason — it is the order the gates are meant to be read in, and it is
/// the order two gates opening on the same delivery would be announced in.
///
/// The two lookups presentation and the command path need
/// ([`Self::gate_for_ingredient`], [`Self::gate_for_contract`]) are indexed
/// by id into `BTreeMap`s rather than scanned: the market and the board ask
/// them once per entry, every poll.
#[derive(Debug, Default)]
pub struct UnlockRegistry {
    unlocks: Vec<Unlock>,
    /// Gated ingredient id -> index into `unlocks`.
    by_ingredient: BTreeMap<String, usize>,
    /// Gated contract id -> index into `unlocks`.
    by_contract: BTreeMap<String, usize>,
}

impl UnlockRegistry {
    pub fn from_ron_str(text: &str) -> Result<Self, UnlockLoadError> {
        let file: UnlockFile = ron::from_str(text).map_err(UnlockLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(UnlockLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        let mut ids: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_ingredient: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_contract: BTreeMap<String, usize> = BTreeMap::new();
        for (index, unlock) in file.unlocks.iter().enumerate() {
            if ids.insert(unlock.id.clone(), index).is_some() {
                return Err(UnlockLoadError::DuplicateId {
                    id: unlock.id.clone(),
                });
            }
            if unlock.after_sold.boxes == 0 {
                return Err(UnlockLoadError::OpenFromTheStart {
                    id: unlock.id.clone(),
                });
            }
            if unlock.ingredients.is_empty() && unlock.contracts.is_empty() {
                return Err(UnlockLoadError::ReleasesNothing {
                    id: unlock.id.clone(),
                });
            }
            // The two namespaces are checked separately, not together: an
            // item id and a contract id live in different data files and
            // could legitimately collide without either gate being wrong.
            for (entries, map) in [
                (&unlock.ingredients, &mut by_ingredient),
                (&unlock.contracts, &mut by_contract),
            ] {
                for entry in entries {
                    if let Some(&first) = map.get(entry) {
                        return Err(UnlockLoadError::DoublyGated {
                            entry: entry.clone(),
                            first: file.unlocks[first].id.clone(),
                            second: unlock.id.clone(),
                        });
                    }
                    map.insert(entry.clone(), index);
                }
            }
        }
        Ok(Self {
            unlocks: file.unlocks,
            by_ingredient,
            by_contract,
        })
    }

    /// The shipped `data/unlocks.ron`. Panics only if that bundled file is
    /// itself invalid — a build-time content bug, not a runtime condition
    /// any caller could recover from.
    pub fn bundled() -> Self {
        Self::from_ron_str(UNLOCKS_RON)
            .unwrap_or_else(|e| panic!("bundled data/unlocks.ron must load: {e}"))
    }

    /// A registry that gates nothing — every id available from tick zero.
    /// The seam a test uses to build a world without the shipped gate in
    /// the way of what it is actually testing.
    pub fn ungated() -> Self {
        Self::default()
    }

    /// Every gate, in the data file's order.
    pub fn ordered(&self) -> &[Unlock] {
        &self.unlocks
    }

    pub fn get(&self, id: &str) -> Option<&Unlock> {
        self.unlocks.iter().find(|u| u.id == id)
    }

    /// The gate holding `id` back from the market, if any is.
    pub fn gate_for_ingredient(&self, id: &str) -> Option<&Unlock> {
        self.by_ingredient.get(id).map(|&i| &self.unlocks[i])
    }

    /// The gate holding `id` back from the contracts board, if any is.
    pub fn gate_for_contract(&self, id: &str) -> Option<&Unlock> {
        self.by_contract.get(id).map(|&i| &self.unlocks[i])
    }

    /// Where in [`Self::ordered`] the gate holding `id` back sits — what a
    /// caller holding several gated ids uses to pick the DEEPEST of them.
    ///
    /// It has to be file order, and that is the change the boxes-sold
    /// trigger forced: two gates' thresholds are no longer comparable
    /// numbers. "100 `boxed_ketchup`" and "100 `boxed_bbq`" are the same
    /// integer and different tiers, so `max_by_key(after_sold.boxes)` — what
    /// this replaced — would have picked between them by a coin toss the day
    /// a balance pass made the numbers equal. `data/unlocks.ron`'s order IS
    /// its tier order, stated in that file's header, so the later entry is
    /// the deeper tier by construction.
    pub fn gate_index_for_ingredient(&self, id: &str) -> Option<usize> {
        self.by_ingredient.get(id).copied()
    }

    /// The gate at `index` in [`Self::ordered`].
    pub fn at(&self, index: usize) -> Option<&Unlock> {
        self.unlocks.get(index)
    }

    /// Whether the market may sell `id` against the ledger `sold`. An
    /// ungated ingredient is always available — the default, and what every
    /// id in the game was before this module existed.
    pub fn ingredient_available(&self, id: &str, sold: &BoxesSold) -> bool {
        self.gate_for_ingredient(id)
            .is_none_or(|gate| gate.is_open(sold))
    }

    /// Whether the board may offer `id` against the ledger `sold`.
    /// [`Self::ingredient_available`]'s rule, other list.
    pub fn contract_available(&self, id: &str, sold: &BoxesSold) -> bool {
        self.gate_for_contract(id)
            .is_none_or(|gate| gate.is_open(sold))
    }

    /// Every gate that is open against the ledger `after` and was not against
    /// `before`, in file order — what an announcement announces.
    ///
    /// Takes both ledgers rather than "the ones that opened this tick"
    /// because presentation polls rather than subscribing: Godot diffs the
    /// unlocked set it last drew against the one it just read, exactly as
    /// it diffs the active-contract list to announce a signature or a loss.
    pub fn opened_between(&self, before: &BoxesSold, after: &BoxesSold) -> Vec<&Unlock> {
        self.unlocks
            .iter()
            .filter(|u| !u.is_open(before) && u.is_open(after))
            .collect()
    }

    /// Checks that every gated id is something another data file actually
    /// defines — the cross-file invariant between this file,
    /// `ingredients.ron` and `contracts.ron`.
    ///
    /// **Deliberately not called from `World`'s construction**, for
    /// [`super::ingredients::IngredientCatalog::validate_covers`]'s reason:
    /// a gate naming a nonexistent id leaves the world perfectly coherent,
    /// it just holds nothing back, and every test in this crate that builds
    /// a world out of fabricated content would otherwise have to fabricate
    /// the shipped gate's ids too. It is run against the shipped files by
    /// this module's own test instead.
    ///
    /// It matters more here than it does there, though, and that is worth
    /// saying: an unpriced ingredient shows up as a hopper that cannot be
    /// filled, while a gate with a typo'd id shows up as *nothing at all* —
    /// the market and the board look exactly as they should, because the id
    /// was never in them.
    ///
    /// It also catches the one gate that would be actively wrong rather
    /// than merely inert: a `starter: true` contract held back from the
    /// board it is already active on.
    ///
    /// And since the trigger became boxes sold, it checks the trigger too,
    /// twice. `recipes` is here for the first half: a gate keyed off an item
    /// no recipe produces can never open, which is a worse version of the
    /// same invisible failure — the market and the board look exactly right,
    /// forever. `contracts` covers the second half, because **produced is
    /// not ordered**: the ledger is fed only by a paid window's `mix`, so a
    /// gate on a product every machine can make but no customer ever asks
    /// for is exactly as stuck. Most recipe outputs are in that class — the
    /// bottles and the intermediates — and only the boxed cases are ordered,
    /// so it is the easier of the two mistakes to make.
    pub fn validate_covers(
        &self,
        ingredients: &IngredientCatalog,
        contracts: &ContractRegistry,
        recipes: &RecipeRegistry,
    ) -> Result<(), UnlockLoadError> {
        // Everything a recipe makes, and everything any data file names as an
        // item at all. The second set is what tells a typo ("boxed_ketchupp")
        // apart from a real id that nothing produces ("tomato") — two
        // different authoring mistakes with two different fixes.
        let mut produced: BTreeSet<&str> = BTreeSet::new();
        let mut known: BTreeSet<String> = ingredients.ids().into_iter().collect();
        // ...and everything a contract actually asks for. "Produced" is not
        // "ordered": only a paid window's `mix` feeds the ledger, so a gate
        // on a product no customer wants is as unopenable as a gate on a
        // typo, and looks just as correct in the file.
        let ordered: BTreeSet<&str> = contracts
            .ordered()
            .iter()
            .flat_map(|contract| contract.mix.iter())
            .map(|(id, _)| id.as_str())
            .collect();
        for recipe in recipes.ordered() {
            for (id, _) in &recipe.outputs {
                produced.insert(id.as_str());
                known.insert(id.clone());
            }
            for (id, _) in &recipe.inputs {
                known.insert(id.clone());
            }
        }
        for unlock in &self.unlocks {
            let item = &unlock.after_sold.item;
            if !known.contains(item) {
                return Err(UnlockLoadError::UnknownSoldItem {
                    item: item.clone(),
                    unlock: unlock.id.clone(),
                    known: known.iter().cloned().collect(),
                });
            }
            if !produced.contains(item.as_str()) {
                return Err(UnlockLoadError::SoldItemNothingProduces {
                    item: item.clone(),
                    unlock: unlock.id.clone(),
                });
            }
            if !ordered.contains(item.as_str()) {
                return Err(UnlockLoadError::SoldItemNoContractOrders {
                    item: item.clone(),
                    unlock: unlock.id.clone(),
                    ordered: ordered.iter().map(|id| (*id).to_string()).collect(),
                });
            }
            for id in &unlock.ingredients {
                if ingredients.get(id).is_none() {
                    return Err(UnlockLoadError::UnknownEntry {
                        entry: id.clone(),
                        unlock: unlock.id.clone(),
                        known: ingredients.ids(),
                    });
                }
            }
            for id in &unlock.contracts {
                let Some(contract) = contracts.get(id) else {
                    return Err(UnlockLoadError::UnknownEntry {
                        entry: id.clone(),
                        unlock: unlock.id.clone(),
                        known: contracts.ids(),
                    });
                };
                if contract.starter {
                    return Err(UnlockLoadError::GatedStarter {
                        entry: id.clone(),
                        unlock: unlock.id.clone(),
                    });
                }
            }
        }
        // File order is tier order (`data/unlocks.ron`'s header). Thresholds
        // on two DIFFERENT products are not comparable — a hundred boxes of
        // ketchup is neither more nor less than a hundred of BBQ — but on the
        // same product they are, and that is the one case where the file can
        // contradict its own claim.
        let mut highest: BTreeMap<&str, (&str, u32)> = BTreeMap::new();
        for unlock in &self.unlocks {
            let item = unlock.after_sold.item.as_str();
            let boxes = unlock.after_sold.boxes;
            if let Some((earlier, earlier_boxes)) = highest.get(item)
                && *earlier_boxes >= boxes
            {
                return Err(UnlockLoadError::TiersOutOfOrder {
                    item: item.to_string(),
                    earlier: (*earlier).to_string(),
                    earlier_boxes: *earlier_boxes,
                    later: unlock.id.clone(),
                    later_boxes: boxes,
                });
            }
            highest.insert(item, (unlock.id.as_str(), boxes));
        }

        Ok(())
    }
}

/// A presentation-facing snapshot of one gate (ARCHITECTURE.md §4/§5) —
/// carried on [`super::economy::EconomySnapshot`], since the counter that
/// drives it is the economy's.
///
/// `open` is the sim's own answer and the only thing presentation should
/// act on; `item`, `sold` and `required` are there so a UI can say "34 of
/// 100 boxes of Boxed Ketchup" without knowing the rule — the id, the two
/// counts, and not the sentence. `name` is what an announcement says.
///
/// `item` is an **id** and stays one: the display name reaches Godot through
/// `display_names_json` and is applied by `_item_display_name`, so titling it
/// here would be a second title-caser to keep in step (`super::cookbook`'s
/// module doc makes the same point about every other id on the wire).
///
/// `ingredients` and `contracts` are the SIZES of the two lists, not the
/// lists: the ids are already on the wire in the market and board they
/// belong to, and repeating them here would be a second copy to keep in
/// step. The counts are here so the announcement can say which halves of
/// the game just changed — "new ingredients in the shop, new orders on the
/// board" — without the presentation layer assuming every gate releases
/// both, which the shipped one happens to and a later one need not.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UnlockSnapshot {
    pub id: String,
    pub name: String,
    pub open: bool,
    /// The item id whose boxes this gate counts.
    pub item: String,
    /// Boxes of `item` sold so far.
    pub sold: u32,
    /// Boxes of `item` the gate wants.
    pub required: u32,
    pub ingredients: usize,
    pub contracts: usize,
}

#[cfg(test)]
mod tests {
    use super::super::recipe::RecipeRegistry;
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        unlocks: [
            Unlock(
                id: "bbq_sauce",
                name: "BBQ Sauce",
                after_sold: (item: "boxed_ketchup", boxes: 100),
                ingredients: ["spice_mix", "molasses"],
                contracts: ["smokehouse_trial"],
            ),
            Unlock(
                id: "later",
                name: "Later",
                after_sold: (item: "boxed_bbq", boxes: 100),
                contracts: ["something_else"],
            ),
        ],
    )"#;

    fn example() -> UnlockRegistry {
        UnlockRegistry::from_ron_str(EXAMPLE).expect("fixture must load")
    }

    /// `EXAMPLE` with its deliberately-unknown contract swapped for a real
    /// one. A test that breaks exactly one thing has to fail *for that thing*:
    /// with `something_else` left in, a test that disables the check it is
    /// about trips `UnknownEntry` on the next gate instead, and the failure
    /// reads as the wrong bug.
    fn example_with_real_content() -> String {
        EXAMPLE.replacen(r#"["something_else"]"#, r#"["bbq_festival"]"#, 1)
    }

    /// Two gates whose every released id is real shipped content, so
    /// `validate_covers` reaches the tier-ordering pass instead of stopping
    /// at an unknown id. `first` and `second` are their two thresholds, both
    /// counting the same product (`boxed_ketchup`), which is the only case
    /// where two thresholds can be compared at all.
    fn two_gates_on_one_product(first: u32, second: u32) -> UnlockRegistry {
        let text = format!(
            r#"(
        schema_version: 1,
        unlocks: [
            Unlock(
                id: "bbq_sauce",
                name: "BBQ Sauce",
                after_sold: (item: "boxed_ketchup", boxes: {first}),
                ingredients: ["spice_mix", "molasses"],
                contracts: ["smokehouse_trial"],
            ),
            Unlock(
                id: "later",
                name: "Later",
                after_sold: (item: "boxed_ketchup", boxes: {second}),
                contracts: ["bbq_festival"],
            ),
        ],
    )"#
        );
        UnlockRegistry::from_ron_str(&text).expect("fixture must load")
    }

    /// A ledger, written the way a test reads: `sold([("boxed_ketchup", 99)])`.
    fn sold(entries: &[(&str, u32)]) -> BoxesSold {
        entries
            .iter()
            .map(|(id, n)| ((*id).to_string(), *n))
            .collect()
    }

    fn recipes() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("bundled data/recipes.ron must parse")
    }

    #[test]
    fn loads_in_file_order_and_indexes_both_lists() {
        let unlocks = example();
        assert_eq!(
            unlocks
                .ordered()
                .iter()
                .map(|u| u.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bbq_sauce", "later"]
        );
        assert_eq!(
            unlocks.gate_for_ingredient("molasses").unwrap().id,
            "bbq_sauce"
        );
        assert_eq!(
            unlocks.gate_for_contract("something_else").unwrap().id,
            "later"
        );
        // An id nothing gates has no gate, which is what "available" means
        // for every ingredient and contract the game shipped before gates.
        assert!(unlocks.gate_for_ingredient("tomato").is_none());
    }

    /// The trigger, read off the fixture's own `after_sold`: a named product
    /// and a count of boxes of it, not a count of delivery windows.
    #[test]
    fn a_gate_names_the_product_it_waits_on() {
        let unlocks = example();
        let bbq = unlocks.get("bbq_sauce").expect("the fixture's first gate");
        assert_eq!(bbq.after_sold.item, "boxed_ketchup");
        assert_eq!(bbq.after_sold.boxes, 100);
        let later = unlocks.get("later").expect("the fixture's second gate");
        assert_eq!(
            later.after_sold.item, "boxed_bbq",
            "the second tier waits on the FIRST tier's product, which is the whole point \
             of naming a product rather than counting windows"
        );
    }

    #[test]
    fn a_gate_is_closed_below_its_threshold_and_open_at_it() {
        let unlocks = example();
        let ninety_nine = sold(&[("boxed_ketchup", 99)]);
        let hundred = sold(&[("boxed_ketchup", 100)]);
        assert!(!unlocks.ingredient_available("spice_mix", &ninety_nine));
        assert!(!unlocks.contract_available("smokehouse_trial", &ninety_nine));
        assert!(unlocks.ingredient_available("spice_mix", &hundred));
        assert!(unlocks.contract_available("smokehouse_trial", &hundred));
        // Open stays open.
        assert!(unlocks.ingredient_available("spice_mix", &sold(&[("boxed_ketchup", 4_000)])));
        // …and one gate opening says nothing about the next, because the next
        // one counts a DIFFERENT product: a thousand boxes of ketchup is not
        // one box of BBQ.
        assert!(!unlocks.contract_available("something_else", &sold(&[("boxed_ketchup", 4_000)])));
        assert!(unlocks.contract_available("something_else", &sold(&[("boxed_bbq", 100)])));
    }

    /// The failure the old trigger could not have: selling plenty of the
    /// WRONG product moves nothing.
    #[test]
    fn boxes_of_another_product_do_not_open_a_gate() {
        let unlocks = example();
        let wrong = sold(&[("boxed_bbq", 5_000), ("boxed_mustard", 5_000)]);
        assert!(
            !unlocks.ingredient_available("spice_mix", &wrong),
            "the BBQ gate counts boxed_ketchup; boxes of anything else are not its business"
        );
        let bbq = unlocks.get("bbq_sauce").unwrap();
        assert_eq!(bbq.sold(&wrong), 0);
        assert_eq!(bbq.sold(&sold(&[("boxed_ketchup", 7)])), 7);
    }

    #[test]
    fn an_ungated_id_is_available_from_tick_zero() {
        let unlocks = example();
        let nothing = BoxesSold::new();
        assert!(unlocks.ingredient_available("tomato", &nothing));
        assert!(unlocks.contract_available("corner_shop", &nothing));
        // …and so is everything, under a registry that gates nothing.
        let none = UnlockRegistry::ungated();
        assert!(none.ingredient_available("spice_mix", &nothing));
        assert!(none.contract_available("smokehouse_trial", &nothing));
        assert!(none.ordered().is_empty());
    }

    #[test]
    fn opened_between_reports_the_crossing_only() {
        let unlocks = example();
        let nothing = BoxesSold::new();
        let ninety_nine = sold(&[("boxed_ketchup", 99)]);
        let hundred = sold(&[("boxed_ketchup", 100)]);
        assert!(unlocks.opened_between(&nothing, &ninety_nine).is_empty());
        assert_eq!(
            unlocks
                .opened_between(&ninety_nine, &hundred)
                .iter()
                .map(|u| u.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bbq_sauce"]
        );
        // Already open at `before`, so not news.
        assert!(
            unlocks
                .opened_between(&hundred, &sold(&[("boxed_ketchup", 101)]))
                .is_empty()
        );
        // A ledger past both reports both, in file order.
        assert_eq!(
            unlocks
                .opened_between(
                    &nothing,
                    &sold(&[("boxed_ketchup", 100), ("boxed_bbq", 100)])
                )
                .iter()
                .map(|u| u.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bbq_sauce", "later"]
        );
    }

    /// The padlock's four facts, which is everything a caption needs and
    /// nothing a caption has to work out.
    #[test]
    fn progress_carries_the_product_the_count_and_the_target() {
        let unlocks = example();
        let gate = unlocks.get("bbq_sauce").unwrap();
        let progress = gate.progress(&sold(&[("boxed_ketchup", 34)]));
        assert_eq!(progress.name, "BBQ Sauce");
        assert_eq!(progress.item, "boxed_ketchup");
        assert_eq!(progress.sold, 34);
        assert_eq!(progress.required, 100);
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 9", 1);
        assert!(matches!(
            UnlockRegistry::from_ron_str(&text).unwrap_err(),
            UnlockLoadError::UnsupportedSchemaVersion {
                found: 9,
                supported: 1
            }
        ));
    }

    #[test]
    fn rejects_a_duplicate_id() {
        let text = EXAMPLE.replacen(r#"id: "later""#, r#"id: "bbq_sauce""#, 1);
        assert!(matches!(
            UnlockRegistry::from_ron_str(&text).unwrap_err(),
            UnlockLoadError::DuplicateId { id } if id == "bbq_sauce"
        ));
    }

    /// Zero boxes is a gate that is open in a new game, which is not a gate.
    /// The same rejection the delivery threshold had, on the field that
    /// replaced it — and it matters more here, because `boxes: 0` is what a
    /// half-written `after_sold` looks like.
    #[test]
    fn rejects_a_gate_that_is_open_in_a_new_game() {
        let text = EXAMPLE.replacen("boxes: 100", "boxes: 0", 1);
        assert!(matches!(
            UnlockRegistry::from_ron_str(&text).unwrap_err(),
            UnlockLoadError::OpenFromTheStart { id } if id == "bbq_sauce"
        ));
    }

    #[test]
    fn rejects_a_gate_that_releases_nothing() {
        let text = r#"(
            schema_version: 1,
            unlocks: [Unlock(
                id: "empty",
                name: "Empty",
                after_sold: (item: "boxed_ketchup", boxes: 5),
            )],
        )"#;
        assert!(matches!(
            UnlockRegistry::from_ron_str(text).unwrap_err(),
            UnlockLoadError::ReleasesNothing { id } if id == "empty"
        ));
    }

    #[test]
    fn rejects_one_id_gated_twice() {
        let text = EXAMPLE.replacen(r#"["something_else"]"#, r#"["smokehouse_trial"]"#, 1);
        assert!(matches!(
            UnlockRegistry::from_ron_str(&text).unwrap_err(),
            UnlockLoadError::DoublyGated { entry, first, second }
                if entry == "smokehouse_trial" && first == "bbq_sauce" && second == "later"
        ));
    }

    #[test]
    fn an_ingredient_and_a_contract_may_share_an_id() {
        // The two lists are separate namespaces in separate data files, so
        // a collision between them is not a double gate.
        let text = r#"(
            schema_version: 1,
            unlocks: [
                Unlock(id: "a", name: "A", after_sold: (item: "boxed_ketchup", boxes: 5), ingredients: ["x"]),
                Unlock(id: "b", name: "B", after_sold: (item: "boxed_ketchup", boxes: 9), contracts: ["x"]),
            ],
        )"#;
        let unlocks = UnlockRegistry::from_ron_str(text).expect("must load");
        assert_eq!(unlocks.gate_for_ingredient("x").unwrap().id, "a");
        assert_eq!(unlocks.gate_for_contract("x").unwrap().id, "b");
    }

    /// The shipped file, against the shipped ingredients, contracts and
    /// recipes — the cross-file check [`UnlockRegistry::validate_covers`]
    /// exists for.
    #[test]
    fn bundled_unlocks_gate_ids_that_exist() {
        UnlockRegistry::bundled()
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .expect("every gated id must be real content");
    }

    #[test]
    fn validate_covers_refuses_a_gated_starter() {
        let text = EXAMPLE.replacen(r#""smokehouse_trial""#, r#""corner_shop""#, 1);
        let unlocks = UnlockRegistry::from_ron_str(&text).unwrap();
        // The shipped board has no starter, so make `corner_shop` one.
        let board =
            ContractRegistry::from_ron_str(&include_str!("../../../data/contracts.ron").replacen(
                r#"id: "corner_shop","#,
                r#"id: "corner_shop", starter: true,"#,
                1,
            ))
            .unwrap();
        let err = unlocks
            .validate_covers(&IngredientCatalog::bundled(), &board, &recipes())
            .unwrap_err();
        assert!(matches!(
            err,
            UnlockLoadError::GatedStarter { ref entry, .. } if entry == "corner_shop"
        ));
    }

    #[test]
    fn validate_covers_names_a_typo() {
        let text = EXAMPLE.replacen(r#""molasses""#, r#""mollasses""#, 1);
        let unlocks = UnlockRegistry::from_ron_str(&text).unwrap();
        let err = unlocks
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            UnlockLoadError::UnknownEntry { ref entry, ref unlock, .. }
                if entry == "mollasses" && unlock == "bbq_sauce"
        ));
    }

    /// A typo in the PRODUCT is the trigger-side mirror of the one above,
    /// and it fails louder on purpose: an unknown gated id ships a gate that
    /// holds nothing back, while an unknown product ships a gate that never
    /// opens.
    #[test]
    fn validate_covers_refuses_a_product_no_data_file_names() {
        let text = example_with_real_content().replacen(
            r#"item: "boxed_ketchup""#,
            r#"item: "boxed_ketchupp""#,
            1,
        );
        let unlocks = UnlockRegistry::from_ron_str(&text).unwrap();
        let err = unlocks
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .unwrap_err();
        assert!(
            matches!(
                err,
                UnlockLoadError::UnknownSoldItem { ref item, ref unlock, .. }
                    if item == "boxed_ketchupp" && unlock == "bbq_sauce"
            ),
            "got {err}"
        );
        // And the message lists what it could have meant, like every other
        // unknown-id refusal in this file.
        assert!(format!("{err}").contains("boxed_ketchup"));
    }

    /// A real id that nothing makes. `tomato` is priced, buyable and named
    /// all over the content — and no recipe outputs it, so no dock can ever
    /// ship a box of it and the gate would be shut forever.
    #[test]
    fn validate_covers_refuses_a_product_nothing_produces() {
        let text = example_with_real_content().replacen(
            r#"item: "boxed_ketchup""#,
            r#"item: "tomato""#,
            1,
        );
        let unlocks = UnlockRegistry::from_ron_str(&text).unwrap();
        let err = unlocks
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .unwrap_err();
        assert!(
            matches!(
                err,
                UnlockLoadError::SoldItemNothingProduces { ref item, ref unlock }
                    if item == "tomato" && unlock == "bbq_sauce"
            ),
            "got {err}"
        );
    }

    /// A real id that recipes really do produce, that **no contract ever
    /// orders**. `bottled_ketchup` is made by the bottler on the way to a
    /// case, and every `mix` in `contracts.ron` asks for boxed cases, so the
    /// ledger would never see one and the gate would be shut forever —
    /// exactly as dead as `tomato` above, and much easier to author.
    #[test]
    fn validate_covers_refuses_a_product_no_contract_orders() {
        let text = example_with_real_content().replacen(
            r#"item: "boxed_ketchup""#,
            r#"item: "bottled_ketchup""#,
            1,
        );
        let unlocks = UnlockRegistry::from_ron_str(&text).unwrap();
        let err = unlocks
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .expect_err(
                "a gate on a product no contract ever orders can never open — the ledger \
                 is fed only by a paid window's mix, so `validate_covers` has to refuse \
                 it (this check is the test suite's, not the loader's)",
            );
        assert!(
            matches!(
                err,
                UnlockLoadError::SoldItemNoContractOrders { ref item, ref unlock, .. }
                    if item == "bottled_ketchup" && unlock == "bbq_sauce"
            ),
            "a gate on a product nobody orders can never open, and 4 of the 7 solid \
             recipe outputs are in that class — got {err}"
        );
        // It lists what IS ordered, so the fix is in the message.
        assert!(format!("{err}").contains("boxed_ketchup"), "got {err}");
    }

    /// Two gates on the same product, the later one cheaper. `data/unlocks.ron`'s
    /// header and `test_layouts.gd` both state that file order IS tier order;
    /// this pair would make that false — at 10 boxes the *second* gate is open
    /// while the first is shut — so it is refused rather than silently
    /// reordering progression.
    #[test]
    fn validate_covers_refuses_two_gates_on_one_product_out_of_order() {
        let unlocks = two_gates_on_one_product(100, 10);
        let err = unlocks
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .expect_err(
                "a later gate that is cheaper than an earlier one on the SAME product \
                 makes file order stop being tier order — `validate_covers` has to \
                 refuse it (this check is the test suite's, not the loader's)",
            );
        assert!(
            matches!(
                err,
                UnlockLoadError::TiersOutOfOrder { ref item, ref earlier, ref later, .. }
                    if item == "boxed_ketchup" && earlier == "bbq_sauce" && later == "later"
            ),
            "got {err}"
        );
    }

    /// ...and the same pair the right way round is fine: two tiers of one
    /// product is a legitimate shape, it just has to climb.
    #[test]
    fn validate_covers_accepts_two_gates_on_one_product_in_order() {
        let unlocks = two_gates_on_one_product(100, 250);
        unlocks
            .validate_covers(
                &IngredientCatalog::bundled(),
                &ContractRegistry::bundled(),
                &recipes(),
            )
            .expect("a rising pair on one product is tier order, not a mistake");
    }

    /// The shipped gates, pinned: the owner's rule of 2026-09-26 is a
    /// hundred boxes of the tier below, and the second gate's product is the
    /// first gate's output.
    #[test]
    fn the_shipped_gates_are_the_owners_rule() {
        let unlocks = UnlockRegistry::bundled();
        let bbq = unlocks.get("bbq_sauce").expect("the BBQ gate must ship");
        assert_eq!(bbq.name, "BBQ Sauce");
        assert_eq!(bbq.after_sold.item, "boxed_ketchup");
        assert_eq!(bbq.after_sold.boxes, 100);
        assert_eq!(bbq.ingredients, ["spice_mix", "molasses", "liquid_smoke"]);
        assert_eq!(
            bbq.contracts,
            [
                "smokehouse_trial",
                "grill_pub",
                "roadhouse_group",
                "bbq_festival"
            ]
        );
        let mustard = unlocks
            .get("mustard_sauce")
            .expect("the mustard gate must ship");
        assert_eq!(mustard.after_sold.item, "boxed_bbq");
        assert_eq!(mustard.after_sold.boxes, 100);
        // The ladder: each gate's product is the sauce the gate below it
        // released, so the tiers are in the file's order for a reason.
        assert_eq!(
            unlocks
                .ordered()
                .iter()
                .map(|u| u.after_sold.item.as_str())
                .collect::<Vec<_>>(),
            vec!["boxed_ketchup", "boxed_bbq"]
        );
    }
}
