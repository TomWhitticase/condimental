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
//! - **Choosing what a pipe carries.** A network's fluid is still decided by
//!   content, not by the player: it is whatever the recipe of the first
//!   machine tapping it names. A network nothing taps carries `None` and can
//!   neither supply nor accept — a length of pipe with fluid in it and no
//!   customer.
//!
//! **Fluid *outputs* are no longer excluded (the chain-rework epic).** They
//! were, for two milestones: a recipe could consume a fluid and nothing
//! could make one, so the only thing that could add volume to the system was
//! the player filling a tank ([`FluidTank::fill`]). [`FluidNetwork::
//! can_accept_at`]/[`FluidNetwork::deposit_at`] are the producer-side mirror
//! of `can_supply_at`/`draw_at`, with the same all-or-nothing semantics for
//! the same reason: a craft must not half-deposit its output any more than
//! it may half-consume its input. The conservation property in
//! [`FluidNetwork::tick`]'s doc is unchanged in shape — there are simply two
//! ways in (a tank fill, a machine deposit) and one way out (a machine
//! draw), all of them exact.
//!
//! **What the "Real Placement" epic changed here.** Two further exclusions
//! this list used to carry are now real behaviour instead:
//!
//! - **Pipes are player-placed**, so the topology changes at runtime. A
//!   network is therefore no longer a `Vec` whose adjacency is "consecutive
//!   indices": it is a genuine graph — `links` holds the adjacent index
//!   pairs, derived from tile adjacency when the world's topology changes
//!   and never again (ARCHITECTURE.md §3.8's "once per topology change, not
//!   every tick"). Flow is **undirected**: a pipe tile's `direction` is what
//!   it looks like, not which way fluid goes, because §3.4's model is a
//!   pressure system and pressure does not care which way a fitting was
//!   turned.
//! - **There can be several networks**, one per connected component of pipe
//!   tiles, each with its own fluid and its own tank(s). `World` owns the
//!   list and rebuilds it on a topology change.
//!
//! **The tank is no longer owned by the network**, which is the other
//! consequence of a topology that can change. A network is *derived* — it is
//! rebuilt from scratch whenever a pipe is placed, removed or rotated — and a
//! player's tank full of vinegar must not be rebuilt away, so the tank lives
//! on its own placed entity in `world.rs` and the network merely knows which
//! segment it discharges into ([`FluidNetwork::discharge_from_tank`]). Pipe
//! *volumes* do survive a recompute, carried across by tile; a pipe that is
//! removed spills what it held, which is the honest reading of picking a
//! pipe up.
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
//! it *does* share with a segment is the flow rule:
//! [`FluidNetwork::discharge_from_tank`] and [`FluidNetwork::tick`] both run
//! the identical pressure-difference formula ([`equalize`]), so there is one
//! flow mechanism in this file, not two.
//!
//! All volume state is `Fx` fixed-point (ARCHITECTURE.md §3.9) — no `f32`/
//! `f64` appears anywhere in this file. Floats only appear one layer out,
//! in `world.rs`'s presentation snapshots, the same one-way conversion
//! `BeltSnapshot` already makes.

use super::types::{Direction, Fx, FluidId, TileCoord};

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
    /// **A supply end: a tile some [`FluidTank`] discharges into.** The name
    /// is unchanged from milestone 6 (presentation keys a different
    /// silhouette off it, and renaming it would break that for no gain), but
    /// its meaning has narrowed twice. The "Visual overhaul round 2" epic's
    /// ticket 1 stopped it meaning "an off-map tap that refills itself" and
    /// made it mean "the tank is on the other side of this tile"; the "Real
    /// Placement" epic made it *derived* — the topology pass sets it on
    /// whichever tile a placed tank happens to face, so it moves when the
    /// player moves the tank, and a run with no tank has none at all.
    pub is_source: bool,
    /// Which way this pipe tile was placed. Presentation only: flow is
    /// undirected (see the module doc), so nothing in this file reads it —
    /// it is carried here so a snapshot can report the rotation the player
    /// chose without a second lookup.
    pub direction: Direction,
}

