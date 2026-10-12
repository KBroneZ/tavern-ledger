//! A hand-made saved replay rebuilt into logged steps.

use serde_json::json;

use super::*;
use crate::harness::load_cards;

const CARDS: &str = include_str!("../tests/data/cards.json");

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
    let steps = saved_steps(&replay, &db).expect("readable");
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
    assert!(saved_steps(&incomplete, &db).is_none());
    let unknown_minion = json!({
        "complete": true, "start": 1,
        "minions": [m("own", "BGS_119", 1, 1, 1)],
        "events": [{"kind": "attack", "attacker": 5, "defender": null}],
    });
    assert!(saved_steps(&unknown_minion, &db).is_none());
}
