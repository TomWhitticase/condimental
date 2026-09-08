# Sauce Factory — Gameplay, Progression & Economy Plan

This doc is about **what the game actually is to play** — the core loop, the
sauce/equipment progression, and the money economy that ties them together.
It deliberately does not cover engine/tech choices (that's
[docs/ARCHITECTURE.md](ARCHITECTURE.md)) or visual style (that's
[docs/UI_UX.md](UI_UX.md)). Read this one for "why am I playing this and what
happens over the course of a playthrough."

---

## 0. At a glance

| Aspect | Direction |
|---|---|
| Core loop | Build production lines → fulfill sauce contracts → earn money → reinvest in equipment/ingredients/staff → unlock new sauce categories |
| Sauce progression | Simple, few-ingredient sauces (tomato sauce) → composite condiments (ketchup, BBQ) → equipment-gated categories (mayo needs refrigeration, hot sauce needs acid handling) → elaborate long-process sauces (aged/fermented) |
| Derived sauces | Higher-tier sauces can require other *finished sauces* as an ingredient (chilli mayo needs mayo + hot sauce; honey mustard needs mustard), turning the sauce list into a branching dependency tree, not a flat list |
| Variants | Many sauces come in flavour variants (garlic mayo, chilli mayo, honey mayo; smoky BBQ, spicy BBQ) built by taking a plain base sauce and running it through a small flavour-add step — cheap content, real contract variety |
| Demand model | **Contracts**, not a spot market — standing orders for a *mix* of sauces, delivered on a recurring cadence, that must be kept up or lost |
| Money | Earned from contract deliveries; spent on ingredients (recurring), equipment (capital), and staff wages (recurring) |
| Staff | Not simulated NPCs — an abstract resource pool per category (manufacturing, maintenance, packaging, logistics). Running equipment consumes staff capacity the way it would consume power; understaffing stalls production, not a person failing to show up |
| Design pressure | Contracts always demand a *variety* of sauces, so the winning strategy is a diversified factory, not maxing out whichever single sauce has the best margin |
| Recipe complexity | Scales with tier: early recipes are 2–3 ingredients and one process step; late recipes are many ingredients, multiple equipment stages, and long cure/ferment times |

---

## 1. Core loop

1. **Produce** sauce from raw ingredients through machines (this is the Factorio-like part — belts, machines, recipes, per [ARCHITECTURE.md](ARCHITECTURE.md)).
2. **Deliver** against active **contracts** — standing agreements to supply a mix of sauces at a regular interval.
3. **Earn** money per successful delivery; **lose** the contract (and/or take a reputation hit) if a delivery interval is missed.
4. **Spend** that money three ways: ingredients (ongoing input cost), equipment (one-time capital unlock), staff (ongoing wage upkeep).
5. New equipment unlocks new sauce **categories**, which unlock access to new, higher-value contracts that demand those categories — closing the loop.

The whole progression is gated through this loop rather than through an abstract XP/skill-point system: **money buys equipment, equipment unlocks sauces, sauces satisfy contracts, contracts pay money.** There is no separate currency or tech-point resource layered on top of it.

---

## 2. Sauce progression tiers

Concrete example tiers, in the order the user's original pitch laid them out. Exact ingredients/quantities are game-design balancing, not locked by this doc — what *is* locked in is the shape of the progression: each tier adds either **more ingredients/steps** (recipe complexity) or **a new equipment dependency that gates an entire category** (not just a harder recipe).

| Tier | Sauce | New ingredients | New equipment/process required | Why it's gated here |
|---|---|---|---|---|
| 1 | Tomato sauce | Tomatoes, salt | Basic cooker/kettle | Baseline: one process step, no special handling |
| 2 | Ketchup | + vinegar, sugar, spices | Reduction/thickening step, bottling & sealing | First multi-step recipe: cook down, blend, bottle |
| 3 | BBQ sauce | + molasses/brown sugar, smoke flavoring | Smoker or low-and-slow reduction unit | Longer `TimedProcess` (see ARCHITECTURE.md §3.5), first "slow cooking" recipe |
| 4 | Mayonnaise / aioli | Eggs, oil, acid | **Refrigeration** (cold storage + cold-chain handling) + emulsifier machine | Perishable-ingredient handling is a new *axis*, not just a harder recipe — eggs/oil spoil without cold storage upstream of the machine, not just in a warehouse |
| 5 | Hot sauce | Chilies, vinegar (high concentration) | **Acid-handling equipment** (acid-resistant tanks/piping) + fermentation vessel for mash-based variants | High-acid ingredients corrode standard equipment; this tier is gated by a *material/durability* requirement, not just a recipe unlock |
| 6 | Elaborate/aged sauces (fermented hot sauce, barrel-aged BBQ, whole-grain mustard, etc.) | Combines multiple tier 2–5 ingredient classes | Multiple equipment lines feeding one recipe, plus multi-stage `TimedProcess` chains (mash → ferment → strain → blend → age → bottle) lasting far longer than any earlier tier | The payoff tier: production chains that span the whole factory, not one station |

