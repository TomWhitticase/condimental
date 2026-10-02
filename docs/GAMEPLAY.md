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
| Sauce progression | Simple, few-ingredient sauces (tomato sauce) → composite condiments (ketchup, BBQ) → equipment-gated categories (mayo needs an emulsifier, hot sauce needs acid handling) → elaborate long-process sauces (aged/fermented) |
| Derived sauces | Higher-tier sauces can require other *finished sauces* as an ingredient (chilli mayo needs mayo + hot sauce; honey mustard needs mustard), turning the sauce list into a branching dependency tree, not a flat list |
| Variants | Many sauces come in flavour variants (garlic mayo, chilli mayo, honey mayo; smoky BBQ, spicy BBQ) built by taking a plain base sauce and running it through a small flavour-add step — cheap content, real contract variety |
| Demand model | **Contracts**, not a spot market — standing orders for a *mix* of sauces, delivered on a recurring cadence, that must be kept up or lost |
| Money | Earned from contract deliveries; spent on ingredients (recurring), equipment (capital), and staff wages (recurring) |
| Staff | Not simulated NPCs — an abstract resource pool per category (manufacturing, engineering, packaging, logistics). Running equipment consumes staff capacity the way it would consume power; understaffing stalls production, not a person failing to show up |
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
| 3 | BBQ sauce — **shipped 2026-09-18**, and it needed **no new machine** | + spice mix, molasses, liquid smoke, and *finished ketchup* | None: a second recipe on the mixer and the cooking vat the ketchup line already owns | A 600-tick cook — the longest process in the game by 2.5x — bought the "low and slow" identity with a number instead of a machine. It is gated instead by output sold — a hundred boxes of ketchup (§9's progression-gates bullet) — which is this table's first gate that is neither of the two kinds below |
| 3 (a second one) | Mustard — **shipped 2026-09-26**, and it needed **no new machine** either | + mustard seed, turmeric. **Not** vinegar: the ketchup mixer drinks that from tick 0, so the gate cannot hold it back (`data/unlocks.ron`) | None: the same crusher, mixer, cooking vat, bottler and packer. The novelty is a *wet mill*: `mustard_mash` takes 4 seed **and 2 vinegar** and sends a fluid mash out, where the crusher's only other recipe (`tomato_paste`) is solid in, solid out. That is placement, not new capital: `ketchup_mix` already drinks raw `vinegar` on the mixer, and a second mixer recipe drinking any raw fluid leaves a pipe run the narrowing pass cannot name — the shipped ketchup line would stall on `AmbiguousPipe` — so the crusher, which drank nothing, is where a raw fluid could still enter (`data/recipes.ron`) | A second tier that buys no capital, so like BBQ the only thing making it arrive later is a **progression gate**: 100 boxes of BBQ sold. Its cook sits between the two it follows (360 ticks against ketchup's 240 and BBQ's 600). Measured from a new game, the gate opens at tick 131 000 — ~73 min at 30 ticks/sec, 79 000 ticks after the BBQ gate at 52 000 — on a line that never runs dry, so that is a floor (`godot_project/measure_gate_ticks.gd`) |
| 4 | Mayonnaise / aioli | Eggs, oil, acid | **Emulsifier** machine | An emulsion is a process the mixer cannot do, so the capital purchase is the gate. **No spoilage and no cold chain** — the game does not model either (decided 2026-10-02, §11). Once mayo exists it is the base its own variants and derived sauces draw on (§3) |
| 5 | Hot sauce | Chilies, vinegar (high concentration) | **Acid-handling equipment** (acid-resistant tanks/piping) + fermentation vessel for mash-based variants | High-acid ingredients corrode standard equipment; this tier is gated by a *material/durability* requirement, not just a recipe unlock |
| 6 | Elaborate/aged sauces (fermented hot sauce, barrel-aged BBQ, whole-grain mustard, etc.) | Combines multiple tier 2–5 ingredient classes | Multiple equipment lines feeding one recipe, plus multi-stage `TimedProcess` chains (mash → ferment → strain → blend → age → bottle) lasting far longer than any earlier tier | The payoff tier: production chains that span the whole factory, not one station |

Two structurally different kinds of gate recur through this table, and both should stay legible to the player as distinct reasons a sauce is locked:

- **Recipe-complexity gates** (tiers 1→3): more ingredients, more process steps, longer single `TimedProcess` — no new equipment *category*, just a bigger version of what you already have.
- **Equipment-category gates** (tiers 4, 5): an entire class of sauce is inaccessible until a specific piece of capital equipment exists, because the sauce cannot be made at all without it (an emulsion needs an emulsifier; high-acid ingredients corrode non-resistant equipment).

Tier 6 combines both: by then the player has every equipment category, and the remaining challenge is orchestrating long multi-stage chains across them.

**Mustard is a second tier 3, not a tier 4.** By this table's own
definition that is what it is: a recipe-complexity gate — no new machine, no
new handling axis, one more recipe on a line that already exists — and the
alternative, renumbering it to 4, would push mayonnaise, hot sauce and the
aged sauces to 5/6/7 and change what "tiers 1→3" and "tiers 4, 5" mean in the
two bullets above, to record a sauce that introduces nothing new. A tier is a
class of obstacle here, and two sauces can share one. Nothing in the code
reads the number: gate order is `data/unlocks.ron`'s file order, and the only
thresholds the sim ever compares are two gates on the *same* product
(`docs/DECISIONS.md`, 2026-09-26). This column is design prose, and what
actually holds mustard back is the progression gate below.

**A third kind of gate exists now, and it is the one both shipped sauces above
tier 2 use.** BBQ shipped as a recipe-complexity tier that buys nothing — its
machines are the ones the ketchup line already has — so neither gate above could hold it back,
and the game would have handed a new player the whole two-sauce board in their
first minute. What gates it is a **progression gate**: a count of boxes of a
named product SOLD that releases a group of ingredients and contracts
(`data/unlocks.ron`, §9). Each tier counts the tier below it — BBQ opens at 100
boxes of ketchup, mustard at 100 boxes of BBQ — so a sauce is earned by the
line before it actually running, not by time passed or money banked. It is not a third *design* axis so much as the
admission that a tier which costs no capital still needs a reason to arrive
later than the first minute. See `docs/DECISIONS.md`, 2026-09-21, and
2026-09-26 for the trigger it counts now.

---

## 3. Derived sauces & variants

Two more axes of depth sit on top of the tier table, and both are deliberately *cheap content* — small recipe additions rather than whole new production lines — because that's what lets the sauce list grow wide without every addition demanding a new machine.

**Derived sauces (sauce-as-ingredient):** some recipes consume another sauce as an input rather than only raw ingredients — chilli mayo requires a batch of mayo plus a batch of hot sauce; honey mustard requires mustard plus honey. This needs no new architecture: a sauce is just another item, and Factorio-style intermediate products (an item that is simultaneously a sellable output *and* an input to another recipe) are already exactly how `recipes.ron` works. The design payoff is that the sauce list becomes a branching dependency tree instead of a flat list of unrelated recipes — unlocking hot sauce (tier 5) doesn't just unlock hot sauce, it unlocks every recipe downstream of it too (chilli mayo, chilli BBQ, etc.), so later equipment purchases pay off across multiple sauces at once.

**BBQ is the first of these to ship (2026-09-18), and the branching turned out
to have teeth.** Its mixer takes *finished ketchup* as a fluid input, tapped
off the pipe leaving the ketchup vat **before the bottler**. A fluid network is
a shared pool, so one ketchup run touching both a bottler and a BBQ mixer feeds
both and they contend for it: every box of BBQ is a box of ketchup you did not
ship. That is §6's diversification pressure made concrete — serving a mixed
contract is not "run two lines", it is "build enough ketchup for both" — and it
is why the 600-tick cook is load-bearing content rather than flavour. Against
the vat's 240 ticks it settles one ketchup line plus one BBQ branch at
**3 ketchup : 2 BBQ**, which is exactly the mix `grill_pub` asks for, so a
player who builds the obvious thing can serve it without tuning anything.
Change the cook time and those contract mixes change with it, or the claim
stops being true.

**Variants (flavour families):** many sauces should exist as a family — plain mayo, garlic mayo, chilli mayo, honey mayo; plain BBQ, smoky BBQ, spicy BBQ — built as a **base sauce** (the tier-gated recipe from §2) plus a small **flavour-finishing step** that consumes the base plus one or two flavour ingredients to produce a distinct, separately-sellable item. Variants are not a new tier or a new equipment unlock; they're a cheap multiplier on existing tiers, specifically so the *contract* system (§6) has a much larger pool of distinct sauces to demand mixes of without the player needing new machinery for every one. Note the flavour-finishing step overlaps with derived sauces above when the flavour ingredient is itself a sauce (chilli mayo is simultaneously "mayo + hot sauce" as a derived sauce *and* readable as "a chilli variant of mayo") — that's intentional; they're the same underlying mechanic (a recipe that consumes a finished sauce) viewed from two design angles.

**Contract implication:** once variants exist, contracts should be able to name a specific variant (e.g. "50 garlic mayo + 50 chilli mayo," not just "100 mayo") — this is what actually makes variants matter for the diversification design pressure in §6, rather than being purely cosmetic flavour text.

---

## 4. Equipment & tech tree

Equipment purchases *are* the tech tree — there's no separate abstract skill-point currency. Each major piece of equipment is a capital purchase (one-time cost, paid from contract earnings) that unlocks a sauce category outright, the same way Factorio's research unlocks recipes but paid in cash instead of science packs.

**One sauce has shipped that this table does not explain, deliberately.** BBQ
(tier 3) needs no equipment tier at all: it runs on the mixer and cooking vat
the ketchup line already owns, and it arrives on a count of boxes sold rather than a
purchase (§9, `data/unlocks.ron`). The table below still describes the intended
shape for tiers 4-6, where the equipment genuinely is the gate — a sauce that
cannot be made at all without it. Do not read "every sauce is bought"
out of it; read "every sauce that needs a new *capability* is bought".

| Equipment tier | Unlocks | Gameplay effect |
|---|---|---|
| Basic cooker/kettle | Tomato sauce | Starting equipment |
| Reduction/thickening + bottling line | Ketchup | First composite recipe, first bottling step |
| ~~Smoker/slow-reduction unit~~ — **not built, and not needed** | BBQ sauce (shipped) | A long single-stage `TimedProcess` on the existing cooking vat: 600 ticks against ketchup's 240. The identity was bought with the number, not the machine |
| **Emulsifier** | Mayo/aioli, and the variants and derived sauces built on mayo | A new machine for a process the mixer cannot do. ~~Refrigeration (cold storage + cold-chain conveyance), introducing spoilage~~ — **dropped 2026-10-02**: the game has no spoilage, so nothing needs keeping cold (§11) |
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

**A gate is a rule, and it is also a promise.** `data/unlocks.ron` names the
ids a tier holds back, and the simulation REFUSES every command that would
reach one: `BuyIngredient`, `SignContract`, and — since 2026-09-21, issue
#90 — pointing a placed hopper or fluid tank at a locked ingredient. The
market omits a locked row; the configuration panel's CONTENTS picker keeps
it and padlocks it, naming the unlock and the boxes still to sell. That
asymmetry is deliberate and is the promise half: a gate the player can see
coming is a thing to aim at, while one that materialises unannounced is a
surprise they could not have planned for. The one place the rule stops is
`Place`'s `item` field, which is how the build ghost and the test fixtures
seed a bin rather than a choice made against a gate — the ghost never offers
a locked id, and a bin seeded with one can hold nothing anyway, because the
raw cannot be bought.

**Expansion is a cost the contract board charges, and nobody warns you**
(measured 2026-09-20, re-measured after review, issue #55). Building the BBQ
branch off a running ketchup line stops boxed-ketchup output for about
8 000-9 000 ticks: the new pipe runs and the second vat fill from the
*existing* network, so roughly 35 boxes' worth of throughput goes into priming
pipework before a single BBQ box exists. Delivery windows keep ticking through
that, and three missed windows lose a contract. **What decides whether it
costs you an order is how many finished boxes are banked at the moment you
build** — a player with 22 banked and one order loses nothing, 22 banked and
three orders loses one, 7 banked and three orders loses two. The unlock gate
(`data/unlocks.ron`) only steers that bank by proxy, and badly, because
signing more contracts is both how a player reaches the gate sooner and what
makes the transient cost more. That was measured while the gate counted
delivery windows: pushed to 40 windows it did cover even the three-order
player, but 40 windows was far outside the first session the gate existed to
sit inside. **The 2026-09-26 change to 100 boxes of ketchup lands at about
that 40-window mark** (44 windows with all three starter-era orders signed,
50 with the starter order alone), so the trap measured here mostly does not
bite at the shipped numbers any more — by side effect, not by design. The
reasoning stands regardless: **a progression gate is a poor instrument for a
transient cost**, and a balance pass that moved the gate back would restore
the trap without touching anything that looks like it. The right ones are a warning at placement
time, before the player commits, and a fluid network that does not let a
brand-new consumer take 100% of a network the old one is living on. Both are
still to build (issues #92, #93); the measured table and the protocol behind
it are in `data/unlocks.ron`'s comment and the full record is in
`docs/DECISIONS.md`.


---

## 7. Staff — a resource pool, not NPCs

Staff are **not** simulated as entities walking around the factory interacting with machines — that's deliberately out of scope, both because it adds pathfinding/animation complexity the sim core doesn't need and because it doesn't serve the actual design goal, which is staff as another *capacity constraint* alongside power/ingredients, not a character system.

**Model:** staff are an abstract resource pool, tracked per category, with a hired **headcount** (capacity) and a **recurring wage per head**. Every machine or process declares how many staff of which category it needs *to be crewed at all* — the same shape as Factorio-style power draw: a machine that can't get enough of its required resource stalls, it doesn't run degraded or explode. The **requirement** for a category is the sum of the crews of every machine the player has **placed** and not mothballed, running or not, so it moves only when the player places, removes or mothballs a machine, or loads a save — never with production flow. That is the number "do I need to hire?" is answered from. Until 2026-09-16 it was the per-tick figure instead: a machine starved of input asked for nobody, so the requirement breathed with the belts and the advice to hire had changed by the time the player acted on it. The per-tick figure is still computed and still shown, demoted to "what is working right now". If a tick's demand for a category exceeds hired headcount, some subset of that category's machines simply cannot run that tick (which subset — player-set priority, first-come, proportional throttling — is an open question, §11). This reuses the same conceptual pattern `PowerNetwork` already uses in ARCHITECTURE.md §3.2/§3.8: aggregate demand vs. capacity, no per-entity simulation.

**Categories (four, deliberately not more):** chosen to map directly onto the four pipeline stages a sauce actually passes through, so the split stays legible rather than feeling like an arbitrary RPG-style stat split:

| Category | Required by | What it represents |
|---|---|---|
| **Manufacturing** | Cookers, reducers, emulsifiers, fermenters — anything actively turning ingredients into sauce | Running the production machinery itself |
| **Engineering** | Repairing degraded/broken equipment and keeping it running | Directly tied to the corrosion/wear mechanic from acid-handling equipment (§4) and any other durability-based gate added later — more high-wear equipment means more engineering demand, not just more manufacturing demand. Called *maintenance* until 2026-10-02; renamed by the owner, with no change in meaning |
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
4. Save toward an emulsifier. Once bought, mayo-inclusive contracts become available/eligible, and mayo becomes the base that later variants and derived sauces draw from (§3).
5. Save toward acid-handling equipment; hot-sauce contracts open up; standard piping used for a hot-sauce line starts showing corrosion, driving up engineering-staff demand and nudging the player toward the acid-resistant upgrade instead of just hiring more repair staff to keep patching it.
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
- **A belt and a pipe may cross, at right angles, on one tile.** Place a pipe
  onto a belt (or a belt onto a pipe) and the tile becomes a `crossing`: one
  entity that is both a belt carrying items and a pipe carrying fluid, drawn
  as the pipe lifted over the lane on a gantry. There is nothing to buy and
  nothing on the hotbar — the ghost turns valid over a perpendicular belt and
  says so. A parallel one is refused (that is a pipe laid along a belt, not
  across it). Removing takes the whole tile and refunds both a belt and a
  pipe; rotating turns both. No other pair stacks. (Full record:
  `docs/DECISIONS.md`, 2026-10-01.)
- **A machine kind may name several recipes, and what is plumbed into a
  placed machine decides which one it runs** — first fully-satisfied wins,
  `recipes.ron` file order is the tiebreak, and there is **no player-facing
  recipe picker**. A mixer on a vinegar pipe beside a sugar hopper blends
  ketchup mix; one on a ketchup pipe beside spice and molasses hoppers blends
  BBQ mix. The layout is the choice. (Full record: `docs/DECISIONS.md`,
  2026-09-21.)
- **Progression gates are content too** — `data/unlocks.ron` (schema 1). A
  gate names a product, a number of boxes of it that must be SOLD (delivered
  into a satisfied contract window), and two lists: ingredients held back from
  the market, contracts held back from the board.
  Both are enforced in the simulation, not the panel: buying or signing a
  gated id is refused. A held-back contract is still LISTED on the board,
  padlocked with its gate and progress, so the next tier is visible before
  it opens (2026-09-29). **No recipe is ever locked** — a gated sauce's recipes
  sit in `recipes.ron` from a new game and are merely unbuildable and
  unsellable — which keeps progression entirely off the tick path. The gate
  announces itself by name when it opens, which is the point of having one.
  Two gates ship: BBQ at 100 boxes of ketchup, mustard at 100 boxes of BBQ.
  Gating a tier on the tier below it is what makes the chain a chain; a third
  sauce behind a fourth product is a data edit and nothing else. The trigger
  counted delivery windows until 2026-09-26 — windows are blind to both the
  quantity and the product, so they did not measure output. (Full record:
  `docs/DECISIONS.md`, 2026-09-26.)
- **Contracts are a board of offers, not a list of what's running.**
  `data/contracts.ron` (schema 2) gives every contract an id, mix, cadence,
  payout, `price` to sign and a `strikes` count. A new game runs none of
  them (since 2026-09-26; a `starter` flag would run one from tick 0, and
  the shipped board sets it on nothing) — every contract is signed from the
  contracts board (**C**) for its price. Missing `strikes` windows in a
  row loses the contract (losing IS the penalty, no fine on top); a paid
  window resets the count. A signed contract takes its file-order position
  in the active list, which is also the deterministic tiebreak for
  scarcity.
- **Each dock holds its own stock, and contracts draw on the docks
  together** (per dock since 2026-09-26; one shared pool before that). A
  contract is satisfied at its cadence boundary only if the docks able to
  ship this tick hold its full mix *between them*, and payout consumes each
  line from those docks in topology order — the first dock gives what it
  has, then the next — so goods are never credited to two contracts at once
  and the split is deterministic. A dock that is mothballed or was refused
  its logistics crew this tick is not drawn from. A dock asks for its
  logistics crew only while it holds something an active contract orders:
  shipping is that crew's work, so a dock of goods nobody has ordered asks
  for nobody and never keeps a head from the dock whose stock is wanted. Every dock a payout took
  from is the dock that paid: the coins rise over it and no other.
- **A dock has a ceiling: `DOCK_CAPACITY` (32) places, one case each,
  whatever the case holds.** Production nobody has ordered is a real loss
  (§5), and an unbounded sink made overproduction cost nothing but a
  growing number. A dock at its ceiling refuses the belt feeding it — the
  box stays on the belt and the line backs up behind it, exactly as a full
  machine buffer does. A full dock is not a penalty on top; the backed-up
  line IS the consequence. One total per dock can clog a dock with a
  product nobody orders; the valve is **removing the dock**, which
  destroys what it holds (the ordinary `Remove` rule), and a clog is
  always local to the dock that overproduced — the rest of the factory's
  docks keep taking. (The 2026-09-16 shared pool was capped per item for
  exactly that deadlock, since it had no valve.) The number is a
  *relationship*, not a literal, like `STARTING_MONEY`: it must comfortably
  exceed the largest single mix quantity on the board (at least 1.5x it —
  20 today, so 32 is 1.6x), and `tests/dock.rs` pins that against `contracts.ron` rather than
  the figure.
- **The contract board is sized in a line's worth of demand**: a starter
  tier one kept-fed line can just serve, plus a growth tier of larger,
  slower orders sized so a second/third/fourth line always has an order to
  go after. Per-box payout never falls as order size rises; file order
  (smallest first) is the scarcity tiebreak.
- Multi-stage aged/fermented recipes (tier 6) are exactly what the generic
  `TimedProcess` component (ARCHITECTURE.md §3.5) was designed for — a
  chain of stages feeding into each other, no new architecture needed.
- **Corrosion is not yet implemented.** It will need a component (or a tag
  + countdown on `Machine`) for wear when tier 5 lands, and it also feeds
  engineering-staff demand (§7). Still an open design question, not decided
  here. **Spoilage is not a mechanic at all** (decided 2026-10-02, §11): no
  item, ingredient or fluid perishes, so nothing needs a freshness countdown
  and no tier needs a cold chain.
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
  whatever is downstream of its facing, and into a machine or dock only
  when it points through the inlet face — a belt into a flank, or back into
  the outlet face, feeds nothing (enforced 2026-10-01, issue #1); a fluid connection is made by
  touching, not facing, since a pressure network is not a conveyor. Flow is
  derived from adjacency, recomputed only on placement changes. The hopper,
  fluid tank and delivery dock are ordinary placeable kinds — a world may
  hold any number of each, and contracts settle against every dock's stock
  together.
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
  engineering, packaging, logistics — `data/staff.ron`), a plain field on
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
  of (notice pay, so firing before payday saves nothing), so nobody works a
  period for free. A hire the balance could not cover — the next bill plus the
  new wage — was **refused** until 2026-09-26; it is **warned** now: the sim
  answers `hire_warns`, the player is asked to confirm, and the hire happens
  if they say yes. It changed because every other spend gained the same
  warning at the same time (`docs/UI_UX.md` §2b point 40), and of the two a
  refusal is the one a player who knows what they are doing cannot override.
  **MVP-provisional, open to revision:** shortfall allocation is
  first-come in stable order (no player priority yet); no recruiting fee;
  firing is free and immediate but still pays out the period; an unpaid
  head is idled once and gone the second time (no debt, no reputation
  meter).
- **A crewed building can be MOTHBALLED, and that is the way out of
  arrears** (`set_mothballed`, 2026-09-20; full record in
  [DECISIONS.md](DECISIONS.md)). A mothballed building asks for no crew and
  its crew is **stood down**: it comes off the payroll, so the wage bill
  falls on the tick the switch is thrown WITHOUT anybody being fired, and a
  stood-down head never walks out. Standing crew down on payday eve buys
  notice pay, not free labour — a head already owed for this period stays
  owed. The building produces nothing and HOLDS the batch it was part-way
  through, so switching it back on resumes rather than restarts.
  **Anything that asks for a crew can be switched off**, which is decided
  by the kind's `staff` list in `data/machines.ron` — so a machine can, and
  so can the **dock**, which has to be able to or `required("logistics")`
  never reaches zero and a factory with no income can never get its bill to
  zero. A hopper, a tank, a belt, a pipe and a splitter ask for nobody and
  are refused. It costs nothing and saves nothing directly, so there is no
  new balance number: what it changes is who is on the payroll. What it
  does **not** do is conjure income — a standing order lost to three missed
  windows during a shutdown costs money to re-sign, which is issue #85 and
  still open.
- **Every placement reports whether it has its crew** (`crewed`, 2026-09-20)
  — the establishment's stable answer to *if everything placed wanted to run
  at once, who goes without?*, allocated in the order the tick claims in.
  Deliberately not a replay of the last tick, which flickers as the line
  ebbs. It is what lets the UI name the one machine that is short instead of
  only saying the factory is.
- **A staff category can be renamed without breaking a save**
  (2026-10-02; full record in [DECISIONS.md](DECISIONS.md)). Saves key the
  crew by category id, so the renamed category lists its old id in
  `renamed_from` in `data/staff.ron`; a save naming the old id loads into
  it, counts and all, and the next save writes the new one. No save-schema
  bump — a rename is a content edit. The first use is `engineering`, which
  was `maintenance`.
- **Money and contract state are plain fields on `sim_core::sim::
  economy::Economy`**, owned by `World`, not ECS entities — no per-entity
  multiplicity to justify that. `money` is a single fixed-point (`types::Money`,
  `I48F16`) counter (ARCHITECTURE.md §3.9). `Economy` holds a `Vec<ActiveContract>`
  (each with its own cadence clock); what waits to be sold stands on the docks
  themselves (`dock::Dock`), lent to `Economy::tick` as a `DeliveryStock`
  on the tick it settles. Several contracts are a `Vec` inside one struct, not a reason for ECS
  machinery. Exposed to Godot as `SimBridge::economy_snapshot_json`.

---

## 10. What this doc does not cover

- **Exact numeric balance** — ingredient costs, contract payouts, equipment prices, wage rates, staff-per-machine counts. Belongs in a future balancing pass, not this doc.
- **Specific real-world recipes/ratios** — this doc names sauces and roughly what's realistic (mayo needs an emulsifier, hot sauce needs acid handling) but not actual formulations.
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
  is the price of a stable number. ~~The open part is whether a machine
  should be *mothballable* — switched off deliberately, releasing its crew —
  so a player can build ahead without paying ahead.~~ **Decided 2026-09-20
  (§7, the mothballing bullet):** yes, and it is the escape from the unpaid
  spiral as much as a way to build ahead. `set_mothballed` drops the
  building out of the establishment, STANDS ITS CREW DOWN so the wage bill
  falls without firing anyone, stops its output and HOLDS its batch. Any
  kind that asks for a crew can be switched off, the dock included. What it
  does not do is replace a standing order lost while the factory was dark —
  see the decision record, and issue #85.
- ~~Is there a reputation/trust score gating access to higher-tier
  contracts, separate from equipment-tier gating?~~ **Half-decided 2026-09-19
  (§9's progression-gates bullet):** higher-tier contracts *are* gated
  separately from equipment, and the key is a **lifetime successful delivery
  count** — proof that a line ran end to end — releasing a named group of
  ingredients and contracts together (`data/unlocks.ron`). **There is no
  reputation meter, and a gate never closes**: the count only rises, a missed
  window costs nothing against it, and a lost contract does not take its past
  deliveries back. Still open: whether anything ever *degrades* standing —
  the strike system (above) loses you a contract but is invisible to the
  board — and whether tiers beyond the first want a shape other than one
  threshold per group. A delivery count is also a poor instrument for the
  *transient* cost of building a new branch; see §6 and issues #92/#93.
- When staff demand exceeds headcount in a category, which machines lose out — player-set priority, first-come-first-served, or proportional throttling across all of them? **MVP-provisional (2026-09-12, §9's staff-pools bullet):** first-come in the simulation's stable machine order, so the same machine is always the one short; player-set priority is the expected replacement and needs no wire change.
- Is there a one-time recruiting/training cost on top of the recurring per-head wage, and can headcount be reduced (firing) freely or with a penalty/delay? **MVP-provisional (2026-09-12), amended by the quality pass (2026-09-13) and again on 2026-09-26:** no recruiting fee; a hire the balance cannot cover the next bill plus the new wage for is WARNED and confirmable rather than refused (it was refused from 2026-09-13 — see §7 and `docs/UI_UX.md` §2b point 40); firing is free and immediate, but the fired head is still paid for the period it worked any part of (notice pay); a head that cannot be paid on payday is idled, not lost, and leaves only if the next payday cannot pay it either. Still placeholders for the balancing pass, not decisions.
- ~~Are perishability (spoilage) and corrosion (durability) modeled as full ECS components with their own tick-based decay, or as simpler flags/costs?~~ **Half-decided 2026-10-02: there is no spoilage.** The project owner dropped perishability from the game outright — no item, ingredient or fluid spoils, and mayo (tier 4) is gated by its emulsifier, not by refrigeration or a cold chain. Still open: how corrosion (durability) is modelled, which needs deciding before tier 5 implementation starts.
- Exact tier list beyond the six in §2 (this doc's tiers are illustrative of the *shape* of progression, not necessarily the final content list), and the actual catalog of derived sauces/variants — both are a game-design task, not an architecture or gameplay-systems one.
