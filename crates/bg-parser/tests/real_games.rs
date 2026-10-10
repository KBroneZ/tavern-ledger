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

/// "D hh:mm:ss.fffffff " (time since the game's first line, D-046), then a
/// power or game line, or an option or choice header that holds numbers only.
fn allowed(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("D ") else {
        return false;
    };
    let (time, rest) = rest.split_at(rest.len().min(17));
    let time_ok = time.len() == 17
        && time.bytes().enumerate().all(|(i, b)| match i {
            2 | 5 => b == b':',
            8 => b == b'.',
            16 => b == b' ',
            _ => b.is_ascii_digit(),
        });
    let numbers = |text: &str, keys: &[&str]| {
        let fields: Vec<&str> = text.split(' ').collect();
        fields.len() == keys.len()
            && fields.iter().zip(keys).all(|(field, key)| {
                field.strip_prefix(key).is_some_and(|v| {
                    let v = v.strip_prefix('-').unwrap_or(v);
                    !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())
                })
            })
    };
    let body_ok = rest.starts_with("GameState.DebugPrintPower() - ")
        || rest.starts_with("GameState.DebugPrintGame() - ")
        || rest
            .strip_prefix("GameState.SendOption() - ")
            .is_some_and(|b| {
                numbers(
                    b,
                    &[
                        "selectedOption=",
                        "selectedSubOption=",
                        "selectedTarget=",
                        "selectedPosition=",
                    ],
                )
            })
        || rest
            .strip_prefix("GameState.SendChoices() - ")
            .and_then(|b| b.split_once(" ChoiceType="))
            .is_some_and(|(id, kind)| {
                numbers(id, &["id="]) && kind.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
            });
    time_ok && body_ok
}

#[test]
fn the_allow_list_check_knows_each_line_kind() {
    assert!(allowed(
        "D 00:01:02.0030000 GameState.DebugPrintPower() - BLOCK_END"
    ));
    assert!(allowed(
        "D 00:01:02.0030000 GameState.SendOption() - selectedOption=1 selectedSubOption=-1 \
         selectedTarget=336 selectedPosition=0"
    ));
    assert!(allowed(
        "D 00:01:02.0030000 GameState.SendChoices() - id=3 ChoiceType=GENERAL"
    ));
    for bad in [
        "D 00:01:02 GameState.DebugPrintPower() - BLOCK_END",
        "D 00:01:02.0030000 PowerTaskList.DebugPrintPower() - BLOCK_END",
        "D 00:01:02.0030000 GameState.SendChoices() -   m_chosenEntities[0]=5",
        "D 00:01:02.0030000 GameState.SendOption() - selectedOption=1 Someone",
    ] {
        assert!(!allowed(bad), "{bad}");
    }
}

#[test]
fn no_player_name_or_account_id_is_in_a_fixture() {
    for name in fixtures() {
        let (text, _) = read(&name);
        for (n, line) in text.lines().enumerate() {
            let at = format!("{name}.log line {}", n + 1);
            assert!(allowed(line), "{at}: line outside the allow-list");
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

// Shop and actions (T-204, T-205), checked against counts of the raw logs in
// session 034 (docs/research/shop-and-apm.md): options and choices sent,
// top-level shop blocks, gold tags and tier changes.

fn shop(name: &str) -> bg_parser::shop::ShopRecord {
    game(name).shop.expect("a shop record")
}

fn sum(s: &bg_parser::shop::ShopRecord, f: impl Fn(&bg_parser::shop::ShopTurn) -> u32) -> u32 {
    s.turns.iter().map(f).sum()
}

#[test]
fn solo_shop_build_253216() {
    let s = shop("b253216_solo");
    assert_eq!(s.turns.len(), 9);
    assert_eq!((sum(&s, |t| t.rolls), sum(&s, |t| t.free_rolls)), (10, 5));
    assert_eq!(sum(&s, |t| t.buys.len() as u32), 11);
    assert_eq!(sum(&s, |t| t.spell_buys), 3);
    assert_eq!(sum(&s, |t| t.sells.len() as u32), 8);
    assert_eq!(sum(&s, |t| t.freezes), 2);
    let gold: Vec<_> = s.turns.iter().map(|t| t.gold).collect();
    let expected: Vec<_> = [3, 4, 5, 6, 7, 8, 9, 10, 14].map(Some).into();
    assert_eq!(gold, expected);
    // Spent over the game: the last NUM_RESOURCES_SPENT_THIS_GAME.
    assert_eq!(s.turns.iter().filter_map(|t| t.gold_spent).sum::<i64>(), 78);
    let tiers: Vec<_> = s.tier_ups.iter().map(|u| (u.turn, u.tier)).collect();
    assert_eq!(tiers, [(3, 2), (4, 3), (6, 4), (7, 5)]);
    // 113 options and 14 discover picks sent; the hero pick is not counted.
    assert_eq!(s.actions.len(), 127);
    assert!(s.ended);
    let minutes = (s.end_ms.unwrap() - s.start_ms.unwrap()) as f64 / 60_000.0;
    assert!((14.0..14.2).contains(&minutes), "{minutes}");
    // Every offer and every bought or sold minion has its card id.
    for t in &s.turns {
        assert!(t.offers.iter().all(|o| o.card_id.is_some()));
        assert!(t.buys.iter().chain(&t.sells).all(Option::is_some));
    }
}

#[test]
fn duos_shop_build_253216() {
    let s = shop("b253216_duos");
    assert_eq!(s.turns.len(), 8);
    assert_eq!((sum(&s, |t| t.rolls), sum(&s, |t| t.free_rolls)), (8, 4));
    assert_eq!(sum(&s, |t| t.buys.len() as u32), 9);
    assert_eq!(sum(&s, |t| t.sells.len() as u32), 14);
    let tiers: Vec<_> = s.tier_ups.iter().map(|u| (u.turn, u.tier)).collect();
    assert_eq!(tiers, [(3, 2), (4, 3), (6, 4), (7, 5)]);
    // 97 options and 8 picks: only the local player's, never the teammate's.
    assert_eq!(s.actions.len(), 105);
    assert!(s.ended);
}
