//! Client build 255130 (installed 2026-10-15) against a real Solo game played
//! on it (`tests/data/real/b254477_solo.log`, made by `tools/make_fixture.py`).
//!
//! HearthstoneJSON and Battle.net number the client; a game writes its own
//! number to `Power.log`. The 255130 client still writes `BuildNumber=254477`
//! (and `Hearthstone/37.0.254477` in `Net.log`), its card data is the same as
//! 254477's, and its games read like 254477's, so the parser's tested build
//! is 254477 and 255130 is only a client build that maps to it.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use bg_parser::report::{GameReport, Status};
use bg_parser::{CLIENT_BUILDS, TESTED_BUILDS};

fn solo_game() -> GameReport {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real/b254477_solo.log");
    let text = fs::read_to_string(path).expect("fixture");
    let mut reports = bg_parser::parse_reader(text.as_bytes()).expect("in-memory read");
    assert_eq!(reports.len(), 1, "one game in the fixture");
    reports.remove(0)
}

#[test]
fn a_client_build_maps_to_an_older_tested_game_build() {
    for &(client, game) in CLIENT_BUILDS {
        assert!(TESTED_BUILDS.contains(&game), "{client}: game build {game}");
        assert!(client > game, "{client} is not newer than {game}");
    }
}

#[test]
fn a_client_build_is_listed_once_and_is_not_a_game_build() {
    let clients: BTreeSet<i64> = CLIENT_BUILDS.iter().map(|&(client, _)| client).collect();
    assert_eq!(clients.len(), CLIENT_BUILDS.len(), "a client build twice");
    for client in clients {
        assert!(
            !TESTED_BUILDS.contains(&client),
            "{client} is a tested game build and a client build"
        );
    }
}

#[test]
fn client_build_255130_writes_game_build_254477() {
    assert!(CLIENT_BUILDS.contains(&(255_130, 254_477)));
}

#[test]
fn a_solo_game_on_the_255130_client_reads_clean() {
    let report = solo_game();
    assert_eq!(report.build, Some(254_477));
    assert_eq!(report.game_type.as_deref(), Some("GT_BATTLEGROUNDS"));
    assert_eq!(report.status, Status::Ok);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert!(report.final_place.is_some());
    assert!(!report.rounds.is_empty());
}
