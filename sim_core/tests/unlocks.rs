//! Headless integration test for the first progression gate
//! (`data/unlocks.ron`, docs/BBQ_SAUCE_PLAN.md §5).
//!
//! `sim_core/src/sim/unlocks.rs` already unit-tests the loader — the schema
//! rejection, the cross-file validation, the threshold arithmetic — against
//! hand-written content. This file drives the real `World` through the real
//! shipped chain, to prove the four things only the whole system can show:
//!
//! 1. **The ledger means what the gate claims it means.** It is boxes of a
//!    named product SOLD, so a factory that ships nothing never advances it
//!    however long it runs, and a window that carries three boxes advances it
//!    by three and not by one.
//! 2. **The gate is shut the box before it opens.** Ninety-nine boxes of
//!    ketchup is still a locked market and a BBQ-free board; the hundredth
//!    opens both, in the same tick, with nothing else changed — and a hundred
//!    boxes of anything ELSE opens nothing.
//! 3. **A locked id is genuinely unavailable, not merely unlisted.** The
//!    simulation refuses `BuyIngredient` and `SignContract` for one, so a
//!    presentation layer that forgot to filter — or a player driving the
//!    bridge directly — still cannot spend its way past the gate. This is
//!    the assertion that makes the gate a rule rather than a decoration.
//! 4. **It survives a save.** The ledger round-trips byte-equal, and the
//!    loaded world derives the same open gates from it.

mod common;

use std::collections::BTreeSet;

use sim_core::sim::save::{SAVE_SCHEMA_VERSION, SaveError};
use sim_core::sim::types::Money;
use sim_core::sim::{
    ContractRegistry, FLUID_TANK_CAPACITY, PlayerCommand, RecipeRegistry, UnlockRegistry, World,
    reference_chain_with_budget,
};

/// Every id the shipped gate holds back, read from the content rather than
/// repeated here — a balance pass that moves an id between tiers must not
/// need this file edited to keep passing.
fn gated() -> (Vec<String>, Vec<String>) {
    let unlock = bbq_gate();
    (unlock.ingredients, unlock.contracts)
}

/// The shipped BBQ gate, read from the content — the product it waits on and
/// the count it wants are `data/unlocks.ron`'s to choose, so no test below
/// writes either as a literal.
fn bbq_gate() -> sim_core::sim::Unlock {
    UnlockRegistry::bundled()
        .get("bbq_sauce")
        .expect("data/unlocks.ron must ship the bbq_sauce gate")
        .clone()
}

/// Boxes of `item` this world has sold.
fn sold(world: &World, item: &str) -> u32 {
    world.boxes_sold().get(item).copied().unwrap_or(0)
}

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// Stock plus a wage reserve, exactly as `tests/economy.rs` sizes it: these
/// fixtures run for tens of thousands of ticks, and a broke factory loses
/// its crew and then stops delivering, which would look like the counter
/// being wrong.
const BUDGET: u32 = common::STOCK_COST + common::WAGE_RESERVE;

fn stocked_chain() -> World {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    common::stock_up(&mut world);
    world
}

/// Runs the shipped chain, kept fed, until its lifetime delivery counter
/// reaches `target` — and stops on the *exact* tick it does.
///
/// Panics rather than returning short: every caller's assertions are
/// meaningless against a world that never got there, and a silent shortfall
/// would read as a gate bug.
fn run_until_delivered(world: &mut World, target: u32) {
    for _ in 0..500_000u32 {
        if world.lifetime_deliveries() >= target {
            return;
        }
        common::supply(world);
        world.tick();
    }
    panic!(
        "the shipped chain must reach {target} deliveries within 500k ticks, got {}",
        world.lifetime_deliveries()
    );
}

/// Runs the shipped chain, kept fed and kept stocked, until it has SOLD
/// `target` boxes of `item`, and returns the tick it happened on.
///
/// It restocks as it goes, which [`run_until_delivered`]'s callers do not need
/// to: a hundred boxes is about 52 000 ticks on the starter board, which is
/// more raw than one larder holds, and a fixture that ran out would stall,
/// lose its contracts and look exactly like a gate that never opens.
fn run_until_sold(world: &mut World, item: &str, target: u32) -> u32 {
    for tick in 0..500_000u32 {
        if sold(world, item) >= target {
            return tick;
        }
        if tick % 500 == 0 {
            common::restock(world);
        }
        common::supply(world);
        world.tick();
    }
    panic!(
        "the shipped chain must sell {target} boxes of {item} within 500k ticks, got {}",
        sold(world, item)
    );
}

