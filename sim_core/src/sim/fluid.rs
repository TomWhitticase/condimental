//! Fluids (ARCHITECTURE.md §9 milestone 6; §3.2's `FluidNetwork`
//! component, §3.4's model).
//!
//! §3.4 is explicit about the model and this module follows it literally:
//! a fluid is **a connected-network pressure system, never particles**.
//! There is no droplet, no per-unit entity, no position along a pipe — a
//! [`FluidNetwork`] is a run of [`PipeSegment`]s, each holding a *volume*
//! of one fluid type, and every tick volume flows between adjacent
//! segments toward equal pressure. That is the whole model. (§3.4's own
//! warning: individual-droplet simulation is "the exact mistake Factorio's
//! own devs made with belt items early on ... fluids are the same trap at a
//! smaller item size.")
//!
//! **Pressure** here is a segment's fill fraction, `volume / capacity` —
//! dimensionless, in `[0, 1]`. Flow between two adjacent segments is
//! proportional to their pressure difference (see `FLOW_RATE_*` below), so a
//! full segment next to an empty one pushes hard and two near-equal
//! segments barely move. With this milestone's uniform segment capacities
//! that reduces to "move half the volume difference each tick," but it is
//! written in terms of pressure so a future non-uniform pipe (a tank
//! segment, say) behaves correctly without rewriting the flow rule.
//!
//! **Scope — deliberately narrow, mirroring milestone 5's single hardcoded
//! `Contract`.** This milestone builds *one fixed pipe run*, constructed
//! once in `World::new` exactly the way the belt chain is (see
//! `world.rs`), feeding *one* machine. Specifically NOT in scope, and
//! deliberately not half-built here:
//!
//! - **Player-placeable pipes.** Placement is a much later milestone
//!   (ARCHITECTURE.md §9 items 8/9); nothing here assumes the topology can
//!   ever change after construction.
//! - **Union-find topology recomputation (§3.8).** That machinery exists
//!   to re-derive connected components *when a pipe is placed or removed*.
//!   With a fixed topology there is nothing to recompute, so adjacency is
//!   simply "consecutive indices in `segments`" — one `Vec`, no graph.
//! - **Multiple fluids / multiple networks.** One network, one fluid type.
//! - **Fluid *outputs*.** Recipes can consume a fluid (see
//!   `recipe::Recipe::fluid_inputs`); producing one back into a network is
//!   future work, so the only thing that can add volume to the system is the
//!   player filling the tank ([`FluidTank::fill`]).
//!
//! **Where the fluid comes from (the "Visual overhaul round 2" epic's ticket
//! 1).** Milestone 6 shipped the smallest thing that wasn't a lie about
//! provenance: the run's first segment was flagged `is_source` and refilled
//! itself to capacity every tick — an off-map tap that could never run out.
//! That is now replaced by a [`FluidTank`]: a capped, player-loadable buffer
//! that is the network's *only* supply, and that is drained by the very same
//! per-tick equalization pass that moves fluid between pipe tiles. This is
//! the fluid twin of what [`super::hopper::Hopper`] is for solids — the
//! player sets the *supply*, the machinery sets the *rate* — and it means
//! running dry is a real state the player can get into and fix.
//!
//! The tank is not a `PipeSegment` and is not in `segments`: it is a
//! different *kind* of thing (a placed building with a footprint, like the
//! hopper, rather than a tile of pipe), and keeping it out of `segments`
//! keeps the pipe run's snapshot geometry exactly what it always was. What
//! it *does* share with a segment is the flow rule: `tick` equalizes
//! tank <-> `segments[0]` with the identical pressure-difference formula it
//! then applies to every adjacent pipe pair, so there is one flow mechanism
//! in this file, not two.
//!
//! All volume state is `Fx` fixed-point (ARCHITECTURE.md §3.9) — no `f32`/
//! `f64` appears anywhere in this file. Floats only appear one layer out,
//! in `world.rs`'s presentation snapshots, the same one-way conversion
//! `BeltSnapshot` already makes.

use super::types::{Fx, FluidId, TileCoord};

