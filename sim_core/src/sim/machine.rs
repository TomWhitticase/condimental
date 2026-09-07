//! `Machine` (ARCHITECTURE.md §3.2) and the generic `TimedProcess` pattern
//! (§3.5) it uses to run a recipe over many ticks.

use super::fluid::FluidNetwork;
use super::inventory::Inventory;
use super::recipe::RecipeRegistry;
use super::types::{Fx, RecipeId};

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
    pub recipe: RecipeId,
    pub input: Inventory,
    pub output: Inventory,
    pub process: Option<TimedProcess>,
}

impl Machine {
    pub fn new(recipe: RecipeId, input_capacity: u32, output_capacity: u32) -> Self {
        Self {
            recipe,
            input: Inventory::new(input_capacity),
            output: Inventory::new(output_capacity),
            process: None,
        }
    }

    /// Advances the machine by one tick: starts a new process if it's idle
    /// and has enough input, or deposits output once the timer's elapsed.
    /// Returns whether anything changed (dirty-chunk tracking, §4) — a
    /// process merely ticking toward completion is *not* a change, since
    /// Godot can derive progress itself from `start_tick`/`required_ticks`
    /// plus the current tick count, without a new snapshot every tick.
    ///
    /// `fluid` is the network this machine is plumbed into, if any
    /// (milestone 6 — ARCHITECTURE.md §9 item 6/§3.4). A recipe with
    /// `fluid_inputs` treats it exactly the way it treats its solid
    /// `inputs`: enough of the right fluid must be available *before* a
    /// craft starts, and starting consumes it. A machine that can't get
    /// its fluid simply sits idle — it never runs degraded, never crafts a
    /// partial batch, and never half-consumes its solid input; the same
    /// "stalls, doesn't run degraded" rule power and staff will use later
    /// (GAMEPLAY.md §7). Passing `None` for a recipe that wants fluid is
    /// therefore a permanent stall, which is the correct reading of "this
    /// machine has no pipe connected."
    pub fn tick(
        &mut self,
        current_tick: u64,
        recipes: &RecipeRegistry,
        fluid: Option<&mut FluidNetwork>,
    ) -> bool {
        let mut changed = false;
        let Some(recipe) = recipes.get(&self.recipe) else {
            return false;
        };

        if self.process.is_none() {
            let has_inputs = recipe
                .inputs
                .iter()
                .all(|(item, qty)| self.input.has_at_least(item, *qty));
            // Checked before anything is consumed, so a fluid shortage can
            // never leave the machine having eaten its solid input.
            let has_fluids = match &fluid {
                Some(network) => recipe
                    .fluid_inputs
                    .iter()
                    .all(|(id, units)| network.can_supply(id, Fx::from_num(*units))),
                None => recipe.fluid_inputs.is_empty(),
            };
            if has_inputs && has_fluids {
                for (item, qty) in &recipe.inputs {
                    self.input.remove(item, *qty);
                }
                if let Some(network) = fluid {
                    for (id, units) in &recipe.fluid_inputs {
                        let drawn = network.draw(id, Fx::from_num(*units));
                        debug_assert!(
                            drawn,
                            "fluid availability was checked immediately above, so this draw                              cannot fail without something having mutated the network in between"
                        );
                    }
                }
                self.process = Some(TimedProcess {
                    start_tick: current_tick,
                    required_ticks: recipe.time_ticks,
                });
                changed = true;
            }
        }

        if let Some(process) = self.process {
            if process.is_complete(current_tick) {
                let has_room = recipe
                    .outputs
                    .iter()
                    .all(|(item, qty)| self.output.has_room_for(item, *qty));
                if has_room {
                    for (item, qty) in &recipe.outputs {
                        self.output.add(item, *qty);
                    }
                    self.process = None;
                    changed = true;
                }
                // else: finished but blocked on a full output buffer —
                // retried next tick, same back-pressure idea as the belt.
            }
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::fluid::PipeSegment;

    fn registry() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(
            r#"(
                schema_version: 2,
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
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        machine.input.add("tomato", 2);
        for tick in 0..20 {
            machine.tick(tick, &recipes, None);
        }
        assert!(machine.process.is_none());
        assert_eq!(machine.output.count("tomato_sauce"), 0);
    }

    #[test]
    fn runs_a_full_craft_cycle() {
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes, None); // consumes input, starts process
        assert_eq!(machine.input.count("tomato"), 0);
        assert!(machine.process.is_some());

        for tick in 1..10 {
            machine.tick(tick, &recipes, None);
            assert_eq!(
                machine.output.count("tomato_sauce"),
                0,
                "must not finish early"
            );
        }

        machine.tick(10, &recipes, None); // required_ticks elapsed
        assert_eq!(machine.output.count("tomato_sauce"), 1);
        assert!(machine.process.is_none());
    }

    #[test]
    fn holds_finished_output_when_buffer_is_full() {
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 1);
        machine.output.add("tomato_sauce", 1); // pre-fill the only slot
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes, None);
        for tick in 1..=10 {
            machine.tick(tick, &recipes, None);
        }
        // Blocked: process completed but couldn't deposit, so it must
        // still be holding, not silently dropping the crafted item.
        assert!(machine.process.is_some());
        assert_eq!(machine.output.count("tomato_sauce"), 1);