/// The counter is satisfied windows, not ticks, not boxes and not money.
///
/// **Nothing gates off it since 2026-09-26** — the gates read the boxes-sold
/// ledger, which is [`the_ledger_counts_every_box_of_a_satisfied_window`]'s
/// subject. It is still saved and still displayed, so it is still pinned.
#[test]
fn the_counter_advances_once_per_satisfied_window_and_not_at_all_without_one() {
    // An unstocked world: the factory is fully built and running, and has
    // nothing to run *on*. The fixture signs `corner_shop` on tick 0 and its
    // windows come round every 1 000 ticks, so this passes several of them.
    let mut starving = reference_chain_with_budget(recipes(), BUDGET);
    assert_eq!(
        starving.lifetime_deliveries(),
        0,
        "a new game has shipped nothing"
    );
    for _ in 0..5_000 {
        starving.tick();
    }
    assert_eq!(
        starving.lifetime_deliveries(),
        0,
        "a missed window is not a delivery — the counter must not advance on the \
         clock alone, or the gate would open for a factory that never ran"
    );

    // The same chain, fed. The counter must agree with the per-contract
    // ledgers it is the lifetime sum of.
    let mut world = stocked_chain();
    run_until_delivered(&mut world, 3);
    let per_contract: u32 = world
        .economy()
        .contracts
        .iter()
        .map(|active| active.deliveries_made)
        .sum();
    assert_eq!(
        world.lifetime_deliveries(),
        per_contract,
        "the lifetime counter must be the sum of what the active contracts were \
         actually paid for"
    );
}

/// The ledger the gates actually read: **boxes**, one entry per product id,
/// and every box of a satisfied window counted rather than one per window.
///
/// This is the whole point of the 2026-09-26 trigger change. A window counted
/// the same whether it carried one box or eight, so the old number measured
/// elapsed windows and not output; this test fails if the new one does the
/// same, by checking the ledger against the boxes those windows actually
/// asked for and by insisting the two numbers DIFFER.
#[test]
fn the_ledger_counts_every_box_of_a_satisfied_window() {
    // Nothing shipped, nothing sold, however long the clock runs.
    let mut starving = reference_chain_with_budget(recipes(), BUDGET);
    assert!(
        starving.boxes_sold().is_empty(),
        "a new game has sold nothing, and an empty ledger is how it says so"
    );
    for _ in 0..5_000 {
        starving.tick();
    }
    assert!(
        starving.boxes_sold().is_empty(),
        "a missed window sells nothing — a gate that advanced on the clock would \
         open for a factory that never ran"
    );

    // A fed chain with a MULTI-BOX order signed, so "every box" has something
    // to be wrong about: `diner_supply` asks for three at a time.
    let mut world = stocked_chain();
    world.issue_command(PlayerCommand::SignContract {
        id: "diner_supply".to_string(),
    });
    run_until_delivered(&mut world, 6);

    // What the windows those contracts were paid for actually carried, read
    // off the board rather than written here, so a `contracts.ron` balance
    // pass cannot make this test wrong.
    let board = ContractRegistry::bundled();
    let mut expected: std::collections::BTreeMap<String, u32> = Default::default();
    let mut windows = 0;
    for active in &world.economy().contracts {
        let contract = board
            .get(&active.contract.id)
            .expect("an active contract is one the board offers");
        windows += active.deliveries_made;
        for (item, quantity) in &contract.mix {
            *expected.entry(item.clone()).or_insert(0) += quantity * active.deliveries_made;
        }
    }
    assert_eq!(
        world.boxes_sold(),
        &expected,
        "the ledger must be the sum of the MIX of every window that was paid for — \
         per product id, box by box"
    );
    let total: u32 = expected.values().sum();
    assert!(
        total > windows,
        "this fixture has to ship more boxes than windows for the assertion above to \
         be able to tell them apart: {total} boxes in {windows} windows"
    );
    assert_eq!(
        world.lifetime_deliveries(),
        windows,
        "and the old counter is still the window count, unchanged beside it"
    );
}

/// The gate itself: shut one box short, open on the threshold box, and open
/// for both lists at once.
///
/// The boundary is reached through a **save** rather than by playing, for the
/// reason [`world_with_boxes_sold`] documents — and here for one more: the
/// starter board sells ketchup two boxes to a window, so a played world goes
/// 96, 98, 100 and never visits 99 at all.
/// [`the_gate_opens_by_playing_and_is_shut_every_box_before_that`] is the
/// played half.
#[test]
fn the_gate_is_shut_one_box_short_and_open_on_the_threshold() {
    let gate = bbq_gate();
    let item = gate.after_sold.item.clone();
    let threshold = gate.after_sold.boxes;
    let (ingredients, contracts) = gated();

    let world = world_with_boxes_sold(&[(&item, threshold - 1)]);
    let market = world.ingredients_snapshot();
    let board = world.economy_snapshot();
    for id in &ingredients {
        assert!(
            !world.ingredient_unlocked(id),
            "'{id}' must still be locked one box short of the threshold"
        );
        assert!(
            !market.entries.iter().any(|e| &e.id == id),
            "'{id}' must not be listed in the market one box short"
        );
    }
    for id in &contracts {
        assert!(
            !world.contract_unlocked(id),
            "'{id}' must still be locked one box short of the threshold"
        );
        assert!(
            board.board.iter().any(|o| &o.id == id && o.locked),
            "'{id}' must be listed on the board padlocked one box short"
        );
    }

    let world = world_with_boxes_sold(&[(&item, threshold)]);
    assert_eq!(sold(&world, &item), threshold);
    for id in &ingredients {
        assert!(
            world.ingredient_unlocked(id),
            "'{id}' must be on sale on the threshold box"
        );
    }
    for id in &contracts {
        assert!(
            world.contract_unlocked(id),
            "'{id}' must be on the board on the threshold box"
        );
    }

    // Both lists, the same tick: the announcement is one sentence about one
    // tier, so a gate that released its ingredients before its contracts
    // would be two events and a lie.
    let market = world.ingredients_snapshot();
    let board = world.economy_snapshot();
    for id in &ingredients {
        assert!(
            market.entries.iter().any(|e| &e.id == id),
            "'{id}' must be listed in the market the tick the gate opens"
        );
    }
    for id in &contracts {
        assert!(
            board.board.iter().any(|o| &o.id == id && !o.locked),
            "'{id}' must be listed on the board unlocked the tick the gate opens"
        );
    }
    let bbq = board
        .unlocks
        .iter()
        .find(|u| u.id == "bbq_sauce")
        .expect("the snapshot must carry the gate, so the HUD can announce it");
    assert!(bbq.open);
    assert_eq!(bbq.name, "BBQ Sauce");
    assert_eq!(
        (bbq.item.as_str(), bbq.sold, bbq.required),
        (item.as_str(), threshold, threshold),
        "and it carries what it counted, so the HUD can say so without reading \
         data/unlocks.ron itself"
    );
}

