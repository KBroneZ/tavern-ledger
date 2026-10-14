//! Tests of the error log (T-580): its lines, rotation, the panic hook and
//! the marker of an unclean exit.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::fake_secrets as fake;
use tracker::store::Store;
use tracker::Watcher;

use super::*;

static NEXT: AtomicUsize = AtomicUsize::new(0);
/// The panic hook is the process's: the tests that set it take turns.
static HOOK: Mutex<()> = Mutex::new(());

/// A folder of its own, removed when the test is done.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("tl-applog-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn log(&self) -> Log {
        Log::new(&self.0)
    }

    fn small(&self, max_bytes: u64, files: usize) -> Log {
        Log::with_limits(self.0.join(DIR), max_bytes, files)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn app_log(scratch: &Scratch) -> String {
    fs::read_to_string(scratch.0.join(DIR).join(FILE)).unwrap()
}

// --- lines ---

#[test]
fn a_line_has_time_level_area_and_message() {
    let line = format_line(
        1_790_000_000,
        Level::Warn,
        "Upload",
        "the server had an error (503)",
    );
    assert_eq!(
        line,
        "2026-09-21T14:13:20Z WARN upload the server had an error (503)"
    );
}

#[test]
fn the_time_is_utc_and_zero_padded() {
    assert_eq!(timestamp(0), "1970-01-01T00:00:00Z");
    assert_eq!(timestamp(86_400 + 3_661), "1970-01-02T01:01:01Z");
}

#[test]
fn a_message_is_one_redacted_line_of_limited_length() {
    let text = format!("tag Jaina#1234\nnext {}", "word ".repeat(300));
    let line = format_line(0, Level::Error, "ui", &text);
    assert!(!line.contains('\n'));
    assert!(!line.contains("Jaina"), "{line}");
    assert!(line.chars().count() < 600, "{}", line.chars().count());
}

#[test]
fn the_area_is_a_short_plain_word() {
    assert_eq!(area_word("Tracker"), "tracker");
    assert_eq!(area_word("a b\nc#1"), "abc1");
    assert_eq!(area_word(&"x".repeat(40)).len(), 16);
}

#[test]
fn lines_are_appended_in_order_and_the_folder_is_made() {
    let scratch = Scratch::new();
    let log = scratch.log();
    log.record(Level::Info, "app", "first");
    log.record(Level::Error, "app", "second");
    let text = app_log(&scratch);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with(" INFO app first"), "{}", lines[0]);
    assert!(lines[1].ends_with(" ERROR app second"), "{}", lines[1]);
}

#[test]
fn what_is_written_never_holds_names_or_secrets() {
    let scratch = Scratch::new();
    let log = scratch.log();
    let word = fake::word();
    log.record(
        Level::Error,
        "tracker",
        &format!("could not read for Jaina#1234 (GameAccountId=[hi=1441151938 lo=98765432]) token={word}"),
    );
    let text = app_log(&scratch);
    for secret in ["Jaina", "1234", "1441151938", "98765432", word.as_str()] {
        assert!(!text.contains(secret), "{secret} in {text}");
    }
}

// --- rotation ---

#[test]
fn a_full_file_becomes_the_next_one_and_the_oldest_goes() {
    let scratch = Scratch::new();
    let log = scratch.small(200, 3);
    for n in 0..30 {
        log.record(Level::Info, "app", &format!("line number {n:02}"));
    }
    let dir = scratch.0.join(DIR);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["app.log", "app.log.1", "app.log.2"]);
    for name in &names {
        let size = fs::metadata(dir.join(name)).unwrap().len();
        assert!(size <= 200, "{name}: {size}");
    }
    let all = log.tail(1000);
    assert!(all.last().unwrap().ends_with("line number 29"));
    assert!(
        !all.iter().any(|l| l.ends_with("line number 00")),
        "the oldest went"
    );
}

#[test]
fn the_tail_reaches_into_older_files_in_order() {
    let scratch = Scratch::new();
    let log = scratch.small(150, 3);
    for n in 0..9 {
        log.record(Level::Info, "app", &format!("n{n}"));
    }
    let last_five = log.tail(5);
    assert_eq!(last_five.len(), 5);
    let ends: Vec<&str> = last_five
        .iter()
        .map(|l| l.rsplit(' ').next().unwrap())
        .collect();
    assert_eq!(ends, ["n4", "n5", "n6", "n7", "n8"]);
}

#[test]
fn a_log_that_does_not_exist_has_an_empty_tail() {
    let scratch = Scratch::new();
    assert!(scratch.log().tail(10).is_empty());
}

// --- panics ---