Two structurally different kinds of gate recur through this table, and both should stay legible to the player as distinct reasons a sauce is locked:

- **Recipe-complexity gates** (tiers 1→3): more ingredients, more process steps, longer single `TimedProcess` — no new equipment *category*, just a bigger version of what you already have.
- **Equipment-category gates** (tiers 4, 5): an entire class of sauce is inaccessible until a specific piece of capital equipment exists, because the ingredient itself can't be handled safely/at all without it (perishables need cold chain, high-acid ingredients corrode non-resistant equipment).

Tier 6 combines both: by then the player has every equipment category, and the remaining challenge is orchestrating long multi-stage chains across them.

---

## 3. Derived sauces & variants

Two more axes of depth sit on top of the tier table, and both are deliberately *cheap content* — small recipe additions rather than whole new production lines — because that's what lets the sauce list grow wide without every addition demanding a new machine.

**Derived sauces (sauce-as-ingredient):** some recipes consume another sauce as an input rather than only raw ingredients — chilli mayo requires a batch of mayo plus a batch of hot sauce; honey mustard requires mustard plus honey. This needs no new architecture: a sauce is just another item, and Factorio-style intermediate products (an item that is simultaneously a sellable output *and* an input to another recipe) are already exactly how `recipes.ron` works. The design payoff is that the sauce list becomes a branching dependency tree instead of a flat list of unrelated recipes — unlocking hot sauce (tier 5) doesn't just unlock hot sauce, it unlocks every recipe downstream of it too (chilli mayo, chilli BBQ, etc.), so later equipment purchases pay off across multiple sauces at once.

**Variants (flavour families):** many sauces should exist as a family — plain mayo, garlic mayo, chilli mayo, honey mayo; plain BBQ, smoky BBQ, spicy BBQ — built as a **base sauce** (the tier-gated recipe from §2) plus a small **flavour-finishing step** that consumes the base plus one or two flavour ingredients to produce a distinct, separately-sellable item. Variants are not a new tier or a new equipment unlock; they're a cheap multiplier on existing tiers, specifically so the *contract* system (§6) has a much larger pool of distinct sauces to demand mixes of without the player needing new machinery for every one. Note the flavour-finishing step overlaps with derived sauces above when the flavour ingredient is itself a sauce (chilli mayo is simultaneously "mayo + hot sauce" as a derived sauce *and* readable as "a chilli variant of mayo") — that's intentional; they're the same underlying mechanic (a recipe that consumes a finished sauce) viewed from two design angles.

**Contract implication:** once variants exist, contracts should be able to name a specific variant (e.g. "50 garlic mayo + 50 chilli mayo," not just "100 mayo") — this is what actually makes variants matter for the diversification design pressure in §6, rather than being purely cosmetic flavour text.

---

## 4. Equipment & tech tree

Equipment purchases *are* the tech tree — there's no separate abstract skill-point currency. Each major piece of equipment is a capital purchase (one-time cost, paid from contract earnings) that unlocks a sauce category outright, the same way Factorio's research unlocks recipes but paid in cash instead of science packs.

| Equipment tier | Unlocks | Gameplay effect |
|---|---|---|
| Basic cooker/kettle | Tomato sauce | Starting equipment |
| Reduction/thickening + bottling line | Ketchup | First composite recipe, first bottling step |
| Smoker/slow-reduction unit | BBQ sauce | Long single-stage `TimedProcess` |
| **Refrigeration** (cold storage + cold-chain conveyance) | Mayo/aioli and other cold sauces | Introduces spoilage as a mechanic for any ingredient/product tagged perishable — cold sauces need refrigerated storage *and* refrigerated transport between stations, not just a fridge at the end |
| **Acid-handling equipment** (resistant tanks/piping) + fermentation vessel | Hot sauce and other high-acid/fermented sauces | Introduces corrosion/durability as a mechanic for high-acid throughput on non-rated equipment — using a standard pipe for a hot-sauce line should visibly degrade it, pushing the player toward the upgrade rather than silently blocking the recipe |
| Multi-line integration (no single new machine — this is a factory-layout milestone, not a purchase) | Elaborate/aged sauces | Requires successfully routing multiple prior equipment categories' outputs into one long chain |