/// The played half of the gate: a factory that runs the shipped chain and
/// nothing else reaches the BBQ tier by SELLING, and is shut at every box
/// before it.
///
/// The loop asserts the gate the whole way up rather than only at the end,
/// which is what would catch an off-by-one the other direction — a gate that
/// opened at the first box, or at half of them, passes an at-the-end check.
/// It is the expensive test in this file (about 52 000 ticks on the starter
/// board) and it is the only one that proves the number in
/// `data/unlocks.ron` is reachable at all by playing.
#[test]
fn the_gate_opens_by_playing_and_is_shut_every_box_before_that() {
    let gate = bbq_gate();
    let item = gate.after_sold.item.clone();
    let threshold = gate.after_sold.boxes;
    let raw = gate
        .ingredients
        .first()
        .expect("the BBQ gate holds back raws")
        .clone();

    let mut world = stocked_chain();
    let mut highest = 0;
    while sold(&world, &item) < threshold {
        assert!(
            !world.ingredient_unlocked(&raw),
            "'{raw}' must stay locked at {} boxes of {item}, which is short of {threshold}",
            sold(&world, &item)
        );
        highest = highest.max(sold(&world, &item));
        run_until_sold(&mut world, &item, highest + 1);
    }
    assert!(
        world.ingredient_unlocked(&raw),
        "selling {threshold} boxes of {item} on the shipped board is what opens the tier — \
         if this fails the number in data/unlocks.ron is not reachable by playing"
    );
    assert!(
        highest >= threshold - 4,
        "the loop has to get close before it stops, or 'shut every box before that' is \
         a claim about the first few: highest shut reading was {highest}"
    );
}

/// A hundred boxes of the WRONG product opens nothing. The gate names one
/// item, and this is the assertion that says it reads that item and not the
/// size of the ledger.
#[test]
fn boxes_of_another_product_never_open_a_gate() {
    let gate = bbq_gate();
    let raw = gate
        .ingredients
        .first()
        .expect("the BBQ gate holds back raws")
        .clone();
    let elsewhere = "boxed_mustard";
    assert_ne!(
        gate.after_sold.item, elsewhere,
        "this test needs a product the BBQ gate does NOT count"
    );

    let world = world_with_boxes_sold(&[(elsewhere, gate.after_sold.boxes * 10)]);
    assert!(
        !world.ingredient_unlocked(&raw),
        "'{raw}' waits on {} and nothing else — ten times the threshold in {elsewhere} \
         must leave it exactly as locked",
        gate.after_sold.item
    );
    for id in &gate.contracts {
        assert!(
            !world.contract_unlocked(id),
            "'{id}' waits on {} too",
            gate.after_sold.item
        );
    }
}

/// A world that has sold exactly `ledger` and run for no ticks at all — the
/// ledger is saved state, so writing it into a save is a legitimate way to
/// reach a threshold, and
/// [`the_ledger_and_the_gates_it_opens_survive_a_save_round_trip`] is what
/// proves the gates a loaded ledger derives are the same ones a *played* one
/// does ([`the_gate_opens_by_playing_and_is_shut_every_box_before_that`]).
///
/// Some thresholds have to be reached this way. The BBQ gate's hundred boxes
/// of ketchup are 52 000 ticks and worth simulating once; the mustard gate's
/// hundred boxes of BBQ are another 89 000 on top of a branch the player
/// must first build (tick 141 000 from a new game — `measure_gate_ticks.gd`),
/// which is about eighty minutes of simulated factory for a claim about a
/// number in a content file. A played world also never visits an odd
/// ketchup count, the starter order being two boxes wide, so an exact
/// "one short" is only reachable from here.
fn world_with_boxes_sold(ledger: &[(&str, u32)]) -> World {
    let entries: Vec<String> = ledger
        .iter()
        .map(|(item, boxes)| format!("\"{item}\": {boxes}"))
        .collect();
    let replacement = format!("  lifetime_boxes_sold: {{{}}},", entries.join(", "));
    let text = reference_chain_with_budget(recipes(), 1_000).to_save_ron();
    let mut seen = 0;
    let rewritten: Vec<String> = text
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("lifetime_boxes_sold:") {
                seen += 1;
                replacement.clone()
            } else {
                line.to_string()
            }
        })
        .collect();
    assert_eq!(
        seen, 1,
        "a save must carry exactly one boxes-sold ledger for this fixture to write it"
    );
    let world = World::from_save_ron(&rewritten.join("\n"), recipes())
        .expect("a save with a rewritten ledger must load");
    for (item, boxes) in ledger {
        assert_eq!(
            sold(&world, item),
            *boxes,
            "the fixture has to actually have sold what it claims"
        );
    }
    world
}

