//! Contract content, loaded from `data/contracts.ron` (GAMEPLAY.md §6/§9,
//! ARCHITECTURE.md §3.6) — the third sibling of [`super::recipe`] and
//! [`super::machines`], built the same way for the same reason: what the
//! game's *demand* looks like is content, never hardcoded in simulation
//! code (ARCHITECTURE.md §10's "content changes should never require
//! touching Rust").
//!
//! This module owns the [`Contract`] type and its loader/validator only.
//! What a contract *does* over time — its cadence bookkeeping, whether it
//! got paid this window, where the delivered goods come from — belongs to
//! [`super::economy`], which is the same split `recipe.rs`/`machines.rs`
//! already keep from `world.rs`: this file answers "what does the content
//! say," that one answers "what happens each tick."
//!
//! # File order is priority
//!
//! [`ContractRegistry`] preserves `contracts.ron`'s list order, and that
//! order is load-bearing rather than incidental: every active contract
//! draws from one shared pool of delivered stock (see [`super::economy`]'s
//! module doc for the allocation rule), so when two contracts fall due on
//! the same tick and stock is short, the earlier-listed one is filled
//! first. A deterministic tiebreak is required by ARCHITECTURE.md §3.9;
//! this is that tiebreak, and it is stated in `contracts.ron` itself too so
//! whoever edits the file knows reordering it changes behaviour.

use serde::Deserialize;

use super::recipe::RecipeRegistry;
use super::types::{Fx, ItemId};

/// The schema version this build of sim_core understands. Bump this and
/// reject/migrate on mismatch (ARCHITECTURE.md §4) whenever [`Contract`]'s
/// data shape changes in a way older data can't satisfy — adding
/// GAMEPLAY.md §11's still-undecided penalty terms will be the first such
/// change.
///
/// v1 is this file's first version. Mismatched data is **rejected, not
/// migrated**, for exactly the reason
/// [`super::recipe::CURRENT_SCHEMA_VERSION`] gives: `contracts.ron` is
/// bundled into the binary at compile time, so there is one such file and
/// it is updated in the same commit as this constant — a version mismatch
/// means data and binary have drifted apart, which deserves a loud error.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// A standing agreement: a fixed item mix, delivered in full on a recurring
/// cadence, for a fixed payout each time (GAMEPLAY.md §6).
///
/// This used to be a single `Contract::hardcoded()` constructed in
/// `economy.rs` from four named constants (milestone 5's Economy MVP, whose
/// own scope note said the generalization was deferred). It is now loaded
/// from `data/contracts.ron`, and several of them are active at once.
#[derive(Debug, Clone)]
pub struct Contract {
    /// Stable content id — how a contract is named in errors, tests and
    /// (via the economy snapshot) presentation. Never displayed raw to the
    /// player; that's what `name` is for.
    pub id: String,
    /// Player-facing title, e.g. "Corner Shop Standing Order". Content, not
    /// derived from the id, so it can read like a customer rather than a
    /// slug (UI_UX.md §5's clipboard is going to print this).
    pub name: String,
    /// (item id, quantity) pairs required per delivery — GAMEPLAY.md §6's
    /// "mix." All-or-nothing: a window pays only if *every* pair is
    /// satisfiable at the cadence boundary.
    pub mix: Vec<(ItemId, u32)>,
    /// Delivery cadence in sim ticks (ARCHITECTURE.md §3.9: tick-counted,
    /// never wall-clock).
    pub cadence_ticks: u64,
    /// Payout per satisfied delivery, fixed-point (§3.9 — money is shared,
    /// persisted simulation state, so never a float).
    pub payout: Fx,
}

/// The on-disk shape. Separate from [`Contract`] for one reason: `payout`
/// is authored as a whole number of money units and converted to [`Fx`]
/// once, here, at load time — so no float ever reaches simulation state
/// (ARCHITECTURE.md §3.9) and no data file has to know what fixed-point is.
/// When fractional prices are eventually needed, this field changes shape
/// and `schema_version` goes to 2.
#[derive(Debug, Clone, Deserialize)]
// RON matches struct names literally, and the data file spells these
// `Contract(...)` — the name the content author writes should be the domain
// word, not this loader's internal wrapper type.
#[serde(rename = "Contract")]
struct ContractDef {
    id: String,
    name: String,
    mix: Vec<(ItemId, u32)>,
    cadence_ticks: u64,
    payout: u32,
}

#[derive(Debug, Deserialize)]
struct ContractFile {
    schema_version: u32,
    contracts: Vec<ContractDef>,
}