impl PipeSegment {
    /// A new, **empty** segment facing `direction` — the pipe filling up is
    /// observable behaviour driven by a tank, not an initial-state fiction.
    /// `is_source` is not an argument because it is not a property of the
    /// pipe: the topology pass sets it from whether a tank faces this tile.
    pub fn new(tile: TileCoord, direction: Direction, capacity: Fx) -> Self {
        Self {
            tile,
            volume: Fx::ZERO,
            capacity,
            is_source: false,
            direction,
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

    /// Rebuilds a tank already holding `volume` — the save/load module's
    /// seam (`sim::save`). `pub(crate)`, and taking an [`Fx`] rather than
    /// [`FluidTank::fill`]'s whole `u32` units, because a tank that has been
    /// discharging holds a *fractional* volume that no player command could
    /// ever produce: `fill` is the player's way in, this is the loader's.
    /// The volume is clamped into `[0, capacity]` so a hand-edited save
    /// cannot create a tank holding more than it can.
    pub(crate) fn restore(capacity: Fx, volume: Fx) -> Self {
        Self {
            volume: volume.max(Fx::ZERO).min(capacity),
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
/// (ARCHITECTURE.md §3.2's `FluidNetwork`).
///
/// **Derived, never authored.** Since the "Real Placement" epic a network is
/// rebuilt by `world.rs` from whatever pipe tiles are currently placed, every
/// time the world's topology changes — so nothing outside this file should
/// hold on to one across a `Place`/`Remove`/`Rotate`. What survives a rebuild
/// is the per-tile *volume* (carried across by tile) and the tanks (which are
/// placed entities of their own, not part of a network).
#[derive(Debug, Clone)]
pub struct FluidNetwork {
    /// What this network carries, or `None` when nothing taps it.
    ///
    /// Decided by content rather than by the player: it is the fluid named
    /// by the recipe of a fluid-consuming machine adjacent to one of these
    /// segments (`world.rs` picks it when it derives the network). A
    /// `None` network still holds volume — a player can fill a tank into a
    /// pipe run that feeds nothing — it simply cannot supply anybody, since
    /// [`FluidNetwork::can_supply_at`] compares against a named fluid and
    /// an unnamed network matches nothing.
    pub fluid: Option<FluidId>,
    segments: Vec<PipeSegment>,
    /// Adjacent index pairs, `(a, b)` with `a < b`, in a deterministic
    /// order — the graph edges of this connected component. Computed once
    /// per topology change (ARCHITECTURE.md §3.8), never per tick.
    links: Vec<(usize, usize)>,
}

impl FluidNetwork {
    /// Panics if `segments` is empty or a link indexes out of range — the
    /// same "clear panic at construction time, not a silently-stuck
    /// simulation" stance the rest of this crate takes. Both are internal
    /// invariants of the topology pass in `world.rs`, not player input.
    pub fn new(fluid: Option<FluidId>, segments: Vec<PipeSegment>, links: Vec<(usize, usize)>) -> Self {
        assert!(
            !segments.is_empty(),
            "a FluidNetwork must have at least one pipe segment"
        );
        for &(a, b) in &links {
            assert!(
                a < segments.len() && b < segments.len(),
                "FluidNetwork link ({a}, {b}) is out of range for {} segment(s)",
                segments.len()
            );
        }
        Self {
            fluid,
            segments,
            links,
        }
    }

    pub fn segments(&self) -> &[PipeSegment] {
        &self.segments
    }

    pub fn links(&self) -> &[(usize, usize)] {
        &self.links
    }

    /// Flags the segment at `index` as a supply end — a tile some tank
    /// discharges into. Called by the topology pass in `world.rs` once per
    /// rebuild; see [`PipeSegment::is_source`] for what a renderer does
    /// with it.
    pub fn mark_source(&mut self, index: usize) {
        if let Some(segment) = self.segments.get_mut(index) {
            segment.is_source = true;
        }
    }

    /// Sets one segment's volume outright — the save/load module's seam
    /// (`sim::save`), reached through `world.rs` because a network is
    /// derived and a loader addresses a pipe by *tile*, not by index.
    /// `pub(crate)`: volume otherwise only ever moves through
    /// [`Self::tick`], [`Self::draw_at`], [`Self::deposit_at`] and
    /// [`Self::discharge_from_tank`], and it should stay that way for
    /// anything a player can do. Clamped into `[0, capacity]`.
    pub(crate) fn restore_volume_at(&mut self, index: usize, volume: Fx) {
        if let Some(segment) = self.segments.get_mut(index) {
            segment.volume = volume.max(Fx::ZERO).min(segment.capacity);
        }
    }

    /// Which segment sits on `tile`, if any — how `world.rs` turns a tile
    /// (a tank's discharge target, a machine's tap) into an index into this
    /// network.
    pub fn index_of_tile(&self, tile: TileCoord) -> Option<usize> {
        self.segments.iter().position(|s| s.tile == tile)
    }

    /// Total volume held across this network's pipes — §3.4's "each
    /// `FluidNetwork` has a total volume and a fluid type."
    ///
    /// Deliberately excludes any tank feeding it: a tank is a building with
    /// its own capacity and its own snapshot, not a length of pipe. Ask the
    /// tank for the tank's contents.
    pub fn total_volume(&self) -> Fx {
        self.segments.iter().fold(Fx::ZERO, |acc, s| acc + s.volume)
    }

    pub fn total_capacity(&self) -> Fx {
        self.segments
            .iter()
            .fold(Fx::ZERO, |acc, s| acc + s.capacity)
    }

    /// Whether the segment at `index` can currently supply `amount` of
    /// `fluid`. Wrong fluid type — including an untapped network's `None` —
    /// is a hard "no", never an implicit conversion.
    ///
    /// Indexed rather than network-wide because a machine draws from the
    /// **one segment it is adjacent to**, which is what makes the
    /// equalization pass load-bearing rather than cosmetic: drain that tile
    /// and the machine stalls until pressure carries more fluid to it.
    pub fn can_supply_at(&self, index: usize, fluid: &str, amount: Fx) -> bool {
        self.fluid.as_deref() == Some(fluid)
            && self
                .segments
                .get(index)
                .is_some_and(|segment| segment.volume >= amount)
    }

    /// Removes `amount` of `fluid` from the segment at `index`, or returns
    /// `false` and changes nothing. All-or-nothing on purpose: a machine
    /// must not start a craft on a partial fluid charge, exactly as it must
    /// not start on partial solid input.
    pub fn draw_at(&mut self, index: usize, fluid: &str, amount: Fx) -> bool {
        if !self.can_supply_at(index, fluid, amount) {
            return false;
        }
        self.segments[index].volume -= amount;
        true
    }

    /// Whether the segment at `index` has room for `amount` more of
    /// `fluid` — the producer-side mirror of [`Self::can_supply_at`], and
    /// deliberately its exact shape.
    ///
    /// Wrong fluid type is a hard "no", and so is an **unnamed** network:
    /// `None` matches nothing here just as it supplies nobody there. That is
    /// not a gap, it is the same rule read from the other side — a network's
    /// fluid is derived by the topology pass from whichever machine taps it
    /// (`world.rs`), including a machine that taps it to *deposit*, so by
    /// the time any deposit is attempted the network has already been named.
    /// A network that is still `None` is one nothing taps, and a machine
    /// with nowhere to put its output simply holds it (see
    /// [`super::machine::Machine::tick`]).
    ///
    /// Indexed, like its mirror, because a machine deposits into the **one
    /// segment it discharges against** rather than into a network total:
    /// fill that tile and the machine backs up until pressure carries the
    /// charge away down the run, which is what makes a full pipe a real
    /// back-pressure state rather than a cosmetic one.
    pub fn can_accept_at(&self, index: usize, fluid: &str, amount: Fx) -> bool {
        self.fluid.as_deref() == Some(fluid)
            && self
                .segments
                .get(index)
                .is_some_and(|segment| segment.room() >= amount)
    }

    /// Adds `amount` of `fluid` to the segment at `index`, or returns
    /// `false` and changes nothing — [`Self::draw_at`]'s mirror, all-or-
    /// nothing for its reason.
    pub fn deposit_at(&mut self, index: usize, fluid: &str, amount: Fx) -> bool {
        if !self.can_accept_at(index, fluid, amount) {
            return false;
        }
        self.segments[index].volume += amount;
        true
    }

    /// Equalizes `tank` against the segment at `into` — the tank half of the
    /// per-tick pass, run by `world.rs` immediately before [`Self::tick`]
    /// because the tank is a placed entity this network does not own.
    ///
    /// The tank enters the shared flow rule with its
    /// [`discharge_pressure`](FluidTank::discharge_pressure) rather than its
    /// fill fraction — it is a raised vessel, see that method for why. The
    /// rule itself ([`equalize`]) is symmetric and unchanged; the asymmetry
    /// is entirely in what pressure each side presents.
    pub fn discharge_from_tank(&mut self, tank: &mut FluidTank, into: usize) -> bool {
        let Some(head) = self.segments.get_mut(into) else {
            return false;
        };
        let rate = flow_rate();
        let tank_pressure = tank.discharge_pressure();
        let tank_capacity = tank.capacity;
        let head_pressure = head.pressure();
        equalize(
            Side {
                volume: &mut tank.volume,
                pressure: tank_pressure,
                capacity: tank_capacity,
            },
            Side {
                volume: &mut head.volume,
                pressure: head_pressure,
                capacity: head.capacity,
            },
            rate,
        )
    }

    /// Advances the network one tick: equalize pressure across every
    /// adjacent pipe pair. Returns whether any volume actually moved (used
    /// the same way `Machine::tick`'s return value is — "did anything a
    /// viewer would notice change").
    ///
    /// The pass is sequential (each pair sees the result of the pair before
    /// it) and its order is fixed — `links` is built in a deterministic
    /// order by the topology pass and never reordered — so it is
    /// deterministic, which is the property ARCHITECTURE.md §3.9 actually
    /// requires. Volume is conserved exactly: every transfer is clamped to
    /// what the donor has and what the receiver has room for and applied as
    /// a paired subtract/add, so nothing is created or destroyed here. The
    /// only source in the whole system is [`FluidTank::fill`] (reaching the
    /// pipes through [`Self::discharge_from_tank`]), and the only sink is
    /// [`Self::draw_at`].
    pub fn tick(&mut self) -> bool {
        let rate = flow_rate();
        let mut changed = false;
        for k in 0..self.links.len() {
            let (i, j) = self.links[k];
            debug_assert!(i != j, "a pipe tile cannot be adjacent to itself");
            let (lo, hi) = if i < j { (i, j) } else { (j, i) };
            let (left, right) = self.segments.split_at_mut(hi);
            let a = &mut left[lo];
            let b = &mut right[0];
            let (a_pressure, b_pressure) = (a.pressure(), b.pressure());
            let moved = equalize(
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
            changed |= moved;
        }
        changed
    }
}

/// The one machine-facing view of a network: which network, and which
/// segment that machine is plumbed into.
///
/// It exists because a machine draws from (or deposits into) the single pipe
/// tile it touches rather than a network total (see
/// [`FluidNetwork::can_supply_at`]), and because
/// [`super::machine::Machine::tick`] should not have to be told that fact
/// twice. `world.rs` builds one per *role* per machine when it derives the
/// topology: an **input** tap for a recipe's `fluid_inputs` and a separate
/// **output** tap for its `fluid_outputs`. The two are always different
/// networks — the topology pass refuses to hand a machine the same one
/// twice — which is what lets a machine hold a mutable borrow of both at
/// once.
///
/// The field is still called `inlet` for both roles. Renaming it per role
/// would need two types (or a direction flag) for one `usize`, and the type
/// already says which role it is by which argument it was passed as.
pub struct FluidTap<'a> {
    pub network: &'a mut FluidNetwork,
    /// Index into `network.segments()` of the tile this machine touches.
    pub inlet: usize,
}

impl FluidTap<'_> {
    pub fn can_supply(&self, fluid: &str, amount: Fx) -> bool {
        self.network.can_supply_at(self.inlet, fluid, amount)
    }

    pub fn draw(&mut self, fluid: &str, amount: Fx) -> bool {
        self.network.draw_at(self.inlet, fluid, amount)
    }

    pub fn can_accept(&self, fluid: &str, amount: Fx) -> bool {
        self.network.can_accept_at(self.inlet, fluid, amount)
    }

    pub fn deposit(&mut self, fluid: &str, amount: Fx) -> bool {
        self.network.deposit_at(self.inlet, fluid, amount)
    }
}

fn flow_rate() -> Fx {
    Fx::from_num(FLOW_RATE_NUMERATOR) / Fx::from_num(FLOW_RATE_DENOMINATOR)
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
    use crate::sim::types::Direction;

    fn cap() -> Fx {
        Fx::from_num(10)
    }

    fn tank_cap() -> Fx {
        Fx::from_num(FLUID_TANK_CAPACITY)
    }

    /// A tank plus the straight run it feeds, held together — the shape
    /// `world.rs`'s topology pass derives from a tank placed at the head of
    /// a line of pipe tiles.
    ///
    /// It exists because the "Real Placement" epic moved the tank *out* of
    /// [`FluidNetwork`] (a network is rebuilt whenever the player places a
    /// pipe, and a tank full of vinegar must not be rebuilt away), so a test
    /// about "the tank and its run" now has to hold both halves itself. Its
    /// methods keep the shape the old API had, which is why every assertion
    /// below is unchanged.
    struct Run {
        net: FluidNetwork,
        tank: FluidTank,
        inlet: usize,
    }

    impl Run {
        /// One tick of the pair, in the order `World::tick` runs it: the
        /// tank discharges into the run's head, then the run equalizes.
        fn tick(&mut self) -> bool {
            let discharged = self.net.discharge_from_tank(&mut self.tank, 0);
            let flowed = self.net.tick();
            discharged || flowed
        }

        fn fill_tank(&mut self, amount: u32) -> Fx {
            self.tank.fill(amount)
        }

        fn tank(&self) -> &FluidTank {
            &self.tank
        }

        /// The segment a machine would tap: the far end of the run.
        fn inlet(&self) -> &PipeSegment {
            &self.net.segments()[self.inlet]
        }

        fn draw(&mut self, fluid: &str, amount: Fx) -> bool {
            self.net.draw_at(self.inlet, fluid, amount)
        }

        fn total_volume(&self) -> Fx {
            self.net.total_volume()
        }

        fn total_capacity(&self) -> Fx {
            self.net.total_capacity()
        }

        fn segments(&self) -> &[PipeSegment] {
            self.net.segments()
        }
    }

    /// A straight run of `pipes + 1` segments fed by a full-size tank at
    /// index 0; the last segment is the inlet.
    fn run(pipes: usize) -> Run {
        let mut segments = vec![PipeSegment::new((0, 0), Direction::East, cap())];
        for i in 0..pipes {
            segments.push(PipeSegment::new((i as i32 + 1, 0), Direction::East, cap()));
        }
        let inlet = segments.len() - 1;
        let links = (0..segments.len() - 1).map(|i| (i, i + 1)).collect();
        let mut net = FluidNetwork::new(Some("vinegar".to_string()), segments, links);
        net.mark_source(0);
        Run {
            net,
            tank: FluidTank::new(tank_cap()),
            inlet,
        }
    }

    /// A run with hand-set segment volumes and a zero-capacity tank, for
    /// testing the pipe-to-pipe half of the pass in isolation: a tank that
    /// can neither give nor take drops out of the pass entirely, which is
    /// exactly what "no source" means here.
    fn sourceless(volumes: &[i32]) -> Run {
        let segments: Vec<PipeSegment> = volumes
            .iter()
            .enumerate()
            .map(|(i, v)| PipeSegment {
                tile: (i as i32, 0),
                volume: Fx::from_num(*v),
                capacity: cap(),
                is_source: false,
                direction: Direction::East,
            })
            .collect();
        let inlet = segments.len() - 1;
        let links = (0..segments.len() - 1).map(|i| (i, i + 1)).collect();
        Run {
            net: FluidNetwork::new(Some("vinegar".to_string()), segments, links),
            tank: FluidTank::new(Fx::ZERO),
            inlet,
        }
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