/// The third sauce's gate: it holds back every raw that feeds nothing but
/// mustard and every order that asks for a case of it, it holds back nothing
/// else, and it opens on its own threshold — which counts the tier BELOW it,
/// because since 2026-09-26 each sauce is earned by selling the sauce before
/// it.
///
/// **THE EXPECTED IDS COME FROM THE CONTENT, NOT FROM THE GATE.** The first
/// draft of this test looped over `mustard_sauce`'s own two lists, so
/// deleting an id from `data/unlocks.ron` made the loop vacuous instead of
/// false: the whole suite stayed green with a 416-money mustard order on a
/// brand-new game's board — signable, unfulfillable, straight to strikes.
/// So the two expected sets are derived here from `data/contracts.ron`
/// (every order whose mix names `boxed_mustard`) and from THE COOKBOOK's
/// `used_in` (every raw the recipe graph feeds to mustard and to nothing
/// else), the shut-on-a-new-game check runs over THOSE, and the gate's lists
/// are then compared with them in both directions — an omission and a stray
/// addition each fail, naming the id and the gate.
///
/// `vinegar` is the id this test exists to watch. Mustard drinks it and so
/// does the shipped ketchup line, so it must NOT be gated: a gate on it would
/// lock the starting factory out of its own ingredient. That is also why the
/// raw side is derived from `used_in` and not from "every raw a mustard
/// recipe touches" — the second phrasing would demand vinegar be gated.
#[test]
fn the_mustard_gate_holds_back_its_own_tier_and_nothing_the_first_line_needs() {
    // --- what the content says the tier is, read without the gate ---------
    let ordered_mustard: BTreeSet<String> = ContractRegistry::bundled()
        .ordered()
        .iter()
        .filter(|c| c.mix.iter().any(|(item, _)| item == "boxed_mustard"))
        .map(|c| c.id.clone())
        .collect();
    let book = World::new(recipes()).cookbook();
    let mustard_raws: BTreeSet<String> = book
        .ingredients
        .iter()
        .filter(|page| page.used_in == ["boxed_mustard"])
        .map(|page| page.id.clone())
        .collect();
    assert!(
        !ordered_mustard.is_empty() && !mustard_raws.is_empty(),
        "data/contracts.ron must order boxed_mustard and the recipe graph must have a raw that \
         feeds nothing else, or the mustard_sauce gate has nothing to hold back and this test \
         proves nothing"
    );
    assert!(
        !mustard_raws.contains("vinegar"),
        "vinegar feeds the shipped ketchup line as well, so it is not mustard's to gate"
    );

    // --- a new game can reach none of it -----------------------------------
    // These loops run over the DERIVED ids, so an id dropped from the gate
    // fails here by name rather than quietly shortening a list.
    let shut = world_with_boxes_sold(&[]);
    let market = shut.ingredients_snapshot();
    let board = shut.economy_snapshot();
    for id in &mustard_raws {
        assert!(
            !shut.ingredient_unlocked(id),
            "'{id}' feeds nothing but mustard, so a new game must not be able to buy it — the \
             mustard_sauce gate in data/unlocks.ron has to name it in `ingredients`"
        );
        assert!(
            !market.entries.iter().any(|e| &e.id == id),
            "'{id}' must not be listed in the market at nought deliveries — the mustard_sauce \
             gate in data/unlocks.ron has to name it in `ingredients`"
        );
    }
    for id in &ordered_mustard {
        assert!(
            !shut.contract_unlocked(id),
            "'{id}' orders a case of mustard, so a new game must not be able to sign it — the \
             mustard_sauce gate in data/unlocks.ron has to name it in `contracts`, or the board \
             offers an order for a sauce that cannot be made yet and it runs to strikes"
        );
        assert!(
            board.board.iter().any(|o| &o.id == id && o.locked),
            "'{id}' must be padlocked at nought deliveries — the mustard_sauce gate in \
             data/unlocks.ron has to name it in `contracts`"
        );
    }
    assert!(
        shut.ingredient_unlocked("vinegar"),
        "the first line's own fluid is on sale from the first tick"
    );
    assert!(
        shut.boxes_sold().is_empty(),
        "and 'a new game' here means a factory that has sold nothing at all"
    );

    // --- and the gate's own lists are exactly that, no more ----------------
    let unlocks = UnlockRegistry::bundled();
    let mustard = unlocks
        .get("mustard_sauce")
        .expect("data/unlocks.ron must ship the mustard_sauce gate")
        .clone();
    let bbq = unlocks.get("bbq_sauce").expect("the shipped gate").clone();
    // Each tier waits on the tier below it — the chain the owner's rule
    // describes. Read from the content in both directions: the product mustard
    // waits on has to be what the BBQ tier's own contracts SELL, so moving a
    // tier in this file cannot silently point a gate at a dead end.
    assert_eq!(
        bbq.after_sold.item, "boxed_ketchup",
        "the second sauce is earned by selling the first one's case"
    );
    assert_eq!(
        mustard.after_sold.item, "boxed_bbq",
        "and the third by selling the second's: {} is not what the BBQ tier ships",
        mustard.after_sold.item
    );
    assert!(
        ContractRegistry::bundled()
            .ordered()
            .iter()
            .filter(|c| bbq.contracts.contains(&c.id))
            .any(|c| c
                .mix
                .iter()
                .any(|(item, _)| item == &mustard.after_sold.item)),
        "the mustard gate counts boxes of '{}', so some contract the BBQ tier releases \
         has to BUY that — a gate waiting on a product nothing orders could never open",
        mustard.after_sold.item
    );
    assert!(
        mustard.after_sold.boxes > 0 && bbq.after_sold.boxes > 0,
        "a gate at zero boxes is a gate that is open in a new game"
    );
    let listed_raws: BTreeSet<String> = mustard.ingredients.iter().cloned().collect();
    let listed_contracts: BTreeSet<String> = mustard.contracts.iter().cloned().collect();
    let names = |ids: &BTreeSet<String>, other: &BTreeSet<String>| -> String {
        ids.difference(other)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    assert!(
        mustard_raws.is_subset(&listed_raws),
        "'{}' feeds nothing but mustard and the mustard_sauce gate in data/unlocks.ron does not \
         hold it back",
        names(&mustard_raws, &listed_raws)
    );
    assert!(
        listed_raws.is_subset(&mustard_raws),
        "the mustard_sauce gate in data/unlocks.ron holds back '{}', which the recipe graph says \
         is not a mustard-only raw",
        names(&listed_raws, &mustard_raws)
    );
    assert!(
        ordered_mustard.is_subset(&listed_contracts),
        "'{}' in data/contracts.ron asks for boxed_mustard and the mustard_sauce gate in \
         data/unlocks.ron does not hold it back",
        names(&ordered_mustard, &listed_contracts)
    );
    assert!(
        listed_contracts.is_subset(&ordered_mustard),
        "the mustard_sauce gate in data/unlocks.ron holds back '{}', which asks for no mustard",
        names(&listed_contracts, &ordered_mustard)
    );

    // One short of the threshold the tier is still shut, while the tier
    // BELOW it is open — which is what makes this gate its own and not a
    // side effect of the BBQ one.
    let nearly = world_with_boxes_sold(&[
        (&bbq.after_sold.item, bbq.after_sold.boxes),
        (&mustard.after_sold.item, mustard.after_sold.boxes - 1),
    ]);
    for id in &mustard_raws {
        assert!(
            !nearly.ingredient_unlocked(id),
            "'{id}' must still be locked one box short of the mustard_sauce gate"
        );
    }
    for id in &ordered_mustard {
        assert!(
            !nearly.contract_unlocked(id),
            "'{id}' must still be locked one box short of the mustard_sauce gate"
        );
    }
    for id in bbq.ingredients.iter().chain(bbq.contracts.iter()) {
        assert!(
            nearly.ingredient_unlocked(id) || nearly.contract_unlocked(id),
            "the BBQ tier's '{id}' opened long ago and stays open"
        );
    }

    // And on the threshold, every id the content calls mustard's, at once.
    let open = world_with_boxes_sold(&[
        (&bbq.after_sold.item, bbq.after_sold.boxes),
        (&mustard.after_sold.item, mustard.after_sold.boxes),
    ]);
    let market = open.ingredients_snapshot();
    let board = open.economy_snapshot();
    for id in &mustard_raws {
        assert!(
            open.ingredient_unlocked(id),
            "'{id}' must be on sale on the mustard_sauce gate's threshold box"
        );
        assert!(
            market.entries.iter().any(|e| &e.id == id),
            "'{id}' must be listed in the market the tick the mustard_sauce gate opens"
        );
    }
    for id in &ordered_mustard {
        assert!(
            open.contract_unlocked(id),
            "'{id}' must be on the board on the mustard_sauce gate's threshold box"
        );
        assert!(
            board.board.iter().any(|o| &o.id == id && !o.locked),
            "'{id}' must be offered unlocked the tick the mustard_sauce gate opens"
        );
    }
    let announced = board
        .unlocks
        .iter()
        .find(|u| u.id == "mustard_sauce")
        .expect("the snapshot must carry the gate, so the HUD can announce it");
    assert!(announced.open);
    assert_eq!(announced.name, "Mustard");
}

/// A gate is enforced by the simulation, not by a panel that chose not to
/// draw a row. Refusing the command is what makes it a rule.
#[test]
fn a_locked_ingredient_cannot_be_bought_however_the_command_arrives() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let before = world.money();

    for id in &ingredients {
        world.issue_command(PlayerCommand::BuyIngredient {
            item: id.clone(),
            quantity: 5,
        });
        assert_eq!(
            world.owned_ingredient(id),
            0,
            "'{id}' is behind a closed gate — buying it must be refused, not merely \
             hidden from the market panel"
        );
    }
    assert_eq!(
        world.money(),
        before,
        "a refused purchase is a total no-op: no stock, and no money spent either"
    );
}

