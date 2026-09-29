//! THE COOKBOOK: the whole of `data/*.ron` read back as something a player
//! can page through — a sauce tree, and a page per sauce, machine and
//! ingredient.
//!
//! # Why this is in Rust at all
//!
//! Every fact on a Cookbook page already exists in the data files, and none
//! of it is *stated* there. "Ketchup is tier 0 and BBQ is tier 1" is not
//! written anywhere; it falls out of BBQ's mixer drawing a fluid the
//! ketchup vat makes. "A case of ketchup costs 11 in ingredients" is not
//! written anywhere either; it is four tomatoes at 1, one sugar at 3 and
//! two vinegar at 2, discovered by walking five recipes backwards. So the
//! Cookbook needs a *derivation*, and CLAUDE.md's one rule says where a
//! derivation goes: the sim decides, Godot renders.
//!
//! The alternative — a GDScript function that knows ketchup comes before
//! BBQ — is the exact failure the owner asked this feature to avoid. A
//! sauce added to `data/recipes.ron` has to appear in the book with no Rust
//! edit and no Godot edit, and that is only true if nothing anywhere names
//! a sauce.
//!
//! # What it is built from, and what it is NOT built from
//!
//! **No `World`.** This module takes registries and a delivery count, and
//! that is all it can see. There is no placed machine, no belt, no money
//! and no save file in here. `sim_core/tests/display_names.rs` states the
//! principle for its own map — "an empty world is enough: the map is
//! content read back, not state" — and it is stronger here, because it is
//! what makes the Cookbook the same book for every player. A page that
//! changed with the factory would be a HUD panel, and the game already has
//! five of those.
//!
//! The one exception is the padlock. Boxes sold decide what the market will
//! sell (`super::unlocks`), so they decide which pages are readable, and
//! that ledger is the only piece of world state that reaches this file. It
//! arrives as a plain `&BoxesSold` argument rather than as a `&World`
//! precisely so that nothing else can.
//!
//! # Four derivations, and where each one comes from
//!
//! 1. **Recipe depth** — how many crafts stand between a raw ingredient and
//!    this one — is the obtainability fixpoint `super::world::validate_chain`
//!    already runs to refuse a chain nothing can start, with the *iteration
//!    number* kept instead of thrown away. Same closure, same seed set
//!    (`super::ingredients::raw_ingredient_ids`, "the definition of 'raw'
//!    the whole crate uses"), one extra counter.
//!
//! 2. **A sauce** is a family of recipes rooted at something a contract
//!    asks for. Contracts are what the game is *for* (GAMEPLAY.md §6), so
//!    they are the right definition of "a product" — a chain nobody buys is
//!    an intermediate however long it is. Shallow roots claim shared
//!    upstream first, so the five ketchup stages belong to Ketchup rather
//!    than being duplicated into BBQ, and BBQ's page shows ketchup arriving
//!    as an input it can click through to.
//!
//! 3. **Tier** is depth in the family graph, not in the recipe graph: a
//!    sauce made only of raws is tier 0, and a sauce drawing on another
//!    sauce is one past the deepest one it draws on. That is the tree's
//!    column, and it is why adding a sauce that consumes BBQ would push a
//!    third column into the tree with nothing else changing.
//!
//! 4. **Lockedness is derived, never declared.** `data/unlocks.ron` gates
//!    ingredient ids and contract ids and deliberately never gates a
//!    recipe — its own header explains why at length. So "BBQ is locked"
//!    is really "BBQ needs spice mix, and spice mix is behind a gate", and
//!    this module works that out by walking a sauce's raws. A sauce whose
//!    every raw is on the shelf is readable; one with a shut gate anywhere
//!    upstream is a silhouette carrying the nearest gate that would open
//!    it.
//!
//! # What is deliberately absent
//!
//! **Display names.** Every id here goes out as an id, and the
//! presentation layer titles it through `_item_display_name` /
//! `_kind_display_name` exactly as it titles every other id on the wire —
//! `data/*.ron`'s own `display_name` fields reach it through
//! `display_names_json`, which is read once at load. Formatting is Godot's
//! job (`godot_project/CLAUDE.md`), and a second title-caser in Rust would
//! be a second thing to keep in step.
//!
//! **Seconds.** A step carries `time_ticks`, and the panel divides by
//! `TICK_HZ` — the same split `main.gd`'s load window already uses. The
//! tick rate belongs to the loop that drives the sim, not to the sim.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::contracts::ContractRegistry;
use super::dock::DOCK_CAPACITY;
use super::economy::STARTING_MONEY;
use super::guide::{GuideFact, GuideRegistry, GuideTopic};
use super::ingredients::{IngredientCatalog, raw_ingredient_ids};
use super::machines::MachineRegistry;
use super::recipe::RecipeRegistry;
use super::shop::ShopCatalog;
use super::staff::StaffCatalog;
use super::types::{Fx, Money};
use super::unlocks::{BoxesSold, Unlock, UnlockRegistry};

