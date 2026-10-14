//! A stray, cut or blank line in `Power.log` must not throw away a whole
//! game; a log that is really unreadable (no times at all, or many bad
//! lines) must fail with a reason that says what is wrong.

use bg_parser::report::{GameReport, Status};
use bg_parser::{
    is_read_failure, parse_reader, LogReader, MAX_SKIPPED_LINES, PROBLEM_NO_TIMES,
    PROBLEM_TOO_MANY_BAD_LINES,
};

const SOLO: &str = include_str!("data/solo_game.log");
const TIMED: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

fn lines() -> Vec<String> {
    SOLO.lines().map(String::from).collect()
}

fn read(lines: &[String]) -> Vec<GameReport> {
    let mut text = lines.join("\n");
    text.push('\n');
    parse_reader(text.as_bytes()).expect("in-memory read")
}

fn only(lines: &[String]) -> GameReport {
    let mut games = read(lines);
    assert_eq!(games.len(), 1);
    games.remove(0)
}

fn with_inserted(at: usize, extra: &[&str]) -> Vec<String> {
    let mut l = lines();
    for (i, e) in extra.iter().enumerate() {
        l.insert(at + i, (*e).to_string());
    }
    l
}

#[test]
fn the_fixture_is_clean() {
    let report = only(&lines());
    assert_eq!(report.status, Status::Ok);
    assert!(report.warnings.is_empty());
}

#[test]
fn a_line_without_a_time_is_skipped_and_noted() {
    let report = only(&with_inserted(20, &["a stray line with no time"]));
    assert_eq!(report.status, Status::Ok);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert_eq!(report.warnings, ["skipped 1 unreadable log line"]);
}

#[test]
fn a_line_cut_inside_its_time_is_skipped_and_noted() {
    let report = only(&with_inserted(20, &["D 12:00:0"]));
    assert_eq!(report.status, Status::Ok);
    assert_eq!(report.warnings, ["skipped 1 unreadable log line"]);
}

#[test]
fn a_packet_cut_in_the_middle_is_skipped_and_noted() {
    let cut = format!("{TIMED}    TAG_CHANGE Entity=GameEntity tag=TU");
    let mut l = lines();
    l.push(cut);
    let report = only(&l);
    assert_eq!(report.status, Status::Ok);
    assert_eq!(report.warnings, ["skipped 1 unreadable log line"]);
}

#[test]
fn blank_lines_are_ignored_without_a_warning() {
    let report = only(&with_inserted(20, &["", "   ", "\t"]));
    assert_eq!(report.status, Status::Ok);
    assert!(report.warnings.is_empty());
}

#[test]
fn the_most_bad_lines_a_game_may_have_are_skipped() {
    let extra: Vec<&str> = vec!["stray"; MAX_SKIPPED_LINES];
    let report = only(&with_inserted(20, &extra));
    assert_eq!(report.status, Status::Ok);
    assert_eq!(
        report.warnings,
        [format!("skipped {MAX_SKIPPED_LINES} unreadable log lines")]
    );
}

#[test]
fn one_more_bad_line_makes_the_game_unreadable_with_a_reason() {
    let extra: Vec<&str> = vec!["stray"; MAX_SKIPPED_LINES + 1];
    let report = only(&with_inserted(20, &extra));
    assert_eq!(report.status, Status::Unsupported);
    assert_eq!(report.problems, [PROBLEM_TOO_MANY_BAD_LINES]);
}

fn without_times(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            l.strip_prefix("D 12:00:00.0000000 ")
                .unwrap_or(l)
                .to_string()
        })
        .collect()
}

#[test]
fn a_log_with_no_times_says_so() {
    let report = only(&without_times(&lines()));
    assert_eq!(report.status, Status::Unsupported);
    assert_eq!(report.problems, [PROBLEM_NO_TIMES]);
}

#[test]
fn a_short_log_with_no_times_says_so_too() {
    let report = only(&[
        "GameState.DebugPrintPower() - CREATE_GAME".to_string(),
        "GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".to_string(),
    ]);
    assert_eq!(report.problems, [PROBLEM_NO_TIMES]);
}

#[test]
fn a_broken_game_does_not_hide_the_next_one() {
    let mut l = without_times(&lines());
    l.extend(lines());
    let games = read(&l);
    assert_eq!(games.len(), 2);
    assert_eq!(games[0].status, Status::Unsupported);
    assert_eq!(games[1].status, Status::Ok);
}