/// The same, for the board. A signing fee is real money, so a refusal that
/// charged for it would be worse than no gate at all.
#[test]
fn a_locked_contract_cannot_be_signed_however_the_command_arrives() {
    let (_, contracts) = gated();
    let board = ContractRegistry::bundled();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let before = world.money();
    let active_before = world.economy().contracts.len();

    for id in &contracts {
        assert!(
            board.get(id).is_some(),
            "'{id}' must exist in data/contracts.ron for this to be a gate rather \
             than a typo"
        );
        world.issue_command(PlayerCommand::SignContract { id: id.clone() });
    }

    assert_eq!(
        world.economy().contracts.len(),
        active_before,
        "a contract behind a closed gate must not become active"
    );
    assert_eq!(
        world.money(),
        before,
        "a refused signing must not charge the signing fee"
    );
}

/// Determinism: the ledger is persisted state, so it round-trips exactly, and
/// the loaded world derives the same gates from it.
#[test]
fn the_ledger_and_the_gates_it_opens_survive_a_save_round_trip() {
    let (ingredients, contracts) = gated();

    // A PLAYED ledger rather than a written one: the round trip has to carry
    // whatever a running factory actually accumulates, several products
    // included, not the one tidy entry a fixture would write.
    let mut world = stocked_chain();
    world.issue_command(PlayerCommand::SignContract {
        id: "diner_supply".to_string(),
    });
    run_until_delivered(&mut world, 4);
    sim_core::sim::reference::open_every_gate(&mut world);
    assert!(
        !world.boxes_sold().is_empty(),
        "the fixture must have sold something for the round trip to be about anything"
    );

    let text = world.to_save_ron();
    let loaded = World::from_save_ron(&text, recipes()).expect("the save must load");

    assert_eq!(
        loaded.boxes_sold(),
        world.boxes_sold(),
        "the ledger is saved state, not something recomputed from the active \
         contracts — a lost contract must not re-lock a tier"
    );
    assert_eq!(
        loaded.lifetime_deliveries(),
        world.lifetime_deliveries(),
        "and the window counter still rides along beside it"
    );
    for id in ingredients.iter() {
        assert!(
            loaded.ingredient_unlocked(id),
            "'{id}' must stay unlocked across a load"
        );
    }
    for id in contracts.iter() {
        assert!(
            loaded.contract_unlocked(id),
            "'{id}' must stay unlocked across a load"
        );
    }
    assert_eq!(
        loaded.to_save_ron(),
        text,
        "a save of a loaded save must be byte-identical, ledger included"
    );
}

