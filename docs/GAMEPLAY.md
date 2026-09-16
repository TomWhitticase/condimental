# CONDIMENTAL — Gameplay, Progression & Economy Plan

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

**Model:** staff are an abstract resource pool, tracked per category, with a hired **headcount** (capacity) and a **recurring wage per head**. Every machine or process declares how many staff of which category it needs *to be crewed at all* — the same shape as Factorio-style power draw: a machine that can't get enough of its required resource stalls, it doesn't run degraded or explode. The **requirement** for a category is the sum of the crews of every machine the player has **placed**, running or not, so it moves only when a machine is placed, removed or rotated — never with production flow. That is the number "do I need to hire?" is answered from. Until 2026-09-16 it was the per-tick figure instead: a machine starved of input asked for nobody, so the requirement breathed with the belts and the advice to hire had changed by the time the player acted on it. The per-tick figure is still computed and still shown, demoted to "what is working right now". If a tick's demand for a category exceeds hired headcount, some subset of that category's machines simply cannot run that tick (which subset — player-set priority, first-come, proportional throttling — is an open question, §11). This reuses the same conceptual pattern `PowerNetwork` already uses in ARCHITECTURE.md §3.2/§3.8: aggregate demand vs. capacity, no per-entity simulation.

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

Everything in this doc is **content**, not simulation code, per
[ARCHITECTURE.md](ARCHITECTURE.md)'s data-driven-recipes rule. This section
states the current design rules only; `docs/DEVLOG.md` has the dated
implementation history (epics, tickets, balancing passes) behind each one.

- Sauce recipes and their tier/equipment requirements live in
  `recipes.ron`/`machines.ron`. Derived sauces and variants (§3) need no new
  mechanism — a recipe listing another sauce item as an input is the
  standard Factorio-style intermediate-product pattern.
- **Contracts are a board of offers, not a list of what's running.**
  `data/contracts.ron` (schema 2) gives every contract an id, mix, cadence,
  payout, `price` to sign and a `strikes` count; some are marked `starter`
  and run from a new game, the rest are signed from the contracts board
  (**C**) for their price. Missing `strikes` windows in a row loses the
  contract (losing IS the penalty, no fine on top); a paid window resets the
  count. A signed contract takes its file-order position in the active
  list, which is also the deterministic tiebreak for scarcity.
- **All active contracts draw from one shared pool of collected-but-
  undelivered stock.** A contract is satisfied at its cadence boundary only
  if the pool holds its full mix, and payout consumes that mix from the
  pool — goods are never credited to two contracts at once.
- **That pool has a ceiling, and it is per item id** (48,
  `DELIVERY_STOCK_CAPACITY`, added 2026-09-16): production nobody has
  ordered is a real loss (§5), and an unbounded sink made overproduction
  cost nothing but a growing number. Per item rather than one total,
  because one total can deadlock the game — an overproduced product would
  crowd every other item out of the pool, a contract whose mix names a
  crowded-out item could never be satisfied again, and nothing in the game
  dumps excess stock (§11's spot-market question is still open). Per item,
  every id always has its own room and a clog is always specific. The
  number is a *relationship*, not a literal, like `STARTING_MONEY`: it must
  comfortably exceed the largest single mix quantity on the board (20, so
  48 is 2.4x it), and `tests/dock.rs` pins that against `contracts.ron`
  rather than the figure.
- **The delivery dock is a pass-through, not a store.** It ships everything
  it receives the same tick into the shared delivery pool
  (`EconomySnapshot::delivery_stock`); its own buffer reads empty between
  ticks. It keeps a bounded per-item buffer (32) so intake can still refuse
  and back up the belt rather than lose product — and since the pool it
  drains into can be full, that refusal is now a state the player meets: a
  dock with nowhere to put a box leaves it on the belt and the line backs
  up behind it, exactly as a full machine buffer does. A full dock is not a
  penalty on top; the backed-up line IS the consequence.
- **The contract board is sized in a line's worth of demand**: a starter
  tier one kept-fed line can just serve, plus a growth tier of larger,
  slower orders sized so a second/third/fourth line always has an order to
  go after. Per-box payout never falls as order size rises; file order
  (smallest first) is the scarcity tiebreak.
- Multi-stage aged/fermented recipes (tier 6) are exactly what the generic
  `TimedProcess` component (ARCHITECTURE.md §3.5) was designed for — a
  chain of stages feeding into each other, no new architecture needed.
- **Perishability and corrosion are not yet implemented.** They'll need a
  component (or a tag + countdown on `Inventory`/`Machine`) for spoilage or
  wear when tier 4/5 lands; corrosion also feeds maintenance-staff demand
  (§7). Still an open design question, not decided here.
- **Equipment purchasing is `data/shop.ron`** (schema 1, one price per
  buildable kind — a superset of machine kinds, since belts and pipes are
  priced too). Money is not duplicated there: a purchase debits the same
  single `Economy::money` a contract delivery credits, via
  `Economy::try_spend` (all-or-nothing). `World::new` builds nothing; every
  tile in a world was `Purchase`d then `Place`d. `Place`/`Remove` move a
  unit between owned and placed; `Rotate` turns a placement 90°; none of
  the three touch money. `STARTING_MONEY` (3000) is pinned as a
  *relationship* to the shipped chain's build cost (>2.5x it, not a bare
  literal), so a new player can build the whole line outright with a real
  margin for mistakes and a second line.
