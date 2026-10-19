//! "Copy diagnostics" text (T-626): what a tester sends when something fails,
//! so nobody has to ask for files one by one. App version, game build, parser
//! revision, the `[Power]` lines of `log.config` and the app log's known
//! warnings and errors.
//!
//! It is built from a **fixed schema** and holds nothing but values the app
//! itself vouches for. No input is ever copied through as text:
//!
//! - the app version is checked against a version shape, else "unknown";
//! - the game build and the parser revision are numbers;
//! - a `log.config` line is shown only if it is one of the five known
//!   settings with a number or true/false for a value, rewritten from its
//!   parts; every other line (a comment, a path, a name) is only counted;
//! - an app log line is shown only if it has the log's shape (a UTC time, a
//!   level, an area) *and* its message starts with one of the messages the app
//!   writes (`KNOWN`); then only that fixed message is shown, never the rest of
//!   the line (an error's detail can hold a name or a path), plus the one
//!   number or version a message is allowed to carry. A panic keeps only the
//!   fact that it happened. Every other line is only counted.
//!
//! A last check refuses the whole text, naming nothing, if it still holds
//! something shaped like a BattleTag, an email, a drive path, a profile folder
//! or a long number. The caller shows the text to the player and copies exactly
//! what was shown. Nothing here touches the network or the clipboard.

use std::fmt;
use std::io::Read;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

pub const FORMAT_VERSION: u32 = 1;

/// How many known log lines are shown, the newest.
pub const MAX_LOG_LINES: usize = 20;
/// How many raw log lines the caller hands over to pick from.
pub const LOG_LINES_TO_SCAN: usize = 300;

/// What the diagnostics are made from.
pub struct Input<'a> {
    pub app_version: &'a str,
    /// The newest game build the history saw; `None` before any game.
    pub game_build: Option<i64>,
    pub parser_revision: u32,
    /// The contents of `log.config`; `None` when the file is not there.
    pub log_config: Option<&'a str>,
    /// The app log's lines, oldest first (already redacted when written).
    pub app_log: &'a [String],
}

/// The text was refused: something in it is not allowed to leave the app. It
/// names the kind, never the value.
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal(pub &'static str);

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

static VERSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}(-[A-Za-z0-9.]{1,16})?$")
        .expect("version regex")
});

/// "2026-10-19T21:14:22Z WARN app message".
static LOG_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z) (WARN|ERROR) ([a-z][a-z0-9_-]{0,19}) (.*)$",
    )
    .expect("log line regex")
});

/// "thread 'name' panicked at path/file.rs:1:2: message". Only the fact is
/// kept: a file name or a place read from a log the player can edit is text
/// nobody vouches for, and the whole line is in the app log for whoever asks.
static PANIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^thread '[^']{1,40}' panicked at ").expect("panic regex"));

/// "... registered as 155.0.4283.45 but its files are not there ...".
static WEBVIEW2_STALE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^The WebView2 runtime is registered as ([0-9]{1,6}(?:\.[0-9]{1,6}){0,3}) but its files are not there")
        .expect("webview2 regex")
});

/// "the game history has 3 lines that could not be read".
static HISTORY_LINES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^the game history has ([0-9]{1,6}) lines that could not be read")
        .expect("history regex")
});

/// "unknown start-up task state 7".
static TASK_STATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^unknown start-up task state ([0-9]{1,4})\b").expect("task regex")
});

