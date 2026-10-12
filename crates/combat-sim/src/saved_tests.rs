//! Hand-made saved games: a replay rebuilt into logged steps, and each
//! combat's odds and cause read from the report.

use serde_json::json;

use super::*;
use crate::combat::replay::Rule;
use crate::harness::{load_cards, CardDb};
use crate::why::{Cause, NoReplay, NotSeen, Said, Unseen};

const CARDS: &str = include_str!("../tests/data/cards.json");

fn steps_of(replay: &Value, db: &CardDb) -> Option<Vec<LoggedStep>> {
    saved_steps(replay, &|m| crate::harness::minion(m, db))
}

fn m(side: &str, card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"side": side, "card_id": card, "golden": false, "atk": atk, "health": health,
           "position": position, "keywords": []})
}

#[test]
fn a_saved_replay_gives_the_boards_before_each_attack() {
    let db = load_cards(CARDS).unwrap();
    let replay = json!({
        "complete": true,
        "start": 3,
        "minions": [
            m("own", "BGS_119", 2, 4, 1),
            m("own", "BG28_300", 1, 1, 2),
            m("opponent", "BGS_119", 1, 5, 1),
            // A Reborn copy with no slot yet: where the same card died.
            m("own", "BG28_300", 1, 1, 0),
        ],
        "events": [
            {"kind": "attack", "attacker": 1, "defender": 2},
            {"kind": "damage", "minion": 2, "amount": 1, "health": 4},
            {"kind": "damage", "minion": 1, "amount": 1, "health": 0},
            {"kind": "death", "minion": 1},
            {"kind": "summon", "minion": 3},
            {"kind": "shield", "minion": 0, "on": true},
            {"kind": "attack", "attacker": 2, "defender": 0},
        ],
    });
    let steps = steps_of(&replay, &db).expect("readable");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].attacker, (0, 1));
    assert_eq!(steps[0].defender, Some(0));
    assert!(!steps[0].all_keywords);
    let second = &steps[1];
    assert_eq!(second.attacker, (1, 0));
    let own: Vec<(&str, i32, i32)> = second.before[0]
        .iter()
        .map(|m| (m.card_id.as_str(), m.attack, m.health))
        .collect();
    assert_eq!(own, [("BGS_119", 2, 4), ("BG28_300", 1, 1)]);
    assert!(second.before[0][0]
        .keywords
        .contains(Keywords::DIVINE_SHIELD));
    assert_eq!(second.before[1][0].health, 4);
}

#[test]
fn an_incomplete_or_broken_replay_is_not_read() {
    let db = load_cards(CARDS).unwrap();
    let incomplete = json!({"complete": false, "start": 0, "minions": [], "events": []});
    assert!(steps_of(&incomplete, &db).is_none());
    let unknown_minion = json!({
        "complete": true, "start": 1,
        "minions": [m("own", "BGS_119", 1, 1, 1)],
        "events": [{"kind": "attack", "attacker": 5, "defender": null}],
    });
    assert!(steps_of(&unknown_minion, &db).is_none());
}

/// A card with no combat text (`cards::NO_COMBAT_EFFECT`).
const PLAIN: &str = "BG28_603t";

fn facts(id: &str) -> Option<CardFacts> {
    id.starts_with("BG").then_some(CardFacts {
        tier: Some(1),
        tribes: Tribes::NONE,
        base: None,
    })
}

fn bm(card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"card_id": card, "golden": false, "atk": atk, "health": health,
           "position": position, "keywords": []})
}

/// The player's 3/3 and `extra` against a 1/1: the player attacks first
/// and always wins.
fn entries(extra: Value) -> Value {
    json!([
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1), extra]},
        {"side": "opponent", "player_id": 2, "board": [bm(PLAIN, 1, 1, 1)]},
    ])
}

/// A saved replay of those boards where the opponent's 1/1 attacks first.
fn opponent_first() -> Value {
    let on = |side: &str, mut m: Value| {
        m["side"] = json!(side);
        m
    };
    json!({
        "complete": true,
        "start": 3,
        "minions": [
            on("own", bm(PLAIN, 3, 3, 1)),
            on("own", bm(PLAIN, 1, 1, 2)),
            on("opponent", bm(PLAIN, 1, 1, 1)),
        ],
        "events": [{"kind": "attack", "attacker": 2, "defender": 0}],
    })
}

fn game(game_type: &str, rounds: Value) -> Value {
    json!({"game_type": game_type, "rounds": rounds})
}

fn whys(report: &Value) -> Vec<CombatWhy> {
    let options = Options {
        trials: 100,
        ..Options::default()
    };
    explain_game(report, &facts, &options)
}