/// Fraction of the pressure difference that moves between two adjacent
/// segments per tick. `0.5` equalizes a pair in a single tick when their
/// capacities match, which keeps a short run responsive without letting a
/// draw at the far end be invisible — pick something smaller for a
/// deliberately sluggish long pipe later.
const FLOW_RATE_NUMERATOR: i32 = 1;
const FLOW_RATE_DENOMINATOR: i32 = 2;

/// One tile of pipe. Holds a volume of the network's fluid — never
/// individual droplets, never a position along the pipe (§3.4).
#[derive(Debug, Clone)]
pub struct PipeSegment {
    pub tile: TileCoord,
    /// How much fluid is currently in this segment, in whole fluid units
    /// (fractional because equalization moves partial units around).
    pub volume: Fx,
    /// Maximum `volume` this segment can hold.
    pub capacity: Fx,
    /// **The run's supply end: the segment the [`FluidTank`] discharges
    /// into.** The name is unchanged from milestone 6 (presentation keys a
    /// different silhouette off it, and renaming it would break that for no
    /// gain), but its *meaning* changed with the "Visual overhaul round 2"
    /// epic's ticket 1: it used to mark an off-map tap that refilled itself
    /// to `capacity` every tick and could never run out. It no longer
    /// refills, and it drains exactly like any other pipe tile — the flag
    /// now says only "the tank is on the other side of this tile," which is
    /// what a renderer actually needs it for. Every segment, this one
    /// included, starts empty; the run filling up is something the player
    /// causes by filling the tank.
    pub is_source: bool,
}

impl PipeSegment {
    pub fn new(tile: TileCoord, capacity: Fx, is_source: bool) -> Self {
        Self {
            tile,
            // Every segment starts empty, the supply end included: the pipe
            // run filling up is observable behavior driven by the tank, not
            // an initial-state fiction.
            volume: Fx::ZERO,
            capacity,
            is_source,
        }
    }

    /// Fill fraction in `[0, 1]` — see the module doc on what "pressure"
    /// means here.
    pub fn pressure(&self) -> Fx {
        if self.capacity == Fx::ZERO {
            Fx::ZERO
        } else {
            self.volume / self.capacity
        }
    }

    pub fn room(&self) -> Fx {
        self.capacity - self.volume
    }
}

/// The machine kind id (`data/machines.ron`) the fluid tank is placed as.
///
/// Spelled out in Rust for exactly the reason [`super::hopper::HOPPER_KIND`]
/// is: the tank is *structural* — a world either has a fluid-requiring
/// recipe and therefore exactly one tank at the head of its pipe run, or it
/// has neither — so it is not derivable from `recipes.ron`'s list the way a
/// processing stage's kind is. Its footprint still comes from
/// `data/machines.ron` like every other placed building; only the id lives
/// here.
pub const FLUID_TANK_KIND: &str = "fluid_tank";

/// How many whole fluid units the tank holds when full.
///
/// Sized against the line it feeds, the same way [`super::hopper::
/// HOPPER_CAPACITY`] was, rather than picked round. The shipped mixer draws
/// 2 vinegar per craft and one craft needs 1 tomato paste, i.e. 4 tomatoes
/// (`data/recipes.ron`) — so a full 64-tomato hopper is 16 crafts, or 32
/// vinegar. Against that:
///
/// - The pipe run itself holds `4 x 10 = 40` units when full, and a few of
///   those stay in it as standing charge: the run equalizes to a common
///   pressure, so once the tank is dry the inlet stops being able to hand
///   over a whole craft's worth in one draw while the pipes still read a
///   little above empty. A tank sized like the hopper (64) would spend most
///   of a fill just wetting the line.
/// - 120 units is 60 crafts nominal — call it ~56 net of that standing
///   charge — which is roughly **three and a half full hoppers of tomatoes
///   per tank fill**.
///
/// That ratio is the intended feel, and it is a design call, not a technical
/// constraint: vinegar is the slower-burning of the two consumables, so
/// topping the tank up is a thing the player does occasionally rather than
/// constantly, while still being a thing that can be forgotten and stall the
/// line. Like `HOPPER_CAPACITY` it is a Rust constant rather than a
/// `machines.ron` field, and for the same reason — when buffer sizes become
/// per-kind content, they should all move there together.
pub const FLUID_TANK_CAPACITY: u32 = 120;

