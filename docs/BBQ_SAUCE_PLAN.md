# BBQ Sauce — the second sauce

> **SUPERSEDED — BBQ shipped between 2026-09-18 and 2026-09-19, and this file
> is now history.** Every ticket landed: T1 the sim capability (PR #45), T2 the
> content (#56), T3 the unlock gate (#66), T4 the assets (#39), and this doc's
> own ticket T5 (the docs, late — 2026-09-21). Read the authoritative docs, not
> the plan below, for what the game does now:
>
> | What the plan called | Where it lives now |
> |---|---|
> | §2 the chain, §2.1 the ratio | `data/recipes.ron`, and GAMEPLAY.md §3's derived-sauces section |
> | §3 the three sim changes | `docs/DECISIONS.md` 2026-09-21, two records (several recipes per kind; several fluid inputs). `validate_chain` has since been refined again — issue #42's record, not §3.3 |
> | §3.1 the save consequence | `SavedMachine::recipe`. Note the additive-field window it used has **closed**: the save schema is 2 since 2026-09-20 and a shape change is a bump now |
> | §4 the contracts | `data/contracts.ron` |
> | §5 the unlock | `data/unlocks.ron` and its comment, GAMEPLAY.md §9, `docs/DECISIONS.md` 2026-09-21 |
> | the balance of the gate | `docs/DECISIONS.md` 2026-09-20, "The bank at the moment of building…" — issue #55 is closed `NOT_PLANNED` and its successors are open as #92 and #93 |
>
> Two rulings made while the epic ran are **not** in the text below, and both
> are pinned by tests on `main`: per-box payout monotonicity is **per sauce**
> (BBQ pays more per box at every size because it costs more to make; mixed
> orders are excluded from the ladder outright), and "one line's throughput"
> means **one ketchup line**, from `reference::linear_prefix`, not the maximum
> over every recipe.
>
> The brief is kept verbatim for the record. Where it and a doc above
> disagree, the doc above is right.

The plan for the game's second sauce, agreed with the project owner on
2026-09-16. This doc is the **shared spec for the workers building it**:
they work in separate worktrees whose sessions share no memory, so every
number a ticket depends on is written down here rather than held in a
conversation.

It is a *plan*, not a rule. Once the work has landed, the durable parts of
it belong in [GAMEPLAY.md](GAMEPLAY.md) (the sauce tier table, §2/§3) and
[DECISIONS.md](DECISIONS.md) (the fluid-input rule change), and this file
becomes history like `docs/QUALITY_PASS_PLAN.md` before it.

---

## 1. Why a second sauce, and why this one

Every one of the nine contracts on the board today asks for
`boxed_ketchup`. GAMEPLAY.md §6's central claim — *contracts demand a mix,
so a diversified factory is the correct strategy* — has therefore never
once been exercised in play. **The job of the second sauce is to turn that
pillar on**, not to add content for its own sake.

BBQ is the right one because it needs no mechanic that GAMEPLAY.md §11
still lists as undecided. Mayo (tier 4) needs spoilage; hot sauce (tier 5)
needs corrosion. Both would mean settling a large open question before we
have shown that a second line is even fun. BBQ needs neither, and it is
tier 3 in §2's existing table.

It is also a **derived sauce** in GAMEPLAY.md §3's sense — it consumes
finished ketchup — which makes the sauce list a branching dependency tree
from the second entry onward rather than after some later refactor.

## 2. The chain

No new machine. Every stage is a second recipe on a kind that already
exists.

| # | Machine | In | Out | Ticks |
|---|---|---|---|---|
| 1 | `mixer` | `ketchup` x1 *(fluid)* + `spice_mix` x1 + `molasses` x1 *(solids)* | `bbq_mix` *(fluid)* | 90 |
| 2 | `cooking_vat` | `bbq_mix` x1 + `liquid_smoke` x1 *(both fluid)* | `bbq` *(fluid)* | **600** |
| 3 | `bottler` | `bbq` x1 *(fluid)* | `bottled_bbq` | 60 |
| 4 | `packer` | `bottled_bbq` x1 | `boxed_bbq` | 40 |

Three new raw ingredients: `spice_mix` (solid, 5), `molasses` (solid, 4),
`liquid_smoke` (fluid, 3).

At 600 ticks the BBQ cook is the longest process in the game by 2.5x. That
is the "low and slow" identity of a barbecue sauce bought with a number
instead of a new machine.

**BBQ taps finished ketchup off the pipe, before the bottler.** A fluid
network is a shared pool, so one ketchup run touching both a bottler and a
BBQ mixer feeds both and they contend for it. The ketchup you sell and the
ketchup you cook into BBQ come out of the same tank — every box of BBQ is a
box of ketchup you did not ship. That is GAMEPLAY.md §6's diversification
pressure with teeth: serving a mixed contract is not "run two lines", it is
"build enough ketchup for both".

### 2.1 The ratio, which is load-bearing

At a 600-tick cook a BBQ branch draws one ketchup per 600 ticks from a line
producing one per 240. The remainder bottles out as ketchup at one per 400.

> **One ketchup line plus one BBQ branch naturally produces 3 ketchup : 2
> BBQ.**

The first mixed contract (`grill_pub`, §4) is sized at exactly 3 + 2 for
that reason: a player who builds the obvious thing can serve it without
tuning anything, and every larger mixed contract is the same shape scaled
up, so the ratio teaches itself.

**If the cook time changes, this ratio and those contract mixes change with
it, or the claim above becomes false.** Change them together or not at all
— the same rule `data/recipes.ron` already states about the crush/blend/cook
ordering.

## 3. What this costs in the sim

Three changes, all of which the shipped ketchup chain never needed. They are
one ticket (§5, T1) because all three land in the same ~30 lines of
`World::new`.

1. **Multiple fluid inputs per recipe.** `validate_fluid_scope`
   (`world.rs`) asserts `fluid_inputs.len() <= 1`. The BBQ cook draws two
   networks, `bbq_mix` and `liquid_smoke`. Each required fluid is matched to
   a touching network by **fluid id**; deposits stay at exactly one, off the
   outlet face, unchanged.

   This is the honest cost of the owner's two choices (mixer outputs a
   liquid; liquid smoke goes in at the cook) and there is no layout that
   dodges it. It is also wanted by every later sauce — mayo needs oil plus
   acid, hot sauce needs vinegar plus mash — so it is better bent once, now,
   with a real two-fluid recipe to test it against.

2. **Multiple recipes per machine kind.** `world.rs`'s `recipe_for_kind`
   panics today on a second recipe naming a kind, deliberately, because
   there is no recipe-selection mechanic. There still should not be a
   *player-facing* one: **a placed machine runs whichever of its kind's
   recipes its current inputs satisfy**, with `recipes.ron` file order as
   the deterministic tiebreak. No UI, no new command, no player verb.

   In practice the plumbing already decides: a mixer on a vinegar pipe with
   a sugar hopper makes ketchup mix, a mixer on a ketchup pipe with spice
   and molasses hoppers makes BBQ mix. The pipe you run to a machine is what
   it makes.

3. **`validate_chain` relaxed from a line to a graph.** The file no longer
   describes one production line, so "recipe i+1 consumes something recipe i
   produces" is the wrong invariant. The replacement must still catch a
   typo'd id at construction time: every non-raw input of every recipe is
   produced by some recipe, and the branch roots remain reachable.

### 3.1 The save-format consequence

`SavedMachine` deliberately does **not** store the recipe of an in-flight
craft — `save.rs` says so in a comment, because "a placed machine runs the
one recipe its kind names, so saving it would be saving content". Once a
kind runs several recipes that stops being true: a machine halfway through a
BBQ cook would re-derive its recipe on load from inputs it has already
consumed.

So T1 must persist the in-flight recipe id. It can be added as a
`#[serde(default)]` field without bumping `SAVE_SCHEMA_VERSION`, exactly as
`SavedMachine::load` and `SavedHopper::auto_fill` already were — a save
written before it loads with no in-flight recipe, which is what a machine
with no in-flight craft reads as anyway.

**Because it touches the save format, T1 does not self-merge** (§6).

## 4. Contracts

**New contracts only; the existing ketchup ones keep their mixes.** Amending
a live contract to demand BBQ would silently break a factory that was
serving it fine, and losing a contract you had been paying for because the
game changed the terms reads as a bug rather than as difficulty.

All four appear together when BBQ unlocks (§5), placed after the ketchup
entries so the board's file order still reads as a progression.

| id | Name | Mix | Cadence | Payout | Price | Strikes |
|---|---|---|---|---|---|---|
| `smokehouse_trial` | Smokehouse Trial | 2 BBQ | 2500 | 120 | 200 | 3 |
| `grill_pub` | Grill Pub Chain | 3 ketchup + 2 BBQ | 3000 | 210 | 350 | 2 |
| `roadhouse_group` | Roadhouse Group | 6 ketchup + 4 BBQ | 4000 | 440 | 700 | 2 |
| `bbq_festival` | Barbecue Festival | 10 BBQ | 5000 | 650 | 1000 | 2 |

`smokehouse_trial` gets 3 strikes rather than 2 on purpose: it is the
tutorial for a line the player has never built.

BBQ pays ~60 per box against ketchup's 24-40. A box costs 23 in raws — 11 of
which is the ketchup it consumes — holding the roughly one-third ingredient
ratio `data/ingredients.ron` pins as its shape. Converting a line's output
to BBQ lifts revenue about 40% for one extra mixer, one extra vat and four
more heads: worth doing, not so good that the ketchup contracts die.

Largest single mix quantity stays 10, comfortably under
`DELIVERY_STOCK_CAPACITY` (48), so `tests/dock.rs`'s relationship holds
unchanged.

## 5. The unlock

The game gates nothing today; everything is buyable from a new game. BBQ
introduces the first progression gate, and it gates **two things only**:

- the three new ingredients appear in the shop, and
- the four new contracts appear on the board.

**No recipe is locked.** The BBQ recipes sit in `recipes.ron` from a new
game and are simply unbuildable, because the ingredients cannot be bought,
and unsellable, because nobody is asking. That keeps the gate out of the
simulation's hot path entirely — it is a filter on two lists, not a state a
machine has to consult every tick.

Keyed off **lifetime successful contract deliveries, threshold 10**. That is
roughly 90 seconds to three minutes depending on how many contracts have
been signed, so the ketchup line has been built and tuned before it fires.
The number is the balance dial if it plays wrong.

`data/unlocks.ron` (schema 1) names the group rather than putting an
`unlock_after` field on each entry, for one reason: the toast that announces
*"BBQ Sauce unlocked — new ingredients in the shop, new orders on the
board"* needs the group to have a name. That announcement is the whole
gameplay point of gating it.

## 6. Tickets

| # | Ticket | Depends on | Merge |
|---|---|---|---|
| T1 | Sim: multi-recipe kinds, multi-fluid inputs, graph chain validation | — | **owner review** (save format) |
| T2 | Content: the BBQ chain, ingredients and contracts in `data/` | T1 | self-merge if green |
| T3 | The unlock gate: `unlocks.ron`, delivery counter, shop/board filter, toast | T2 | **owner review** (save format) |
| T4 | Assets: BBQ bottle and box, label variants, item model mapping | — | self-merge if green |
| T5 | Docs: GAMEPLAY.md, DECISIONS.md, DEVLOG.md | T1-T4 | self-merge if green |

T1 and T4 start together; the rest follow their dependencies. The three sim
changes in T1 are deliberately **not** split across workers: they land in the
same few lines of `World::new`, and three worktrees editing that region
would spend more time in conflict resolution than in the work.

T1 and T3 both add a persisted field. Per `docs/WORKFLOW.md`, a PR that
changes the save format is not self-merged — both go to the owner with the
diff and the reasoning, however additive the change looks.

## 7. Deliberately not in this plan

- **A player-facing recipe picker.** Inputs select the recipe (§3.2). A
  machine-selection UI is a different game's affordance and nothing here
  needs it.
- **Amending live contracts on unlock** (§4).
- **Tier-locking the contract board.** BBQ contracts are gated by the
  unlock, and nothing else on the board is gated at all. A general
  contract-tier system is GAMEPLAY.md §11's open reputation question and is
  not being settled here.
- **A smoker, or any new machine kind.** The owner's constraint, and the
  600-tick cook buys the same identity without one.