fn cause(w: &CombatWhy) -> &Cause {
    match &w.why {
        Why::Explained(e) => e.cause.as_ref().expect("odds"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn each_combat_with_a_result_gets_its_odds_and_cause() {
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([
            {"number": 1, "result": "won", "entries": entries(bm(PLAIN, 1, 1, 2))},
            {"number": 2, "result": "lost", "entries": entries(bm(PLAIN, 1, 1, 2))},
            {"number": 3, "result": "lost", "entries": entries(bm(PLAIN, 1, 1, 2)),
             "replay": opponent_first()},
            // No result: left out.
            {"number": 4, "entries": entries(bm(PLAIN, 1, 1, 2))},
        ]),
    );
    let w = whys(&report);
    assert_eq!(w.iter().map(|w| w.round).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(cause(&w[0]), &Cause::Matched);
    let Why::Explained(e) = &w[0].why else {
        unreachable!()
    };
    assert!(matches!(e.said, Said::Odds(p) if p.won == 1.0));
    assert_eq!(cause(&w[1]).id(), "no_replay_exact");
    let Cause::RuleDifference(d) = cause(&w[2]) else {
        panic!("{:?}", w[2])
    };
    assert_eq!((d.attack, d.rule), (1, Rule::FirstAttacker));
}

#[test]
fn a_replay_whose_first_boards_are_not_the_reports_is_unreadable() {
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([{"number": 2, "result": "lost", "entries": entries(bm(PLAIN, 2, 2, 2)),
                "replay": opponent_first()}]),
    );
    assert!(matches!(
        cause(&whys(&report)[0]),
        Cause::NoReplay {
            why: NoReplay::Unreadable,
            ..
        }
    ));
}

#[test]
fn values_and_unnamed_gifts_come_from_the_report() {
    // Blood Gems with no upgrades saved: missing inputs.
    let gems = "BG33_886";
    let mut gifted = bm(PLAIN, 1, 1, 2);
    gifted["gift_unknown"] = json!(true);
    let mut gem_replay = opponent_first();
    gem_replay["minions"][1]["card_id"] = json!(gems);
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([
            {"number": 1, "result": "lost", "entries": entries(bm(gems, 1, 1, 2)),
             "replay": gem_replay.clone()},
            {"number": 2, "result": "lost", "entries": entries(gifted),
             "replay": opponent_first()},
        ]),
    );
    let w = whys(&report);
    let Cause::MissingInputs { inputs, .. } = cause(&w[0]) else {
        panic!("{:?}", w[0])
    };
    assert_eq!(inputs[0].what, Unseen::BloodGems);
    let Cause::MissingInputs { inputs, .. } = cause(&w[1]) else {
        panic!("{:?}", w[1])
    };
    let gift = NotSeen {
        what: Unseen::Gift,
        card: PLAIN.into(),
    };
    assert_eq!(inputs, &vec![gift]);
    // With the upgrades the board saved, the card is exact.
    let mut e = entries(bm(gems, 1, 1, 2));
    e[0]["values"] = json!({"blood_gem_atk": 2, "blood_gem_health": 2});
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([{"number": 1, "result": "lost", "entries": e, "replay": gem_replay}]),
    );
    assert!(matches!(cause(&whys(&report)[0]), Cause::RuleDifference(_)));
}

#[test]
fn a_combat_without_boards_or_card_data_gets_no_odds() {
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([
            {"number": 1, "result": "won", "entries": [
                {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1)]},
                {"side": "opponent", "player_id": 2, "board": null},
            ]},
            {"number": 2, "result": "won", "entries": entries(bm("UNKNOWN_1", 1, 1, 2))},
        ]),
    );
    let w = whys(&report);
    assert_eq!(w[0].why, Why::NoBoards);
    assert_eq!(w[1].why, Why::NoCardData(vec!["UNKNOWN_1".into()]));
}

#[test]
fn duos_teams_bring_each_seats_board_once_in_order() {
    let rounds = json!([{"number": 3, "result": "won", "entries": [
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 1, 1, 1)]},
        {"side": "opponent", "player_id": 5, "board": [bm(PLAIN, 1, 1, 1)]},
        {"side": "own", "player_id": 2, "board": [bm(PLAIN, 9, 9, 1)]},
        // The first board again (the hero switching back): left out.
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 1, 1, 1)]},
    ]}]);
    // The teammate's 9/9 comes in after the two 1/1s trade: a sure win.
    let w = whys(&game("GT_BATTLEGROUNDS_DUO", rounds.clone()));
    assert_eq!(cause(&w[0]), &Cause::Matched);
    // Solo takes one board a side only.
    let solo = whys(&game("GT_BATTLEGROUNDS", rounds));
    assert_eq!(solo[0].why, Why::NoBoards);
}

#[test]
fn a_game_that_is_not_battlegrounds_gives_nothing() {
    let rounds = json!([{"number": 1, "result": "won", "entries": entries(bm(PLAIN, 1, 1, 2))}]);
    assert!(whys(&game("GT_RANKED", rounds.clone())).is_empty());
    assert!(whys(&json!({ "rounds": rounds })).is_empty());
}

#[test]
fn the_kept_values_match_the_parsers_bound() {
    assert_eq!(MAX_GAME_VALUE, bg_parser::report::MAX_GAME_VALUE);
}