/// The messages the app writes at WARN and ERROR, by area: the start of the
/// message and the fixed words that stand for it. Whatever follows the start
/// is never shown. A message the app starts to write later is not here until
/// someone adds it, and is only counted until then.
const KNOWN: &[(&str, &str, &str)] = &[
    (
        "app",
        "the previous run ended unexpectedly",
        "the previous run ended unexpectedly",
    ),
    ("app", "failed to start:", "the app failed to start"),
    (
        "app",
        "start-up notice:",
        "start-up notice (details not shown)",
    ),
    (
        "app",
        "The WebView2 runtime is not installed",
        "the WebView2 runtime is not installed",
    ),
    (
        "app",
        "could not open the WebView2 download page",
        "could not open the WebView2 download page",
    ),
    (
        "game-memory",
        "reading from the game stopped after an internal error",
        "reading from the game stopped after an internal error",
    ),
    (
        "setup",
        "the fix stopped after an internal error",
        "the fix stopped after an internal error",
    ),
    (
        "setup",
        "log.config was kept in the app's private storage",
        "log.config was kept in the app's private storage",
    ),
    (
        "tracker",
        "cannot use the game history",
        "cannot use the game history (details not shown)",
    ),
    (
        "tracker",
        "the tracker stopped after an internal error",
        "the tracker stopped after an internal error",
    ),
    (
        "overlay",
        "the overlay stopped after an internal error",
        "the overlay stopped after an internal error",
    ),
    (
        "io",
        "saved tribes:",
        "saved tribes could not be used (details not shown)",
    ),
    (
        "io",
        "saved ratings:",
        "saved ratings could not be used (details not shown)",
    ),
    (
        "startup",
        "reading the start-up entry failed",
        "reading the start-up entry failed (details not shown)",
    ),
    (
        "startup",
        "start-up task call failed",
        "the start-up task call failed (details not shown)",
    ),
    (
        "updates",
        "update check failed",
        "the update check failed (details not shown)",
    ),
    ("upload", "", "an upload problem (details not shown)"),
];

/// The five settings of a `[Power]` section the app asks for or reads.
const POWER_KEYS: [&str; 5] = [
    "LogLevel",
    "FilePrinting",
    "ConsolePrinting",
    "ScreenPrinting",
    "Verbose",
];

/// Shapes a finished text must not contain (a last check behind the schema).
static FORBIDDEN: LazyLock<[(Regex, &'static str); 5]> = LazyLock::new(|| {
    [
        (
            Regex::new(r"[A-Za-z0-9._-]+#\d{3,}").expect("tag"),
            "a BattleTag-shaped text",
        ),
        (
            Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+\.[A-Za-z]{2,}").expect("mail"),
            "an email-shaped text",
        ),
        (
            Regex::new(r"[A-Za-z]:[\\/]").expect("drive"),
            "a drive path",
        ),
        (
            Regex::new(r"(?i)\\users\\|/users/|%userprofile%").expect("profile"),
            "a profile folder",
        ),
        (Regex::new(r"\d{9,}").expect("digits"), "a long number"),
    ]
});

/// The most of `log.config` that is read: the file is a few lines; anything
/// bigger is not one.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// `log.config` as text, or `None` when it is not there or cannot be read.
pub fn read_log_config(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_CONFIG_BYTES)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The `[Power]` lines worth showing, and how many other lines the section
/// had. `None` when there is no file or no `[Power]` section.
fn power_lines(config: &str) -> Option<(Vec<String>, usize)> {
    let mut in_power = false;
    let mut found = false;
    let mut shown = Vec::new();
    let mut other = 0;
    for raw in config.lines() {
        let line = raw.trim();
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            in_power = name.trim().eq_ignore_ascii_case("power");
            found |= in_power;
            continue;
        }
        if !in_power || line.is_empty() {
            continue;
        }
        match power_setting(line) {
            // The first value of a key; a repeated key is not another line.
            Some(setting)
                if !shown
                    .iter()
                    .any(|s: &String| s.split('=').next() == setting.split('=').next()) =>
            {
                shown.push(setting)
            }
            Some(_) => {}
            None => other += 1,
        }
    }
    found.then_some((shown, other))
}

