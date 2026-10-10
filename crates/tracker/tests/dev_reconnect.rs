//! Games played with the reconnect dev tool are found from the log's clock
//! and the tool's file (T-D01, D-043), with synthetic logs only.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tracker::dev_reconnect::{Reconnects, FILE_NAME};
use tracker::store::{GameKey, Store};
use tracker::Watcher;

const SESSION: &str = "Hearthstone_2026_10_09_10_00_00";

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
        "tavern-ledger-devrc-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The local clock is two hours ahead of UTC.
fn utc_plus_2(local: i64) -> Option<i64> {
    Some(local - 2 * 3600)
}

fn key(index: u64) -> GameKey {
    GameKey {
        session: SESSION.into(),
        index,
    }
}

/// Two games in one session: the first at 11:00 local, the second at 12:30.
fn two_games(logs: &Path) {
    let first = fixture("solo_game").replace("D 12:00:00.", "D 11:00:00.");
    let second = fixture("duo_game").replace("D 12:00:00.", "D 12:30:00.");
    let dir = logs.join(SESSION);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Power.log"), first + &second).unwrap();
}

#[test]
fn only_the_game_the_tool_was_used_in_is_marked() {
    let root = temp_dir();
    let (logs, data) = (root.join("Logs"), root.join("data"));
    two_games(&logs);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap()).with_local_time(utc_plus_2);
    watcher.poll().unwrap();
    let (_, store) = watcher.finish().unwrap();

    let first = store.played(&key(1)).copied().expect("first game span");
    let second = store.played(&key(2)).copied().expect("second game span");
    // 2026-10-09 09:00:00 and 10:30:00 UTC.
    assert_eq!(first.from, 1_791_536_400);
    assert_eq!(second.from, 1_791_541_800);

    // 11:00:10 local = 09:00:10 UTC: during the first game only.
    fs::write(
        data.join(FILE_NAME),
        "{\"utc\": \"2026-10-09T09:00:10Z\"}\n",
    )
    .unwrap();
    let reconnects = Reconnects::load(&data);
    assert!(reconnects.marks(store.played(&key(1))));
    assert!(!reconnects.marks(store.played(&key(2))));
}

#[test]
fn the_span_survives_reopening_the_history_and_a_save_without_one() {
    let root = temp_dir();
    let (logs, data) = (root.join("Logs"), root.join("data"));
    two_games(&logs);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap()).with_local_time(utc_plus_2);
    watcher.poll().unwrap();
    let (_, store) = watcher.finish().unwrap();
    let span = store.played(&key(1)).copied();
    assert!(span.is_some());

    let mut store = Store::open(&data).unwrap();
    assert_eq!(store.played(&key(1)).copied(), span);
    let mut changed = store.report(&key(1)).unwrap().clone();
    changed["final_place"] = serde_json::json!(8);
    assert!(store.save(key(1), changed).unwrap());
    assert_eq!(
        Store::open(&data).unwrap().played(&key(1)).copied(),
        span,
        "unknown never erases known"
    );
}

#[test]
fn without_a_time_zone_no_game_has_a_span_and_none_is_marked() {
    let root = temp_dir();
    let (logs, data) = (root.join("Logs"), root.join("data"));
    two_games(&logs);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap()).with_local_time(|_| None);
    watcher.poll().unwrap();
    let (_, store) = watcher.finish().unwrap();
    assert_eq!(store.games().count(), 2);
    fs::write(
        data.join(FILE_NAME),
        "{\"utc\": \"2026-10-09T09:00:10Z\"}\n",
    )
    .unwrap();
    let reconnects = Reconnects::load(&data);
    assert_eq!(store.played(&key(1)), None);
    assert!(!reconnects.marks(store.played(&key(1))));
}

#[test]
fn a_missing_or_broken_file_never_stops_the_tracker() {
    let root = temp_dir();
    let (logs, data) = (root.join("Logs"), root.join("data"));
    two_games(&logs);
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join(FILE_NAME), [0xff, 0xfe, b'\n', b'{']).unwrap();
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap()).with_local_time(utc_plus_2);
    watcher.poll().unwrap();
    let (_, store) = watcher.finish().unwrap();
    assert_eq!(store.games().count(), 2);
    let reconnects = Reconnects::load(&data);
    assert!(reconnects.warning.is_some());
    assert!(!reconnects.marks(store.played(&key(1))));
}
