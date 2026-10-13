//! What the windows can ask of the error log (T-580): forward an error the
//! page itself raised, and open the log's folder. The page never writes the
//! file: the app does, after trimming, redacting and rate-limiting the text.

use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use regex::Regex;
use tauri::{State, WebviewWindow};

use crate::applog::{self, Level, Logged};
use crate::redact::{one_line, redact};

/// At most this many page errors are logged per [`WINDOW`]; a page stuck in
/// an error loop cannot fill the log.
const MAX_PER_WINDOW: u32 = 5;
const WINDOW: Duration = Duration::from_secs(60);
/// A page error's message and place are cut to this many characters.
const MAX_MESSAGE_CHARS: usize = 300;
const MAX_PLACE_CHARS: usize = 120;
/// "Open log folder" does nothing again within this time.
const OPEN_COOLDOWN: Duration = Duration::from_secs(2);

/// The count of page errors in the current minute.
#[derive(Debug)]
pub struct Limiter {
    since: Option<Instant>,
    taken: u32,
    dropped: u32,
}

/// What to do with one page error.
#[derive(Debug, PartialEq, Eq)]
pub enum Admit {
    /// Log it; `dropped` errors of the minute before were left out.
    Log {
        dropped: u32,
    },
    Skip,
}

impl Limiter {
    pub const fn new() -> Self {
        Limiter {
            since: None,
            taken: 0,
            dropped: 0,
        }
    }

