//! The overlay's live state against the saved report, on synthetic logs
//! (bg-parser's fixtures): live and saved must say the same.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use bg_parser::LogReader;
use tracker::live::{live_game, LiveGame, Phase};
use tracker::scene::Scene;
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

const LOADING_FROM_LOBBY: &str = "D 10:00:01.0000000 LoadingScreen.OnScenePreUnload() - prevMode=BACON nextMode=GAMEPLAY m_phase=INVALID\n";

fn scene_line(prev: &str, curr: &str) -> String {
    format!("D 10:00:00.0000000 LoadingScreen.OnSceneLoaded() - prevMode={prev} currMode={curr}\n")
}

/// The game in progress is cut off (no end) and the player went back to the
/// lobby: with LoadingScreen.log the overlay's game ends (T-314).
#[test]
fn leaving_the_game_screen_ends_the_live_game_of_a_cut_off_log() {
    let root = temp_dir();
    let logs = root.join("Logs");
    let session = logs.join(SESSION);
    fs::create_dir_all(&session).unwrap();
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    let log = fixture("duo_game_incomplete");
    append(&session.join("Power.log"), &log);
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), None, "no LoadingScreen.log: unknown");
    assert!(
        watcher.live_report().is_some(),
        "as before without the file"
    );

    let scenes = session.join("LoadingScreen.log");
    append(&scenes, &scene_line("HUB", "BACON"));
    append(&scenes, LOADING_FROM_LOBBY);
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::BattlegroundsLoading));
    assert!(watcher.live_report().is_some());
    append(&scenes, &scene_line("BACON", "GAMEPLAY"));
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::Game));
    assert!(watcher.live_report().is_some());
    assert!(watcher.in_progress().is_some());

    append(&scenes, &scene_line("GAMEPLAY", "BACON"));
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::BattlegroundsLobby));
    assert!(watcher.live_report().is_none(), "the player left the game");
    assert!(watcher.in_progress().is_none());
    assert!(watcher.live_lobby().is_empty());
    assert_eq!(watcher.live_next_opponent(), None);

    // The next game loads (the order of the real logs: the lobby closes,
    // CREATE_GAME, then the game screen loads): the game that was left
    // does not come back meanwhile.
    append(&scenes, LOADING_FROM_LOBBY);
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::BattlegroundsLoading));
    assert!(watcher.live_report().is_none(), "the left game stays left");
    assert!(watcher.in_progress().is_none());

    // Its CREATE_GAME arrives, then the game screen: the new game is on.
    append(&session.join("Power.log"), &fixture("duo_game_incomplete"));
    watcher.poll().unwrap();
    assert!(watcher.live_report().is_some(), "the new game");
    append(&scenes, &scene_line("BACON", "GAMEPLAY"));
    watcher.poll().unwrap();
    assert!(watcher.live_report().is_some());
    assert!(watcher.in_progress().is_some());
}

#[test]
fn the_watcher_gives_the_next_opponent_of_the_game_on() {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    let log = fixture("solo_game");
    let turn = log.find("tag=TURN value=1").expect("a shop turn");
    let cut = log[..turn].rfind('\n').unwrap() + 1;
    append(&power, &log[..cut]);
    append(
        &power,
        "D 12:00:00.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=10 tag=NEXT_OPPONENT_PLAYER_ID value=3 \n",
    );
    watcher.poll().unwrap();
    assert_eq!(watcher.live_next_opponent(), Some(3));
}

/// Power.log and LoadingScreen.log are written apart: a new game's
/// CREATE_GAME can be read while the scene still says "lobby". The game is
/// hidden only until the scene reaches the game, never for good (review H1).
#[test]
fn a_game_read_before_its_scene_change_shows_once_the_scene_catches_up() {
    let root = temp_dir();
    let logs = root.join("Logs");
    let session = logs.join(SESSION);
    fs::create_dir_all(&session).unwrap();
    let scenes = session.join("LoadingScreen.log");
    append(&scenes, &scene_line("HUB", "BACON"));
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    watcher.poll().unwrap();
    append(&session.join("Power.log"), &fixture("duo_game_incomplete"));
    watcher.poll().unwrap();
    assert!(
        watcher.live_report().is_none(),
        "the scene says lobby for now"
    );

    append(&scenes, LOADING_FROM_LOBBY);
    append(&scenes, &scene_line("BACON", "GAMEPLAY"));
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::Game));
    assert!(
        watcher.live_report().is_some(),
        "the game shows once the scene is the game"
    );
    assert!(watcher.in_progress().is_some());
}

#[test]
fn a_new_session_starts_with_an_unknown_scene() {
    let root = temp_dir();
    let logs = root.join("Logs");
    let first = logs.join(SESSION);
    fs::create_dir_all(&first).unwrap();
    append(
        &first.join("LoadingScreen.log"),
        &scene_line("HUB", "BACON"),
    );
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::BattlegroundsLobby));
    fs::create_dir_all(logs.join("Hearthstone_2026_10_09_11_00_00")).unwrap();
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), None, "nothing read in the new session yet");
    assert_eq!(watcher.scene_error(), None);
}

/// LoadingScreen.log rotates like Power.log: the lines left in the old file
/// are read before the new one (review M3).
#[test]
fn the_scene_file_is_followed_across_a_rotation() {
    let root = temp_dir();
    let logs = root.join("Logs");
    let session = logs.join(SESSION);
    fs::create_dir_all(&session).unwrap();
    let scenes = session.join("LoadingScreen.log");
    append(&scenes, &scene_line("HUB", "BACON"));
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    watcher.poll().unwrap();
    assert_eq!(watcher.scene(), Some(Scene::BattlegroundsLobby));
    append(&scenes, &scene_line("BACON", "GAMEPLAY"));
    fs::rename(&scenes, session.join("LoadingScreen_old.log")).unwrap();
    append(&scenes, "D 10:01:00.0000000 LoadingScreen.FadeIn()\n");
    watcher.poll().unwrap();
    assert_eq!(
        watcher.scene(),
        Some(Scene::Game),
        "the old file's last line counted"
    );
    assert_eq!(watcher.scene_error(), None);
}