/// How much one fill command adds when it doesn't say (see
/// `world::PlayerCommand::FillFluidTank`) — a drum of vinegar, a quarter of
/// [`FLUID_TANK_CAPACITY`], so a full tank is four deliberate actions.
/// Exactly the rule [`super::hopper::HOPPER_FILL_AMOUNT`] follows.
pub const FLUID_TANK_FILL_AMOUNT: u32 = 30;

/// A capped, player-loadable buffer of one fluid — the network's only
/// supply, and the fluid twin of [`super::hopper::Hopper`].
///
/// It holds no fluid *id* of its own: a [`FluidNetwork`] carries exactly one
/// fluid type and owns its tank, so the id would be a second copy of the
/// same fact and therefore a second thing that can disagree.
///
/// It is drained by nothing in this type: [`FluidNetwork::tick`] equalizes
/// it against the run's supply segment with the same pressure rule it uses
/// between pipe tiles, and the machine draws from the far end of that run.
/// So "the tank empties as the machine consumes" is a consequence of the
/// model rather than a second bookkeeping path, which is exactly how
/// `Hopper` relates to `BeltSegment::try_spawn`.
#[derive(Debug, Clone)]
pub struct FluidTank {
    volume: Fx,
    capacity: Fx,
}

impl FluidTank {
    pub fn new(capacity: Fx) -> Self {
        Self {
            // A new tank starts empty, exactly as a new hopper does: the
            // first thing a player does is fill it.
            volume: Fx::ZERO,
            capacity,
        }
    }

    pub fn volume(&self) -> Fx {
        self.volume
    }

    pub fn capacity(&self) -> Fx {
        self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.volume <= Fx::ZERO
    }

    /// Fill fraction in `[0, 1]` — the same "pressure" this module's doc
    /// defines for a pipe segment. This is what a fill-level UI reads; it is
    /// **not** what the tank discharges at (see
    /// [`FluidTank::discharge_pressure`]).
    pub fn pressure(&self) -> Fx {
        if self.capacity == Fx::ZERO {
            Fx::ZERO
        } else {
            self.volume / self.capacity
        }
    }

    /// The pressure this tank presents to the pipe it feeds: **full head,
    /// always.**
    ///
    /// This is the one place the tank differs from a pipe tile, and it is a
    /// deliberate modelling choice rather than a shortcut, so it is worth
    /// stating why. The tank is a *raised supply vessel* — it stands beside
    /// the run and discharges downhill into it. If instead it equalized on
    /// its own fill *fraction*, like two pipe tiles do, then a 120-unit tank
    /// feeding a 40-unit run would settle at a common fraction and keep
    /// three quarters of every remaining unit inside itself forever: the
    /// player would watch the mixer starve while the tank still read a
    /// quarter full, and no amount of waiting would move it. Elevation head
    /// is both the physically real reason a plant puts a supply tank up on a
    /// stand and the behavior the mechanic needs — the tank empties into the
    /// run, and *then* the run empties into the machine.
    ///
    /// It is a *constant*, including when the tank is empty, and that is
    /// load-bearing rather than lazy: it is what makes flow between the tank
    /// and the run strictly one-way, with no special case anywhere in
    /// [`equalize`]. A constant maximum means the signed pressure difference
    /// always points *out* of the tank, and `equalize` already clamps every
    /// transfer to what the donor actually has — so a full tank pushes, an
    /// empty one moves nothing, and neither can ever pull fluid back uphill
    /// out of the run. Making it fall to zero when empty looks tidier and is
    /// a trap: it inverts the difference, and the empty tank starts
    /// siphoning the pipes.
    pub fn discharge_pressure(&self) -> Fx {
        Fx::ONE
    }

    pub fn room(&self) -> Fx {
        self.capacity - self.volume
    }

