//! The overlay's counters through the watcher (T-611, D-139): held until the
//! game has played the lines they came from, also when a player enchantment is
//! created by `FULL_ENTITY` (no `TAG_CHANGE`). Synthetic log: each packet is
//! written as the game does, once as it arrives (`GameState`) and once as the
//! client plays it (`PowerTaskList`).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use bg_parser::counters::{Counter, CounterKind};
use tracker::store::Store;
use tracker::Watcher;

const SESSION: &str = "Hearthstone_2026_10_14_10_00_00";
const ARRIVED: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";
const PLAYED: &str = "D 12:00:00.0000000 PowerTaskList.DebugPrintPower() - ";

fn temp_dir() -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "tavern-ledger-counters-{}-{}",
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

fn at(kind: &str, indent: usize, body: &str) -> String {
    format!("{kind}{}{body}", "    ".repeat(indent))
}

fn tag_change(kind: &str, entity: &str, tag: &str, value: &str) -> String {
    at(
        kind,
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

/// The player enchantment `BG25_011pe` (Undead Attack) as a packet of `kind`.
fn undead_enchantment(kind: &str, attack: i64) -> Vec<String> {
    vec![
        at(kind, 1, "FULL_ENTITY - Creating ID=60 CardID=BG25_011pe"),
        at(kind, 2, "tag=CARDTYPE value=ENCHANTMENT"),
        at(kind, 2, "tag=ATTACHED value=2"),
        at(kind, 2, "tag=ZONE value=PLAY"),
        at(
            kind,
            2,
            &format!("tag=TAG_SCRIPT_DATA_NUM_1 value={attack}"),
        ),
        at(kind, 2, "tag=ENTITY_ID value=60"),
        // Closes the packet without being a TAG_CHANGE.
        at(kind, 0, "BLOCK_END"),
    ]
}

/// The game up to its first shop turn, both ways of writing it.
fn start() -> Vec<String> {
    let mut lines = vec![
        at(ARRIVED, 0, "CREATE_GAME"),
        at(ARRIVED, 1, "GameEntity EntityID=1"),
        at(ARRIVED, 2, "tag=CARDTYPE value=GAME"),
        at(ARRIVED, 2, "tag=ENTITY_ID value=1"),
        at(
            ARRIVED,
            1,
            "Player EntityID=2 PlayerID=2 GameAccountId=[hi=1 lo=1]",
        ),
        at(ARRIVED, 2, "tag=PLAYER_ID value=2"),
        at(ARRIVED, 2, "tag=CARDTYPE value=PLAYER"),
        at(ARRIVED, 2, "tag=ENTITY_ID value=2"),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - BuildNumber=253216".into(),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".into(),
        at(PLAYED, 0, "CREATE_GAME"),
    ];
    for kind in [ARRIVED, PLAYED] {
        lines.push(tag_change(kind, "GameEntity", "TURN", "1"));
    }
    lines
}

fn watcher_with_game() -> (Watcher, PathBuf) {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let power = logs.join(SESSION).join("Power.log");
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    append(&power, &start());
    watcher.poll().unwrap();
    (watcher, power)
}

#[test]
fn counters_are_empty_when_known_and_unknown_with_no_game() {
    let root = temp_dir();
    let logs = root.join("Logs");
    fs::create_dir_all(logs.join(SESSION)).unwrap();
    let mut watcher = Watcher::new(&logs, Store::open(&root.join("data")).unwrap());
    watcher.poll().unwrap();
    assert_eq!(watcher.live_counters(), None, "no game");

    let (watcher, _) = watcher_with_game();
    assert_eq!(watcher.live_counters(), Some(vec![]));
}

#[test]
fn an_enchantment_created_by_full_entity_shows_once_the_game_has_played_it() {
    let (mut watcher, power) = watcher_with_game();

    // Arrived, not played yet: the screen has not shown it.
    append(&power, &undead_enchantment(ARRIVED, 7));
    watcher.poll().unwrap();
    assert_eq!(watcher.live_counters(), Some(vec![]), "still held");

    // The client plays it: the counter shows.
    append(&power, &undead_enchantment(PLAYED, 7));
    watcher.poll().unwrap();
    assert_eq!(
        watcher.live_counters(),
        Some(vec![Counter::of_attack(CounterKind::UndeadAttack, 7)])
    );
}

#[test]
fn a_value_set_by_a_tag_change_follows_the_same_hold() {
    let (mut watcher, power) = watcher_with_game();
    let spell = |kind| tag_change(kind, "2", "TAVERN_SPELL_ATTACK_INCREASE", "2");
    append(&power, &[spell(ARRIVED)]);
    watcher.poll().unwrap();
    assert_eq!(watcher.live_counters(), Some(vec![]), "not played yet");
    append(&power, &[spell(PLAYED)]);
    watcher.poll().unwrap();
    assert_eq!(
        watcher.live_counters(),
        Some(vec![Counter::of_stats(CounterKind::TavernSpell, 2, 0)])
    );
}

#[test]
fn a_reader_error_makes_the_counters_unknown() {
    let (mut watcher, power) = watcher_with_game();
    assert!(watcher.live_counters().is_some());
    append(
        &power,
        &["D 12:00:00.0000000 GameState.DebugPrintPower() -     SOMETHING_NEW foo".to_string()],
    );
    watcher.poll().unwrap();
    assert_eq!(watcher.live_counters(), None);
}