/// A save from before the trigger changed is **refused**, not read as a
/// factory that has sold nothing.
///
/// This is why `SAVE_SCHEMA_VERSION` moved to 5. Loading a v4 file with one
/// `#[serde(default)]` does not merely lose a counter: gate state is derived
/// from the ledger on every load, so a save that had BBQ open comes back with
/// the tier shut, its raws unbuyable and a signed BBQ contract stranded. A v4
/// file never recorded which products its deliveries carried, so no honest
/// ledger can be written from it — see `save.rs`'s module doc, "Version 5".
///
/// The refusal has to *say* that, in words a player can read, or the next
/// person to meet it files it as a parse bug.
#[test]
fn a_save_from_before_the_trigger_changed_is_refused_with_the_reason() {
    let world = reference_chain_with_budget(recipes(), 1_000);
    let text = world.to_save_ron();
    assert!(
        text.contains("lifetime_boxes_sold"),
        "a save written now records the ledger the gates read"
    );
    assert!(
        text.contains(&format!("schema_version: {SAVE_SCHEMA_VERSION}")),
        "and stamps the version that describes its shape"
    );

    // Exactly what a v4 save is: no ledger, and the version that predates it.
    let older: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("lifetime_boxes_sold:"))
        .collect::<Vec<_>>()
        .join("\n")
        .replace(
            &format!("schema_version: {SAVE_SCHEMA_VERSION}"),
            "schema_version: 4",
        );
    assert!(!older.contains("lifetime_boxes_sold"));

    let Err(error) = World::from_save_ron(&older, recipes()) else {
        panic!("a save from before the gates counted boxes must be refused, not guessed at");
    };
    assert!(
        matches!(
            error,
            SaveError::UnsupportedSchemaVersion {
                found: 4,
                supported: SAVE_SCHEMA_VERSION
            }
        ),
        "it is refused as an old VERSION, not as a broken file — got {error}"
    );

    let message = error.to_string();
    for phrase in ["delivery windows", "boxes of a", "will not guess"] {
        assert!(
            message.contains(phrase),
            "the refusal has to explain what changed, not just that a number differs — \
             `{phrase}` is missing from: {message}"
        );
    }
}

/// A save this build wrote loads back, ledger and all — the other side of the
/// refusal above, so a version bump cannot be "fixed" by refusing everything.
#[test]
fn a_save_this_build_wrote_still_round_trips() {
    let world = reference_chain_with_budget(recipes(), 1_000);
    let text = world.to_save_ron();
    let loaded = World::from_save_ron(&text, recipes()).expect("this build's own save must load");
    assert_eq!(
        loaded.to_save_ron(),
        text,
        "a loaded save must be byte-identical, ledger included"
    );
    assert_eq!(loaded.money(), Money::from_num(1_000));
}

// =====================================================================
// The contents picker (issue #90)
// =====================================================================
//
// A gate holds back an ingredient, and until this the two commands that
// point a BUILDING at one — `SetHopperItem` and `SetTankFluid` — accepted it
// anyway. The player could aim a hopper at an ingredient nobody could buy,
// and the hopper then sat empty forever saying only "in stock: 0". The
// market's rule ("refused, not merely hidden") now covers them, and the
// picker's wire carries what the panel needs to say so.

