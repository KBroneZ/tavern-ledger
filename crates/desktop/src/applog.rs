//! The app's error log (T-580): `logs\app.log` in the data folder, one line per
//! event (time, level, area, message), UTF-8, rotating over a few files. Only
//! the Rust side writes it. Errors and warnings are always logged, and info
//! only for the few things worth knowing when something went wrong (start,
//! stop, version, the Logs folder in use, a game's start and end, upload
//! results). There is no debug level.
//!
//! Every message is redacted ([`crate::redact`]) before it is written, so the
//! file holds no BattleTag, account id, player name, email, token or key, and
//! no user name inside a path. It stays on this PC; the problem report offers
//! its last lines to the player, who decides whether to send them.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::{self, Display};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use bg_parser::report::Status as ReportStatus;
use tracker::game_clock::civil_from_days;
use tracker::Saved;

use crate::redact::{one_line, redact};

/// The log's folder inside the data folder.
pub const DIR: &str = "logs";
pub const FILE: &str = "app.log";
/// Present while the app that holds the history runs; a clean quit removes it.
const MARKER: &str = "running.marker";
/// A file is rotated once it would pass this size.
const MAX_BYTES: u64 = 1024 * 1024;
/// `app.log` plus the older ones (`app.log.1`, `app.log.2`).
const FILES: usize = 3;
/// A longer message is cut: one line, never a dump.
const MAX_MESSAGE_CHARS: usize = 500;
/// The most of a message that is looked at (and redacted) before it is cut.
const MAX_INPUT_CHARS: usize = 4_000;
/// More lines than this in a minute are counted, not written.
const MAX_LINES_PER_MINUTE: u32 = 120;
const SECS_PER_DAY: i64 = 86_400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
}

impl Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
        })
    }
}

