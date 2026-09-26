//! Headless integration test for THE COOKBOOK (`sim::cookbook`).
//!
//! What is under test is the **derivation**, not any loader — every file
//! this reads has its own unit tests. The claim being pinned is the one the
//! feature was asked for: that a page's contents fall out of `data/*.ron`
//! and are authored nowhere. So every assertion below is a fact no data
//! file states, checked against the shipped content:
//!
//! - Ketchup is tier 0 and BBQ is tier 1, because BBQ's mixer draws a fluid
//!   the ketchup vat makes.
//! - A case of ketchup costs 11 to feed, which is four tomatoes at 1, one
//!   sugar at 3 and two vinegar at 2, found by walking five recipes back.
//! - The five ketchup stages belong to Ketchup and not to BBQ, even though
//!   BBQ's walk reaches all five of them.
//! - BBQ is locked at 0 deliveries and readable at 10, although
//!   `data/unlocks.ron` never mentions a recipe.
//!
//! An empty world is enough, for `display_names.rs`'s reason — the book is
//! content read back, not state — with the single exception of the delivery
//! count, which is the only thing in the whole snapshot that moves.

use sim_core::sim::{CookbookInput, CookbookSnapshot, RecipeRegistry, World, cookbook};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("data/recipes.ron must parse")
}

fn book() -> CookbookSnapshot {
    World::new(recipes()).cookbook()
}

/// The same book at a chosen delivery count. Reaching past `World` rather
/// than driving an economy to ten deliveries: the padlock is a pure
/// function of the number, and a test that shipped a thousand boxes to
/// prove it would be testing the dock.
fn book_after(deliveries: u32) -> CookbookSnapshot {
    let machines = sim_core::sim::MachineRegistry::bundled();
    let ingredients = sim_core::sim::IngredientCatalog::bundled();
    let contracts = sim_core::sim::ContractRegistry::bundled();
    let shop = sim_core::sim::ShopCatalog::bundled();
    let staff = sim_core::sim::StaffCatalog::bundled();
    let unlocks = sim_core::sim::UnlockRegistry::bundled();
    let guide = sim_core::sim::GuideRegistry::bundled();
    cookbook::build(&CookbookInput {
        recipes: &recipes(),
        machines: &machines,
        ingredients: &ingredients,
        contracts: &contracts,
        shop: &shop,
        staff: &staff,
        unlocks: &unlocks,
        guide: &guide,
        deliveries,
    })
}

fn sauce<'a>(book: &'a CookbookSnapshot, id: &str) -> &'a sim_core::sim::cookbook::SaucePage {
    book.sauces
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no sauce page for {id}"))
}

/// A raw ingredient's page, by id. The shelf grows with every sauce, so a
/// page is never reached by its index here: `&book.ingredients[6..]` used to
/// mean "mustard's raws" and would have silently stopped checking them the
/// day a fourth sauce added one of its own.
fn raw<'a>(book: &'a CookbookSnapshot, id: &str) -> &'a sim_core::sim::cookbook::IngredientPage {
    book.ingredients
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("no ingredient page for {id}"))
}

/// Three sauces, and the book tells the two *kinds* of sauce apart without
/// being told which is which: the tier is the depth of the recipe graph, so a
/// sauce that starts at a hopper of its own is tier 0 however late it was
/// added, and only a sauce that drinks another one is deeper. Mustard is the
/// third sauce and the second tier-0 one — the proof that the difference
/// between "a second line" and "a branch" is content rather than a flag.
#[test]
fn the_shipped_content_makes_three_sauces_and_only_one_of_them_is_a_branch() {
    let book = book();
    let ids: Vec<&str> = book.sauces.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec!["boxed_ketchup", "boxed_mustard", "boxed_bbq"]);
    assert_eq!(sauce(&book, "boxed_ketchup").tier, 0);
    assert_eq!(sauce(&book, "boxed_mustard").tier, 0);
    assert_eq!(sauce(&book, "boxed_bbq").tier, 1);

    // Still one arrow, and mustard is not on either end of it: a line from
    // raw has nothing pointing into it. BBQ's does, and it names the form
    // that actually crosses — BBQ's mixer draws the vat's `ketchup`, not a
    // case of it.
    assert_eq!(book.edges.len(), 1);
    let edge = &book.edges[0];
    assert_eq!(edge.from, "boxed_ketchup");
    assert_eq!(edge.to, "boxed_bbq");
    assert_eq!(edge.via, "ketchup");
    assert!(edge.via_fluid);
}