/// The reference chain's first hopper, and its tank — the two buildings the
/// configuration panel's CONTENTS picker is drawn for.
const FIRST_HOPPER: (i32, i32) = (-1, 0);

fn hopper_item(world: &World, tile: (i32, i32)) -> String {
    match world.query_tile(tile) {
        Some(sim_core::sim::TileSnapshot::Hopper { item, .. }) => item,
        other => panic!("expected a hopper at {tile:?}, found {other:?}"),
    }
}

/// Pointing a placed hopper at a locked ingredient is refused, and refused
/// *totally*: the bin keeps the item it had, so a misdirected click cannot
/// even empty it.
#[test]
fn a_hopper_cannot_be_pointed_at_a_locked_ingredient() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let before = hopper_item(&world, FIRST_HOPPER);
    assert!(
        !ingredients.contains(&before),
        "the fixture's hopper must start on an UNLOCKED item for this to mean anything"
    );

    for id in &ingredients {
        world.issue_command(PlayerCommand::SetHopperItem {
            tile: FIRST_HOPPER,
            item: id.clone(),
        });
        assert_eq!(
            hopper_item(&world, FIRST_HOPPER),
            before,
            "'{id}' is behind a closed gate, so the hopper must keep holding '{before}' — \
             a dimmed slot the simulation still honours is a decoration, not a gate"
        );
    }
}

/// And the same click, once the gate has opened, is accepted — so the
/// refusal above is the gate and not some other rule quietly declining.
#[test]
fn the_same_hopper_click_lands_once_the_gate_opens() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    sim_core::sim::reference::open_every_gate(&mut world);
    let wanted = ingredients
        .iter()
        .find(|id| world.raw_items().iter().any(|raw| raw == *id))
        .expect("the gate must hold back at least one RAW solid for this test");

    world.issue_command(PlayerCommand::SetHopperItem {
        tile: FIRST_HOPPER,
        item: wanted.clone(),
    });
    assert_eq!(
        &hopper_item(&world, FIRST_HOPPER),
        wanted,
        "with the gate open this is an ordinary retarget"
    );
}

/// Where the rule deliberately STOPS: a hopper may be placed holding a
/// locked ingredient, and is then a bin that can never be filled.
///
/// `Place`'s `item` is not a choice made against a gate. The build ghost
/// seeds it — and skips locked ids when it does (`_selected_hopper_item`) —
/// while a layout fixture names it to stand a factory up for a screenshot.
/// Refusing it would close nothing a player can reach and would cost every
/// BBQ fixture ten deliveries of real play before it could place a bin,
/// which `godot_project/test_layouts.gd` has no honest way to pay.
///
/// What makes that safe is the half asserted below: the gate still bites the
/// bin. `BuyIngredient` is refused for the locked raw, so the larder never
/// holds a unit of it, so neither a manual fill nor auto-fill can put
/// anything in the hopper. The player cannot reach a state the picker would
/// not have let them reach; they just get an empty bin instead of a refusal.
#[test]
fn a_hopper_placed_holding_a_locked_ingredient_can_never_be_filled() {
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    let locked = ingredients
        .iter()
        .find(|id| world.raw_items().iter().any(|raw| raw == *id))
        .expect("the gate must hold back at least one RAW solid for this test")
        .clone();
    let free = (40, 40);

    world.issue_command(PlayerCommand::Purchase {
        kind: "hopper".to_string(),
        quantity: 1,
    });
    world.issue_command(PlayerCommand::Place {
        kind: "hopper".to_string(),
        tile: free,
        direction: sim_core::sim::Direction::East,
        item: Some(locked.clone()),
    });
    assert_eq!(
        hopper_item(&world, free),
        locked,
        "a Place naming a locked raw stands the bin up — the gate is not on this command"
    );

    // And now the half that matters. Buy all you like: the gate refuses.
    world.issue_command(PlayerCommand::BuyIngredient {
        item: locked.clone(),
        quantity: 50,
    });
    world.issue_command(PlayerCommand::SetHopperAutoFill {
        tile: free,
        enabled: true,
    });
    for _ in 0..50 {
        world.tick();
    }
    let held = world
        .hopper_snapshots()
        .into_iter()
        .find(|hopper| hopper.tile == free)
        .expect("the hopper this test placed");
    assert_eq!(
        held.quantity, 0,
        "'{locked}' cannot be bought, so an auto-filling hopper of it stays empty forever"
    );

    // Once the gate opens the same bin fills from the same commands.
    sim_core::sim::reference::open_every_gate(&mut world);
    world.issue_command(PlayerCommand::BuyIngredient {
        item: locked.clone(),
        quantity: 50,
    });
    world.tick();
    let filled = world
        .hopper_snapshots()
        .into_iter()
        .find(|hopper| hopper.tile == free)
        .expect("the hopper this test placed");
    assert!(
        filled.quantity > 0,
        "with the gate open the same bin fills — the emptiness was the gate's, not the hopper's"
    );
}