This mirrors the game's underlying architecture cleanly: every sauce, machine, and (per §6 below) contract is data-driven (`recipes.ron`, `machines.ron`, and a new `contracts.ron` — see §9), so adding a new tier is a content change, never a simulation-code change.

---

## 5. Economy

**Income:** contract deliveries (§6) are the primary and intended income source. Whether a secondary "dump excess product at a lower flat rate" spot-sell mechanic exists is an open question (§11) — the design intent is that contracts, not a spot market, should be where most money comes from, specifically so that chasing a single high-margin sauce isn't a viable strategy (see §6's rationale).

**Costs**, three categories with different pacing:

- **Ingredients** — recurring, scales directly with production volume. The ongoing cost of actually running the factory.
- **Equipment** — one-time capital cost, the "tech tree" purchase from §4. Large up front, zero ongoing cost (aside from any maintenance implied by durability/corrosion in §4).
- **Staff wages** — recurring upkeep, paid per hired head across the four categories in §7, and scaling with how much equipment is actually running rather than a flat factory-size tax. This is what keeps a large idle factory from being free to run, and is a deliberate separate cost axis from ingredients so the two can't be optimized against each other identically.

The interplay of these three is the actual resource-management game: contract income has to outpace ingredients + wages on an ongoing basis *and* leave enough surplus to save up for the next equipment tier.

---

## 6. Contracts — the demand system

This is the central economic mechanic and the main departure from a plain Factorio-style "produce whatever's most profitable" loop.

**Shape of a contract:** a standing agreement specifying a *mix* of sauces and quantities (e.g. "100 bottles ketchup + 50 bottles mayo"), a delivery cadence (deliver toward that quota at regular intervals, not all at once at a deadline), and a payout per successful delivery. Missing a delivery interval risks losing the contract (and/or a reputation penalty — see open questions). Once variants exist (§3), a contract's mix can name specific variants, not just a sauce category.

**Why contracts instead of a spot market:** if the player could just sell any amount of any sauce at a posted price, the dominant strategy is always "identify the single most profitable sauce, build nothing else." Requiring a *mix* per contract, and running several contracts concurrently with different mixes, makes diversified production the actually-correct strategy instead of an aesthetic choice — the player is rewarded for having built out multiple sauce lines because that's literally what's needed to keep multiple contracts alive at once, not because variety is fun in the abstract.

**Progression coupling:** early contracts should only demand tier-1/2 sauces (accessible from the start), and higher-value contracts should demand mixes that reach into tiers the player hasn't unlocked yet — creating the pull to invest contract earnings into the next equipment tier specifically to become eligible for the next tier of contract, rather than equipment unlocks being disconnected from what the contract board is asking for.

---

## 7. Staff — a resource pool, not NPCs

Staff are **not** simulated as entities walking around the factory interacting with machines — that's deliberately out of scope, both because it adds pathfinding/animation complexity the sim core doesn't need and because it doesn't serve the actual design goal, which is staff as another *capacity constraint* alongside power/ingredients, not a character system.

**Model:** staff are an abstract resource pool, tracked per category, with a hired **headcount** (capacity) and a **recurring wage per head**. Every machine or process declares how many staff of which category it needs *while actively running* — the same shape as Factorio-style power draw: a machine that can't get enough of its required resource stalls, it doesn't run degraded or explode. If total demand for a category across all currently-active machines exceeds hired headcount, some subset of that category's machines simply cannot run that tick (which subset — player-set priority, first-come, proportional throttling — is an open question, §11). This reuses the same conceptual pattern `PowerNetwork` already uses in ARCHITECTURE.md §3.2/§3.8: aggregate demand vs. capacity, no per-entity simulation.

**Categories (four, deliberately not more):** chosen to map directly onto the four pipeline stages a sauce actually passes through, so the split stays legible rather than feeling like an arbitrary RPG-style stat split:

