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
use tracker::odds::{odds_input, LiveFacts};
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

/// The line numbers of a Solo fixture's first combat: its `TURN`, the first
/// attack (where the boards enter play) and the next shop turn's `TURN`.
fn first_combat(lines: &[&str]) -> (usize, usize, usize) {
    let turn = |n: i64| {
        lines
            .iter()
            .position(|l| l.contains(&format!("Entity=GameEntity tag=TURN value={n}")))
            .expect("turn in the fixture")
    };
    let start = turn(2);
    let attack = start
        + lines[start..]
            .iter()
            .position(|l| l.contains("BlockType=ATTACK"))
            .expect("an attack");
    (start, attack, turn(3))
}

/// The combat arrives whole in one burst, with the next shop turn right
/// behind it, and the game plays it for many seconds. The odds need the
/// combat's boards as soon as the game has played them in (the first
/// attack), not once it has played the whole combat (session 192).
#[test]
fn the_combat_boards_show_when_the_game_plays_them_in_not_at_the_end() {
    let log = fixture("real/b253216_solo");
    let lines: Vec<&str> = log.lines().collect();
    let (start, attack, shop) = first_combat(&lines);
    assert!(attack + 20 < shop, "a combat with attacks before its end");

    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    let own = |lines: &[&str]| lines.iter().map(|l| l.to_string()).collect::<Vec<_>>();

    append(&power, &played_along(&lines[..start]));
    watcher.poll().unwrap();
    assert_eq!(watcher.live_combat_round(), None, "a shop turn");

    // The whole combat and the shop turn after it arrive in one poll.
    let burst_end = (shop + 10).min(lines.len());
    append(&power, &own(&lines[start..burst_end]));
    watcher.poll().unwrap();
    assert_eq!(
        watcher.live_combat_round(),
        None,
        "the screen has not played the combat in yet"
    );

    // The screen plays the boards in and the first attack.
    append(&power, &played_copies(&lines[start..=attack]));
    watcher.poll().unwrap();
    let round = watcher.live_combat_round();
    assert_eq!(round, Some(1), "the combat on screen");
    let report = watcher.live_report().expect("a game is on");
    let lobby = watcher.live_lobby();
    let live = LiveFacts {
        combat_round: round,
        lobby: &lobby,
        damage_cap: watcher.live_damage_cap(),
    };
    assert!(
        odds_input(report, live).is_ok(),
        "both boards are in the view the screen shows"
    );

    // The screen plays the rest: the shop turn, no odds.
    append(&power, &played_copies(&lines[attack + 1..burst_end]));
    watcher.poll().unwrap();
    assert_eq!(watcher.live_combat_round(), None);
}

/// The first combat of a fixture whose legs begin a start of combat before
/// their first attack, by line number: (its `TURN`, the line where the legs
/// begin, its first attack, the next shop turn's `TURN`).
fn first_combat_with_a_start(lines: &[&str]) -> Option<(usize, usize, usize, usize)> {
    let mut reader = LogReader::default();
    let mut begun: Option<(i64, usize)> = None;
    for (i, l) in lines.iter().enumerate() {
        reader.feed(l);
        let opening = reader.combat_legs_opening_now().unwrap_or(0);
        if begun.is_none() && opening == 2 && reader.combat_boards_in_now().unwrap_or(0) == 0 {
            begun = Some((reader.combat_round_now()?, i));
        }
        if let Some((round, at)) = begun {
            if reader.combat_boards_in_now().unwrap_or(0) > 0 {
                let turn = |n: i64| {
                    lines
                        .iter()
                        .position(|l| l.contains(&format!("Entity=GameEntity tag=TURN value={n}")))
                };
                return Some((turn(round * 2)?, at, i, turn(round * 2 + 1)?));
            }
        }
    }
    None
}

/// D-165: while the screen plays a combat's start of combat, the odds' input
/// is the early estimate from the boards before it; from the first attack on,
/// the boards then.
#[test]
fn the_estimate_shows_when_the_game_plays_the_start_of_combat() {
    let log = fixture("real/b253216_solo");
    let lines: Vec<&str> = log.lines().collect();
    let (start, begun, attack, shop) =
        first_combat_with_a_start(&lines).expect("a combat with a start of combat");

    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    let own = |lines: &[&str]| lines.iter().map(|l| l.to_string()).collect::<Vec<_>>();
    let input = |watcher: &Watcher| {
        let lobby = watcher.live_lobby();
        let live = LiveFacts {
            combat_round: watcher.live_combat_round(),
            lobby: &lobby,
            damage_cap: watcher.live_damage_cap(),
        };
        odds_input(watcher.live_report().expect("a game is on"), live)
    };

    append(&power, &played_along(&lines[..start]));
    watcher.poll().unwrap();
    // The whole combat and the shop turn after it arrive in one poll.
    let burst_end = (shop + 10).min(lines.len());
    append(&power, &own(&lines[start..burst_end]));
    watcher.poll().unwrap();
    assert_eq!(watcher.live_combat_round(), None, "nothing played yet");

    // The screen plays the combat in, up to its start of combat.
    append(&power, &played_copies(&lines[start..=begun]));
    watcher.poll().unwrap();
    let early = input(&watcher).expect("the estimate's input");
    assert!(early.estimate, "the boards before start of combat");

    // The screen plays the start of combat and the first attack.
    append(&power, &played_copies(&lines[begun + 1..=attack]));
    watcher.poll().unwrap();
    let odds = input(&watcher).expect("the odds' input");
    assert!(!odds.estimate, "the boards at the first attack");
    assert_eq!(
        (odds.round, odds.opponent_seat),
        (early.round, early.opponent_seat)
    );
}
