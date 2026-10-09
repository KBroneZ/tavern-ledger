//! Health of every lobby hero after each combat (and before the first one).
//! Rust-only fields: the Python prototype does not write them, so the parity
//! test removes them; these tests cover them.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use bg_parser::report::GameReport;

fn parse(name: &str) -> GameReport {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(format!("{name}.log"));
    let log = fs::read(path).expect("fixture");
    let mut reports = bg_parser::parse_reader(log.as_slice()).expect("in-memory read");
    assert_eq!(reports.len(), 1);
    reports.remove(0)
}

fn map(pairs: &[(i64, i64)]) -> BTreeMap<i64, i64> {
    pairs.iter().copied().collect()
}

#[test]
fn health_is_kept_for_every_hero_before_and_after_each_combat() {
    let r = parse("solo_local_eliminated");
    assert_eq!(r.start_health, map(&[(2, 30), (3, 30), (4, 30), (5, 30)]));
    assert_eq!(
        r.rounds[0].health_after,
        map(&[(2, 18), (3, 30), (4, 30), (5, 30)])
    );
    // The killing blow can overshoot: health never goes below 0.
    assert_eq!(
        r.rounds[1].health_after,
        map(&[(2, 0), (3, 30), (4, 30), (5, 30)])
    );
}

#[test]
fn the_last_round_of_a_finished_game_has_the_final_health() {
    let r = parse("solo_game");
    assert_eq!(r.rounds.len(), 1);
    assert_eq!(r.rounds[0].health_after, map(&[(2, 23), (3, 30)]));
}

#[test]
fn the_own_health_stays_the_one_the_prototype_writes() {
    let r = parse("solo_local_eliminated");
    for round in &r.rounds {
        let local = r.local_player_id.expect("local player");
        assert_eq!(
            round.own_health_after,
            round.health_after.get(&local).copied()
        );
    }
}

#[test]
fn an_unfinished_game_has_no_health_for_its_last_combat() {
    // The same game, but the log stops before it completes: the heroes' health
    // at that moment is not the health after the last combat.
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/solo_local_eliminated.log");
    let log = fs::read_to_string(path).expect("fixture");
    let log = log.replace("tag=STATE value=COMPLETE", "tag=STATE value=RUNNING");
    let mut reports = bg_parser::parse_reader(log.as_bytes()).expect("in-memory read");
    let r = reports.remove(0);
    let last = r.rounds.last().expect("a round");
    assert_eq!(last.own_health_after, None);
    assert!(
        last.health_after.is_empty(),
        "unknown, not the current health"
    );
}

#[test]
fn health_is_serialized_with_the_report() {
    let r = parse("solo_local_eliminated");
    let json = serde_json::to_value(&r).expect("serializable");
    assert_eq!(json["start_health"]["2"], 30);
    assert_eq!(json["rounds"][0]["health_after"]["2"], 18);
}
