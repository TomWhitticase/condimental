//! The chunk DELTA (the quality pass, 2026-09-13): `World::chunk_delta` is
//! `World::chunk_snapshot` filtered to what changed since the dirty set was
//! last cleared, and the whole snapshot after a structural change. It
//! exists because a running line dirties every chunk every tick, and the
//! full snapshot re-sent every belt in the chunk — most of them empty — to
//! the presentation layer thirty times a second (ARCHITECTURE.md §4).
//!
//! Also here: `World::revision`, the tick-plus-command counter a poller
//! compares to skip frames on which nothing on the wire can have changed.

mod common;

use sim_core::sim::{Direction, PlayerCommand, RecipeRegistry, World, reference_chain_with_budget};

const BUDGET: u32 = 400 + common::WAGE_RESERVE;

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

/// A fed reference chain, ticked until product is on the belts.
fn running_chain() -> World {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    common::buy(&mut world, "tomato", 40);
    common::buy(&mut world, "sugar", 40);
    world.issue_command(PlayerCommand::FillHopper { amount: 40, tile: None });
    for _ in 0..60 {
        world.tick();
    }
    world
}

#[test]
fn a_delta_carries_exactly_the_belts_whose_snapshot_changed_and_no_pipes() {
    let mut world = running_chain();
    let chunk = (0, 0);
    let before = world.chunk_snapshot(chunk);
    world.tick();
    assert!(world.dirty_chunks().contains(&chunk), "a running line dirties its chunk every tick");
    let after = world.chunk_snapshot(chunk);
    let delta = world.chunk_delta(chunk);

    assert!(!delta.belts.is_empty(), "something moved this tick");
    assert!(
        delta.belts.len() < after.belts.len(),
        "the delta must be smaller than the chunk: {} of {} belts moved",
        delta.belts.len(),
        after.belts.len()
    );
    // Tile by tile: a belt is in the delta IF AND ONLY IF its snapshot
    // differs from the previous tick's — and the entry is that snapshot.
    for now in &after.belts {
        let then = before.belts.iter().find(|b| b.tile == now.tile).expect("no structural change");
        let changed = serde_json::to_string(then).unwrap() != serde_json::to_string(now).unwrap();
        let sent = delta.belts.iter().find(|b| b.tile == now.tile);
        assert_eq!(sent.is_some(), changed, "belt {:?}: changed {changed}, sent {}", now.tile, sent.is_some());
        if let Some(sent) = sent {
            assert_eq!(serde_json::to_string(sent).unwrap(), serde_json::to_string(now).unwrap());
        }
    }
    assert!(delta.pipes.is_empty(), "fluid is polled, a delta never carries a pipe");
}

#[test]
fn a_jammed_line_sends_nothing() {
    // An unfed chain: after the tick that clears the build's dirt, every
    // tick moves nothing and the delta of every chunk is empty.
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    world.tick();
    world.tick();
    assert!(world.dirty_chunks().is_empty());
    for chunk in world.all_chunks() {
        let delta = world.chunk_delta(chunk);
        assert!(delta.belts.is_empty() && delta.machines.is_empty() && delta.hoppers.is_empty());
    }
}

#[test]
fn a_structural_change_makes_the_delta_the_whole_chunk() {
    let mut world = running_chain();
    world.tick();
    // A belt placed somewhere empty in the line's chunk.
    world.issue_command(PlayerCommand::Purchase { kind: "belt".into(), quantity: 1 });
    world.issue_command(PlayerCommand::Place {
        kind: "belt".into(),
        tile: (5, 20),
        direction: Direction::East,
        item: None,
    });
    assert_eq!(world.world_layout().iter().filter(|p| p.tile == (5, 20)).count(), 1, "the belt was placed");
    let chunk = (0, 0);
    let full = world.chunk_snapshot(chunk);
    let delta = world.chunk_delta(chunk);
    assert_eq!(
        serde_json::to_string(&full).unwrap(),
        serde_json::to_string(&delta).unwrap(),
        "after a place the delta is the full snapshot: the renderer holds no last state for the new tile"
    );
    // The next tick forgets the structural mark and the delta shrinks again.
    world.tick();
    assert!(world.chunk_delta(chunk).belts.len() < world.chunk_snapshot(chunk).belts.len());
}

#[test]
fn a_hopper_that_unloads_is_in_the_delta_and_one_that_idles_is_not() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    common::buy(&mut world, "tomato", 4);
    world.issue_command(PlayerCommand::FillHopper { amount: 4, tile: None });
    world.tick();
    // Every hopper is in the full snapshot of its chunk; only the tomato
    // hopper, which unloaded this tick, is in the delta.
    let mut in_full = 0;
    let mut in_delta = 0;
    for chunk in world.all_chunks() {
        in_full += world.chunk_snapshot(chunk).hoppers.len();
        in_delta += world.chunk_delta(chunk).hoppers.len();
    }
    assert!(in_full >= 2, "the shipped line has two hoppers");
    assert_eq!(in_delta, 1, "only the hopper whose level changed is sent");
}

#[test]
fn revision_counts_ticks_and_commands_including_refused_ones() {
    let mut world = reference_chain_with_budget(recipes(), BUDGET);
    let start = world.revision();
    world.tick();
    assert_eq!(world.revision(), start + 1);
    // A refused command (nothing is owned of this kind at this tile) still
    // bumps it: the hotbar has to re-read to learn the refusal.
    world.issue_command(PlayerCommand::Remove { tile: (99, 99) });
    assert_eq!(world.revision(), start + 2);
    let before = world.revision();
    let _ = world.chunk_snapshot((0, 0));
    let _ = world.economy_snapshot();
    assert_eq!(world.revision(), before, "a read never moves it");
}
