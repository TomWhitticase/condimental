//! `Machine` (ARCHITECTURE.md §3.2) and the generic `TimedProcess` pattern
//! (§3.5) it uses to run a recipe over many ticks.

use serde::{Deserialize, Serialize};

use super::fluid::FluidTap;
use super::inventory::Inventory;
use super::recipe::{Recipe, RecipeRegistry};
use super::staff::CrewRequest;
use super::stats::{ProductionLedger, StatKind};
use super::types::{Fx, RecipeId};

/// How many ticks one bucket of the [`LoadWindow`] accumulates before the
/// ring advances — half a second at Godot's 30 Hz tick rate. Integers
/// throughout (ARCHITECTURE.md §3.9): the window is tick counts, and the
/// percentage is integer arithmetic over them.
pub const LOAD_BUCKET_TICKS: u32 = 30;
/// How many buckets the ring holds: twenty half-seconds, a ten-second
/// window. Long enough that the one-tick gap `Machine::tick` leaves between
/// a deposit and the next start is invisible, short enough that a
/// bottleneck that has just moved shows up while the player is watching.
pub const LOAD_BUCKETS: usize = 20;
/// The whole window, in ticks — what `observed` climbs toward.
pub const LOAD_WINDOW_TICKS: u32 = LOAD_BUCKET_TICKS * LOAD_BUCKETS as u32;
/// A working share (in percent of the observed window) at or above this
/// reads as `working` — flat out. A machine fed and drained without
/// interruption reads exactly 100 (see [`WorkState`]'s counting rule), so
/// the margin below it is for a line that hiccups, not for the mechanism.
pub const FULL_LOAD_PERCENT: u32 = 95;

/// What a machine did with one tick — the one fact [`Machine::tick`] records
/// into its [`LoadWindow`] every time it runs, and the vocabulary of the
/// window's `status`. The counting rule, from `docs/MACHINE_LOAD_PLAN.md`:
///
/// | At entry to `tick`                  | Happened                       | Recorded  |
/// |-------------------------------------|--------------------------------|-----------|
/// | no process                          | a craft started                | `Working` |
/// | no process                          | nothing started                | `Starved` |
/// | process running, not complete       | timer advanced                 | `Working` |
/// | process complete                    | output deposited, cleared      | `Working` |
/// | process complete                    | held (output or pipe full)     | `Blocked` |
/// | would work, crew refused            | nothing (paused if mid-craft)  | `Starved` |
/// | mothballed                          | nothing (paused if mid-craft)  | `Starved` |
///
/// The fifth row is the staff pools MVP's: a machine waiting on people is
/// waiting on a resource, exactly like one waiting on sugar, so it records
/// `Starved` and the andon shows amber; what distinguishes it is the stall
/// reason (`Machine::refused_by`, the `staff` stall kind). No fourth state.
///
/// The last row is mothballing's (issue #19), and it takes the same answer
/// for the same reason: a machine switched off did no work this tick, which
/// is what this window measures, and *why* it did none is the stall's job to
/// say (`StallKind::Mothballed`). A fourth `Off` state would have to be
/// counted in [`LoadBucket`], which rides the save — a schema change to
/// record a fact the window already records correctly.
///
/// **The completion tick counts as work.** `Machine::tick` clears
/// `process` on the tick a batch lands and starts the next on the tick
/// after, so a saturated machine is briefly "idle" once per cycle; counting
/// the deposit as work is what makes a continuously fed and drained machine
/// read exactly 100 %, with no dip, and lets `FULL_LOAD_PERCENT` be a real
/// threshold rather than a fudge for the mechanism's own gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkState {
    Working,
    Starved,
    Blocked,
}

impl WorkState {
    /// The wire's word for this state — the string `MachineLoadSnapshot::status`
    /// carries, which Godot reads as-is and never re-derives from the counts.
    pub fn as_str(self) -> &'static str {
        match self {
            WorkState::Working => "working",
            WorkState::Starved => "starved",
            WorkState::Blocked => "blocked",
        }
    }
}

/// One bucket of the ring: how many of its ticks went to each state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadBucket {
    pub working: u32,
    pub starved: u32,
    pub blocked: u32,
}

impl LoadBucket {
    fn total(&self) -> u32 {
        self.working + self.starved + self.blocked
    }
}

/// A rolling record of what a machine did over the last
/// [`LOAD_WINDOW_TICKS`] ticks: a ring of [`LOAD_BUCKETS`] buckets, each
/// holding [`LOAD_BUCKET_TICKS`] ticks' worth of [`WorkState`] counts. The
/// current bucket fills until it holds a bucket's worth, then the cursor
/// advances and the bucket it lands on is cleared — so totals always cover
/// the last ten seconds and the partial current one, never more.
///
/// It exists because an instantaneous "is it working" flag flickers (see
/// [`WorkState`]) and because the player's question is *"is this machine the
/// bottleneck"*, which is a question about a stretch of time. The sim owns
/// it rather than Godot (ARCHITECTURE.md §1's hard rule): it is
/// deterministic, it survives a save (`save.rs`'s `SavedMachine`), and it is
/// tested headlessly here. Nothing about it dirties a chunk — it changes
/// every tick on every machine, so it is a polled readout
/// (`World::machine_load_snapshots`), like fluid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadWindow {
    buckets: [LoadBucket; LOAD_BUCKETS],
    cursor: usize,
}

impl Default for LoadWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadWindow {
    pub fn new() -> Self {
        Self {
            buckets: [LoadBucket::default(); LOAD_BUCKETS],
            cursor: 0,
        }
    }

    /// Records one tick's state. Called exactly once per `Machine::tick`.
    pub fn record(&mut self, state: WorkState) {
        if self.buckets[self.cursor].total() >= LOAD_BUCKET_TICKS {
            self.cursor = (self.cursor + 1) % LOAD_BUCKETS;
            self.buckets[self.cursor] = LoadBucket::default();
        }
        let bucket = &mut self.buckets[self.cursor];
        match state {
            WorkState::Working => bucket.working += 1,
            WorkState::Starved => bucket.starved += 1,
            WorkState::Blocked => bucket.blocked += 1,
        }
    }

