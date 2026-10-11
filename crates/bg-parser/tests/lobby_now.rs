//! The leaderboard as the log has it during a game (T-306): each lobby
//! hero's place and tavern tier. Live only: the saved report does not change.

use std::fs;
use std::path::Path;

use bg_parser::report::LobbySlot;
use bg_parser::LogReader;

fn lines(name: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(format!("{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .lines()
        .map(String::from)
        .collect()
}

fn tag(entity: i64, tag: &str, value: i64) -> String {
    format!(
        "D 12:00:00.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity={entity} tag={tag} value={value} "
    )
}

fn reader_with(lines: &[String]) -> LogReader {
    let mut reader = LogReader::default();
    lines.iter().for_each(|l| reader.feed(l));
    reader
}

fn up_to(all: &[String], needle: &str) -> Vec<String> {
    let at = all
        .iter()
        .position(|l| l.contains(needle))
        .expect("fixture has the line");
    all[..at].to_vec()
}

#[test]
fn no_game_no_leaderboard() {
    assert!(LogReader::default().lobby_now().is_empty());
}

#[test]
fn places_and_tiers_follow_the_log_as_they_change() {
    let all = lines("solo_game");
    let mut part = up_to(&all, "tag=TURN value=1");
    part.push(tag(10, "PLAYER_TECH_LEVEL", 1));
    part.push(tag(21, "PLAYER_TECH_LEVEL", 2));
    let reader = reader_with(&part);
    assert_eq!(
        reader.lobby_now(),
        [
            LobbySlot {
                player_id: 2,
                place: Some(1),
                tech_level: Some(1)
            },
            LobbySlot {
                player_id: 3,
                place: Some(2),
                tech_level: Some(2)
            },
        ]
    );

    let mut later = part.clone();
    later.push(tag(10, "PLAYER_LEADERBOARD_PLACE", 2));
    later.push(tag(21, "PLAYER_LEADERBOARD_PLACE", 1));
    later.push(tag(21, "PLAYER_TECH_LEVEL", 3));
    let now = reader_with(&later).lobby_now();
    assert_eq!((now[0].place, now[0].tech_level), (Some(2), Some(1)));
    assert_eq!((now[1].place, now[1].tech_level), (Some(1), Some(3)));
}

#[test]
fn a_tier_the_log_never_gave_is_unknown_not_zero() {
    let all = lines("solo_game");
    let reader = reader_with(&up_to(&all, "tag=TURN value=2"));
    let now = reader.lobby_now();
    assert_eq!(now.len(), 2);
    assert!(now.iter().all(|s| s.tech_level.is_none()));
}

#[test]
fn numbers_the_game_cannot_have_are_unknown() {
    let all = lines("solo_game");
    let mut part = up_to(&all, "tag=TURN value=1");
    part.push(tag(10, "PLAYER_TECH_LEVEL", 9));
    part.push(tag(21, "PLAYER_LEADERBOARD_PLACE", 12));
    let now = reader_with(&part).lobby_now();
    assert_eq!(now[0].tech_level, None);
    assert_eq!(now[1].place, None);
}

#[test]
fn a_finished_game_has_no_live_leaderboard() {
    let reader = reader_with(&lines("solo_game"));
    assert!(reader.lobby_now().is_empty());
}

#[test]
fn a_broken_game_has_no_leaderboard() {
    let all = lines("solo_game");
    let mut part = up_to(&all, "tag=TURN value=1");
    part.push("not a log line".into());
    assert!(reader_with(&part).lobby_now().is_empty());
}

#[test]
fn in_duos_both_heroes_of_a_team_share_its_place() {
    // The user's own Duos game (T-108 fixture), stopped before its end.
    let all = lines("real/b253216_duos");
    let reader = reader_with(&up_to(&all, "STATE value=COMPLETE"));
    let now = reader.lobby_now();
    assert_eq!(now.len(), 8);
    let report = reader.snapshot_current().expect("a game is on");
    let mut teams = 0;
    for team in 1..=4 {
        let places: Vec<_> = report
            .lobby
            .iter()
            .filter(|p| p.duo_team == Some(team))
            .filter_map(|p| now.iter().find(|s| s.player_id == p.player_id))
            .map(|s| s.place)
            .collect();
        if places.len() == 2 {
            assert!(places[0].is_some(), "team {team} has a place");
            assert_eq!(places[0], places[1], "team {team}");
            teams += 1;
        }
    }
    assert!(
        teams >= 3,
        "the teams the report knows share places: {teams}"
    );
    let mut places: Vec<_> = now.iter().filter_map(|s| s.place).collect();
    places.sort_unstable();
    assert_eq!(places, [1, 1, 2, 2, 3, 3, 4, 4]);
    assert!(
        now.iter().all(|s| s.tech_level.is_some()),
        "the fixture keeps every hero's tier (D-046)"
    );
}

#[test]
fn the_saved_report_has_no_new_fields() {
    let reports = bg_parser::parse_reader(lines("solo_game").join("\n").as_bytes()).unwrap();
    let json = serde_json::to_string(&reports).unwrap();
    assert!(!json.contains("tech_level"));
}

/// The next opponent the game marks on its leaderboard (T-313): the local
/// leaderboard hero's `NEXT_OPPONENT_PLAYER_ID`, set once each shop turn.
/// Live only, like the places and tiers.
#[test]
fn the_next_opponent_is_the_local_heros_mark() {
    let all = lines("solo_game");
    let part = up_to(&all, "tag=TURN value=1");
    assert_eq!(reader_with(&part).next_opponent_now(), None, "not set yet");

    let mut marked = part.clone();
    marked.push(tag(10, "NEXT_OPPONENT_PLAYER_ID", 3));
    assert_eq!(reader_with(&marked).next_opponent_now(), Some(3));
}

#[test]
fn a_next_opponent_mark_that_is_not_an_opponent_seat_is_ignored() {
    let all = lines("solo_game");
    let part = up_to(&all, "tag=TURN value=1");
    for (entity, value) in [(21, 2), (10, 2), (10, 7), (10, 0)] {
        let mut marked = part.clone();
        marked.push(tag(entity, "NEXT_OPPONENT_PLAYER_ID", value));
        assert_eq!(
            reader_with(&marked).next_opponent_now(),
            None,
            "entity {entity} value {value}"
        );
    }
}

#[test]
fn no_game_no_next_opponent() {
    assert_eq!(LogReader::default().next_opponent_now(), None);
}

fn game_tag(tag: &str, value: i64) -> String {
    format!(
        "D 12:00:00.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=GameEntity tag={tag} value={value} "
    )
}

/// The combat damage cap the game shows (T-315), as on the user's games of
/// 2026-10-11: another value at hero select, 15 from the first shop turn,
/// turned off late in the game.
#[test]
fn the_damage_cap_is_the_game_entitys_while_it_is_on() {
    let all = lines("solo_game");
    let mut part = up_to(&all, "tag=TURN value=1");
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP", 2));
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP_ENABLED", 1));
    assert_eq!(
        reader_with(&part).damage_cap_now(),
        None,
        "before the first shop"
    );
    part.push(game_tag("TURN", 1));
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP", 15));
    assert_eq!(reader_with(&part).damage_cap_now(), Some(15));
    part.push(game_tag("TURN", 2));
    assert_eq!(reader_with(&part).damage_cap_now(), Some(15), "combat too");
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP_ENABLED", 0));
    assert_eq!(reader_with(&part).damage_cap_now(), None, "turned off");
}

#[test]
fn no_damage_cap_without_the_game_setting_it_or_a_game_on() {
    let all = lines("solo_game");
    let mut part = up_to(&all, "tag=TURN value=2");
    assert_eq!(reader_with(&part).damage_cap_now(), None, "never set");
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP", 15));
    assert_eq!(reader_with(&part).damage_cap_now(), None, "not on");
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP_ENABLED", 1));
    part.push(game_tag("BACON_COMBAT_DAMAGE_CAP", 500));
    assert_eq!(reader_with(&part).damage_cap_now(), None, "out of range");
    assert_eq!(LogReader::default().damage_cap_now(), None);
    let ended = reader_with(&all);
    assert_eq!(ended.damage_cap_now(), None, "the game is over");
}