/// The tree's shape is derived from the recipe graph and from nothing else.
/// Pinning it against a *fabricated* chain as well as the shipped one is
/// what makes that claim testable rather than a coincidence of the current
/// content: this file has never heard of ketchup.
#[test]
fn a_chain_this_test_invents_gets_its_own_tiers_with_no_rust_edit() {
    let recipes = RecipeRegistry::from_ron_str(
        r#"(
            schema_version: 3,
            recipes: [
                Recipe(id: "flour", inputs: [("wheat", 2)], outputs: [("flour", 1)],
                       machine: "crusher", time_ticks: 10),
                Recipe(id: "dough", inputs: [("flour", 1), ("sugar", 1)],
                       outputs: [("dough", 1)], machine: "mixer", time_ticks: 10),
            ],
        )"#,
    )
    .expect("fixture must parse");
    let machines = sim_core::sim::MachineRegistry::bundled();
    let ingredients = sim_core::sim::IngredientCatalog::bundled();
    let contracts = sim_core::sim::ContractRegistry::bundled();
    let shop = sim_core::sim::ShopCatalog::bundled();
    let staff = sim_core::sim::StaffCatalog::bundled();
    let unlocks = sim_core::sim::UnlockRegistry::bundled();
    let guide = sim_core::sim::GuideRegistry::bundled();
    let book = cookbook::build(&CookbookInput {
        recipes: &recipes,
        machines: &machines,
        ingredients: &ingredients,
        contracts: &contracts,
        shop: &shop,
        staff: &staff,
        unlocks: &unlocks,
        guide: &guide,
        deliveries: 0,
    });

    // No contract asks for dough, so it is rooted at the end of its own
    // line instead — content is never invisible for want of a buyer.
    assert_eq!(
        book.sauces
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        vec!["dough"]
    );
    let dough = sauce(&book, "dough");
    assert_eq!(dough.tier, 0);
    assert_eq!(dough.steps.len(), 2);
    assert!(dough.contracts.is_empty());
    assert_eq!(dough.pay_low, None, "nothing buys it, and that is not a 0");
}

#[test]
fn a_case_costs_what_walking_the_chain_back_to_the_market_says_it_costs() {
    let book = book();
    // 4 tomato @1 + 1 sugar @3 + 2 vinegar @2.
    assert_eq!(sauce(&book, "boxed_ketchup").cost_per_unit, 11.0);
    // …plus 1 spice_mix @5 + 1 molasses @4 + 1 liquid_smoke @3, on top of
    // one whole ketchup. Every case of BBQ contains a case of ketchup's
    // worth of ingredients, which is the point the page is making.
    assert_eq!(sauce(&book, "boxed_bbq").cost_per_unit, 23.0);
    // 4 mustard_seed @2 + 1 turmeric @5 + 2 vinegar @2 — dearer than a case
    // of ketchup and cheaper than a case of BBQ, which is the whole of why
    // the mustard tier pays between them.
    assert_eq!(sauce(&book, "boxed_mustard").cost_per_unit, 17.0);
}

#[test]
fn the_shopping_list_counts_raws_per_case_through_every_stage_between() {
    let book = book();
    let ketchup = sauce(&book, "boxed_ketchup");
    let lines: Vec<(&str, f64)> = ketchup
        .inputs
        .iter()
        .map(|line| (line.id.as_str(), line.quantity))
        .collect();
    assert_eq!(
        lines,
        vec![("sugar", 1.0), ("tomato", 4.0), ("vinegar", 2.0)]
    );
    assert!(ketchup.inputs.iter().all(|line| line.from_sauce.is_none()));
    assert!(
        ketchup.inputs.iter().any(|line| line.fluid),
        "vinegar arrives by pipe and the page has to say so"
    );

    // BBQ's ketchup is a link across to the other page, not a shopping
    // line — the doc card's "tapped".
    let bbq = sauce(&book, "boxed_bbq");
    let tapped = bbq
        .inputs
        .iter()
        .find(|line| line.id == "ketchup")
        .expect("BBQ is fed by ketchup");
    assert_eq!(tapped.from_sauce.as_deref(), Some("boxed_ketchup"));
    assert!(!tapped.locked, "a sauce you make is never behind a gate");
}

