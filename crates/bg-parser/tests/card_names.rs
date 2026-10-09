//! Hero names come from the log itself ("[entityName=... cardId=...]"), in the
//! game client's language: what the player sees on screen. Never a player
//! name, never a name the log does not give.

use std::fs;
use std::path::Path;

use serde_json::Value;

const PREFIX: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() -     ";

fn solo_log_with(extra: &[&str]) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/solo_game.log");
    let mut log = fs::read_to_string(path).expect("fixture");
    for line in extra {
        log.push_str(PREFIX);
        log.push_str(line);
        log.push('\n');
    }
    log.into_bytes()
}

fn report(log: &[u8]) -> Value {
    let reports = bg_parser::parse_reader(log).expect("in-memory read");
    assert_eq!(reports.len(), 1);
    serde_json::to_value(&reports[0]).expect("serializable")
}

#[test]
fn hero_name_comes_from_the_log() {
    let r = report(&solo_log_with(&[
        "TAG_CHANGE Entity=[entityName=Lord Jaraxxus id=10 zone=PLAY zonePos=0 cardId=TB_BaconShop_HERO_37 player=2] tag=1068 value=0 ",
    ]));
    assert_eq!(r["hero"], "TB_BaconShop_HERO_37");
    assert_eq!(
        r["card_names"]["TB_BaconShop_HERO_37"], "Lord Jaraxxus",
        "{r}"
    );
}

#[test]
fn heroes_without_a_name_in_the_log_have_none() {
    let r = report(&solo_log_with(&[]));
    assert_eq!(r["card_names"], serde_json::json!({}));
}

#[test]
fn only_hero_names_are_kept_never_players_or_unknown_entities() {
    let r = report(&solo_log_with(&[
        "TAG_CHANGE Entity=[entityName=SyntheticPlayer#0001 id=2 zone=PLAY zonePos=0 cardId= player=2] tag=1068 value=0 ",
        "TAG_CHANGE Entity=[entityName=UNKNOWN ENTITY [cardType=INVALID] id=21 zone=PLAY zonePos=0 cardId=TB_BaconShop_HERO_60 player=10] tag=1068 value=0 ",
        "TAG_CHANGE Entity=[entityName=Some Minion id=10 zone=PLAY zonePos=1 cardId=BG_NOT_A_HERO player=2] tag=1068 value=0 ",
    ]));
    let names = r["card_names"].as_object().expect("map");
    assert!(names.is_empty(), "{names:?}");
    assert!(!r.to_string().contains("SyntheticPlayer"));
}

#[test]
fn names_do_not_change_the_rest_of_the_report() {
    let mut with = report(&solo_log_with(&[
        "TAG_CHANGE Entity=[entityName=Lord Jaraxxus id=10 zone=PLAY zonePos=0 cardId=TB_BaconShop_HERO_37 player=2] tag=1068 value=0 ",
    ]));
    let mut without = report(&solo_log_with(&[]));
    with.as_object_mut().unwrap().remove("card_names");
    without.as_object_mut().unwrap().remove("card_names");
    assert_eq!(with, without);
}