/// `2026-10-13T09:05:00Z`, from seconds since 1970.
fn timestamp(unix: i64) -> String {
    let (year, month, day) = civil_from_days(unix.div_euclid(SECS_PER_DAY));
    let secs = unix.rem_euclid(SECS_PER_DAY);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// The area is a short lowercase word; anything else in it is dropped.
fn area_word(area: &str) -> String {
    area.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(16)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// What a line says after its time: level, area and the redacted message.
fn format_body(level: Level, area: &str, message: &str) -> String {
    // Redacted whole first and cut after: a cut must not leave half a secret.
    let text = one_line(
        &redact(&one_line(message, MAX_INPUT_CHARS)),
        MAX_MESSAGE_CHARS,
    );
    format!("{level} {} {text}", area_word(area))
}

/// One log line, without its newline.
#[cfg(test)]
fn format_line(unix: i64, level: Level, area: &str, message: &str) -> String {
    format!("{} {}", timestamp(unix), format_body(level, area, message))
}

/// What the log remembers between lines, to keep a loop of failures from
/// pushing everything else out of three small files.
#[derive(Default)]
struct Flow {
    last: Option<String>,
    repeats: u32,
    minute: i64,
    in_minute: u32,
    dropped: u32,
}

impl Flow {
    /// The lines to write for one more line (a note first when the lines
    /// before it were repeats or were left out).
    fn admit(&mut self, unix: i64, body: String) -> Vec<String> {
        let mut out = Vec::new();
        if self.last.as_ref() == Some(&body) {
            self.repeats += 1;
            return out;
        }
        if self.repeats > 0 {
            let note = format!(
                "INFO app the line above was repeated {} more times",
                self.repeats
            );
            out.push(format!("{} {note}", timestamp(unix)));
        }
        let minute = unix.div_euclid(60);
        if minute != self.minute {
            if self.dropped > 0 {
                let note = format!("WARN app {} lines were not logged (too many)", self.dropped);
                out.push(format!("{} {note}", timestamp(unix)));
            }
            (self.minute, self.in_minute, self.dropped) = (minute, 0, 0);
        }
        self.repeats = 0;
        if self.in_minute >= MAX_LINES_PER_MINUTE {
            self.dropped += 1;
        } else {
            self.in_minute += 1;
            out.push(format!("{} {body}", timestamp(unix)));
        }
        self.last = Some(body);
        out
    }
}

/// Clears the thread's "writing" mark when the write ends, also by a panic.
struct Writing;

impl Drop for Writing {
    fn drop(&mut self) {
        WRITING.set(false);
    }
}

thread_local! {
    /// Set while this thread is writing a line: a panic inside the log (or a
    /// line logged from the panic hook meanwhile) must not write again.
    static WRITING: Cell<bool> = const { Cell::new(false) };
}

pub struct Log {
    dir: PathBuf,
    max_bytes: u64,
    files: usize,
    /// One writer at a time, so a line is never torn and a rotation never
    /// runs under another write.
    write: Mutex<Flow>,
}

impl Log {
    /// The log of this data folder.
    pub fn new(data_dir: &Path) -> Self {
        Self::with_limits(data_dir.join(DIR), MAX_BYTES, FILES)
    }

    fn with_limits(dir: PathBuf, max_bytes: u64, files: usize) -> Self {
        Log {
            dir,
            max_bytes,
            files: files.max(1),
            write: Mutex::new(Flow::default()),
        }
    }

    pub fn record(&self, level: Level, area: &str, message: &str) {
        self.record_at(now_unix(), level, area, message);
    }

    fn record_at(&self, unix: i64, level: Level, area: &str, message: &str) {
        if WRITING.replace(true) {
            return;
        }
        let _done = Writing;
        let body = format_body(level, area, message);
        let mut flow = self.write.lock().unwrap_or_else(|e| e.into_inner());
        for line in flow.admit(unix, body) {
            // A log that cannot be written must never stop the app; the line
            // goes to stderr, where a console run shows it.
            if self.append(&line).is_err() {
                let _ = writeln!(io::stderr(), "{line}");
            }
        }
    }

    fn path(&self, n: usize) -> PathBuf {
        match n {
            0 => self.dir.join(FILE),
            n => self.dir.join(format!("{FILE}.{n}")),
        }
    }

    fn append(&self, line: &str) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let current = self.path(0);
        let size = fs::metadata(&current).map_or(0, |m| m.len());
        let added = u64::try_from(line.len() + 1).unwrap_or(u64::MAX);
        if size > 0 && size + added > self.max_bytes && self.rotate().is_err() {
            // An editor or a scanner holds an older file: start the newest
            // over rather than stop logging or let it grow without end.
            let _ = File::create(&current);
        }
        let mut file = OpenOptions::new().create(true).append(true).open(current)?;
        file.write_all(format!("{line}\n").as_bytes())
    }

    /// `app.log` becomes `app.log.1`, that one `app.log.2`…; the oldest goes.
    fn rotate(&self) -> io::Result<()> {
        let oldest = self.path(self.files - 1);
        if oldest.exists() {
            fs::remove_file(&oldest)?;
        }
        for n in (0..self.files - 1).rev() {
            let from = self.path(n);
            if from.exists() {
                fs::rename(&from, self.path(n + 1))?;
            }
        }
        Ok(())
    }

    /// The last `max_lines` lines, oldest first, from the newest files back.
    pub fn tail(&self, max_lines: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for n in 0..self.files {
            if lines.len() >= max_lines {
                break;
            }
            let Ok(text) = read_lossy(&self.path(n)) else {
                continue;
            };
            // Redacted again: the file is the user's to edit, and an older
            // version of the app wrote some of it.
            let older: Vec<String> = text
                .lines()
                .map(|l| one_line(&redact(l), MAX_MESSAGE_CHARS + 40))
                .collect();
            lines.splice(0..0, older);
        }
        let skip = lines.len().saturating_sub(max_lines);
        lines.split_off(skip)
    }

    /// Notes that this run started. True when the last one did not end
    /// cleanly: its marker was still there.
    pub fn mark_running(&self) -> bool {
        let marker = self.dir.join(MARKER);
        let left_behind = marker.exists();
        let written =
            fs::create_dir_all(&self.dir).and_then(|()| fs::write(&marker, timestamp(now_unix())));
        if let Err(e) = written {
            self.record(
                Level::Warn,
                "app",
                &format!("could not write the running marker ({:?})", e.kind()),
            );
        }
        left_behind
    }

    /// A clean quit: the marker goes.
    pub fn mark_stopped(&self) {
        match fs::remove_file(self.dir.join(MARKER)) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => self.record(
                Level::Warn,
                "app",
                &format!("could not remove the running marker ({:?})", e.kind()),
            ),
        }
    }

    /// Shows the folder in the system's file browser.
    pub fn open_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        // The system's own explorer, never one found on a search path, and an
        // absolute folder, which can never read as a switch.
        let opener = if cfg!(windows) {
            let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
            PathBuf::from(root).join("explorer.exe")
        } else {
            PathBuf::from("xdg-open")
        };
        tracker::quiet::command(opener)
            .arg(std::path::absolute(&self.dir)?)
            .spawn()
            .map(|_| ())
    }
}

/// A file as text; bytes that are not UTF-8 become `�` (a torn write must not
/// hide the rest of the log).
fn read_lossy(path: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// --- the log of this run ---

static LOG: OnceLock<Arc<Log>> = OnceLock::new();

/// Starts the log of this run in the data folder and sets the panic hook.
/// Returns the log, or the one already started.
pub fn init(data_dir: &Path) -> Arc<Log> {
    let mut first = false;
    let log = LOG.get_or_init(|| {
        first = true;
        Arc::new(Log::new(data_dir))
    });
    if first {
        install_panic_hook(log.clone());
    }
    log.clone()
}

pub fn get() -> Option<Arc<Log>> {
    LOG.get().cloned()
}

fn write(level: Level, area: &str, message: &str) {
    if let Some(log) = LOG.get() {
        log.record(level, area, message);
    }
}

pub fn error(area: &str, message: &str) {
    write(Level::Error, area, message);
}

pub fn warn(area: &str, message: &str) {
    write(Level::Warn, area, message);
}

/// What a polled problem said last, by key: a problem that comes back every
/// second is one line, until it changes or [`recovered`].
static LAST: Mutex<BTreeMap<&'static str, String>> = Mutex::new(BTreeMap::new());

/// True when `message` is not what `key` said last (and remembers it).
fn is_new(key: &'static str, message: &str) -> bool {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.get(key).is_some_and(|m| m == message) {
        return false;
    }
    last.insert(key, message.to_string());
    true
}