/// The five ketchup stages are reachable from BBQ's walk too. They belong
/// to the shallower sauce, so the book has one page per stage rather than
/// printing the whole ketchup line twice.
#[test]
fn a_stage_two_sauces_share_belongs_to_the_shallower_one() {
    let book = book();
    assert_eq!(
        sauce(&book, "boxed_ketchup")
            .steps
            .iter()
            .map(|s| s.recipe.as_str())
            .collect::<Vec<_>>(),
        vec![
            "tomato_paste",
            "ketchup_mix",
            "ketchup",
            "bottled_ketchup",
            "boxed_ketchup"
        ]
    );
    assert_eq!(
        sauce(&book, "boxed_bbq")
            .steps
            .iter()
            .map(|s| s.recipe.as_str())
            .collect::<Vec<_>>(),
        vec!["bbq_mix", "bbq", "bottled_bbq", "boxed_bbq"]
    );
}

/// The `NEW` badge, worked out rather than authored: ketchup introduces
/// every machine on it, and BBQ introduces none, because it is the same
/// kit rearranged.
#[test]
fn a_machine_is_new_only_on_the_shallowest_sauce_that_uses_it() {
    let book = book();
    assert!(
        sauce(&book, "boxed_ketchup")
            .steps
            .iter()
            .all(|s| s.new_machine)
    );
    assert!(
        sauce(&book, "boxed_bbq")
            .steps
            .iter()
            .all(|s| !s.new_machine)
    );
}

/// `data/unlocks.ron` gates ingredient ids and contract ids and never once
/// names a recipe — its header explains why at length. "BBQ is locked" is
/// therefore something this module works out, and it has to stop being
/// true at exactly the delivery the gate names.
#[test]
fn lockedness_is_derived_from_the_ingredient_gates_rather_than_declared() {
    let shut = book_after(0);
    let bbq = sauce(&shut, "boxed_bbq");
    assert!(bbq.locked);
    assert_eq!(bbq.gate.as_ref().map(|g| g.after_deliveries), Some(10));
    assert_eq!(
        bbq.gate.as_ref().map(|g| g.name.as_str()),
        Some("BBQ Sauce")
    );
    assert_eq!(bbq.gate.as_ref().map(|g| g.deliveries), Some(0));
    assert!(!sauce(&shut, "boxed_ketchup").locked);

    let nearly = book_after(9);
    assert!(sauce(&nearly, "boxed_bbq").locked, "9 of 10 is still shut");

    let open = book_after(10);
    assert!(!sauce(&open, "boxed_bbq").locked);
    assert!(
        open.sauces
            .iter()
            .all(|s| s.tier == sauce(&shut, &s.id).tier),
        "opening a gate reveals a page; it does not move the tree"
    );
}

/// Every machine kind gets a page, including the five nothing cooks on. A
/// player wanting to know what a hopper is has the same question as one
/// wanting to know what a mixer is.
#[test]
fn every_machine_kind_has_a_page_and_the_line_reads_in_chain_order() {
    let book = book();
    let ids: Vec<&str> = book.machines.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "crusher",
            "mixer",
            "cooking_vat",
            "bottler",
            "packer",
            // Nothing runs a recipe on these, so they sort after the ones
            // that do, in id order.
            "dock",
            "fluid_tank",
            "hopper",
            "splitter",
            "warehouse_rack",
        ]
    );

    let vat = book
        .machines
        .iter()
        .find(|m| m.id == "cooking_vat")
        .expect("the vat has a page");
    assert_eq!((vat.width_tiles, vat.height_tiles), (2, 2));
    assert_eq!(vat.price, Some(200.0));
    assert_eq!(vat.makes, vec!["ketchup", "bbq", "mustard"]);
    assert_eq!(vat.crew.len(), 1);
    assert_eq!(vat.crew[0].category, "manufacturing");
    assert_eq!(
        vat.crew[0].name, "Manufacturing",
        "content names a category"
    );
    assert_eq!(vat.crew[0].count, 2);

    let hopper = book
        .machines
        .iter()
        .find(|m| m.id == "hopper")
        .expect("the hopper has a page");
    assert!(
        hopper.makes.is_empty(),
        "it moves product without changing it"
    );
    assert!(hopper.crew.is_empty());
}