/// "LogLevel = 1" -> "LogLevel=1": a known key with a number or true/false,
/// rewritten from the parts.
fn power_setting(line: &str) -> Option<String> {
    let (key, value) = line.split_once('=')?;
    let key = POWER_KEYS
        .iter()
        .find(|k| k.eq_ignore_ascii_case(key.trim()))?;
    let value = value.trim();
    let number = !value.is_empty() && value.len() <= 2 && value.bytes().all(|b| b.is_ascii_digit());
    let word = ["true", "false"]
        .iter()
        .find(|w| w.eq_ignore_ascii_case(value));
    match (number, word) {
        (true, _) => Some(format!("{key}={value}")),
        (false, Some(word)) => Some(format!("{key}={word}")),
        _ => None,
    }
}

/// One log line as the diagnostics may show it, or `None` for a line that is
/// not a known one.
fn known_log_line(line: &str) -> Option<String> {
    let parts = LOG_LINE.captures(line.trim_end())?;
    let (time, level, area, message) = (&parts[1], &parts[2], &parts[3], &parts[4]);
    let words = if area == "panic" && PANIC.is_match(message) {
        let withheld = message.ends_with("formatted message withheld");
        format!(
            "a panic (place not shown, message {})",
            if withheld { "withheld" } else { "not shown" }
        )
    } else if let Some(stale) = WEBVIEW2_STALE.captures(message).filter(|_| area == "app") {
        format!(
            "the WebView2 runtime is registered as {} but its files are not there",
            &stale[1]
        )
    } else if let Some(n) = HISTORY_LINES.captures(message).filter(|_| area == "io") {
        format!(
            "the game history has {} lines that could not be read",
            &n[1]
        )
    } else if let Some(n) = TASK_STATE.captures(message).filter(|_| area == "startup") {
        format!("unknown start-up task state {}", &n[1])
    } else {
        KNOWN
            .iter()
            .find(|(a, start, _)| *a == area && message.starts_with(start))
            .map(|(_, _, words)| (*words).to_string())?
    };
    Some(format!("{time} {level} {area} {words}"))
}

/// The diagnostics text, or a refusal if the finished text holds something it
/// must not.
pub fn build(input: &Input<'_>) -> Result<String, Refusal> {
    let version = if VERSION.is_match(input.app_version) {
        input.app_version
    } else {
        "unknown"
    };
    let build = input
        .game_build
        .filter(|b| (1..=999_999).contains(b))
        .map_or_else(|| "unknown".to_string(), |b| b.to_string());

    let mut text = format!(
        "Tavern Ledger diagnostics (format {FORMAT_VERSION})\nApp version: {version}\nGame build: {build}\nParser revision: {}\n",
        input.parser_revision
    );

    text.push_str("log.config [Power]:\n");
    match input.log_config.map(power_lines) {
        None => text.push_str("  log.config not found\n"),
        Some(None) => text.push_str("  no [Power] section\n"),
        Some(Some((shown, other))) => {
            if shown.is_empty() {
                text.push_str("  no known setting\n");
            }
            for setting in &shown {
                text.push_str(&format!("  {setting}\n"));
            }
            if other > 0 {
                text.push_str(&format!("  ({other} other lines not shown)\n"));
            }
        }
    }

    let mut known: Vec<String> = Vec::new();
    let mut hidden = 0;
    for line in input.app_log {
        match known_log_line(line) {
            Some(shown) => known.push(shown),
            None if LOG_LINE.is_match(line.trim_end()) => hidden += 1,
            None => {}
        }
    }
    let skipped = known.len().saturating_sub(MAX_LOG_LINES);
    let recent = &known[skipped..];
    text.push_str("App log, warnings and errors:\n");
    if recent.is_empty() {
        text.push_str("  none among the known messages\n");
    }
    for line in recent {
        text.push_str(&format!("  {line}\n"));
    }
    if hidden + skipped > 0 {
        text.push_str(&format!(
            "  ({} more not shown: older, or not a known message)\n",
            hidden + skipped
        ));
    }

    for (pattern, what) in FORBIDDEN.iter() {
        if pattern.is_match(&text) {
            return Err(Refusal(what));
        }
    }
    Ok(text)
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