pub fn error_once(key: &'static str, area: &str, message: &str) {
    if is_new(key, message) {
        error(area, message);
    }
}

pub fn warn_once(key: &'static str, area: &str, message: &str) {
    if is_new(key, message) {
        warn(area, message);
    }
}

/// The problem of `key` is gone: it is logged again if it comes back.
pub fn recovered(key: &'static str) {
    LAST.lock().unwrap_or_else(|e| e.into_inner()).remove(key);
}

pub fn info(area: &str, message: &str) {
    write(Level::Info, area, message);
}

/// Whether this run holds the history (and so owns the running marker): a
/// second window that only shows the history must not mark or unmark a run
/// that is not its own.
static OWNS_MARKER: AtomicBool = AtomicBool::new(false);

/// This run holds the history lock: it writes the running marker, and says
/// so when the last run that held it never removed its own.
pub fn history_held() {
    let Some(log) = LOG.get() else { return };
    OWNS_MARKER.store(true, Ordering::SeqCst);
    if log.mark_running() {
        log.record(Level::Warn, "app", "the previous run ended unexpectedly");
    }
}

/// The app is quitting by its own way out: a clean stop.
pub fn stopped() {
    info("app", "Tavern Ledger stopped");
    if let Some(log) = LOG.get() {
        if OWNS_MARKER.swap(false, Ordering::SeqCst) {
            log.mark_stopped();
        }
    }
}

/// A game the tracker saved: its place in the history and how well it was
/// read, in numbers and words the parser chose (never names, never the
/// parser's own free text, which can hold them).
pub fn saved_game_line(saved: &Saved) -> (Level, String) {
    let report = &saved.report;
    let status = serde_json::to_value(report.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let mut line = format!(
        "game {} saved ({}): status {status}, {} warnings, {} problems",
        saved.key.index,
        saved.key.session,
        report.warnings.len(),
        report.problems.len()
    );
    // The window says only that the game could not be read; the parser's
    // reason is kept here for the problem report (T-357).
    let errors = parser_errors(&report.problems);
    if !errors.is_empty() {
        line.push_str(&format!(" (parser error: {})", errors.join(", ")));
    }
    let clean = report.status == ReportStatus::Ok && report.problems.is_empty();
    (if clean { Level::Info } else { Level::Warn }, line)
}

/// The parser's own error names ("parser error: NotImplementedError"): fixed
/// words of the parser, safe to log. Its other problems are free text and can
/// hold names, so they are not.
pub fn parser_errors(problems: &[String]) -> Vec<&str> {
    problems
        .iter()
        .filter_map(|p| p.strip_prefix("parser error: "))
        .filter(|name| !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric()))
        .collect()
}

pub fn saved_game(saved: &Saved) {
    let (level, line) = saved_game_line(saved);
    write(level, "game", &line);
}

/// What a command that failed leaves in the log: its name and the error.
pub trait Logged<T, E> {
    fn logged(self, command: &str) -> Result<T, E>;
}

impl<T, E: Display> Logged<T, E> for Result<T, E> {
    fn logged(self, command: &str) -> Result<T, E> {
        if let Err(e) = &self {
            error("command", &format!("{command} failed: {e}"));
        }
        self
    }
}

// --- panics ---

type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Send + Sync + 'static>;

/// "thread 'name' panicked at src/file.rs:1:2: message".
fn panic_text(info: &PanicHookInfo<'_>) -> String {
    let payload = info.payload();
    // A literal is the app's own words. A formatted message can quote what
    // was being read (a name from the log), so only its place is kept.
    let message = match payload.downcast_ref::<&str>() {
        Some(literal) => (*literal).to_string(),
        None if payload.is::<String>() => "formatted message withheld".to_string(),
        None => "no message".to_string(),
    };
    let place = info.location().map_or_else(String::new, |l| {
        format!(" at {}:{}:{}", l.file(), l.line(), l.column())
    });
    let thread = std::thread::current();
    let name = thread.name().unwrap_or("unnamed");
    format!("thread '{name}' panicked{place}: {message}")
}

/// A hook that logs the panic, then runs `previous` (the default hook prints
/// to the console as before; the app's own handling of a panic is unchanged).
fn panic_hook(log: Arc<Log>, previous: PanicHook) -> PanicHook {
    Box::new(move |info| {
        // A panic inside the hook would abort the app, and with it the recovery
        // the tracker thread does after a panic of its own.
        if let Ok(text) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| panic_text(info)))
        {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                log.record(Level::Error, "panic", &text);
            }));
        }
        previous(info);
    })
}

fn install_panic_hook(log: Arc<Log>) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(panic_hook(log, previous));
}

#[cfg(test)]
#[path = "applog_tests.rs"]
mod tests;
