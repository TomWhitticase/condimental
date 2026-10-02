//! Headless integration test for `World::preview_command` (the belt/pipe
//! crossing epic, ticket T3a): the build ghost's question "would this
//! command succeed, and what would the tile become?", answered by the sim.
//!
//! Driven through the public command API only, like every file here. What
//! it pins:
//!
//! 1. **Preview and command agree, case by case.** A grid of `Place` and
//!    `Purchase` commands — crossings, parallel refusals, overlaps, hopper
//!    items, nothing owned, unaffordable — each goes through
//!    `preview_command` and then the real `issue_command` on the same world,
//!    and `valid` must equal "the world changed", with `result_kind` the kind
//!    that then stands on the tile. This is the anti-drift test: a ghost
//!    that goes green on a placement the sim refuses is worse than no ghost.
//! 2. **A preview mutates nothing** — not the save, not `revision()`.
//! 3. **An uncovered command is refused, never guessed valid.**
//! 4. **The wire shape** Godot reads: `valid`, `reason`, `result_kind`.

use sim_core::sim::machines::MachineRegistry;
use sim_core::sim::types::{Money, TileCoord};
use sim_core::sim::{
    BELT_KIND, CROSSING_KIND, ContractRegistry, Direction, PIPE_KIND, PlayerCommand, PreviewResult,
    RecipeRegistry, SPLITTER_KIND, World, command_from_json,
};

fn recipes() -> RecipeRegistry {
    RecipeRegistry::from_ron_str(include_str!("../../data/recipes.ron"))
        .expect("bundled data/recipes.ron must parse")
}

fn world_with(money: i64) -> World {
    World::with_content_and_money(
        recipes(),
        MachineRegistry::bundled(),
        ContractRegistry::bundled(),
        Money::from_num(money),
    )
}

fn buy(world: &mut World, kind: &str) {
    let before = world.owned_count(kind);
    world.issue_command(PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity: 1,
    });
    assert_eq!(
        world.owned_count(kind),
        before + 1,
        "the fixture must buy {kind}"
    );
}

fn place(kind: &str, tile: TileCoord, direction: Direction, item: Option<&str>) -> PlayerCommand {
    PlayerCommand::Place {
        kind: kind.to_string(),
        tile,
        direction,
        item: item.map(str::to_string),
    }
}

fn build(world: &mut World, kind: &str, tile: TileCoord, direction: Direction) {
    buy(world, kind);
    world.issue_command(place(kind, tile, direction, None));
    assert_eq!(
        kind_at(world, tile).as_deref(),
        Some(kind),
        "the fixture must place {kind}"
    );
}

fn kind_at(world: &World, tile: TileCoord) -> Option<String> {
    world
        .world_layout()
        .into_iter()
        .find(|p| p.occupies.contains(&tile))
        .map(|p| p.kind)
}

const BELT_TILE: TileCoord = (0, 0);
const PIPE_TILE: TileCoord = (4, 0);
const CRUSHER_TILE: TileCoord = (8, 0);
const CROSSING_TILE: TileCoord = (0, 4);
const EMPTY_TILE: TileCoord = (20, 20);

/// A funded world with an east belt, a north pipe, a crusher and a
/// crossing already standing, and nothing else owned.
fn scene() -> World {
    let mut world = world_with(50_000);
    build(&mut world, BELT_KIND, BELT_TILE, Direction::East);
    build(&mut world, PIPE_KIND, PIPE_TILE, Direction::North);
    build(&mut world, "crusher", CRUSHER_TILE, Direction::East);
    build(&mut world, BELT_KIND, CROSSING_TILE, Direction::East);
    buy(&mut world, PIPE_KIND);
    world.issue_command(place(PIPE_KIND, CROSSING_TILE, Direction::North, None));
    assert_eq!(
        kind_at(&world, CROSSING_TILE).as_deref(),
        Some(CROSSING_KIND)
    );
    world
}

/// One case: what to own before the command, the command, and what the
/// preview is expected to say (`Some(kind)` = valid, ending up as `kind`).
struct Case {
    name: &'static str,
    own: Option<&'static str>,
    cmd: PlayerCommand,
    expect: Option<&'static str>,
}

fn case(
    name: &'static str,
    own: Option<&'static str>,
    cmd: PlayerCommand,
    expect: Option<&'static str>,
) -> Case {
    Case {
        name,
        own,
        cmd,
        expect,
    }
}

