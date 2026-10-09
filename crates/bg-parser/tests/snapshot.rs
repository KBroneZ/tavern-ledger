//! A look at the game in progress (the overlay's live state). It must not
//! change what the game's final report says.

use std::fs;
use std::path::Path;

use bg_parser::report::Status;
use bg_parser::LogReader;

fn lines(name: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(format!("{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .lines()
        .map(String::from)
        .collect()
}

fn feed(reader: &mut LogReader, lines: &[String]) {
    lines.iter().for_each(|l| reader.feed(l));
}

#[test]
fn there_is_no_snapshot_before_a_game_starts() {
    assert!(LogReader::default().snapshot_current().is_none());
}

#[test]
fn a_game_in_progress_gives_an_incomplete_report_with_what_is_known() {
    let all = lines("duo_game");
    let complete_at = all
        .iter()
        .position(|l| l.contains("STATE value=COMPLETE"))
        .expect("fixture completes");
    let mut reader = LogReader::default();
    feed(&mut reader, &all[..complete_at]);
    let report = reader.snapshot_current().expect("a game is on");
    assert_eq!(report.status, Status::Incomplete);
    assert_eq!(report.index, 1);
    assert!(report.hero.is_some());
    assert!(!report.rounds.is_empty());
    assert_eq!(report.final_place, None, "no place before the game ends");
}

#[test]
fn looking_does_not_change_the_final_report() {
    for name in ["solo_game", "duo_game", "solo_local_eliminated"] {
        let all = lines(name);
        let mut plain = LogReader::default();
        feed(&mut plain, &all);
        let mut watched = LogReader::default();
        for line in &all {
            watched.feed(line);
            let _ = watched.snapshot_current();
        }
        assert_eq!(watched.finish(), plain.finish(), "{name}");
    }
}

#[test]
fn the_snapshot_has_the_number_the_saved_report_will_have() {
    let all = lines("two_games_new_player_id");
    let mut reader = LogReader::default();
    let second_start = all
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("CREATE_GAME"))
        .nth(1)
        .map(|(i, _)| i)
        .expect("two games");
    feed(&mut reader, &all[..second_start + 3]);
    assert_eq!(reader.snapshot_current().map(|r| r.index), Some(2));
}