#[test]
fn a_panic_in_any_thread_leaves_a_line_and_runs_the_previous_hook() {
    let _turn = HOOK.lock().unwrap_or_else(|e| e.into_inner());
    let scratch = Scratch::new();
    let log = Arc::new(scratch.log());
    let seen = Arc::new(AtomicUsize::new(0));
    let counted = seen.clone();
    let previous: PanicHook = Box::new(move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
    });
    let before = std::panic::take_hook();
    std::panic::set_hook(panic_hook(log, previous));
    let worker = std::thread::Builder::new()
        .name("tracker-test".into())
        .spawn(|| panic!("boom for Jaina#1234"))
        .unwrap();
    let outcome = worker.join();
    std::panic::set_hook(before);
    assert!(outcome.is_err(), "the panic still unwinds as before");
    // Another test's panic can land in this hook while it is set.
    assert!(seen.load(Ordering::SeqCst) >= 1);
    let text = app_log(&scratch);
    assert!(text.contains(" ERROR panic "), "{text}");
    assert!(
        text.contains("thread 'tracker-test' panicked at "),
        "{text}"
    );
    assert!(text.contains("applog_tests.rs"), "{text}");
    assert!(text.contains("boom for"), "{text}");
    assert!(!text.contains("Jaina"), "{text}");
}

// --- unclean exit ---

#[test]
fn a_marker_left_behind_says_the_last_run_ended_unexpectedly() {
    let scratch = Scratch::new();
    let log = scratch.log();
    assert!(!log.mark_running(), "a first start finds nothing");
    assert!(log.mark_running(), "the marker of a run that never stopped");
    log.mark_stopped();
    assert!(!log.mark_running(), "a clean quit removed it");
    log.mark_stopped();
    log.mark_stopped();
}

#[test]
fn stopping_without_a_marker_is_fine() {
    let scratch = Scratch::new();
    let log = scratch.log();
    log.mark_stopped();
    assert!(app_log_missing(&scratch));
}

fn app_log_missing(scratch: &Scratch) -> bool {
    !scratch.0.join(DIR).join(FILE).exists()
}

// --- commands ---

#[test]
fn a_failed_command_passes_its_result_on_unchanged() {
    let failed: Result<u8, String> = Err("no".into());
    assert_eq!(failed.logged("some_command"), Err("no".to_string()));
    let fine: Result<u8, String> = Ok(3);
    assert_eq!(fine.logged("some_command"), Ok(3));
}

// --- a full run on the fixtures leaves nothing private ---

fn fixture(name: &str) -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../bg-parser/tests/data/{name}.log"));
    fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n")
}

/// Every `Name#1234` a fixture holds.
fn battletags(text: &str) -> Vec<String> {
    let shape = regex::Regex::new(r"[\p{L}\p{N}_]+#\d{3,}").unwrap();
    let mut tags: Vec<String> = shape
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect();
    tags.sort();
    tags.dedup();
    tags
}