    pub fn admit(&mut self, now: Instant) -> Admit {
        let fresh = self.since.is_none_or(|s| now.duration_since(s) >= WINDOW);
        let mut dropped = 0;
        if fresh {
            dropped = std::mem::take(&mut self.dropped);
            self.since = Some(now);
            self.taken = 0;
        }
        if self.taken >= MAX_PER_WINDOW {
            self.dropped = self.dropped.saturating_add(1);
            return Admit::Skip;
        }
        self.taken += 1;
        Admit::Log { dropped }
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

/// One limiter per window, so a page in a loop cannot use up the other's share.
#[derive(Default)]
pub struct UiErrors(Mutex<BTreeMap<String, Limiter>>);

/// The longest page error text looked at before it is redacted and cut.
const MAX_INPUT_CHARS: usize = 2_000;

/// Quoted text (`'Bob'`, `"Bob"`) is what a page error copies from its data:
/// the quotes stay, what is inside goes.
fn strip_quoted(text: &str) -> String {
    static QUOTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"'[^']*'|"[^"]*"|`[^`]*`"#).expect("a fixed pattern"));
    QUOTED
        .replace_all(text, |c: &regex::Captures| {
            let quote = &c[0][..1];
            format!("{quote}…{quote}")
        })
        .into_owned()
}

/// "error" and "rejection" are what the page reports; anything else is "other".
fn kind_word(kind: &str) -> &'static str {
    match kind {
        "error" => "error",
        "rejection" => "rejection",
        _ => "other",
    }
}

/// The line a page error leaves, before redaction.
fn ui_message(window: &str, kind: &str, message: &str, place: Option<&str>) -> String {
    // Redacted whole, then cut: a cut must not leave half a name or a key.
    let bounded = one_line(message, MAX_INPUT_CHARS);
    let message = one_line(&redact(&strip_quoted(&bounded)), MAX_MESSAGE_CHARS);
    match place
        .map(|p| one_line(p, MAX_PLACE_CHARS))
        .filter(|p| !p.is_empty())
    {
        Some(place) => format!("[{window}] {} {message} ({place})", kind_word(kind)),
        None => format!("[{window}] {} {message}", kind_word(kind)),
    }
}

/// An error the page raised (`window.onerror`, an unhandled rejection).
#[tauri::command]
pub fn log_ui_error(
    window: WebviewWindow,
    errors: State<'_, UiErrors>,
    kind: String,
    message: String,
    place: Option<String>,
) {
    let Some(log) = applog::get() else { return };
    let admitted = errors
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(window.label().to_string())
        .or_default()
        .admit(Instant::now());
    let Admit::Log { dropped } = admitted else {
        return;
    };
    if dropped > 0 {
        log.record(
            Level::Warn,
            "ui",
            &format!("{dropped} more page errors were not logged (too many)"),
        );
    }
    let text = ui_message(window.label(), &kind, &message, place.as_deref());
    log.record(Level::Error, "ui", &text);
}

/// Shows the log's folder in the file browser (Settings: "Open log folder").
#[tauri::command]
pub fn open_log_folder() -> Result<(), String> {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    {
        // One file browser at a time, not one per call.
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        if last.is_some_and(|at| now.duration_since(at) < OPEN_COOLDOWN) {
            return Ok(());
        }
        *last = Some(now);
    }
    let log = applog::get().ok_or("The error log is not running.")?;
    log.open_dir()
        .map_err(|e| format!("Could not open the log folder ({:?}).", e.kind()))
        .logged("open_log_folder")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: u64, base: Instant) -> Instant {
        base + Duration::from_secs(secs)
    }

    #[test]
    fn only_a_few_page_errors_are_logged_each_minute() {
        let base = Instant::now();
        let mut limiter = Limiter::new();
        for _ in 0..MAX_PER_WINDOW {
            assert_eq!(limiter.admit(at(1, base)), Admit::Log { dropped: 0 });
        }
        for _ in 0..20 {
            assert_eq!(limiter.admit(at(30, base)), Admit::Skip);
        }
    }

    #[test]
    fn the_next_minute_logs_again_and_says_how_many_were_left_out() {
        let base = Instant::now();
        let mut limiter = Limiter::new();
        for _ in 0..MAX_PER_WINDOW + 7 {
            limiter.admit(at(0, base));
        }
        assert_eq!(limiter.admit(at(61, base)), Admit::Log { dropped: 7 });
        assert_eq!(limiter.admit(at(62, base)), Admit::Log { dropped: 0 });
    }

    /// Every `.rs` file under src/, as text with LF line ends.
    fn sources() -> Vec<(String, String)> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .map(|p| {
                let text = std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n");
                (p.file_name().unwrap().to_string_lossy().into_owned(), text)
            })
            .collect()
    }

    #[test]
    fn every_command_that_can_fail_leaves_its_error_in_the_log() {
        let mut checked = 0;
        for (file, text) in sources() {
            for (at, _) in text.match_indices(concat!("#[tauri::", "command]")) {
                let rest = &text[at..];
                let fn_at = rest.find("fn ").unwrap();
                let open = rest.find('{').unwrap();
                let signature = &rest[fn_at..open];
                if !signature.contains("-> Result") {
                    continue;
                }
                let name: String = signature[3..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                let body = &rest[open..open + rest[open..].find("\n}\n").unwrap()];
                assert!(
                    body.contains(&format!(".logged(\"{name}\")")),
                    "{file}: the command {name} can fail but does not log its error"
                );
                checked += 1;
            }
        }
        assert!(checked >= 15, "the scan found the commands ({checked})");
    }

    #[test]
    fn a_page_error_is_trimmed_to_one_short_line() {
        let long = format!("bad\nthing {}", "x".repeat(1_000));
        let text = ui_message("main", "error", &long, Some("app.js:10:5"));
        assert!(!text.contains('\n'));
        assert!(text.starts_with("[main] error bad thing "), "{text}");
        assert!(text.ends_with("(app.js:10:5)"), "{text}");
        assert!(text.chars().count() < 450, "{}", text.chars().count());
    }

    #[test]
    fn an_unknown_kind_is_called_other_and_no_place_is_fine() {
        assert_eq!(
            ui_message("overlay", "weird", "m", None),
            "[overlay] other m"
        );
        assert_eq!(
            ui_message("overlay", "rejection", "m", Some("  ")),
            "[overlay] rejection m"
        );
    }

    #[test]
    fn quoted_text_of_a_page_error_is_cut_but_the_quotes_stay() {
        assert_eq!(
            strip_quoted("Cannot read properties of undefined (reading 'Bob') in \"Jane\" and `x`"),
            "Cannot read properties of undefined (reading '…') in \"…\" and `…`"
        );
    }

    #[test]
    fn a_name_is_cut_whole_even_where_the_message_is_later_trimmed() {
        let text = format!("{} Jaina#1234", "w".repeat(MAX_MESSAGE_CHARS - 20));
        let line = ui_message("main", "error", &text, None);
        assert!(!line.contains("Jaina") && !line.contains("#12"), "{line}");
    }
}
