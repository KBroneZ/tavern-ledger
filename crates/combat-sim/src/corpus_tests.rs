//! The corpus on hand-made saved games: what a line keeps and leaves out,
//! its id, the checks on reading, and that the engine reads a corpus
//! combat exactly as the saved game it came from.

use serde_json::json;

use super::*;
use crate::saved::{explain_game, CardFacts};
use crate::trials::Options;

/// A card with no combat text (`cards::NO_COMBAT_EFFECT`).
const PLAIN: &str = "BG28_603t";

fn facts(id: &str) -> Option<CardFacts> {
    (id == PLAIN).then_some(CardFacts {
        tier: Some(1),
        tribes: crate::model::Tribes::NONE,
        base: None,
    })
}

fn bm(card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"card_id": card, "golden": false, "atk": atk, "health": health,
           "position": position, "keywords": []})
}

/// A round where the player's 3/3 and 1/1 meet a 1/1, with a saved replay
/// in which the opponent attacks first, and fields a corpus leaves out.
fn round(number: i64, result: &str) -> Value {
    let on = |side: &str, mut m: Value| {
        m["side"] = json!(side);
        m
    };
    json!({
        "number": number,
        "result": result,
        "opponents": [2],
        "health_after": {"1": 30, "2": 25},
        "entries": [
            {"side": "own", "player_id": 1, "hero": "BG20_HERO_100",
             "board": [bm(PLAIN, 3, 3, 1), bm(PLAIN, 1, 1, 2)], "name": "Someone"},
            {"side": "opponent", "player_id": 2, "hero": "BG21_HERO_030_SKIN_G",
             "board": [bm(PLAIN, 1, 1, 1)]},
        ],
        "replay": {
            "complete": true,
            "start": 3,
            "minions": [
                on("own", bm(PLAIN, 3, 3, 1)),
                on("own", bm(PLAIN, 1, 1, 2)),
                on("opponent", bm(PLAIN, 1, 1, 1)),
            ],
            "events": [{"kind": "attack", "attacker": 2, "defender": 0}],
        },
    })
}

fn record(session: &str, index: u64, saved_at: u64, rounds: Value) -> String {
    json!({
        "session": session, "index": index, "saved_at": saved_at,
        "played": {"from": 1_792_462_422, "to": 1_792_464_001},
        "report": {
            "game_type": "GT_BATTLEGROUNDS", "build": "254477",
            "lobby": [{"player_id": 2, "name": "Opponent#1234"}],
            "card_names": {"BG20_HERO_100": "A hero"},
            "rounds": rounds,
        },
    })
    .to_string()
}

fn options() -> Options {
    Options {
        trials: 200,
        ..Options::default()
    }
}

