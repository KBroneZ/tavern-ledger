//! Tests of `live` (moved out to keep `live.rs` under 800 lines).

use super::*;
use crate::possible::{Reason, TribesBasis};
use crate::stats::{DUOS, SOLO};
use serde_json::json;

fn minion(card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"card_id": card, "atk": atk, "health": health, "position": position, "golden": false})
}

fn round(n: i64, opp: i64, board: Value, health: Value) -> Value {
    json!({
        "number": n,
        "own_health_after": health["1"],
        "health_after": health,
        "entries": [
            {"side": "own", "player_id": 1, "hero": "H_ME", "board": []},
            {"side": "opponent", "player_id": opp, "hero": format!("H_{opp}"), "board": board},
        ],
        "opponents": [opp],
    })
}

/// Solo, four players, we are seat 1.
fn solo(rounds: Vec<Value>, status: &str) -> Value {
    json!({
        "status": status, "game_type": SOLO, "local_player_id": 1, "hero": "H_ME",
        "card_names": {"H_ME": "Me Hero", "H_2": "Second"},
        "lobby": [
            {"player_id": 1, "hero": "H_ME", "duo_team": null, "final_place": null, "final_health": 40},
            {"player_id": 2, "hero": "H_2", "duo_team": null, "final_place": null, "final_health": 40},
            {"player_id": 3, "hero": "H_3", "duo_team": null, "final_place": null, "final_health": 40},
            {"player_id": 4, "hero": "H_4", "duo_team": null, "final_place": null, "final_health": 40},
        ],
        "start_health": {"1": 40, "2": 40, "3": 40, "4": 40},
        "shop_tribes": {"BEAST": 4, "MURLOC": 9},
        "rounds": rounds, "warnings": [], "problems": [],
    })
}

fn opponent(live: &LiveGame, seat: i64) -> &LiveOpponent {
    live.opponents
        .iter()
        .find(|o| o.seats == [seat])
        .expect("opponent")
}

#[test]
fn every_lobby_hero_but_us_is_an_opponent_even_before_a_combat() {
    let live = live_game(&solo(vec![], "incomplete"));
    let mut seats: Vec<_> = live.opponents.iter().map(|o| o.seats.clone()).collect();
    seats.sort();
    assert_eq!(seats, [[2], [3], [4]]);
    let first = opponent(&live, 2);
    assert_eq!(first.heroes[0].name.as_deref(), Some("Second"));
    assert_eq!(first.record, None, "not fought yet is not 0-0-0");
    assert_eq!(first.record_source, Source::Unknown);
    assert!(first.boards.is_empty());
    assert_eq!(first.boards_source, Source::Unknown);
    assert_eq!(first.health.value, Some(40), "start health from the log");
}

#[test]
fn the_last_board_that_entered_play_comes_with_its_round() {
    let rounds = vec![
        round(
            1,
            2,
            json!([minion("OLD", 1, 1, 1)]),
            json!({"1": 40, "2": 40, "3": 40, "4": 40}),
        ),
        round(
            2,
            3,
            json!([minion("C3", 2, 2, 1)]),
            json!({"1": 35, "2": 40, "3": 40, "4": 40}),
        ),
        round(
            3,
            2,
            json!([minion("B", 5, 6, 2), minion("A", 3, 3, 1)]),
            json!({"1": 35, "2": 31, "3": 40, "4": 40}),
        ),
    ];
    let live = live_game(&solo(rounds, "incomplete"));
    let two = opponent(&live, 2);
    assert_eq!(two.boards.len(), 1);
    assert_eq!(two.boards[0].round, 3, "the newest board, not the first");
    let ids: Vec<_> = two.boards[0]
        .minions
        .iter()
        .map(|m| m.card_id.as_deref())
        .collect();
    assert_eq!(ids, [Some("A"), Some("B")], "in board order");
    assert_eq!(two.boards_source, Source::Log);
    assert_eq!(opponent(&live, 3).boards[0].round, 2);
    assert!(opponent(&live, 4).boards.is_empty());
}

#[test]
fn a_board_the_log_never_replayed_is_unknown_not_empty() {
    let hidden = round(1, 2, Value::Null, json!({"1": 40, "2": 40}));
    let live = live_game(&solo(vec![hidden], "incomplete"));
    assert!(opponent(&live, 2).boards.is_empty());
    assert_eq!(opponent(&live, 2).boards_source, Source::Unknown);
}

