//! `--reparse` (T-107): re-read the sessions still on disk, save only games
//! whose report changed, and list the games whose logs are gone.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use bg_parser::{PARSER_REVISION, PARSER_VERSION};
use serde_json::json;
use tracker::store::{GameKey, ParserStamp, Store};
use tracker::{import_session, reparse, UnavailableReason};

const SESSION_A: &str = "Hearthstone_2026_10_09_10_00_00";
const SESSION_B: &str = "Hearthstone_2026_10_09_12_00_00";

fn fixture(name: &str) -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../bg-parser/tests/data/{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n")
}

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tavern-ledger-reparse-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn append(path: &Path, text: &str) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    f.write_all(text.as_bytes()).unwrap();
}

fn key(session: &str, index: u64) -> GameKey {
    GameKey {
        session: session.into(),
        index,
    }
}

/// Logs with two sessions (A: two games, B: one game), all imported once.
fn setup() -> (PathBuf, PathBuf, Store) {
    let root = temp_dir();
    let logs = root.join("Logs");
    for session in [SESSION_A, SESSION_B] {
        fs::create_dir_all(logs.join(session)).unwrap();
    }
    append(
        &logs.join(SESSION_A).join("Power_old.log"),
        &fixture("duo_game"),
    );
    append(
        &logs.join(SESSION_A).join("Power.log"),
        &fixture("solo_game"),
    );
    append(
        &logs.join(SESSION_B).join("Power.log"),
        &fixture("solo_game"),
    );
    let data = root.join("data");
    let mut store = Store::open(&data).unwrap();
    for session in [SESSION_A, SESSION_B] {
        import_session(&logs.join(session), &mut store).unwrap();
    }
    (logs, data, store)
}

#[test]
fn unchanged_games_are_not_saved_again() {
    let (logs, data, mut store) = setup();
    let before = fs::read_to_string(data.join("games.jsonl")).unwrap();
    let outcome = reparse(&logs, &mut store).unwrap();
    assert_eq!(outcome.sessions_read, 2);
    assert!(outcome.changed.is_empty());
    assert!(outcome.unavailable.is_empty());
    assert_eq!(
        fs::read_to_string(data.join("games.jsonl")).unwrap(),
        before
    );
}

#[test]
fn a_game_whose_report_changed_is_saved_and_the_last_record_wins() {
    let (logs, data, mut store) = setup();
    // An older parser read this game differently.
    let stale = json!({"index": 2, "status": "unsupported", "problems": ["old parser"]});
    store.save(key(SESSION_A, 2), stale).unwrap();

    let outcome = reparse(&logs, &mut store).unwrap();
    let changed: Vec<_> = outcome.changed.iter().map(|s| s.key.clone()).collect();
    assert_eq!(changed, vec![key(SESSION_A, 2)]);

    let reopened = Store::open(&data).unwrap();
    let report = reopened
        .games()
        .find(|(k, _)| **k == key(SESSION_A, 2))
        .map(|(_, r)| r)
        .unwrap();
    assert_eq!(report["status"], "ok");
}

#[test]
fn games_whose_logs_are_gone_are_listed_and_left_untouched() {
    let (logs, data, mut store) = setup();
    fs::remove_dir_all(logs.join(SESSION_B)).unwrap();
    let before = fs::read_to_string(data.join("games.jsonl")).unwrap();

    let outcome = reparse(&logs, &mut store).unwrap();
    assert_eq!(outcome.sessions_read, 1);
    assert!(outcome.changed.is_empty());
    assert_eq!(outcome.unavailable.len(), 1);
    assert_eq!(outcome.unavailable[0].key, key(SESSION_B, 1));
    assert_eq!(outcome.unavailable[0].reason, UnavailableReason::LogsGone);
    assert_eq!(
        fs::read_to_string(data.join("games.jsonl")).unwrap(),
        before
    );
}

#[test]
fn a_session_folder_without_a_power_log_counts_as_gone() {
    let (logs, _, mut store) = setup();
    fs::remove_file(logs.join(SESSION_B).join("Power.log")).unwrap();
    let outcome = reparse(&logs, &mut store).unwrap();
    assert_eq!(outcome.unavailable.len(), 1);
    assert_eq!(outcome.unavailable[0].reason, UnavailableReason::LogsGone);
}

#[test]
fn incomplete_logs_never_overwrite_games_whose_numbers_could_shift() {
    let (logs, data, mut store) = setup();
    // The game rotated away Power_old.log: session A now shows one game,
    // numbered 1, where the history has games 1 and 2.
    fs::remove_file(logs.join(SESSION_A).join("Power_old.log")).unwrap();
    let before = fs::read_to_string(data.join("games.jsonl")).unwrap();

    let outcome = reparse(&logs, &mut store).unwrap();
    assert!(outcome.changed.is_empty());
    let unavailable: Vec<_> = outcome
        .unavailable
        .iter()
        .map(|u| (u.key.clone(), u.reason.clone()))
        .collect();
    assert_eq!(
        unavailable,
        vec![(key(SESSION_A, 2), UnavailableReason::LogsIncomplete)]
    );
    assert_eq!(
        fs::read_to_string(data.join("games.jsonl")).unwrap(),
        before
    );
}

#[test]
fn new_records_carry_the_parser_version_and_old_ones_load_as_unknown() {
    let (_, data, store) = setup();
    let current = ParserStamp {
        version: PARSER_VERSION.to_string(),
        revision: PARSER_REVISION,
    };
    assert_eq!(store.parser(&key(SESSION_A, 1)), Some(&current));
    let history = fs::read_to_string(data.join("games.jsonl")).unwrap();
    let first: serde_json::Value = serde_json::from_str(history.lines().next().unwrap()).unwrap();
    assert_eq!(first["parser"]["version"], PARSER_VERSION);
    assert_eq!(first["parser"]["revision"], PARSER_REVISION);

    // A record written before T-107 has no parser field; another has junk.
    let old = r#"{"session":"Hearthstone_2026_01_01_00_00_00","index":1,"saved_at":1,"report":{"status":"ok"}}"#;
    let junk = r#"{"session":"Hearthstone_2026_01_01_00_00_00","index":2,"saved_at":1,"parser":"x","report":{"status":"ok"}}"#;
    append(&data.join("games.jsonl"), &format!("{old}\n{junk}\n"));
    let reopened = Store::open(&data).unwrap();
    assert_eq!(reopened.unreadable_lines, 0, "old records are not errors");
    let session = "Hearthstone_2026_01_01_00_00_00";
    assert_eq!(reopened.parser(&key(session, 1)), None);
    assert_eq!(reopened.parser(&key(session, 2)), None);
    assert_eq!(reopened.parser(&key(SESSION_A, 1)), Some(&current));
}

#[test]
fn the_build_is_kept_in_the_report_of_each_game() {
    let (_, _, store) = setup();
    let (_, report) = store.games().next().unwrap();
    assert!(report.get("build").is_some_and(|b| b.is_i64()));
}