    /// Adds up to `amount` **whole fluid units**, clamped at capacity, and
    /// returns how much was actually added (zero when already full).
    ///
    /// Clamping is silent — never an error, never a panic — mirroring
    /// [`super::hopper::Hopper::fill`] exactly, which in turn mirrors
    /// `Inventory::add`: an impossible player command is a no-op, not a
    /// failure (`World::issue_command`'s documented contract).
    ///
    /// `amount` is a `u32` rather than an [`Fx`] on purpose. It is raw
    /// player input arriving off the JSON wire, and `Fx` is a 16.16
    /// fixed-point type whose integer range stops around 32 767 — so
    /// `Fx::from_num(u32::MAX)` would *overflow* rather than clamp. Clamping
    /// happens in integer space first, before any conversion, so
    /// `fill(u32::MAX)` is simply "fill it up".
    pub fn fill(&mut self, amount: u32) -> Fx {
        let room = self.room();
        if room <= Fx::ZERO {
            return Fx::ZERO;
        }
        // Anything at or above the tank's own whole-unit capacity means
        // "fill it up", and is never converted to Fx at all.
        let capacity_units = self.capacity.ceil().to_num::<i64>().max(0) as u64;
        let added = if u64::from(amount) >= capacity_units {
            room
        } else {
            Fx::from_num(amount).min(room)
        };
        self.volume += added;
        added
    }
}

/// A connected group of pipe segments carrying one fluid type
/// (ARCHITECTURE.md §3.2's `FluidNetwork`). Adjacency is "consecutive
/// indices in `segments`" — see the module doc for why a fixed topology
/// needs no graph/union-find machinery at this milestone.
#[derive(Debug, Clone)]
pub struct FluidNetwork {
    pub fluid: FluidId,
    segments: Vec<PipeSegment>,
    /// Which segment a machine taps. A machine draws from *this segment
    /// only*, not from the network total — which is what makes the
    /// equalization pass load-bearing rather than cosmetic: drain the
    /// inlet and the machine stalls until pressure carries more fluid down
    /// the run from the tank.
    inlet_index: usize,
    /// The network's only supply (ticket 1). Owned here, rather than beside
    /// the network in `world.rs`, because `tick` has to equalize it against
    /// `segments[0]` — the tank and the run are one hydraulic system, and
    /// splitting them would mean either a second flow mechanism or a
    /// borrow-checker fight for no benefit. Its *placement* (tile, footprint,
    /// kind) lives in `world.rs` alongside the hopper's, since that is world
    /// layout rather than fluid dynamics.
    tank: FluidTank,
}

impl FluidNetwork {
    /// Panics if `segments` is empty or `inlet_index` is out of range —
    /// same "clear panic at construction time, not a silently-stuck
    /// simulation" stance `world::validate_chain` takes.
    pub fn new(
        fluid: FluidId,
        segments: Vec<PipeSegment>,
        inlet_index: usize,
        tank: FluidTank,
    ) -> Self {
        assert!(
            !segments.is_empty(),
            "a FluidNetwork must have at least one pipe segment"
        );
        assert!(
            inlet_index < segments.len(),
            "FluidNetwork inlet_index {inlet_index} is out of range for {} segment(s)",
            segments.len()
        );
        Self {
            fluid,
            segments,
            inlet_index,
            tank,
        }
    }

    pub fn tank(&self) -> &FluidTank {
        &self.tank
    }

    /// Loads the tank — see [`FluidTank::fill`] for the clamping contract.
    /// Routed through the network rather than exposing the tank mutably so
    /// there is one way in, matching how `World` only ever fills the hopper
    /// through `Hopper::fill`.
    pub fn fill_tank(&mut self, amount: u32) -> Fx {
        self.tank.fill(amount)
    }

    pub fn segments(&self) -> &[PipeSegment] {
        &self.segments
    }

    pub fn inlet_index(&self) -> usize {
        self.inlet_index
    }

    pub fn inlet(&self) -> &PipeSegment {
        &self.segments[self.inlet_index]
    }

    /// Total volume held across the whole **pipe run** — §3.4's "each
    /// `FluidNetwork` has a total volume and a fluid type."
    ///
    /// Deliberately excludes the tank: the tank is a building with its own
    /// capacity and its own snapshot, not a length of pipe, and folding it
    /// in here would silently change what every existing consumer of
    /// `FluidSnapshot::total_volume`/`pressure` was reading. Ask the tank
    /// for the tank's contents ([`FluidNetwork::tank`]).
    pub fn total_volume(&self) -> Fx {
        self.segments.iter().fold(Fx::ZERO, |acc, s| acc + s.volume)
    }