#[test]
fn an_empty_board_that_did_play_is_a_board() {
    let live = live_game(&solo(
        vec![round(1, 2, json!([]), json!({"1": 40, "2": 40}))],
        "incomplete",
    ));
    let two = opponent(&live, 2);
    assert_eq!(two.boards.len(), 1);
    assert!(two.boards[0].minions.is_empty());
    assert_eq!(two.boards_source, Source::Log);
}

#[test]
fn the_record_is_worked_out_from_health_and_inferred() {
    let rounds = vec![
        round(1, 2, json!([]), json!({"1": 35, "2": 40, "3": 40, "4": 40})),
        round(2, 2, json!([]), json!({"1": 35, "2": 30, "3": 40, "4": 40})),
    ];
    let live = live_game(&solo(rounds, "incomplete"));
    let two = opponent(&live, 2);
    let record = two.record.as_ref().expect("fought");
    assert_eq!((record.won, record.lost, record.tie), (1, 1, 0));
    assert_eq!(two.record_source, Source::Inferred);
    assert_eq!(two.health.value, Some(30));
    assert_eq!(live.own_health.value, Some(35));
    assert_eq!(live.last_combat.value, Some(2));
}

#[test]
fn health_during_a_combat_is_the_one_after_the_last_closed_combat() {
    // Round 2 is on but its health is not known yet (the combat is open).
    let mut open = round(2, 3, json!([]), json!({}));
    open["own_health_after"] = Value::Null;
    let rounds = vec![
        round(1, 2, json!([]), json!({"1": 35, "2": 40, "3": 40, "4": 40})),
        open,
    ];
    let live = live_game(&solo(rounds, "incomplete"));
    assert_eq!(live.own_health.value, Some(35));
    assert_eq!(opponent(&live, 3).health.value, Some(40));
}

#[test]
fn an_older_round_without_health_is_a_gap_not_a_stale_number() {
    let mut gap = round(2, 3, json!([]), json!({}));
    gap["own_health_after"] = Value::Null;
    let rounds = vec![
        round(1, 2, json!([]), json!({"1": 35, "2": 40, "3": 40, "4": 40})),
        gap,
        round(3, 4, json!([]), json!({})),
    ];
    let live = live_game(&solo(rounds, "incomplete"));
    assert_eq!(opponent(&live, 3).health.value, None);
    assert_eq!(opponent(&live, 3).health.source, Source::Unknown);
}

#[test]
fn a_minion_number_the_log_lacks_is_unknown_not_zero() {
    let mut broken = minion("A", 1, 1, 1);
    broken.as_object_mut().unwrap().remove("health");
    let live = live_game(&solo(
        vec![round(1, 2, json!([broken]), json!({"1": 40, "2": 40}))],
        "incomplete",
    ));
    let m = &opponent(&live, 2).boards[0].minions[0];
    assert_eq!((m.atk, m.health), (Some(1), None));
}

#[test]
fn health_the_log_does_not_give_is_unknown_not_zero() {
    let mut report = solo(vec![], "incomplete");
    report["start_health"] = json!({});
    let live = live_game(&report);
    assert_eq!(opponent(&live, 2).health.value, None);
    assert_eq!(opponent(&live, 2).health.source, Source::Unknown);
    assert_eq!(live.own_health.value, None);
    assert_eq!(live.own_health.source, Source::Unknown);
}

