//! Tests of the live state's counters (T-611), apart from `live_tests.rs`
//! to keep that file short.

use super::*;
use bg_parser::counters::{Counter, CounterKind};
use serde_json::json;

fn with_status(status: &str) -> LiveGame {
    live_game(&json!({"status": status, "warnings": [], "problems": []}))
}

fn playing() -> LiveGame {
    let live = with_status("incomplete");
    assert_eq!(live.phase, Phase::Playing);
    live
}

#[test]
fn counters_are_unknown_until_given_and_only_while_the_game_is_on() {
    let spell = Counter::of_stats(CounterKind::TavernSpell, 0, 5);
    assert_eq!(playing().counters, None, "not added yet");
    let live = playing().with_counters(Some(vec![spell.clone()]));
    assert_eq!(live.counters, Some(vec![spell.clone()]), "+0/+5 is kept");
    assert!(live.warnings.is_empty());
    assert_eq!(playing().with_counters(Some(vec![])).counters, Some(vec![]));
    assert_eq!(playing().with_counters(None).counters, None, "reader error");

    let over = with_status("ok").with_counters(Some(vec![spell.clone()]));
    assert_eq!(over.phase, Phase::Over);
    assert_eq!(over.counters, None);
    let starting = with_status("").with_counters(Some(vec![spell]));
    assert_eq!(starting.phase, Phase::Starting);
    assert_eq!(starting.counters, None, "no game being played");
}

#[test]
fn a_counter_out_of_range_is_dropped_with_one_warning_never_clamped() {
    let kept = Counter::of_count(CounterKind::EternalKnights, 99_999);
    let live = playing().with_counters(Some(vec![
        Counter::of_stats(CounterKind::BloodGem, 100_000, 2),
        kept.clone(),
        Counter::of_stats(CounterKind::TavernSpell, 3, -1),
        Counter::of_attack(CounterKind::UndeadAttack, 4),
    ]));
    assert_eq!(
        live.counters,
        Some(vec![kept, Counter::of_attack(CounterKind::UndeadAttack, 4)])
    );
    assert_eq!(
        live.warnings,
        [
            "counter out of range: blood_gem",
            "counter out of range: tavern_spell"
        ]
    );
}

#[test]
fn counters_serialize_with_each_kinds_own_fields_and_null_when_unknown() {
    let live = playing().with_counters(Some(vec![
        Counter::of_stats(CounterKind::BloodGem, 4, 3),
        Counter::of_count(CounterKind::EternalKnights, 5),
    ]));
    let json = serde_json::to_value(&live).expect("serializable");
    assert_eq!(
        json["counters"],
        json!([
            {"kind": "blood_gem", "atk": 4, "health": 3},
            {"kind": "eternal_knights", "count": 5},
        ])
    );
    let unknown = serde_json::to_value(playing()).expect("serializable");
    assert_eq!(unknown["counters"], Value::Null);
}
