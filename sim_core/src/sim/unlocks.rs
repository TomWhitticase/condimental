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
//! Lifetime **successful** contract deliveries
//! ([`super::economy::Economy::lifetime_deliveries`]) — satisfied delivery
//! windows only. Not money (which can be sat on), not elapsed ticks (which
//! pass while the keybinds are read), not machines placed (which need never
//! run). A delivery is proof that a whole line ran end to end and shipped
//! something. It never decreases, so a gate, once open, stays open.
//!
//! # What is saved
//!
//! The counter, and nothing else. `save.rs` stores `lifetime_deliveries`
//! and every gate's state is re-derived from it against the *current*
//! `data/unlocks.ron`. Saving "bbq_sauce is unlocked" would freeze a save
//! against the numbers in the file on the day it was written; deriving it
//! means a balance pass on the data file re-reads correctly for every
//! existing save.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::contracts::ContractRegistry;
use super::ingredients::IngredientCatalog;

/// The schema version this build understands. Rejected (not migrated) on
/// mismatch, for the reason [`super::ingredients::CURRENT_SCHEMA_VERSION`]
/// gives: the file is bundled into the binary at compile time, so a
/// mismatch means data and binary have drifted apart.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

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
    /// Lifetime successful deliveries that open this gate. At least 1: a
    /// gate open in a new game is not a gate.
    pub after_deliveries: u32,
    /// `data/ingredients.ron` ids held back from the market until it opens.
    #[serde(default)]
    pub ingredients: Vec<String>,
    /// `data/contracts.ron` ids held back from the board until it opens.
    #[serde(default)]
    pub contracts: Vec<String>,
}