/// The whole book, as one value. Serialized to JSON by `lib.rs` and read by
/// `godot_project/cookbook_panel.gd`.
///
/// Money and quantities are `f64` **here and only here**, for
/// [`super::economy::EconomySnapshot::money`]'s reason: this is a snapshot
/// crossing the wire, not simulation state, and nothing on the other side
/// of it does arithmetic that has to agree with the sim's.
#[derive(Debug, Clone, Serialize)]
pub struct CookbookSnapshot {
    /// Every sauce, ordered by tier and then by the contract order that
    /// rooted it, which is the order the tree draws them in.
    pub sauces: Vec<SaucePage>,
    /// "This sauce is an ingredient of that one" — the tree's arrows.
    pub edges: Vec<SauceEdge>,
    /// Every machine kind in `data/machines.ron`, including the ones no
    /// recipe runs on (the hopper, the tank, the splitter, the dock, the
    /// shelving). A player wanting to know what a hopper is has the same
    /// question as one wanting to know what a mixer is.
    pub machines: Vec<MachinePage>,
    /// Every priced raw, in `data/ingredients.ron` order.
    pub ingredients: Vec<IngredientPage>,
    /// The book's BASICS pages — `data/guide.ron`'s topics in file order,
    /// **with their `{placeholder}`s still in the text**. Substituting them
    /// here would mean formatting money and ticks in `sim_core`, and a tick
    /// span has to be printed in seconds by whoever owns the tick rate
    /// (`godot_project/CLAUDE.md`: seconds, never ticks).
    pub guide: Vec<GuideTopic>,
    /// The values those placeholders stand for, one per
    /// [`super::guide::GuideFact`] and always all of them — a page that asks
    /// for a fact must never find it missing, so the wire carries the whole
    /// vocabulary rather than only what today's prose happens to use.
    pub facts: Vec<GuideFactLine>,
}

/// One fact, ready to be formatted by the presentation layer.
///
/// `kind` says which of the two value fields means anything: `money`,
/// `count` and `ticks` are in `number`, `text` is in `text`. Two fields
/// rather than an untagged union because the wire is JSON and a reader in
/// GDScript branches on a string far more legibly than it tests a type.
#[derive(Debug, Clone, Serialize)]
pub struct GuideFactLine {
    /// The placeholder name, without braces — what the prose asks for.
    pub id: String,
    /// `"money"`, `"count"`, `"ticks"` or `"text"`
    /// ([`super::guide::GuideFactKind::wire`]).
    pub kind: String,
    /// The numeric value for every kind but `text`, where it is 0.
    pub number: f64,
    /// The string value for kind `text`, empty otherwise.
    pub text: String,
}

/// One arrow in the tree: `from`'s product is consumed by `to`, arriving as
/// `via`.
#[derive(Debug, Clone, Serialize)]
pub struct SauceEdge {
    pub from: String,
    pub to: String,
    /// The id that actually crosses — `ketchup`, not `boxed_ketchup`. The
    /// arrow is labelled with it, which is the doc plate's own rule: every
    /// arrow is a sauce used as an ingredient, and it says which form.
    pub via: String,
    pub via_fluid: bool,
}

/// A sauce: a chain of recipes ending in something a contract wants.
#[derive(Debug, Clone, Serialize)]
pub struct SaucePage {
    /// The terminal item id — `boxed_ketchup`. This is the family's key
    /// everywhere, including in [`SauceEdge`].
    pub id: String,
    pub tagline: Option<String>,
    pub blurb: Option<String>,
    pub icon: Option<String>,
    /// Tree column: 0 for a sauce made only of bought ingredients.
    pub tier: u32,
    /// The process, raw end first.
    pub steps: Vec<SauceStep>,
    /// What one unit of [`Self::id`] costs to feed, broken down — bought
    /// ingredients and sauces of your own, each with the amount one case
    /// consumes.
    pub inputs: Vec<InputLine>,
    /// Those lines' total: the page's "per case in ingredients".
    pub cost_per_unit: f64,
    /// Names of the contracts that ask for this, in board order.
    pub contracts: Vec<String>,
    /// What those contracts pay per case, low and high. `None` when
    /// nothing on the board wants it — which is a real state worth showing
    /// rather than a zero.
    pub pay_low: Option<f64>,
    pub pay_high: Option<f64>,
    /// Whether a gate upstream is still shut. A locked sauce is drawn as a
    /// silhouette: it keeps its place in the tree, because the shape of
    /// what is coming is not a spoiler, but it gives up its name, its
    /// process and its numbers until it opens.
    pub locked: bool,
    /// The gate that would open it — the LAST one to open if more than one
    /// stands in the way, since that is the one the player is actually
    /// waiting on. `None` for a sauce with no gate anywhere upstream.
    pub gate: Option<GateLine>,
}