#[test]
fn tavern_tribes_and_entered_tribes_stay_apart_and_labelled() {
    let live = live_game(&solo(vec![], "incomplete"));
    assert_eq!(live.tribes[0].tribe, "MURLOC");
    assert_eq!(live.tribes_source, Source::Inferred);
    assert!(live.entered_tribes.is_empty());
    assert_eq!(live.entered_tribes_source, Source::Unknown);
    let tribes: Vec<String> = ["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]
        .map(String::from)
        .to_vec();
    let live = live.with_entered_tribes(Some(&tribes));
    assert_eq!(live.entered_tribes, tribes);
    assert_eq!(live.entered_tribes_source, Source::Entered);
    assert_eq!(
        live.tribes[0].tribe, "MURLOC",
        "the tavern list is untouched"
    );
}

#[test]
fn no_tribes_seen_is_unknown() {
    let mut report = solo(vec![], "incomplete");
    report["shop_tribes"] = json!({});
    let live = live_game(&report);
    assert!(live.tribes.is_empty());
    assert_eq!(live.tribes_source, Source::Unknown);
}

#[test]
fn phase_follows_what_the_parser_says() {
    assert_eq!(live_game(&solo(vec![], "incomplete")).phase, Phase::Playing);
    assert_eq!(live_game(&solo(vec![], "ok")).phase, Phase::Over);
    let starting = json!({"status": "unsupported", "game_type": SOLO,
        "problems": ["could not identify the local player and the shop player"]});
    assert_eq!(live_game(&starting).phase, Phase::Starting);
    let broken = json!({"status": "unsupported", "game_type": SOLO,
        "problems": ["parser error: RegexParsingError"]});
    let live = live_game(&broken);
    assert_eq!(live.phase, Phase::Unreadable);
    assert!(live.opponents.is_empty());
    assert_eq!(live.problems, ["parser error: RegexParsingError"]);
}

#[test]
fn without_our_own_seat_nobody_is_called_an_opponent() {
    let mut report = solo(vec![], "incomplete");
    report.as_object_mut().unwrap().remove("local_player_id");
    assert!(live_game(&report).opponents.is_empty());
}

/// Duos: we are seats 1 and 2 (team 1); 3 and 4 are team 2; 5 and 6 team 3.
fn duos(rounds: Vec<Value>) -> Value {
    let seat = |pid: i64, team: i64| {
        json!({"player_id": pid, "hero": format!("H_{pid}"), "duo_team": team,
               "final_place": null, "final_health": 30})
    };
    json!({
        "status": "incomplete", "game_type": DUOS, "local_player_id": 1, "hero": "H_1",
        "teammate_player_id": 2, "teammate_hero": "H_2",
        "lobby": [seat(1, 1), seat(2, 1), seat(3, 2), seat(4, 2), seat(5, 3), seat(6, 3)],
        "start_health": {"1": 30, "2": 30, "3": 30, "4": 30, "5": 30, "6": 30},
        "shop_tribes": {}, "rounds": rounds, "warnings": [], "problems": [],
    })
}

#[test]
fn in_duos_an_opponent_is_a_team_with_a_board_per_hero() {
    let both_legs = json!({
        "number": 2,
        "own_health_after": 30,
        "health_after": {"1": 30, "2": 30, "3": 30, "4": 30, "5": 30, "6": 30},
        "entries": [
            {"side": "own", "player_id": 1, "hero": "H_1", "board": []},
            {"side": "opponent", "player_id": 3, "hero": "H_3", "board": [minion("X", 1, 1, 1)]},
            {"side": "own", "player_id": 2, "hero": "H_2", "board": []},
            {"side": "opponent", "player_id": 4, "hero": "H_4", "board": [minion("Y", 2, 2, 1)]},
        ],
        "opponents": [3, 4],
    });
    let live = live_game(&duos(vec![both_legs]));
    assert!(live.is_duos);
    assert_eq!(
        live.opponents.len(),
        2,
        "two opposing teams, not four heroes"
    );
    let team = live.opponents.iter().find(|o| o.seats == [3, 4]).unwrap();
    assert_eq!(team.heroes.len(), 2);
    let boards: Vec<_> = team
        .boards
        .iter()
        .map(|b| b.minions[0].card_id.as_deref())
        .collect();
    assert_eq!(boards, [Some("X"), Some("Y")]);
    let other = live.opponents.iter().find(|o| o.seats == [5, 6]).unwrap();
    assert!(other.boards.is_empty());
    assert_eq!(live.teammate_hero.value.as_ref().unwrap().id, "H_2");
}

fn slot(player_id: i64, place: Option<i64>, tech_level: Option<i64>) -> LobbySlot {
    LobbySlot {
        player_id,
        place,
        tech_level,
    }
}

#[test]
fn the_leaderboard_gives_places_tiers_and_its_order() {
    let lobby = [
        slot(1, Some(2), Some(3)),
        slot(2, Some(4), Some(1)),
        slot(3, Some(1), Some(2)),
        slot(4, Some(3), None),
    ];
    let live = live_game(&solo(vec![], "incomplete")).with_leaderboard(&lobby);
    let order: Vec<_> = live.opponents.iter().map(|o| o.seats[0]).collect();
    assert_eq!(order, [3, 4, 2], "top of the leaderboard first");
    assert_eq!(opponent(&live, 3).place.value, Some(1));
    assert_eq!(opponent(&live, 2).tiers[0].value, Some(1));
    assert_eq!(opponent(&live, 4).tiers[0].source, Source::Unknown);
    assert_eq!(live.own_place.value, Some(2));
    assert_eq!(live.own_seats, [1]);
    assert_eq!(live.leaderboard_slots, Some(4));
}

#[test]
fn without_the_leaderboard_places_and_tiers_are_unknown() {
    let live = live_game(&solo(vec![], "incomplete"));
    let two = opponent(&live, 2);
    assert_eq!(two.place.source, Source::Unknown);
    assert_eq!(two.tiers.len(), 1);
    assert_eq!(two.tiers[0].value, None);
    assert_eq!(live.leaderboard_slots, None);
    let live = live.with_leaderboard(&[]);
    assert_eq!(live.own_place.value, None);
}

#[test]
fn a_leaderboard_with_gaps_or_missing_places_has_no_slot_count() {
    assert_eq!(
        leaderboard_slots(&[slot(1, Some(1), None), slot(2, Some(3), None)]),
        None
    );
    assert_eq!(
        leaderboard_slots(&[slot(1, Some(1), None), slot(2, None, None)]),
        None
    );
    assert_eq!(leaderboard_slots(&[]), None);
    let three_on_one = [
        slot(1, Some(1), None),
        slot(2, Some(1), None),
        slot(3, Some(1), None),
    ];
    assert_eq!(leaderboard_slots(&three_on_one), None);
}

#[test]
fn in_duos_a_team_has_one_place_and_a_tier_per_hero() {
    let lobby = [
        slot(1, Some(2), Some(2)),
        slot(2, Some(2), Some(3)),
        slot(3, Some(1), Some(1)),
        slot(4, Some(1), Some(2)),
        slot(5, Some(3), Some(4)),
        slot(6, Some(3), Some(5)),
    ];
    let live = live_game(&duos(vec![])).with_leaderboard(&lobby);
    assert_eq!(live.leaderboard_slots, Some(3));
    assert_eq!(live.own_place.value, Some(2));
    let first = &live.opponents[0];
    assert_eq!(first.seats, [3, 4]);
    assert_eq!(first.place.value, Some(1));
    let tiers: Vec<_> = first.tiers.iter().map(|t| t.value).collect();
    assert_eq!(tiers, [Some(1), Some(2)]);
}

/// A shop record whose newest turn is `turn` (none for 0).
fn at_turn(mut report: Value, turn: i64) -> Value {
    let turns: Vec<Value> = (1..=turn).map(|t| json!({"turn": t})).collect();
    report["shop"] = json!({"turns": turns});
    report
}

fn pool() -> Vec<crate::possible::PoolCard> {
    vec![crate::possible::PoolCard {
        id: "NEUTRAL_1".into(),
        tier: 1,
        tribes: Vec::new(),
        duos_only: false,
    }]
}

#[test]
fn the_game_turn_is_the_newest_shop_turn() {
    assert_eq!(live_game(&solo(vec![], "incomplete")).turn.value, None);
    assert_eq!(
        live_game(&at_turn(solo(vec![], "incomplete"), 0))
            .turn
            .value,
        None
    );
    let live = live_game(&at_turn(solo(vec![], "incomplete"), 5));
    assert_eq!(live.turn.value, Some(5));
    assert_eq!(live.turn.source, Source::Log);
}

#[test]
fn possible_minions_on_turns_two_and_three_then_only_for_heroes_not_met() {
    let solo_lobby = [
        slot(1, Some(1), Some(1)),
        slot(2, Some(2), Some(1)),
        slot(3, Some(3), Some(2)),
        slot(4, Some(4), Some(1)),
    ];
    let duos_lobby = [
        slot(1, Some(1), Some(1)),
        slot(2, Some(1), Some(1)),
        slot(3, Some(2), Some(2)),
        slot(4, Some(2), Some(1)),
        slot(5, Some(3), Some(1)),
        slot(6, Some(3), Some(1)),
    ];
    let tribes = ["BEAST".to_string()];
    for (name, report, lobby) in [
        ("solo", solo(vec![], "incomplete"), &solo_lobby[..]),
        ("duos", duos(vec![]), &duos_lobby[..]),
    ] {
        for turn in 1..=4 {
            let live = live_game(&at_turn(report.clone(), turn))
                .with_leaderboard(lobby)
                .with_entered_tribes(Some(&tribes))
                .with_possible(&pool());
            let listed = live.opponents.iter().all(|o| {
                o.possible.len() == o.seats.len()
                    && o.possible.iter().all(|p| p.source == Source::Possible)
            });
            let none = live.opponents.iter().all(|o| o.possible.is_empty());
            let reasons: Vec<Reason> = live
                .opponents
                .iter()
                .flat_map(|o| o.possible.iter().map(|p| p.reason))
                .collect();
            match turn {
                1 => assert!(none, "{name} turn 1: nothing listed"),
                2 | 3 => {
                    assert!(listed, "{name} turn {turn}: every opponent hero listed");
                    assert!(reasons.iter().all(|r| *r == Reason::EarlyTurn));
                }
                _ => {
                    assert!(listed, "{name} turn {turn}: no board seen, all listed");
                    assert!(reasons.iter().all(|r| *r == Reason::NotMetYet));
                }
            }
        }
    }
}

#[test]
fn from_turn_four_a_hero_whose_board_was_seen_gets_no_list() {
    let lobby = [
        slot(1, Some(1), Some(1)),
        slot(2, Some(2), Some(3)),
        slot(3, Some(3), Some(3)),
        slot(4, Some(4), Some(3)),
    ];
    let rounds = vec![round(
        1,
        2,
        json!([minion("BEAST_1", 1, 1, 1)]),
        json!({"1": 40, "2": 40}),
    )];
    let tribes = ["BEAST".to_string()];
    let live = live_game(&at_turn(solo(rounds, "incomplete"), 4))
        .with_leaderboard(&lobby)
        .with_entered_tribes(Some(&tribes))
        .with_possible(&pool());
    let met = live.opponents.iter().find(|o| o.seats == [2]).unwrap();
    assert_eq!(met.boards[0].seat, 2);
    assert!(met.possible.is_empty(), "board seen: no guess");
    for seat in [3, 4] {
        let other = live.opponents.iter().find(|o| o.seats == [seat]).unwrap();
        assert_eq!(other.possible.len(), 1, "seat {seat} not met yet");
        assert_eq!(
            other.possible[0].min_tier, 2,
            "their tier and the tier below"
        );
    }
}

#[test]
fn the_tavern_offers_confirm_the_lobby_tribes_with_the_card_data() {
    let mut report = at_turn(solo(vec![], "incomplete"), 2);
    report["shop"]["turns"][0]["offers"] = json!([
        {"card_id": "BEAST_1", "roll": 0, "frozen": false},
        {"card_id": "DUAL_1", "roll": 0, "frozen": false},
        {"card_id": null, "roll": 1, "frozen": false},
    ]);
    report["shop"]["turns"][1]["offers"] = json!([
        {"card_id": "MURLOC_1", "roll": 0, "frozen": false},
    ]);
    let live = live_game(&report);
    assert_eq!(live.offered, ["BEAST_1", "DUAL_1", "MURLOC_1"]);
    let card = |id: &str, tribes: &[&str]| crate::possible::PoolCard {
        id: id.into(),
        tier: 1,
        tribes: tribes.iter().map(|t| t.to_string()).collect(),
        duos_only: false,
    };
    let pool = [
        card("BEAST_1", &["BEAST"]),
        card("MURLOC_1", &["MURLOC"]),
        card("DUAL_1", &["DEMON", "PIRATE"]),
    ];
    let live = live.with_possible(&pool);
    assert_eq!(live.lobby_tribes.tribes, ["BEAST", "MURLOC"]);
    assert_eq!(live.lobby_tribes.basis, TribesBasis::TavernConfirmed);
    assert_eq!(live.lobby_tribes.source, Source::Inferred);
    assert!(!live.lobby_tribes.complete);
    let text = serde_json::to_string(&live).unwrap();
    assert!(
        !text.contains("\"offered\""),
        "offers stay out of the overlay state"
    );
}

#[test]
fn the_words_of_every_source_mark_travel_with_the_state() {
    let live = live_game(&solo(vec![], "incomplete"));
    let labels: Vec<_> = live.legend.iter().map(|e| (e.source, e.label)).collect();
    assert!(labels.contains(&(Source::Inferred, "inferred")));
    assert!(labels.contains(&(Source::Entered, "entered by you")));
    assert!(labels.contains(&(Source::Unknown, "unknown")));
}

#[test]
fn nothing_in_the_live_state_can_hold_a_name() {
    // Opponents are heroes and seats; the serialized state has no field
    // for a player name, BattleTag or account.
    let mut report = solo(vec![], "incomplete");
    report["player_name"] = json!("Someone#4321");
    report["battletag"] = json!("Someone#4321");
    report["lobby"][1]["player_name"] = json!("Other#1234");
    report["lobby"][1]["game_account_id"] = json!("hi=1 lo=2");
    report["rounds"] = json!([round(1, 2, json!([]), json!({"1": 40, "2": 40}))]);
    report["rounds"][0]["entries"][1]["player_name"] = json!("Other#1234");
    let text = serde_json::to_string(&live_game(&report)).unwrap();
    for forbidden in ["Someone", "Other#", "1234", "4321", "hi=1", "account"] {
        assert!(
            !text.contains(forbidden),
            "{forbidden} reached the live state"
        );
    }
}