/// What the scan looks for in a log: a BattleTag shape, an account id, an
/// email, a token, a long opaque key, or a user name in a path.
fn private_things(log: &str) -> Vec<String> {
    let shapes = [
        r"#\d{3,}",
        // The label may stay when its value is cut.
        r"(?i)gameaccountid(?:[^=]|=[^\[])|\bhi=\d|\blo=\d",
        r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
        r"eyJ[A-Za-z0-9_-]{5,}",
        r"(?i)bearer\s+[A-Za-z0-9]",
        r"[A-Za-z0-9+=]{32,}",
        r"(?i)users[\/]+(jane|bob)",
    ];
    shapes
        .iter()
        .flat_map(|s| {
            regex::Regex::new(s)
                .unwrap()
                .find_iter(log)
                .map(|m| m.as_str().to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn the_parsers_error_names_reach_the_log_and_its_free_text_does_not() {
    let problems = vec![
        "parser error: NotImplementedError".to_string(),
        "parser error: with a name Jane#1234".to_string(),
        "Player Jane#1234 left".to_string(),
        "parser error: Jane Doe".to_string(),
        "parser error: no game data".to_string(),
    ];
    assert_eq!(
        parser_errors(&problems),
        ["NotImplementedError", "no game data"]
    );
    assert!(parser_errors(&[]).is_empty());
}

#[test]
fn a_full_fixture_run_leaves_no_names_or_secrets_in_the_log() {
    let scratch = Scratch::new();
    let log = scratch.log();
    let mut tags = Vec::new();
    let mut games = 0;
    for (n, name) in ["solo_game", "duo_game", "broken_then_ok", "missing_entity"]
        .iter()
        .enumerate()
    {
        let text = fixture(name);
        tags.extend(battletags(&text));
        let logs = scratch.0.join(format!("Logs{n}"));
        let session = logs.join("Hearthstone_2026_10_09_10_00_00");
        fs::create_dir_all(&session).unwrap();
        fs::write(session.join("Power.log"), &text).unwrap();
        let store = Store::open(&scratch.0.join(format!("data{n}"))).unwrap();
        let mut watcher = Watcher::new(&logs, store);
        for saved in watcher.poll().unwrap() {
            games += 1;
            let (level, line) = saved_game_line(&saved);
            log.record(level, "game", &line);
            // Worse than the app ever does: the parser's own free text, which
            // can hold names, goes through the same door.
            for text in saved.report.warnings.iter().chain(&saved.report.problems) {
                log.record(Level::Warn, "game", text);
            }
        }
    }
    assert!(games >= 3, "the fixtures were read ({games} games)");
    assert!(!tags.is_empty(), "the fixtures hold names to look for");
    // Every place a name or a secret could come from, in one line each.
    for tag in &tags {
        log.record(
            Level::Error,
            "tracker",
            &format!("could not read the entry of {tag}"),
        );
        log.record(
            Level::Error,
            "ui",
            &format!("[main] error {tag} is not defined (app.js:1:1)"),
        );
    }
    let jwt = fake::jwt();
    let long_word = fake::long_word();
    let hostile = [
        format!("sign-in failed for jane.doe@example.com: Bearer {jwt}"),
        format!("upload stopped: server said {{\"access_token\":\"{jwt}\",\"refresh_token\":\"{long_word}\"}}"),
        "GameAccountId=[hi=144115193835963207 lo=987654321] seen".to_string(),
        format!("the key {} was refused", fake::hex()),
        r"cannot open C:\Users\jane\AppData\Roaming\TavernLedger\games.jsonl".to_string(),
        r"cannot read D:\Users\Bob Smith\Desktop\Power.log".to_string(),
    ];
    for message in &hostile {
        log.record(Level::Error, "upload", message);
    }
    let text = log.tail(10_000).join("\n");
    assert!(text.lines().count() > 10, "the run wrote its lines");
    assert_eq!(private_things(&text), Vec::<String>::new(), "{text}");
    for tag in &tags {
        let name = tag.split('#').next().unwrap();
        assert!(!text.contains(name), "{name} is in the log");
    }
}

// --- floods, a stuck rotation and a formatted panic ---

#[test]
fn a_line_repeated_many_times_is_written_once_and_then_counted() {
    let scratch = Scratch::new();
    let log = scratch.log();
    for n in 0..50 {
        log.record_at(
            1_790_000_000 + n,
            Level::Error,
            "command",
            "game_recap failed: no",
        );
    }
    log.record_at(1_790_000_060, Level::Info, "app", "next");
    let text = app_log(&scratch);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(
        lines[1].ends_with("the line above was repeated 49 more times"),
        "{text}"
    );
}

#[test]
fn more_lines_than_the_limit_in_a_minute_are_counted_not_written() {
    let scratch = Scratch::new();
    let log = scratch.log();
    let start = 1_790_000_040;
    for n in 0..200 {
        log.record_at(start, Level::Warn, "app", &format!("different {n}"));
    }
    log.record_at(start + 60, Level::Warn, "app", "a minute later");
    let text = app_log(&scratch);
    assert_eq!(text.lines().count(), 120 + 2, "{text}");
    assert!(
        text.contains("80 lines were not logged (too many)"),
        "{text}"
    );
}

#[test]
fn a_rotation_that_cannot_be_done_does_not_stop_the_log() {
    let scratch = Scratch::new();
    let log = scratch.small(150, 3);
    // A folder where the oldest file goes: removing it as a file fails.
    fs::create_dir_all(scratch.0.join(DIR).join("app.log.2")).unwrap();
    for n in 0..20 {
        log.record(Level::Info, "app", &format!("line {n:02}"));
    }
    let text = app_log(&scratch);
    assert!(text.lines().last().unwrap().ends_with("line 19"), "{text}");
    assert!(fs::metadata(scratch.0.join(DIR).join(FILE)).unwrap().len() <= 150);
}

#[test]
fn a_formatted_panic_message_is_withheld_and_a_literal_one_is_kept() {
    let _turn = HOOK.lock().unwrap_or_else(|e| e.into_inner());
    let scratch = Scratch::new();
    let log = Arc::new(scratch.log());
    let previous: PanicHook = Box::new(|_| {});
    let before = std::panic::take_hook();
    std::panic::set_hook(panic_hook(log, previous));
    let name = String::from("Bob");
    let literal = std::thread::spawn(|| panic!("tracker invariant broken"));
    let formatted = std::thread::spawn(move || panic!("cannot read {name}"));
    let _ = (literal.join(), formatted.join());
    std::panic::set_hook(before);
    let text = app_log(&scratch);
    assert!(text.contains("tracker invariant broken"), "{text}");
    assert!(text.contains("formatted message withheld"), "{text}");
    assert!(!text.contains("Bob"), "{text}");
}

#[test]
fn a_write_from_inside_a_write_is_skipped_not_stuck() {
    let scratch = Scratch::new();
    let log = scratch.log();
    WRITING.set(true);
    log.record(Level::Error, "panic", "from inside");
    WRITING.set(false);
    assert!(app_log_missing(&scratch));
    log.record(Level::Error, "app", "after");
    assert!(app_log(&scratch).contains("after"));
}

#[test]
fn the_report_lines_are_redacted_again_when_they_are_read() {
    let scratch = Scratch::new();
    let dir = scratch.0.join(DIR);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(FILE),
        "2026-10-13T09:00:00Z WARN app written by hand for Jaina#1234\n",
    )
    .unwrap();
    let lines = scratch.log().tail(10);
    assert_eq!(lines.len(), 1);
    assert!(!lines[0].contains("Jaina"), "{lines:?}");
}