/// One machine in a sauce's process row.
#[derive(Debug, Clone, Serialize)]
pub struct SauceStep {
    pub recipe: String,
    pub machine: String,
    pub machine_icon: Option<String>,
    /// Ticks per craft. Seconds are the panel's business — see the module
    /// doc.
    pub time_ticks: u64,
    /// What leaves this step, as an id. Labels the arrow to the next one.
    pub carries: String,
    pub carries_fluid: bool,
    /// True when no shallower sauce uses this machine kind — the doc
    /// plate's `NEW` badge, worked out rather than authored. Every step of
    /// a tier-0 sauce is new; BBQ, which reuses the whole ketchup kit, has
    /// none.
    pub new_machine: bool,
}

/// One line of a sauce's shopping list, per case of finished product.
#[derive(Debug, Clone, Serialize)]
pub struct InputLine {
    pub id: String,
    pub icon: Option<String>,
    /// How much of it one case of the sauce consumes. Fractional when a
    /// recipe makes more than one unit per craft, hence not a `u32`.
    pub quantity: f64,
    pub fluid: bool,
    /// Set when this input is another sauce's product rather than
    /// something bought — the doc card's "tapped", and the page's link
    /// across to that sauce.
    pub from_sauce: Option<String>,
    /// Whether the market will sell it yet. A raw behind a shut gate is
    /// what makes a sauce a silhouette.
    pub locked: bool,
}

/// A machine kind's page.
#[derive(Debug, Clone, Serialize)]
pub struct MachinePage {
    pub id: String,
    pub blurb: Option<String>,
    pub icon: Option<String>,
    pub width_tiles: i32,
    pub height_tiles: i32,
    /// What the build menu charges for one. `None` for a kind
    /// `data/shop.ron` does not sell, which is a state the shop already
    /// allows.
    pub price: Option<f64>,
    /// The crew it claims while running, as it is authored — `(category
    /// id, category name, headcount)`.
    pub crew: Vec<CrewLine>,
    /// Ids it can produce, in `data/recipes.ron` order. Empty for the
    /// hopper, the tank, the splitter, the dock and the shelving, which is
    /// the honest answer: they move product without changing it.
    pub makes: Vec<String>,
}

/// One line of a machine's crew requirement.
#[derive(Debug, Clone, Serialize)]
pub struct CrewLine {
    pub category: String,
    /// The category's own player-facing name, out of `data/staff.ron` —
    /// content, so it comes from here rather than being title-cased.
    pub name: String,
    pub count: u32,
}

/// A bought ingredient's page.
#[derive(Debug, Clone, Serialize)]
pub struct IngredientPage {
    pub id: String,
    pub blurb: Option<String>,
    pub icon: Option<String>,
    pub price: f64,
    pub fluid: bool,
    /// The sauces that consume it, as sauce ids — the page's links back
    /// into the tree.
    pub used_in: Vec<String>,
    pub locked: bool,
    pub gate: Option<GateLine>,
}

/// A padlock, in the shape §5g's decision settled: what is waiting, how far
/// off it is, and how far along the player is.
#[derive(Debug, Clone, Serialize)]
pub struct GateLine {
    pub id: String,
    /// The unlock's authored name, e.g. "BBQ Sauce".
    pub name: String,
    /// The product whose boxes the gate counts, as an **id**
    /// (`boxed_ketchup`) — `cookbook_panel.gd` titles it through
    /// `_item_display_name`, the one display-name map.
    pub item: String,
    /// Boxes of `item` the gate opens at.
    pub required: u32,
    /// Boxes of `item` sold so far. Carried per padlock rather than once
    /// for the book, because since 2026-09-26 two gates no longer count the
    /// same thing.
    pub sold: u32,
}

/// Everything the book is built out of. A struct rather than nine arguments
/// because every caller passes the same nine, and because the next field
/// this needs should not change every call site.
pub struct CookbookInput<'a> {
    pub recipes: &'a RecipeRegistry,
    pub machines: &'a MachineRegistry,
    pub ingredients: &'a IngredientCatalog,
    pub contracts: &'a ContractRegistry,
    pub shop: &'a ShopCatalog,
    pub staff: &'a StaffCatalog,
    pub unlocks: &'a UnlockRegistry,
    pub guide: &'a GuideRegistry,
    /// Boxes sold per product — [`super::economy::Economy::lifetime_boxes_sold`].
    pub sold: &'a BoxesSold,
}