#[derive(Debug)]
pub enum ContractLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// A file that parses but defines nothing. Rejected rather than
    /// tolerated: a world with no contracts has no way to earn money at
    /// all, which is a content mistake, not a playable state.
    NoContracts,
    DuplicateId {
        id: String,
    },
    /// A contract asking for nothing. It would be satisfied by an empty
    /// delivery buffer every single cadence window and pay out forever —
    /// free money, and a silent one.
    EmptyMix {
        id: String,
    },
    /// A mix line asking for zero units of something, which is the same
    /// free-money bug as `EmptyMix` hiding inside an otherwise valid mix.
    ZeroQuantityInMix {
        id: String,
        item: ItemId,
    },
    /// A zero cadence would make the contract due on literally every tick.
    ZeroCadence {
        id: String,
    },
    /// A contract that consumes delivered stock and pays nothing for it.
    /// Almost certainly a data-entry slip rather than intent, and it is
    /// indistinguishable at runtime from one, so it is rejected.
    ZeroPayout {
        id: String,
    },
    /// A contract asks for an item no recipe in `recipes.ron` produces —
    /// so nothing could ever satisfy it. Reported the same way
    /// [`super::machines::MachineLoadError::UnknownKindInRecipe`] reports a
    /// missing machine kind: as an error naming both sides, not a silent
    /// contract that mysteriously never pays.
    ///
    /// Note this is deliberately a *content* check, not a permanent
    /// architectural rule: GAMEPLAY.md §6 wants higher-tier contracts to
    /// eventually demand sauces the player hasn't unlocked yet. When that
    /// lands, an unlock gate decides which contracts are offered and this
    /// check softens to "the item must exist in the item catalogue" —
    /// today there is no catalogue and no gate, so "some recipe produces
    /// it" is the tightest honest check available.
    UnproducibleItem {
        contract: String,
        item: ItemId,
        producible: Vec<String>,
    },
}

impl std::fmt::Display for ContractLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractLoadError::Parse(e) => write!(f, "failed to parse contracts.ron: {e}"),
            ContractLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "contracts.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            ContractLoadError::NoContracts => write!(
                f,
                "contracts.ron defines no contracts — a world with no contracts has no way \
                 to earn money, so this is rejected rather than loaded as an empty board"
            ),
            ContractLoadError::DuplicateId { id } => write!(
                f,
                "contracts.ron defines contract '{id}' more than once — contract ids are \
                 looked up by name, so a duplicate silently shadows the other"
            ),
            ContractLoadError::EmptyMix { id } => write!(
                f,
                "contracts.ron contract '{id}' has an empty mix — an empty mix is satisfied \
                 by an empty delivery buffer, so it would pay out every cadence window \
                 forever without anything being delivered"
            ),
            ContractLoadError::ZeroQuantityInMix { id, item } => write!(
                f,
                "contracts.ron contract '{id}' asks for 0 of '{item}' — every mix line must \
                 require at least one unit, or the line is satisfied by delivering nothing"
            ),
            ContractLoadError::ZeroCadence { id } => write!(
                f,
                "contracts.ron contract '{id}' has cadence_ticks 0 — a contract is checked \
                 once per cadence window, so a zero cadence would make it due on every tick"
            ),
            ContractLoadError::ZeroPayout { id } => write!(
                f,
                "contracts.ron contract '{id}' pays 0 — it would consume delivered stock and \
                 pay nothing for it, which is a data-entry slip rather than a contract"
            ),
            ContractLoadError::UnproducibleItem {
                contract,
                item,
                producible,
            } => write!(
                f,
                "contracts.ron contract '{contract}' asks for '{item}', which no recipe in \
                 data/recipes.ron produces (producible items: {}) — nothing could ever \
                 satisfy it, so fix the contract's mix or add the recipe that makes it",
                producible.join(", ")
            ),
        }
    }
}

impl std::error::Error for ContractLoadError {}

/// The contracts a world starts with, in `contracts.ron` file order.
///
/// Unlike [`super::machines::MachineRegistry`] (a pure lookup table) and
/// like [`RecipeRegistry`], order here is meaningful — see this module's
/// doc.
#[derive(Debug, Default)]
pub struct ContractRegistry {
    contracts: Vec<Contract>,
}

/// The bundled contract content, embedded at compile time for exactly the
/// reason `recipes.ron` and `machines.ron` are: resolving it must be
/// identical under `cargo test`, the Godot editor and an exported build,
/// none of which share a working directory.
const CONTRACTS_RON: &str = include_str!("../../../data/contracts.ron");

