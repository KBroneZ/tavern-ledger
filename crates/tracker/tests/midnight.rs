//! A game that crosses midnight is dated by its first line, in the app's
//! history, whatever time zone the PC is in (T-359). Synthetic logs only.
//!
//! Fixture: `solo_game.log` with its timestamps rewritten. Session
//! `Hearthstone_2026_10_09_23_50_00` (local time). The first 40 lines are at
//! 23:58:30 on 9 October, the rest at 00:03:10 on 10 October, and a second
//! game written entirely after midnight (00:20) follows.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tracker::store::{GameKey, Store};
use tracker::Watcher;

const SESSION: &str = "Hearthstone_2026_10_09_23_50_00";
/// Lines of the first game written before midnight.
const BEFORE_MIDNIGHT: usize = 40;

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
        "tavern-ledger-midnight-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The synthetic session: game 1 crosses midnight, game 2 is after it.
fn crossing_log(logs: &Path) {
    let first: Vec<String> = fixture("solo_game")
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let time = if i < BEFORE_MIDNIGHT {
                "23:58:30."
            } else {
                "00:03:10."
            };
            line.replacen("D 12:00:00.", &format!("D {time}"), 1)
        })
        .collect();
    let second = fixture("duo_game").replace("D 12:00:00.", "D 00:20:00.");
    let dir = logs.join(SESSION);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Power.log"), first.join("\n") + "\n" + &second).unwrap();
}

fn key(index: u64) -> GameKey {
    GameKey {
        session: SESSION.into(),
        index,
    }
}

/// 2026-10-09 00:00:00 UTC.
const OCT_9_UTC: i64 = 1_791_504_000;

fn spans(to_utc: fn(i64) -> Option<i64>) -> ((i64, i64), (i64, i64)) {
    let root = temp_dir();
    let (logs, data) = (root.join("Logs"), root.join("data"));
    crossing_log(&logs);
    let mut watcher = Watcher::new(&logs, Store::open(&data).unwrap()).with_local_time(to_utc);
    watcher.poll().unwrap();
    let (_, store) = watcher.finish().unwrap();
    let one = store.played(&key(1)).copied().expect("game 1 has a span");
    let two = store.played(&key(2)).copied().expect("game 2 has a span");
    ((one.from, one.to), (two.from, two.to))
}

/// A PC two hours ahead of UTC.
fn plus_2(local: i64) -> Option<i64> {
    Some(local - 2 * 3600)
}

/// A PC seven hours behind UTC.
fn minus_7(local: i64) -> Option<i64> {
    Some(local + 7 * 3600)
}

fn utc(local: i64) -> Option<i64> {
    Some(local)
}

#[test]
fn a_game_across_midnight_starts_on_the_day_of_its_first_line() {
    // PC at UTC+2: 23:58:30 local on 9 Oct is 21:58:30 UTC on 9 Oct; the game ends at
    // 00:03:10 local on 10 Oct, which is 22:03:10 UTC on 9 Oct.
    let ((from, to), _) = spans(plus_2);
    assert_eq!(from, OCT_9_UTC + 21 * 3600 + 58 * 60 + 30);
    assert_eq!(to, OCT_9_UTC + 22 * 3600 + 3 * 60 + 10);
    assert!(to > from);
}

#[test]
fn a_game_written_after_midnight_is_placed_on_the_next_local_day() {
    // The session began on 9 Oct at 23:50; a game at 00:20 is on 10 Oct, local.
    let (_, (from, _)) = spans(plus_2);
    let local = from + 2 * 3600;
    assert_eq!(
        local / 86_400,
        OCT_9_UTC / 86_400 + 1,
        "10 October, on the PC's clock"
    );
    assert_eq!(local % 86_400, 20 * 60, "at 00:20");
}

#[test]
fn the_same_log_in_other_time_zones_keeps_the_same_span_lengths_and_order() {
    for (zone, to_utc) in [
        ("UTC-7", minus_7 as fn(i64) -> Option<i64>),
        ("UTC", utc),
        ("UTC+2", plus_2),
    ] {
        let ((from1, to1), (from2, _)) = spans(to_utc);
        assert_eq!(
            to1 - from1,
            4 * 60 + 40,
            "{zone}: the game's length does not depend on the zone"
        );
        assert_eq!(
            from2 - from1,
            21 * 60 + 30,
            "{zone}: game 2 starts 21 min 30 s after game 1 (23:58:30 -> 00:20:00)"
        );
    }
}