    pub fn working(&self) -> u32 {
        self.buckets.iter().map(|b| b.working).sum()
    }

    pub fn starved(&self) -> u32 {
        self.buckets.iter().map(|b| b.starved).sum()
    }

    pub fn blocked(&self) -> u32 {
        self.buckets.iter().map(|b| b.blocked).sum()
    }

    /// Ticks the window has seen: `working + starved + blocked`, at most
    /// [`LOAD_WINDOW_TICKS`].
    pub fn observed(&self) -> u32 {
        self.buckets.iter().map(LoadBucket::total).sum()
    }

    /// The working share of the observed window, in whole percent; 0 when
    /// nothing has been observed.
    pub fn percent(&self) -> u32 {
        (self.working() * 100)
            .checked_div(self.observed())
            .unwrap_or(0)
    }

    /// The window's verdict: `Starved` before anything has happened;
    /// `Working` when the working share is at least [`FULL_LOAD_PERCENT`];
    /// otherwise whichever of starved and blocked took more ticks, starved
    /// on a tie.
    pub fn status(&self) -> WorkState {
        let observed = self.observed();
        if observed == 0 {
            return WorkState::Starved;
        }
        if self.working() * 100 >= FULL_LOAD_PERCENT * observed {
            return WorkState::Working;
        }
        if self.starved() >= self.blocked() {
            WorkState::Starved
        } else {
            WorkState::Blocked
        }
    }

    /// The raw ring, oldest-to-newest is *not* the order — it is the ring's
    /// storage order with [`LoadWindow::cursor`] marking the current bucket.
    /// For `save.rs`, which stores the ring as-is so a loaded window is
    /// bit-identical to the saved one.
    pub fn buckets(&self) -> &[LoadBucket; LOAD_BUCKETS] {
        &self.buckets
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Rebuilds a window from a saved ring. `None` if the ring is not
    /// exactly [`LOAD_BUCKETS`] long, the cursor is outside it, or any
    /// bucket holds more than [`LOAD_BUCKET_TICKS`] ticks — none of which
    /// `buckets()`/`cursor()` can produce, so all of which mean a corrupt
    /// or hand-edited save. Restore-only: the tick path records, it never
    /// writes the ring wholesale.
    pub(crate) fn restore(buckets: &[LoadBucket], cursor: usize) -> Option<Self> {
        let buckets: [LoadBucket; LOAD_BUCKETS] = buckets.try_into().ok()?;
        if cursor >= LOAD_BUCKETS || buckets.iter().any(|b| b.total() > LOAD_BUCKET_TICKS) {
            return None;
        }
        Some(Self { buckets, cursor })
    }
}

/// A tick-counted long-running process (ARCHITECTURE.md §3.5). Progress is
/// always a function of `(current_tick - start_tick)` vs `required_ticks` —
/// never wall-clock time, so it stays deterministic under fast-forward,
/// pause, and (eventually) lockstep replay.
#[derive(Debug, Clone, Copy)]
pub struct TimedProcess {
    pub start_tick: u64,
    pub required_ticks: u64,
}

impl TimedProcess {
    pub fn is_complete(&self, current_tick: u64) -> bool {
        current_tick.saturating_sub(self.start_tick) >= self.required_ticks
    }
}

#[derive(Debug)]
pub struct Machine {
    /// The recipe this machine is running, or — when it is idle — the one
    /// it would run next.
    ///
    /// A machine's *kind* used to name exactly one recipe, so this was
    /// content wearing a field's clothes: `world.rs` looked it up once at
    /// placement and it never changed again, which is why `save.rs`
    /// deliberately did not store it. Since the BBQ epic a kind may name
    /// several ([`RecipeRegistry::for_machine`]) and this is live state:
    /// [`Machine::tick`] re-picks it on every idle tick from what the
    /// buffers and the touching pipes actually hold, and **freezes it for
    /// the length of a craft** — a batch that started as BBQ finishes as
    /// BBQ even if its inputs are gone, which is exactly why it is now
    /// saved (`save.rs`'s `SavedMachine::recipe`).
    pub recipe: RecipeId,
    pub input: Inventory,
    pub output: Inventory,
    pub process: Option<TimedProcess>,
    /// What this machine did with each of the last [`LOAD_WINDOW_TICKS`]
    /// ticks — the bottleneck readout. Written once per [`Machine::tick`],
    /// read by `World::machine_load_snapshots`; never dirties a chunk.
    pub load: LoadWindow,
    /// The staff category that refused this machine its crew on the last
    /// tick, if the last tick was such a refusal (the staff pools MVP
    /// epic). Cleared by every tick that is not one, so it is a per-tick
    /// fact like a stall reason — `World::machine_stall` reports it as the
    /// `staff` stall kind, after the six older kinds have found nothing.
    /// Saved (`save.rs`), since it shows in snapshots.
    pub refused_by: Option<String>,
    /// **Switched off deliberately by the player** (`PlayerCommand::
    /// SetMothballed`, issue #19's escape hatch from the unpaid spiral).
    /// A mothballed machine asks for no crew, so it drops out of the
    /// ESTABLISHMENT (`World::recompute_establishment`) and the player can
    /// shrink their payroll to fit; it also starts nothing, advances
    /// nothing and deposits nothing.
    ///
    /// Live state the player owns, so it is saved (`save.rs`'s
    /// `SavedMachine::mothballed`) — a factory reopened from a save must
    /// still be off where the player switched it off, or the load is a
    /// hidden hiring bill.
    pub mothballed: bool,
}

impl Machine {
    pub fn new(recipe: RecipeId, input_capacity: u32, output_capacity: u32) -> Self {
        Self {
            recipe,
            input: Inventory::new(input_capacity),
            output: Inventory::new(output_capacity),
            process: None,
            load: LoadWindow::new(),
            refused_by: None,
            mothballed: false,
        }
    }