    pub fn total_capacity(&self) -> Fx {
        self.segments
            .iter()
            .fold(Fx::ZERO, |acc, s| acc + s.capacity)
    }

    /// Whether the inlet segment can currently supply `amount` of `fluid`.
    /// Wrong fluid type is a hard "no," never an implicit conversion.
    pub fn can_supply(&self, fluid: &str, amount: Fx) -> bool {
        self.fluid == fluid && self.inlet().volume >= amount
    }

    /// Removes `amount` of `fluid` from the inlet segment, or returns
    /// `false` and changes nothing. All-or-nothing on purpose: a machine
    /// must not start a craft on a partial fluid charge, exactly as it
    /// must not start on partial solid input.
    pub fn draw(&mut self, fluid: &str, amount: Fx) -> bool {
        if !self.can_supply(fluid, amount) {
            return false;
        }
        self.segments[self.inlet_index].volume -= amount;
        true
    }

    /// Advances the network one tick: equalize the tank against the run's
    /// supply segment, then equalize pressure across each adjacent pipe
    /// pair. Returns whether any volume actually moved (used the same way
    /// `Machine::tick`'s return value is — "did anything a viewer would
    /// notice change").
    ///
    /// **The tank is just the first participant in the same pass.** Ticket 1
    /// deleted the old per-tick source refill; there is now exactly one flow
    /// rule in this file ([`equalize`]), applied first to
    /// `tank <-> segments[0]` and then to every `segments[i] <->
    /// segments[i + 1]`. That is what makes "the machine's consumption
    /// drains the tank" fall out of the model instead of needing its own
    /// bookkeeping: a draw at the inlet lowers that segment's pressure,
    /// which pulls from its neighbour, which pulls from the tank.
    ///
    /// The tank enters that rule with its
    /// [`discharge_pressure`](FluidTank::discharge_pressure) rather than its
    /// fill fraction — it is a raised vessel, see that method for why. The
    /// rule itself is symmetric and unchanged; the asymmetry is entirely in
    /// what pressure each side presents.
    ///
    /// The pass is sequential (each pair sees the results of the pair before
    /// it), which makes flow travel a full run in one tick in the pass
    /// direction. That is order-dependent by construction, and the order is
    /// fixed (tank first, then `0..len-1`, always) — so it is deterministic,
    /// which is the property ARCHITECTURE.md §3.9 actually requires. Volume
    /// is conserved exactly: every transfer is clamped to what the donor has
    /// and what the receiver has room for and applied as a paired
    /// subtract/add, so nothing is created or destroyed here. Since ticket 1
    /// the *only* source in the whole system is [`FluidTank::fill`], and the
    /// only sink is [`FluidNetwork::draw`].
    pub fn tick(&mut self) -> bool {
        let rate = Fx::from_num(FLOW_RATE_NUMERATOR) / Fx::from_num(FLOW_RATE_DENOMINATOR);
        let mut changed = false;

        {
            let tank_pressure = self.tank.discharge_pressure();
            let tank_capacity = self.tank.capacity;
            let head = &mut self.segments[0];
            let head_pressure = head.pressure();
            changed |= equalize(
                Side {
                    volume: &mut self.tank.volume,
                    pressure: tank_pressure,
                    capacity: tank_capacity,
                },
                Side {
                    volume: &mut head.volume,
                    pressure: head_pressure,
                    capacity: head.capacity,
                },
                rate,
            );
        }

        for i in 0..self.segments.len().saturating_sub(1) {
            let (left, right) = self.segments.split_at_mut(i + 1);
            let a = &mut left[i];
            let b = &mut right[0];
            let (a_pressure, b_pressure) = (a.pressure(), b.pressure());
            changed |= equalize(
                Side {
                    volume: &mut a.volume,
                    pressure: a_pressure,
                    capacity: a.capacity,
                },
                Side {
                    volume: &mut b.volume,
                    pressure: b_pressure,
                    capacity: b.capacity,
                },
                rate,
            );
        }

        changed
    }
}

