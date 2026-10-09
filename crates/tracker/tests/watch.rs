//! Follows synthetic logs (bg-parser's fixtures) written the way the game
//! writes them: in pieces, rotated, or in a new session folder.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tracker::store::Store;
use tracker::{import_session, Watcher};

const SESSION_A: &str = "Hearthstone_2026_10_09_10_00_00";
const SESSION_B: &str = "Hearthstone_2026_10_09_12_00_00";

fn fixture(name: &str) -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../bg-parser/tests/data/{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n")
}

/// A fresh, empty folder under the system temp dir.
fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tavern-ledger-test-{}-{}",
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

fn setup() -> (PathBuf, PathBuf) {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION_A)).unwrap();
    (logs, root.join("data"))
}

fn statuses(saved: &[tracker::Saved]) -> Vec<(u64, String)> {
    saved
        .iter()
        .map(|s| {
            (
                s.key.index,
                serde_json::to_value(s.report.status)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string(),
            )
        })
        .collect()
}

#[test]
fn a_game_written_in_pieces_is_saved_once_when_complete() {
    let (logs, data) = setup();
    let power = logs.join(SESSION_A).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    assert!(watcher.poll().unwrap().is_empty(), "no Power.log yet");

    let log = fixture("solo_game");
    let cut = log.find("STATE value=COMPLETE").unwrap() + 5; // mid-line
    append(&power, &log[..cut]);
    assert!(
        watcher.poll().unwrap().is_empty(),
        "half a line is not read yet"
    );

    append(&power, &log[cut..]);
    let saved = watcher.poll().unwrap();
    assert_eq!(statuses(&saved), vec![(1, "ok".into())]);
    assert_eq!(saved[0].report.final_place, Some(3));
    assert!(watcher.poll().unwrap().is_empty());
    let (at_finish, _) = watcher.finish().unwrap();
    assert!(at_finish.is_empty(), "same report, not saved twice");
}

#[test]
fn a_rotated_log_is_read_to_the_end_before_the_new_one() {
    let (logs, data) = setup();
    let dir = logs.join(SESSION_A);
    let first = fixture("duo_game");
    let (head, tail) = first.split_at(first.len() / 2);
    append(&dir.join("Power.log"), head);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    assert!(watcher.poll().unwrap().is_empty());

    // The game renames the full log and starts a new one.
    append(&dir.join("Power.log"), tail);
    fs::rename(dir.join("Power.log"), dir.join("Power_old.log")).unwrap();
    append(&dir.join("Power.log"), &fixture("solo_game"));
    let saved = watcher.poll().unwrap();
    assert_eq!(statuses(&saved), vec![(1, "ok".into()), (2, "ok".into())]);
}

#[test]
fn a_new_session_folder_closes_the_previous_one() {
    let (logs, data) = setup();
    append(
        &logs.join(SESSION_A).join("Power.log"),
        &fixture("duo_game_incomplete"),
    );
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    assert!(
        watcher.poll().unwrap().is_empty(),
        "the game is still going"
    );
    assert_eq!(watcher.session(), Some(SESSION_A));

    fs::create_dir_all(logs.join(SESSION_B)).unwrap();
    append(
        &logs.join(SESSION_B).join("Power.log"),
        &fixture("solo_game"),
    );
    let saved = watcher.poll().unwrap();
    assert_eq!(
        statuses(&saved),
        vec![(1, "incomplete".into()), (1, "ok".into())]
    );
    assert_eq!(saved[0].key.session, SESSION_A);
    assert_eq!(saved[1].key.session, SESSION_B);
    assert_eq!(watcher.session(), Some(SESSION_B));
}

#[test]
fn restarting_reads_the_session_again_without_duplicates() {
    let (logs, data) = setup();
    append(
        &logs.join(SESSION_A).join("Power.log"),
        &(fixture("duo_game") + &fixture("solo_game")),
    );
    let mut first = Watcher::new(&logs, Store::open(&data).unwrap());
    assert_eq!(first.poll().unwrap().len(), 2);
    drop(first);

    let mut second = Watcher::new(&logs, Store::open(&data).unwrap());
    assert!(second.poll().unwrap().is_empty());
    assert_eq!(second.store().games().count(), 2);
    let lines = fs::read_to_string(data.join("games.jsonl")).unwrap();
    assert_eq!(lines.lines().count(), 2);
}

#[test]
fn other_game_modes_are_not_saved_but_unsupported_games_are() {
    let (logs, data) = setup();
    let text = fixture("not_battlegrounds") + &fixture("missing_own_hero");
    append(&logs.join(SESSION_A).join("Power.log"), &text);
    let watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    let mut watcher = watcher;
    watcher.poll().unwrap();
    let (saved, _) = watcher.finish().unwrap();
    assert_eq!(statuses(&saved), vec![(2, "unsupported".into())]);
}

