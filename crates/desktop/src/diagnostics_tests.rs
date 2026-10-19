//! The diagnostics text (T-626): synthetic sensitive values go through every
//! input and none of them may come out. Names, tags, emails and tokens here are
//! made up.

use super::*;

/// Fake secrets, written in two parts so no scanner takes the source for a real
/// key; a `{TOKEN}` or `{KEY}` in a test line stands for them (`lines`, `fill`).
const TOKEN: &str = concat!("ghp_", "abcdef0123456789");
const KEY: &str = concat!("sk-", "live-secret");

fn fill(text: &str) -> String {
    text.replace("{TOKEN}", TOKEN).replace("{KEY}", KEY)
}

/// Things that must never appear in a diagnostics text.
const SECRETS: [&str; 12] = [
    "Andrew",
    "andrew",
    "Andrew#1234",
    "1234",
    "andrew@example.com",
    "example.com",
    TOKEN,
    KEY,
    "C:\\Users",
    "AppData",
    "987654321012",
    "Zorbo",
];

fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| fill(s)).collect()
}

fn input<'a>(config: Option<&'a str>, log: &'a [String]) -> Input<'a> {
    Input {
        app_version: "0.2.5",
        game_build: Some(254_477),
        parser_revision: 16,
        log_config: config,
        app_log: log,
    }
}

fn assert_clean(text: &str) {
    for secret in SECRETS {
        assert!(!text.contains(secret), "{secret:?} in:\n{text}");
    }
}

const GOOD_CONFIG: &str = "[Power]\nLogLevel=1\nFilePrinting=true\nConsolePrinting=false\nScreenPrinting=false\nVerbose=true\n";

#[test]
fn a_normal_machine_gives_the_fixed_fields_and_the_power_lines() {
    let text = build(&input(Some(GOOD_CONFIG), &[])).unwrap();
    assert_eq!(
        text,
        "Tavern Ledger diagnostics (format 1)\nApp version: 0.2.5\nGame build: 254477\nParser revision: 16\n\
log.config [Power]:\n  LogLevel=1\n  FilePrinting=true\n  ConsolePrinting=false\n  ScreenPrinting=false\n  Verbose=true\n\
App log, warnings and errors:\n  none among the known messages\n"
    );
}

#[test]
fn the_app_version_is_a_version_or_unknown() {
    for (given, shown) in [
        ("0.2.5", "0.2.5"),
        ("1.12.0-beta.1", "1.12.0-beta.1"),
        ("0.2.5 Andrew", "unknown"),
        ("Andrew#1234", "unknown"),
        ("0.2.5\nGame build: 1", "unknown"),
        ("", "unknown"),
        ("C:\\Users\\andrew", "unknown"),
    ] {
        let mut data = input(None, &[]);
        data.app_version = given;
        let text = build(&data).unwrap();
        assert!(
            text.contains(&format!("App version: {shown}\n")),
            "{given:?}"
        );
        assert_clean(&text);
    }
}

#[test]
fn the_game_build_is_a_plausible_number_or_unknown() {
    for (given, shown) in [
        (Some(254_477), "254477"),
        (None, "unknown"),
        (Some(0), "unknown"),
        (Some(-5), "unknown"),
        (Some(987_654_321_012), "unknown"),
    ] {
        let mut data = input(None, &[]);
        data.game_build = given;
        let text = build(&data).unwrap();
        assert!(
            text.contains(&format!("Game build: {shown}\n")),
            "{given:?}"
        );
        assert_clean(&text);
    }
}

#[test]
fn a_log_config_line_that_is_not_a_known_setting_is_only_counted() {
    let config = fill("[Power]\nLogLevel=1\n; Andrew's notes\nContact=andrew@example.com\nPath=C:\\Users\\andrew\\AppData\\x\nToken={TOKEN}\nName=Andrew#1234\nVerbose=true\n");
    let text = build(&input(Some(&config), &[])).unwrap();
    assert_clean(&text);
    assert!(text.contains("  LogLevel=1\n  Verbose=true\n"));
    assert!(text.contains("(5 other lines not shown)"), "{text}");
}

#[test]
fn a_known_setting_with_a_free_text_value_is_not_shown() {
    for value in [
        "Andrew",
        "1234",
        "C:\\Users\\andrew",
        "true # Andrew",
        "",
        "-1",
        "1e3",
    ] {
        let config = format!("[Power]\nLogLevel={value}\n");
        let text = build(&input(Some(&config), &[])).unwrap();
        assert!(!text.contains("LogLevel="), "{value:?}: {text}");
        assert_clean(&text);
    }
}