    /// Advances the machine by one tick: starts a new process if it's idle
    /// and has enough input, or deposits output once the timer's elapsed.
    /// Returns whether anything changed (dirty-chunk tracking, §4) — a
    /// process merely ticking toward completion is *not* a change, since
    /// Godot can derive progress itself from `start_tick`/`required_ticks`
    /// plus the current tick count, without a new snapshot every tick.
    ///
    /// **Which recipe is decided here, never by the player.** A machine
    /// mid-craft is running the recipe in [`Machine::recipe`] and nothing
    /// re-picks it. An idle one re-picks every tick: the first of its
    /// kind's recipes (`data/recipes.ron` order) whose solid inputs are in
    /// the buffer *and* whose every fluid input is carried by one of the
    /// networks in `fluid_in`. When none is satisfiable it holds the one it
    /// is closest to running, so the stall readout names what the machine
    /// is actually waiting for rather than an arbitrary first entry. A kind
    /// with one recipe — every kind before the BBQ epic — therefore behaves
    /// exactly as it always did: there is nothing to pick.
    ///
    /// `fluid_in` is this machine's taps into the pipe networks it *draws*
    /// from (milestone 6 — ARCHITECTURE.md §9 item 6/§3.4; a [`FluidTap`]
    /// is the network plus the one segment this machine touches, since the
    /// "Real Placement" epic made which segment that is a placement
    /// question). `world.rs` hands over **every** network touching the
    /// footprint that is not the outlet one, in tile order, and a recipe's
    /// fluid inputs are matched against them **by fluid id** — so a cooking
    /// vat standing between a mix pipe and a liquid-smoke pipe draws one
    /// unit from each without either the content or the player having to
    /// say which run is which. Each tap supplies at most one input.
    ///
    /// A recipe with `fluid_inputs` treats them exactly the way it treats
    /// its solid `inputs`: enough of the right fluid must be available
    /// *before* a craft starts, and starting consumes it. A machine that
    /// can't get its fluid simply sits idle — it never runs degraded, never
    /// crafts a partial batch, and never half-consumes its solid input; the
    /// same "stalls, doesn't run degraded" rule power and staff will use
    /// later (GAMEPLAY.md §7). An empty `fluid_in` for a recipe that wants
    /// fluid is therefore a permanent stall, which is the correct reading of
    /// "this machine has no pipe connected."
    ///
    /// `fluid_out` is the **second** tap, added by the chain-rework epic: the
    /// network this machine *deposits* into, for a recipe with
    /// `fluid_outputs`. It is a separate argument rather than a second use
    /// of the same tap because it is a genuinely different network — a
    /// machine that both consumes and produces liquid stands between two
    /// unconnected runs, which is exactly the shipped cooking vat. `world.rs`
    /// guarantees every tap here names a **distinct** network, so holding
    /// all of these mutable borrows at once is sound.
    ///
    /// **A fluid output is gated at completion, not at start**, symmetrically
    /// with the solid output buffer it sits beside: a finished batch that has
    /// nowhere to go is *held*, and retried next tick, rather than being
    /// dropped or spilled. So a full pipe backs a machine up exactly the way
    /// a full output buffer does, which is the same back-pressure rule the
    /// belt follows.
    ///
    /// Every call records exactly one [`WorkState`] into `self.load`, by
    /// the table on that type — including the completion tick, which counts
    /// as work. The load is a readout, not a change: it never contributes
    /// to the returned `changed` flag.
    ///
    /// `crew` is the staff pools MVP's addition: the kind's crew requirement
    /// and the pools to claim it from ([`CrewRequest`]), or `None` for a
    /// machine that asks for nobody. **The claim is made only on a tick the
    /// machine would otherwise have worked** — a craft would start, the
    /// timer would advance, or a finished batch would deposit — so a
    /// machine with no inputs or a full output asks for nothing and counts
    /// toward no category's demand. A refused machine takes nothing, does
    /// nothing, records `Starved` and remembers the category in
    /// `refused_by`. A refused machine **mid-craft pauses where it is**: its
    /// `start_tick` moves one tick later so `elapsed` stands still, which
    /// keeps `TimedProcess`'s "progress is `current_tick - start_tick`"
    /// rule true and is why a pause is reported as a change (Godot derives
    /// the progress bar from `start_tick`, so a paused machine's chunk is
    /// re-read every tick it is paused — the same rate an auto-filled hopper
    /// already dirties at). Progress is never lost.
    ///
    /// `ledger` is the world's production ledger (the production-stats
    /// epic, `stats.rs`): a craft that starts records its solid and fluid
    /// inputs as consumed, a batch that lands records its outputs as
    /// produced, at the two points that already move them — so the ledger
    /// can never disagree with the inventories. A batch held on a full
    /// output records nothing until it lands, and a crew refusal records
    /// nothing at all.
    pub fn tick(
        &mut self,
        current_tick: u64,
        recipes: &RecipeRegistry,
        mut fluid_in: Vec<FluidTap<'_>>,
        fluid_out: Option<FluidTap<'_>>,
        crew: Option<CrewRequest<'_>>,
        ledger: &mut ProductionLedger,
    ) -> bool {
        let mut changed = false;
        self.refused_by = None;
        if self.mothballed {
            // Switched off: no crew is asked for (which is the whole point —
            // `recompute_establishment` has already dropped this machine's
            // crew from the establishment), no buffer is touched and the
            // ledger records nothing. A craft that was running is PAUSED
            // exactly the way a crew refusal pauses one — `elapsed` stands
            // still, progress is never lost — so returning a machine to
            // service resumes the batch it was in the middle of.
            if let Some(process) = self.process.as_mut() {
                // This is the one place a process advancing on its own DOES
                // dirty the chunk, against the rule in this method's doc, and
                // it has to: Godot derives progress from `start_tick` against
                // the live tick count, so a paused batch whose `start_tick`
                // moved silently would keep filling its bar on screen while
                // the sim stood still. The cost is one chunk re-read per tick
                // per mothballed machine that is holding a part-finished
                // batch — bounded by how many the player switched off
                // mid-craft, and it stops the moment that batch is resumed
                // and finished or the machine is emptied.
                process.start_tick += 1;
                changed = true;
            }
            self.load.record(WorkState::Starved);
            return changed;
        }
        let Some(running) = recipes.get(&self.recipe) else {
            // A machine with no recipe can never start, which is a
            // permanent starve; `World` refuses to place one, so this is
            // for completeness of the "exactly one state per tick" rule.
            self.load.record(WorkState::Starved);
            return false;
        };
        // An in-flight craft owns the recipe for its whole length; only an
        // idle machine re-reads its inputs to pick one (see this method's
        // doc). `satisfied` is the same question `Plan::Start` needs asked,
        // asked once.
        let (recipe, satisfied) = match self.process {
            Some(_) => (running, false),
            None => {
                let (chosen, satisfied) = self
                    .choose(recipes, &running.machine, &fluid_in)
                    .unwrap_or((running, false));
                if chosen.id != self.recipe {
                    self.recipe = chosen.id.clone();
                    changed = true;
                }
                (chosen, satisfied)
            }
        };
        // Starved until something below says otherwise: a tick that starts
        // nothing and completes nothing is a tick spent waiting for input.
        let mut state = WorkState::Starved;

        // What this tick WOULD do, decided before anything is touched, so
        // the crew is asked for exactly when the answer is "work", and a
        // refusal can leave every buffer and tap as it found them.
        enum Plan {
            Start,
            Advance,
            Deposit,
            Blocked,
            Idle,
        }
        let plan = match self.process {
            // Solids and fluids were both checked before anything was
            // consumed, so a shortage can never leave the machine having
            // eaten half its input.
            None if satisfied => Plan::Start,
            None => Plan::Idle,
            Some(process) if !process.is_complete(current_tick) => Plan::Advance,
            Some(_) => {
                let has_room = recipe
                    .outputs
                    .iter()
                    .all(|(item, qty)| self.output.has_room_for(item, *qty));
                // The fluid half of the same question. An empty
                // `fluid_outputs` is vacuously satisfiable, so an ordinary
                // solid-only stage is unaffected by any of this; a recipe
                // that wants to deposit and has no tap at all can never
                // finish, which is the correct reading of "this machine has
                // no pipe to discharge into."
                let has_fluid_room = match &fluid_out {
                    Some(tap) => recipe
                        .fluid_outputs
                        .iter()
                        .all(|(id, units)| tap.can_accept(id, Fx::from_num(*units))),
                    None => recipe.fluid_outputs.is_empty(),
                };
                if has_room && has_fluid_room {
                    Plan::Deposit
                } else {
                    Plan::Blocked
                }
            }
        };

        // The crew is asked for only when the plan is to work.
        let refused = match (&plan, crew) {
            (Plan::Start | Plan::Advance | Plan::Deposit, Some(request)) => {
                request.pools.claim(request.crew).err()
            }
            _ => None,
        };
        if let Some(category) = refused {
            self.refused_by = Some(category);
            if let Some(process) = self.process.as_mut() {
                // Paused, not lost: elapsed stands still.
                process.start_tick += 1;
                changed = true;
            }
            self.load.record(WorkState::Starved);
            return changed;
        }

        match plan {
            Plan::Start => {
                for (item, qty) in &recipe.inputs {
                    self.input.remove(item, *qty);
                    ledger.record_consumed(item, StatKind::Item, *qty);
                }
                // The same id-matching `satisfaction` just did, run again to
                // draw rather than to count — a craft starts at most once
                // every `time_ticks`, so re-matching there is cheaper than
                // carrying an allocation through every idle tick.
                let mut claimed = 0u64;
                for (id, units) in &recipe.fluid_inputs {
                    let amount = Fx::from_num(*units);
                    let Some(index) = Self::supplier(&fluid_in, claimed, id, amount) else {
                        debug_assert!(
                            false,
                            "every fluid input was matched to a tap immediately above, so this \
                             lookup cannot fail without something having mutated the topology"
                        );
                        continue;
                    };
                    claimed |= 1 << index;
                    ledger.record_consumed(id, StatKind::Fluid, *units);
                    let drawn = fluid_in[index].draw(id, amount);
                    debug_assert!(
                        drawn,
                        "fluid availability was checked immediately above, so this draw \
                         cannot fail without something having mutated the network in between"
                    );
                }
                self.process = Some(TimedProcess {
                    start_tick: current_tick,
                    required_ticks: recipe.time_ticks,
                });
                changed = true;
                // A craft started this very tick: the mechanism is turning.
                state = WorkState::Working;
            }
            Plan::Advance => {
                // The timer advanced: the mechanism is turning.
                state = WorkState::Working;
            }
            Plan::Deposit => {
                for (item, qty) in &recipe.outputs {
                    self.output.add(item, *qty);
                    ledger.record_produced(item, StatKind::Item, *qty);
                }
                if let Some(mut tap) = fluid_out {
                    for (id, units) in &recipe.fluid_outputs {
                        ledger.record_produced(id, StatKind::Fluid, *units);
                        let deposited = tap.deposit(id, Fx::from_num(*units));
                        debug_assert!(
                            deposited,
                            "room was checked immediately above, so this deposit cannot \
                             fail without something having mutated the network in between"
                        );
                    }
                }
                self.process = None;
                changed = true;
                state = WorkState::Working;
            }
            Plan::Blocked => {
                // Finished but blocked on a full output buffer or a full
                // pipe — retried next tick, same back-pressure idea as the
                // belt.
                state = WorkState::Blocked;
            }
            Plan::Idle => {}
        }

        self.load.record(state);
        changed
    }

