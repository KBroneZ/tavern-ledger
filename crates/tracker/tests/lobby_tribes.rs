//! Tribes entered by hand (T-303) against the real watcher: which game is in
//! progress, where a waiting entry attaches, and that re-reading a game never
//! loses an entry.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tracker::lobby_tribes::Entries;
use tracker::store::{GameKey, Store};
use tracker::{import_session, reparse, Watcher};

const SESSION: &str = "Hearthstone_2026_10_09_10_00_00";
const FIVE: [&str; 5] = ["MURLOC", "BEAST", "UNDEAD", "NAGA", "DRAGON"];

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
        "tavern-ledger-lobby-{}-{}",
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

fn key(index: u64) -> GameKey {
    GameKey {
        session: SESSION.into(),
        index,
    }
}

fn pick() -> Vec<String> {
    FIVE.iter().map(|s| s.to_string()).collect()
}

/// A logs folder with one empty session, and a data folder.
fn setup() -> (PathBuf, PathBuf) {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    (logs, root.join("data"))
}

#[test]
fn the_game_in_progress_is_known_from_hero_select_until_it_completes() {
    let (logs, data) = setup();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    assert_eq!(watcher.in_progress(), None, "no log yet");

    let log = fixture("solo_game");
    let complete = log.find("STATE value=COMPLETE").unwrap();
    let cut = log[..complete].rfind('\n').unwrap() + 1;
    append(&power, &log[..cut]);
    watcher.poll().unwrap();
    assert_eq!(watcher.in_progress(), Some(key(1)));

    append(&power, &log[cut..]);
    watcher.poll().unwrap();
    assert_eq!(watcher.in_progress(), None, "the game is over");
}

#[test]
fn a_game_that_is_not_battlegrounds_is_never_the_game_in_progress() {
    let (logs, data) = setup();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    let log = fixture("not_battlegrounds");
    let complete = log.find("STATE value=COMPLETE").unwrap_or(log.len());
    let cut = log[..complete].rfind('\n').map_or(0, |i| i + 1);
    append(&power, &log[..cut]);
    watcher.poll().unwrap();
    assert_eq!(watcher.in_progress(), None);
}

#[test]
fn a_waiting_entry_attaches_to_the_game_in_progress_and_survives_a_reparse() {
    let (logs, data) = setup();
    let power = logs.join(SESSION).join("Power.log");
    let mut entries = Entries::open(&data).unwrap();
    entries.set_pending(&pick()).unwrap();

    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    let log = fixture("solo_game");
    let complete = log.find("STATE value=COMPLETE").unwrap();
    let cut = log[..complete].rfind('\n').unwrap() + 1;
    append(&power, &log[..cut]);
    watcher.poll().unwrap();
    let started = watcher.in_progress().expect("a game is in progress");
    assert!(entries.attach_pending(&started).unwrap());

    append(&power, &log[cut..]);
    let saved = watcher.poll().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(
        saved[0].key, started,
        "the entry is on the game that was saved"
    );

    // Re-reading the game rewrites its report; the entry stays.
    let (_, mut store) = watcher.finish().unwrap();
    let outcome = reparse(&logs, &mut store).unwrap();
    assert_eq!(outcome.sessions_read, 1);
    assert!(store.report(&started).is_some());
    let again = Entries::open(&data).unwrap();
    assert_eq!(again.entry(&started).unwrap().len(), 5);
    let report = store.report(&started).unwrap();
    assert!(
        report.get("lobby_tribes").is_none(),
        "the entry is not in the report"
    );
}

#[test]
fn importing_a_session_again_does_not_touch_the_entries_file() {
    let (logs, data) = setup();
    append(&logs.join(SESSION).join("Power.log"), &fixture("solo_game"));
    let mut store = Store::open(&data).unwrap();
    import_session(&logs.join(SESSION), &mut store).unwrap();
    let mut entries = Entries::open(&data).unwrap();
    entries.set(key(1), &pick()).unwrap();
    let before = fs::read(data.join(tracker::lobby_tribes::FILE_NAME)).unwrap();
    reparse(&logs, &mut store).unwrap();
    import_session(&logs.join(SESSION), &mut store).unwrap();
    let after = fs::read(data.join(tracker::lobby_tribes::FILE_NAME)).unwrap();
    assert_eq!(before, after);
}