impl ContractRegistry {
    pub fn from_ron_str(text: &str) -> Result<Self, ContractLoadError> {
        let file: ContractFile = ron::from_str(text).map_err(ContractLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ContractLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if file.contracts.is_empty() {
            return Err(ContractLoadError::NoContracts);
        }

        let mut contracts = Vec::with_capacity(file.contracts.len());
        for def in file.contracts {
            if contracts.iter().any(|c: &Contract| c.id == def.id) {
                return Err(ContractLoadError::DuplicateId { id: def.id });
            }
            if def.mix.is_empty() {
                return Err(ContractLoadError::EmptyMix { id: def.id });
            }
            if let Some((item, _)) = def.mix.iter().find(|(_, qty)| *qty == 0) {
                return Err(ContractLoadError::ZeroQuantityInMix {
                    id: def.id.clone(),
                    item: item.clone(),
                });
            }
            if def.cadence_ticks == 0 {
                return Err(ContractLoadError::ZeroCadence { id: def.id });
            }
            if def.payout == 0 {
                return Err(ContractLoadError::ZeroPayout { id: def.id });
            }
            contracts.push(Contract {
                id: def.id,
                name: def.name,
                mix: def.mix,
                cadence_ticks: def.cadence_ticks,
                payout: Fx::from_num(def.payout),
            });
        }
        Ok(Self { contracts })
    }

    /// The shipped `data/contracts.ron`. Panics only if that bundled file is
    /// itself invalid, which is a build-time content bug, not a runtime
    /// condition any caller could recover from — same rule as
    /// [`super::machines::MachineRegistry::bundled`].
    pub fn bundled() -> Self {
        Self::from_ron_str(CONTRACTS_RON)
            .unwrap_or_else(|e| panic!("bundled data/contracts.ron must load: {e}"))
    }

    /// Every contract, in file order — which is priority order under
    /// scarcity (see this module's doc).
    pub fn ordered(&self) -> &[Contract] {
        &self.contracts
    }

    pub fn get(&self, id: &str) -> Option<&Contract> {
        self.contracts.iter().find(|c| c.id == id)
    }

    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }

    /// Checks that every item any contract asks for is produced by some
    /// recipe — the cross-file invariant between `contracts.ron` and
    /// `recipes.ron`, and the counterpart of
    /// [`super::machines::MachineRegistry::validate_covers`]. Returns the
    /// first offending contract rather than a list, matching how the other
    /// two loaders report the first structural problem they hit.
    ///
    /// Deliberately **not** called from `World`'s constructor, unlike
    /// `validate_covers`. A missing machine kind has no footprint, so the
    /// chain layout would be silently wrong and construction must fail; an
    /// unsatisfiable contract leaves the world perfectly coherent and merely
    /// never pays — which is the legitimate state a deliberately-out-of-reach
    /// contract will be in once GAMEPLAY.md §6's tier gating exists. So this
    /// is a check on the *shipped files against each other*, run by
    /// `contracts.rs`'s own bundled-content test, not a runtime gate that
    /// would also forbid building a `World` from a synthetic recipe set.
    pub fn validate_producible(&self, recipes: &RecipeRegistry) -> Result<(), ContractLoadError> {
        let mut producible: Vec<String> = recipes
            .ordered()
            .iter()
            .flat_map(|r| r.outputs.iter().map(|(item, _)| item.clone()))
            .collect();
        producible.sort();
        producible.dedup();

        for contract in &self.contracts {
            for (item, _) in &contract.mix {
                if !producible.iter().any(|p| p == item) {
                    return Err(ContractLoadError::UnproducibleItem {
                        contract: contract.id.clone(),
                        item: item.clone(),
                        producible,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"(
        schema_version: 1,
        contracts: [
            Contract(
                id: "small",
                name: "Small Order",
                mix: [("boxed_ketchup", 2)],
                cadence_ticks: 1000,
                payout: 50,
            ),
            Contract(
                id: "big",
                name: "Big Order",
                mix: [("boxed_ketchup", 3), ("bottled_ketchup", 1)],
                cadence_ticks: 2500,
                payout: 90,
            ),
        ],
    )"#;

    fn bundled_recipes() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(include_str!("../../../data/recipes.ron"))
            .expect("bundled recipes must parse")
    }

    #[test]
    fn loads_contracts_in_file_order_with_payout_as_fixed_point() {
        let registry = ContractRegistry::from_ron_str(EXAMPLE).unwrap();
        assert_eq!(registry.len(), 2);
        let ids: Vec<&str> = registry.ordered().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["small", "big"], "file order must be preserved");

        let big = registry.get("big").expect("looked up by id");
        assert_eq!(big.name, "Big Order");
        assert_eq!(big.cadence_ticks, 2500);
        assert_eq!(big.payout, Fx::from_num(90));
        assert_eq!(
            big.mix,
            vec![
                ("boxed_ketchup".to_string(), 3),
                ("bottled_ketchup".to_string(), 1)
            ],
            "a multi-item mix is supported by the shape even though shipped content has none yet"
        );
    }

    #[test]
    fn the_bundled_file_loads_and_every_contract_it_names_is_producible() {
        let registry = ContractRegistry::bundled();
        assert!(
            registry.len() >= 2,
            "the point of this ticket is several concurrent contracts; shipped content must \
             actually have several"
        );
        registry
            .validate_producible(&bundled_recipes())
            .expect("every shipped contract must ask for something the chain can make");
        // Distinct cadences are what create the "one contract is due while
        // another isn't" pressure — if the shipped file collapsed to a
        // single cadence, the multi-contract machinery would have no real
        // content behind it (same reasoning as machines.rs's
        // `the_shipped_content_actually_uses_a_multi_tile_footprint`).
        let mut cadences: Vec<u64> = registry.ordered().iter().map(|c| c.cadence_ticks).collect();
        cadences.sort();
        cadences.dedup();
        assert!(
            cadences.len() > 1,
            "shipped contracts all share one cadence — nothing would ever fall due independently"
        );
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 1", "schema_version: 99", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            ContractLoadError::UnsupportedSchemaVersion {
                found: 99,
                supported: 1
            }
        ));
    }