#[test]
fn fnv64_matches_the_published_test_vectors() {
    assert_eq!(fnv64(""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv64("a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn canonical_sorts_keys_at_every_depth() {
    let a: Value = serde_json::from_str(r#"{"b":1,"a":{"d":[{"z":1,"y":2}],"c":"x"}}"#).unwrap();
    assert_eq!(
        canonical(&a),
        r#"{"a":{"c":"x","d":[{"y":2,"z":1}]},"b":1}"#
    );
}

#[test]
fn a_combat_keeps_only_what_the_engine_reads() {
    let history = record("S1", 1, 1, json!([round(1, "won"), round(2, "")]));
    let e = export(&history).unwrap();
    assert_eq!(e.lines.len(), 1);
    assert_eq!(e.skipped.get("no result"), Some(&1));
    assert_eq!(e.written.get(&(254_477, "solo")), Some(&1));
    let line = &e.lines[0];
    for gone in [
        "Someone",
        "Opponent",
        "lobby",
        "session",
        "played",
        "card_names",
        "opponents",
        "health_after",
    ] {
        assert!(!line.contains(gone), "{gone} must not be in the corpus");
    }
    let c = read_line(line).unwrap();
    assert_eq!(c.build, 254_477);
    assert!(!c.duos);
    assert_eq!(c.id.len(), 16);
    // Card names and card data stay out (HearthstoneJSON's data is not in
    // the repository).
    assert!(!line.contains("\"cards\""));
    assert_eq!(c.round()["number"], 1);
}

#[test]
fn the_engine_reads_a_corpus_combat_as_the_saved_game() {
    let history = record("S1", 1, 1, json!([round(1, "won"), round(2, "lost")]));
    let saved: Value = serde_json::from_str(&history).unwrap();
    let from_saved = explain_game(&saved["report"], &facts, &options());
    let e = export(&history).unwrap();
    let mut from_corpus: Vec<_> = e
        .lines
        .iter()
        .map(|l| read_line(l).unwrap())
        .flat_map(|c| explain_game(&c.report, &facts, &options()))
        .collect();
    from_corpus.sort_by_key(|w| w.round);
    assert_eq!(from_corpus, from_saved);
    assert_eq!(from_saved.len(), 2);
}

#[test]
fn the_last_record_of_a_game_wins_and_one_combat_is_kept_once() {
    let history = [
        record("S1", 1, 1, json!([round(1, "lost")])),
        record("S1", 1, 2, json!([round(1, "won")])),
        // Another game with the very same combat.
        record("S2", 1, 3, json!([round(1, "won")])),
    ]
    .join("\n");
    let e = export(&history).unwrap();
    assert_eq!(e.games, 2);
    assert_eq!(e.lines.len(), 1);
    assert_eq!(read_line(&e.lines[0]).unwrap().round()["result"], "won");
    assert_eq!(e.skipped.get("same combat twice"), Some(&1));
}

#[test]
fn a_game_without_a_build_is_left_out() {
    let history = record("S1", 1, 1, json!([round(1, "won")])).replace("\"254477\"", "null");
    let e = export(&history).unwrap();
    assert!(e.lines.is_empty());
    assert_eq!(e.skipped.get("game without a build"), Some(&1));
}

#[test]
fn a_new_field_in_a_saved_game_stops_the_export() {
    let mut r = round(1, "won");
    r["entries"][0]["board"][0]["owner"] = json!("x");
    let history = record("S1", 1, 1, json!([r]));
    let err = export(&history).unwrap_err();
    assert!(err.contains("owner"), "{err}");
}

#[test]
fn private_keys_and_strings_are_refused() {
    assert!(check_private(&json!({"board": [{"card_id": "BG28_603t"}]})).is_ok());
    for bad in [
        json!({"name": "x"}),
        json!({"board": [{"card_id": "Someone#1234"}]}),
        json!({"hero": "Two words"}),
        json!({"hero": ""}),
        json!({"hero": "x".repeat(65)}),
    ] {
        assert!(check_private(&bad).is_err(), "{bad}");
    }
}

#[test]
fn a_line_is_refused_when_changed_or_malformed() {
    let history = record("S1", 1, 1, json!([round(1, "won")]));
    let line = export(&history).unwrap().lines.remove(0);
    let changed = line.replace("\"won\"", "\"lost\"");
    assert!(read_line(&changed).unwrap_err().contains("does not match"));
    let mut v: Value = serde_json::from_str(&line).unwrap();
    v["format"] = json!(2);
    assert!(read_line(&v.to_string()).unwrap_err().contains("format"));
    let mut v: Value = serde_json::from_str(&line).unwrap();
    let r = v["report"]["rounds"][0].clone();
    v["report"]["rounds"] = json!([r.clone(), r]);
    v.as_object_mut().unwrap().remove("id");
    v["id"] = json!(format!(
        "{:016x}",
        fnv64(&canonical(&{
            let mut w = v.clone();
            w.as_object_mut().unwrap().remove("id");
            w
        }))
    ));
    assert!(read_line(&v.to_string()).unwrap_err().contains("one round"));
}

#[test]
fn a_corpus_refuses_a_repeated_line_and_names_it() {
    let history = record("S1", 1, 1, json!([round(1, "won")]));
    let line = export(&history).unwrap().lines.remove(0);
    let err = read_corpus(&format!("{line}\n\n{line}\n")).unwrap_err();
    assert!(err.contains("line 3") && err.contains("repeated"), "{err}");
    assert_eq!(read_corpus(&format!("{line}\n")).unwrap().len(), 1);
}

#[test]
fn duos_and_other_modes() {
    let duo = record("S1", 1, 1, json!([round(1, "won")]))
        .replace("\"GT_BATTLEGROUNDS\"", "\"GT_BATTLEGROUNDS_DUO\"");
    let other = record("S2", 1, 1, json!([round(1, "won")]))
        .replace("\"GT_BATTLEGROUNDS\"", "\"GT_RANKED\"");
    let e = export(&format!("{duo}\n{other}")).unwrap();
    assert_eq!(e.lines.len(), 1);
    assert!(read_line(&e.lines[0]).unwrap().duos);
    assert_eq!(e.skipped.get("not a Battlegrounds game"), Some(&1));
    let report = render_export(&e);
    assert!(report.contains("build 254477 duos: 1"), "{report}");
}