fn place_cases() -> Vec<Case> {
    use Direction::{East, North, South, West};
    vec![
        // The crossing rule, both ways round.
        case(
            "pipe across a belt",
            Some(PIPE_KIND),
            place(PIPE_KIND, BELT_TILE, North, None),
            Some(CROSSING_KIND),
        ),
        case(
            "pipe across a belt, other way",
            Some(PIPE_KIND),
            place(PIPE_KIND, BELT_TILE, South, None),
            Some(CROSSING_KIND),
        ),
        case(
            "pipe along a belt",
            Some(PIPE_KIND),
            place(PIPE_KIND, BELT_TILE, East, None),
            None,
        ),
        case(
            "pipe against a belt",
            Some(PIPE_KIND),
            place(PIPE_KIND, BELT_TILE, West, None),
            None,
        ),
        case(
            "belt across a pipe",
            Some(BELT_KIND),
            place(BELT_KIND, PIPE_TILE, East, None),
            Some(CROSSING_KIND),
        ),
        case(
            "belt along a pipe",
            Some(BELT_KIND),
            place(BELT_KIND, PIPE_TILE, North, None),
            None,
        ),
        case(
            "belt against a pipe",
            Some(BELT_KIND),
            place(BELT_KIND, PIPE_TILE, South, None),
            None,
        ),
        case(
            "pipe across a belt, none owned",
            None,
            place(PIPE_KIND, BELT_TILE, North, None),
            None,
        ),
        // Everything else on an occupied tile is an overlap.
        case(
            "belt onto a belt",
            Some(BELT_KIND),
            place(BELT_KIND, BELT_TILE, North, None),
            None,
        ),
        case(
            "pipe onto a pipe",
            Some(PIPE_KIND),
            place(PIPE_KIND, PIPE_TILE, East, None),
            None,
        ),
        case(
            "pipe onto a crossing",
            Some(PIPE_KIND),
            place(PIPE_KIND, CROSSING_TILE, East, None),
            None,
        ),
        case(
            "belt onto a crossing",
            Some(BELT_KIND),
            place(BELT_KIND, CROSSING_TILE, North, None),
            None,
        ),
        case(
            "splitter onto a belt",
            Some(SPLITTER_KIND),
            place(SPLITTER_KIND, BELT_TILE, North, None),
            None,
        ),
        case(
            "pipe onto a crusher",
            Some(PIPE_KIND),
            place(PIPE_KIND, CRUSHER_TILE, North, None),
            None,
        ),
        case(
            "crusher overlapping a belt",
            Some("crusher"),
            place("crusher", BELT_TILE, East, None),
            None,
        ),
        // Free ground.
        case(
            "pipe on free ground",
            Some(PIPE_KIND),
            place(PIPE_KIND, EMPTY_TILE, North, None),
            Some(PIPE_KIND),
        ),
        case(
            "pipe on free ground, none owned",
            None,
            place(PIPE_KIND, EMPTY_TILE, North, None),
            None,
        ),
        case(
            "crusher on free ground",
            Some("crusher"),
            place("crusher", EMPTY_TILE, South, None),
            Some("crusher"),
        ),
        // Hopper items: named raw, omitted with two raws, not raw at all.
        case(
            "hopper of tomato",
            Some("hopper"),
            place("hopper", EMPTY_TILE, East, Some("tomato")),
            Some("hopper"),
        ),
        case(
            "hopper of nothing named",
            Some("hopper"),
            place("hopper", EMPTY_TILE, East, None),
            None,
        ),
        case(
            "hopper of ketchup",
            Some("hopper"),
            place("hopper", EMPTY_TILE, East, Some("ketchup")),
            None,
        ),
        // Not a buildable kind.
        case(
            "unknown kind",
            None,
            place("rocket", EMPTY_TILE, East, None),
            None,
        ),
        case(
            "crossing bought as such",
            None,
            place(CROSSING_KIND, EMPTY_TILE, East, None),
            None,
        ),
    ]
}

fn purchase(kind: &str, quantity: u32) -> PlayerCommand {
    PlayerCommand::Purchase {
        kind: kind.to_string(),
        quantity,
    }
}

