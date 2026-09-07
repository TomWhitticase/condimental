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
//!   future work, so nothing here can add volume except the source refill.
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
    /// An off-map supply tap: refilled to `capacity` at the start of every
    /// network tick. This is milestone 6's stand-in for an ingredient the
    /// player would eventually buy/produce (GAMEPLAY.md §5's recurring
    /// ingredient cost) — the fluid has to come from *somewhere* to prove
    /// the mechanic, and an always-refilling source is the smallest thing
    /// that isn't a lie about where it came from.
    pub is_source: bool,
}

impl PipeSegment {
    pub fn new(tile: TileCoord, capacity: Fx, is_source: bool) -> Self {
        Self {
            tile,
            // A source starts full; a plain pipe starts empty and has to be
            // filled by the equalization pass, so "the pipe run fills up"
            // is observable behavior rather than an initial-state fiction.
            volume: if is_source { capacity } else { Fx::ZERO },
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
    /// the run from the source.
    inlet_index: usize,
}

impl FluidNetwork {
    /// Panics if `segments` is empty or `inlet_index` is out of range —
    /// same "clear panic at construction time, not a silently-stuck
    /// simulation" stance `world::validate_chain` takes.
    pub fn new(fluid: FluidId, segments: Vec<PipeSegment>, inlet_index: usize) -> Self {
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
        }
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

    /// Total volume held across the whole network — §3.4's "each
    /// `FluidNetwork` has a total volume and a fluid type."
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

    /// Advances the network one tick: refill every source segment, then
    /// equalize pressure across each adjacent pair. Returns whether any
    /// volume actually moved (used the same way `Machine::tick`'s return
    /// value is — "did anything a viewer would notice change").
    ///
    /// The equalization pass is sequential (each pair sees the results of
    /// the pair before it), which makes flow travel a full run in one tick
    /// in the pass direction. That is order-dependent by construction, and
    /// the order is fixed (`0..len-1`, always) — so it is deterministic,
    /// which is the property ARCHITECTURE.md §3.9 actually requires.
    /// Volume is conserved exactly: every transfer is clamped to what the
    /// donor has and what the receiver has room for, and is applied as a
    /// paired subtract/add, so nothing is ever created or destroyed here
    /// (the only sources/sinks in the whole system are the source refill
    /// above and `draw`).
    pub fn tick(&mut self) -> bool {
        let mut changed = false;

        for segment in self.segments.iter_mut() {
            if segment.is_source && segment.volume < segment.capacity {
                segment.volume = segment.capacity;
                changed = true;
            }
        }

        let rate = Fx::from_num(FLOW_RATE_NUMERATOR) / Fx::from_num(FLOW_RATE_DENOMINATOR);
        for i in 0..self.segments.len().saturating_sub(1) {
            let (left, right) = self.segments.split_at_mut(i + 1);
            let a = &mut left[i];
            let b = &mut right[0];

            let scale = if a.capacity < b.capacity {
                a.capacity
            } else {
                b.capacity
            };
            // Signed: positive flows a -> b, negative flows b -> a.
            let desired = (a.pressure() - b.pressure()) * rate * scale;
            if desired == Fx::ZERO {
                continue;
            }

            if desired > Fx::ZERO {
                let flow = desired.min(a.volume).min(b.room());
                if flow > Fx::ZERO {
                    a.volume -= flow;
                    b.volume += flow;
                    changed = true;
                }
            } else {
                let flow = (-desired).min(b.volume).min(a.room());
                if flow > Fx::ZERO {
                    b.volume -= flow;
                    a.volume += flow;
                    changed = true;
                }
            }
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap() -> Fx {
        Fx::from_num(10)
    }

    /// A source at index 0 followed by `pipes` plain segments; the last
    /// segment is the inlet — the same shape `World` builds.
    fn run(pipes: usize) -> FluidNetwork {
        let mut segments = vec![PipeSegment::new((0, 0), cap(), true)];
        for i in 0..pipes {
            segments.push(PipeSegment::new((i as i32 + 1, 0), cap(), false));
        }
        let inlet = segments.len() - 1;
        FluidNetwork::new("vinegar".to_string(), segments, inlet)
    }

    #[test]
    fn a_source_segment_refills_itself_every_tick() {
        let mut net = run(0);
        net.segments[0].volume = Fx::ZERO;
        assert!(net.tick());
        assert_eq!(net.segments()[0].volume, cap());
    }

    #[test]
    fn pressure_equalizes_between_two_adjacent_segments() {
        // No source involved: a full segment beside an empty one must even
        // out, which is §3.4's entire model in its smallest form.
        let segments = vec![
            PipeSegment {
                tile: (0, 0),
                volume: cap(),
                capacity: cap(),
                is_source: false,
            },
            PipeSegment::new((1, 0), cap(), false),
        ];
        let mut net = FluidNetwork::new("vinegar".to_string(), segments, 1);

        net.tick();

        let (a, b) = (net.segments()[0].volume, net.segments()[1].volume);
        assert_eq!(a, b, "one tick at FLOW_RATE 0.5 fully equalizes a pair");
        assert_eq!(a, cap() / 2);
    }

    #[test]
    fn equalization_conserves_total_volume_exactly() {
        let segments = vec![
            PipeSegment {
                tile: (0, 0),
                volume: Fx::from_num(7),
                capacity: cap(),
                is_source: false,
            },
            PipeSegment::new((1, 0), cap(), false),
            PipeSegment {
                tile: (2, 0),
                volume: Fx::from_num(3),
                capacity: cap(),
                is_source: false,
            },
        ];
        let mut net = FluidNetwork::new("vinegar".to_string(), segments, 2);
        let before = net.total_volume();

        for _ in 0..100 {
            net.tick();
        }

        assert_eq!(
            net.total_volume(),
            before,
            "no source, no draw: total volume must be conserved exactly (fixed-point, so this is \
             an exact equality, not an epsilon comparison)"
        );
    }

    #[test]
    fn a_sourceless_network_settles_to_even_pressure_and_stops_changing() {
        let segments = vec![
            PipeSegment {
                tile: (0, 0),
                volume: cap(),
                capacity: cap(),
                is_source: false,
            },
            PipeSegment::new((1, 0), cap(), false),
            PipeSegment::new((2, 0), cap(), false),
        ];
        let mut net = FluidNetwork::new("vinegar".to_string(), segments, 2);

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
    fn fluid_reaches_the_far_inlet_of_a_run_from_the_source() {
        let mut net = run(3);
        assert_eq!(net.inlet().volume, Fx::ZERO, "inlet starts empty");

        for _ in 0..50 {
            net.tick();
        }

        assert!(
            net.inlet().volume > Fx::ZERO,
            "pressure from the source must carry fluid down the run to the inlet"
        );
    }

    #[test]
    fn draw_is_all_or_nothing_and_refuses_the_wrong_fluid() {
        let mut net = run(2);
        for _ in 0..50 {
            net.tick();
        }
        let available = net.inlet().volume;

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
    fn a_drained_inlet_refills_from_the_source_over_the_following_ticks() {
        let mut net = run(2);
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