#[test]
fn setting_names_and_values_are_rewritten_from_their_parts() {
    let config = "[power]\n  loglevel = 1  \nFILEPRINTING= TRUE\nverbose=False\n";
    let text = build(&input(Some(config), &[])).unwrap();
    assert!(
        text.contains("  LogLevel=1\n  FilePrinting=true\n  Verbose=false\n"),
        "{text}"
    );
}

#[test]
fn only_the_power_section_is_read() {
    let config =
        "[LoadingScreen]\nLogLevel=1\n[Power]\nVerbose=true\n[Zone]\nLogLevel=2\nAndrew=1\n";
    let text = build(&input(Some(config), &[])).unwrap();
    assert!(text.contains("  Verbose=true\n"));
    assert!(!text.contains("LogLevel"));
    assert!(!text.contains("other lines"));
}

#[test]
fn no_file_and_no_power_section_say_so() {
    let missing = build(&input(None, &[])).unwrap();
    assert!(missing.contains("  log.config not found\n"));
    let none = build(&input(Some("[Zone]\nLogLevel=1\n"), &[])).unwrap();
    assert!(none.contains("  no [Power] section\n"));
    let empty = build(&input(Some("[Power]\n"), &[])).unwrap();
    assert!(empty.contains("  no known setting\n"));
}

#[test]
fn a_known_log_line_shows_its_fixed_words_and_never_its_detail() {
    let log = lines(&[
        "2026-10-19T21:14:22Z WARN app the previous run ended unexpectedly",
        "2026-10-19T21:14:23Z ERROR tracker cannot use the game history: C:\\Users\\andrew\\AppData\\Roaming\\TavernLedger\\games.jsonl Andrew#1234",
        "2026-10-19T21:14:24Z WARN io saved tribes: could not read Andrew's file andrew@example.com",
        "2026-10-19T21:14:25Z WARN updates update check failed: token {TOKEN} refused",
        "2026-10-19T21:14:26Z WARN upload Andrew#1234 was refused with {KEY}",
        "2026-10-19T21:14:27Z ERROR app failed to start: Zorbo is not a valid name",
    ]);
    let text = build(&input(Some(GOOD_CONFIG), &log)).unwrap();
    assert_clean(&text);
    for shown in [
        "2026-10-19T21:14:22Z WARN app the previous run ended unexpectedly",
        "2026-10-19T21:14:23Z ERROR tracker cannot use the game history (details not shown)",
        "2026-10-19T21:14:24Z WARN io saved tribes could not be used (details not shown)",
        "2026-10-19T21:14:25Z WARN updates the update check failed (details not shown)",
        "2026-10-19T21:14:26Z WARN upload an upload problem (details not shown)",
        "2026-10-19T21:14:27Z ERROR app the app failed to start",
    ] {
        assert!(text.contains(shown), "{shown}\n{text}");
    }
}

#[test]
fn a_line_that_is_not_a_known_message_is_only_counted() {
    let log = lines(&[
        "2026-10-19T21:14:22Z ERROR ui [main] error Andrew#1234 is not defined (app.js:10:5)",
        "2026-10-19T21:14:23Z WARN app something nobody wrote a pattern for: Zorbo",
        "2026-10-19T21:14:24Z INFO app Tavern Ledger 0.2.5 started; data folder C:\\Users\\andrew",
        "not a log line at all, Andrew#1234",
        "2026-10-19 21:14:25 ERROR app wrong time shape Zorbo",
        "2026-10-19T21:14:26Z ERROR App wrong area shape Zorbo",
    ]);
    let text = build(&input(Some(GOOD_CONFIG), &log)).unwrap();
    assert_clean(&text);
    assert!(text.contains("  none among the known messages\n"));
    assert!(
        text.contains("(2 more not shown: older, or not a known message)"),
        "{text}"
    );
}

#[test]
fn a_line_must_come_from_the_area_its_message_belongs_to() {
    // The same words under another area are not the known message.
    let log = lines(&[
        "2026-10-19T21:14:22Z ERROR upload2 the tracker stopped after an internal error Zorbo",
    ]);
    let text = build(&input(None, &log)).unwrap();
    assert_clean(&text);
    assert!(text.contains("none among the known messages"));
}