#[test]
fn every_bought_ingredient_has_a_page_saying_what_it_is_for() {
    let book = book();
    let ids: Vec<&str> = book.ingredients.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "tomato",
            "sugar",
            "vinegar",
            "spice_mix",
            "molasses",
            "liquid_smoke",
            "mustard_seed",
            "turmeric"
        ],
        "data/ingredients.ron's own order — the market lists them in it too"
    );
    for page in &book.ingredients {
        assert!(page.blurb.is_some(), "{} has no blurb", page.id);
        assert!(page.icon.is_some(), "{} has no icon", page.id);
        assert!(!page.used_in.is_empty(), "{} is used by nothing", page.id);
        assert!(page.price > 0.0);
    }

    // THE ONE INGREDIENT TWO SAUCES BUY. Every other raw on the shelf feeds
    // one product; vinegar goes into the ketchup mixer and the mustard
    // crusher — mustard mills its seed WET, which is how the two sauces stay
    // off one another's machine kind (`docs/DECISIONS.md`, 2026-09-26) — so
    // its page names two sauces and a player reading it learns that the two
    // lines compete in the shop. Derived from `data/recipes.ron`, so this is
    // also what fails if mustard ever stops drinking it.
    let vinegar = raw(&book, "vinegar");
    assert!(vinegar.fluid, "vinegar is a tank, not a hopper");
    assert_eq!(vinegar.used_in, vec!["boxed_ketchup", "boxed_mustard"]);
    assert!(!vinegar.locked);

    let smoke = raw(&book, "liquid_smoke");
    assert!(smoke.fluid);
    assert!(smoke.locked);
    assert_eq!(smoke.gate.as_ref().map(|g| g.after_deliveries), Some(10));

    // Mustard's raws are solids, they belong to one sauce, and they are held
    // back further than BBQ's — the gate is in `data/unlocks.ron` and the
    // page only reports it. WHICH raws those are is derived from `used_in`,
    // not from a slice of the shelf: a raw that feeds nothing but mustard is
    // mustard's, whether it is the two shipped today or a third tomorrow.
    let mustard_raws: Vec<&sim_core::sim::cookbook::IngredientPage> = book
        .ingredients
        .iter()
        .filter(|page| page.used_in == ["boxed_mustard"])
        .collect();
    assert_eq!(
        mustard_raws
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        vec!["mustard_seed", "turmeric"],
        "the raws the recipe graph says feed mustard and nothing else"
    );
    for page in mustard_raws {
        assert!(!page.fluid, "{} rides a belt out of a hopper", page.id);
        assert!(
            page.locked,
            "'{}' feeds nothing but mustard, so a new game must not be able to buy it — \
             the mustard_sauce gate in data/unlocks.ron has to name it",
            page.id
        );
        assert_eq!(
            page.gate.as_ref().map(|g| g.after_deliveries),
            Some(25),
            "'{}' is held back by a gate other than mustard_sauce's 25 deliveries",
            page.id
        );
    }
}

/// The prose the pages are written in comes out of the content files, and
/// the fields are optional so a stage in the middle of a chain does not
/// have to invent a tagline for itself.
#[test]
fn a_page_carries_the_prose_its_recipe_authored_and_nothing_it_did_not() {
    let book = book();
    let ketchup = sauce(&book, "boxed_ketchup");
    assert_eq!(
        ketchup.tagline.as_deref(),
        Some("What the dock ships, and what the contracts pay for.")
    );
    assert!(ketchup.blurb.is_some());
    assert_eq!(
        ketchup.icon.as_deref(),
        Some("item_bottle"),
        "the book pictures a sauce by its bottle, not by the case it ships in"
    );
    assert_eq!(
        sauce(&book, "boxed_bbq").tagline,
        None,
        "a sauce with no tagline drops the line rather than filling it"
    );
}

