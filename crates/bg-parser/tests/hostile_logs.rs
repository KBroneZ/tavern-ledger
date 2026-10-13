//! Logs no real game writes: counters far out of range, very long lines and
//! very many entities. Each must end as an unsupported game or be read in
//! time that grows with the log, never a panic or a stall.

use std::time::{Duration, Instant};

use bg_parser::report::GameReport;
use bg_parser::state::{MAX_ENTITIES, MAX_HEROES};
use bg_parser::LogReader;

const T: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

/// Generous for a debug build; the quadratic paths these guard took
/// seconds to minutes on the same input.
const FAST: Duration = Duration::from_secs(10);

fn line(indent: usize, body: &str) -> String {
    format!("{T}{}{body}", "    ".repeat(indent))
}

fn tag(entity: &str, tag: &str, value: &str) -> String {
    line(
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

fn hero(id: i64, pid: i64) -> Vec<String> {
    vec![
        line(
            1,
            &format!("FULL_ENTITY - Creating ID={id} CardID=SYN_HERO"),
        ),
        line(2, "tag=CONTROLLER value=2"),
        line(2, "tag=ZONE value=PLAY"),
        line(2, &format!("tag=PLAYER_ID value={pid}")),
        line(2, "tag=PLAYER_LEADERBOARD_PLACE value=1"),
        line(2, "tag=CARDTYPE value=HERO"),
        line(2, "tag=HEALTH value=30"),
        line(2, &format!("tag=ENTITY_ID value={id}")),
    ]
}

/// A Solo game up to its first shop turn, with `game_tags` on the game entity.
fn start(game_tags: &[&str]) -> Vec<String> {
    let mut l = vec![
        line(0, "CREATE_GAME"),
        line(1, "GameEntity EntityID=1"),
        line(2, "tag=CARDTYPE value=GAME"),
    ];
    l.extend(game_tags.iter().map(|t| line(2, t)));
    l.extend([
        line(2, "tag=ENTITY_ID value=1"),
        line(1, "Player EntityID=2 PlayerID=2 GameAccountId=[hi=1 lo=1]"),
        line(2, "tag=PLAYER_ID value=2"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=HERO_ENTITY value=10"),
        line(2, "tag=ENTITY_ID value=2"),
        line(1, "Player EntityID=3 PlayerID=10 GameAccountId=[hi=0 lo=0]"),
        line(2, "tag=PLAYER_ID value=10"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=BACON_DUMMY_PLAYER value=1"),
        line(2, "tag=ENTITY_ID value=3"),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - BuildNumber=253216".into(),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".into(),
    ]);
    l.extend(hero(10, 2));
    l.push(tag("2", "HERO_ENTITY", "10"));
    l.push(tag("GameEntity", "TURN", "1"));
    l
}

fn read(lines: &[String]) -> Vec<GameReport> {
    let mut reader = LogReader::default();
    for l in lines {
        reader.feed(l);
    }
    // Every live view the app asks for, as it does each second.
    let _ = reader.snapshot_current();
    let _ = reader.lobby_now();
    let _ = reader.next_opponent_now();
    let _ = reader.own_board_now();
    let _ = reader.own_values_now();
    reader.finish()
}

fn problems(lines: &[String]) -> Vec<String> {
    let games = read(lines);
    assert_eq!(games.len(), 1);
    games[0].problems.clone()
}

#[test]
fn a_turn_no_game_reaches_makes_the_game_unsupported() {
    for value in [
        "202",
        "2000000000",
        "9223372036854775807",
        "-1",
        "-9223372036854775808",
    ] {
        let mut l = start(&[]);
        l.push(tag("GameEntity", "TURN", "2"));
        l.push(tag("GameEntity", "TURN", value));
        l.push(tag("GameEntity", "TURN", "3"));
        assert_eq!(
            problems(&l),
            ["parser error: UnsupportedTurn"],
            "TURN={value}"
        );
    }
}

#[test]
fn a_turn_given_when_the_game_entity_is_created_is_checked_too() {
    let l = start(&["tag=TURN value=2000000000"]);
    assert_eq!(problems(&l), ["parser error: UnsupportedTurn"]);
}

#[test]
fn turns_a_long_real_game_reaches_are_read() {
    let mut l = start(&[]);
    for turn in 2..=80 {
        l.push(tag("GameEntity", "TURN", &turn.to_string()));
    }
    assert!(problems(&l).is_empty());
}

#[test]
fn damage_at_the_ends_of_the_number_range_does_not_overflow() {
    let mut l = start(&[]);
    l.extend([
        line(1, "FULL_ENTITY - Creating ID=20 CardID=SYN_MINION"),
        line(2, "tag=CARDTYPE value=MINION"),
        line(2, "tag=CONTROLLER value=2"),
        line(2, "tag=ZONE value=PLAY"),
        line(2, "tag=ZONE_POSITION value=1"),
        line(2, "tag=ATK value=1"),
        line(2, "tag=HEALTH value=1"),
        line(2, "tag=ENTITY_ID value=20"),
    ]);
    l.push(tag("GameEntity", "TURN", "2"));
    l.push(line(
        0,
        "BLOCK_START BlockType=ATTACK Entity=20 EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 ",
    ));
    l.push(tag("20", "DAMAGE", "-9223372036854775808"));
    l.push(tag("20", "DAMAGE", "9223372036854775807"));
    l.push(tag("20", "HEALTH", "-9223372036854775808"));
    l.push(tag("20", "ARMOR", "9223372036854775807"));
    l.push(line(0, "BLOCK_END"));
    l.push(tag("GameEntity", "TURN", "3"));
    let _ = read(&l);
}

#[test]
fn a_long_line_of_broken_card_references_reads_in_linear_time() {
    // One ~1 MiB line of "[entityName=" with the only " id=N zone=" at its end.
    let mut body = String::from("TAG_CHANGE Entity=GameEntity tag=STEP value=MAIN_ACTION ");
    while body.len() < 1_000_000 {
        body.push_str("[entityName=x ");
    }
    body.push_str(" id=5 zone=PLAY cardId=SYN_HERO]");
    let mut l = start(&[]);
    l.push(line(1, &body));
    let started = Instant::now();
    let _ = read(&l);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
}

#[test]
fn very_many_leaderboard_heroes_read_in_linear_time() {
    let mut l = start(&[]);
    for i in 0..(MAX_HEROES as i64 - 100) {
        let id = 100 + i;
        l.extend(hero(id, 2 + i % 8));
        l.push(tag(
            &id.to_string(),
            "COPIED_FROM_ENTITY_ID",
            &(id - 1).to_string(),
        ));
        l.push(tag(&id.to_string(), "PLAYER_LEADERBOARD_PLACE", "2"));
    }
    let started = Instant::now();
    let problems = problems(&l);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
    assert!(
        !problems.iter().any(|p| p.starts_with("parser error")),
        "{problems:?}"
    );
}

#[test]
fn a_game_with_more_hero_entities_than_any_real_one_stops_as_unsupported() {
    let mut l = start(&[]);
    for i in 0..MAX_HEROES as i64 {
        l.extend(hero(100 + i, 2 + i % 8));
    }
    assert_eq!(problems(&l), ["parser error: TooManyHeroes"]);
}

#[test]
fn a_game_with_more_entities_than_any_real_one_stops_as_unsupported() {
    let mut l = start(&[]);
    for id in 100..100 + MAX_ENTITIES as i64 {
        l.push(line(
            1,
            &format!("FULL_ENTITY - Creating ID={id} CardID=SYN"),
        ));
        l.push(line(2, "tag=CARDTYPE value=MINION"));
    }
    let started = Instant::now();
    assert_eq!(problems(&l), ["parser error: TooManyEntities"]);
    assert!(started.elapsed() < FAST, "took {:?}", started.elapsed());
}

#[test]
fn an_entity_with_more_tags_than_any_real_one_stops_the_game_as_unsupported() {
    let mut l = start(&[]);
    for i in 0..5_000 {
        l.push(tag("1", &format!("SYN_TAG_{i}"), "1"));
    }
    assert_eq!(problems(&l), ["parser error: TooManyTags"]);
}