/// Reads the content files into [`CookbookSnapshot`].
///
/// Total: there is no error case. Every file this reads has already been
/// validated by its own loader, and a question this cannot answer — an
/// ingredient with no price, a sauce nothing buys — comes back as a `None`
/// on the page rather than as a failure, because a book that refuses to
/// open is worse than a book with a gap in it.
pub fn build(input: &CookbookInput<'_>) -> CookbookSnapshot {
    let ordered = input.recipes.ordered();

    // Which ids are fluids. Taken from usage rather than declared: an id is
    // a fluid exactly when a recipe moves it through a pipe. `types.rs`
    // makes ItemId and FluidId the same `String`, so this is the only way
    // to tell, and it is the same way `world.rs` tells.
    let mut fluids: BTreeSet<&str> = BTreeSet::new();
    for recipe in ordered {
        for (id, _) in recipe
            .fluid_inputs
            .iter()
            .chain(recipe.fluid_outputs.iter())
        {
            fluids.insert(id.as_str());
        }
    }

    // Who makes what. First writer wins: two recipes producing one id is
    // not something the shipped content does, and picking the earlier one
    // keeps the book deterministic if it ever happens.
    let mut producer: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, recipe) in ordered.iter().enumerate() {
        for (id, _) in recipe.outputs.iter().chain(recipe.fluid_outputs.iter()) {
            producer.entry(id.as_str()).or_insert(index);
        }
    }

    let depth = recipe_depths(input);

    // --- sauces -----------------------------------------------------------
    //
    // Roots, shallowest first. `contracts.ron` order breaks a tie, so the
    // board's own priority decides which of two equally deep sauces claims
    // a shared stage — content's call, not this file's.
    let mut roots: Vec<(usize, usize, &str)> = Vec::new();
    let mut seen_root: BTreeSet<&str> = BTreeSet::new();
    for (position, contract) in input.contracts.ordered().iter().enumerate() {
        for (item, _) in &contract.mix {
            let id = item.as_str();
            if !seen_root.insert(id) {
                continue;
            }
            let Some(&index) = producer.get(id) else {
                continue;
            };
            roots.push((depth[index], position, id));
        }
    }
    roots.sort_unstable();

    // A chain nobody buys still gets a page. Its root is an output no other
    // recipe consumes — the end of the line, wherever the line stops. This
    // is empty against the shipped content and is here so that content
    // added ahead of the contract that will sell it is never invisible.
    let consumed: BTreeSet<&str> = ordered
        .iter()
        .flat_map(|r| r.inputs.iter().chain(r.fluid_inputs.iter()))
        .map(|(id, _)| id.as_str())
        .collect();
    for (index, recipe) in ordered.iter().enumerate() {
        for (id, _) in recipe.outputs.iter().chain(recipe.fluid_outputs.iter()) {
            if !consumed.contains(id.as_str()) && seen_root.insert(id.as_str()) {
                roots.push((depth[index], usize::MAX, id.as_str()));
            }
        }
    }

    // Claim upstream stages, shallowest root first.
    let mut owner: Vec<Option<usize>> = vec![None; ordered.len()];
    let mut families: Vec<(&str, Vec<usize>)> = Vec::new();
    for &(_, _, root) in &roots {
        let family = families.len();
        let mut mine: Vec<usize> = Vec::new();
        let mut frontier = vec![root];
        let mut walked: BTreeSet<&str> = BTreeSet::new();
        while let Some(id) = frontier.pop() {
            if !walked.insert(id) {
                continue;
            }
            let Some(&index) = producer.get(id) else {
                continue;
            };
            if owner[index].is_some() {
                continue;
            }
            owner[index] = Some(family);
            mine.push(index);
            for (input_id, _) in ordered[index]
                .inputs
                .iter()
                .chain(ordered[index].fluid_inputs.iter())
            {
                frontier.push(input_id.as_str());
            }
        }
        mine.sort_unstable_by_key(|&i| (depth[i], i));
        families.push((root, mine));
    }

    // Tier: one past the deepest sauce this one draws on. Iterated to a
    // fixpoint rather than recursed, because the family graph is a DAG only
    // as far as the content is, and a loop here must settle rather than
    // blow the stack.
    let mut tier: Vec<u32> = vec![0; families.len()];
    for _ in 0..families.len() {
        let mut moved = false;
        for (family, (_, mine)) in families.iter().enumerate() {
            let mut want = 0;
            for &index in mine {
                for (id, _) in ordered[index]
                    .inputs
                    .iter()
                    .chain(ordered[index].fluid_inputs.iter())
                {
                    if let Some(&source) = producer.get(id.as_str())
                        && let Some(other) = owner[source]
                        && other != family
                    {
                        want = want.max(tier[other] + 1);
                    }
                }
            }
            if want > tier[family] {
                tier[family] = want;
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    // Which machine kinds a shallower sauce already uses — the `NEW` badge.
    let mut kinds_by_tier: BTreeMap<u32, BTreeSet<&str>> = BTreeMap::new();
    for (family, (_, mine)) in families.iter().enumerate() {
        for &index in mine {
            kinds_by_tier
                .entry(tier[family])
                .or_default()
                .insert(ordered[index].machine.as_str());
        }
    }

    let mut edges: Vec<SauceEdge> = Vec::new();
    let mut sauces: Vec<SaucePage> = Vec::new();
    let mut used_in: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (family, (root, mine)) in families.iter().enumerate() {
        let member: BTreeSet<usize> = mine.iter().copied().collect();

        let steps: Vec<SauceStep> = mine
            .iter()
            .map(|&index| {
                let recipe = &ordered[index];
                let kind = recipe.machine.as_str();
                let carries = recipe
                    .outputs
                    .iter()
                    .chain(recipe.fluid_outputs.iter())
                    .map(|(id, _)| id.clone())
                    .next()
                    .unwrap_or_default();
                let earlier = kinds_by_tier
                    .range(..tier[family])
                    .any(|(_, kinds)| kinds.contains(kind));
                SauceStep {
                    recipe: recipe.id.clone(),
                    machine: recipe.machine.clone(),
                    machine_icon: input.machines.get(kind).and_then(|k| k.icon.clone()),
                    time_ticks: recipe.time_ticks,
                    carries_fluid: fluids.contains(carries.as_str()),
                    carries,
                    new_machine: !earlier,
                }
            })
            .collect();

        // What one case consumes, expanded through this family's own stages
        // and stopping at anything it does not make.
        let mut wants: BTreeMap<String, Fx> = BTreeMap::new();
        expand(root, Fx::ONE, ordered, &producer, &member, &mut wants, 0);

        let mut inputs: Vec<InputLine> = Vec::new();
        for (id, quantity) in &wants {
            let from_sauce = producer
                .get(id.as_str())
                .and_then(|&index| owner[index])
                .filter(|&other| other != family)
                .map(|other| families[other].0.to_string());
            if let Some(source) = &from_sauce {
                edges.push(SauceEdge {
                    from: source.clone(),
                    to: (*root).to_string(),
                    via: id.clone(),
                    via_fluid: fluids.contains(id.as_str()),
                });
            } else {
                used_in
                    .entry(id.clone())
                    .or_default()
                    .push((*root).to_string());
            }
            inputs.push(InputLine {
                icon: input
                    .ingredients
                    .get(id)
                    .and_then(|e| e.icon.clone())
                    .or_else(|| {
                        producer
                            .get(id.as_str())
                            .and_then(|&i| ordered[i].icon.clone())
                    }),
                quantity: quantity.to_num::<f64>(),
                fluid: fluids.contains(id.as_str()),
                locked: from_sauce.is_none() && !input.unlocks.ingredient_available(id, input.sold),
                from_sauce,
                id: id.clone(),
            });
        }

        // The gate the player is actually waiting on: the last one of the
        // ones still shut. Raws only — a sauce arriving from another sauce
        // carries its own gate on its own page.
        //
        // "Last" is `data/unlocks.ron`'s file order, which that file's header
        // states IS the tier order. It used to be the largest
        // `after_deliveries`, and that stopped being an answer when each gate
        // started counting boxes of a different product: 100 boxes of ketchup
        // and 100 boxes of BBQ are not two numbers you can compare.
        let gate = wants
            .keys()
            .filter(|id| !input.unlocks.ingredient_available(id, input.sold))
            .filter_map(|id| input.unlocks.gate_index_for_ingredient(id))
            .max()
            .and_then(|index| input.unlocks.at(index))
            .map(|unlock| gate_line(unlock, input.sold));

        let mut contracts: Vec<String> = Vec::new();
        let mut pay_low: Option<Money> = None;
        let mut pay_high: Option<Money> = None;
        for contract in input.contracts.ordered() {
            let cases: u32 = contract.mix.iter().map(|(_, n)| n).sum();
            if cases == 0 || !contract.mix.iter().any(|(id, _)| id == root) {
                continue;
            }
            contracts.push(contract.name.clone());
            let per_case = contract.payout / Money::from_num(cases);
            pay_low = Some(pay_low.map_or(per_case, |low: Money| low.min(per_case)));
            pay_high = Some(pay_high.map_or(per_case, |high: Money| high.max(per_case)));
        }

        // The terminal recipe carries the sauce's own prose: it is the one
        // whose id a player would call the sauce by.
        let terminal = producer.get(*root).map(|&index| &ordered[index]);

        sauces.push(SaucePage {
            id: (*root).to_string(),
            tagline: terminal.and_then(|r| r.tagline.clone()),
            blurb: terminal.and_then(|r| r.blurb.clone()),
            icon: terminal.and_then(|r| r.cookbook_icon.clone().or_else(|| r.icon.clone())),
            tier: tier[family],
            steps,
            cost_per_unit: unit_cost(root, ordered, &producer, input.ingredients, &mut Vec::new())
                .unwrap_or(Money::ZERO)
                .to_num::<f64>(),
            locked: inputs.iter().any(|line| line.locked),
            gate,
            inputs,
            contracts,
            pay_low: pay_low.map(|m| m.to_num::<f64>()),
            pay_high: pay_high.map(|m| m.to_num::<f64>()),
        });
    }

    sauces.sort_by_key(|sauce| sauce.tier);
    edges.sort_by(|a, b| (&a.from, &a.to, &a.via).cmp(&(&b.from, &b.to, &b.via)));
    edges.dedup_by(|a, b| (&a.from, &a.to, &a.via) == (&b.from, &b.to, &b.via));

    // --- machines ---------------------------------------------------------
    // Ordered by how far down a chain the kind first appears, so the list
    // reads as the line does — hopper and crusher at the top, dock at the
    // bottom — and a kind no recipe runs on sorts after the ones that do.
    // Derived, because `MachineRegistry::kinds` has no file order to give.
    let mut first_use: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, recipe) in ordered.iter().enumerate() {
        let entry = first_use
            .entry(recipe.machine.as_str())
            .or_insert(depth[index]);
        *entry = (*entry).min(depth[index]);
    }
    let mut machines: Vec<MachinePage> = Vec::new();
    let mut kinds = input.machines.kinds();
    kinds.sort_by_key(|kind| {
        (
            first_use
                .get(kind.id.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            kind.id.clone(),
        )
    });
    for kind in kinds {
        let id = kind.id.clone();
        machines.push(MachinePage {
            blurb: kind.blurb.clone(),
            icon: kind.icon.clone(),
            width_tiles: kind.footprint.width_tiles,
            height_tiles: kind.footprint.height_tiles,
            price: input
                .shop
                .get(&id)
                .map(|entry| entry.price_money().to_num::<f64>()),
            crew: kind
                .staff
                .iter()
                .map(|(category, count)| CrewLine {
                    name: input
                        .staff
                        .categories()
                        .iter()
                        .find(|c| &c.id == category)
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| category.clone()),
                    category: category.clone(),
                    count: *count,
                })
                .collect(),
            makes: ordered
                .iter()
                .filter(|recipe| recipe.machine == id)
                .flat_map(|recipe| recipe.outputs.iter().chain(recipe.fluid_outputs.iter()))
                .map(|(item, _)| item.clone())
                .collect(),
            id,
        });
    }

    // --- ingredients ------------------------------------------------------
    let raws = raw_ingredient_ids(input.recipes);
    let ingredients: Vec<IngredientPage> = input
        .ingredients
        .entries()
        .iter()
        .filter(|entry| raws.iter().any(|raw| raw == &entry.id))
        .map(|entry| IngredientPage {
            id: entry.id.clone(),
            blurb: entry.blurb.clone(),
            icon: entry.icon.clone(),
            price: entry.price_money().to_num::<f64>(),
            fluid: fluids.contains(entry.id.as_str()),
            used_in: used_in.get(entry.id.as_str()).cloned().unwrap_or_default(),
            locked: !input.unlocks.ingredient_available(&entry.id, input.sold),
            gate: input
                .unlocks
                .gate_for_ingredient(&entry.id)
                .map(|unlock| gate_line(unlock, input.sold)),
        })
        .collect();

    // The BASICS pages go out as authored; what the book computes for them
    // is the facts their `{placeholder}`s stand for.
    let open_raws = ingredients.iter().filter(|page| !page.locked).count();
    let facts = guide_facts(input, sauces.len(), ingredients.len(), open_raws);

    CookbookSnapshot {
        guide: input.guide.ordered().to_vec(),
        facts,
        sauces,
        edges,
        machines,
        ingredients,
    }
}

/// One padlock, read off the gate and the ledger. Two call sites build the
/// same line — a sauce page's deepest shut gate and a raw ingredient's own —
/// and a `GateLine` assembled twice is a `GateLine` that can disagree with
/// itself.
fn gate_line(unlock: &Unlock, sold: &BoxesSold) -> GateLine {
    let progress = unlock.progress(sold);
    GateLine {
        id: unlock.id.clone(),
        name: progress.name,
        item: progress.item,
        required: progress.required,
        sold: progress.sold,
    }
}

/// Every fact in [`GuideFact::ALL`], in that order.
///
/// **All of them, every time, whether the shipped prose uses them or not.**
/// A page that asks for a fact the wire left out would render a gap, and the
/// whole reason the vocabulary is closed and checked at load
/// (`super::guide`) is that a missing number must be impossible rather than
/// merely unlikely — so the two halves meet here: the loader refuses a
/// placeholder nothing answers, and this refuses to answer less than
/// everything.
///
/// `sauce_count`, `raw_count` and `open_raw_count` come in as arguments
/// because they are what [`build`] has just finished deciding: how many
/// sauces the book ended up with is a fact about the book, not about a
/// registry, and re-deriving it here would be a second definition of "a
/// sauce" — or, for the open count, a second reading of the same gate the
/// ingredient pages already resolved.
fn guide_facts(
    input: &CookbookInput<'_>,
    sauce_count: usize,
    raw_count: usize,
    open_raw_count: usize,
) -> Vec<GuideFactLine> {
    // The cheapest thing on the shelf, by the price the shop actually
    // charges. Ties go to the earlier entry (`data/shop.ron` order), which
    // is the order the build menu shows them in, so the book names the one
    // the player sees first.
    let cheapest = input
        .shop
        .entries()
        .iter()
        .min_by_key(|entry| entry.price_money());

    // The earliest gate, which is the one a new game is walking towards.
    // File order, for the same reason the sauce page's deepest gate uses it:
    // two gates' thresholds count different products and cannot be ordered
    // against each other, and `data/unlocks.ron` orders itself by tier.
    let first_gate = input.unlocks.ordered().first();

    // Orders open to sign right now: no gate, or a gate that has opened.
    // The board lists the gated rest too, padlocked. This is what "open to a brand-new factory" means in the
    // guide's prose, and it is not `starters()` — a starter is one the game
    // signs for you, and since 2026-09-26 `data/contracts.ron` marks none,
    // so `starters()` is empty while this list is not.
    let open_contracts = input
        .contracts
        .ordered()
        .iter()
        .filter(|contract| input.unlocks.contract_available(&contract.id, input.sold))
        .count();

    let starting_crew: u32 = input
        .staff
        .categories()
        .iter()
        .map(|category| category.starting_headcount)
        .sum();
    let starting_wage_bill: u32 = input
        .staff
        .categories()
        .iter()
        .map(|category| category.starting_headcount * category.wage)
        .sum();

    GuideFact::ALL
        .into_iter()
        .map(|fact| {
            // A number for every kind but `text`/`kind_id`, a string for
            // those two — and an absent answer is 0 or empty rather than a
            // failure, for the reason `build` itself has no error case: a
            // book with a gap in it beats a book that will not open. Content
            // with no shop entries or no gates at all is a test fixture, not
            // a shipped game.
            let (number, text) = match fact {
                GuideFact::StartingMoney => (f64::from(STARTING_MONEY), String::new()),
                GuideFact::CheapestBuyable => {
                    (0.0, cheapest.map(|e| e.id.clone()).unwrap_or_default())
                }
                GuideFact::CheapestBuyablePrice => (
                    cheapest
                        .map(|e| e.price_money().to_num::<f64>())
                        .unwrap_or(0.0),
                    String::new(),
                ),
                GuideFact::BuyableCount => (input.shop.entries().len() as f64, String::new()),
                GuideFact::RawCount => (raw_count as f64, String::new()),
                GuideFact::OpenRawCount => (open_raw_count as f64, String::new()),
                GuideFact::SauceCount => (sauce_count as f64, String::new()),
                GuideFact::ContractCount => (input.contracts.len() as f64, String::new()),
                GuideFact::OpenContractCount => (open_contracts as f64, String::new()),
                GuideFact::StarterContracts => {
                    (input.contracts.starters().count() as f64, String::new())
                }
                GuideFact::DockCapacity => (f64::from(DOCK_CAPACITY), String::new()),
                GuideFact::StartingCrew => (f64::from(starting_crew), String::new()),
                GuideFact::StartingWageBill => (f64::from(starting_wage_bill), String::new()),
                GuideFact::PaydayTicks => (input.staff.pay_period_ticks() as f64, String::new()),
                GuideFact::FirstGateName => {
                    (0.0, first_gate.map(|u| u.name.clone()).unwrap_or_default())
                }
                GuideFact::FirstGateBoxes => (
                    first_gate
                        .map(|u| f64::from(u.after_sold.boxes))
                        .unwrap_or(0.0),
                    String::new(),
                ),
                GuideFact::FirstGateProduct => (
                    0.0,
                    first_gate
                        .map(|u| u.after_sold.item.clone())
                        .unwrap_or_default(),
                ),
            };
            GuideFactLine {
                id: fact.placeholder().to_string(),
                kind: fact.kind().wire().to_string(),
                number,
                text,
            }
        })
        .collect()
}

/// How many crafts stand between a raw ingredient and each recipe's output.
///
/// This is `world::validate_chain`'s obtainability closure with the
/// iteration counter kept. Everything about the loop is that one's: the
/// seed is what the market prices, a recipe enters the set when everything
/// it eats is already in it, and a stage fed only by an unreachable one
/// never enters at all. Such a recipe gets depth 0 here rather than being
/// dropped — the chain validator has already refused to build a world out
/// of it, so this only ever sees one in a test fixture, and a book that
/// omits a stage is harder to debug than one that draws it at the wrong
/// column.
fn recipe_depths(input: &CookbookInput<'_>) -> Vec<usize> {
    let ordered = input.recipes.ordered();
    let mut obtainable: BTreeSet<String> = raw_ingredient_ids(input.recipes)
        .into_iter()
        .filter(|id| input.ingredients.get(id).is_some())
        .collect();
    let mut depth: Vec<usize> = vec![0; ordered.len()];
    let mut placed: Vec<bool> = vec![false; ordered.len()];
    for round in 0..ordered.len() {
        let mut grew = false;
        let mut arrived: Vec<String> = Vec::new();
        for (index, recipe) in ordered.iter().enumerate() {
            if placed[index] {
                continue;
            }
            let fed = recipe
                .inputs
                .iter()
                .chain(recipe.fluid_inputs.iter())
                .all(|(id, _)| obtainable.contains(id));
            if !fed {
                continue;
            }
            placed[index] = true;
            depth[index] = round;
            grew = true;
            for (id, _) in recipe.outputs.iter().chain(recipe.fluid_outputs.iter()) {
                arrived.push(id.clone());
            }
        }
        obtainable.extend(arrived);
        if !grew {
            break;
        }
    }
    depth
}

/// How much of everything one unit of `item` consumes, expanded through
/// `member`'s recipes and stopping at anything they do not make.
///
/// `guard` is a plain depth counter rather than a visited set on purpose: a
/// legitimate chain reaches one ingredient down several paths and must add
/// them up, so "seen it" is not a stop condition. What must stop is a cycle,
/// and the recipe count bounds any acyclic walk.
fn expand(
    item: &str,
    quantity: Fx,
    ordered: &[super::recipe::Recipe],
    producer: &BTreeMap<&str, usize>,
    member: &BTreeSet<usize>,
    into: &mut BTreeMap<String, Fx>,
    guard: usize,
) {
    if guard > ordered.len() {
        return;
    }
    let made_here = producer
        .get(item)
        .copied()
        .filter(|index| member.contains(index));
    let Some(index) = made_here else {
        *into.entry(item.to_string()).or_insert(Fx::ZERO) += quantity;
        return;
    };
    let recipe = &ordered[index];
    let per_craft: u32 = recipe
        .outputs
        .iter()
        .chain(recipe.fluid_outputs.iter())
        .find(|(id, _)| id == item)
        .map(|(_, n)| *n)
        .unwrap_or(1);
    let crafts = quantity / Fx::from_num(per_craft.max(1));
    for (id, count) in recipe.inputs.iter().chain(recipe.fluid_inputs.iter()) {
        expand(
            id,
            crafts * Fx::from_num(*count),
            ordered,
            producer,
            member,
            into,
            guard + 1,
        );
    }
}

/// What one unit of `item` costs in bought ingredients, all the way down.
///
/// `None` when the walk reaches something neither priced nor produced, or
/// when it comes back round on itself — both of which mean the page shows
/// no cost rather than a wrong one.
fn unit_cost(
    item: &str,
    ordered: &[super::recipe::Recipe],
    producer: &BTreeMap<&str, usize>,
    ingredients: &IngredientCatalog,
    visiting: &mut Vec<String>,
) -> Option<Money> {
    if let Some(entry) = ingredients.get(item) {
        return Some(entry.price_money());
    }
    if visiting.iter().any(|seen| seen == item) {
        return None;
    }
    let index = *producer.get(item)?;
    let recipe = &ordered[index];
    let per_craft: u32 = recipe
        .outputs
        .iter()
        .chain(recipe.fluid_outputs.iter())
        .find(|(id, _)| id == item)
        .map(|(_, n)| *n)
        .unwrap_or(1);
    visiting.push(item.to_string());
    let mut total = Money::ZERO;
    for (id, count) in recipe.inputs.iter().chain(recipe.fluid_inputs.iter()) {
        let Some(each) = unit_cost(id, ordered, producer, ingredients, visiting) else {
            visiting.pop();
            return None;
        };
        total += each * Money::from_num(*count);
    }
    visiting.pop();
    Some(total / Money::from_num(per_craft.max(1)))
}