    #[test]
    fn rejects_a_file_with_no_contracts_at_all() {
        let err = ContractRegistry::from_ron_str("(schema_version: 1, contracts: [])").unwrap_err();
        assert!(matches!(err, ContractLoadError::NoContracts));
        assert!(err.to_string().contains("no contracts"));
    }

    #[test]
    fn rejects_a_duplicated_contract_id() {
        let text = EXAMPLE.replacen(r#"id: "big""#, r#"id: "small""#, 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::DuplicateId { ref id } => assert_eq!(id, "small"),
            other => panic!("expected DuplicateId, got {other:?}"),
        }
    }

    #[test]
    fn rejects_an_empty_mix() {
        let text = EXAMPLE.replacen(r#"mix: [("boxed_ketchup", 2)]"#, "mix: []", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::EmptyMix { ref id } => assert_eq!(id, "small"),
            other => panic!("expected EmptyMix, got {other:?}"),
        }
        assert!(err.to_string().contains("empty mix"));
    }

    #[test]
    fn rejects_a_zero_quantity_mix_line() {
        let text = EXAMPLE.replacen(r#"("boxed_ketchup", 2)"#, r#"("boxed_ketchup", 0)"#, 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::ZeroQuantityInMix { ref id, ref item } => {
                assert_eq!(id, "small");
                assert_eq!(item, "boxed_ketchup");
            }
            other => panic!("expected ZeroQuantityInMix, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_zero_cadence() {
        let text = EXAMPLE.replacen("cadence_ticks: 1000", "cadence_ticks: 0", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::ZeroCadence { ref id } => assert_eq!(id, "small"),
            other => panic!("expected ZeroCadence, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_zero_payout() {
        let text = EXAMPLE.replacen("payout: 50", "payout: 0", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::ZeroPayout { ref id } => assert_eq!(id, "small"),
            other => panic!("expected ZeroPayout, got {other:?}"),
        }
    }

    #[test]
    fn a_contract_asking_for_an_unproducible_item_is_an_error_not_a_silent_never_paying_contract() {
        let text = EXAMPLE.replacen(r#"("bottled_ketchup", 1)"#, r#"("chilli_mayo", 1)"#, 1);
        let registry = ContractRegistry::from_ron_str(&text).unwrap();
        let err = registry.validate_producible(&bundled_recipes()).unwrap_err();
        match err {
            ContractLoadError::UnproducibleItem {
                ref contract,
                ref item,
                ref producible,
            } => {
                assert_eq!(contract, "big");
                assert_eq!(item, "chilli_mayo");
                assert!(producible.iter().any(|p| p == "boxed_ketchup"));
            }
            other => panic!("expected UnproducibleItem, got {other:?}"),
        }
        // The message has to name both sides of the mismatch, since the fix
        // could be in either file.
        let message = err.to_string();
        assert!(message.contains("big"), "{message}");
        assert!(message.contains("chilli_mayo"), "{message}");
    }

    #[test]
    fn a_contract_asking_only_for_producible_items_validates() {
        ContractRegistry::from_ron_str(EXAMPLE)
            .unwrap()
            .validate_producible(&bundled_recipes())
            .expect("both boxed_ketchup and bottled_ketchup are chain outputs");
    }
}
