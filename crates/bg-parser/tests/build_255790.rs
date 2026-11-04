//! Build 255790 against a real Duos game played on it
//! (`tests/data/real/b255790_duos.log`, made by `tools/make_fixture.py`).
//!
//! Unlike client 255130, this client writes its own number to `Power.log`, so
//! it is a tested game build and not a `CLIENT_BUILDS` pair. HearthstoneJSON's
//! card data of 255790 is byte-identical to 254477's.

use std::fs;
use std::path::Path;

use bg_parser::report::{GameReport, Status};
use bg_parser::{CLIENT_BUILDS, TESTED_BUILDS};

fn game() -> GameReport {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real/b255790_duos.log");
    let text = fs::read_to_string(path).expect("fixture");
    let mut reports = bg_parser::parse_reader(text.as_bytes()).expect("in-memory read");
    assert_eq!(reports.len(), 1, "one game in the fixture");
    reports.remove(0)
}

#[test]
fn build_255790_is_tested_and_reads_clean() {
    assert!(TESTED_BUILDS.contains(&255_790));
    let report = game();
    assert_eq!(report.build, Some(255_790));
    assert_eq!(report.game_type.as_deref(), Some("GT_BATTLEGROUNDS_DUO"));
    assert_eq!(report.status, Status::Ok);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert_eq!(report.final_place, Some(3));
    assert_eq!(report.rounds.len(), 10);
}

#[test]
fn build_255790_is_a_game_build_and_not_a_client_pair() {
    assert!(CLIENT_BUILDS.iter().all(|&(client, _)| client != 255_790));
}