fn purchase_cases() -> Vec<Case> {
    vec![
        case("buy a belt", None, purchase(BELT_KIND, 1), Some("")),
        case("buy nothing", None, purchase(BELT_KIND, 0), None),
        case("buy an unknown kind", None, purchase("rocket", 1), None),
        case(
            "buy more than money holds",
            None,
            purchase("cooking_vat", u32::MAX),
            None,
        ),
        case(
            "buy more than the balance",
            None,
            purchase("cooking_vat", 10_000),
            None,
        ),
    ]
}

/// Runs `case` on a fresh `setup()` world: preview, check it changed
/// nothing, then issue for real and check the two agree.
fn check(setup: fn() -> World, case: &Case) {
    let mut world = setup();
    if let Some(kind) = case.own {
        buy(&mut world, kind);
    }
    let save_before = world.to_save_ron();
    let revision_before = world.revision();

    let preview = world.preview_command(&case.cmd);

    assert_eq!(
        world.to_save_ron(),
        save_before,
        "{}: a preview changed the world",
        case.name
    );
    assert_eq!(
        world.revision(),
        revision_before,
        "{}: a preview moved the revision",
        case.name
    );
    assert_eq!(
        preview.valid,
        preview.reason.is_empty(),
        "{}: reason must be empty exactly when valid ({preview:?})",
        case.name
    );

    world.issue_command(case.cmd.clone());
    let changed = world.to_save_ron() != save_before;
    assert_eq!(
        preview.valid,
        changed,
        "{}: preview said {preview:?} but the command {} the world",
        case.name,
        if changed { "changed" } else { "did not change" }
    );
    assert_eq!(
        preview.valid.then_some(preview.result_kind.as_str()),
        case.expect,
        "{}: unexpected preview {preview:?}",
        case.name
    );
    if let PlayerCommand::Place { tile, .. } = &case.cmd
        && preview.valid
    {
        assert_eq!(
            kind_at(&world, *tile).as_deref(),
            Some(preview.result_kind.as_str()),
            "{}: result_kind is not what the tile became",
            case.name
        );
    }
}

#[test]
fn preview_agrees_with_the_command_on_every_placement() {
    for case in place_cases() {
        check(scene, &case);
    }
}

#[test]
fn preview_agrees_with_the_command_on_every_purchase() {
    for case in purchase_cases() {
        check(|| world_with(1_000), &case);
    }
}

#[test]
fn a_parallel_pipe_is_refused_with_the_reason() {
    let mut world = scene();
    buy(&mut world, PIPE_KIND);
    let preview = world.preview_command(&place(PIPE_KIND, BELT_TILE, Direction::East, None));
    assert!(!preview.valid);
    assert!(preview.reason.contains("right angles"), "{preview:?}");
}

#[test]
fn a_placement_with_none_owned_says_so() {
    let world = scene();
    let preview = world.preview_command(&place(PIPE_KIND, EMPTY_TILE, Direction::East, None));
    assert!(!preview.valid);
    assert!(preview.reason.contains("own no"), "{preview:?}");
}

#[test]
fn an_uncovered_command_is_refused_not_guessed() {
    let world = scene();
    for cmd in [
        PlayerCommand::Remove { tile: BELT_TILE },
        PlayerCommand::Rotate { tile: BELT_TILE },
        PlayerCommand::Hire {
            category: "manufacturing".to_string(),
        },
    ] {
        let preview = world.preview_command(&cmd);
        assert!(!preview.valid, "{cmd:?} was guessed valid: {preview:?}");
        assert!(preview.reason.contains("does not check"), "{preview:?}");
        assert!(preview.result_kind.is_empty());
    }
}

#[test]
fn the_wire_carries_exactly_three_fields() {
    let mut world = scene();
    buy(&mut world, PIPE_KIND);
    let cmd = command_from_json(
        r#"{"type": "place", "kind": "pipe", "tile": [0, 0], "direction": "north"}"#,
    )
    .expect("a place command parses");
    let json = serde_json::to_value(world.preview_command(&cmd)).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"valid": true, "reason": "", "result_kind": "crossing"})
    );
    let json = serde_json::to_value(PreviewResult::malformed()).unwrap();
    assert_eq!(json["valid"], false);
    assert!(!json["reason"].as_str().unwrap().is_empty());
    assert_eq!(json["result_kind"], "");
}
