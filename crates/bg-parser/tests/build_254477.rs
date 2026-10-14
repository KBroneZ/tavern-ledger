//! Build 254477 (October 14, 2026) against a real Duos game of that build
//! (`tests/data/real/b254477_duos.log`, made by `tools/make_fixture.py`).
//! What this build's log shows that 253216's did not: the roll button's
//! `BACON_FREE_REFRESH_COUNT` (so a turn can report free rolls), `MAGNETIC`
//! where `MODULAR` was, and plain `BG_ShopBuff` entities. Everything else the
//! parser reads (phases, boards, combats, leaderboard, trinkets, Dark Gifts,
//! buffs) must read the same way as on 253216.

use std::fs;
use std::path::Path;

use bg_parser::report::{GameReport, Status};
use bg_parser::TESTED_BUILDS;
use serde_json::Value;

fn game() -> GameReport {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real/b254477_duos.log");
    let text = fs::read_to_string(path).expect("fixture");
    let mut reports = bg_parser::parse_reader(text.as_bytes()).expect("in-memory read");
    assert_eq!(reports.len(), 1, "one game in the fixture");
    reports.remove(0)
}

fn json() -> Value {
    serde_json::to_value(game()).expect("serializable")
}

fn array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    value[key].as_array().unwrap_or_else(|| panic!("{key}"))
}

fn board_minions(report: &Value) -> Vec<&Value> {
    array(report, "rounds")
        .iter()
        .flat_map(|round| array(round, "entries"))
        .filter_map(|entry| entry["board"].as_array())
        .flatten()
        .collect()
}

#[test]
fn build_254477_is_tested_and_reads_clean() {
    assert!(TESTED_BUILDS.contains(&254_477));
    let report = game();
    assert_eq!(report.build, Some(254_477));
    assert_eq!(report.status, Status::Ok);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
}

#[test]
fn phases_combats_and_result_are_read() {
    let report = json();
    assert_eq!(report["game_type"], "GT_BATTLEGROUNDS_DUO");
    assert_eq!(report["final_place"], 1);
    let rounds = array(&report, "rounds");
    assert_eq!(rounds.len(), 13);
    assert!(rounds.iter().all(|r| r["result"].is_string()));
    assert!(rounds.iter().all(|r| !array(r, "entries").is_empty()));
    let with_replay = rounds
        .iter()
        .filter(|r| {
            r["replay"]["events"]
                .as_array()
                .is_some_and(|e| !e.is_empty())
        })
        .count();
    assert!(with_replay >= 10, "{with_replay} combats with a replay");
}

#[test]
fn leaderboard_trinkets_and_triples_are_read() {
    let report = json();
    let lobby = array(&report, "lobby");
    assert_eq!(lobby.len(), 8);
    assert!(lobby.iter().all(|p| p["trinkets"].is_array()));
    assert!(lobby.iter().any(|p| p["triples"].is_array()));
}

#[test]
fn dark_gifts_and_buffs_are_read() {
    let report = json();
    let minions = board_minions(&report);
    assert!(minions.iter().any(|m| m["gifts"].is_array()), "Dark Gifts");
    assert!(minions.iter().any(|m| m["buffs"].is_array()), "buffs");
    let gift_choices = array(&report, "discovers")
        .iter()
        .filter(|d| d["source"] == "BG36_MidGameEffect_010")
        .count();
    assert_eq!(gift_choices, 3);
}

/// 254477 is the first build whose logs carry the roll button's free roll
/// count; the shop record reports it for the turns that had free rolls.
#[test]
fn free_rolls_are_reported_on_the_turns_that_had_them() {
    let report = json();
    let turns = array(&report["shop"], "turns");
    let with_free: Vec<(i64, i64)> = turns
        .iter()
        .filter_map(|t| Some((t["turn"].as_i64()?, t["free_rolls"].as_i64()?)))
        .filter(|(_, free)| *free > 0)
        .collect();
    assert_eq!(with_free, [(7, 2), (12, 2)]);
}