    /// Which of `kind`'s recipes an idle machine should hold, and whether
    /// its inputs are satisfied right now.
    ///
    /// Two rules, in this order:
    ///
    /// 1. **The first fully-satisfied recipe wins**, so `data/recipes.ron`'s
    ///    own order is the tiebreak when the buffers and the pipes satisfy
    ///    more than one. That is the only role the file's order plays in
    ///    recipe selection, and it is why a mixer standing on a vinegar pipe
    ///    beside a sugar hopper blends ketchup mix while one on a ketchup
    ///    pipe beside spice and molasses hoppers blends BBQ mix: the two
    ///    never both match.
    /// 2. When none is satisfied, **the one closest to being satisfied**,
    ///    counting requirements met. Nothing in the simulation turns on
    ///    this — an unsatisfied machine does not move — but the snapshot and
    ///    the stall readout are read off [`Machine::recipe`], and "waiting
    ///    for molasses" is a true sentence about a BBQ mixer where "waiting
    ///    for vinegar" is not.
    /// 3. **A tie goes to the recipe the machine is already holding**, and
    ///    only then to the earliest in the file. This is rule 2's own
    ///    argument applied to the case rule 2 got wrong: a bottler that has
    ///    just filled twenty bottles of BBQ and drained its tap ties every
    ///    candidate at zero, and taking the earliest handed it
    ///    `bottled_ketchup` — so it reported waiting for ketchup it had
    ///    never touched, and (since 2026-09-21, when a machine started
    ///    drawing the sauce its recipe names) stood a row of red bottles in
    ///    a BBQ line. A machine that has never run holds its kind's first
    ///    recipe and keeps on holding it, so a fresh factory is unchanged.
    ///
    /// `None` only for a kind no recipe names, which `World` refuses to
    /// place.
    fn choose<'r>(
        &self,
        recipes: &'r RecipeRegistry,
        kind: &str,
        fluid_in: &[FluidTap<'_>],
    ) -> Option<(&'r Recipe, bool)> {
        let mut closest: Option<(u32, &'r Recipe)> = None;
        for candidate in recipes.for_machine(kind) {
            let (met, wanted) = self.satisfaction(candidate, fluid_in);
            if met == wanted {
                return Some((candidate, true));
            }
            // Rule 3's tie-break: `met > best` alone leaves the earliest
            // candidate holding a tie, so the held recipe has to be able to
            // take it back off one that got there first.
            if closest
                .is_none_or(|(best, _)| met > best || (met == best && candidate.id == self.recipe))
            {
                closest = Some((met, candidate));
            }
        }
        closest.map(|(_, recipe)| (recipe, false))
    }

    /// How many of `recipe`'s inputs this machine could satisfy this tick,
    /// out of how many it asks for — `met == wanted` is exactly the
    /// condition for starting a craft.
    ///
    /// Solids are counted against the input buffer. Each fluid is matched
    /// **by id** against the networks in `fluid_in`, each of which may
    /// supply at most one input; the match is greedy in recipe order, which
    /// is enough because a recipe never names the same fluid twice
    /// (`world.rs`'s `validate_fluid_scope`) and two touching runs carrying
    /// the same fluid are interchangeable.
    fn satisfaction(&self, recipe: &Recipe, fluid_in: &[FluidTap<'_>]) -> (u32, u32) {
        let mut met = 0;
        for (item, qty) in &recipe.inputs {
            if self.input.has_at_least(item, *qty) {
                met += 1;
            }
        }
        // A bitmask rather than a `Vec<bool>`: this runs for every candidate
        // recipe of every idle machine every tick, and a machine touches far
        // fewer than 64 pipe runs.
        let mut claimed = 0u64;
        for (id, units) in &recipe.fluid_inputs {
            if let Some(index) = Self::supplier(fluid_in, claimed, id, Fx::from_num(*units)) {
                claimed |= 1 << index;
                met += 1;
            }
        }
        let wanted = (recipe.inputs.len() + recipe.fluid_inputs.len()) as u32;
        (met, wanted)
    }

    /// The first tap in `fluid_in` not already `claimed` that carries
    /// `fluid` and can supply `amount` of it.
    fn supplier(fluid_in: &[FluidTap<'_>], claimed: u64, fluid: &str, amount: Fx) -> Option<usize> {
        fluid_in.iter().enumerate().position(|(index, tap)| {
            index < 64 && claimed & (1 << index) == 0 && tap.can_supply(fluid, amount)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::fluid::{FluidNetwork, FluidTank, PipeSegment};
    use crate::sim::types::Direction;

    fn registry() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(
            r#"(
                schema_version: 3,
                recipes: [
                    Recipe(
                        id: "tomato_sauce",
                        inputs: [("tomato", 4)],
                        outputs: [("tomato_sauce", 1)],
                        machine: "cooker",
                        time_ticks: 10,
                    ),
                ],
            )"#,
        )
        .unwrap()
    }

    #[test]
    fn stays_idle_without_enough_input() {
        let mut ledger = ProductionLedger::new();
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        machine.input.add("tomato", 2);
        for tick in 0..20 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }
        assert!(machine.process.is_none());
        assert_eq!(machine.output.count("tomato_sauce"), 0);
    }

    #[test]
    fn runs_a_full_craft_cycle() {
        let mut ledger = ProductionLedger::new();
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes, vec![], None, None, &mut ledger); // consumes input, starts process
        assert_eq!(machine.input.count("tomato"), 0);
        assert!(machine.process.is_some());

        for tick in 1..10 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
            assert_eq!(
                machine.output.count("tomato_sauce"),
                0,
                "must not finish early"
            );
        }

        machine.tick(10, &recipes, vec![], None, None, &mut ledger); // required_ticks elapsed
        assert_eq!(machine.output.count("tomato_sauce"), 1);
        assert!(machine.process.is_none());
    }

    #[test]
    fn holds_finished_output_when_buffer_is_full() {
        let mut ledger = ProductionLedger::new();
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 1);
        machine.output.add("tomato_sauce", 1); // pre-fill the only slot
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes, vec![], None, None, &mut ledger);
        for tick in 1..=10 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }
        // Blocked: process completed but couldn't deposit, so it must
        // still be holding, not silently dropping the crafted item.
        assert!(machine.process.is_some());
        assert_eq!(machine.output.count("tomato_sauce"), 1);

