//! The overlay's live state against the saved report, on synthetic logs
//! (bg-parser's fixtures): live and saved must say the same.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use bg_parser::LogReader;
use tracker::live::{live_game, LiveGame, Phase};
use tracker::store::Store;
use tracker::Watcher;

const SESSION: &str = "Hearthstone_2026_10_09_10_00_00";

fn fixture(name: &str) -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../bg-parser/tests/data/{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n")
}

fn live_of(report: &bg_parser::report::GameReport) -> LiveGame {
    live_game(&serde_json::to_value(report).expect("serializable"))
}

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tavern-ledger-live-{}-{}",
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

#[test]
fn live_state_equals_the_saved_report_for_the_same_log() {
    for name in [
        "solo_game",
        "solo_game_friendly",
        "solo_local_eliminated",
        "solo_local_eliminated_after_opponent",
        "duo_game",
        "duo_game_hidden_leg",
        "duo_game_incomplete",
    ] {
        let mut reader = LogReader::default();
        fixture(name).lines().for_each(|l| reader.feed(l));
        let live = live_of(&reader.snapshot_current().expect("a game is on"));
        let saved = reader.finish().remove(0);
        assert_eq!(live, live_of(&saved), "{name}");
        assert!(!live.opponents.is_empty(), "{name}: something to compare");
    }
}

#[test]
fn the_live_turn_follows_the_shop_phases_of_the_log() {
    // TURN 1 is the first shop (turn 1), TURN 2 its combat (still turn 1),
    // TURN 3 the second shop (turn 2), and so on (D-049).
    for name in [
        "solo_game",
        "duo_game",
        "real/b253216_solo",
        "real/b253216_duos",
    ] {
        let log = fixture(name);
        let mut reader = LogReader::default();
        let mut seen = Vec::new();
        for line in log.lines() {
            reader.feed(line);
            if line.contains("Entity=GameEntity tag=TURN value=") {
                let Some(report) = reader.snapshot_current() else {
                    continue;
                };
                let tag: i64 = line
                    .rsplit("value=")
                    .next()
                    .and_then(|v| v.trim().parse().ok())
                    .expect("a turn number");
                let turn = live_of(&report).turn.value;
                if tag >= 1 {
                    assert_eq!(turn, Some((tag + 1) / 2), "{name} TURN {tag}");
                }
                seen.push(turn);
            }
        }
        assert!(seen.contains(&Some(2)), "{name}: reaches turn 2");
        if name.starts_with("real/") {
            assert!(seen.contains(&Some(4)), "{name}: reaches turn 4");
        }
    }
}

#[test]
fn a_finished_game_reads_as_over_and_an_unfinished_one_as_playing() {
    let phase_of = |name: &str| {
        let mut reader = LogReader::default();
        fixture(name).lines().for_each(|l| reader.feed(l));
        live_of(&reader.snapshot_current().unwrap()).phase
    };
    assert_eq!(phase_of("duo_game"), Phase::Over);
    assert_eq!(phase_of("duo_game_incomplete"), Phase::Playing);
}

#[test]
fn the_watcher_gives_the_live_game_while_it_is_on_and_not_after() {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    assert!(watcher.poll().unwrap().is_empty());
    assert!(watcher.live_report().is_none(), "no game yet");

    let log = fixture("duo_game");
    let complete = log.find("STATE value=COMPLETE").expect("fixture completes");
    let cut = log[..complete].rfind('\n').unwrap() + 1;
    append(&power, &log[..cut]);
    assert!(watcher.poll().unwrap().is_empty());
    let live = watcher.live_report().expect("a game is on").clone();
    let lobby = watcher.live_lobby();
    assert!(!lobby.is_empty(), "the leaderboard of the game on");
    let with_board = live_of(&live).with_leaderboard(&lobby);
    assert!(with_board.own_place.value.is_some());

    // Same lines through the parser alone: the same state.
    let mut reader = LogReader::default();
    log[..cut].lines().for_each(|l| reader.feed(l));
    assert_eq!(live, reader.snapshot_current().unwrap());
    assert_eq!(live_of(&live).phase, Phase::Playing);

    append(&power, &log[cut..]);
    assert_eq!(watcher.poll().unwrap().len(), 1, "the game is saved");
    assert!(watcher.live_report().is_none(), "no game on any more");
    assert!(watcher.live_lobby().is_empty(), "no leaderboard either");
}

#[test]
fn the_live_report_follows_new_lines_between_polls() {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    let log = fixture("duo_game_incomplete");
    let (head, tail) = log.split_at(log.len() / 2);
    let head_end = head.rfind('\n').unwrap() + 1;
    append(&power, &log[..head_end]);
    watcher.poll().unwrap();
    let early = watcher.live_report().map(|r| r.rounds.len());
    append(&power, &log[head_end..]);
    let _ = tail;
    watcher.poll().unwrap();
    let late = watcher.live_report().map(|r| r.rounds.len());
    assert!(late >= early, "more of the game is known, never less");
    assert_ne!(early, None);
}