#[test]
fn an_unknown_packet_still_makes_the_game_unreadable() {
    let l = with_inserted(20, &[&format!("{TIMED}    SOMETHING_NEW foo")]);
    let report = only(&l);
    assert_eq!(report.status, Status::Unsupported);
    assert_eq!(report.problems, ["parser error: NotImplementedError"]);
}

#[test]
fn live_views_stay_known_after_one_stray_line() {
    let mut reader = LogReader::default();
    for l in lines() {
        reader.feed(&l);
    }
    reader.feed("stray");
    let snapshot = reader.snapshot_current().expect("a game is on");
    assert_eq!(snapshot.status, Status::Ok);
}

#[test]
fn the_reasons_in_the_lines_count_as_read_failures() {
    assert!(is_read_failure(PROBLEM_NO_TIMES));
    assert!(is_read_failure(PROBLEM_TOO_MANY_BAD_LINES));
    assert!(is_read_failure("parser error: NotImplementedError"));
    assert!(!is_read_failure("unexpected lobby size: 0"));
}

#[test]
fn the_reasons_are_short_notes_the_server_accepts() {
    // supabase/functions/upload-game/validate.ts NOTE
    for text in [PROBLEM_NO_TIMES, PROBLEM_TOO_MANY_BAD_LINES] {
        assert!(text.len() <= 160, "{text}");
        assert!(
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || " _:.()-".contains(c)),
            "{text}"
        );
    }
}

#[test]
fn a_garbled_entity_header_followed_by_its_tags_is_not_read_as_wrong_data() {
    // Its tag lines would otherwise land on the entity before it.
    let mut l = lines();
    let at = l
        .iter()
        .position(|x| x.contains("FULL_ENTITY - Creating ID=11 "))
        .expect("an entity in the fixture");
    l[at] = format!("{TIMED}    FULL_ENTITY - Creating ID=11 CardID");
    let report = only(&l);
    assert_eq!(report.status, Status::Unsupported);
    assert_eq!(report.problems, ["parser error: RegexParsingError"]);
}

#[test]
fn a_malformed_packet_is_only_skipped_when_nothing_valid_follows_it() {
    let mut l = lines();
    l.push(format!("{TIMED}    TAG_CHANGE Entity=GameEntity tag=TU"));
    let ok = only(&l);
    assert_eq!(ok.status, Status::Ok);
    l.push(format!(
        "{TIMED}    TAG_CHANGE Entity=GameEntity tag=STATE value=COMPLETE "
    ));
    assert_eq!(only(&l).status, Status::Unsupported);
}

#[test]
fn untimed_game_lines_still_say_the_type_and_the_build() {
    let l = lines();
    let at = l
        .iter()
        .position(|x| x.contains("GameType=GT_BATTLEGROUNDS"))
        .expect("the game type line");
    let mut l = l;
    l[at] = "GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".into();
    let report = only(&l);
    assert_eq!(report.game_type.as_deref(), Some("GT_BATTLEGROUNDS"));
    assert_eq!(report.status, Status::Ok);
}

#[test]
fn a_game_that_is_not_battlegrounds_stays_so_with_many_stray_lines() {
    let mut l = vec![
        format!("{TIMED}CREATE_GAME"),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_RANKED".to_string(),
    ];
    l.extend(vec!["stray".to_string(); MAX_SKIPPED_LINES + 5]);
    assert_eq!(only(&l).status, Status::NotBattlegrounds);
}

#[test]
fn a_fatal_error_is_the_one_a_game_reports_not_the_stray_lines() {
    let mut l = with_inserted(20, &["stray", "stray"]);
    l.insert(30, format!("{TIMED}    SOMETHING_NEW foo"));
    let report = only(&l);
    assert_eq!(report.problems, ["parser error: NotImplementedError"]);
}

#[test]
fn windows_line_ends_and_blank_lines_are_fine() {
    let mut text = lines().join("\r\n");
    text.push_str("\r\n\r\n");
    let games = parse_reader(text.as_bytes()).expect("in-memory read");
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].status, Status::Ok);
    assert!(games[0].warnings.is_empty());
}

#[test]
fn many_untimed_lines_at_the_start_do_not_hide_that_the_game_has_times() {
    let extra = vec!["stray"; MAX_SKIPPED_LINES + 1];
    let report = only(&with_inserted(1, &extra));
    assert_eq!(report.problems, [PROBLEM_TOO_MANY_BAD_LINES]);
}