- **Placement direction decides flow.** A machine accepts input opposite
  its facing and discharges on the face it points at; a belt hands off to
  whatever is downstream of its facing; a fluid connection is made by
  touching, not facing, since a pressure network is not a conveyor. Flow is
  derived from adjacency, recomputed only on placement changes. The hopper,
  fluid tank and delivery dock are ordinary placeable kinds — a world may
  hold any number of each, all settling against the one shared delivery
  pool.
- **A raw ingredient is defined structurally**: an item or fluid some
  recipe consumes and none produces. Each is priced per unit in
  `data/ingredients.ron` (separate from `shop.ron` — a shop entry is a
  placeable you can pick back up, an ingredient is stock a machine eats and
  never returns). `BuyIngredient` debits money and credits a per-id stock
  ledger that `FillHopper`/`FillFluidTank` draw from; nothing enters the
  factory that wasn't paid for. Overproduction is a genuine loss.
- **Auto-fill spends stock, never money.** A hopper or tank can be switched
  to automatic and tops itself up every tick from already-bought stock; it
  never debits the money counter, keeping the ingredient spend a visible
  shop decision rather than a background drain. An empty larder just stops
  the top-ups.
- **Every recipe-running machine measures its own load** (a rolling
  window of working/starved/blocked, with the concrete stall reason) so a
  bottleneck is legible: the machine reading *working* while everything
  after it reads *starved* and everything before it reads *blocked* is the
  one to duplicate. It costs nothing and gates nothing — read via hover
  only (the in-world stack lights were tried and removed after a
  playtest verdict on looks; a floor-wide readout is an open question).
- **Staff pools are four aggregate categories** (manufacturing,
  maintenance, packaging, logistics — `data/staff.ron`), a plain field on
  `World` beside `Economy`, not an ECS component. Every machine kind
  declares the crew it needs while running; each tick the pools reset to
  headcount and machines claim their whole crew in stable order — a
  refused machine holds its progress and reads *starved* with a `staff`
  reason (understaffed is a kind of starved, no fourth state). Payday pays
  one head at a time in `staff.ron` order out of the one money counter; the
  first unpayable head, and every one after it, goes **unpaid** — still
  hired, idle until the next payday, gone only if unpaid there too, and a
  covered payday clears the arrears (the contract board's strikes, applied
  to people). A head is billed for any period it was employed in any part
  of (notice pay, so firing before payday saves nothing), and a hire is
  refused unless the balance covers the next bill plus the new wage, so
  nobody works a period for free and no head is hired who cannot be paid.
  **MVP-provisional, open to revision:** shortfall allocation is
  first-come in stable order (no player priority yet); no recruiting fee;
  firing is free and immediate but still pays out the period; an unpaid
  head is idled once and gone the second time (no debt, no reputation
  meter).
- **Money and contract state are plain fields on `sim_core::sim::
  economy::Economy`**, owned by `World`, not ECS entities — no per-entity
  multiplicity to justify that. `money` is a single fixed-point (`types::Money`,
  `I48F16`) counter (ARCHITECTURE.md §3.9). `Economy` holds a `Vec<ActiveContract>`
  (each with its own cadence clock) plus the one shared `delivery_buffer`;
  several contracts are a `Vec` inside one struct, not a reason for ECS
  machinery. Exposed to Godot as `SimBridge::economy_snapshot_json`.

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
- ~~What exactly happens when a delivery interval is missed once — immediate contract loss, a grace period, a reputation meter that only loses the contract after repeated misses?~~ **Decided 2026-09-10 (§9, the contract-board bullet):** a grace period counted in strikes — `strikes` consecutive missed windows lose the contract, a paid window resets the count, and a lost contract is bought back at its listed price. No reputation meter yet.
- Does an idle machine cost anything? **Decided 2026-09-16 (§7): no, but it
  still needs a crew on the books.** The requirement is what is PLACED, so
  buying a machine you are not yet feeding raises the bill immediately. That
  is the price of a stable number. The open part is whether a machine should
  be *mothballable* — switched off deliberately, releasing its crew — so a
  player can build ahead without paying ahead. Nothing in the wire or the save
  format precludes it; it is a balance question, not a technical one.
- Is there a reputation/trust score gating access to higher-tier contracts, separate from equipment-tier gating?
- When staff demand exceeds headcount in a category, which machines lose out — player-set priority, first-come-first-served, or proportional throttling across all of them? **MVP-provisional (2026-09-12, §9's staff-pools bullet):** first-come in the simulation's stable machine order, so the same machine is always the one short; player-set priority is the expected replacement and needs no wire change.
- Is there a one-time recruiting/training cost on top of the recurring per-head wage, and can headcount be reduced (firing) freely or with a penalty/delay? **MVP-provisional (2026-09-12), amended by the quality pass (2026-09-13):** no recruiting fee, but a hire needs the balance to cover the next bill plus the new wage; firing is free and immediate, but the fired head is still paid for the period it worked any part of (notice pay); a head that cannot be paid on payday is idled, not lost, and leaves only if the next payday cannot pay it either. Still placeholders for the balancing pass, not decisions.
- Are perishability (spoilage) and corrosion (durability) modeled as full ECS components with their own tick-based decay, or as simpler flags/costs? Needs a decision before tier 4/5 implementation starts.
- Exact tier list beyond the six in §2 (this doc's tiers are illustrative of the *shape* of progression, not necessarily the final content list), and the actual catalog of derived sauces/variants — both are a game-design task, not an architecture or gameplay-systems one.