/// One side of a transfer: a mutable volume, the pressure that holder
/// presents, and how much it can hold.
///
/// Pressure is passed in rather than derived because the two kinds of holder
/// answer it differently — a [`PipeSegment`] presents its fill fraction, a
/// [`FluidTank`] presents its
/// [`discharge_pressure`](FluidTank::discharge_pressure) — while sharing the
/// transfer rule itself.
struct Side<'a> {
    volume: &'a mut Fx,
    pressure: Fx,
    capacity: Fx,
}

/// Moves volume between two adjacent fluid holders toward equal *pressure*,
/// returning whether anything moved.
///
/// This is §3.4's whole flow model in one function, and deliberately the
/// only place it is written down: [`FluidNetwork::tick`] applies it to the
/// tank/supply-segment pair and to every adjacent pipe pair alike, so a tank
/// and a pipe tile can never drift into behaving differently. It takes loose
/// [`Side`]s rather than a trait over the two types precisely because they
/// are different kinds of thing that happen to share this one rule —
/// abstracting over them would be more machinery than the rule is.
///
/// The transfer is clamped to what the donor actually has and what the
/// receiver has room for, and applied as a paired subtract/add, so volume is
/// conserved exactly (fixed-point, so that is an exact equality, not an
/// epsilon).
fn equalize(a: Side, b: Side, rate: Fx) -> bool {
    // Scaling by the *smaller* capacity keeps a big tank from shoving more
    // into a small pipe in one tick than the pressure difference warrants.
    let scale = a.capacity.min(b.capacity);
    // Signed: positive flows a -> b, negative flows b -> a.
    let desired = (a.pressure - b.pressure) * rate * scale;
    if desired == Fx::ZERO {
        return false;
    }

    if desired > Fx::ZERO {
        let flow = desired.min(*a.volume).min(b.capacity - *b.volume);
        if flow > Fx::ZERO {
            *a.volume -= flow;
            *b.volume += flow;
            return true;
        }
    } else {
        let flow = (-desired).min(*b.volume).min(a.capacity - *a.volume);
        if flow > Fx::ZERO {
            *b.volume -= flow;
            *a.volume += flow;
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap() -> Fx {
        Fx::from_num(10)
    }

    fn tank_cap() -> Fx {
        Fx::from_num(FLUID_TANK_CAPACITY)
    }

    /// A supply segment at index 0 followed by `pipes` plain segments, fed
    /// by a tank; the last segment is the inlet — the same shape `World`
    /// builds.
    fn run(pipes: usize) -> FluidNetwork {
        let mut segments = vec![PipeSegment::new((0, 0), cap(), true)];
        for i in 0..pipes {
            segments.push(PipeSegment::new((i as i32 + 1, 0), cap(), false));
        }
        let inlet = segments.len() - 1;
        FluidNetwork::new(
            "vinegar".to_string(),
            segments,
            inlet,
            FluidTank::new(tank_cap()),
        )
    }

    /// A network with no tank contents and hand-set segment volumes, for
    /// testing the pipe-to-pipe half of the pass in isolation.
    fn sourceless(volumes: &[i32]) -> FluidNetwork {
        let segments: Vec<PipeSegment> = volumes
            .iter()
            .enumerate()
            .map(|(i, v)| PipeSegment {
                tile: (i as i32, 0),
                volume: Fx::from_num(*v),
                capacity: cap(),
                is_source: false,
            })
            .collect();
        let inlet = segments.len() - 1;
        // A zero-capacity tank can neither give nor take, so it drops out of
        // the pass entirely — which is exactly what "no source" means here.
        FluidNetwork::new(
            "vinegar".to_string(),
            segments,
            inlet,
            FluidTank::new(Fx::ZERO),
        )
    }

    // -----------------------------------------------------------------
    // The tank itself (ticket 1) — mirrors hopper.rs's own fill tests.
    // -----------------------------------------------------------------

    #[test]
    fn a_new_tank_is_empty_and_fill_reports_what_it_added() {
        let mut tank = FluidTank::new(tank_cap());
        assert!(tank.is_empty());
        assert_eq!(tank.volume(), Fx::ZERO);
        assert_eq!(tank.pressure(), Fx::ZERO);

        assert_eq!(tank.fill(10), Fx::from_num(10));
        assert_eq!(tank.volume(), Fx::from_num(10));
        assert_eq!(
            tank.fill(FLUID_TANK_CAPACITY),
            tank_cap() - Fx::from_num(10),
            "only the remaining room is accepted"
        );
        assert_eq!(tank.volume(), tank_cap());
        assert_eq!(tank.pressure(), Fx::from_num(1));
        assert_eq!(tank.fill(1), Fx::ZERO, "filling a full tank adds nothing");
        assert_eq!(tank.volume(), tank_cap());
    }

    /// Overfilling must be a clamp, never a panic and never a fixed-point
    /// overflow — `amount` is raw player input, and `Fx` is 16.16, so
    /// `Fx::from_num(u32::MAX)` would not even be representable. See
    /// `FluidTank::fill`'s doc.
    #[test]
    fn filling_far_past_capacity_is_clamped_not_overflowed() {
        let mut tank = FluidTank::new(tank_cap());
        assert_eq!(tank.fill(u32::MAX), tank_cap());
        assert_eq!(tank.volume(), tank_cap());
        assert_eq!(tank.fill(u32::MAX), Fx::ZERO);
        assert_eq!(tank.volume(), tank_cap());
    }

    // -----------------------------------------------------------------
    // The tank as the network's supply.
    // -----------------------------------------------------------------

    /// The behavior ticket 1 deliberately removed: the supply segment is a
    /// plain pipe tile now, so an untouched network stays empty forever
    /// instead of conjuring fluid every tick.
    #[test]
    fn a_network_with_an_empty_tank_never_fills_itself() {
        let mut net = run(3);
        for _ in 0..500 {
            net.tick();
        }
        assert_eq!(net.total_volume(), Fx::ZERO);
        assert_eq!(net.segments()[0].volume, Fx::ZERO);
        assert!(net.tank().is_empty());
    }

    #[test]
    fn fluid_reaches_the_far_inlet_of_a_run_from_the_tank() {
        let mut net = run(3);
        net.fill_tank(FLUID_TANK_CAPACITY);
        assert_eq!(net.inlet().volume, Fx::ZERO, "inlet starts empty");

        for _ in 0..50 {
            net.tick();
        }

        assert!(
            net.inlet().volume > Fx::ZERO,
            "pressure from the tank must carry fluid down the run to the inlet"
        );
        assert!(
            net.tank().volume() < tank_cap(),
            "and the fluid in the run must have come out of the tank"
        );
    }

    /// Nothing is created or destroyed once the tank has been filled: the
    /// tank plus the run is a closed system with the fill as its only input.
    #[test]
    fn tank_plus_run_conserves_volume_exactly() {
        let mut net = run(3);
        let filled = net.fill_tank(40);
        for _ in 0..500 {
            net.tick();
        }
        assert_eq!(
            net.tank().volume() + net.total_volume(),
            filled,
            "fixed-point, so this is an exact equality, not an epsilon comparison"
        );
    }

    /// The whole point of the replacement: a machine's draws come out of the
    /// tank, and once the tank is dry the run cannot supply the machine any
    /// more. This is the fluid twin of `hopper.rs`'s jammed-belt hold.
    #[test]
    fn drawing_repeatedly_empties_the_tank_and_then_the_run_runs_dry() {
        let mut net = run(3);
        // More than the run itself can hold (4 x 10), so the tank is still
        // holding a reserve once the line is wet — which is the state a
        // sustained draw has to eat through.
        net.fill_tank(80);
        for _ in 0..200 {
            net.tick();
        }
        let start = net.tank().volume() + net.total_volume();
        assert!(
            net.tank().volume() > Fx::ZERO,
            "the tank keeps the reserve the run had no room for"
        );
        // "To the brim" within fixed-point quantization: each transfer is a
        // fraction of a shrinking pressure difference, so the last sliver
        // rounds to zero rather than closing exactly. That residue is
        // bounded and does not accumulate — the same property
        // `a_sourceless_network_settles_to_even_pressure_and_stops_changing`
        // relies on.
        assert!(
            net.total_capacity() - net.total_volume() < Fx::from_num(0.01),
            "a raised tank fills the run to the brim before holding the rest back (run at {} \
             of {})",
            net.total_volume(),
            net.total_capacity()
        );

        let draw = Fx::from_num(1);
        let mut drawn = Fx::ZERO;
        for _ in 0..10_000 {
            if net.draw("vinegar", draw) {
                drawn += draw;
            }
            net.tick();
        }

        assert!(drawn > Fx::ZERO, "the machine must have been supplied at all");
        assert!(
            net.tank().is_empty(),
            "sustained draw must drain the tank completely, not leave a share of every \
             remaining unit stranded in it (tank at {}) — see FluidTank::discharge_pressure",
            net.tank().volume()
        );
        assert!(
            !net.draw("vinegar", draw),
            "with the tank dry the inlet must be unable to supply another draw"
        );
        assert_eq!(
            net.tank().volume() + net.total_volume() + drawn,
            start,
            "every unit is either still in the system or was drawn — none created or lost"
        );
    }

    // -----------------------------------------------------------------
    // The pressure model itself (unchanged by ticket 1).
    // -----------------------------------------------------------------

    #[test]
    fn pressure_equalizes_between_two_adjacent_segments() {
        // No tank involved: a full segment beside an empty one must even
        // out, which is §3.4's entire model in its smallest form.
        let mut net = sourceless(&[10, 0]);

        net.tick();

        let (a, b) = (net.segments()[0].volume, net.segments()[1].volume);
        assert_eq!(a, b, "one tick at FLOW_RATE 0.5 fully equalizes a pair");
        assert_eq!(a, cap() / 2);
    }

    #[test]
    fn equalization_conserves_total_volume_exactly() {
        let mut net = sourceless(&[7, 0, 3]);
        let before = net.total_volume();

        for _ in 0..100 {
            net.tick();
        }

        assert_eq!(
            net.total_volume(),
            before,
            "no tank, no draw: total volume must be conserved exactly (fixed-point, so this is \
             an exact equality, not an epsilon comparison)"
        );
    }

    #[test]
    fn a_sourceless_network_settles_to_even_pressure_and_stops_changing() {
        let mut net = sourceless(&[10, 0, 0]);

        for _ in 0..200 {
            net.tick();
        }

        assert!(
            !net.tick(),
            "a settled network must report no change, so it can eventually be slept \
             (ARCHITECTURE.md §3.7)"
        );
        // "Even" means even to within fixed-point quantization: 10 units
        // across 3 segments has no exact I16F16 representation, so a
        // settled network can sit a ULP or two apart and correctly report
        // no further change (a sub-ULP flow rounds to zero). That residue
        // is bounded and never accumulates — it is not drift.
        let first = net.segments()[0].pressure();
        let tolerance = Fx::DELTA * 2;
        for segment in net.segments() {
            let diff = (segment.pressure() - first).abs();
            assert!(
                diff <= tolerance,
                "pressure must be even once settled (got {} vs {first}, diff {diff})",
                segment.pressure()
            );
        }
    }

    #[test]
    fn draw_is_all_or_nothing_and_refuses_the_wrong_fluid() {
        let mut net = run(2);
        net.fill_tank(FLUID_TANK_CAPACITY);
        for _ in 0..50 {
            net.tick();
        }
        let available = net.inlet().volume;
        assert!(available > Fx::ZERO, "the tank must have supplied the inlet");

        assert!(
            !net.draw("water", Fx::from_num(1)),
            "a network carrying vinegar must never supply water"
        );
        assert!(
            !net.draw("vinegar", available + Fx::from_num(1)),
            "a draw larger than the inlet holds must fail outright, not partially drain it"
        );
        assert_eq!(net.inlet().volume, available, "a refused draw must change nothing");

        assert!(net.draw("vinegar", available));
        assert_eq!(net.inlet().volume, Fx::ZERO);
    }

    #[test]
    fn a_drained_inlet_refills_from_the_tank_over_the_following_ticks() {
        let mut net = run(2);
        net.fill_tank(FLUID_TANK_CAPACITY);
        for _ in 0..50 {
            net.tick();
        }
        let available = net.inlet().volume;
        assert!(net.draw("vinegar", available));

        net.tick();

        assert!(
            net.inlet().volume > Fx::ZERO,
            "equalization must carry more fluid to a drained inlet — this is what makes the \
             pressure model load-bearing rather than decorative"
        );
    }
}