#[test]
fn the_history_keeps_the_last_record_and_reports_broken_lines() {
    let (_, data) = setup();
    fs::create_dir_all(&data).unwrap();
    let record = |place: u32| {
        format!("{{\"session\":\"S\",\"index\":1,\"saved_at\":0,\"report\":{{\"final_place\":{place}}}}}\n")
    };
    append(
        &data.join("games.jsonl"),
        &(record(4) + "{\"session\":\"S\",\"ind" + "\n" + &record(2)),
    );
    let store = Store::open(&data).unwrap();
    assert_eq!(store.unreadable_lines, 1);
    let games: Vec<_> = store.games().collect();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].1["final_place"], 2);
}

#[test]
fn import_reads_old_then_current_log_of_a_session() {
    let (logs, data) = setup();
    let dir = logs.join(SESSION_A);
    append(&dir.join("Power_old.log"), &fixture("duo_game"));
    append(&dir.join("Power.log"), &fixture("solo_game"));
    let mut store = Store::open(&data).unwrap();
    let saved = import_session(&dir, &mut store).unwrap();
    assert_eq!(statuses(&saved), vec![(1, "ok".into()), (2, "ok".into())]);
    assert!(
        import_session(&dir, &mut store).unwrap().is_empty(),
        "importing twice adds nothing"
    );
}

#[test]
fn the_history_never_contains_player_names() {
    let (logs, data) = setup();
    append(
        &logs.join(SESSION_A).join("Power.log"),
        &fixture("two_games_new_player_id"),
    );
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    watcher.poll().unwrap();
    watcher.finish().unwrap();
    let history = fs::read_to_string(data.join("games.jsonl")).unwrap();
    assert!(!history.is_empty());
    assert!(!history.contains("SyntheticPlayer"));
    assert!(!history.contains("Innkeeper"));
}

#[test]
fn a_history_cut_short_by_a_crash_keeps_the_next_record_readable() {
    let (_, data) = setup();
    fs::create_dir_all(&data).unwrap();
    append(
        &data.join("games.jsonl"),
        "{\"session\":\"S\",\"index\":1,\"rep",
    );
    append(&data.join("games.jsonl"), "\n\u{0}");
    let mut raw = fs::read(data.join("games.jsonl")).unwrap();
    raw.extend_from_slice(&[0xff, 0xfe]); // invalid UTF-8, no newline at the end
    fs::write(data.join("games.jsonl"), raw).unwrap();

    let mut store = Store::open(&data).unwrap();
    assert_eq!(store.unreadable_lines, 2);
    let key = tracker::store::GameKey {
        session: "S".into(),
        index: 2,
    };
    assert!(store
        .save(key, serde_json::json!({"final_place": 3}))
        .unwrap());

    let reopened = Store::open(&data).unwrap();
    assert_eq!(reopened.unreadable_lines, 2);
    assert_eq!(reopened.games().count(), 1);
}

#[test]
fn a_rotation_is_seen_even_if_the_new_log_is_already_longer() {
    let (logs, data) = setup();
    let dir = logs.join(SESSION_A);
    let first = format!(
        "D 10:00:00.0000000 Other.Start() - a\n{}",
        fixture("duo_game")
    );
    let cut = first
        .find("TAG_CHANGE Entity=GameEntity tag=TURN value=3")
        .unwrap();
    append(&dir.join("Power.log"), &first[..cut]);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    assert!(watcher.poll().unwrap().is_empty());

    // Rotated and the new file outgrew the old offset before the next poll
    // (NTFS may even keep the old creation time): the first bytes differ.
    append(&dir.join("Power.log"), &first[cut..]);
    fs::rename(dir.join("Power.log"), dir.join("Power_old.log")).unwrap();
    let second = format!(
        "D 11:00:00.0000000 Other.Start() - b\n{}{}",
        fixture("solo_game"),
        fixture("solo_game")
    );
    append(&dir.join("Power.log"), &second);
    let saved = watcher.poll().unwrap();
    assert_eq!(
        statuses(&saved),
        vec![(1, "ok".into()), (2, "ok".into()), (3, "ok".into())]
    );
}

#[test]
fn the_last_line_of_a_session_without_newline_is_not_lost() {
    let (logs, data) = setup();
    let log = fixture("solo_game");
    append(
        &logs.join(SESSION_A).join("Power.log"),
        log.trim_end_matches('\n'),
    );
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    assert!(
        watcher.poll().unwrap().is_empty(),
        "STATE=COMPLETE is the unfinished last line"
    );

    fs::create_dir_all(logs.join(SESSION_B)).unwrap();
    let saved = watcher.poll().unwrap();
    assert_eq!(statuses(&saved), vec![(1, "ok".into())]);
}

#[test]
fn a_line_too_long_marks_the_game_unsupported() {
    let (logs, data) = setup();
    let log = fixture("solo_game");
    let cut = log.find("D 12:00:00.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=GameEntity tag=STATE").unwrap();
    let huge = "y".repeat(bg_parser::lines::MAX_LINE + 5);
    let text = format!("{}{huge}\n{}", &log[..cut], &log[cut..]);
    append(&logs.join(SESSION_A).join("Power.log"), &text);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap());
    // A broken game stops being read, so it is saved when it closes (next
    // game, new session or app exit), not at STATE=COMPLETE.
    assert!(watcher.poll().unwrap().is_empty());
    let (saved, _) = watcher.finish().unwrap();
    assert_eq!(statuses(&saved), vec![(1, "unsupported".into())]);
}