/// Contract pay is a range across the board, and `None` — not zero — when
/// nothing wants the thing. The two overlap on the shipped content because
/// the mixed orders ask for both sauces at once, which is exactly what a
/// player reading both pages should be able to see.
#[test]
fn a_sauce_page_says_what_the_board_pays_for_a_case_of_it() {
    let book = book();
    let ketchup = sauce(&book, "boxed_ketchup");
    assert_eq!(ketchup.pay_low, Some(24.0));
    assert_eq!(ketchup.pay_high, Some(44.0));
    assert!(ketchup.contracts.contains(&"Grill Pub Chain".to_string()));

    let bbq = sauce(&book, "boxed_bbq");
    assert_eq!(bbq.pay_low, Some(42.0));
    assert_eq!(bbq.pay_high, Some(65.0));
    assert!(
        bbq.pay_low.unwrap() > bbq.cost_per_unit,
        "every sauce has to be worth making"
    );

    // Mustard's band, read off `data/contracts.ron` the same way: the low is
    // `deli_chain`'s blended 204 over five cases, the high is
    // `sandwich_franchise`'s 416 over eight.
    let mustard = sauce(&book, "boxed_mustard");
    // 204 over five cases is 40.8, which fixed point (ARCHITECTURE.md §3.2)
    // cannot hold exactly, so the page is read to the penny rather than
    // compared for equality. The other three rungs are whole numbers.
    let low = mustard
        .pay_low
        .expect("the board pays for a case of mustard");
    assert!((low - 40.8).abs() < 0.001, "pay_low read {low}");
    assert_eq!(mustard.pay_high, Some(52.0));
    assert!(
        low > mustard.cost_per_unit,
        "every sauce has to be worth making"
    );
    assert!(
        mustard.contracts.contains(&"Deli Chain".to_string()),
        "the mixed order belongs to both of its sauces' pages"
    );
}

/// The BASICS pages are the one part of the book that is authored prose, and
/// the whole reason they are content is that their numbers are not: every
/// quantity is a `{placeholder}` the sim answers. These two tests are that
/// claim from both ends — the prose asks, and the answers are the registries'
/// own numbers rather than anything typed twice.
#[test]
fn the_guide_ships_pages_and_every_one_of_them_asks_the_sim_for_its_numbers() {
    let book = book();
    assert!(
        book.guide.len() >= 4,
        "the guide is how the game explains itself; {} pages is not that",
        book.guide.len()
    );
    // Placeholders survive the wire: substitution is the panel's, because
    // formatting ticks as seconds is.
    let asked: usize = book
        .guide
        .iter()
        .map(|topic| {
            std::iter::once(&topic.title)
                .chain(std::iter::once(&topic.tagline))
                .chain(topic.body.iter())
                .map(|text| text.matches('{').count())
                .sum::<usize>()
        })
        .sum();
    assert!(
        asked >= 10,
        "only {asked} placeholders across the whole guide — prose that states \
         its own numbers is prose that goes stale"
    );
    for topic in &book.guide {
        assert!(!topic.title.trim().is_empty());
        assert!(!topic.body.is_empty());
    }
}

