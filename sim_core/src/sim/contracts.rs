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
//!
//! # The file is a board of offers, not the active list (schema 2)
//!
//! Since the contract-board epic (2026-09-10) a contract in this file is
//! something the player can *sign*, for its `price`, and can *lose*, after
//! `strikes` consecutive missed windows. Only the contracts flagged
//! `starter` are active when a new game begins; the rest wait on the board.
//! Which contracts are active right now, and how each is doing, is
//! [`super::economy`]'s state — this module still only answers "what does
//! the content say", and the three new fields are content like the others.

use serde::Deserialize;

use super::recipe::RecipeRegistry;
use super::types::{Money, ItemId};

/// The schema version this build of sim_core understands. Bump this and
/// reject/migrate on mismatch (ARCHITECTURE.md §4) whenever [`Contract`]'s
/// data shape changes in a way older data can't satisfy.
///
/// v1 was this file's first version: mix, cadence and payout only, every
/// contract active from tick 0. **v2 (2026-09-10, the contract-board epic)
/// added `price`, `strikes` and `starter`** — the penalty terms GAMEPLAY.md
/// §11 had left open, now decided as "lose the contract, buy it back". A v1
/// file has no answer for what a contract costs or when it is lost, so it
/// is rejected rather than defaulted. Mismatched data is **rejected, not
/// migrated**, for exactly the reason
/// [`super::recipe::CURRENT_SCHEMA_VERSION`] gives: `contracts.ron` is
/// bundled into the binary at compile time, so there is one such file and
/// it is updated in the same commit as this constant — a version mismatch
/// means data and binary have drifted apart, which deserves a loud error.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

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
    pub payout: Money,
    /// What signing it costs — debited from the one money counter by
    /// `PlayerCommand::SignContract`, and again every time it is signed
    /// after being lost. Never zero (the loader refuses it): a contract
    /// that is free to recover cannot be lost in any sense that matters.
    pub price: Money,
    /// How many windows may be missed **in a row** before the contract is
    /// lost. `1` means a single miss loses it. A paid window resets the
    /// count. Never zero (the loader refuses it): a contract that can never
    /// be lost is v1's contract, and v1 is gone on purpose.
    pub strikes: u32,
    /// Active from the first tick of a new game, without being signed and
    /// without a fee. The board must have at least one, so a new game is
    /// never a factory with nobody to sell to.
    pub starter: bool,
}