| Category | Required by | What it represents |
|---|---|---|
| **Manufacturing** | Cookers, reducers, emulsifiers, fermenters — anything actively turning ingredients into sauce | Running the production machinery itself |
| **Maintenance** | Repairing degraded/broken equipment | Directly tied to the corrosion/wear mechanic from acid-handling equipment (§4) and any other durability-based gate added later — more high-wear equipment means more maintenance demand, not just more manufacturing demand |
| **Packaging** | Bottling/sealing stations | Turning finished sauce in a tank/buffer into a sellable bottled unit |
| **Logistics** | Pallet-wrapping and truck-loading | The step that actually fulfills a contract delivery (§6) — no logistics capacity, no deliveries going out even if product is sitting bottled and ready |

Because each category gates a different pipeline stage, understaffing reads clearly to the player: a manufacturing shortfall means machines idle, a packaging shortfall means product backs up in tanks, a logistics shortfall means bottled product piles up but contracts still go unfulfilled. This also means growing the factory has four independent staffing costs to balance, not one lump number — mirroring how equipment/ingredient costs already split by category (§5).

**Hiring:** treated as increasing a category's headcount for an ongoing per-head wage (§5) — a capacity you pay to maintain, not a one-time purchase like equipment. Whether there's also a one-time recruiting/training cost on top of the recurring wage, and whether headcount can be reduced (firing) without penalty, are open questions (§11).

---

## 8. Example progression walkthrough

A rough shape of an early playthrough, to sanity-check that the systems above actually cohere into a game:

1. Start with a basic cooker and one small tomato-sauce contract. Produce and deliver on cadence — even at this scale, running the cooker needs manufacturing staff and getting bottles onto the delivery truck needs logistics staff.
2. Save contract earnings, buy the ketchup line (reduction/bottling) and hire enough packaging staff to run it. A new contract type appears wanting ketchup, or an existing contract's mix expands to include it.
3. Juggling two-plus concurrent contracts with different sauce mixes becomes the actual gameplay tension — production has to be laid out for variety, not a single optimized line. Add a couple of flavour variants (§3) cheaply to widen what contracts can be taken without new machinery.
4. Save toward refrigeration. Once bought, mayo-inclusive contracts become available/eligible; cold-chain routing becomes a new factory-layout concern (spoilage risk on any perishable leg of the line).
5. Save toward acid-handling equipment; hot-sauce contracts open up; standard piping used for a hot-sauce line starts showing corrosion, driving up maintenance-staff demand and nudging the player toward the acid-resistant upgrade instead of just hiring more repair staff to keep patching it.
6. Derived sauces open up (chilli mayo from mayo + hot sauce) once both parent categories exist, rewarding the equipment investment across multiple downstream recipes at once.
7. By the elaborate/aged tier, the player is running long multi-stage chains (mash → ferment → age → bottle) alongside everything from earlier tiers simultaneously, with all four staff categories under real pressure, because contracts by this point demand a wide spread across nearly every category unlocked so far.

---

## 9. Data & systems this implies for the architecture

Everything in this doc should be **content**, not simulation code, per [ARCHITECTURE.md](ARCHITECTURE.md)'s data-driven-recipes rule:

- Sauce recipes and their tier/equipment requirements live in `recipes.ron`/`machines.ron` as already planned. Derived sauces and variants (§3) need no new mechanism — a recipe listing another sauce item as an input is already the standard Factorio-style intermediate-product pattern `recipes.ron` is built for.
- ~~**Contracts need their own data file**~~ — **done (2026-09-08)**: `data/contracts.ron` (schema_version 1) holds a list of contracts, each with an id, a player-facing name, a sauce-mix (item id -> quantity pairs, multi-item supported), a cadence in ticks and a payout in whole money units; `sim_core::sim::contracts::ContractRegistry` loads and validates it, rejecting a mismatched `schema_version` and any degenerate contract (no contracts at all, duplicate id, empty mix, a zero-quantity mix line, zero cadence, zero payout) with an error naming what is wrong — the same `schema_version` treatment `recipes.ron`/`machines.ron` already get (ARCHITECTURE.md §4). **Penalty terms are deliberately not in the file yet**: §11 below still hasn't decided what a missed window costs, and a field nothing reads would be inventing that answer in data. It lands, with a `schema_version` bump, once that question is decided.
- **The chain has a real delivery point (added 2026-09-08 — this closes the "a real delivery-dock tile is future polish" note `economy.rs` and the entry below both left open):** finished product is no longer lifted out of the last machine's buffer by a player command. `data/machines.ron` gained a `dock` kind (priced in `shop.ron` like every other buildable kind), `World` lays one more ordinary belt past the chain's last machine and stands the dock at its end, and `sim_core::sim::dock::Dock` takes arriving product off that belt and calls `Economy::deposit` **every tick, automatically**. `PlayerCommand::CollectOutput` was **removed**, not kept alongside — two ways to sell the same box, one of which teleports, is exactly the contradiction the hopper removed at the feed end. Two calls worth recording. (1) **The dock is a pass-through, not a store:** it ships everything it receives in the same tick, so its own buffer reads empty between ticks and the visible "what's on the dock" number stays exactly one number in one place — `EconomySnapshot::delivery_stock`, the shared pool every contract settles against. A second staging buffer in front of that pool would show the player boxes standing on the dock that no contract can yet count, which is a gauge reading one thing while the mechanism does another. It gets a real hold the day something gates the release (a truck on a cadence, or §7's logistics-staff capacity) — that is a decision, where a bare delay is only latency. (2) **It still has a bounded buffer (32 per item) even though the drain makes that unreachable today**, because the intake has to be able to refuse: that is what makes a full dock back the belt up rather than annihilate the box, and it is the property the chain's mass-conservation test actually checks.
- **The delivery-allocation rule (decided 2026-09-08, this doc had left it open):** several contracts now run concurrently, and they all draw from **one shared pool** of collected-but-undelivered stock. At its own cadence boundary a contract is satisfied only if the pool holds its full mix, and on payout it **consumes** that mix from the pool — goods are never credited to two contracts at once. The alternative (credit one delivered unit to every contract that wants that item) duplicates goods, so stacking several contracts onto a single product line would double income at zero production cost — exactly the "find the best sauce and build nothing else" strategy §6 rejects a spot market in order to prevent. Consequence: contracts genuinely compete for throughput, so ties need an order, and `contracts.ron` file order is the deterministic tiebreak (ARCHITECTURE.md §3.9) until a player-set priority exists — the same shape as §11's open question about staff allocation.
- Multi-stage aged/fermented recipes (tier 6) are exactly what the generic `TimedProcess` component (ARCHITECTURE.md §3.5) was designed for — a chain of several `TimedProcess`-bearing stages feeding into each other, no new architecture needed.
- Perishability (refrigeration) and corrosion/durability (acid-handling) are the first concrete gameplay uses of "equipment condition/requirements" as a concept — neither exists yet as an ECS component. When these tiers are implemented, they'll need a component (or a tag + countdown on `Inventory`/`Machine`) representing spoilage or wear; that's an implementation detail for whoever builds tier 4/5, not decided here. Corrosion is also what feeds maintenance-staff demand (§7), so this component and the staff-pool mechanic below are linked.
- **Staff pools (§7) are a new architectural concept, closest in shape to `PowerNetwork`:** four aggregate demand/capacity counters (one per category), with machines declaring a per-category staff requirement while active, resolved as a capacity check each tick rather than per-entity simulation. This likely wants its own small resource type (e.g. `StaffPools { manufacturing, maintenance, packaging, logistics }`) checked alongside power in the same tiered-scheduler pass (ARCHITECTURE.md §3.7/§3.8), not a new ECS component per machine.
- **Equipment purchasing has its own data file, `shop.ron` (added 2026-09-08,
  the "Shop & Inventory" epic's ticket 1):** §4's "equipment purchases *are*
  the tech tree" needs a price per buildable kind, and that lives in
  `data/shop.ron` (`schema_version` 1, one `ShopEntry(id, price)` per kind)
  rather than as a field on `machines.ron`, because the set of *buildable*
  kinds is a superset of the set of *machine* kinds — belt and pipe tiles are
  priced and are not machines. The loader
  (`sim_core::sim::shop::ShopCatalog`) rejects a mismatched schema version, a
  duplicate kind, or a price below 1, and its `validate_covers` makes a
  machine kind with no price an error rather than something the player can
  never buy. What the player owns is a per-kind count
  (`shop::OwnedEquipment`), a plain field on `World` for the same reason
  `Economy` is one. **Money is deliberately not duplicated there:** §1 allows
  exactly one currency, so a purchase debits the same `Economy::money` a
  contract delivery credits, via `Economy::try_spend` (all-or-nothing, so a
  refused purchase changes nothing at all). Exposed to Godot as
  `SimBridge::shop_snapshot_json` (catalog + owned counts + money in one
  object). **No longer deferred (2026-09-08, the "Real Placement" epic's
  ticket 1):** an owned count is unplaced inventory now. `World::new` builds
  nothing at all, a new game starts with `economy.rs`'s `STARTING_MONEY`
  (3000 — enough to build the whole shipped line outright with room to spare;
  see the balance note at the end of this bullet), and every belt tile,
  pipe tile, machine and building in a world is something a `Purchase` paid
  for and a `Place` put down. `Place` spends one owned unit and `Remove`
  credits one back, so what you own plus what you have placed always equals
  what you bought; `Rotate` turns a placement 90° clockwise. All three leave
  money alone — it moved at purchase time, which is what keeps §1's single
  currency single. Prices, and the starting balance they are measured
  against, are still placeholders per §10 — but **the balancing pass's first
  job is done (2026-09-08, the inventory-hotbar ticket).** The "Real
  Placement" epic's ticket 2 had recorded a concrete hole here: at
  `STARTING_MONEY` 200, a new save could not be played forward to a contract
  payout *at all*, because money arrives only through deliveries, deliveries
  need a line that reaches the dock, and that line costs 1087. Not a slow
  game — a dead one. `STARTING_MONEY` is **3000** now, deliberately more than
  2.5x that bill: a new player can build the entire shipped six-stage line
  outright if they choose and still have about 1900 left for the mistakes and
  the second line, rather than being unable to reach the shipped content at
  all. The margin is the point — a balance that only just covered the bill
  would make a first factory a puzzle with one solution. `sim_core/tests/shop.rs`
  pins the *relationship* (starting money against the chain's own dynamically
  priced bill) rather than the two literals, so a price change moves the
  expectation with it instead of breaking it. Of the three options that entry
  listed, this is the larger starting balance; the third — an earlier sellable
  product, a contract the front end alone can satisfy — is still the one that
  would add progression rather than money, and is still worth doing.

- **Where things stand, and which way they face, is the player's decision
  (the same epic).** A machine accepts input on the face opposite the way it
  points and discharges on the face it points at; a belt carries toward the
  way it faces and hands off to whatever is standing there; a fluid
  connection is made by touching rather than by facing, because a pressure
  network is not a conveyor. Flow is therefore derived from what is adjacent
  to what, recomputed only when something is placed, removed or rotated — so
  a production line can turn corners, run in any of the four directions, and
  take shapes the old fixed west-to-east chain could not express. The hopper,
  the fluid tank and the delivery dock became ordinary placeable kinds in the
  same change, and a world may hold **any number of each**: several fill
  points, several delivery points, all still settling against the one shared
  delivery pool this section's allocation rule describes (a second dock is
  another door out of the factory, not a second ledger). `data/recipes.ron`'s
  file order no longer describes any geometry — it describes the production
  line as *content*, which is what makes the raw-ingredient rule below true,
  and `sim::reference_chain` is a test fixture that lays that line out using
  ordinary placement commands.
- **Raw ingredients are bought, and there is more than one (added 2026-09-08,
  the chain-rework epic's ticket 1):** §5's first cost axis — "ingredients —
  recurring, scales directly with production volume" — is implemented. A
  **raw** ingredient is defined once, structurally: an item or fluid some
  recipe in `data/recipes.ron` consumes and no recipe produces. The shipped
  chain has three (tomato, sugar, vinegar), which is what retired the old
  "the first recipe's first input is *the* raw ingredient" shortcut. Each is
  priced per unit in a new `data/ingredients.ron` — a separate file from
  `shop.ron` because the two price different kinds of thing: a shop entry is
  a building you place and can pick back up, an ingredient is stock a machine
  eats and never returns. A new `BuyIngredient` command debits the same
  single `Economy::money` counter §1 allows and credits a per-id stock ledger;
  `FillHopper`/`FillFluidTank` now draw from that stock and are silent no-ops
  without it, so nothing enters the factory that was not paid for. A hopper
  holds one raw item, chosen by the player when it is placed (the simulation
  still infers it when a chain has only one raw solid to choose from); a tank
  holds whatever the pipe run it discharges into carries. **Consequence worth
  stating:** overproduction is now a genuine loss, since a unit no contract
  has ordered is ingredients converted into unsold stock — which is the
  design pressure §6 chooses contracts over a spot market to create, and it
  makes §11's open question about a secondary spot market a live one rather
  than a theoretical one.
- **Money and active-contract state's architectural home (resolved at milestone 5, 2026-09-06):** both live as plain fields on a new `sim_core::sim::economy::Economy` struct, owned directly by `World` and advanced once per tick — not a new ECS component/entity, since milestone 5 has exactly one money counter and one active contract (no per-entity multiplicity to justify ECS machinery yet). `money` is a single fixed-point (`Fx`) counter, per ARCHITECTURE.md §3.9. The one hardcoded `Contract` (fixed mix/cadence/payout, named constants in `economy.rs`) is checked against an accumulating `delivery_buffer` `Inventory` at its cadence boundary (`Economy::tick`), which is where `World::issue_command(PlayerCommand::CollectOutput)` then deposited collected product instead of discarding it — this was milestone 5's "delivery mechanism." (**Superseded 2026-09-08 by the delivery dock**, two bullets up: that command no longer exists and the pool is filled by a building at the chain's tail instead. The pool itself, and everything about how contracts settle against it, is unchanged.) Exposed to Godot as a `SimBridge::economy_snapshot_json` JSON snapshot (money, contract mix, per-item progress, cadence), same pattern as the existing chunk/tile/layout snapshots. **Superseded in part on 2026-09-08:** `contracts.ron` data-file loading and multiple concurrent contracts have since landed — `Economy` now holds a `Vec<ActiveContract>` (each with its own cadence clock and delivery counters) plus one shared `delivery_buffer`, and `economy_snapshot_json` returns an array of contracts rather than one inline. The `Economy`-as-a-plain-field-on-`World` call above was unaffected and still stands: several contracts are a `Vec` inside the one struct, not a reason for ECS machinery. **Still explicitly deferred:** contract-loss/reputation penalties on a missed cadence window (a miss today just skips that window's payout and keeps the contract active — see this doc's own open question below), and the `StaffPools` resource — all future work, per GAMEPLAY.md §6/§9's original fuller design and the milestone-5 task's own scope note.

---

## 10. What this doc does not cover

- **Exact numeric balance** — ingredient costs, contract payouts, equipment prices, wage rates, staff-per-machine counts. Belongs in a future balancing pass, not this doc.
- **Specific real-world recipes/ratios** — this doc names sauces and roughly what's realistic (mayo needs cold handling, hot sauce needs acid handling) but not actual formulations.
- **Engine/architecture, data formats, ECS components** — see [ARCHITECTURE.md](ARCHITECTURE.md).
- **Visual style, camera, UI widgets** — see [UI_UX.md](UI_UX.md), though note §6's contract mixes/cadences are a natural fit for UI_UX.md §5's clipboard/checklist diegetic framing (a contract *is* a checklist), and staff headcounts per category (§7) are a natural fit for a simple gauge/meter widget per category.
- **Multiplayer/co-op factory ownership** — not considered here; ARCHITECTURE.md §8 covers multiplayer only as a lockstep-networking/determinism concern, not an economy-sharing one.

---

## 11. Open questions (not yet decided)

- Does a secondary spot-market/"sell excess at a flat lower rate" mechanic exist alongside contracts, or is contract delivery the *only* way to convert product into money? Affects how punishing overproduction is.
- What exactly happens when a delivery interval is missed once — immediate contract loss, a grace period, a reputation meter that only loses the contract after repeated misses?
- Is there a reputation/trust score gating access to higher-tier contracts, separate from equipment-tier gating?
- When staff demand exceeds headcount in a category, which machines lose out — player-set priority, first-come-first-served, or proportional throttling across all of them?
- Is there a one-time recruiting/training cost on top of the recurring per-head wage, and can headcount be reduced (firing) freely or with a penalty/delay?
- Are perishability (spoilage) and corrosion (durability) modeled as full ECS components with their own tick-based decay, or as simpler flags/costs? Needs a decision before tier 4/5 implementation starts.
- Exact tier list beyond the six in §2 (this doc's tiers are illustrative of the *shape* of progression, not necessarily the final content list), and the actual catalog of derived sauces/variants — both are a game-design task, not an architecture or gameplay-systems one.
