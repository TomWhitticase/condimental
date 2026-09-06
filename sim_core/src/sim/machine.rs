//! `Machine` (ARCHITECTURE.md §3.2) and the generic `TimedProcess` pattern
//! (§3.5) it uses to run a recipe over many ticks.

use super::inventory::Inventory;
use super::recipe::RecipeRegistry;
use super::types::RecipeId;

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
    pub fn tick(&mut self, current_tick: u64, recipes: &RecipeRegistry) -> bool {
        let mut changed = false;
        let Some(recipe) = recipes.get(&self.recipe) else {
            return false;
        };

        if self.process.is_none() {
            let has_inputs = recipe
                .inputs
                .iter()
                .all(|(item, qty)| self.input.has_at_least(item, *qty));
            if has_inputs {
                for (item, qty) in &recipe.inputs {
                    self.input.remove(item, *qty);
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

    fn registry() -> RecipeRegistry {
        RecipeRegistry::from_ron_str(
            r#"(
                schema_version: 1,
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
            machine.tick(tick, &recipes);
        }
        assert!(machine.process.is_none());
        assert_eq!(machine.output.count("tomato_sauce"), 0);
    }

    #[test]
    fn runs_a_full_craft_cycle() {
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 10);
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes); // consumes input, starts process
        assert_eq!(machine.input.count("tomato"), 0);
        assert!(machine.process.is_some());

        for tick in 1..10 {
            machine.tick(tick, &recipes);
            assert_eq!(
                machine.output.count("tomato_sauce"),
                0,
                "must not finish early"
            );
        }

        machine.tick(10, &recipes); // required_ticks elapsed
        assert_eq!(machine.output.count("tomato_sauce"), 1);
        assert!(machine.process.is_none());
    }

    #[test]
    fn holds_finished_output_when_buffer_is_full() {
        let recipes = registry();
        let mut machine = Machine::new("tomato_sauce".into(), 10, 1);
        machine.output.add("tomato_sauce", 1); // pre-fill the only slot
        machine.input.add("tomato", 4);

        machine.tick(0, &recipes);
        for tick in 1..=10 {
            machine.tick(tick, &recipes);
        }
        // Blocked: process completed but couldn't deposit, so it must
        // still be holding, not silently dropping the crafted item.
        assert!(machine.process.is_some());
        assert_eq!(machine.output.count("tomato_sauce"), 1);

        machine.output.remove("tomato_sauce", 1);
        machine.tick(11, &recipes);
        assert_eq!(machine.output.count("tomato_sauce"), 1);
        assert!(machine.process.is_none());
    }
}