/// The on-disk shape. Separate from [`Contract`] for one reason: `payout`
/// is authored as a whole number of money units and converted to [`Money`]
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
    price: u32,
    strikes: u32,
    /// Optional in the file — most contracts are offers, so the common
    /// case is the one that needs no line.
    #[serde(default)]
    starter: bool,
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
    /// A contract that costs nothing to sign. Losing it would cost nothing
    /// either, which makes `strikes` decorative; refused so the board's
    /// terms always mean something.
    ZeroPrice {
        id: String,
    },
    /// A contract that is lost after zero consecutive misses — i.e. never,
    /// since the check runs only when a miss is counted. Either the author
    /// meant 1 or meant "unlosable", and the second is not a thing this
    /// board offers.
    ZeroStrikes {
        id: String,
    },
    /// A board with no `starter` contract. A new game would begin with no
    /// active contract at all — a factory with nobody to sell to until the
    /// player finds the board, which is a bad first minute, not content.
    NoStarter,
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
            ContractLoadError::ZeroPrice { id } => write!(
                f,
                "contracts.ron contract '{id}' has price 0 — a contract that is free to sign \
                 is free to lose, so its strikes would mean nothing; price it"
            ),
            ContractLoadError::ZeroStrikes { id } => write!(
                f,
                "contracts.ron contract '{id}' has strikes 0 — it could never be lost; use 1 \
                 for 'one miss loses it'"
            ),
            ContractLoadError::NoStarter => write!(
                f,
                "contracts.ron marks no contract `starter: true` — a new game would begin \
                 with nothing to deliver against; flag at least one"
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

/// The whole contract board, in `contracts.ron` file order — what a new
/// game starts with (the `starter`s) and everything else it can sign.
///
/// Unlike [`super::machines::MachineRegistry`] (a pure lookup table) and
/// like [`RecipeRegistry`], order here is meaningful — see this module's
/// doc.
#[derive(Debug, Default, Clone)]
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
            if def.price == 0 {
                return Err(ContractLoadError::ZeroPrice { id: def.id });
            }
            if def.strikes == 0 {
                return Err(ContractLoadError::ZeroStrikes { id: def.id });
            }
            contracts.push(Contract {
                id: def.id,
                name: def.name,
                mix: def.mix,
                cadence_ticks: def.cadence_ticks,
                payout: Money::from_num(def.payout),
                price: Money::from_num(def.price),
                strikes: def.strikes,
                starter: def.starter,
            });
        }
        if !contracts.iter().any(|c| c.starter) {
            return Err(ContractLoadError::NoStarter);
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

    /// Where `id` sits in file order — the priority rank a signed contract
    /// takes in the active list, whenever it is signed.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.contracts.iter().position(|c| c.id == id)
    }

    /// The contracts active in a new game, in file order.
    pub fn starters(&self) -> impl Iterator<Item = &Contract> {
        self.contracts.iter().filter(|c| c.starter)
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
        schema_version: 2,
        contracts: [
            Contract(
                id: "small",
                name: "Small Order",
                mix: [("boxed_ketchup", 2)],
                cadence_ticks: 1000,
                payout: 50,
                price: 40,
                strikes: 3,
                starter: true,
            ),
            Contract(
                id: "big",
                name: "Big Order",
                mix: [("boxed_ketchup", 3), ("bottled_ketchup", 1)],
                cadence_ticks: 2500,
                payout: 90,
                price: 150,
                strikes: 2,
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
        assert_eq!(big.payout, Money::from_num(90));
        assert_eq!(big.price, Money::from_num(150));
        assert_eq!(big.strikes, 2);
        assert!(!big.starter, "`starter` defaults to false when the line is absent");
        assert!(registry.get("small").unwrap().starter);
        assert_eq!(registry.position("big"), Some(1));
        let starters: Vec<&str> = registry.starters().map(|c| c.id.as_str()).collect();
        assert_eq!(starters, vec!["small"]);
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

    /// The board-sizing rule `data/contracts.ron`'s balance note states
    /// (2026-09-11): the first three contracts — the starter tier — fit
    /// inside ONE line's throughput, and the board as a whole asks for
    /// several lines' worth, so a second line always has an order to go
    /// after. Measured against the chain's own bottleneck (the slowest
    /// recipe) rather than a literal, so a tuning pass that changes the
    /// vat's cook time moves the expectation with it.
    ///
    /// Also pins the per-box rate rule from the same note: a bigger order
    /// never pays worse per box than a smaller one.
    #[test]
    fn the_bundled_board_is_a_starter_tier_within_one_line_plus_a_growth_tier_beyond_it() {
        let board = ContractRegistry::bundled();
        let recipes = bundled_recipes();
        let bottleneck_ticks = recipes
            .ordered()
            .iter()
            .map(|r| r.time_ticks)
            .max()
            .expect("the chain has recipes") as f64;
        // One line's steady state: one finished unit per bottleneck cycle.
        let one_line_per_1000 = 1000.0 / bottleneck_ticks;
        let demand_per_1000 = |c: &Contract| -> f64 {
            let boxes: u32 = c.mix.iter().map(|(_, q)| q).sum();
            boxes as f64 * 1000.0 / c.cadence_ticks as f64
        };

        let contracts = board.ordered();
        let starter: f64 = contracts.iter().take(3).map(demand_per_1000).sum();
        let whole: f64 = contracts.iter().map(demand_per_1000).sum();
        assert!(
            starter <= one_line_per_1000,
            "the starter tier ({starter:.2}/1000 ticks) must be servable by one line \
             ({one_line_per_1000:.2}/1000 ticks)"
        );
        assert!(
            whole >= 3.0 * one_line_per_1000,
            "the whole board ({whole:.2}/1000 ticks) must ask for several lines' worth \
             ({one_line_per_1000:.2}/1000 ticks each), or a second line has nothing to sell to"
        );

        // Bigger orders pay at least as well per box.
        let mut by_size: Vec<(u32, f64)> = contracts
            .iter()
            .map(|c| {
                let boxes: u32 = c.mix.iter().map(|(_, q)| q).sum();
                (boxes, c.payout.to_num::<f64>() / boxes as f64)
            })
            .collect();
        by_size.sort_by(|a, b| a.0.cmp(&b.0));
        for pair in by_size.windows(2) {
            assert!(
                pair[1].1 >= pair[0].1,
                "a {}-box order pays {:.1}/box, less than a {}-box order's {:.1}/box",
                pair[1].0,
                pair[1].1,
                pair[0].0,
                pair[0].1
            );
        }
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let text = EXAMPLE.replacen("schema_version: 2", "schema_version: 99", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(
            err,
            ContractLoadError::UnsupportedSchemaVersion {
                found: 99,
                supported: 2
            }
        ));
    }

    /// A v1 file — the shape before signing fees and strikes existed — has
    /// no answer for either, so it is refused outright rather than loaded
    /// with invented terms. (It trips the parser on the missing fields
    /// before the version check is reached; either way it does not load.)
    #[test]
    fn rejects_a_schema_1_file_rather_than_defaulting_its_terms() {
        let text = r#"(schema_version: 1, contracts: [Contract(id: "a", name: "A", mix: [("boxed_ketchup", 1)], cadence_ticks: 100, payout: 5)])"#;
        assert!(ContractRegistry::from_ron_str(text).is_err());
        // And a v1 version stamp on an otherwise complete file is refused
        // by the version check itself.
        let stamped = EXAMPLE.replacen("schema_version: 2", "schema_version: 1", 1);
        let err = ContractRegistry::from_ron_str(&stamped).unwrap_err();
        assert!(matches!(err, ContractLoadError::UnsupportedSchemaVersion { found: 1, .. }));
    }

    #[test]
    fn rejects_a_file_with_no_contracts_at_all() {
        let err = ContractRegistry::from_ron_str("(schema_version: 2, contracts: [])").unwrap_err();
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
    fn rejects_a_zero_price() {
        let text = EXAMPLE.replacen("price: 40", "price: 0", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::ZeroPrice { ref id } => assert_eq!(id, "small"),
            other => panic!("expected ZeroPrice, got {other:?}"),
        }
        assert!(err.to_string().contains("price 0"));
    }

    #[test]
    fn rejects_zero_strikes() {
        let text = EXAMPLE.replacen("strikes: 3", "strikes: 0", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        match err {
            ContractLoadError::ZeroStrikes { ref id } => assert_eq!(id, "small"),
            other => panic!("expected ZeroStrikes, got {other:?}"),
        }
    }

    #[test]
    fn rejects_a_board_with_no_starter() {
        let text = EXAMPLE.replacen("starter: true,", "", 1);
        let err = ContractRegistry::from_ron_str(&text).unwrap_err();
        assert!(matches!(err, ContractLoadError::NoStarter));
        assert!(err.to_string().contains("starter"));
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