impl Unlock {
    /// Whether this gate is open at `deliveries` lifetime deliveries.
    ///
    /// `>=`, so [`Self::after_deliveries`] is the delivery that opens it
    /// rather than the one before it: a threshold of 10 opens on the tenth
    /// delivery, which is what `data/unlocks.ron`'s comment promises and
    /// what a player counting dockets will expect.
    pub fn is_open(&self, deliveries: u32) -> bool {
        deliveries >= self.after_deliveries
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
    /// `after_deliveries: 0` — open in a new game, so not a gate at all.
    /// Rejected rather than shipped, because the shape of the mistake (a
    /// field left at its default) looks exactly like a gate that works.
    OpenFromTheStart {
        id: String,
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
                "unlocks.ron gives '{id}' after_deliveries 0, which is open in a new \
                 game and therefore not a gate — use at least 1, or delete the unlock \
                 and ship its content ungated"
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
            if unlock.after_deliveries == 0 {
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

    /// Whether the market may sell `id` at `deliveries` lifetime
    /// deliveries. An ungated ingredient is always available — the default,
    /// and what every id in the game was before this module existed.
    pub fn ingredient_available(&self, id: &str, deliveries: u32) -> bool {
        self.gate_for_ingredient(id)
            .is_none_or(|gate| gate.is_open(deliveries))
    }

    /// Whether the board may offer `id` at `deliveries` lifetime
    /// deliveries. [`Self::ingredient_available`]'s rule, other list.
    pub fn contract_available(&self, id: &str, deliveries: u32) -> bool {
        self.gate_for_contract(id)
            .is_none_or(|gate| gate.is_open(deliveries))
    }

    /// Every gate that is open at `deliveries` and was not at `before`, in
    /// file order — what an announcement announces.
    ///
    /// Takes both counts rather than "the ones that opened this tick"
    /// because presentation polls rather than subscribing: Godot diffs the
    /// unlocked set it last drew against the one it just read, exactly as
    /// it diffs the active-contract list to announce a signature or a loss.
    pub fn opened_between(&self, before: u32, after: u32) -> Vec<&Unlock> {
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
    pub fn validate_covers(
        &self,
        ingredients: &IngredientCatalog,
        contracts: &ContractRegistry,
    ) -> Result<(), UnlockLoadError> {
        for unlock in &self.unlocks {
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
        Ok(())
    }
}

/// A presentation-facing snapshot of one gate (ARCHITECTURE.md §4/§5) —
/// carried on [`super::economy::EconomySnapshot`], since the counter that
/// drives it is the economy's.
///
/// `open` is the sim's own answer and the only thing presentation should
/// act on; `deliveries` and `after_deliveries` are there so a UI can say
/// how close it is without doing the comparison itself. `name` is what an
/// announcement says.
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
    pub deliveries: u32,
    pub after_deliveries: u32,
    pub ingredients: usize,
    pub contracts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        unlocks: [
            Unlock(
                id: "bbq_sauce",
                name: "BBQ Sauce",
                after_deliveries: 10,
                ingredients: ["spice_mix", "molasses"],
                contracts: ["smokehouse_trial"],
            ),
            Unlock(
                id: "later",
                name: "Later",
                after_deliveries: 40,
                contracts: ["something_else"],
            ),
        ],
    )"#;

    fn example() -> UnlockRegistry {
        UnlockRegistry::from_ron_str(EXAMPLE).expect("fixture must load")
    }

    #[test]
    fn loads_in_file_order_and_indexes_both_lists() {
        let unlocks = example();
        assert_eq!(
            unlocks.ordered().iter().map(|u| u.id.as_str()).collect::<Vec<_>>(),
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

    #[test]
    fn a_gate_is_closed_below_its_threshold_and_open_at_it() {
        let unlocks = example();
        assert!(!unlocks.ingredient_available("spice_mix", 9));
        assert!(!unlocks.contract_available("smokehouse_trial", 9));
        assert!(unlocks.ingredient_available("spice_mix", 10));
        assert!(unlocks.contract_available("smokehouse_trial", 10));
        // Open stays open.
        assert!(unlocks.ingredient_available("spice_mix", 4000));
        // …and one gate opening says nothing about the next.
        assert!(!unlocks.contract_available("something_else", 10));
        assert!(unlocks.contract_available("something_else", 40));
    }

    #[test]
    fn an_ungated_id_is_available_from_tick_zero() {
        let unlocks = example();
        assert!(unlocks.ingredient_available("tomato", 0));
        assert!(unlocks.contract_available("corner_shop", 0));
        // …and so is everything, under a registry that gates nothing.
        let none = UnlockRegistry::ungated();
        assert!(none.ingredient_available("spice_mix", 0));
        assert!(none.contract_available("smokehouse_trial", 0));
        assert!(none.ordered().is_empty());
    }

    #[test]
    fn opened_between_reports_the_crossing_only() {
        let unlocks = example();
        assert!(unlocks.opened_between(0, 9).is_empty());
        assert_eq!(
            unlocks
                .opened_between(9, 10)
                .iter()
                .map(|u| u.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bbq_sauce"]
        );
        // Already open at `before`, so not news.
        assert!(unlocks.opened_between(10, 11).is_empty());
        // A jump past both reports both, in file order.
        assert_eq!(
            unlocks
                .opened_between(0, 40)
                .iter()
                .map(|u| u.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bbq_sauce", "later"]
        );
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

    #[test]
    fn rejects_a_gate_that_is_open_in_a_new_game() {
        let text = EXAMPLE.replacen("after_deliveries: 10", "after_deliveries: 0", 1);
        assert!(matches!(
            UnlockRegistry::from_ron_str(&text).unwrap_err(),
            UnlockLoadError::OpenFromTheStart { id } if id == "bbq_sauce"
        ));
    }

    #[test]
    fn rejects_a_gate_that_releases_nothing() {
        let text = r#"(
            schema_version: 1,
            unlocks: [Unlock(id: "empty", name: "Empty", after_deliveries: 5)],
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
                Unlock(id: "a", name: "A", after_deliveries: 5, ingredients: ["x"]),
                Unlock(id: "b", name: "B", after_deliveries: 9, contracts: ["x"]),
            ],
        )"#;
        let unlocks = UnlockRegistry::from_ron_str(text).expect("must load");
        assert_eq!(unlocks.gate_for_ingredient("x").unwrap().id, "a");
        assert_eq!(unlocks.gate_for_contract("x").unwrap().id, "b");
    }

    /// The shipped file, against the shipped ingredients and contracts —
    /// the cross-file check [`UnlockRegistry::validate_covers`] exists for.
    #[test]
    fn bundled_unlocks_gate_ids_that_exist() {
        UnlockRegistry::bundled()
            .validate_covers(&IngredientCatalog::bundled(), &ContractRegistry::bundled())
            .expect("every gated id must be real content");
    }

    #[test]
    fn validate_covers_refuses_a_gated_starter() {
        let text = EXAMPLE.replacen(r#""smokehouse_trial""#, r#""corner_shop""#, 1);
        let unlocks = UnlockRegistry::from_ron_str(&text).unwrap();
        let err = unlocks
            .validate_covers(&IngredientCatalog::bundled(), &ContractRegistry::bundled())
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
            .validate_covers(&IngredientCatalog::bundled(), &ContractRegistry::bundled())
            .unwrap_err();
        assert!(matches!(
            err,
            UnlockLoadError::UnknownEntry { ref entry, ref unlock, .. }
                if entry == "mollasses" && unlock == "bbq_sauce"
        ));
    }

    /// The shipped gate, pinned: `docs/BBQ_SAUCE_PLAN.md` §5 names ten
    /// deliveries and the two lists, and this is the file that has to agree
    /// with it.
    #[test]
    fn the_shipped_gate_is_the_plans() {
        let unlocks = UnlockRegistry::bundled();
        let bbq = unlocks.get("bbq_sauce").expect("the BBQ gate must ship");
        assert_eq!(bbq.name, "BBQ Sauce");
        assert_eq!(bbq.after_deliveries, 10);
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
    }
}