#[test]
fn a_panic_keeps_only_the_fact_never_a_file_a_place_or_a_message() {
    let log = lines(&[
        "2026-10-19T21:14:22Z ERROR panic thread 'main' panicked at C:\\Users\\andrew\\.cargo\\registry\\src\\x\\tauri-2.12.2\\src\\app.rs:1444:11: formatted message withheld",
        "2026-10-19T21:14:23Z ERROR panic thread 'tracker' panicked at crates/tracker/src/lib.rs:12:3: Zorbo exploded",
        "2026-10-19T21:14:24Z ERROR panic thread 'Andrew#1234' panicked at src/x.rs:1:1: no message",
        // Crafted: a file name that is a secret, and a later path inside the message.
        "2026-10-19T21:14:25Z ERROR panic thread 'main' panicked at src/{KEY}.rs:1:2: formatted message withheld",
        "2026-10-19T21:14:26Z ERROR panic thread 'main' panicked at src/Andrew.rs:1:2: read C:\\Users\\andrew\\Zorbo.rs:9:9: oops",
    ]);
    let text = build(&input(None, &log)).unwrap();
    assert_clean(&text);
    assert!(!text.contains(KEY) && !text.contains(".rs"), "{text}");
    assert!(
        text.contains(
            "2026-10-19T21:14:22Z ERROR panic a panic (place not shown, message withheld)"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "2026-10-19T21:14:23Z ERROR panic a panic (place not shown, message not shown)"
        ),
        "{text}"
    );
    assert_eq!(text.matches("a panic (place not shown").count(), 5);
}

#[test]
fn a_panic_must_come_from_the_panic_area() {
    // The app's panic hook writes under the area "panic"; the same text under another area is not one.
    let log =
        lines(&["2026-10-19T21:14:22Z ERROR app thread 'main' panicked at src/x.rs:1:1: Zorbo"]);
    let text = build(&input(None, &log)).unwrap();
    assert!(text.contains("none among the known messages"));
    assert!(!text.contains("a panic"));
}

#[test]
fn the_webview2_lines_keep_only_a_version() {
    let log = lines(&[
        "2026-10-19T21:48:18Z ERROR app The WebView2 runtime is registered as 155.0.4283.45 but its files are not there; the app cannot open a window.",
        "2026-10-19T21:48:19Z ERROR app The WebView2 runtime is registered as Andrew but its files are not there",
        "2026-10-19T21:48:20Z ERROR app The WebView2 runtime is not installed; the app cannot open a window.",
    ]);
    let text = build(&input(None, &log)).unwrap();
    assert_clean(&text);
    assert!(text.contains("registered as 155.0.4283.45 but its files are not there\n"));
    assert!(text.contains("the WebView2 runtime is not installed\n"));
    assert!(!text.contains("registered as Andrew"));
}

#[test]
fn a_number_in_a_message_is_short_or_the_line_is_not_shown() {
    let log = lines(&[
        "2026-10-19T21:14:22Z WARN io the game history has 3 lines that could not be read",
        "2026-10-19T21:14:23Z WARN io the game history has 987654321012 lines that could not be read",
        "2026-10-19T21:14:24Z WARN startup unknown start-up task state 7",
    ]);
    let text = build(&input(None, &log)).unwrap();
    assert_clean(&text);
    assert!(text.contains("the game history has 3 lines that could not be read\n"));
    assert!(text.contains("unknown start-up task state 7\n"));
    assert!(!text.contains("987654321012"));
}

#[test]
fn only_the_newest_known_lines_are_shown_and_the_rest_counted() {
    let log: Vec<String> = (0..30)
        .map(|i| format!("2026-10-19T21:{i:02}:00Z WARN app the previous run ended unexpectedly"))
        .collect();
    let text = build(&input(None, &log)).unwrap();
    assert_eq!(
        text.matches("the previous run ended unexpectedly").count(),
        MAX_LOG_LINES
    );
    assert!(text.contains("21:29:00Z"), "the newest is kept");
    assert!(!text.contains("21:09:00Z"), "the oldest are cut");
    assert!(text.contains("(10 more not shown"), "{text}");
}

