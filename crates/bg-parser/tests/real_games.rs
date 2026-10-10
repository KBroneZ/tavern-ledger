//! Real games from the user's own logs (T-108, D-044), one or more per game
//! build, trimmed and scrubbed by `tools/make_fixture.py`: no player names,
//! no account ids. Each `tests/data/real/<name>.log` has the report the
//! parser gave when it was made (`<name>.json`, checked by hand). A game
//! patch that changes the log shows up here as a changed report.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use bg_parser::report::{GameReport, Status};
use bg_parser::TESTED_BUILDS;
use serde_json::Value;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real")
}

fn fixtures() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir())
        .expect("tests/data/real")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "log"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn read(name: &str) -> (String, Vec<GameReport>) {
    let text = fs::read_to_string(dir().join(format!("{name}.log"))).expect("fixture");
    let reports = bg_parser::parse_reader(text.as_bytes()).expect("in-memory read");
    (text, reports)
}

fn game(name: &str) -> GameReport {
    let (_, mut reports) = read(name);
    assert_eq!(reports.len(), 1, "{name}: one game per fixture");
    reports.remove(0)
}

#[test]
fn every_real_game_gives_its_saved_report() {
    let names = fixtures();
    assert!(!names.is_empty());
    for name in names {
        let (_, reports) = read(&name);
        let expected: Value = serde_json::from_str(
            &fs::read_to_string(dir().join(format!("{name}.json"))).expect("expected report"),
        )
        .expect("valid json");
        let actual = serde_json::to_value(&reports).expect("serializable");
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn every_real_game_is_a_clean_read_of_a_tested_build() {
    for name in fixtures() {
        let report = game(&name);
        assert_eq!(report.status, Status::Ok, "{name}");
        assert!(report.warnings.is_empty(), "{name}: {:?}", report.warnings);
        assert!(report.problems.is_empty(), "{name}: {:?}", report.problems);
        let build = report.build.expect("build in the log");
        assert!(TESTED_BUILDS.contains(&build), "{name}: build {build}");
    }
}

/// A build is only "tested" when a real game of it is here.
#[test]
fn every_tested_build_has_a_real_game() {
    let covered: BTreeSet<i64> = fixtures()
        .iter()
        .filter_map(|name| game(name).build)
        .collect();
    for build in TESTED_BUILDS {
        assert!(covered.contains(build), "build {build} has no real fixture");
    }
}

#[test]
fn no_player_name_or_account_id_is_in_a_fixture() {
    for name in fixtures() {
        let (text, _) = read(&name);
        for (n, line) in text.lines().enumerate() {
            let at = format!("{name}.log line {}", n + 1);
            assert!(
                line.starts_with("D 00:00:00.0000000 GameState.DebugPrintPower() - ")
                    || line.starts_with("D 00:00:00.0000000 GameState.DebugPrintGame() - "),
                "{at}: line outside the allow-list"
            );
            let bytes = line.as_bytes();
            assert!(
                !bytes
                    .windows(2)
                    .any(|w| w[0] == b'#' && w[1].is_ascii_digit()),
                "{at}: BattleTag-like text"
            );
            if let Some(pos) = line.find("GameAccountId=") {
                let id = &line[pos..];
                assert!(
                    id.starts_with("GameAccountId=[hi=0 lo=0]")
                        || id.starts_with("GameAccountId=[hi=1 lo="),
                    "{at}: account id is not a placeholder"
                );
            }
            if let Some(pos) = line.find("PlayerName=") {
                let player = &line[pos + "PlayerName=".len()..];
                let number = player
                    .strip_prefix("Player")
                    .or_else(|| player.strip_prefix("Opponent"));
                assert!(
                    number.is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())),
                    "{at}: player name is not a placeholder"
                );
            }
            // Names and card text in brackets are dropped (D-017): ids only.
            assert!(
                !line.contains("[entityName="),
                "{at}: bracketed entity text"
            );
        }
    }
}

// Checked by hand when the fixtures were made (session 032).

#[test]
fn solo_build_253216() {
    let r = game("b253216_solo");
    assert_eq!(r.game_type.as_deref(), Some("GT_BATTLEGROUNDS"));
    assert_eq!(r.hero.as_deref(), Some("BG21_HERO_000_SKIN_D"));
    assert_eq!(r.teammate_hero, None);
    assert_eq!(r.final_place, Some(8));
    assert_eq!(r.final_health, Some(0));
    assert_eq!(r.lobby.len(), 8);
    let mut places: Vec<_> = r.lobby.iter().filter_map(|p| p.final_place).collect();
    places.sort();
    assert_eq!(places, (1..=8).collect::<Vec<_>>());
    assert_eq!(r.rounds.len(), 9);
    let healths: Vec<_> = r.rounds.iter().map(|r| r.own_health_after).collect();
    assert_eq!(&healths[6..], &[Some(27), Some(12), Some(0)]);
    // One combat per round, both boards seen, never more than 7 minions.
    for round in &r.rounds {
        assert_eq!(round.entries.len(), 2, "round {}", round.number);
        for entry in &round.entries {
            let board = entry.board.as_ref().expect("board seen");
            assert!((1..=7).contains(&board.len()), "round {}", round.number);
        }
    }
}

#[test]
fn duos_build_253216() {
    let r = game("b253216_duos");
    assert_eq!(r.game_type.as_deref(), Some("GT_BATTLEGROUNDS_DUO"));
    assert_eq!(r.hero.as_deref(), Some("BG24_HERO_100_SKIN_E"));
    assert_eq!(
        r.teammate_hero.as_deref(),
        Some("TB_BaconShop_HERO_43_SKIN_N")
    );
    assert_eq!(r.final_place, Some(4));
    assert_eq!(r.final_health, Some(0));
    assert_eq!(r.rounds.len(), 8);
    // Teammates share a team, a place and a health pool; four teams, places 1-4.
    let team = |pid| {
        r.lobby
            .iter()
            .find(|p| p.player_id == pid)
            .expect("in lobby")
    };
    let (me, mate) = (team(2), team(1));
    assert_eq!(me.duo_team, mate.duo_team);
    assert_eq!((me.final_place, mate.final_place), (Some(4), Some(4)));
    let places: BTreeSet<_> = r.lobby.iter().filter_map(|p| p.final_place).collect();
    assert_eq!(places, (1..=4).collect());
    // The log hides some legs: they are "not visible", never guessed.
    let hidden = r
        .rounds
        .iter()
        .flat_map(|r| &r.entries)
        .filter(|e| e.board.is_none())
        .count();
    assert_eq!(hidden, 3);
}