#[test]
fn every_fact_the_guide_can_ask_for_is_answered_from_the_content_files() {
    let book = book();
    let fact = |id: &str| {
        book.facts
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("no fact answers {{{id}}}"))
    };

    // The vocabulary goes out whole, so a page can never find a gap.
    assert_eq!(book.facts.len(), sim_core::sim::GuideFact::ALL.len());

    // Each of these is the registry's own number, read back a second way.
    // A rebalance therefore moves the prose, which is the point.
    assert_eq!(fact("starting_money").number, 3000.0);
    assert_eq!(fact("starting_money").kind, "money");
    // One dock's places, not the retired shared pool's per-item cap of 48.
    assert_eq!(
        fact("dock_capacity").number,
        f64::from(sim_core::sim::DOCK_CAPACITY)
    );
    assert_eq!(fact("sauce_count").number, book.sauces.len() as f64);
    assert_eq!(
        fact("raw_count").number,
        book.ingredients.len() as f64,
        "the raws the guide counts are the raws the book has pages for"
    );
    // "On the market" is not "in the file". Three of the six raws sit behind
    // the BBQ gate in a new game, so a sentence that called `raw_count` the
    // market was wrong by three — that is what `open_raw_count` is for.
    assert_eq!(
        fact("open_raw_count").number,
        book.ingredients.iter().filter(|page| !page.locked).count() as f64,
        "the open raws are the book's own ingredient pages that are not locked"
    );
    assert!(
        fact("open_raw_count").number < fact("raw_count").number,
        "a new game with every raw already on sale has no gate left to open, and the guide's prose about waiting behind gates would be a lie"
    );

    assert_eq!(
        fact("buyable_count").number,
        sim_core::sim::ShopCatalog::bundled().entries().len() as f64
    );

    let contracts = sim_core::sim::ContractRegistry::bundled();
    assert_eq!(fact("contract_count").number, contracts.len() as f64);
    assert_eq!(
        fact("starter_contracts").number,
        contracts.starters().count() as f64
    );
    assert!(
        fact("starter_contracts").number < fact("contract_count").number,
        "a board where every order is a starter is a board with no progression"
    );

    let staff = sim_core::sim::StaffCatalog::bundled();
    let crew: u32 = staff
        .categories()
        .iter()
        .map(|c| c.starting_headcount)
        .sum();
    assert_eq!(fact("starting_crew").number, f64::from(crew));
    assert_eq!(
        fact("payday_ticks").number,
        staff.pay_period_ticks() as f64,
        "and it goes out as TICKS — the panel divides by TICK_HZ, because \
         the tick rate is the presentation layer's"
    );
    assert_eq!(fact("payday_ticks").kind, "ticks");

    // The earliest gate in `data/unlocks.ron`, by the number it wants.
    let unlocks = sim_core::sim::UnlockRegistry::bundled();
    let first = unlocks
        .ordered()
        .iter()
        .min_by_key(|u| u.after_deliveries)
        .expect("the shipped game has a gate");
    assert_eq!(fact("first_gate_name").text, first.name);
    assert_eq!(fact("first_gate_name").kind, "text");
    assert_eq!(
        fact("first_gate_deliveries").number,
        f64::from(first.after_deliveries)
    );

    // The orders the board is showing a brand-new factory, which is neither
    // the file's count nor the starters: a starter is one the game signs for
    // you, and there is one of those, while nine are simply ungated.
    let open_now = contracts
        .ordered()
        .iter()
        .filter(|c| unlocks.contract_available(&c.id, 0))
        .count();
    assert_eq!(fact("open_contract_count").number, open_now as f64);
    assert!(
        fact("open_contract_count").number > fact("starter_contracts").number,
        "if the only orders on a new board were the ones already signed, there would be nothing for a new player to sign"
    );
    assert!(
        fact("open_contract_count").number < fact("contract_count").number,
        "a board that shows every order at zero deliveries has no gated orders, and the guide says it has"
    );

    // The cheapest thing on the shelf goes out as an ID, titled by Godot
    // like every other id on this wire.
    let shop = sim_core::sim::ShopCatalog::bundled();
    let cheapest = shop
        .entries()
        .iter()
        .min_by_key(|e| e.price_money())
        .expect("the shop sells something");
    assert_eq!(fact("cheapest_buyable").text, cheapest.id);
    assert_eq!(fact("cheapest_buyable").kind, "kind_id");
    assert_eq!(
        fact("cheapest_buyable_price").number,
        cheapest.price_money().to_num::<f64>()
    );
}

/// The two counts that mean "right now" have to move with the gates, and the
/// two that mean "in the file" have to stay put. This is the finding that
/// sent the first version of the BASICS prose back: it said six raws were on
/// the market when three were behind a gate, and the only reason nothing
/// caught it is that no fact knew the difference.
#[test]
fn the_open_counts_follow_the_gates_and_the_file_counts_do_not() {
    let fact = |book: &CookbookSnapshot, id: &str| {
        book.facts
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("no fact answers {{{id}}}"))
            .number
    };

    let new_game = book_after(0);
    // Past every gate the shipped file holds, without driving a dock to get
    // there, for `book_after`'s own reason.
    let veteran = book_after(10_000);

    assert_eq!(
        fact(&new_game, "raw_count"),
        fact(&veteran, "raw_count"),
        "the file's raws do not change because somebody shipped a box"
    );
    assert_eq!(
        fact(&new_game, "contract_count"),
        fact(&veteran, "contract_count"),
    );
    assert_eq!(
        fact(&new_game, "starter_contracts"),
        fact(&veteran, "starter_contracts"),
        "a starter is a property of the order, not of your progress"
    );

    assert!(
        fact(&veteran, "open_raw_count") > fact(&new_game, "open_raw_count"),
        "a gate that opens puts a raw on the market, so the guide's count of \
         what the market sells has to rise with it"
    );
    assert_eq!(
        fact(&veteran, "open_raw_count"),
        fact(&veteran, "raw_count"),
        "past every gate, everything in the file is for sale"
    );
    assert!(fact(&veteran, "open_contract_count") > fact(&new_game, "open_contract_count"));
    assert_eq!(
        fact(&veteran, "open_contract_count"),
        fact(&veteran, "contract_count"),
        "past every gate, the board shows the whole file"
    );
}