/// The wire the CONTENTS picker is drawn from. A locked id is KEPT here and
/// flagged, where the market listing omits it: the picker is the full set of
/// things this building will ever hold, and a gate nobody can see coming is
/// one nobody anticipates (GAMEPLAY.md §2).
#[test]
fn the_picker_wire_keeps_a_locked_choice_and_names_its_gate() {
    let (ingredients, _) = gated();
    let world = reference_chain_with_budget(recipes(), 10_000);
    let snapshot: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&world.ingredients_snapshot()).unwrap())
            .unwrap();

    // Read against EVERY gate in the file, not just BBQ's: mustard holds
    // back two raws of its own behind a later threshold, so a choice's flag
    // has to be checked against whichever unlock names that id.
    let gates = UnlockRegistry::bundled();

    let mut seen_locked = 0;
    for list in ["hopper_items", "tank_fluids"] {
        let choices = snapshot[list].as_array().expect("a list of choices");
        assert!(!choices.is_empty(), "{list} must not be empty");
        for choice in choices {
            let id = choice["id"].as_str().unwrap().to_string();
            let locked = choice["locked"].as_bool().unwrap();
            let gate = gates.gate_for_ingredient(&id);
            assert_eq!(
                locked,
                gate.is_some(),
                "'{id}' in {list} reports locked={locked}, which disagrees with the gate"
            );
            if let Some(gate) = gate {
                seen_locked += 1;
                assert_eq!(choice["unlock_name"].as_str().unwrap(), gate.name);
                assert_eq!(
                    choice["unlock_item"].as_str().unwrap(),
                    gate.after_sold.item,
                    "the picker has to name the PRODUCT the gate counts, as an id — the \
                     panel titles it through the display-name map"
                );
                assert_eq!(
                    choice["unlock_boxes"].as_u64().unwrap() as u32,
                    gate.after_sold.boxes
                );
                assert_eq!(
                    choice["unlock_sold"].as_u64().unwrap(),
                    0,
                    "this world has sold nothing, so the progress it reports is nought"
                );
            } else {
                assert_eq!(choice["unlock_name"].as_str().unwrap(), "");
                assert_eq!(choice["unlock_item"].as_str().unwrap(), "");
                assert_eq!(choice["unlock_boxes"].as_u64().unwrap(), 0);
                assert_eq!(choice["unlock_sold"].as_u64().unwrap(), 0);
            }
            assert!(
                locked
                    || snapshot["entries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|entry| entry["id"] == id.as_str()),
                "an unlocked choice is on the market too"
            );
        }
    }
    assert!(
        seen_locked > 0,
        "the shipped gate must hold back at least one raw, or this test proves nothing"
    );
    for id in &ingredients {
        assert!(
            !snapshot["entries"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["id"] == id.as_str()),
            "'{id}' is still absent from the MARKET listing — the picker shows what \
             the market hides, not the other way round"
        );
    }
}

/// Every choice unlocks together with the gate, and nothing then claims a
/// gate to describe.
#[test]
fn the_picker_wire_goes_quiet_once_the_gate_opens() {
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    sim_core::sim::reference::open_every_gate(&mut world);
    let snapshot = world.ingredients_snapshot();
    assert!(
        snapshot
            .hopper_items
            .iter()
            .chain(snapshot.tank_fluids.iter())
            .all(|choice| !choice.locked
                && choice.unlock_name.is_empty()
                && choice.unlock_item.is_empty()
                && choice.unlock_boxes == 0
                && choice.unlock_sold == 0),
        "with every gate open no choice is locked, and an unlocked one carries no gate"
    );
}

/// The tank half of the same rule. Issue #43 built both pickers out of one
/// piece of Godot code and #90 notes that two pickers behaving differently
/// would be worse than one consistent gap, so the fluid command declines a
/// locked fluid exactly as the solid one declines a locked ingredient — and
/// declines it BEFORE the purge, so a refused declaration does not cost the
/// player the vinegar already in the drum.
#[test]
fn a_tank_cannot_be_declared_to_a_locked_fluid() {
    /// The reference chain's fluid tank (`tests/auto_fill.rs` pins the same tile).
    const TANK: (i32, i32) = (5, 2);
    let (ingredients, _) = gated();
    let mut world = reference_chain_with_budget(recipes(), 10_000);
    common::stock_up(&mut world);
    world.issue_command(PlayerCommand::FillFluidTank {
        amount: FLUID_TANK_CAPACITY,
        tile: Some(TANK),
    });
    let held = world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|tank| tank.tile == TANK)
        .expect("the reference chain ships a fluid tank");
    assert!(
        held.quantity > 0.0,
        "the fixture wants fluid standing in the drum, so a wrongly-allowed purge would show"
    );

    let locked = ingredients
        .iter()
        .find(|id| world.raw_fluids().contains(*id))
        .expect("the gate must hold back at least one RAW fluid for this test")
        .clone();
    world.issue_command(PlayerCommand::SetTankFluid {
        tile: TANK,
        fluid: locked.clone(),
    });
    let after = world
        .fluid_tank_snapshots()
        .into_iter()
        .find(|tank| tank.tile == TANK)
        .unwrap();
    assert_eq!(
        after.fluid, held.fluid,
        "'{locked}' is behind a closed gate, so the drum still holds what it held"
    );
    assert_eq!(
        after.quantity, held.quantity,
        "and a refused declaration purges nothing — the refusal comes first"
    );
}