        machine.output.remove("tomato_sauce", 1);
        machine.tick(11, &recipes, None);
        assert_eq!(machine.output.count("tomato_sauce"), 1);
        assert!(machine.process.is_none());
    }

    /// A recipe that wants a fluid, for the fluid-gating tests below.
    fn fluid_registry() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(
            r#"(
                schema_version: 2,
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

    fn supplied_network() -> FluidNetwork {
        let cap = Fx::from_num(10);
        FluidNetwork::new(
            "vinegar".to_string(),
            vec![
                PipeSegment::new((0, 0), cap, true),
                PipeSegment::new((1, 0), cap, false),
            ],
            1,
        )
    }

    fn empty_network() -> FluidNetwork {
        let cap = Fx::from_num(10);
        FluidNetwork::new(
            "vinegar".to_string(),
            vec![PipeSegment::new((0, 0), cap, false)],
            0,
        )
    }

    #[test]
    fn never_crafts_with_plenty_of_solid_input_but_no_fluid() {
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 10);
        let mut network = empty_network(); // no source: stays empty forever

        for tick in 0..500 {
            network.tick();
            machine.tick(tick, &recipes, Some(&mut network));
        }

        assert!(machine.process.is_none(), "must never start a craft without its fluid");
        assert_eq!(machine.output.count("ketchup"), 0);
        assert_eq!(
            machine.input.count("tomato_sauce"),
            10,
            "a fluid shortage must not consume the solid input either"
        );
    }

    #[test]
    fn never_crafts_when_no_network_is_connected_at_all() {
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 10);

        for tick in 0..500 {
            machine.tick(tick, &recipes, None);
        }

        assert!(machine.process.is_none());
        assert_eq!(machine.output.count("ketchup"), 0);
    }

    #[test]
    fn crafts_and_consumes_fluid_once_the_network_can_supply_it() {
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 1);
        let mut network = supplied_network();
        for _ in 0..10 {
            network.tick(); // let the run fill from its source
        }
        let before = network.inlet().volume;

        machine.tick(0, &recipes, Some(&mut network));
        assert!(machine.process.is_some(), "craft must start once fluid is available");
        assert_eq!(
            network.inlet().volume,
            before - Fx::from_num(2),
            "starting a craft must consume exactly the recipe's fluid quantity"
        );

        for tick in 1..=10 {
            network.tick();
            machine.tick(tick, &recipes, Some(&mut network));
        }
        assert_eq!(machine.output.count("ketchup"), 1);
    }

    #[test]
    fn refuses_a_network_carrying_the_wrong_fluid() {
        let recipes = fluid_registry();
        let mut machine = Machine::new("ketchup".into(), 10, 10);
        machine.input.add("tomato_sauce", 10);
        let mut network = supplied_network();
        network.fluid = "water".to_string();

        for tick in 0..500 {
            network.tick();
            machine.tick(tick, &recipes, Some(&mut network));
        }

        assert!(machine.process.is_none(), "vinegar recipe must not run on a water pipe");
    }
}
