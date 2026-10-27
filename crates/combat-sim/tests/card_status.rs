//! `data/card_status.json` lists how the engine plays each card it knows
//! (T-647), for the game build watcher's issue (`tools/game_build_watch.py`).
//! A change to the card rules that this file does not show fails here;
//! rewrite it with `CARD_STATUS_BLESS=1 cargo test -p combat-sim --test
//! card_status` and commit it.

use std::path::Path;

use combat_sim::cards::{modelled_ids, status};
use serde_json::{json, Map, Value};

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/card_status.json");

fn now() -> String {
    let cards: Map<String, Value> = modelled_ids()
        .into_iter()
        .map(|id| (id.to_string(), json!(status(id).unwrap_or("unknown"))))
        .collect();
    let mut text = serde_json::to_string_pretty(&json!({ "format": 1, "cards": cards })).unwrap();
    text.push('\n');
    text
}

#[test]
fn the_committed_card_status_is_the_engines() {
    let now = now();
    if std::env::var_os("CARD_STATUS_BLESS").is_some() {
        std::fs::write(PATH, &now).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(Path::new(PATH))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        committed == now,
        "data/card_status.json is not the engine's: rewrite it with CARD_STATUS_BLESS=1"
    );
}

#[test]
fn every_status_is_one_of_three() {
    for id in modelled_ids() {
        let s = status(id).unwrap();
        assert!(
            ["exact", "approximate", "refused"].contains(&s),
            "{id}: {s}"
        );
    }
    assert_eq!(status("ZZ_NOT_A_CARD"), None);
    assert_eq!(status("BGS_119"), Some("exact"));
    assert_eq!(status("BG31_999"), Some("refused"));
}