        machine.output.remove("tomato_sauce", 1);
        machine.tick(11, &recipes, vec![], None, None, &mut ledger);
        assert_eq!(machine.output.count("tomato_sauce"), 1);
        assert!(machine.process.is_none());
    }

    /// A recipe that wants a fluid, for the fluid-gating tests below.
    fn fluid_registry() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(
            r#"(
                schema_version: 3,
                recipes: [
                    Recipe(
                        id: "ketchup",
                        inputs: [("tomato_sauce", 1)],
                        fluid_inputs: [("vinegar", 2)],
                        outputs: [("ketchup", 1)],
                        machine: "reducer",
                        time_ticks: 10,
                    ),
                ],
            )"#,
        )
        .unwrap()
    }

    /// A network whose tank has been filled, so the run carries fluid once
    /// it has been ticked a few times.
    /// A two-tile run with a full tank already discharged into it, and the
    /// machine tapping the far tile — the shape `World` derives for a
    /// fluid-fed machine.
    fn supplied_network() -> (FluidNetwork, FluidTank) {
        let cap = Fx::from_num(10);
        let mut network = FluidNetwork::new(
            Some("vinegar".to_string()),
            vec![
                PipeSegment::new((0, 0), Direction::East, cap),
                PipeSegment::new((1, 0), Direction::East, cap),
            ],
            vec![(0, 1)],
        );
        let mut tank = FluidTank::new(Fx::from_num(100));
        tank.fill(100);
        for _ in 0..10 {
            network.discharge_from_tank(&mut tank, 0);
            network.tick();
        }
        (network, tank)
    }

    /// A network whose tank was never filled. Since the "Visual overhaul
    /// round 2" epic's ticket 1 removed the self-refilling source segment,
    /// this stays dry forever no matter how long it is ticked — which is
    /// what makes it a fluid-starvation fixture.
    fn empty_network() -> FluidNetwork {
        let cap = Fx::from_num(10);
        FluidNetwork::new(
            Some("vinegar".to_string()),
            vec![PipeSegment::new((0, 0), Direction::East, cap)],
            vec![],
        )
    }

    fn tap(network: &mut FluidNetwork, inlet: usize) -> FluidTap<'_> {
        FluidTap { network, inlet }
    }

    #[test]
    fn never_crafts_with_plenty_of_solid_input_but_no_fluid() {
        let mut ledger = ProductionLedger::new();
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 10);
        let mut network = empty_network(); // unfilled tank: stays empty forever

        for tick in 0..500 {
            network.tick();
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut network, 0)],
                None,
                None,
                &mut ledger,
            );
        }

        assert!(
            machine.process.is_none(),
            "must never start a craft without its fluid"
        );
        assert_eq!(machine.output.count("ketchup"), 0);
        assert_eq!(
            machine.input.count("tomato_sauce"),
            10,
            "a fluid shortage must not consume the solid input either"
        );
    }

    #[test]
    fn never_crafts_when_no_network_is_connected_at_all() {
        let mut ledger = ProductionLedger::new();
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 10);

        for tick in 0..500 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }

        assert!(machine.process.is_none());
        assert_eq!(machine.output.count("ketchup"), 0);
    }

    #[test]
    fn crafts_and_consumes_fluid_once_the_network_can_supply_it() {
        let mut ledger = ProductionLedger::new();
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 1);
        let (mut network, mut tank) = supplied_network();
        let before = network.segments()[1].volume;

        machine.tick(
            0,
            &recipes,
            vec![tap(&mut network, 1)],
            None,
            None,
            &mut ledger,
        );
        assert!(
            machine.process.is_some(),
            "craft must start once fluid is available"
        );
        assert_eq!(
            network.segments()[1].volume,
            before - Fx::from_num(2),
            "starting a craft must consume exactly the recipe's fluid quantity"
        );

        for tick in 1..=10 {
            network.discharge_from_tank(&mut tank, 0);
            network.tick();
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut network, 1)],
                None,
                None,
                &mut ledger,
            );
        }
        assert_eq!(machine.output.count("ketchup"), 1);
    }

    /// A recipe whose entire output is a fluid and whose entire input is a
    /// fluid — the shipped cooking vat's shape, and the one that needs two
    /// separate networks at once.
    fn producer_registry() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(
            r#"(
                schema_version: 3,
                recipes: [
                    Recipe(
                        id: "ketchup",
                        inputs: [],
                        fluid_inputs: [("ketchup_mix", 1)],
                        outputs: [],
                        fluid_outputs: [("ketchup", 1)],
                        machine: "cooking_vat",
                        time_ticks: 5,
                    ),
                ],
            )"#,
        )
        .unwrap()
    }

    /// One tile of pipe carrying `fluid`, already holding `volume`.
    fn one_tile_network(fluid: &str, capacity: i32, volume: i32) -> FluidNetwork {
        let mut network = FluidNetwork::new(
            Some(fluid.to_string()),
            vec![PipeSegment::new(
                (0, 0),
                Direction::East,
                Fx::from_num(capacity),
            )],
            vec![],
        );
        network.deposit_at(0, fluid, Fx::from_num(volume));
        network
    }

    #[test]
    fn deposits_its_fluid_output_into_the_outlet_network_on_completion() {
        let mut ledger = ProductionLedger::new();
        let recipes = producer_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        let mut inflow = one_tile_network("ketchup_mix", 10, 4);
        let mut outflow = one_tile_network("ketchup", 10, 0);

        machine.tick(
            0,
            &recipes,
            vec![tap(&mut inflow, 0)],
            Some(tap(&mut outflow, 0)),
            None,
            &mut ledger,
        );
        assert!(
            machine.process.is_some(),
            "a fluid-only input must start a craft"
        );
        assert_eq!(
            inflow.segments()[0].volume,
            Fx::from_num(3),
            "starting consumes exactly the recipe's fluid input"
        );
        assert_eq!(
            outflow.segments()[0].volume,
            Fx::ZERO,
            "nothing is deposited until the process completes"
        );

        for tick in 1..=5 {
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut inflow, 0)],
                Some(tap(&mut outflow, 0)),
                None,
                &mut ledger,
            );
        }
        assert!(machine.process.is_none());
        assert_eq!(outflow.segments()[0].volume, Fx::from_num(1));
    }

    /// The producer-side mirror of `holds_finished_output_when_buffer_is_full`:
    /// a full pipe must back the machine up, not spill.
    #[test]
    fn holds_a_finished_batch_when_the_outlet_pipe_is_full() {
        let mut ledger = ProductionLedger::new();
        let recipes = producer_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        let mut inflow = one_tile_network("ketchup_mix", 10, 4);
        let mut outflow = one_tile_network("ketchup", 10, 10); // brim full

        for tick in 0..=5 {
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut inflow, 0)],
                Some(tap(&mut outflow, 0)),
                None,
                &mut ledger,
            );
        }
        assert!(
            machine.process.is_some(),
            "the batch is finished but has nowhere to go, so it is still held"
        );
        assert_eq!(
            outflow.segments()[0].volume,
            Fx::from_num(10),
            "nothing spilled"
        );

        // Make room, and the very next tick the held batch lands.
        outflow.draw_at(0, "ketchup", Fx::from_num(2));
        machine.tick(
            6,
            &recipes,
            vec![tap(&mut inflow, 0)],
            Some(tap(&mut outflow, 0)),
            None,
            &mut ledger,
        );
        assert!(machine.process.is_none());
        assert_eq!(outflow.segments()[0].volume, Fx::from_num(9));
    }

    #[test]
    fn never_completes_a_fluid_producing_craft_with_no_outlet_network_at_all() {
        let mut ledger = ProductionLedger::new();
        let recipes = producer_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        let mut inflow = one_tile_network("ketchup_mix", 10, 10);

        for tick in 0..500 {
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut inflow, 0)],
                None,
                None,
                &mut ledger,
            );
        }
        assert!(
            machine.process.is_some(),
            "an unplumbed outlet is a permanent hold, never a silent discard"
        );
        assert_eq!(
            inflow.segments()[0].volume,
            Fx::from_num(9),
            "exactly one craft's worth was drawn, and the machine has been stuck since"
        );
    }

    #[test]
    fn refuses_to_deposit_into_a_network_carrying_the_wrong_fluid() {
        let mut ledger = ProductionLedger::new();
        let recipes = producer_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        let mut inflow = one_tile_network("ketchup_mix", 10, 10);
        let mut outflow = one_tile_network("water", 10, 0);

        for tick in 0..500 {
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut inflow, 0)],
                Some(tap(&mut outflow, 0)),
                None,
                &mut ledger,
            );
        }
        assert!(machine.process.is_some());
        assert_eq!(outflow.segments()[0].volume, Fx::ZERO);
    }

    #[test]
    fn refuses_a_network_carrying_the_wrong_fluid() {
        let mut ledger = ProductionLedger::new();
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 10);
        let (mut network, mut tank) = supplied_network();
        network.fluid = Some("water".to_string());

        for tick in 0..500 {
            network.discharge_from_tank(&mut tank, 0);
            network.tick();
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut network, 1)],
                None,
                None,
                &mut ledger,
            );
        }

        assert!(
            machine.process.is_none(),
            "vinegar recipe must not run on a water pipe"
        );
    }

    // ---- the load window (machine-load epic) -------------------------------

    #[test]
    fn load_ring_wraps_and_clears_so_observed_never_exceeds_the_window() {
        let mut window = LoadWindow::new();
        assert_eq!(window.observed(), 0);
        assert_eq!(window.percent(), 0);
        assert_eq!(
            window.status(),
            WorkState::Starved,
            "nothing observed reads starved"
        );

        for _ in 0..LOAD_WINDOW_TICKS * 3 {
            window.record(WorkState::Working);
        }
        assert_eq!(
            window.observed(),
            LOAD_WINDOW_TICKS,
            "three windows' worth still fit in one"
        );
        assert_eq!(window.working(), LOAD_WINDOW_TICKS);
        assert_eq!(window.percent(), 100);
        assert_eq!(window.status(), WorkState::Working);

        // One full window of starving pushes every working tick out.
        for _ in 0..LOAD_WINDOW_TICKS {
            window.record(WorkState::Starved);
        }
        assert_eq!(
            window.working(),
            0,
            "the ring forgets what is more than a window old"
        );
        assert_eq!(window.starved(), LOAD_WINDOW_TICKS);
        assert_eq!(window.status(), WorkState::Starved);

        // The tick after a full ring clears one whole bucket before counting.
        window.record(WorkState::Blocked);
        assert_eq!(window.observed(), LOAD_WINDOW_TICKS - LOAD_BUCKET_TICKS + 1);
        assert_eq!(window.blocked(), 1);
        assert_eq!(window.starved(), LOAD_WINDOW_TICKS - LOAD_BUCKET_TICKS);
    }

    #[test]
    fn load_status_follows_the_full_load_threshold_and_the_tiebreak() {
        let mut window = LoadWindow::new();
        // 95 working + 5 starved of 100 = exactly the threshold: working.
        for _ in 0..95 {
            window.record(WorkState::Working);
        }
        for _ in 0..5 {
            window.record(WorkState::Starved);
        }
        assert_eq!(window.percent(), FULL_LOAD_PERCENT);
        assert_eq!(window.status(), WorkState::Working);

        // One more starved tick drops it below, and starved wins the tie
        // against blocked.
        window.record(WorkState::Starved);
        assert_eq!(window.status(), WorkState::Starved);
        for _ in 0..6 {
            window.record(WorkState::Blocked);
        }
        assert_eq!(window.starved(), 6);
        assert_eq!(window.blocked(), 6);
        assert_eq!(window.status(), WorkState::Starved, "a tie reads starved");
        window.record(WorkState::Blocked);
        assert_eq!(window.status(), WorkState::Blocked);
    }

    #[test]
    fn load_window_restores_only_a_ring_it_could_have_written() {
        let mut window = LoadWindow::new();
        for _ in 0..(LOAD_BUCKET_TICKS * 7 + 3) {
            window.record(WorkState::Working);
        }
        let restored =
            LoadWindow::restore(window.buckets(), window.cursor()).expect("its own ring restores");
        assert_eq!(restored, window);

        assert!(LoadWindow::restore(&window.buckets()[..LOAD_BUCKETS - 1], 0).is_none());
        assert!(LoadWindow::restore(window.buckets(), LOAD_BUCKETS).is_none());
        let mut over = *window.buckets();
        over[0].working = LOAD_BUCKET_TICKS + 1;
        assert!(LoadWindow::restore(&over, 0).is_none());
    }

    /// The counting rule's reason to exist: the completion tick counts as
    /// work, so a machine that is fed and drained without interruption reads
    /// exactly 100 % with no one-tick dip per cycle.
    #[test]
    fn a_continuously_fed_and_drained_machine_reads_exactly_full_load() {
        let mut ledger = ProductionLedger::new();
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        for tick in 0..LOAD_WINDOW_TICKS as u64 * 2 {
            let short = 10 - machine.input.count("tomato").min(10);
            machine.input.add("tomato", short);
            let made = machine.output.count("tomato_sauce");
            machine.output.remove("tomato_sauce", made);
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
            assert_eq!(
                machine.load.starved() + machine.load.blocked(),
                0,
                "tick {tick}: a fed and drained machine never records a non-working tick"
            );
        }
        assert_eq!(machine.load.observed(), LOAD_WINDOW_TICKS);
        assert_eq!(machine.load.percent(), 100);
        assert_eq!(machine.load.status(), WorkState::Working);
    }

    #[test]
    fn a_machine_that_is_never_fed_reads_starved() {
        let mut ledger = ProductionLedger::new();
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        for tick in 0..50 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }
        assert_eq!(machine.load.starved(), 50);
        assert_eq!(machine.load.working(), 0);
        assert_eq!(machine.load.percent(), 0);
        assert_eq!(machine.load.status(), WorkState::Starved);
    }

    #[test]
    fn a_machine_held_on_a_full_output_reads_blocked() {
        let mut ledger = ProductionLedger::new();
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        for tick in 0..LOAD_WINDOW_TICKS as u64 * 2 {
            let short = 10 - machine.input.count("tomato").min(10);
            machine.input.add("tomato", short);
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }
        assert!(
            machine.process.is_some(),
            "the last batch is finished and held"
        );
        assert_eq!(
            machine.output.count("tomato_sauce"),
            10,
            "output is brim full"
        );
        assert_eq!(machine.load.starved(), 0, "it was never short of tomatoes");
        assert_eq!(machine.load.status(), WorkState::Blocked);
        assert!(machine.load.blocked() > machine.load.working());
    }

    /// The fluid side of the same rule: a finished batch with nowhere to go.
    #[test]
    fn a_machine_held_on_a_full_outlet_pipe_reads_blocked() {
        let mut ledger = ProductionLedger::new();
        let recipes = producer_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        let mut inflow = one_tile_network("ketchup_mix", 10, 10);
        let mut outflow = one_tile_network("ketchup", 10, 10); // brim full
        for tick in 0..100 {
            machine.tick(
                tick,
                &recipes,
                vec![tap(&mut inflow, 0)],
                Some(tap(&mut outflow, 0)),
                None,
                &mut ledger,
            );
        }
        // Ticks 0..=4 working (start + 4 advancing), then held from tick 5 on.
        assert_eq!(machine.load.working(), 5);
        assert_eq!(machine.load.blocked(), 95);
        assert_eq!(machine.load.status(), WorkState::Blocked);
    }

    #[test]
    fn a_craft_records_its_inputs_as_consumed_and_its_outputs_as_produced() {
        let recipes = registry();
        let mut ledger = ProductionLedger::new();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes, vec![], None, None, &mut ledger);
        // Consumed at the start, in the recipe's own quantity; nothing made
        // yet.
        assert_eq!(ledger.consumed_total("tomato"), 4);
        assert_eq!(ledger.produced_total("tomato_sauce"), 0);
        for tick in 1..10 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }
        assert_eq!(
            ledger.produced_total("tomato_sauce"),
            0,
            "not before it lands"
        );
        machine.tick(10, &recipes, vec![], None, None, &mut ledger);
        assert_eq!(ledger.produced_total("tomato_sauce"), 1);
        assert_eq!(
            ledger.consumed_total("tomato"),
            4,
            "starting is the only consumption"
        );
        assert_eq!(ledger.kind_of("tomato"), Some(StatKind::Item));
    }

    #[test]
    fn a_batch_held_on_a_full_output_is_recorded_only_when_it_lands() {
        let recipes = registry();
        let mut ledger = ProductionLedger::new();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 1);
        machine.output.add("tomato_sauce", 1);
        machine.input.add("tomato", 4);
        for tick in 0..=20 {
            machine.tick(tick, &recipes, vec![], None, None, &mut ledger);
        }
        assert!(machine.process.is_some(), "held");
        assert_eq!(ledger.produced_total("tomato_sauce"), 0);
        machine.output.remove("tomato_sauce", 1);
        machine.tick(21, &recipes, vec![], None, None, &mut ledger);
        assert_eq!(ledger.produced_total("tomato_sauce"), 1);
    }

    #[test]
    fn fluids_are_recorded_as_fluids_in_the_recipe_s_whole_units() {
        let recipes = fluid_registry();
        let mut ledger = ProductionLedger::new();
        let (mut network, _tank) = supplied_network();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 1);
        machine.tick(
            0,
            &recipes,
            vec![tap(&mut network, 1)],
            None,
            None,
            &mut ledger,
        );
        assert!(
            machine.process.is_some(),
            "the fixture must be able to start"
        );
        assert_eq!(ledger.consumed_total("vinegar"), 2);
        assert_eq!(ledger.kind_of("vinegar"), Some(StatKind::Fluid));
        assert_eq!(ledger.consumed_total("tomato_sauce"), 1);
    }
}
