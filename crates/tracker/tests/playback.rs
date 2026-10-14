//! The overlay follows what the game has played on screen, not what its log
//! already says (T-335): a combat's outcome arrives in `GameState` lines long
//! before the game plays it back (`PowerTaskList` lines), and the leaderboard
//! the player sees only changes then.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use bg_parser::report::LobbySlot;
use bg_parser::LogReader;
use tracker::store::Store;
use tracker::Watcher;

const SESSION: &str = "Hearthstone_2026_10_11_10_00_00";
const ARRIVED: &str = "GameState.DebugPrintPower()";
const PLAYED: &str = "PowerTaskList.DebugPrintPower()";

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
        "tavern-ledger-playback-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn append(path: &Path, lines: &[String]) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    for line in lines {
        writeln!(f, "{line}").unwrap();
    }
}

/// An arrived packet the client plays back: a `TAG_CHANGE`, or an entity
/// created or shown.
fn is_played_back(line: &str) -> bool {
    line.contains(ARRIVED)
        && ["TAG_CHANGE", "FULL_ENTITY", "SHOW_ENTITY"]
            .iter()
            .any(|word| line.contains(word))
}

/// The lines as the game writes them while in step with the screen: each
/// arrived `TAG_CHANGE`, `FULL_ENTITY` or `SHOW_ENTITY` followed by its
/// played copy.
fn played_along(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .flat_map(|l| {
            let mut out = vec![l.to_string()];
            if is_played_back(l) {
                out.push(l.replace(ARRIVED, PLAYED));
            }
            out
        })
        .collect()
}

fn played_copies(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| is_played_back(l))
        .map(|l| l.replace(ARRIVED, PLAYED))
        .collect()
}

fn lobby_after(lines: &[&str]) -> Vec<LobbySlot> {
    let mut reader = LogReader::default();
    lines.iter().for_each(|l| reader.feed(l));
    reader.lobby_now()
}

fn places(lobby: &[LobbySlot]) -> Vec<(i64, Option<i64>)> {
    lobby.iter().map(|s| (s.player_id, s.place)).collect()
}

/// A real Duos game (scrubbed, T-108) cut in three: in step up to a combat,
/// then the combat's outcome arrives, then the game plays it back.
#[test]
fn the_leaderboard_moves_only_when_the_game_plays_the_combat_back() {
    let log = fixture("real/b253216_duos");
    let lines: Vec<&str> = log.lines().collect();
    let end = lines
        .iter()
        .position(|l| l.contains("STATE value=COMPLETE"))
        .expect("fixture completes");
    // The last place change before the end, and the combat start before it.
    let change = lines[..end]
        .iter()
        .rposition(|l| l.contains("tag=PLAYER_LEADERBOARD_PLACE") && l.contains("TAG_CHANGE"))
        .expect("places change");
    let cut = lines[..change]
        .iter()
        .rposition(|l| {
            l.contains("tag=NEXT_STEP value=MAIN_COMBAT") || l.contains("BlockType=ATTACK")
        })
        .expect("a combat before it");
    let before = lobby_after(&lines[..cut]);
    let after = lobby_after(&lines[..end]);
    assert_ne!(
        places(&before),
        places(&after),
        "the combat moves the leaderboard"
    );

    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());

    append(&power, &played_along(&lines[..cut]));
    watcher.poll().unwrap();
    assert_eq!(places(&watcher.live_lobby()), places(&before));
    let shown_report = watcher.live_report().expect("a game is on").clone();

    // The combat arrives whole; the game has not played it yet.
    append(
        &power,
        &lines[cut..end]
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>(),
    );
    watcher.poll().unwrap();
    assert_eq!(
        places(&watcher.live_lobby()),
        places(&before),
        "the screen still shows the places from before the combat"
    );
    assert_eq!(
        watcher.live_report(),
        Some(&shown_report),
        "no early result"
    );

    // Played back: the leaderboard moves.
    append(&power, &played_copies(&lines[cut..end]));
    watcher.poll().unwrap();
    assert_eq!(places(&watcher.live_lobby()), places(&after));
    assert_ne!(watcher.live_report(), Some(&shown_report));
}

/// Logs without played lines (older setups, the parser's fixtures) are not
/// held back.
#[test]
fn a_log_without_played_lines_is_shown_as_it_is_read() {
    let log = fixture("real/b253216_duos");
    let lines: Vec<&str> = log.lines().collect();
    let end = lines
        .iter()
        .position(|l| l.contains("STATE value=COMPLETE"))
        .unwrap();
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    append(
        &power,
        &lines[..end]
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>(),
    );
    watcher.poll().unwrap();
    assert_eq!(
        places(&watcher.live_lobby()),
        places(&lobby_after(&lines[..end]))
    );
    assert!(watcher.live_report().is_some());
}