#[test]
fn a_hostile_log_line_cannot_add_lines_to_the_text() {
    let log = lines(&["2026-10-19T21:14:22Z WARN app the previous run ended unexpectedly\nGame build: 1\nApp version: Andrew#1234"]);
    // A "line" holding a newline does not have the log's shape, so it is not
    // used at all (and the fixed words are all that is ever copied anyway).
    let text = build(&input(None, &log)).unwrap();
    assert_clean(&text);
    assert_eq!(text.matches("Game build:").count(), 1);
    assert_eq!(text.matches("App version:").count(), 1);
}

#[test]
fn the_last_check_refuses_a_text_that_somehow_holds_a_forbidden_shape() {
    // The schema cannot produce these; the check is a second line behind it. Feed it directly.
    for (text, what) in [
        ("Andrew#1234", "a BattleTag-shaped text"),
        ("andrew@example.com", "an email-shaped text"),
        ("C:\\Users\\andrew", "a drive path"),
        ("%USERPROFILE%\\x", "a profile folder"),
        ("987654321012", "a long number"),
    ] {
        let found = FORBIDDEN.iter().find(|(pattern, _)| pattern.is_match(text));
        assert_eq!(found.map(|(_, w)| *w), Some(what), "{text}");
    }
    // And the text a normal run makes never trips it.
    for (pattern, _) in FORBIDDEN.iter() {
        assert!(!pattern.is_match(&build(&input(Some(GOOD_CONFIG), &[])).unwrap()));
    }
}

#[test]
fn every_message_in_the_table_is_checked_against_its_own_words() {
    // A table row whose words would trip the last check would refuse every text.
    for (area, start, words) in KNOWN {
        let log = lines(&[&format!("2026-10-19T21:14:22Z WARN {area} {start} Zorbo")]);
        let text = build(&input(None, &log)).unwrap_or_else(|r| panic!("{area}/{start}: {r}"));
        assert!(text.contains(words), "{area}/{start}");
        assert_clean(&text);
    }
}

#[test]
fn the_config_is_read_up_to_a_cap_and_a_missing_file_is_none() {
    let dir = std::env::temp_dir().join(format!("tl-diagnostics-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let small = dir.join("log.config");
    std::fs::write(&small, GOOD_CONFIG).unwrap();
    assert_eq!(read_log_config(&small).as_deref(), Some(GOOD_CONFIG));
    let big = dir.join("big.config");
    std::fs::write(&big, "x".repeat(200_000)).unwrap();
    assert_eq!(read_log_config(&big).map(|t| t.len()), Some(64 * 1024));
    assert_eq!(read_log_config(&dir.join("nope.config")), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_repeated_setting_shows_only_its_first_value() {
    let config = "[Power]\nLogLevel=1\nLogLevel=2\nLogLevel=99\nVerbose=true\nVerbose=false\n";
    let text = build(&input(Some(config), &[])).unwrap();
    assert!(text.contains("  LogLevel=1\n  Verbose=true\n"), "{text}");
    assert!(!text.contains("LogLevel=2") && !text.contains("Verbose=false"));
}

#[test]
fn only_ascii_digits_count_in_a_log_line() {
    // Arabic-Indic digits in a hand-edited log are not a time or a number.
    let log = lines(&[
        "\u{0662}\u{0660}26-10-19T21:14:22Z WARN app the previous run ended unexpectedly",
        "2026-10-19T21:14:23Z WARN io the game history has \u{0663} lines that could not be read",
    ]);
    let text = build(&input(None, &log)).unwrap();
    assert!(
        !text.contains('\u{0662}') && !text.contains('\u{0663}'),
        "{text}"
    );
}

#[test]
fn a_number_is_taken_whole_never_a_prefix_of_a_longer_one() {
    let log = lines(&[
        "2026-10-19T21:14:22Z WARN startup unknown start-up task state 987654321012",
        "2026-10-19T21:14:23Z WARN startup unknown start-up task state 12345",
        "2026-10-19T21:14:24Z WARN startup unknown start-up task state 7x",
        "2026-10-19T21:14:25Z WARN io the game history has 1234567 lines that could not be read",
        "2026-10-19T21:14:26Z ERROR app The WebView2 runtime is registered as 1234567.1 but its files are not there",
    ]);
    let text = build(&input(None, &log)).unwrap();
    assert!(!text.contains("9876") && !text.contains("1234"), "{text}");
    assert!(text.contains("(5 more not shown"), "{text}");
}
